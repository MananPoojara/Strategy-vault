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
pub struct Needs { pub volume: bool, pub intraday: bool, pub breadth: bool, pub second_symbol: bool, pub fundamentals: bool, pub derivatives: bool, pub onchain: bool,
    /// Footprint (volume at bid/ask per price level per session) and order-book depth: see x_orderflow.rs.
    pub order_flow: bool, pub order_book: bool }
impl Needs {
    pub const NONE: Needs = Needs { volume: false, intraday: false, breadth: false, second_symbol: false, fundamentals: false, derivatives: false, onchain: false, order_flow: false, order_book: false };
    /// Data the lab does not hold for every market (volume is per instrument and checked at run time).
    pub const fn any_data(&self) -> bool { self.intraday || self.breadth || self.second_symbol || self.fundamentals || self.derivatives || self.onchain || self.order_flow || self.order_book }
}
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
    /// Why a listed-only entry is not computed ("not available yet — needs …"); empty for computed entries.
    pub note: &'static str,
}
/// Integer length parameter helper used by the family files.
pub const fn len(key: &'static str, label: &'static str, default: f64) -> ParamSpec {
    ParamSpec { key, label, default, min: 1., max: 2000., step: 1., integer: true }
}
pub const fn int(key: &'static str, label: &'static str, default: f64, min: f64, max: f64) -> ParamSpec {
    ParamSpec { key, label, default, min, max, step: 1., integer: true }
}
pub const fn real(key: &'static str, label: &'static str, default: f64, min: f64, max: f64, step: f64) -> ParamSpec {
    ParamSpec { key, label, default, min, max, step, integer: false }
}
/// The Phase 6/7 master list (302 ids); `all()` adds the 2026-10-19 cross-platform families.
pub fn base() -> Vec<IndicatorDef> {
    [super::ma::DEFS, super::momentum::DEFS, super::trend::DEFS, super::volatility::DEFS, super::volume::DEFS, super::price::DEFS, super::listed::DEFS, super::breadth::DEFS, super::ma2::DEFS, super::bands::DEFS, super::stats::DEFS, super::osc2::DEFS, super::vol2::DEFS, super::stops::DEFS, super::volume2::DEFS, super::periods::DEFS, super::patterns::DEFS, super::candles::DEFS, super::ratings::DEFS, super::futures::DEFS, super::pairs::DEFS].concat()
}
pub fn all() -> Vec<IndicatorDef> {
    [&base()[..], super::x_ma::DEFS, super::x_momentum::DEFS, super::x_momentum2::DEFS, super::x_trend::DEFS, super::x_volatility::DEFS, super::x_volume::DEFS, super::x_candles::DEFS, super::x_cycles::DEFS, super::x_derived::DEFS, super::x_atlas::DEFS, super::x_timeless::DEFS, super::x_census_trend::DEFS, super::x_census_momentum::DEFS, super::x_census_candles::DEFS, super::x_census_stats::DEFS, super::x_census_misc::DEFS, super::x_census_patterns::DEFS, super::x_census_harmonic::DEFS, super::x_census2_a::DEFS, super::x_census2_b::DEFS, super::x_census2_c::DEFS, super::x_census2_d::DEFS, super::x_orderflow::DEFS, super::x_smc::DEFS].concat()
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
    if def.needs.order_flow && def.compute.is_some() && bars.footprint.is_none() { return Err(format!("needs order-flow data|{}", bars.footprint_file)); }
    if def.needs.volume && bars.volume.is_none() {
        // volume only from bar k (NIFTY 50 before 2000): computed from bar k, empty before it (AmiBroker computes from the bars with volume)
        let plain = !(def.needs.intraday || def.needs.derivatives || def.needs.breadth || def.needs.second_symbol);
        if let (true, Some((k, tail)), Some(f)) = (plain, bars.late_volume_bars(), def.compute) {
            return Ok(f(&tail, &resolve_params(def, given)?)?.into_iter().map(|s| std::iter::repeat_n(f64::NAN, k).chain(s).collect()).collect());
        }
        return Err("needs volume".into());
    }
    if def.needs.derivatives && def.compute.is_some() && bars.expiry.is_none() { return Err("needs contract data".into()); }
    if def.needs.breadth && bars.breadth.is_none() { return Err("needs breadth".into()); }
    if def.needs.second_symbol && bars.second.is_none() { return Err("needs a second instrument".into()); }
    let f = def.compute.ok_or_else(|| if def.note.is_empty() { "computed by a later batch".to_string() } else { def.note.to_string() })?;
    f(bars, &params)
}
pub fn p(params: &Params, key: &str) -> f64 { *params.get(key).unwrap_or_else(|| panic!("parameter {key} not declared")) }
pub fn n(params: &Params, key: &str) -> usize { p(params, key) as usize }

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lab::testdata::{close_eq, walk};
    /// Breadth indicators need market breadth on the bars; Task 2 tests them with testdata::with_breadth.
    fn computable(d: &IndicatorDef) -> bool { d.compute.is_some() && !d.needs.breadth && !d.needs.derivatives && !d.needs.second_symbol && !d.needs.order_flow } // order flow: x_orderflow tests
    #[test] fn listed_entries_carry_a_note() {
        for d in all() {
            if d.compute.is_some() { assert!(d.note.is_empty(), "{}: computed entries have no note", d.id); continue; }
            assert!(!d.note.is_empty(), "{}: listed entry without a note", d.id);
            if d.needs.any_data() { assert!(d.note.starts_with("not available yet — needs"), "{}: {}", d.id, d.note); }
        }
    }
    #[test] fn sources_cover_every_indicator_once() {
        let rows: Vec<_> = crate::lab::sources::rows().collect();
        let ids: Vec<&str> = all().iter().map(|d| d.id).collect();
        for (id, tv, ab) in rows.iter().copied() {
            assert!(ids.contains(id), "sources row for unknown id {id}");
            assert!(!tv.is_empty() || !ab.is_empty() || *id == "opening_range", "{id}: no TradingView or AmiBroker name");
        }
        for id in &ids { assert_eq!(rows.iter().filter(|r| r.0 == *id).count(), 1, "{id}: exactly one sources row"); }
    }
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
        let bars = walk(1000, 7, true);
        for d in all().iter().filter(|d| computable(d)) {
            let out = run(d, &bars, &Params::new()).unwrap_or_else(|e| panic!("{}: {e}", d.id));
            assert_eq!(out.len(), d.outputs.len(), "{}: output count", d.id);
            for (k, s) in out.iter().enumerate() {
                assert_eq!(s.len(), bars.len(), "{}.{} length", d.id, d.outputs[k]);
                if d.sparse { continue; }
                let first = s.iter().position(|v| !v.is_nan()).unwrap_or(usize::MAX);
                assert_eq!(first, d.first[k], "{}.{} first valid bar", d.id, d.outputs[k]);
                if !d.sparse { assert!(s[first..].iter().all(|v| v.is_finite()), "{}.{} gap or inf after warm-up", d.id, d.outputs[k]); }
            }
        }
    }
    #[test] fn every_indicator_is_causal() {
        let bars = walk(400, 11, true);
        // declared readers of future session dates have their own property test (periods.rs)
        for d in all().iter().filter(|d| computable(d) && !crate::lab::periods::USES_SESSION_DATES.contains(&d.id)) {
            let full = run(d, &bars, &Params::new()).unwrap();
            // many cut points: a sparse lookahead (e.g. a fractal written on its pivot bar) only shows when a cut lands right after it
            for cut in (30usize..400).step_by(2) {
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
    #[test] fn catalogue_matches_the_phase6_master_list() {
        let defs = base();
        assert_eq!(defs.len(), 299, "Phase 6 master list: 66 Batch A + 233 new; Phase 7: session, sessions_left, expiry; 2026-10-26: cvd, volume_delta, up_down_volume, volume_profile moved to x_orderflow; 2026-10-29: period_end");
        assert_eq!(defs.iter().filter(|d| d.compute.is_some()).count(), 191);
        assert_eq!(defs.iter().filter(|d| d.compute.is_none()).count(), 108);
        let mut c = std::collections::BTreeMap::new();
        for (_, tv, ab) in crate::lab::sources::SOURCES {
            *c.entry(match (tv.is_empty(), ab.is_empty()) { (false, false) => "both", (false, true) => "tv", (true, false) => "ab", _ => "lab" }).or_insert(0) += 1;
        }
        assert_eq!((c["tv"], c["ab"], c["both"], c["lab"]), (231, 27, 40, 1)); // 3 tv rows and volume_profile (both) moved to x_orderflow; +period_end (ab)
    }
    #[test] fn volume_indicators_refuse_without_volume() {
        let bars = walk(300, 3, false);
        for d in all().iter().filter(|d| d.needs.volume && d.compute.is_some()) {
            assert_eq!(run(d, &bars, &Params::new()).unwrap_err(), "needs volume", "{}", d.id);
        }
    }
    #[test]
    fn intraday_indicators_are_listed_not_computed() {
        for d in all().iter().filter(|d| d.needs.intraday) { assert!(d.compute.is_none(), "{}", d.id); }
        assert!(get("vwap").is_some() && get("opening_range").is_some());
    }
    #[test] fn flat_series_never_inf() {
        let mut bars = walk(300, 5, true);
        for i in 0..bars.len() { bars.open[i] = 1017.3; bars.high[i] = 1017.3; bars.low[i] = 1017.3; bars.close[i] = 1017.3; }
        bars.volume = Some(vec![1000.; bars.len()]);
        for d in all().iter().filter(|d| computable(d)) {
            for s in run(d, &bars, &Params::new()).unwrap() { assert!(s.iter().all(|v| !v.is_infinite()), "{} produced inf on flat prices", d.id); }
        }
    }
    #[test] fn short_history_never_panics() {
        for len in [0usize, 1, 3] {
            let bars = walk(len, 9, true);
            for d in all().iter().filter(|d| computable(d)) {
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
