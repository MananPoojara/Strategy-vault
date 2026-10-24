//! Howard Abell: "Digital Day Trading" (Dearborn 1999; book id `haddt`) and "Spread Trading" (`haspr`). Reading:
//! claude-continuation-20261020/authors/abell/agents/. Catalogue only: Taylor buy/sell-day swings, inside-day and constricted-range
//! breakouts without exits, and futures spreads. The Outer Game of Trading (with Robert Koppel) was read under Koppel.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub fn catalogue() -> Vec<StrategyDef> {
    let nt = |id, name, group, book, why, rules| StrategyDef { id, name, group, session: 0, dir: "Long/Short", status: Status::NotTestable(why),
        needs_volume: false, needs_vix: false, needs_series: &[], book, rules, notes: "Held after the 2026-10-24 reading (authors/abell/agents).", claim: None };
    vec![
        nt("haddt_method", "Digital Day Trading method (Taylor swing days, inside/constricted/wide-range days, two-day tests)", "Digital Day Trading · Ch.9 (p.107-166)", "haddt",
           "patterns are described with judgement terms ('the smallest range of the past several days', 'considerably larger', 'be prepared to trade the breakouts in either direction') and no exit; intraday stock trading via market makers",
           &["E|\"Inside days ... should be traded by buying the breakout above the previous day's high and selling a breakout below the previous day's low\" (Ch.9)"]),
        nt("haddt_interviews", "Interviews and trading psychology", "Digital Day Trading · Ch.1-8, 10", "haddt",
           "interviews (Jamail, Shalowitz, Kershner) and principles; no rule",
           &[]),
        nt("haspr_spreads", "Futures spread trading", "Spread Trading", "haspr",
           "intra- and inter-market futures spreads need futures contract prices; the copy is a .djvu file this machine cannot read, so it was triaged by title only",
           &[]),
    ]
}

fn mine(id: &str) -> bool { id.starts_with("haddt_") || id.starts_with("haspr_") }

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    if mine(id) { Some(("habell_nt", "Abell day trading (held)", "-")) } else { None }
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> { if mine(id) { Some(vec![]) } else { None } }

pub fn run(_id: &str, _x: &Ctx, _from: usize, _to: usize, _p: &Params) -> Option<Vec<Trade>> { None }
