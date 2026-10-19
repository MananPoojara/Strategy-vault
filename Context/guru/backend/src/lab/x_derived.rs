//! Cross-platform indicators, derived indicators (one rule-expression step from existing outputs) (2026-10-19 build list: TradingView, AmiBroker, MetaTrader, cTrader, NinjaTrader, TradeStation, thinkorswim, Sierra Chart, MetaStock). Formulas: see each definition string.
//! Components are computed by calling the catalogue (as ratings.rs does), so a derived value equals the rule expression on the component.
use super::bars::{Bars, Breadth};
use super::catalogue::{get, int, len, n, p, real, run, IndicatorDef, Needs, Output, ParamSpec, Params, Placement};
use super::ta::{self, NA};

const PR: Placement = Placement::Price;
const PN: Placement = Placement::Pane;
const NO: Needs = Needs::NONE;
const V: Needs = Needs { volume: true, ..Needs::NONE };
const B: Needs = Needs { breadth: true, ..Needs::NONE };
#[allow(clippy::too_many_arguments)]
const fn def(id: &'static str, name: &'static str, category: &'static str, params: &'static [ParamSpec], outputs: &'static [&'static str], placement: Placement,
             needs: Needs, definition: &'static str, first: &'static [usize], sparse: bool, f: fn(&Bars, &Params) -> Output) -> IndicatorDef {
    IndicatorDef { id, name, category, params, outputs, placement, needs, definition, first, sparse, compute: Some(f), note: "" }
}

fn div(a: f64, b: f64) -> f64 { if b == 0. || a.is_nan() || b.is_nan() { NA } else { a / b } }
fn back(v: &[f64], i: usize, k: usize) -> f64 { if i >= k { v[i - k] } else { NA } }
fn map(m: usize, f: impl FnMut(usize) -> f64) -> Vec<f64> { (0..m).map(f).collect() }
/// A catalogue indicator with the given parameters (others at their defaults).
fn comp(id: &str, b: &Bars, kv: &[(&str, f64)]) -> Output { run(&get(id).expect("component in catalogue"), b, &kv.iter().map(|(k, v)| (k.to_string(), *v)).collect()) }
fn one(id: &str, b: &Bars, kv: &[(&str, f64)]) -> Result<Vec<f64>, String> { Ok(comp(id, b, kv)?.remove(0)) }
/// 1/0, NaN while an input is NaN.
fn flag(c: bool, inputs: &[f64]) -> f64 { if inputs.iter().any(|v| v.is_nan()) { NA } else if c { 1. } else { 0. } }
/// +1 when both series rose, −1 when both fell, else 0; NaN until both changes exist.
fn both_dir(a: &[f64], c: &[f64], i: usize) -> f64 {
    let (da, dc) = (a[i] - back(a, i, 1), c[i] - back(c, i, 1));
    if da.is_nan() || dc.is_nan() { NA } else if da > 0. && dc > 0. { 1. } else if da < 0. && dc < 0. { -1. } else { 0. }
}
fn vol(b: &Bars) -> &[f64] { b.volume.as_deref().expect("catalogue::run checks needs.volume") }
fn br(b: &Bars) -> &Breadth { b.breadth.as_ref().expect("catalogue::run checks needs.breadth") }
fn ma(x: &[f64], k: usize, exponential: bool) -> Vec<f64> { if exponential { ta::ema(x, k) } else { ta::sma(x, k) } }

// ---- Trend ----
fn adx_of(b: &Bars, di: f64, sm: f64) -> Result<Vec<f64>, String> { Ok(comp("dmi", b, &[("di_length", di), ("adx_smoothing", sm)])?.remove(2)) }
fn adxr_s(adx: &[f64], lag: usize) -> Vec<f64> { map(adx.len(), |i| (adx[i] + back(adx, i, lag)) / 2.) }
fn adxr(b: &Bars, prm: &Params) -> Output { Ok(vec![adxr_s(&adx_of(b, p(prm, "di_length"), p(prm, "adx_smoothing"))?, n(prm, "lag"))]) }
fn dx(b: &Bars, prm: &Params) -> Output {
    let d = comp("dmi", b, &[("di_length", p(prm, "di_length"))])?;
    // same zero-sum guard as dmi's ADX, so rma(dx, n) = adx
    Ok(vec![map(b.len(), |i| { let s = d[0][i] + d[1][i]; 100. * (d[0][i] - d[1][i]).abs() / if s == 0. { 1. } else { s } })])
}
fn di_osc(b: &Bars, prm: &Params) -> Output { let d = comp("dmi", b, &[("di_length", p(prm, "di_length"))])?; Ok(vec![map(b.len(), |i| d[0][i] - d[1][i])]) }
fn csi(b: &Bars, prm: &Params) -> Output {
    let k = p(prm, "length");
    let (r, atr) = (adxr_s(&adx_of(b, k, k)?, n(prm, "lag")), one("atr", b, &[("length", k)])?);
    let f = p(prm, "point_value") / p(prm, "margin").sqrt() / (150. + p(prm, "commission")) * 100.;
    Ok(vec![map(b.len(), |i| r[i] * atr[i] * f)])
}
fn elder_impulse(b: &Bars, prm: &Params) -> Output {
    let e = one("ema", b, &[("length", p(prm, "ema"))])?;
    let h = comp("macd", b, &[("fast", p(prm, "fast")), ("slow", p(prm, "slow")), ("signal", p(prm, "signal"))])?.remove(2);
    Ok(vec![map(b.len(), |i| both_dir(&e, &h, i))])
}
fn swing_index(b: &Bars, prm: &Params) -> Output {
    let a = one("asi", b, &[("limit", p(prm, "limit"))])?;
    // asi accumulates from bar 1, so SI at bar 1 is asi[1]
    Ok(vec![map(b.len(), |i| if i == 1 { a[1] } else { a[i] - back(&a, i, 1) })])
}

// ---- Volatility ----
fn natr(b: &Bars, prm: &Params) -> Output { let a = one("atr", b, &[("length", p(prm, "length"))])?; Ok(vec![map(b.len(), |i| div(100. * a[i], b.close[i]))]) }
fn hv_ratio(b: &Bars, prm: &Params) -> Output {
    let (s, l) = (one("hv", b, &[("length", p(prm, "short"))])?, one("hv", b, &[("length", p(prm, "long"))])?);
    Ok(vec![map(b.len(), |i| div(s[i], l[i]))])
}
fn donchian_width(b: &Bars, prm: &Params) -> Output { let d = comp("donchian", b, &[("length", p(prm, "length"))])?; Ok(vec![map(b.len(), |i| d[0][i] - d[1][i])]) }
fn kc_width(b: &Bars, prm: &Params) -> Output {
    let k = n(prm, "length");
    let (basis, rng) = (ta::ema(&b.close, k), ta::ema(&ta::tr(b, false), k));
    let m = p(prm, "mult");
    Ok(vec![map(b.len(), |i| div((basis[i] + rng[i] * m) - (basis[i] - rng[i] * m), basis[i]))])
}
fn pdist(b: &Bars, _: &Params) -> Output {
    Ok(vec![map(b.len(), |i| 2. * (b.high[i] - b.low[i]) + (b.open[i] - back(&b.close, i, 1)).abs() - (b.close[i] - b.open[i]).abs())])
}

// ---- Statistics ----
fn rsquared(b: &Bars, prm: &Params) -> Output { Ok(vec![one("trend_strength", b, &[("length", p(prm, "length"))])?.iter().map(|r| r * r).collect()]) }
fn variance(b: &Bars, prm: &Params) -> Output { Ok(vec![one("stdev", b, &[("length", p(prm, "length"))])?.iter().map(|s| s * s).collect()]) }
fn summation(b: &Bars, prm: &Params) -> Output { Ok(vec![ta::sum(&b.close, n(prm, "length"))]) }
fn drawdown(b: &Bars, _: &Params) -> Output {
    let mut mx = NA;
    let peak: Vec<f64> = b.close.iter().map(|&c| { if !c.is_nan() && (mx.is_nan() || c > mx) { mx = c; } mx }).collect();
    let c = &b.close;
    Ok(vec![map(b.len(), |i| peak[i] - c[i]), map(b.len(), |i| 1. - div(c[i], peak[i])), map(b.len(), |i| peak[i].ln() - c[i].ln())])
}
fn streak(b: &Bars, _: &Params) -> Output {
    let (m, c) = (b.len(), &b.close);
    let (mut up, mut dn) = (vec![NA; m], vec![NA; m]);
    for i in 1..m {
        up[i] = if c[i] > c[i - 1] { ta::nz(up[i - 1]) + 1. } else { 0. };
        dn[i] = if c[i] < c[i - 1] { ta::nz(dn[i - 1]) + 1. } else { 0. };
    }
    Ok(vec![up, dn])
}

// ---- Momentum ----
fn fosc(b: &Bars, prm: &Params) -> Output { let t = one("tsf", b, &[("length", p(prm, "length"))])?; Ok(vec![map(b.len(), |i| div(100. * (b.close[i] - back(&t, i, 1)), b.close[i]))]) }
fn disparity(b: &Bars, prm: &Params) -> Output {
    let m = ma(&b.close, n(prm, "length"), p(prm, "exponential") == 1.);
    let v = p(prm, "variant");
    Ok(vec![map(b.len(), |i| { let c = b.close[i]; if v == 1. { div(100. * (c - m[i]), c) } else if v == 2. { div(100. * c, m[i]) } else { div(100. * (c - m[i]), m[i]) } })])
}
fn apo(b: &Bars, prm: &Params) -> Output {
    let e = p(prm, "exponential") == 1.;
    let (f, s) = (ma(&b.close, n(prm, "fast"), e), ma(&b.close, n(prm, "slow"), e));
    Ok(vec![map(b.len(), |i| f[i] - s[i])])
}
fn slope(b: &Bars, prm: &Params) -> Output { let k = n(prm, "length"); Ok(vec![ta::change(&b.close, k).iter().map(|v| v / k as f64).collect()]) }
fn bw_zone(b: &Bars, prm: &Params) -> Output {
    let (f, s) = (p(prm, "fast"), p(prm, "slow"));
    let ao = one("ao", b, &[("fast", f), ("slow", s)])?;
    let ac = one("accel_osc", b, &[("fast", f), ("slow", s), ("smooth", p(prm, "smooth"))])?;
    Ok(vec![map(b.len(), |i| both_dir(&ac, &ao, i))])
}
fn gator(b: &Bars, prm: &Params) -> Output {
    let a = run(&get("alligator").expect("alligator in catalogue"), b, prm)?;
    Ok(vec![map(b.len(), |i| (a[0][i] - a[1][i]).abs()), map(b.len(), |i| -(a[1][i] - a[2][i]).abs())])
}

