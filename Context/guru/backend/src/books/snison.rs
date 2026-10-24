//! Steve Nison: Japanese Candlestick Charting Techniques (1991; book id `snjcct`), The Candlestick Course (2003; `sncc`) and
//! Candlestick Charting Basics (slides; `snccb`). Reading: claude-continuation-20261020/authors/nison/agents/. Catalogue only:
//! the patterns need a trend and candle sizes judged by eye and have no exits (same finding as Bigalow).
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub fn catalogue() -> Vec<StrategyDef> {
    let nt = |id, name, group, book, why, rules| StrategyDef { id, name, group, session: 0, dir: "Long/Short", status: Status::NotTestable(why),
        needs_volume: false, needs_vix: false, needs_series: &[], book, rules, notes: "Held after the 2026-10-24 reading (authors/nison/agents).", claim: None };
    vec![
        nt("snjcct_reversal", "Candlestick reversal patterns (hammer, engulfing, stars, harami, doji)", "Japanese Candlestick Charting Techniques · Ch.4-6, 8", "snjcct",
           "the trend before each pattern and 'long' or 'small' bodies are judged by eye (e.g. an engulfing pattern needs 'a clearly definable trend'); no entry order, stop or exit is given - stops are chosen chart by chart",
           &["F|Hammer: real body at the upper end of the range, lower shadow at least twice the height of the real body, no or a very short upper shadow, after a decline (p.29)"]),
        nt("snjcct_continuation", "Candlestick continuation patterns (windows, tasuki, three methods)", "Japanese Candlestick Charting Techniques · Ch.7", "snjcct",
           "windows are support and resistance areas; no entry or exit rule",
           &[]),
        nt("snjcct_western", "Candles with Western techniques", "Japanese Candlestick Charting Techniques · Ch.10-15", "snjcct",
           "candle signals are combined with trendlines, retracements, moving averages and oscillators by judgement (confluence); moving-average crosses are illustrations without fixed lengths or exits",
           &[]),
        nt("sncc_guidelines", "Candle patterns and trading guidelines (workbook)", "The Candlestick Course · Ch.1-4", "sncc",
           "the author: 'Candle charts are best used as a tool, not a system' (p.122); trend direction, risk/reward and stops are judged per chart",
           &[]),
        nt("snccb_basics", "Candlestick charting basics (slides)", "Candlestick Charting Basics", "snccb",
           "definitions and chart examples only",
           &[]),
    ]
}

fn mine(id: &str) -> bool { id.starts_with("sncc_") || id.starts_with("snccb_") || id.starts_with("snjcct_") }

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    if mine(id) { Some(("snison_nt", "Nison candlesticks (held)", "-")) } else { None }
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> { if mine(id) { Some(vec![]) } else { None } }

pub fn run(_id: &str, _x: &Ctx, _from: usize, _to: usize, _p: &Params) -> Option<Vec<Trade>> { None }
