//! Moving-average family (TradingView built-ins).
use super::bars::Bars;
use super::catalogue::{len, n, p, real, IndicatorDef, Needs, Output, ParamSpec, Params, Placement};
use super::ta;

const CAT: &str = "Moving averages";
const NO: Needs = Needs { volume: false, intraday: false };
const LEN9: &[ParamSpec] = &[len("length", "Length", 9.)];

fn sma(b: &Bars, prm: &Params) -> Output { Ok(vec![ta::sma(&b.close, n(prm, "length"))]) }
fn ema(b: &Bars, prm: &Params) -> Output { Ok(vec![ta::ema(&b.close, n(prm, "length"))]) }
fn wma(b: &Bars, prm: &Params) -> Output { Ok(vec![ta::wma(&b.close, n(prm, "length"))]) }
fn smma(b: &Bars, prm: &Params) -> Output { Ok(vec![ta::rma(&b.close, n(prm, "length"))]) }
fn lsma(b: &Bars, prm: &Params) -> Output { Ok(vec![ta::linreg(&b.close, n(prm, "length"), 0)]) }
fn vwma(b: &Bars, prm: &Params) -> Output {
    let v = b.volume.as_ref().ok_or("needs volume")?;
    let n = n(prm, "length");
    let cv: Vec<f64> = b.close.iter().zip(v).map(|(c, v)| c * v).collect();
    let (num, den) = (ta::sma(&cv, n), ta::sma(v, n));
    Ok(vec![num.iter().zip(&den).map(|(a, d)| if *d == 0. { ta::NA } else { a / d }).collect()])
}
fn hma(b: &Bars, prm: &Params) -> Output {
    let n = n(prm, "length");
    let (half, full) = (ta::wma(&b.close, n / 2), ta::wma(&b.close, n));
    let diff: Vec<f64> = half.iter().zip(&full).map(|(h, f)| 2. * h - f).collect();
    Ok(vec![ta::wma(&diff, (n as f64).sqrt().floor() as usize)])
}
fn alma(b: &Bars, prm: &Params) -> Output {
    let n = n(prm, "length");
    let (m, s) = (p(prm, "offset") * (n as f64 - 1.), n as f64 / p(prm, "sigma"));
    let w: Vec<f64> = (0..n).map(|i| (-(i as f64 - m).powi(2) / (2. * s * s)).exp()).collect();
    let ws: f64 = w.iter().sum();
    let mut out = vec![ta::NA; b.len()];
    for i in n.saturating_sub(1)..b.len() {
        let x = &b.close[i + 1 - n..=i];
        if x.iter().any(|v| v.is_nan()) { continue; }
        out[i] = x.iter().zip(&w).map(|(a, w)| a * w).sum::<f64>() / ws;
    }
    Ok(vec![out])
}
fn dema(b: &Bars, prm: &Params) -> Output {
    let n = n(prm, "length");
    let e1 = ta::ema(&b.close, n);
    let e2 = ta::ema(&e1, n);
    Ok(vec![e1.iter().zip(&e2).map(|(a, c)| 2. * a - c).collect()])
}
fn tema(b: &Bars, prm: &Params) -> Output {
    let n = n(prm, "length");
    let e1 = ta::ema(&b.close, n);
    let e2 = ta::ema(&e1, n);
    let e3 = ta::ema(&e2, n);
    Ok(vec![(0..b.len()).map(|i| 3. * (e1[i] - e2[i]) + e3[i]).collect()])
}
fn kama(b: &Bars, prm: &Params) -> Output {
    let n = n(prm, "length");
    let (f, s) = (2. / (p(prm, "fast") + 1.), 2. / (p(prm, "slow") + 1.));
    let c = &b.close;
    let vol = ta::sum(&ta::change(c, 1).iter().map(|d| d.abs()).collect::<Vec<_>>(), n);
    let mut out = vec![ta::NA; c.len()];
    let mut prev = ta::NA;
    for i in n..c.len() {
        if c[i].is_nan() || vol[i].is_nan() { continue; }
        let er = if vol[i] == 0. { 0. } else { (c[i] - c[i - n]).abs() / vol[i] };
        let a = (er * (f - s) + s).powi(2);
        prev = if prev.is_nan() { c[i] } else { a * c[i] + (1. - a) * prev };
        out[i] = prev;
    }
    Ok(vec![out])
}
fn mcginley(b: &Bars, prm: &Params) -> Output {
    let n = n(prm, "length");
    let e = ta::ema(&b.close, n);
    let mut out = vec![ta::NA; b.len()];
    let mut prev = ta::NA;
    for i in 0..b.len() {
        prev = if prev.is_nan() { e[i] } else {
            let d = n as f64 * (b.close[i] / prev).powi(4);
            if d == 0. || !d.is_finite() { ta::NA } else { prev + (b.close[i] - prev) / d }
        };
        out[i] = prev;
    }
    Ok(vec![out])
}