// ---- Price functions / Moving averages / Bands ----
fn wclprice(b: &Bars, _: &Params) -> Output { Ok(vec![map(b.len(), |i| (b.high[i] + b.low[i] + 2. * b.close[i]) / 4.)]) }
fn midpoint(b: &Bars, prm: &Params) -> Output { let k = n(prm, "length"); let (h, l) = (ta::highest(&b.close, k), ta::lowest(&b.close, k)); Ok(vec![map(b.len(), |i| (h[i] + l[i]) / 2.)]) }
fn hl_range(b: &Bars, prm: &Params) -> Output { let k = n(prm, "length"); let (h, l) = (ta::highest(&b.high, k), ta::lowest(&b.low, k)); Ok(vec![map(b.len(), |i| h[i] - l[i])]) }
fn ibs(b: &Bars, _: &Params) -> Output {
    let (h, l, c) = (&b.high, &b.low, &b.close);
    Ok(vec![map(b.len(), |i| div(c[i] - l[i], h[i] - l[i])), map(b.len(), |i| div((c[i] - l[i]) - (h[i] - c[i]), h[i] - l[i]))])
}
fn change_from_open(b: &Bars, _: &Params) -> Output { Ok(vec![map(b.len(), |i| div(100. * (b.close[i] - b.open[i]), b.open[i]))]) }
fn high_low_52w(b: &Bars, prm: &Params) -> Output { let k = n(prm, "length"); Ok(vec![ta::highest(&b.high, k), ta::lowest(&b.low, k)]) }
fn lookup_extreme(b: &Bars, prm: &Params) -> Output {
    let k = p(prm, "length");
    let at = |bars: Vec<f64>| map(b.len(), |i| if bars[i].is_nan() { NA } else { b.close[i - bars[i] as usize] });
    Ok(vec![at(one("highest_bars", b, &[("length", k)])?), at(one("lowest_bars", b, &[("length", k)])?)])
}
fn all_time_hl(b: &Bars, _: &Params) -> Output {
    let ext = |x: &[f64], hi: bool| { let mut e = NA; x.iter().map(|&v| { if !v.is_nan() && (e.is_nan() || (hi && v > e) || (!hi && v < e)) { e = v; } e }).collect::<Vec<f64>>() };
    Ok(vec![ext(&b.high, true), ext(&b.low, false)])
}
fn cpr(b: &Bars, prm: &Params) -> Output {
    let m = comp("mtp", b, &[("period", p(prm, "period"))])?;
    let pv = map(b.len(), |i| (m[3][i] + m[4][i] + m[5][i]) / 3.);
    let bc = map(b.len(), |i| (m[3][i] + m[4][i]) / 2.);
    let tc = map(b.len(), |i| 2. * pv[i] - bc[i]);
    Ok(vec![pv, bc, tc])
}
fn dma(b: &Bars, prm: &Params) -> Output { let (m, d) = (ma(&b.close, n(prm, "length"), p(prm, "exponential") == 1.), n(prm, "displace")); Ok(vec![map(b.len(), |i| back(&m, i, d))]) }
fn aberration(b: &Bars, prm: &Params) -> Output {
    let (zg, a) = (ta::sma(&b.hlc3(), n(prm, "length")), one("atr", b, &[("length", p(prm, "atr_length"))])?);
    Ok(vec![zg.clone(), map(b.len(), |i| zg[i] + a[i]), map(b.len(), |i| zg[i] - a[i])])
}
fn fractal_chaos_bands(b: &Bars, prm: &Params) -> Output {
    let f = comp("fractals", b, &[("periods", p(prm, "periods"))])?;
    let hold = |x: &[f64]| { let mut last = NA; x.iter().map(|&v| { if !v.is_nan() { last = v; } last }).collect::<Vec<f64>>() };
    Ok(vec![hold(&f[0]), hold(&f[1])])
}

// ---- Second batch (formulas checked against the platform pages / Traders' Tips code) ----
fn rising(x: &[f64], i: usize) -> f64 { let d = x[i] - back(x, i, 1); if d.is_nan() { NA } else if d > 0. { 1. } else { 0. } }
fn cam(b: &Bars, prm: &Params) -> Output {
    let a = p(prm, "adx_length");
    let adx = adx_of(b, a, a)?;
    let m = one("macd", b, &[("fast", p(prm, "fast")), ("slow", p(prm, "slow"))])?;
    Ok(vec![map(b.len(), |i| {
        let (ar, mr) = (rising(&adx, i), rising(&m, i));
        if ar.is_nan() || mr.is_nan() { NA } else { match (ar == 1., mr == 1.) { (true, true) => 1., (false, false) => 2., (true, false) => 3., (false, true) => 4. } }
    })])
}
fn gann_trend_osc(b: &Bars, _: &Params) -> Output {
    let (h, l) = (&b.high, &b.low);
    Ok(vec![map(b.len(), |i| if i == 0 { NA } else if h[i] > h[i - 1] && l[i] > l[i - 1] { 1. } else if h[i] < h[i - 1] && l[i] < l[i - 1] { -1. } else { 0. })])
}
fn ttm_trend(b: &Bars, prm: &Params) -> Output {
    let avg = ta::sma(&b.hl2(), n(prm, "length"));
    Ok(vec![map(b.len(), |i| if avg[i].is_nan() { NA } else if b.close[i] > avg[i] { 1. } else { -1. })])
}
fn amat(b: &Bars, prm: &Params) -> Output {
    let (f, s, k) = (ta::ema(&b.close, n(prm, "fast")), ta::ema(&b.close, n(prm, "slow")), n(prm, "lookback"));
    let dir = |x: &[f64], i: usize| { let d = x[i] - back(x, i, k); if d.is_nan() { NA } else if d > 0. { 1. } else if d < 0. { -1. } else { 0. } };
    let run_of = |up: f64| map(b.len(), |i| { let (df, ds) = (dir(&f, i), dir(&s, i)); if df.is_nan() || ds.is_nan() { NA } else if df == up && ds != 0. { 1. } else { 0. } });
    Ok(vec![run_of(1.), run_of(-1.)])
}
fn mhl_ma(b: &Bars, prm: &Params) -> Output {
    let (k, hl) = (n(prm, "length"), n(prm, "hl_length"));
    let (h, l) = (ta::highest(&b.high, hl), ta::lowest(&b.low, hl));
    Ok(vec![ta::sma(&map(b.len(), |i| (h[i] + l[i]) / 2.), k), ta::sma(&b.close, k)])
}
fn pain(b: &Bars, _: &Params) -> Output { let (o, h, l, c) = (&b.open, &b.high, &b.low, &b.close); Ok(vec![map(b.len(), |i| ((c[i] - o[i]) + (c[i] - l[i]) + (c[i] - h[i])) / 2.)]) }
fn market_structure(b: &Bars, _: &Params) -> Output {
    let (o, h, l, c) = (&b.open, &b.high, &b.low, &b.close);
    // pattern centred on bar t = i − 1, confirmed by bar i
    let msh = map(b.len(), |i| if i >= 2 && o[i - 2] < c[i - 2] && o[i - 1] < c[i - 1] && o[i] > c[i] && h[i - 2] < h[i - 1] && h[i] < h[i - 1] { h[i - 1] } else { NA });
    let msl = map(b.len(), |i| if i >= 2 && o[i - 2] > c[i - 2] && o[i - 1] > c[i - 1] && o[i] < c[i] && l[i - 2] > l[i - 1] && l[i] > l[i - 1] { l[i - 1] } else { NA });
    Ok(vec![msh, msl])
}
fn linreg_reversal(b: &Bars, prm: &Params) -> Output {
    let lr = one("lsma", b, &[("length", p(prm, "length"))])?;
    Ok(vec![map(b.len(), |i| { let d = lr[i] - back(&lr, i, 1); if d.is_nan() { NA } else if d >= 0. { 1. } else { -1. } })])
}
fn dmi_stoch(b: &Bars, prm: &Params) -> Output {
    let d = comp("dmi", b, &[("di_length", p(prm, "length"))])?;
    let osc = map(b.len(), |i| d[0][i] - d[1][i]);
    let (hl, sl) = (n(prm, "hl_length"), n(prm, "sum_length"));
    let (hh, ll) = (ta::highest(&osc, hl), ta::lowest(&osc, hl));
    let (num, den) = (ta::sum(&map(b.len(), |i| osc[i] - ll[i]), sl), ta::sum(&map(b.len(), |i| hh[i] - ll[i]), sl));
    let st = map(b.len(), |i| div(num[i], den[i]) * 100.);
    let avg = ta::sma(&st, n(prm, "avg_length"));
    Ok(vec![st, avg])
}
fn mom_dots(b: &Bars, prm: &Params) -> Output {
    let m = one("mom", b, &[("length", p(prm, "length"))])?;
    Ok(vec![map(b.len(), |i| { let d = m[i] - back(&m, i, 1); if d.is_nan() { NA } else if d > 0. { b.high[i] } else { b.low[i] } })])
}
fn reverse_macd(b: &Bars, prm: &Params) -> Output {
    let (fl, sl) = (p(prm, "fast"), p(prm, "slow"));
    let (ef, es) = (one("ema", b, &[("length", fl)])?, one("ema", b, &[("length", sl)])?);
    let (fc, sc, lvl) = (2. / (1. + fl), 2. / (1. + sl), p(prm, "level"));
    // fast = slow makes fc = sc: no price moves the MACD
    let eq = map(b.len(), |i| div(back(&ef, i, 1) * fc - back(&es, i, 1) * sc, fc - sc));
    let level = map(b.len(), |i| div(lvl - back(&ef, i, 1) * (1. - fc) + back(&es, i, 1) * (1. - sc), fc - sc));
    let avg = ta::ema(&eq, n(prm, "signal"));
    Ok(vec![eq, level, avg])
}
fn weekly_daily_macd(b: &Bars, prm: &Params) -> Output {
    let d = one("macd", b, &[("fast", p(prm, "fast")), ("slow", p(prm, "slow"))])?;
    let w = one("macd", b, &[("fast", p(prm, "weekly_fast")), ("slow", p(prm, "weekly_slow"))])?;
    Ok(vec![map(b.len(), |i| d[i] + w[i]), d, w])
}
fn range_compression(b: &Bars, prm: &Params) -> Output {
    let k = n(prm, "length");
    let r = if p(prm, "mode") == 1. { one("atr", b, &[("length", k as f64)])? } else { let (h, l) = (ta::highest(&b.high, k), ta::lowest(&b.low, k)); map(b.len(), |i| h[i] - l[i]) };
    let ratio = map(b.len(), |i| div(100. * r[i], back(&r, i, k)));
    let f = p(prm, "factor");
    let comp_flag = map(b.len(), |i| flag(ratio[i] < f, &[ratio[i]]));
    Ok(vec![ratio, comp_flag])
}
fn fractal_chaos_osc(b: &Bars, _: &Params) -> Output {
    let fractal = |x: &[f64], i: usize, hi: bool| {
        let beats = |a: f64, c: f64| if hi { a > c } else { a < c };
        let cand = x[i - 2];
        if !(beats(cand, x[i - 1]) && beats(cand, x[i])) { return false; }
        let mut lower = 0;
        for j in (0..i - 2).rev() { if beats(cand, x[j]) { lower += 1; if lower == 2 { return true; } } else if beats(x[j], cand) { return false; } }
        false
    };
    Ok(vec![map(b.len(), |i| if i < 4 { NA } else if fractal(&b.low, i, false) { -1. } else if fractal(&b.high, i, true) { 1. } else { 0. })])
}
fn cycle_trend(b: &Bars, _: &Params) -> Output {
    let mas: Vec<Vec<f64>> = (5..=30).map(|k| ta::sma(&b.close, k)).collect();
    let mut out: Vec<Vec<f64>> = mas.iter().map(|m| map(b.len(), |i| b.close[i] - m[i])).collect();
    out.push(map(b.len(), |i| mas[0][i] - mas[25][i]));
    Ok(out)
}
fn tirone(b: &Bars, prm: &Params) -> Output {
    let k = n(prm, "length");
    let (hh, ll) = (ta::highest(&b.high, k), ta::lowest(&b.low, k));
    let am = map(b.len(), |i| (hh[i] + ll[i] + b.close[i]) / 3.);
    Ok(vec![map(b.len(), |i| hh[i] - (hh[i] - ll[i]) / 3.), map(b.len(), |i| ll[i] + (hh[i] - ll[i]) / 2.), map(b.len(), |i| ll[i] + (hh[i] - ll[i]) / 3.),
        map(b.len(), |i| am[i] + (hh[i] - ll[i])), map(b.len(), |i| 2. * am[i] - ll[i]), am.clone(), map(b.len(), |i| 2. * am[i] - hh[i]), map(b.len(), |i| am[i] - (hh[i] - ll[i]))])
}
fn quadrant(b: &Bars, prm: &Params) -> Output {
    let k = n(prm, "length");
    let (hh, ll) = (ta::highest(&b.high, k), ta::lowest(&b.low, k));
    Ok((0..5).map(|q| map(b.len(), |i| ll[i] + q as f64 * (hh[i] - ll[i]) / 4.)).collect())
}
fn volume_accumulation(b: &Bars, _: &Params) -> Output { let v = vol(b); Ok(vec![map(b.len(), |i| v[i] * (b.close[i] - (b.high[i] + b.low[i]) / 2.))]) }

