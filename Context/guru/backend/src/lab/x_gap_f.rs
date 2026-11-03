//! Gap group "f" (2026-11-02 cross-platform survey, Statistics & Cycles items our software lacked).
//! Computed: OHL3 price, Hurst's FLD, Ehlers' DELI and Optimal Tracking Filter, a rolling ARIMA(p, d, 0) forecast, a classical
//! periodogram's dominant period, and rolling return statistics (Sharpe, Sortino, target downside deviation, upside potential,
//! Omega, Martin, historical VaR) on close-to-close returns. Listed only: Cycle Lines (a chart drawing) and Hurst Cycles (an
//! analyst's phasing method, no bar-by-bar rule). Not added, already in the catalogue: Mean Squared Error (Skender SMA Analysis
//! Mse = Σ(close − sma)²/n = the population `variance`), Rescaled Range Analysis (the R/S method of `hurst`), and the
//! OC2/HL2/HLC3/OHLC4 price transforms (rule price sources oc2, hl2, hlc3, ohlc4). Formulas: see each definition string.
use super::bars::Bars;
use super::catalogue::{int, n, p, real, IndicatorDef, Needs, Output, ParamSpec, Params, Placement};
use super::ta::NA;

#[allow(clippy::too_many_arguments)]
const fn def(id: &'static str, name: &'static str, category: &'static str, params: &'static [ParamSpec], outputs: &'static [&'static str], placement: Placement,
             needs: Needs, definition: &'static str, first: &'static [usize], sparse: bool, f: fn(&Bars, &Params) -> Output) -> IndicatorDef {
    IndicatorDef { id, name, category, params, outputs, placement, needs, definition, first, sparse, compute: Some(f), note: "" }
}
/// Listed in the picker, not computed (the app shows the note).
const fn listed(id: &'static str, name: &'static str, category: &'static str, needs: Needs, definition: &'static str, note: &'static str) -> IndicatorDef {
    IndicatorDef { id, name, category, params: &[], outputs: &["value"], placement: Placement::Pane, needs, definition,
                   first: &[usize::MAX], sparse: false, compute: None, note }
}
const PR: Placement = Placement::Price;
const PN: Placement = Placement::Pane;
const NO: Needs = Needs::NONE;
const PERF: &str = "Performance";

fn ohl3(b: &Bars, _: &Params) -> Output { Ok(vec![(0..b.len()).map(|i| (b.open[i] + b.high[i] + b.low[i]) / 3.).collect()]) }

const FL: &[ParamSpec] = &[int("cycle", "Cycle length (bars)", 20., 2., 2000.)];
/// hl2 displaced forward by half the cycle: the value at t is hl2[t − ⌊cycle/2⌋].
fn fld(b: &Bars, prm: &Params) -> Output {
    let (k, m) = (n(prm, "cycle") / 2, b.hl2());
    Ok(vec![(0..b.len()).map(|t| if t >= k { m[t - k] } else { NA }).collect()])
}

const DL: &[ParamSpec] = &[int("period", "Period", 14., 1., 2000.)];
/// MotiveWave DELI: price = midpoint of the two-bar range; dsp = ema(α) − ema(α/2) seeded with price; deli = dsp − ema(dsp, α) seeded 0.
fn deli(b: &Bars, prm: &Params) -> Output {
    let per = p(prm, "period");
    let a = if per > 2. { 2. / (per + 1.) } else { 0.67 };
    let mut o = vec![vec![NA; b.len()]; 2];
    let (mut e1, mut e2, mut tmp) = (NA, NA, 0.);
    for t in 1..b.len() {
        let x = (b.high[t].max(b.high[t - 1]) + b.low[t].min(b.low[t - 1])) / 2.;
        if e1.is_nan() { (e1, e2) = (x, x); }
        e1 = a * x + (1. - a) * e1;
        e2 = a / 2. * x + (1. - a / 2.) * e2;
        let dsp = e1 - e2;
        tmp = a * dsp + (1. - a) * tmp;
        (o[0][t], o[1][t]) = (dsp, dsp - tmp);
    }
    Ok(o)
}

/// Ehlers' Optimal Tracking Filter on hl2: tracking index λ = |v1/v2| sets the alpha of an EMA.
fn optimal_tracking_filter(b: &Bars, _: &Params) -> Output {
    let (x, len) = (b.hl2(), b.len());
    let mut o = vec![NA; len];
    if len == 0 { return Ok(vec![o]); }
    o[0] = x[0];
    let (mut v1, mut v2, mut lam) = (0., 0., 0.);
    for t in 1..len {
        v1 = 0.2 * (x[t] - x[t - 1]) + 0.8 * v1;
        v2 = 0.1 * (b.high[t] - b.low[t]) + 0.8 * v2;
        if v2 != 0. { lam = f64::abs(v1 / v2); }
        let l2: f64 = lam * lam;
        let a = (-l2 + (l2 * l2 + 16. * l2).sqrt()) / 8.;
        o[t] = a * x[t] + (1. - a) * o[t - 1];
    }
    Ok(vec![o])
}

