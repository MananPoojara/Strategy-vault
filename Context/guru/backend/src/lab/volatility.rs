//! Volatility family (TradingView built-ins).
use super::bars::Bars;
use super::catalogue::{int, len, n, p, real, IndicatorDef, Needs, Output, ParamSpec, Params, Placement};
use super::ta::{self, NA};

const CAT: &str = "Volatility";
const NO: Needs = Needs::NONE;

fn div(a: f64, b: f64) -> f64 { if b == 0. { NA } else { a / b } }
/// [basis, upper, lower] = sma(c,n) +- mult*stdev(c,n)
fn bands(b: &Bars, prm: &Params) -> Vec<Vec<f64>> {
    let (l, m) = (n(prm, "length"), p(prm, "mult"));
    let (ba, sd) = (ta::sma(&b.close, l), ta::stdev(&b.close, l));
    vec![ba.clone(), ba.iter().zip(&sd).map(|(a, s)| a + m * s).collect(), ba.iter().zip(&sd).map(|(a, s)| a - m * s).collect()]
}
fn atr(b: &Bars, prm: &Params) -> Output {
    if p(prm, "variant") == 1. { return Ok(vec![ta::sma(&ta::tr(b, false), n(prm, "length"))]); }
    Ok(vec![ta::rma(&ta::tr(b, true), n(prm, "length"))])
}
fn bb(b: &Bars, prm: &Params) -> Output {
    if p(prm, "source") == 0. { return Ok(bands(b, prm)); }
    let (l, m) = (n(prm, "length"), p(prm, "mult"));
    let (mh, sh, ml, sl) = (ta::sma(&b.high, l), ta::stdev(&b.high, l), ta::sma(&b.low, l), ta::stdev(&b.low, l));
    Ok(vec![(0..b.len()).map(|i| (mh[i] + ml[i]) / 2.).collect(), (0..b.len()).map(|i| mh[i] + m * sh[i]).collect(), (0..b.len()).map(|i| ml[i] - m * sl[i]).collect()])
}
fn bbpct(b: &Bars, prm: &Params) -> Output {
    let k = bands(b, prm);
    Ok(vec![(0..b.len()).map(|i| div(b.close[i] - k[2][i], k[1][i] - k[2][i])).collect()])
}
fn bbw(b: &Bars, prm: &Params) -> Output {
    let k = bands(b, prm);
    Ok(vec![(0..b.len()).map(|i| div(100. * (k[1][i] - k[2][i]), k[0][i])).collect()])
}
fn kc(b: &Bars, prm: &Params) -> Output {
    let (l, al) = (n(prm, "length"), n(prm, "atr_length"));
    let src = if p(prm, "source") == 1. { b.hlc3() } else { b.close.clone() };
    let (m, ba) = (p(prm, "mult"), if p(prm, "basis_type") == 1. { ta::sma(&src, l) } else { ta::ema(&src, l) });
    let r = if p(prm, "range") == 1. { ta::sma(&(0..b.len()).map(|i| b.high[i] - b.low[i]).collect::<Vec<_>>(), al) } else { ta::rma(&ta::tr(b, true), al) };
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
    let days = p(prm, "days");
    Ok(vec![ta::stdev(&r, n(prm, "length")).iter().map(|s| 100. * s * days.sqrt()).collect()])
}

