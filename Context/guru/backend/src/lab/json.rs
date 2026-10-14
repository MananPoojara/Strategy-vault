//! Minimal JSON reader for lab rule specs (std only). Objects keep key order; duplicate keys, non-finite
//! numbers, nesting deeper than MAX_DEPTH and trailing text are errors. Writers use `esc` for every user string.
#[derive(Clone, Debug, PartialEq)]
pub enum Json { Null, Bool(bool), Num(f64), Str(String), Arr(Vec<Json>), Obj(Vec<(String, Json)>) }

const MAX_DEPTH: usize = 16;

impl Json {
    pub fn get(&self, key: &str) -> Option<&Json> {
        match self { Json::Obj(kv) => kv.iter().find(|(k, _)| k == key).map(|(_, v)| v), _ => None }
    }
    pub fn as_f64(&self) -> Option<f64> { if let Json::Num(n) = self { Some(*n) } else { None } }
    pub fn as_str(&self) -> Option<&str> { if let Json::Str(s) = self { Some(s) } else { None } }
}

pub fn parse(text: &str) -> Result<Json, String> {
    let mut p = Parser { b: text.as_bytes(), i: 0 };
    let v = p.value(0)?;
    p.ws();
    if p.i != p.b.len() { return Err(format!("unexpected text at byte {}", p.i)); }
    Ok(v)
}

/// JSON string body escaping (quotes, backslash and every control character).
pub fn esc(s: &str) -> String {
    let mut o = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            c if (c as u32) < 0x20 => o.push_str(&format!("\\u{:04x}", c as u32)),
            c => o.push(c),
        }
    }
    o
}

