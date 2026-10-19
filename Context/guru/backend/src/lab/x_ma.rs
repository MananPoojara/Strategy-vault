//! Cross-platform indicators, Moving averages (2026-10-19 build list: TradingView, AmiBroker, MetaTrader, cTrader, NinjaTrader, TradeStation, thinkorswim, Sierra Chart, MetaStock). Formulas: see each definition string.
use super::bars::Bars;
use super::catalogue::{int, len, n, p, real, IndicatorDef, Needs, Output, ParamSpec, Params, Placement};
use super::ta::{self, NA};
use std::f64::consts::PI;

const CAT: &str = "Moving averages";
const P: Placement = Placement::Price;
const NO: Needs = Needs::NONE;
#[allow(clippy::too_many_arguments)]
const fn def(id: &'static str, name: &'static str, category: &'static str, params: &'static [ParamSpec], outputs: &'static [&'static str], placement: Placement,
             needs: Needs, definition: &'static str, first: &'static [usize], f: fn(&Bars, &Params) -> Output) -> IndicatorDef {
    IndicatorDef { id, name, category, params, outputs, placement, needs, definition, first, sparse: false, compute: Some(f), note: "" }
}

/// Fixed-weight window (weights oldest -> newest), normalised by their sum.
fn weighted(x: &[f64], w: &[f64]) -> Vec<f64> {
    let (k, s) = (w.len(), w.iter().sum::<f64>());
    (0..x.len()).map(|i| if i + 1 < k { NA } else { x[i + 1 - k..=i].iter().zip(w).map(|(a, b)| a * b).sum::<f64>() / s }).collect()
}

// ---- fixed-weight averages (pandas-ta) ----
fn fwma(b: &Bars, prm: &Params) -> Output {
    let k = n(prm, "length");
    let mut w = vec![1.; k];
    for j in 2..k { w[j] = w[j - 1] + w[j - 2]; }
    Ok(vec![weighted(&b.close, &w)])
}
fn pwma(b: &Bars, prm: &Params) -> Output {
    let k = n(prm, "length");
    // binomial row k-1 in log space so long lengths do not overflow
    let mut lw = vec![0.; k];
    for j in 1..k { lw[j] = lw[j - 1] + (((k - j) as f64) / j as f64).ln(); }
    let m = lw.iter().cloned().fold(f64::MIN, f64::max);
    Ok(vec![weighted(&b.close, &lw.iter().map(|v| (v - m).exp()).collect::<Vec<_>>())])
}
fn sinwma(b: &Bars, prm: &Params) -> Output {
    let k = n(prm, "length");
    Ok(vec![weighted(&b.close, &(0..k).map(|i| ((i + 1) as f64 * PI / (k + 1) as f64).sin()).collect::<Vec<_>>())])
}
fn hwma(b: &Bars, prm: &Params) -> Output {
    let (na, nb, nc) = (p(prm, "na"), p(prm, "nb"), p(prm, "nc"));
    let (mut f, mut v, mut a) = (b.close.first().copied().unwrap_or(NA), 0., 0.);
    Ok(vec![b.close.iter().map(|&x| {
        let f1 = (1. - na) * (f + v + 0.5 * a) + na * x;
        let v1 = (1. - nb) * (v + a) + nb * (f1 - f);
        let a1 = (1. - nc) * a + nc * (v1 - v);
        (f, v, a) = (f1, v1, a1);
        f + v + 0.5 * a
    }).collect()])
}

// ---- TA-Lib family ----
fn t3(b: &Bars, prm: &Params) -> Output {
    let (k, v) = (n(prm, "length"), p(prm, "vfactor"));
    let mut e = vec![ta::ema(&b.close, k)];
    for _ in 0..5 { let next = ta::ema(e.last().unwrap(), k); e.push(next); }
    let (c1, c2, c3, c4) = (-v * v * v, 3. * v * v + 3. * v * v * v, -6. * v * v - 3. * v - 3. * v * v * v, 1. + 3. * v + v * v * v + 3. * v * v);
    Ok(vec![(0..b.len()).map(|i| c1 * e[5][i] + c2 * e[4][i] + c3 * e[3][i] + c4 * e[2][i]).collect()])
}
fn trima(b: &Bars, prm: &Params) -> Output {
    let k = n(prm, "length");
    let (a, c) = if k % 2 == 1 { ((k + 1) / 2, (k + 1) / 2) } else { (k / 2, k / 2 + 1) };
    Ok(vec![ta::sma(&ta::sma(&b.close, a), c)])
}
fn zlema(b: &Bars, prm: &Params) -> Output {
    let k = n(prm, "length");
    let lag = (k - 1) / 2;
    let x = &b.close;
    Ok(vec![ta::ema(&(0..x.len()).map(|i| if i >= lag { 2. * x[i] - x[i - lag] } else { NA }).collect::<Vec<_>>(), k)])
}
fn swak_lowpass(b: &Bars, prm: &Params) -> Output {
    let (k, butter) = (n(prm, "length") as f64, p(prm, "mode") == 1.);
    let s = (PI / k).sin();
    let bb = 2.415 * (2. * s * s);
    let a = -bb + (bb * bb + 2. * bb).sqrt();
    let (om, x) = (1. - a, &b.close);
    let mut out = vec![NA; x.len()];
    // TA-Lib seeds every slot with the first bar (steady state of a constant input)
    let (mut y1, mut y2) = (x.first().copied().unwrap_or(NA), x.first().copied().unwrap_or(NA));
    for i in 0..x.len() {
        let inp = if butter { a * a / 4. * (x[i] + 2. * x[i.saturating_sub(1)] + x[i.saturating_sub(2)]) } else { a * a * x[i] };
        let y = 2. * om * y1 - om * om * y2 + inp;
        (y2, y1) = (y1, y);
        out[i] = y;
    }
    Ok(vec![out])
}
fn frama(b: &Bars, prm: &Params) -> Output {
    let (k, x) = (n(prm, "length"), b.len());
    let rng = |lo: usize, hi: usize| -> (f64, f64) { (b.high[lo..=hi].iter().cloned().fold(f64::MIN, f64::max), b.low[lo..=hi].iter().cloned().fold(f64::MAX, f64::min)) };
    let mut out = vec![NA; x];
    if p(prm, "variant") == 0. {
        let half = k / 2;
        let w = 2 * half;
        let price = b.hl2();
        let es = -4.6 / std::f64::consts::LN_2;
        if x < w + 1 { return Ok(vec![out]); }
        let mut prev = price[w - 1];
        for t in w..x {
            let (h1, l1) = rng(t + 1 - half, t);
            let (h2, l2) = rng(t + 1 - w, t - half);
            let (r1, r2, r) = (h1 - l1, h2 - l2, h1.max(h2) - l1.min(l2));
            prev = if r1 > 0. && r2 > 0. && r1 + r2 > r { let a = (es * ((r1 + r2) / r).ln()).exp(); (1. - a) * prev + a * price[t] } else { price[t] };
            out[t] = prev;
        }
    } else {
        let c = &b.close;
        if x < 2 * k { return Ok(vec![out]); }
        let mut prev = c[2 * k - 2];
        for i in 2 * k - 1..x {
            let (h1, l1) = rng(i + 1 - k, i);
            let (h2, l2) = rng(i + 1 - 2 * k, i - k);
            let (h3, l3) = rng(i + 1 - 2 * k, i);
            let d = (((h1 - l1) / k as f64 + (h2 - l2) / k as f64).ln() - ((h3 - l3) / (2 * k) as f64).ln()) / std::f64::consts::LN_2;
            let a = (-4.6 * (d - 1.)).exp();
            // zero-range windows make MT5's log undefined: take the close there (as TA-Lib)
            prev = if a.is_finite() { a * c[i] + (1. - a) * prev } else { c[i] };
            out[i] = prev;
        }
    }
    Ok(vec![out])
}
fn vidya(b: &Bars, prm: &Params) -> Output {
    let (l, cl, x) = (n(prm, "length"), n(prm, "cmo_length"), &b.close);
    let a = 2. / (l as f64 + 1.);
    let mut out = vec![NA; x.len()];
    if p(prm, "variant") == 0. {
        let start = l + cl - 1;
        if x.len() <= start { return Ok(vec![out]); }
        let mut prev = x[start - 1];
        for i in start..x.len() {
            let (mut up, mut dn) = (0., 0.);
            for j in 0..cl { let d = x[i - j] - x[i - j - 1]; if d > 0. { up += d } else { dn -= d } }
            let cmo = if up + dn != 0. { (up - dn) / (up + dn) } else { 0. };
            prev = x[i] * a * cmo.abs() + prev * (1. - a * cmo.abs());
            out[i] = prev;
        }
    } else {
        if l < 2 || cl > l { return Err("vidya: variant 1 needs length >= 2 and cmo_length <= length".into()); }
        if x.len() < l - 1 { return Ok(vec![out]); }
        let (ss, sl) = (ta::stdev(x, cl), ta::stdev(x, l));
        let mut prev = x[l - 2];
        out[l - 2] = prev;
        for i in l - 1..x.len() {
            let k = ss[i] / sl[i];
            let k = if k.is_finite() { k * a } else { 0. };
            prev += (x[i] - prev) * k;
            out[i] = prev;
        }
    }
    Ok(vec![out])
}

