//! Cross-platform indicators, Volatility, Bands & channels, Statistics, Price functions (2026-10-19 build list: TradingView, AmiBroker, MetaTrader, cTrader, NinjaTrader, TradeStation, thinkorswim, Sierra Chart, MetaStock). Formulas: see each definition string.
use super::bars::Bars;
use super::catalogue::{int, len, n, p, real, IndicatorDef, Needs, Output, ParamSpec, Params, Placement};
use super::ta::{self, NA};

fn div(a: f64, b: f64) -> f64 { if b == 0. || a.is_nan() || b.is_nan() { NA } else { a / b } }
#[allow(clippy::too_many_arguments)]
const fn def(id: &'static str, name: &'static str, category: &'static str, params: &'static [ParamSpec], outputs: &'static [&'static str], placement: Placement,
             needs: Needs, definition: &'static str, first: &'static [usize], f: fn(&Bars, &Params) -> Output) -> IndicatorDef {
    IndicatorDef { id, name, category, params, outputs, placement, needs, definition, first, sparse: false, compute: Some(f), note: "" }
}
const VOL: &str = "Volatility";
const BC: &str = "Bands & channels";
const ST: &str = "Statistics";
const PF: &str = "Price functions";
const P: Placement = Placement::Price;
const PN: Placement = Placement::Pane;
const NO: Needs = Needs::NONE;
const VO: Needs = Needs { volume: true, ..Needs::NONE };

fn zip(a: &[f64], b: &[f64], f: impl Fn(f64, f64) -> f64) -> Vec<f64> { a.iter().zip(b).map(|(&x, &y)| f(x, y)).collect() }
fn scale(a: &[f64], k: f64) -> Vec<f64> { a.iter().map(|v| v * k).collect() }
/// A flat window's mean is not exact in floating point; snap tiny deviations to 0 like `ta::stdev`.
fn snap(d: f64) -> f64 { if d.abs() <= 1e-10 { 0. } else { d } }
/// f(window) for every bar with a full window of `n` bars; any NaN in the window → NaN (bar-counted).
fn rolling(x: &[f64], n: usize, f: impl Fn(&[f64]) -> f64) -> Vec<f64> {
    (0..x.len()).map(|i| if n == 0 || i + 1 < n { NA } else { let w = &x[i + 1 - n..=i]; if w.iter().any(|v| v.is_nan()) { NA } else { f(w) } }).collect()
}
/// Least squares over w with k = 1..n (oldest → newest): y = a + slope·k.
fn fit(w: &[f64]) -> (f64, f64) {
    let nf = w.len() as f64;
    let (mut sk, mut sy, mut skk, mut sky) = (0., 0., 0., 0.);
    for (j, &y) in w.iter().enumerate() { let k = (j + 1) as f64; sk += k; sy += y; skk += k * k; sky += k * y; }
    let den = nf * skk - sk * sk;
    let slope = if den == 0. { 0. } else { (nf * sky - sk * sy) / den };
    (sy / nf - slope * sk / nf, slope)
}
fn minmax(w: &[f64]) -> (f64, f64) { w.iter().fold((f64::MAX, f64::MIN), |(lo, hi), &v| (lo.min(v), hi.max(v))) }
fn shift1(x: &[f64]) -> Vec<f64> { (0..x.len()).map(|i| if i == 0 { NA } else { x[i - 1] }).collect() }
/// Ehlers UltimateSmoother (TASC Apr 2024 EasyLanguage): the first three values of the series pass through, then
/// US = (1 − c1)·x + (2c1 − c2)·x[1] − (c1 + c3)·x[2] + c2·US[1] + c3·US[2]; leading NaN skipped.
fn ultimate_smoother(x: &[f64], period: usize) -> Vec<f64> {
    let pf = period as f64;
    let a1 = (-1.414 * std::f64::consts::PI / pf).exp();
    let c2 = 2. * a1 * (1.414 * std::f64::consts::PI / pf).cos();
    let c3 = -a1 * a1;
    let c1 = (1. + c2 - c3) / 4.;
    let mut out = vec![NA; x.len()];
    let Some(f) = x.iter().position(|v| !v.is_nan()) else { return out };
    for i in f..x.len() {
        out[i] = if i < f + 3 { x[i] } else { (1. - c1) * x[i] + (2. * c1 - c2) * x[i - 1] - (c1 + c3) * x[i - 2] + c2 * out[i - 1] + c3 * out[i - 2] };
    }
    out
}

// ---------------------------------------------------------------- Volatility
fn aptr(b: &Bars, prm: &Params) -> Output {
    let ptr: Vec<f64> = (0..b.len()).map(|i| if i == 0 { NA } else {
        let (h, l, pc) = (b.high[i], b.low[i], b.close[i - 1]);
        let tr = (h - l).max((h - pc).abs()).max((l - pc).abs());
        div(tr, (h.max(pc) + l.min(pc)) / 2.)
    }).collect();
    Ok(vec![scale(&ta::rma(&ptr, n(prm, "length")), 100.)])
}
fn gapo(b: &Bars, prm: &Params) -> Output {
    let k = n(prm, "length");
    let r = zip(&ta::highest(&b.high, k), &ta::lowest(&b.low, k), |h, l| h - l);
    Ok(vec![r.iter().map(|&r| if r > 0. { r.ln() / (k as f64).ln() } else { NA }).collect()])
}
fn hl_volatility(b: &Bars, prm: &Params) -> Output {
    let k = n(prm, "length");
    let r = zip(&ta::highest(&b.high, k), &ta::lowest(&b.low, k), |h, l| h - l);
    Ok(vec![zip(&ta::ema(&r, k), &ta::ema(&b.close, k), |a, c| if a.is_nan() || c.is_nan() { NA } else if c == 0. { 0. } else { 100. * a / c })])
}
fn rind(b: &Bars, prm: &Params) -> Output {
    let tr = ta::tr(b, false);
    let v: Vec<f64> = (0..b.len()).map(|i| if i == 0 { NA } else { let d = b.close[i] - b.close[i - 1]; if d > 0. { tr[i] / d } else { tr[i] } }).collect();
    let st = rolling(&v, n(prm, "length"), |w| { let (lo, hi) = minmax(w); let x = w[w.len() - 1]; if hi - lo > 0. { 100. * (x - lo) / (hi - lo) } else { 100. * (x - lo) } });
    Ok(vec![ta::ema(&st, n(prm, "smooth"))])
}
fn rogers_satchell(b: &Bars, prm: &Params) -> Output {
    let (k, days) = (n(prm, "length"), p(prm, "days"));
    let term: Vec<f64> = (0..b.len()).map(|i| {
        let (o, h, l, c) = (b.open[i], b.high[i], b.low[i], b.close[i]);
        if o > 0. && h > 0. && l > 0. && c > 0. { let (s1, i1, x1) = ((h / o).ln(), (l / o).ln(), (c / o).ln()); s1 * (s1 - x1) + i1 * (i1 - x1) } else { 0. }
    }).collect();
    Ok(vec![ta::sum(&term, k).iter().map(|&s| if s.is_nan() { NA } else if s > 0. { 100. * days.sqrt() * (s / k as f64).sqrt() } else { 0. }).collect()])
}
fn squeeze(b: &Bars, prm: &Params) -> Output {
    let (bl, bm, kl, km) = (n(prm, "bb_length"), p(prm, "bb_mult"), n(prm, "kc_length"), p(prm, "kc_mult"));
    let (basis, sd) = (ta::sma(&b.close, bl), ta::stdev(&b.close, bl));
    let (kcm, band) = (ta::sma(&b.close, kl), ta::sma(&ta::tr(b, false), kl));
    let (hh, ll) = (ta::highest(&b.high, kl), ta::lowest(&b.low, kl));
    let (mut on, mut off, mut level) = (vec![NA; b.len()], vec![NA; b.len()], vec![NA; b.len()]);
    let dev: Vec<f64> = (0..b.len()).map(|i| b.close[i] - ((hh[i] + ll[i]) / 2. + kcm[i]) / 2.).collect();
    for i in 0..b.len() {
        let (bbu, bbl) = (basis[i] + bm * sd[i], basis[i] - bm * sd[i]);
        if [bbu, bbl, kcm[i], band[i]].iter().any(|v| v.is_nan()) { continue; }
        let inside = |m: f64| bbl > kcm[i] - m * band[i] && bbu < kcm[i] + m * band[i];
        let outside = |m: f64| bbl < kcm[i] - m * band[i] && bbu > kcm[i] + m * band[i];
        on[i] = inside(km) as u8 as f64;
        off[i] = outside(km) as u8 as f64;
        level[i] = if inside(1.) { 3. } else if inside(1.5) { 2. } else if inside(2.) { 1. } else if outside(2.) { -1. } else { 0. };
    }
    Ok(vec![on, off, level, ta::linreg(&dev, kl, 0)])
}
fn thermo(b: &Bars, prm: &Params) -> Output {
    let t: Vec<f64> = (0..b.len()).map(|i| if i == 0 { NA } else { let (tl, th) = ((b.low[i - 1] - b.low[i]).abs(), (b.high[i] - b.high[i - 1]).abs()); if th < tl { tl } else { th } }).collect();
    let ma = ta::ema(&t, n(prm, "length"));
    let (lf, sf) = (p(prm, "long"), p(prm, "short"));
    let long = zip(&t, &ma, |t, m| if m.is_nan() { NA } else { (t < m * lf) as u8 as f64 });
    let short = zip(&t, &ma, |t, m| if m.is_nan() { NA } else { (t > m * sf) as u8 as f64 });
    Ok(vec![t, ma, long, short])
}
fn vix_fix(b: &Bars, prm: &Params) -> Output {
    let hc = ta::highest(&b.close, n(prm, "length"));
    let wvf: Vec<f64> = (0..b.len()).map(|i| div(100. * (hc[i] - b.low[i]), hc[i])).collect();
    let (bl, bm) = (n(prm, "bb_length"), p(prm, "bb_mult"));
    let upper = zip(&ta::sma(&wvf, bl), &ta::stdev(&wvf, bl), |m, s| m + bm * s);
    let rh = scale(&ta::highest(&wvf, n(prm, "lookback")), p(prm, "ph"));
    let sig = (0..b.len()).map(|i| if upper[i].is_nan() || rh[i].is_nan() { NA } else { (wvf[i] >= upper[i] || wvf[i] >= rh[i]) as u8 as f64 }).collect();
    Ok(vec![wvf, upper, rh, sig])
}
fn volatility_switch(b: &Bars, prm: &Params) -> Output {
    let k = n(prm, "length"); let c = &b.close;
    let r: Vec<f64> = (0..c.len()).map(|i| if i == 0 { NA } else { div(c[i] - c[i - 1], (c[i] + c[i - 1]) / 2.) }).collect();
    let s = ta::stdev(&r, k);
    Ok(vec![(0..c.len()).map(|i| if i + 1 < k || s[i + 1 - k..=i].iter().any(|v| v.is_nan()) { NA } else {
        s[i + 1 - k..=i].iter().filter(|&&v| v <= s[i]).count() as f64 / k as f64
    }).collect()])
}

