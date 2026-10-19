//! /api/lab/scan: the AmiBroker Analysis window's Scan and Explore over the lab's own rule evaluation (read-only calls to
//! series::compile_with and RCond::holds; nothing here changes how a backtest runs).
//! Scan (`mode=scan`, the default) lists every bar where a raw rule signal is true — Buy, Sell, Short, Cover — for the current
//! instrument (`apply=current`) or for every admitted member of the spec's portfolio universe (`apply=all`).
//! Explore (`mode=explore`, current instrument) returns each bar's OHLCV and the four signals as 0/1.
//! Raw signals are the rules alone, like AmiBroker's Buy/Sell/Short/Cover arrays before the backtester (see NOTES).
use super::api;
use super::bars::Bars;
use super::json::esc;
use super::rules::{self, LabError, Spec, LABEL};
use super::series::{self, Cache, Compiled, Frames};
use crate::State;
use std::collections::BTreeMap;

type Query = BTreeMap<String, String>;
/// A scan publishing more signal rows than this is refused (no truncated list).
pub const MAX_ROWS: usize = 20_000;
/// Members × bars evaluated by one "all symbols" scan.
pub const MAX_CELLS: usize = 2_000_000;
const SECONDS: u64 = 90;
pub const TRADES: [&str; 4] = ["Buy", "Sell", "Short", "Cover"];
pub const NOTES: [&str; 2] = [
    "Raw rule signals, as AmiBroker's Buy/Sell/Short/Cover arrays before the backtester: no position state, edge-only entry, delay, order level, stop or re-entry rule is applied, so a signal is not a filled trade.",
    "An exit rule that reads bars held needs an open position and is never true here; an empty entry rule (rotation) gives no Buy or Short.",
];

fn inv(m: impl Into<String>) -> LabError { LabError::Invalid(m.into()) }

/// Buy, Sell, Short, Cover per bar 0..n of a compiled spec (no open position: `entry` is None).
pub fn signals(c: &Compiled, n: usize) -> [Vec<bool>; 4] {
    let entry = |s: &Compiled, i| !s.entry.is_empty() && s.entry.iter().all(|r| r.holds(i, None));
    let exit = |s: &Compiled, i| s.exit.iter().any(|r| r.holds(i, None));
    let (long, short) = if c.exec.short { (None, Some(c)) } else { (Some(c), c.other.as_deref()) };
    let side = |s: Option<&Compiled>, f: &dyn Fn(&Compiled, usize) -> bool| (0..n).map(|i| s.is_some_and(|s| f(s, i))).collect::<Vec<_>>();
    [side(long, &entry), side(long, &exit), side(short, &entry), side(short, &exit)]
}

/// Bars whose date lies inside the optional from/to query (inclusive), as an index range.
fn dated(dates: &[String], q: &Query) -> Result<std::ops::Range<usize>, LabError> {
    let get = |k: &str| -> Result<Option<&str>, LabError> {
        match q.get(k).filter(|v| !v.is_empty()) {
            Some(v) if !crate::baskets::valid_date(v) => Err(inv(format!("{k} must be a valid YYYY-MM-DD date"))),
            v => Ok(v.map(String::as_str)),
        }
    };
    let (from, to) = (get("from")?, get("to")?);
    let a = from.map_or(0, |f| dates.partition_point(|d| d.as_str() < f));
    let b = to.map_or(dates.len(), |t| dates.partition_point(|d| d.as_str() <= t));
    Ok(a..b.max(a))
}

fn row(symbol: &str, b: &Bars, i: usize, k: usize) -> String {
    format!("[\"{}\",\"{}\",\"{}\",{}]", esc(symbol), b.date[i], TRADES[k], crate::num(b.close[i]))
}
fn push_rows(out: &mut Vec<String>, symbol: &str, b: &Bars, sig: &[Vec<bool>; 4], range: std::ops::Range<usize>) -> Result<(), LabError> {
    for i in range { for (k, s) in sig.iter().enumerate() { if s[i] {
        if out.len() == MAX_ROWS { return Err(inv(format!("the scan finds more than {MAX_ROWS} signals; narrow the From-To dates or the rules (no partial list is published)"))); }
        out.push(row(symbol, b, i, k));
    } } }
    Ok(())
}
fn notes() -> String { format!("[{}]", NOTES.iter().map(|n| format!("\"{}\"", esc(n))).collect::<Vec<_>>().join(",")) }