// ---- Volume ----
fn bw_mfi(b: &Bars, _: &Params) -> Output {
    let v = vol(b);
    let mfi = map(b.len(), |i| div(b.high[i] - b.low[i], v[i]));
    let state = map(b.len(), |i| {
        let (dm, dv) = (mfi[i] - back(&mfi, i, 1), v[i] - back(v, i, 1));
        if dm.is_nan() || dv.is_nan() { NA } else if dm > 0. && dv > 0. { 1. } else if dm < 0. && dv < 0. { 2. } else if dm > 0. && dv < 0. { 3. } else if dm < 0. && dv > 0. { 4. } else { 0. }
    });
    Ok(vec![mfi, state])
}
fn rvol(b: &Bars, prm: &Params) -> Output {
    let v = vol(b);
    let a = ta::sma(v, n(prm, "length"));
    let pct = p(prm, "variant") == 1.;
    Ok(vec![map(b.len(), |i| if pct { div(100. * v[i], a[i]) } else { div(v[i], back(&a, i, 1)) })])
}
fn vol_per_range(b: &Bars, _: &Params) -> Output {
    let (v, tr) = (vol(b), one("atr", b, &[("length", 1.)])?);
    Ok(vec![map(b.len(), |i| div(v[i], tr[i])), map(b.len(), |i| div(v[i], b.high[i] - b.low[i]))])
}
fn turnover(b: &Bars, _: &Params) -> Output { let v = vol(b); Ok(vec![map(b.len(), |i| b.close[i] * v[i])]) }
fn wvad(b: &Bars, _: &Params) -> Output { let v = vol(b); Ok(vec![map(b.len(), |i| div(b.close[i] - b.open[i], b.high[i] - b.low[i]) * v[i])]) }

// ---- Patterns ----
fn key_reversal(b: &Bars, prm: &Params) -> Output {
    let k = n(prm, "length");
    let (lo, hi, c) = (ta::lowest(&b.low, k), ta::highest(&b.high, k), &b.close);
    Ok(vec![
        map(b.len(), |i| { let (l1, c1) = (back(&lo, i, 1), back(c, i, 1)); if i <= k { NA } else { flag(b.low[i] < l1 && c[i] > c1, &[l1, c1]) } }),
        map(b.len(), |i| { let (h1, c1) = (back(&hi, i, 1), back(c, i, 1)); if i <= k { NA } else { flag(b.high[i] > h1 && c[i] < c1, &[h1, c1]) } }),
    ])
}
fn consecutive(b: &Bars, prm: &Params) -> Output {
    let (k, c) = (n(prm, "length"), &b.close);
    let run_of = |up: bool| map(b.len(), |i| if i < k { NA } else if (0..k).all(|j| if up { c[i - j] > c[i - j - 1] } else { c[i - j] < c[i - j - 1] }) { 1. } else { 0. });
    Ok(vec![run_of(true), run_of(false)])
}
fn narrow_range(b: &Bars, prm: &Params) -> Output {
    let k = n(prm, "length");
    let r = map(b.len(), |i| b.high[i] - b.low[i]);
    let (lo, hi) = (ta::lowest(&r, k), ta::highest(&r, k));
    Ok(vec![map(b.len(), |i| flag(r[i] <= lo[i], &[lo[i]])), map(b.len(), |i| flag(r[i] >= hi[i], &[hi[i]]))])
}

// ---- Breadth ----
fn abi(b: &Bars, _: &Params) -> Output { let r = br(b); Ok(vec![map(b.len(), |i| (r.adv[i] - r.dec[i]).abs())]) }
fn market_thrust(b: &Bars, _: &Params) -> Output { let r = br(b); Ok(vec![map(b.len(), |i| r.adv[i] * r.adv_vol[i] - r.dec[i] * r.dec_vol[i])]) }
fn ud_vol_ratio(b: &Bars, _: &Params) -> Output { let r = br(b); Ok(vec![map(b.len(), |i| div(r.adv_vol[i], r.dec_vol[i]))]) }
fn breadth_obos(b: &Bars, prm: &Params) -> Output { let r = br(b); Ok(vec![ta::sma(&map(b.len(), |i| r.adv[i] - r.dec[i]), n(prm, "length"))]) }
fn trin_open(b: &Bars, prm: &Params) -> Output {
    let (r, k) = (br(b), n(prm, "length"));
    let (a, d, av, dv) = (ta::sum(&r.adv, k), ta::sum(&r.dec, k), ta::sum(&r.adv_vol, k), ta::sum(&r.dec_vol, k));
    Ok(vec![map(b.len(), |i| div(div(a[i], d[i]), div(av[i], dv[i])))])
}

const CAM: &[ParamSpec] = &[len("adx_length", "ADX length", 10.), len("fast", "MACD fast", 12.), len("slow", "MACD slow", 26.)];
const L6: &[ParamSpec] = &[len("length", "Length", 6.)];
const AMAT: &[ParamSpec] = &[len("fast", "Fast EMA", 8.), len("slow", "Slow EMA", 21.), len("lookback", "Lookback", 2.)];
const MHL: &[ParamSpec] = &[len("length", "MA length", 50.), len("hl_length", "High/low length", 10.)];
const L25: &[ParamSpec] = &[int("length", "Length", 25., 2., 2000.)]; // length 1: lsma = close
const DMS: &[ParamSpec] = &[len("length", "DMI length", 10.), len("hl_length", "High/low length", 3.), len("sum_length", "Sum length", 3.), len("avg_length", "Average length", 3.)];
const L10M: &[ParamSpec] = &[len("length", "Momentum length", 10.)];
const RM: &[ParamSpec] = &[len("fast", "Fast", 12.), len("slow", "Slow", 26.), len("signal", "EMA of the price", 9.), real("level", "MACD level", 0., -1e9, 1e9, 0.01)];
const WDM: &[ParamSpec] = &[len("fast", "Daily fast", 12.), len("slow", "Daily slow", 26.), len("weekly_fast", "Weekly fast", 60.), len("weekly_slow", "Weekly slow", 130.)];
const RC: &[ParamSpec] = &[len("length", "Length", 4.), real("factor", "Compression below (%)", 75., 1., 1000., 1.), int("mode", "Mode (0 high-low range, 1 ATR)", 0., 0., 1.)];
const NONE: &[ParamSpec] = &[];
const DI: &[ParamSpec] = &[len("di_length", "DI length", 14.)];
const ADXR: &[ParamSpec] = &[len("di_length", "DI length", 14.), len("adx_smoothing", "ADX smoothing", 14.), len("lag", "Lag (13 TA-Lib, 14 Wilder/NinjaTrader)", 13.)];
const CSI: &[ParamSpec] = &[len("length", "Length", 14.), len("lag", "ADXR lag", 13.), real("point_value", "Value of a 1-point move", 1., 0.0001, 1e9, 0.0001),
    real("margin", "Margin", 1., 0.0001, 1e12, 0.0001), real("commission", "Commission", 0., 0., 1e9, 0.01)];
