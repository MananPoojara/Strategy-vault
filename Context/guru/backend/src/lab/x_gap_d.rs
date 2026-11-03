//! Gap group "d" (2026-11-02 cross-platform survey, Volume & Money Flow, order-flow and volume-chart items our software lacked).
//! Computed: Rolling VWAP with bands (Sierra Chart), EquiVolume / CandleVolume / Arms CandleVolume as bar-aligned box series,
//! Sierra Chart's Bid Ask Volume Ratio (from the order-flow footprint), Finite Volume Elements (Katsanos), Multi Vote OBV,
//! Holt's NVDI and OBVDI, Slow Volume Strength Index (Apirine), TFS Volume Oscillator, Trend Thrust Indicator (Dormeier),
//! Volume Momentum (FXCodeBase), VPCI (Dormeier) and Volume RSI. Listed only: Time and Sales and the Ergodic Tick Volume
//! Indicator (both need tick data), Triple Brick and Triple XO (no published rule found). Not added, already in the catalogue:
//! Volume Accumulation Percent (MotiveWave / Omega Research: 100·Σ(V·((C − L) − (H − C))/(H − L), n)/Σ(V, n), H = L → 0, is
//! `iii` output ii_pct) and Order Flow Trading (a method, not one indicator: the Order flow family volume_delta, cvd,
//! delta_percent, footprint_imbalance, volume_profile). Formulas: see each definition string.
use super::bars::Bars;
use super::catalogue::{int, len, n, p, real, IndicatorDef, Needs, Output, ParamSpec, Params, Placement};
use super::ta::{self, NA};

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
const VOL: Needs = Needs { volume: true, ..Needs::NONE };
const OF: Needs = Needs { order_flow: true, ..Needs::NONE };
const TICK: Needs = Needs { intraday: true, ..Needs::NONE };
const NO: Needs = Needs::NONE;
const PR: Placement = Placement::Price;
const PN: Placement = Placement::Pane;
const V: &str = "Volume";
const PF: &str = "Price functions";

fn vol(b: &Bars) -> &[f64] { b.volume.as_deref().unwrap_or(&[]) }
fn div(a: f64, b: f64) -> f64 { if b == 0. || a.is_nan() || b.is_nan() { NA } else { a / b } }
fn zip(a: &[f64], b: &[f64], f: impl Fn(f64, f64) -> f64) -> Vec<f64> { a.iter().zip(b).map(|(&x, &y)| f(x, y)).collect() }
fn sgn(d: f64) -> f64 { if d > 0. { 1. } else if d < 0. { -1. } else { 0. } }
/// Σ x·v / Σ v over bars a..=t (NaN if any x is NaN or Σ v = 0).
fn win(x: &[f64], v: &[f64], a: usize, t: usize) -> f64 {
    let (mut s, mut sv) = (0., 0.);
    for j in a..=t { s += x[j] * v[j]; sv += v[j]; }
    div(s, sv)
}
/// Volume-weighted simple average over the last k bars (TradeStation BD.VWMA / AmiBroker Sum(x·V, k)/Sum(V, k)).
fn vwma(x: &[f64], v: &[f64], k: usize) -> Vec<f64> { (0..x.len()).map(|t| if t + 1 < k { NA } else { win(x, v, t + 1 - k, t) }).collect() }
/// Bollinger %b: (x − (sma − sd·stdev))/(2·sd·stdev), population stdev; zero band width → 0 (MotiveWave).
fn pctb(x: &[f64], k: usize, sd: f64) -> Vec<f64> {
    let (m, s) = (ta::sma(x, k), ta::stdev(x, k));
    (0..x.len()).map(|i| if m[i].is_nan() || s[i].is_nan() { NA } else if s[i] == 0. { 0. } else { (x[i] - (m[i] - sd * s[i])) / (2. * sd * s[i]) }).collect()
}

const RV: &[ParamSpec] = &[len("length", "Length (bars)", 20.), real("mult", "Band multiplier", 1., 0.1, 10., 0.1)];
/// Volume-weighted close over the last `length` bars with volume-weighted standard deviation bands.
fn rolling_vwap(b: &Bars, prm: &Params) -> Output {
    let (k, m, v, c, len) = (n(prm, "length"), p(prm, "mult"), vol(b), &b.close, b.len());
    let mut o = vec![vec![NA; len]; 3];
    for t in (k.max(1) - 1)..len {
        let (mut sv, mut s1, mut s2) = (0., 0., 0.);
        for j in t + 1 - k..=t { sv += v[j]; s1 += c[j] * v[j]; s2 += c[j] * c[j] * v[j]; }
        if sv == 0. { continue; }
        let x = s1 / sv;
        let sd = (s2 / sv - x * x).max(0.).sqrt();
        (o[0][t], o[1][t], o[2][t]) = (x, x + m * sd, x - m * sd);
    }
    Ok(o)
}

const BOX: &[ParamSpec] = &[len("length", "Volume average length (box width)", 14.)];
/// Box width: volume / sma(volume, length) (1 = average volume).
fn width(b: &Bars, prm: &Params) -> Vec<f64> { zip(vol(b), &ta::sma(vol(b), n(prm, "length")), div) }
fn height(b: &Bars) -> Vec<f64> { zip(&b.high, &b.low, |h, l| h - l) }
fn body(b: &Bars) -> Vec<f64> { zip(&b.close, &b.open, |c, o| c - o) }
fn equivolume(b: &Bars, prm: &Params) -> Output { Ok(vec![height(b), width(b, prm)]) }
fn candlevolume(b: &Bars, prm: &Params) -> Output { Ok(vec![body(b), width(b, prm)]) }
fn arms_candlevolume(b: &Bars, prm: &Params) -> Output { Ok(vec![body(b), height(b), width(b, prm)]) }

