//! Hurst, Ooi & Pedersen (AQR), "A Century of Evidence on Trend-Following Investing" (Fall 2012). Book id `aqrtf`.
//! Source copy: Books/Hurst Ooi Pedersen/A Century of Evidence on Trend-Following Investing (16).pdf
//! (research drive TF_MORE (1).zip!Trading_Papers/16.pdf). Transcription and checks: library/aqrtf/.
//!
//! Paper p.2: "the position taken in each market is determined by assessing the past return in that market over the
//! relevant look-back horizon. A positive past return is considered an 'up trend' and leads to a long position; a
//! negative past return ... a short position. Therefore, each strategy always holds either a long or short position
//! in every market." Positions are "aggregated each month" (monthly rebalance).
//! Single-market reading used here: at each month's last trading bar, past return = close / close k month-ends ago - 1;
//! hold that sign at the close until the next month-end. Unit position (the paper's volatility targeting changes size,
//! never direction, and its per-market estimator is not stated). Zero past return (book silent): keep the current side.
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "aqrtf";

const MAP: &str = "Paper: 59 futures/forward markets (24 commodities, 11 equity index futures incl. S&P 500 and Russell 2000, 15 bond, 9 FX), Jan 1903-Jun 2012, returns in excess of local short rates. Here the sign rule runs on one index's price series (instrument swap: NIFTY 50 / SPX etc.); index price return stands in for the futures excess return (dividends and financing not included). Unit long/short position; no volatility targeting, no 2/20 fees. Published results are for the 59-market portfolio only, so no single-index figure exists to reproduce.";

macro_rules! def {
    ($id:expr, $name:expr, $status:expr, $rules:expr, $notes:expr) => {
        StrategyDef { id: $id, name: $name, group: "Time series momentum (Section 2)", session: 0, dir: "Long/Short",
            status: $status, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules: $rules,
            notes: $notes, claim: None }
    };
}

const R_COMMON: &str = "X|\"each strategy always holds either a long or short position\" (p.2): the opposite sign at a month-end closes and reverses on that close";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![
        def!("aqrtf_tsmom_1m", "Time series momentum · 1-month look-back", Status::Live,
            &["F|\"the position taken in each market is determined by assessing the past return in that market over the relevant look-back horizon\" (p.2); look-back = 1 month",
              "E|\"A positive past return is considered an 'up trend' and leads to a long position; a negative past return is considered a 'down trend' and leads to a short position\" (p.2). Evaluated on the month's last trading-day close",
              "E|\"The positions ... are aggregated each month\" (p.2): monthly rebalance, position taken at that close", R_COMMON], MAP),
        def!("aqrtf_tsmom_3m", "Time series momentum · 3-month look-back", Status::Live,
            &["F|Past return over the 3-month look-back (p.2)", "E|Positive -> long, negative -> short, at the month-end close (p.2)", R_COMMON], MAP),
        def!("aqrtf_tsmom_12m", "Time series momentum · 12-month look-back", Status::Live,
            &["F|Past return over the 12-month look-back (p.2)", "E|Positive -> long, negative -> short, at the month-end close (p.2)", R_COMMON], MAP),
        def!("aqrtf_tsmom_combo", "Equal-weighted 1/3/12-month time series momentum", Status::Live,
            &["F|\"an equal-weighted combination of 1-month, 3-month, and 12-month time series momentum strategies\" (p.2)",
              "E|Net position each month-end = sum of the three ±1 signals; on one market this is long when 2 or 3 of them are long, short when 2 or 3 are short",
              "X|The combined portfolio is scaled to a 10% ex-ante volatility target (p.2, fn.5): sizing only, not modelled (unit position)", R_COMMON],
            "Single-market reading: after the paper's portfolio volatility scaling a one-market portfolio keeps only the sign of the aggregated position, so trades follow the majority sign of the 1/3/12-month signals. Position size (vol target, 3-year covariance) is not modelled. Same instrument mapping as the single look-back variants."),
        def!("aqrtf_portfolio", "59-market diversified TSMOM portfolio (Exhibit 1)", Status::NeedsData(
            "Needs 59 futures/forward return series back to 1903 (24 commodities, 11 developed equity index futures, 15 government bond futures, 9 USD currency forwards) and local short rates; the vault has 6 equity indices only, no bonds, commodities, FX or rates."),
            &["F|59 markets across 4 asset classes, Jan 1903-Jun 2012 (p.2, App. A)",
              "E|Equal-weighted 1/3/12-month time series momentum per market, each position sized to the same volatility (p.2)",
              "X|Aggregate scaled to 10% annualised ex-ante volatility using a rolling 3-year equally weighted monthly covariance matrix (fn.5); 2% management + 20% performance fee with high-water mark (App. B)"],
            "Exhibit 1: gross 20.0%/yr, net of 2/20 14.3%/yr, vol 9.9%, Sharpe 1.00 net, 1903-2012. Transaction costs per App. B table (equities 0.36%/0.12%/0.06% one-way by era)."),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let f = |v| Some(("aqrtf_tsmom", "Time series momentum (AQR)", v));
    match id {
        "aqrtf_tsmom_1m" => f("1-month"),
        "aqrtf_tsmom_3m" => f("3-month"),
        "aqrtf_tsmom_12m" => f("12-month"),
        "aqrtf_tsmom_combo" => f("1/3/12 combined"),
        "aqrtf_portfolio" => f("59-market portfolio · data missing"),
        _ => None,
    }
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def: f64| ParamDef { key, label, def, min: 1.0, max: 24.0, step: 1.0 };
    match id {
        "aqrtf_tsmom_1m" => Some(vec![p("months", "Look-back (months)", 1.0)]),
        "aqrtf_tsmom_3m" => Some(vec![p("months", "Look-back (months)", 3.0)]),
        "aqrtf_tsmom_12m" => Some(vec![p("months", "Look-back (months)", 12.0)]),
        "aqrtf_tsmom_combo" => Some(vec![p("m1", "Look-back A (months)", 1.0), p("m2", "Look-back B (months)", 3.0), p("m3", "Look-back C (months)", 12.0)]),
        _ => None,
    }
}

