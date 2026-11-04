//! Joe Luisi, "Monitoring Equity For Market Analysis", Technical Analysis of Stocks & Commodities V.11:4 (1993) pp.166-168, with the
//! sidebar "Least-squares cycle and the time series moving average" (research share Trading Knowledge I/Stocks & Commodities DVD
//! 11.26 (1982 - 2008) (traders.com)/VOLUMES/V11/C04/MONITOR.pdf, SIDELEA.pdf). Book id `scjlme`: a least-squares oscillator system
//! and a time-series MA crossover, each traded only while its own equity is above the equity's 25-day average.
//! Lane K41 (batch T7a, 4 Nov 2026); independent check authors/titles-1030/K41w/k41_ref.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::msws::sma;
use super::scdkdmi::{at_close, close_equity, gt, ols_end};

pub const REVIEWED: &[&str] = &["scjlme_osc", "scjlme_trend"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] Least-squares value = Excel TREND over the last n closes evaluated at the last point (=TREND(D2:D14,,13)); comparisons ignore floating-point noise (an exact tie is no cross and no turn). Oscillator system: osc = 3-day simple average of (close - 13-day least-squares value); 'Sell signals are taken when the oscillator rises above the zero line and then turns down' = the previous value is above zero and the oscillator falls after rising (osc[t-1] > 0, osc[t] < osc[t-1], osc[t-1] >= osc[t-2]); buys mirrored. Trend system: 10-day time-series MA (least-squares value) of the lows crossing the 10-day simple average of the lows. Each system is a stop-and-reverse shadow system from the first bar of the data, positions at the signal close ('Signals are taken on the close of the day of the signal'); its equity = the sum of its price-point changes marked at each close (the $10,000 start does not move the crossings; the $16 round-turn commission is left out: a dollar amount on currency contracts has no meaning on the vault's instruments (decision 166), and a per-trade cost would step the equity down at each trade, which can move some crossings). The published position = the shadow position while the equity closes above its 25-day simple average, else flat ('A trader can enter when the equity rises above the moving average in the direction of the latest signal ... and exited when the equity fell below'); changes at the close. Option filter = 0 publishes the shadow system. The article runs both systems together and takes no signals when both equities are below their averages; each card here is one system with its own filter. Article tests: Canadian dollar and Swiss franc, March-August 1992 (charts only). Runs on indices, ETFs and stocks, US and India, are a declared application.";

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, rules| StrategyDef { id, name, group: "S&C: monitoring equity (Luisi, V.11:4)", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "scjlme", rules, notes: NOTE, claim: None };
    const F: &str = "F|\"By tracking the equity, a trader can invest more capital into the system whose equity is above the moving average and ignore signals from the system whose equity is below the moving average.\" (V.11:4, PDF p.1); \"A trader can enter when the equity rises above the moving average in the direction of the latest signal, which was short at 1.4276 and exited when the equity fell below at 1.3151.\" (PDF p.2)";
    const X: &str = "X|The opposite shadow signal reverses the position at the signal close; the published position goes flat at the close on which the equity is not above its 25-day average (\"exited when the equity fell below\", MONITOR.pdf p.2)";
    vec![
        d("scjlme_osc", "13-day least-squares oscillator (3-day average) turns, traded only while its equity is above its 25-day average", &[
            "F|\"the difference between the current closing price and the current value of the least-squares regression line is determined. This value is smoothed with an n period moving average\"; \"=TREND(D2:D14,,13)\"; \"=AVERAGE(F15:F17)\" (SIDELEA.pdf p.1-2)",
            "E|\"Sell signals are taken when the oscillator rises above the zero line and then turns down. Buy signals occur when the indicator goes below the zero line and turns up. Signals are taken on the close of the day of the signal.\" (MONITOR.pdf p.2)", F, X]),
        d("scjlme_trend", "10-day time-series MA of the lows crossing their 10-day SMA, traded only while its equity is above its 25-day average", &[
            "E|\"The first is a 10-day simple moving average and the second is a 10-day time series moving average (see sidebar). Both moving averages are calculated using the daily lows ... Buys occur when the time series moving average crosses above the simple moving average and sells occur when the time series moving average crosses below\" (MONITOR.pdf p.2); \"=TREND(D5:D14,,10)\" (SIDELEA.pdf p.2)", F, X]),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let v = match id { "scjlme_osc" => "Least-squares oscillator", "scjlme_trend" => "Time-series MA of lows", _ => return None };
    Some(("scjlme_equity", "Equity monitoring (Luisi, S&C)", v))
}
pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    reviewed(id).then(|| vec![
        ParamDef { key: "ma", label: "Equity moving average days", def: 25., min: 2., max: 60., step: 1. },
        ParamDef { key: "filter", label: "1 = trade only while the equity is above its average; 0 = the shadow system", def: 1., min: 0., max: 1., step: 1. }])
}

