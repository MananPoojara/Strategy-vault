//! Lane B1 (16 Nov 2026, R4 #339): MetaStock built-ins that the NIKSAN records name but do not define (DMI, IMI, PFE,
//! ProjOsc, RVI, VHF), rebuilt from their published definitions (authors/relax-339/B1/w/DEFINITIONS.md). Signals for the
//! ten records below; nkms::run and nkms2::run call `signals`. Every reading the definitions leave open is listed in `NOTE`.
//! Not a book module (no `pub fn catalogue`).
use crate::books::msws::Sig;
use crate::connors::Ctx;

const NAN: f64 = f64::NAN;
pub const IDS: &[&str] = &["nkms_dynamic_momentum_index_1", "nkms_dynamic_momentum_index_2", "nkms_dynamic_momentum_index_3",
    "nkms2_intraday_momentum_index", "nkms2_polarized_fractal_efficiency", "nkms2_projection_oscillator_1", "nkms2_projection_oscillator_2",
    "nkms2_relative_volatility_index_1", "nkms2_relative_volatility_index_2", "nkms2_vertical_horizontal_filter"];
pub fn has(id: &str) -> bool { IDS.contains(&id) }

/// Card text: the unstated details of each record, each a labelled reading (R4 #297 as relaxed by #339).
pub fn note(id: &str) -> &'static str {
    match id {
        "nkms_dynamic_momentum_index_1" | "nkms_dynamic_momentum_index_2" | "nkms_dynamic_momentum_index_3" =>
            "Unverified declared application; adapted: 4 unstated details (R4 #339). DMI() is MetaStock's built-in Dynamic Momentum Index (Chande & Kroll); its formula is rebuilt from the published definition (MetaStock forum / Chande-Kroll: an RSI whose length varies with a 5-bar standard deviation against its 10-bar average; bounds 3 to 30), see authors/relax-339/B1/w/DEFINITIONS.md. Unstated in the source: (1) base length 14 divided by the volatility index, truncated; (2) the length is clamped to 3..30; (3) the RSI is the simple sums of up and down changes over that variable length (no Wilder smoothing); (4) population standard deviation. Trades fill at the signal-bar close.",
        "nkms2_intraday_momentum_index" =>
            "Unverified declared application; adapted: 2 unstated details (R4 #339). IMI(14) is Chande's Intraday Momentum Index: 100 x sum of up candle bodies (close - open) / (sum of up bodies + sum of down bodies) over 14 bars (published definition, w/DEFINITIONS.md). Unstated in the source: (1) simple sums over the 14 bars, no smoothing; (2) a window with no body at all is undefined. Reads the open, so results start where real opens begin.",
        "nkms2_polarized_fractal_efficiency" =>
            "Unverified declared application; adapted: 4 unstated details (R4 #339). PFE(C,34,3) is Hannula's Polarized Fractal Efficiency: 100 x sign(C - C[33]) x sqrt((C - C[33])^2 + 33^2) / sum over 33 steps of sqrt(1 + (C - C[-1])^2), smoothed by a 3-bar EMA. Unstated in the source: (1) the n-1 = 33 step convention; (2) EMA seeded with the first PFE value; (3) a zero change over the window counts as negative; (4) Lowest()/Highest() without a period are read as the expanding minimum / maximum since the first PFE value (no look-ahead).",
        "nkms2_projection_oscillator_1" | "nkms2_projection_oscillator_2" =>
            "Unverified declared application; adapted: 4 unstated details (R4 #339). ProjOsc(n,3) is Widner's Projection Oscillator: 100 x (C - lower band) / (upper band - lower band); the bands slide each bar's high / low of the last n bars forward along the regression slope of the highs / lows (published definition, w/DEFINITIONS.md). Unstated in the source: (1) the bar offset j runs 0..n-1 over the n-bar window; (2) slopes are least-squares over the same n bars; (3) the second argument 3 is a simple 3-bar smoothing of the oscillator; (4) a zero-width band is undefined. Reads H and L, so results start where real highs and lows begin.",
        "nkms2_relative_volatility_index_1" | "nkms2_relative_volatility_index_2" =>
            "Unverified declared application; adapted: 3 unstated details (R4 #339). RVI(14) is Dorsey's Relative Volatility Index: the 10-bar standard deviation of the close is the up value on up-close bars and the down value on down-close bars, each Wilder-smoothed over 14 (13/14 weighting), RVI = 100 x U / (U + D) (MetaStock formula collection). Unstated in the source: (1) the 1993 close-based version, not the 1995 high/low version; (2) Wilder smoothing seeded with the mean of the first 14 values; (3) population standard deviation.",
        "nkms2_vertical_horizontal_filter" =>
            "Unverified declared application; adapted: 2 unstated details (R4 #339). VHF(C,21) = (highest close - lowest close over 21 bars) / sum of 21 absolute close changes (White; published definition). Unstated in the source: (1) the 21-bar window includes the current bar with 21 changes; (2) LinRegSlope(C,21) is the least-squares slope of the 21 closes against time.",
        _ => "",
    }
}

