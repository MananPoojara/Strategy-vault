//! Census group "stats_vol" (2026-10-26, research: sv-census-work/research_stats_vol.md). Formulas: see each definition string.
//! True Bar Average (Sierra Chart) is listed only: its formula is confirmed but needs per-bar volume-at-price data the lab does not hold.
//! Nothing in the group was UNCONFIRMED. Partly unpublished defaults are declared lab choices in the definitions (Linear Estimation
//! length, Covariance length, MAPE length); Detrended Synthetic Price follows the platform scripts, not Ehlers' book text (not checked).
use super::bars::Bars;
use super::catalogue::{get, int, len, n, p, real, run, IndicatorDef, Needs, Output, ParamSpec, Params, Placement};
use super::ta::{self, NA};

#[allow(clippy::too_many_arguments)]
const fn def(id: &'static str, name: &'static str, category: &'static str, params: &'static [ParamSpec], outputs: &'static [&'static str], placement: Placement,
             needs: Needs, definition: &'static str, first: &'static [usize], f: fn(&Bars, &Params) -> Output) -> IndicatorDef {
    IndicatorDef { id, name, category, params, outputs, placement, needs, definition, first, sparse: false, compute: Some(f), note: "" }
}
const BC: &str = "Bands & channels";
const ST: &str = "Statistics";
const PR: Placement = Placement::Price;
const PN: Placement = Placement::Pane;
const NO: Needs = Needs::NONE;

fn call(id: &str, b: &Bars, kv: &[(&str, f64)]) -> Result<Vec<Vec<f64>>, String> {
    run(&get(id).ok_or_else(|| format!("{id} missing"))?, b, &kv.iter().map(|(k, v)| (k.to_string(), *v)).collect())
}
fn mean(w: &[f64]) -> f64 { w.iter().sum::<f64>() / w.len() as f64 }

const HB: &[ParamSpec] = &[len("length", "Length", 10.), real("extreme", "Extreme band (% of the average)", 0.05, 0., 100., 0.0001),
    real("outer", "Outer band (% of the average)", 0.025, 0., 100., 0.0001), real("inner", "Inner band (% of the average)", 0.0125, 0., 100., 0.0001)];
fn hurst_bands(b: &Bars, prm: &Params) -> Output {
    let (k, hl) = (n(prm, "length"), b.hl2());
    let d = k / 2 + 1;
    let xd: Vec<f64> = (0..hl.len()).map(|i| if i >= d { hl[i - d] } else { NA }).collect();
    let ma = ta::sma(&xd, k);
    let band = |key: &str, s: f64| ma.iter().map(|v| v + s * p(prm, key) / 100. * v).collect::<Vec<_>>();
    Ok(vec![ma.clone(), band("inner", 1.), band("inner", -1.), band("outer", 1.), band("outer", -1.), band("extreme", 1.), band("extreme", -1.)])
}

const DRB: &[ParamSpec] = &[int("range_study", "Range study (Sierra Chart 0 Average Daily Range, 1 Average True Range)", 0., 0., 1.),
    len("length", "Range study length (Sierra Chart defaults: ADR 30, ATR 14)", 30.), real("mult", "Band multiplier (0 acts as 1)", 1., 0., 100., 0.01),
    int("gap_adjust", "Gap adjust with yesterday's close (0 no, 1 yes)", 1., 0., 1.), int("displacement", "Daily range displacement (bars)", 0., 0., 1000.)];
fn daily_range_bands(b: &Bars, prm: &Params) -> Output {
    let (k, m, dsp) = (n(prm, "length"), b.len(), n(prm, "displacement"));
    let r = if p(prm, "range_study") == 1. { ta::sma(&ta::tr(b, true), k) } else { call("adr", b, &[("length", k as f64)])?.swap_remove(0) };
    let mult = if p(prm, "mult") == 0. { 1. } else { p(prm, "mult") };
    let gap = p(prm, "gap_adjust") == 1.;
    let (mut up, mut lo) = (vec![NA; m], vec![NA; m]);
    for i in dsp..m {
        let rv = r[i - dsp] * mult;
        if rv.is_nan() { continue; }
        let (h, l) = (b.high[i], b.low[i]);
        let (gl, gh) = match i.checked_sub(1).map(|j| b.close[j]) { Some(pc) if gap && pc != 0. => (l.min(pc), h.max(pc)), _ => (l, h) };
        up[i] = gl + rv; lo[i] = gh - rv;
    }
    Ok(vec![up, lo])
}

