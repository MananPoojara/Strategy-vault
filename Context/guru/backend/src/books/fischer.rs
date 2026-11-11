//! Robert Fischer (with Jens Fischer) - Fibonacci books. Book ids `rfcfcp` (Candlesticks, Fibonacci, and Chart Pattern
//! Trading Tools, 2003), `rffaps` (Fibonacci Applications and Strategies for Traders, 1993), `rfnft` (The New Fibonacci
//! Trader, 2001, and its Workbook). Reading: claude-continuation-20261020/authors/fischer/agents/. Catalogue only: the five
//! systems rest on a minimum swing size in fixed futures points; R4 #277 (1) makes the default the percent those points
//! were at the book's own test, but lane K77 (12 Nov 2026) found each one still held for its own reason (per card).
//! PHI-ellipses, channels, spirals and candle patterns are judgement.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, group, book, why, rules| StrategyDef { id, name, group, session: 0, dir: "Long/Short", status: Status::NotTestable(why), needs_volume: false,
        needs_vix: false, needs_series: &[], book, rules, notes: "Read 2026-10-24 (authors/fischer/agents).", claim: None };
    vec![
        d("rfcfcp_dtdb", "Double-top / double-bottom swing breakout", "Ch.3 (p.45-51)", "rfcfcp", "Lane K77 (R4 #277 (1)): the swing default would be the percent of price the book's 300 points were at its own test (DJ EuroStoxx 50 futures, Aug 2001 - Aug 2002), but the vault holds no EuroStoxx 50 prices, so that percent cannot be derived from data (Table 4.1's three rounded trade prices, 3,727 / 3,500 / 2,700, are not a period level and disagree with each other: exit 3,550 vs re-entry 3,500), and the 3-trade table cannot be re-run. Held",
          &["E|Long on a close above the last swing peak, short below the last swing valley (p.45-51)", "X|Stop on a close beyond the previous opposite swing point; target one swing size; opposite signal reverses"]),
        d("rfcfcp_fib618", "Fibonacci 61.8% correction system", "Ch.4 (p.52-66); also The New Fibonacci Trader Ch.3", "rfcfcp", "Lane K77 (R4 #277 (1)): the S&P sample rules (p.60) do not say when the 4-day trailing stop starts, and the stop is given two ways (p.55: the peak or valley before the entry; p.60: the low of the impulse's starting day). The plainest reading (zigzag swings of 80 S&P points on daily highs and lows, trail from the day after the entry, stop at the impulse start) on SPX Jan-Nov 2000 gives 17 trades and 7 winners against Table 4.3's 9 trades and 8 winners (p.62), and the table's prices (1,427.50, 1,449.80, 1,517.50) are not SPX prints (futures-like ticks). No reading fixed by the book reproduces the table. Held",
          &["E|After an impulse of at least the minimum swing and a 61.8% correction, buy stop at the previous day's high (p.57-60)", "X|Stop at the low of the impulse's first day; target 0.618 x impulse; trail the lowest low of the previous 4 days"]),
        d("rfcfcp_patterns", "Candlestick entries, chart patterns, PHI-channels and PHI-ellipses", "Ch.1-2, 5-9", "rfcfcp",
          "'end of a downtrend' is never defined and there is no exit besides the stop; the authors say PHI-ellipse signals cannot be computerised (p.146)", &["E|Judgement"]),
        d("rffaps_breakout", "Swing breakout with 0.618 target", "Ch.4 (p.54-58)", "rffaps", "Lane K77 (R4 #277 (1)): the book's test is British-pound futures with a 400-point swing (10/1991 - 5/1992); the vault holds no pound prices to turn the 400 points into a percent at the test period, and the entry/exit columns of the 7-trade table are illegible in the scan. Held",
          &["E|Buy a break of the previous swing high (p.54-58)", "X|Target breakout level + 0.618 x the prior swing; stop the previous 2-day low, trailed"]),
        d("rffaps_wave3", "Wave-3 extension counter-trend trade", "Ch.5 (p.73-86); band from the NFT Workbook (p.60)", "rffaps", "Lane K77 (R4 #277 (1)): examples only (no trade table) on soybeans, the British pound and the yen, none of them held in the vault for the percent derivation; the cancellation of a setup is not stated and the p.77 target example disagrees with itself ($6.24 computed vs $6.39 printed). Held",
          &["E|At 1.618 x wave 1, sell on a break of the previous two lows or a close below the low of the highest day (p.73-86)", "X|Stop one tick above the previous high; target 50% of the move or a 2-day trailing stop"]),
        d("rffaps_timegoal", "Fibonacci time-goal-day system", "Ch.7 (p.103-125)", "rffaps", "Lane K77 (R4 #277 (1)): the stop is a 'price square' drawn on the chart (side = five business days, p.123), so it depends on the chart's scale; the p.107 product table's S&P line (filter 400, stop 400) is a 1993 futures setting with no S&P test to derive a percent from (the tests are on the D-mark and the pound). Held",
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