const AR: &[ParamSpec] = &[int("p", "AR order p", 2., 1., 10.), int("d", "Differencing order d", 1., 0., 2.), int("length", "Fit window (differenced values)", 100., 3., 2000.)];
/// Levinson–Durbin recursion: Yule–Walker AR coefficients φ[1..=ord] from autocovariances r[0..=ord] (φ = 0 when r[0] = 0).
fn levinson(r: &[f64], ord: usize) -> Vec<f64> {
    let mut phi = vec![0.; ord + 1];
    let mut e = r[0];
    for k in 1..=ord {
        if e <= 0. { break; }
        let kap = (r[k] - (1..k).map(|j| phi[j] * r[k - j]).sum::<f64>()) / e;
        let prev = phi.clone();
        phi[k] = kap;
        for j in 1..k { phi[j] = prev[j] - kap * prev[k - j]; }
        e *= 1. - kap * kap;
    }
    phi
}
fn arima_forecast(b: &Bars, prm: &Params) -> Output {
    let (ord, d, nn, c) = (n(prm, "p"), n(prm, "d"), n(prm, "length"), &b.close);
    if nn < ord + 2 { return Err(format!("arima_forecast: length must be at least p + 2 ({})", ord + 2)); }
    let y: Vec<f64> = (0..b.len()).map(|i| match d {
        0 => c[i],
        1 if i >= 1 => c[i] - c[i - 1],
        2 if i >= 2 => c[i] - 2. * c[i - 1] + c[i - 2],
        _ => NA,
    }).collect();
    let (mut o, nf) = (vec![NA; b.len()], nn as f64);
    for t in (d + nn - 1)..b.len() {
        let w = &y[t + 1 - nn..=t];
        let mu = w.iter().sum::<f64>() / nf;
        let r: Vec<f64> = (0..=ord).map(|k| (k..nn).map(|i| (w[i] - mu) * (w[i - k] - mu)).sum::<f64>() / nf).collect();
        let phi = levinson(&r, ord);
        let yhat = mu + (1..=ord).map(|k| phi[k] * (w[nn - k] - mu)).sum::<f64>();
        o[t] = match d { 0 => yhat, 1 => c[t] + yhat, _ => 2. * c[t] - c[t - 1] + yhat };
    }
    Ok(vec![o])
}

const PG: &[ParamSpec] = &[int("length", "Window (bars)", 64., 4., 2000.), int("min_period", "Shortest period tested", 8., 2., 1000.),
    int("max_period", "Longest period tested", 48., 3., 2000.)];
/// Classical (Schuster) periodogram of the window's least-squares-detrended closes at each whole period; output = the peak period.
fn periodogram(b: &Bars, prm: &Params) -> Output {
    let (nn, lo, hi, c) = (n(prm, "length"), n(prm, "min_period"), n(prm, "max_period"), &b.close);
    if lo >= hi || hi > nn { return Err("periodogram: need min_period < max_period ≤ length".into()); }
    let mut o = vec![NA; b.len()];
    let nf = nn as f64;
    let (sx, sxx) = ((nn * (nn - 1) / 2) as f64, ((nn - 1) * nn * (2 * nn - 1) / 6) as f64);
    // cos/sin tables per tested period, built once
    let tab: Vec<(usize, Vec<f64>, Vec<f64>)> = (lo..=hi).map(|per| {
        let w = std::f64::consts::TAU / per as f64;
        (per, (0..nn).map(|j| (w * j as f64).cos()).collect(), (0..nn).map(|j| (w * j as f64).sin()).collect())
    }).collect();
    for t in (nn - 1)..b.len() {
        let win = &c[t + 1 - nn..=t];
        let (mut sy, mut sxy) = (0., 0.);
        for (j, &v) in win.iter().enumerate() { sy += v; sxy += j as f64 * v; }
        let slope = (nf * sxy - sx * sy) / (nf * sxx - sx * sx);
        let icpt = (sy - slope * sx) / nf;
        let x: Vec<f64> = win.iter().enumerate().map(|(j, &v)| v - icpt - slope * j as f64).collect();
        let (mut best, mut bp) = (-1., lo);
        for (per, cs, sn) in &tab {
            let (re, im) = x.iter().enumerate().fold((0., 0.), |(re, im): (f64, f64), (j, &v)| (re + v * cs[j], im + v * sn[j]));
            let pw = (re * re + im * im) / nf;
            if pw > best { (best, bp) = (pw, *per); }
        }
        o[t] = bp as f64;
    }
    Ok(vec![o])
}

