//! /api/lab/* routes. Book code (data selection, costs, trade validation, equity, stats, ranking) is called
//! read-only; every response carries LABEL and no gate, readiness or book claim.
use super::bars::Bars;
use super::catalogue::{self, Placement};
use super::json::esc;
use super::resample::Tf;
use super::rules::{self, LabError, LABEL};
use super::series::{self, Cache, Frames};
use crate::{connors::Ctx, instrument_policy, State};
use std::collections::BTreeMap;
use std::net::TcpStream;

type Query = BTreeMap<String, String>;

pub fn serve(stream: &mut TcpStream, path: &str, st: &State, q: &Query) {
    match route(path, st, q) {
        None => crate::respond(stream, "404 Not Found", "text/plain", b"not found"),
        Some(Ok(body)) => crate::respond(stream, "200 OK", "application/json; charset=utf-8", body.as_bytes()),
        Some(Err(e)) => crate::respond(stream, "400 Bad Request", "application/json; charset=utf-8", e.json().as_bytes()),
    }
}
pub fn route(path: &str, st: &State, q: &Query) -> Option<Result<String, LabError>> {
    let f: fn(&State, &Query) -> Result<String, LabError> = match path {
        "/api/lab/catalogue" => |_, _| Ok(catalogue_json()),
        "/api/lab/series" => series_json,
        _ => return None,
    };
    Some(guarded(|| f(st, q)))
}
/// A panic anywhere in a lab request becomes a typed 400, never a dropped connection.
pub fn guarded(f: impl FnOnce() -> Result<String, LabError>) -> Result<String, LabError> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(f))
        .unwrap_or_else(|_| Err(LabError::Invalid("internal error while evaluating this request; no result is published".into())))
}

fn full(v: f64) -> String { if v.is_finite() { v.to_string() } else { "null".into() } }
fn arr(it: impl Iterator<Item = String>) -> String { format!("[{}]", it.collect::<Vec<_>>().join(",")) }
fn quoted<'a>(it: impl Iterator<Item = &'a str>) -> String { arr(it.map(|s| format!("\"{}\"", esc(s)))) }

pub fn catalogue_json() -> String {
    let defs = catalogue::all().into_iter().map(|d| {
        let params = arr(d.params.iter().map(|p| format!(
            "{{\"key\":\"{}\",\"label\":\"{}\",\"default\":{},\"min\":{},\"max\":{},\"step\":{},\"integer\":{}}}",
            p.key, esc(p.label), full(p.default), full(p.min), full(p.max), full(p.step), p.integer)));
        format!(
            "{{\"id\":\"{}\",\"name\":\"{}\",\"category\":\"{}\",\"placement\":\"{}\",\"needs\":{{\"volume\":{},\"intraday\":{}}},\"computed\":{},\"params\":{params},\"outputs\":{},\"first\":{},\"sparse\":{},\"definition\":\"{}\"}}",
            d.id, esc(d.name), esc(d.category), if d.placement == Placement::Price { "price" } else { "pane" }, d.needs.volume, d.needs.intraday,
            d.compute.is_some(), quoted(d.outputs.iter().copied()), arr(d.first.iter().map(|&f| if f == usize::MAX { "null".into() } else { f.to_string() })), d.sparse, esc(d.definition))
    });
    format!(
        "{{\"label\":\"{LABEL}\",\"price_sources\":{},\"operators\":{},\"timeframes\":[\"D\",\"W\",\"M\"],\"max_conditions\":{},\"max_spec_bytes\":{},\"max_combos\":{},\"indicators\":{}}}",
        quoted(rules::PRICE_SOURCES.iter().copied()), quoted(rules::OPERATORS.iter().copied()), rules::MAX_CONDITIONS, rules::MAX_SPEC_BYTES, crate::MAX_COMBOS, arr(defs))
}

