//! ATH sub-variants that run on the Rust engine (`ath_registry`): the stock-universe info (`GET guru/yt/ath-universe`) and one
//! portfolio run (`GET guru/yt/ath-run?id=…&p.<key>=…&from=&to=&slippage=`). The answer has the shape of the BTST universe
//! run (`kind: "universe"`) so the same result page shows it.
//!
//! Data: converted copies of the team's AmiBroker databases (`scripts/ath_convert.py`), read only:
//! * cash: `C&P DATABASE UPDATED TILL 06-09-2021_WITH T.O` in `/data/ath-cp-2021-09-06` (override `VAULT_ATH_DATA_DIR`);
//! * F&O: `NSE FO + ATH UPDATED TILL_26-10-2021` in `/data/ath-foath-2021-10-26` (override `VAULT_ATH_FO_DATA_DIR`).
//! The universe of a variant is a watchlist of its database; the `Foreign()` index series come from the cash database.
use crate::ath_core::{alpha_rank, backtest_full, common_schema, load_many, load_series, read_watchlist, Backtest, Data, Env, PortfolioParams, Series, Signals, VariantDef, P};
use crate::connors::ParamDef;
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::Instant;

pub const UNIVERSE: &str = "NSE 500 C&P";
/// The buy & hold index of a basket (the watchlist a run trades): the NSE 500 index for an NSE 500 watchlist, the Nifty 50 for a
/// Nifty 50 one; none for the others (the F&O lists have no index file in the database).
fn benchmark_of(watchlist: &str) -> Option<(&'static str, &'static str)> {
    let w = watchlist.to_ascii_uppercase();
    if w.starts_with("NSE 500") || w.starts_with("NSE500") {
        Some(("NSE500", "NSE 500"))
    } else if w.starts_with("NIFTY 50") || w.starts_with("NIFTY50") || w.starts_with("NSENIFTY") {
        Some(("NSENIFTY", "NIFTY 50"))
    } else {
        None
    }
}
/// An index series of the cash database; NSENIFTY falls back to NSENIFTY1 (same index, longer history; the 31-03-2022 database has only NSENIFTY1).
fn load_index(dir: &str, name: &str) -> Option<Series> {
    let p = std::path::Path::new(dir);
    let got = load_series(p, name).ok().filter(|s| !s.bars.is_empty());
    if got.is_none() && name == "NSENIFTY" {
        return load_series(p, "NSENIFTY1").ok().filter(|s| !s.bars.is_empty());
    }
    got
}
const CASH_DATABASE: &str = "C&P DATABASE UPDATED TILL 06-09-2021_WITH T.O (team snapshot, converted copy)";
const CASH22_DATABASE: &str = "C&P DATABASE UPDATED TILL 31-03-2022 (team database, converted copy)";
const FO22_DATABASE: &str = "NSE FO + ATH UPDATED TILL_31-03-2022 (team database, converted copy)";
const FO_DATABASE: &str = "NSE FO + ATH UPDATED TILL_26-10-2021 (team snapshot, converted copy)";
const DEFAULT_FROM: i32 = 20000101; // the team's runs start 1/1/2000
const TIME_LIMIT_SECS: f64 = 100.0;
// Every ATH variant books its trades with AmiBroker's own arithmetic (18 Nov 2026, user: "apply this same calculation to the remaining ATH
// variants"): profit and cash as AmiBroker (`amibroker_profit`), the full set (`amibroker_full`: float32 position value, unrounded-equity
// sizing, float32 share count, closing commission on the stored close, commission shrink), sizing on the stored float32 price
// (`size_f32_price`), prices to the decimals of their float32 step (`price_digits` 0, `ath_core::ab_price_digits`) and the day summary
// printed half up (`detail_half_up`). Proven on the team's sheets: the four Verification workbooks (zero and 0.65 % brokerage, 20 % x 5 and
// 2 % x 50) equal on every day; Fund Issue 2 % 4,008/4,008 trades and shares; hard-SL Tradelist 1,384/1,384 shares, no cash drift;
// running_pctswing 6-decimal sheets 1,928 / 1,492; Shrey 10 % 103/103; replay of the 215 team sheets of ATH/verify and ATH/v2:
// equal shares 346,760 -> 363,804, equal profit 330,707 -> 357,904, same trades 625,927 -> 626,223 (before 17-18 Nov these were per-variant lists).
/// Variants whose AFL leaves `UsePrevBarEquityForPosSizing` at AmiBroker's default (False): each entry is sized from the equity at that
/// moment, which falls with every entry of the bar (`PortfolioParams::current_equity_sizing`). Supertrend long only: the team's
/// 2020-2021 sheet buys MARICO 143 / GOKEX 654 on 01-01-2020 where the previous bar's equity gives 144 / 655.
const CURRENT_EQUITY_SIZING: [&str; 1] = ["ytath_supertrend_long_only_buy_st_5_2_up"];
/// A run holds every bar of its universe in memory (about 120 MB for the 1,301 symbols of NSE 500 C&P) next to the engine's own arrays; the container has 1 GB.
const MAX_SYMBOLS: usize = 1800;
/// Index series the AFLs read with `Foreign()`.
const INDEX_NAMES: [&str; 8] = ["NSENIFTY", "NSENIFTY1", "NSE500", "VIX", "NSE100", "NSEMIDCAP", "NSESMLCAP100", "BANKNIFTY"];

fn cash_dir() -> String {
    std::env::var("VAULT_ATH_DATA_DIR").unwrap_or_else(|_| "/data/ath-cp-2021-09-06".into())
}
fn fo_dir() -> String {
    std::env::var("VAULT_ATH_FO_DATA_DIR").unwrap_or_else(|_| "/data/ath-foath-2021-10-26".into())
}
fn cash22_dir() -> String {
    std::env::var("VAULT_ATH_DATA22_DIR").unwrap_or_else(|_| "/data/ath-cp-2022-03-31".into())
}
fn fo22_dir() -> String {
    std::env::var("VAULT_ATH_FO22_DATA_DIR").unwrap_or_else(|_| "/data/ath-foath-2022-03-31".into())
}
fn dir_of(d: Data) -> String {
    match d {
        Data::Cash { .. } => cash_dir(),
        Data::Fo { .. } => fo_dir(),
        Data::Cash22 { .. } => cash22_dir(),
        Data::Fo22 { .. } => fo22_dir(),
    }
}
/// The cash database whose index series (`Foreign()`, buy & hold) go with this data spec.
fn index_dir(d: Data) -> String {
    match d {
        Data::Cash { .. } | Data::Fo { .. } => cash_dir(),
        Data::Cash22 { .. } | Data::Fo22 { .. } => cash22_dir(),
    }
}
fn watchlist_of(d: Data) -> &'static str {
    match d {
        Data::Cash { watchlist } | Data::Fo { watchlist } | Data::Cash22 { watchlist } | Data::Fo22 { watchlist } => watchlist,
    }
}

