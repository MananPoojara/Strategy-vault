//! Platform-survey gap group "b" (2026-11-02): momentum & oscillator titles the survey found missing from the lab.
//! Computed here from a cited public formula (see each definition string). Listed only, nothing invented: Divergence Index
//! (Storz's article not reachable; the two restatements disagree), Mobility Oscillator (Widner's article not reachable; the
//! only published code is internally inconsistent), Gain Loss Count (no primary text found) and Swami Stochastics (a heatmap
//! of 36 stochastics, display only). Not added, already in the catalogue: none of this group.
#![allow(dead_code)]
use super::bars::Bars;
use super::catalogue::{int, len, n, p, real, IndicatorDef, Needs, Output, ParamSpec, Params, Placement};
use super::osc2::rsi_like;
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
const M: &str = "Momentum";
const PN: Placement = Placement::Pane;
const NO: Needs = Needs::NONE;
const PAIR: Needs = Needs { second_symbol: true, ..Needs::NONE };
const UNPUB: &str = "Unpublished formulas";

fn div(a: f64, b: f64) -> f64 { if b == 0. || a.is_nan() || b.is_nan() { NA } else { a / b } }
fn zip(a: &[f64], b: &[f64], f: impl Fn(f64, f64) -> f64) -> Vec<f64> { a.iter().zip(b).map(|(&x, &y)| f(x, y)).collect() }
/// Wilder RSI of any series (the lab rsi, variant 0): rma of gains / losses of the 1-bar change, SMA seed.
fn rsi(x: &[f64], k: usize) -> Vec<f64> {
    let d = ta::change(x, 1);
    let up: Vec<f64> = d.iter().map(|v| if v.is_nan() { NA } else { v.max(0.) }).collect();
    let dn: Vec<f64> = d.iter().map(|v| if v.is_nan() { NA } else { (-v).max(0.) }).collect();
    rsi_like(&up, &dn, k)
}
/// Wilder ATR as TradingView ta.atr (bar 0 true range = high − low).
fn atr(b: &Bars, k: usize) -> Vec<f64> { ta::rma(&ta::tr(b, true), k) }
/// Raw stochastic %K = 100·(close − lowest low)/(highest high − lowest low) over k bars; a zero range gives no value.
fn stoch_k(b: &Bars, k: usize) -> Vec<f64> {
    let (h, l) = (ta::highest(&b.high, k), ta::lowest(&b.low, k));
    (0..b.len()).map(|i| div(100. * (b.close[i] - l[i]), h[i] - l[i])).collect()
}

const CT: &[ParamSpec] = &[len("length", "ATR length", 14.)];
fn change_over_true_range(b: &Bars, prm: &Params) -> Output {
    let a = atr(b, n(prm, "length"));
    Ok(vec![zip(&ta::change(&b.close, 1), &a, div)])
}

const CV: &[ParamSpec] = &[len("length", "Length", 5.)];
fn chartmill_value(b: &Bars, prm: &Params) -> Output {
    let k = n(prm, "length");
    let (f, v) = (ta::sma(&b.hl2(), k), ta::sma(&ta::tr(b, true), k));
    let rel = |x: &[f64]| -> Vec<f64> { (0..b.len()).map(|i| div(x[i] - f[i], v[i])).collect() };
    Ok(vec![rel(&b.open), rel(&b.high), rel(&b.low), rel(&b.close)])
}

const DT: &[ParamSpec] = &[len("r", "First EMA length", 14.), len("s", "Second EMA length", 10.), len("u", "Third EMA length", 5.)];
fn blau_dti(b: &Bars, prm: &Params) -> Output {
    let (h, l) = (&b.high, &b.low);
    let diff: Vec<f64> = (0..b.len()).map(|t| if t == 0 { NA } else { (h[t] - h[t - 1]).max(0.) - (l[t - 1] - l[t]).max(0.) }).collect();
    let abs: Vec<f64> = diff.iter().map(|v| v.abs()).collect();
    let e3 = |x: &[f64]| ta::ema(&ta::ema(&ta::ema(x, n(prm, "r")), n(prm, "s")), n(prm, "u"));
    Ok(vec![zip(&e3(&diff), &e3(&abs), |a, d| div(100. * a, d))])
}