const EI: &[ParamSpec] = &[len("ema", "EMA length", 13.), len("fast", "MACD fast", 12.), len("slow", "MACD slow", 26.), len("signal", "MACD signal", 9.)];
const LIM: &[ParamSpec] = &[real("limit", "Limit move", 10., 0.01, 1000000., 0.01)];
const L14: &[ParamSpec] = &[len("length", "Length", 14.)];
const L14R: &[ParamSpec] = &[int("length", "Length", 14., 2., 2000.)]; // regression on one bar is undefined (trend_strength, tsf)
const L20: &[ParamSpec] = &[len("length", "Length", 20.)];
const L1: &[ParamSpec] = &[len("length", "Length", 1.)];
const HVR: &[ParamSpec] = &[int("short", "Short length", 6., 2., 2000.), int("long", "Long length", 100., 2., 2000.)]; // length 1: hv needs 2 returns
const KCW: &[ParamSpec] = &[len("length", "Length", 20.), real("mult", "Multiplier", 2., 0.1, 10., 0.1)];
const DISP: &[ParamSpec] = &[len("length", "Length", 14.), int("exponential", "Exponential (0 SMA, 1 EMA)", 0., 0., 1.),
    int("variant", "Variant (0 % of MA, 1 % of close (NinjaTrader), 2 close/MA×100 (TradeStation))", 0., 0., 2.)];
const APO: &[ParamSpec] = &[len("fast", "Fast", 12.), len("slow", "Slow", 26.), int("exponential", "Exponential (0 SMA, 1 EMA)", 0., 0., 1.)];
const BWZ: &[ParamSpec] = &[len("fast", "AO fast", 5.), len("slow", "AO slow", 34.), len("smooth", "AC smoothing", 5.)];
const GATOR: &[ParamSpec] = &[len("jaw_length", "Jaw length", 13.), len("teeth_length", "Teeth length", 8.), len("lips_length", "Lips length", 5.),
    int("jaw_offset", "Jaw offset", 8., 0., 100.), int("teeth_offset", "Teeth offset", 5., 0., 100.), int("lips_offset", "Lips offset", 3., 0., 100.)];
const W52: &[ParamSpec] = &[len("length", "Length (bars)", 252.)];
const CPR: &[ParamSpec] = &[int("period", "Period (1 week, 2 month, 3 quarter, 4 year)", 1., 1., 4.)];
const DMA: &[ParamSpec] = &[len("length", "Length", 20.), int("displace", "Displacement (bars back)", 5., 0., 500.), int("exponential", "Exponential (0 SMA, 1 EMA)", 0., 0., 1.)];
const ABR: &[ParamSpec] = &[len("length", "Length", 5.), len("atr_length", "ATR length", 15.)];
const FR: &[ParamSpec] = &[int("periods", "Periods", 2., 2., 100.)];
const RV: &[ParamSpec] = &[len("length", "Length", 20.), int("variant", "Variant (0 volume/average of the prior n bars, 1 100·volume/average incl. this bar)", 0., 0., 1.)];
const L3: &[ParamSpec] = &[len("length", "Bars", 3.)];
const L7: &[ParamSpec] = &[len("length", "Length", 7.)];
const L10: &[ParamSpec] = &[len("length", "Length", 10.)];

