//! Census 2 group "c" (2026-10-27): indicators from trading_indicator_census.html that the lab did not have.
//! Formulas: see each definition string. Listed-only entries carry "not available yet — needs …" (data the lab does not hold).
//! Skipped, not indicators or already here: Ermanometry (a drawing tool), Hurst Cycles (a cycle-analysis method; hurst and
//! hurst_bands are the lab's indicators), ARIMA (a fitted forecasting model), Drunkard's Walk (Ehlers' market model; the
//! article's indicator is ehlers_autocorrelation), Periodogram (a generic spectral method; Ehlers' is ehlers_acp).
#![allow(dead_code)]
use super::bars::Bars;
use super::catalogue::{get, int, n, p, real, run, IndicatorDef, Needs, Output, ParamSpec, Params, Placement};
use super::ta::{self, NA};

#[allow(clippy::too_many_arguments)]
const fn def(id: &'static str, name: &'static str, category: &'static str, params: &'static [ParamSpec], outputs: &'static [&'static str], placement: Placement,
             needs: Needs, definition: &'static str, first: &'static [usize], sparse: bool, f: fn(&Bars, &Params) -> Output) -> IndicatorDef {
    IndicatorDef { id, name, category, params, outputs, placement, needs, definition, first, sparse, compute: Some(f), note: "" }
}
/// Listed in the picker, not computed: the data it needs is not admitted (the app shows the note as "data not available").
const fn listed(id: &'static str, name: &'static str, category: &'static str, needs: Needs, definition: &'static str, note: &'static str) -> IndicatorDef {
    IndicatorDef { id, name, category, params: &[], outputs: &["value"], placement: Placement::Pane, needs, definition,
                   first: &[usize::MAX], sparse: false, compute: None, note }
}
const PR: Placement = Placement::Price;
const PN: Placement = Placement::Pane;
const NO: Needs = Needs::NONE;
const V: Needs = Needs { volume: true, ..Needs::NONE };
const SECOND: Needs = Needs { second_symbol: true, ..Needs::NONE };
const INTRADAY_V: Needs = Needs { intraday: true, volume: true, ..Needs::NONE };
const DERIV: Needs = Needs { derivatives: true, ..Needs::NONE };
const FUND: Needs = Needs { fundamentals: true, ..Needs::NONE };
const ST: &str = "Statistics";
const CY: &str = "Cycles";
const VOL: &str = "Volume";
const PF: &str = "Price functions";
const ID: &str = "Intraday & order flow";
const DM: &str = "Derivatives market data";
const SI: &str = "Second instrument";

/// Pearson correlation as Ehlers' listings write it; None when either side has no variance (the listing then keeps the
/// previous value). A variance below 1e-12 of n·Σx² counts as none, so float noise on a flat series is not correlated.
fn pearson(x: impl Iterator<Item = f64>, y: impl Iterator<Item = f64>) -> Option<f64> {
    let (mut m, mut sx, mut sy, mut sxx, mut syy, mut sxy) = (0., 0., 0., 0., 0., 0.);
    for (a, b) in x.zip(y) { m += 1.; sx += a; sy += b; sxx += a * a; syy += b * b; sxy += a * b; }
    let (vx, vy) = (m * sxx - sx * sx, m * syy - sy * sy);
    (vx > 1e-12 * m * sxx && vy > 1e-12 * m * syy).then(|| (m * sxy - sx * sy) / (vx * vy).sqrt())
}

const FDI: &[ParamSpec] = &[int("length", "Length (bars in the window)", 30., 2., 2000.)];
fn fdi(b: &Bars, prm: &Params) -> Output {
    let (k, c) = (n(prm, "length"), &b.close);
    let np = (k - 1) as f64;
    let mut out = vec![NA; c.len()];
    for i in k.saturating_sub(1)..c.len() {
        let w = &c[i + 1 - k..=i];
        if w.iter().any(|v| !v.is_finite()) { continue; }
        let (lo, hi) = w.iter().fold((f64::MAX, f64::MIN), |(a, z), &v| (a.min(v), z.max(v)));
        let r = if hi > lo { hi - lo } else { 1. }; // flat window: y* = 0, L = 1, D = 1
        let len: f64 = w.windows(2).map(|q| (1. / np).hypot((q[1] - q[0]) / r)).sum();
        out[i] = 1. + len.ln() / (2. * np).ln();
    }
    Ok(vec![out])
}

const MMI: &[ParamSpec] = &[int("length", "Length", 300., 2., 2000.), int("source", "Data (0 close, 1 close change)", 0., 0., 1.)];
fn mmi(b: &Bars, prm: &Params) -> Output {
    let k = n(prm, "length");
    let x = if p(prm, "source") == 1. { ta::change(&b.close, 1) } else { b.close.clone() };
    let mut out = vec![NA; x.len()];
    for i in k.saturating_sub(1)..x.len() {
        let w = &x[i + 1 - k..=i];
        if w.iter().any(|v| !v.is_finite()) { continue; }
        let mut s = w.to_vec();
        s.sort_by(|a, z| a.total_cmp(z));
        let m = if k % 2 == 1 { s[k / 2] } else { (s[k / 2 - 1] + s[k / 2]) / 2. };
        // older bar q[0] vs the bar after it q[1]: above the median and falling, or below it and rising
        let hits = w.windows(2).filter(|q| (q[0] > m && q[0] > q[1]) || (q[0] < m && q[0] < q[1])).count();
        out[i] = 100. * hits as f64 / (k - 1) as f64;
    }
    Ok(vec![out])
}

fn true_range_specified_volume(b: &Bars, _: &Params) -> Output {
    let v = b.volume.as_deref().unwrap_or(&[]);
    Ok(vec![ta::tr(b, false).iter().zip(v).map(|(&t, &v)| if t > 0. { v / t } else { NA }).collect()])
}

