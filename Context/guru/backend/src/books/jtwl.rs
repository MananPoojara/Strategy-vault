//! N. Jegadeesh & S. Titman, "Returns to Buying Winners and Selling Losers: Implications for Stock Market Efficiency",
//! Journal of Finance 48(1), March 1993, p.65-91. Book id `jtwl`. Source: research drive "USB HDD 2/tRADING KNO-2/
//! Narasim Jegadeesh, Sheridam Titman - .../Returns to Buying Winners & Selling Losers.pdf" (JSTOR scan, read in place;
//! the copy into Books/ was refused - COPY PENDING, see library/jtwl/README.md). Page refs are J. Finance pages.
//! Every strategy ranks the whole NYSE/AMEX cross-section into deciles each month (CRSP 1965-1989) and holds
//! overlapping equal-weighted winner/loser portfolios; the engine runs one instrument at a time (NSE baskets are
//! per-symbol tickers, no monthly cross-sectional ranking or portfolio book), so nothing is Live and `run` returns None.
//! Same family as cjlmom (Chan, Jegadeesh & Lakonishok 1996), which is also NeedsData.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "jtwl";

const XS: &str = "needs a monthly cross-sectional stock universe (CRSP NYSE/AMEX daily returns 1965-1989, all stocks with returns over the past J months) ranked into deciles each month, and a portfolio engine holding K overlapping equal-weighted vintages; the vault runs single instruments only (NSE baskets are per-symbol tickers, not a ranked cross-section)";
const NOTE: &str = "Paper period January 1965 - December 1989 (Table I); Section VII repeats the test on 1927-1964. Monthly returns compound daily CRSP returns; portfolios rebalanced monthly to equal weights (buy-and-hold versions were 'very similar', p.68).";

macro_rules! nd {
    ($id:expr, $name:expr, $rules:expr) => {
        StrategyDef { id: $id, name: $name, group: "Sec. I-II · Relative strength portfolios (Table I)", session: 0, dir: "Long/Short", status: Status::NeedsData(XS),
            needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules: $rules, notes: NOTE, claim: None }
    };
}

pub fn catalogue() -> Vec<StrategyDef> {
    vec![
        nd!("jtwl_jk", "J-month / K-month winners minus losers (no skip)",
            &["F|\"At the beginning of each month t the securities are ranked in ascending order on the basis of their returns in the past J months. Based on these rankings, ten decile portfolios are formed that equally weight the stocks contained in the top decile, the second decile, and so on.\" (p.68); J, K in {3, 6, 9, 12} (16 strategies)",
              "E|\"In each month t, the strategy buys the winner portfolio and sells the loser portfolio, holding this position for K months.\" (p.68)",
              "X|\"In addition, the strategy closes out the position initiated in month t - K. Hence, under this trading strategy we revise the weights on 1/K of the securities in the entire portfolio in any given month and carry over the rest from the previous month.\" (p.68)",
              "F|Published (Table I Panel A, p.70): 6/6 buy-sell 0.95% per month (t 3.07); best 12/3 1.31% per month; 3/3 0.32% (t 1.10, the only insignificant one)"]),
        nd!("jtwl_jk_skip", "J-month / K-month winners minus losers, one-week skip",
            &["F|\"a second set of 16 strategies that skip a week between the portfolio formation period and the holding period\" (p.68)",
              "E|Same ranking and K-month overlapping holding as jtwl_jk, formed one week after the J-month ranking period ends (Table I Panel B)",
              "F|Published (Table I Panel B, p.70): 6/6 buy-sell 1.10% per month (t 3.61); 12/3 1.49% per month (t 4.28)"]),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "jtwl_jk" => ("jtwl_momentum", "Jegadeesh-Titman momentum", "No skip"),
        "jtwl_jk_skip" => ("jtwl_momentum", "Jegadeesh-Titman momentum", "One-week skip"),
        _ => return None,
    })
}
pub fn params(_id: &str) -> Option<Vec<ParamDef>> { None }
pub fn run(_id: &str, _x: &Ctx, _from: usize, _to: usize, _p: &Params) -> Option<Vec<Trade>> { None }
