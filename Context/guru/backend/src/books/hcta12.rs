//! Jeffrey A. Hirsch & John L. Person, Commodity Trader's Almanac 2012 (Wiley; research drive Seasonality Top Reads/13-
//! Commodity-Trader-s-Almanac-2012/Reading.epub, OEBPS/9781118168691_c01.htm). Book id `hcta12`. Live: 'Short S&P into
//! early January strength' (trade box "JANUARY SHORT S&P 500 MARCH (MARCH) TRADING DAY: 2-HOLD: 12 DAYS"). Same trade
//! conventions as module hcta (CTA 2010): enter at the close of the month's Nth trading day, exit at the close `hold`
//! trading days later. Lane K2 (titles loop, 30 Oct 2026); independent check authors/niksan-k2/admission/k2b_ref.py.
use crate::connors::{claim, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

/// Admitted ids (authors/niksan-k2/admission/k2b_ref.py (header)).
pub const REVIEWED: &[&str] = &["hcta12_sp_jan_short"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] 'On or about the second trading day' read as exactly trading day 2, at its close; held 12 trading days counted from the day after entry, covered at that close (module hcta convention). Book instrument: March S&P 500 futures; the cash index, ETFs, other indices, single stocks and India are a declared application (instrument swap).";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "hcta12_sp_jan_short", name: "January short S&P 500: sell on the 2nd trading day of January, cover 12 trading days later",
        group: "Commodity Trader's Almanac 2012", session: 0, dir: "Short", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: "hcta12",
        rules: &["E|\"JANUARY SHORT S&P 500 MARCH (MARCH) TRADING DAY: 2-HOLD: 12 DAYS\"; \"Selling on or about the second trading day of the New Year\" (ch.1, January)",
            "X|\"and holding for twelve trading sessions\" (ch.1, January)"],
        notes: NOTE, claim: claim("29", "55.2% (16 of 29)", "$82,700 cumulative (March S&P futures)", "12 trading days", "1983-2011 (ch.1, January)") }]
}
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    (id == "hcta12_sp_jan_short").then_some(("hcta12_sp_jan_short", "January short S&P 500", "TD 2, hold 12"))
}
pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    (id == "hcta12_sp_jan_short").then(|| vec![
        ParamDef { key: "td", label: "Enter on the Nth trading day of January (book 2)", def: 2., min: 1., max: 20., step: 1. },
        ParamDef { key: "hold", label: "Hold (trading days after entry, book 12)", def: 12., min: 1., max: 100., step: 1. }])
}
pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let td = p.get("td").copied().unwrap_or(2.).round().max(1.) as usize;
    let hold = p.get("hold").copied().unwrap_or(12.).round().max(1.) as usize;
    let ym = |i: usize| &x.r[i].date[..7];
    let mut out: Vec<Trade> = vec![];
    for f in 1..x.r.len() {
        if ym(f) == ym(f - 1) || &x.r[f].date[5..7] != "01" { continue; }
        let e = f + td - 1;
        if e >= x.r.len() || ym(e) != ym(f) || e < from || e >= to || out.last().is_some_and(|t| e <= t.exit_i) { continue; }
        let (xi, open) = if e + hold <= to { (e + hold, false) } else { (to, true) };
        out.push(Trade { dir: -1., legs: vec![Leg { i: e, px: x.c[e], w: 1. }], exit_i: xi, exit_px: x.c[xi], reason: if open { "open".into() } else { format!("{hold} trading days held · close") },
            entry_kind: "close", signal_i: xi, open, exits: vec![] });
    }
    Some(out)
}

// Instrument admission: Jeffrey A. Hirsch and John L. Person (Commodity Trader's Almanac 2012) 2026-10-30 (titles lane K2) block, moved from instrument_policy.rs on 31 Oct 2026 (lane ST).
// Admission only for the module REVIEWED lists (authors/niksan-k2/admission/k2b_ref.py (header)); kept in one block to merge.
// Runs on actual indices, ETFs and stocks, US and India (index_etf_or_stock, nifty50 basket, declared application).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book=="hcta12" && crate::books::hcta12::reviewed(def.id),
    label: "The Almanac trades March S&P 500 futures; the cash index, ETFs, single stocks and India are a declared application (instrument swap).",
    basis: "Jeffrey A. Hirsch and John L. Person, Commodity Trader's Almanac 2012, ch.1 (January)",
};