// ---- Hilbert transform core shared by MAMA and HT_TRENDLINE (TA-Lib ta_MAMA.c / ta_HT_TRENDLINE.c); slot 0 = even bars, 1 = odd ----
#[derive(Default)]
struct Tf { buf: [[f64; 3]; 2], prev: [f64; 2], pin: [f64; 2] }
impl Tf {
    fn step(&mut self, e: usize, idx: usize, input: f64, adj: f64) -> f64 {
        let t = 0.0962 * input;
        let mut out = -self.buf[e][idx];
        self.buf[e][idx] = t;
        out += t - self.prev[e];
        self.prev[e] = 0.5769 * self.pin[e];
        out += self.prev[e];
        self.pin[e] = input;
        out * adj
    }
}
#[derive(Default)]
struct Hilbert { det: Tf, q1: Tf, ji: Tf, jq: Tf, idx: usize, i1p2: [f64; 2], i1p3: [f64; 2], pi2: f64, pq2: f64, re: f64, im: f64, period: f64 }
impl Hilbert {
    /// One bar: returns (Q1, I1 used for the phase); updates the period afterwards.
    fn step(&mut self, today: usize, smooth: f64) -> (f64, f64) {
        let adj = 0.075 * self.period + 0.54;
        let e = today % 2;
        let o = 1 - e;
        let i1 = self.i1p3[e];
        let det = self.det.step(e, self.idx, smooth, adj);
        let q1 = self.q1.step(e, self.idx, det, adj);
        let ji = self.ji.step(e, self.idx, i1, adj);
        let jq = self.jq.step(e, self.idx, q1, adj);
        if e == 0 { self.idx = (self.idx + 1) % 3; }
        let q2 = 0.2 * (q1 + ji) + 0.8 * self.pq2;
        let i2 = 0.2 * (i1 - jq) + 0.8 * self.pi2;
        self.i1p3[o] = self.i1p2[o];
        self.i1p2[o] = det;
        self.re = 0.8 * self.re + 0.2 * (i2 * self.pi2 + q2 * self.pq2);
        self.im = 0.8 * self.im + 0.2 * (i2 * self.pq2 - q2 * self.pi2);
        (self.pq2, self.pi2) = (q2, i2);
        let prev = self.period;
        if self.im != 0. && self.re != 0. { self.period = 360. / (self.im / self.re).atan().to_degrees(); }
        self.period = self.period.min(1.5 * prev).max(0.67 * prev).clamp(6., 50.);
        self.period = 0.2 * self.period + 0.8 * prev;
        (q1, i1)
    }
}
fn wma4(x: &[f64], t: usize) -> f64 { (4. * x[t] + 3. * x[t - 1] + 2. * x[t - 2] + x[t - 3]) / 10. }
fn mama(b: &Bars, prm: &Params) -> Output {
    let (fast, slow, x) = (p(prm, "fast_limit"), p(prm, "slow_limit"), &b.close);
    let (mut m, mut f) = (vec![NA; x.len()], vec![NA; x.len()]);
    let (mut h, mut mm, mut ff, mut prev_phase) = (Hilbert::default(), 0., 0., 0.);
    for t in 12..x.len() {
        let (q1, i1) = h.step(t, wma4(x, t));
        let phase = if i1 != 0. { (q1 / i1).atan().to_degrees() } else { 0. };
        let dphase = (prev_phase - phase).max(1.);
        prev_phase = phase;
        let alpha = if dphase > 1. { (fast / dphase).max(slow) } else { fast };
        mm = (1. - alpha) * mm + alpha * x[t];
        ff = (1. - alpha / 2.) * ff + alpha / 2. * mm;
        if t >= 32 { m[t] = mm; f[t] = ff; }
    }
    Ok(vec![m, f])
}
fn ht_trendline(b: &Bars, _: &Params) -> Output {
    let x = &b.close;
    let mut out = vec![NA; x.len()];
    let (mut h, mut sp, mut it) = (Hilbert::default(), 0., [0.; 3]);
    for t in 37..x.len() {
        h.step(t, wma4(x, t));
        sp = 0.67 * sp + 0.33 * h.period;
        let dc = (sp + 0.5).max(0.) as usize;
        let s: f64 = (0..dc.min(50).min(t + 1)).map(|i| x[t - i]).sum();
        let avg = if dc > 0 { s / dc as f64 } else { s };
        let v = (4. * avg + 3. * it[0] + 2. * it[1] + it[2]) / 10.;
        it = [avg, it[0], it[1]];
        if t >= 63 { out[t] = v; }
    }
    Ok(vec![out])
}

