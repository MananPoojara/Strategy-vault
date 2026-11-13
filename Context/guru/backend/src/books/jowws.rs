//! Lane K81 (13 Nov 2026; authors/titles-1030/K81.tsv): James P. O'Shaughnessy, "What Works on Wall Street" (1st ed., 1997,
//! file dated 1996). Book id "jowws". Source copy: research share 'USB HDD 2/tRADING KNO-2/James P.O'Shaughnessy - What Works on
//! Wall Street/...pdf' (text layer; printed page = PDF page - 21). Read: ch.3 'Rules of the Game' (p.27-29), ch.15 'Relative
//! Price Strength' (p.191-194). Reader row: authors/o2/O2.tsv (RULES-COMPLETE). Decision #295 (3): run on the vault's stock
//! baskets as a labelled universe change (undated members, survivorship stated).
//! The coded card runs on the basket account (books::rotation EP5): jowws_rs50 is in rotation::BASKET_IDS.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

const SRC: &str = "O'Shaughnessy, What Works on Wall Street (1997); printed pages (PDF page = printed + 21).";

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, dir, status, rules, notes| StrategyDef { id, name, group: "Ch.15 · Relative price strength", session: 0, dir, status, needs_volume: false,
        needs_vix: false, needs_series: &[], book: "jowws", rules, notes, claim: None };
    vec![
        d("jowws_rs50", "Top 50 one-year price performers: bought each 31 December, equal weight, held one year", "Long", Status::Live, &[
            "E|\"We'll start on December 31, 1951, and buy the 50 stocks with the largest price appreciation from the previous year. We arrive at this number by dividing this year's closing price by that from 12 months earlier.\" (p.192)",
            "X|\"I construct and rebalance portfolios annually. Stocks are equally weighted\" (p.28)",
            "E|option worst=1: \"We'll look at buying the 50 stocks with the best and the worst 1-year price changes\" (p.192)"],
          concat!("O'Shaughnessy, What Works on Wall Street (1997); printed pages (PDF page = printed + 21). ",
            "EP5 basket ranking (decisions #20, #295 (3)): at the last close of each year, rank the basket's stocks by that close / the close of the previous year's last trading day; buy the 50 best at that close in equal parts and sell them at the next year-end close, where the next 50 are bought (one portfolio a year). Option p.worst = 1: the 50 worst instead (the book's 'bottom fishers' test, p.192). ",
            "Universe change (labelled): the book ranks Compustat 'All Stocks' (market capitalisation above an inflation-adjusted $150 million) or 'Large Stocks' (above the database average, p.27-28); we hold no market capitalisations, so the card ranks every stock in the vault that was ever in the S&P 500 (membership dates unknown, so a stock is ranked whenever it has prices: survivorship differs from the book's), or with p.members = 1 today's list (more survivorship bias). ",
            "Price only (no dividends), split-adjusted vault files; the run ends where the basket's files end. The book's results (All Stocks top 50: $10,000 -> $3,310,255, 14.45% a year, Dec 1951 - Dec 1994, p.192; Table 15-1) cannot be matched on our span or universe: unverified. India withheld (#239 (2))."),
          ),
        d("jowws_factors", "Value and growth factor portfolios (P/E, price-to-book, price-to-sales, yield, cornerstone composites)", "Long",
          Status::NeedsData("annual fundamentals per stock (earnings, book value, sales, dividends, market capitalisation from Compustat): the vault holds prices only"), &[
            "E|\"I construct and rebalance portfolios annually\" on accounting ratios from the Compustat database (p.28)"],
          SRC),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let d = catalogue().into_iter().find(|d| d.id == id)?;
    Some((d.id, d.name, "O'Shaughnessy, What Works on Wall Street"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label| ParamDef { key, label, def: 0., min: 0., max: 1., step: 1. };
    match id {
        "jowws_rs50" => Some(vec![p("worst", "1 = buy the 50 worst one-year performers instead (p.192)"),
            p("members", "Universe: 0 = every stock ever in the S&P 500 (default), 1 = today's members only (survivorship bias)")]),
        "jowws_factors" => Some(vec![]),
        _ => None,
    }
}

/// The basket account runs jowws_rs50 (books::rotation::basket_account); there is no single-instrument run.
pub fn run(_id: &str, _x: &Ctx, _from: usize, _to: usize, _p: &Params) -> Option<Vec<Trade>> { None }
