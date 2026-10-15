//! Rule spec v1 (indicators lab spec §4): parse, validate against the catalogue, expand optimize grids.
//! Reserved fields return `Unsupported`; malformed input returns `Invalid` naming the field.
use super::catalogue::{self, Params};
use super::json::{self, Json};
use super::resample::{Expand, Tf, TF_HELP};

pub const LABEL: &str = "User-defined indicator strategy · simulated · not a book method";
pub const MAX_CONDITIONS: usize = 12;
pub const MAX_SPEC_BYTES: usize = 8192;
pub const MAX_OFFSET: usize = 1000;
pub const PRICE_SOURCES: &[&str] = &["open", "high", "low", "close", "hl2", "hlc3", "ohlc4", "volume"];
pub const OPERATORS: &[&str] = &["crosses_above", "crosses_below", ">", "<", ">=", "<=", "=", "!=", "crosses", "between", "outside", "rising", "falling"];
/// Group kinds a rule list may hold besides comparisons (spec §4.1).
pub const NODE_KINDS: &[&str] = &["all", "any", "not", "at_least", "then"];
/// Whole-spec bounds (spec §4.3): comparisons, and comparisons + groups + expressions, over every rule list.
pub const MAX_LEAVES: usize = 40;
pub const MAX_NODES: usize = 120;
pub const MAX_STEPS: usize = 4;

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
pub enum Op { CrossesAbove, CrossesBelow, Gt, Lt, Ge, Le, Eq, Ne, Crosses }
#[derive(Clone, Debug, PartialEq)]
pub struct SeriesRef { pub id: String, pub params: Params, pub tf: Tf, pub out: usize, pub offset: usize }
#[derive(Clone, Debug, PartialEq)]
pub enum Operand { Series(SeriesRef), Value(f64), BarsHeld, Expr(Box<Expr>) }
/// Expression operands (spec §4.1), evaluated on the daily bars.
pub const EXPRESSIONS: &[&str] = &["add", "sub", "mul", "div", "min", "max", "pct", "abs", "ref", "sum", "avg", "highest", "lowest", "change",
    "bars_since", "value_when", "highest_since", "lowest_since", "count", "compress"];
