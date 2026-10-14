use super::bars::Bars;
pub fn walk(n: usize, seed: u64, with_volume: bool) -> Bars {
    let mut s = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
    let mut next = || { s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407); ((s >> 11) as f64) / ((1u64 << 53) as f64) };
    let mut b = Bars::default();
    let mut c = 100.0;
    for i in 0..n {
        let o = c * (1. + (next() - 0.5) * 0.01);
        let cl = o * (1. + (next() - 0.5) * 0.04);
        let h = o.max(cl) * (1. + next() * 0.01);
        let l = o.min(cl) * (1. - next() * 0.01);
        b.date.push(format!("2020-{:02}-{:02}", 1 + (i / 28) % 12, 1 + i % 28));
        b.open.push(o); b.high.push(h); b.low.push(l); b.close.push(cl); b.end.push(i);
        c = cl;
    }
    if with_volume { b.volume = Some((0..n).map(|_| 1000. + (next() * 1000.).round()).collect()); }
    b
}
pub fn from_ohlc(rows: &[(f64, f64, f64, f64)]) -> Bars {
    let mut b = Bars::default();
    for (i, &(o, h, l, c)) in rows.iter().enumerate() {
        b.date.push(format!("2024-01-{:02}", i + 1));
        b.open.push(o); b.high.push(h); b.low.push(l); b.close.push(c); b.end.push(i);
    }
    b
}
pub fn close_eq(a: f64, b: f64) -> bool {
    (a.is_nan() && b.is_nan()) || (a - b).abs() <= 1e-9 * a.abs().max(b.abs()).max(1.)
}
pub fn assert_series(got: &[f64], want: &[f64]) {
    assert_eq!(got.len(), want.len(), "length");
    for (i, (g, w)) in got.iter().zip(want).enumerate() {
        assert!(close_eq(*g, *w), "index {i}: got {g}, want {w}");
    }
}