// ---------------------------------------------------------------- Bands & channels
fn accbands(b: &Bars, prm: &Params) -> Output {
    let (k, fc) = (n(prm, "length"), p(prm, "factor"));
    let f: Vec<f64> = (0..b.len()).map(|i| div(fc * (b.high[i] - b.low[i]), b.high[i] + b.low[i])).collect();
    let up: Vec<f64> = (0..b.len()).map(|i| b.high[i] * (1. + f[i])).collect();
    let dn: Vec<f64> = (0..b.len()).map(|i| b.low[i] * (1. - f[i])).collect();
    Ok(vec![ta::sma(&up, k), ta::sma(&b.close, k), ta::sma(&dn, k)])
}
fn esd_bands(b: &Bars, prm: &Params) -> Output {
    let (k, m) = (n(prm, "length"), p(prm, "mult"));
    let mid = ta::ema(&b.close, k);
    let dev: Vec<f64> = if p(prm, "mode") == 0. {
        (0..b.len()).map(|i| if i + 1 < k || mid[i].is_nan() { NA } else { (b.close[i + 1 - k..=i].iter().map(|c| snap(c - mid[i]).powi(2)).sum::<f64>() / k as f64).sqrt() }).collect()
    } else { ta::ema(&zip(&mid, &b.close, |a, c| (a - c).abs()), k) };
    Ok(vec![mid.clone(), zip(&mid, &dev, |a, d| a + m * d), zip(&mid, &dev, |a, d| a - m * d)])
}
fn fib_bb(b: &Bars, prm: &Params) -> Output {
    let v = b.volume.as_ref().ok_or("needs volume")?;
    let (k, m) = (n(prm, "length"), p(prm, "mult"));
    let src = b.hlc3();
    let basis = zip(&ta::sma(&zip(&src, v, |s, v| s * v), k), &ta::sma(v, k), div);
    let dev = scale(&ta::stdev(&src, k), m);
    let mut out = vec![basis.clone()];
    for sign in [1., -1.] { for f in [0.236, 0.382, 0.5, 0.618, 0.764, 1.] { out.push(zip(&basis, &dev, |a, d| a + sign * f * d)); } }
    Ok(out)
}
fn gann_sq9(b: &Bars, prm: &Params) -> Output {
    let st = p(prm, "degrees") / 180.;
    let anchor = shift1(&b.close);
    Ok([-4., -3., -2., -1., 1., 2., 3., 4.].iter().map(|k| anchor.iter().map(|&a| if a > 0. { (a.sqrt() + k * st).powi(2) } else { NA }).collect()).collect())
}
fn greatest_swing(b: &Bars, prm: &Params) -> Output {
    let (k, m) = (n(prm, "length"), p(prm, "mult"));
    let bs: Vec<f64> = (0..b.len()).map(|i| if b.close[i] < b.open[i] { b.high[i] - b.open[i] } else { 0. }).collect();
    let ss: Vec<f64> = (0..b.len()).map(|i| if b.close[i] > b.open[i] { b.open[i] - b.low[i] } else { 0. }).collect();
    // Sierra Chart "Simple Moving Average - Skip Zeros": mean of the non-zero values in the last n bars, 0 when none
    let szma = |x: &[f64]| rolling(x, k, |w| { let nz = w.iter().filter(|v| **v != 0.).count(); if nz == 0 { 0. } else { w.iter().sum::<f64>() / nz as f64 } });
    let (avg_bs, avg_ss) = (shift1(&szma(&bs)), shift1(&szma(&ss)));
    Ok(vec![zip(&b.open, &avg_bs, |o, a| o + m * a), zip(&b.open, &avg_ss, |o, a| o - m * a)])
}
fn mavg_bands(b: &Bars, prm: &Params) -> Output {
    let (fast, slow, m) = (n(prm, "fast"), n(prm, "slow"), p(prm, "mult"));
    let (ma1, ma2) = (ta::ema(&b.close, slow), ta::ema(&b.close, fast));
    let dst2 = zip(&ma1, &ma2, |a, c| (a - c) * (a - c));
    let dev: Vec<f64> = ta::sma(&dst2, fast).iter().map(|v| v.sqrt() * m).collect();
    let (up, lo) = (zip(&ma1, &dev, |a, d| a + d), zip(&ma1, &dev, |a, d| a - d));
    let width = (0..b.len()).map(|i| div(100. * (up[i] - lo[i]), ma1[i])).collect();
    Ok(vec![ma1, up, lo, width])
}
/// MT5 MurreyMath_Channel.mq5 DetermineFractal.
fn murrey_fractal(v: f64) -> f64 {
    match v {
        v if v <= 250000. && v > 25000. => 100000.,
        v if v <= 25000. && v > 2500. => 10000.,
        v if v <= 2500. && v > 250. => 1000.,
        v if v <= 250. && v > 25. => 100.,
        v if v <= 25. && v > 6.25 => 12.5,
        v if v <= 6.25 && v > 3.125 => 6.25,
        v if v <= 3.125 && v > 1.5625 => 3.125,
        v if v <= 1.5625 && v > 0.390625 => 1.5625,
        v if v <= 0.390625 && v > 0. => 0.1953125,
        _ => 0.,
    }
}
fn murrey(b: &Bars, prm: &Params) -> Output {
    let per = n(prm, "length");
    let mut out = vec![vec![NA; b.len()]; 13];
    for i in per..b.len() {
        let (min, _) = minmax(&b.low[i - per..i]);
        let (_, max) = minmax(&b.high[i - per..i]);
        let fractal = murrey_fractal(max);
        let range = max - min;
        if fractal == 0. || !(range > 0.) { continue; }
        let octave = fractal * 0.5f64.powf(((fractal / range).ln() / 2f64.ln()).floor());
        let mn = (min / octave).floor() * octave;
        let mx = if mn + octave >= max { mn + octave } else { mn + 2. * octave };
        let d = mx - mn;
        let (mut x1, mut x2, mut x3, mut x4, mut x5, mut x6) = (0., 0., 0., 0., 0., 0.);
        if min >= 3. * d / 16. + mn && max <= 9. * d / 16. + mn { x2 = mn + d / 2.; }
        if min >= mn - d / 8. && max <= 5. * d / 8. + mn && x2 == 0. { x1 = mn + d / 2.; }
        if min >= mn + 7. * d / 16. && max <= 13. * d / 16. + mn { x4 = mn + 3. * d / 4.; }
        if min >= mn + 3. * d / 8. && max <= 9. * d / 8. + mn && x4 == 0. { x5 = mx; }
        if min >= mn + d / 8. && max <= 7. * d / 8. + mn && x1 == 0. && x2 == 0. && x4 == 0. && x5 == 0. { x3 = mn + 3. * d / 4.; }
        if x1 + x2 + x3 + x4 + x5 == 0. { x6 = mx; }
        let res = x1 + x2 + x3 + x4 + x5 + x6;
        let (y1, y2, y3, y4, y5) = (if x1 > 0. { mn } else { 0. }, if x2 > 0. { mn + d / 4. } else { 0. }, if x3 > 0. { mn + d / 4. } else { 0. },
                                    if x4 > 0. { mn + d / 2. } else { 0. }, if x5 > 0. { mn + d / 2. } else { 0. });
        let y6 = if res > 0. && y1 + y2 + y3 + y4 + y5 == 0. { mn } else { 0. };
        let sup = y1 + y2 + y3 + y4 + y5 + y6;
        let step = (res - sup) / 8.;
        for (k, s) in out.iter_mut().enumerate() { s[i] = sup + (k as f64 - 2.) * step; }
    }
    Ok(out)
}
fn nw_envelope(b: &Bars, prm: &Params) -> Output {
    let (h, m, w) = (p(prm, "bandwidth"), p(prm, "mult"), n(prm, "window"));
    let g: Vec<f64> = (0..w).map(|i| (-((i * i) as f64) / (h * h * 2.)).exp()).collect();
    let den: f64 = g.iter().sum();
    let line: Vec<f64> = (0..b.len()).map(|t| if t + 1 < w { NA } else { (0..w).map(|i| b.close[t - i] * g[i]).sum::<f64>() / den }).collect();
    let mae = scale(&ta::sma(&zip(&b.close, &line, |c, f| (c - f).abs()), w - 1), m);
    Ok(vec![line.clone(), zip(&line, &mae, |f, e| f + e), zip(&line, &mae, |f, e| f - e)])
}
fn projection_bands(b: &Bars, prm: &Params) -> Output {
    let k = n(prm, "length");
    let band = |x: &[f64], upper: bool| -> Vec<f64> {
        (0..x.len()).map(|t| if t + 1 < k { NA } else {
            let s = fit(&x[t + 1 - k..=t]).1;
            let it = (0..k).map(|i| x[t - i] + i as f64 * s);
            if upper { it.fold(f64::MIN, f64::max) } else { it.fold(f64::MAX, f64::min) }
        }).collect()
    };
    Ok(vec![band(&b.high, true), band(&b.low, false)])
}
/// Least-squares polynomial of degree `deg` through w (oldest → newest), x scaled to [−1, 0] (0 = newest);
/// returns the fitted value at the newest bar and the residuals.
fn polyfit_end(w: &[f64], deg: usize) -> Option<(f64, Vec<f64>)> {
    let nw = w.len(); let m = deg + 1;
    let xs: Vec<f64> = (0..nw).map(|j| (j as f64 - (nw - 1) as f64) / (nw - 1) as f64).collect();
    let mut a = vec![vec![0.; m + 1]; m];
    for (j, &x) in xs.iter().enumerate() {
        let pw: Vec<f64> = (0..2 * m).map(|e| x.powi(e as i32)).collect();
        for r in 0..m { for c in 0..m { a[r][c] += pw[r + c]; } a[r][m] += pw[r] * w[j]; }
    }
    for col in 0..m {
        let piv = (col..m).max_by(|&i, &j| a[i][col].abs().total_cmp(&a[j][col].abs()))?;
        a.swap(col, piv);
        if a[col][col].abs() < 1e-300 { return None; }
        for r in 0..m { if r != col { let f = a[r][col] / a[col][col]; for c in col..=m { a[r][c] -= f * a[col][c]; } } }
    }
    let coef: Vec<f64> = (0..m).map(|r| a[r][m] / a[r][r]).collect();
    let res = xs.iter().zip(w).map(|(&x, &y)| snap(y - coef.iter().enumerate().map(|(e, c)| c * x.powi(e as i32)).sum::<f64>())).collect();
    Some((coef[0], res))
}
fn reg_channel(b: &Bars, prm: &Params) -> Output {
    let (k, deg, m, raff) = (n(prm, "length"), n(prm, "degree"), p(prm, "mult"), p(prm, "dev_type") == 1.);
    let (mut mid, mut up, mut lo) = (vec![NA; b.len()], vec![NA; b.len()], vec![NA; b.len()]);
    for t in k.saturating_sub(1)..b.len() {
        let w = &b.close[t + 1 - k..=t];
        if w.iter().any(|v| v.is_nan()) { continue; }
        let Some((f, res)) = polyfit_end(w, deg) else { continue };
        let d = if raff { res.iter().fold(0f64, |a, r| a.max(r.abs())) } else { (res.iter().map(|r| r * r).sum::<f64>() / k as f64).sqrt() };
        mid[t] = f; up[t] = f + m * d; lo[t] = f - m * d;
    }
    Ok(vec![mid, up, lo])
}
fn ultimate_bands(b: &Bars, prm: &Params) -> Output {
    let (k, m) = (n(prm, "length"), p(prm, "mult"));
    let mid = ultimate_smoother(&b.close, k);
    let dev: Vec<f64> = if p(prm, "mode") == 0. {
        let mut sd = 0.; // Ehlers: SD keeps its last value when the sum is 0
        (0..b.len()).map(|i| if i + 1 < k { NA } else {
            let s: f64 = (i + 1 - k..=i).map(|j| (b.close[j] - mid[j]).powi(2)).sum();
            if s != 0. { sd = (s / k as f64).sqrt(); }
            sd
        }).collect()
    } else {
        let r: Vec<f64> = (0..b.len()).map(|i| if i == 0 { NA } else { let pc = b.close[i - 1]; b.high[i].max(pc) - b.low[i].min(pc) }).collect();
        ultimate_smoother(&r, k)
    };
    Ok(vec![mid.clone(), zip(&mid, &dev, |a, d| a + m * d), zip(&mid, &dev, |a, d| a - m * d)])
}

