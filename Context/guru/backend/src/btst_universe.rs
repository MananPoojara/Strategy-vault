//! BTST on the F&O stock universe (Phase 2): an on-demand reader of the per-symbol `.bt5` 5-minute files, per-symbol
//! candidate generation with the unchanged single-symbol BTST rules (`books::ytbtst::universe_fills`) and a cross-sectional
//! portfolio executor that follows AmiBroker's portfolio backtester as settled from the team's trade lists
//! (`BTST/BTST_Phase2_Portfolio_Findings.md`): PositionScore ranking at the entry bar (equal scores in ascending symbol
//! name), exits before entries on a bar, `max_positions` slots, size = floor(pct% x previous-bar equity / price) whole
//! shares with equity compounding (open positions marked at the previous bar's close), shrink to the cash available
//! (skip below 1 share), signals on a symbol's last data day never entered, commission/slippage per side from the
//! Vault's cost settings. Since 19 Nov 2026 with AmiBroker's full arithmetic, as every ATH variant (`simulate`).
//!
//! Memory: the universe (~1.1 GB on disk) is never held; each worker reads one file, builds that symbol's series,
//! keeps only its candidate trades, and drops the rest. One run at a time (`Permit`), bounded threads and time.
use crate::ath_core;
use crate::books::ytbtst;
use crate::connors::{Ctx, Params};
use crate::Row;
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

static RUNNING: AtomicBool = AtomicBool::new(false);
const TIME_LIMIT: Duration = Duration::from_secs(100);
const WORKERS: usize = 2;

struct Permit;
impl Permit {
    fn take() -> Result<Self, String> {
        RUNNING.compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire).map(|_| Self).map_err(|_| "busy: A universe backtest is already running; retry when it finishes.".into())
    }
}
impl Drop for Permit {
    fn drop(&mut self) {
        RUNNING.store(false, Ordering::Release);
    }
}

/// Directory of the `.bt5` files and `universe.json` (read only): src 1 = IEOD as stored, src 3 = the same series
/// rebuilt with the minutes the team's 2021/22 copy did not have (scripts/btst_stock_series_team.py).
pub fn data_dir_for(src: i64) -> String {
    if src == 3 {
        std::env::var("VAULT_BTST_STOCKS_TEAM_DIR").unwrap_or_else(|_| "/data/btst-stocks-team".into())
    } else {
        std::env::var("VAULT_BTST_STOCKS_DIR").unwrap_or_else(|_| "/data/btst-stocks".into())
    }
}
pub fn data_dir() -> String {
    data_dir_for(1)
}

// ------------------------------------------------------------------ .bt5 reader

/// One day of a `.bt5` file: yyyymmdd, the day's OHLCV from all minutes, and its bars `first..first + n`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Day5 {
    pub date: i32,
    pub o: f64,
    pub h: f64,
    pub l: f64,
    pub c: f64,
    pub v: f64,
    pub first: usize,
    pub n: usize,
}
/// One start-stamped 5-minute bar.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bar5 {
    pub t: u16,
    pub o: f64,
    pub h: f64,
    pub l: f64,
    pub c: f64,
    pub v: f64,
}
pub struct Bt5 {
    pub name: String,
    pub days: Vec<Day5>,
    pub bars: Vec<Bar5>,
}

/// Parse a `.bt5` file (spec section 1): magic `BT5STK01`, u32 n_days, u32 n_bars, u16 name_len, name; days 32 bytes
/// (i32 yyyymmdd, f32 DO DH DL DC DV, u32 NB, u32 first_bar); bars 22 bytes (u16 hhmm, f32 O H L C V). Little-endian.
pub fn parse_bt5(b: &[u8]) -> Result<Bt5, String> {
    let bad = |w: &str| format!("bad .bt5 file: {w}");
    if b.len() < 18 || &b[..8] != b"BT5STK01" {
        return Err(bad("magic"));
    }
    let u32_at = |o: usize| u32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]]);
    let f32_at = |o: usize| f64::from(f32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]]));
    let (nd, nb) = (u32_at(8) as usize, u32_at(12) as usize);
    let nl = u16::from_le_bytes([b[16], b[17]]) as usize;
    let d0 = 18 + nl;
    let b0 = d0 + 32 * nd;
    if b.len() != b0 + 22 * nb {
        return Err(bad("length"));
    }
    let name = String::from_utf8(b[18..d0].to_vec()).map_err(|_| bad("name"))?;
    let bars: Vec<Bar5> = (0..nb)
        .map(|k| {
            let o = b0 + 22 * k;
            Bar5 { t: u16::from_le_bytes([b[o], b[o + 1]]), o: f32_at(o + 2), h: f32_at(o + 6), l: f32_at(o + 10), c: f32_at(o + 14), v: f32_at(o + 18) }
        })
        .collect();
    let mut days: Vec<Day5> = (0..nd)
        .map(|k| {
            let o = d0 + 32 * k;
            Day5 { date: u32_at(o) as i32, o: f32_at(o + 4), h: f32_at(o + 8), l: f32_at(o + 12), c: f32_at(o + 16), v: f32_at(o + 20), first: u32_at(o + 28) as usize, n: 0 }
        })
        .collect();
    for k in 0..nd {
        let end = if k + 1 < nd { days[k + 1].first } else { nb };
        if end < days[k].first || end > nb {
            return Err(bad("day table"));
        }
        days[k].n = end - days[k].first;
    }
    Ok(Bt5 { name, days, bars })
}

fn iso(date: i32) -> String {
    format!("{:04}-{:02}-{:02}", date / 10000, date / 100 % 100, date % 100)
}

/// AmiBroker "pad and align to reference symbol": from the symbol's first to its last bar the bar set becomes the
/// reference's; a reference bar the symbol lacks is a copy of the previous close (O = H = L = C, V 0); the symbol's bars
/// at times the reference lacks are dropped; the day's OHLCV is compressed from the aligned bars.
pub fn pad_align(s: &Bt5, r: &Bt5) -> Bt5 {
    let (Some(fd), Some(ld)) = (s.days.first(), s.days.last()) else { return Bt5 { name: s.name.clone(), days: vec![], bars: vec![] } };
    let first = (fd.date, s.bars[fd.first].t);
    let last = (ld.date, s.bars[ld.first + ld.n - 1].t);
    let own: BTreeMap<(i32, u16), Bar5> = s.days.iter().flat_map(|d| s.bars[d.first..d.first + d.n].iter().map(move |b| ((d.date, b.t), *b))).collect();
    let (mut days, mut bars) = (Vec::new(), Vec::new());
    let mut prev = f64::NAN;
    for d in &r.days {
        let start = bars.len();
        for rb in &r.bars[d.first..d.first + d.n] {
            let key = (d.date, rb.t);
            if key < first || key > last {
                continue;
            }
            let b = own.get(&key).copied().unwrap_or(Bar5 { t: rb.t, o: prev, h: prev, l: prev, c: prev, v: 0.0 });
            if !b.c.is_finite() {
                continue;
            }
            prev = b.c;
            bars.push(b);
        }
        if bars.len() > start {
            let day = &bars[start..];
            days.push(Day5 {
                date: d.date,
                o: day[0].o,
                h: day.iter().map(|b| b.h).fold(f64::MIN, f64::max),
                l: day.iter().map(|b| b.l).fold(f64::MAX, f64::min),
                c: day[day.len() - 1].c,
                v: day.iter().map(|b| b.v).sum(),
                first: start,
                n: bars.len() - start,
            });
        }
    }
    Bt5 { name: s.name.clone(), days, bars }
}

/// NIFTY futures as a lookup on (date, hhmm): the close of the latest NIFTY bar at or before a time (AmiBroker Foreign,
/// aligned to the current symbol's timeline).
pub struct Nifty {
    keys: Vec<i64>,
    closes: Vec<f64>,
    days: Vec<(i32, f64)>,
}
impl Nifty {
    pub fn new(n: &Bt5) -> Self {
        let mut keys = Vec::with_capacity(n.bars.len());
        let mut closes = Vec::with_capacity(n.bars.len());
        for d in &n.days {
            for b in &n.bars[d.first..d.first + d.n] {
                keys.push(i64::from(d.date) * 10_000 + i64::from(b.t));
                closes.push(b.c);
            }
        }
        Nifty { keys, closes, days: n.days.iter().map(|d| (d.date, d.c)).collect() }
    }
    /// NIFTY's own daily close of the latest NIFTY day at or before `date` (daily timeframe, aligned by date).
    pub fn daily(&self, date: i32) -> f64 {
        match self.days.partition_point(|d| d.0 <= date) {
            0 => f64::NAN,
            p => self.days[p - 1].1,
        }
    }
    pub fn at(&self, date: i32, t: u16) -> f64 {
        let k = i64::from(date) * 10_000 + i64::from(t);
        match self.keys.partition_point(|&x| x <= k) {
            0 => f64::NAN,
            p => self.closes[p - 1],
        }
    }
}

