//! Trend family (TradingView built-ins).
use super::bars::Bars;
use super::catalogue::{int, len, n, p, real, IndicatorDef, Needs, Output, ParamSpec, Params, Placement};
use super::ta::{self, NA};

const CAT: &str = "Trend";
const NO: Needs = Needs::NONE;

fn div(a: f64, b: f64) -> f64 { if b == 0. { NA } else { a / b } }

fn dmi(b: &Bars, prm: &Params) -> Output {
    let (dl, al) = (n(prm, "di_length"), n(prm, "adx_smoothing"));
    if p(prm, "variant") == 1. { return Ok(dmi_mt(b, dl, al)); }
    let m = b.len();
    let dm = |plus: bool| -> Vec<f64> { (0..m).map(|i| if i == 0 { NA } else {
        let (up, dn) = (b.high[i] - b.high[i - 1], b.low[i - 1] - b.low[i]);
        let (a, o) = if plus { (up, dn) } else { (dn, up) };
        if a > o && a > 0. { a } else { 0. }
    }).collect() };
    let trs = ta::rma(&ta::tr(b, false), dl);
    // zero rma(tr) holds the previous DI (fixnan)
    let di = |dm: Vec<f64>| -> Vec<f64> {
        let r = ta::rma(&dm, dl);
        let mut prev = NA;
        (0..m).map(|i| { if !r[i].is_nan() && !trs[i].is_nan() && trs[i] != 0. { prev = 100. * r[i] / trs[i]; } prev }).collect()
    };
    let (pl, mi) = (di(dm(true)), di(dm(false)));
    let x: Vec<f64> = (0..m).map(|i| { let s = pl[i] + mi[i]; (pl[i] - mi[i]).abs() / if s == 0. { 1. } else { s } }).collect();
    let adx: Vec<f64> = ta::rma(&x, al).iter().map(|v| 100. * v).collect();
    Ok(vec![pl, mi, adx])
}

/// MetaTrader 5 ADX.mq5: per-bar DM (smaller side and ties zeroed) as 100·DM/TR, EMA-smoothed (alpha 2/(n+1)) from 0 at bar 0.
fn dmi_mt(b: &Bars, dl: usize, al: usize) -> Vec<Vec<f64>> {
    let (m, a, aa) = (b.len(), 2. / (dl as f64 + 1.), 2. / (al as f64 + 1.));
    let mut out = vec![vec![NA; m]; 3];
    let (mut pdi, mut ndi, mut adx) = (0., 0., 0.);
    for i in 1..m {
        let (mut up, mut dn) = ((b.high[i] - b.high[i - 1]).max(0.), (b.low[i - 1] - b.low[i]).max(0.));
        if up > dn { dn = 0. } else if up < dn { up = 0. } else { up = 0.; dn = 0. }
        let pc = b.close[i - 1];
        let tr = (b.high[i] - b.low[i]).abs().max((b.high[i] - pc).abs()).max((b.low[i] - pc).abs());
        let (pd, nd) = if tr != 0. { (100. * up / tr, 100. * dn / tr) } else { (0., 0.) };
        pdi = pd * a + pdi * (1. - a); ndi = nd * a + ndi * (1. - a);
        let s = pdi + ndi;
        let x = if s != 0. { 100. * ((pdi - ndi) / s).abs() } else { 0. };
        adx = x * aa + adx * (1. - aa);
        out[0][i] = pdi; out[1][i] = ndi; out[2][i] = adx;
    }
    out
}

fn supertrend(b: &Bars, prm: &Params) -> Output {
    let (f, m) = (p(prm, "factor"), b.len());
    let atr = ta::rma(&ta::tr(b, true), n(prm, "atr_length"));
    let hl2 = b.hl2();
    let (mut st, mut dir) = (vec![NA; m], vec![NA; m]);
    let (mut u1, mut l1, mut st1) = (NA, NA, NA);
    for i in 0..m {
        if atr[i].is_nan() { continue; }
        let (mut u, mut l) = (hl2[i] + f * atr[i], hl2[i] - f * atr[i]);
        let first = u1.is_nan();
        if !first {
            let pc = b.close[i - 1];
            if !(l > l1 || pc < l1) { l = l1; }
            if !(u < u1 || pc > u1) { u = u1; }
        }
        let d = if first { 1. } else if st1 == u1 { if b.close[i] > u { -1. } else { 1. } } else if b.close[i] < l { 1. } else { -1. };
        let s = if d == -1. { l } else { u };
        st[i] = s; dir[i] = d; u1 = u; l1 = l; st1 = s;
    }
    Ok(vec![st, dir])
}

