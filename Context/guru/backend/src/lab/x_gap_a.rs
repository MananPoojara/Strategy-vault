//! Gap list "a" (2026-11-02): trend, moving-average and stop items from the external platform survey that the lab did not have.
//! Computed: Congestion Index (Katsanos), Didi Index, Drawdown Trailing Stop (Hull), Four Percent Model (Davis/Zweig), Gann Swing
//! Chart (causal), Geometric MA, IE/2 (Tillson), LBR Paint Bars, QEMA, Percentage Trailing Stop, SafeZone Stop (Elder),
//! Trend Analysis Index (White) and a causal High/Low Trailing Stop. Listed only: Countback Line and Fixed Days High/Low Stop
//! (drawings anchored at a bar the user picks), Hart Swing Chart and X-Bar Trailing Stop (formulas not published).
//! Not added, already in the catalogue: TFS Tether Line = donchian.basis with length 50 (MotiveWave: tl = (highest(high, 50) +
//! lowest(low, 50))/2). The master list's "Tillson IE/2" row is the same indicator as its "IE/2" row (tillson_ie2 here).
//! Formulas: see each definition string.
#![allow(dead_code)]
use super::bars::Bars;
use super::catalogue::{int, len, n, p, real, IndicatorDef, Needs, Output, ParamSpec, Params, Placement};
use super::ta::{self, NA};

#[allow(clippy::too_many_arguments)]
const fn def(id: &'static str, name: &'static str, category: &'static str, params: &'static [ParamSpec], outputs: &'static [&'static str], placement: Placement,
             needs: Needs, definition: &'static str, first: &'static [usize], sparse: bool, f: fn(&Bars, &Params) -> Output) -> IndicatorDef {
    IndicatorDef { id, name, category, params, outputs, placement, needs, definition, first, sparse, compute: Some(f), note: "" }
}
/// Listed in the picker, not computed (the app shows the note).
const fn listed(id: &'static str, name: &'static str, category: &'static str, definition: &'static str, note: &'static str) -> IndicatorDef {
    IndicatorDef { id, name, category, params: &[], outputs: &["value"], placement: Placement::Pane, needs: Needs::NONE, definition,
                   first: &[usize::MAX], sparse: false, compute: None, note }
}
const TR: &str = "Trend";
const MA: &str = "Moving averages";
const PR: Placement = Placement::Price;
const PN: Placement = Placement::Pane;
const NO: Needs = Needs::NONE;
const X: usize = usize::MAX;

/// Highest / lowest of the full window x[i + 1 − k ..= i]; NaN unless all k values exist.
fn full_ext(x: &[f64], k: usize, max: bool) -> Vec<f64> {
    (0..x.len()).map(|i| {
        if i + 1 < k { return NA; }
        let w = &x[i + 1 - k..=i];
        if w.iter().any(|v| v.is_nan()) { return NA; }
        if max { w.iter().cloned().fold(f64::MIN, f64::max) } else { w.iter().cloned().fold(f64::MAX, f64::min) }
    }).collect()
}
/// Index of the highest (max) or lowest value of x[from..=to]; ties go to the most recent bar.
fn arg_ext(x: &[f64], from: usize, to: usize, max: bool) -> usize {
    let mut a = from;
    for j in from..=to { if (max && x[j] >= x[a]) || (!max && x[j] <= x[a]) { a = j; } }
    a
}

const CI: &[ParamSpec] = &[int("length", "Days in congestion", 15., 2., 500.)];
fn congestion_index(b: &Bars, prm: &Params) -> Output {
    let (k, c) = (n(prm, "length"), &b.close);
    let (hh, ll) = (ta::highest(&b.high, k), ta::lowest(&b.low, k));
    let raw: Vec<f64> = (0..b.len()).map(|t| if t + 1 < k { NA } else {
        let roc = 100. * (c[t] - c[t + 1 - k]) / c[t + 1 - k];
        roc / ((hh[t] - ll[t]) / (ll[t] + 0.01) + 0.000001)
    }).collect();
    Ok(vec![ta::ema(&raw, 3), raw])
}

const DI: &[ParamSpec] = &[len("fast", "Fast SMA", 3.), len("mid", "Middle SMA (zero line)", 8.), len("slow", "Slow SMA", 20.)];
fn didi_index(b: &Bars, prm: &Params) -> Output {
    let m = ta::sma(&b.close, n(prm, "mid"));
    let r = |k: usize| -> Vec<f64> { ta::sma(&b.close, k).iter().zip(&m).map(|(a, d)| a / d - 1.).collect() };
    Ok(vec![r(n(prm, "fast")), r(n(prm, "slow"))])
}

const DD: &[ParamSpec] = &[real("pct", "Drawdown (%)", 20., 0.1, 90., 0.1), len("reset", "Closes below the stop before it resets", 10.)];
fn drawdown_trailing_stop(b: &Bars, prm: &Params) -> Output {
    let (q, r, c, m) = (p(prm, "pct") / 100., n(prm, "reset"), &b.close, b.len());
    let mut o = vec![vec![NA; m]; 2]; // stop, below
    let (mut peak, mut below) = (NA, 0usize);
    for t in 0..m {
        let x = c[t];
        if t == 0 || x > peak { peak = x; }
        below = if x < peak * (1. - q) { below + 1 } else { 0 };
        if below >= r { peak = x; below = 0; }
        (o[0][t], o[1][t]) = (peak * (1. - q), below as f64);
    }
    Ok(o)
}

