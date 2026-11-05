//! T. J. George & C.-Y. Hwang, "The 52-Week High and Momentum Investing", Journal of Finance 59(5), October 2004,
//! p.2145-2176. Book id `gh52w`. Source: research drive "USB HDD 2/tRADING KNO-2/_Abstracts and Misc. Articles/
//! Journal of Finance/The 52-Week High and Momentum Investing.pdf", copied to Books/Thomas J. George & Chuan-Yang Hwang/.
//! Page refs are J. Finance pages. The strategy ranks the whole CRSP cross-section each month by P / 52-week high and
//! holds six overlapping equal-weighted winner/loser vintages. Since lane K56 (5 Nov 2026, EP5) it runs in books::rotation
//! on the vault's stock baskets (us_sp500 on SPY, nse500 on NIFTY 50) as a declared application (decision #20; check
//! authors/titles-1030/K56w/k56_ref.py). `run` returns None. Same family as jtwl and cjlmom.
use crate::connors::{claim, Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "gh52w";

const NOTE: &str = "EP5 basket ranking (lane K56). Declared application: the paper ranks all CRSP stocks, July 1963 - December 2001; here the stocks of the vault's us_sp500 basket (SPY calendar, to 11 Sep 2020) or nse500 basket (NIFTY 50 calendar). Membership caveat: each basket is today's member list (US: Norgate filename snapshot of 8 Sep 2020; India: the 2026 NSE list) applied to every past date, not dated membership (survivorship bias). Readings: ranked at each month's last close on that close / the highest daily close in the 12 months ending that day (the bars after the month end 12 months earlier, through this month end, and a stock needs a close at that earlier month end, i.e. a full 12 months of history; 'highest price' read as the highest close, CRSP's daily price; split-adjusted, price only); winners = top floor(0.3 N), losers = bottom floor(0.3 N), ties by ticker; bought and sold short at that close (no skip, as Tables I-IV, footnote 3 p.2149), held 6 months as a buy-and-hold vintage and closed at the 6th month's last close. One cash sleeve per vintage (1/6 of the account at the start, reused by the vintage formed 6 months later), equal money on every leg; shorts reserve their own principal (half long, half short, unlevered; the paper's portfolio is self-financing). p.long_only = 1 keeps only the winners (labelled option). The paper's period is outside the SPY calendar (from 1993), so 0.45% a month is not comparable.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef {
        id: "gh52w_52wh_6x6", name: "52-week-high winners minus losers (6-month hold)",
        group: "Sec. I · Data and Method (Table I)", session: 0, dir: "Long/Short", status: Status::Live,
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
pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    (id == "gh52w_52wh_6x6").then(|| vec![ParamDef { key: "long_only", label: "1 = winners only, no short side (labelled option)", def: 0., min: 0., max: 1., step: 1. }])
}
pub fn run(_id: &str, _x: &Ctx, _from: usize, _to: usize, _p: &Params) -> Option<Vec<Trade>> { None }

#[cfg(test)]
mod tests {
    #[test]
    fn catalogue_is_live_on_the_basket_account_and_owned() {
        let c = super::catalogue();
        assert!(!c.is_empty());
        for s in &c {
            assert!(s.id.starts_with("gh52w_") && s.book == super::BOOK);
            assert!(matches!(s.status, crate::connors::Status::Live) && crate::books::rotation::is_basket(s.id));
            assert!(super::family_of(s.id).is_some());
        }
    }
}
