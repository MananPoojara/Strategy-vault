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
pub struct Target { pub exit: bool, pub cond: usize, pub rhs: bool, pub param: Option<String> }
#[derive(Clone, Debug, PartialEq)]
pub struct Dim { pub path: String, pub target: Target, pub values: Vec<f64> }
#[derive(Clone, Debug, PartialEq)]
pub struct Spec { pub entry: Vec<Cond>, pub exit: Vec<Cond>, pub cost_bps: Option<f64>, pub optimize: Vec<Dim> }

pub fn parse_spec(text: &str) -> Result<Spec, LabError> {
    if text.len() > MAX_SPEC_BYTES { return Err(inv(format!("spec is {} bytes; the limit is {MAX_SPEC_BYTES}", text.len()))); }
    let root = json::parse(text).map_err(|e| inv(format!("spec is not valid JSON: {e}")))?;
    fields(&root, "spec", &["v", "direction", "entry", "exit", "execution", "optimize"], &["short", "portfolio", "scale_in"])?;
    if root.get("v").and_then(Json::as_f64) != Some(1.) { return Err(inv("v must be 1")); }
    match root.get("direction") {
        None => {}
        Some(Json::Str(d)) if d == "long" => {}
        Some(Json::Str(d)) if d == "short" => return Err(LabError::Unsupported("direction 'short' is reserved; v1 simulates long only".into())),
        _ => return Err(inv("direction must be \"long\"")),
    }
    let entry_obj = root.get("entry").ok_or_else(|| inv("entry is required"))?;
    fields(entry_obj, "entry", &["all"], &["limit", "stop", "scale_in"])?;
    let exit_obj = root.get("exit").ok_or_else(|| inv("exit is required"))?;
    fields(exit_obj, "exit", &["any"], &["stop_loss", "target", "trailing"])?;
    let entry = conds(entry_obj.get("all"), "entry.all", false)?;
    let exit = conds(exit_obj.get("any"), "exit.any", true)?;
    if entry.len() + exit.len() > MAX_CONDITIONS {
        return Err(inv(format!("{} conditions; the limit is {MAX_CONDITIONS}", entry.len() + exit.len())));
    }
    let mut cost_bps = None;
    if let Some(ex) = root.get("execution") {
        fields(ex, "execution", &["fill", "cost_bps"], &["limit", "stop"])?;
        match ex.get("fill") {
            None => {}
            Some(Json::Str(f)) if f == "next_open" => {}
            Some(Json::Str(f)) => return Err(LabError::Unsupported(format!("execution.fill '{f}' is reserved; v1 fills at the next open"))),
            _ => return Err(inv("execution.fill must be \"next_open\"")),
        }
        if let Some(c) = ex.get("cost_bps") {
            cost_bps = Some(c.as_f64().filter(|c| (0. ..=5000.).contains(c)).ok_or_else(|| inv("execution.cost_bps must be a number from 0 to 5000"))?);
        }
    }
    let mut spec = Spec { entry, exit, cost_bps, optimize: vec![] };
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

fn conds(v: Option<&Json>, at: &str, exit: bool) -> Result<Vec<Cond>, LabError> {
    let Some(Json::Arr(items)) = v else { return Err(inv(format!("{at} must be a list of conditions"))) };
    if items.is_empty() { return Err(inv(format!("{at} needs at least one condition"))); }
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
        let values = crate::grid(&format!("{}:{}:{}", num("from")?, num("to")?, num("step")?)).map_err(|e| inv(format!("{at}: {e}")))?;
        for &x in &values { check_value(spec, &target, x).map_err(|e| inv(format!("{at}: {e}")))?; }
        combos = combos.saturating_mul(values.len());
        if combos > crate::MAX_COMBOS { return Err(inv(format!("optimize gives more than {} combinations; narrow a range or raise a step", crate::MAX_COMBOS))); }
        out.push(Dim { path: path.into(), target, values });
    }
    Ok(out)
}

fn target(spec: &Spec, path: &str) -> Result<Target, String> {
    let (exit, rest) = match (path.strip_prefix("entry.all["), path.strip_prefix("exit.any[")) {
        (Some(r), _) => (false, r),
        (_, Some(r)) => (true, r),
        _ => return Err("must start with entry.all[i] or exit.any[i]".into()),
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
        (Operand::Value(_), "value") => Ok(Target { exit, cond, rhs, param: None }),
        (Operand::Series(s), f) if f.starts_with("p.") => {
            let def = catalogue::get(&s.id).ok_or("price sources have no parameters")?;
            let key = &f[2..];
            if !def.params.iter().any(|p| p.key == key) { return Err(format!("{} has no parameter '{key}'", def.id)); }
            Ok(Target { exit, cond, rhs, param: Some(key.into()) })
        }
        _ => Err("must end in .value (a number operand) or .p.<key> (an indicator parameter)".into()),
    }
}

fn operand_mut<'a>(s: &'a mut Spec, t: &Target) -> &'a mut Operand {
    let c = if t.exit { &mut s.exit[t.cond] } else { &mut s.entry[t.cond] };
    if t.rhs { &mut c.rhs } else { &mut c.lhs }
}
fn set(s: &mut Spec, t: &Target, x: f64) {
    match (operand_mut(s, t), &t.param) {
        (Operand::Value(v), None) => *v = x,
        (Operand::Series(r), Some(k)) => { r.params.insert(k.clone(), x); }
        _ => {}
    }
}
fn check_value(spec: &Spec, t: &Target, x: f64) -> Result<(), String> {
    let mut s = spec.clone();
    set(&mut s, t, x);
    match operand_mut(&mut s, t) {
        Operand::Series(r) => catalogue::resolve_params(&catalogue::get(&r.id).ok_or("unknown series")?, &r.params).map(|_| ()),
        _ => Ok(()),
    }
}
/// The spec's own value at an optimize target (the "base" setting the ranking prefers on ties).
pub fn current(spec: &Spec, t: &Target) -> f64 {
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
            (with(r#""direction": "long""#, r#""direction": "short""#), "short"),
            (with(r#""v": 1,"#, r#""v": 1, "portfolio": {"max_open": 3},"#), "spec.portfolio"),
            (with(r#""all": ["#, r#""limit": {"value": 1}, "all": ["#), "entry.limit"),
            (with(r#""any": ["#, r#""stop_loss": {"pct": 2}, "any": ["#), "exit.stop_loss"),
            (with(r#""any": ["#, r#""trailing": {"pct": 2}, "any": ["#), "exit.trailing"),
            (with(r#""fill": "next_open""#, r#""fill": "limit""#), "execution.fill 'limit'"),
            (with(r#""v": 1,"#, r#""v": 1, "scale_in": [],"#), "spec.scale_in"),
        ] {
            let e = err(&text);
            assert_eq!(e.status(), "unsupported", "{text}: {e:?}");
            assert!(e.message().contains(needle), "{needle:?} not in {:?}", e.message());
        }
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
    #[test] fn engine_errors_map_to_types() {
        assert_eq!(LabError::from("needs_data: no VIX".to_string()), LabError::NeedsData("no VIX".into()));
        assert_eq!(LabError::from("empty date range".to_string()).status(), "invalid");
        let j = LabError::Invalid("bad \"x\"\n".into()).json();
        assert!(j.starts_with("{\"status\":\"invalid\",\"error\":\"bad \\\"x\\\"\\u000a\",\"label\":\"User-defined indicator strategy"), "{j}");
    }
}
