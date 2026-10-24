//! Larry Pesavento - book ids `lppps` (Profitable Patterns for Stock Trading), `lpopp` (The Opening Price Principle, with
//! Peggy MacKay, 2000; the share holds the Italian edition), `lprmc` (The Ruth Miller Method of Trading Corn), `lpastro`
//! (Astro Cycles). Reading: claude-continuation-20261020/authors/pesavento/agents/. Catalogue only: the ratio patterns start
//! from swing points chosen on the chart (no swing-size rule, as with Fischer), the opening-price rule is a statistic used
//! with judgement, and the corn and astro works need futures data and an ephemeris.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub fn catalogue() -> Vec<StrategyDef> {
    use Status::*;
    let d = |id, name, group, book, st, rules| StrategyDef { id, name, group, session: 0, dir: "Long/Short", status: st, needs_volume: false,
        needs_vix: false, needs_series: &[], book, rules, notes: "Read 2026-10-24 (authors/pesavento/agents).", claim: None };
    vec![
        d("lppps_patterns", "Gartley 222, AB=CD, butterfly and three-drives patterns", "Profitable Patterns for Stock Trading", "lppps",
          NotTestable("the X-A-B-C-D swing points are chosen on the chart; the book gives no minimum swing size, so any automatic swing rule would be ours"),
          &["E|Enter at the pattern's completion point (e.g. D at .786 of XA) with a stop beyond it"]),
        d("lpopp_opening", "Opening Price Principle with pattern entries", "The Opening Price Principle", "lpopp",
          NotTestable("a statistic (the open lies within 20% of the day's high or low about 70% of the time) used with judgement: 'Use your common sense. Exercise your trading judgment' (p.19); no entry or exit rule"),
          &["F|Trade away from an open that sits near the day's extreme, at a pattern level, with risk set per trade"]),
        d("lprmc_corn", "Ruth Miller corn method: Moon-Uranus aspects and Square of Nine ranges", "The Ruth Miller Method of Trading Corn", "lprmc",
          NeedsData("corn futures prices and an ephemeris; the Square-of-Nine levels are chosen by judgement"), &["E|Trade corn at Square-of-Nine levels on Moon-Uranus aspect days, 5-cent stops"]),
        d("lpastro_cycles", "Planetary aspect timing", "Astro Cycles", "lpastro",
          NotTestable("astrological cycle observations without a trading rule"), &["F|Planetary aspects as turning-point windows"]),
    ]
}
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    (id.starts_with("lppps_") || id.starts_with("lpopp_") || id.starts_with("lprmc_") || id.starts_with("lpastro_")).then_some(("lp_nt", "Pesavento patterns (held)", "-"))
}
pub fn params(_id: &str) -> Option<Vec<ParamDef>> { None }
pub fn run(_id: &str, _x: &Ctx, _from: usize, _to: usize, _p: &Params) -> Option<Vec<Trade>> { None }
