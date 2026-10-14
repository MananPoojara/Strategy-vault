//! Rule spec v1 (indicators lab spec §4): parse, validate against the catalogue, expand optimize grids.
//! Reserved fields return `Unsupported`; malformed input returns `Invalid` naming the field.
use super::catalogue::{self, Params};
use super::json::{self, Json};
use super::resample::Tf;

pub const LABEL: &str = "User-defined indicator strategy · simulated · not a book method";
pub const MAX_CONDITIONS: usize = 12;
pub const MAX_SPEC_BYTES: usize = 8192;
pub const MAX_OFFSET: usize = 1000;
pub const PRICE_SOURCES: &[&str] = &["open", "high", "low", "close", "hl2", "hlc3", "ohlc4", "volume"];
pub const OPERATORS: &[&str] = &["crosses_above", "crosses_below", ">", "<", ">=", "<="];

#[derive(Clone, Debug, PartialEq)]
pub enum LabError { Invalid(String), Unsupported(String), NeedsData(String) }
impl LabError {
    pub fn status(&self) -> &'static str {
        match self { Self::Invalid(_) => "invalid", Self::Unsupported(_) => "unsupported", Self::NeedsData(_) => "needs_data" }
    }
    pub fn message(&self) -> &str { match self { Self::Invalid(m) | Self::Unsupported(m) | Self::NeedsData(m) => m } }
    pub fn json(&self) -> String {
        format!("{{\"status\":\"{}\",\"error\":\"{}\",\"label\":\"{LABEL}\"}}", self.status(), json::esc(self.message()))
    }
}
/// Engine helpers report data gaps as "needs_data: …"; any other engine error is an invalid request.
impl From<String> for LabError {
    fn from(e: String) -> Self {
        match e.strip_prefix("needs_data:") { Some(m) => Self::NeedsData(m.trim().into()), None => Self::Invalid(e) }
    }
}
fn inv(m: impl Into<String>) -> LabError { LabError::Invalid(m.into()) }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Op { CrossesAbove, CrossesBelow, Gt, Lt, Ge, Le }
#[derive(Clone, Debug, PartialEq)]
pub struct SeriesRef { pub id: String, pub params: Params, pub tf: Tf, pub out: usize, pub offset: usize }
#[derive(Clone, Debug, PartialEq)]
pub enum Operand { Series(SeriesRef), Value(f64), BarsHeld }
#[derive(Clone, Debug, PartialEq)]
pub struct Cond { pub lhs: Operand, pub op: Op, pub rhs: Operand }
/// Where an optimize path points: entry/exit list, condition index, side, and parameter key (None = `value`).
#[derive(Clone, Debug, PartialEq)]
pub struct Target { pub exit: bool, pub cond: usize, pub rhs: bool, pub param: Option<String>, pub slot: Option<Slot> }
/// Market orders fill at the next bar's open (default) or at the signal bar's own close (the book convention).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fill { NextOpen, Close }
/// A price distance: percent of a reference price, or a multiple of ATR(length) read at a bar already closed.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Dist { Pct(f64), Atr { mult: f64, length: usize } }
/// The price-order settings an optimize path can address (`entry.limit.pct`, `exit.stop_loss.atr`, ...).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Slot { Limit, StopLoss, Target, Trailing }
#[derive(Clone, Debug, PartialEq)]
pub struct Exec { pub short: bool, pub fill: Fill, pub exit_fill: Fill, pub limit: Option<Dist>, pub stop_loss: Option<Dist>, pub target: Option<Dist>, pub trailing: Option<Dist> }
impl Default for Exec {
    fn default() -> Self { Exec { short: false, fill: Fill::NextOpen, exit_fill: Fill::NextOpen, limit: None, stop_loss: None, target: None, trailing: None } }
}
impl Exec {
    pub fn slot(&self, s: Slot) -> Option<Dist> { match s { Slot::Limit => self.limit, Slot::StopLoss => self.stop_loss, Slot::Target => self.target, Slot::Trailing => self.trailing } }
    fn slot_mut(&mut self, s: Slot) -> &mut Option<Dist> { match s { Slot::Limit => &mut self.limit, Slot::StopLoss => &mut self.stop_loss, Slot::Target => &mut self.target, Slot::Trailing => &mut self.trailing } }
}
/// Lineage of a customized book template (`origin` in the spec). Checked against the template catalogue by the API.
#[derive(Clone, Debug, PartialEq)]
pub struct Origin { pub book: String, pub method: String, pub version: u32 }
/// Slot, spec path, largest pct. A limit may sit at the signal close (0%); stop-loss, target and trailing need pct > 0.
const SLOTS: [(Slot, &str, f64); 4] = [(Slot::Limit, "entry.limit", 50.), (Slot::StopLoss, "exit.stop_loss", 99.), (Slot::Target, "exit.target", 1000.), (Slot::Trailing, "exit.trailing", 99.)];
pub const MAX_ATR_MULT: f64 = 50.;
pub const MAX_ATR_LENGTH: usize = 500;
#[derive(Clone, Debug, PartialEq)]
pub struct Dim { pub path: String, pub target: Target, pub values: Vec<f64> }
#[derive(Clone, Debug, PartialEq)]
pub struct Spec { pub entry: Vec<Cond>, pub exit: Vec<Cond>, pub cost_bps: Option<f64>, pub optimize: Vec<Dim>, pub exec: Exec, pub origin: Option<Origin> }

