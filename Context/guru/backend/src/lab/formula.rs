//! Custom indicator formulas (design 2026-10-19 §1): a small AmiBroker-flavoured language evaluated on lab bars.
//! Every function is causal (bar i reads bars <= i only); a positive Ref (future) is refused. NaN propagates through
//! arithmetic (warm-up); comparisons with NaN are false. Errors read `line L, col C: message`.
use super::bars::Bars;
use super::catalogue::{self, IndicatorDef, Params};
use super::json::esc;
use super::rules::LabError;
use super::ta;
use std::collections::BTreeMap;
use std::sync::OnceLock;

pub const MAX_CHARS: usize = 4000;
pub const MAX_NODES: usize = 300;
pub const MAX_PLOTS: usize = 8;
pub const MAX_PARAMS: usize = 16;
pub const MAX_LOOKBACK: usize = 2000;
pub const MAX_NAME: usize = 60;
pub const DEFAULT_NAME: &str = "Custom";
/// Distinct catalogue indicator calls per formula.
pub const MAX_CALLS: usize = 20;
/// Window work (bars × length, summed over window functions) one evaluation may spend: no formula can hang the engine.
const MAX_WORK: usize = 300_000_000;
const MAX_DEPTH: usize = 64;
const MAX_NTH: usize = 20;
const MAX_LABEL: usize = 40;
const NAN: f64 = f64::NAN;

/// (name, args, description) for the editor's help list (`/api/lab/catalogue` formula_functions).
pub const FUNCTIONS: &[(&str, &str, &str)] = &[
    ("Param", "\"Label\", default, min, max, step", "A tunable number; its key is the label. At most 16."),
    ("Plot", "x, \"Name\"[, \"price\"]", "One output (1 to 8). \"price\" draws the indicator on the price chart."),
    ("MA", "x, n", "Simple moving average of x over n bars (alias SMA)."),
    ("EMA", "x, n", "Exponential moving average, seeded with the SMA of the first n values."),
    ("WMA", "x, n", "Linearly weighted moving average."),
    ("Wilders", "x, n", "Wilder's smoothing, alpha 1/n (alias RMA)."),
    ("StDev", "x, n", "Population standard deviation over n bars."),
    ("Sum", "x, n", "Sum of the last n values."),
    ("HHV", "x, n", "Highest value of the last n bars (alias Highest)."),
    ("LLV", "x, n", "Lowest value of the last n bars (alias Lowest)."),
    ("ROC", "x, n", "Percent change over n bars: 100 × (x − x[n]) / x[n]."),
    ("Change", "x, n", "x − x[n] (alias Mom)."),
    ("LinRegSlope", "x, n", "Slope of the least-squares line through the last n values."),
    ("AMA", "x, alpha", "Adaptive moving average: alpha × x + (1 − alpha) × previous; alpha 0 to 1 (number or series)."),
    ("Ref", "x, -n", "x n bars ago (same as x[n]); a positive offset would read the future and is refused."),
    ("Abs", "x", "Absolute value."),
    ("Sqrt", "x", "Square root (NaN below 0)."),
    ("Log", "x", "Natural logarithm (NaN at or below 0)."),
    ("Exp", "x", "e to the power x."),
    ("Max", "a, b", "Larger of a and b, bar by bar."),
    ("Min", "a, b", "Smaller of a and b, bar by bar."),
    ("IIf", "cond, a, b", "a where cond is true, otherwise b."),
    ("Cross", "a, b", "1 on the bar a crosses above b, otherwise 0."),
    ("BarsSince", "cond", "Bars since cond was last true (NaN before the first time)."),
    ("ValueWhen", "cond, x[, nth]", "x on the nth most recent bar where cond was true (nth 1 to 20)."),
    ("Nz", "x[, v]", "x with missing values replaced by v (default 0)."),
    ("Cum", "x", "Running total of x from the first bar; a missing value adds nothing (NaN until the first value)."),
    ("Round", "x", "Nearest whole number (halves away from zero)."),
    ("Int", "x", "Whole-number part, towards zero."),
    ("Floor", "x", "Largest whole number at or below x."),
    ("Ceil", "x", "Smallest whole number at or above x."),
    ("Frac", "x", "Fractional part: x − Int(x)."),
    ("Sign", "x", "1 above 0, −1 below 0, 0 at 0."),
    ("HighestSince", "cond, x", "Highest x since the latest bar where cond was true, that bar included (NaN before; a missing x or cond since then makes it unknown)."),
    ("LowestSince", "cond, x", "Lowest x since the latest bar where cond was true, that bar included (as HighestSince)."),
    ("ExRem", "a, b", "1 on the first bar a is true, then 0 until b is true on a later bar (removes repeated signals). Same-bar a and b: a starts a signal; b ends one only on a later bar (AmiBroker's same-bar rule is not documented; not yet confirmed)."),
    ("Flip", "a, b", "1 from the bar a is true until the bar b is true (a latch). Same bar a and b: 0, b wins (AmiBroker's same-bar rule is not documented; not yet confirmed)."),
    ("<indicator>", "args…, name=value", "Any computed catalogue indicator by id, e.g. rsi(14), macd(fast=8).signal, Supertrend(10, 3).direction."),
];
/// (name, min args, max args). Param and Plot are handled by the parser; the rest evaluate bar by bar.
const BUILTINS: &[(&str, usize, usize)] = &[
    ("ma", 2, 2), ("sma", 2, 2), ("ema", 2, 2), ("wma", 2, 2), ("wilders", 2, 2), ("rma", 2, 2), ("stdev", 2, 2), ("sum", 2, 2),
    ("hhv", 2, 2), ("highest", 2, 2), ("llv", 2, 2), ("lowest", 2, 2), ("roc", 2, 2), ("change", 2, 2), ("mom", 2, 2),
    ("linregslope", 2, 2), ("ama", 2, 2), ("abs", 1, 1), ("sqrt", 1, 1), ("log", 1, 1), ("exp", 1, 1), ("max", 2, 2), ("min", 2, 2),
    ("iif", 3, 3), ("cross", 2, 2), ("barssince", 1, 1), ("valuewhen", 2, 3), ("nz", 1, 2), ("ref", 2, 2), ("param", 5, 5), ("plot", 2, 3),
    ("cum", 1, 1), ("round", 1, 1), ("int", 1, 1), ("floor", 1, 1), ("ceil", 1, 1), ("frac", 1, 1), ("sign", 1, 1),
    ("highestsince", 2, 2), ("lowestsince", 2, 2), ("exrem", 2, 2), ("flip", 2, 2),
];
/// Query keys of /api/lab/series, which carries Param values as bare keys (`&Fast=9`).
pub const RESERVED_KEYS: &[&str] = &["ticker", "tf", "formula", "name", "id", "symbol", "vs", "fixup", "from", "to", "market"];
const RESERVED: &[&str] = &["o", "h", "l", "c", "v", "open", "high", "low", "close", "volume", "avg", "hl2", "hlc3", "ohlc4", "true", "false", "and", "or", "not"];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Pos { pub line: usize, pub col: usize }
fn at(p: Pos, m: impl std::fmt::Display) -> String { format!("line {}, col {}: {m}", p.line, p.col) }
fn inv(p: Pos, m: impl std::fmt::Display) -> LabError { LabError::Invalid(at(p, m)) }

