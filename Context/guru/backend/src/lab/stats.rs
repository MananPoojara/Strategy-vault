//! Statistics family (TradingView / AmiBroker built-ins, Phase 6). Formulas: see each definition string.
use super::bars::Bars;
use super::catalogue::{int, len, n, p, real, IndicatorDef, Needs, Output, ParamSpec, Params, Placement};
use super::ta::{self, NA};

const CAT: &str = "Statistics";
fn div(a: f64, b: f64) -> f64 { if b == 0. || a.is_nan() || b.is_nan() { NA } else { a / b } }
#[allow(clippy::too_many_arguments)]
const fn def(id: &'static str, name: &'static str, params: &'static [ParamSpec], outputs: &'static [&'static str], placement: Placement,
             needs: Needs, definition: &'static str, first: &'static [usize], sparse: bool, f: fn(&Bars, &Params) -> Output) -> IndicatorDef {
    IndicatorDef { id, name, category: CAT, params, outputs, placement, needs, definition, first, sparse, compute: Some(f), note: "" }
}
const P: Placement = Placement::Price;
const PN: Placement = Placement::Pane;
const NO: Needs = Needs::NONE;
const L14: &[ParamSpec] = &[int("length", "Length", 14., 2., 2000.)];
const L14S: &[ParamSpec] = &[int("length", "Length", 14., 3., 2000.)];
const SEB: &[ParamSpec] = &[int("length", "Length", 21., 3., 2000.), real("mult", "Multiplier", 2., 0.1, 10., 0.1), len("smooth", "Smoothing", 3.)];
const RCI: &[ParamSpec] = &[int("length", "Length", 10., 2., 2000.)];
const PR: &[ParamSpec] = &[len("length", "Length", 20.), int("variant", "Variant (0 TradingView, 1 AmiBroker)", 0., 0., 1.)];
const PCT: &[ParamSpec] = &[len("length", "Length", 20.), real("pct", "Percentile", 50., 0., 100., 1.), int("variant", "Variant (0 interpolated, 1 nearest rank)", 0., 0., 1.)];
const L20: &[ParamSpec] = &[len("length", "Length", 20.)];
const M20: &[ParamSpec] = &[int("length", "Length", 20., 2., 2000.)];
const RIB: &[ParamSpec] = &[int("short", "Short", 10., 2., 2000.), int("middle", "Middle", 30., 2., 2000.), int("long", "Long", 50., 2., 2000.)];

/// Least squares over w with k = 1..n (oldest → newest): y = alpha + slope·k.
fn fit(w: &[f64]) -> (f64, f64) {
    let nf = w.len() as f64;
    let (mut sk, mut sy, mut skk, mut sky) = (0., 0., 0., 0.);
    for (j, &y) in w.iter().enumerate() { let k = (j + 1) as f64; sk += k; sy += y; skk += k * k; sky += k * y; }
    let den = nf * skk - sk * sk;
    let slope = if den == 0. { 0. } else { (nf * sky - sk * sy) / den };
    (sy / nf - slope * sk / nf, slope)
}
/// f(window) for every bar with a full window of `n` bars; any NaN in the window → NaN (bar-counted, like ta::linreg).
fn rolling(x: &[f64], n: usize, f: impl Fn(&[f64]) -> f64) -> Vec<f64> {
    (0..x.len()).map(|i| if i + 1 < n { NA } else { let w = &x[i + 1 - n..=i]; if w.iter().any(|v| v.is_nan()) { NA } else { f(w) } }).collect()
}
fn stderr_w(w: &[f64]) -> f64 {
    let (a, s) = fit(w);
    (w.iter().enumerate().map(|(j, y)| (y - (a + s * (j + 1) as f64)).powi(2)).sum::<f64>() / (w.len() as f64 - 2.)).sqrt()
}
fn linreg_slope(b: &Bars, prm: &Params) -> Output { Ok(vec![rolling(&b.close, n(prm, "length"), |w| fit(w).1)]) }
fn linreg_intercept(b: &Bars, prm: &Params) -> Output { Ok(vec![rolling(&b.close, n(prm, "length"), |w| fit(w).0)]) }
fn tsf(b: &Bars, prm: &Params) -> Output {
    let h = n(prm, "horizon");
    Ok(vec![rolling(&b.close, n(prm, "length"), |w| { let (a, s) = fit(w); a + s * (w.len() + h) as f64 })])
}
fn stderr(b: &Bars, prm: &Params) -> Output { Ok(vec![rolling(&b.close, n(prm, "length"), stderr_w)]) }
fn se_bands(b: &Bars, prm: &Params) -> Output {
    let (k, m, s) = (n(prm, "length"), p(prm, "mult"), n(prm, "smooth"));
    let (lr, se) = (ta::linreg(&b.close, k, 0), rolling(&b.close, k, stderr_w));
    let up: Vec<f64> = (0..b.len()).map(|i| lr[i] + m * se[i]).collect();
    let dn: Vec<f64> = (0..b.len()).map(|i| lr[i] - m * se[i]).collect();
    Ok(vec![ta::sma(&lr, s), ta::sma(&up, s), ta::sma(&dn, s)])
}
fn corr_w(w: &[f64]) -> f64 {
    let nf = w.len() as f64; let (mk, my) = ((nf + 1.) / 2., w.iter().sum::<f64>() / nf);
    let (mut num, mut vk, mut vy) = (0., 0., 0.);
    for (j, y) in w.iter().enumerate() { let dk = (j + 1) as f64 - mk; let dy = snap(y - my); num += dk * dy; vk += dk * dk; vy += dy * dy; }
    div(num, (vk * vy).sqrt())
}
fn trend_strength(b: &Bars, prm: &Params) -> Output { Ok(vec![rolling(&b.close, n(prm, "length"), corr_w)]) }
/// 1-based ranks, 1 = highest value, ties share their average rank.
fn ranks_desc(w: &[f64]) -> Vec<f64> {
    let mut idx: Vec<usize> = (0..w.len()).collect();
    idx.sort_by(|&a, &b| w[b].total_cmp(&w[a]));
    let mut r = vec![0.; w.len()];
    let mut j = 0;
    while j < idx.len() {
        let mut k = j;
        while k + 1 < idx.len() && w[idx[k + 1]] == w[idx[j]] { k += 1; }
        for t in j..=k { r[idx[t]] = (j + k) as f64 / 2. + 1.; }
        j = k + 1;
    }
    r
}
fn rci_s(x: &[f64], n: usize) -> Vec<f64> {
    let nf = n as f64;
    rolling(x, n, |w| { let pr = ranks_desc(w); let d2: f64 = pr.iter().enumerate().map(|(j, p)| ((n - j) as f64 - p).powi(2)).sum(); 100. * (1. - 6. * d2 / (nf * (nf * nf - 1.))) })
}
fn rci(b: &Bars, prm: &Params) -> Output { Ok(vec![rci_s(&b.close, n(prm, "length"))]) }
fn rci_ribbon(b: &Bars, prm: &Params) -> Output { Ok(["short", "middle", "long"].iter().map(|k| rci_s(&b.close, n(prm, k))).collect()) }

fn percentrank(b: &Bars, prm: &Params) -> Output {
    let (k, strict, x) = (n(prm, "length"), p(prm, "variant") == 1., &b.close);
    Ok(vec![(0..x.len()).map(|i| if i < k || x[i].is_nan() { NA } else {
        100. * x[i - k..i].iter().filter(|&&v| if strict { v < x[i] } else { v <= x[i] }).count() as f64 / k as f64
    }).collect()])
}
fn percentile(b: &Bars, prm: &Params) -> Output {
    let (k, pct, nearest) = (n(prm, "length"), p(prm, "pct"), p(prm, "variant") == 1.);
    Ok(vec![rolling(&b.close, k, |w| {
        let mut s = w.to_vec(); s.sort_by(|a, b| a.total_cmp(b));
        if nearest { s[((pct / 100. * k as f64).ceil() as usize).clamp(1, k) - 1] }
        else { let pos = pct / 100. * (k as f64 - 1.); let lo = pos.floor() as usize; let hi = (lo + 1).min(k - 1); s[lo] + (s[hi] - s[lo]) * (pos - lo as f64) }
    })])
}
fn zscore(b: &Bars, prm: &Params) -> Output {
    let k = n(prm, "length"); let (m, s) = (ta::sma(&b.close, k), ta::stdev(&b.close, k));
    Ok(vec![(0..b.len()).map(|i| div(b.close[i] - m[i], s[i])).collect()])
}
/// A flat window's mean is not exact in floating point; snap tiny deviations to 0 like `ta::stdev` so it reads as flat.
fn snap(d: f64) -> f64 { if d.abs() <= 1e-10 { 0. } else { d } }
fn moment(x: &[f64], k: usize, pow: i32) -> Vec<f64> {
    rolling(x, k, |w| {
        let nf = w.len() as f64; let m = w.iter().sum::<f64>() / nf;
        let sd = (w.iter().map(|v| snap(v - m).powi(2)).sum::<f64>() / nf).sqrt();
        if sd == 0. { return NA; }
        let r = w.iter().map(|v| snap(v - m).powi(pow)).sum::<f64>() / nf / sd.powi(pow);
        if pow == 4 { r - 3. } else { r }
    })
}
fn skewness(b: &Bars, prm: &Params) -> Output { Ok(vec![moment(&b.close, n(prm, "length"), 3)]) }
fn kurtosis(b: &Bars, prm: &Params) -> Output { Ok(vec![moment(&b.close, n(prm, "length"), 4)]) }
fn majority_rule(b: &Bars, prm: &Params) -> Output {
    let k = n(prm, "length"); let c = &b.close;
    let up: Vec<f64> = if p(prm, "variant") == 1. { (0..c.len()).map(|i| if c[i] > b.open[i] { 1. } else { 0. }).collect() }
        else { (0..c.len()).map(|i| if i == 0 { NA } else if c[i] > c[i - 1] { 1. } else { 0. }).collect() };
    Ok(vec![ta::sum(&up, k).iter().map(|s| 100. * s / k as f64).collect()])
}

pub const DEFS: &[IndicatorDef] = &[
    def("linreg_slope", "Linear Regression Slope", L14, &["slope"], PN, NO, "least-squares slope of close per bar over length bars", &[13], false, linreg_slope),
    def("linreg_intercept", "Linear Regression Intercept", L14, &["intercept"], PN, NO, "AmiBroker LinRegIntercept: a in a + b·k, k = 1..n oldest→newest (= fitted end value − slope·n)", &[13], false, linreg_intercept),
    def("tsf", "Time Series Forecast", &[int("length", "Length", 14., 2., 2000.), int("horizon", "Bars ahead (1 AmiBroker TSF; thinkorswim barPlus)", 1., 0., 500.)], &["tsf"], P, NO,
        "AmiBroker TSF: fitted end value + slope (forecast of the next bar); horizon k: fitted end value + k·slope (thinkorswim TimeSeriesForecast 'bar plus'; k = 0 is the fitted end value)", &[13], false, tsf),
    def("stderr", "Standard Error", L14S, &["stderr"], PN, NO, "√(Σ(close − fitted)²/(n − 2)) of the least-squares fit (AmiBroker StdErr; charting-library Standard Error)", &[13], false, stderr),
    def("se_bands", "Standard Error Bands", SEB, &["basis", "upper", "lower"], P, NO, "basis = sma(linreg(close, length), smooth); upper/lower = sma(linreg ± mult·stderr, smooth) (declared defaults 21/2/3)", &[22, 22, 22], false, se_bands),
    def("trend_strength", "Trend Strength Index", L14, &["tsi"], PN, NO, "Pearson correlation of close with bar index over length; zero variance → NaN (declared default 14)", &[13], false, trend_strength),
    def("rci", "Rank Correlation Index", RCI, &["rci"], PN, NO, "Spearman: time rank 1 = newest bar, price rank 1 = highest close (ties share the average rank); 100·(1 − 6Σd²/(n(n²−1)))", &[9], false, rci),
    def("rci_ribbon", "RCI Ribbon", RIB, &["short", "middle", "long"], PN, NO, "rci(close, short), rci(close, middle), rci(close, long)", &[9, 29, 49], false, rci_ribbon),
    def("percentrank", "Percent Rank", PR, &["rank"], PN, NO, "100·(count of the previous length closes ≤ close)/length (TradingView ta.percentrank); variant 1: strictly lower (AmiBroker PercentRank)", &[20], false, percentrank),
    def("percentile", "Percentile", PCT, &["percentile"], P, NO, "sorted window of the last length closes: variant 0 linear interpolation at position (n−1)·pct/100 (TradingView ta.percentile_linear_interpolation, AmiBroker Percentile); variant 1 nearest rank ceil(pct/100·n) (minimum 1) (ta.percentile_nearest_rank; AmiBroker Median = pct 50)", &[19], false, percentile),
    def("zscore", "Z-Score (Bandy)", L20, &["z"], PN, NO, "(close − sma(close, n))/stdev(close, n) (population); zero stdev → NaN", &[19], false, zscore),
    def("skewness", "Skewness", M20, &["skew"], PN, NO, "population skewness of close: mean((x − m)³)/σ³ (AmiBroker Skewness, population = True); σ = 0 → NaN", &[19], false, skewness),
    def("kurtosis", "Kurtosis", M20, &["kurt"], PN, NO, "population excess kurtosis: mean((x − m)⁴)/σ⁴ − 3 (AmiBroker Kurtosis, population = True); σ = 0 → NaN", &[19], false, kurtosis),
    def("majority_rule", "Majority Rule", &[len("length", "Length", 14.), int("variant", "Variant (0 close > previous close: TradingView/TradeStation/ChartIQ Psychological Line, 1 close > open: NinjaTrader 8 PsychologicalLine)", 0., 0., 1.)], &["pct"], PN, NO,
        "100·(number of bars with close > close[1] in the last length bars)/length (declared; charting-library formula unpublished); variant 1 (NinjaTrader 8 PsychologicalLine, default length 10): counts bars with close > open, so bar 0 counts and the first value is at bar length − 1 (NinjaTrader also prints a partial-window value over the first length − 1 bars; here NaN)", &[14], false, majority_rule),
];

#[cfg(test)]
mod tests {
    use crate::lab::testdata::{assert_series, closes, compiles, d1, go};
    const N: f64 = f64::NAN;
    fn one(id: &str, kv: &[(&str, f64)]) -> Vec<f64> { go(id, &d1(), kv).remove(0) }
    #[test] fn regression_parts_len3() {
        let l = [("length", 3.)];
        assert_series(&one("linreg_slope", &l), &[N, N, 0.75, -0.25, -0.9, -0.1, 0.85, 0.9]);
        assert_series(&one("linreg_intercept", &l), &[N, N, 9.833333333333, 12.0, 12.866666666667, 10.866666666667, 9.266666666667, 9.966666666667]);
        assert_series(&one("tsf", &l), &[N, N, 12.833333333333, 11.0, 9.266666666667, 10.466666666667, 12.666666666667, 13.566666666667]);
        assert_series(&one("stderr", &l), &[N, N, 0.204124145232, 0.612372435696, 0.081649658093, 0.571547606649, 0.204124145232, 0.163299316186]);
        assert_series(&one("trend_strength", &l), &[N, N, 0.981980506062, -0.5, -0.997948715789, -0.240192230708, 0.98588695811, 0.9918697838]);
    }
    #[test] fn se_bands_len3() {
        let o = go("se_bands", &d1(), &[("length", 3.), ("mult", 2.), ("smooth", 2.)]);
        assert_series(&o[0], &[N, N, N, 11.666666666667, 10.708333333333, 10.366666666667, 11.191666666667, 12.241666666667]);
        assert_series(&o[1], &[N, N, N, 12.483163247594, 11.402355427122, 11.019863931409, 11.967338418548, 12.609090128084]);
        assert_series(&o[2], &[N, N, N, 10.850170085739, 10.014311239545, 9.713469401924, 10.415994914785, 11.874243205249]);
    }
    #[test] fn rci_len4_and_ties() {
        assert_series(&one("rci", &[("length", 4.)]), &[N, N, N, 40.0, -80.0, -80.0, 40.0, 100.0]);
        // closes 1,2,2,3: price ranks 4, 2.5, 2.5, 1 vs time ranks 4,3,2,1 → Σd² = 0.5 → 100·(1 − 3/60) = 95
        assert_series(&go("rci", &closes(&[1., 2., 2., 3.]), &[("length", 4.)])[0], &[N, N, N, 95.]);
    }
    #[test] fn flat_window_has_no_trend_strength() { assert!(go("trend_strength", &closes(&[5.; 4]), &[("length", 3.)])[0][3].is_nan()); }
    #[test] fn rules_reach_regression() { for (id, out) in [("linreg_slope", "slope"), ("se_bands", "lower"), ("rci_ribbon", "long")] { compiles(id, out).unwrap(); } }
    #[test] fn percentrank_variant_counts_ties_differently() {
        assert_series(&one("percentrank", &[("length", 3.), ("variant", 0.)]), &[N, N, N, 33.333333333333, 0.0, 33.333333333333, 100.0, 100.0]);
        let t = closes(&[1., 2., 2., 2.]);
        assert_series(&go("percentrank", &t, &[("length", 3.), ("variant", 0.)])[0], &[N, N, N, 100.]);
        assert_series(&go("percentrank", &t, &[("length", 3.), ("variant", 1.)])[0], &[N, N, N, 33.333333333333]);
    }
    #[test] fn percentile_interpolated_and_nearest_rank() {
        assert_series(&one("percentile", &[("length", 4.), ("pct", 50.), ("variant", 0.)]), &[N, N, N, 11.25, 11.25, 10.9, 10.9, 11.35]);
        assert_series(&one("percentile", &[("length", 4.), ("pct", 50.), ("variant", 1.)]), &[N, N, N, 11.0, 11.0, 10.8, 10.8, 10.8]);
        assert_series(&one("percentile", &[("length", 4.), ("pct", 30.), ("variant", 0.)]), &[N, N, N, 10.95, 10.92, 10.74, 10.74, 10.74]);
    }
    #[test] fn percentile_bounds_pick_min_and_max() {
        let t = closes(&[3., 1., 2.]);
        assert_series(&go("percentile", &t, &[("length", 3.), ("pct", 0.), ("variant", 0.)])[0], &[N, N, 1.]);
        assert_series(&go("percentile", &t, &[("length", 3.), ("pct", 0.), ("variant", 1.)])[0], &[N, N, 1.]);
        assert_series(&go("percentile", &t, &[("length", 3.), ("pct", 100.), ("variant", 1.)])[0], &[N, N, 3.]);
        assert_series(&go("percentile", &t, &[("length", 3.), ("pct", 100.), ("variant", 0.)])[0], &[N, N, 3.]);
    }
    #[test] fn moments_and_zscore() {
        assert_series(&one("zscore", &[("length", 3.)]), &[N, N, 1.06904496765, -1.224744871392, -1.176964698553, 0.392232270276, 1.325838426669, 1.124803176665]);
        assert_series(&one("skewness", &[("length", 4.)]), &[N, N, N, 0.0, -0.28629553356, 0.440866714177, 0.353461337519, 0.05239638633]);
        assert_series(&one("kurtosis", &[("length", 4.)]), &[N, N, N, -1.36, -1.234304760038, -1.0, -1.022679189323, -1.571600570393]);
        assert!(go("skewness", &closes(&[2.; 4]), &[("length", 3.)])[0][3].is_nan());
    }
    #[test] fn flat_windows_are_nan_not_rounding_noise() {
        let t = closes(&[0.1; 6]); // sum([0.1; 3]) / 3 != 0.1 in floating point
        for id in ["skewness", "kurtosis", "trend_strength"] { assert!(go(id, &t, &[("length", 3.)])[0].iter().all(|v| v.is_nan()), "{id}"); }
    }
    #[test] fn nearest_rank_rounds_up_and_ties_are_not_up_days() {
        assert_series(&go("percentile", &closes(&[4., 3., 2., 1.]), &[("length", 4.), ("pct", 30.), ("variant", 1.)])[0], &[N, N, N, 2.]); // ceil(1.2) = rank 2
        assert_series(&go("majority_rule", &closes(&[1., 1., 2., 2.]), &[("length", 3.)])[0], &[N, N, N, 33.333333333333]);
    }
    #[test] fn tsf_horizon_and_psychological_line_close_over_open() {
        assert_series(&one("tsf", &[("length", 3.), ("horizon", 2.)]), &[N, N, 13.583333333333, 10.75, 8.366666666667, 10.366666666667, 13.516666666667, 14.466666666667]);
        assert_series(&one("tsf", &[("length", 3.), ("horizon", 0.)])[2..3], &[12.083333333333]);
        // close > open: 1, 1, 1, 0, 0, 1, 1, 1 (bar 0 counts)
        assert_series(&one("majority_rule", &[("length", 3.), ("variant", 1.)]), &[N, N, 100., 66.666666666667, 33.333333333333, 33.333333333333, 66.666666666667, 100.]);
    }
    #[test] fn majority_rule_len3() { assert_series(&one("majority_rule", &[("length", 3.)]), &[N, N, N, 66.666666666667, 33.333333333333, 33.333333333333, 66.666666666667, 100.0]); }
    #[test] fn rules_reach_distribution() { for (id, out) in [("percentrank", "rank"), ("percentile", "percentile"), ("kurtosis", "kurt")] { compiles(id, out).unwrap(); } }
}