/// The symbol's series in the engine's single-symbol form (rows = its days, extra = `BTST_5M.<O|H|L|C|V><HHMM>` and
/// `BTST_5M.D*`), plus the aligned NIFTY series when `nifty` is given: (NIFTY, entry time, own daily close). With
/// `own daily close` the NIFTY daily close is NIFTY's own (the incremental AFL sets the daily timeframe before
/// SetForeign, so NIFTY is compressed by date); otherwise it is NIFTY's close at the stock's last bar of the day (the
/// one-up-close AFL aligns NIFTY to the stock's intraday timeline first, then compresses).
pub fn rows_and_extra(s: &Bt5, nifty: Option<(&Nifty, u16, bool)>) -> (Vec<Row>, BTreeMap<String, Vec<f64>>) {
    let n = s.days.len();
    let rows: Vec<Row> = s.days.iter().map(|d| Row { ticker: s.name.clone(), date: iso(d.date), open: d.o, high: d.h, low: d.l, close: d.c, volume: Some(d.v) }).collect();
    let mut cols: BTreeMap<u16, [Vec<f64>; 5]> = BTreeMap::new();
    for (k, d) in s.days.iter().enumerate() {
        for b in &s.bars[d.first..d.first + d.n] {
            let e = cols.entry(b.t).or_insert_with(|| std::array::from_fn(|_| vec![f64::NAN; n]));
            e[0][k] = b.o;
            e[1][k] = b.h;
            e[2][k] = b.l;
            e[3][k] = b.c;
            e[4][k] = b.v;
        }
    }
    let mut extra = BTreeMap::new();
    for (t, [o, h, l, c, v]) in cols {
        for (key, col) in [("O", o), ("H", h), ("L", l), ("C", c), ("V", v)] {
            extra.insert(format!("BTST_5M.{key}{t:04}"), col);
        }
    }
    for (key, f) in [("DO", 0usize), ("DH", 1), ("DL", 2), ("DC", 3), ("DV", 4)] {
        extra.insert(format!("BTST_5M.{key}"), s.days.iter().map(|d| [d.o, d.h, d.l, d.c, d.v][f]).collect());
    }
    if let Some((nf, entry_t, own)) = nifty {
        extra.insert("NIFTYREF.DC".into(), s.days.iter().map(|d| if own { nf.daily(d.date) } else if d.n > 0 { nf.at(d.date, s.bars[d.first + d.n - 1].t) } else { f64::NAN }).collect());
        extra.insert("NIFTYREF.E".into(), s.days.iter().map(|d| nf.at(d.date, entry_t)).collect());
    }
    (rows, extra)
}

// ------------------------------------------------------------------ universe list

/// A minimal JSON value (universe.json only).
#[derive(Debug, Clone, PartialEq)]
enum J {
    S(String),
    N(f64),
    A(Vec<J>),
    O(Vec<(String, J)>),
    Lit,
}
fn json(s: &str) -> Option<J> {
    fn ws(b: &[u8], i: &mut usize) {
        while *i < b.len() && b[*i].is_ascii_whitespace() {
            *i += 1;
        }
    }
    fn val(b: &[u8], i: &mut usize) -> Option<J> {
        ws(b, i);
        match *b.get(*i)? {
            b'"' => {
                *i += 1;
                let mut out = Vec::new();
                while *b.get(*i)? != b'"' {
                    if b[*i] == b'\\' {
                        *i += 1;
                        match *b.get(*i)? {
                            b'u' => {
                                let hex = std::str::from_utf8(b.get(*i + 1..*i + 5)?).ok()?;
                                let c = char::from_u32(u32::from_str_radix(hex, 16).ok()?)?;
                                out.extend(c.to_string().bytes());
                                *i += 4;
                            }
                            b'n' => out.push(b'\n'),
                            b't' => out.push(b'\t'),
                            c => out.push(c),
                        }
                    } else {
                        out.push(b[*i]);
                    }
                    *i += 1;
                }
                *i += 1;
                Some(J::S(String::from_utf8(out).ok()?))
            }
            b'[' => {
                *i += 1;
                let mut v = Vec::new();
                loop {
                    ws(b, i);
                    if *b.get(*i)? == b']' {
                        *i += 1;
                        return Some(J::A(v));
                    }
                    v.push(val(b, i)?);
                    ws(b, i);
                    if *b.get(*i)? == b',' {
                        *i += 1;
                    }
                }
            }
            b'{' => {
                *i += 1;
                let mut v = Vec::new();
                loop {
                    ws(b, i);
                    if *b.get(*i)? == b'}' {
                        *i += 1;
                        return Some(J::O(v));
                    }
                    let J::S(k) = val(b, i)? else { return None };
                    ws(b, i);
                    if *b.get(*i)? != b':' {
                        return None;
                    }
                    *i += 1;
                    v.push((k, val(b, i)?));
                    ws(b, i);
                    if *b.get(*i)? == b',' {
                        *i += 1;
                    }
                }
            }
            c if c == b'-' || c.is_ascii_digit() => {
                let st = *i;
                while *i < b.len() && (b[*i].is_ascii_digit() || b"+-.eE".contains(&b[*i])) {
                    *i += 1;
                }
                std::str::from_utf8(&b[st..*i]).ok()?.parse().ok().map(J::N)
            }
            _ => {
                while *i < b.len() && b[*i].is_ascii_alphabetic() {
                    *i += 1;
                }
                Some(J::Lit)
            }
        }
    }
    let b = s.as_bytes();
    let mut i = 0;
    val(b, &mut i)
}
fn get<'a>(o: &'a J, key: &str) -> Option<&'a J> {
    match o {
        J::O(v) => v.iter().find(|(k, _)| k == key).map(|(_, v)| v),
        _ => None,
    }
}

/// One universe member: symbol, file, first and last date.
#[derive(Clone, Debug, PartialEq)]
pub struct Member {
    pub symbol: String,
    pub file: String,
    pub first: String,
    pub last: String,
}

/// The symbol names of `watchlist` in universe.json.
fn members_of(universe_json: &str, watchlist: &str) -> Result<Vec<String>, String> {
    Ok(members(universe_json, watchlist)?.into_iter().map(|m| m.symbol).collect())
}

/// The page's baskets of the F&O stock universe: "All F&O" = FO STOCKS C&P (current and past) / CURRENT F&O STOCKS.
fn baskets_json(universe_json: &str) -> String {
    let cp = members_of(universe_json, ytbtst::UNIVERSE).unwrap_or_default();
    let cur = members_of(universe_json, "CURRENT F&O STOCKS").unwrap_or_default();
    let past = cp.iter().filter(|n| !cur.contains(n)).count();
    format!("[{{\"key\":\"allfo\",\"label\":\"All F&O\",\"cp\":\"{}\",\"current\":\"{}\",\"n_cp\":{},\"n_current\":{},\"n_past\":{past}}}]",
        crate::esc(ytbtst::UNIVERSE), if cur.is_empty() { "" } else { "CURRENT F&O STOCKS" }, cp.len(), cur.len())
}

/// The members of `watchlist` listed in universe.json, in ascending symbol name.
pub fn members(universe_json: &str, watchlist: &str) -> Result<Vec<Member>, String> {
    let root = json(universe_json).ok_or("universe.json is not valid JSON")?;
    let Some(J::A(stocks)) = get(&root, "stocks") else { return Err("universe.json has no stocks list".into()) };
    let s = |o: &J, k: &str| match get(o, k) {
        Some(J::S(v)) => Some(v.clone()),
        _ => None,
    };
    let mut out: Vec<Member> = stocks
        .iter()
        .filter(|o| matches!(get(o, "watchlists"), Some(J::A(w)) if w.iter().any(|x| *x == J::S(watchlist.into()))))
        .filter_map(|o| Some(Member { symbol: s(o, "symbol")?, file: s(o, "file")?, first: s(o, "first").unwrap_or_default(), last: s(o, "last").unwrap_or_default() }))
        .filter(|m| !m.file.contains('/') && !m.file.contains("..") && m.file.ends_with(".bt5"))
        .collect();
    out.sort_by(|a, b| a.symbol.cmp(&b.symbol));
    Ok(out)
}

