//! Volatility family (TradingView built-ins).
use super::bars::Bars;
use super::catalogue::{len, n, p, real, IndicatorDef, Needs, Output, ParamSpec, Params, Placement};
use super::ta::{self, NA};

const CAT: &str = "Volatility";
const NO: Needs = Needs { volume: false, intraday: false };

fn div(a: f64, b: f64) -> f64 { if b == 0. { NA } else { a / b } }
/// [basis, upper, lower] = sma(c,n) +- mult*stdev(c,n)
fn bands(b: &Bars, prm: &Params) -> Vec<Vec<f64>> {
    let (l, m) = (n(prm, "length"), p(prm, "mult"));
    let (ba, sd) = (ta::sma(&b.close, l), ta::stdev(&b.close, l));
    vec![ba.clone(), ba.iter().zip(&sd).map(|(a, s)| a + m * s).collect(), ba.iter().zip(&sd).map(|(a, s)| a - m * s).collect()]
}
fn atr(b: &Bars, prm: &Params) -> Output { Ok(vec![ta::rma(&ta::tr(b, true), n(prm, "length"))]) }
fn bb(b: &Bars, prm: &Params) -> Output { Ok(bands(b, prm)) }
fn bbpct(b: &Bars, prm: &Params) -> Output {
    let k = bands(b, prm);
    Ok(vec![(0..b.len()).map(|i| div(b.close[i] - k[2][i], k[1][i] - k[2][i])).collect()])
}
fn bbw(b: &Bars, prm: &Params) -> Output {
    let k = bands(b, prm);
    Ok(vec![(0..b.len()).map(|i| div(100. * (k[1][i] - k[2][i]), k[0][i])).collect()])
}
fn kc(b: &Bars, prm: &Params) -> Output {
    let (m, ba) = (p(prm, "mult"), ta::ema(&b.close, n(prm, "length")));
    let r = ta::rma(&ta::tr(b, true), n(prm, "atr_length"));
    Ok(vec![ba.clone(), ba.iter().zip(&r).map(|(a, r)| a + m * r).collect(), ba.iter().zip(&r).map(|(a, r)| a - m * r).collect()])
}
fn donchian(b: &Bars, prm: &Params) -> Output {
    let l = n(prm, "length");
    let (u, d) = (ta::highest(&b.high, l), ta::lowest(&b.low, l));
    let m = u.iter().zip(&d).map(|(a, b)| (a + b) / 2.).collect();
    Ok(vec![u, d, m])
}
fn stdev(b: &Bars, prm: &Params) -> Output { Ok(vec![ta::stdev(&b.close, n(prm, "length"))]) }
fn hv(b: &Bars, prm: &Params) -> Output {
    let r: Vec<f64> = (0..b.len()).map(|i| if i > 0 && b.close[i] > 0. && b.close[i - 1] > 0. { (b.close[i] / b.close[i - 1]).ln() } else { NA }).collect();
    Ok(vec![ta::stdev(&r, n(prm, "length")).iter().map(|s| 100. * s * 365f64.sqrt()).collect()])
}