pub const DEFS: &[IndicatorDef] = &[
    def("adxr", "Average Directional Movement Rating (ADXR)", "Trend", ADXR, &["adxr"], PN, NO, "adxr = (adx + adx[lag])/2 with adx from dmi (Wilder, TradingView seeding); lag 13 = n − 1 as TA-Lib/Tulip/pandas-ta, 14 = Wilder/NinjaTrader 8", &[40], false, adxr),
    def("dx", "Directional Movement Index (DX)", "Trend", DI, &["dx"], PN, NO, "100·|+DI − −DI|/(+DI + −DI) from dmi (zero sum → divide by 1, as dmi's ADX; rma(dx, n) = adx) (TA-Lib DX, Tulip dx)", &[14], false, dx),
    def("di_osc", "DI Oscillator", "Trend", DI, &["osc"], PN, NO, "+DI − −DI from dmi (thinkorswim DMI_Oscillator; reversal alerts = crosses of 0)", &[14], false, di_osc),
    def("csi", "Commodity Selection Index (Wilder)", "Trend", CSI, &["csi"], PN, NO, "Wilder: csi = adxr·atr(n)·(point_value/√margin)/(150 + commission)·100, adxr as the adxr id (lag param), atr Wilder; neutral defaults 1/1/0 give a scale-only CSI — enter the contract's point value, margin and commission to compare contracts", &[40], false, csi),
    def("elder_impulse", "Elder Impulse System", "Trend", EI, &["impulse"], PN, NO, "Elder: +1 (green) when ema(close, 13) and macd(12, 26, 9).hist both rose from the previous bar, −1 (red) when both fell, else 0 (blue)", &[34], false, elder_impulse),
    def("swing_index", "Swing Index", "Trend", LIM, &["si"], PN, NO, "Wilder per-bar Swing Index = change(asi, 1) (bar 1: asi[1]) with the same limit move", &[1], false, swing_index),
    def("natr", "Normalized ATR", "Volatility", L14, &["natr"], PN, NO, "100·atr(n)/close, atr Wilder (TA-Lib NATR)", &[13], false, natr),
    def("hv_ratio", "Historical Volatility Ratio", "Volatility", HVR, &["ratio"], PN, NO, "hv(short)/hv(long) (Sierra Chart 6/100); zero long hv → NaN", &[100], false, hv_ratio),
    def("donchian_width", "Donchian Channel Width", "Volatility", L20, &["width"], PN, NO, "highest(high, n) − lowest(low, n) = donchian.upper − donchian.lower", &[19], false, donchian_width),
    def("kc_width", "Keltner Channel Width", "Volatility", KCW, &["width"], PN, NO, "Pine ta.kcw(close, n, mult, true): basis = ema(close, n), r = ema(ta.tr, n) (bar 0 TR na); ((basis + mult·r) − (basis − mult·r))/basis", &[20], false, kc_width),
    def("pdist", "Price Distance", "Volatility", NONE, &["pdist"], PN, NO, "pandas-ta pdist: 2·(high − low) + |open − close[1]| − |close − open| (pandas-ta's zero-range epsilon not added)", &[1], false, pdist),
    def("rsquared", "R-Squared", "Statistics", L14R, &["r2"], PN, NO, "trend_strength² (Pearson r of close with bar index over n, squared)", &[13], false, rsquared),
    def("variance", "Variance", "Statistics", L20, &["var"], PN, NO, "population variance = stdev(close, n)² (Pine ta.variance biased, TA-Lib VAR nbdev 1)", &[19], false, variance),
    def("summation", "Summation", "Statistics", L14, &["sum"], PN, NO, "sum(close, n) (NinjaTrader 8 SUM, AmiBroker Sum)", &[13], false, summation),
    def("drawdown", "Drawdown", "Statistics", NONE, &["dd", "dd_pct", "dd_log"], PN, NO, "pandas-ta drawdown: peak = running max of close since the first loaded bar; dd = peak − close; dd_pct = 1 − close/peak; dd_log = ln(peak) − ln(close)", &[0, 0, 0], false, drawdown),
    def("streak", "Up/Down Streak", "Statistics", NONE, &["up", "down"], PN, NO, "consecutive closes above (below) the previous close, 0 on a bar that breaks the run; counted from bar 1", &[1, 1], false, streak),
    def("fosc", "Forecast Oscillator", "Momentum", L14R, &["fosc"], PN, NO, "Chande: 100·(close − tsf[1])/close, tsf = the time series forecast made on the previous bar (Tulip fosc)", &[14], false, fosc),
    def("disparity", "Disparity Index", "Momentum", DISP, &["disparity"], PN, NO, "variant 0: 100·(close − ma)/ma (Disparity Index, BIAS, Kairi, MA deviation); 1: 100·(close − ma)/close (NinjaTrader 8); 2: 100·close/ma (TradeStation Rate of Deviation); ma = sma or ema(close, n)", &[13], false, disparity),
    def("apo", "Absolute Price Oscillator", "Momentum", APO, &["apo"], PN, NO, "ma(close, fast) − ma(close, slow) in points, ma = sma (TA-Lib APO default) or ema", &[25], false, apo),
    def("slope", "Slope", "Momentum", L1, &["slope"], PN, NO, "pandas-ta slope: (close − close[n])/n", &[1], false, slope),
    def("bw_zone", "Bill Williams Zone Trade", "Momentum", BWZ, &["zone"], PN, NO, "+1 (green) when accel_osc and ao both rose from the previous bar, −1 (red) when both fell, else 0 (grey)", &[38], false, bw_zone),
    def("gator", "Gator Oscillator", "Momentum", GATOR, &["upper", "lower"], PN, NO, "Bill Williams: upper = |jaw − teeth|, lower = −|teeth − lips| from the alligator lines as plotted (offsets applied)", &[20, 12], false, gator),
    def("wclprice", "Weighted Close", "Price functions", NONE, &["wcl"], PR, NO, "(high + low + 2·close)/4 (hlcc4; TA-Lib WCLPRICE)", &[0], false, wclprice),
    def("midpoint", "MidPoint over Period", "Price functions", L14, &["midpoint"], PR, NO, "(highest(close, n) + lowest(close, n))/2 (TA-Lib MIDPOINT)", &[13], false, midpoint),
    def("hl_range", "Range", "Price functions", L1, &["range"], PN, NO, "highest(high, n) − lowest(low, n); n = 1 is the bar's high − low (NinjaTrader Range, ChartIQ High Minus Low)", &[0], false, hl_range),
    def("ibs", "Internal Bar Strength", "Price functions", NONE, &["ibs", "clv"], PN, NO, "ibs = (close − low)/(high − low); clv = ((close − low) − (high − close))/(high − low) = 2·ibs − 1; zero range → NaN", &[0, 0], false, ibs),
    def("change_from_open", "Percent Change from Open", "Price functions", NONE, &["pct"], PN, NO, "100·(close − open)/open of the bar", &[0], false, change_from_open),
    def("high_low_52w", "52 Week High/Low", "Price functions", W52, &["high", "low"], PR, NO, "highest(high, 252), lowest(low, 252) bars (declared window; TradingView's exact window is unpublished)", &[251, 251], false, high_low_52w),
    def("lookup_extreme", "Close at Highest/Lowest Bar", "Price functions", L14, &["at_high", "at_low"], PR, NO, "thinkorswim LookUpHighest/LookUpLowest: close on the bar of the highest high (lowest low) of the last n bars (default 14; ties → the most recent, as highest_bars — thinkorswim does not document ties)", &[13, 13], false, lookup_extreme),
    def("all_time_hl", "All-Time High/Low", "Price functions", NONE, &["high", "low"], PR, NO, "running max of high and min of low since the first loaded bar (AmiBroker Highest/Lowest)", &[0, 0], false, all_time_hl),
    def("cpr", "Central Pivot Range", "Periods & calendar", CPR, &["pivot", "bc", "tc"], PR, NO, "from the previous completed period (mtp): pivot = (H + L + C)/3, bc = (H + L)/2, tc = 2·pivot − bc (tc below bc when close < pivot; not swapped)", &[5, 5, 5], false, cpr),
    def("dma", "Displaced Moving Average", "Moving averages", DMA, &["dma"], PR, NO, "ma(close, n)[d]: sma or ema shifted d bars back (positive displacement only; negative shifts look ahead)", &[24], false, dma),
    def("aberration", "Aberration", "Bands & channels", ABR, &["zg", "sg", "xg"], PR, NO, "pandas-ta aberration: zg = sma(hlc3, n); sg = zg + atr(atr_length); xg = zg − atr(atr_length) (atr Wilder with TradingView seeding)", &[4, 14, 14], false, aberration),
    def("fractal_chaos_bands", "Fractal Chaos Bands", "Bands & channels", FR, &["upper", "lower"], PR, NO, "upper = high of the latest confirmed up fractal, lower = low of the latest confirmed down fractal (fractals confirm n bars late); NaN until the first fractal", &[usize::MAX, usize::MAX], true, fractal_chaos_bands),
    def("bw_mfi", "Market Facilitation Index", "Volume", NONE, &["mfi", "state"], PN, V, "Bill Williams: mfi = (high − low)/volume; state 1 green (mfi and volume up), 2 fade (both down), 3 fake (mfi up, volume down), 4 squat (mfi down, volume up), 0 when either is unchanged", &[0, 1], false, bw_mfi),
    def("rvol", "Relative Volume", "Volume", RV, &["rvol"], PN, V, "variant 0: volume/sma(volume, n)[1] (average of the n bars before this one); variant 1: 100·volume/sma(volume, n) (volume breakout: variant 1 > 150 with n = 50)", &[20], false, rvol),
    def("vol_per_range", "Volume per Range", "Volume", NONE, &["per_tr", "per_range"], PN, V, "volume/true range (atr(1); bar 0 high − low) and volume/(high − low); zero range → NaN", &[0, 0], false, vol_per_range),
    def("turnover", "Price × Volume (Turnover)", "Volume", NONE, &["turnover"], PN, V, "close·volume", &[0], false, turnover),
    def("wvad", "Williams Variable Accumulation/Distribution", "Volume", NONE, &["wvad"], PN, V, "Pine ta.wvad: (close − open)/(high − low)·volume; zero range → NaN", &[0], false, wvad),
    def("key_reversal", "Key Reversal", "Patterns", L1, &["up", "down"], PN, NO, "up: low < lowest(low, n)[1] and close > close[1]; down: high > highest(high, n)[1] and close < close[1] (NinjaTrader 8 KeyReversalUp/Down, default period 1, evaluated from bar n + 1); 1/0", &[2, 2], false, key_reversal),
    def("consecutive", "Consecutive Up/Down Closes", "Patterns", L3, &["up", "down"], PN, NO, "1 when each of the last n closes is above (below) the close before it, else 0", &[3, 3], false, consecutive),
    def("narrow_range", "Narrow/Wide Range Bar", "Patterns", L7, &["nr", "wr"], PN, NO, "nr: high − low ≤ lowest(high − low, n) (NR7 for n = 7); wr: high − low ≥ highest(high − low, n); windows include this bar; 1/0", &[6, 6], false, narrow_range),
    def("abi", "Absolute Breadth Index", "Breadth", NONE, &["abi"], PN, B, "|advances − declines|", &[0], false, abi),
    def("market_thrust", "Market Thrust", "Breadth", NONE, &["thrust"], PN, B, "advances·advancing volume − declines·declining volume", &[0], false, market_thrust),
    def("ud_vol_ratio", "Upside/Downside Volume Ratio", "Breadth", NONE, &["ratio"], PN, B, "advancing volume/declining volume; zero → NaN", &[0], false, ud_vol_ratio),
    def("breadth_obos", "Overbought/Oversold (Breadth)", "Breadth", L10, &["obos"], PN, B, "sma(advances − declines, n) (MetaStock Overbought/Oversold, n = 10); days without breadth are skipped by the average", &[9], false, breadth_obos),
    def("trin_open", "Open-10 TRIN", "Breadth", L10, &["trin"], PN, B, "(sum(advances, n)/sum(declines, n))/(sum(advancing volume, n)/sum(declining volume, n)); days without breadth are skipped by the sums", &[9], false, trin_open),
    def("cam", "CAM Indicator", "Trend", CAM, &["state"], PN, NO, "Barbara Star (TASC Jan 2018; thinkorswim CAM_Indicator): adx rising (adx > adx[1]) and macd line rising → 1 CAM UP; both not rising → 2 CAM PB; adx rising, macd not → 3 CAM DN; macd rising, adx not → 4 CAM CT; ties count as not rising; adx = dmi(adx_length, adx_length) (Traders' Tips defaults 10, 12/26)", &[26], false, cam),
    def("gann_trend_osc", "Gann Trend Oscillator", "Trend", NONE, &["gto"], PN, NO, "Sierra Chart: +1 when high > high[1] and low > low[1]; −1 when high < high[1] and low < low[1]; else 0", &[1], false, gann_trend_osc),
    def("ttm_trend", "TTM Trend", "Trend", L6, &["trend"], PN, NO, "pandas-ta ttm_trend: +1 when close > average hl2 of the last n bars including this one, else −1 (pandas-ta prints −1 during warm-up; NaN here)", &[5], false, ttm_trend),
    def("amat", "Archer MA Trends", "Trend", AMAT, &["long_run", "short_run"], PN, NO, "pandas-ta amat: f = ema(close, fast), s = ema(close, slow); long_run = f − f[lookback] > 0 and s − s[lookback] ≠ 0; short_run = f − f[lookback] < 0 and s − s[lookback] ≠ 0; 1/0", &[22, 22], false, amat),
    def("mhl_ma", "Middle-High-Low Moving Average", "Moving averages", MHL, &["mhl_ma", "ma"], PR, NO, "Apirine (TASC Aug 2016; thinkorswim MiddleHighLowMA): mhl_ma = sma((highest(high, hl_length) + lowest(low, hl_length))/2, length); ma = sma(close, length) (Traders' Tips defaults 10/50; SMA type)", &[58, 49], false, mhl_ma),
    def("pain", "Price Action Indicator", "Momentum", NONE, &["pain"], PN, NO, "Garety (thinkorswim PriceActionIndicator): ((close − open) + (close − low) + (close − high))/2", &[0], false, pain),
    def("market_structure", "Market Structure Highs/Lows", "Patterns", NONE, &["msh", "msl"], PR, NO, "Sierra Chart MSH/MSL: msh at t when bars t−1 and t close above their open, bar t+1 below, and high[t] above high[t−1] and high[t+1]; msl mirrors with lows; reported on bar t+1 (confirmed one bar late) with the high (low) of bar t; NaN otherwise", &[usize::MAX, usize::MAX], true, market_structure),
    def("linreg_reversal", "Linear Regression Reversal", "Trend", L25, &["lrr"], PN, NO, "thinkorswim LinearRegrReversal: +1 when lsma(close, n) ≥ lsma[1], else −1 (declared default 25 = our lsma; thinkorswim's default unpublished)", &[25], false, linreg_reversal),
    def("dmi_stoch", "DMI Stochastic", "Trend", DMS, &["stoch", "avg"], PN, NO, "thinkorswim DMI_StochasticExtreme / DMI_ReversalAlerts (TASC Jan 2013): osc = +DI − −DI (dmi length); stoch = 100·sum(osc − lowest(osc, 3), 3)/sum(highest(osc, 3) − lowest(osc, 3), 3) (zero → NaN); avg = sma(stoch, 3); extremes = crosses of 10/90, reversal alerts = crosses of avg (highest/lowest restart after NaN as ta.highest)", &[12, 14], false, dmi_stoch),
    def("mom_dots", "Momentum Trend Dots", "Momentum", L10M, &["dot"], PR, NO, "Sierra Chart Momentum Trend: high when mom(close, n) rose from the previous bar, else low (ties → low) (declared default 10 = our mom)", &[11], false, mom_dots),
    def("reverse_macd", "Reverse-Engineered MACD", "Momentum", RM, &["eq", "level", "eq_avg"], PR, NO, "thinkorswim ReverseEngineeringMACD (TASC Jan 2012): fc = 2/(1 + fast), sc = 2/(1 + slow), pf/ps = ema(close, fast/slow)[1]; eq = (pf·fc − ps·sc)/(fc − sc) (close that leaves MACD unchanged); level = (target − pf·(1 − fc) + ps·(1 − sc))/(fc − sc) (close that puts MACD at the level); eq_avg = ema(eq, 9)", &[26, 26, 34], false, reverse_macd),
    def("weekly_daily_macd", "Weekly and Daily MACD", "Momentum", WDM, &["combined", "daily", "weekly"], PN, NO, "Apirine (TASC Dec 2017; thinkorswim WeeklyAndDailyMACD): daily = ema(close, 12) − ema(close, 26); weekly = ema(close, 60) − ema(close, 130); combined = daily + weekly", &[129, 25, 129], false, weekly_daily_macd),
    def("range_compression", "Range Compression (Accumulation/Distribution)", "Volatility", RC, &["ratio", "compressed"], PN, NO, "D'Errico (TASC Aug 2018; thinkorswim AccumulationDistribution): range = highest(high, n) − lowest(low, n) (mode 1: atr(n)); ratio = 100·range/range[n]; compressed = ratio < factor (Traders' Tips 4 and 0.75 → 75 %)", &[7, 7], false, range_compression),
    def("fractal_chaos_osc", "Fractal Chaos Oscillator", "Patterns", NONE, &["fco"], PN, NO, "ChartIQ fractalChaos: on bar i, bar i−2 is a fractal high when its high is above bars i−1 and i and, scanning left (equal highs skipped), two lower highs come before any higher one → +1; fractal low mirrors → −1 (wins when both); else 0; from bar 4", &[4], false, fractal_chaos_osc),
    def("cycle_trend", "Cycle/Trend Analytics", "Cycles", NONE, &["osc5", "osc6", "osc7", "osc8", "osc9", "osc10", "osc11", "osc12", "osc13", "osc14", "osc15", "osc16", "osc17", "osc18", "osc19", "osc20", "osc21", "osc22", "osc23", "osc24", "osc25", "osc26", "osc27", "osc28", "osc29", "osc30", "madiff"], PN, NO, "Ehlers (TASC Oct 2021; thinkorswim CycleTrendAnalytics, trend mode): osc_k = close − sma(close, k) for k = 5..30; madiff = sma(close, 5) − sma(close, 30)", &[4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 29], false, cycle_trend),
    def("tirone", "Tirone Levels", "Bands & channels", L20, &["top", "center", "bottom", "extreme_high", "regular_high", "adjusted_mean", "regular_low", "extreme_low"], PR, NO, "Tirone over the last n bars (hh, ll; declared window — MetaStock draws over a selected range): midpoint method top = hh − (hh − ll)/3, center = ll + (hh − ll)/2, bottom = ll + (hh − ll)/3; mean method am = (hh + ll + close)/3, extreme_high = am + (hh − ll), regular_high = 2·am − ll, regular_low = 2·am − hh, extreme_low = am − (hh − ll)", &[19; 8], false, tirone),
    def("quadrant", "Quadrant Lines", "Bands & channels", L20, &["low", "q1", "mid", "q3", "high"], PR, NO, "lowest(low, n) + k·(highest(high, n) − lowest(low, n))/4 for k = 0..4 (declared n-bar window; the platforms draw over a selected range)", &[19; 5], false, quadrant),
    def("volume_accumulation", "Volume Accumulation", "Volume", NONE, &["va"], PN, V, "thinkorswim VolumeAccumulation: volume·(close − (high + low)/2) per bar (the Learning Center does not say it is cumulative)", &[0], false, volume_accumulation),
];
/// (id, platform names, AmiBroker names): one row per id in DEFS.
pub const SOURCES: &[(&str, &str, &str)] = &[
    ("adxr", "NinjaTrader 8: ADXR; TradeStation: ADXR; thinkorswim: ADXR; Sierra Chart: ADXR; MetaStock: ADXR; cTrader: ADXR; TA-Lib: ADXR; Tulip: adxr; pandas-ta: adx; TA4J: ADXR", ""),
    ("dx", "TA-Lib: DX; Tulip: dx; pandas-ta: dx", ""),
    ("di_osc", "thinkorswim: DMI_Oscillator; thinkorswim: DMIReversalAlerts; Sierra Chart: DMI Oscillator", ""),
    ("csi", "TradeStation: Commodity Selection Index; thinkorswim: CSI; MetaStock: Commodity Selection Index", ""),
    ("elder_impulse", "thinkorswim: ElderImpulse; Sierra Chart: Elder Impulse; ChartIQ: Elder Impulse System; TradingView: Elder Impulse System (community)", ""),
    ("swing_index", "TA-Lib: SI; MetaStock: Swing Index; MultiCharts: SwingIndex; cTrader: Swing Index; ChartIQ: Swing Index; Zerodha Streak: Swing Index", ""),
    ("natr", "TA-Lib: NATR; Tulip: natr; pandas-ta: natr; Sierra Chart: Normalized Average True Range", ""),
    ("hv_ratio", "Sierra Chart: Historical Volatility Ratio", ""),
    ("donchian_width", "ChartIQ: Donchian Width", ""),
    ("kc_width", "TradingView: ta.kcw (Pine); ChartIQ: Keltner Channel Width", ""),
    ("pdist", "pandas-ta: pdist", ""),
    ("rsquared", "NinjaTrader 8: R-squared; Sierra Chart: R Squared; MetaStock: R-Squared; thinkorswim: RSquared; cTrader: R-Squared; ChartIQ: R-Squared", ""),
    ("variance", "TradingView: ta.variance (Pine); TA-Lib: VAR; Tulip: var; MetaStock: Var", ""),
    ("summation", "NinjaTrader 8: SUM", "Sum"),
    ("drawdown", "pandas-ta: drawdown", ""),
    ("streak", "TA4J: StreakIndicator", ""),
    ("fosc", "Tulip: fosc; pandas-ta: fosc; NinjaTrader 8: ChandeForecastOscillator; thinkorswim: ForecastOscillator; Sierra Chart: Forecast Oscillator; MetaStock: Forecast Oscillator; Zerodha Kite (ChartIQ): Forecast Oscillator", ""),
    ("disparity", "NinjaTrader 8: Disparity Index; thinkorswim: DisparityIndex; ChartIQ: Disparity Index; Dhan: Kairi; TA4J: Distance from MA; pandas-ta: bias; TradeStation: Rate of Deviation", ""),
    ("apo", "TA-Lib: APO; Tulip: apo; pandas-ta: apo", ""),
    ("slope", "pandas-ta: slope", ""),
    ("bw_zone", "MetaTrader 5: BW ZoneTrade", ""),
    ("gator", "MetaTrader 4: Gator Oscillator; MetaTrader 5: Gator Oscillator; cTrader: Gator Oscillator; Zerodha Kite (ChartIQ): Gator Oscillator; TA4J: Gator; MultiCharts: Gator; thinkorswim: GatorOscillator", ""),
    ("wclprice", "TradeStation: WeightedClose; thinkorswim: WeightedClose; MetaStock: Weighted Close; cTrader: Weighted Close; TA-Lib: WCLPRICE; Tulip: wcprice; pandas-ta: wcp; ChartIQ: Weighted Close", ""),
    ("midpoint", "TA-Lib: MIDPOINT; pandas-ta: midpoint", ""),
    ("hl_range", "NinjaTrader 8: Range; TradeStation: Range; cTrader: Simple Range; ChartIQ: High Minus Low; Dhan: Range; TradingView: ta.range (Pine)", ""),
    ("ibs", "TA-Lib: Internal Bar Strength; TA4J: CloseLocationValue; thinkorswim: CLV", ""),
    ("change_from_open", "Sierra Chart: Percent Change Since Open; TradeStation: Percent Change from Open", ""),
    ("high_low_52w", "TradingView: 52 Week High/Low (Charting Library)", ""),
    ("lookup_extreme", "thinkorswim: LookUpHighest; thinkorswim: LookUpLowest", ""),
    ("all_time_hl", "", "Highest; Lowest"),
    ("cpr", "Zerodha Kite (ChartIQ): Central Pivot Range; Zerodha Streak: CPR; Dhan: CPR; TradingView: CPR (community)", ""),
    ("dma", "TA4J: DMA; TA4J: EDMA; thinkorswim: Displacer", "Ref(MA(C, n), -d)"),
    ("aberration", "pandas-ta: aberration", ""),
    ("fractal_chaos_bands", "cTrader: Fractal Chaos Bands; ChartIQ: Fractal Chaos Bands; Dhan: Fractal Chaos Bands; Zerodha Streak: Fractal Chaos Bands", ""),
    ("bw_mfi", "MetaTrader 4: Market Facilitation Index; MetaTrader 5: Market Facilitation Index; cTrader: Market Facilitation Index; TradeStation: Market Facilitation Index; thinkorswim: MarketFacilitationIndex; Sierra Chart: Market Facilitation Index; MetaStock: Market Facilitation Index; TA4J: MarketFacilitationIndex; ChartIQ: Market Facilitation Index", ""),
    ("rvol", "TradeStation: Relative Volume; Sierra Chart: Relative Volume; TradeStation: Volume Breakout", ""),
    ("vol_per_range", "thinkorswim: TrueRangeSpecifiedVolume; Sierra Chart: Volume Bar Range Ratio", ""),
    ("turnover", "Dhan: Price Volume", ""),
    ("wvad", "TradingView: ta.wvad (Pine); TradingView: Williams Variable Accumulation/Distribution", ""),
    ("key_reversal", "NinjaTrader 8: KeyReversalUp; NinjaTrader 8: KeyReversalDown; TradeStation: Key Reversal Up/Down; MultiCharts: Key Reversal", ""),
    ("consecutive", "NinjaTrader 8: Consecutive Up/Down; TradeStation: Consecutive Ups/Downs", ""),
    ("narrow_range", "TradeStation: NR7; Sierra Chart: Narrow Range Bar; Sierra Chart: Wide Range Bar", ""),
    ("abi", "MetaStock: Absolute Breadth Index; TradeStation: Absolute Breadth Index", ""),
    ("market_thrust", "TradeStation: Market Thrust; MetaStock: Market Thrust", ""),
    ("ud_vol_ratio", "MetaStock: Upside/Downside Ratio; TradeStation: Upside/Downside Volume", ""),
    ("breadth_obos", "MetaStock: Overbought/Oversold", ""),
    ("trin_open", "MetaStock: Open-10 TRIN; TradeStation: Open 10 TRIN", ""),
    ("cam", "thinkorswim: CAM_Indicator", ""),
    ("gann_trend_osc", "Sierra Chart: Gann Trend Oscillator", ""),
    ("ttm_trend", "pandas-ta: ttm_trend", ""),
    ("amat", "pandas-ta: amat; TA4J: AMAT", ""),
    ("mhl_ma", "thinkorswim: MiddleHighLowMA", ""),
    ("pain", "thinkorswim: PriceActionIndicator", ""),
    ("market_structure", "Sierra Chart: Market Structure MSL/MSH", ""),
    ("linreg_reversal", "thinkorswim: LinearRegrReversal", ""),
    ("dmi_stoch", "thinkorswim: DMI_StochasticExtreme; thinkorswim: DMI_ReversalAlerts", ""),
    ("mom_dots", "Sierra Chart: Momentum Trend", ""),
    ("reverse_macd", "thinkorswim: ReverseEngineeringMACD", ""),
    ("weekly_daily_macd", "thinkorswim: WeeklyAndDailyMACD", ""),
    ("range_compression", "thinkorswim: AccumulationDistribution", ""),
    ("fractal_chaos_osc", "ChartIQ: Fractal Chaos Oscillator; Zerodha Kite (ChartIQ): Fractal Chaos Oscillator; Dhan: Fractal Chaos Oscillator", ""),
    ("cycle_trend", "thinkorswim: CycleTrendAnalytics", ""),
    ("tirone", "MetaStock: Tirone Levels", ""),
    ("quadrant", "MetaStock: Quadrant Lines; thinkorswim: Quadrant Lines", ""),
    ("volume_accumulation", "thinkorswim: VolumeAccumulation", ""),
];

