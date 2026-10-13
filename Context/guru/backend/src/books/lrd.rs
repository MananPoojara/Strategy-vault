//! A Strategy For Playing Large-Range Days (Larry Connors, TradingMarkets article). Book id "lrd".
//! A shorter reprint of Advanced Trading Strategies ch. 27. The article gives an options trade with no exit,
//! and statistical claims with no entry or exit, so neither is a backtestable rule: both are catalogued with
//! the article's text so the vault shows them, and the ATS book module carries the testable 2-SD material.
#![allow(unused_imports, dead_code)]
use crate::connors::*;

pub const BOOK: &str = "lrd";

const COMBO: &[&str] = &[
    "F|The market closed two standard deviations from its previous close (daily SD = 100-day historical volatility / √256 × price)",
    "F|Skip in strongly trending markets: ADX 25 or higher",
    "E|Sell a naked combination: calls one standard deviation above the large-range day's high, puts one standard deviation below its low",
    "X|No exit, stop, expiry or size given: \"If the market moves sideways the premiums on both sides will collapse.\"",
];
const STUDY: &[&str] = &[
    "F|A close two standard deviations from the previous close: \"approximately one out of every twenty trading days\"",
    "F|Works best in non-trending markets (ADX below 25)",
    "X|Claim: the next day's direction is a coin flip",
    "X|Claim: \"the market often closed near the closing price of the large-move day three to four days later\"",
];

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, status, rules, notes| StrategyDef {
        id, name, group: "Large-Range Days", session: 0, dir: "Long/Short", status, needs_volume: false, needs_vix: false,
        needs_series: &[], book: BOOK, rules, notes, claim: None,
    };
    vec![
        d("lrd_large_range_day_combo", "Large-Range Days · sell the naked combination", Status::NotTestable("needs option chains; the article gives no exit"), COMBO,
          "Article's only trade. Tested on T-bond futures over two years (no dates). Table 1 prints √265 but its own numbers only work with √256. The same idea, with 44 dated 2-SD bond days, is Advanced Trading Strategies ch. 27."),
        d("lrd_large_range_day_base_rate", "Large-Range Days · follow-through study", Status::NotTestable("event study: the article states claims, not entry and exit rules"), STUDY,
          "Recorded so the vault shows the article's claims; it is not a trading system. Measurable on daily OHLC (NIFTY included) as a study, not as a backtest."),
    ]
}

pub fn params(_id: &str) -> Option<Vec<ParamDef>> {
    None
}

pub fn run(_id: &str, _x: &Ctx, _from: usize, _to: usize, _p: &Params) -> Option<Vec<Trade>> {
    None
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match id {
        "lrd_large_range_day_combo" => Some(("lrd", "Large-Range Days", "Naked combination")),
        "lrd_large_range_day_base_rate" => Some(("lrd", "Large-Range Days", "Follow-through study")),
        _ => None,
    }
}