const FP: &[ParamSpec] = &[real("pct", "Reversal (%)", 4., 0.1, 50., 0.1)];
fn four_percent_model(b: &Bars, prm: &Params) -> Output {
    let (q, c, m) = (p(prm, "pct") / 100., &b.close, b.len());
    let mut o = vec![vec![NA; m]; 2]; // direction, signal
    if m == 0 { return Ok(o); }
    let (mut dir, mut hi, mut lo) = (0., c[0], c[0]);
    (o[0][0], o[1][0]) = (0., 0.);
    for t in 1..m {
        let (x, mut sig) = (c[t], 0.);
        if dir >= 0. { hi = hi.max(x); }
        if dir <= 0. { lo = lo.min(x); }
        if dir <= 0. && x >= lo * (1. + q) { (dir, hi, sig) = (1., x, 1.); }
        else if dir >= 0. && x <= hi * (1. - q) { (dir, lo, sig) = (-1., x, -1.); }
        (o[0][t], o[1][t]) = (dir, sig);
    }
    Ok(o)
}

const GS: &[ParamSpec] = &[int("swing_count", "Swing count (bars)", 2., 1., 20.)];
fn gann_swing_chart(b: &Bars, prm: &Params) -> Output {
    let (need, h, l, m) = (n(prm, "swing_count"), &b.high, &b.low, b.len());
    let mut o = vec![vec![NA; m]; 4]; // swing, direction, swing_high, swing_low
    let (mut dir, mut ext, mut ext_i, mut cu, mut cd, mut sh, mut sl) = (0i32, NA, 0usize, 0usize, 0usize, NA, NA);
    for t in 0..m {
        // against the previous bar: 1 higher high and higher low, −1 lower high and lower low, 2 outside, 0 inside / equal / bar 0
        let kind = if t == 0 { 0 } else {
            let (hh, lh, hl, ll) = (h[t] > h[t - 1], h[t] < h[t - 1], l[t] > l[t - 1], l[t] < l[t - 1]);
            if hh && hl { 1 } else if lh && ll { -1 } else if hh && ll { 2 } else { 0 }
        };
        match dir {
            1 => {
                if h[t] > ext { (ext, ext_i) = (h[t], t); }
                if kind == -1 || kind == 2 { cd += 1; } else if kind == 1 { cd = 0; }
                if cd >= need {
                    sh = ext;
                    ext_i = arg_ext(l, ext_i, t, false);
                    (dir, ext, cu, cd) = (-1, l[ext_i], 0, 0);
                }
            }
            -1 => {
                if l[t] < ext { (ext, ext_i) = (l[t], t); }
                if kind == 1 || kind == 2 { cu += 1; } else if kind == -1 { cu = 0; }
                if cu >= need {
                    sl = ext;
                    ext_i = arg_ext(h, ext_i, t, true);
                    (dir, ext, cu, cd) = (1, h[ext_i], 0, 0);
                }
            }
            _ => {
                if kind == 1 { (cu, cd) = (cu + 1, 0); } else if kind == -1 { (cu, cd) = (0, cd + 1); }
                if cu >= need { ext_i = arg_ext(h, 0, t, true); (dir, ext, cu, cd) = (1, h[ext_i], 0, 0); }
                else if cd >= need { ext_i = arg_ext(l, 0, t, false); (dir, ext, cu, cd) = (-1, l[ext_i], 0, 0); }
            }
        }
        o[0][t] = if dir == 0 { NA } else { ext };
        (o[1][t], o[2][t], o[3][t]) = (dir as f64, sh, sl);
    }
    Ok(o)
}

const GM: &[ParamSpec] = &[len("length", "Length", 20.)];
fn gma(b: &Bars, prm: &Params) -> Output {
    let (k, c) = (n(prm, "length"), &b.close);
    Ok(vec![(0..b.len()).map(|i| {
        if i + 1 < k { return NA; }
        let w = &c[i + 1 - k..=i];
        if w.iter().any(|&v| v.is_nan() || v <= 0.) { return NA; }
        (w.iter().map(|v| v.ln()).sum::<f64>() / k as f64).exp()
    }).collect()])
}

const IE: &[ParamSpec] = &[int("length", "Length", 15., 2., 2000.)];
fn tillson_ie2(b: &Bars, prm: &Params) -> Output {
    let k = n(prm, "length");
    let (e0, e1, s) = (ta::linreg(&b.close, k, 0), ta::linreg(&b.close, k, 1), ta::sma(&b.close, k));
    let ilrs: Vec<f64> = (0..b.len()).map(|i| s[i] + e0[i] - e1[i]).collect(); // slope = linreg value now − linreg value one bar back on the same line
    Ok(vec![(0..b.len()).map(|i| (ilrs[i] + e0[i]) / 2.).collect(), ilrs])
}

