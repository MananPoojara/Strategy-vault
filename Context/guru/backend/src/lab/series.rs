//! Operands → daily-aligned series and compiled conditions. Indicators run on their own timeframe's bars;
//! W/M values reach daily bars through `resample::expand` (usable after the period's last daily close).
use super::bars::Bars;
use super::catalogue;
use super::resample::{self, Tf};
use super::rules::{Bin, Cond, Dist, Exec, Expr, LabError, Node, Op, Operand, SeriesRef, Spec, Win};
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
                        "needs breadth" => LabError::NeedsData(format!("{}: needs market breadth (advances/declines) on daily bars; this market or timeframe has none", def.name)),
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
            Op::Eq => a == b,
            Op::Ne => !a.is_nan() && !b.is_nan() && a != b,
            Op::Crosses => { let (a1, b1) = prev(); (a > b && a1 <= b1) || (a < b && a1 >= b1) }
        }
    }
}
pub struct Compiled { pub entry: Vec<RCond>, pub exit: Vec<RCond>, pub exec: Exec, pub atr: BTreeMap<usize, Vec<f64>> }
impl Compiled {
    /// Long, market orders at the next open, no price exits (the v1 defaults).
    pub fn new(entry: Vec<RCond>, exit: Vec<RCond>) -> Compiled { Compiled { entry, exit, exec: Exec::default(), atr: BTreeMap::new() } }
    /// Price distance of `d` from `px`, using ATR at bar `k` (a bar whose close is already known). NaN while ATR warms up.
    pub fn dist(&self, d: Dist, px: f64, k: usize) -> f64 {
        match d { Dist::Pct(p) => px * p / 100., Dist::Atr { mult, length } => self.atr.get(&length).map_or(f64::NAN, |a| mult * a[k]) }
    }
}

