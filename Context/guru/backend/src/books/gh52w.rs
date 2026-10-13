//! T. J. George & C.-Y. Hwang, "The 52-Week High and Momentum Investing", Journal of Finance 59(5), October 2004,
//! p.2145-2176. Book id `gh52w`. Source: research drive "USB HDD 2/tRADING KNO-2/_Abstracts and Misc. Articles/
//! Journal of Finance/The 52-Week High and Momentum Investing.pdf", copied to Books/Thomas J. George & Chuan-Yang Hwang/.
//! Page refs are J. Finance pages. The strategy ranks the whole CRSP cross-section each month by P / 52-week high and
//! holds six overlapping equal-weighted winner/loser vintages; the engine runs one instrument at a time (NSE baskets are
//! per-symbol tickers with no monthly cross-sectional ranking or portfolio book), so nothing is Live and `run` returns None.
//! Same family as jtwl (Jegadeesh-Titman 1993) and cjlmom, which are also NeedsData.
use crate::connors::{claim, Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "gh52w";

const XS: &str = "needs a monthly cross-sectional stock universe (all CRSP stocks 1963-2001: month-end price and the highest price of the prior 12 months for every stock) ranked into top/middle/bottom 30% each month, and a portfolio engine holding 6 overlapping equal-weighted long/short vintages; the vault runs single instruments only (NSE baskets are per-symbol tickers, not a ranked cross-section)";
const NOTE: &str = "Paper period July 1963 - December 2001, all CRSP stocks (p.2148). Tables I-IV do not skip a month between ranking and holding; the regression tests (Table V on) skip one month (footnote 3, p.2149). The (6,12), (12,6) and (12,12) horizons are robustness checks of the same rule (p.2149). The JT and MG rows of Table I are other authors' strategies (JT = jtwl in the vault). India: would run on an NSE cross-section (survivorship-free membership needed), not on an index.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef {
        id: "gh52w_52wh_6x6", name: "52-week-high winners minus losers (6-month hold)",
        group: "Sec. I · Data and Method (Table I)", session: 0, dir: "Long/Short", status: Status::NeedsData(XS),
        needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK,
        rules: &[
            "F|\"For the 52-week high strategy, stocks are ranked based on P(i,t-1)/high(i,t-1), where P(i,t-1) is the price of stock i at the end of month t - 1 and high(i,t-1) is the highest price of stock i during the 12-month period that ends on the last day of month t - 1.\" (p.2149)",
            "E|\"The winner (loser) portfolio for the 52-week high strategy is the equally weighted portfolio of the 30% of stocks with the highest (lowest) ratio of current price to 52-week high.\" (Table I caption, p.2148)",
            "E|\"The strategy is to hold, for 6 months, a self-financing portfolio that is long the winner and short the loser portfolios.\" (p.2149)",
            "X|\"In any particular month t of a (6, 6) strategy, the return to winners is calculated as the equally weighted average of the month t returns from six separate winner portfolios, each formed in one of the 6 consecutive prior months t - 6 to t - 1.\" (p.2149) - each vintage is closed after 6 months",
            "F|Published (Table I, p.2148): winner 1.51%, loser 1.06%, winner - loser 0.45% per month (t 2.00), July 1963 - Dec 2001; excluding January 1.23% (t 7.06), January only -8.27% (Table II, p.2150)",
        ],
        notes: NOTE,
        claim: claim("monthly portfolio", "n/a", "0.45%/month W-L (t 2.00)", "6 months, overlapping", "Jul 1963 - Dec 2001, all CRSP stocks"),
    }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "gh52w_52wh_6x6" => ("gh52w_52wh", "George-Hwang 52-week-high momentum", "(6,6)"),
        _ => return None,
    })
}
pub fn params(_id: &str) -> Option<Vec<ParamDef>> { None }
pub fn run(_id: &str, _x: &Ctx, _from: usize, _to: usize, _p: &Params) -> Option<Vec<Trade>> { None }

#[cfg(test)]
mod tests {
    #[test]
    fn catalogue_is_needsdata_and_owned() {
        let c = super::catalogue();
        assert!(!c.is_empty());
        for s in &c {
            assert!(s.id.starts_with("gh52w_") && s.book == super::BOOK);
            assert!(matches!(s.status, crate::connors::Status::NeedsData(_)));
            assert!(super::family_of(s.id).is_some());
        }
    }
}
