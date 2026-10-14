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
const PM: &[ParamSpec] = &[len("length1", "Length 1", 35.), len("length2", "Length 2", 20.), len("signal", "Signal", 10.)];
const SKP: &[ParamSpec] = &[len("signal1", "Signal 1", 100.), len("signal2", "Signal 2", 100.)];
const SE: &[ParamSpec] = &[len("long", "Long", 20.), len("short", "Short", 5.), len("signal", "Signal", 5.)];
const SM: &[ParamSpec] = &[len("k", "%K length", 10.), len("d", "%D length", 3.), len("ema", "EMA length", 3.)];
const RV: &[ParamSpec] = &[len("length", "Length", 10.), len("ema", "EMA length", 14.)];

fn roc_s(x: &[f64], k: usize) -> Vec<f64> { (0..x.len()).map(|i| if i >= k { div(100. * (x[i] - x[i - k]), x[i - k]) } else { NA }).collect() }
/// DecisionPoint custom smoothing: SMA seed, then alpha = 2/L (not 2/(L+1)).
fn csf(x: &[f64], k: usize) -> Vec<f64> {
    let seed = ta::sma(x, k); let mut prev = NA;
    x.iter().enumerate().map(|(i, &v)| { if v.is_nan() { return NA; } prev = if prev.is_nan() { seed[i] } else { prev + (v - prev) * 2. / k as f64 }; prev }).collect()
}
fn pmo(b: &Bars, prm: &Params) -> Output {
    let r1: Vec<f64> = csf(&roc_s(&b.close, 1), n(prm, "length1")).iter().map(|v| 10. * v).collect();
    let pm = csf(&r1, n(prm, "length2"));
    let sig = ta::ema(&pm, n(prm, "signal"));
    Ok(vec![pm, sig])
}
const SK: [(usize, usize, f64); 12] = [(10, 10, 1.), (15, 10, 2.), (20, 10, 3.), (30, 15, 4.), (40, 50, 1.), (65, 65, 2.), (75, 75, 3.), (100, 100, 4.), (195, 130, 1.), (265, 130, 2.), (390, 130, 3.), (530, 195, 4.)];
fn special_k(b: &Bars, prm: &Params) -> Output {
    let mut sk = vec![0.; b.len()];
    for (r, m, w) in SK { let s = ta::sma(&roc_s(&b.close, r), m); for i in 0..b.len() { sk[i] += w * s[i]; } }
    let sig = ta::sma(&ta::sma(&sk, n(prm, "signal1")), n(prm, "signal2"));
    Ok(vec![sk, sig])
}
fn smi_ergodic(b: &Bars, prm: &Params) -> Output {
    let (lg, sh) = (n(prm, "long"), n(prm, "short"));
    let d = ta::change(&b.close, 1); let ad: Vec<f64> = d.iter().map(|v| v.abs()).collect();
    let erg = zip(&ta::ema(&ta::ema(&d, lg), sh), &ta::ema(&ta::ema(&ad, lg), sh), div);
    let sig = ta::ema(&erg, n(prm, "signal"));
    let osc = zip(&erg, &sig, |a, c| a - c);
    Ok(vec![erg, sig, osc])
}
fn smi(b: &Bars, prm: &Params) -> Output {
    let (k, d) = (n(prm, "k"), n(prm, "d"));
    let (hh, ll) = (ta::highest(&b.high, k), ta::lowest(&b.low, k));
    let rel: Vec<f64> = (0..b.len()).map(|i| b.close[i] - (hh[i] + ll[i]) / 2.).collect();
    let rng: Vec<f64> = (0..b.len()).map(|i| hh[i] - ll[i]).collect();
    let ee = |x: &[f64]| ta::ema(&ta::ema(x, d), d);
    let s = zip(&ee(&rel), &ee(&rng), |a, c| div(200. * a, c));
    let sig = ta::ema(&s, n(prm, "ema"));
    Ok(vec![s, sig])
}
fn swma(x: &[f64]) -> Vec<f64> { (0..x.len()).map(|i| if i < 3 { NA } else { (x[i - 3] + 2. * x[i - 2] + 2. * x[i - 1] + x[i]) / 6. }).collect() }
fn rvgi(b: &Bars, prm: &Params) -> Output {
    let k = n(prm, "length");
    let num = swma(&(0..b.len()).map(|i| b.close[i] - b.open[i]).collect::<Vec<_>>());
    let den = swma(&(0..b.len()).map(|i| b.high[i] - b.low[i]).collect::<Vec<_>>());
    let r = zip(&ta::sum(&num, k), &ta::sum(&den, k), div);
    let sig = swma(&r);
    Ok(vec![r, sig])
}
fn rvi(b: &Bars, prm: &Params) -> Output {
    let (sd, ch, e) = (ta::stdev(&b.close, n(prm, "length")), ta::change(&b.close, 1), n(prm, "ema"));
    let u: Vec<f64> = (0..b.len()).map(|i| if ch[i] <= 0. { 0. } else { sd[i] }).collect(); // NaN change fails `<= 0` -> sd, as in Pine
    let d: Vec<f64> = (0..b.len()).map(|i| if ch[i] > 0. { 0. } else { sd[i] }).collect();
    Ok(vec![zip(&ta::ema(&u, e), &ta::ema(&d, e), |a, c| div(100. * a, a + c))])
}
pub const DEFS: &[IndicatorDef] = &[
    def("bop", "Balance of Power", NONE, &["bop"], PN, NO, "(close − open)/(high − low); zero range → NaN", &[0], false, bop),
    def("bbp", "Bull Bear Power", L13, &["bbp", "bull", "bear"], PN, NO, "bull = high − ema(close, n); bear = low − ema(close, n); bbp = bull + bear (the TradingView plot)", &[12, 12, 12], false, bbp),
    def("accel_osc", "Accelerator Oscillator", AC, &["ac"], PN, NO, "ao = sma(hl2, fast) − sma(hl2, slow); ac = ao − sma(ao, smooth)", &[37], false, accel_osc),
    def("price_osc", "Price Oscillator", PO, &["po"], PN, NO, "100·(ma(close, fast) − ma(close, slow))/ma(close, slow), ma = sma (ema if exponential = 1; AmiBroker OscP = exponential 1)", &[20], false, price_osc),
    def("woodies_cci", "Woodies CCI", WC, &["cci", "turbo"], PN, NO, "cci of close (not hlc3) over length and over turbo: (close − sma)/(0.015·mean absolute deviation); zero deviation → NaN", &[13, 5], false, woodies_cci),
    def("rmi", "Relative Momentum Index", RM, &["rmi"], PN, NO, "up = max(close − close[m], 0), dn = max(close[m] − close, 0); RSI formula on rma(up, n), rma(dn, n) (dn 0 → 100, up 0 → 0) (AmiBroker RMI; rma seeded as TradingView)", &[24], false, rmi),
    def("cg_osc", "Center of Gravity (Ehlers)", L10, &["cg", "trigger"], PN, NO, "cg = −Σ_{i=0}^{n−1}(i+1)·hl2[t−i] / Σ hl2[t−i]; trigger = cg[1] (declared Ehlers form for Bandy's CGOscillator)", &[9, 10], false, cg_osc),
    def("pmo", "Price Momentum Oscillator", PM, &["pmo", "signal"], PN, NO, "roc1 = 100·(close/close[1] − 1); csf(x, L): seeded with sma(x, L) at its first value, then s += (x − s)·2/L; pmo = csf(10·csf(roc1, length1), length2); signal = ema(pmo, signal) (seed declared)", &[54, 63], false, pmo),
    def("special_k", "Pring's Special K", SKP, &["sk", "signal"], PN, NO, "Σ w·sma(roc(close, r), m) over (r, m, w) = (10,10,1), (15,10,2), (20,10,3), (30,15,4), (40,50,1), (65,65,2), (75,75,3), (100,100,4), (195,130,1), (265,130,2), (390,130,3), (530,195,4); signal = sma(sma(sk, signal1), signal2)", &[724, 922], false, special_k),
    def("smi_ergodic", "SMI Ergodic", SE, &["smi", "signal", "osc"], PN, NO, "smi = ema(ema(Δclose, long), short)/ema(ema(|Δclose|, long), short) (ta.tsi ratio, not ×100); signal = ema(smi, signal); osc = smi − signal (Indicator + Oscillator)", &[24, 28, 28], false, smi_ergodic),
    def("smi", "Stochastic Momentum Index", SM, &["smi", "signal"], PN, NO, "hh/ll = highest(high, k)/lowest(low, k); smi = 200·ema(ema(close − (hh + ll)/2, d), d)/ema(ema(hh − ll, d), d); signal = ema(smi, ema)", &[13, 15], false, smi),
    def("rvgi", "Relative Vigor Index", L10, &["rvi", "signal"], PN, NO, "rvi = sum(swma(close − open), n)/sum(swma(high − low), n); signal = swma(rvi)", &[12, 15], false, rvgi),
    def("rvi", "Relative Volatility Index", RV, &["rvi"], PN, NO, "sd = stdev(close, length); u = change(close) ≤ 0 ? 0 : sd; d = change(close) > 0 ? 0 : sd (bar 0: neither → sd); rvi = 100·ema(u, ema)/(ema(u, ema) + ema(d, ema)); warm-up depends on the data (zeros before stdev exists count, as on TradingView)", &[usize::MAX], true, rvi),
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
    use crate::lab::testdata::closes;
    #[test] fn pmo_short() {
        let o = go("pmo", &d1(), &[("length1", 2.), ("length2", 2.), ("signal", 2.)]);
        assert_series(&o[0], &[N, N, N, -6.987577639752, -72.727272727273, 58.823529411765, 101.851851851852, 58.823529411765]);
        assert_series(&o[1], &[N, N, N, N, -39.857425183512, 25.929877880006, 76.54452719457, 64.7305286727]);
    }
    #[test] fn special_k_on_constant_growth() {
        // close_i = 100·1.01^i → every ROC(r) is 100·(1.01^r − 1): sk = Σ w·100·(1.01^r − 1) = 96572.2974214603 from bar 724
        let o = go("special_k", &closes(&(0..1000).map(|i| 100. * 1.01f64.powi(i)).collect::<Vec<_>>()), &[]);
        assert!(o[0][723].is_nan() && o[1][921].is_nan());
        assert!((o[0][724] - 96572.2974214603).abs() < 1e-6 && (o[1][922] - 96572.2974214603).abs() < 1e-6);
    }
    #[test] fn smi_ergodic_and_smi() {
        let o = go("smi_ergodic", &d1(), &[("long", 3.), ("short", 2.), ("signal", 2.)]);
        assert_series(&o[0], &[N, N, N, N, -0.090909090909, 0.092936802974, 0.513043478261, 0.715739868049]);
        assert_series(&o[2], &[N, N, N, N, N, 0.091922946942, 0.170676540743, 0.12445764351]);
        let s = go("smi", &d1(), &[("k", 3.), ("d", 2.), ("ema", 2.)]);
        assert_series(&s[0], &[N, N, N, N, -1.176470588235, -10.690121786198, 31.700423928403, 59.571840263483]);
        assert_series(&s[1], &[N, N, N, N, N, -5.933296187216, 19.15585055653, 46.099843694499]);
    }
    #[test] fn relative_vigor_and_volatility() {
        let v = go("rvgi", &d1(), &[("length", 2.)]);
        assert_series(&v[0], &[N, N, N, N, 0.086294416244, -0.182320441989, -0.158192090395, 0.167597765363]);
        assert_series(&v[1], &[N, N, N, N, N, N, N, -0.07118881386]);
        assert_series(&go("rvi", &d1(), &[("length", 2.), ("ema", 2.)])[0], &[N, N, 100.0, 27.272727272727, 9.933774834437, 62.942779291553, 91.254019292604, 96.442584357834]);
    }
    #[test] fn rules_reach_osc2() { for (id, out) in [("pmo", "signal"), ("smi_ergodic", "osc"), ("rvi", "rvi")] { compiles(id, out).unwrap(); } }
}