// ---------------------------------------------------------------- Statistics
fn decay(b: &Bars, prm: &Params) -> Output {
    let (k, expo) = (n(prm, "length") as f64, p(prm, "mode") == 1.);
    let mut out = vec![NA; b.len()];
    for i in 0..b.len() {
        let x = b.close[i];
        out[i] = if i == 0 { x } else { let d = if expo { out[i - 1] * (1. - 1. / k) } else { out[i - 1] - 1. / k }; if x > d { x } else { d } };
    }
    Ok(vec![out])
}
fn entropy(b: &Bars, prm: &Params) -> Output {
    let lb = p(prm, "base").ln();
    Ok(vec![rolling(&b.close, n(prm, "length"), |w| {
        let s: f64 = w.iter().sum();
        if s == 0. { return NA; }
        w.iter().map(|v| v / s).filter(|&p| p > 0.).map(|p| -p * p.ln() / lb).sum()
    })])
}
/// Sierra Chart R/S: mean rescaled range over the n/N consecutive blocks of size N (blocks tile the newest floor(n/N)·N values).
fn rescaled_range(w: &[f64], size: usize) -> Option<f64> {
    let mut acc = 0.;
    let w = &w[w.len() % size..];
    for blk in w.chunks(size) {
        let m = blk.iter().sum::<f64>() / size as f64;
        let (mut z, mut zmin, mut zmax, mut ss) = (0., f64::MAX, f64::MIN, 0.);
        for v in blk { let y = v - m; z += y; zmin = zmin.min(z); zmax = zmax.max(z); ss += y * y; }
        let s = (ss / size as f64).sqrt();
        if !(s > 0.) { return None; }
        acc += (zmax - zmin) / s;
    }
    Some(acc / (w.len() / size) as f64)
}
fn ls_slope(xs: &[f64], ys: &[f64]) -> f64 {
    let nf = xs.len() as f64;
    let (sx, sy) = (xs.iter().sum::<f64>(), ys.iter().sum::<f64>());
    let sxy: f64 = xs.iter().zip(ys).map(|(x, y)| x * y).sum();
    let sxx: f64 = xs.iter().map(|x| x * x).sum();
    (nf * sxy - sx * sy) / (nf * sxx - sx * sx)
}
fn hurst(b: &Bars, prm: &Params) -> Output {
    let k = n(prm, "length");
    let c = &b.close;
    if p(prm, "variant") == 1. {
        // TA4J HurstExponentIndicator: variogram of close over lags 1..min(10, n − 1), floor 1e-12, 0.5·slope clamped to [0, 1]
        let maxlag = 10usize.min(k - 1);
        return Ok(vec![rolling(c, k, |w| {
            let (mut xs, mut ys) = (vec![], vec![]);
            for lag in 1..=maxlag {
                let v = (lag..w.len()).map(|j| (w[j] - w[j - lag]).powi(2)).sum::<f64>() / (w.len() - lag) as f64;
                xs.push((lag as f64).ln()); ys.push(v.max(1e-12).ln());
            }
            let s = ls_slope(&xs, &ys);
            if s.is_finite() { (0.5 * s).clamp(0., 1.) } else { NA }
        })]);
    }
    let r: Vec<f64> = (0..c.len()).map(|i| if i == 0 || !(c[i] > 0. && c[i - 1] > 0.) { NA } else { (c[i] / c[i - 1]).ln() }).collect();
    let sizes: Vec<usize> = (3..).map(|e| 1usize << e).take_while(|&s| s <= k).collect();
    let lnn: Vec<f64> = sizes.iter().map(|&s| (s as f64).ln()).collect();
    Ok(vec![rolling(&r, k, |w| {
        let mut ys = vec![];
        for &s in &sizes { match rescaled_range(w, s) { Some(e) => ys.push(e.ln()), None => return NA } }
        ls_slope(&lnn, &ys)
    })])
}
fn leavitt_conv(b: &Bars, prm: &Params) -> Output {
    let k = n(prm, "length");
    let lc = (k as f64).sqrt() as usize;
    let fwd = |w: &[f64]| { let (a, s) = fit(w); a + s * (w.len() + 1) as f64 };
    let proj = rolling(&b.close, k, fwd);
    let slope = rolling(&proj, lc, |w| fit(w).1);
    let accel = (0..b.len()).map(|i| if i == 0 { NA } else { slope[i] - slope[i - 1] }).collect();
    Ok(vec![rolling(&proj, lc, fwd), slope, accel])
}
fn linreg_angle(b: &Bars, prm: &Params) -> Output { Ok(vec![rolling(&b.close, n(prm, "length"), |w| fit(w).1.atan().to_degrees())]) }
fn mean_dev(b: &Bars, prm: &Params) -> Output { Ok(vec![ta::dev(&b.close, n(prm, "length"))]) }
fn mode(b: &Bars, prm: &Params) -> Output {
    Ok(vec![rolling(&b.close, n(prm, "length"), |w| {
        let mut s = w.to_vec(); s.sort_by(|a, b| a.total_cmp(b));
        let (mut best, mut cnt, mut j) = (s[0], 0, 0);
        while j < s.len() {
            let mut e = j; while e + 1 < s.len() && s[e + 1] == s[j] { e += 1; }
            if e - j + 1 > cnt { cnt = e - j + 1; best = s[j]; } // strict >: ties keep the smallest value
            j = e + 1;
        }
        best
    })])
}

