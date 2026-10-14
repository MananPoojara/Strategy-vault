pub fn allowed_engine_path(path: &str) -> bool {
    matches!(
        path,
        "tickers"
            | "ohlc"
            | "gurus"
            | "guru/connors/strategies"
            | "guru/connors/summary"
            | "guru/connors/readiness"
            | "guru/connors/run"
            | "guru/connors/params"
            | "guru/connors/optimize"
            | "guru/ross/strategies"
            | "guru/ross/summary"
            | "guru/ross/readiness"
            | "guru/ross/run"
            | "guru/ross/params"
            | "guru/ross/optimize"
            | "guru/chande/strategies"
            | "guru/chande/summary"
            | "guru/chande/readiness"
            | "guru/chande/run"
            | "guru/chande/params"
            | "guru/chande/optimize"
            | "lab/catalogue"
            | "lab/series"
            | "lab/run"
            | "lab/optimize"
            | "lab/instruments"
    )
}
pub fn is_optimization_path(path:&str)->bool {
    matches!(path,"guru/connors/optimize"|"guru/ross/optimize"|"guru/chande/optimize")
}
pub fn can_optimize(role: &str) -> bool {
    matches!(role, "admin" | "researcher")
}
pub fn valid_query(query: Option<&str>) -> bool {
    query.map_or(true, |q| {
        q.len() <= 16384 && !q.chars().any(char::is_control)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn engine_routes_are_allowlisted_and_read_only() {
        assert!(allowed_engine_path("tickers"));
        assert!(allowed_engine_path("ohlc"));
        assert!(allowed_engine_path("guru/connors/run"));
        assert!(allowed_engine_path("guru/connors/readiness"));
        assert!(allowed_engine_path("guru/connors/optimize"));
        for route in ["strategies","summary","readiness","run","params","optimize"] {
            assert!(allowed_engine_path(&format!("guru/ross/{route}")));
            assert!(allowed_engine_path(&format!("guru/chande/{route}")));
        }
        for owner in ["connors","ross","chande"] {assert!(is_optimization_path(&format!("guru/{owner}/optimize")));}
        assert!(!is_optimization_path("guru/other/optimize"));
        for path in [
            "../notes",
            "api/auth/sign-up/email",
            "unknown",
            "guru/other/run",
            "notes",
        ] {
            assert!(!allowed_engine_path(path));
        }
    }
    #[test]
    fn only_research_roles_can_optimize() {
        assert!(can_optimize("admin"));
        assert!(can_optimize("researcher"));
        assert!(!can_optimize("viewer"));
        assert!(!can_optimize(""));
    }
    #[test]
    fn api_queries_are_bounded() {
        assert!(valid_query(Some("id=f1a&ticker=NIFTY+50")));
        assert!(valid_query(None));
        assert!(!valid_query(Some(&"a".repeat(16385))));
        assert!(!valid_query(Some("id=f1a\r\n")));
    }
}

#[cfg(test)]
mod numeric_tests {
    use super::*;
    use std::collections::BTreeMap;
    fn params(items: &[(&str, &str)]) -> BTreeMap<String, String> {
        items
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }
    #[test]
    fn numerical_inputs_cannot_overflow_or_expand_the_grid_without_bounds() {
        assert!(safe_engine_numbers(&params(&[
            ("g.hold", "2:10:1"),
            ("p.trend", "200"),
            ("cost", "0.001")
        ])));
        assert!(safe_engine_numbers(&params(&[("g.threshold", "-2:2:0.1")])));
        for pair in [
            ("p.trend", "NaN"),
            ("g.hold", "0:inf:1"),
            ("g.hold", "1:2:0"),
            ("g.hold", "1:1000000:1"),
            ("cost", "inf"),
        ] {
            assert!(!safe_engine_numbers(&params(&[pair])));
        }
        assert!(!safe_engine_numbers(&params(&[
            ("g.one", "1:1000:1"),
            ("g.two", "1:1000:1")
        ])));
    }
}

pub fn safe_engine_numbers(params: &std::collections::BTreeMap<String, String>) -> bool {
    let mut combinations = 1usize;
    for (key, value) in params {
        if key.starts_with("p.") || key == "cost" {
            if !value
                .parse::<f64>()
                .is_ok_and(|n| n.is_finite() && n.abs() <= 1_000_000.0)
            {
                return false;
            }
        }
        if key.starts_with("g.") {
            let numbers = match value
                .split(':')
                .map(|v| v.trim().parse::<f64>())
                .collect::<Result<Vec<_>, _>>()
            {
                Ok(n) => n,
                Err(_) => return false,
            };
            if numbers
                .iter()
                .any(|n| !n.is_finite() || n.abs() > 1_000_000.0)
            {
                return false;
            }
            let count = match numbers.as_slice() {
                [_] => 1.0,
                [lo, hi, step] if *step > 0.0 && hi >= lo => {
                    ((hi - lo) / step + 1e-9).floor() + 1.0
                }
                _ => return false,
            };
            if !count.is_finite() || !(1.0..=1000.0).contains(&count) {
                return false;
            }
            combinations = match combinations.checked_mul(count as usize) {
                Some(n) if n <= 50_000 => n,
                _ => return false,
            };
        }
    }
    true
}
/// Indicators lab: one optimization at a time and research roles only, like the book optimizers.
pub fn is_lab_optimization_path(path: &str) -> bool {
    path == "lab/optimize"
}
pub const LAB_SPEC_MAX_BYTES: usize = 8192;
pub const LAB_MAX_CONDITIONS: usize = 12;
/// Bounds a lab rule spec before it reaches the engine (the engine re-validates every field).
pub fn lab_spec_problem(path: &str, params: &std::collections::BTreeMap<String, String>) -> Option<&'static str> {
    if !matches!(path, "lab/run" | "lab/optimize") {
        return None;
    }
    let Some(spec) = params.get("spec") else { return Some("A lab rule spec is required.") };
    if spec.len() > LAB_SPEC_MAX_BYTES {
        return Some("The rule spec is larger than 8 KB.");
    }
    let Ok(value) = serde_json::from_str::<serde_json::Value>(spec) else { return Some("The rule spec is not valid JSON.") };
    let count = |list: &str, key: &str| value.get(list).and_then(|v| v.get(key)).and_then(|v| v.as_array()).map_or(0, Vec::len);
    if count("entry", "all") + count("exit", "any") > LAB_MAX_CONDITIONS {
        return Some("A rule spec allows at most 12 conditions.");
    }
    fn numbers_ok(v: &serde_json::Value) -> bool {
        match v {
            serde_json::Value::Number(n) => n.as_f64().is_some_and(|x| x.is_finite() && x.abs() <= 1e12),
            serde_json::Value::Array(a) => a.iter().all(numbers_ok),
            serde_json::Value::Object(o) => o.values().all(numbers_ok),
            _ => true,
        }
    }
    if !numbers_ok(&value) {
        return Some("Rule spec numbers must be finite and no larger than 1e12.");
    }
    let mut combinations = 1usize;
    for item in value.get("optimize").and_then(|v| v.as_array()).into_iter().flatten() {
        let n = |k: &str| item.get(k).and_then(serde_json::Value::as_f64);
        let (Some(lo), Some(hi), Some(step)) = (n("from"), n("to"), n("step")) else {
            return Some("Each optimize range needs numeric from, to and step.");
        };
        let count = ((hi - lo) / step + 1e-9).floor() + 1.0;
        if step <= 0.0 || hi < lo || !(1.0..=1000.0).contains(&count) {
            return Some("Each optimize range needs a positive step and at most 1,000 values.");
        }
        combinations = match combinations.checked_mul(count as usize) {
            Some(c) if c <= 50_000 => c,
            _ => return Some("Use a grid of at most 50,000 combinations."),
        };
    }
    None
}
#[cfg(test)]
mod lab_tests {
    use super::*;
    use std::collections::BTreeMap;
    const SPEC: &str = r#"{"v":1,"entry":{"all":[{"lhs":{"s":"close"},"op":">","rhs":{"value":1}}]},"exit":{"any":[{"lhs":{"s":"bars_held"},"op":">=","rhs":{"value":5}}]},"optimize":[{"path":"exit.any[0].rhs.value","from":1,"to":10,"step":1}]}"#;
    fn q(spec: &str) -> BTreeMap<String, String> {
        BTreeMap::from([("spec".to_string(), spec.to_string())])
    }
    #[test]
    fn lab_routes_are_allowlisted_and_lab_optimize_is_an_optimization() {
        for route in ["catalogue", "series", "run", "optimize"] {
            assert!(allowed_engine_path(&format!("lab/{route}")));
        }
        assert!(!allowed_engine_path("lab/strategies"));
        assert!(!allowed_engine_path("lab/../notes"));
        assert!(is_lab_optimization_path("lab/optimize"));
        assert!(!is_lab_optimization_path("lab/run"));
        assert!(!is_optimization_path("lab/optimize"), "existing book list stays unchanged");
    }
    #[test]
    fn lab_specs_are_bounded_before_the_engine() {
        assert_eq!(lab_spec_problem("lab/run", &q(SPEC)), None);
        assert_eq!(lab_spec_problem("lab/catalogue", &BTreeMap::new()), None);
        assert_eq!(lab_spec_problem("guru/connors/run", &BTreeMap::new()), None);
        assert!(lab_spec_problem("lab/run", &BTreeMap::new()).is_some());
        let padded = SPEC.replacen("{", &format!("{{\"pad\":\"{}\",", "x".repeat(LAB_SPEC_MAX_BYTES)), 1);
        let cond = r#"{"lhs":{"s":"close"},"op":">","rhs":{"value":1}}"#;
        let thirteen = format!(r#"{{"v":1,"entry":{{"all":[{}]}},"exit":{{"any":[{}]}}}}"#, vec![cond; 7].join(","), vec![cond; 6].join(","));
        for (spec, why) in [
            (padded.as_str(), "8 KB"),
            ("{", "valid JSON"),
            (thirteen.as_str(), "12 conditions"),
            (&SPEC.replace("\"value\":1}", "\"value\":1e13}"), "finite"),
            (&SPEC.replace("\"to\":10", "\"to\":5000"), "1,000 values"),
            (&SPEC.replace("\"step\":1", "\"step\":0"), "positive step"),
            (&SPEC.replace("\"step\":1}", "\"step\":1},{\"path\":\"x\",\"from\":1,\"to\":1000,\"step\":1},{\"path\":\"y\",\"from\":1,\"to\":1000,\"step\":1}"), "50,000"),
        ] {
            let problem = lab_spec_problem("lab/optimize", &q(spec)).unwrap_or_else(|| panic!("accepted {spec}"));
            assert!(problem.contains(why), "{why}: {problem}");
        }
    }
}

/// Engine markets: India and US serve every allowlisted path; `gl` (engine-gl: provisional global and crypto
/// TradingView exports) serves indicator-lab paths only, because book strategies have no admission rules for that data.
pub fn engine_market_allows(market: &str, path: &str) -> bool {
    match market {
        "in" | "us" => true,
        "gl" => path.starts_with("lab/"),
        _ => false,
    }
}

#[cfg(test)]
mod gl_tests {
    use super::*;
    #[test]
    fn global_and_crypto_reach_lab_routes_only() {
        for path in ["lab/catalogue", "lab/series", "lab/run", "lab/optimize", "lab/instruments"] {
            assert!(allowed_engine_path(path) && engine_market_allows("gl", path), "{path}");
            assert!(engine_market_allows("in", path) && engine_market_allows("us", path));
        }
        for path in ["tickers", "ohlc", "gurus", "guru/connors/run", "guru/connors/optimize", "guru/chande/summary", "notes"] {
            assert!(!engine_market_allows("gl", path), "{path}");
            assert!(engine_market_allows("in", path) && engine_market_allows("us", path), "{path}");
        }
        for market in ["crypto", "GL", "", "eu", "in/../gl"] {
            assert!(!engine_market_allows(market, "lab/run"), "{market}");
        }
    }
}
