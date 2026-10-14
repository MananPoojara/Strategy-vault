//! Price-function family (TradingView built-ins).
use super::bars::Bars;
use super::catalogue::{len, n, IndicatorDef, Needs, Output, ParamSpec, Params, Placement};
use super::ta::{self, NA};

const NEEDS: Needs = Needs::NONE;

fn highest(b: &Bars, prm: &Params) -> Output { Ok(vec![ta::highest(&b.high, n(prm, "length"))]) }
fn lowest(b: &Bars, prm: &Params) -> Output { Ok(vec![ta::lowest(&b.low, n(prm, "length"))]) }
fn change(b: &Bars, prm: &Params) -> Output { Ok(vec![ta::change(&b.close, n(prm, "length"))]) }
/// Pivot at p = t - right, reported on bar t: ties on the left allowed, ties on the right cancel.
fn pivot(x: &[f64], prm: &Params, high: bool) -> Output {
    let (left, right) = (n(prm, "left"), n(prm, "right"));
    let beats = |a: f64, b: f64, strict: bool| if high { if strict { a > b } else { a >= b } } else if strict { a < b } else { a <= b };
    Ok(vec![(0..x.len()).map(|t| {
        if t < right || t - right < left { return NA; }
        let p = t - right;
        let ok = x[p - left..p].iter().all(|&v| beats(x[p], v, false)) && x[p + 1..=t].iter().all(|&v| beats(x[p], v, true));
        if ok { x[p] } else { NA }
    }).collect()])
}
fn pivot_high(b: &Bars, prm: &Params) -> Output { pivot(&b.high, prm, true) }
fn pivot_low(b: &Bars, prm: &Params) -> Output { pivot(&b.low, prm, false) }

const LEN20: &[ParamSpec] = &[len("length", "Length", 20.)];
const LEN1: &[ParamSpec] = &[len("length", "Length", 1.)];
const LR: &[ParamSpec] = &[len("left", "Left bars", 5.), len("right", "Right bars", 5.)];

const fn def(id: &'static str, name: &'static str, params: &'static [ParamSpec], out: &'static [&'static str], placement: Placement, definition: &'static str, first: &'static [usize], sparse: bool, f: fn(&Bars, &Params) -> Output) -> IndicatorDef {
    IndicatorDef { id, name, category: "Price functions", params, outputs: out, placement, needs: NEEDS, definition, first, sparse, compute: Some(f), note: "" }
}

pub const DEFS: &[IndicatorDef] = &[
    def("highest", "Highest High", LEN20, &["highest"], Placement::Price, "highest(high, length)", &[19], false, highest),
    def("lowest", "Lowest Low", LEN20, &["lowest"], Placement::Price, "lowest(low, length)", &[19], false, lowest),
    def("change", "Change", LEN1, &["change"], Placement::Pane, "close - close[length]", &[1], false, change),
    def("pivot_high", "Pivot High", LR, &["pivot"], Placement::Price, "ta.pivothigh: high[t-right] if >= the left bars and > the right bars (right tie cancels), reported on bar t", &[usize::MAX], true, pivot_high),
    def("pivot_low", "Pivot Low", LR, &["pivot"], Placement::Price, "ta.pivotlow: low[t-right] if <= the left bars and < the right bars (right tie cancels), reported on bar t", &[usize::MAX], true, pivot_low),
];

#[cfg(test)]
mod tests {
    use crate::lab::{catalogue::{get, run, Params}, testdata::{assert_series, from_ohlc}};
    const N: f64 = f64::NAN;
    fn many(kv: &[(&str, f64)]) -> Params { kv.iter().map(|(k, v)| (k.to_string(), *v)).collect() }
    fn with(k: &str, v: f64) -> Params { many(&[(k, v)]) }
    fn hs(h: &[f64]) -> crate::lab::bars::Bars { from_ohlc(&h.iter().map(|&x| (x, x, x, x)).collect::<Vec<_>>()) }
    fn piv(id: &str, h: &[f64], l: f64, r: f64) -> Vec<f64> { run(&get(id).unwrap(), &hs(h), &many(&[("left", l), ("right", r)])).unwrap().remove(0) }
    #[test] fn highest_len2() { assert_series(&run(&get("highest").unwrap(), &from_ohlc(&[(1.,3.,1.,1.),(1.,2.,1.,1.),(1.,5.,1.,1.)]), &with("length", 2.)).unwrap()[0], &[N, 3., 5.]); }
    #[test] fn lowest_len2() { assert_series(&run(&get("lowest").unwrap(), &from_ohlc(&[(1.,3.,2.,1.),(1.,2.,1.,1.),(1.,5.,4.,1.)]), &with("length", 2.)).unwrap()[0], &[N, 1., 1.]); }
    #[test] fn change_len2() { assert_series(&run(&get("change").unwrap(), &hs(&[1., 4., 9., 3.]), &with("length", 2.)).unwrap()[0], &[N, N, 8., -1.]); }
    #[test] fn pivot_high_reported_right_bars_late() {
        assert_series(&piv("pivot_high", &[1., 2., 5., 2., 1.], 2., 2.), &[N, N, N, N, 5.]);
    }
    #[test] fn pivot_high_left_tie_reported() { assert_series(&piv("pivot_high", &[5., 5., 2., 1.], 1., 2.), &[N, N, N, 5.]); }
    #[test] fn pivot_high_right_tie_cancelled() { assert_series(&piv("pivot_high", &[1., 5., 5., 1.], 1., 2.), &[N, N, N, N]); }
    #[test] fn pivot_low_mirrors() {
        assert_series(&piv("pivot_low", &[5., 4., 1., 4., 5.], 2., 2.), &[N, N, N, N, 1.]);
        assert_series(&piv("pivot_low", &[3., 1., 1., 4.], 1., 2.), &[N, N, N, N]);
        assert_series(&piv("pivot_low", &[1., 1., 4., 5.], 1., 2.), &[N, N, N, 1.]);
    }
}
