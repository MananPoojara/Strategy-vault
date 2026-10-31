//! John Ehlers, Mesa98 v4.1 seminar slides 'Predictive Indicators' (PREDICT.ppt): 'A Simple Predictive Trading System';
//! research share USB HDD 1/TradeStation Systems & Indicators/R-MESA 5 (mesasoftware.com)/_Old Mesa versions/Mesa 98 4.1 for
//! TS & Manual/Mesa98 v4.1 - Powerpoint Tutorial & Word documents/Powerpoint Tuturiol/PREDICT.ppt. Book id `ehpred`. Lane K7
//! (titles loop, 1 Nov 2026); slide text authors/batch-c/C4w/ehlers-predict.txt; independent check
//! authors/titles-1030/K7w/k7_ref.py (header).
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::rbtie::target_trades;

/// Admitted ids (authors/titles-1030/K7w/k7_ref.py (header)).
pub const REVIEWED: &[&str] = &["ehpred_simple"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] Price = the close. EMA with a = 0.25 (the slides' worked 'EMA a = .25'), started at the first close; the slides' better '3 Pole filter' is not coded. Predictor = close + (close - EMA). Buy when the Predictor crosses above the EMA, sell when it crosses below; 'Sell' is read as reversing to short (option: long only). Signals on the close, filled at the next day's open. The Predictor is above the EMA exactly when the close is, so this is a close/EMA crossover. The author's own verdict on the slide: 'Usually produces too many whipsaws to be practical'; it is shown as his teaching example, without a claim. The BandPass and RSI 'optimum predictor' crossings on the same slides are not coded (they need MESA's dominant cycle or a hand-tuned RSI length). Ehlers's examples are futures (gold); US and India indices, ETFs and stocks are a declared application.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "ehpred_simple", name: "A Simple Predictive Trading System: the predictor (close + (close - EMA)) crossing its EMA, stop and reverse",
        group: "Ehlers predictive filters", session: 0, dir: "Long/Short", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: "ehpred", rules: &[
            "F|\"Procedure to generate a predictive line: Take an EMA of price (better, a 3 Pole filter); Take the difference (delta) between the price and its EMA; Form the predictor by adding delta to the price\"; \"EMA a = .25\"",
            "E|\"Rules: Buy when Predictor crosses EMA from bottom to top\"",
            "X|\"Sell when Predictor crosses EMA from top to bottom\"; \"Usually produces too many whipsaws to be practical\""],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    (id == "ehpred_simple").then_some(("ehpred_simple", "Ehlers simple predictive system", "EMA a = 0.25"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    (id == "ehpred_simple").then(|| vec![
        ParamDef { key: "a", label: "EMA weight a (slides 0.25)", def: 0.25, min: 0.01, max: 0.9, step: 0.01 },
        ParamDef { key: "side", label: "0 = stop and reverse (reading), 1 = long only", def: 0., min: 0., max: 1., step: 1. },
    ])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let a = p.get("a").copied().unwrap_or(0.25);
    let long_only = p.get("side").copied().unwrap_or(0.) >= 0.5;
    let c = &x.c;
    let mut e = vec![0.; c.len()];
    for i in 0..c.len() { e[i] = if i == 0 { c[0] } else { e[i - 1] + a * (c[i] - e[i - 1]) }; }
    let mut tgt = vec![0i8; c.len()];
    for i in 0..c.len() {
        let pr = c[i] + (c[i] - e[i]);
        let prev = if i > 0 { tgt[i - 1] } else { 0 };
        tgt[i] = if pr > e[i] { 1 } else if pr < e[i] { if long_only { 0 } else { -1 } } else { prev };
    }
    Some(target_trades(x, &tgt, from, to, true, "predictor crossed the EMA"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn reverses_on_each_predictor_cross() {
        let c: Vec<f64> = (0..300).map(|i| 100. + 5. * (i as f64 / 9.).sin()).collect();
        let rows: Vec<Row> = c.iter().enumerate().map(|(i, &v)| Row { ticker: "T".into(), date: format!("{}", i), open: v, high: v + 1., low: v - 1., close: v, volume: None }).collect();
        let x = Ctx::new(&rows, &rows, &[]);
        let t = run("ehpred_simple", &x, 0, rows.len() - 1, &Params::new()).unwrap();
        assert!(t.len() > 10);
        for w in t.windows(2) { assert_eq!(w[0].exit_i, w[1].legs[0].i); assert_eq!(w[0].dir, -w[1].dir); }
        let mut p = Params::new(); p.insert("side".into(), 1.);
        assert!(run("ehpred_simple", &x, 0, rows.len() - 1, &p).unwrap().iter().all(|z| z.dir > 0.));
        assert!(run("ehpred_x", &x, 0, 5, &Params::new()).is_none());
    }
}

// Instrument admission: John Ehlers (Mesa98 'Predictive Indicators' slides) 2026-11-01 (titles lane K7).
// Admission only for the module REVIEWED lists (authors/titles-1030/K7w/k7_ref.py (header)).
// Runs on actual indices, ETFs and stocks, US and India (index_etf_or_stock, nifty50 basket, declared application).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "ehpred" && reviewed(def.id),
    label: "John Ehlers showed this predictor crossover on futures charts as a teaching example; US and India indices, ETFs and stocks are a declared application.",
    basis: "John Ehlers, Mesa98 v4.1 seminar slides 'Predictive Indicators' (PREDICT.ppt): 'A Simple Predictive Trading System'",
};