const HW: &[ParamSpec] = &[real("na", "Level smoothing", 0.2, 0.01, 1., 0.01), real("nb", "Trend smoothing", 0.1, 0.01, 1., 0.01), real("nc", "Acceleration smoothing", 0.1, 0.01, 1., 0.01),
    real("nd", "Variance smoothing", 0.1, 0.01, 1., 0.01), real("scalar", "Band width (standard deviations)", 1., 0., 100., 0.1)];
fn hwc(b: &Bars, prm: &Params) -> Output {
    let mid = call("hwma", b, &[("na", p(prm, "na")), ("nb", p(prm, "nb")), ("nc", p(prm, "nc"))])?.swap_remove(0);
    let (nd, s, c, m) = (p(prm, "nd"), p(prm, "scalar"), &b.close, b.len());
    let mut out = vec![vec![NA; m]; 4];
    let mut var = 0f64;
    for i in 0..m {
        let sd = var.sqrt(); // pandas-ta bands use the previous bar's variance
        if i > 0 { var = (1. - nd) * var + nd * (c[i - 1] - mid[i - 1]).powi(2); }
        let (lo, up) = (mid[i] - s * sd, mid[i] + s * sd);
        let w = up - lo;
        out[0][i] = lo; out[1][i] = up; out[2][i] = w;
        if w > 0. { out[3][i] = (c[i] - lo) / (w + f64::EPSILON); }
    }
    Ok(std::iter::once(mid).chain(out).collect())
}

const VT: &[ParamSpec] = &[len("atr_length", "ATR length", 21.), real("mult", "ATR multiplier", 3., 0.01, 100., 0.01), len("max_period", "Maximum dynamic period", 50.)];
fn volatility_trend(b: &Bars, prm: &Params) -> Output {
    let (k, mu, pmax, c, m) = (n(prm, "atr_length"), p(prm, "mult"), n(prm, "max_period"), &b.close, b.len());
    let mut tr = ta::tr(b, true);
    if m > 0 { tr[0] = 0.; } // Sierra returns before writing bar 0, so its TR stays 0 inside the WMA
    let (mut vti, mut dir, mut per) = (0., 0., 0usize);
    let (mut ov, mut od) = (vec![NA; m], vec![NA; m]);
    for i in 1..m {
        let l = k.min(i + 1); // Sierra WMA shrinks its length while warming up
        let atr = (0..l).map(|j| tr[i - j] * (l - j) as f64).sum::<f64>() * 2. / (l * (l + 1)) as f64;
        let d = if c[i] > vti { 1. } else { -1. };
        if d != dir { per = 0; }
        if per < pmax { per += 1; }
        let w = &c[i + 1 - per..=i];
        vti = if d == 1. { w.iter().cloned().fold(f64::MIN, f64::max) - mu * atr } else { w.iter().cloned().fold(f64::MAX, f64::min) + mu * atr };
        dir = d;
        if i + 1 >= k { ov[i] = vti; od[i] = d; }
    }
    Ok(vec![ov, od])
}

const L20: &[ParamSpec] = &[int("length", "Length", 20., 2., 2000.)];
fn covariance(b: &Bars, prm: &Params) -> Output {
    let (y, k, x) = (super::pairs::second(b)?, n(prm, "length"), &b.close);
    Ok(vec![(0..x.len()).map(|i| {
        if i + 1 < k { return NA; }
        let (wx, wy) = (&x[i + 1 - k..=i], &y[i + 1 - k..=i]);
        let (mx, my) = (mean(wx), mean(wy));
        wx.iter().zip(wy).map(|(a, c)| (a - mx) * (c - my)).sum::<f64>() / k as f64
    }).collect()])
}

fn mape(b: &Bars, prm: &Params) -> Output {
    let (k, x) = (n(prm, "length"), &b.close);
    let s = ta::sma(x, k);
    Ok(vec![(0..x.len()).map(|i| if i + 1 < k { NA } else { x[i + 1 - k..=i].iter().map(|v| if *v == 0. { NA } else { (v - s[i]).abs() / v }).sum::<f64>() / k as f64 }).collect()])
}

const L10: &[ParamSpec] = &[len("length", "Length", 10.)];
fn dispersion(b: &Bars, prm: &Params) -> Output { call("variance", b, &[("length", p(prm, "length"))]) }

