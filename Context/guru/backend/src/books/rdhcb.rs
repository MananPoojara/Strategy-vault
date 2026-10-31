//! Roger Darley, "Optimizing With Hilbert Indicators" (Stocks & Commodities, Nov 2000) and that issue's Traders' Tips
//! (research share USB HDD 1/MetaStock/@Systems & Indicators/@Misc/- Metastock Systems & Indicators/Hilbert Indicators +
//! Info/hillbert_metastock.doc). Book id `rdhcb`. One rule: the Hilbert Channel Breakout System with its printed
//! default inputs (15-bar entry and exit channels). Lane K6 (titles loop, 31 Oct 2026); independent check
//! authors/titles-1030/K6w/k6_ref.py (header).
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::pbwts::{highest, lowest};
use super::rbtie::mk;

/// Admitted ids (authors/titles-1030/K6w/k6_ref.py (header)).
pub const REVIEWED: &[&str] = &["rdhcb_channel"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] The printed inputs EntryVal 15 and ExitVal 15 are not 0, so the channels use fixed 15-bar look-backs and Ehlers' HilbertPeriod is not used (the adaptive form EntryK x Period, used when EntryVal = 0, is not coded here: the article's 'optimum' K ranges are not fixed values). EntryChannel = the highest high of the 15 bars before the current one, ExitChannel = the lowest low of the 15 bars before it. 'Buy' and 'ExitLong' without an order type are TradeStation's next-bar market orders: filled at the next open. One long position; the exit test starts on the entry bar. Equis' MetaStock long/short reversal version is not coded. Daily bars; indices, ETFs and US/India stocks are a declared application; no results printed.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "rdhcb_channel", name: "Hilbert Channel Breakout: buy above the 15-bar high, out below the 15-bar low, next open", group: "Darley Hilbert Channel Breakout",
        session: 0, dir: "Long", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: "rdhcb", rules: &[
            "F|\"Inputs: Price((H+L)/2), EntryVal(15), EntryK(0), ExitVal(15), ExitK(0)\"; EntryChannel = Highest(High, EntryLookBack)[1], ExitChannel = Lowest(Low, ExitLookBack)[1]",
            "E|\"If MarketPosition = 0 and High > EntryChannel then Buy\"",
            "X|\"If MarketPosition = 1 and Low < ExitChannel then ExitLong\""],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    (id == "rdhcb_channel").then_some(("rdhcb_channel", "Darley Hilbert channel breakout", "15/15 bars"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    (id == "rdhcb_channel").then(|| vec![p("entry", "EntryVal, entry channel bars (printed 15)", 15., 2., 100., 1.), p("exit", "ExitVal, exit channel bars (printed 15)", 15., 2., 100., 1.)])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(f64::NAN, |d| d.def));
    let (en, ex) = (highest(&x.h, g("entry").round().max(1.) as usize), lowest(&x.l, g("exit").round().max(1.) as usize));
    let (mut out, mut cur) = (vec![], None::<(usize, f64)>);
    let mut i = from.max(1);
    while i < to {
        match cur {
            None => if x.h[i] > en[i - 1] { cur = Some((i + 1, x.o[i + 1])); },
            Some((e, ep)) => if x.l[i] < ex[i - 1] { out.push(mk(1., e, ep, "open", i + 1, x.o[i + 1], "low below the exit channel", false, i)); cur = None; },
        }
        i += 1;
    }
    if let Some((e, ep)) = cur { out.push(mk(1., e, ep, "open", to, x.c[to], "open", true, to)); }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn channel_breakout_next_open() {
        let b = [(10., 9.), (10., 9.), (10., 9.), (11., 9.5), (11.5, 10.), (10.5, 8.5), (10., 9.)];
        let r: Vec<Row> = b.iter().enumerate().map(|(i, &(h, l))| Row { ticker: "T".into(), date: format!("2001-01-{:02}", i + 1), open: l + 0.5, high: h, low: l, close: h - 0.5, volume: None }).collect();
        let x = Ctx::new(&r, &r, &[]);
        let p: Params = [("entry".to_string(), 3.), ("exit".to_string(), 3.)].into_iter().collect();
        let t = run("rdhcb_channel", &x, 0, r.len() - 1, &p).unwrap();
        // bar 3: high 11 > 10 -> buy bar 4 open 10.5; bar 5: low 8.5 < 9 -> out at bar 6 open 9.5
        assert_eq!(t.iter().map(|z| (z.legs[0].i, z.legs[0].px, z.exit_i, z.exit_px)).collect::<Vec<_>>(), vec![(4, 10.5, 6, 9.5)]);
        assert!(run("rdhcb_x", &x, 0, 6, &p).is_none());
    }
}

// Instrument admission: Roger Darley (Hilbert Channel Breakout) 2026-10-31 (titles lane K6).
// Admission only for the module REVIEWED lists (authors/titles-1030/K6w/k6_ref.py (header)).
// Runs on actual indices, ETFs and stocks, US and India (index_etf_or_stock, nifty50 basket, declared application).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "rdhcb" && crate::books::rdhcb::reviewed(def.id),
    label: "A magazine system for any market with no printed results; indices, ETFs and US/India stocks are a declared application.",
    basis: "Roger Darley, 'Optimizing With Hilbert Indicators', S&C Nov 2000, and Traders' Tips (Hilbert Channel Breakout EasyLanguage), hillbert_metastock.doc",
};