const BAVR: &[ParamSpec] = &[len("length", "Length", 10.), int("ma_type", "Moving average (0 EMA, 1 SMA)", 0., 0., 1.)];
/// Sierra Chart Bid Ask Volume Ratio from the daily footprint: per bar 100·(ask − bid)/(ask + bid), then its moving average.
fn bid_ask_volume_ratio(b: &Bars, prm: &Params) -> Output {
    let f = b.footprint.as_ref().expect("catalogue::run checks needs.order_flow");
    let vr: Vec<f64> = (0..b.len()).map(|i| match f.day(i) {
        Some(d) => { let (a, s): (f64, f64) = (d.a.iter().sum(), d.b.iter().sum()); if a + s > 0. { 100. * (a - s) / (a + s) } else { 0. } }
        None => NA,
    }).collect();
    let k = n(prm, "length");
    let avg = if n(prm, "ma_type") == 1 { ta::sma(&vr, k) } else { ta::ema(&vr, k) };
    Ok(vec![avg, vr])
}

const FVE: &[ParamSpec] = &[len("length", "FVE period", 22.), real("coef", "Cutoff coefficient", 0.1, 0., 2., 0.01), len("ma_length", "FVE moving average", 30.)];
/// Katsanos' AmiBroker FVE: volume counts +V / −V / 0 when the intra+interday money flow clears a volatility cutoff.
fn fve(b: &Bars, prm: &Params) -> Output {
    let (k, coef, v, len) = (n(prm, "length"), p(prm, "coef"), vol(b), b.len());
    let tp = b.hlc3();
    let intra: Vec<f64> = (0..len).map(|i| b.high[i].ln() - b.low[i].ln()).collect();
    let inter: Vec<f64> = (0..len).map(|i| if i == 0 { NA } else { tp[i].ln() - tp[i - 1].ln() }).collect();
    let (vi, ve) = (ta::stdev(&intra, k), ta::stdev(&inter, k));
    let vc: Vec<f64> = (0..len).map(|i| {
        if i == 0 || vi[i].is_nan() || ve[i].is_nan() { return NA; }
        let cut = coef * (vi[i] + ve[i]) * b.close[i];
        let mf = b.close[i] - (b.high[i] + b.low[i]) / 2. + tp[i] - tp[i - 1];
        if mf > cut { v[i] } else if mf < -cut { -v[i] } else { 0. }
    }).collect();
    let (sv, av) = (ta::sum(&vc, k), ta::sma(v, k));
    let f: Vec<f64> = (0..len).map(|i| 100. * div(sv[i], av[i] * k as f64)).collect();
    let ma = ta::sma(&f, n(prm, "ma_length"));
    Ok(vec![f, ma])
}

/// MotiveWave Multi Vote OnBalance Volume: running Σ volume·(vote of high + vote of low + vote of close).
fn multivote_obv(b: &Bars, _: &Params) -> Output {
    let (len, v) = (b.len(), vol(b));
    let (mut o, mut s) = (vec![NA; len], 0.);
    for i in 1..len {
        s += v[i] * (sgn(b.high[i] - b.high[i - 1]) + sgn(b.low[i] - b.low[i - 1]) + sgn(b.close[i] - b.close[i - 1]));
        o[i] = s;
    }
    Ok(vec![o])
}

const DI: &[ParamSpec] = &[len("length", "Bollinger length", 33.), real("stdevs", "Standard deviations", 2., 0.1, 10., 0.1), len("signal", "Signal average", 4.)];
/// Holt's disparity: (1 + %b(close))/(1 + %b(volume indicator)), and its simple average.
fn disparity(b: &Bars, ind: &[f64], prm: &Params) -> Output {
    let (k, sd) = (n(prm, "length"), p(prm, "stdevs"));
    let (pb, ib) = (pctb(&b.close, k, sd), pctb(ind, k, sd));
    let d = zip(&pb, &ib, |x, y| div(1. + x, 1. + y));
    let s = ta::sma(&d, n(prm, "signal"));
    Ok(vec![d, s])
}
fn obvdi(b: &Bars, prm: &Params) -> Output {
    let (len, v) = (b.len(), vol(b));
    let mut obv = vec![0.; len];
    for i in 1..len { obv[i] = obv[i - 1] + sgn(b.close[i] - b.close[i - 1]) * v[i]; }
    disparity(b, &obv, prm)
}
fn nvdi(b: &Bars, prm: &Params) -> Output {
    let (len, v) = (b.len(), vol(b));
    let mut nv = vec![100.; len];
    for i in 1..len { nv[i] = nv[i - 1] + if v[i] < v[i - 1] { 100. * (b.close[i] - b.close[i - 1]) / b.close[i - 1] } else { 0. }; }
    disparity(b, &nv, prm)
}

const SV: &[ParamSpec] = &[len("ema_length", "EMA length", 6.), len("length", "Smoothing length (Wilder)", 14.)];
/// Apirine SVSI: RSI-style ratio of Wilder-averaged volume on closes above / below ema(close, ema_length).
fn svsi(b: &Bars, prm: &Params) -> Output {
    let (e, k, v, len) = (ta::ema(&b.close, n(prm, "ema_length")), n(prm, "length"), vol(b), b.len());
    let side = |up: bool| -> Vec<f64> { (0..len).map(|i| if e[i].is_nan() { NA } else if (up && b.close[i] > e[i]) || (!up && b.close[i] < e[i]) { v[i] } else { 0. }).collect() };
    let (ap, an) = (ta::rma(&side(true), k), ta::rma(&side(false), k));
    Ok(vec![zip(&ap, &an, |x, y| if x.is_nan() || y.is_nan() { NA } else { 100. - 100. / (1. + if y != 0. { x / y } else { 100. }) })])
}

const TFS: &[ParamSpec] = &[len("length", "Period", 7.)];
/// Strain's TFS Volume Oscillator: Σ(sign(close − open)·volume, n)/n.
fn tfs_volume_osc(b: &Bars, prm: &Params) -> Output {
    let v = vol(b);
    let x: Vec<f64> = (0..b.len()).map(|i| sgn(b.close[i] - b.open[i]) * v[i]).collect();
    Ok(vec![ta::sma(&x, n(prm, "length"))])
}

