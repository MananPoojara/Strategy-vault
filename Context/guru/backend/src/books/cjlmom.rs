//! L. K. C. Chan, N. Jegadeesh & J. Lakonishok, "Momentum Strategies", Journal of Finance 51(5), Dec 1996,
//! p.1681-1713. Book id `cjlmom`. Source copy: Books/Louis K. C. Chan/... Momentum Strategies.pdf.
//! Every strategy ranks the whole NYSE/AMEX/Nasdaq cross-section into NYSE-breakpoint deciles each month and
//! holds equal-weighted portfolios; three of the four ranking variables need earnings / analyst data. The
//! engine runs one instrument at a time (NSE baskets are per-symbol tickers, no cross-sectional ranking or
//! portfolio book), so nothing is Live and `run` returns None.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "cjlmom";

const XS: &str = "needs a monthly cross-sectional stock universe (CRSP NYSE/AMEX/Nasdaq domestic primary issues 1977-1993 with delisting returns and NYSE size breakpoints) and a portfolio engine that ranks all stocks into deciles each month; the vault has single-instrument runs only (NSE baskets are per-symbol tickers, not a ranked cross-section)";
const EARN: &str = "needs the cross-sectional universe above plus quarterly EPS and announcement dates (COMPUSTAT) and daily stock + equal-weighted market returns; not in the vault";
const IBES: &str = "needs the cross-sectional universe above plus monthly I/B/E/S consensus FY1 EPS forecasts; not in the vault";

macro_rules! nd {
    ($id:expr, $name:expr, $status:expr, $rules:expr) => {
        StrategyDef { id: $id, name: $name, group: "Sec. II-III · Momentum portfolios (Tables II-VI, X)", session: 0, dir: "Long/Short", status: $status,
            needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules: $rules,
            notes: "Paper period: portfolios formed monthly Jan 1977 - Jan 1993; returns measured from 5 days after formation (p.1686); delisted stocks replaced by the value-weighted market return (p.1686). Equal-weighted deciles, NYSE breakpoints.", claim: None }
    };
}

pub fn catalogue() -> Vec<StrategyDef> {
    vec![
        nd!("cjlmom_r6", "Price momentum: prior six-month return deciles", Status::NeedsData(XS),
            &["F|\"At the beginning of every month from January 1977 to January 1993, we rank stocks on the basis of either past returns or a measure of earnings news ... assigned to one of ten decile portfolios, where the breakpoints are based only on NYSE stocks\" (p.1684)",
              "E|\"The ranking variable used in our price momentum strategy is a stock's past compound return, extending back six months prior to portfolio formation\" (p.1684); buy decile 10 (winners), sell decile 1 (losers)",
              "X|Buy-and-hold returns over the following six months / one year, skipping the first five days after formation (p.1686). Published: 10 minus 1 = 8.8% in six months, 15.4% in the first year (Table II, p.1687-1688)"]),
        nd!("cjlmom_sue", "Earnings momentum: standardized unexpected earnings deciles", Status::NeedsData(EARN),
            &["F|SUE_it = (e_iq - e_iq-4) / sigma_it, sigma = std of unexpected earnings over the preceding eight quarters (eq. 1, p.1685)",
              "E|Rank monthly on the most recent SUE into NYSE-breakpoint deciles (Table III, p.1692); 10 minus 1 = 6.8% in six months (p.1691)"]),
        nd!("cjlmom_abr", "Earnings momentum: abnormal announcement return deciles", Status::NeedsData(EARN),
            &["F|ABR_it = sum_{j=-2..+1} (r_ij - r_mj), equal-weighted market (eq. 2, p.1685)", "E|Decile ranking as Table IV (p.1694)"]),
        nd!("cjlmom_rev6", "Earnings momentum: analyst revision (REV6) deciles", Status::NeedsData(IBES),
            &["F|REV6_it = sum_{j=0..5} (f_it-j - f_it-j-1) / p_it-j-1, I/B/E/S mean FY1 estimate (eq. 3, p.1685; printed sum limit 6)", "E|Decile ranking as Table V"]),
        nd!("cjlmom_two_way", "Two-way price x earnings momentum (3x3), six-month rebalanced winners minus losers", Status::NeedsData(IBES),
            &["E|\"we sort the securities ... on the basis of their past six-month returns and assign them to one of three equally-sized portfolios. Independently, we sort stocks ... on the basis of the most recent earnings surprise\" (p.1695)",
              "X|\"The portfolio is held for six months, at which time the portfolio is reformed and the strategy repeated\" (Table X, p.1708): winners (3,3) minus losers (1,1), intercept 1.431%/month"]),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "cjlmom_r6" => ("cjlmom_price", "CJL price momentum", "Prior 6-month return"),
        "cjlmom_sue" => ("cjlmom_earn", "CJL earnings momentum", "SUE"),
        "cjlmom_abr" => ("cjlmom_earn", "CJL earnings momentum", "Announcement return"),
        "cjlmom_rev6" => ("cjlmom_earn", "CJL earnings momentum", "Analyst revisions"),
        "cjlmom_two_way" => ("cjlmom_price", "CJL price momentum", "Two-way with earnings"),
        _ => return None,
    })
}
pub fn params(_id: &str) -> Option<Vec<ParamDef>> { None }
pub fn run(_id: &str, _x: &Ctx, _from: usize, _to: usize, _p: &Params) -> Option<Vec<Trade>> { None }
