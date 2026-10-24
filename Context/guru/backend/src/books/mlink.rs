//! Marcel Link: "Trading Without Gambling" (Wiley 2003; book id `mltwg`). Reading: claude-continuation-20261020/authors/link/agents/.
//! A trading-plan book: its only signal example is a moving-average crossover with no lengths; held card only. Link's High Probability
//! Trading rules live in books/mlhpt.rs.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub fn catalogue() -> Vec<StrategyDef> {
    let nt = |id, name, group, book, why, rules| StrategyDef { id, name, group, session: 0, dir: "Long/Short", status: Status::NotTestable(why),
        needs_volume: false, needs_vix: false, needs_series: &[], book, rules, notes: "Held after the 2026-10-25 reading (authors/link/agents).", claim: None };
    vec![
        nt("mltwg_ma_cross", "Moving-average crossover in the direction of the major trend", "Trading Without Gambling · Ch.5 Trading strategies", "mltwg",
           "an illustration: the averages' lengths, the 'major trend' test and the exit signal are not given; the stop sits 'just below the lows of the previous move' (chart reading)",
           &["E|\"a simple moving average crossover system that only takes trades in the direction of the major trend. It buys at the close of the day when the faster moving average line crosses over the slower one\" (ch.5)"]),
    ]
}

fn mine(id: &str) -> bool { id.starts_with("mltwg_") }

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    if mine(id) { Some(("mlink_nt", "Link, Trading Without Gambling (held)", "-")) } else { None }
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> { if mine(id) { Some(vec![]) } else { None } }

pub fn run(_id: &str, _x: &Ctx, _from: usize, _to: usize, _p: &Params) -> Option<Vec<Trade>> { None }