// Deadband bound (decision 209). Prices on a tick grid make every real value a multiple of a fixed quantum: the 13-day
// least-squares end value is sum (3k-11)/91 x C, so the oscillator (3-day mean of close minus that value) and its day-to-day
// change are multiples of tick/273 (gcd of the integer coefficients is 1); the 10-day TSMA minus the 10-day SMA of the lows is
// sum 3(2k-9)/110 x L, a multiple of 3 x tick/110. The deadband is 1e-12 x price (decision 210; 1e-9 capped out near 36,600 on a 0.01 tick, under the DJI) (relative, as gt() in scdkdmi.rs), so a real
// one-quantum turn, zero-line move or cross is detected while price < tick / 273e-9: about 36,600 on a 0.01 tick and 183,000 on
// a 0.05 tick for the oscillator (272,700 / 1.36 million for the trend cross). Floating-point noise on an exact mathematical tie
// is about 1e-16 x price per operation (NIFTY 50 2004-08-02: -1.5e-13 at 1,639), far inside the band. Above those levels (DJI on
// a 0.01 grid has traded above 36,600 since 2021) a one-quantum oscillator turn could fall inside the band; with 1e-12 instead of
// 1e-9 the two systems give the same trades on every index and ETF the vault holds (checked 4 Nov 2026, K41w/eps.py). Split-
// adjusted stock prices are not on a tick grid, so there the band only removes ties at the 1e-12 level.
/// Oscillator signal at the close of t: -1 = it was above the zero line and turned down, +1 = below and turned up, 0 = none.
pub fn osc_turn(o: &[f64], c: &[f64], t: usize) -> f64 {
    if t < 2 || !o[t - 2].is_finite() { return 0.0; }
    let z = 1e-12 * c[t - 1]; // price-level noise: the zero line and the turns ignore changes within it
    if o[t - 1] > z && o[t - 1] - o[t] > z && !(o[t - 2] - o[t - 1] > z) { -1.0 }
    else if o[t - 1] < -z && o[t] - o[t - 1] > z && !(o[t - 1] - o[t - 2] > z) { 1.0 } else { 0.0 }
}
/// Trend signal at the close of t: +1 = the time-series MA crossed above the SMA, -1 = below, 0 = none.
pub fn trend_cross(ts: &[f64], s: &[f64], t: usize) -> f64 {
    if t < 1 || !s[t - 1].is_finite() { return 0.0; }
    if gt(ts[t], s[t]) && !gt(ts[t - 1], s[t - 1]) { 1.0 } else if gt(s[t], ts[t]) && !gt(s[t - 1], ts[t - 1]) { -1.0 } else { 0.0 }
}
/// The oscillator of the oscillator system: 3-day SMA of (close - 13-day least-squares end value).
pub fn oscillator(c: &[f64]) -> Vec<f64> {
    let ls = ols_end(c, 13);
    sma(&(0..c.len()).map(|i| c[i] - ls[i]).collect::<Vec<f64>>(), 3)
}

/// Shadow position (+1/-1/0) after each close.
fn shadow(id: &str, x: &Ctx) -> Vec<f64> {
    let n = x.c.len();
    let mut pos = vec![0.0; n];
    let mut side = 0.0;
    let (o, ts, s) = if id == "scjlme_osc" { (oscillator(&x.c), vec![], vec![]) } else { (vec![], ols_end(&x.l, 10), sma(&x.l, 10)) };
    for t in 1..n {
        let sig = if id == "scjlme_osc" { osc_turn(&o, &x.c, t) } else { trend_cross(&ts, &s, t) };
        if sig != 0.0 { side = sig; }
        pos[t] = side;
    }
    pos
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let nm = p.get("ma").copied().unwrap_or(25.).round().max(1.) as usize;
    let filt = p.get("filter").copied().unwrap_or(1.) > 0.5;
    let sh = shadow(id, x);
    let eq = close_equity(&x.c, &sh);
    let ma = sma(&eq, nm);
    Some(at_close(x, from, to, "Equity below its average", |t, _| if !filt || eq[t] > ma[t] { sh[t] } else { 0.0 }))
}