pub fn parse_spec(text: &str) -> Result<Spec, LabError> {
    if text.len() > MAX_SPEC_BYTES { return Err(inv(format!("spec is {} bytes; the limit is {MAX_SPEC_BYTES}", text.len()))); }
    let root = json::parse(text).map_err(|e| inv(format!("spec is not valid JSON: {e}")))?;
    fields(&root, "spec", &["v", "direction", "origin", "entry", "exit", "execution", "optimize"], &["short", "portfolio", "scale_in"])?;
    if root.get("v").and_then(Json::as_f64) != Some(1.) { return Err(inv("v must be 1")); }
    let mut exec = Exec::default();
    match root.get("direction") {
        None => {}
        Some(Json::Str(d)) if d == "long" => {}
        Some(Json::Str(d)) if d == "short" => exec.short = true,
        _ => return Err(inv("direction must be \"long\" or \"short\"")),
    }
    let origin = root.get("origin").map(origin).transpose()?;
    let entry_obj = root.get("entry").ok_or_else(|| inv("entry is required"))?;
    fields(entry_obj, "entry", &["all", "limit"], &["stop", "scale_in"])?;
    let exit_obj = root.get("exit").ok_or_else(|| inv("exit is required"))?;
    fields(exit_obj, "exit", &["any", "stop_loss", "target", "trailing"], &[])?;
    for (slot, at, hi) in SLOTS {
        let obj = if slot == Slot::Limit { entry_obj } else { exit_obj };
        if let Some(v) = obj.get(at.split_once('.').unwrap().1) { *exec.slot_mut(slot) = Some(dist(v, at, hi)?); }
    }
    let price_exit = exec.stop_loss.is_some() || exec.target.is_some() || exec.trailing.is_some();
    let entry = conds(entry_obj.get("all"), "entry.all", false, false)?;
    let exit = conds(exit_obj.get("any"), "exit.any", true, price_exit)?;
    if entry.len() + exit.len() > MAX_CONDITIONS {
        return Err(inv(format!("{} conditions; the limit is {MAX_CONDITIONS}", entry.len() + exit.len())));
    }
    let mut cost_bps = None;
    if let Some(ex) = root.get("execution") {
        fields(ex, "execution", &["fill", "exit_fill", "cost_bps"], &["limit", "stop"])?;
        if let Some(f) = ex.get("fill") { exec.fill = fill(f, "execution.fill")?; exec.exit_fill = exec.fill; }
        if let Some(f) = ex.get("exit_fill") { exec.exit_fill = fill(f, "execution.exit_fill")?; }
        if exec.limit.is_some() && ex.get("fill").is_some() {
            return Err(inv("execution.fill applies to market entries; entry.limit already fills on the bar after the signal, so remove execution.fill"));
        }
        if let Some(c) = ex.get("cost_bps") {
            cost_bps = Some(c.as_f64().filter(|c| (0. ..=5000.).contains(c)).ok_or_else(|| inv("execution.cost_bps must be a number from 0 to 5000"))?);
        }
    }
    let mut spec = Spec { entry, exit, cost_bps, optimize: vec![], exec, origin };
    if let Some(o) = root.get("optimize") { spec.optimize = dims(&spec, o)?; }
    Ok(spec)
}

fn fields(v: &Json, at: &str, allowed: &[&str], reserved: &[&str]) -> Result<(), LabError> {
    let Json::Obj(kv) = v else { return Err(inv(format!("{at} must be an object"))) };
    for (k, _) in kv {
        if reserved.contains(&k.as_str()) { return Err(LabError::Unsupported(format!("{at}.{k} is reserved for a later version; v1 does not simulate it"))); }
        if !allowed.contains(&k.as_str()) { return Err(inv(format!("unknown field '{k}' in {at}"))); }
    }
    Ok(())
}

