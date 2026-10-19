//! Cross-platform indicators, Momentum (part 2) (2026-10-19 build list: TradingView, AmiBroker, MetaTrader, cTrader, NinjaTrader, TradeStation, thinkorswim, Sierra Chart, MetaStock). Formulas: see each definition string.
//! "lab ema" = EMA seeded with the SMA of its first n values (ta::ema); "rsi" = the lab's Wilder RSI (dn 0 → 100, up 0 → 0).
use super::bars::Bars;
use super::catalogue::{int, len, n, p, real, IndicatorDef, Needs, Output, ParamSpec, Params, Placement};
use super::osc2::rsi_like;
use super::ta::{self, NA};

#[allow(clippy::too_many_arguments)]
const fn def(id: &'static str, name: &'static str, category: &'static str, params: &'static [ParamSpec], outputs: &'static [&'static str], placement: Placement,
             needs: Needs, definition: &'static str, first: &'static [usize], f: fn(&Bars, &Params) -> Output) -> IndicatorDef {
    IndicatorDef { id, name, category, params, outputs, placement, needs, definition, first, sparse: false, compute: Some(f), note: "" }
}
const M: &str = "Momentum";
const PN: Placement = Placement::Pane;
const PR: Placement = Placement::Price;
const NO: Needs = Needs::NONE;
const VOL: Needs = Needs { volume: true, ..Needs::NONE };

fn div(a: f64, b: f64) -> f64 { if b == 0. || a.is_nan() || b.is_nan() { NA } else { a / b } }
fn zip(a: &[f64], b: &[f64], f: impl Fn(f64, f64) -> f64) -> Vec<f64> { a.iter().zip(b).map(|(&x, &y)| f(x, y)).collect() }
fn map(a: &[f64], f: impl Fn(f64) -> f64) -> Vec<f64> { a.iter().map(|&x| f(x)).collect() }
fn sign(x: f64) -> f64 { if x > 0. { 1. } else if x < 0. { -1. } else { 0. } }
fn roc_s(x: &[f64], k: usize) -> Vec<f64> { (0..x.len()).map(|i| if i >= k { div(100. * (x[i] - x[i - k]), x[i - k]) } else { NA }).collect() }
fn gains(d: &[f64], up: bool) -> Vec<f64> { map(d, |v| if v.is_nan() { NA } else if up { v.max(0.) } else { (-v).max(0.) }) }
fn rsi_s(x: &[f64], k: usize) -> Vec<f64> { let d = ta::change(x, 1); rsi_like(&gains(&d, true), &gains(&d, false), k) }
/// (lowest, highest) of the last `k` values ending at `i`; None unless all `k` exist and are non-NaN (no partial windows).
fn win(x: &[f64], k: usize, i: usize) -> Option<(f64, f64)> {
    if i + 1 < k { return None; }
    let w = &x[i + 1 - k..=i];
    if w.iter().any(|v| v.is_nan()) { return None; }
    Some(w.iter().fold((f64::MAX, f64::MIN), |(lo, hi), &v| (lo.min(v), hi.max(v))))
}
/// Full-window stochastic of x: 100·(x − ll)/(hh − ll); zero range keeps the previous value (0 before any).
fn stoch_hold(x: &[f64], k: usize) -> Vec<f64> {
    let mut prev = 0.;
    (0..x.len()).map(|i| match win(x, k, i) { None => NA, Some((lo, hi)) => { if hi - lo > 0. { prev = 100. * (x[i] - lo) / (hi - lo); } prev } }).collect()
}
/// Recursive smoother seeded with its first input: s = s[1] + f·(x − s[1]).
fn smooth_first(x: &[f64], f: f64) -> Vec<f64> {
    let mut s = NA;
    x.iter().map(|&v| { if v.is_nan() { return NA; } s = if s.is_nan() { v } else { s + f * (v - s) }; s }).collect()
}

// ---- Projection Oscillator ----
const PROJ: &[ParamSpec] = &[len("length", "Length", 14.), len("signal", "Signal EMA", 3.)];
/// Least-squares slope per bar over the last k values (k = 1 → 0).
fn ls_slope(x: &[f64], k: usize) -> Vec<f64> {
    let kf = k as f64;
    let (sk, skk) = (kf * (kf + 1.) / 2., kf * (kf + 1.) * (2. * kf + 1.) / 6.);
    let den = kf * skk - sk * sk;
    (0..x.len()).map(|i| if i + 1 < k { NA } else if den == 0. { 0. } else {
        let (sy, sky) = x[i + 1 - k..=i].iter().enumerate().fold((0., 0.), |(a, c), (j, &y)| (a + y, c + (j + 1) as f64 * y));
        (kf * sky - sk * sy) / den
    }).collect()
}
fn projection_osc(b: &Bars, prm: &Params) -> Output {
    let k = n(prm, "length");
    let (sh, sl) = (ls_slope(&b.high, k), ls_slope(&b.low, k));
    let po: Vec<f64> = (0..b.len()).map(|t| if t + 1 < k { NA } else {
        let up = (0..k).map(|i| b.high[t - i] + i as f64 * sh[t]).fold(f64::MIN, f64::max);
        let lo = (0..k).map(|i| b.low[t - i] + i as f64 * sl[t]).fold(f64::MAX, f64::min);
        div(100. * (b.close[t] - lo), up - lo)
    }).collect();
    let sig = ta::ema(&po, n(prm, "signal"));
    Ok(vec![po, sig])
}

// ---- Price Zone Oscillator ----
const L14: &[ParamSpec] = &[len("length", "Length", 14.)];
fn pzo(b: &Bars, prm: &Params) -> Output {
    let k = n(prm, "length"); let c = &b.close;
    let cp: Vec<f64> = (0..c.len()).map(|i| if i == 0 { NA } else { sign(c[i] - c[i - 1]) * c[i] }).collect();
    Ok(vec![zip(&ta::ema(&cp, k), &ta::ema(c, k), |a, d| div(100. * a, d))])
}

// ---- QQE ----
const QQE: &[ParamSpec] = &[len("length", "RSI length", 14.), len("smooth", "RSI smoothing", 5.), real("factor", "QQE factor", 4.236, 0.1, 10., 0.001)];
/// Pine ta.cross: crossed over or under between the previous and the current bar.
fn crossed(a: f64, b: f64, a1: f64, b1: f64) -> bool { (a > b && a1 <= b1) || (a < b && a1 >= b1) }
fn qqe(b: &Bars, prm: &Params) -> Output {
    let k = n(prm, "length"); let w = 2 * k - 1;
    let rm = ta::ema(&rsi_s(&b.close, k), n(prm, "smooth"));
    let atr: Vec<f64> = (0..rm.len()).map(|i| if i == 0 { NA } else { (rm[i - 1] - rm[i]).abs() }).collect();
    let dar = map(&ta::ema(&ta::ema(&atr, w), w), |v| v * p(prm, "factor"));
    let len = b.len();
    let (mut lb, mut sb, mut q, mut tr) = (vec![NA; len], vec![NA; len], vec![NA; len], vec![NA; len]);
    let g = |v: &[f64], i: usize, back: usize| if i >= back { v[i - back] } else { NA };
    let mut trend = 1.;
    for i in 0..len {
        let (nl, ns) = (rm[i] - dar[i], rm[i] + dar[i]);
        let (r1, l1, s1) = (g(&rm, i, 1), g(&lb, i, 1), g(&sb, i, 1));
        lb[i] = if r1 > l1 && rm[i] > l1 { l1.max(nl) } else { nl };
        sb[i] = if r1 < s1 && rm[i] < s1 { s1.min(ns) } else { ns };
        if crossed(rm[i], s1, r1, g(&sb, i, 2)) { trend = 1.; } else if crossed(l1, rm[i], g(&lb, i, 2), r1) { trend = -1.; }
        if !lb[i].is_nan() { tr[i] = trend; q[i] = if trend == 1. { lb[i] } else { sb[i] }; }
    }
    Ok(vec![rm, q, tr])
}

