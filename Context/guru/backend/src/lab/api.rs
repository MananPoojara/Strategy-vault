//! /api/lab/* routes. Book code (data selection, costs, trade validation, equity, stats, ranking) is called
//! read-only; every response carries LABEL and no gate, readiness or book claim.
use super::bars::Bars;
use super::catalogue::{self, Placement};
use super::json::esc;
use super::markets;
use super::resample::Tf;
use super::rules::{self, LabError, LABEL};
use super::series::{self, Cache, Compiled, Frames};
use super::rules::Spec;
use super::simulate::simulate;
use super::{fidelity, templates};
use crate::{accounting, connors::{Ctx, StrategyDef}, instrument_policy, trade_validation, State};
use std::collections::BTreeMap;
use std::net::TcpStream;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;

type Query = BTreeMap<String, String>;
/// Wall-clock bound for one lab optimization (the gateway allows 600 s).
const OPT_SECONDS: u64 = 300;
const STOPPED: &str = "optimization stopped after another combination failed";
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
        "/api/lab/catalogue" => |_, _| Ok(catalogue_json()),
        "/api/lab/series" => series_json,
        "/api/lab/run" => run_json,
        "/api/lab/optimize" => optimize_json,
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

pub fn catalogue_json() -> String {
    let defs = catalogue::all().into_iter().map(|d| {
        let (tv, ab) = super::sources::names(d.id);
        let params = arr(d.params.iter().map(|p| format!(
            "{{\"key\":\"{}\",\"label\":\"{}\",\"default\":{},\"min\":{},\"max\":{},\"step\":{},\"integer\":{}}}",
            p.key, esc(p.label), full(p.default), full(p.min), full(p.max), full(p.step), p.integer)));
        format!(
            "{{\"id\":\"{}\",\"name\":\"{}\",\"category\":\"{}\",\"placement\":\"{}\",\"needs\":{{\"volume\":{},\"intraday\":{},\"breadth\":{},\"second_symbol\":{},\"fundamentals\":{},\"derivatives\":{},\"onchain\":{}}},\"computed\":{},\"params\":{params},\"outputs\":{},\"first\":{},\"sparse\":{},\"definition\":\"{}\",\"note\":\"{}\",\"tv\":\"{}\",\"ab\":\"{}\"}}",
            d.id, esc(d.name), esc(d.category), if d.placement == Placement::Price { "price" } else { "pane" }, d.needs.volume, d.needs.intraday, d.needs.breadth, d.needs.second_symbol, d.needs.fundamentals, d.needs.derivatives, d.needs.onchain,
            d.compute.is_some(), quoted(d.outputs.iter().copied()), arr(d.first.iter().map(|&f| if f == usize::MAX { "null".into() } else { f.to_string() })), d.sparse, esc(d.definition), esc(d.note), esc(tv), esc(ab))
    });
    format!(
        "{{\"label\":\"{LABEL}\",\"price_sources\":{},\"operators\":{},\"timeframes\":[\"D\",\"W\",\"M\"],\"max_conditions\":{},\"max_spec_bytes\":{},\"max_combos\":{},\"indicators\":{}}}",
        quoted(rules::PRICE_SOURCES.iter().copied()), quoted(rules::OPERATORS.iter().copied()), rules::MAX_CONDITIONS, rules::MAX_SPEC_BYTES, crate::MAX_COMBOS, arr(defs))
}

