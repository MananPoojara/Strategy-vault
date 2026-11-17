//! Jay Kaeppel, interview "Searching for Clues with Jay Kaeppel" (by Thom Hartle), Technical Analysis of Stocks & Commodities V.15:5
//! (1997) pp.225-234, sidebar "Refinement 1/2" (research share Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008)
//! (traders.com)/VOLUMES/V15/C05/SEARCHI.pdf). Book id `sckaep`. Lane C6 (second pass, 17 Nov 2026); independent check
//! authors/convert-1117/C6/ref/c6_ref.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::msws::sma;
use super::sctraub::next_open;

pub const REVIEWED: &[&str] = &["sckaep_9_50_80"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings, 2 unstated details, R4 #339; unverified] The sidebar's four-state rule: 1 go long when 9d > 50d and close > 80d; 2 go flat when 9d < 50d and close > 80d; 3 go short when 9d < 50d and close < 80d; 4 go flat when 9d > 50d and close < 80d. Readings: (1) 9d, 50d and 80d are simple moving averages of the close (the sidebar names day counts only); (2) when a comparison is exactly equal the position is left as it is. Signals are read at the close and filled at the next open. The sidebar applies it to 17 futures markets 1977-1997 (P/L tables, no trade list; coffee uses 9 and 20 days, a labelled option here: fast 9, mid 20); the vault runs it on indices, ETFs and stocks, US and India, as a declared application. The other models in the interview (gold-stock index and bullion, discount rate minus T-bill, Dow yield) need series the vault does not hold.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "sckaep_9_50_80", name: "Kaeppel 9/50/80-day trend rule: long above both, short below both, flat when the 80-day disagrees (next open)", group: "S&C: Searching for Clues with Jay Kaeppel (V.15:5)", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "sckaep", rules: &[
            "E|\"Apply the indicator using the following rules: 1 Go long when 9d > 50d and close > 80d  2 Go flat when 9d < 50d and close > 80d  3 Go short when 9d < 50d and close < 80d  4 Go flat when 9d > 50d and close < 80d\" (V.15:5 sidebar, 'First, calculate a trend-following indicator ... Start with a simple moving average crossover (nine-day and 50-day)')",
            "X|Rules 2 and 4 take the position flat; rules 1 and 3 reverse or enter (the four states are the whole rule).",
            "F|\"For the coffee market, a simple moving average crossover nine and 20 days was used instead of nine and 50 days.\" (Sidebar Figure 4 note)"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { reviewed(id).then_some(("sckaep_9_50_80", "Kaeppel 9/50/80 trend rule (S&C)", "9, 50 and 80 days")) }
pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    reviewed(id).then(|| vec![p("fast", "Fast average, days (printed 9)", 9., 2., 60., 1.), p("mid", "Middle average, days (printed 50; coffee 20)", 50., 3., 200., 1.), p("slow", "Close compared with the average of, days (printed 80)", 80., 3., 300., 1.)])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let g = |k: &str, d: f64| p.get(k).copied().unwrap_or(d).round().max(2.) as usize;
    let (a, b, c) = (sma(&x.c, g("fast", 9.)), sma(&x.c, g("mid", 50.)), sma(&x.c, g("slow", 80.)));
    Some(next_open(x, from, to, "flat signal", |t, side| {
        if !(a[t].is_finite() && b[t].is_finite() && c[t].is_finite()) { return side; }
        if a[t] > b[t] && x.c[t] > c[t] { 1.0 }
        else if a[t] < b[t] && x.c[t] > c[t] { 0.0 }
        else if a[t] < b[t] && x.c[t] < c[t] { -1.0 }
        else if a[t] > b[t] && x.c[t] < c[t] { 0.0 }
        else { side }
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn kaeppel_goes_long_short_and_flat() {
        let r = super::super::sctraub::tests::bars(2500);
        let x = Ctx::new(&r, &r, &[]);
        let t = run("sckaep_9_50_80", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(t.iter().any(|z| z.dir > 0.0) && t.iter().any(|z| z.dir < 0.0));
    }
}

// Instrument admission: S&C Kaeppel 9/50/80 2026-11-17 (lane C6 second pass).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "sckaep" && reviewed(def.id),
    label: "Jay Kaeppel's 9/50/80-day rule is printed with P/L tables for 17 futures markets 1977-1997 (no trade list). Runs on indices, ETFs and stocks, US and India, are a declared application.",
    basis: "Technical Analysis of Stocks & Commodities V.15:5 pp.225-234 (Searching for Clues with Jay Kaeppel, sidebar); independent check authors/convert-1117/C6/ref/c6_ref.py",
};