/// Instrument identity from the server's own data (never from the query): type, native flag, market, currency.
fn instrument(selected: &crate::SelectedContext, ticker: &str) -> (&'static str, String) {
    let native = selected.native();
    let kind = instrument_policy::selected_type(ticker, native);
    let market = selected.evidence().map(|e| e.market()).or_else(|| crate::preloaded_market(ticker));
    let opt = |s: Option<&str>| s.map_or("null".into(), |s| format!("\"{s}\""));
    (kind, format!("{{\"ticker\":\"{}\",\"type\":\"{kind}\",\"native\":{native},\"market\":{},\"currency\":{}}}",
        esc(ticker), opt(market.map(|m| m.code())), opt(market.map(|m| m.currency()))))
}
fn select<'a>(st: &'a State, q: &Query) -> Result<crate::SelectedContext<'a>, LabError> {
    crate::select_context(st, q).map_err(|e| LabError::NeedsData(e.trim_start_matches("needs_data:").trim().into()))
}
pub fn series_json(st: &State, q: &Query) -> Result<String, LabError> {
    let id = q.get("id").ok_or_else(|| LabError::Invalid("id is required".into()))?;
    let tf = q.get("tf").map_or(Some(Tf::D), |t| Tf::parse(t)).ok_or_else(|| LabError::Invalid("tf must be D, W or M".into()))?;
    if let Some((k, _)) = q.iter().find(|(k, v)| k.starts_with("p.") && !v.parse::<f64>().is_ok_and(f64::is_finite)) {
        return Err(LabError::Invalid(format!("{k} must be a finite number")));
    }
    let r = rules::series_ref("series", id, &crate::params_of(q), tf, None, 0)?;
    let selected = select(st, q)?;
    let held = selected.context(st);
    let ctx: &Ctx = &held;
    let (_, instrument) = instrument(&selected, &ctx.r[0].ticker);
    let fr = Frames::new(Bars::from_rows(ctx.r));
    let outputs: Vec<&str> = catalogue::get(id).map_or(vec![id.as_str()], |d| d.outputs.to_vec());
    let mut cache = Cache::new();
    let mut values = Vec::new();
    for (k, name) in outputs.iter().enumerate() {
        let s = series::series(&fr, &mut cache, &rules::SeriesRef { out: k, ..r.clone() })?;
        values.push(format!("\"{}\":{}", esc(name), arr(s.into_iter().map(full))));
    }
    let params = r.params.iter().map(|(k, v)| format!("\"{}\":{}", esc(k), full(*v))).collect::<Vec<_>>().join(",");
    Ok(format!("{{\"label\":\"{LABEL}\",\"instrument\":{instrument},\"id\":\"{}\",\"tf\":\"{:?}\",\"params\":{{{params}}},\"outputs\":{},\"dates\":{},\"series\":{{{}}},\"provenance\":{}}}",
        esc(id), tf, quoted(outputs.iter().copied()), quoted(ctx.r.iter().map(|r| r.date.as_str())), values.join(","), crate::source_provenance(st, q)))
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;

    fn state(n: usize, ticker: &str, volume: bool) -> State {
        let rows: &'static [Row] = Box::leak((0..n).map(|i| {
            let c = 100. + (i as f64 * 0.3).sin() * 8. + i as f64 * 0.02;
            Row { ticker: ticker.into(), date: format!("{}-{:02}-{:02}", 2015 + i / 336, 1 + (i / 28) % 12, 1 + i % 28), open: c - 0.2, high: c + 1., low: c - 1., close: c, volume: volume.then_some(1000.) }
        }).collect::<Vec<_>>().into_boxed_slice());
        State { ctxs: BTreeMap::from([(ticker.into(), Ctx::new(rows, rows, &[]))]), baskets: Default::default(), vix: vec![], market_extra: Default::default() }
    }
    const SPEC: &str = r#"{"v":1,"entry":{"all":[{"lhs":{"s":"close"},"op":"crosses_above","rhs":{"s":"sma","p":{"length":5}}}]},"exit":{"any":[{"lhs":{"s":"bars_held"},"op":">=","rhs":{"value":3}}]},"optimize":[{"path":"entry.all[0].rhs.p.length","from":3,"to":7,"step":2},{"path":"exit.any[0].rhs.value","from":2,"to":4,"step":1}]}"#;
    fn q(extra: &[(&str, &str)]) -> Query {
        let mut q: Query = [("ticker", "SPX"), ("spec", SPEC), ("cost", "5")].iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();
        for (k, v) in extra { q.insert(k.to_string(), v.to_string()); }
        q
    }

    #[test] fn catalogue_lists_every_indicator_with_label() {
        let j = catalogue_json();
        assert!(j.starts_with(&format!("{{\"label\":\"{LABEL}\"")));
        for d in catalogue::all() { assert!(j.contains(&format!("{{\"id\":\"{}\"", d.id)), "{}", d.id); }
        assert!(j.contains("\"id\":\"vwap\"") && j.contains("\"intraday\":true") && j.contains("\"first\":[null]"));
        assert!(crate::lab::json::parse(&j).is_ok());
    }
    #[test] fn panics_become_typed_errors() {
        assert_eq!(guarded(|| panic!("boom")).unwrap_err().status(), "invalid");
        let st = state(50, "SPX", false);
        assert!(route("/api/lab/nothing", &st, &q(&[])).is_none());
        assert!(route("/api/lab/catalogue", &st, &q(&[])).unwrap().is_ok());
    }
    #[test] fn series_route_returns_daily_aligned_outputs() {
        let st = state(120, "SPX", true);
        let mut query = q(&[("id", "macd"), ("tf", "W"), ("p.fast", "3")]);
        let j = series_json(&st, &query).unwrap();
        let v = crate::lab::json::parse(&j).unwrap();
        let crate::lab::json::Json::Arr(dates) = v.get("dates").unwrap() else { panic!() };
        let crate::lab::json::Json::Arr(signal) = v.get("series").unwrap().get("signal").unwrap() else { panic!() };
        assert_eq!((dates.len(), signal.len()), (120, 120));
        assert!(j.contains("\"params\":{\"fast\":3,\"signal\":9,\"slow\":26}") && j.contains("\"tf\":\"W\""));
        query.insert("p.fast".into(), "abc".into());
        assert!(series_json(&st, &query).unwrap_err().message().contains("p.fast"));
        query.insert("p.fast".into(), "0".into());
        assert!(series_json(&st, &query).unwrap_err().message().contains("fast must be between"));
        assert_eq!(series_json(&st, &q(&[("id", "close"), ("tf", "Q")])).unwrap_err().message(), "tf must be D, W or M");
    }
}