// ---- Ehlers filters (Cybernetic Analysis / Cycle Analytics / TASC code) and Sierra Chart studies ----
const V: Needs = Needs { volume: true, ..Needs::NONE };
fn vol(b: &Bars) -> &[f64] { b.volume.as_deref().unwrap_or(&[]) }
/// a = exp(-1.414*pi/k), b = 2a*cos(1.414*pi/k) (Ehlers' two-pole constants; his EasyLanguage cosine takes degrees, 1.414*180/k).
fn pole2(k: f64) -> (f64, f64) { let a = (-1.414 * PI / k).exp(); (a, 2. * a * (1.414 * PI / k).cos()) }
/// a = exp(-pi/k), b = 2a*cos(1.738*pi/k), c = a^2 (Ehlers' three-pole constants).
fn pole3(k: f64) -> (f64, f64, f64) { let a = (-PI / k).exp(); (a, 2. * a * (1.738 * PI / k).cos(), a * a) }
/// Recursion over bars `from..`, earlier bars copy x (the platforms' seed); `f(i, y)` sees the outputs so far.
fn recur(x: &[f64], from: usize, f: impl Fn(usize, &[f64]) -> f64) -> Vec<f64> {
    let mut y = x.to_vec();
    for i in from..x.len() { y[i] = f(i, &y); }
    y
}
pub(super) fn supersmoother_s(x: &[f64], k: f64, poles: usize) -> Vec<f64> {
    if poles == 2 {
        let (a, b) = pole2(k);
        let (c2, c3) = (b, -a * a);
        let c1 = 1. - c2 - c3;
        recur(x, 2, |i, y| c1 * (x[i] + x[i - 1]) / 2. + c2 * y[i - 1] + c3 * y[i - 2])
    } else {
        let (_, b, c) = pole3(k);
        let (d2, d3, d4) = (b + c, -(c + b * c), c * c);
        let d1 = 1. - d2 - d3 - d4;
        recur(x, 3, |i, y| d1 * x[i] + d2 * y[i - 1] + d3 * y[i - 2] + d4 * y[i - 3])
    }
}
fn supersmoother(b: &Bars, prm: &Params) -> Output { Ok(vec![supersmoother_s(&b.close, p(prm, "length"), n(prm, "poles"))]) }
fn butterworth(b: &Bars, prm: &Params) -> Output {
    let (k, x) = (p(prm, "length"), &b.close);
    Ok(vec![if n(prm, "poles") == 2 {
        let (a, bb) = pole2(k);
        let c1 = (1. - bb + a * a) / 4.;
        recur(x, 2, |i, y| c1 * (x[i] + 2. * x[i - 1] + x[i - 2]) + bb * y[i - 1] - a * a * y[i - 2])
    } else {
        let (_, bb, c) = pole3(k);
        let (c1, c2, c3, c4) = ((1. - bb + c) * (1. - c) / 8., bb + c, -(c + bb * c), c * c);
        recur(x, 3, |i, y| c1 * (x[i] + 3. * x[i - 1] + 3. * x[i - 2] + x[i - 3]) + c2 * y[i - 1] + c3 * y[i - 2] + c4 * y[i - 3])
    }])
}
fn ultimate_smoother(b: &Bars, prm: &Params) -> Output {
    let (a, c2) = pole2(p(prm, "length"));
    let c3 = -a * a;
    let c1 = (1. + c2 - c3) / 4.;
    let x = &b.close;
    Ok(vec![recur(x, 3, |i, y| (1. - c1) * x[i] + (2. * c1 - c2) * x[i - 1] - (c1 + c3) * x[i - 2] + c2 * y[i - 1] + c3 * y[i - 2])])
}
/// Ehlers' EMA constant for a cutoff period k: (cos w + sin w - 1)/cos w, w = 2pi/k.
fn ehlers_alpha(k: f64) -> f64 { let w = 2. * PI / k; (w.cos() + w.sin() - 1.) / w.cos() }
fn decycler(b: &Bars, prm: &Params) -> Output {
    let al = ehlers_alpha(p(prm, "length") / 0.707);
    let x = &b.close;
    let mut hp = vec![0.; x.len()];
    for i in 2..x.len() { hp[i] = (1. - al / 2.).powi(2) * (x[i] - 2. * x[i - 1] + x[i - 2]) + 2. * (1. - al) * hp[i - 1] - (1. - al).powi(2) * hp[i - 2]; }
    Ok(vec![(0..x.len()).map(|i| if i < 2 { NA } else { x[i] - hp[i] }).collect()])
}
fn inst_trendline(b: &Bars, prm: &Params) -> Output {
    let a = 2. / (p(prm, "length") + 1.);
    let x = b.hl2();
    let mut it = vec![NA; x.len()];
    for i in 2..x.len() {
        it[i] = if i < 6 { (x[i] + 2. * x[i - 1] + x[i - 2]) / 4. } else {
            (a - a * a / 4.) * x[i] + a * a / 2. * x[i - 1] - (a - 0.75 * a * a) * x[i - 2] + 2. * (1. - a) * it[i - 1] - (1. - a).powi(2) * it[i - 2]
        };
    }
    let trig = (0..x.len()).map(|i| if i < 4 { NA } else { 2. * it[i] - it[i - 2] }).collect();
    Ok(vec![it, trig])
}
fn laguerre_filter(b: &Bars, prm: &Params) -> Output {
    let g = p(prm, "gamma");
    let x = b.hl2();
    let mut l = [x.first().copied().unwrap_or(NA); 4];
    Ok(vec![x.iter().enumerate().map(|(i, &v)| {
        if i > 0 {
            let l0 = (1. - g) * v + g * l[0];
            let l1 = -g * l0 + l[0] + g * l[1];
            let l2 = -g * l1 + l[1] + g * l[2];
            let l3 = -g * l2 + l[2] + g * l[3];
            l = [l0, l1, l2, l3];
        }
        (l[0] + 2. * l[1] + 2. * l[2] + l[3]) / 6.
    }).collect()])
}
fn rmf(b: &Bars, prm: &Params) -> Output {
    let al = ehlers_alpha(p(prm, "length"));
    let x = &b.close;
    let med: Vec<f64> = (0..x.len()).map(|i| if i < 4 { NA } else { let mut w = x[i - 4..=i].to_vec(); w.sort_by(|a, b| a.total_cmp(b)); w[2] }).collect();
    let mut out = vec![NA; x.len()];
    for i in 4..x.len() { out[i] = if i == 4 { med[i] } else { al * med[i] + (1. - al) * out[i - 1] }; }
    Ok(vec![out])
}
fn dsma(b: &Bars, prm: &Params) -> Output {
    let k = p(prm, "length");
    let (a, c2) = pole2(0.5 * k);
    let c3 = -a * a;
    let c1 = 1. - c2 - c3;
    let (x, m) = (&b.close, n(prm, "length"));
    let at = |v: &[f64], i: usize, back: usize| if i >= back { v[i - back] } else { 0. };
    let z: Vec<f64> = (0..x.len()).map(|i| if i >= 2 { x[i] - x[i - 2] } else { 0. }).collect();
    let mut f = vec![0.; x.len()];
    let mut out = vec![NA; x.len()];
    let (mut scaled, mut prev) = (0., x.first().copied().unwrap_or(NA));
    for i in 0..x.len() {
        f[i] = c1 * (z[i] + at(&z, i, 1)) / 2. + c2 * at(&f, i, 1) + c3 * at(&f, i, 2);
        let rms = ((0..m).map(|c| at(&f, i, c).powi(2)).sum::<f64>() / k).sqrt();
        if rms != 0. { scaled = f[i] / rms; }
        let al = scaled.abs() * 5. / k;
        prev = al * x[i] + (1. - al) * prev;
        out[i] = prev;
    }
    Ok(vec![out])
}
fn distant_coef(b: &Bars, prm: &Params) -> Output {
    let k = n(prm, "length");
    let x = b.hl2();
    let mut out = vec![NA; x.len()];
    let mut last = NA;
    for i in 2 * (k - 1)..x.len() {
        let (mut num, mut sc) = (0., 0.);
        for c in 0..k {
            let d2: f64 = (1..k).map(|lb| (x[i - c] - x[i - c - lb]).powi(2)).sum();
            num += d2 * x[i - c];
            sc += d2;
        }
        if sc != 0. { last = num / sc; }
        out[i] = last;
    }
    Ok(vec![out])
}
fn evwma(b: &Bars, prm: &Params) -> Output {
    let (k, x, v) = (n(prm, "length"), &b.close, vol(b));
    let mut out = vec![NA; x.len()];
    let mut nsum = 0.;
    for t in 0..x.len() {
        // Sierra Chart pads the first length bars with volume[0]
        nsum = if t == 0 { k as f64 * v[0] } else { nsum + v[t] - if t >= k { v[t - k] } else { v[0] } };
        out[t] = if t == 0 { x[0] } else if nsum != 0. { (1. - v[t] / nsum) * out[t - 1] + v[t] * x[t] / nsum } else { out[t - 1] };
    }
    Ok(vec![out])
}
fn moma(b: &Bars, prm: &Params) -> Output {
    let (k, x, v) = (n(prm, "length"), &b.close, vol(b));
    let xv: Vec<f64> = x.iter().zip(v).map(|(a, w)| a * w).collect();
    let (sxv, sv, w) = (ta::sum(&xv, k), ta::sum(v, k), ta::wma(x, k));
    let mut out = vec![vec![NA; x.len()]; 3];
    for t in k..x.len() {
        let (mut num, mut den) = (0., 0.);
        for i in t + 1 - k..=t { let m = (x[i] - x[i - 1]).abs(); num += x[i] * m; den += m; }
        let mo = if den != 0. { num / den } else { x[t] };
        let vw = if sv[t] != 0. { sxv[t] / sv[t] } else { NA };
        out[0][t] = mo;
        out[1][t] = (mo + vw) / 2.;
        out[2][t] = (mo + vw + w[t]) / 3.;
    }
    Ok(out)
}
fn rainbow(b: &Bars, prm: &Params) -> Output {
    let k = n(prm, "length");
    let mut lines = vec![ta::sma(&b.close, k)];
    for _ in 1..10 { let next = ta::sma(lines.last().unwrap(), k); lines.push(next); }
    let avg = (0..b.len()).map(|i| lines.iter().map(|l| l[i]).sum::<f64>() / 10.).collect();
    lines.push(avg);
    Ok(lines)
}
fn coral(b: &Bars, prm: &Params) -> Output {
    let (sm, cd) = (p(prm, "length"), p(prm, "cd"));
    let di = (sm - 1.) / 2. + 1.;
    let c1 = 2. / (di + 1.);
    let c2 = 1. - c1;
    let (c3, c4, c5) = (3. * (cd * cd + cd * cd * cd), -3. * (2. * cd * cd + cd + cd * cd * cd), 3. * cd + 1. + cd * cd * cd + 3. * cd * cd);
    let mut e = [0.; 6];
    Ok(vec![b.close.iter().map(|&x| {
        let mut inp = x;
        for s in e.iter_mut() { *s = c1 * inp + c2 * *s; inp = *s; }
        -cd * cd * cd * e[5] + c3 * e[4] + c4 * e[3] + c5 * e[2]
    }).collect()])
}

