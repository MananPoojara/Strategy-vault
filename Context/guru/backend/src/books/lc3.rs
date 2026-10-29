//! Lane C (29 Oct 2026) shared helpers for the single-author modules cfwtab, awcpf, bsichi, ggcharts, astk3.
use crate::connors::{Ctx, Leg, Trade};

/// One position at a time; an opposite entry reverses, a same-direction entry is ignored.
pub struct Pos { pub out: Vec<Trade>, cur: Option<(f64, usize, f64, &'static str)> }

impl Pos {
    pub fn new() -> Self { Pos { out: vec![], cur: None } }
    pub fn side(&self) -> f64 { self.cur.map_or(0., |c| c.0) }
    pub fn entry_bar(&self) -> Option<usize> { self.cur.map(|c| c.1) }
    pub fn exit(&mut self, i: usize, px: f64, why: &str) {
        if let Some((d, ei, ep, k)) = self.cur.take() {
            self.out.push(Trade { dir: d, legs: vec![Leg { i: ei, px: ep, w: 1. }], exit_i: i, exit_px: px, reason: why.into(), entry_kind: k, signal_i: i, open: false, exits: vec![] });
        }
    }
    /// Returns true when a position was opened.
    pub fn enter(&mut self, d: f64, i: usize, px: f64, kind: &'static str) -> bool {
        if self.side() == d { return false; }
        if self.side() != 0. { self.exit(i, px, "reversed"); }
        self.cur = Some((d, i, px, kind));
        true
    }
    pub fn done(mut self, to: usize, c_to: f64) -> Vec<Trade> {
        if let Some((d, ei, ep, k)) = self.cur.take() {
            self.out.push(Trade { dir: d, legs: vec![Leg { i: ei, px: ep, w: 1. }], exit_i: to, exit_px: c_to, reason: "open".into(), entry_kind: k, signal_i: to, open: true, exits: vec![] });
        }
        self.out
    }
}

/// Simple average of the last n values (NaN until n values exist).
pub fn sma(v: &[f64], n: usize) -> Vec<f64> {
    let mut out = vec![f64::NAN; v.len()];
    for i in 0..v.len() {
        if i + 1 >= n && n > 0 {
            let w = &v[i + 1 - n..=i];
            if w.iter().all(|x| x.is_finite()) { out[i] = w.iter().sum::<f64>() / n as f64; }
        }
    }
    out
}

/// Exponential average, alpha 2/(n+1), seeded with the simple average of the first n finite values.
pub fn ema(v: &[f64], n: usize) -> Vec<f64> {
    let mut out = vec![f64::NAN; v.len()];
    let Some(s) = v.iter().position(|x| x.is_finite()) else { return out };
    if s + n > v.len() || n == 0 { return out; }
    let a = 2. / (n as f64 + 1.);
    let mut e = v[s..s + n].iter().sum::<f64>() / n as f64;
    out[s + n - 1] = e;
    for i in s + n..v.len() { e += a * (v[i] - e); out[i] = e; }
    out
}

/// Highest / lowest of v over bars a..=b.
pub fn hi(v: &[f64], a: usize, b: usize) -> f64 { v[a..=b].iter().copied().fold(f64::NEG_INFINITY, f64::max) }
pub fn lo(v: &[f64], a: usize, b: usize) -> f64 { v[a..=b].iter().copied().fold(f64::INFINITY, f64::min) }

/// TradeStation daily-bar path: the extreme nearer the open is reached first (ties: the high).
pub fn high_first(o: f64, h: f64, l: f64) -> bool { h - o <= o - l }

/// Monday-based week number of a YYYY-MM-DD date.
pub fn week_no(date: &str) -> i64 {
    let (y, m, d): (i64, i64, i64) = (date[0..4].parse().unwrap_or(1970), date[5..7].parse().unwrap_or(1), date[8..10].parse().unwrap_or(1));
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * (m + if m > 2 { -3 } else { 9 }) + 2) / 5 + d - 1;
    let dn = era * 146_097 + (yoe * 365 + yoe / 4 - yoe / 100 + doy) - 719_468;
    (dn + 3).div_euclid(7)
}

/// Bars built from the daily series up to `to`: each day (weekly = false) or each Monday-based calendar week.
/// (first day, last day, open, high, low, close)
pub fn bars(x: &Ctx, to: usize, weekly: bool) -> Vec<(usize, usize, f64, f64, f64, f64)> {
    let mut v: Vec<(usize, usize, f64, f64, f64, f64)> = vec![];
    for i in 0..=to {
        let same = weekly && v.last().is_some_and(|b| week_no(&x.r[b.1].date) == week_no(&x.r[i].date));
        if same {
            let b = v.last_mut().unwrap();
            b.1 = i; b.3 = b.3.max(x.h[i]); b.4 = b.4.min(x.l[i]); b.5 = x.c[i];
        } else {
            v.push((i, i, x.o[i], x.h[i], x.l[i], x.c[i]));
        }
    }
    v
}

#[cfg(test)]
mod tests {
    #[test]
    fn helpers() {
        assert_eq!(super::week_no("2026-10-26"), super::week_no("2026-10-30"));
        assert_ne!(super::week_no("2026-10-25"), super::week_no("2026-10-26"));
        let e = super::ema(&[1., 2., 3., 4.], 3);
        assert!(e[1].is_nan() && (e[2] - 2.).abs() < 1e-12 && (e[3] - 3.).abs() < 1e-12);
    }
}
