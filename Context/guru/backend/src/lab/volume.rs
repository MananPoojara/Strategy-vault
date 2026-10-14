//! Volume family (TradingView built-ins) plus listed-only intraday entries.
use super::bars::Bars;
use super::catalogue::{len, n, p, real, IndicatorDef, Needs, Output, ParamSpec, Params, Placement};
use super::ta::{self, NA};

const CAT: &str = "Volume";
const V: Needs = Needs { volume: true, intraday: false };

fn div(a: f64, b: f64) -> f64 { if b == 0. { NA } else { a / b } }
fn vol(b: &Bars) -> &[f64] { b.volume.as_deref().unwrap_or(&[]) }
/// Running total starting at the first non-NaN term.
fn cum(x: impl Iterator<Item = f64>) -> Vec<f64> {
    let mut s = 0.;
    x.map(|v| if v.is_nan() { NA } else { s += v; s }).collect()
}
fn sign(x: f64) -> f64 { if x > 0. { 1. } else if x < 0. { -1. } else { x } }
fn dc(b: &Bars) -> Vec<f64> { ta::change(&b.close, 1) }
/// Per-bar money-flow volume.
fn mfv(b: &Bars) -> Vec<f64> {
    let v = vol(b);
    (0..b.len()).map(|i| {
        let (h, l, c) = (b.high[i], b.low[i], b.close[i]);
        if h == l || (c == h && c == l) { 0. } else { (2. * c - l - h) / (h - l) * v[i] }
    }).collect()
}
fn ad(b: &Bars) -> Vec<f64> { cum(mfv(b).into_iter()) }

fn obv(b: &Bars, _: &Params) -> Output { Ok(vec![cum(dc(b).iter().zip(vol(b)).map(|(d, v)| sign(*d) * v))]) }
fn accdist(b: &Bars, _: &Params) -> Output { Ok(vec![ad(b)]) }
fn cmf(b: &Bars, prm: &Params) -> Output {
    let l = n(prm, "length");
    let (m, v) = (ta::sum(&mfv(b), l), ta::sum(vol(b), l));
    Ok(vec![m.iter().zip(&v).map(|(a, b)| div(*a, *b)).collect()])
}
fn chaikin_osc(b: &Bars, prm: &Params) -> Output {
    let a = ad(b);
    Ok(vec![ta::ema(&a, n(prm, "fast")).iter().zip(ta::ema(&a, n(prm, "slow"))).map(|(f, s)| f - s).collect()])
}
fn pvt(b: &Bars, _: &Params) -> Output {
    let c = &b.close;
    Ok(vec![cum(dc(b).iter().zip(vol(b)).enumerate().map(|(i, (d, v))| if i == 0 { NA } else { div(*d, c[i - 1]) * v }))])
}
fn pvo(b: &Bars, prm: &Params) -> Output {
    let s = ta::ema(vol(b), n(prm, "slow"));
    let o: Vec<f64> = ta::ema(vol(b), n(prm, "fast")).iter().zip(&s).map(|(f, s)| div(100. * (f - s), *s)).collect();
    let sg = ta::ema(&o, n(prm, "signal"));
    let h = o.iter().zip(&sg).map(|(a, b)| a - b).collect();
    Ok(vec![o, sg, h])
}
fn efi(b: &Bars, prm: &Params) -> Output {
    let x: Vec<f64> = dc(b).iter().zip(vol(b)).map(|(d, v)| d * v).collect();
    Ok(vec![ta::ema(&x, n(prm, "length"))])
}
fn eom(b: &Bars, prm: &Params) -> Output {
    let (d, v) = (p(prm, "divisor"), vol(b));
    let hl2: Vec<f64> = (0..b.len()).map(|i| (b.high[i] + b.low[i]) / 2.).collect();
    let x: Vec<f64> = ta::change(&hl2, 1).iter().enumerate().map(|(i, c)| div(d * c * (b.high[i] - b.low[i]), v[i])).collect();
    Ok(vec![ta::sma(&x, n(prm, "length"))])
}
/// Negative (v < prev) or positive (v > prev) volume index; carries when the previous close is 0.
fn vindex(b: &Bars, prm: &Params, up: bool) -> Output {
    let (c, v) = (&b.close, vol(b));
    let mut s = Vec::with_capacity(b.len());
    for i in 0..b.len() {
        let prev = if i == 0 { 1000. } else { s[i - 1] };
        let hit = i > 0 && c[i - 1] != 0. && if up { v[i] > v[i - 1] } else { v[i] < v[i - 1] };
        s.push(if hit { prev * (1. + (c[i] - c[i - 1]) / c[i - 1]) } else { prev });
    }
    let sg = ta::ema(&s, n(prm, "ema"));
    Ok(vec![s, sg])
}
fn nvi(b: &Bars, prm: &Params) -> Output { vindex(b, prm, false) }
fn pvi(b: &Bars, prm: &Params) -> Output { vindex(b, prm, true) }

