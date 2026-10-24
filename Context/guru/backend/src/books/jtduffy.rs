//! Joseph T. Duffy: Bull's Eye Trading (2001; `jtdbet`), The Trading Advantage (1991; `jtdta`) and Turning Point Analysis in Price
//! and Time (`jtdtpa`). Reading: claude-continuation-20261020/authors/duffy/agents/. Catalogue only. Pulsar 2.0 stays under jtdpls.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub fn catalogue() -> Vec<StrategyDef> {
    let nt = |id, name, group, book, why, rules| StrategyDef { id, name, group, session: 0, dir: "Long/Short", status: Status::NotTestable(why),
        needs_volume: false, needs_vix: false, needs_series: &[], book, rules, notes: "Held after the 2026-10-24 reading (authors/duffy/agents).", claim: None };
    vec![
        nt("jtdbet_confluence", "Confluence of support and resistance (pivots, pivot channel, Fibonacci)", "Bull's Eye Trading", "jtdbet",
           "trades are taken where several levels cluster, judged by the trader; the author: 'what I am teaching you here are techniques, not systems' (p.2)",
           &["F|Pivot channel: 7-day EMA of the daily pivot +/- the 7-day average of (1st high pivot - 1st low pivot) (p.4-5)"]),
        nt("jtdbet_3x5atr", "MACD/stochastic direction filter and 3x5ATR band", "Bull's Eye Trading · Day-trading secrets", "jtdbet",
           "settings are 'around' 10-30-10 and 'somewhere around 20 days'; intraday dip buying without an exit; the author: 'this is not a total day-trading strategy' (p.14)",
           &[]),
        nt("jtdta_gannswing", "Gann mechanical trend indicator and swing exhaustion", "The Trading Advantage · Section II", "jtdta",
           "short swings may be ignored by judgement and the exhaustion pattern is read by 'simple observation'; no entry or exit",
           &["F|the up trend continues until a four bar low is made (a low lower than the lows of the previous three bars) (p.31)"]),
        nt("jtdta_timing", "Fibonacci, astronomical, Square of Nine and geometric timing", "The Trading Advantage · Sections I-III", "jtdta",
           "forecasts from hand-picked turning points, planetary aspects (needs an ephemeris) and lines drawn by hand",
           &[]),
        nt("jtdtpa_levels", "Square of Nine, pivot and high/low-line levels", "Turning Point Analysis in Price and Time", "jtdtpa",
           "support/resistance and time targets combined by judgement; no entry or exit system",
           &[]),
    ]
}

fn mine(id: &str) -> bool { id.starts_with("jtdbet_") || id.starts_with("jtdta_") || id.starts_with("jtdtpa_") }

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    if mine(id) { Some(("jtduffy_nt", "Duffy turning points (held)", "-")) } else { None }
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> { if mine(id) { Some(vec![]) } else { None } }

pub fn run(_id: &str, _x: &Ctx, _from: usize, _to: usize, _p: &Params) -> Option<Vec<Trade>> { None }
