//! /api/lab/* routes. Book code (data selection, costs, trade validation, equity, stats, ranking) is called
//! read-only; every response carries LABEL and no gate, readiness or book claim.
use super::bars::Bars;
use super::catalogue::{self, Placement};
use super::json::esc;
use super::markets;
use super::report;
use super::resample::Tf;
use super::rules::{self, LabError, LABEL};
use super::series::{self, Cache, Compiled, Frames};
use super::rules::Spec;
use super::simulate::{simulate, simulate_events};
use super::{fidelity, ledger, templates};
use crate::{accounting, connors::{Ctx, StrategyDef}, instrument_policy, trade_validation, State};
use std::collections::BTreeMap;
use std::net::TcpStream;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;

type Query = BTreeMap<String, String>;
/// Wall-clock bound for one lab optimization (the gateway allows 600 s).
pub(super) const OPT_SECONDS: u64 = 300;
const STOPPED: &str = "optimization stopped after another combination failed";
/// `,"info":{…}` — AmiBroker Analysis "Info" tab: elapsed ms, combinations evaluated, worker threads and warnings.
pub(super) fn info_json(ms: u128, combos: usize, threads: usize, warnings: &[String]) -> String {
    format!(",\"info\":{{\"ms\":{ms},\"combos\":{combos},\"threads\":{threads},\"warnings\":{}}}", quoted(warnings.iter().map(String::as_str)))
}
/// A moved window start (stale opens) as an info warning; `start` is the response's `start` JSON.
fn start_warning(start: &str) -> Vec<String> {
    super::json::parse(start).ok().and_then(|j| Some(format!("the window starts {}: {}", j.get("date")?.as_str()?, j.get("reason")?.as_str()?))).into_iter().collect()
}
#[cfg(test)]
thread_local! { static LAST_DONE: std::cell::RefCell<Option<Arc<AtomicUsize>>> = const { std::cell::RefCell::new(None) }; }

pub fn serve(stream: &mut TcpStream, path: &str, st: &State, q: &Query) {
    let (status, body) = response(markets::engine_market().as_deref(), path, st, q);
    crate::respond(stream, status, "application/json; charset=utf-8", body.as_bytes());
}
/// HTTP status and body of one lab request; on engine-gl (`market` "gl") every body carries the provisional flag.
pub fn response(market: Option<&str>, path: &str, st: &State, q: &Query) -> (&'static str, String) {
    let (status, body) = match route(path, st, q) {
        None => ("404 Not Found", not_found_json(path)),
        Some(Ok(body)) => ("200 OK", body),
        Some(Err(e)) => ("400 Bad Request", e.json()),
    };
    (status, if market == Some("gl") { markets::mark_provisional(&body) } else { body })
}
pub fn not_found_json(path: &str) -> String {
    format!("{{\"status\":\"not_found\",\"error\":\"unknown lab route {}\",\"label\":\"{LABEL}\"}}", esc(path))
}
pub fn route(path: &str, st: &State, q: &Query) -> Option<Result<String, LabError>> {
    let f: fn(&State, &Query) -> Result<String, LabError> = match path {
        "/api/lab/catalogue" => |st, _| Ok(catalogue_for_state(st)),
        "/api/lab/series" => series_json,
        "/api/lab/formula" => formula_json,
        "/api/lab/run" => run_json,
        "/api/lab/optimize" => optimize_json,
        "/api/lab/walkforward" => super::walkforward::walkforward_json,
        "/api/lab/scan" => super::scan::scan_json,
        "/api/lab/instruments" => |st, _| Ok(markets::instruments_json(st)),
        "/api/lab/templates" => |_, _| Ok(templates_json()),
        "/api/lab/template" => template_json,
        "/api/lab/fidelity" => fidelity_json,
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

/// `max_query_bytes` (16384) must match the gateway; tick distances wait for admitted futures contracts.
pub fn catalogue_json() -> String {
    let defs = catalogue::all().into_iter().map(|d| {
        let (tv, ab) = super::sources::names(d.id);
        let params = arr(d.params.iter().map(|p| format!(
            "{{\"key\":\"{}\",\"label\":\"{}\",\"default\":{},\"min\":{},\"max\":{},\"step\":{},\"integer\":{}}}",
            p.key, esc(p.label), full(p.default), full(p.min), full(p.max), full(p.step), p.integer)));
        format!(
            "{{\"id\":\"{}\",\"name\":\"{}\",\"category\":\"{}\",\"placement\":\"{}\",\"needs\":{{\"volume\":{},\"intraday\":{},\"breadth\":{},\"second_symbol\":{},\"fundamentals\":{},\"derivatives\":{},\"onchain\":{},\"order_flow\":{},\"order_book\":{}}},\"computed\":{},\"params\":{params},\"outputs\":{},\"first\":{},\"sparse\":{},\"definition\":\"{}\",\"note\":\"{}\",\"tv\":\"{}\",\"ab\":\"{}\"}}",
            d.id, esc(d.name), esc(d.category), if d.placement == Placement::Price { "price" } else { "pane" }, d.needs.volume, d.needs.intraday, d.needs.breadth, d.needs.second_symbol, d.needs.fundamentals, d.needs.derivatives, d.needs.onchain, d.needs.order_flow, d.needs.order_book,
            d.compute.is_some(), quoted(d.outputs.iter().copied()), arr(d.first.iter().map(|&f| if f == usize::MAX { "null".into() } else { f.to_string() })), d.sparse, esc(d.definition), esc(d.note), esc(tv), esc(ab))
    });
    let mut body = format!(
        "{{\"label\":\"{LABEL}\",\"price_sources\":{},\"operators\":{},\"timeframes\":[\"D\",\"W\",\"M\",\"Q\",\"Y\"],\"custom_days\":[2,{}],\"expand_modes\":[\"last\",\"first\",\"point\"],\"max_conditions\":{},\"max_leaves\":{},\"max_nodes\":{},\"node_kinds\":{},\"expressions\":{},\"max_spec_bytes\":{},\"max_query_bytes\":16384,\"execution\":{{\"direction\":[\"long\",\"short\",\"both\"],\"fill\":[\"next_open\",\"close\"],\"opposite\":[\"reverse\",\"exit\",\"ignore\"],\"entry_on\":[\"level\",\"edge\"],\"intrabar\":[\"stop_first\",\"ohlc_path\"],\"sizing\":[\"pct_equity\",\"risk_pct\",\"units\",\"value\"],\"distances\":[\"pct\",\"atr\",\"points\",\"at\"],\"max_take_profits\":{},\"max_legs\":{},\"max_valid_bars\":{},\"max_delay_bars\":{},\"cooldown_after\":[\"any\",\"loss\",\"win\"],\"max_cooldown_bars\":{},\"max_trades_per\":[\"W\",\"M\",\"Q\",\"Y\"],\"max_trades_count\":{},\"max_consecutive_losses\":{},\"max_pause_bars\":{},\"max_drawdown_pct_below\":100}},\"max_combos\":{},\"indicators\":{}}}",
        quoted(rules::PRICE_SOURCES.iter().copied()), quoted(rules::OPERATORS.iter().copied()), super::resample::MAX_DAYS, rules::MAX_CONDITIONS, rules::MAX_LEAVES, rules::MAX_NODES,
        quoted(rules::NODE_KINDS.iter().copied()), quoted(rules::EXPRESSIONS.iter().copied()), rules::MAX_SPEC_BYTES, rules::MAX_TAKE_PROFITS, rules::MAX_LEGS, rules::MAX_VALID_BARS, rules::MAX_DELAY,
        rules::MAX_OFFSET, rules::MAX_OFFSET, rules::MAX_OFFSET, rules::MAX_PAUSE, crate::MAX_COMBOS, arr(defs));
    body.pop();body.push_str(&format!(",\"formula_functions\":{},\"max_foreign\":{},\"foreign_volatility\":null,\"portfolio\":null}}",super::formula::functions_json(),rules::MAX_FOREIGN));body
}

/// Instrument identity from the server's own data (never from the query): type, native flag, market, currency.
pub(super) fn instrument(selected: &crate::SelectedContext, ticker: &str) -> (&'static str, String) {
    if let Some(m) = markets::gl(ticker) { return (m.kind, m.json()); }
    let native = selected.native();
    let kind = instrument_policy::selected_type(ticker, native);
    let market = selected.evidence().map(|e| e.market()).or_else(|| crate::preloaded_market(ticker));
    let opt = |s: Option<&str>| s.map_or("null".into(), |s| format!("\"{s}\""));
    (kind, format!("{{\"ticker\":\"{}\",\"type\":\"{kind}\",\"native\":{native},\"market\":{},\"currency\":{}}}",
        esc(ticker), opt(market.map(|m| m.code())), opt(market.map(|m| m.currency()))))
}
/// Only an unknown ticker is the caller's mistake; every other selection error is a data gap.
fn select_error(e: String) -> LabError {
    if let Some(m) = e.strip_prefix("unknown ticker ").and_then(markets::gl) { return LabError::NeedsData(markets::not_staged(m.ticker)); }
    if e.starts_with("unknown ticker ") { LabError::Invalid(e) } else { LabError::NeedsData(e.trim_start_matches("needs_data:").trim().into()) }
}
pub(super) fn select<'a>(st: &'a State, q: &Query) -> Result<crate::SelectedContext<'a>, LabError> {
    crate::select_context(st, q).map_err(select_error)
}
/// Establish market identity before reading a foreign source; the engine never fetches another market.
fn symbol_market(st: &State, ticker: &str) -> Option<&'static str> {
    if markets::gl(ticker).is_some() { return Some("gl"); }
    match ticker {
        "INDIA VIX" => Some("in"), "CBOE VIX" => Some("us"),
        _ => st.baskets.native_evidence(ticker).map(|e| e.market()).or_else(|| crate::preloaded_market(ticker)).map(|m| m.code()),
    }
}
pub(super) fn close_fills(spec: &Spec) -> bool {
    std::iter::once(&spec.exec).chain(spec.short_side.as_ref().map(|s| &s.exec))
        .any(|x| x.market.map_or(x.fill == rules::Fill::Close || x.exit_fill == rules::Fill::Close, |m| m.iter().any(|o| o.1 == 0)))
}
/// Load each referenced instrument once and retain operand paths on every refusal.
pub(super) fn others_for(st: &State, run: &str, refs: &[(String, rules::Foreign)], close: bool, member_scope: bool) -> Result<(series::Others, String), LabError> {
    let mut names = BTreeMap::<String, (String, bool)>::new();
    for (path, value) in refs {
        for (key, name, fixup) in [("symbol", value.symbol.as_ref(), value.fixup), ("vs", value.vs.as_ref(), false)] {
            if let Some(name) = name {
                let entry = names.entry(name.clone()).or_insert_with(|| (format!("{path}.{key}"), false));
                entry.1 |= fixup;
            }
        }
    }
    if names.len() > rules::MAX_FOREIGN { return Err(LabError::Invalid(format!("the other-instrument limit is {}", rules::MAX_FOREIGN))); }
    let mut frames = BTreeMap::new();
    let mut items = Vec::new();
    // First validate all market identities. No expensive source read happens before these checks.
    for (name, (path, _)) in &names {
        let Some(other_market) = symbol_market(st, name) else {
            return Err(LabError::Invalid(format!("{path}: unknown ticker {name} on this market")));
        };
        let run_market = symbol_market(st, run).ok_or_else(|| LabError::NeedsData(format!("{run}: source market is not established")))?;
        if other_market != run_market {
            let note = if other_market == "gl" { "served by /api/gl; provisional data cannot be mixed into India/US" }
                else if run_market == "gl" { "served by /api/in or /api/us; provisional data cannot be mixed with India/US" }
                else { "cross-market references are not available on this engine" };
            return Err(LabError::NeedsData(format!("{path}: {name} is {note}")));
        }
    }
    for (name, (path, fixup)) in names {
        let (loaded, kind, first, last) = frames_of(st, &name).map_err(|e| match e {
            LabError::Invalid(m) => LabError::Invalid(format!("{path}: {m}")),
            LabError::Unsupported(m) => LabError::Unsupported(format!("{path}: {m}")),
            LabError::NeedsData(m) => LabError::NeedsData(format!("{path}: {m}")),
        })?;
        let mut uses=Vec::new();
        for (path,value) in refs {
            for (key,target,source,fill) in [("symbol",value.symbol.as_deref(),run,value.fixup),("vs",value.vs.as_deref(),value.symbol.as_deref().unwrap_or(run),false)] {
                if target==Some(name.as_str()){
                    let scope=if member_scope&&(key=="symbol"||value.symbol.is_none()) {",\"source_scope\":\"portfolio_member\""}else{""};
                    uses.push(format!("{{\"path\":\"{}\",\"source\":\"{}\",\"alignment\":\"{}\",\"fixup\":{fill}{scope}}}",esc(&format!("{path}.{key}")),esc(source),super::foreign::alignment(source,&name,close).name()));
                }
            }
        }
        let alignment = super::foreign::alignment(run, &name, close);
        items.push(format!("{{\"ticker\":\"{}\",\"type\":\"{kind}\",\"alignment\":\"{}\",\"fixup\":{fixup},\"first\":\"{first}\",\"last\":\"{last}\",\"uses\":[{}]}}", esc(&name), alignment.name(),uses.join(",")));
        frames.insert(name, loaded);
    }
    Ok((series::Others { run: run.into(), close_fills: close, frames: Arc::new(frames) },
        if items.is_empty() { String::new() } else { format!(",\"foreign\":[{}]", items.join(",")) }))
}
/// Shared read-only loader for foreign signals and portfolio members; coverage is never invented.
pub(super) fn frames_of(st: &State, ticker: &str) -> Result<(Frames, &'static str, String, String), LabError> {
    let calendar = |bars: &Bars| super::foreign::validate_dates(&bars.date, ticker)
        .map_err(|e| LabError::NeedsData(e.message().into()));
    if super::foreign::VIX.contains(&ticker) {
        if st.vix.is_empty() || st.vix.iter().any(|r| r.ticker != ticker) {
            return Err(LabError::NeedsData(format!("{ticker} is not loaded on this engine")));
        }
        raw_prices_clean(&st.vix)?;
        let bars = Bars::from_rows(&st.vix);
        calendar(&bars)?;
        let first = bars.date.first().cloned().ok_or_else(|| LabError::NeedsData(format!("{ticker}: no dates are loaded")))?;
        let last = bars.date.last().cloned().unwrap();
        return Ok((Frames::new(bars), "volatility_index", first, last));
    }
    if crate::preloaded_market(ticker).is_some() && !st.ctxs.contains_key(ticker) {
        return Err(LabError::NeedsData(format!("{ticker} is not loaded on this engine")));
    }
    let query = Query::from([("ticker".into(), ticker.into())]);
    let selected = select(st, &query)?;
    let held = selected.context(st);
    if !selected.native() && held.r.iter().any(|r| r.ticker != ticker) {
        return Err(LabError::NeedsData(format!("{ticker}: loaded source identity does not match the requested instrument")));
    }
    let (kind, _) = instrument(&selected, ticker);
    known_type(kind, ticker)?;
    let first = held.r.first().ok_or_else(|| LabError::NeedsData(format!("{ticker}: no price bars are loaded")))?.date.clone();
    let last = held.r.last().unwrap().date.clone();
    raw_prices_clean(held.r)?;
    let bars = Bars::from_ctx(&held);
    calendar(&bars)?;
    Ok((Frames::new(bars), kind, first, last))
}
pub fn series_json(st: &State, q: &Query) -> Result<String, LabError> {
    if let Some(text) = q.get("formula") { return formula_series_json(st, q, text); }
    let id = q.get("id").ok_or_else(|| LabError::Invalid("id is required".into()))?;
    let tf = q.get("tf").map_or(Some(Tf::D), |t| Tf::parse(t)).ok_or_else(|| LabError::Invalid(format!("tf must be {}", super::resample::TF_HELP)))?;
    if let Some((k, _)) = q.iter().find(|(k, v)| k.starts_with("p.") && !v.parse::<f64>().is_ok_and(f64::is_finite)) {
        return Err(LabError::Invalid(format!("{k} must be a finite number")));
    }
    let foreign = rules::foreign_ref("series", q.get("symbol").map(String::as_str), q.get("vs").map(String::as_str), q.get("fixup").map(String::as_str))?;
    let r = rules::series_ref("series", id, &crate::params_of(q), tf, None, 0, foreign.clone())?;
    let selected = select(st, q)?;
    let held = selected.context(st);
    let ctx: &Ctx = &held;
    let (_, instrument) = instrument(&selected, &ctx.r[0].ticker);
    let (others, foreign_json) = others_for(st, &ctx.r[0].ticker, &[("series".into(), foreign)], false, false)?;
    let fr = Frames::new(Bars::from_ctx(ctx));
    let outputs: Vec<&str> = catalogue::get(id).map_or(vec![id.as_str()], |d| d.outputs.to_vec());
    let mut cache = Cache::new();
    let mut values = Vec::new();
    for (k, name) in outputs.iter().enumerate() {
        let s = series::series_in(&fr, &others, &mut cache, &rules::SeriesRef { out: k, ..r.clone() })?;
        values.push(format!("\"{}\":{}", esc(name), arr(s.into_iter().map(full))));
    }
    let params = series::shown_params(&r.id, &r.params).map(|(k, v)| format!("\"{}\":{}", esc(k), full(*v))).collect::<Vec<_>>().join(",");
    Ok(format!("{{\"label\":\"{LABEL}\",\"instrument\":{instrument}{foreign_json},\"id\":\"{}\",\"tf\":\"{}\",\"params\":{{{params}}},\"outputs\":{},\"dates\":{},\"series\":{{{}}},\"provenance\":{}{}}}",
        esc(id), tf.name(), quoted(outputs.iter().copied()), quoted(ctx.r.iter().map(|r| r.date.as_str())), values.join(","), crate::source_provenance(st, q), adaptations([id.as_str()].into_iter())))
}
/// `/api/lab/formula?formula=…`: `{"ok":true,"params":[…],"plots":[…],"placement":…}` or 400 `line L, col C: message`.
pub fn formula_json(_: &State, q: &Query) -> Result<String, LabError> {
    let text = q.get("formula").ok_or_else(|| LabError::Invalid("formula is required".into()))?;
    let prog = super::formula::parse(text).map_err(LabError::Invalid)?;
    Ok(format!("{{\"ok\":true,{}}}", super::formula::info_json(&prog)))
}
/// `/api/lab/series?formula=…&name=…&<Param>=<v>…&tf=…&ticker=…`: an indicator series body plus `"formula":{params,plots,placement}`.
/// Param values come as bare keys (`Fast=9`; formula::RESERVED_KEYS cannot be Param names) or as `p.Fast`.
fn formula_series_json(st: &State, q: &Query, text: &str) -> Result<String, LabError> {
    let tf = q.get("tf").map_or(Some(Tf::D), |t| Tf::parse(t)).ok_or_else(|| LabError::Invalid(format!("tf must be {}", super::resample::TF_HELP)))?;
    if ["symbol", "vs", "fixup"].iter().any(|k| q.get(*k).is_some_and(|v| !v.is_empty())) {
        return Err(LabError::Invalid("symbol, vs and fixup apply to catalogue indicators; a formula runs on the selected ticker".into()));
    }
    let prog = super::formula::parse(text).map_err(LabError::Invalid)?;
    if let Some(k) = q.keys().filter_map(|k| k.strip_prefix("p.")).find(|k| !prog.params.iter().any(|p| p.key == *k)) {
        return Err(LabError::Invalid(format!("unknown parameter '{k}'")));
    }
    let mut given = catalogue::Params::new();
    for p in &prog.params {
        let Some(v) = q.get(&format!("p.{}", p.key)).or_else(|| q.get(&p.key)) else { continue };
        given.insert(p.key.clone(), v.parse::<f64>().ok().filter(|v| v.is_finite()).ok_or_else(|| LabError::Invalid(format!("{} must be a finite number", p.key)))?);
    }
    let (f, params, _) = super::formula::operand(q.get("name").map(String::as_str), text, &given, None).map_err(LabError::Invalid)?;
    let selected = select(st, q)?;
    let held = selected.context(st);
    let ctx: &Ctx = &held;
    let (_, instrument) = instrument(&selected, &ctx.r[0].ticker);
    let fr = Frames::new(Bars::from_ctx(ctx));
    let shown = f.prog.params.iter().map(|p| format!("\"{}\":{}", esc(&p.key), full(params[&p.key]))).collect::<Vec<_>>().join(",");
    let (name, info, plots) = (f.name.clone(), super::formula::info_json(&f.prog), f.prog.plots.clone());
    let r = rules::SeriesRef { id: "formula".into(), params, tf, out: 0, offset: 0, foreign: Default::default(), formula: Some(Arc::new(f)) };
    let mut cache = Cache::new();
    let mut values = Vec::new();
    for (k, plot) in plots.iter().enumerate() {
        let s = series::series(&fr, &mut cache, &rules::SeriesRef { out: k, ..r.clone() })?;
        values.push(format!("\"{}\":{}", esc(plot), arr(s.into_iter().map(full))));
    }
    Ok(format!("{{\"label\":\"{LABEL}\",\"instrument\":{instrument},\"id\":\"formula\",\"name\":\"{}\",\"tf\":\"{}\",\"params\":{{{shown}}},\"outputs\":{},\"dates\":{},\"series\":{{{}}},\"provenance\":{},\"formula\":{{{info}}}}}",
        esc(&name), tf.name(), quoted(plots.iter().map(String::as_str)), quoted(ctx.r.iter().map(|r| r.date.as_str())), values.join(","), crate::source_provenance(st, q)))
}
fn known_type(kind: &str, ticker: &str) -> Result<(), LabError> {
    if kind == "unknown" { return Err(LabError::NeedsData(format!("{ticker}: instrument type is not established (index, ETF or catalogued stock); no lab result is published"))); }
    Ok(())
}
/// Requested dates must be real and inside admitted coverage; nothing is filled or silently shortened.
/// Also returns the `start` disclosure JSON: lab fills use the next open, which is stale for some indices before a known date.
pub(super) fn window(ctx: &Ctx, q: &Query) -> Result<(usize, usize, String), LabError> {
    for key in ["from", "to", "split"] {
        if q.get(key).is_some_and(|v| !v.is_empty() && !crate::baskets::valid_date(v)) { return Err(LabError::Invalid(format!("{key} must be a valid YYYY-MM-DD date"))); }
    }
    let (first, last) = (&ctx.r[0].date, &ctx.r[ctx.r.len() - 1].date);
    if q.get("from").is_some_and(|f| !f.is_empty() && f < first) { return Err(LabError::NeedsData(format!("from precedes admitted data starting {first}; no earlier prices are filled"))); }
    if q.get("to").is_some_and(|t| !t.is_empty() && t > last) { return Err(LabError::NeedsData(format!("to exceeds admitted data ending {last}; the range is not silently shortened"))); }
    let (_, mut from, to, _) = crate::resolve_context(ctx, q)?;
    let mut start = "null".to_string();
    let gl_cut = markets::gl(&ctx.r[0].ticker).and_then(|m| markets::tradable_from(ctx.r, m.continuous));
    if let Some(cut) = crate::gate::real_open_from(&ctx.r[0].ticker).or(gl_cut.as_deref()) {
        let (ticker, stale) = (&ctx.r[0].ticker, format!("opens before {cut} are stale"));
        let k = ctx.r.partition_point(|r| r.date.as_str() < cut);
        if q.get("from").is_some_and(|f| !f.is_empty() && f.as_str() < cut) { return Err(LabError::NeedsData(format!("{stale} for {ticker}; choose from >= {cut}"))); }
        if k > from {
            if k >= to { return Err(LabError::NeedsData(format!("{stale} for {ticker} and no admitted bar follows; no lab result is published"))); }
            from = k;
            start = format!("{{\"date\":\"{}\",\"reason\":\"{}\"}}", ctx.r[k].date, stale);
        }
    }
    Ok((from, to, start))
}
fn spec_of(q: &Query) -> Result<Spec, LabError> {
    let spec = rules::parse_spec(q.get("spec").ok_or_else(|| LabError::Invalid("spec is required".into()))?)?;
    Ok(spec)
}
/// execution.cost_bps (saved with a strategy) and the `cost` query must agree; one of them may be absent.
fn with_spec_cost(q: &Query, spec: &Spec) -> Result<Query, LabError> {
    let mut q = q.clone();
    if let Some(c) = spec.cost_bps {
        match q.get("cost").map(|v| v.parse::<f64>()) {
            None => { q.insert("cost".into(), c.to_string()); }
            Some(Ok(v)) if v == c => {}
            _ => return Err(LabError::Invalid(format!("execution.cost_bps ({c}) differs from the cost query; send one value"))),
        }
    }
    Ok(q)
}
/// Book statistics (read-only) rescaled to the instrument's own bar frequency (lab/markets.rs annualise).
fn lab_stats(ctx: &Ctx, run: &crate::Run, costs: accounting::Costs) -> BTreeMap<&'static str, f64> {
    let mut s = crate::financial_stats(ctx, run, costs);
    markets::annualise(&mut s, markets::bars_per_year(&ctx.r[0].ticker));
    s
}
fn gross_stats(ctx: &Ctx, run: &crate::Run) -> BTreeMap<&'static str, f64> {
    let mut s = crate::stats(ctx, run, 0.);
    markets::annualise(&mut s, markets::bars_per_year(&ctx.r[0].ticker));
    s
}
/// The run plus the simulator's equity-level events (risk pauses and stops) and, in money mode, the money ledger whose
/// effective-price trades the run then holds (cost 0: costs are inside the prices).
fn backtest(ctx: &Ctx, c: &Compiled, d: &Bars, from: usize, to: usize, cost: f64, money: Option<&ledger::Money>) -> Result<(crate::Run, Vec<String>, Option<ledger::Booked>), LabError> {
    let (real, events) = simulate_events(c, d, from, to);
    let real = if c.exec.tick_size > 0. { ledger::snap_ticks(real, c.exec.tick_size) } else { real }; // AmiBroker Tick size
    trade_validation::check(&real, from, to, cost)?;
    let dates: Vec<String> = ctx.r.iter().map(|r| r.date.clone()).collect();
    if money.is_some_and(|m| m.limits.max_volume_pct.is_some()) && d.volume.is_none() {
        return Err(LabError::NeedsData("execution.limits.max_volume_pct needs volume, which is missing on some admitted bars".into()));
    }
    let span = ledger::Span { close: &ctx.c, volume: d.volume.as_deref(), from, to };
    let booked = money.map(|m| ledger::book(&real, &dates, m, &span));
    let trades = booked.as_ref().map_or(real, |b| b.trades());
    // cash: no leverage, so the book's own check applies; on account margin a position may exceed equity (the raw trades were checked above)
    if booked.is_some() && money.is_some_and(|m| m.limits.margin_pct >= 100.) { trade_validation::check(&trades, from, to, 0.)?; }
    let equity = match (&booked, money) {
        (Some(b), Some(m)) => b.equity(&ctx.c, &dates, from, to, m, true),
        _ => crate::equity_curve(&ctx.c, &trades, from, to, cost),
    };
    if equity.iter().any(|v| !v.is_finite() || *v <= 0.) { return Err(LabError::Invalid("calculation_unavailable: nonpositive or nonfinite equity; no result is published".into())); }
    let bh = (from..=to).map(|k| ctx.c[k] / ctx.c[from]).collect();
    Ok((crate::Run { trades, from, to, equity, bh, source_account: None, bandy_account: None, unit_exposure_report: None, option_account: None }, events, booked))
}
/// Money mode on a cash instrument: capital, sizing and the declared costs (futures add their contract in lab/futures.rs).
fn money_of(spec: &Spec, q: &Query, market: Option<&str>) -> Result<Option<ledger::Money>, LabError> {
    let Some(capital) = spec.exec.capital else { return Ok(None) };
    if ["cost", "slippage"].iter().any(|k| q.contains_key(*k)) {
        return Err(LabError::Invalid("money mode takes costs from execution.costs; remove the cost and slippage query".into()));
    }
    // shortcut: Trade retains "reverse", not the originating order kind; admit money limit reversals once that metadata is preserved.
    let limit = |x: &rules::Exec| x.limit.is_some() || x.limit_at.is_some();
    if spec.short_side.as_ref().is_some_and(|s| spec.exec.opposite == rules::Opposite::Reverse && (limit(&spec.exec) || limit(&s.exec))) {
        return Err(LabError::Unsupported("money mode cannot classify a resting limit reversal's fill costs; use unit mode or opposite exit/ignore until reversal order metadata is retained".into()));
    }
    let shorts = spec.exec.short || spec.short_side.is_some();
    if shorts && market == Some("us") && spec.exec.costs.borrow_pct_year.is_none() {
        return Err(LabError::Invalid("declare execution.costs.borrow_pct_year (0 is allowed) for US short positions in money mode".into()));
    }
    if spec.exec.costs.slip_ticks > 0. { return Err(LabError::Invalid("slippage in ticks needs a futures contract's tick size; use bps".into())); }
    Ok(Some(ledger::Money { capital, sizing: spec.exec.sizing, costs: spec.exec.costs.clone(), contract: ledger::CASH, limits: spec.exec.limits.unwrap_or_else(rules::Limits::none) }))
}
/// The instrument's market code ("in" / "us"; None for gl instruments).
fn market_of(selected: &crate::SelectedContext, ticker: &str) -> Option<&'static str> {
    markets::gl(ticker).map_or_else(|| selected.evidence().map(|e| e.market().code()).or_else(|| crate::preloaded_market(ticker).map(|m| m.code())), |_| None)
}
/// AmiBroker "Pad and align all data to reference symbol" (execution.pad_align): the run's bars on the market benchmark's
/// calendar (series::pad_align), each bar's source row, and the disclosure for execution_model.events. None when off.
pub(super) fn padded(st: &State, spec: &Spec, selected: &crate::SelectedContext, ctx: &Ctx) -> Result<Option<(Vec<crate::Row>, Vec<Option<usize>>, String)>, LabError> {
    if !spec.exec.pad_align { return Ok(None); }
    let reference = match market_of(selected, &ctx.r[0].ticker) { Some("in") => crate::BENCHMARK, Some("us") => "SPX",
        _ => return Err(LabError::Unsupported("execution.pad_align needs the market's reference index (NIFTY 50 or SPX); this market has none".into())) };
    let calendar = st.ctxs.get(reference).map(|c| c.r).filter(|r| !r.is_empty()).ok_or_else(|| LabError::NeedsData(format!("execution.pad_align: the reference {reference} is not loaded")))?;
    let (rows, src, n_pad, n_drop) = series::pad_align(ctx.r, calendar);
    if rows.is_empty() { return Err(LabError::NeedsData(format!("execution.pad_align: no {reference} date falls inside this instrument's data"))); }
    Ok(Some((rows, src, format!("Pad and align to {reference}: {n_pad} bars padded with the previous close, {n_drop} dropped"))))
}
/// The padded bars as a context; extra series follow their source rows (NaN on a padded bar).
pub(super) fn padded_ctx<'r>(held: &Ctx, rows: &'r [crate::Row], src: &[Option<usize>]) -> Ctx<'r> {
    let mut c = Ctx::new(rows, &[], &[]);
    c.extra = held.extra.iter().map(|(k, v)| (k.clone(), src.iter().map(|s| s.map_or(f64::NAN, |i| v[i])).collect())).collect();
    c
}
/// The execution model of this spec on this instrument (spec §7); refusals are typed needs_data.
fn model_of(selected: &crate::SelectedContext, ticker: &str, kind: &str, spec: &Spec) -> Result<super::model::Model, LabError> {
    super::model::model(kind, market_of(selected, ticker), spec.exec.short || spec.short_side.is_some(), spec.exec.capital.is_some())
}
fn money_json(b: &ledger::Booked, m: &ledger::Money, currency: Option<&str>) -> String {
    // only specs with execution.limits get the limits block, so every other money body is unchanged
    let limits = if m.limits == rules::Limits::none() { String::new() } else {
        // margin_interest only when a margin rate is set, so every other limits body is unchanged
        let loan = if m.limits.margin_rate_pct > 0. { format!(",\"margin_interest\":{}", crate::num(b.margin_interest)) } else { String::new() };
        format!(",\"limits\":{{\"interest\":{}{loan},\"skip_reasons\":{{{}}},\"basis\":\"{}\"}}", crate::num(b.interest_earned),
            b.skip_reasons.iter().map(|(k, n)| format!("\"{k}\":{n}")).collect::<Vec<_>>().join(","), esc(ledger::LIMITS_BASIS))
    };
    format!("{{\"capital\":{},\"final_equity\":{},\"costs_total\":{},\"skipped\":{},\"currency\":{},\"basis\":\"{}\"{limits}}}", crate::num(m.capital), crate::num(b.final_equity),
        crate::num(b.costs_total), b.skipped, currency.map_or("null".into(), |c| format!("\"{c}\"")), esc(ledger::BASIS))
}

