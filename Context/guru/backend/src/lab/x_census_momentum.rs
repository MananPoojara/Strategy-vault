//! Census group "momentum" (2026-10-26, research: sv-census-work/research_momentum.md). Formulas: see each definition string.
//! Skipped as UNCONFIRMED (no published formula, nothing invented): RRG Relative Strength (JdK RS-Ratio / RS-Momentum; RRG Research
//! does not publish the lengths or the normalisation). CCI Predictor (Sierra Chart) is listed, not computed: it needs Range-bar charts.
use super::bars::Bars;
use super::catalogue::{get, int, len, n, p, real, run, IndicatorDef, Needs, Output, ParamSpec, Params, Placement};
use super::ta::{self, NA};

#[allow(clippy::too_many_arguments)]
const fn def(id: &'static str, name: &'static str, params: &'static [ParamSpec], outputs: &'static [&'static str], placement: Placement,
             needs: Needs, definition: &'static str, first: &'static [usize], f: fn(&Bars, &Params) -> Output) -> IndicatorDef {
    IndicatorDef { id, name, category: M, params, outputs, placement, needs, definition, first, sparse: false, compute: Some(f), note: "" }
}
const M: &str = "Momentum";
const PN: Placement = Placement::Pane;
const NO: Needs = Needs::NONE;

fn call(id: &str, b: &Bars, kv: &[(&str, f64)]) -> Result<Vec<Vec<f64>>, String> {
    run(&get(id).ok_or_else(|| format!("{id} missing"))?, b, &kv.iter().map(|(k, v)| (k.to_string(), *v)).collect())
}
fn pct(a: f64, base: f64) -> f64 { if base > 0. && a.is_finite() { 100. * a / base } else { NA } }

const SP: &[ParamSpec] = &[int("length", "Length", 10., 2., 2000.), len("ma_length", "Moving average length", 10.)];
fn stoch_percentile(b: &Bars, prm: &Params) -> Output {
    let (k, x) = (n(prm, "length"), &b.close);
    let sp: Vec<f64> = (0..x.len()).map(|t| {
        if t + 1 < k { return NA; }
        let w = &x[t + 1 - k..=t];
        if w.iter().any(|v| v.is_nan()) { NA } else { 100. * w.iter().filter(|v| **v < x[t]).count() as f64 / (k - 1) as f64 }
    }).collect();
    let avg = ta::sma(&sp, n(prm, "ma_length"));
    Ok(vec![sp, avg])
}

const SM: &[ParamSpec] = &[real("alpha", "Cyber Cycle alpha", 0.07, 0.001, 1., 0.001), len("median_length", "Median phase change length", 5.),
    real("cutoff", "SuperSmoother cutoff", 8., 1., 2000., 1.), int("variant", "Variant (0 Ehlers Cybernetic Analysis: (H+L)/2, lag Period − 1; 1 Sierra Chart: close, lag Period)", 0., 0., 1.)];
fn smoothed_adaptive_momentum(b: &Bars, prm: &Params) -> Output {
    let (sierra, a, ml) = (n(prm, "variant") == 1, p(prm, "alpha"), n(prm, "median_length"));
    let x = if sierra { b.close.clone() } else { b.hl2() };
    let m = x.len();
    let c = super::x_cycles::cyber(&x, |_| a);
    let (mut q1, mut dp, mut v1) = (vec![NA; m], vec![NA; m], vec![NA; m]);
    let (mut inst, mut per) = (0., 0.);
    for t in 0..m {
        if t >= 6 && [0, 2, 4, 6].iter().all(|&k| c[t - k].is_finite()) {
            q1[t] = (0.0962 * c[t] + 0.5769 * c[t - 2] - 0.5769 * c[t - 4] - 0.0962 * c[t - 6]) * (0.5 + 0.08 * inst);
        }
        if t >= 1 && q1[t].is_finite() && q1[t - 1].is_finite() {
            let (i1, i0) = (c[t - 3], c[t - 4]);
            let raw = if q1[t] != 0. && q1[t - 1] != 0. { (i1 / q1[t] - i0 / q1[t - 1]) / (1. + i1 * i0 / (q1[t] * q1[t - 1])) } else { 0. };
            dp[t] = if raw > 1.1 { 1.1 } else if raw >= 0.1 { raw } else { 0.1 }; // a 0/0 NaN falls to 0.1 like the Q1 = 0 case
        }
        if t + 1 >= ml && dp[t + 1 - ml..=t].iter().all(|v| v.is_finite()) {
            let mut w = dp[t + 1 - ml..=t].to_vec();
            w.sort_by(f64::total_cmp);
            let md = w[ml / 2];
            let dc = if md == 0. { 15. } else { 2. * std::f64::consts::PI / md + 0.5 };
            inst = 0.33 * dc + 0.67 * inst;
            per = 0.15 * inst + 0.85 * per;
            let lag = (per as usize).saturating_sub(if sierra { 0 } else { 1 });
            v1[t] = x[t] - x[t.saturating_sub(lag)]; // a lag past bar 0 reads bar 0, as Sierra's arrays clamp the index
        }
    }
    let s = v1.iter().position(|v| v.is_finite()).unwrap_or(m);
    let mut out = vec![NA; m];
    out[s..].copy_from_slice(&super::x_ma::supersmoother_s(&v1[s..], p(prm, "cutoff"), 3));
    Ok(vec![out])
}

const TM: &[ParamSpec] = &[len("long_length", "Long VIDYA length", 25.), int("long_sd_length", "Long standard deviation length", 13., 2., 2000.),
    real("long_ref_sd", "Long reference standard deviation (price units)", 6., 0.0001, 1e9, 0.01), len("short_length", "Short VIDYA length", 12.),
    int("short_sd_length", "Short standard deviation length", 10., 2., 2000.), real("short_ref_sd", "Short reference standard deviation (price units)", 4., 0.0001, 1e9, 0.01),
    len("length", "Turbo MACD length", 9.)];
fn turbo_macd(b: &Bars, prm: &Params) -> Output {
    let (x, m) = (&b.close, b.len());
    let (sl, ss) = (n(prm, "long_sd_length"), n(prm, "short_sd_length"));
    let (dl, ds) = (ta::stdev(x, sl), ta::stdev(x, ss));
    let (kl, ks) = (2. / (p(prm, "long_length") + 1.) / p(prm, "long_ref_sd"), 2. / (p(prm, "short_length") + 1.) / p(prm, "short_ref_sd"));
    let a = 2. / (p(prm, "length") + 1.);
    let (mut ravi, mut tm, mut vl, mut vs) = (vec![NA; m], vec![NA; m], NA, NA);
    let t0 = sl.max(ss) - 2; // Sierra seeds both VIDYAs with the input here, one bar before the longer window is full
    for t in t0..m {
        if t == t0 { (vl, vs, ravi[t], tm[t]) = (x[t], x[t], 0., 0.); continue; }
        let (fl, fs) = (kl * dl[t], ks * ds[t]); // not clamped: σ above σref·(n + 1)/2 overshoots, as in Sierra
        vl = fl * x[t] + (1. - fl) * vl;
        vs = fs * x[t] + (1. - fs) * vs;
        ravi[t] = vs - vl;
        tm[t] = ravi[t] - (a * ravi[t] + (1. - a) * ravi[t - 1]);
    }
    Ok([ravi, tm].into_iter().map(|s| s.into_iter().map(|v| if v.is_finite() { v } else { NA }).collect()).collect())
}

const L10: &[ParamSpec] = &[len("length", "Length", 10.)];
fn qstick(b: &Bars, prm: &Params) -> Output {
    let body: Vec<f64> = b.close.iter().zip(&b.open).map(|(c, o)| c - o).collect();
    Ok(vec![ta::sma(&body, n(prm, "length"))])
}

const IF: &[ParamSpec] = &[len("hl_length", "Highest/lowest length", 10.), len("wma_length", "Input WMA length", 9.), len("ma_length", "Output SMA length", 3.),
    int("input", "Input data (0 Open, Sierra Chart's default; 1 Close)", 0., 0., 1.)];
fn inverse_fisher(b: &Bars, prm: &Params) -> Output {
    let x = if n(prm, "input") == 1 { &b.close } else { &b.open };
    let k = n(prm, "hl_length");
    let (hi, lo) = (ta::highest(x, k), ta::lowest(x, k));
    // a zero range leaves ξ unwritten (0 in Sierra's array) and carries the transform
    let xi: Vec<f64> = (0..x.len()).map(|i| if hi[i] == lo[i] { 0. } else { 10. * (x[i] - lo[i]) / (hi[i] - lo[i]) - 5. }).collect();
    let w = ta::wma(&xi, n(prm, "wma_length"));
    let mut prev = NA;
    let ift: Vec<f64> = (0..x.len()).map(|i| { if hi[i] != lo[i] { prev = w[i].tanh(); } prev }).collect();
    let avg = ta::sma(&ift, n(prm, "ma_length"));
    Ok(vec![ift, avg])
}

const RE: &[ParamSpec] = &[len("length", "Length", 5.), int("variant", "Condition (0 DeMark / ProRealCode, 1 Sierra Chart / Infront)", 0., 0., 1.)];
fn range_expansion_index(b: &Bars, prm: &Params) -> Output {
    let (h, l, c, m) = (&b.high, &b.low, &b.close, b.len());
    let sierra = n(prm, "variant") == 1;
    let (mut num, mut den) = (vec![NA; m], vec![NA; m]);
    for i in 8..m {
        let (dh, dl) = (h[i] - h[i - 2], l[i] - l[i - 2]);
        let (c1, c2) = (h[i] >= l[i - 5] || h[i] >= l[i - 6], h[i - 2] >= c[i - 7] || h[i - 2] >= c[i - 8]);
        let (c3, c4) = (l[i] <= h[i - 5] || l[i] <= h[i - 6], l[i - 2] <= c[i - 7] || l[i - 2] <= c[i - 8]);
        let ok = if sierra { (c1 && c3) || (c2 && c4) } else { (c1 || c2) && (c3 || c4) };
        (num[i], den[i]) = (if ok { dh + dl } else { 0. }, dh.abs() + dl.abs());
    }
    let k = n(prm, "length");
    Ok(vec![ta::sum(&num, k).iter().zip(ta::sum(&den, k)).map(|(a, d)| if d == 0. || d.is_nan() { NA } else { 100. * a / d }).collect()])
}

const DS: &[ParamSpec] = &[len("length", "Lookback bars (252 = 52 weeks, 189 = 9 months, 126 = 6 months, 63 = 3 months)", 252.),
    int("basis", "High/low type (0 closing, StockCharts CLOSE; 1 intraday, StockCharts INTRADAY)", 0., 0., 1.)];
fn extreme(b: &Bars, prm: &Params, high: bool) -> Vec<f64> {
    let (k, intra) = (n(prm, "length"), n(prm, "basis") == 1);
    if high { ta::highest(if intra { &b.high } else { &b.close }, k) } else { ta::lowest(if intra { &b.low } else { &b.close }, k) }
}
fn distance_from_highs(b: &Bars, prm: &Params) -> Output { Ok(vec![extreme(b, prm, true).iter().zip(&b.close).map(|(&e, c)| pct(c - e, e)).collect()]) }
fn distance_from_lows(b: &Bars, prm: &Params) -> Output { Ok(vec![extreme(b, prm, false).iter().zip(&b.close).map(|(&e, c)| pct(c - e, e)).collect()]) }
fn distance_to_highs(b: &Bars, prm: &Params) -> Output { Ok(vec![extreme(b, prm, true).iter().zip(&b.close).map(|(e, &c)| pct(e - c, c)).collect()]) }
fn distance_to_lows(b: &Bars, prm: &Params) -> Output { Ok(vec![extreme(b, prm, false).iter().zip(&b.close).map(|(e, &c)| pct(e - c, c)).collect()]) }

fn performance_spread(b: &Bars, _: &Params) -> Output {
    let (s, c) = (super::pairs::second(b)?, &b.close);
    let a = (0..c.len()).find(|&i| c[i].is_finite() && s[i].is_finite());
    Ok(vec![(0..c.len()).map(|i| match a {
        Some(a) if i >= a && c[a] != 0. && s[a] != 0. => 100. * (c[i] - c[a]) / c[a] - 100. * (s[i] - s[a]) / s[a],
        _ => NA,
    }).collect()])
}

const DD: &[ParamSpec] = &[int("length", "Divergence length", 10., 2., 2000.), real("threshold", "Divergence threshold (degrees)", 45., -90., 90., 0.5),
    real("opposite_threshold", "Opposite slope divergence threshold (degrees)", 10., -90., 90., 0.5), int("rsi_length", "RSI length (the oscillator)", 14., 2., 2000.),
    real("price_scale", "Price per bar unit (Sierra Chart: the chart's Value Increment Per Bar)", 1., 0.0001, 1e9, 0.0001),
    real("osc_scale", "Oscillator value per point (Sierra Chart: Value Per Point for Study Reference)", 1., 0.0001, 1e9, 0.0001)];
fn divergence_detector(b: &Bars, prm: &Params) -> Output {
    let k = n(prm, "length");
    let osc = call("rsi", b, &[("length", p(prm, "rsi_length"))])?.swap_remove(0);
    // least-squares slope = fitted value at the newest bar − fitted value one bar earlier
    let angle = |x: &[f64], scale: f64| { let s: Vec<f64> = x.iter().map(|v| v / scale).collect(); ta::linreg(&s, k, 0).iter().zip(ta::linreg(&s, k, 1)).map(|(a, c)| (a - c).atan().to_degrees()).collect::<Vec<f64>>() };
    let (pa, oa) = (angle(&b.close, p(prm, "price_scale")), angle(&osc, p(prm, "osc_scale")));
    let (th, op) = (p(prm, "threshold"), p(prm, "opposite_threshold"));
    let (mut pos, mut neg) = (vec![NA; b.len()], vec![NA; b.len()]);
    for i in 0..b.len() {
        if pa[i].is_nan() || oa[i].is_nan() { continue; }
        let d = pa[i] - oa[i];
        let div = d.abs() >= th || (pa[i] > 0. && oa[i] < 0. && d.abs() >= op) || (pa[i] < 0. && oa[i] > 0. && d.abs() >= op);
        (pos[i], neg[i]) = if !div { (0., 0.) } else if oa[i] > pa[i] { (1., 0.) } else { (0., 1.) };
    }
    Ok(vec![pa, oa, pos, neg])
}

pub const DEFS: &[IndicatorDef] = &[
    def("stoch_percentile", "Stochastic - Percentile (Worden, Sierra Chart)", SP, &["sp", "sp_avg"], PN, NO, "Sierra Chart Stochastic - Percentile (Worden Stochastic; ACSIL source): window = the last n closes including this bar; rank = count of window values strictly below close (ties take the lowest rank); sp = 100·rank/(n − 1), 0..100; sp_avg = sma(sp, m); no value until the window is full (the doc's expanding start is not in the source; a missing close in the window gives no value); unlike our percentrank (previous n bars, ≤, ÷ n); defaults n 10, m 10; guide lines 20/80", &[9, 18], stoch_percentile),
    def("smoothed_adaptive_momentum", "Smoothed Adaptive Momentum (Ehlers)", SM, &["sam"], PN, NO, "Ehlers, Cybernetic Analysis (2004) ch.12, via Sierra Chart ACSIL and the Gehtsoft port: price x = (H+L)/2 (variant 1 Sierra: close); cycle = our cyber_cycle recursion (alpha; bars 2..5 (x − 2x[1] + x[2])/4); ch.9 cycle period: Q1 = (0.0962c + 0.5769c[2] − 0.5769c[4] − 0.0962c[6])·(0.5 + 0.08·inst[1]), I1 = c[3]; Δ = (I1/Q1 − I1[1]/Q1[1])/(1 + I1·I1[1]/(Q1·Q1[1])), 0 when Q1 or Q1[1] is 0, clamped to 0.1..1.1; md = median of the last 5 Δ (upper middle for even lengths, as Sierra); dc = 2π/md + 0.5; inst = 0.33·dc + 0.67·inst[1], period = 0.15·inst + 0.85·period[1] (both from 0); value1 = x − x[lag], lag = int(period) − 1 (variant 1 Sierra source: int(period)), a lag past bar 0 reads bar 0 (Sierra's index clamp); sam = 3-pole SuperSmoother(value1, cutoff) (a = e^(−π/cutoff), b = 2a·cos(1.738π/cutoff), seeded with value1 on its first 3 bars); from the first full median window (bar 13); Sierra's default Cyber Cycle length 28 is alpha 2/29 ≈ 0.069; defaults alpha 0.07, median 5, cutoff 8", &[13], smoothed_adaptive_momentum),
    def("turbo_macd", "Turbo MACD (Sierra Chart)", TM, &["ravi", "tmacd"], PN, NO, "Sierra Chart Turbo MACD (Chande VIDYA; ACSIL source): σL = stdev(close, 13), σS = stdev(close, 10) (population); at bar t0 = max(13, 10) − 2 both VIDYAs = close and ravi = tmacd = 0; after it kL = 2/(25 + 1)·σL/σrefL, VL = kL·close + (1 − kL)·VL[1], VS likewise with 12, σS, σrefS; ravi = VS − VL; tmacd = ravi − (a·ravi + (1 − a)·ravi[1]), a = 2/(9 + 1) (a one-step blend as in the source, not an EMA); σref is a fixed price amount (defaults 6 and 4), so k grows with the instrument's price level and is not clamped: set σref to the instrument's typical σ; an overflowing recursion gives no value", &[11, 11], turbo_macd),
    def("qstick", "QStick (Chande)", L10, &["qstick"], PN, NO, "Chande & Kroll (The New Technical Trader, 1994), TA-Lib / Tulip / pandas-ta qstick: sma(close − open, n); default n 10", &[9], qstick),
    def("inverse_fisher", "Inverse Fisher Transform (Sierra Chart)", IF, &["ift", "ift_avg"], PN, NO, "Sierra Chart Inverse Fisher Transform (ACSIL source): hi/lo = highest/lowest of the input over n_HL bars including this bar; ξ = 10·(x − lo)/(hi − lo) − 5; w = wma(ξ, n_I); ift = (e^(2w) − 1)/(e^(2w) + 1) = tanh(w); hi = lo → ξ = 0 (Sierra leaves it unwritten) and ift = ift[1]; ift_avg = sma(ift, n_O); input: Sierra's default is the open (input 0), close is input 1; first ift at (n_HL − 1) + (n_I − 1); our ift_rsi is Ehlers' RSI form; defaults n_HL 10, WMA 9, SMA 3; guide lines ±0.5", &[17, 19], inverse_fisher),
    def("range_expansion_index", "Range Expansion Index (DeMark)", RE, &["rei"], PN, NO, "DeMark TD REI (New Market Timing Techniques, 1994): dh = H − H[2], dl = L − L[2]; c1 = H ≥ L[5] or H ≥ L[6]; c2 = H[2] ≥ C[7] or H[2] ≥ C[8]; c3 = L ≤ H[5] or L ≤ H[6]; c4 = L[2] ≤ C[7] or L[2] ≤ C[8]; variant 0 (DeMark text, ProRealCode): counted when (c1 or c2) and (c3 or c4); variant 1 (Sierra Chart basic mode, Infront): when (c1 and c3) or (c2 and c4); rei = 100·Σ_n(counted ? dh + dl : 0)/Σ_n(|dh| + |dl|), −100..100; denominator 0 → no value (Sierra holds the previous value); from bar 8 + n − 1; Sierra's advanced mode rewrites the previous bar and is not offered; Sierra's default length is 8; default n 5; guide lines ±45", &[12], range_expansion_index),
    def("distance_from_highs", "Distance From Highs (StockCharts)", DS, &["value"], PN, NO, "StockCharts Distance From Highs: 100·(close − high_n)/high_n, ≤ 0; high_n = highest close over n bars including this bar (basis 1 INTRADAY: highest high); StockCharts' calendar lookbacks as bar counts: 252 (52 weeks, default), 189, 126, 63; a high ≤ 0 gives no value", &[251], distance_from_highs),
    def("distance_from_lows", "Distance From Lows (StockCharts)", DS, &["value"], PN, NO, "StockCharts Distance From Lows: 100·(close − low_n)/low_n, ≥ 0; low_n = lowest close over n bars including this bar (basis 1 INTRADAY: lowest low); lookbacks 252 (52 weeks, default), 189, 126, 63; a low ≤ 0 gives no value", &[251], distance_from_lows),
    def("distance_to_highs", "Distance To Highs (StockCharts)", DS, &["value"], PN, NO, "StockCharts Distance To Highs: 100·(high_n − close)/close, ≥ 0; high_n = highest close over n bars including this bar (basis 1 INTRADAY: highest high); lookbacks 252 (52 weeks, default), 189, 126, 63; a close ≤ 0 gives no value", &[251], distance_to_highs),
    def("distance_to_lows", "Distance To Lows (StockCharts)", DS, &["value"], PN, NO, "StockCharts Distance To Lows: 100·(low_n − close)/close, ≤ 0; low_n = lowest close over n bars including this bar (basis 1 INTRADAY: lowest low); lookbacks 252 (52 weeks, default), 189, 126, 63; a close ≤ 0 gives no value", &[251], distance_to_lows),
    def("performance_spread", "Performance Spread (StockCharts)", &[], &["spread"], PN, Needs { second_symbol: true, ..Needs::NONE }, "StockCharts Performance Spread: symbol performance = 100·(close − close_start)/close_start, benchmark performance likewise on the aligned second instrument (vs); spread = symbol − benchmark; start = the first loaded bar where both have a close (StockCharts: the first bar on the chart), so the value depends on where the loaded history begins; a start price of 0 gives no value; StockCharts has no length parameter (a rolling roc difference would be an adaptation and is not offered)", &[0], performance_spread),
    def("divergence_detector", "Divergence Detector (Sierra Chart)", DD, &["price_angle", "osc_angle", "positive", "negative"], PN, NO, "Sierra Chart Divergence Detector (ACSIL source): price_angle = atan(least-squares slope of close/price_scale over n bars) in degrees; osc_angle likewise for the oscillator/osc_scale; divergence when |d| ≥ threshold, or price_angle > 0 > osc_angle with |d| ≥ opposite threshold, or price_angle < 0 < osc_angle with |d| ≥ opposite threshold (d = price_angle − osc_angle); positive = 1 when osc_angle > price_angle, else negative = 1 (Sierra marks the bar's low − tick / high + tick); 0 otherwise; the oscillator is rsi(close, rsi_length) (Sierra takes any study subgraph); the angles depend on the scales, which Sierra takes from the chart (Value Increment Per Bar) and the lab sets to 1 price unit and 1 point per bar; defaults n 10, 45°, 10°; for pivot divergences see rsi_divergence", &[9, 23, 23, 23], divergence_detector),
    IndicatorDef { id: "cci_predictor", name: "CCI Predictor (Sierra Chart)", category: M, params: &[len("length", "CCI length", 14.)], outputs: &["proj_high", "proj_low"], placement: PN,
        needs: Needs { intraday: true, ..Needs::NONE }, definition: "Sierra Chart CCI Predictor (ACSIL source), last bar of a Range-bar chart with range R: proj_high = cci(n) with this bar's typical price replaced by (2(L + R) + L)/3 (the bar closing at its high), proj_low = cci(n) with (2(H − R) + H)/3; listed only", first: &[usize::MAX, usize::MAX], sparse: false, compute: None,
        note: "not available yet — needs Range-bar (tick-built) charts; the projection is defined only for the forming last Range bar" },
];
/// (id, platform names, AmiBroker names): one row per id in DEFS.
pub const SOURCES: &[(&str, &str, &str)] = &[
    ("stoch_percentile", "Sierra Chart: Stochastic - Percentile", ""),
    ("smoothed_adaptive_momentum", "Sierra Chart: Smoothed Adaptive Momentum; Gehtsoft FXCodeBase: SmoothedAdaptiveMomentum (Ehlers)", ""),
    ("turbo_macd", "Sierra Chart: Turbo MACD", ""),
    ("qstick", "TA-Lib: QSTICK; Tulip Indicators: qstick; pandas-ta: qstick", ""),
    ("inverse_fisher", "Sierra Chart: Inverse Fisher Transform", ""),
    ("range_expansion_index", "Sierra Chart: Range Expansion Index; ProRealCode: TD REI; Infront: Range Expansion Index", ""),
    ("distance_from_highs", "StockCharts: Distance From Highs", ""),
    ("distance_from_lows", "StockCharts: Distance From Lows", ""),
    ("distance_to_highs", "StockCharts: Distance To Highs", ""),
    ("distance_to_lows", "StockCharts: Distance To Lows", ""),
    ("performance_spread", "StockCharts: Performance Spread", ""),
    ("divergence_detector", "Sierra Chart: Divergence Detector", ""),
    ("cci_predictor", "Sierra Chart: CCI Predictor", ""),
];

#[cfg(test)]
mod tests {
    use crate::lab::testdata::{assert_series, close_eq, closes, compiles, cut, from_ohlc, go, walk};
    const N: f64 = f64::NAN;
    fn pair(c: &[f64], s: &[f64]) -> crate::lab::bars::Bars { let mut b = closes(c); b.second = Some(s.to_vec()); b }

    #[test] fn stoch_percentile_ranks_strictly_below() {
        let o = go("stoch_percentile", &closes(&[3., 1., 2., 2., 5.]), &[("length", 3.), ("ma_length", 2.)]);
        // [3,1,2]: one value below 2 -> 50; [1,2,2]: tie takes the lowest rank -> 50; [2,2,5] -> 100
        assert_series(&o[0], &[N, N, 50., 50., 100.]);
        assert_series(&o[1], &[N, N, N, 50., 75.]);
        assert!(go("stoch_percentile", &closes(&[4.; 12]), &[])[0][9..].iter().all(|&v| v == 0.));
    }
    #[test] fn sam_period_and_lag_on_a_ramp() {
        // ramp: cycle 0 -> Q1 0 -> Δ 0.1 -> dc 2π/0.1 + 0.5; periods 3.13, 7.90, 13.36 on bars 13..15
        let c: Vec<f64> = (0..20).map(f64::from).collect();
        let e = go("smoothed_adaptive_momentum", &closes(&c), &[]);
        assert!(e[0][..13].iter().all(|v| v.is_nan()));
        assert_series(&e[0][13..16], &[2., 6., 12.]); // Ehlers lag int(period) − 1; first 3 bars seed the SuperSmoother
        assert_series(&go("smoothed_adaptive_momentum", &closes(&c), &[("variant", 1.)])[0][13..16], &[3., 7., 13.]);
        let ss = crate::lab::x_ma::supersmoother_s(&[2., 6., 12., 16.], 8., 3);
        assert!(close_eq(e[0][16], ss[3])); // bar 16: lag int(period 18.94) − 1 = 17 > 16 reads bar 0 (Sierra clamp)
    }
    #[test] fn turbo_macd_hand_trace() {
        // sd lengths 2 -> t0 = 0; kL = 2/2·σ/1, kS = 2/4·σ/2, a = 0.5; σ = |Δ|/2: 1, 0, 2
        let kv = [("long_length", 1.), ("long_sd_length", 2.), ("long_ref_sd", 1.), ("short_length", 3.), ("short_sd_length", 2.), ("short_ref_sd", 2.), ("length", 3.)];
        let o = go("turbo_macd", &closes(&[1., 3., 3., 7.]), &kv);
        // VL 1, 3, 3, 2·7 − 3 = 11 (k > 1 overshoots); VS 1, 1.5, 1.5, 4.25
        assert_series(&o[0], &[0., -1.5, -1.5, -6.75]);
        assert_series(&o[1], &[0., -0.75, 0., -2.625]);
    }
    #[test] fn qstick_averages_the_body() {
        let b = from_ohlc(&[(10., 12., 9., 11.), (11., 11., 8., 9.), (9., 13., 9., 12.)]);
        assert_series(&go("qstick", &b, &[("length", 2.)])[0], &[N, -0.5, 0.5]);
    }
    #[test] fn inverse_fisher_hand_values_and_flat_carry() {
        let kv = [("hl_length", 2.), ("wma_length", 2.), ("ma_length", 2.), ("input", 1.)];
        // ξ 5, 5, 0 (flat range), 5; wma(2): 5, 5/3, 10/3; the flat bar carries tanh(5)
        let o = go("inverse_fisher", &closes(&[1., 2., 3., 3., 5.]), &kv);
        assert_series(&o[0], &[N, N, 5f64.tanh(), 5f64.tanh(), (10f64 / 3.).tanh()]);
        assert_series(&o[1], &[N, N, N, 5f64.tanh(), (5f64.tanh() + (10f64 / 3.).tanh()) / 2.]);
        // default input is the open (Sierra's default): falling opens, rising closes
        let b = from_ohlc(&(0..30).map(|i| { let i = i as f64; (100. - i, 200., 50., 100. + i) }).collect::<Vec<_>>());
        assert!(go("inverse_fisher", &b, &[])[0][17] < -0.99 && go("inverse_fisher", &b, &[("input", 1.)])[0][17] > 0.99);
    }
    #[test] fn rei_condition_grouping_differs() {
        let mut rows = vec![(9.5, 10., 9., 9.5); 10];
        rows[6] = (9.2, 9.4, 9., 9.2); // H[2] 9.4 below C[7], C[8] 9.5: c2 false
        rows[8] = (19.5, 20., 19., 19.5); // gap up: c1 true, c3 false, c4 true
        rows[9] = (18.5, 19.5, 18., 18.5); // all conditions hold except c3
        let b = from_ohlc(&rows);
        // bar 8: dh 10.6, dl 10; DeMark (c1 or c2) and (c3 or c4) counts it, Sierra (c1 and c3) or (c2 and c4) does not
        assert_series(&go("range_expansion_index", &b, &[("length", 1.)])[0][8..], &[100., 100.]);
        assert_series(&go("range_expansion_index", &b, &[("length", 1.), ("variant", 1.)])[0][8..], &[0., 100.]);
        // n 2 on bar 9: dh 9.5, dl 9 counted by both
        assert_series(&go("range_expansion_index", &b, &[("length", 2.), ("variant", 1.)])[0][8..], &[N, 100. * 18.5 / 39.1]);
        assert!(go("range_expansion_index", &closes(&[5.; 20]), &[])[0].iter().all(|v| v.is_nan()));
    }
    #[test] fn distances_signs_and_basis() {
        let b = from_ohlc(&[(10., 11., 9., 10.), (12., 13., 11., 12.), (9., 10., 8., 9.)]);
        let kv = [("length", 2.)];
        assert_series(&go("distance_from_highs", &b, &kv)[0], &[N, 0., -25.]);
        assert_series(&go("distance_from_lows", &b, &kv)[0], &[N, 20., 0.]);
        assert_series(&go("distance_to_highs", &b, &kv)[0], &[N, 0., 100. / 3.]);
        assert_series(&go("distance_to_lows", &b, &kv)[0], &[N, -100. / 6., 0.]);
        let i = [("length", 2.), ("basis", 1.)]; // intraday: highest high 13, lowest low 9 then 8
        assert_series(&go("distance_from_highs", &b, &i)[0], &[N, 100. * -1. / 13., 100. * -4. / 13.]);
        assert_series(&go("distance_to_lows", &b, &i)[0], &[N, 100. * -3. / 12., 100. * -1. / 9.]);
    }
    #[test] fn performance_spread_anchors_at_the_first_shared_bar() {
        assert_series(&go("performance_spread", &pair(&[10., 11., 12.], &[100., 105., 90.]), &[])[0], &[0., 5., 30.]);
        assert_series(&go("performance_spread", &pair(&[10., 11., 12.], &[N, 100., 110.]), &[])[0], &[N, 0., 100. / 11. - 10.]);
        let d = crate::lab::catalogue::get("performance_spread").unwrap();
        assert_eq!(crate::lab::catalogue::run(&d, &closes(&[1., 2.]), &Default::default()).unwrap_err(), "needs a second instrument");
        let mut full = walk(120, 4, false);
        full.second = Some((0..120).map(|i| 50. + (i % 17) as f64).collect());
        let want = go("performance_spread", &full, &[]);
        for k in (10..120).step_by(7) {
            let mut pre = cut(&full, k);
            pre.second = Some(full.second.as_ref().unwrap()[..k].to_vec());
            let got = go("performance_spread", &pre, &[]);
            for i in 0..k { assert!(close_eq(got[0][i], want[0][i]), "bar {i} cut {k}"); }
        }
    }
    #[test] fn divergence_detector_angles_on_ramps() {
        let up: Vec<f64> = (0..30).map(|i| 100. + i as f64).collect();
        let kv = [("length", 3.)];
        let o = go("divergence_detector", &closes(&up), &kv);
        // price slope 1 -> 45°; rsi stays 100 -> 0°; |d| 45 ≥ 45 and osc below price -> negative
        assert!(close_eq(o[0][2], 45.) && o[1][15].is_nan() && close_eq(o[1][16], 0.));
        assert_series(&o[2][15..17], &[N, 0.]);
        assert_series(&o[3][15..17], &[N, 1.]);
        assert_eq!(go("divergence_detector", &closes(&up), &[kv[0], ("price_scale", 2.)])[3][20], 0.); // 26.6° < 45°, osc not opposite
        let dn: Vec<f64> = up.iter().rev().cloned().collect();
        assert_eq!(go("divergence_detector", &closes(&dn), &kv)[2][20], 1.);
    }
    #[test] fn cci_predictor_is_listed_only() {
        let d = crate::lab::catalogue::get("cci_predictor").unwrap();
        assert_eq!(crate::lab::catalogue::run(&d, &walk(50, 1, false), &Default::default()).unwrap_err(), "needs intraday data");
    }
    #[test] fn parameter_bounds_never_panic() {
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
