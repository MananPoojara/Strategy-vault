//! Anonymous excerpt "50 Bar.pdf" (3 pp, trading-software-collection.com repack): "Try a Simple Entry-Exit Strategy"
//! (50-day high/low midpoint) followed by six day-trading entry methods. Book id `fbs50`. Source: research drive
//! "USB HDD 2/tRADING KNO-2/50 Bar Trading Strategy/50 Bar.pdf". Nothing here is a complete mechanical rule.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "fbs50";

const MID: &str = "incomplete/ambiguous: the text calls it only \"one part of a simple trading strategy\"; 'price' (close or intraday), the order type/fill and whether 'sell' means exit or reverse to short are not stated";
const DAY: &str = "no exit and/or not mechanical: entry ideas for intraday bars only (methods 1-2 need tick counts, 3-4 are chart judgement - ABC correction, 'level of significance' - and 5-6 are commentary); none states an exit";

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, status, rules: &'static [&'static str]| StrategyDef { id, name, group: "50 Bar.pdf", session: 0, dir: "Long/Short", status,
        needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules,
        notes: "Close to kftsm_moving_channel (moving-channel midpoint) already in the vault; recorded so the source is marked as seen.", claim: None };
    vec![
        d("fbs50_midpoint", "50-day high/low midpoint line", Status::NotTestable(MID), &[
            "F|p.1: \"Take the value of the highest high during the last 50 days and the value of the lowest low during the last 50 days. Average these two prices daily.\"",
            "E|p.1: \"When price rises above the line, buy.\"",
            "X|p.1: \"When price moves below the line, sell.\"",
        ]),
        d("fbs50_daytrade_entries", "Day-trading entry methods 1-6", Status::NotTestable(DAY), &[
            "E|1. \"the present bar's high is less than the low of two bars ago. It is a good selling entry\" (buy: low above the high two bars ago)",
            "E|2. \"Upticks are greater than Downticks by a factor of say 175% or 150%, perhaps 125%, the close is greater than the open and the close is greater than the previous close\"",
            "E|3. ABC correction; 4. three-bar high/low 'if we have achieved a mathematical level of significance'; 5. news and gaps; 6. sharp trending market",
        ]),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    id.starts_with("fbs50_").then_some(("fbs50_50bar", "50 Bar Trading Strategy", "Not testable"))
}
pub fn params(_id: &str) -> Option<Vec<ParamDef>> { None }
pub fn run(_id: &str, _x: &Ctx, _from: usize, _to: usize, _p: &Params) -> Option<Vec<Trade>> { None }

#[cfg(test)]
mod tests {
    #[test]
    fn not_testable() {
        assert!(super::catalogue().iter().all(|s| matches!(s.status, crate::connors::Status::NotTestable(_)) && super::family_of(s.id).is_some()));
    }
}
