//! YT Strategies · Long Using Price Relative: the team's AmiBroker AFL "Long using Price Relative and Average - C.afl"
//! (Old Topics folder "Long Using Price Relative"). Coded from the AFL as written, with its own default Param values;
//! there is no book PDF and no trade sheet was compared.
//!
//! What runs: the AFL trades the chart symbol (long only) on its price relative to `Foreign(priceRelativeWith, "C")`,
//! default "NIFTY BANK": relative = Close / NIFTY BANK Close, compared with an average of it. Its rules read only
//! closes and moving averages, so they are coded on daily bars; the AFL names no timeframe and its NewDay histogram
//! is a chart marker only (no rule reads it). It runs on one series at a time: a daily spot index (e.g. NIFTY 50; on
//! NIFTY BANK itself the relative is constant and never trades) or one actual stock.
//!
//! Data: the NIFTY BANK close as the external series `NIFTYBANK.Close` (extra_data `NIFTYBANK.csv`: Date, Open, High,
//! Low, Close), aligned to the traded symbol's dates. Like AmiBroker's Foreign with the default fix-up a missing day
//! repeats the previous close; before the first NIFTY BANK bar it is Null. Without the series the rule gives no
//! trades (needs_series). The ParamStr "Price Relative with" is offered at its default (NIFTY BANK) only.
//!
//! AmiBroker behaviour followed (as in `ytusd.rs` / `ytwh.rs`):
//! - MA() / WMA() Null until `n` values exist; EMA() seeded with the simple average of the first `n` values; TEMA =
//!   3 EMA − 3 EMA(EMA) + EMA(EMA(EMA)); the AFL's "Triangular" is its own EMA(Sum(x, q) / q, q) with
//!   q = Round((n + 1.1) / 2), coded as written;
//! - "First Averages then Price Relative" (default) = average(Close) / average(NIFTY BANK); "First Price Relative then
//!   Average" = average(Close / NIFTY BANK);
//! - the ROC filter's toggle is inverted as written: `IIf(rocFilterSwitch, True, ROC(C, 10) >= 2)`, so at the default
//!   "OFF" the filter applies and "ON" removes it;
//! - the bar loop (from bar 1, over all loaded bars): flat and ROC filter and relative > average now and relative <
//!   average on the previous bar → Buy at the Close; in a position, relative < average → Sell at the Close (not on
//!   the entry bar: else-if); a Null element in the loop is -1e10; trades open only inside the test window and one
//!   still open at its end is marked at the last Close.
//!
//! Not coded / modelled: the AFL's portfolio settings (Initial Equity 1 crore, Compounding / Non-compounding
//! allocation, Maximum open position 1, commission 1% per leg); this is a single-symbol run. Other "Price Relative
//! with" symbols are not offered (only NIFTY BANK is loaded as an external series).
#![allow(dead_code)]
use crate::books::ytusd::{ema, sma, wma};
use crate::books::ytwh::{collect, loop_val, Ev};
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "yt_pr";
pub const PR_AVG: &str = "ytpr_price_relative_average";
pub const IDS: [&str; 1] = [PR_AVG];

/// The foreign series: NIFTY BANK close aligned to the traded symbol's dates.
pub const FOREIGN: &str = "NIFTYBANK.Close";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef {
        id: PR_AVG,
        name: "Long Using Price Relative (vs NIFTY BANK) and its Average",
        group: "YT Strategies · Long Using Price Relative",
        session: 0,
        dir: "Long",
        status: Status::Live,
        needs_volume: false,
        needs_vix: false,
        needs_series: &[FOREIGN],
        book: BOOK,
        rules: &[
            "F|Price relative = Close / Foreign(\"NIFTY BANK\", Close), aligned to the symbol's dates (a missing day repeats the previous NIFTY BANK close)",
            "F|Average (default 'First Averages then Price Relative', Simple, period 8) = MA(Close, 8) / MA(NIFTY BANK, 8); option: the average of the relative itself; types Simple, Exponential, Triple (TEMA), Triangular (the AFL's EMA of a q-bar mean, q = Round((n + 1.1) / 2)), Weighted",
            "F|ROC filter: ROC(Close, 10) >= 2%; applied at the AFL's default toggle 'OFF' (the AFL's IIf is inverted: 'ON' removes the filter)",
            "E|Buy at the Close when flat, the ROC filter passes, the relative is above its average and was below it on the previous bar",
            "X|Sell at the Close when the relative is below its average (not on the entry bar)",
            "F|Long only; one position at a time; daily bars (the AFL's rules read only closes; its NewDay marker is a chart plot)",
        ],
        notes: "Long using Price Relative and Average - C.afl (folder 'Long Using Price Relative'), coded as the AFL is written with its default Param values (Moving Average period 8, Price Relative with NIFTY BANK, First Averages then Price Relative, Simple, ROC filter toggle OFF = filter applied, ROC period 10, threshold 2). Needs the NIFTY BANK close as the external series NIFTYBANK.Close. Runs on one daily spot index or one actual stock; on NIFTY BANK itself the relative is constant (no trades). Its portfolio settings (Initial Equity 1 crore, compounding allocation, Maximum open position 1, commission 1% per leg) are not modelled in this single-symbol run. Not compared with the team's results.",
        claim: None,
    }]
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    match id {
        PR_AVG => Some(vec![
            p("ma_period", "Moving Average period", 8.0, 1.0, 50.0, 1.0),
            p("ratio_type", "Price Relative Method (0 first averages then price relative, 1 first price relative then average)", 0.0, 0.0, 1.0, 1.0),
            p("ma_type", "Moving Average Type (1 Simple, 2 Exponential, 3 Triple, 4 Triangular, 5 Weighted)", 1.0, 1.0, 5.0, 1.0),
            p("roc_switch", "ROC Filter toggle as written (0 'OFF' = filter applied, 1 'ON' = no filter)", 0.0, 0.0, 1.0, 1.0),
            p("roc_period", "ROC Period", 10.0, 1.0, 100.0, 1.0),
            p("roc_threshold", "ROC Threshold %", 2.0, 0.0, 10.0, 0.1),
        ]),
        _ => None,
    }
}