#[cfg(test)]
mod tests {
    use super::*;
    // Decision 209: integer tick offsets (found by an exact integer solve, K41w/solve.py; quanta from K41w/gcd.py) that give the smallest real
    // signals on top of a flat price: oscillator values (0, +1, 0) and (0, -1, 0) quanta of tick/273 at bars 34-36, a zero-line
    // cross (0, +1, -1), and TSMA - SMA differences (0, +-3) of tick/110 at bars 35-36. Offsets start at bar 16.
    const OSC_DOWN: [i64; 21] = [0, 0, 0, 0, 5, 3, 1, -3, 1, 0, 1, 1, -2, 1, -2, -2, -3, -2, -2, -3, -3];
    const OSC_CROSS: [i64; 21] = [0, 0, 0, 0, 3, 1, 1, -3, 2, 0, 2, 2, -1, 2, -1, -2, -1, 0, -1, -1, -1];
    const TREND_UP: [i64; 8] = [-2, 6, 0, 0, 0, 0, 0, 1]; // bars 29-36
    /// NIFTY 50 closes 2004-07-09 .. 2004-08-03 (vault idx_data.csv).
    const NIFTY_2004: [f64; 18] = [1553.2, 1556.95, 1539.3, 1522.75, 1539.4, 1558.8, 1571.6, 1566.1, 1581.4, 1598.1, 1601.6, 1618.0, 1600.75, 1594.15, 1618.7, 1632.3, 1639.05, 1630.6];
    fn series(level: f64, tick: f64, offs: &[i64], start: usize) -> Vec<f64> {
        let base = (level / tick).round() as i64;
        (0..40).map(|i| (base + if i >= start && i < start + offs.len() { offs[i - start] } else { 0 }) as f64 * tick).collect()
    }
    #[test]
    fn smallest_real_tick_signals_are_detected_and_exact_zeros_are_not() {
        for tick in [0.01, 0.05] {
            for level in [10., 100., 1_000., 25_000.] {
                let neg = |v: &[i64]| v.iter().map(|x| -x).collect::<Vec<i64>>();
                for (offs, want) in [(OSC_DOWN.to_vec(), -1.0), (neg(&OSC_DOWN), 1.0), (OSC_CROSS.to_vec(), -1.0), (neg(&OSC_CROSS), 1.0)] {
                    let c = series(level, tick, &offs, 16);
                    let o = oscillator(&c);
                    let q = tick / 273.0;
                    assert!((o[35].abs() - q).abs() < q * 1e-3 && o[34].abs() < q * 1e-3, "quantum {level} {tick}: {} {}", o[34], o[35]);
                    assert_eq!(osc_turn(&o, &c, 36), want, "oscillator turn {level} {tick}");
                }
                for (offs, want) in [(TREND_UP.to_vec(), 1.0), (TREND_UP.iter().map(|x| -x).collect::<Vec<i64>>(), -1.0)] {
                    let l = series(level, tick, &offs, 29);
                    let (ts, s) = (ols_end(&l, 10), sma(&l, 10));
                    assert!(((ts[36] - s[36]).abs() - 3.0 * tick / 110.0).abs() < tick * 1e-6, "trend quantum {level} {tick}");
                    assert_eq!(trend_cross(&ts, &s, 36), want, "trend cross {level} {tick}");
                }
                // exact mathematical zeros: flat and straight-line prices give an oscillator of exactly 0 (any computed value is
                // noise) and TSMA = SMA + 4.5 x slope with no cross; 'rises above the zero line and then turns down' needs a value
                // off the line, so no bar signals
                for slope in [0i64, 1, -1, 7] {
                    let c: Vec<f64> = (0..60).map(|i| ((level / tick).round() as i64 + slope * i) as f64 * tick).collect();
                    let o = oscillator(&c);
                    let (ts, s) = (ols_end(&c, 10), sma(&c, 10));
                    assert!((2..60).all(|t| osc_turn(&o, &c, t) == 0.0), "flat/linear oscillator {level} {tick} {slope}");
                    assert!((1..60).all(|t| trend_cross(&ts, &s, t) == 0.0), "flat/linear trend {level} {tick} {slope}");
                }
            }
        }
        // NIFTY 50 2004-07-09 .. 2004-08-03: the oscillator of 2 Aug 2004 is exactly 0 (computed
        // -1.5e-13); it never rose above the zero line, so 3 Aug 2004 is no sell
        let c = NIFTY_2004.to_vec();
        let o = oscillator(&c);
        assert!(o[c.len() - 2].abs() < 1e-9);
        assert_eq!(osc_turn(&o, &c, c.len() - 1), 0.0);
    }
    #[test]
    fn shadow_and_filter() {
        let r = super::super::sctraub::tests::bars(1500);
        let x = Ctx::new(&r, &r, &[]);
        for id in REVIEWED {
            let mut p = Params::new(); p.insert("filter".into(), 0.);
            let all = run(id, &x, 0, r.len() - 1, &p).unwrap();
            let f = run(id, &x, 0, r.len() - 1, &Params::new()).unwrap();
            assert!(all.len() > 5 && !f.is_empty() && all.windows(2).all(|w| w[0].dir != w[1].dir), "{id}");
        }
    }
}
