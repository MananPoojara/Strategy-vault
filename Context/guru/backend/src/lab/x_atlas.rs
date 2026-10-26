//! Indicators from the Strategy Showcase Atlas catalogue not covered elsewhere (2026-10-19). Formulas: see each definition string.
use super::bars::Bars;
use super::catalogue::{int, len, n, p, real, IndicatorDef, Needs, Output, ParamSpec, Params, Placement};
use super::ta::{self, NA};

#[allow(clippy::too_many_arguments)]
const fn def(id: &'static str, name: &'static str, category: &'static str, params: &'static [ParamSpec], outputs: &'static [&'static str],
             placement: Placement, needs: Needs, definition: &'static str, first: &'static [usize], f: fn(&Bars, &Params) -> Output) -> IndicatorDef {
    IndicatorDef { id, name, category, params, outputs, placement, needs, definition, first, sparse: false, compute: Some(f), note: "" }
}
/// Listed-only entry (listed.rs convention): never computed, carries its reason.
const fn listed(id: &'static str, name: &'static str, category: &'static str, needs: Needs, note: &'static str) -> IndicatorDef {
    IndicatorDef { id, name, category, params: &[], outputs: &["value"], placement: Placement::Pane, needs, definition: name,
                   first: &[usize::MAX], sparse: false, compute: None, note }
}
const SECOND: Needs = Needs { second_symbol: true, ..Needs::NONE };
const DERIVATIVES: Needs = Needs { derivatives: true, ..Needs::NONE };
const BETA: &[ParamSpec] = &[int("length", "Length", 20., 2., 2000.), int("returns", "Returns (0 simple, 1 log)", 0., 0., 1.)];
pub(super) const RENKO: &[ParamSpec] = &[int("mode", "Box size (0 ATR, 1 fixed points, 2 % of close)", 0., 0., 2.), len("length", "ATR length", 14.),
    real("box", "Box size (points, mode 1)", 1., 0.0001, 1e9, 0.0001), real("percent", "Box size (% of the bar's close, mode 2)", 1., 0.001, 100., 0.001),
    int("anchor", "First brick (0 at the first close, 1 AmiBroker grid)", 0., 0., 1.), int("source", "Price (0 close, 1 StockCharts high/low)", 0., 0., 1.)];
const FIB: &[ParamSpec] = &[len("length", "Length", 100.)];
const FIB_LEVELS: [f64; 7] = [0., 0.236, 0.382, 0.5, 0.618, 0.786, 1.];

