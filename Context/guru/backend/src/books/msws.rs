//! Gregory L. Morris, "Indicators & Trading Systems" GM1-GM9 MetaStock series, as anthologised in
//! "Wallstreet Trade Systems & Indicators for MetaStock" (research drive MetaStock/@Systems & Indicators/
//! Wallstreet Trade Syst. & Indicators for MS/: DBZ_Wallstreet TradSys Meta.doc = the systems, 46 pp. when
//! printed; DBZ_Wallstreet formulas.doc = the Fml() custom indicators, 57 pp.). The candle section names
//! Morris ("CandlePower") as the developer of the series. Book id `msws`.
//! Every system is stored as the printed MetaStock formula text and run by the formula engine below; a
//! system is Live only when a Close formula is printed and every function it calls is defined.
//! Page refs = page of the .doc printed to PDF (LibreOffice). Transcription and checks: library/msws/.
// ===================================================================== MetaStock formula engine
// The systems in these anthologies are MetaStock (and Window on WallStreet) formula text. Rather than
// re-typing ~150 systems by hand, the printed formula text itself is stored and evaluated by this small
// interpreter, so the rules in the catalogue and the rules that run are the same characters.
// Semantics (sources in library/msws/README.md):
//  * precedence: unary minus, * /, + -, comparisons (left to right), AND, OR (MetaStock Formula Primer);
//    TRUE = 1, FALSE = 0, any non-zero value is TRUE in AND / OR / If.
//  * a bar is undefined (NaN) until every input of the expression is defined; undefined is never TRUE.
//    Division by zero gives an undefined bar (MetaStock reports an error; it prints no value rule).
//  * Mov(x,n,E): seeded with the first defined value and smoothed from the next bar, shown from bar n
//    (MetaStock forum, "EMA 9 periods - initial value"). Wilder averages (RSI, ADX, ATR) are seeded with
//    the simple mean of the first n values. MACD() = EMA(alpha 0.15) - EMA(alpha 0.075) of the close,
//    seeded on the first close (R. Larsen, MetaStock list 2003). Sin() takes degrees, Log() is natural.
//  * Cum and recursive averages skip undefined input bars (the bar is undefined, the state is kept).
use crate::connors::{Ctx, Leg, Trade};
use std::collections::HashMap;
use std::rc::Rc;

const NAN: f64 = f64::NAN;
type V = Rc<Vec<f64>>;

#[derive(Clone, Debug, PartialEq)]
pub enum E { N(f64), Id(String), S(String), Sym(String), Call(String, Vec<E>), B(u8, Box<E>, Box<E>), Neg(Box<E>) }
// B ops: 0 OR, 1 AND, 2 <, 3 >, 4 <=, 5 >=, 6 =, 7 <>, 8 +, 9 -, 10 *, 11 /

#[derive(Clone, Debug, PartialEq)]
enum T { Num(f64), Id(String), Str(String), Sym(String), Op(&'static str), LP, RP, Comma, Semi, Assign }

fn lex(src: &str) -> Result<Vec<T>, String> {
    let s: Vec<char> = src.replace(['\u{2019}', '\u{2018}'], "'").replace(['\u{201c}', '\u{201d}'], "\"").chars().collect();
    let mut out = vec![];
    let mut i = 0;
    while i < s.len() {
        let c = s[i];
        if c.is_whitespace() { i += 1; continue; }
        if c == '{' { while i < s.len() && s[i] != '}' { i += 1; } i += 1; continue; }
        if c == '"' {
            let j = i + 1; let mut k = j;
            while k < s.len() && s[k] != '"' { k += 1; }
            out.push(T::Str(s[j..k.min(s.len())].iter().collect::<String>().trim().to_string()));
            i = k + 1; continue;
        }
        if c.is_ascii_digit() || (c == '.' && i + 1 < s.len() && s[i + 1].is_ascii_digit()) {
            let j = i;
            while i < s.len() && (s[i].is_ascii_digit() || s[i] == '.') { i += 1; }
            let t: String = s[j..i].iter().collect();
            out.push(T::Num(t.parse().map_err(|_| format!("bad number {t}"))?)); continue;
        }
        if c.is_ascii_alphabetic() || c == '_' {
            let j = i;
            while i < s.len() && (s[i].is_ascii_alphanumeric() || s[i] == '_') { i += 1; }
            let w: String = s[j..i].iter().collect::<String>().to_ascii_lowercase();
            out.push(match w.as_str() { "and" => T::Op("and"), "or" => T::Op("or"), _ => T::Id(w) });
            continue;
        }
        let two: String = s[i..(i + 2).min(s.len())].iter().collect();
        match two.as_str() {
            ":=" => { out.push(T::Assign); i += 2; continue; }
            "<=" => { out.push(T::Op("<=")); i += 2; continue; }
            ">=" => { out.push(T::Op(">=")); i += 2; continue; }
            "<>" => { out.push(T::Op("<>")); i += 2; continue; }
            _ => {}
        }
        out.push(match c {
            '(' => T::LP, ')' => T::RP, ',' => T::Comma, ';' => T::Semi,
            '<' => T::Op("<"), '>' => T::Op(">"), '=' => T::Op("="), '+' => T::Op("+"), '-' => T::Op("-"),
            '*' => T::Op("*"), '/' => T::Op("/"), '%' => T::Sym("%".into()), '$' => T::Sym("$".into()),
            _ => return Err(format!("unexpected character '{c}'")),
        });
        i += 1;
    }
    Ok(out)
}

struct P { t: Vec<T>, i: usize }
impl P {
    fn peek(&self) -> Option<&T> { self.t.get(self.i) }
    fn op(&self, o: &str) -> bool { matches!(self.peek(), Some(T::Op(x)) if *x == o) }
    fn expr(&mut self) -> Result<E, String> { self.or() }
    fn or(&mut self) -> Result<E, String> {
        let mut a = self.and()?;
        while self.op("or") { self.i += 1; let b = self.and()?; a = E::B(0, Box::new(a), Box::new(b)); }
        Ok(a)
    }
    fn and(&mut self) -> Result<E, String> {
        let mut a = self.cmp()?;
        while self.op("and") { self.i += 1; let b = self.cmp()?; a = E::B(1, Box::new(a), Box::new(b)); }
        Ok(a)
    }
    fn cmp(&mut self) -> Result<E, String> {
        let mut a = self.add()?;
        loop {
            let k = match self.peek() { Some(T::Op("<")) => 2, Some(T::Op(">")) => 3, Some(T::Op("<=")) => 4, Some(T::Op(">=")) => 5, Some(T::Op("=")) => 6, Some(T::Op("<>")) => 7, _ => break };
            self.i += 1; let b = self.add()?; a = E::B(k, Box::new(a), Box::new(b));
        }
        Ok(a)
    }
    fn add(&mut self) -> Result<E, String> {
        let mut a = self.mul()?;
        loop {
            let k = if self.op("+") { 8 } else if self.op("-") { 9 } else { break };
            self.i += 1; let b = self.mul()?; a = E::B(k, Box::new(a), Box::new(b));
        }
        Ok(a)
    }
    fn mul(&mut self) -> Result<E, String> {
        let mut a = self.un()?;
        loop {
            let k = if self.op("*") { 10 } else if self.op("/") { 11 } else { break };
            self.i += 1; let b = self.un()?; a = E::B(k, Box::new(a), Box::new(b));
        }
        Ok(a)
    }
    fn un(&mut self) -> Result<E, String> {
        if self.op("-") { self.i += 1; return Ok(E::Neg(Box::new(self.un()?))); }
        if self.op("+") { self.i += 1; return self.un(); }
        self.prim()
    }
    fn prim(&mut self) -> Result<E, String> {
        let t = self.t.get(self.i).cloned().ok_or("unexpected end of formula")?;
        self.i += 1;
        match t {
            T::Num(v) => Ok(E::N(v)),
            T::Str(s) => Ok(E::S(s)),
            T::Sym(s) => Ok(E::Sym(s)),
            T::LP => { let e = self.expr()?; if self.peek() != Some(&T::RP) { return Err("missing ')'".into()); } self.i += 1; Ok(e) }
            T::Id(w) => {
                if self.peek() == Some(&T::LP) {
                    self.i += 1;
                    let mut args = vec![];
                    if self.peek() != Some(&T::RP) {
                        loop {
                            args.push(self.expr()?);
                            match self.peek() { Some(T::Comma) => { self.i += 1; } Some(T::RP) => break, _ => return Err(format!("bad argument list in {w}()")) }
                        }
                    }
                    self.i += 1;
                    Ok(E::Call(w, args))
                } else { Ok(E::Id(w)) }
            }
            other => Err(format!("unexpected token {other:?}")),
        }
    }
}

/// A formula: optional `name := expr;` statements, then the value expression.
#[derive(Clone, Debug)]
pub struct Prog { pub defs: Vec<(String, E)>, pub value: E }

pub fn parse(src: &str) -> Result<Prog, String> {
    let t = lex(src)?;
    let mut p = P { t, i: 0 };
    let mut defs = vec![];
    loop {
        if let (Some(T::Id(n)), Some(T::Assign)) = (p.t.get(p.i).cloned(), p.t.get(p.i + 1).cloned()) {
            p.i += 2;
            let e = p.expr()?;
            defs.push((n, e));
            while p.peek() == Some(&T::Semi) { p.i += 1; }
            continue;
        }
        let e = p.expr()?;
        while p.peek() == Some(&T::Semi) { p.i += 1; }
        if p.i < p.t.len() { return Err(format!("unexpected text after formula at token {:?}", p.t[p.i])); }
        return Ok(Prog { defs, value: e });
    }
}

/// Built-ins defined by formula text (MetaStock built-ins that the anthologies spell out long-hand).
fn macro_text(name: &str, a: &[String]) -> Option<String> {
    Some(match (name, a.len()) {
        // "OBV - Raw" (Wallstreet formulas, GM1): Granville OBV
        ("obv", 0) => "Cum(If(C>Ref(C,-1),VOLUME,If(C<Ref(C,-1),-VOLUME,0)))".into(),
        // "Chaikin Oscillator ... the long-hand version" (Wallstreet formulas, GM1)
        ("co", 0) => "Mov(Cum((((C-L)-(H-C))/(H-L))*V),3,E)-Mov(Cum((((C-L)-(H-C))/(H-L))*V),10,E)".into(),
        // "Price Volume Trend" (Wallstreet formulas, GM1)
        ("pvt", 0) => "Cum(((C-Ref(C,-1))/Ref(C,-1))*V)".into(),
        // Williams' Accumulation/Distribution (MetaStock WillA): true-range-low / true-range-high based
        ("willa", 0) => "Cum(If(C>Ref(C,-1),C-Min(L,Ref(C,-1)),If(C<Ref(C,-1),C-Max(H,Ref(C,-1)),0)))".into(),
        // Williams' Ultimate Oscillator, Ult(c1,c2,c3), weights 4/2/1
        ("ult", 3) => {
            let bp = "(C-Min(L,Ref(C,-1)))"; let tr = "(Max(H,Ref(C,-1))-Min(L,Ref(C,-1)))";
            format!("100*(4*Sum({bp},{0})/Sum({tr},{0})+2*Sum({bp},{1})/Sum({tr},{1})+Sum({bp},{2})/Sum({tr},{2}))/7", a[0], a[1], a[2])
        }
        _ => return None,
    })
}

fn fin(v: f64) -> bool { v.is_finite() }
fn konst(v: f64, n: usize) -> Vec<f64> { vec![v; n] }
fn window<F: Fn(&[f64]) -> f64>(v: &[f64], n: usize, f: F) -> Vec<f64> {
    (0..v.len()).map(|i| if n == 0 || i + 1 < n { NAN } else { let w = &v[i + 1 - n..=i]; if w.iter().all(|x| fin(*x)) { f(w) } else { NAN } }).collect()
}
pub fn sma(v: &[f64], n: usize) -> Vec<f64> { window(v, n, |w| w.iter().sum::<f64>() / n as f64) }
pub fn wma(v: &[f64], n: usize) -> Vec<f64> {
    let den = (n * (n + 1) / 2) as f64;
    window(v, n, |w| w.iter().enumerate().map(|(k, x)| (k + 1) as f64 * x).sum::<f64>() / den)
}
/// MetaStock Mov(x,n,E): seed = first defined value, alpha 2/(n+1), shown from the n-th bar.
pub fn ema(v: &[f64], n: usize) -> Vec<f64> { ema_alpha(v, 2.0 / (n as f64 + 1.0), n) }
fn ema_alpha(v: &[f64], a: f64, show_from: usize) -> Vec<f64> {
    let mut out = vec![NAN; v.len()];
    let mut prev = NAN; let mut first = None;
    for i in 0..v.len() {
        if !fin(v[i]) { continue; }
        if !fin(prev) { prev = v[i]; first = Some(i); } else { prev += a * (v[i] - prev); }
        if let Some(f) = first { if i + 1 >= f + show_from { out[i] = prev; } }
    }
    out
}
/// Wilder smoothing seeded with the mean of the first n defined values.
pub fn wild(v: &[f64], n: usize) -> Vec<f64> {
    let mut out = vec![NAN; v.len()];
    let (mut cnt, mut acc, mut prev) = (0usize, 0.0, NAN);
    for i in 0..v.len() {
        if !fin(v[i]) { continue; }
        if !fin(prev) {
            cnt += 1; acc += v[i];
            if cnt == n { prev = acc / n as f64; out[i] = prev; }
            continue;
        }
        prev += (v[i] - prev) / n as f64;
        out[i] = prev;
    }
    out
}
fn back(v: &[f64], k: usize) -> Vec<f64> { (0..v.len()).map(|i| if i >= k { v[i - k] } else { NAN }).collect() }
fn zip(a: &[f64], b: &[f64], f: impl Fn(f64, f64) -> f64) -> Vec<f64> {
    a.iter().zip(b).map(|(x, y)| if fin(*x) && fin(*y) { let r = f(*x, *y); if r.is_finite() { r } else { NAN } } else { NAN }).collect()
}
fn map1(a: &[f64], f: impl Fn(f64) -> f64) -> Vec<f64> { a.iter().map(|x| if fin(*x) { let r = f(*x); if r.is_finite() { r } else { NAN } } else { NAN }).collect() }
fn near(x: f64, y: f64) -> bool { (x - y).abs() <= 1e-12 * x.abs().max(y.abs()) }
fn truth(x: f64) -> bool { fin(x) && x != 0.0 }
/// Least-squares fit over the last n bars, x = 1..n: (value at x = n, slope).
fn linreg(v: &[f64], n: usize) -> (Vec<f64>, Vec<f64>) {
    let len = v.len();
    let (mut val, mut slope) = (vec![NAN; len], vec![NAN; len]);
    let nf = n as f64; let sx = nf * (nf + 1.) / 2.; let sxx = nf * (nf + 1.) * (2. * nf + 1.) / 6.;
    let den = nf * sxx - sx * sx;
    if n < 2 { return (val, slope); }
    for i in n - 1..len {
        let w = &v[i + 1 - n..=i];
        if !w.iter().all(|x| fin(*x)) { continue; }
        let sy: f64 = w.iter().sum(); let sxy: f64 = w.iter().enumerate().map(|(k, y)| (k + 1) as f64 * y).sum();
        let b = (nf * sxy - sx * sy) / den; let a = (sy - b * sx) / nf;
        val[i] = a + b * nf; slope[i] = b;
    }
    (val, slope)
}

pub struct Eng<'a> {
    pub x: &'a Ctx<'a>,
    n: usize,
    vol: Vec<f64>,
    cache: HashMap<String, V>,
    fml: &'a dyn Fn(&str) -> Option<String>,
    vars: Vec<HashMap<String, V>>,
    depth: usize,
}

impl<'a> Eng<'a> {
    pub fn new(x: &'a Ctx<'a>, fml: &'a dyn Fn(&str) -> Option<String>) -> Self {
        let vol = x.r.iter().map(|r| r.volume.unwrap_or(NAN)).collect();
        Eng { x, n: x.c.len(), vol, cache: HashMap::new(), fml, vars: vec![], depth: 0 }
    }
    pub fn run(&mut self, p: &Prog) -> Result<V, String> {
        self.vars.push(HashMap::new());
        let r = (|| {
            for (k, e) in &p.defs { let v = self.ev(e)?; self.vars.last_mut().unwrap().insert(k.clone(), v); }
            self.ev(&p.value)
        })();
        self.vars.pop();
        r
    }
    pub fn eval_src(&mut self, src: &str) -> Result<V, String> { let p = parse(src)?; self.run(&p) }
    fn var(&self, k: &str) -> Option<V> { self.vars.last().and_then(|m| m.get(k).cloned()) }
    fn uses_var(&self, e: &E) -> bool {
        match e {
            E::Id(k) => self.var(k).is_some(),
            E::Call(_, a) => a.iter().any(|x| self.uses_var(x)),
            E::B(_, a, b) => self.uses_var(a) || self.uses_var(b),
            E::Neg(a) => self.uses_var(a),
            _ => false,
        }
    }
    fn k(&self, e: &E) -> Result<f64, String> {
        match e {
            E::N(v) => Ok(*v),
            E::Neg(a) => Ok(-self.k(a)?),
            E::B(op, a, b) if *op >= 8 => { let (a, b) = (self.k(a)?, self.k(b)?); Ok(match op { 8 => a + b, 9 => a - b, 10 => a * b, _ => a / b }) }
            E::Id(k) => match self.var(k) { Some(v) if v.iter().all(|x| *x == v[0]) && !v.is_empty() => Ok(v[0]), _ => Err(format!("'{k}' is not a constant")) },
            _ => Err("a period or constant argument must be a number".into()),
        }
    }
    fn per(&self, e: &E) -> Result<usize, String> {
        let v = self.k(e)?;
        if v < 1.0 - 1e-9 || (v - v.round()).abs() > 1e-9 { return Err(format!("period {v} is not a whole number of bars")); }
        Ok(v.round() as usize)
    }
    fn ev(&mut self, e: &E) -> Result<V, String> {
        match e {
            E::N(v) => Ok(Rc::new(konst(*v, self.n))),
            E::Neg(a) => { let a = self.ev(a)?; Ok(Rc::new(map1(&a, |x| -x))) }
            E::Id(k) => {
                if let Some(v) = self.var(k) { return Ok(v); }
                let x = self.x;
                Ok(Rc::new(match k.as_str() {
                    "o" | "open" => x.o.clone(), "h" | "high" => x.h.clone(), "l" | "low" => x.l.clone(),
                    "c" | "close" => x.c.clone(), "v" | "vol" | "volume" => self.vol.clone(),
                    _ => return Err(format!("unknown name '{k}'")),
                }))
            }
            E::S(_) | E::Sym(_) => Err("text argument outside a function".into()),
            E::B(op, a, b) => {
                let (a, b) = (self.ev(a)?, self.ev(b)?);
                let r: Vec<f64> = match op {
                    0 => zip(&a, &b, |x, y| ((x != 0.0) || (y != 0.0)) as u8 as f64),
                    1 => zip(&a, &b, |x, y| ((x != 0.0) && (y != 0.0)) as u8 as f64),
                    // comparisons treat values within 1e-12 (relative) as equal: two results that are equal in exact
                    // arithmetic (e.g. RSI on an unchanged close) must not differ by float rounding (review 25 Oct 2026)
                    2 => zip(&a, &b, |x, y| (x < y && !near(x, y)) as u8 as f64), 3 => zip(&a, &b, |x, y| (x > y && !near(x, y)) as u8 as f64),
                    4 => zip(&a, &b, |x, y| (x <= y || near(x, y)) as u8 as f64), 5 => zip(&a, &b, |x, y| (x >= y || near(x, y)) as u8 as f64),
                    6 => zip(&a, &b, |x, y| near(x, y) as u8 as f64), 7 => zip(&a, &b, |x, y| (!near(x, y)) as u8 as f64),
                    8 => zip(&a, &b, |x, y| x + y), 9 => zip(&a, &b, |x, y| x - y), 10 => zip(&a, &b, |x, y| x * y),
                    _ => zip(&a, &b, |x, y| x / y),
                };
                Ok(Rc::new(r))
            }
            E::Call(name, args) => {
                let key = if self.uses_var(e) { None } else { Some(format!("{e:?}")) };
                if let Some(k) = &key { if let Some(v) = self.cache.get(k) { return Ok(v.clone()); } }
                self.depth += 1;
                if self.depth > 60 { return Err("formula nesting too deep".into()); }
                let r = self.call(name, args);
                self.depth -= 1;
                let r = r?;
                if let Some(k) = key { self.cache.insert(k, r.clone()); }
                Ok(r)
            }
        }
    }
    fn call(&mut self, name: &str, a: &[E]) -> Result<V, String> {
        let n = self.n;
        let x = self.x;
        let argc = |m: usize| -> Result<(), String> { if a.len() == m { Ok(()) } else { Err(format!("{name}() needs {m} arguments")) } };
        let arg_txt = |e: &E| -> Result<String, String> { Ok(format!("{}", self.k(e)?)) };
        // functions spelled out as formula text
        let txt: Vec<String> = if matches!(name, "ult") { a.iter().map(arg_txt).collect::<Result<_, _>>()? } else { vec![] };
        if let Some(src) = macro_text(name, &txt) { return self.eval_src(&src); }
        let v: Vec<f64> = match name {
            "fml" => {
                let Some(E::S(nm)) = a.first() else { return Err("Fml() needs a name".into()) };
                let src = (self.fml)(nm).ok_or_else(|| format!("Fml(\"{nm}\") is not defined in the source"))?;
                return self.eval_src(&src);
            }
            "ref" | "prev" => {
                argc(2)?;
                let k = self.k(&a[1])?;
                let back_by = if name == "ref" { -k } else { k };
                if back_by < 0.0 { return Err(format!("{name}() with offset {k} reads a future bar")); }
                if (back_by - back_by.round()).abs() > 1e-9 { return Err("fractional offset".into()); }
                let s = self.ev(&a[0])?; back(&s, back_by.round() as usize)
            }
            "mov" => {
                argc(3)?;
                let p = self.per(&a[1])?;
                let m = match &a[2] { E::Id(m) => m.clone(), _ => return Err("Mov() method".into()) };
                let s = self.ev(&a[0])?;
                match m.as_str() {
                    "s" | "simple" | "simp" => sma(&s, p), "e" | "exponential" => ema(&s, p), "w" | "weighted" => wma(&s, p),
                    _ => return Err(format!("Mov() method '{m}' is not supported")),
                }
            }
            "sum" => { argc(2)?; let p = self.per(&a[1])?; let s = self.ev(&a[0])?; window(&s, p, |w| w.iter().sum()) }
            "hhv" => { argc(2)?; let p = self.per(&a[1])?; let s = self.ev(&a[0])?; window(&s, p, |w| w.iter().copied().fold(f64::MIN, f64::max)) }
            "llv" => { argc(2)?; let p = self.per(&a[1])?; let s = self.ev(&a[0])?; window(&s, p, |w| w.iter().copied().fold(f64::MAX, f64::min)) }
            "std" | "stdev" => {
                if a.len() != 2 && a.len() != 3 { return Err("Std() needs 2 or 3 arguments".into()); }
                let p = self.per(&a[1])?;
                // Window on WallStreet std(x,n,k) = k standard deviations (Bollinger band use in the source)
                let mult = if a.len() == 3 { self.k(&a[2])? } else { 1.0 };
                let s = self.ev(&a[0])?;
                window(&s, p, |w| { let m = w.iter().sum::<f64>() / p as f64; mult * (w.iter().map(|x| (x - m) * (x - m)).sum::<f64>() / p as f64).sqrt() })
            }
            "cum" => {
                argc(1)?; let s = self.ev(&a[0])?;
                let mut acc = 0.0; s.iter().map(|x| if fin(*x) { acc += x; acc } else { NAN }).collect()
            }
            "abs" => { argc(1)?; map1(&self.ev(&a[0])?, f64::abs) }
            "int" => { argc(1)?; map1(&self.ev(&a[0])?, f64::trunc) }
            "sqr" | "sqrt" => { argc(1)?; map1(&self.ev(&a[0])?, f64::sqrt) }
            "log" => { argc(1)?; map1(&self.ev(&a[0])?, |v| if v > 0.0 { v.ln() } else { NAN }) }
            "sin" => { argc(1)?; map1(&self.ev(&a[0])?, |v| v.to_radians().sin()) }
            "min" => { argc(2)?; let (p, q) = (self.ev(&a[0])?, self.ev(&a[1])?); zip(&p, &q, f64::min) }
            "max" => { argc(2)?; let (p, q) = (self.ev(&a[0])?, self.ev(&a[1])?); zip(&p, &q, f64::max) }
            "pwr" => { argc(2)?; let (p, q) = (self.ev(&a[0])?, self.ev(&a[1])?); zip(&p, &q, f64::powf) }
            "if" => {
                argc(3)?;
                let (c, t, f) = (self.ev(&a[0])?, self.ev(&a[1])?, self.ev(&a[2])?);
                (0..n).map(|i| if fin(c[i]) && fin(t[i]) && fin(f[i]) { if c[i] != 0.0 { t[i] } else { f[i] } } else { NAN }).collect()
            }
            "cross" => {
                argc(2)?; let (p, q) = (self.ev(&a[0])?, self.ev(&a[1])?);
                (0..n).map(|i| if i >= 1 && fin(p[i]) && fin(q[i]) && fin(p[i - 1]) && fin(q[i - 1]) { (p[i] > q[i] && p[i - 1] <= q[i - 1]) as u8 as f64 } else { NAN }).collect()
            }
            "barssince" => {
                argc(1)?; let s = self.ev(&a[0])?;
                let mut last: Option<usize> = None;
                (0..n).map(|i| { if truth(s[i]) { last = Some(i); } match last { Some(j) if fin(s[i]) => (i - j) as f64, _ => NAN } }).collect()
            }
            "roc" => {
                argc(3)?; let p = self.per(&a[1])?; let s = self.ev(&a[0])?; let b = back(&s, p);
                match &a[2] { E::Sym(m) if m == "%" => zip(&s, &b, |x, y| 100.0 * (x - y) / y), E::Sym(m) if m == "$" => zip(&s, &b, |x, y| x - y), _ => return Err("ROC() needs % or $".into()) }
            }
            "rsi" => {
                let (src, p) = match a.len() { 1 => (Rc::new(x.c.clone()), self.per(&a[0])?), 2 => (self.ev(&a[0])?, self.per(&a[1])?), _ => return Err("RSI() arguments".into()) };
                rsi(&src, p)
            }
            "cci" => { argc(1)?; let p = self.per(&a[0])?; cci(x, p) }
            "stoch" => { argc(2)?; let (k, s) = (self.per(&a[0])?, self.per(&a[1])?); stoch(x, k, s) }
            "willr" => {
                argc(1)?; let p = self.per(&a[0])?;
                let (hh, ll) = (window(&x.h, p, |w| w.iter().copied().fold(f64::MIN, f64::max)), window(&x.l, p, |w| w.iter().copied().fold(f64::MAX, f64::min)));
                (0..n).map(|i| { let d = hh[i] - ll[i]; if fin(d) && d > 0.0 { -100.0 * (hh[i] - x.c[i]) / d } else { NAN } }).collect()
            }
            "trix" => { argc(1)?; let p = self.per(&a[0])?; let e3 = ema(&ema(&ema(&x.c, p), p), p); let b = back(&e3, 1); zip(&e3, &b, |x, y| 100.0 * (x - y) / y) }
            "macd" => {
                argc(0)?;
                let (f, s) = (ema_alpha(&x.c, 0.15, 1), ema_alpha(&x.c, 0.075, 1));
                let mut m = zip(&f, &s, |p, q| p - q);
                for v in m.iter_mut().take(25) { *v = NAN; }
                m
            }
            "adx" | "pdi" | "mdi" => { argc(1)?; let p = self.per(&a[0])?; let (pd, md, adx) = dmi(x, p); match name { "adx" => adx, "pdi" => pd, _ => md } }
            "atr" => { argc(1)?; let p = self.per(&a[0])?; wild(&tr(x), p) }
            "sar" => { argc(2)?; let (st, mx) = (self.k(&a[0])?, self.k(&a[1])?); sar(x, st, mx) }
            "tsf" | "linregslope" | "slope" | "linearreg" => {
                argc(2)?; let p = self.per(&a[1])?; let s = self.ev(&a[0])?; let (val, sl) = linreg(&s, p);
                match name { "tsf" => zip(&val, &sl, |a, b| a + b), "linearreg" => val, _ => sl }
            }
            _ => return Err(format!("function {name}() is not defined in the source or not supported")),
        };
        Ok(Rc::new(v))
    }
}

fn tr(x: &Ctx) -> Vec<f64> {
    (0..x.c.len()).map(|i| if i == 0 { NAN } else { let pc = x.c[i - 1]; (x.h[i] - x.l[i]).max((x.h[i] - pc).abs()).max((x.l[i] - pc).abs()) }).collect()
}
/// Wilder RSI on any series (seeded with the mean of the first n changes).
pub fn rsi(c: &[f64], n: usize) -> Vec<f64> {
    let d: Vec<f64> = (0..c.len()).map(|i| if i == 0 { NAN } else { c[i] - c[i - 1] }).collect();
    let up = wild(&map1(&d, |v| v.max(0.0)), n);
    let dn = wild(&map1(&d, |v| (-v).max(0.0)), n);
    (0..c.len()).map(|i| if fin(up[i]) && fin(dn[i]) { if dn[i] == 0.0 { if up[i] == 0.0 { 50.0 } else { 100.0 } } else { 100.0 - 100.0 / (1.0 + up[i] / dn[i]) } } else { NAN }).collect()
}
/// Lambert CCI on the typical price, mean absolute deviation, constant 0.015.
pub fn cci(x: &Ctx, n: usize) -> Vec<f64> {
    let tp: Vec<f64> = (0..x.c.len()).map(|i| (x.h[i] + x.l[i] + x.c[i]) / 3.).collect();
    (0..tp.len()).map(|i| {
        if i + 1 < n { return NAN; }
        let w = &tp[i + 1 - n..=i];
        let m = w.iter().sum::<f64>() / n as f64;
        let md = w.iter().map(|v| (v - m).abs()).sum::<f64>() / n as f64;
        if md > 0. { (tp[i] - m) / (0.015 * md) } else { NAN }
    }).collect()
}
/// MetaStock Stoch(k, slowing) = 100 * Sum(C - LLV(L,k), s) / Sum(HHV(H,k) - LLV(L,k), s).
pub fn stoch(x: &Ctx, k: usize, s: usize) -> Vec<f64> {
    let hh = window(&x.h, k, |w| w.iter().copied().fold(f64::MIN, f64::max));
    let ll = window(&x.l, k, |w| w.iter().copied().fold(f64::MAX, f64::min));
    let num = window(&zip(&x.c, &ll, |c, l| c - l), s, |w| w.iter().sum());
    let den = window(&zip(&hh, &ll, |h, l| h - l), s, |w| w.iter().sum());
    zip(&num, &den, |a, b| if b > 0. { 100. * a / b } else { NAN })
}
/// Wilder +DI, -DI and ADX (all Wilder-smoothed, seeded with simple means).
pub fn dmi(x: &Ctx, n: usize) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
    let len = x.c.len();
    let (mut pdm, mut mdm) = (vec![NAN; len], vec![NAN; len]);
    for i in 1..len {
        let up = x.h[i] - x.h[i - 1]; let dn = x.l[i - 1] - x.l[i];
        pdm[i] = if up > dn && up > 0. { up } else { 0. };
        mdm[i] = if dn > up && dn > 0. { dn } else { 0. };
    }
    let (sp, sm, st) = (wild(&pdm, n), wild(&mdm, n), wild(&tr(x), n));
    let (mut pdi, mut mdi, mut dx) = (vec![NAN; len], vec![NAN; len], vec![NAN; len]);
    for i in 0..len {
        if fin(st[i]) && st[i] > 0. {
            pdi[i] = 100. * sp[i] / st[i]; mdi[i] = 100. * sm[i] / st[i];
            dx[i] = if pdi[i] + mdi[i] > 0. { 100. * (pdi[i] - mdi[i]).abs() / (pdi[i] + mdi[i]) } else { 0. };
        }
    }
    let adx = wild(&dx, n);
    (pdi, mdi, adx)
}
/// Wilder Parabolic SAR. Start: direction of bar 1's close versus bar 0's, SAR = bar 0's extreme.
pub fn sar(x: &Ctx, step: f64, maxa: f64) -> Vec<f64> {
    let n = x.c.len();
    let mut out = vec![NAN; n];
    if n < 2 { return out; }
    let mut long = x.c[1] >= x.c[0];
    let mut s = if long { x.l[0] } else { x.h[0] };
    let mut ep = if long { x.h[1].max(x.h[0]) } else { x.l[1].min(x.l[0]) };
    let mut af = step;
    out[1] = s;
    for i in 2..n {
        s += af * (ep - s);
        if long { s = s.min(x.l[i - 1]).min(x.l[i - 2]); } else { s = s.max(x.h[i - 1]).max(x.h[i - 2]); }
        if long && x.l[i] < s {
            long = false; s = ep; ep = x.l[i]; af = step;
        } else if !long && x.h[i] > s {
            long = true; s = ep; ep = x.h[i]; af = step;
        } else if long && x.h[i] > ep { ep = x.h[i]; af = (af + step).min(maxa); }
        else if !long && x.l[i] < ep { ep = x.l[i]; af = (af + step).min(maxa); }
        out[i] = s;
    }
    out
}

