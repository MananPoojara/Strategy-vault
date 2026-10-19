//! /api/lab/walkforward: AmiBroker "easy mode" walk-forward over the lab optimizer (one instrument).
//! Each window runs the existing optimizer (same ranking, combo cap and OPT_SECONDS clock, shared by every window) on its
//! in-sample bars only, takes the best row by `target` and runs that setting on the following out-of-sample bars.
//! Indicators keep their history before a window (warm-up); a window's result never reads a bar after its last bar.
//! Out-of-sample segments are stitched by compounding; that stitched curve and its summary are the only published result.
//! Declared conventions (NOTES): a position open at a window's end, and CAGR when step > oos leaves gaps between windows.
use super::api::{self, OPT_SECONDS};
use super::json::{self, esc, Json};
use super::rules::{self, LabError};
use crate::State;
use std::collections::BTreeMap;

type Query = BTreeMap<String, String>;
/// At most this many windows; also combos × windows ≤ crate::MAX_COMBOS (the per-request optimizer cap).
/// 26 years of NIFTY data in 1-year steps already gives 23 windows; combos × windows and the 300 s clock are the real bounds.
pub const MAX_WINDOWS: usize = 60;
/// "score" is the optimizer's own pick (#1 of its ranking); the others take the leader of that rank statistic among eligible rows.
pub const TARGETS: [&str; 6] = ["score", "cagr", "sharpe", "monthly_thickness", "expectancy_ratio", "mar"];
const KEYS: [&str; 8] = ["total_return", "cagr", "sharpe", "max_dd", "mar", "trades", "win_rate", "profit_factor"];
const MAX_MONTHS: u32 = 600;
pub const NOTES: [&str; 2] = [
    "A position still open at the end of an out-of-sample window is marked at that window's last close (no exit order or exit costs in money mode) and the next window starts flat; it is counted in open_at_edge, not in trades or win rate.",
    "CAGR compounds over the out-of-sample windows' own lengths; when step is longer than oos the gaps between windows are not traded and not counted.",
];

fn inv(m: impl Into<String>) -> LabError { LabError::Invalid(m.into()) }

#[derive(Debug, PartialEq)]
pub struct Window { pub is: (usize, usize), pub oos: (usize, usize) }

/// `date` moved forward by `months` calendar months; the day is kept as text (boundaries are only compared, never shown).
fn add_months(date: &str, months: u32) -> String {
    let (y, m) = (date[..4].parse::<u32>().unwrap_or(0), date[5..7].parse::<u32>().unwrap_or(1));
    let k = y * 12 + (m - 1) + months;
    format!("{:04}-{:02}{}", k / 12, k % 12 + 1, &date[7..])
}

/// Windows over bars from..=to, lengths in months. Rolling moves the in-sample start by `step`; anchored keeps it at `from`.
/// The last out-of-sample window is clipped at `to`; no window is made once the out-of-sample start passes `to`.
pub fn windows(dates: &[&str], from: usize, to: usize, is_m: u32, oos_m: u32, step_m: u32, anchored: bool) -> Result<Vec<Window>, LabError> {
    let at = |months: u32| dates.partition_point(|d| *d < add_months(dates[from], months).as_str());
    let mut out = Vec::new();
    for k in 0u32.. {
        let is_from = if anchored { from } else { at(k * step_m).max(from) };
        let oos_from = at(is_m + k * step_m);
        if oos_from > to { break; }
        if out.len() == MAX_WINDOWS { return Err(inv(format!("walk-forward gives more than {MAX_WINDOWS} windows; lengthen the step or shorten the range"))); }
        let oos_end = at(is_m + k * step_m + oos_m).min(to + 1);
        if oos_from < is_from + 21 { return Err(inv(format!("in-sample window starting {} has {} bars; at least 21 are needed", dates[is_from], oos_from - is_from))); }
        if oos_end <= oos_from { return Err(LabError::NeedsData(format!("no bars in the out-of-sample window starting {}; no result is published", dates[oos_from]))); }
        out.push(Window { is: (is_from, oos_from - 1), oos: (oos_from, oos_end - 1) });
    }
    if out.is_empty() { return Err(LabError::NeedsData("the range is shorter than one in-sample window plus one out-of-sample bar".into())); }
    Ok(out)
}