fn gross_run(ctx: &Ctx, run: &crate::Run, booked: Option<&ledger::Booked>, money: Option<&ledger::Money>) -> crate::Run {
    match (booked, money) {
        (Some(b), Some(m)) => crate::Run { trades: b.fills(), from: run.from, to: run.to,
            equity: b.equity(&ctx.c, &ctx.r.iter().map(|r| r.date.clone()).collect::<Vec<_>>(), run.from, run.to, m, false), bh: run.bh.clone(), source_account: None, bandy_account: None, unit_exposure_report: None, option_account: None },
        _ => crate::Run { trades: run.trades.clone(), from: run.from, to: run.to,
            equity: crate::equity_curve(&ctx.c, &run.trades, run.from, run.to, 0.), bh: run.bh.clone(), source_account: None, bandy_account: None, unit_exposure_report: None, option_account: None },
    }
}
fn result_stats(ctx: &Ctx, run: &crate::Run, costs: accounting::Costs, booked: Option<&ledger::Booked>, money: Option<&ledger::Money>) -> BTreeMap<&'static str, f64> {
    if money.is_none() { return lab_stats(ctx, run, costs); }
    let mut stats = gross_stats(ctx, run);
    let gross = gross_stats(ctx, &gross_run(ctx, run, booked, money));
    for (key, source) in [("gross_total_return", "total_return"), ("gross_avg_trade", "avg_trade"), ("gross_win_rate", "win_rate"), ("gross_sharpe", "sharpe"), ("gross_max_dd", "max_dd")] { stats.insert(key, gross[source]); }
    stats.insert("avg_net_pnl", stats["avg_trade"]);
    if let (Some(booked), Some(money)) = (booked, money) {
        let slip: f64 = booked.rows.iter().flat_map(|r| r.entries.iter().chain(&r.exits)).map(|f| f.slippage).sum();
        stats.insert("ledger_gross_return", gross["total_return"]);
        stats.insert("fee_return", (booked.costs_total - slip) / money.capital);
        stats.insert("slippage_return", slip / money.capital);
    }
    stats
}
fn declared_costs_json(m: &ledger::Money) -> String {
    let c = &m.costs;
    format!("{{\"commission\":{{\"per_order\":{},\"per_unit\":{},\"pct\":{}}},\"slippage\":{{\"bps\":{}}},\"borrow_pct_year\":{},\"taxes\":{},\"basis\":\"{}\"}}",
        crate::num(c.per_order), crate::num(c.per_unit), crate::num(c.pct), crate::num(c.slip_bps), c.borrow_pct_year.map_or("null".into(), crate::num),
        arr(c.taxes.iter().map(|(name, pct, side)| format!("{{\"name\":\"{}\",\"pct\":{},\"side\":\"{}\"}}", esc(name), crate::num(*pct), match side { rules::TaxSide::Buy => "buy", rules::TaxSide::Sell => "sell", rules::TaxSide::Both => "both" }))), esc(ledger::BASIS))
}
fn ledger_json(b: &ledger::Booked, ctx: &Ctx) -> String {
    let fills = |v: &[ledger::Fill]| arr(v.iter().map(|f| format!("{{\"date\":\"{}\",\"px\":{},\"qty\":{},\"commission\":{},\"taxes\":{},\"slippage\":{}}}", ctx.r[f.i].date,
        crate::num(f.px), crate::num(f.qty), crate::num(f.commission), crate::num(f.taxes), crate::num(f.slippage))));
    arr(b.rows.iter().map(|r| format!("{{\"entry\":\"{}\",\"qty\":{},\"notional\":{},\"costs\":{},\"borrow\":{},\"pnl\":{},\"open\":{},\"entries\":{},\"exits\":{}}}",
        ctx.r[r.fills.legs[0].i].date, crate::num(r.qty), crate::num(r.notional), crate::num(r.costs), crate::num(r.borrow), crate::num(r.pnl), r.fills.open, fills(&r.entries), fills(&r.exits))))
}

/// `,"adaptations":[…]` when a response reads future session dates (spec §5.2); empty otherwise, so every other body is unchanged.
fn adaptations<'a>(mut ids: impl Iterator<Item = &'a str>) -> String {
    if !ids.any(|id| super::periods::USES_SESSION_DATES.contains(&id)) { return String::new(); }
    format!(",\"adaptations\":[\"{}\"]", esc(super::periods::SESSION_DATES_NOTE))
}
fn futures_refusal(spec: &Spec, q: &Query) -> Result<(),LabError> {
    if spec.exec.futures.is_some() { return Err(LabError::NeedsData(format!("futures contract data for {} is not admitted: lot, tick, point value, margin, expiry, scheduled calendar and contract OHLC are required; a cash index or ETF is never used as a stand-in",q.get("ticker").map_or("the instrument",String::as_str)))); }
    Ok(())
}
pub fn run_json(st: &State, q: &Query) -> Result<String, LabError> {
    if q.get("split").is_some_and(|s| !s.is_empty()) { return Err(LabError::Invalid("split applies to /api/lab/optimize only".into())); }
    run_spec(st, q, spec_of(q)?)
}
/// One run of an already parsed (or optimizer-applied) spec; walk-forward runs each out-of-sample window through it.
pub(super) fn run_spec(st: &State, q: &Query, spec: Spec) -> Result<String, LabError> {
    if spec.portfolio.is_some() { return portfolio_run_json(st,q,&spec,markets::engine_market().as_deref()==Some("gl")); }
    futures_refusal(&spec,q)?;
    let origin = origin_of(&spec)?;
    let q = &with_spec_cost(q, &spec)?;
    let selected = select(st, q)?;
    let held = selected.context(st);
    let pad = padded(st, &spec, &selected, &held)?;
    let pad_ctx = pad.as_ref().map(|(rows, src, _)| padded_ctx(&held, rows, src));
    let ctx: &Ctx = pad_ctx.as_ref().unwrap_or(&*held);
    let (kind, instrument) = instrument(&selected, &ctx.r[0].ticker);
    known_type(kind, &ctx.r[0].ticker)?;
    let (others, foreign_json) = others_for(st, &ctx.r[0].ticker, &spec.foreign_locations(), close_fills(&spec), false)?;
    origin_admits(origin.as_ref(), kind, &ctx.r[0].ticker)?;
    let (label, origin) = labelled(origin.as_ref(), spec.origin.as_ref().map_or(0, |o| o.version));
    let (from, to, start) = window(ctx, q)?;
    bars_clean(ctx, from, to)?;
    let model = model_of(&selected, &ctx.r[0].ticker, kind, &spec)?;
    let money = money_of(&spec, q, market_of(&selected, &ctx.r[0].ticker))?;
    let costs = accounting::Costs::parse(q)?;
    costs.ensure_rollover()?;
    let cost = costs.side_rate();
    let fr = Frames::new(Bars::from_ctx(ctx));
    let mut compiled = series::compile_with(&spec, &fr, &others, &mut Cache::new())?;
    compiled.cost = cost;
    let (run, mut events, booked) = backtest(ctx, &compiled, &fr.d, from, to, cost, money.as_ref())?;
    if let Some(p) = &pad { events.insert(0, p.2.clone()); } // first, so the event cap never hides it
    let gross = gross_run(ctx, &run, booked.as_ref(), money.as_ref());
    let shown = booked.as_ref().map_or_else(|| run.trades.clone(), ledger::Booked::fills);
    // scale-in legs and scale-out pieces as [date, price, weight / fraction] (only when a trade has more than one)
    let pieces = |key: &str, v: Vec<(usize, f64, f64)>| if v.len() < 2 { String::new() } else {
        format!(",\"{key}\":{}", arr(v.iter().map(|(k, p, w)| format!("[\"{}\",{},{}]", ctx.r[*k].date, crate::num(*p), crate::num(*w)))))
    };
    // AmiBroker report inputs: one row per shown trade (fill prices, effective return, money profit × initial equity: the
    // ledger P&L in money mode, otherwise the compounded equity before the trade × its return).
    let mut before = 1.;
    let report_trades: Vec<report::ReportTrade> = shown.iter().zip(&run.trades).enumerate().map(|(k, (t, effective))| {
        let ret = effective.ret(cost);
        let profit = match (&booked, &money) { (Some(b), Some(m)) => b.rows[k].pnl / m.capital, _ => before * ret };
        before += profit;
        report::ReportTrade { dir: t.dir, entry_i: t.legs[0].i, exit_i: t.exit_i, entry_px: t.legs[0].px, ret, profit }
    }).collect();
    let trades = arr(shown.iter().zip(&run.trades).zip(&report_trades).map(|((t, effective), rt)| { let (mae, mfe) = report::excursion(&ctx.r, rt); format!(
        "{{\"entry\":\"{}\",\"exit\":\"{}\",\"entry_i\":{},\"exit_i\":{},\"entry_px\":{},\"exit_px\":{},\"ret\":{},\"bars\":{},\"open\":{},\"signal\":\"{}\",\"reason\":\"{}\",\"dir\":{},\"entry_kind\":\"{}\",\"mae\":{},\"mfe\":{}{}{}{}}}",
        ctx.r[t.legs[0].i].date, ctx.r[t.exit_i].date, t.legs[0].i - from, t.exit_i - from, crate::num(t.legs[0].px), crate::num(t.exit_px),
        crate::num(effective.ret(cost)), t.exit_i - t.legs[0].i, t.open, ctx.r[t.signal_i].date, esc(&t.reason), t.dir, t.entry_kind, crate::num(mae), crate::num(mfe),
        pieces("legs", t.legs.iter().map(|l| (l.i, l.px, l.w)).collect()), pieces("fills", t.exits.clone()),
        // the position's total weight (unit sizing, scale-in), only when it is not one unit
        if t.deployed() == 1. { String::new() } else { format!(",\"weight\":{}", crate::num(t.deployed())) }) }));
    let money_block = match (&booked, &money) {
        (Some(b), Some(m)) => format!(",\"money\":{},\"ledger\":{}", money_json(b, m, market_of(&selected, &ctx.r[0].ticker).map(|c| if c == "in" { "INR" } else { "USD" })), ledger_json(b, ctx)),
        _ => String::new(),
    };
    let stats = result_stats(ctx, &run, costs, booked.as_ref(), money.as_ref());
    // AmiBroker's calendar (365-day years, actual bars per year) and exposure for the report only: `stats` keeps the shared definitions
    let years = report::years(&ctx.r, from, to);
    let report = report::report_json(&ctx.r, &run.equity, &report_trades, report::car(&run.equity, years), report::exposure(&report_trades, from, to),
        run.equity.len() as f64 / years, &run.bh, report::car(&run.bh, years), spec.report.unwrap_or(report::DEFAULT_RATES));
    let cost_json = money.as_ref().map_or_else(|| costs.json(), declared_costs_json);
    Ok(format!(
        "{{\"label\":\"{}\",\"kind\":\"lab_simulation\",\"origin\":{origin},\"instrument\":{instrument}{foreign_json},\"execution_model\":{},\"from\":\"{}\",\"to\":\"{}\",\"start\":{start},\"dates\":{},\"close\":{},\"equity\":{},\"gross_equity\":{},\"bh\":{},\"trades\":{trades}{money_block},\"stats\":{},\"gross_stats\":{},\"report\":{report},\"costs\":{},\"provenance\":{}{}}}",
        esc(&label), model.json(Some(&events)), ctx.r[from].date, ctx.r[to].date, quoted((from..=to).map(|k| ctx.r[k].date.as_str())), arr((from..=to).map(|k| crate::num(ctx.c[k]))),
        arr(run.equity.iter().map(|v| crate::num(*v))), arr(gross.equity.iter().map(|v| crate::num(*v))), arr(run.bh.iter().map(|v| crate::num(*v))),
        crate::stats_json(&stats), crate::stats_json(&gross_stats(ctx, &gross)), cost_json, crate::source_provenance(st, q), adaptations(rules::series_ids(&spec).iter().map(String::as_str))))
}
pub fn optimize_json(st: &State, q: &Query) -> Result<String, LabError> { optimize_since(st, q, std::time::Instant::now()) }
/// The optimizer with its OPT_SECONDS clock started at `t0` (walk-forward shares one clock across all its windows).
/// `mode=individual` (AmiBroker Individual Optimize) optimizes each member of a portfolio universe on its own.
pub(super) fn optimize_since(st: &State, q: &Query, t0: std::time::Instant) -> Result<String, LabError> {
    let spec = spec_of(q)?;
    match q.get("mode").map_or("", String::as_str) {
        "" => {}
        "individual" => return individual_optimize_json(st, q, spec, t0),
        _ => return Err(LabError::Invalid("mode must be individual (or absent)".into())),
    }
    optimize_spec(st, q, spec, t0)
}
fn optimize_spec(st: &State, q: &Query, spec: Spec, t0: std::time::Instant) -> Result<String, LabError> {
    if spec.portfolio.is_some() { return portfolio_optimize_json(st,q,&spec,markets::engine_market().as_deref()==Some("gl"),t0); }
    futures_refusal(&spec,q)?;
    if spec.optimize.is_empty() { return Err(LabError::Invalid("optimize lists no parameter ranges".into())); }
    let origin = origin_of(&spec)?;
    let q = &with_spec_cost(q, &spec)?;
    let selected = select(st, q)?;
    let held = selected.context(st);
    let pad = padded(st, &spec, &selected, &held)?;
    let pad_ctx = pad.as_ref().map(|(rows, src, _)| padded_ctx(&held, rows, src));
    let ctx: &Ctx = pad_ctx.as_ref().unwrap_or(&*held);
    let (kind, instrument) = instrument(&selected, &ctx.r[0].ticker);
    known_type(kind, &ctx.r[0].ticker)?;
    let (others, foreign_json) = others_for(st, &ctx.r[0].ticker, &spec.foreign_locations(), close_fills(&spec), false)?;
    origin_admits(origin.as_ref(), kind, &ctx.r[0].ticker)?;
    let (label, origin) = labelled(origin.as_ref(), spec.origin.as_ref().map_or(0, |o| o.version));
    let (from, to, start) = window(ctx, q)?;
    bars_clean(ctx, from, to)?;
    let model = model_of(&selected, &ctx.r[0].ticker, kind, &spec)?;
    let money = money_of(&spec, q, market_of(&selected, &ctx.r[0].ticker))?;
    let costs = accounting::Costs::parse(q)?;
    costs.ensure_rollover()?;
    let cost = costs.side_rate();
    let dates: Vec<&str> = ctx.r.iter().map(|r| r.date.as_str()).collect();
    let split = match q.get("split").filter(|s| !s.is_empty()) {
        None => None,
        Some(s) => Some(Some(dates.partition_point(|d| *d < s.as_str())).filter(|&k| k > from + 20 && k + 20 < to)
            .ok_or_else(|| LabError::Invalid("split must leave more than 20 bars on each side of the range".into()))?),
    };
    let fr = Frames::new(Bars::from_ctx(ctx));
    let dims = &spec.optimize;
    let combos: usize = dims.iter().map(|d| d.values.len()).product();
    let metrics = |cache: &mut Cache, s: &Spec| -> Result<Vec<f64>, LabError> {
        rules::validate_applied(s)?;
        let mut c = series::compile_with(s, &fr, &others, cache)?;
        c.cost = cost;
        let money = money_of(s, q, market_of(&selected, &ctx.r[0].ticker))?;
        let (run, _, booked) = backtest(ctx, &c, &fr.d, from, to, cost, money.as_ref())?;
        let stats = result_stats(ctx, &run, costs, booked.as_ref(), money.as_ref());
        let mut out: Vec<f64> = crate::FULL_COLS.iter().map(|k| stats[k]).collect();
        let mut ins = Vec::new();
        for (n, (a, b)) in split.map(|s| [(from, s - 1), (s, to)]).into_iter().flatten().enumerate() {
            let (run, _, booked) = backtest(ctx, &c, &fr.d, a, b, cost, money.as_ref())?;
            let part = result_stats(ctx, &run, costs, booked.as_ref(), money.as_ref());
            out.extend(crate::PART_COLS.iter().map(|k| part[k]));
            if n == 0 { ins = crate::FULL_COLS.iter().map(|k| part[k]).collect(); }
        }
        if split.is_some() { out.extend(extra_rank_cols().iter().map(|k| ins[crate::FULL_COLS.iter().position(|c| c == k).unwrap()])); } // visible is_ ranked values
        out.extend(ins); // trailing in-sample copy of FULL_COLS: ranking basis only, never displayed
        Ok(out)
    };
    let base = metrics(&mut Cache::new(), &spec)?;
    let width = base.len();
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get()).min(combos);
    // Rows in mixed-radix order (last dimension fastest), as the book optimizer (main.rs optimize()).
    let (stop, done) = (AtomicBool::new(false), Arc::new(AtomicUsize::new(0)));
    #[cfg(test)]
    LAST_DONE.with(|d| *d.borrow_mut() = Some(done.clone()));
    let rows_v: Vec<Vec<f64>> = std::thread::scope(|sc| -> Result<Vec<Vec<f64>>, LabError> {
        let hs: Vec<_> = (0..threads).map(|t| {
            let (metrics, spec, stop, done) = (&metrics, &spec, &stop, &done);
            sc.spawn(move || {
                let mut cache = Cache::new();
                (t..combos).step_by(threads).map(|k| {
                    // first error or timeout in any worker stops the others promptly
                    if stop.load(Ordering::Relaxed) { return Err(LabError::Invalid(STOPPED.into())); }
                    if t0.elapsed().as_secs() >= OPT_SECONDS {
                        stop.store(true, Ordering::Relaxed);
                        return Err(LabError::Invalid(format!("optimization stopped after {OPT_SECONDS} s; narrow the grid. No partial ranking is published.")));
                    }
                    let (mut r, mut vals) = (k, vec![0.0; dims.len()]);
                    for (j, d) in dims.iter().enumerate().rev() { vals[j] = d.values[r % d.values.len()]; r /= d.values.len(); }
                    let s = rules::apply(spec, dims, &vals);
                    #[cfg(test)]
                    if vals[0] == 77. { stop.store(true, Ordering::Relaxed); return Err(LabError::Invalid("test hook: combination failed".into())); }
                    // every rule switched off: an unranked row (NaN trades never pass the floor), not a failed grid
                    if rules::switched_all_off(&s) { vals.extend(vec![f64::NAN; width]); } else {
                        vals.extend(metrics(&mut cache, &s).inspect_err(|_| stop.store(true, Ordering::Relaxed))?);
                    }
                    done.fetch_add(1, Ordering::Relaxed);
                    Ok((k, vals))
                }).collect::<Result<Vec<_>, LabError>>()
            })
        }).collect();
        let mut all: Vec<(usize, Vec<f64>)> = Vec::new();
        let mut first_err: Option<LabError> = None;
        for h in hs {
            match h.join().map_err(|_| LabError::Invalid("optimization worker failed; no result is published".into()))? {
                Ok(v) => all.extend(v),
                // keep the real failure, not the "stopped" echo workers return after the stop flag is set
                Err(e) if first_err.is_none() || first_err.as_ref().is_some_and(|f| f.message() == STOPPED) => first_err = Some(e),
                Err(_) => {}
            }
        }
        if let Some(e) = first_err { return Err(e); }
        all.sort_by_key(|r| r.0);
        Ok(all.into_iter().map(|r| r.1).collect())
    })?;
    // Ranking exactly as the book optimizer (main.rs optimize(): trade-rate floor, outcome columns, conclude_rank).
    let nd = dims.len();
    let ndisp = crate::FULL_COLS.len() + split.map_or(0, |_| 2 * crate::PART_COLS.len() + extra_rank_cols().len());
    let ti = crate::FULL_COLS.iter().position(|c| *c == "trades").unwrap();
    // With a split the ranking reads the in-sample window only; oos_ columns are display-only.
    let rank_rows: Vec<Vec<f64>> = rows_v.iter().map(|r| rank_basis(r, nd, ndisp, split.is_some())).collect();
    let base_rank = rank_basis(&[vec![0.0; nd], base.clone()].concat(), nd, ndisp, split.is_some());
    let yrs = crate::years(ctx, from, split.map_or(to, |s| s - 1));
    let base_tpy = base_rank[nd + ti] / yrs;
    let per_year = (base_tpy * 0.5).clamp(crate::MIN_PER_YEAR.0, crate::MIN_PER_YEAR.1);
    let min_trades = (per_year * yrs).ceil().max(crate::MIN_TRADES);
    let defs: Vec<f64> = dims.iter().map(|d| rules::current(&spec, &d.target)).collect();
    let outcome_cols = outcome_cols(nd, ndisp, split.is_some());
    let col = |name: &str| nd + crate::FULL_COLS.iter().position(|c| *c == name).unwrap();
    let (rank_json, pool) = crate::conclude_rank_with_outcomes(&rank_rows, col("trades"), &crate::RANK_STATS.map(|(k, _)| col(k)), min_trades, per_year, &defs, Some(&outcome_cols));
    let table = table_json(q, &rank_rows, nd, crate::FULL_COLS, crate::num); // ranking-period values, as ranked
    let rows_v: Vec<Vec<f64>> = rows_v.into_iter().map(|mut r| { r.truncate(nd + ndisp); r }).collect();
    let base = &base[..ndisp];
    let slim = q.get("slim").is_some_and(|v| v == "1");
    let row_s = |r: &Vec<f64>| arr(r.iter().map(|v| crate::num(*v)));
    let rows = if slim { String::new() } else { rows_v.iter().map(row_s).collect::<Vec<_>>().join(",") };
    let picked = if slim { pool.iter().map(|&i| format!("\"{i}\":{}", row_s(&rows_v[i]))).collect::<Vec<_>>().join(",") } else { String::new() };
    let mut cols: Vec<String> = dims.iter().map(|d| d.path.clone()).collect();
    cols.extend(crate::FULL_COLS.iter().map(|c| c.to_string()));
    for part in split.map(|_| ["is_", "oos_"]).into_iter().flatten() { cols.extend(crate::PART_COLS.iter().map(|c| format!("{part}{c}"))); }
    if split.is_some() { cols.extend(extra_rank_cols().iter().map(|c| format!("is_{c}"))); }
    let dims_json = arr(dims.iter().map(|d| format!("{{\"path\":\"{}\",\"values\":{}}}", esc(&d.path), arr(d.values.iter().map(|v| crate::num(*v))))));
    let money_block = money.as_ref().map_or(String::new(), |m| format!(",\"money\":{{\"capital\":{},\"basis\":\"{}\"}}", crate::num(m.capital), esc(ledger::BASIS)));
    let cost_json = money.as_ref().map_or_else(|| costs.json(), declared_costs_json);
    let mut warnings = start_warning(&start);
    if pool.is_empty() { warnings.push(format!("no combination is eligible: each needs at least {min_trades} trades")); }
    let info = info_json(t0.elapsed().as_millis(), combos, threads, &warnings);
    Ok(format!(
        "{{\"label\":\"{}\",\"kind\":\"lab_optimization\",\"origin\":{origin}{money_block},\"instrument\":{instrument}{foreign_json},\"execution_model\":{},\"from\":\"{}\",\"to\":\"{}\",\"start\":{start},\"split\":{},\"combos\":{combos},\"threads\":{threads},\"ms\":{},\"dims\":{dims_json},\"cols\":{},\"base\":{},\"ranked_on\":\"{}\",\"rank\":{rank_json},\"picked\":{{{picked}}},\"rows\":[{rows}]{table}{info},\"costs\":{},\"provenance\":{}{}}}",
        esc(&label), model.json(None), dates[from], dates[to], split.map_or("null".into(), |s| format!("\"{}\"", dates[s])), t0.elapsed().as_millis(),
        quoted(cols.iter().map(String::as_str)), arr(base.iter().map(|v| crate::num(*v))), if split.is_some() { "in_sample" } else { "full" }, cost_json, crate::source_provenance(st, q), adaptations(rules::series_ids(&spec).iter().map(String::as_str))))
}

