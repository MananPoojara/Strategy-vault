//! Robert Borowski (Rapid Forex): Forex Surfing (`rbfsurf`), Forex Sailing (`rbfsail`) and Forex Scalping (`rbfscalp`).
//! Reading: claude-continuation-20261020/authors/borowski/agents/. Catalogue only (intraday forex, pip sizes, lines drawn by hand).
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub fn catalogue() -> Vec<StrategyDef> {
    let nt = |id, name, group, book, why, rules| StrategyDef { id, name, group, session: 0, dir: "Long/Short", status: Status::NotTestable(why),
        needs_volume: false, needs_vix: false, needs_series: &[], book, rules, notes: "Held after the 2026-10-24 reading (authors/borowski/agents).", claim: None };
    vec![
        nt("rbfsurf_waves", "Forex wave and channel surfing", "Forex Surfing", "rbfsurf",
           "waves, channels and triangles are drawn by the trader on 1- and 5-minute forex charts; stops and targets are in pips",
           &[]),
        nt("rbfsail_sex", "S.E.X. lines trend trading", "Forex Sailing", "rbfsail",
           "the author says the 5/20/60 simple-exponential crosses 'ARE NOT MEANT TO BE USED ALONE' (p.58); entries and trailing stops are judged on the chart",
           &[]),
        nt("rbfscalp_1min", "One-minute forex scalping", "Forex Scalping", "rbfscalp",
           "1-minute forex bars with 10-pip stops; ranges and trend lines are drawn by the trader",
           &[]),
    ]
}

fn mine(id: &str) -> bool { id.starts_with("rbfsail_") || id.starts_with("rbfscalp_") || id.starts_with("rbfsurf_") }

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    if mine(id) { Some(("rborowski_nt", "Rapid Forex (held)", "-")) } else { None }
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> { if mine(id) { Some(vec![]) } else { None } }

pub fn run(_id: &str, _x: &Ctx, _from: usize, _to: usize, _p: &Params) -> Option<Vec<Trade>> { None }
