//! Gap group "g" (2026-11-27 census check of Volume & Money Flow: entries our catalogue lacked).
//! Computed: Weis Wave (ATAS WeissWave.cs), Volume Accumulation Percent (MotiveWave; MultiCharts true-range variant) and
//! Normalized Volume (Sierra Chart study 451). Not added: Volume Accumulation Oscillator = MotiveWave's name for the
//! thinkorswim Volume Accumulation already in x_derived (volume·(close − (high + low)/2), MotiveWave's price input at its
//! default, close). Formulas: see each definition string.
use super::bars::Bars;
use super::catalogue::{int, len, n, p, IndicatorDef, Needs, Output, ParamSpec, Params, Placement};
use super::ta::{self, NA};

const VOL: &str = "Volume";
const V: Needs = Needs { volume: true, ..Needs::NONE };
const PN: Placement = Placement::Pane;
fn vol(b: &Bars) -> &[f64] { b.volume.as_deref().unwrap_or(&[]) }

/// ATAS WeissWave.cs: volume summed while the candle keeps the previous candle's direction, sign(open − close).
fn weis_wave(b: &Bars, _: &Params) -> Output {
    let (v, len) = (vol(b), b.len());
    let (mut w, mut dir) = (vec![NA; len], vec![NA; len]);
    // C# Math.Sign: a doji is 0 (f64::signum would give ±1)
    let side = |i: usize| if b.open[i] == b.close[i] { 0. } else { (b.open[i] - b.close[i]).signum() };
    for i in 0..len {
        w[i] = if i > 0 && side(i) == side(i - 1) { w[i - 1] + v[i] } else { v[i] };
        dir[i] = if b.open[i] < b.close[i] { 1. } else { -1. };
    }
    Ok(vec![w, dir])
}

const VAP: &[ParamSpec] = &[len("length", "Length", 21.), int("variant", "Variant (0 MotiveWave: high/low; 1 MultiCharts: true high/low, Length 10)", 0., 0., 1.)];
fn vol_accum_pct(b: &Bars, prm: &Params) -> Output {
    let (v, k, mc, len) = (vol(b), n(prm, "length"), p(prm, "variant") == 1., b.len());
    let mut xt = vec![0.; len];
    let mut ok = vec![true; len];
    for i in 0..len {
        let (h, l, c) = if mc && i > 0 { (b.high[i].max(b.close[i - 1]), b.low[i].min(b.close[i - 1]), b.close[i]) } else { (b.high[i], b.low[i], b.close[i]) };
        if h != l { xt[i] = ((c - l) - (h - c)) / (h - l); }
        // MultiCharts: a zero true range plots nothing and XT keeps its last value (EasyLanguage variables carry over)
        else if mc { xt[i] = if i > 0 { xt[i - 1] } else { 0. }; ok[i] = false; }
    }
    let (tv, tva) = (ta::sum(v, k), ta::sum(&(0..len).map(|i| v[i] * xt[i]).collect::<Vec<_>>(), k));
    Ok(vec![(0..len).map(|i| {
        if tv[i].is_nan() { NA } else if tv[i] == 0. { if mc { NA } else { 0. } } else if !ok[i] { NA } else { 100. * tva[i] / tv[i] }
    }).collect()])
}

const NV: &[ParamSpec] = &[len("length", "Length", 10.)];
/// Sierra Chart 451: 100·volume ÷ sma(volume, n); a zero average repeats the previous value.
fn normalized_volume(b: &Bars, prm: &Params) -> Output {
    let (v, k) = (vol(b), n(prm, "length"));
    let ma = ta::sma(v, k);
    let mut o = vec![NA; b.len()];
    for i in 0..b.len() {
        if ma[i].is_nan() { continue; }
        o[i] = if ma[i] == 0. { if i > 0 { o[i - 1] } else { NA } } else { 100. * v[i] / ma[i] };
    }
    Ok(vec![o])
}

pub const DEFS: &[IndicatorDef] = &[
    IndicatorDef { id: "weis_wave", name: "Weis Wave", category: VOL, params: &[], outputs: &["wave", "dir"], placement: PN, needs: V,
        definition: "Weis Wave as ATAS computes it (WeissWave.cs, open source): wave = volume added up while each candle points the same way as the one before, sign(open − close) (a doji starts a new wave unless the bar before was a doji too); a change of direction starts again from this bar's volume. dir = +1 when the bar closes up (open < close), else −1 (ATAS colours a doji as down). ATAS's Filter input only recolours bars above it and is not offered. Not David Weis's swing-based wave and not LazyBear's TradingView port, which turns on closes after a 2-bar trend.",
        first: &[0, 0], sparse: false, compute: Some(weis_wave), note: "" },
    IndicatorDef { id: "vol_accum_pct", name: "Volume Accumulation Percent", category: VOL, params: VAP, outputs: &["vap"], placement: PN, needs: V,
        definition: "Volume Accumulation Percent = 100·Σ(volume·xT) ÷ Σ volume over Length bars, xT = ((close − low) − (high − close)) ÷ (high − low). Variant 0 MotiveWave (Length 21): the bar's high and low, a zero range gives xT = 0 and a zero volume total gives 0. Variant 1 MultiCharts (Omega Research EasyLanguage, Length 10 there): true high max(high, previous close) and true low min(low, previous close); a zero true range plots nothing and keeps the previous xT inside the sums, a zero volume total plots nothing. NaN for the first Length − 1 bars.",
        first: &[20], sparse: false, compute: Some(vol_accum_pct), note: "" },
    IndicatorDef { id: "normalized_volume", name: "Normalized Volume", category: VOL, params: NV, outputs: &["nv"], placement: PN, needs: V,
        definition: "Normalized Volume (Sierra Chart study 451) = 100·volume ÷ simple moving average of volume over Length bars (default 10, from Sierra's study code); when the average is 0 the previous value is repeated. Sierra draws High Volume 150 and Low Volume 75 lines (shown as levels). Sierra's other moving-average types are not offered. NaN for the first Length − 1 bars.",
        first: &[9], sparse: false, compute: Some(normalized_volume), note: "" },
];
pub const SOURCES: &[(&str, &str, &str)] = &[
    ("weis_wave", "ATAS: Weis Wave", ""),
    ("vol_accum_pct", "MotiveWave: Volume Accum Percent; MultiCharts: Volume Accum %", ""),
    ("normalized_volume", "Sierra Chart: Normalized Volume (study 451)", ""),
];