fn catalogue_for_state(st:&State)->String{
    let markets:std::collections::BTreeSet<_>=st.ctxs.keys().filter_map(|t|symbol_market(st,t)).collect();
    let market=if markets.len()==1{markets.iter().next().copied().filter(|m|*m=="in"||*m=="us")}else{None};
    let vix=st.vix.first().filter(|r|super::foreign::VIX.contains(&r.ticker.as_str())&&st.vix.iter().all(|v|v.ticker==r.ticker)&&symbol_market(st,&r.ticker)==market&&market.is_some()&&raw_prices_clean(&st.vix).is_ok()&&super::foreign::validate_dates(&st.vix.iter().map(|r|r.date.clone()).collect::<Vec<_>>(),"volatility source").is_ok()).map(|r|r.ticker.as_str());
    let baskets=arr(["allfo","midcap100","nifty50","nse500","us_dow30","us_dow30_past","us_nasdaq100","us_nasdaq100_past","us_russell2000","us_russell2000_past","us_sp500","us_sp500_past"].iter().filter_map(|id|{
        let members=st.baskets.members(id);if members.is_empty()||members.iter().any(|m|Some(m.market.code())!=market){return None;}let available=members.len();let n=st.baskets.expected_member_count(id).unwrap_or(available).max(available);let kind=if members.iter().any(|m|m.membership_kind=="current_plus_past_snapshot"){"current_plus_past_snapshot"}else{"current_snapshot"};Some(format!("{{\"id\":\"{id}\",\"members\":{n},\"available_members\":{available},\"membership_kind\":\"{kind}\",\"supported\":{}}}",n==available&&(2..=rules::MAX_UNIVERSE).contains(&n)))
    }));
    let instruments=arr(st.ctxs.iter().filter_map(|(ticker,ctx)|{let kind=instrument_policy::selected_type(ticker,st.baskets.native_evidence(ticker).is_some());let m=symbol_market(st,ticker);if !matches!(kind,"etf"|"individual_stock")||m!=market||market.is_none()||ctx.r.is_empty()||ctx.r.iter().any(|r|r.ticker!=*ticker){return None;}Some(format!("{{\"ticker\":\"{}\",\"type\":\"{kind}\",\"market\":\"{}\"}}",esc(ticker),m.unwrap()))}));
    let capabilities=if market.is_some()&&markets::engine_market().as_deref()!=Some("gl"){format!("{{\"modes\":[\"signals\",\"rotation\"],\"max_universe\":{},\"max_universe_list\":{},\"max_open\":{},\"max_combos\":{},\"max_work\":{},\"max_source_bars\":500000,\"max_member_bars\":50000,\"max_compiled_cells\":8000000,\"max_workers\":2,\"rebalance\":[\"bar\",\"W\",\"M\",\"Q\",\"Y\"],\"baskets\":{baskets},\"instruments\":{instruments}}}",rules::MAX_UNIVERSE,rules::MAX_UNIVERSE_LIST,rules::MAX_OPEN,rules::MAX_PORTFOLIO_COMBOS,PORTFOLIO_MAX_WORK)}else{"null".into()};
    catalogue_json().replace("\"foreign_volatility\":null",&format!("\"foreign_volatility\":{}",vix.map_or("null".into(),|s|format!("\"{}\"",esc(s))))).replace("\"portfolio\":null",&format!("\"portfolio\":{capabilities}"))
}
fn portfolio_costs_json(spec:&Spec)->String{declared_costs_json(&ledger::Money{capital:spec.exec.capital.unwrap(),sizing:spec.exec.sizing,costs:spec.exec.costs.clone(),contract:ledger::CASH,limits:rules::Limits::none()})}


pub(super) const PORTFOLIO_MAX_SOURCE_BARS:usize=500_000;
pub(super) const PORTFOLIO_MAX_MEMBER_BARS:usize=50_000;
const PORTFOLIO_MAX_COMPILED_CELLS:usize=8_000_000;
const PORTFOLIO_MAX_WORKERS:usize=2;
fn finish_portfolio(body:String,deadline:std::time::Instant,opt:bool)->Result<String,LabError>{
    portfolio_deadline(deadline).map_err(|e|if opt{LabError::Invalid("portfolio optimization stopped after 300 s; narrow the grid. No partial ranking is published.".into())}else{e})?;Ok(body)
}
/// Conservative cells include evaluated tree arrays and all raw output channels; repeated leaves deliberately overcount.
fn operand_cells(o:&rules::Operand)->usize{
    use rules::{Operand,Expr};match o{
        Operand::Value(_)|Operand::BarsHeld=>0,
        Operand::SinceEntry(s)=>1+s.of.as_ref().map_or(0,operand_cells)+s.when.as_ref().map_or(0,node_cells),
        Operand::Series(s)=>1+catalogue::get(&s.id).map_or(s.formula.as_ref().map_or(0,|f|f.prog.plots.len()),|d|d.outputs.len()),
        Operand::Expr(e)=>1+match &**e{
            Expr::Bin(_,a,b)=>operand_cells(a)+operand_cells(b),Expr::Abs(a)|Expr::Ref(a,_)|Expr::Win(_,a,_)=>operand_cells(a),
            Expr::BarsSince(n)|Expr::Count(n,_)=>node_cells(n),Expr::ValueWhen{when,of,..}|Expr::Since{when,of,..}=>node_cells(when)+operand_cells(of),
            Expr::Expand(s,_)=>1+catalogue::get(&s.id).map_or(s.formula.as_ref().map_or(0,|f|f.prog.plots.len()),|d|d.outputs.len()),Expr::Compress{of,..}=>operand_cells(of),
        }
    }
}
fn node_cells(n:&rules::Node)->usize{use rules::Node;1+match n{
    Node::Cmp(c)=>operand_cells(&c.lhs)+operand_cells(&c.rhs),Node::Range{x,lo,hi,..}=>operand_cells(x)+operand_cells(lo)+operand_cells(hi),Node::Trend{x,..}=>operand_cells(x),
    Node::Off(n)=>node_cells(n),Node::All(v)|Node::Any(v)=>v.iter().map(node_cells).sum(),Node::Not(n)|Node::AtLeast{when:n,..}=>node_cells(n),Node::Then{steps,..}=>steps.iter().map(node_cells).sum(),
    Node::Setup{setup,trigger,cancel,..}=>node_cells(setup)+node_cells(trigger)+cancel.as_deref().map_or(0,node_cells),
}}
fn portfolio_resource_counts(member_bars:&[usize],other_bars:usize,units:usize)->Result<usize,LabError>{
    let refuse=|why:String|LabError::Invalid(format!("portfolio resource limit: {why}; use fewer members or shorter admitted histories (from/to does not truncate indicator warm-up)"));
    if member_bars.iter().any(|n|*n>PORTFOLIO_MAX_MEMBER_BARS){return Err(refuse(format!("at most {PORTFOLIO_MAX_MEMBER_BARS} source bars per member")));}
    let total=member_bars.iter().try_fold(other_bars,|sum,n|sum.checked_add(*n)).ok_or_else(||refuse("source-bar count overflow".into()))?;
    if total>PORTFOLIO_MAX_SOURCE_BARS{return Err(refuse(format!("at most {PORTFOLIO_MAX_SOURCE_BARS} total source bars")));}
    let cells=total.checked_mul(units.max(1)).ok_or_else(||refuse("compiled-cell count overflow".into()))?;
    if cells>PORTFOLIO_MAX_COMPILED_CELLS{return Err(refuse(format!("at most {PORTFOLIO_MAX_COMPILED_CELLS} compiled cells")));}Ok(cells)
}
fn portfolio_resources(prepared:&PreparedPortfolio,spec:&Spec)->Result<(usize,usize),LabError>{
    let bars:Vec<_>=prepared.loaded.members.iter().map(|m|m.frames.d.len()).collect();let other=prepared.others.frames.values().try_fold(prepared.loaded.dates.len(),|n,f|n.checked_add(f.d.len())).ok_or_else(||LabError::Invalid("portfolio source-bar count overflow".into()))?;
    // Reserve six arrays for protective ATR/score/marks in addition to the actual parsed tree.
    let units=6+spec.entry.iter().chain(&spec.exit).map(node_cells).sum::<usize>()+spec.portfolio.as_ref().and_then(|p|p.score.as_ref()).map_or(0,operand_cells);
    let cells=portfolio_resource_counts(&bars,other,units)?;let total=bars.iter().sum::<usize>()+other;
    let working=cells.checked_mul(8).and_then(|n|n.checked_add(total*64)).ok_or_else(||LabError::Invalid("portfolio working-set estimate overflow".into()))?;
    Ok((cells,((128*1024*1024)/working.max(1)).clamp(1,PORTFOLIO_MAX_WORKERS)))
}

const PORTFOLIO_SECONDS:u64=90;
const PORTFOLIO_MAX_WORK:usize=12_000;
const PCOLS:&[&str]=&["total_return","cagr","sharpe","sortino","max_dd","mar","trades","win_rate","profit_factor","avg_trade","exposure","t_stat","avg_bars","expectancy_ratio","monthly_thickness","avg_hold_days","avg_exposure","max_positions"];
struct PreparedPortfolio {loaded:super::universe::Loaded,from:usize,to:usize,others:series::Others,foreign_json:String}
fn portfolio_deadline(deadline:std::time::Instant)->Result<(),LabError>{if std::time::Instant::now()>=deadline{Err(LabError::Invalid(format!("portfolio run stopped after {PORTFOLIO_SECONDS} s; use fewer members or a shorter window")))}else{Ok(())}}
fn portfolio_request(q:&Query,spec:&Spec,gl:bool,opt:bool)->Result<(),LabError>{
    if gl{return Err(LabError::Unsupported("portfolio mode runs on India/US stocks and ETFs; gl instruments are provisional indices, CFDs and crypto".into()));}
    if q.contains_key("ticker"){return Err(LabError::Invalid("portfolio runs read their instruments from spec.portfolio.universe; omit ticker".into()));}
    if q.get("split").is_some_and(|s|!s.is_empty()){return Err(LabError::Invalid(if opt{"split is not available for portfolio optimizations in this phase"}else{"split applies to /api/lab/optimize only"}.into()));}
    if ["cost","slippage","rollover"].iter().any(|k|q.contains_key(*k))||spec.cost_bps.is_some(){return Err(LabError::Invalid("money mode takes costs from execution.costs; remove the cost, slippage and rollover query".into()));}
    rules::validate_applied(spec)
}
fn prepare_portfolio(st:&State,q:&Query,spec:&Spec,deadline:std::time::Instant)->Result<PreparedPortfolio,LabError>{
    portfolio_deadline(deadline)?;
    let p=spec.portfolio.as_ref().ok_or_else(||LabError::Invalid("portfolio is required".into()))?;
    let loaded=super::universe::load_with_deadline(st,&p.universe,deadline)?;
    let ctx=&st.ctxs[&loaded.reference];raw_prices_clean(ctx.r)?;
    for key in ["from","to"] {if let Some(date)=q.get(key).filter(|s|!s.is_empty()){
        if date.len()!=10||!date.bytes().enumerate().all(|(i,c)|if i==4||i==7{c==b'-'}else{c.is_ascii_digit()})||!crate::baskets::valid_date(date){return Err(LabError::Invalid(format!("{key} must be a valid YYYY-MM-DD date")));}
    }}
    if q.get("from").is_some_and(|f|!f.is_empty()&&(f<&loaded.dates[0]||f>loaded.dates.last().unwrap())){return Err(LabError::NeedsData(format!("from precedes or exceeds admitted reference coverage {} through {}",loaded.dates[0],loaded.dates.last().unwrap())));}
    if q.get("to").is_some_and(|t|!t.is_empty()&&(t>loaded.dates.last().unwrap()||t<&loaded.dates[0])){return Err(LabError::NeedsData(format!("to exceeds or precedes admitted reference coverage {} through {}",loaded.dates[0],loaded.dates.last().unwrap())));}
    let (_,mut from,mut to,_)=crate::resolve_context(ctx,q).map_err(LabError::Invalid)?;
    if q.get("from").is_none_or(|s|s.is_empty()){if let Some(first)=loaded.members.iter().filter_map(|m|m.at.iter().position(Option::is_some)).min(){from=from.max(first);}}
    if q.get("to").is_none_or(|s|s.is_empty()){if let Some(last)=loaded.members.iter().filter_map(|m|m.at.iter().rposition(Option::is_some)).max(){to=to.min(last);}}
    if from>=to{return Err(LabError::NeedsData("no member has data inside the window".into()));}
    let (others,foreign_json)=others_for(st,&loaded.reference,&spec.foreign_locations(),close_fills(spec),true)?;
    portfolio_deadline(deadline)?;
    let prepared=PreparedPortfolio{loaded,from,to,others,foreign_json};portfolio_resources(&prepared,spec)?;Ok(prepared)
}
fn portfolio_lanes<'a>(prepared:&'a PreparedPortfolio,spec:&Spec,caches:&mut[Cache],deadline:std::time::Instant,stop:Option<&AtomicBool>)->Result<(Vec<super::portfolio::Lane<'a>>,Vec<(String,String)>),LabError>{
    rules::validate_applied(spec)?;
    let p=spec.portfolio.as_ref().unwrap();let mut lanes=vec![];let mut refused=prepared.loaded.refused.clone();
    for (i,m) in prepared.loaded.members.iter().enumerate(){
        portfolio_deadline(deadline)?;if stop.is_some_and(|s|s.load(Ordering::Relaxed)){return Err(LabError::Invalid(STOPPED.into()));}
        if !(prepared.from..=prepared.to).any(|i|m.at[i].is_some()){refused.push((m.ticker.clone(),"no bars inside the window".into()));continue;}
        let others=prepared.others.with_run(&m.ticker);
        let result=(||{let c=series::compile_with(spec,&m.frames,&others,&mut caches[i])?;let score=p.score.as_ref().map(|o|series::operand_values(&m.frames,&others,&mut caches[i],o)).transpose()?;Ok::<_,LabError>((c,score))})();
        caches[i].clear(); // compiled arrays own their values; do not retain raw outputs across members or combinations
        match result {Ok((c,score))=>lanes.push(super::portfolio::Lane{m,c,score}),Err(LabError::NeedsData(reason))=>refused.push((m.ticker.clone(),reason)),Err(error)=>return Err(error)}
        portfolio_deadline(deadline)?;
    }
    if lanes.is_empty(){return Err(LabError::NeedsData(format!("no universe member is admitted for this window; {}",refused.iter().map(|(t,r)|format!("{t}: {r}")).collect::<Vec<_>>().join("; "))));}
    Ok((lanes,refused))
}
fn portfolio_book(spec:&Spec)->Result<super::portfolio::Book,LabError>{
    rules::validate_applied(spec)?;let p=spec.portfolio.as_ref().unwrap();
    if spec.exec.costs.slip_ticks>0.{return Err(LabError::Unsupported("tick slippage needs a futures contract".into()));}
    let capital=spec.exec.capital.ok_or_else(||LabError::Invalid("portfolio money mode requires execution.capital".into()))?;
    Ok(super::portfolio::Book{capital,mode:p.mode,max_open:p.max_open,worst_rank_held:p.worst_rank_held(),rebalance:p.rebalance,lowest:p.lowest,fill:spec.exec.fill,exit_fill:spec.exec.exit_fill,size:if spec.exec.sizing==rules::Sizing::Full{rules::Sizing::PctEquity(100./p.max_open as f64)}else{spec.exec.sizing},costs:spec.exec.costs.clone()})
}
fn portfolio_stats(ctx:&Ctx,prepared:&PreparedPortfolio,led:&super::portfolio::Ledger,capital:f64,net:bool)->BTreeMap<&'static str,f64>{
    let mut trades:Vec<_>=led.trades.iter().map(|t|t.as_trade(net)).collect();trades.sort_by_key(|t|(t.legs[0].i,t.exit_i));
    let run=crate::Run{trades,from:prepared.from,to:prepared.to,equity:if net{&led.equity}else{&led.gross_equity}.iter().map(|v|v/capital).collect(),bh:(prepared.from..=prepared.to).map(|i|ctx.c[i]/ctx.c[prepared.from]).collect(),source_account:None,bandy_account:None,unit_exposure_report:None,option_account:None};
    let mut stats=crate::stats(ctx,&run,0.);stats.insert("avg_exposure",led.invested.iter().zip(&led.equity).map(|(v,e)|v/e).sum::<f64>()/led.equity.len() as f64);stats.insert("max_positions",led.positions.iter().copied().max().unwrap_or(0) as f64);stats
}
fn membership_json(loaded:&super::universe::Loaded)->String{format!("{{\"kind\":\"{}\",\"historical_membership_verified\":false,\"note\":\"{}\"}}",loaded.membership.0,esc(loaded.membership.1))}
fn portfolio_meta(spec:&Spec,prepared:&PreparedPortfolio,admitted:usize,refused:&[(String,String)])->String{
    let p=spec.portfolio.as_ref().unwrap();format!("{{\"mode\":\"{}\",\"max_open\":{},\"worst_rank_held\":{},\"rebalance\":{},\"rank\":\"{}\",\"universe\":{{\"basket\":{},\"members\":{},\"admitted\":{admitted},\"refused\":{}}}}}",if p.mode==rules::Mode::Rotation{"rotation"}else{"signals"},p.max_open,p.worst_rank_held(),p.rebalance.map_or("null".into(),|v|format!("\"{}\"",["","W","M","Q","Y"][v])),if p.lowest{"lowest"}else{"highest"},prepared.loaded.basket.as_ref().map_or("null".into(),|s|format!("\"{}\"",esc(s))),prepared.loaded.members.len()+prepared.loaded.refused.len(),arr(refused.iter().map(|(t,r)|format!("{{\"ticker\":\"{}\",\"reason\":\"{}\"}}",esc(t),esc(r)))))
}
fn portfolio_model(events:Option<&[String]>)->String{format!("{{\"kind\":\"cash\",\"mode\":\"money\",\"executable\":true,\"notes\":[\"User-declared costs; not a verified statutory schedule. Quantities use admitted prices whose corporate-action adjustment is not established; not verified as-traded quantities. Terminal member coverage can cause data_end liquidation; this is a historical coverage adaptation, not live-causal delisting knowledge. Gross revalues the same booked fills and quantities before declared costs.\"],\"events\":{}}}",events.map_or("null".into(),|v|quoted(v.iter().map(String::as_str))))}
fn portfolio_run_json(st:&State,q:&Query,spec:&Spec,gl:bool)->Result<String,LabError>{
    let deadline=std::time::Instant::now()+std::time::Duration::from_secs(PORTFOLIO_SECONDS);portfolio_request(q,spec,gl,false)?;
    let prepared=prepare_portfolio(st,q,spec,deadline)?;let mut caches:Vec<_>=(0..prepared.loaded.members.len()).map(|_|Cache::new()).collect();let (lanes,refused)=portfolio_lanes(&prepared,spec,&mut caches,deadline,None)?;let book=portfolio_book(spec)?;
    let led=super::portfolio::run(&lanes,&book,&prepared.loaded.dates,prepared.from,prepared.to,deadline,None)?;portfolio_deadline(deadline)?;
    let ctx=&st.ctxs[&prepared.loaded.reference];let stats=portfolio_stats(ctx,&prepared,&led,book.capital,true);let gross=portfolio_stats(ctx,&prepared,&led,book.capital,false);
    let dates=&prepared.loaded.dates;let from=prepared.from;let to=prepared.to;
    let trades=arr(led.trades.iter().map(|t|format!("{{\"ticker\":\"{}\",\"entry\":\"{}\",\"exit\":\"{}\",\"entry_i\":{},\"exit_i\":{},\"entry_px\":{},\"exit_px\":{},\"qty\":{},\"notional\":{},\"costs\":{},\"ret\":{},\"bars\":{},\"open\":{},\"signal\":\"{}\",\"entry_signal\":\"{}\",\"reason\":\"{}\",\"rank\":{},\"score\":{},\"dir\":1,\"entry_kind\":\"{}\"}}",esc(&lanes[t.lane].m.ticker),dates[t.entry_i],dates[t.exit_i],t.entry_i-from,t.exit_i-from,full(t.entry_px),full(t.exit_px),full(t.qty),full(t.entry_px*t.qty),full(t.entry_cost+t.exit_cost),full(t.pnl()/t.equity_at_signal),t.exit_i-t.entry_i,t.open,dates[t.exit_signal_i],dates[t.signal_i],esc(&t.reason),t.rank,full(t.score),t.kind)));
    let provenance=arr(lanes.iter().map(|l|crate::source_provenance(st,&Query::from([("ticker".into(),l.m.ticker.clone())]))));
    let costs=portfolio_costs_json(spec);
    let body=format!("{{\"label\":\"{LABEL}\",\"kind\":\"lab_portfolio_simulation\",\"origin\":null,\"portfolio\":{},\"membership\":{},\"execution_model\":{},\"reference\":{{\"ticker\":\"{}\",\"type\":\"index\"}}{},\"from\":\"{}\",\"to\":\"{}\",\"dates\":{},\"equity\":{},\"gross_equity\":{},\"cash\":{},\"exposure\":{},\"positions\":{},\"bh\":{},\"trades\":{trades},\"skipped\":{{\"no_slot\":{},\"no_cash\":{},\"no_bar\":{},\"data_end\":{},\"final_bar\":{}}},\"stats\":{},\"gross_stats\":{},\"money\":{{\"capital\":{},\"final_equity\":{},\"costs_total\":{},\"currency\":\"{}\"}},\"costs\":{costs},\"provenance\":{provenance}{}}}",portfolio_meta(spec,&prepared,lanes.len(),&refused),membership_json(&prepared.loaded),portfolio_model(Some(&led.events)),esc(&prepared.loaded.reference),prepared.foreign_json,dates[from],dates[to],quoted(dates[from..=to].iter().map(String::as_str)),arr(led.equity.iter().map(|v|full(v/book.capital))),arr(led.gross_equity.iter().map(|v|full(v/book.capital))),arr(led.cash.iter().map(|v|full(*v))),arr(led.invested.iter().map(|v|full(*v))),arr(led.positions.iter().map(|v|v.to_string())),arr((from..=to).map(|i|full(ctx.c[i]/ctx.c[from]))),led.skipped.no_slot,led.skipped.no_cash,led.skipped.no_bar,led.skipped.data_end,led.skipped.final_bar,crate::stats_json(&stats),crate::stats_json(&gross),full(book.capital),full(*led.equity.last().unwrap()),full(led.costs_total),if symbol_market(st,&prepared.loaded.reference)==Some("in"){"INR"}else{"USD"},adaptations(rules::series_ids(spec).iter().map(String::as_str)));
    finish_portfolio(body,deadline,false)
}
fn portfolio_work(members:usize,combos:usize)->Result<(),LabError>{
    if combos==0||combos>rules::MAX_PORTFOLIO_COMBOS{return Err(LabError::Invalid("portfolio optimize allows at most 200 combinations".into()));}
    let work=members.checked_mul(combos).ok_or_else(||LabError::Invalid("members × combinations overflows the 12000 work limit".into()))?;
    if work>PORTFOLIO_MAX_WORK{return Err(LabError::Invalid(format!("members × combinations is {work}; the limit is 12000 — use fewer members or a smaller grid")));}Ok(())
}
fn portfolio_optimize_json(st:&State,q:&Query,spec:&Spec,gl:bool,t0:std::time::Instant)->Result<String,LabError>{
    portfolio_request(q,spec,gl,true)?;if spec.optimize.is_empty(){return Err(LabError::Invalid("optimize lists no parameter ranges".into()));}
    let deadline=t0+std::time::Duration::from_secs(OPT_SECONDS);let timeout=|e:LabError|if e.message().contains("stopped after 90 s"){LabError::Invalid("portfolio optimization stopped after 300 s; narrow the grid. No partial ranking is published.".into())}else{e};
    let combos=spec.optimize.iter().try_fold(1usize,|n,d|n.checked_mul(d.values.len())).ok_or_else(||LabError::Invalid("portfolio combination count overflow".into()))?;portfolio_work(0,combos)?;
    let prepared=prepare_portfolio(st,q,spec,deadline).map_err(timeout)?;portfolio_work(prepared.loaded.members.len()+prepared.loaded.refused.len(),combos)?;
    let mut caches:Vec<_>=(0..prepared.loaded.members.len()).map(|_|Cache::new()).collect();let (base_lanes,refused)=portfolio_lanes(&prepared,spec,&mut caches,deadline,None).map_err(timeout)?;
    let cohort:Vec<_>=base_lanes.iter().map(|l|l.m.ticker.clone()).collect();let base_book=portfolio_book(spec)?;let base_ledger=super::portfolio::run(&base_lanes,&base_book,&prepared.loaded.dates,prepared.from,prepared.to,deadline,None).map_err(timeout)?;
    let ctx=&st.ctxs[&prepared.loaded.reference];let base_stats=portfolio_stats(ctx,&prepared,&base_ledger,base_book.capital,true);let base:Vec<_>=PCOLS.iter().map(|k|base_stats[k]).collect();let provenance=arr(base_lanes.iter().map(|l|crate::source_provenance(st,&Query::from([("ticker".into(),l.m.ticker.clone())]))));drop(base_lanes);drop(base_ledger);drop(caches);let (_,worker_limit)=portfolio_resources(&prepared,spec)?;
    let stop=AtomicBool::new(false);let done=Arc::new(AtomicUsize::new(0));let errors=std::sync::Mutex::new(None::<LabError>);let threads=std::thread::available_parallelism().map_or(1,|n|n.get()).min(worker_limit).min(combos);let dims=&spec.optimize;
    #[cfg(test)] LAST_DONE.with(|d|*d.borrow_mut()=Some(done.clone()));
    let rows=std::thread::scope(|scope|->Result<Vec<Vec<f64>>,LabError>{
        let handles:Vec<_>=(0..threads).map(|worker|{let (stop,done,errors,prepared,cohort,timeout)=(&stop,&done,&errors,&prepared,&cohort,&timeout);scope.spawn(move||{
            let mut out=vec![];let mut caches:Vec<_>=(0..prepared.loaded.members.len()).map(|_|Cache::new()).collect();
            for k in (worker..combos).step_by(threads){if stop.load(Ordering::Relaxed){break;}let mut index=k;let mut vals=vec![0.;dims.len()];for (j,d) in dims.iter().enumerate().rev(){vals[j]=d.values[index%d.values.len()];index/=d.values.len();}
                let result=(||{portfolio_deadline(deadline)?;
                    #[cfg(test)] if vals[0]==77.{return Err(LabError::Invalid("test hook: portfolio combination failed; no partial ranking is published".into()));}
                    let candidate=rules::apply(spec,dims,&vals);if rules::switched_all_off(&candidate){vals.extend(vec![f64::NAN;PCOLS.len()]);return Ok(vals);}let (lanes,_)=portfolio_lanes(prepared,&candidate,&mut caches,deadline,Some(stop))?;
                    if lanes.iter().map(|l|l.m.ticker.clone()).collect::<Vec<_>>()!=*cohort{return Err(LabError::NeedsData("a combination changed the admitted member cohort; no partial ranking is published".into()));}
                    let book=portfolio_book(&candidate)?;let led=super::portfolio::run(&lanes,&book,&prepared.loaded.dates,prepared.from,prepared.to,deadline,Some(stop))?;let stats=portfolio_stats(ctx,prepared,&led,book.capital,true);portfolio_deadline(deadline)?;vals.extend(PCOLS.iter().map(|key|stats[key]));Ok(vals)})();
                match result {Ok(row)=>{out.push((k,row));done.fetch_add(1,Ordering::Relaxed);},Err(e)=>{let e=timeout(e);let cancellation=e.message()==STOPPED||e.message().contains("cancelled after another");if !cancellation{let mut first=errors.lock().unwrap_or_else(|p|p.into_inner());if first.is_none(){*first=Some(e);}}stop.store(true,Ordering::Relaxed);break;}}
            }out
        })}).collect();let mut rows=vec![];for h in handles{match h.join(){Ok(v)=>rows.extend(v),Err(_)=>{stop.store(true,Ordering::Relaxed);let mut first=errors.lock().unwrap_or_else(|p|p.into_inner());if first.is_none(){*first=Some(LabError::Invalid("portfolio optimization worker failed; no result is published".into()));}}}}
        if let Some(error)=errors.lock().unwrap_or_else(|p|p.into_inner()).take(){return Err(error);}if rows.len()!=combos{return Err(LabError::Invalid("portfolio optimization cancelled; no partial ranking is published".into()));}rows.sort_by_key(|r|r.0);Ok(rows.into_iter().map(|r|r.1).collect())
    })?;
    portfolio_deadline(deadline).map_err(timeout)?;
    let nd=dims.len();let col=|name:&str|nd+PCOLS.iter().position(|k|*k==name).unwrap();let years=crate::years(ctx,prepared.from,prepared.to);let per_year=(base_stats["trades"]/years*0.5).clamp(crate::MIN_PER_YEAR.0,crate::MIN_PER_YEAR.1);let min_trades=(per_year*years).ceil().max(crate::MIN_TRADES);let defs:Vec<_>=dims.iter().map(|d|rules::current(spec,&d.target)).collect();let (rank,pool)=crate::conclude_rank_with_outcomes(&rows,col("trades"),&crate::RANK_STATS.map(|(k,_)|col(k)),min_trades,per_year,&defs,None);
    let slim=q.get("slim").is_some_and(|v|v=="1");let row=|r:&Vec<f64>|arr(r.iter().map(|v|full(*v)));let picked=if slim{pool.iter().map(|i|format!("\"{i}\":{}",row(&rows[*i]))).collect::<Vec<_>>().join(",")}else{String::new()};let shown=if slim{String::new()}else{rows.iter().map(row).collect::<Vec<_>>().join(",")};let cols=dims.iter().map(|d|d.path.as_str()).chain(PCOLS.iter().copied());
    let costs=portfolio_costs_json(spec);let table=table_json(q,&rows,nd,PCOLS,full);
    let body=format!("{{\"label\":\"{LABEL}\",\"kind\":\"lab_portfolio_optimization\",\"origin\":null,\"portfolio\":{},\"membership\":{},\"reference\":{{\"ticker\":\"{}\",\"type\":\"index\"}}{},\"execution_model\":{},\"money\":{{\"capital\":{}}},\"from\":\"{}\",\"to\":\"{}\",\"split\":null,\"combos\":{combos},\"threads\":{threads},\"ms\":{},\"dims\":{},\"cols\":{},\"base\":{},\"ranked_on\":\"full\",\"rank\":{rank},\"picked\":{{{picked}}},\"rows\":[{shown}]{table}{},\"costs\":{costs},\"provenance\":{provenance}{}}}",portfolio_meta(spec,&prepared,cohort.len(),&refused),membership_json(&prepared.loaded),esc(&prepared.loaded.reference),prepared.foreign_json,portfolio_model(None),full(base_book.capital),prepared.loaded.dates[prepared.from],prepared.loaded.dates[prepared.to],t0.elapsed().as_millis(),arr(dims.iter().map(|d|format!("{{\"path\":\"{}\",\"values\":{}}}",esc(&d.path),arr(d.values.iter().map(|v|full(*v)))))),quoted(cols),arr(base.iter().map(|v|full(*v))),info_json(t0.elapsed().as_millis(),combos,threads,&refused.iter().map(|(t,r)|format!("{t} skipped: {r}")).chain(pool.is_empty().then(||format!("no combination is eligible: each needs at least {min_trades} trades"))).collect::<Vec<_>>()),adaptations(rules::series_ids(spec).iter().map(String::as_str)));
    finish_portfolio(body,deadline,true)
}