// ---- Rainbow Oscillator / Rahul Mohindar Oscillator (shared core) ----
const RB: &[ParamSpec] = &[len("step", "Step length", 2.), len("lookback", "HH/LL length", 10.)];
const RMO: &[ParamSpec] = &[len("step", "Nested SMA length", 2.), len("lookback", "HH/LL length", 10.), len("ema1", "Swing EMA", 30.), len("ema2", "RMO EMA", 81.)];
/// Ten nested SMAs m1 = sma(close, step), m_k = sma(m_{k−1}, step): per bar (mean, max, min, hh − ll of close) once all exist.
fn rainbow_core(b: &Bars, step: usize, lb: usize) -> Vec<Option<(f64, f64, f64, f64)>> {
    let mut ms = vec![ta::sma(&b.close, step)];
    for _ in 1..10 { let nx = ta::sma(ms.last().unwrap(), step); ms.push(nx); }
    let (hh, ll) = (ta::highest(&b.close, lb), ta::lowest(&b.close, lb));
    (0..b.len()).map(|i| {
        if hh[i].is_nan() || ms.iter().any(|m| m[i].is_nan()) { return None; }
        let mean = ms.iter().map(|m| m[i]).sum::<f64>() / 10.;
        let (mx, mn) = ms.iter().fold((f64::MIN, f64::MAX), |(a, c), m| (a.max(m[i]), c.min(m[i])));
        Some((mean, mx, mn, hh[i] - ll[i]))
    }).collect()
}
fn rainbow_osc(b: &Bars, prm: &Params) -> Output {
    let core = rainbow_core(b, n(prm, "step"), n(prm, "lookback"));
    let osc: Vec<f64> = core.iter().enumerate().map(|(i, c)| c.map_or(NA, |(m, _, _, r)| 100. * (b.close[i] - m) / r.max(1e-6))).collect();
    let up: Vec<f64> = core.iter().map(|c| c.map_or(NA, |(_, mx, mn, r)| 100. * (mx - mn) / r.max(1e-6))).collect();
    let lo = map(&up, |v| -v);
    Ok(vec![osc, up, lo])
}
fn rmo(b: &Bars, prm: &Params) -> Output {
    let core = rainbow_core(b, n(prm, "step"), n(prm, "lookback"));
    let s1: Vec<f64> = core.iter().enumerate().map(|(i, c)| c.map_or(NA, |(m, _, _, r)| if r != 0. { 100. * (b.close[i] - m) / r } else { 0. })).collect();
    let e1 = n(prm, "ema1");
    let s2 = ta::ema(&s1, e1);
    let s3 = ta::ema(&s2, e1);
    let r = ta::ema(&s1, n(prm, "ema2"));
    Ok(vec![s1, s2, s3, r])
}

// ---- Repulse ----
const L10: &[ParamSpec] = &[len("length", "Length", 10.)];
fn repulse(b: &Bars, prm: &Params) -> Output {
    let k = n(prm, "length");
    let (hh, ll) = (ta::highest(&b.high, k), ta::lowest(&b.low, k));
    let o = |i: usize| if i + 1 >= k { b.open[i + 1 - k] } else { NA };
    let bull: Vec<f64> = (0..b.len()).map(|i| div(100. * (3. * b.close[i] - 2. * ll[i] - o(i)), b.close[i])).collect();
    let bear: Vec<f64> = (0..b.len()).map(|i| div(100. * (o(i) + 2. * hh[i] - 3. * b.close[i]), b.close[i])).collect();
    Ok(vec![zip(&ta::ema(&bull, 5 * k), &ta::ema(&bear, 5 * k), |a, c| a - c)])
}

// ---- Reverse-engineered RSI ----
const REV: &[ParamSpec] = &[len("length", "Length", 14.), real("target", "Target RSI", 50., 1., 99., 0.5)];
fn rev_rsi(b: &Bars, prm: &Params) -> Output {
    let (k, t) = (n(prm, "length"), p(prm, "target"));
    let d = ta::change(&b.close, 1);
    let (up, dn) = (ta::rma(&gains(&d, true), k), ta::rma(&gains(&d, false), k));
    Ok(vec![(0..b.len()).map(|i| {
        let x = (k as f64 - 1.) * (dn[i] * t / (100. - t) - up[i]);
        if x >= 0. { b.close[i] + x } else { b.close[i] + x * (100. - t) / t } // NaN x lands here and stays NaN
    }).collect()])
}

// ---- ROC with RMS bands ----
const RCB: &[ParamSpec] = &[len("length", "ROC length", 12.), len("avg_length", "ROC EMA length", 3.), len("rms_length", "RMS length", 12.)];
fn roc_bands(b: &Bars, prm: &Params) -> Output {
    let r = roc_s(&b.close, n(prm, "length"));
    let up = map(&ta::sma(&map(&r, |v| v * v), n(prm, "rms_length")), f64::sqrt);
    let lo = map(&up, |v| -v);
    Ok(vec![ta::ema(&r, n(prm, "avg_length")), up, lo])
}

// ---- Ehlers RocketRSI / RSIH ----
const ROCKET: &[ParamSpec] = &[len("rsi_length", "RSI length", 10.), int("smooth", "SuperSmoother length", 8., 2., 2000.)];
fn rocket_rsi(b: &Bars, prm: &Params) -> Output {
    let (k, s) = (n(prm, "rsi_length"), p(prm, "smooth"));
    let c = &b.close;
    // Ehlers' EasyLanguage constants verbatim (1.414, 3.14159; Cosine takes degrees)
    let a1 = (-1.414 * 3.14159 / s).exp();
    let c2 = 2. * a1 * (1.414 * 180. / s).to_radians().cos();
    let c3 = -a1 * a1;
    let c1 = 1. - c2 - c3;
    let mom = |i: usize| c[i] - c[i + 1 - k];
    let mut f = vec![0.; c.len()];
    for i in k.max(1)..c.len() { f[i] = c1 * (mom(i) + mom(i - 1)) / 2. + c2 * f[i - 1] + c3 * if i >= 2 { f[i - 2] } else { 0. }; }
    let mut my = 0.;
    Ok(vec![(0..c.len()).map(|i| if i < 2 * k { NA } else {
        let (mut cu, mut cd) = (0., 0.);
        for j in 0..k { let d = f[i - j] - f[i - j - 1]; if d > 0. { cu += d; } else if d < 0. { cd -= d; } }
        if cu + cd != 0. { my = (cu - cd) / (cu + cd); }
        let m: f64 = my.clamp(-0.999, 0.999);
        0.5 * ((1. + m) / (1. - m)).ln()
    }).collect()])
}
fn rsih(b: &Bars, prm: &Params) -> Output {
    let k = n(prm, "length"); let c = &b.close;
    let w: Vec<f64> = (0..=k).map(|j| 1. - (2. * std::f64::consts::PI * j as f64 / (k as f64 + 1.)).cos()).collect();
    let mut my = 0.;
    Ok(vec![(0..c.len()).map(|i| if i < k { NA } else {
        let (mut cu, mut cd) = (0., 0.);
        for j in 1..=k { let d = c[i + 1 - j] - c[i - j]; if d > 0. { cu += w[j] * d; } else if d < 0. { cd -= w[j] * d; } }
        if cu + cd != 0. { my = (cu - cd) / (cu + cd); }
        my
    }).collect()])
}