const fn def(id: &'static str, out: &'static [&'static str], name: &'static str, params: &'static [ParamSpec], placement: Placement, definition: &'static str, first: &'static [usize], f: fn(&Bars, &Params) -> Output) -> IndicatorDef {
    IndicatorDef { id, name, category: CAT, params, outputs: out, placement, needs: NO, definition, first, sparse: false, compute: Some(f) }
}
const BB: &[ParamSpec] = &[len("length", "Length", 20.), real("mult", "Multiplier", 2., 0.1, 10., 0.1)];
const L20: &[ParamSpec] = &[len("length", "Length", 20.)];
const KC: &[ParamSpec] = &[len("length", "Length", 20.), real("mult", "Multiplier", 2., 0.1, 10., 0.1), len("atr_length", "ATR length", 10.)];

pub const DEFS: &[IndicatorDef] = &[
    def("atr", &["atr"], "Average True Range", &[len("length", "Length", 14.)], Placement::Pane, "rma(tr,n); bar-0 TR = high-low", &[13], atr),
    def("bb", &["basis", "upper", "lower"], "Bollinger Bands", BB, Placement::Price, "basis=sma(close,n); upper/lower=basis +- mult*stdev(close,n) (population)", &[19, 19, 19], bb),
    def("bbpct", &["pct_b"], "Bollinger %B", BB, Placement::Pane, "(close-lower)/(upper-lower) as a fraction; zero width -> NaN", &[19], bbpct),
    def("bbw", &["width"], "Bollinger Bandwidth", BB, Placement::Pane, "100*(upper-lower)/basis; zero basis -> NaN", &[19], bbw),
    def("kc", &["basis", "upper", "lower"], "Keltner Channels", KC, Placement::Price, "basis=ema(close,n); upper/lower=basis +- mult*rma(tr,atr_length) (chart built-in, not ta.kc)", &[19, 19, 19], kc),
    def("donchian", &["upper", "lower", "basis"], "Donchian Channels", L20, Placement::Price, "upper=highest(high,n); lower=lowest(low,n); basis=(upper+lower)/2", &[19, 19, 19], donchian),
    def("stdev", &["stdev"], "Standard Deviation", L20, Placement::Pane, "population stdev(close,n)", &[19], stdev),
    def("hv", &["hv"], "Historical Volatility", &[len("length", "Length", 10.)], Placement::Pane, "100*stdev(ln(close/close[1]),n)*sqrt(365) (TradingView daily annualisation); non-positive prices -> NaN", &[10], hv),
];

#[cfg(test)]
mod tests {
    use crate::lab::{catalogue::{run, get, Params}, testdata::{assert_series, from_ohlc}};
    const N: f64 = f64::NAN;
    fn closes(c: &[f64]) -> crate::lab::bars::Bars { from_ohlc(&c.iter().map(|&x| (x, x, x, x)).collect::<Vec<_>>()) }
    fn many(kv: &[(&str, f64)]) -> Params { kv.iter().map(|(k, v)| (k.to_string(), *v)).collect() }
    fn with(k: &str, v: f64) -> Params { many(&[(k, v)]) }
    #[test] fn atr_len2_counts_bar0_range() {
        let b = from_ohlc(&[(10., 12., 9., 11.), (11., 13., 10.5, 12.), (12., 12.5, 11., 11.5)]);
        assert_series(&run(&get("atr").unwrap(), &b, &with("length", 2.)).unwrap()[0], &[f64::NAN, 2.75, 2.125]);
    }
    #[test] fn bb_len2_mult1() {
        let o = run(&get("bb").unwrap(), &closes(&[1., 3., 2.]), &many(&[("length", 2.), ("mult", 1.)])).unwrap();
        assert_series(&o[0], &[N, 2., 2.5]); assert_series(&o[1], &[N, 3., 3.]); assert_series(&o[2], &[N, 1., 2.]);
    }
    #[test] fn bbpct_fraction_and_zero_width() {
        let pr = many(&[("length", 2.), ("mult", 1.)]);
        assert_series(&run(&get("bbpct").unwrap(), &closes(&[1., 3., 2.]), &pr).unwrap()[0], &[N, 1., 0.]);
        assert_series(&run(&get("bbpct").unwrap(), &closes(&[2., 2., 2.]), &pr).unwrap()[0], &[N, N, N]);
    }
    #[test] fn bbw_len2_mult1() {
        let pr = many(&[("length", 2.), ("mult", 1.)]);
        assert_series(&run(&get("bbw").unwrap(), &closes(&[1., 3., 2.]), &pr).unwrap()[0], &[N, 100., 40.]);
    }
    #[test] fn kc_short_lengths() {
        // tr = [0,2,1]; rma(2) = [_,1,1]; ema(2) of closes seeds 2 then stays 2
        let o = run(&get("kc").unwrap(), &closes(&[1., 3., 2.]), &many(&[("length", 2.), ("mult", 1.), ("atr_length", 2.)])).unwrap();
        assert_series(&o[0], &[N, 2., 2.]); assert_series(&o[1], &[N, 3., 3.]); assert_series(&o[2], &[N, 1., 1.]);
    }
    #[test] fn donchian_len2() {
        let b = from_ohlc(&[(1., 2., 0., 1.), (1., 3., 1., 2.), (1., 4., 2., 3.), (1., 5., 3., 5.), (1., 6., 4., 4.)]);
        let o = run(&get("donchian").unwrap(), &b, &with("length", 2.)).unwrap();
        assert_series(&o[0], &[N, 3., 4., 5., 6.]); assert_series(&o[1], &[N, 0., 1., 2., 3.]); assert_series(&o[2], &[N, 1.5, 2.5, 3.5, 4.5]);
    }
    #[test] fn hv_log_returns_annualised() {
        let e = std::f64::consts::E;
        // returns [1,-1] -> population stdev 1 -> 100*sqrt(365)
        assert_series(&run(&get("hv").unwrap(), &closes(&[1., e, 1.]), &with("length", 2.)).unwrap()[0], &[N, N, 100. * 365f64.sqrt()]);
    }
}