const TTI: &[ParamSpec] = &[len("fast", "Fast length", 12.), len("slow", "Slow length", 26.), len("signal", "Signal length (W)", 9.)];
/// Dormeier's Trend Thrust Indicator, as in the posted TradeStation code (see the definition for the steps).
fn trend_thrust(b: &Bars, prm: &Params) -> Output {
    let (f, s, w, v, c, len) = (n(prm, "fast"), n(prm, "slow"), n(prm, "signal"), vol(b), &b.close, b.len());
    let (fa, sa) = (vwma(c, v, f), vwma(c, v, s));
    let m = zip(&ta::ema(c, f), &ta::ema(c, s), |x, y| x - y);
    let (vef, ves) = (ta::ema(v, f), ta::ema(v, s));
    let vf = zip(&vwma(v, v, f), &ves, |x, y| 10. * div(x, y));
    let ivf = zip(&vwma(v, v, s), &vef, |x, y| 10. * div(x, y));
    // FAST = fastavg/(vf/ivf), SLOW = (ivf/vf)·slowavg → sprd = (fastavg − slowavg)/(vf/ivf)
    let sprd: Vec<f64> = (0..len).map(|i| div(fa[i] - sa[i], div(vf[i], ivf[i]))).collect();
    let vavg = zip(&vwma(&vf, v, f), &ta::ema(&ivf, s), |x, y| div(x, y) * w as f64);
    let mcad = ta::ema(&m, w);
    let (mut avg, mut diff) = (vec![NA; len], vec![NA; len]);
    if let Some(z) = sprd.iter().position(|x| !x.is_nan()) {
        for t in z..len {
            if vavg[t].is_nan() { continue; }
            // shortcut: BD.VWMA's fractional length runs floor(vavg) bars; clamped to 1..=bars of sprd so far
            let k = (vavg[t].floor().max(1.) as usize).min(t + 1 - z);
            let a = (win(&sprd, v, t + 1 - k, t) * 1000.).round() / 1000.;
            (avg[t], diff[t]) = (a, sprd[t] - a - mcad[t]);
        }
    }
    Ok(vec![sprd, avg, diff])
}

const VM: &[ParamSpec] = &[len("length", "Volume average length", 10.), len("lag", "Bars back (N)", 10.), int("multiply", "Multiply by volume (0 no, 1 yes)", 0., 0., 1.)];
/// FXCodeBase Volume Momentum: sma(V, length) − sma(V, length)[lag], optionally × volume.
fn volume_momentum(b: &Bars, prm: &Params) -> Output {
    let (lag, mult, v) = (n(prm, "lag"), n(prm, "multiply") == 1, vol(b));
    let m = ta::sma(v, n(prm, "length"));
    Ok(vec![(0..b.len()).map(|i| if i < lag { NA } else { let d = m[i] - m[i - lag]; if mult { d * v[i] } else { d } }).collect()])
}

const VP: &[ParamSpec] = &[len("short", "Short period", 5.), len("long", "Long period", 20.), len("smooth", "Smoothing period", 20.)];
/// Dormeier VPCI (AmiBroker Traders' Tips form): (vwma_long − sma_long)·(vwma_short/sma_short)·(sma(V, short)/sma(V, long)).
fn vpci(b: &Bars, prm: &Params) -> Output {
    let (s, l, v, c) = (n(prm, "short"), n(prm, "long"), vol(b), &b.close);
    let vpc = zip(&vwma(c, v, l), &ta::sma(c, l), |x, y| x - y);
    let vpr = zip(&vwma(c, v, s), &ta::sma(c, s), div);
    let vm = zip(&ta::sma(v, s), &ta::sma(v, l), div);
    let x: Vec<f64> = (0..b.len()).map(|i| vpc[i] * vpr[i] * vm[i]).collect();
    let sm = ta::sma(&x, n(prm, "smooth"));
    Ok(vec![x, sm])
}

const VR: &[ParamSpec] = &[len("length", "Length (Wilder)", 14.)];
/// Wilder RSI of the volume series; no gains and no losses → 50.
fn volume_rsi(b: &Bars, prm: &Params) -> Output {
    let (ch, k) = (ta::change(vol(b), 1), n(prm, "length"));
    let up: Vec<f64> = ch.iter().map(|&d| if d.is_nan() { NA } else { d.max(0.) }).collect();
    let dn: Vec<f64> = ch.iter().map(|&d| if d.is_nan() { NA } else { (-d).max(0.) }).collect();
    Ok(vec![zip(&ta::rma(&up, k), &ta::rma(&dn, k), |g, l| if g.is_nan() || l.is_nan() { NA } else if g + l == 0. { 50. } else { 100. * g / (g + l) })])
}

