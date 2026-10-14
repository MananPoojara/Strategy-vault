//! Indicator catalogue: declared parameters, outputs, data needs and warm-up for every indicator.
use super::bars::Bars;
use std::collections::BTreeMap;

pub type Params = BTreeMap<String, f64>;
pub type Output = Result<Vec<Vec<f64>>, String>;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Placement { Price, Pane }
#[derive(Clone, Copy, Debug)]
pub struct ParamSpec { pub key: &'static str, pub label: &'static str, pub default: f64, pub min: f64, pub max: f64, pub step: f64, pub integer: bool }
#[derive(Clone, Copy, Debug, Default)]
pub struct Needs { pub volume: bool, pub intraday: bool }
#[derive(Clone, Copy)]
pub struct IndicatorDef {
    pub id: &'static str, pub name: &'static str, pub category: &'static str,
    pub params: &'static [ParamSpec], pub outputs: &'static [&'static str], pub placement: Placement,
    pub needs: Needs, pub definition: &'static str,
    /// First non-NaN index of each output for default parameters on gap-free OHLCV (TradingView-verified where noted).
    pub first: &'static [usize],
    /// Outputs that are legitimately NaN after warm-up (e.g. pivots between pivots).
    pub sparse: bool,
    pub compute: Option<fn(&Bars, &Params) -> Output>,
}
/// Integer length parameter helper used by the family files.
pub const fn len(key: &'static str, label: &'static str, default: f64) -> ParamSpec {
    ParamSpec { key, label, default, min: 1., max: 2000., step: 1., integer: true }
}
pub const fn real(key: &'static str, label: &'static str, default: f64, min: f64, max: f64, step: f64) -> ParamSpec {
    ParamSpec { key, label, default, min, max, step, integer: false }
}
pub fn all() -> Vec<IndicatorDef> {
    [super::ma::DEFS, super::momentum::DEFS, super::trend::DEFS, super::volatility::DEFS, super::volume::DEFS, super::price::DEFS].concat()
}
pub fn get(id: &str) -> Option<IndicatorDef> { all().into_iter().find(|d| d.id == id) }
pub fn resolve_params(def: &IndicatorDef, given: &Params) -> Result<Params, String> {
    if let Some(k) = given.keys().find(|k| !def.params.iter().any(|p| p.key == k.as_str())) {
        return Err(format!("{}: unknown parameter '{k}'", def.id));
    }
    let mut out = Params::new();
    for p in def.params {
        let v = *given.get(p.key).unwrap_or(&p.default);
        if !v.is_finite() || v < p.min || v > p.max { return Err(format!("{}: {} must be between {} and {}", def.id, p.key, p.min, p.max)); }
        if p.integer && v.fract() != 0. { return Err(format!("{}: {} must be a whole number", def.id, p.key)); }
        out.insert(p.key.to_string(), v);
    }
    Ok(out)
}
pub fn run(def: &IndicatorDef, bars: &Bars, given: &Params) -> Output {
    let params = resolve_params(def, given)?;
    if def.needs.intraday { return Err("needs intraday data".into()); }
    if def.needs.volume && bars.volume.is_none() { return Err("needs volume".into()); }
    let f = def.compute.ok_or_else(|| "computed by a later batch".to_string())?;
    f(bars, &params)
}
pub fn p(params: &Params, key: &str) -> f64 { *params.get(key).unwrap_or_else(|| panic!("parameter {key} not declared")) }
pub fn n(params: &Params, key: &str) -> usize { p(params, key) as usize }

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lab::testdata::{close_eq, walk};
    fn defaults(d: &IndicatorDef) -> Params { resolve_params(d, &Params::new()).unwrap() }

