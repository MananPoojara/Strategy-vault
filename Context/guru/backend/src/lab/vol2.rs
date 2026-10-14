//! Volatility family II (TradingView / AmiBroker built-ins, Phase 6). Formulas: see each definition string.
use super::bars::Bars;
use super::catalogue::{int, len, n, p, real, IndicatorDef, Needs, Output, ParamSpec, Params, Placement};
use super::ta::{self, NA};

const CAT: &str = "Volatility";
fn div(a: f64, b: f64) -> f64 { if b == 0. || a.is_nan() || b.is_nan() { NA } else { a / b } }
#[allow(clippy::too_many_arguments)]
const fn def(id: &'static str, name: &'static str, params: &'static [ParamSpec], outputs: &'static [&'static str], placement: Placement,
             needs: Needs, definition: &'static str, first: &'static [usize], sparse: bool, f: fn(&Bars, &Params) -> Output) -> IndicatorDef {
    IndicatorDef { id, name, category: CAT, params, outputs, placement, needs, definition, first, sparse, compute: Some(f), note: "" }
}
const PN: Placement = Placement::Pane;
const NO: Needs = Needs::NONE;
const CV: &[ParamSpec] = &[len("length", "Length", 10.), len("roc", "Rate of change length", 10.)];
const MI: &[ParamSpec] = &[int("ema", "EMA length", 9., 2., 2000.), len("length", "Sum length", 10.)];
const UL: &[ParamSpec] = &[int("length", "Length", 14., 2., 2000.)];
const L14: &[ParamSpec] = &[len("length", "Length", 14.)];
const VZ: &[ParamSpec] = &[len("length", "Length", 10.), ParamSpec { max: 366., ..len("days", "Days per year", 252.) }];
const BT: &[ParamSpec] = &[len("short", "Short length", 20.), len("long", "Long length", 50.), real("mult", "Multiplier", 2., 0.1, 10., 0.1)];

fn range(b: &Bars) -> Vec<f64> { (0..b.len()).map(|i| b.high[i] - b.low[i]).collect() }
fn chaikin_vol(b: &Bars, prm: &Params) -> Output {
    let (e, r) = (ta::ema(&range(b), n(prm, "length")), n(prm, "roc"));
    Ok(vec![(0..b.len()).map(|i| if i < r { NA } else { div(100. * (e[i] - e[i - r]), e[i - r]) }).collect()])
}
fn mass_index(b: &Bars, prm: &Params) -> Output {
    let k = n(prm, "ema"); let e1 = ta::ema(&range(b), k); let e2 = ta::ema(&e1, k);
    Ok(vec![ta::sum(&(0..b.len()).map(|i| div(e1[i], e2[i])).collect::<Vec<_>>(), n(prm, "length"))])
}
fn ulcer(b: &Bars, prm: &Params) -> Output {
    let k = n(prm, "length"); let hi = ta::highest(&b.close, k);
    let dd2: Vec<f64> = (0..b.len()).map(|i| div(100. * (b.close[i] - hi[i]), hi[i]).powi(2)).collect();
    Ok(vec![ta::sma(&dd2, k).iter().map(|v| v.sqrt()).collect()])
}
fn adr(b: &Bars, prm: &Params) -> Output { Ok(vec![ta::sma(&range(b), n(prm, "length"))]) }
fn annualised(x: &[f64], k: usize, days: f64) -> Vec<f64> { ta::sma(x, k).iter().map(|v| if *v < 0. { NA } else { 100. * (v * days).sqrt() }).collect() }
fn volatility_zt(b: &Bars, prm: &Params) -> Output {
    let c = &b.close;
    let r2: Vec<f64> = (0..c.len()).map(|i| if i > 0 && c[i] > 0. && c[i - 1] > 0. { (c[i] / c[i - 1]).ln().powi(2) } else { NA }).collect();
    Ok(vec![annualised(&r2, n(prm, "length"), p(prm, "days"))])
}
fn volatility_ohlc(b: &Bars, prm: &Params) -> Output {
    let gk: Vec<f64> = (0..b.len()).map(|i| {
        let (o, h, l, c) = (b.open[i], b.high[i], b.low[i], b.close[i]);
        if o <= 0. || h <= 0. || l <= 0. || c <= 0. { NA } else { 0.5 * (h / l).ln().powi(2) - (2. * 2f64.ln() - 1.) * (c / o).ln().powi(2) }
    }).collect();
    Ok(vec![annualised(&gk, n(prm, "length"), p(prm, "days"))])
}
fn bbtrend(b: &Bars, prm: &Params) -> Output {
    let m = p(prm, "mult");
    let bands = |k: usize| { let (ba, sd) = (ta::sma(&b.close, k), ta::stdev(&b.close, k)); (ba.clone(), (0..b.len()).map(|i| ba[i] + m * sd[i]).collect::<Vec<_>>(), (0..b.len()).map(|i| ba[i] - m * sd[i]).collect::<Vec<_>>()) };
    let ((sb, su, sl), (_, lu, ll)) = (bands(n(prm, "short")), bands(n(prm, "long")));
    Ok(vec![(0..b.len()).map(|i| div(100. * ((sl[i] - ll[i]).abs() - (su[i] - lu[i]).abs()), sb[i])).collect()])
}

pub const DEFS: &[IndicatorDef] = &[
    def("chaikin_vol", "Chaikin Volatility", CV, &["cv"], PN, NO, "e = ema(high − low, length); 100·(e − e[roc])/e[roc] (declared defaults length 10, roc 10)", &[19], false, chaikin_vol),
    def("mass_index", "Mass Index", MI, &["mi"], PN, NO, "e1 = ema(high − low, ema); e2 = ema(e1, ema); sum(e1/e2, length)", &[25], false, mass_index),
    def("ulcer", "Ulcer Index", UL, &["ulcer"], PN, NO, "dd = 100·(close − highest(close, n))/highest(close, n); √sma(dd², n)", &[26], false, ulcer),
    def("adr", "Average Daily Range", L14, &["adr"], PN, NO, "sma(high − low, n) (= mean high − mean low; declared default 14)", &[13], false, adr),
    def("volatility_zt", "Volatility Zero Trend Close-to-Close", VZ, &["vol"], PN, NO, "100·√(days·sma(r², n)), r = ln(close/close[1]); non-positive prices → NaN (declared defaults length 10, days 252)", &[10], false, volatility_zt),
    def("volatility_ohlc", "Volatility O-H-L-C", VZ, &["vol"], PN, NO, "Garman–Klass (declared): 100·√(days·sma(0.5·ln(h/l)² − (2·ln 2 − 1)·ln(c/o)², n)); a negative mean or non-positive price → NaN (declared defaults length 10, days 252)", &[9], false, volatility_ohlc),
    def("bbtrend", "BBTrend", BT, &["bbtrend"], PN, NO, "100·(|lower_s − lower_l| − |upper_s − upper_l|)/basis_s with Bollinger(close, short/long, mult) (population stdev)", &[49], false, bbtrend),
];

#[cfg(test)]
mod tests {
    use crate::lab::testdata::{assert_series, compiles, d1, go};
    const N: f64 = f64::NAN;
    fn one(id: &str, kv: &[(&str, f64)]) -> Vec<f64> { go(id, &d1(), kv).remove(0) }
    #[test] fn range_based_volatility() {
        assert_series(&one("chaikin_vol", &[("length", 2.), ("roc", 2.)]), &[N, N, N, -25.555555555556, -10.222222222222, 0.663349917081, 0.2200220022, 0.073219842577]);
        assert_series(&one("mass_index", &[("ema", 2.), ("length", 2.)]), &[N, N, N, 1.837497375604, 1.905038215103, 1.969272795197, 1.990359400171, 1.997017668111]);
        assert_series(&one("adr", &[("length", 3.)]), &[N, N, 1.833333333333, 1.633333333333, 1.466666666667, 1.466666666667, 1.5, 1.5]);
    }
    #[test] fn drawdown_and_return_volatility() {
        assert_series(&one("ulcer", &[("length", 3.)]), &[N, N, N, N, 9.906974722293, 9.962433263154, 8.723641921506, 1.049727762163]);
        assert_series(&one("volatility_zt", &[("length", 3.), ("days", 252.)]), &[N, N, N, 121.790223521499, 112.562429044232, 117.869172248916, 124.240969541656, 115.719494008877]);
        assert_series(&one("volatility_ohlc", &[("length", 3.), ("days", 252.)]), &[N, N, 183.553605141884, 146.064378916513, 128.112965729936, 134.551337938428, 136.956224350665, 132.876028180043]);
    }
    #[test] fn bbtrend_short_long() {
        assert_series(&one("bbtrend", &[("short", 2.), ("long", 3.), ("mult", 2.)]), &[N, N, 7.09219858156, 0.0, -8.805031446541, -1.521319399601, 5.425799802576, 7.891156462585]);
    }
    #[test] fn rules_reach_vol2() { for (id, out) in [("mass_index", "mi"), ("bbtrend", "bbtrend"), ("volatility_ohlc", "vol")] { compiles(id, out).unwrap(); } }
}