const L14: &[ParamSpec] = &[ParamSpec { min: 2., ..len("length", "Length", 14.) }];
fn linear_estimation(b: &Bars, prm: &Params) -> Output {
    let k = [("length", p(prm, "length"))];
    Ok(vec![call("linreg_slope", b, &k)?.swap_remove(0), call("linreg_intercept", b, &k)?.swap_remove(0)])
}

const LR: &[ParamSpec] = &[len("length", "Length", 1.), int("cumulative", "Cumulative (0 off, 1 since the first loaded bar)", 0., 0., 1.)];
fn log_return(b: &Bars, prm: &Params) -> Output {
    let (k, cum, c) = (n(prm, "length"), p(prm, "cumulative") == 1., &b.close);
    Ok(vec![(0..c.len()).map(|i| {
        let base = if cum { c[0] } else if i >= k { c[i - k] } else { return NA };
        let r = c[i] / base;
        if r > 0. && r.is_finite() { r.ln() } else { NA }
    }).collect()])
}

const DS: &[ParamSpec] = &[len("length", "Length", 14.), int("variant", "Variant (0 TradingView HPotter D_DSP, 1 MultiCharts D_DSP)", 0., 0., 1.)];
fn dsp(b: &Bars, prm: &Params) -> Output {
    let (k, hl) = (n(prm, "length"), b.hl2());
    if p(prm, "variant") == 1. {
        let a = if k > 2 { 2. / (k as f64 + 1.) } else { 0.67 };
        let (mut e1, mut e2) = (NA, NA);
        return Ok(vec![hl.iter().map(|&x| {
            if e1.is_nan() { (e1, e2) = (x, x); } else { e1 = a * x + (1. - a) * e1; e2 = a / 2. * x + (1. - a / 2.) * e2; }
            e1 - e2
        }).collect()]);
    }
    Ok(vec![ta::ema(&hl, k).iter().zip(ta::ema(&hl, 2 * k)).map(|(a, c)| a - c).collect()])
}