/// (universe name, database label, badge, unit) of a data spec.
fn words(d: Data) -> (String, &'static str, &'static str, &'static str) {
    let name = watchlist_of(d).trim_end_matches(".tls").to_string();
    match d {
        Data::Cash { .. } => (name, CASH_DATABASE, "NSE 500 stocks", "NSE 500 stocks, cash prices"),
        Data::Fo { .. } => (name, FO_DATABASE, "F&O stocks (futures)", "F&O stocks, near-month futures prices"),
        Data::Cash22 { .. } => (name, CASH22_DATABASE, "NSE 500 stocks (to 2022-03)", "NSE 500 stocks, cash prices, database of 31-03-2022"),
        Data::Fo22 { .. } => (name, FO22_DATABASE, "F&O stocks (futures, to 2022-03)", "F&O stocks, futures prices, database of 31-03-2022"),
    }
}

static RUNNING: AtomicBool = AtomicBool::new(false);
struct Permit;
impl Permit {
    fn take() -> Result<Self, String> {
        RUNNING.compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire).map(|_| Self).map_err(|_| "busy: An ATH universe backtest is already running; retry when it finishes.".into())
    }
}
impl Drop for Permit {
    fn drop(&mut self) {
        RUNNING.store(false, Ordering::Release);
    }
}

/// The ids the engine runs.
pub fn ids() -> Vec<&'static str> {
    crate::ath_registry::all().iter().map(|v| v.id).collect()
}

/// Common portfolio parameters + the variant's own, as the connors `ParamDef`s the page uses.
pub fn schema_of(v: &VariantDef) -> Vec<ParamDef> {
    let all: Vec<P> = common_schema().into_iter().chain((v.schema)()).collect();
    // AmiBroker clamps a Param() default that lies outside the Param's own range to the nearest bound (some AFLs list 10 in a 11..18 range)
    // The ranges of exit %, turnover period and ROC period in the AFLs are only their Optimize() ranges: the team's sheets use other values, so the page accepts wider ones.
    all.into_iter()
        .map(|q| {
            let def = q.def.clamp(q.min, q.max);
            let (min, max) = match q.key {
                "exitpct" => (q.min.min(1.0), q.max.max(50.0)),
                "turnoverpd" => (q.min.min(1.0), q.max.max(250.0)),
                "rocpd" => (q.min.min(1.0), q.max.max(500.0)),
                "pt_pct" => (q.min.min(1.0), q.max.max(1000.0)), // a profit target of 1000 % = no target
                _ => (q.min, q.max),
            };
            ParamDef { key: q.key, label: q.label, def, min, max, step: q.step }
        })
        .collect()
}

fn pjson(d: &ParamDef) -> String {
    format!("{{\"key\":\"{}\",\"label\":\"{}\",\"def\":{},\"min\":{},\"max\":{},\"step\":{}}}", d.key, crate::esc(d.label), crate::num(d.def), crate::num(d.min), crate::num(d.max), crate::num(d.step))
}

fn iso(date: i32) -> String {
    format!("{:04}-{:02}-{:02}", date / 10000, date / 100 % 100, date % 100)
}

fn date_num(s: &str) -> Option<i32> {
    let b = s.as_bytes();
    if b.len() != 10 || b[4] != b'-' || b[7] != b'-' {
        return None;
    }
    let (y, m, d): (i32, i32, i32) = (s[0..4].parse().ok()?, s[5..7].parse().ok()?, s[8..10].parse().ok()?);
    ((1..=12).contains(&m) && (1..=31).contains(&d) && y >= 1900).then_some(y * 10000 + m * 100 + d)
}

fn watchlist(d: Data) -> Result<Vec<String>, String> {
    let dir = dir_of(d);
    read_watchlist(std::path::Path::new(&dir), watchlist_of(d)).map_err(|_| format!("needs_data: the ATH data is not loaded ({dir}/watchlists/{})", watchlist_of(d)))
}

/// (symbols of the universe present in the converted data, first date, last date) from the converter's `universe.json` (one key per line).
fn coverage(d: Data, names: &[String]) -> Result<(usize, i32, i32), String> {
    let dir = dir_of(d);
    let t = std::fs::read_to_string(format!("{dir}/universe.json")).map_err(|_| format!("needs_data: the ATH data is not loaded ({dir}/universe.json)"))?;
    let want: std::collections::HashSet<&str> = names.iter().map(|s| s.as_str()).collect();
    let (mut name, mut first, mut n, mut lo, mut hi) = (String::new(), 0i32, 0usize, i32::MAX, 0i32);
    let num = |l: &str| l.split(':').nth(1).and_then(|x| x.trim().trim_end_matches(',').parse::<i32>().ok());
    for l in t.lines() {
        let l = l.trim();
        if let Some(r) = l.strip_prefix("\"name\": \"") {
            name = r.trim_end_matches(',').trim_end_matches('"').replace("\\\"", "\"").replace("\\\\", "\\");
        } else if l.starts_with("\"first\":") {
            first = num(l).unwrap_or(0);
        } else if l.starts_with("\"last\":") && want.contains(name.as_str()) {
            n += 1;
            lo = lo.min(first);
            hi = hi.max(num(l).unwrap_or(0));
        }
    }
    if n == 0 {
        return Err("needs_data: no watchlist symbol is in the converted data".into());
    }
    // the team's runs start 1/1/2000; the earlier bars only feed the all-time high
    Ok((n, lo.max(DEFAULT_FROM), hi))
}

/// The page's baskets: (key, label, CURRENT watchlist candidates, C&P watchlist candidates); the first file that exists in the database is used.
const BASKETS: &[(&str, &str, &[&str], &[&str])] = &[
    ("nse500", "NSE 500", &["NSE 500 CURRENT.tls"], &["NSE 500 C&P.tls"]),
    ("allfo", "All F&O", &["FO CASH CURRENT.tls", "CURRENT F&O STOCK.tls"], &["FO SCRIPTS.tls", "FO SCRIPTS C&P.tls", "FO STOCKS C&P.tls"]),
];

/// The baskets one database offers, as a JSON array: current / C&P watchlist files and the counts of Current, Past (C&P minus current) and C&P.
fn baskets_json(d: Data) -> String {
    let dir = dir_of(d);
    let path = std::path::Path::new(&dir);
    let first = |files: &[&str]| files.iter().find_map(|f| read_watchlist(path, f).ok().map(|n| (f.to_string(), n)));
    let mut out = Vec::new();
    for (key, label, cur, cp) in BASKETS {
        let Some((cp_file, cp_names)) = first(cp) else { continue };
        let (cur_file, cur_names) = first(cur).unwrap_or_default();
        let past = cp_names.iter().filter(|n| !cur_names.contains(n)).count();
        out.push(format!(
            "{{\"key\":\"{key}\",\"label\":\"{}\",\"cp\":\"{}\",\"current\":\"{}\",\"n_cp\":{},\"n_current\":{},\"n_past\":{past}}}",
            crate::esc(label), crate::esc(&cp_file), crate::esc(&cur_file), cp_names.len(), cur_names.len()
        ));
    }
    format!("[{}]", out.join(","))
}

