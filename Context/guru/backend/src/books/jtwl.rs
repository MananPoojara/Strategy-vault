//! N. Jegadeesh & S. Titman, "Returns to Buying Winners and Selling Losers: Implications for Stock Market Efficiency",
//! Journal of Finance 48(1), March 1993, p.65-91. Book id `jtwl`. Source: research drive "USB HDD 2/tRADING KNO-2/
//! Narasim Jegadeesh, Sheridam Titman - .../Returns to Buying Winners & Selling Losers.pdf" (JSTOR scan, read in place;
//! the copy into Books/ was refused - COPY PENDING, see library/jtwl/README.md). Page refs are J. Finance pages.
//! Every strategy ranks the whole NYSE/AMEX cross-section into deciles each month (CRSP 1965-1989) and holds
//! overlapping equal-weighted winner/loser portfolios. Since lane K56 (5 Nov 2026, EP5) both run in books::rotation on
//! the vault's stock baskets (us_sp500 on the SPY calendar, nse500 on NIFTY 50) as a declared application (decision #20);
//! independent check authors/titles-1030/K56w/k56_ref.py. `run` returns None (the account is in books::rotation).
use crate::connors::{claim, Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "jtwl";

const NOTE: &str = "EP5 basket ranking (lane K56). Declared application: the paper ranks every NYSE/AMEX stock on CRSP, January 1965 - December 1989; here the stocks of the vault's us_sp500 basket (SPY calendar, to 11 Sep 2020) or nse500 basket (NIFTY 50 calendar). Membership caveat: each basket is today's member list (US: Norgate filename snapshot of 8 Sep 2020; India: the 2026 NSE list) applied to every past date, not dated index membership, so stocks that were dropped or delisted are missing (survivorship bias). Readings: ranked at each month's last close on the close-to-close return over the last J month ends (price only: split-adjusted closes, no dividends); deciles = the top and bottom floor(N/10) of the N stocks with both closes, ties by ticker; winners bought and losers sold short at that close and held K months as a buy-and-hold vintage, closed at the K-th month's last close (the paper reports monthly-rebalanced returns and says buy-and-hold was very similar, p.68). One cash sleeve per vintage (1/K of the account at the start; a new vintage reuses the proceeds of the vintage it replaces), split equally over its legs; shorts reserve their own principal, so a vintage is half long, half short, unlevered (the paper's portfolio is self-financing). p.long_only = 1 keeps only the winners (labelled option). The paper's period is outside the SPY calendar (from 1993), so the published monthly returns are not comparable.";

macro_rules! live {
    ($id:expr, $name:expr, $rules:expr, $claim:expr) => {
        StrategyDef { id: $id, name: $name, group: "Sec. I-II · Relative strength portfolios (Table I)", session: 0, dir: "Long/Short", status: Status::Live,
            needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules: $rules, notes: NOTE, claim: $claim }
    };
}

pub fn catalogue() -> Vec<StrategyDef> {
    vec![
        live!("jtwl_jk", "J-month / K-month winners minus losers (no skip)",
            &["F|\"At the beginning of each month t the securities are ranked in ascending order on the basis of their returns in the past J months. Based on these rankings, ten decile portfolios are formed that equally weight the stocks contained in the top decile, the second decile, and so on.\" (p.68); J, K in {3, 6, 9, 12} (16 strategies)",
              "E|\"In each month t, the strategy buys the winner portfolio and sells the loser portfolio, holding this position for K months.\" (p.68)",
              "X|\"In addition, the strategy closes out the position initiated in month t - K. Hence, under this trading strategy we revise the weights on 1/K of the securities in the entire portfolio in any given month and carry over the rest from the previous month.\" (p.68)",
              "F|Published (Table I Panel A, p.70): 6/6 buy-sell 0.95% per month (t 3.07); best 12/3 1.31% per month; 3/3 0.32% (t 1.10, the only insignificant one)",
              "F|[Declared reading] J = p.j, K = p.k (defaults 6 and 6, the paper's 6/6); ranked and filled at the month's last close; vintages held buy-and-hold"],
            claim("monthly portfolio", "n/a", "0.95%/month (6/6 buy-sell, t 3.07)", "6 months, overlapping", "Jan 1965 - Dec 1989, CRSP NYSE/AMEX (Table I Panel A, p.70)")),
        live!("jtwl_jk_skip", "J-month / K-month winners minus losers, one-week skip",
            &["F|\"a second set of 16 strategies that skip a week between the portfolio formation period and the holding period\" (p.68)",
              "E|Same ranking and K-month overlapping holding as jtwl_jk, formed one week after the J-month ranking period ends (Table I Panel B)",
              "X|Each vintage is closed after K months, one week after the K-th month end (as jtwl_jk: \"the strategy closes out the position initiated in month t - K\", p.68)",
              "F|Published (Table I Panel B, p.70): 6/6 buy-sell 1.10% per month (t 3.61); 12/3 1.49% per month (t 4.28)",
              "F|[Declared reading] one week = 5 trading days: bought and sold short at the close 5 trading days after the ranking month's last close, closed 5 trading days after the K-th month's last close"],
            claim("monthly portfolio", "n/a", "1.10%/month (6/6 buy-sell, t 3.61)", "6 months, overlapping", "Jan 1965 - Dec 1989, CRSP NYSE/AMEX (Table I Panel B, p.70)")),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "jtwl_jk" => ("jtwl_momentum", "Jegadeesh-Titman momentum", "No skip"),
        "jtwl_jk_skip" => ("jtwl_momentum", "Jegadeesh-Titman momentum", "One-week skip"),
        _ => return None,
    })
}
pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    matches!(id, "jtwl_jk" | "jtwl_jk_skip").then(|| vec![
        ParamDef { key: "j", label: "Ranking period J in months (paper: 3, 6, 9, 12)", def: 6., min: 1., max: 12., step: 3. },
        ParamDef { key: "k", label: "Holding period K in months (paper: 3, 6, 9, 12)", def: 6., min: 1., max: 12., step: 3. },
        ParamDef { key: "long_only", label: "1 = winners only, no short side (labelled option)", def: 0., min: 0., max: 1., step: 1. },
    ])
}
pub fn run(_id: &str, _x: &Ctx, _from: usize, _to: usize, _p: &Params) -> Option<Vec<Trade>> { None }
