//! Market breadth family (TradingView Advance/Decline Line, A/D Ratio, CVI; AmiBroker ADLine, Trin, AdvIssues…):
//! statistics of the instrument's market (NSE for India, NYSE for US), daily bars only.
use super::bars::{Bars, Breadth};
use super::catalogue::{IndicatorDef, Needs, Output, Params, Placement};
use super::ta::NA;

const CAT: &str = "Breadth";
const B: Needs = Needs { breadth: true, ..Needs::NONE };
fn br(b: &Bars) -> &Breadth { b.breadth.as_ref().expect("catalogue::run checks needs.breadth") }
fn div(a: f64, b: f64) -> f64 { if b == 0. || a.is_nan() || b.is_nan() { NA } else { a / b } }
/// Running total; a NaN term outputs NaN and the total continues from the last value.
fn cum(x: impl Iterator<Item = f64>) -> Vec<f64> { let mut s = 0.; x.map(|v| if v.is_nan() { NA } else { s += v; s }).collect() }

fn breadth(b: &Bars, _: &Params) -> Output { let r = br(b); Ok(vec![r.adv.clone(), r.dec.clone(), r.unch.clone(), r.adv_vol.clone(), r.dec_vol.clone(), r.unch_vol.clone()]) }
fn adl(b: &Bars, _: &Params) -> Output { let r = br(b); Ok(vec![cum(r.adv.iter().zip(&r.dec).map(|(a, d)| a - d))]) }
fn ad_ratio(b: &Bars, _: &Params) -> Output { let r = br(b); Ok(vec![r.adv.iter().zip(&r.dec).map(|(a, d)| div(*a, *d)).collect()]) }
fn cvi(b: &Bars, _: &Params) -> Output { let r = br(b); Ok(vec![cum(r.adv_vol.iter().zip(&r.dec_vol).map(|(a, d)| a - d))]) }
fn trin(b: &Bars, _: &Params) -> Output {
    let r = br(b);
    Ok(vec![(0..b.len()).map(|i| div(div(r.adv[i], r.dec[i]), div(r.adv_vol[i], r.dec_vol[i]))).collect()])
}
const fn def(id: &'static str, name: &'static str, outputs: &'static [&'static str], definition: &'static str, first: &'static [usize], f: fn(&Bars, &Params) -> Output) -> IndicatorDef {
    IndicatorDef { id, name, category: CAT, params: &[], outputs, placement: Placement::Pane, needs: B, definition, first, sparse: false, compute: Some(f), note: "" }
}
pub const DEFS: &[IndicatorDef] = &[
    def("breadth", "Market Breadth (raw)", &["advances", "declines", "unchanged", "adv_volume", "dec_volume", "unch_volume"],
        "the market's advancing/declining/unchanged issues and volumes (NSE for India, NYSE for US) aligned to this instrument's dates; NaN where the source has no value; unch_volume is US only", &[0, 0, 0, 0, 0, 0], breadth),
    def("adl", "Advance/Decline Line", &["adl"], "cumulative (advances − declines); a day without breadth is NaN and the total continues", &[0], adl),
    def("ad_ratio", "Advance/Decline Ratio", &["ratio"], "advances / declines; zero declines → NaN", &[0], ad_ratio),
    def("cvi", "Cumulative Volume Index", &["cvi"], "cumulative (advancing volume − declining volume); missing days NaN, total continues", &[0], cvi),
    def("trin", "TRIN (Arms Index)", &["trin"], "(advances/declines) / (advancing volume/declining volume); any zero → NaN", &[0], trin),
];

#[cfg(test)]
mod tests {
    use crate::lab::bars::{Bars, Breadth};
    use crate::lab::catalogue::{get, run, Params};
    use crate::lab::resample::Tf;
    use crate::lab::rules::SeriesRef;
    use crate::lab::series::{series, Cache, Frames};
    use crate::lab::testdata::{assert_series, close_eq, closes, compiles, cut, go, walk, with_breadth};
    const N: f64 = f64::NAN;
    fn bars4() -> Bars {
        let mut b = closes(&[1.; 4]);
        b.breadth = Some(Breadth { adv: vec![100., N, 200., 150.], dec: vec![50., 60., 100., 150.], unch: vec![5.; 4],
            adv_vol: vec![1000., N, 3000., 2000.], dec_vol: vec![500., 600., 1000., 4000.], unch_vol: vec![N; 4] });
        b
    }
    #[test] fn breadth_gaps_stay_nan_and_cumulative_lines_continue() {
        let b = bars4();
        assert_series(&go("adl", &b, &[])[0], &[50., N, 150., 150.]);
        assert_series(&go("ad_ratio", &b, &[])[0], &[2., N, 2., 1.]);
        assert_series(&go("cvi", &b, &[])[0], &[500., N, 2500., 500.]);
        assert_series(&go("trin", &b, &[])[0], &[1., N, 2. / 3., 2.]);
        let raw = go("breadth", &b, &[]);
        assert_eq!(raw.len(), 6);
        assert_series(&raw[0], &[100., N, 200., 150.]);
        assert_series(&raw[5], &[N; 4]);
    }
    #[test] fn breadth_needs_daily_market_data() {
        assert_eq!(run(&get("adl").unwrap(), &closes(&[1., 2.]), &Params::new()).unwrap_err(), "needs breadth");
        let fr = Frames::new(with_breadth(walk(60, 3, true)));
        let r = |tf| SeriesRef { id: "adl".into(), params: Params::new(), tf, out: 0, offset: 0, foreign: Default::default(), formula: None };
        assert!(series(&fr, &mut Cache::new(), &r(Tf::D)).is_ok());
        for tf in [Tf::W, Tf::M] {
            let e = series(&fr, &mut Cache::new(), &r(tf)).unwrap_err();
            assert_eq!(e.status(), "needs_data");
            assert!(e.message().contains("breadth"), "{}", e.message());
        }
    }
    #[test] fn breadth_family_is_causal_finite_and_reachable() {
        let full = with_breadth(walk(400, 11, true));
        for id in ["breadth", "adl", "ad_ratio", "cvi", "trin"] {
            let d = get(id).unwrap();
            let want = run(&d, &full, &Params::new()).unwrap();
            for c in [150usize, 271, 399] {
                let got = run(&d, &cut(&full, c), &Params::new()).unwrap();
                for (k, s) in got.iter().enumerate() { for i in 0..c { assert!(close_eq(s[i], want[k][i]), "{id}.{k} bar {i}"); } }
            }
            for s in &want { assert!(s.iter().all(|v| v.is_finite()), "{id}: synthetic breadth has no gaps"); }
            compiles(id, d.outputs[d.outputs.len() - 1]).unwrap();
        }
    }
}