/// The universe wording of one data spec, as JSON fields of the info answer.
fn universe_json(d: Data) -> Result<String, String> {
    let names = watchlist(d)?;
    let (n, first, last) = coverage(d, &names)?;
    let (name, database, badge, unit) = words(d);
    let n_name = crate::esc(&name);
    Ok(format!(
        "\"universe\":\"{n_name}\",\"symbols\":{n},\"first\":\"{}\",\"last\":\"{}\",\"choice_label\":\"{} · {n_name} ({n})\",\"traded\":\"Traded: {} on the watchlist {n_name} ({n} symbols), database {}\",\"database\":\"{}\",\"badge\":\"{badge}\",\"stamp\":\"Runs on {badge}\",\"run_label\":\"Run on {badge}\",\"on_label\":\"On {badge}\",\"unit\":\"{unit}\",\"calendar_note\":\"the stocks' own bars supply the calendar; the buy & hold comparison is the index of the chosen basket (the NSE 500 index for NSE 500; none for F&O).\",\"card_note\":\"A portfolio of the team's rule on every stock of the {n_name} watchlist ({n} stocks, {}–{}). Not run on an index; open it to run the stock-universe backtest.\"",
        iso(first), iso(last), crate::esc(badge), crate::esc(unit), crate::esc(database), crate::esc(database), first / 10000, last / 10000
    ))
}

/// `GET guru/yt/ath-universe`: the universes and the sub-variants an ATH run accepts. Top-level fields describe the first variant's
/// universe; `by_id` holds each variant's own wording (the F&O variants run on another database).
pub fn info_json() -> Result<String, String> {
    static CACHE: Mutex<Option<String>> = Mutex::new(None);
    if let Some(s) = CACHE.lock().unwrap().clone() {
        return Ok(s);
    }
    let vs = crate::ath_registry::all();
    let first = vs.first().ok_or("needs_data: no ATH engine is registered")?;
    let ids: Vec<String> = vs.iter().map(|v| format!("\"{}\"", v.id)).collect();
    let mut schemas: Vec<String> = Vec::new();
    let mut by_id: Vec<String> = Vec::new();
    let mut uni_cache: BTreeMap<String, String> = BTreeMap::new();
    // `members`: each universe's watchlist symbols once (the page's Stock list; a run takes `only=` one of them), `members_key` per id
    let mut member_keys: Vec<String> = Vec::new();
    let mut members: Vec<String> = Vec::new();
    let mut baskets: Vec<String> = Vec::new();
    for v in &vs {
        schemas.push(format!("\"{}\":[{}]", v.id, schema_of(v).iter().map(pjson).collect::<Vec<_>>().join(",")));
        let key = format!("{:?}", v.data);
        if !uni_cache.contains_key(&key) {
            uni_cache.insert(key.clone(), universe_json(v.data)?);
        }
        let mk = match member_keys.iter().position(|k| *k == key) {
            Some(i) => i,
            None => {
                let names = watchlist(v.data)?;
                members.push(format!("\"u{}\":[{}]", member_keys.len(), names.iter().map(|n| format!("\"{}\"", crate::esc(n))).collect::<Vec<_>>().join(",")));
                baskets.push(format!("\"u{}\":{}", member_keys.len(), baskets_json(v.data)));
                member_keys.push(key.clone());
                member_keys.len() - 1
            }
        };
        by_id.push(format!("\"{}\":{{{},\"members_key\":\"u{mk}\"}}", v.id, uni_cache[&key]));
    }
    let top = uni_cache[&format!("{:?}", first.data)].clone();
    let out = format!("{{{top},\"ids\":[{}],\"params\":[{}],\"schemas\":{{{}}},\"by_id\":{{{}}},\"members\":{{{}}},\"baskets\":{{{}}}}}", ids.join(","), schema_of(first).iter().map(pjson).collect::<Vec<_>>().join(","), schemas.join(","), by_id.join(","), members.join(","), baskets.join(","));
    *CACHE.lock().unwrap() = Some(out.clone());
    Ok(out)
}

/// The `Foreign()` index series of the cash database.
fn load_env(d: Data) -> Env {
    let dir = index_dir(d);
    let mut env = Env::empty();
    for n in INDEX_NAMES {
        if let Ok(s) = load_series(std::path::Path::new(&dir), n) {
            if !s.bars.is_empty() {
                env.index.insert(n.to_string(), s);
            }
        }
    }
    // the team pads NSENIFTY with NSENIFTY1 (same index, longer history): a database without an NSENIFTY file (the 31-03-2022 cash database) reads NSENIFTY1
    if !env.index.contains_key("NSENIFTY") {
        if let Some(s) = env.index.get("NSENIFTY1") {
            let padded = Series { name: "NSENIFTY".to_string(), bars: s.bars.clone() };
            env.index.insert("NSENIFTY".to_string(), padded);
        }
    }
    env
}

/// The variant of a run, on the database `data=cash|fo|cash22|fo22` asks for (the sheets of one AFL may come from different databases).
fn variant_of(q: &BTreeMap<String, String>) -> Result<VariantDef, String> {
    let id = q.get("id").ok_or("id missing")?.as_str();
    let Some(mut variant) = crate::ath_registry::find(id) else {
        return Err(format!("not_testable: {id} has no engine yet; the ATH engine runs {} sub-variants", ids().len()));
    };
    if let Some(d) = q.get("data").filter(|d| !d.is_empty()) {
        variant.data = match d.as_str() {
            "cash" => Data::Cash { watchlist: "NSE 500 C&P.tls" },
            "cash22" => Data::Cash22 { watchlist: "NSE 500 C&P.tls" },
            "fo" => Data::Fo { watchlist: "FO STOCKS C&P.tls" },
            "fo22" => Data::Fo22 { watchlist: "FO STOCKS C&P.tls" },
            other => return Err(format!("data must be cash, cash22, fo or fo22, not {other}")),
        };
    }
    Ok(variant)
}

/// The settings of one run: the page's `p.<key>` values over the AFL defaults; the AFL's own BROKERAGE parameter follows the page's
/// brokerage as entered (0 means 0, kept inside the AFL's range as AmiBroker's Param() does); only a run that gives no brokerage at all
/// (no `cost` / `slippage`) uses the AFL default.
struct Knobs {
    raw: crate::connors::Params,
    defaults: BTreeMap<&'static str, f64>,
    comm_range: BTreeMap<&'static str, (f64, f64, f64)>,
    comm_for_afl: Option<f64>,
    max_positions: f64,
    pct: f64,
    equity0: f64,
    alloc: Option<f64>,
}
impl Knobs {
    fn new(q: &BTreeMap<String, String>, schema: &[ParamDef], costs: crate::accounting::Costs) -> Result<Self, String> {
        let raw: crate::connors::Params = crate::params_of(q);
        let mut k = Knobs {
            raw,
            defaults: schema.iter().map(|d| (d.key, d.def)).collect(),
            comm_range: schema.iter().filter(|d| d.key == "comm" || d.key == "brokerage").map(|d| (d.key, (d.min, d.max, d.def))).collect(),
            // the page's brokerage, for AFLs whose own logic reads BROKERAGE (key `comm`)
            comm_for_afl: (q.contains_key("cost") || q.contains_key("slippage")).then(|| costs.side_rate() * 100.0),
            max_positions: 0.0,
            pct: 0.0,
            equity0: 0.0,
            alloc: None,
        };
        for d in schema {
            value_ok(d, k.get(d.key)).map_err(|e| format!("{} {e}", d.key))?;
        }
        k.max_positions = k.get("max_positions").round();
        k.pct = match k.get("pct") {
            x if x <= 0.0 => 100.0 / k.max_positions,
            x => x,
        };
        k.equity0 = match k.raw.get("equity") {
            Some(&v) => v,
            None => q.get("equity").map(|s| s.parse::<f64>().map_err(|_| "equity must be a number".to_string())).transpose()?.unwrap_or(1_000_000.0),
        };
        k.alloc = (k.get("sizing").round() == 2.0).then(|| k.get("alloc"));
        Ok(k)
    }
    fn get(&self, k: &str) -> f64 {
        match self.comm_range.get(k) {
            Some(&(lo, hi, def)) if k == "comm" || k == "brokerage" => self.comm_for_afl.map_or(def, |c| c.clamp(lo, hi)),
            _ if k == "comm" || k == "brokerage" => self.comm_for_afl.unwrap_or(0.0),
            _ => self.raw.get(k).copied().unwrap_or_else(|| self.defaults.get(k).copied().unwrap_or(0.0)),
        }
    }
}

