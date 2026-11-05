//! Larry Connors & Cesar Alvarez, "How Markets Really Work", 2nd ed. (Bloomberg/Wiley, 2012). Book id "hmrw".
//! Chapters 2-11 are every-occurrence statistics (each qualifying day is a separate 1-share trade held 1 day,
//! 2 days or 1 week, holding periods overlapping); the book gives no position rules for them, so they are not
//! strategies and are documented in library/hmrw/README.md only. Chapter 12 is a stock-universe statistic.
//! The one complete strategy (Ch. 13) trades S&P 500 member stocks: since lane K56 (5 Nov 2026, EP5) it runs in
//! books::rotation on the vault's us_sp500 basket (SPY calendar) and nse500 basket (NIFTY 50) as a declared application
//! (check authors/titles-1030/K56w/k56_ref.py). `run` returns None.
#![allow(dead_code)]
use crate::connors::*;

pub const BOOK: &str = "hmrw";


pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef {
        id: "hmrw_sp500_rotation",
        name: "S&P 500 weekly pullback rotation",
        group: "Ch 13 · Creating a Sample Strategy",
        session: 0,
        dir: "Long",
        status: Status::Live,
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
            "F|[Declared reading] RSI = Wilder RSI(2) (rule 6 gives no period; the book's RSI chapter is the 2-period RSI, p.147); HV100 = sample standard deviation of 100 daily log returns x sqrt(252) x 100; Friday = the week's last trading day",
        ],
        notes: "EP5 basket ranking (lane K56). Book: p.159-162, Table 13.1 (1/1/2001-9/30/2011, 1 cent/share each way): CAGR 10.48% vs S&P 0.04%, max daily DD -14.98% vs -55.25%, Sharpe 0.81, correlation 0.41; the book's universe is every stock in the S&P 500 at the time, with dividends and delistings (rule 1). Declared application: the stocks of the vault's us_sp500 basket on the SPY calendar (to 11 Sep 2020; India: nse500 on NIFTY 50). Membership caveat: the basket is the index's member list of 8 Sep 2020 (Norgate filename snapshot) applied to every past date, not dated membership: survivorship bias, large over 2001-2011 (stocks dropped from the index are missing, later entrants are present before they joined). Price only (split-adjusted, no dividends). Readings: the index filter on the calendar's own close (SPY > its 200-day SMA; NIFTY 50 for India); stock above its 200-day SMA, down two closes in a row, RSI(2) < 15, HV100 < 35; at the week's last close, up to 10 qualifiers with the highest HV100 (ties by ticker) are bought at 10% of the equity at that close (after the week's exits), at most the free cash (no borrowing); a position whose exit day has no bar for the stock stays held to its next bar and keeps its slot; every position is sold at the next week's last close, so a stock that qualifies again is sold and rebought (one trade per week). Unused slots stay in cash (no interest).",
        claim: claim("-", "-", "CAGR 10.48%", "1 week", "1/1/01 - 9/30/11, S&P 500 stocks"),
    }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    (id == "hmrw_sp500_rotation").then_some(("hmrw_sp500_rotation", "S&P 500 rotation (How Markets Really Work)", "S&P 500 basket"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    (id == "hmrw_sp500_rotation").then(|| vec![
        ParamDef { key: "rsi_n", label: "RSI period (rule 6 states none; 2 = the book's 2-period RSI, labelled reading)", def: 2., min: 2., max: 14., step: 1. },
        ParamDef { key: "rsi", label: "RSI below (rule 6)", def: 15., min: 5., max: 30., step: 5. },
        ParamDef { key: "hv", label: "100-day HV below (rule 7)", def: 35., min: 20., max: 60., step: 5. },
    ])
}

pub fn run(_id: &str, _x: &Ctx, _from: usize, _to: usize, _p: &Params) -> Option<Vec<Trade>> {
    None
}