/// Zamansky–Stendahl dynamic zones over the last `w` values: buy = the k-th smallest, sell = the k-th largest, k = ⌈prob·w⌉ (≥ 1).
fn dz(x: &[f64], w: usize, prob: f64) -> (Vec<f64>, Vec<f64>) {
    let k = ((prob * w as f64 - 1e-9).ceil() as usize).clamp(1, w);
    let (mut buy, mut sell) = (vec![NA; x.len()], vec![NA; x.len()]);
    for i in w.saturating_sub(1)..x.len() {
        let win = &x[i + 1 - w..=i];
        if win.iter().any(|v| !v.is_finite()) { continue; }
        let mut s = win.to_vec();
        s.sort_by(|a, c| a.total_cmp(c));
        (buy[i], sell[i]) = (s[k - 1], s[w - k]);
    }
    (buy, sell)
}
const DR: &[ParamSpec] = &[len("rsi_length", "RSI length", 9.), len("lookback", "Zone lookback", 70.), real("probability", "Zone probability", 0.1, 0.01, 0.5, 0.01)];
fn dz_rsi(b: &Bars, prm: &Params) -> Output {
    let r = rsi(&b.close, n(prm, "rsi_length"));
    let (buy, sell) = dz(&r, n(prm, "lookback"), p(prm, "probability"));
    Ok(vec![r, buy, sell])
}
const DS: &[ParamSpec] = &[len("stoch_length", "%R (stochastic) length", 9.), len("smooth", "WMA smoothing", 3.), len("lookback", "Zone lookback", 70.),
    real("probability", "Zone probability", 0.12, 0.01, 0.5, 0.01)];
fn dz_stoch(b: &Bars, prm: &Params) -> Output {
    let s = ta::wma(&stoch_k(b, n(prm, "stoch_length")), n(prm, "smooth"));
    let (buy, sell) = dz(&s, n(prm, "lookback"), p(prm, "probability"));
    Ok(vec![s, buy, sell])
}

/// fsk = 0.03·(mom − mom[1]) + 0.97·fsk[1], mom = x − x[k]; the recursion starts from 0 (MotiveWave's empty previous value).
fn fsk_s(x: &[f64], k: usize) -> Vec<f64> {
    let m = ta::change(x, k);
    let (mut o, mut prev) = (vec![NA; x.len()], 0.);
    for t in 1..x.len() {
        let d = m[t] - m[t - 1];
        if d.is_nan() { continue; }
        prev = 0.03 * d + 0.97 * prev;
        o[t] = prev;
    }
    o
}
const FK: &[ParamSpec] = &[len("length", "Momentum and signal length", 3.)];
fn fsk(b: &Bars, prm: &Params) -> Output {
    let k = n(prm, "length");
    let f = fsk_s(&b.close, k);
    let sig = ta::wma(&f, k);
    Ok(vec![f, sig])
}
const FS: &[ParamSpec] = &[len("length", "FSK momentum length", 3.), len("avg_length", "FSK WMA length", 6.), len("stoch_length", "Stochastic length (and its WMA)", 9.),
    len("signal_length", "Signal WMA length", 9.)];
fn fsst(b: &Bars, prm: &Params) -> Output {
    let v4 = ta::wma(&fsk_s(&b.close, n(prm, "length")), n(prm, "avg_length"));
    let k = n(prm, "stoch_length");
    let slowk = ta::wma(&stoch_k(b, k), k);
    let f = zip(&v4, &slowk, |a, s| 500. * a + s);
    let sig = ta::wma(&f, n(prm, "signal_length"));
    Ok(vec![f, sig])
}

const HR: &[ParamSpec] = &[len("length", "Linear regression length", 52.), len("lag", "Change lag", 26.), int("bars_per_year", "Bars per year (52 weekly, about 250 daily)", 52., 1., 400.)];
fn hull_roar(b: &Bars, prm: &Params) -> Output {
    let (k, scale) = (n(prm, "lag"), 100. * p(prm, "bars_per_year") / p(prm, "lag"));
    let lr = ta::linreg(&b.close, n(prm, "length"), 0);
    let ch = ta::change(&lr, k);
    Ok(vec![zip(&ch, &b.close, |d, c| div(scale * d, c))])
}

const MR: &[ParamSpec] = &[len("length", "Average length (52 weekly, 200 daily)", 200.)];
fn mansfield_rs(b: &Bars, prm: &Params) -> Output {
    let s = super::pairs::second(b)?;
    let rp: Vec<f64> = b.close.iter().zip(s).map(|(&c, &r)| if r > 0. { c / r } else { NA }).collect();
    let avg = ta::sma(&rp, n(prm, "length"));
    Ok(vec![zip(&rp, &avg, |x, a| div(x, a) * 100. - 100.)])
}

const MP: &[ParamSpec] = &[len("length", "RSI length", 3.)];
fn momentum_pinball(b: &Bars, prm: &Params) -> Output { Ok(vec![rsi(&ta::change(&b.close, 1), n(prm, "length"))]) }

const MO: &[ParamSpec] = &[int("length", "Lookback (pairs of consecutive changes)", 250., 2., 2000.)];
fn momersion(b: &Bars, prm: &Params) -> Output {
    let (c, k) = (&b.close, n(prm, "length"));
    let prod: Vec<f64> = (0..c.len()).map(|t| if t < 2 { NA } else { (c[t] - c[t - 1]) * (c[t - 1] - c[t - 2]) }).collect();
    let mut o = vec![NA; c.len()];
    for i in (k + 1)..c.len() {
        let w = &prod[i + 1 - k..=i];
        let (mc, mr) = (w.iter().filter(|v| **v > 0.).count(), w.iter().filter(|v| **v < 0.).count());
        if mc + mr > 0 { o[i] = 100. * mc as f64 / (mc + mr) as f64; }
    }
    Ok(vec![o])
}

