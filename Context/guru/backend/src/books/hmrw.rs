//! Larry Connors & Cesar Alvarez, "How Markets Really Work", 2nd ed. (Bloomberg/Wiley, 2012). Book id "hmrw".
//! Chapters 2-11 are every-occurrence statistics (each qualifying day is a separate 1-share trade held 1 day,
//! 2 days or 1 week, holding periods overlapping); the book gives no position rules for them, so they are not
//! strategies and are documented in library/hmrw/README.md only. Chapter 12 is a stock-universe statistic.
//! The one complete strategy (Ch. 13) trades S&P 500 member stocks, which the vault does not have.
#![allow(dead_code)]
use crate::connors::*;

pub const BOOK: &str = "hmrw";

const SP500: &str = "needs daily OHLC + dividends/splits for every S&P 500 member stock since 1/1/2001 with point-in-time index membership (incl. removed and delisted names); the vault has index data only";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef {
        id: "hmrw_sp500_rotation",
        name: "S&P 500 weekly pullback rotation",
        group: "Ch 13 · Creating a Sample Strategy",
        session: 0,
        dir: "Long",
        status: Status::NeedsData(SP500),
        needs_volume: false,
        needs_vix: false,
        needs_series: &[],
        book: BOOK,
        rules: &[
            "F|(p159) \"We took the S&P 500 index universe since January 1, 2001. All stocks traded in the index universe were included.\"",
            "F|\"The simulated trades were done once a week each Friday on the close.\"",
            "F|\"the S&P must have been above its 200-day moving average on the trade date\"; \"both the S&P 500 index and the individual stock must be above the 200-day MA\"",
            "E|\"we require the stock to be down two days in a row (yesterday and today)\"; \"the RSI must be under 15\"; \"only trade stocks whose 100-day historical volatility is below 35\"",
            "E|\"On the close on Friday buy up to 10 percent per position with up to 10 total positions. If more than 10 stocks qualify, take the ones that have the highest 100-day historical volatility\"",
            "X|\"Exit the next Friday on the close and rotate into the next round of 10 S&P 500 stocks.\"",
        ],
        notes: "p.159-162, Table 13.1 (1/1/2001-9/30/2011, 1 cent/share each way): CAGR 10.48% vs S&P 0.04%, max daily DD -14.98% vs -55.25%, Sharpe 0.81, correlation 0.41. The RSI period in rule 6 is not stated (the book's 2-period RSI is the likely one); to be resolved when the stock data arrives.",
        claim: claim("-", "-", "CAGR 10.48%", "1 week", "1/1/01 - 9/30/11, S&P 500 stocks"),
    }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    (id == "hmrw_sp500_rotation").then_some(("hmrw_sp500_rotation", "S&P 500 rotation (How Markets Really Work)", "data missing"))
}

pub fn params(_id: &str) -> Option<Vec<ParamDef>> {
    None
}

pub fn run(_id: &str, _x: &Ctx, _from: usize, _to: usize, _p: &Params) -> Option<Vec<Trade>> {
    None
}