/// What a run reads before its settings matter: the stocks of the watchlist, the period, the `Foreign()` index series and the symbol
/// order. An optimization reads it once for all its grid points.
struct Setup {
    series: Vec<Series>,
    n_names: usize,
    from: i32,
    to: i32,
    env: Env,
    rank: Vec<usize>,
    basket: String,
}
fn setup(q: &BTreeMap<String, String>, variant: &VariantDef) -> Result<Setup, String> {
    let dir = dir_of(variant.data);
    let mut names = match q.get("watchlist").filter(|w| !w.is_empty()) {
        // another watchlist of the same database (a plain file name, no path)
        Some(w) if !w.contains('/') && !w.contains('\\') && w.ends_with(".tls") => {
            read_watchlist(std::path::Path::new(&dir), w).map_err(|_| format!("needs_data: watchlist {w} is not in {dir}/watchlists"))?
        }
        Some(w) => return Err(format!("watchlist must be the name of a .tls file of the database, not {w}")),
        None => watchlist(variant.data)?,
    };
    // `exclude_watchlist=`: leave out the symbols of another watchlist of the same database (the page's basket "Past" = C&P minus CURRENT)
    if let Some(x) = q.get("exclude_watchlist").filter(|w| !w.is_empty()) {
        if x.contains('/') || x.contains('\\') || !x.ends_with(".tls") {
            return Err(format!("exclude_watchlist must be the name of a .tls file of the database, not {x}"));
        }
        let out = read_watchlist(std::path::Path::new(&dir), x).map_err(|_| format!("needs_data: watchlist {x} is not in {dir}/watchlists"))?;
        names.retain(|n| !out.contains(n));
    }
    // `only=A;B`: run on these symbols alone (a single-stock sample, or one symbol at a time for the team's individual-symbol sheets)
    if let Some(o) = q.get("only").filter(|s| !s.is_empty()) {
        names = o.split(';').map(|s| s.to_string()).collect();
    }
    if names.len() > MAX_SYMBOLS {
        return Err(format!("unavailable: {} symbols are more than the engine container's memory holds for one run (limit {MAX_SYMBOLS}); use a smaller watchlist", names.len()));
    }
    let series: Vec<Series> = load_many(std::path::Path::new(&dir), &names);
    if series.is_empty() {
        return Err("needs_data: no watchlist symbol is in the converted data".into());
    }
    let last_data = series.iter().map(|s| s.bars.last().unwrap().date).max().unwrap();
    let from = match q.get("from").filter(|s| !s.is_empty()) {
        Some(s) => date_num(s).ok_or("from must be YYYY-MM-DD")?,
        None => DEFAULT_FROM,
    };
    let to = match q.get("to").filter(|s| !s.is_empty()) {
        Some(s) => date_num(s).ok_or("to must be YYYY-MM-DD")?,
        None => last_data,
    };
    if from > to {
        return Err("from must not be after to".into());
    }
    let env = load_env(variant.data);
    let rank = alpha_rank(&series);
    // buy & hold follows the basket of this run: the page's watchlist, else the variant's own
    let basket = q.get("watchlist").filter(|w| !w.is_empty()).map(|w| w.as_str()).unwrap_or(watchlist_of(variant.data)).to_string();
    Ok(Setup { series, n_names: names.len(), from, to, env, rank, basket })
}

/// The buy / sell signals of every stock for one set of settings.
fn signals_of(variant: &VariantDef, s: &Setup, g: crate::ath_core::G) -> Result<Vec<Signals>, String> {
    let sgs: Vec<Signals> = (variant.signals)(&s.series, &s.env, g);
    if sgs.len() != s.series.len() {
        return Err("unavailable: the engine returned the wrong number of signal sets".into());
    }
    Ok(sgs)
}

fn portfolio_of(variant: &VariantDef, s: &Setup, k: &Knobs, costs: crate::accounting::Costs) -> PortfolioParams {
    PortfolioParams { initial_equity: k.equity0, pct_of_equity: k.pct, fixed_amount: k.alloc, max_positions: k.max_positions as usize, commission_pct: costs.side_rate() * 100.0, start: s.from, end: s.to, amibroker_profit: true, amibroker_full: true, current_equity_sizing: CURRENT_EQUITY_SIZING.contains(&variant.id), size_f32_price: true, detail_half_up: true, price_digits: 0 }
}

/// `GET guru/yt/ath-run`: one portfolio backtest of an ATH sub-variant on the watchlist of its database.
pub fn run_json(q: &BTreeMap<String, String>) -> Result<String, String> {
    let variant = variant_of(q)?;
    let costs = crate::accounting::Costs::parse(q)?;
    let k = Knobs::new(q, &schema_of(&variant), costs)?;

    let _permit = Permit::take()?;
    let started = Instant::now();
    let s = setup(q, &variant)?;
    let sgs = signals_of(&variant, &s, &|key| k.get(key))?;
    let syms: Vec<(&Series, &Signals)> = s.series.iter().zip(sgs.iter()).collect();
    let pp = portfolio_of(&variant, &s, &k, costs);
    let want_detail_log = q.get("detail_log").is_some_and(|v| v == "1");
    let net = crate::ath_core::backtest_full_with_detail_log(&syms, &s.rank, &pp, want_detail_log);
    if started.elapsed().as_secs_f64() > TIME_LIMIT_SECS {
        return Err("unavailable: the ATH run reached its 100-second budget; no partial result is published".into());
    }
    let gross = backtest_full(&syms, &s.rank, &PortfolioParams { commission_pct: 0.0, ..pp });
    if net.dates.is_empty() {
        return Err("needs_data: no bar in the chosen period".into());
    }
    // `bars=1`: each trade also carries its own daily bars, entry to exit (the team's Excel tradesheet reads them for MAE)
    let trade_bars = q.get("bars").is_some_and(|v| v == "1");
    Ok(result_json(&variant, &s.series, &net, &gross, &pp, costs, s.n_names, started.elapsed().as_secs_f64(), &s.basket, trade_bars))
}

const OPT_MAX_COMBOS: usize = 60;
const OPT_MAX_VALUES: usize = 20;
const OPT_TIME_LIMIT_SECS: f64 = 110.0;

/// A pick-list setting (coded choices such as "1 NONE; 2 ROC", or an on/off toggle) stays inside its own range and grid.
fn is_pick(d: &ParamDef) -> bool {
    let coded = d.label.trim_end().strip_suffix(')').and_then(|l| l.rsplit_once('(')).is_some_and(|(_, inner)| inner.contains(';') || inner.contains(", 1") || inner.contains(", 2") || inner.contains(", 3"));
    // a count whose AFL range happens to be two values (CONSICUTIVE LOSS 1..2) is still a free number
    coded || (d.key != "noofloss" && d.step > 0.0 && ((d.max - d.min) / d.step).round() + 1.0 <= 3.0)
}