pub const DEFS: &[IndicatorDef] = &[
    def("hurst_bands", "Hurst Bands (Sierra Chart)", BC, HB, &["ma", "inner_top", "inner_bottom", "outer_top", "outer_bottom", "extreme_top", "extreme_bottom"], PR, NO, "Sierra Chart Hurst Bands (doc ID 446, ACSIL scsf_HurstBands; after J. M. Hurst 1970, whose centred average looks ahead and is not built): d = floor(n/2) + 1; ma = sma(hl2[d], n), the input shifted d bars back; band = ma ± (v/100)·ma for v = inner, outer, extreme (percent); first value at bar n + d − 1; defaults n 10, simple average (Sierra's other average types are not offered), extreme 0.05, outer 0.025, inner 0.0125 as the 2025 Sierra doc names them (the 2021 ACSIL source swaps the inner and extreme labels); these are intraday percentages, the doc suggests extreme 4.2, outer 2.6, inner 1.6 for daily charts", &[15; 7], hurst_bands),
    def("daily_range_bands", "Daily Range Bands (Sierra Chart)", BC, DRB, &["upper", "lower"], PR, NO, "Sierra Chart Daily Range Bands (doc ID 349, ACSIL scsf_DailyRangeBand) on daily bars, where every bar is its own day: R = Average Daily Range sma(H − L, n) (range_study 0) or Average True Range sma(tr, n), tr bar 0 = H − L (range_study 1); r = R[displacement]·mult (mult 0 acts as 1, as the source); gap adjust and C[1] ≠ 0: GL = min(L, C[1]), GH = max(H, C[1]), else GL = L, GH = H; upper = GL + r, lower = GH − r (the source multiplies only the range; the doc's bracket is misplaced); upper < lower can happen; blank until R exists (Sierra plots its partial warm-up values); the intraday stair-step, days-to-display, daily-chart high/low and stop-updating options do not apply to daily bars and are not offered; defaults ADR, n 30, mult 1, gap adjust on, displacement 0 (the doc suggests 1 for stable values)", &[29, 29], daily_range_bands),
    def("hwc", "Holt-Winters Channel (pandas-ta)", BC, HW, &["mid", "lower", "upper", "width", "pct"], PR, NO, "pandas-ta 0.4.71b0 hwc (rengel8, after an MQL5 script not compared): mid = our hwma (F = (1 − na)(F[1] + V[1] + A[1]/2) + na·close, V = (1 − nb)(V[1] + A[1]) + nb(F − F[1]), A = (1 − nc)A[1] + nc(V − V[1]), mid = F + V + A/2, seed F = close[0]); var = (1 − nd)·var[1] + nd·(close[1] − mid[1])², var 0 on bar 0; sd = √var[1] (the previous bar's variance, as pandas-ta); upper/lower = mid ± scalar·sd; width = upper − lower (0 on bars 0..2); pct = (close − lower)/(width + ε), blank while width is 0 (pandas-ta prints (close − lower)/ε there); defaults na 0.2, nb 0.1, nc 0.1, nd 0.1, scalar 1", &[0, 0, 0, 0, 3], hwc),
    def("volatility_trend", "Volatility Trend Indicator (Sierra Chart)", "Trend", VT, &["vti", "direction"], PR, NO, "Sierra Chart Volatility Trend Indicator (doc ID 155, ACSIL scsf_VolatilityTrend): bar 0 vti = direction = period = 0 and tr = 0; tr = max(H − L, |H − C[1]|, |L − C[1]|); atr = Sierra WMA(tr, n), length min(n, bar + 1) while warming up, so bar 0's zero stays in until bar n; direction = close > vti[1] ? 1 : −1 (a tie is −1); period = direction ≠ direction[1] ? 0 : period[1], then + 1 while below the maximum; uptrend vti = highest(close, period) − mult·atr, downtrend vti = lowest(close, period) + mult·atr (the source's +; the doc prints −); shown from bar n − 1 (Sierra DataStartIndex); defaults n 21 (weighted ATR; Sierra's other ATR average types are not offered), mult 3, maximum period 50, input close", &[20, 20], volatility_trend),
    def("covariance", "Covariance (Sierra Chart)", "Second instrument", L20, &["cov"], PN, Needs { second_symbol: true, ..Needs::NONE }, "Sierra Chart Covariance (doc ID 483; Skender Correlation's Covariance is the same): population covariance of close and the aligned second-instrument close over n bars, cov = sma(x·y, n) − sma(x, n)·sma(y, n), computed as Σ(x − x̄)(y − ȳ)/n; a missing value in the window gives no value; scale-dependent (raw prices); first value at bar n − 1 (Skender; the Sierra doc says from bar n); pandas rolling cov is the sample form, ×n/(n − 1); length 20 is the lab's default (Sierra and Skender publish none)", &[19], covariance),
    def("mape", "Mean Absolute Percentage Error (Skender SMA Analysis)", ST, L20, &["mape"], PN, NO, "Skender Stock.Indicators SMA Analysis Mape (v2.6.1 and v3): sma = sma(close, n); mape = Σ_{window}|close_j − sma|/close_j / n, every term against the current window's sma; a fraction, not ×100; a zero close in the window gives no value; no abs() on the denominator; siblings Mad = our mean_dev; first value at bar n − 1; length 20 is the lab's default (Skender requires one)", &[19], mape),
    def("dispersion", "Dispersion (Sierra Chart)", ST, L10, &["var"], PN, NO, "Sierra Chart Dispersion (doc ID 25, ACSIL GetDispersion): population variance Σ(close − sma(close, n))²/n = our variance (stdev²) with Sierra's default length 10; first value at bar n − 1 (Sierra writes 0 before it)", &[9], dispersion),
    def("linear_estimation", "Linear Estimation (Sierra Chart)", ST, L14, &["slope", "intercept"], PN, NO, "Sierra Chart Linear Estimation (doc ID 522, 'similar to Excel LINEST'): least squares over the last n bars with x = 1..n oldest→newest; slope b = [nΣxX − (n(n + 1)/2)ΣX]/[n·n(n + 1)(2n + 1)/6 − n²(n + 1)²/4]; intercept a = [ΣX − b·n(n + 1)/2]/n, the fitted value at x = 0 (= our linreg_slope and linreg_intercept, AmiBroker LinRegSlope / LinRegIntercept); first value at bar n − 1; length 14 is the lab's default (Sierra publishes none), at least 2", &[13, 13], linear_estimation),
    def("log_return", "Log Return (pandas-ta)", "Performance", LR, &["logret"], PN, NO, "pandas-ta 0.4.71b0 log_return: logret = ln(close/close[length]), blank on the first length bars; cumulative 1: ln(close/close at the first loaded bar), 0 on that bar, so the value depends on where the loaded history starts; a ratio that is not a positive finite number (a zero or negative close) gives no value (pandas-ta prints ±inf or NaN); defaults length 1, cumulative 0", &[1], log_return),
    def("dsp", "Detrended Synthetic Price (D_DSP)", "Cycles", DS, &["dsp"], PN, NO, "D_DSP (after Ehlers, MESA and Trading Market Cycles 1992 pp. 64–70, quarter-cycle EMA minus half-cycle EMA; the book text was not checked): variant 0 TradingView HPotter: dsp = ema(hl2, L) − ema(hl2, 2L) (SMA-seeded EMAs), first value at bar 2L − 1; variant 1 MultiCharts D_DSP.ela on daily bars: α = L > 2 ? 2/(L + 1) : 0.67, e1 = α·hl2 + (1 − α)·e1[1], e2 = (α/2)·hl2 + (1 − α/2)·e2[1], both seeded with hl2 of bar 0, dsp = e1 − e2 from bar 0 (the script's prior-day price and one-bar-back plot cancel on daily bars; its intraday day aggregation is not offered); default L 14", &[27], dsp),
    IndicatorDef { id: "true_bar_average", name: "True Bar Average (Sierra Chart)", category: "Intraday & order flow", params: &[], outputs: &["avg"], placement: PR,
        needs: Needs { intraday: true, ..Needs::NONE }, definition: "Sierra Chart True Bar Average (ACSIL scsf_TrueBarAverage; the doc page ID 395 is empty): the unweighted mean of the distinct price levels traded within the bar, from the bar's volume-at-price data (volume is not a weight); listed only, and hl2 is not offered as a stand-in", first: &[usize::MAX], sparse: false, compute: None,
        note: "not available yet — needs per-bar volume-at-price (tick) data" },
];
/// (id, platform names, AmiBroker names): one row per id in DEFS.
pub const SOURCES: &[(&str, &str, &str)] = &[
    ("hurst_bands", "Sierra Chart: Hurst Bands", ""),
    ("daily_range_bands", "Sierra Chart: Daily Range Bands", ""),
    ("hwc", "pandas-ta: hwc (Holt-Winter Channel)", ""),
    ("volatility_trend", "Sierra Chart: Volatility Trend Indicator", ""),
    ("covariance", "Sierra Chart: Covariance; Skender Stock.Indicators: Correlation (Covariance)", ""),
    ("mape", "Skender Stock.Indicators: SMA Analysis (Mape)", ""),
    ("dispersion", "Sierra Chart: Dispersion", ""),
    ("linear_estimation", "Sierra Chart: Linear Estimation", "LinRegSlope; LinRegIntercept"),
    ("log_return", "pandas-ta: log_return", ""),
    ("dsp", "TradingView (community): D_DSP (Detrended Synthetic Price) by HPotter; MultiCharts: D_DSP (Detrended Synthetic Price)", ""),
    ("true_bar_average", "Sierra Chart: True Bar Average", ""),
];