pub const DEFS: &[IndicatorDef] = &[
    def("beta", "Beta", "Second instrument", BETA, &["value"], Placement::Pane, SECOND,
        "TA-Lib BETA (ta_BETA.c): least-squares slope of this instrument's 1-bar returns (y) on the aligned vs instrument's returns (x) over length returns, beta = (n·Σxy − Σx·Σy)/(n·Σx² − (Σx)²); returns 0 = simple ROC (p/p[1] − 1, as TA-Lib), 1 = ln(p/p[1]); a missing pair, zero or non-positive (log) base price blanks the window; zero regressor variance gives 0 as TA-Lib",
        &[20], beta),
    def("renko", "Renko (bar-aligned)", "Price functions", RENKO, &["brick_close", "direction", "bricks", "brick_high", "brick_low", "flip", "run", "confirm2"], Placement::Pane, Needs::NONE,
        "Classic Renko on closes (TradingView Renko chart, source close): box s = ATR(length) Wilder (ta.atr) at the bar being tested (mode 0; causal — TradingView's ATR box uses one ATR value for the whole chart) or fixed box points (mode 1) or percent × that bar's close / 100 (mode 2; causal — TradingView's Percentage box uses the last price for the whole chart); anchored at the first close with a box; up bricks while close ≥ top + s, down bricks while close ≤ bottom − s, so a reversal needs 2 boxes (TradingView: brick 52–54, box 2 → next brick at ≥56 or ≤50); several bricks per bar, part-box moves dropped (MetaStock: 100 → 103, box 2 → one brick 100–102). First brick 1 (AmiBroker Formula 562): a seed down brick ceil(close/s)·s to that + s on the anchor bar (direction −1, run −1, bricks 0), so with a fixed box every brick edge is a multiple of the box; with an ATR or % box only the seed is on the grid. Price 1 (StockCharts High-Low; declared rule as StockCharts/MetaStock P&F): the high is tested for up bricks and the low for down bricks, the current direction's extreme first (high first before the first brick) and the other only if it formed no brick, at most one direction per bar; no wicks. Outputs: last brick close (anchor close before the first brick), direction +1/−1 (0 before the first brick), bricks formed this bar (signed), last brick top and bottom (both the anchor close before the first brick; top − bottom = the box used), flip 1 on a bar whose bricks reverse the direction (not the first brick off the anchor), run = signed count of consecutive bricks in the current direction including this bar's (0 before the first brick), confirm2 1 on the first bar after a flip on which run reaches ±2 (the flip bar itself when it forms 2 or more bricks)",
        &[13, 13, 13, 13, 13, 13, 13, 13], renko),
    def("fib_retracement", "Fibonacci Retracement (rolling range)", "Bands & channels", FIB,
        &["level_0", "level_236", "level_382", "level_500", "level_618", "level_786", "level_100", "direction"], Placement::Price, Needs::NONE,
        "Fibonacci retracement of the rolling length-bar range H = highest high, L = lowest low: if H's bar is more recent than L's (direction +1) level_x = H − x·(H − L), else (direction −1) level_x = L + x·(H − L), x = 0, 0.236, 0.382, 0.5, 0.618, 0.786, 1; causal rolling-range version, not TradingView's pivot-based Auto Fib Retracement (which redraws)",
        &[99, 99, 99, 99, 99, 99, 99, 99], fib_retracement),
    listed("pcr", "Put-Call Ratio", "Derivatives market data", DERIVATIVES, "not available yet — needs options open-interest data (put and call open interest by strike and expiry)"),
    listed("rollover", "Rollover %", "Derivatives market data", DERIVATIVES, "not available yet — needs futures open interest by expiry (current and next contracts)"),
];
/// (id, platform names, AmiBroker names): one row per id in DEFS.
pub const SOURCES: &[(&str, &str, &str)] = &[
    ("beta", "TA-Lib: BETA; TradingView: Beta (community ta.correlation/stdev scripts)", ""),
    ("renko", "TradingView: Renko chart (Traditional / ATR box size); StockCharts: Renko (Close / High-Low)", "Renko (Kavanagh AFL library, Formula 562)"),
    ("fib_retracement", "TradingView: Fib Retracement / Auto Fib Retracement (pivot-based; this is a rolling-range version)", ""),
    ("pcr", "NSE: Put-Call Ratio (option chain open interest)", ""),
    ("rollover", "NSE: Futures rollover %", ""),
];

fn simple_returns(x: &[f64]) -> Vec<f64> { (0..x.len()).map(|i| if i == 0 || x[i - 1] == 0. { NA } else { x[i] / x[i - 1] - 1. }).collect() }
fn beta(b: &Bars, prm: &Params) -> Output {
    let s = super::pairs::second(b)?;
    let k = n(prm, "length");
    let r = |v: &[f64]| if p(prm, "returns") == 1. { super::pairs::log_returns(v) } else { simple_returns(v) };
    let (y, x) = (r(&b.close), r(s));
    let mut out = vec![NA; x.len()];
    for i in k..x.len() {
        let (wx, wy) = (&x[i + 1 - k..=i], &y[i + 1 - k..=i]);
        if wx.iter().chain(wy).any(|v| v.is_nan()) { continue; }
        let (mx, my) = (wx.iter().sum::<f64>() / k as f64, wy.iter().sum::<f64>() / k as f64);
        let (sxx, sxy) = wx.iter().zip(wy).fold((0., 0.), |(a, c), (x, y)| (a + (x - mx) * (x - mx), c + (x - mx) * (y - my)));
        // centered form of TA-Lib's (n·Σxy − Σx·Σy)/(n·Σx² − (Σx)²); its zero-denominator guard (denom ≤ 1e-14·scale → 0)
        let v = if sxx > 1e-14 * wx.iter().map(|x| x * x).sum::<f64>() { sxy / sxx } else { 0. };
        if !v.is_finite() { return Err("pair beta arithmetic overflow".into()); }
        out[i] = v;
    }
    Ok(vec![out])
}

