//! TradingView `ta.*` primitives. Windows over "length non-na values" (sma/ema/rma/sum/stdev/dev) skip NaN
//! bars, which output NaN; bar-counted windows (linreg/change) let one NaN blank the next `n` outputs.
use super::bars::Bars;
pub const NA: f64 = f64::NAN;
pub fn nz(x: f64) -> f64 { if x.is_nan() { 0. } else { x } }

/// Calls `f(window)` with the last `n` non-NaN values for every non-NaN bar once `n` exist.
fn compact<F: Fn(&[f64]) -> f64>(x: &[f64], n: usize, f: F) -> Vec<f64> {
    let mut out = vec![NA; x.len()];
    let mut win: Vec<f64> = Vec::new();
    for (i, &v) in x.iter().enumerate() {
        if v.is_nan() { continue; }
        win.push(v);
        if win.len() > n { win.remove(0); }
        if win.len() == n { out[i] = f(&win); }
    }
    out
}
pub fn sum(x: &[f64], n: usize) -> Vec<f64> { compact(x, n, |w| w.iter().sum()) }
pub fn sma(x: &[f64], n: usize) -> Vec<f64> { compact(x, n, |w| w.iter().sum::<f64>() / n as f64) }
fn smoothed(x: &[f64], n: usize, alpha: f64) -> Vec<f64> {
    let seed = sma(x, n);
    let mut out = vec![NA; x.len()];
    let mut prev = NA;
    for i in 0..x.len() {
        if x[i].is_nan() { continue; }
        prev = if prev.is_nan() { seed[i] } else { prev + alpha * (x[i] - prev) };
        out[i] = prev;
    }
    out
}
/// EMA seeded with the SMA of the first n non-NaN values (verified on TradingView 2026-10-14).
pub fn ema(x: &[f64], n: usize) -> Vec<f64> { smoothed(x, n, 2. / (n as f64 + 1.)) }
/// Wilder / RMA, alpha 1/n, SMA seed.
pub fn rma(x: &[f64], n: usize) -> Vec<f64> { smoothed(x, n, 1. / n as f64) }
pub fn wma(x: &[f64], n: usize) -> Vec<f64> {
    let den = (n * (n + 1)) as f64 / 2.;
    compact(x, n, |w| w.iter().enumerate().map(|(j, v)| v * (j + 1) as f64).sum::<f64>() / den)
}
pub fn stdev(x: &[f64], n: usize) -> Vec<f64> {
    compact(x, n, |w| { let m = w.iter().sum::<f64>() / n as f64; (w.iter().map(|v| { let d = v - m; if d.abs() <= 1e-10 { 0. } else { d * d } }).sum::<f64>() / n as f64).sqrt() })
}
pub fn dev(x: &[f64], n: usize) -> Vec<f64> {
    compact(x, n, |w| { let m = w.iter().sum::<f64>() / n as f64; w.iter().map(|v| (v - m).abs()).sum::<f64>() / n as f64 })
}
fn window_extreme(x: &[f64], n: usize, max: bool) -> Vec<f64> {
    let mut out = vec![NA; x.len()];
    let mut start = 0; // reset after a NaN (TradingView G6)
    for i in 0..x.len() {
        if x[i].is_nan() { start = i + 1; continue; }
        if i + 1 < n { continue; }
        let lo = start.max(i + 1 - n);
        let w = &x[lo..=i];
        out[i] = if max { w.iter().cloned().fold(f64::MIN, f64::max) } else { w.iter().cloned().fold(f64::MAX, f64::min) };
    }
    out
}
pub fn highest(x: &[f64], n: usize) -> Vec<f64> { window_extreme(x, n, true) }
pub fn lowest(x: &[f64], n: usize) -> Vec<f64> { window_extreme(x, n, false) }
pub fn change(x: &[f64], k: usize) -> Vec<f64> { (0..x.len()).map(|i| if i >= k { x[i] - x[i - k] } else { NA }).collect() }
pub fn tr(b: &Bars, handle_na: bool) -> Vec<f64> {
    (0..b.len()).map(|i| if i == 0 {
        if handle_na { b.high[0] - b.low[0] } else { NA }
    } else {
        let pc = b.close[i - 1];
        (b.high[i] - b.low[i]).max((b.high[i] - pc).abs()).max((b.low[i] - pc).abs())
    }).collect()
}
/// Least-squares value at offset `off` from the newest bar (k = 1..n oldest→newest).
pub fn linreg(x: &[f64], n: usize, off: usize) -> Vec<f64> {
    let mut out = vec![NA; x.len()];
    for i in n.saturating_sub(1)..x.len() {
        let w = &x[i + 1 - n..=i];
        if w.iter().any(|v| v.is_nan()) { continue; }
        let nf = n as f64;
        let (mut sk, mut sy, mut skk, mut sky) = (0., 0., 0., 0.);
        for (j, &y) in w.iter().enumerate() { let k = (j + 1) as f64; sk += k; sy += y; skk += k * k; sky += k * y; }
        let den = nf * skk - sk * sk;
        let slope = if den == 0. { 0. } else { (nf * sky - sk * sy) / den };
        let intercept = sy / nf - slope * sk / nf + slope;
        out[i] = intercept + slope * (nf - 1. - off as f64);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lab::testdata::{assert_series, from_ohlc};
    const N: f64 = f64::NAN;

    #[test] fn sma_basic_and_na_compaction() {
        assert_series(&sma(&[1., 2., 3., 4.], 2), &[N, 1.5, 2.5, 3.5]);
        // na bar returns na and does not enter the window (TradingView G4)
        assert_series(&sma(&[1., N, 3., 5.], 2), &[N, N, 2., 4.]);
    }
    #[test] fn ema_seeds_with_sma_of_first_n_verified_on_tradingview() {
        // alpha = 2/3 for n=2; seed = (1+2)/2 = 1.5 at index 1; then 1.5 + 2/3*(3-1.5) = 2.5
        assert_series(&ema(&[1., 2., 3.], 2), &[N, 1.5, 2.5]);
    }
    #[test] fn rma_wilder() { assert_series(&rma(&[1., 2., 3., 4.], 2), &[N, 1.5, 2.25, 3.125]); }
    #[test] fn wma_linear_weights() { assert_series(&wma(&[1., 2., 3.], 3), &[N, N, (1. + 4. + 9.) / 6.]); }
    #[test] fn stdev_flat_window_is_exactly_zero() { assert!(stdev(&[1017.3; 20], 20)[19] == 0.); }
    #[test] fn stdev_is_population() { assert_series(&stdev(&[1., 3.], 2), &[N, 1.]); }
    #[test] fn highest_lowest_window() {
        assert_series(&highest(&[3., 1., 2., 5.], 2), &[N, 3., 2., 5.]);
        assert_series(&lowest(&[3., 1., 2., 5.], 2), &[N, 1., 1., 2.]);
    }
    #[test] fn tr_bar0_handle_na() {
        let b = from_ohlc(&[(10., 12., 9., 11.), (11., 13., 10.5, 12.)]);
        assert_series(&tr(&b, true), &[3., 2.5]);
        assert_series(&tr(&b, false), &[N, 2.5]);
    }
    #[test] fn linreg_fits_line_exactly() { assert_series(&linreg(&[1., 3., 5.], 3, 0), &[N, N, 5.]); }
    #[test] fn dev_mean_absolute() { assert_series(&dev(&[1., 3.], 2), &[N, 1.]); }
    #[test] fn highest_resets_after_nan_with_partial_window() {
        assert_series(&highest(&[3., N, 1., 2., 5.], 2), &[N, N, 1., 2., 5.]);
        assert_series(&lowest(&[3., N, 4., 2., 5.], 2), &[N, N, 4., 2., 2.]);
    }
    #[test] fn ema_skips_leading_and_interior_nan() {
        assert_series(&ema(&[N, 1., 2., 3.], 2), &[N, N, 1.5, 2.5]);
        assert_series(&ema(&[1., 2., N, 3.], 2), &[N, 1.5, N, 2.5]);
    }
    #[test] fn rma_skips_interior_nan() { assert_series(&rma(&[1., 2., N, 3.], 2), &[N, 1.5, N, 2.25]); }
    #[test] fn change_and_nz() {
        assert_series(&change(&[1., 4., 9.], 2), &[N, N, 8.]);
        assert_eq!(nz(N), 0.0);
        assert_eq!(nz(2.), 2.0);
    }
    #[test] fn linreg_non_collinear_and_offset() {
        assert_series(&linreg(&[1., 2., 4.], 3, 0), &[N, N, 23. / 6.]);
        assert_series(&linreg(&[1., 2., 4.], 3, 1), &[N, N, 7. / 3.]);
    }
    #[test] fn linreg_nan_in_window_is_bar_counted() {
        assert_series(&linreg(&[1., N, 3., 5., 7.], 2, 0), &[N, N, N, 5., 7.]);
    }
}
