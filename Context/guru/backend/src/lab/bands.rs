//! Bands & channels family (TradingView / AmiBroker built-ins, Phase 6). Formulas: see each definition string.
use super::bars::Bars;
use super::catalogue::{int, len, n, p, real, IndicatorDef, Needs, Output, ParamSpec, Params, Placement};
use super::ta::{self, NA};

const CAT: &str = "Bands & channels";
#[allow(clippy::too_many_arguments)]
const fn def(id: &'static str, name: &'static str, params: &'static [ParamSpec], outputs: &'static [&'static str], placement: Placement,
             needs: Needs, definition: &'static str, first: &'static [usize], sparse: bool, f: fn(&Bars, &Params) -> Output) -> IndicatorDef {
    IndicatorDef { id, name, category: CAT, params, outputs, placement, needs, definition, first, sparse, compute: Some(f), note: "" }
}
const P: Placement = Placement::Price;
const NO: Needs = Needs::NONE;
const ENV: &[ParamSpec] = &[len("length", "Length", 20.), real("percent", "Percent", 10., 0.1, 100., 0.1), int("exponential", "MA (0 SMA, 1 EMA, 2 ChartIQ triangular)", 0., 0., 2.)];
const CH: &[ParamSpec] = &[len("upper_length", "Upper length", 20.), len("lower_length", "Lower length", 20.)];
const APZ: &[ParamSpec] = &[len("length", "Length", 50.), real("mult", "Multiplier", 2., 0.1, 10., 0.1)];
const ALL: &[ParamSpec] = &[len("jaw_length", "Jaw length", 13.), len("teeth_length", "Teeth length", 8.), len("lips_length", "Lips length", 5.),
    int("jaw_offset", "Jaw offset", 8., 0., 100.), int("teeth_offset", "Teeth offset", 5., 0., 100.), int("lips_offset", "Lips offset", 3., 0., 100.)];
const MED: &[ParamSpec] = &[len("length", "Length", 3.), len("atr_length", "ATR length", 14.), real("mult", "ATR multiplier", 2., 0.1, 10., 0.1),
    int("source", "Price (0 hl2, 1 close, 2 open, 3 high, 4 low, 5 hlc3, 6 ohlc4)", 0., 0., 6.)];

fn envelope(b: &Bars, prm: &Params) -> Output {
    let k = n(prm, "length"); let pc = p(prm, "percent") / 100.;
    let ba = match n(prm, "exponential") {
        1 => ta::ema(&b.close, k),
        2 => { let h = k.div_ceil(2); ta::sma(&ta::sma(&b.close, h), h + (1 - k % 2)) }
        _ => ta::sma(&b.close, k),
    };
    Ok(vec![ba.clone(), ba.iter().map(|x| x * (1. + pc)).collect(), ba.iter().map(|x| x * (1. - pc)).collect()])
}
fn ma_channel(b: &Bars, prm: &Params) -> Output { Ok(vec![ta::sma(&b.high, n(prm, "upper_length")), ta::sma(&b.low, n(prm, "lower_length"))]) }
fn apz(b: &Bars, prm: &Params) -> Output {
    let a = 2. / ((n(prm, "length") as f64).sqrt() + 1.);
    let ds = |x: &[f64]| super::ma2::ama_s(&super::ma2::ama_s(x, a, 1. - a), a, 1. - a);
    let (basis, rng) = (ds(&b.close), ds(&(0..b.len()).map(|i| b.high[i] - b.low[i]).collect::<Vec<_>>()));
    let m = p(prm, "mult");
    Ok(vec![basis.clone(), (0..b.len()).map(|i| basis[i] + m * rng[i]).collect(), (0..b.len()).map(|i| basis[i] - m * rng[i]).collect()])
}
fn alligator(b: &Bars, prm: &Params) -> Output {
    let h = b.hl2();
    let line = |l: &str, o: &str| { let (v, s) = (ta::rma(&h, n(prm, l)), n(prm, o)); (0..v.len()).map(|i| if i >= s { v[i - s] } else { NA }).collect::<Vec<_>>() };
    Ok(vec![line("jaw_length", "jaw_offset"), line("teeth_length", "teeth_offset"), line("lips_length", "lips_offset")])
}
fn median(b: &Bars, prm: &Params) -> Output {
    let (k, m) = (n(prm, "length"), p(prm, "mult"));
    let h = match n(prm, "source") { 1 => b.close.clone(), 2 => b.open.clone(), 3 => b.high.clone(), 4 => b.low.clone(), 5 => b.hlc3(), 6 => b.ohlc4(), _ => b.hl2() };
    let md: Vec<f64> = (0..b.len()).map(|i| {
        if i + 1 < k { return NA; }
        let mut w = h[i + 1 - k..=i].to_vec();
        if w.iter().any(|v| v.is_nan()) { return NA; }
        w.sort_by(|a, b| a.total_cmp(b));
        if k % 2 == 1 { w[k / 2] } else { (w[k / 2 - 1] + w[k / 2]) / 2. }
    }).collect();
    let atr = ta::rma(&ta::tr(b, true), n(prm, "atr_length"));
    Ok(vec![md.clone(), (0..b.len()).map(|i| md[i] + m * atr[i]).collect(), (0..b.len()).map(|i| md[i] - m * atr[i]).collect(), ta::ema(&md, k)])
}

pub const DEFS: &[IndicatorDef] = &[
    def("envelope", "Envelope", ENV, &["basis", "upper", "lower"], P, NO, "basis = sma(close, length) (ema if exponential = 1); upper = basis·(1 + percent/100); lower = basis·(1 − percent/100); exponential 2 = ChartIQ triangular MA sma(sma(close, ceil(n/2)), ceil(n/2) + (n even ? 1 : 0)) (ChartIQ High Low Bands: length 10, percent 5)", &[19, 19, 19], false, envelope),
    def("ma_channel", "Moving Average Channel", CH, &["upper", "lower"], P, NO, "upper = sma(high, upper_length); lower = sma(low, lower_length) (charting-library offsets: use rule offsets)", &[19, 19], false, ma_channel),
    def("apz", "Adaptive Price Zone (Bandy)", APZ, &["basis", "upper", "lower"], P, NO, "a = 2/(√length + 1); basis = AMA(AMA(close, a), a); band = mult·AMA(AMA(high − low, a), a); upper/lower = basis ± band (Bandy QTS Fig 10.6 DblSmoothEMA; AMA seeded with the first value)", &[0, 0, 0], false, apz),
    def("alligator", "Williams Alligator", ALL, &["jaw", "teeth", "lips"], P, NO, "rma(hl2, length) as plotted: the value shown at bar i is rma[i − offset] (forward plot offset; causal)", &[20, 12, 7], false, alligator),
    def("median", "Median", MED, &["median", "upper", "lower", "ema"], P, NO, "median = median of hl2 over length (even length: mean of the two middle values); upper/lower = median ± mult·rma(tr, atr_length); ema = ema(median, length) (declared defaults 3/14/2); source picks the price instead of hl2 (thinkorswim MedianAverage = the median output on any price, without the ATR bands)", &[2, 13, 13, 4], false, median),
];

#[cfg(test)]
mod tests {
    use crate::lab::testdata::{assert_series, compiles, d1, go};
    const N: f64 = f64::NAN;
    #[test] fn envelope_sma_10pct() {
        let o = go("envelope", &d1(), &[("length", 3.), ("percent", 10.), ("exponential", 0.)]);
        assert_series(&o[0], &[N, N, 11.333333333333, 11.5, 11.066666666667, 10.666666666667, 10.966666666667, 11.766666666667]);
        assert_series(&o[1], &[N, N, 12.466666666667, 12.65, 12.173333333333, 11.733333333333, 12.063333333333, 12.943333333333]);
        assert_series(&o[2], &[N, N, 10.2, 10.35, 9.96, 9.6, 9.87, 10.59]);
    }
    #[test] fn ma_channel_high_low() {
        let o = go("ma_channel", &d1(), &[("upper_length", 2.), ("lower_length", 3.)]);
        assert_series(&o[0], &[N, 11.5, 12.25, 12.35, 11.85, 11.25, 11.5, 12.5]);
        assert_series(&o[1], &[N, N, 10.0, 10.6, 10.6, 10.1, 10.0, 10.5]);
    }
    #[test] fn apz_bandy() {
        let o = go("apz", &d1(), &[("length", 4.), ("mult", 2.)]);
        assert_series(&o[0], &[10.5, 10.944444444444, 11.462962962963, 11.314814814815, 10.802880658436, 10.744718792867, 11.251714677641, 11.907285474775]);
        assert_series(&o[1], &[14.5, 14.944444444444, 15.018518518519, 14.485185185185, 13.854732510288, 13.760356652949, 14.256378600823, 14.908657216888]);
        assert_series(&o[2], &[6.5, 6.944444444444, 7.907407407407, 8.144444444444, 7.751028806584, 7.729080932785, 8.247050754458, 8.905913732663]);
    }
    #[test] fn alligator_as_plotted() {
        let o = go("alligator", &d1(), &[("jaw_length", 3.), ("teeth_length", 2.), ("lips_length", 2.), ("jaw_offset", 2.), ("teeth_offset", 1.), ("lips_offset", 1.)]);
        assert_series(&o[0], &[N, N, N, N, 10.916666666667, 11.111111111111, 10.990740740741, 10.743827160494]);
        assert_series(&o[1], &[N, N, 10.5, 11.125, 11.3125, 11.03125, 10.640625, 10.9453125]);
    }
    #[test] fn median_with_atr_bands() {
        let o = go("median", &d1(), &[("length", 3.), ("atr_length", 2.), ("mult", 1.)]);
        assert_series(&o[0], &[N, N, 11.0, 11.5, 11.5, 10.75, 10.75, 11.25]);
        assert_series(&o[1], &[N, N, 12.75, 13.075, 13.0375, 12.26875, 12.259375, 12.7546875]);
        assert_series(&o[2], &[N, N, 9.25, 9.925, 9.9625, 9.23125, 9.240625, 9.7453125]);
        assert_series(&o[3], &[N, N, N, N, 11.333333333333, 11.041666666667, 10.895833333333, 11.072916666667]);
        assert_series(&go("median", &d1(), &[("length", 2.), ("atr_length", 2.), ("mult", 1.)])[0][..3], &[N, 10.5, 11.375]); // even length: mean of the middle two hl2
    }
    #[test] fn envelope_chartiq_triangular() {
        // n = 4: sma(sma(close, 2), 3)
        let o = go("envelope", &crate::lab::testdata::closes(&[1., 2., 3., 4., 5., 6.]), &[("length", 4.), ("percent", 10.), ("exponential", 2.)]);
        assert_series(&o[0], &[N, N, N, 2.5, 3.5, 4.5]);
        assert_series(&o[1], &[N, N, N, 2.75, 3.85, 4.95]);
    }
    #[test] fn median_of_a_chosen_price() {
        assert_series(&go("median", &d1(), &[("length", 3.), ("source", 1.)])[0], &[N, N, 11.5, 11.5, 11.0, 10.8, 10.8, 11.9]);
    }
    #[test] fn rules_reach_bands() { for (id, out) in [("envelope", "lower"), ("apz", "upper"), ("alligator", "lips"), ("median", "ema")] { compiles(id, out).unwrap(); } }
}