fn val(o: &Operand, fr: &Frames, cache: &mut Cache) -> Result<Val, LabError> {
    Ok(match o { Operand::Series(s) => Val::Arr(series(fr, cache, s)?), Operand::Value(x) => Val::Const(*x), Operand::BarsHeld => Val::Held, Operand::Expr(e) => Val::Arr(eval(e, fr, cache)?) })
}
/// A v1 comparison compiles exactly as before; every other node becomes a precomputed 1/0/NaN array tested with `> 0.5`.
fn list(ns: &[Node], at: &str, fr: &Frames, cache: &mut Cache) -> Result<Vec<RCond>, LabError> {
    ns.iter().enumerate().map(|(k, n)| Ok(match n {
        Node::Cmp(c) => RCond { lhs: val(&c.lhs, fr, cache)?, op: c.op, rhs: val(&c.rhs, fr, cache)?, label: format!("{at}[{k}]: {}", describe(c)) },
        _ => RCond { lhs: Val::Arr(mask(n, fr, cache)?), op: Op::Gt, rhs: Val::Const(0.5), label: format!("{at}[{k}]: {}", describe_node(n)) },
    })).collect()
}
/// An operand as a daily array (constants repeated). bars_held never reaches here: the parser keeps it top-level.
fn arr(o: &Operand, fr: &Frames, cache: &mut Cache) -> Result<Vec<f64>, LabError> {
    match val(o, fr, cache)? {
        Val::Arr(v) => Ok(v),
        Val::Const(x) => Ok(vec![x; fr.d.len()]),
        Val::Held => Err(LabError::Invalid("bars_held is only available in top-level exit conditions".into())),
    }
}
/// An expression on the daily bars (spec §4.1–§4.2): a NaN input gives NaN, division by zero gives NaN, windows are NaN until full.
fn eval(e: &Expr, fr: &Frames, cache: &mut Cache) -> Result<Vec<f64>, LabError> {
    let len = fr.d.len();
    let nan = f64::NAN;
    Ok(match e {
        Expr::Bin(op, l, r) => {
            let (a, b) = (arr(l, fr, cache)?, arr(r, fr, cache)?);
            a.iter().zip(&b).map(|(&x, &y)| if x.is_nan() || y.is_nan() { nan } else {
                match op {
                    Bin::Add => x + y,
                    Bin::Sub => x - y,
                    Bin::Mul => x * y,
                    Bin::Div => if y == 0. { nan } else { x / y },
                    Bin::Min => x.min(y),
                    Bin::Max => x.max(y),
                    Bin::Pct => if y == 0. { nan } else { 100. * (x - y) / y },
                }
            }).collect()
        }
        Expr::Abs(x) => arr(x, fr, cache)?.into_iter().map(f64::abs).collect(),
        Expr::Ref(x, n) => { let a = arr(x, fr, cache)?; (0..len).map(|i| if i >= *n { a[i - n] } else { nan }).collect() }
        Expr::Win(Win::Change, x, n) => { let a = arr(x, fr, cache)?; (0..len).map(|i| if i >= *n { a[i] - a[i - n] } else { nan }).collect() }
        Expr::Win(w, x, n) => {
            let a = arr(x, fr, cache)?;
            // shortcut: O(bars × window); a running sum / monotone deque is the upgrade if optimizer profiles show it.
            (0..len).map(|i| {
                if i + 1 < *n { return nan; }
                let s = &a[i + 1 - n..=i];
                if s.iter().any(|v| v.is_nan()) { return nan; }
                match w {
                    Win::Sum => s.iter().sum(),
                    Win::Avg => s.iter().sum::<f64>() / *n as f64,
                    Win::Highest => s.iter().copied().fold(f64::MIN, f64::max),
                    _ => s.iter().copied().fold(f64::MAX, f64::min),
                }
            }).collect()
        }
    })
}
fn truth(t: bool) -> f64 { if t { 1. } else { 0. } }
/// One comparison as 1 (true), 0 (false) or NaN (unknown: an input, or the previous bar for crosses, is missing).
fn cmp_at(op: Op, a: &[f64], b: &[f64], i: usize) -> f64 {
    let (x, y) = (a[i], b[i]);
    let (x1, y1) = if i == 0 { (f64::NAN, f64::NAN) } else { (a[i - 1], b[i - 1]) };
    let crossing = matches!(op, Op::CrossesAbove | Op::CrossesBelow | Op::Crosses);
    if x.is_nan() || y.is_nan() || (crossing && (x1.is_nan() || y1.is_nan())) { return f64::NAN; }
    truth(match op {
        Op::Gt => x > y,
        Op::Lt => x < y,
        Op::Ge => x >= y,
        Op::Le => x <= y,
        Op::Eq => x == y,
        Op::Ne => x != y,
        Op::CrossesAbove => x > y && x1 <= y1,
        Op::CrossesBelow => x < y && x1 >= y1,
        Op::Crosses => (x > y && x1 <= y1) || (x < y && x1 >= y1),
    })
}
/// Three-valued truth of a node on every daily bar (spec §4.2): 1 true, 0 false, NaN unknown.
pub fn mask(n: &Node, fr: &Frames, cache: &mut Cache) -> Result<Vec<f64>, LabError> {
    let len = fr.d.len();
    Ok(match n {
        Node::Cmp(c) => { let (a, b) = (arr(&c.lhs, fr, cache)?, arr(&c.rhs, fr, cache)?); (0..len).map(|i| cmp_at(c.op, &a, &b, i)).collect() }
        Node::Range { x, lo, hi, outside } => {
            let (x, lo, hi) = (arr(x, fr, cache)?, arr(lo, fr, cache)?, arr(hi, fr, cache)?);
            (0..len).map(|i| if x[i].is_nan() || lo[i].is_nan() || hi[i].is_nan() { f64::NAN } else { truth((lo[i] <= x[i] && x[i] <= hi[i]) != *outside) }).collect()
        }
        Node::Trend { x, bars, rising } => {
            let x = arr(x, fr, cache)?;
            (0..len).map(|i| {
                if i < *bars || x[i - bars..=i].iter().any(|v| v.is_nan()) { return f64::NAN; }
                truth(x[i - bars..=i].windows(2).all(|w| if *rising { w[1] > w[0] } else { w[1] < w[0] }))
            }).collect()
        }
        Node::All(v) | Node::Any(v) => {
            let all = matches!(n, Node::All(_));
            let kids = v.iter().map(|c| mask(c, fr, cache)).collect::<Result<Vec<_>, _>>()?;
            (0..len).map(|i| {
                let (decisive, other) = if all { (0., 1.) } else { (1., 0.) };
                if kids.iter().any(|k| k[i] == decisive) { decisive } else if kids.iter().any(|k| k[i].is_nan()) { f64::NAN } else { other }
            }).collect()
        }
        Node::Not(c) => mask(c, fr, cache)?.into_iter().map(|v| if v.is_nan() { v } else { 1. - v }).collect(),
        Node::AtLeast { k, of, when } => {
            // Kleene count over the last `of` bars: true if trues >= k, false if even every unknown could not reach k, else unknown.
            let m = mask(when, fr, cache)?;
            let (mut t, mut u) = (0usize, 0usize);
            (0..len).map(|i| {
                t += usize::from(m[i] == 1.); u += usize::from(m[i].is_nan());
                if i >= *of { t -= usize::from(m[i - of] == 1.); u -= usize::from(m[i - of].is_nan()); }
                if i + 1 < *of { f64::NAN } else if t >= *k { 1. } else if t + u < *k { 0. } else { f64::NAN }
            }).collect()
        }
        Node::Then { steps, within } => {
            let ms = steps.iter().map(|c| mask(c, fr, cache)).collect::<Result<Vec<_>, _>>()?;
            // start[i]: the latest first-step bar of a chain whose current step is true on bar i (-1 = none).
            let mut start: Vec<i64> = ms[0].iter().enumerate().map(|(i, v)| if *v == 1. { i as i64 } else { -1 }).collect();
            for m in &ms[1..] {
                let mut best = -1i64; // latest start among chains ending strictly before bar i
                start = (0..len).map(|i| {
                    let here = if m[i] == 1. { best } else { -1 };
                    best = best.max(start[i]);
                    here
                }).collect();
            }
            let last = &ms[ms.len() - 1];
            // unknown steps in [i-within, i] make a missing chain unknown, not false (Kleene).
            let mut unk = vec![0usize; len + 1];
            for j in 0..len { unk[j + 1] = unk[j] + usize::from(ms.iter().any(|m| m[j].is_nan())); }
            (0..len).map(|i| {
                if last[i].is_nan() { return f64::NAN; }
                if start[i] >= 0 && i as i64 - start[i] <= *within as i64 { return 1.; }
                if last[i] == 1. && unk[i + 1] - unk[i.saturating_sub(*within)] > 0 { f64::NAN } else { 0. }
            }).collect()
        }
    })
}
pub fn compile(spec: &Spec, fr: &Frames, cache: &mut Cache) -> Result<Compiled, LabError> {
    let mut c = Compiled { entry: list(&spec.entry, "entry.all", fr, cache)?, exit: list(&spec.exit, "exit.any", fr, cache)?, exec: spec.exec.clone(), atr: BTreeMap::new() };
    let x = &spec.exec;
    for d in [x.limit, x.stop_loss, x.target, x.trailing].into_iter().flatten() {
        if let Dist::Atr { length, .. } = d {
            if c.atr.contains_key(&length) { continue; }
            let r = SeriesRef { id: "atr".into(), params: [("length".to_string(), length as f64)].into(), tf: Tf::D, out: 0, offset: 0 };
            c.atr.insert(length, series(fr, cache, &r)?);
        }
    }
    Ok(c)
}

