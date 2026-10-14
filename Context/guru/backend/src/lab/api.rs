//! /api/lab/* routes. Book code (data selection, costs, trade validation, equity, stats, ranking) is called
//! read-only; every response carries LABEL and no gate, readiness or book claim.
use super::bars::Bars;
use super::catalogue::{self, Placement};
use super::json::esc;
use super::resample::Tf;
use super::rules::{self, LabError, LABEL};
use super::series::{self, Cache, Compiled, Frames};
use super::rules::Spec;
use super::simulate::simulate;
use crate::{accounting, connors::Ctx, instrument_policy, trade_validation, State};
use std::collections::BTreeMap;
use std::net::TcpStream;

type Query = BTreeMap<String, String>;
/// Wall-clock bound for one lab optimization (the gateway allows 600 s).
const OPT_SECONDS: u64 = 300;

pub fn serve(stream: &mut TcpStream, path: &str, st: &State, q: &Query) {
    match route(path, st, q) {
        None => crate::respond(stream, "404 Not Found", "application/json; charset=utf-8", not_found_json(path).as_bytes()),
        Some(Ok(body)) => crate::respond(stream, "200 OK", "application/json; charset=utf-8", body.as_bytes()),
        Some(Err(e)) => crate::respond(stream, "400 Bad Request", "application/json; charset=utf-8", e.json().as_bytes()),
    }
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
/// Only an unknown ticker is the caller's mistake; every other selection error is a data gap.
fn select_error(e: String) -> LabError {
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
fn window(ctx: &Ctx, q: &Query) -> Result<(usize, usize), LabError> {
    for key in ["from", "to", "split"] {
        if q.get(key).is_some_and(|v| !v.is_empty() && !crate::baskets::valid_date(v)) { return Err(LabError::Invalid(format!("{key} must be a valid YYYY-MM-DD date"))); }
    }
    let (first, last) = (&ctx.r[0].date, &ctx.r[ctx.r.len() - 1].date);
    if q.get("from").is_some_and(|f| !f.is_empty() && f < first) { return Err(LabError::NeedsData(format!("from precedes admitted data starting {first}; no earlier prices are filled"))); }
    if q.get("to").is_some_and(|t| !t.is_empty() && t > last) { return Err(LabError::NeedsData(format!("to exceeds admitted data ending {last}; the range is not silently shortened"))); }
    let (_, from, to, _) = crate::resolve_context(ctx, q)?;
    Ok((from, to))
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
fn backtest(ctx: &Ctx, c: &Compiled, d: &Bars, from: usize, to: usize, cost: f64) -> Result<crate::Run, LabError> {
    let trades = simulate(c, d, from, to);
    trade_validation::check(&trades, from, to, cost)?;
    let equity = crate::equity_curve(&ctx.c, &trades, from, to, cost);
    if equity.iter().any(|v| !v.is_finite() || *v <= 0.) { return Err(LabError::Invalid("calculation_unavailable: nonpositive or nonfinite equity; no result is published".into())); }
    let bh = (from..=to).map(|k| ctx.c[k] / ctx.c[from]).collect();
    Ok(crate::Run { trades, from, to, equity, bh })
}

pub fn run_json(st: &State, q: &Query) -> Result<String, LabError> {
    let spec = spec_of(q)?;
    let q = &with_spec_cost(q, &spec)?;
    let selected = select(st, q)?;
    let held = selected.context(st);
    let ctx: &Ctx = &held;
    let (kind, instrument) = instrument(&selected, &ctx.r[0].ticker);
    known_type(kind, &ctx.r[0].ticker)?;
    let (from, to) = window(ctx, q)?;
    let costs = accounting::Costs::parse(q)?;
    costs.ensure_rollover()?;
    let cost = costs.side_rate();
    let fr = Frames::new(Bars::from_rows(ctx.r));
    let compiled = series::compile(&spec, &fr, &mut Cache::new())?;
    let run = backtest(ctx, &compiled, &fr.d, from, to, cost)?;
    let gross = crate::Run { trades: run.trades.clone(), from, to, equity: crate::equity_curve(&ctx.c, &run.trades, from, to, 0.), bh: run.bh.clone() };
    let trades = arr(run.trades.iter().map(|t| format!(
        "{{\"entry\":\"{}\",\"exit\":\"{}\",\"entry_i\":{},\"exit_i\":{},\"entry_px\":{},\"exit_px\":{},\"ret\":{},\"bars\":{},\"open\":{},\"signal\":\"{}\",\"reason\":\"{}\"}}",
        ctx.r[t.legs[0].i].date, ctx.r[t.exit_i].date, t.legs[0].i - from, t.exit_i - from, crate::num(t.legs[0].px), crate::num(t.exit_px),
        crate::num(t.ret(cost)), t.exit_i - t.legs[0].i, t.open, ctx.r[t.signal_i].date, esc(&t.reason))));
    Ok(format!(
        "{{\"label\":\"{LABEL}\",\"kind\":\"lab_simulation\",\"instrument\":{instrument},\"from\":\"{}\",\"to\":\"{}\",\"dates\":{},\"close\":{},\"equity\":{},\"gross_equity\":{},\"bh\":{},\"trades\":{trades},\"stats\":{},\"gross_stats\":{},\"costs\":{},\"provenance\":{}}}",
        ctx.r[from].date, ctx.r[to].date, quoted((from..=to).map(|k| ctx.r[k].date.as_str())), arr((from..=to).map(|k| crate::num(ctx.c[k]))),
        arr(run.equity.iter().map(|v| crate::num(*v))), arr(gross.equity.iter().map(|v| crate::num(*v))), arr(run.bh.iter().map(|v| crate::num(*v))),
        crate::stats_json(&crate::financial_stats(ctx, &run, costs)), crate::stats_json(&crate::stats(ctx, &gross, 0.)), costs.json(), crate::source_provenance(st, q)))
}
pub fn optimize_json(st: &State, q: &Query) -> Result<String, LabError> {
    let t0 = std::time::Instant::now();
    let spec = spec_of(q)?;
    if spec.optimize.is_empty() { return Err(LabError::Invalid("optimize lists no parameter ranges".into())); }
    let q = &with_spec_cost(q, &spec)?;
    let selected = select(st, q)?;
    let held = selected.context(st);
    let ctx: &Ctx = &held;
    let (kind, instrument) = instrument(&selected, &ctx.r[0].ticker);
    known_type(kind, &ctx.r[0].ticker)?;
    let (from, to) = window(ctx, q)?;
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
        let stats = crate::financial_stats(ctx, &backtest(ctx, &c, &fr.d, from, to, cost)?, costs);
        let mut out: Vec<f64> = crate::FULL_COLS.iter().map(|k| stats[k]).collect();
        let mut ins = Vec::new();
        for (n, (a, b)) in split.map(|s| [(from, s - 1), (s, to)]).into_iter().flatten().enumerate() {
            let part = crate::financial_stats(ctx, &backtest(ctx, &c, &fr.d, a, b, cost)?, costs);
            out.extend(crate::PART_COLS.iter().map(|k| part[k]));
            if n == 0 { ins = crate::FULL_COLS.iter().map(|k| part[k]).collect(); }
        }
        out.extend(ins); // trailing in-sample copy of FULL_COLS: ranking basis only, never displayed
        Ok(out)
    };
    let base = metrics(&mut Cache::new(), &spec)?;
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get()).min(combos);
    // Rows in mixed-radix order (last dimension fastest), as the book optimizer (main.rs optimize()).
    let rows_v: Vec<Vec<f64>> = std::thread::scope(|sc| -> Result<Vec<Vec<f64>>, LabError> {
        let hs: Vec<_> = (0..threads).map(|t| {
            let (metrics, spec) = (&metrics, &spec);
            sc.spawn(move || {
                let mut cache = Cache::new();
                (t..combos).step_by(threads).map(|k| {
                    if t0.elapsed().as_secs() >= OPT_SECONDS {
                        return Err(LabError::Invalid(format!("optimization stopped after {OPT_SECONDS} s; narrow the grid. No partial ranking is published.")));
                    }
                    let (mut r, mut vals) = (k, vec![0.0; dims.len()]);
                    for (j, d) in dims.iter().enumerate().rev() { vals[j] = d.values[r % d.values.len()]; r /= d.values.len(); }
                    let s = rules::apply(spec, dims, &vals);
                    vals.extend(metrics(&mut cache, &s)?);
                    Ok((k, vals))
                }).collect::<Result<Vec<_>, LabError>>()
            })
        }).collect();
        let mut all: Vec<(usize, Vec<f64>)> = Vec::new();
        for h in hs { all.extend(h.join().map_err(|_| LabError::Invalid("optimization worker failed; no result is published".into()))??); }
        all.sort_by_key(|r| r.0);
        Ok(all.into_iter().map(|r| r.1).collect())
    })?;
    // Ranking exactly as the book optimizer (main.rs optimize(): trade-rate floor, outcome columns, conclude_rank).
    let nd = dims.len();
    let ndisp = crate::FULL_COLS.len() + split.map_or(0, |_| 2 * crate::PART_COLS.len());
    let ti = crate::FULL_COLS.iter().position(|c| *c == "trades").unwrap();
    // With a split the ranking reads the in-sample window only; oos_ columns are display-only.
    let rank_rows: Vec<Vec<f64>> = rows_v.iter().map(|r| rank_basis(r, nd, ndisp, split.is_some())).collect();
    let base_rank = rank_basis(&[vec![0.0; nd], base.clone()].concat(), nd, ndisp, split.is_some());
    let yrs = crate::years(ctx, from, split.map_or(to, |s| s - 1));
    let base_tpy = base_rank[nd + ti] / yrs;
    let per_year = (base_tpy * 0.5).clamp(crate::MIN_PER_YEAR.0, crate::MIN_PER_YEAR.1);
    let min_trades = (per_year * yrs).ceil().max(crate::MIN_TRADES);
    let defs: Vec<f64> = dims.iter().map(|d| rules::current(&spec, &d.target)).collect();
    let outcome_cols: Vec<usize> = (nd..nd + 16).chain(nd + crate::FULL_COLS.len()..nd + ndisp).collect();
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
    let dims_json = arr(dims.iter().map(|d| format!("{{\"path\":\"{}\",\"values\":{}}}", esc(&d.path), arr(d.values.iter().map(|v| crate::num(*v))))));
    Ok(format!(
        "{{\"label\":\"{LABEL}\",\"kind\":\"lab_optimization\",\"instrument\":{instrument},\"from\":\"{}\",\"to\":\"{}\",\"split\":{},\"combos\":{combos},\"threads\":{threads},\"ms\":{},\"dims\":{dims_json},\"cols\":{},\"base\":{},\"ranked_on\":\"{}\",\"rank\":{rank_json},\"picked\":{{{picked}}},\"rows\":[{rows}],\"costs\":{},\"provenance\":{}}}",
        dates[from], dates[to], split.map_or("null".into(), |s| format!("\"{}\"", dates[s])), t0.elapsed().as_millis(),
        quoted(cols.iter().map(String::as_str)), arr(base.iter().map(|v| crate::num(*v))), if split.is_some() { "in_sample" } else { "full" }, costs.json(), crate::source_provenance(st, q)))
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
    #[test] fn ranked_on_follows_the_split() {
        let st = state(700, "SPX", false);
        let with = optimize_json(&st, &q(&[("split", "2016-06-01")])).unwrap();
        assert!(with.contains("\"ranked_on\":\"in_sample\"") && with.contains("\"split\":\"2016-06-01\""));
        let none = optimize_json(&st, &q(&[])).unwrap();
        assert!(none.contains("\"ranked_on\":\"full\"") && none.contains("\"split\":null") && !none.contains("is_total_return") && !none.contains("oos_"));
    }
}