#[derive(Clone, Debug, PartialEq)]
enum Tok { Num(f64), Id(String), Str(String), Sym(&'static str), End }
const SYMS: &[&str] = &["==", "!=", ">=", "<=", "&&", "||", "(", ")", "[", "]", ",", ".", "=", ">", "<", "+", "-", "*", "/", "^", "!"];

/// `;` and newlines outside brackets end a statement (Tok::End); `//` starts a comment.
fn lex(text: &str) -> Result<Vec<(Tok, Pos)>, String> {
    let cs: Vec<char> = text.chars().collect();
    let (mut i, mut line, mut col, mut depth) = (0usize, 1usize, 1usize, 0i64);
    let mut out = Vec::new();
    while i < cs.len() {
        let (c, p) = (cs[i], Pos { line, col });
        if c == '\n' {
            if depth <= 0 { out.push((Tok::End, p)); }
            i += 1; line += 1; col = 1;
            continue;
        }
        if c.is_whitespace() { i += 1; col += 1; continue; }
        if c == '/' && cs.get(i + 1) == Some(&'/') { while i < cs.len() && cs[i] != '\n' { i += 1; col += 1; } continue; }
        let start = i;
        if c.is_ascii_digit() || (c == '.' && cs.get(i + 1).is_some_and(char::is_ascii_digit)) {
            while i < cs.len() && (cs[i].is_ascii_digit() || cs[i] == '.') { i += 1; }
            let exp = |k: usize| cs.get(k).is_some_and(char::is_ascii_digit);
            if i < cs.len() && matches!(cs[i], 'e' | 'E') && (exp(i + 1) || (matches!(cs.get(i + 1), Some('+' | '-')) && exp(i + 2))) {
                i += 2;
                while i < cs.len() && cs[i].is_ascii_digit() { i += 1; }
            }
            let t: String = cs[start..i].iter().collect();
            let v = t.parse::<f64>().ok().filter(|v| v.is_finite()).ok_or_else(|| at(p, format!("bad number {t}")))?;
            out.push((Tok::Num(v), p));
        } else if c.is_ascii_alphabetic() || c == '_' {
            while i < cs.len() && (cs[i].is_ascii_alphanumeric() || cs[i] == '_') { i += 1; }
            out.push((Tok::Id(cs[start..i].iter().collect()), p));
        } else if c == '"' {
            i += 1;
            while i < cs.len() && cs[i] != '"' && cs[i] != '\n' { i += 1; }
            if cs.get(i) != Some(&'"') { return Err(at(p, "text is missing its closing \"")); }
            out.push((Tok::Str(cs[start + 1..i].iter().collect()), p));
            i += 1;
        } else if c == ';' {
            out.push((Tok::End, p));
            i += 1;
        } else {
            let s = SYMS.iter().find(|s| s.chars().enumerate().all(|(k, sc)| cs.get(i + k) == Some(&sc)))
                .ok_or_else(|| at(p, format!("unexpected character '{c}'")))?;
            match *s { "(" | "[" => depth += 1, ")" | "]" => depth -= 1, _ => {} }
            out.push((Tok::Sym(s), p));
            i += s.len();
        }
        col += i - start;
    }
    out.push((Tok::End, Pos { line, col }));
    Ok(out)
}

#[derive(Clone, Debug)]
pub struct E { pos: Pos, ex: Ex }
#[derive(Clone, Debug)]
enum Ex {
    Num(f64), Str(String), Name(String),
    Call { name: String, args: Vec<Arg>, out: Option<(String, Pos)> },
    Neg(Box<E>), Not(Box<E>), Bin(&'static str, Box<E>, Box<E>), Index(Box<E>, Box<E>),
}
#[derive(Clone, Debug)]
struct Arg { name: Option<String>, e: E }
#[derive(Clone, Debug)]
enum Stmt { Let(String, E, Pos), Plot(E, String, bool, Pos) }

struct Parser { t: Vec<(Tok, Pos)>, k: usize, nodes: usize, depth: usize }
type R<T> = Result<T, String>;
fn tok_text(t: &Tok, last: bool) -> String {
    match t {
        Tok::Num(v) => format!("{v}"), Tok::Id(s) => s.clone(), Tok::Str(s) => format!("\"{s}\""), Tok::Sym(s) => format!("'{s}'"),
        Tok::End => (if last { "the end of the formula" } else { "the end of the statement" }).into(),
    }
}
impl Parser {
    fn peek(&self) -> &Tok { &self.t[self.k].0 }
    fn pos(&self) -> Pos { self.t[self.k].1 }
    fn last(&self) -> bool { self.k + 1 >= self.t.len() }
    fn bump(&mut self) -> (Tok, Pos) { let x = self.t[self.k].clone(); if !self.last() { self.k += 1; } x }
    fn is(&self, s: &str) -> bool { matches!(self.peek(), Tok::Sym(x) if *x == s) }
    fn word(&self, w: &str) -> bool { matches!(self.peek(), Tok::Id(x) if x.eq_ignore_ascii_case(w)) }
    fn eat(&mut self, s: &str) -> bool { let y = self.is(s); if y { self.bump(); } y }
    fn found(&self) -> String { tok_text(self.peek(), self.last()) }
    fn expect(&mut self, s: &str) -> R<()> { if self.eat(s) { Ok(()) } else { Err(at(self.pos(), format!("expected '{s}' but found {}", self.found()))) } }
    fn node(&mut self, pos: Pos, ex: Ex) -> R<E> {
        self.nodes += 1;
        if self.nodes > MAX_NODES { return Err(at(pos, format!("the formula has more than {MAX_NODES} expression parts; the limit is {MAX_NODES}"))); }
        Ok(E { pos, ex })
    }
    /// Runs `f` one nesting level deeper; deep nesting is refused before it can exhaust the stack.
    fn nested(&mut self, f: fn(&mut Parser) -> R<E>) -> R<E> {
        self.depth += 1;
        if self.depth > MAX_DEPTH { return Err(at(self.pos(), format!("expressions nest deeper than {MAX_DEPTH} levels"))); }
        let r = f(self);
        self.depth -= 1;
        r
    }
    fn program(&mut self) -> R<Vec<Stmt>> {
        let mut out = Vec::new();
        loop {
            while matches!(self.peek(), Tok::End) && !self.last() { self.k += 1; }
            if self.last() { return Ok(out); }
            out.push(self.statement()?);
            if !matches!(self.peek(), Tok::End) { return Err(at(self.pos(), format!("expected ';' or a new line but found {}", self.found()))); }
        }
    }
    fn statement(&mut self) -> R<Stmt> {
        let pos = self.pos();
        if let (Tok::Id(name), Some((Tok::Sym("="), _))) = (self.peek().clone(), self.t.get(self.k + 1)) {
            self.k += 2;
            return Ok(Stmt::Let(name, self.expr()?, pos));
        }
        let e = self.expr()?;
        let Ex::Call { name, args, out } = e.ex else { return Err(at(pos, "a statement is `name = expression` or `Plot(expression, \"Name\")`")) };
        if !name.eq_ignore_ascii_case("plot") { return Err(at(pos, "a statement is `name = expression` or `Plot(expression, \"Name\")`")); }
        let form = || at(pos, "Plot takes (expression, \"Name\") or (expression, \"Name\", \"price\")");
        if !(2..=3).contains(&args.len()) || args.iter().any(|a| a.name.is_some()) || out.is_some() { return Err(form()); }
        let text = |a: &Arg| if let Ex::Str(s) = &a.e.ex { Some(s.trim().to_string()) } else { None };
        let plot = text(&args[1]).ok_or_else(form)?;
        if plot.is_empty() || plot.chars().count() > MAX_LABEL { return Err(at(args[1].e.pos, format!("a Plot name needs 1 to {MAX_LABEL} characters"))); }
        let price = match args.get(2) {
            None => false,
            Some(a) if text(a).is_some_and(|s| s.eq_ignore_ascii_case("price")) => true,
            Some(a) => return Err(at(a.e.pos, "the third Plot argument may only be \"price\"")),
        };
        Ok(Stmt::Plot(args.into_iter().next().unwrap().e, plot, price, pos))
    }
    fn expr(&mut self) -> R<E> { self.nested(Parser::or) }
    fn or(&mut self) -> R<E> {
        let mut l = self.and()?;
        loop {
            let pos = self.pos();
            if !(self.word("or") || self.is("||")) { return Ok(l); }
            self.bump();
            let r = self.and()?;
            l = self.node(pos, Ex::Bin("or", Box::new(l), Box::new(r)))?;
        }
    }
    fn and(&mut self) -> R<E> {
        let mut l = self.not()?;
        loop {
            let pos = self.pos();
            if !(self.word("and") || self.is("&&")) { return Ok(l); }
            self.bump();
            let r = self.not()?;
            l = self.node(pos, Ex::Bin("and", Box::new(l), Box::new(r)))?;
        }
    }
    fn not(&mut self) -> R<E> {
        let pos = self.pos();
        if !(self.word("not") || self.is("!")) { return self.cmp(); }
        self.bump();
        let x = self.nested(Parser::not)?;
        self.node(pos, Ex::Not(Box::new(x)))
    }
    fn cmp(&mut self) -> R<E> {
        const OPS: [&str; 6] = [">=", "<=", "==", "!=", ">", "<"];
        let l = self.add()?;
        let pos = self.pos();
        if self.is("=") { return Err(at(pos, "use == to compare (= names a value at the start of a statement)")); }
        let Some(op) = OPS.into_iter().find(|o| self.is(o)) else { return Ok(l) };
        self.bump();
        let r = self.add()?;
        if OPS.iter().any(|o| self.is(o)) { return Err(at(self.pos(), "comparisons do not chain; join them with AND")); }
        self.node(pos, Ex::Bin(op, Box::new(l), Box::new(r)))
    }
    fn add(&mut self) -> R<E> {
        let mut l = self.mul()?;
        loop {
            let pos = self.pos();
            let Some(op) = ["+", "-"].into_iter().find(|o| self.is(o)) else { return Ok(l) };
            self.bump();
            let r = self.mul()?;
            l = self.node(pos, Ex::Bin(op, Box::new(l), Box::new(r)))?;
        }
    }
    fn mul(&mut self) -> R<E> {
        let mut l = self.unary()?;
        loop {
            let pos = self.pos();
            let Some(op) = ["*", "/"].into_iter().find(|o| self.is(o)) else { return Ok(l) };
            self.bump();
            let r = self.unary()?;
            l = self.node(pos, Ex::Bin(op, Box::new(l), Box::new(r)))?;
        }
    }
    fn unary(&mut self) -> R<E> {
        let pos = self.pos();
        if !(self.is("-") || self.is("+")) { return self.pow(); }
        let neg = self.is("-");
        self.bump();
        let x = self.nested(Parser::unary)?;
        if neg { self.node(pos, Ex::Neg(Box::new(x))) } else { Ok(x) }
    }
    /// `^` binds tighter than unary minus (-2^2 = -4) and is right-associative.
    fn pow(&mut self) -> R<E> {
        let l = self.postfix()?;
        let pos = self.pos();
        if !self.eat("^") { return Ok(l); }
        let r = self.nested(Parser::unary)?;
        self.node(pos, Ex::Bin("^", Box::new(l), Box::new(r)))
    }
    fn postfix(&mut self) -> R<E> {
        let mut e = self.primary()?;
        loop {
            let pos = self.pos();
            if self.eat("[") {
                let n = self.expr()?;
                self.expect("]")?;
                e = self.node(pos, Ex::Index(Box::new(e), Box::new(n)))?;
            } else if self.eat(".") {
                let (t, p) = self.bump();
                let Tok::Id(o) = t else { return Err(at(p, "expected an output name after '.'")) };
                match &mut e.ex {
                    Ex::Call { out, .. } if out.is_none() => *out = Some((o, p)),
                    _ => return Err(at(pos, ".output follows an indicator call, e.g. Supertrend(10, 3).direction")),
                }
            } else {
                return Ok(e);
            }
        }
    }
    fn primary(&mut self) -> R<E> {
        let last = self.last();
        let (t, pos) = self.bump();
        match t {
            Tok::Num(v) => self.node(pos, Ex::Num(v)),
            Tok::Str(s) => self.node(pos, Ex::Str(s)),
            Tok::Sym("(") => { let e = self.expr()?; self.expect(")")?; Ok(e) }
            Tok::Id(name) if ["and", "or"].contains(&name.to_ascii_lowercase().as_str()) => Err(at(pos, format!("{name} needs a value on its left"))),
            Tok::Id(name) => {
                if !self.eat("(") { return self.node(pos, Ex::Name(name)); }
                let mut args = Vec::new();
                if !self.eat(")") {
                    loop {
                        let named = match (self.peek().clone(), self.t.get(self.k + 1)) {
                            (Tok::Id(n), Some((Tok::Sym("="), _))) => { self.k += 2; Some(n) }
                            _ => None,
                        };
                        args.push(Arg { name: named, e: self.expr()? });
                        if self.eat(",") { continue; }
                        self.expect(")")?;
                        break;
                    }
                }
                self.node(pos, Ex::Call { name, args, out: None })
            }
            t => Err(at(pos, format!("unexpected {}", tok_text(&t, last)))),
        }
    }
}

/// One `Param()`: its key is the label.
#[derive(Clone, Debug, PartialEq)]
pub struct ParamDef { pub key: String, pub default: f64, pub min: f64, pub max: f64, pub step: f64 }
impl ParamDef { fn whole(&self) -> bool { self.step.fract() == 0. } }
/// A parsed, validated formula: statements, its Params in source order, its Plot names and placement.
#[derive(Clone, Debug)]
pub struct Program { stmts: Vec<Stmt>, pub params: Vec<ParamDef>, pub plots: Vec<String>, pub price: bool }

fn visit<'a>(e: &'a E, f: &mut dyn FnMut(&'a E)) {
    f(e);
    match &e.ex {
        Ex::Call { args, .. } => for a in args { visit(&a.e, f) },
        Ex::Neg(x) | Ex::Not(x) => visit(x, f),
        Ex::Bin(_, a, b) | Ex::Index(a, b) => { visit(a, f); visit(b, f) }
        Ex::Num(_) | Ex::Str(_) | Ex::Name(_) => {}
    }
}
fn is_call(e: &E, name: &str) -> bool { matches!(&e.ex, Ex::Call { name: n, .. } if n.eq_ignore_ascii_case(name)) }
fn param_def(e: &E) -> R<ParamDef> {
    let Ex::Call { args, out, .. } = &e.ex else { unreachable!() };
    let form = || at(e.pos, "Param takes (\"Label\", default, min, max, step) with plain numbers");
    if args.len() != 5 || args.iter().any(|a| a.name.is_some()) || out.is_some() { return Err(form()); }
    let Ex::Str(label) = &args[0].e.ex else { return Err(form()) };
    let num = |a: &Arg| match &a.e.ex { Ex::Num(v) => Some(*v), Ex::Neg(x) => if let Ex::Num(v) = x.ex { Some(-v) } else { None }, _ => None };
    let v: Vec<f64> = args[1..].iter().map(num).collect::<Option<_>>().ok_or_else(form)?;
    let key = label.trim().to_string();
    if key.is_empty() || key.chars().count() > MAX_LABEL || !key.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == ' ') {
        return Err(at(args[0].e.pos, format!("a Param label needs 1 to {MAX_LABEL} letters, digits, spaces or _")));
    }
    if RESERVED_KEYS.iter().any(|r| r.eq_ignore_ascii_case(&key)) { return Err(at(args[0].e.pos, format!("Param name \"{key}\" is reserved"))); }
    let d = ParamDef { key, default: v[0], min: v[1], max: v[2], step: v[3] };
    if v.iter().any(|x| x.abs() > 1e6) { return Err(at(e.pos, format!("Param \"{}\": numbers must be within ±1000000", d.key))); }
    if !(d.step > 0.) || d.min > d.max || d.default < d.min || d.default > d.max {
        return Err(at(e.pos, format!("Param \"{}\" needs min <= default <= max and step > 0", d.key)));
    }
    if d.whole() && [d.default, d.min, d.max].iter().any(|x| x.fract() != 0.) {
        return Err(at(e.pos, format!("Param \"{}\" has a whole-number step, so default, min and max must be whole numbers", d.key)));
    }
    Ok(d)
}

/// Parses and validates a formula (a dry run with default params checks names, arities, lengths and indicator calls).
pub fn parse(text: &str) -> Result<Program, String> {
    let start = Pos { line: 1, col: 1 };
    let chars = text.chars().count();
    if chars > MAX_CHARS { return Err(at(start, format!("the formula has {chars} characters; the limit is {MAX_CHARS}"))); }
    let mut p = Parser { t: lex(text)?, k: 0, nodes: 0, depth: 0 };
    let stmts = p.program()?;
    let mut prog = Program { stmts, params: Vec::new(), plots: Vec::new(), price: false };
    let mut calls = Vec::new();
    for s in &prog.stmts {
        let (Stmt::Let(_, e, _) | Stmt::Plot(e, ..)) = s;
        visit(e, &mut |x| if is_call(x, "param") { calls.push(x) });
    }
    for e in calls {
        let d = param_def(e)?;
        if prog.params.iter().any(|q| q.key.eq_ignore_ascii_case(&d.key)) { return Err(at(e.pos, format!("Param \"{}\" is declared twice", d.key))); }
        if prog.params.len() == MAX_PARAMS { return Err(at(e.pos, format!("more than {MAX_PARAMS} Params; the limit is {MAX_PARAMS}"))); }
        prog.params.push(d);
    }
    for s in &prog.stmts {
        let Stmt::Plot(_, name, price, pos) = s else { continue };
        if prog.plots.iter().any(|q| q.eq_ignore_ascii_case(name)) { return Err(at(*pos, format!("Plot \"{name}\" is used twice"))); }
        if prog.plots.len() == MAX_PLOTS { return Err(at(*pos, format!("more than {MAX_PLOTS} Plots; the limit is {MAX_PLOTS}"))); }
        prog.plots.push(name.clone());
        prog.price |= price;
    }
    if prog.plots.is_empty() { return Err(at(start, "the formula needs at least one Plot(expression, \"Name\")")); }
    check(&prog, &resolve(&prog, &Params::new())?)?;
    Ok(prog)
}
/// Param values with overrides: unknown keys are refused, values stay within min..max and whole when the step is whole.
pub fn resolve(prog: &Program, given: &Params) -> Result<Params, String> {
    if let Some(k) = given.keys().find(|k| !prog.params.iter().any(|p| &p.key == *k)) {
        let keys: Vec<&str> = prog.params.iter().map(|p| p.key.as_str()).collect();
        return Err(format!("unknown parameter '{k}'{}", if keys.is_empty() { " (the formula has no Params)".into() } else { format!("; parameters: {}", keys.join(", ")) }));
    }
    prog.params.iter().map(|p| {
        let v = given.get(&p.key).copied().unwrap_or(p.default);
        if !v.is_finite() || v < p.min || v > p.max { return Err(format!("{} must be between {} and {}", p.key, p.min, p.max)); }
        if p.whole() && v.fract() != 0. { return Err(format!("{} must be a whole number", p.key)); }
        Ok((p.key.clone(), v))
    }).collect()
}
/// Validates resolved params against every use (lengths, offsets, indicator parameters) without bars.
pub fn check(prog: &Program, params: &Params) -> Result<(), String> {
    let empty = Bars { volume: Some(Vec::new()), ..Bars::default() };
    run(prog, &empty, params, true).map(|_| ()).map_err(|e| e.message().to_string())
}
/// One series per Plot, aligned to `bars`. `params` come from `resolve`.
pub fn eval(prog: &Program, bars: &Bars, params: &Params) -> Result<Vec<Vec<f64>>, LabError> { run(prog, bars, params, false) }

fn run(prog: &Program, bars: &Bars, params: &Params, check: bool) -> Result<Vec<Vec<f64>>, LabError> {
    let mut cx = Cx { bars, params, vars: Vec::new(), check, work: 0, calls: Vec::new() };
    let mut plots = Vec::new();
    for s in &prog.stmts {
        match s {
            Stmt::Let(name, e, pos) => {
                let lc = name.to_ascii_lowercase();
                if RESERVED.contains(&lc.as_str()) { return Err(inv(*pos, format!("{name} is a built-in name; choose another"))); }
                if cx.vars.iter().any(|(n, _)| *n == lc) { return Err(inv(*pos, format!("{name} is already defined; each name is assigned once"))); }
                let v = cx.eval(e)?;
                cx.vars.push((lc, v));
            }
            Stmt::Plot(e, ..) => { let v = cx.eval(e)?; plots.push(cx.ser(v)); }
        }
    }
    Ok(plots)
}

/// A constant (numbers, Params and arithmetic on them) or a series aligned to the bars.
#[derive(Clone, Debug)]
enum V { K(f64), S(Vec<f64>) }
fn fin(x: f64) -> f64 { if x.is_finite() { x } else { NAN } }
fn truth(t: bool) -> f64 { if t { 1. } else { 0. } }
fn on(x: f64) -> bool { x != 0. && !x.is_nan() }
fn map1(v: V, f: impl Fn(f64) -> f64) -> V { match v { V::K(x) => V::K(fin(f(x))), V::S(s) => V::S(s.into_iter().map(|x| fin(f(x))).collect()) } }
fn zip(a: V, b: V, f: impl Fn(f64, f64) -> f64) -> V {
    match (a, b) {
        (V::K(x), V::K(y)) => V::K(fin(f(x, y))),
        (V::S(x), V::K(y)) => V::S(x.into_iter().map(|x| fin(f(x, y))).collect()),
        (V::K(x), V::S(y)) => V::S(y.into_iter().map(|y| fin(f(x, y))).collect()),
        (V::S(x), V::S(y)) => V::S(x.iter().zip(&y).map(|(&x, &y)| fin(f(x, y))).collect()),
    }
}
fn shift(v: V, n: usize) -> V {
    match v { V::K(x) => V::K(x), V::S(s) => V::S((0..s.len()).map(|i| if i >= n { s[i - n] } else { NAN }).collect()) }
}
fn bin(op: &str, a: V, b: V) -> V {
    let f: fn(f64, f64) -> f64 = match op {
        "+" => |x, y| x + y, "-" => |x, y| x - y, "*" => |x, y| x * y, "/" => |x, y| x / y, "^" => f64::powf,
        ">" => |x, y| truth(x > y), "<" => |x, y| truth(x < y), ">=" => |x, y| truth(x >= y), "<=" => |x, y| truth(x <= y),
        "==" => |x, y| truth(x == y), "!=" => |x, y| truth(!x.is_nan() && !y.is_nan() && x != y),
        "and" => |x, y| if x.is_nan() || y.is_nan() { NAN } else { truth(x != 0. && y != 0.) },
        _ => |x, y| if x.is_nan() || y.is_nan() { NAN } else { truth(x != 0. || y != 0.) },
    };
    zip(a, b, f)
}
fn roc(x: &[f64], n: usize) -> Vec<f64> { (0..x.len()).map(|i| if i >= n { fin(100. * (x[i] - x[i - n]) / x[i - n]) } else { NAN }).collect() }
/// Least-squares slope over the last n bars (bar-counted: a NaN in the window gives NaN).
fn slope(x: &[f64], n: usize) -> Vec<f64> {
    (0..x.len()).map(|i| {
        if i + 1 < n { return NAN; }
        let w = &x[i + 1 - n..=i];
        if w.iter().any(|v| v.is_nan()) { return NAN; }
        let nf = n as f64;
        let (mut sk, mut sy, mut skk, mut sky) = (0., 0., 0., 0.);
        for (j, &y) in w.iter().enumerate() { let k = (j + 1) as f64; sk += k; sy += y; skk += k * k; sky += k * y; }
        let den = nf * skk - sk * sk;
        if den == 0. { 0. } else { (nf * sky - sk * sy) / den }
    }).collect()
}

/// Catalogue ids and their AmiBroker / Pine (`ta.x`) names; an alias naming several ids is ambiguous.
fn aliases() -> &'static BTreeMap<String, Vec<&'static str>> {
    static A: OnceLock<BTreeMap<String, Vec<&'static str>>> = OnceLock::new();
    A.get_or_init(|| {
        let mut m: BTreeMap<String, Vec<&'static str>> = BTreeMap::new();
        for (id, tv, ab) in super::sources::rows().copied() {
            let ab_names = ab.split(';').map(str::trim);
            let tv_names = tv.split(';').filter_map(|s| s.trim().strip_prefix("ta."));
            for s in ab_names.chain(tv_names) {
                let w: String = s.chars().take_while(|c| c.is_ascii_alphanumeric() || *c == '_').collect();
                if w.is_empty() || !w.starts_with(|c: char| c.is_ascii_alphabetic()) { continue; }
                let ids = m.entry(w.to_ascii_lowercase()).or_default();
                if !ids.contains(&id) { ids.push(id); }
            }
        }
        m
    })
}
/// The catalogue indicator a call names: its id, else an unambiguous alias.
fn indicator(lc: &str) -> Result<Option<IndicatorDef>, String> {
    if let Some(d) = catalogue::get(lc) { return Ok(Some(d)); }
    match aliases().get(lc).map(Vec::as_slice) {
        Some([id]) => Ok(catalogue::get(id)),
        Some(ids) if ids.len() > 1 => Err(format!("{lc} could mean {}; call the indicator by its id", ids.join(", "))),
        _ => Ok(None),
    }
}
/// A catalogue error at a call site: same status and wording as an indicator operand, prefixed with the position.
fn located(p: Pos, e: LabError) -> LabError {
    match e { LabError::Invalid(m) => inv(p, m), LabError::Unsupported(m) => LabError::Unsupported(at(p, m)), LabError::NeedsData(m) => LabError::NeedsData(at(p, m)) }
}

struct Cx<'a> { bars: &'a Bars, params: &'a Params, vars: Vec<(String, V)>, check: bool, work: usize, calls: Vec<(String, Vec<Vec<f64>>)> }
impl Cx<'_> {
    fn ser(&self, v: V) -> Vec<f64> { match v { V::S(s) => s, V::K(k) => vec![k; self.bars.len()] } }
    fn series(&mut self, e: &E) -> Result<Vec<f64>, LabError> { let v = self.eval(e)?; Ok(self.ser(v)) }
    fn konst(&mut self, e: &E, what: &str) -> Result<f64, LabError> {
        match self.eval(e)? { V::K(x) => Ok(x), V::S(_) => Err(inv(e.pos, format!("{what} must be a number or a Param, not a series"))) }
    }
    fn whole(&mut self, e: &E, min: usize, what: &str) -> Result<usize, LabError> {
        let x = self.konst(e, what)?;
        if x.fract() != 0. || x < min as f64 || x > MAX_LOOKBACK as f64 { return Err(inv(e.pos, format!("{what} must be a whole number from {min} to {MAX_LOOKBACK}, not {x}"))); }
        Ok(x as usize)
    }
    fn spend(&mut self, p: Pos, n: usize) -> Result<(), LabError> {
        self.work = self.work.saturating_add(self.bars.len().saturating_mul(n));
        if self.work > MAX_WORK { return Err(inv(p, "the formula needs too much computation on this history (window lengths × bars); use shorter windows or fewer window functions")); }
        Ok(())
    }
    fn eval(&mut self, e: &E) -> Result<V, LabError> {
        Ok(match &e.ex {
            Ex::Num(v) => V::K(*v),
            Ex::Str(_) => return Err(inv(e.pos, "text is only allowed as a Param label, a Plot name or Plot's \"price\"")),
            Ex::Name(n) => self.name(n, e.pos)?,
            Ex::Neg(x) => map1(self.eval(x)?, |v| -v),
            Ex::Not(x) => map1(self.eval(x)?, |v| if v.is_nan() { NAN } else { truth(v == 0.) }),
            Ex::Bin(op, a, b) => { let a = self.eval(a)?; let b = self.eval(b)?; bin(op, a, b) }
            Ex::Index(x, n) => { let n = self.whole(n, 0, "x[n] looks n bars back; n")?; let x = self.eval(x)?; shift(x, n) }
            Ex::Call { name, args, out } => self.call(e.pos, name, args, out.as_ref())?,
        })
    }
    fn name(&self, n: &str, p: Pos) -> Result<V, LabError> {
        let lc = n.to_ascii_lowercase();
        if let Some((_, v)) = self.vars.iter().find(|(k, _)| *k == lc) { return Ok(v.clone()); }
        let b = self.bars;
        Ok(V::S(match lc.as_str() {
            "o" | "open" => b.open.clone(), "h" | "high" => b.high.clone(), "l" | "low" => b.low.clone(), "c" | "close" => b.close.clone(),
            "v" | "volume" => b.volume_or_late().ok_or_else(|| LabError::NeedsData(at(p, "volume: volume is missing on some admitted bars of this instrument")))?,
            "avg" | "hlc3" => b.hlc3(), "hl2" => b.hl2(), "ohlc4" => b.ohlc4(),
            "true" => return Ok(V::K(1.)), "false" => return Ok(V::K(0.)),
            _ if BUILTINS.iter().any(|f| f.0 == lc) || matches!(indicator(&lc), Ok(Some(_))) => return Err(inv(p, format!("{n} is a function; call it with ( )"))),
            _ => return Err(inv(p, format!("unknown name {n}"))),
        }))
    }
    fn call(&mut self, p: Pos, name: &str, args: &[Arg], out: Option<&(String, Pos)>) -> Result<V, LabError> {
        let lc = name.to_ascii_lowercase();
        match lc.as_str() {
            "plot" => return Err(inv(p, "Plot is a statement of its own: Plot(expression, \"Name\");")),
            "param" => {
                let Ex::Str(label) = &args[0].e.ex else { unreachable!("checked by parse") };
                return self.params.get(label.trim()).map(|v| V::K(*v)).ok_or_else(|| inv(p, format!("Param \"{label}\" has no value")));
            }
            _ => {}
        }
        let builtin = BUILTINS.iter().find(|b| b.0 == lc);
        if let Some(&(_, lo, hi)) = builtin {
            if args.iter().all(|a| a.name.is_none()) && (lo..=hi).contains(&args.len()) {
                if let Some((_, op)) = out { return Err(inv(*op, format!(".output applies to catalogue indicators, not {name}"))); }
                return self.builtin(p, &lc, args);
            }
        }
        match indicator(&lc).map_err(|e| inv(p, e))? {
            Some(def) => self.indicator(p, def, args, out),
            None => Err(inv(p, match builtin {
                Some(&(_, lo, hi)) if lo == hi => format!("{name} takes {lo} argument{}", if lo == 1 { "" } else { "s" }),
                Some(&(_, lo, hi)) => format!("{name} takes {lo} to {hi} arguments"),
                None => format!("unknown function {name}"),
            })),
        }
    }
    fn builtin(&mut self, p: Pos, f: &str, a: &[Arg]) -> Result<V, LabError> {
        let n = self.bars.len();
        Ok(match f {
            "ma" | "sma" | "ema" | "wma" | "wilders" | "rma" | "stdev" | "sum" | "hhv" | "highest" | "llv" | "lowest" | "roc" | "change" | "mom" | "linregslope" => {
                let len = self.whole(&a[1].e, 1, "the length")?;
                self.spend(p, len)?;
                let x = self.series(&a[0].e)?;
                V::S(match f {
                    "ma" | "sma" => ta::sma(&x, len), "ema" => ta::ema(&x, len), "wma" => ta::wma(&x, len), "wilders" | "rma" => ta::rma(&x, len),
                    "stdev" => ta::stdev(&x, len), "sum" => ta::sum(&x, len), "hhv" | "highest" => ta::highest(&x, len), "llv" | "lowest" => ta::lowest(&x, len),
                    "roc" => roc(&x, len), "linregslope" => slope(&x, len), _ => ta::change(&x, len),
                }.into_iter().map(fin).collect())
            }
            "ref" => {
                let k = self.konst(&a[1].e, "the Ref offset")?;
                if k > 0. { return Err(inv(a[1].e.pos, format!("Ref(x, {k}) would read {k} bars in the future; use Ref(x, -n) for n bars ago"))); }
                if k.fract() != 0. || -k > MAX_LOOKBACK as f64 { return Err(inv(a[1].e.pos, format!("the Ref offset must be a whole number from -{MAX_LOOKBACK} to 0, not {k}"))); }
                let x = self.eval(&a[0].e)?;
                shift(x, (-k) as usize)
            }
            "ama" => {
                let x = self.series(&a[0].e)?;
                let al = self.series(&a[1].e)?;
                let mut prev = NAN;
                V::S((0..n).map(|i| {
                    if x[i].is_nan() || !(0. ..=1.).contains(&al[i]) { return NAN; }
                    prev = if prev.is_nan() { x[i] } else { fin(al[i] * x[i] + (1. - al[i]) * prev) };
                    prev
                }).collect())
            }
            "abs" => map1(self.eval(&a[0].e)?, f64::abs),
            "sqrt" => map1(self.eval(&a[0].e)?, f64::sqrt),
            "log" => map1(self.eval(&a[0].e)?, f64::ln),
            "exp" => map1(self.eval(&a[0].e)?, f64::exp),
            "round" => map1(self.eval(&a[0].e)?, f64::round),
            "int" => map1(self.eval(&a[0].e)?, f64::trunc),
            "floor" => map1(self.eval(&a[0].e)?, f64::floor),
            "ceil" => map1(self.eval(&a[0].e)?, f64::ceil),
            "frac" => map1(self.eval(&a[0].e)?, f64::fract),
            "sign" => map1(self.eval(&a[0].e)?, |x| if x > 0. { 1. } else if x < 0. { -1. } else { x }),
            "cum" => {
                let x = self.series(&a[0].e)?;
                let mut sum: Option<f64> = None;
                V::S(x.iter().map(|&v| { if !v.is_nan() { sum = Some(sum.unwrap_or(0.) + v); } sum.map_or(NAN, fin) }).collect())
            }
            "highestsince" | "lowestsince" => {
                // as the rule builder's highest_since / lowest_since (series.rs Expr::Since)
                let (c, x, hi) = (self.series(&a[0].e)?, self.series(&a[1].e)?, f == "highestsince");
                let (mut started, mut unk, mut ext) = (false, false, NAN);
                V::S((0..n).map(|i| {
                    if on(c[i]) { started = true; unk = x[i].is_nan(); ext = x[i]; }
                    else if c[i].is_nan() || x[i].is_nan() { unk = true; }
                    else if started { ext = if hi { ext.max(x[i]) } else { ext.min(x[i]) }; }
                    if started && !unk { ext } else { NAN }
                }).collect())
            }
            "exrem" => {
                let (x, y) = (self.series(&a[0].e)?, self.series(&a[1].e)?);
                let mut active = false;
                V::S((0..n).map(|i| {
                    if !active && on(x[i]) { active = true; 1. } else { if active && on(y[i]) { active = false; } 0. }
                }).collect())
            }
            "flip" => {
                let (x, y) = (self.series(&a[0].e)?, self.series(&a[1].e)?);
                let mut state = false;
                V::S((0..n).map(|i| { if on(x[i]) { state = true; } if on(y[i]) { state = false; } truth(state) }).collect())
            }
            "max" | "min" => {
                let (x, y) = (self.eval(&a[0].e)?, self.eval(&a[1].e)?);
                let hi = f == "max";
                zip(x, y, move |x, y| if x.is_nan() || y.is_nan() { NAN } else if hi { x.max(y) } else { x.min(y) })
            }
            "iif" => {
                let (c, x, y) = (self.series(&a[0].e)?, self.series(&a[1].e)?, self.series(&a[2].e)?);
                V::S((0..n).map(|i| if c[i].is_nan() { NAN } else if c[i] != 0. { x[i] } else { y[i] }).collect())
            }
            "cross" => {
                let (x, y) = (self.series(&a[0].e)?, self.series(&a[1].e)?);
                V::S((0..n).map(|i| truth(i > 0 && x[i] > y[i] && x[i - 1] <= y[i - 1])).collect())
            }
            "barssince" => {
                let c = self.series(&a[0].e)?;
                let mut last = None;
                V::S((0..n).map(|i| { if on(c[i]) { last = Some(i); } last.map_or(NAN, |l| (i - l) as f64) }).collect())
            }
            "valuewhen" => {
                let nth = if a.len() == 3 { self.whole(&a[2].e, 1, "nth")? } else { 1 };
                if nth > MAX_NTH { return Err(inv(a[2].e.pos, format!("nth must be a whole number from 1 to {MAX_NTH}"))); }
                let (c, x) = (self.series(&a[0].e)?, self.series(&a[1].e)?);
                let mut hits = Vec::new();
                V::S((0..n).map(|i| { if on(c[i]) { hits.push(i); } if hits.len() >= nth { x[hits[hits.len() - nth]] } else { NAN } }).collect())
            }
            "nz" => {
                let x = self.eval(&a[0].e)?;
                let v = if a.len() == 2 { self.eval(&a[1].e)? } else { V::K(0.) };
                zip(x, v, |x, v| if x.is_nan() { v } else { x })
            }
            _ => unreachable!("every BUILTINS entry is handled"),
        })
    }
    fn indicator(&mut self, p: Pos, def: IndicatorDef, args: &[Arg], out: Option<&(String, Pos)>) -> Result<V, LabError> {
        if super::periods::USES_SESSION_DATES.contains(&def.id) { return Err(LabError::Unsupported(at(p, format!("{}: reads future session dates and is not available in formulas", def.name)))); }
        if def.needs.second_symbol { return Err(LabError::Unsupported(at(p, format!("{}: compares two instruments and is not available in formulas", def.name)))); }
        if def.needs.intraday || def.compute.is_none() {
            let m = at(p, format!("{}: {}", def.name, def.note));
            return Err(if def.needs.any_data() { LabError::NeedsData(m) } else { LabError::Unsupported(m) });
        }
        let mut given = Params::new();
        let (mut positional, mut named) = (0usize, false);
        for a in args {
            let key = match &a.name {
                Some(n) => {
                    named = true;
                    def.params.iter().find(|q| q.key.eq_ignore_ascii_case(n)).ok_or_else(|| inv(a.e.pos, format!("{} has no parameter {n}; parameters: {}", def.id, def.params.iter().map(|q| q.key).collect::<Vec<_>>().join(", "))))?.key
                }
                None if named => return Err(inv(a.e.pos, "positional arguments come before named ones")),
                None => {
                    positional += 1;
                    def.params.get(positional - 1).ok_or_else(|| inv(a.e.pos, format!("{} takes at most {} argument{}", def.id, def.params.len(), if def.params.len() == 1 { "" } else { "s" })))?.key
                }
            };
            let v = self.konst(&a.e, &format!("{}'s {key}", def.id))?;
            if given.insert(key.to_string(), v).is_some() { return Err(inv(a.e.pos, format!("{} is given twice", key))); }
        }
        let params = catalogue::resolve_params(&def, &given).map_err(|e| inv(p, e))?;
        let o = match out {
            None => 0,
            Some((o, op)) => def.outputs.iter().position(|x| x.eq_ignore_ascii_case(o)).ok_or_else(|| inv(*op, format!("{} has outputs {}", def.id, def.outputs.join(", "))))?,
        };
        let key = format!("{}|{:?}", def.id, params);
        if let Some((_, outs)) = self.calls.iter().find(|(k, _)| *k == key) { return Ok(V::S(outs[o].clone())); }
        if self.calls.len() == MAX_CALLS { return Err(inv(p, format!("more than {MAX_CALLS} different indicator calls; the limit is {MAX_CALLS}"))); }
        let outs = if self.check { vec![Vec::new(); def.outputs.len()] } else {
            catalogue::run(&def, self.bars, &params).map_err(|e| located(p, super::series::catalogue_error(&def, e)))?
        };
        let v = outs[o].clone();
        self.calls.push((key, outs));
        Ok(V::S(v))
    }
}

/// A formula operand carried by a spec (the text travels with the spec, so a saved strategy keeps its formula).
#[derive(Debug)]
pub struct Formula { pub name: String, pub text: String, pub prog: Program }
impl PartialEq for Formula { fn eq(&self, o: &Self) -> bool { self.name == o.name && self.text == o.text } }
pub fn name_of(name: Option<&str>) -> Result<String, String> {
    let n = name.unwrap_or(DEFAULT_NAME).trim();
    if n.is_empty() || n.chars().count() > MAX_NAME || n.chars().any(char::is_control) { return Err(format!("name must be 1 to {MAX_NAME} characters")); }
    Ok(n.into())
}
/// Parses a formula operand: (formula, resolved params, plot index). Errors carry line/col where they come from the text.
pub fn operand(name: Option<&str>, text: &str, given: &Params, out: Option<&str>) -> Result<(Formula, Params, usize), String> {
    let name = name_of(name)?;
    let prog = parse(text)?;
    let params = resolve(&prog, given)?;
    check(&prog, &params)?;
    let out = match out {
        None => 0,
        Some(o) => prog.plots.iter().position(|q| q.eq_ignore_ascii_case(o)).ok_or_else(|| format!("out: {name} has plots {}", prog.plots.join(", ")))?,
    };
    Ok((Formula { name, text: text.into(), prog }, params, out))
}
/// `My Momentum(Fast=9).Osc`: params at their default are omitted (like LATE_PARAMS), the plot is always named.
pub fn label(f: &Formula, params: &Params, out: usize) -> String {
    let shown: Vec<String> = f.prog.params.iter().filter_map(|p| params.get(&p.key).filter(|v| **v != p.default).map(|v| format!("{}={v}", p.key))).collect();
    let head = if shown.is_empty() { f.name.clone() } else { format!("{}({})", f.name, shown.join(",")) };
    format!("{head}.{}", f.prog.plots.get(out).map_or("?", String::as_str))
}
fn num(v: f64) -> String { if v.is_finite() { v.to_string() } else { "null".into() } }
/// `"params":[{key,default,min,max,step}],"plots":[…],"placement":"pane"|"price"` (no braces).
pub fn info_json(prog: &Program) -> String {
    let params: Vec<String> = prog.params.iter().map(|p| format!("{{\"key\":\"{}\",\"default\":{},\"min\":{},\"max\":{},\"step\":{}}}", esc(&p.key), num(p.default), num(p.min), num(p.max), num(p.step))).collect();
    let plots: Vec<String> = prog.plots.iter().map(|p| format!("\"{}\"", esc(p))).collect();
    format!("\"params\":[{}],\"plots\":[{}],\"placement\":\"{}\"", params.join(","), plots.join(","), if prog.price { "price" } else { "pane" })
}
pub fn functions_json() -> String {
    let items: Vec<String> = FUNCTIONS.iter().map(|(n, a, d)| format!("{{\"name\":\"{}\",\"args\":\"{}\",\"desc\":\"{}\"}}", esc(n), esc(a), esc(d))).collect();
    format!("[{}]", items.join(","))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lab::testdata::{assert_series, close_eq, closes, go, walk};
    const N: f64 = f64::NAN;

    fn one(text: &str, b: &Bars) -> Vec<f64> { let p = parse(text).unwrap_or_else(|e| panic!("{text}: {e}")); eval(&p, b, &resolve(&p, &Params::new()).unwrap()).unwrap().remove(0) }
    fn plot(expr: &str, b: &Bars) -> Vec<f64> { one(&format!("Plot({expr}, \"X\")"), b) }
    fn err(text: &str) -> String { parse(text).unwrap_err() }
    /// Exactly equal, NaN included.
    fn same(a: &[f64], b: &[f64]) { assert_eq!(a.len(), b.len()); for (i, (x, y)) in a.iter().zip(b).enumerate() { assert!(x.to_bits() == y.to_bits() || (x.is_nan() && y.is_nan()), "bar {i}: {x} != {y}"); } }
    fn run_err(text: &str, b: &Bars) -> LabError { let p = parse(text).unwrap(); eval(&p, b, &resolve(&p, &Params::new()).unwrap()).unwrap_err() }

    #[test] fn arithmetic_precedence_and_operators() {
        let b = closes(&[2., 4.]);
        assert_series(&plot("1 + 2 * 3 - 4 / 2", &b), &[5., 5.]);
        assert_series(&plot("-2 ^ 2", &b), &[-4., -4.]);
        assert_series(&plot("2 ^ 3 ^ 2", &b), &[512., 512.]);
        assert_series(&plot("(C + 1) * 2", &b), &[6., 10.]);
        assert_series(&plot("C / (C - 2)", &b), &[N, 2.]); // division by zero is NaN
        assert_series(&plot("C > 3", &b), &[0., 1.]);
        assert_series(&plot("C < 3", &b), &[1., 0.]);
        assert_series(&plot("C >= 4", &b), &[0., 1.]);
        assert_series(&plot("C <= 2", &b), &[1., 0.]);
        assert_series(&plot("C == 4", &b), &[0., 1.]);
        assert_series(&plot("C != 4", &b), &[1., 0.]);
        assert_series(&plot("C > 3 AND C < 5", &b), &[0., 1.]);
        assert_series(&plot("C > 3 && true", &b), &[0., 1.]);
        assert_series(&plot("C > 3 OR false", &b), &[0., 1.]);
        assert_series(&plot("C < 3 || C > 3", &b), &[1., 1.]);
        assert_series(&plot("NOT C > 3", &b), &[1., 0.]);
        assert_series(&plot("!(C > 3)", &b), &[1., 0.]);
        assert_series(&plot("not c > 3 and C == 2", &b), &[1., 0.]);
    }
    #[test] fn nan_propagates_and_comparisons_with_nan_are_false() {
        let b = closes(&[1., 2., 3.]);
        assert_series(&plot("C[1] + 1", &b), &[N, 2., 3.]);
        assert_series(&plot("C[1] > 0", &b), &[0., 1., 1.]);
        assert_series(&plot("C[1] != 5", &b), &[0., 1., 1.]);
        assert_series(&plot("C[1] == C[1]", &b), &[0., 1., 1.]);
        assert_series(&plot("NOT C[1]", &b), &[N, 0., 0.]);
        assert_series(&plot("C[1] AND 1", &b), &[N, 1., 1.]);
        assert_series(&plot("Sqrt(C - 2)", &b), &[N, 0., 1.]);
        assert_series(&plot("Log(C - 1)", &b), &[N, 0., 2f64.ln()]);
    }
    #[test] fn price_arrays_and_names_are_case_insensitive() {
        let b = crate::lab::testdata::d1();
        for (n, want) in [("O", &b.open), ("open", &b.open), ("H", &b.high), ("High", &b.high), ("l", &b.low), ("LOW", &b.low), ("c", &b.close), ("Close", &b.close)] {
            assert_series(&plot(n, &b), want);
        }
        assert_series(&plot("V", &b), b.volume.as_ref().unwrap());
        assert_series(&plot("Volume", &b), b.volume.as_ref().unwrap());
        assert_series(&plot("Avg", &b), &b.hlc3());
        assert_series(&plot("hlc3", &b), &b.hlc3());
        assert_series(&plot("hl2", &b), &b.hl2());
        assert_series(&plot("ohlc4", &b), &b.ohlc4());
        assert_series(&one("Fast = 3\nx = c * fast\nPLOT(X, \"A\")", &b), &b.close.iter().map(|v| v * 3.).collect::<Vec<_>>());
        let e = run_err("Plot(V, \"A\")", &closes(&[1.]));
        assert_eq!((e.status(), e.message()), ("needs_data", "line 1, col 6: volume: volume is missing on some admitted bars of this instrument"));
    }
    #[test] fn window_functions_match_the_ta_primitives() {
        let b = walk(120, 3, true);
        let c = &b.close;
        for (f, want) in [("MA", ta::sma(c, 7)), ("SMA", ta::sma(c, 7)), ("EMA", ta::ema(c, 7)), ("WMA", ta::wma(c, 7)), ("Wilders", ta::rma(c, 7)),
            ("RMA", ta::rma(c, 7)), ("StDev", ta::stdev(c, 7)), ("Sum", ta::sum(c, 7)), ("HHV", ta::highest(c, 7)), ("Highest", ta::highest(c, 7)),
            ("LLV", ta::lowest(c, 7)), ("Lowest", ta::lowest(c, 7)), ("Change", ta::change(c, 7)), ("Mom", ta::change(c, 7))] {
            assert_series(&plot(&format!("{f}(C, 7)"), &b), &want);
        }
        assert_series(&plot("ROC(C, 2)", &closes(&[100., 50., 110., 0., 5.])), &[N, N, 10., -100., 100. * (5. - 110.) / 110.]);
        assert_series(&plot("LinRegSlope(C, 3)", &closes(&[1., 3., 5., 4.])), &[N, N, 2., 0.5]);
        assert_series(&plot("MA(C, 2)", &closes(&[1., 3., 5.])), &[N, 2., 4.]);
        assert_series(&plot("MA(5, 2)", &closes(&[1., 3., 5.])), &[N, 5., 5.]); // constants broadcast
    }
    #[test] fn roc_divides_by_the_old_value() {
        assert_series(&plot("ROC(C, 1)", &closes(&[100., 110., 0., 5.])), &[N, 10., -100., N]);
    }
    #[test] fn elementwise_and_state_functions() {
        let b = closes(&[1., 3., 2., 5., 4.]);
        assert_series(&plot("Abs(C - 3)", &b), &[2., 0., 1., 2., 1.]);
        assert_series(&plot("Sqrt(C - 1)", &b), &[0., 2f64.sqrt(), 1., 2., 3f64.sqrt()]);
        assert_series(&plot("Exp(0 * C)", &b), &[1.; 5]);
        assert_series(&plot("Log(Exp(C))", &b), &b.close);
        assert_series(&plot("Max(C, 3)", &b), &[3., 3., 3., 5., 4.]);
        assert_series(&plot("Min(C, C[1])", &b), &[N, 1., 2., 2., 4.]);
        assert_series(&plot("IIf(C > 2, C, -C)", &b), &[-1., 3., -2., 5., 4.]);
        assert_series(&plot("IIf(C[1] > 2, 1, 0)", &b), &[0., 0., 1., 0., 1.]); // a comparison with NaN is false
        assert_series(&plot("IIf(C[1], 1, 0)", &b), &[N, 1., 1., 1., 1.]);
        assert_series(&plot("Cross(C, 2.5)", &b), &[0., 1., 0., 1., 0.]);
        assert_series(&plot("Cross(2.5, C)", &b), &[0., 0., 1., 0., 0.]);
        assert_series(&plot("BarsSince(C > 4)", &b), &[N, N, N, 0., 1.]);
        assert_series(&plot("ValueWhen(C > 2, C)", &b), &[N, 3., 3., 5., 4.]);
        assert_series(&plot("ValueWhen(C > 2, C, 2)", &b), &[N, N, N, 3., 5.]);
        assert_series(&plot("Nz(C[1])", &b), &[0., 1., 3., 2., 5.]);
        assert_series(&plot("Nz(C[2], -1)", &b), &[-1., -1., 1., 3., 2.]);
        assert_series(&plot("Ref(C, -1)", &b), &[N, 1., 3., 2., 5.]);
        assert_series(&plot("Ref(C, 0)", &b), &b.close);
        assert_series(&plot("C[2]", &b), &[N, N, 1., 3., 2.]);
        assert_series(&plot("AMA(C, 0.5)", &b), &[1., 2., 2., 3.5, 3.75]);
        assert_series(&plot("AMA(C, IIf(C > 2, 1, 0))", &b), &[1., 3., 3., 5., 4.]);
        assert_series(&plot("AMA(C, 2)", &b), &[N; 5]);
    }
    #[test] fn ema_and_catalogue_calls_equal_the_catalogue() {
        let b = walk(300, 9, true);
        same(&plot("EMA(C, 9)", &b), &go("ema", &b, &[("length", 9.)])[0]);
        same(&plot("ema(9)", &b), &go("ema", &b, &[("length", 9.)])[0]);
        same(&plot("RSI(14)", &b), &go("rsi", &b, &[("length", 14.)])[0]);
        same(&plot("rsi(length=14)", &b), &go("rsi", &b, &[("length", 14.)])[0]);
        same(&plot("rsi()", &b), &go("rsi", &b, &[])[0]);
        same(&plot("macd(8, signal=5).signal", &b), &go("macd", &b, &[("fast", 8.), ("signal", 5.)])[1]);
        same(&plot("Supertrend(10, 3).direction", &b), &go("supertrend", &b, &[("atr_length", 10.), ("factor", 3.)])[1]);
        same(&one("Len = Param(\"Len\", 9, 2, 50, 1)\nPlot(EMA(C, Len), \"E\")", &b), &go("ema", &b, &[("length", 9.)])[0]);
        same(&one("Fast = Param(\"Fast\",12,2,50,1); Plot(EMA(C, Fast), \"x\");", &b), &go("ema", &b, &[("length", 12.)])[0]);
        same(&one("Plot(rsi(Param(\"N\", 14, 2, 50, 1)), \"R\")", &b), &go("rsi", &b, &[("length", 14.)])[0]);
    }
    #[test] fn indicator_call_errors_name_the_problem() {
        let b = walk(50, 1, false);
        assert!(err("Plot(rsi(foo=3), \"R\")").contains("rsi has no parameter foo"));
        assert!(err("Plot(rsi(1, 0, 3), \"R\")").contains("rsi takes at most 2 arguments"));
        assert!(err("Plot(rsi(length=3, 2), \"R\")").contains("positional arguments come before named ones"));
        assert!(err("Plot(rsi(14, length=3), \"R\")").contains("length is given twice"));
        assert!(err("Plot(rsi(0), \"R\")").contains("length must be between"));
        assert!(err("Plot(rsi(C), \"R\")").contains("must be a number or a Param, not a series"));
        assert!(err("Plot(macd().nope, \"R\")").contains("macd has outputs macd, signal, hist"));
        assert!(err("Plot(EMA(C, 9).x, \"R\")").contains(".output applies to catalogue indicators"));
        assert!(err("Plot(vwap(), \"R\")").starts_with("line 1, col 6: "));
        assert!(err("Plot(sessions_left(), \"R\")").contains("future session dates"));
        assert!(err("Plot(correlation(), \"R\")").contains("two instruments"));
        let e = run_err("Plot(obv(), \"R\")", &b);
        assert_eq!(e.status(), "needs_data");
        assert!(e.message().starts_with("line 1, col 6: ") && e.message().contains("needs volume"), "{}", e.message());
        let many: String = (1..=21).map(|k| format!("x{k} = ema({k})\n")).collect::<String>() + "Plot(x1, \"A\")";
        assert!(err(&many).contains("more than 20 different indicator calls"));
        let same: String = (1..=30).map(|k| format!("x{k} = ema(9)\n")).collect::<String>() + "Plot(x1, \"A\")";
        assert!(parse(&same).is_ok(), "repeated identical calls are computed once");
    }
    #[test] fn look_ahead_is_refused() {
        assert_eq!(err("Plot(Ref(C, 1), \"X\")"), "line 1, col 13: Ref(x, 1) would read 1 bars in the future; use Ref(x, -n) for n bars ago");
        assert!(err("Plot(C[-1], \"X\")").starts_with("line 1, col 8: x[n] looks n bars back; n must be a whole number from 0 to 2000"));
        assert!(err("k = Param(\"K\", 1, 1, 5, 1)\nPlot(Ref(C, k), \"X\")").contains("future"));
        assert!(err("Plot(C[2001], \"X\")").contains("from 0 to 2000"));
        assert!(err("Plot(C[1.5], \"X\")").contains("whole number"));
        assert!(err("Plot(C[C], \"X\")").contains("not a series"));
    }
    #[test] fn every_function_is_causal() {
        let full = walk(260, 5, true);
        let text = "f = Param(\"F\", 5, 1, 50, 1)\na = MA(C, f) + SMA(C, 3) + EMA(C, f) + WMA(C, 4) + Wilders(C, 6) + RMA(C, 3)\n\
            b = StDev(C, 5) + Sum(C, 3) + HHV(H, 7) + Highest(H, 2) + LLV(L, 7) + Lowest(L, 3) + ROC(C, 4) + Change(C, 2) + Mom(C, 1)\n\
            d = LinRegSlope(C, 6) + AMA(C, 0.3) + Abs(C - O) + Sqrt(V) + Log(C) + Exp(C / 1000) + Max(O, C) + Min(O, C) + IIf(C > O, 1, 2)\n\
            e = Cross(C, MA(C, 5)) + BarsSince(C > O) + ValueWhen(Cross(C, O), C, 2) + Nz(Ref(C, -3)) + C[2] + rsi(5) + Supertrend(10, 3).direction\n\
            Plot(a, \"A\"); Plot(b, \"B\"); Plot(d, \"D\"); Plot(e, \"E\")";
        let p = parse(text).unwrap();
        let params = resolve(&p, &Params::new()).unwrap();
        let want = eval(&p, &full, &params).unwrap();
        for cut in (20..260).step_by(9) {
            let got = eval(&p, &crate::lab::testdata::cut(&full, cut), &params).unwrap();
            for (k, s) in got.iter().enumerate() { for i in 0..cut { assert!(close_eq(s[i], want[k][i]), "plot {k} bar {i} changes when bars after {cut} exist"); } }
        }
        assert!(want.iter().all(|s| s.len() == 260 && s[100..].iter().all(|v| v.is_finite())));
    }
    #[test] fn warm_up_is_nan() {
        let s = plot("EMA(C, 10)", &walk(30, 2, false));
        assert!(s[..9].iter().all(|v| v.is_nan()) && s[9..].iter().all(|v| v.is_finite()));
    }
    #[test] fn params_are_declared_overridden_and_bounded() {
        let p = parse("Fast = Param(\"Fast\", 9, 2, 50, 1)\nk = Param(\"Mult\", 1.5, 0.5, 3, 0.25)\nPlot(EMA(C, Fast) * k, \"Osc\")\nPlot(C, \"Signal\", \"price\")").unwrap();
        assert_eq!(p.params, vec![ParamDef { key: "Fast".into(), default: 9., min: 2., max: 50., step: 1. }, ParamDef { key: "Mult".into(), default: 1.5, min: 0.5, max: 3., step: 0.25 }]);
        assert_eq!((p.plots.clone(), p.price), (vec!["Osc".to_string(), "Signal".to_string()], true));
        assert_eq!(info_json(&p), r#""params":[{"key":"Fast","default":9,"min":2,"max":50,"step":1},{"key":"Mult","default":1.5,"min":0.5,"max":3,"step":0.25}],"plots":["Osc","Signal"],"placement":"price""#);
        let given = |kv: &[(&str, f64)]| -> Params { kv.iter().map(|(k, v)| (k.to_string(), *v)).collect() };
        assert_eq!(resolve(&p, &given(&[("Fast", 20.)])).unwrap(), given(&[("Fast", 20.), ("Mult", 1.5)]));
        assert_eq!(resolve(&p, &given(&[("Fast", 1.)])).unwrap_err(), "Fast must be between 2 and 50");
        assert_eq!(resolve(&p, &given(&[("Fast", 9.5)])).unwrap_err(), "Fast must be a whole number");
        assert!(resolve(&p, &given(&[("Mult", 1.3)])).is_ok(), "a fractional step accepts any value in range");
        assert_eq!(resolve(&p, &given(&[("Slow", 3.)])).unwrap_err(), "unknown parameter 'Slow'; parameters: Fast, Mult");
        let b = walk(80, 4, false);
        let over = eval(&p, &b, &resolve(&p, &given(&[("Fast", 20.)])).unwrap()).unwrap();
        same(&over[0], &ta::ema(&b.close, 20).iter().map(|v| fin(v * 1.5)).collect::<Vec<_>>());
        // a Param whose range reaches an invalid use is caught when the override is checked
        let q = parse("n = Param(\"N\", 1, -3, 3, 1)\nPlot(MA(C, n), \"X\")").unwrap();
        assert!(check(&q, &resolve(&q, &given(&[("N", 0.)])).unwrap()).unwrap_err().contains("the length must be a whole number from 1"));
        for (bad, why) in [("Param(\"A\", 5, 6, 9, 1)", "min <= default <= max"), ("Param(\"A\", 5, 1, 9, 0)", "step > 0"), ("Param(\"A\", 5.5, 1, 9, 1)", "whole numbers"),
            ("Param(A, 5, 1, 9, 1)", "plain numbers"), ("Param(\"A\", C, 1, 9, 1)", "plain numbers"), ("Param(\"A\", 5, 1, 9)", "plain numbers"), ("Param(\"\", 5, 1, 9, 1)", "label"),
            ("Param(\"A.b\", 5, 1, 9, 1)", "label"), ("Param(\"A\", 5, 1, 1e7, 1)", "±1000000"), ("Param(\"tf\", 5, 1, 9, 1)", "Param name \"tf\" is reserved"), ("Param(\" Ticker \", 5, 1, 9, 1)", "reserved"),
            ("Param(\"a[0]\", 5, 1, 9, 1)", "label"), ("Param(\"   \", 5, 1, 9, 1)", "label")] {
            let e = err(&format!("x = {bad}\nPlot(C * x, \"X\")"));
            assert!(e.starts_with("line 1, col ") && e.contains(why), "{bad}: {e}");
        }
        assert!(err("a = Param(\"A\", 5, 1, 9, 1)\nb = Param(\"a\", 5, 1, 9, 1)\nPlot(a + b, \"X\")").contains("declared twice"));
        let seventeen: String = (0..17).map(|k| format!("p{k} = Param(\"P{k}\", 1, 1, 9, 1)\n")).collect::<String>() + "Plot(C, \"X\")";
        assert!(err(&seventeen).contains("more than 16 Params"));
        assert_eq!(parse("Plot(-Param(\"Neg\", -2, -5, 0, 1) * C, \"X\")").unwrap().params[0].default, -2.);
    }
    #[test] fn limits_are_enforced() {
        assert!(err(&format!("Plot(C, \"X\") // {}", "x".repeat(4000))).contains("the limit is 4000"));
        let deep = format!("Plot({}C{}, \"X\")", "(".repeat(70), ")".repeat(70));
        assert!(err(&deep).contains("nest deeper than 64"));
        let big = format!("Plot({}, \"X\")", vec!["C"; 160].join(" + "));
        assert!(err(&big).contains("more than 300 expression parts"));
        let nine: String = (0..9).map(|k| format!("Plot(C, \"P{k}\")\n")).collect();
        assert!(err(&nine).contains("more than 8 Plots"));
        assert!(err("x = C").contains("at least one Plot"));
        assert!(err("").contains("at least one Plot"));
        assert!(err("Plot(C, \"A\")\nPlot(O, \"a\")").contains("used twice"));
        let heavy: String = (0..40).map(|k| format!("x{k} = MA(C, {})\n", 1900 + k)).collect::<String>() + "Plot(x0, \"A\")";
        let e = run_err(&heavy, &walk(5000, 1, false));
        assert!(e.message().contains("too much computation"), "{}", e.message());
    }
    #[test] fn errors_carry_line_and_column() {
        assert_eq!(err("x = C\n  y = FOO(C)\nPlot(y, \"A\")"), "line 2, col 7: unknown function FOO");
        assert_eq!(err("Plot(Q, \"A\")"), "line 1, col 6: unknown name Q");
        assert_eq!(err("Plot(EMA, \"A\")"), "line 1, col 6: EMA is a function; call it with ( )");
        assert_eq!(err("Plot(EMA(C), \"A\")"), "line 1, col 10: ema's length must be a number or a Param, not a series");
        assert_eq!(err("Plot(ema(9, 1, 2), \"A\")"), "line 1, col 13: ema takes at most 1 argument");
        assert!(err("Plot(IIf(C, 1), \"A\")").contains("IIf takes 3 arguments"));
        assert!(err("Plot(ValueWhen(C), \"A\")").contains("ValueWhen takes 2 to 3 arguments"));
        assert_eq!(err("x = C\nx = O\nPlot(x, \"A\")"), "line 2, col 1: x is already defined; each name is assigned once");
        assert_eq!(err("x = x + 1\nPlot(x, \"A\")"), "line 1, col 5: unknown name x");
        assert_eq!(err("C = 1\nPlot(C, \"A\")"), "line 1, col 1: C is a built-in name; choose another");
        assert_eq!(err("x = C = 1\nPlot(x, \"A\")"), "line 1, col 7: use == to compare (= names a value at the start of a statement)");
        assert_eq!(err("Plot(1 < C < 3, \"A\")"), "line 1, col 12: comparisons do not chain; join them with AND");
        assert_eq!(err("Plot((C + 1, \"A\")"), "line 1, col 12: expected ')' but found ','");
        assert_eq!(err("Plot(C, \"A\") Plot(O, \"B\")"), "line 1, col 14: expected ';' or a new line but found Plot");
        assert_eq!(err("Plot(C, \"A)"), "line 1, col 9: text is missing its closing \"");
        assert_eq!(err("Plot(C # 2, \"A\")"), "line 1, col 8: unexpected character '#'");
        assert_eq!(err("C + 1\nPlot(C, \"A\")"), "line 1, col 1: a statement is `name = expression` or `Plot(expression, \"Name\")`");
        assert_eq!(err("x = Plot(C, \"A\")\nPlot(C, \"B\")"), "line 1, col 5: Plot is a statement of its own: Plot(expression, \"Name\");");
        assert_eq!(err("Plot(C, \"A\", \"pane\")"), "line 1, col 14: the third Plot argument may only be \"price\"");
        assert_eq!(err("Plot(C, \"A\"); Plot(\"B\", \"B\")"), "line 1, col 20: text is only allowed as a Param label, a Plot name or Plot's \"price\"");
        assert_eq!(err("Plot(C.x, \"A\")"), "line 1, col 7: .output follows an indicator call, e.g. Supertrend(10, 3).direction");
        assert_eq!(err("Plot(AND, \"A\")"), "line 1, col 6: AND needs a value on its left");
        assert_eq!(err("Plot(C +, \"A\")"), "line 1, col 9: unexpected ','");
        assert_eq!(err("Plot(C +"), "line 1, col 9: unexpected the end of the formula");
    }
    #[test] fn statements_split_on_semicolons_newlines_and_comments_but_not_inside_brackets() {
        let b = closes(&[1., 2., 3.]);
        assert_series(&one("// header\nx = C * 2; y = (x +\n1) // trailing\nPlot(MA(y,\n 2), \"A\");;\n\n", &b), &[N, 4., 6.]);
        assert_series(&one("Plot(C[1] + 1e1, \"A\")", &b), &[N, 11., 12.]);
        assert_eq!(err("x = C +\n1\nPlot(x, \"A\")"), "line 1, col 8: unexpected the end of the statement");
    }
    #[test] fn aliases_resolve_only_when_unambiguous() {
        let b = walk(120, 2, true);
        assert!(matches!(indicator("ema"), Ok(Some(d)) if d.id == "ema"));
        assert!(indicator("nonexistent_xyz").unwrap().is_none());
        for (alias, ids) in aliases() {
            if ids.len() == 1 && catalogue::get(alias).is_none() { assert_eq!(indicator(alias).unwrap().unwrap().id, ids[0]); }
            if ids.len() > 1 && catalogue::get(alias).is_none() { assert!(indicator(alias).err().unwrap().contains("could mean")); }
        }
        same(&plot("RSIa(14)", &b), &go("rsi", &b, &[("length", 14.)])[0]);
    }
    #[test] fn functions_json_lists_every_builtin() {
        let j = crate::lab::json::parse(&functions_json()).unwrap();
        let crate::lab::json::Json::Arr(items) = j else { panic!() };
        assert_eq!(items.len(), FUNCTIONS.len());
        for (name, ..) in BUILTINS.iter().filter(|b| !["sma", "rma", "highest", "lowest", "mom"].contains(&b.0)) {
            assert!(FUNCTIONS.iter().any(|f| f.0.eq_ignore_ascii_case(name)), "{name} missing from FUNCTIONS");
        }
    }
    #[test] fn rounding_cum_since_exrem_and_flip() {
        let b = closes(&[1.4, -2.5, 3., 2., 5., 4.]);
        assert_series(&plot("Round(C)", &b), &[1., -3., 3., 2., 5., 4.]);
        assert_series(&plot("Int(C)", &b), &[1., -2., 3., 2., 5., 4.]);
        assert_series(&plot("Floor(C)", &b), &[1., -3., 3., 2., 5., 4.]);
        assert_series(&plot("Ceil(C)", &b), &[2., -2., 3., 2., 5., 4.]);
        assert_series(&plot("Frac(C)", &b), &[0.4, -0.5, 0., 0., 0., 0.]);
        assert_series(&plot("Sign(C - 3)", &b), &[-1., -1., 0., -1., 1., 1.]);
        assert_series(&plot("Cum(C)", &b), &[1.4, -1.1, 1.9, 3.9, 8.9, 12.9]);
        // a missing value adds nothing; NaN until the first value
        assert_series(&plot("Cum(Ref(C, -1))", &b), &[N, 1.4, -1.1, 1.9, 3.9, 8.9]);
        // from the latest up-close (inclusive): bars 2 (3) and 4 (5); unknown before
        assert_series(&plot("HighestSince(C > Ref(C, -1), C)", &b), &[N, N, 3., 3., 5., 5.]);
        assert_series(&plot("LowestSince(C > Ref(C, -1), C)", &b), &[N, N, 3., 2., 5., 4.]);
        // the same as the rule builder's highest_since on the same data
        assert_series(&plot("ExRem(C > 0, C > 4)", &b), &[1., 0., 0., 0., 0., 1.]);
        // same bar: a starts a signal (bar 5), b only ends one on a later bar (bar 2 ends bar 0's)
        assert_series(&plot("ExRem(C > 0, C > 2)", &b), &[1., 0., 0., 1., 0., 1.]);
        assert_series(&plot("Flip(C > 4, C < 2.5)", &b), &[0., 0., 0., 0., 1., 1.]);
        // same bar: b wins
        assert_series(&plot("Flip(C > 0, C < 2.5)", &b), &[0., 0., 1., 0., 1., 1.]);
    }
    #[test] fn operand_labels_and_out() {
        let text = "Fast = Param(\"Fast\", 9, 2, 50, 1)\nSlow = Param(\"Slow\", 21, 5, 100, 1)\nPlot(EMA(C, Fast) - EMA(C, Slow), \"Osc\")\nPlot(C, \"Signal\")";
        let given: Params = [("Fast".to_string(), 12.)].into();
        let (f, p, out) = operand(Some(" My Momentum "), text, &given, Some("signal")).unwrap();
        assert_eq!((f.name.as_str(), out, label(&f, &p, 0), label(&f, &p, 1)), ("My Momentum", 1, "My Momentum(Fast=12).Osc".into(), "My Momentum(Fast=12).Signal".into()));
        let (f, p, _) = operand(None, text, &Params::new(), None).unwrap();
        assert_eq!(label(&f, &p, 0), "Custom.Osc");
        assert!(operand(None, text, &Params::new(), Some("Nope")).unwrap_err().contains("has plots Osc, Signal"));
        assert!(operand(Some(""), text, &Params::new(), None).unwrap_err().contains("name must be"));
        assert!(operand(Some(&"x".repeat(61)), text, &Params::new(), None).is_err());
    }
}