/// Simple close-to-close returns (NaN on bar 0).
fn rets(c: &[f64]) -> Vec<f64> { (0..c.len()).map(|i| if i > 0 { c[i] / c[i - 1] - 1. } else { NA }).collect() }
/// f(last `nn` returns) on every bar t ≥ nn.
fn rolling(b: &Bars, nn: usize, f: impl Fn(&[f64]) -> f64) -> Output {
    let r = rets(&b.close);
    Ok(vec![(0..b.len()).map(|t| if t >= nn { f(&r[t + 1 - nn..=t]) } else { NA }).collect()])
}
fn mean(w: &[f64]) -> f64 { w.iter().sum::<f64>() / w.len() as f64 }
/// Target downside deviation: √(Σ min(r − T, 0)²/N), all N returns in the denominator.
fn tdd(w: &[f64], t: f64) -> f64 { (w.iter().map(|x| (x - t).min(0.).powi(2)).sum::<f64>() / w.len() as f64).sqrt() }
const fn length() -> ParamSpec { int("length", "Length (returns)", 252., 2., 2000.) }
const fn ppy() -> ParamSpec { int("ppy", "Bars per year (annualisation; 1 = per bar)", 252., 1., 100000.) }
const fn rf() -> ParamSpec { real("rf", "Risk-free rate (% a year)", 0., -100., 100., 0.1) }
const fn target() -> ParamSpec { real("target", "Target return per bar (%)", 0., -100., 100., 0.01) }

const SH: &[ParamSpec] = &[length(), rf(), ppy()];
fn sharpe_ratio(b: &Bars, prm: &Params) -> Output {
    let ppy = p(prm, "ppy");
    let rf = p(prm, "rf") / 100. / ppy;
    rolling(b, n(prm, "length"), |w| {
        let m = mean(w);
        let sd = (w.iter().map(|x| (x - m).powi(2)).sum::<f64>() / (w.len() - 1) as f64).sqrt();
        if sd > 0. { (m - rf) / sd * ppy.sqrt() } else { NA }
    })
}
const SO: &[ParamSpec] = &[length(), target(), ppy()];
fn sortino_ratio(b: &Bars, prm: &Params) -> Output {
    let (t, ppy) = (p(prm, "target") / 100., p(prm, "ppy"));
    rolling(b, n(prm, "length"), |w| { let d = tdd(w, t); if d > 0. { (mean(w) - t) / d * ppy.sqrt() } else { NA } })
}
fn downside_deviation(b: &Bars, prm: &Params) -> Output {
    let (t, ppy) = (p(prm, "target") / 100., p(prm, "ppy"));
    rolling(b, n(prm, "length"), |w| 100. * tdd(w, t) * ppy.sqrt())
}
const UP: &[ParamSpec] = &[length(), target()];
fn upside_potential_ratio(b: &Bars, prm: &Params) -> Output {
    let t = p(prm, "target") / 100.;
    rolling(b, n(prm, "length"), |w| {
        let (up, d) = (w.iter().map(|x| (x - t).max(0.)).sum::<f64>() / w.len() as f64, tdd(w, t));
        if d > 0. { up / d } else { NA }
    })
}
fn omega_ratio(b: &Bars, prm: &Params) -> Output {
    let t = p(prm, "target") / 100.;
    rolling(b, n(prm, "length"), |w| {
        let (up, dn) = (w.iter().map(|x| (x - t).max(0.)).sum::<f64>(), w.iter().map(|x| (t - x).max(0.)).sum::<f64>());
        if dn > 0. { up / dn } else { NA }
    })
}
const VR: &[ParamSpec] = &[length(), real("confidence", "Confidence (%)", 95., 50., 99.9, 0.1)];
fn value_at_risk(b: &Bars, prm: &Params) -> Output {
    let q = 1. - p(prm, "confidence") / 100.;
    rolling(b, n(prm, "length"), |w| {
        let mut s = w.to_vec();
        s.sort_by(|x, y| x.total_cmp(y));
        let h = (s.len() - 1) as f64 * q;
        let lo = h.floor() as usize;
        let hi = (lo + 1).min(s.len() - 1);
        -100. * (s[lo] + (h - lo as f64) * (s[hi] - s[lo]))
    })
}
const MR: &[ParamSpec] = &[length(), rf(), ppy()];
fn martin_ratio(b: &Bars, prm: &Params) -> Output {
    let (nn, ppy, rf, c) = (n(prm, "length"), p(prm, "ppy"), p(prm, "rf"), &b.close);
    Ok(vec![(0..b.len()).map(|t| {
        if t < nn { return NA; }
        let (mut peak, mut ss) = (c[t - nn], 0.);
        for &x in &c[t + 1 - nn..=t] { peak = f64::max(peak, x); ss += (100. * (x - peak) / peak).powi(2); }
        let ui = f64::sqrt(ss / nn as f64);
        let ann = 100. * ((c[t] / c[t - nn]).powf(ppy / nn as f64) - 1.);
        if ui > 0. && ann.is_finite() { (ann - rf) / ui } else { NA }
    }).collect()])
}