const LB: &[ParamSpec] = &[len("hl_length", "High/low period", 16.), len("atr_length", "ATR length", 9.), real("factor", "Factor", 2.5, 0.1, 20., 0.1)];
fn lbr_paint_bars(b: &Bars, prm: &Params) -> Output {
    let (k, a, f, c, m) = (n(prm, "hl_length"), n(prm, "atr_length"), p(prm, "factor"), &b.close, b.len());
    let aatr = ta::sma(&ta::rma(&ta::tr(b, true), a), a);
    let (ll, hh) = (ta::lowest(&b.low, k), ta::highest(&b.high, k));
    let mut o = vec![vec![NA; m]; 4]; // band1, band2, bull, bear
    for t in 0..m {
        let (u, d) = (ll[t] + f * aatr[t], hh[t] - f * aatr[t]);
        if u.is_nan() || d.is_nan() { continue; }
        (o[0][t], o[1][t]) = (u, d);
        o[2][t] = if c[t] > u && c[t] > d { 1. } else { 0. };
        o[3][t] = if c[t] < u && c[t] < d { 1. } else { 0. };
    }
    Ok(o)
}

const QE: &[ParamSpec] = &[len("length", "Length", 14.)];
fn qema(b: &Bars, prm: &Params) -> Output {
    let k = n(prm, "length");
    let mut e = vec![ta::ema(&b.close, k)];
    for j in 1..5 { let next = ta::ema(&e[j - 1], k); e.push(next); }
    Ok(vec![(0..b.len()).map(|i| 5. * e[0][i] - 10. * e[1][i] + 10. * e[2][i] - 5. * e[3][i] + e[4][i]).collect()])
}

const PT: &[ParamSpec] = &[real("pct", "Trail (%)", 5., 0.1, 50., 0.1)];
fn pct_trailing_stop(b: &Bars, prm: &Params) -> Output {
    let (q, c, m) = (p(prm, "pct") / 100., &b.close, b.len());
    let mut o = vec![vec![NA; m]; 2]; // stop, direction
    if m == 0 { return Ok(o); }
    let (mut dir, mut stop) = (1., c[0] * (1. - q));
    for t in 0..m {
        let x = c[t];
        if t > 0 {
            if dir == 1. {
                if x < stop { (dir, stop) = (-1., x * (1. + q)); } else { stop = stop.max(x * (1. - q)); }
            } else if x > stop { (dir, stop) = (1., x * (1. - q)); } else { stop = stop.min(x * (1. + q)); }
        }
        (o[0][t], o[1][t]) = (stop, dir);
    }
    Ok(o)
}

const SZ: &[ParamSpec] = &[len("length", "Lookback", 10.), real("coef", "Coefficient", 2., 0.1, 10., 0.1), len("prevent", "Bars the stop may not loosen", 5.)];
fn safezone_stop(b: &Bars, prm: &Params) -> Output {
    let (k, f, w, m) = (n(prm, "length"), p(prm, "coef"), n(prm, "prevent"), b.len());
    let (h, l) = (&b.high, &b.low);
    // mean of the positive penetrations in a window (0 when there are none)
    let avg = |w: &[f64]| -> f64 {
        let (s, cnt) = w.iter().filter(|v| **v > 0.).fold((0f64, 0f64), |(s, c), v| (s + v, c + 1.));
        if cnt == 0. { 0. } else { s / cnt }
    };
    let dn: Vec<f64> = (0..m).map(|i| if i == 0 { 0. } else { (l[i - 1] - l[i]).max(0.) }).collect();
    let up: Vec<f64> = (0..m).map(|i| if i == 0 { 0. } else { (h[i] - h[i - 1]).max(0.) }).collect();
    let (mut lr, mut sr) = (vec![NA; m], vec![NA; m]);
    for t in k + 1..m { lr[t] = l[t - 1] - f * avg(&dn[t - k..t]); sr[t] = h[t - 1] + f * avg(&up[t - k..t]); }
    let (mut ls, mut ss) = (vec![NA; m], vec![NA; m]);
    for t in k + w..m {
        ls[t] = lr[t + 1 - w..=t].iter().cloned().fold(f64::MIN, f64::max);
        ss[t] = sr[t + 1 - w..=t].iter().cloned().fold(f64::MAX, f64::min);
    }
    Ok(vec![ls, ss])
}

const TA: &[ParamSpec] = &[len("avg_length", "SMA length", 28.), len("tai_length", "TAI length", 5.)];
fn tai(b: &Bars, prm: &Params) -> Output {
    let s = ta::sma(&b.close, n(prm, "avg_length"));
    let k = n(prm, "tai_length");
    let (hi, lo) = (full_ext(&s, k, true), full_ext(&s, k, false));
    Ok(vec![(0..b.len()).map(|i| (hi[i] - lo[i]) * 100. / b.close[i]).collect()])
}

const HL: &[ParamSpec] = &[len("length", "Period (bars)", 10.)];
fn high_low_trailing_stop(b: &Bars, prm: &Params) -> Output {
    let (k, c, m) = (n(prm, "length"), &b.close, b.len());
    let (hh, ll) = (ta::highest(&b.high, k), ta::lowest(&b.low, k));
    let mut o = vec![vec![NA; m]; 2]; // stop, direction
    let mut dir = 1.;
    for t in k - 1..m {
        let prev = if t > 0 { o[0][t - 1] } else { NA }; // NaN on the first bar: no flip
        if dir == 1. && c[t] < prev { dir = -1.; } else if dir == -1. && c[t] > prev { dir = 1.; }
        (o[0][t], o[1][t]) = (if dir == 1. { ll[t] } else { hh[t] }, dir);
    }
    Ok(o)
}