pub fn scan_json(st: &State, q: &Query) -> Result<String, LabError> {
    let spec = rules::parse_spec(q.get("spec").ok_or_else(|| inv("spec is required"))?)?;
    if spec.exec.futures.is_some() { return Err(LabError::NeedsData("futures contract data is not admitted; a cash index or ETF is never used as a stand-in".into())); }
    let explore = match q.get("mode").map_or("scan", String::as_str) { "scan" | "" => false, "explore" => true, _ => return Err(inv("mode must be scan or explore")) };
    match q.get("apply").map_or("current", String::as_str) {
        "current" | "" => current(st, q, &spec, explore),
        "all" if explore => Err(inv("explore runs on the current symbol; use apply=current")),
        "all" => all(st, q, &spec),
        _ => Err(inv("apply must be current or all")),
    }
}

/// The current instrument, with the run's own admission checks (type, template lineage, clean bars in the window).
fn current(st: &State, q: &Query, spec: &Spec, explore: bool) -> Result<String, LabError> {
    let ticker = q.get("ticker").filter(|t| !t.is_empty()).ok_or_else(|| inv("ticker is required for the current symbol"))?;
    let origin = api::origin_of(spec)?;
    let selected = api::select(st, q)?;
    let held = selected.context(st);
    // "Pad and align" as the run does, so the signals sit on the bars the backtest uses
    let pad = api::padded(st, spec, &selected, &held)?;
    let pad_ctx = pad.as_ref().map(|(rows, src, _)| api::padded_ctx(&held, rows, src));
    let ctx: &crate::connors::Ctx = pad_ctx.as_ref().unwrap_or(&*held);
    let (kind, instrument) = api::instrument(&selected, ticker);
    if kind == "unknown" { return Err(LabError::NeedsData(format!("{ticker}: instrument type is not established (index, ETF or catalogued stock); no lab result is published"))); }
    api::origin_admits(origin.as_ref(), kind, ticker)?;
    let (from, to, _) = api::window(ctx, q)?;
    api::bars_clean(ctx, from, to)?;
    let (others, foreign_json) = api::others_for(st, &ctx.r[0].ticker, &spec.foreign_locations(), api::close_fills(spec), false)?;
    let fr = Frames::new(Bars::from_ctx(ctx));
    let c = series::compile_with(spec, &fr, &others, &mut Cache::new())?;
    let sig = signals(&c, fr.d.len());
    let b = &fr.d;
    let head = format!("\"label\":\"{LABEL}\",\"apply\":\"current\",\"instrument\":{instrument}{foreign_json},\"from\":\"{}\",\"to\":\"{}\",\"notes\":{}", b.date[from], b.date[to], notes());
    if explore {
        let col = |v: &[f64]| format!("[{}]", v[from..=to].iter().map(|x| crate::num(*x)).collect::<Vec<_>>().join(","));
        let flags = |s: &[bool]| format!("[{}]", s[from..=to].iter().map(|x| if *x { "1" } else { "0" }).collect::<Vec<_>>().join(","));
        return Ok(format!("{{{head},\"kind\":\"lab_explore\",\"dates\":[{}],\"open\":{},\"high\":{},\"low\":{},\"close\":{},\"volume\":{},\"signals\":{{\"buy\":{},\"sell\":{},\"short\":{},\"cover\":{}}}}}",
            b.date[from..=to].iter().map(|d| format!("\"{d}\"")).collect::<Vec<_>>().join(","), col(&b.open), col(&b.high), col(&b.low), col(&b.close),
            b.volume.as_deref().map_or("null".into(), col), flags(&sig[0]), flags(&sig[1]), flags(&sig[2]), flags(&sig[3])));
    }
    let mut rows = Vec::new();
    push_rows(&mut rows, ticker, b, &sig, from..to + 1)?;
    Ok(format!("{{{head},\"kind\":\"lab_scan\",\"symbols\":1,\"rows\":[{}],\"refused\":[]}}", rows.join(",")))
}