const OS: &[ParamSpec] = &[len("length", "Length", 8.)];
fn oscar(b: &Bars, prm: &Params) -> Output {
    let rough = stoch_k(b, n(prm, "length"));
    let (mut o, mut prev) = (vec![NA; b.len()], 0.);
    for t in 0..b.len() {
        if rough[t].is_nan() { continue; } // zero range: no value, the recursion keeps its last state
        prev = prev / 3. * 2. + rough[t] / 3.;
        o[t] = prev;
    }
    Ok(vec![o])
}

const CR: &[ParamSpec] = &[len("rsi_length", "RSI length", 2.), len("sum_length", "Days summed", 2.)];
fn cumulative_rsi(b: &Bars, prm: &Params) -> Output { Ok(vec![ta::sum(&rsi(&b.close, n(prm, "rsi_length")), n(prm, "sum_length"))]) }

const SR: &[ParamSpec] = &[len("ema_length", "EMA length", 13.), len("roc_length", "Rate-of-change length", 21.)];
fn sroc(b: &Bars, prm: &Params) -> Output {
    let (e, k) = (ta::ema(&b.close, n(prm, "ema_length")), n(prm, "roc_length"));
    Ok(vec![(0..e.len()).map(|t| if t >= k { div(e[t], e[t - k]) * 100. - 100. } else { NA }).collect()])
}

const TM: &[ParamSpec] = &[len("fast_length", "Fast SMA length", 25.), len("slow_length", "Slow SMA length", 200.)];
fn tfs_mbo(b: &Bars, prm: &Params) -> Output { Ok(vec![zip(&ta::sma(&b.close, n(prm, "fast_length")), &ta::sma(&b.close, n(prm, "slow_length")), |f, s| f - s)]) }

const VB: &[ParamSpec] = &[len("length", "Change length", 22.), len("atr_length", "ATR length", 65.)];
fn vbm(b: &Bars, prm: &Params) -> Output { Ok(vec![zip(&ta::change(&b.close, n(prm, "length")), &atr(b, n(prm, "atr_length")), div)]) }

const PG: &[ParamSpec] = &[len("length", "Average length", 14.)];
fn williams_progo(b: &Bars, prm: &Params) -> Output {
    let k = n(prm, "length");
    let pro: Vec<f64> = (0..b.len()).map(|t| b.close[t] - b.open[t]).collect();
    let public: Vec<f64> = (0..b.len()).map(|t| if t == 0 { NA } else { b.open[t] - b.close[t - 1] }).collect();
    Ok(vec![ta::sma(&pro, k), ta::sma(&public, k)])
}