const TBP: &[ParamSpec] = &[int("period", "Center period", 20., 2., 2000.), real("bandwidth", "Bandwidth", 0.1, 0.01, 1., 0.01), int("length", "Truncation length (bars)", 10., 1., 2000.)];
fn truncated_bandpass(b: &Bars, prm: &Params) -> Output {
    let (per, bw, k, c) = (p(prm, "period"), p(prm, "bandwidth"), n(prm, "length"), &b.close);
    if bw * 360. / per >= 90. { return Err("truncated_bandpass: bandwidth·360/period must be below 90°".into()); }
    let (l1, g1) = ((360. / per).to_radians().cos(), (bw * 360. / per).to_radians().cos());
    let s1 = 1. / g1 - (1. / (g1 * g1) - 1.).sqrt();
    let mut out = vec![NA; c.len()];
    let mut t = vec![0.; k + 3];
    for i in k + 1..c.len() {
        t[k + 2] = 0.; t[k + 1] = 0.;
        // Trunc[j] uses Close[j − 1] and Close[j + 1], j bars back from bar i
        for j in (1..=k).rev() { t[j] = 0.5 * (1. - s1) * (c[i + 1 - j] - c[i - 1 - j]) + l1 * (1. + s1) * t[j + 1] - s1 * t[j + 2]; }
        out[i] = t[1];
    }
    Ok(vec![out])
}

const AC: &[ParamSpec] = &[int("length", "Correlation length", 20., 2., 2000.), int("lag", "Lag (bars)", 10., 0., 2000.)];
fn ehlers_autocorrelation(b: &Bars, prm: &Params) -> Output {
    let (k, lag) = (n(prm, "length"), n(prm, "lag"));
    let us = get("ultimate_smoother").ok_or("ultimate_smoother missing")?;
    let filt = run(&us, b, &[("length".to_string(), 20.)].into())?.swap_remove(0);
    let mut out = vec![NA; filt.len()];
    let mut corr = 0.; // the listing's Corr array starts at 0 and keeps its value on a degenerate window
    for i in lag + k - 1..filt.len() {
        if let Some(r) = pearson((0..k).map(|j| filt[i - j]), (0..k).map(|j| filt[i - lag - j])) { corr = r; }
        out[i] = corr;
    }
    Ok(vec![out])
}

fn ehlers_acp(b: &Bars, _: &Params) -> Output {
    let (c, m) = (&b.close, b.len());
    let w = (360f64 / 48.).to_radians();
    let a = (1. - w.sin()) / w.cos();
    let a1 = (-1.414 * std::f64::consts::PI / 8.).exp();
    let (c2, c3) = (2. * a1 * (1.414 * 180f64 / 8.).to_radians().cos(), -a1 * a1);
    let c1 = 1. - c2 - c3;
    let (mut hp, mut f) = (vec![0.; m], vec![0.; m]);
    for i in 0..m {
        if i >= 1 { hp[i] = 0.5 * (1. + a) * (c[i] - c[i - 1]) + a * hp[i - 1]; }
        let at = |v: &[f64], k: usize| if i >= k { v[i - k] } else { 0. };
        f[i] = c1 * (hp[i] + at(&hp, 1)) / 2. + c2 * at(&f, 1) + c3 * at(&f, 2);
    }
    let mut out = vec![NA; m];
    let (mut corr, mut r, mut dc) = ([0.; 49], [0.; 49], 0.);
    for i in 50..m {
        for (lag, cv) in corr.iter_mut().enumerate() {
            if let Some(v) = pearson((0..3).map(|j| f[i - j]), (0..3).map(|j| f[i - lag - j])) { *cv = v; }
        }
        for (per, rv) in r.iter_mut().enumerate().skip(8) {
            let (mut cs, mut sn) = (0., 0.);
            for (k, cv) in corr.iter().enumerate().skip(3) {
                let ang = (360. * k as f64 / per as f64).to_radians();
                cs += cv * ang.cos(); sn += cv * ang.sin();
            }
            let sq = cs * cs + sn * sn;
            *rv = 0.2 * sq * sq + 0.8 * *rv;
        }
        let max = r[8..].iter().cloned().fold(0., f64::max);
        let pw: Vec<f64> = r[8..].iter().map(|x| if max > 0. { x / max } else { 0. }).collect();
        let peak = pw.iter().cloned().fold(0., f64::max);
        let (mut spx, mut sp) = (0., 0.);
        for (j, &x) in pw.iter().enumerate() { if peak >= 0.25 && x >= 0.25 { spx += (j + 8) as f64 * x; sp += x; } }
        let prev = dc;
        dc = if sp != 0. { spx / sp } else { 0. };
        if sp < 0.25 { dc = prev; }
        if dc < 1. { dc = 1.; }
        out[i] = dc;
    }
    Ok(vec![out])
}

const KF: &[ParamSpec] = &[real("r", "Measurement noise R", 0.005, 0.000001, 100., 0.0001), real("q", "Process noise Q", 0.0005, 0., 100., 0.0001)];
fn kalman_filter(b: &Bars, prm: &Params) -> Output {
    let (r, q) = (p(prm, "r"), p(prm, "q"));
    let (mut e, mut g, mut k) = (0., 1., NA);
    Ok(vec![b.close.iter().map(|&c| {
        if !c.is_finite() { return NA; }
        if k.is_nan() { k = c; }
        e = (1. - g) * e + q;
        g = e / (e + r);
        k += g * (c - k);
        k
    }).collect()])
}