// ------------------------------------------------------------------ candidates

/// One candidate trade of one symbol (dates yyyymmdd, times HHMM).
#[derive(Clone, Debug)]
pub struct Cand {
    pub sym: usize,
    pub entry_day: i32,
    pub entry_t: u16,
    pub entry_px: f64,
    pub exit_day: i32,
    pub exit_t: u16,
    pub exit_px: f64,
    pub open: bool,
    pub reason: String,
    pub score: f64,
    /// Day-end closes while held, and closes before the entry time on later days still held at it.
    pub day_marks: Vec<(i32, f64)>,
    pub t_marks: Vec<(i32, f64)>,
}

/// The candidates of one symbol: the single-symbol trades of `id`, minus signals on the symbol's last data day (never
/// entered by AmiBroker, no slot taken). Returns (candidates, dropped last-day signals).
pub fn symbol_candidates(id: &str, s: &Bt5, sym: usize, p: &Params, nifty: Option<&Nifty>) -> (Vec<Cand>, usize) {
    if s.days.is_empty() {
        return (Vec::new(), 0);
    }
    let entry_t = p.get("entry_time").map_or(1450, |v| v.round() as u16);
    let own = id == "ytbtst_nifty_incremental";
    let (rows, extra) = rows_and_extra(s, nifty.map(|n| (n, entry_t, own)));
    let mut x = Ctx::new(&rows, &rows, &[]);
    x.extra = extra;
    let mut q = p.clone();
    q.insert("list_rank".into(), sym as f64);
    q.insert("src".into(), 1.0);
    let Some(fills) = ytbtst::universe_fills(id, &x, &q) else { return (Vec::new(), 0) };
    let date = |i: usize| s.days[i].date;
    let last_day = s.days[s.days.len() - 1].date;
    let mut dropped = 0;
    let mut out = Vec::new();
    for f in fills {
        let entry_day = date(f.trade.legs[0].i);
        if entry_day == last_day {
            dropped += 1;
            continue;
        }
        out.push(Cand {
            sym,
            entry_day,
            entry_t: f.entry_t as u16,
            entry_px: f.trade.legs[0].px,
            exit_day: if f.trade.open { i32::MAX } else { date(f.trade.exit_i) },
            exit_t: f.exit_t as u16,
            exit_px: f.trade.exit_px,
            open: f.trade.open,
            reason: f.trade.reason.clone(),
            score: f.score,
            day_marks: f.day_marks.iter().map(|&(i, c)| (date(i), c)).collect(),
            t_marks: f.t_marks.iter().map(|&(i, c)| (date(i), c)).collect(),
        });
    }
    (out, dropped)
}

// ------------------------------------------------------------------ portfolio

/// Portfolio settings: slots, size % (of previous-bar equity), cost rate per side, initial equity, window.
#[derive(Clone, Copy, Debug)]
pub struct Book {
    pub max_positions: usize,
    pub pct: f64,
    /// Fixed amount per position (sizing 2: shares = floor(alloc / price) on every trade); None = pct of
    /// previous-bar equity, compounding (sizing 1).
    pub alloc: Option<f64>,
    pub cost: f64,
    pub equity: f64,
    pub from: i32,
    pub to: i32,
}

/// One accepted trade.
#[derive(Clone, Debug, PartialEq)]
pub struct Taken {
    pub cand: usize,
    pub shares: f64,
    pub weight: f64,
    pub open: bool,
    pub exit_day: i32,
    pub exit_t: u16,
    pub exit_px: f64,
    /// AmiBroker's trade profit (float32; an open trade at its last mark)
    pub profit: f64,
}

pub struct Book2 {
    pub taken: Vec<Taken>,
    pub dates: Vec<i32>,
    pub equity: Vec<f64>,
    pub positions: Vec<usize>,
    pub rejected_capacity: usize,
    pub rejected_cash: usize,
}

struct Pos {
    cand: usize,
    shares: f64,
    taken: usize,
}

/// The position's mark at the bar before (date, t) on a later day: the stored close before the entry time on that day,
/// else the latest day-end close before that day, else the entry price.
fn mark_before(c: &Cand, date: i32) -> f64 {
    if let Some(&(_, v)) = c.t_marks.iter().find(|m| m.0 == date) {
        return v;
    }
    c.day_marks.iter().filter(|m| m.0 < date).last().map_or(c.entry_px, |m| m.1)
}
/// The position's day-end mark on `date` (latest stored close at or before it, else the entry price).
fn mark_day(c: &Cand, date: i32) -> f64 {
    c.day_marks.iter().filter(|m| m.0 <= date).last().map_or(c.entry_px, |m| m.1)
}

/// AmiBroker's profit before it is stored as a float32: shares x the change of the AmiBroker prices, less the commission on the float32
/// entry and exit values (the ATH full set, `ath_core`; team BTST lists: 56,475 of 56,478 profits on the database prices, 455 / 456 NIFTY).
fn ab_profit_raw(sh: f64, e: f64, x: f64, c: f64) -> f64 {
    sh * (ath_core::ab_full_price(x) - ath_core::ab_full_price(e)) - (ath_core::ab_value(sh, e) + ath_core::ab_value(sh, x)) * c
}

/// AmiBroker's value of an open long position at `m`: its float32 entry value plus the unrealised profit on AmiBroker prices, less the
/// commission to close it on the float32 value at `m` (`ath_core::liquidation_mark`).
fn ab_mark(sh: f64, e: f64, m: f64, c: f64) -> f64 {
    ath_core::ab_value(sh, e) + sh * (ath_core::ab_full_price(m) - ath_core::ab_full_price(e)) - c * ath_core::ab_value(sh, m)
}