#[cfg(test)]
mod tests {
    use crate::lab::catalogue::{get, run};
    use crate::lab::ta::NA;

    fn bars(o: &[f64], h: &[f64], l: &[f64], c: &[f64], v: &[f64]) -> crate::lab::bars::Bars {
        let rows: Vec<crate::Row> = (0..o.len()).map(|i| crate::Row { ticker: "T".into(), date: format!("2024-01-{:02}", i + 1), open: o[i], high: h[i], low: l[i], close: c[i], volume: Some(v[i]) }).collect();
        crate::lab::bars::Bars::from_rows(&rows)
    }
    fn out(id: &str, b: &crate::lab::bars::Bars, prm: &[(&str, f64)]) -> Vec<Vec<f64>> {
        run(&get(id).unwrap(), b, &prm.iter().map(|(k, v)| (k.to_string(), *v)).collect()).unwrap()
    }
    fn same(a: &[f64], b: &[f64]) -> bool { a.len() == b.len() && a.iter().zip(b).all(|(x, y)| (x.is_nan() && y.is_nan()) || (x - y).abs() < 1e-9) }

    #[test] fn weis_wave_sums_same_direction_candles() {
        // up, up, down, doji, doji, up
        let o = [10., 11., 13., 12., 12., 12.];
        let c = [11., 12., 12., 12., 12., 13.];
        let b = bars(&o, &[14.; 6], &[9.; 6], &c, &[5., 7., 4., 2., 3., 6.]);
        let w = out("weis_wave", &b, &[]);
        assert!(same(&w[0], &[5., 12., 4., 2., 5., 6.]), "{:?}", w[0]);
        assert!(same(&w[1], &[1., 1., -1., -1., -1., 1.]));
    }
    #[test] fn volume_accumulation_percent_both_variants() {
        // bar 1: xT = ((12−10) − (13−12))/3 = 1/3; bar 2: high = low = close = 11 → MotiveWave xT 0
        let b = bars(&[10., 11., 11.], &[12., 13., 11.], &[9., 10., 11.], &[11., 12., 11.], &[100., 200., 300.]);
        // bar 0 xT = ((11−9) − (12−11))/3 = 1/3
        let mw = out("vol_accum_pct", &b, &[("length", 2.)]);
        assert!(same(&mw[0], &[NA, 100. * (100. / 3. + 200. / 3.) / 300., 100. * (200. / 3.) / 500.]));
        // MultiCharts: bar 1 true high max(13, 11) 13, true low min(10, 11) 10 → 1/3; bar 2 true range max(11,12)=12, min(11,12)=11 → xT = ((11−11) − (12−11))/1 = −1
        let mc = out("vol_accum_pct", &b, &[("length", 2.), ("variant", 1.)]);
        assert!(same(&mc[0], &[NA, 100. * (100. / 3. + 200. / 3.) / 300., 100. * (200. / 3. - 300.) / 500.]));
        // a flat bar after a flat close: zero true range → nothing plotted, xT carried into the next sum
        let f = bars(&[10., 10., 10.], &[11., 10., 12.], &[9., 10., 10.], &[10., 10., 11.], &[10., 10., 10.]);
        let m = out("vol_accum_pct", &f, &[("length", 2.), ("variant", 1.)]);
        // bar 0 xT 0; bar 1 TR 0 → xT 0 carried, NaN; bar 2: th 12, tl 10 → xT = (1 − 1)/2 = 0 → 0
        assert!(same(&m[0], &[NA, NA, 0.]));
    }
    #[test] fn normalized_volume_against_its_average() {
        let b = bars(&[1.; 4], &[2.; 4], &[0.5; 4], &[1.; 4], &[10., 30., 0., 0.]);
        let nv = out("normalized_volume", &b, &[("length", 2.)]);
        // sma 20, 15, 0 → 150, 0, repeat 0
        assert!(same(&nv[0], &[NA, 150., 0., 0.]));
        assert!(same(&out("normalized_volume", &bars(&[1.; 2], &[2.; 2], &[0.5; 2], &[1.; 2], &[0., 0.]), &[("length", 2.)])[0], &[NA, NA]));
    }
}
