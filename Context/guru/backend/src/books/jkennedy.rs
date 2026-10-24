//! Jeffrey Kennedy (Elliott Wave International): The Best of Trader's Classroom (`jfkbotc`), Trader's Classroom Collection Vol.1-4
//! (`jfktcc`) and Trading the Line (`jfkttl`). Reading: claude-continuation-20261020/authors/kennedy/agents/. Catalogue only.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub fn catalogue() -> Vec<StrategyDef> {
    let nt = |id, name, group, book, why, rules| StrategyDef { id, name, group, session: 0, dir: "Long/Short", status: Status::NotTestable(why),
        needs_volume: false, needs_vix: false, needs_series: &[], book, rules, notes: "Held after the 2026-10-24 reading (authors/kennedy/agents).", claim: None };
    vec![
        nt("jfkbotc_wave", "Elliott wave trade set-ups", "The Best of Trader's Classroom", "jfkbotc",
           "every set-up rests on a hand-made Elliott wave count (entries beyond the wave B extreme, stops where the count fails, Fibonacci targets)",
           &[]),
        nt("jfktcc_wave", "Elliott wave patterns and wave-based trades", "Trader's Classroom Collection Vol.1-4", "jfktcc",
           "hand-made wave counts",
           &[]),
        nt("jfktcc_10dayrev", "10-Day Reversal", "Trader's Classroom Collection Vol.3 p.19-20", "jfktcc",
           "the author offers it 'as an alert to a potential trade set-up' (about 60% reliable), not as a trade; no entry order, stop or exit",
           &["F|\"it occurs when prices make a new 10-day price extreme but close in the opposite direction of the 10-day trend\" (Vol.3 p.19)"]),
        nt("jfktcc_macdhook", "MACD Hook, Slingshot and Zero-Line Reversal", "Trader's Classroom Collection Vol.2 Ch.10", "jfktcc",
           "'attempts to penetrate', 'near zero' and swing points are judged; the author: 'It's not a trading system that I blindly follow'; no exit",
           &[]),
        nt("jfktcc_3in1", "Three-in-One bar pattern", "Trader's Classroom Collection Vol.4 p.30-31", "jfktcc",
           "the text says the setup bar engulfs the last three bars but the example has the next three bars inside it; the trade goes with the 'larger trend' judged by the trader; no stop or exit",
           &["E|\"insist on a close above the high or low of the setup bar by the trigger bar before attempting a trade\" (Vol.4 p.31)"]),
        nt("jfkttl_trendline", "Trendline set-ups and the Kennedy channelling technique", "Trading the Line", "jfkttl",
           "trendlines connect points chosen by the trader; no fixed entry, stop or exit",
           &[]),
    ]
}

fn mine(id: &str) -> bool { id.starts_with("jfkbotc_") || id.starts_with("jfktcc_") || id.starts_with("jfkttl_") }

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    if mine(id) { Some(("jkennedy_nt", "Kennedy Elliott wave set-ups (held)", "-")) } else { None }
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> { if mine(id) { Some(vec![]) } else { None } }

pub fn run(_id: &str, _x: &Ctx, _from: usize, _to: usize, _p: &Params) -> Option<Vec<Trade>> { None }