// ---------------------------------------------------------------- Price functions
fn cdl_z(b: &Bars, prm: &Params) -> Output {
    let k = n(prm, "length");
    let z = |x: &[f64]| rolling(x, k, |w| {
        let m = w.iter().sum::<f64>() / k as f64;
        let sd = (w.iter().map(|v| snap(v - m).powi(2)).sum::<f64>() / (k as f64 - 1.)).sqrt();
        div(snap(w[k - 1] - m), sd)
    });
    Ok(vec![z(&b.open), z(&b.high), z(&b.low), z(&b.close)])
}
fn gapless(b: &Bars, _: &Params) -> Output {
    let mut g = vec![0.; b.len()];
    for i in 1..b.len() { g[i] = g[i - 1] + b.open[i] - b.close[i - 1]; }
    Ok([&b.open, &b.high, &b.low, &b.close].iter().map(|x| zip(x, &g, |v, g| v - g)).collect())
}
fn heikin_ashi(b: &Bars, prm: &Params) -> Output {
    let (pre, post) = (n(prm, "pre_smooth"), n(prm, "post_smooth"));
    let (o, h, l, c) = (ta::rma(&b.open, pre), ta::rma(&b.high, pre), ta::rma(&b.low, pre), ta::rma(&b.close, pre));
    let (mut ho, mut hh, mut hl, mut hc) = (vec![NA; b.len()], vec![NA; b.len()], vec![NA; b.len()], vec![NA; b.len()]);
    for i in 0..b.len() {
        if [o[i], h[i], l[i], c[i]].iter().any(|v| v.is_nan()) { continue; }
        hc[i] = (o[i] + h[i] + l[i] + c[i]) / 4.;
        ho[i] = if i == 0 || ho[i - 1].is_nan() { (o[i] + c[i]) / 2. } else { (ho[i - 1] + hc[i - 1]) / 2. };
        hh[i] = h[i].max(ho[i]).max(hc[i]);
        hl[i] = l[i].min(ho[i]).min(hc[i]);
    }
    Ok([ho, hh, hl, hc].iter().map(|x| ta::wma(x, post)).collect())
}
fn linreg_candles(b: &Bars, prm: &Params) -> Output {
    let k = n(prm, "length");
    let mut lr: Vec<Vec<f64>> = [&b.open, &b.high, &b.low, &b.close].iter().map(|x| ta::linreg(x, k, 0)).collect();
    let sig = ta::sma(&lr[3], n(prm, "signal"));
    lr.push(sig);
    Ok(lr)
}

const APTR: &[ParamSpec] = &[len("length", "Length", 14.)];
const GAPO: &[ParamSpec] = &[int("length", "Length", 14., 2., 2000.)];
const HLV: &[ParamSpec] = &[len("length", "Length", 9.)];
const RIND: &[ParamSpec] = &[len("length", "Stochastic length", 10.), len("smooth", "EMA smoothing", 3.)];
const RS: &[ParamSpec] = &[len("length", "Length", 10.), int("days", "Annualisation days", 252., 1., 400.)];
const SQZ: &[ParamSpec] = &[len("bb_length", "BB length", 20.), real("bb_mult", "BB multiplier", 2., 0.1, 5., 0.1), len("kc_length", "KC length", 20.), real("kc_mult", "KC multiplier", 1.5, 0.1, 5., 0.1)];
const THERMO: &[ParamSpec] = &[len("length", "Length", 20.), real("long", "Long (buy) factor", 2., 0.1, 10., 0.1), real("short", "Short (sell) factor", 0.5, 0.01, 10., 0.01)];
const VIXF: &[ParamSpec] = &[len("length", "Length", 22.), len("bb_length", "Band length", 20.), real("bb_mult", "Band multiplier", 2., 0.1, 5., 0.1),
    len("lookback", "Range lookback", 50.), real("ph", "Range high factor", 0.85, 0.1, 2., 0.01)];
const VS: &[ParamSpec] = &[len("length", "Length", 21.)];
const ACC: &[ParamSpec] = &[len("length", "Length", 20.), real("factor", "Factor", 4., 0., 20., 0.1)];
const ESD: &[ParamSpec] = &[len("length", "Length", 20.), real("mult", "Multiplier", 2., 0.1, 10., 0.1), int("mode", "Mode (0 ESD bands 2017, 1 exponential deviation bands 2019)", 0., 0., 1.)];
const FIB: &[ParamSpec] = &[len("length", "Length", 200.), real("mult", "Multiplier", 3., 0.1, 10., 0.1)];
const SQ9: &[ParamSpec] = &[real("degrees", "Degrees", 22.5, 1., 360., 0.5)];
const GSV: &[ParamSpec] = &[len("length", "Length", 4.), real("mult", "Multiplier", 1.8, 0.1, 10., 0.1)];
const MAB: &[ParamSpec] = &[len("fast", "Fast", 10.), len("slow", "Slow", 50.), real("mult", "Multiplier", 1., 0.1, 10., 0.1)];
const MUR: &[ParamSpec] = &[len("length", "Period", 64.)];
const NW: &[ParamSpec] = &[real("bandwidth", "Bandwidth", 8., 0.5, 100., 0.5), real("mult", "Multiplier", 3., 0., 10., 0.1), int("window", "Window", 500., 2., 2000.)];
const PB: &[ParamSpec] = &[len("length", "Length", 14.)];
const RC: &[ParamSpec] = &[int("length", "Length", 100., 3., 2000.), int("degree", "Polynomial degree", 1., 1., 4.), real("mult", "Deviations", 2., 0.1, 10., 0.1),
    int("dev_type", "Deviation (0 stdev of residuals, 1 max |residual| - Raff)", 0., 0., 1.)];