// ---- adaptive EMAs (Apirine, Cong) and Arms' VAMA ----
/// Full-window extreme: NaN unless the last k values all exist (Pine/TradeStation highest/lowest on a series with a warm-up).
fn ext(x: &[f64], k: usize, max: bool) -> Vec<f64> {
    (0..x.len()).map(|i| if i + 1 < k { NA } else {
        let w = &x[i + 1 - k..=i];
        if w.iter().any(|v| v.is_nan()) { NA } else if max { w.iter().cloned().fold(f64::MIN, f64::max) } else { w.iter().cloned().fold(f64::MAX, f64::min) }
    }).collect()
}
/// `y = y[1] + rate(i)*(x - y[1])` from bar `seed` (which outputs x), NaN before.
fn adaptive(x: &[f64], seed: usize, rate: impl Fn(usize) -> f64) -> Vec<f64> {
    let mut out = vec![NA; x.len()];
    for i in seed..x.len() { out[i] = if i == seed { x[i] } else { out[i - 1] + rate(i) * (x[i] - out[i - 1]) }; }
    out
}
/// (v - lo)/(hi - lo), 0 when the window is flat (a spread below 1e-12 is rounding noise, e.g. a regression residual).
fn norm(v: f64, lo: f64, hi: f64) -> f64 { if hi - lo <= 1e-12 * hi.abs().max(1.) { 0. } else { (v - lo) / (hi - lo) } }
fn aema_apirine(b: &Bars, prm: &Params) -> Output {
    let (k, hl, x) = (n(prm, "length"), n(prm, "hl_length"), &b.close);
    let (hh, ll, sma) = (ta::highest(&b.high, hl), ta::lowest(&b.low, hl), ta::sma(x, k));
    let m1 = 2. / (k as f64 + 1.);
    let s0 = k.max(hl) - 1;
    let mut out = vec![NA; x.len()];
    for i in s0..x.len() {
        out[i] = if i < s0 + k { sma[i] } else {
            let m = if hh[i] == ll[i] { 0. } else { ((x[i] - ll[i]) - (hh[i] - x[i])).abs() / (hh[i] - ll[i]) };
            out[i - 1] + m1 * (1. + m) * (x[i] - out[i - 1])
        };
    }
    Ok(vec![out])
}
fn cama(b: &Bars, prm: &Params) -> Output {
    let (k, x) = (n(prm, "length"), &b.close);
    let (hh, ll, effort) = (ta::highest(&b.high, k), ta::lowest(&b.low, k), ta::sum(&ta::tr(b, true), k));
    let mut out = x.clone();
    for i in k + 1..x.len() {
        let a = if effort[i] != 0. { (hh[i] - ll[i]) / effort[i] } else { 0. };
        out[i] = a * x[i] + (1. - a) * out[i - 1];
    }
    Ok(vec![out])
}
fn lr_adj_ema(b: &Bars, prm: &Params) -> Output {
    let (k, lr, hl, mult, x) = (n(prm, "length"), n(prm, "lr_length"), n(prm, "hl_length"), p(prm, "mult"), &b.close);
    let dist: Vec<f64> = ta::linreg(x, lr, 0).iter().zip(x).map(|(l, c)| (l - c).abs()).collect();
    let (hh, ll) = (ext(&dist, hl, true), ext(&dist, hl, false));
    let m1 = 2. / (k as f64 + 1.);
    Ok(vec![adaptive(x, (k.max(lr) + 1).max(lr + hl - 2), |i| m1 * (1. + mult * norm(dist[i], ll[i], hh[i])))])
}
fn tr_adj_ema(b: &Bars, prm: &Params) -> Output {
    let (k, tl, mult, x) = (n(prm, "length"), n(prm, "tr_length"), p(prm, "mult"), &b.close);
    let tr = ta::tr(b, false);
    let (hh, ll) = (ext(&tr, tl, true), ext(&tr, tl, false));
    let m1 = 2. / (k as f64 + 1.);
    Ok(vec![adaptive(x, k.max(tl) + 1, |i| m1 * (1. + mult * norm(tr[i], ll[i], hh[i])))])
}
fn rs_va_ema(b: &Bars, prm: &Params) -> Output {
    let (k, vs, mult, x, v) = (n(prm, "length"), n(prm, "vs_length"), p(prm, "mult"), &b.close, vol(b));
    let (m1, c1) = (2. / (k as f64 + 1.), 2. / (vs as f64 + 1.));
    let (mut apv, mut anv) = (0., 0.);
    let mut out = vec![NA; x.len()];
    for i in 0..x.len() {
        let (up, dn) = if i == 0 { (false, false) } else { (x[i] > x[i - 1], x[i] < x[i - 1]) };
        apv = c1 * if up { v[i] } else { 0. } + (1. - c1) * apv;
        anv = c1 * if dn { v[i] } else { 0. } + (1. - c1) * anv;
        let s = if apv + anv != 0. { (apv - anv).abs() / (apv + anv) } else { 0.00001 };
        let rate = m1 * (1. + s * mult);
        out[i] = rate * x[i] + (1. - rate) * if i == 0 { x[i] } else { out[i - 1] };
    }
    Ok(vec![out])
}
fn vama(b: &Bars, prm: &Params) -> Output {
    let (k, f, x, v) = (n(prm, "length"), p(prm, "factor"), &b.close, vol(b));
    let mut cum = 0.;
    // each bar's volume in increments of the running average volume up to that bar
    let r: Vec<f64> = v.iter().enumerate().map(|(j, &w)| { cum += w; let q = w / (cum / (j + 1) as f64 * f); if q.is_finite() { q } else { 0. } }).collect();
    let kf = k as f64;
    Ok(vec![(0..x.len()).map(|i| {
        let (mut sw, mut sv, mut last) = (0., 0., i);
        for j in (i.saturating_sub(10 * k - 1)..=i).rev() {
            sw += x[j] * r[j]; sv += r[j]; last = j;
            if sv >= kf { break; }
        }
        (sw - (sv - kf) * x[last]) / kf
    }).collect()])
}

