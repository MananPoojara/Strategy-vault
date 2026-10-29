//! Census 2 group "b" (2026-10-27): indicators from trading_indicator_census.html that the lab did not have.
//! Formulas: see each definition string. Listed-only entries carry "not available yet — needs …" (data the lab does not hold)
//! or "not available yet — formula not published (…)" (proprietary or no primary text; nothing invented).
//! Skipped (not built, not listed): Adaptive Smoothing Indicator (MotiveWave: Tillson T3, our t3); Countback Line (Guppy;
//! a drawing tool anchored at a low the analyst picks, as Optuma's Countback Line tools are, not a bar-by-bar indicator).
use super::bars::Bars;
use super::catalogue::{get, int, len, n, p, real, run, IndicatorDef, Needs, Output, ParamSpec, Params, Placement};
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
const TR: &str = "Trend";
const MA: &str = "Moving averages";
const VO: &str = "Volatility";
const BC: &str = "Bands & channels";
const PN: Placement = Placement::Pane;
const PR: Placement = Placement::Price;
const NO: Needs = Needs::NONE;
const OPT: Needs = Needs { derivatives: true, ..Needs::NONE };

fn call(id: &str, b: &Bars, kv: &[(&str, f64)]) -> Result<Vec<Vec<f64>>, String> {
    run(&get(id).ok_or_else(|| format!("{id} missing"))?, b, &kv.iter().map(|(k, v)| (k.to_string(), *v)).collect())
}
fn fin(v: f64) -> f64 { if v.is_finite() { v } else { NA } }

// ---- Sepiashvili, "Trend-Quality Indicator", TASC April 2004 (Traders' Tips: TradeStation, MetaStock, AmiBroker) ----
const TPP: &[ParamSpec] = &[len("fast", "Fast EMA length", 7.), len("slow", "Slow EMA length", 15.)];
const TQP: &[ParamSpec] = &[len("fast", "Fast EMA length", 7.), len("slow", "Slow EMA length", 15.), len("trend_length", "Trend length m", 4.),
    len("noise_length", "Noise length n", 250.), real("correction", "Correction factor", 2., 0.01, 100., 0.01),
    int("noise_type", "Noise (0 root mean square, 1 linear)", 0., 0., 1.),
    int("variant", "Variant (0 AmiBroker/thinkorswim: CPC = Σ change, EMA 2/(m+1); 1 TradeStation/MetaStock: CPC = Σ change·periods, alpha 1/m)", 0., 0., 1.)];
fn periods(b: &Bars, prm: &Params) -> Vec<f64> {
    let (f, s) = (ta::ema(&b.close, n(prm, "fast")), ta::ema(&b.close, n(prm, "slow")));
    f.iter().zip(&s).map(|(a, c)| { let d = a - c; if d.is_nan() { NA } else if d > 0. { 1. } else if d < 0. { -1. } else { 0. } }).collect()
}
/// (q, b): trend/noise and 100·|trend|/(|trend| + noise).
fn trend_quality_core(b: &Bars, prm: &Params) -> (Vec<f64>, Vec<f64>) {
    let (pd, c, m) = (periods(b, prm), &b.close, b.len());
    let (tl, v1) = (p(prm, "trend_length"), n(prm, "variant") == 1);
    let a = if v1 { 1. / tl } else { 2. / (tl + 1.) };
    let (mut trend, mut dt) = (vec![NA; m], vec![NA; m]);
    let (mut cpc, mut tr) = (0., 0.);
    for t in 0..m {
        if pd[t].is_nan() { continue; }
        if t == 0 || pd[t - 1].is_nan() || pd[t] != pd[t - 1] { (cpc, tr) = (0., 0.); } else {
            cpc += (c[t] - c[t - 1]) * if v1 { pd[t] } else { 1. };
            tr += a * (cpc - tr);
        }
        (trend[t], dt[t]) = (tr, cpc - tr);
    }
    let (k, cf) = (n(prm, "noise_length"), p(prm, "correction"));
    let noise: Vec<f64> = if n(prm, "noise_type") == 1 { ta::sma(&dt.iter().map(|v| v.abs()).collect::<Vec<_>>(), k).iter().map(|v| cf * v).collect() }
        else { ta::sma(&dt.iter().map(|v| v * v).collect::<Vec<_>>(), k).iter().map(|v| cf * v.sqrt()).collect() };
    let q = trend.iter().zip(&noise).map(|(t, z)| if *z > 0. { t / z } else { NA }).collect();
    let bal = trend.iter().zip(&noise).map(|(t, z)| if t.abs() + z > 0. { 100. * t.abs() / (t.abs() + z) } else { NA }).collect();
    (q, bal)
}
fn trend_periods(b: &Bars, prm: &Params) -> Output { Ok(vec![periods(b, prm)]) }
fn trend_quality(b: &Bars, prm: &Params) -> Output { Ok(vec![trend_quality_core(b, prm).0]) }
fn trend_noise_balance(b: &Bars, prm: &Params) -> Output { Ok(vec![trend_quality_core(b, prm).1]) }

// ---- Satchwell, "Regularization", TASC July 2003 (Traders' Tips: TradeStation RegEMA) ----
const RE: &[ParamSpec] = &[len("length", "EMA length", 9.), real("lambda", "Lambda (regularization)", 9., 0., 1000., 0.1)];
fn rema(b: &Bars, prm: &Params) -> Output {
    let (a, l, c) = (2. / (p(prm, "length") + 1.), p(prm, "lambda"), &b.close);
    let mut r = vec![NA; c.len()];
    for t in 0..c.len() { r[t] = if t < 3 { c[t] } else { (r[t - 1] * (1. + 2. * l) + a * (c[t] - r[t - 1]) - l * r[t - 2]) / (1. + l) }; }
    Ok(vec![r.into_iter().map(fin).collect()])
}

