//! Robert Fischer (with Jens Fischer) - Fibonacci books. Book ids `rfcfcp` (Candlesticks, Fibonacci, and Chart Pattern
//! Trading Tools, 2003), `rffaps` (Fibonacci Applications and Strategies for Traders, 1993), `rfnft` (The New Fibonacci
//! Trader, 2001, and its Workbook). Reading: claude-continuation-20261020/authors/fischer/agents/. Catalogue only: the five
//! complete systems all rest on a minimum swing size in fixed futures points; held until a size rule for other
//! instruments is chosen. PHI-ellipses, channels, spirals and candle patterns are judgement.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

const SWING: &str = "the swing size that defines every swing point is a fixed number of futures points (300 DJ EuroStoxx 50 points, 80 S&P 500 points, 400 British-pound points, 1.80 yen). The book gives no rule for other markets or price levels, so a percent or ATR swing size for indices, ETFs and stocks would be our choice, not the book's; held until the user picks one";

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, group, book, why, rules| StrategyDef { id, name, group, session: 0, dir: "Long/Short", status: Status::NotTestable(why), needs_volume: false,
        needs_vix: false, needs_series: &[], book, rules, notes: "Read 2026-10-24 (authors/fischer/agents).", claim: None };
    vec![
        d("rfcfcp_dtdb", "Double-top / double-bottom swing breakout", "Ch.3 (p.45-51)", "rfcfcp", SWING,
          &["E|Long on a close above the last swing peak, short below the last swing valley (p.45-51)", "X|Stop on a close beyond the previous opposite swing point; target one swing size; opposite signal reverses"]),
        d("rfcfcp_fib618", "Fibonacci 61.8% correction system", "Ch.4 (p.52-66); also The New Fibonacci Trader Ch.3", "rfcfcp", SWING,
          &["E|After an impulse of at least the minimum swing and a 61.8% correction, buy stop at the previous day's high (p.57-60)", "X|Stop at the low of the impulse's first day; target 0.618 x impulse; trail the lowest low of the previous 4 days"]),
        d("rfcfcp_patterns", "Candlestick entries, chart patterns, PHI-channels and PHI-ellipses", "Ch.1-2, 5-9", "rfcfcp",
          "'end of a downtrend' is never defined and there is no exit besides the stop; the authors say PHI-ellipse signals cannot be computerised (p.146)", &["E|Judgement"]),
        d("rffaps_breakout", "Swing breakout with 0.618 target", "Ch.4 (p.54-58)", "rffaps", SWING,
          &["E|Buy a break of the previous swing high (p.54-58)", "X|Target breakout level + 0.618 x the prior swing; stop the previous 2-day low, trailed"]),
        d("rffaps_wave3", "Wave-3 extension counter-trend trade", "Ch.5 (p.73-86); band from the NFT Workbook (p.60)", "rffaps", SWING,
          &["E|At 1.618 x wave 1, sell on a break of the previous two lows or a close below the low of the highest day (p.73-86)", "X|Stop one tick above the previous high; target 50% of the move or a 2-day trailing stop"]),
        d("rffaps_timegoal", "Fibonacci time-goal-day system", "Ch.7 (p.103-125)", "rffaps", SWING,
          &["E|After a passed time-goal day, sell on a close below the low of the highest day or a break of the previous 4 lows (p.103-125)", "X|Fixed-point stop from the product table (p.107); reverse on the opposite signal"]),
        d("rfnft_tools", "Time-goal days, 3- and 5-wave extensions, PHI-spirals", "The New Fibonacci Trader", "rfnft",
          "'We do not explicitly impose entry rules ... exit rules' (p.276); the complete correction system is the same as rfcfcp_fib618", &["E|Judgement"]),
    ]
}
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    (id.starts_with("rfcfcp_") || id.starts_with("rffaps_") || id.starts_with("rfnft_")).then_some(("rf_nt", "Fischer Fibonacci systems (held)", "-"))
}
pub fn params(_id: &str) -> Option<Vec<ParamDef>> { None }
pub fn run(_id: &str, _x: &Ctx, _from: usize, _to: usize, _p: &Params) -> Option<Vec<Trade>> { None }
