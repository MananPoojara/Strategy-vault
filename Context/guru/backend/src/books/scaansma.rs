//! Peter Aan, "Simple moving average crossover", Technical Analysis of Stocks & Commodities V.7:6 (June 1989) pp.185-187
//! (research share Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V07/C06/SIMPLEM.pdf).
//! Book id `scaansma`. Lane K16 (titles loop, 2 Nov 2026), to-code row 237, lead decision 73; independent check
//! authors/titles-1030/K16w/k16_ref.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::msws::sma;
use super::sctraub::next_open;

pub const REVIEWED: &[&str] = &["scaansma_3_10"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] The article's worked example, a 3-day and a 10-day simple average of closes, is the default (decision 73); it tested 3-19 days for the short and 10-50 days for the long average, which are options. A cross = the short average above (below) the long one today and not yesterday; the order goes in at tomorrow's open; always in after the first cross. The $100 per trade for commissions and slippage of the article's futures tests is a tester setting (vault standard costs apply). The article's results (futures, about 5.5 years to mid-1988) are shown only as a chart. Other vault cards with a 3/10 average cross come from other authors and rules. Runs on indices, ETFs and stocks, US and India, are a declared application.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "scaansma_3_10", name: "Aan simple MA crossover: 3-day SMA crosses the 10-day SMA, stop and reverse at the next open", group: "S&C: Simple moving average crossover (Aan, V.7:6)", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "scaansma", rules: &[
            "E|\"When the shorter-term (fewer days) average crosses above the longer-term average from below, this is a buy signal for tomorrow's open.\" (V.7:6, PDF p.1)",
            "X|\"When the shorter-term average crosses below the longer-term average from above, this is a sell signal for tomorrow's open.\" (PDF p.1); \"liquidate your long position and establish a new short position on the next day's open\" (PDF p.3)",
            "F|\"you might calculate a 3-day and a 10-day MA of the closing prices\" (PDF p.1, p.3)"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    reviewed(id).then_some(("scaansma_3_10", "Aan simple MA crossover (S&C)", "3/10 days"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    reviewed(id).then(|| vec![p("fast", "Short SMA, days (example 3; tested 3-19)", 3., 1., 19., 1.), p("slow", "Long SMA, days (example 10; tested 10-50)", 10., 2., 50., 1.)])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let g = |k: &str, d: f64| p.get(k).copied().unwrap_or(d).round().max(1.) as usize;
    let (a, b) = (sma(&x.c, g("fast", 3.)), sma(&x.c, g("slow", 10.)));
    Some(next_open(x, from, to, "reversed", |t, side| {
        if t == 0 || !(a[t - 1].is_finite() && b[t - 1].is_finite()) { return side; }
        if a[t] > b[t] && a[t - 1] <= b[t - 1] { 1.0 } else if a[t] < b[t] && a[t - 1] >= b[t - 1] { -1.0 } else { side }
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sma_cross_reverses() {
        let r = super::super::sctraub::tests::bars(1500);
        let x = Ctx::new(&r, &r, &[]);
        let t = run("scaansma_3_10", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(t.len() > 10);
        for w in t.windows(2) { assert_eq!(w[0].exit_i, w[1].legs[0].i); }
    }
}

// Instrument admission: S&C Aan SMA crossover 2026-11-02 (titles lane K16).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "scaansma" && reviewed(def.id),
    label: "Peter Aan tested the crossover on futures (about 5.5 years, results as a chart). Runs on indices, ETFs and stocks, US and India, are a declared application.",
    basis: "Technical Analysis of Stocks & Commodities V.7:6 pp.185-187 (Simple moving average crossover); independent check authors/titles-1030/K16w/k16_ref.py",
};