struct Parser<'a> { b: &'a [u8], i: usize }
impl Parser<'_> {
    fn ws(&mut self) { while self.i < self.b.len() && matches!(self.b[self.i], b' ' | b'\t' | b'\n' | b'\r') { self.i += 1; } }
    fn err<T>(&self, what: &str) -> Result<T, String> { Err(format!("{what} at byte {}", self.i)) }
    fn lit(&mut self, word: &str, v: Json) -> Result<Json, String> {
        if self.b[self.i..].starts_with(word.as_bytes()) { self.i += word.len(); Ok(v) } else { self.err("invalid literal") }
    }
    fn value(&mut self, depth: usize) -> Result<Json, String> {
        if depth >= MAX_DEPTH { return self.err("nesting too deep"); }
        self.ws();
        match self.b.get(self.i) {
            None => self.err("unexpected end"),
            Some(b'{') => {
                self.i += 1;
                let mut kv: Vec<(String, Json)> = Vec::new();
                self.ws();
                if self.b.get(self.i) == Some(&b'}') { self.i += 1; return Ok(Json::Obj(kv)); }
                loop {
                    self.ws();
                    if self.b.get(self.i) != Some(&b'"') { return self.err("expected a quoted key"); }
                    let k = self.string()?;
                    if kv.iter().any(|(x, _)| *x == k) {
                        let shown: String = k.chars().take(64).collect();
                        return Err(format!("duplicate key '{shown}' at byte {}", self.i));
                    }
                    self.ws();
                    if self.b.get(self.i) != Some(&b':') { return self.err("expected ':'"); }
                    self.i += 1;
                    let v = self.value(depth + 1)?;
                    kv.push((k, v));
                    self.ws();
                    match self.b.get(self.i) {
                        Some(b',') => self.i += 1,
                        Some(b'}') => { self.i += 1; return Ok(Json::Obj(kv)); }
                        _ => return self.err("expected ',' or '}'"),
                    }
                }
            }
            Some(b'[') => {
                self.i += 1;
                let mut a = Vec::new();
                self.ws();
                if self.b.get(self.i) == Some(&b']') { self.i += 1; return Ok(Json::Arr(a)); }
                loop {
                    a.push(self.value(depth + 1)?);
                    self.ws();
                    match self.b.get(self.i) {
                        Some(b',') => self.i += 1,
                        Some(b']') => { self.i += 1; return Ok(Json::Arr(a)); }
                        _ => return self.err("expected ',' or ']'"),
                    }
                }
            }
            Some(b'"') => Ok(Json::Str(self.string()?)),
            Some(b't') => self.lit("true", Json::Bool(true)),
            Some(b'f') => self.lit("false", Json::Bool(false)),
            Some(b'n') => self.lit("null", Json::Null),
            Some(c) if *c == b'-' || c.is_ascii_digit() => self.number(),
            Some(_) => self.err("unexpected character"),
        }
    }
    /// Strict JSON grammar: -? (0 | [1-9][0-9]*) (\.[0-9]+)? ([eE][+-]?[0-9]+)?, then finite f64.
    fn number(&mut self) -> Result<Json, String> {
        let start = self.i;
        let digits = |p: &mut Self| { let s = p.i; while p.b.get(p.i).is_some_and(u8::is_ascii_digit) { p.i += 1; } p.i > s };
        if self.b.get(self.i) == Some(&b'-') { self.i += 1; }
        match self.b.get(self.i) {
            Some(b'0') => self.i += 1,
            Some(b'1'..=b'9') => { digits(self); }
            _ => return self.err("invalid number"),
        }
        if self.b.get(self.i) == Some(&b'.') {
            self.i += 1;
            if !digits(self) { return self.err("invalid number"); }
        }
        if matches!(self.b.get(self.i), Some(b'e' | b'E')) {
            self.i += 1;
            if matches!(self.b.get(self.i), Some(b'+' | b'-')) { self.i += 1; }
            if !digits(self) { return self.err("invalid number"); }
        }
        let s = std::str::from_utf8(&self.b[start..self.i]).unwrap_or("");
        match s.parse::<f64>() {
            Ok(n) if n.is_finite() => Ok(Json::Num(n)),
            _ => Err(format!("number out of range '{s}' at byte {start}")),
        }
    }
    fn string(&mut self) -> Result<String, String> {
        self.i += 1; // opening quote
        let mut out: Vec<u8> = Vec::new();
        loop {
            let Some(&c) = self.b.get(self.i) else { return self.err("unterminated string") };
            self.i += 1;
            match c {
                b'"' => return String::from_utf8(out).map_err(|_| "string is not UTF-8".to_string()),
                b'\\' => {
                    let Some(&e) = self.b.get(self.i) else { return self.err("unterminated escape") };
                    self.i += 1;
                    match e {
                        b'"' => out.push(b'"'), b'\\' => out.push(b'\\'), b'/' => out.push(b'/'),
                        b'b' => out.push(8), b'f' => out.push(12), b'n' => out.push(b'\n'), b'r' => out.push(b'\r'), b't' => out.push(b'\t'),
                        b'u' => {
                            let hex = self.b.get(self.i..self.i + 4).filter(|h| h.iter().all(u8::is_ascii_hexdigit))
                                .and_then(|h| std::str::from_utf8(h).ok()).and_then(|h| u32::from_str_radix(h, 16).ok());
                            // shortcut: surrogate pairs (non-BMP \u escapes) are rejected; spec ids and keys are ASCII.
                            let Some(ch) = hex.and_then(char::from_u32) else { return self.err("unsupported \\u escape") };
                            self.i += 4;
                            out.extend_from_slice(ch.to_string().as_bytes());
                        }
                        _ => return self.err("invalid escape"),
                    }
                }
                c if c < 0x20 => return self.err("control character in string"),
                c => out.push(c),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn parses_spec_shapes_in_order() {
        let v = parse(r#" {"v":1,"a":[true,false,null,-2.5e1,"x\"y\u00e9"],"o":{}} "#).unwrap();
        assert_eq!(v.get("v").and_then(Json::as_f64), Some(1.));
        assert_eq!(v.get("a"), Some(&Json::Arr(vec![Json::Bool(true), Json::Bool(false), Json::Null, Json::Num(-25.), Json::Str("x\"yé".into())])));
        assert_eq!(v.get("o"), Some(&Json::Obj(vec![])));
        if let Json::Obj(kv) = &v { assert_eq!(kv.iter().map(|(k, _)| k.as_str()).collect::<Vec<_>>(), ["v", "a", "o"]); }
    }
    #[test] fn rejects_malformed_input_without_panicking() {
        for bad in ["", "{", "[1,]", "{\"a\":1,}", "{\"a\" 1}", "{\"a\":1}x", "\"abc", "{\"a\":1,\"a\":2}", "1e999", "-", "nul", "[\"\\ud800\"]", "\"a\u{1}\"", "{a:1}", "\"\\x\"",
            "01", "00", "-01", "1.", "-.5", "1.e2", "+1", "1e", "1e+", "\"\\u+123\"", "\"\\u 123\""] {
            assert!(parse(bad).is_err(), "{bad:?} should fail");
        }
        for good in ["0", "-0", "1.5", "1e5", "1E+5", "-2.5e-3"] {
            assert!(matches!(parse(good), Ok(Json::Num(_))), "{good:?} should parse");
        }
        let long = format!("{{\"{0}\":1,\"{0}\":2}}", "k".repeat(200));
        assert!(parse(&long).unwrap_err().len() < 100, "duplicate-key error must cap the echoed key");
    }
    #[test] fn depth_is_bounded() {
        assert!(parse(&"[".repeat(17).chars().chain("]".repeat(17).chars()).collect::<String>()).is_err());
        assert!(parse(&format!("{}{}", "[".repeat(16), "]".repeat(16))).is_ok());
        assert!(parse(&"[".repeat(100_000)).is_err());
    }
    #[test] fn esc_escapes_controls() { assert_eq!(esc("a\"b\\c\nd"), "a\\\"b\\\\c\\u000ad"); }
}