// ---- Ehlers, "Just Ignore Them", TASC April 2023 (Traders' Tips: TradeStation $Hann, PineCoders) ----
const UD: &[ParamSpec] = &[len("sample", "Sampling period (bars)", 5.), len("fast", "Fast Hann length", 6.), len("slow", "Slow Hann length", 12.)];
fn hann(x: &[f64], l: usize) -> Vec<f64> {
    let w: Vec<f64> = (1..=l).map(|k| 1. - (2. * std::f64::consts::PI * k as f64 / (l as f64 + 1.)).cos()).collect();
    let sw: f64 = w.iter().sum();
    (0..x.len()).map(|t| if t + 1 < l { NA } else { w.iter().enumerate().map(|(k, wk)| wk * x[t - k]).sum::<f64>() / sw }).collect()
}
fn udma(b: &Bars, prm: &Params) -> Output {
    let k = n(prm, "sample");
    let mut s = b.close.clone();
    for t in 1..s.len() { if t % k != 0 { s[t] = s[t - 1]; } }
    Ok(vec![hann(&s, n(prm, "fast")), hann(&s, n(prm, "slow"))])
}

// ---- Star, "Trade Breakouts And Retracements With TMV", TASC February 2012 (Traders' Tips: TradeStation TrendInd) ----
const TM: &[ParamSpec] = &[len("adx_length", "ADX length", 10.), len("sma_length", "Close SMA length", 8.)];
fn tmv(b: &Bars, prm: &Params) -> Output {
    let k = p(prm, "adx_length");
    let adx = call("dmi", b, &[("di_length", k), ("adx_smoothing", k)])?.swap_remove(2);
    let sma = ta::sma(&b.close, n(prm, "sma_length"));
    let (mut out, mut prev) = (vec![NA; b.len()], 0.);
    for t in 1..b.len() {
        if adx[t].is_nan() || adx[t - 1].is_nan() || sma[t].is_nan() { continue; }
        prev = if adx[t] > adx[t - 1] { if b.close[t] > sma[t] { 1. } else if b.close[t] < sma[t] { -1. } else { prev } } else { 0. };
        out[t] = prev;
    }
    Ok(vec![out])
}

// ---- Rich & Rich, "Simplify It" (TASC November 2015); thinkorswim SimpleTrendChannelFilter ----
const SC: &[ParamSpec] = &[int("trend_check", "Trend check (0 Min: close vs SMA50; 1 Normal: + SMA20 vs SMA50; 2 Max: + SMA50 vs SMA200)", 1., 0., 2.)];
fn simple_trend_channel(b: &Bars, prm: &Params) -> Output {
    let (c, mode) = (&b.close, n(prm, "trend_check"));
    let (s20, s50, s200) = (ta::sma(c, 20), ta::sma(c, 50), ta::sma(c, 200));
    let state = |t: usize, up: bool| -> f64 {
        let gt = |a: f64, z: f64| if up { a > z } else { a < z };
        let need = [s50[t], if mode >= 1 { s20[t] } else { 0. }, if mode == 2 { s200[t] } else { 0. }];
        if need.iter().any(|v| v.is_nan()) { return NA; }
        (gt(c[t], s50[t]) && (mode < 1 || gt(s20[t], s50[t])) && (mode < 2 || gt(s50[t], s200[t]))) as u8 as f64
    };
    Ok(vec![(0..c.len()).map(|t| state(t, true)).collect(), (0..c.len()).map(|t| state(t, false)).collect()])
}

// ---- Wilson, "The Wilson Relative Price Channel", TASC July 2006 (Traders' Tips: TradeStation, AmiBroker) ----
const WR: &[ParamSpec] = &[len("length", "RSI length", 34.), len("smoothing", "EMA smoothing (1 = none)", 1.),
    real("overbought", "Overbought", 70., 0., 100., 1.), real("upper_neutral", "Upper neutral zone", 55., 0., 100., 1.),
    real("lower_neutral", "Lower neutral zone", 45., 0., 100., 1.), real("oversold", "Oversold", 30., 0., 100., 1.)];
fn wilson_rpc(b: &Bars, prm: &Params) -> Output {
    let r = call("rsi", b, &[("length", p(prm, "length"))])?.swap_remove(0);
    let s = n(prm, "smoothing");
    Ok(["overbought", "upper_neutral", "lower_neutral", "oversold"].iter().map(|k| {
        let e = ta::ema(&r.iter().map(|v| v - p(prm, k)).collect::<Vec<_>>(), s);
        e.iter().zip(&b.close).map(|(e, c)| c - c * e / 100.).collect()
    }).collect())
}

// ---- Downs, "Mutated Variables, Volatility And A New Market Paradigm", TASC June 1998 (Traders' Tips: TradeStation) ----
const NM: &[ParamSpec] = &[len("length", "Bollinger length", 28.), real("sd_up", "Upper band deviations", 2., 0.1, 10., 0.1), real("sd_dn", "Lower band deviations", 2., 0.1, 10., 0.1)];
fn new_market_paradigm(b: &Bars, prm: &Params) -> Output {
    let k = n(prm, "length");
    let (m, sd) = (ta::sma(&b.close, k), ta::stdev(&b.close, k));
    let up: Vec<f64> = m.iter().zip(&sd).map(|(m, s)| m + p(prm, "sd_up") * s).collect();
    let lo: Vec<f64> = m.iter().zip(&sd).map(|(m, s)| m - p(prm, "sd_dn") * s).collect();
    Ok(vec![(0..b.len()).map(|t| {
        if t == 0 || up[t].is_nan() || up[t - 1].is_nan() { NA }
        else if up[t] < up[t - 1] && lo[t] > lo[t - 1] { 1. } else if up[t] > up[t - 1] && lo[t] < lo[t - 1] { 2. } else { 3. }
    }).collect()])
}