    #[test] fn ids_unique_and_params_sane() {
        let defs = all();
        let mut ids: Vec<_> = defs.iter().map(|d| d.id).collect(); ids.sort(); let len = ids.len(); ids.dedup();
        assert_eq!(ids.len(), len, "duplicate indicator id");
        for d in &defs {
            assert_eq!(d.first.len(), d.outputs.len(), "{}: first[] per output", d.id);
            for p in d.params {
                assert!(p.min <= p.default && p.default <= p.max, "{}.{} default out of range", d.id, p.key);
                assert!(!p.integer || p.default.fract() == 0., "{}.{} integer default", d.id, p.key);
            }
        }
    }
    #[test]
    fn resolve_params_rejects_bad_input() {
        let d = get("sma").unwrap();
        let mut g = Params::new();
        g.insert("length".into(), 0.); assert!(resolve_params(&d, &g).unwrap_err().contains("length"));
        g.insert("length".into(), 10.5); assert!(resolve_params(&d, &g).unwrap_err().contains("length"));
        g.insert("length".into(), 1e6); assert!(resolve_params(&d, &g).is_err());
        g.clear(); g.insert("bogus".into(), 1.); assert!(resolve_params(&d, &g).unwrap_err().contains("bogus"));
    }
    #[test] fn every_indicator_length_warmup_and_no_gaps() {
        let bars = walk(700, 7, true);
        for d in all().iter().filter(|d| d.compute.is_some()) {
            let out = run(d, &bars, &Params::new()).unwrap_or_else(|e| panic!("{}: {e}", d.id));
            assert_eq!(out.len(), d.outputs.len(), "{}: output count", d.id);
            for (k, s) in out.iter().enumerate() {
                assert_eq!(s.len(), bars.len(), "{}.{} length", d.id, d.outputs[k]);
                let first = s.iter().position(|v| !v.is_nan()).unwrap_or(usize::MAX);
                assert_eq!(first, d.first[k], "{}.{} first valid bar", d.id, d.outputs[k]);
                if !d.sparse { assert!(s[first..].iter().all(|v| v.is_finite()), "{}.{} gap or inf after warm-up", d.id, d.outputs[k]); }
            }
        }
    }
    #[test] fn every_indicator_is_causal() {
        let bars = walk(400, 11, true);
        for d in all().iter().filter(|d| d.compute.is_some()) {
            let full = run(d, &bars, &Params::new()).unwrap();
            for cut in [150usize, 271, 399] {
                let mut pre = bars.clone();
                for v in [&mut pre.open, &mut pre.high, &mut pre.low, &mut pre.close] { v.truncate(cut); }
                pre.date.truncate(cut); pre.end.truncate(cut);
                if let Some(v) = pre.volume.as_mut() { v.truncate(cut); }
                let part = run(d, &pre, &Params::new()).unwrap();
                for (k, s) in part.iter().enumerate() {
                    for i in 0..cut { assert!(close_eq(s[i], full[k][i]), "{}.{} changes at bar {i} when later bars exist", d.id, d.outputs[k]); }
                }
            }
        }
    }
    #[test] fn volume_indicators_refuse_without_volume() {
        let bars = walk(300, 3, false);
        for d in all().iter().filter(|d| d.needs.volume && d.compute.is_some()) {
            assert_eq!(run(d, &bars, &Params::new()).unwrap_err(), "needs volume", "{}", d.id);
        }
    }
    #[test]
    #[ignore = "enabled in Task 9"]
    fn intraday_indicators_are_listed_not_computed() {
        for d in all().iter().filter(|d| d.needs.intraday) { assert!(d.compute.is_none(), "{}", d.id); }
        assert!(get("vwap").is_some() && get("opening_range").is_some());
    }
    #[test] fn flat_series_never_inf() {
        let mut bars = walk(300, 5, true);
        for i in 0..bars.len() { bars.open[i] = 50.; bars.high[i] = 50.; bars.low[i] = 50.; bars.close[i] = 50.; }
        for d in all().iter().filter(|d| d.compute.is_some()) {
            for s in run(d, &bars, &Params::new()).unwrap() { assert!(s.iter().all(|v| !v.is_infinite()), "{} produced inf on flat prices", d.id); }
        }
    }
    #[test] fn short_history_never_panics() {
        for len in [0usize, 1, 3] {
            let bars = walk(len, 9, true);
            for d in all().iter().filter(|d| d.compute.is_some()) {
                let out = run(d, &bars, &Params::new()).unwrap_or_else(|e| panic!("{}: {e}", d.id));
                assert_eq!(out.len(), d.outputs.len(), "{}: output count", d.id);
                for (k, s) in out.iter().enumerate() {
                    assert_eq!(s.len(), len, "{}.{} length", d.id, d.outputs[k]);
                    if !d.sparse { for i in 0..d.first[k].min(len) { assert!(s[i].is_nan(), "{}.{} value at warm-up bar {i}", d.id, d.outputs[k]); } }
                }
            }
        }
    }
    #[test] fn defaults_resolve_for_all() { for d in all() { defaults(&d); } }
}