/// The box tested on each bar (mode 0 ATR, 1 fixed, 2 % of close).
pub(super) fn renko_box(b: &Bars, prm: &Params) -> Vec<f64> {
    match n(prm, "mode") { 1 => vec![p(prm, "box"); b.len()], 2 => b.close.iter().map(|c| c * p(prm, "percent") / 100.).collect(), _ => ta::rma(&ta::tr(b, true), n(prm, "length")) }
}
fn renko(b: &Bars, prm: &Params) -> Output {
    let (len, size) = (b.len(), renko_box(b, prm));
    let (hl, grid) = (p(prm, "source") == 1., p(prm, "anchor") == 1.);
    let mut out = vec![vec![NA; len]; 8];
    let (mut hi, mut lo, mut dir, mut run, mut after_flip) = (NA, NA, 0., 0f64, false);
    for i in 0..len {
        let (c, s) = (b.close[i], size[i]);
        if c.is_nan() || s.is_nan() { continue; }
        let (u, d) = if hl { (b.high[i], b.low[i]) } else { (c, c) };
        let (mut k, was) = (0., dir);
        if hi.is_nan() {
            if grid && s > 0. { lo = (c / s).ceil() * s; hi = lo + s; dir = -1.; run = -1.; } else { hi = c; lo = c; }
        } else {
            // with closes at most one test can pass; with high/low the current direction's extreme goes first
            let (up, dn) = (s > 0. && u >= hi + s, s > 0. && d <= lo - s);
            if up && (dir >= 0. || !dn) { k = ((u - hi) / s).floor().max(1.); lo = hi + (k - 1.) * s; hi += k * s; dir = 1.; }
            else if dn { k = ((lo - d) / s).floor().max(1.); hi = lo - (k - 1.) * s; lo -= k * s; dir = -1.; k = -k; }
        }
        let flip = k != 0. && was != 0. && dir != was;
        let before = if flip { 0. } else { run.abs() };
        if k != 0. { run = if flip || run == 0. { k } else { run + k }; }
        if flip { after_flip = true; }
        out[0][i] = if dir < 0. { lo } else { hi };
        out[1][i] = dir;
        out[2][i] = k;
        out[3][i] = hi;
        out[4][i] = lo;
        out[5][i] = if flip { 1. } else { 0. };
        out[6][i] = run;
        out[7][i] = if after_flip && before < 2. && run.abs() >= 2. { 1. } else { 0. };
    }
    Ok(out)
}