// ---- RSS / RSX / Slow RSI ----
const RSS: &[ParamSpec] = &[len("fast", "Fast EMA", 10.), len("slow", "Slow EMA", 40.), len("rsi_length", "RSI length", 5.), len("smooth", "SMA length", 5.)];
fn rss(b: &Bars, prm: &Params) -> Output {
    let spread = zip(&ta::ema(&b.close, n(prm, "fast")), &ta::ema(&b.close, n(prm, "slow")), |a, c| a - c);
    Ok(vec![ta::sma(&rsi_s(&spread, n(prm, "rsi_length")), n(prm, "smooth"))])
}
/// Line-by-line port of pandas_ta/momentum/rsx.py (Jurik RSX).
fn rsx(b: &Bars, prm: &Params) -> Output {
    let k = n(prm, "length"); let c = &b.close; let lf = k as f64;
    let mut out = vec![NA; c.len()];
    if k <= c.len() { out[k - 1] = 50.; }
    let (mut f0, mut f8, mut f18, mut f20, mut f28, mut f30, mut f38, mut f40, mut f48, mut f50) = (0f64, 0f64, 0f64, 0f64, 0f64, 0f64, 0f64, 0f64, 0f64, 0f64);
    let (mut f58, mut f60, mut f68, mut f70, mut f78, mut f80, mut f88, mut f90) = (0f64, 0f64, 0f64, 0f64, 0f64, 0f64, 0f64, 0f64);
    let (mut v14, mut v20) = (0f64, 0f64);
    for i in k..c.len() {
        if f90 == 0. {
            f90 = 1.; f0 = 0.;
            f88 = if lf - 1. >= 5. { lf - 1. } else { 5. };
            f8 = 100. * c[i]; f18 = 3. / (lf + 2.); f20 = 1. - f18;
        } else {
            f90 = if f88 <= f90 { f88 + 1. } else { f90 + 1. };
            let f10 = f8; f8 = 100. * c[i]; let v8 = f8 - f10;
            f28 = f20 * f28 + f18 * v8; f30 = f18 * f28 + f20 * f30; let vc = 1.5 * f28 - 0.5 * f30;
            f38 = f20 * f38 + f18 * vc; f40 = f18 * f38 + f20 * f40; let v10 = 1.5 * f38 - 0.5 * f40;
            f48 = f20 * f48 + f18 * v10; f50 = f18 * f48 + f20 * f50; v14 = 1.5 * f48 - 0.5 * f50;
            f58 = f20 * f58 + f18 * v8.abs(); f60 = f18 * f58 + f20 * f60; let v18 = 1.5 * f58 - 0.5 * f60;
            f68 = f20 * f68 + f18 * v18; f70 = f18 * f68 + f20 * f70; let v1c = 1.5 * f68 - 0.5 * f70;
            f78 = f20 * f78 + f18 * v1c; f80 = f18 * f78 + f20 * f80; v20 = 1.5 * f78 - 0.5 * f80;
            if f88 >= f90 && f8 != f10 { f0 = 1.; }
            if f88 == f90 && f0 == 0. { f90 = 0.; }
        }
        out[i] = if f88 < f90 && v20 > 0.0000000001 { ((v14 / v20 + 1.) * 50.).clamp(0., 100.) } else { 50. };
    }
    Ok(vec![out])
}
const SRSI: &[ParamSpec] = &[len("length", "RSI length", 14.), len("ema_length", "EMA length", 6.)];
fn slow_rsi(b: &Bars, prm: &Params) -> Output {
    let e = ta::ema(&b.close, n(prm, "ema_length"));
    let d = zip(&b.close, &e, |c, e| c - e); // NaN until the EMA exists
    Ok(vec![rsi_like(&gains(&d, true), &gains(&d, false), n(prm, "length"))])
}

// ---- Schaff Trend Cycle ----
const STC: &[ParamSpec] = &[len("fast", "Fast", 23.), len("slow", "Slow", 50.), len("cycle", "Cycle", 10.), real("factor", "Factor", 0.5, 0.01, 1., 0.01)];
fn stc(b: &Bars, prm: &Params) -> Output {
    let (mut f, mut s) = (n(prm, "fast"), n(prm, "slow"));
    if s < f { std::mem::swap(&mut f, &mut s); }
    let (c, fac) = (n(prm, "cycle"), p(prm, "factor"));
    // TA-Lib MACD seeding: both EMA seed windows end on bar slow − 1
    let mut late = b.close.clone();
    for v in late.iter_mut().take(s - f) { *v = NA; }
    let m = zip(&ta::ema(&late, f), &ta::ema(&b.close, s), |a, d| a - d);
    let pf = smooth_first(&stoch_hold(&m, c), fac);
    Ok(vec![smooth_first(&stoch_hold(&pf, c), fac)])
}

// ---- Stochastic MACD ----
const STM: &[ParamSpec] = &[len("fast", "Fast", 12.), len("slow", "Slow", 26.), len("signal", "Signal", 9.), len("stoch_length", "Stochastic length", 45.)];
fn stoch_macd(b: &Bars, prm: &Params) -> Output {
    let k = n(prm, "stoch_length");
    let (hh, ll) = (ta::highest(&b.high, k), ta::lowest(&b.low, k));
    let (ef, es) = (ta::ema(&b.close, n(prm, "fast")), ta::ema(&b.close, n(prm, "slow")));
    let (mut fs, mut ss) = (0., 0.);
    let st: Vec<f64> = (0..b.len()).map(|i| {
        if hh[i].is_nan() || ef[i].is_nan() || es[i].is_nan() { return NA; }
        if hh[i] != ll[i] { fs = (ef[i] - ll[i]) / (hh[i] - ll[i]); ss = (es[i] - ll[i]) / (hh[i] - ll[i]); }
        (fs - ss) * 100.
    }).collect();
    let sig = ta::ema(&st, n(prm, "signal"));
    Ok(vec![st, sig])
}

// ---- SVE Stochastic RSI ----
const SVE: &[ParamSpec] = &[len("rsi_length", "RSI length", 21.), len("stoch_length", "Stochastic length", 5.), len("avg_length", "Average length", 8.)];
fn sve_stochrsi(b: &Bars, prm: &Params) -> Output {
    let (sk, a) = (n(prm, "stoch_length"), n(prm, "avg_length"));
    let r = rsi_s(&b.close, n(prm, "rsi_length"));
    let w: Vec<Option<(f64, f64)>> = (0..r.len()).map(|i| win(&r, sk, i)).collect();
    let num: Vec<f64> = (0..r.len()).map(|i| w[i].map_or(NA, |(lo, _)| r[i] - lo)).collect();
    let den: Vec<f64> = w.iter().map(|x| x.map_or(NA, |(lo, hi)| hi - lo)).collect();
    Ok(vec![zip(&ta::sma(&num, a), &ta::sma(&den, a), |x, y| div(100. * x, 0.1 + y))])
}