const UB: &[ParamSpec] = &[int("length", "Length", 20., 2., 2000.), real("mult", "Multiplier", 1., 0.1, 10., 0.1), int("mode", "Mode (0 stdev bands, 1 true-range channel)", 0., 0., 1.)];
const DEC: &[ParamSpec] = &[len("length", "Length", 5.), int("mode", "Mode (0 linear, 1 exponential)", 0., 0., 1.)];
const ENT: &[ParamSpec] = &[int("length", "Length", 10., 2., 2000.), real("base", "Log base", 2., 1.01, 100., 0.01)];
const HUR: &[ParamSpec] = &[int("length", "Window", 128., 16., 2000.), int("variant", "Variant (0 R/S Sierra Chart, 1 variogram TA4J)", 0., 0., 1.)];
const LEA: &[ParamSpec] = &[int("length", "Length", 61., 4., 2000.)];
const L14: &[ParamSpec] = &[int("length", "Length", 14., 2., 2000.)];
const MODE: &[ParamSpec] = &[int("length", "Length", 20., 2., 2000.)];
const CZ: &[ParamSpec] = &[int("length", "Length", 30., 2., 2000.)];
const HA: &[ParamSpec] = &[len("pre_smooth", "Pre-smoothing length (1 = none)", 1.), len("post_smooth", "Post-smoothing length (1 = none)", 1.)];
const LRC: &[ParamSpec] = &[int("length", "Length", 11., 2., 2000.), len("signal", "Signal SMA length", 11.)];
const OHLC: &[&str] = &["open", "high", "low", "close"];