fn sar(b: &Bars, prm: &Params) -> Output {
    let (start, inc, max) = (p(prm, "start"), p(prm, "increment"), p(prm, "max"));
    let (h, l) = (&b.high, &b.low);
    let mut out = vec![NA; b.len()];
    if b.len() < 2 { return Ok(vec![out]); }
    let mut long = h[1] > h[0];
    let (mut s, mut ep, mut af) = if long { (l[0], h[1], start) } else { (h[0], l[1], start) };
    out[1] = s;
    for i in 2..b.len() {
        s += af * (ep - s);
        let mut rev = false;
        if long && l[i] < s { rev = true; s = h[i].max(ep); ep = l[i]; af = start; long = false; }
        else if !long && h[i] > s { rev = true; s = l[i].min(ep); ep = h[i]; af = start; long = true; }
        if !rev {
            if long && h[i] > ep { ep = h[i]; af = (af + inc).min(max); }
            if !long && l[i] < ep { ep = l[i]; af = (af + inc).min(max); }
        }
        s = if long { s.min(l[i - 1]).min(l[i - 2]) } else { s.max(h[i - 1]).max(h[i - 2]) };
        out[i] = s;
    }
    Ok(vec![out])
}

/// 100*(len - bars since the oldest extreme in len+1 bars)/len, for highs (up) or lows (down).
fn aroon_s(x: &[f64], k: usize, high: bool) -> Vec<f64> {
    (0..x.len()).map(|i| {
        if i < k { return NA; }
        let mut best = i - k;
        for j in i - k..=i { if (high && x[j] > x[best]) || (!high && x[j] < x[best]) { best = j; } }
        100. * (k - (i - best)) as f64 / k as f64
    }).collect()
}
fn aroon(b: &Bars, prm: &Params) -> Output { let k = n(prm, "length"); Ok(vec![aroon_s(&b.high, k, true), aroon_s(&b.low, k, false)]) }
fn aroon_osc(b: &Bars, prm: &Params) -> Output {
    let k = n(prm, "length");
    Ok(vec![aroon_s(&b.high, k, true).iter().zip(aroon_s(&b.low, k, false)).map(|(u, d)| u - d).collect()])
}

fn mid(b: &Bars, k: usize) -> Vec<f64> {
    ta::highest(&b.high, k).iter().zip(ta::lowest(&b.low, k)).map(|(h, l)| (h + l) / 2.).collect()
}
fn ichimoku(b: &Bars, prm: &Params) -> Output {
    let (t, k) = (mid(b, n(prm, "conversion")), mid(b, n(prm, "base")));
    let a = t.iter().zip(&k).map(|(x, y)| (x + y) / 2.).collect();
    let sb = mid(b, n(prm, "span_b"));
    let sh = n(prm, "displacement").saturating_sub(1); // spans as plotted: calculated displacement-1 bars earlier (causal)
    let shift = |v: Vec<f64>| -> Vec<f64> { (0..v.len()).map(|i| if i < sh { NA } else { v[i - sh] }).collect() };
    Ok(vec![t, k, shift(a), shift(sb)])
}

fn vortex(b: &Bars, prm: &Params) -> Output {
    let k = n(prm, "length");
    let vm = |plus: bool| -> Vec<f64> { (0..b.len()).map(|i| if i == 0 { NA } else if plus { (b.high[i] - b.low[i - 1]).abs() } else { (b.low[i] - b.high[i - 1]).abs() }).collect() };
    let den = ta::sum(&ta::tr(b, true), k);
    let vi = |v: Vec<f64>| -> Vec<f64> { ta::sum(&v, k).iter().zip(&den).map(|(a, d)| div(*a, *d)).collect() };
    Ok(vec![vi(vm(true)), vi(vm(false))])
}

fn chop(b: &Bars, prm: &Params) -> Output {
    let k = n(prm, "length");
    let (s, hh, ll) = (ta::sum(&ta::tr(b, true), k), ta::highest(&b.high, k), ta::lowest(&b.low, k));
    Ok(vec![(0..b.len()).map(|i| {
        let r = div(s[i], hh[i] - ll[i]);
        if r.is_nan() || r <= 0. { NA } else { div(100. * r.log10(), (k as f64).log10()) }
    }).collect()])
}