/// Replay the candidates through the portfolio on the calendar `dates` (yyyymmdd, ascending). `names` = symbol names
/// (tie order). Entries are grouped by (day, entry time); exits at a bar happen before entries at it.
/// Arithmetic: AmiBroker's full set, as for every ATH variant (`Verification.md`; 19 Nov 2026, replay of the 20 team BTST lists,
/// 549,288 trades: equal shares 541,246 -> 546,611). Prices are AmiBroker's float32 arrays, the commission rate a float32 %.
pub fn simulate(cands: &[Cand], names: &[String], dates: &[i32], book: Book) -> Book2 {
    let c = ((book.cost * 100.0) as f32) as f64 / 100.0;
    let (ep, xp) = (|cd: &Cand| cd.entry_px as f32 as f64, |cd: &Cand| cd.exit_px as f32 as f64);
    let mut by_bar: BTreeMap<(i32, u16), Vec<usize>> = BTreeMap::new();
    for (k, cd) in cands.iter().enumerate() {
        if cd.entry_day >= book.from && cd.entry_day <= book.to {
            by_bar.entry((cd.entry_day, cd.entry_t)).or_default().push(k);
        }
    }
    for v in by_bar.values_mut() {
        v.sort_by(|&a, &b| cands[b].score.partial_cmp(&cands[a].score).unwrap_or(std::cmp::Ordering::Equal).then_with(|| names[cands[a].sym].cmp(&names[cands[b].sym])));
    }
    let mut cash = book.equity;
    let mut open: Vec<Pos> = Vec::new();
    let mut taken: Vec<Taken> = Vec::new();
    let (mut rejected_capacity, mut rejected_cash) = (0, 0);
    let exit_key = |cd: &Cand| if cd.open || cd.exit_day > book.to { (i32::MAX, u16::MAX) } else { (cd.exit_day, cd.exit_t) };
    // close every open position whose exit is at or before `key` (strictly before when `strict`)
    let close_until = |open: &mut Vec<Pos>, cash: &mut f64, taken: &mut Vec<Taken>, key: (i32, u16), strict: bool| {
        let mut due: Vec<usize> = (0..open.len()).filter(|&k| { let e = exit_key(&cands[open[k].cand]); if strict { e < key } else { e <= key } }).collect();
        due.sort_by_key(|&k| exit_key(&cands[open[k].cand]));
        for &k in &due {
            let cd = &cands[open[k].cand];
            // the entry cost back plus the unrounded profit; the trade reports the float32 profit
            let (sh, raw) = (open[k].shares, ab_profit_raw(open[k].shares, ep(cd), xp(cd), c));
            *cash += ath_core::ab_cost(sh, ep(cd), c) + raw;
            let t = &mut taken[open[k].taken];
            t.profit = raw as f32 as f64;
            t.open = false;
            t.exit_day = cd.exit_day;
            t.exit_t = cd.exit_t;
            t.exit_px = cd.exit_px;
        }
        let mut k = 0;
        open.retain(|_| { let keep = !due.contains(&k); k += 1; keep });
    };
    let mut bars = by_bar.iter().peekable();
    let (mut eq_dates, mut equity, mut positions) = (Vec::new(), Vec::new(), Vec::new());
    for &day in dates.iter().filter(|&&d| d >= book.from && d <= book.to) {
        while let Some((&(bd, bt), list)) = bars.peek().map(|(k, v)| (*k, *v)) {
            if bd > day {
                break;
            }
            bars.next();
            close_until(&mut open, &mut cash, &mut taken, (bd, bt), true);
            // previous-bar equity (unrounded): cash + positions still open at the previous bar, at their previous-bar marks less the
            // commission to close them
            let e = cash + open.iter().map(|p| { let cd = &cands[p.cand]; ab_mark(p.shares, ep(cd), mark_before(cd, bd), c) }).sum::<f64>();
            close_until(&mut open, &mut cash, &mut taken, (bd, bt), false);
            let mut funds_exhausted = false;
            for &k in list {
                let cd = &cands[k];
                if open.iter().any(|p| cands[p.cand].sym == cd.sym) {
                    continue;
                }
                if open.len() >= book.max_positions {
                    rejected_capacity += 1;
                    continue;
                }
                // AmiBroker ends the entries of a bar at the first one it cannot fund (ATH, `ath_core`; no case in the team BTST lists)
                if funds_exhausted {
                    rejected_cash += 1;
                    continue;
                }
                let px = ep(cd);
                let wanted = match book.alloc { Some(a) => (a / px).floor(), None => (e * book.pct / 100.0 / px).floor() };
                let afford = ath_core::affordable_shares(cash, px, c, true);
                let mut shares = wanted.min(afford);
                // the requested value fits the cash but not its commission: sized again from the equity less the commission on it
                if book.alloc.is_none() && c > 0.0 && wanted > afford && e * book.pct / 100.0 < cash {
                    shares = shares.min(((e - c * ath_core::ab_value(wanted, px)) * book.pct / 100.0 / px).floor());
                }
                let shares = shares as f32 as f64; // AmiBroker holds the share count as a float32
                if shares < 1.0 {
                    funds_exhausted = true;
                    rejected_cash += 1;
                    continue;
                }
                cash -= ath_core::ab_cost(shares, px, c);
                taken.push(Taken { cand: k, shares, weight: shares * cd.entry_px / e, open: true, exit_day: 0, exit_t: 0, exit_px: f64::NAN, profit: f64::NAN });
                open.push(Pos { cand: k, shares, taken: taken.len() - 1 });
            }
        }
        close_until(&mut open, &mut cash, &mut taken, (day, u16::MAX), false);
        eq_dates.push(day);
        equity.push(cash + open.iter().map(|p| { let cd = &cands[p.cand]; ab_mark(p.shares, ep(cd), mark_day(cd, day), c) }).sum::<f64>());
        positions.push(open.len());
    }
    // still open at the end: marked at the last day-end close
    let end = eq_dates.last().copied().unwrap_or(book.to);
    for p in &open {
        let cd = &cands[p.cand];
        let t = &mut taken[p.taken];
        t.open = true;
        t.exit_day = end;
        t.exit_t = 0;
        t.exit_px = mark_day(cd, end);
        t.profit = ab_profit_raw(p.shares, ep(cd), t.exit_px as f32 as f64, c) as f32 as f64;
    }
    Book2 { taken, dates: eq_dates, equity, positions, rejected_capacity, rejected_cash }
}

// ------------------------------------------------------------------ route

fn read(path: &str) -> Result<Vec<u8>, String> {
    std::fs::read(path).map_err(|e| format!("needs_data: {path}: {e}"))
}

fn qf(q: &BTreeMap<String, String>, key: &str, def: f64) -> Result<f64, String> {
    match q.get(key).filter(|v| !v.is_empty()) {
        None => Ok(def),
        Some(v) => v.parse::<f64>().ok().filter(|x| x.is_finite()).ok_or_else(|| format!("{key} must be a number")),
    }
}

fn date_num(s: &str) -> Option<i32> {
    (s.len() == 10 && crate::books::ytmr::day_number(s).is_some()).then(|| s.replace('-', "").parse().ok()).flatten()
}

/// `GET guru/yt/universe`: the universe and the sub-variants a universe run accepts.
pub fn info_json() -> Result<String, String> {
    let dir = data_dir();
    let text = std::fs::read_to_string(format!("{dir}/universe.json")).map_err(|_| format!("needs_data: the stock universe is not loaded ({dir}/universe.json)"))?;
    let m = members(&text, ytbtst::UNIVERSE)?;
    let ids: Vec<String> = ytbtst::catalogue().iter().filter(|d| ytbtst::universe_runnable(d.id)).map(|d| format!("\"{}\"", d.id)).collect();
    let first = m.iter().map(|x| x.first.as_str()).filter(|s| !s.is_empty()).min().unwrap_or("");
    let last = m.iter().map(|x| x.last.as_str()).max().unwrap_or("");
    let pjson = |p: &crate::connors::ParamDef| format!("{{\"key\":\"{}\",\"label\":\"{}\",\"def\":{},\"min\":{},\"max\":{},\"step\":{}}}", p.key, crate::esc(p.label), crate::num(p.def), crate::num(p.min), crate::num(p.max), crate::num(p.step));
    let schemas: Vec<String> = ytbtst::catalogue().iter().filter(|d| ytbtst::universe_runnable(d.id)).map(|d| format!("\"{}\":[{}]", d.id, universe_schema(d.id).iter().map(pjson).collect::<Vec<_>>().join(","))).collect();
    // `members`: the watchlist's symbols (the page's Stock list; a universe run takes `only=` one of them)
    let names: Vec<String> = m.iter().map(|x| format!("\"{}\"", crate::esc(&x.symbol))).collect();
    Ok(format!("{{\"universe\":\"{}\",\"symbols\":{},\"first\":\"{first}\",\"last\":\"{last}\",\"ids\":[{}],\"params\":[{}],\"schemas\":{{{}}},\"members\":[{}],\"baskets\":{}}}", crate::esc(ytbtst::UNIVERSE), m.len(), ids.join(","),
        ytbtst::universe_params().iter().map(pjson).collect::<Vec<_>>().join(","), schemas.join(","), names.join(","), baskets_json(&text)))
}

/// The full parameter list a universe run accepts for `id`: src (1 as stored | 3 the team's 2021 copy), the
/// sub-variant's own rule parameters, and max_positions / pct / pad.
pub fn universe_schema(id: &str) -> Vec<crate::connors::ParamDef> {
    let mut out: Vec<crate::connors::ParamDef> = ytbtst::params(id).unwrap_or_default();
    let src = crate::connors::ParamDef { key: "src", label: "Data source (1 IEOD as stored, 3 IEOD as the team's 2021 copy, to reproduce the reference sheets)", def: 1.0, min: 1.0, max: 3.0, step: 2.0 };
    match out.iter_mut().find(|p| p.key == "src") {
        Some(p) => *p = src,
        None => out.insert(0, src),
    }
    for p in ytbtst::universe_params() {
        match out.iter_mut().find(|q| q.key == p.key) {
            Some(q) => *q = p,
            None => out.push(p),
        }
    }
    out
}