// ---------------------------------------------------------------- indicators

/// Foreign(symbol, "C") with AmiBroker's default fix-up: a missing day repeats the previous close; NaN (Null) before
/// the first bar. None without the series.
fn foreign_close(x: &Ctx) -> Option<Vec<f64>> {
    let mut f = x.series(FOREIGN)?.to_vec();
    for i in 1..f.len() {
        if !f[i].is_finite() {
            f[i] = f[i - 1];
        }
    }
    Some(f)
}

/// The AFL's moving averages: 1 MA, 2 EMA, 3 TEMA, 4 its "Triangular" EMA(Sum(v, q) / q, q), 5 WMA. NaN = Null.
fn average(v: &[f64], n: usize, kind: i64) -> Vec<f64> {
    let n = n.max(1);
    match kind {
        2 => ema(v, n),
        3 => {
            let e1 = ema(v, n);
            let e2 = ema(&e1, n);
            let e3 = ema(&e2, n);
            (0..v.len()).map(|i| 3.0 * e1[i] - 3.0 * e2[i] + e3[i]).collect()
        }
        4 => {
            let q = ((n as f64 + 1.1) / 2.0).round().max(1.0) as usize;
            ema(&sma(v, q), q)
        }
        5 => wma(v, n),
        _ => sma(v, n),
    }
}

/// AFL ROC(C, n) in percent; Null for the first n bars.
fn roc(c: &[f64], n: usize) -> Vec<Option<f64>> {
    (0..c.len()).map(|i| if i < n { None } else { Some((c[i] - c[i - n]) / c[i - n] * 100.0) }).collect()
}

fn opt(v: f64) -> Option<f64> {
    if v.is_finite() {
        Some(v)
    } else {
        None
    }
}

// ---------------------------------------------------------------- the AFL