/// TimeFrameCompress modes (spec §5.1).
pub const AGGREGATES: &[&str] = &["last", "first", "high", "low", "sum"];
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Agg { Last, First, High, Low, Sum }
impl Agg { pub fn key(self) -> &'static str { AGGREGATES[self as usize] } }
pub const MAX_NTH: usize = 20;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Bin { Add, Sub, Mul, Div, Min, Max, Pct }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Win { Sum, Avg, Highest, Lowest, Change }
impl Bin { pub fn key(self) -> &'static str { EXPRESSIONS[self as usize] } }
impl Win { pub fn key(self) -> &'static str { EXPRESSIONS[9 + self as usize] } }
#[derive(Clone, Debug, PartialEq)]
pub enum Expr {
    Bin(Bin, Operand, Operand),
    Abs(Operand),
    Ref(Operand, usize),
    Win(Win, Operand, usize),
    BarsSince(Node),
    ValueWhen { when: Node, of: Operand, nth: usize },
    Since { when: Node, of: Operand, highest: bool },
    Count(Node, usize),
    /// A series operand expanded with `expand: first|point` (`last` stays a plain series operand, the v1 path).
    Expand(SeriesRef, Expand),
    Compress { of: Operand, tf: Tf, agg: Agg, offset: usize, expand: Expand },
}
#[derive(Clone, Debug, PartialEq)]
pub struct Cond { pub lhs: Operand, pub op: Op, pub rhs: Operand }
/// One entry of a rule list (spec §4.1). `Cmp` is the v1 condition; the other kinds are Phase 7.
#[derive(Clone, Debug, PartialEq)]
pub enum Node {
    Cmp(Cond),
    Range { x: Operand, lo: Operand, hi: Operand, outside: bool },
    Trend { x: Operand, bars: usize, rising: bool },
    All(Vec<Node>),
    Any(Vec<Node>),
    Not(Box<Node>),
    AtLeast { k: usize, of: usize, when: Box<Node> },
    Then { steps: Vec<Node>, within: usize },
}
impl Node {
    /// The v1 comparison, when this node is one.
    pub fn cmp(&self) -> Option<&Cond> { if let Node::Cmp(c) = self { Some(c) } else { None } }
}
/// Where an optimize path points: a price-order slot, or a number inside a rule list addressed by its spec path.
#[derive(Clone, Debug, PartialEq)]
pub struct Target { pub path: String, pub slot: Option<Slot> }
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
pub struct Spec { pub entry: Vec<Node>, pub exit: Vec<Node>, pub cost_bps: Option<f64>, pub optimize: Vec<Dim>, pub exec: Exec, pub origin: Option<Origin> }

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
    let mut budget = Budget::default();
    let entry = conds(entry_obj.get("all"), "entry.all", false, false, &mut budget)?;
    let exit = conds(exit_obj.get("any"), "exit.any", true, price_exit, &mut budget)?;
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

/// Counts comparisons and nodes while a spec is parsed (spec §4.3).
#[derive(Default)]
struct Budget { leaves: usize, nodes: usize }
impl Budget {
    fn take(&mut self, leaf: bool) -> Result<(), LabError> {
        self.nodes += 1;
        if leaf { self.leaves += 1; }
        if self.leaves > MAX_LEAVES { return Err(inv(format!("more than {MAX_LEAVES} comparisons in the spec; the limit is {MAX_LEAVES}"))); }
        if self.nodes > MAX_NODES { return Err(inv(format!("more than {MAX_NODES} comparisons, groups and expressions in the spec; the limit is {MAX_NODES}"))); }
        Ok(())
    }
}

fn conds(v: Option<&Json>, at: &str, exit: bool, may_be_empty: bool, b: &mut Budget) -> Result<Vec<Node>, LabError> {
    let Some(Json::Arr(items)) = v else { return Err(inv(format!("{at} must be a list of conditions"))) };
    if items.is_empty() && !may_be_empty { return Err(inv(format!("{at} needs at least one condition{}", if exit { " (or a stop_loss, target or trailing exit)" } else { "" }))); }
    if items.len() > MAX_CONDITIONS { return Err(inv(format!("{at} has {} conditions; the limit is {MAX_CONDITIONS}", items.len()))); }
    items.iter().enumerate().map(|(i, c)| node(c, &format!("{at}[{i}]"), exit, b)).collect()
}

/// One rule-list entry. `held` allows bars_held: top-level exit comparisons only, because every other node is a precomputed array.
fn node(v: &Json, at: &str, held: bool, b: &mut Budget) -> Result<Node, LabError> {
    if v.get("lhs").is_some() { return leaf(v, at, held, b); }
    b.take(false)?;
    let list = |key: &str, b: &mut Budget| -> Result<Vec<Node>, LabError> {
        let Some(Json::Arr(items)) = v.get(key) else { return Err(inv(format!("{at}.{key} must be a list"))) };
        if items.is_empty() { return Err(inv(format!("{at}.{key} needs at least one entry"))); }
        items.iter().enumerate().map(|(i, c)| node(c, &format!("{at}.{key}[{i}]"), false, b)).collect()
    };
    if v.get("all").is_some() { fields(v, at, &["all"], &[])?; return Ok(Node::All(list("all", b)?)); }
    if v.get("any").is_some() { fields(v, at, &["any"], &[])?; return Ok(Node::Any(list("any", b)?)); }
    if let Some(n) = v.get("not") { fields(v, at, &["not"], &[])?; return Ok(Node::Not(Box::new(node(n, &format!("{at}.not"), false, b)?))); }
    if v.get("at_least").is_some() {
        fields(v, at, &["at_least", "of", "when"], &[])?;
        let (k, of) = (whole(v.get("at_least"), &format!("{at}.at_least"), 1)?, whole(v.get("of"), &format!("{at}.of"), 1)?);
        if k > of { return Err(inv(format!("{at}: at_least ({k}) cannot exceed of ({of})"))); }
        let when = v.get("when").ok_or_else(|| inv(format!("{at}.when is required")))?;
        return Ok(Node::AtLeast { k, of, when: Box::new(node(when, &format!("{at}.when"), false, b)?) });
    }
    if v.get("then").is_some() {
        fields(v, at, &["then", "within"], &[])?;
        let steps = list("then", b)?;
        if !(2..=MAX_STEPS).contains(&steps.len()) { return Err(inv(format!("{at}.then needs 2 to {MAX_STEPS} steps"))); }
        let within = whole(v.get("within"), &format!("{at}.within"), 1)?;
        if within + 1 < steps.len() { return Err(inv(format!("{at}: within ({within}) is shorter than the {} steps need ({}); the sequence could never complete", steps.len(), steps.len() - 1))); }
        return Ok(Node::Then { steps, within });
    }
    Err(inv(format!("{at} must be a comparison (lhs, op, rhs) or a group ({})", NODE_KINDS.join(", "))))
}

/// A whole number from `min` to MAX_OFFSET (window lengths, counts, sequence spans).
fn whole(v: Option<&Json>, at: &str, min: usize) -> Result<usize, LabError> {
    v.and_then(Json::as_f64).filter(|n| n.fract() == 0. && *n >= min as f64 && *n <= MAX_OFFSET as f64).map(|n| n as usize)
        .ok_or_else(|| inv(format!("{at} must be a whole number from {min} to {MAX_OFFSET}")))
}

fn leaf(v: &Json, at: &str, held: bool, b: &mut Budget) -> Result<Node, LabError> {
    b.take(true)?;
    let side = |k: &str, b: &mut Budget| operand(v.get(k), &format!("{at}.{k}"), false, b);
    match v.get("op").and_then(Json::as_str) {
        Some(op @ ("between" | "outside")) => {
            fields(v, at, &["lhs", "op", "lo", "hi"], &[])?;
            let (x, lo, hi) = (side("lhs", b)?, side("lo", b)?, side("hi", b)?);
            if let (Operand::Value(l), Operand::Value(h)) = (&lo, &hi) { if l > h { return Err(inv(format!("{at}: lo ({l}) is above hi ({h}); {op} would be {}", if op == "between" { "never true" } else { "always true" }))); } }
            Ok(Node::Range { x, lo, hi, outside: op == "outside" })
        }
        Some(op @ ("rising" | "falling")) => {
            fields(v, at, &["lhs", "op", "bars"], &[])?;
            Ok(Node::Trend { x: side("lhs", b)?, bars: whole(v.get("bars"), &format!("{at}.bars"), 1)?, rising: op == "rising" })
        }
        _ => cond(v, at, held, b).map(Node::Cmp),
    }
}

fn cond(v: &Json, at: &str, held: bool, b: &mut Budget) -> Result<Cond, LabError> {
    fields(v, at, &["lhs", "op", "rhs"], &[])?;
    let op = match v.get("op").and_then(Json::as_str) {
        Some("crosses_above") => Op::CrossesAbove,
        Some("crosses_below") => Op::CrossesBelow,
        Some("crosses") => Op::Crosses,
        Some(">") => Op::Gt,
        Some("<") => Op::Lt,
        Some(">=") => Op::Ge,
        Some("<=") => Op::Le,
        Some("=") => Op::Eq,
        Some("!=") => Op::Ne,
        _ => return Err(inv(format!("{at}.op must be one of {}", OPERATORS.join(", ")))),
    };
    let lhs = operand(v.get("lhs"), &format!("{at}.lhs"), held, b)?;
    let rhs = operand(v.get("rhs"), &format!("{at}.rhs"), held, b)?;
    if matches!((&lhs, &rhs), (Operand::Value(_), Operand::Value(_))) { return Err(inv(format!("{at} compares two numbers; use at least one series"))); }
    Ok(Cond { lhs, op, rhs })
}

fn operand(v: Option<&Json>, at: &str, held: bool, b: &mut Budget) -> Result<Operand, LabError> {
    let v = v.ok_or_else(|| inv(format!("{at} is required")))?;
    if v.get("value").is_some() {
        fields(v, at, &["value"], &[])?;
        return v.get("value").and_then(Json::as_f64).map(Operand::Value).ok_or_else(|| inv(format!("{at}.value must be a number")));
    }
    if let Some(e) = expr(v, at, b)? { return Ok(Operand::Expr(Box::new(e))); }
    if v.get("symbol").is_some() {
        return Err(LabError::Unsupported(format!("{at}.symbol: operands from another symbol (AmiBroker Foreign, TradingView request.security on another ticker) arrive in Phase 8")));
    }
    fields(v, at, &["s", "p", "tf", "out", "offset", "expand"], &[])?;
    let id = v.get("s").and_then(Json::as_str).ok_or_else(|| inv(format!("{at}.s must name a series (or give a value)")))?;
    if id == "bars_held" {
        if !held { return Err(inv(format!("{at}: bars_held is only available as a plain comparison at the top level of the exit list (not inside a group, range, trend or expression)"))); }
        if ["p", "tf", "out", "offset", "expand"].iter().any(|k| v.get(k).is_some()) { return Err(inv(format!("{at}: bars_held takes no p, tf, out, offset or expand"))); }
        return Ok(Operand::BarsHeld);
    }
    let tf = match v.get("tf") { None => Tf::D, Some(t) => t.as_str().and_then(Tf::parse).ok_or_else(|| inv(format!("{at}.tf must be {TF_HELP}")))? };
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
    let expand = expand_of(v, at, tf, offset, id == "open")?;
    let r = series_ref(at, id, &given, tf, out, offset)?;
    Ok(if expand == Expand::Last { Operand::Series(r) } else { Operand::Expr(Box::new(Expr::Expand(r, expand))) })
}
/// `expand` (default last). `first` is accepted only where the value is known by the close of the period's first daily bar:
/// a completed earlier period (offset >= 1) or the period's own first value (`known_at_start`). AmiBroker's docs: expandFirst
/// looks into the future for anything else.
fn expand_of(v: &Json, at: &str, tf: Tf, offset: usize, known_at_start: bool) -> Result<Expand, LabError> {
    let Some(e) = v.get("expand") else { return Ok(Expand::Last) };
    let e = e.as_str().and_then(Expand::parse).ok_or_else(|| inv(format!("{at}.expand must be last, first or point")))?;
    if tf == Tf::D && e != Expand::Last { return Err(inv(format!("{at}.expand applies to W, M, Q, Y and nD operands"))); }
    if e == Expand::First && offset == 0 && !known_at_start {
        return Err(LabError::Unsupported(format!("{at}: expandFirst would show the period's final value from its first day (lookahead); use offset >= 1, the period open, or expand last")));
    }
    Ok(e)
}

/// An expression operand (spec §4.1), or None when `v` is a series operand.
fn expr(v: &Json, at: &str, b: &mut Budget) -> Result<Option<Expr>, LabError> {
    let Json::Obj(kv) = v else { return Ok(None) };
    if kv.iter().any(|(k, _)| EXPRESSIONS.contains(&k.as_str())) && v.get("symbol").is_some() {
        return Err(LabError::Unsupported(format!("{at}.symbol: operands from another symbol (AmiBroker Foreign, TradingView request.security on another ticker) arrive in Phase 8")));
    }
    let Some(key) = kv.iter().map(|(k, _)| k.as_str()).find(|k| EXPRESSIONS.contains(k)) else { return Ok(None) };
    b.take(false)?;
    // a unary, window or state expression over a constant is that constant (or 0): refuse it like two constants
    let sub = |k: &str, b: &mut Budget| match operand(v.get(k), &format!("{at}.{k}"), false, b)? {
        Operand::Value(_) => Err(inv(format!("{at}.{k} is a number; {key} needs a series"))),
        o => Ok(o),
    };
    Ok(Some(match key {
        "add" | "sub" | "mul" | "div" | "min" | "max" | "pct" => {
            fields(v, at, &[key], &[])?;
            let pair = || inv(format!("{at}.{key} must be a list of two operands"));
            let Some(Json::Arr(xs)) = v.get(key) else { return Err(pair()) };
            if xs.len() != 2 { return Err(pair()); }
            let (l, r) = (operand(Some(&xs[0]), &format!("{at}.{key}[0]"), false, b)?, operand(Some(&xs[1]), &format!("{at}.{key}[1]"), false, b)?);
            if matches!((&l, &r), (Operand::Value(_), Operand::Value(_))) { return Err(inv(format!("{at}.{key} combines two numbers; use at least one series"))); }
            let op = [Bin::Add, Bin::Sub, Bin::Mul, Bin::Div, Bin::Min, Bin::Max, Bin::Pct].into_iter().find(|o| o.key() == key).unwrap();
            Expr::Bin(op, l, r)
        }
        "abs" => { fields(v, at, &["abs"], &[])?; Expr::Abs(sub("abs", b)?) }
        "ref" => { fields(v, at, &["ref", "bars"], &[])?; Expr::Ref(sub("ref", b)?, whole(v.get("bars"), &format!("{at}.bars"), 0)?) }
        "bars_since" => { fields(v, at, &[key], &[])?; Expr::BarsSince(cond_of(v, key, at, b)?) }
        "count" => { fields(v, at, &[key, "bars"], &[])?; Expr::Count(cond_of(v, key, at, b)?, whole(v.get("bars"), &format!("{at}.bars"), 1)?) }
        "value_when" => {
            fields(v, at, &[key, "of", "nth"], &[])?;
            let nth = match v.get("nth") {
                None => 1,
                Some(n) => n.as_f64().filter(|n| n.fract() == 0. && (1. ..=MAX_NTH as f64).contains(n)).ok_or_else(|| inv(format!("{at}.nth must be a whole number from 1 to {MAX_NTH}")))? as usize,
            };
            Expr::ValueWhen { when: cond_of(v, key, at, b)?, of: sub("of", b)?, nth }
        }
        "compress" => {
            fields(v, at, &[key, "tf", "mode", "offset", "expand"], &[])?;
            let tf = v.get("tf").and_then(Json::as_str).and_then(Tf::parse).filter(|t| *t != Tf::D)
                .ok_or_else(|| inv(format!("{at}.tf must be one of W, M, Q, Y or nD (2D to 63D)")))?;
            let agg = match v.get("mode") {
                None => Agg::Last,
                Some(m) => [Agg::Last, Agg::First, Agg::High, Agg::Low, Agg::Sum].into_iter().find(|a| Some(a.key()) == m.as_str())
                    .ok_or_else(|| inv(format!("{at}.mode must be one of {}", AGGREGATES.join(", "))))?,
            };
            let offset = if v.get("offset").is_some() { whole(v.get("offset"), &format!("{at}.offset"), 0)? } else { 0 };
            let expand = expand_of(v, at, tf, offset, agg == Agg::First)?;
            Expr::Compress { of: sub(key, b)?, tf, agg, offset, expand }
        }
        "highest_since" | "lowest_since" => { fields(v, at, &[key, "of"], &[])?; Expr::Since { when: cond_of(v, key, at, b)?, of: sub("of", b)?, highest: key == "highest_since" } }
        _ => {
            fields(v, at, &[key, "bars"], &[])?;
            let w = [Win::Sum, Win::Avg, Win::Highest, Win::Lowest, Win::Change].into_iter().find(|w| w.key() == key).unwrap();
            Expr::Win(w, sub(key, b)?, whole(v.get("bars"), &format!("{at}.bars"), 1)?)
        }
    }))
}

/// The condition a state operand watches (`{"bars_since": node}` …).
fn cond_of(v: &Json, key: &str, at: &str, b: &mut Budget) -> Result<Node, LabError> {
    node(v.get(key).ok_or_else(|| inv(format!("{at}.{key} is required")))?, &format!("{at}.{key}"), false, b)
}

/// Validates one series operand against the catalogue (shared by rule specs and /api/lab/series).
pub fn series_ref(at: &str, id: &str, given: &Params, tf: Tf, out: Option<&str>, offset: usize) -> Result<SeriesRef, LabError> {
    if PRICE_SOURCES.contains(&id) {
        if !given.is_empty() || out.is_some() { return Err(inv(format!("{at}: price source '{id}' takes no p or out"))); }
        return Ok(SeriesRef { id: id.into(), params: Params::new(), tf, out: 0, offset });
    }
    let def = catalogue::get(id).ok_or_else(|| inv(format!("{at}.s: unknown series '{id}'")))?;
    if def.needs.intraday { return Err(LabError::NeedsData(format!("{at}: {}: {}", def.name, def.note))); }
    if def.compute.is_none() {
        let msg = format!("{at}: {}: {}", def.name, def.note);
        return Err(if def.needs.any_data() { LabError::NeedsData(msg) } else { LabError::Unsupported(msg) });
    }
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
    // at_least and of may both be optimized: the worst pairing must still be a valid node.
    for a in &out {
        let Some(base) = a.path.strip_suffix(".at_least") else { continue };
        let of_path = format!("{base}.of");
        let Some(o) = out.iter().find(|d| d.path == of_path) else { continue };
        let (hi, lo) = (a.values.iter().copied().fold(f64::MIN, f64::max), o.values.iter().copied().fold(f64::MAX, f64::min));
        if hi > lo { return Err(inv(format!("optimize '{}' reaches {hi} but '{of_path}' reaches down to {lo}; at_least cannot exceed of", a.path))); }
    }
    Ok(out)
}

fn target(spec: &Spec, path: &str) -> Result<Target, String> {
    if let Some((slot, at, _)) = SLOTS.iter().find(|(_, at, _)| path.starts_with(&format!("{at}."))) {
        let field = &path[at.len() + 1..];
        return match (spec.exec.slot(*slot), field) {
            (Some(Dist::Pct(_)), "pct") | (Some(Dist::Atr { .. }), "atr") => Ok(Target { path: path.into(), slot: Some(*slot) }),
            (None, _) => Err(format!("the spec has no {at}")),
            _ => Err(format!("{at} is set by {}; optimize that field", if matches!(spec.exec.slot(*slot), Some(Dist::Pct(_))) { "pct" } else { "atr" })),
        };
    }
    if !path.starts_with("entry.all[") && !path.starts_with("exit.any[") {
        return Err("must start with entry.all[i], exit.any[i], entry.limit, exit.stop_loss, exit.target or exit.trailing".into());
    }
    let (mut found, mut near) = (false, None::<String>);
    walk(&mut spec.clone(), &mut |p, k| match k {
        Knob::Series(r) => if let Some(key) = path.strip_prefix(&format!("{p}.p.")) {
            near = Some(catalogue::get(&r.id).map_or("price sources have no parameters".into(), |d| format!("{} has no parameter '{key}'", d.id)));
        },
        _ => found |= p == path,
    });
    if found { return Ok(Target { path: path.into(), slot: None }); }
    if let Some(m) = near { return Err(m); }
    let beyond = |list: &str, n: usize| path.strip_prefix(list).and_then(|r| r.split_once(']')).and_then(|(i, _)| i.parse::<usize>().ok()).is_some_and(|i| i >= n);
    if beyond("entry.all[", spec.entry.len()) || beyond("exit.any[", spec.exit.len()) { return Err("no condition at that index".into()); }
    Err("names no number in the spec (paths end in .value, .p.<key> or a whole-number field: bars, within, of, at_least)".into())
}

/// A number an optimize path can reach inside a rule list; `Series` is reported for error messages only.
enum Knob<'a> { Value(&'a mut f64), Param(&'a mut SeriesRef, String), Whole(&'a mut usize), Shift(&'a mut usize), Series(&'a SeriesRef) }
/// Visits every optimizable number in the rule lists with its spec path (`entry.all[0].lhs.p.length`, `exit.any[1].rhs.value`, `….bars`).
fn walk(s: &mut Spec, f: &mut dyn FnMut(String, Knob)) {
    for (k, n) in s.entry.iter_mut().enumerate() { walk_node(n, &format!("entry.all[{k}]"), f); }
    for (k, n) in s.exit.iter_mut().enumerate() { walk_node(n, &format!("exit.any[{k}]"), f); }
}
fn walk_node(n: &mut Node, at: &str, f: &mut dyn FnMut(String, Knob)) {
    match n {
        Node::Cmp(c) => { walk_operand(&mut c.lhs, &format!("{at}.lhs"), f); walk_operand(&mut c.rhs, &format!("{at}.rhs"), f); }
        Node::Range { x, lo, hi, .. } => { walk_operand(x, &format!("{at}.lhs"), f); walk_operand(lo, &format!("{at}.lo"), f); walk_operand(hi, &format!("{at}.hi"), f); }
        Node::Trend { x, bars, .. } => { walk_operand(x, &format!("{at}.lhs"), f); f(format!("{at}.bars"), Knob::Whole(bars)); }
        Node::All(v) => for (k, c) in v.iter_mut().enumerate() { walk_node(c, &format!("{at}.all[{k}]"), f) },
        Node::Any(v) => for (k, c) in v.iter_mut().enumerate() { walk_node(c, &format!("{at}.any[{k}]"), f) },
        Node::Not(c) => walk_node(c, &format!("{at}.not"), f),
        Node::AtLeast { k, of, when } => { f(format!("{at}.at_least"), Knob::Whole(k)); f(format!("{at}.of"), Knob::Whole(of)); walk_node(when, &format!("{at}.when"), f); }
        Node::Then { steps, within } => {
            for (k, c) in steps.iter_mut().enumerate() { walk_node(c, &format!("{at}.then[{k}]"), f); }
            f(format!("{at}.within"), Knob::Whole(within));
        }
    }
}
fn walk_operand(o: &mut Operand, at: &str, f: &mut dyn FnMut(String, Knob)) {
    match o {
        Operand::Value(x) => f(format!("{at}.value"), Knob::Value(x)),
        Operand::Series(r) => walk_series(r, at, f),
        Operand::BarsHeld => {}
        Operand::Expr(e) => match &mut **e {
            Expr::Bin(op, l, r) => { walk_operand(l, &format!("{at}.{}[0]", op.key()), f); walk_operand(r, &format!("{at}.{}[1]", op.key()), f); }
            Expr::Abs(x) => walk_operand(x, &format!("{at}.abs"), f),
            Expr::Ref(x, n) => { walk_operand(x, &format!("{at}.ref"), f); f(format!("{at}.bars"), Knob::Shift(n)); }
            Expr::Win(w, x, n) => { walk_operand(x, &format!("{at}.{}", w.key()), f); f(format!("{at}.bars"), Knob::Whole(n)); }
            Expr::BarsSince(c) => walk_node(c, &format!("{at}.bars_since"), f),
            Expr::Count(c, n) => { walk_node(c, &format!("{at}.count"), f); f(format!("{at}.bars"), Knob::Whole(n)); }
            Expr::ValueWhen { when, of, nth } => { walk_node(when, &format!("{at}.value_when"), f); walk_operand(of, &format!("{at}.of"), f); f(format!("{at}.nth"), Knob::Whole(nth)); }
            Expr::Since { when, of, highest } => { walk_node(when, &format!("{at}.{}", if *highest { "highest_since" } else { "lowest_since" }), f); walk_operand(of, &format!("{at}.of"), f); }
            Expr::Expand(r, _) => walk_series(r, at, f),
            Expr::Compress { of, .. } => walk_operand(of, &format!("{at}.compress"), f),
        },
    }
}
fn walk_series(r: &mut SeriesRef, at: &str, f: &mut dyn FnMut(String, Knob)) {
    f(at.to_string(), Knob::Series(r));
    let keys: Vec<String> = r.params.keys().cloned().collect();
    for k in keys { f(format!("{at}.p.{k}"), Knob::Param(&mut *r, k)); }
}
/// Node invariants the parser checks, re-checked after an optimize value is applied (at_least <= of, then spans, constant range bounds).
fn consistent(n: &Node) -> Result<(), String> {
    match n {
        Node::AtLeast { k, of, .. } if k > of => Err(format!("at_least ({k}) cannot exceed of ({of})")),
        Node::Then { steps, within } if *within + 1 < steps.len() => Err(format!("within ({within}) is shorter than the {} steps need ({})", steps.len(), steps.len() - 1)),
        Node::Range { lo: Operand::Value(l), hi: Operand::Value(h), .. } if l > h => Err(format!("lo ({l}) is above hi ({h})")),
        Node::AtLeast { when, .. } | Node::Not(when) => consistent(when),
        Node::All(v) | Node::Any(v) | Node::Then { steps: v, .. } => v.iter().try_for_each(consistent),
        Node::Cmp(c) => [&c.lhs, &c.rhs].into_iter().try_for_each(consistent_operand),
        Node::Range { x, lo, hi, .. } => [x, lo, hi].into_iter().try_for_each(consistent_operand),
        Node::Trend { x, .. } => consistent_operand(x),
    }
}
/// Nodes inside state operands obey the same rules; value_when's nth stays within MAX_NTH.
fn consistent_operand(o: &Operand) -> Result<(), String> {
    let Operand::Expr(e) = o else { return Ok(()) };
    match &**e {
        Expr::Bin(_, l, r) => { consistent_operand(l)?; consistent_operand(r) }
        Expr::Abs(x) | Expr::Ref(x, _) | Expr::Win(_, x, _) => consistent_operand(x),
        Expr::BarsSince(c) | Expr::Count(c, _) => consistent(c),
        Expr::ValueWhen { nth, .. } if *nth > MAX_NTH => Err(format!("nth ({nth}) must be a whole number from 1 to {MAX_NTH}")),
        Expr::ValueWhen { when, of, .. } | Expr::Since { when, of, .. } => { consistent(when)?; consistent_operand(of) }
        Expr::Compress { of, .. } => consistent_operand(of),
        Expr::Expand(..) => Ok(()),
    }
}

fn set(s: &mut Spec, t: &Target, x: f64) {
    if let Some(slot) = t.slot {
        match s.exec.slot_mut(slot) { Some(Dist::Pct(p)) => *p = x, Some(Dist::Atr { mult, .. }) => *mult = x, None => {} }
        return;
    }
    walk(s, &mut |p, k| if p == t.path {
        match k {
            Knob::Value(v) => *v = x,
            Knob::Param(r, key) => { r.params.insert(key, x); }
            Knob::Whole(n) | Knob::Shift(n) => *n = x as usize,
            Knob::Series(_) => {}
        }
    });
}
fn check_value(spec: &Spec, t: &Target, x: f64) -> Result<(), String> {
    if let Some(slot) = t.slot {
        let (_, at, hi) = SLOTS.iter().find(|(s, _, _)| *s == slot).unwrap();
        let ok = match spec.exec.slot(slot) { Some(Dist::Pct(_)) => x <= *hi && (x > 0. || (x == 0. && slot == Slot::Limit)), _ => x > 0. && x <= MAX_ATR_MULT };
        return if ok { Ok(()) } else { Err(format!("{at} value {x} is out of range")) };
    }
    let mut s = spec.clone();
    set(&mut s, t, x);
    let mut res = Ok(());
    walk(&mut s, &mut |p, k| if p == t.path {
        res = match k {
            Knob::Param(r, _) => catalogue::get(&r.id).ok_or_else(|| "unknown series".to_string()).and_then(|d| catalogue::resolve_params(&d, &r.params).map(|_| ())),
            Knob::Shift(_) if x.fract() != 0. || x < 0. || x > MAX_OFFSET as f64 => Err(format!("{} must be a whole number from 0 to {MAX_OFFSET}, not {x}", t.path)),
            Knob::Whole(_) if x.fract() != 0. || x < 1. || x > MAX_OFFSET as f64 => Err(format!("{} must be a whole number from 1 to {MAX_OFFSET}, not {x}", t.path)),
            _ => Ok(()),
        };
    });
    res?;
    s.entry.iter().chain(&s.exit).try_for_each(consistent)
}
/// Every catalogue or price series id the rule lists read, nested ones included.
pub fn series_ids(spec: &Spec) -> Vec<String> {
    let mut ids = Vec::new();
    walk(&mut spec.clone(), &mut |_, k| if let Knob::Series(r) = k { ids.push(r.id.clone()); });
    ids
}
/// The spec's own value at an optimize target (the "base" setting the ranking prefers on ties).
pub fn current(spec: &Spec, t: &Target) -> f64 {
    if let Some(slot) = t.slot { return match spec.exec.slot(slot) { Some(Dist::Pct(p)) => p, Some(Dist::Atr { mult, .. }) => mult, None => f64::NAN }; }
    let mut v = f64::NAN;
    walk(&mut spec.clone(), &mut |p, k| if p == t.path {
        match k {
            Knob::Value(x) => v = *x,
            Knob::Param(r, key) => v = r.params.get(&key).copied().unwrap_or(f64::NAN),
            Knob::Whole(n) | Knob::Shift(n) => v = *n as f64,
            Knob::Series(_) => {}
        }
    });
    v
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
        let Operand::Series(r) = &s.entry[1].cmp().unwrap().rhs else { panic!() };
        assert_eq!((r.id.as_str(), r.tf, r.params["length"]), ("sma", Tf::W, 40.));
        assert_eq!(s.exit[1].cmp().unwrap().lhs, Operand::BarsHeld);
        assert_eq!(s.optimize[0].values, vec![2., 3., 4., 5., 6.]);
        assert_eq!(current(&s, &s.optimize[0].target), 2.);
        let t = apply(&s, &s.optimize, &[5.]);
        let Operand::Series(r) = &t.entry[0].cmp().unwrap().lhs else { panic!() };
        assert_eq!(r.params["length"], 5.);
    }
    #[test] fn defaults_fill_params_tf_and_output() {
        let s = parse_spec(&with(r#"{"s":"close","tf":"D"}"#, r#"{"s":"macd","out":"signal"}"#)).unwrap();
        let Operand::Series(r) = &s.entry[1].cmp().unwrap().lhs else { panic!() };
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
            (with(r#"{"s":"close","tf":"D"}"#, r#"{"s":"bars_held"}"#), "only available as a plain comparison"),
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

    // ---- Phase 7: nodes (spec §4.1–§4.3)
    const ONE_EXIT: &str = r#"{"v":1,"entry":{"all":[NODE]},"exit":{"any":[{"lhs":{"s":"bars_held"},"op":">=","rhs":{"value":5}}]}}"#;
    #[test] fn nodes_parse_and_name_their_fields() {
        let s = parse_spec(r#"{"v":1,"entry":{"all":[
            {"any":[{"lhs":{"s":"close"},"op":"=","rhs":{"value":1}},{"not":{"lhs":{"s":"close"},"op":"crosses","rhs":{"s":"open"}}}]},
            {"lhs":{"s":"rsi"},"op":"between","lo":{"value":20},"hi":{"value":40}},
            {"lhs":{"s":"close"},"op":"rising","bars":3},
            {"at_least":3,"of":5,"when":{"lhs":{"s":"close"},"op":">","rhs":{"s":"open"}}},
            {"then":[{"lhs":{"s":"rsi"},"op":"<","rhs":{"value":30}},{"lhs":{"s":"rsi"},"op":">","rhs":{"value":50}}],"within":10}]},
            "exit":{"any":[{"lhs":{"s":"bars_held"},"op":">=","rhs":{"value":5}}]}}"#).unwrap();
        assert!(matches!(&s.entry[0], Node::Any(v) if matches!(&v[1], Node::Not(_))));
        assert!(matches!(&s.entry[1], Node::Range { outside: false, .. }));
        assert!(matches!(&s.entry[2], Node::Trend { bars: 3, rising: true, .. }));
        assert!(matches!(&s.entry[3], Node::AtLeast { k: 3, of: 5, .. }));
        assert!(matches!(&s.entry[4], Node::Then { within: 10, .. }));
        for (node, needle) in [
            (r#"{"all":[]}"#, "entry.all[0].all needs at least one entry"),
            (r#"{"at_least":4,"of":3,"when":{"lhs":{"s":"close"},"op":">","rhs":{"value":1}}}"#, "at_least (4) cannot exceed of (3)"),
            (r#"{"then":[{"lhs":{"s":"close"},"op":">","rhs":{"value":1}}],"within":3}"#, "then needs 2 to 4 steps"),
            (r#"{"lhs":{"s":"close"},"op":"rising","bars":0}"#, "entry.all[0].bars must be a whole number from 1"),
            (r#"{"lhs":{"s":"close"},"op":"between","lo":{"value":1}}"#, "entry.all[0].hi is required"),
            (r#"{"lhs":{"s":"close"},"op":"between","lo":{"value":1},"hi":{"value":2},"rhs":{"value":3}}"#, "unknown field 'rhs' in entry.all[0]"),
            (r#"{"not":{"lhs":{"s":"bars_held"},"op":">","rhs":{"value":1}}}"#, "only available as a plain comparison"),
            (r#"{"maybe":[]}"#, "must be a comparison (lhs, op, rhs) or a group (all, any, not, at_least, then)"),
        ] {
            let e = err(&ONE_EXIT.replace("NODE", node));
            assert_eq!(e.status(), "invalid", "{node}");
            assert!(e.message().contains(needle), "{needle:?} not in {:?}", e.message());
        }
        // bars_held stays a top-level exit operand: inside an exit group it is refused
        let grouped = ONE_EXIT.replace("NODE", r#"{"lhs":{"s":"close"},"op":">","rhs":{"value":1}}"#)
            .replace(r#""any":[{"lhs":{"s":"bars_held"},"op":">=","rhs":{"value":5}}]"#, r#""any":[{"any":[{"lhs":{"s":"bars_held"},"op":">=","rhs":{"value":5}}]}]"#);
        assert!(err(&grouped).message().contains("only available as a plain comparison"));
    }
    #[test] fn whole_spec_bounds_count_nested_comparisons_and_nodes() {
        let c = r#"{"lhs":{"s":"close"},"op":">","rhs":{"value":1}}"#;
        let group = |n: usize| format!(r#"{{"all":[{}]}}"#, vec![c; n].join(","));
        let spec = |items: Vec<String>| format!(r#"{{"v":1,"entry":{{"all":[{}]}},"exit":{{"any":[{c}]}}}}"#, items.join(","));
        assert!(parse_spec(&spec(vec![group(13), group(13), group(13)])).is_ok()); // 39 nested + 1 exit = 40 comparisons
        assert!(err(&spec(vec![group(13), group(13), group(14)])).message().contains("more than 40 comparisons"));
        let nots = format!("{}{c}{}", r#"{"not":"#.repeat(10), "}".repeat(10)); // 11 nodes, 1 comparison
        assert!(err(&spec(vec![nots; 11])).message().contains("more than 120 comparisons, groups and expressions"));
        // exactly 120 nodes (119 in entry + 1 in exit) pass, 121 are refused
        let chain = |n: usize| format!("{}{c}{}", r#"{"not":"#.repeat(n - 1), "}".repeat(n - 1));
        let mut ok: Vec<String> = vec![chain(11); 10];
        ok.push(chain(9));
        assert!(parse_spec(&spec(ok.clone())).is_ok());
        ok.pop();
        ok.push(chain(10));
        assert!(err(&spec(ok)).message().contains("more than 120 comparisons, groups and expressions"));
    }
    #[test] fn optimize_reaches_numbers_inside_nodes() {
        let text = r#"{"v":1,"entry":{"all":[{"any":[{"lhs":{"s":"rsi","p":{"length":2}},"op":"<","rhs":{"value":10}}]},{"at_least":2,"of":3,"when":{"lhs":{"s":"close"},"op":"rising","bars":2}}]},
            "exit":{"any":[{"lhs":{"s":"bars_held"},"op":">=","rhs":{"value":5}}]},
            "optimize":[{"path":"entry.all[0].any[0].rhs.value","from":5,"to":15,"step":5},{"path":"entry.all[0].any[0].lhs.p.length","from":2,"to":3,"step":1},{"path":"entry.all[1].when.bars","from":1,"to":3,"step":1},{"path":"entry.all[1].at_least","from":1,"to":3,"step":1}]}"#;
        let s = parse_spec(text).unwrap();
        assert_eq!(s.optimize.iter().map(|d| current(&s, &d.target)).collect::<Vec<_>>(), vec![10., 2., 2., 2.]);
        let t = apply(&s, &s.optimize, &[15., 3., 3., 1.]);
        let Node::Any(v) = &t.entry[0] else { panic!() };
        let c = v[0].cmp().unwrap();
        let Operand::Series(r) = &c.lhs else { panic!() };
        assert_eq!((&c.rhs, r.params["length"]), (&Operand::Value(15.), 3.));
        assert!(matches!(&t.entry[1], Node::AtLeast { k: 1, of: 3, when } if matches!(**when, Node::Trend { bars: 3, .. })));
        let one = |path: &str, from: f64, to: f64| text.split(r#""optimize":"#).next().unwrap().to_string()
            + &format!(r#""optimize":[{{"path":"{path}","from":{from},"to":{to},"step":1}}]}}"#);
        for (path, from, to, needle) in [
            ("entry.all[1].at_least", 1., 4., "at_least (4) cannot exceed of (3)"),
            ("entry.all[1].when.bars", 0., 2., "entry.all[1].when.bars must be a whole number from 1"),
            ("entry.all[0].any[3].rhs.value", 1., 2., "names no number in the spec"),
            ("entry.all[0].any[0].lhs.p.len", 1., 2., "rsi has no parameter 'len'"),
            ("entry.all[2].lhs.p.length", 1., 2., "no condition at that index"),
        ] {
            let e = err(&one(path, from, to));
            assert!(e.message().contains(needle), "{path}: {needle:?} not in {:?}", e.message());
        }
    }

    #[test] fn optimizer_keeps_node_dimensions_consistent_and_useless_nodes_are_refused() {
        let base = r#"{"v":1,"entry":{"all":[{"at_least":2,"of":3,"when":{"lhs":{"s":"close"},"op":">","rhs":{"value":1}}},{"then":[{"lhs":{"s":"close"},"op":">","rhs":{"value":1}},{"lhs":{"s":"close"},"op":"<","rhs":{"value":9}},{"lhs":{"s":"close"},"op":">","rhs":{"value":2}}],"within":4}]},
            "exit":{"any":[{"lhs":{"s":"bars_held"},"op":">=","rhs":{"value":5}}]},"optimize":[OPT]}"#;
        let d = |p: &str, f: f64, t: f64| format!(r#"{{"path":"{p}","from":{f},"to":{t},"step":1}}"#);
        let run = |opts: Vec<String>| parse_spec(&base.replace("OPT", &opts.join(",")));
        // each dimension is valid alone, but the worst pairing is not
        let e = run(vec![d("entry.all[0].at_least", 1., 3.), d("entry.all[0].of", 2., 4.)]).unwrap_err();
        assert!(e.message().contains("entry.all[0].at_least") && e.message().contains("entry.all[0].of"), "{}", e.message());
        assert!(run(vec![d("entry.all[0].at_least", 1., 2.), d("entry.all[0].of", 2., 4.)]).is_ok());
        // within below steps-1 is refused value by value
        assert!(run(vec![d("entry.all[1].within", 2., 5.)]).is_ok());
        assert!(run(vec![d("entry.all[1].within", 1., 5.)]).unwrap_err().message().contains("within"));
        // a ref shift may be optimized down to 0
        let r = r#"{"v":1,"entry":{"all":[{"lhs":{"s":"close"},"op":">","rhs":{"ref":{"s":"close"},"bars":2}}]},"exit":{"any":[{"lhs":{"s":"bars_held"},"op":">=","rhs":{"value":5}}]},"optimize":[{"path":"entry.all[0].rhs.bars","from":0,"to":3,"step":1}]}"#;
        assert!(parse_spec(r).is_ok());
        // an optimized constant bound may not cross the other constant bound
        let range = r#"{"v":1,"entry":{"all":[{"lhs":{"s":"close"},"op":"between","lo":{"value":2},"hi":{"value":5}}]},"exit":{"any":[{"lhs":{"s":"bars_held"},"op":">=","rhs":{"value":5}}]},"optimize":[{"path":"entry.all[0].lo.value","from":1,"to":TO,"step":1}]}"#;
        assert!(parse_spec(&range.replace("TO", "5")).is_ok());
        assert!(err(&range.replace("TO", "6")).message().contains("lo (6) is above hi (5)"));
        for (node, needle, status) in [
            (r#"{"lhs":{"s":"close"},"op":"between","lo":{"value":5},"hi":{"value":2}}"#, "lo (5) is above hi (2)", "invalid"),
            (r#"{"lhs":{"s":"close"},"op":"outside","lo":{"value":5},"hi":{"value":2}}"#, "lo (5) is above hi (2)", "invalid"),
            (r#"{"then":[{"lhs":{"s":"close"},"op":">","rhs":{"value":1}},{"lhs":{"s":"close"},"op":">","rhs":{"value":1}},{"lhs":{"s":"close"},"op":">","rhs":{"value":1}}],"within":1}"#, "within (1) is shorter", "invalid"),
            (r#"{"lhs":{"add":[{"value":1},{"value":2}]},"op":">","rhs":{"value":1}}"#, "combines two numbers", "invalid"),
            (r#"{"lhs":{"abs":{"value":-1}},"op":">","rhs":{"s":"close"}}"#, "entry.all[0].lhs.abs is a number", "invalid"),
            (r#"{"lhs":{"sum":{"value":1},"bars":3},"op":">","rhs":{"s":"close"}}"#, "entry.all[0].lhs.sum is a number", "invalid"),
            (r#"{"lhs":{"ref":{"value":1},"bars":3},"op":">","rhs":{"s":"close"}}"#, "entry.all[0].lhs.ref is a number", "invalid"),
            (r#"{"lhs":{"abs":{"s":"close"},"symbol":"X"},"op":">","rhs":{"value":1}}"#, "arrive in Phase 8", "unsupported"),
        ] {
            let e = err(&ONE_EXIT.replace("NODE", node));
            assert_eq!(e.status(), status, "{node}");
            assert!(e.message().contains(needle), "{needle:?} not in {:?}", e.message());
        }
    }

    // ---- Phase 7: expressions (spec §4.1)
    #[test] fn expressions_parse_validate_and_reach_optimize() {
        let s = parse_spec(&ONE_EXIT.replace("NODE", r#"{"lhs":{"sum":{"s":"rsi","p":{"length":2}},"bars":3},"op":"<","rhs":{"mul":[{"s":"atr"},{"value":2}]}}"#)).unwrap();
        let c = s.entry[0].cmp().unwrap();
        assert!(matches!(&c.lhs, Operand::Expr(e) if matches!(**e, Expr::Win(Win::Sum, Operand::Series(_), 3))));
        assert!(matches!(&c.rhs, Operand::Expr(e) if matches!(**e, Expr::Bin(Bin::Mul, Operand::Series(_), Operand::Value(_)))));
        for (lhs, needle, status) in [
            (r#"{"add":[{"value":1}]}"#, "entry.all[0].lhs.add must be a list of two operands", "invalid"),
            (r#"{"sum":{"s":"close"}}"#, "entry.all[0].lhs.bars must be a whole number from 1", "invalid"),
            (r#"{"sum":{"s":"close"},"bars":3,"x":1}"#, "unknown field 'x' in entry.all[0].lhs", "invalid"),
            (r#"{"ref":{"s":"close"},"bars":-1}"#, "entry.all[0].lhs.bars must be a whole number from 0", "invalid"),
            (r#"{"abs":{"s":"bars_held"}}"#, "only available as a plain comparison", "invalid"),
            (r#"{"s":"close","symbol":"NIFTY BANK"}"#, "arrive in Phase 8", "unsupported"),
        ] {
            let e = err(&ONE_EXIT.replace("NODE", &format!(r#"{{"lhs":{lhs},"op":">","rhs":{{"value":1}}}}"#)));
            assert_eq!(e.status(), status, "{lhs}");
            assert!(e.message().contains(needle), "{needle:?} not in {:?}", e.message());
        }
        let base = ONE_EXIT.replace("NODE", r#"{"lhs":{"sum":{"s":"rsi","p":{"length":2}},"bars":3},"op":"<","rhs":{"add":[{"s":"close"},{"value":5}]}}"#);
        let text = format!(r#"{},"optimize":[{{"path":"entry.all[0].lhs.bars","from":2,"to":4,"step":1}},{{"path":"entry.all[0].lhs.sum.p.length","from":2,"to":3,"step":1}},{{"path":"entry.all[0].rhs.add[1].value","from":1,"to":9,"step":4}}]}}"#, &base[..base.len() - 1]);
        let s = parse_spec(&text).unwrap();
        assert_eq!(s.optimize.iter().map(|d| current(&s, &d.target)).collect::<Vec<_>>(), vec![3., 2., 5.]);
        let t = apply(&s, &s.optimize, &[4., 3., 9.]);
        assert_eq!(crate::lab::series::describe(t.entry[0].cmp().unwrap()), "sum(rsi(length=3), 4) < add(close, 9)");
    }

    #[test] fn state_operands_parse_and_reach_optimize() {
        let base = ONE_EXIT.replace("NODE", r#"{"lhs":{"value_when":{"lhs":{"s":"rsi"},"op":"<","rhs":{"value":30}},"of":{"s":"low"},"nth":2},"op":"<","rhs":{"s":"close"}}"#);
        let text = format!(r#"{},"optimize":[{{"path":"entry.all[0].lhs.value_when.rhs.value","from":20,"to":30,"step":10}},{{"path":"entry.all[0].lhs.nth","from":1,"to":3,"step":1}}]}}"#, &base[..base.len() - 1]);
        let s = parse_spec(&text).unwrap();
        assert_eq!(s.optimize.iter().map(|d| current(&s, &d.target)).collect::<Vec<_>>(), vec![30., 2.]);
        for (lhs, needle) in [
            (r#"{"value_when":{"lhs":{"s":"rsi"},"op":"<","rhs":{"value":30}},"of":{"s":"low"},"nth":21}"#, "entry.all[0].lhs.nth must be a whole number from 1 to 20"),
            (r#"{"count":{"lhs":{"s":"rsi"},"op":"<","rhs":{"value":30}}}"#, "entry.all[0].lhs.bars must be a whole number from 1"),
            (r#"{"bars_since":{"value":1}}"#, "entry.all[0].lhs.bars_since must be a comparison"),
            (r#"{"highest_since":{"lhs":{"s":"close"},"op":">","rhs":{"value":1}}}"#, "entry.all[0].lhs.of is required"),
        ] {
            let e = err(&ONE_EXIT.replace("NODE", &format!(r#"{{"lhs":{lhs},"op":">","rhs":{{"value":1}}}}"#)));
            assert!(e.message().contains(needle), "{needle:?} not in {:?}", e.message());
        }
        // a node inside a state operand stays consistent under optimize
        let nested = ONE_EXIT.replace("NODE", r#"{"lhs":{"bars_since":{"at_least":2,"of":3,"when":{"lhs":{"s":"close"},"op":">","rhs":{"s":"open"}}}},"op":">","rhs":{"value":1}}"#);
        let opt = |to: u32| format!(r#"{},"optimize":[{{"path":"entry.all[0].lhs.bars_since.at_least","from":1,"to":{to},"step":1}}]}}"#, &nested[..nested.len() - 1]);
        assert!(parse_spec(&opt(3)).is_ok());
        assert!(err(&opt(4)).message().contains("at_least (4) cannot exceed of (3)"));
        let vw = ONE_EXIT.replace("NODE", r#"{"lhs":{"value_when":{"lhs":{"s":"rsi"},"op":"<","rhs":{"value":30}},"of":{"s":"low"},"nth":2},"op":"<","rhs":{"s":"close"}}"#);
        let nth = |to: u32| format!(r#"{},"optimize":[{{"path":"entry.all[0].lhs.nth","from":1,"to":{to},"step":1}}]}}"#, &vw[..vw.len() - 1]);
        assert!(parse_spec(&nth(20)).is_ok());
        assert!(err(&nth(21)).message().contains("nth (21) must be a whole number from 1 to 20"));
    }
}