/// Every Fml("...") name used by a formula (recursively, through the lookup), plus whether volume is read.
pub fn scan(src: &str, fml: &dyn Fn(&str) -> Option<String>) -> (bool, Vec<String>) {
    fn walk(e: &E, vol: &mut bool, refs: &mut Vec<String>, fml: &dyn Fn(&str) -> Option<String>, d: usize) {
        match e {
            E::Id(k) if matches!(k.as_str(), "v" | "vol" | "volume") => *vol = true,
            E::Call(f, a) => {
                if matches!(f.as_str(), "obv" | "co" | "pvt") { *vol = true; }
                if f == "fml" { if let Some(E::S(n)) = a.first() { if !refs.contains(n) { refs.push(n.clone()); if d < 20 { if let Some(s) = fml(n) { if let Ok(p) = parse(&s) { for (_, e) in &p.defs { walk(e, vol, refs, fml, d + 1); } walk(&p.value, vol, refs, fml, d + 1); } } } } } }
                for x in a { walk(x, vol, refs, fml, d); }
            }
            E::B(_, a, b) => { walk(a, vol, refs, fml, d); walk(b, vol, refs, fml, d); }
            E::Neg(a) => walk(a, vol, refs, fml, d),
            _ => {}
        }
    }
    let (mut vol, mut refs) = (false, vec![]);
    if let Ok(p) = parse(src) { for (_, e) in &p.defs { walk(e, &mut vol, &mut refs, fml, 0); } walk(&p.value, &mut vol, &mut refs, fml, 0); }
    (vol, refs)
}

// ------------------------------------------------------------------ book constants as parameters
/// Distinct numeric literals of a system's own formulas (not inside Fml definitions, not Ref/prev
/// offsets, not 0 or 1), by absolute value in order of appearance. Each becomes one parameter whose
/// default is the printed value; changing it changes every use of that number in the system.
pub fn constants(srcs: &[&str]) -> Vec<f64> {
    fn walk(e: &E, out: &mut Vec<f64>) {
        match e {
            E::N(v) => { let a = v.abs(); if a != 0.0 && a != 1.0 && !out.iter().any(|x| (x - a).abs() < 1e-12) { out.push(a); } }
            E::Call(f, a) => {
                if f == "fml" { return; }
                for (i, x) in a.iter().enumerate() { if (f == "ref" || f == "prev") && i == 1 { continue; } walk(x, out); }
            }
            E::B(_, a, b) => { walk(a, out); walk(b, out); }
            E::Neg(a) => walk(a, out),
            _ => {}
        }
    }
    let mut out = vec![];
    for s in srcs { if let Ok(p) = parse(s) { for (_, e) in &p.defs { walk(e, &mut out); } walk(&p.value, &mut out); } }
    out
}
pub fn const_key(v: f64) -> String { format!("k{}", format!("{v}").replace('.', "p")) }
/// Replace book constants by parameter values (same exclusions as `constants`).
pub fn substitute(p: &mut Prog, map: &[(f64, f64)]) {
    fn walk(e: &mut E, map: &[(f64, f64)]) {
        match e {
            E::N(v) => { let a = v.abs(); if let Some((_, nv)) = map.iter().find(|(k, _)| (k - a).abs() < 1e-12) { *v = nv * v.signum(); } }
            E::Call(f, a) => {
                if f == "fml" { return; }
                let f = f.clone();
                for (i, x) in a.iter_mut().enumerate() { if (f == "ref" || f == "prev") && i == 1 { continue; } walk(x, map); }
            }
            E::B(_, a, b) => { walk(a, map); walk(b, map); }
            E::Neg(a) => walk(a, map),
            _ => {}
        }
    }
    for (_, e) in p.defs.iter_mut() { walk(e, map); }
    walk(&mut p.value, map);
}

// ------------------------------------------------------------------ simulator
/// Signals on each bar's close: enter long, close long, enter short, close short.
pub struct Sig { pub el: Vec<bool>, pub xl: Vec<bool>, pub es: Vec<bool>, pub xs: Vec<bool> }
/// entry_open / exit_open: fill at the next bar's open instead of this bar's close (trade delay 1, price open).
#[derive(Clone, Copy, Default)]
pub struct Exec { pub entry_open: bool, pub exit_open: bool }

fn new_trade(dir: f64, i: usize, px: f64, sig: usize, kind: &'static str) -> Trade {
    Trade { dir, legs: vec![Leg { i, px, w: 1. }], exit_i: i, exit_px: px, reason: String::new(), entry_kind: kind, signal_i: sig, open: false, exits: vec![] }
}

/// One position at a time. A position closes only through its own close formula (checked from the bar
/// after a close-filled entry, or from the entry bar for an open-filled entry). An exit and an opposite
/// entry on the same bar reverse the position; after an exit no same-direction re-entry on that bar.
/// Signals are evaluated only inside [from, to]; the position still open at `to` is marked to its close.
pub fn simulate(x: &Ctx, from: usize, to: usize, s: &Sig, e: Exec) -> Vec<Trade> {
    let mut out = vec![];
    let mut cur: Option<Trade> = None;
    let mut pend_exit = false;
    let mut pend_entry: Option<(f64, usize)> = None;
    for i in from..=to {
        if pend_exit {
            if let Some(mut t) = cur.take() { t.exit_i = i; t.exit_px = x.o[i]; t.signal_i = i - 1; t.reason = "Close formula · next open".into(); out.push(t); }
            pend_exit = false;
        }
        if let Some((d, sg)) = pend_entry.take() { cur = Some(new_trade(d, i, x.o[i], sg, "open")); }
        let mut exited_dir = 0.;
        if let Some(t) = cur.as_ref() {
            let live = if t.entry_kind == "close" { i > t.legs[0].i } else { i >= t.legs[0].i };
            let rule = if t.dir > 0. { s.xl[i] } else { s.xs[i] };
            if live && rule {
                if e.exit_open {
                    if i < to { pend_exit = true; exited_dir = t.dir; }
                } else {
                    let mut t = cur.take().unwrap();
                    t.exit_i = i; t.exit_px = x.c[i]; t.signal_i = i; t.reason = "Close formula · close".into(); exited_dir = t.dir; out.push(t);
                }
            }
        }
        if (cur.is_none() || pend_exit) && pend_entry.is_none() {
            let want = if s.el[i] && exited_dir <= 0. { 1. } else if s.es[i] && exited_dir >= 0. { -1. } else { 0. };
            if want != 0. {
                if e.entry_open {
                    if i < to { pend_entry = Some((want, i)); }
                } else if cur.is_none() {
                    cur = Some(new_trade(want, i, x.c[i], i, "close"));
                }
            }
        }
    }
    if let Some(mut t) = cur { t.exit_i = to; t.exit_px = x.c[to]; t.signal_i = to; t.reason = "open".into(); t.open = true; out.push(t); }
    out
}

/// Evaluate a system's four formulas (missing = never) into boolean signals.
pub fn signals(eng: &mut Eng, f: [Option<&str>; 4], map: &[(f64, f64)], side: i32) -> Result<Sig, String> {
    let n = eng.x.c.len();
    let mut v: Vec<Vec<bool>> = vec![];
    for (k, src) in f.iter().enumerate() {
        let on = match k { 0 | 1 => side >= 0, _ => side <= 0 };
        match src {
            Some(s) if on => {
                let mut p = parse(s)?;
                substitute(&mut p, map);
                let r = eng.run(&p)?;
                v.push(r.iter().map(|x| truth(*x)).collect());
            }
            _ => v.push(vec![false; n]),
        }
    }
    let xs = v.pop().unwrap(); let es = v.pop().unwrap(); let xl = v.pop().unwrap(); let el = v.pop().unwrap();
    Ok(Sig { el, xl, es, xs })
}

// ===================================================================== catalogue / params / run (shared)
use crate::connors::{ParamDef, Params, Status, StrategyDef};
use std::sync::OnceLock;
#[allow(unused_imports)]
use Status::{Live, NotTestable, NeedsData};

