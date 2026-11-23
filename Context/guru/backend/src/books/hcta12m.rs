//! Jeffrey A. Hirsch & John L. Person, Commodity Trader's Almanac 2012 (Wiley; research drive Seasonality Top Reads/13-Commodity-Trader-s-Almanac-2012/
//! Reading.epub, OEBPS/9781118168691_c01..c12.htm): the 3 printed seasonal trade boxes of the 40 that are not already cards of `hcta12` (January S&P short) or of `hcta` (CTA 2010: 36 boxes have the same month, trading day, hold and side)
//! ("MONTH LONG|SHORT MARKET (CONTRACT) TRADING DAY: N-HOLD: D DAYS"). Book id `hcta12m`. Lane K3 (24 Nov 2026). Each card runs only on the
//! Norgate continuous contract of its own market (futures.rs cards). Independent check authors/conv-1123/k3w/hcta12m_ref.py.
use crate::connors::{Ctx, Leg, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "hcta12m";
const NOTE: &str = "[Printed trade box; unverified] Readings (labelled, 3): (1) 'on or about' the Nth trading day = exactly the Nth session of that calendar month in the continuous contract's own bars, at its close; (2) the HOLD of D days = covered at the close D sessions after the entry session; (3) the box names one contract month ('MARCH', 'JULY', ...) and the Norgate continuous contract (nearby, back-adjusted) stands in for it, a labelled adaptation, so a trade can cross a roll. One trade per year; no stop. A trade still open on the last bar is marked at its close.";

/// (id, month, direction, trading day, hold)
pub const ROWS: &[(&str, u32, f64, usize, usize)] = &[
    ("hcta12m_02_l_cl_10", 2, 1.0, 10, 60),
    ("hcta12m_06_l_sb_11", 6, 1.0, 11, 32),
    ("hcta12m_12_l_zw_5", 12, 1.0, 5, 20),
];
pub fn row(id: &str) -> Option<&'static (&'static str, u32, f64, usize, usize)> { ROWS.iter().find(|r| r.0 == id) }
pub fn reviewed(id: &str) -> bool { row(id).is_some() }

pub fn catalogue() -> Vec<StrategyDef> {
    vec![
        StrategyDef { id: "hcta12m_02_l_cl_10", name: "February long crude oil: enter on trading day 10, hold 60 days", group: "Commodity Trader's Almanac 2012 · seasonal trades", session: 0, dir: "Long", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK,
            rules: &["E|\"FEBRUARY LONG CRUDE OIL (JULY) TRADING DAY: 10-HOLD: 60 DAYS\" (ch.2); the trade is taken on the 10th trading day of February at the close", "X|held 60 trading days, covered at that close (the box's HOLD)"], notes: NOTE, claim: None },
        StrategyDef { id: "hcta12m_06_l_sb_11", name: "June long sugar: enter on trading day 11, hold 32 days", group: "Commodity Trader's Almanac 2012 · seasonal trades", session: 0, dir: "Long", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK,
            rules: &["E|\"JUNE LONG SUGAR (OCTOBER) TRADING DAY: 11-HOLD: 32 DAYS\" (ch.6); the trade is taken on the 11th trading day of June at the close", "X|held 32 trading days, covered at that close (the box's HOLD)"], notes: NOTE, claim: None },
        StrategyDef { id: "hcta12m_12_l_zw_5", name: "December long wheat: enter on trading day 5, hold 20 days", group: "Commodity Trader's Almanac 2012 · seasonal trades", session: 0, dir: "Long", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK,
            rules: &["E|\"DECEMBER LONG CBOT WHEAT (MAY) TRADING DAY: 5-HOLD: 20 DAYS\" (ch.12); the trade is taken on the 5th trading day of December at the close", "X|held 20 trading days, covered at that close (the box's HOLD)"], notes: NOTE, claim: None },
    ]
}
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { reviewed(id).then_some(("hcta12m", "Commodity Trader's Almanac 2012 · seasonal trades", "calendar")) }
pub fn params(_id: &str) -> Option<Vec<crate::connors::ParamDef>> { None }

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, _p: &Params) -> Option<Vec<Trade>> {
    let &(_, mon, dir, td, hold) = row(id)?;
    if from >= to || to >= x.r.len() { return None; }
    let ym = |i: usize| &x.r[i].date[..7];
    let mut out: Vec<Trade> = vec![];
    for f in 1..x.r.len() {
        if ym(f) == ym(f - 1) || x.r[f].date[5..7].parse::<u32>().ok() != Some(mon) { continue; }
        let e = f + td - 1;
        if e >= x.r.len() || ym(e) != ym(f) || e < from || e >= to { continue; }
        let (xi, open) = if e + hold <= to { (e + hold, false) } else { (to, true) };
        out.push(Trade { dir, legs: vec![Leg { i: e, px: x.c[e], w: 1. }], exit_i: xi, exit_px: x.c[xi], reason: if open { "open".into() } else { format!("{hold} trading days held · close") },
            entry_kind: "close", signal_i: xi, open, exits: vec![] });
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_row_runs_on_a_long_series() {
        let r = super::super::sctraub::tests::bars(2600);
        let x = Ctx::new(&r, &r, &[]);
        assert_eq!(catalogue().len(), ROWS.len());
        for (id, mon, dir, _, hold) in ROWS {
            let t = run(id, &x, 0, r.len() - 1, &Params::new()).unwrap();
            assert!(t.iter().all(|z| z.dir == *dir && z.exit_i >= z.legs[0].i && (z.open || z.exit_i - z.legs[0].i == *hold)) && !t.is_empty() || *mon > 12);
        }
    }
    /// hand-run dump for the pandas reference: HC12M_DIR=<norgate dir> HC12M_OUT=<file> cargo test --release hcta12m_dump -- --ignored
    #[test]
    #[ignore]
    fn hcta12m_dump() {
        use std::io::Write;
        let mut data = std::collections::BTreeMap::new();
        crate::futures::load(std::path::Path::new(&std::env::var("HC12M_DIR").unwrap()), &mut data);
        let mut f = std::io::BufWriter::new(std::fs::File::create(std::env::var("HC12M_OUT").unwrap()).unwrap());
        writeln!(f, "id,ticker,dir,entry_date,entry_px,exit_date,exit_px,open").unwrap();
        for def in catalogue() {
            let sym = SYMS.iter().find(|s| s.0 == def.id).unwrap().1;
            let tk = crate::futures::MARKETS.iter().find(|m| m.sym == sym).map(crate::futures::ticker).unwrap();
            let Some(rows) = data.get(&tk) else { continue };
            let x = Ctx::new(rows, &[], &[]);
            for z in run(def.id, &x, 0, rows.len() - 1, &Params::new()).unwrap() {
                writeln!(f, "{},{tk},{},{},{},{},{},{}", def.id, z.dir, rows[z.legs[0].i].date, z.legs[0].px, rows[z.exit_i].date, z.exit_px, z.open).unwrap();
            }
        }
    }
}

/// (id, Norgate symbol)
pub const SYMS: &[(&str, &str)] = &[
    ("hcta12m_02_l_cl_10", "&CL"),
    ("hcta12m_06_l_sb_11", "&SB"),
    ("hcta12m_12_l_zw_5", "&ZW"),
];