pub const DEFS: &[IndicatorDef] = &[
    def("change_over_true_range", "Change Over True Range (Barchart)", M, CT, &["cotr"], PN, NO, "Barchart cmdtyView Change Over True Range: cotr = (close − previous close)/ATR(length), each bar's net change in ATR units; ATR = Wilder's smoothing (rma, SMA seed) of the true range, bar 0's true range = high − low (TradingView ta.atr), Barchart's default smoothing; a zero ATR gives no value; default length 14; first value at bar length − 1. Adaptation: Barchart's other smoothings (simple, exponential, weighted) and its ATRADJ option (an ATR from closing prices only, formula not given) are not offered. Source: Barchart cmdtyView wiki, Change Over True Range (https://barchartsolutions.atlassian.net/wiki/spaces/CMDTYV/pages/1094222055/Change+Over+True+Range).",
        &[13], false, change_over_true_range),
    def("chartmill_value", "Chartmill Value Indicator (Vandycke)", M, CV, &["open", "high", "low", "close"], PN, NO, "Vandycke (S&C January 2013), as MotiveWave's Chartmill Value Indicator code: f = sma(hl2, length); v = sma(true range, length); each output = (price − f)/v for open, high, low and close (MotiveWave draws them as a price bar in a separate pane); the true range of bar 0 is high − low; a zero v gives no value; default length 5 (MotiveWave), simple averages; first value at bar length − 1. Not ProRealCode's 'Modified' CVI, which also divides by √length. Sources: MotiveWave studies C–D, Chartmill Value Indicator (https://docs.motivewave.com/studies/c-d); Linn Software, Chartmill Value Indicator (https://www.linnsoft.com/techind/chartmill-value-indicator-rtx).",
        &[4, 4, 4, 4], false, chartmill_value),
    def("blau_dti", "Directional Trend Index (Blau, DTI)", M, DT, &["dti"], PN, NO, "Blau (Momentum, Direction, and Divergence, 1995), as MotiveWave's Directional Trend Index code: hmu = high − previous high when positive, else 0; lmd = previous low − low when positive, else 0; diff = hmu − lmd; dti = 100·ema(ema(ema(diff, r), s), u)/ema(ema(ema(|diff|, r), s), u), −100..100; EMAs alpha 2/(n + 1) seeded with the SMA of their first n values; a zero denominator gives no value; defaults r 14, s 10, u 5; guide lines ±25; first value at bar r + s + u − 2 (diff starts on bar 1). Source: MotiveWave studies C–D, Directional Trend Index (https://docs.motivewave.com/studies/c-d).",
        &[27], false, blau_dti),
    def("dz_rsi", "Dynamic Zone RSI (Zamansky & Stendahl)", M, DR, &["rsi", "buy_zone", "sell_zone"], PN, NO, "Zamansky & Stendahl, Dynamic Zones (S&C V15:7, 1997, pp. 306–310): over the last lookback values of the indicator, buy zone V with P{X ≤ V} = probability and sell zone V with P{X ≥ V} = probability from the empirical distribution, i.e. with k = ⌈probability·lookback⌉ (at least 1) the buy zone is the k-th smallest and the sell zone the k-th largest value (the article's worked example: 80 values, probability 0.1, the zone is where the counted frequencies from the top reach 8); the window includes the current bar; indicator = RSI(close, rsi_length) (lab rsi: Wilder, SMA seed); defaults are the article's RSI test: RSI 9, lookback 70, probability 10 % on both sides; a window with a missing value gives no zone; rsi from bar rsi_length, zones from bar rsi_length + lookback − 1. Sources: the article (copy at https://c.mql5.com/forextsd/forum/165/zamansky.stendahl_-_dynamic_zones_e.pdf).",
        &[9, 78, 78], false, dz_rsi),
    def("dz_stoch", "Dynamic Zone Stochastic (Zamansky & Stendahl DZ %R)", M, DS, &["stoch", "buy_zone", "sell_zone"], PN, NO, "Zamansky & Stendahl, Dynamic Zones (S&C V15:7, 1997, pp. 306–310), the article's DZ %R system: stoch = WAverage(PercentR(9), 3) = wma(100·(close − lowest low)/(highest high − lowest low) over stoch_length, smooth) (TradeStation's PercentR is the 0..100 form, the fast stochastic %K); zones as dz_rsi: with k = ⌈probability·lookback⌉ (at least 1) the buy zone is the k-th smallest and the sell zone the k-th largest of the last lookback values, current bar included; defaults are the article's system: 9, 3, lookback 70, probability 12 %; a zero range or a missing value in the window gives no value; stoch from bar stoch_length + smooth − 2, zones from there + lookback − 1. Sources: the article (copy at https://c.mql5.com/forextsd/forum/165/zamansky.stendahl_-_dynamic_zones_e.pdf).",
        &[10, 79, 79], false, dz_stoch),
    def("fsk", "Fast and Slow Kurtosis (FSK)", M, FK, &["fsk", "signal"], PN, NO, "U. Jenson (Omega Research), as MotiveWave's Fast And Slow Kurtosis code: mom = close − close[length]; fsk = 0.03·(mom − mom[1]) + 0.97·fsk[1]; signal = wma(fsk, length); the recursion starts from fsk = 0 (an empty previous value) on the first bar with mom and mom[1] (bar length + 1); default length 3 (momentum and signal); first values at bars length + 1 and 2·length. Source: MotiveWave studies E–F, Fast And Slow Kurtosis (https://docs.motivewave.com/studies/e-f).",
        &[4, 6], false, fsk),
    def("fsst", "Fast and Slow Stochastic (FSST)", M, FS, &["fsst", "signal"], PN, NO, "U. Jenson (Omega Research, 1997), as MotiveWave's Fast And Slow Stochastics code: fsk as the fsk entry (momentum length 3, recursion from 0); v4 = wma(fsk, avg_length); fastk = 100·(close − lowest low)/(highest high − lowest low) over stoch_length; slowk = wma(fastk, stoch_length); fsst = 500·v4 + slowk; signal = wma(fsst, signal_length); a zero range gives no %K on that bar; defaults 3, 6, 9, 9 (weighted averages, MotiveWave's default method); first values at bars 2·stoch_length − 2 (16) and that + signal_length − 1 (24). Source: MotiveWave studies E–F, Fast And Slow Stochastics (https://docs.motivewave.com/studies/e-f).",
        &[16, 24], false, fsst),
    def("hull_roar", "Hull Rate of Annual Return (ROAR)", M, HR, &["roar"], PN, NO, "Alan Hull's ROAR (Active Investing): the annualised rise of a smoothed price over a shorter sample, divided by the current price, as Marketcalls' AmiBroker code 200·(LinearReg(C, 52) − Ref(LinearReg(C, 52), −26))/C for weekly bars: roar = 100·(bars_per_year/lag)·(linreg(close, length) − linreg(close, length)[lag])/close, linreg = the least-squares end point; defaults length 52, lag 26, bars_per_year 52 (weekly bars, as Optuma says the tool is calibrated for; about 250 for daily bars); the AFL's cutoff (values below it drawn as 0) is a display filter and not applied; a zero close gives no value; first value at bar length + lag − 1. Hull's own page (alanhull.com/hull-roar-indicator) gives the idea, not the lengths. Sources: Marketcalls, Hull ROAR AmiBroker AFL (https://www.marketcalls.in/?p=44950; code https://gist.github.com/marketcalls/0186910c1fced1fcbf1f45f2655519bd); Optuma KB, Hull Rate of Annual Return (https://www.optuma.com/kb/optuma/tools/hull/hull-rate-of-annual-return).",
        &[77], false, hull_roar),
    def("mansfield_rs", "Mansfield Relative Strength", M, MR, &["mrs"], PN, PAIR, "Mansfield relative performance (Weinstein, Secrets for Profiting in Bull and Bear Markets, 1988, ratio of the stock to the market average, normalised by Mansfield Charts): rp = close/close of the second instrument; mrs = 100·(rp/sma(rp, length) − 1); zero line = the ratio's moving average; a second-instrument close ≤ 0 gives no value; default length 200 (daily bars; 52 on weekly bars, as Stage Analysis); first value at bar length − 1. Source: Stage Analysis, How to create the Mansfield Relative Performance indicator (https://www.stageanalysis.net/blog/4266/how-to-create-the-mansfield-relative-performance-indicator).",
        &[199], false, mansfield_rs),
    def("momentum_pinball", "Momentum Pinball (Raschke, LBR/RSI)", M, MP, &["pinball"], PN, NO, "Connors & Raschke, Street Smarts (1995), Momentum Pinball: a 3-period RSI of the one-period rate of change taken as the daily net change (close − previous close); RSI = the lab rsi (Wilder, SMA seed) applied to that change series; below 30 a buy day, above 70 a sell day; default length 3; first value at bar length + 1. Sources: the book; MQL5 article 2825 (https://www.mql5.com/en/articles/2825); eSignal forum LBR/RSI thread (https://forum.esignal.com/forum/efs-development/efs-new-study-suggestions/27498-looking-for-simple-study-lbr-rsi-does-it-already-exist).",
        &[4], false, momentum_pinball),
    def("momersion", "Momersion (Harris)", M, MO, &["momersion"], PN, NO, "Harris, Momersion Indicator (Price Action Lab blog, 13 August 2015): over the last length pairs of consecutive close-to-close changes, MC = the number with r(t)·r(t − 1) > 0 (momentum), MRC = the number < 0 (mean reversion); momersion = 100·MC/(MC + MRC), 0..100, above 50 momentum dominates; the sign of a change equals the sign of its return, so price differences are used (as QuantConnect LEAN's Momersion); zero products are not counted; MC + MRC = 0 gives no value (LEAN returns 50 there and when half the products are 0); the lab default 250 is a choice (the post's code is paywalled; LEAN's example uses 20); first value at bar length + 1. Sources: QuantConnect docs, Momersion (https://www.quantconnect.com/docs/v2/writing-algorithms/indicators/supported-indicators/momersion); LEAN source (https://github.com/QuantConnect/Lean/blob/master/Indicators/Momersion.cs).",
        &[251], false, momersion),
    def("oscar", "Oscar Indicator", M, OS, &["oscar"], PN, NO, "MotiveWave's Oscar code: rough = 100·(close − lowest low)/(highest high − lowest low) over length; oscar = oscar[1]·2/3 + rough/3, from oscar = 0 (an empty previous value) on the first bar with rough; a zero range gives no value on that bar and the recursion keeps its last state; guide lines 30/70; default length 8; first value at bar length − 1. Source: MotiveWave studies O–P, Oscar (https://docs.motivewave.com/studies/o-p).",
        &[7], false, oscar),
    def("cumulative_rsi", "Cumulative RSI (Connors)", M, CR, &["cum_rsi"], PN, NO, "Connors & Alvarez, Short Term Trading Strategies That Work (2008), ch. 9 pp. 66–68: \"Use a 2-Period RSI. Take the past X days of the 2-period RSI and add them up\" (X = 2); cum_rsi = sum of rsi(close, rsi_length) over sum_length bars (lab rsi: Wilder, SMA seed); 0..100·sum_length; defaults 2 and 2 (the book's rules buy below 35 or 50); first value at bar rsi_length + sum_length − 1. Source: the book (as quoted in the vault's ststw_cum_rsi_35 rule).",
        &[3], false, cumulative_rsi),
    def("sroc", "Smoothed Rate of Change (S-ROC)", M, SR, &["sroc"], PN, NO, "Elder, Trading for a Living (1993) pp. 146–147: \"calculate a 13-day exponential moving average of closing prices and then apply a 21-day Rate of Change to it\"; sroc = 100·(ema(close, ema_length)/ema[roc_length] − 1), zero-centred (Elder's ratio form with centreline 1 is 1 + sroc/100); EMA alpha 2/(n + 1) seeded with the SMA of its first n values; defaults 13, 21; first value at bar ema_length + roc_length − 1. MotiveWave's Smoothed Rate of Change also draws an EMA signal line of unstated length; not an output here. Sources: the book (as quoted in the vault's eltfl_sroc rule); MotiveWave studies S–T, Smoothed Rate of Change (https://docs.motivewave.com/studies/s-t).",
        &[33], false, sroc),
    def("tfs_mbo", "TFS MBO Indicator (Strain)", M, TM, &["mbo"], PN, NO, "Strain & Whitley, TFS trading system (S&C Traders' Tips, June 2000): mbo = sma(close, 25) − sma(close, 200), the system's third component; defaults 25 and 200 simple averages (MotiveWave's TFS MOB Indicator); first value at bar slow_length − 1. Sources: S&C Traders' Tips June 2000 (https://www.traders.com/Documentation/FEEDbk_docs/2000/06/TradersTips/TradersTips.html); MotiveWave studies S–T, TFS MOB Indicator (https://docs.motivewave.com/studies/s-t).",
        &[199], false, tfs_mbo),
    def("vbm", "Volatility-Based Momentum (VBM)", M, VB, &["vbm"], PN, NO, "VBM: rate of change in volatility units, vbm = (close − close[length])/ATR(atr_length); 1.0 = a move of one average true range; ATR = Wilder's smoothing of the true range (TradingView ta.atr, SMA seed, bar 0 = high − low); a zero ATR gives no value; defaults 22 and 65 (the script's daily example VBM(22, 65)); first value at bar max(length, atr_length − 1). Sources: SteveFromAZ, TradingView open-source Volatility-Based Momentum (https://www.tradingview.com/script/g0XVuxKc-Volatility-Based-Momentum-VBM/); LuxAlgo library (https://www.luxalgo.com/library/indicator/g0XVuxKc-volatility-based-momentum-vbm/).",
        &[64], false, vbm),
    def("williams_progo", "Williams ProGo", M, PG, &["pro", "public"], PN, NO, "Larry Williams' ProGo: professional line = sma(close − open, length), the change from today's open to today's close; public line = sma(open − previous close, length), the change from yesterday's close to today's open; default 14; pro from bar length − 1, public from bar length. Williams' own text was not reachable; this is the construction both community ports describe. Sources: TradingView open-source William's ProGo indicator (https://www.tradingview.com/script/ISkKvK49-William-s-ProGo-indicator/); useThinkScript, Larry Williams \"Pro-Go\" Indicator (https://usethinkscript.com/threads/larry-williams-pro-go-indicator-for-thinkorswim.4377/).",
        &[13, 14], false, williams_progo),
    listed("divergence_index", "Divergence Index (Storz)", UNPUB, NO, "Storz, Quantifying Divergence with the Divergence Index (S&C V14:1, January 1996): a volatility-adjusted difference of a fast and a slow moving average; listed only",
        "not available yet — formula not published (Storz's article not reachable; Optuma gives only 10/40 averages and a standard deviation of changes, a GPL R port uses (ema 5 − ema 20)/stdev(1-bar momentum, 10)²; the two disagree)"),
    listed("mobility_oscillator", "Mobility Oscillator (Widner)", UNPUB, NO, "Widner (S&C February 1996): compares the prior close with the peak of a price distribution of the bars' high-low ranges over m bins of the period's range; listed only",
        "not available yet — formula not published (Widner's article not reachable; MotiveWave's condensed code is internally inconsistent: it reads close[period − 1] as the prior close and mixes the loop's low with the bar's low)"),
    listed("gain_loss_count", "Gain Loss Count", UNPUB, NO, "a count of gaining and losing bars; no primary text defines it; listed only", "not available yet — formula not published (no primary source found)"),
    listed("swami_stochastics", "Swami Stochastics (Ehlers & Way)", "Drawings & display", NO, "Ehlers & Way SwamiCharts: a heatmap of stochastics for every length 12..47 (MotiveWave code: num = (close − lowest low + num[1])/2, denom = (highest high − lowest low + denom[1])/2, stoch = 0.2·num/denom + 0.8·stoch[1], coloured red..green); one colour cell per length, not one series; listed only",
        "chart drawing/display — not a bar-by-bar series; not usable in rules"),
];
/// (id, platform name, AmiBroker name), one per entry.
pub const SOURCES: &[(&str, &str, &str)] = &[
    ("change_over_true_range", "Barchart: Change Over True Range", ""),
    ("chartmill_value", "MotiveWave: Chartmill Value Indicator", ""),
    ("blau_dti", "MotiveWave: Directional Trend Index", ""),
    ("dz_rsi", "census: Dynamic Zone RSI", ""),
    ("dz_stoch", "census: Dynamic Zone Stochastic", ""),
    ("fsk", "MotiveWave: Fast And Slow Kurtosis", ""),
    ("fsst", "MotiveWave: Fast And Slow Stochastics", ""),
    ("hull_roar", "Optuma: Hull Rate of Annual Return", ""),
    ("mansfield_rs", "census: Mansfield Relative Strength", ""),
    ("momentum_pinball", "census: Momentum Pinball", ""),
    ("momersion", "QuantConnect: Momersion", ""),
    ("oscar", "MotiveWave: Oscar", ""),
    ("cumulative_rsi", "census: Cumulative RSI", ""),
    ("sroc", "MotiveWave: Smoothed Rate of Change", ""),
    ("tfs_mbo", "MotiveWave: TFS MOB Indicator", ""),
    ("vbm", "TradingView (community): Volatility-Based Momentum (VBM)", ""),
    ("williams_progo", "TradingView (community): William's ProGo indicator", ""),
    ("divergence_index", "Optuma: Divergence Index", ""),
    ("mobility_oscillator", "MotiveWave: Mobility Oscillator", ""),
    ("gain_loss_count", "census: Gain Loss Count", ""),
    ("swami_stochastics", "MotiveWave: Swami Stochastics", ""),
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lab::catalogue::{get, run};
    use crate::lab::testdata::{assert_series, closes, compiles, from_ohlc, go};
    const N: f64 = f64::NAN;

    #[test] fn change_over_true_range_and_vbm_divide_by_wilder_atr() {
        // true range 2, 2, 3; atr(2) = 2, then 2 + (3 − 2)/2 = 2.5
        let b = from_ohlc(&[(10., 11., 9., 10.), (10., 12., 10., 11.), (11., 14., 11., 13.)]);
        assert_series(&go("change_over_true_range", &b, &[("length", 2.)])[0], &[N, 0.5, 0.8]);
        assert_series(&go("vbm", &b, &[("length", 2.), ("atr_length", 2.)])[0], &[N, N, 1.2]);
    }
    #[test] fn chartmill_value_centres_on_the_midpoint_average() {
        let b = from_ohlc(&[(10., 11., 9., 10.), (10., 12., 10., 11.), (11., 14., 11., 13.)]);
        let o = go("chartmill_value", &b, &[("length", 2.)]);
        // f = 10.5, 11.75; v = 2, 2.5
        assert_series(&o[3], &[N, 0.25, 0.5]);
        assert_series(&o[1], &[N, 0.75, 0.9]);
        assert_series(&o[0], &[N, -0.25, -0.3]);
    }
    #[test] fn blau_dti_triple_smoothed_ratio() {
        // diff = [_, 1, −2, 2]; ema2: −0.5, 7/6; |diff| ema2: 1.5, 11/6
        let b = from_ohlc(&[(10., 11., 9., 10.), (10., 12., 10., 10.), (10., 11., 8., 10.), (10., 13., 9., 10.)]);
        assert_series(&go("blau_dti", &b, &[("r", 2.), ("s", 1.), ("u", 1.)])[0], &[N, N, -100. / 3., 700. / 11.]);
    }
    #[test] fn dynamic_zones_follow_the_article_example() {
        // S&C V15:7 example: 80 values, probability 0.1 → sell zone 19 (top counts 1 + 3 + 4 reach 8); buy zone: 1 + 3 + 7 ≥ 8 → 6
        let mut x = Vec::new();
        for (v, k) in [(1., 1), (3., 3), (6., 7), (9., 16), (11., 24), (13., 13), (15., 8), (19., 4), (20., 3), (21., 1)] { x.extend(std::iter::repeat_n(v, k)); }
        assert_eq!(x.len(), 80);
        let (buy, sell) = dz(&x, 80, 0.1);
        assert_eq!((buy[79], sell[79]), (6., 19.));
        assert!(buy[78].is_nan());
        // RSI(1) = 100 on a rise, 0 on a fall
        let o = go("dz_rsi", &closes(&[1., 2., 1., 2., 3.]), &[("rsi_length", 1.), ("lookback", 3.), ("probability", 0.3)]);
        assert_series(&o[0], &[N, 100., 0., 100., 100.]);
        assert_series(&o[1], &[N, N, N, 0., 0.]);
        assert_series(&o[2], &[N, N, N, 100., 100.]);
    }
    #[test] fn dz_stoch_is_smoothed_percent_r_with_zones() {
        let b = from_ohlc(&[(5., 10., 0., 5.), (5., 10., 0., 10.), (5., 10., 0., 0.), (5., 10., 0., 2.5)]);
        let o = go("dz_stoch", &b, &[("stoch_length", 1.), ("smooth", 1.), ("lookback", 2.), ("probability", 0.5)]);
        assert_series(&o[0], &[50., 100., 0., 25.]);
        assert_series(&o[1], &[N, 50., 0., 0.]);
        assert_series(&o[2], &[N, 100., 100., 25.]);
        assert_series(&go("dz_stoch", &b, &[("stoch_length", 1.), ("smooth", 2.), ("lookback", 2.)])[0], &[N, 250. / 3., 100. / 3., 50. / 3.]);
    }
    #[test] fn fsk_and_fsst_feedback_from_zero() {
        // length 1: mom = 1, 2, 3 → mom change 1, 1 → fsk 0.03, 0.03 + 0.97·0.03
        let c = closes(&[1., 2., 4., 7.]);
        assert_series(&go("fsk", &c, &[("length", 1.)])[0], &[N, N, 0.03, 0.0591]);
        // length 2: mom = 3, 5, 7 → fsk 0.06, 0.1182; signal wma2 = (0.06 + 2·0.1182)/3
        let o = go("fsk", &closes(&[1., 2., 4., 7., 11.]), &[("length", 2.)]);
        assert_series(&o[0], &[N, N, N, 0.06, 0.1182]);
        assert_series(&o[1], &[N, N, N, N, (0.06 + 2. * 0.1182) / 3.]);
        // high = close + 1, low = close − 1 → %K(1) = 50 every bar
        let b = from_ohlc(&[(1., 2., 0., 1.), (2., 3., 1., 2.), (4., 5., 3., 4.), (7., 8., 6., 7.)]);
        let ones = [("length", 1.), ("avg_length", 1.), ("stoch_length", 1.), ("signal_length", 1.)];
        assert_series(&go("fsst", &b, &ones)[0], &[N, N, 65., 500. * 0.0591 + 50.]);
    }
    #[test] fn hull_roar_annualises_the_regression_change() {
        // linreg of 2 points ends on the close; 100·(2/1)·(4 − 2)/4
        assert_series(&go("hull_roar", &closes(&[1., 2., 4.]), &[("length", 2.), ("lag", 1.), ("bars_per_year", 2.)])[0], &[N, N, 100.]);
    }
    #[test] fn mansfield_normalises_the_ratio_by_its_average() {
        let mut b = closes(&[10., 20., 30.]);
        b.second = Some(vec![10., 10., 10.]);
        assert_series(&go("mansfield_rs", &b, &[("length", 2.)])[0], &[N, 100. / 3., 20.]);
        assert_eq!(run(&get("mansfield_rs").unwrap(), &closes(&[1., 2.]), &Params::new()).unwrap_err(), "needs a second instrument");
    }
    #[test] fn momentum_pinball_is_rsi_of_the_daily_change() {
        // change = 1, 2, 1, 2 → its change +1, −1, +1 → RSI(1) 100, 0, 100
        assert_series(&go("momentum_pinball", &closes(&[1., 2., 4., 5., 7.]), &[("length", 1.)])[0], &[N, N, 100., 0., 100.]);
    }
    #[test] fn momersion_counts_same_sign_pairs() {
        // changes +1 +1 +1 −1 −1: products + + − + → windows of 2: 100, 50, 50
        assert_series(&go("momersion", &closes(&[1., 2., 3., 4., 3., 2.]), &[("length", 2.)])[0], &[N, N, N, 100., 50., 50.]);
    }
    #[test] fn oscar_smooths_the_raw_stochastic_from_zero() {
        let b = from_ohlc(&[(0., 10., 0., 5.), (0., 10., 0., 10.), (0., 12., 2., 2.)]);
        assert_series(&go("oscar", &b, &[("length", 2.)])[0], &[N, 100. / 3., 250. / 9.]);
    }
    #[test] fn cumulative_rsi_sums_rsi() {
        assert_series(&go("cumulative_rsi", &closes(&[1., 2., 1., 2., 3.]), &[("rsi_length", 1.)])[0], &[N, N, 100., 100., 200.]);
    }
    #[test] fn sroc_is_rate_of_change_of_the_ema() {
        assert_series(&go("sroc", &closes(&[1., 2., 4.]), &[("ema_length", 1.), ("roc_length", 1.)])[0], &[N, 100., 100.]);
        // ema2 = 1.5, 19/6 → 100·(19/9 − 1)
        assert_series(&go("sroc", &closes(&[1., 2., 4.]), &[("ema_length", 2.), ("roc_length", 1.)])[0], &[N, N, 1000. / 9.]);
    }
    #[test] fn tfs_mbo_and_progo_average_differences() {
        assert_series(&go("tfs_mbo", &closes(&[1., 3., 5.]), &[("fast_length", 1.), ("slow_length", 2.)])[0], &[N, 1., 1.]);
        // close − open = 1, −1, 3; open − previous close = 1, −1
        let b = from_ohlc(&[(10., 12., 9., 11.), (12., 13., 10., 11.), (10., 14., 9., 13.)]);
        let o = go("williams_progo", &b, &[("length", 2.)]);
        assert_series(&o[0], &[N, 0., 1.]);
        assert_series(&o[1], &[N, N, 0.]);
    }
    #[test] fn listed_entries_refuse_with_their_note() {
        for d in DEFS.iter().filter(|d| d.compute.is_none()) {
            assert_eq!(run(&get(d.id).unwrap(), &closes(&[1., 2., 3.]), &Params::new()).unwrap_err(), d.note);
        }
        for d in DEFS { assert_eq!(SOURCES.iter().filter(|s| s.0 == d.id).count(), 1, "{}", d.id); }
        assert_eq!(SOURCES.len(), DEFS.len());
    }
    #[test] fn rules_reach_gap_b() {
        for d in DEFS.iter().filter(|d| d.compute.is_some() && !d.needs.second_symbol) {
            for o in d.outputs { compiles(d.id, o).unwrap_or_else(|e| panic!("{}.{o}: {e:?}", d.id)); }
        }
    }
}