fn fin(v: f64) -> bool { v.is_finite() }
fn tr(v: bool) -> bool { v }
fn cross(a: &[f64], b: &[f64]) -> Vec<bool> {
    (0..a.len()).map(|i| i >= 1 && fin(a[i]) && fin(b[i]) && fin(a[i - 1]) && fin(b[i - 1]) && a[i] > b[i] && a[i - 1] <= b[i - 1]).collect()
}
fn kv(n: usize, v: f64) -> Vec<f64> { vec![v; n] }
fn or(a: &[bool], b: &[bool]) -> Vec<bool> { a.iter().zip(b).map(|(x, y)| *x || *y).collect() }
fn and(a: &[bool], b: &[bool]) -> Vec<bool> { a.iter().zip(b).map(|(x, y)| *x && *y).collect() }
fn gt0(v: &[f64], up: bool) -> Vec<bool> { v.iter().map(|&x| fin(x) && if up { x > 0.0 } else { x < 0.0 }).collect() }
fn sma(v: &[f64], n: usize) -> Vec<f64> {
    (0..v.len()).map(|i| if i + 1 < n { NAN } else { let w = &v[i + 1 - n..=i]; if w.iter().all(|x| fin(*x)) { w.iter().sum::<f64>() / n as f64 } else { NAN } }).collect()
}
fn pstd(v: &[f64], n: usize) -> Vec<f64> {
    (0..v.len()).map(|i| if i + 1 < n { NAN } else { let w = &v[i + 1 - n..=i]; let m = w.iter().sum::<f64>() / n as f64; (w.iter().map(|x| (x - m) * (x - m)).sum::<f64>() / n as f64).sqrt() }).collect()
}
/// Least-squares slope of the last n values against 0..n-1.
fn slope(v: &[f64], n: usize) -> Vec<f64> {
    let xm = (n as f64 - 1.0) / 2.0;
    let sxx: f64 = (0..n).map(|j| (j as f64 - xm).powi(2)).sum();
    (0..v.len()).map(|i| if i + 1 < n { NAN } else {
        let w = &v[i + 1 - n..=i]; let ym = w.iter().sum::<f64>() / n as f64;
        (0..n).map(|j| (j as f64 - xm) * (w[j] - ym)).sum::<f64>() / sxx }).collect()
}