/// `GET guru/yt/universe-run?id=…&p.<key>=…&from=&to=&cost=&slippage=`: one universe backtest.
pub fn run_json(q: &BTreeMap<String, String>) -> Result<String, String> {
    let id = q.get("id").ok_or("id missing")?.as_str();
    if !ytbtst::universe_runnable(id) {
        return Err(format!("not_testable: {id} is not a universe sub-variant"));
    }
    let costs = crate::accounting::Costs::parse(q)?;
    costs.ensure_rollover()?;
    let mut p: Params = crate::params_of(q);
    let src = ytbtst::src_of(&p);
    if src != 1 && src != 3 {
        return Err("not_testable: the stock universe exists only in the IEOD database: src 1 (as stored) or 3 (as the team's 2021 copy)".into());
    }
    p.remove("src");
    let defaults: BTreeMap<&str, f64> = ytbtst::params(id).unwrap_or_default().into_iter().chain(ytbtst::universe_params()).map(|d| (d.key, d.def)).collect();
    let g = |p: &Params, k: &str| p.get(k).copied().unwrap_or_else(|| defaults.get(k).copied().unwrap_or(0.0));
    let max_positions = g(&p, "max_positions").round();
    if !(1.0..=100.0).contains(&max_positions) {
        return Err("max_positions must be 1 to 100".into());
    }
    let pct = match g(&p, "pct") {
        x if x <= 0.0 => 100.0 / max_positions,
        x if x <= 100.0 => x,
        _ => return Err("pct must be 0 to 100".into()),
    };
    let pad = g(&p, "pad").round() == 1.0;
    let equity0 = match p.get("equity") { Some(&v) => v, None => qf(q, "equity", 1_000_000.0)? };
    if !(equity0 >= 1000.0) {
        return Err("equity must be at least 1000".into());
    }
    let sizing = g(&p, "sizing").round();
    let alloc = match sizing {
        x if x == 1.0 => None,
        x if x == 2.0 => Some(match g(&p, "alloc") { a if a > 0.0 => a, _ => equity0 / max_positions }),
        _ => return Err("sizing must be 1 (% of equity, compounding) or 2 (fixed amount per position)".into()),
    };
    for k in ["max_positions", "pct", "pad", "sizing", "alloc", "equity"] {
        p.remove(k);
    }
    let _permit = Permit::take()?;
    let started = Instant::now();
    let dir = data_dir_for(src);
    let text = std::fs::read_to_string(format!("{dir}/universe.json")).map_err(|_| format!("needs_data: the stock universe is not loaded ({dir}/universe.json)"))?;
    // `watchlist=` another watchlist listed in universe.json (the page's basket); `exclude_watchlist=` one whose symbols are left out ("Past")
    let wl = q.get("watchlist").map(|w| w.trim_end_matches(".tls")).filter(|w| !w.is_empty()).unwrap_or(ytbtst::UNIVERSE);
    let mut members = members(&text, wl)?;
    if members.is_empty() {
        return Err(format!("needs_data: no symbol of the watchlist {wl} in the stock universe"));
    }
    if let Some(x) = q.get("exclude_watchlist").map(|w| w.trim_end_matches(".tls")).filter(|w| !w.is_empty()) {
        let out: Vec<String> = members_of(&text, x)?;
        members.retain(|m| !out.contains(&m.symbol));
    }
    // `only=A;B`: run on these watchlist symbols alone (the page's Stock choice)
    if let Some(o) = q.get("only").filter(|s| !s.is_empty()) {
        let want: Vec<&str> = o.split(';').collect();
        members.retain(|m| want.contains(&m.symbol.as_str()));
        if members.is_empty() {
            return Err(format!("{o} is not in the {} watchlist", ytbtst::UNIVERSE));
        }
    }
    let nifty_file = parse_bt5(&read(&format!("{dir}/NIFTY.bt5"))?)?;
    let needs_nifty = matches!(id, "ytbtst_nifty_upclose" | "ytbtst_nifty_incremental");
    let nifty = Nifty::new(&nifty_file);
    let all_dates: Vec<i32> = nifty_file.days.iter().map(|d| d.date).collect();
    let from = match q.get("from").filter(|s| !s.is_empty()) { Some(s) => date_num(s).ok_or("from must be YYYY-MM-DD")?, None => *all_dates.first().ok_or("needs_data: NIFTY calendar empty")? };
    let to = match q.get("to").filter(|s| !s.is_empty()) { Some(s) => date_num(s).ok_or("to must be YYYY-MM-DD")?, None => *all_dates.last().unwrap() };
    if from > to {
        return Err("from must not be after to".into());
    }
    let names: Vec<String> = members.iter().map(|m| m.symbol.clone()).collect();
    let next = AtomicUsize::new(0);
    let results: Mutex<Vec<(Vec<Cand>, usize)>> = Mutex::new(Vec::new());
    let failed: Mutex<Option<String>> = Mutex::new(None);
    let timed_out = AtomicBool::new(false);
    std::thread::scope(|sc| {
        for _ in 0..WORKERS {
            sc.spawn(|| loop {
                let k = next.fetch_add(1, Ordering::Relaxed);
                if k >= members.len() || failed.lock().unwrap().is_some() {
                    break;
                }
                if started.elapsed() > TIME_LIMIT {
                    timed_out.store(true, Ordering::Relaxed);
                    break;
                }
                let m = &members[k];
                let parsed = read(&format!("{dir}/{}", m.file)).and_then(|b| parse_bt5(&b));
                match parsed {
                    Ok(s) => {
                        let s = if pad { pad_align(&s, &nifty_file) } else { s };
                        let r = symbol_candidates(id, &s, k, &p, needs_nifty.then_some(&nifty));
                        results.lock().unwrap().push(r);
                    }
                    Err(e) => *failed.lock().unwrap() = Some(format!("{}: {e}", m.symbol)),
                }
            });
        }
    });
    if let Some(e) = failed.into_inner().unwrap() {
        return Err(e);
    }
    if timed_out.load(Ordering::Relaxed) {
        return Err("unavailable: the universe run reached its 100-second budget; no partial result is published".into());
    }
    let mut cands: Vec<Cand> = Vec::new();
    let mut dropped = 0;
    for (mut c, d) in results.into_inner().unwrap() {
        cands.append(&mut c);
        dropped += d;
    }
    cands.sort_by(|a, b| (a.sym, a.entry_day).cmp(&(b.sym, b.entry_day)));
    let candidate_seconds = started.elapsed().as_secs_f64();
    let book = Book { max_positions: max_positions as usize, pct, alloc, cost: costs.side_rate(), equity: equity0, from, to };
    let res = simulate(&cands, &names, &all_dates, book);
    let gross = simulate(&cands, &names, &all_dates, Book { cost: 0.0, ..book });
    let hhmm = |t: u16| format!("{:02}:{:02}", t / 100, t % 100);
    let ret = |t: &Taken| {
        let cd = &cands[t.cand];
        (t.exit_px * (1.0 - book.cost) - cd.entry_px * (1.0 + book.cost)) / (cd.entry_px * (1.0 + book.cost))
    };
    let trades: Vec<String> = res
        .taken
        .iter()
        .map(|t| {
            let cd = &cands[t.cand];
            format!(
                "{{\"symbol\":\"{}\",\"entry\":\"{}\",\"entry_time\":\"{}\",\"entry_ts\":\"{} {}\",\"exit_ts\":{},\"entry_px\":{},\"exit\":\"{}\",\"exit_time\":{},\"exit_px\":{},\"shares\":{},\"pnl\":{},\"weight\":{},\"score\":{},\"ret\":{},\"reason\":\"{}\",\"open\":{}}}",
                crate::esc(&names[cd.sym]), iso(cd.entry_day), hhmm(cd.entry_t), iso(cd.entry_day), hhmm(cd.entry_t), if t.open { "null".to_string() } else { format!("\"{} {}\"", iso(t.exit_day), hhmm(t.exit_t)) }, crate::num(cd.entry_px), iso(t.exit_day), if t.open { "null".to_string() } else { format!("\"{}\"", hhmm(t.exit_t)) }, crate::num(t.exit_px), t.shares, crate::num(t.profit), crate::num(t.weight), crate::num(cd.score), crate::num(ret(t)), crate::esc(if t.open { "open" } else { &cd.reason }), t.open
            )
        })
        .collect();
    let stats = stats(&res, &res.taken.iter().filter(|t| !t.open).map(ret).collect::<Vec<_>>(), equity0);
    let gstats = stats_gross(&gross, equity0);
    let bh_base = nifty_file.days.iter().find(|d| d.date >= from).map_or(f64::NAN, |d| d.c);
    let bh: Vec<String> = res.dates.iter().map(|&d| nifty_file.days.iter().find(|x| x.date == d).map_or("null".into(), |x| crate::num(x.c / bh_base))).collect();
    let list = |v: Vec<String>| format!("[{}]", v.join(","));
    Ok(format!(
        "{{\"kind\":\"universe\",\"status\":\"ready\",\"id\":\"{id}\",\"src\":{src},\"universe\":\"{}\",\"symbols\":{},\"max_positions\":{},\"pct\":{},\"sizing\":{},\"alloc\":{},\"pad\":{},\"model_initial_equity\":{},\"dates\":{},\"equity\":{},\"gross_equity\":{},\"bh\":{},\"benchmark_symbol\":\"NIFTY_F1\",\"benchmark_basis\":\"Buy & hold of NIFTY near-month continuous futures (IEOD_FO daily close; src 3: the team-copy rebuild), not back-adjusted: roll jumps included, as in AmiBroker's NIFTY_F1\",\"instrument\":{{\"symbol\":\"{}\",\"description\":\"{}\",\"database\":\"{}\"}},\"positions\":{},\"trades\":{},\"stats\":{},\"gross_stats\":{},\"costs\":{},\"portfolio\":{{\"candidates\":{},\"accepted\":{},\"rejected_capacity\":{},\"rejected_cash\":{},\"last_day_signals_dropped\":{},\"candidate_seconds\":{},\"seconds\":{},\"ranking\":\"PositionScore, higher first; equal scores in ascending symbol name\",\"sizing\":\"floor(pct% x previous-bar equity / price) whole shares, compounding; shrink to available cash, skipped below 1 share (the rest of the bar too)\",\"arithmetic\":\"AmiBroker full set as every ATH variant: float32 prices, values, shares and commission %; profit on AmiBroker prices; equity less the commission to close\",\"timing\":\"exits before entries on the same bar\"}},\"notices\":[\"Prices as stored in the team's 2023 IEOD_FO database (some symbols back-adjusted for corporate actions); a symbol exists only on days it has data (no survivorship filter).\",\"Simulated portfolio of the team's AFL rules; not a published result.\"]}}",
        crate::esc(ytbtst::UNIVERSE), names.len(), book.max_positions, crate::num(pct), if alloc.is_some() { 2 } else { 1 }, alloc.map_or("null".into(), crate::num), u8::from(pad), crate::num(equity0),
        list(res.dates.iter().map(|&d| format!("\"{}\"", iso(d))).collect()),
        list(res.equity.iter().map(|v| crate::num(v / equity0)).collect()),
        list(gross.equity.iter().map(|v| crate::num(v / equity0)).collect()),
        list(bh),
        crate::esc(ytbtst::UNIVERSE),
        if src == 3 { "F&O stock futures (near-month, continuous per F&O period), IEOD_FO database updated till 07-02-2023, as the team's 2021 copy" } else { "F&O stock futures (near-month, continuous per F&O period), IEOD_FO database updated till 07-02-2023" },
        crate::esc(ytbtst::DATABASE),
        list(res.positions.iter().map(|v| v.to_string()).collect()),
        list(trades), stats, gstats, costs.json(), cands.len(), res.taken.len(), res.rejected_capacity, res.rejected_cash, dropped, crate::num(candidate_seconds), crate::num(started.elapsed().as_secs_f64())
    ))
}

