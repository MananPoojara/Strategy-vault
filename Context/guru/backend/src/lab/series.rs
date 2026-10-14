//! Operands → daily-aligned series and compiled conditions. Indicators run on their own timeframe's bars;
//! W/M values reach daily bars through `resample::expand` (usable after the period's last daily close).
use super::bars::Bars;
use super::catalogue;
use super::resample::{self, Tf};
use super::rules::{Cond, LabError, Op, Operand, SeriesRef, Spec};
use std::collections::BTreeMap;
use std::rc::Rc;

/// Daily bars of one instrument plus their weekly and monthly resamples (built once per request).
pub struct Frames { pub d: Bars, pub w: Bars, pub m: Bars }
impl Frames {
    pub fn new(d: Bars) -> Frames { let (w, m) = (resample::resample(&d, Tf::W), resample::resample(&d, Tf::M)); Frames { d, w, m } }
    pub fn bars(&self, tf: Tf) -> &Bars { match tf { Tf::D => &self.d, Tf::W => &self.w, Tf::M => &self.m } }
}
/// Indicator outputs on their own timeframe, keyed by instrument fingerprint, id, tf and resolved params. One per request (per instrument) / optimizer thread.
pub type Cache = BTreeMap<String, Rc<Vec<Vec<f64>>>>;
const CACHE_CAP: usize = 64;

fn price(b: &Bars, id: &str) -> Option<Result<Vec<f64>, LabError>> {
    Some(Ok(match id {
        "open" => b.open.clone(),
        "high" => b.high.clone(),
        "low" => b.low.clone(),
        "close" => b.close.clone(),
        "hl2" => b.hl2(),
        "hlc3" => b.hlc3(),
        "ohlc4" => b.ohlc4(),
        "volume" => return Some(b.volume.clone().ok_or_else(|| LabError::NeedsData("volume: volume is missing on some admitted bars of this instrument".into()))),
        _ => return None,
    }))
}

/// One operand as a series aligned to the daily bars (NaN = no value).
pub fn series(fr: &Frames, cache: &mut Cache, s: &SeriesRef) -> Result<Vec<f64>, LabError> {
    let b = fr.bars(s.tf);
    let raw = match price(b, &s.id) {
        Some(r) => r?,
        None => {
            let d = &fr.d; // instrument fingerprint: closes too, since two instruments can share a calendar
            let fp = |v: Option<&f64>| v.map_or(0, |x| x.to_bits());
            let key = format!("{}|{:?}|{:?}|{}|{:?}|{:?}|{}|{}", s.id, s.tf, s.params, d.len(), d.date.first(), d.date.last(), fp(d.close.first()), fp(d.close.last()));
            let outs = match cache.get(&key) {
                Some(o) => o.clone(),
                None => {
                    let def = catalogue::get(&s.id).ok_or_else(|| LabError::Invalid(format!("unknown series '{}'", s.id)))?;
                    let o = Rc::new(catalogue::run(&def, b, &s.params).map_err(|e| match e.as_str() {
                        "needs volume" => LabError::NeedsData(format!("{}: needs volume, which is missing on some admitted bars of this instrument", def.name)),
                        "needs intraday data" => LabError::NeedsData(format!("{}: needs intraday data", def.name)),
                        _ => LabError::Invalid(format!("{}: {e}", def.id)),
                    })?);
                    // shortcut: a full reset bounds memory (64 × outputs × bars); switch to LRU if optimizer profiles show recomputation.
                    if cache.len() >= CACHE_CAP { cache.clear(); }
                    cache.insert(key, o.clone());
                    o
                }
            };
            outs.get(s.out).ok_or_else(|| LabError::Invalid(format!("{}: unknown output {}", s.id, s.out)))?.clone()
        }
    };
    // Offset counts bars of the operand's own timeframe (W offset 1 = the previous completed week).
    let shifted: Vec<f64> = (0..raw.len()).map(|i| if i >= s.offset { raw[i - s.offset] } else { f64::NAN }).collect();
    Ok(if s.tf == Tf::D { shifted } else { resample::expand(&shifted, b, fr.d.len()) })
}