// ---- MotiveWave MWDX Average ----
const MW: &[ParamSpec] = &[real("factor", "Factor", 0.2, 0.001, 1., 0.001)];
fn mwdx(b: &Bars, prm: &Params) -> Output {
    let f = p(prm, "factor");
    let mut prev = NA;
    Ok(vec![b.close.iter().map(|&c| { prev = if prev.is_nan() { c } else { f * c + (1. - f) * prev }; prev }).collect()])
}

// ---- Optuma DGL Trade Bands ----
fn dgl_trade_bands(b: &Bars, _: &Params) -> Output {
    let tp = b.hlc3();
    let rg: Vec<f64> = (0..b.len()).map(|i| b.high[i] - b.low[i]).collect();
    Ok(vec![(0..b.len()).map(|i| 2. * tp[i] - b.low[i]).collect(), (0..b.len()).map(|i| tp[i] + rg[i]).collect(),
            (0..b.len()).map(|i| 2. * tp[i] - b.high[i]).collect(), (0..b.len()).map(|i| tp[i] - rg[i]).collect()])
}

// ---- Katsanos, "Trading The Loonie", TASC December 2015 (Traders' Tips: AmiBroker, eSignal, Updata) ----
const BD: &[ParamSpec] = &[len("length", "Bollinger length", 20.)];
fn bb_divergence(b: &Bars, prm: &Params) -> Output {
    let (s, k) = (super::pairs::second(b)?, n(prm, "length"));
    let bol = |x: &[f64]| -> Vec<f64> {
        let (m, sd) = (ta::sma(x, k), ta::stdev(x, k));
        (0..x.len()).map(|i| 1. + (x[i] - m[i] + 2. * sd[i]) / (4. * sd[i] + 0.0001)).collect()
    };
    let (b1, b2) = (bol(&b.close), bol(s));
    Ok(vec![b1.iter().zip(&b2).map(|(a, z)| if *a == 0. { NA } else { fin(100. * (z - a) / a) }).collect()])
}

// ---- ProRealCode ATR percentile rank (Nicolas) ----
const AP: &[ParamSpec] = &[len("atr_length", "ATR length", 20.), len("lookback", "Rank lookback", 200.)];
fn atr_percentile_rank(b: &Bars, prm: &Params) -> Output {
    let a = call("atr", b, &[("length", p(prm, "atr_length"))])?.swap_remove(0);
    let k = n(prm, "lookback");
    Ok(vec![(0..a.len()).map(|t| {
        if t < k || a[t - k..=t].iter().any(|v| v.is_nan()) { return NA; }
        100. * (1..=k).filter(|&i| a[t] > a[t - i]).count() as f64 / k as f64
    }).collect()])
}

// ---- Poster, "Trend Strength: Measuring The Duration Of A Trend", TASC February 2021 (Traders' Tips: TradersStudio, eSignal) ----
const TP: &[ParamSpec] = &[len("tpr_length", "TPR length", 20.), len("sma_length", "SMA length", 5.), real("threshold", "Threshold (points × multiplier)", 1., 0., 1e6, 0.1),
    real("mult", "Multiplier", 10., 0.001, 1e6, 1.), real("point", "Point (minimum tick; 0.01 for stocks)", 0.01, 1e-8, 1e6, 0.0001)];
fn trend_persistence_rate(b: &Bars, prm: &Params) -> Output {
    let s = ta::sma(&b.close, n(prm, "sma_length"));
    let (unit, th, k) = (p(prm, "mult") * p(prm, "point"), p(prm, "threshold"), n(prm, "tpr_length"));
    let d: Vec<f64> = (0..s.len()).map(|t| if t == 0 { NA } else { (s[t] - s[t - 1]) / unit }).collect();
    Ok(vec![(0..d.len()).map(|t| {
        if t + 1 < k || d[t + 1 - k..=t].iter().any(|v| v.is_nan()) { return NA; }
        let w = &d[t + 1 - k..=t];
        let net = w.iter().filter(|v| **v > th).count() as f64 - w.iter().filter(|v| **v < -th).count() as f64;
        (100. * net / k as f64).abs()
    }).collect()])
}

