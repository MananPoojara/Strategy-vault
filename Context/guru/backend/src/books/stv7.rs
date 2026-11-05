//! Sam Stovall, "The Seven Rules of Wall Street" (McGraw-Hill 2009). Book id `stv7`. Source: research share
//! Trading Knowledge I/Sam Stovall - The Seven Rules of Wall Street/Sam Stovall - The Seven Rules of Wall Street.pdf
//! (pdf page = printed + 16; text authors/batch-c/C1w/stovall.txt). Rule 3 (Sell in May rotation, p.65-68) and Rule 5
//! (There's Always a Bull Market Someplace, sectors, p.87-101), each with the book's own ETF implementation.
//! The rotations run in books::rotation on the SPY calendar (lane K28, 3 Nov 2026; check authors/titles-1030/K28w/k28_ref.py).
//! Not coded: the industry versions (S&P industry indices and STARS-ranked proxy stocks are not in the vault); India
//! needs NIFTY sector indices the vault does not hold (decision #11).
use crate::connors::{claim, Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "stv7";

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, group, rules, notes, claim| StrategyDef { id, name, group, session: 0, dir: "Long", status: Status::Live, needs_volume: false,
        needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim };
    vec![
        d("stv7_sell_in_may", "Sell in May rotation: SPY November-April, Consumer Staples (XLP) May-October", "Rule 3 · Sell in May and Then Go Away",
          &["E|\"On the close of trading on April 30 (or the last trading day of the month), invest the entire $30,000 in the Consumer Staples Select Sector ETF (XLP)\" (p.67)",
            "X|\"Toward the close of trading on the last day of October, sell your 1,084 shares of XLP and invest all of the proceeds in the S&P 500 Depositary Receipts ETF (SPY)\" (p.67)",
            "F|\"This owning of SPY from November through April, followed by the owning of XLP from May through October, would occur in perpetuity\" (p.67)",
            "F|Option (p.68): \"substitute the Health Care Select Sector SPDR ETF (XLV) for ... (XLP) in the May-October period\" (p.sector = 1)"],
          "Fills at the last close of April and October (as printed). The run starts at the first switch date at which both ETFs have prices (XLP and XLV start 22 Dec 1998), so the first holding is XLP from 30 Apr 1999. Price-only ETF closes (the book also excludes dividends). The book's half XLP / half XLV version is not coded. Runs on the SPY calendar; the holdings are SPY and XLP (or XLV). US only: India needs a NIFTY FMCG or Pharma index series the vault does not hold.",
          claim("n/a", "n/a", "10.0% a year", "6 months", "S&P 500 Nov-Apr + S&P Consumer Staples May-Oct, 30 Apr 1990 - 31 Oct 2008, price only ($1,000 -> $6,121; Health Care 10.1%; S&P 500 5.8%) (p.66)")),
        d("stv7_bull_someplace", "There's Always a Bull Market Someplace: top 3 sector SPDRs by 12-month change, monthly", "Rule 5 · There's Always a Bull Market Someplace",
          &["F|\"At the beginning of each month ... identify the ... three sectors with the best trailing 12-month price performances. You can also go to www.sectorspdr.com to find out ... the price performances for their nine-sector ETFs\" (p.99)",
            "E|\"One-third would be invested in each of the three sectors that posted the best performances in the prior 12 months. ... as of June 30, 2008 ... If, on July 1, 2008, you chose to invest in the Select Sector SPDRs ETFs\" (p.99-100)",
            "X|\"leave this portfolio untouched until August 1, 2008, when you would look to see which three ... had the highest trailing 12-month price performances. If the S&P 500 Materials sector had been replaced by the S&P 500 Consumer Staples sector, you would sell your shares in ... (XLB) and use the proceeds to purchase shares in ... (XLP)\" (p.100)"],
          "Universe: the nine Select Sector SPDRs XLB XLE XLF XLI XLK XLP XLU XLV XLY (the book ranks the ten S&P 500 sectors; nine ETFs existed, telecom had none). Readings: 12-month change = last close of this month / last close of the same month a year earlier - 1; ranked at the month's last close, traded at the next day's open (the book: 'as of June 30 ... on July 1'); ties by ticker. A holding that stays in the top 3 is not touched; a sold holding's proceeds buy the newcomer (as printed), so the three positions drift apart in value between changes. Price-only ETF data. Runs on the SPY calendar; the holdings are the ETFs named in each trade. US only (no NIFTY sector index series, decision #11).",
          claim("n/a", "n/a", "14.6% a year", "monthly review", "S&P 500 sector indices, 1991-2007, average annual price change (S&P 500 10.4%) (Figure 5.1, p.92)")),
    ]
}
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match id {
        "stv7_sell_in_may" => Some(("stv7_sell_in_may", "Stovall Sell in May rotation", "SPY / XLP")),
        "stv7_bull_someplace" => Some(("stv7_bull_someplace", "Stovall sector momentum", "Top 3 sector SPDRs, monthly")),
        _ => None,
    }
}
pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    match id {
        "stv7_sell_in_may" => Some(vec![ParamDef { key: "sector", label: "May-October sector: 0 = Consumer Staples XLP (book), 1 = Health Care XLV (book option)", def: 0., min: 0., max: 1., step: 1. }]),
        "stv7_bull_someplace" => Some(vec![]),
        _ => None,
    }
}
/// The rotations run on their own account (books::rotation), not as one-ticker trades.
pub fn run(_id: &str, _x: &Ctx, _from: usize, _to: usize, _p: &Params) -> Option<Vec<Trade>> { None }
