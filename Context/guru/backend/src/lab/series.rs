//! Operands → daily-aligned series and compiled conditions. Indicators run on their own timeframe's bars;
//! W/M values reach daily bars through `resample::expand` (usable after the period's last daily close).
use super::bars::Bars;
use super::catalogue;
use super::foreign;
use super::resample::{self, Expand, Tf};
use super::rules::{Agg, Bin, Cond, Dist, Exec, Expr, LabError, Node, Op, Operand, SeriesRef, Spec, Win};
use std::borrow::Cow;
use std::collections::BTreeMap;
use std::rc::Rc;
use std::sync::{Arc, atomic::{AtomicU64, Ordering}};

/// Daily bars of one instrument plus their calendar resamples (built once per request); n-session blocks are built on use.
pub struct Frames { pub d: Bars, pub w: Bars, pub m: Bars, pub q: Bars, pub y: Bars, cache_identity: String }
static FRAME_ID: AtomicU64 = AtomicU64::new(1);
impl Frames {
    pub fn new(d: Bars) -> Frames {
        let r = |tf| resample::resample(&d, tf);
        let (w, m, q, y) = (r(Tf::W), r(Tf::M), r(Tf::Q), r(Tf::Y));
        Frames { d, w, m, q, y, cache_identity: FRAME_ID.fetch_add(1, Ordering::Relaxed).to_string() }
    }
    pub fn bars(&self, tf: Tf) -> Cow<'_, Bars> {
        match tf {
            Tf::D => Cow::Borrowed(&self.d), Tf::W => Cow::Borrowed(&self.w), Tf::M => Cow::Borrowed(&self.m),
            Tf::Q => Cow::Borrowed(&self.q), Tf::Y => Cow::Borrowed(&self.y),
            // shortcut: rebuilt per call; cache per (tf) in Frames if optimizer profiling shows it
            Tf::Days(_) => Cow::Owned(resample::resample(&self.d, tf)),
        }
    }
}
/// Indicator outputs on their own timeframe, keyed by immutable Frames identity, id, tf and params.
/// Frames are built once and borrowed throughout compilation; paired identity also includes both sources and availability policy.
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
pub fn series(fr: &Frames, cache: &mut Cache, s: &SeriesRef) -> Result<Vec<f64>, LabError> { series_mode(fr, cache, s, Expand::Last) }
/// One operand on daily bars with an AmiBroker expand mode (spec §5.1).
fn series_mode(fr: &Frames, cache: &mut Cache, s: &SeriesRef, mode: Expand) -> Result<Vec<f64>, LabError> {
    if s.foreign!=super::rules::Foreign::default(){return Err(LabError::Unsupported("foreign series need the explicit multi-symbol resolver".into()));}
    let held = fr.bars(s.tf);
    let b: &Bars = &held;
    let raw = match price(b, &s.id) {
        Some(r) => r?,
        None => {
            // a formula's identity is its text: two formulas with one name never share outputs
            let id = s.formula.as_ref().map_or(s.id.as_str(), |f| f.text.as_str());
            let key = format!("{}|{}|{:?}|{:?}", fr.cache_identity, id, s.tf, s.params);
            let outs = match cache.get(&key) {
                Some(o) => o.clone(),
                None => {
                    let o = Rc::new(match &s.formula {
                        // errors keep the exact `line L, col C: message` form the editor highlights
                        Some(f) => super::formula::eval(&f.prog, b, &s.params)?,
                        None => {
                            let def = catalogue::get(&s.id).ok_or_else(|| LabError::Invalid(format!("unknown series '{}'", s.id)))?;
                            catalogue::run(&def, b, &s.params).map_err(|e| catalogue_error(&def, e))?
                        }
                    });
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
    Ok(if s.tf == Tf::D { shifted } else { resample::expand_mode(&shifted, b, fr.d.len(), mode) })
}

/// A catalogue run error as a typed lab error (shared by indicator operands and formula indicator calls).
pub fn catalogue_error(def: &catalogue::IndicatorDef, e: String) -> LabError {
    match e.as_str() {
        "needs a second instrument" => LabError::NeedsData(format!("{}: needs a second instrument", def.name)),
        "needs volume" => LabError::NeedsData(format!("{}: needs volume, which is missing on some admitted bars of this instrument", def.name)),
        "needs breadth" => LabError::NeedsData(format!("{}: needs market breadth (advances/declines) on daily bars; this market or timeframe has none", def.name)),
        "needs contract data" => LabError::NeedsData(format!("{}: needs admitted futures contract metadata",def.name)),
        "needs intraday data" => LabError::NeedsData(format!("{}: needs intraday data", def.name)),
        _ => LabError::Invalid(format!("{}: {e}", def.id)),
    }
}
/// Shared loaded instruments and conservative daily-session availability for one request.
#[derive(Default)]
pub struct Others { pub run: String, pub close_fills: bool, pub frames: Arc<BTreeMap<String, Frames>> }
impl Others {
    /// Reuse loaded frames when compiling another portfolio member.
    pub fn with_run(&self, run: &str) -> Others { Others { run: run.into(), close_fills: self.close_fills, frames: self.frames.clone() } }
    fn get<'a>(&'a self, symbol: &str, run: &'a Frames) -> Result<&'a Frames, LabError> {
        if symbol == self.run { return Ok(run); }
        self.frames.get(symbol).ok_or_else(|| LabError::Invalid(format!("{symbol}: not loaded for this request")))
    }
}
/// Compute on the source instrument's calendar, including timeframe, offset and expansion, then align to the run.
pub fn series_in(fr: &Frames, others: &Others, cache: &mut Cache, s: &SeriesRef) -> Result<Vec<f64>, LabError> {
    series_mode_in(fr, others, cache, s, Expand::Last)
}
fn series_mode_in(fr: &Frames, others: &Others, cache: &mut Cache, s: &SeriesRef, mode: Expand) -> Result<Vec<f64>, LabError> {
    let f = &s.foreign;
    if f == &super::rules::Foreign::default() { return series_mode(fr, cache, s, mode); }
    let base_name = f.symbol.as_deref().unwrap_or(&others.run);
    let base = match &f.symbol { Some(t) => others.get(t, fr)?, None => fr };
    let local = SeriesRef { foreign: Default::default(), ..s.clone() };
    let raw = match &f.vs {
        None => series_mode(base, cache, &local, mode)?,
        Some(vs) => {
            let second = if vs == base_name { base } else { others.get(vs, fr)? };
            let policy = foreign::alignment(base_name, vs, others.close_fills);
            let mut d = base.d.clone();
            d.second = Some(foreign::align(&base.d.date, &second.d.date, &second.d.close, policy, false)?);
            // Pair indicators are daily-only. Preserve all native data and resamples; identity makes repeated pair operands reuse raw outputs.
            let mut paired = Frames::new(d);
            paired.cache_identity = format!("pair:{}:{}:{}", base.cache_identity, second.cache_identity, policy.name());
            series_mode(&paired, cache, &local, mode)?
        }
    };
    match &f.symbol {
        None => Ok(raw),
        Some(symbol) => foreign::align(&fr.d.date, &base.d.date, &raw, foreign::alignment(&others.run, symbol, others.close_fills), f.fixup),
    }
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
/// A compiled side. `other` is the short side of a `direction: both` spec; `cost` is the run's cost per side
/// (fraction), used by equity-level rules (spec §6.5).
/// `limit_at` / `stop_at` hold the daily values of `entry.limit.at` / `entry.stop.at` operands.
pub struct Compiled {
    pub entry: Vec<RCond>, pub exit: Vec<RCond>, pub exec: Exec, pub atr: BTreeMap<usize, Vec<f64>>, pub other: Option<Box<Compiled>>, pub cost: f64,
    pub limit_at: Option<Vec<f64>>, pub stop_at: Option<Vec<f64>>,
    /// (actual tick, forward offset, ratio factor) per execution bar; empty for cash.
    pub grid: Vec<(f64,f64,f64)>, pub expiry_bar: Vec<bool>,
}
impl Compiled {
    /// Long, market orders at the next open, no price exits (the v1 defaults).
    pub fn new(entry: Vec<RCond>, exit: Vec<RCond>) -> Compiled { Compiled { entry, exit, exec: Exec::default(), atr: BTreeMap::new(), other: None, cost: 0., limit_at: None, stop_at: None, grid: vec![], expiry_bar: vec![] } }
    pub fn has_order(&self) -> bool { self.exec.has_order() }
    /// Price distance of `d` from `px`, using ATR at bar `k` (a bar whose close is already known). NaN while ATR warms up.
    pub fn dist(&self, d: Dist, px: f64, k: usize) -> f64 {
        self.dist_at(d,px,k,k)
    }
    pub fn dist_at(&self, d: Dist, px: f64, source: usize, at: usize) -> f64 {
        let (_,off,factor)=self.grid.get(at).copied().unwrap_or((0.,0.,1.));
        match d { Dist::Pct(p) => (px-off)*p/100., Dist::Atr { mult,length } => self.atr.get(&length).map_or(f64::NAN,|a|mult*a[source]), Dist::Points(p) => p*factor,
            Dist::Ticks(t) => self.grid.get(at).map_or(f64::NAN,|g|t*g.0*g.2), Dist::ProfitPct(_) => f64::NAN } // a trailing level, not a distance (simulate.rs)

    }
    pub fn round_at(&self, i: usize, px: f64) -> f64 {
        self.grid.get(i).map_or(px, |&(tick,off,factor)| (((px-off)/factor/tick).round()*tick)*factor+off)
    }

}

/// Compile a single-instrument spec without a foreign-data context.
pub fn compile(spec: &Spec, fr: &Frames, cache: &mut Cache) -> Result<Compiled, LabError> { compile_with(spec, fr, &Others::default(), cache) }
/// Existing local mask helper; multi-instrument compilation uses the same recursive evaluator with Others.
pub fn mask(n: &Node, fr: &Frames, cache: &mut Cache) -> Result<Vec<f64>, LabError> { mask_in(n, fr, &Others::default(), cache) }
#[cfg(test)]
fn arr(o: &Operand, fr: &Frames, cache: &mut Cache) -> Result<Vec<f64>, LabError> { operand_values(fr, &Others::default(), cache, o) }
/// Evaluate a complete operand (including expressions and state) for a portfolio member's score.
pub fn operand_values(fr: &Frames, others: &Others, cache: &mut Cache, operand: &Operand) -> Result<Vec<f64>, LabError> { arr_in(operand, fr, others, cache) }
fn val(o: &Operand, fr: &Frames, others: &Others, cache: &mut Cache) -> Result<Val, LabError> {
    Ok(match o { Operand::Series(s) => Val::Arr(series_in(fr, others, cache, s)?), Operand::Value(x) => Val::Const(*x), Operand::BarsHeld => Val::Held, Operand::Expr(e) => Val::Arr(eval(e, fr, others, cache)?) })
}
/// A v1 comparison compiles exactly as before; every other node becomes a precomputed 1/0/NaN array tested with `> 0.5`.
fn list(ns: &[Node], at: &str, fr: &Frames, others: &Others, cache: &mut Cache) -> Result<Vec<RCond>, LabError> {
    ns.iter().enumerate().map(|(k, n)| Ok(match n {
        Node::Cmp(c) => RCond { lhs: val(&c.lhs, fr, others, cache)?, op: c.op, rhs: val(&c.rhs, fr, others, cache)?, label: format!("{at}[{k}]: {}", describe(c)) },
        _ => RCond { lhs: Val::Arr(mask_in(n, fr, others, cache)?), op: Op::Gt, rhs: Val::Const(0.5), label: format!("{at}[{k}]: {}", describe_node(n)) },
    })).collect()
}
/// An operand as a daily array (constants repeated). bars_held never reaches here: the parser keeps it top-level.
fn arr_in(o: &Operand, fr: &Frames, others: &Others, cache: &mut Cache) -> Result<Vec<f64>, LabError> {
    match val(o, fr, others, cache)? {
        Val::Arr(v) => Ok(v),
        Val::Const(x) => Ok(vec![x; fr.d.len()]),
        Val::Held => Err(LabError::Invalid("bars_held is only available in top-level exit conditions".into())),
    }
}
/// An expression on the daily bars (spec §4.1–§4.2): a NaN input gives NaN, division by zero gives NaN, windows are NaN until full.
fn eval(e: &Expr, fr: &Frames, others: &Others, cache: &mut Cache) -> Result<Vec<f64>, LabError> {
    let len = fr.d.len();
    let nan = f64::NAN;
    Ok(match e {
        Expr::Bin(op, l, r) => {
            let (a, b) = (arr_in(l, fr, others, cache)?, arr_in(r, fr, others, cache)?);
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
        Expr::Abs(x) => arr_in(x, fr, others, cache)?.into_iter().map(f64::abs).collect(),
        Expr::Ref(x, n) => { let a = arr_in(x, fr, others, cache)?; (0..len).map(|i| if i >= *n { a[i - n] } else { nan }).collect() }
        Expr::Win(Win::Change, x, n) => { let a = arr_in(x, fr, others, cache)?; (0..len).map(|i| if i >= *n { a[i] - a[i - n] } else { nan }).collect() }
        // State operands follow Kleene logic (spec §4.2): an unknown bar that could change the answer makes it unknown.
        Expr::BarsSince(c) => {
            let (mut last, mut unk) = (None, None);
            mask_in(c, fr, others, cache)?.iter().enumerate().map(|(i, m)| {
                if *m == 1. { last = Some(i); } else if m.is_nan() { unk = Some(i); }
                match last { Some(l) if unk < Some(l) => (i - l) as f64, _ => nan }
            }).collect()
        }
        Expr::Count(c, n) => {
            let m = mask_in(c, fr, others, cache)?;
            let (mut count, mut unk) = (0usize, 0usize);
            (0..len).map(|i| {
                count += usize::from(m[i] == 1.); unk += usize::from(m[i].is_nan());
                if i >= *n { count -= usize::from(m[i - n] == 1.); unk -= usize::from(m[i - n].is_nan()); }
                if i + 1 < *n || unk > 0 { nan } else { count as f64 }
            }).collect()
        }
        Expr::ValueWhen { when, of, nth } => {
            let (m, a) = (mask_in(when, fr, others, cache)?, arr_in(of, fr, others, cache)?);
            let (mut hits, mut unk): (Vec<usize>, Option<usize>) = (Vec::new(), None);
            (0..len).map(|i| {
                if m[i] == 1. { hits.push(i); } else if m[i].is_nan() { unk = Some(i); }
                if hits.len() >= *nth && unk < Some(hits[hits.len() - nth]) { a[hits[hits.len() - nth]] } else { nan }
            }).collect()
        }
        Expr::Since { when, of, highest } => {
            // AmiBroker HighestSince/LowestSince: from the latest bar where `when` held (inclusive). A missing `of` on any bar
            // since then, like an unknown `when`, makes the extreme unknown (Kleene, spec §4.2).
            let (m, a) = (mask_in(when, fr, others, cache)?, arr_in(of, fr, others, cache)?);
            let (mut started, mut unk, mut ext) = (false, false, nan);
            (0..len).map(|i| {
                if m[i] == 1. { started = true; unk = a[i].is_nan(); ext = a[i]; }
                else if m[i].is_nan() || a[i].is_nan() { unk = true; }
                else if started { ext = if *highest { ext.max(a[i]) } else { ext.min(a[i]) }; }
                if started && !unk { ext } else { nan }
            }).collect()
        }
        Expr::Expand(r, mode) => series_mode_in(fr, others, cache, r, *mode)?,
        Expr::Compress { of, tf, agg, offset, expand } => {
            let (a, held) = (arr_in(of, fr, others, cache)?, fr.bars(*tf));
            let b: &Bars = &held;
            let c: Vec<f64> = (0..b.len()).map(|w| {
                let g = &a[if w == 0 { 0 } else { b.end[w - 1] + 1 }..=b.end[w]];
                match agg {
                    Agg::Last => g[g.len() - 1],
                    Agg::First => g[0],
                    _ if g.iter().any(|v| v.is_nan()) => nan,
                    Agg::High => g.iter().copied().fold(f64::MIN, f64::max),
                    Agg::Low => g.iter().copied().fold(f64::MAX, f64::min),
                    Agg::Sum => g.iter().sum(),
                }
            }).collect();
            let shifted: Vec<f64> = (0..c.len()).map(|w| if w >= *offset { c[w - offset] } else { nan }).collect();
            resample::expand_mode(&shifted, b, len, *expand)
        }
        Expr::Win(w, x, n) => {
            let a = arr_in(x, fr, others, cache)?;
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
fn mask_in(n: &Node, fr: &Frames, others: &Others, cache: &mut Cache) -> Result<Vec<f64>, LabError> {
    let len = fr.d.len();
    Ok(match n {
        Node::Cmp(c) => { let (a, b) = (arr_in(&c.lhs, fr, others, cache)?, arr_in(&c.rhs, fr, others, cache)?); (0..len).map(|i| cmp_at(c.op, &a, &b, i)).collect() }
        Node::Range { x, lo, hi, outside } => {
            let (x, lo, hi) = (arr_in(x, fr, others, cache)?, arr_in(lo, fr, others, cache)?, arr_in(hi, fr, others, cache)?);
            (0..len).map(|i| if x[i].is_nan() || lo[i].is_nan() || hi[i].is_nan() { f64::NAN } else { truth((lo[i] <= x[i] && x[i] <= hi[i]) != *outside) }).collect()
        }
        Node::Trend { x, bars, rising } => {
            let x = arr_in(x, fr, others, cache)?;
            (0..len).map(|i| {
                if i < *bars || x[i - bars..=i].iter().any(|v| v.is_nan()) { return f64::NAN; }
                truth(x[i - bars..=i].windows(2).all(|w| if *rising { w[1] > w[0] } else { w[1] < w[0] }))
            }).collect()
        }
        Node::All(v) | Node::Any(v) => {
            let all = matches!(n, Node::All(_));
            let kids = v.iter().map(|c| mask_in(c, fr, others, cache)).collect::<Result<Vec<_>, _>>()?;
            (0..len).map(|i| {
                let (decisive, other) = if all { (0., 1.) } else { (1., 0.) };
                if kids.iter().any(|k| k[i] == decisive) { decisive } else if kids.iter().any(|k| k[i].is_nan()) { f64::NAN } else { other }
            }).collect()
        }
        Node::Not(c) => mask_in(c, fr, others, cache)?.into_iter().map(|v| if v.is_nan() { v } else { 1. - v }).collect(),
        Node::AtLeast { k, of, when } => {
            // Kleene count over the last `of` bars: true if trues >= k, false if even every unknown could not reach k, else unknown.
            let m = mask_in(when, fr, others, cache)?;
            let (mut t, mut u) = (0usize, 0usize);
            (0..len).map(|i| {
                t += usize::from(m[i] == 1.); u += usize::from(m[i].is_nan());
                if i >= *of { t -= usize::from(m[i - of] == 1.); u -= usize::from(m[i - of].is_nan()); }
                if i + 1 < *of { f64::NAN } else if t >= *k { 1. } else if t + u < *k { 0. } else { f64::NAN }
            }).collect()
        }
        Node::Then { steps, within } => {
            let ms = steps.iter().map(|c| mask_in(c, fr, others, cache)).collect::<Result<Vec<_>, _>>()?;
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
pub fn compile_with(spec: &Spec, fr: &Frames, others: &Others, cache: &mut Cache) -> Result<Compiled, LabError> {
    let mut c = side(&spec.entry, &spec.exit, &spec.exec, "", fr, others, cache)?;
    if let Some(s) = &spec.short_side {
        let mut o = side(&s.entry, &s.exit, &s.exec, "short_side.", fr, others, cache)?;
        o.exec.sizing = spec.exec.sizing; // one sizing for both sides (optimize changes the spec's)
        c.other = Some(Box::new(o));
    }
    Ok(c)
}
fn side(entry: &[Node], exit: &[Node], x: &Exec, pre: &str, fr: &Frames, others: &Others, cache: &mut Cache) -> Result<Compiled, LabError> {
    let mut c = Compiled { entry: list(entry, &format!("{pre}entry.all"), fr, others, cache)?, exit: list(exit, &format!("{pre}exit.any"), fr, others, cache)?, ..Compiled::new(vec![], vec![]) };
    c.exec = x.clone();
    c.limit_at = x.limit_at.as_ref().map(|o| arr_in(o, fr, others, cache)).transpose()?;
    c.stop_at = x.stop_at.as_ref().map(|o| arr_in(o, fr, others, cache)).transpose()?;
    let later = x.breakeven.iter().flat_map(|b| [Some(b.0), b.1]).chain(x.take_profits.iter().map(|t| Some(t.0)));
    for d in [x.limit, x.stop_loss, x.target, x.trailing, x.stop, x.trail_activate].into_iter().chain(later).flatten() {
        if let Dist::Atr { length, .. } = d {
            if c.atr.contains_key(&length) { continue; }
            let r = SeriesRef { id: "atr".into(), params: [("length".to_string(), length as f64)].into(), tf: Tf::D, out: 0, offset: 0, foreign: Default::default(), formula: None };
            c.atr.insert(length, series(fr, cache, &r)?);
        }
    }
    Ok(c)
}

/// Human-readable condition, recorded as a trade's exit reason, e.g. `rsi(length=2) crosses_above 70`.
pub fn describe(c: &Cond) -> String {
    format!("{} {} {}", operand_text(&c.lhs), op_text(c.op), operand_text(&c.rhs))
}
/// Parameters added to existing ids after the Phase 6/7 master list (cross-platform variants, 2026-10-19). Labels are
/// exit reasons, hashed by pinned.rs and saved with results, so these keys print only when set off their default: a spec
/// written before they existed keeps its exact label. Earlier keys (stoch/trix/percentrank `variant`) always print.
pub const LATE_PARAMS: &[(&str, &str)] = &[
    ("atr", "variant"), ("bb", "source"), ("kc", "basis_type"), ("kc", "source"), ("kc", "range"), ("hv", "days"),
    ("mom", "variant"), ("dpo", "variant"), ("stochrsi", "variant"), ("rsi", "variant"), ("roc", "variant"),
    ("macd", "signal_type"), ("macd", "ma_type"), ("dmi", "variant"), ("efi", "variant"), ("eom", "variant"),
    ("pvo", "exponential"), ("pvo", "points"), ("price_osc", "points"), ("tsf", "horizon"), ("majority_rule", "variant"),
    ("hma", "variant"), ("median", "source"), ("vwap_period", "anchor_date"), ("cdl_kicking_bull", "variant"), ("cdl_kicking_bear", "variant"),
];
/// The resolved parameters a label or echo shows: all of them except a `LATE_PARAMS` key at its catalogue default.
pub fn shown_params<'a>(id: &str, params: &'a catalogue::Params) -> impl Iterator<Item = (&'a String, &'a f64)> {
    let late: Vec<(&'static str, f64)> = LATE_PARAMS.iter().filter(|(i, _)| *i == id)
        .filter_map(|(_, k)| catalogue::get(id)?.params.iter().find(|p| p.key == *k).map(|p| (p.key, p.default))).collect();
    params.iter().filter(move |(k, v)| !late.iter().any(|&(lk, d)| lk == k.as_str() && d == **v))
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
            Expr::BarsSince(c) => format!("bars_since({})", describe_node(c)),
            Expr::Count(c, n) => format!("count({}, {n})", describe_node(c)),
            Expr::ValueWhen { when, of, nth } => format!("value_when({}, {}, {nth})", describe_node(when), operand_text(of)),
            Expr::Since { when, of, highest } => format!("{}({}, {})", if *highest { "highest_since" } else { "lowest_since" }, describe_node(when), operand_text(of)),
            Expr::Expand(r, mode) => format!("{} expand {}", operand_text(&Operand::Series(r.clone())), mode.key()),
            Expr::Compress { of, tf, agg, offset, expand } => format!("compress({}, {}, {}){} expand {}", operand_text(of), tf.name(), agg.key(), if *offset > 0 { format!("[{offset}]") } else { String::new() }, expand.key()),
        },
        Operand::Series(s) => {
            let mut t = if let Some(f) = &s.formula { super::formula::label(f, &s.params, s.out) } else {
                let p: Vec<String> = shown_params(&s.id, &s.params).map(|(k, v)| format!("{k}={v}")).collect();
                if p.is_empty() { s.id.clone() } else { format!("{}({})", s.id, p.join(",")) }
            };
            if s.out > 0 && s.formula.is_none() { if let Some(d) = catalogue::get(&s.id) { t = format!("{t}.{}", d.outputs[s.out]); } }
            if s.tf != Tf::D { t = format!("{t}@{}", s.tf.name()); }
            if s.offset > 0 { t = format!("{t}[{}]", s.offset); }
            if let Some(vs)=&s.foreign.vs {t=format!("{t} vs {vs}");}
            if let Some(symbol)=&s.foreign.symbol {t=format!("{symbol}:{t}");}
            if s.foreign.fixup {t.push_str(" (fixup previous)");}
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
        SeriesRef { id: id.into(), params, tf, out, offset, foreign: Default::default(), formula: None }
    }

    #[test] fn every_computed_indicator_is_reachable_from_a_rule() {
        for d in all().iter().filter(|d| d.compute.is_some() && !d.needs.derivatives && !d.needs.second_symbol) {
            let out = d.outputs[d.outputs.len() - 1];
            if let Err(e) = crate::lab::testdata::compiles(d.id, out) { panic!("{}.{out}: {} {}", d.id, e.status(), e.message()); }
        }
    }
    #[test] fn listed_entries_refuse_with_their_note() {
        for (id, status) in [("vwap", "needs_data"), ("cvd", "needs_data"), ("dividend_yield", "needs_data"),
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
        for d in all().iter().filter(|d| d.compute.is_some() && !d.needs.second_symbol && !crate::lab::periods::USES_SESSION_DATES.contains(&d.id)) {
            for tf in [Tf::D, Tf::W, Tf::M, Tf::Q, Tf::Y, Tf::Days(5)] {
                let r = sref(d.id, tf, d.outputs.len() - 1, 0);
                if (d.needs.breadth && tf != Tf::D) || d.needs.derivatives { assert_eq!(series(&full, &mut Cache::new(), &r).unwrap_err().status(), "needs_data"); continue; }
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
            for d in all().iter().filter(|d| d.compute.is_some() && !d.needs.second_symbol) {
                let picks: [fn(&catalogue::ParamSpec) -> f64; 2] = [|p| p.min, |p| p.max];
                for pick in picks {
                    let params: Params = d.params.iter().map(|p| (p.key.to_string(), pick(p))).collect();
                    for tf in [Tf::D, Tf::W, Tf::M, Tf::Q, Tf::Y, Tf::Days(5)] {
                        let r = SeriesRef { id: d.id.into(), params: params.clone(), tf, out: 0, offset: 0, foreign: Default::default(), formula: None };
                        if (d.needs.breadth && tf != Tf::D) || d.needs.derivatives { assert_eq!(series(&fr, &mut Cache::new(), &r).unwrap_err().status(), "needs_data"); continue; }
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
    #[test] fn late_params_print_only_off_default() {
        for (id, key) in super::LATE_PARAMS { assert!(catalogue::get(id).is_some_and(|d| d.params.iter().any(|p| p.key == *key)), "{id}.{key} not in the catalogue"); }
        let text = |p: &str| { let s = parse_spec(&format!(r#"{{"v":1,"entry":{{"all":[{{"lhs":{{"s":"macd","p":{{{p}}}}},"op":"<","rhs":{{"value":0}}}}]}},"exit":{{"any":[{{"lhs":{{"s":"bars_held"}},"op":">=","rhs":{{"value":3}}}}]}}}}"#)).unwrap(); describe(s.entry[0].cmp().unwrap()) };
        assert_eq!(text(r#""signal_type":0"#), "macd(fast=12,signal=9,slow=26) < 0");
        assert_eq!(text(r#""signal_type":1,"ma_type":2"#), "macd(fast=12,ma_type=2,signal=9,signal_type=1,slow=26) < 0");
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

    // ---- Phase 7: state operands (spec §4.1)
    #[test] fn state_operands_follow_amibroker_definitions() {
        let fr = Frames::new(crate::lab::testdata::closes(&[1., 3., 2., 5., 4., 6., 1.]));
        let x = f64::NAN;
        let up = r#"{"lhs":{"s":"close"},"op":">","rhs":{"s":"close","offset":1}}"#; // unknown, 1, 0, 1, 0, 1, 0
        let e = |t: &str| { let Node::Cmp(c) = node(&format!(r#"{{"lhs":{t},"op":">","rhs":{{"value":0}}}}"#)) else { panic!() }; arr(&c.lhs, &fr, &mut Cache::new()).unwrap() };
        assert_series(&e(&format!(r#"{{"bars_since":{up}}}"#)), &[x, 0., 1., 0., 1., 0., 1.]);
        assert_series(&e(&format!(r#"{{"value_when":{up},"of":{{"s":"close"}}}}"#)), &[x, 3., 3., 5., 5., 6., 6.]);
        assert_series(&e(&format!(r#"{{"value_when":{up},"of":{{"s":"close"}},"nth":2}}"#)), &[x, x, x, 3., 3., 5., 5.]);
        assert_series(&e(&format!(r#"{{"highest_since":{up},"of":{{"s":"close"}}}}"#)), &[x, 3., 3., 5., 5., 6., 6.]);
        assert_series(&e(&format!(r#"{{"lowest_since":{up},"of":{{"s":"close"}}}}"#)), &[x, 3., 2., 5., 4., 6., 1.]);
        assert_series(&e(&format!(r#"{{"count":{up},"bars":3}}"#)), &[x, x, x, 2., 1., 2., 1.]); // bar 2's window holds the unknown bar 0 (Kleene)
    }
    #[test] fn state_operands_never_use_later_bars() {
        let full = business_days(420, true);
        let rsi_low = r#"{"lhs":{"s":"rsi","p":{"length":2}},"op":"<","rhs":{"value":20}}"#;
        for t in [format!(r#"{{"bars_since":{rsi_low}}}"#), format!(r#"{{"value_when":{rsi_low},"of":{{"s":"low"}},"nth":2}}"#),
                  format!(r#"{{"highest_since":{rsi_low},"of":{{"s":"high"}}}}"#), format!(r#"{{"lowest_since":{{"lhs":{{"s":"close"}},"op":">","rhs":{{"s":"sma","p":{{"length":4}},"tf":"W"}}}},"of":{{"s":"close"}}}}"#),
                  format!(r#"{{"count":{rsi_low},"bars":10}}"#)] {
            let Node::Cmp(c) = node(&format!(r#"{{"lhs":{t},"op":">","rhs":{{"value":0}}}}"#)) else { panic!() };
            let want = arr(&c.lhs, &Frames::new(full.clone()), &mut Cache::new()).unwrap();
            assert!(want.iter().filter(|v| v.is_finite()).count() > 300, "{t}: vacuous");
            for cut in (60..400usize).step_by(7) {
                let got = arr(&c.lhs, &Frames::new(crate::lab::testdata::cut(&full, cut)), &mut Cache::new()).unwrap();
                // only W/M items may differ on the cut bar itself (the still-open period's value)
                let upto = if t.contains(r#""tf""#) { cut - 1 } else { cut };
                for i in 0..upto { assert!(close_eq(got[i], want[i]), "{t} bar {i} changes when bars after {cut} exist"); }
            }
        }
    }
    #[test] fn state_operands_stay_unknown_through_warm_up() {
        let fr = Frames::new(business_days(300, false));
        let c = r#"{"lhs":{"s":"close"},"op":">","rhs":{"s":"sma","p":{"length":200}}}"#;
        let base = mask(&node(c), &fr, &mut Cache::new()).unwrap();
        assert!(base[210..].iter().any(|v| *v == 1.) && base[210..].iter().any(|v| *v == 0.), "vacuous");
        for t in [format!(r#"{{"bars_since":{c}}}"#), format!(r#"{{"count":{c},"bars":1}}"#), format!(r#"{{"value_when":{c},"of":{{"s":"close"}}}}"#),
                  format!(r#"{{"highest_since":{c},"of":{{"s":"close"}}}}"#), format!(r#"{{"lowest_since":{c},"of":{{"s":"close"}}}}"#)] {
            // `x >= 0` is true whenever x is known, so its `not` is false when known and unknown only while x is unknown
            let n = mask(&node(&format!(r#"{{"not":{{"lhs":{t},"op":">=","rhs":{{"value":0}}}}}}"#)), &fr, &mut Cache::new()).unwrap();
            for i in 0..199 { assert!(n[i].is_nan(), "{t}: bar {i} must be unknown in warm-up"); }
        }
        // after warm-up count(c, 1) is the plain truth of c
        let Node::Cmp(k) = node(&format!(r#"{{"lhs":{{"count":{c},"bars":1}},"op":">","rhs":{{"value":0}}}}"#)) else { panic!() };
        let k = arr(&k.lhs, &fr, &mut Cache::new()).unwrap();
        for i in 199..300 { assert_eq!(k[i], base[i], "count bar {i}"); }
        // an unknown bar after the latest true makes the answer unknown; one before it does not
        let fr = Frames::new(crate::lab::testdata::closes(&[3., 1., f64::NAN, 1., 4., 1.]));
        let (x, big) = (f64::NAN, r#"{"lhs":{"s":"close"},"op":">","rhs":{"value":2}}"#); // 1, 0, unknown, 0, 1, 0
        let e = |t: &str| { let Node::Cmp(c) = node(&format!(r#"{{"lhs":{t},"op":">","rhs":{{"value":0}}}}"#)) else { panic!() }; arr(&c.lhs, &fr, &mut Cache::new()).unwrap() };
        assert_series(&e(&format!(r#"{{"bars_since":{big}}}"#)), &[0., 1., x, x, 0., 1.]);
        assert_series(&e(&format!(r#"{{"count":{big},"bars":2}}"#)), &[x, 1., x, x, 1., 1.]);
        assert_series(&e(&format!(r#"{{"value_when":{big},"of":{{"s":"close"}}}}"#)), &[3., 3., x, x, 4., 4.]);
        assert_series(&e(&format!(r#"{{"value_when":{big},"of":{{"s":"close"}},"nth":2}}"#)), &[x, x, x, x, x, x]); // bar 2 might have been the 2nd-latest
        assert_series(&e(&format!(r#"{{"highest_since":{big},"of":{{"s":"close"}}}}"#)), &[3., 3., x, x, 4., 4.]);
        assert_series(&e(&format!(r#"{{"lowest_since":{big},"of":{{"s":"close"}}}}"#)), &[3., 1., x, x, 4., 1.]);
        // a missing `of` since the start bar is unknown too (division by zero where close = 1)
        let fr = Frames::new(crate::lab::testdata::closes(&[3., 2., 1., 2., 3., 2.]));
        let e = |t: &str| { let Node::Cmp(c) = node(&format!(r#"{{"lhs":{t},"op":">","rhs":{{"value":0}}}}"#)) else { panic!() }; arr(&c.lhs, &fr, &mut Cache::new()).unwrap() };
        let (hi, of) = (r#"{"lhs":{"s":"close"},"op":">","rhs":{"value":2.5}}"#, r#"{"div":[{"s":"close"},{"sub":[{"s":"close"},{"value":1}]}]}"#); // 1, 0, 0, 0, 1, 0 / 1.5, 2, -, 2, 1.5, 2
        assert_series(&e(&format!(r#"{{"highest_since":{hi},"of":{of}}}"#)), &[1.5, 2., x, x, 1.5, 2.]);
        assert_series(&e(&format!(r#"{{"lowest_since":{hi},"of":{of}}}"#)), &[1.5, 1.5, x, x, 1.5, 1.5]);
        assert_series(&e(&format!(r#"{{"highest_since":{{"lhs":{{"s":"close"}},"op":"<","rhs":{{"value":1.5}}}},"of":{of}}}"#)), &[x, x, x, x, x, x]); // the start bar's own `of` is missing
    }
    #[test] fn the_amibroker_bars_since_time_exit_is_expressible() {
        // hbqts_entry_id1's book exit counts bars since the latest (repeating) buy signal; the lab can now say so.
        let spec = parse_spec(r#"{"v":1,"entry":{"all":[{"lhs":{"s":"sma","p":{"length":3}},"op":">","rhs":{"s":"sma","p":{"length":16}}}]},
            "exit":{"any":[{"lhs":{"bars_since":{"lhs":{"s":"sma","p":{"length":3}},"op":">","rhs":{"s":"sma","p":{"length":16}}}},"op":">=","rhs":{"value":6}}]},"execution":{"fill":"close"}}"#).unwrap();
        let c = compile(&spec, &Frames::new(business_days(200, false)), &mut Cache::new()).unwrap();
        assert_eq!(c.exit[0].label, "exit.any[0]: bars_since(sma(length=3) > sma(length=16)) >= 6");
    }

    // ---- Phase 7: AmiBroker TimeFrame semantics (spec §5.1)
    fn operand_of(t: &str) -> Operand { let Node::Cmp(c) = node(&format!(r#"{{"lhs":{t},"op":">","rhs":{{"value":0}}}}"#)) else { panic!() }; c.lhs }
    #[test] fn each_expand_mode_is_causal_and_placed_like_amibroker() {
        let fr = Frames::new(business_days(30, false)); // 2021-01-04 is a Monday: weeks are daily 0-4, 5-9, …
        let x = f64::NAN;
        let v = |t: &str| arr(&operand_of(t), &fr, &mut Cache::new()).unwrap();
        let (c, o) = (&fr.d.close, &fr.d.open);
        // last (the default, = v1): week 1's close from its Friday (4) until week 2's Friday (9)
        assert_series(&v(r#"{"s":"close","tf":"W"}"#)[..10], &[x, x, x, x, c[4], c[4], c[4], c[4], c[4], c[9]]);
        // point: only on each week's last day
        assert_series(&v(r#"{"s":"close","tf":"W","expand":"point"}"#)[..10], &[x, x, x, x, c[4], x, x, x, x, c[9]]);
        // first + offset 1: the previous completed week from Monday (TradingView expr[1] with lookahead_on)
        assert_series(&v(r#"{"s":"close","tf":"W","offset":1,"expand":"first"}"#)[..10], &[x, x, x, x, x, c[4], c[4], c[4], c[4], c[4]]);
        // first on the open: the week's own open from Monday
        assert_series(&v(r#"{"s":"open","tf":"W","expand":"first"}"#)[..7], &[o[0], o[0], o[0], o[0], o[0], o[5], o[5]]);
        // compress: the week's highest close at its Friday (last), and the week's first close from Monday (first)
        let hi = c[..5].iter().copied().fold(f64::MIN, f64::max);
        assert_series(&v(r#"{"compress":{"s":"close"},"tf":"W","mode":"high"}"#)[..6], &[x, x, x, x, hi, hi]);
        assert_series(&v(r#"{"compress":{"s":"close"},"tf":"W","mode":"first","expand":"first"}"#)[..6], &[c[0], c[0], c[0], c[0], c[0], c[5]]);
    }
    #[test] fn lookahead_expand_first_is_refused_with_a_note() {
        for (t, status, needle) in [
            (r#"{"s":"close","tf":"W","expand":"first"}"#, "unsupported", "expandFirst would show the period's final value"),
            (r#"{"s":"high","tf":"M","expand":"first"}"#, "unsupported", "lookahead"),
            (r#"{"compress":{"s":"close"},"tf":"W","mode":"high","expand":"first"}"#, "unsupported", "lookahead"),
            (r#"{"s":"close","expand":"point"}"#, "invalid", "expand applies to W, M, Q, Y and nD"),
            (r#"{"s":"close","tf":"W","expand":"later"}"#, "invalid", "expand must be last, first or point"),
            (r#"{"compress":{"s":"close"},"tf":"D"}"#, "invalid", "tf must be one of W, M, Q, Y or nD"),
            (r#"{"s":"close","tf":"1D"}"#, "invalid", "tf must be D, W, M, Q, Y or nD (2D to 63D)"),
        ] {
            let e = parse_spec(&format!(r#"{{"v":1,"entry":{{"all":[{{"lhs":{t},"op":">","rhs":{{"value":0}}}}]}},"exit":{{"any":[{{"lhs":{{"s":"bars_held"}},"op":">=","rhs":{{"value":1}}}}]}}}}"#)).unwrap_err();
            assert_eq!(e.status(), status, "{t}");
            assert!(e.message().contains(needle), "{needle:?} not in {:?}", e.message());
        }
    }
    #[test] fn a_quarter_close_is_invisible_until_the_quarters_last_session() {
        let fr = Frames::new(business_days(200, false)); // from 2021-01-04; Q1 ends 2021-03-31
        let q = series(&fr, &mut Cache::new(), &sref("close", Tf::Q, 0, 0)).unwrap();
        let last_q1 = fr.d.date.iter().rposition(|d| d.as_str() <= "2021-03-31").unwrap();
        assert!(q[..last_q1].iter().all(|v| v.is_nan()));
        assert!(q[last_q1..last_q1 + 20].iter().all(|v| *v == fr.d.close[last_q1]));
    }
    #[test] fn expand_modes_and_compress_never_use_later_bars() {
        let full = business_days(420, true);
        for t in [r#"{"s":"close","tf":"W","expand":"point"}"#, r#"{"s":"rsi","tf":"M","offset":1,"expand":"first"}"#, r#"{"s":"open","tf":"Q","expand":"first"}"#,
                  r#"{"s":"sma","p":{"length":3},"tf":"5D"}"#, r#"{"compress":{"s":"rsi","p":{"length":2}},"tf":"W","mode":"low"}"#,
                  r#"{"compress":{"s":"volume"},"tf":"M","mode":"sum","offset":1,"expand":"first"}"#, r#"{"s":"close","tf":"Y","offset":1,"expand":"first"}"#,
                  r#"{"s":"close","tf":"Q","expand":"point"}"#, r#"{"s":"close","tf":"Y","expand":"point"}"#, r#"{"s":"rsi","tf":"5D","expand":"point"}"#,
                  r#"{"compress":{"s":"close"},"tf":"Q","mode":"high"}"#, r#"{"compress":{"s":"high"},"tf":"Y","mode":"high","expand":"point"}"#,
                  r#"{"compress":{"s":"volume"},"tf":"5D","mode":"sum"}"#, r#"{"compress":{"s":"volume"},"tf":"Q","mode":"sum","expand":"point"}"#,
                  r#"{"s":"close","tf":"5D","offset":1,"expand":"first"}"#] {
            let o = operand_of(t);
            let want = arr(&o, &Frames::new(full.clone()), &mut Cache::new()).unwrap();
            assert!(want.iter().any(|v| v.is_finite()), "{t}: vacuous");
            for cut in (60..400usize).step_by(7) {
                let got = arr(&o, &Frames::new(crate::lab::testdata::cut(&full, cut)), &mut Cache::new()).unwrap();
                // expand first shows only values complete by the period's first bar, so even the cut bar must match;
                // last/point may show the still-open period's value on the cut bar itself
                let upto = if t.contains(r#""expand":"first""#) { cut } else { cut - 1 };
                for i in 0..upto { assert!(close_eq(got[i], want[i]), "{t} bar {i} changes when bars after {cut} exist"); }
            }
        }
    }
}

#[cfg(test)]
mod foreign_resolver_tests {
    use super::*;
    use super::tests::business_days;
    fn sref(id:&str,tf:Tf,out:usize,offset:usize)->SeriesRef { let params=catalogue::get(id).map_or(Default::default(),|d|catalogue::resolve_params(&d,&Default::default()).unwrap()); SeriesRef{id:id.into(),params,tf,out,offset,foreign:Default::default(),formula:None} }
    use crate::lab::rules::{parse_spec,Foreign};
    use crate::lab::testdata::{assert_series,close_eq};
    fn others(run:&str,close_fills:bool,list:Vec<(&str,Bars)>)->Others {Others{run:run.into(),close_fills,frames:std::sync::Arc::new(list.into_iter().map(|(t,b)|(t.into(),Frames::new(b))).collect())}}
    fn reference(id:&str,symbol:Option<&str>,vs:Option<&str>,fixup:bool)->SeriesRef {SeriesRef{foreign:Foreign{symbol:symbol.map(Into::into),vs:vs.map(Into::into),fixup},..sref(id,Tf::D,0,0)}}
    fn operand(text:&str)->Operand {parse_spec(&format!(r#"{{"v":1,"entry":{{"all":[{{"lhs":{text},"op":">","rhs":{{"value":0}}}}]}},"exit":{{"any":[{{"lhs":{{"s":"bars_held"}},"op":">=","rhs":{{"value":3}}}}]}}}}"#)).unwrap().entry[0].cmp().unwrap().lhs.clone()}
    fn remove(b:&mut Bars,i:usize) {for v in [&mut b.open,&mut b.high,&mut b.low,&mut b.close]{v.remove(i);}b.date.remove(i);b.end=(0..b.len()).collect();if let Some(v)=&mut b.volume{v.remove(i);}}
    #[test] fn own_calendar_offsets_expand_and_fixup_run_before_alignment() {
        let run=business_days(90,true);let mut source=business_days(90,true);for v in [&mut source.open,&mut source.high,&mut source.low,&mut source.close]{for p in v{*p*=10.;}}remove(&mut source,20);
        let fr=Frames::new(run);let o=others("SPY",false,vec![("SPX",source.clone())]);
        let got=series_in(&fr,&o,&mut Cache::new(),&reference("close",Some("SPX"),None,false)).unwrap();assert!(got[20].is_nan());assert_eq!(got[21],source.close[20]);
        let got=series_in(&fr,&o,&mut Cache::new(),&reference("close",Some("SPX"),None,true)).unwrap();assert_eq!(got[20],source.close[19]);
        for tf in ["W","M","Q","Y","5D"] {for expand in ["last","point","first"] {
            let text=format!(r#"{{"s":"close","symbol":"SPX","tf":"{tf}","offset":1,"expand":"{expand}"}}"#);let local=text.replace(r#""symbol":"SPX","#,"");
            let expected=operand_values(&Frames::new(source.clone()),&Others::default(),&mut Cache::new(),&operand(&local)).unwrap();
            let expected=crate::lab::foreign::align(&fr.d.date,&source.date,&expected,crate::lab::foreign::Align::SameDate,false).unwrap();
            assert_series(&operand_values(&fr,&o,&mut Cache::new(),&operand(&text)).unwrap(),&expected);
        }}
    }
    #[test] fn sparse_nan_and_warmup_are_not_filled_by_fixup() {
        let run=business_days(90,true);let mut source=business_days(90,true);remove(&mut source,20);let source_fr=Frames::new(source.clone());let o=others("SPY",false,vec![("SPX",source.clone())]);let fr=Frames::new(run);
        for id in ["sma","fractals"] {
            let raw=series(&source_fr,&mut Cache::new(),&sref(id,Tf::D,0,0)).unwrap();let expected=crate::lab::foreign::align(&fr.d.date,&source.date,&raw,crate::lab::foreign::Align::SameDate,true).unwrap();
            let got=series_in(&fr,&o,&mut Cache::new(),&reference(id,Some("SPX"),None,true)).unwrap();assert_series(&got,&expected);
            assert!(got.iter().any(|v|v.is_nan()));
        }
    }
    #[test] fn cache_identity_covers_interior_source_data_and_paired_inputs() {
        let a=business_days(100,false);let mut b=a.clone();b.close[40]*=1.2;b.high[40]=b.high[40].max(b.close[40]);
        let (fa,fb)=(Frames::new(a.clone()),Frames::new(b.clone()));let mut cache=Cache::new();let r=sref("sma",Tf::D,0,0);
        let x=series(&fa,&mut cache,&r).unwrap();let y=series(&fb,&mut cache,&r).unwrap();assert!(!close_eq(x[40],y[40]));assert_series(&y,&series(&fb,&mut Cache::new(),&r).unwrap());
        let oa=others("R",false,vec![("B",a)]);let ob=others("R",false,vec![("B",b)]);let r=reference("ratio",None,Some("B"),false);
        let x=series_in(&fa,&oa,&mut cache,&r).unwrap();let y=series_in(&fa,&ob,&mut cache,&r).unwrap();assert!(!close_eq(x[40],y[40]));
        let n=cache.len();series_in(&fa,&ob,&mut cache,&r).unwrap();assert_eq!(cache.len(),n,"same paired inputs reuse their raw computation");
    }
    #[test] fn paired_cache_separates_session_availability_policy() {
        let fr=Frames::new(business_days(40,false));let o=others("SPY",false,vec![("CBOE VIX",business_days(40,false))]);
        let later=Others{close_fills:true,run:o.run.clone(),frames:o.frames.clone()};let r=reference("ratio",None,Some("CBOE VIX"),false);let mut cache=Cache::new();
        let same=series_in(&fr,&o,&mut cache,&r).unwrap();let prior=series_in(&fr,&later,&mut cache,&r).unwrap();assert_eq!(same[0],1.);assert!(prior[0].is_nan());assert_series(&prior,&series_in(&fr,&later,&mut Cache::new(),&r).unwrap());
    }
    #[test] fn self_references_and_pair_base_vs_use_the_right_frames() {
        let a=business_days(80,false);let mut b=a.clone();for v in [&mut b.open,&mut b.high,&mut b.low,&mut b.close]{for p in v{*p*=2.;}}
        let fr=Frames::new(a.clone());let o=others("A",false,vec![("B",b)]);let mut cache=Cache::new();
        assert_series(&series_in(&fr,&o,&mut cache,&reference("close",Some("A"),None,false)).unwrap(),&a.close);
        let self_pair=series_in(&fr,&o,&mut cache,&reference("ratio",None,Some("A"),false)).unwrap();assert!(self_pair.iter().all(|v|*v==1.));
        let other=series_in(&fr,&o,&mut cache,&reference("ratio",Some("B"),Some("A"),false)).unwrap();assert!(other.iter().all(|v|*v==2.));
    }
    #[test] fn every_recursive_location_and_both_order_levels_receive_context() {
        let mut a=business_days(40,false);let mut b=a.clone();for v in [&mut a.open,&mut a.high,&mut a.low,&mut a.close]{v.fill(100.);}for v in [&mut b.open,&mut b.high,&mut b.low,&mut b.close]{v.fill(40.);}
        let fr=Frames::new(business_days(40,false));let o=others("R",false,vec![("A",a),("B",b)]);
        let spec=parse_spec(r#"{"v":1,"direction":"both","entry":{"all":[{"all":[{"not":{"lhs":{"s":"close","symbol":"A"},"op":"<","rhs":{"value":50}}},{"lhs":{"count":{"lhs":{"s":"close","symbol":"A"},"op":">","rhs":{"value":50}},"bars":2},"op":">","rhs":{"value":1}}]}],"limit":{"at":{"add":[{"s":"close","symbol":"A"},{"value":1}]}}},"exit":{"any":[]},"short_side":{"entry":{"all":[{"lhs":{"value_when":{"lhs":{"s":"close","symbol":"A"},"op":">","rhs":{"value":0}},"of":{"s":"close","symbol":"B"}},"op":">","rhs":{"value":0}}],"stop":{"at":{"s":"close","symbol":"B","tf":"W","offset":1,"expand":"first"}}},"exit":{"any":[]}}}"#).unwrap();
        let c=compile_with(&spec,&fr,&o,&mut Cache::new()).unwrap();assert!(c.entry[0].holds(3,None));assert_eq!(c.limit_at.as_ref().unwrap()[3],101.);assert_eq!(c.other.as_ref().unwrap().stop_at.as_ref().unwrap()[10],40.);
        assert!(compile(&spec,&fr,&mut Cache::new()).err().unwrap().message().contains("not loaded"));
    }
    #[test] fn future_foreign_prices_do_not_change_earlier_values() {
        let run=business_days(100,false);let source=business_days(100,false);let fr=Frames::new(run);let o=others("R",false,vec![("X",source.clone())]);
        for text in [r#"{"s":"rsi","symbol":"X","p":{"length":2}}"#,r#"{"s":"close","symbol":"X","tf":"W","offset":1,"expand":"first"}"#,r#"{"sum":{"s":"close","symbol":"X"},"bars":3}"#,r#"{"compress":{"s":"close","symbol":"X"},"tf":"5D","mode":"last"}"#] {
            let operand=operand(text);let want=operand_values(&fr,&o,&mut Cache::new(),&operand).unwrap();
            let mut alt=source.clone();for v in [&mut alt.open,&mut alt.high,&mut alt.low,&mut alt.close]{for p in &mut v[51..]{*p*=1.3;}}
            let got=operand_values(&fr,&others("R",false,vec![("X",alt)]),&mut Cache::new(),&operand).unwrap();assert_series(&got[..=50],&want[..=50]);
        }
    }
}

/// AmiBroker "Pad and align all data to reference symbol": the instrument's bars on the reference's calendar, inside the
/// instrument's own first..last dates (no bar before its first is invented). A reference day the instrument lacks repeats the
/// latest earlier close (O=H=L=C, volume 0 where the instrument has volume); a day the reference lacks is dropped. Returns
/// the bars, each bar's source row (None: padded) and the padded / dropped counts.
pub fn pad_align(rows: &[crate::Row], reference: &[crate::Row]) -> (Vec<crate::Row>, Vec<Option<usize>>, usize, usize) {
    let (Some(first), Some(last)) = (rows.first(), rows.last()) else { return (vec![], vec![], 0, 0) };
    let (mut out, mut src) = (Vec::new(), Vec::new());
    for d in reference.iter().map(|r| r.date.as_str()).filter(|d| *d >= first.date.as_str() && *d <= last.date.as_str()) {
        let k = rows.partition_point(|r| r.date.as_str() <= d) - 1; // d >= the first date, so k is a row on or before d
        let p = &rows[k];
        if p.date == d { out.push(p.clone()); src.push(Some(k)); }
        else { out.push(crate::Row { date: d.into(), open: p.close, high: p.close, low: p.close, volume: p.volume.map(|_| 0.), ..p.clone() }); src.push(None); }
    }
    let padded = src.iter().filter(|s| s.is_none()).count();
    let dropped = rows.len() - (src.len() - padded);
    (out, src, padded, dropped)
}

#[cfg(test)]
mod pad_align_tests {
    fn row(d: &str, c: f64) -> crate::Row { crate::Row { ticker: "X".into(), date: d.into(), open: c - 1., high: c + 1., low: c - 2., close: c, volume: Some(100.) } }
    #[test] fn pads_missing_reference_days_with_the_previous_close_and_drops_extra_days() {
        let rows = [row("2024-01-02", 10.), row("2024-01-03", 11.), row("2024-01-06", 12.), row("2024-01-08", 13.)];
        let reference = [row("2024-01-01", 0.), row("2024-01-02", 0.), row("2024-01-04", 0.), row("2024-01-05", 0.), row("2024-01-08", 0.), row("2024-01-09", 0.)];
        let (out, src, padded, dropped) = super::pad_align(&rows, &reference);
        assert_eq!(out.iter().map(|r| r.date.as_str()).collect::<Vec<_>>(), ["2024-01-02", "2024-01-04", "2024-01-05", "2024-01-08"], "within the instrument's own range");
        assert_eq!(src, [Some(0), None, None, Some(3)]);
        assert_eq!((padded, dropped), (2, 2)); // 01-03 and 01-06 are not reference days
        let p = &out[1];
        assert_eq!((p.open, p.high, p.low, p.close, p.volume), (11., 11., 11., 11., Some(0.)), "the latest earlier close, a dropped day's included");
        assert_eq!((out[0].open, out[3].close), (9., 13.), "real bars are untouched");
        let no_volume: Vec<_> = rows.iter().map(|r| crate::Row { volume: None, ..r.clone() }).collect();
        assert_eq!(super::pad_align(&no_volume, &reference).0[1].volume, None);
        assert_eq!(super::pad_align(&rows, &rows).0.len(), 4, "the same calendar is unchanged");
    }
}

#[cfg(test)]
mod formula_tests {
    use super::*;
    use crate::lab::rules::parse_spec;
    use crate::lab::testdata::{close_eq, walk};
    const EMA_X: &str = "Fast = Param(\"Fast\", 9, 2, 50, 1)\nSlow = Param(\"Slow\", 21, 5, 100, 1)\nPlot(EMA(C, Fast) - EMA(C, Slow), \"Osc\")\nPlot(Cross(EMA(C, Fast), EMA(C, Slow)), \"Up\")";
    fn spec(entry: &str) -> Spec {
        parse_spec(&format!(r#"{{"v":1,"entry":{{"all":[{entry}]}},"exit":{{"any":[{{"lhs":{{"s":"bars_held"}},"op":">=","rhs":{{"value":5}}}}]}}}}"#)).unwrap()
    }
    fn formula(extra: &str) -> String { format!(r#"{{"formula":"{}","name":"My Momentum"{extra}}}"#, crate::lab::json::esc(EMA_X)) }
    fn trades(s: &Spec, bars: &Bars) -> Vec<String> {
        let c = compile(s, &Frames::new(bars.clone()), &mut Cache::new()).unwrap();
        crate::lab::simulate::simulate(&c, bars, 0, bars.len() - 1).iter().map(|t| format!("{t:?}")).collect()
    }
    #[test] fn a_formula_rule_trades_like_the_equivalent_built_in_rule() {
        let bars = walk(600, 13, true);
        let built = spec(r#"{"lhs":{"s":"ema","p":{"length":12}},"op":"crosses_above","rhs":{"s":"ema","p":{"length":30}}}"#);
        let by_cross = spec(&format!(r#"{{"lhs":{},"op":">","rhs":{{"value":0.5}}}}"#, formula(r#","p":{"Fast":12,"Slow":30},"out":"Up""#)));
        let by_osc = spec(&format!(r#"{{"lhs":{},"op":"crosses_above","rhs":{{"value":0}}}}"#, formula(r#","p":{"Fast":12,"Slow":30}"#)));
        let want = trades(&built, &bars);
        assert!(want.len() > 3, "the test needs trades");
        assert_eq!(trades(&by_cross, &bars), want);
        assert_eq!(trades(&by_osc, &bars), want);
        assert_eq!(describe_node(&by_cross.entry[0]), "My Momentum(Fast=12,Slow=30).Up > 0.5");
        assert_eq!(describe_node(&spec(&format!(r#"{{"lhs":{},"op":">","rhs":{{"value":0}}}}"#, formula(r#","tf":"W","offset":1"#))).entry[0]), "My Momentum.Osc@W[1] > 0");
    }
    #[test] fn formula_operands_resample_offset_and_stay_causal() {
        let full = super::tests::business_days(400, true);
        let s = spec(&format!(r#"{{"lhs":{},"op":">","rhs":{{"value":0}}}}"#, formula(r#","tf":"W","offset":1"#)));
        let Node::Cmp(c) = &s.entry[0] else { panic!() };
        let want = operand_values(&Frames::new(full.clone()), &Others::default(), &mut Cache::new(), &c.lhs).unwrap();
        assert!(want.iter().filter(|v| v.is_finite()).count() > 200);
        for cut in (100..400).step_by(23) {
            let got = operand_values(&Frames::new(crate::lab::testdata::cut(&full, cut)), &Others::default(), &mut Cache::new(), &c.lhs).unwrap();
            for i in 0..cut - 1 { assert!(close_eq(got[i], want[i]), "bar {i} changes when bars after {cut} exist"); }
        }
    }
    #[test] fn formula_cache_is_keyed_by_text_not_name() {
        let fr = Frames::new(walk(100, 3, false));
        let a = spec(r#"{"lhs":{"formula":"Plot(C, \"X\")","name":"Same"},"op":">","rhs":{"value":0}}"#);
        let b = spec(r#"{"lhs":{"formula":"Plot(-C, \"X\")","name":"Same"},"op":">","rhs":{"value":0}}"#);
        let mut cache = Cache::new();
        let val = |s: &Spec, cache: &mut Cache| operand_values(&fr, &Others::default(), cache, &s.entry[0].cmp().unwrap().lhs).unwrap();
        let (x, y) = (val(&a, &mut cache), val(&b, &mut cache));
        assert_eq!(x[5], -y[5]);
    }
    #[test] fn formula_run_errors_keep_their_position_and_type() {
        let s = spec(r#"{"lhs":{"formula":"x = 1\nPlot(V, \"X\")"},"op":">","rhs":{"value":0}}"#);
        let e = compile(&s, &Frames::new(walk(50, 1, false)), &mut Cache::new()).err().unwrap();
        assert_eq!((e.status(), e.message()), ("needs_data", "line 2, col 6: volume: volume is missing on some admitted bars of this instrument"));
    }
}