const CAL: &[ParamSpec] = &[int("length", "Length (bars; 756 ≈ 36 months of daily bars)", 756., 2., 5000.), real("periods_per_year", "Bars per year", 252., 1., 400., 1.)];
fn calmar(b: &Bars, prm: &Params) -> Output {
    let (k, ppy, c) = (n(prm, "length"), p(prm, "periods_per_year"), &b.close);
    let mut out = vec![NA; c.len()];
    for i in k..c.len() {
        let w = &c[i - k..=i];
        if w.iter().any(|v| !(v.is_finite() && *v > 0.)) { continue; }
        let mut peak = w[0];
        let mdd = w.iter().fold(0f64, |d, &x| { peak = peak.max(x); d.max((peak - x) / peak) });
        if mdd > 0. { out[i] = ((w[k] / w[0]).powf(ppy / k as f64) - 1.) / mdd; }
    }
    Ok(vec![out])
}

const IR: &[ParamSpec] = &[int("length", "Length (returns)", 252., 2., 5000.)];
fn information_ratio(b: &Bars, prm: &Params) -> Output {
    let s = super::pairs::second(b)?;
    let k = n(prm, "length");
    let ret = |x: &[f64], i: usize| if x[i - 1] != 0. { x[i] / x[i - 1] - 1. } else { NA };
    let d: Vec<f64> = (0..s.len()).map(|i| if i == 0 { NA } else { ret(&b.close, i) - ret(s, i) }).collect();
    let mut out = vec![NA; d.len()];
    for i in k..d.len() {
        let w = &d[i + 1 - k..=i];
        if w.iter().any(|v| !v.is_finite()) { continue; }
        let mean = w.iter().sum::<f64>() / k as f64;
        let sd = (w.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / (k - 1) as f64).sqrt();
        if sd > 0. { out[i] = mean / sd; }
    }
    Ok(vec![out])
}

fn major_gann_levels(b: &Bars, _: &Params) -> Output {
    let (mut hi, mut lo) = (NA, NA);
    let mut g = vec![vec![NA; b.len()]; 4];
    for i in 0..b.len() {
        // running extremes stand in for thinkorswim's all-time High/Low so no bar sees later bars; f64::max/min skip NaN
        hi = hi.max(b.high[i]); lo = lo.min(b.low[i]);
        if hi.is_nan() || lo.is_nan() { continue; }
        let r = hi - lo;
        g[0][i] = 0.5 * hi; g[1][i] = lo + 0.5 * r; g[2][i] = 0.25 * hi; g[3][i] = lo + 0.25 * r;
    }
    Ok(g)
}

const OI_NOTE: &str = "not available yet — needs futures open interest (open interest is not admitted)";
const RF_NOTE: &str = "not available yet — needs a risk-free rate series (and the benchmark as a second instrument)";
const INTRA_NOTE: &str = "not available yet — needs intraday (lower-timeframe/tick) bars";
const NOT_FOUND: &str = "not available yet — formula not published (no primary definition found)";
const SWAMI: &str = "not available yet — formula not published (thinkorswim Swami studies; no code published)";
const VENDOR: &str = "not available yet — formula not published (no canonical definition; vendor scripts differ and none was retrieved)";