pub const DEFS: &[IndicatorDef] = &[
    def("aema_apirine", "Adaptive EMA (Apirine)", CAT, &[len("length", "Length", 10.), len("hl_length", "High-low length", 10.)], &["aema"], P, NO,
        "m = |(close - lowest(low,hl_length)) - (highest(high,hl_length) - close)|/(highest - lowest) (0 when the range is 0); aema = aema[1] + 2/(length+1)*(1+m)*(close - aema[1]); the first length bars from bar max(length,hl_length)-1 are sma(close,length) (TradeStation's CurrentBar <= Periods) (Apirine, TASC Apr 2019 Traders' Tips EasyLanguage _AEMA; defaults 10/10 from it)", &[9], aema_apirine),
    def("cama", "Cong Adaptive Moving Average", CAT, &[len("length", "Length", 10.)], &["cama"], P, NO,
        "alpha = (highest(high,length) - lowest(low,length)) / sum(true range, length) (true range of bar 0 = high-low; alpha 0 when the sum is 0); cama = alpha*close + (1-alpha)*cama[1]; bars 0..length are the close (Cong, TASC May 2023 Traders' Tips, PineCoders 'TASC 2023.05 Cong Adaptive Moving Average')", &[0], cama),
    def("lr_adj_ema", "Linear-Regression-Adjusted EMA (Apirine)", CAT, &[len("length", "Length", 20.), len("lr_length", "Linear regression length", 20.), len("hl_length", "Normalisation length", 20.), real("mult", "Multiplier", 5., 0., 100., 0.1)], &["ema"], P, NO,
        "dist = |linreg(close,lr_length) - close|; d = (dist - lowest(dist,hl_length))/(highest - lowest) (0 when flat); ema = ema[1] + 2/(length+1)*(1 + mult*d)*(close - ema[1]), starting from the close on bar max(length+1, lr_length+1, lr_length+hl_length-2) (Apirine, TASC Aug 2022; PineCoders 'TASC 2022.09 LRAdj EMA', which uses one lookback for lr_length and hl_length; defaults declared)", &[38], lr_adj_ema),
    def("tr_adj_ema", "True-Range-Adjusted EMA (Apirine)", CAT, &[len("length", "Length", 20.), len("tr_length", "True range normalisation length", 20.), real("mult", "Multiplier", 5., 0., 100., 0.1)], &["ema"], P, NO,
        "d = (tr - lowest(tr,tr_length))/(highest(tr,tr_length) - lowest) (0 when flat); ema = ema[1] + 2/(length+1)*(1 + mult*d)*(close - ema[1]), starting from the close on bar max(length,tr_length)+1 (Apirine, TASC Jan 2023 Traders' Tips; PineCoders 'TASC 2023.01 TRAdj EMA'; defaults 20/20/5 from the article's chart)", &[21], tr_adj_ema),
    def("rs_va_ema", "Volume-Adjusted EMA (Apirine RS-VA-EMA)", CAT, &[len("length", "Length", 10.), len("vs_length", "Volume strength length", 10.), real("mult", "Multiplier", 10., 0., 100., 0.1)], &["ema"], P, V,
        "pv = ema0(close > close[1] ? volume : 0, vs_length), nv = ema0(close < close[1] ? volume : 0, vs_length) (2/(vs_length+1) recursions from 0); vs = |pv - nv|/(pv + nv) (0.00001 when both are 0); rate = 2/(length+1)*(1 + vs*mult); ema = rate*close + (1-rate)*ema[1], bar 0 = close (Apirine, TASC Feb 2022 / Oct 2022 Traders' Tips, PineCoders 'TASC 2022.10 RS VA EMA'; defaults 10/10/10)", &[0], rs_va_ema),
    def("vama", "Volume-Adjusted Moving Average (Arms)", CAT, &[len("length", "Length", 13.), real("factor", "Volume increment factor", 0.67, 0.01, 10., 0.01)], &["vama"], P, V,
        "r_j = volume_j/(factor * mean volume of bars 0..j) (causal running average as allanster's Pine; MetaStock averages the whole chart, which looks ahead); walking back from this bar (at most 10*length bars, stopping at bar 0) sum r_j and close_j*r_j until sum r >= length; vama = (sum close*r - (sum r - length)*close_last)/length, the excess taken from the last included bar (Arms; allanster's Pine takes it from the bar after) (MetaStock mov(,,VOL); TradingView 'VAMA Volume Adjusted Moving Average Function' by allanster)", &[0], vama),

    def("supersmoother", "Ehlers SuperSmoother Filter", CAT, &[int("length", "Length", 20., 2., 2000.), int("poles", "Poles (2 or 3)", 2., 2., 3.)], &["ss"], P, NO,
        "2-pole: a = exp(-1.414pi/length), b = 2a*cos(1.414pi/length), c2 = b, c3 = -a^2, c1 = 1-c2-c3; ss = c1*(close+close[1])/2 + c2*ss[1] + c3*ss[2]. 3-pole: a = exp(-pi/length), b = 2a*cos(1.738pi/length), c = a^2; ss = (1-d2-d3-d4)*close + d2*ss[1] + d3*ss[2] + d4*ss[3] with d2 = b+c, d3 = -(c+bc), d4 = c^2. The first 2 (3) bars are the close (pandas-ta ssf/ssf3 seed); constants as Ehlers' EasyLanguage (degrees), not pandas-ta's default radians-of-180 cosine (Ehlers, Cycle Analytics for Traders 2013)", &[0], supersmoother),
    def("butterworth", "Butterworth Filter (Ehlers)", CAT, &[int("length", "Length", 15., 2., 2000.), int("poles", "Poles (2 or 3)", 2., 2., 3.)], &["bw"], P, NO,
        "2-pole: a = exp(-1.414pi/length), b = 2a*cos(1.414pi/length); bw = (1-b+a^2)/4*(close + 2close[1] + close[2]) + b*bw[1] - a^2*bw[2], bars 0-1 = close. 3-pole: a = exp(-pi/length), b = 2a*cos(1.738pi/length), c = a^2; bw = (1-b+c)(1-c)/8*(close + 3close[1] + 3close[2] + close[3]) + (b+c)*bw[1] - (c+bc)*bw[2] + c^2*bw[3], bars 0-2 = close (Sierra Chart Butterworth Filter; Ehlers, Cybernetic Analysis 2004 fig. 13.4-13.7)", &[0], butterworth),
    def("ultimate_smoother", "Ehlers Ultimate Smoother", CAT, &[int("length", "Length", 20., 2., 2000.)], &["us"], P, NO,
        "a = exp(-1.414pi/length), c2 = 2a*cos(1.414pi/length), c3 = -a^2, c1 = (1+c2-c3)/4; us = (1-c1)*close + (2c1-c2)*close[1] - (c1+c3)*close[2] + c2*us[1] + c3*us[2]; bars 0-2 = close (Ehlers, TASC Apr 2024 Traders' Tips EasyLanguage: CurrentBar < 4)", &[0], ultimate_smoother),
    def("decycler", "Ehlers Simple Decycler", CAT, &[int("length", "High-pass cutoff period", 125., 4., 2000.)], &["decycler"], P, NO,
        "alpha = (cos w + sin w - 1)/cos w with w = 0.707*2pi/length; hp = (1-alpha/2)^2*(close - 2close[1] + close[2]) + 2(1-alpha)*hp[1] - (1-alpha)^2*hp[2], hp of bars 0-1 = 0 (EasyLanguage initial values); decycler = close - hp from bar 2 (Ehlers, TASC Sep 2015 Traders' Tips 'Simple Decycler'; the +/-0.5% hysteresis lines are decycler*1.005 and *0.995)", &[2], decycler),
    def("inst_trendline", "Ehlers Instantaneous Trendline", CAT, &[int("length", "Length", 28., 2., 2000.)], &["it", "trigger"], P, NO,
        "x = hl2, a = 2/(length+1); bars 2-5: it = (x + 2x[1] + x[2])/4; then it = (a - a^2/4)*x + a^2/2*x[1] - (a - 3a^2/4)*x[2] + 2(1-a)*it[1] - (1-a)^2*it[2]; trigger = 2*it - it[2] (Sierra Chart Instantaneous Trendline; Ehlers, Cybernetic Analysis 2004 fig. 3.4)", &[2, 4], inst_trendline),
    def("laguerre_filter", "Ehlers Laguerre Filter", CAT, &[real("gamma", "Gamma", 0.8, 0., 0.99, 0.01)], &["filt"], P, NO,
        "x = hl2; l0 = (1-g)*x + g*l0[1]; l1 = -g*l0 + l0[1] + g*l1[1]; l2 = -g*l1 + l1[1] + g*l2[1]; l3 = -g*l2 + l2[1] + g*l3[1]; filt = (l0 + 2l1 + 2l2 + l3)/6; all four seeded with x on bar 0 (declared; the EasyLanguage starts them at 0) (Sierra Chart Laguerre Filter; Ehlers, Cybernetic Analysis 2004 fig. 14.5)", &[0], laguerre_filter),
    def("rmf", "Ehlers Recursive Median Filter", CAT, &[int("length", "Low-pass period", 12., 5., 2000.)], &["rmf"], P, NO,
        "alpha = (cos w + sin w - 1)/cos w, w = 2pi/length (length >= 5 keeps alpha in (0,1]); rmf = alpha*median(close,5) + (1-alpha)*rmf[1], seeded with the first 5-bar median on bar 4 (declared; the EasyLanguage starts at 0) (Ehlers, TASC Mar 2018 Traders' Tips 'Recursive Median Filters')", &[4], rmf),
    def("dsma", "Deviation-Scaled Moving Average (Ehlers)", CAT, &[int("length", "Length", 40., 2., 2000.)], &["dsma"], P, NO,
        "zeros = close - close[2]; filt = SuperSmoother(zeros, length/2) = c1*(zeros+zeros[1])/2 + c2*filt[1] + c3*filt[2]; rms = sqrt(sum of the last length filt^2 / length); scaled = filt/rms (kept when rms = 0); alpha = |scaled|*5/length (no clamp, as Ehlers); dsma = alpha*close + (1-alpha)*dsma[1]. History before bar 0 counts as 0 (EasyLanguage initial values) and dsma starts from close[0] (Ehlers, TASC Jul 2018 Traders' Tips EasyLanguage EhlersDSMA)", &[0], dsma),
    def("distant_coef", "Ehlers Distant Coefficient Filter", CAT, &[int("length", "Length", 15., 2., 100.)], &["filt"], P, NO,
        "x = hl2; for c = 0..length-1: coef_c = sum_{k=1..length-1} (x[c] - x[c+k])^2; filt = sum(coef_c*x[c])/sum(coef_c); when sum(coef) = 0 the previous filt is kept (NaN before any); first value bar 2(length-1) (Ehlers 'Ehlers Filters' 2001, fig. 6 EasyLanguage; default 15 from that listing)", &[28], distant_coef),
    def("evwma", "Elastic Volume-Weighted Moving Average", CAT, &[len("length", "Length", 20.)], &["evwma"], P, V,
        "N = volume sum of the last length bars (bars before the first padded with volume[0], Sierra Chart default 'Use Cumulative Volume = No'); evwma = (1 - V/N)*evwma[1] + V*close/N; evwma[0] = close[0]; N = 0 keeps the previous value (declared; Sierra Chart writes 0) (Sierra Chart Moving Average - Elastic Volume Weighted; Fries, TASC 2001)", &[0], evwma),
    def("moma", "Move-Adjusted Moving Averages (MOMA/VOMOMA/WEVOMO)", CAT, &[len("length", "Length", 14.)], &["moma", "vomoma", "wevomo"], P, V,
        "m_i = |close_i - close_{i-1}|; moma = sum(close_i*m_i)/sum(m_i) over the last length bars (close when sum m = 0); vomoma = (moma + vwma(close,length))/2; wevomo = (moma + vwma + wma(close,length))/3; first value bar length (Sierra Chart Moving Average - Move-Adjusted / Volume Move-Adjusted / Weight Volume Move-Adjusted; Bisse, TASC Apr 2005)", &[14, 14, 14], moma),
    def("rainbow", "Rainbow Moving Average", CAT, &[len("length", "Step length", 2.)], &["ma1", "ma2", "ma3", "ma4", "ma5", "ma6", "ma7", "ma8", "ma9", "ma10", "avg"], P, NO,
        "ma1 = sma(close, length); ma_k = sma(ma_{k-1}, length), k = 2..10; avg = (ma1 + ... + ma10)/10, the equal-weight mean used by the Rainbow oscillator (Widner, TASC 1997; ChartIQ calculateRainbow; TOS RainbowAverage's 5,4,3,2,1,... weighting is not used)", &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 10], rainbow),
    def("coral", "Coral Trend", CAT, &[len("length", "Smoothing period", 21.), real("cd", "Constant D", 0.4, 0., 1., 0.01)], &["coral"], P, NO,
        "di = (length-1)/2 + 1, c1 = 2/(di+1); i1..i6 = cascaded EMAs i_k = c1*i_{k-1} + (1-c1)*i_k[1] with i_0 = close, all starting from 0 (LazyBear's nz seed: early bars climb from 0); coral = -cd^3*i6 + 3(cd^2+cd^3)*i5 - 3(2cd^2+cd+cd^3)*i4 + (3cd+1+cd^3+3cd^2)*i3 (TradingView public Pine 'Coral Trend Indicator [LazyBear]')", &[0], coral),

    def("fwma", "Fibonacci Weighted Moving Average", CAT, &[len("length", "Length", 10.)], &["fwma"], P, NO,
        "sum(w_i*close)/sum(w) over the last length bars, w = first length Fibonacci numbers 1,1,2,3,5,... ascending to the newest bar (pandas-ta fwma)", &[9], fwma),
    def("pwma", "Pascal's Weighted Moving Average", CAT, &[len("length", "Length", 10.)], &["pwma"], P, NO,
        "sum(w_i*close) over the last length bars, w = row length-1 of Pascal's triangle / 2^(length-1) (pandas-ta pwma)", &[9], pwma),
    def("sinwma", "Sine-Weighted Moving Average", CAT, &[len("length", "Length", 14.)], &["sinwma"], P, NO,
        "w_i = sin((i+1)*pi/(length+1)), i = 0..length-1 oldest->newest; sum(w_i*close)/sum(w) (pandas-ta sinwma; Sierra Chart's 5-bar version is length 5)", &[13], sinwma),
    def("hwma", "Holt-Winters Moving Average", CAT, &[real("na", "Level smoothing", 0.2, 0.01, 1., 0.01), real("nb", "Trend smoothing", 0.1, 0.01, 1., 0.01), real("nc", "Acceleration smoothing", 0.1, 0.01, 1., 0.01)], &["hwma"], P, NO,
        "F = (1-na)*(F[1]+V[1]+A[1]/2) + na*close; V = (1-nb)*(V[1]+A[1]) + nb*(F-F[1]); A = (1-nc)*A[1] + nc*(V-V[1]); hwma = F + V + A/2; seeds F = close[0], V = A = 0 (pandas-ta hwma)", &[0], hwma),
    def("t3", "Tillson T3", CAT, &[int("length", "Length", 5., 2., 2000.), real("vfactor", "Volume factor", 0.7, 0., 1., 0.01)], &["t3"], P, NO,
        "e1..e6 = six chained ema(., length) of close, each seeded with the SMA of its first length inputs; v = vfactor; T3 = -v^3*e6 + (3v^2+3v^3)*e5 + (-6v^2-3v-3v^3)*e4 + (1+3v+v^3+3v^2)*e3 (TA-Lib ta_T3.c, unstable period 0: first value at bar 6(length-1))", &[24], t3),
    def("trima", "Triangular Moving Average", CAT, &[int("length", "Length", 30., 2., 2000.)], &["trima"], P, NO,
        "odd length: sma(sma(close,(length+1)/2),(length+1)/2); even length: sma(sma(close,length/2),length/2+1) (TA-Lib TRIMA / MetaStock mov(,,TRI))", &[29], trima),
    def("zlema", "Zero-Lag EMA", CAT, &[len("length", "Length", 30.)], &["zlema"], P, NO,
        "lag = floor((length-1)/2); zlema = ema(2*close - close[lag], length), SMA seed (TA-Lib ta_ZLEMA.c, pandas-ta zlma, Tulip zlema)", &[43], zlema),
    def("swak_lowpass", "Swiss Army Knife Low-Pass (Gaussian / Butterworth)", CAT, &[int("length", "Length", 20., 2., 2000.), int("mode", "Mode (0 Gaussian, 1 Butterworth)", 0., 0., 1.)], &["filt"], P, NO,
        "b = 2.415*(1 - cos(2pi/length)); a = -b + sqrt(b^2 + 2b); Gaussian y = a^2*x + 2(1-a)*y[1] - (1-a)^2*y[2]; Butterworth y = a^2/4*(x + 2x[1] + x[2]) + 2(1-a)*y[1] - (1-a)^2*y[2]; x = close, all history slots seeded with the first bar (TA-Lib ta_SWAK_GAUSS.c / ta_SWAK_BUTTER.c, unstable period 0; Ehlers TASC Jan 2006)", &[0], swak_lowpass),
    def("frama", "Fractal Adaptive Moving Average", CAT, &[int("length", "Length", 16., 4., 2000.), int("variant", "Variant (0 Ehlers/TA-Lib halves, 1 MT5 N and 2N)", 0., 0., 1.)], &["frama"], P, NO,
        "Variant 0 (TA-Lib ta_FRAMA.c, odd length rounded down to even w): P = hl2; R1, R2 = high-low range of the newer / older half of the last w bars, R = range of all w; alpha = ((R1+R2)/R)^(-4.6/ln 2) when R1>0, R2>0 and R1+R2>R, else 1; frama = alpha*P + (1-alpha)*frama[1], seeded with P on bar w-1, first value bar w. Variant 1 (MT5 FrAMA.mq5): N = length, N1 = range of bars i-N+1..i / N, N2 = range of the N bars before / N, N3 = range of 2N bars / 2N; D = (ln(N1+N2) - ln N3)/ln 2; alpha = exp(-4.6(D-1)) (unclamped, as MT5); frama on close, seeded with close on bar 2N-2; zero-range windows take the close", &[16], frama),
    def("vidya", "Variable Index Dynamic Average (VIDYA)", CAT, &[len("length", "Length", 12.), len("cmo_length", "CMO length", 9.), int("variant", "Variant (0 CMO form, 1 stdev ratio)", 0., 0., 1.)], &["vidya"], P, NO,
        "a = 2/(length+1). Variant 0 (MT5 VIDYA.mq5, Chande): cmo = (U-D)/(U+D) of the last cmo_length close changes (0 when no change); vidya = close*a*|cmo| + vidya[1]*(1 - a*|cmo|); bars before length+cmo_length-1 hold the close (MT5 seed) and are not shown. Variant 1 (Tulip vidya.c): k = a*stdev(close,cmo_length)/stdev(close,length) (population; 0 when undefined), vidya = vidya[1] + k*(close - vidya[1]), seeded with close on bar length-2; needs cmo_length <= length", &[20], vidya),
    def("mama", "MESA Adaptive Moving Average (MAMA/FAMA)", CAT, &[real("fast_limit", "Fast limit", 0.5, 0.01, 0.99, 0.01), real("slow_limit", "Slow limit", 0.05, 0.01, 0.99, 0.01)], &["mama", "fama"], P, NO,
        "TA-Lib ta_MAMA.c port: smooth = (4x + 3x[1] + 2x[2] + x[3])/10 of close; even/odd Hilbert transform (0.0962, 0.5769, scaled by 0.075*period + 0.54) -> I1, Q1; phase = atan(Q1/I1); dphase = max(1, phase[1] - phase); alpha = dphase > 1 ? max(fast/dphase, slow) : fast; mama = alpha*close + (1-alpha)*mama[1]; fama = alpha/2*mama + (1-alpha/2)*fama[1]; mama/fama start at 0 on bar 12 as TA-Lib; first value bar 32 (TA-Lib lookback, unstable period 0)", &[32, 32], mama),
    def("ht_trendline", "Hilbert Transform Instantaneous Trendline", CAT, &[], &["trendline"], P, NO,
        "TA-Lib ta_HT_TRENDLINE.c port: Hilbert homodyne period (as MAMA) from bar 37, smoothPeriod = 0.33*period + 0.67*smoothPeriod[1]; avg = mean of the last int(smoothPeriod + 0.5) raw closes; trendline = (4*avg + 3*avg[1] + 2*avg[2] + avg[3])/10; first value bar 63 (TA-Lib lookback, unstable period 0)", &[63], ht_trendline),
];
/// (id, platform names, AmiBroker names): one row per id in DEFS.
pub const SOURCES: &[(&str, &str, &str)] = &[
    ("fwma", "pandas-ta: fwma", ""),
    ("pwma", "pandas-ta: pwma", ""),
    ("sinwma", "pandas-ta: sinwma; Sierra Chart: Moving Average - Sine-Wave Weighted", ""),
    ("hwma", "pandas-ta: hwma", ""),
    ("t3", "TA-Lib: T3; pandas-ta: t3; NinjaTrader 8: T3; Sierra Chart: Moving Average - T3", ""),
    ("trima", "TA-Lib: TRIMA; pandas-ta: trima; Tulip: trima; TA4J: TriangularMovingAverage; NinjaTrader 8: TMA; TradeStation: TriAverage; MultiCharts: TriAverage; thinkorswim: MovAvgTriangular; Sierra Chart: Moving Average - Triangular; MetaStock: Mov(TRI); cTrader: TriangularMovingAverage; Zerodha Kite (ChartIQ): Triangular MA; Chartink: TMA", ""),
    ("zlema", "TA-Lib: ZLEMA; pandas-ta: zlma; Tulip: zlema; TA4J: ZLEMAIndicator; NinjaTrader 8: ZLEMA; Sierra Chart: Moving Average - Zero Lag Exponential; Dhan: ZLEMA", ""),
    ("swak_lowpass", "TA-Lib: SWAK_GAUSS; TA-Lib: SWAK_BUTTER", ""),
    ("frama", "TA-Lib: FRAMA; MetaTrader 5: iFrAMA; cTrader: FractalAdaptiveMovingAverage", ""),
    ("vidya", "MetaTrader 5: iVIDyA; cTrader: Vidya; TA-Lib: VIDYA; pandas-ta: vidya; Tulip: vidya; TA4J: VIDYAIndicator; NinjaTrader 8: VMA; thinkorswim: VariableMA; Sierra Chart: Moving Average - Variable Index Dynamic; MetaStock: Mov(VAR); Zerodha Kite (ChartIQ): VIDYA", ""),
    ("mama", "TA-Lib: MAMA; pandas-ta: mama; NinjaTrader 8: MAMA; TradeStation: Mov Avg MAMA; Dhan: MAMA", ""),
    ("ht_trendline", "TA-Lib: HT_TRENDLINE; pandas-ta: ht_trendline", ""),
    ("supersmoother", "pandas-ta: ssf; pandas-ta: ssf3; TA-Lib: SSF; thinkorswim: EhlersSuperSmootherFilter; Sierra Chart: Super Smoother Filter", ""),
    ("butterworth", "Sierra Chart: Butterworth Filter", ""),
    ("ultimate_smoother", "thinkorswim: UltimateSmootherFilter", ""),
    ("decycler", "thinkorswim: EhlersSimpleDecycler", ""),
    ("inst_trendline", "Sierra Chart: Instantaneous Trendline", ""),
    ("laguerre_filter", "Sierra Chart: Laguerre Filter", ""),
    ("rmf", "thinkorswim: RecursiveMedianFilter", ""),
    ("dsma", "thinkorswim: DeviationScaledMovAvg", ""),
    ("distant_coef", "thinkorswim: EhlersDistantCoefficientFilter", ""),
    ("evwma", "Sierra Chart: Moving Average - Elastic Volume Weighted", ""),
    ("moma", "Sierra Chart: Moving Average - Move-Adjusted; Sierra Chart: Moving Average - Volume Move-Adjusted; Sierra Chart: Moving Average - Weight Volume Move-Adjusted", ""),
    ("rainbow", "Zerodha Kite (ChartIQ): Rainbow Moving Average; thinkorswim: RainbowAverage; MetaStock: Rainbow; cTrader: Rainbow", ""),
    ("coral", "TradingView community scripts (India): Coral Trend Indicator [LazyBear]", ""),
    ("aema_apirine", "thinkorswim: AdaptiveEMA", ""),
    ("cama", "thinkorswim: CongAdaptiveMovingAverage", ""),
    ("lr_adj_ema", "thinkorswim: LinearRegAdjEMA", ""),
    ("tr_adj_ema", "thinkorswim: TrueRangeAdjEMA", ""),
    ("rs_va_ema", "thinkorswim: RS-VA-EMA", ""),
    ("vama", "MetaStock: Mov(VOL); Dhan: VAMA; TradingView community scripts (India): VAMA Volume Adjusted Moving Average Function", ""),
];

