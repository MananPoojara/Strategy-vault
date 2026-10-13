//! Anonymous, "The Black or White Trading Model" (three Word documents: "Black-White Pg 1.doc", "Black-White Pg2.doc"
//! with hand-drawn close-channel examples, "Black-White pg 3.doc" weekly version). Book id `bwtm`. Source: research drive
//! "USB HDD 2/tRADING KNO-2/Black or White Trading Model/".
//! The colour rules are mechanical (Pg2 examples 1-3 show support/resistance = the close of the most recent
//! counter-trend close), but the signals are not fully specified, so both models are NotTestable (see AMB).
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "bwtm";

const AMB: &str = "rules incomplete/ambiguous: (1) 'Sell' is never defined as exit-only or stop-and-reverse (no short rule, no long exit other than the sell signal); (2) no rule for when the close after the colour change is not higher (lower): wait for a later pair of consecutive higher closes, or skip the signal; (3) 'use [unchanged closes] when applicable to generate buy and sell signals' does not say whether an unchanged close counts toward or breaks the 'second consecutive' count; (4) 'most recent support/resistance of the last five days': what happens when no counter-trend close occurred in the last five days. No code, trade list or results resolve these";

const DAILY: &[&str] = &[
    "F|Pg2 #1: \"With each higher close, the market is indicating the most recent resistance, with each additional higher close (channel) being white.\"",
    "F|Pg2 #2: \"When the market closes lower this becomes the most recent Support; when the market closes below the most recent Support of the last five days, the close (channel) changes to black.\"",
    "F|Pg2 #3: \"When the market reverses and closes above the most recent resistance of the last five days, the close (channel) changes back to white.\" (example: 80.90, 80.50, 80.00, 80.30 black, 80.70 white = above the 80.30 counter-trend close)",
    "F|Pg2 #4: \"Treat unchanged closings as if they were the same color as the day prior and use when applicable for buy and sell signals.\"",
    "E|Pg1: \"Buy on the second consecutive higher white close after the market has changed from black to white. I would start or include the day it changed from black to white as day one, so if the next day (2nd day) it closed higher.. that would be my buy day on the close.\"",
    "X|Pg2: \"SELL on the Second consecutive lower black close after the market has changed from white to black.\" (Pg1 prints 'lower white close'; Pg2 example: 42.80 support, 42.50 black, sell 42.30)",
];
const WEEKLY: &[&str] = &[
    "F|pg3: \"If prices are moving in a WHITE channel (uptrend) and a weekly close closes lower than the previous week then that lower close is support. Then if the following week closes thru that support to the downside then the channel turns BLACK.\"",
    "X|pg3: \"The second consecutive lower weekly close after the channel turns BLACK would be trend change to the downside and time to sell.\"",
    "E|pg3: \"A BUY signal is just the opposite.\"",
];

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, rules| StrategyDef { id, name, group: "Black or White Trading Model", session: 0, dir: "Long/Short",
        status: Status::NotTestable(AMB), needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules,
        notes: "Close-only channel model; no market, test period or results are given. Would run on daily (weekly = Friday close) index closes if the signal rules were complete.", claim: None };
    vec![
        d("bwtm_daily", "Black or White · daily close model (Pg 1-2)", DAILY),
        d("bwtm_weekly", "Black or White · weekly (Friday close) model (pg 3)", WEEKLY),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match id {
        "bwtm_daily" => Some(("bwtm_black_white", "Black or White Trading Model", "Daily · not testable")),
        "bwtm_weekly" => Some(("bwtm_black_white", "Black or White Trading Model", "Weekly · not testable")),
        _ => None,
    }
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