/// The AFL's bar loop over all loaded bars: (relative, average) and the positions it takes.
fn positions(x: &Ctx, f: &[f64], g: &dyn Fn(&str) -> f64) -> Vec<Ev> {
    let c = &x.c;
    let n = c.len();
    let gi = |k: &str| g(k).round() as i64;
    let period = g("ma_period").round().max(1.0) as usize;
    let kind = gi("ma_type");
    let rel: Vec<f64> = (0..n).map(|i| c[i] / f[i]).collect();
    let avg: Vec<f64> = if gi("ratio_type") == 1 {
        average(&rel, period, kind)
    } else {
        let (a, b) = (average(c, period, kind), average(f, period, kind));
        (0..n).map(|i| a[i] / b[i]).collect()
    };
    let r = roc(c, g("roc_period").round().max(1.0) as usize);
    let roc_ok = |i: usize| gi("roc_switch") == 1 || r[i].map_or(false, |v| v >= g("roc_threshold"));
    let (rv, av) = (|i: usize| loop_val(opt(rel[i])), |i: usize| loop_val(opt(avg[i])));
    let mut evs: Vec<Ev> = Vec::new();
    let mut open = false;
    for i in 1..n {
        if !open && roc_ok(i) && rv(i) > av(i) && rv(i - 1) < av(i - 1) {
            evs.push(Ev { dir: 1.0, e: i, epx: c[i], exit: None });
            open = true;
        } else if open && rv(i) < av(i) {
            if let Some(v) = evs.last_mut() {
                v.exit = Some((i, c[i], i, "Sell: price relative below its average"));
            }
            open = false;
        }
    }
    evs
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    let n = x.c.len();
    if n == 0 || from > to || to >= n {
        return Some(Vec::new());
    }
    let Some(f) = foreign_close(x) else { return Some(Vec::new()) };
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(0.0, |d| d.def));
    Some(collect(x, positions(x, &f, &g), from, to))
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let d = catalogue().into_iter().find(|d| d.id == id)?;
    Some((d.id, d.name, "Long Using Price Relative"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::books::ytwh::testbars::*;
    use crate::Row;

    fn bar(c: f64) -> (f64, f64, f64, f64) {
        (c + 0.3, c + 1.0, c - 1.0, c)
    }
    fn run_with(r: &[Row], f: &[f64], kv: &[(&str, f64)]) -> Vec<(usize, f64, usize, f64, bool)> {
        let mut x = Ctx::new(r, r, &[]);
        x.extra.insert(FOREIGN.into(), f.to_vec());
        run(PR_AVG, &x, 0, r.len() - 1, &ps(kv)).unwrap().into_iter().map(|t| (t.legs[0].i, t.legs[0].px, t.exit_i, t.exit_px, t.open)).collect()
    }

    #[test]
    fn buys_on_the_relative_crossing_its_average_with_the_roc_filter() {
        // NIFTY BANK flat at 100: relative = C / 100 and its average = MA(C, 8) / 100
        let mut v: Vec<(f64, f64, f64, f64)> = (0..20).map(|_| bar(100.0)).collect();
        v[19] = bar(99.0); // relative below its average on the bar before the cross
        v.extend([bar(103.0), bar(104.0), bar(95.0), bar(95.0)]);
        let r = bars(&v);
        let f = vec![100.0; r.len()];
        // bar 20: 103 / 100 above (4 x 100 + 99 + ... ) / 8, ROC(10) = 3% >= 2 -> buy; bar 22: 95 below the average -> sell
        assert_eq!(run_with(&r, &f, &[]), vec![(20, 103.0, 22, 95.0, false)]);
        // a 5% ROC threshold blocks it
        assert!(run_with(&r, &f, &[("roc_threshold", 5.0)]).is_empty());
        // without the series: no trades
        assert!(go(run, PR_AVG, &r, &[]).is_empty());
    }

    #[test]
    fn the_foreign_close_drives_the_relative_and_the_toggle_is_inverted() {
        // the stock is flat; NIFTY BANK falls on bar 20, so the relative rises above its average
        let r = bars(&(0..24).map(|_| bar(100.0)).collect::<Vec<_>>());
        let mut f = vec![100.0; 24];
        f[19] = 101.0;
        f[20] = 97.0;
        f[21] = f64::NAN; // a missing NIFTY BANK day repeats 97
        f[22] = 104.0;
        f[23] = 104.0;
        // default toggle 'OFF' applies the ROC filter: ROC(C, 10) = 0 -> no trade
        assert!(run_with(&r, &f, &[]).is_empty());
        // toggle 'ON' removes it: buy at bar 20, the relative falls below its average on bar 22
        assert_eq!(run_with(&r, &f, &[("roc_switch", 1.0)]), vec![(20, 100.0, 22, 100.0, false)]);
        // the relative-first average method gives the same cross here
        assert_eq!(run_with(&r, &f, &[("roc_switch", 1.0), ("ratio_type", 1.0)])[0].0, 20);
    }

    #[test]
    fn averages_follow_amibroker() {
        let v: Vec<f64> = (1..=6).map(|k| k as f64).collect();
        assert!(average(&v, 3, 1)[1].is_nan());
        assert_eq!(average(&v, 3, 1)[2], 2.0);
        // EMA seeded with the mean of the first 3 values, then k = 0.5
        assert_eq!(average(&v, 3, 2)[3], 3.0);
        // WMA(3) on 1, 2, 3 = (1 + 4 + 9) / 6
        assert!((average(&v, 3, 5)[2] - 14.0 / 6.0).abs() < 1e-12);
        // TEMA of a straight line is the line itself once all three EMAs exist
        let t = average(&v, 2, 3);
        assert!((t[5] - 6.0).abs() < 1e-9);
        assert_eq!(roc(&[100.0, 103.0], 1), vec![None, Some(3.0)]);
    }

    #[test]
    fn every_strategy_runs() {
        let r = wave(400);
        let mut x = Ctx::new(&r, &r, &[]);
        x.extra.insert(FOREIGN.into(), (0..400).map(|k| 100.0 + 5.0 * (k as f64 / 17.0).sin()).collect());
        for id in IDS {
            assert!(family_of(id).is_some());
            for kind in 1..=5 {
                for rt in [0.0, 1.0] {
                    let p = ps(&[("ma_type", kind as f64), ("ratio_type", rt), ("roc_switch", 1.0)]);
                    for (from, to) in [(0, 399), (50, 300)] {
                        for t in run(id, &x, from, to, &p).unwrap() {
                            assert!(t.legs[0].i >= from && t.exit_i <= to && t.exit_i >= t.legs[0].i);
                        }
                    }
                }
            }
            assert!(!run(id, &x, 0, 399, &ps(&[("roc_switch", 1.0)])).unwrap().is_empty());
        }
    }
}