// ---- Sentiment Zone Oscillator ----
const SZO: &[ParamSpec] = &[len("length", "Length", 14.), len("level_length", "Dynamic level length", 30.), real("pct", "Dynamic level percent", 95., 50., 100., 0.5)];
fn szo(b: &Bars, prm: &Params) -> Output {
    let (k, lk, q) = (n(prm, "length"), n(prm, "level_length"), 1. - p(prm, "pct") / 100.);
    let c = &b.close;
    let r: Vec<f64> = (0..c.len()).map(|i| if i == 0 { NA } else if c[i] > c[i - 1] { 1. } else { -1. }).collect();
    let e1 = ta::ema(&r, k); let e2 = ta::ema(&e1, k); let e3 = ta::ema(&e2, k);
    let s: Vec<f64> = (0..c.len()).map(|i| 100. * (3. * (e1[i] - e2[i]) + e3[i]) / k as f64).collect();
    let w: Vec<Option<(f64, f64)>> = (0..s.len()).map(|i| win(&s, lk, i)).collect();
    let ob = w.iter().map(|x| x.map_or(NA, |(lo, hi)| hi - (hi - lo) * q)).collect();
    let os = w.iter().map(|x| x.map_or(NA, |(lo, hi)| lo + (hi - lo) * q)).collect();
    Ok(vec![s, ob, os])
}

// ---- Traders Dynamic Index ----
const TDI: &[ParamSpec] = &[len("length", "RSI length", 13.), len("band_length", "Band length", 34.), real("band_mult", "Band multiplier", 1.6185, 0.1, 5., 0.0001),
    len("price_line", "Price line SMA", 2.), len("signal", "Signal SMA", 7.)];
fn tdi(b: &Bars, prm: &Params) -> Output {
    let r = rsi_s(&b.close, n(prm, "length"));
    let (bl, m) = (n(prm, "band_length"), p(prm, "band_mult"));
    let (mid, sd) = (ta::sma(&r, bl), ta::stdev(&r, bl));
    let up = zip(&mid, &sd, |a, d| a + m * d); let lo = zip(&mid, &sd, |a, d| a - m * d);
    Ok(vec![ta::sma(&r, n(prm, "price_line")), ta::sma(&r, n(prm, "signal")), up, mid, lo])
}

// ---- Trend Intensity Index ----
const TII: &[ParamSpec] = &[len("length", "Length", 14.), len("signal", "Signal EMA", 9.)];
fn tii(b: &Bars, prm: &Params) -> Output {
    let k = n(prm, "length"); let m = k.div_ceil(2);
    let dev = zip(&b.close, &ta::sma(&b.close, k), |c, s| c - s);
    let t: Vec<f64> = (0..dev.len()).map(|i| if dev[i].is_nan() { NA } else {
        // ChartIQ: the window grows from the first SMA bar up to m deviations
        let (up, dn) = dev[i + 1 - m.min(i + 1)..=i].iter().filter(|v| !v.is_nan()).fold((0., 0.), |(u, d), &v| if v < 0. { (u, d - v) } else { (u + v, d) });
        if dn < 1e-8 { 100. } else { 100. * up / (up + dn) }
    }).collect();
    let sig = ta::ema(&t, n(prm, "signal"));
    Ok(vec![t, sig])
}

// ---- True Momentum Oscillator ----
const TMO: &[ParamSpec] = &[len("length", "Length", 14.), len("calc", "Calc length", 5.), len("smooth", "Smooth length", 3.)];
fn tmo(b: &Bars, prm: &Params) -> Output {
    let (k, sm) = (n(prm, "length"), n(prm, "smooth"));
    let data: Vec<f64> = (0..b.len()).map(|t| if t + 1 < k { NA } else { (0..k).map(|i| sign(b.close[t] - b.open[t - i])).sum() }).collect();
    let main = ta::ema(&ta::ema(&data, n(prm, "calc")), sm);
    let sig = ta::ema(&main, sm);
    Ok(vec![main, sig])
}

// ---- Value Chart ----
const VC: &[ParamSpec] = &[len("length", "Length", 5.)];
fn value_chart(b: &Bars, prm: &Params) -> Output {
    let k = n(prm, "length");
    let m = ta::sma(&b.hl2(), k);
    let r = map(&ta::sma(&ta::tr(b, true), k), |v| v / 5.);
    let f = |x: &[f64]| (0..x.len()).map(|i| div(x[i] - m[i], r[i])).collect::<Vec<f64>>();
    Ok(vec![f(&b.open), f(&b.high), f(&b.low), f(&b.close)])
}

// ---- Velocity and acceleration ----
const VEL: &[ParamSpec] = &[len("length", "Length", 21.), len("smooth", "Smoothing EMA", 5.)];
/// (1/k)·Σ_{i=1..k}(x − x[i])/i (NaN propagates from x[t − k..=t]).
fn speed(x: &[f64], k: usize) -> Vec<f64> {
    (0..x.len()).map(|t| if t < k { NA } else { (1..=k).map(|i| (x[t] - x[t - i]) / i as f64).sum::<f64>() / k as f64 }).collect()
}
fn velocity(b: &Bars, prm: &Params) -> Output {
    let k = n(prm, "length");
    let v = ta::ema(&speed(&b.close, k), n(prm, "smooth"));
    let a = speed(&v, k);
    Ok(vec![v, a])
}

// ---- Volume-weighted MACD ----
const MACD: &[ParamSpec] = &[len("fast", "Fast", 12.), len("slow", "Slow", 26.), len("signal", "Signal", 9.)];
fn vwma_s(c: &[f64], v: &[f64], k: usize) -> Vec<f64> { zip(&ta::sma(&zip(c, v, |c, v| c * v), k), &ta::sma(v, k), div) }
fn vw_macd(b: &Bars, prm: &Params) -> Output {
    let v = b.volume.as_deref().ok_or("needs volume")?;
    let m = zip(&vwma_s(&b.close, v, n(prm, "fast")), &vwma_s(&b.close, v, n(prm, "slow")), |a, c| a - c);
    let s = ta::ema(&m, n(prm, "signal"));
    let h = zip(&m, &s, |a, c| a - c);
    Ok(vec![m, s, h])
}

// ---- WaveTrend ----
const WT: &[ParamSpec] = &[len("channel", "Channel length", 10.), len("average", "Average length", 21.), len("signal", "Signal SMA", 4.)];
fn wavetrend(b: &Bars, prm: &Params) -> Output {
    let ch = n(prm, "channel");
    let ap = b.hlc3();
    // TA-Lib: price equal to its average (to rounding) counts as zero distance
    let dist = zip(&ap, &ta::ema(&ap, ch), |a, e| if (a - e).abs() <= 1e-10 * a.abs() { 0. } else { a - e });
    let d = ta::ema(&map(&dist, f64::abs), ch);
    let ci = zip(&dist, &d, |x, d| if d.is_nan() { NA } else if d == 0. { 0. } else { x / (0.015 * d) });
    let wt1 = ta::ema(&ci, n(prm, "average"));
    let wt2 = ta::sma(&wt1, n(prm, "signal"));
    let diff = zip(&wt1, &wt2, |a, c| a - c);
    Ok(vec![wt1, wt2, diff])
}