pub fn imi(o: &[f64], c: &[f64], n: usize) -> Vec<f64> {
    (0..c.len()).map(|i| if i + 1 < n { NAN } else {
        let (mut u, mut d) = (0.0, 0.0);
        for j in i + 1 - n..=i { if c[j] > o[j] { u += c[j] - o[j]; } else if c[j] < o[j] { d += o[j] - c[j]; } }
        if u + d > 0.0 { 100.0 * u / (u + d) } else { NAN } }).collect()
}
pub fn dmi(c: &[f64]) -> Vec<f64> {
    let n = c.len();
    let sd = pstd(c, 5);
    let m = sma(&sd, 10);
    (0..n).map(|i| {
        if !fin(m[i]) || m[i] == 0.0 { return NAN; }
        let vi = sd[i] / m[i];
        if vi <= 0.0 { return NAN; }
        let td = ((14.0 / vi).trunc() as i64).clamp(3, 30) as usize;
        if i < td { return NAN; }
        let (mut u, mut d) = (0.0, 0.0);
        for j in i + 1 - td..=i { let ch = c[j] - c[j - 1]; if ch > 0.0 { u += ch } else { d -= ch } }
        if u + d > 0.0 { 100.0 * u / (u + d) } else { NAN }
    }).collect()
}
pub fn pfe(c: &[f64], n: usize, s: usize) -> Vec<f64> {
    let len = c.len();
    let raw: Vec<f64> = (0..len).map(|i| if i + 1 < n { NAN } else {
        let dc = c[i] - c[i + 1 - n];
        let den: f64 = (i + 2 - n..=i).map(|j| (1.0 + (c[j] - c[j - 1]).powi(2)).sqrt()).sum();
        let num = (dc * dc + ((n - 1) as f64).powi(2)).sqrt();
        (if dc > 0.0 { 100.0 } else { -100.0 }) * num / den }).collect();
    let a = 2.0 / (s as f64 + 1.0);
    let mut out = vec![NAN; len]; let mut prev = NAN;
    for i in 0..len { if fin(raw[i]) { prev = if fin(prev) { prev + a * (raw[i] - prev) } else { raw[i] }; out[i] = prev; } }
    out
}
pub fn projosc(h: &[f64], l: &[f64], c: &[f64], n: usize, s: usize) -> Vec<f64> {
    let (sh, sl) = (slope(h, n), slope(l, n));
    let raw: Vec<f64> = (0..c.len()).map(|i| if i + 1 < n { NAN } else {
        let (mut up, mut lo) = (f64::MIN, f64::MAX);
        for j in 0..n { up = up.max(h[i - j] + j as f64 * sh[i]); lo = lo.min(l[i - j] + j as f64 * sl[i]); }
        if up > lo { 100.0 * (c[i] - lo) / (up - lo) } else { NAN } }).collect();
    sma(&raw, s)
}
pub fn rvi(c: &[f64], sdn: usize, n: usize) -> Vec<f64> {
    let len = c.len();
    let sd = pstd(c, sdn);
    let (mut u, mut d) = (vec![NAN; len], vec![NAN; len]);
    // Wilder smoothing seeded with the mean of the first n defined values
    let (mut su, mut sdd, mut cnt) = (0.0, 0.0, 0usize);
    for i in 1..len {
        if !fin(sd[i]) { continue; }
        let (xu, xd) = (if c[i] > c[i - 1] { sd[i] } else { 0.0 }, if c[i] < c[i - 1] { sd[i] } else { 0.0 });
        cnt += 1;
        if cnt < n { su += xu; sdd += xd; }
        else if cnt == n { su += xu; sdd += xd; u[i] = su / n as f64; d[i] = sdd / n as f64; }
        else { u[i] = (u[i - 1] * (n as f64 - 1.0) + xu) / n as f64; d[i] = (d[i - 1] * (n as f64 - 1.0) + xd) / n as f64; }
    }
    (0..len).map(|i| if fin(u[i]) && u[i] + d[i] > 0.0 { 100.0 * u[i] / (u[i] + d[i]) } else { NAN }).collect()
}
pub fn vhf(c: &[f64], n: usize) -> Vec<f64> {
    (0..c.len()).map(|i| if i < n { NAN } else {
        let w = &c[i + 1 - n..=i];
        let (hi, lo) = (w.iter().copied().fold(f64::MIN, f64::max), w.iter().copied().fold(f64::MAX, f64::min));
        let s: f64 = (i + 1 - n..=i).map(|j| (c[j] - c[j - 1]).abs()).sum();
        if s > 0.0 { (hi - lo) / s } else { NAN } }).collect()
}

fn sig(el: Vec<bool>, xl: Vec<bool>, es: Vec<bool>, xs: Vec<bool>) -> Sig { Sig { el, xl, es, xs } }
/// Level crossings: Cross(a, level) / Cross(level, a) for a constant level.
fn up(a: &[f64], lv: f64) -> Vec<bool> { cross(a, &kv(a.len(), lv)) }
fn dn(a: &[f64], lv: f64) -> Vec<bool> { cross(&kv(a.len(), lv), a) }