pub const DEFS: &[IndicatorDef] = &[
    def("aptr", "Average Percentage True Range (Apirine)", VOL, APTR, &["aptr"], PN, NO, "ptr = tr/((max(high, close[1]) + min(low, close[1]))/2); aptr = 100·rma(ptr, n) (Apirine TASC Nov 2015; eSignal Traders' Tips code: Wilder smma ×100, SMA seed)", &[14], aptr),
    def("gapo", "Gopalakrishnan Range Index", VOL, GAPO, &["gapo"], PN, NO, "ln(highest(high, n) − lowest(low, n))/ln(n) (ChartIQ GAPO); range ≤ 0 → NaN", &[13], gapo),
    def("hl_volatility", "High-Low Volatility", VOL, HLV, &["hlv"], PN, NO, "100·ema(highest(high, n) − lowest(low, n), n)/ema(close, n); ema(close) = 0 → 0 (Sierra Chart HL Volatility, EMA type; our EMA seeds with the SMA)", &[16], hl_volatility),
    def("rind", "Range Indicator (Weinberg)", VOL, RIND, &["rind"], PN, NO, "v = close > close[1] ? tr/(close − close[1]) : tr; st = 100·(v − lowest(v, length))/(highest(v, length) − lowest(v, length)), range 0 → 100·(v − lowest) = 0; rind = ema(st, smooth) (Weinberg TASC Jun 1995; AmiBroker TRI code by M. Chlopek, defaults 10/3)", &[12], rind),
    def("rogers_satchell", "Rogers-Satchell Volatility", VOL, RS, &["rs"], PN, NO, "term = ln(H/O)·(ln(H/O) − ln(C/O)) + ln(L/O)·(ln(L/O) − ln(C/O)) (non-positive price → 0); S = sum(term, n); rs = 100·√days·√(S/n), S ≤ 0 → 0 (TA-Lib ROGERSSATCHELL ×100 to percent, like volatility_ohlc)", &[9], rogers_satchell),
    def("squeeze", "Squeeze Momentum (TA-Lib SQZMOM)", VOL, SQZ, &["sqz_on", "sqz_off", "level", "momentum"], PN, NO, "TA-Lib SQZMOM: BB = sma(close, bb_length) ± bb_mult·stdev (population); KCM = sma(close, kc_length), BAND = sma(tr, kc_length) (no bar-0 TR); inside(m) = BBL > KCM − m·BAND and BBU < KCM + m·BAND; sqz_on = inside(kc_mult), sqz_off = BBL < KCM − kc_mult·BAND and BBU > KCM + kc_mult·BAND (pandas-ta, 1/0); level = 3/2/1 when inside at m = 1/1.5/2, −1 when both bands are outside m = 2, else 0; momentum = linreg(close − ((highest(high, k) + lowest(low, k))/2 + KCM)/2, k, 0)", &[20, 20, 20, 38], squeeze),
    def("thermo", "Elder's Thermometer", VOL, THERMO, &["thermo", "ma", "long", "short"], PN, NO, "thermo = max(|high − high[1]|, |low[1] − low|); ma = ema(thermo, n); long = thermo < long·ma (1/0); short = thermo > short·ma (1/0) (pandas-ta thermo / MotiveWave; neither source zeroes inside bars)", &[1, 20, 20, 20], thermo),
    def("vix_fix", "Williams VIX Fix", VOL, VIXF, &["wvf", "upper", "range_high", "signal"], PN, NO, "wvf = 100·(highest(close, length) − low)/highest(close, length) (Sierra Chart Synthetic VIX); upper = sma(wvf, bb_length) + bb_mult·stdev(wvf, bb_length); range_high = highest(wvf, lookback)·ph; signal = wvf ≥ upper or wvf ≥ range_high (1/0) (ChrisMoody CM_Williams_Vix_Fix; ta.highest window semantics)", &[21, 40, 49, 49], vix_fix),
    def("volatility_switch", "Volatility Switch (McEwan)", VOL, VS, &["vs"], PN, NO, "r = (close − close[1])/((close + close[1])/2); s = stdev(r, n); vs = (count of the last n s values ≤ current s)/n, 0..1, level 0.5 (McEwan TASC Feb 2013, TradeStation/AmiBroker tips; the count is the same for sample or population stdev)", &[41], volatility_switch),
    def("accbands", "Acceleration Bands (Headley)", BC, ACC, &["upper", "middle", "lower"], P, NO, "f = factor·(high − low)/(high + low); upper = sma(high·(1 + f), n); middle = sma(close, n); lower = sma(low·(1 − f), n) (TA-Lib ACCBANDS, factor 4)", &[19, 19, 19], accbands),
    def("esd_bands", "Exponential Deviation Bands (Apirine)", BC, ESD, &["middle", "upper", "lower"], P, NO, "middle = ema(close, n); mode 0 (ESD bands, TASC Feb 2017 TradeStation code): dev = √(Σ_{i<n}(close[i] − middle)²/n) around the current EMA; mode 1 (exponential deviation bands, TASC Jul 2019): dev = ema(|middle − close|, n) (our EMA seeds with the SMA; TradeStation seeds with 0); upper/lower = middle ± mult·dev", &[19, 19, 19], esd_bands),
    def("fib_bb", "Fibonacci Bollinger Bands", BC, FIB, &["basis", "u236", "u382", "u500", "u618", "u764", "u1", "l236", "l382", "l500", "l618", "l764", "l1"], P, VO, "src = hlc3; basis = vwma(src, n); dev = mult·stdev(src, n) (population); bands = basis ± {0.236, 0.382, 0.5, 0.618, 0.764, 1}·dev (Rashad, TradingView public script)", &[199; 13], fib_bb),
    def("gann_sq9", "Gann Square of 9 Levels", BC, SQ9, &["l_m4", "l_m3", "l_m2", "l_m1", "l_p1", "l_p2", "l_p3", "l_p4"], P, NO, "anchor = close[1] (Sierra Chart auto-anchor is the last close; yesterday keeps it causal); level_k = (√anchor + k·degrees/180)², k = −4..−1, 1..4 (Sierra Chart Square of 9, Auto Adjust = No)", &[1; 8], gann_sq9),
    def("greatest_swing", "Greatest Swing Value (Williams)", BC, GSV, &["buy_level", "sell_level"], P, NO, "bs = close < open ? high − open : 0; ss = close > open ? open − low : 0; avg = Simple MA Skip Zeros over n bars (mean of the non-zero values, 0 when none); buy_level = open + mult·avg_bs[1]; sell_level = open − mult·avg_ss[1] (Sierra Chart ID 437)", &[4, 4], greatest_swing),
    def("mavg_bands", "Moving Average Bands (Apirine)", BC, MAB, &["middle", "upper", "lower", "width"], P, NO, "middle = ema(close, slow); dst = middle − ema(close, fast); dev = mult·√(sum(dst², fast)/fast); upper/lower = middle ± dev; width = 100·(upper − lower)/middle (Apirine TASC Jul/Aug 2021 TradeStation code: the deviation window is the fast length)", &[49, 58, 58, 58], mavg_bands),
    def("murrey", "Murrey Math Lines", BC, MUR, &["mm_m2", "mm_m1", "mm_0", "mm_1", "mm_2", "mm_3", "mm_4", "mm_5", "mm_6", "mm_7", "mm_8", "mm_p1", "mm_p2"], P, NO, "MT5 MurreyMath_Channel.mq5: min/max of low/high over the previous period bars (current bar excluded); fractal table on max (> 250000 or ≤ 0 → NaN); octave = fractal·0.5^floor(log2(fractal/(max − min))); mn = floor(min/octave)·octave; mx = mn + 2·octave (mn + octave if that covers max); resistance/support from the x1..x6/y1..y6 rules; lines = support + k·(resistance − support)/8, k = −2..10 (first value at bar period; MT5 starts at period + 1 and blanks repeated segments for display)", &[64; 13], murrey),
    def("nw_envelope", "Nadaraya-Watson Envelope (end-point)", BC, NW, &["fit", "upper", "lower"], P, NO, "non-repainting mode of LuxAlgo's script, implemented from the formula: g(i) = exp(−i²/(2h²)), i = 0..window−1 (0 = current bar); fit = Σ close[i]·g(i)/Σ g(i); mae = mult·sma(|close − fit|, window − 1); upper/lower = fit ± mae (the default repainting mode looks ahead and is excluded)", &[499, 997, 997], nw_envelope),
    def("projection_bands", "Projection Bands (Widner)", BC, PB, &["upper", "lower"], P, NO, "sh/sl = least-squares slopes of high/low over n; upper = max_{i<n}(high[i] + i·sh); lower = min_{i<n}(low[i] + i·sl), i bars ago projected to the current bar (Widner TASC Jul 1995; thinkorswim ProjectionBands)", &[13, 13], projection_bands),
    def("reg_channel", "Regression Channel (end-point, causal)", BC, RC, &["fit", "upper", "lower"], P, NO, "least-squares polynomial of the given degree through the last n closes, evaluated at the newest bar (degree 1 = lsma); upper/lower = fit ± mult·d with d = √(Σresidual²/n) (dev_type 0) or max|residual| (dev_type 1, Raff); causal end-point replacement for repainting regression channels", &[99, 99, 99], reg_channel),
    def("ultimate_bands", "Ehlers Ultimate Bands / Channel", BC, UB, &["middle", "upper", "lower"], P, NO, "middle = UltimateSmoother(close, n) (Ehlers TASC Apr 2024: first three bars pass through); mode 0 Ultimate Bands: sd = √(Σ_{i<n}(close[i] − middle[i])²/n), kept from the previous bar when the sum is 0; mode 1 Ultimate Channel: sd = UltimateSmoother(max(high, close[1]) − min(low, close[1]), n) (one length for both smoothers); upper/lower = middle ± mult·sd (Ehlers TASC May 2024 TradeStation code)", &[0, 19, 19], ultimate_bands),
    def("decay", "Linear / Exponential Decay", ST, DEC, &["decay"], P, NO, "of close: out[0] = close[0]; linear out = max(close, out[1] − 1/n); exponential out = max(close, out[1]·(1 − 1/n)) (Tulip decay.c / edecay.c)", &[0], decay),
    def("entropy", "Shannon Entropy", ST, ENT, &["entropy"], PN, NO, "p_i = close_i/sum(close, n) over the window; entropy = −Σ p_i·ln(p_i)/ln(base), non-positive p skipped, zero sum → NaN (pandas-ta entropy)", &[9], entropy),
    def("hurst", "Hurst Exponent", ST, HUR, &["hurst"], PN, NO, "variant 0 (Sierra Chart rescaled range; Sierra Chart allows length 32/64/128 — other lengths tile the newest floor(n/N)·N returns, our extension): r = ln(close/close[1]) over the window; for N = 8, 16, 32, … ≤ n the mean over the consecutive blocks of size N of R/S (R = range of the cumulative mean-adjusted sum, S = population stdev; S = 0 → NaN); hurst = least-squares slope of ln(R/S) on ln(N) (we feed log returns as Sierra Chart's Input Data); variant 1 (TA4J HurstExponentIndicator): 0.5·slope of ln(mean squared lag-difference of close, floor 1e-12) on ln(lag), lags 1..min(10, n − 1), clamped to [0, 1]", &[128], hurst),
    def("leavitt_conv", "Leavitt Convolution (value, slope, acceleration)", ST, LEA, &["conv", "slope", "accel"], P, NO, "proj = LinearRegValue(close, length, −1) (one bar ahead); lc = int(√length); conv = LinearRegValue(proj, lc, −1); slope = LinearRegSlope(proj, lc); accel = slope − slope[1] (Leavitt, TASC Jan 2020 TradeStation functions, default length 61; accel per thinkorswim LeavittConvAcceleration)", &[66, 66, 67], leavitt_conv),
    def("linreg_angle", "Linear Regression Angle", ST, L14, &["angle"], PN, NO, "atan(slope)·180/π, slope = least-squares slope of close per bar (TA-Lib LINEARREG_ANGLE); depends on price units", &[13], linreg_angle),
    def("mean_dev", "Mean Absolute Deviation", ST, L14, &["dev"], PN, NO, "Σ_{i<n}|close[i] − sma(close, n)|/n around the current window mean (TradingView ta.dev, TA-Lib AVGDEV, Tulip md)", &[13], mean_dev),
    def("mode", "Mode (most frequent value)", ST, MODE, &["mode"], P, NO, "most frequent close over n bars (exact equality); ties or no repeats → the smallest value (TradingView ta.mode)", &[19], mode),
    def("cdl_z", "Z-Score Candles", PF, CZ, OHLC, PN, NO, "each of open, high, low, close → (x − sma(x, n))/stdev(x, n) with the sample stdev (ddof 1, pandas-ta cdl_z default; our zscore uses population); zero stdev → NaN", &[29, 29, 29, 29], cdl_z),
    def("gapless", "Gapless Bars", PF, &[], OHLC, P, NO, "g = Σ_{k=1..i}(open[k] − close[k−1]); each of open, high, low, close minus g (bar 0 unchanged). Synthetic bars for display or indicator input; fills stay on real prices (TradeStation / Sierra Chart gapless)", &[0, 0, 0, 0], gapless),
    def("heikin_ashi", "Heikin-Ashi", PF, HA, OHLC, P, NO, "inputs first smoothed by rma(pre_smooth) (Sierra Chart Heikin-Ashi Smoothed SMMA; 1 = none); ha_close = (O + H + L + C)/4; ha_open = (ha_open[1] + ha_close[1])/2, seeded (O + C)/2 on the first bar (TA-Lib / pandas-ta; MT5 and Sierra Chart seed with the open); ha_high = max(H, ha_open, ha_close); ha_low = min(L, ha_open, ha_close); outputs wma(post_smooth). Synthetic bars: fills stay on real prices", &[0, 0, 0, 0], heikin_ashi),
    def("linreg_candles", "Linear Regression Candles", PF, LRC, &["open", "high", "low", "close", "signal"], P, NO, "open/high/low/close = linreg(field, length, 0) (lsma of each field); signal = sma(linreg close, signal) (ugurvu 'Humble LinReg Candles', TradingView public script)", &[10, 10, 10, 10, 20], linreg_candles),
];
/// (id, platform names, AmiBroker names): one row per id in DEFS.
pub const SOURCES: &[(&str, &str, &str)] = &[
    ("aptr", "thinkorswim: APTR", ""),
    ("gapo", "Zerodha Kite (ChartIQ): Gopalakrishnan Range Index", ""),
    ("hl_volatility", "Sierra Chart: HL Volatility; thinkorswim: HLVolatility", ""),
    ("rind", "NinjaTrader 8: Range Indicator (RIND); MetaStock: Range Indicator; thinkorswim: RangeIndicator", ""),
    ("rogers_satchell", "TA-Lib: ROGERSSATCHELL", ""),
    ("squeeze", "TA-Lib: SQZMOM; pandas-ta: squeeze, squeeze_pro; TA4J: squeeze; thinkorswim: TTM_Squeeze; Sierra Chart: Bollinger Squeeze; Dhan / TradingView community: Squeeze Momentum Indicator [LazyBear]", ""),
    ("thermo", "pandas-ta: thermo", ""),
    ("vix_fix", "Sierra Chart: Synthetic VIX; Dhan / TradingView community: CM_Williams_Vix_Fix", ""),
    ("volatility_switch", "thinkorswim: VolatilitySwitch", ""),
    ("accbands", "TA-Lib: ACCBANDS; pandas-ta: accbands; thinkorswim: AccelerationBands", ""),
    ("esd_bands", "thinkorswim: ESDBands, ExponentialDeviationBands", ""),
    ("fib_bb", "Dhan / TradingView community: Fibonacci Bollinger Bands", ""),
    ("gann_sq9", "Sierra Chart: Square of 9", ""),
    ("greatest_swing", "Sierra Chart: Greatest Swing Value", ""),
    ("mavg_bands", "thinkorswim: MovAvgBands, MovAvgBandWidth", ""),
    ("murrey", "MetaTrader 5: MurreyMath Channel; Sierra Chart: Murrey Math", ""),
    ("nw_envelope", "TradingView community: Nadaraya-Watson Envelope [LuxAlgo]", ""),
    ("projection_bands", "MetaStock: Projection Bands; thinkorswim: ProjectionBands; MultiCharts: Upper-Lower Projection Band", ""),
    ("reg_channel", "cTrader: Polynomial Regression Channels; pandas-ta: tos_stdevall; TA4J: SGMA; MetaStock: Raff / Standard Error channel; TradingView: Linear Regression Channel; Zerodha Kite (ChartIQ): Linear Regression", ""),
    ("ultimate_bands", "thinkorswim: UltimateBands, UltimateChannels", ""),
    ("decay", "Tulip: decay, edecay; pandas-ta: decay", ""),
    ("entropy", "pandas-ta: entropy", ""),
    ("hurst", "Sierra Chart: Hurst Exponent; TA4J: HurstExponentIndicator", ""),
    ("leavitt_conv", "thinkorswim: LeavittConvolution, LeavittConvSlope, LeavittConvAcceleration", ""),
    ("linreg_angle", "TA-Lib: LINEARREG_ANGLE; Zerodha Kite (ChartIQ): Lin R2", ""),
    ("mean_dev", "TradingView: ta.dev; TA-Lib: AVGDEV; Tulip: md; pandas-ta: mad; TA4J: MeanDeviationIndicator; Chartink: mean deviation", ""),
    ("mode", "TradingView: ta.mode", ""),
    ("cdl_z", "pandas-ta: cdl_z", ""),
    ("gapless", "TradeStation: Gapless; MultiCharts: Gapless; Sierra Chart: Gapless bars", ""),
    ("heikin_ashi", "TA-Lib: HA; pandas-ta: ha; MetaTrader 4/5: Heiken Ashi; Sierra Chart: Heikin-Ashi (Smoothed); thinkorswim: Heikin Ashi; Zerodha Kite (ChartIQ): Heikin-Ashi; Chartink: Heikin-Ashi", ""),
    ("linreg_candles", "Dhan / TradingView community: Humble LinReg Candles", ""),
];