pub const DEFS: &[IndicatorDef] = &[
    def("projection_osc", "Projection Oscillator (Widner)", M, PROJ, &["po", "signal"], PN, NO,
        "slope_h, slope_l = least-squares slopes of high and low over n; upper = max_{i<n}(high[i] + i·slope_h), lower = min_{i<n}(low[i] + i·slope_l) (i = bars ago, Projection Bands); po = 100·(close − lower)/(upper − lower) (zero width → NaN); signal = lab ema(po, signal) (Widner TASC Jul 1995, original code not online; formula as LuxAlgo's published Projection Bands/Oscillator definition Upper_t = max_{i<n}(high_{t−i} + i·m_high), Lower_t = min_{i<n}(low_{t−i} + i·m_low); defaults 14/3 EMA as MotiveWave)", &[13, 15], projection_osc),
    def("pzo", "Price Zone Oscillator", M, L14, &["pzo"], PN, NO,
        "pzo = 100·ema(sign(close − close[1])·close, n)/ema(close, n), lab ema seeds (Khalil & Steckler, Traders' Tips Jun 2011: TradeStation R = Sign(Close − Close[1])·Close, CP = XAverage(R, 14), TC = XAverage(Close, 14), PZO = 100·CP/TC; thinkorswim identical with ExpAverage; unchanged close → 0 as Sign, MetaStock's tip uses −C there — not followed; XAverage seeds with the first value); levels ±60, ±40, ±5", &[14], pzo),
    def("qqe", "QQE (Quantitative Qualitative Estimation)", M, QQE, &["rsi_ma", "qqe", "trend"], PN, NO,
        "rsi_ma = ema(rsi(close, n), smooth); dar = factor·ema(ema(|rsi_ma − rsi_ma[1]|, 2n−1), 2n−1); long = rsi_ma[1] > long[1] and rsi_ma > long[1] ? max(long[1], rsi_ma − dar) : rsi_ma − dar; short mirrored with min and rsi_ma + dar; trend = 1 when rsi_ma crosses short[1] (either direction, Pine cross), −1 when long[1] crosses rsi_ma, else trend[1] (starts 1); qqe = trend 1 ? long : short (QQE Pine by colinmck/glaz, as pandas-ta qqe; lab ema seeds; long/short/trend start on the first dar bar)", &[18, 71, 71], qqe),
    def("rainbow_osc", "Rainbow Oscillator (Widner)", M, RB, &["osc", "upper", "lower"], PN, NO,
        "m1 = sma(close, step), m_k = sma(m_{k−1}, step), k = 2..10; rng = max(highest(close, lookback) − lowest(close, lookback), 1e−6); osc = 100·(close − mean(m1..m10))/rng; upper = 100·(max(m_k) − min(m_k))/rng; lower = −upper (ChartIQ calculateRainbow incl. its 1e−6 floor; starts once all ten averages exist; Widner TASC 1997)", &[10, 10, 10], rainbow_osc),
    def("repulse", "Repulse", M, L10, &["repulse"], PN, NO,
        "bull = 100·(3·close − 2·lowest(low, n) − open[n−1])/close; bear = 100·(open[n−1] + 2·highest(high, n) − 3·close)/close; repulse = ema(bull, 5n) − ema(bear, 5n) (Sierra Chart Repulse; volume not used; lab: full windows only and lab ema seeds, Sierra Chart starts at bar 0 with partial windows and its own EMA warm-up)", &[58], repulse),
    def("rev_rsi", "Reverse-Engineered RSI (price for an RSI level)", M, REV, &["price"], PR, NO,
        "up/dn = Wilder averages of gains/losses over n (as rsi); x = (n − 1)·(dn·target/(100 − target) − up); price = x ≥ 0 ? close + x : close + x·(100 − target)/target: the next close that puts rsi at target (Siligardos, TASC Jun 2003 Traders' Tips TradeStation RevEngRSI: AUC/ADC = XAverage(gain/loss, 2n − 1) (alpha 1/n = Wilder; our rma seeds with the SMA), X = (n − 1)·(ADC·RSIValue/(100 − RSIValue) − AUC), RevEngRSI = X ≥ 0 ? C + X : C + X·(100 − RSIValue)/RSIValue, defaults 14/50; TradeStation plots it one bar forward (Plot1[-1]), here it stays on the bar it is computed from)", &[14], rev_rsi),
    def("rmo", "Rahul Mohindar Oscillator", M, RMO, &["swing1", "swing2", "swing3", "rmo"], PN, NO,
        "m1 = sma(close, step), m_k = sma(m_{k−1}, step), k = 2..10; swing1 = 100·(close − mean(m1..m10))/(highest(close, lookback) − lowest(close, lookback)) (zero range → 0); swing2 = ema(swing1, ema1); swing3 = ema(swing2, ema1); rmo = ema(swing1, ema2) (Sierra Chart Rahul Mohindar Oscillator; lab ema seeds)", &[10, 39, 68, 90], rmo),
    def("roc_bands", "Rate of Change with RMS Bands", M, RCB, &["roc_avg", "upper", "lower"], PN, NO,
        "roc = 100·(close − close[n])/close[n]; roc_avg = ema(roc, avg_length); upper = sqrt(sma(roc², rms_length)); lower = −upper (Apirine, TASC Mar 2021, Traders' Tips TradeStation: oROCDev = SquareRoot(Average(Square(RateOfChange(Price, 12)), 12)), oMARoc = XAverage(RateOfChg, 3), defaults 12/3/12; thinkorswim RateOfChangeWithBands cites it; lab ema seeds with the SMA, XAverage with the first value)", &[14, 23, 23], roc_bands),
    def("rocket_rsi", "Ehlers Rocket RSI", M, ROCKET, &["rocket"], PN, NO,
        "Ehlers TASC May 2018 EasyLanguage: a1 = exp(−1.414·3.14159/smooth), c2 = 2·a1·cos(1.414·180°/smooth), c3 = −a1², c1 = 1 − c2 − c3; mom = close − close[L − 1]; filt = c1·(mom + mom[1])/2 + c2·filt[1] + c3·filt[2] (filt 0 before bar L); cu/cd = sums of positive/negative filt − filt[1] over L; myrsi = (cu − cd)/(cu + cd) (kept when cu + cd = 0), clamped ±0.999; rocket = 0.5·ln((1 + myrsi)/(1 − myrsi)); output from bar 2L; smooth default 8 as thinkorswim (Ehlers uses 10)", &[20], rocket_rsi),
    def("rsih", "Ehlers Hann-Windowed RSI (RSIH)", M, L14, &["rsih"], PN, NO,
        "Ehlers TASC Jan 2022 EasyLanguage: for i = 1..n, w = 1 − cos(360°·i/(n + 1)), d = close[i−1] − close[i]: cu += w·max(d, 0), cd += w·max(−d, 0); rsih = (cu − cd)/(cu + cd) (kept when cu + cd = 0), −1..1", &[14], rsih),
    def("rss", "Relative Spread Strength (Copsey)", M, RSS, &["rss"], PN, NO,
        "spread = ema(close, fast) − ema(close, slow); rss = sma(rsi(spread, rsi_length), smooth) (NinjaTrader 8 RSS.cs; lab ema/rsi seeds, NinjaTrader seeds its EMA on bar 0 and its RSI SMA seed includes bar 0's zero change)", &[48], rss),
    def("rsx", "RSX (smoothed RSI)", M, L14, &["rsx"], PN, NO,
        "Jurik RSX as pandas_ta rsx.py line by line: from bar n, three cascaded double-smoothing stages (f18 = 3/(n + 2), f20 = 1 − f18, stage = 1.5·a − 0.5·b) of 100·Δclose and of |100·Δclose|; rsx = 50·(v14/v20 + 1) clamped 0..100 once the max(n − 1, 5)-bar warm-up counter has run (else 50); bar n − 1 = 50", &[13], rsx),
    def("slow_rsi", "Slow RSI (Apirine)", M, SRSI, &["srsi"], PN, NO,
        "e = ema(close, ema_length); up = rma(max(close − e, 0), n); dn = rma(max(e − close, 0), n); srsi = 100 − 100/(1 + up/dn) (dn 0 → 100) (Apirine TASC Apr 2015, as the Jul 2015 AmiBroker and eSignal Traders' Tips; TradeStation's tip computes an RSI of the EMA instead)", &[18], slow_rsi),
    def("stc", "Schaff Trend Cycle", M, STC, &["stc"], PN, NO,
        "TA-Lib STC: m = ema(close, fast) − ema(close, slow) (both seed windows end on bar slow − 1, as TA-Lib MACD); frac1 = 100·(m − lowest(m, cycle))/(highest − lowest) over full windows (zero range keeps the previous frac1, 0 before any); pf = pf[1] + factor·(frac1 − pf[1]) seeded with its first input; frac2 the same on pf; stc = stc[1] + factor·(frac2 − stc[1]) seeded with its first input", &[67], stc),
    def("stoch_macd", "Stochastic MACD (Apirine)", M, STM, &["stmacd", "signal"], PN, NO,
        "hh = highest(high, L), ll = lowest(low, L); stmacd = 100·((ema(close, fast) − ll)/(hh − ll) − (ema(close, slow) − ll)/(hh − ll)) (hh = ll keeps the previous ratios); signal = ema(stmacd, signal) (Apirine TASC Nov 2019, TradeStation Traders' Tip; lab ema seeds)", &[44, 52], stoch_macd),
    def("sve_stochrsi", "SVE Stochastic RSI (Vervoort)", M, SVE, &["svestochrsi"], PN, NO,
        "r = rsi(close, rsi_length); svestochrsi = 100·sma(r − lowest(r, stoch), avg)/(0.1 + sma(highest(r, stoch) − lowest(r, stoch), avg)) (full windows) (Vervoort, TASC Aug 2018; Traders' Tips Feb 2019 TradeStation: SVEStochRSI = AvgRSILow/(0.1 + AvgHiLow)·100 with Average = SMA; defaults 21/5/8 as the AIQ and NeuroShell tips, TradeStation uses RSI 13)", &[32], sve_stochrsi),
    def("szo", "Sentiment Zone Oscillator", M, SZO, &["szo", "ob", "os"], PN, NO,
        "r = close > close[1] ? 1 : −1; szo = 100·tema(r, n)/n (tema = 3·e1 − 3·e2 + e3, lab ema seeds); ob = hh − (hh − ll)·(1 − pct/100), os = ll + (hh − ll)·(1 − pct/100), hh/ll = highest/lowest(szo, level_length) (Khalil & Steckler, Traders' Tips May 2012 TradeStation/MetaStock: R = C > C[1] ? 1 : −1, SZO = 100·TEMA(R, 14)/14, levels from highest/lowest SZO over 30 at 95%; thinkorswim uses Sign (0 on an unchanged close) — not followed; XAverage seeds with the first value)", &[40, 69, 69], szo),
    def("tdi", "Traders Dynamic Index", M, TDI, &["price_line", "signal", "upper", "middle", "lower"], PN, NO,
        "r = rsi(close, n); price_line = sma(r, price_line); signal = sma(r, signal); middle = sma(r, band_length); upper/lower = middle ± band_mult·stdev(r, band_length) (population) (Sierra Chart Traders Dynamic Index; lab Wilder rsi)", &[14, 19, 46, 46, 46], tdi),
    def("tii", "Trend Intensity Index", M, TII, &["tii", "signal"], PN, NO,
        "dev = close − sma(close, n); over the last m = ceil(n/2) deviations (fewer on the first bars after the SMA starts, as ChartIQ): up = Σ positive dev, dn = Σ |negative dev|; tii = 100·up/(up + dn) (dn < 1e−8 → 100); signal = ema(tii, signal) (ChartIQ calculateTrendIntensity; lab ema seed)", &[13, 21], tii),
    def("tmo", "True Momentum Oscillator", M, TMO, &["main", "signal"], PN, NO,
        "data = Σ_{i=0..n−1} sign(close − open[i]) (thinkorswim TMO fold incl. the current open; pandas-ta's default window is open[1..n]); main = ema(ema(data, calc), smooth); signal = ema(main, smooth) (lab ema seeds = pandas-ta presma)", &[19, 21], tmo),
    def("value_chart", "Value Chart (Helweg)", M, VC, &["v_open", "v_high", "v_low", "v_close"], PN, NO,
        "m = sma(hl2, n); r = sma(true range, n)/5 (volatility unit; bar 0 range = high − low); v_x = (x − m)/r for x in open, high, low, close; r 0 → NaN (Sierra Chart Value Chart, length ≤ 7 form; its block-range form for longer lengths not built); levels ±4, ±8", &[4, 4, 4, 4], value_chart),
    def("velocity", "Velocity and Acceleration", M, VEL, &["velocity", "acceleration"], PN, NO,
        "v_raw = (1/n)·Σ_{i=1..n}(close − close[i])/i; velocity = ema(v_raw, smooth); acceleration = (1/n)·Σ_{i=1..n}(velocity − velocity[i])/i (Scott Cong, TASC Sep 2023; Traders' Tips Nov 2023 Zorro: Avg_vel = EMA(Sum_vel/Length, EMA_Smooth), return Sum_acc/Length; defaults 21/5 as the TradeStation and NinjaTrader tips; unscaled — the TradeStation/Pine display factors 100·2/n and 100 are not applied; lab ema seeds with the SMA)", &[25, 46], velocity),
    def("vw_macd", "Volume-Weighted MACD", M, MACD, &["macd", "signal", "hist"], PN, VOL,
        "vwma(x, k) = sma(close·volume, k)/sma(volume, k); macd = vwma(fast) − vwma(slow); signal = ema(macd, signal); hist = macd − signal (thinkorswim VolumeWeightedMACD; Sierra Chart MACD-Volume Weighted)", &[25, 33, 33], vw_macd),
    def("wavetrend", "WaveTrend Oscillator", M, WT, &["wt1", "wt2", "diff"], PN, NO,
        "ap = hlc3; esa = ema(ap, channel); dist = ap − esa (|dist| ≤ 1e−10·|ap| → 0); d = ema(|dist|, channel); ci = dist/(0.015·d) (d 0 → 0, TA-Lib); wt1 = ema(ci, average); wt2 = sma(wt1, signal); diff = wt1 − wt2 (LazyBear WaveTrend; TA-Lib WAVETREND, SMA-seeded EMAs)", &[38, 41, 41], wavetrend),
];
/// (id, platform names, AmiBroker names): one row per id in DEFS.
pub const SOURCES: &[(&str, &str, &str)] = &[
    ("projection_osc", "MetaStock: Projection Oscillator; thinkorswim: ProjectionOscillator", ""),
    ("pzo", "thinkorswim: PriceZoneOscillator", ""),
    ("qqe", "pandas-ta: qqe; TradeStation: QQE; TradingView community: QQE signals", ""),
    ("rainbow_osc", "Zerodha Kite (ChartIQ): Rainbow Oscillator; MetaStock: Rainbow Oscillator; cTrader: RainbowOscillator", ""),
    ("repulse", "Sierra Chart: Repulse", ""),
    ("rev_rsi", "thinkorswim: ReverseEngineeringRSI", ""),
    ("rmo", "Sierra Chart: Rahul Mohindar Oscillator", ""),
    ("roc_bands", "thinkorswim: RateOfChangeWithBands", ""),
    ("rocket_rsi", "thinkorswim: RocketRSI", ""),
    ("rsih", "thinkorswim: RSIH", ""),
    ("rss", "NinjaTrader 8: RSS", ""),
    ("rsx", "pandas-ta: rsx", ""),
    ("slow_rsi", "thinkorswim: SlowRSI", ""),
    ("stc", "TA-Lib: STC; pandas-ta: stc; thinkorswim: SchaffTrendCycle; Sierra Chart: Schaff Trend Cycle; TA4J: SchaffTrendCycle; Zerodha Kite (ChartIQ): Schaff Trend Cycle; Chartink: Schaff Trend Cycle", ""),
    ("stoch_macd", "thinkorswim: StochasticMACD", ""),
    ("sve_stochrsi", "thinkorswim: SVEStochRSI", ""),
    ("szo", "thinkorswim: SentimentZoneOscillator", ""),
    ("tdi", "Sierra Chart: Traders Dynamic Index", ""),
    ("tii", "Zerodha Kite (ChartIQ): Trend Intensity Index; Zerodha Streak: TII; Dhan: Trend Intensity Index", ""),
    ("tmo", "pandas-ta: tmo", ""),
    ("value_chart", "Sierra Chart: Value Chart", ""),
    ("velocity", "thinkorswim: Velocity; thinkorswim: Acceleration", ""),
    ("vw_macd", "thinkorswim: VolumeWeightedMACD; Sierra Chart: MACD-Volume Weighted", ""),
    ("wavetrend", "TA-Lib: WAVETREND; Sierra Chart: WaveTrend; Chartink: WaveTrend; Dhan: WaveTrend; TradingView community: WaveTrend Oscillator [LazyBear]", ""),
];