/// The value the page typed for a setting. A pick-list must be inside its range; any other number runs as typed (the AFL's range is only
/// a hint): whole numbers when the AFL steps in whole numbers, never below zero when the AFL's minimum is zero or more, up to a safety cap.
fn value_ok(d: &ParamDef, v: f64) -> Result<(), String> {
    if !v.is_finite() {
        return Err("must be a number".into());
    }
    if is_pick(d) {
        return if v < d.min || v > d.max { Err(format!("must be {} to {}", crate::num(d.min), crate::num(d.max))) } else { Ok(()) };
    }
    let whole = d.step >= 1.0 && d.step.fract() == 0.0 && d.min.fract() == 0.0;
    let lo = if d.min < 0.0 { -1.0e12 } else if whole && d.min >= 1.0 { 1.0 } else { 0.0 };
    let hi = (d.max.abs() * 100.0).max(100_000.0);
    if v < lo || v > hi {
        return Err(format!("must be {} to {}", crate::num(lo), crate::num(hi)));
    }
    if whole && v.fract() != 0.0 {
        return Err("must be a whole number".into());
    }
    Ok(())
}

/// `grid.<key>=v1,v2,...` of an optimization query, checked against the variant's schema: known key, 1..=20 distinct values inside
/// the key's range, at most 60 combinations over all keys. Keys come back sorted (BTreeMap order of the query).
fn parse_grids(q: &BTreeMap<String, String>, schema: &[ParamDef]) -> Result<Vec<(String, Vec<f64>)>, String> {
    let mut dims: Vec<(String, Vec<f64>)> = Vec::new();
    for (k, spec) in q {
        let Some(key) = k.strip_prefix("grid.") else { continue };
        let d = schema.iter().find(|d| d.key == key).ok_or_else(|| format!("grid.{key} is not a setting of {}", q.get("id").map_or("this variant", |s| s.as_str())))?;
        let mut vals: Vec<f64> = Vec::new();
        for part in spec.split(',').map(str::trim).filter(|s| !s.is_empty()) {
            let v: f64 = part.parse().map_err(|_| format!("grid.{key}: '{part}' is not a number"))?;
            if let Err(e) = value_ok(d, v) {
                return Err(format!("grid.{key}: {part} {e}"));
            }
            if !vals.iter().any(|x| (x - v).abs() < 1e-9) {
                vals.push(v);
            }
        }
        if vals.is_empty() {
            return Err(format!("grid.{key} has no value"));
        }
        if vals.len() > OPT_MAX_VALUES {
            return Err(format!("grid.{key} has {} values; the limit is {OPT_MAX_VALUES}. Use a bigger step.", vals.len()));
        }
        dims.push((key.to_string(), vals));
    }
    if dims.is_empty() {
        return Err("tick at least one setting to vary (grid.<key>=v1,v2,...)".into());
    }
    let combos = dims.iter().fold(1usize, |a, d| a.saturating_mul(d.1.len()));
    if combos > OPT_MAX_COMBOS {
        return Err(format!("{combos} combinations; the limit is {OPT_MAX_COMBOS}. Tick fewer settings or use a bigger step."));
    }
    Ok(dims)
}

/// The flat `"stats":{...}` object of a run answer as (key, value); null -> NaN.
fn stats_of(json: &str) -> Result<Vec<(String, f64)>, String> {
    let at = json.find("\"stats\":{").ok_or("unavailable: a run answered without statistics")? + 9;
    let body = &json[at..at + json[at..].find('}').ok_or("unavailable: a run answered with broken statistics")?];
    Ok(body.split(',').filter_map(|kv| kv.split_once(':')).map(|(k, v)| (k.trim_matches('"').to_string(), v.trim().parse().unwrap_or(f64::NAN))).collect())
}


// ---- the 36 AmiBroker result-list columns of one run (amsheet.rs), read back from the run's own JSON so the grid page can write the lab's optimization sheet
fn jstr<'a>(o: &'a str, key: &str) -> Option<&'a str> {
    let k = format!("\"{key}\":\"");
    let at = o.find(&k)? + k.len();
    Some(&o[at..at + o[at..].find('"')?])
}
fn jnum(o: &str, key: &str) -> Option<f64> {
    let k = format!("\"{key}\":");
    let at = o.find(&k)? + k.len();
    let rest = &o[at..];
    rest[..rest.find(|c| c == ',' || c == '}' || c == ']').unwrap_or(rest.len())].trim().parse().ok()
}
/// the text between `"key":[` and its closing `]`, brackets inside strings ignored
fn jarray<'a>(o: &'a str, key: &str) -> Option<&'a str> {
    let k = format!("\"{key}\":[");
    let at = o.find(&k)? + k.len();
    let (mut depth, mut in_str) = (1, false);
    for (i, c) in o[at..].char_indices() {
        match c {
            '"' => in_str = !in_str,
            '[' if !in_str => depth += 1,
            ']' if !in_str => { depth -= 1; if depth == 0 { return Some(&o[at..at + i]); } }
            _ => {}
        }
    }
    None
}
fn sheet_row_of(json: &str) -> Vec<f64> {
    let (Some(dates), Some(eq), Some(trades)) = (jarray(json, "dates"), jarray(json, "equity"), jarray(json, "trades")) else { return vec![f64::NAN; crate::amsheet::COLUMNS] };
    let dates: Vec<&str> = dates.split(',').map(|d| d.trim().trim_matches('"')).collect();
    let equity: Vec<f64> = eq.split(',').filter_map(|v| v.trim().parse().ok()).collect();
    let at: std::collections::HashMap<&str, usize> = dates.iter().enumerate().map(|(i, d)| (*d, i)).collect();
    let equity0 = jnum(json, "model_initial_equity").unwrap_or(1_000_000.0);
    let last = dates.len().saturating_sub(1);
    let mut list: Vec<crate::amsheet::TradeIn> = Vec::new();
    for o in trades.split("},{") {
        let (Some(entry), Some(exit)) = (jstr(o, "entry"), jstr(o, "exit")) else { continue };
        let (ep, xp, ret, pnl, w) = (jnum(o, "entry_px").unwrap_or(f64::NAN), jnum(o, "exit_px").unwrap_or(f64::NAN), jnum(o, "ret").unwrap_or(f64::NAN), jnum(o, "pnl").unwrap_or(f64::NAN), jnum(o, "weight").unwrap_or(0.0));
        let Some(&i) = at.get(entry) else { continue };
        if !(ret.is_finite() && pnl.is_finite() && ep > 0.0) { continue; }
        let dir = if (xp / ep - 1.0) * ret >= 0.0 { 1.0 } else { -1.0 };
        list.push(crate::amsheet::TradeIn { dir, entry: i, exit: at.get(exit).copied().unwrap_or(last), ret, avg_px: ep, exit_px: xp, profit: Some(pnl / equity0), weight: w });
    }
    let w: Vec<&str> = dates.clone();
    crate::amsheet::row(&w, &[], &equity, &list)
}

