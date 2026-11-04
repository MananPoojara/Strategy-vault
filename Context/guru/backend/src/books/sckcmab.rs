//! Kent Calhoun, "Optimal parameter selection", Technical Analysis of Stocks & Commodities V.7:11 (1989) pp.418-421 (research share
//! Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V07/C11/OPTIMAL.pdf): the moving average
//! percentage price band system of the worked example. Book id `sckcmab`. Lane K41 (batch T7a, 4 Nov 2026); independent check
//! authors/titles-1030/K41w/k41_ref.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::msws::sma;
use super::sctraub::next_open;

pub const REVIEWED: &[&str] = &["sckcmab_band"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] Default = the article's chosen T-bond parameters, a 25-day moving average with a 3.3% band (PDF p.5); the Figure 4 example (20 days, 5%) and the 18-22 day / 4.5-5.5% shifts are options. 'Moving average' = simple. A close above the upper band goes long and a close below the lower band goes short (reversing); 'The moving average serves only as a stop at which the trade is closed': a long is closed when the close is below the average, a short when the close is above it. The article does not give the fill: orders fill at the next open. Optimisation study on T-bond futures with $100 per trade (no trade list). Runs on indices, ETFs and stocks, US and India, are a declared application.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "sckcmab_band", name: "Moving average % band: close above SMA(25) + 3.3% long, below SMA - 3.3% short, the SMA is the stop, next open", group: "S&C: moving average price band (Calhoun, V.7:11)", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "sckcmab", rules: &[
            "E|\"This price band is a channel that encloses the moving average (Figure 4). When price closes above the upper band, the system is giving a buy signal. Sell signals are closes below the lower band.\" (V.7:11, PDF p.4)",
            "X|\"The moving average serves only as a stop at which the trade is closed.\" (PDF p.4)",
            "F|\"the T-bond optimal parameters in the moving average percentage price band system will be a 25-day moving average with a 3.3% price band\" (PDF p.5); example \"a 20-day moving average's 5% band\" (PDF p.4)"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { reviewed(id).then_some(("sckcmab_band", "Moving average % band (S&C)", "SMA stop")) }
pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    reviewed(id).then(|| vec![
        ParamDef { key: "n", label: "Moving average days (25 = the article's T-bond choice; Figure 4 example 20)", def: 25., min: 5., max: 60., step: 1. },
        ParamDef { key: "band", label: "Band % (3.3 = the article's choice; Figure 4 example 5)", def: 3.3, min: 0.5, max: 10., step: 0.1 }])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let n = p.get("n").copied().unwrap_or(25.).round().max(2.) as usize;
    let b = p.get("band").copied().unwrap_or(3.3) / 100.0;
    let m = sma(&x.c, n);
    let c = &x.c;
    Some(next_open(x, from, to, "Close beyond the moving average", |t, side| {
        if !m[t].is_finite() { return side; }
        if c[t] > m[t] * (1.0 + b) { 1.0 } else if c[t] < m[t] * (1.0 - b) { -1.0 }
        else if (side > 0.0 && c[t] < m[t]) || (side < 0.0 && c[t] > m[t]) { 0.0 } else { side }
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn band_entries_and_ma_stop() {
        let r = super::super::sctraub::tests::bars(1500);
        let x = Ctx::new(&r, &r, &[]);
        let mut p = Params::new(); p.insert("band".into(), 1.0);
        let t = run("sckcmab_band", &x, 0, r.len() - 1, &p).unwrap();
        assert!(t.iter().any(|z| z.dir > 0.0) && t.iter().any(|z| z.dir < 0.0));
        assert!(t.iter().any(|z| z.reason.contains("moving average")));
    }
}