#[cfg(test)]
mod tests {
    use crate::lab::testdata::{assert_series, closes, compiles, d1, from_ohlc, go, walk};
    const N: f64 = f64::NAN;
    fn rising(n: usize) -> crate::lab::bars::Bars { closes(&(0..n).map(|i| 100. * 1.01f64.powi(i as i32)).collect::<Vec<_>>()) }
    fn last(v: &[f64]) -> f64 { *v.last().unwrap() }
    fn near(a: f64, b: f64) -> bool { (a - b).abs() < 1e-9 }

    #[test] fn projection_oscillator_hand_values_and_upper_band() {
        let o = go("projection_osc", &d1(), &[("length", 3.), ("signal", 2.)]); // values from an independent Python port
        assert_series(&o[0], &[N, N, 57.142857142857, 11.111111111111, 11.764705882353, 87.878787878788, 95.555555555556, 73.333333333333]);
        assert_series(&o[1], &[N, N, N, 34.126984126984, 19.218798630563, 64.992124796046, 85.367745302386, 77.344803989684]);
        // close on the projected upper band -> 100
        let b = from_ohlc(&(0..20).map(|i| { let x = i as f64; (x, x + 1., x - 1., x + 1.) }).collect::<Vec<_>>());
        assert!(near(last(&go("projection_osc", &b, &[])[0]), 100.));
    }
    #[test] fn price_zone_oscillator() {
        assert_series(&go("pzo", &d1(), &[("length", 2.)])[0], &[N, N, 100.714285714286, -30.445544554455, -75.316233309909, 42.50403318737, 82.152310774074, 94.405888420486]);
    }
    #[test] fn qqe_collapses_on_a_flat_rsi_and_brackets_rsi_ma() {
        let o = go("qqe", &rising(120), &[]); // rsi 100 every bar -> dar 0 -> long = short = rsi_ma = 100
        assert!(o[1][70].is_nan() && near(o[1][71], 100.) && near(last(&o[1]), 100.) && last(&o[2]) == 1.);
        let o = go("qqe", &walk(400, 3, false), &[]);
        for i in 71..400 { assert!(o[2][i].abs() == 1. && (o[1][i] - o[0][i]) * o[2][i] <= 1e-9, "bar {i}"); } // long ≤ rsi_ma ≤ short
        assert!(o[2].iter().skip(71).any(|&t| t == -1.) && o[2].iter().skip(71).any(|&t| t == 1.));
    }
    #[test] fn rainbow_and_rmo_on_a_ramp_and_flat() {
        let ramp = closes(&(0..40).map(|i| i as f64).collect::<Vec<_>>()); // m_k = close − k/2, mean = close − 2.75, range 9
        let r = go("rainbow_osc", &ramp, &[]);
        assert!(r[0][9].is_nan() && near(r[0][10], 100. * 2.75 / 9.) && near(r[1][10], 50.) && near(r[2][10], -50.));
        let m = go("rmo", &ramp, &[]);
        assert!(near(m[0][10], 100. * 2.75 / 9.) && near(m[1][39], 100. * 2.75 / 9.));
        let flat = closes(&[50.; 120]);
        assert!(go("rainbow_osc", &flat, &[])[0][20] == 0. && go("rmo", &flat, &[])[3][119] == 0.);
    }
    #[test] fn repulse_hand_values_and_flat_zero() {
        assert_series(&go("repulse", &d1(), &[("length", 1.)])[0], &[N, N, N, N, 1.280433121865, 11.347449241737, 21.010344312419, 21.414303615687]);
        assert!(last(&go("repulse", &closes(&[7.; 80]), &[])[0]) == 0.);
    }
    #[test] fn reverse_rsi_price_hits_the_target() {
        let w = walk(200, 5, false);
        for target in [30., 50., 72.5] {
            let px = go("rev_rsi", &w, &[("target", target)])[0][150];
            let mut c = w.close[..=150].to_vec(); c.push(px);
            let r = go("rsi", &closes(&c), &[])[0][151];
            assert!((r - target).abs() < 1e-9, "target {target}: rsi {r}");
        }
    }
    #[test] fn roc_bands_rms_and_average() {
        let o = go("roc_bands", &d1(), &[("length", 1.), ("avg_length", 2.), ("rms_length", 2.)]);
        // roc = 100·Δc/c[1] on d1: bar1 9.5238…, bar2 4.3478…; ema(·, 2) seeds with their mean
        assert!(o[0][1].is_nan() && near(o[0][2], (100. / 10.5 + 50. / 11.5) / 2.));
        assert_series(&o[1], &[N, N, 7.402923055396, 6.646353741973, 7.821029549488, 6.61417561409, 8.316852571168, 8.316852571168]);
        assert!(near(o[2][7], -o[1][7]));
        let r = go("roc_bands", &rising(80), &[]); // constant roc -> its ema = roc, bands ±|roc|
        assert!(near(r[1][79], r[0][79]) && near(r[2][79], -r[0][79]));
    }
    #[test] fn rocket_rsi_and_rsih() {
        let q = closes(&(0..200).map(|i| 100. + (i * i) as f64 * 0.01).collect::<Vec<_>>()); // filt keeps rising -> myrsi 0.999
        assert!(near(last(&go("rocket_rsi", &q, &[])[0]), 0.5 * (1.999f64 / 0.001).ln()));
        let o = go("rocket_rsi", &walk(300, 8, false), &[])[0].clone();
        assert!(o[19].is_nan() && o[20..].iter().all(|v| v.abs() <= 3.81));
        let up = closes(&(0..30).map(|i| i as f64).collect::<Vec<_>>());
        assert!(go("rsih", &up, &[])[0][13].is_nan() && go("rsih", &up, &[])[0][14] == 1.);
        assert!(go("rsih", &walk(100, 1, false), &[])[0].iter().skip(14).all(|v| v.abs() <= 1.));
    }
    #[test] fn rss_rsx_and_slow_rsi() {
        assert!(near(last(&go("rss", &rising(100), &[])[0]), 100.)); // spread widens every bar -> rsi 100
        let up = closes(&(0..60).map(|i| 10. + i as f64).collect::<Vec<_>>());
        let x = go("rsx", &up, &[])[0].clone();
        assert!(x[12].is_nan() && x[13] == 50. && near(last(&x), 100.));
        assert!(go("rsx", &walk(300, 6, false), &[])[0].iter().skip(13).all(|v| (0. ..=100.).contains(v)));
        let s = go("slow_rsi", &up, &[])[0].clone(); // close always above its EMA
        assert!(s[17].is_nan() && s[18] == 100.);
    }
    #[test] fn schaff_trend_cycle_hand_values() {
        // independent Python port: m = c − ema(c, 2); pf = 0, 0, 0, 50, 75, 37.5
        assert_series(&go("stc", &d1(), &[("fast", 1.), ("slow", 2.), ("cycle", 2.), ("factor", 0.5)])[0], &[N, N, N, 0., 0., 50., 75., 37.5]);
        assert!(go("stc", &walk(400, 4, false), &[])[0].iter().skip(67).all(|v| (0. ..=100.).contains(v)));
    }
    #[test] fn stochastic_macd_is_scaled_macd() {
        let b = walk(200, 9, false);
        let s = go("stoch_macd", &b, &[])[0].clone();
        let m = go("macd", &b, &[])[0].clone();
        let (hh, ll) = (crate::lab::ta::highest(&b.high, 45), crate::lab::ta::lowest(&b.low, 45));
        for i in 44..200 { assert!(near(s[i], 100. * m[i] / (hh[i] - ll[i])), "bar {i}"); }
    }
    #[test] fn sve_stochrsi_matches_a_direct_ratio_of_sums() {
        let b = walk(120, 12, false);
        let v = go("sve_stochrsi", &b, &[("rsi_length", 3.), ("stoch_length", 4.), ("avg_length", 2.)])[0].clone();
        let r = go("rsi", &b, &[("length", 3.)])[0].clone();
        let lh = |i: usize| { let w = &r[i - 3..=i]; (w.iter().cloned().fold(f64::MAX, f64::min), w.iter().cloned().fold(f64::MIN, f64::max)) };
        let i = 60; let ((l0, h0), (l1, h1)) = (lh(i - 1), lh(i));
        assert!(near(v[i], 100. * ((r[i - 1] - l0) + (r[i] - l1)) / 2. / (0.1 + ((h0 - l0) + (h1 - l1)) / 2.)));
        assert!(v[6].is_nan() && v[7].is_finite());
    }
    #[test] fn sentiment_zone_all_up_closes() {
        let o = go("szo", &rising(120), &[]); // tema(1) = 1 -> szo = 100/n; flat szo -> ob = os = szo
        assert!(o[0][39].is_nan() && near(o[0][40], 100. / 14.) && near(o[1][69], 100. / 14.) && near(o[2][69], 100. / 14.));
    }
    #[test] fn traders_dynamic_index_constant_rsi_collapses_bands() {
        let o = go("tdi", &rising(80), &[]);
        for k in 0..5 { assert!(near(o[k][79], 100.)); }
    }
    #[test] fn trend_intensity_hand_values() {
        let o = go("tii", &d1(), &[("length", 4.), ("signal", 2.)]);
        assert_series(&o[0], &[N, N, N, 0., 0., 0., 82.222222222222, 100.]);
        assert_series(&o[1], &[N, N, N, N, 0., 0., 54.814814814815, 84.938271604938]);
    }
    #[test] fn true_momentum_oscillator_hand_values() {
        let o = go("tmo", &d1(), &[("length", 3.), ("calc", 2.), ("smooth", 2.)]);
        assert_series(&o[0], &[N, N, N, N, -0.333333333333, -0.925925925926, 0.753086419753, 1.938271604938]);
        assert_series(&o[1], &[N, N, N, N, N, -0.62962962963, 0.292181069959, 1.389574759945]);
    }
    #[test] fn value_chart_hand_values() {
        let o = go("value_chart", &d1(), &[("length", 2.)]);
        assert_series(&o[3], &[N, 2.5, 1.785714285714, -2.155172413793, -3.189655172414, 1.0, 3.833333333333, 2.833333333333]);
    }
    #[test] fn velocity_and_acceleration() {
        let o = go("velocity", &d1(), &[("length", 2.), ("smooth", 2.)]);
        // v_raw from bar 2 (0.625, −0.625, …); ema(·, 2) seeds with their mean 0 on bar 3
        assert_series(&o[0], &[N, N, N, 0.0, -0.566666666667, -0.022222222222, 0.642592592593, 0.747530864198]);
        assert_series(&o[1], &[N, N, N, N, N, 0.266666666667, 0.634722222222, 0.244907407407]);
        let up = closes(&(0..60).map(|i| i as f64).collect::<Vec<_>>()); // +1 per bar -> velocity 1, acceleration 0 (defaults 21/5)
        let v = go("velocity", &up, &[]);
        assert!(v[0][24].is_nan() && near(v[0][25], 1.) && v[1][45].is_nan() && near(v[1][46], 0.));
    }
    #[test] fn volume_weighted_macd() {
        let o = go("vw_macd", &d1(), &[("fast", 2.), ("slow", 3.), ("signal", 2.)]);
        assert_series(&o[0], &[N, N, 0.391705069124, -0.041666666667, -0.333590733591, -0.201923076923, 0.406851851852, 0.382764227642]);
        assert_series(&o[1], &[N, N, N, 0.175019201229, -0.164054088651, -0.189300080832, 0.208134540957, 0.324554332081]);
        let mut b = walk(100, 2, true); b.volume = Some(vec![500.; 100]); // constant volume -> sma(close, 12) − sma(close, 26)
        let (f, s) = (crate::lab::ta::sma(&b.close, 12), crate::lab::ta::sma(&b.close, 26));
        assert!(near(go("vw_macd", &b, &[])[0][60], f[60] - s[60]));
    }
    #[test] fn wavetrend_hand_values_and_flat_zero() {
        let o = go("wavetrend", &d1(), &[("channel", 2.), ("average", 2.), ("signal", 2.)]);
        assert_series(&o[0], &[N, N, N, 22.02380952381, -43.065234223771, -49.758128183523, 35.498779764415, 62.342526703629]);
        assert_series(&o[1], &[N, N, N, N, -10.520712349981, -46.411681203647, -7.129674209554, 48.920653234022]);
        assert!(last(&go("wavetrend", &closes(&[1017.3; 80]), &[])[0]) == 0.);
    }
    /// The catalogue warm-up/no-gap check for this file's entries alone (the shared test stops at the first failing id of any family).
    #[test] fn own_warmup_and_no_gaps() {
        let bars = walk(1000, 7, true);
        for d in super::DEFS {
            let out = crate::lab::catalogue::run(d, &bars, &Default::default()).unwrap();
            for (k, s) in out.iter().enumerate() {
                let first = s.iter().position(|v| !v.is_nan()).unwrap_or(usize::MAX);
                assert_eq!(first, d.first[k], "{}.{}", d.id, d.outputs[k]);
                assert!(s[first..].iter().all(|v| v.is_finite()), "{}.{} gap", d.id, d.outputs[k]);
            }
        }
    }
    #[test] fn own_parameter_bounds_never_panic() {
        for len in [0usize, 1, 3, 300] {
            let bars = walk(len, 13, true);
            for d in super::DEFS {
                for pick in [|p: &crate::lab::catalogue::ParamSpec| p.min, |p: &crate::lab::catalogue::ParamSpec| p.max] {
                    let prm = d.params.iter().map(|p| (p.key.to_string(), pick(p))).collect();
                    let out = crate::lab::catalogue::run(d, &bars, &prm).unwrap();
                    assert!(out.iter().all(|s| s.len() == len && !s.iter().any(|v| v.is_infinite())), "{} at {len} bars", d.id);
                }
            }
        }
    }
    #[test] fn rules_reach_x_momentum2() {
        for (id, out) in [("qqe", "trend"), ("rmo", "swing3"), ("tdi", "lower"), ("value_chart", "v_close"), ("vw_macd", "hist"), ("wavetrend", "diff")] { compiles(id, out).unwrap(); }
    }
}