pub enum Val { Arr(Vec<f64>), Const(f64), Held }
impl Val {
    /// Value at the close of daily bar i. `entry` = fill bar of the open position; bars_held is 0 on that bar.
    fn at(&self, i: usize, entry: Option<usize>) -> f64 {
        match self {
            Val::Arr(v) => v[i],
            Val::Const(x) => *x,
            Val::Held => entry.filter(|&e| e <= i).map_or(f64::NAN, |e| (i - e) as f64),
        }
    }
}
pub struct RCond { pub lhs: Val, pub op: Op, pub rhs: Val, pub label: String }
impl RCond {
    /// NaN on either side (or on the previous bar for crosses) makes the condition false.
    pub fn holds(&self, i: usize, entry: Option<usize>) -> bool {
        let (a, b) = (self.lhs.at(i, entry), self.rhs.at(i, entry));
        let prev = || if i == 0 { (f64::NAN, f64::NAN) } else { (self.lhs.at(i - 1, entry), self.rhs.at(i - 1, entry)) };
        match self.op {
            Op::Gt => a > b,
            Op::Lt => a < b,
            Op::Ge => a >= b,
            Op::Le => a <= b,
            Op::CrossesAbove => { let (a1, b1) = prev(); a > b && a1 <= b1 }
            Op::CrossesBelow => { let (a1, b1) = prev(); a < b && a1 >= b1 }
        }
    }
}
pub struct Compiled { pub entry: Vec<RCond>, pub exit: Vec<RCond> }

fn val(o: &Operand, fr: &Frames, cache: &mut Cache) -> Result<Val, LabError> {
    Ok(match o { Operand::Series(s) => Val::Arr(series(fr, cache, s)?), Operand::Value(x) => Val::Const(*x), Operand::BarsHeld => Val::Held })
}
fn list(cs: &[Cond], at: &str, fr: &Frames, cache: &mut Cache) -> Result<Vec<RCond>, LabError> {
    cs.iter().enumerate().map(|(k, c)| Ok(RCond { lhs: val(&c.lhs, fr, cache)?, op: c.op, rhs: val(&c.rhs, fr, cache)?, label: format!("{at}[{k}]: {}", describe(c)) })).collect()
}
pub fn compile(spec: &Spec, fr: &Frames, cache: &mut Cache) -> Result<Compiled, LabError> {
    Ok(Compiled { entry: list(&spec.entry, "entry.all", fr, cache)?, exit: list(&spec.exit, "exit.any", fr, cache)? })
}