fn list(j: Option<&Json>) -> &[Json] { if let Some(Json::Arr(v)) = j { v } else { &[] } }
/// The optimizer row chosen by the target (None when no combination was eligible).
fn best_row(opt: &Json, stat: usize) -> Option<usize> {
    let rank = opt.get("rank")?;
    let row = if stat == 0 { list(rank.get("top3")).first()?.get("row") } else {
        list(list(rank.get("groups")).iter().find(|g| g.get("stat").and_then(Json::as_str) == Some(TARGETS[stat]))?.get("rows")).first()
    };
    row?.as_f64().map(|r| r as usize)
}
fn metrics(get: impl Fn(&str) -> f64) -> String {
    format!("{{{}}}", KEYS.iter().map(|k| format!("\"{k}\":{}", crate::num(get(k)))).collect::<Vec<_>>().join(","))
}
fn parse(body: &str) -> Result<Json, LabError> { json::parse(body).map_err(|e| inv(format!("internal: unreadable optimizer output ({e})"))) }

pub fn walkforward_json(st: &State, q: &Query) -> Result<String, LabError> {
    let t0 = std::time::Instant::now();
    let spec = rules::parse_spec(q.get("spec").ok_or_else(|| inv("spec is required"))?)?;
    if spec.portfolio.is_some() { return Err(LabError::Unsupported("walk-forward covers one instrument in this phase; portfolio walk-forward is not available".into())); }
    if spec.optimize.is_empty() { return Err(inv("optimize lists no parameter ranges")); }
    if q.get("split").is_some_and(|s| !s.is_empty()) { return Err(inv("split does not apply to walk-forward; the windows set the in-sample and out-of-sample periods")); }
    let len = |k: &str, default: Option<u32>| -> Result<u32, LabError> {
        match q.get(k).filter(|v| !v.is_empty()) {
            None => default.ok_or_else(|| inv(format!("{k} is required"))),
            Some(v) => v.parse::<u32>().ok().filter(|n| *n >= 1).ok_or_else(|| inv(format!("{k} must be a whole number of at least 1"))),
        }
    };
    let unit_name = q.get("unit").filter(|u| !u.is_empty()).map_or("years", String::as_str);
    let unit = match unit_name { "years" => 12, "months" => 1, _ => return Err(inv("unit must be years or months")) };
    let (is_n, oos_n) = (len("is", None)?, len("oos", None)?);
    let step_n = len("step", Some(oos_n))?;
    if [is_n, oos_n, step_n].iter().any(|n| n * unit > MAX_MONTHS) { return Err(inv(format!("is, oos and step are at most {MAX_MONTHS} months each"))); }
    if step_n < oos_n { return Err(inv("step must be at least the out-of-sample length; a shorter step overlaps out-of-sample windows")); }
    let anchored = match q.get("anchored").map_or("0", String::as_str) { "1" | "true" => true, "" | "0" | "false" => false, _ => return Err(inv("anchored must be 1 or 0")) };
    let target = q.get("target").filter(|t| !t.is_empty()).map_or("score", String::as_str);
    let stat = TARGETS.iter().position(|t| *t == target).ok_or_else(|| inv(format!("target must be one of {}", TARGETS.join(", "))))?;
    let selected = api::select(st, q)?;
    let held = selected.context(st);
    // windows on the padded calendar when "Pad and align" is on: the bars every window's optimize and run use
    let pad = api::padded(st, &spec, &selected, &held)?;
    let pad_ctx = pad.as_ref().map(|(rows, src, _)| api::padded_ctx(&held, rows, src));
    let ctx: &crate::connors::Ctx = pad_ctx.as_ref().unwrap_or(&*held);
    let (from, to, _) = api::window(ctx, q)?;
    let dates: Vec<&str> = ctx.r.iter().map(|r| r.date.as_str()).collect();
    let wins = windows(&dates, from, to, is_n * unit, oos_n * unit, step_n * unit, anchored)?;
    let combos: usize = spec.optimize.iter().map(|d| d.values.len()).product();
    if combos.saturating_mul(wins.len()) > crate::MAX_COMBOS {
        return Err(inv(format!("{combos} combinations × {} windows is more than {} optimizer runs; narrow a range or use fewer windows", wins.len(), crate::MAX_COMBOS)));
    }
    let nd = spec.optimize.len();
    let span = |(a, b): (usize, usize)| format!("{{\"from\":\"{}\",\"to\":\"{}\"}}", dates[a], dates[b]);
    let ranged = |(a, b): (usize, usize)| { let mut q = q.clone(); q.insert("from".into(), dates[a].into()); q.insert("to".into(), dates[b].into()); q };
    let (mut label, mut rows, mut eq_dates, mut equity, mut carry) = (String::new(), Vec::new(), Vec::new(), Vec::new(), 1.0);
    let (mut trades, mut won, mut skipped, mut open_at_edge) = (0.0, 0.0, 0, 0);
    let (mut warnings, mut threads) = (Vec::new(), 1usize); // the Info log
    for w in &wins {
        if t0.elapsed().as_secs() >= OPT_SECONDS { return Err(inv(format!("walk-forward stopped after {OPT_SECONDS} s; use fewer windows or a smaller grid. No partial result is published."))); }
        let mut qi = ranged(w.is);
        qi.insert("slim".into(), "1".into());
        let opt = parse(&api::optimize_since(st, &qi, t0)?)?; // in-sample bars only: the chosen setting cannot see the OOS window
        label = opt.get("label").and_then(Json::as_str).filter(|l| !l.is_empty()).unwrap_or(rules::LABEL).to_string();
        threads = threads.max(opt.get("threads").and_then(Json::as_f64).unwrap_or(1.) as usize);
        let eligible = opt.get("rank").and_then(|r| r.get("eligible")).and_then(Json::as_f64).unwrap_or(0.);
        let head = format!("\"is\":{},\"oos\":{},\"eligible\":{eligible}", span(w.is), span(w.oos));
        let Some(r) = best_row(&opt, stat) else {
            // No setting qualifies in-sample: the OOS window is held flat (no trades), as declared in `skipped`.
            skipped += 1;
            warnings.push(format!("window {} – {}: no eligible combination in-sample; out-of-sample held flat", dates[w.is.0], dates[w.is.1]));
            for k in w.oos.0..=w.oos.1 { eq_dates.push(dates[k].to_string()); equity.push(carry); }
            rows.push(format!("{{{head},\"params\":null,\"is_metrics\":null,\"oos_metrics\":null,\"open_at_end\":false,\"skipped\":\"no eligible combination in-sample; out-of-sample held flat\"}}"));
            continue;
        };
        let cols: Vec<&str> = list(opt.get("cols")).iter().filter_map(Json::as_str).collect();
        let picked: Vec<f64> = list(opt.get("picked").and_then(|p| p.get(&r.to_string()))).iter().map(|v| v.as_f64().unwrap_or(f64::NAN)).collect();
        if picked.len() < nd { return Err(inv("internal: the optimizer did not return the chosen row; no result is published")); }
        let col = |k: &str| cols.iter().position(|c| *c == k).and_then(|i| picked.get(i)).copied().unwrap_or(f64::NAN);
        let applied = rules::apply(&spec, &spec.optimize, &picked[..nd]);
        rules::validate_applied(&applied)?;
        let run = parse(&api::run_spec(st, &ranged(w.oos), applied)?)?;
        let stats = run.get("stats");
        let stat_of = |k: &str| stats.and_then(|s| s.get(k)).and_then(Json::as_f64).unwrap_or(f64::NAN);
        for (d, e) in list(run.get("dates")).iter().zip(list(run.get("equity"))) {
            eq_dates.push(d.as_str().unwrap_or_default().to_string());
            equity.push(carry * e.as_f64().unwrap_or(f64::NAN));
        }
        carry = *equity.last().unwrap_or(&carry);
        let open = list(run.get("trades")).iter().any(|t| matches!(t.get("open"), Some(Json::Bool(true))));
        open_at_edge += usize::from(open);
        if open { warnings.push(format!("window ending {}: a position was open at the end, marked at the last close", dates[w.oos.1])); }
        let n = stat_of("trades");
        if n > 0. { trades += n; won += stat_of("win_rate") * n; }
        let params = spec.optimize.iter().zip(&picked).map(|(d, v)| format!("\"{}\":{}", esc(&d.path), crate::num(*v))).collect::<Vec<_>>().join(",");
        rows.push(format!("{{{head},\"params\":{{{params}}},\"is_metrics\":{},\"oos_metrics\":{},\"open_at_end\":{open}}}", metrics(col), metrics(stat_of)));
    }
    if !carry.is_finite() || equity.iter().any(|v| !v.is_finite()) { return Err(inv("calculation_unavailable: nonfinite out-of-sample equity; no result is published")); }
    let (a, b) = (wins[0].oos.0, wins[wins.len() - 1].oos.1);
    // contiguous windows span a..b; with gaps (step > oos) only the windows' own lengths count
    let yrs = if step_n == oos_n { crate::years(ctx, a, b) } else { wins.iter().map(|w| crate::years(ctx, w.oos.0, w.oos.1)).sum() };
    let summary = format!("{{\"total_return\":{},\"cagr\":{},\"max_dd\":{},\"trades\":{},\"win_rate\":{},\"windows\":{},\"skipped\":{skipped},\"open_at_edge\":{open_at_edge},\"gaps\":{},\"from\":\"{}\",\"to\":\"{}\"}}",
        crate::num(carry - 1.), crate::num(carry.powf(1. / yrs) - 1.), crate::num(crate::max_dd(&equity).0), trades,
        crate::num(if trades > 0. { won / trades } else { f64::NAN }), wins.len(), step_n > oos_n, dates[a], dates[b]);
    Ok(format!("{{\"label\":\"{}\",\"kind\":\"lab_walkforward\",\"target\":\"{target}\",\"unit\":\"{unit_name}\",\"is\":{is_n},\"oos\":{oos_n},\"step\":{step_n},\"anchored\":{anchored},\"combos\":{combos},\"ms\":{},\"windows\":[{}],\"oos_equity\":{{\"dates\":[{}],\"equity\":[{}]}},\"summary\":{summary},\"notes\":[{}]{}}}",
        esc(&label), t0.elapsed().as_millis(), rows.join(","), eq_dates.iter().map(|d| format!("\"{d}\"")).collect::<Vec<_>>().join(","),
        equity.iter().map(|v| crate::num(*v)).collect::<Vec<_>>().join(","), NOTES.iter().map(|n| format!("\"{}\"", esc(n))).collect::<Vec<_>>().join(","),
        api::info_json(t0.elapsed().as_millis(), combos * wins.len(), threads, &warnings)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{connors::Ctx, Row};

    fn monthly(years: std::ops::Range<u32>) -> Vec<String> {
        years.flat_map(|y| (1..=12).flat_map(move |m| [1, 8, 15, 22].map(|d| format!("{y}-{m:02}-{d:02}")))).collect()
    }
    fn wins(dates: &[String], to: usize, is: u32, oos: u32, step: u32, anchored: bool) -> Result<Vec<Window>, LabError> {
        windows(&dates.iter().map(String::as_str).collect::<Vec<_>>(), 0, to, is, oos, step, anchored)
    }
    #[test] fn rolling_and_anchored_windows_in_years() {
        let d = monthly(2010..2015);
        let at = |s: &str| d.iter().position(|x| x == s).unwrap();
        let rolling = wins(&d, d.len() - 1, 24, 12, 12, false).unwrap();
        assert_eq!(rolling, vec![
            Window { is: (0, at("2011-12-22")), oos: (at("2012-01-01"), at("2012-12-22")) },
            Window { is: (at("2011-01-01"), at("2012-12-22")), oos: (at("2013-01-01"), at("2013-12-22")) },
            Window { is: (at("2012-01-01"), at("2013-12-22")), oos: (at("2014-01-01"), at("2014-12-22")) },
        ]);
        let anchored = wins(&d, d.len() - 1, 24, 12, 12, true).unwrap();
        assert_eq!(anchored.iter().map(|w| w.is.0).collect::<Vec<_>>(), vec![0, 0, 0]);
        assert_eq!(anchored.iter().map(|w| w.oos).collect::<Vec<_>>(), rolling.iter().map(|w| w.oos).collect::<Vec<_>>());
        assert_eq!(anchored[2].is.1, at("2013-12-22"));
    }
    #[test] fn months_clip_and_cap() {
        let d = monthly(2010..2015);
        let at = |s: &str| d.iter().position(|x| x == s).unwrap();
        let m = wins(&d, d.len() - 1, 6, 3, 3, false).unwrap();
        assert_eq!((m.len(), m[0].is, m[0].oos), (18, (0, at("2010-06-22")), (at("2010-07-01"), at("2010-09-22"))));
        assert_eq!(m[1].is.0, at("2010-04-01"));
        // data ending mid-year: the last out-of-sample window stops at `to`
        let to = at("2014-06-22");
        let c = wins(&d, to, 24, 12, 12, false).unwrap();
        assert_eq!((c.len(), c[2].oos), (3, (at("2014-01-01"), to)));
        let long = monthly(2010..2020);
        assert!(wins(&long, long.len() - 1, 6, 1, 1, false).unwrap_err().message().contains("more than 60 windows"));
        assert!(wins(&d, d.len() - 1, 3, 1, 1, false).unwrap_err().message().contains("at least 21"));
        assert_eq!(wins(&d, at("2010-12-22"), 12, 12, 12, false).unwrap_err().status(), "needs_data");
    }

    fn state(n: usize, close: impl Fn(usize) -> f64) -> State {
        let rows: &'static [Row] = Box::leak((0..n).map(|i| {
            let c = close(i);
            Row { ticker: "SPX".into(), date: format!("{}-{:02}-{:02}", 2015 + i / 336, 1 + (i / 28) % 12, 1 + i % 28), open: c - 0.2, high: c + 1., low: c - 1., close: c, volume: None }
        }).collect::<Vec<_>>().into_boxed_slice());
        State { ctxs: BTreeMap::from([("SPX".into(), Ctx::new(rows, rows, &[]))]), baskets: Default::default(), vix: vec![], market_extra: Default::default() }
    }
    // a wave with deterministic noise, so in-sample windows hold both winning and losing trades
    fn wave(i: usize) -> f64 { 100. + (i as f64 * 0.3).sin() * 8. + i as f64 * 0.02 + ((i * 7919) % 17) as f64 * 0.4 }
    const SPEC: &str = r#"{"v":1,"entry":{"all":[{"lhs":{"s":"close"},"op":"crosses_above","rhs":{"s":"sma","p":{"length":5}}}]},"exit":{"any":[{"lhs":{"s":"bars_held"},"op":">=","rhs":{"value":3}}]},"optimize":[{"path":"entry.all[0].rhs.p.length","from":3,"to":7,"step":2},{"path":"exit.any[0].rhs.value","from":2,"to":4,"step":1}]}"#;
    fn q(extra: &[(&str, &str)]) -> Query {
        let mut q: Query = [("ticker", "SPX"), ("spec", SPEC), ("cost", "5"), ("is", "1"), ("oos", "1")].iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();
        for (k, v) in extra { q.insert(k.to_string(), v.to_string()); }
        q
    }
    fn get<'a>(v: &'a Json, path: &[&str]) -> &'a Json { path.iter().fold(v, |v, k| v.get(k).unwrap_or_else(|| panic!("{path:?}"))) }

    #[test] fn walkforward_stitches_out_of_sample_windows() {
        let st = state(1400, wave);
        let j = walkforward_json(&st, &q(&[])).unwrap();
        let v = json::parse(&j).unwrap();
        assert_eq!((get(&v, &["kind"]).as_str(), get(&v, &["target"]).as_str()), (Some("lab_walkforward"), Some("score")));
        let w = list(v.get("windows"));
        assert_eq!(w.len(), 4);
        assert_eq!((get(&w[0], &["is", "from"]).as_str(), get(&w[0], &["oos", "from"]).as_str()), (Some("2015-01-01"), Some("2016-01-01")));
        assert_eq!(get(&w[3], &["oos", "to"]).as_str(), Some(st.ctxs["SPX"].r[1399].date.as_str()));
        assert_eq!(get(&v, &["label"]).as_str(), Some(rules::LABEL), "simulated · not a book method");
        assert!(!matches!(w[0].get("params"), Some(Json::Null)), "window 0 chose a setting: {j}");
        for x in w.iter().filter(|x| !matches!(x.get("params"), Some(Json::Null))) {
            assert!(get(x, &["params", "entry.all[0].rhs.p.length"]).as_f64().is_some());
            assert!(get(x, &["oos_metrics", "total_return"]).as_f64().is_some());
        }
        let eq: Vec<f64> = list(get(&v, &["oos_equity"]).get("equity")).iter().map(|x| x.as_f64().unwrap()).collect();
        assert_eq!(eq.len(), list(get(&v, &["oos_equity"]).get("dates")).len());
        assert_eq!(eq.len(), 1400 - 336);
        assert!((get(&v, &["summary", "total_return"]).as_f64().unwrap() - (eq[eq.len() - 1] - 1.)).abs() < 1e-5);
        // the first OOS window compounds from 1: its total return is the stitched curve at its last bar
        let r = get(&w[0], &["oos_metrics", "total_return"]).as_f64().unwrap();
        assert!((eq[335] - 1. - r).abs() < 1e-5);
        let open = w.iter().filter(|x| matches!(x.get("open_at_end"), Some(Json::Bool(true)))).count() as f64;
        assert_eq!(get(&v, &["summary", "open_at_edge"]).as_f64(), Some(open));
        assert_eq!(list(v.get("notes")).len(), NOTES.len());
        // the Info log: every window's grid counted, threads and one warning per window left open
        assert_eq!(get(&v, &["info", "combos"]).as_f64(), Some(9. * 4.));
        assert!(get(&v, &["info", "threads"]).as_f64().unwrap() >= 1. && get(&v, &["info", "ms"]).as_f64().is_some());
        assert_eq!(list(get(&v, &["info"]).get("warnings")).len() as f64, open);
        // target by a rank statistic and anchored windows also run
        let a = json::parse(&walkforward_json(&st, &q(&[("target", "sharpe"), ("anchored", "1")])).unwrap()).unwrap();
        assert!(list(a.get("windows")).iter().all(|x| get(x, &["is", "from"]).as_str() == Some("2015-01-01")));
    }
    #[test] fn the_target_picks_its_ranks_leader() {
        let opt = json::parse(r#"{"rank":{"top3":[{"row":2}],"groups":[{"stat":"cagr","rows":[7]},{"stat":"sharpe","rows":[5,1]}]}}"#).unwrap();
        assert_eq!((best_row(&opt, 0), best_row(&opt, 1), best_row(&opt, 2), best_row(&opt, 3)), (Some(2), Some(7), Some(5), None));
        assert_eq!(best_row(&json::parse(r#"{"rank":{"top3":[],"groups":[]}}"#).unwrap(), 0), None);
    }
    #[test] fn a_skipped_window_holds_the_carry_flat_and_gaps_shorten_the_cagr_years() {
        // year 2 has flat prices: window 1 (in-sample year 2) has no eligible setting, so its out-of-sample year 3 is flat
        let st = state(1400, |i| if (336..672).contains(&i) { 100. } else { wave(i) });
        let v = json::parse(&walkforward_json(&st, &q(&[])).unwrap()).unwrap();
        let w = list(v.get("windows"));
        assert!(matches!(w[1].get("params"), Some(Json::Null)) && !matches!(w[0].get("params"), Some(Json::Null)));
        assert!(list(get(&v, &["info"]).get("warnings")).iter().any(|x| x.as_str().is_some_and(|x| x.contains("no eligible combination"))));
        let eq: Vec<f64> = list(get(&v, &["oos_equity"]).get("equity")).iter().map(|x| x.as_f64().unwrap()).collect();
        assert!(eq[336..672].iter().all(|e| *e == eq[335]), "held at the carry, never reset to 1");
        assert_eq!((get(&v, &["summary", "skipped"]).as_f64(), get(&v, &["summary", "gaps"])), (Some(1.), &Json::Bool(false)));
        // is 1, oos 1, step 2: out-of-sample years 2 and 4 only; CAGR over their two years, not the three-year span
        let g = json::parse(&walkforward_json(&st, &q(&[("step", "2")])).unwrap()).unwrap();
        let (total, cagr) = (get(&g, &["summary", "total_return"]).as_f64().unwrap(), get(&g, &["summary", "cagr"]).as_f64().unwrap());
        let yrs: f64 = list(g.get("windows")).iter().map(|x| {
            let at = |k: &str| st.ctxs["SPX"].r.iter().position(|r| Some(r.date.as_str()) == get(x, &["oos", k]).as_str()).unwrap();
            crate::years(&st.ctxs["SPX"], at("from"), at("to"))
        }).sum();
        assert_eq!(get(&g, &["summary", "gaps"]), &Json::Bool(true));
        assert!(((1. + total).powf(1. / yrs) - 1. - cagr).abs() < 1e-5, "{cagr}");
    }
    #[test] fn walkforward_refusals() {
        let st = state(1400, wave);
        let msg = |extra: &[(&str, &str)]| walkforward_json(&st, &q(extra)).unwrap_err().message().to_string();
        let big = SPEC.replace("\"from\":3,\"to\":7,\"step\":2", "\"from\":1,\"to\":1000,\"step\":1").replace("\"from\":2,\"to\":4", "\"from\":1,\"to\":30");
        assert!(msg(&[("spec", &big)]).contains("windows is more than 50000"));
        assert!(msg(&[("target", "profit")]).contains("target must be one of"));
        assert!(msg(&[("oos", "2"), ("step", "1")]).contains("step must be at least"));
        assert!(msg(&[("unit", "weeks")]).contains("unit must be"));
        assert!(msg(&[("split", "2016-01-01")]).contains("split does not apply"));
        assert!(msg(&[("is", "0")]).contains("whole number"));
        let none = SPEC.split(",\"optimize\"").next().unwrap().to_string() + "}";
        assert!(msg(&[("spec", &none)]).contains("no parameter ranges"));
    }
    #[test] fn out_of_sample_prices_never_change_the_chosen_parameters() {
        // window 0: in-sample 2015 (bars 0..336), out-of-sample from bar 336; change every price from there on
        let (a, b) = (state(1400, wave), state(1400, |i| if i < 336 { wave(i) } else { wave(i) * (1. + 0.08 * (i as f64 * 1.7).sin()) }));
        let first = |st: &State| { let v = json::parse(&walkforward_json(st, &q(&[])).unwrap()).unwrap(); let w = &list(v.get("windows"))[0]; (format!("{:?}", w.get("params")), format!("{:?}", w.get("is_metrics")), format!("{:?}", w.get("oos_metrics"))) };
        let (pa, pb) = (first(&a), first(&b));
        assert!(pa.0.starts_with("Some(Obj"), "window 0 chose a setting: {}", pa.0);
        assert_eq!((&pa.0, &pa.1), (&pb.0, &pb.1));
        assert_ne!(pa.2, pb.2, "the out-of-sample prices did change");
    }
}
