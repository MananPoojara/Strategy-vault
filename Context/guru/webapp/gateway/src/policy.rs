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
            | "guru/connors/basket-run"
            | "guru/connors/params"
            | "guru/connors/optimize"
            | "guru/ross/strategies"
            | "guru/ross/summary"
            | "guru/ross/readiness"
            | "guru/ross/run"
            | "guru/ross/basket-run"
            | "guru/ross/params"
            | "guru/ross/optimize"
            | "guru/chande/strategies"
            | "guru/chande/summary"
            | "guru/chande/readiness"
            | "guru/chande/run"
            | "guru/chande/basket-run"
            | "guru/chande/params"
            | "guru/chande/optimize"
            | "guru/pristine/strategies"
            | "guru/pristine/summary"
            | "guru/pristine/readiness"
            | "guru/pristine/run"
            | "guru/pristine/params"
    )
}
pub fn is_optimization_path(path:&str)->bool {
    matches!(path,"guru/connors/optimize"|"guru/ross/optimize"|"guru/chande/optimize")
}
/// Calls that run backtests on the engine (seconds), kept apart from fixed catalogue calls (milliseconds).
pub fn is_expensive_path(path:&str)->bool {
    path.strip_prefix("guru/").and_then(|rest|rest.split_once('/')).is_some_and(|(_,endpoint)|matches!(endpoint,"summary"|"run"|"basket-run"|"readiness"))
}
/// Unhashed static files that the browser keeps but must recheck each time. The session check runs on every recheck.
pub fn revalidates(path:&str)->bool {
    path.starts_with("/data/")||path.starts_with("/vendor/")
}
/// Vite puts only content-hashed names under /assets/. Anything failed, signed out or dynamic stays no-store.
pub fn cache_control(path:&str,status:u16)->&'static str {
    match status {
        200|206 if path.starts_with("/assets/") => "private, max-age=31536000, immutable",
        200|206|304 if revalidates(path) => "private, no-cache",
        _ => "no-store",
    }
}
/// Weak: the same file may be sent gzipped or not.
pub fn file_etag(last_modified:&str,length:&str)->String {
    let stamp:String=format!("{last_modified}-{length}").chars().filter(char::is_ascii_alphanumeric).collect();
    format!("W/\"{stamp}\"")
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
    fn backtests_and_catalogue_calls_use_separate_slots() {
        for owner in ["connors","ross","chande","pristine"] {
            for route in ["summary","run","basket-run","readiness"] {assert!(is_expensive_path(&format!("guru/{owner}/{route}")));}
            for route in ["strategies","params"] {assert!(!is_expensive_path(&format!("guru/{owner}/{route}")));}
        }
        for path in ["tickers","ohlc","gurus","summary"] {assert!(!is_expensive_path(path));}
    }
    #[test]
    fn only_successful_static_files_are_cached() {
        assert_eq!(cache_control("/assets/index-BWV1hsov.js",200),"private, max-age=31536000, immutable");
        for (path,status) in [("/assets/index-BWV1hsov.js",401),("/assets/gone.js",404),("/index.html",200),("/",200),("/login.js",200),("/api/in/gurus",200),("/data/x.json",401),("/data/x.json",404),("/api/data/x",200)] {
            assert_eq!(cache_control(path,status),"no-store","{path} {status}");
        }
        for (path,status) in [("/data/x.json",200),("/data/x.json",304),("/vendor/a.js",200)] {
            assert_eq!(cache_control(path,status),"private, no-cache");
        }
    }
    #[test]
    fn etag_changes_when_a_file_changes_in_either_direction() {
        let a=file_etag("Fri, 30 Oct 2026 10:00:00 GMT","100");
        assert_ne!(a,file_etag("Thu, 29 Oct 2026 10:00:00 GMT","100"));
        assert_ne!(a,file_etag("Fri, 30 Oct 2026 10:00:00 GMT","101"));
        assert!(a.starts_with("W/\"")&&a.ends_with('"'));
    }
    #[test]
    fn engine_routes_are_allowlisted_and_read_only() {
        assert!(allowed_engine_path("tickers"));
        assert!(allowed_engine_path("ohlc"));
        assert!(allowed_engine_path("guru/connors/run"));
        assert!(allowed_engine_path("guru/connors/readiness"));
        assert!(allowed_engine_path("guru/connors/optimize"));
        for route in ["strategies","summary","readiness","run","basket-run","params","optimize"] {
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
    fn pristine_has_only_the_reviewed_fixed_read_endpoints() {
        for route in ["strategies","summary","readiness","run","params"] {
            assert!(allowed_engine_path(&format!("guru/pristine/{route}")));
            assert!(!is_optimization_path(&format!("guru/pristine/{route}")));
        }
        for route in ["optimize","basket-run","notes","run/","../run"] {
            assert!(!allowed_engine_path(&format!("guru/pristine/{route}")));
        }
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
    #[test]
    fn basket_capacity_membership_and_all_execution_costs_are_bounded(){
        assert!(safe_engine_numbers(&params(&[("max_positions","20"),("membership","current"),("slippage","2"),("rollover","0")])));
        assert!(safe_engine_numbers(&params(&[("max_positions","100"),("membership","past")])));
        for pair in [("max_positions","0"),("max_positions","101"),("max_positions","2.5"),("max_positions","-1"),("membership","alive"),("slippage","NaN"),("rollover","inf")]{
            assert!(!safe_engine_numbers(&params(&[pair])),"{pair:?}");
        }
        assert!(!allowed_engine_path("guru/other/basket-run"));
        assert!(allowed_engine_path("guru/connors/basket-run"));
        assert!(!is_optimization_path("guru/connors/basket-run"));
    }
}

pub fn safe_engine_numbers(params: &std::collections::BTreeMap<String, String>) -> bool {
    let mut combinations = 1usize;
    for (key, value) in params {
        if key == "max_positions" && !value.parse::<usize>().is_ok_and(|n|(1..=100).contains(&n)) {
            return false;
        }
        if key == "membership" && !matches!(value.as_str(),"current"|"past") {
            return false;
        }
        if key.starts_with("p.") || matches!(key.as_str(),"cost"|"slippage"|"rollover") {
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