const fn def(id: &'static str, out: &'static [&'static str], name: &'static str, params: &'static [ParamSpec], placement: Placement, definition: &'static str, first: &'static [usize], f: fn(&Bars, &Params) -> Output) -> IndicatorDef {
    IndicatorDef { id, name, category: CAT, params, outputs: out, placement, needs: NO, definition, first, sparse: false, compute: Some(f), note: "" }
}
const BB: &[ParamSpec] = &[len("length", "Length", 20.), real("mult", "Multiplier", 2., 0.1, 10., 0.1)];
const L20: &[ParamSpec] = &[len("length", "Length", 20.)];
const KC: &[ParamSpec] = &[len("length", "Length", 20.), real("mult", "Multiplier", 2., 0.1, 10., 0.1), len("atr_length", "ATR length", 10.),
    int("basis_type", "Basis MA (0 EMA TradingView, 1 SMA AmiBroker/NinjaTrader/STARC)", 0., 0., 1.), int("source", "Basis source (0 close, 1 hlc3 NinjaTrader)", 0., 0., 1.),
    int("range", "Band range (0 Wilder ATR, 1 sma(high − low) NinjaTrader)", 0., 0., 1.)];

pub const DEFS: &[IndicatorDef] = &[
    def("atr", &["atr"], "Average True Range", &[len("length", "Length", 14.), int("variant", "Variant (0 Wilder/TradingView/AmiBroker, 1 MetaTrader simple average)", 0., 0., 1.)], Placement::Pane,
        "rma(tr,n); bar-0 TR = high-low; variant 1 (MetaTrader ATR.mq5): sma(tr, n) with TR from bar 1 (no bar-0 range), so the first value is at bar n (MetaTrader shows 0 before it)", &[13], atr),
    def("bb", &["basis", "upper", "lower"], "Bollinger Bands", &[len("length", "Length", 20.), real("mult", "Multiplier", 2., 0.1, 10., 0.1), int("source", "Source (0 close, 1 high/low Sierra Chart Standard Deviation Bands)", 0., 0., 1.)], Placement::Price,
        "basis=sma(close,n); upper/lower=basis +- mult*stdev(close,n) (population); source 1 (Sierra Chart Standard Deviation Bands, defaults length 10, mult 1.8): upper = sma(high, n) + mult·stdev(high, n), lower = sma(low, n) − mult·stdev(low, n) (population); basis = (sma(high, n) + sma(low, n))/2, the midpoint of the two averages Sierra Chart plots", &[19, 19, 19], bb),
    def("bbpct", &["pct_b"], "Bollinger %B", BB, Placement::Pane, "(close-lower)/(upper-lower) as a fraction; zero width -> NaN", &[19], bbpct),
    def("bbw", &["width"], "Bollinger Bandwidth", BB, Placement::Pane, "100*(upper-lower)/basis; zero basis -> NaN", &[19], bbw),
    def("kc", &["basis", "upper", "lower"], "Keltner Channels", KC, Placement::Price, "basis=ema(close,n); upper/lower=basis +- mult*rma(tr,atr_length) (chart built-in, not ta.kc); basis_type 1 = sma, source 1 = hlc3, range 1 = sma(high − low, atr_length). Platform settings: AmiBroker Keltner Bands = sma(close, 15) ± 2·Wilder ATR(15) (basis_type 1, length 15, atr_length 15); NinjaTrader 8 KeltnerChannel = sma(hlc3, 10) ± 1.5·sma(high − low, 10) (basis_type 1, source 1, range 1); STARC bands (thinkorswim, Sierra Chart) = sma(close, 6) ± 2·ATR(15) (basis_type 1); ChartIQ ATR Bands = close ± shift·ATR(5) (basis_type 1, length 1)", &[19, 19, 19], kc),
    def("donchian", &["upper", "lower", "basis"], "Donchian Channels", L20, Placement::Price, "upper=highest(high,n); lower=lowest(low,n); basis=(upper+lower)/2", &[19, 19, 19], donchian),
    def("stdev", &["stdev"], "Standard Deviation", L20, Placement::Pane, "population stdev(close,n)", &[19], stdev),
    def("hv", &["hv"], "Historical Volatility", &[len("length", "Length", 10.), int("days", "Annualisation days (365 TradingView, 252 cTrader/thinkorswim)", 365., 1., 366.)], Placement::Pane,
        "100*stdev(ln(close/close[1]),n)*sqrt(days) (TradingView daily annualisation days = 365; cTrader HistoricalVolatility BarHistory 252 and thinkorswim VolatilityStdDev use 252, and cTrader does not multiply by 100); non-positive prices -> NaN", &[10], hv),
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
    #[test] fn atr_metatrader_simple_average_from_bar1() {
        let b = from_ohlc(&[(10., 12., 9., 11.), (11., 13., 10.5, 12.), (12., 12.5, 11., 11.5)]);
        assert_series(&run(&get("atr").unwrap(), &b, &many(&[("length", 2.), ("variant", 1.)])).unwrap()[0], &[N, N, 2.]);
    }
    #[test] fn kc_amibroker_and_ninjatrader_settings() {
        let o = run(&get("kc").unwrap(), &closes(&[1., 3., 2.]), &many(&[("length", 2.), ("mult", 1.), ("atr_length", 2.), ("basis_type", 1.)])).unwrap();
        assert_series(&o[0], &[N, 2., 2.5]); assert_series(&o[1], &[N, 3., 3.5]); assert_series(&o[2], &[N, 1., 1.5]);
        // hlc3 = 30.5/3, 33.5/3, 35.5/3; high − low = 2, 2, 1.5
        let b = from_ohlc(&[(10., 11., 9., 10.5), (10.5, 12., 10., 11.5), (11.5, 12.5, 11., 12.)]);
        let o = run(&get("kc").unwrap(), &b, &many(&[("length", 2.), ("mult", 1.), ("atr_length", 2.), ("basis_type", 1.), ("source", 1.), ("range", 1.)])).unwrap();
        assert_series(&o[0], &[N, 32. / 3., 11.5]); assert_series(&o[1], &[N, 38. / 3., 13.25]); assert_series(&o[2], &[N, 26. / 3., 9.75]);
    }
    #[test] fn bb_sierra_chart_high_low_bands() {
        let b = from_ohlc(&[(2., 3., 1., 2.), (3., 5., 1., 4.)]);
        let o = run(&get("bb").unwrap(), &b, &many(&[("length", 2.), ("mult", 1.), ("source", 1.)])).unwrap();
        assert_series(&o[0], &[N, 2.5]); assert_series(&o[1], &[N, 5.]); assert_series(&o[2], &[N, 1.]);
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
    #[test] fn hv_252_days() {
        let e = std::f64::consts::E;
        assert_series(&run(&get("hv").unwrap(), &closes(&[1., e, 1.]), &many(&[("length", 2.), ("days", 252.)])).unwrap()[0], &[N, N, 100. * 252f64.sqrt()]);
    }
    #[test] fn hv_log_returns_annualised() {
        let e = std::f64::consts::E;
        // returns [1,-1] -> population stdev 1 -> 100*sqrt(365)
        assert_series(&run(&get("hv").unwrap(), &closes(&[1., e, 1.]), &with("length", 2.)).unwrap()[0], &[N, N, 100. * 365f64.sqrt()]);
    }
}
