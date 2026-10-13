//! Chick Goslin, "Intelligent Futures Trading" (1997; scan, 178 pdf pages, printed page = pdf page - 1).
//! Book id `cgift`. Source and checks: library/cgift/. The earlier book of the three-line momentum
//! method (direction 49-day / ten-week MA, timing = 3-day minus 10-day MA, confirming = 13-17-day MA of
//! the timing line). Here the lines are trading guidelines, not a system: the author never tested a
//! mechanical version (p.36-37) and calls the rules "guidelines rather than absolute rules" (p.54).
//! The mechanical form of the method (the point Status) is in the later book -> cgtdd_status. Nothing Live.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "cgift";

macro_rules! nt {
    ($id:expr, $name:expr, $group:expr, $why:expr, $rules:expr) => {
        StrategyDef { id: $id, name: $name, group: $group, session: 0, dir: "Long/Short", status: Status::NotTestable($why),
            needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules: $rules,
            notes: "Read in the scan (library/cgift/README.md). Mechanical form of the same lines: cgtdd_status (Trading Day by Day, 2002).", claim: None }
    };
}

pub fn catalogue() -> Vec<StrategyDef> {
    vec![
        nt!("cgift_timing_line", "Timing line rules (trend up / timing low = buy)", "Ch.5 · The Timing Line (p.33-41)",
            "'low' / 'high' and the buy zone 'plus ten or lower' / sell zone 'minus ten or higher' are in SMR chart units with no formula (p.35, 39); rule 2 is a permission ('be long or be out', p.34); the author: 'I have never tried it ... mechanical buying of the timing line when it turns up and selling when it turns down would not work' (p.36-37)",
            &["E|\"1. Trend up, timing line low - buy. Trend down, timing line high - sell.\" (p.34)",
              "E|\"2. Trend up, timing line pointed up - be long or be out. Trend down, timing line pointed down - be short or be out.\" (p.34)",
              "F|Timing line: \"The average of the closing prices for the past 'X' number of days minus the average of the closing prices for the past 'Y' number of days - with X being two, three or four days and Y being nine, ten or eleven days\" (p.34); direction line: \"SMR uses a 49-day moving average\", \"use a ten-week moving average\" (p.28)"]),
        nt!("cgift_confirming_line", "Confirming line qualifiers", "Ch.5 · The Confirming Line (p.42-47)",
            "'the confirming line are the least precise and require the most interpretation' (p.42); its use is to 'qualify' and 'modify' trades (no entry/exit of its own)",
            &["F|\"simply use a 13- to 17-day moving average of your short-term momentum line\" (p.42)"]),
        nt!("cgift_never_hold_loser", "Never hold a loser overnight against both lines", "Ch.5 (p.36)",
            "exit-only rule with no entry system ('Never, never hold a loser overnight if both the timing line and direction line are going against you', p.36)",
            &["X|\"Never, never hold a loser overnight if both the timing line and direction line are going against you.\" (p.36)"]),
        nt!("cgift_dips_rallies", "Buy dips / sell rallies", "Ch.5 · Trading Techniques (p.24-25)",
            "'two to four days' is a range and no exit is given",
            &["E|\"If the trend is up and the market goes down for two to four days in a row, that is a dip\" (p.24)"]),
        nt!("cgift_divergences", "Price / timing-line divergences", "Ch.5 · Divergences (p.49-53)",
            "chart reading ('there will always be judgment involved', p.49)", &["E|Classic divergence: price reaches or barely surpasses a previous high/low and the timing line does not (p.49)"]),
        nt!("cgift_money_management", "Profit and loss guidelines, stops, checklist", "Ch.5 (p.55-60)",
            "dollar and %-of-margin guidelines are 'only my current guidelines' and change with volatility (p.56); needs exchange margin history; stops 'at the location and direction of the three momentum lines' (p.58)",
            &["X|\"A gain of 50% or more of margin ... is a good profit\" (p.57); \"A loss of 33% - or more - of margin is a fairly big loss\" (p.58)"]),
        nt!("cgift_relative_strength", "Buy the strongest / sell the weakest of a group", "Ch.8 · Final Comments on Trading (p.170-171)",
            "eyeball comparison of line and price patterns across a futures universe",
            &["E|\"buying the strongest market in a group when going long and selling the weakest market in a group when going short\" (p.171)"]),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    id.starts_with("cgift_").then_some(("cgift_nt", "Goslin Intelligent Futures Trading (not testable)", "-"))
}
pub fn params(_id: &str) -> Option<Vec<ParamDef>> { None }
pub fn run(_id: &str, _x: &Ctx, _from: usize, _to: usize, _p: &Params) -> Option<Vec<Trade>> { None }
