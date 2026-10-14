//! Volume II and period-anchored price indicators (TradingView built-ins, Phase 6). Formulas: see each definition string.
use super::bars::Bars;
use super::catalogue::{int, len, n, p, real, IndicatorDef, Needs, Output, ParamSpec, Params, Placement};
use super::resample::period_key;
use super::ta::{self, NA};

const CAT: &str = "Volume";
#[allow(clippy::too_many_arguments)]
const fn def(id: &'static str, name: &'static str, params: &'static [ParamSpec], outputs: &'static [&'static str], placement: Placement,
             needs: Needs, definition: &'static str, first: &'static [usize], sparse: bool, f: fn(&Bars, &Params) -> Output) -> IndicatorDef {
    IndicatorDef { id, name, category: CAT, params, outputs, placement, needs, definition, first, sparse, compute: Some(f), note: "" }
}
const PR: Placement = Placement::Price;
const PN: Placement = Placement::Pane;
const NO: Needs = Needs::NONE;
const VOL: Needs = Needs { volume: true, ..Needs::NONE };
const KL: &[ParamSpec] = &[len("fast", "Fast length", 34.), len("slow", "Slow length", 55.), len("signal", "Signal length", 13.)];
const VL: &[ParamSpec] = &[len("length", "Length", 20.)];
const VW: &[ParamSpec] = &[int("anchor", "Anchor (1 week, 2 month, 3 quarter, 4 year)", 1., 1., 4.), real("mult", "Multiplier", 1., 0.1, 10., 0.1)];
const TW: &[ParamSpec] = &[int("anchor", "Anchor (1 week, 2 month, 3 quarter, 4 year)", 1., 1., 4.)];

fn vol(b: &Bars) -> &[f64] { b.volume.as_deref().unwrap_or(&[]) }
fn klinger(b: &Bars, prm: &Params) -> Output {
    let (v, ch) = (vol(b), ta::change(&b.hlc3(), 1));
    let sv: Vec<f64> = (0..b.len()).map(|i| if ch[i] >= 0. { v[i] } else { -v[i] }).collect();
    let k: Vec<f64> = ta::ema(&sv, n(prm, "fast")).iter().zip(ta::ema(&sv, n(prm, "slow"))).map(|(f, s)| f - s).collect();
    let sig = ta::ema(&k, n(prm, "signal"));
    Ok(vec![k, sig])
}
fn volume(b: &Bars, prm: &Params) -> Output { Ok(vec![vol(b).to_vec(), ta::sma(vol(b), n(prm, "length"))]) }
fn vwap_period(b: &Bars, prm: &Params) -> Output {
    let (a, m, v, src) = (n(prm, "anchor"), p(prm, "mult"), vol(b), b.hlc3());
    let len = b.len();
    let (mut w, mut u, mut l) = (vec![NA; len], vec![NA; len], vec![NA; len]);
    let (mut last, mut sv, mut spv, mut spv2) = (None, 0., 0., 0.);
    for i in 0..len {
        let k = period_key(&b.date[i], a);
        if last != Some(k) { sv = 0.; spv = 0.; spv2 = 0.; last = Some(k); }
        sv += v[i]; spv += src[i] * v[i]; spv2 += src[i] * src[i] * v[i];
        if sv == 0. { continue; }
        let x = spv / sv; let sd = (spv2 / sv - x * x).max(0.).sqrt();
        w[i] = x; u[i] = x + m * sd; l[i] = x - m * sd;
    }
    Ok(vec![w, u, l])
}
fn twap(b: &Bars, prm: &Params) -> Output {
    let (a, src) = (n(prm, "anchor"), b.ohlc4());
    let (mut last, mut s, mut c) = (None, 0., 0.);
    Ok(vec![(0..b.len()).map(|i| {
        let k = period_key(&b.date[i], a);
        if last != Some(k) { s = 0.; c = 0.; last = Some(k); }
        s += src[i]; c += 1.; s / c
    }).collect()])
}

pub const DEFS: &[IndicatorDef] = &[
    def("klinger", "Klinger Oscillator", KL, &["kvo", "signal"], PN, VOL, "sv = change(hlc3) ≥ 0 ? volume : −volume (bar 0: NaN change → −volume, TradingView quirk); kvo = ema(sv, fast) − ema(sv, slow); signal = ema(kvo, signal)", &[54, 66], false, klinger),
    def("vol", "Volume", VL, &["volume", "ma"], PN, VOL, "volume and sma(volume, length)", &[0, 19], false, volume),
    def("vwap_period", "VWAP (anchored period)", VW, &["vwap", "upper", "lower"], PR, VOL, "from the first bar of each anchor period: vwap = Σ(hlc3·v)/Σv; sd = √max(Σ(v·hlc3²)/Σv − vwap², 0); upper/lower = vwap ± mult·sd; Σv = 0 → NaN (session VWAP stays listed: it needs intraday bars)", &[0, 0, 0], false, vwap_period),
    def("twap", "Time Weighted Average Price", TW, &["twap"], PR, NO, "mean of ohlc4 over the bars since the anchor period began", &[0], false, twap),
];

#[cfg(test)]
mod tests {
    use crate::lab::testdata::{assert_series, compiles, d1, go};
    const N: f64 = f64::NAN;
    #[test] fn klinger_short() {
        let o = go("klinger", &d1(), &[("fast", 2.), ("slow", 3.), ("signal", 2.)]);
        assert_series(&o[0], &[N, N, 26.666666666667, -22.222222222222, -19.62962962963, -9.320987654321, 40.504115226337, 31.973593964335]);
        assert_series(&o[1], &[N, N, N, 2.222222222222, -12.345679012346, -10.329218106996, 23.559670781893, 29.168952903521]);
    }
    #[test] fn volume_and_its_average() {
        let o = go("vol", &d1(), &[("length", 2.)]);
        assert_series(&o[0], &[100., 120., 90., 150., 130., 110., 160., 140.]);
        assert_series(&o[1], &[N, 110., 105., 120., 140., 120., 135., 150.]);
    }
    #[test] fn weekly_anchored_vwap_restarts_on_monday() {
        // d1: 2024-01-01 is a Monday; index 7 (2024-01-08) starts the next week
        let o = go("vwap_period", &d1(), &[("anchor", 1.), ("mult", 1.)]);
        assert_series(&o[0], &[10.166666666667, 10.712121212121, 11.037634408602, 11.134057971014, 11.009039548023, 10.918571428571, 11.020542635659, 12.366666666667]);
        assert_series(&o[1], &[10.166666666667, 11.210050809853, 11.697152095707, 11.692934048575, 11.555689351692, 11.462415352094, 11.555548941127, 12.366666666667]);
        assert_series(&o[2], &[10.166666666667, 10.214191614389, 10.378116721497, 10.575181893454, 10.462389744353, 10.374727505049, 10.485536330191, 12.366666666667]);
        assert_series(&go("twap", &d1(), &[("anchor", 1.)])[0], &[10.125, 10.5625, 10.958333333333, 11.09375, 11.01, 10.904166666667, 10.960714285714, 12.25]);
    }
    #[test] fn rules_reach_volume2() { for (id, out) in [("klinger", "signal"), ("vwap_period", "lower"), ("twap", "twap"), ("vol", "ma")] { compiles(id, out).unwrap(); } }
}
