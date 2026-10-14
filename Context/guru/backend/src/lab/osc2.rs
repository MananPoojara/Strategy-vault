//! Oscillators family II (TradingView / AmiBroker built-ins, Phase 6). Formulas: see each definition string.
use super::bars::Bars;
use super::catalogue::{int, len, n, p, IndicatorDef, Needs, Output, ParamSpec, Params, Placement};
use super::ta::{self, NA};

const CAT: &str = "Momentum";
fn div(a: f64, b: f64) -> f64 { if b == 0. || a.is_nan() || b.is_nan() { NA } else { a / b } }
#[allow(clippy::too_many_arguments)]
const fn def(id: &'static str, name: &'static str, params: &'static [ParamSpec], outputs: &'static [&'static str], placement: Placement,
             needs: Needs, definition: &'static str, first: &'static [usize], sparse: bool, f: fn(&Bars, &Params) -> Output) -> IndicatorDef {
    IndicatorDef { id, name, category: CAT, params, outputs, placement, needs, definition, first, sparse, compute: Some(f), note: "" }
}
const PN: Placement = Placement::Pane;
const NO: Needs = Needs::NONE;
const NONE: &[ParamSpec] = &[];
const L13: &[ParamSpec] = &[len("length", "Length", 13.)];
const AC: &[ParamSpec] = &[len("fast", "Fast", 5.), len("slow", "Slow", 34.), len("smooth", "Smoothing", 5.)];
const PO: &[ParamSpec] = &[len("fast", "Fast", 10.), len("slow", "Slow", 21.), int("exponential", "Exponential (0 SMA, 1 EMA)", 0., 0., 1.)];
const WC: &[ParamSpec] = &[len("length", "Length", 14.), len("turbo", "Turbo length", 6.)];
const RM: &[ParamSpec] = &[len("length", "Length", 20.), len("momentum", "Momentum", 5.)];
const L10: &[ParamSpec] = &[len("length", "Length", 10.)];

fn zip(a: &[f64], b: &[f64], f: impl Fn(f64, f64) -> f64) -> Vec<f64> { a.iter().zip(b).map(|(&x, &y)| f(x, y)).collect() }
pub(super) fn cci_s(x: &[f64], k: usize) -> Vec<f64> {
    let (m, d) = (ta::sma(x, k), ta::dev(x, k));
    (0..x.len()).map(|i| div(x[i] - m[i], 0.015 * d[i])).collect()
}
/// RSI-style ratio of two smoothed series: dn 0 -> 100, up 0 -> 0.
pub(super) fn rsi_like(up: &[f64], dn: &[f64], k: usize) -> Vec<f64> {
    zip(&ta::rma(up, k), &ta::rma(dn, k), |u, d| if u.is_nan() || d.is_nan() { NA } else if d == 0. { 100. } else if u == 0. { 0. } else { 100. - 100. / (1. + u / d) })
}
fn bop(b: &Bars, _: &Params) -> Output { Ok(vec![(0..b.len()).map(|i| div(b.close[i] - b.open[i], b.high[i] - b.low[i])).collect()]) }
fn bbp(b: &Bars, prm: &Params) -> Output {
    let e = ta::ema(&b.close, n(prm, "length"));
    let bull = zip(&b.high, &e, |h, e| h - e); let bear = zip(&b.low, &e, |l, e| l - e);
    Ok(vec![zip(&bull, &bear, |a, c| a + c), bull, bear])
}
fn accel_osc(b: &Bars, prm: &Params) -> Output {
    let h = b.hl2(); let ao = zip(&ta::sma(&h, n(prm, "fast")), &ta::sma(&h, n(prm, "slow")), |a, c| a - c);
    Ok(vec![zip(&ao, &ta::sma(&ao, n(prm, "smooth")), |a, c| a - c)])
}
fn price_osc(b: &Bars, prm: &Params) -> Output {
    let ma = |k| if p(prm, "exponential") == 1. { ta::ema(&b.close, k) } else { ta::sma(&b.close, k) };
    Ok(vec![zip(&ma(n(prm, "fast")), &ma(n(prm, "slow")), |f, s| div(100. * (f - s), s))])
}
fn woodies_cci(b: &Bars, prm: &Params) -> Output { Ok(vec![cci_s(&b.close, n(prm, "length")), cci_s(&b.close, n(prm, "turbo"))]) }
fn rmi(b: &Bars, prm: &Params) -> Output {
    let ch = ta::change(&b.close, n(prm, "momentum"));
    let up: Vec<f64> = ch.iter().map(|v| if v.is_nan() { NA } else { v.max(0.) }).collect();
    let dn: Vec<f64> = ch.iter().map(|v| if v.is_nan() { NA } else { (-v).max(0.) }).collect();
    Ok(vec![rsi_like(&up, &dn, n(prm, "length"))])
}
fn cg_osc(b: &Bars, prm: &Params) -> Output {
    let (h, k) = (b.hl2(), n(prm, "length"));
    let cg: Vec<f64> = (0..b.len()).map(|t| if t + 1 < k { NA } else {
        let (num, den) = (0..k).fold((0., 0.), |(a, d), i| (a + (i + 1) as f64 * h[t - i], d + h[t - i]));
        div(-num, den)
    }).collect();
    let trig = (0..cg.len()).map(|i| if i == 0 { NA } else { cg[i - 1] }).collect();
    Ok(vec![cg, trig])
}
pub const DEFS: &[IndicatorDef] = &[
    def("bop", "Balance of Power", NONE, &["bop"], PN, NO, "(close − open)/(high − low); zero range → NaN", &[0], false, bop),
    def("bbp", "Bull Bear Power", L13, &["bbp", "bull", "bear"], PN, NO, "bull = high − ema(close, n); bear = low − ema(close, n); bbp = bull + bear (the TradingView plot)", &[12, 12, 12], false, bbp),
    def("accel_osc", "Accelerator Oscillator", AC, &["ac"], PN, NO, "ao = sma(hl2, fast) − sma(hl2, slow); ac = ao − sma(ao, smooth)", &[37], false, accel_osc),
    def("price_osc", "Price Oscillator", PO, &["po"], PN, NO, "100·(ma(close, fast) − ma(close, slow))/ma(close, slow), ma = sma (ema if exponential = 1; AmiBroker OscP = exponential 1)", &[20], false, price_osc),
    def("woodies_cci", "Woodies CCI", WC, &["cci", "turbo"], PN, NO, "cci of close (not hlc3) over length and over turbo: (close − sma)/(0.015·mean absolute deviation); zero deviation → NaN", &[13, 5], false, woodies_cci),
    def("rmi", "Relative Momentum Index", RM, &["rmi"], PN, NO, "up = max(close − close[m], 0), dn = max(close[m] − close, 0); RSI formula on rma(up, n), rma(dn, n) (dn 0 → 100, up 0 → 0) (AmiBroker RMI; rma seeded as TradingView)", &[24], false, rmi),
    def("cg_osc", "Center of Gravity (Ehlers)", L10, &["cg", "trigger"], PN, NO, "cg = −Σ_{i=0}^{n−1}(i+1)·hl2[t−i] / Σ hl2[t−i]; trigger = cg[1] (declared Ehlers form for Bandy's CGOscillator)", &[9, 10], false, cg_osc),
];

#[cfg(test)]
mod tests {
    use crate::lab::testdata::{assert_series, compiles, d1, go};
    const N: f64 = f64::NAN;
    #[test] fn bop_and_bull_bear_power() {
        assert_series(&go("bop", &d1(), &[])[0], &[0.25, 0.5, 0.333333333333, -0.714285714286, -0.533333333333, 0.4, 0.733333333333, 0.466666666667]);
        let o = go("bbp", &d1(), &[("length", 3.)]);
        assert_series(&o[0], &[N, N, 0.833333333333, 0.666666666667, 0.133333333333, -0.983333333333, -0.141666666667, 0.579166666667]);
        assert_series(&o[1], &[N, N, 1.166666666667, 1.033333333333, 0.816666666667, 0.258333333333, 0.679166666667, 1.039583333333]);
        assert_series(&o[2], &[N, N, -0.333333333333, -0.366666666667, -0.683333333333, -1.241666666667, -0.820833333333, -0.460416666667]);
    }
    #[test] fn accelerator_and_price_oscillator() {
        assert_series(&go("accel_osc", &d1(), &[("fast", 2.), ("slow", 3.), ("smooth", 2.)])[0], &[N, N, N, -0.125, -0.208333333333, -0.0625, 0.166666666667, 0.25]);
        assert_series(&go("price_osc", &d1(), &[("fast", 2.), ("slow", 3.), ("exponential", 0.)])[0], &[N, N, 3.676470588235, 0.0, -4.21686746988, -1.5625, 3.495440729483, 4.107648725212]);
    }
    #[test] fn woodies_cci_uses_close() {
        let o = go("woodies_cci", &d1(), &[("length", 3.), ("turbo", 2.)]);
        assert_series(&o[0], &[N, N, 80.0, -100.0, -92.857142857143, 28.571428571429, 100.0, 86.206896551724]);
        assert_series(&o[1], &[N, 66.666666666667, 66.666666666667, -66.666666666667, -66.666666666667, 66.666666666667, 66.666666666667, 66.666666666667]);
    }
    #[test] fn rmi_and_center_of_gravity() {
        assert_series(&go("rmi", &d1(), &[("length", 2.), ("momentum", 2.)])[0], &[N, N, N, 75.0, 26.785714285714, 23.4375, 75.5, 89.959016393443]);
        let o = go("cg_osc", &d1(), &[("length", 3.)]);
        assert_series(&o[0], &[N, N, -1.946564885496, -1.985401459854, -2.029411764706, -2.038461538462, -1.984496124031, -1.940740740741]);
        assert_series(&o[1], &[N, N, N, -1.946564885496, -1.985401459854, -2.029411764706, -2.038461538462, -1.984496124031]);
    }
    #[test] fn rules_reach_osc1() { for (id, out) in [("bbp", "bear"), ("woodies_cci", "turbo"), ("cg_osc", "trigger")] { compiles(id, out).unwrap(); } }
}
