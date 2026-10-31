//! Wayne A. Thorp, two AAII Journal technical-analysis articles (2000; research share Trading Knowledge III/!PDFs/): "Measuring
//! Internal Strength: Wilder's RSI Indicator" (May 2000) and "When to Buy & Sell Using the Stochastic Oscillator" (Oct 2000).
//! Book id `wtaaii`. Two rules: the nine-day RSI 30/70 crossover and the slow stochastic 20/80 cross-back. Lane K8 (titles
//! loop, 1 Nov 2026); independent check authors/titles-1030/K8w/k8_ref.py (header).
use crate::connors::{rsi, sma, Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::mkits::{hhv, llv};
use super::rbtie::target_trades;

/// Admitted ids (authors/titles-1030/K8w/k8_ref.py (header)).
pub const REVIEWED: &[&str] = &["wtaaii_rsi9", "wtaaii_stoch"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }
/// An indicator sitting exactly on a level (a stochastic of 20.000000000000004 from penny prices) counts as on it, not
/// beyond it: levels are compared with a 1e-9 allowance so that rounding in the averages cannot decide a crossing.
const EPS: f64 = 1e-9;

const RSI_NOTE: &str = "[Declared readings] Wilder's RSI (the article's figures were made with MetaStock: averages of up and down closes smoothed Wilder's way) over nine days. 'Crosses above 30' = above 30 at today's close after being at or below 30 the day before; 'crosses below 70' mirrored. Long only, as the article counts round trips (option: also short, so that the cross below 70 reverses to short and the cross above 30 covers). Signals and fills at the close (MetaStock's default; with close fills our MSFT run gives the article's five round trips and +106.45% compounded vs the printed +106.5%; next-open fills, an option, give +100.8%). No stop in the article, none added. Options from the same page: 14 days (Figure 1) and the 'strict' version (buy when the RSI falls below 30, sell when it rises above 70). Printed check: Microsoft, 30 Mar 1998 - 28 Mar 2000, 'five round-trip trades ... returned a 106.5% profit'. Indices, ETFs and India are a declared application.";
const STO_NOTE: &str = "[Declared readings] Slow stochastic (5,3): fast %K = 100 x (close - 5-day lowest low) / (5-day highest high - 5-day lowest low), slow %K = its 3-day simple average, slow %D = the 3-day simple average of slow %K (the article's '(5,3)' and 'five-day stochastics (with three-day slowing)'). The signal line is %D ('most technicians consider the %D line to be more accurate'; %K is an option). Buy when %D rises back above 20 (above 20 today, at or below the day before); sell when it falls back below 80. Long only (option: also short, reversing at the same crossings). Signals and fills at the close, like the RSI article (same author, MetaStock figures); next-open fills are an option. No stop given, none added. Option: the 'strictest rule' (buy when %D falls below 20, sell when it rises above 80). The %K/%D crossover inside the zones is another strategy on the same page, not coded. No printed results. Indices, ETFs and India are a declared application.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![
        StrategyDef { id: "wtaaii_rsi9", name: "Nine-day RSI crossover: buy when the RSI crosses above 30, sell when it crosses below 70", group: "Thorp AAII indicator articles",
            session: 0, dir: "Long", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: "wtaaii", rules: &[
                "F|\"RS = Avg. price change on up days / Avg. price change on down days ... RSI = 100 - [100 / (1 + RS)]\"; nine-day RSI (May 2000, p.29-30)",
                "E|\"You could also sell when the RSI crosses below 70 and buy when it crosses above 30. This also happens to be a popular trading strategy when using the nine-day RSI\" (p.30)",
                "X|Sell when the RSI crosses below 70 (p.30)"],
            notes: RSI_NOTE, claim: None },
        StrategyDef { id: "wtaaii_stoch", name: "Slow stochastic cross-back: buy when %D rises back above 20, sell when it falls back below 80", group: "Thorp AAII indicator articles",
            session: 0, dir: "Long", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: "wtaaii", rules: &[
                "F|Slow stochastic (5,3): %K smoothed by a 3-period average, %D = 3-period average of %K (Oct 2000, p.25-26)",
                "E|\"wait until it rises above 20 before buying\" (p.27)",
                "X|\"wait until it crosses back below 80 before selling\" (p.27)"],
            notes: STO_NOTE, claim: None },
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match id {
        "wtaaii_rsi9" => Some(("wtaaii_rsi9", "Thorp RSI crossover", "RSI 9, 30/70")),
        "wtaaii_stoch" => Some(("wtaaii_stoch", "Thorp stochastic cross-back", "slow %D (5,3), 20/80")),
        _ => None,
    }
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    let common = || vec![p("mode", "1 = the 'strict' version (buy below the low line, sell above the high line); default 0", 0., 0., 1., 1.),
        p("short", "1 = also short (reverse at the same crossings); default 0 long only", 0., 0., 1., 1.),
        p("close", "1 = fill at the signal close (default, reproduces the printed MSFT result); 0 = next open", 1., 0., 1., 1.)];
    match id {
        "wtaaii_rsi9" => Some([p("n", "RSI days (9; Figure 1 uses 14)", 9., 2., 50., 1.)].into_iter().chain(common()).collect()),
        "wtaaii_stoch" => Some([p("line", "1 = trade slow %K instead of %D", 0., 0., 1., 1.)].into_iter().chain(common()).collect()),
        _ => None,
    }
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(f64::NAN, |d| d.def));
    let (v, lo, hi) = if id == "wtaaii_rsi9" { (rsi(&x.c, g("n").round().max(1.) as usize), 30., 70.) } else {
        let (hh, ll) = (hhv(&x.h, 5), llv(&x.l, 5));
        let fk: Vec<f64> = (0..x.c.len()).map(|i| if hh[i].is_finite() && hh[i] != ll[i] { 100. * (x.c[i] - ll[i]) / (hh[i] - ll[i]) } else { f64::NAN }).collect();
        let k = sma(&fk, 3);
        (if g("line") >= 1. { k } else { sma(&k, 3) }, 20., 80.)
    };
    let strict = g("mode") >= 1.;
    let short = if g("short") >= 1. { -1 } else { 0 };
    let mut tgt = vec![0i8; x.c.len()];
    for i in 1..x.c.len() {
        let (buy, sell) = if strict { (v[i] < lo - EPS && v[i - 1] >= lo - EPS, v[i] > hi + EPS && v[i - 1] <= hi + EPS) }
            else { (v[i] > lo + EPS && v[i - 1] <= lo + EPS, v[i] < hi - EPS && v[i - 1] >= hi - EPS) };
        tgt[i] = if buy { 1 } else if sell { short } else { tgt[i - 1] };
    }
    Some(target_trades(x, &tgt, from, to, g("close") < 1., "crossed the opposite line"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn rsi_cross_back_long_only() {
        let c: Vec<f64> = (0..60).map(|i| match i { 0..=19 => 100. - i as f64, 20..=44 => 81. + (i - 20) as f64 * 1.5, _ => 118.5 - (i - 44) as f64 * 2. }).collect();
        let r: Vec<Row> = c.iter().enumerate().map(|(i, &v)| Row { ticker: "T".into(), date: format!("2001-{:02}-{:02}", 1 + i / 28, 1 + i % 28), open: v, high: v + 0.5, low: v - 0.5, close: v, volume: None }).collect();
        let x = Ctx::new(&r, &r, &[]);
        let t = run("wtaaii_rsi9", &x, 0, r.len() - 1, &Params::new()).unwrap();
        let v = rsi(&x.c, 9);
        assert_eq!(t.len(), 1);
        let s = t[0].legs[0].i; // close fills: the signal bar itself
        assert!(v[s] > 30. && v[s - 1] <= 30. && t[0].dir == 1. && t[0].entry_kind == "close");
        assert!(v[t[0].exit_i] < 70. && v[t[0].exit_i - 1] >= 70.);
    }
}

// Instrument admission: Wayne A. Thorp (AAII Journal articles) 2026-11-01 (titles lane K8).
// Admission only for the module REVIEWED lists (authors/titles-1030/K8w/k8_ref.py (header)).
// Runs on actual indices, ETFs and stocks, US and India (index_etf_or_stock, nifty50 basket, declared application).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "wtaaii" && crate::books::wtaaii::reviewed(def.id),
    label: "Indicator articles illustrated on US stocks (Microsoft for the RSI rule); ETFs, US/India indices and India stocks are a declared application.",
    basis: "Wayne A. Thorp, AAII Journal May 2000 p.29-30 (RSI) and Oct 2000 p.25-28 (stochastic); independent check authors/titles-1030/K8w/k8_ref.py",
};