#[cfg(test)]
mod tests {
    use crate::lab::ta;
    use crate::lab::testdata::{assert_series, close_eq, closes, compiles, d1, from_ohlc, go, walk, with_breadth};
    const N: f64 = f64::NAN;
    fn same(a: &[f64], b: &[f64]) { assert_eq!(a.len(), b.len()); for i in 0..a.len() { assert!(close_eq(a[i], b[i]), "bar {i}: {} vs {}", a[i], b[i]); } }
    fn growth() -> crate::lab::bars::Bars { from_ohlc(&(0..300).map(|i| { let c = 100. * 1.01f64.powi(i); (c, c * 1.005, c * 0.995, c) }).collect::<Vec<_>>()) }

    #[test] fn directional_family_matches_dmi() {
        let b = walk(400, 3, false);
        let d = go("dmi", &b, &[]);
        let r = go("adxr", &b, &[]);
        for i in 40..400 { assert!(close_eq(r[0][i], (d[2][i] + d[2][i - 13]) / 2.)); }
        assert!(close_eq(go("adxr", &b, &[("lag", 14.)])[0][100], (d[2][100] + d[2][86]) / 2.));
        let dx = go("dx", &b, &[]).remove(0);
        same(&ta::rma(&dx, 14), &d[2]);
        same(&go("di_osc", &b, &[]).remove(0), &(0..400).map(|i| d[0][i] - d[1][i]).collect::<Vec<_>>());
        // point value 4, margin 4, commission 50: factor 4/2/200·100 = 1
        let (c, atr) = (go("csi", &b, &[("point_value", 4.), ("margin", 4.), ("commission", 50.)]).remove(0), go("atr", &b, &[]).remove(0));
        for i in 40..400 { assert!(close_eq(c[i], r[0][i] * atr[i])); }
    }
    #[test] fn elder_impulse_and_zone_trade() {
        let g = growth();
        assert_eq!(go("elder_impulse", &g, &[])[0][299], 1.);
        assert_eq!(go("bw_zone", &g, &[])[0][299], 1.);
        let b = walk(400, 8, false);
        let (e, h, ei) = (go("ema", &b, &[("length", 13.)]).remove(0), go("macd", &b, &[]).remove(2), go("elder_impulse", &b, &[]).remove(0));
        let (ao, ac, z) = (go("ao", &b, &[]).remove(0), go("accel_osc", &b, &[]).remove(0), go("bw_zone", &b, &[]).remove(0));
        let dir = |a: &[f64], c: &[f64], i: usize| if a[i] > a[i - 1] && c[i] > c[i - 1] { 1. } else if a[i] < a[i - 1] && c[i] < c[i - 1] { -1. } else { 0. };
        for i in 40..400 { assert_eq!(ei[i], dir(&e, &h, i)); assert_eq!(z[i], dir(&ac, &ao, i)); }
        assert!(ei.contains(&1.) && ei.contains(&-1.) && ei.contains(&0.));
    }
    #[test] fn swing_index_sums_to_asi() {
        let b = walk(200, 2, false);
        let (si, asi) = (go("swing_index", &b, &[]).remove(0), go("asi", &b, &[]).remove(0));
        let mut s = 0.;
        for i in 1..200 { s += si[i]; assert!(close_eq(s, asi[i])); }
    }
    #[test] fn volatility_family() {
        let b = walk(300, 4, false);
        let (natr, atr) = (go("natr", &b, &[]).remove(0), go("atr", &b, &[]).remove(0));
        for i in 13..300 { assert!(close_eq(natr[i] * b.close[i] / 100., atr[i])); }
        let (r, s, l) = (go("hv_ratio", &b, &[]).remove(0), go("hv", &b, &[("length", 6.)]).remove(0), go("hv", &b, &[("length", 100.)]).remove(0));
        for i in 100..300 { assert!(close_eq(r[i], s[i] / l[i])); }
        assert_series(&go("donchian_width", &d1(), &[("length", 3.)])[0], &[N, N, 3.5, 2.5, 2.5, 2.7, 2.5, 3.5]);
        // tr (bar 0 na) = [_, 2, 1]; ema2 seeds 1.5 at bar 2; basis ema2(close) = 2 → 2·1.5/2
        assert_series(&go("kc_width", &closes(&[1., 3., 2.]), &[("length", 2.), ("mult", 1.)])[0], &[N, N, 1.5]);
        // bar 1: 2·(12 − 10) + |10.5 − 10.5| − |11.5 − 10.5| = 3
        assert_eq!(go("pdist", &d1(), &[])[0][1], 3.);
    }
    #[test] fn statistics_family() {
        let b = walk(200, 6, false);
        let (r2, r) = (go("rsquared", &b, &[]).remove(0), go("trend_strength", &b, &[]).remove(0));
        for i in 13..200 { assert!(close_eq(r2[i], r[i] * r[i])); }
        assert_series(&go("variance", &closes(&[1., 3.]), &[("length", 2.)])[0], &[N, 1.]);
        assert_series(&go("summation", &closes(&[1., 2., 3.]), &[("length", 2.)])[0], &[N, 3., 5.]);
        let dd = go("drawdown", &closes(&[1., 3., 2.]), &[]);
        assert_series(&dd[0], &[0., 0., 1.]);
        assert_series(&dd[1], &[0., 0., 1. / 3.]);
        assert_series(&dd[2], &[0., 0., 1.5f64.ln()]);
        let st = go("streak", &closes(&[1., 2., 3., 2., 2.]), &[]);
        assert_series(&st[0], &[N, 1., 2., 0., 0.]);
        assert_series(&st[1], &[N, 0., 0., 1., 0.]);
    }
    #[test] fn momentum_family() {
        // on a straight line the forecast made yesterday is today's close
        assert_series(&go("fosc", &closes(&[1., 2., 3., 4., 5.]), &[("length", 3.)])[0], &[N, N, N, 0., 0.]);
        let c = closes(&[1., 2., 3.]);
        assert!(close_eq(go("disparity", &c, &[("length", 2.)])[0][2], 20.));
        assert!(close_eq(go("disparity", &c, &[("length", 2.), ("variant", 1.)])[0][2], 50. / 3.));
        assert!(close_eq(go("disparity", &c, &[("length", 2.), ("variant", 2.)])[0][2], 120.));
        assert_series(&go("apo", &closes(&[1., 2., 4.]), &[("fast", 1.), ("slow", 2.)])[0], &[N, 0.5, 1.]);
        assert_series(&go("slope", &closes(&[1., 3., 7.]), &[("length", 2.)])[0], &[N, N, 3.]);
        let b = walk(200, 9, false);
        let (g, a) = (go("gator", &b, &[]), go("alligator", &b, &[]));
        for i in 20..200 { assert!(close_eq(g[0][i], (a[0][i] - a[1][i]).abs()) && close_eq(g[1][i], -(a[1][i] - a[2][i]).abs())); }
    }
    #[test] fn price_functions() {
        let b = d1();
        assert_eq!(go("wclprice", &b, &[])[0][0], 10.25);
        assert_series(&go("midpoint", &closes(&[1., 3., 2.]), &[("length", 2.)])[0], &[N, 2., 2.5]);
        assert_eq!(go("hl_range", &b, &[])[0][0], 2.);
        assert_eq!(go("hl_range", &b, &[("length", 2.)])[0][1], 3.);
        let s = go("ibs", &b, &[]);
        assert!(close_eq(s[0][0], 0.75) && close_eq(s[1][0], 0.5));
        assert!(close_eq(go("change_from_open", &b, &[])[0][0], 5.));
        let w = go("high_low_52w", &b, &[("length", 3.)]);
        assert!(w[0][1].is_nan() && w[0][2] == 12.5 && w[1][2] == 9.);
        // bar 3, window bars 1..3: highest high 12.5 on bar 2 (close 12), lowest low 10 on bar 1 (close 11.5)
        let lk = go("lookup_extreme", &b, &[("length", 3.)]);
        assert!(lk[0][3] == 12. && lk[1][3] == 11.5);
        let at = go("all_time_hl", &b, &[]);
        assert_series(&at[0], &[11., 12., 12.5, 12.5, 12.5, 12.5, 12.5, 13.]);
        assert_series(&at[1], &[9.; 8]);
        // week 1 = bars 0..6: H 12.5, L 9, C 11.9
        let c = go("cpr", &b, &[]);
        assert!(c[0][6].is_nan());
        assert!(close_eq(c[0][7], 33.4 / 3.) && close_eq(c[1][7], 10.75) && close_eq(c[2][7], 2. * 33.4 / 3. - 10.75));
        assert_series(&go("dma", &closes(&[1., 2., 3., 4.]), &[("length", 2.), ("displace", 1.)])[0], &[N, N, 1.5, 2.5]);
    }
    #[test] fn bands() {
        let b = walk(200, 5, false);
        let (a, z, atr) = (go("aberration", &b, &[]), ta::sma(&b.hlc3(), 5), go("atr", &b, &[("length", 15.)]).remove(0));
        for i in 14..200 { assert!(close_eq(a[0][i], z[i]) && close_eq(a[1][i], z[i] + atr[i]) && close_eq(a[2][i], z[i] - atr[i])); }
        let h = [1., 2., 3., 5., 4., 3., 2., 3., 3., 6., 5., 4.];
        let f = go("fractal_chaos_bands", &from_ohlc(&h.iter().map(|&x| (x, x, x - 1., x)).collect::<Vec<_>>()), &[]);
        assert_series(&f[0], &[N, N, N, N, N, 5., 5., 5., 5., 5., 5., 6.]);
    }
    #[test] fn volume_family() {
        let b = d1();
        let m = go("bw_mfi", &b, &[]);
        assert!(close_eq(m[0][0], 0.02) && m[1][0].is_nan());
        assert_eq!((m[1][1], m[1][3], m[1][4], m[1][6]), (4., 4., 3., 4.));
        assert!(close_eq(go("rvol", &b, &[("length", 2.)])[0][2], 90. / 110.));
        assert!(close_eq(go("rvol", &b, &[("length", 2.), ("variant", 1.)])[0][2], 9000. / 105.));
        let v = go("vol_per_range", &b, &[]);
        assert!(v[0][0] == 50. && v[0][1] == 60. && v[1][1] == 60.);
        assert_eq!(go("turnover", &b, &[])[0][0], 1050.);
        assert!(close_eq(go("wvad", &b, &[])[0][0], 25.));
    }
    #[test] fn bar_patterns() {
        let k = go("key_reversal", &from_ohlc(&[(10., 11., 9., 10.), (10., 11., 9., 10.), (10., 10.5, 8.5, 10.5), (10.5, 11.5, 10., 10.)]), &[]);
        assert_series(&k[0], &[N, N, 1., 0.]);
        assert_series(&k[1], &[N, N, 0., 1.]);
        let c = go("consecutive", &closes(&[1., 2., 3., 4., 3.]), &[]);
        assert_series(&c[0], &[N, N, N, 1., 0.]);
        assert_series(&c[1], &[N, N, N, 0., 0.]);
        // d1 ranges 2, 2, 1.5, 1.4, 1.5, 1.5, 1.5, 1.5
        let r = go("narrow_range", &d1(), &[("length", 2.)]);
        assert_series(&r[0], &[N, 1., 1., 1., 0., 1., 1., 1.]);
        assert_series(&r[1], &[N, 1., 0., 0., 1., 1., 1., 1.]);
    }
    #[test] fn breadth_family() {
        let b = with_breadth(walk(60, 1, true));
        let r = b.breadth.clone().unwrap();
        let (abi, th, ud, ob, tr) = (go("abi", &b, &[]), go("market_thrust", &b, &[]), go("ud_vol_ratio", &b, &[]), go("breadth_obos", &b, &[]), go("trin_open", &b, &[]));
        for i in 0..60 {
            assert_eq!(abi[0][i], (r.adv[i] - r.dec[i]).abs());
            assert_eq!(th[0][i], r.adv[i] * r.adv_vol[i] - r.dec[i] * r.dec_vol[i]);
            assert_eq!(ud[0][i], r.adv_vol[i] / r.dec_vol[i]);
        }
        let s = |x: &[f64], i: usize| x[i - 9..=i].iter().sum::<f64>();
        assert!(close_eq(ob[0][9], (s(&r.adv, 9) - s(&r.dec, 9)) / 10.) && ob[0][8].is_nan());
        assert!(close_eq(tr[0][20], (s(&r.adv, 20) / s(&r.dec, 20)) / (s(&r.adv_vol, 20) / s(&r.dec_vol, 20))));
    }
    #[test] fn second_batch_states() {
        let b = walk(400, 12, false);
        let (adx, m, cam) = (go("dmi", &b, &[("di_length", 10.), ("adx_smoothing", 10.)]).remove(2), go("macd", &b, &[]).remove(0), go("cam", &b, &[]).remove(0));
        for i in 26..400 {
            let (a, c) = (adx[i] > adx[i - 1], m[i] > m[i - 1]);
            assert_eq!(cam[i], match (a, c) { (true, true) => 1., (false, false) => 2., (true, false) => 3., (false, true) => 4. });
        }
        for s in [1., 2., 3., 4.] { assert!(cam.contains(&s)); }
        assert_series(&go("gann_trend_osc", &d1(), &[])[0], &[N, 1., 1., -1., -1., -1., 1., 1.]);
        assert_series(&go("ttm_trend", &d1(), &[("length", 2.)])[0], &[N, 1., 1., -1., -1., 1., 1., 1.]);
        let am = go("amat", &growth(), &[]);
        assert!(am[0][21].is_nan() && am[0][299] == 1. && am[1][299] == 0.);
        assert_series(&go("linreg_reversal", &closes(&[1., 2., 2., 1.]), &[("length", 2.)])[0], &[N, N, 1., -1.]);
        // mom(1) of d1 = [_, 1, 0.5, −1, −0.8, 0.6, 1.1, 0.7]: rising → high, else low
        assert_series(&go("mom_dots", &d1(), &[("length", 1.)])[0], &[N, N, 11., 10.8, 11.5, 11., 12., 11.5]);
    }
    #[test] fn second_batch_levels() {
        let b = d1();
        let mh = go("mhl_ma", &b, &[("length", 2.), ("hl_length", 2.)]);
        assert!(close_eq(mh[0][2], (10.5 + 11.25) / 2.) && close_eq(mh[1][2], 11.75));
        assert!(close_eq(go("pain", &b, &[])[0][0], 0.75));
        let ms = go("market_structure", &from_ohlc(&[(10., 11., 9., 10.5), (10.5, 12., 10., 11.5), (11.5, 11.8, 10.5, 11.)]), &[]);
        assert_series(&ms[0], &[N, N, 12.]);
        assert_series(&ms[1], &[N, N, N]);
        let t = go("tirone", &b, &[("length", 3.)]);
        let am = 33.5 / 3.;
        for (k, want) in [12.5 - 3.5 / 3., 10.75, 9. + 3.5 / 3., am + 3.5, 2. * am - 9., am, 2. * am - 12.5, am - 3.5].iter().enumerate() { assert!(close_eq(t[k][2], *want), "tirone {k}"); }
        let q = go("quadrant", &b, &[("length", 3.)]);
        for k in 0..5 { assert!(close_eq(q[k][2], 9. + k as f64 * 0.875)); }
        assert_eq!(go("volume_accumulation", &b, &[])[0][0], 50.);
        let r = go("range_compression", &b, &[("length", 2.)]);
        assert!(close_eq(r[0][3], 170. / 3.) && r[1][3] == 1. && close_eq(r[0][4], 88.) && r[1][4] == 0.);
        let line: Vec<f64> = (1..=40).map(|x| x as f64).collect();
        let ct = go("cycle_trend", &closes(&line), &[]);
        assert!(close_eq(ct[0][39], 2.) && close_eq(ct[25][39], 14.5) && close_eq(ct[26][39], 12.5));
    }
    #[test] fn second_batch_oscillators() {
        let b = walk(120, 13, false);
        let d = go("dmi", &b, &[("di_length", 10.)]);
        let osc: Vec<f64> = (0..120).map(|i| d[0][i] - d[1][i]).collect();
        let st = go("dmi_stoch", &b, &[]);
        let ext = |j: usize, hi: bool| osc[j - 2..=j].iter().cloned().fold(if hi { f64::MIN } else { f64::MAX }, if hi { f64::max } else { f64::min });
        for i in 20..120 {
            let (mut num, mut den) = (0., 0.);
            for j in i - 2..=i { num += osc[j] - ext(j, false); den += ext(j, true) - ext(j, false); }
            assert!(close_eq(st[0][i], 100. * num / den));
            assert!(close_eq(st[1][i], (st[0][i] + st[0][i - 1] + st[0][i - 2]) / 3.));
        }
        let w = go("weekly_daily_macd", &b, &[("weekly_fast", 6.), ("weekly_slow", 13.)]);
        let (dm, wm) = (go("macd", &b, &[]).remove(0), go("macd", &b, &[("fast", 6.), ("slow", 13.)]).remove(0));
        for i in 30..120 { assert!(close_eq(w[0][i], dm[i] + wm[i]) && w[1][i] == dm[i] && w[2][i] == wm[i]); }
        // the reverse-engineered closes really put MACD at the level / leave it unchanged
        let kv = [("fast", 3.), ("slow", 6.), ("level", 0.5)];
        let rm = go("reverse_macd", &b, &kv);
        for (k, want) in [(1, 0.5), (0, f64::NAN)] {
            let mut c = b.clone();
            c.close[60] = rm[k][60];
            let m = go("macd", &c, &[("fast", 3.), ("slow", 6.)]).remove(0);
            let want = if want.is_nan() { m[59] } else { want };
            assert!((m[60] - want).abs() < 1e-9, "output {k}: {} vs {want}", m[60]);
        }
        // fractal high on bar 3 shows on bar 5, fractal low on bar 6 on bar 8
        let h = [1., 2., 3., 5., 4., 3., 2., 3., 3., 6., 5., 4.];
        let f = go("fractal_chaos_osc", &from_ohlc(&h.iter().map(|&x| (x, x, x - 1., x)).collect::<Vec<_>>()), &[]);
        assert_series(&f[0][..9], &[N, N, N, N, 0., 1., 0., 0., -1.]);
    }
    #[test] fn rules_reach_derived() {
        for (id, out) in [("adxr", "adxr"), ("elder_impulse", "impulse"), ("ibs", "clv"), ("bw_mfi", "state"), ("trin_open", "trin"), ("narrow_range", "nr"), ("cam", "state"), ("cycle_trend", "madiff"), ("tirone", "extreme_low")] { compiles(id, out).unwrap(); }
    }
}