const UNPUB_TOS: &str = "not available yet — formula not published (thinkorswim; the study's source code and exact rule are not published)";
pub const DEFS: &[IndicatorDef] = &[
    def("trend_periods", "Trend Periods (Sepiashvili)", TR, TPP, &["periods"], PN, NO, "Sepiashvili, Trend-Quality Indicator (TASC April 2004; Traders' Tips TradeStation TrendPeriods, thinkorswim TrendPeriods): periods = sign(ema(close, fast) − ema(close, slow)): +1, −1, or 0 when equal; EMAs SMA-seeded (lab ema); defaults 7 and 15", &[14], false, trend_periods),
    def("trend_quality", "Trend Quality Indicator (Sepiashvili Q)", TR, TQP, &["q"], PN, NO, "Sepiashvili, Trend-Quality Indicator (TASC April 2004; Traders' Tips): periods = sign(ema(close, 7) − ema(close, 15)); on the first bar with periods and on every bar where periods changes CPC = 0 and trend = 0 (AmiBroker's piecewise EMA restart; AmiBroker's Sum(DC, 0) is taken as 0); otherwise variant 0 (AmiBroker, Janeczko; thinkorswim: signed) CPC += close − close[1] and trend += 2/(m + 1)·(CPC − trend); variant 1 (TradeStation, Mills; MetaStock) CPC += (close − close[1])·periods and trend += (1/m)·(CPC − trend); dt = CPC − trend; noise = cf·sqrt(sma(dt², n)) (noise_type 1 linear: cf·sma(|dt|, n)); q = trend/noise (noise 0 → no value); defaults m 4, n 250, cf 2; guide lines ±1, ±2, ±5", &[263], false, trend_quality),
    def("trend_noise_balance", "Trend-Noise Balance (Sepiashvili B)", TR, TQP, &["b"], PN, NO, "Sepiashvili's B-indicator (TASC April 2004; TradeStation TrendQ plot, MetaStock B-indicator, thinkorswim TrendNoiseBalance): b = 100·|trend|/(|trend| + noise) with trend and noise as in trend_quality (same parameters and variants); 0..100, 50 = trend and noise in balance; |trend| + noise = 0 → no value", &[263], false, trend_noise_balance),
    def("rema", "Regularized EMA (Satchwell)", MA, RE, &["rema"], PR, NO, "Satchwell, Regularization (TASC July 2003; Traders' Tips TradeStation RegEMA): a = 2/(n + 1); rema = close on bars 0..2; then rema = (rema[1]·(1 + 2λ) + a·(close − rema[1]) − λ·rema[2])/(1 + λ); λ = 0 is an EMA seeded with the close; defaults n 9, λ 9", &[0], false, rema),
    def("udma", "Undersampled Double Moving Average (Ehlers)", MA, UD, &["fast", "slow"], PR, NO, "Ehlers, Just Ignore Them (TASC April 2023; Traders' Tips PineCoders and TradeStation $Hann): sample = close on bars where bar_index mod k = 0 (bar 0 included, PineCoders), held in between; Hann FIR: Σ_{j=1..L} w_j·sample[j − 1]/Σ w_j, w_j = 1 − cos(2πj/(L + 1)); fast and slow lengths; no value before L bars (Pine fills the warm-up with nz zeros); bar_index counts from the first loaded bar, so the sampling phase depends on where the history begins; defaults k 5, fast 6, slow 12", &[5, 11], false, udma),
    def("tmv", "TMV Trend (Barbara Star)", TR, TM, &["trend"], PN, NO, "Star, Trade Breakouts And Retracements With TMV (TASC February 2012; Traders' Tips TradeStation strategy TrendInd; thinkorswim TMV): trend = +1 when ADX(10) > ADX[1] and close > sma(close, 8), −1 when ADX rises and close < sma, the previous value when ADX rises and close = sma, 0 when ADX does not rise (the bar colour); ADX = our dmi adx with both lengths 10 (Wilder); the TMV's other parts (Keltner channel sma(typical price, 20) ± 1·ATR(20) in the TradeStation code, CCI(13), volume oscillator) are separate studies and not in this output", &[20], false, tmv),
    def("simple_trend_channel", "Simple Trend Channel Filter (Rich)", TR, SC, &["uptrend", "downtrend"], PN, NO, "J. E. Rich & J. B. Rich, Simplify It (TASC November 2015), as thinkorswim SimpleTrendChannelFilter: uptrend = 1 when close > sma(close, 50) (Min), and sma(20) > sma(50) (Normal), and sma(50) > sma(200) (Max); downtrend likewise with <; else 0; the system's S&P 500 check (index above its SMA50) is a second instrument and is not part of this filter; default Normal (lab choice; thinkorswim does not state its default)", &[49, 49], false, simple_trend_channel),
    def("wilson_rpc", "Wilson Relative Price Channel", BC, WR, &["overbought", "upper_neutral", "lower_neutral", "oversold"], PR, NO, "Leon Wilson, The Wilson Relative Price Channel (TASC July 2006; Traders' Tips TradeStation and AmiBroker): r = rsi(close, 34) (Wilder); for each level L (70, 55, 45, 30): e = ema(r − L, smoothing) (1 = no smoothing); line = close − close·e/100", &[34, 34, 34, 34], false, wilson_rpc),
    def("new_market_paradigm", "New Market Paradigm (Downs)", BC, NM, &["phase"], PN, NO, "Downs, Mutated Variables, Volatility And A New Market Paradigm (TASC June 1998; Traders' Tips TradeStation NewMarketParadigm; MotiveWave NMP Contraction/Expansion): upper = sma(close, 28) + 2σ, lower = sma − 2σ (population σ); phase = 1 contraction when upper < upper[1] and lower > lower[1], 2 expansion when upper > upper[1] and lower < lower[1], 3 transition otherwise; Downs' system counts two consecutive expansion bars as an expansion phase; the bands are our bb", &[28], false, new_market_paradigm),
    def("mwdx", "MWDX Average (MotiveWave)", MA, MW, &["mwdx"], PR, NO, "MotiveWave MWDX Average (documented condensed code): value2 = 2/fac − 1, factor = 2/(value2 + 1) = fac; mwdx = factor·close + (1 − factor)·mwdx[1], the first bar = close (isNull(price, prev)); an EMA with alpha fac seeded with the first close; default fac 0.2", &[0], false, mwdx),
    def("dgl_trade_bands", "DGL Trade Bands (Optuma)", BC, &[], &["buy1", "buy2", "sell1", "sell2"], PR, NO, "Optuma DGL Trade Bands (knowledge base): tp = (high + low + close)/3; buy1 = 2·tp − low; buy2 = tp + (high − low); sell1 = 2·tp − high; sell2 = tp − (high − low) (Optuma prints ((High + Low + Close/3) − (High − Low)), a misplaced parenthesis); levels from this bar's prices, read against the next bar", &[0, 0, 0, 0], false, dgl_trade_bands),
    def("bb_divergence", "Bollinger Band Divergence (Katsanos)", BC, BD, &["divergence"], PN, Needs { second_symbol: true, ..Needs::NONE }, "Katsanos, Trading The Loonie (TASC December 2015; Traders' Tips AmiBroker, eSignal; thinkorswim BBDivergence): bol(x) = 1 + (x − sma(x, n) + 2·σ(x, n))/(4·σ(x, n) + 0.0001) (population σ, AmiBroker StDev); divergence = 100·(bol(second) − bol(close))/bol(close) (bol(close) = 0 → no value); the second instrument is the related market (Katsanos: crude oil for the Canadian dollar); default n 20; guide lines ±20", &[19], false, bb_divergence),
    def("atr_percentile_rank", "ATR Percentile Rank", VO, AP, &["rank"], PN, NO, "ProRealCode ATR percentile rank (Nicolas, ProBuilder source): atr = Wilder ATR(n) (our atr); rank = 100·(count of i = 1..L with atr > atr[i])/L, strictly greater; defaults n 20, L 200", &[219], false, atr_percentile_rank),
    def("trend_persistence_rate", "Trend Persistence Rate (Poster)", TR, TP, &["tpr"], PN, NO, "Poster, Trend Strength: Measuring The Duration Of A Trend (TASC February 2021; Traders' Tips TradersStudio TPR, eSignal): s = sma(close, m); slope = (s − s[1])/(mult·point); over the last N bars including this one, P = count of slope > threshold, M = count of slope < −threshold; tpr = |100·(P − M)/N|; defaults N 20, m 5, threshold 1, mult 10, point 0.01 (stocks; use the instrument's tick)", &[24], false, trend_persistence_rate),
    listed("implied_volatility", "Implied Volatility", VO, OPT, "Volatility implied by option prices (option pricing model inverted on traded option premiums)", "not available yet — needs option implied-volatility data (option chains)"),
    listed("iv_percentile", "Implied Volatility Percentile", VO, OPT, "Share of the past year's days with implied volatility below today's", "not available yet — needs option implied-volatility data (option chains)"),
    listed("iv_rank", "Implied Volatility Rank", VO, OPT, "100·(IV − lowest IV)/(highest IV − lowest IV) over the past year", "not available yet — needs option implied-volatility data (option chains)"),
    listed("ma_rolling_high_accuracy", "Moving Average - Rolling High Accuracy (Sierra Chart)", MA, Needs { order_flow: true, ..Needs::NONE }, "Sierra Chart (ACSIL source): Σ(price·number of trades) / Σ(number of trades) over the volume-at-price levels of the bars in a rolling time window", "not available yet — needs volume-at-price data (number of trades at each price level per bar)"),
    listed("long_haul_filter", "Long Haul Filter (Pendergast)", TR, Needs { fundamentals: true, ..Needs::NONE }, "thinkorswim stock screen for Pendergast's Long Haul method (TASC 2014): RSI below 5, close above its average, average volume above 1,000,000, beta above 1, last actual earnings above the estimate", "not available yet — needs company earnings (actual versus estimate) data and a benchmark for beta"),
    listed("gonogo_trend", "GoNoGo Trend", TR, NO, "GoNoGo Charts proprietary trend colouring", "not available yet — formula not published (GoNoGo Charts)"),
    listed("gonogo_squeeze", "GoNoGo Squeeze", VO, NO, "GoNoGo Charts proprietary squeeze", "not available yet — formula not published (GoNoGo Charts)"),
    listed("dvan_smartlines", "DVAN SmartLines (D Smart Line)", TR, NO, "DVAN methodology (Jim Kennedy), Optuma add-on", "not available yet — formula not published (Jim Kennedy, Divergence Analysis DVAN)"),
    listed("dvan_long_term", "DVAN Long Term", TR, NO, "DVAN methodology (Jim Kennedy), Optuma add-on", "not available yet — formula not published (Jim Kennedy, Divergence Analysis DVAN)"),
    listed("dvan_trending", "DVAN Trending", TR, NO, "DVAN methodology (Jim Kennedy), Optuma add-on", "not available yet — formula not published (Jim Kennedy, Divergence Analysis DVAN)"),
    listed("dvan_trend_signals", "DVAN Trend Signals", TR, NO, "DVAN methodology (Jim Kennedy), Optuma add-on", "not available yet — formula not published (Jim Kennedy, Divergence Analysis DVAN)"),
    listed("prime_number_bands", "Prime Number Bands", BC, NO, "Bands at the prime numbers nearest the high and the low (Modulus Financial Engineering); the price scaling and rounding rule are not stated", "not available yet — formula not published (Modulus Financial Engineering)"),
    listed("valuation_lines", "Valuation Lines", BC, NO, "Valuation Lines", "not available yet — formula not published (platform not identified; no primary text found)"),
    listed("narrow_sideways_channel", "Narrow Sideways Channel (Omega Research)", BC, NO, "SMA(close, 14) ± 3 standard deviations, recomputed every 14 bars (MotiveWave)", "not available yet — formula not published (Omega Research 1997; MotiveWave's version paints each channel back over the bars it measures, which is look-ahead, and does not state its deviation type)"),
    listed("relative_range_index", "Relative Range Index", VO, NO, "thinkorswim: −100..+100 ratio of 'strong' range changes to all range changes over n bars", UNPUB_TOS),
    listed("weakness_in_strong_trend", "Weakness In A Strong Trend", TR, NO, "thinkorswim: three (four) consecutive lower prices while the average rises, and the mirror", UNPUB_TOS),
    listed("swami_laguerre_trend", "Swami Laguerre Trend", TR, NO, "thinkorswim Thermo Mode heatmap of Laguerre-filter trend over a range of lengths (Ehlers & Way SwamiCharts)", "not available yet — formula not published (thinkorswim / Ehlers & Way SwamiCharts; filter and heatmap scaling not stated)"),
    listed("swami_volatility", "Swami Volatility", VO, NO, "thinkorswim Thermo Mode heatmap comparing true range with its SMA over a range of lengths", "not available yet — formula not published (thinkorswim / Ehlers & Way SwamiCharts; comparison and scaling not stated)"),
    listed("trend_seeker", "Trend Seeker", TR, NO, "Barchart proprietary trend signal", "not available yet — formula not published (Barchart)"),
    listed("chande_trend_meter", "Chande Trend Meter", TR, NO, "StockCharts trend score (Chande)", "not available yet — formula not published (StockCharts; weights and scoring not published)"),
    listed("traffic_light", "Traffic Light", TR, NO, "StockCharts shading tool", "not available yet — formula not published (StockCharts; compared series and boundaries not stated)"),
    listed("chats", "Chande Absolute Trend Strength", TR, NO, "TrendSpider CHATS (with Tushar Chande): RSI(14), normalised 100-day returns, 100-day channel and Bollinger breakouts", "not available yet — formula not published (TrendSpider, Tushar Chande; weights not published)"),
    listed("avsl", "Anti-Volume Stop Loss (Dormeier)", TR, NO, "Dormeier, Investing with Volume Analysis: a lower Bollinger band adjusted by VPCI terms", "not available yet — formula not published (Buff Dormeier; the book's exact formula could not be checked and community versions differ)"),
    listed("hathaway_average_price", "Hathaway Average Price", MA, NO, "Pattern to Profit Hathaway Tools average of closes from a chosen start", "not available yet — formula not published (Hathaway Tools, Pattern to Profit)"),
    listed("ma_score", "Moving Average Score", TR, NO, "Moving Average Score", "not available yet — formula not published (platform not identified; no primary text found)"),
    listed("optex_bands", "Optex Bands (Optuma)", BC, NO, "Optuma Extremes: Market Profile based deviation from the point of balance", "not available yet — formula not published (Optuma)"),
    listed("percentage_trend", "Percentage Trend", TR, NO, "Modern Trading (2001): a trailing line K% from the highest/lowest close of a dynamic period up to Max_per", "not available yet — formula not published (Modern Trading magazine 2001; MotiveWave's pseudo-code is internally inconsistent)"),
    listed("prorealtime_bands", "ProRealTime Bands", BC, NO, "ProRealTime Bands", "not available yet — formula not published (ProRealTime)"),
    listed("relative_ma", "Relative Moving Average", MA, NO, "Relative Moving Average", "not available yet — formula not published (platform not identified; no primary text found)"),
    listed("volatility_profit_taker", "Volatility Profit Taker (Optuma)", VO, NO, "Optuma Berg tool: moving average plus an ATR multiple", "not available yet — formula not published (Optuma Berg tools)"),
    listed("super_swing_line", "Super Swing Line", TR, NO, "Super Swing Line", "not available yet — formula not published (platform not identified; no primary text found)"),
    listed("twiggs_trend_index", "Twiggs Trend Index", TR, NO, "Colin Twiggs proprietary indicator (Incredible Charts)", "not available yet — formula not published (Colin Twiggs, Incredible Charts)"),
    listed("twiggs_volatility", "Twiggs Volatility", VO, NO, "Colin Twiggs proprietary ATR-based volatility (Incredible Charts)", "not available yet — formula not published (Colin Twiggs, Incredible Charts)"),
    listed("vwar", "Volume Weighted Average Range", VO, NO, "TrendSpider (Donovan Wall): VWAP with bands at multiples of a volume-weighted average range", "not available yet — formula not published (TrendSpider, Donovan Wall; the anchor/reset rule for daily bars is not documented)"),
];
/// (id, platform names, AmiBroker names): one row per id in DEFS.
pub const SOURCES: &[(&str, &str, &str)] = &[
    ("trend_periods", "thinkorswim: TrendPeriods; TradeStation (TASC Traders' Tips 2004-04): TrendPeriods", ""),
    ("trend_quality", "thinkorswim: TrendQuality; MetaStock and AmiBroker (TASC Traders' Tips 2004-04): Q-indicator", ""),
    ("trend_noise_balance", "thinkorswim: TrendNoiseBalance; ProRealCode: B-indicator (trend noise balance); MetaStock: B-indicator", ""),
    ("rema", "TradeStation (TASC Traders' Tips 2003-07): RegEMA", ""),
    ("udma", "TradingView (PineCoders TASC 2023.04): Undersampled Double MA; thinkorswim: UndersampledDoubleMovAvg", ""),
    ("tmv", "thinkorswim: TMV; MotiveWave: Trend Momentum Volatility Volume (TMV); TradeStation (TASC Traders' Tips 2012-02): _TMV", ""),
    ("simple_trend_channel", "thinkorswim: SimpleTrendChannelFilter", ""),
    ("wilson_rpc", "Optuma: Wilson Relative Price Channel; TradeStation and AmiBroker (TASC Traders' Tips 2006-07): Wilson RelPriceChan", ""),
    ("new_market_paradigm", "MotiveWave: New Market Paradigm Contraction / Expansion; TradeStation (TASC Traders' Tips 1998-06): NewMarketParadigm", ""),
    ("mwdx", "MotiveWave: MWDX Average", ""),
    ("dgl_trade_bands", "Optuma: DGL Trade Bands", ""),
    ("bb_divergence", "thinkorswim: BBDivergence; eSignal and AmiBroker (TASC Traders' Tips 2015-12)", ""),
    ("atr_percentile_rank", "ProRealCode: ATR percentile rank", ""),
    ("trend_persistence_rate", "thinkorswim: TrendPersistenceRate; eSignal and TradersStudio (TASC Traders' Tips 2021-02)", ""),
    ("implied_volatility", "thinkorswim: ImpVolatility", ""),
    ("iv_percentile", "Implied Volatility Percentile (options platforms)", ""),
    ("iv_rank", "Implied Volatility Rank (options platforms)", ""),
    ("ma_rolling_high_accuracy", "Sierra Chart: Moving Average - Rolling High Accuracy", ""),
    ("long_haul_filter", "thinkorswim: LongHaulFilter", ""),
    ("gonogo_trend", "Optuma / StockCharts: GoNoGo Trend", ""),
    ("gonogo_squeeze", "Optuma / StockCharts: GoNoGo Squeeze", ""),
    ("dvan_smartlines", "Optuma: DVAN SmartLines", ""),
    ("dvan_long_term", "Optuma: DVAN Long Term", ""),
    ("dvan_trending", "Optuma: DVAN Trending", ""),
    ("dvan_trend_signals", "Optuma: DVAN Trend Signals", ""),
    ("prime_number_bands", "Prime Number Bands (Modulus Financial Engineering)", ""),
    ("valuation_lines", "Valuation Lines", ""),
    ("narrow_sideways_channel", "MotiveWave: Narrow Sideways Channel; MultiCharts: Narrow Sideways Channel", ""),
    ("relative_range_index", "thinkorswim: RelativeRangeIndex", ""),
    ("weakness_in_strong_trend", "thinkorswim: WeaknessInAStrongTrend", ""),
    ("swami_laguerre_trend", "thinkorswim: SwamiLaguerreTrend", ""),
    ("swami_volatility", "thinkorswim: SwamiVolatility", ""),
    ("trend_seeker", "Barchart: Trend Seeker", ""),
    ("chande_trend_meter", "StockCharts: Chande Trend Meter", ""),
    ("traffic_light", "StockCharts: Traffic Light", ""),
    ("chats", "TrendSpider: Chande Absolute Trend Strength", ""),
    ("avsl", "Optuma: Anti-Volume Stop Loss", ""),
    ("hathaway_average_price", "Optuma (Hathaway Tools): Hathaway Average Price", ""),
    ("ma_score", "Moving Average Score", ""),
    ("optex_bands", "Optuma: Optex Bands", ""),
    ("percentage_trend", "MotiveWave: Percentage Trend", ""),
    ("prorealtime_bands", "ProRealTime Bands", ""),
    ("relative_ma", "Relative Moving Average", ""),
    ("volatility_profit_taker", "Optuma: Volatility Profit Taker", ""),
    ("super_swing_line", "Super Swing Line", ""),
    ("twiggs_trend_index", "Incredible Charts: Twiggs Trend Index", ""),
    ("twiggs_volatility", "Incredible Charts: Twiggs Volatility", ""),
    ("vwar", "TrendSpider: Volume Weighted Average Range", ""),
];