pub const DEFS: &[IndicatorDef] = &[
    def("rolling_vwap", "Rolling VWAP (bars)", V, RV, &["vwap", "upper", "lower"], PR, VOL,
        "Sierra Chart Volume Weighted Average Price - VWAP - Rolling with Standard Deviation Lines, Time Period Type = Bars, Input Data = Last (close), Total Volume. At every bar t the window is the last length bars t − length + 1 ..= t (rolling bar by bar, no reset). vwap = Σ(close·v)/Σv. Band offset = √max(Σ(v·close²)/Σv − vwap², 0), the volume-weighted deviation of close around the window's vwap (Sierra Chart's \"VWAP Variance\" method), and upper/lower = vwap ± mult·offset (band 1; bands 2–4 are 2·, 3·, 4·mult, run again with another mult). Σv = 0 → NaN. Defaults: length 20 (Sierra Chart's page gives no default for Bars), mult 1. Blank before bar length − 1. Adaptation: Sierra Chart writes the variance with VWAP_i inside the sum; here the window's vwap at t is used, as in the lab's vwap_period. The vwap line alone equals vwma (close·volume over volume); this entry adds the rolling bands. The Days and Minutes period types need intraday bars and are not offered. Sources: Sierra Chart, Volume Weighted Average Price - VWAP - Rolling with Standard Deviation Lines (https://www.sierrachart.com/index.php?page=doc/StudiesReference.php&ID=352); Sierra Chart, VWAP Standard Deviation Band Calculation (https://www.sierrachart.com/index.php?page=doc/StudiesReference.php&ID=108).",
        &[19, 19, 19], false, rolling_vwap),
    def("equivolume", "EquiVolume (bar-aligned boxes)", PF, BOX, &["height", "width"], PN, VOL,
        "EquiVolume (Richard Arms; StockCharts chart type): each bar is a box whose height is the price range and whose width is its volume. height = high − low. width = volume/sma(volume, length), so 1 is an average-volume box (Wickra Equivolume). Default length 14. height from bar 0, width from bar length − 1; zero average volume → NaN. Adaptation: StockCharts sizes each box against the volume of the periods shown on the chart, so a box's width changes with the visible range and the chart's end date. Here the width is relative to the trailing length-bar average, which is causal and usable in rules. Sources: StockCharts ChartSchool, Arms CandleVolume and EquiVolume (https://chartschool.stockcharts.com/table-of-contents/chart-analysis/chart-types/arms-candlevolume); Wickra docs v0.7.7, Equivolume (https://4e3995d1.wickra-docs.pages.dev/Indicators/Indicator-Equivolume).",
        &[0, 13], false, equivolume),
    def("candlevolume", "CandleVolume (bar-aligned)", PF, BOX, &["body", "width"], PN, VOL,
        "CandleVolume (StockCharts chart type): a candlestick whose width is proportional to its volume. body = close − open (signed, + bullish). width = volume/sma(volume, length), 1 = average volume (Wickra CandleVolume). Default length 14. body from bar 0, width from bar length − 1; zero average volume → NaN. Adaptation: StockCharts sizes candles against the volume visible on the chart; here the width is relative to the trailing length-bar average (causal). The candle's own open/high/low/close are the bar's prices. Sources: StockCharts, Chip Anderson, Introducing the Arms CandleVolume charts (https://stockcharts.com/articles/chipanderson/2013/10/introducing-the-arms-candlevolume-charts.html: \"CandleVolume charts are candlestick charts where the width of the candle is proportional to its volume\"); Wickra docs v0.7.7, CandleVolume (https://4e3995d1.wickra-docs.pages.dev/Indicators/Indicator-CandleVolume).",
        &[0, 13], false, candlevolume),
    def("arms_candlevolume", "Arms CandleVolume (bar-aligned)", PF, BOX, &["body", "height", "width"], PN, VOL,
        "Arms CandleVolume (StockCharts chart type): a CandleVolume candle drawn inside its EquiVolume box. body = close − open, height = high − low, width = volume/sma(volume, length) (1 = average volume). Default length 14. body and height from bar 0, width from bar length − 1; zero average volume → NaN. Adaptation: StockCharts states that the true width of a box is only shown when the chart ends on that date (widths are relative to the visible volume); here the width is relative to the trailing length-bar average, so it is causal. Sources: StockCharts ChartSchool, Arms CandleVolume (https://chartschool.stockcharts.com/table-of-contents/chart-analysis/chart-types/arms-candlevolume); StockCharts, Chip Anderson, Introducing the Arms CandleVolume charts (https://stockcharts.com/articles/chipanderson/2013/10/introducing-the-arms-candlevolume-charts.html).",
        &[0, 0, 13], false, arms_candlevolume),
    def("bid_ask_volume_ratio", "Bid Ask Volume Ratio", "Order flow", BAVR, &["ratio_avg", "ratio"], PN, OF,
        "Sierra Chart Bid Ask Volume Ratio, Difference Calculation Method = Ask Volume − Bid Volume, from the lab's daily footprint (volume at the ask and at the bid per price level). ratio = 100·(ask − bid)/(ask + bid) for the bar, 0 when ask + bid ≤ 0 (Sierra Chart), NaN on a bar without footprint rows. ratio_avg = its moving average over length (ma_type 0 EMA, Sierra Chart's documented type; 1 SMA); bars without footprint are skipped by the average. Defaults: length 10 (Sierra Chart's page gives no default), EMA. ratio_avg from the length-th bar with footprint. Adaptation: the lab's EMA is seeded with the SMA of the first length values; Sierra Chart starts from VR_0. The Bid − Ask method is the negative of this ratio. The unsmoothed ratio equals delta_percent (basis 0) except that a bar with no volume is 0 here. Source: Sierra Chart, Bid Ask Volume Ratio (https://www.sierrachart.com/index.php?page=doc/StudiesReference.php&ID=230).",
        &[9, 0], false, bid_ask_volume_ratio),
    def("fve", "Finite Volume Elements (Katsanos)", V, FVE, &["fve", "ma"], PN, VOL,
        "Markos Katsanos' FVE, his AmiBroker code. tp = (high + low + close)/3. intra = ln(high) − ln(low); vintra = stdev(intra, length). inter = ln(tp) − ln(tp[1]); vinter = stdev(inter, length) (population, AmiBroker StDev). cutoff = coef·(vinter + vintra)·close. mf = close − (high + low)/2 + tp − tp[1]. vc = mf > cutoff ? volume : mf < −cutoff ? −volume : 0 (NaN until the cutoff exists). fve = 100·Σ(vc, length)/(sma(volume, length)·length); ma = sma(fve, ma_length). Defaults (AmiBroker code): length 22, coef 0.1, ma_length 30. fve from bar 2·length − 1, ma from bar 2·length + ma_length − 2; zero average volume → NaN. Readings above 0 = accumulation, below 0 = distribution. The author's EasyLanguage version uses separate intraday/interday coefficients and 21/50 defaults; the original April 2003 TASC form used a fixed 0.3 % cutoff. Source: Markos Katsanos, Finite Volume Elements (FVE) Indicator, AmiBroker and EasyLanguage code (https://mkatsanos.com/finite-volume-elements/).",
        &[43, 72], false, fve),
    def("multivote_obv", "Multi Vote On Balance Volume", V, &[], &["mvo"], PN, VOL,
        "MotiveWave Multi Vote OnBalance Volume (Barry M. McVicar, spelled VcVicar on the page). On each bar from bar 1, high, low and close each vote +1 if above the previous bar's value, −1 if below, 0 if equal; mvo = mvo[1] + volume·(high vote + low vote + close vote), starting from 0 before bar 1. NaN on bar 0. Adaptation: MotiveWave divides volume by 1,000,000 (volume in millions) for display; raw volume is used here, so values are 10⁶ times MotiveWave's. Source: MotiveWave Studies M–N, Multi Vote OnBalance Volume (https://docs.motivewave.com/studies/m-n).",
        &[1], false, multivote_obv),
    def("nvdi", "Negative Volume Disparity Indicator (Holt)", V, DI, &["nvdi", "signal"], PN, VOL,
        "Phillip C. Holt's NVDI (TASC June 1996), as in the Omega Research TradeStation function and MotiveWave. %b(x) = (x − lower)/(upper − lower) with Bollinger bands sma(x, length) ± stdevs·stdev(x, length) (population, TradeStation BollingerBand); zero band width → 0 (MotiveWave; TradeStation keeps the previous %b). negvol starts at 100 on bar 0; on a bar whose volume is below the previous bar's it adds 100·(close − close[1])/close[1] (additive percentage points, as published). nvdi = (1 + %b(close))/(1 + %b(negvol)); 1 + %b(negvol) = 0 → NaN. signal = sma(nvdi, signal). Defaults: length 33, stdevs 2, signal 4 (Holt). nvdi from bar length − 1, signal from bar length + signal − 2. Holt's system trades crosses of the 4-bar average through set levels. Sources: TASC Traders' Tips June 1996 (TechniFilter Plus, Omega Research TradeStation and Wave Wi$e code, https://www.traders.com/Documentation/FEEDbk_docs/1996/06/0696tradetips.html); MotiveWave Studies M–N, Negative Volume Disparity Indicator (https://docs.motivewave.com/studies/m-n).",
        &[32, 35], false, nvdi),
    def("obvdi", "On Balance Volume Disparity Indicator (Holt)", V, DI, &["obvdi", "signal"], PN, VOL,
        "Phillip C. Holt's OBVDI (TASC June 1996), as in the Omega Research TradeStation function, TechniFilter Plus and Wave Wi$e. %b(x) = (x − lower)/(upper − lower) with Bollinger bands sma(x, length) ± stdevs·stdev(x, length) (population); zero band width → 0. obv = running Σ sign(close − close[1])·volume (unchanged close adds 0; %b does not depend on OBV's starting value). obvdi = (1 + %b(close))/(1 + %b(obv)); 1 + %b(obv) = 0 → NaN. signal = sma(obvdi, signal). Defaults: length 33, stdevs 2, signal 4 (Holt). obvdi from bar length − 1, signal from bar length + signal − 2. Note: MotiveWave's OnBalance Volume Disparity study takes the second %b of the price %b itself (its page says volume is not considered); this entry follows Holt's published OBV form. Sources: TASC Traders' Tips June 1996 (https://www.traders.com/Documentation/FEEDbk_docs/1996/06/0696tradetips.html); MotiveWave Studies O–P, OnBalance Volume Disparity Indicator (https://docs.motivewave.com/studies/o-p).",
        &[32, 35], false, obvdi),
    def("svsi", "Slow Volume Strength Index (Apirine)", V, SV, &["svsi"], PN, VOL,
        "Vitali Apirine's SVSI (TASC June 2015), as in the TradeStation Traders' Tips code. e = ema(close, ema_length). Positive volume = volume when close > e, else 0; negative volume = volume when close < e, else 0 (equal → both 0). ap/an = Wilder averages (rma) of positive/negative volume over length. svs = ap/an (an = 0 → 100); svsi = 100 − 100/(1 + svs). Defaults: ema_length 6, length 14; levels 80/20, midline 50. First value at bar ema_length + length − 2. Adaptations: the lab's EMA and Wilder averages are seeded with an SMA (TradeStation XAverage seeds with the first close, and its Wilder averages with Average on bar 1). When ap = 0 and an > 0 the value is 0 here; the TradeStation code keeps the previous SVSI in that case. Sources: TASC Traders' Tips August 2015, TradeStation SVSI (https://traders.com/Documentation/FEEDbk_docs/2015/08/TradersTips.html); thinkorswim SlowVSI (https://toslc.thinkorswim.com/center/reference/Tech-Indicators/studies-library/R-S/SlowVSI).",
        &[18], false, svsi),
    def("tfs_volume_osc", "TFS Volume Oscillator", V, TFS, &["tfs_vo"], PN, VOL,
        "Bryan Strain's TFS Volume Oscillator (TASC June 2000), MotiveWave code: tfs_vo = Σ over the last length bars of (+volume if close > open, −volume if close < open, 0 if equal), divided by length. Default length 7. First value at bar length − 1. Adaptation: MotiveWave divides volume by 1,000,000 (millions); raw volume is used here. Source: MotiveWave Studies S–T, TFS Volume Oscillator (https://docs.motivewave.com/studies/s-t).",
        &[6], false, tfs_volume_osc),
    def("trend_thrust", "Trend Thrust Indicator (Dormeier)", V, TTI, &["sprd", "avg_sprd", "buff_diff"], PN, VOL,
        "Buff Pelz Dormeier's Trend Thrust Indicator (Investing With Volume Analysis; TASC August 2011), following the TradeStation code with Fred Schutzman's BD.VWMA function as posted on NexusFi. vwma(x, k) = Σ(x·volume, k)/Σ(volume, k). fastavg = vwma(close, fast), slowavg = vwma(close, slow). vfactor = 10·vwma(volume, fast)/ema(volume, slow); ivfactor = 10·vwma(volume, slow)/ema(volume, fast). FAST = fastavg/(vfactor/ivfactor), SLOW = (ivfactor/vfactor)·slowavg, sprd = FAST − SLOW. vavg = W·vwma(vfactor, fast)/ema(ivfactor, slow). avg_sprd = round(vwma(sprd, vavg), 3 decimals). buff_diff = (sprd − avg_sprd) − ema(ema(close, fast) − ema(close, slow), W). Defaults: fast 12, slow 26, W 9. sprd from bar slow − 1, avg_sprd and buff_diff from bar 2·slow − 2. Adaptations: the vwma of sprd has a fractional, changing length vavg; BD.VWMA's loop runs floor(vavg) bars, used here clamped to at least 1 and at most the bars of sprd so far (the TradeStation function returns 0 below 1). EMAs are the lab's SMA-seeded EMA (TradeStation XAverage seeds with the first value). A zero average volume gives NaN instead of TradeStation's held value. With equal volume on every bar the factors are 10 and the TTI reduces to an SMA MACD. Sources: Dormeier, Trend Thrust Indicator, TASC V.29:8 (https://www.traders.com/Documentation/FEEDbk_docs/2011/08/Dormeier.html); TradeStation TTI and BD.VWMA code posted on NexusFi (https://nexusfi.com/showthread.php?t=28672); Optuma, Trend Thrust Indicator (https://www.optuma.com/kb/tools/volume/trend-thrust-indicator).",
        &[25, 50, 50], false, trend_thrust),
    def("volume_momentum", "Volume Momentum", V, VM, &["vm"], PN, VOL,
        "FXCodeBase Volume Momentum: vm = sma(volume, length) − sma(volume, length)[lag] (\"Ma of Volume − MA of Volume N periods ago\"); multiply 1: vm·volume. Defaults: length 10, lag 10, multiply 0, SMA; the post gives no defaults or average type, so these are the lab's. First value at bar length + lag − 1. No platform in the survey lists this name; other forms exist (LuxAlgo smooths and differences the same way; momentum of raw volume is mom on volume). Source: FXCodeBase, Volume Momentum, forum topic 65047 (https://docs.gehtsoftusa.com/fxcodebase-backup/backup/custom-indicators/v/volume-momentum/).",
        &[19], false, volume_momentum),
    def("vpci", "Volume Price Confirmation Indicator (Dormeier)", V, VP, &["vpci", "smooth"], PN, VOL,
        "Buff Dormeier's VPCI (TASC July 2007, Between Price and Volume; 2007 Charles H. Dow Award paper), AmiBroker Traders' Tips code. vwma(k) = Σ(close·volume, k)/Σ(volume, k). VPC = vwma(long) − sma(close, long); VPR = vwma(short)/sma(close, short); VM = sma(volume, short)/sma(volume, long); vpci = VPC·VPR·VM. smooth = sma(vpci, smooth). Defaults: short 5, long 20, smooth 20 (Traders' Tips). vpci from bar long − 1, smooth from bar long + smooth − 2. Zero denominators → NaN. Note: the TradeStation Traders' Tips code divides vwma(short) by the average of the low, not of the close; the AmiBroker and MetaStock code and Dormeier's text use the close, as here. Sources: TASC Traders' Tips July 2007 (https://www.traders.com/Documentation/FEEDbk_docs/2007/07/TradersTips/TradersTips.html); MotiveWave Studies U–V, Volume Price Confirmation Indicator (https://docs.motivewave.com/studies/u-v).",
        &[19, 38], false, vpci),
    def("volume_rsi", "Volume RSI", V, VR, &["vrsi"], PN, VOL,
        "Wilder's RSI applied to the volume series (Wickra VolumeRsi): change = volume − volume[1]; gain = max(change, 0), loss = max(−change, 0); avg gain/loss = Wilder averages over length seeded with the SMA of the first length changes; vrsi = 100·avg_gain/(avg_gain + avg_loss); no gains and no losses → 50. Default length 14. First value at bar length. No platform in the survey lists a primary definition; other tools call an up-volume/down-volume ratio or the Money Flow Index a volume RSI, which are different calculations (see up/down volume and mfi). Source: Wickra docs v0.7.7, VolumeRsi (https://4e3995d1.wickra-docs.pages.dev/Indicators/Indicator-VolumeRsi).",
        &[14], false, volume_rsi),
    listed("time_and_sales", "Time and Sales", "Intraday & order flow", TICK,
        "Time and Sales (Sierra Chart, MotiveWave, ATAS and other platforms): the tape, a list of every trade with its time, price, size and whether it traded at the bid or the ask. It is a data window, not a bar series; tape studies built on it (speed_of_tape, big_trades) are listed separately. Source: MotiveWave User Guide, Time and Sales (https://docs.motivewave.com).",
        "not available yet — needs time-ordered intraday (tick) trade data"),
    listed("ergodic_tvi", "Ergodic Tick Volume Indicator (Blau)", "Intraday & order flow", TICK,
        "William Blau's Tick Volume Indicator (Momentum, Direction, and Divergence, 1995), per the secondary sources: tvi = 100·ema(ema(upticks − downticks, r), s)/ema(ema(upticks + downticks, r), s), where upticks/downticks are the counts of trades on an uptick/downtick in each bar; the ergodic form adds a signal line (a TradingView port cites periods 32, 5, 1 and signal 5). It needs uptick and downtick trade counts, which daily bars and the lab's footprint (volume at bid/ask per price level, no trade counts) do not hold; bid/ask volume is not a substitute for tick counts. Sources: backtrader blau_tvi (https://backtrader.readthedocs.io/en/latest/_modules/backtrader/indicators/contrib/blau_tvi.html); MultiCharts, Ergotic TVI (https://www.multicharts.com/support/base/miscellaneous-gt-ergotic-tvi-tick-volume-indicator/).",
        "not available yet — needs uptick and downtick trade counts (tick data)"),
    listed("triple_brick", "Triple Brick", "Unpublished formulas", NO,
        "Triple Brick: a time-independent (brick) chart type named in one platform survey listing. No help page, source code or construction rule was found (searched 2026-11-02). For the lab's brick chart see renko; for three-line reversals see line_break.",
        "not available yet — formula not published (no help page or construction rule found for this name)"),
    listed("triple_xo", "Triple XO", "Unpublished formulas", NO,
        "Triple XO: a time-independent X/O (point-and-figure style) chart type named in one platform survey listing. No help page, source code or construction rule was found (searched 2026-11-02). For point-and-figure columns and triple tops/bottoms see pnf and pnf_patterns.",
        "not available yet — formula not published (no help page or construction rule found for this name)"),
];
/// (id, platform names, AmiBroker names): one row per id in DEFS.
pub const SOURCES: &[(&str, &str, &str)] = &[
    ("rolling_vwap", "Sierra Chart: Volume Weighted Average Price - VWAP - Rolling with Standard Deviation Lines (study 352)", ""),
    ("equivolume", "StockCharts: EquiVolume; Wickra: Equivolume", ""),
    ("candlevolume", "StockCharts: CandleVolume; TradingView: Volume candles; Wickra: CandleVolume", ""),
    ("arms_candlevolume", "StockCharts: Arms CandleVolume", ""),
    ("bid_ask_volume_ratio", "Sierra Chart: Bid Ask Volume Ratio (study 230)", ""),
    ("fve", "Markos Katsanos: Finite Volume Elements (AmiBroker and EasyLanguage code); thinkorswim community: FVE", ""),
    ("multivote_obv", "MotiveWave: Multi Vote OnBalance Volume", ""),
    ("nvdi", "MotiveWave: Negative Volume Disparity Indicator; TradeStation (Omega Research, TASC Jun 1996): NVDI; TechniFilter Plus: NVI Disparity Indicator", ""),
    ("obvdi", "MotiveWave: OnBalance Volume Disparity Indicator; TradeStation (Omega Research, TASC Jun 1996): OBVDI; TechniFilter Plus: OBV Disparity Indicator", ""),
    ("svsi", "thinkorswim: SlowVSI; TradeStation (TASC Aug 2015 Traders' Tips): SVSI", ""),
    ("tfs_volume_osc", "MotiveWave: TFS Volume Oscillator", ""),
    ("trend_thrust", "Optuma: Trend Thrust Indicator; TradeStation (Dormeier code): TTI", ""),
    ("volume_momentum", "FXCodeBase: Volume Momentum", ""),
    ("vpci", "MotiveWave: Volume Price Confirmation Indicator (VPCI); TradeStation, MetaStock (TASC Jul 2007 Traders' Tips): VPCI", "VPCI (TASC Jul 2007 Traders' Tips AmiBroker code)"),
    ("volume_rsi", "Wickra: VolumeRsi", ""),
    ("time_and_sales", "Sierra Chart: Time and Sales; MotiveWave: Time and Sales", ""),
    ("ergodic_tvi", "MultiCharts: Ergotic TVI (Tick Volume Indicator)", ""),
    ("triple_brick", "Platform survey listing: Triple Brick (platform help not found)", ""),
    ("triple_xo", "Platform survey listing: Triple XO (platform help not found)", ""),
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lab::catalogue::{get, run};
    use crate::lab::footprint::{File, Footprint};
    use crate::lab::testdata::{assert_series, compiles, from_ohlc, go};
    use std::sync::Arc;
    const N: f64 = f64::NAN;
    fn bars(rows: &[(f64, f64, f64, f64)], v: &[f64]) -> Bars { let mut b = from_ohlc(rows); b.volume = Some(v.to_vec()); b }
    /// (o, h, l, c): 10/11/9/10, 10/12/10/11, 11/12/9/11, 11/11/8/9; volume 100, 200, 50, 10
    fn four() -> Bars { bars(&[(10., 11., 9., 10.), (10., 12., 10., 11.), (11., 12., 9., 11.), (11., 11., 8., 9.)], &[100., 200., 50., 10.]) }

    #[test] fn multivote_and_tfs() {
        // votes: bar 1 +3·200, bar 2 (h =, l down, c =) −1·50, bar 3 −3·10
        assert_series(&go("multivote_obv", &four(), &[])[0], &[N, 600., 550., 520.]);
        // sign(c − o)·v = 0, 200, 0, −10
        assert_series(&go("tfs_volume_osc", &four(), &[("length", 2.)])[0], &[N, 100., 100., -5.]);
    }
    #[test] fn volume_rsi_wilder_on_volume_changes() {
        let b = bars(&[(1., 1., 1., 1.); 5], &[100., 200., 50., 10., 60.]);
        // changes +100, −150, −40, +50: seed 50/75, then 25/57.5, then 37.5/28.75
        assert_series(&go("volume_rsi", &b, &[("length", 2.)])[0], &[N, N, 40., 2500. / 82.5, 3750. / 66.25]);
        assert_series(&go("volume_rsi", &bars(&[(1., 1., 1., 1.); 4], &[5.; 4]), &[("length", 2.)])[0], &[N, N, 50., 50.]);
    }
    #[test] fn volume_momentum_lags_the_average() {
        // sma2(v) = N, 150, 125, 30
        assert_series(&go("volume_momentum", &four(), &[("length", 2.), ("lag", 1.)])[0], &[N, N, -25., -95.]);
        assert_series(&go("volume_momentum", &four(), &[("length", 2.), ("lag", 1.), ("multiply", 1.)])[0], &[N, N, -1250., -950.]);
    }
    #[test] fn volume_boxes() {
        let w = [N, 200. / 150., 50. / 125., 10. / 30.];
        let e = go("equivolume", &four(), &[("length", 2.)]);
        assert_series(&e[0], &[2., 2., 3., 3.]);
        assert_series(&e[1], &w);
        let c = go("candlevolume", &four(), &[("length", 2.)]);
        assert_series(&c[0], &[0., 1., 0., -2.]);
        let a = go("arms_candlevolume", &four(), &[("length", 2.)]);
        assert_series(&a[0], &c[0]);
        assert_series(&a[1], &e[0]);
        assert_series(&a[2], &w);
    }
    #[test] fn rolling_vwap_two_bars() {
        let o = go("rolling_vwap", &four(), &[("length", 2.)]);
        let (x1, x3): (f64, f64) = (3200. / 300., 640. / 60.);
        let (s1, s3) = ((34200. / 300. - x1 * x1).sqrt(), (6860. / 60. - x3 * x3).sqrt());
        assert_series(&o[0], &[N, x1, 11., x3]);
        assert_series(&o[1], &[N, x1 + s1, 11., x3 + s3]);
        assert_series(&go("rolling_vwap", &four(), &[("length", 2.), ("mult", 2.)])[2], &[N, x1 - 2. * s1, 11., x3 - 2. * s3]);
    }
    #[test] fn svsi_counts_volume_against_the_ema() {
        // ema2(close) = N, 10.5, 65/6, 173/18: closes above on bars 1, 2 (200, 50), below on bar 3 (10)
        // Wilder 2: seed ap 125, an 0 → svs 100; bar 3 ap 62.5, an 5 → svs 12.5
        assert_series(&go("svsi", &four(), &[("ema_length", 2.), ("length", 2.)])[0], &[N, N, 100. - 100. / 101., 100. - 100. / 13.5]);
    }
    #[test] fn fve_with_zero_cutoff() {
        // tp = 10, 11, 32/3, 28/3; mf2 = 11 − 10.5 + 32/3 − 11 > 0 → +50; mf3 = 9 − 9.5 + 28/3 − 32/3 < 0 → −10 (first vc on bar 2)
        // fve3 = 100·40/(30·2)
        let o = go("fve", &four(), &[("length", 2.), ("coef", 0.), ("ma_length", 1.)]);
        assert_series(&o[0], &[N, N, N, 200. / 3.]);
        assert_series(&o[1], &o[0]);
    }
    #[test] fn vpci_product_of_three_terms() {
        // short 1: VPR = 1, VM = V/sma(V, 2); VPC = vwma2 − sma2
        let o = go("vpci", &four(), &[("short", 1.), ("long", 2.), ("smooth", 2.)]);
        let (a, c) = ((3200. / 300. - 10.5) * (200. / 150.), (640. / 60. - 10.) * (10. / 30.));
        assert_series(&o[0], &[N, a, 0., c]);
        assert_series(&o[1], &[N, N, a / 2., c / 2.]);
    }
    #[test] fn holt_disparities() {
        // closes 10, 9, 13: mean 32/3, population variance 26/9 → %b = (7 + √26)/(2√26) with 1 sd
        let pb = (7. + 26f64.sqrt()) / (2. * 26f64.sqrt());
        let b = |v: &[f64]| bars(&[(10., 10., 10., 10.), (9., 9., 9., 9.), (13., 13., 13., 13.)], v);
        let prm = [("length", 3.), ("stdevs", 1.), ("signal", 1.)];
        // rising volume: negvol stays 100 → %b 0
        assert_series(&go("nvdi", &b(&[1., 2., 3.]), &prm)[0], &[N, N, 1. + pb]);
        // volume drops on bar 2: negvol 100, 100, 100 + 400/9 → %b = (2 + √2)/(2√2)
        assert_series(&go("nvdi", &b(&[1., 2., 1.]), &prm)[0], &[N, N, (1. + pb) / (1. + (2. + 2f64.sqrt()) / (2. * 2f64.sqrt()))]);
        // obv 0, −5, 0 → %b = (1 + √2)/(2√2)
        let o = go("obvdi", &b(&[5., 5., 5.]), &prm);
        assert_series(&o[0], &[N, N, (1. + pb) / (1. + (1. + 2f64.sqrt()) / (2. * 2f64.sqrt()))]);
        assert_series(&o[1], &o[0]);
    }
    #[test] fn trend_thrust_with_equal_volume_is_an_sma_macd() {
        // equal volume: vfactor = ivfactor = 10, vavg = W; fast 1, slow 2: sprd = close − sma2(close)
        let b = bars(&[(10., 10., 10., 10.), (11., 11., 11., 11.), (11., 11., 11., 11.), (9., 9., 9., 9.), (12., 12., 12., 12.)], &[7.; 5]);
        let o = go("trend_thrust", &b, &[("fast", 1.), ("slow", 2.), ("signal", 1.)]);
        assert_series(&o[0], &[N, 0.5, 0., -1., 1.5]);
        assert_series(&o[1], &[N, N, 0., -1., 1.5]);
        // m = close − ema2(close): 0.5, 1/6, −11/18, 12 − (173/18 + 2/3·(12 − 173/18))
        let e4 = 173. / 18. + 2. / 3. * (12. - 173. / 18.);
        assert_series(&o[2], &[N, N, -1. / 6., 11. / 18., -(12. - e4)]);
        // W 2: avg_sprd = round(sma2(sprd), 3)
        assert_series(&go("trend_thrust", &b, &[("fast", 1.), ("slow", 2.), ("signal", 2.)])[1], &[N, N, 0.25, -0.5, 0.25]);
    }
    #[test] fn bid_ask_ratio_from_the_footprint() {
        let d = ["2024-01-01", "2024-01-02", "2024-01-03", "2024-01-04", "2024-01-05"];
        let rws: Vec<crate::Row> = d.iter().map(|x| crate::Row { ticker: "T".into(), date: x.to_string(), open: 100., high: 101., low: 99., close: 100., volume: Some(1.) }).collect();
        let mut b = Bars::from_rows(&rws);
        // (date, price, bid, ask); no footprint on Jan 3; Jan 5 has rows with no volume
        let rows = [("2024-01-01", 100., 10., 30.), ("2024-01-02", 100., 40., 10.), ("2024-01-04", 100., 5., 25.), ("2024-01-05", 100., 0., 0.)];
        let file = File::from_rows(&rows.iter().map(|r| (r.0.to_string(), r.1, r.2, r.3)).collect::<Vec<_>>()).unwrap();
        b.footprint = Some(Footprint::align(Arc::new(file), &b.date));
        let o = run(&get("bid_ask_volume_ratio").unwrap(), &b, &[("length".to_string(), 2.)].into()).unwrap();
        assert_series(&o[1], &[50., -60., N, 200. / 3., 0.]);
        let e3 = -5. + 2. / 3. * (200. / 3. + 5.);
        assert_series(&o[0], &[N, -5., N, e3, e3 + 2. / 3. * (0. - e3)]);
        let s = run(&get("bid_ask_volume_ratio").unwrap(), &b, &[("length".to_string(), 2.), ("ma_type".to_string(), 1.)].into()).unwrap();
        assert_series(&s[0], &[N, -5., N, (200. / 3. - 60.) / 2., 100. / 3.]);
    }
    #[test] fn listed_entries_carry_notes_and_every_entry_one_source() {
        for d in DEFS {
            assert_eq!(SOURCES.iter().filter(|s| s.0 == d.id).count(), 1, "{}", d.id);
            if d.compute.is_none() { assert!(d.note.starts_with("not available yet — "), "{}: {}", d.id, d.note); }
        }
        assert_eq!(SOURCES.len(), DEFS.len());
    }
    #[test] fn rules_reach_gap_d() {
        for (id, out) in [("rolling_vwap", "upper"), ("equivolume", "width"), ("candlevolume", "body"), ("arms_candlevolume", "height"), ("fve", "ma"),
            ("multivote_obv", "mvo"), ("nvdi", "signal"), ("obvdi", "obvdi"), ("svsi", "svsi"), ("tfs_volume_osc", "tfs_vo"), ("trend_thrust", "buff_diff"),
            ("volume_momentum", "vm"), ("vpci", "smooth"), ("volume_rsi", "vrsi")] {
            compiles(id, out).unwrap();
        }
    }
}