pub const DEFS: &[IndicatorDef] = &[
    def("fdi", "Fractal Dimension Index (Sevcik)", ST, FDI, &["fdi"], PN, NO, "Sevcik, 'A procedure to estimate the fractal dimension of waveforms', Complexity International 5 (1998), arXiv:1003.5266, eq. 4-6: over the last N closes (N = length, N' = N − 1) map the window into the unit square, x*_j = j/N', y*_j = (y_j − min)/(max − min); L = Σ_{j=1..N'} √((1/N')² + (y*_j − y*_{j−1})²); fdi = 1 + ln(L)/ln(2·N'); 1 = a straight line, toward 2 = noise; a flat window gives L = 1 and fdi = 1; a missing close in the window gives no value; default length 30 (lab choice; the paper has no default)", &[29], false, fdi),
    def("mmi", "Market Meanness Index", ST, MMI, &["mmi"], PN, NO, "J. C. Lotter, 'The Market Meanness Index', financial-hacker.com (Zorro MMI source): m = median of the last N values (Zorro's Median 'returns their middle value' and leaves even N unstated; here the mean of the two middle values); over the N − 1 consecutive pairs in the window count a bar that is above m with the next bar lower, or below m with the next bar higher (Zorro: Data[i] > m && Data[i] > Data[i−1], Data[i] < m && Data[i] < Data[i−1], Data[0] newest); mmi = 100·count/(N − 1), about 75 for random data, lower when trending; data = close or the close change ('prices or price changes'); a missing value in the window gives no value; default N 300 (a reader's daily example on the page; Lotter leaves it to the user)", &[299], false, mmi),
    def("true_range_specified_volume", "True Range Specified Volume", VOL, &[], &["trsv"], PN, V, "thinkorswim TrueRangeSpecifiedVolume ('calculated as volume divided by TrueRange'; resistance when it is at least 1.5 times its previous value): trsv = volume/TR with TR = max(H, C[1]) − min(L, C[1]); bar 0 has no previous close and no value; TR = 0 gives no value", &[1], false, true_range_specified_volume),
    def("truncated_bandpass", "Ehlers Truncated Bandpass Filter", CY, TBP, &["bpt"], PN, NO, "J. F. Ehlers, 'Truncated Indicators' (TASC July 2020; mesasoftware.com paper, Code Listing 1): L1 = cos(360°/period), G1 = cos(bandwidth·360°/period), S1 = 1/G1 − √(1/G1² − 1); each bar Trunc[length + 2] = Trunc[length + 1] = 0 and for j = length down to 1: Trunc[j] = .5(1 − S1)(close[j − 1] − close[j + 1]) + L1(1 + S1)Trunc[j + 1] − S1·Trunc[j + 2]; bpt = Trunc[1], so only length + 2 closes enter each value (no IIR memory); from bar length + 1; bandwidth·360/period must be below 90°; defaults period 20, bandwidth 0.1, length 10 (the listing's; the untruncated filter is our bandpass)", &[11], false, truncated_bandpass),
    def("ehlers_autocorrelation", "Ehlers Autocorrelation", CY, AC, &["corr"], PN, NO, "J. F. Ehlers, 'The Drunkard's Walk: Theory and Measurement by Autocorrelation' (TASC Feb 2025; mesasoftware.com paper, listing © 2024), Code Listing 1 AutoCorrelation Indicator: Filt = UltimateSmoother(close, 20) (our ultimate_smoother; π where the listings write 3.14159, about 1e-7 relative); for a lag, Pearson correlation of Filt[j] with Filt[lag + j], j = 0..length − 1: (n·Σxy − Σx·Σy)/√((n·Σx² − (Σx)²)(n·Σy² − (Σy)²)); when either variance is not positive (below 1e-12 of n·Σx², float noise) the value is kept from the previous bar (0 before any), as the listing's Corr array; the listing draws every lag 0..99 as a heatmap, here one lag per output (default lag 10 is a lab choice; length 20 as the listing); from bar lag + length − 1", &[29], false, ehlers_autocorrelation),
    def("ehlers_acp", "Ehlers Autocorrelation Periodogram", CY, &[], &["dominant_cycle"], PN, NO, "J. F. Ehlers, 'Measuring Market Cycles' (TASC Sept 2016), TradeStation Traders' Tip listing (AutoLength CCI, periodogram part): roofing filter HP = .5(1 + α)(close − close[1]) + α·HP[1], α = (1 − sin(360°/48))/cos(360°/48), Filt = c1(HP + HP[1])/2 + c2·Filt[1] + c3·Filt[2] (SuperSmoother 8: a1 = e^(−1.414π/8), π where the listing writes 3.14159 (about 1e-7 relative), c2 = 2a1·cos(1.414·180°/8), c3 = −a1², c1 = 1 − c2 − c3), values before bar 0 taken as 0; Corr[lag] = Pearson of Filt[j] with Filt[lag + j], j = 0..2 (AvgLength 3), lag 0..48, kept from the previous bar when degenerate; for period 8..48: SqSum = (Σ_{n=3..48} Corr[n]cos(360n/period))² + (Σ Corr[n]sin(360n/period))², R = .2·SqSum² + .8·R[1]; Pwr = R/max R; dominant_cycle = Σ period·Pwr / Σ Pwr over Pwr ≥ .25 (when the peak ≥ .25), the previous value when that sum < .25, at least 1; the recursions run from bar 0 (TradeStation starts after MaxBarsBack, so early values differ); the optional cubing of Pwr is not offered; from bar 50", &[50], false, ehlers_acp),
    def("kalman_filter", "Ehlers Kalman Filter", "Moving averages", KF, &["kalman"], PR, NO, "J. F. Ehlers, 'The Kalman Filter versus the Alpha-Beta Filter' (mesasoftware.com, 2025), Code Listing 2: EstimateError = (1 − KalmanGain[1])·EstimateError[1] + Q; KalmanGain = EstimateError/(EstimateError + R); kalman = kalman[1] + KalmanGain·(close − kalman[1]); starting values EstimateError 0, KalmanGain 1 and kalman[1] = the first close (the listing's CurrentBar = 1 seed, read as the previous value so the filter starts at the price); a missing close gives no value and leaves the state; defaults R 0.005, Q 0.0005 (the listing's)", &[0], false, kalman_filter),
    def("calmar", "Calmar Ratio", ST, CAL, &["calmar"], PN, NO, "T. W. Young, 'Calmar Ratio: A Smoother Tool', Futures (October 1991): compound annual return over the last 36 months divided by the maximum drawdown over the same period; here on closes: over the bars i − length .. i, cagr = (close/close[length])^(periods_per_year/length) − 1 and mdd = the largest fall from a running peak, (peak − close)/peak, within the window; calmar = cagr/mdd; Young used month-end values, daily closes give deeper drawdowns (declared); no drawdown or a non-positive close in the window gives no value; defaults 756 bars (36 × 21) and 252 bars per year", &[756], false, calmar),
    def("information_ratio", "Information Ratio", SI, IR, &["ir"], PN, SECOND, "W. F. Sharpe, 'The Sharpe Ratio', Journal of Portfolio Management (Fall 1994), ex-post ratio against a benchmark (the information ratio): D_t = R_t − R_Bt with simple returns of this close and the aligned second-instrument close; ir = mean(D)/σ_D over the last length returns, σ_D the sample standard deviation (divisor length − 1); per bar, not annualised; zero σ_D or a missing return in the window gives no value; default length 252 (lab choice)", &[252], false, information_ratio),
    listed("herrick_payoff_index", "Herrick Payoff Index", DM, DERIV, "Herrick Payoff Index (J. Herrick): volume, price change and the change in open interest", OI_NOTE),
    listed("on_balance_open_interest", "On Balance Open Interest", DM, DERIV, "On Balance Open Interest: OBV's running sum applied to open interest", OI_NOTE),
    listed("oiwap", "Open Interest Weighted Average Price", DM, DERIV, "Open Interest Weighted Average Price: price weighted by open interest", OI_NOTE),
    listed("margin_zones", "Margin Zones", DM, DERIV, "Margin Zones: price zones spaced by the exchange's futures margin requirement", "not available yet — needs exchange futures margin requirements (not admitted)"),
    listed("jensens_alpha", "Jensen's Alpha", SI, SECOND, "Jensen's alpha: R_p − [R_f + β(R_m − R_f)], which needs the risk-free rate R_f and a market benchmark R_m", RF_NOTE),
    listed("treynor_ratio", "Treynor Ratio", SI, SECOND, "Treynor ratio: (R_p − R_f)/β with β against a market benchmark, which needs the risk-free rate R_f", RF_NOTE),
    listed("projected_aggregate_volume", "Projected Aggregate Volume", ID, INTRADAY_V, "Projected Aggregate Volume: the session's volume so far projected to the close from the intraday volume profile", INTRA_NOTE),
    listed("projected_volume_at_time", "Projected Volume at Time", ID, INTRADAY_V, "Projected Volume at Time: volume at this time of day projected from past sessions' volume at the same time", INTRA_NOTE),
    listed("cumulative_daily_volume", "Cumulative Daily Volume", ID, INTRADAY_V, "Cumulative Daily Volume: the running volume within each session", INTRA_NOTE),
    listed("swami_intraday_volume", "Swami Intraday Volume", ID, INTRADAY_V, "Swami chart (heatmap over lengths) of intraday volume", INTRA_NOTE),
    listed("float_analysis", "Float Analysis", "Fundamentals", FUND, "Float Analysis: cumulative volume measured against the share float (float turnover)", "not available yet — needs share float (shares outstanding/free float) data"),
    listed("planetary_lines", "Planetary Lines", CY, NO, "Planetary Lines: price levels from planetary longitudes", "not available yet — needs a planetary ephemeris (the lab holds only moon phases)"),
    listed("better_volume", "Better Volume", VOL, NO, "Better Volume (emini-watch.com, B. Taylor): volume bars coloured climax/churn/low volume", "not available yet — formula not published (emini-watch.com vendor code; it was not retrieved and its versions differ)"),
    listed("swami_accumulation_distribution", "Swami Accumulation/Distribution", VOL, NO, "Swami chart (heatmap over lengths) of accumulation/distribution", SWAMI),
    listed("swami_volume", "Swami Volume", VOL, NO, "Swami chart (heatmap over lengths) of volume", SWAMI),
    listed("swami_convolution", "Swami Convolution", CY, NO, "Swami chart (heatmap over lengths) of Ehlers' convolution", SWAMI),
    listed("swami_market_mode", "Swami Market Mode", CY, NO, "Swami chart (heatmap over lengths) of the market mode (trend/cycle)", SWAMI),
    listed("swami_predict", "Swami Predict", CY, NO, "Swami chart (heatmap over lengths) of a predictive filter", SWAMI),
    listed("approval_flow_index", "Approval Flow Index", VOL, NO, "Approval Flow Index (StockSharp): price change related to volume, summed over a length", "not available yet — formula not published (StockSharp describes it in words only)"),
    listed("capital_weighted_volume", "Capital Weighted Volume", VOL, NO, "Capital Weighted Volume", NOT_FOUND),
    listed("swing_price_volume", "Swing Price Volume Indicator", VOL, NO, "Swing Price Volume Indicator", NOT_FOUND),
    listed("vpmc", "Volume Price Momentum Confirmation Indicator", VOL, NO, "Volume Price Momentum Confirmation Indicator", NOT_FOUND),
    listed("barros_swing_average_volume", "Barros Swing Average Volume", VOL, NO, "Barros Swing Average Volume (R. Barros): average volume of each Barros swing", "not available yet — formula not published (Barros swing rules not retrieved)"),
    listed("barros_swings", "Barros Swings", PF, NO, "Barros Swings (R. Barros): swing points", "not available yet — formula not published (Barros swing rules not retrieved)"),
    listed("dvan_buy_sell_pressure", "DVAN Buy/Sell Pressure", VOL, NO, "DVAN Buy/Sell Pressure (proprietary DVAN study)", "not available yet — formula not published (DVAN, proprietary)"),
    listed("dvan_short_term_cycle", "DVAN Short Term Cycle", CY, NO, "DVAN Short Term Cycle (proprietary DVAN study)", "not available yet — formula not published (DVAN, proprietary)"),
    listed("persons_pivots", "Persons Pivots", PF, NO, "John Person's pivots (thinkorswim PersonsPivots): S1/R2 in a bullish, S2/R1 in a bearish, S1/R1 in a neutral market, set by comparing the pivot with its three-day average", "not available yet — formula not published (thinkorswim gives no formula or threshold; J. Person's book was not retrieved)"),
    listed("regression_divergence", "Regression Divergence", SI, NO, "Regression Divergence (M. Katsanos; thinkorswim): linear regression of two symbols' rates of change forecasts the price, minus the actual price, scaled 0-100", "not available yet — formula not published (thinkorswim gives words only; Katsanos' Intermarket Trading Strategies was not retrieved)"),
    def("major_gann_levels", "Major Gann Levels", PF, &[], &["g1", "g2", "g3", "g4"], PR, NO, "thinkorswim MajorGannLevels (studies library: G1 '50% retracement from the all-time High to zero', G2 '50% retracement from the all-time High to the all-time Low', G3 '75% retracement from the all-time High to zero', G4 '75% retracement from the all-time High to the all-time Low', i.e. 'a quarter of the all-time High-Low range added to the all time Low'): with H = the highest high and L = the lowest low of bars 0..i, g1 = H/2, g2 = L + (H − L)/2, g3 = H/4, g4 = L + (H − L)/4; the all-time extremes are taken up to the current bar (declared: a whole-chart High/Low would read later bars); from bar 0", &[0, 0, 0, 0], false, major_gann_levels),
    listed("ma_support_resistance", "Moving Average Support/Resistance", PF, NO, "Moving Average Support/Resistance", NOT_FOUND),
    listed("order_blocks", "Order Blocks", PF, NO, "Order Blocks (smart-money concepts)", VENDOR),
    listed("supply_demand_zones", "Supply and Demand Zones", PF, NO, "Supply and Demand Zones", VENDOR),
    listed("raman_sr_bands", "Raman Support & Resistance Bands", PF, NO, "Raman Support & Resistance Bands", NOT_FOUND),
    listed("volatility_based_sr", "Volatility-Based Support and Resistance", PF, NO, "Volatility-Based Support and Resistance", NOT_FOUND),
    listed("hurst_inverse_ma", "Hurst Inverse Moving Average", CY, NO, "Hurst Inverse Moving Average", NOT_FOUND),
    listed("noise_rate", "Noise Rate Indicator", ST, NO, "Noise Rate Indicator", NOT_FOUND),
    listed("median_average_adaptive_filter", "Median-Average Adaptive Filter", "Moving averages", NO, "Ehlers Median-Average Adaptive Filter: an EMA whose length shrinks until it agrees with the median", "not available yet — formula not published (Ehlers' listing was not retrieved)"),
    listed("ehlers_snr", "Signal to Noise Ratio (Ehlers)", CY, NO, "Ehlers Signal to Noise Ratio: Hilbert-transform cycle power over the bar range, in dB", "not available yet — formula not published (Ehlers, Rocket Science for Traders 2001; the listing was not retrieved)"),
];
/// (id, platform names, AmiBroker names): one row per id in DEFS.
pub const SOURCES: &[(&str, &str, &str)] = &[
    ("fdi", "Sevcik (1998): fractal dimension of waveforms; TradingView (community): Fractal Dimension Index", ""),
    ("mmi", "Zorro / Financial Hacker: Market Meanness Index (MMI)", ""),
    ("true_range_specified_volume", "thinkorswim: TrueRangeSpecifiedVolume", ""),
    ("truncated_bandpass", "Ehlers (TASC July 2020): Truncated Bandpass Filter", ""),
    ("ehlers_autocorrelation", "Ehlers (The Drunkard's Walk, 2024): AutoCorrelation Indicator", ""),
    ("ehlers_acp", "Ehlers (TASC Sept 2016): Autocorrelation Periodogram", ""),
    ("kalman_filter", "Ehlers (2025): Kalman Filter", ""),
    ("calmar", "Young (1991): Calmar Ratio", ""),
    ("information_ratio", "Sharpe (1994): Information Ratio (benchmark-relative Sharpe ratio)", ""),
    ("herrick_payoff_index", "TradingView / MetaStock: Herrick Payoff Index", ""),
    ("on_balance_open_interest", "On Balance Open Interest", ""),
    ("oiwap", "Open Interest Weighted Average Price (OIWAP)", ""),
    ("margin_zones", "Margin Zones", ""),
    ("jensens_alpha", "Jensen's Alpha", ""),
    ("treynor_ratio", "Treynor Ratio", ""),
    ("projected_aggregate_volume", "Projected Aggregate Volume (PAV)", ""),
    ("projected_volume_at_time", "Projected Volume at Time (PVAT)", ""),
    ("cumulative_daily_volume", "Cumulative Daily Volume", ""),
    ("swami_intraday_volume", "thinkorswim: Swami Intraday Volume", ""),
    ("float_analysis", "Float Analysis", ""),
    ("planetary_lines", "Planetary Lines", ""),
    ("better_volume", "emini-watch: Better Volume", ""),
    ("swami_accumulation_distribution", "thinkorswim: Swami Accumulation/Distribution", ""),
    ("swami_volume", "thinkorswim: Swami Volume", ""),
    ("swami_convolution", "thinkorswim: Swami Convolution", ""),
    ("swami_market_mode", "thinkorswim: Swami Market Mode", ""),
    ("swami_predict", "thinkorswim: Swami Predict", ""),
    ("approval_flow_index", "StockSharp: Approval Flow Index (AFI)", ""),
    ("capital_weighted_volume", "Capital Weighted Volume", ""),
    ("swing_price_volume", "Swing Price Volume Indicator", ""),
    ("vpmc", "Volume Price Momentum Confirmation Indicator", ""),
    ("barros_swing_average_volume", "Barros Swing Average Volume", ""),
    ("barros_swings", "Barros Swings", ""),
    ("dvan_buy_sell_pressure", "DVAN: Buy/Sell Pressure", ""),
    ("dvan_short_term_cycle", "DVAN: Short Term Cycle", ""),
    ("persons_pivots", "thinkorswim: PersonsPivots", ""),
    ("regression_divergence", "thinkorswim: RegressionDivergence", ""),
    ("major_gann_levels", "thinkorswim: MajorGannLevels", ""),
    ("ma_support_resistance", "Moving Average Support/Resistance", ""),
    ("order_blocks", "Order Blocks", ""),
    ("supply_demand_zones", "Supply and Demand Zones", ""),
    ("raman_sr_bands", "Raman Support & Resistance Bands", ""),
    ("volatility_based_sr", "Volatility-Based Support and Resistance", ""),
    ("hurst_inverse_ma", "Hurst Inverse Moving Average", ""),
    ("noise_rate", "Noise Rate Indicator", ""),
    ("median_average_adaptive_filter", "Ehlers: Median-Average Adaptive Filter", ""),
    ("ehlers_snr", "Ehlers: Signal to Noise Ratio", ""),
];