pub(crate) fn curve_stats(eq: &[f64], equity0: f64) -> BTreeMap<&'static str, f64> {
    let mut s = BTreeMap::new();
    let last = eq.last().copied().unwrap_or(equity0);
    let years = eq.len() as f64 / 252.0;
    s.insert("total_return", last / equity0 - 1.0);
    s.insert("net_profit", last / equity0 - 1.0);
    s.insert("cagr", if years > 0.0 { (last / equity0).powf(1.0 / years) - 1.0 } else { f64::NAN });
    let (mut peak, mut dd) = (equity0, 0.0f64);
    for &v in eq {
        peak = peak.max(v);
        dd = dd.min(v / peak - 1.0);
    }
    s.insert("max_dd", dd);
    let r: Vec<f64> = std::iter::once(equity0).chain(eq.iter().copied()).collect::<Vec<_>>().windows(2).map(|w| w[1] / w[0] - 1.0).collect();
    let m = r.iter().sum::<f64>() / r.len().max(1) as f64;
    let sd = (r.iter().map(|x| (x - m).powi(2)).sum::<f64>() / (r.len().max(2) - 1) as f64).sqrt();
    s.insert("sharpe", if sd > 0.0 { m / sd * 252f64.sqrt() } else { f64::NAN });
    s.insert("mar", if dd < 0.0 { s["cagr"] / -dd } else { f64::NAN });
    s
}

fn stats(b: &Book2, rets: &[f64], equity0: f64) -> String {
    let mut s = curve_stats(&b.equity, equity0);
    let n = rets.len() as f64;
    let wins: Vec<f64> = rets.iter().copied().filter(|r| *r > 0.0).collect();
    let losses: f64 = rets.iter().copied().filter(|r| *r < 0.0).map(|r| -r).sum();
    s.insert("trades", n);
    s.insert("win_rate", if n > 0.0 { wins.len() as f64 / n } else { f64::NAN });
    s.insert("avg_trade", if n > 0.0 { rets.iter().sum::<f64>() / n } else { f64::NAN });
    s.insert("profit_factor", if losses > 0.0 { wins.iter().sum::<f64>() / losses } else { f64::NAN });
    s.insert("exposure", b.positions.iter().filter(|&&p| p > 0).count() as f64 / b.positions.len().max(1) as f64);
    crate::stats_json(&s)
}

