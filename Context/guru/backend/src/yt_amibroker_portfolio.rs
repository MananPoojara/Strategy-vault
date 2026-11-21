//! AmiBroker-style portfolio backtest for the daily YT stock AFLs (`mode=amibroker`).
//!
//! This is a separate path from `basket_portfolio` (fixed 1/N slots, fractional units), which is unchanged.
//! It follows the team's stock-AFL template ("BACKTEST PARAMETERS" section, e.g. Old Topics AFL/NR7/
//! "Entry with NR7 & exit with ST as SL.afl"):
//!
//! ```text
//! RoundLotSize=1; SetOption("AllowPositionShrinking",True); SetOption("UsePrevBarEquityForPosSizing",True);
//! SetOption("InitialEquity",INEQ); SetOption("commissionmode",1); SetOption("commissionamount",COMM);
//! BKTYPE=IIf(BKTEST==0 AND BKTEST1==1,1,IIf(BKTEST==1,4,2));   // 1 spsValue, 4 spsShares, 2 spsPercentOfEquity
//! SIZE1=IIf(BKTYPE==1,ALLO,SIZE); SetPositionSize(SIZE1,BKTYPE); SetOption("MAXOPENPOSITIONS",MAXPO);
//! ```
//!
//! Semantics as built:
//! - Universe: every symbol file of the chosen native AmiBroker database (WatchLists/, Layouts/ and `~~~` system
//!   symbols skipped). Dated eligibility is the database's AUX field (AUX1, AUX2 or both equal to 1).
//! - Signals: each symbol's unchanged single-symbol runner over its full native history (as the AFL loop runs from
//!   bar 0), trades kept inside the requested window. AUX gate: for the NR7 and Candle Range modules the gate is applied
//!   inside the AFL loop (an `AND Aux1 == 1` on the Buy condition; the loop state never sees a gated entry). For every
//!   other module the gate is applied at the entry (signal) bar of the AFL's own trade list; that equals AmiBroker
//!   when the AFL's Buy is a loop/ExRem'd signal with `AND Aux1==1`, and is a declared adaptation otherwise (a gated
//!   entry does not re-run the AFL loop, so no later entry is generated in its place). The Tripple RSI "(FO)" AFLs
//!   read AUX2 themselves (`Aux2[i]==1` on the Buy, `Aux2[i+1]` in the sell): they receive the symbol's dated AUX2 from
//!   the same native file in the loop, with the basket's rule as an extra in-loop Buy condition (on `fo_cp` the two
//!   coincide); they run only on the C&P databases, whose AUX2 is F&O membership.
//! - Ledger: chronological over the union of eligible symbols' dates; on each bar exits first, then entries
//!   (AmiBroker's backtester processes exits before entries on a bar). A same-bar round trip (entry and exit on one
//!   bar) holds its slot and cash until all of that bar's entries are processed. Competing entries are ranked by AmiBroker's
//!   rules without PositionScore (no daily YT stock module defines one): AmiBroker KB "Symbol selection when
//!   PositionScore is not defined" (2015-09-28): 1) greater PositionSize preferred (identical for every candidate in
//!   all three template modes), 2) alphabetical order, 3) long before short. A rejected entry drops that whole trade.
//! - Sizing: % of equity uses the previous bar's closing portfolio equity (UsePrevBarEquityForPosSizing); shares;
//!   fixed value. Whole shares (RoundLotSize 1, rounded down). Insufficient cash shrinks to the affordable whole
//!   shares including the entry commission (AllowPositionShrinking); zero shares = rejected.
//! - Commission: commissionmode 1 = percent of traded value, charged on entry and on exit.
//! - Shorts: 100% margin; the entry value plus commission is reserved from cash, released with P&L on cover.
//! - Equity: cash + held positions marked at each date's close (last known close carried for a held symbol without
//!   a quote that day). Positions open at the end (or whose symbol's data ended) stay open, marked to market.
#![allow(dead_code)]
use crate::{connors::{Ctx, Params, Trade}, yt_aux::{Aux, Rule}, Row, State};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
    sync::{atomic::{AtomicBool, AtomicUsize, Ordering}, Mutex},
    time::{Duration, Instant},
};

const HEADER: usize = 1184;
const REC: usize = 40;
const MAX_FILES: usize = 8000;
const MAX_CANDIDATES: usize = 400_000;
const MAX_MARKS: usize = 20_000_000;
const THREADS: usize = 4;
/// Same request budget as the fixed-slot basket path (90 s).
const TIME_LIMIT: Duration = Duration::from_secs(90);

static RUNNING: AtomicBool = AtomicBool::new(false);
struct Permit;
impl Permit {
    fn take() -> Result<Self, String> {
        RUNNING.compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire).map(|_| Self)
            .map_err(|_| "busy: An AmiBroker portfolio backtest is already running; retry when it finishes.".into())
    }
}
impl Drop for Permit { fn drop(&mut self) { RUNNING.store(false, Ordering::Release); } }

// ---------------------------------------------------------------- baskets

pub(crate) struct Basket { pub id: &'static str, pub name: &'static str, pub db: &'static str, pub rule: Rule, pub rule_name: &'static str }
pub(crate) const BASKETS: [Basket; 7] = [
    Basket { id: "nse500_cp", name: "NSE 500 C&P (AUX1)", db: "cp_updated_2026", rule: Rule::Aux1, rule_name: "AUX1 == 1" },
    Basket { id: "fo_cp", name: "F&O stocks C&P (AUX2)", db: "cp_updated_2026", rule: Rule::Aux2, rule_name: "AUX2 == 1" },
    Basket { id: "nse500_cp_old", name: "NSE 500 C&P old (to 2022)", db: "cp_old_2022", rule: Rule::Aux1, rule_name: "AUX1 == 1" },
    Basket { id: "nifty50_cp", name: "Nifty 50 C&P", db: "nifty50_cp_2026", rule: Rule::Aux1, rule_name: "AUX1 == 1" },
    Basket { id: "midcap100_cp", name: "Midcap 100 C&P", db: "midcap100_cp_2026", rule: Rule::Aux1, rule_name: "AUX1 == 1" },
    Basket { id: "midcap100_fo", name: "Midcap 100 F&O (AUX1 & AUX2)", db: "midcap100_aux12_2025", rule: Rule::Aux1AndAux2, rule_name: "AUX1 == 1 AND AUX2 == 1" },
    Basket { id: "midcapselect_cp", name: "Midcap Select C&P", db: "midcapselect_cp_2026", rule: Rule::Aux1, rule_name: "AUX1 == 1" },
];
pub(crate) fn basket(id: &str) -> Option<&'static Basket> { BASKETS.iter().find(|b| b.id == id) }

// ---------------------------------------------------------------- native reader

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct RawBar { pub date: (u16, u8, u8), pub c: f32, pub o: f32, pub h: f32, pub l: f32, pub v: f32, pub oi: f32, pub aux1: f32, pub aux2: f32 }

/// AmiBroker format 5 ("BROKDAt5"): 1,184-byte header, bar count u32 at 0x49C, 40-byte records (u64 packed date,
/// f32 close, open, high, low, volume, openint, aux1, aux2). Exactly the declared count is read; stale records after
/// it (seen in a few files) are ignored, as AmiBroker does. Records with an invalid packed date are skipped.
pub(crate) fn read_native(bytes: &[u8]) -> Result<Vec<RawBar>, String> {
    if bytes.len() < HEADER || &bytes[..8] != b"BROKDAt5" { return Err("not an AmiBroker format-5 file".into()); }
    let count = u32::from_le_bytes(bytes[HEADER - 4..HEADER].try_into().unwrap()) as usize;
    if count > 200_000 { return Err("invalid AmiBroker bar count".into()); }
    let end = HEADER + count * REC;
    if end > bytes.len() { return Err("truncated AmiBroker file: fewer records than its declared bar count".into()); }
    let mut out = Vec::with_capacity(count);
    for r in bytes[HEADER..end].chunks_exact(REC) {
        let d = u64::from_le_bytes(r[..8].try_into().unwrap());
        let (y, m, day) = ((d >> 52) as u16, ((d >> 48) & 15) as u8, ((d >> 43) & 31) as u8);
        if !(1900..=2200).contains(&y) || !(1..=12).contains(&m) || day == 0 { continue; }
        let f = |k: usize| f32::from_le_bytes(r[k..k + 4].try_into().unwrap());
        out.push(RawBar { date: (y, m, day), c: f(8), o: f(12), h: f(16), l: f(20), v: f(24), oi: f(28), aux1: f(32), aux2: f(36) });
    }
    Ok(out)
}