/// Most single-symbol backtests (symbols × combinations) one Individual Optimize request may make: the portfolio work limit
/// (a portfolio spec's grid is already capped at rules::MAX_PORTFOLIO_COMBOS).
pub(super) const INDIVIDUAL_MAX_WORK: usize = PORTFOLIO_MAX_WORK;
fn individual_work(symbols: usize, combos: usize) -> Result<(), LabError> {
    let work = symbols.saturating_mul(combos);
    if work > INDIVIDUAL_MAX_WORK { return Err(LabError::Invalid(format!("Individual optimize: {symbols} symbols × {combos} combinations = {work} backtests; the limit is {INDIVIDUAL_MAX_WORK} — use fewer symbols or a smaller grid"))); }
    Ok(())
}
/// Rows of each symbol's all-results table kept in an Individual Optimize response, in grid order (the best row is separate).
pub(super) const INDIVIDUAL_TABLE_ROWS: usize = 500;
const INDIVIDUAL_NOTES: [&str; 3] = [
    "Each symbol is optimized alone as a single-instrument backtest with the whole capital: portfolio max positions, ranking and shared cash do not apply.",
    "Best is the optimizer's own pick for that symbol (#1 of the same ranking as the single-symbol optimizer); none when no combination has enough trades.",
    "Each symbol's all-results table keeps at most 500 combinations in grid order; the best row is always shown.",
];
/// AmiBroker Individual Optimize (`mode=individual`): every member of spec.portfolio.universe runs the single-instrument
/// optimizer on the spec without its portfolio block; one clock (OPT_SECONDS) and one work cap cover all symbols.
fn individual_optimize_json(st: &State, q: &Query, spec: Spec, t0: std::time::Instant) -> Result<String, LabError> {
    let p = spec.portfolio.as_ref().ok_or_else(|| LabError::Invalid("mode=individual optimizes each symbol of spec.portfolio.universe; this spec has no portfolio universe".into()))?;
    portfolio_request(q, &spec, markets::engine_market().as_deref() == Some("gl"), true)?;
    if p.mode == rules::Mode::Rotation { return Err(LabError::Unsupported("Individual optimize needs signals mode; rotation ranks the symbols against each other".into())); }
    if spec.optimize.is_empty() { return Err(LabError::Invalid("optimize lists no parameter ranges".into())); }
    if let Some(d) = spec.optimize.iter().find(|d| d.path.starts_with("portfolio.")) { return Err(LabError::Invalid(format!("{} is a portfolio setting; Individual optimize runs each symbol alone", d.path))); }
    let tickers: Vec<String> = match &p.universe { rules::Universe::Basket(b) => super::universe::basket_members(st, b).into_iter().map(|m| m.0).collect(), rules::Universe::Tickers(t) => t.clone() };
    if tickers.is_empty() { return Err(LabError::Invalid("portfolio.universe has no member on this engine".into())); }
    for t in &tickers {
        let kind = instrument_policy::selected_type(t, st.baskets.native_evidence(t).is_some());
        if !matches!(kind, "individual_stock" | "etf") { return Err(LabError::Invalid(format!("{t} ({kind}) cannot be optimized as a universe member; members are actual stocks or ETFs with their own prices"))); }
    }
    let combos = spec.optimize.iter().try_fold(1usize, |n, d| n.checked_mul(d.values.len())).unwrap_or(usize::MAX);
    individual_work(tickers.len(), combos)?;
    let single = Spec { portfolio: None, ..spec.clone() };
    let nums = |v: Option<&super::json::Json>| match v { Some(super::json::Json::Arr(v)) => Some(arr(v.iter().map(|x| x.as_f64().map_or("null".into(), full)))), _ => None };
    let (mut results, mut warnings, mut label, mut cols, mut table_cols, mut threads, mut evaluated) = (vec![], vec![], LABEL.to_string(), "[]".to_string(), String::new(), 1, 0);
    for t in &tickers {
        if t0.elapsed().as_secs() >= OPT_SECONDS { return Err(LabError::Invalid(format!("individual optimization stopped after {OPT_SECONDS} s; use fewer symbols or a smaller grid. No partial result is published."))); }
        let mut qi = q.clone();
        qi.remove("mode"); qi.insert("ticker".into(), t.clone()); qi.insert("slim".into(), "1".into());
        let body = match optimize_spec(st, &qi, single.clone(), t0) {
            Ok(b) => b,
            Err(LabError::NeedsData(why)) => { warnings.push(format!("{t} skipped: {why}")); results.push(format!("{{\"ticker\":\"{}\",\"skipped\":\"{}\"}}", esc(t), esc(&why))); continue; }
            Err(LabError::Invalid(m)) => return Err(LabError::Invalid(format!("{t}: {m}"))),
            Err(LabError::Unsupported(m)) => return Err(LabError::Unsupported(format!("{t}: {m}"))),
        };
        let j = super::json::parse(&body).map_err(|e| LabError::Invalid(format!("internal: unreadable optimizer output ({e})")))?;
        let get = |k: &str| j.get(k);
        label = get("label").and_then(|l| l.as_str()).unwrap_or(LABEL).to_string();
        if let Some(super::json::Json::Arr(v)) = get("cols") { cols = quoted(v.iter().filter_map(|x| x.as_str())); }
        if let Some(super::json::Json::Arr(v)) = get("table_cols") { table_cols = format!(",\"table_cols\":{}", quoted(v.iter().filter_map(|x| x.as_str()))); }
        threads = threads.max(get("threads").and_then(|x| x.as_f64()).unwrap_or(1.) as usize);
        evaluated += combos;
        let rank = get("rank");
        let num_of = |k: &str| rank.and_then(|r| r.get(k)).and_then(|x| x.as_f64()).map_or("null".into(), full);
        let top = rank.and_then(|r| r.get("top3")).and_then(|v| if let super::json::Json::Arr(v) = v { v.first() } else { None }).and_then(|c| c.get("row")).and_then(|r| r.as_f64());
        let best = top.and_then(|r| nums(get("picked").and_then(|p| p.get(&(r as usize).to_string())))).unwrap_or_else(|| "null".into());
        let table = match get("table") { Some(super::json::Json::Arr(v)) => format!(",\"table\":[{}],\"table_rows\":{}", v.iter().take(INDIVIDUAL_TABLE_ROWS).filter_map(|r| nums(Some(r))).collect::<Vec<_>>().join(","), v.len()), _ => String::new() };
        if let Some(super::json::Json::Arr(w)) = get("info").and_then(|i| i.get("warnings")) { warnings.extend(w.iter().filter_map(|x| x.as_str()).map(|x| format!("{t}: {x}"))); }
        if combos > INDIVIDUAL_TABLE_ROWS && !table.is_empty() { warnings.push(format!("{t}: all-results table capped at {INDIVIDUAL_TABLE_ROWS} of {combos} combinations")); }
        results.push(format!("{{\"ticker\":\"{}\",\"from\":\"{}\",\"to\":\"{}\",\"eligible\":{},\"min_trades\":{},\"best\":{best}{table}}}", esc(t),
            esc(get("from").and_then(|x| x.as_str()).unwrap_or_default()), esc(get("to").and_then(|x| x.as_str()).unwrap_or_default()), num_of("eligible"), num_of("min_trades")));
    }
    let dims_json = arr(spec.optimize.iter().map(|d| format!("{{\"path\":\"{}\",\"values\":{}}}", esc(&d.path), arr(d.values.iter().map(|v| crate::num(*v))))));
    let basket = match &p.universe { rules::Universe::Basket(b) => format!("\"{}\"", esc(b)), _ => "null".into() };
    Ok(format!("{{\"label\":\"{}\",\"kind\":\"lab_individual_optimization\",\"origin\":null,\"universe\":{{\"basket\":{basket},\"symbols\":{}}},\"combos\":{combos},\"dims\":{dims_json},\"cols\":{cols}{table_cols},\"results\":[{}],\"notes\":{}{}{}}}",
        esc(&label), tickers.len(), results.join(","), quoted(INDIVIDUAL_NOTES.iter().copied()), info_json(t0.elapsed().as_millis(), evaluated, threads, &warnings),
        adaptations(rules::series_ids(&spec).iter().map(String::as_str))))
}

