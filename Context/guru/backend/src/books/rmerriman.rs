//! Raymond A. Merriman: Stock Market Timing (`rmsmt`), The Sun, the Moon and Silver (`rmsms`) and The Ultimate Book on Stock
//! Market Timing Vol.I-IV (`rmubsmt`). Reading: claude-continuation-20261020/authors/merriman/agents/. Catalogue only (cycles and astrology).
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub fn catalogue() -> Vec<StrategyDef> {
    let nt = |id, name, group, book, why, rules| StrategyDef { id, name, group, session: 0, dir: "Long/Short", status: Status::NotTestable(why),
        needs_volume: false, needs_vix: false, needs_series: &[], book, rules, notes: "Held after the 2026-10-24 reading (authors/merriman/agents).", claim: None };
    vec![
        nt("rmubsmt_4yr12", "Four-year cycle investor rule (12% decline in the trough band)", "The Ultimate Book on Stock Market Timing Vol.I · Ch.12 (p.126-127)", "rmubsmt",
           "the 'previous crest' and the four-year-cycle trough time band are identified by the cycle analyst; no exit is given",
           &["E|\"Purchase stocks (invest, buy and hold) when the stock market declines at least 12% from its previous crest, and when it is within the time band for a four-year cycle trough\" (p.126)"]),
        nt("rmubsmt_cycles", "Cycle phases and primary-cycle trading", "The Ultimate Book on Stock Market Timing Vol.I", "rmubsmt",
           "cycle troughs and crests are labelled by the analyst inside time bands and confirmed by judgement",
           &[]),
        nt("rmubsmt_geocosmic", "Geocosmic and solar-lunar correlations", "The Ultimate Book on Stock Market Timing Vol.II-IV", "rmubsmt",
           "astrology (needs a planetary ephemeris); time bands for turns without entries or exits",
           &[]),
        nt("rmsmt_geocosmic", "Geocosmic signatures for trading-cycle turns", "Stock Market Timing", "rmsmt",
           "astrology (needs a planetary ephemeris); no entry or exit",
           &[]),
        nt("rmsms_silver", "Silver cycles and solar/lunar timing", "The Sun, the Moon and Silver", "rmsms",
           "silver futures and astrology; no entry or exit",
           &[]),
    ]
}

fn mine(id: &str) -> bool { id.starts_with("rmsms_") || id.starts_with("rmsmt_") || id.starts_with("rmubsmt_") }

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    if mine(id) { Some(("rmerriman_nt", "Merriman cycles (held)", "-")) } else { None }
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> { if mine(id) { Some(vec![]) } else { None } }

pub fn run(_id: &str, _x: &Ctx, _from: usize, _to: usize, _p: &Params) -> Option<Vec<Trade>> { None }