/// Bar hygiene of jay-local/_checks/build_basket.py: drop bars with a price <= 0 / non-finite or a duplicate date
/// (first kept), widen high/low to cover open/close; volume floored at 0.
pub(crate) fn clean(symbol: &str, raw: &[RawBar]) -> (Vec<Row>, Vec<Aux>) {
    let mut idx: Vec<usize> = (0..raw.len()).collect();
    idx.sort_by_key(|&i| raw[i].date); // stable: the first of duplicate dates stays first
    let (mut rows, mut aux) = (Vec::with_capacity(raw.len()), Vec::with_capacity(raw.len()));
    let mut last = None;
    for i in idx {
        let b = raw[i];
        if last == Some(b.date) { continue; }
        let (o, h, l, c, v) = (b.o as f64, b.h as f64, b.l as f64, b.c as f64, b.v as f64);
        if [o, h, l, c, v].iter().any(|x| !x.is_finite()) || o.min(h).min(l).min(c) <= 0. { continue; }
        let date = format!("{:04}-{:02}-{:02}", b.date.0, b.date.1, b.date.2);
        if !crate::baskets::valid_date(&date) { continue; }
        last = Some(b.date);
        rows.push(Row { ticker: symbol.into(), date, open: o, high: h.max(o).max(c), low: l.min(o).min(c), close: c, volume: Some(v.max(0.)) });
        aux.push(Aux { aux1: b.aux1, aux2: b.aux2, open_interest: b.oi });
    }
    (rows, aux)
}

/// Every symbol file of the database (`<db>/<first letter|digit|_>/<SYMBOL>`), sorted by symbol.
pub(crate) fn universe(db: &Path) -> Result<Vec<(String, PathBuf)>, String> {
    let mut out = Vec::new();
    for dir in std::fs::read_dir(db).map_err(|e| format!("needs_data: cannot read the AmiBroker database {}: {e}", db.display()))? {
        let dir = dir.map_err(|e| e.to_string())?;
        let name = dir.file_name().to_string_lossy().into_owned();
        if name.chars().count() != 1 || !dir.path().is_dir() { continue; } // WatchLists/, Layouts/ and loose files skipped
        for f in std::fs::read_dir(dir.path()).map_err(|e| e.to_string())? {
            let f = f.map_err(|e| e.to_string())?;
            let symbol = f.file_name().to_string_lossy().into_owned();
            if symbol.starts_with("~~~") || !f.path().is_file() { continue; }
            out.push((symbol, f.path()));
        }
    }
    out.sort_by(|a, b| rank_key(&a.0).cmp(&rank_key(&b.0)).then_with(|| a.0.cmp(&b.0)));
    Ok(out)
}
/// Coerce a closure to the higher-ranked context builder signature.
pub(crate) fn ctx_builder<F: for<'a> Fn(&'a [Row]) -> Ctx<'a> + Sync>(f: F) -> F { f }
/// Alphabetical order used for ranking ties (case-insensitive byte order).
fn rank_key(s: &str) -> String { s.to_ascii_uppercase() }

// ---------------------------------------------------------------- ledger

