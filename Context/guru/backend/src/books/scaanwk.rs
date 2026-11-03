//! Peter Aan, "Weekly high/low moving average", Technical Analysis of Stocks & Commodities V.7:12 (December 1989) pp.431-432
//! (research share Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V07/C12/WEEKLYH.pdf).
//! Book id `scaanwk`. Lane K16 (titles loop, 2 Nov 2026), to-code row 240, decision 73; independent check
//! authors/titles-1030/K16w/k16_ref.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::lc3::{hi, lo};
use super::scaancb::sar_stops;
use super::sctraub::week_groups;

pub const REVIEWED: &[&str] = &["scaanwk_3"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] The article's example, a 3-week average, is the default (decision 73; it tested 2-20 weeks, options). Weeks are calendar weeks of the vault's daily bars (high = the week's highest high, low = its lowest low). For every day of a week the buy stop is the average of the weekly highs, and the sell stop the average of the weekly lows, of the n completed calendar weeks before it ('On Friday, recalculate the new moving average high and low and place your new stop and reverse at these levels for the next week'). If short, buy and reverse on the buy stop; if long, sell and reverse on the sell stop; a stop fills at its level or at the open when the open is beyond it; one fill a day; with no position yet the first stop touched opens the trade (both: the one the open is through, else the extreme nearer the open). Article results (futures, 5.5 years to June 1989, System Writer Plus) are a chart only. Runs on indices, ETFs and stocks, US and India, are a declared application.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "scaanwk_3", name: "Aan weekly high/low average: stop and reverse at the 3-week average weekly high / low", group: "S&C: Weekly high/low moving average (Aan, V.7:12)", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "scaanwk", rules: &[
            "E|\"If short, reverse your position on an intraday buy stop placed at the n-week average of the highs of the most recent n calendar weeks.\" (V.7:12, PDF p.1)",
            "X|\"If long, reverse your position on an intraday sell stop placed at the n-week average of the lows of the most recent n calendar weeks.\" (PDF p.1)",
            "F|\"if the bond market has a 3-week average weekly high of 98-07 and a 3-week average low of 95-18, you would go long if the market trades above 98-07 ... On Friday, recalculate the new moving average high and low and place your new stop and reverse at these levels for the next week.\" (PDF p.1)"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    reviewed(id).then_some(("scaanwk_3", "Aan weekly high/low average (S&C)", "3 weeks"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    reviewed(id).then(|| vec![p("n", "Weeks in the averages (example 3; tested 2-20)", 3., 1., 20., 1.)])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let n = p.get("n").copied().unwrap_or(3.).round().max(1.) as usize;
    let g = week_groups(x);
    let wh: Vec<f64> = g.iter().map(|&(s, e)| hi(&x.h, s, e)).collect();
    let wl: Vec<f64> = g.iter().map(|&(s, e)| lo(&x.l, s, e)).collect();
    let mut wk = vec![0usize; x.r.len()];
    for (k, &(s, e)) in g.iter().enumerate() { for i in s..=e { wk[i] = k; } }
    let avg = |v: &[f64], k: usize| v[k - n..k].iter().sum::<f64>() / n as f64;
    Some(sar_stops(x, from, to, |i| { let k = wk[i]; (k >= n).then(|| (avg(&wh, k), avg(&wl, k))) }))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn weekly_average_reverses() {
        let r = super::super::sctraub::tests::bars(3000);
        let x = Ctx::new(&r, &r, &[]);
        let t = run("scaanwk_3", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(t.len() > 5);
        for w in t.windows(2) { assert_eq!(w[0].exit_i, w[1].legs[0].i); }
    }
}

// Instrument admission: S&C Aan weekly high/low average 2026-11-02 (titles lane K16).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "scaanwk" && reviewed(def.id),
    label: "Peter Aan tested the weekly averages on futures (5.5 years, results as a chart). Runs on indices, ETFs and stocks, US and India, are a declared application.",
    basis: "Technical Analysis of Stocks & Commodities V.7:12 pp.431-432 (Weekly high/low moving average); independent check authors/titles-1030/K16w/k16_ref.py",
};