#[cfg(test)]
mod tests {
    use crate::lab::catalogue::{get, run, Params};
    use crate::lab::testdata::{assert_series, close_eq, closes, compiles, cut, d1, go, walk};
    const N: f64 = f64::NAN;

    #[test] fn pearson_matches_hand_sums() {
        // x 1,2,3 y 1,3,2: n 3, Σx = Σy = 6, Σx² = Σy² = 14, Σxy = 13 → (39 − 36)/√(6·6) = 0.5
        assert!(close_eq(super::pearson([1., 2., 3.].into_iter(), [1., 3., 2.].into_iter()).unwrap(), 0.5));
        assert!(super::pearson([2., 2.].into_iter(), [1., 3.].into_iter()).is_none());
    }
    #[test] fn fdi_is_sevcik_on_the_window() {
        // 0,1,0 and 1,0,1: N' 2, steps (1/2, ±1) → L = √5, D = 1 + ln√5/ln4; 0,1,2 a line: L = √2 → 1.25
        let d = 1. + 5f64.sqrt().ln() / 4f64.ln();
        assert_series(&go("fdi", &closes(&[0., 1., 0., 1., 2.]), &[("length", 3.)])[0], &[N, N, d, d, 1.25]);
        // 2,3,3: y* 0,1,1 → L = √1.25 + 0.5; flat → 1
        let e = 1. + (1.25f64.sqrt() + 0.5).ln() / 4f64.ln();
        assert_series(&go("fdi", &closes(&[1., 2., 3., 3., 3., 3.]), &[("length", 3.)])[0], &[N, N, 1.25, e, 1., 1.]);
    }
    #[test] fn mmi_counts_reverting_pairs() {
        // 1,3,2,4,3: median 3; 1→3 below and rising, 3→2 at the median, 2→4 below and rising, 4→3 above and falling → 3/4
        assert_series(&go("mmi", &closes(&[1., 3., 2., 4., 3.]), &[("length", 5.)])[0], &[N, N, N, N, 75.]);
        // 1,2,3,4: median 2.5; 1→2 and 2→3 below and rising, 3→4 above and rising → 2/3
        assert_series(&go("mmi", &closes(&[1., 2., 3., 4.]), &[("length", 4.)])[0], &[N, N, N, 200. / 3.]);
        // changes N,1,1,1: the first window holds a missing value; then all equal the median → 0
        assert_series(&go("mmi", &closes(&[1., 2., 3., 4.]), &[("length", 3.), ("source", 1.)])[0], &[N, N, N, 0.]);
    }
    #[test] fn trsv_is_volume_over_true_range() {
        // d1 bar 1: H 12, L 10, C[1] 10.5 → TR 2, V 120; bar 2: H 12.5, L 11, C[1] 11.5 → 1.5, V 90
        assert_series(&go("true_range_specified_volume", &d1(), &[])[0][..3], &[N, 60., 60.]);
        assert_eq!(run(&get("true_range_specified_volume").unwrap(), &walk(5, 1, false), &Params::new()).unwrap_err(), "needs volume");
    }
    #[test] fn truncated_bandpass_uses_only_length_plus_two_closes() {
        let (l1, g1) = (18f64.to_radians().cos(), 1.8f64.to_radians().cos());
        let s1 = 1. / g1 - (1. / (g1 * g1) - 1.).sqrt();
        let c = [1., 4., 2., 8., 5.];
        // length 2 on bar i: T2 = .5(1 − S1)(c[i−1] − c[i−3]), T1 = .5(1 − S1)(c[i] − c[i−2]) + L1(1 + S1)T2
        let t = |i: usize| { let t2 = 0.5 * (1. - s1) * (c[i - 1] - c[i - 3]); 0.5 * (1. - s1) * (c[i] - c[i - 2]) + l1 * (1. + s1) * t2 };
        assert_series(&go("truncated_bandpass", &closes(&c), &[("length", 2.)])[0], &[N, N, N, t(3), t(4)]);
        assert!(run(&get("truncated_bandpass").unwrap(), &closes(&c), &[("bandwidth".to_string(), 1.), ("period".to_string(), 4.)].into()).is_err());
    }
    #[test] fn autocorrelation_of_a_line_is_one_and_flat_keeps_zero() {
        // the UltimateSmoother passes a straight line exactly (its second difference is 0), so every lag correlates +1
        let ramp: Vec<f64> = (0..40).map(f64::from).collect();
        let o = go("ehlers_autocorrelation", &closes(&ramp), &[("length", 3.), ("lag", 5.)]);
        assert!(o[0][..7].iter().all(|v| v.is_nan()) && o[0][7..].iter().all(|v| close_eq(*v, 1.)));
        let f = go("ehlers_autocorrelation", &closes(&[5.; 40]), &[]);
        assert!(f[0][..29].iter().all(|v| v.is_nan()) && f[0][29..].iter().all(|v| *v == 0.));
    }
    #[test] fn acp_finds_the_cycle_and_holds_one_when_flat() {
        assert!(go("ehlers_acp", &closes(&[100.; 60]), &[])[0][50..].iter().all(|v| *v == 1.));
        let wave: Vec<f64> = (0..400).map(|i| 100. + (2. * std::f64::consts::PI * i as f64 / 20.).sin()).collect();
        let dc = go("ehlers_acp", &closes(&wave), &[])[0][399];
        assert!((dc - 20.).abs() < 2., "dominant cycle {dc}");
    }
    #[test] fn kalman_follows_the_listing() {
        // R = Q = 1: E 1, K .5, k 10; E 1.5, K .6, k 10 + .6·10 = 16; E .4·1.5 + 1 = 1.6, K 8/13, close 16 → 16
        assert_series(&go("kalman_filter", &closes(&[10., 20., 16.]), &[("r", 1.), ("q", 1.)])[0], &[10., 16., 16.]);
    }
    #[test] fn calmar_is_cagr_over_window_drawdown() {
        // length 2, 2 bars a year: 100, 80, 120 → cagr 1.2 − 1 = .2, mdd .2 → 1; 80, 120, 132 has no drawdown → no value
        assert_series(&go("calmar", &closes(&[100., 80., 120., 132.]), &[("length", 2.), ("periods_per_year", 2.)])[0], &[N, N, 1., N]);
        // 100, 90, 99 with 1 bar a year: cagr .99^(1/2) − 1, mdd .1
        assert_series(&go("calmar", &closes(&[100., 90., 99.]), &[("length", 2.), ("periods_per_year", 1.)])[0], &[N, N, (0.99f64.sqrt() - 1.) / 0.1]);
    }
    fn pair(a: &[f64], b: &[f64]) -> crate::lab::bars::Bars { let mut x = closes(a); x.second = Some(b.to_vec()); x }
    #[test] fn information_ratio_is_mean_over_sample_sd_of_active_returns() {
        // returns +10 %, −10 %, +10 % vs +5 %, 0, +5 %: D = .05, −.10, .05; windows of 2: mean −.025, sd .075√2
        let b = pair(&[100., 110., 99., 108.9], &[100., 105., 105., 110.25]);
        let v = -0.025 / (0.075 * 2f64.sqrt());
        assert_series(&go("information_ratio", &b, &[("length", 2.)])[0], &[N, N, v, v]);
        let d = get("information_ratio").unwrap();
        assert_eq!(run(&d, &walk(30, 1, false), &Params::new()).unwrap_err(), "needs a second instrument");
        let mut full = walk(400, 2, false);
        full.second = Some(walk(400, 3, false).close);
        let want = run(&d, &full, &Params::new()).unwrap();
        assert_eq!(want[0].iter().position(|v| !v.is_nan()), Some(d.first[0]));
        assert!(want[0][d.first[0]..].iter().all(|v| v.is_finite()));
        for c in [260usize, 333, 399] {
            let mut pre = cut(&full, c);
            pre.second = Some(full.second.as_ref().unwrap()[..c].to_vec());
            let got = run(&d, &pre, &Params::new()).unwrap();
            for i in 0..c { assert!(close_eq(got[0][i], want[0][i]), "bar {i}"); }
        }
    }
    #[test] fn calmar_warms_up_at_its_length_and_is_causal() {
        // the catalogue's causality test uses 400 bars, shorter than calmar's 756: check it here on a short length
        let full = walk(300, 4, false);
        let want = go("calmar", &full, &[("length", 60.)]);
        assert_eq!(want[0].iter().position(|v| !v.is_nan()), Some(60));
        for c in [61usize, 150, 299] {
            let got = go("calmar", &cut(&full, c), &[("length", 60.)]);
            for i in 0..c { assert!(close_eq(got[0][i], want[0][i]), "bar {i}"); }
        }
    }
    #[test] fn gann_levels_use_running_extremes() {
        // d1 bar 0: H 11, L 9 → 5.5, 10, 2.75, 9.5; bar 2: H 12.5, L 9 → 6.25, 10.75, 3.125, 9.875
        let g = go("major_gann_levels", &crate::lab::testdata::d1(), &[]);
        let want = [[5.5, 10., 2.75, 9.5], [6., 10.5, 3., 9.75], [6.25, 10.75, 3.125, 9.875]];
        for (i, w) in want.iter().enumerate() { for k in 0..4 { assert!(close_eq(g[k][i], w[k]), "g{} bar {i}", k + 1); } }
    }
    #[test] fn listed_entries_say_why() {
        for d in super::DEFS.iter().filter(|d| d.compute.is_none()) {
            assert!(d.note.starts_with("not available yet — needs ") || d.note.starts_with("not available yet — formula not published ("), "{}", d.id);
            assert!(run(d, &walk(10, 1, true), &Params::new()).is_err(), "{}", d.id);
        }
    }
    #[test] fn rules_can_read_every_output() {
        for d in super::DEFS.iter().filter(|d| d.compute.is_some() && !d.needs.second_symbol) {
            for o in d.outputs { compiles(d.id, o).unwrap_or_else(|e| panic!("{}.{o}: {e:?}", d.id)); }
        }
    }
    /// CENSUS_BARS=<csv date,open,high,low,close[,volume[,second close]]> CENSUS_OUT=<json> cargo test --bin guru
    /// lab::x_census2_c::tests::census_c_dump -- --ignored --exact; also asserts every output is causal at 15 cut points.
    #[test] #[ignore] fn census_c_dump() {
        let text = std::fs::read_to_string(std::env::var("CENSUS_BARS").expect("CENSUS_BARS")).expect("read CENSUS_BARS");
        let mut b = crate::lab::bars::Bars::default();
        let (mut vol, mut sec) = (vec![], vec![]);
        for (i, line) in text.lines().skip(1).filter(|s| !s.trim().is_empty()).enumerate() {
            let f: Vec<&str> = line.split(',').map(str::trim).collect();
            let x = |k: usize| f.get(k).and_then(|s| s.parse::<f64>().ok()).unwrap_or(f64::NAN);
            b.date.push(f[0].to_string());
            b.open.push(x(1)); b.high.push(x(2)); b.low.push(x(3)); b.close.push(x(4)); b.end.push(i);
            if f.len() > 5 { vol.push(x(5)); }
            if f.len() > 6 { sec.push(x(6)); }
        }
        if vol.len() == b.len() { b.volume = Some(vol); }
        if sec.len() == b.len() { b.second = Some(sec); }
        let same = |a: f64, z: f64| a.to_bits() == z.to_bits() || (a.is_nan() && z.is_nan());
        let num = |v: f64| if v.is_finite() { format!("{v:?}") } else { "null".into() };
        let body: Vec<String> = super::DEFS.iter().filter(|d| d.compute.is_some()).map(|d| {
            let out = run(d, &b, &Params::new()).unwrap_or_else(|e| panic!("{}: {e}", d.id));
            for k in (1..=15).map(|j| j * b.len() / 15) {
                let mut part = cut(&b, k);
                if let Some(s) = part.second.as_mut() { s.truncate(k); }
                let got = run(d, &part, &Params::new()).unwrap();
                for (f, g) in out.iter().zip(&got) { assert!(g.iter().zip(&f[..k]).all(|(a, z)| same(*a, *z)), "{} not causal at cut {k}", d.id); }
            }
            let cols: Vec<String> = d.outputs.iter().zip(&out).map(|(k, s)| format!("\"{k}\":[{}]", s.iter().map(|&v| num(v)).collect::<Vec<_>>().join(","))).collect();
            format!("\"{}\":{{{}}}", d.id, cols.join(","))
        }).collect();
        std::fs::write(std::env::var("CENSUS_OUT").expect("CENSUS_OUT"), format!("{{{}}}", body.join(","))).expect("write CENSUS_OUT");
    }
}
