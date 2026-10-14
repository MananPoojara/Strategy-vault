//! Moving averages II family (TradingView / AmiBroker built-ins, Phase 6). Formulas: see each definition string.
use super::bars::Bars;
use super::catalogue::{int, len, n, p, real, IndicatorDef, Needs, Output, ParamSpec, Params, Placement};
use super::ta::{self, NA};

const CAT: &str = "Moving averages";
fn div(a: f64, b: f64) -> f64 { if b == 0. || a.is_nan() || b.is_nan() { NA } else { a / b } }
#[allow(clippy::too_many_arguments)]
const fn def(id: &'static str, name: &'static str, params: &'static [ParamSpec], outputs: &'static [&'static str], placement: Placement,
             needs: Needs, definition: &'static str, first: &'static [usize], sparse: bool, f: fn(&Bars, &Params) -> Output) -> IndicatorDef {
    IndicatorDef { id, name, category: CAT, params, outputs, placement, needs, definition, first, sparse, compute: Some(f), note: "" }
}
const P: Placement = Placement::Price;
const NO: Needs = Needs::NONE;
const RIBBON: &[ParamSpec] = &[int("type", "Type (0 SMA, 1 EMA, 2 SMMA, 3 WMA, 4 VWMA)", 0., 0., 4.), len("len1", "Length 1", 20.), len("len2", "Length 2", 50.), len("len3", "Length 3", 100.), len("len4", "Length 4", 200.)];
const CROSS: &[ParamSpec] = &[len("fast", "Fast length", 9.), len("slow", "Slow length", 21.), int("fast_type", "Fast type (0 SMA, 1 EMA, 2 SMMA, 3 WMA)", 0., 0., 3.), int("slow_type", "Slow type (0 SMA, 1 EMA, 2 SMMA, 3 WMA)", 0., 0., 3.)];
const HAM: &[ParamSpec] = &[ParamSpec { min: 2., ..len("length", "Length", 10.) }];
const AMA1: &[ParamSpec] = &[real("alpha", "Alpha", 0.1, 0.001, 1., 0.001)];
const AMA2: &[ParamSpec] = &[real("alpha", "Alpha", 0.1, 0.001, 1., 0.001), real("feedback", "Feedback", 0.9, 0., 1., 0.001)];
const IIR: &[ParamSpec] = &[real("f0", "f0", 0.2, -2., 2., 0.01), real("f1", "f1", 1.4, -2., 2., 0.01), real("f2", "f2", -0.6, -2., 2., 0.01)];

pub(super) fn ma_of(b: &Bars, x: &[f64], n: usize, t: usize) -> Result<Vec<f64>, String> {
    Ok(match t {
        0 => ta::sma(x, n), 1 => ta::ema(x, n), 2 => ta::rma(x, n), 3 => ta::wma(x, n),
        _ => {
            let v = b.volume.as_ref().ok_or("needs volume")?;
            let xv: Vec<f64> = x.iter().zip(v).map(|(a, w)| a * w).collect();
            ta::sma(&xv, n).iter().zip(ta::sma(v, n)).map(|(a, d)| div(*a, d)).collect()
        }
    })
}
pub(super) fn ama_s(x: &[f64], alpha: f64, feedback: f64) -> Vec<f64> {
    let mut prev = NA;
    x.iter().map(|&v| { prev = if prev.is_nan() { v } else { alpha * v + feedback * prev }; prev }).collect()
}
fn ma_ribbon(b: &Bars, prm: &Params) -> Output {
    let t = n(prm, "type");
    ["len1", "len2", "len3", "len4"].iter().map(|k| ma_of(b, &b.close, n(prm, k), t)).collect()
}
fn ma_cross(b: &Bars, prm: &Params) -> Output {
    let f = ma_of(b, &b.close, n(prm, "fast"), n(prm, "fast_type"))?;
    let s = ma_of(b, &b.close, n(prm, "slow"), n(prm, "slow_type"))?;
    let cross = (0..b.len()).map(|i| {
        if i == 0 || [f[i], s[i], f[i - 1], s[i - 1]].iter().any(|v| v.is_nan()) { NA }
        else if f[i] > s[i] && f[i - 1] <= s[i - 1] { 1. } else if f[i] < s[i] && f[i - 1] >= s[i - 1] { -1. } else { 0. }
    }).collect();
    Ok(vec![f, s, cross])
}
fn gmma(b: &Bars, _: &Params) -> Output { Ok([3, 5, 8, 10, 12, 15, 30, 35, 40, 45, 50, 60].iter().map(|&k| ta::ema(&b.close, k)).collect()) }
fn hamming_ma(b: &Bars, prm: &Params) -> Output {
    let k = n(prm, "length");
    let w: Vec<f64> = (0..k).map(|j| 0.54 - 0.46 * (2. * std::f64::consts::PI * j as f64 / (k as f64 - 1.)).cos()).collect();
    let ws: f64 = w.iter().sum();
    Ok(vec![(0..b.len()).map(|i| if i + 1 < k { NA } else { b.close[i + 1 - k..=i].iter().zip(&w).map(|(x, w)| x * w).sum::<f64>() / ws }).collect()])
}
fn ama(b: &Bars, prm: &Params) -> Output { let a = p(prm, "alpha"); Ok(vec![ama_s(&b.close, a, 1. - a)]) }
fn ama2(b: &Bars, prm: &Params) -> Output { Ok(vec![ama_s(&b.close, p(prm, "alpha"), p(prm, "feedback"))]) }
fn iir2(b: &Bars, prm: &Params) -> Output {
    let (f0, f1, f2) = (p(prm, "f0"), p(prm, "f1"), p(prm, "f2"));
    let mut out: Vec<f64> = Vec::with_capacity(b.len());
    for i in 0..b.len() {
        let v = if i < 2 { b.close[i] } else { f0 * b.close[i] + f1 * out[i - 1] + f2 * out[i - 2] };
        out.push(if v.is_finite() { v } else { NA });
    }
    Ok(vec![out])
}

pub const DEFS: &[IndicatorDef] = &[
    def("ma_ribbon", "Moving Average Ribbon", RIBBON, &["ma1", "ma2", "ma3", "ma4"], P, NO, "ma_k = MA(close, len_k) of type 0 SMA, 1 EMA, 2 SMMA (RMA), 3 WMA, 4 VWMA (type 4 needs volume)", &[19, 49, 99, 199], false, ma_ribbon),
    def("ma_cross", "MA Cross", CROSS, &["fast", "slow", "cross"], P, NO, "fast/slow = MA(close, len) of type 0 SMA, 1 EMA, 2 SMMA, 3 WMA; cross = +1 when fast > slow and fast[1] <= slow[1], −1 when fast < slow and fast[1] >= slow[1], else 0; NaN until both MAs exist on this and the previous bar", &[8, 20, 21], false, ma_cross),
    def("gmma", "Guppy Multiple Moving Average", &[], &["e3", "e5", "e8", "e10", "e12", "e15", "e30", "e35", "e40", "e45", "e50", "e60"], P, NO, "ema(close, n) for n = 3, 5, 8, 10, 12, 15 (short group) and 30, 35, 40, 45, 50, 60 (long group)", &[2, 4, 7, 9, 11, 14, 29, 34, 39, 44, 49, 59], false, gmma),
    def("hamming_ma", "Moving Average Hamming", HAM, &["hamming"], P, NO, "Σ w_k·close[n−1−k] / Σ w_k, w_k = 0.54 − 0.46·cos(2πk/(n−1)), k = 0 oldest (declared Hamming window; TradingView formula unpublished)", &[9], false, hamming_ma),
    def("ama", "Adaptive Moving Average (AmiBroker AMA)", AMA1, &["ama"], P, NO, "AmiBroker AMA(close, alpha): out = alpha·close + (1 − alpha)·out[1]; out[0] = close[0]", &[0], false, ama),
    def("ama2", "Adaptive Moving Average 2 (AmiBroker AMA2)", AMA2, &["ama2"], P, NO, "AmiBroker AMA2: out = alpha·close + feedback·out[1]; out[0] = close[0]", &[0], false, ama2),
    def("iir2", "Second-Order IIR Filter (Bandy)", IIR, &["iir2"], P, NO, "Bandy QTS Fig 8.1: out[0] = close[0], out[1] = close[1], out[i] = f0·close + f1·out[i−1] + f2·out[i−2]; once a value is not finite it and every later value are NaN", &[0], false, iir2),
];

#[cfg(test)]
mod tests {
    use crate::lab::catalogue::{get, run, Params};
    use crate::lab::testdata::{assert_series, closes, compiles, d1, go};
    const N: f64 = f64::NAN;
    #[test] fn ma_ribbon_ema_type_and_vwma_needs_volume() {
        let o = go("ma_ribbon", &d1(), &[("type", 1.), ("len1", 2.), ("len2", 3.), ("len3", 4.), ("len4", 5.)]);
        assert_series(&o[0], &[N, 11.0, 11.666666666667, 11.222222222222, 10.540740740741, 10.713580246914, 11.504526748971, 12.234842249657]);
        assert_series(&o[1], &[N, N, 11.333333333333, 11.166666666667, 10.683333333333, 10.741666666667, 11.320833333333, 11.960416666667]);
        assert_series(&o[3], &[N, N, N, N, 11.04, 10.96, 11.273333333333, 11.715555555556]);
        let p: Params = [("type".to_string(), 4.)].into_iter().collect();
        assert_eq!(run(&get("ma_ribbon").unwrap(), &closes(&[1., 2., 3.]), &p).unwrap_err(), "needs volume");
    }
    #[test] fn ma_cross_sma_over_ema() {
        let o = go("ma_cross", &d1(), &[("fast", 2.), ("slow", 3.), ("fast_type", 0.), ("slow_type", 1.)]);
        assert_series(&o[0], &[N, 11.0, 11.75, 11.5, 10.6, 10.5, 11.35, 12.25]);
        assert_series(&o[1], &[N, N, 11.333333333333, 11.166666666667, 10.683333333333, 10.741666666667, 11.320833333333, 11.960416666667]);
        assert_series(&o[2], &[N, N, N, 0., -1., 0., 1., 0.]);
    }
    #[test] fn gmma_short_lines() {
        let o = go("gmma", &d1(), &[]);
        assert_eq!(o.len(), 12);
        assert_series(&o[0], &[N, N, 11.333333333333, 11.166666666667, 10.683333333333, 10.741666666667, 11.320833333333, 11.960416666667]);
        assert_series(&o[1], &[N, N, N, N, 11.04, 10.96, 11.273333333333, 11.715555555556]);
    }
    #[test] fn hamming_len3() {
        assert_series(&go("hamming_ma", &d1(), &[("length", 3.)])[0], &[N, N, 11.465517241379, 11.896551724138, 11.013793103448, 10.296551724138, 10.834482758621, 11.872413793103]);
    }
    #[test] fn ama_and_ama2_seed_with_the_first_close() {
        assert_series(&go("ama", &d1(), &[("alpha", 0.5)])[0], &[10.5, 11.0, 11.5, 11.25, 10.725, 10.7625, 11.33125, 11.965625]);
        assert_series(&go("ama2", &d1(), &[("alpha", 0.5), ("feedback", 0.25)])[0], &[10.5, 8.375, 8.09375, 7.5234375, 6.980859375, 7.14521484375, 7.736303710938, 8.234075927734]);
    }
    #[test] fn iir2_bandy_defaults() {
        assert_series(&go("iir2", &d1(), &[])[0], &[10.5, 11.5, 12.2, 12.38, 12.052, 11.6048, 11.39552, 11.510848]);
    }
    #[test] fn iir2_unstable_coefficients_stay_finite_or_nan() {
        let o = go("iir2", &closes(&[1.; 3000]), &[("f0", 2.), ("f1", 2.), ("f2", 2.)]);
        assert!(o[0].iter().all(|v| !v.is_infinite()));
        assert!(o[0][2999].is_nan(), "a diverging filter ends as NaN, not inf");
    }
    #[test] fn rules_reach_ma2() { for (id, out) in [("ma_cross", "cross"), ("gmma", "e60"), ("ma_ribbon", "ma4")] { compiles(id, out).unwrap(); } }
}