pub const DEFS: &[IndicatorDef] = &[
    def("congestion_index", "Congestion Index (Katsanos)", TR, CI, &["ci", "raw"], PN, NO,
        "Markos Katsanos, Congestion Index (Intermarket Trading Strategies, Wiley 2008; MetaStock code © Katsanos 2008): raw = ROC(close, n − 1, %)/((highest(high, n) − lowest(low, n))/(lowest(low, n) + 0.01) + 0.000001), with ROC = 100·(close − close[n − 1])/close[n − 1] and the high/low window the last n bars; ci = ema(raw, 3). Readings between −20 and +20 mean congestion; a cross above +20 / below −20 starts a trend. Default n = 15 (Katsanos's 'days in congestion'). raw from bar n − 1, ci from bar n + 1. Adaptation: the 3-bar EMA is the lab's EMA (seeded with the SMA of its first 3 values); MetaStock's Mov(…, E) seeds differently, so the first few ci values differ. Optuma ships a Congestion Index (CIDX(), default 20 per its forum) without publishing its formula; this entry is Katsanos's published one. Sources: TradingView script 'Congestion Index by Katsanos' (ceyhun, MetaStock code quoted, https://www.tradingview.com/script/V7e88j8Y); Optuma Knowledge Base, Congestion Index (https://www.optuma.com/kb/optuma/tools/price/congestion-index).",
        &[16, 14], false, congestion_index),
    def("didi_index", "Didi Index (Aguiar)", TR, DI, &["fast", "slow"], PN, NO,
        "Didi Index ('Agulhada do Didi', Odir Aguiar): three SMAs of close, 3, 8 and 20; the 8-period average is normalised to the zero line and the other two are divided by it: fast = sma(close, 3)/sma(close, 8) − 1, slow = sma(close, 20)/sma(close, 8) − 1. Buy 'needle' when fast crosses above 0 while slow crosses below 0; sell is the reverse. fast from bar max(fast, mid) − 1 (7), slow from bar max(slow, mid) − 1 (19). Adaptation: Aguiar published no code; ports differ between the ratio form (the MetaTrader 5 port, per the NinjaTrader thread) and a difference form (sma3 − sma8). The lab uses the ratio form, minus 1 so the 8-period line sits at 0 as the MQL5 listing describes. Sources: MQL5 Code Base 1725, Didi Index (Rudinei Felipetto, https://www.mql5.com/en/code/1725); MQL5 Market, Didi Index Indicator (https://www.mql5.com/es/market/product/56652); NinjaTrader forum 'Didi index help' (https://forum.ninjatrader.com/forum/ninjatrader-7/indicator-development-aa/81642-didi-index-help).",
        &[7, 19], false, didi_index),
    def("drawdown_trailing_stop", "Drawdown Trailing Stop (Hull)", TR, DD, &["stop", "below"], PR, NO,
        "Alan Hull's Drawdown Trailing Stop Loss (Optuma): stop = peak·(1 − pct/100), where peak = the highest close since the last reset, so the stop never falls while price falls. below = the number of consecutive closes under the stop; when it reaches reset, the stop resets to pct below that bar's close (peak = that close) and below returns to 0. Defaults pct 20, reset 10 (Optuma's defaults). Long side only, as published. From bar 0. Adaptation: Optuma starts the tool at a bar the user clicks; here it starts at bar 0, so the line depends on the data's first bar until the first reset. Optuma's 'based on highest high' option is not offered. Sources: Optuma Knowledge Base, Drawdown Trailing Stop Loss (https://www.optuma.com/kb/tools/hull/drawdown-trailing-stop-loss).",
        &[0, 0], false, drawdown_trailing_stop),
    def("four_percent_model", "Four Percent Model (Davis / Zweig)", TR, FP, &["direction", "signal"], PN, NO,
        "Ned Davis's Four Percent Model as popularised by Martin Zweig (Winning on Wall Street): buy when the weekly close rises pct (4%) or more from its lowest close since the last sell; sell when it falls pct or more from its highest close since the last buy; hold until the opposite signal. Before the first signal both extremes are tracked from bar 0 (the buy test runs first if both would trigger on one bar). direction = 1 after a buy, −1 after a sell, 0 before the first signal; signal = 1 on a buy bar, −1 on a sell bar, else 0. From bar 0. Adaptation: the published model uses weekly closes of the Value Line Geometric Index; here it runs on the bars of whatever instrument and timeframe is loaded (resample to weekly for the original rule). Sources: MetaStock, Technical Analysis from A to Z, Four Percent Model (https://www.metastock.com/customer/resources/taaz/?p=57); CXO Advisory, Martin Zweig's Four Percent Model Indicator (https://www.cxoadvisory.com/technical-trading/martin-zweigs-four-percent-model-indicator/).",
        &[0, 0], false, four_percent_model),
    def("gann_swing_chart", "Gann Swing Chart (causal)", TR, GS, &["swing", "direction", "swing_high", "swing_low"], PR, NO,
        "Gann swing chart as Optuma documents it: each bar is compared with the previous one: higher bar = higher high and higher low, lower bar = lower high and lower low, outside bar = higher high and lower low, inside bar = anything else (inside bars are not counted). An up swing keeps moving to new highs; it turns down after swing_count lower bars in a row (an outside bar counts as a reversal bar, an up bar resets the count); a down swing turns up likewise. On a turn the new swing starts at the lowest low (highest high) since the old swing's extreme bar, ties to the latest bar. Before the first swing, the first run of swing_count higher (lower) bars starts an up (down) swing from the highest high (lowest low) so far. swing = the current swing's extreme so far (the end of the swing line on this bar); direction = 1 up, −1 down, 0 before the first swing; swing_high / swing_low = the last confirmed turning-point prices. Default swing_count 2 (lab choice; Optuma publishes no default; Gann's 1-, 2- and 3-bar charts are all valid). Adaptation: a drawn swing chart redraws its line back to the extreme bar once a turn is confirmed; here each bar only shows what was known on that bar, so values are causal. Optuma's Use Breakout, Use Inside and Use Next Bar options are not offered. swing is NaN before the first swing, swing_high / swing_low until the first confirmed turn (sparse). Sources: Optuma Knowledge Base, Gann Swing Charts (https://www.optuma.com/kb/charts/gann-swing-charts); Optuma GANNSWING() function (https://www.optuma.com/kb/scripting/formulas-and-scripting-functions/gannswing-function).",
        &[X, 0, X, X], true, gann_swing_chart),
    def("gma", "Geometric Moving Average (GMA)", MA, GM, &["gma"], PR, NO,
        "Geometric moving average: the n-th root of the product of the last n closes, computed in log space as exp(Σ ln(close)/n) over the last n bars. NaN when a close in the window is ≤ 0. From bar n − 1. Default n 20 (lab choice; the platforms publish no default). Sources: TrendSpider Help, Geometric Moving Average (https://help.trendspider.com/kb/indicators/geometric-moving-average); LuxAlgo Library, Geometric MA (https://www.luxalgo.com/library/concept/geometric-ma/).",
        &[19], false, gma),
    def("tillson_ie2", "IE/2 (Tillson)", MA, IE, &["ie2", "ilrs"], PR, NO,
        "Tim Tillson, 'Smoothing Techniques For More Accurate Signals' (S&C Jan 1998; TradeStation code in S&C Traders' Tips Feb 1998): ILRS (integral of linear regression slope) = sma(close, n) + linreg slope(close, n); EPMA (end-point moving average) = the least-squares line's value on the newest bar (linreg(close, n, 0)); IE/2 = (ILRS + EPMA)/2. The slope is per bar, taken as linreg(close, n, 0) − linreg(close, n, 1) on the same fitted line. Since EPMA = sma + slope·(n − 1)/2, IE/2 lags about n/4 bars, as Tillson states. From bar n − 1. Default n 15 (the Traders' Tips and eSignal default). Sources: S&C Traders' Tips February 1998 (https://traders.com/Documentation/FEEDbk_docs/1998/02/TradersTips/Tips9802.html); TradingView, Tillson IE/2 (https://www.tradingview.com/script/x8gGFDiW).",
        &[14, 14], false, tillson_ie2),
    def("lbr_paint_bars", "LBR Paint Bars (Raschke)", TR, LB, &["band1", "band2", "bull", "bear"], PR, NO,
        "Linda Bradford Raschke's LBR Paint Bars, as in MotiveWave's published code: atr = Wilder ATR(atr_length) (the lab's atr, rma of true range); aatr = factor·sma(atr, atr_length); band1 = lowest(low, hl_length) + aatr; band2 = highest(high, hl_length) − aatr; bull = 1 when close > band1 and close > band2, bear = 1 when close < band1 and close < band2, else 0 (MotiveWave's default 'And'; thinkorswim: bars closing above both volatility lines are painted bullish, below both bearish). Defaults hl_length 16, atr_length 9, factor 2.5, SMA average (MotiveWave). From bar max(hl_length − 1, 2·atr_length − 2) (16). The 'Or' option and other average types are not offered. Sources: MotiveWave studies, LBR Paint Bars (https://docs.motivewave.com/studies/k-l#lbr-paint-bars); thinkorswim Learning Center, LBR_PaintBars (https://toslc.thinkorswim.com/center/reference/Tech-Indicators/studies-library/G-L/LBR-PaintBars).",
        &[16, 16, 16, 16], false, lbr_paint_bars),
    def("qema", "Quadruple Exponential Moving Average (QEMA)", MA, QE, &["qema"], PR, NO,
        "QEMA = 5·e1 − 10·e2 + 10·e3 − 5·e4 + e5, where e1 = ema(close, n) and each next e is the ema of the previous one (the lab's SMA-seeded EMA). From bar 5·(n − 1) (65). Default n 14 (ProRealCode). On a straight-line series QEMA equals the price. Adaptation: TradeStation ships a QEMA(Price, Length) function but its help page gives no formula; this is the five-term form ProRealCode published (adapted from the TS2 platform) with the EMA as the average. A TradingView community script uses a different four-term form (4·e1 − 6·e2 + 4·e3 − e4 with changing alphas) and is not this. Sources: ProRealCode, Quadruple Exponential Moving Average indicator (Nicolas, 2017, https://www.prorealcode.com/prorealtime-indicators/quadruple-exponential-moving-average-indicator/); TradeStation EasyLanguage help, QEMA (Function) (https://help.tradestation.com/10_00/eng/tsdevhelp/elword/function/QEMA_function_.htm).",
        &[65], false, qema),
    def("pct_trailing_stop", "Percentage Trailing Stop", TR, PT, &["stop", "direction"], PR, NO,
        "Percentage trailing stop on closes: long: stop = max(stop[1], close·(1 − pct/100)); a close below the stop flips short with stop = close·(1 + pct/100); short: stop = min(stop[1], close·(1 + pct/100)); a close above flips long with stop = close·(1 − pct/100). direction 1 long, −1 short. Starts long on bar 0 with stop = close·(1 − pct/100) (lab choice). From bar 0. Default pct 5 (lab choice). Sources: Investopedia, Trailing Stop (https://www.investopedia.com/terms/t/trailingstop.asp).",
        &[0, 0], false, pct_trailing_stop),
    def("safezone_stop", "SafeZone Stop (Elder)", TR, SZ, &["long_stop", "short_stop"], PR, NO,
        "Alexander Elder, SafeZone (Come Into My Trading Room, 2002, pp. 173–180): downside penetration of bar i = max(low[i − 1] − low[i], 0); upside = max(high[i] − high[i − 1], 0). On bar t, avg = the mean of the positive penetrations among bars t − length ..= t − 1 (0 if none); raw long stop = low[t − 1] − coef·avg_down; raw short stop = high[t − 1] + coef·avg_up (the stop for bar t from the bars before it). So the stop does not loosen, long_stop = the highest raw long stop of the last prevent bars and short_stop = the lowest raw short stop (prevent 1 = raw). Defaults length 10, coef 2, prevent 5 (pverspeelt Quantfunctions safezone()). From bar length + prevent (15). Sources: Quantfunctions R package, safezone (https://rdrr.io/github/pverspeelt/Quantfunctions/man/safezone.html); LuxAlgo Library, Elder SafeZone Stop (https://www.luxalgo.com/library/concept/elder-safezone-stop/); ProRealCode, SafeZone Stop - Elder (https://www.prorealcode.com/prorealtime-indicators/safezone-stop-elder).",
        &[15, 15], false, safezone_stop),
    def("tai", "Trend Analysis Index (TAI)", TR, TA, &["tai"], PN, NO,
        "Adam White, 'Filtering Trades With A Moving Average Slope' (S&C Aug 1992), as MotiveWave publishes it: av = sma(close, avg_length); tai = (highest(av, tai_length) − lowest(av, tai_length))·100/close. Defaults avg_length 28, tai_length 5. From bar avg_length + tai_length − 2 (31), when the window of averages is full. Sources: MotiveWave studies, Trend Analysis Index (https://docs.motivewave.com/studies/s-t#trend-analysis-index); FXCodeBase, Trend Analysis Index (https://docs.gehtsoftusa.com/fxcodebase-backup/backup/custom-indicators/t/trend-analysis-index/).",
        &[31], false, tai),
    def("high_low_trailing_stop", "High/Low Trailing Stop (causal)", TR, HL, &["stop", "direction"], PR, NO,
        "Days Low / High trailing stop: while long the stop is lowest(low, length) over the last length bars; a close below the previous bar's stop flips short, and the stop becomes highest(high, length); a close above the previous bar's stop flips long again. direction 1 long, −1 short; starts long on bar length − 1. From bar length − 1. Default length 10 (lab choice; Optuma lists no default). Adaptation: Optuma's Days Low / High Trailing Stop starts at a bar the user clicks, tracks the lowest low (highest high) over the period and stops plotting once crossed; here it runs from the first full window and turns to the other side on a cross, so it can be used in rules. Sources: Optuma Knowledge Base, Days Low / High Trailing Stop (https://optuma.com/kb/optuma/tools/price/days-low-high-trailing-stop).",
        &[9, 9], false, high_low_trailing_stop),
    listed("countback_line", "Countback Line (Guppy)", "Drawings & display",
        "Daryl Guppy's Countback Line: from a significant low the analyst picks, take that bar's high, count back to the left to the next bar with a higher high, then the next; the third high is the countback line, a breakout/stop level (mirrored from a high for shorts). Optuma ships it as drawing tools placed on a chosen bar",
        "chart drawing/display — anchored at a low (or high) the user picks; not a bar-by-bar series; not usable in rules"),
    listed("fixed_days_hl_stop", "Fixed Days High/Low Stop", "Drawings & display",
        "Optuma Fixed Days High Stop / Fixed Days Low Stop: from a bar the user clicks, a stop line at the highest high (lowest low) of the previous period bars (8 by default), drawn until price crosses it (https://www.optuma.com/kb/optuma/tools/stops/fixed-days-high-stop)",
        "chart drawing/display — anchored at a bar the user picks; not a bar-by-bar series; not usable in rules"),
    listed("hart_swing_chart", "Hart Swing Chart", "Unpublished formulas",
        "Joseph Hart (Trend Dynamics) swing chart, Optuma's Hart Swings Chart Overlay / HARTSWING(): a trend swing chart said to work like Gann swings; the rule is not published (https://www.optuma.com/kb/tools/barros/hart-swings-chart-overlay)",
        "not available yet — formula not published (Optuma's Hart Swings tool page gives no rule; Trend Dynamics' method is proprietary)"),
    listed("xbar_trailing_stop", "X-Bar Trailing Stop (Beck)", "Unpublished formulas",
        "Optuma X Bar Trailing Stop, 'a proprietary tool, based on the work of Ross Beck, FCSI from Geometric Trading', placed from a clicked bar, inputs Period and Stop Offset (https://optuma.com/kb/optuma/tools/beck/x-bar-trailing-stop)",
        "not available yet — formula not published (Optuma calls the X Bar Trailing Stop proprietary; its help page gives no rule)"),
];
/// (id, platforms that ship it, ""), one per entry.
pub const SOURCES: &[(&str, &str, &str)] = &[
    ("congestion_index", "Optuma: Congestion Index; TradingView (ceyhun): Congestion Index by Katsanos; ProRealCode: Congestion index", ""),
    ("didi_index", "MetaTrader 5 (MQL5 Code Base): Didi Index; TradingView (community): Didi Index", ""),
    ("drawdown_trailing_stop", "Optuma: Drawdown Trailing Stop Loss (Hull)", ""),
    ("four_percent_model", "MetaStock (Technical Analysis from A to Z): Four Percent Model", ""),
    ("gann_swing_chart", "Optuma: Gann Swing Chart", ""),
    ("gma", "TrendSpider: Geometric Moving Average", ""),
    ("tillson_ie2", "TradeStation (S&C Traders' Tips Feb 1998): IE/2; eSignal: IE/2; TradingView (community): Tillson IE/2", ""),
    ("lbr_paint_bars", "thinkorswim: LBR_PaintBars; MotiveWave: LBR Paint Bars", ""),
    ("qema", "TradeStation: QEMA (function); ProRealCode: Quadruple Exponential Moving Average", ""),
    ("pct_trailing_stop", "Percentage Trailing Stop (generic)", ""),
    ("safezone_stop", "Elder SafeZone Stop (ProRealCode; Quantfunctions R package)", ""),
    ("tai", "MotiveWave: Trend Analysis Index; MultiCharts: Trend Analysis Index; FXCodeBase: TAI", ""),
    ("high_low_trailing_stop", "Optuma: Days Low / High Trailing Stop", ""),
    ("countback_line", "Optuma: Countback Line", ""),
    ("fixed_days_hl_stop", "Optuma: Fixed Days High Stop / Fixed Days Low Stop", ""),
    ("hart_swing_chart", "Optuma: Hart Swings Chart Overlay", ""),
    ("xbar_trailing_stop", "Optuma: X Bar Trailing Stop", ""),
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lab::testdata::{assert_series, closes, compiles, from_ohlc, go};
    const N: f64 = f64::NAN;

    #[test] fn congestion_index_is_roc_over_relative_range() {
        let b = from_ohlc(&[(10., 11., 9., 10.), (10., 12., 10., 11.), (11., 13., 10., 12.), (12., 12., 9., 9.), (9., 10., 8., 10.)]);
        let o = go("congestion_index", &b, &[("length", 3.)]);
        let r2 = 20. / (4. / 9.01 + 1e-6);
        let r3 = -(200. / 11.) / (4. / 9.01 + 1e-6);
        let r4 = -(100. / 6.) / (5. / 8.01 + 1e-6);
        assert_series(&o[1], &[N, N, r2, r3, r4]);
        assert_series(&o[0], &[N, N, N, N, (r2 + r3 + r4) / 3.]);
    }
    #[test] fn didi_divides_by_the_middle_average() {
        let o = go("didi_index", &closes(&[1., 2., 3., 4.]), &[("fast", 1.), ("mid", 2.), ("slow", 3.)]);
        assert_series(&o[0], &[N, 2. / 1.5 - 1., 3. / 2.5 - 1., 4. / 3.5 - 1.]);
        assert_series(&o[1], &[N, N, 2. / 2.5 - 1., 3. / 3.5 - 1.]);
    }
    #[test] fn drawdown_stop_resets_after_closes_below() {
        let o = go("drawdown_trailing_stop", &closes(&[100., 110., 98., 98.5, 97., 105.]), &[("pct", 10.), ("reset", 2.)]);
        assert_series(&o[0], &[90., 99., 99., 88.65, 88.65, 94.5]);
        assert_series(&o[1], &[0., 0., 1., 0., 0., 0.]);
    }
    #[test] fn four_percent_flips_on_four_percent_moves() {
        let o = go("four_percent_model", &closes(&[100., 97., 100.9, 101., 96.9, 97.]), &[]);
        assert_series(&o[0], &[0., 0., 1., 1., -1., -1.]);
        assert_series(&o[1], &[0., 0., 1., 0., -1., 0.]);
    }
    #[test] fn gann_swing_turns_after_two_lower_bars() {
        let hl = [(10., 9.), (11., 10.), (12., 11.), (11.5, 10.5), (11.6, 10.8), (11., 10.), (10.5, 9.5), (10., 9.6)];
        let b = from_ohlc(&hl.iter().map(|&(h, l)| (l, h, l, h)).collect::<Vec<_>>());
        let o = go("gann_swing_chart", &b, &[]);
        assert_series(&o[0], &[N, N, 12., 12., 12., 12., 9.5, 9.5]);
        assert_series(&o[1], &[0., 0., 1., 1., 1., 1., -1., -1.]);
        assert_series(&o[2], &[N, N, N, N, N, N, 12., 12.]);
        assert_series(&o[3], &[N; 8]);
    }
    #[test] fn gma_is_the_geometric_mean() {
        assert_series(&go("gma", &closes(&[1., 4., 2., 8.]), &[("length", 2.)])[0], &[N, 2., 8f64.sqrt(), 4.]);
    }
    #[test] fn ie2_averages_ilrs_and_epma() {
        // x 1..4, y 1,1,1,5: slope 1.2, mean 2 → EPMA 3.8, ILRS 3.2
        let o = go("tillson_ie2", &closes(&[1., 1., 1., 5.]), &[("length", 4.)]);
        assert_series(&o[0], &[N, N, N, 3.5]);
        assert_series(&o[1], &[N, N, N, 3.2]);
    }
    #[test] fn lbr_paints_outside_both_lines() {
        let b = from_ohlc(&[(10., 11., 9., 10.), (10., 12., 10., 11.5), (11., 11.5, 10.5, 10.6)]);
        let o = go("lbr_paint_bars", &b, &[("hl_length", 2.), ("atr_length", 1.), ("factor", 1.)]);
        assert_series(&o[0], &[N, 11., 11.]);
        assert_series(&o[1], &[N, 10., 11.]);
        assert_series(&o[2], &[N, 1., 0.]);
        assert_series(&o[3], &[N, 0., 1.]);
    }
    #[test] fn qema_tracks_a_straight_line() {
        assert_series(&go("qema", &closes(&[1., 2., 3., 4., 5., 6., 7.]), &[("length", 2.)])[0], &[N, N, N, N, N, 6., 7.]);
    }
    #[test] fn pct_stop_trails_and_flips() {
        let o = go("pct_trailing_stop", &closes(&[100., 110., 104., 103.]), &[]);
        assert_series(&o[0], &[95., 104.5, 109.2, 108.15]);
        assert_series(&o[1], &[1., 1., -1., -1.]);
    }
    #[test] fn safezone_uses_mean_penetration_of_earlier_bars() {
        let b = from_ohlc(&[(10., 12., 10., 11.), (11., 13., 9., 12.), (12., 13.5, 9.5, 13.), (13., 14., 8.5, 13.5), (13., 14., 9., 13.)]);
        let o = go("safezone_stop", &b, &[("length", 2.), ("prevent", 1.)]);
        assert_series(&o[0], &[N, N, N, 7.5, 6.5]);
        assert_series(&o[1], &[N, N, N, 15., 15.]);
        let o = go("safezone_stop", &b, &[("length", 2.), ("prevent", 2.)]);
        assert_series(&o[0], &[N, N, N, N, 7.5]);
        assert_series(&o[1], &[N, N, N, N, 15.]);
    }
    #[test] fn tai_is_the_sma_range_over_price() {
        assert_series(&go("tai", &closes(&[1., 2., 4., 8.]), &[("avg_length", 2.), ("tai_length", 2.)])[0], &[N, N, 37.5, 37.5]);
    }
    #[test] fn high_low_stop_flips_on_a_close_through_the_previous_stop() {
        let b = from_ohlc(&[(10., 11., 9., 10.), (10., 12., 10., 11.), (11., 13., 11., 12.), (12., 12., 9., 9.5), (9., 10., 8., 9.), (9., 13., 9., 12.5)]);
        let o = go("high_low_trailing_stop", &b, &[("length", 2.)]);
        assert_series(&o[0], &[N, 9., 10., 13., 12., 8.]);
        assert_series(&o[1], &[N, 1., 1., -1., -1., 1.]);
    }
    #[test] fn rules_reach_gap_a() {
        for (id, out) in [("congestion_index", "ci"), ("didi_index", "fast"), ("drawdown_trailing_stop", "stop"), ("four_percent_model", "direction"),
            ("gann_swing_chart", "direction"), ("gma", "gma"), ("tillson_ie2", "ie2"), ("lbr_paint_bars", "bull"), ("qema", "qema"),
            ("pct_trailing_stop", "stop"), ("safezone_stop", "long_stop"), ("tai", "tai"), ("high_low_trailing_stop", "direction")] {
            compiles(id, out).unwrap();
        }
    }
    #[test] fn every_entry_has_one_source_row() {
        for d in DEFS { assert_eq!(SOURCES.iter().filter(|s| s.0 == d.id).count(), 1, "{}", d.id); }
        assert_eq!(SOURCES.len(), DEFS.len());
        for d in DEFS.iter().filter(|d| d.compute.is_none()) { assert!(!d.note.is_empty(), "{}", d.id); }
    }
}
