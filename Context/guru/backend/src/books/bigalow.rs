//! Stephen W. Bigalow - Candlestick Forum e-books. Book ids `swbmcs` (The Major Candlestick Signals), `swbbpp` (Big Profit
//! Patterns Using Candlestick Signals and Gaps), `swbtc` (How to Use TC2000 to Find Candlestick Trades), `swbcee`
//! (Profitable Candlestick Entry and Exit Strategies). Reading: claude-continuation-20261020/authors/bigalow/agents/.
//! Catalogue only: the author states "Numeric formulas do not work for Candlestick formations" and sets stops "by visual
//! analysis" (Entry and Exit Strategies p.23-24); every signal needs a "definable trend" and judged candle sizes.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, group, book, why, rules| StrategyDef { id, name, group, session: 0, dir: "Long/Short", status: Status::NotTestable(why), needs_volume: false,
        needs_vix: false, needs_series: &[], book, rules, notes: "Read 2026-10-24 (authors/bigalow/agents).", claim: None };
    vec![
        d("swbmcs_major", "Twelve major candlestick signals with stochastic confirmation", "The Major Candlestick Signals", "swbmcs",
          "each signal needs a 'definable' prior trend, a 'long' candle or a 'very small' shadow and next-day confirmation, none defined in numbers; no exit is given",
          &["E|E.g. bullish engulfing after a definable downtrend; hammer with a lower shadow at least twice the body, confirmed by a strong next day"]),
        d("swbbpp_gaps", "Gap set-ups: kicker, fry-pan bottom, J-hook, San-ku", "Big Profit Patterns Using Candlestick Signals and Gaps", "swbbpp",
          "patterns and their entries are read on charts; no numeric definitions or exits", &["E|Buy a gap up at the start of a new trend after a candle buy signal"]),
        d("swbtc_scan", "TC2000 scan: oversold stocks with a candle buy signal", "How to Use TC2000 to Find Candlestick Trades", "swbtc",
          "a cross-sectional scan of the whole stock universe followed by a manual pick of four or five charts; the signal formulas are left to a later e-book; no exit",
          &["F|Stochastics below 20, price above $5, a candle buy signal; choose the best few by chart"]),
        d("swbcee_entries", "Candle entry tactics and signal-retracement stop", "Profitable Candlestick Entry and Exit Strategies", "swbcee",
          "the author: 'Numeric formulas do not work for Candlestick formations' and 'percentage stop points cannot be applied' (p.23-24); stops by visual analysis",
          &["E|Buy stop at the previous close after a lower open on a confirmed signal", "X|Exit when the signal is fully retraced or a sell signal appears"]),
    ]
}
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    (id.starts_with("swbmcs_") || id.starts_with("swbbpp_") || id.starts_with("swbtc_") || id.starts_with("swbcee_")).then_some(("swb_nt", "Bigalow candlestick signals (held)", "-"))
}
pub fn params(_id: &str) -> Option<Vec<ParamDef>> { None }
pub fn run(_id: &str, _x: &Ctx, _from: usize, _to: usize, _p: &Params) -> Option<Vec<Trade>> { None }