#[cfg(test)]
mod tests {
    use crate::lab::testdata::{assert_series, close_eq, closes, compiles, from_ohlc, go, walk};
    const N: f64 = f64::NAN;
    fn pair(c: &[f64], s: &[f64]) -> crate::lab::bars::Bars { let mut b = closes(c); b.second = Some(s.to_vec()); b }
    fn c18() -> Vec<f64> { (1..=8).map(f64::from).collect() }

    #[test] fn hurst_bands_average_the_shifted_hl2() {
        // n 2: d = 2, ma = mean(hl2[t−3], hl2[t−2]); n 3: d = 2, first at bar 4
        let o = go("hurst_bands", &closes(&c18()), &[("length", 2.), ("inner", 1.), ("outer", 5.), ("extreme", 10.)]);
        assert_series(&o[0], &[N, N, N, 1.5, 2.5, 3.5, 4.5, 5.5]);
        assert_series(&o[1][3..5], &[1.515, 2.525]);
        assert_series(&o[2][3..5], &[1.485, 2.475]);
        assert_series(&o[3][3..4], &[1.575]);
        assert_series(&o[6][3..4], &[1.35]);
        assert_series(&go("hurst_bands", &closes(&c18()), &[("length", 3.)])[0], &[N, N, N, N, 2., 3., 4., 5.]);
        let h = go("hurst_bands", &from_ohlc(&[(0., 4., 2., 0.), (0., 6., 2., 0.), (0., 9., 1., 0.), (0., 0., 0., 0.)]), &[("length", 1.)]);
        assert_series(&h[0], &[N, 3., 4., 5.]); // n 1: d = 1, hl2 one bar back
    }
    fn drb_bars() -> crate::lab::bars::Bars { from_ohlc(&[(10., 11., 9., 10.), (10., 12., 10., 11.), (12., 14., 12., 13.), (12., 13., 10., 11.)]) }
    #[test] fn daily_range_bands_gap_adjust_and_displace() {
        let b = drb_bars();
        let o = go("daily_range_bands", &b, &[("length", 2.)]); // ADR 2: N, 2, 2, 2.5
        assert_series(&o[0], &[N, 12., 13., 12.5]);
        assert_series(&o[1], &[N, 10., 12., 10.5]);
        assert_series(&go("daily_range_bands", &b, &[("length", 2.), ("gap_adjust", 0.)])[0], &[N, 12., 14., 12.5]);
        let d = go("daily_range_bands", &b, &[("length", 2.), ("displacement", 1.)]);
        assert_series(&d[0], &[N, N, 13., 12.]);
        assert_series(&d[1], &[N, N, 12., 11.]);
        assert_series(&go("daily_range_bands", &b, &[("length", 2.), ("mult", 2.)])[0], &[N, 14., 15., 15.]);
        assert_series(&go("daily_range_bands", &b, &[("length", 2.), ("mult", 0.)])[0], &[N, 12., 13., 12.5]);
        // ATR: tr 2, 2, 3, 3 → sma 2: N, 2, 2.5, 3
        assert_series(&go("daily_range_bands", &b, &[("length", 2.), ("range_study", 1.)])[1], &[N, 10., 11.5, 10.]);
    }
    #[test] fn hwc_matches_a_port_of_the_pandas_ta_loop() {
        // python port of pandas_ta 0.4.71b0 volatility/hwc.py, defaults
        let b = closes(&[10., 11., 13., 12., 9., 10.]);
        let o = go("hwc", &b, &[]);
        assert_series(&o[0], &[10., 10.220999999999998, 10.858053999999997, 11.198829795999998, 10.834554966104, 10.730676224660495]);
        assert_series(&o[1], &[10., 10.220999999999998, 10.858053999999997, 10.952488366272881, 10.1180293512851, 10.005241575225977]);
        assert_series(&o[2], &[10., 10.220999999999998, 10.858053999999997, 11.445171225727115, 11.551080580922898, 11.456110874095014]);
        assert_series(&o[3], &[0., 0., 0., 0.4926828594542343, 1.4330512296377975, 1.450869298869037]);
        assert_series(&o[4], &[N, N, N, 2.126137765148744, -0.7801740287872899, -0.003612713584926462]);
        assert_series(&o[0], &go("hwma", &b, &[])[0]);
    }
    #[test] fn volatility_trend_follows_the_sierra_loop() {
        // H = L = C: tr 0 (bar 0), 1, 2, 1, 3; partial WMA 2: 2/3, 5/3, 4/3, 7/3
        let b = closes(&[10., 11., 13., 12., 9.]);
        let o = go("volatility_trend", &b, &[("atr_length", 2.), ("mult", 1.), ("max_period", 2.)]);
        assert_series(&o[0], &[N, 31. / 3., 34. / 3., 35. / 3., 34. / 3.]); // period 1, 2, 2 (max), reset to 1 on the turn
        assert_series(&o[1], &[N, 1., 1., 1., -1.]);
        // n 3: bar 2 atr = (3·2 + 2·1 + 1·0)/6, bar 0's zero still inside
        let k = go("volatility_trend", &b, &[("atr_length", 3.), ("mult", 1.), ("max_period", 2.)]);
        assert_series(&k[0][..4], &[N, N, 35. / 3., 35. / 3.]);
    }
    #[test] fn covariance_is_population_and_needs_the_pair() {
        assert_series(&go("covariance", &pair(&[1., 2., 3., 5.], &[2., 4., 6., 7.]), &[("length", 3.)])[0], &[N, N, 4. / 3., 13. / 9.]);
        assert_series(&go("covariance", &pair(&[1., 2., 3., 5.], &[1., N, 3., 4.]), &[("length", 2.)])[0], &[N, N, N, 0.5]);
        let d = crate::lab::catalogue::get("covariance").unwrap();
        assert_eq!(crate::lab::catalogue::run(&d, &closes(&[1., 2.]), &Default::default()).unwrap_err(), "needs a second instrument");
        let mut full = walk(300, 2, false);
        full.second = Some(walk(300, 3, false).close);
        let want = go("covariance", &full, &[]);
        for cut in [37usize, 150, 299] {
            let mut pre = crate::lab::testdata::cut(&full, cut);
            pre.second = Some(full.second.as_ref().unwrap()[..cut].to_vec());
            let got = go("covariance", &pre, &[]);
            for i in 0..cut { assert!(close_eq(got[0][i], want[0][i]), "bar {i} changes when later bars exist"); }
        }
        assert!(want[0][..19].iter().all(|v| v.is_nan()) && want[0][19..].iter().all(|v| v.is_finite()));
    }
    #[test] fn mape_uses_the_current_window_average() {
        assert_series(&go("mape", &closes(&[1., 2., 3., 6.]), &[("length", 3.)])[0], &[N, N, 4. / 9., 13. / 27.]);
        assert_series(&go("mape", &closes(&[0., 1., 2.]), &[("length", 2.)])[0], &[N, N, 0.375]);
    }
    #[test] fn dispersion_and_linear_estimation() {
        assert_series(&go("dispersion", &closes(&[1., 3., 5.]), &[("length", 2.)])[0], &[N, 1., 1.]);
        assert_series(&go("dispersion", &closes(&[1., 3., 5.]), &[("length", 3.)])[0], &[N, N, 8. / 3.]);
        let o = go("linear_estimation", &closes(&[1., 2., 4.]), &[("length", 3.)]); // b = (3·17 − 6·7)/(3·14 − 36), a = 7/3 − 2b
        assert_series(&o[0], &[N, N, 1.5]);
        assert_series(&o[1], &[N, N, -2. / 3.]);
    }
    #[test] fn log_return_plain_cumulative_and_bad_prices() {
        let e = std::f64::consts::E;
        let b = closes(&[1., e, e * e * e]);
        assert_series(&go("log_return", &b, &[])[0], &[N, 1., 2.]);
        assert_series(&go("log_return", &b, &[("length", 2.)])[0], &[N, N, 3.]);
        assert_series(&go("log_return", &b, &[("cumulative", 1.)])[0], &[0., 1., 3.]);
        assert_series(&go("log_return", &closes(&[1., 0., 2.]), &[])[0], &[N, N, N]);
    }
    #[test] fn dsp_variants() {
        let b = closes(&[1., 2., 4.]);
        assert_series(&go("dsp", &b, &[("length", 1.)])[0], &[N, 0.5, 5. / 6.]); // ema 1 − ema 2 (SMA seed 1.5, then α 2/3)
        assert_series(&go("dsp", &b, &[("length", 3.), ("variant", 1.)])[0], &[0., 0.25, 0.8125]); // α 0.5 and 0.25
        assert_series(&go("dsp", &closes(&[1., 2.]), &[("length", 2.), ("variant", 1.)])[0], &[0., 0.67 - 0.335]);
    }
    #[test] fn true_bar_average_is_listed_only() {
        let d = crate::lab::catalogue::get("true_bar_average").unwrap();
        assert!(d.compute.is_none() && d.needs.intraday);
        assert_eq!(crate::lab::catalogue::run(&d, &closes(&[1.]), &Default::default()).unwrap_err(), "needs intraday data");
    }
    #[test] fn parameter_extremes_keep_lengths() {
        for mut bars in [walk(3, 1, true), walk(300, 1, true)] {
            bars.second = Some(vec![7.; bars.len()]);
            for d in super::DEFS.iter().filter(|d| d.compute.is_some()) {
                for pick in [|p: &crate::lab::catalogue::ParamSpec| p.min, |p: &crate::lab::catalogue::ParamSpec| p.max] {
                    let kv: Vec<(&str, f64)> = d.params.iter().map(|p| (p.key, pick(p))).collect();
                    assert!(go(d.id, &bars, &kv).iter().all(|s| s.len() == bars.len()), "{}", d.id);
                }
            }
        }
    }
    #[test] fn rules_can_read_every_output() {
        for d in super::DEFS.iter().filter(|d| d.compute.is_some() && !d.needs.second_symbol) { for o in d.outputs { compiles(d.id, o).unwrap_or_else(|e| panic!("{}.{o}: {e:?}", d.id)); } }
    }
}
