//! Erol Bortucene (with Cynthia Macy): "The Day Trade Forex System" (daytradeforex.com; book id `ebdtf`, incl. the Advanced Training
//! Course), "The Euro Fractal Trading System" (`ebeft`) and "The Forex Profit System" (`ebfps`). Reading:
//! claude-continuation-20261020/authors/bortucene/agents/. Catalogue only: intraday currency systems on 1/5/15/60-minute charts with stops,
//! targets and entry offsets in pips and EUR/USD round-number key levels.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub fn catalogue() -> Vec<StrategyDef> {
    let nt = |id, name, group, book, why, rules| StrategyDef { id, name, group, session: 0, dir: "Long/Short", status: Status::NotTestable(why),
        needs_volume: false, needs_vix: false, needs_series: &[], book, rules, notes: "Held after the 2026-10-24 reading (authors/bortucene/agents).", claim: None };
    vec![
        nt("ebdtf_scalp", "Day Trade Forex System (5-minute EMA 10/50, Bollinger, Parabolic SAR scalps)", "The Day Trade Forex System · Steps 1-3", "ebdtf",
           "5-minute currency charts with 5-20 pip profit targets, tight pip stops and hedging of losing trades; the stop and target sizes are pips",
           &["E|\"trading ... on 5 minute charts, with the goal of taking 5-20 pip profits per trade\""]),
        nt("ebeft_fractal", "Euro fractal breakout at key levels", "The Euro Fractal Trading System · Rules (p.28)", "ebeft",
           "EUR/USD only: entries at round-number 'key levels' +/- 5 pips after a fractal breakout, a 15-pip stop and 20-30 pip targets on 5- and 1-minute charts",
           &["E|\"Always open a position at a key level. +/- 5 pips\"", "X|\"Always set your stop at 15 pips. Always set your limits at one of the key levels\""]),
        nt("ebfps_ema_sar", "Forex Profit System (EMA 10 crossing EMA 25 and 50, Parabolic SAR)", "The Forex Profit System · Steps 1-2", "ebfps",
           "hourly bars confirmed by the 15-minute Parabolic SAR ('Never trade against the 15 min Parabolic SAR'), a stop 'just below the EMA 50' moved 'accordingly'; the SAR settings and the stop rule are not fixed (user option: run on 60-minute SPY/NIFTY bars from the 1-minute data)",
           &["E|\"If the ten crosses the 25 and 50 up from the bottom, you enter your trade long ... Make sure ... the Parabolic SAR is on the bottom\"", "X|\"The best time to exit a trade is when the price crosses back down through all 3 EMA's\""]),
    ]
}

fn mine(id: &str) -> bool { id.starts_with("ebdtf_") || id.starts_with("ebeft_") || id.starts_with("ebfps_") }

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    if mine(id) { Some(("ebdtf_nt", "Bortucene forex systems (held)", "-")) } else { None }
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> { if mine(id) { Some(vec![]) } else { None } }

pub fn run(_id: &str, _x: &Ctx, _from: usize, _to: usize, _p: &Params) -> Option<Vec<Trade>> { None }