#[derive(Clone, Debug)]
pub(crate) struct Cand {
    pub symbol: String,
    pub dir: f64,
    pub entry_day: i64,
    pub exit_day: i64,
    pub entry_px: f64,
    pub exit_px: f64,
    /// Still open at the end of the window (or the symbol's data ended): marked, never closed.
    pub open: bool,
    pub reason: String,
    pub bars: usize,
    /// (day number, close) from the entry bar to the exit bar (or the last bar for open trades).
    pub marks: Vec<(i64, f64)>,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Size { PctEquity(f64), Shares(f64), Value(f64) }
#[derive(Clone, Copy, Debug)]
pub(crate) struct Settings { pub initial: f64, pub size: Size, pub max_positions: usize, pub fee: f64 }
#[derive(Clone, Debug)]
pub(crate) struct Filled { pub cand: usize, pub shares: f64, pub entry_fee: f64, pub exit_fee: f64, pub pnl: f64, pub open: bool, pub mark: f64, pub shrunk: bool }
#[derive(Debug, Default)]
pub(crate) struct Ledger {
    pub equity: Vec<f64>, pub cash: Vec<f64>, pub positions: Vec<usize>, pub fills: Vec<Filled>,
    pub rejected_capacity: usize, pub rejected_cash: usize, pub rejected_zero_size: usize, pub shrunk: usize, pub stale_marks: usize,
}

struct Held { fill: usize, cursor: usize, mark: f64 }

/// Portfolio ledger over a sorted calendar of day numbers. See the module notes for the rules.
pub(crate) fn ledger(c: &[Cand], calendar: &[i64], s: Settings) -> Result<Ledger, String> {
    if calendar.is_empty() || calendar.windows(2).any(|w| w[0] >= w[1]) { return Err("needs_data: no sorted portfolio calendar".into()); }
    if !s.initial.is_finite() || s.initial <= 0. || s.max_positions == 0 || !s.fee.is_finite() || !(0.0..0.5).contains(&s.fee) { return Err("invalid portfolio settings".into()); }
    let mut by_entry: BTreeMap<i64, Vec<usize>> = BTreeMap::new();
    for (i, t) in c.iter().enumerate() {
        if ![1., -1.].contains(&t.dir) || !(t.entry_px > 0.) || !(t.exit_px > 0.) || !t.entry_px.is_finite() || !t.exit_px.is_finite() || t.exit_day < t.entry_day || t.marks.is_empty() {
            return Err(format!("calculation_unavailable: invalid AFL trade for {}", t.symbol));
        }
        by_entry.entry(t.entry_day).or_default().push(i);
    }
    for v in by_entry.values_mut() { v.sort_by(|&a, &b| rank_key(&c[a].symbol).cmp(&rank_key(&c[b].symbol)).then_with(|| c[a].symbol.cmp(&c[b].symbol)).then(a.cmp(&b))); }
    let mut out = Ledger::default();
    let mut cash = s.initial;
    let mut prev_equity = s.initial;
    let mut held: Vec<Held> = Vec::new();
    let close = |out: &mut Ledger, cash: &mut f64, f: usize| {
        let fill = &mut out.fills[f];
        let t = &c[fill.cand];
        let exit_fee = fill.shares * t.exit_px * s.fee;
        let principal = fill.shares * t.entry_px;
        // Long: sale proceeds. Short: reserved margin released plus the cover P&L.
        *cash += if t.dir > 0. { fill.shares * t.exit_px } else { principal + fill.shares * (t.entry_px - t.exit_px) } - exit_fee;
        fill.exit_fee = exit_fee;
        fill.pnl = t.dir * fill.shares * (t.exit_px - t.entry_px) - fill.entry_fee - exit_fee;
        fill.mark = t.exit_px;
    };
    for &day in calendar {
        // 1) exits
        let mut k = 0;
        while k < held.len() {
            let t = &c[out.fills[held[k].fill].cand];
            if !t.open && t.exit_day == day { let f = held.remove(k).fill; close(&mut out, &mut cash, f); } else { k += 1; }
        }
        // 2) entries, ranked
        if let Some(list) = by_entry.get(&day) {
            for &i in list {
                let t = &c[i];
                if held.len() >= s.max_positions || held.iter().any(|h| c[out.fills[h.fill].cand].symbol == t.symbol) { out.rejected_capacity += 1; continue; }
                let desired = match s.size {
                    Size::PctEquity(p) => (prev_equity * p / 100. / t.entry_px).floor(),
                    Size::Shares(n) => n.floor(),
                    Size::Value(v) => (v / t.entry_px).floor(),
                };
                if !(desired >= 1.) { out.rejected_zero_size += 1; continue; }
                let unit = t.entry_px * (1. + s.fee);
                let mut shares = desired;
                let mut shrunk = false;
                if shares * unit > cash + 1e-9 {
                    shares = (cash.max(0.) / unit).floor();
                    if shares < 1. { out.rejected_cash += 1; continue; }
                    shrunk = true;
                    out.shrunk += 1;
                }
                let entry_fee = shares * t.entry_px * s.fee;
                cash -= shares * t.entry_px + entry_fee;
                if cash < 0. && cash > -1e-6 { cash = 0.; }
                out.fills.push(Filled { cand: i, shares, entry_fee, exit_fee: 0., pnl: 0., open: t.open, mark: t.entry_px, shrunk });
                let f = out.fills.len() - 1;
                held.push(Held { fill: f, cursor: 0, mark: t.entry_px });
            }
            // Same-bar round trips: the position occupied a slot (and its cash) for every entry of this bar; it is
            // closed after all of the bar's entries, so its slot and proceeds cannot fund another entry on the same bar.
            let mut k = 0;
            while k < held.len() {
                let t = &c[out.fills[held[k].fill].cand];
                if !t.open && t.exit_day == day && t.entry_day == day { let f = held.remove(k).fill; close(&mut out, &mut cash, f); } else { k += 1; }
            }
        }
        // 3) marks
        let mut equity = cash;
        for h in held.iter_mut() {
            let fill = &out.fills[h.fill];
            let t = &c[fill.cand];
            while h.cursor < t.marks.len() && t.marks[h.cursor].0 <= day { h.mark = t.marks[h.cursor].1; h.cursor += 1; }
            if h.cursor == 0 || t.marks[h.cursor - 1].0 != day { out.stale_marks += 1; }
            equity += if t.dir > 0. { fill.shares * h.mark } else { fill.shares * (2. * t.entry_px - h.mark) };
        }
        if !equity.is_finite() || !cash.is_finite() { return Err("calculation_unavailable: non-finite portfolio equity".into()); }
        out.equity.push(equity);
        out.cash.push(cash);
        out.positions.push(held.len());
        prev_equity = equity;
    }
    for h in &held {
        let fill = &mut out.fills[h.fill];
        let t = &c[fill.cand];
        fill.open = true;
        fill.mark = h.mark;
        fill.pnl = t.dir * fill.shares * (h.mark - t.entry_px) - fill.entry_fee; // no exit commission on an open mark
    }
    Ok(out)
}

// ---------------------------------------------------------------- signals per symbol

/// How the AUX eligibility reaches the AFL.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Gate { InLoop, EntryBar, AflAux2 }
pub(crate) fn gate_of(id: &str) -> Gate {
    if crate::yt_aux::reads_aux2(id) { Gate::AflAux2 }
    else if crate::books::ytnr7::IDS.contains(&id) || crate::books::ytcr::IDS.contains(&id) { Gate::InLoop } else { Gate::EntryBar }
}

pub(crate) struct SymbolResult { pub symbol: String, pub cands: Vec<Cand>, pub days: Vec<i64>, pub aux_filtered: usize, pub error: Option<String> }

/// The AFL's trades for one symbol inside [from_day, to_day] with the AUX rule applied (see module notes).
pub(crate) fn symbol_trades(id: &str, def: &crate::connors::StrategyDef, symbol: &str, rows: &[Row], aux: &[Aux], rule: Rule, from_day: i64, to_day: i64, params: &Params, make_ctx: &(dyn Fn(&[Row]) -> Ctx<'_> + Sync)) -> SymbolResult {
    let mut res = SymbolResult { symbol: symbol.into(), cands: vec![], days: vec![], aux_filtered: 0, error: None };
    let day: Vec<i64> = rows.iter().map(|r| crate::day_number(&r.date)).collect();
    let from_i = day.partition_point(|&d| d < from_day);
    let to_end = day.partition_point(|&d| d <= to_day);
    let eligible: Vec<bool> = aux.iter().map(|&a| rule.allows(a)).collect();
    if from_i >= to_end || !eligible[from_i..to_end].iter().any(|&e| e) { return res; } // never a member in the window
    let to_i = to_end - 1;
    res.days = day[from_i..=to_i].to_vec();
    let mut ctx = make_ctx(rows);
    if gate_of(id) == Gate::AflAux2 { ctx.extra.insert(crate::yt_aux::AUX2_SERIES.into(), aux.iter().map(|a| a.aux2 as f64).collect()); }
    let missing = ctx.missing(def.needs_series);
    if !missing.is_empty() { res.error = Some(format!("needs_data: {symbol} lacks required series {}", missing.join(", "))); return res; }
    let Some(start) = crate::window_start(&ctx, def, from_i).filter(|&s| s <= to_i) else { return res; };
    let trades: Option<Vec<Trade>> = match gate_of(id) {
        Gate::InLoop if crate::books::ytnr7::IDS.contains(&id) => crate::books::ytnr7::source_signals_with_eligibility(id, &ctx, params, Some(&eligible)).map(|s| crate::books::ytnr7::fills(&ctx, &s, start, to_i, "close")),
        Gate::InLoop => {
            let turnover_on = params.get("turnover").is_some_and(|v| v.round() == 1.);
            if turnover_on && aux.iter().any(|a| !a.open_interest.is_finite()) { res.error = Some(format!("needs_data: {symbol} OpenInt turnover field is invalid")); return res; }
            let oi: Vec<f64> = aux.iter().map(|a| a.open_interest as f64).collect();
            crate::books::ytcr::run_with_eligibility(id, &ctx, start, to_i, params, Some(&eligible), Some(&oi))
        }
        Gate::EntryBar => crate::connors::run(id, &ctx, start, to_i, params),
        Gate::AflAux2 => crate::books::yttr::run_with_gate(id, &ctx, start, to_i, params, Some(&eligible)),
    };
    let Some(trades) = trades else { res.error = Some(format!("calculation_unavailable: strategy runner unavailable for {symbol}")); return res; };
    for t in trades {
        if t.legs.len() != 1 || !t.exits.is_empty() { res.error = Some(format!("calculation_unavailable: {symbol} has a scale-in/scale-out trade; AmiBroker sigScaleIn/sigScaleOut sizing is not modelled in this portfolio mode")); return res; }
        let e = t.legs[0].i;
        if e < from_i || t.exit_i > to_i || t.exit_i < e { continue; }
        if gate_of(id) == Gate::EntryBar && !eligible[e] { res.aux_filtered += 1; continue; }
        // The YT runners close a trade still open at the window end "at the last bar's close"; here it stays open.
        let open = t.open || t.reason.starts_with("End of backtest");
        let last = if open { to_i } else { t.exit_i };
        res.cands.push(Cand {
            symbol: symbol.into(), dir: t.dir, entry_day: day[e], exit_day: day[t.exit_i], entry_px: t.legs[0].px, exit_px: t.exit_px,
            open, reason: if open { "Open".into() } else { t.reason.clone() }, bars: t.exit_i - e,
            marks: (e..=last).map(|k| (day[k], ctx.c[k])).collect(),
        });
    }
    res
}

/// Load the universe in parallel (bounded threads), lazily per symbol.
pub(crate) fn collect(files: &[(String, PathBuf)], id: &str, def: &crate::connors::StrategyDef, rule: Rule, from_day: i64, to_day: i64, params: &Params, make_ctx: &(dyn Fn(&[Row]) -> Ctx<'_> + Sync), deadline: Instant) -> Result<Vec<SymbolResult>, String> {
    let next = AtomicUsize::new(0);
    let out: Mutex<Vec<SymbolResult>> = Mutex::new(Vec::new());
    let failure: Mutex<Option<String>> = Mutex::new(None);
    let stop = AtomicBool::new(false);
    std::thread::scope(|scope| {
        for _ in 0..std::thread::available_parallelism().map_or(2, |n| n.get()).min(THREADS).min(files.len().max(1)) {
            scope.spawn(|| loop {
                if stop.load(Ordering::Relaxed) { break; }
                let k = next.fetch_add(1, Ordering::Relaxed);
                let Some((symbol, path)) = files.get(k) else { break };
                if Instant::now() > deadline { *failure.lock().unwrap() = Some("unavailable: The AmiBroker portfolio request reached its 90-second budget; no partial result is published.".into()); stop.store(true, Ordering::Relaxed); break; }
                let result = std::fs::read(path).map_err(|e| format!("needs_data: cannot read {symbol}: {e}")).and_then(|b| read_native(&b).map_err(|e| format!("needs_data: {symbol}: {e}")));
                let raw = match result { Ok(r) => r, Err(e) => { *failure.lock().unwrap() = Some(e); stop.store(true, Ordering::Relaxed); break; } };
                let (rows, aux) = clean(symbol, &raw);
                drop(raw);
                if rows.is_empty() { continue; }
                let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| symbol_trades(id, def, symbol, &rows, &aux, rule, from_day, to_day, params, make_ctx)))
                    .unwrap_or_else(|_| SymbolResult { symbol: symbol.clone(), cands: vec![], days: vec![], aux_filtered: 0, error: Some(format!("calculation_unavailable: the strategy runner failed on {symbol}")) });
                if let Some(e) = &r.error { *failure.lock().unwrap() = Some(e.clone()); stop.store(true, Ordering::Relaxed); break; }
                if !r.days.is_empty() { out.lock().unwrap().push(r); }
            });
        }
    });
    if let Some(e) = failure.into_inner().unwrap() { return Err(e); }
    let mut v = out.into_inner().unwrap();
    v.sort_by(|a, b| rank_key(&a.symbol).cmp(&rank_key(&b.symbol)).then_with(|| a.symbol.cmp(&b.symbol)));
    Ok(v)
}

// ---------------------------------------------------------------- request

pub(crate) struct Request { pub id: String, pub basket: &'static Basket, pub initial: f64, pub size: Size, pub size_mode: &'static str, pub size_value: f64, pub max_positions: usize, pub brokerage: f64, pub from: Option<String>, pub to: Option<String> }

pub(crate) fn parse(q: &BTreeMap<String, String>) -> Result<Request, String> {
    let id = q.get("id").filter(|s| !s.is_empty()).ok_or("id missing")?.clone();
    let basket = q.get("basket").and_then(|b| basket(b)).ok_or_else(|| format!("basket must be one of {}", BASKETS.iter().map(|b| b.id).collect::<Vec<_>>().join(", ")))?;
    let number = |k: &str, default: f64| -> Result<f64, String> {
        match q.get(k).filter(|s| !s.is_empty()) { None => Ok(default), Some(s) => s.parse::<f64>().ok().filter(|v| v.is_finite()).ok_or(format!("{k} must be a number")) }
    };
    let initial = number("initial_equity", 1_000_000.)?;
    if !(initial > 0. && initial <= 1e12) { return Err("initial_equity must be above 0 and at most 1e12".into()); }
    let size_mode = match q.get("size_mode").map_or("pct", String::as_str) { "pct" => "pct", "shares" => "shares", "value" => "value", _ => return Err("size_mode must be pct, shares or value".into()) };
    let size_value = number("size_value", match size_mode { "pct" => 5., "shares" => 1., _ => 100_000. })?;
    let size = match size_mode {
        "pct" if size_value > 0. && size_value <= 100. => Size::PctEquity(size_value),
        "shares" if size_value >= 1. && size_value <= 1e9 && size_value.fract() == 0. => Size::Shares(size_value),
        "value" if size_value > 0. && size_value <= 1e12 => Size::Value(size_value),
        "pct" => return Err("size_value for % of equity must be above 0 and at most 100".into()),
        "shares" => return Err("size_value for shares must be a whole number of at least 1".into()),
        _ => return Err("size_value for a fixed value must be above 0".into()),
    };
    let max_positions = number("max_positions", 20.)?;
    if !(1. ..=5000.).contains(&max_positions) || max_positions.fract() != 0. { return Err("max_positions must be a whole number from 1 to 5000".into()); }
    let brokerage = number("brokerage", 0.)?;
    if !(0. ..=5.).contains(&brokerage) { return Err("brokerage must be a percentage from 0 to 5".into()); }
    let date = |k: &str| -> Result<Option<String>, String> {
        match q.get(k).filter(|s| !s.is_empty()) { None => Ok(None), Some(s) if crate::baskets::valid_date(s) => Ok(Some(s.clone())), _ => Err(format!("{k} must be a valid YYYY-MM-DD date")) }
    };
    let (from, to) = (date("from")?, date("to")?);
    if from.as_ref().zip(to.as_ref()).is_some_and(|(f, t)| f >= t) { return Err("from must be before to".into()); }
    for (k, v) in q.iter().filter(|(k, _)| k.starts_with("p.")) {
        if !v.parse::<f64>().is_ok_and(|n| n.is_finite() && n.abs() <= 1_000_000.) { return Err(format!("{k} must be a finite bounded strategy parameter")); }
    }
    Ok(Request { id, basket, initial, size, size_mode, size_value, max_positions: max_positions as usize, brokerage, from, to })
}

/// Daily YT stock strategies admitted to this mode: live, stock-admitted by the instrument policy, not intraday.
pub(crate) fn admission(def: &crate::connors::StrategyDef) -> Result<(), String> {
    if !def.book.starts_with("yt_") { return Err("not_testable: The AmiBroker portfolio mode is available for the YT daily stock strategies only.".into()); }
    if crate::instrument_policy::intraday_rule(def) { return Err("not_testable: This is an intraday AFL (1-minute F&O futures bars); the AmiBroker portfolio mode runs daily stock AFLs only.".into()); }
    if def.status != crate::connors::Status::Live { return Err("not_testable: This strategy is not live in the catalogue.".into()); }
    let p = crate::instrument_policy::policy(def);
    // The team's own types (`team_types`): Jay's widening of the single-series YT admission (20-11-2026) leaves this
    // stock-universe portfolio mode unchanged.
    if !p.team_types.contains(&"individual_stock") || p.requirement == "source_review_required" {
        return Err("not_testable: This AFL is admitted for index, ETF or futures series only; an index cannot substitute for a stock universe.".into());
    }
    Ok(())
}

fn q(s: &str) -> String {
    let mut o = String::from("\"");
    for ch in s.chars() { match ch { '"' => o.push_str("\\\""), '\\' => o.push_str("\\\\"), c if c.is_control() => o.push_str(&format!("\\u{:04x}", c as u32)), c => o.push(c) } }
    o.push('"');
    o
}
fn n(v: f64) -> String { if v.is_finite() { format!("{}", (v * 1e6).round() / 1e6) } else { "null".into() } }
fn civil(z: i64) -> String {
    // inverse of crate::day_number (Howard Hinnant's civil_from_days)
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + if m <= 2 { 1 } else { 0 };
    format!("{y:04}-{m:02}-{d:02}")
}

fn hold(basket: &str, status: &str, note: &str) -> String {
    format!("{{\"kind\":\"basket\",\"mode\":\"amibroker\",\"status\":{},\"blocked\":{},\"basket\":{},\"stats\":null,\"trades\":[],\"dates\":[],\"equity\":[]}}", q(status), q(note), q(basket))
}
fn status_of(note: &str) -> (&'static str, &str) {
    for s in ["needs_data", "not_testable", "calculation_unavailable", "unavailable"] {
        if let Some(rest) = note.strip_prefix(s).and_then(|r| r.strip_prefix(": ")) { return (s, rest); }
    }
    ("unavailable", note)
}

pub fn run(st: &State, q_: &BTreeMap<String, String>) -> Result<String, String> {
    let root = std::env::var_os("AMIBROKER_DATA_DIR").map(PathBuf::from);
    run_with_root(st, q_, root.as_deref())
}

pub(crate) fn run_with_root(st: &State, query: &BTreeMap<String, String>, root: Option<&Path>) -> Result<String, String> {
    let req = parse(query)?;
    let catalogue = crate::query_catalogue(query);
    let def = catalogue.iter().find(|d| d.id == req.id).ok_or("unknown strategy")?;
    if let Err(e) = admission(def) { let (s, note) = status_of(&e); return Ok(hold(req.basket.id, s, note)); }
    if crate::yt_aux::reads_aux2(def.id) && !crate::yt_aux::AUX2_FO_DATABASES.contains(&req.basket.db) {
        return Ok(hold(req.basket.id, "not_testable", "This AFL reads AUX2 as F&O membership, which is established only for the C&P cash databases (baskets fo_cp, nse500_cp, nse500_cp_old)."));
    }
    let params = crate::params_of(query);
    if def.id == crate::books::ytcr::SHORT && params.get("hsl_pct").copied().unwrap_or(2.) > 0. {
        return Ok(hold(req.basket.id, "not_testable", "This short AFL's hard stop references an unassigned BuyPrice. Set the hard stop to zero or supply an AmiBroker trade comparison that resolves its behavior."));
    }
    let Some(root) = root else { return Ok(hold(req.basket.id, "needs_data", "AMIBROKER_DATA_DIR is not set: the engine has no read-only copy of the native AmiBroker databases.")); };
    let db = root.join(req.basket.db);
    if !db.is_dir() { return Ok(hold(req.basket.id, "needs_data", &format!("The native AmiBroker database {} is not available to the engine.", req.basket.db))); }
    let _permit = Permit::take()?;
    let started = Instant::now();
    let files = match universe(&db) { Ok(f) => f, Err(e) => { let (s, note) = status_of(&e); return Ok(hold(req.basket.id, s, note)); } };
    if files.is_empty() || files.len() > MAX_FILES { return Ok(hold(req.basket.id, "needs_data", "The native database has no symbol files or exceeds the bounded 8,000-file request limit.")); }
    let from_day = req.from.as_deref().map_or(i64::MIN, crate::day_number);
    let to_day = req.to.as_deref().map_or(i64::MAX, crate::day_number);
    let make_ctx = ctx_builder(|rows| crate::native_stock_context(st, rows, crate::baskets::Market::India));
    let collected = match collect(&files, def.id, def, req.basket.rule, from_day, to_day, &params, &make_ctx, started + TIME_LIMIT) {
        Ok(v) => v,
        Err(e) => { let (s, note) = status_of(&e); return Ok(hold(req.basket.id, s, &format!("Full portfolio withheld: {note}"))); }
    };
    let mut calendar: BTreeSet<i64> = BTreeSet::new();
    let mut cands = Vec::new();
    let mut aux_filtered = 0;
    let mut marks = 0usize;
    let mut symbols_with_trades = 0;
    for r in &collected {
        calendar.extend(r.days.iter().copied());
        aux_filtered += r.aux_filtered;
        if !r.cands.is_empty() { symbols_with_trades += 1; }
        marks += r.cands.iter().map(|c| c.marks.len()).sum::<usize>();
        cands.extend(r.cands.iter().cloned());
    }
    if cands.len() > MAX_CANDIDATES || marks > MAX_MARKS { return Ok(hold(req.basket.id, "unavailable", "The portfolio exceeds its bounded candidate/quote memory budget; no truncated result is published.")); }
    let calendar: Vec<i64> = calendar.into_iter().collect();
    if calendar.len() < 2 { return Ok(hold(req.basket.id, "needs_data", "Fewer than two dates with an eligible basket member are covered by this request.")); }
    cands.sort_by(|a, b| a.entry_day.cmp(&b.entry_day).then_with(|| rank_key(&a.symbol).cmp(&rank_key(&b.symbol))));
    let settings = Settings { initial: req.initial, size: req.size, max_positions: req.max_positions, fee: req.brokerage / 100. };
    let l = match ledger(&cands, &calendar, settings) { Ok(l) => l, Err(e) => { let (s, note) = status_of(&e); return Ok(hold(req.basket.id, s, note)); } };
    if started.elapsed() > TIME_LIMIT { return Ok(hold(req.basket.id, "unavailable", "The AmiBroker portfolio request reached its 90-second budget; no partial result is published.")); }
    let dates: Vec<String> = calendar.iter().map(|&d| civil(d)).collect();
    let stats = stats(&cands, &l, &calendar, req.initial);
    // Benchmark NIFTY 50 buy & hold on the portfolio calendar (last close carried).
    let (bh, bench_symbol) = match st.ctxs.get(crate::native_benchmark(crate::baskets::Market::India)).filter(|c| !c.r.is_empty()) {
        Some(b) => {
            let m: BTreeMap<&str, f64> = b.r.iter().map(|r| (r.date.as_str(), r.close)).collect();
            let mut last = f64::NAN;
            let raw: Vec<f64> = dates.iter().map(|d| { if let Some(&v) = m.get(d.as_str()) { last = v; } else if last.is_nan() { last = m.range(d.as_str()..).next().map_or(f64::NAN, |(_, &v)| v); } last }).collect();
            let base = raw[0];
            (raw.iter().map(|v| v / base).collect::<Vec<_>>(), Some(crate::native_benchmark(crate::baskets::Market::India)))
        }
        None => (vec![f64::NAN; dates.len()], None),
    };
    let index_of = |day: i64| calendar.binary_search(&day).unwrap_or_else(|i| i.min(calendar.len() - 1));
    let trades: Vec<String> = l.fills.iter().map(|f| {
        let t = &cands[f.cand];
        let (ei, xi) = (index_of(t.entry_day), if f.open { calendar.len() - 1 } else { index_of(t.exit_day) });
        let exit_px = if f.open { f.mark } else { t.exit_px };
        let value = f.shares * t.entry_px;
        format!("{{\"symbol\":{},\"ticker\":{},\"dir\":{},\"shares\":{},\"legs\":[[{},{},1]],\"entry\":{},\"entry_px\":{},\"avg_px\":{},\"exit\":{},\"exit_i\":{},\"exit_px\":{},\"open\":{},\"reason\":{},\"bars\":{},\"entry_value\":{},\"fees\":{},\"pnl\":{},\"ret\":{},\"shrunk\":{},\"entry_kind\":\"close\",\"exits\":[]}}",
            q(&t.symbol), q(&t.symbol), t.dir, n(f.shares), ei, n(t.entry_px), q(&dates[ei]), n(t.entry_px), n(t.entry_px), q(&dates[xi]), xi, n(exit_px), f.open, q(&t.reason), t.bars, n(value), n(f.entry_fee + f.exit_fee), n(f.pnl), n(f.pnl / value), f.shrunk)
    }).collect();
    // Every signal of the AFL trade lists, taken by the portfolio or not, for the download's DetailLog sheet:
    // [symbol, direction, entry date index, exit date index (-1 while open), 1 if the portfolio took the trade].
    let taken: std::collections::HashSet<usize> = l.fills.iter().map(|f| f.cand).collect();
    let signals: Vec<String> = cands.iter().enumerate().map(|(k, t)| format!("[{},{},{},{},{}]", q(&t.symbol), t.dir, index_of(t.entry_day), if t.open { -1 } else { index_of(t.exit_day) as i64 }, taken.contains(&k) as u8)).collect();
    let arr = |v: &[f64]| format!("[{}]", v.iter().map(|x| n(*x)).collect::<Vec<_>>().join(","));
    let size_basis = match req.size {
        Size::PctEquity(p) => format!("{p}% of the previous bar's closing portfolio equity (SetPositionSize(SIZE, spsPercentOfEquity) with UsePrevBarEquityForPosSizing = True)"),
        Size::Shares(s) => format!("{s} shares per entry (SetPositionSize(SIZE, spsShares))"),
        Size::Value(v) => format!("Fixed value {v} per entry, the AFL's EQUITY ALLOCATION (SetPositionSize(ALLO, spsValue))"),
    };
    let gate = gate_of(def.id);
    let aux_basis = match gate {
        Gate::InLoop => format!("Dated {} eligibility is applied inside the AFL loop as an extra Buy condition (the loop state never sees a gated entry). The source AFL has no AUX condition of its own; this equals running it with `AND {}` on its Buy.", req.basket.rule_name, req.basket.rule_name.replace(" == ", "==")),
        Gate::AflAux2 => format!("The AFL reads the database's dated AUX2 (F&O membership) itself: Buy AND Aux2[i]==1 and the Aux2[i+1] terms of its sell test, inside the loop. The basket's {} is applied in the same loop as an extra Buy condition (identical to the AFL's own on fo_cp).", req.basket.rule_name),
        Gate::EntryBar => format!("Dated {} eligibility is checked on the entry (signal) bar of the AFL's own trade list; a gated entry is dropped and the AFL loop is not re-run. This equals AmiBroker when the AFL's Buy is a loop/ExRem'd signal with `AND {}`; otherwise it is a declared adaptation.", req.basket.rule_name, req.basket.rule_name.replace(" == ", "==")),
    };
    let open_count = l.fills.iter().filter(|f| f.open).count();
    let notices = [
        format!("Universe: every symbol of the native AmiBroker database {} with dated {} membership (AUX), so former members and delisted stocks are included; no survivorship bias from a current list.", req.basket.db, req.basket.rule_name),
        "Trade prices are the AFL's own BuyPrice/SellPrice from the unchanged single-symbol runner; the portfolio only decides which trades are taken and how many shares.".into(),
        format!("Sizing: {size_basis}; whole shares (RoundLotSize 1, rounded down); insufficient cash shrinks the order to the affordable whole shares (AllowPositionShrinking)."),
        "This is a research reproduction of AmiBroker's portfolio backtester rules, not a comparison with an AmiBroker result file; differences remain possible (see portfolio notes).".into(),
    ];
    let p = format!("{{\"mode\":\"amibroker\",\"basket\":{},\"basket_name\":{},\"database\":{},\"aux_rule\":{},\"aux_gate\":{},\"initial_equity\":{},\"size_mode\":{},\"size_value\":{},\"max_positions\":{},\"brokerage_pct\":{},\"effective_from\":{},\"effective_to\":{},\"universe_files\":{},\"eligible_symbols\":{},\"symbols_with_trades\":{},\"candidates\":{},\"accepted_positions\":{},\"rejected_capacity\":{},\"rejected_cash\":{},\"rejected_zero_size\":{},\"shrunk_positions\":{},\"filtered_by_aux\":{},\"open_at_end\":{},\"stale_marks\":{},\"runtime_ms\":{},\"ranking\":{},\"timing_basis\":{},\"sizing_basis\":{},\"shrink_basis\":{},\"brokerage_basis\":{},\"short_basis\":{},\"aux_basis\":{},\"open_basis\":{}}}",
        q(req.basket.id), q(req.basket.name), q(req.basket.db), q(req.basket.rule_name), q(match gate { Gate::InLoop => "in_loop", Gate::EntryBar => "entry_bar", Gate::AflAux2 => "in_loop_afl_aux2" }), n(req.initial), q(req.size_mode), n(req.size_value), req.max_positions, n(req.brokerage), q(&dates[0]), q(dates.last().unwrap()),
        files.len(), collected.len(), symbols_with_trades, cands.len(), l.fills.len(), l.rejected_capacity, l.rejected_cash, l.rejected_zero_size, l.shrunk,
        if gate == Gate::EntryBar { aux_filtered.to_string() } else { "null".into() }, open_count, l.stale_marks, started.elapsed().as_millis(),
        q("No PositionScore in this AFL: AmiBroker's default order applies (AmiBroker KB 2015-09-28: larger PositionSize first, equal for all here; then alphabetical symbol order; then long before short)"),
        q("Each bar: exits first (at the AFL's exit price), then entries; MaxOpenPositions caps open positions and a rejected entry drops that whole trade (its later exit is ignored)"),
        q(&size_basis),
        q("Orders larger than the available cash are shrunk to the affordable whole shares including entry commission; an order of zero shares is rejected"),
        q(&format!("commissionmode 1: {}% of traded value on entry and on exit", req.brokerage)),
        q("Shorts reserve 100% of the entry value (plus commission) as margin; the reserve and the cover P&L return to cash on exit; marked as entry value + (entry - close) x shares"),
        q(&aux_basis),
        q("Positions open at the end, or whose symbol's data ended (DEAD STOCK BACKTEST off), stay open: marked at the last known close, no exit commission, shown as Open"));
    Ok(format!("{{\"kind\":\"basket\",\"mode\":\"amibroker\",\"status\":\"ready\",\"basket\":{},\"membership\":\"aux\",\"model_initial_equity\":{},\"dates\":[{}],\"equity\":{},\"equity_money\":{},\"cash\":{},\"positions\":[{}],\"bh\":{},\"benchmark_symbol\":{},\"close\":[],\"ohlc\":null,\"trades\":[{}],\"signals\":[{}],\"stats\":{},\"portfolio\":{},\"notices\":[{}],\"accounting_basis\":{}}}",
        q(req.basket.id), n(req.initial), dates.iter().map(|d| q(d)).collect::<Vec<_>>().join(","), arr(&l.equity.iter().map(|e| e / req.initial).collect::<Vec<_>>()), arr(&l.equity), arr(&l.cash), l.positions.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(","),
        arr(&bh), bench_symbol.map_or("null".into(), q), trades.join(","), signals.join(","), crate::stats_json(&stats), p, notices.iter().map(|s| q(s)).collect::<Vec<_>>().join(","),
        q("AmiBroker-style money account: initial equity, whole shares, chronological cash, daily closing marks. Net Profit = final equity - initial equity (open positions marked).")))
}

/// AmiBroker-style statistics in money and percent. Equity-based figures use daily closing equity.
pub(crate) fn stats(c: &[Cand], l: &Ledger, calendar: &[i64], initial: f64) -> BTreeMap<&'static str, f64> {
    let last = *l.equity.last().unwrap();
    let years = ((calendar[calendar.len() - 1] - calendar[0]) as f64 / 365.25).max(1. / 365.25);
    let (mut peak, mut dd, mut dd_money) = (initial, 0f64, 0f64);
    for &e in &l.equity { peak = peak.max(e); dd = dd.min(e / peak - 1.); dd_money = dd_money.min(e - peak); }
    let closed: Vec<&Filled> = l.fills.iter().filter(|f| !f.open).collect();
    let pnl: Vec<f64> = closed.iter().map(|f| f.pnl).collect();
    let pct: Vec<f64> = closed.iter().map(|f| f.pnl / (f.shares * c[f.cand].entry_px)).collect();
    let k = pnl.len() as f64;
    let wins: Vec<f64> = pnl.iter().copied().filter(|v| *v > 0.).collect();
    let losses: Vec<f64> = pnl.iter().copied().filter(|v| *v <= 0.).collect();
    let gross_profit: f64 = wins.iter().sum();
    let gross_loss: f64 = losses.iter().sum();
    let ret: Vec<f64> = std::iter::once(l.equity[0] / initial - 1.).chain(l.equity.windows(2).map(|w| w[1] / w[0] - 1.)).collect();
    let mean = ret.iter().sum::<f64>() / ret.len() as f64;
    let sd = (ret.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / (ret.len() - 1).max(1) as f64).sqrt();
    let car = (last / initial).powf(1. / years) - 1.;
    let exposure = l.positions.iter().filter(|&&p| p > 0).count() as f64 / l.positions.len() as f64;
    let avg = |v: &[f64]| if v.is_empty() { f64::NAN } else { v.iter().sum::<f64>() / v.len() as f64 };
    BTreeMap::from([
        ("initial_equity", initial),
        ("final_equity", last),
        ("net_profit_money", last - initial),
        ("net_profit", last / initial - 1.),
        ("total_return", last / initial - 1.),
        ("cagr", car),
        ("car", car),
        ("years", years),
        ("max_dd", dd),
        ("max_dd_money", dd_money),
        ("car_mdd", car / -dd),
        ("trades", l.fills.len() as f64),
        ("closed_trades", k),
        ("open_trades", (l.fills.len() - closed.len()) as f64),
        ("winners", wins.len() as f64),
        ("losers", losses.len() as f64),
        ("win_rate", wins.len() as f64 / k),
        ("avg_pnl_money", avg(&pnl)),
        ("avg_net_pnl", avg(&pct)),
        ("avg_win_money", avg(&wins)),
        ("avg_loss_money", avg(&losses)),
        ("expectancy_ratio", avg(&pnl) / -avg(&losses)),
        ("gross_profit", gross_profit),
        ("gross_loss", gross_loss),
        ("profit_factor", gross_profit / -gross_loss),
        ("closed_net_profit_money", pnl.iter().sum::<f64>()),
        ("fees_money", l.fills.iter().map(|f| f.entry_fee + f.exit_fee).sum::<f64>()),
        ("avg_bars", avg(&closed.iter().map(|f| c[f.cand].bars as f64).collect::<Vec<_>>())),
        ("sharpe", mean / sd * 252f64.sqrt()),
        ("exposure", exposure),
        ("rejected_capacity", l.rejected_capacity as f64),
        ("rejected_cash", l.rejected_cash as f64),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    fn cand(sym: &str, e: i64, x: i64, epx: f64, xpx: f64) -> Cand {
        Cand { symbol: sym.into(), dir: 1., entry_day: e, exit_day: x, entry_px: epx, exit_px: xpx, open: false, reason: "t".into(), bars: (x - e) as usize, marks: (e..=x).map(|d| (d, if d == x { xpx } else { epx })).collect() }
    }
    fn cal(n: i64) -> Vec<i64> { (0..n).collect() }
    fn set(size: Size, max: usize) -> Settings { Settings { initial: 1000., size, max_positions: max, fee: 0. } }
    fn near(a: f64, b: f64) { assert!((a - b).abs() < 1e-9, "{a} != {b}"); }

    #[test] fn pct_of_previous_bar_equity_whole_shares() {
        // A gains on day 1; B enters on day 2 sized from day 1's closing equity.
        let mut a = cand("A", 0, 3, 10., 20.); a.marks = vec![(0, 10.), (1, 20.), (2, 20.), (3, 20.)];
        let b = cand("B", 2, 3, 7., 7.);
        let l = ledger(&[a, b], &cal(4), set(Size::PctEquity(25.), 5)).unwrap();
        assert_eq!(l.fills[0].shares, 25.); // 25% of 1000 / 10
        near(l.equity[1], 1250.);
        assert_eq!(l.fills[1].shares, 44.); // floor(0.25 * 1250 / 7) = 44, not sized on day 2's own equity
        near(l.equity[3], 1250.);
    }
    #[test] fn shares_and_fixed_value_modes() {
        let l = ledger(&[cand("A", 0, 1, 10., 11.)], &cal(2), set(Size::Shares(7.), 5)).unwrap();
        assert_eq!(l.fills[0].shares, 7.); near(l.fills[0].pnl, 7.);
        let l = ledger(&[cand("A", 0, 1, 30., 33.)], &cal(2), set(Size::Value(100.), 5)).unwrap();
        assert_eq!(l.fills[0].shares, 3.); near(l.equity[1], 1009.);
        let l = ledger(&[cand("A", 0, 1, 300., 330.)], &cal(2), set(Size::Value(100.), 5)).unwrap();
        assert_eq!((l.fills.len(), l.rejected_zero_size), (0, 1));
    }
    #[test] fn shrink_on_cash_and_reject_when_nothing_affordable() {
        let src = [cand("A", 0, 2, 100., 100.), cand("B", 0, 2, 100., 100.), cand("C", 0, 2, 100., 100.)];
        let l = ledger(&src, &cal(3), set(Size::Shares(6.), 5)).unwrap();
        assert_eq!(l.fills.iter().map(|f| f.shares).collect::<Vec<_>>(), vec![6., 4.]);
        assert!(l.fills[1].shrunk); assert_eq!((l.shrunk, l.rejected_cash), (1, 1));
        near(l.cash[0], 0.);
    }
    #[test] fn cap_rejects_whole_trade_and_ignores_its_exit() {
        let src = [cand("A", 0, 2, 10., 10.), cand("B", 0, 1, 10., 50.), cand("C", 1, 2, 10., 10.)];
        let l = ledger(&src, &cal(3), set(Size::Value(100.), 1)).unwrap();
        assert_eq!(l.fills.iter().map(|f| c_sym(&src, f)).collect::<Vec<_>>(), vec!["A"]);
        assert_eq!(l.rejected_capacity, 2);
        near(l.equity[2], 1000.);
    }
    fn c_sym<'a>(src: &'a [Cand], f: &Filled) -> &'a str { &src[f.cand].symbol }
    #[test] fn exits_before_entries_on_the_same_bar() {
        let src = [cand("Z", 0, 1, 10., 12.), cand("A", 1, 2, 10., 10.)];
        let l = ledger(&src, &cal(3), set(Size::Value(100.), 1)).unwrap();
        assert_eq!(l.fills.len(), 2); assert_eq!(l.rejected_capacity, 0);
    }
    #[test] fn same_bar_round_trips_hold_a_slot_until_the_bar_ends() {
        let src = [cand("A", 0, 0, 10., 11.), cand("B", 0, 0, 10., 11.), cand("C", 0, 1, 10., 10.)];
        let l = ledger(&src, &cal(2), set(Size::Value(100.), 2)).unwrap();
        assert_eq!(l.fills.iter().map(|f| c_sym(&src, f)).collect::<Vec<_>>(), vec!["A", "B"]);
        assert_eq!(l.rejected_capacity, 1); near(l.equity[0], 1020.); assert_eq!(l.positions[0], 0);
    }
    #[test] fn ties_rank_alphabetically_case_insensitive_not_by_input_order() {
        let src = [cand("b", 0, 1, 10., 10.), cand("C", 0, 1, 10., 10.), cand("A", 0, 1, 10., 10.)];
        let l = ledger(&src, &cal(2), set(Size::Value(100.), 2)).unwrap();
        assert_eq!(l.fills.iter().map(|f| c_sym(&src, f)).collect::<Vec<_>>(), vec!["A", "b"]);
    }
    #[test] fn brokerage_each_side_on_traded_value() {
        let l = ledger(&[cand("A", 0, 1, 100., 110.)], &cal(2), Settings { initial: 10_000., size: Size::Shares(10.), max_positions: 1, fee: 0.01 }).unwrap();
        near(l.fills[0].entry_fee, 10.); near(l.fills[0].exit_fee, 11.); near(l.fills[0].pnl, 100. - 21.);
        near(l.equity[0], 10_000. - 10.); near(l.equity[1], 10_079.);
        // shrink accounts for commission: 1000 cash, price 100, fee 1% -> 9 shares
        let l = ledger(&[cand("A", 0, 1, 100., 100.)], &cal(2), Settings { initial: 1000., size: Size::Shares(10.), max_positions: 1, fee: 0.01 }).unwrap();
        assert_eq!(l.fills[0].shares, 9.);
    }
    #[test] fn short_reserves_full_margin_and_marks_inverse() {
        let mut s = cand("A", 0, 2, 100., 80.); s.dir = -1.; s.marks = vec![(0, 100.), (1, 120.), (2, 80.)];
        let l = ledger(&[s, cand("B", 0, 2, 100., 100.)], &cal(3), set(Size::Shares(10.), 5)).unwrap();
        assert_eq!(l.fills.len(), 1); assert_eq!(l.rejected_cash, 1); // margin used all cash
        near(l.cash[0], 0.); near(l.equity[1], 800.); near(l.equity[2], 1200.); near(l.fills[0].pnl, 200.);
    }
    #[test] fn open_positions_stay_marked_with_carried_close() {
        let mut a = cand("A", 0, 1, 10., 12.); a.open = true; a.marks = vec![(0, 10.), (1, 12.)];
        let l = ledger(&[a], &cal(4), set(Size::Shares(10.), 5)).unwrap();
        assert!(l.fills[0].open); near(l.equity[3], 1020.); near(l.fills[0].pnl, 20.); assert_eq!(l.stale_marks, 2);
    }
    #[test] fn reader_reads_declared_count_and_ignores_trailing_records() {
        let mut b = vec![0u8; HEADER + 3 * REC + 4];
        b[..8].copy_from_slice(b"BROKDAt5");
        b[HEADER - 4..HEADER].copy_from_slice(&2u32.to_le_bytes());
        for (k, (day, close, a1)) in [(2u64, 10f32, 1f32), (3, 11., 0.), (9, 99., 1.)].iter().enumerate() {
            let o = HEADER + REC * k;
            b[o..o + 8].copy_from_slice(&((2024u64 << 52) | (1u64 << 48) | (day << 43)).to_le_bytes());
            for (off, v) in [(8, *close), (12, *close), (16, *close + 1.), (20, *close - 1.), (24, 100.), (28, 5.), (32, *a1), (36, 0.)] { b[o + off..o + off + 4].copy_from_slice(&v.to_le_bytes()); }
        }
        let r = read_native(&b).unwrap();
        assert_eq!(r.len(), 2); assert_eq!(r[1].date, (2024, 1, 3)); assert_eq!(r[0].aux1, 1.);
        b[HEADER - 4..HEADER].copy_from_slice(&9u32.to_le_bytes());
        assert!(read_native(&b).is_err());
        assert!(read_native(b"nope").is_err());
    }
    #[test] fn hygiene_drops_bad_and_duplicate_bars_and_widens_range() {
        let bar = |d: u8, o: f32, h: f32, l: f32, c: f32| RawBar { date: (2024, 1, d), c, o, h, l, v: -5., oi: 0., aux1: d as f32, aux2: 0. };
        let (rows, aux) = clean("X", &[bar(3, 10., 10., 9., 12.), bar(2, 10., 11., 9., 10.), bar(2, 50., 51., 49., 50.), bar(4, 0., 1., 1., 1.)]);
        assert_eq!(rows.iter().map(|r| r.date.as_str()).collect::<Vec<_>>(), vec!["2024-01-02", "2024-01-03"]);
        assert_eq!(rows[0].close, 10.); assert_eq!(rows[1].high, 12.); assert_eq!(rows[1].volume, Some(0.)); assert_eq!(aux[1].aux1, 3.);
    }
    #[test] fn aux_gate_at_entry_bar_and_in_loop_for_nr7() {
        let rows = crate::books::ytnr7::tests::wave(420);
        let def = crate::connors::catalogue().into_iter().find(|d| d.id == crate::books::ytnr7::NR7_ST).unwrap();
        let make = ctx_builder(|r| Ctx::new(r, &[], &[]));
        let all: Vec<Aux> = rows.iter().map(|_| Aux { aux1: 1., aux2: 0., open_interest: 0. }).collect();
        let (lo, hi) = (crate::day_number(&rows[0].date), crate::day_number(&rows[rows.len() - 1].date));
        let full = symbol_trades(def.id, &def, "ACTUAL", &rows, &all, Rule::Aux1, lo, hi, &Params::new(), &make);
        let expected = crate::books::ytnr7::run(def.id, &Ctx::new(&rows, &[], &[]), 0, rows.len() - 1, &Params::new()).unwrap();
        assert!(!full.cands.is_empty()); assert_eq!(full.cands.len(), expected.len());
        // Ineligible on the first trade's entry bar: that entry is gated in the loop.
        let first = full.cands[0].entry_day;
        let gated: Vec<Aux> = rows.iter().map(|r| Aux { aux1: if crate::day_number(&r.date) == first { 0. } else { 1. }, aux2: 0., open_interest: 0. }).collect();
        let g = symbol_trades(def.id, &def, "ACTUAL", &rows, &gated, Rule::Aux1, lo, hi, &Params::new(), &make);
        assert!(g.cands.iter().all(|c| c.entry_day != first));
        // Never eligible: no run and no calendar contribution.
        let none: Vec<Aux> = rows.iter().map(|_| Aux { aux1: 0., aux2: 1., open_interest: 0. }).collect();
        let z = symbol_trades(def.id, &def, "ACTUAL", &rows, &none, Rule::Aux1, lo, hi, &Params::new(), &make);
        assert!(z.cands.is_empty() && z.days.is_empty());
        assert_eq!(gate_of(def.id), Gate::InLoop);
    }
    #[test] fn request_parsing_and_admission() {
        let q = |kv: &[(&str, &str)]| kv.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect::<BTreeMap<_, _>>();
        let r = parse(&q(&[("id", "x"), ("basket", "nse500_cp")])).unwrap();
        assert_eq!((r.initial, r.size, r.max_positions, r.brokerage), (1_000_000., Size::PctEquity(5.), 20, 0.));
        assert!(parse(&q(&[("id", "x"), ("basket", "nse500")])).is_err());
        assert!(parse(&q(&[("id", "x"), ("basket", "fo_cp"), ("size_mode", "shares"), ("size_value", "1.5")])).is_err());
        assert_eq!(parse(&q(&[("id", "x"), ("basket", "fo_cp"), ("size_mode", "value"), ("size_value", "100000")])).unwrap().size, Size::Value(100000.));
        let cat = crate::connors::catalogue();
        let nr7 = cat.iter().find(|d| d.id == crate::books::ytnr7::NR7_ST).unwrap();
        assert!(admission(nr7).is_ok());
        let admitted = cat.iter().filter(|d| d.book.starts_with("yt_") && admission(d).is_ok()).count();
        assert!(admitted > 10, "{admitted}");
        for d in cat.iter().filter(|d| crate::instrument_policy::intraday_rule(d)) { assert!(admission(d).is_err()); }
        if let Some(mr) = cat.iter().find(|d| d.id == crate::books::ytmr::ID) { assert!(admission(mr).is_err()); }
        assert_eq!(civil(crate::day_number("2024-02-29")), "2024-02-29");
    }
    #[test] fn tripple_rsi_fo_reads_aux2_in_loop() {
        let id = crate::books::yttr::ATHC_FO;
        let def = crate::connors::catalogue().into_iter().find(|d| d.id == id).unwrap();
        assert!(admission(&def).is_ok());
        assert_eq!(gate_of(id), Gate::AflAux2);
        let v: Vec<_> = (0..900).map(|k| { let k = k as f64; let c = 100.0 + 0.4 * k + 12.0 * (k / 6.0).sin(); (c, c + 1.0, c - 1.0, c) }).collect();
        let rows = crate::books::ytwh::testbars::bars(&v);
        let make = ctx_builder(|r| Ctx::new(r, &[], &[]));
        let (lo, hi) = (crate::day_number(&rows[0].date), crate::day_number(&rows[rows.len() - 1].date));
        let p: Params = [("rsi_trail".to_string(), 20.)].into_iter().collect();
        let fo: Vec<Aux> = rows.iter().map(|_| Aux { aux1: 0., aux2: 1., open_interest: 0. }).collect();
        let full = symbol_trades(id, &def, "ACTUAL", &rows, &fo, Rule::Aux2, lo, hi, &p, &make);
        assert!(full.error.is_none() && full.cands.len() >= 2, "{:?} {}", full.error, full.cands.len());
        // the AFL's own Aux2[i]==1 blocks the first entry even under a basket rule that allows it
        let first = full.cands[0].entry_day;
        let gated: Vec<Aux> = rows.iter().map(|r| Aux { aux1: 1., aux2: if crate::day_number(&r.date) == first { 0. } else { 1. }, open_interest: 0. }).collect();
        let g = symbol_trades(id, &def, "ACTUAL", &rows, &gated, Rule::Aux1, lo, hi, &p, &make);
        assert!(g.error.is_none() && !g.cands.is_empty() && g.cands.iter().all(|c| c.entry_day != first));
        assert_eq!(g.aux_filtered, 0);
    }
    /// Real-data check of the Tripple RSI (FO) AFLs on F&O stocks of cp_updated_2026 (skipped when absent).
    #[test] fn tripple_rsi_fo_on_real_native_stocks() {
        let db = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../jay-local/Data/amibroker/cp_updated_2026"));
        if !db.is_dir() { eprintln!("skipped: {} absent", db.display()); return; }
        let pick = ["RELIANCE", "SBIN"];
        let files: Vec<_> = universe(db).unwrap().into_iter().filter(|(s, _)| pick.contains(&s.as_str())).collect();
        assert_eq!(files.len(), 2);
        let make = ctx_builder(|r| Ctx::new(r, &[], &[]));
        for id in crate::books::yttr::FO_IDS {
            let def = crate::connors::catalogue().into_iter().find(|d| d.id == id).unwrap();
            for (label, p) in [("defaults", Params::new()), ("rsi_trail 20, hard SL on", [("rsi_trail".to_string(), 20.), ("hard_sl".to_string(), 1.)].into_iter().collect::<Params>())] {
                let got = collect(&files, id, &def, Rule::Aux2, i64::MIN, i64::MAX, &p, &make, Instant::now() + Duration::from_secs(60)).unwrap();
                for r in &got {
                    let closed = r.cands.iter().filter(|c| !c.open).count();
                    let hsl = r.cands.iter().filter(|c| c.reason == "Hard SL").count();
                    eprintln!("{id} {label} {}: {} trades ({closed} closed, {hsl} hard SL), {} dates", r.symbol, r.cands.len(), r.days.len());
                    assert!(r.cands.iter().all(|c| c.entry_px > 0. && c.exit_day >= c.entry_day));
                }
                assert!(got.iter().any(|r| !r.cands.is_empty()), "{id} {label}");
            }
        }
    }
    /// Real-data check on a small sample of cp_updated_2026 (skipped when the local copy is absent).
    #[test] fn nr7_on_real_native_sample() {
        let db = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../jay-local/Data/amibroker/cp_updated_2026"));
        if !db.is_dir() { eprintln!("skipped: {} absent", db.display()); return; }
        let all = universe(db).unwrap();
        assert!(all.len() > 3000);
        let pick = ["RELIANCE", "TCS", "INFY", "SBIN", "AARTIIND"];
        let files: Vec<_> = all.into_iter().filter(|(s, _)| pick.contains(&s.as_str())).collect();
        assert_eq!(files.len(), 5);
        let def = crate::connors::catalogue().into_iter().find(|d| d.id == crate::books::ytnr7::NR7_ST).unwrap();
        let make = ctx_builder(|r| Ctx::new(r, &[], &[]));
        let (f, t) = (crate::day_number("2018-01-01"), crate::day_number("2020-12-31"));
        let got = collect(&files, def.id, &def, Rule::Aux1, f, t, &Params::new(), &make, Instant::now() + Duration::from_secs(60)).unwrap();
        let mut cands: Vec<Cand> = got.iter().flat_map(|r| r.cands.clone()).collect();
        assert!(!cands.is_empty());
        cands.sort_by(|a, b| a.entry_day.cmp(&b.entry_day).then_with(|| a.symbol.cmp(&b.symbol)));
        let calendar: Vec<i64> = got.iter().flat_map(|r| r.days.clone()).collect::<BTreeSet<_>>().into_iter().collect();
        let l = ledger(&cands, &calendar, Settings { initial: 1_000_000., size: Size::PctEquity(5.), max_positions: 20, fee: 0. }).unwrap();
        assert_eq!(l.fills.len() + l.rejected_capacity + l.rejected_cash + l.rejected_zero_size, cands.len());
        for fl in &l.fills { assert!(fl.shares >= 1. && fl.shares.fract() == 0.); }
        assert!(l.equity.iter().all(|e| e.is_finite() && *e > 0.));
    }
}