#[cfg(test)]
mod tests {
    use crate::lab::testdata::{assert_series, closes, compiles, from_ohlc, go, walk};
    const N: f64 = f64::NAN;

    #[test] fn trend_quality_hand_trace_and_reset() {
        // fast 1 (= close), slow 2: ema2 1.5, 3.1667, 5.7222 -> periods +1 from bar 1; bar 4 (close 0) flips to −1
        let b = closes(&[1., 2., 4., 7., 0., -1.]);
        let kv = [("fast", 1.), ("slow", 2.), ("trend_length", 2.), ("noise_length", 2.), ("correction", 1.)];
        assert_series(&go("trend_periods", &b, &kv[..2])[0], &[N, 1., 1., 1., -1., -1.]);
        // bar 1 resets; bar 2 cpc 2, trend 4/3; bar 3 cpc 5, trend 34/9; dt 0, 2/3, 11/9
        let q = go("trend_quality", &b, &kv);
        let (n2, n3) = ((2f64 / 9.).sqrt(), (157f64 / 162.).sqrt());
        assert_series(&q[0][..4], &[N, N, (4. / 3.) / n2, (34. / 9.) / n3]);
        assert_eq!(q[0][4], 0.); // reversal bar: trend 0
        assert!(q[0][5] < 0.); // variant 0: falling closes give a negative trend
        assert!(go("trend_quality", &b, &[kv[0], kv[1], kv[2], kv[3], kv[4], ("variant", 1.)])[0][5] > 0.); // variant 1: change·periods
        // variant 1 alpha 1/2: trend 1, 3 on bars 2, 3; linear noise: sma(|dt|, 2)
        let v1 = go("trend_noise_balance", &b, &[kv[0], kv[1], kv[2], kv[3], kv[4], ("variant", 1.), ("noise_type", 1.)]);
        assert_series(&v1[0][2..4], &[100. * 1. / (1. + 0.5), 100. * 3. / (3. + 1.5)]);
    }
    #[test] fn rema_reduces_to_ema_and_regularizes() {
        let b = closes(&[1., 2., 3., 4., 5.]);
        assert_series(&go("rema", &b, &[("length", 3.), ("lambda", 0.)])[0], &[1., 2., 3., 3.5, 4.25]);
        assert_series(&go("rema", &b, &[("length", 3.), ("lambda", 1.)])[0], &[1., 2., 3., 3.75, 4.4375]);
    }
    #[test] fn udma_samples_and_hann_weights() {
        let b = closes(&[1., 2., 3., 4., 5.]); // samples 1, 1, 3, 3, 5
        let o = go("udma", &b, &[("sample", 2.), ("fast", 1.), ("slow", 2.)]);
        assert_series(&o[0], &[1., 1., 3., 3., 5.]);
        assert_series(&o[1], &[N, 1., 2., 3., 4.]); // L 2: weights 1.5, 1.5
        assert_series(&go("udma", &b, &[("sample", 1.), ("fast", 3.), ("slow", 3.)])[0], &[N, N, 2., 3., 4.]); // weights 1, 2, 1
    }
    #[test] fn tmv_follows_adx_and_sma() {
        let b = walk(200, 3, false);
        let (adx, sma) = (go("dmi", &b, &[("di_length", 10.), ("adx_smoothing", 10.)])[2].clone(), go("sma", &b, &[("length", 8.)])[0].clone());
        let o = go("tmv", &b, &[])[0].clone();
        for t in 20..200 {
            let want = if adx[t] > adx[t - 1] { if b.close[t] > sma[t] { 1. } else { -1. } } else { 0. };
            assert_eq!(o[t], want, "bar {t}");
        }
        assert!(o[..20].iter().all(|v| v.is_nan()));
    }
    #[test] fn simple_trend_channel_modes() {
        let mut c: Vec<f64> = (0..60).map(f64::from).collect();
        c.push(0.); // bar 60: below sma50 34.3, while sma20 47.5 is still above sma50
        let b = closes(&c);
        let min = go("simple_trend_channel", &b, &[("trend_check", 0.)]);
        assert_series(&min[0][49..], &[[1.; 11].as_slice(), &[0.]].concat());
        assert_eq!(min[1][60], 1.);
        assert_eq!(go("simple_trend_channel", &b, &[])[1][60], 0.); // Normal also needs sma20 < sma50
        assert!(go("simple_trend_channel", &b, &[("trend_check", 2.)])[0].iter().all(|v| v.is_nan())); // no sma200 yet
    }
    #[test] fn wilson_levels_on_a_rising_ramp() {
        let b = closes(&[1., 2., 3., 4., 5.]); // rsi 100 from bar 3
        let o = go("wilson_rpc", &b, &[("length", 3.)]);
        assert_series(&o[0], &[N, N, N, 4. * 0.7, 5. * 0.7]);
        assert_series(&o[1], &[N, N, N, 4. * 0.55, 5. * 0.55]);
        assert_series(&o[2], &[N, N, N, 4. * 0.45, 5. * 0.45]);
        assert_series(&o[3], &[N, N, N, 4. * 0.3, 5. * 0.3]);
    }
    #[test] fn new_market_paradigm_phases() {
        // length 2, population σ = |Δ|/2: bands m ± |Δ|: (15, −5), (12.5, 2.5), (5, 5), (27.5, −2.5), (21.5, 19.5), (24, 20)
        let o = go("new_market_paradigm", &closes(&[0., 10., 5., 5., 20., 21., 23.]), &[("length", 2.)]);
        assert_series(&o[0], &[N, N, 1., 1., 2., 1., 3.]);
    }
    #[test] fn mwdx_seeds_with_the_first_close() { assert_series(&go("mwdx", &closes(&[10., 20., 30.]), &[("factor", 0.5)])[0], &[10., 15., 22.5]); }
    #[test] fn dgl_trade_bands_hand_values() {
        let o = go("dgl_trade_bands", &from_ohlc(&[(10., 12., 8., 10.)]), &[]);
        assert_series(&o.iter().map(|s| s[0]).collect::<Vec<_>>(), &[12., 14., 8., 6.]);
    }
    #[test] fn bb_divergence_hand_value() {
        let mut b = closes(&[1., 3.]);
        b.second = Some(vec![2., 2.]); // flat second: bol 1
        let b1 = 1. + 3. / 4.0001; // m 2, σ 1
        assert_series(&go("bb_divergence", &b, &[("length", 2.)])[0], &[N, 100. * (1. - b1) / b1]);
        let d = crate::lab::catalogue::get("bb_divergence").unwrap();
        assert_eq!(crate::lab::catalogue::run(&d, &closes(&[1., 2.]), &Default::default()).unwrap_err(), "needs a second instrument");
    }
    #[test] fn atr_percentile_rank_counts_strictly_lower() {
        let b = from_ohlc(&[(10., 11., 9., 10.), (10., 10.5, 9.5, 10.), (10., 13., 7., 10.), (10., 11., 9., 10.)]); // tr 2, 1, 6, 2
        assert_series(&go("atr_percentile_rank", &b, &[("atr_length", 1.), ("lookback", 2.)])[0], &[N, N, 100., 50.]);
    }
    #[test] fn trend_persistence_rate_counts_slopes() {
        let kv = [("sma_length", 1.), ("tpr_length", 2.), ("mult", 1.), ("point", 1.), ("threshold", 0.5)];
        assert_series(&go("trend_persistence_rate", &closes(&[0., 1., 2., 2., 1.]), &kv)[0], &[N, N, 100., 50., 50.]);
    }
    #[test] fn listed_entries_refuse_with_their_note() {
        for d in super::DEFS.iter().filter(|d| d.compute.is_none()) {
            let e = crate::lab::catalogue::run(d, &walk(30, 1, true), &Default::default()).unwrap_err();
            assert!(e.starts_with("not available yet — "), "{}: {e}", d.id);
        }
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
    /// CENSUS_BARS=<csv date,open,high,low,close[,volume]> CENSUS_OUT=<json> cargo test --bin guru lab::x_census2_b::tests::census_b_dump -- --ignored --exact
    /// Default parameters; bb_divergence reads the open column as its second instrument.
    #[test] #[ignore] fn census_b_dump() {
        use crate::lab::{bars::Bars, catalogue::{run, Params}};
        let text = std::fs::read_to_string(std::env::var("CENSUS_BARS").expect("CENSUS_BARS")).expect("read CENSUS_BARS");
        let (mut b, mut vol) = (Bars::default(), Vec::new());
        for (i, line) in text.lines().skip(1).filter(|s| !s.trim().is_empty()).enumerate() {
            let f: Vec<&str> = line.split(',').map(str::trim).collect();
            let x = |k: usize| f[k].parse::<f64>().unwrap_or(f64::NAN);
            b.date.push(f[0].to_string());
            b.open.push(x(1)); b.high.push(x(2)); b.low.push(x(3)); b.close.push(x(4)); b.end.push(i);
            if f.len() > 5 { vol.push(x(5)); }
        }
        if vol.len() == b.len() { b.volume = Some(vol); }
        b.second = Some(b.open.clone());
        let num = |v: f64| if v.is_finite() { format!("{v:?}") } else { "null".into() };
        let body: Vec<String> = super::DEFS.iter().filter(|d| d.compute.is_some()).map(|d| {
            let out = run(d, &b, &Params::new()).unwrap_or_else(|e| panic!("{}: {e}", d.id));
            let cols: Vec<String> = d.outputs.iter().zip(&out).map(|(k, s)| format!("\"{k}\":[{}]", s.iter().map(|&v| num(v)).collect::<Vec<_>>().join(","))).collect();
            format!("\"{}\":{{{}}}", d.id, cols.join(","))
        }).collect();
        std::fs::write(std::env::var("CENSUS_OUT").expect("CENSUS_OUT"), format!("{{{}}}", body.join(","))).expect("write CENSUS_OUT");
    }
}
