//! Larry Williams, "Accumulation and Distribution" / "On Selection and Timing" (Telerate TAG XVII seminar paper,
//! Las Vegas 1995, pp.875-882, with "When a Dollar is Not a Dollar: the Williams True Trend Index"). Book id `lwad`.
//! Source copy: Books/Larry Williams/Larry Williams - Accumulation and Distribution (INO).pdf (23-page scan).
//! Catalogue only: every pattern in the paper gives an entry without an exit. README: library/lwad/README.md.
use crate::connors::{ParamDef, Params, Status, StrategyDef, Ctx, Trade};

pub const BOOK: &str = "lwad";

macro_rules! off {
    ($id:expr, $name:expr, $status:expr, $rules:expr) => {
        StrategyDef { id: $id, name: $name, group: "TAG XVII 1995", session: 0, dir: "Long/Short", status: $status, needs_volume: false,
            needs_vix: false, needs_series: &[], book: BOOK, rules: $rules, notes: "", claim: None }
    };
}

pub fn catalogue() -> Vec<StrategyDef> {
    use Status::{NeedsData, NotTestable};
    vec![
        off!("lwad_sp_open_close", "S&P: down close above the open, buy tomorrow at today's high", NotTestable("No exit or stop is stated (printout 5: 64 trades, 81%, $35,700, S&P 06/93 test); the bond version needs T-bond futures."),
            &["E|\"If today's close is lower than yesterday's close, yet today's close is higher than today's open, we should buy the S&P 500 tomorrow (unless tomorrow is a Friday) at the value of today's high\"", "X|Not stated"]),
        off!("lwad_pctr_patterns", "%R < 20 + down close: buy at its high; %R > 80 + up close: sell Fridays at its low", NotTestable("No exit or stop is stated (printouts 7-8: 35 trades 97% $31,925; 19 trades 84% $10,200)."),
            &["E|\"demand that a 10-day %R be less than 20 ... a close that is lower than the previous day's low. Should that take place, buy the following day at the high of the down close day\" (any day but a Friday)", "X|Not stated"]),
        off!("lwad_bonds_monday", "Buy the S&P on Monday's open if the 18-day MA of bonds is rising", NeedsData("Needs T-bond futures; the exit is also not stated (printout 11: 313 trades 83% $151,225)."),
            &["F|\"If the 18-day moving average of Bonds is >/= to the previous day\"", "E|\"buy the S&P on Monday's opening\"", "X|Not stated"]),
        off!("lwad_day_of_month", "Days-of-the-month count: range-based entries on calendar days", NotTestable("Exit not stated and the order direction of the calendar-day-1 example is ambiguous (printouts 12-15)."),
            &["E|\"taking 50% of the range of day 1 (... calendar day 1) and subtracting that from the next day's opening makes $14,275\"; day 2 sell at 15%, day 3 at 45%", "X|Not stated"]),
        off!("lwad_indicators", "A/D line, True Trend Index, sentiment, Oops!, gaps, channel breaks, contract expiration", NotTestable("Divergence and trend-line readings (judgement) and entry ideas without exits. Oops! = lwlts_oops_*; selection rules (commercials net long, public <= 35%) need COT/sentiment data."),
            &["F|LWAccDist: up close adds (C - TrueLow), down close subtracts (TrueHigh - C) (TradeStation code p.4)", "F|True Trend: cumulative (net change / true range)"]),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    id.starts_with("lwad_").then_some(("lwad", "Accumulation & Distribution (TAG 1995)", ""))
}
pub fn params(_id: &str) -> Option<Vec<ParamDef>> { None }
pub fn run(_id: &str, _x: &Ctx, _from: usize, _to: usize, _p: &Params) -> Option<Vec<Trade>> { None }