pub fn signals(id: &str, x: &Ctx) -> Option<Sig> {
    let (n, c) = (x.c.len(), &x.c);
    Some(match id {
        "nkms2_intraday_momentum_index" => { let v = imi(&x.o, c, 14); sig(up(&v, 30.0), dn(&v, 70.0), dn(&v, 70.0), up(&v, 30.0)) }
        "nkms_dynamic_momentum_index_1" => {
            let v = dmi(c); let m5 = sma(c, 5);
            let (a, b) = (dn(&v, 30.0), up(&v, 70.0)); // Cross(30,DMI), Cross(DMI,70)
            let bars_since = |cr: &[bool], k: usize| -> Vec<bool> { let mut last = None; (0..n).map(|i| { if cr[i] { last = Some(i); } last.is_some_and(|j| i - j == k) }).collect() };
            let roc3: Vec<f64> = (0..n).map(|i| if i >= 3 { (c[i] / c[i - 3] - 1.0) * 100.0 } else { NAN }).collect();
            let xl = or(&cross(&m5, c), &and(&bars_since(&a, 3), &gt0(&roc3, false)));
            let xs = or(&cross(c, &m5), &and(&bars_since(&b, 3), &gt0(&roc3, true)));
            sig(a, xl, b, xs)
        }
        "nkms_dynamic_momentum_index_2" => { let v = dmi(c); sig(up(&v, 20.0), dn(&v, 80.0), dn(&v, 80.0), up(&v, 20.0)) }
        "nkms_dynamic_momentum_index_3" => {
            let v = dmi(c); let m200 = sma(c, 200);
            let (eu, ed) = (up(&v, 30.0), dn(&v, 80.0)); // Cross(DMI,30), Cross(80,DMI)
            let vw = |cr: &[bool]| -> Vec<f64> { let mut last = NAN; (0..n).map(|i| { if cr[i] { last = c[i]; } last }).collect() };
            let (vu, vd) = (vw(&eu), vw(&ed));
            let xl2: Vec<bool> = (0..n).map(|i| fin(vu[i]) && fin(m200[i]) && vu[i] > m200[i] && cross(&m200, c)[i]).collect();
            let xs2: Vec<bool> = (0..n).map(|i| fin(vd[i]) && fin(m200[i]) && vd[i] < m200[i] && cross(c, &m200)[i]).collect();
            sig(eu.clone(), or(&ed, &xl2), ed.clone(), or(&eu, &xs2))
        }
        "nkms2_polarized_fractal_efficiency" => {
            let p = pfe(c, 34, 3);
            let (mut lo, mut hi) = (f64::MAX, f64::MIN);
            let v1: Vec<f64> = p.iter().map(|&q| { if !fin(q) { return NAN; } lo = lo.min(q); hi = hi.max(q); if hi > lo { (q - lo) / (hi - lo) } else { NAN } }).collect();
            let (a, b) = (up(&v1, 0.5), dn(&v1, 0.5));
            sig(a.clone(), or(&b, &up(&v1, 0.8)), b, or(&a, &dn(&v1, 0.2)))
        }
        "nkms2_projection_oscillator_1" => {
            let v = projosc(&x.h, &x.l, c, 8, 3); let t = sma(&v, 3);
            let (a, b) = (cross(&v, &t), cross(&t, &v));
            sig(a.clone(), b.clone(), b, a)
        }
        "nkms2_projection_oscillator_2" => { let v = projosc(&x.h, &x.l, c, 14, 3); sig(up(&v, 30.0), dn(&v, 70.0), dn(&v, 70.0), up(&v, 30.0)) }
        "nkms2_relative_volatility_index_1" => { let v = rvi(c, 10, 14); sig(up(&v, 30.0), dn(&v, 70.0), dn(&v, 70.0), up(&v, 30.0)) }
        "nkms2_relative_volatility_index_2" => { let v = rvi(c, 10, 14); sig(up(&v, 40.0), dn(&v, 60.0), dn(&v, 60.0), up(&v, 40.0)) }
        "nkms2_vertical_horizontal_filter" => {
            let v = vhf(c, 21); let s = slope(c, 21);
            let z = kv(n, 0.0);
            let (a, b) = (up(&v, 0.3), dn(&v, 0.75));
            sig(and(&a, &gt0(&s, true)), or(&b, &cross(&z, &s)), and(&a, &gt0(&s, false)), or(&b, &cross(&s, &z)))
        }
        _ => return None,
    })
}
#[allow(dead_code)]
fn _t(b: bool) -> bool { tr(b) }
