use super::bars::Bars;
use super::catalogue::{get, run};
use super::resample::{civil, day_number};
use super::rules::{parse_spec, LabError};
use super::series::{compile, Cache, Frames};
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
        b.date.push(civil(day_number("2020-01-01") + i as i64));
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
/// Eight OHLCV bars used by the Phase 6 family tests (2024-01-01 is a Monday; index 7 starts week 2).
pub fn d1() -> Bars {
    let mut b = from_ohlc(&[(10., 11., 9., 10.5), (10.5, 12., 10., 11.5), (11.5, 12.5, 11., 12.), (12., 12.2, 10.8, 11.),
        (11., 11.5, 10., 10.2), (10.2, 11., 9.5, 10.8), (10.8, 12., 10.5, 11.9), (11.9, 13., 11.5, 12.6)]);
    b.volume = Some(vec![100., 120., 90., 150., 130., 110., 160., 140.]);
    b
}
pub fn closes(c: &[f64]) -> Bars { from_ohlc(&c.iter().map(|&x| (x, x, x, x)).collect::<Vec<_>>()) }
pub fn go(id: &str, b: &Bars, kv: &[(&str, f64)]) -> Vec<Vec<f64>> {
    let def = get(id).unwrap_or_else(|| panic!("unknown indicator {id}"));
    run(&def, b, &kv.iter().map(|(k, v)| (k.to_string(), *v)).collect()).unwrap_or_else(|e| panic!("{id}: {e}"))
}
/// A one-condition long rule on `id.out`, compiled on 300 synthetic days with volume: proves a rule can reach the series.
pub fn compiles(id: &str, out: &str) -> Result<(), LabError> {
    let spec = parse_spec(&format!(r#"{{"v":1,"entry":{{"all":[{{"lhs":{{"s":"{id}","out":"{out}"}},"op":">","rhs":{{"value":0}}}}]}},"exit":{{"any":[{{"lhs":{{"s":"bars_held"}},"op":">=","rhs":{{"value":1}}}}]}}}}"#))?;
    compile(&spec, &Frames::new(walk(300, 4, true)), &mut Cache::new()).map(|_| ())
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
