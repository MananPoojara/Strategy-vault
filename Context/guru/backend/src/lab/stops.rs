//! Stops and trend state family (TradingView / AmiBroker built-ins, Phase 6). Formulas: see each definition string.
use super::bars::Bars;
use super::catalogue::{int, len, n, p, real, IndicatorDef, Needs, Output, ParamSpec, Params, Placement};
use super::ta::{self, NA};

const CAT: &str = "Trend";
fn div(a: f64, b: f64) -> f64 { if b == 0. || a.is_nan() || b.is_nan() { NA } else { a / b } }
#[allow(clippy::too_many_arguments)]
const fn def(id: &'static str, name: &'static str, params: &'static [ParamSpec], outputs: &'static [&'static str], placement: Placement,
             needs: Needs, definition: &'static str, first: &'static [usize], sparse: bool, f: fn(&Bars, &Params) -> Output) -> IndicatorDef {
    IndicatorDef { id, name, category: CAT, params, outputs, placement, needs, definition, first, sparse, compute: Some(f), note: "" }
}
const PR: Placement = Placement::Price;
const PN: Placement = Placement::Pane;
const NO: Needs = Needs::NONE;
const CK: &[ParamSpec] = &[len("p", "ATR length", 10.), real("x", "ATR multiplier", 1., 0.1, 10., 0.1), len("q", "Stop length", 9.)];
const CH: &[ParamSpec] = &[len("length", "Length", 22.), real("mult", "ATR multiplier", 3., 0.1, 20., 0.1), int("use_close", "Use close (1) or high/low (0)", 1., 0., 1.)];
const VS: &[ParamSpec] = &[len("length", "ATR length", 20.), real("factor", "Factor", 2., 0.1, 20., 0.1)];
const WV: &[ParamSpec] = &[len("length", "ATR length", 7.), real("mult", "ARC multiplier", 3., 0.1, 20., 0.1)];
const CZ: &[ParamSpec] = &[len("length", "Length", 30.), len("ema_length", "EMA length", 34.)];
const RW: &[ParamSpec] = &[len("min_length", "Minimum length", 9.), len("max_length", "Maximum length", 40.)];
const AS: &[ParamSpec] = &[real("limit", "Limit move", 10., 0.01, 1000000., 0.01)];

fn atr(b: &Bars, k: usize) -> Vec<f64> { ta::rma(&ta::tr(b, true), k) }
fn chande_kroll(b: &Bars, prm: &Params) -> Output {
    let (pp, x, q) = (n(prm, "p"), p(prm, "x"), n(prm, "q"));
    let (a, hh, ll) = (atr(b, pp), ta::highest(&b.high, pp), ta::lowest(&b.low, pp));
    let fhs: Vec<f64> = (0..b.len()).map(|i| hh[i] - x * a[i]).collect();
    let fls: Vec<f64> = (0..b.len()).map(|i| ll[i] + x * a[i]).collect();
    Ok(vec![ta::lowest(&fls, q), ta::highest(&fhs, q)])
}
fn chandelier(b: &Bars, prm: &Params) -> Output {
    let (k, m, uc) = (n(prm, "length"), p(prm, "mult"), p(prm, "use_close") == 1.);
    let a: Vec<f64> = atr(b, k).iter().map(|v| m * v).collect();
    let (hi, lo) = if uc { (ta::highest(&b.close, k), ta::lowest(&b.close, k)) } else { (ta::highest(&b.high, k), ta::lowest(&b.low, k)) };
    let len = b.len();
    let (mut ls, mut ss, mut dir) = (vec![NA; len], vec![NA; len], vec![NA; len]);
    let mut d = 1.;
    for i in 0..len {
        let (l0, s0) = (hi[i] - a[i], lo[i] + a[i]);
        let lp = if i > 0 && !ls[i - 1].is_nan() { ls[i - 1] } else { l0 }; // nz(longStop[1], longStop)
        let sp = if i > 0 && !ss[i - 1].is_nan() { ss[i - 1] } else { s0 };
        ls[i] = if i > 0 && b.close[i - 1] > lp { l0.max(lp) } else { l0 };
        ss[i] = if i > 0 && b.close[i - 1] < sp { s0.min(sp) } else { s0 };
        if b.close[i] > sp { d = 1.; } else if b.close[i] < lp { d = -1.; }
        if !ls[i].is_nan() { dir[i] = d; }
    }
    Ok(vec![ls, ss, dir])
}
fn vstop(b: &Bars, prm: &Params) -> Output {
    let (k, f) = (n(prm, "length"), p(prm, "factor"));
    let (a, tr) = (atr(b, k), ta::tr(b, false));
    let len = b.len();
    let (mut st, mut dir) = (vec![NA; len], vec![NA; len]);
    if len == 0 { return Ok(vec![st, dir]); }
    let (mut mx, mut mn, mut up, mut stop) = (b.close[0], b.close[0], true, NA);
    for i in 0..len {
        let src = b.close[i];
        let m = if a[i].is_nan() { tr[i] } else { a[i] * f }; // nz(atr·factor, tr)
        mx = mx.max(src); mn = mn.min(src);
        // Pine math.max/min return na when an argument is na; nz(…, src) then restarts the stop at src
        let cand = if stop.is_nan() || m.is_nan() { NA } else if up { stop.max(mx - m) } else { stop.min(mn + m) };
        stop = if cand.is_nan() { src } else { cand };
        let was = up;
        up = src - stop >= 0.;
        if up != was { mx = src; mn = src; stop = if up { mx - m } else { mn + m }; }
        st[i] = stop; dir[i] = if up { 1. } else { -1. };
    }
    Ok(vec![st, dir])
}
fn wilder_vi(b: &Bars, prm: &Params) -> Output {
    let (a, m) = (atr(b, n(prm, "length")), p(prm, "mult"));
    let len = b.len();
    let (mut s, mut dir) = (vec![NA; len], vec![NA; len]);
    let (mut started, mut long, mut sic, mut sar) = (false, true, NA, NA);
    for i in 0..len {
        if a[i].is_nan() { continue; }
        let c = b.close[i];
        if !started { started = true; long = true; sic = c; }
        else if long && c < sar { long = false; sic = c; }
        else if !long && c > sar { long = true; sic = c; }
        else { sic = if long { sic.max(c) } else { sic.min(c) }; }
        sar = if long { sic - m * a[i] } else { sic + m * a[i] };
        s[i] = sar; dir[i] = if long { 1. } else { -1. };
    }
    Ok(vec![s, dir])
}
fn chop_zone(b: &Bars, prm: &Params) -> Output {
    let k = n(prm, "length");
    let (avg, hh, ll, e) = (b.hlc3(), ta::highest(&b.high, k), ta::lowest(&b.low, k), ta::ema(&b.close, n(prm, "ema_length")));
    let len = b.len();
    let (mut ang, mut zone) = (vec![NA; len], vec![NA; len]);
    for i in 1..len {
        if [hh[i], ll[i], e[i], e[i - 1]].iter().any(|v| v.is_nan()) || hh[i] == ll[i] || avg[i] == 0. { continue; }
        let span = 25. / (hh[i] - ll[i]) * ll[i];
        let y = (e[i - 1] - e[i]) / avg[i] * span;
        let a = (180. * (1. / (1. + y * y).sqrt()).acos() / std::f64::consts::PI).round();
        let a = if y > 0. { -a } else { a };
        ang[i] = a;
        zone[i] = if a >= 5. { 4. } else if a >= 3.57 { 3. } else if a >= 2.14 { 2. } else if a >= 0.71 { 1. }
                  else if a <= -5. { -4. } else if a <= -3.57 { -3. } else if a <= -2.14 { -2. } else if a <= -0.71 { -1. } else { 0. };
    }
    Ok(vec![ang, zone])
}
fn rwi(b: &Bars, prm: &Params) -> Output {
    let (lo_n, hi_n) = (n(prm, "min_length"), n(prm, "max_length"));
    if lo_n > hi_n { return Err("min_length must not exceed max_length".into()); }
    let t = ta::tr(b, true); let len = b.len();
    let atrs: Vec<Vec<f64>> = (lo_n..=hi_n).map(|k| ta::sma(&t, k)).collect();
    let (mut h, mut l, mut r) = (vec![NA; len], vec![NA; len], vec![NA; len]);
    for i in hi_n..len {
        let (mut bh, mut bl) = (NA, NA);
        for (j, k) in (lo_n..=hi_n).enumerate() {
            let d = atrs[j][i] * (k as f64).sqrt();
            let (x, y) = (div(b.high[i] - b.low[i - k], d), div(b.high[i - k] - b.low[i], d));
            if !x.is_nan() { bh = if bh.is_nan() { x } else { bh.max(x) }; }
            if !y.is_nan() { bl = if bl.is_nan() { y } else { bl.max(y) }; }
        }
        h[i] = bh; l[i] = bl; r[i] = bh - bl;
    }
    Ok(vec![h, l, r])
}
fn asi(b: &Bars, prm: &Params) -> Output {
    let t = p(prm, "limit");
    let (o, h, l, c) = (&b.open, &b.high, &b.low, &b.close);
    let mut s = 0.;
    let mut out = vec![NA; b.len()];
    for i in 1..b.len() {
        let (a1, a2, a3, d) = ((h[i] - c[i - 1]).abs(), (l[i] - c[i - 1]).abs(), h[i] - l[i], (c[i - 1] - o[i - 1]).abs());
        let k = a1.max(a2);
        let r = if a1 >= a2 && a1 >= a3 { a1 - 0.5 * a2 + 0.25 * d } else if a2 >= a3 { a2 - 0.5 * a1 + 0.25 * d } else { a3 + 0.25 * d };
        s += if r == 0. { 0. } else { 50. * (c[i] - c[i - 1] + 0.5 * (c[i] - o[i]) + 0.25 * (c[i - 1] - o[i - 1])) / r * k / t };
        out[i] = s;
    }
    Ok(vec![out])
}

pub const DEFS: &[IndicatorDef] = &[
    def("chande_kroll", "Chande Kroll Stop", CK, &["stop_long", "stop_short"], PR, NO, "fhs = highest(high, p) − x·atr(p); fls = lowest(low, p) + x·atr(p); stop_short = highest(fhs, q); stop_long = lowest(fls, q) (window restarts after NaN, as ta.highest)", &[9, 9], false, chande_kroll),
    def("chandelier", "Chandelier Exit", CH, &["long_stop", "short_stop", "direction"], PR, NO, "long = highest(close or high, n) − mult·atr(n), ratcheted up while close[1] > long[1]; short = lowest(close or low, n) + mult·atr(n), ratcheted down while close[1] < short[1]; direction +1 when close > short[1], −1 when close < long[1], else unchanged (starts +1)", &[21, 21, 21], false, chandelier),
    def("vstop", "Volatility Stop", VS, &["stop", "direction"], PR, NO, "TradingView pine_vstop on close: atrM = atr(n)·factor (tr before the ATR exists); trailing max/min of close since the last flip; stop = max(stop, max − atrM) in an uptrend, min(stop, min + atrM) in a downtrend; flip when close crosses the stop", &[0, 0], false, vstop),
    def("wilder_vi", "Volatility Index (Wilder)", WV, &["sar", "direction"], PR, NO, "Wilder Volatility System (declared): ARC = mult·atr(length); SIC = extreme close since the trend began; sar = SIC − ARC (long) / SIC + ARC (short); reverse when close crosses the previous sar; starts long", &[6, 6], false, wilder_vi),
    def("chop_zone", "Chop Zone", CZ, &["angle", "zone"], PN, NO, "span = 25/(highest(high, length) − lowest(low, length))·lowest(low, length); y = (ema[1] − ema)/hlc3·span; angle = ±round(degrees(acos(1/√(1 + y²)))), negative when y > 0; zone 4…−4 by thresholds 5, 3.57, 2.14, 0.71 (else 0)", &[34, 34], false, chop_zone),
    def("rwi", "Random Walk Index", RW, &["hi", "lo", "rwi"], PN, NO, "for n in min..=max: (high − low[n])/(sma(tr, n)·√n) and (high[n] − low)/(sma(tr, n)·√n); hi/lo = max over n (NaN terms skipped); rwi = hi − lo; min_length > max_length → invalid (AmiBroker RWIHi/RWILo/RWI)", &[40, 40, 40], false, rwi),
    def("asi", "Accumulative Swing Index", AS, &["asi"], PN, NO, "Wilder: K = max(|h − c1|, |l − c1|); R from the largest of |h − c1|, |l − c1|, h − l (with 0.25·|c1 − o1|); SI = 50·(c − c1 + 0.5(c − o) + 0.25(c1 − o1))/R·K/limit (R = 0 → 0); asi = cumulative SI from bar 1", &[1], false, asi),
];

#[cfg(test)]
mod tests {
    use crate::lab::catalogue::{get, run, Params};
    use crate::lab::testdata::{assert_series, compiles, d1, go};
    const N: f64 = f64::NAN;
    #[test] fn chande_kroll_short_lengths() {
        let o = go("chande_kroll", &d1(), &[("p", 2.), ("x", 1.), ("q", 2.)]);
        assert_series(&o[0], &[N, 11.0, 11.0, 11.75, 11.5375, 11.01875, 11.009375, 11.009375]);
        assert_series(&o[1], &[N, 10.0, 10.75, 10.925, 10.925, 10.6625, 10.490625, 11.4953125]);
    }
    #[test] fn chandelier_ratchets_and_flips() {
        let o = go("chandelier", &d1(), &[("length", 2.), ("mult", 1.), ("use_close", 1.)]);
        assert_series(&o[0], &[N, 9.5, 10.25, 10.425, 10.425, 9.28125, 10.390625, 11.0953125]);
        assert_series(&o[1], &[N, 12.5, 12.5, 12.5, 11.7375, 11.71875, 11.71875, 13.4046875]);
        assert_series(&o[2], &[N, 1., 1., 1., -1., -1., 1., 1.]);
    }
    #[test] fn volatility_stop_from_bar_zero() {
        let o = go("vstop", &d1(), &[("length", 2.), ("factor", 1.)]);
        assert_series(&o[0], &[10.5, 10.5, 10.5, 10.5, 11.7375, 11.71875, 10.390625, 11.0953125]);
        assert_series(&o[1], &[1., 1., 1., 1., -1., -1., 1., 1.]);
    }
    #[test] fn wilder_volatility_system() {
        let o = go("wilder_vi", &d1(), &[("length", 2.), ("mult", 1.)]);
        assert_series(&o[0], &[N, 9.5, 10.25, 10.425, 11.7375, 11.71875, 10.390625, 11.0953125]);
        assert_series(&o[1], &[N, 1., 1., 1., -1., -1., 1., 1.]);
    }
    #[test] fn chop_zone_angles() {
        let o = go("chop_zone", &d1(), &[("length", 2.), ("ema_length", 2.)]);
        assert_series(&o[0], &[N, N, 80., -81., -82., 63., 81., 81.]);
        assert_series(&o[1], &[N, N, 4., -4., -4., 4., 4., 4.]);
    }
    #[test] fn random_walk_index() {
        let o = go("rwi", &d1(), &[("min_length", 2.), ("max_length", 3.)]);
        assert_series(&o[0], &[N, N, N, 1.131135221269, 0.590471866217, 0.094280904158, 0.942809041582, 1.649915822769]);
        assert_series(&o[1], &[N, N, N, 0.585191818913, 1.219149622735, 1.272792206136, 0.654330305082, 0.0]);
        assert_series(&o[2], &[N, N, N, 0.545943402356, -0.628677756519, -1.178511301978, 0.2884787365, 1.649915822769]);
    }
    #[test] fn rwi_rejects_min_above_max() {
        let p: Params = [("min_length".to_string(), 10.), ("max_length".to_string(), 5.)].into_iter().collect();
        assert!(run(&get("rwi").unwrap(), &d1(), &p).unwrap_err().contains("min_length"));
    }
    #[test] fn accumulative_swing_index() {
        assert_series(&go("asi", &d1(), &[("limit", 10.)])[0], &[N, 5.735294117647, 8.59243697479, 3.182600909216, -0.960256233641, 0.686802589888, 7.232257135343, 11.33789093816]);
    }
    #[test] fn rules_reach_stops() { for (id, out) in [("chandelier", "direction"), ("vstop", "stop"), ("chop_zone", "zone"), ("rwi", "rwi")] { compiles(id, out).unwrap(); } }
}