fn fib_retracement(b: &Bars, prm: &Params) -> Output {
    let (len, k) = (b.len(), n(prm, "length"));
    let mut out = vec![vec![NA; len]; FIB_LEVELS.len() + 1];
    for i in k.saturating_sub(1)..len {
        let w = i + 1 - k..=i;
        if w.clone().any(|j| b.high[j].is_nan() || b.low[j].is_nan()) { continue; }
        // most recent bar of each extreme
        let hj = w.clone().fold(i + 1 - k, |m, j| if b.high[j] >= b.high[m] { j } else { m });
        let lj = w.fold(i + 1 - k, |m, j| if b.low[j] <= b.low[m] { j } else { m });
        let (h, l, up) = (b.high[hj], b.low[lj], hj > lj);
        for (o, x) in out.iter_mut().zip(FIB_LEVELS) { o[i] = if up { h - x * (h - l) } else { l + x * (h - l) }; }
        out[FIB_LEVELS.len()][i] = if up { 1. } else { -1. };
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use crate::lab::catalogue::{get, run};
    use crate::lab::testdata::{assert_series, close_eq, closes, cut, from_ohlc, go, walk};
    const N: f64 = f64::NAN;
    fn pair(a: &[f64], b: &[f64]) -> crate::lab::bars::Bars { let mut x = closes(a); x.second = Some(b.to_vec()); x }

    #[test] fn beta_is_the_return_regression_slope_on_the_vs_instrument() {
        // vs returns +10%, −10%, +10%; chart returns exactly twice → beta 2 (simple returns)
        let b = pair(&[100., 120., 96., 115.2], &[100., 110., 99., 108.9]);
        assert_series(&go("beta", &b, &[("length", 2.)])[0], &[N, N, 2., 2.]);
        // chart = 3 × vs: identical log returns → beta 1
        assert_series(&go("beta", &pair(&[3., 6., 9., 6.], &[1., 2., 3., 2.]), &[("length", 2.), ("returns", 1.)])[0], &[N, N, 1., 1.]);
        // flat vs: zero regressor variance gives 0 (TA-Lib)
        assert_series(&go("beta", &pair(&[1., 2., 3.], &[5., 5., 5.]), &[("length", 2.)])[0], &[N, N, 0.]);
        // a missing vs close blanks every window containing its returns
        assert_series(&go("beta", &pair(&[1., 2., 3., 4., 5., 6.], &[1., N, 3., 4., 5., 6.]), &[("length", 2.)])[0][..4], &[N, N, N, N]);
    }
    #[test] fn beta_needs_a_second_instrument_and_never_uses_later_bars() {
        let d = get("beta").unwrap();
        assert_eq!(run(&d, &walk(30, 1, false), &Default::default()).unwrap_err(), "needs a second instrument");
        let mut full = walk(300, 2, false);
        full.second = Some(walk(300, 3, false).close);
        let want = run(&d, &full, &Default::default()).unwrap();
        assert!(want[0][..d.first[0]].iter().all(|v| v.is_nan()) && want[0][d.first[0]..].iter().all(|v| v.is_finite()));
        for c in [37usize, 150, 299] {
            let mut pre = cut(&full, c);
            pre.second = Some(full.second.as_ref().unwrap()[..c].to_vec());
            let got = run(&d, &pre, &Default::default()).unwrap();
            for i in 0..c { assert!(close_eq(got[0][i], want[0][i]), "beta bar {i} changes when later bars exist"); }
        }
    }
    #[test] fn renko_fixed_box_needs_two_boxes_to_reverse() {
        // TradingView's example: brick 52–54, box 2 → next brick at ≥56 or ≤50
        let o = go("renko", &closes(&[50., 52., 54., 55., 56., 51., 50., 45., 49., 50.]), &[("mode", 1.), ("box", 2.)]);
        assert_series(&o[0], &[50., 52., 54., 54., 56., 52., 50., 46., 46., 50.]);
        assert_series(&o[1], &[0., 1., 1., 1., 1., -1., -1., -1., -1., 1.]);
        assert_series(&o[2], &[0., 1., 1., 0., 1., -1., -1., -2., 0., 1.]);
    }
    #[test] fn renko_atr_box_is_the_wilder_atr_of_the_tested_bar() {
        // high/low = close ± 1 and |Δclose| ≤ 1 → true range 2 every bar, ATR(2) = 2 from bar 1
        let b = from_ohlc(&[50., 50., 51., 52., 51., 50., 49., 48.].map(|c| (c, c + 1., c - 1., c)));
        let o = go("renko", &b, &[("length", 2.)]);
        assert_series(&o[0], &[N, 50., 50., 52., 52., 52., 52., 48.]);
        assert_series(&o[1], &[N, 0., 0., 1., 1., 1., 1., -1.]);
        assert_series(&o[2], &[N, 0., 0., 1., 0., 0., 0., -1.]);
    }
    #[test] fn renko_new_outputs_track_edges_flips_runs_and_two_brick_confirmation() {
        let o = go("renko", &closes(&[50., 52., 54., 55., 56., 51., 50., 45., 49., 50.]), &[("mode", 1.), ("box", 2.)]);
        assert_series(&o[3], &[50., 52., 54., 54., 56., 54., 52., 48., 48., 50.]);
        assert_series(&o[4], &[50., 50., 52., 52., 54., 52., 50., 46., 46., 48.]);
        // the first brick off the anchor is not a flip and its run never confirms
        assert_series(&o[5], &[0., 0., 0., 0., 0., 1., 0., 0., 0., 1.]);
        assert_series(&o[6], &[0., 1., 2., 2., 3., -1., -2., -4., -4., 1.]);
        assert_series(&o[7], &[0., 0., 0., 0., 0., 0., 1., 0., 0., 0.]);
    }
    #[test] fn renko_amibroker_grid_seeds_a_down_brick_on_box_multiples() {
        // 51, box 2 → seed down brick 52–54 (ceil(51/2)·2); up needs ≥ 56, down ≤ 50; a 2-brick flip confirms on its own bar
        let o = go("renko", &closes(&[51., 52., 53., 55., 56., 52., 47., 54.]), &[("mode", 1.), ("box", 2.), ("anchor", 1.)]);
        assert_series(&o[0], &[52., 52., 52., 52., 56., 52., 48., 54.]);
        assert_series(&o[1], &[-1., -1., -1., -1., 1., -1., -1., 1.]);
        assert_series(&o[2], &[0., 0., 0., 0., 1., -1., -2., 2.]);
        assert_series(&o[3], &[54., 54., 54., 54., 56., 54., 50., 54.]);
        assert_series(&o[4], &[52., 52., 52., 52., 54., 52., 48., 52.]);
        assert_series(&o[5], &[0., 0., 0., 0., 1., 1., 0., 1.]);
        assert_series(&o[6], &[-1., -1., -1., -1., 1., -1., -3., 2.]);
        assert_series(&o[7], &[0., 0., 0., 0., 0., 0., 1., 1.]);
    }
    #[test] fn renko_percent_box_is_that_bars_close() {
        // 10 %: bar 1 box 12 → brick 100–112; bar 2 box 9 → 90 ≤ 100 − 9 reverses (a fixed box of 12 would need ≤ 88)
        let o = go("renko", &closes(&[100., 120., 90.]), &[("mode", 2.), ("percent", 10.)]);
        assert_series(&o[0], &[100., 112., 91.]);
        assert_series(&o[2], &[0., 1., -1.]);
        assert_series(&o[3], &[100., 112., 100.]);
        assert_series(&o[4], &[100., 100., 91.]);
    }
    #[test] fn renko_stockcharts_high_low_tests_the_trend_extreme_first() {
        let b = from_ohlc(&[(50., 51., 49., 50.), (50., 52.5, 47., 51.), (51., 54.5, 47.5, 48.), (48., 55., 49.9, 50.), (50., 54.2, 47.9, 50.), (48., 52.1, 47., 48.)]);
        let o = go("renko", &b, &[("mode", 1.), ("box", 2.), ("source", 1.)]);
        // bar 1: no direction yet → high first; bar 2: up trend → high; bar 4: down trend → low first though the high also reached 54
        assert_series(&o[0], &[50., 52., 54., 50., 48., 52.]);
        assert_series(&o[2], &[0., 1., 1., -1., -1., 1.]);
        assert_series(&o[6], &[0., 1., 2., -1., -2., 1.]);
        assert_series(&o[7], &[0., 0., 0., 0., 1., 0.]);
        // on closes the same bars form a different chart
        assert_series(&go("renko", &b, &[("mode", 1.), ("box", 2.)])[0], &[50., 50., 48., 48., 48., 48.]);
    }
    #[test] fn fib_levels_follow_the_more_recent_extreme() {
        let b = from_ohlc(&[(9., 10., 8., 9.), (10., 12., 9., 11.), (10., 11., 7., 8.), (9., 13., 10., 12.)]);
        let o = go("fib_retracement", &b, &[("length", 3.)]);
        // bar 2: high 12 (bar 1) before low 7 (bar 2) → measured up from the low
        assert_eq!((o[0][2], o[3][2], o[6][2], o[7][2]), (7., 9.5, 12., -1.));
        // bar 3: high 13 (bar 3) after low 7 (bar 2) → measured down from the high
        assert!(close_eq(o[4][3], 13. - 0.618 * 6.) && o[0][3] == 13. && o[6][3] == 7. && o[7][3] == 1.);
        assert!(o.iter().all(|s| s[1].is_nan()));
    }
    #[test] fn listed_derivatives_entries_are_never_computed() {
        for id in ["pcr", "rollover"] {
            let d = get(id).unwrap();
            assert!(d.compute.is_none() && d.needs.derivatives);
            assert_eq!(run(&d, &walk(10, 1, false), &Default::default()).unwrap_err(), d.note);
        }
    }
}
