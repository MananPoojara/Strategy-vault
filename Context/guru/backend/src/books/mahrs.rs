//! Marvin Appel, Higher Returns from Safe Investments (FT Press / Pearson, 2010).
//! A bond / income book. Its only mechanical timing rules trade high-yield bond and municipal bond
//! mutual funds by their total-return NAV, and its option rule writes S&P 500 calls: the vault has
//! neither fund NAVs nor option prices, so these are NeedsData. Running bond-fund stops on an equity
//! index would be an asset-class swap (not allowed; the book itself says the stops do not suit most
//! stock investments, p.87). Page refs are printed pages. Sources: library/mahrs/.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "mahrs";

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, group, status, rules, notes| StrategyDef {
        id, name, group, session: 0, dir: "Long", status, needs_volume: false, needs_vix: false, needs_series: &[],
        book: BOOK, rules, notes, claim: None,
    };
    use Status::*;
    vec![
        d("mahrs_hy_stops", "High-yield bond fund 3% monthly buy and sell stops", "Ch 7 · Risk management: the stop loss (p.84-89)",
            NeedsData("needs month-end total-return NAV (distributions reinvested) of high-yield bond mutual funds, e.g. Delaware Delchester DETWX, Northeast Investors Trust NTHEX; fund data is not in the vault"),
            &["X|\"check the value of those shares (including reinvested interest distributions) at the end of each month, and to sell your shares any time they lose 3% of their most recent peak value\" (p.84)",
              "E|\"reenter the shares ... as soon as the shares rise 3% off of the most recent month-end low\" (p.86)"],
            "Book: DETWX 1988-2009 7.2%/yr, drawdown 17% vs 4.3%/yr buy and hold, 20 transactions; NTHEX 6.7%/yr, drawdown 15% (p.87-89). Weekly evaluation mentioned as better but not specified (p.90)."),
        d("mahrs_muni_stop", "Nuveen High Yield Municipal 6% weekly stops", "Ch 8 · Earn 7% per year, free of federal income tax (p.100-101)",
            NeedsData("needs weekly total-return NAV of the Nuveen High Yield Municipal Bond Fund (NHMAX); fund data is not in the vault"),
            &["X|\"evaluate the total return of your investment ... at the end of each week. You should sell your shares and move to cash whenever the fund slips by more than 6% from its peak weekly value.\" (p.101)",
              "E|\"Then you stay out of the fund until its total return lifts it at least 6% off of its weekly low point.\" (p.101)"],
            "Book: losses in 2007-2008 cut from 45% to below 10% (p.101); the author warns the rule was fitted to one episode (p.102)."),
        d("mahrs_covered_calls", "S&P 500 covered call writing (BXM)", "Ch 12 · Using options to earn income (p.153-166)",
            NeedsData("needs S&P 500 / SPY call option prices (CBOE BXM buy-write index or option chains); options are not in the vault"),
            &["E|Hold SPY and write the nearest-expiration call \"whose strike price is closest to (but not below)\" the price (p.161-162)",
              "X|Roll at each expiration (CBOE BuyWrite index methodology, p.159-161)"],
            "Book compares BXM with the S&P 500 (p.159-161)."),
        d("mahrs_high_dividend", "50 highest-dividend-yield large stocks (O'Shaughnessy)", "Ch 11 · Equity ETFs for dividend income (p.141-142)",
            NeedsData("needs a US large-company stock universe with yearly dividend yields; not in the vault"),
            &["E|\"all large company U.S. stocks compared with the 50 with the highest-dividend yield (evaluated once each year)\" (p.141, citing What Works on Wall Street)"],
            "A cited study (1952-2003, 13.6%/yr vs 11.7%), not the author's own rule."),
        d("mahrs_portfolios", "Bond ladders and lifetime-income portfolios", "Ch 4, 13 (p.45-49, p.167-175)",
            NotTestable("portfolio construction (ladders, allocation percentages) with no entry or exit timing"),
            &["F|e.g. \"Allocate 25% of Your Capital to High-Yield Bond Fund Trading\" (p.172)"], "Not a trading rule."),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    id.starts_with("mahrs_").then_some(("mahrs_income", "Income investing rules", "Rule"))
}

pub fn params(_id: &str) -> Option<Vec<ParamDef>> {
    None
}

pub fn run(_id: &str, _x: &Ctx, _from: usize, _to: usize, _p: &Params) -> Option<Vec<Trade>> {
    None
}