pub const DEFS: &[IndicatorDef] = &[
    def("ohl3", "OHL3 Price ((O+H+L)/3)", "Price functions", &[], &["ohl3"], PR, NO,
        "Skender Stock.Indicators Basic Price Transforms (Quote.GetBaseQuote / CandlePart.OHL3): ohl3 = (open + high + low)/3 on every bar, from bar 0. The sibling transforms are already rule price sources: oc2 = (open + close)/2, hl2, hlc3 and ohlc4 (and open/high/low/close/volume). Sources: Skender Stock.Indicators, Basic quote transforms (https://dotnet.stockindicators.dev/indicators/BasicQuote/), CandlePart enum (https://dotnet.stockindicators.dev/utilities/#candlepart).",
        &[0], false, ohl3),
    def("fld", "Future Line of Demarcation (FLD, Hurst)", "Cycles", FL, &["fld"], PR, NO,
        "J. M. Hurst's Future Line of Demarcation: the median price hl2 = (high + low)/2 displaced forward (to the right) by half the cycle's wavelength, so fld_t = hl2_(t − ⌊cycle/2⌋). Default cycle 20 bars (displacement 10, Hurst's nominal 20-day cycle); first value on bar ⌊cycle/2⌋. Price crossing the FLD is Hurst's signal that the cycle has turned. Causal: a forward displacement only reads earlier bars; the half-cycle segment drawn beyond the last bar on charts is display only and not output. Adaptation: the cycle length is a parameter (Hurst finds it by phasing analysis), the displacement is rounded down to whole bars, and the median price is not smoothed (some platforms smooth it). Sources: J. M. Hurst, The Profit Magic of Stock Transaction Timing (1970); LuxAlgo Library, Hurst Cycle Analysis (https://www.luxalgo.com/library/concept/hurst-cycle-analysis.md); TradingView open-source script FLD's - Future Lines of Demarcation (https://www.tradingview.com/script/rCRZL8mJ-FLD-s-Future-Lines-of-Demarcation/: a 48-bar cycle displaces its FLD 24 bars).",
        &[10], false, fld),
    def("deli", "Detrended Ehlers Leading Indicator (DELI)", "Cycles", DL, &["dsp", "deli"], PN, NO,
        "MotiveWave's Detrended Ehlers Leading Indicator (after J. Ehlers, MESA and Trading Market Cycles, 1992), from bar 1: high' = max(high, high[1]), low' = min(low, low[1]), price = (high' + low')/2; α = 2/(period + 1) when period > 2, else 0.67; ema1 = α·price + (1 − α)·ema1[1], ema2 = (α/2)·price + (1 − α/2)·ema2[1], both seeded with the first price; dsp = ema1 − ema2 (Detrended Synthetic Price); temp = α·dsp + (1 − α)·temp[1], seeded 0; deli = dsp − temp. Default period 14 (MotiveWave's only input). Both outputs start on bar 1 (bar 0 has no previous bar). The page's 'dsnp' in the temp line is read as dsp (its typo). A ProRealCode port (AlgoAlex) instead holds the last higher high / lower low and uses α = 0.67 for the first bars; that variant is not offered. The lab's dsp entry (D_DSP) is the hl2 form without the two-bar range. Sources: MotiveWave Studies C–D, Detrended Ehlers Leading Indicator (https://docs.motivewave.com/studies/c-d#detrended-ehlers-leading-indicator); ProRealCode, Ehlers Detrended Leading Indicator (https://www.prorealcode.com/prorealtime-indicators/ehlers-detrended-leading-indicator/).",
        &[1, 1], false, deli),
    def("optimal_tracking_filter", "Ehlers Optimal Tracking Filter", "Moving averages", &[], &["otf"], PR, NO,
        "J. Ehlers, Optimal Tracking Filters (MESA Software paper; Kalata's tracking index for an alpha filter): price = hl2; value1 = 0.2·(price − price[1]) + 0.8·value1[1]; value2 = 0.1·(high − low) + 0.8·value2[1]; when value2 ≠ 0, λ = |value1/value2| (otherwise λ keeps its last value); α = (−λ² + √(λ⁴ + 16λ²))/8; otf = α·price + (1 − α)·otf[1]. No parameters (the constants are Ehlers'). Adaptation: value1, value2 and λ start at 0 and otf is seeded with bar 0's hl2, so the output starts on bar 0 (the ProRealCode port starts after bar 5 from a zero filter value, which needs a long run-in). Sources: ProRealCode, John Ehlers Optimal Tracking Filter (https://www.prorealcode.com/prorealtime-indicators/john-ehlers-optimal-tracking-filter/: the code above, adapted from jamesgoulding.com); TradingView open-source script STD-Stepped Ehlers Optimal Tracking Filter by Loxx (https://www.tradingview.com/script/4ebYbQYv-STD-Stepped-Ehlers-Optimal-Tracking-Filter-MTF-w-Alerts-Loxx/).",
        &[0], false, optimal_tracking_filter),
    def("arima_forecast", "ARIMA(p, d, 0) one-bar forecast (rolling)", "Statistics", AR, &["forecast"], PR, NO,
        "Box–Jenkins ARIMA with the moving-average order q = 0, refitted on every bar: y = close differenced d times (d = 0 close, 1 close − close[1], 2 close − 2·close[1] + close[2]); on bar t the window is the last length values of y; μ = their mean; autocovariances r_k = Σ_(i≥k)(y_i − μ)(y_(i−k) − μ)/length for k = 0..p; AR coefficients φ_1..φ_p by Yule–Walker (Levinson–Durbin recursion; all 0 when r_0 = 0); ŷ = μ + Σ φ_k·(y_(t+1−k) − μ); forecast = the predicted next close: d = 0 ŷ, d = 1 close + ŷ, d = 2 2·close − close[1] + ŷ. The value on bar t forecasts bar t + 1 from bars 0..=t only. Defaults p 2, d 1, length 100 (first value on bar d + length − 1 = 100); length must be at least p + 2. Adaptation: no MA terms (q > 0 needs iterative maximum likelihood with no closed form), the mean is always included, and the fit is Yule–Walker on a rolling window rather than a one-off fit. Sources: G. E. P. Box & G. M. Jenkins, Time Series Analysis: Forecasting and Control (1970); R. J. Hyndman & G. Athanasopoulos, Forecasting: Principles and Practice (3rd ed.), ch. 9 ARIMA models (https://otexts.com/fpp3/arima.html); statsmodels yule_walker (https://www.statsmodels.org/stable/generated/statsmodels.regression.linear_model.yule_walker.html).",
        &[100], false, arima_forecast),
    def("periodogram", "Periodogram dominant period", "Cycles", PG, &["period"], PN, NO,
        "Classical (Schuster) periodogram on a rolling window: on bar t take the last length closes x_0..x_(N−1), remove their least-squares line, and for every whole period P = min_period..=max_period compute I(P) = [(Σ x_j·cos(2πj/P))² + (Σ x_j·sin(2πj/P))²]/N; period = the P with the largest I(P) (the shortest on ties). Defaults length 64, periods 8..48 (first value on bar 63); min_period < max_period ≤ length is required. Adaptation: the spectrum itself is a chart, not a bar series, so only its peak period is output. Ehlers' version (autocorrelation-based) is ehlers_acp. Sources: A. Schuster, On the investigation of hidden periodicities (Terrestrial Magnetism 3, 1898); NIST/SEMATECH e-Handbook of Statistical Methods, Spectral plot (https://www.itl.nist.gov/div898/handbook/eda/section3/spectrum.htm).",
        &[63], false, periodogram),
    def("sharpe_ratio", "Sharpe Ratio (rolling)", PERF, SH, &["sharpe"], PN, NO,
        "Sharpe ratio on a rolling window: rf_bar = rf/100/ppy (rf in % a year, default 0); sharpe = (mean(r) − rf_bar)/sd(r)·√ppy, sd = the sample standard deviation (N − 1) of the window's returns; ppy (bars per year) default 252 annualises, 1 gives the per-bar ratio; sd = 0 → NaN. r_i = close_i/close_(i−1) − 1 (simple close-to-close returns); the window is the last length returns, so the first value is on bar length (default 252). Sources: W. F. Sharpe, The Sharpe Ratio (Journal of Portfolio Management, Fall 1994, https://web.stanford.edu/~wfsharpe/art/sr/sr.htm: ex-post mean differential return over its standard deviation); R PerformanceAnalytics SharpeRatio.annualized (https://search.r-project.org/CRAN/refmans/PerformanceAnalytics/html/SharpeRatio.annualized.html).",
        &[252], false, sharpe_ratio),
    def("sortino_ratio", "Sortino Ratio (rolling)", PERF, SO, &["sortino"], PN, NO,
        "Sortino ratio on a rolling window: T = target/100 per bar (default 0); TDD = √(Σ min(r_i − T, 0)²/N) over all N returns; sortino = (mean(r) − T)/TDD·√ppy (ppy default 252; 1 gives the per-bar ratio); TDD = 0 (no return below target) → NaN. r_i = close_i/close_(i−1) − 1; the window is the last length returns, first value on bar length (default 252). Sources: F. A. Sortino & L. N. Price, Performance measurement in a downside risk framework (Journal of Investing, 1994); T. Rollinger & S. Hoffman, Sortino: A 'Sharper' Ratio (Red Rock Capital, 2013, https://www.cmegroup.com/education/files/rr-sortino-a-sharper-ratio.pdf); R PerformanceAnalytics SortinoRatio (https://search.r-project.org/CRAN/refmans/PerformanceAnalytics/html/SortinoRatio.html).",
        &[252], false, sortino_ratio),
    def("downside_deviation", "Target Downside Deviation (rolling)", PERF, SO, &["tdd"], PN, NO,
        "Target downside deviation on a rolling window, in %: T = target/100 per bar (default 0); tdd = 100·√(Σ min(r_i − T, 0)²/N)·√ppy, every one of the N returns in the denominator (returns above target count as 0); ppy default 252 annualises, 1 gives the per-bar value. r_i = close_i/close_(i−1) − 1; the window is the last length returns, first value on bar length (default 252). Sources: T. Rollinger & S. Hoffman, Sortino: A 'Sharper' Ratio (Red Rock Capital, 2013, https://www.cmegroup.com/education/files/rr-sortino-a-sharper-ratio.pdf: target downside deviation); R PerformanceAnalytics DownsideDeviation, method \"full\" (https://search.r-project.org/CRAN/refmans/PerformanceAnalytics/html/DownsideDeviation.html).",
        &[252], false, downside_deviation),
    def("upside_potential_ratio", "Upside Potential Ratio (rolling)", PERF, UP, &["upr"], PN, NO,
        "Upside potential ratio on a rolling window: T = target/100 per bar (default 0); upr = [Σ max(r_i − T, 0)/N] / √(Σ min(r_i − T, 0)²/N), both over all N returns (Sortino, van der Meer & Plantinga's form); no return below target → NaN; not annualised. r_i = close_i/close_(i−1) − 1; the window is the last length returns, first value on bar length (default 252). Sources: F. A. Sortino, R. van der Meer & A. Plantinga, The Dutch triangle (Journal of Portfolio Management, 1999); R PerformanceAnalytics UpsidePotentialRatio, method \"full\" (https://search.r-project.org/CRAN/refmans/PerformanceAnalytics/html/UpsidePotentialRatio.html).",
        &[252], false, upside_potential_ratio),
    def("omega_ratio", "Omega Ratio (rolling)", PERF, UP, &["omega"], PN, NO,
        "Omega ratio on a rolling window at threshold L = target/100 per bar (default 0): omega = Σ max(r_i − L, 0) / Σ max(L − r_i, 0) (the discrete form of Keating & Shadwick's gains-over-losses probability-weighted ratio); no return below L → NaN. r_i = close_i/close_(i−1) − 1; the window is the last length returns, first value on bar length (default 252). Sources: C. Keating & W. F. Shadwick, A Universal Performance Measure (Journal of Performance Measurement, 2002); R PerformanceAnalytics Omega (https://search.r-project.org/CRAN/refmans/PerformanceAnalytics/html/Omega.html).",
        &[252], false, omega_ratio),
    def("value_at_risk", "Value at Risk (historical, rolling)", PERF, VR, &["var"], PN, NO,
        "Historical-simulation Value at Risk on a rolling window, in % of price: sort the last length returns r_i = close_i/close_(i−1) − 1, take their (1 − confidence/100) quantile q by linear interpolation between order statistics (h = (N − 1)·(1 − c), the R type 7 / Excel PERCENTILE.INC rule) and output var = −100·q, the one-bar loss not exceeded with the given confidence (negative when even that quantile is a gain). Defaults length 252, confidence 95 (first value on bar 252). Adaptation: only the historical method (the Gaussian and Cornish–Fisher methods of PerformanceAnalytics are not offered). Sources: P. Jorion, Value at Risk (3rd ed., 2006), historical simulation; R PerformanceAnalytics VaR, method \"historical\" (https://search.r-project.org/CRAN/refmans/PerformanceAnalytics/html/VaR.html); R quantile type 7 (https://stat.ethz.ch/R-manual/R-devel/library/stats/html/quantile.html).",
        &[252], false, value_at_risk),
    def("martin_ratio", "Martin Ratio (Ulcer Performance Index, rolling)", PERF, MR, &["martin"], PN, NO,
        "Peter Martin's Ulcer Performance Index on a rolling window of the closes c_(t−N)..c_t (N = length): ann = 100·((c_t/c_(t−N))^(ppy/N) − 1), the compound return annualised in %; D_i = 100·(c_i − peak_i)/peak_i with peak_i = the highest close from c_(t−N) to c_i, for i = t−N+1..=t; UI = √(Σ D_i²/N); martin = (ann − rf)/UI. rf in % a year (default 0), ppy default 252, length default 252 (first value on bar 252). UI = 0 (no drawdown in the window) or an overflowing annualisation → NaN. Adaptation: the Ulcer Index here is measured inside the same window as the return (the lab's ulcer indicator uses a rolling highest close instead). Sources: P. Martin, The Ulcer Index (https://www.tangotools.com/ui/ui.htm: UPI = excess return / UI, also called the Martin ratio); P. Martin & B. McCann, The Investor's Guide to Fidelity Funds (1989); R PerformanceAnalytics MartinRatio (https://search.r-project.org/CRAN/refmans/PerformanceAnalytics/html/MartinRatio.html).",
        &[252], false, martin_ratio),
    listed("cycle_lines", "Cycle Lines", "Drawings & display", NO,
        "StockCharts ChartNotes Cycle Lines: equally spaced vertical lines whose start and spacing the user sets by clicking and dragging on the chart (a line study tool, used to count bars between turning points)",
        "chart drawing/display — not a bar-by-bar series; not usable in rules (the user places the first line and the spacing)"),
    listed("hurst_cycles", "Hurst Cycles", "Cycles", NO,
        "J. M. Hurst's cycle analysis (The Profit Magic of Stock Transaction Timing, 1970): nominal cycles phased from price troughs, with FLDs, valid trend lines and cycle envelopes drawn on the chart",
        "not available yet — formula not published (Hurst's phasing of cycle troughs is an analyst's judgement, not a bar-by-bar rule; its published FLD is computed as fld)"),
];