#[cfg(test)]
mod tests {
    use crate::lab::testdata::{assert_series, closes, compiles, d1, from_ohlc, go, walk};
    const N: f64 = f64::NAN;
    fn flat_ohlc(o: f64, h: f64, l: f64, c: f64, k: usize) -> crate::lab::bars::Bars { from_ohlc(&vec![(o, h, l, c); k]) }

    #[test] fn aptr_constant_two_percent_range() {
        // H = 101, L = 99, C = 100 every bar: TR 2, midpoint 100 → 2 %
        let o = go("aptr", &flat_ohlc(100., 101., 99., 100., 20), &[("length", 5.)]);
        assert_series(&o[0][..7], &[N, N, N, N, N, 2., 2.]);
    }
    #[test] fn gapo_range_equals_length_gives_one() {
        let b = from_ohlc(&[(10., 12., 10., 10.), (10., 11., 11., 11.)]);
        assert_series(&go("gapo", &b, &[("length", 2.)])[0], &[N, 1.]); // 12 − 10 = 2 = n
    }
    #[test] fn hl_volatility_constant_bars() {
        let o = go("hl_volatility", &flat_ohlc(50., 51., 49., 50., 10), &[("length", 2.)]);
        assert_series(&o[0], &[N, N, 4., 4., 4., 4., 4., 4., 4., 4.]);
    }
    #[test] fn rind_bounded_and_flat_zero() {
        let o = go("rind", &walk(300, 3, false), &[]);
        assert!(o[0][12..].iter().all(|v| (0. ..=100.).contains(v)));
        assert!(go("rind", &flat_ohlc(5., 5., 5., 5., 20), &[])[0][12..].iter().all(|&v| v == 0.));
    }
    #[test] fn rogers_satchell_flat_zero_and_value() {
        assert!(go("rogers_satchell", &flat_ohlc(5., 5., 5., 5., 12), &[])[0][9..].iter().all(|&v| v == 0.));
        // O = 100, H = 110, L = 90, C = 100: term = ln(1.1)² + ln(0.9)²
        let b = from_ohlc(&[(100., 110., 90., 100.)]);
        let t: f64 = 1.1f64.ln().powi(2) + 0.9f64.ln().powi(2);
        assert_series(&go("rogers_satchell", &b, &[("length", 1.), ("days", 1.)])[0], &[100. * t.sqrt()]);
    }
    #[test] fn squeeze_flat_has_no_squeeze_and_quiet_bars_do() {
        let o = go("squeeze", &flat_ohlc(5., 5., 5., 5., 40), &[]);
        assert_eq!((o[0][25], o[1][25], o[2][25]), (0., 0., 0.));
        // wide bars, closes alternating by 0.2: BB inside the TR channel at every factor
        let b = from_ohlc(&(0..60).map(|i| { let c = 100. + if i % 2 == 0 { 0.1 } else { -0.1 }; (c, c + 2., c - 2., c) }).collect::<Vec<_>>());
        let o = go("squeeze", &b, &[]);
        assert_eq!((o[0][30], o[1][30], o[2][30]), (1., 0., 3.));
    }
    #[test] fn squeeze_momentum_is_ta_linreg_of_deviation() {
        // k = 2: bar 1 dev = 11.5 − (10.5 + 11)/2 = 0.75; bar 2 dev = 12 − (11.25 + 11.75)/2 = 0.5; a 2-point linreg ends on the last point
        let o = go("squeeze", &d1(), &[("bb_length", 2.), ("kc_length", 2.)]);
        assert_series(&o[3][..3], &[N, N, 0.5]);
    }
    #[test] fn thermo_values_and_signals() {
        let b = from_ohlc(&[(10., 12., 9., 11.), (11., 11.5, 9.5, 11.), (11., 14., 10., 13.)]);
        let o = go("thermo", &b, &[("length", 2.)]);
        assert_series(&o[0], &[N, 0.5, 2.5]);
        assert_series(&o[1], &[N, N, 1.5]);
        assert_series(&o[2], &[N, N, 1.]); // 2.5 < 2·1.5
        assert_series(&o[3], &[N, N, 1.]); // 2.5 > 0.5·1.5
    }
    #[test] fn vix_fix_low_at_highest_close_is_zero() {
        assert_series(&go("vix_fix", &closes(&[10., 11., 12.]), &[("length", 2.)])[0], &[N, 0., 0.]);
        let b = from_ohlc(&[(10., 10., 10., 10.), (10., 10., 8., 9.)]);
        assert_series(&go("vix_fix", &b, &[("length", 2.)])[0], &[N, 20.]);
    }
    #[test] fn volatility_switch_rising_volatility_is_one() {
        let mut c = vec![100.];
        for i in 1..40 { let last = c[c.len() - 1]; c.push(last * (1. + if i % 2 == 0 { 1. } else { -1. } * 0.001 * i as f64)); }
        let o = go("volatility_switch", &closes(&c), &[("length", 5.)]);
        assert_eq!(o[0].iter().position(|v| !v.is_nan()), Some(9));
        assert!(o[0][9..].iter().all(|&v| v == 1.));
    }
    #[test] fn accbands_zero_factor_and_hand_value() {
        let o = go("accbands", &d1(), &[("length", 2.), ("factor", 0.)]);
        assert_series(&o[0], &[N, 11.5, 12.25, 12.35, 11.85, 11.25, 11.5, 12.5]);
        assert_series(&o[2][..3], &[N, 9.5, 10.5]);
        let b = from_ohlc(&[(10., 12., 8., 10.)]);
        assert_series(&go("accbands", &b, &[("length", 1.), ("factor", 4.)])[0], &[12. * 1.8]); // f = 4·4/20 = 0.8
    }
    #[test] fn esd_bands_collapse_and_hand_value() {
        let o = go("esd_bands", &closes(&[5.; 30]), &[]);
        assert_series(&o[1][19..21], &[5., 5.]);
        // n = 2: ema seed (1 + 3)/2 = 2; dev = √(((1 − 2)² + (3 − 2)²)/2) = 1
        assert_series(&go("esd_bands", &closes(&[1., 3.]), &[("length", 2.), ("mult", 2.)])[1], &[N, 4.]);
    }
    #[test] fn fib_bb_outer_band_is_mult_stdev() {
        let o = go("fib_bb", &d1(), &[("length", 3.), ("mult", 3.)]);
        let sd = crate::lab::ta::stdev(&d1().hlc3(), 3);
        for i in 2..8 { assert!((o[6][i] - o[0][i] - 3. * sd[i]).abs() < 1e-9 && (o[0][i] - o[12][i] - 3. * sd[i]).abs() < 1e-9); }
    }
    #[test] fn gann_sq9_180_degrees() {
        let o = go("gann_sq9", &closes(&[100., 50.]), &[("degrees", 180.)]);
        assert_series(&o[4], &[N, 121.]);
        assert_series(&o[3], &[N, 81.]);
    }
    #[test] fn greatest_swing_skip_zeros() {
        let up = from_ohlc(&[(10., 12., 9., 11.); 6]); // no down bars → average 0 → buy level = open
        assert_series(&go("greatest_swing", &up, &[("length", 2.), ("mult", 2.)])[0], &[N, N, 10., 10., 10., 10.]);
        // bar 0 down: bs = 12 − 11 = 1; bar 1 up: 0 (skipped) → average 1 → bar 2 buy = 10 + 2·1
        let b = from_ohlc(&[(11., 12., 9., 10.), (10., 12., 9., 11.), (10., 12., 9., 11.)]);
        assert_series(&go("greatest_swing", &b, &[("length", 2.), ("mult", 2.)])[0], &[N, N, 12.]);
    }
    #[test] fn mavg_bands_equal_lengths_zero_width() {
        let o = go("mavg_bands", &d1(), &[("fast", 3.), ("slow", 3.)]);
        assert!(o[3][4..].iter().all(|&v| v == 0.));
    }
    #[test] fn murrey_lines_equally_spaced_and_hand_value() {
        let o = go("murrey", &walk(200, 2, false), &[]);
        for i in 64..200 { let s = o[3][i] - o[2][i]; for k in 1..13 { assert!((o[k][i] - o[k - 1][i] - s).abs() < 1e-9); } }
        // min 90, max 110: fractal 100, octave 25, mn 75, mx 125; only the x3 rule holds → resistance 112.5, support 87.5
        let c: Vec<_> = (0..4).map(|i| (100., 110., 90., 100. + i as f64)).collect();
        let o = go("murrey", &from_ohlc(&c), &[("length", 3.)]);
        assert_series(&[o[2][3], o[10][3], o[0][3]], &[87.5, 112.5, 81.25]);
    }
    #[test] fn nw_envelope_constant_collapses() {
        let o = go("nw_envelope", &closes(&[7.; 12]), &[("window", 4.), ("bandwidth", 2.)]);
        assert_series(&o[0][..5], &[N, N, N, 7., 7.]);
        assert_series(&o[1][4..6], &[N, 7.]);
    }
    #[test] fn projection_bands_flat_highs() {
        let b = from_ohlc(&[(10., 12., 9., 11.), (10., 12., 8., 11.), (10., 12., 7., 11.)]);
        let o = go("projection_bands", &b, &[("length", 3.)]);
        assert_series(&o[0], &[N, N, 12.]);
        assert_series(&o[1], &[N, N, 7.]); // lows on a −1 line: every projection lands on 7
    }
    #[test] fn reg_channel_degree1_is_lsma_and_exact_fit_collapses() {
        let b = d1();
        let o = go("reg_channel", &b, &[("length", 4.), ("degree", 1.)]);
        let l = crate::lab::ta::linreg(&b.close, 4, 0);
        for i in 3..8 { assert!((o[0][i] - l[i]).abs() < 1e-9); }
        let o = go("reg_channel", &closes(&[1., 4., 9., 16., 25.]), &[("length", 4.), ("degree", 2.)]);
        assert_series(&o[0], &[N, N, N, 16., 25.]);
        assert_series(&o[1], &[N, N, N, 16., 25.]);
    }
    #[test] fn ultimate_bands_constant_collapses_and_first_bars_pass_through() {
        let o = go("ultimate_bands", &closes(&[3.; 30]), &[]);
        assert!((o[1][25] - 3.).abs() < 1e-9 && (o[2][25] - 3.).abs() < 1e-9);
        assert_series(&go("ultimate_bands", &closes(&[1., 2., 4., 8.]), &[("length", 3.)])[0][..3], &[1., 2., 4.]);
    }
    #[test] fn decay_linear_and_exponential() {
        assert_series(&go("decay", &closes(&[1., 0., 0., 0.]), &[("length", 5.)])[0], &[1., 0.8, 0.6, 0.4]);
        assert_series(&go("decay", &closes(&[1., 0., 0.]), &[("length", 5.), ("mode", 1.)])[0], &[1., 0.8, 0.64]);
    }
    #[test] fn entropy_equal_closes() { assert_series(&go("entropy", &closes(&[4.; 8]), &[("length", 8.)])[0][7..], &[3.]); }
    #[test] fn hurst_random_vs_persistent() {
        let v = go("hurst", &walk(400, 5, false), &[])[0][399];
        assert!(v > 0.2 && v < 0.8, "{v}");
        // returns in long same-sign runs are persistent
        let mut c = vec![100.];
        for i in 1..400 { let last = c[c.len() - 1]; c.push(last * (1. + if (i / 32) % 2 == 0 { 0.01 } else { -0.01 } + 0.001 * ((i * 7919) % 13) as f64 / 13.)); }
        let t = go("hurst", &closes(&c), &[])[0][399];
        assert!(t > v, "{t} vs {v}");
        assert!(go("hurst", &walk(300, 5, false), &[("variant", 1.)])[0][200].is_finite());
    }
    #[test] fn leavitt_on_a_line() {
        let c: Vec<f64> = (0..20).map(|i| 2. * i as f64).collect();
        let o = go("leavitt_conv", &closes(&c), &[("length", 4.)]);
        assert_series(&o[0][5..7], &[14., 16.]); // proj = next point; conv projects proj one more bar
        assert_series(&o[1][4..6], &[2., 2.]);
        assert_series(&o[2][4..6], &[N, 0.]);
    }
    #[test] fn linreg_angle_45() { assert_series(&go("linreg_angle", &closes(&[1., 2., 3., 4.]), &[("length", 3.)])[0], &[N, N, 45., 45.]); }
    #[test] fn mean_dev_hand() { assert_series(&go("mean_dev", &closes(&[1., 2., 3., 4.]), &[("length", 4.)])[0], &[N, N, N, 1.]); }
    #[test] fn mode_ties_smallest() {
        assert_series(&go("mode", &closes(&[3., 1., 3., 2.]), &[("length", 4.)])[0][3..], &[3.]);
        assert_series(&go("mode", &closes(&[4., 2., 7.]), &[("length", 3.)])[0][2..], &[2.]);
    }
    #[test] fn cdl_z_sample_stdev() { assert_series(&go("cdl_z", &closes(&[1., 2., 3.]), &[("length", 3.)])[3], &[N, N, 1.]); }
    #[test] fn gapless_removes_gap() {
        let b = from_ohlc(&[(10., 11., 9., 10.), (12., 13., 11., 12.5), (12.5, 13., 12., 12.)]);
        let o = go("gapless", &b, &[]);
        assert_series(&o[0], &[10., 10., 10.5]);
        assert_series(&o[3], &[10., 10.5, 10.]);
    }
    #[test] fn heikin_ashi_bar0_and_next() {
        let o = go("heikin_ashi", &from_ohlc(&[(10., 12., 9., 11.), (11., 13., 10., 12.)]), &[]);
        assert_series(&o[3], &[10.5, 11.5]);
        assert_series(&o[0], &[10.5, 10.5]);
        assert_series(&o[1], &[12., 13.]);
        assert_series(&o[2], &[9., 10.]);
    }
    #[test] fn linreg_candles_close_is_lsma() {
        let b = d1(); let o = go("linreg_candles", &b, &[("length", 3.), ("signal", 2.)]);
        assert_series(&o[3], &crate::lab::ta::linreg(&b.close, 3, 0));
        assert_series(&o[4][..4], &[N, N, N, (o[3][2] + o[3][3]) / 2.]);
    }
    #[test] fn rules_reach_new_series() {
        for (id, out) in [("squeeze", "momentum"), ("murrey", "mm_4"), ("heikin_ashi", "close"), ("hurst", "hurst"), ("fib_bb", "u618")] { compiles(id, out).unwrap(); }
    }
}