/// Every admitted member of the portfolio universe, each on its own bars (AmiBroker "Apply to: all symbols").
fn all(st: &State, q: &Query, spec: &Spec) -> Result<String, LabError> {
    let p = spec.portfolio.as_ref().ok_or_else(|| inv("Apply to: all symbols needs a portfolio universe in the spec"))?;
    rules::validate_applied(spec)?;
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(SECONDS);
    let late = || if std::time::Instant::now() >= deadline { Err(inv(format!("the scan stopped after {SECONDS} s; use fewer members or a shorter range. No partial list is published."))) } else { Ok(()) };
    let loaded = super::universe::load_with_deadline(st, &p.universe, deadline)?;
    let ranges = loaded.members.iter().map(|m| dated(&m.frames.d.date, q)).collect::<Result<Vec<_>, _>>()?;
    // indicators warm up on each member's whole history, so the bound counts every loaded bar
    let cells: usize = loaded.members.iter().map(|m| m.frames.d.len()).sum();
    if cells > MAX_CELLS { return Err(inv(format!("{} members × their bars is {cells}; an all-symbols scan evaluates at most {MAX_CELLS} — choose a smaller universe", loaded.members.len()))); }
    let (others, foreign_json) = api::others_for(st, &loaded.reference, &spec.foreign_locations(), api::close_fills(spec), true)?;
    let (mut rows, mut refused, mut scanned) = (Vec::new(), loaded.refused.clone(), 0);
    for (m, range) in loaded.members.iter().zip(ranges) {
        late()?;
        if range.is_empty() { refused.push((m.ticker.clone(), "no bars inside the range".into())); continue; }
        match series::compile_with(spec, &m.frames, &others.with_run(&m.ticker), &mut Cache::new()) {
            Ok(c) => { push_rows(&mut rows, &m.ticker, &m.frames.d, &signals(&c, m.frames.d.len()), range)?; scanned += 1; }
            Err(LabError::NeedsData(why)) => refused.push((m.ticker.clone(), why)),
            Err(e) => return Err(e),
        }
    }
    late()?;
    let refused = refused.iter().map(|(t, r)| format!("{{\"ticker\":\"{}\",\"reason\":\"{}\"}}", esc(t), esc(r))).collect::<Vec<_>>().join(",");
    Ok(format!("{{\"label\":\"{LABEL}\",\"kind\":\"lab_scan\",\"apply\":\"all\",\"reference\":{{\"ticker\":\"{}\"}}{foreign_json},\"from\":{},\"to\":{},\"membership\":{{\"kind\":\"{}\",\"note\":\"{}\"}},\"notes\":{},\"symbols\":{scanned},\"rows\":[{}],\"refused\":[{refused}]}}",
        esc(&loaded.reference), q.get("from").filter(|v| !v.is_empty()).map_or("null".into(), |v| format!("\"{v}\"")), q.get("to").filter(|v| !v.is_empty()).map_or("null".into(), |v| format!("\"{v}\"")),
        loaded.membership.0, esc(loaded.membership.1), notes(), rows.join(",")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::json::{self, Json};
    use crate::{connors::Ctx, Row};

    fn state(n: usize) -> State {
        let rows: &'static [Row] = Box::leak((0..n).map(|i| {
            let c = 100. + (i as f64 * 0.3).sin() * 8.;
            Row { ticker: "SPX".into(), date: format!("{}-{:02}-{:02}", 2015 + i / 336, 1 + (i / 28) % 12, 1 + i % 28), open: c - 0.2, high: c + 1., low: c - 1., close: c, volume: Some(1000.) }
        }).collect::<Vec<_>>().into_boxed_slice());
        State { ctxs: BTreeMap::from([("SPX".into(), Ctx::new(rows, rows, &[]))]), baskets: Default::default(), vix: vec![], market_extra: Default::default() }
    }
    const SPEC: &str = r#"{"v":1,"entry":{"all":[{"lhs":{"s":"close"},"op":"crosses_above","rhs":{"s":"sma","p":{"length":5}}}]},"exit":{"any":[{"lhs":{"s":"close"},"op":"crosses_below","rhs":{"s":"sma","p":{"length":5}}},{"lhs":{"s":"bars_held"},"op":">=","rhs":{"value":3}}]}}"#;
    fn q(extra: &[(&str, &str)]) -> Query {
        let mut q: Query = [("ticker", "SPX"), ("spec", SPEC)].iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();
        for (k, v) in extra { q.insert(k.to_string(), v.to_string()); }
        q
    }
    fn arr(v: &Json, k: &str) -> Vec<Json> { match v.get(k) { Some(Json::Arr(a)) => a.clone(), x => panic!("{k}: {x:?}") } }
    fn flags(v: &Json, k: &str) -> Vec<bool> { arr(v.get("signals").unwrap(), k).iter().map(|x| x.as_f64() == Some(1.)).collect() }

    #[test] fn scan_lists_exactly_the_bars_where_a_raw_signal_holds() {
        let st = state(200);
        let v = json::parse(&scan_json(&st, &q(&[])).unwrap()).unwrap();
        assert_eq!((v.get("kind").and_then(Json::as_str), v.get("label").and_then(Json::as_str)), (Some("lab_scan"), Some(LABEL)));
        let rows = arr(&v, "rows");
        // independent oracle: crosses of close over its own 5-bar SMA
        let c: Vec<f64> = st.ctxs["SPX"].r.iter().map(|r| r.close).collect();
        let sma = |i: usize| if i < 4 { f64::NAN } else { c[i - 4..=i].iter().sum::<f64>() / 5. };
        let (mut buy, mut sell) = (vec![], vec![]);
        for i in 1..c.len() {
            if c[i] > sma(i) && c[i - 1] <= sma(i - 1) { buy.push(i); }
            if c[i] < sma(i) && c[i - 1] >= sma(i - 1) { sell.push(i); }
        }
        let got = |t: &str| rows.iter().filter(|r| matches!(r, Json::Arr(x) if x[2].as_str() == Some(t))).count();
        assert_eq!((got("Buy"), got("Sell"), got("Short"), got("Cover")), (buy.len(), sell.len(), 0, 0));
        assert!(!buy.is_empty() && rows.len() == buy.len() + sell.len(), "bars_held never fires without a position");
        let Json::Arr(first) = &rows[0] else { panic!() };
        let i0 = *buy.first().unwrap().min(sell.first().unwrap());
        assert_eq!((first[0].as_str(), first[1].as_str(), first[3].as_f64()), (Some("SPX"), Some(st.ctxs["SPX"].r[i0].date.as_str()), Some((c[i0] * 1e6).round() / 1e6)));
        assert_eq!(arr(&v, "notes").len(), NOTES.len());
    }
    #[test] fn explore_columns_match_the_bars_and_the_scan() {
        let st = state(200);
        let (from, to) = (st.ctxs["SPX"].r[50].date.clone(), st.ctxs["SPX"].r[120].date.clone());
        let v = json::parse(&scan_json(&st, &q(&[("mode", "explore"), ("from", &from), ("to", &to)])).unwrap()).unwrap();
        assert_eq!(v.get("kind").and_then(Json::as_str), Some("lab_explore"));
        let dates = arr(&v, "dates");
        assert_eq!((dates.len(), dates[0].as_str(), arr(&v, "volume")[0].as_f64()), (71, Some(from.as_str()), Some(1000.)));
        assert_eq!(arr(&v, "close")[3].as_f64(), Some((st.ctxs["SPX"].r[53].close * 1e6).round() / 1e6));
        let s = json::parse(&scan_json(&st, &q(&[("from", &from), ("to", &to)])).unwrap()).unwrap();
        let buys = flags(&v, "buy").iter().filter(|b| **b).count();
        assert_eq!(buys, arr(&s, "rows").iter().filter(|r| matches!(r, Json::Arr(x) if x[2].as_str() == Some("Buy"))).count());
        assert!(flags(&v, "short").iter().all(|b| !b));
    }
    #[test] fn short_side_signals_are_short_and_cover() {
        let st = state(200);
        let both = SPEC.replace(r#"}]}}"#, r#"}]},"short_side":{"entry":{"all":[{"lhs":{"s":"close"},"op":"crosses_below","rhs":{"s":"sma","p":{"length":5}}}]},"exit":{"any":[{"lhs":{"s":"close"},"op":"crosses_above","rhs":{"s":"sma","p":{"length":5}}}]}},"direction":"both"}"#);
        let v = json::parse(&scan_json(&st, &q(&[("spec", &both), ("mode", "explore")])).unwrap()).unwrap();
        assert_eq!(flags(&v, "short"), flags(&v, "sell"));
        assert_eq!(flags(&v, "cover"), flags(&v, "buy"));
        assert!(flags(&v, "short").iter().any(|b| *b));
    }
    #[test] fn refusals_are_typed() {
        let st = state(200);
        let msg = |extra: &[(&str, &str)]| scan_json(&st, &q(extra)).unwrap_err().message().to_string();
        assert!(msg(&[("mode", "list")]).contains("mode must be"));
        assert!(msg(&[("apply", "some")]).contains("apply must be"));
        assert!(msg(&[("apply", "all")]).contains("needs a portfolio universe"));
        assert!(msg(&[("apply", "all"), ("mode", "explore")]).contains("current symbol"));
        assert!(msg(&[("ticker", "")]).contains("ticker is required"));
        assert!(msg(&[("to", "2099-01-01")]).contains("exceeds admitted data"), "the range is not silently shortened");
        assert!(msg(&[("from", "bad")]).contains("YYYY-MM-DD"));
        assert_eq!(scan_json(&st, &q(&[("ticker", "NOPE")])).unwrap_err().status(), "invalid");
        // a rule that is true on every bar overflows the row cap rather than publishing a cut list
        let always = SPEC.replace(r#""op":"crosses_above""#, r#""op":">""#).replace(r#"{"s":"sma","p":{"length":5}}}]},"exit""#, r#"{"value":0}}]},"exit""#);
        let big = state(MAX_ROWS + 50);
        assert!(scan_json(&big, &q(&[("spec", &always)])).unwrap_err().message().contains("more than 20000 signals"));
    }
    #[test] fn all_symbols_scans_each_universe_member() {
        let (_d, st) = super::super::universe::tests::india("scan-all");
        let spec = r#"{"v":1,"entry":{"all":[{"lhs":{"s":"close"},"op":">","rhs":{"s":"sma","p":{"length":5}}}]},"exit":{"any":[{"lhs":{"s":"bars_held"},"op":">=","rhs":{"value":3}}]},"execution":{"capital":100000},"portfolio":{"universe":{"basket":"nifty50"},"score":{"s":"roc","p":{"length":3}},"max_open":2}}"#;
        let v = json::parse(&scan_json(&st, &[("spec".to_string(), spec.to_string()), ("apply".into(), "all".into())].into()).unwrap()).unwrap();
        assert_eq!((v.get("apply").and_then(Json::as_str), v.get("symbols").and_then(Json::as_f64)), (Some("all"), Some(2.)));
        let rows = arr(&v, "rows");
        let syms: std::collections::BTreeSet<_> = rows.iter().map(|r| match r { Json::Arr(x) => x[0].as_str().unwrap().to_string(), _ => panic!() }).collect();
        assert_eq!(syms.len(), 2, "{syms:?}");
        // closes rise by 1 a bar: close > SMA(5) from the fifth bar on every member
        assert!(rows.iter().all(|r| matches!(r, Json::Arr(x) if x[2].as_str() == Some("Buy"))));
        assert!(v.get("membership").is_some());
        let ranged = json::parse(&scan_json(&st, &[("spec".to_string(), spec.to_string()), ("apply".into(), "all".into()), ("from".into(), "2024-02-01".into()), ("to".into(), "2024-02-05".into())].into()).unwrap()).unwrap();
        assert!(arr(&ranged, "rows").iter().all(|r| matches!(r, Json::Arr(x) if x[1].as_str().is_some_and(|d| ("2024-02-01"..="2024-02-05").contains(&d)))));
    }
}