/// (id, source, licence/notes), one per entry.
pub const SOURCES: &[(&str, &str, &str)] = &[
    ("ohl3", "Skender Stock.Indicators: Basic Price Transforms (CandlePart.OHL3)", ""),
    ("fld", "J. M. Hurst (1970): Future Line of Demarcation (FLD)", ""),
    ("deli", "MotiveWave: Detrended Ehlers Leading Indicator (DELI)", ""),
    ("optimal_tracking_filter", "Ehlers: Optimal Tracking Filter (ProRealCode / TradingView ports)", ""),
    ("arima_forecast", "Box–Jenkins ARIMA(p, d, 0), Yule–Walker fit (rolling)", ""),
    ("periodogram", "Schuster periodogram (dominant period)", ""),
    ("sharpe_ratio", "Sharpe (1994): Sharpe Ratio; PerformanceAnalytics SharpeRatio.annualized", ""),
    ("sortino_ratio", "Sortino & Price (1994): Sortino Ratio; PerformanceAnalytics SortinoRatio", ""),
    ("downside_deviation", "Target Downside Deviation (Rollinger & Hoffman 2013); PerformanceAnalytics DownsideDeviation", ""),
    ("upside_potential_ratio", "Sortino, van der Meer & Plantinga (1999): Upside Potential Ratio", ""),
    ("omega_ratio", "Keating & Shadwick (2002): Omega Ratio", ""),
    ("value_at_risk", "Value at Risk (historical simulation); PerformanceAnalytics VaR", ""),
    ("martin_ratio", "Peter Martin: Ulcer Performance Index (Martin Ratio)", ""),
    ("cycle_lines", "StockCharts ChartNotes: Cycle Lines, https://help.stockcharts.com/charts-and-tools/sharpcharts/chartnotes/line-studies", ""),
    ("hurst_cycles", "J. M. Hurst (1970): Hurst cycle analysis", ""),
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lab::testdata::{assert_series, closes, compiles, from_ohlc, go};
    const N: f64 = f64::NAN;

    #[test] fn ohl3_and_fld() {
        assert_series(&go("ohl3", &from_ohlc(&[(1., 4., 1., 2.), (3., 6., 0., 5.)]), &[])[0], &[2., 3.]);
        let b = closes(&[1., 2., 3., 4., 5.]);
        assert_series(&go("fld", &b, &[("cycle", 4.)])[0], &[N, N, 1., 2., 3.]);
        assert_series(&go("fld", &b, &[("cycle", 5.)])[0], &[N, N, 1., 2., 3.]);
    }
    #[test] fn deli_uses_the_two_bar_range() {
        // bar 1: range 12/8 → price 10, seeds; bar 2: range 12/9 → 10.5; α = 0.5
        let b = from_ohlc(&[(9., 10., 8., 9.), (10., 12., 9., 11.), (10., 11., 10., 10.5)]);
        let o = go("deli", &b, &[("period", 3.)]);
        assert_series(&o[0], &[N, 0., 0.125]);
        assert_series(&o[1], &[N, 0., 0.0625]);
    }
    #[test] fn otf_alpha_from_tracking_index() {
        // hl2 10 → 12, v1 = 0.4, v2 = 0.2, λ = 2 → α = (√80 − 4)/8 = (√5 − 1)/2
        let b = from_ohlc(&[(10., 11., 9., 10.), (12., 13., 11., 12.)]);
        assert_series(&go("optimal_tracking_filter", &b, &[])[0], &[10., 10. + 5f64.sqrt() - 1.]);
    }
    #[test] fn arima_yule_walker() {
        // d = 0, p = 1 on 1,2,3,4: μ 2.5, r0 1.25, r1 0.3125 → φ 0.25, ŷ = 2.5 + 0.25·1.5
        let b = closes(&[1., 2., 3., 4.]);
        assert_series(&go("arima_forecast", &b, &[("p", 1.), ("d", 0.), ("length", 4.)])[0], &[N, N, N, 2.875]);
        // d = 1: constant differences → φ 0, ŷ = mean difference 1
        assert_series(&go("arima_forecast", &b, &[("p", 1.), ("d", 1.), ("length", 3.)])[0], &[N, N, N, 5.]);
    }
    #[test] fn periodogram_finds_a_sine() {
        let b = closes(&(0..40).map(|j| 100. + 5. * (std::f64::consts::TAU * j as f64 / 10.).sin()).collect::<Vec<_>>());
        let o = go("periodogram", &b, &[("length", 40.), ("min_period", 8.), ("max_period", 20.)]);
        assert!(o[0][..39].iter().all(|v| v.is_nan()));
        assert_eq!(o[0][39], 10.);
    }
    #[test] fn return_ratios_by_hand() {
        // returns +10 %, −10 %, +10 %; per-bar values (ppy 1)
        let b = closes(&[100., 110., 99., 108.9]);
        let k = [("length", 3.), ("ppy", 1.)];
        assert_series(&go("sharpe_ratio", &b, &k)[0], &[N, N, N, 3f64.sqrt() / 6.]);
        assert_series(&go("sortino_ratio", &b, &k)[0], &[N, N, N, 1. / 3f64.sqrt()]);
        assert_series(&go("downside_deviation", &b, &k)[0], &[N, N, N, 10. / 3f64.sqrt()]);
        assert_series(&go("upside_potential_ratio", &b, &[("length", 3.)])[0], &[N, N, N, 2. / 3f64.sqrt()]);
        assert_series(&go("omega_ratio", &b, &[("length", 3.)])[0], &[N, N, N, 2.]);
        // sorted −.1, .1, .1: 5 % quantile at h = 0.1 → −0.08
        assert_series(&go("value_at_risk", &b, &[("length", 3.)])[0], &[N, N, N, 8.]);
        // ppy = length: annualised return = total return 8.9 %; drawdowns 0, −10, −1
        assert_series(&go("martin_ratio", &b, &[("length", 3.), ("ppy", 3.)])[0], &[N, N, N, 8.9 / (101f64 / 3.).sqrt()]);
    }
    #[test] fn rules_reach_gap_f() {
        for (id, out) in [("ohl3", "ohl3"), ("fld", "fld"), ("deli", "deli"), ("optimal_tracking_filter", "otf"), ("arima_forecast", "forecast"),
            ("periodogram", "period"), ("sharpe_ratio", "sharpe"), ("sortino_ratio", "sortino"), ("downside_deviation", "tdd"),
            ("upside_potential_ratio", "upr"), ("omega_ratio", "omega"), ("value_at_risk", "var"), ("martin_ratio", "martin")] {
            compiles(id, out).unwrap();
        }
    }
    #[test] fn every_entry_has_one_source() {
        for d in DEFS { assert_eq!(SOURCES.iter().filter(|s| s.0 == d.id).count(), 1, "{}", d.id); }
        assert_eq!(SOURCES.len(), DEFS.len());
    }
}