/// Human-readable condition, recorded as a trade's exit reason, e.g. `rsi(length=2) crosses_above 70`.
pub fn describe(c: &Cond) -> String {
    format!("{} {} {}", operand_text(&c.lhs), op_text(c.op), operand_text(&c.rhs))
}
pub fn operand_text(o: &Operand) -> String {
    match o {
        Operand::Value(x) => format!("{x}"),
        Operand::BarsHeld => "bars_held".into(),
        Operand::Expr(e) => match &**e {
            Expr::Bin(op, l, r) => format!("{}({}, {})", op.key(), operand_text(l), operand_text(r)),
            Expr::Abs(x) => format!("abs({})", operand_text(x)),
            Expr::Ref(x, n) => format!("ref({}, {n})", operand_text(x)),
            Expr::Win(w, x, n) => format!("{}({}, {n})", w.key(), operand_text(x)),
        },
        Operand::Series(s) => {
            let p: Vec<String> = s.params.iter().map(|(k, v)| format!("{k}={v}")).collect();
            let mut t = if p.is_empty() { s.id.clone() } else { format!("{}({})", s.id, p.join(",")) };
            if s.out > 0 { if let Some(d) = catalogue::get(&s.id) { t = format!("{t}.{}", d.outputs[s.out]); } }
            if s.tf != Tf::D { t = format!("{t}@{:?}", s.tf); }
            if s.offset > 0 { t = format!("{t}[{}]", s.offset); }
            t
        }
    }
}
fn op_text(op: Op) -> &'static str {
    match op { Op::CrossesAbove => "crosses_above", Op::CrossesBelow => "crosses_below", Op::Gt => ">", Op::Lt => "<", Op::Ge => ">=", Op::Le => "<=", Op::Eq => "=", Op::Ne => "!=", Op::Crosses => "crosses" }
}
/// Human-readable node, recorded as an exit reason, e.g. `all(rsi(length=2) < 10, close > 5)`.
pub fn describe_node(n: &Node) -> String {
    let join = |v: &[Node]| v.iter().map(describe_node).collect::<Vec<_>>().join(", ");
    match n {
        Node::Cmp(c) => describe(c),
        Node::Range { x, lo, hi, outside } => format!("{} {} {} and {}", operand_text(x), if *outside { "outside" } else { "between" }, operand_text(lo), operand_text(hi)),
        Node::Trend { x, bars, rising } => format!("{} {} {bars} bars", operand_text(x), if *rising { "rising" } else { "falling" }),
        Node::All(v) => format!("all({})", join(v)),
        Node::Any(v) => format!("any({})", join(v)),
        Node::Not(c) => format!("not({})", describe_node(c)),
        Node::AtLeast { k, of, when } => format!("at_least {k} of {of}({})", describe_node(when)),
        Node::Then { steps, within } => format!("then({}) within {within}", join(steps)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lab::catalogue::{all, resolve_params, Params};
    use crate::lab::rules::parse_spec;
    use crate::lab::testdata::{assert_series, close_eq, walk, with_breadth};

    /// Real calendar days (weekends skipped) so W/M grouping matches an exchange calendar.
    pub fn business_days(n: usize, with_volume: bool) -> Bars {
        let mut b = walk(n, 21, with_volume);
        let (mut day, mut k) = (resample::day_number("2021-01-04"), 0); // a Monday
        while k < n {
            if (day + 3).rem_euclid(7) < 5 { b.date[k] = resample::civil(day); k += 1; }
            day += 1;
        }
        b
    }
    fn sref(id: &str, tf: Tf, out: usize, offset: usize) -> SeriesRef {
        let params = catalogue::get(id).map_or(Params::new(), |d| resolve_params(&d, &Params::new()).unwrap());
        SeriesRef { id: id.into(), params, tf, out, offset }
    }

    #[test] fn every_computed_indicator_is_reachable_from_a_rule() {
        for d in all().iter().filter(|d| d.compute.is_some()) {
            let out = d.outputs[d.outputs.len() - 1];
            if let Err(e) = crate::lab::testdata::compiles(d.id, out) { panic!("{}.{out}: {} {}", d.id, e.status(), e.message()); }
        }
    }
    #[test] fn listed_entries_refuse_with_their_note() {
        for (id, status) in [("vwap", "needs_data"), ("cvd", "needs_data"), ("correlation", "needs_data"), ("dividend_yield", "needs_data"),
            ("open_interest", "needs_data"), ("onchain_hash_rate", "needs_data"), ("zigzag", "unsupported"), ("peak", "unsupported"),
            ("rob_booker_reversal", "unsupported"), ("fir", "unsupported"), ("seasonality", "unsupported")] {
            let d = catalogue::get(id).unwrap();
            let e = crate::lab::testdata::compiles(id, d.outputs[0]).unwrap_err();
            assert_eq!(e.status(), status, "{id}");
            assert!(e.message().contains(d.note), "{id}: {}", e.message());
        }
        for d in all().iter().filter(|d| d.compute.is_none()) { assert!(crate::lab::testdata::compiles(d.id, d.outputs[0]).is_err(), "{}", d.id); }
    }
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
        let full_bars = with_breadth(business_days(420, true));
        let full = Frames::new(full_bars.clone());
        for d in all().iter().filter(|d| d.compute.is_some()) {
            for tf in [Tf::D, Tf::W, Tf::M] {
                let r = sref(d.id, tf, d.outputs.len() - 1, 0);
                if d.needs.breadth && tf != Tf::D { assert_eq!(series(&full, &mut Cache::new(), &r).unwrap_err().status(), "needs_data"); continue; }
                let want = series(&full, &mut Cache::new(), &r).unwrap();
                for cut in (60..400usize).step_by(7) {
                    let pre = crate::lab::testdata::cut(&full_bars, cut);
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
        for bars in [with_breadth(business_days(3, true)), with_breadth(business_days(300, true))] {
            let fr = Frames::new(bars);
            for d in all().iter().filter(|d| d.compute.is_some()) {
                let picks: [fn(&catalogue::ParamSpec) -> f64; 2] = [|p| p.min, |p| p.max];
                for pick in picks {
                    let params: Params = d.params.iter().map(|p| (p.key.to_string(), pick(p))).collect();
                    for tf in [Tf::D, Tf::W, Tf::M] {
                        let r = SeriesRef { id: d.id.into(), params: params.clone(), tf, out: 0, offset: 0 };
                        if d.needs.breadth && tf != Tf::D { assert_eq!(series(&fr, &mut Cache::new(), &r).unwrap_err().status(), "needs_data"); continue; }
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

    // ---- Phase 7: nodes (spec §4.2)
    fn node(t: &str) -> Node {
        parse_spec(&format!(r#"{{"v":1,"entry":{{"all":[{t}]}},"exit":{{"any":[{{"lhs":{{"s":"bars_held"}},"op":">=","rhs":{{"value":1}}}}]}}}}"#)).unwrap().entry.remove(0)
    }
    #[test] fn nodes_use_three_valued_logic() {
        let (fr, x) = (Frames::new(crate::lab::testdata::closes(&[1., 2., 3., 2., 4., 5., 6.])), f64::NAN);
        let m = |t: &str| mask(&node(t), &fr, &mut Cache::new()).unwrap();
        let up = r#"{"lhs":{"s":"close"},"op":">","rhs":{"s":"close","offset":1}}"#;
        let big = r#"{"lhs":{"s":"close"},"op":">","rhs":{"value":3}}"#;
        let small = r#"{"lhs":{"s":"close"},"op":"<","rhs":{"value":2}}"#;
        assert_series(&m(up), &[x, 1., 1., 0., 1., 1., 1.]);
        assert_series(&m(&format!(r#"{{"not":{up}}}"#)), &[x, 0., 0., 1., 0., 0., 0.]);
        assert_series(&m(&format!(r#"{{"all":[{up},{big}]}}"#)), &[0., 0., 0., 0., 1., 1., 1.]); // a false child decides
        assert_series(&m(&format!(r#"{{"any":[{up},{small}]}}"#)), &[1., 1., 1., 0., 1., 1., 1.]); // a true child decides
        assert_series(&m(&format!(r#"{{"any":[{up},{big}]}}"#)), &[x, 1., 1., 0., 1., 1., 1.]); // otherwise unknown stays unknown
        assert_series(&m(r#"{"lhs":{"s":"close"},"op":"between","lo":{"value":2},"hi":{"value":4}}"#), &[0., 1., 1., 1., 1., 0., 0.]);
        assert_series(&m(r#"{"lhs":{"s":"close"},"op":"outside","lo":{"value":2},"hi":{"value":4}}"#), &[1., 0., 0., 0., 0., 1., 1.]);
        assert_series(&m(r#"{"lhs":{"s":"close"},"op":"rising","bars":2}"#), &[x, x, 1., 0., 0., 1., 1.]);
        assert_series(&m(r#"{"lhs":{"s":"close"},"op":"falling","bars":1}"#), &[x, 0., 0., 1., 0., 0., 0.]);
        assert_series(&m(&format!(r#"{{"at_least":2,"of":3,"when":{up}}}"#)), &[x, x, 1., 1., 1., 1., 1.]);
        assert_series(&m(&format!(r#"{{"at_least":3,"of":3,"when":{up}}}"#)), &[x, x, x, 0., 0., 0., 1.]); // bar 2 holds one unknown that could still make three
        assert_series(&m(r#"{"lhs":{"s":"close"},"op":"=","rhs":{"value":2}}"#), &[0., 1., 0., 1., 0., 0., 0.]);
        assert_series(&m(r#"{"lhs":{"s":"close"},"op":"!=","rhs":{"value":2}}"#), &[1., 0., 1., 0., 1., 1., 1.]);
        assert_series(&m(r#"{"lhs":{"s":"close"},"op":"crosses","rhs":{"value":2.5}}"#), &[x, 0., 1., 1., 1., 0., 0.]);
        // the same comparison at the top of a rule list (v1 path) treats unknown as false
        let top = RCond { lhs: Val::Arr(m(up)), op: Op::Gt, rhs: Val::Const(0.5), label: String::new() };
        assert!(!top.holds(0, None) && top.holds(1, None));
    }
    #[test] fn sequences_need_steps_in_order_within_the_span() {
        let fr = Frames::new(crate::lab::testdata::closes(&[1., 3., 3., 3., 5., 1., 5., 3., 5.]));
        let (a, b) = (r#"{"lhs":{"s":"close"},"op":"<","rhs":{"value":2}}"#, r#"{"lhs":{"s":"close"},"op":">","rhs":{"value":4}}"#);
        let m = |steps: &str, within: usize| mask(&node(&format!(r#"{{"then":[{steps}],"within":{within}}}"#)), &fr, &mut Cache::new()).unwrap();
        // A on bars 0 and 5, B on 4, 6 and 8: B counts when an A came strictly earlier and at most `within` bars before
        assert_series(&m(&format!("{a},{b}"), 4), &[0., 0., 0., 0., 1., 0., 1., 0., 1.]);
        assert_series(&m(&format!("{a},{b}"), 3), &[0., 0., 0., 0., 0., 0., 1., 0., 1.]);
        assert_series(&m(&format!("{a},{b},{a}"), 6), &[0., 0., 0., 0., 0., 1., 0., 0., 0.]);
        // one bar is never two steps
        let same = Frames::new(crate::lab::testdata::closes(&[5., 5.]));
        assert_series(&mask(&node(&format!(r#"{{"then":[{b},{b}],"within":5}}"#)), &same, &mut Cache::new()).unwrap(), &[0., 1.]);
    }
    #[test] fn counting_nodes_stay_unknown_through_warm_up() {
        let fr = Frames::new(business_days(300, false));
        let c = r#"{"lhs":{"s":"close"},"op":">","rhs":{"s":"sma","p":{"length":200}}}"#;
        let m = |t: &str| mask(&node(t), &fr, &mut Cache::new()).unwrap();
        let base = m(c);
        let (al, th) = (m(&format!(r#"{{"at_least":1,"of":1,"when":{c}}}"#)), m(&format!(r#"{{"then":[{c},{{"lhs":{{"s":"close"}},"op":">","rhs":{{"value":0}}}}],"within":3}}"#)));
        let (nal, nth) = (m(&format!(r#"{{"not":{{"at_least":1,"of":1,"when":{c}}}}}"#)), m(&format!(r#"{{"not":{{"then":[{c},{{"lhs":{{"s":"close"}},"op":">","rhs":{{"value":0}}}}],"within":3}}}}"#)));
        for i in 0..199 { assert!(base[i].is_nan() && al[i].is_nan() && th[i].is_nan() && nal[i].is_nan() && nth[i].is_nan(), "bar {i} must be unknown in warm-up"); }
        // after warm-up (and the sequence span) the answers are the plain true/false ones
        assert!(base[210..].iter().any(|v| *v == 1.) && base[210..].iter().any(|v| *v == 0.), "vacuous");
        for i in 210..300 { assert_eq!(al[i], base[i], "at_least bar {i}"); assert_eq!(th[i], truth((1..=3).any(|d| base[i - d] == 1.)), "then bar {i}"); assert_eq!(nal[i], 1. - base[i]); assert_eq!(nth[i], 1. - th[i]); }
        // a known true still decides while other bars of the window are unknown
        let fr2 = Frames::new(crate::lab::testdata::closes(&[1., 2., 3., 2., 4.]));
        let up = r#"{"lhs":{"s":"close"},"op":">","rhs":{"s":"close","offset":1}}"#;
        assert_series(&mask(&node(&format!(r#"{{"at_least":1,"of":2,"when":{up}}}"#)), &fr2, &mut Cache::new()).unwrap(), &[f64::NAN, 1., 1., 1., 1.]);
    }
    #[test] fn unknown_bounds_and_windows_stay_unknown() {
        let fr = Frames::new(business_days(300, false));
        let m = |t: &str| mask(&node(t), &fr, &mut Cache::new()).unwrap();
        let r = m(r#"{"lhs":{"s":"close"},"op":"between","lo":{"s":"sma","p":{"length":200}},"hi":{"value":1e9}}"#);
        let o = m(r#"{"lhs":{"s":"close"},"op":"outside","lo":{"value":0},"hi":{"s":"sma","p":{"length":200}}}"#);
        assert!(r[..199].iter().all(|v| v.is_nan()) && o[..199].iter().all(|v| v.is_nan()) && r[199..].iter().all(|v| !v.is_nan()));
        let t = m(r#"{"lhs":{"s":"sma","p":{"length":200}},"op":"rising","bars":3}"#);
        assert!(t[..202].iter().all(|v| v.is_nan()) && !t[202].is_nan());
    }
    #[test] fn nodes_never_use_later_bars() {
        let full = business_days(420, true);
        let spec = parse_spec(r#"{"v":1,"entry":{"all":[
            {"any":[{"not":{"lhs":{"s":"close"},"op":"crosses","rhs":{"s":"sma","p":{"length":10},"tf":"W"}}},{"lhs":{"s":"rsi"},"op":"=","rhs":{"value":50}}]},
            {"lhs":{"s":"close"},"op":"between","lo":{"s":"bb","out":"lower"},"hi":{"s":"bb","out":"upper"}},
            {"lhs":{"s":"close"},"op":"rising","bars":3},
            {"at_least":3,"of":7,"when":{"lhs":{"s":"close"},"op":">","rhs":{"s":"open"}}},
            {"then":[{"lhs":{"s":"rsi"},"op":"<","rhs":{"value":40}},{"lhs":{"s":"rsi"},"op":">","rhs":{"value":60}}],"within":12}]},
            "exit":{"any":[{"lhs":{"s":"bars_held"},"op":">=","rhs":{"value":5}}]}}"#).unwrap();
        for n in &spec.entry {
            let want = mask(n, &Frames::new(full.clone()), &mut Cache::new()).unwrap();
            assert!(want.iter().any(|v| *v == 1.) && want.iter().any(|v| *v == 0.), "{}: vacuous", describe_node(n));
            for cut in (60..400usize).step_by(7) {
                let got = mask(n, &Frames::new(crate::lab::testdata::cut(&full, cut)), &mut Cache::new()).unwrap();
                // W/M frames publish the still-open period's value on the cut bar itself, so only those items skip the last bar.
                let upto = if describe_node(n).contains('@') { cut - 1 } else { cut };
                for i in 0..upto { assert!(close_eq(got[i], want[i]), "{} bar {i} changes when bars after {cut} exist", describe_node(n)); }
            }
        }
    }
    #[test] fn groups_compile_with_readable_labels_and_v1_comparisons_keep_theirs() {
        let spec = parse_spec(r#"{"v":1,"entry":{"all":[{"lhs":{"s":"close"},"op":">","rhs":{"value":1}},{"any":[{"lhs":{"s":"close"},"op":"<","rhs":{"value":5}},{"not":{"lhs":{"s":"close"},"op":"crosses","rhs":{"s":"open"}}}]}]},
            "exit":{"any":[{"at_least":2,"of":3,"when":{"lhs":{"s":"close"},"op":"falling","bars":1}},{"lhs":{"s":"bars_held"},"op":">=","rhs":{"value":3}}]}}"#).unwrap();
        let c = compile(&spec, &Frames::new(business_days(60, false)), &mut Cache::new()).unwrap();
        assert_eq!(c.entry[0].label, "entry.all[0]: close > 1");
        assert!(matches!(c.entry[0].lhs, Val::Arr(_)) && c.entry[0].op == Op::Gt);
        assert_eq!(c.entry[1].label, "entry.all[1]: any(close < 5, not(close crosses open))");
        assert_eq!(c.exit[0].label, "exit.any[0]: at_least 2 of 3(close falling 1 bars)");
        assert!(matches!(c.exit[1].lhs, Val::Held));
    }

    // ---- Phase 7: expressions (spec §4.1)
    #[test] fn expressions_compute_on_daily_bars() {
        let fr = Frames::new(crate::lab::testdata::closes(&[1., 2., 3., 2., 4.]));
        let x = f64::NAN;
        let e = |t: &str| { let Node::Cmp(c) = node(&format!(r#"{{"lhs":{t},"op":">","rhs":{{"value":0}}}}"#)) else { panic!() }; arr(&c.lhs, &fr, &mut Cache::new()).unwrap() };
        assert_series(&e(r#"{"add":[{"s":"close"},{"value":1}]}"#), &[2., 3., 4., 3., 5.]);
        assert_series(&e(r#"{"mul":[{"s":"close"},{"s":"close"}]}"#), &[1., 4., 9., 4., 16.]);
        assert_series(&e(r#"{"div":[{"s":"close"},{"sub":[{"s":"close"},{"value":2}]}]}"#), &[-1., x, 3., x, 2.]); // division by zero is no value
        assert_series(&e(r#"{"pct":[{"s":"close"},{"ref":{"s":"close"},"bars":1}]}"#), &[x, 100., 50., -100. / 3., 100.]);
        assert_series(&e(r#"{"abs":{"sub":[{"s":"close"},{"value":3}]}}"#), &[2., 1., 0., 1., 1.]);
        assert_series(&e(r#"{"min":[{"s":"close"},{"value":2.5}]}"#), &[1., 2., 2.5, 2., 2.5]);
        assert_series(&e(r#"{"max":[{"s":"close"},{"value":2.5}]}"#), &[2.5, 2.5, 3., 2.5, 4.]);
        assert_series(&e(r#"{"sum":{"s":"close"},"bars":3}"#), &[x, x, 6., 7., 9.]);
        assert_series(&e(r#"{"avg":{"s":"close"},"bars":3}"#), &[x, x, 2., 7. / 3., 3.]);
        assert_series(&e(r#"{"highest":{"s":"close"},"bars":2}"#), &[x, 2., 3., 3., 4.]);
        assert_series(&e(r#"{"lowest":{"s":"close"},"bars":2}"#), &[x, 1., 2., 2., 2.]);
        assert_series(&e(r#"{"change":{"s":"close"},"bars":2}"#), &[x, x, 2., 0., 1.]);
        assert_series(&e(r#"{"sum":{"ref":{"s":"close"},"bars":1},"bars":2}"#), &[x, x, 3., 5., 5.]); // a NaN inside the window gives NaN
    }
    #[test] fn expressions_never_use_later_bars() {
        let full = business_days(420, true);
        for t in [r#"{"pct":[{"s":"close"},{"s":"sma","p":{"length":4},"tf":"W"}]}"#, r#"{"sum":{"s":"rsi","p":{"length":2}},"bars":3}"#,
                  r#"{"highest":{"sub":[{"s":"high"},{"s":"low"}]},"bars":10}"#, r#"{"change":{"s":"close","tf":"M"},"bars":1}"#,
                  r#"{"div":[{"abs":{"change":{"s":"close"},"bars":5}},{"ref":{"s":"atr"},"bars":1}]}"#] {
            let Node::Cmp(c) = node(&format!(r#"{{"lhs":{t},"op":">","rhs":{{"value":0}}}}"#)) else { panic!() };
            let want = arr(&c.lhs, &Frames::new(full.clone()), &mut Cache::new()).unwrap();
            assert!(want.iter().filter(|v| v.is_finite()).count() > 300, "{t}: vacuous");
            for cut in (60..400usize).step_by(7) {
                let got = arr(&c.lhs, &Frames::new(crate::lab::testdata::cut(&full, cut)), &mut Cache::new()).unwrap();
                let upto = if t.contains(r#""tf""#) { cut - 1 } else { cut };
                for i in 0..upto { assert!(close_eq(got[i], want[i]), "{t} bar {i} changes when bars after {cut} exist"); }
            }
        }
    }
}