/// Indices of each calendar month's last bar in the FULL series (the final bar only counts once the next bar exists).
fn month_ends(x: &Ctx) -> Vec<usize> {
    (0..x.r.len().saturating_sub(1)).filter(|&i| x.r[i].date[..7] != x.r[i + 1].date[..7]).collect()
}

fn sign(v: f64) -> i32 {
    if v > 0.0 { 1 } else if v < 0.0 { -1 } else { 0 }
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let defs = params(id)?;
    if from >= to || to >= x.r.len() {
        return None;
    }
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| defs.iter().find(|d| d.key == k).unwrap().def).round().max(1.0) as usize;
    let looks: Vec<usize> = if id == "aqrtf_tsmom_combo" { vec![g("m1"), g("m2"), g("m3")] } else { vec![g("months")] };
    let me = month_ends(x);
    let mut out = Vec::new();
    let mut cur: Option<Trade> = None;
    for (n, &i) in me.iter().enumerate() {
        if i < from || i > to {
            continue;
        }
        if i == to {
            // a signal on the final bar has no holding period inside the window
            break;
        }
        // all look-backs need k earlier month-ends; otherwise no signal yet
        if looks.iter().any(|&k| n < k) {
            continue;
        }
        let s: i32 = looks.iter().map(|&k| sign(x.c[i] / x.c[me[n - k]] - 1.0)).sum();
        let want = sign(s as f64);
        if want == 0 || cur.as_ref().is_some_and(|t| t.dir as i32 == want) {
            continue;
        }
        if let Some(mut t) = cur.take() {
            t.exit_i = i;
            t.exit_px = x.c[i];
            t.signal_i = i;
            t.reason = "month-end sign flip · close".into();
            out.push(t);
        }
        cur = Some(Trade { dir: want as f64, legs: vec![Leg { i, px: x.c[i], w: 1.0 }], exit_i: i, exit_px: x.c[i],
            reason: String::new(), entry_kind: "close", signal_i: i, open: false, exits: vec![] });
    }
    if let Some(mut t) = cur {
        t.exit_i = to;
        t.exit_px = x.c[to];
        t.signal_i = to;
        t.reason = "open".into();
        t.open = true;
        out.push(t);
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn twelve_month_sign_flips_on_month_end_close() {
        // one bar per month on the 15th: every bar is a month-end; prices rise 24 months then fall
        let mut r = Vec::new();
        for k in 0..48 {
            let px = if k < 24 { 100.0 + k as f64 } else { 123.0 - 2.0 * (k - 23) as f64 };
            r.push(Row { ticker: "T".into(), date: format!("{:04}-{:02}-15", 2000 + k / 12, k % 12 + 1), open: px, high: px, low: px, close: px, volume: None });
        }
        let x = Ctx::new(&r, &r, &[]);
        let ts = run("aqrtf_tsmom_12m", &x, 0, 47, &Params::new()).unwrap();
        assert_eq!(ts[0].dir, 1.0);
        assert_eq!(ts[0].legs[0].i, 12); // first bar with 12 earlier month-ends
        // close < close 12 months earlier: 169-2k < 88+k -> k >= 28 (k=27 is a tie: hold)
        assert_eq!(ts[0].exit_i, 28);
        assert_eq!(ts[1].dir, -1.0);
        assert!(ts[1].open);
    }
}