/// `GET guru/yt/ath-optimize`: the backtest of `ath-run` for the default settings and for every combination of the `grid.<key>` value
/// lists. The rows are not ranked here; the page sorts them. The run permit is held for the whole grid, and the work a grid point
/// shares with the one before is not repeated (results are the same as separate `ath-run` calls):
/// * the stocks, the index series and the period are read once;
/// * the gross backtest (brokerage 0) is left out: the rows and the sheet read only the net run;
/// * the buy / sell signals are reused while the settings the signal code read keep their values (a MAXIMUM POSITION or POS. SIZE grid
///   finds the signals once). The engine notes which settings a variant's signal code reads, so a variant whose signals do read a
///   portfolio setting gets new signals for each value. Grid points are run grouped by those settings; the rows keep the grid order.
pub fn optimize_json(q: &BTreeMap<String, String>) -> Result<String, String> {
    let id = q.get("id").ok_or("id missing")?.as_str();
    let variant = variant_of(q)?;
    let schema = schema_of(&variant);
    let dims = parse_grids(q, &schema)?;
    let costs = crate::accounting::Costs::parse(q)?;
    let started = Instant::now();
    let base: BTreeMap<String, String> = q.iter().filter(|(k, _)| !k.starts_with("grid.")).map(|(k, v)| (k.clone(), v.clone())).collect();
    let mut points: Vec<BTreeMap<String, f64>> = vec![BTreeMap::new()];
    for (key, vals) in &dims {
        points = points.into_iter().flat_map(|p| vals.iter().map(move |&v| { let mut p = p.clone(); p.insert(key.clone(), v); p })).collect();
    }
    // row 0 = the default settings, then the grid points; every run's settings are checked before the permit is taken
    let default = BTreeMap::new();
    let all: Vec<&BTreeMap<String, f64>> = std::iter::once(&default).chain(points.iter()).collect();
    let knobs: Vec<Knobs> = all
        .iter()
        .map(|point| {
            let mut one = base.clone();
            for (k, v) in point.iter() {
                one.insert(format!("p.{k}"), crate::num(*v));
            }
            Knobs::new(&one, &schema, costs)
        })
        .collect::<Result<_, _>>()?;
    let want_sheet = q.get("sheet").is_some_and(|v| v == "1");
    let trade_bars = q.get("bars").is_some_and(|v| v == "1");

    let _permit = Permit::take()?;
    let s = setup(q, &variant)?;
    // the signals of the last run, with the settings its signal code read and their values
    let mut cache: Option<(Vec<(String, f64)>, Vec<Signals>)> = None;
    let mut order: Vec<usize> = (0..all.len()).collect();
    let mut rows: Vec<String> = vec![String::new(); all.len()];
    let mut sheet: Vec<String> = vec![String::new(); all.len()];
    let mut n = 0;
    while n < order.len() {
        if started.elapsed().as_secs_f64() > OPT_TIME_LIMIT_SECS {
            return Err(format!("unavailable: the optimization reached its {OPT_TIME_LIMIT_SECS:.0}-second budget after {n} of {} runs; no partial ranking is published. Tick fewer settings.", all.len()));
        }
        let i = order[n];
        let k = &knobs[i];
        let same = cache.as_ref().is_some_and(|(read, _)| read.iter().all(|(key, v)| k.get(key).to_bits() == v.to_bits()));
        if !same {
            drop(cache.take()); // free the old signals before the new ones are found
            let read = std::cell::RefCell::new(std::collections::BTreeSet::<String>::new());
            let g = |key: &str| {
                if !read.borrow().contains(key) {
                    read.borrow_mut().insert(key.to_string());
                }
                k.get(key)
            };
            let sgs = signals_of(&variant, &s, &g)?;
            let read: Vec<(String, f64)> = read.into_inner().into_iter().map(|key| { let v = k.get(&key); (key, v) }).collect();
            if n == 0 {
                // run the grid points that give the same signals one after another: sorted by the values of the settings the signals read
                let keys: Vec<&str> = dims.iter().map(|(key, _)| key.as_str()).filter(|key| read.iter().any(|(r, _)| r == key)).collect();
                let at = |j: usize| -> Vec<f64> { keys.iter().map(|key| knobs[j].get(key)).collect() };
                order[1..].sort_by(|&a, &b| at(a).partial_cmp(&at(b)).unwrap_or(std::cmp::Ordering::Equal));
            }
            cache = Some((read, sgs));
        }
        let sgs = &cache.as_ref().unwrap().1;
        let syms: Vec<(&Series, &Signals)> = s.series.iter().zip(sgs.iter()).collect();
        let pp = portfolio_of(&variant, &s, k, costs);
        let net = backtest_full(&syms, &s.rank, &pp);
        if net.dates.is_empty() {
            return Err("needs_data: no bar in the chosen period".into());
        }
        // the net run stands in for the gross one: the rows and the sheet read neither `gross_stats` nor the gross curve
        let out = result_json(&variant, &s.series, &net, &net, &pp, costs, s.n_names, started.elapsed().as_secs_f64(), &s.basket, trade_bars);
        let stats = stats_of(&out)?;
        if want_sheet { sheet[i] = format!("[{}]", sheet_row_of(&out).iter().map(|v| crate::num(*v)).collect::<Vec<_>>().join(",")); }
        let ps: Vec<String> = all[i].iter().map(|(k, v)| format!("\"{k}\":{}", crate::num(*v))).collect();
        let ss: Vec<String> = stats.iter().map(|(k, v)| format!("\"{k}\":{}", crate::num(*v))).collect();
        rows[i] = format!("{{\"default\":{},\"p\":{{{}}},\"stats\":{{{}}}}}", i == 0, ps.join(","), ss.join(","));
        n += 1;
    }
    let grid: Vec<String> = dims.iter().map(|(k, v)| format!("\"{k}\":[{}]", v.iter().map(|x| crate::num(*x)).collect::<Vec<_>>().join(","))).collect();
    Ok(format!("{{\"kind\":\"ath-optimize\",\"status\":\"ready\",\"id\":\"{}\",\"combos\":{},\"grid\":{{{}}},\"seconds\":{},\"rows\":[{}]{}}}", crate::esc(id), points.len(), grid.join(","), crate::num(started.elapsed().as_secs_f64()), rows.join(","), if want_sheet { format!(",\"sheet\":[{}]", sheet.join(",")) } else { String::new() }))
}