/// A spec with `origin` is a customized book template: the template must exist (same book, version not newer).
pub(super) fn origin_of(spec: &Spec) -> Result<Option<StrategyDef>, LabError> {
    let Some(o) = &spec.origin else { return Ok(None) };
    let found = templates::template(&o.method).filter(|t| o.version <= t.version).and(templates::method(&o.method)).filter(|d| d.book == o.book);
    let d = found.ok_or_else(|| LabError::Invalid(format!("origin names no book template ({} · {} · version {})", o.book, o.method, o.version)))?;
    demoted(templates::unavailable(&d)).map(|_| Some(d))
}
/// A variant of a method the book no longer admits (demoted) is refused; `why` is templates::unavailable().
fn demoted(why: Option<String>) -> Result<(), LabError> {
    why.map_or(Ok(()), |w| Err(LabError::NeedsData(format!("the book method behind this variant is not available for Customize: {w}"))))
}
fn price_problem(r: &crate::Row) -> Option<&'static str> {
    if ![r.open, r.high, r.low, r.close].iter().all(|n| n.is_finite() && *n > 0.0) { Some("non-positive or missing price") }
    else if r.low > r.high { Some("low above the high") } else if r.open < r.low { Some("open below the low") } else if r.open > r.high { Some("open above the high") }
    else if r.close < r.low { Some("close below the low") } else if r.close > r.high { Some("close above the high") } else { None }
}
/// Foreign warm-up and resampling consume the source history, so validate that history before computing.
fn raw_prices_clean(rows: &[crate::Row]) -> Result<(), LabError> {
    for r in rows {
        let why = price_problem(r).or_else(|| r.volume.filter(|v| !v.is_finite() || *v < 0.).map(|_| "invalid volume"));
        if let Some(why) = why { return Err(LabError::NeedsData(format!("{}: invalid source bar on {} ({why}); repair the source before using it", r.ticker, r.date))); }
    }
    Ok(())
}
/// Same predicate as the book's own check (readiness::check): every bar in the window must be a valid OHLC bar.
pub(super) fn bars_clean(ctx: &Ctx, from: usize, to: usize) -> Result<(), LabError> {
    for r in &ctx.r[from..=to] {
        if let Some(why) = price_problem(r) {
            return Err(LabError::NeedsData(format!("{}: invalid bar on {} ({why}); choose a window after it", r.ticker, r.date)));
        }
    }
    Ok(())
}
/// A variant keeps its book method's instrument admission: stock methods on actual stocks, ETF methods on ETF prices.
pub(super) fn origin_admits(d: Option<&StrategyDef>, kind: &str, ticker: &str) -> Result<(), LabError> {
    let Some(d) = d else { return Ok(()) };
    let allowed = instrument_policy::policy(d).allowed_types;
    if allowed.contains(&kind) { return Ok(()); }
    Err(LabError::NeedsData(format!("{ticker} ({kind}): variants of {} · {} run on {} only (no index or other substitute)", d.book, d.name, allowed.join(", "))))
}
fn method_json(d: &StrategyDef) -> String {
    format!("{{\"id\":\"{}\",\"name\":\"{}\",\"book\":\"{}\",\"group\":\"{}\",\"dir\":\"{}\"}}", esc(d.id), esc(d.name), esc(d.book), esc(d.group), esc(d.dir))
}
/// The run label and the `origin` JSON (null for a plain lab strategy).
fn labelled(d: Option<&StrategyDef>, version: impl std::fmt::Display) -> (String, String) {
    match d {
        None => (LABEL.into(), "null".into()),
        Some(d) => (templates::variant_label(d), format!("{{\"method\":{},\"template_version\":{}}}", method_json(d), version)),
    }
}
pub fn templates_json() -> String {
    let all = crate::connors::catalogue();
    let live = all.iter().filter(|d| templates::book_status(d).0 == "live").count();
    let items = arr(templates::TEMPLATES.iter().filter_map(|t| templates::method(t.method).map(|d| {
        let p = instrument_policy::policy(&d);
        format!("{{\"method\":{},\"template_version\":{},\"requirement\":\"{}\",\"allowed_types\":{}}}", method_json(&d), t.version, p.requirement, quoted(p.allowed_types.iter().copied()))
    })));
    format!("{{\"label\":\"{}\",\"methods\":{},\"admitted\":{live},\"templates\":{items}}}", templates::TEMPLATE_LABEL, all.len())
}
pub fn template_json(_: &State, q: &Query) -> Result<String, LabError> {
    let id = q.get("method").ok_or_else(|| LabError::Invalid("method is required".into()))?;
    let d = templates::method(id).ok_or_else(|| LabError::Invalid(format!("unknown book method {id}")))?;
    let head = format!("{{\"label\":\"{}\",\"method\":{}", templates::TEMPLATE_LABEL, method_json(&d));
    if let Some(why) = templates::unavailable(&d) { return Ok(format!("{head},\"status\":\"unavailable\",\"reason\":\"{}\"}}", esc(&why))); }
    let t = templates::template(id).expect("available means templated");
    let p = instrument_policy::policy(&d);
    Ok(format!("{head},\"status\":\"available\",\"instrument\":{{\"requirement\":\"{}\",\"allowed_types\":{}}},\"template\":{{\"version\":{},\"spec\":{},\"notes\":{}}}}}",
        p.requirement, quoted(p.allowed_types.iter().copied()), t.version, t.spec, quoted(templates::COMMON_NOTES.iter().copied())))
}
/// Unmodified template vs the book engine on the book's own admitted window and default parameters (spec §6).
/// Display only: one book run and one lab run on one instrument; book code is only called.
pub fn fidelity_json(st: &State, q: &Query) -> Result<String, LabError> {
    let id = q.get("method").ok_or_else(|| LabError::Invalid("method is required".into()))?;
    let d = templates::method(id).ok_or_else(|| LabError::Invalid(format!("unknown book method {id}")))?;
    if let Some(why) = templates::unavailable(&d) { return Err(LabError::Unsupported(format!("Customize not available — {why}"))); }
    if q.keys().any(|k| k.starts_with("p.")) { return Err(LabError::Invalid("the fidelity check uses the book's default parameters; remove p.* settings".into())); }
    let t = templates::template(id).expect("available means templated");
    let selected = select(st, q)?;
    let held = selected.context(st);
    let ticker = &held.r[0].ticker;
    // engine-gl data are provisional TradingView exports; book methods are not admitted on them
    if markets::gl(ticker).is_some() { return Err(LabError::NeedsData("book methods do not run on provisional data".into())); }
    let (kind, instrument) = instrument(&selected, ticker);
    origin_admits(Some(&d), kind, ticker)?;
    let (ctx, from, to, _) = crate::resolve_context(&held, q)?;
    let none = crate::connors::Params::new();
    if let Some(i) = crate::readiness::check(ctx, &d, from, to, &none).or_else(|| crate::native_admission(ctx, &d, from, to, &selected, st, q)) {
        return Err(LabError::NeedsData(format!("the book method is not admitted on this instrument and window ({}: {}); no comparison is shown", i.status, i.note)));
    }
    let from = crate::window_start(ctx, &d, from).filter(|&f| f < to).ok_or_else(|| LabError::NeedsData("required volatility-index data does not cover this period".into()))?;
    let book = crate::connors::run(id, ctx, from, to, &none).ok_or_else(|| LabError::Invalid("book implementation unavailable".into()))?;
    let fr = Frames::new(Bars::from_ctx(ctx));
    let lab = simulate(&series::compile(&templates::parsed(t), &fr, &mut Cache::new())?, &fr.d, from, to);
    let c = fidelity::compare(&book, &lab);
    let tj = |t: Option<&crate::connors::Trade>| t.map_or("null".into(), |t| format!(
        "{{\"entry\":\"{}\",\"exit\":\"{}\",\"entry_px\":{},\"exit_px\":{},\"dir\":{},\"legs\":{},\"open\":{}}}",
        ctx.r[t.legs[0].i].date, ctx.r[t.exit_i].date, crate::num(t.legs[0].px), crate::num(t.exit_px), t.dir, t.legs.len(), t.open));
    let diffs = arr(c.diffs.iter().take(fidelity::MAX_DIFFS).map(|(b, l)| format!("{{\"book\":{},\"template\":{}}}", tj(*b), tj(*l))));
    Ok(format!(
        "{{\"label\":\"{}\",\"kind\":\"lab_template_fidelity\",\"method\":{},\"template_version\":{},\"instrument\":{instrument},\"from\":\"{}\",\"to\":\"{}\",\"book_trades\":{},\"template_trades\":{},\"matched\":{},\"reproduces\":{},\"differences_total\":{},\"differences\":{diffs},\"note\":\"{}\"}}",
        fidelity::FIDELITY_LABEL, method_json(&d), t.version, ctx.r[from].date, ctx.r[to].date, book.len(), lab.len(), c.matched,
        fidelity::reproduces(book.len(), lab.len(), c.matched), c.diffs.len(), fidelity::note(book.len(), lab.len())))
}
/// AmiBroker result-list columns (Net %, Exposure, CAR, Max. sys % DD, CAR/MDD, # trades, % winners, PF, Avg. % P/L, Sharpe).
/// `net_profit` is left out: it sums trade percentages, not AmiBroker's money Net Profit.
const TABLE_KEYS: [&str; 10] = ["total_return", "exposure", "cagr", "max_dd", "mar", "trades", "win_rate", "profit_factor", "avg_trade", "sharpe"];
/// Opt-in (`table=1`) list of every combination: each row is the dim values then `table_cols`, read from metric columns starting at `nd`.
fn table_json(q: &Query, rows: &[Vec<f64>], nd: usize, metric_cols: &[&str], num: fn(f64) -> String) -> String {
    if q.get("table").map_or(true, |v| v != "1") { return String::new(); }
    let keys: Vec<(&str, usize)> = TABLE_KEYS.iter().filter_map(|k| metric_cols.iter().position(|c| c == k).map(|i| (*k, nd + i))).collect();
    let row = |r: &Vec<f64>| arr(r[..nd].iter().chain(keys.iter().map(|k| &r[k.1])).map(|v| num(*v)));
    format!(",\"table_cols\":{},\"table\":[{}]", quoted(keys.iter().map(|k| k.0)), rows.iter().map(row).collect::<Vec<_>>().join(","))
}
/// RANK_STATS metrics without an is_ PART_COLS column: shown in-sample so the ranked numbers are visible.
fn extra_rank_cols() -> Vec<&'static str> {
    crate::RANK_STATS.iter().map(|(k, _)| *k).filter(|k| !crate::PART_COLS.contains(k)).collect()
}
/// Dedup identity columns of a rank row. With a split: in-sample full stats + is_ part stats (oos_ is display-only).
fn outcome_cols(nd: usize, ndisp: usize, split: bool) -> Vec<usize> {
    let part_end = if split { nd + crate::FULL_COLS.len() + crate::PART_COLS.len() } else { nd + ndisp };
    (nd..nd + 16).chain(nd + crate::FULL_COLS.len()..part_end).collect()
}
/// Ranking input for one grid row: with a split, the in-sample FULL_COLS (trailing block) replace the full-window ones.
fn rank_basis(row: &[f64], nd: usize, ndisp: usize, split: bool) -> Vec<f64> {
    if !split { return row[..nd + ndisp].to_vec(); }
    [&row[..nd], &row[nd + ndisp..], &row[nd + crate::FULL_COLS.len()..nd + ndisp]].concat()

}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;

    fn state(n: usize, ticker: &str, volume: bool) -> State { state_bad(n, ticker, volume, usize::MAX) }
    fn state_bad(n: usize, ticker: &str, volume: bool, bad: usize) -> State {
        let rows: &'static [Row] = Box::leak((0..n).map(|i| {
            let c = 100. + (i as f64 * 0.3).sin() * 8. + i as f64 * 0.02;
            Row { ticker: ticker.into(), date: format!("{}-{:02}-{:02}", 2015 + i / 336, 1 + (i / 28) % 12, 1 + i % 28), open: if i == bad { c - 2. } else { c - 0.2 }, high: c + 1., low: c - 1., close: c, volume: volume.then_some(1000.) }
        }).collect::<Vec<_>>().into_boxed_slice());
        State { ctxs: BTreeMap::from([(ticker.into(), Ctx::new(rows, rows, &[]))]), baskets: Default::default(), vix: vec![], market_extra: Default::default() }
    }
    const SPEC: &str = r#"{"v":1,"entry":{"all":[{"lhs":{"s":"close"},"op":"crosses_above","rhs":{"s":"sma","p":{"length":5}}}]},"exit":{"any":[{"lhs":{"s":"bars_held"},"op":">=","rhs":{"value":3}}]},"optimize":[{"path":"entry.all[0].rhs.p.length","from":3,"to":7,"step":2},{"path":"exit.any[0].rhs.value","from":2,"to":4,"step":1}]}"#;
    fn q(extra: &[(&str, &str)]) -> Query {
        let mut q: Query = [("ticker", "SPX"), ("spec", SPEC), ("cost", "5")].iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();
        for (k, v) in extra { q.insert(k.to_string(), v.to_string()); }
        q
    }

    #[test] fn shared_frames_loader_preserves_identity_coverage_and_volatility_source() {
        let mut st = state(30, "NIFTY 50", true);
        let (frames, kind, first, last) = frames_of(&st, "NIFTY 50").unwrap();
        assert_eq!((kind, frames.d.len()), ("index", 30));
        assert_eq!((&first, &last), (frames.d.date.first().unwrap(), frames.d.date.last().unwrap()));
        assert!(matches!(frames_of(&st, "missing"), Err(LabError::Invalid(_))));
        assert!(matches!(frames_of(&st, "SPY"), Err(LabError::NeedsData(_))));
        assert!(matches!(frames_of(&st, "INDIA VIX"), Err(LabError::NeedsData(_))));
        st.vix = st.ctxs["NIFTY 50"].r[..3].iter().map(|r| Row { ticker: "INDIA VIX".into(), ..r.clone() }).collect();
        let (vix, kind, first, last) = frames_of(&st, "INDIA VIX").unwrap();
        assert_eq!((kind, vix.d.len(), first.as_str(), last.as_str()), ("volatility_index", 3, "2015-01-01", "2015-01-03"));
        assert!(matches!(frames_of(&st, "CBOE VIX"), Err(LabError::NeedsData(_))), "one volatility source cannot stand in for another");
        st.vix[1].ticker = "CBOE VIX".into();
        assert!(matches!(frames_of(&st, "INDIA VIX"), Err(LabError::NeedsData(_))));
        st.vix[1].ticker = "INDIA VIX".into();
        st.vix[1].date = st.vix[0].date.clone();
        assert!(frames_of(&st, "INDIA VIX").err().unwrap().message().contains("dates"));
        let index = st.ctxs["NIFTY 50"].r;
        st.ctxs.insert("SPY".into(), Ctx::new(index, index, &[]));
        assert!(frames_of(&st, "SPY").err().unwrap().message().contains("source identity"), "an index cannot stand in for an ETF");
    }

    #[test] fn shared_frames_reject_invalid_raw_prices_before_resampling() {
        for price in [f64::NAN, f64::INFINITY, 0., 1e9] {
            let mut st = foreign_state();
            st.vix = st.ctxs["SPX"].r[..3].iter().map(|r| Row {ticker:"CBOE VIX".into(), ..r.clone()}).collect();
            st.vix[0].close = price;
            assert!(matches!(frames_of(&st, "CBOE VIX"), Err(LabError::NeedsData(_))), "raw volatility price {price} is not an indicator warm-up hole");
            let mut rows = st.ctxs["SPY"].r.to_vec(); rows[0].close = price;
            let rows = Box::leak(rows.into_boxed_slice());
            st.ctxs.insert("SPY".into(), Ctx::new(rows, rows, &[]));
            assert!(matches!(frames_of(&st, "SPY"), Err(LabError::NeedsData(_))), "raw ETF price {price}");
        }
    }

    #[test] fn portfolio_grammar_never_falls_through_to_single_instrument_execution() {
        let portfolio=SPEC.replace("\"v\":1", "\"v\":1,\"execution\":{\"capital\":100000},\"portfolio\":{\"universe\":{\"tickers\":[\"SPY\",\"QQQ\"]},\"max_open\":2}");
        let st=state(100,"SPX",true);let query=q(&[("spec",&portfolio)]);
        for result in [run_json(&st,&query),optimize_json(&st,&query)]{assert!(result.unwrap_err().message().contains("omit ticker"));}
    }

    fn foreign_state() -> State {
        let mut st = state(500, "SPX", true);
        let rows: &'static [Row] = Box::leak(st.ctxs["SPX"].r.iter().enumerate().filter(|(i, _)| i % 97 != 5).map(|(i, r)| {
            let factor = 1. + (i % 7) as f64 / 100.;
            Row { ticker: "SPY".into(), open: r.open * factor, high: r.high * factor, low: r.low * factor, close: r.close * factor, ..r.clone() }
        }).collect::<Vec<_>>().into_boxed_slice());
        st.ctxs.insert("SPY".into(), Ctx::new(rows, rows, &[]));
        st
    }
    const FOREIGN_FILTER: &str = r#"{"v":1,"entry":{"all":[{"lhs":{"s":"close","symbol":"SPX"},"op":">","rhs":{"s":"sma","p":{"length":50},"symbol":"SPX"}}]},"exit":{"any":[{"lhs":{"s":"bars_held"},"op":">=","rhs":{"value":5}}]}}"#;
    #[test] fn foreign_market_filter_uses_its_source_but_fills_the_selected_etf() {
        let st = foreign_state();
        let body = run_json(&st, &q(&[("ticker", "SPY"), ("spec", FOREIGN_FILTER)])).unwrap();
        assert!(body.contains("\"instrument\":{\"ticker\":\"SPY\",\"type\":\"etf\""));
        assert!(!body.contains("source_scope"),"plain single-instrument metadata keeps its existing schema");
        assert!(body.contains("\"foreign\":[{\"ticker\":\"SPX\",\"type\":\"index\",\"alignment\":\"same_date\",\"fixup\":false"));
        let entry: f64 = body.split("\"entry_px\":").nth(1).unwrap().split(',').next().unwrap().parse().unwrap();
        let json=crate::lab::json::parse(&body).unwrap();let trades=match json.get("trades").unwrap(){crate::lab::json::Json::Arr(v)=>v,_=>panic!()};let date=trades[0].get("entry").unwrap().as_str().unwrap();let raw=st.ctxs["SPY"].r.iter().find(|r|r.date==date).unwrap();assert_eq!(crate::num(raw.open),crate::num(entry));
        assert!(!run_json(&st, &q(&[("ticker", "SPY")])).unwrap().contains("\"foreign\""));
    }
    #[test] fn foreign_trade_fills_closed_trades_and_equity_are_causal(){
        use crate::lab::json::{parse,Json};let baseline=foreign_state();let query=q(&[("ticker","SPY"),("spec",FOREIGN_FILTER)]);let want=parse(&run_json(&baseline,&query).unwrap()).unwrap();let list=|j:&Json,key:&str|match j.get(key).unwrap(){Json::Arr(v)=>v.clone(),_=>panic!()};let trades=list(&want,"trades");assert!(trades.iter().any(|t|t.get("open")==Some(&Json::Bool(false))));
        for cut in [150usize,300]{let mut changed=foreign_state();let cutdate=changed.ctxs["SPY"].r[cut].date.clone();let rows: &'static [crate::Row]=Box::leak(changed.ctxs["SPX"].r.iter().map(|r|if r.date>cutdate{crate::Row{open:r.open*1.7,high:r.high*1.7,low:r.low*1.7,close:r.close*1.7,..r.clone()}}else{r.clone()}).collect::<Vec<_>>().into_boxed_slice());changed.ctxs.insert("SPX".into(),Ctx::new(rows,rows,&[]));let got=parse(&run_json(&changed,&query).unwrap()).unwrap();assert_eq!(&list(&got,"equity")[..=cut],&list(&want,"equity")[..=cut]);
            let closed=|j:&Json|list(j,"trades").into_iter().filter(|t|t.get("open")==Some(&Json::Bool(false))&&t.get("exit").unwrap().as_str().unwrap()<=cutdate.as_str()).collect::<Vec<_>>();assert_eq!(closed(&got),closed(&want));
            let fills=|j:&Json|list(j,"trades").into_iter().filter(|t|t.get("entry").unwrap().as_str().unwrap()<=cutdate.as_str()).map(|t|["entry","entry_px","entry_kind"].iter().map(|k|t.get(k).unwrap().clone()).collect::<Vec<_>>()).collect::<Vec<_>>();assert_eq!(fills(&got),fills(&want));
        }
    }
    #[test] fn foreign_metadata_records_each_real_pair_alignment_and_fixup_use(){
        use crate::lab::json::{parse,Json};let mut st=state(6,"HSI:HSI",false);for (ticker,start,step) in [("HSI:HSI",200.,1.),("IG:DAX",100.,1.),("IG:FTSE",50.,2.)]{let rows: &'static [crate::Row]=Box::leak((0..6).map(|i|{let c=start+i as f64*step;crate::Row{ticker:ticker.into(),date:format!("2024-01-0{}",i+1),open:c,high:c+1.,low:c-1.,close:c,volume:None}}).collect::<Vec<_>>().into_boxed_slice());st.ctxs.insert(ticker.into(),Ctx::new(rows,rows,&[]));}
        let pair=rules::Foreign{symbol:Some("IG:DAX".into()),vs:Some("IG:FTSE".into()),fixup:false};let refs=vec![("entry.all[0].lhs".into(),pair.clone()),("entry.all[1].lhs".into(),rules::Foreign{symbol:Some("IG:DAX".into()),fixup:true,..Default::default()})];let (others,meta)=others_for(&st,"HSI:HSI",&refs,false,false).unwrap();let j=parse(&format!("{{{}}}",meta.trim_start_matches(','))).unwrap();let rows=match j.get("foreign").unwrap(){Json::Arr(v)=>v,_=>panic!()};let dax=rows.iter().find(|r|r.get("ticker").and_then(Json::as_str)==Some("IG:DAX")).unwrap();let uses=match dax.get("uses").unwrap(){Json::Arr(v)=>v,_=>panic!()};assert_eq!(uses.len(),2);assert_eq!(uses[0].get("fixup"),Some(&Json::Bool(false)));assert_eq!(uses[1].get("fixup"),Some(&Json::Bool(true)));
        let ftse=rows.iter().find(|r|r.get("ticker").and_then(Json::as_str)==Some("IG:FTSE")).unwrap();let uses=match ftse.get("uses").unwrap(){Json::Arr(v)=>v,_=>panic!()};assert_eq!(uses[0].get("source").and_then(Json::as_str),Some("IG:DAX"));assert_eq!(uses[0].get("alignment").and_then(Json::as_str),Some("prior_date"));let (fr,_,_,_)=frames_of(&st,"HSI:HSI").unwrap();let r=rules::series_ref("test","spread",&Default::default(),Tf::D,None,0,pair).unwrap();let values=series::series_in(&fr,&others,&mut Cache::new(),&r).unwrap();assert!(values[0].is_nan()&&values[1].is_nan());assert_eq!(values[2],51.);
    }
    #[test] fn foreign_api_refusals_preserve_operand_path_and_market_identity() {
        let st = foreign_state();
        for (symbol, status, note) in [("NOPE", "invalid", "unknown ticker"), ("IG:DAX", "needs_data", "/api/gl"), ("NIFTY 50", "needs_data", "cross-market")] {
            let spec = FOREIGN_FILTER.replace("\"symbol\":\"SPX\"", &format!("\"symbol\":\"{symbol}\""));
            let error = run_json(&st, &q(&[("ticker", "SPY"), ("spec", &spec)])).unwrap_err();
            assert_eq!(error.status(), status, "{symbol}: {}", error.message());
            assert!(error.message().contains(note) && error.message().contains("entry.all[0].lhs"), "{}", error.message());
        }
    }
    #[test] fn foreign_volatility_identity_and_close_availability_are_disclosed() {
        let mut st = foreign_state();
        st.vix = st.ctxs["SPX"].r.iter().map(|r| Row {ticker: "CBOE VIX".into(), ..r.clone()}).collect();
        let spec = FOREIGN_FILTER.replace("\"symbol\":\"SPX\"", "\"symbol\":\"CBOE VIX\"");
        assert!(run_json(&st, &q(&[("ticker", "SPY"), ("spec", &spec)])).unwrap().contains("\"alignment\":\"same_date\""));
        let close = spec.replace("\"v\":1", "\"v\":1,\"execution\":{\"fill\":\"close\"}");
        assert!(run_json(&st, &q(&[("ticker", "SPY"), ("spec", &close)])).unwrap().contains("\"alignment\":\"prior_date\""));
        let wrong = spec.replace("CBOE VIX", "INDIA VIX");
        assert!(run_json(&st, &q(&[("ticker", "SPY"), ("spec", &wrong)])).unwrap_err().message().contains("cross-market"));
    }
    #[test] fn pair_chart_query_and_optimization_use_the_same_foreign_validation() {
        let st = foreign_state();
        let body = series_json(&st, &q(&[("ticker", "SPY"), ("id", "ratio"), ("vs", "SPX")])).unwrap();
        assert!(body.contains("\"id\":\"ratio\"") && body.contains("\"outputs\":[\"value\"]"));
        assert!(body.contains("\"foreign\""));
        for other in ["", "SPX\n", &"A".repeat(81)] {
            assert_eq!(series_json(&st, &q(&[("ticker", "SPY"), ("id", "ratio"), ("vs", other)])).unwrap_err().status(), "invalid");
        }
        assert_eq!(series_json(&st, &q(&[("ticker", "SPY"), ("id", "ratio")])).unwrap_err().status(), "invalid");
        let optimize = FOREIGN_FILTER.replace("\"v\":1", "\"v\":1,\"optimize\":[{\"path\":\"entry.all[0].rhs.p.length\",\"from\":40,\"to\":50,\"step\":10}]");
        let body = optimize_json(&st, &q(&[("ticker", "SPY"), ("spec", &optimize)])).unwrap();
        assert!(body.contains("\"foreign\"") && body.contains("\"combos\":2"));
    }

    #[test] fn catalogue_lists_notes_needs_and_sources() {
        let j = catalogue_json();
        assert!(crate::lab::json::parse(&j).is_ok());
        assert!(j.contains("\"id\":\"cvd\"") && j.contains("\"note\":\"not available yet — needs intraday (lower-timeframe/tick) bars\""));
        assert!(j.contains("\"tv\":\"Relative Strength Index (RSI)\"") && j.contains("\"ab\":\"RSI; RSIa on close"));
        assert!(j.contains("\"needs\":{\"volume\":false,\"intraday\":false,\"breadth\":false,\"second_symbol\":false,\"fundamentals\":false,\"derivatives\":false,\"onchain\":false,\"order_flow\":false,\"order_book\":false}"));
        assert!(j.contains("\"id\":\"correlation\"") && j.contains("\"second_symbol\":true"));
    }
    #[test] fn catalogue_lists_every_indicator_with_label() {
        let j = catalogue_json();
        assert!(j.starts_with(&format!("{{\"label\":\"{LABEL}\"")));
        for d in catalogue::all() { assert!(j.contains(&format!("{{\"id\":\"{}\"", d.id)), "{}", d.id); }
        assert!(j.contains("\"id\":\"vwap\"") && j.contains("\"intraday\":true") && j.contains("\"first\":[null]"));
        assert!(crate::lab::json::parse(&j).is_ok());
    }
    #[test] fn unknown_route_is_labelled_json_and_unknown_ticker_is_invalid() {
        let j = not_found_json("/api/lab/no\"pe");
        let v = crate::lab::json::parse(&j).unwrap();
        assert_eq!(v.get("status").unwrap().as_str(), Some("not_found"));
        assert!(j.contains(LABEL));
        let st = state(50, "SPX", false);
        let e = series_json(&st, &q(&[("id", "close"), ("ticker", "NOPE")])).unwrap_err();
        assert_eq!((e.status(), e.message()), ("invalid", "unknown ticker NOPE"));
    }
    #[test] fn select_errors_map_by_prefix() {
        assert_eq!(select_error("unknown ticker X".into()).status(), "invalid");
        assert_eq!(select_error("Native source CSV unavailable".into()).status(), "needs_data");
        let e = select_error("needs_data: Native requested from must be a valid date".into());
        assert_eq!((e.status(), e.message()), ("needs_data", "Native requested from must be a valid date"));
    }
    #[test] fn panics_become_typed_errors() {
        assert_eq!(guarded(|| panic!("boom")).unwrap_err().status(), "invalid");
        let st = state(50, "SPX", false);
        assert!(route("/api/lab/nothing", &st, &q(&[])).is_none());
        assert!(route("/api/lab/catalogue", &st, &q(&[])).unwrap().is_ok());
    }
    const FORMULA: &str = "Fast = Param(\"Fast\", 9, 2, 50, 1)\nPlot(EMA(C, Fast) - EMA(C, 21), \"Osc\")\nPlot(EMA(C, Fast), \"Line\", \"price\")";
    #[test] fn formula_route_validates_and_describes() {
        let st = state(10, "SPX", false);
        let j = route("/api/lab/formula", &st, &q(&[("formula", FORMULA)])).unwrap().unwrap();
        assert_eq!(j, r#"{"ok":true,"params":[{"key":"Fast","default":9,"min":2,"max":50,"step":1}],"plots":["Osc","Line"],"placement":"price"}"#);
        let (status, body) = response(None, "/api/lab/formula", &st, &q(&[("formula", "x = C\n  y = FOO(x)\nPlot(y, \"A\")")]));
        assert_eq!(status, "400 Bad Request");
        assert_eq!(crate::lab::json::parse(&body).unwrap().get("error").unwrap().as_str(), Some("line 2, col 7: unknown function FOO"));
        assert_eq!(formula_json(&st, &q(&[])).unwrap_err().message(), "formula is required");
    }
    #[test] fn series_route_runs_formulas_with_bare_or_prefixed_params() {
        let st = state(120, "SPX", true);
        let j = series_json(&st, &q(&[("formula", FORMULA), ("name", "My Momentum"), ("Fast", "12")])).unwrap();
        let v = crate::lab::json::parse(&j).unwrap();
        assert!(j.contains(r#""id":"formula","name":"My Momentum","tf":"D","params":{"Fast":12},"outputs":["Osc","Line"]"#), "{j}");
        assert!(j.ends_with(r#","formula":{"params":[{"key":"Fast","default":9,"min":2,"max":50,"step":1}],"plots":["Osc","Line"],"placement":"price"}}"#));
        let crate::lab::json::Json::Arr(line) = v.get("series").unwrap().get("Line").unwrap() else { panic!() };
        let closes: Vec<f64> = st.ctxs["SPX"].r.iter().map(|r| r.close).collect();
        let want = crate::lab::ta::ema(&closes, 12);
        assert_eq!(line.len(), 120);
        for (k, w) in want.iter().enumerate() { assert_eq!(line[k].as_f64().unwrap_or(f64::NAN).to_bits() == w.to_bits() || (w.is_nan() && line[k] == crate::lab::json::Json::Null), true, "bar {k}"); }
        assert!(series_json(&st, &q(&[("formula", FORMULA), ("p.Fast", "12")])).unwrap().contains(r#""params":{"Fast":12}"#));
        assert!(series_json(&st, &q(&[("formula", FORMULA), ("tf", "W")])).unwrap().contains(r#""tf":"W","params":{"Fast":9}"#));
        let e = |extra: &[(&str, &str)]| series_json(&st, &q(&[&[("formula", FORMULA)], extra].concat())).unwrap_err().message().to_string();
        assert_eq!(e(&[("Fast", "1")]), "Fast must be between 2 and 50");
        assert_eq!(e(&[("Fast", "abc")]), "Fast must be a finite number");
        assert_eq!(e(&[("p.Slow", "3")]), "unknown parameter 'Slow'");
        assert!(e(&[("vs", "NDX")]).contains("a formula runs on the selected ticker"));
        assert_eq!(series_json(&st, &q(&[("formula", "Plot(Ref(C, 1), \"A\")")])).unwrap_err().message(), "line 1, col 13: Ref(x, 1) would read 1 bars in the future; use Ref(x, -n) for n bars ago");
        let no_volume = state(60, "SPX", false);
        let nv = series_json(&no_volume, &q(&[("formula", "Plot(V, \"A\")")])).unwrap_err();
        assert_eq!((nv.status(), nv.message()), ("needs_data", "line 1, col 6: volume: volume is missing on some admitted bars of this instrument"));
    }
    #[test] fn catalogue_lists_formula_functions() {
        let v = crate::lab::json::parse(&catalogue_json()).unwrap();
        let crate::lab::json::Json::Arr(f) = v.get("formula_functions").unwrap() else { panic!() };
        assert_eq!(f.len(), super::super::formula::FUNCTIONS.len());
        assert_eq!((f[2].get("name").unwrap().as_str(), f[2].get("args").unwrap().as_str()), (Some("MA"), Some("x, n")));
        assert!(f.iter().all(|x| x.get("desc").and_then(|d| d.as_str()).is_some_and(|d| !d.is_empty())));
    }
    #[test] fn a_formula_spec_runs_and_optimizes_through_the_routes() {
        let st = state(400, "SPX", true);
        let f = crate::lab::json::esc(FORMULA);
        let spec = format!(r#"{{"v":1,"entry":{{"all":[{{"lhs":{{"formula":"{f}","name":"Mom"}},"op":"crosses_above","rhs":{{"value":0}}}}]}},"exit":{{"any":[{{"lhs":{{"s":"bars_held"}},"op":">=","rhs":{{"value":3}}}}]}},"optimize":[{{"path":"entry.all[0].lhs.p.Fast","from":5,"to":15,"step":5}}]}}"#);
        let run = run_json(&st, &q(&[("spec", &spec)])).unwrap();
        assert!(run.contains(LABEL));
        let opt = optimize_json(&st, &q(&[("spec", &spec)])).unwrap();
        assert!(opt.contains("entry.all[0].lhs.p.Fast"), "{}", &opt[..opt.len().min(600)]);
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
        assert_eq!(series_json(&st, &q(&[("id", "close"), ("tf", "H")])).unwrap_err().message(), "tf must be D, W, M, Q, Y or nD (2D to 63D)");
        assert!(series_json(&st, &q(&[("id", "close"), ("tf", "Q")])).unwrap().contains("\"tf\":\"Q\""));
    }
    #[test] fn run_is_labelled_simulated_and_reuses_book_accounting() {
        let st = state(400, "SPX", false);
        let j = run_json(&st, &q(&[("from", "2015-02-01")])).unwrap();
        assert!(j.starts_with(&format!("{{\"label\":\"{LABEL}\",\"kind\":\"lab_simulation\"")), "{}", &j[..200]);
        assert!(j.contains("\"instrument\":{\"ticker\":\"SPX\",\"type\":\"index\",\"native\":false,\"market\":\"us\",\"currency\":\"USD\"}"));
        assert!(j.contains("\"reason\":\"exit.any[0]: bars_held >= 3\"") && j.contains("\"fees_bps\":5"));
        let v = crate::lab::json::parse(&j).unwrap();
        assert!(v.get("stats").and_then(|s| s.get("trades")).and_then(|t| t.as_f64()).unwrap() > 5.);
        assert!(!j.contains("\"gate\"") && !j.contains("\"claim\"") && !j.contains("\"readiness\""));
    }
    #[test] fn responses_reading_future_session_dates_carry_the_adaptation_note() {
        let st = state(700, "SPX", false);
        let note = format!(",\"adaptations\":[\"{}\"]}}", esc(crate::lab::periods::SESSION_DATES_NOTE));
        // nested inside a `not`, after the optimized comparison
        let spec = SPEC.replacen(r#"]},"exit":"#, r#",{"not":{"lhs":{"s":"sessions_left","out":"m"},"op":"<","rhs":{"value":1}}}]},"exit":"#, 1);
        assert!(spec.contains("sessions_left"));
        for j in [run_json(&st, &q(&[("spec", &spec)])).unwrap(), optimize_json(&st, &q(&[("spec", &spec)])).unwrap(), series_json(&st, &q(&[("id", "sessions_left")])).unwrap()] {
            assert!(j.ends_with(&note), "{}", &j[j.len().saturating_sub(300)..]);
            assert!(crate::lab::json::parse(&j).is_ok());
        }
        // everything else keeps its exact body: no adaptations key at all
        for j in [run_json(&st, &q(&[])).unwrap(), optimize_json(&st, &q(&[])).unwrap(), series_json(&st, &q(&[("id", "session")])).unwrap()] {
            assert!(!j.contains("adaptations") && j.ends_with(r#""provenance":null}"#), "{}", &j[j.len().saturating_sub(200)..]);
        }
    }
    #[test] fn invalid_bar_in_window_withholds_runs_and_optimizations() {
        let st = state_bad(400, "SPX", false, 100); // index 100 -> 2015-04-17 (open below the low)
        let date = st.ctxs["SPX"].r[100].date.clone();
        for e in [run_json(&st, &q(&[("from", "2015-02-01")])).unwrap_err(), optimize_json(&st, &q(&[("from", "2015-02-01")])).unwrap_err()] {
            assert_eq!(e.status(), "needs_data");
            assert!(e.message().contains(&format!("SPX: invalid bar on {date} (open below the low)")), "{}", e.message());
        }
        let after = st.ctxs["SPX"].r[110].date.clone();
        assert!(run_json(&st, &q(&[("from", after.as_str())])).is_ok());
        assert!(series_json(&st, &q(&[("id", "close")])).is_ok());
    }
    #[test] fn demoted_and_stale_origins_are_refused_and_echo_the_spec_version() {
        let e = demoted(Some("not admitted".into())).unwrap_err();
        assert_eq!(e.status(), "needs_data");
        assert!(e.message().contains("not available for Customize: not admitted"));
        assert!(demoted(None).is_ok());
        let d = templates::method(templates::TEMPLATES[0].method).unwrap();
        assert!(labelled(Some(&d), 1).1.ends_with("\"template_version\":1}"));
        assert_eq!(labelled(None, 1).1, "null");
    }
    #[test] fn errors_are_typed_and_name_the_problem() {
        let st = state(400, "SPX", false);
        let cases: Vec<(Query, &str, &str)> = vec![
            (q(&[("spec", "{")]), "invalid", "not valid JSON"),
            (q(&[("ticker", "NOPE")]), "invalid", "unknown ticker NOPE"),
            (q(&[("from", "2014-01-01")]), "needs_data", "precedes admitted data"),
            (q(&[("to", "2031-01-01")]), "needs_data", "exceeds admitted data"),
            (q(&[("from", "2015-02-30")]), "invalid", "valid YYYY-MM-DD"),
            (q(&[("rollover", "4")]), "needs_data", "contract"),
            (q(&[("spec", &SPEC.replace("\"close\"},\"op\"", "\"obv\"},\"op\""))]), "needs_data", "volume, which is missing on some admitted bars"),
            (q(&[("spec", &SPEC.replace("\"v\":1,", "\"v\":1,\"execution\":{\"cost_bps\":9},"))]), "invalid", "differs from the cost query"),
        ];
        for (query, status, needle) in cases {
            let e = run_json(&st, &query).unwrap_err();
            assert_eq!(e.status(), status, "{e:?}");
            assert!(e.message().contains(needle), "{needle:?} not in {:?}", e.message());
        }
        let unknown = state(400, "MYSTERY", false);
        assert_eq!(run_json(&unknown, &q(&[("ticker", "MYSTERY")])).unwrap_err().status(), "needs_data");
    }
    #[test] fn stale_index_opens_move_or_refuse_the_start() {
        let st = state(700, "DJI", false); // DJI opens are stale before 2016-01-01; rows start 2015
        let e = run_json(&st, &q(&[("ticker", "DJI"), ("from", "2015-06-01")])).unwrap_err();
        assert_eq!(e.status(), "needs_data");
        assert!(e.message().contains("2016-01-01") && e.message().contains("DJI"), "{}", e.message());
        let j = run_json(&st, &q(&[("ticker", "DJI")])).unwrap();
        let v = crate::lab::json::parse(&j).unwrap();
        assert!(v.get("from").unwrap().as_str().unwrap() >= "2016-01-01");
        assert_eq!(v.get("start").unwrap().get("reason").unwrap().as_str(), Some("opens before 2016-01-01 are stale"));
        assert_eq!(v.get("start").unwrap().get("date"), v.get("from"));
        let o = optimize_json(&st, &q(&[("ticker", "DJI")])).unwrap();
        assert!(o.contains("\"start\":{\"date\":\"2016-01-01\"") || o.contains("\"reason\":\"opens before 2016-01-01 are stale\""));
        assert!(run_json(&st, &q(&[("ticker", "DJI"), ("from", "2016-02-01")])).unwrap().contains("\"start\":null"));
        let short = state(100, "DJI", false); // all bars precede the cut
        assert_eq!(run_json(&short, &q(&[("ticker", "DJI")])).unwrap_err().status(), "needs_data");
        let ok = run_json(&state(400, "SPX", false), &q(&[])).unwrap();
        assert!(ok.contains("\"start\":null"));
    }
    #[test] fn run_rejects_split() {
        let e = run_json(&state(400, "SPX", false), &q(&[("split", "2015-06-01")])).unwrap_err();
        assert_eq!((e.status(), e.message()), ("invalid", "split applies to /api/lab/optimize only"));
    }
    #[test] fn degenerate_windows_are_errors_not_panics() {
        let st = state(400, "SPX", false);
        for (f, t) in [("2015-03-01", "2015-03-01"), ("2015-03-01", "2015-02-01"), ("2015-03-01", "2015-03-02")] {
            let r = guarded(|| run_json(&st, &q(&[("from", f), ("to", t)])));
            assert!(r.is_err() || r.is_ok(), "never unwinds");
            if let Err(e) = r { assert_ne!(e.message(), "internal error while evaluating this request; no result is published"); }
        }
    }
    #[test] fn optimize_ranks_like_the_book_optimizer() {
        let st = state(700, "SPX", false);
        let j = optimize_json(&st, &q(&[("split", "2016-06-01")])).unwrap();
        let v = crate::lab::json::parse(&j).unwrap();
        assert_eq!(v.get("combos").and_then(|c| c.as_f64()), Some(9.));
        assert!(j.starts_with(&format!("{{\"label\":\"{LABEL}\",\"kind\":\"lab_optimization\"")));
        assert!(j.contains("\"cols\":[\"entry.all[0].rhs.p.length\",\"exit.any[0].rhs.value\",\"total_return\""));
        assert!(j.contains("\"is_total_return\"") && j.contains("\"oos_t_stat\""));
        let crate::lab::json::Json::Arr(rows) = v.get("rows").unwrap() else { panic!() };
        assert_eq!(rows.len(), 9);
        // first row = first value of every dimension, last dimension fastest
        assert!(j.contains("\"rows\":[[3,2,") && j.contains("[3,3,"));
        assert!(v.get("rank").and_then(|r| r.get("eligible")).is_some());
        let big = SPEC.replace("\"from\":3,\"to\":7,\"step\":2", "\"from\":1,\"to\":1000,\"step\":1").replace("\"from\":2,\"to\":4", "\"from\":1,\"to\":100");
        assert!(optimize_json(&st, &q(&[("spec", &big)])).unwrap_err().message().contains("more than 50000"));
        assert!(optimize_json(&st, &q(&[("split", "2015-01-05")])).unwrap_err().message().contains("split"));
        let none = SPEC.split(",\"optimize\"").next().unwrap().to_string() + "}";
        assert!(optimize_json(&st, &q(&[("spec", &none)])).unwrap_err().message().contains("no parameter ranges"));
    }
    #[test] fn optimize_has_an_info_log_and_refuses_an_unknown_mode() {
        use crate::lab::json::{parse, Json};
        let st = state(700, "SPX", false);
        let v = parse(&optimize_json(&st, &q(&[("slim", "1")])).unwrap()).unwrap();
        let info = v.get("info").unwrap();
        assert_eq!((info.get("combos").and_then(Json::as_f64), info.get("threads").and_then(Json::as_f64)), (Some(9.), v.get("threads").and_then(Json::as_f64)));
        assert!(matches!(info.get("warnings"), Some(Json::Arr(_))));
        assert!(optimize_json(&st, &q(&[("mode", "pairs")])).unwrap_err().message().contains("mode must be individual"));
        assert!(optimize_json(&st, &q(&[("mode", "individual")])).unwrap_err().message().contains("no portfolio universe"));
    }
    #[test] fn optimize_table_lists_every_combination_on_request() {
        use crate::lab::json::Json;
        let st = state(700, "SPX", false);
        assert!(!optimize_json(&st, &q(&[("slim", "1")])).unwrap().contains("\"table\""));
        let v = crate::lab::json::parse(&optimize_json(&st, &q(&[("slim", "1"), ("table", "1")])).unwrap()).unwrap();
        let Json::Arr(names) = v.get("table_cols").unwrap() else { panic!() };
        let names: Vec<&str> = names.iter().filter_map(Json::as_str).collect();
        assert_eq!(names, TABLE_KEYS);
        let Json::Arr(rows) = v.get("table").unwrap() else { panic!() };
        assert_eq!(rows.len(), 9);
        let Json::Arr(r0) = &rows[0] else { panic!() };
        assert_eq!(r0.len(), 2 + TABLE_KEYS.len());
        assert_eq!((r0[0].as_f64(), r0[1].as_f64()), (Some(3.), Some(2.)));
        // same values as the full rows for every combination (no split: the ranking period is the full window)
        let w = crate::lab::json::parse(&optimize_json(&st, &q(&[("table", "1")])).unwrap()).unwrap();
        let (Json::Arr(cols), Json::Arr(full_rows)) = (w.get("cols").unwrap(), w.get("rows").unwrap()) else { panic!() };
        let at = |k: &str| cols.iter().position(|c| c.as_str() == Some(k)).unwrap();
        for (t, f) in rows.iter().zip(full_rows) {
            let (Json::Arr(t), Json::Arr(f)) = (t, f) else { panic!() };
            for (i, k) in TABLE_KEYS.iter().enumerate() { assert_eq!(t[2 + i], f[at(k)], "{k}"); }
        }
    }
    #[test] fn split_ranks_on_the_in_sample_columns_only() {
        let n = crate::FULL_COLS.len();
        let row: Vec<f64> = [vec![1.], (0..n).map(|i| i as f64).collect(), vec![9.; 14], (0..n).map(|i| 100. + i as f64).collect()].concat();
        let r = rank_basis(&row, 1, n + 14, true);
        assert_eq!((r[0], r[1], r[n]), (1., 100., 100. + (n - 1) as f64));
        assert_eq!(r.len(), 1 + n + 14);
        assert_eq!(rank_basis(&row[..1 + n + 14], 1, n + 14, false), row[..1 + n + 14].to_vec());
    }
    #[test] fn split_dedup_ignores_oos_columns() {
        let (f, p) = (crate::FULL_COLS.len(), crate::PART_COLS.len());
        let c = outcome_cols(2, f + 2 * p, true);
        assert_eq!(c.len(), 16 + p);
        assert!(c.iter().all(|&i| i < 2 + f + p));
        assert_eq!(outcome_cols(2, f, false).len(), 16);
    }
    #[test] fn split_shows_in_sample_ranked_values() {
        let st = state(700, "SPX", false);
        let j = optimize_json(&st, &q(&[("split", "2016-06-01")])).unwrap();
        let v = crate::lab::json::parse(&j).unwrap();
        let crate::lab::json::Json::Arr(cols) = v.get("cols").unwrap() else { panic!() };
        let names: Vec<&str> = cols.iter().filter_map(|c| c.as_str()).collect();
        for k in ["is_mar", "is_monthly_thickness", "is_expectancy_ratio"] { assert_eq!(names.iter().filter(|n| **n == k).count(), 1, "{k}"); }
        assert_eq!(names.iter().filter(|n| **n == "is_cagr").count(), 1);
        let crate::lab::json::Json::Arr(rows) = v.get("rows").unwrap() else { panic!() };
        let crate::lab::json::Json::Arr(r0) = &rows[0] else { panic!() };
        let crate::lab::json::Json::Arr(base) = v.get("base").unwrap() else { panic!() };
        assert_eq!((r0.len(), base.len() + 2), (names.len(), names.len()));
        let i = names.iter().position(|n| *n == "is_mar").unwrap();
        let rank_mar = names.iter().position(|n| *n == "mar").unwrap();
        assert!(r0[i].as_f64().is_some() || matches!(r0[i], crate::lab::json::Json::Null));
        assert_ne!(i, rank_mar);
        assert!(!optimize_json(&st, &q(&[])).unwrap().contains("is_mar"));
    }
    #[test] fn one_failed_combo_stops_the_grid() {
        let st = state(700, "SPX", false);
        // test hook fails the first combo (length 77); the other combos must not all run
        let spec = SPEC.replace("\"from\":3,\"to\":7,\"step\":2", "\"from\":77,\"to\":1076,\"step\":1");
        let e = optimize_json(&st, &q(&[("spec", &spec)])).unwrap_err();
        assert!(e.message().contains("test hook"), "{}", e.message());
        let done = LAST_DONE.with(|d| d.borrow().as_ref().unwrap().load(Ordering::Relaxed));
        assert!(done < 1500, "{done} of 3000 combos still ran");
    }
    #[test] fn ranked_on_follows_the_split() {
        let st = state(700, "SPX", false);
        let with = optimize_json(&st, &q(&[("split", "2016-06-01")])).unwrap();
        assert!(with.contains("\"ranked_on\":\"in_sample\"") && with.contains("\"split\":\"2016-06-01\""));
        let none = optimize_json(&st, &q(&[])).unwrap();
        assert!(none.contains("\"ranked_on\":\"full\"") && none.contains("\"split\":null") && !none.contains("is_total_return") && !none.contains("oos_"));
    }
    fn num_at(j: &str, path: &[&str]) -> f64 {
        let v = crate::lab::json::parse(j).unwrap();
        path.iter().fold(&v, |v, k| v.get(k).unwrap()).as_f64().unwrap_or_else(|| panic!("{path:?} is null"))
    }
    #[test] fn gl_instruments_are_labelled_provisional_and_annualised_by_their_own_calendar() {
        let (spx, btc) = (run_json(&state(800, "SPX", false), &q(&[])).unwrap(), run_json(&state(800, "CRYPTO:BTCUSD", false), &q(&[("ticker", "CRYPTO:BTCUSD")])).unwrap());
        assert!(btc.contains(&format!("\"instrument\":{}", markets::gl("CRYPTO:BTCUSD").unwrap().json())), "{}", &btc[..400]);
        assert!(btc.contains("\"provisional\":true") && btc.contains("\"label\":\"Provisional · TradingView export CRYPTO:BTCUSD, daily\"") && btc.contains("\"bars_per_year\":365"));
        assert!(!spx.contains("provisional"), "India/US lab output is unchanged");
        let f = (365f64 / 252.).sqrt();
        for path in [&["stats", "sharpe"][..], &["stats", "vol"], &["stats", "gross_sharpe"], &["gross_stats", "sharpe"]] {
            let (a, b) = (num_at(&spx, path), num_at(&btc, path));
            assert!((b - a * f).abs() <= 2e-6 * (1. + b.abs()), "{path:?}: {a} vs {b}");
        }
        for path in [&["stats", "cagr"][..], &["stats", "trades"], &["stats", "max_dd"], &["stats", "trades_per_year"]] { assert_eq!(num_at(&spx, path), num_at(&btc, path), "{path:?}"); }
        let o = optimize_json(&state(800, "CRYPTO:BTCUSD", false), &q(&[("ticker", "CRYPTO:BTCUSD")])).unwrap();
        assert!(o.contains("\"provisional\":true") && o.contains("\"kind\":\"lab_optimization\""));
    }
    #[test] fn gl_data_gaps_are_needs_data_and_unknown_tickers_stay_invalid() {
        let st = state(400, "SPX", false);
        for t in ["HSI:HSI", "CRYPTO:ETHUSD"] {
            let e = run_json(&st, &q(&[("ticker", t)])).unwrap_err();
            assert_eq!(e.status(), "needs_data");
            assert!(e.message().contains(t) && e.message().contains("not staged"), "{}", e.message());
        }
        assert_eq!(run_json(&st, &q(&[("ticker", "HSI:NOPE")])).unwrap_err().status(), "invalid");
        let e = run_json(&state(400, "TVC:CAC40", false), &q(&[("ticker", "TVC:CAC40"), ("spec", r#"{"v":1,"entry":{"all":[{"lhs":{"s":"obv"},"op":">","rhs":{"value":0}}]},"exit":{"any":[{"lhs":{"s":"bars_held"},"op":">=","rhs":{"value":3}}]}}"#)])).unwrap_err();
        assert_eq!(e.status(), "needs_data", "{}", e.message());
    }
    #[test] fn gl_close_only_years_move_the_start() {
        // HSI:HSI rows start 2015 (state()); make 2015 close-only so next-open fills begin 2016-01-01.
        let rows: &'static [Row] = Box::leak((0..700).map(|i| {
            let c = 100. + (i as f64 * 0.3).sin() * 8. + i as f64 * 0.02;
            let date = format!("{}-{:02}-{:02}", 2015 + i / 336, 1 + (i / 28) % 12, 1 + i % 28);
            let (o, h, l) = if i < 336 { (c, c, c) } else { (c - 0.2, c + 1., c - 1.) };
            Row { ticker: "HSI:HSI".into(), date, open: o, high: h, low: l, close: c, volume: None }
        }).collect::<Vec<_>>().into_boxed_slice());
        let st = State { ctxs: BTreeMap::from([("HSI:HSI".into(), Ctx::new(rows, rows, &[]))]), baskets: Default::default(), vix: vec![], market_extra: Default::default() };
        let hsi = |extra: &[(&str, &str)]| { let mut x = q(&[("ticker", "HSI:HSI")]); for (k, v) in extra { x.insert(k.to_string(), v.to_string()); } x };
        let j = run_json(&st, &hsi(&[])).unwrap();
        assert!(j.contains("\"start\":{\"date\":\"2016-01-01\",\"reason\":\"opens before 2016-01-01 are stale\"}"), "{}", &j[..600]);
        let e = run_json(&st, &hsi(&[("from", "2015-06-01")])).unwrap_err();
        assert!(e.status() == "needs_data" && e.message().contains("choose from >= 2016-01-01"), "{}", e.message());
        assert!(markets::instruments_json(&st).contains("\"tradable_from\":\"2016-01-01\""));
    }
    #[test] fn engine_gl_responses_all_carry_the_provisional_flag() {
        let st = state(400, "CRYPTO:BTCUSD", false);
        let cases = [("/api/lab/catalogue", q(&[]), "200 OK"), ("/api/lab/run", q(&[("ticker", "CRYPTO:BTCUSD")]), "200 OK"),
            ("/api/lab/run", q(&[("ticker", "CRYPTO:BTCUSD"), ("spec", "{")]), "400 Bad Request"), ("/api/lab/run", q(&[("ticker", "HSI:HSI")]), "400 Bad Request"),
            ("/api/lab/instruments", q(&[]), "200 OK"), ("/api/lab/nope", q(&[]), "404 Not Found")];
        for (path, query, status) in cases {
            let (s, body) = response(Some("gl"), path, &st, &query);
            assert_eq!(s, status, "{path}: {body}");
            assert!(body.starts_with("{\"provisional\":true,\"provisional_note\":\"Provisional · TradingView export"), "{path}: {}", &body[..80.min(body.len())]);
            crate::lab::json::parse(&body).unwrap();
            let (_, plain) = response(None, path, &st, &query);
            assert!(!plain.starts_with("{\"provisional\""), "India/US engines are not marked");
        }
    }
    #[test] fn instruments_route_lists_ready_and_missing_gl_data() {
        let j = route("/api/lab/instruments", &state(400, "IG:DAX", false), &q(&[])).unwrap().unwrap();
        let v = crate::lab::json::parse(&j).unwrap();
        let Some(crate::lab::json::Json::Arr(rows)) = v.get("instruments") else { panic!("{j}") };
        assert_eq!(rows.len(), markets::GL.len());
        for r in rows {
            let t = r.get("instrument").unwrap().get("ticker").unwrap().as_str().unwrap();
            assert_eq!(r.get("status").unwrap().as_str(), Some(if t == "IG:DAX" { "ready" } else { "needs_data" }), "{t}");
        }
        assert!(j.contains("\"first\":\"2015-01-01\"") && j.contains("\"tradable_from\":null"));
    }
    // ---- Customize (spec 2026-10-16)
    fn spec_of_template(m: &str) -> String { templates::template(m).unwrap().spec.to_string() }
    #[test] fn variant_runs_carry_the_variant_label_and_the_methods_instrument_rule() {
        let st = state(700, "SPX", false);
        let j = run_json(&st, &q(&[("spec", &spec_of_template("r25_rsi25_basic_no_filter"))])).unwrap();
        let d = templates::method("r25_rsi25_basic_no_filter").unwrap();
        assert!(j.starts_with(&format!("{{\"label\":\"{}\",\"kind\":\"lab_simulation\",\"origin\":{{\"method\":{{\"id\":\"r25_rsi25_basic_no_filter\"", templates::variant_label(&d))), "{}", &j[..300]);
        assert!(j.contains("\"dir\":1,\"entry_kind\":\"close\""));
        assert!(!j.contains("\"gate\"") && !j.contains("\"claim\"") && !j.contains("\"readiness\""));
        // hbqts_ma5_25 is a stock method: an index never stands in for a stock (stc9 f1a runs on an index since 2026-10-21)
        let e = run_json(&st, &q(&[("spec", &spec_of_template("hbqts_ma5_25"))])).unwrap_err();
        assert_eq!(e.status(), "needs_data");
        assert!(e.message().starts_with("SPX (index): variants of hbqts") && e.message().contains("individual_stock only"), "{}", e.message());
        let bad = spec_of_template("rsi2_4");
        for (from, to) in [("\"method\":\"rsi2_4\"", "\"method\":\"r3\""), ("\"template_version\":1", "\"template_version\":2"), ("\"book\":\"stc9\"", "\"book\":\"spmt\"")] {
            let e = run_json(&st, &q(&[("spec", &bad.replace(from, to))])).unwrap_err();
            assert!(e.message().starts_with("origin names no book template"), "{}", e.message());
        }
        // optimize keeps the variant label and the instrument rule
        let grid = |m: &str| { let t = spec_of_template(m); format!("{},\"optimize\":[{{\"path\":\"exit.any[0].rhs.value\",\"from\":50,\"to\":60,\"step\":5}}]}}", &t[..t.len() - 1]) };
        let o = optimize_json(&st, &q(&[("spec", &grid("r25_rsi25_basic_no_filter"))])).unwrap();
        assert!(o.starts_with(&format!("{{\"label\":\"{}\",\"kind\":\"lab_optimization\",\"origin\":{{", templates::variant_label(&d))), "{}", &o[..200]);
        assert_eq!(optimize_json(&st, &q(&[("spec", &grid("hbqts_ma5_25").replace("exit.any[0].rhs.value\",\"from\":50,\"to\":60", "exit.any[0].rhs.p.length\",\"from\":20,\"to\":30"))])).unwrap_err().status(), "needs_data");
        // plain lab specs keep the v1 label and a null origin
        assert!(run_json(&st, &q(&[])).unwrap().contains(&format!("\"label\":\"{LABEL}\",\"kind\":\"lab_simulation\",\"origin\":null")));
    }
    #[test] fn engine_gl_variant_bodies_still_carry_the_provisional_flag() {
        let st = state(700, "SPX", false);
        let d = templates::method("r25_rsi25_basic_no_filter").unwrap();
        for (m, status) in [("r25_rsi25_basic_no_filter", "200 OK"), ("hbqts_ma5_25", "400 Bad Request")] {
            let (s, body) = response(Some("gl"), "/api/lab/run", &st, &q(&[("spec", &spec_of_template(m))]));
            assert_eq!(s, status, "{m}: {}", &body[..200.min(body.len())]);
            assert!(body.starts_with("{\"provisional\":true,\"provisional_note\":\"Provisional · TradingView export"), "{m}");
            crate::lab::json::parse(&body).unwrap();
            if s == "200 OK" { assert!(body.contains(&format!("\"label\":\"{}\",\"kind\":\"lab_simulation\",\"origin\":{{", templates::variant_label(&d)))); }
        }
    }
    #[test] fn template_routes_list_explain_and_refuse() {
        let st = state(50, "SPX", false);
        let list = route("/api/lab/templates", &st, &q(&[])).unwrap().unwrap();
        let v = crate::lab::json::parse(&list).unwrap();
        let crate::lab::json::Json::Arr(items) = v.get("templates").unwrap() else { panic!() };
        assert_eq!(items.len(), templates::TEMPLATES.len());
        assert!(list.starts_with(&format!("{{\"label\":\"{}\"", templates::TEMPLATE_LABEL)) && !list.contains("\"claim\"") && !list.contains("\"gate\""));
        let one = template_json(&st, &q(&[("method", "rsi2_4")])).unwrap();
        let v = crate::lab::json::parse(&one).unwrap();
        assert_eq!(v.get("status").unwrap().as_str(), Some("available"));
        assert!(one.contains("\"allowed_types\":[") && one.contains("\"notes\":["));
        let crate::lab::json::Json::Obj(_) = v.get("template").unwrap().get("spec").unwrap() else { panic!("spec is embedded JSON") };
        let no = template_json(&st, &q(&[("method", "r3")])).unwrap();
        assert!(no.contains("\"status\":\"unavailable\"") && no.contains("scale-in"), "{no}");
        assert_eq!(template_json(&st, &q(&[("method", "nope")])).unwrap_err().message(), "unknown book method nope");
        assert_eq!(template_json(&st, &q(&[])).unwrap_err().status(), "invalid");
    }
    #[test] fn fidelity_compares_the_template_with_the_book_engine() {
        let st = state(700, "SPX", false);
        let j = fidelity_json(&st, &q(&[("method", "r25_rsi25_basic_no_filter")])).unwrap();
        let v = crate::lab::json::parse(&j).unwrap();
        let n = |k: &str| v.get(k).and_then(|x| x.as_f64()).unwrap();
        assert!(n("book_trades") > 3., "{j}");
        assert_eq!((n("matched"), n("template_trades"), v.get("reproduces"), n("differences_total")), (n("book_trades"), n("book_trades"), Some(&crate::lab::json::Json::Bool(true)), 0.));
        assert!(j.starts_with(&format!("{{\"label\":\"{}\",\"kind\":\"lab_template_fidelity\"", fidelity::FIDELITY_LABEL)) && j.contains("does not verify"));
        assert_eq!(fidelity_json(&st, &q(&[("method", "r3")])).unwrap_err().status(), "unsupported");
        assert_eq!(fidelity_json(&st, &q(&[("method", "rsi2_4"), ("p.rsi_lt", "5")])).unwrap_err().status(), "invalid");
        let e = fidelity_json(&st, &q(&[("method", "hbqts_ma5_25")])).unwrap_err();
        assert_eq!(e.status(), "needs_data", "{e:?}");
    }
    #[test] fn template_bodies_carry_no_verdict_keys_and_fidelity_refuses_provisional_data() {
        let st = state(700, "SPX", false);
        let bodies = [templates_json(), template_json(&st, &q(&[("method", "rsi2_4")])).unwrap(), template_json(&st, &q(&[("method", "r3")])).unwrap(),
            fidelity_json(&st, &q(&[("method", "r25_rsi25_basic_no_filter")])).unwrap()];
        for b in &bodies { for k in ["\"claim\"", "\"gate\"", "\"readiness\""] { assert!(!b.contains(k), "{k} in {}", &b[..120]); } }
        assert_eq!(fidelity_json(&st, &q(&[("method", "r3")])).unwrap_err().message(), format!("Customize not available — {}", templates::unavailable(&templates::method("r3").unwrap()).unwrap()));
        // the book engine never runs on provisional (engine-gl) data; the template routes stay provisional-marked there
        let gl = state(700, "CRYPTO:BTCUSD", false);
        let e = fidelity_json(&gl, &q(&[("ticker", "CRYPTO:BTCUSD"), ("method", "r25_rsi25_basic_no_filter")])).unwrap_err();
        assert_eq!((e.status(), e.message()), ("needs_data", "book methods do not run on provisional data"));
        for (path, extra, status) in [("/api/lab/templates", vec![], "200 OK"), ("/api/lab/template", vec![("method", "rsi2_4")], "200 OK"),
            ("/api/lab/fidelity", vec![("ticker", "CRYPTO:BTCUSD"), ("method", "r25_rsi25_basic_no_filter")], "400 Bad Request")] {
            let (s, body) = response(Some("gl"), path, &gl, &q(&extra));
            assert_eq!(s, status, "{path}: {body}");
            assert!(body.starts_with("{\"provisional\":true,\"provisional_note\":\"Provisional · TradingView export"), "{path}");
            crate::lab::json::parse(&body).unwrap();
        }
    }

    // ---- Phase 7: execution model, legs and fills, catalogue limits (spec §7, §9)
    #[test] fn runs_carry_the_execution_model_and_scale_out_fills() {
        let st = state(400, "SPX", false);
        let both = r#"{"v":1,"direction":"both","entry":{"all":[{"lhs":{"s":"close"},"op":"crosses_above","rhs":{"s":"sma","p":{"length":5}}}]},"exit":{"any":[],"stop_loss":{"pct":3},"take_profits":[{"pct":1,"size":0.5}]},
            "short_side":{"entry":{"all":[{"lhs":{"s":"close"},"op":"crosses_below","rhs":{"s":"sma","p":{"length":5}}}]},"exit":{"any":[{"lhs":{"s":"bars_held"},"op":">=","rhs":{"value":3}}]}}}"#;
        let j = run_json(&st, &q(&[("spec", both)])).unwrap();
        assert!(j.contains(&format!("\"execution_model\":{{\"kind\":\"index\",\"mode\":\"unit\",\"executable\":false,\"notes\":[\"{}\"],\"events\":[]}}", crate::lab::model::INDEX)), "{}", &j[..400]);
        assert!(j.contains("\"dir\":-1") && j.contains("\"dir\":1") && j.contains("\"fills\":[["), "both sides trade and take-profits fill");
        let o = optimize_json(&st, &q(&[])).unwrap();
        assert!(o.contains("\"execution_model\":{\"kind\":\"index\",\"mode\":\"unit\"") && o.contains("\"events\":null},"), "events are reported by run only");
        let c = crate::lab::json::parse(&catalogue_json()).unwrap();
        assert_eq!((c.get("max_leaves").and_then(|v| v.as_f64()), c.get("max_query_bytes").and_then(|v| v.as_f64())), (Some(40.), Some(16384.)));
        let ex = c.get("execution").unwrap();
        assert_eq!((ex.get("max_pause_bars").and_then(|v| v.as_f64()), catalogue_json().contains("\"ticks\"")), (Some(100000.), false));
        assert!(catalogue_json().contains("\"node_kinds\":[\"all\",\"any\",\"not\",\"at_least\",\"then\",\"setup\"]") && catalogue_json().contains("\"timeframes\":[\"D\",\"W\",\"M\",\"Q\",\"Y\"]"));
    }
    #[test] fn each_instrument_kind_labels_or_refuses_its_result() {
        use crate::lab::model::{CRYPTO_SHORT, INDEX, US_SHORT_UNIT};
        let model = |j: &str| { let s = j.find("\"execution_model\":").unwrap(); j[s..s + j[s..].find("]},").unwrap() + 2].to_string() };
        // an index is a signal study even for a plain long v1 spec; that body carries no legs or fills keys
        let long = run_json(&state(400, "SPX", false), &q(&[])).unwrap();
        assert!(model(&long).contains("\"executable\":false") && model(&long).contains(INDEX));
        assert!(!long.contains("\"legs\"") && !long.contains("\"fills\"") && !long.contains("\"weight\""));
        // a US ETF: long executable without notes; short labelled (borrow not modelled in unit mode)
        let short = r#"{"v":1,"direction":"short","entry":{"all":[{"lhs":{"s":"close"},"op":"crosses_below","rhs":{"s":"sma","p":{"length":5}}}]},"exit":{"any":[{"lhs":{"s":"bars_held"},"op":">=","rhs":{"value":3}}]}}"#;
        let spy = state(400, "SPY", false);
        assert!(model(&run_json(&spy, &q(&[("ticker", "SPY")])).unwrap()).contains("\"kind\":\"etf\",\"mode\":\"unit\",\"executable\":true,\"notes\":[]"));
        assert_eq!(model(&run_json(&spy, &q(&[("ticker", "SPY"), ("spec", short)])).unwrap()),
            format!("\"execution_model\":{{\"kind\":\"etf\",\"mode\":\"unit\",\"executable\":true,\"notes\":[\"{US_SHORT_UNIT}\"],\"events\":[]}}"));
        // crypto stays provisional (Phase 5) and a short is hypothetical
        let btc = run_json(&state(800, "CRYPTO:BTCUSD", false), &q(&[("ticker", "CRYPTO:BTCUSD"), ("spec", short)])).unwrap();
        assert!(model(&btc).contains(CRYPTO_SHORT) && btc.contains("\"provisional\":true"));
        // scale-in legs and risk events reach the run JSON
        let si = r#"{"v":1,"entry":{"all":[{"lhs":{"s":"close"},"op":">","rhs":{"s":"sma","p":{"length":5}}}],"scale_in":{"legs":[0.5,0.5]}},"exit":{"any":[{"lhs":{"s":"bars_held"},"op":">=","rhs":{"value":6}}]},"execution":{"risk":{"max_drawdown_pct":1},"sizing":{"pct_equity":50}}}"#;
        let j = run_json(&spy, &q(&[("ticker", "SPY"), ("spec", si)])).unwrap();
        assert!(j.contains("\"legs\":[[\"") && j.contains("\"weight\":") && model(&j).contains("closed-trade drawdown reached 1%"), "{}", model(&j));
        // futures: no contract data is admitted, on any ticker, for run and optimize
        let fut = SPEC.replace(r#""v":1,"#, r#""v":1,"execution":{"instrument":{"kind":"futures"}},"#);
        for tk in ["SPX", "SPY", "NIFTY 50", "CRYPTO:BTCUSD", "NOPE"] {
            let st = state(400, tk, false);
            for e in [run_json(&st, &q(&[("ticker", tk), ("spec", &fut)])).unwrap_err(), optimize_json(&st, &q(&[("ticker", tk), ("spec", &fut)])).unwrap_err()] {
                assert!(e.status() == "needs_data" && e.message().contains(tk) && e.message().contains("never used as a stand-in"), "{e:?}");
            }
        }
    }
    #[test] fn money_mode_keeps_raw_prices_and_optimizer_sizing() {
        let st = state(400, "SPY", false);
        let spec = r#"{"v":1,"entry":{"all":[{"lhs":{"s":"close"},"op":"crosses_above","rhs":{"s":"sma","p":{"length":5}}}]},"exit":{"any":[{"lhs":{"s":"bars_held"},"op":">=","rhs":{"value":3}}]},"execution":{"capital":100000,"sizing":{"pct_equity":25},"costs":{"commission":{"per_order":10}}},"optimize":[{"path":"execution.sizing.pct_equity","from":25,"to":50,"step":25}]}"#;
        let mut query = q(&[("spec", spec),("ticker", "SPY")]); query.remove("cost");
        let body = run_json(&st, &query).unwrap(); let v = crate::lab::json::parse(&body).unwrap();
        let super::super::json::Json::Arr(trades) = v.get("trades").unwrap() else { panic!() };
        let super::super::json::Json::Arr(dates) = v.get("dates").unwrap() else { panic!() };
        let super::super::json::Json::Arr(eq) = v.get("equity").unwrap() else { panic!() };
        let fin_eq = v.get("money").unwrap().get("final_equity").unwrap().as_f64().unwrap();
        assert!((eq.last().unwrap().as_f64().unwrap()*100000. - fin_eq).abs() < 0.1);
        assert!(!trades.is_empty());
        let entry = trades[0].get("entry").unwrap().as_str().unwrap();
        let px = st.ctxs["SPY"].r.iter().find(|r| r.date == entry).unwrap().open;
        assert!((trades[0].get("entry_px").unwrap().as_f64().unwrap() - px).abs() < 1e-5, "effective prices must remain internal");
        assert!(dates.len() > 20);
        let body = optimize_json(&st, &query).unwrap(); let o = crate::lab::json::parse(&body).unwrap();
        let super::super::json::Json::Arr(rows) = o.get("rows").unwrap() else { panic!() };
        assert_eq!(rows.len(),2);
        let super::super::json::Json::Arr(a) = &rows[0] else { panic!() }; let super::super::json::Json::Arr(b) = &rows[1] else { panic!() };
        assert!(a.iter().skip(1).zip(b.iter().skip(1)).any(|(x,y)| x.as_f64() != y.as_f64()), "optimized sizing must change results");
        // India money shorts are refused through the real API, before any quantity is published.
        let indian = state(400, "NIFTY 50", false);
        assert_eq!(run_json(&indian, &q(&[("spec", &spec.replace("\"v\":1", "\"v\":1,\"direction\":\"short\"")), ("ticker", "NIFTY 50")])).unwrap_err().status(), "needs_data");
    }

    #[test] fn money_model_uses_trusted_india_stock_identity_for_short_refusal() {
        let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target").join(format!("lab-money-identity-{}",std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("index.csv"),format!("basket,ticker,path,first,last,bars,source,source_sha256,csv_sha256,membership,status,has_volume\nnifty50,RELIANCE,quote.csv,2020-01-01,2020-01-02,2,synthetic,{},{},synthetic snapshot,ready,true\n", "a".repeat(64), "b".repeat(64))).unwrap();
        let catalogue = crate::baskets::Catalogue::load(&dir).unwrap();
        let selected = crate::SelectedContext::Native { rows: vec![], evidence: catalogue.native_evidence("basket:nifty50:RELIANCE").unwrap() };
        let spec = rules::parse_spec(&SPEC.replace("\"v\":1", "\"v\":1,\"direction\":\"short\",\"execution\":{\"capital\":10000}" )).unwrap();
        let err=model_of(&selected,"RELIANCE","individual_stock",&spec).unwrap_err();
        assert_eq!(err.status(),"needs_data"); assert!(err.message().contains("India cash shorts"));
    }

    #[test] fn money_refuses_limit_reversals_whose_fill_type_was_not_retained() {
        let st=state(400,"SPY",false);
        let spec=r#"{"v":1,"direction":"both","entry":{"all":[{"lhs":{"s":"close"},"op":">","rhs":{"value":0}}],"limit":{"pct":1}},"exit":{"any":[{"lhs":{"s":"bars_held"},"op":">=","rhs":{"value":3}}]},"short_side":{"entry":{"all":[{"lhs":{"s":"close"},"op":"<","rhs":{"value":0}}]},"exit":{"any":[{"lhs":{"s":"bars_held"},"op":">=","rhs":{"value":3}}]}},"execution":{"capital":10000,"costs":{"borrow_pct_year":0}},"optimize":[{"path":"entry.all[0].rhs.value","from":0,"to":1,"step":1}]}"#;
        let mut query=q(&[("ticker","SPY"),("spec",spec)]);query.remove("cost");
        for err in [run_json(&st,&query).unwrap_err(),optimize_json(&st,&query).unwrap_err()] {
            assert_eq!(err.status(),"unsupported"); assert!(err.message().contains("limit reversal"));
        }
        let safer=spec.replace("\"capital\":10000", "\"capital\":10000,\"opposite\":\"exit\"");
        query.insert("spec".into(),safer); assert!(run_json(&st,&query).is_ok());
    }
    #[test] fn money_mode_books_whole_shares_on_an_etf_and_refuses_an_index() {
        let st = state(400, "SPY", false);
        let money = r#"{"v":1,"entry":{"all":[{"lhs":{"s":"close"},"op":"crosses_above","rhs":{"s":"sma","p":{"length":5}}}]},"exit":{"any":[{"lhs":{"s":"bars_held"},"op":">=","rhs":{"value":3}}]},
            "execution":{"capital":100000,"sizing":{"units":10},"costs":{"commission":{"per_order":1}}}}"#;
        let qm = |spec: &str, t: &str| { let mut m = q(&[("spec", spec), ("ticker", t)]); m.remove("cost"); m };
        let j = run_json(&st, &qm(money, "SPY")).unwrap();
        let v = crate::lab::json::parse(&j).unwrap();
        assert!(j.contains("\"execution_model\":{\"kind\":\"etf\",\"mode\":\"money\",\"executable\":true"), "{}", &j[..300]);
        let Some(crate::lab::json::Json::Arr(rows)) = v.get("ledger") else { panic!("no ledger") };
        assert!(rows.len() > 5 && rows.iter().all(|r| r.get("qty").and_then(|q| q.as_f64()) == Some(10.)));
        let pnl: f64 = rows.iter().map(|r| r.get("pnl").unwrap().as_f64().unwrap()).sum();
        let fin = v.get("money").and_then(|m| m.get("final_equity")).and_then(|f| f.as_f64()).unwrap();
        assert!((fin - 100000. - pnl).abs() < 1e-3, "final equity = capital + the ledger's P&L");
        assert!(run_json(&st, &q(&[("spec", money), ("ticker", "SPY")])).unwrap_err().message().contains("remove the cost and slippage query"));
        let short = money.replace(r#"{"v":1,"#, r#"{"v":1,"direction":"short","#);
        assert!(run_json(&st, &qm(&short, "SPY")).unwrap_err().message().contains("borrow_pct_year"));
        let idx = state(400, "SPX", false);
        assert_eq!(run_json(&idx, &qm(money, "SPX")).unwrap_err().status(), "needs_data");
    }
    #[test] fn execution_limits_leave_todays_ledger_alone_until_set() {
        use crate::lab::json::{parse, Json};
        let money = r#"{"v":1,"entry":{"all":[{"lhs":{"s":"close"},"op":"crosses_above","rhs":{"s":"sma","p":{"length":5}}}]},"exit":{"any":[{"lhs":{"s":"bars_held"},"op":">=","rhs":{"value":3}}]},"execution":{"capital":100000,"sizing":{"pct_equity":50},"costs":{"commission":{"per_order":1}}}}"#;
        let with = |limits: &str| money.replace(r#""capital":100000,"#, &format!(r#""capital":100000,"limits":{limits},"#));
        let run = |st: &State, spec: &str| { let mut m = q(&[("spec", spec), ("ticker", "SPY")]); m.remove("cost"); run_json(st, &m) };
        // the money block minus the limits echo (present only when limits differ from the default)
        let parts = |j: &str| { let v = parse(j).unwrap(); ["trades", "ledger", "money", "equity", "gross_equity", "stats"].map(|k| v.get(k).cloned().map(|x| match x {
            Json::Obj(kv) if k == "money" => Json::Obj(kv.into_iter().filter(|(k, _)| k != "limits").collect()), x => x })) };
        for volume in [false, true] {
            let st = state(400, "SPY", volume);
            let base = parts(&run(&st, money).unwrap());
            assert!(matches!(&base[1], Some(Json::Arr(rows)) if rows.len() > 5));
            for limits in ["{}", r#"{"round_lot":1,"min_shares":0,"min_value":0,"shrink":true,"prev_bar_equity":false,"interest_pct":0}"#] {
                assert_eq!(parts(&run(&st, &with(limits)).unwrap()), base, "{limits} is today's behaviour");
            }
        }
        // 100% of equity plus a 2,000 commission: without limits the order shrinks to the cash (today); limits without shrink skip it
        let st = state(400, "SPY", false);
        let all_in = money.replace(r#""pct_equity":50"#, r#""pct_equity":100"#).replace(r#""per_order":1"#, r#""per_order":2000"#);
        let all_in_with = |limits: &str| all_in.replace(r#""capital":100000,"#, &format!(r#""capital":100000,"limits":{limits},"#));
        let rows = |v: &Json| match v.get("ledger") { Some(Json::Arr(r)) => r.len(), _ => panic!() };
        let today = run(&st, &all_in).unwrap();
        assert_eq!(parts(&run(&st, &all_in_with(r#"{"shrink":true}"#)).unwrap()), parts(&today));
        let v = parse(&run(&st, &all_in_with("{}")).unwrap()).unwrap();
        let short = v.get("money").unwrap().get("limits").unwrap().get("skip_reasons").unwrap().get("insufficient_cash").unwrap().as_f64().unwrap();
        assert!(short > 0. && rows(&v) + short as usize == rows(&parse(&today).unwrap()), "{short}");
        let e = run(&st, &with(r#"{"max_volume_pct":10}"#)).unwrap_err();
        assert!(e.status() == "needs_data" && e.message().contains("volume"), "{}", e.message());
        let st = state(400, "SPY", true); // 1,000 a bar: 10% caps ~500 wanted shares at 100 shares
        let v = parse(&run(&st, &with(r#"{"max_volume_pct":10,"shrink":true}"#)).unwrap()).unwrap();
        let Some(Json::Arr(rows)) = v.get("ledger") else { panic!() };
        assert!(!rows.is_empty() && rows.iter().all(|r| r.get("qty").unwrap().as_f64() == Some(100.)));
        let v = parse(&run(&st, &with(r#"{"max_volume_pct":10}"#)).unwrap()).unwrap();
        assert!(matches!(v.get("ledger"), Some(Json::Arr(rows)) if rows.is_empty()));
        let skipped = v.get("money").unwrap().get("skipped").unwrap().as_f64().unwrap();
        assert!(skipped > 5. && v.get("money").unwrap().get("limits").unwrap().get("skip_reasons").unwrap().get("max_volume_pct").unwrap().as_f64() == Some(skipped));
        let v = parse(&run(&st, &with(r#"{"interest_pct":5}"#)).unwrap()).unwrap();
        let base = parse(&run(&st, money).unwrap()).unwrap();
        let fin = |v: &Json| v.get("money").unwrap().get("final_equity").unwrap().as_f64().unwrap();
        let interest = v.get("money").unwrap().get("limits").unwrap().get("interest").unwrap().as_f64().unwrap();
        assert!(interest > 0. && fin(&v) > fin(&base));
        let Some(Json::Arr(eq)) = v.get("equity") else { panic!() };
        assert!((eq.last().unwrap().as_f64().unwrap() * 100000. / fin(&v) - 1.).abs() < 1e-6, "the curve ends at final equity (curve numbers are rounded)");
    }
}

#[cfg(test)]
mod amibroker_report_tests {
    use super::*;
    use super::super::json::{parse, Json};
    const RULES: &str = r#""entry":{"all":[{"lhs":{"s":"close"},"op":">","rhs":{"value":0}}]},"exit":{"any":[{"lhs":{"s":"bars_held"},"op":">=","rhs":{"value":3}}]}"#;
    fn long(execution: &str) -> Result<Spec, LabError> { rules::parse_spec(&format!(r#"{{"v":1,{RULES},"execution":{execution}}}"#)) }
    fn both(execution: &str) -> Spec {
        rules::parse_spec(&format!(r#"{{"v":1,"direction":"both",{RULES},"short_side":{{{}}},"execution":{execution}}}"#, RULES.replace(r#""op":">""#, r#""op":"<""#))).unwrap()
    }
    // a same-bar close fill reads foreign closes (CBOE VIX) at the prior date; every form of it must be seen
    #[test] fn close_fills_counts_a_delay_zero_side_in_either_form() {
        assert!(close_fills(&long(r#"{"fill":"close"}"#).unwrap()) && close_fills(&long(r#"{"exit_fill":"close"}"#).unwrap()));
        assert!(!close_fills(&long(r#"{"fill":"next_open"}"#).unwrap()) && !close_fills(&long("{}").unwrap()));
        assert!(close_fills(&long(r#"{"prices":{"buy":"close"},"delays":{"buy":0}}"#).unwrap()));
        assert!(close_fills(&long(r#"{"prices":{"sell":"close"},"delays":{"sell":0}}"#).unwrap()));
        assert!(!close_fills(&long(r#"{"prices":{"buy":"close","sell":"high"},"delays":{"buy":1,"sell":2}}"#).unwrap()));
        assert!(close_fills(&both(r#"{"prices":{"cover":"close"},"delays":{"cover":0}}"#)), "the short side's cover counts");
        assert!(!close_fills(&both(r#"{"delays":{"buy":1,"sell":1,"short":1,"cover":1}}"#)));
        assert!(long(r#"{"prices":{"cover":"close"},"delays":{"cover":0}}"#).is_err(), "a key of a side not traded is refused, not ignored");
    }
    #[test] fn the_report_counts_every_listed_trade_by_side_and_every_trade_has_excursions() {
        let rows: &'static [crate::Row] = Box::leak((0..400).map(|i| {
            let c = 100. + (i as f64 * 0.3).sin() * 8.;
            crate::Row { ticker: "SPX".into(), date: format!("{}-{:02}-{:02}", 2015 + i / 336, 1 + (i / 28) % 12, 1 + i % 28), open: c - 0.2, high: c + 1., low: c - 1., close: c, volume: None }
        }).collect::<Vec<_>>().into_boxed_slice());
        let st = State { ctxs: BTreeMap::from([("SPX".into(), Ctx::new(rows, rows, &[]))]), baskets: Default::default(), vix: vec![], market_extra: Default::default() };
        let spec = r#"{"v":1,"direction":"both","entry":{"all":[{"lhs":{"s":"close"},"op":"crosses_above","rhs":{"s":"sma","p":{"length":5}}}]},"exit":{"any":[{"lhs":{"s":"bars_held"},"op":">=","rhs":{"value":4}}]},"short_side":{"entry":{"all":[{"lhs":{"s":"close"},"op":"crosses_below","rhs":{"s":"sma","p":{"length":5}}}]},"exit":{"any":[{"lhs":{"s":"bars_held"},"op":">=","rhs":{"value":4}}]}}}"#;
        let q: Query = [("ticker", "SPX"), ("spec", spec), ("cost", "5")].iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();
        let v = parse(&run_json(&st, &q).unwrap()).unwrap();
        let Some(Json::Arr(trades)) = v.get("trades") else { panic!() };
        let closed = trades.iter().filter(|t| !matches!(t.get("open"), Some(Json::Bool(true)))).count() as f64;
        let n = |side: &str| v.get("report").unwrap().get(side).unwrap().get("trades").unwrap().as_f64().unwrap();
        assert!(n("long") > 0. && n("short") > 0.);
        // the report counts the open trade too (valued at the last close, as the tradesheet); the headline stats stay closed-only
        let listed = trades.len() as f64;
        assert!(listed > closed, "the fixture ends with a trade still open");
        assert_eq!((n("all"), n("long") + n("short")), (listed, listed));
        assert_eq!(Some(closed), v.get("stats").unwrap().get("trades").and_then(Json::as_f64), "the headline stats count closed trades");
        for t in trades {
            let (mae, mfe) = (t.get("mae").unwrap().as_f64().unwrap(), t.get("mfe").unwrap().as_f64().unwrap());
            assert!(mae <= 0. && 0. <= mfe);
        }
        assert!(v.get("report").unwrap().get("buy_hold").unwrap().get("net_profit").unwrap().as_f64().is_some());
        // Buy&Hold statistics column: one trade over the run, same keys as All; default rates echoed
        let r = v.get("report").unwrap();
        let bh = r.get("buyhold").unwrap();
        assert_eq!(bh.get("trades").and_then(Json::as_f64), Some(1.));
        let Some(Json::Arr(close)) = v.get("close") else { panic!() };
        assert!((bh.get("avg").unwrap().as_f64().unwrap() - (close[close.len() - 1].as_f64().unwrap() / close[0].as_f64().unwrap() - 1.)).abs() < 1e-5);
        assert_eq!((r.get("rates").unwrap().get("sharpe").and_then(Json::as_f64), r.get("rates").unwrap().get("upi").and_then(Json::as_f64)), (Some(5.), Some(5.4)));
        // execution.report changes only the two ratios that read a risk-free rate
        let set = spec.replacen(r#""direction":"both","#, r#""direction":"both","execution":{"report":{"rf_sharpe":0,"rf_upi":0}},"#, 1);
        let mut q2 = q.clone(); q2.insert("spec".into(), set);
        let w = parse(&run_json(&st, &q2).unwrap()).unwrap();
        let sys = |v: &Json, k: &str| v.get("report").unwrap().get("system").unwrap().get(k).and_then(Json::as_f64);
        assert_eq!(w.get("report").unwrap().get("rates").unwrap().get("upi").and_then(Json::as_f64), Some(0.));
        assert!(sys(&w, "sharpe_trades") > sys(&v, "sharpe_trades") && sys(&w, "upi") > sys(&v, "upi"));
        for k in ["net_profit", "cagr", "max_sys_dd", "ulcer_index", "k_ratio"] { assert_eq!(sys(&w, k), sys(&v, k), "{k}"); }
        assert_eq!(v.get("trades"), w.get("trades"));
    }
}

#[cfg(test)]
mod origin_admission_tests {
    use super::*;
    #[test] fn variants_keep_their_methods_instrument_types() {
        let stock = templates::method("hbqts_ma5_25").unwrap();
        assert!(origin_admits(Some(&stock), "individual_stock", "RELIANCE").is_ok(), "a stock method's variant runs on an actual stock");
        assert!(origin_admits(Some(&stock), "index", "SPX").is_err());
        // hpetf_3dhl_short stays ETF-only; Manan opened hpetf_rsi75_short to indices as a declared application (cbd6eb9)
        let etf = templates::method("hpetf_3dhl_short").unwrap();
        assert!(origin_admits(Some(&etf), "etf", "SPY").is_ok());
        assert!(origin_admits(Some(&etf), "index", "SPX").is_err(), "an index never stands in for an ETF");
        assert!(origin_admits(None, "index", "SPX").is_ok(), "plain lab strategies have no admission rule");
    }
}

#[cfg(test)]
mod portfolio_api_tests {
    use super::*;
    use super::super::json::{parse,Json};
    const PSPEC:&str=r#"{"v":1,"entry":{"all":[{"lhs":{"s":"close"},"op":">","rhs":{"s":"sma","p":{"length":5}}},{"lhs":{"s":"close","symbol":"NIFTY 50"},"op":">","rhs":{"value":0}}]},"exit":{"any":[{"lhs":{"s":"bars_held"},"op":">=","rhs":{"value":3}}]},"execution":{"capital":100000,"costs":{"commission":{"per_order":2}}},"portfolio":{"universe":{"basket":"nifty50"},"score":{"s":"roc","p":{"length":3}},"max_open":2}}"#;
    fn pq(s:&str)->Query {[("spec".into(),s.into())].into()}
    fn array(j:&Json)->&[Json] {match j {Json::Arr(v)=>v,_=>panic!("expected array")}}
    fn num(j:&Json,k:&str)->f64 {j.get(k).and_then(Json::as_f64).unwrap()}
    fn grid()->String {PSPEC.replace(r#""max_open":2}}"#,r#""max_open":2},"optimize":[{"path":"portfolio.max_open","from":1,"to":2,"step":1},{"path":"entry.all[0].rhs.p.length","from":3,"to":6,"step":3}]}"#)}
    #[test] fn portfolio_api_run_schema_cash_equity_and_same_fills_gross_reconcile() {
        let (_d,st)=super::super::universe::tests::india("api-run");let j=parse(&run_json(&st,&pq(PSPEC)).unwrap()).unwrap();
        assert_eq!(j.get("kind").and_then(Json::as_str),Some("lab_portfolio_simulation"));assert_eq!(j.get("origin"),Some(&Json::Null));
        assert_eq!(j.get("membership").unwrap().get("historical_membership_verified"),Some(&Json::Bool(false)));
        assert_eq!(j.get("reference").unwrap().get("ticker").and_then(Json::as_str),Some("NIFTY 50"));
        let (eq,cash,exposure,gross)=(array(j.get("equity").unwrap()),array(j.get("cash").unwrap()),array(j.get("exposure").unwrap()),array(j.get("gross_equity").unwrap()));
        for i in 0..eq.len(){assert!((eq[i].as_f64().unwrap()*100000.-cash[i].as_f64().unwrap()-exposure[i].as_f64().unwrap()).abs()<0.1);assert!(gross[i].as_f64().unwrap()>=eq[i].as_f64().unwrap());}
        let costs=num(j.get("money").unwrap(),"costs_total");assert!((gross.last().unwrap().as_f64().unwrap()*100000.-eq.last().unwrap().as_f64().unwrap()*100000.-costs).abs()<0.1);
        let foreign=array(j.get("foreign").unwrap());let uses=array(foreign[0].get("uses").unwrap());assert_eq!(uses[0].get("source_scope").and_then(Json::as_str),Some("portfolio_member"));
        assert!(!array(j.get("trades").unwrap()).is_empty());assert!(j.get("skipped").unwrap().get("final_bar").is_some());assert!(j.get("skipped").unwrap().get("data_end").is_some());
        assert!(j.get("provenance").is_some());assert!(j.get("foreign").is_some());assert!(!j.get("membership").unwrap().get("note").unwrap().as_str().unwrap().is_empty());
    }
    #[test] fn portfolio_api_refusals_and_deadline_start_before_universe_load() {
        let (_d,st)=super::super::universe::tests::india("api-refusals");let spec=rules::parse_spec(PSPEC).unwrap();
        for (k,v,note) in [("ticker","NIFTY 50","omit ticker"),("cost","0","money mode"),("slippage","0","money mode"),("from","2000-01-01","precedes"),("to","2030-01-01","exceeds"),("from","bad","YYYY-MM-DD")] {let mut q=pq(PSPEC);q.insert(k.into(),v.into());assert!(run_json(&st,&q).unwrap_err().message().contains(note),"{k}");}
        assert_eq!(portfolio_run_json(&st,&pq(PSPEC),&spec,true).unwrap_err().status(),"unsupported");
        assert!(prepare_portfolio(&st,&pq(PSPEC),&spec,std::time::Instant::now()-std::time::Duration::from_secs(1)).err().unwrap().message().contains("90 s"));
        let invalid=PSPEC.replace(r#""symbol":"NIFTY 50""#,r#""symbol":"NOPE""#);assert_eq!(run_json(&st,&pq(&invalid)).unwrap_err().status(),"invalid");
        let list=PSPEC.replace(r#"{"basket":"nifty50"}"#,r#"{"tickers":["basket:nifty50:B","basket:nifty50:A"]}"#);let j=run_json(&st,&pq(&list)).unwrap();assert!(j.contains("user_list")&&j.contains("hindsight"));
    }
    #[test] fn portfolio_api_default_window_and_score_expressions_work() {
        let (_d,st)=super::super::universe::tests::india("api-window");let s=PSPEC.replace(r#""score":{"s":"roc","p":{"length":3}}"#,r#""score":{"add":[{"s":"roc","p":{"length":3}},{"value":1}]}"#);
        let j=parse(&run_json(&st,&pq(&s)).unwrap()).unwrap();assert_eq!(j.get("from").and_then(Json::as_str),Some("2024-01-01"));assert_eq!(j.get("to").and_then(Json::as_str),Some("2024-03-04"));
    }
    #[test] fn portfolio_api_default_window_is_member_union_and_outside_members_are_refused(){
        let (_d,mut st)=super::super::universe::tests::india("api-default-real-bounds");let reference=st.ctxs["NIFTY 50"].r;
        for (ticker,a,z) in [("SPX",0,60),("SPY",10,55),("QQQ",20,59)] {let rows: &'static [crate::Row]=Box::leak(reference[a..z].iter().map(|r|crate::Row{ticker:ticker.into(),..r.clone()}).collect::<Vec<_>>().into_boxed_slice());st.ctxs.insert(ticker.into(),Ctx::new(rows,rows,&[]));}
        let spec=PSPEC.replace(r#"{"basket":"nifty50"}"#,r#"{"tickers":["SPY","QQQ"]}"#).replace("NIFTY 50","SPX");
        let j=parse(&run_json(&st,&pq(&spec)).unwrap()).unwrap();assert_eq!(j.get("from").and_then(Json::as_str),Some(reference[10].date.as_str()));assert_eq!(j.get("to").and_then(Json::as_str),Some(reference[58].date.as_str()));
        let mut q=pq(&spec);q.insert("from".into(),reference[56].date.clone());let j=parse(&run_json(&st,&q).unwrap()).unwrap();let universe=j.get("portfolio").unwrap().get("universe").unwrap();assert_eq!(num(universe,"admitted"),1.);assert!(array(universe.get("refused").unwrap()).iter().any(|m|m.get("ticker").and_then(Json::as_str)==Some("SPY")));
    }
    #[test] fn portfolio_api_optimize_deterministic_full_ranking_and_bounds() {
        let (_d,st)=super::super::universe::tests::india("api-opt");let s=grid();let j=parse(&optimize_json(&st,&pq(&s)).unwrap()).unwrap();assert_eq!(num(&j,"combos"),4.);assert_eq!(j.get("kind").and_then(Json::as_str),Some("lab_portfolio_optimization"));assert_eq!(j.get("ranked_on").and_then(Json::as_str),Some("full"));assert_eq!(j.get("split"),Some(&Json::Null));assert_eq!(array(j.get("rows").unwrap()).len(),4);
        let k=parse(&optimize_json(&st,&pq(&s)).unwrap()).unwrap();for field in ["base","rows","rank","cols","portfolio","membership"]{assert_eq!(j.get(field),k.get(field),"{field}");}
        let mut q=pq(&s);q.insert("split".into(),"2024-02-01".into());assert!(optimize_json(&st,&q).unwrap_err().message().contains("split is not available"));
        assert!(portfolio_work(61,200).unwrap_err().message().contains("12000"));assert!(portfolio_work(2,201).unwrap_err().message().contains("200"));assert!(portfolio_work(usize::MAX,200).is_err());
    }
    #[test] fn individual_optimize_runs_each_symbol_alone_and_bounds_the_work() {
        let (_d,st)=super::super::universe::tests::india("api-individual");
        let dims=r#""optimize":[{"path":"entry.all[0].rhs.p.length","from":3,"to":6,"step":3},{"path":"exit.any[0].rhs.value","from":2,"to":3,"step":1}]"#;
        // the NIFTY 50 condition is dropped: the fixture's member rows carry bare tickers without an established market
        let s=PSPEC.replace(r#",{"lhs":{"s":"close","symbol":"NIFTY 50"},"op":">","rhs":{"value":0}}"#,"").replace(r#""max_open":2}}"#,&format!(r#""max_open":2}},{dims}}}"#));
        let mut q=pq(&s);q.insert("mode".into(),"individual".into());q.insert("table".into(),"1".into());
        let j=parse(&optimize_json(&st,&q).unwrap()).unwrap();
        assert_eq!(j.get("kind").and_then(Json::as_str),Some("lab_individual_optimization"));assert_eq!(num(&j,"combos"),4.);
        let results=array(j.get("results").unwrap());
        assert_eq!(results.iter().map(|r|r.get("ticker").and_then(Json::as_str).unwrap()).collect::<Vec<_>>(),["basket:nifty50:A","basket:nifty50:B"]);
        assert_eq!(num(j.get("info").unwrap(),"combos"),8.,"{j:?}");assert_eq!(array(j.get("notes").unwrap()).len(),3);
        // each symbol's table and best row are the single-instrument optimizer's for that symbol
        let single=s.replace(r#","portfolio":{"universe":{"basket":"nifty50"},"score":{"s":"roc","p":{"length":3}},"max_open":2}"#,"");
        for r in results {
            let t=r.get("ticker").and_then(Json::as_str).unwrap();
            let one:Query=[("spec",single.as_str()),("ticker",t),("slim","1"),("table","1")].iter().map(|(k,v)|(k.to_string(),v.to_string())).collect();
            let o=parse(&optimize_json(&st,&one).unwrap()).unwrap();
            assert_eq!(r.get("table"),o.get("table"),"{t}");assert_eq!(num(r,"table_rows"),4.);
            let top=array(o.get("rank").unwrap().get("top3").unwrap()).first().map(|c|num(c,"row") as usize);
            assert_eq!(r.get("best"),top.map_or(Some(&Json::Null),|k|o.get("picked").unwrap().get(&k.to_string())),"{t}");
        }
        // bounds and refusals
        assert!(individual_work(60,200).is_ok());assert!(individual_work(61,200).unwrap_err().message().contains("61 symbols × 200 combinations = 12200 backtests; the limit is 12000"));assert!(individual_work(usize::MAX,2).is_err());
        let mut port=pq(&grid());port.insert("mode".into(),"individual".into());
        assert!(optimize_json(&st,&port).unwrap_err().message().contains("portfolio.max_open is a portfolio setting"));
        let mut rot=pq(&s.replace(r#""max_open":2}"#,r#""max_open":2,"mode":"rotation"}"#));rot.insert("mode".into(),"individual".into());
        let e=optimize_json(&st,&rot).unwrap_err();assert!(e.message().contains("rotation"),"{}",e.message());
    }
    #[test] fn portfolio_api_all_reference_bounds_and_final_publication_deadline_are_enforced(){
      let (_d,st)=super::super::universe::tests::india("api-extra-bounds");for (key,date) in [("from","2030-01-01"),("to","2000-01-01")]{let mut q=pq(PSPEC);q.insert(key.into(),date.into());assert_eq!(run_json(&st,&q).unwrap_err().status(),"needs_data");}
      let deadline=std::time::Instant::now()-std::time::Duration::from_millis(1);assert!(finish_portfolio("ready".into(),deadline,false).unwrap_err().message().contains("90 s"));assert!(finish_portfolio("ready".into(),deadline,true).unwrap_err().message().contains("300 s"));
    }
    #[test] fn portfolio_api_resource_caps_have_checked_boundaries_without_large_allocations(){
       assert!(portfolio_resource_counts(&[50_000;10],0,16).is_ok());assert!(portfolio_resource_counts(&[50_001],0,1).unwrap_err().message().contains("50000"));assert!(portfolio_resource_counts(&[50_000;11],0,1).is_err());assert!(portfolio_resource_counts(&[50_000;10],0,17).unwrap_err().message().contains("8000000"));assert!(portfolio_resource_counts(&[1],usize::MAX,1).is_err());assert!(portfolio_resource_counts(&[1],0,usize::MAX).is_err());
    }
    #[test] fn portfolio_api_failed_combination_returns_error_without_partial_ranking() {
        let (_d,st)=super::super::universe::tests::india("api-opt-fail");let s=PSPEC.replace(r#""max_open":2}}"#,r#""max_open":2},"optimize":[{"path":"entry.all[0].rhs.p.length","from":76,"to":78,"step":1}]}"#);
        let e=optimize_json(&st,&pq(&s)).unwrap_err();assert!(e.message().contains("combination failed"));
    }
}

#[cfg(test)]
mod catalogue_state_tests {
 use super::*;
 #[test] fn state_catalogue_identifies_actual_preloaded_etfs_without_guessing_indices(){
    let (_d,mut st)=super::super::universe::tests::india("etf-catalogue");let reference=st.ctxs["NIFTY 50"].r;for ticker in ["SPX","SPY"]{let rows: &'static [crate::Row]=Box::leak(reference.iter().map(|r|crate::Row{ticker:ticker.into(),..r.clone()}).collect::<Vec<_>>().into_boxed_slice());st.ctxs.insert(ticker.into(),Ctx::new(rows,rows,&[]));}
    // Mixed synthetic market context deliberately yields no portfolio capability; actual single-market engine is required.
    st.ctxs.remove("NIFTY 50");let j=super::super::json::parse(&catalogue_for_state(&st)).unwrap();let i=match j.get("portfolio").unwrap().get("instruments").unwrap(){super::super::json::Json::Arr(v)=>v,_=>panic!()};assert_eq!(i.len(),1);assert_eq!(i[0].get("ticker").and_then(super::super::json::Json::as_str),Some("SPY"));assert_eq!(i[0].get("type").and_then(super::super::json::Json::as_str),Some("etf"));
 }
 #[test] fn state_catalogue_caps_and_vix_use_only_actual_same_market_source(){
  let rows: &'static [crate::Row]=Box::leak((0..3).map(|i|crate::Row{ticker:"SPX".into(),date:format!("2024-01-0{}",i+1),open:100.,high:101.,low:99.,close:100.,volume:None}).collect::<Vec<_>>().into_boxed_slice());let mut st=State{ctxs:[("SPX".into(),Ctx::new(rows,rows,&[]))].into(),baskets:Default::default(),vix:vec![],market_extra:Default::default()};
  let empty=catalogue_for_state(&st);assert!(empty.contains("\"max_foreign\":4"));assert!(empty.contains("\"foreign_volatility\":null"));
  st.vix=st.ctxs["SPX"].r[..3].iter().map(|r|crate::Row{ticker:"CBOE VIX".into(),..r.clone()}).collect();
  assert!(catalogue_for_state(&st).contains("\"foreign_volatility\":\"CBOE VIX\""));st.vix[0].ticker="INDIA VIX".into();assert!(catalogue_for_state(&st).contains("\"foreign_volatility\":null"));
  st.vix[0].ticker="CBOE VIX".into();st.vix[0].date=st.vix[1].date.clone();assert!(catalogue_for_state(&st).contains("\"foreign_volatility\":null"));
  assert!(catalogue_json().contains("\"foreign_volatility\":null"));
  let (_d,india)=super::super::universe::tests::india("catalogue-caps");let j=super::super::json::parse(&catalogue_for_state(&india)).unwrap();let p=j.get("portfolio").unwrap();assert_eq!(p.get("max_combos").and_then(super::super::json::Json::as_f64),Some(200.));let b=match p.get("baskets").unwrap(){super::super::json::Json::Arr(v)=>v,_=>panic!()};assert_eq!(b.len(),2);assert!(b.iter().any(|r|r.get("id").and_then(super::super::json::Json::as_str)==Some("nse500")&&r.get("members").and_then(super::super::json::Json::as_f64)==Some(130.)));
 }
}

#[cfg(test)]
mod money_settings_tests {
    use super::*;
    use crate::Row;
    use super::super::json::{parse, Json};
    fn rows(ticker: &str, n: usize, keep: impl Fn(usize) -> bool) -> &'static [Row] {
        Box::leak((0..n).filter(|i| keep(*i)).map(|i| { let c = 100. + (i as f64 * 0.3).sin() * 8. + i as f64 * 0.02;
            Row { ticker: ticker.into(), date: format!("{}-{:02}-{:02}", 2015 + i / 336, 1 + (i / 28) % 12, 1 + i % 28), open: c - 0.2, high: c + 1., low: c - 1., close: c, volume: Some(1000.) } }).collect::<Vec<_>>().into_boxed_slice())
    }
    /// SPY misses every 10th SPX day; SPX misses SPY's day 5.
    fn st() -> State {
        let (spy, spx) = (rows("SPY", 400, |i| i % 10 != 3), rows("SPX", 400, |i| i != 5));
        State { ctxs: BTreeMap::from([("SPY".into(), Ctx::new(spy, spy, &[])), ("SPX".into(), Ctx::new(spx, spx, &[]))]), baskets: Default::default(), vix: vec![], market_extra: Default::default() }
    }
    const RULES: &str = r#""entry":{"all":[{"lhs":{"s":"close"},"op":"crosses_above","rhs":{"s":"sma","p":{"length":5}}}]},"exit":{"any":[{"lhs":{"s":"bars_held"},"op":">=","rhs":{"value":3}}]}"#;
    fn run(st: &State, execution: &str) -> Result<String, LabError> {
        let q: Query = [("ticker", "SPY"), ("spec", &format!(r#"{{"v":1,{RULES},"execution":{execution}}}"#) as &str)].iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();
        run_json(st, &q)
    }
    #[test] fn defaults_are_todays_results() {
        let st = st();
        assert_eq!(run(&st, r#"{"fill":"next_open"}"#).unwrap(), run(&st, r#"{"fill":"next_open","tick_size":0,"pad_align":false}"#).unwrap());
        let money = r#"{"capital":100000,"sizing":{"pct_equity":100},"limits":{"shrink":true}}"#;
        assert_eq!(run(&st, money).unwrap(), run(&st, &money.replace(r#""shrink":true"#, r#""shrink":true,"margin_pct":100"#)).unwrap());
    }
    #[test] fn tick_size_rounds_only_stop_exits_against_the_trader() {
        let st = st();
        // AmiBroker: the tick size affects only trades exited by the built-in stops; entries and rule exits keep their prices
        let rules = format!("{},\"stop_loss\":{{\"pct\":0.5}}}}", RULES.strip_suffix('}').unwrap());
        let go = |execution: &str| { let q: Query = [("ticker", "SPY"), ("spec", &format!(r#"{{"v":1,{rules},"execution":{execution}}}"#) as &str)].iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();
            match parse(&run_json(&st, &q).unwrap()).unwrap().get("trades") { Some(Json::Arr(t)) => t.iter().map(|t| (t.get("entry_px").unwrap().as_f64().unwrap(), t.get("exit_px").unwrap().as_f64().unwrap(),
                t.get("reason").unwrap().as_str().unwrap().to_string(), t.get("open") == Some(&Json::Bool(true)))).collect::<Vec<_>>(), _ => panic!() } };
        let (raw, snapped) = (go(r#"{"fill":"next_open"}"#), go(r#"{"fill":"next_open","tick_size":0.25}"#));
        assert_eq!(raw.len(), snapped.len());
        let on = |x: f64| ((x / 0.25) - (x / 0.25).round()).abs() < 1e-6;
        let mut stopped = 0;
        for ((e, x, why, open), (se, sx, swhy, _)) in raw.iter().zip(&snapped) {
            assert_eq!((e, why), (se, swhy), "entries and exit reasons are unchanged");
            if why.starts_with("exit.stop_loss") && !open { stopped += 1; assert!(on(*sx) && sx <= x && x - sx < 0.25 + 1e-6, "a stop sells down: {x} → {sx}"); }
            else { assert_eq!(x, sx, "{why}: not a stop exit"); }
        }
        assert!(stopped > 0 && raw.iter().any(|t| !on(t.0)), "the fixture has stop exits and off-grid entries");
    }
    #[test] fn account_margin_doubles_the_position_at_fifty_percent() {
        let st = st();
        let qty = |margin: &str| match parse(&run(&st, &format!(r#"{{"capital":100000,"sizing":{{"pct_equity":100}},"limits":{{"shrink":true{margin}}}}}"#)).unwrap()).unwrap().get("ledger") {
            Some(Json::Arr(r)) => r[0].get("qty").unwrap().as_f64().unwrap(), _ => panic!() };
        let (one, two) = (qty(""), qty(r#","margin_pct":50"#));
        assert!(two >= 2. * one - 1. && two <= 2. * one + 1., "{one} {two}");
    }
    #[test] fn pad_align_follows_the_benchmark_calendar_and_says_so() {
        let st = st();
        let v = parse(&run(&st, r#"{"fill":"next_open","pad_align":true}"#).unwrap()).unwrap();
        let Some(Json::Arr(dates)) = v.get("dates") else { panic!() };
        assert_eq!(dates.len(), 399, "SPX's 399 days within SPY's first (day 0) and last (day 399)");
        assert!(dates.iter().all(|d| d.as_str() != Some("2015-01-06")), "a day the reference lacks is dropped");
        let Some(Json::Arr(events)) = v.get("execution_model").unwrap().get("events") else { panic!() };
        assert_eq!(events[0].as_str(), Some("Pad and align to SPX: 40 bars padded with the previous close, 1 dropped"));
        // Explore (and so Scan) reads the same padded bars as the run
        let spec = format!(r#"{{"v":1,{RULES},"execution":{{"fill":"next_open","pad_align":true}}}}"#);
        let q: Query = [("ticker", "SPY"), ("mode", "explore"), ("spec", spec.as_str())].iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();
        assert_eq!(parse(&super::super::scan::scan_json(&st, &q).unwrap()).unwrap().get("dates"), v.get("dates"));
        let spx = State { ctxs: BTreeMap::from([("SPY".into(), Ctx::new(rows("SPY", 50, |_| true), rows("SPY", 50, |_| true), &[]))]), ..st };
        assert_eq!(run(&spx, r#"{"fill":"next_open","pad_align":true}"#).unwrap_err().status(), "needs_data", "no reference loaded");
    }
}
