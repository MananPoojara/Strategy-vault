//! Markus Heitkoetter, "The Complete Guide to Day Trading" (BookSurge 2008; text PDF, printed pages).
//! Book id `hkcgdt`. Source and checks: library/hkcgdt/. A how-to-build-a-strategy book: the "popular
//! trading approaches" (p.135-146) are entry signals only ("Please note that we just defined an entry
//! signal. You still need to apply profit targets and stop losses", p.139), with "e.g." parameters on
//! intraday charts; the exit chapter (p.151-170) is a menu of stop/target types. Nothing Live.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "hkcgdt";

macro_rules! nt {
    ($id:expr, $name:expr, $why:expr, $rules:expr) => {
        StrategyDef { id: $id, name: $name, group: "Part 2 · Your Trading Strategy", session: 0, dir: "Long/Short", status: Status::NotTestable($why),
            needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules: $rules,
            notes: "Read in the text layer (library/hkcgdt/README.md). Meant for intraday charts (10-, 15-, 60-minute examples).", claim: None }
    };
}

const NO_EXIT: &str = "entry signal only: 'You still need to apply profit targets and stop losses' (p.139); parameters are examples ('e.g.'); intraday charts";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![
        nt!("hkcgdt_ma_close", "Close crosses a moving average", NO_EXIT,
            &["E|\"A buy signal is generated when the closing price moves above the moving average. A sell signal is generated when the closing price dips below the moving average.\" (p.136); period: \"such as a 5- or 10-day\" (p.136); \"only apply it in trending markets\" (p.137)"]),
        nt!("hkcgdt_ma_cross", "Fast / slow moving-average crossover (e.g. 14 / 20)", NO_EXIT,
            &["E|\"a 'fast' moving average (e.g. 14 bars) and a 'slow' moving average (e.g. 20 bars)\"; \"A buy signal is generated when the fast-moving average crosses the slow moving average from above\" (p.138, wording as printed)"]),
        nt!("hkcgdt_turtle20", "20-bar high / low breakout", NO_EXIT,
            &["E|\"A buy signal is generated when the current prices move higher than the high of the previous 20 bars. A sell signal is generated when the current prices move lower than the low of the previous 20 bars.\" (p.139)"]),
        nt!("hkcgdt_macd", "MACD signal-line crossover (12/26/9)", NO_EXIT,
            &["E|\"A buy signal is generated when the signal line ... crosses the MACD ... from below\" (p.141)"]),
        nt!("hkcgdt_wpr", "Williams %R 80 / 20", NO_EXIT,
            &["E|\"A sell signal is generated when the indicator has a value above 80. A buy signal is generated when the indicator has a value below 20.\" (p.142), 14 bars"]),
        nt!("hkcgdt_rsi", "RSI 70 / 30 crossings", NO_EXIT,
            &["E|\"A sell signal is generated when the RSI crosses the 70-line ... from above. A buy signal is generated when the RSI crosses the 30-line ... from below.\" (p.144)"]),
        nt!("hkcgdt_bollinger", "Bollinger band fade (21, 2)", NO_EXIT,
            &["E|\"A buy signal is generated when prices move below the lower Bollinger Band. A sell signal is generated when prices move above the upper Bollinger Band.\" (p.145)"]),
        nt!("hkcgdt_exits", "Stop-loss, profit-target, trailing and time-stop menu", "a menu of exit types with example values ($ amount, 1% of price, 50% of ATR, support/resistance, 3x the bar interval), not tied to an entry (p.151-170)",
            &["X|\"Stop loss and profit-taking exit rules can be expressed in four ways\" (p.151)"]),
        nt!("hkcgdt_300_200", "$300 target / $200 stop strategy", "the entry rules of the strategy are not given in this book (p.12-14)",
            &["X|\"a profit target of $300 per contract and a stop loss of $200 per contract\" (p.12)"]),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    id.starts_with("hkcgdt_").then_some(("hkcgdt_nt", "Heitkoetter Complete Guide (not testable)", "-"))
}
pub fn params(_id: &str) -> Option<Vec<ParamDef>> { None }
pub fn run(_id: &str, _x: &Ctx, _from: usize, _to: usize, _p: &Params) -> Option<Vec<Trade>> { None }