const fn def(id: &'static str, name: &'static str, params: &'static [ParamSpec], out: &'static [&'static str], placement: Placement, definition: &'static str, first: &'static [usize], f: fn(&Bars, &Params) -> Output) -> IndicatorDef {
    IndicatorDef { id, name, category: CAT, params, outputs: out, placement, needs: V, definition, first, sparse: false, compute: Some(f) }
}
const fn listed(id: &'static str, name: &'static str, params: &'static [ParamSpec], out: &'static [&'static str], needs: Needs, definition: &'static str, first: &'static [usize]) -> IndicatorDef {
    IndicatorDef { id, name, category: CAT, params, outputs: out, placement: Placement::Price, needs, definition, first, sparse: false, compute: None }
}
const FS: &[ParamSpec] = &[len("fast", "Fast length", 3.), len("slow", "Slow length", 10.)];
const PVO: &[ParamSpec] = &[len("fast", "Fast length", 12.), len("slow", "Slow length", 26.), len("signal", "Signal length", 9.)];
const EOM: &[ParamSpec] = &[len("length", "Length", 14.), real("divisor", "Divisor", 10000., 1., 1e9, 1.)];
const EMA255: &[ParamSpec] = &[len("ema", "EMA length", 255.)];

pub const DEFS: &[IndicatorDef] = &[
    def("obv", "On Balance Volume", &[], &["obv"], Placement::Pane, "cumulative sign(close-close[1])*volume; bar 0 NaN", &[1], obv),
    def("accdist", "Accumulation/Distribution", &[], &["ad"], Placement::Pane, "cumulative ((2c-l-h)/(h-l))*v; 0 when h==l or c==h==l", &[0], accdist),
    def("cmf", "Chaikin Money Flow", &[len("length", "Length", 20.)], &["cmf"], Placement::Pane, "sum(mfv,n)/sum(volume,n); zero volume sum -> NaN", &[19], cmf),
    def("chaikin_osc", "Chaikin Oscillator", FS, &["osc"], Placement::Pane, "ema(accdist,fast)-ema(accdist,slow)", &[9], chaikin_osc),
    def("pvt", "Price Volume Trend", &[], &["pvt"], Placement::Pane, "cumulative (close-close[1])/close[1]*volume; bar 0 NaN", &[1], pvt),
    def("pvo", "Percentage Volume Oscillator", PVO, &["pvo", "signal", "hist"], Placement::Pane, "100*(ema(v,fast)-ema(v,slow))/ema(v,slow); signal=ema(pvo); hist=pvo-signal", &[25, 33, 33], pvo),
    def("efi", "Elder's Force Index", &[len("length", "Length", 13.)], &["efi"], Placement::Pane, "ema((close-close[1])*volume,n)", &[13], efi),
    def("eom", "Ease of Movement", EOM, &["eom"], Placement::Pane, "sma(divisor*change(hl2)*(high-low)/volume,n); zero volume -> NaN", &[14], eom),
    def("nvi", "Negative Volume Index", EMA255, &["nvi", "signal"], Placement::Pane, "starts at 1000 like TradingView's chart built-in (= 1000 × ta.nvi); volume<prev volume -> prev*(1+dClose/close[1]) else prev; signal=ema(nvi)", &[0, 254], nvi),
    def("pvi", "Positive Volume Index", EMA255, &["pvi", "signal"], Placement::Pane, "starts at 1000 like TradingView's chart built-in (= 1000 × ta.pvi); volume>prev volume -> prev*(1+dClose/close[1]) else prev; signal=ema(pvi)", &[0, 254], pvi),
    listed("vwap", "VWAP", &[], &["vwap"], Needs { volume: true, intraday: true }, "session-anchored sum(hlc3*v)/sum(v); needs intraday bars (listed only)", &[usize::MAX]),
    listed("opening_range", "Opening Range", &[ParamSpec { min: 1., max: 120., ..len("minutes", "Minutes", 15.) }], &["high", "low"], Needs { volume: false, intraday: true }, "high/low of the first N minutes of the session; needs intraday bars (listed only)", &[usize::MAX, usize::MAX]),
];

#[cfg(test)]
mod tests {
    use crate::lab::{catalogue::{get, run, Params}, testdata::{assert_series, from_ohlc}};
    const N: f64 = f64::NAN;
    fn with(rows: &[(f64, f64, f64, f64)], v: &[f64]) -> crate::lab::bars::Bars { let mut b = from_ohlc(rows); b.volume = Some(v.to_vec()); b }
    fn go(id: &str, b: &crate::lab::bars::Bars, kv: &[(&str, f64)]) -> Vec<Vec<f64>> {
        run(&get(id).unwrap(), b, &kv.iter().map(|(k, v)| (k.to_string(), *v)).collect::<Params>()).unwrap()
    }
    #[test] fn opening_range_minutes_bounded() {
        use crate::lab::catalogue::resolve_params;
        let d = get("opening_range").unwrap();
        let pm = |m: f64| -> Params { [("minutes".to_string(), m)].into_iter().collect() };
        assert!(resolve_params(&d, &pm(120.)).is_ok());
        assert!(resolve_params(&d, &pm(121.)).unwrap_err().contains("minutes"));
    }
    #[test] fn obv_starts_at_bar1() {
        let b = with(&[(1., 1., 1., 1.), (2., 2., 2., 2.), (1.5, 1.5, 1.5, 1.5)], &[10., 20., 30.]);
        assert_series(&run(&get("obv").unwrap(), &b, &Params::new()).unwrap()[0], &[N, 20., -10.]);
    }
    // bar0 h=2 l=0 c=1 -> 0; bar1 h=4 l=2 c=4 -> +1*10; bar2 h==l -> 0
    fn mf() -> crate::lab::bars::Bars { with(&[(1., 2., 0., 1.), (3., 4., 2., 4.), (3., 3., 3., 3.)], &[5., 10., 20.]) }
    #[test] fn accdist_handles_flat_bar() { assert_series(&go("accdist", &mf(), &[])[0], &[0., 10., 10.]); }
    #[test] fn cmf_len2() { assert_series(&go("cmf", &mf(), &[("length", 2.)])[0], &[N, 10. / 15., 10. / 30.]); }
    #[test] fn pvt_cumulates() {
        let b = with(&[(1., 1., 1., 1.), (2., 2., 2., 2.), (1.5, 1.5, 1.5, 1.5)], &[10., 20., 30.]);
        assert_series(&go("pvt", &b, &[])[0], &[N, 20., 12.5]);
    }
    #[test] fn nvi_down_and_up_volume() {
        let r: Vec<_> = [10., 11., 12., 6.].iter().map(|&x| (x, x, x, x)).collect();
        let b = with(&r, &[5., 3., 4., 2.]);
        assert_series(&go("nvi", &b, &[("ema", 1.)])[0], &[1000., 1100., 1100., 550.]);
        assert_series(&go("pvi", &b, &[("ema", 1.)])[0], &[1000., 1000., 1000. * 12. / 11., 1000. * 12. / 11.]);
    }
    #[test] fn efi_len2() {
        let b = with(&[(1., 1., 1., 1.), (3., 3., 3., 3.), (2., 2., 2., 2.)], &[1., 2., 3.]);
        assert_series(&go("efi", &b, &[("length", 2.)])[0], &[N, N, 0.5]);
    }
    #[test] fn eom_len2() {
        // hl2 = 1,3,4; range 2,2,4; terms: 2*2/4 = 1, 1*4/2 = 2
        let b = with(&[(1., 2., 0., 1.), (3., 4., 2., 3.), (4., 6., 2., 4.)], &[1., 4., 2.]);
        assert_series(&go("eom", &b, &[("length", 2.), ("divisor", 1.)])[0], &[N, N, 1.5]);
    }
}