const fn def(id: &'static str, out: &'static [&'static str], name: &'static str, params: &'static [ParamSpec], needs: Needs, definition: &'static str, first: &'static [usize], f: fn(&Bars, &Params) -> Output) -> IndicatorDef {
    IndicatorDef { id, name, category: CAT, params, outputs: out, placement: Placement::Price, needs, definition, first, sparse: false, compute: Some(f) }
}

pub const DEFS: &[IndicatorDef] = &[
    def("sma", &["sma"], "Simple Moving Average", LEN9, NO, "sma(close, length)", &[8], sma),
    def("ema", &["ema"], "Exponential Moving Average", LEN9, NO, "ema(close, length), seeded with the SMA of the first length values", &[8], ema),
    def("wma", &["wma"], "Weighted Moving Average", LEN9, NO, "wma(close, length)", &[8], wma),
    def("smma", &["smma"], "Smoothed Moving Average", &[len("length", "Length", 7.)], NO, "rma(close, length) (Wilder smoothing)", &[6], smma),
    def("vwma", &["vwma"], "Volume Weighted Moving Average", &[len("length", "Length", 20.)], Needs { volume: true, intraday: false }, "sma(close*volume, length) / sma(volume, length)", &[19], vwma),
    def("hma", &["hma"], "Hull Moving Average", &[ParamSpec { min: 2., ..len("length", "Length", 9.) }], NO, "wma(2*wma(close, floor(length/2)) - wma(close, length), floor(sqrt(length)))", &[10], hma),
    def("alma", &["alma"], "Arnaud Legoux Moving Average", &[len("length", "Length", 9.), real("offset", "Offset", 0.85, 0., 1., 0.05), real("sigma", "Sigma", 6., 0.5, 50., 0.5)], NO,
        "m=offset*(length-1), s=length/sigma, w_i=exp(-(i-m)^2/(2*s^2)) with i=0 oldest; sum(w_i*close)/sum(w_i)", &[8], alma),
    def("lsma", &["lsma"], "Least Squares Moving Average", &[len("length", "Length", 25.)], NO, "linreg(close, length, 0)", &[24], lsma),
    def("dema", &["dema"], "Double EMA", LEN9, NO, "2*ema(close, length) - ema(ema(close, length), length)", &[16], dema),
    def("tema", &["tema"], "Triple EMA", LEN9, NO, "3*(e1 - e2) + e3 with e1=ema(close), e2=ema(e1), e3=ema(e2)", &[24], tema),
    def("kama", &["kama"], "Kaufman Adaptive Moving Average", &[len("length", "Length", 10.), len("fast", "Fast", 2.), len("slow", "Slow", 30.)], NO,
        "ER=|close-close[n]|/sum_n|change| (0 if denominator 0); a=(ER*(2/(fast+1)-2/(slow+1))+2/(slow+1))^2; k=a*close+(1-a)*k[1], seeded with close", &[10], kama),
    def("mcginley", &["mg"], "McGinley Dynamic", &[len("length", "Length", 14.)], NO, "mg = na(mg[1]) ? ema(close, length) : mg[1] + (close-mg[1])/(length*(close/mg[1])^4)", &[13], mcginley),
];

#[cfg(test)]
mod tests {
    use crate::lab::{catalogue::{run, get, Params}, testdata::{assert_series, from_ohlc}};
    const N: f64 = f64::NAN;
    fn closes(c: &[f64]) -> crate::lab::bars::Bars { from_ohlc(&c.iter().map(|&x| (x, x, x, x)).collect::<Vec<_>>()) }
    fn with(k: &str, v: f64) -> Params { let mut p = Params::new(); p.insert(k.into(), v); p }
    #[test] fn sma_len3() { assert_series(&run(&get("sma").unwrap(), &closes(&[1., 2., 3., 4.]), &with("length", 3.)).unwrap()[0], &[N, N, 2., 3.]); }
    #[test] fn ema_len2_sma_seed() { assert_series(&run(&get("ema").unwrap(), &closes(&[1., 2., 3.]), &with("length", 2.)).unwrap()[0], &[N, 1.5, 2.5]); }
    #[test] fn hma_len4_constant_input() { // any MA of a constant is the constant
        assert_series(&run(&get("hma").unwrap(), &closes(&[5.; 6]), &with("length", 4.)).unwrap()[0], &[N, N, N, N, 5., 5.]);
    }
}