const fn def(id: &'static str, name: &'static str, params: &'static [ParamSpec], outputs: &'static [&'static str], placement: Placement, definition: &'static str, first: &'static [usize], f: fn(&Bars, &Params) -> Output) -> IndicatorDef {
    IndicatorDef { id, name, category: CAT, params, outputs, placement, needs: NO, definition, first, sparse: false, compute: Some(f), note: "" }
}
const LEN14: &[ParamSpec] = &[len("length", "Length", 14.)];

pub const DEFS: &[IndicatorDef] = &[
    def("dmi", "Directional Movement Index", &[len("di_length", "DI length", 14.), len("adx_smoothing", "ADX smoothing", 14.),
        int("variant", "Variant (0 Wilder/TradingView/MetaTrader iADXWilder, 1 MetaTrader iADX)", 0., 0., 1.)], &["plus_di", "minus_di", "adx"], Placement::Pane,
        "up=change(high), dn=-change(low); +DM=(up>dn&&up>0)?up:0; tr=tr(false); +DI=100*rma(+DM)/rma(tr) held when rma(tr)=0; ADX=100*rma(|+DI--DI|/(sum==0?1:sum)); variant 1 (MetaTrader 4/5 iADX, ADX.mq5): +DM = max(up, 0), −DM = max(dn, 0), the smaller zeroed and both zeroed on a tie; +DI = ema(100·+DM/TR, di_length) (TR 0 → 0), −DI likewise; ADX = ema(100·|+DI − −DI|/(+DI + −DI), adx_smoothing) (sum 0 → 0); every EMA alpha 2/(n+1) starts from 0 at bar 0 (MetaTrader's seed), so values exist from bar 1 and the first ~n bars are seed-biased (MetaTrader starts drawing at bar n)", &[14, 14, 27], dmi),
    def("supertrend", "Supertrend", &[len("atr_length", "ATR length", 10.), real("factor", "Factor", 3., 0.5, 20., 0.5)], &["supertrend", "direction"], Placement::Price,
        "atr=rma(tr(true)); bands hl2+-factor*atr with carry rules; direction -1 uptrend / +1 downtrend; NaN where atr is NaN", &[9, 9], supertrend),
    def("sar", "Parabolic SAR", &[real("start", "Start", 0.02, 0.001, 1., 0.001), real("increment", "Increment", 0.02, 0.001, 1., 0.001), real("max", "Maximum", 0.2, 0.01, 1., 0.01)], &["sar"], Placement::Price,
        "TradingView ta.sar: initial direction long if high[1]>high[0]; sar+=af*(ep-sar); reverse on strict penetration; clipped to the last two lows (long) / highs (short)", &[1], sar),
    def("aroon", "Aroon", LEN14, &["up", "down"], Placement::Pane,
        "100*(length - bars since highest high in length+1 bars)/length (ties -> oldest); down uses lows", &[14, 14], aroon),
    def("aroon_osc", "Aroon Oscillator", LEN14, &["osc"], Placement::Pane, "aroon up - aroon down", &[14], aroon_osc),
    def("ichimoku", "Ichimoku Cloud", &[len("conversion", "Conversion line", 9.), len("base", "Base line", 26.), len("span_b", "Leading span B", 52.), len("displacement", "Displacement", 26.)],
        &["tenkan", "kijun", "span_a", "span_b"], Placement::Price,
        "mid(n)=(highest(high,n)+lowest(low,n))/2; span_a=(tenkan+kijun)/2; spans shown as plotted: calculated displacement-1 bars earlier", &[8, 25, 50, 76], ichimoku),
    def("vortex", "Vortex Indicator", LEN14, &["vi_plus", "vi_minus"], Placement::Pane,
        "sum|high-low[1]|/sum(tr(true)); sum|low-high[1]|/sum(tr(true)) over n; zero denominator -> NaN", &[14, 14], vortex),
    def("chop", "Choppiness Index", LEN14, &["chop"], Placement::Pane,
        "100*log10(sum(tr(true),n)/(highest(high,n)-lowest(low,n)))/log10(n); zero range or non-positive ratio -> NaN", &[13], chop),
];

#[cfg(test)]
mod tests {
    use crate::lab::{catalogue::{get, run, Params}, testdata::{assert_series, from_ohlc}};
    const N: f64 = f64::NAN;
    fn many(kv: &[(&str, f64)]) -> Params { kv.iter().map(|(k, v)| (k.to_string(), *v)).collect() }
    fn go(id: &str, b: &crate::lab::bars::Bars, p: &Params) -> Vec<Vec<f64>> { run(&get(id).unwrap(), b, p).unwrap() }

    #[test] fn sar_matches_tradingview_swiggy_first_bars() {
        // NSE:SWIGGY first four daily bars, ta.sar(0.02,0.02,0.2) read on TradingView 2026-10-14
        let b = from_ohlc(&[(420., 465.8, 391., 456.), (473.15, 489.4, 418.6, 430.7), (416.1, 438.7, 410.4, 420.1), (424., 430.9, 408.85, 412.45)]);
        assert_series(&run(&get("sar").unwrap(), &b, &Params::new()).unwrap()[0], &[f64::NAN, 391., 391., 392.968]);
    }
    #[test] fn ichimoku_spans_shifted_displacement_minus_one() {
        let b = from_ohlc(&[(1., 1., 1., 1.), (2., 2., 2., 2.), (4., 4., 4., 4.), (8., 8., 8., 8.)]);
        let o = go("ichimoku", &b, &many(&[("conversion", 2.), ("base", 2.), ("span_b", 2.), ("displacement", 2.)]));
        assert_series(&o[0], &[N, 1.5, 3., 6.]);
        assert_series(&o[1], &[N, 1.5, 3., 6.]);
        assert_series(&o[2], &[N, N, 1.5, 3.]);
        assert_series(&o[3], &[N, N, 1.5, 3.]);
    }
    #[test] fn aroon_with_tie() {
        // highs tie at bars 1,2 (oldest wins); window is length+1 bars
        let b = from_ohlc(&[(1., 1., 5., 1.), (1., 3., 4., 1.), (1., 3., 4., 1.), (1., 2., 6., 1.), (1., 5., 7., 1.)]);
        let o = go("aroon", &b, &many(&[("length", 2.)]));
        assert_series(&o[0], &[N, N, 50., 0., 100.]);
        assert_series(&o[1], &[N, N, 50., 0., 0.]);
        assert_series(&go("aroon_osc", &b, &many(&[("length", 2.)]))[0], &[N, N, 0., 0., 100.]);
    }
    #[test] fn chop_constant_range_is_100() {
        let b = from_ohlc(&[(11., 12., 10., 11.); 3]);
        assert_series(&go("chop", &b, &many(&[("length", 2.)]))[0], &[N, 100., 100.]);
    }
    #[test] fn vortex_short_length() {
        let b = from_ohlc(&[(9., 10., 8., 9.), (9., 12., 9., 11.), (11., 11., 10., 10.5)]);
        let o = go("vortex", &b, &many(&[("length", 2.)]));
        assert_series(&o[0], &[N, N, 1.5]);
        assert_series(&o[1], &[N, N, 0.75]);
    }
    #[test] fn dmi_metatrader_iadx_ema_from_zero() {
        // n = 3 → alpha 0.5; bar 1: +DM 1, TR 2.5 → 40; bar 2: −DM 1.5, TR 3.5; bar 3: up 0 = dn 0 (tie) → both 0
        let b = from_ohlc(&[(10., 11., 9., 10.), (10., 12., 9.5, 11.), (11., 11.5, 8., 9.), (9., 10., 8., 9.5)]);
        let o = go("dmi", &b, &many(&[("di_length", 3.), ("adx_smoothing", 3.), ("variant", 1.)]));
        assert_series(&o[0], &[N, 20., 10., 5.]);
        assert_series(&o[1], &[N, 0., 150. / 7., 75. / 7.]);
        assert_series(&o[2], &[N, 50., 50. / 2. + 400. / 22., 43.181818181818 / 2. + 400. / 22.]);
    }
    #[test] fn supertrend_flips_up() {
        let b = from_ohlc(&[(10., 11., 9., 10.), (11., 12., 10., 11.5), (13., 15., 12., 14.), (14., 16., 13., 15.)]);
        let o = go("supertrend", &b, &many(&[("atr_length", 1.), ("factor", 1.)]));
        assert_series(&o[0], &[12., 12., 10., 11.5]);
        assert_series(&o[1], &[1., 1., -1., -1.]);
    }
}
