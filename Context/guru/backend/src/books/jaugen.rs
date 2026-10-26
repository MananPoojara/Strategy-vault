//! Jeff Augen: "The Option Trader's Handbook" (FT Press 2004/2009; book id `jaoth`), "The Volatility Edge in Options Trading" (2008; `javeo`),
//! "Trading Options at Expiration" (2009; `jatoe`). Reading: claude-continuation-20261020/authors/augen/agents/. Catalogue only: option
//! structures on single stocks chosen from volatility analysis, expiration-day trades on minute-by-minute stock and option prices.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub fn catalogue() -> Vec<StrategyDef> {
    let nt = |id, name, group, book, why, rules| StrategyDef { id, name, group, session: 0, dir: "Long/Short", status: Status::NotTestable(why),
        needs_volume: false, needs_vix: false, needs_series: &[], book, rules, notes: "Held after the 2026-10-24 reading (authors/augen/agents).", claim: None };
    vec![
        nt("jaoth_structures", "Option structures (covered, complex and ratio trades)", "The Option Trader's Handbook · Ch.2-5", "jaoth",
           "position structures and adjustments chosen by the trader from option prices; no entry or exit rule on the underlying",
           &[]),
        nt("javeo_price_spikes", "Price-spike volatility analysis and volatility trades", "The Volatility Edge in Options Trading", "javeo",
           "price changes in standard deviations are an analysis tool; trades are option structures on selected stocks",
           &["F|daily price change measured in standard deviations of recent volatility"]),
        nt("jatoe_expiration", "Expiration-day strike pinning and day trades", "Trading Options at Expiration · Ch.1-3", "jatoe",
           "single-stock candidates are selected from statistics and traded on expiration Thursday/Friday with minute-level option prices; the vault has daily index/ETF option chains and minute bars for four indices only",
           &["E|expiration Thursday/Friday trades on stocks near a strike (Ch.3)"]),
    ]
}

fn mine(id: &str) -> bool { id.starts_with("jaoth_") || id.starts_with("jatoe_") || id.starts_with("javeo_") }

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    if mine(id) { Some(("jaugen_nt", "Augen options (held)", "-")) } else { None }
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> { if mine(id) { Some(vec![]) } else { None } }

pub fn run(_id: &str, _x: &Ctx, _from: usize, _to: usize, _p: &Params) -> Option<Vec<Trade>> { None }