fn result_json(v: &VariantDef, series: &[Series], net: &Backtest, gross: &Backtest, pp: &PortfolioParams, costs: crate::accounting::Costs, n_names: usize, seconds: f64, basket: &str, trade_bars: bool) -> String {
    let (universe, database, _, _) = words(v.data);
    let equity0 = pp.initial_equity;
    let c = pp.commission_pct / 100.0;
    let ret = |t: &crate::ath_core::TradeOut| if t.short { (t.entry_price * (1.0 - c) - t.exit_price * (1.0 + c)) / (t.entry_price * (1.0 + c)) } else { (t.exit_price * (1.0 - c) - t.entry_price * (1.0 + c)) / (t.entry_price * (1.0 + c)) };
    let trades: Vec<String> = net
        .trades
        .iter()
        .map(|t| {
            let s = &series[t.sym];
            let bars = if trade_bars {
                let px = |x: f32| crate::num(x.to_string().parse::<f64>().unwrap_or(f64::NAN));   // the stored price as written (25.55, not 25.549999)
                let a = s.bars.partition_point(|b| b.date < t.entry_date);
                let z = s.bars.partition_point(|b| b.date <= t.exit_date);
                let rows: Vec<String> = s.bars[a..z.max(a)].iter().map(|b| format!("[\"{}\",{},{},{},{}]", iso(b.date), px(b.o), px(b.h), px(b.l), px(b.c))).collect();
                format!(",\"bars\":[{}]", rows.join(","))
            } else { String::new() };
            let reason = if t.open { "open" } else if t.stopped { "stop" } else if t.short { "cover (short sale)" } else if s.bars.last().map_or(false, |b| b.date == t.exit_date) { "sell signal (or last bar of the symbol)" } else { "sell signal" };
            format!(
                "{{\"symbol\":\"{}\",\"entry\":\"{}\",\"entry_time\":\"\",\"entry_ts\":\"{}\",\"exit_ts\":{},\"entry_px\":{},\"exit\":\"{}\",\"exit_time\":null,\"exit_px\":{},\"shares\":{},\"pnl\":{},\"weight\":{},\"score\":0,\"ret\":{},\"reason\":\"{}\",\"open\":{},\"short\":{}{}}}",
                crate::esc(&s.name), iso(t.entry_date), iso(t.entry_date), if t.open { "null".to_string() } else { format!("\"{}\"", iso(t.exit_date)) }, crate::num(t.entry_price), iso(t.exit_date), crate::num(t.exit_price), t.shares, crate::num(t.profit), crate::num(t.weight), crate::num(ret(t)), reason, t.open, t.short, bars
            )
        })
        .collect();
    let curve = |b: &Backtest| -> BTreeMap<&'static str, f64> { crate::btst_universe::curve_stats(&b.equity, equity0) };
    let mut stats = curve(net);
    let closed: Vec<f64> = net.trades.iter().filter(|t| !t.open).map(ret).collect();
    let n = closed.len() as f64;
    let wins: Vec<f64> = closed.iter().copied().filter(|r| *r > 0.0).collect();
    let losses: f64 = closed.iter().copied().filter(|r| *r < 0.0).map(|r| -r).sum();
    stats.insert("trades", n);
    stats.insert("win_rate", if n > 0.0 { wins.len() as f64 / n } else { f64::NAN });
    stats.insert("avg_trade", if n > 0.0 { closed.iter().sum::<f64>() / n } else { f64::NAN });
    stats.insert("profit_factor", if losses > 0.0 { wins.iter().sum::<f64>() / losses } else { f64::NAN });
    stats.insert("exposure", net.positions.iter().filter(|&&x| x > 0).count() as f64 / net.positions.len().max(1) as f64);
    let mut gstats = curve(gross);
    gstats.insert("trades", gross.trades.iter().filter(|t| !t.open).count() as f64);
    // buy & hold of the basket's index (NSE 500 / Nifty 50) of the cash database; none for a basket without an index
    let bmark = benchmark_of(basket);
    let bench = bmark.and_then(|(sym, _)| load_index(&index_dir(v.data), sym));
    let (bsym, blabel, bbasis) = match (bmark, &bench) {
        (Some((sym, label)), Some(_)) => (format!("\"{sym}\""), format!("\"{label}\""), format!("Buy & hold of the {label} index ({sym}) of the cash database, the index of the traded basket (price index, dividends not included)")),
        _ => ("null".to_string(), "null".to_string(), format!("No buy & hold: the basket {} has no index in the database", basket.trim_end_matches(".tls"))),
    };
    let base = bench.as_ref().and_then(|b| b.at_or_before(net.dates[0]).map(|i| b.bars[i].c as f64));
    let bh: Vec<String> = net.dates.iter().map(|&d| match (&bench, base) {
        (Some(b), Some(base)) if base > 0.0 => b.at_or_before(d).map_or("null".into(), |i| crate::num(b.bars[i].c as f64 / base)),
        _ => "null".into(),
    }).collect();
    let list = |v: Vec<String>| format!("[{}]", v.join(","));
    let (sizing, alloc) = match pp.fixed_amount { Some(a) => (2, crate::num(a)), None => (1, "null".to_string()) };
    let id = v.id;
    let mut json = format!(
        "{{\"kind\":\"universe\",\"status\":\"ready\",\"id\":\"{id}\",\"src\":1,\"universe\":\"{}\",\"symbols\":{},\"max_positions\":{},\"pct\":{},\"sizing\":{sizing},\"alloc\":{alloc},\"pad\":0,\"model_initial_equity\":{},\"dates\":{},\"equity\":{},\"gross_equity\":{},\"bh\":{},\"benchmark_symbol\":{bsym},\"benchmark_label\":{blabel},\"benchmark_basis\":\"{}\",\"instrument\":{{\"symbol\":\"{}\",\"description\":\"{}\",\"database\":\"{}\"}},\"positions\":{},\"trades\":{},\"stats\":{},\"gross_stats\":{},\"costs\":{},\"portfolio\":{{\"candidates\":{},\"accepted\":{},\"rejected_capacity\":{},\"rejected_cash\":{},\"last_day_signals_dropped\":0,\"candidate_seconds\":0,\"seconds\":{},\"ranking\":\"signals of the same bar by PositionScore (higher first), equal scores in ascending symbol name (as the AmiBroker Analysis window tests them)\",\"sizing\":\"floor(pct% x previous-bar equity / price) whole shares, compounding; equity = cash + open positions at the previous close less the commission to close them; shrink to available cash; a candidate that cannot be funded ends the bar's entries\",\"timing\":\"stops and exits before entries on the same bar; fills at the AFL's BuyPrice / SellPrice inside the bar's range\"}},\"notices\":[\"Prices as stored in the team's database ({}); {} of {} watchlist symbols have data; dead stocks stay in the data.\",\"Simulated portfolio of the team's AFL rules; not a published result.\"]",
        crate::esc(&universe), n_names, pp.max_positions, crate::num(pp.pct_of_equity), crate::num(equity0),
        list(net.dates.iter().map(|&d| format!("\"{}\"", iso(d))).collect()),
        list(net.equity.iter().map(|v| crate::num(v / equity0)).collect()),
        list(gross.equity.iter().map(|v| crate::num(v / equity0)).collect()),
        list(bh), crate::esc(&bbasis), crate::esc(&universe), crate::esc(&universe), crate::esc(database),
        list(net.positions.iter().map(|v| v.to_string()).collect()),
        list(trades), crate::stats_json(&stats), crate::stats_json(&gstats), costs.json(), net.signals, net.accepted, net.rejected_capacity, net.rejected_cash, crate::num(seconds), crate::esc(database), series.len(), n_names
    );
    if let Some(rows) = &net.detail_log {
        let rows = rows.iter().map(|r| format!("[{},\"{}\"]", r.date.map_or("null".to_string(), |d| format!("\"{}\"", iso(d))), crate::esc(&r.information))).collect::<Vec<_>>().join(",");
        json.push_str(&format!(",\"detail_log\":[{rows}]"));
    }
    json.push('}');
    json
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn buy_and_hold_follows_the_basket() {
        assert_eq!(benchmark_of("NSE 500 C&P.tls"), Some(("NSE500", "NSE 500")));
        assert_eq!(benchmark_of("NSE 500 CURRENT.tls"), Some(("NSE500", "NSE 500")));
        assert_eq!(benchmark_of("NIFTY 50.tls"), Some(("NSENIFTY", "NIFTY 50")));
        assert_eq!(benchmark_of("FO STOCKS C&P.tls"), None);
        assert_eq!(benchmark_of("CURRENT F&O STOCK.tls"), None);
        assert!(load_index("/no/such/folder", "NSENIFTY").is_none());
    }

    #[test]
    fn dates_parse_strictly() {
        assert_eq!(date_num("2000-01-03"), Some(20000103));
        assert_eq!(date_num("2000-13-03"), None);
        assert_eq!(date_num("20000103"), None);
    }

    #[test]
    fn every_registered_variant_has_the_common_keys_and_valid_ranges() {
        for v in crate::ath_registry::all() {
            let keys: Vec<&str> = schema_of(&v).iter().map(|d| d.key).collect();
            for k in ["equity", "sizing", "pct", "alloc", "max_positions"] {
                assert!(keys.contains(&k), "{} lacks {k}", v.id);
            }
            assert!(schema_of(&v).iter().all(|d| d.min <= d.def && d.def <= d.max), "{} has a default outside its range", v.id);
        }
    }

    #[test]
    fn registered_ids_are_unique_and_belong_to_the_ath_catalogue() {
        let ids = ids();
        let set: std::collections::BTreeSet<&&str> = ids.iter().collect();
        assert_eq!(set.len(), ids.len());
        for id in &ids {
            assert!(crate::books::ytath::is_variant(id), "{id} is not an ATH sub-variant");
        }
    }

    #[test]
    fn grids_are_parsed_checked_and_capped() {
        let v = crate::ath_registry::all().into_iter().next().unwrap();
        let schema = schema_of(&v);
        let k = schema.iter().find(|d| d.key == "max_positions").unwrap();
        let (lo, hi) = (k.min, k.max);
        let mk = |pairs: &[(&str, String)]| -> BTreeMap<String, String> { pairs.iter().map(|(a, b)| (a.to_string(), b.clone())).collect() };
        let ok = parse_grids(&mk(&[("id", v.id.into()), ("grid.max_positions", format!("{lo}, {hi},{lo}"))]), &schema).unwrap();
        assert_eq!(ok, vec![("max_positions".to_string(), vec![lo, hi])]);
        assert!(parse_grids(&mk(&[("id", v.id.into())]), &schema).is_err());
        assert!(parse_grids(&mk(&[("grid.nope", "1".into())]), &schema).is_err());
        assert!(parse_grids(&mk(&[("grid.max_positions", "x".into())]), &schema).unwrap_err().contains("not a number"));
        // a value past the AFL's own range runs as typed; only a value past the safety cap (or below zero) is refused
        assert!(parse_grids(&mk(&[("grid.max_positions", format!("{}", hi + 1.0))]), &schema).is_ok());
        assert!(parse_grids(&mk(&[("grid.max_positions", "-1".into())]), &schema).unwrap_err().contains("must be"));
        let wide = |n: usize| (0..n).map(|i| format!("{}", (lo + (hi - lo) * i as f64 / n as f64).round())).collect::<Vec<_>>().join(",");
        assert!(parse_grids(&mk(&[("grid.max_positions", wide(21))]), &schema).unwrap_err().contains("limit"));
        let pct = schema.iter().find(|d| d.key == "pct").unwrap();
        let pw = |n: usize| (0..n).map(|i| format!("{}", pct.min + (pct.max - pct.min) * i as f64 / n as f64)).collect::<Vec<_>>().join(",");
        assert!(parse_grids(&mk(&[("grid.max_positions", wide(8)), ("grid.pct", pw(8))]), &schema).unwrap_err().contains("combinations"));
        assert_eq!(parse_grids(&mk(&[("grid.max_positions", wide(6)), ("grid.pct", pw(10))]), &schema).unwrap().len(), 2);
    }

    #[test]
    fn stats_are_read_from_a_run_answer() {
        let s = stats_of("{\"gross_stats\":{\"a\":9},\"stats\":{\"cagr\":0.12,\"sharpe\":null,\"trades\":5},\"costs\":{}}").unwrap();
        assert_eq!(s.len(), 3);
        assert_eq!(s[0], ("cagr".to_string(), 0.12));
        assert!(s[1].1.is_nan() && s[2].1 == 5.0);
    }

    #[test]
    fn an_optimization_of_an_unknown_id_is_refused() {
        let q: BTreeMap<String, String> = [("id".to_string(), "ytath_unknown".to_string()), ("grid.pct".to_string(), "1,2".to_string())].into();
        assert!(optimize_json(&q).unwrap_err().starts_with("not_testable"));
    }

    #[test]
    fn a_run_of_an_unknown_id_is_refused() {
        let q: BTreeMap<String, String> = [("id".to_string(), "ytath_unknown".to_string())].into();
        assert!(run_json(&q).unwrap_err().starts_with("not_testable"));
    }
}