pub struct Sys {
    pub id: &'static str,
    pub name: &'static str,
    pub group: &'static str,
    pub family: (&'static str, &'static str),
    pub page: &'static str,
    pub status: Status,
    /// Enter Long, Close Long, Enter Short, Close Short, exactly as printed (None = not printed)
    pub f: [Option<&'static str>; 4],
    pub exec: Exec,
    pub notes: &'static str,
}

/// One anthology: its systems, its Fml() custom-indicator file and the text appended to every note.
pub struct Book {
    pub id: &'static str,
    pub sys: &'static [Sys],
    pub fml: &'static [(&'static str, &'static str)],
    pub conv: &'static str,
    pub cat: OnceLock<Vec<(&'static [&'static str], &'static str, bool)>>,
    pub par: OnceLock<Vec<(&'static str, Vec<ParamDef>)>>,
}

fn leak(s: String) -> &'static str { Box::leak(s.into_boxed_str()) }

impl Book {
    pub fn fml_lookup(&self, name: &str) -> Option<String> {
        let n = name.replace(['\u{2019}', '\u{2018}'], "'");
        self.fml.iter().find(|(k, _)| k.eq_ignore_ascii_case(n.trim())).map(|(_, v)| v.to_string())
    }
    fn rules_of(&'static self, s: &Sys) -> &'static [&'static str] {
        let lab = ["E|Enter Long: ", "X|Close Long: ", "E|Enter Short: ", "X|Close Short: "];
        let mut v: Vec<&'static str> = s.f.iter().zip(lab).filter_map(|(f, l)| f.map(|f| leak(format!("{l}{f}")))).collect();
        let lk = |n: &str| self.fml_lookup(n);
        let mut refs = vec![];
        for f in s.f.iter().flatten() { for r in scan(f, &lk).1 { if !refs.contains(&r) { refs.push(r); } } }
        for r in refs {
            let def = self.fml_lookup(&r).unwrap_or_else(|| "NOT DEFINED IN THE SOURCE".into());
            v.push(leak(format!("F|Fml(\"{r}\") = {def}")));
        }
        Box::leak(v.into_boxed_slice())
    }
    pub fn catalogue(&'static self) -> Vec<StrategyDef> {
        let cache = self.cat.get_or_init(|| self.sys.iter().map(|s| {
            let lk = |n: &str| self.fml_lookup(n);
            let vol = s.status == Status::Live && s.f.iter().flatten().any(|f| scan(f, &lk).0);
            let fills = if s.exec.entry_open { "Fills: next bar's open for entries and exits. " } else { "Fills: MetaStock System Tester defaults (trade price = close, trade delay = 0). " };
            let notes = if s.status == Status::Live { leak(format!("{} Source p. {}. {}{}", s.notes, s.page, fills, self.conv)) } else { leak(format!("{} Source p. {}.", s.notes, s.page)) };
            (self.rules_of(s), notes, vol)
        }).collect());
        self.sys.iter().zip(cache).map(|(s, (rules, notes, vol))| StrategyDef {
            id: s.id, name: s.name, group: s.group, session: 0,
            dir: match (s.f[0].is_some(), s.f[2].is_some()) { (true, true) => "Long/Short", (true, false) => "Long", (false, true) => "Short", _ => "Long/Short" },
            status: s.status, needs_volume: *vol, needs_vix: false, needs_series: &[], book: self.id, rules, notes, claim: None,
        }).collect()
    }
    pub fn family_of(&self, id: &str) -> Option<(&'static str, &'static str, &'static str)> {
        let s = self.sys.iter().find(|s| s.id == id)?;
        let variant = match s.status { Status::Live => s.name, Status::NeedsData(_) => "data missing", Status::NotTestable(_) => "not testable" };
        Some((s.family.0, s.family.1, variant))
    }
    fn consts_of(s: &Sys) -> Vec<f64> { let srcs: Vec<&str> = s.f.iter().flatten().copied().collect(); constants(&srcs) }
    pub fn params(&self, id: &str) -> Option<Vec<ParamDef>> {
        let all = self.par.get_or_init(|| self.sys.iter().filter(|s| s.status == Status::Live).map(|s| {
            let both = s.f[0].is_some() && s.f[2].is_some();
            let mut v = vec![];
            if both { v.push(ParamDef { key: "side", label: "Sides traded: 0 = both as printed, 1 = long side only, -1 = short side only", def: 0., min: -1., max: 1., step: 1. }); }
            for c in Self::consts_of(s) {
                let int = c.fract() == 0.0;
                let (min, max, step) = if int && c >= 5.0 { ((c * 0.5).round().max(1.0), (c * 2.0).round(), 1.0) } else { (c * 0.5, c * 2.0, (c / 10.0 * 1000.0).round() / 1000.0) };
                v.push(ParamDef { key: leak(const_key(c)), label: leak(format!("Book constant {c} (every use of {c} in this system's printed formulas; Ref offsets excluded)")), def: c, min, max, step: if step > 0.0 { step } else { 0.001 } });
            }
            (s.id, v)
        }).collect());
        all.iter().find(|(k, _)| *k == id).map(|(_, v)| v.iter().map(|d| ParamDef { key: d.key, label: d.label, def: d.def, min: d.min, max: d.max, step: d.step }).collect())
    }
    pub fn run(&'static self, id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
        let s = self.sys.iter().find(|s| s.id == id)?;
        if s.status != Status::Live || from >= to || to >= x.r.len() { return None; }
        let side = p.get("side").copied().unwrap_or(0.0).round() as i32;
        let map: Vec<(f64, f64)> = Self::consts_of(s).into_iter().map(|c| (c, p.get(&const_key(c)).copied().unwrap_or(c))).collect();
        let lk = |n: &str| self.fml_lookup(n);
        let fl: &dyn Fn(&str) -> Option<String> = &lk;
        let mut eng = Eng::new(x, fl);
        let sig = signals(&mut eng, s.f, &map, side).ok()?;
        Some(simulate(x, from, to, &sig, s.exec))
    }
    /// Evaluate every Live system on a context; returns (id, error) for any that fail.
    pub fn check_all(&'static self, x: &Ctx) -> Vec<(String, String)> {
        let mut bad = vec![];
        for s in self.sys.iter().filter(|s| s.status == Status::Live) {
            let lk = |n: &str| self.fml_lookup(n);
            let fl: &dyn Fn(&str) -> Option<String> = &lk;
            let mut eng = Eng::new(x, fl);
            if let Err(e) = signals(&mut eng, s.f, &[], 0) { bad.push((s.id.to_string(), e)); }
        }
        bad
    }
}

pub const BOOK: &str = "msws";
/// Admitted ids (authors/morris/admission/SOURCE-CONTRACT.md): every Live system of the GM1-GM9 series except the 3 the accuracy gate withholds (fixed point / volume units).
pub const REVIEWED: &[&str] = &["msws_3_crows_soldiers_mfi", "msws_3_crows_soldiers_pctd", "msws_3_crows_soldiers_rsi", "msws_all_cci", "msws_all_mfi", "msws_all_pctd", "msws_all_rsi", "msws_arms_eom_norm_pmzero", "msws_cci_100_100_crossover", "msws_cci_100_100_crossover_entry_exit_w_stops_chg", "msws_cci_100_100_crossover_entry_exit_w_stops_ma", "msws_cci_100_100_crossover_w_stops", "msws_dark_cloud_piercing_line_cci", "msws_dark_cloud_piercing_line_mfi", "msws_dark_cloud_piercing_line_pctd", "msws_dark_cloud_piercing_line_rsi", "msws_directional_movement_roc_ma_system", "msws_directional_movement_roc_ma_system_w_stops", "msws_directional_movement_roc_system", "msws_doji_stars_cci", "msws_doji_stars_mfi", "msws_doji_stars_pctd", "msws_doji_stars_rsi", "msws_engulfing_cci", "msws_engulfing_mfi", "msws_engulfing_pctd", "msws_engulfing_rsi", "msws_envelope_crossings", "msws_generic_oscillator_w_ma_gm1", "msws_generic_oscillator_w_ma_gm2", "msws_generic_oscillator_w_ma_gm4", "msws_generic_oscillator_w_threshold_and_ma_gm2", "msws_generic_oscillator_w_threshold_and_ma_gm4", "msws_generic_oscillator_w_threshold_gm2", "msws_generic_oscillator_w_threshold_gm4", "msws_gm6_cci_100_100_system", "msws_gm6_cci_zero_cross_system", "msws_gm6_macd_mo_trader", "msws_gm6_pctv_system", "msws_gm6_rsi_quality_system", "msws_gm6_swing_line_crossover", "msws_gm6_va_crossover_system", "msws_gm7_adx_system", "msws_gm7_barics_pctb_system", "msws_gm7_pctm_and_std_dev", "msws_gm8_raff_obv_system", "msws_hanging_man_hammer_cci", "msws_hanging_man_hammer_mfi", "msws_hanging_man_hammer_pctd", "msws_hanging_man_hammer_rsi", "msws_harami_cci", "msws_harami_mfi", "msws_harami_pctd", "msws_harami_rsi", "msws_linear_trend_system", "msws_linear_trend_system_w_entry_exit_chg", "msws_linear_trend_system_w_ma", "msws_macd_true", "msws_meeting_lines_cci", "msws_meeting_lines_mfi", "msws_meeting_lines_pctd", "msws_meeting_lines_rsi", "msws_morris_master_trader", "msws_morris_volume_rsi", "msws_parabolic_sar_crossover", "msws_pctr_20_80_crossover", "msws_pctr_20_80_crossover_entry_exit_w_stops_chg", "msws_pctr_20_80_crossover_entry_exit_w_stops_ma", "msws_pctr_20_80_crossover_w_stops", "msws_positive_vol_crossover", "msws_price_volume_trend_w_ma", "msws_pring_s_kst_w_moving_average", "msws_pring_s_kst_w_threshold_and_ma", "msws_pring_s_short_kst_on_volume_w_moving_avg", "msws_roc_crossover", "msws_roc_crossover_w_stops", "msws_rsi_30_70_crossover", "msws_rsi_30_70_crossover_entry_exit_w_stops_chg", "msws_rsi_30_70_crossover_entry_exit_w_stops_ma", "msws_rsi_30_70_crossover_w_stops", "msws_rsi_pctd_and_cci_trader", "msws_stars_cci", "msws_stars_mfi", "msws_stars_pctd", "msws_stars_rsi", "msws_stochastics_pctd_20_80_w_stops", "msws_stochastics_pctk_20_80_crossover", "msws_stochastics_pctk_20_80_crossover_w_stops", "msws_stochastics_pctk_20_80_entry_exit_w_stops_ma", "msws_triple_averages_in_sync", "msws_trix_entry_exit_chg", "msws_trix_entry_exit_ma", "msws_trix_entry_exit_w_stops_chg"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }
pub const EXEC_DEFAULT: Exec = Exec { entry_open: false, exit_open: false };
pub const EXEC_NEXT_OPEN: Exec = Exec { entry_open: true, exit_open: true };
const CONV: &str = "One position at a time; a position closes only by its own Close formula (from the bar after a close-filled entry); an exit plus an opposite entry on one bar reverse; no same-direction re-entry on the exit bar; parameter side = 1/-1 runs one side alone. Indicators: MetaStock definitions (library/msws/README.md). India: runs on NIFTY 50 / indices instead of the stocks and futures it was written for (instrument swap only); price-point and volume-unit constants are applied literally.";
pub static MSWS: Book = Book { id: BOOK, sys: SYS, fml: FML, conv: CONV, cat: OnceLock::new(), par: OnceLock::new() };
pub fn catalogue() -> Vec<StrategyDef> { MSWS.catalogue() }
pub fn params(id: &str) -> Option<Vec<ParamDef>> { MSWS.params(id) }
pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> { MSWS.run(id, x, from, to, p) }
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { MSWS.family_of(id) }

const FML: &[(&str, &str)] = &[
    (r##"Candle - 3 Black Crows"##, r##"If(Ref((OPEN-C),-2)>Mov(Abs(OPEN-C),10,S), {long black} If(Ref((OPEN-C),-1)>Mov(Abs(OPEN-C),10,S), {long black} If((OPEN-C)>Mov(Abs(OPEN-C),10,S), {long black} If(Ref((Abs(OPEN+C)/2),-1)<Ref((Abs(OPEN+C)/2),-2), {lower midpoint} If((Abs(OPEN+C)/2)<Ref((Abs(OPEN+C)/2),-1), {lower midpoint} -1,0),0),0),0),0)"##),
    (r##"Candle - 3 White Soldiers"##, r##"If(Ref((C-OPEN),-2)>Mov(Abs(OPEN-C),10,S), {long white} If(Ref((C-OPEN),-1)>Mov(Abs(OPEN-C),10,S), {long white} If((C-OPEN)>Mov(Abs(OPEN-C),10,S), {long white} If(Ref((Abs(OPEN+C)/2),-1)>Ref((Abs(OPEN+C)/2),-2), {higher midpoint} If((Abs(OPEN+C)/2)>Ref((Abs(OPEN+C)/2),-1), {higher midpoint} 1,0),0),0),0),0)"##),
    (r##"Candle - All Bearish"##, r##"Fml("Candle - 3 Black Crows")+ Fml("Candle - Dark Cloud Cover")+ Fml("Candle - Evening Doji")+ Fml("Candle - Engulfing - Bearish")+ Fml("Candle - Harami - Bearish")+ Fml("Candle - Evening Star")+ Fml("Candle - Meeting Lines - Bearish")"##),
    (r##"Candle - All Bullish"##, r##"Fml("Candle - 3 White Soldiers")+ Fml("Candle - Piercing Line")+ Fml("Candle - Morning Doji")+ Fml("Candle - Engulfing - Bullish")+ Fml("Candle - Harami - Bullish")+ Fml("Candle - Morning Star")+ Fml("Candle - Meeting Lines - Bullish")"##),
    (r##"Candle - Dark Cloud Cover"##, r##"If(Ref(C-OPEN,-1)>(Mov(Abs(OPEN-C),10,S)), {long white} If(C<Ref(C,-1),If(C>Ref(OPEN,-1), {close within previous body} If(Ref(Abs((OPEN+C)/2),-1)>Ref(Mov(C,10,E),-1), {uptrend} If(OPEN>Ref(H,-1), {open at new high} -1,0),0),0),0),0)"##),
    (r##"Candle - Engulfing - Bearish"##, r##"If(Ref(OPEN,-1)<Ref(C,-1),If(OPEN-C>(Mov(Abs(OPEN-C),10,S)) ,If(C<Ref(OPEN,-1),If(Ref(Abs((OPEN+C)/2),-1)>Ref(Mov(C,10,E),-1), If(OPEN>Ref(C,-1),-1,0),0),0),0),0)"##),
    (r##"Candle - Engulfing - Bullish"##, r##"If(Ref(OPEN,-1)>Ref(C,-1),If(C-OPEN>(Mov(Abs(OPEN-C),10,S)) ,If(C>Ref(OPEN,-1),If(Ref(Abs((OPEN+C)/2),-1)<Ref(Mov(C,10,E),-1), If(OPEN<Ref(C,-1),1,0),0),0),0),0)"##),
    (r##"Candle - Evening Doji"##, r##"If(Ref(C-OPEN,-2)>(Mov(Abs(OPEN-C),10,S)),If(Ref(Abs(C-OPEN),-1)<((Mov(Abs(OPEN-C),10,S))/10),If(Ref(C,-1)>Ref(C,-2),If(Ref(OPEN,-1)>Ref(OPEN,-2), If(OPEN<Ref(C,-1),If(C<Ref(C,-1),-1,0),0),0),0),0),0)"##),
    (r##"Candle - Evening Star"##, r##"If(Ref(C-OPEN,-2)>(Mov(Abs(OPEN-C),10,S)), If(Ref(Abs(C-OPEN),-1)<(Mov(Abs(OPEN-C),10,S))/2, If(Ref(C,-1)>Ref(C,-2), If(Ref(OPEN,-1)>Ref(OPEN,-2), If(C<Ref((OPEN+C)/2,-2),-1,0),0),0),0),0)"##),
    (r##"Candle - Hammer"##, r##"If(Abs((H+L)/2)<Ref(Mov(C,10,E),-1), {down trend} If(Min(OPEN,C)>H-((H-L)/3), {body in upper 1/3} If(C<Ref(C,-1),If(OPEN<Ref(OPEN,-1), {body gap} 1,0),0),0),0)"##),
    (r##"Candle - Hanging Man'"##, r##"If(Abs((H+L)/2)>Ref(Mov(C,10,E),-1), {up trend} If(Min(OPEN,C)>H-((H-L)/3), {body in upper 1/3} If(C>Ref(C,-1),If(OPEN>Ref(OPEN,-1), {body gap} -1,0),0),0),0)"##),
    (r##"Candle - Harami - Bearish"##, r##"If(C<(OPEN), {black day} If(Ref(C-OPEN,-1)>Mov(Abs(OPEN-C),10,S), {long white} If(C>Ref(OPEN,-1),If(OPEN<Ref(C,-1), {engulfment} If(Ref(Abs((H+L)/2),-1)>Ref(Mov(C,10,E),-1), {up trend} -1,0),0),0),0),0)"##),
    (r##"Candle - Harami - Bullish"##, r##"If(C>OPEN, {white day} If(Ref(OPEN-C,-1)>Mov(Abs(OPEN-C),10,S), {long black} If(C<Ref(OPEN,-1),If(OPEN>Ref(C,-1), {engulfment} If(Ref(Abs((H+L)/2),-1)<Ref(Mov(C,10,E),-1), {down trend} 1,0),0),0),0),0)"##),
    (r##"Candle - Meeting Lines - Bearish"##, r##"If(Ref(C-OPEN,-1)>(Mov(Abs(OPEN-C),10,S)), If(OPEN-C>(Mov(Abs(OPEN-C),10,S)),If(Abs(C-Ref(C,-1)) <((Mov(Abs(OPEN-C),10,S))/10),-1,0),0),0)"##),
    (r##"Candle - Meeting Lines - Bullish"##, r##"If(Ref(OPEN-C,-1)>(Mov(Abs(OPEN-C),10,S)), {long black} If(C-OPEN>(Mov(Abs(OPEN-C),10,S)), {long white} If(Abs(C-Ref(C,-1))<((Mov(Abs(OPEN-C),10,S))/10), {doji close} 1,0),0),0)"##),
    (r##"Candle - Money Flow Index"##, r##"Mov(100 - (100/(1+(( Mov((If(C - Ref(C,-1)>0,(C-Ref(C,-1)*V),0)),14,E)  /  Mov((If(C - Ref(C,-1)<0,(C-Ref(C,-1)*V),0)),14,E))))),3,E)"##),
    (r##"Candle - Morning Doji"##, r##"If(Ref(OPEN-C,-2)>(Mov(Abs(OPEN-C),10,S)),If(Ref(Abs(C-OPEN),-1)<((Mov(Abs(OPEN-C),10,S))/10),If(Ref(C,-1)<Ref(C,-2),If(Ref(OPEN,-1)<Ref(OPEN,-2), If(OPEN>Ref(C,-1),If(C>Ref(C,-1),1,0),0),0),0),0),0)"##),
    (r##"Candle - Morning Star"##, r##"If(Ref(OPEN-C,-2)>(Mov(Abs(OPEN-C),10,S)), If(Ref(Abs(C-OPEN),-1)<(Mov(Abs(OPEN-C),10,S))/2, If(Ref(C,-1)<Ref(C,-2), If(Ref(OPEN,-1)<Ref(OPEN,-2), If(C>Ref((OPEN+C)/2,-2),1,0),0),0),0),0)"##),
    (r##"Candle - Piercing Line"##, r##"If(C-OPEN>(Mov(Abs(OPEN-C),10,S)), {long white} If(Ref(OPEN-C,-1)>(Mov(Abs(OPEN-C),10,S)), {long black} If(C>Ref(C,-1),If(C<Ref(OPEN,-1), {close inside previous body} If(Ref(Abs((OPEN+C)/2),-1)<Ref(Mov(C,10,E),-1), {down trend} If(OPEN<Ref(L,-1),1,0),0),0),0),0),0)"##),
    (r##"Candle - Stochastics %D"##, r##"Mov((((CLOSE - LLV(L,14))/ (HHV(H,14)-LLV(L,14)))*100),3,S)"##),
    (r##"GM6(83) - Commodity Channel Index"##, r##"(((H+L+C)/3) - Mov(((H+L+C)/3),11,S))/(Mov((Abs(((H+L+C)/3) - Mov(((H+L+C)/3),10,S))),11,S)*.015)"##),
    (r##"GM6(83) - Swing Line'"##, r##"If(Mov(C,9,S)>Ref(Mov(C,9,S),-1),(If((H>Ref(H,-1) AND Ref(H,-1)>Ref(H,-2)),H,L)),If(Mov(C,9,S)<Ref(Mov(C,9,S),-1),(If((L<Ref(L,-1) AND Ref(L,-1)<Ref(L,-2)),L,H)),C))"##),
    (r##"GM6(85) - Morris Volume RSI"##, r##"100 - 100/(1+(Mov(If(ROC(C,1,$)>0 ,ROC(C,1,$)*V,0),14,S)/Mov(If(ROC(C,1,$)<0,-ROC(C,1,$)*V,0),14,S))))"##),
    (r##"GM6(86) - RSI Quality Factor"##, r##"(RSI(14) -50)/ Mov((Abs(RSI(14) - 50)),14,S)"##),
    (r##"GM6(88) - MACD-MO"##, r##"Mov(ROC(Mov((Mov(C,10,E) - Mov(C,20,E)),10,E),10,$),3,E)"##),
    (r##"GM6(89) - %V"##, r##"(100*Mov((Abs(O-H)),21,E))/((Mov((Abs(O-H)),21,E) + Mov((Abs(O-L)),21,E)))"##),
    (r##"GM6(89) - Channel Breakout-12"##, r##"If(C > Ref((HHV(H,12)),-1),1,If(C < Ref((LLV(L,12)),-1),-1,0))"##),
    (r##"GM6(89) - Channel Breakout-8"##, r##"If(C > Ref((HHV(H,8)),-1),1,If(C < Ref((LLV(L,8)),-1),-1,0))"##),
    (r##"GM6(89) - EOM"##, r##"Mov(((((H+L)/2) - Ref(((H+L)/2),-1))/((H+L)/2)*100) /(((V-Mov(V,21,S)/Mov(V,30,S)) / ((H+L)/2)) / Mov(((H+L)/2),21,S)*100),13,E)"##),
    (r##"GM6(89) - Up Volatility - Down Volatility"##, r##"Mov((Abs(O-H)),21,E) - Mov((Abs(O-L)),21,E)"##),
    (r##"GM6(89) - Volume Accumulator (Chaikin)"##, r##"Mov((V*(((C-L) - (H-C))/(H-L))),5,S)"##),
    (r##"GM7(90) - %B (Barics)"##, r##"100*(Fml("GM7(90) - TsubB") / (Fml("GM7(90) - TsubB") + Fml("GM7(90) - TsubS")))"##),
    (r##"GM7(90) - TsubB"##, r##"If((H>O),(H-O+C-L),(C-L))"##),
    (r##"GM7(90) - TsubS"##, r##"If(O>L,(O-L+H-C),(H-C))"##),
    (r##"GM7(91) - %M Smoothed"##, r##"Mov(100*(((2*C) - HHV(H,11) - LLV(L,11)) / (HHV(H,11) - LLV(L,11))),5,E)"##),
    (r##"GM7(91) - (%M) Midpoint Oscillator"##, r##"100*(((2*C) - HHV(H,11) - LLV(L,11)) / (HHV(H,11) - LLV(L,11)))"##),
    (r##"GM7(91) - Double Smooth Stochastic"##, r##"100*(Mov(Mov((C-LLV(L,5)),3,E),3,E) / Mov(Mov((HHV(H,5) - LLV(L,5)),3,E),3,E))"##),
    (r##"GM7(91) - HLC Index"##, r##"100*(Mov(Mov((C-L),5,E),15,E) / Mov(Mov((H-L),5,E),15,E))"##),
    (r##"GM7(91) - Maddox Momentum"##, r##"Sum(((C-Ref(C,-1))*V),10)/100"##),
    (r##"GM7(91) - Pivot Points"##, r##"If((Mov(((H+L+C)/3),9,S)) > Ref(Mov(((H+L+C)/3),9,S),-3), ((((H+L+C)/3)*2) - Ref(L,-1)), ((((H+L+C)/3)*2) - Ref(H,-1)))"##),
    (r##"GM7(91) - True Strength Index"##, r##"Mov(Mov((C-Ref(C,-1)),15,E),5,E) / Mov(Mov((Abs(C-Ref(C,-1))),15,E),5,E)"##),
    (r##"GM7(91) - Variable Length Average"##, r##"If(Stdev(C,10)>1,If(Stdev(C,10)>1.5,Mov(C,8,E),Mov(C,10,S)),Mov(C,15,S))"##),
    (r##"GM8(92) - Fast TAI"##, r##"Std(Mov(C,38,S),4)"##),
    (r##"GM8(92) - Mass Index"##, r##"Sum(Mov((H-L),9,E)/Mov(Mov((H-L),9,E),9,E),25)"##),
    (r##"GM8(92) - Pring's Short KST"##, r##"(Mov(ROC(C,10,%),10,S))+(2*(Mov(ROC(C,15,%),10,S)))+(3*(Mov(ROC(C,20,%),10,S)))+(4*(Mov(ROC(C,30,%),15,S)))/10"##),
    (r##"GM8(92) - RMI Derivation #2"##, r##"(100-(100/(1+ Sum(If(C>Ref(C,-5),C-Ref(C,-5),0),20)/ Sum(If(C<Ref(C,-5),Abs(C-Ref(C,-5)),0),20))))"##),
    (r##"GM8(92) - Slow TAI"##, r##"Std(Mov(C,38,S),10)"##),
    (r##"GM8(92) - Two Stochastics"##, r##"If(Stoch(10,3)>80, If(Stoch(30,3)>80, 2,1), If(Stoch(10,3)<20, If(Stoch(30,3)<20, -2, -1),0))"##),
    (r##"GM8(92) - Volume Weighted RSI"##, r##"(100-(100/(1+ Sum(If((H+L+C)/3>Ref(((H+L+C)/3),-1),V*((H+L+C)/3),0),14) / Sum( (If((H+L+C)/3<Ref(((H+L+C)/3),-1),V*((H+L+C)/3),0)),14))))"##),
    (r##"GM8(93) - Coppock Guide"##, r##"Mov((ROC(C,14,%) + ROC(C,11,%)),10,W)"##),
    (r##"GM8(93) - DYR Implied Risk"##, r##"100*(((C-Mov(C,10,S))/Mov(C,10,S))*33*2* ((Sqrt(365)/Sqrt(10))/(22)))"##),
    (r##"GM8(93) - Smoothed Offset Momentum"##, r##"Mov(C,30,S) - Ref(Mov(C,30,S),-10)"##),
    (r##"GM9(94) - Hybrid Indicator"##, r##"(Stoch(14,3) + RSI(14)) / 2"##),
    (r##"GM9(94) - Polarized Fractal Efficiency"##, r##"Mov(If(C>Ref(C,-14),Sqr(Pwr(ROC(C,14,$),2)+Pwr(15,2))/Sum(Sqr(Pwr(ROC(C,1,$),2)+1),14),-1*Sqr(Pwr(ROC(C,14,$),2)+Pwr(15,2))/Sum(Sqr(Pwr(ROC(C,1,$),2)+1),14))*100,5,E)"##),
    (r##"GM9(95) - CCI-5 day smooth"##, r##"Mov((C-Mov(C,20,S))/(.015*Stdev(C,20)),5,E)"##),
    (r##"GM9(95) - DJROC"##, r##"Mov(((C/(Ref(C,11)))-1)*100,10,W)"##),
    (r##"GM9(95) - Historical Volatility Indicator"##, r##"Std(Log(C/Ref(C,-1)),10)/Std(Log(C/Ref(C,-1)),100)"##),
    (r##"GM9(95) - Modified VIX"##, r##"((C-Mov(C,21,S))/Mov(C,21,E))*100*33*2*(((Sqrt(252)/Sqrt(21))/(50)))"##),
    (r##"GM9(95) - Tight & Narrow - High"##, r##"If((5*HHV(H,5)-Sum(H,5))+(Sum(L,5)-5*(LLV(L,5))<2*(HHV(H,5)-5*LLV(L,5))),HHV(H,5),Peak(1,Cross(.0001,If((5*HHV(H,5)-Sum(H,5))+(Sum(L,5)-5*LLV(L,5))<2*(HHV(H,5)-LLV(L,5)),1,0))*Ref(HHV(H,5),-1),1))"##),
    (r##"GM9(95) - Tight & Narrow - Low"##, r##"If((5*HHV(H,5)-Sum(H,5)+(Sum(L,5)-5*LLV(L,5))<2*(HHV(H,5)-LLV(L,5))),LLV(L,5),Peak(1,Cross(.0001,If((5*HHV(H,5)-Sum(H,5))+(Sum(L,5)-5*LLV(L,5))<2*(HHV(H,5)-LLV(L,5)),1,0))*Ref(LLV(L,5),-1),1))"##),
    (r##"GM9(95) - Volume Momentum"##, r##"Mov(ROC((Cum(If(C>(H+L)/2,V, If(C<(H+L)/2,-V,0)))),12,$),10,S)"##),
    (r##"New - CCI"##, r##"(((H+L+C)/3) - Mov(((H+L+C)/3),11,S))/ (Mov((Abs(((H+L+C)/3) - Mov(((H+L+C)/3),10,S))),11,S)*.015)"##),
    (r##"New - Directional Movement ROC"##, r##"ROC((PDI(14) - MDI(14)),12,$)"##),
    (r##"New - Linear Trend Oscillator"##, r##"(Sin(((10*(Sum(Cum(1)*C,10)))-(Sum(Cum(1),10)*(Sum(C,10))))/((10*Sum(Pwr(Cum(1),2),10))-Pwr(Sum(Cum(1),10),2))) - Sin(((20*(Sum(Cum(1)*C,20)))-(Sum(Cum(1),20)*(Sum(C,20))))/((20*Sum(Pwr(Cum(1),2),20))-Pwr(Sum(Cum(1),20),2))))*100"##),
    (r##"New - Morris Double Momentum Oscillator"##, r##"Mov(((ROC(C,12.8,%))+(ROC(C,19.2,%))),10,W)"##),
    (r##"New - Pring's Short KST - Daily"##, r##"(Mov(ROC(C,10,%),10,S))+(2*(Mov(ROC(C,15,%),10,S)))+(3*(Mov(ROC(C,20,%),10,S)))+(4*(Mov(ROC(C,30,%),15,S)))/10"##),
    (r##"POP - Stochastics %D'"##, r##"Mov((((CLOSE - LLV(L,14))/(HHV(H,14)-LLV(L,14)))*100),3,S)"##),
    (r##"Volume - Arms Ease of Movement (Normalized)"##, r##"Mov(((((H+L)/2) - Ref(((H+L)/2),-1))/((H+L)/2)*100) / (((V-Mov(V,30,S)/Mov(V,30,S)) / ((H+L)/2)) / Mov(((H+L)/2),30,S)*100),13,E)"##),
    (r##"Volume - OBV - Raw"##, r##"Cum(If(C>Ref(C,-1),VOLUME,If(C<Ref(C,-1),-VOLUME,0)))"##),
    (r##"Volume - Price Volume Trend"##, r##"Cum(((C-Ref(C,-1))/Ref(C,-1))*V)"##),
];

const SYS: &[Sys] = &[
    Sys { id: "msws_arms_eom_norm_pmzero", name: r##"Arms EOM (norm) +/-zero"##, group: "GM1 · Volume and price", family: ("msws_gm1", "Morris GM1 volume and price systems"), page: "1", status: Live, f: [Some(r##"Mov(Fml("Volume - Arms Ease of Movement (Normalized)"),5,S) > 0"##), Some(r##"Mov(Fml("Volume - Arms Ease of Movement (Normalized)"),5,S) < 0"##), Some(r##"Mov(Fml("Volume - Arms Ease of Movement (Normalized)"),5,S) < 0"##), Some(r##"Mov(Fml("Volume - Arms Ease of Movement (Normalized)"),5,S) > 0"##)], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "msws_generic_oscillator_w_ma_gm1", name: r##"Generic Oscillator w/ MA"##, group: "GM1 · Volume and price", family: ("msws_gm1", "Morris GM1 volume and price systems"), page: "1", status: Live, f: [Some(r##"Fml("Volume - OBV - Raw") > Mov(Fml("Volume - OBV - Raw"),7,S)"##), Some(r##"Fml("Volume - OBV - Raw") < Mov(Fml("Volume - OBV - Raw"),7,S)"##), Some(r##"Fml("Volume - OBV - Raw") < Mov(Fml("Volume - OBV - Raw"),7,S)"##), Some(r##"Fml("Volume - OBV - Raw") < Mov(Fml("Volume - OBV - Raw"),7,S)"##)], exec: EXEC_DEFAULT, notes: r##"Printed as a template ("change the item(s) in the fml function"); coded with the indicator printed (OBV - Raw). Close Short is printed identical to Enter Short (OBV < its 7-day MA) and is coded literally."## },
    Sys { id: "msws_generic_oscillator_w_threshold_and_ma_gm1", name: r##"Generic Oscillator w/ threshold and MA"##, group: "GM1 · Volume and price", family: ("msws_gm1", "Morris GM1 volume and price systems"), page: "1", status: NotTestable("template only: the source prints OBV with Stochastics thresholds 20/80 and says \"You will also need to adjust the upper and lower thresholds to an appropriate number\"; no thresholds are given for OBV"), f: [Some(r##"(Fml("Volume - OBV - Raw") > Mov(Fml("Volume - OBV - Raw"),5,S)) AND (Fml("Volume - OBV - Raw") < 20)"##), Some(r##"Fml("Volume - OBV - Raw") < Mov(Fml("Volume - OBV - Raw"),5,S) AND (Fml("Volume - OBV - Raw") > 80)"##), Some(r##"Fml("Volume - OBV - Raw") < Mov(Fml("Volume - OBV - Raw"),5,S) AND (Fml("Volume - OBV - Raw") > 80)"##), Some(r##"Fml("Volume - OBV - Raw") > Mov(Fml("Volume - OBV - Raw"),5,S) AND (Fml("Volume - OBV - Raw") < 20)"##)], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "msws_positive_vol_crossover", name: r##"Positive Vol Crossover"##, group: "GM1 · Volume and price", family: ("msws_gm1", "Morris GM1 volume and price systems"), page: "2", status: Live, f: [Some(r##"Cum(If(V>Ref(V,-1),ROC(C,1,%),0))<0"##), Some(r##"Cum(If(V>Ref(V,-1),ROC(C,1,%),0))>0"##), None, None], exec: EXEC_DEFAULT, notes: r##"Cum() accumulates from the first bar of the loaded data, as MetaStock does from the first loaded bar."## },
    Sys { id: "msws_price_volume_trend_w_ma", name: r##"Price Volume Trend w/ MA"##, group: "GM1 · Volume and price", family: ("msws_gm1", "Morris GM1 volume and price systems"), page: "2", status: Live, f: [Some(r##"Fml("Volume - Price Volume Trend") > Mov(Fml("Volume - Price Volume Trend"),7,S)"##), Some(r##"Fml("Volume - Price Volume Trend") < Mov(Fml("Volume - Price Volume Trend"),7,S)"##), Some(r##"Fml("Volume - Price Volume Trend") < Mov(Fml("Volume - Price Volume Trend"),7,S)"##), Some(r##"Fml("Volume - Price Volume Trend") > Mov(Fml("Volume - Price Volume Trend"),7,S)"##)], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "msws_pring_s_short_kst_on_volume_w_moving_avg", name: r##"Pring's Short KST on Volume - w Moving Avg"##, group: "GM1 · Volume and price", family: ("msws_gm1", "Morris GM1 volume and price systems"), page: "2", status: Live, f: [Some(r##"((Mov(ROC(V,10,%),10,S))+(2*(Mov(ROC(V,15,%),10,S)))+(3*(Mov(ROC(V,20,%),10,S)))+(4*(Mov(ROC(V,30,%),15,S)))/10) > (Mov((Mov(ROC(V,10,%),10,S))+(2*(Mov(ROC(V,15,%),10,S)))+(3*(Mov(ROC(V,20,%),10,S)))+(4*(Mov(ROC(V,30,%),15,S)))/10,7,S))"##), Some(r##"((Mov(ROC(V,10,%),10,S))+(2*(Mov(ROC(V,15,%),10,S)))+(3*(Mov(ROC(V,20,%),10,S)))+(4*(Mov(ROC(V,30,%),15,S)))/10) < (Mov((Mov(ROC(V,10,%),10,S))+(2*(Mov(ROC(V,15,%),10,S)))+(3*(Mov(ROC(V,20,%),10,S)))+(4*(Mov(ROC(V,30,%),15,S)))/10,7,S))"##), Some(r##"((Mov(ROC(V,10,%),10,S))+(2*(Mov(ROC(V,15,%),10,S)))+(3*(Mov(ROC(V,20,%),10,S)))+(4*(Mov(ROC(V,30,%),15,S)))/10) < (Mov((Mov(ROC(V,10,%),10,S))+(2*(Mov(ROC(V,15,%),10,S)))+(3*(Mov(ROC(V,20,%),10,S)))+(4*(Mov(ROC(V,30,%),15,S)))/10,7,S))"##), Some(r##"((Mov(ROC(V,10,%),10,S))+(2*(Mov(ROC(V,15,%),10,S)))+(3*(Mov(ROC(V,20,%),10,S)))+(4*(Mov(ROC(V,30,%),15,S)))/10) > (Mov((Mov(ROC(V,10,%),10,S))+(2*(Mov(ROC(V,15,%),10,S)))+(3*(Mov(ROC(V,20,%),10,S)))+(4*(Mov(ROC(V,30,%),15,S)))/10,7,S))"##)], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "msws_pctr_20_80_crossover", name: r##"%R (20/80) Crossover"##, group: "GM2 · Time-tested", family: ("msws_gm2", "Morris GM2 time-tested indicator systems"), page: "3", status: Live, f: [Some(r##"(Ref(WillR(14),-1)<-80) AND (WillR(14)>-80)"##), Some(r##"(Ref(WillR(14),-1)<-20) AND (WillR(14)>-20)"##), Some(r##"(Ref(WillR(14),-1)>-20) AND (WillR(14)<-20)"##), Some(r##"(Ref(WillR(14),-1)<-80) AND (WillR(14)>-80)"##)], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "msws_pctr_20_80_crossover_entry_exit_w_stops_chg", name: r##"%R (20/80) Crossover Entry/Exit w/ Stops-Chg"##, group: "GM2 · Time-tested", family: ("msws_gm2", "Morris GM2 time-tested indicator systems"), page: "3", status: Live, f: [Some(r##"(Ref(WillR(14),-1)<-80) AND (Ref(WillR(14),-1)<WillR(14))"##), Some(r##"((Ref(WillR(14),-1)>-80) AND (WillR(14)<-80)) OR ((Ref(WillR(14),-1))> WillR(14)>-20)"##), Some(r##"(Ref(WillR(14),-1)>-20) AND (Ref(WillR(14),-1)>WillR(14))"##), Some(r##"((Ref(WillR(14),-1)<-20) AND (WillR(14)>-20)) OR ((Ref(WillR(14),-1))< WillR(14)<-80)"##)], exec: EXEC_DEFAULT, notes: r##""(Ref(WillR(14),-1))> WillR(14)>-20" is a chained comparison: MetaStock evaluates (a>b)>-20, which is always TRUE; coded literally."## },
    Sys { id: "msws_pctr_20_80_crossover_entry_exit_w_stops_ma", name: r##"%R (20/80) Crossover Entry/Exit w/ Stops-MA"##, group: "GM2 · Time-tested", family: ("msws_gm2", "Morris GM2 time-tested indicator systems"), page: "3", status: Live, f: [Some(r##"WillR(14)<-80 AND (Mov(WillR(14),3,S)<WillR(14))"##), Some(r##"((Ref(WillR(14),-1)>-80) AND (WillR(14)<-80)) OR (WillR(14)>-20) AND (Mov(WillR(14),3,S)>WillR(14))"##), Some(r##"WillR(14)>-20 AND (Mov(WillR(14),3,S)>WillR(14))"##), Some(r##"((Ref(WillR(14),-1)<-20) AND (WillR(14)>-20)) OR (WillR(14)<-80) AND (Mov(WillR(14),3,S)<WillR(14))"##)], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "msws_pctr_20_80_crossover_w_stops", name: r##"%R (20/80) Crossover w/ stops"##, group: "GM2 · Time-tested", family: ("msws_gm2", "Morris GM2 time-tested indicator systems"), page: "4", status: Live, f: [Some(r##"(Ref(WillR(14),-1)<-80) AND (WillR(14)>-80)"##), Some(r##"(Ref(WillR(14),-1)>-20) AND (WillR(14)<-20) OR (WillR(14)<-80)"##), Some(r##"(Ref(WillR(14),-1)>-20) AND (WillR(14)<-20)"##), Some(r##"(Ref(WillR(14),-1)<-80) AND (WillR(14)>-80) OR (WillR(14)>-20)"##)], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "msws_cci_100_100_crossover", name: r##"CCI (-100/100) Crossover"##, group: "GM2 · Time-tested", family: ("msws_gm2", "Morris GM2 time-tested indicator systems"), page: "5", status: Live, f: [Some(r##"(Ref(CCI(11),-1)<-100) AND (CCI(11)>-100)"##), Some(r##"(Ref(CCI(11),-1)>100) AND (CCI(11)<100)"##), Some(r##"(Ref(CCI(11),-1)>100) AND (CCI(11)<100)"##), Some(r##"(Ref(CCI(11),-1)<-100) AND (CCI(11)>-100)"##)], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "msws_cci_100_100_crossover_entry_exit_w_stops_chg", name: r##"CCI (-100/100) Crossover Entry/Exit w/ Stops-Chg"##, group: "GM2 · Time-tested", family: ("msws_gm2", "Morris GM2 time-tested indicator systems"), page: "5", status: Live, f: [Some(r##"(Ref(CCI(11),-1)<-100) AND (Mov(CCI(11),3,S)<CCI(11))"##), Some(r##"((Ref(CCI(11),-1)>-100) AND (CCI(11)<-100)) OR ((CCI(11)>100) AND (Ref(CCI(11),-1))>CCI(11))"##), Some(r##"(Ref(CCI(11),-1)>100) AND (Ref(CCI(11),-1)<CCI(11))"##), Some(r##"((Ref(CCI(11),-1)<100) AND (CCI(11)>100)) OR ((CCI(11)<-100) AND (Ref(CCI(11),-1))<CCI(11))"##)], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "msws_cci_100_100_crossover_entry_exit_w_stops_ma", name: r##"CCI (-100/100) Crossover Entry/Exit w/ Stops-MA"##, group: "GM2 · Time-tested", family: ("msws_gm2", "Morris GM2 time-tested indicator systems"), page: "5", status: Live, f: [Some(r##"(Ref(CCI(11),-1)<-100) AND (Mov(CCI(11),3,S)<(CCI(11)))"##), Some(r##"((Ref(CCI(11),-1)>-100) AND (CCI(11)<-100)) OR ((CCI(11)>100) AND (Mov(CCI(11),3,S))>CCI(11))"##), Some(r##"(Ref(CCI(11),-1)>100) AND (Mov(CCI(11),3,S)>CCI(11))"##), Some(r##"((Ref(CCI(11),-1)<100) AND (CCI(11)>100)) OR ((CCI(11)<-100) AND (Mov(CCI(11),3,S))<CCI(11))"##)], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "msws_cci_100_100_crossover_w_stops", name: r##"CCI (-100/100) Crossover w/ stops"##, group: "GM2 · Time-tested", family: ("msws_gm2", "Morris GM2 time-tested indicator systems"), page: "6", status: Live, f: [Some(r##"(Ref(CCI(11),-1)<-100) AND (CCI(11)>-100)"##), Some(r##"(Ref(CCI(11),-1)<-100) AND (CCI(11)>-100)"##), Some(r##"(Ref(CCI(11),-1)>100) AND (CCI(11)<100)"##), Some(r##"(Ref(CCI(11),-1)<-100) AND (CCI(11)>-100) OR (CCI(11)>100)"##)], exec: EXEC_DEFAULT, notes: r##"Close Long is printed identical to Enter Long (probable typo); coded literally: the long exits on the next CCI re-cross above -100."## },
    Sys { id: "msws_chaikin_entry_exit_w_stops", name: r##"Chaikin Entry/Exit w/ Stops"##, group: "GM2 · Time-tested", family: ("msws_gm2", "Morris GM2 time-tested indicator systems"), page: "6", status: Live, f: [Some(r##"(Ref(CO(),-1)<-1000) AND (CO()>Mov(CO(),3,S))"##), Some(r##"(Ref(CO(),-1)>-1000) AND (CO()<-1000 OR (Ref(CO(),-1)>1000) AND (CO()<Mov(CO(),3,S)))"##), Some(r##"(Ref(CO(),-1)>1000) AND (CO()<Mov(CO(),3,S))"##), Some(r##"((Ref(CO(),-1)<1000) AND (CO()>1000)) OR ((CO()<-1000) AND (CO()>Mov(CO(),3,S)))"##)], exec: EXEC_DEFAULT, notes: r##"Thresholds -1000/1000 are in Chaikin-oscillator (volume) units; applied literally to the index volume."## },
    Sys { id: "msws_generic_oscillator_w_threshold_gm2", name: r##"Generic Oscillator w/ threshold"##, group: "GM2 · Time-tested", family: ("msws_gm2", "Morris GM2 time-tested indicator systems"), page: "7", status: Live, f: [Some(r##"Fml("POP - Stochastics %D'")>20 AND Ref(Fml("POP - Stochastics %D'"),-1)<20"##), Some(r##"Fml("POP - Stochastics %D'")<80 AND Ref(Fml("POP - Stochastics %D'"),-1)>80"##), Some(r##"Fml("POP - Stochastics %D'")<80 AND Ref(Fml("POP - Stochastics %D'"),-1)>80"##), Some(r##"Fml("POP - Stochastics %D'")>20 AND Ref(Fml("POP - Stochastics %D'"),-1)<20"##)], exec: EXEC_DEFAULT, notes: r##"Printed as a template; coded with the indicator printed (POP - Stochastics %D', the GM2 %D' formula)."## },
    Sys { id: "msws_generic_oscillator_w_threshold_and_ma_gm2", name: r##"Generic Oscillator w/ threshold and MA"##, group: "GM2 · Time-tested", family: ("msws_gm2", "Morris GM2 time-tested indicator systems"), page: "7", status: Live, f: [Some(r##"(Fml("POP - Stochastics %D'") > Mov(Fml("POP - Stochastics %D'"),5,S)) AND (Fml("POP - Stochastics %D'") < 20)"##), Some(r##"Fml("POP - Stochastics %D'") < Mov(Fml("POP - Stochastics %D'"),5,S) AND (Fml("POP - Stochastics %D'") > 80)"##), Some(r##"Fml("POP - Stochastics %D'") < Mov(Fml("POP - Stochastics %D'"),5,S) AND (Fml("POP - Stochastics %D'") > 80)"##), Some(r##"Fml("POP - Stochastics %D'") > Mov(Fml("POP - Stochastics %D'"),5,S) AND (Fml("POP - Stochastics %D'") < 20)"##)], exec: EXEC_DEFAULT, notes: r##"Printed as a template; coded with the indicator printed (POP - Stochastics %D')."## },
    Sys { id: "msws_generic_oscillator_w_ma_gm2", name: r##"Generic Oscillator w/ MA"##, group: "GM2 · Time-tested", family: ("msws_gm2", "Morris GM2 time-tested indicator systems"), page: "7", status: Live, f: [Some(r##"Fml("POP - Stochastics %D'") > Mov(Fml("POP - Stochastics %D'"),7,S)"##), Some(r##"Fml("POP - Stochastics %D'") < Mov(Fml("POP - Stochastics %D'"),7,S)"##), Some(r##"Fml("POP - Stochastics %D'") < Mov(Fml("POP - Stochastics %D'"),7,S)"##), Some(r##"Fml("POP - Stochastics %D'") > Mov(Fml("POP - Stochastics %D'"),7,S)"##)], exec: EXEC_DEFAULT, notes: r##"Printed as a template; coded with the indicator printed (POP - Stochastics %D')."## },
    Sys { id: "msws_parabolic_sar_crossover", name: r##"Parabolic SAR Crossover"##, group: "GM2 · Time-tested", family: ("msws_gm2", "Morris GM2 time-tested indicator systems"), page: "8", status: Live, f: [Some(r##"(SAR(.02,.2)<L)"##), Some(r##"(SAR(.02,.2)>H)"##), Some(r##"(SAR(.02,.2)>H)"##), Some(r##"(SAR(.02,.2)<L)"##)], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "msws_roc_crossover", name: r##"ROC Crossover"##, group: "GM2 · Time-tested", family: ("msws_gm2", "Morris GM2 time-tested indicator systems"), page: "8", status: Live, f: [Some(r##"(Ref(ROC(C,12,%),-1)<-5) AND (ROC(C,12,%)>-5)"##), Some(r##"(Ref(ROC(C,12,%),-1)>5) AND (ROC(C,12,%)<5)"##), Some(r##"(Ref(ROC(C,12,%),-1)>5) AND (ROC(C,12,%)<5)"##), Some(r##"(Ref(ROC(C,12,%),-1)<-5) AND (ROC(C,12,%)>-5)"##)], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "msws_roc_crossover_w_stops", name: r##"ROC Crossover w/ Stops"##, group: "GM2 · Time-tested", family: ("msws_gm2", "Morris GM2 time-tested indicator systems"), page: "8", status: Live, f: [Some(r##"(Ref(ROC(C,12,%),-1)<-5) AND (ROC(C,12,%)>-5)"##), Some(r##"((ROC(C,12,%)<-5)) OR (Ref(ROC(C,12,%),-1)>5) AND (ROC(C,12,%)<5)"##), Some(r##"(Ref(ROC(C,12,%),-1)>5) AND (ROC(C,12,%)<5)"##), Some(r##"((ROC(C,12,%)>5)) OR (Ref(ROC(C,12,%),-1)<-5) AND (ROC(C,12,%)>-5)"##)], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "msws_rsi_30_70_crossover", name: r##"RSI (30/70) Crossover"##, group: "GM2 · Time-tested", family: ("msws_gm2", "Morris GM2 time-tested indicator systems"), page: "9", status: Live, f: [Some(r##"(Ref(RSI(14),-1)<30) AND (RSI(14)>30)"##), Some(r##"(Ref(RSI(14),-1)>70) AND (RSI(14)<70)"##), Some(r##"(Ref(RSI(14),-1)>70) AND (RSI(14)<70)"##), Some(r##"(Ref(RSI(14),-1)<30) AND (RSI(14)>30)"##)], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "msws_rsi_30_70_crossover_entry_exit_w_stops_chg", name: r##"RSI (30/70) Crossover Entry/Exit w/ Stops-Chg"##, group: "GM2 · Time-tested", family: ("msws_gm2", "Morris GM2 time-tested indicator systems"), page: "9", status: Live, f: [Some(r##"(Ref(RSI(14),-1)<30) AND (Ref(RSI(14),-1)<RSI(14))"##), Some(r##"((Ref(RSI(14),-1)>30) AND (RSI(14)<30)) OR (Ref(RSI(14),-1)>RSI(14)>70)"##), Some(r##"(Ref(RSI(14),-1)>70) AND (Ref(RSI(14),-1)>RSI(14))"##), Some(r##"((Ref(RSI(14),-1)<70) AND (RSI(14)>70)) OR (Ref(RSI(14),-1)<RSI(14)<30)"##)], exec: EXEC_DEFAULT, notes: r##""Ref(RSI(14),-1)>RSI(14)>70" is a chained comparison: (a>b) is 0 or 1, so ">70" is never TRUE and that exit branch never fires; coded literally."## },
    Sys { id: "msws_rsi_30_70_crossover_entry_exit_w_stops_ma", name: r##"RSI (30/70) Crossover Entry/Exit w/ Stops-MA"##, group: "GM2 · Time-tested", family: ("msws_gm2", "Morris GM2 time-tested indicator systems"), page: "9", status: Live, f: [Some(r##"(Ref(RSI(14),-1)<30) AND (Mov(RSI(14),3,S)<RSI(14))"##), Some(r##"(Ref(RSI(14),-1)>30) AND ((RSI(14)<30)) OR ((RSI(14)>70) AND (Mov(RSI(14),3,S)>RSI(14)))"##), Some(r##"(Ref(RSI(14),-1)>70) AND (Mov(RSI(14),3,S)>RSI(14))"##), Some(r##"(Ref(RSI(14),-1)<70) AND (RSI(14)>70) OR ((RSI(14)<30) AND (Mov(RSI(14),3,S)< RSI(14)))"##)], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "msws_rsi_30_70_crossover_w_stops", name: r##"RSI (30/70) Crossover w/ Stops"##, group: "GM2 · Time-tested", family: ("msws_gm2", "Morris GM2 time-tested indicator systems"), page: "10", status: Live, f: [Some(r##"(Ref(RSI(14),-1)<30) AND (RSI(14)>30)"##), Some(r##"(Ref(RSI(14),-1) >30 AND (RSI(14)<30)) OR (RSI(14)>70)"##), Some(r##"(Ref(RSI(14),-1)>70) AND (RSI(14)<70)"##), Some(r##"((Ref(RSI(14),-1) < 70) AND (RSI(14)>70)) OR (RSI(14)<30)"##)], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "msws_stochastics_pctd_20_80_w_stops", name: r##"Stochastics %D (20/80) w/ Stops"##, group: "GM2 · Time-tested", family: ("msws_gm2", "Morris GM2 time-tested indicator systems"), page: "10", status: Live, f: [Some(r##"(Ref(Mov((((CLOSE - LLV(L,14))/ (HHV(H,14)-LLV(L,14)))*100),3,S),-1)<20) AND (Mov((((CLOSE - LLV(L,14))/ (HHV(H,14)-LLV(L,14)))*100),3,S)>20)"##), Some(r##"((Mov((((CLOSE - LLV(L,14))/ (HHV(H,14)-LLV(L,14)))*100),3,S)) >80) OR ((Mov((((CLOSE - LLV(L,14))/ (HHV(H,14)-LLV(L,14)))*100),3,S)) < 20)"##), Some(r##"(Ref(Mov((((CLOSE - LLV(L,14))/ (HHV(H,14)-LLV(L,14)))*100),3,S),-1)>80) AND (Mov((((CLOSE - LLV(L,14))/ (HHV(H,14)-LLV(L,14)))*100),3,S)<80)"##), Some(r##"((Mov((((CLOSE - LLV(L,14))/ (HHV(H,14)-LLV(L,14)))*100),3,S)) <20) OR ((Mov((((CLOSE - LLV(L,14))/ (HHV(H,14)-LLV(L,14)))*100),3,S)) >80)"##)], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "msws_stochastics_pctk_20_80_crossover", name: r##"Stochastics %K (20/80) Crossover"##, group: "GM2 · Time-tested", family: ("msws_gm2", "Morris GM2 time-tested indicator systems"), page: "11", status: Live, f: [Some(r##"(Ref(Stoch(14,3),-1)<20) AND (Stoch(14,3)>20)"##), Some(r##"(Ref(Stoch(14,3),-1)>80) AND (Stoch(14,3)<80)"##), Some(r##"(Ref(Stoch(14,3),-1)>80) AND (Stoch(14,3)<80)"##), Some(r##"(Ref(Stoch(14,3),-1)<20) AND (Stoch(14,3)>20)"##)], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "msws_stochastics_pctk_20_80_crossover_w_stops", name: r##"Stochastics %K (20/80) Crossover w/ Stops"##, group: "GM2 · Time-tested", family: ("msws_gm2", "Morris GM2 time-tested indicator systems"), page: "11", status: Live, f: [Some(r##"(Ref(Stoch(14,3),-1)<20) AND (Stoch(14,3)>20)"##), Some(r##"(Ref(Stoch(14,3),-1)>80) AND (Stoch(14,3)<80) OR (Stoch(14,3)<20)"##), Some(r##"(Ref(Stoch(14,3),-1)>80) AND (Stoch(14,3)<80)"##), Some(r##"(Ref(Stoch(14,3),-1)>20) AND (Stoch(14,3)<20) OR (Stoch(14,3)>80)"##)], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "msws_stochastics_pctk_20_80_entry_exit_w_stops_ma", name: r##"Stochastics %K (20/80) Entry/Exit w/ Stops-MA"##, group: "GM2 · Time-tested", family: ("msws_gm2", "Morris GM2 time-tested indicator systems"), page: "11", status: Live, f: [Some(r##"(Ref(Stoch(14,3),-1)<20) AND (Stoch(14,3)>Mov(Stoch(14,3),3,S))"##), Some(r##"((Stoch(14,3)<20) AND (Ref(Stoch(14,3),-1)>20)) OR ((Ref(Stoch(14,3),-1)>80) AND (Stoch(14,3)< Mov(Stoch(14,3),3,S)))"##), Some(r##"(Ref(Stoch(14,3),-1)>80) AND (Stoch(14,3)<Mov(Stoch(14,3),3,S))"##), Some(r##"((Stoch(14,3)>80) AND (Ref(Stoch(14,3),-1)<80)) OR ((Ref(Stoch(14,3),-1)<20) AND (Stoch(14,3) > Mov(Stoch(14,3),3,S)))"##)], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "msws_trix_entry_exit_chg", name: r##"Trix Entry/Exit - Chg"##, group: "GM2 · Time-tested", family: ("msws_gm2", "Morris GM2 time-tested indicator systems"), page: "12", status: Live, f: [Some(r##"(TRIX(14)>Ref(TRIX(14),-1)) AND (TRIX(14)<0)"##), Some(r##"(TRIX(14)<Ref(TRIX(14),-1)) AND TRIX(14)>0"##), Some(r##"(TRIX(14)<Ref(TRIX(14),-1)) AND TRIX(14)>0"##), Some(r##"(TRIX(14)>Ref(TRIX(14),-1)) AND (TRIX(14)<0)"##)], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "msws_trix_entry_exit_ma", name: r##"Trix Entry/Exit - MA"##, group: "GM2 · Time-tested", family: ("msws_gm2", "Morris GM2 time-tested indicator systems"), page: "12", status: Live, f: [Some(r##"(TRIX(14)-Ref(TRIX(14),-1)<0) AND (TRIX(14)-Ref(TRIX(14),-1)>Mov(TRIX(14)- Ref(TRIX(14),-1),3,E))"##), Some(r##"(TRIX(14)-Ref(TRIX(14),-1)>0) AND (TRIX(14)-Ref(TRIX(14),-1)<Mov(TRIX(14)-Ref(TRIX(14),-1),3,E))"##), Some(r##"(TRIX(14)-Ref(TRIX(14),-1)>0) AND (TRIX(14)-Ref(TRIX(14),-1)<Mov(TRIX(14)-Ref(TRIX(14),-1),3,E))"##), Some(r##"(TRIX(14)-Ref(TRIX(14),-1)<0) AND (TRIX(14)-Ref(TRIX(14),-1)>Mov(TRIX(14)-Ref(TRIX(14),-1),3,E))"##)], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "msws_trix_entry_exit_w_stops_chg", name: r##"Trix Entry/Exit w/ stops - Chg"##, group: "GM2 · Time-tested", family: ("msws_gm2", "Morris GM2 time-tested indicator systems"), page: "12", status: Live, f: [Some(r##"(TRIX(14)>Ref(TRIX(14),-1)) AND (TRIX(14)<0)"##), Some(r##"(TRIX(14)<Ref(TRIX(14),-1)) AND TRIX(14)>0 OR (TRIX(14)<Ref(TRIX(14),-5))"##), Some(r##"(TRIX(14)<Ref(TRIX(14),-1)) AND TRIX(14)>0"##), Some(r##"(TRIX(14)>Ref(TRIX(14),-1)) AND (TRIX(14)<0) OR (TRIX(14)>Ref(TRIX(14),-5))"##)], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "msws_3_crows_soldiers_pctd", name: r##"3 Crows & Soldiers & %D"##, group: "GM3 · Candlestick", family: ("msws_gm3", "Morris GM3 candlestick + oscillator filter systems"), page: "13", status: Live, f: [Some(r##"(Fml("Candle - 3 White Soldiers")=1) AND (Fml("Candle - Stochastics %D")<30)"##), Some(r##"((Fml("Candle - Stochastics %D")<80) AND (Ref(Fml("Candle - Stochastics %D"),-1)>80)) OR ((Fml("Candle - Stochastics %D")<20) AND Ref((Fml("Candle - Stochastics %D")>20),-1))"##), Some(r##"(Fml("Candle - 3 Black Crows")=-1) AND (Fml("Candle - Stochastics %D")>70)"##), Some(r##"((Fml("Candle - Stochastics %D")>20) AND (Ref(Fml("Candle - Stochastics %D"),-1)<20)) OR ((Fml("Candle - Stochastics %D")>80) AND Ref((Fml("Candle - Stochastics %D")<80),-1))"##)], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "msws_3_crows_soldiers_cci", name: r##"3 Crows & Soldiers & CCI"##, group: "GM3 · Candlestick", family: ("msws_gm3", "Morris GM3 candlestick + oscillator filter systems"), page: "13", status: NotTestable("Close Long prints CCI(1) (a 1-bar CCI, mean deviation 0, so 0/0 on every bar); MetaStock's result for that is not documented and correcting it to CCI(11) would be inventing"), f: [Some(r##"(Fml("Candle - 3 White Soldiers")=1) AND (CCI(11)<-50)"##), Some(r##"((CCI(1)<80) AND (Ref(CCI(11),-1)>80)) OR ((CCI(11)<-80) AND Ref(CCI(11)>-80,-1))"##), Some(r##"(Fml("Candle - 3 Black Crows")=-1) AND (CCI(11)>50)"##), Some(r##"((CCI(11)>-80) AND (Ref(CCI(11),-1)<-80)) OR ((CCI(11)<80) AND Ref(CCI(11)>80,-1))"##)], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "msws_3_crows_soldiers_mfi", name: r##"3 Crows & Soldiers & MFI"##, group: "GM3 · Candlestick", family: ("msws_gm3", "Morris GM3 candlestick + oscillator filter systems"), page: "14", status: Live, f: [Some(r##"(Fml("Candle - 3 White Soldiers")=1) AND (Fml("Candle - Money Flow Index")<40)"##), Some(r##"((Fml("Candle - Money Flow Index")>70) AND (Ref(Fml("Candle - Money Flow Index"),-1)>70)) OR ((Fml("Candle - Money Flow Index")<30) AND (Ref(Fml("Candle - Money Flow Index")>30,-1)))"##), Some(r##"(Fml("Candle - 3 Black Crows")=-1) AND (Fml("Candle - Money Flow Index")>60)"##), Some(r##"((Fml("Candle - Money Flow Index")>30) AND (Ref(Fml("Candle - Money Flow Index"),-1)<30)) OR ((Fml("Candle - Money Flow Index")>70) AND (Ref(Fml("Candle - Money Flow Index")<70,-1)))"##)], exec: EXEC_DEFAULT, notes: r##"Close Long first clause prints MFI>70 AND Ref(MFI,-1)>70 (the sister systems use <70); coded literally. Fml("Candle - Money Flow Index") is coded exactly as printed: (C-Ref(C,-1)*V) multiplies only the previous close by volume (MetaStock precedence)."## },
    Sys { id: "msws_3_crows_soldiers_rsi", name: r##"3 Crows & Soldiers & RSI"##, group: "GM3 · Candlestick", family: ("msws_gm3", "Morris GM3 candlestick + oscillator filter systems"), page: "14", status: Live, f: [Some(r##"(Fml("Candle - 3 White Soldiers")=1) AND (RSI(14)<40)"##), Some(r##"((RSI(14)<70) AND (Ref(RSI(14),-1)>70)) OR ((RSI(14)<30) AND (Ref(RSI(14)>30,-1)))"##), Some(r##"(Fml("Candle - 3 Black Crows")=-1) AND (RSI(14)>60)"##), Some(r##"((RSI(14)>30) AND (Ref(RSI(14),-1)<30)) OR ((RSI(14)>70) AND (Ref(RSI(14)<70,-1)))"##)], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "msws_all_pctd", name: r##"ALL & %D"##, group: "GM3 · Candlestick", family: ("msws_gm3", "Morris GM3 candlestick + oscillator filter systems"), page: "14", status: Live, f: [Some(r##"(Fml("Candle - All Bullish")>0) AND (Fml("Candle - Stochastics %D")<30)"##), Some(r##"((Fml("Candle - Stochastics %D")<80) AND (Ref(Fml("Candle - Stochastics %D"),-1)>80)) OR ((Fml("Candle - Stochastics %D")<20) AND Ref((Fml("Candle - Stochastics %D")>20),-1))"##), Some(r##"(Fml("Candle - All Bearish")<0) AND (Fml("Candle - Stochastics %D")>70)"##), Some(r##"((Fml("Candle - Stochastics %D")>20) AND (Ref(Fml("Candle - Stochastics %D"),-1)<20)) OR ((Fml("Candle - Stochastics %D")>80) AND Ref((Fml("Candle - Stochastics %D")<80),-1))"##)], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "msws_all_cci", name: r##"ALL & CCI"##, group: "GM3 · Candlestick", family: ("msws_gm3", "Morris GM3 candlestick + oscillator filter systems"), page: "15", status: Live, f: [Some(r##"(Fml("Candle - All Bullish")>0) AND (CCI(11)<-50)"##), Some(r##"((CCI(11)<80) AND (Ref(CCI(11),-1)>80)) OR ((CCI(11)<-80) AND Ref(CCI(11)>-80,-1))"##), Some(r##"(Fml("Candle - All Bearish")<0) AND (CCI(11)>50)"##), Some(r##"((CCI(11)>-80) AND (Ref(CCI(11),-1)<-80)) OR ((CCI(11)<80) AND Ref(CCI(11)>80,-1))"##)], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "msws_all_mfi", name: r##"ALL & MFI"##, group: "GM3 · Candlestick", family: ("msws_gm3", "Morris GM3 candlestick + oscillator filter systems"), page: "15", status: Live, f: [Some(r##"(Fml("Candle - All Bullish")>0) AND (Fml("Candle - Money Flow Index")<40)"##), Some(r##"((Fml("Candle - Money Flow Index")<70) AND (Ref(Fml("Candle - Money Flow Index"),-1)>70)) OR ((Fml("Candle - Money Flow Index")<30) AND (Ref(Fml("Candle - Money Flow Index")>30,-1)))"##), Some(r##"(Fml("Candle - All Bearish")<0) AND (Fml("Candle - Money Flow Index")>60)"##), Some(r##"((Fml("Candle - Money Flow Index")>30) AND (Ref(Fml("Candle - Money Flow Index"),-1)<30)) OR ((Fml("Candle - Money Flow Index")>70) AND (Ref(Fml("Candle - Money Flow Index")<70,-1)))"##)], exec: EXEC_DEFAULT, notes: r##"The Close Long label is missing in the source; the unlabeled formula printed in the Close Long position is used. Fml("Candle - Money Flow Index") is coded exactly as printed: (C-Ref(C,-1)*V) multiplies only the previous close by volume (MetaStock precedence)."## },
    Sys { id: "msws_all_rsi", name: r##"ALL & RSI"##, group: "GM3 · Candlestick", family: ("msws_gm3", "Morris GM3 candlestick + oscillator filter systems"), page: "16", status: Live, f: [Some(r##"(Fml("Candle - All Bullish")>0) AND (RSI(14)<40)"##), Some(r##"((RSI(14)<70) AND (Ref(RSI(14),-1)>70)) OR ((RSI(14)<30) AND (Ref(RSI(14)>30,-1)))"##), Some(r##"(Fml("Candle - All Bearish")<0) AND (RSI(14)>60)"##), Some(r##"((RSI(14)>30) AND (Ref(RSI(14),-1)<30)) OR ((RSI(14)>70) AND (Ref(RSI(14)<70,-1)))"##)], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "msws_dark_cloud_piercing_line_pctd", name: r##"Dark Cloud/Piercing Line & %D"##, group: "GM3 · Candlestick", family: ("msws_gm3", "Morris GM3 candlestick + oscillator filter systems"), page: "16", status: Live, f: [Some(r##"(Fml("Candle - Piercing Line")=1) AND (Fml("Candle - Stochastics %D")<30)"##), Some(r##"((Fml("Candle - Stochastics %D")<80) AND (Ref(Fml("Candle - Stochastics %D"),-1)>80)) OR ((Fml("Candle - Stochastics %D")<20) AND Ref((Fml("Candle - Stochastics %D")>20),-1))"##), Some(r##"(Fml("Candle - Dark Cloud Cover")=-1) AND (Fml("Candle - Stochastics %D")>70)"##), Some(r##"((Fml("Candle - Stochastics %D")>20) AND (Ref(Fml("Candle - Stochastics %D"),-1)<20)) OR ((Fml("Candle - Stochastics %D")>80) AND Ref((Fml("Candle - Stochastics %D")<80),-1))"##)], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "msws_dark_cloud_piercing_line_cci", name: r##"Dark Cloud/Piercing Line & CCI"##, group: "GM3 · Candlestick", family: ("msws_gm3", "Morris GM3 candlestick + oscillator filter systems"), page: "17", status: Live, f: [Some(r##"(Fml("Candle - Piercing Line")=1) AND (CCI(11)<-50)"##), Some(r##"((CCI(11)<80) AND (Ref(CCI(11),-1)>80)) OR ((CCI(11)<-80) AND Ref(CCI(11)>-80,-1))"##), Some(r##"(Fml("Candle - Dark Cloud Cover")=-1) AND (CCI(11)>50)"##), Some(r##"((CCI(11)-80) AND (Ref(CCI(11),-1)<-80)) OR ((CCI(11)<80) AND Ref(CCI(11)>80,-1))"##)], exec: EXEC_DEFAULT, notes: r##"Close Short prints "(CCI(11)-80) AND ..." with no comparison operator; MetaStock treats any non-zero value as TRUE, which is how it is coded."## },
    Sys { id: "msws_dark_cloud_piercing_line_mfi", name: r##"Dark Cloud/Piercing Line & MFI"##, group: "GM3 · Candlestick", family: ("msws_gm3", "Morris GM3 candlestick + oscillator filter systems"), page: "17", status: Live, f: [Some(r##"(Fml("Candle - Piercing Line")=1) AND (Fml("Candle - Money Flow Index")<40)"##), Some(r##"((Fml("Candle - Money Flow Index")<70) AND (Ref(Fml("Candle - Money Flow Index"),-1)>70)) OR ((Fml("Candle - Money Flow Index")<30) AND (Ref(Fml("Candle - Money Flow Index")>30,-1)))"##), Some(r##"(Fml("Candle - Dark Cloud Cover")=-1) AND (Fml("Candle - Money Flow Index")>60)"##), Some(r##"((Fml("Candle - Money Flow Index")>30) AND (Ref(Fml("Candle - Money Flow Index"),-1)<30)) OR ((Fml("Candle - Money Flow Index")>70) AND (Ref(Fml("Candle - Money Flow Index")<70,-1)))"##)], exec: EXEC_DEFAULT, notes: r##"Fml("Candle - Money Flow Index") is coded exactly as printed: (C-Ref(C,-1)*V) multiplies only the previous close by volume (MetaStock precedence)."## },
    Sys { id: "msws_dark_cloud_piercing_line_rsi", name: r##"Dark Cloud/Piercing Line & RSI"##, group: "GM3 · Candlestick", family: ("msws_gm3", "Morris GM3 candlestick + oscillator filter systems"), page: "17", status: Live, f: [Some(r##"(Fml("Candle - Piercing Line")=1) AND (RSI(14)<40)"##), Some(r##"((RSI(14)<70) AND (Ref(RSI(14),-1)>70)) OR ((RSI(14)<30) AND (Ref(RSI(14)>30,-1)))"##), Some(r##"(Fml("Candle - Dark Cloud Cover")=-1) AND (RSI(14)>60)"##), Some(r##"((RSI(14)>30) AND (Ref(RSI(14),-1)<30)) OR ((RSI(14)>70) AND (Ref(RSI(14)<70,-1)))"##)], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "msws_doji_stars_pctd", name: r##"Doji Stars & %D"##, group: "GM3 · Candlestick", family: ("msws_gm3", "Morris GM3 candlestick + oscillator filter systems"), page: "18", status: Live, f: [Some(r##"(Fml("Candle - Morning Doji")=1) AND (Fml("Candle - Stochastics %D")<30)"##), Some(r##"((Fml("Candle - Stochastics %D")<80) AND (Ref(Fml("Candle - Stochastics %D"),-1)>80)) OR ((Fml("Candle - Stochastics %D")<20) AND Ref((Fml("Candle - Stochastics %D")>20),-1))"##), Some(r##"(Fml("Candle - Evening Doji")=-1) AND (Fml("Candle - Stochastics %D")>70)"##), Some(r##"((Fml("Candle - Stochastics %D")>20) AND (Ref(Fml("Candle - Stochastics %D"),-1)<20)) OR ((Fml("Candle - Stochastics %D")>80) AND Ref((Fml("Candle - Stochastics %D")<80),-1))"##)], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "msws_doji_stars_cci", name: r##"Doji Stars & CCI"##, group: "GM3 · Candlestick", family: ("msws_gm3", "Morris GM3 candlestick + oscillator filter systems"), page: "18", status: Live, f: [Some(r##"(Fml("Candle - Morning Doji")=1) AND (CCI(11)<-50)"##), Some(r##"((CCI(11)<80) AND (Ref(CCI(11),-1)>80)) OR ((CCI(11)<-80) AND Ref(CCI(11)>-80,-1))"##), Some(r##"(Fml("Candle - Evening Doji")=-1) AND (CCI(11)>50)"##), Some(r##"((CCI(11)>-80) AND (Ref(CCI(11),-1)<-80)) OR ((CCI(11)<80) AND Ref(CCI(11)>80,-1))"##)], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "msws_doji_stars_mfi", name: r##"Doji Stars & MFI"##, group: "GM3 · Candlestick", family: ("msws_gm3", "Morris GM3 candlestick + oscillator filter systems"), page: "19", status: Live, f: [Some(r##"(Fml("Candle - Morning Doji")=1) AND (Fml("Candle - Money Flow Index")<40)"##), Some(r##"((Fml("Candle - Money Flow Index")<70) AND (Ref(Fml("Candle - Money Flow Index"),-1)>70)) OR ((Fml("Candle - Money Flow Index")<30) AND (Ref(Fml("Candle - Money Flow Index")>30,-1)))"##), Some(r##"(Fml("Candle - Evening Doji")=-1) AND (Fml("Candle - Money Flow Index")>60)"##), Some(r##"((Fml("Candle - Money Flow Index")>30) AND (Ref(Fml("Candle - Money Flow Index"),-1)<30)) OR ((Fml("Candle - Money Flow Index")>70) AND (Ref(Fml("Candle - Money Flow Index")<70,-1)))"##)], exec: EXEC_DEFAULT, notes: r##"Fml("Candle - Money Flow Index") is coded exactly as printed: (C-Ref(C,-1)*V) multiplies only the previous close by volume (MetaStock precedence)."## },
    Sys { id: "msws_doji_stars_rsi", name: r##"Doji Stars & RSI"##, group: "GM3 · Candlestick", family: ("msws_gm3", "Morris GM3 candlestick + oscillator filter systems"), page: "19", status: Live, f: [Some(r##"(Fml("Candle - Morning Doji")=1) AND (RSI(14)<40)"##), Some(r##"((RSI(14)<70) AND (Ref(RSI(14),-1)>70)) OR ((RSI(14)<30) AND (Ref(RSI(14)>30,-1)))"##), Some(r##"(Fml("Candle - Evening Doji")=-1) AND (RSI(14)>60)"##), Some(r##"((RSI(14)>30) AND (Ref(RSI(14),-1)<30)) OR ((RSI(14)>70) AND (Ref(RSI(14)<70,-1)))"##)], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "msws_engulfing_pctd", name: r##"Engulfing & %D"##, group: "GM3 · Candlestick", family: ("msws_gm3", "Morris GM3 candlestick + oscillator filter systems"), page: "19", status: Live, f: [Some(r##"(Fml("Candle - Engulfing - Bullish")=1) AND (Fml("Candle - Stochastics %D")<30)"##), Some(r##"((Fml("Candle - Stochastics %D")<80) AND (Ref(Fml("Candle - Stochastics %D"),-1)>80)) OR ((Fml("Candle - Stochastics %D")<20) AND Ref((Fml("Candle - Stochastics %D")>20),-1))"##), Some(r##"(Fml("Candle - Engulfing - Bearish")=-1) AND (Fml("Candle - Stochastics %D")>70)"##), Some(r##"((Fml("Candle - Stochastics %D")>20) AND (Ref(Fml("Candle - Stochastics %D"),-1)<20)) OR ((Fml("Candle - Stochastics %D")>80) AND Ref((Fml("Candle - Stochastics %D")<80),-1))"##)], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "msws_engulfing_cci", name: r##"Engulfing & CCI"##, group: "GM3 · Candlestick", family: ("msws_gm3", "Morris GM3 candlestick + oscillator filter systems"), page: "20", status: Live, f: [Some(r##"(Fml("Candle - Engulfing - Bullish")=1) AND (CCI(11)<-50)"##), Some(r##"((CCI(11)<80) AND (Ref(CCI(11),-1)>80)) OR ((CCI(11)<-80) AND Ref(CCI(11)>-80,-1))"##), Some(r##"(Fml("Candle - Engulfing - Bearish")=-1) AND (CCI(11)>50)"##), Some(r##"(CCI(11)>-80) AND (Ref(CCI(11),-1)<-80) OR ((CCI(11)<80) AND Ref(CCI(11)>80,-1))"##)], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "msws_engulfing_mfi", name: r##"Engulfing & MFI"##, group: "GM3 · Candlestick", family: ("msws_gm3", "Morris GM3 candlestick + oscillator filter systems"), page: "20", status: Live, f: [Some(r##"(Fml("Candle - Engulfing - Bullish")=1) AND (Fml("Candle - Money Flow Index")<40)"##), Some(r##"((Fml("Candle - Money Flow Index")<70) AND (Ref(Fml("Candle - Money Flow Index"),-1)>70)) OR ((Fml("Candle - Money Flow Index")<30) AND (Ref(Fml("Candle - Money Flow Index")>30,-1)))"##), Some(r##"(Fml("Candle - Engulfing - Bearish")=-1) AND (Fml("Candle - Money Flow Index")>60)"##), Some(r##"((Fml("Candle - Money Flow Index")>30) AND (Ref(Fml("Candle - Money Flow Index"),-1)<30)) OR ((Fml("Candle - Money Flow Index")>70) AND (Ref(Fml("Candle - Money Flow Index")<70,-1)))"##)], exec: EXEC_DEFAULT, notes: r##"Fml("Candle - Money Flow Index") is coded exactly as printed: (C-Ref(C,-1)*V) multiplies only the previous close by volume (MetaStock precedence)."## },
    Sys { id: "msws_engulfing_rsi", name: r##"Engulfing & RSI"##, group: "GM3 · Candlestick", family: ("msws_gm3", "Morris GM3 candlestick + oscillator filter systems"), page: "21", status: Live, f: [Some(r##"(Fml("Candle - Engulfing - Bullish")=1) AND (RSI(14)<40)"##), Some(r##"((RSI(14)<70) AND (Ref(RSI(14),-1)>70)) OR ((RSI(14)<30) AND (Ref(RSI(14)>30,-1)))"##), Some(r##"(Fml("Candle - Engulfing - Bearish")=-1) AND (RSI(14)>60)"##), Some(r##"((RSI(14)>30) AND (Ref(RSI(14),-1)<30)) OR ((RSI(14)>70) AND (Ref(RSI(14)<70,-1)))"##)], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "msws_hanging_man_hammer_pctd", name: r##"Hanging Man/Hammer & %D"##, group: "GM3 · Candlestick", family: ("msws_gm3", "Morris GM3 candlestick + oscillator filter systems"), page: "21", status: Live, f: [Some(r##"(Fml("Candle - Hammer")=1) AND (Fml("Candle - Stochastics %D")<30)"##), Some(r##"((Fml("Candle - Stochastics %D")<80) AND (Ref(Fml("Candle - Stochastics %D"),-1)>80)) OR ((Fml("Candle - Stochastics %D")<20) AND Ref((Fml("Candle - Stochastics %D")>20),-1))"##), Some(r##"(Fml("Candle - Hanging Man'")=-1) AND (Fml("Candle - Stochastics %D")>70)"##), Some(r##"((Fml("Candle - Stochastics %D")>20) AND (Ref(Fml("Candle - Stochastics %D"),-1)<20)) OR ((Fml("Candle - Stochastics %D")>80) AND Ref((Fml("Candle - Stochastics %D")<80),-1))"##)], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "msws_hanging_man_hammer_cci", name: r##"Hanging Man/Hammer & CCI"##, group: "GM3 · Candlestick", family: ("msws_gm3", "Morris GM3 candlestick + oscillator filter systems"), page: "21", status: Live, f: [Some(r##"(Fml("Candle - Hammer")=1) AND (CCI(11)<-50)"##), Some(r##"((CCI(11)<80) AND (Ref(CCI(11),-1)>80)) OR ((CCI(11)<-80) AND Ref(CCI(11)>-80,-1))"##), Some(r##"(Fml("Candle - Hanging Man'")=-1) AND (CCI(11)>50)"##), Some(r##"((CCI(11)>-80) AND (Ref(CCI(11),-1)<-80)) OR ((CCI(11)<80) AND Ref(CCI(11)>80,-1))"##)], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "msws_hanging_man_hammer_mfi", name: r##"Hanging Man/Hammer & MFI"##, group: "GM3 · Candlestick", family: ("msws_gm3", "Morris GM3 candlestick + oscillator filter systems"), page: "22", status: Live, f: [Some(r##"(Fml("Candle - Hammer")=1) AND (Fml("Candle - Money Flow Index")<40)"##), Some(r##"((Fml("Candle - Money Flow Index")<70) AND (Ref(Fml("Candle - Money Flow Index"),-1)>70)) OR ((Fml("Candle - Money Flow Index")<30) AND (Ref(Fml("Candle - Money Flow Index")>30,-1)))"##), Some(r##"(Fml("Candle - Hanging Man'")=-1) AND (Fml("Candle - Money Flow Index")>60)"##), Some(r##"((Fml("Candle - Money Flow Index")>30) AND (Ref(Fml("Candle - Money Flow Index"),-1)<30)) OR ((Fml("Candle - Money Flow Index")>70) AND (Ref(Fml("Candle - Money Flow Index")<70,-1)))"##)], exec: EXEC_DEFAULT, notes: r##"Fml("Candle - Money Flow Index") is coded exactly as printed: (C-Ref(C,-1)*V) multiplies only the previous close by volume (MetaStock precedence)."## },
    Sys { id: "msws_hanging_man_hammer_rsi", name: r##"Hanging Man/Hammer & RSI"##, group: "GM3 · Candlestick", family: ("msws_gm3", "Morris GM3 candlestick + oscillator filter systems"), page: "22", status: Live, f: [Some(r##"(Fml("Candle - Hammer")=1) AND (RSI(14)<40)"##), Some(r##"((RSI(14)<70) AND (Ref(RSI(14),-1)>70)) OR ((RSI(14)<30) AND (Ref(RSI(14)>30,-1)))"##), Some(r##"(Fml("Candle - Hanging Man'")=-1) AND (RSI(14)>60)"##), Some(r##"((RSI(14)>30) AND (Ref(RSI(14),-1)<30)) OR ((RSI(14)>70) AND (Ref(RSI(14)<70,-1)))"##)], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "msws_harami_pctd", name: r##"Harami & %D"##, group: "GM3 · Candlestick", family: ("msws_gm3", "Morris GM3 candlestick + oscillator filter systems"), page: "23", status: Live, f: [Some(r##"(Fml("Candle - Harami - Bullish")=1) AND (Fml("Candle - Stochastics %D")<30)"##), Some(r##"((Fml("Candle - Stochastics %D")<80) AND (Ref(Fml("Candle - Stochastics %D"),-1)>80)) OR ((Fml("Candle - Stochastics %D")<20) AND Ref((Fml("Candle - Stochastics %D")>20),-1))"##), Some(r##"(Fml("Candle - Harami - Bearish")=-1) AND (Fml("Candle - Stochastics %D")>70)"##), Some(r##"((Fml("Candle - Stochastics %D")>20) AND (Ref(Fml("Candle - Stochastics %D"),-1)<20)) OR ((Fml("Candle - Stochastics %D")>80) AND Ref((Fml("Candle - Stochastics %D")<80),-1))"##)], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "msws_harami_cci", name: r##"Harami & CCI"##, group: "GM3 · Candlestick", family: ("msws_gm3", "Morris GM3 candlestick + oscillator filter systems"), page: "23", status: Live, f: [Some(r##"(Fml("Candle - Harami - Bullish")=1) AND (CCI(11)<-50)"##), Some(r##"((CCI(11)<80) AND (Ref(CCI(11),-1)>80)) OR ((CCI(11)<-80) AND Ref(CCI(11)>-80,-1))"##), Some(r##"(Fml("Candle - Harami - Bearish")=-1) AND (CCI(11)>50)"##), Some(r##"((CCI(11)>-80) AND (Ref(CCI(11),-1)<-80)) OR ((CCI(11)<80) AND Ref(CCI(11)>80,-1))"##)], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "msws_harami_mfi", name: r##"Harami & MFI"##, group: "GM3 · Candlestick", family: ("msws_gm3", "Morris GM3 candlestick + oscillator filter systems"), page: "23", status: Live, f: [Some(r##"(Fml("Candle - Harami - Bullish")=1) AND (Fml("Candle - Money Flow Index")<40)"##), Some(r##"((Fml("Candle - Money Flow Index")<70) AND (Ref(Fml("Candle - Money Flow Index"),-1)>70)) OR ((Fml("Candle - Money Flow Index")<30) AND (Ref(Fml("Candle - Money Flow Index")>30,-1)))"##), Some(r##"(Fml("Candle - Harami - Bearish")=-1) AND (Fml("Candle - Money Flow Index")>60)"##), Some(r##"((Fml("Candle - Money Flow Index")>30) AND (Ref(Fml("Candle - Money Flow Index"),-1)<30)) OR ((Fml("Candle - Money Flow Index")>70) AND (Ref(Fml("Candle - Money Flow Index")<70,-1)))"##)], exec: EXEC_DEFAULT, notes: r##"Fml("Candle - Money Flow Index") is coded exactly as printed: (C-Ref(C,-1)*V) multiplies only the previous close by volume (MetaStock precedence)."## },
    Sys { id: "msws_harami_rsi", name: r##"Harami & RSI"##, group: "GM3 · Candlestick", family: ("msws_gm3", "Morris GM3 candlestick + oscillator filter systems"), page: "24", status: Live, f: [Some(r##"(Fml("Candle - Harami - Bullish")=1) AND (RSI(14)<40)"##), Some(r##"((RSI(14)<70) AND (Ref(RSI(14),-1)>70)) OR ((RSI(14)<30) AND (Ref(RSI(14)>30,-1)))"##), Some(r##"(Fml("Candle - Harami - Bearish")=-1) AND (RSI(14)>60)"##), Some(r##"((RSI(14)>30) AND (Ref(RSI(14),-1)<30)) OR ((RSI(14)>70) AND (Ref(RSI(14)<70,-1)))"##)], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "msws_meeting_lines_pctd", name: r##"Meeting Lines & %D"##, group: "GM3 · Candlestick", family: ("msws_gm3", "Morris GM3 candlestick + oscillator filter systems"), page: "24", status: Live, f: [Some(r##"(Fml("Candle - Meeting Lines - Bullish")=1) AND (Fml("Candle - Stochastics %D")<30)"##), Some(r##"((Fml("Candle - Stochastics %D")<80) AND (Ref(Fml("Candle - Stochastics %D"),-1)>80)) OR ((Fml("Candle - Stochastics %D")<20) AND Ref((Fml("Candle - Stochastics %D")>20),-1))"##), Some(r##"(Fml("Candle - Meeting Lines - Bearish")=-1) AND (Fml("Candle - Stochastics %D")>70)"##), Some(r##"((Fml("Candle - Stochastics %D")>20) AND (Ref(Fml("Candle - Stochastics %D"),-1)<20)) OR ((Fml("Candle - Stochastics %D")>80) AND Ref((Fml("Candle - Stochastics %D")<80),-1))"##)], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "msws_meeting_lines_cci", name: r##"Meeting Lines & CCI"##, group: "GM3 · Candlestick", family: ("msws_gm3", "Morris GM3 candlestick + oscillator filter systems"), page: "24", status: Live, f: [Some(r##"(Fml("Candle - Meeting Lines - Bullish")=1) AND (CCI(11)<-50)"##), Some(r##"((CCI(11)<80) AND (Ref(CCI(11),-1)>80)) OR ((CCI(11)<-80) AND Ref(CCI(11)>-80,-1))"##), Some(r##"(Fml("Candle - Meeting Lines - Bearish")=-1) AND (CCI(11)>50)"##), Some(r##"((CCI(11)>-80) AND (Ref(CCI(11),-1)<-80)) OR ((CCI(11)<80) AND Ref(CCI(11)>80,-1))"##)], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "msws_meeting_lines_mfi", name: r##"Meeting Lines & MFI"##, group: "GM3 · Candlestick", family: ("msws_gm3", "Morris GM3 candlestick + oscillator filter systems"), page: "25", status: Live, f: [Some(r##"(Fml("Candle - Meeting Lines - Bullish")=1) AND (Fml("Candle - Money Flow Index")<40)"##), Some(r##"((Fml("Candle - Money Flow Index")<70) AND (Ref(Fml("Candle - Money Flow Index"),-1)>70)) OR ((Fml("Candle - Money Flow Index")<30) AND (Ref(Fml("Candle - Money Flow Index")>30,-1)))"##), Some(r##"Fml("Candle - Meeting Lines - Bearish")=-1 AND (Fml("Candle - Money Flow Index")>60)"##), Some(r##"((Fml("Candle - Money Flow Index")>30) AND (Ref(Fml("Candle - Money Flow Index"),-1)<30)) OR ((Fml("Candle - Money Flow Index")>70) AND (Ref(Fml("Candle - Money Flow Index")<70,-1)))"##)], exec: EXEC_DEFAULT, notes: r##"Fml("Candle - Money Flow Index") is coded exactly as printed: (C-Ref(C,-1)*V) multiplies only the previous close by volume (MetaStock precedence)."## },
    Sys { id: "msws_meeting_lines_rsi", name: r##"Meeting Lines & RSI"##, group: "GM3 · Candlestick", family: ("msws_gm3", "Morris GM3 candlestick + oscillator filter systems"), page: "25", status: Live, f: [Some(r##"(Fml("Candle - Meeting Lines - Bullish")=1) AND (RSI(14)<40)"##), Some(r##"((RSI(14)<70) AND (Ref(RSI(14),-1)>70)) OR ((RSI(14)<30) AND (Ref(RSI(14)>30,-1)))"##), Some(r##"Fml("Candle - Meeting Lines - Bearish")=-1 AND (RSI(14)>60)"##), Some(r##"((RSI(14)>30) AND (Ref(RSI(14),-1)<30)) OR ((RSI(14)>70) AND (Ref(RSI(14)<70,-1)))"##)], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "msws_stars_pctd", name: r##"Stars & %D"##, group: "GM3 · Candlestick", family: ("msws_gm3", "Morris GM3 candlestick + oscillator filter systems"), page: "26", status: Live, f: [Some(r##"(Fml("Candle - Morning Star")=1) AND (Fml("Candle - Stochastics %D")<30)"##), Some(r##"((Fml("Candle - Stochastics %D")<80) AND (Ref(Fml("Candle - Stochastics %D"),-1)>80)) OR ((Fml("Candle - Stochastics %D")<20) AND Ref((Fml("Candle - Stochastics %D")>20),-1))"##), Some(r##"(Fml("Candle - Evening Star")=-1) AND (Fml("Candle - Stochastics %D")>70)"##), Some(r##"((Fml("Candle - Stochastics %D")>20) AND (Ref(Fml("Candle - Stochastics %D"),-1)<20)) OR ((Fml("Candle - Stochastics %D")>80) AND Ref((Fml("Candle - Stochastics %D")<80),-1))"##)], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "msws_stars_cci", name: r##"Stars & CCI"##, group: "GM3 · Candlestick", family: ("msws_gm3", "Morris GM3 candlestick + oscillator filter systems"), page: "26", status: Live, f: [Some(r##"(Fml("Candle - Morning Star")=1) AND (CCI(11)<-50)"##), Some(r##"((CCI(11)<80) AND (Ref(CCI(11),-1)>80)) OR ((CCI(11)<-80) AND Ref(CCI(11)>-80,-1))"##), Some(r##"(Fml("Candle - Evening Star")=-1) AND (CCI(11)>50)"##), Some(r##"((CCI(11)>-80) AND (Ref(CCI(11),-1)<-80)) OR ((CCI(11)<80) AND Ref(CCI(11)>80,-1))"##)], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "msws_stars_mfi", name: r##"Stars & MFI"##, group: "GM3 · Candlestick", family: ("msws_gm3", "Morris GM3 candlestick + oscillator filter systems"), page: "26", status: Live, f: [Some(r##"(Fml("Candle - Morning Star")=1) AND (Fml("Candle - Money Flow Index")<40)"##), Some(r##"((Fml("Candle - Money Flow Index")<70) AND (Ref(Fml("Candle - Money Flow Index"),-1)>70)) OR ((Fml("Candle - Money Flow Index")<30) AND (Ref(Fml("Candle - Money Flow Index")>30,-1)))"##), Some(r##"(Fml("Candle - Evening Star")=-1) AND (Fml("Candle - Money Flow Index")>60)"##), Some(r##"((Fml("Candle - Money Flow Index")>30) AND (Ref(Fml("Candle - Money Flow Index"),-1)<30)) OR ((Fml("Candle - Money Flow Index")>70) AND (Ref(Fml("Candle - Money Flow Index")<70,-1)))"##)], exec: EXEC_DEFAULT, notes: r##"Fml("Candle - Money Flow Index") is coded exactly as printed: (C-Ref(C,-1)*V) multiplies only the previous close by volume (MetaStock precedence)."## },
    Sys { id: "msws_stars_rsi", name: r##"Stars & RSI"##, group: "GM3 · Candlestick", family: ("msws_gm3", "Morris GM3 candlestick + oscillator filter systems"), page: "27", status: Live, f: [Some(r##"(Fml("Candle - Morning Star")=1) AND (RSI(14)<40)"##), Some(r##"((RSI(14)<70) AND (Ref(RSI(14),-1)>70)) OR ((RSI(14)<30) AND (Ref(RSI(14)>30,-1)))"##), Some(r##"(Fml("Candle - Evening Star")=-1) AND (RSI(14)>60)"##), Some(r##"((RSI(14)>30) AND (Ref(RSI(14),-1)<30)) OR ((RSI(14)>70) AND (Ref(RSI(14)<70,-1)))"##)], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "msws_directional_movement_roc_system", name: r##"Directional Movement ROC System"##, group: "GM4 · New and different", family: ("msws_gm4", "Morris GM4 new and different systems"), page: "27", status: Live, f: [Some(r##"Fml("New - Directional Movement ROC") > 0"##), Some(r##"Fml("New - Directional Movement ROC") < 0"##), Some(r##"Fml("New - Directional Movement ROC") < 0"##), Some(r##"Fml("New - Directional Movement ROC") > 0"##)], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "msws_directional_movement_roc_ma_system", name: r##"Directional Movement ROC/MA System"##, group: "GM4 · New and different", family: ("msws_gm4", "Morris GM4 new and different systems"), page: "28", status: Live, f: [Some(r##"(Fml("New - Directional Movement ROC") > Mov(Fml("New - Directional Movement ROC"),7,S)) AND (Fml("New - Directional Movement ROC")<0)"##), Some(r##"(Fml("New - Directional Movement ROC") > Mov(Fml("New - Directional Movement ROC"),7,S)) AND (Fml("New - Directional Movement ROC")<0)"##), Some(r##"(Fml("New - Directional Movement ROC") < Mov(Fml("New - Directional Movement ROC"),7,S)) AND (Fml("New - Directional Movement ROC")>0)"##), Some(r##"(Fml("New - Directional Movement ROC") > Mov(Fml("New - Directional Movement ROC"),7,S)) AND (Fml("New - Directional Movement ROC")<0)"##)], exec: EXEC_DEFAULT, notes: r##"Close Long is printed identical to Enter Long (probable typo); coded literally."## },
    Sys { id: "msws_directional_movement_roc_ma_system_w_stops", name: r##"Directional Movement ROC/MA System w/ stops"##, group: "GM4 · New and different", family: ("msws_gm4", "Morris GM4 new and different systems"), page: "28", status: Live, f: [Some(r##"(Fml("New - Directional Movement ROC") > Mov(Fml("New - Directional Movement ROC"),7,S)) AND (Fml("New - Directional Movement ROC")<0)"##), Some(r##"((Fml("New - Directional Movement ROC") < Mov(Fml("New - Directional Movement ROC"),7,S)) AND (Fml("New - Directional Movement ROC")>0)) OR (Fml("New - Directional Movement ROC") > Mov(Fml("New - Directional Movement ROC"),7,S))"##), Some(r##"(Fml("New - Directional Movement ROC") < Mov(Fml("New - Directional Movement ROC"),7,S)) AND (Fml("New - Directional Movement ROC")>0)"##), Some(r##"((Fml("New - Directional Movement ROC") > Mov(Fml("New - Directional Movement ROC"),7,S)) AND (Fml("New - Directional Movement ROC")<0)) OR (Fml("New - Directional Movement ROC") < Mov(Fml("New - Directional Movement ROC"),7,S))"##)], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "msws_double_mo_system", name: r##"Double MO System"##, group: "GM4 · New and different", family: ("msws_gm4", "Morris GM4 new and different systems"), page: "28", status: NotTestable("Fml(\"New - Morris Double Momentum Oscillator\") uses ROC(C,12.8,%) and ROC(C,19.2,%): fractional look-back periods whose MetaStock handling the source does not define"), f: [Some(r##"Fml("New - Morris Double Momentum Oscillator") < 0"##), Some(r##"Fml("New - Morris Double Momentum Oscillator") > 0"##), Some(r##"Fml("New - Morris Double Momentum Oscillator") > 0"##), Some(r##"Fml("New - Morris Double Momentum Oscillator") < 0"##)], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "msws_envelope_crossings", name: r##"Envelope Crossings"##, group: "GM4 · New and different", family: ("msws_gm4", "Morris GM4 new and different systems"), page: "29", status: Live, f: [Some(r##"(L<Mov(C,15,E) - (.04*Mov(C,15,E))) AND (Ref(L,-1)>Mov(C,15,E) - (.04*Mov(C,15,E)))"##), Some(r##"(H>Mov(C,15,E) + (.04*Mov(C,15,E))) OR (H<Mov(C,15,E) - (.04*Mov(C,15,E)))"##), Some(r##"(H>Mov(C,15,E) + (.04*Mov(C,15,E))) AND (Ref(H,-1)<Mov(C,15,E) + (.04*Mov(C,15,E)))"##), Some(r##"(L<Mov(C,15,E) - (.04*Mov(C,15,E))) OR (L>Mov(C,15,E) + (.04*Mov(C,15,E)))"##)], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "msws_macd_true", name: r##"MACD (True)"##, group: "GM4 · New and different", family: ("msws_gm4", "Morris GM4 new and different systems"), page: "29", status: Live, f: [Some(r##"MACD()<0 AND MACD()>Mov(MACD(),9,E) AND Ref(MACD(),-1)<=Ref(Mov(MACD(),9,E),-1)"##), Some(r##"MACD()>0 AND MACD()<Mov(MACD(),9,E) AND Ref(MACD(),-1)>=Ref(Mov(MACD(),9,E),-1)"##), Some(r##"MACD()>0 AND MACD()<Mov(MACD(),9,E) AND Ref(MACD(),-1)>=Ref(Mov(MACD(),9,E),-1)"##), Some(r##"MACD()<0 AND MACD()>Mov(MACD(),9,E) AND Ref(MACD(),-1)<=Ref(Mov(MACD(),9,E),-1)"##)], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "msws_morris_master_trader", name: r##"Morris Master Trader"##, group: "GM4 · New and different", family: ("msws_gm4", "Morris GM4 new and different systems"), page: "29", status: Live, f: [Some(r##"(Ref(Mov((((L+2*Std(C,21) - Mov(L,21,S))/(4*Std(C,21)))*100),1,S),-1)<5) AND C > Ref(C,-1)"##), Some(r##"(((((H+2*Std(C,21) - Mov(H,21,S))/(4*Std(C,21)))*100)<95)) OR Stoch(14,3)<Mov(Stoch(14,3),3,S) AND Stoch(14,3)<80"##), Some(r##"(Ref(Mov((((H+2*Std(C,21) - Mov(H,21,S))/(4*Std(C,21)))*100),1,S),-1)>95) AND C < Ref(C,-1)"##), Some(r##"((((L+2*Std(C,21) - Mov(L,21,S))/(4*Std(C,21)))*100)>5) OR Stoch(14,3)>Mov(Stoch(14,3),3,S) AND Stoch(14,3)>20"##)], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "msws_morris_volume_rsi", name: r##"Morris Volume-RSI"##, group: "GM4 · New and different", family: ("msws_gm4", "Morris GM4 new and different systems"), page: "30", status: Live, f: [Some(r##"(100 - 100/(1+(Mov(If(ROC(C,1,$)>0,ROC(C,1,$)*VOLUME,0),14,S)/ Mov(If(ROC(C,1,$)<0,-ROC(C,1,$)*VOLUME,0),14,S))))>25 AND (Ref((100 - 100/(1+(Mov(If(ROC(C,1,$)>0,ROC(C,1,$)*VOLUME,0),14,S)/ Mov(If(ROC(C,1,$)<0,-ROC(C,1,$)*VOLUME,0),14,S)))),-1)<25)"##), Some(r##"(100 - 100/(1+(Mov(If(ROC(C,1,$)>0,ROC(C,1,$)*VOLUME,0),14,S)/ Mov(If(ROC(C,1,$)<0,-ROC(C,1,$)*VOLUME,0),14,S))))<75 AND (Ref((100 - 100/(1+(Mov(If(ROC(C,1,$)>0,ROC(C,1,$)*VOLUME,0),14,S)/ Mov(If(ROC(C,1,$)<0,-ROC(C,1,$)*VOLUME,0),14,S)))),-1)>75)"##), Some(r##"(100 - 100/(1+(Mov(If(ROC(C,1,$)>0,ROC(C,1,$)*VOLUME,0),14,S)/ Mov(If(ROC(C,1,$)<0,-ROC(C,1,$)*VOLUME,0),14,S))))<75 AND (Ref((100 - 100/(1+(Mov(If(ROC(C,1,$)>0,ROC(C,1,$)*VOLUME,0),14,S)/ Mov(If(ROC(C,1,$)<0,-ROC(C,1,$)*VOLUME,0),14,S)))),-1)>75)"##), Some(r##"(100 - 100/(1+(Mov(If(ROC(C,1,$)>0,ROC(C,1,$)*VOLUME,0),14,S)/ Mov(If(ROC(C,1,$)<0,-ROC(C,1,$)*VOLUME,0),14,S))))>25 AND (Ref((100 - 100/(1+(Mov(If(ROC(C,1,$)>0,ROC(C,1,$)*VOLUME,0),14,S)/ Mov(If(ROC(C,1,$)<0,-ROC(C,1,$)*VOLUME,0),14,S)))),-1)<25)"##)], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "msws_pring_s_kst_w_moving_average", name: r##"Pring's KST w/ moving average"##, group: "GM4 · New and different", family: ("msws_gm4", "Morris GM4 new and different systems"), page: "30", status: Live, f: [Some(r##"Fml("New - Pring's Short KST - Daily") > Mov(Fml("New - Pring's Short KST - Daily"),7,S)"##), Some(r##"Fml("New - Pring's Short KST - Daily") < Mov(Fml("New - Pring's Short KST - Daily"),7,S)"##), Some(r##"Fml("New - Pring's Short KST - Daily") < Mov(Fml("New - Pring's Short KST - Daily"),7,S)"##), Some(r##"Fml("New - Pring's Short KST - Daily") > Mov(Fml("New - Pring's Short KST - Daily"),7,S)"##)], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "msws_pring_s_kst_w_threshold_and_ma", name: r##"Pring's KST w/ threshold and MA"##, group: "GM4 · New and different", family: ("msws_gm4", "Morris GM4 new and different systems"), page: "31", status: Live, f: [Some(r##"(Fml("New - Pring's Short KST - Daily") > Mov(Fml("New - Pring's Short KST - Daily"),5,S)) AND (Fml("New - Pring's Short KST - Daily") < -0)"##), Some(r##"Fml("New - Pring's Short KST - Daily") < Mov(Fml("New - Pring's Short KST - Daily"),5,S) AND (Fml("New - Pring's Short KST - Daily") > 0)"##), Some(r##"Fml("New - Pring's Short KST - Daily") < Mov(Fml("New - Pring's Short KST - Daily"),5,S) AND (Fml("New - Pring's Short KST - Daily") > 0)"##), Some(r##"Fml("New - Pring's Short KST - Daily") > Mov(Fml("New - Pring's Short KST - Daily"),5,S) AND (Fml("New - Pring's Short KST - Daily") < -0)"##)], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "msws_rsi_pctd_and_cci_trader", name: r##"RSI, %D, and CCI Trader"##, group: "GM4 · New and different", family: ("msws_gm4", "Morris GM4 new and different systems"), page: "31", status: Live, f: [Some(r##"RSI(14)<35 AND Stoch(14,3)<25 AND CCI(11)<-100"##), Some(r##"RSI(14)>65 AND Stoch(14,3)>75 AND CCI(11)>100"##), Some(r##"RSI(14)>65 AND Stoch(14,3)>75 AND CCI(11)>100"##), Some(r##"RSI(14)<35 AND Stoch(14,3)<25 AND CCI(11)<-100"##)], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "msws_triple_averages_in_sync", name: r##"Triple Averages In-Sync"##, group: "GM4 · New and different", family: ("msws_gm4", "Morris GM4 new and different systems"), page: "32", status: Live, f: [Some(r##"(Mov(C,5,E)>Mov(C,13,E)) AND (Mov(C,13,E)>Mov(C,21,E))"##), Some(r##"(Mov(C,5,E)<Mov(C,13,E)) AND (Mov(C,13,E)<Mov(C,21,E))"##), Some(r##"(Mov(C,5,E)<Mov(C,13,E)) AND (Mov(C,13,E)<Mov(C,21,E))"##), Some(r##"(Mov(C,5,E)>Mov(C,13,E)) AND (Mov(C,13,E)>Mov(C,21,E))"##)], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "msws_generic_oscillator_w_threshold_gm4", name: r##"Generic Oscillator w/ threshold"##, group: "GM4 · New and different", family: ("msws_gm4", "Morris GM4 new and different systems"), page: "33", status: Live, f: [Some(r##"Fml("New - CCI")>-80"##), Some(r##"Fml("New - CCI")<80"##), Some(r##"Fml("New - CCI")<80"##), Some(r##"Fml("New - CCI")>-80"##)], exec: EXEC_DEFAULT, notes: r##"Printed as a template; coded with the indicator printed (New - CCI)."## },
    Sys { id: "msws_generic_oscillator_w_threshold_and_ma_gm4", name: r##"Generic Oscillator w/ threshold and MA"##, group: "GM4 · New and different", family: ("msws_gm4", "Morris GM4 new and different systems"), page: "33", status: Live, f: [Some(r##"(Fml("New - CCI") > Mov(Fml("New - CCI"),5,S)) AND (Fml("New - CCI") < -80)"##), Some(r##"Fml("New - CCI") < Mov(Fml("New - CCI"),5,S) AND (Fml("New - CCI") > 80)"##), Some(r##"Fml("New - CCI") < Mov(Fml("New - CCI"),5,S) AND (Fml("New - CCI") > 80)"##), Some(r##"Fml("New - CCI") > Mov(Fml("New - CCI"),5,S) AND (Fml("New - CCI") < -80)"##)], exec: EXEC_DEFAULT, notes: r##"Printed as a template; coded with the indicator printed (New - CCI)."## },
    Sys { id: "msws_generic_oscillator_w_ma_gm4", name: r##"Generic Oscillator w/ MA"##, group: "GM4 · New and different", family: ("msws_gm4", "Morris GM4 new and different systems"), page: "33", status: Live, f: [Some(r##"Fml("New - CCI") > Mov(Fml("New - CCI"),7,S)"##), Some(r##"Fml("New - CCI") < Mov(Fml("New - CCI"),7,S)"##), Some(r##"Fml("New - CCI") < Mov(Fml("New - CCI"),7,S)"##), Some(r##"Fml("New - CCI") > Mov(Fml("New - CCI"),7,S)"##)], exec: EXEC_DEFAULT, notes: r##"Printed as a template; coded with the indicator printed (New - CCI)."## },
    Sys { id: "msws_linear_trend_system", name: r##"Linear Trend System"##, group: "GM4 · New and different", family: ("msws_gm4", "Morris GM4 new and different systems"), page: "34", status: Live, f: [Some(r##"Fml("New - Linear Trend Oscillator") > 0"##), Some(r##"Fml("New - Linear Trend Oscillator") < 0"##), Some(r##"Fml("New - Linear Trend Oscillator") < 0"##), Some(r##"Fml("New - Linear Trend Oscillator") > 0"##)], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "msws_linear_trend_system_w_entry_exit_chg", name: r##"Linear Trend System w/ Entry/Exit-Chg"##, group: "GM4 · New and different", family: ("msws_gm4", "Morris GM4 new and different systems"), page: "34", status: Live, f: [Some(r##"(Fml("New - Linear Trend Oscillator") < 0) AND (Fml("New - Linear Trend Oscillator") > Ref(Fml("New - Linear Trend Oscillator"),-1))"##), Some(r##"(Fml("New - Linear Trend Oscillator") > 0) AND (Fml("New - Linear Trend Oscillator") < Ref(Fml("New - Linear Trend Oscillator"),-1))"##), Some(r##"(Fml("New - Linear Trend Oscillator") > 0) AND (Fml("New - Linear Trend Oscillator") < Ref(Fml("New - Linear Trend Oscillator"),-1))"##), Some(r##"(Fml("New - Linear Trend Oscillator") < 0) AND (Fml("New - Linear Trend Oscillator") > Ref(Fml("New - Linear Trend Oscillator"),-1))"##)], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "msws_linear_trend_system_w_ma", name: r##"Linear Trend System w/ MA"##, group: "GM4 · New and different", family: ("msws_gm4", "Morris GM4 new and different systems"), page: "34", status: Live, f: [Some(r##"(Fml("New - Linear Trend Oscillator") < 0) AND (Fml("New - Linear Trend Oscillator") > Mov(Fml("New - Linear Trend Oscillator"),7,S))"##), Some(r##"(Fml("New - Linear Trend Oscillator") > 0) AND (Fml("New - Linear Trend Oscillator") < Mov(Fml("New - Linear Trend Oscillator"),7,S))"##), Some(r##"(Fml("New - Linear Trend Oscillator") > 0) AND (Fml("New - Linear Trend Oscillator") < Mov(Fml("New - Linear Trend Oscillator"),7,S))"##), Some(r##"(Fml("New - Linear Trend Oscillator") < 0) AND (Fml("New - Linear Trend Oscillator") > Mov(Fml("New - Linear Trend Oscillator"),7,S))"##)], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "msws_gm6_cci_100_100_system", name: r##"GM6(83) - CCI 100/-100 System (Volume 1, p. 120)"##, group: "GM6 · S&C 1982-89", family: ("msws_gm6", "Morris GM6 S&C 1982-89 systems"), page: "35", status: Live, f: [Some(r##"Cross(Fml("GM6(83) - Commodity Channel Index"),-100)"##), Some(r##"Cross(100,(Fml("GM6(83) - Commodity Channel Index"))) OR Cross(-100,(Fml("GM6(83) - Commodity Channel Index")))"##), Some(r##"Cross(100,(Fml("GM6(83) - Commodity Channel Index")))"##), Some(r##"Cross(Fml("GM6(83) - Commodity Channel Index"),-100) OR Cross(Fml("GM6(83) - Commodity Channel Index"),100)"##)], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "msws_gm6_cci_zero_cross_system", name: r##"GM6(83) - CCI Zero Cross System (Volume 1, p. 120)"##, group: "GM6 · S&C 1982-89", family: ("msws_gm6", "Morris GM6 S&C 1982-89 systems"), page: "35", status: Live, f: [Some(r##"Fml("GM6(83) - Commodity Channel Index")>0"##), Some(r##"Fml("GM6(83) - Commodity Channel Index")<0"##), Some(r##"Fml("GM6(83) - Commodity Channel Index")<0"##), Some(r##"Fml("GM6(83) - Commodity Channel Index")>0"##)], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "msws_gm6_swing_line_crossover", name: r##"GM6(83) - Swing Line Crossover (Volume 1, p. 48)"##, group: "GM6 · S&C 1982-89", family: ("msws_gm6", "Morris GM6 S&C 1982-89 systems"), page: "36", status: Live, f: [Some(r##"C>Mov((Fml("GM6(83) - Swing Line'")),5,S)"##), Some(r##"C<Mov((Fml("GM6(83) - Swing Line'")),5,S)"##), Some(r##"C<Mov((Fml("GM6(83) - Swing Line'")),5,S)"##), Some(r##"C>Mov((Fml("GM6(83) - Swing Line'")),5,S)"##)], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "msws_gm6_morris_vol_rsi_system", name: r##"GM6(85) - Morris Vol RSI System (Oct., p. 16)"##, group: "GM6 · S&C 1982-89", family: ("msws_gm6", "Morris GM6 S&C 1982-89 systems"), page: "36", status: NotTestable("Fml(\"GM6(85) - Morris Volume RSI\") is printed with one more closing parenthesis than opening ones, so MetaStock would reject it; which parenthesis is wrong is not stated (the GM4 Morris Volume-RSI system prints its own, balanced formula and is Live)"), f: [Some(r##"Fml("GM6(85) - Morris Volume RSI")>25 AND Ref(Fml("GM6(85) - Morris Volume RSI"),-1)<25"##), Some(r##"(Fml("GM6(85) - Morris Volume RSI")<75 AND Ref(Fml("GM6(85) - Morris Volume RSI"),-1)>75) OR Fml("GM6(85) - Morris Volume RSI") <24"##), Some(r##"Fml("GM6(85) - Morris Volume RSI")<75 AND Ref(Fml("GM6(85) - Morris Volume RSI"),-1)>75"##), Some(r##"(Fml("GM6(85) - Morris Volume RSI")<25 AND Ref(Fml("GM6(85) - Morris Volume RSI"),-1)<25) OR Fml("GM6(85) - Morris Volume RSI") >76"##)], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "msws_gm6_rsi_quality_system", name: r##"GM6(86) - RSI Quality System (Oct., p. 40)"##, group: "GM6 · S&C 1982-89", family: ("msws_gm6", "Morris GM6 S&C 1982-89 systems"), page: "36", status: Live, f: [Some(r##"Fml("GM6(86) - RSI Quality Factor")>0"##), Some(r##"Fml("GM6(86) - RSI Quality Factor")<0"##), Some(r##"Fml("GM6(86) - RSI Quality Factor")<0"##), Some(r##"Fml("GM6(86) - RSI Quality Factor")>0"##)], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "msws_gm6_enhanced_index_trader", name: r##"GM6(87) - Enhanced Index Trader (May, p. 45)"##, group: "GM6 · S&C 1982-89", family: ("msws_gm6", "Morris GM6 S&C 1982-89 systems"), page: "36", status: NotTestable("no Close Long / Close Short formula is printed (Enter Long and Enter Short only); the source does not say the opposite entry closes the position"), f: [Some(r##"Mov((Fml("GM6(87) - Enhanced Index")),4,S) >Ref(Mov((Fml("GM6(87) - Enhanced Index")),4,S),-1)"##), None, Some(r##"Mov((Fml("GM6(87) - Enhanced Index")),4,S) <Ref(Mov((Fml("GM6(87) - Enhanced Index")),4,S),-1)"##), None], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "msws_gm6_macd_mo_trader", name: r##"GM6(88) - MACD-MO Trader (Aug., p. 33)"##, group: "GM6 · S&C 1982-89", family: ("msws_gm6", "Morris GM6 S&C 1982-89 systems"), page: "37", status: Live, f: [Some(r##"Fml("GM6(88) - MACD-MO") > Ref(Fml("GM6(88) - MACD-MO"),-2)"##), Some(r##"Fml("GM6(88) - MACD-MO") < Ref(Fml("GM6(88) - MACD-MO"),-2)"##), Some(r##"Fml("GM6(88) - MACD-MO") < Ref(Fml("GM6(88) - MACD-MO"),-2)"##), Some(r##"Fml("GM6(88) - MACD-MO") > Ref(Fml("GM6(88) - MACD-MO"),-2)"##)], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "msws_gm6_pctv_system", name: r##"GM6(89) - %V System (Sep., p. 92)"##, group: "GM6 · S&C 1982-89", family: ("msws_gm6", "Morris GM6 S&C 1982-89 systems"), page: "37", status: Live, f: [Some(r##"Mov((Fml("GM6(89) - %V")),3,E) > 40 AND Ref(Mov((Fml("GM6(89) - %V")),3,E),-1) < 40"##), Some(r##"Mov((Fml("GM6(89) - %V")),3,E) > 50"##), Some(r##"Mov((Fml("GM6(89) - %V")),3,E) < 60 AND Ref(Mov((Fml("GM6(89) - %V")),3,E),-1) > 60"##), Some(r##"Mov((Fml("GM6(89) - %V")),3,E) < 50"##)], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "msws_gm6_channel_system_12", name: r##"GM6(89) - Channel System-12 (Sep., p. 62)"##, group: "GM6 · S&C 1982-89", family: ("msws_gm6", "Morris GM6 S&C 1982-89 systems"), page: "37", status: NotTestable("no Close Long / Close Short formula is printed (Enter Long and Enter Short only); the source does not say the opposite entry closes the position"), f: [Some(r##"Fml("GM6(89) - Channel Breakout-12") = 1"##), None, Some(r##"Fml("GM6(89) - Channel Breakout-12") = -1"##), None], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "msws_gm6_channel_system_8", name: r##"GM6(89) - Channel System-8 (Sep., p. 62)"##, group: "GM6 · S&C 1982-89", family: ("msws_gm6", "Morris GM6 S&C 1982-89 systems"), page: "37", status: NotTestable("no Close Long / Close Short formula is printed (Enter Long and Enter Short only); the source does not say the opposite entry closes the position"), f: [Some(r##"Fml("GM6(89) - Channel Breakout-8") = 1"##), None, Some(r##"Fml("GM6(89) - Channel Breakout-12") = -1"##), None], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "msws_gm6_eom_baseline_crossover", name: r##"GM6(89) - EOM Baseline Crossover (Dec., p. 73)"##, group: "GM6 · S&C 1982-89", family: ("msws_gm6", "Morris GM6 S&C 1982-89 systems"), page: "37", status: NotTestable("no Close Long / Close Short formula is printed (Enter Long and Enter Short only); the source does not say the opposite entry closes the position"), f: [Some(r##"Mov((Fml("GM6(89) - EOM")),3,E) > 0 AND Ref(Mov((Fml("GM6(89) - EOM")),3,E),-1) < 0"##), None, Some(r##"Mov((Fml("GM6(89) - EOM")),3,E) < 0 AND Ref(Mov((Fml("GM6(89) - EOM")),3,E),-1) > 0"##), None], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "msws_gm6_va_crossover_system", name: r##"GM6(89) - VA Crossover System"##, group: "GM6 · S&C 1982-89", family: ("msws_gm6", "Morris GM6 S&C 1982-89 systems"), page: "38", status: Live, f: [Some(r##"Fml("GM6(89) - Volume Accumulator (Chaikin)") > 0"##), Some(r##"Fml("GM6(89) - Volume Accumulator (Chaikin)") < 0"##), Some(r##"Fml("GM6(89) - Volume Accumulator (Chaikin)") < 0"##), Some(r##"Fml("GM6(89) - Volume Accumulator (Chaikin)") > 0"##)], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "msws_gm6_volatility_system", name: r##"GM6(89) - Volatility System (Sep., p. 92)"##, group: "GM6 · S&C 1982-89", family: ("msws_gm6", "Morris GM6 S&C 1982-89 systems"), page: "38", status: Live, f: [Some(r##"Fml("GM6(89) - Up Volatility - Down Volatility")>-.20 AND Ref(Fml("GM6(89) - Up Volatility - Down Volatility"),-1)<-.20"##), Some(r##"Fml("GM6(89) - Up Volatility - Down Volatility")<0 AND Ref(Fml("GM6(89) - Up Volatility - Down Volatility"),-1)>0"##), Some(r##"Fml("GM6(89) - Up Volatility - Down Volatility")<.20 AND Ref(Fml("GM6(89) - Up Volatility - Down Volatility"),-1)>.20"##), Some(r##"Fml("GM6(89) - Up Volatility - Down Volatility")>0 AND Ref(Fml("GM6(89) - Up Volatility - Down Volatility"),-1)<0"##)], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "msws_gm7_barics_pctb_system", name: r##"GM7(90) - Barics %B System (May,. p. 66)"##, group: "GM7 · S&C 1990-91", family: ("msws_gm7", "Morris GM7 S&C 1990-91 systems"), page: "38", status: Live, f: [Some(r##"Mov((Fml("GM7(90) - %B (Barics)")),3,S) > 30 AND Ref(Mov((Fml("GM7(90) - %B (Barics)")),3,S),-1) < 30"##), Some(r##"(Mov((Fml("GM7(90) - %B (Barics)")),3,S) < 70 AND Ref(Mov((Fml("GM7(90) - %B (Barics)")),3,S),-1) > 70) OR Mov((Fml("GM7(90) - %B (Barics)")),3,S) < 25"##), Some(r##"Mov((Fml("GM7(90) - %B (Barics)")),3,S) < 70 AND Ref(Mov((Fml("GM7(90) - %B (Barics)")),3,S),-1) > 70"##), Some(r##"(Mov((Fml("GM7(90) - %B (Barics)")),3,S) > 30 AND Ref(Mov((Fml("GM7(90) - %B (Barics)")),3,S),-1) < 30) OR Mov((Fml("GM7(90) - %B (Barics)")),3,S) > 75"##)], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "msws_gm7_short_signals_for_dji", name: r##"GM7(90) - Short Signals for DJI"##, group: "GM7 · S&C 1990-91", family: ("msws_gm7", "Morris GM7 S&C 1990-91 systems"), page: "39", status: NotTestable("no Close Long / Close Short formula is printed (Enter Long and Enter Short only); the source does not say the opposite entry closes the position"), f: [Some(r##"Stoch(20,3) < 70  AND (Stoch(20,1) < (Stoch(20,3)-4))"##), None, Some(r##"Stoch(20,1) > (Stoch(20,3) + 4) AND Stoch(20,3) > 30"##), None], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "msws_gm7_pctm_and_std_dev", name: r##"GM7(91) - %M and Std Dev (Nov., p. 54)"##, group: "GM7 · S&C 1990-91", family: ("msws_gm7", "Morris GM7 S&C 1990-91 systems"), page: "39", status: Live, f: [Some(r##"Stdev(Fml("GM7(91) - (%M) Midpoint Oscillator"),11)>40"##), Some(r##"Stdev(Fml("GM7(91) - (%M) Midpoint Oscillator"),11)<40"##), Some(r##"Stdev(Fml("GM7(91) - (%M) Midpoint Oscillator"),11)<40"##), Some(r##"Stdev(Fml("GM7(91) - (%M) Midpoint Oscillator"),11)>40"##)], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "msws_gm7_pctm_zero_crossover", name: r##"GM7(91) - %M Zero Crossover (Nov., p. 54)"##, group: "GM7 · S&C 1990-91", family: ("msws_gm7", "Morris GM7 S&C 1990-91 systems"), page: "39", status: NotTestable("no Close Long / Close Short formula is printed (Enter Long and Enter Short only); the source does not say the opposite entry closes the position"), f: [Some(r##"Fml("GM7(91) - %M Smoothed") > 1"##), None, Some(r##"Fml("GM7(91) - %M Smoothed") < -1"##), None], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "msws_gm7_adx_system", name: r##"GM7(91) - ADX System (Mar., p. 30)"##, group: "GM7 · S&C 1990-91", family: ("msws_gm7", "Morris GM7 S&C 1990-91 systems"), page: "39", status: Live, f: [Some(r##"ADX(14) > 20 AND ADX(14) > Ref(ADX(14),-1)"##), Some(r##"ADX(14) > 50 AND ADX(14) > Ref(ADX(14),-1)"##), Some(r##"ADX(14) < 50 AND ADX(14) < Ref(ADX(14),-1)"##), Some(r##"ADX(14) < 20 AND ADX(14) < Ref(ADX(14),-1)"##)], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "msws_gm7_double_stochastic_trader", name: r##"GM7(91) - Double Stochastic Trader (Jan., p. 42)"##, group: "GM7 · S&C 1990-91", family: ("msws_gm7", "Morris GM7 S&C 1990-91 systems"), page: "39", status: NotTestable("no Close Long / Close Short formula is printed (Enter Long and Enter Short only); the source does not say the opposite entry closes the position"), f: [Some(r##"Fml("GM7(91) - Double Smooth Stochastic") > 20 AND Ref(Fml("GM7(91) - Double Smooth Stochastic"),-1) < 20"##), None, Some(r##"Fml("GM7(91) - Double Smooth Stochastic") < 80 AND Ref(Fml("GM7(91) - Double Smooth Stochastic"),-1) > 80"##), None], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "msws_gm7_hlc_index_system", name: r##"GM7(91) - HLC Index System (Jan., p. 42)"##, group: "GM7 · S&C 1990-91", family: ("msws_gm7", "Morris GM7 S&C 1990-91 systems"), page: "40", status: NotTestable("no Close Long / Close Short formula is printed (Enter Long and Enter Short only); the source does not say the opposite entry closes the position"), f: [Some(r##"Fml("GM7(91) - HLC Index") > 38 AND Ref(Fml("GM7(91) - HLC Index"),-1) < 38"##), None, Some(r##"Fml("GM7(91) - HLC Index") < 62 AND Ref(Fml("GM7(91) - HLC Index"),-1) > 62"##), None], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "msws_gm7_maddox_system", name: r##"GM7(91) - Maddox System  (Apr., p. 58)"##, group: "GM7 · S&C 1990-91", family: ("msws_gm7", "Morris GM7 S&C 1990-91 systems"), page: "40", status: NotTestable("no Close Long / Close Short formula is printed (Enter Long and Enter Short only); the source does not say the opposite entry closes the position"), f: [Some(r##"Fml("GM7(91) - Maddox Momentum") > -600 AND Ref(Fml("GM7(91) - Maddox Momentum"),-1) < -600"##), None, Some(r##"Fml("GM7(91) - Maddox Momentum") < 600 AND Ref(Fml("GM7(91) - Maddox Momentum"),-1) > 600"##), None], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "msws_gm7_pivot_point_trader_fast_jan_p_62", name: r##"GM7(91) - Pivot Point Trader (Fast) Jan., p. 62)"##, group: "GM7 · S&C 1990-91", family: ("msws_gm7", "Morris GM7 S&C 1990-91 systems"), page: "40", status: NotTestable("no Close Long / Close Short formula is printed (Enter Long and Enter Short only); the source does not say the opposite entry closes the position"), f: [Some(r##"C > Fml("GM7(91) - Pivot Points")"##), None, Some(r##"C < Fml("GM7(91) - Pivot Points")"##), None], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "msws_gm7_true_strength_system", name: r##"GM7(91) - True Strength System (Nov., p. 18)"##, group: "GM7 · S&C 1990-91", family: ("msws_gm7", "Morris GM7 S&C 1990-91 systems"), page: "40", status: NotTestable("no Close Long / Close Short formula is printed (Enter Long and Enter Short only); the source does not say the opposite entry closes the position"), f: [Some(r##"Fml("GM7(91) - True Strength Index") > -.4 AND Ref(Fml("GM7(91) - True Strength Index"),-1) < -.4"##), None, Some(r##"Fml("GM7(91) - True Strength Index") < .4 AND Ref(Fml("GM7(91) - True Strength Index"),-1) > .4"##), None], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "msws_gm7_variable_ma_crossover", name: r##"GM7(91) - Variable MA Crossover (Jun., p. 18)"##, group: "GM7 · S&C 1990-91", family: ("msws_gm7", "Morris GM7 S&C 1990-91 systems"), page: "40", status: Live, f: [Some(r##"Mov(C,3,S)>Fml("GM7(91) - Variable Length Average")"##), Some(r##"Mov(C,3,S)<Fml("GM7(91) - Variable Length Average")"##), Some(r##"Mov(C,3,S)<Fml("GM7(91) - Variable Length Average")"##), Some(r##"Mov(C,3,S)>Fml("GM7(91) - Variable Length Average")"##)], exec: EXEC_DEFAULT, notes: r##"Stdev(C,10) thresholds 1 and 1.5 are price points; applied literally to the index."## },
    Sys { id: "msws_gm8_2_model", name: r##"GM8(92) - 2% Model (Mar., p. 70)"##, group: "GM8 · S&C 1992-93", family: ("msws_gm8", "Morris GM8 S&C 1992-93 systems"), page: "41", status: NotTestable("no Close Long / Close Short formula is printed (Enter Long and Enter Short only); the source does not say the opposite entry closes the position"), f: [Some(r##"C > (Ref((HHV(C,5)*1.02),-1))"##), None, Some(r##"C < (Ref((LLV(C,5)*.98),-1))"##), None], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "msws_gm8_4_model", name: r##"GM8(92) - 4% Model (Mar., p. 70)"##, group: "GM8 · S&C 1992-93", family: ("msws_gm8", "Morris GM8 S&C 1992-93 systems"), page: "41", status: NotTestable("no Close Long / Close Short formula is printed (Enter Long and Enter Short only); the source does not say the opposite entry closes the position"), f: [Some(r##"C > (Ref((HHV(C,5)*1.04),-1))"##), None, Some(r##"C < (Ref((LLV(C,5)*.96),-1))"##), None], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "msws_gm8_dm_trading_model", name: r##"GM8(92) - DM Trading Model (May, p. 16)"##, group: "GM8 · S&C 1992-93", family: ("msws_gm8", "Morris GM8 S&C 1992-93 systems"), page: "41", status: NotTestable("no Close Long / Close Short formula is printed (Enter Long and Enter Short only); the source does not say the opposite entry closes the position"), f: [Some(r##"C > Mov(C,20,S)*1.005 AND Mov(C,10,E) > Ref(Mov(C,10,E),-1) AND C > Mov(C,7,W)*1.005 AND Mov(ROC(Mov(C,5,E),35,%),45,S) > Ref(Mov(ROC(Mov(C,5,E),35,%),45,S),-1) AND Mov(ROC(C,30,%),25,W) > Ref(Mov(ROC(C,30,%),25,W),-1)"##), None, Some(r##"C < Mov(C,20,S)*.995 AND Mov(C,10,E) < Ref(Mov(C,10,E),-1) AND C < Mov(C,7,W)*.995 AND Mov(ROC(Mov(C,5,E),35,%),45,S) < Ref(Mov(ROC(Mov(C,5,E),35,%),45,S),-1) AND Mov(ROC(C,30,%),25,W) < Ref(Mov(ROC(C,30,%),25,W),-1)"##), None], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "msws_gm8_mass_index_system", name: r##"GM8(92) - Mass Index System (Jun., p. 67)"##, group: "GM8 · S&C 1992-93", family: ("msws_gm8", "Morris GM8 S&C 1992-93 systems"), page: "41", status: NotTestable("no Close Long / Close Short formula is printed (Enter Long and Enter Short only); the source does not say the opposite entry closes the position"), f: [Some(r##"Fml("GM8(92) - Mass Index") < 26.5 AND Ref(Fml("GM8(92) - Mass Index"),-1) > 26.5 AND (Mov(C,9,E) < Mov(Mov(C,9,E),9,E))"##), None, Some(r##"Fml("GM8(92) - Mass Index") < 26.5 AND Ref(Fml("GM8(92) - Mass Index"),-1) > 26.5 AND (Mov(C,9,E) > Mov(Mov(C,9,E),9,E))"##), None], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "msws_gm8_pring_s_kst_crossover", name: r##"GM8(92) - Pring's KST Crossover (Feb., p. 32)"##, group: "GM8 · S&C 1992-93", family: ("msws_gm8", "Morris GM8 S&C 1992-93 systems"), page: "41", status: NotTestable("no Close Long / Close Short formula is printed (Enter Long and Enter Short only); the source does not say the opposite entry closes the position"), f: [Some(r##"(Fml("GM8(92) - Pring's Short KST") > Mov((Fml("GM8(92) - Pring's Short KST")),15,S))"##), None, Some(r##"(Fml("GM8(92) - Pring's Short KST") < Mov((Fml("GM8(92) - Pring's Short KST")),15,S))"##), None], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "msws_gm8_rmi_2_system", name: r##"GM8(92) - RMI#2 System"##, group: "GM8 · S&C 1992-93", family: ("msws_gm8", "Morris GM8 S&C 1992-93 systems"), page: "42", status: NotTestable("no Close Long / Close Short formula is printed (Enter Long and Enter Short only); the source does not say the opposite entry closes the position"), f: [Some(r##"Fml("GM8(92) - RMI Derivation #2") > 20 AND Ref(Fml("GM8(92) - RMI Derivation #2"),-1) < 20"##), None, Some(r##"Fml("GM8(92) - RMI Derivation #2") < 80 AND Ref(Fml("GM8(92) - RMI Derivation #2"),-1) > 80"##), None], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "msws_gm8_tai_system", name: r##"GM8(92) - TAI System (Aug., p. 75)"##, group: "GM8 · S&C 1992-93", family: ("msws_gm8", "Morris GM8 S&C 1992-93 systems"), page: "42", status: NotTestable("no Close Long / Close Short formula is printed (Enter Long and Enter Short only); the source does not say the opposite entry closes the position"), f: [Some(r##"(Fml("GM8(92) - Fast TAI") > Fml("GM8(92) - Slow TAI")) AND (C > Mov(C,26,S))"##), None, Some(r##"(Fml("GM8(92) - Fast TAI") < Fml("GM8(92) - Slow TAI")) AND (C < Mov(C,26,S))"##), None], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "msws_gm8_trix_mo_system", name: r##"GM8(92) - Trix MO System (Dec., p. 78)"##, group: "GM8 · S&C 1992-93", family: ("msws_gm8", "Morris GM8 S&C 1992-93 systems"), page: "42", status: NotTestable("no Close Long / Close Short formula is printed (Enter Long and Enter Short only); the source does not say the opposite entry closes the position"), f: [Some(r##"(TRIX(12) - Ref(TRIX(12),-1) < 0) AND (Mov(TRIX(12) - Ref(TRIX(12),-1),3,S) < TRIX(12) - Ref(TRIX(12),-1))"##), None, Some(r##"(TRIX(12) - Ref(TRIX(12),-1) > 0) AND (Mov(TRIX(12) - Ref(TRIX(12),-1),3,S) < TRIX(12) - Ref(TRIX(12),-1))"##), None], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "msws_gm8_trix_system", name: r##"GM8(92) - Trix System (Dec., p. 78)"##, group: "GM8 · S&C 1992-93", family: ("msws_gm8", "Morris GM8 S&C 1992-93 systems"), page: "42", status: NotTestable("no Close Long / Close Short formula is printed (Enter Long and Enter Short only); the source does not say the opposite entry closes the position"), f: [Some(r##"(Ref(TRIX(12),-1) < Mov(TRIX(12),3,E)) AND (TRIX(12) >= Mov(TRIX(12),3,E))"##), None, Some(r##"(Ref(TRIX(12),-1) > Mov(TRIX(12),3,E)) AND (TRIX(12) <= Mov(TRIX(12),3,E))"##), None], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "msws_gm8_two_stochastics_system", name: r##"GM8(92) - Two Stochastics System (Dec., p. 26)"##, group: "GM8 · S&C 1992-93", family: ("msws_gm8", "Morris GM8 S&C 1992-93 systems"), page: "42", status: NotTestable("no Close Long / Close Short formula is printed (Enter Long and Enter Short only); the source does not say the opposite entry closes the position"), f: [Some(r##"Fml("GM8(92) - Two Stochastics") > -1.9 AND Ref(Fml("GM8(92) - Two Stochastics"),-1) < -1.9"##), None, Some(r##"Fml("GM8(92) - Two Stochastics") < 1.9 AND Ref(Fml("GM8(92) - Two Stochastics"),-1) > 1.9"##), None], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "msws_gm8_vol_rsi_system", name: r##"GM8(92) - Vol RSI System (Oct., p. 59)"##, group: "GM8 · S&C 1992-93", family: ("msws_gm8", "Morris GM8 S&C 1992-93 systems"), page: "43", status: NotTestable("no Close Long / Close Short formula is printed (Enter Long and Enter Short only); the source does not say the opposite entry closes the position"), f: [Some(r##"Fml("GM8(92) - Volume Weighted RSI") > 20 AND Ref(Fml("GM8(92) - Volume Weighted RSI"),-1) < 20"##), None, Some(r##"Fml("GM8(92) - Volume Weighted RSI") < 80 AND Ref(Fml("GM8(92) - Volume Weighted RSI"),-1) > 80"##), None], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "msws_gm8_coppock_system", name: r##"GM8(93) - Coppock System (Mar., p. 50)"##, group: "GM8 · S&C 1992-93", family: ("msws_gm8", "Morris GM8 S&C 1992-93 systems"), page: "43", status: NotTestable("no Close Long / Close Short formula is printed (Enter Long and Enter Short only); the source does not say the opposite entry closes the position"), f: [Some(r##"Fml("GM8(93) - Coppock Guide") > 0 AND Ref(Fml("GM8(93) - Coppock Guide"),-1) < 0"##), None, Some(r##"Fml("GM8(93) - Coppock Guide") < 0 AND Ref(Fml("GM8(93) - Coppock Guide"),-1) > 0"##), None], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "msws_gm8_dyr_implied_risk_trader", name: r##"GM8(93) - DYR Implied Risk Trader (Jun., p. 29)"##, group: "GM8 · S&C 1992-93", family: ("msws_gm8", "Morris GM8 S&C 1992-93 systems"), page: "43", status: NotTestable("no Close Long / Close Short formula is printed (Enter Long and Enter Short only); the source does not say the opposite entry closes the position"), f: [Some(r##"Fml("GM8(93) - DYR Implied Risk") > -300 AND Ref(Fml("GM8(93) - DYR Implied Risk"),-1) < -30"##), None, Some(r##"Fml("GM8(93) - DYR Implied Risk") < 30 AND Ref(Fml("GM8(93) - DYR Implied Risk"),-1) > 30"##), None], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "msws_gm8_offset_mo_system", name: r##"GM8(93) - Offset Mo System (Apr., p. 54)"##, group: "GM8 · S&C 1992-93", family: ("msws_gm8", "Morris GM8 S&C 1992-93 systems"), page: "43", status: NotTestable("no Close Long / Close Short formula is printed (Enter Long and Enter Short only); the source does not say the opposite entry closes the position; Fml(\"GM8(93) - Offset Momentum Oscillator\") is also not defined in the formula file"), f: [Some(r##"Cross(Fml("GM8(93) - Offset Momentum Oscillator"), Fml("GM8(93) - Smoothed Offset Momentum"))"##), None, Some(r##"Cross(Fml("GM8(93) - Smoothed Offset Momentum"), Fml("GM8(93) - Offset Momentum Oscillator"))"##), None], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "msws_gm8_raff_obv_system", name: r##"GM8(93) - Raff OBV System (Feb., p. 75)"##, group: "GM8 · S&C 1992-93", family: ("msws_gm8", "Morris GM8 S&C 1992-93 systems"), page: "43", status: Live, f: [Some(r##"(Mov(OBV(),2,E) > Mov(OBV(),20,E)) AND (Mov(OBV(),20,E) > Mov(OBV(),50,E))"##), Some(r##"(Mov(OBV(),2,E) < Mov(OBV(),20,E)) OR (Mov(OBV(),20,E) < Mov(OBV(),50,E))"##), Some(r##"(Mov(OBV(),2,E) < Mov(OBV(),20,E)) AND (Mov(OBV(),20,E) < Mov(OBV(),50,E))"##), Some(r##"(Mov(OBV(),2,E) > Mov(OBV(),20,E)) OR (Mov(OBV(),20,E) > Mov(OBV(),50,E))"##)], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "msws_gm9_hybrid_system", name: r##"GM9(94) - Hybrid System"##, group: "GM9 · S&C 1994-95", family: ("msws_gm9", "Morris GM9 S&C 1994-95 systems"), page: "44", status: NotTestable("no Close Long / Close Short formula is printed (Enter Long and Enter Short only); the source does not say the opposite entry closes the position"), f: [Some(r##"Fml("GM9(94) - Hybrid Indicator") > 30 AND Ref(Fml("GM9(94) - Hybrid Indicator"),-1) < 30"##), None, Some(r##"Fml("GM9(94) - Hybrid Indicator") < 70 AND Ref(Fml("GM9(94) - Hybrid Indicator"),-1) > 70"##), None], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "msws_gm9_pfe_system", name: r##"GM9(94) - PFE System"##, group: "GM9 · S&C 1994-95", family: ("msws_gm9", "Morris GM9 S&C 1994-95 systems"), page: "44", status: NotTestable("no Close Long / Close Short formula is printed (Enter Long and Enter Short only); the source does not say the opposite entry closes the position"), f: [Some(r##"Fml("GM9(94) - Polarized Fractal Efficiency") > -80 AND Ref(Fml("GM9(94) - Polarized Fractal Efficiency"),-1)< -80"##), None, Some(r##"Fml("GM9(94) - Polarized Fractal Efficiency") < 80 AND Ref(Fml("GM9(94) - Polarized Fractal Efficiency"),-1)> 80"##), None], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "msws_gm9_roc_system", name: r##"GM9(94) - ROC System"##, group: "GM9 · S&C 1994-95", family: ("msws_gm9", "Morris GM9 S&C 1994-95 systems"), page: "44", status: NotTestable("no Close Long / Close Short formula is printed (Enter Long and Enter Short only); the source does not say the opposite entry closes the position"), f: [Some(r##"ROC(C,10,%)>0 AND Ref(ROC(C,10,%),-1)<0"##), None, Some(r##"ROC(C,10,%)<0 AND Ref(ROC(C,10,%),-1)>0"##), None], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "msws_gm9_bottom_fisher_system", name: r##"GM9(95) - Bottom Fisher System"##, group: "GM9 · S&C 1994-95", family: ("msws_gm9", "Morris GM9 S&C 1994-95 systems"), page: "44", status: NotTestable("no Close Long / Close Short formula is printed (Enter Long and Enter Short only); the source does not say the opposite entry closes the position"), f: [Some(r##"(C>(O-4)) AND ((H-L)>3) AND (LLV(L,5)=LLV(L,20))"##), None, Some(r##"(C<(O+4)) AND ((H-L)>3) AND (HHV(H,5)=HHV(H,20))"##), None], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "msws_gm9_cci_zerocrossover", name: r##"GM9(95) - CCI ZeroCrossover"##, group: "GM9 · S&C 1994-95", family: ("msws_gm9", "Morris GM9 S&C 1994-95 systems"), page: "44", status: NotTestable("no Close Long / Close Short formula is printed (Enter Long and Enter Short only); the source does not say the opposite entry closes the position"), f: [Some(r##"Fml("GM9(95) - CCI-5 day smooth")>0"##), None, Some(r##"Fml("GM9(95) - CCI-5 day smooth")<0"##), None], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "msws_gm9_djroc_system", name: r##"GM9(95) - DJROC System"##, group: "GM9 · S&C 1994-95", family: ("msws_gm9", "Morris GM9 S&C 1994-95 systems"), page: "45", status: NotTestable("no Close Long / Close Short formula is printed (Enter Long and Enter Short only); the source does not say the opposite entry closes the position"), f: [Some(r##"Fml("GM9(95) - DJROC")<-1 AND Fml("GM9(95) - DJROC")>Ref(Fml("GM9(95) - DJROC"),-1)"##), None, Some(r##"BarsSince((Fml("GM9(95) - DJROC")<-1 AND Fml("GM9(95) - DJROC")>Ref(Fml("GM9(95) - DJROC"),-1)))=12"##), None], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "msws_gm9_historical_volatility_system", name: r##"GM9(95) - Historical Volatility System"##, group: "GM9 · S&C 1994-95", family: ("msws_gm9", "Morris GM9 S&C 1994-95 systems"), page: "45", status: NotTestable("no Close Long / Close Short formula is printed (Enter Long and Enter Short only); the source does not say the opposite entry closes the position"), f: [Some(r##"Fml("GM9(95) - Historical Volatility Indicator")<=.5 AND C>Mov(C,20,E)"##), None, Some(r##"Fml("GM9(95) - Historical Volatility Indicator")<=.5 AND C<Mov(C,20,E)"##), None], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "msws_gm9_modified_vix_system", name: r##"GM9(95) - Modified VIX System"##, group: "GM9 · S&C 1994-95", family: ("msws_gm9", "Morris GM9 S&C 1994-95 systems"), page: "45", status: NotTestable("no Close Long / Close Short formula is printed (Enter Long and Enter Short only); the source does not say the opposite entry closes the position"), f: [Some(r##"Fml("GM9(95) - Modified VIX") > -7 AND Ref(Fml("GM9(95) - Modified VIX"),-1) < -7"##), None, Some(r##"Fml("GM9(95) - Modified VIX") < 7 AND Ref(Fml("GM9(95) - Modified VIX"),-1) > 7"##), None], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "msws_gm9_range_breakout", name: r##"GM9(95) - Range Breakout"##, group: "GM9 · S&C 1994-95", family: ("msws_gm9", "Morris GM9 S&C 1994-95 systems"), page: "45", status: NotTestable("Fml(\"GM9(95) - Tight & Narrow - High/Low\") calls Peak(1, ..., 1), a zig-zag function whose last swing is revised by later bars (look-ahead), so the rule cannot be applied bar by bar"), f: [Some(r##"Cross(C,Fml("GM9(95) - Tight & Narrow - High")) AND H-L > 1.25 * Mov(H-L,8,S)"##), Some(r##"Cross(Mov(L,10,S),C)"##), Some(r##"Cross(Fml("GM9(95) - Tight & Narrow - Low"),C) AND H-L > 1.25 * Mov(H-L,8,S)"##), Some(r##"Cross(C,Mov(L,10,S))"##)], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "msws_gm9_ud_price_change", name: r##"GM9(95) - UD% Price Change"##, group: "GM9 · S&C 1994-95", family: ("msws_gm9", "Morris GM9 S&C 1994-95 systems"), page: "45", status: NotTestable("no Close Long / Close Short formula is printed (Enter Long and Enter Short only); the source does not say the opposite entry closes the position"), f: [Some(r##"C > (LLV(L,15)*1.03)"##), None, Some(r##"C < (HHV(L,15)*.97)"##), None], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "msws_gm9_ud_utility_system", name: r##"GM9(95) - UD% Utility System"##, group: "GM9 · S&C 1994-95", family: ("msws_gm9", "Morris GM9 S&C 1994-95 systems"), page: "46", status: NotTestable("no Close Long / Close Short formula is printed (Enter Long and Enter Short only); the source does not say the opposite entry closes the position"), f: [Some(r##"C > (LLV(L,15)*1.05)"##), None, Some(r##"C < (HHV(L,15)*.86)"##), None], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "msws_gm9_vol_mo_system", name: r##"GM9(95) - Vol. Mo. System"##, group: "GM9 · S&C 1994-95", family: ("msws_gm9", "Morris GM9 S&C 1994-95 systems"), page: "46", status: NotTestable("no Close Long / Close Short formula is printed (Enter Long and Enter Short only); the source does not say the opposite entry closes the position"), f: [Some(r##"Fml("GM9(95) - Volume Momentum") > Mov(Fml("GM9(95) - Volume Momentum"),5,S)"##), None, Some(r##"Fml("GM9(95) - Volume Momentum") < Mov(Fml("GM9(95) - Volume Momentum"),5,S)"##), None], exec: EXEC_DEFAULT, notes: r##""## },
    Sys { id: "msws_gm9_weekly_sp_system", name: r##"GM9(95) - Weekly S&P System"##, group: "GM9 · S&C 1994-95", family: ("msws_gm9", "Morris GM9 S&C 1994-95 systems"), page: "46", status: NotTestable("no Close Long / Close Short formula is printed (Enter Long and Enter Short only); the source does not say the opposite entry closes the position"), f: [Some(r##"(LLV(L,5)>LLV(L,13)) AND (H=HHV(H,5))"##), None, Some(r##"(((HHV(L,13)-L)/L)>.05) AND (((HHV(Mov(C,26,S),5)-LLV(Mov(C,26,S),5))/C*100) < 1.2)"##), None], exec: EXEC_DEFAULT, notes: r##""## },
];

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parser_precedence_and_chains() {
        let p = parse("1+2*3>6 AND 0 OR 1").unwrap();
        // ((1+(2*3))>6 AND 0) OR 1
        assert!(matches!(p.value, E::B(0, _, _)));
        let p = parse("a:=2; a*3").unwrap();
        assert_eq!(p.defs.len(), 1);
        assert!(parse("Ref(C,-1)>RSI(14)>70").is_ok());
    }
    #[test]
    fn ema_seed_and_window() {
        let v = vec![1.0, 2.0, 3.0, 4.0];
        let e = ema(&v, 3);
        assert!(e[1].is_nan());
        // seed 1, alpha 0.5: 1.5, 2.25, 3.125
        assert!((e[2] - 2.25).abs() < 1e-12 && (e[3] - 3.125).abs() < 1e-12);
        let w = wild(&[NAN, 2.0, 4.0, 6.0], 2);
        assert!((w[2] - 3.0).abs() < 1e-12 && (w[3] - 4.5).abs() < 1e-12);
    }
    fn rows(n: usize) -> Vec<crate::Row> {
        (0..n).map(|i| { let t = i as f64; let c = 1000. + 80. * (t / 23.).sin() + 40. * (t / 7.).cos() + t * 0.3;
            crate::Row { ticker: "T".into(), date: format!("{:04}-{:02}-{:02}", 2000 + i / 300, 1 + (i / 25) % 12, 1 + i % 25), open: c - 3. * (t / 3.).sin(), high: c + 6. + 2. * (t / 5.).sin().abs(), low: c - 6. - 2. * (t / 4.).cos().abs(), close: c, volume: Some(1e6 + 3e5 * (t / 9.).sin()) } }).collect()
    }
    #[test]
    fn every_live_system_evaluates_and_runs() {
        let r = rows(1500);
        let x = Ctx::new(&r, &r, &[]);
        let bad = MSWS.check_all(&x);
        assert!(bad.is_empty(), "{bad:?}");
        for s in SYS.iter() {
            let res = run(s.id, &x, 300, 1499, &Params::new());
            assert_eq!(res.is_some(), s.status == Status::Live, "{}", s.id);
            for t in res.unwrap_or_default() { assert!(t.legs[0].i >= 300 && t.exit_i <= 1499 && t.exit_i >= t.legs[0].i, "{}", s.id); }
        }
    }
    #[test]
    fn rsi_crossover_trades_on_closes() {
        // RSI(14) 30/70 crossover: long entry on the bar RSI crosses up through 30, filled at that close
        let r = rows(1500);
        let x = Ctx::new(&r, &r, &[]);
        let t = run("msws_rsi_30_70_crossover", &x, 300, 1499, &Params::new()).unwrap();
        let rs = rsi(&x.c, 14);
        for tr in t.iter().filter(|t| t.dir > 0.) { let i = tr.legs[0].i; assert!(rs[i - 1] < 30. && rs[i] > 30.); assert_eq!(tr.legs[0].px, x.c[i]); }
    }
}
