//! Patterns family (TradingView / AmiBroker built-ins, Phase 6). Formulas: see each definition string.
use super::bars::Bars;
use super::catalogue::{get, int, len, n, p, run, IndicatorDef, Needs, Output, ParamSpec, Params, Placement};
use super::ta::{self, NA};

const CAT: &str = "Patterns";
fn div(a: f64, b: f64) -> f64 { if b == 0. || a.is_nan() || b.is_nan() { NA } else { a / b } }
#[allow(clippy::too_many_arguments)]
const fn def(id: &'static str, name: &'static str, params: &'static [ParamSpec], outputs: &'static [&'static str], placement: Placement,
             needs: Needs, definition: &'static str, first: &'static [usize], sparse: bool, f: fn(&Bars, &Params) -> Output) -> IndicatorDef {
    IndicatorDef { id, name, category: CAT, params, outputs, placement, needs, definition, first, sparse, compute: Some(f), note: "" }
}
const NO: Needs = Needs::NONE;
const FR: &[ParamSpec] = &[int("periods", "Periods", 2., 1., 100.)];
// length 1 makes RSI degenerate, so the minimum is 2 (as for the other length-1-degenerate indicators)
const DV: &[ParamSpec] = &[int("length", "RSI length", 14., 2., 2000.), len("lbr", "Pivot lookback right", 5.), len("lbl", "Pivot lookback left", 5.), len("range_upper", "Max bars between pivots", 60.), len("range_lower", "Min bars between pivots", 5.)];
const LN9: &[ParamSpec] = &[len("length", "Length", 9.)];

fn fractal(x: &[f64], k: usize, up: bool) -> Vec<f64> {
    let beats = |a: f64, v: f64| if up { v < a } else { v > a };
    let ties = |a: f64, v: f64| if up { v <= a } else { v >= a };
    // value appears on the confirmation bar t = p + k, never on p
    (0..x.len()).map(|t| {
        if t < 2 * k { return NA; }
        let p = t - k;
        if !(1..=k).all(|i| beats(x[p], x[p + i])) { return NA; }
        let left = (0..=4usize).any(|e| p >= k + e && (1..=e).all(|j| ties(x[p], x[p - j])) && (1..=k).all(|i| beats(x[p], x[p - e - i])));
        if left { x[p] } else { NA }
    }).collect()
}
fn fractals(b: &Bars, prm: &Params) -> Output { let k = n(prm, "periods"); Ok(vec![fractal(&b.high, k, true), fractal(&b.low, k, false)]) }
fn rsi_divergence(b: &Bars, prm: &Params) -> Output {
    let rp: Params = [("length".to_string(), p(prm, "length"))].into_iter().collect();
    let osc = run(&get("rsi").expect("rsi is in the catalogue"), b, &rp)?.remove(0);
    let (lbr, lbl, hi_r, lo_r) = (n(prm, "lbr"), n(prm, "lbl"), n(prm, "range_upper"), n(prm, "range_lower"));
    let pl: Vec<bool> = super::price::pivots(&osc, lbl, lbr, false).iter().map(|v| !v.is_nan()).collect();
    let ph: Vec<bool> = super::price::pivots(&osc, lbl, lbr, true).iter().map(|v| !v.is_nan()).collect();
    let len = b.len();
    let mut out = vec![vec![0.; len]; 4];
    let (mut last_pl, mut last_ph): (Option<(f64, f64, usize)>, Option<(f64, f64, usize)>) = (None, None); // (osc, price, bar) of the previous pivot
    // _inRange(flag[1]) = barssince(flag[1]) in [range_lower, range_upper]: bars since the previous pivot bar j is i - 1 - j
    let in_range = |i: usize, j: usize| { let bars = i - 1 - j; lo_r <= bars && bars <= hi_r };
    for i in 0..len {
        if pl[i] {
            let (o, pr) = (osc[i - lbr], b.low[i - lbr]);
            if let Some((po, pp, j)) = last_pl { if in_range(i, j) {
                if o > po && pr < pp { out[0][i] = 1.; }
                if o < po && pr > pp { out[1][i] = 1.; }
            } }
            last_pl = Some((o, pr, i));
        }
        if ph[i] {
            let (o, pr) = (osc[i - lbr], b.high[i - lbr]);
            if let Some((po, pp, j)) = last_ph { if in_range(i, j) {
                if o < po && pr > pp { out[2][i] = 1.; }
                if o > po && pr < pp { out[3][i] = 1.; }
            } }
            last_ph = Some((o, pr, i));
        }
    }
    Ok(out)
}
fn ad_ratio_bars(b: &Bars, prm: &Params) -> Output {
    let k = n(prm, "length");
    let up: Vec<f64> = (0..b.len()).map(|i| if b.close[i] > b.open[i] { 1. } else { 0. }).collect();
    let dn: Vec<f64> = (0..b.len()).map(|i| if b.close[i] < b.open[i] { 1. } else { 0. }).collect();
    Ok(vec![ta::sum(&up, k).iter().zip(ta::sum(&dn, k)).map(|(u, d)| div(*u, d)).collect()])
}
fn bar_patterns(b: &Bars, _: &Params) -> Output {
    let (h, l) = (&b.high, &b.low);
    let f = |c: &dyn Fn(usize) -> bool| -> Vec<f64> { (0..b.len()).map(|i| if i > 0 && c(i) { 1. } else { 0. }).collect() };
    Ok(vec![f(&|i| l[i] > h[i - 1]), f(&|i| h[i] < l[i - 1]), f(&|i| h[i] < h[i - 1] && l[i] > l[i - 1]), f(&|i| h[i] > h[i - 1] && l[i] < l[i - 1])])
}

pub const DEFS: &[IndicatorDef] = &[
    def("fractals", "Williams Fractal", FR, &["up", "down"], Placement::Price, NO, "TradingView built-in: up fractal at bar t − n if its high is above the n bars after it (strict) and above the n bars before it (strict), allowing up to 4 equal-or-lower bars immediately before; reported on bar t (confirmed n bars late) with the fractal's high; down mirrors with lows; NaN otherwise", &[usize::MAX, usize::MAX], true, fractals),
    def("rsi_divergence", "RSI Divergence", DV, &["bull", "hidden_bull", "bear", "hidden_bear"], Placement::Pane, NO, "TradingView RSI Divergence Indicator: osc = rsi(close, length); pivots of osc (lbl, lbr); on a confirmed pivot low, compared with the previous one (bars since it, range_lower..range_upper): bull = osc higher low and price lower low; hidden_bull = osc lower low and price higher low; bear/hidden_bear mirror on pivot highs; 1 on the confirmation bar, else 0", &[0, 0, 0, 0], false, rsi_divergence),
    def("ad_ratio_bars", "Advance/Decline Ratio (Bars)", LN9, &["ratio"], Placement::Pane, NO, "count(close > open)/count(close < open) over length bars; no down bar → NaN (declared default 9)", &[usize::MAX], true, ad_ratio_bars),
    def("bar_patterns", "Gap and Inside/Outside Bars", &[], &["gap_up", "gap_down", "inside", "outside"], Placement::Pane, NO, "AmiBroker GapUp: low > high[1]; GapDown: high < low[1]; Inside: high < high[1] and low > low[1]; Outside: high > high[1] and low < low[1]; 1/0, bar 0 = 0", &[0, 0, 0, 0], false, bar_patterns),
];

#[cfg(test)]
mod tests {
    use crate::lab::testdata::{assert_series, compiles, d1, from_ohlc, go};
    const N: f64 = f64::NAN;
    fn highs(h: &[f64]) -> crate::lab::bars::Bars { from_ohlc(&h.iter().map(|&x| (x, x, x - 1., x)).collect::<Vec<_>>()) }
    #[test] fn fractals_confirmed_late_with_tie_rules() {
        let o = go("fractals", &highs(&[1., 2., 3., 5., 4., 3., 2., 3., 3., 6., 5., 4.]), &[("periods", 2.)]);
        assert_series(&o[0], &[N, N, N, N, N, 5., N, N, N, N, N, 6.]);
        assert_series(&o[1], &[N, N, N, N, N, N, N, N, 1., N, N, N]);
        assert_series(&go("fractals", &highs(&[1., 2., 5., 5., 3., 2.]), &[("periods", 2.)])[0], &[N, N, N, N, N, 5.]); // equal bar on the left allowed
        assert_series(&go("fractals", &highs(&[1., 2., 5., 3., 5., 2.]), &[("periods", 2.)])[0], &[N; 6]); // equal bar on the right cancels
    }
    #[test] fn rsi_bearish_divergence_on_confirmation_bar() {
        let c = [10., 9., 8., 9., 10., 9., 7.5, 8.5, 9., 8., 9.5, 10.5, 11., 10.2, 10.8, 11.5, 11.2, 12., 11.];
        let b = from_ohlc(&c.iter().map(|&x| (x, x + 0.2, x - 0.2, x)).collect::<Vec<_>>());
        let o = go("rsi_divergence", &b, &[("length", 3.), ("lbr", 1.), ("lbl", 1.), ("range_upper", 10.), ("range_lower", 1.)]);
        let mut bear = vec![0.; 19]; bear[16] = 1.;
        assert_series(&o[2], &bear);
        for k in [0, 1, 3] { assert!(o[k].iter().all(|v| *v == 0.), "output {k}"); }
    }
    #[test] fn advance_decline_bar_ratio() {
        assert_series(&go("ad_ratio_bars", &d1(), &[("length", 3.)])[0], &[N, N, N, 2., 0.5, 0.5, 2., N]); // no down bar in the last window → NaN
    }
    #[test] fn gaps_inside_outside() {
        let b = from_ohlc(&[(10., 12., 9., 11.), (13., 14., 12.5, 13.5), (12., 12., 11., 11.5), (11.6, 11.9, 11.2, 11.5), (11., 12.5, 10.5, 12.)]);
        let o = go("bar_patterns", &b, &[]);
        assert_series(&o[0], &[0., 1., 0., 0., 0.]);
        assert_series(&o[1], &[0., 0., 1., 0., 0.]);
        assert_series(&o[2], &[0., 0., 0., 1., 0.]);
        assert_series(&o[3], &[0., 0., 0., 0., 1.]);
    }
    #[test] fn rules_reach_patterns() { for (id, out) in [("fractals", "down"), ("rsi_divergence", "bear"), ("bar_patterns", "outside")] { compiles(id, out).unwrap(); } }
}