fn stats_gross(b: &Book2, equity0: f64) -> String {
    let mut s = curve_stats(&b.equity, equity0);
    s.insert("trades", b.taken.iter().filter(|t| !t.open).count() as f64);
    crate::stats_json(&s)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bt5(name: &str, days: &[(i32, &[(u16, f64, f64, f64, f64, f64)])]) -> Vec<u8> {
        let nb: usize = days.iter().map(|d| d.1.len()).sum();
        let mut out = b"BT5STK01".to_vec();
        out.extend((days.len() as u32).to_le_bytes());
        out.extend((nb as u32).to_le_bytes());
        out.extend((name.len() as u16).to_le_bytes());
        out.extend(name.as_bytes());
        let mut first = 0u32;
        for (date, bars) in days {
            let f = |v: f64| (v as f32).to_le_bytes();
            let (o, c) = (bars[0].1, bars[bars.len() - 1].4);
            let h = bars.iter().map(|b| b.2).fold(f64::MIN, f64::max);
            let l = bars.iter().map(|b| b.3).fold(f64::MAX, f64::min);
            let v: f64 = bars.iter().map(|b| b.5).sum();
            out.extend(date.to_le_bytes());
            for x in [o, h, l, c, v] {
                out.extend(f(x));
            }
            out.extend((bars.len() as u32 * 5).to_le_bytes());
            out.extend(first.to_le_bytes());
            first += bars.len() as u32;
        }
        for (_, bars) in days {
            for &(t, o, h, l, c, v) in bars.iter() {
                out.extend(t.to_le_bytes());
                for x in [o, h, l, c, v] {
                    out.extend((x as f32).to_le_bytes());
                }
            }
        }
        out
    }

    #[test]
    fn bt5_round_trip_and_errors() {
        let b = bt5("ABB 2007-13", &[(20240102, &[(915, 10.0, 11.0, 9.0, 10.5, 100.0), (1450, 10.5, 12.0, 10.0, 11.5, 50.0)]), (20240103, &[(915, 11.0, 11.5, 10.5, 11.25, 70.0)])]);
        let s = parse_bt5(&b).unwrap();
        assert_eq!((s.name.as_str(), s.days.len(), s.bars.len()), ("ABB 2007-13", 2, 3));
        assert_eq!((s.days[0].date, s.days[0].c, s.days[0].h, s.days[0].first, s.days[0].n, s.days[1].first, s.days[1].n), (20240102, 11.5, 12.0, 0, 2, 2, 1));
        assert_eq!(s.bars[1], Bar5 { t: 1450, o: 10.5, h: 12.0, l: 10.0, c: 11.5, v: 50.0 });
        assert!(parse_bt5(&b[..b.len() - 1]).is_err());
        assert!(parse_bt5(b"XXXXXXXXxxxxxxxxxxxx").is_err());
        let (rows, extra) = rows_and_extra(&s, None);
        assert_eq!((rows[1].date.as_str(), rows[0].close), ("2024-01-03", 11.5));
        assert_eq!(extra["BTST_5M.C1450"][0], 11.5);
        assert!(extra["BTST_5M.C1450"][1].is_nan());
        assert_eq!(extra["BTST_5M.DC"], vec![11.5, 11.25]);
        assert_eq!(extra["BTST_5M.V0915"], vec![100.0, 70.0]);
    }

    #[test]
    fn nifty_alignment_and_padding() {
        let n = parse_bt5(&bt5("NIFTY", &[(20240102, &[(915, 100.0, 101.0, 99.0, 100.5, 0.0), (920, 100.5, 101.0, 100.0, 100.75, 0.0), (925, 100.75, 101.0, 100.0, 100.875, 0.0)]), (20240103, &[(915, 101.0, 102.0, 100.5, 101.5, 0.0)])])).unwrap();
        let nf = Nifty::new(&n);
        assert_eq!((nf.at(20240102, 920), nf.at(20240102, 922), nf.at(20240102, 1600), nf.at(20240103, 900)), (100.75, 100.75, 100.875, 100.875));
        assert!(nf.at(20240101, 1600).is_nan());
        // the stock has 0915 and 0930 on day 1 (0930 not in the reference: dropped), nothing on day 2
        let s = parse_bt5(&bt5("X", &[(20240102, &[(915, 10.0, 10.0, 10.0, 10.0, 5.0), (930, 11.0, 11.0, 11.0, 11.0, 5.0)])])).unwrap();
        let pd = pad_align(&s, &n);
        // from the symbol's first bar (0915) to its last (0930): reference bars 0915, 0920, 0925 of day 1
        assert_eq!(pd.bars.iter().map(|b| (b.t, b.c, b.v)).collect::<Vec<_>>(), vec![(915, 10.0, 5.0), (920, 10.0, 0.0), (925, 10.0, 0.0)]);
        assert_eq!((pd.days.len(), pd.days[0].c, pd.days[0].v), (1, 10.0, 5.0));
        let (_, extra) = rows_and_extra(&s, Some((&nf, 930, false)));
        assert_eq!((extra["NIFTYREF.DC"][0], extra["NIFTYREF.E"][0]), (100.875, 100.875));
        // NIFTY's own daily close by date (the day's DC, here the 0925 close 100.875; day 2 = 101.5; before the data NaN)
        assert_eq!((nf.daily(20240102), nf.daily(20240104)), (100.875, 101.5));
        assert!(nf.daily(20231231).is_nan());
    }

    #[test]
    fn universe_schemas_cover_every_runnable_id() {
        for d in ytbtst::catalogue().iter().filter(|d| ytbtst::universe_runnable(d.id)) {
            let keys: Vec<&str> = universe_schema(d.id).iter().map(|p| p.key).collect();
            for k in ["src", "entry_time", "exit_time", "max_positions", "pct", "pad"] {
                // the old fixed AFL has no clock-time parameters (entry / exit candles are counted)
                assert!(keys.contains(&k) || (d.id == "ytbtst_old_fixed" && k.ends_with("_time")), "{} {k}", d.id);
            }
            let src = universe_schema(d.id).into_iter().find(|p| p.key == "src").unwrap();
            assert_eq!((src.min, src.max, src.step), (1.0, 3.0, 2.0));
            let mut sorted = keys.clone();
            sorted.sort();
            sorted.dedup();
            assert_eq!(sorted.len(), keys.len(), "{}", d.id);
        }
        let keys = |id: &str| universe_schema(id).iter().map(|p| p.key).collect::<Vec<_>>();
        assert!(keys("ytbtst_rank_price").contains(&"rank_dir") && keys("ytbtst_rank_lookback").contains(&"lb_version") && keys("ytbtst_rank_roc").contains(&"roc_days"));
        assert!(keys("ytbtst_rank_ibs_volume").contains(&"vol_mult") && keys("ytbtst_core").contains(&"days") && keys("ytbtst_old_fixed").contains(&"max_positions"));
    }

    #[test]
    fn universe_json_members_sorted_and_filtered() {
        let j = r#"{"format":"BT5STK01","stocks":[{"symbol":"ZEEL","file":"ZEEL.bt5","first":"2007-01-02","last":"2023-02-07","days":10,"watchlists":["FO STOCKS C&P","IEOD_FO"]},
            {"symbol":"ABB 2007-13","file":"ABB_2007-13.bt5","first":"2007-01-02","last":"2013-01-01","watchlists":["FO STOCKS C&P"]},
            {"symbol":"OTHER","file":"OTHER.bt5","watchlists":["IT"]},{"symbol":"BAD","file":"../x.bt5","watchlists":["FO STOCKS C&P"]}],"x":true}"#;
        let m = members(j, "FO STOCKS C&P").unwrap();
        assert_eq!(m.iter().map(|x| x.symbol.as_str()).collect::<Vec<_>>(), ["ABB 2007-13", "ZEEL"]);
        assert_eq!(m[0].file, "ABB_2007-13.bt5");
        assert!(members("{", "FO STOCKS C&P").is_err());
        assert_eq!(json(r#"{"a":"Ab","n":-1.5e1}"#), Some(J::O(vec![("a".into(), J::S("Ab".into())), ("n".into(), J::N(-15.0))])));
    }

    fn cand(sym: usize, day: i32, px: f64, exit_day: i32, exit_px: f64, score: f64) -> Cand {
        Cand { sym, entry_day: day, entry_t: 1450, entry_px: px, exit_day, exit_t: 925, exit_px, open: false, reason: "Time exit 0925".into(), score, day_marks: vec![(day, px)], t_marks: vec![] }
    }

    /// Two slots, 10% each of 1,000,000; three signals on day 1: ranked by score (C 3, then A and B tie at 1 -> A by
    /// name); B is rejected for capacity. Sizes: floor(100,000 / 100) = 1000 and floor(100,000 / 50) = 2000 shares.
    /// Commission 0.1% per side. Day 2 exits; the day-2 signal sizes from the compounded equity.
    #[test]
    fn ranking_ties_capacity_sizing_and_compounding() {
        let names: Vec<String> = ["A", "B", "C"].iter().map(|s| s.to_string()).collect();
        let cs = vec![cand(0, 1, 100.0, 2, 110.0, 1.0), cand(1, 1, 80.0, 2, 90.0, 1.0), cand(2, 1, 50.0, 2, 40.0, 3.0), cand(1, 2, 200.0, 3, 210.0, 0.0)];
        let book = Book { max_positions: 2, pct: 10.0, alloc: None, cost: 0.001, equity: 1_000_000.0, from: 1, to: 3 };
        let r = simulate(&cs, &names, &[1, 2, 3], book);
        assert_eq!(r.taken.iter().map(|t| (t.cand, t.shares)).collect::<Vec<_>>(), vec![(2, 2000.0), (0, 1000.0), (3, 494.0)]);
        assert_eq!((r.rejected_capacity, r.rejected_cash), (1, 0));
        // cash after day 1: 1e6 - 2000*50*1.001 - 1000*100*1.001 = 799,800; day 2 exits: + 2000*40*0.999 + 1000*110*0.999
        let cash2: f64 = 799_800.0 + 79_920.0 + 109_890.0;
        // day 2 entry at 14:50: the 09:25 exits happened before; previous-bar equity = cash; 10% / 200 = floor(494.805) = 494
        let e = cash2;
        assert_eq!(r.taken[2].shares, (e * 0.1 / 200.0).floor());
        assert!((r.taken[2].weight - r.taken[2].shares * 200.0 / e).abs() < 1e-9);
        // (the commission is AmiBroker's float32 0.1 %, 0.0010000000149: a few millionths of a rupee from the exact 0.001)
        // equity curve: day 1 marks at the entry prices (day_marks) less the commission to close them, day 3 after the last exit
        assert!((r.equity[0] - (799_800.0 + 2000.0 * 50.0 * 0.999 + 1000.0 * 100.0 * 0.999)).abs() < 1e-4, "{}", r.equity[0]);
        let last = cash2 - 494.0 * 200.0 * 1.001 + 494.0 * 210.0 * 0.999;
        assert!((r.equity[2] - last).abs() < 1e-4, "{} {}", r.equity[2], last);
        assert!(r.taken.iter().all(|t| !t.open));
        // AmiBroker's trade profit: shares x price change less the commission on both values, a float32
        // (A: 1000 x 10 - 0.001 x 210,000 = 9,790; C: 2000 x -10 - 0.001 x 180,000 = -20,180)
        assert_eq!((r.taken[0].profit, r.taken[1].profit), (-20_180.0, 9_790.0));
    }

    /// Sizing 2: floor(alloc / price) shares on every trade regardless of profits; equity = initial + realised P&L.
    /// alloc 1,000 at prices 100 / 50 / 200: 10, 20, 5 shares; day-2 entry after a profit still 5 shares.
    #[test]
    fn fixed_amount_sizing_does_not_compound() {
        let names: Vec<String> = ["A", "B", "C"].iter().map(|s| s.to_string()).collect();
        let cs = vec![cand(0, 1, 100.0, 2, 150.0, 2.0), cand(1, 1, 50.0, 2, 60.0, 1.0), cand(2, 2, 200.0, 3, 220.0, 1.0)];
        let book = Book { max_positions: 5, pct: 20.0, alloc: Some(1000.0), cost: 0.0, equity: 10_000.0, from: 1, to: 3 };
        let r = simulate(&cs, &names, &[1, 2, 3], book);
        assert_eq!(r.taken.iter().map(|t| t.shares).collect::<Vec<_>>(), vec![10.0, 20.0, 5.0]);
        // equity: 10,000 + 10 x 50 + 20 x 10 = 10,700 after day 2, + 5 x 20 = 10,800 after day 3
        assert_eq!(r.equity, vec![10_000.0, 10_700.0, 10_800.0]);
        // compounding (sizing 1, 20%): 20 and 40 shares on day 1, then the day-2 entry sizes from the grown equity
        // 10,000 + 20 x 50 + 40 x 10 = 11,400 -> floor(11,400 x 0.2 / 200) = 11
        let r1 = simulate(&cs, &names, &[1, 2, 3], Book { alloc: None, ..book });
        assert_eq!(r1.taken.iter().map(|t| t.shares).collect::<Vec<_>>(), vec![20.0, 40.0, 11.0]);
        // cash still caps a fixed amount (shrink), and below 1 share the signal is skipped
        let r = simulate(&cs[..1], &names, &[1, 2], Book { alloc: Some(1_000_000.0), ..book });
        assert_eq!(r.taken[0].shares, 100.0);
    }

    /// Cash shortage shrinks the last entry; below one share it is skipped. Exits at the entry bar free a slot first.
    #[test]
    fn shrink_on_cash_shortage_and_exits_before_entries() {
        let names: Vec<String> = ["A", "B", "C"].iter().map(|s| s.to_string()).collect();
        // 60% size: A takes 600 shares x 1000 = 600,000; B wants 600 x 1000 but cash is 400,000 -> floor(400,000/1000) = 400
        let cs = vec![cand(0, 1, 1000.0, 2, 1000.0, 2.0), cand(1, 1, 1000.0, 2, 1000.0, 1.0), cand(2, 1, 500_000.0, 2, 1.0, 0.5)];
        let r = simulate(&cs, &names, &[1, 2], Book { max_positions: 3, pct: 60.0, alloc: None, cost: 0.0, equity: 1_000_000.0, from: 1, to: 2 });
        assert_eq!(r.taken.iter().map(|t| (t.cand, t.shares)).collect::<Vec<_>>(), vec![(0, 600.0), (1, 400.0)]);
        assert_eq!(r.rejected_cash, 1);
        // an exit stamped at the entry bar (14:50) frees the slot for the entry at that bar
        let mut a = cand(0, 1, 100.0, 2, 100.0, 1.0);
        a.exit_t = 1450;
        let b = cand(1, 2, 100.0, 3, 100.0, 1.0);
        let r = simulate(&[a, b], &names, &[1, 2, 3], Book { max_positions: 1, pct: 100.0, alloc: None, cost: 0.0, equity: 1000.0, from: 1, to: 3 });
        assert_eq!(r.taken.len(), 2);
        assert_eq!(r.rejected_capacity, 0);
    }

    /// Previous-bar equity marks a held-over position at its close before the entry time on the later day; a position
    /// still open at the end is reported open at its last mark.
    #[test]
    fn previous_bar_marks_and_open_positions() {
        let names: Vec<String> = ["A", "B"].iter().map(|s| s.to_string()).collect();
        let mut a = cand(0, 1, 100.0, 3, 120.0, 1.0);
        a.day_marks = vec![(1, 100.0), (2, 130.0)];
        a.t_marks = vec![(2, 150.0)];
        let b = cand(1, 2, 10.0, 3, 10.0, 1.0);
        let r = simulate(&[a, b], &names, &[1, 2, 3], Book { max_positions: 2, pct: 10.0, alloc: None, cost: 0.0, equity: 10_000.0, from: 1, to: 3 });
        // day 2 at 14:50: cash 9,000 + 10 shares x 150 = 10,500 -> 10% / 10 = 105 shares
        assert_eq!(r.taken[1].shares, 105.0);
        let mut o = cand(0, 1, 100.0, i32::MAX, f64::NAN, 1.0);
        o.open = true;
        o.day_marks = vec![(1, 100.0), (2, 90.0)];
        let r = simulate(&[o], &names, &[1, 2], Book { max_positions: 1, pct: 50.0, alloc: None, cost: 0.0, equity: 1000.0, from: 1, to: 2 });
        assert_eq!((r.taken[0].open, r.taken[0].exit_day, r.taken[0].exit_px), (true, 2, 90.0));
        assert_eq!(r.equity, vec![1000.0, 950.0]);
    }

    /// Candidates of one symbol through the unchanged single-symbol rules: core signal at 14:50, time exit 09:25; a
    /// signal on the symbol's last data day is dropped; the 3rd data day never trades (buycond2 Null).
    #[test]
    fn symbol_candidates_use_the_single_symbol_rules() {
        let flat = |c: f64| -> Vec<(u16, f64, f64, f64, f64, f64)> { vec![(1525, c, c, c, c, 1.0)] };
        let sig = |p: f64| -> Vec<(u16, f64, f64, f64, f64, f64)> { vec![(915, p - 1.0, p + 1.0, p - 2.0, p - 0.5, 1.0), (1450, p - 0.5, p, p - 1.0, p, 1.0), (1525, p, p, p, p, 1.0)] };
        let morning = |c: f64| -> Vec<(u16, f64, f64, f64, f64, f64)> { vec![(925, c, c, c, c, 1.0), (1525, c, c, c, c, 1.0)] };
        let (d0, d1, d2, d3, d4) = (flat(100.0), flat(102.0), flat(104.0), sig(108.5), morning(109.0));
        let s = parse_bt5(&bt5("X", &[(20240101, &d0), (20240102, &d1), (20240103, &d2), (20240104, &d3), (20240105, &d4)])).unwrap();
        let p: Params = Params::new();
        let (c, dropped) = symbol_candidates("ytbtst_core", &s, 0, &p, None);
        assert_eq!(dropped, 0);
        assert_eq!(c.iter().map(|c| (c.entry_day, c.entry_t, c.entry_px, c.exit_day, c.exit_t, c.exit_px)).collect::<Vec<_>>(), vec![(20240104, 1450, 108.5, 20240105, 925, 109.0)]);
        assert_eq!(c[0].day_marks, vec![(20240104, 108.5)]);
        // the same signal on the last data day is dropped
        let s = parse_bt5(&bt5("X", &[(20240101, &d0), (20240102, &d1), (20240103, &d2), (20240104, &d3)])).unwrap();
        assert_eq!(symbol_candidates("ytbtst_core", &s, 0, &p, None).1, 1);
        // the 3rd data day: history 102, 104 then a signal on day index 2 -> no trade
        let s = parse_bt5(&bt5("X", &[(20240102, &d1), (20240103, &d2), (20240104, &d3), (20240105, &d4)])).unwrap();
        assert!(symbol_candidates("ytbtst_core", &s, 0, &p, None).0.is_empty());
        // price and IBS scores: IBS at 14:50 = (109.5 - 108.5) / (109.5 - 106.5) = 1/3 -> L->H score 100 - 1/3
        let s = parse_bt5(&bt5("X", &[(20240101, &d0), (20240102, &d1), (20240103, &d2), (20240104, &d3), (20240105, &d4)])).unwrap();
        assert_eq!(symbol_candidates("ytbtst_rank_price", &s, 0, &p, None).0[0].score, 108.5);
        let lh: Params = [("rank_dir".to_string(), 2.0)].into_iter().collect();
        assert_eq!(symbol_candidates("ytbtst_rank_price", &s, 0, &lh, None).0[0].score, 1_000_000.0 - 108.5);
        assert!((symbol_candidates("ytbtst_rank_ibs", &s, 0, &lh, None).0[0].score - (100.0 - 1.0 / 3.0)).abs() < 1e-9);
        assert!((symbol_candidates("ytbtst_rank_ibs", &s, 0, &p, None).0[0].score - 1.0 / 3.0).abs() < 1e-9);
        // lookback ranking: cond = how many of lookbacks 3..6 hold at day 3 (100, 102, 104: changes 2, 2 -> lookback 3
        // fails the strict increase) = 0; ver 2 score = list rank, ver 11 = -list rank
        assert_eq!(symbol_candidates("ytbtst_rank_lookback", &s, 7, &p, None).0[0].score, 7.0);
        let v11: Params = [("lb_version".to_string(), 11.0)].into_iter().collect();
        assert_eq!(symbol_candidates("ytbtst_rank_lookback", &s, 7, &v11, None).0[0].score, -7.0);
        // the universe sub-variants never run on a single instrument
        let r: Vec<Row> = Vec::new();
        assert!(ytbtst::run("ytbtst_rank_price", &Ctx::new(&r, &r, &[]), 0, 0, &p).is_none());
    }
}