#[cfg(test)]
mod real_data_tests {
    use super::*;

    /// The reference run of the team's sheet "21%" (Probability Analysis.xlsx): 1,348 trades, total profit 324,073,171.
    /// Needs the converted snapshot: `VAULT_ATH_DATA_DIR=~/.cache/strategy-vault-webapp/data/ath-cp-2021-09-06 cargo test --release ath_run -- --ignored`.
    /// `VAULT_ATH_DATA_DIR=~/.cache/strategy-vault-webapp/data/ath-cp-2021-09-06 cargo test --release ath_optimize -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn ath_optimize_runs_the_default_and_a_two_value_grid() {
        let q: BTreeMap<String, String> = [("id", "ytath_pctswing_exit_turnover_running_buy_2"), ("from", "2015-01-01"), ("to", "2021-09-06"), ("slippage", "65"), ("grid.exitpct", "15,21")]
            .iter().map(|(a, b)| (a.to_string(), b.to_string())).collect();
        let out = optimize_json(&q).expect("optimize");
        println!("{}", &out[..out.len().min(1500)]);
        assert_eq!(out.matches("\"default\":").count(), 3);
        assert!(out.contains("\"default\":true") && out.contains("\"p\":{\"exitpct\":15}"));
    }

    #[test]
    #[ignore]
    fn the_reference_settings_reproduce_the_team_sheet_with_21_percent_exit() {
        let q: BTreeMap<String, String> = [
            ("id", "ytath_pctswing_exit_turnover_running_buy_2"), ("slippage", "65"), ("from", "2000-01-01"), ("to", "2021-09-06"),
            ("p.exitpct", "21"), ("p.max_positions", "50"), ("p.pct", "2"), ("p.pennyflt", "10"), ("p.turnflt", "1"), ("p.deadstock", "0"),
        ].iter().map(|(a, b)| (a.to_string(), b.to_string())).collect();
        let out = run_json(&q).expect("run");
        assert!(out.contains("\"kind\":\"universe\""));
        let n = out.matches("\"entry_px\"").count();
        assert_eq!(n, 1348, "trades");
        let info = info_json().expect("info");
        assert!(info.contains("ytath_pctswing_exit_turnover_running_buy_2"));
    }
}