/// Instrument identity from the server's own data (never from the query): type, native flag, market, currency.
fn instrument(selected: &crate::SelectedContext, ticker: &str) -> (&'static str, String) {
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
fn select<'a>(st: &'a State, q: &Query) -> Result<crate::SelectedContext<'a>, LabError> {
    crate::select_context(st, q).map_err(select_error)
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
fn known_type(kind: &str, ticker: &str) -> Result<(), LabError> {
    if kind == "unknown" { return Err(LabError::NeedsData(format!("{ticker}: instrument type is not established (index, ETF or catalogued stock); no lab result is published"))); }
    Ok(())
}
/// Requested dates must be real and inside admitted coverage; nothing is filled or silently shortened.
/// Also returns the `start` disclosure JSON: lab fills use the next open, which is stale for some indices before a known date.
fn window(ctx: &Ctx, q: &Query) -> Result<(usize, usize, String), LabError> {
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
fn spec_of(q: &Query) -> Result<Spec, LabError> { rules::parse_spec(q.get("spec").ok_or_else(|| LabError::Invalid("spec is required".into()))?) }
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
fn backtest(ctx: &Ctx, c: &Compiled, d: &Bars, from: usize, to: usize, cost: f64) -> Result<crate::Run, LabError> {
    let trades = simulate(c, d, from, to);
    trade_validation::check(&trades, from, to, cost)?;
    let equity = crate::equity_curve(&ctx.c, &trades, from, to, cost);
    if equity.iter().any(|v| !v.is_finite() || *v <= 0.) { return Err(LabError::Invalid("calculation_unavailable: nonpositive or nonfinite equity; no result is published".into())); }
    let bh = (from..=to).map(|k| ctx.c[k] / ctx.c[from]).collect();
    Ok(crate::Run { trades, from, to, equity, bh })
}

pub fn run_json(st: &State, q: &Query) -> Result<String, LabError> {
    if q.get("split").is_some_and(|s| !s.is_empty()) { return Err(LabError::Invalid("split applies to /api/lab/optimize only".into())); }
    let spec = spec_of(q)?;
    let origin = origin_of(&spec)?;
    let q = &with_spec_cost(q, &spec)?;
    let selected = select(st, q)?;
    let held = selected.context(st);
    let ctx: &Ctx = &held;
    let (kind, instrument) = instrument(&selected, &ctx.r[0].ticker);
    known_type(kind, &ctx.r[0].ticker)?;
    origin_admits(origin.as_ref(), kind, &ctx.r[0].ticker)?;
    let (label, origin) = labelled(origin.as_ref(), spec.origin.as_ref().map_or(0, |o| o.version));
    let (from, to, start) = window(ctx, q)?;
    bars_clean(ctx, from, to)?;
    let costs = accounting::Costs::parse(q)?;
    costs.ensure_rollover()?;
    let cost = costs.side_rate();
    let fr = Frames::new(Bars::from_rows(ctx.r));
    let compiled = series::compile(&spec, &fr, &mut Cache::new())?;
    let run = backtest(ctx, &compiled, &fr.d, from, to, cost)?;
    let gross = crate::Run { trades: run.trades.clone(), from, to, equity: crate::equity_curve(&ctx.c, &run.trades, from, to, 0.), bh: run.bh.clone() };
    let trades = arr(run.trades.iter().map(|t| format!(
        "{{\"entry\":\"{}\",\"exit\":\"{}\",\"entry_i\":{},\"exit_i\":{},\"entry_px\":{},\"exit_px\":{},\"ret\":{},\"bars\":{},\"open\":{},\"signal\":\"{}\",\"reason\":\"{}\",\"dir\":{},\"entry_kind\":\"{}\"}}",
        ctx.r[t.legs[0].i].date, ctx.r[t.exit_i].date, t.legs[0].i - from, t.exit_i - from, crate::num(t.legs[0].px), crate::num(t.exit_px),
        crate::num(t.ret(cost)), t.exit_i - t.legs[0].i, t.open, ctx.r[t.signal_i].date, esc(&t.reason), t.dir, t.entry_kind)));
    Ok(format!(
        "{{\"label\":\"{}\",\"kind\":\"lab_simulation\",\"origin\":{origin},\"instrument\":{instrument},\"from\":\"{}\",\"to\":\"{}\",\"start\":{start},\"dates\":{},\"close\":{},\"equity\":{},\"gross_equity\":{},\"bh\":{},\"trades\":{trades},\"stats\":{},\"gross_stats\":{},\"costs\":{},\"provenance\":{}}}",
        esc(&label), ctx.r[from].date, ctx.r[to].date, quoted((from..=to).map(|k| ctx.r[k].date.as_str())), arr((from..=to).map(|k| crate::num(ctx.c[k]))),
        arr(run.equity.iter().map(|v| crate::num(*v))), arr(gross.equity.iter().map(|v| crate::num(*v))), arr(run.bh.iter().map(|v| crate::num(*v))),
        crate::stats_json(&lab_stats(ctx, &run, costs)), crate::stats_json(&gross_stats(ctx, &gross)), costs.json(), crate::source_provenance(st, q)))
}
pub fn optimize_json(st: &State, q: &Query) -> Result<String, LabError> {
    let t0 = std::time::Instant::now();
    let spec = spec_of(q)?;
    if spec.optimize.is_empty() { return Err(LabError::Invalid("optimize lists no parameter ranges".into())); }
    let origin = origin_of(&spec)?;
    let q = &with_spec_cost(q, &spec)?;
    let selected = select(st, q)?;
    let held = selected.context(st);
    let ctx: &Ctx = &held;
    let (kind, instrument) = instrument(&selected, &ctx.r[0].ticker);
    known_type(kind, &ctx.r[0].ticker)?;
    origin_admits(origin.as_ref(), kind, &ctx.r[0].ticker)?;
    let (label, origin) = labelled(origin.as_ref(), spec.origin.as_ref().map_or(0, |o| o.version));
    let (from, to, start) = window(ctx, q)?;
    bars_clean(ctx, from, to)?;
    let costs = accounting::Costs::parse(q)?;
    costs.ensure_rollover()?;
    let cost = costs.side_rate();
    let dates: Vec<&str> = ctx.r.iter().map(|r| r.date.as_str()).collect();
    let split = match q.get("split").filter(|s| !s.is_empty()) {
        None => None,
        Some(s) => Some(Some(dates.partition_point(|d| *d < s.as_str())).filter(|&k| k > from + 20 && k + 20 < to)
            .ok_or_else(|| LabError::Invalid("split must leave more than 20 bars on each side of the range".into()))?),
    };
    let fr = Frames::new(Bars::from_rows(ctx.r));
    let dims = &spec.optimize;
    let combos: usize = dims.iter().map(|d| d.values.len()).product();
    let metrics = |cache: &mut Cache, s: &Spec| -> Result<Vec<f64>, LabError> {
        let c = series::compile(s, &fr, cache)?;
        let stats = lab_stats(ctx, &backtest(ctx, &c, &fr.d, from, to, cost)?, costs);
        let mut out: Vec<f64> = crate::FULL_COLS.iter().map(|k| stats[k]).collect();
        let mut ins = Vec::new();
        for (n, (a, b)) in split.map(|s| [(from, s - 1), (s, to)]).into_iter().flatten().enumerate() {
            let part = lab_stats(ctx, &backtest(ctx, &c, &fr.d, a, b, cost)?, costs);
            out.extend(crate::PART_COLS.iter().map(|k| part[k]));
            if n == 0 { ins = crate::FULL_COLS.iter().map(|k| part[k]).collect(); }
        }
        if split.is_some() { out.extend(extra_rank_cols().iter().map(|k| ins[crate::FULL_COLS.iter().position(|c| c == k).unwrap()])); } // visible is_ ranked values
        out.extend(ins); // trailing in-sample copy of FULL_COLS: ranking basis only, never displayed
        Ok(out)
    };
    let base = metrics(&mut Cache::new(), &spec)?;
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
                    vals.extend(metrics(&mut cache, &s).inspect_err(|_| stop.store(true, Ordering::Relaxed))?);
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
    Ok(format!(
        "{{\"label\":\"{}\",\"kind\":\"lab_optimization\",\"origin\":{origin},\"instrument\":{instrument},\"from\":\"{}\",\"to\":\"{}\",\"start\":{start},\"split\":{},\"combos\":{combos},\"threads\":{threads},\"ms\":{},\"dims\":{dims_json},\"cols\":{},\"base\":{},\"ranked_on\":\"{}\",\"rank\":{rank_json},\"picked\":{{{picked}}},\"rows\":[{rows}],\"costs\":{},\"provenance\":{}}}",
        esc(&label), dates[from], dates[to], split.map_or("null".into(), |s| format!("\"{}\"", dates[s])), t0.elapsed().as_millis(),
        quoted(cols.iter().map(String::as_str)), arr(base.iter().map(|v| crate::num(*v))), if split.is_some() { "in_sample" } else { "full" }, costs.json(), crate::source_provenance(st, q)))
}
/// A spec with `origin` is a customized book template: the template must exist (same book, version not newer).
fn origin_of(spec: &Spec) -> Result<Option<StrategyDef>, LabError> {
    let Some(o) = &spec.origin else { return Ok(None) };
    let found = templates::template(&o.method).filter(|t| o.version <= t.version).and(templates::method(&o.method)).filter(|d| d.book == o.book);
    let d = found.ok_or_else(|| LabError::Invalid(format!("origin names no book template ({} · {} · version {})", o.book, o.method, o.version)))?;
    demoted(templates::unavailable(&d)).map(|_| Some(d))
}
/// A variant of a method the book no longer admits (demoted) is refused; `why` is templates::unavailable().
fn demoted(why: Option<String>) -> Result<(), LabError> {
    why.map_or(Ok(()), |w| Err(LabError::NeedsData(format!("the book method behind this variant is not available for Customize: {w}"))))
}
/// Same predicate as the book's own check (readiness::check): every bar in the window must be a valid OHLC bar.
fn bars_clean(ctx: &Ctx, from: usize, to: usize) -> Result<(), LabError> {
    for r in &ctx.r[from..=to] {
        let why = if ![r.open, r.high, r.low, r.close].iter().all(|n| n.is_finite() && *n > 0.0) { "non-positive or missing price" }
            else if r.low > r.high { "low above the high" } else if r.open < r.low { "open below the low" } else if r.open > r.high { "open above the high" }
            else if r.close < r.low { "close below the low" } else if r.close > r.high { "close above the high" } else { continue };
        return Err(LabError::NeedsData(format!("{}: invalid bar on {} ({why}); choose a window after it", r.ticker, r.date)));
    }
    Ok(())
}
/// A variant keeps its book method's instrument admission: stock methods on actual stocks, ETF methods on ETF prices.
fn origin_admits(d: Option<&StrategyDef>, kind: &str, ticker: &str) -> Result<(), LabError> {
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
    let fr = Frames::new(Bars::from_rows(ctx.r));
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

    #[test] fn catalogue_lists_notes_needs_and_sources() {
        let j = catalogue_json();
        assert!(crate::lab::json::parse(&j).is_ok());
        assert!(j.contains("\"id\":\"cvd\"") && j.contains("\"note\":\"not available yet — needs intraday (lower-timeframe/tick) bars\""));
        assert!(j.contains("\"tv\":\"Relative Strength Index (RSI)\"") && j.contains("\"ab\":\"RSI; RSIa on close"));
        assert!(j.contains("\"needs\":{\"volume\":false,\"intraday\":false,\"breadth\":false,\"second_symbol\":false,\"fundamentals\":false,\"derivatives\":false,\"onchain\":false}"));
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
        // f1a is a stock method: an index never stands in for a stock
        let e = run_json(&st, &q(&[("spec", &spec_of_template("f1a"))])).unwrap_err();
        assert_eq!(e.status(), "needs_data");
        assert!(e.message().starts_with("SPX (index): variants of stc9") && e.message().contains("individual_stock only"), "{}", e.message());
        let bad = spec_of_template("rsi2_4");
        for (from, to) in [("\"method\":\"rsi2_4\"", "\"method\":\"r3\""), ("\"template_version\":1", "\"template_version\":2"), ("\"book\":\"stc9\"", "\"book\":\"spmt\"")] {
            let e = run_json(&st, &q(&[("spec", &bad.replace(from, to))])).unwrap_err();
            assert!(e.message().starts_with("origin names no book template"), "{}", e.message());
        }
        // optimize keeps the variant label and the instrument rule
        let grid = |m: &str| { let t = spec_of_template(m); format!("{},\"optimize\":[{{\"path\":\"exit.any[0].rhs.value\",\"from\":50,\"to\":60,\"step\":5}}]}}", &t[..t.len() - 1]) };
        let o = optimize_json(&st, &q(&[("spec", &grid("r25_rsi25_basic_no_filter"))])).unwrap();
        assert!(o.starts_with(&format!("{{\"label\":\"{}\",\"kind\":\"lab_optimization\",\"origin\":{{", templates::variant_label(&d))), "{}", &o[..200]);
        assert_eq!(optimize_json(&st, &q(&[("spec", &grid("f1a").replace("exit.any[0].rhs.value\",\"from\":50,\"to\":60", "exit.any[0].rhs.value\",\"from\":3,\"to\":6"))])).unwrap_err().status(), "needs_data");
        // plain lab specs keep the v1 label and a null origin
        assert!(run_json(&st, &q(&[])).unwrap().contains(&format!("\"label\":\"{LABEL}\",\"kind\":\"lab_simulation\",\"origin\":null")));
    }
    #[test] fn engine_gl_variant_bodies_still_carry_the_provisional_flag() {
        let st = state(700, "SPX", false);
        let d = templates::method("r25_rsi25_basic_no_filter").unwrap();
        for (m, status) in [("r25_rsi25_basic_no_filter", "200 OK"), ("f1a", "400 Bad Request")] {
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
        let e = fidelity_json(&st, &q(&[("method", "f1a")])).unwrap_err();
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
}

#[cfg(test)]
mod origin_admission_tests {
    use super::*;
    #[test] fn variants_keep_their_methods_instrument_types() {
        let stock = templates::method("f1a").unwrap();
        assert!(origin_admits(Some(&stock), "individual_stock", "RELIANCE").is_ok(), "a stock method's variant runs on an actual stock");
        assert!(origin_admits(Some(&stock), "index", "SPX").is_err());
        let etf = templates::method("hpetf_rsi75_short").unwrap();
        assert!(origin_admits(Some(&etf), "etf", "SPY").is_ok());
        assert!(origin_admits(Some(&etf), "index", "SPX").is_err(), "an index never stands in for an ETF");
        assert!(origin_admits(None, "index", "SPX").is_ok(), "plain lab strategies have no admission rule");
    }
}