/// Human-readable condition, recorded as a trade's exit reason, e.g. `rsi(length=2) crosses_above 70`.
pub fn describe(c: &Cond) -> String {
    let side = |o: &Operand| match o {
        Operand::Value(x) => format!("{x}"),
        Operand::BarsHeld => "bars_held".into(),
        Operand::Series(s) => {
            let p: Vec<String> = s.params.iter().map(|(k, v)| format!("{k}={v}")).collect();
            let mut t = if p.is_empty() { s.id.clone() } else { format!("{}({})", s.id, p.join(",")) };
            if s.out > 0 { if let Some(d) = catalogue::get(&s.id) { t = format!("{t}.{}", d.outputs[s.out]); } }
            if s.tf != Tf::D { t = format!("{t}@{:?}", s.tf); }
            if s.offset > 0 { t = format!("{t}[{}]", s.offset); }
            t
        }
    };
    let op = match c.op { Op::CrossesAbove => "crosses_above", Op::CrossesBelow => "crosses_below", Op::Gt => ">", Op::Lt => "<", Op::Ge => ">=", Op::Le => "<=" };
    format!("{} {op} {}", side(&c.lhs), side(&c.rhs))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lab::catalogue::{all, resolve_params, Params};
    use crate::lab::rules::parse_spec;
    use crate::lab::testdata::{assert_series, close_eq, walk};

    /// Real calendar days (weekends skipped) so W/M grouping matches an exchange calendar.
    pub fn business_days(n: usize, with_volume: bool) -> Bars {
        let mut b = walk(n, 21, with_volume);
        let (mut day, mut k) = (resample::day_number("2021-01-04"), 0); // a Monday
        while k < n {
            if (day + 3).rem_euclid(7) < 5 { b.date[k] = civil(day); k += 1; }
            day += 1;
        }
        b
    }
    fn civil(z: i64) -> String {
        let z = z + 719_468;
        let era = z.div_euclid(146_097);
        let doe = z - era * 146_097;
        let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let d = doy - (153 * mp + 2) / 5 + 1;
        let m = if mp < 10 { mp + 3 } else { mp - 9 };
        format!("{:04}-{m:02}-{d:02}", yoe + era * 400 + i64::from(m <= 2))
    }
    fn sref(id: &str, tf: Tf, out: usize, offset: usize) -> SeriesRef {
        let params = catalogue::get(id).map_or(Params::new(), |d| resolve_params(&d, &Params::new()).unwrap());
        SeriesRef { id: id.into(), params, tf, out, offset }
    }

    #[test] fn civil_round_trips() { for d in ["2021-01-04", "2024-02-29", "2000-03-01"] { assert_eq!(civil(resample::day_number(d)), d); } }
    #[test] fn weekly_operand_is_visible_only_after_the_week_closes() {
        let fr = Frames::new(business_days(30, false));
        let w = series(&fr, &mut Cache::new(), &sref("close", Tf::W, 0, 0)).unwrap();
        // 2021-01-04..08 is week 1 (indices 0..4): Mon–Thu see nothing, Friday sees its own weekly close.
        assert!(w[..4].iter().all(|v| v.is_nan()));
        assert_eq!(w[4], fr.d.close[4]);
        // Week 2 Mon–Thu still show week 1; week 2's close appears on its Friday (index 9).
        assert!(w[5..9].iter().all(|v| *v == fr.d.close[4]));
        assert_eq!(w[9], fr.d.close[9]);
    }
    #[test] fn offsets_count_bars_of_their_own_timeframe() {
        let fr = Frames::new(business_days(30, false));
        let w1 = series(&fr, &mut Cache::new(), &sref("close", Tf::W, 0, 1)).unwrap();
        assert!(w1[..9].iter().all(|v| v.is_nan()));
        assert_eq!(w1[9], fr.d.close[4]); // previous week's close, from week 2's Friday
        let d2 = series(&fr, &mut Cache::new(), &sref("close", Tf::D, 0, 2)).unwrap();
        assert_series(&d2[..4], &[f64::NAN, f64::NAN, fr.d.close[0], fr.d.close[1]]);
    }
    #[test] fn signals_never_use_later_bars() {
        // Every catalogue indicator on D, W and M: truncating the data never changes an earlier daily value.
        let full_bars = business_days(420, true);
        let full = Frames::new(full_bars.clone());
        for d in all().iter().filter(|d| d.compute.is_some()) {
            for tf in [Tf::D, Tf::W, Tf::M] {
                let r = sref(d.id, tf, d.outputs.len() - 1, 0);
                let want = series(&full, &mut Cache::new(), &r).unwrap();
                for cut in [97usize, 203, 311] {
                    let mut pre = full_bars.clone();
                    for v in [&mut pre.open, &mut pre.high, &mut pre.low, &mut pre.close] { v.truncate(cut); }
                    pre.date.truncate(cut); pre.end.truncate(cut);
                    if let Some(v) = pre.volume.as_mut() { v.truncate(cut); }
                    let got = series(&Frames::new(pre), &mut Cache::new(), &r).unwrap();
                    // The final bar of the cut may sit inside an unfinished week/month, so only bars before it must match.
                    for i in 0..cut - 1 { assert!(close_eq(got[i], want[i]), "{} {tf:?} bar {i} changes when bars after {cut} exist", d.id); }
                }
            }
        }
    }
    #[test] fn crosses_and_nan_semantics() {
        let c = |op, a: Vec<f64>| RCond { lhs: Val::Arr(a), op, rhs: Val::Const(10.), label: String::new() };
        let up = c(Op::CrossesAbove, vec![9., 10., 11., 12., f64::NAN, 11.]);
        assert_eq!((0..6).map(|i| up.holds(i, None)).collect::<Vec<_>>(), [false, false, true, false, false, false]);
        let down = c(Op::CrossesBelow, vec![11., 10., 9.]);
        assert_eq!((0..3).map(|i| down.holds(i, None)).collect::<Vec<_>>(), [false, false, true]);
        let ge = c(Op::Ge, vec![10., f64::NAN]);
        assert_eq!((ge.holds(0, None), ge.holds(1, None)), (true, false));
        let held = RCond { lhs: Val::Held, op: Op::Ge, rhs: Val::Const(2.), label: String::new() };
        assert_eq!((held.holds(5, None), held.holds(5, Some(4)), held.holds(6, Some(4))), (false, false, true));
    }
    #[test] fn needs_volume_is_typed() {
        let fr = Frames::new(business_days(60, false));
        assert_eq!(series(&fr, &mut Cache::new(), &sref("obv", Tf::D, 0, 0)).unwrap_err().status(), "needs_data");
        assert_eq!(series(&fr, &mut Cache::new(), &sref("volume", Tf::W, 0, 0)).unwrap_err().status(), "needs_data");
    }
    #[test] fn cache_reuses_runs_and_stays_bounded() {
        let fr = Frames::new(business_days(60, false));
        let mut cache = Cache::new();
        series(&fr, &mut cache, &sref("macd", Tf::D, 0, 0)).unwrap();
        series(&fr, &mut cache, &sref("macd", Tf::D, 1, 0)).unwrap();
        assert_eq!(cache.len(), 1);
        for n in 1..=100 { let mut r = sref("sma", Tf::D, 0, 0); r.params.insert("length".into(), n as f64); series(&fr, &mut cache, &r).unwrap(); }
        assert!(cache.len() <= CACHE_CAP);
    }
    #[test] fn every_indicator_at_parameter_bounds_never_panics() {
        for bars in [business_days(3, true), business_days(300, true)] {
            let fr = Frames::new(bars);
            for d in all().iter().filter(|d| d.compute.is_some()) {
                let picks: [fn(&catalogue::ParamSpec) -> f64; 2] = [|p| p.min, |p| p.max];
                for pick in picks {
                    let params: Params = d.params.iter().map(|p| (p.key.to_string(), pick(p))).collect();
                    for tf in [Tf::D, Tf::W, Tf::M] {
                        let r = SeriesRef { id: d.id.into(), params: params.clone(), tf, out: 0, offset: 0 };
                        assert_eq!(series(&fr, &mut Cache::new(), &r).unwrap().len(), fr.d.len(), "{} {tf:?}", d.id);
                    }
                }
            }
        }
    }
    #[test] fn one_cache_across_instruments_never_leaks_values() {
        let (a, b) = (Frames::new(walk(300, 1, true)), Frames::new(walk(300, 2, true)));
        let r = sref("sma", Tf::D, 0, 0);
        let mut shared = Cache::new();
        let (ya, yb) = (series(&a, &mut shared, &r).unwrap(), series(&b, &mut shared, &r).unwrap());
        assert_series(&ya, &series(&a, &mut Cache::new(), &r).unwrap());
        assert_series(&yb, &series(&b, &mut Cache::new(), &r).unwrap());
        assert!(ya.iter().zip(&yb).any(|(x, y)| !close_eq(*x, *y)));
    }
    #[test] fn out_of_range_output_is_invalid_not_a_panic() {
        let fr = Frames::new(business_days(60, false));
        let e = series(&fr, &mut Cache::new(), &sref("sma", Tf::D, 5, 0)).unwrap_err();
        assert_eq!(e.status(), "invalid");
    }
    #[test] fn compile_labels_conditions() {
        let spec = parse_spec(r#"{"v":1,"entry":{"all":[{"lhs":{"s":"macd","out":"signal","tf":"W","offset":1},"op":"<","rhs":{"value":0}}]},"exit":{"any":[{"lhs":{"s":"bars_held"},"op":">=","rhs":{"value":3}}]}}"#).unwrap();
        let c = compile(&spec, &Frames::new(business_days(60, false)), &mut Cache::new()).unwrap();
        assert_eq!(c.entry[0].label, "entry.all[0]: macd(fast=12,signal=9,slow=26).signal@W[1] < 0");
        assert_eq!(c.exit[0].label, "exit.any[0]: bars_held >= 3");
    }
}
