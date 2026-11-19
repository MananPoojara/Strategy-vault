//! L. K. C. Chan, N. Jegadeesh & J. Lakonishok, "Momentum Strategies", Journal of Finance 51(5), Dec 1996,
//! p.1681-1713. Book id `cjlmom`. Source copy: Books/Louis K. C. Chan/... Momentum Strategies.pdf.
//! Every strategy ranks the whole NYSE/AMEX/Nasdaq cross-section into NYSE-breakpoint deciles each month and
//! holds equal-weighted portfolios; three of the four ranking variables need earnings / analyst data. Since lane K56
//! (5 Nov 2026, EP5) the price-momentum rule cjlmom_r6 runs in books::rotation on the vault's stock baskets (us_sp500 on
//! SPY, #239; India withheld) as a declared application (decision #20; check authors/titles-1030/K56w/k56_ref.py); the
//! earnings rules stay NeedsData. `run` returns None.
use crate::connors::{claim, Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "cjlmom";

#[allow(dead_code)] // the EARN/IBES texts refer to it ("the cross-sectional universe above")
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
        StrategyDef { id: "cjlmom_r6", name: "Price momentum: prior six-month return deciles", group: "Sec. II-III · Momentum portfolios (Tables II-VI, X)", session: 0, dir: "Long/Short", status: Status::Live,
            needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK,
            rules: &["F|\"At the beginning of every month from January 1977 to January 1993, we rank stocks on the basis of either past returns or a measure of earnings news ... assigned to one of ten decile portfolios, where the breakpoints are based only on NYSE stocks\" (p.1684)",
              "E|\"The ranking variable used in our price momentum strategy is a stock's past compound return, extending back six months prior to portfolio formation\" (p.1684); buy decile 10 (winners), sell decile 1 (losers)",
              "X|Buy-and-hold returns over the following six months / one year, skipping the first five days after formation (p.1686). Published: 10 minus 1 = 8.8% in six months, 15.4% in the first year (Table II, p.1687-1688)",
              "F|\"To control for this effect, we skip the first five days after portfolio formation before we begin to measure returns\" (p.1686)",
              "F|[Declared reading] formation = a month's last close; bought (decile 10) and sold short (decile 1) at the close of the 5th trading day after it; held buy-and-hold to the 6th month's last close after formation (p.k = 12: one year)"],
            notes: "EP5 basket ranking (lane K56). Declared application: the paper ranks all NYSE/AMEX/Nasdaq domestic primary issues on CRSP/COMPUSTAT, formations January 1977 - January 1993, with NYSE size breakpoints; here every stock in the vault that was ever in the S&P 500 (us_sp500_past on the SPY calendar, to 11 Sep 2020: current members and former members with their own price files); membership dates are not known, so a stock is ranked whenever it has prices (decision #239). p.members = 1 ranks today's list only (us_sp500, Norgate snapshot of 8 Sep 2020; labelled option, survivorship bias). India: withheld, the NSE basket is the 2026 member list; a ranking over 20 years of today's members is dominated by survivorship bias (#239 (2)); all stocks with both closes are ranked together (no NYSE breakpoints); the paper replaces a delisted stock's return by the market's, here a stock whose file ends is sold at its last close. Readings: compound six-month return = close at the formation month's last close / close six month ends earlier - 1 (price only, split-adjusted, no dividends); deciles = top and bottom floor(N/10), ties by ticker. The paper reports each formation's buy-and-hold return; the account runs one vintage per month (one cash sleeve each, 1/K of the account at the start, reused by the vintage formed K months later), equal money on every leg, shorts reserving their own principal (half long, half short, unlevered). p.long_only = 1 keeps only decile 10 (labelled option). The paper's period is outside the SPY calendar (from 1993), so its 8.8% is not comparable.",
            claim: claim("monthly portfolios", "n/a", "8.8% in 6 months (10 minus 1)", "6 months (15.4% in 1 year)", "formations Jan 1977 - Jan 1993, CRSP NYSE/AMEX/Nasdaq (Table II, p.1687-1688)") },
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
pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    match id {
        "cjlmom_r6" => Some(vec![
            ParamDef { key: "k", label: "Holding period in months: 6 (paper) or 12 (the paper's one-year returns)", def: 6., min: 6., max: 12., step: 6. },
            ParamDef { key: "long_only", label: "1 = decile 10 only, no short side (labelled option)", def: 0., min: 0., max: 1., step: 1. },
            ParamDef { key: "members", label: "Universe: 0 = every stock ever in the index (us_sp500_past, default, #239), 1 = today's members only (us_sp500, labelled option)", def: 0., min: 0., max: 1., step: 1. },
        ]),
        _ => None,
    }
}
pub fn run(_id: &str, _x: &Ctx, _from: usize, _to: usize, _p: &Params) -> Option<Vec<Trade>> { None }
