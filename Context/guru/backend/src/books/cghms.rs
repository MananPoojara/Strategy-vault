//! M. J. Cooper, R. C. Gutierrez Jr. & A. Hameed, "Market States and Momentum", Journal of Finance 59(3), June 2004,
//! p.1345-1365. Book id `cghms`. Source: research drive "USB HDD 2/tRADING KNO-2/_Abstracts and Misc. Articles/
//! Journal of Finance/Market States and Momentum.pdf", copied to Books/Cooper Gutierrez & Hameed/. Page refs are
//! J. Finance pages. The market state is an index rule (36-month return of the CRSP VW index), but the position itself is
//! a decile long/short portfolio over the NYSE/AMEX cross-section, so nothing is Live and `run` returns None.
//! The paper conditions profits on the state; it states no rule that trades the index itself (none is invented here).
use crate::connors::{claim, Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "cghms";

const XS: &str = "needs a monthly cross-sectional stock universe (CRSP NYSE/AMEX monthly returns 1926-1995, month-end prices for the $1 screen) ranked into deciles on months t-5..t-1, and a portfolio engine holding winner-minus-loser decile vintages; the market state also needs a value-weighted total-return index (CRSP VW incl. dividends). The vault runs single instruments only (NSE baskets are per-symbol tickers, not a ranked cross-section)";
const NOTE: &str = "Paper period 1929:01-1995:12 (Table I). Profits are reported per holding-period month and cumulated (CAR) over months 1-6, 1-12 and 13-60, converted to monthly by dividing by six (p.1350). The state uses the CRSP VW index incl. dividends over months t-36..t-1; two-year and one-year state definitions are robustness checks (Sec. II.B). India: the state could be read from NIFTY 50 TRI, but the portfolio needs an NSE cross-section.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![
        StrategyDef {
            id: "cghms_mom_6x6_state", name: "6-month momentum (deciles, skip a month) after UP vs DOWN markets",
            group: "Sec. I.A-B · Momentum profits and market states (Table I)", session: 0, dir: "Long/Short", status: Status::NeedsData(XS),
            needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK,
            rules: &[
                "F|\"Stocks are sorted at the end of each month t into deciles based on their prior six-month returns. To mitigate bid-ask bounce effects, the formation-period six-month returns are calculated from t - 5 to t - 1, skipping month t. We exclude stocks with a price at the end of the formation period below $1\" (p.1348)",
                "E|\"We define each momentum portfolio as long in the prior six-month winners (highest decile) and short in the prior six-month losers (lowest decile).\" (p.1349)",
                "X|Holding periods \"t + 1 to t + 6, t + 1 to t + 12, and t + 13 to t + 60\" (p.1349); the 6/6 strategy holds months 1-6",
                "F|\"we employ the return on the CRSP value-weighted index (including dividends) over the 36 months prior to the beginning of the strategy's holding period. If the market's three-year return is non-negative (negative), we define the state of the market as 'UP' ('DOWN').\" (p.1350)",
                "F|Published (Table I, p.1351), months 1-6: after UP markets mean profit 0.93%/month (t 8.41, N 674); after DOWN markets -0.37%/month (t -0.65, N 124); UP - DOWN t 2.26. Months 1-12: 0.72% vs -0.50%",
            ],
            notes: NOTE,
            claim: claim("674 UP / 124 DOWN formation months", "n/a", "UP 0.93%/month, DOWN -0.37%/month", "6 months", "1929:01-1995:12, NYSE/AMEX"),
        },
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "cghms_mom_6x6_state" => ("cghms_state_momentum", "Market-state conditioned momentum", "6/6, 36-month state"),
        _ => return None,
    })
}
pub fn params(_id: &str) -> Option<Vec<ParamDef>> { None }
pub fn run(_id: &str, _x: &Ctx, _from: usize, _to: usize, _p: &Params) -> Option<Vec<Trade>> { None }

#[cfg(test)]
mod tests {
    #[test]
    fn catalogue_is_needsdata_and_owned() {
        for s in &super::catalogue() {
            assert!(s.id.starts_with("cghms_") && s.book == super::BOOK);
            assert!(matches!(s.status, crate::connors::Status::NeedsData(_)));
            assert!(super::family_of(s.id).is_some());
        }
    }
}
