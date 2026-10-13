//! Futures Truth private opinion letter "Simple Harmony" by Murray Ruggiero (analysis by George Pruitt, 1/31/2011).
//! Book id `rgsh`. Commercial system; entry rules are not disclosed. Sources: library/rgsh/.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "rgsh";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef {
        id: "rgsh_simple_harmony", name: "Simple Harmony (Trend Harmony without the adaptive engine)", group: "Futures Truth review", session: 0,
        dir: "Long/Short", status: Status::NotTestable("commercial system: the review discloses only that it is a channel-breakout trend follower using strength and volatility, stop and reverse, 3 parameters, 125-day ramp-up; no rule or parameter value is given"),
        needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK,
        rules: &["E|\"a somewhat simple channel break out based trend following system. Most Long entries are based on market strength and volatility\"", "X|\"in the market 100% of the time so it exits positions on a reversal basis\""],
        notes: "16-market futures basket, nearly $800K net, $116K drawdown (review). Trade lists in SimpleHarmony*.xls are outputs, not rules.",
        claim: None,
    }]
}
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    (id == "rgsh_simple_harmony").then_some(("rgsh", "Simple Harmony", "Not testable"))
}
pub fn params(_id: &str) -> Option<Vec<ParamDef>> { None }
pub fn run(_id: &str, _x: &Ctx, _from: usize, _to: usize, _p: &Params) -> Option<Vec<Trade>> { None }
