//! float32 array helpers in the style of AmiBroker's AFL functions (std only). `NaN` is AmiBroker's Null.
//! Starting points, NOT verified one by one: when a team sheet disagrees, check the semantics (Null edges, seeds) against the AFL.
#![allow(dead_code)]

/// `Ref(a, k)` with k < 0 = k bars back (AmiBroker style: `ref_(a, -1)[i] == a[i-1]`); bars out of range are Null.
pub fn ref_(a: &[f32], k: i32) -> Vec<f32> {
    let n = a.len() as i64;
    (0..n).map(|i| { let j = i + k as i64; if j >= 0 && j < n { a[j as usize] } else { f32::NAN } }).collect()
}

/// `MA(a, n)`: simple average of the last n bars; the first n-1 bars are Null.
pub fn ma(a: &[f32], n: usize) -> Vec<f32> {
    let mut out = vec![f32::NAN; a.len()];
    let mut sum = 0f64;
    for i in 0..a.len() {
        sum += a[i] as f64;
        if i >= n { sum -= a[i - n] as f64; }
        if i + 1 >= n { out[i] = (sum / n as f64) as f32; }
    }
    out
}

/// `EMA(a, n)`: alpha = 2 / (n + 1), seeded with the first value.
pub fn ema(a: &[f32], n: usize) -> Vec<f32> {
    let al = 2.0f32 / (n as f32 + 1.0);
    let mut out = vec![f32::NAN; a.len()];
    for i in 0..a.len() {
        out[i] = if i == 0 || out[i - 1].is_nan() { a[i] } else { al * a[i] + (1.0 - al) * out[i - 1] };
    }
    out
}

/// `HHV(a, n)`: highest of the last n bars, including the bar (fewer bars at the start: the window is what exists).
pub fn hhv(a: &[f32], n: usize) -> Vec<f32> {
    (0..a.len()).map(|i| a[i.saturating_sub(n.saturating_sub(1))..=i].iter().copied().fold(f32::MIN, f32::max)).collect()
}

/// `LLV(a, n)`: lowest of the last n bars, including the bar.
pub fn llv(a: &[f32], n: usize) -> Vec<f32> {
    (0..a.len()).map(|i| a[i.saturating_sub(n.saturating_sub(1))..=i].iter().copied().fold(f32::MAX, f32::min)).collect()
}

/// `Sum(a, n)`.
pub fn sum(a: &[f32], n: usize) -> Vec<f32> {
    let mut out = vec![f32::NAN; a.len()];
    let mut s = 0f64;
    for i in 0..a.len() {
        s += a[i] as f64;
        if i >= n { s -= a[i - n] as f64; }
        if i + 1 >= n { out[i] = s as f32; }
    }
    out
}

/// `ROC(a, n)` in percent: 100 * (a / Ref(a, -n) - 1); the first n bars are Null.
pub fn roc(a: &[f32], n: usize) -> Vec<f32> {
    (0..a.len()).map(|i| if i >= n && a[i - n] != 0.0 { 100.0 * (a[i] / a[i - n] - 1.0) } else { f32::NAN }).collect()
}

/// True range: max(H, Ref(C,-1)) - min(L, Ref(C,-1)); the first bar is H - L.
pub fn tr(h: &[f32], l: &[f32], c: &[f32]) -> Vec<f32> {
    (0..h.len()).map(|i| if i == 0 { h[i] - l[i] } else { h[i].max(c[i - 1]) - l[i].min(c[i - 1]) }).collect()
}

/// `ATR(n)`: Wilder smoothing of the true range (seed: the simple average of the first n true ranges). VERIFY against the AFL.
pub fn atr(h: &[f32], l: &[f32], c: &[f32], n: usize) -> Vec<f32> {
    let t = tr(h, l, c);
    let mut out = vec![f32::NAN; t.len()];
    if t.len() < n || n == 0 { return out; }
    let seed: f32 = t[..n].iter().sum::<f32>() / n as f32;
    out[n - 1] = seed;
    for i in n..t.len() { out[i] = (out[i - 1] * (n as f32 - 1.0) + t[i]) / n as f32; }
    out
}

/// Calendar helpers on yyyymmdd dates.
pub fn year(d: i32) -> i32 { d / 10000 }
pub fn month(d: i32) -> i32 { d / 100 % 100 }
pub fn day(d: i32) -> i32 { d % 100 }
