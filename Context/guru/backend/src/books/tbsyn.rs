//! Tom Bierovic, "Synergetic Trading" (SynergyTrader.com e-book, c.2001, 47 pp). Book id `tbsyn`.
//! Source: research drive `USB HDD 2/tRADING KNO-2/Tom Bierovic - Synergetic Trading/Synergy_Trader.pdf` (text layer, read in full).
//! Catalogue only: nothing in it is a mechanical rule. Notes: library/tbsyn/README.md.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, group, reason, rules, notes| StrategyDef { id, name, group, session: 0, dir: "Long/Short",
        status: Status::NotTestable(reason), needs_volume: false, needs_vix: false, needs_series: &[], book: "tbsyn", rules, notes, claim: None };
    vec![
        d("tbsyn_patterns", "Classic bar-chart patterns (flags, triangles, wedges, head & shoulders)", "Classic Bar-Chart Patterns",
          "discretionary: the lines are drawn by eye and the book's exits are \"might be\" choices (p.40-41)",
          &["E|Breakout above the top of a bullish flag / below a bearish flag; neckline breaks of head & shoulders; triangle and wedge breakouts (p.5-22)",
            "X|\"The stop usually risks between three and five percent of the stock's price. The exact location of the stop is based on the candlestick chart\" (p.40); targets = pattern height added to the breakout (p.41)"],
          "Swing high = a bar with a lower high on each side (p.4) is the only exact definition."),
        d("tbsyn_candles", "Japanese candlestick patterns", "Japanese Candlestick Patterns",
          "no trade rules: \"tall\" candles and the \"downward/upward wave\" are not defined and no entry or exit is given (p.23-37)",
          &["F|Piercing line / dark cloud cover, engulfing lines, hammer / hanging man, harami, doji, morning / evening star, windows (p.25-37)"],
          "Definitions only."),
        d("tbsyn_ema_trail", "20-day EMA trend and trailing stop", "Exponential Moving Averages / Trade Management",
          "no entry rule; the trailing stop is one of three options (\"might be set below the most recent swing low, below the two-day low, or below the 20-day EMA\", p.41)",
          &["X|\"I use a 20-day EMA ... as a trailing stop, frequently setting my stop below the EMA for a long position\" (p.39)"],
          "Money management: 15% of closed equity per trade at 2-1 margin (p.40)."),
    ]
}
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    id.starts_with("tbsyn_").then_some(("tbsyn", "Synergetic Trading", "Not testable"))
}
pub fn params(_: &str) -> Option<Vec<ParamDef>> { None }
pub fn run(_: &str, _: &Ctx, _: usize, _: usize, _: &Params) -> Option<Vec<Trade>> { None }
