//! Stuart Evens, "Bollinger Bands", Technical Analysis of Stocks & Commodities V.17:3 (March 1999) pp.116-123 (research share
//! Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V17/C03/019BOLL.pdf). Book id `scevbb`.
//! Lane K16 (titles loop, 2 Nov 2026), to-code row 251; independent check authors/titles-1030/K16w/k16_ref.py.
//! Also holds `ms_book`, the MetaStock System Tester loop (enter long / close long / enter short / close short, next open) the
//! lane-K16 Evens modules share.
use crate::connors::{rsi, Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::msws::sma;
use super::scchvid::pstd;
use super::sctraub::next_open;

pub const REVIEWED: &[&str] = &["scevbb_rsi"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

/// MetaStock System Tester semantics: at each close the held side's close rule is tested, then (if flat) the entry rules;
/// orders fill at the next open. `f(t)` = (enter long, close long, enter short, close short); `shorts` false = long only.
pub fn ms_book(x: &Ctx, from: usize, to: usize, shorts: bool, f: impl Fn(usize) -> (bool, bool, bool, bool)) -> Vec<Trade> {
    next_open(x, from, to, "exit rule", |t, side| {
        let (ol, cl, os, cs) = f(t);
        let w = if (side > 0.0 && cl) || (side < 0.0 && cs) { 0.0 } else { side };
        if w != 0.0 { w } else if ol { 1.0 } else if os && shorts { -1.0 } else { 0.0 }
    })
}

const NOTE: &str = "[Declared readings] The printed MetaStock rules (Figure 5): Bollinger Bands = 20-day simple average of closes +/- 2 standard deviations (MetaStock's population deviation), RSI(20) as MetaStock (Wilder). Enter long: Low <= lower band and RSI(20) < 30; close long: High >= upper band and RSI(20) > 70; enter short: High >= upper band and RSI(20) > 70; close short: Low <= lower band and RSI(20) < 30. The article's tests executed 'at the opening price of the stock on the day after the signal was generated' (next open). Long and short is the default; side = 1 is the article's long-only case. Article (points, 1/1993-12/1998, $5/$10 commissions): Dow Chemical long and short +50.438 (buy and hold +34.713), long only +38.113; Fluor long and short +65.075, long only +31.712 (buy and hold +2.025). The author tested stocks; runs on indices and ETFs, US and India, are a declared application.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "scevbb_rsi", name: "Evens Bollinger Bands + RSI(20): buy a lower-band touch with RSI < 30, short an upper-band touch with RSI > 70, next open", group: "S&C: Bollinger Bands (Evens, V.17:3)", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "scevbb", rules: &[
            "E|\"Enter Long: Low <= BBandBot(C,20,S,2) and RSI(20) < 30 ... Enter Short: High >= BBandTop(C,20,S,2) and RSI(20) > 70\" (V.17:3, PDF p.4, Figure 5)",
            "X|\"Close Long: High >= BBandTop(C,20,S,2) and RSI(20) > 70 ... Close Short: Low <= BBandBot(C,20,S,2), and RSI(20) < 30\" (PDF p.4)",
            "F|\"account equity was calculated based on trades being executed at the opening price of the stock on the day after the signal was generated\" (PDF p.6)"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    reviewed(id).then_some(("scevbb_rsi", "Evens Bollinger Bands with RSI (S&C)", "BB(20,2) + RSI(20) 30/70"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    reviewed(id).then(|| vec![p("side", "0 = long and short; 1 = long only", 0., 0., 1., 1.)])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let (m, sd, r) = (sma(&x.c, 20), pstd(&x.c, 20), rsi(&x.c, 20));
    let shorts = p.get("side").copied().unwrap_or(0.) < 0.5;
    Some(ms_book(x, from, to, shorts, |t| {
        let (up, dn) = (m[t] + 2.0 * sd[t], m[t] - 2.0 * sd[t]);
        let (lo, hi) = (x.l[t] <= dn && r[t] < 30.0, x.h[t] >= up && r[t] > 70.0);
        (lo, hi, hi, lo)
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bands_with_rsi() {
        let r = super::super::sctraub::tests::bars(3000);
        let x = Ctx::new(&r, &r, &[]);
        let t = run("scevbb_rsi", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(t.iter().all(|z| z.legs[0].px == x.o[z.legs[0].i]));
    }
}

// Instrument admission: S&C Evens Bollinger Bands 2026-11-02 (titles lane K16).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "scevbb" && reviewed(def.id),
    label: "Stuart Evens tested the rules on Dow Chemical and Fluor (1993-1998). Runs on indices, ETFs and other stocks, US and India, are a declared application.",
    basis: "Technical Analysis of Stocks & Commodities V.17:3 pp.116-123 (Bollinger Bands), Figure 5 MetaStock rules; independent check authors/titles-1030/K16w/k16_ref.py",
};