#[cfg(test)]
mod tests {
    use crate::lab::testdata::{assert_series, closes, from_ohlc, go, walk};
    const N: f64 = f64::NAN;
    fn near(a: f64, b: f64, tol: f64) -> bool { (a - b).abs() <= tol }

    #[test] fn fwma_hand() { assert_series(&go("fwma", &closes(&[1., 2., 3.]), &[("length", 3.)])[0], &[N, N, 2.25]); }
    #[test] fn pwma_hand() { assert_series(&go("pwma", &closes(&[1., 2., 3.]), &[("length", 3.)])[0], &[N, N, 2.]); }
    #[test] fn sinwma_symmetric_on_ramp() { assert!(near(go("sinwma", &closes(&[1., 2., 3., 4., 5.]), &[("length", 5.)])[0][4], 3., 1e-12)); }
    #[test] fn hwma_constant() { assert!(go("hwma", &closes(&[7.; 20]), &[])[0].iter().all(|v| near(*v, 7., 1e-12))); }
    #[test] fn t3_constant_and_first() {
        let o = &go("t3", &closes(&[4.; 40]), &[])[0];
        assert!(o[23].is_nan() && near(o[24], 4., 1e-12) && near(o[39], 4., 1e-12));
    }
    #[test] fn trima_even_weights() { assert!(near(go("trima", &closes(&(1..=10).map(f64::from).collect::<Vec<_>>()), &[("length", 4.)])[0][9], 8.5, 1e-12)); }
    #[test] fn zlema_ramp_has_no_lag() {
        let o = &go("zlema", &closes(&(0..30).map(f64::from).collect::<Vec<_>>()), &[("length", 5.)])[0];
        assert!(o[5].is_nan() && (6..30).all(|i| near(o[i], i as f64, 1e-9)));
    }
    #[test] fn swak_constant() {
        for m in [0., 1.] { assert!(go("swak_lowpass", &closes(&[9.; 50]), &[("mode", m)])[0].iter().all(|v| near(*v, 9., 1e-9))); }
    }
    #[test] fn frama_flat_half_takes_price() {
        // the newest half is flat (R1 = 0): alpha = 1 and frama equals hl2
        let mut c: Vec<f64> = (0..8).map(|i| 10. + i as f64).collect();
        c.extend([20.; 4]);
        let o = &go("frama", &closes(&c), &[("length", 8.)])[0];
        assert!(o[7].is_nan() && near(o[11], 20., 1e-12));
        let m = &go("frama", &walk(80, 3, false), &[("variant", 1.)])[0];
        assert!(m[30].is_nan() && m[31].is_finite());
    }
    #[test] fn vidya_all_up_is_ema_step() {
        let c: Vec<f64> = (0..30).map(|i| 100. + i as f64).collect();
        let o = &go("vidya", &closes(&c), &[])[0];
        let a = 2. / 13.;
        assert!(o[19].is_nan() && near(o[20], c[20] * a + c[19] * (1. - a), 1e-9) && near(o[21], o[20] + a * (c[21] - o[20]), 1e-9));
        let t = &go("vidya", &closes(&[5.; 20]), &[("variant", 1.)])[0];
        assert!(t[9].is_nan() && t[10..].iter().all(|v| *v == 5.));
    }
    #[test] fn mama_constant_converges() {
        let o = go("mama", &closes(&[50.; 200]), &[]);
        assert!(o[0][31].is_nan() && o[1][31].is_nan() && o[0][32].is_finite());
        assert!(near(o[0][199], 50., 1e-6) && near(o[1][199], 50., 1e-6));
    }
    fn constant(id: &str, kv: &[(&str, f64)], from: usize) {
        let mut b = closes(&[12.5; 600]);
        b.volume = Some(vec![1000.; 600]);
        for (k, s) in go(id, &b, kv).iter().enumerate() { assert!(s[from..].iter().all(|v| near(*v, 12.5, 1e-9)), "{id}[{k}] {kv:?}"); }
    }
    #[test] fn ehlers_filters_pass_constants() {
        for poles in [2., 3.] { constant("supersmoother", &[("poles", poles)], 0); constant("butterworth", &[("poles", poles)], 0); }
        constant("ultimate_smoother", &[], 0);
        constant("decycler", &[], 2);
        constant("inst_trendline", &[], 4);
        constant("laguerre_filter", &[], 0);
        constant("rmf", &[], 4);
        constant("dsma", &[], 0);
        constant("evwma", &[], 0);
        constant("moma", &[], 14);
        constant("rainbow", &[], 10);
        constant("coral", &[], 500);
    }
    #[test] fn ultimate_smoother_ramp_zero_lag() {
        let o = &go("ultimate_smoother", &closes(&(0..400).map(f64::from).collect::<Vec<_>>()), &[])[0];
        assert!(near(o[399], 399., 1e-6));
    }
    #[test] fn laguerre_gamma0_is_fir() {
        let c = [1., 4., 2., 8., 5., 7.];
        let o = &go("laguerre_filter", &closes(&c), &[("gamma", 0.)])[0];
        assert!(near(o[5], (7. + 2. * 5. + 2. * 8. + 2.) / 6., 1e-12));
    }
    #[test] fn rmf_ignores_spike() {
        let mut c = [10.; 30]; c[15] = 99.;
        assert!(go("rmf", &closes(&c), &[])[0][4..].iter().all(|v| near(*v, 10., 1e-12)));
    }
    #[test] fn distant_coef_flat_nan_and_outlier_weight() {
        assert!(go("distant_coef", &closes(&[3.; 40]), &[])[0].iter().all(|v| v.is_nan()));
        // one outlier among flat bars gets all the weight
        let mut c = [3.; 5]; c[4] = 9.;
        assert!(near(go("distant_coef", &closes(&c), &[("length", 3.)])[0][4], 9., 1e-12));
    }
    #[test] fn evwma_constant_volume_is_rma() {
        let mut b = walk(60, 5, false);
        b.volume = Some(vec![500.; 60]);
        let o = &go("evwma", &b, &[("length", 4.)])[0];
        for t in 1..60 { assert!(near(o[t], o[t - 1] + (b.close[t] - o[t - 1]) / 4., 1e-9)); }
    }
    #[test] fn moma_equal_moves_is_sma() {
        let mut b = closes(&[1., 2., 1., 2., 1., 2., 1.]);
        b.volume = Some(vec![1.; 7]);
        let o = go("moma", &b, &[("length", 4.)]);
        assert_series(&o[0], &[N, N, N, N, 1.5, 1.5, 1.5]);
    }
    #[test] fn rainbow_step2_ma2_weights() {
        let c = [1., 5., 2., 8.];
        let o = go("rainbow", &closes(&c), &[]);
        assert!(near(o[1][3], (8. + 2. * 2. + 5.) / 4., 1e-12) && o[1][1].is_nan());
    }
    #[test] fn aema_mid_range_is_ema_step() {
        // close on the midpoint of the 3-bar range (high 12, low 8) from bar 5 on: m = 0
        let mut b = from_ohlc(&[(10., 12., 8., 10.); 8]);
        b.close = vec![9., 11., 10., 9., 11., 10., 10., 10.];
        let o = &go("aema_apirine", &b, &[("length", 3.), ("hl_length", 3.)])[0];
        assert!(o[1].is_nan() && near(o[2], 10., 1e-12) && near(o[4], 10., 1e-12));
        assert!(near(o[5], o[4] + 0.5 * (10. - o[4]), 1e-12));
    }
    #[test] fn cama_one_way_run_tracks_close() {
        // each bar spans exactly the previous close to its close: range over n = summed true range -> alpha = 1
        let rows: Vec<_> = (1..30).map(|i| (i as f64 - 1., i as f64, i as f64 - 1., i as f64)).collect();
        let b = from_ohlc(&rows);
        let o = &go("cama", &b, &[("length", 5.)])[0];
        assert!((0..b.len()).all(|i| near(o[i], b.close[i], 1e-12)));
    }
    #[test] fn lr_adj_ema_on_line_is_ema() {
        let c: Vec<f64> = (0..60).map(|i| 5. + 2. * i as f64).collect();
        let o = &go("lr_adj_ema", &closes(&c), &[])[0];
        assert!(o[37].is_nan() && near(o[38], c[38], 1e-9) && near(o[39], o[38] + 2. / 21. * (c[39] - o[38]), 1e-9));
    }
    #[test] fn tr_adj_ema_constant_tr_is_ema() {
        let c: Vec<f64> = (0..40).map(|i| 50. + i as f64).collect();
        let o = &go("tr_adj_ema", &closes(&c), &[])[0];
        assert!(o[20].is_nan() && near(o[21], c[21], 1e-12) && near(o[22], o[21] + 2. / 21. * (c[22] - o[21]), 1e-12));
    }
    #[test] fn rs_va_ema_hand() {
        let mut b = closes(&[10., 11.]);
        b.volume = Some(vec![100., 100.]);
        // vs_length 1: pv = 100, nv = 0 -> vs = 1 -> rate = 2/11*(1+10) = 2 -> 2*11 - 10 = 12
        assert_series(&go("rs_va_ema", &b, &[("vs_length", 1.)])[0], &[10., 12.]);
    }
    #[test] fn vama_unit_increments_is_sma() {
        let mut b = walk(40, 9, false);
        b.volume = Some(vec![300.; 40]);
        let o = &go("vama", &b, &[("length", 5.), ("factor", 1.)])[0];
        for i in 4..40 { assert!(near(o[i], b.close[i - 4..=i].iter().sum::<f64>() / 5., 1e-9)); }
    }
    /// Values from an independent line-by-line Python transcription of TA-Lib ta_MAMA.c / ta_HT_TRENDLINE.c on the same series.
    #[test] fn mama_ht_match_talib_transcription() {
        use std::f64::consts::PI;
        let c: Vec<f64> = (0..300).map(|t| { let t = t as f64; 100. + 10. * (2. * PI * t / 17.).sin() + 3. * (2. * PI * t / 7.3).sin() + 0.05 * t }).collect();
        let b = closes(&c);
        let m = go("mama", &b, &[]);
        for (i, mv, fv) in [(32, 96.89715227199916, 92.13901894752507), (50, 99.45276322595421, 100.25704634138079), (299, 118.33521149664165, 115.5612354751733)] {
            assert!(near(m[0][i], mv, 1e-9) && near(m[1][i], fv, 1e-9), "mama {i}: {} {}", m[0][i], m[1][i]);
        }
        let h = &go("ht_trendline", &b, &[])[0];
        for (i, v) in [(63, 102.67365345004933), (100, 104.58370721280119), (299, 114.25429595182531)] { assert!(near(h[i], v, 1e-9), "ht {i}: {}", h[i]); }
    }
    #[test] fn ht_trendline_constant_and_first() {
        let o = &go("ht_trendline", &closes(&[30.; 120]), &[])[0];
        assert!(o[62].is_nan() && near(o[63], 30., 1e-9) && near(o[119], 30., 1e-9));
    }
}