fn fill(v: &Json, at: &str) -> Result<Fill, LabError> {
    match v.as_str() {
        Some("next_open") => Ok(Fill::NextOpen),
        Some("close") => Ok(Fill::Close),
        Some(f @ ("limit" | "stop")) => Err(LabError::Unsupported(format!("{at} '{f}' is reserved; use entry.limit for limit entries (stop entries are a later version)"))),
        _ => Err(inv(format!("{at} must be \"next_open\" or \"close\""))),
    }
}
/// `{"pct": p}` or `{"atr": m, "atr_length": n}` (n defaults to 14).
fn dist(v: &Json, at: &str, hi: f64) -> Result<Dist, LabError> {
    fields(v, at, &["pct", "atr", "atr_length"], &[])?;
    match (v.get("pct"), v.get("atr")) {
        (Some(p), None) if v.get("atr_length").is_none() => {
            let min = if at == "entry.limit" { "from 0" } else { "above 0" };
            p.as_f64().filter(|p| *p <= hi && (*p > 0. || (*p == 0. && at == "entry.limit"))).map(Dist::Pct)
                .ok_or_else(|| inv(format!("{at}.pct must be a number {min} and at most {hi}")))
        }
        (None, Some(m)) => {
            let mult = m.as_f64().filter(|m| *m > 0. && *m <= MAX_ATR_MULT).ok_or_else(|| inv(format!("{at}.atr must be a number above 0 and at most {MAX_ATR_MULT}")))?;
            let length = match v.get("atr_length") {
                None => 14,
                Some(n) => n.as_f64().filter(|n| n.fract() == 0. && *n >= 1. && *n <= MAX_ATR_LENGTH as f64)
                    .ok_or_else(|| inv(format!("{at}.atr_length must be a whole number from 1 to {MAX_ATR_LENGTH}")))? as usize,
            };
            Ok(Dist::Atr { mult, length })
        }
        _ => Err(inv(format!("{at} needs exactly one of pct or atr (atr_length goes with atr)"))),
    }
}
fn origin(v: &Json) -> Result<Origin, LabError> {
    fields(v, "origin", &["book", "method", "template_version"], &[])?;
    let id = |k: &str| v.get(k).and_then(Json::as_str).filter(|s| !s.is_empty() && s.len() <= 80 && s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_'))
        .map(String::from).ok_or_else(|| inv(format!("origin.{k} must be a catalogue id (letters, digits, _)")));
    let (book, method) = (id("book")?, id("method")?);
    let version = v.get("template_version").and_then(Json::as_f64).filter(|n| n.fract() == 0. && *n >= 1. && *n <= 1000.)
        .ok_or_else(|| inv("origin.template_version must be a whole number from 1 to 1000"))? as u32;
    Ok(Origin { book, method, version })
}

fn conds(v: Option<&Json>, at: &str, exit: bool, may_be_empty: bool) -> Result<Vec<Cond>, LabError> {
    let Some(Json::Arr(items)) = v else { return Err(inv(format!("{at} must be a list of conditions"))) };
    if items.is_empty() && !may_be_empty { return Err(inv(format!("{at} needs at least one condition{}", if exit { " (or a stop_loss, target or trailing exit)" } else { "" }))); }
    if items.len() > MAX_CONDITIONS { return Err(inv(format!("{at} has {} conditions; the limit is {MAX_CONDITIONS}", items.len()))); }
    items.iter().enumerate().map(|(i, c)| cond(c, &format!("{at}[{i}]"), exit)).collect()
}

fn cond(v: &Json, at: &str, exit: bool) -> Result<Cond, LabError> {
    fields(v, at, &["lhs", "op", "rhs"], &[])?;
    let op = match v.get("op").and_then(Json::as_str) {
        Some("crosses_above") => Op::CrossesAbove,
        Some("crosses_below") => Op::CrossesBelow,
        Some(">") => Op::Gt,
        Some("<") => Op::Lt,
        Some(">=") => Op::Ge,
        Some("<=") => Op::Le,
        _ => return Err(inv(format!("{at}.op must be one of {}", OPERATORS.join(", ")))),
    };
    let lhs = operand(v.get("lhs"), &format!("{at}.lhs"), exit)?;
    let rhs = operand(v.get("rhs"), &format!("{at}.rhs"), exit)?;
    if matches!((&lhs, &rhs), (Operand::Value(_), Operand::Value(_))) { return Err(inv(format!("{at} compares two numbers; use at least one series"))); }
    Ok(Cond { lhs, op, rhs })
}

fn operand(v: Option<&Json>, at: &str, exit: bool) -> Result<Operand, LabError> {
    let v = v.ok_or_else(|| inv(format!("{at} is required")))?;
    if v.get("value").is_some() {
        fields(v, at, &["value"], &[])?;
        return v.get("value").and_then(Json::as_f64).map(Operand::Value).ok_or_else(|| inv(format!("{at}.value must be a number")));
    }
    fields(v, at, &["s", "p", "tf", "out", "offset"], &[])?;
    let id = v.get("s").and_then(Json::as_str).ok_or_else(|| inv(format!("{at}.s must name a series (or give a value)")))?;
    if id == "bars_held" {
        if !exit { return Err(inv(format!("{at}: bars_held is only available in exit conditions"))); }
        if ["p", "tf", "out", "offset"].iter().any(|k| v.get(k).is_some()) { return Err(inv(format!("{at}: bars_held takes no p, tf, out or offset"))); }
        return Ok(Operand::BarsHeld);
    }
    let tf = match v.get("tf") { None => Tf::D, Some(t) => t.as_str().and_then(Tf::parse).ok_or_else(|| inv(format!("{at}.tf must be D, W or M")))? };
    let offset = match v.get("offset") {
        None => 0,
        Some(o) => o.as_f64().filter(|n| n.fract() == 0. && *n >= 0. && *n <= MAX_OFFSET as f64)
            .ok_or_else(|| inv(format!("{at}.offset must be a whole number from 0 to {MAX_OFFSET} (past bars only)")))? as usize,
    };
    let mut given = Params::new();
    if let Some(p) = v.get("p") {
        let Json::Obj(kv) = p else { return Err(inv(format!("{at}.p must be an object"))) };
        for (k, x) in kv { given.insert(k.clone(), x.as_f64().ok_or_else(|| inv(format!("{at}.p.{k} must be a number")))?); }
    }
    let out = match v.get("out") { None => None, Some(o) => Some(o.as_str().ok_or_else(|| inv(format!("{at}.out must be a string")))?) };
    series_ref(at, id, &given, tf, out, offset).map(Operand::Series)
}

/// Validates one series operand against the catalogue (shared by rule specs and /api/lab/series).
pub fn series_ref(at: &str, id: &str, given: &Params, tf: Tf, out: Option<&str>, offset: usize) -> Result<SeriesRef, LabError> {
    if PRICE_SOURCES.contains(&id) {
        if !given.is_empty() || out.is_some() { return Err(inv(format!("{at}: price source '{id}' takes no p or out"))); }
        return Ok(SeriesRef { id: id.into(), params: Params::new(), tf, out: 0, offset });
    }
    let def = catalogue::get(id).ok_or_else(|| inv(format!("{at}.s: unknown series '{id}'")))?;
    if def.needs.intraday { return Err(LabError::NeedsData(format!("{at}: {} needs intraday data, which is not admitted", def.name))); }
    if def.compute.is_none() { return Err(LabError::Unsupported(format!("{at}: {} is listed but not computed yet", def.name))); }
    let params = catalogue::resolve_params(&def, given).map_err(|e| inv(format!("{at}.p: {e}")))?;
    let out = match out {
        None => 0,
        Some(o) => def.outputs.iter().position(|x| *x == o).ok_or_else(|| inv(format!("{at}.out: {} has outputs {}", def.id, def.outputs.join(", "))))?,
    };
    Ok(SeriesRef { id: id.into(), params, tf, out, offset })
}

fn dims(spec: &Spec, v: &Json) -> Result<Vec<Dim>, LabError> {
    let Json::Arr(items) = v else { return Err(inv("optimize must be a list")) };
    let mut out: Vec<Dim> = Vec::new();
    let mut combos = 1usize;
    for (i, it) in items.iter().enumerate() {
        let at = format!("optimize[{i}]");
        fields(it, &at, &["path", "from", "to", "step"], &[])?;
        let path = it.get("path").and_then(Json::as_str).ok_or_else(|| inv(format!("{at}.path must be a string")))?;
        let target = target(spec, path).map_err(|e| inv(format!("{at}.path '{path}': {e}")))?;
        if out.iter().any(|d| d.target == target) { return Err(inv(format!("{at}.path '{path}' is listed twice"))); }
        let num = |k: &str| it.get(k).and_then(Json::as_f64).ok_or_else(|| inv(format!("{at}.{k} must be a number")));
        // Same grid rule as the book optimizer (main.rs grid(): ≤ 1000 values, 1e-9 rounding).
        let (from, to, step) = (num("from")?, num("to")?, num("step")?);
        // crate::grid overflows on huge or tiny-step ranges, so bound them before calling it.
        if ![from, to, step].iter().all(|x| x.is_finite()) || step <= 0. || to < from || (to - from) / step > 1000. {
            return Err(inv(format!("{at}.path '{path}': bad range {from}:{to}:{step}; need finite from <= to, step > 0 and at most 1000 values")));
        }
        let values = crate::grid(&format!("{from}:{to}:{step}")).map_err(|e| inv(format!("{at}: {e}")))?;
        if values.is_empty() || values.iter().any(|x| !x.is_finite()) { return Err(inv(format!("{at}.path '{path}': bad range, no values"))); }
        for &x in &values { check_value(spec, &target, x).map_err(|e| inv(format!("{at}: {e}")))?; }
        combos = combos.saturating_mul(values.len());
        if combos > crate::MAX_COMBOS { return Err(inv(format!("optimize gives more than {} combinations; narrow a range or raise a step", crate::MAX_COMBOS))); }
        out.push(Dim { path: path.into(), target, values });
    }
    Ok(out)
}

fn target(spec: &Spec, path: &str) -> Result<Target, String> {
    if let Some((slot, at, _)) = SLOTS.iter().find(|(_, at, _)| path.starts_with(&format!("{at}."))) {
        let field = &path[at.len() + 1..];
        return match (spec.exec.slot(*slot), field) {
            (Some(Dist::Pct(_)), "pct") | (Some(Dist::Atr { .. }), "atr") => Ok(Target { exit: false, cond: 0, rhs: false, param: None, slot: Some(*slot) }),
            (None, _) => Err(format!("the spec has no {at}")),
            _ => Err(format!("{at} is set by {}; optimize that field", if matches!(spec.exec.slot(*slot), Some(Dist::Pct(_))) { "pct" } else { "atr" })),
        };
    }
    let (exit, rest) = match (path.strip_prefix("entry.all["), path.strip_prefix("exit.any[")) {
        (Some(r), _) => (false, r),
        (_, Some(r)) => (true, r),
        _ => return Err("must start with entry.all[i], exit.any[i], entry.limit, exit.stop_loss, exit.target or exit.trailing".into()),
    };
    let (idx, rest) = rest.split_once("].").ok_or("expected ].lhs or ].rhs")?;
    let cond: usize = idx.parse().map_err(|_| "condition index must be a whole number")?;
    let c = (if exit { &spec.exit } else { &spec.entry }).get(cond).ok_or("no condition at that index")?;
    let (rhs, field) = match (rest.strip_prefix("lhs."), rest.strip_prefix("rhs.")) {
        (Some(f), _) => (false, f),
        (_, Some(f)) => (true, f),
        _ => return Err("expected lhs or rhs".into()),
    };
    match (if rhs { &c.rhs } else { &c.lhs }, field) {
        (Operand::Value(_), "value") => Ok(Target { exit, cond, rhs, param: None, slot: None }),
        (Operand::Series(s), f) if f.starts_with("p.") => {
            let def = catalogue::get(&s.id).ok_or("price sources have no parameters")?;
            let key = &f[2..];
            if !def.params.iter().any(|p| p.key == key) { return Err(format!("{} has no parameter '{key}'", def.id)); }
            Ok(Target { exit, cond, rhs, param: Some(key.into()), slot: None })
        }
        _ => Err("must end in .value (a number operand) or .p.<key> (an indicator parameter)".into()),
    }
}

fn operand_mut<'a>(s: &'a mut Spec, t: &Target) -> &'a mut Operand {
    let c = if t.exit { &mut s.exit[t.cond] } else { &mut s.entry[t.cond] };
    if t.rhs { &mut c.rhs } else { &mut c.lhs }
}
fn set(s: &mut Spec, t: &Target, x: f64) {
    if let Some(slot) = t.slot {
        match s.exec.slot_mut(slot) { Some(Dist::Pct(p)) => *p = x, Some(Dist::Atr { mult, .. }) => *mult = x, None => {} }
        return;
    }
    match (operand_mut(s, t), &t.param) {
        (Operand::Value(v), None) => *v = x,
        (Operand::Series(r), Some(k)) => { r.params.insert(k.clone(), x); }
        _ => {}
    }
}
fn check_value(spec: &Spec, t: &Target, x: f64) -> Result<(), String> {
    if let Some(slot) = t.slot {
        let (_, at, hi) = SLOTS.iter().find(|(s, _, _)| *s == slot).unwrap();
        let ok = match spec.exec.slot(slot) { Some(Dist::Pct(_)) => x <= *hi && (x > 0. || (x == 0. && slot == Slot::Limit)), _ => x > 0. && x <= MAX_ATR_MULT };
        return if ok { Ok(()) } else { Err(format!("{at} value {x} is out of range")) };
    }
    let mut s = spec.clone();
    set(&mut s, t, x);
    match operand_mut(&mut s, t) {
        Operand::Series(r) => catalogue::resolve_params(&catalogue::get(&r.id).ok_or("unknown series")?, &r.params).map(|_| ()),
        _ => Ok(()),
    }
}
/// The spec's own value at an optimize target (the "base" setting the ranking prefers on ties).
pub fn current(spec: &Spec, t: &Target) -> f64 {
    if let Some(slot) = t.slot { return match spec.exec.slot(slot) { Some(Dist::Pct(p)) => p, Some(Dist::Atr { mult, .. }) => mult, None => f64::NAN }; }
    let mut s = spec.clone();
    match (operand_mut(&mut s, t), &t.param) {
        (Operand::Value(v), None) => *v,
        (Operand::Series(r), Some(k)) => r.params.get(k).copied().unwrap_or(f64::NAN),
        _ => f64::NAN,
    }
}
/// The spec with one grid combination applied (values in `dims` order; validated when the spec was parsed).
pub fn apply(spec: &Spec, dims: &[Dim], values: &[f64]) -> Spec {
    let mut s = spec.clone();
    for (d, &x) in dims.iter().zip(values) { set(&mut s, &d.target, x); }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    const EXAMPLE: &str = r#"{ "v": 1, "direction": "long",
      "entry": { "all": [
        { "lhs": {"s":"rsi","p":{"length":2},"tf":"D"}, "op":"crosses_below", "rhs": {"value":10} },
        { "lhs": {"s":"close","tf":"D"}, "op":">", "rhs": {"s":"sma","p":{"length":40},"tf":"W"} } ] },
      "exit": { "any": [
        { "lhs": {"s":"rsi","p":{"length":2},"tf":"D"}, "op":"crosses_above", "rhs": {"value":70} },
        { "lhs": {"s":"bars_held"}, "op":">=", "rhs": {"value":10} } ] },
      "execution": { "fill": "next_open", "cost_bps": 12 },
      "optimize": [ {"path":"entry.all[0].lhs.p.length","from":2,"to":6,"step":1} ] }"#;
    fn with(old: &str, new: &str) -> String { assert!(EXAMPLE.contains(old), "{old}"); EXAMPLE.replacen(old, new, 1) }
    fn err(text: &str) -> LabError { parse_spec(text).unwrap_err() }

    #[test] fn spec_example_round_trips() {
        let s = parse_spec(EXAMPLE).unwrap();
        assert_eq!((s.entry.len(), s.exit.len(), s.cost_bps), (2, 2, Some(12.)));
        let Operand::Series(r) = &s.entry[1].rhs else { panic!() };
        assert_eq!((r.id.as_str(), r.tf, r.params["length"]), ("sma", Tf::W, 40.));
        assert_eq!(s.exit[1].lhs, Operand::BarsHeld);
        assert_eq!(s.optimize[0].values, vec![2., 3., 4., 5., 6.]);
        assert_eq!(current(&s, &s.optimize[0].target), 2.);
        let t = apply(&s, &s.optimize, &[5.]);
        let Operand::Series(r) = &t.entry[0].lhs else { panic!() };
        assert_eq!(r.params["length"], 5.);
    }
    #[test] fn defaults_fill_params_tf_and_output() {
        let s = parse_spec(&with(r#"{"s":"close","tf":"D"}"#, r#"{"s":"macd","out":"signal"}"#)).unwrap();
        let Operand::Series(r) = &s.entry[1].lhs else { panic!() };
        assert_eq!((r.tf, r.out, r.params["fast"], r.params["slow"]), (Tf::D, 1, 12., 26.));
    }
    #[test] fn malformed_input_is_invalid_and_names_the_field() {
        for (text, needle) in [
            (with(r#""v": 1"#, r#""v": 2"#), "v must be 1"),
            (with(r#""op":"crosses_below""#, r#""op":"below""#), "entry.all[0].op"),
            (with(r#""tf":"W""#, r#""tf":"H""#), "entry.all[1].rhs.tf"),
            (with(r#"{"length":40}"#, r#"{"length":0}"#), "entry.all[1].rhs.p: sma: length"),
            (with(r#"{"length":40}"#, r#"{"len":40}"#), "unknown parameter 'len'"),
            (with(r#""s":"sma""#, r#""s":"smaa""#), "unknown series 'smaa'"),
            (with(r#""rhs": {"value":10}"#, r#""rhs": {"value":10,"offset":1}"#), "unknown field 'offset' in entry.all[0].rhs"),
            (with(r#"{"s":"close","tf":"D"}"#, r#"{"s":"close","offset":-1}"#), "entry.all[1].lhs.offset"),
            (with(r#"{"s":"close","tf":"D"}"#, r#"{"s":"close","offset":1.5}"#), "entry.all[1].lhs.offset"),
            (with(r#"{"s":"close","tf":"D"}"#, r#"{"s":"bars_held"}"#), "only available in exit"),
            (with(r#"{"s":"rsi","p":{"length":2},"tf":"D"}"#, r#"{"value":3}"#), "compares two numbers"),
            (with(r#"{"s":"close","tf":"D"}"#, r#"{"s":"macd","out":"sig"}"#), "macd has outputs macd, signal, hist"),
            (with(r#""entry""#, r#""entri""#), "unknown field 'entri' in spec"),
            (with(r#""all": ["#, r#""all": [], "x": ["#), "unknown field 'x' in entry"),
            ("{\"v\":1}".to_string(), "entry is required"),
            ("not json".to_string(), "not valid JSON"),
            (format!("{{\"v\":1,\"pad\":\"{}\"}}", "x".repeat(MAX_SPEC_BYTES)), "the limit is 8192"),
            (with("\"from\":2,\"to\":6", "\"from\":0,\"to\":6"), "optimize[0]: rsi: length must be between"),
            (with("entry.all[0].lhs.p.length", "entry.all[0].lhs.p.bogus"), "rsi has no parameter 'bogus'"),
            (with("entry.all[0].lhs.p.length", "entry.all[1].lhs.p.length"), "price sources have no parameters"),
            (with("entry.all[0].lhs.p.length", "entry.all[9].lhs.p.length"), "no condition at that index"),
            (with("\"step\":1", "\"step\":0"), "bad range"),
        ] {
            let e = err(&text);
            assert_eq!(e.status(), "invalid", "{text}: {e:?}");
            assert!(e.message().contains(needle), "{needle:?} not in {:?}", e.message());
        }
    }
    #[test] fn reserved_fields_are_unsupported() {
        for (text, needle) in [
            (with(r#""v": 1,"#, r#""v": 1, "portfolio": {"max_open": 3},"#), "spec.portfolio"),
            (with(r#""all": ["#, r#""stop": {"pct": 1}, "all": ["#), "entry.stop"),
            (with(r#""fill": "next_open""#, r#""fill": "limit""#), "execution.fill 'limit'"),
            (with(r#""fill": "next_open""#, r#""fill": "stop""#), "execution.fill 'stop'"),
            (with(r#""v": 1,"#, r#""v": 1, "scale_in": [],"#), "spec.scale_in"),
        ] {
            let e = err(&text);
            assert_eq!(e.status(), "unsupported", "{text}: {e:?}");
            assert!(e.message().contains(needle), "{needle:?} not in {:?}", e.message());
        }
    }
    #[test] fn customize_fields_parse() {
        let s = parse_spec(&with(r#""direction": "long""#, r#""direction": "short", "origin": {"book":"stc9","method":"rsi2_4","template_version":1}"#)).unwrap();
        assert!(s.exec.short);
        assert_eq!(s.origin, Some(Origin { book: "stc9".into(), method: "rsi2_4".into(), version: 1 }));
        assert_eq!((s.exec.fill, s.exec.exit_fill, s.exec.limit, s.exec.stop_loss), (Fill::NextOpen, Fill::NextOpen, None, None));
        let s = parse_spec(&with(r#""fill": "next_open""#, r#""fill": "close""#)).unwrap();
        assert_eq!((s.exec.fill, s.exec.exit_fill), (Fill::Close, Fill::Close));
        let s = parse_spec(&with(r#""fill": "next_open""#, r#""fill": "close", "exit_fill": "next_open""#)).unwrap();
        assert_eq!((s.exec.fill, s.exec.exit_fill), (Fill::Close, Fill::NextOpen));
        let s = parse_spec(&with(r#""all": ["#, r#""limit": {"pct": 3}, "all": ["#).replace(r#""fill": "next_open", "#, "")).unwrap();
        assert_eq!(s.exec.limit, Some(Dist::Pct(3.)));
        let s = parse_spec(&with(r#""any": ["#, r#""stop_loss": {"atr": 2}, "target": {"pct": 10}, "trailing": {"atr": 3, "atr_length": 20}, "any": ["#)).unwrap();
        assert_eq!((s.exec.stop_loss, s.exec.target, s.exec.trailing), (Some(Dist::Atr { mult: 2., length: 14 }), Some(Dist::Pct(10.)), Some(Dist::Atr { mult: 3., length: 20 })));
        // A price exit lets exit.any be empty; without one it still needs a condition.
        let only_target = r#"{"v":1,"entry":{"all":[{"lhs":{"s":"close"},"op":">","rhs":{"value":1}}]},"exit":{"any":[],"target":{"pct":10}}}"#;
        assert!(parse_spec(only_target).unwrap().exit.is_empty());
        assert!(err(&only_target.replace(r#","target":{"pct":10}"#, "")).message().contains("or a stop_loss, target or trailing exit"));
    }
    #[test] fn customize_fields_are_bounded_and_named() {
        for (text, needle) in [
            (with(r#""direction": "long""#, r#""direction": "both""#), "direction must be"),
            (with(r#""fill": "next_open""#, r#""fill": "open""#), "execution.fill must be"),
            (with(r#""fill": "next_open""#, r#""exit_fill": "later""#), "execution.exit_fill must be"),
            (with(r#""all": ["#, r#""limit": {"pct": 3}, "all": ["#), "remove execution.fill"),
            (with(r#""all": ["#, r#""limit": {"pct": 51}, "all": ["#).replace(r#""fill": "next_open", "#, ""), "entry.limit.pct must be a number from 0 and at most 50"),
            (with(r#""any": ["#, r#""stop_loss": {"pct": 0}, "any": ["#), "exit.stop_loss.pct must be a number above 0"),
            (with(r#""any": ["#, r#""trailing": {"pct": 100}, "any": ["#), "exit.trailing.pct"),
            (with(r#""any": ["#, r#""target": {"pct": 5, "atr": 1}, "any": ["#), "exactly one of pct or atr"),
            (with(r#""any": ["#, r#""target": {"pct": 5, "atr_length": 3}, "any": ["#), "exactly one of pct or atr"),
            (with(r#""any": ["#, r#""stop_loss": {"atr": 0}, "any": ["#), "exit.stop_loss.atr must be"),
            (with(r#""any": ["#, r#""stop_loss": {"atr": 1, "atr_length": 501}, "any": ["#), "atr_length must be a whole number"),
            (with(r#""any": ["#, r#""stop_loss": {"atr": 1, "len": 5}, "any": ["#), "unknown field 'len' in exit.stop_loss"),
            (with(r#""v": 1,"#, r#""v": 1, "origin": {"book":"stc9","method":"rsi 2"},"#), "origin.method"),
            (with(r#""v": 1,"#, r#""v": 1, "origin": {"book":"stc9","method":"rsi2_4","template_version":0},"#), "origin.template_version"),
            (with(r#""v": 1,"#, r#""v": 1, "origin": {"book":"stc9","method":"rsi2_4","template_version":1,"x":1},"#), "unknown field 'x' in origin"),
        ] {
            let e = err(&text);
            assert_eq!(e.status(), "invalid", "{text}: {e:?}");
            assert!(e.message().contains(needle), "{needle:?} not in {:?}", e.message());
        }
    }
    #[test] fn optimize_reaches_price_orders() {
        let text = with(r#""any": ["#, r#""stop_loss": {"pct": 5}, "trailing": {"atr": 3}, "any": ["#)
            .replace(r#"[ {"path":"entry.all[0].lhs.p.length","from":2,"to":6,"step":1} ]"#, r#"[ {"path":"exit.stop_loss.pct","from":2,"to":8,"step":2}, {"path":"exit.trailing.atr","from":1,"to":3,"step":1} ]"#);
        let s = parse_spec(&text).unwrap();
        assert_eq!((s.optimize[0].values.len(), current(&s, &s.optimize[0].target), current(&s, &s.optimize[1].target)), (4, 5., 3.));
        let t = apply(&s, &s.optimize, &[4., 2.]);
        assert_eq!((t.exec.stop_loss, t.exec.trailing), (Some(Dist::Pct(4.)), Some(Dist::Atr { mult: 2., length: 14 })));
        assert!(err(&text.replace("exit.stop_loss.pct", "exit.stop_loss.atr")).message().contains("exit.stop_loss is set by pct"));
        assert!(err(&text.replace("exit.stop_loss.pct", "exit.target.pct")).message().contains("the spec has no exit.target"));
        assert!(err(&text.replace(r#""from":2,"to":8"#, r#""from":50,"to":120"#)).message().contains("out of range"));
    }
    #[test] fn intraday_and_condition_limits() {
        let e = err(&with(r#""s":"sma""#, r#""s":"vwap""#));
        assert_eq!(e.status(), "needs_data");
        let cond = r#"{ "lhs": {"s":"close"}, "op":">", "rhs": {"value":1} }"#;
        let many = format!(r#"{{"v":1,"entry":{{"all":[{}]}},"exit":{{"any":[{}]}}}}"#, vec![cond; 7].join(","), vec![cond; 6].join(","));
        assert!(err(&many).message().contains("13 conditions; the limit is 12"));
    }
    #[test] fn optimize_grid_is_capped_like_the_book_optimizer() {
        let big = with(r#"[ {"path":"entry.all[0].lhs.p.length","from":2,"to":6,"step":1} ]"#,
            r#"[ {"path":"entry.all[0].lhs.p.length","from":2,"to":301,"step":1}, {"path":"entry.all[1].rhs.p.length","from":1,"to":200,"step":1} ]"#);
        assert!(err(&big).message().contains("more than 50000 combinations"));
        let twice = with(r#"{"path":"entry.all[0].lhs.p.length","from":2,"to":6,"step":1}"#, r#"{"path":"entry.all[0].lhs.p.length","from":2,"to":3,"step":1},{"path":"entry.all[0].lhs.p.length","from":2,"to":3,"step":1}"#);
        assert!(err(&twice).message().contains("listed twice"));
        let value = parse_spec(&with("entry.all[0].lhs.p.length\",\"from\":2,\"to\":6", "entry.all[0].rhs.value\",\"from\":5,\"to\":15")).unwrap();
        assert_eq!((value.optimize[0].values.len(), current(&value, &value.optimize[0].target)), (11, 10.));
    }
    #[test] fn unbounded_optimize_ranges_are_rejected() {
        for (f, t, st) in ["0,1e30,1", "-1e308,1e308,1", "0,1,1e-300"].iter().map(|r| { let v: Vec<&str> = r.split(',').collect(); (v[0], v[1], v[2]) }) {
            let e = err(&with("\"from\":2,\"to\":6,\"step\":1", &format!("\"from\":{f},\"to\":{t},\"step\":{st}")));
            assert_eq!(e.status(), "invalid");
            assert!(e.message().contains("bad range"), "{:?}", e.message());
        }
        assert_eq!(parse_spec(EXAMPLE).unwrap().optimize[0].values.len(), 5);
    }
    #[test] fn engine_errors_map_to_types() {
        assert_eq!(LabError::from("needs_data: no VIX".to_string()), LabError::NeedsData("no VIX".into()));
        assert_eq!(LabError::from("empty date range".to_string()).status(), "invalid");
        let j = LabError::Invalid("bad \"x\"\n".into()).json();
        assert!(j.starts_with("{\"status\":\"invalid\",\"error\":\"bad \\\"x\\\"\\u000a\",\"label\":\"User-defined indicator strategy"), "{j}");
    }
}
