//! C. M. C. Lee & B. Swaminathan, "Price Momentum and Trading Volume", Journal of Finance 55(5), October 2000,
//! p.2017-2069. Book id `lswtv`. Source: research drive "USB HDD 2/tRADING KNO-2/_Abstracts and Misc. Articles/
//! Journal of Finance/Price Momentum and Trading Volume.pdf", copied to Books/Lee & Swaminathan/. Page refs are
//! J. Finance pages. Every strategy is a price-momentum decile x turnover tercile portfolio over the NYSE/AMEX
//! cross-section; turnover needs shares outstanding. No index rule exists in the paper, so nothing is Live.
use crate::connors::{claim, Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "lswtv";

const XS: &str = "needs a monthly cross-sectional stock universe (CRSP NYSE/AMEX 1965-1995) with daily turnover = shares traded / shares outstanding for every stock, ranked independently into 10 return deciles and 3 turnover groups each month, and a portfolio engine holding K overlapping equal-weighted vintages; the vault has no shares-outstanding history and runs single instruments only (NSE baskets are per-symbol tickers, not a ranked cross-section)";
const NOTE: &str = "Paper period January 1965 - December 1995, NYSE/AMEX; excludes primes, closed-end funds, REITs, ADRs, foreign companies, firms delisted within five days of formation and stocks under $1 (p.2022). One-week lag between the formation period J and the holding period K (p.2023). J, K in {3, 6, 9, 12}. The simple R10 - R1 momentum row is Jegadeesh-Titman (jtwl). India: needs NSE shares-outstanding history for turnover and an NSE cross-section.";

macro_rules! nd {
    ($id:expr, $name:expr, $rules:expr, $claim:expr) => {
        StrategyDef { id: $id, name: $name, group: "Sec. II-III · Price momentum x trading volume (Tables II, VII)", session: 0, dir: "Long/Short", status: Status::NeedsData(XS),
            needs_volume: true, needs_vix: false, needs_series: &[], book: BOOK, rules: $rules, notes: NOTE, claim: $claim }
    };
}

const FORM: &str = "F|\"At the beginning of each month, from January 1965 to December 1995, we rank all eligible stocks independently on the basis of past returns and past trading volume. The stocks are then assigned to one of 10 portfolios based on returns over the previous J months and one of three portfolios based on the trading volume over the same time period.\" (p.2022); \"Trading volume (Volume) is defined as the average daily turnover in percentage during the portfolio formation period, where daily turnover is the ratio of the number of shares traded each day to the number of shares outstanding at the end of the day.\" (p.2022)";
const HOLD: &str = "X|\"the monthly return for a K-month holding period is based on an equal-weighted average of portfolio returns from strategies implemented in the current month and the previous K - 1 months\" (p.2022-2023); \"we impose a one-week lag between the end of the portfolio formation period (J) and the beginning of the performance measurement period (K)\" (p.2023)";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![
        nd!("lswtv_early", "Early-stage momentum: long low-volume winners, short high-volume losers (R10V1 - R1V3)",
            &[FORM,
              "E|\"buying low volume winners and selling high volume losers, as the early-stage strategy\" (p.2040); Table VII uses J = 6: \"Early represents a zero investment portfolio that is long low volume winners (R10V1 ...) and short high volume losers (R1V3 ...)\" (p.2041)",
              HOLD,
              "F|Published (Table VII Panel A, p.2041): Year 1 16.70% (t 5.85), Year 2 6.19%, Year 3 5.85%, Year 4 1.53%, Year 5 -0.11%; 325 monthly observations, 1965-1995"],
            claim("325 formation months", "n/a", "Year 1 +16.70%", "12-month event-time years", "1965-1995, NYSE/AMEX")),
        nd!("lswtv_late", "Late-stage momentum: long high-volume winners, short low-volume losers (R10V3 - R1V1)",
            &[FORM,
              "E|\"buying high volume winners and selling low volume losers, as the late-stage momentum strategy\" (p.2040); Table VII: \"Late represents a zero investment portfolio that is long high volume winners (R10V3 ...) and short low volume losers (R1V1)\" (p.2041)",
              HOLD,
              "F|Published (Table VII Panel A, p.2041): Year 1 6.84% (t 2.53), Year 2 -5.35%, Year 3 -3.91%, Year 4 -6.33%, Year 5 -4.78%"],
            claim("325 formation months", "n/a", "Year 1 +6.84%", "12-month event-time years", "1965-1995, NYSE/AMEX")),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "lswtv_early" => ("lswtv_volume_momentum", "Lee-Swaminathan volume-based momentum", "Early stage"),
        "lswtv_late" => ("lswtv_volume_momentum", "Lee-Swaminathan volume-based momentum", "Late stage"),
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
            assert!(s.id.starts_with("lswtv_") && s.book == super::BOOK);
            assert!(matches!(s.status, crate::connors::Status::NeedsData(_)));
            assert!(super::family_of(s.id).is_some());
        }
    }
}
