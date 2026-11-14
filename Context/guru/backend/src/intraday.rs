//! Intraday (1-minute) F&O futures bars from the team's AmiBroker IEOD database, for the intraday YT strategies.
//!
//! Data: AmiBroker native format-5 files (header "BROKDAt5"), one per symbol at `<root>/<letter>/<SYMBOL>` where the
//! letter is the symbol's first character lowercased, or `_` when it is not alphanumeric. `root` comes from the
//! environment variable `INTRADAY_DATA_DIR` (Docker: `/intraday-data/fo_1min`, mounted read-only); without it no
//! intraday data is available. File layout: a 1,184-byte header (the bar count is the u32 at 0x49C), then 40-byte
//! records: u64 packed date-time (from the top: year 12 bits, month 4, day 5, hour 5, minute 6, second 6, ...), then
//! f32 close, open, high, low, volume, open interest, aux1, aux2. The seconds field is ignored (the vendor stamps some
//! years hh:mm:59): a bar is its minute.
//!
//! Cleaning, the same as `jay-local/Data/intraday/build_ieod_daily.py` (which builds the daily `ieodfo` basket from
//! these files, so the daily carrier and the minutes agree): records dated before 1990 are skipped; a bar with any
//! price <= 0 or not a number, or a second bar with the same date and minute, is dropped; a bar whose open or close
//! lies outside its high-low range has the high / low widened to cover them. Both counts are kept in `Minutes`.
//!
//! Tickers: only `basket:ieodfo:<SYMBOL>` resolves (the engine's daily carrier of the same symbol). Loaded files are
//! kept in a small bounded cache (`CACHE_SYMBOLS` symbols, least recently used dropped); prices are stored as the
//! source's f32 values to bound memory (about 26 bytes per minute bar).
//!
//! Trades of intraday strategies stay on the engine's daily-index model: entry leg / exit_i are the daily bars of the
//! entry / exit dates (`day_index`), prices are the intraday fill prices, and the intraday times go into the trade's
//! `reason`.
#![allow(dead_code)] // session / candle helpers are the API for the intraday YT modules; the pilot uses part of it
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

pub const BASKET_PREFIX: &str = "basket:ieodfo:";
pub const CACHE_SYMBOLS: usize = 4;
/// Session start used to anchor compressed candles (09:15, minute 555 of the day).
pub const SESSION_START: u16 = 915;
const HEADER: usize = 0x4A0;
const REC: usize = 40;
const MAX_BYTES: u64 = 256 * 1024 * 1024;

/// One symbol's minute bars in time order. `day[i]` indexes `dates`; `hhmm[i]` is hour * 100 + minute.
#[derive(Debug, Default)]
pub struct Minutes {
    pub symbol: String,
    pub dates: Vec<String>,
    pub day: Vec<u32>,
    pub hhmm: Vec<u16>,
    pub o: Vec<f32>,
    pub h: Vec<f32>,
    pub l: Vec<f32>,
    pub c: Vec<f32>,
    pub v: Vec<f32>,
    pub dropped: usize,
    pub widened: usize,
}

impl Minutes {
    pub fn len(&self) -> usize {
        self.c.len()
    }
    pub fn is_empty(&self) -> bool {
        self.c.is_empty()
    }
    /// The bar's date, YYYY-MM-DD.
    pub fn date(&self, i: usize) -> &str {
        &self.dates[self.day[i] as usize]
    }
    /// First and last bar index of each trading day, in date order.
    pub fn sessions(&self) -> Vec<(usize, usize)> {
        let mut out: Vec<(usize, usize)> = Vec::with_capacity(self.dates.len());
        for i in 0..self.len() {
            match out.last_mut() {
                Some(s) if self.day[s.0] == self.day[i] => s.1 = i,
                _ => out.push((i, i)),
            }
        }
        out
    }
    /// Is bar `i` the first bar of its trading day?
    pub fn first_of_day(&self, i: usize) -> bool {
        i == 0 || self.day[i] != self.day[i - 1]
    }
    /// Is bar `i` the last bar of its trading day?
    pub fn last_of_day(&self, i: usize) -> bool {
        i + 1 == self.len() || self.day[i] != self.day[i + 1]
    }
}

/// N-minute candles (or the minute bars themselves for n = 1), f64 for indicator code.
/// `start[k]` / `end[k]` are the first / last minute-bar indices of candle k; `hhmm[k]` is the candle's start label.
#[derive(Debug, Default, Clone)]
pub struct Candles {
    pub day: Vec<u32>,
    pub hhmm: Vec<u16>,
    pub o: Vec<f64>,
    pub h: Vec<f64>,
    pub l: Vec<f64>,
    pub c: Vec<f64>,
    pub v: Vec<f64>,
    pub start: Vec<usize>,
    pub end: Vec<usize>,
}

impl Candles {
    pub fn len(&self) -> usize {
        self.c.len()
    }
    pub fn is_empty(&self) -> bool {
        self.c.is_empty()
    }
    pub fn first_of_day(&self, k: usize) -> bool {
        k == 0 || self.day[k] != self.day[k - 1]
    }
    pub fn last_of_day(&self, k: usize) -> bool {
        k + 1 == self.len() || self.day[k] != self.day[k + 1]
    }
}

/// A stored f32 price as f64 rounded to 4 decimals, the same rounding as the daily ieodfo CSVs (`%.4f`, as
/// build_basket.py writes every AmiBroker basket), so candle and daily prices agree. AmiBroker stores prices as f32:
/// 18808.35 is stored as 18808.349609..., so it reads 18808.3496 (within 0.001 of the vendor's price).
pub fn price(v: f32) -> f64 {
    (v as f64 * 1e4).round() / 1e4
}

fn minute_of_day(hhmm: u16) -> i32 {
    (hhmm / 100) as i32 * 60 + (hhmm % 100) as i32
}
fn label(minute: i32) -> u16 {
    (minute / 60 * 100 + minute % 60) as u16
}

/// N-minute candles per trading day, anchored at the session start 09:15: a minute bar at time t belongs to the
/// bucket floor((t - 09:15) / N) of its own date, so candles never span two dates and the first candle of a 09:15
/// session starts at 09:15 (bars before 09:15, as in the 2010 09:00 pre-session, fall in earlier buckets of the
/// same grid; the 09:55 sessions of 2007-2009 are on the same grid). Each candle is labelled with its bucket's START
/// time (AmiBroker's default "time stamp of compressed intraday bars: start time of interval"), so a 5-minute candle
/// labelled 15:20 holds the minutes 15:20-15:24 and its close is the 15:24 minute's close. Open = first minute's open,
/// high / low = extremes, close = last minute's close, volume = sum (prices via `price`). Empty buckets make no candle.
///
/// For N dividing 15 (1, 3, 5, 15) this grid is the same as AmiBroker's midnight-aligned time compression. For other N
/// it is a stated convention: AmiBroker's own alignment of compressed intraday bars was NOT verified against an
/// AmiBroker run.
pub fn compress(m: &Minutes, n_minutes: u16) -> Candles {
    let n = n_minutes.max(1) as i32;
    let anchor = minute_of_day(SESSION_START);
    let mut out = Candles::default();
    let mut cur: Option<(u32, i32)> = None;
    for i in 0..m.len() {
        let bucket = (minute_of_day(m.hhmm[i]) - anchor).div_euclid(n);
        let (o, h, l, c, v) = (price(m.o[i]), price(m.h[i]), price(m.l[i]), price(m.c[i]), m.v[i].max(0.0) as f64);
        if cur == Some((m.day[i], bucket)) {
            let k = out.len() - 1;
            out.h[k] = out.h[k].max(h);
            out.l[k] = out.l[k].min(l);
            out.c[k] = c;
            out.v[k] += v;
            out.end[k] = i;
        } else {
            cur = Some((m.day[i], bucket));
            out.day.push(m.day[i]);
            out.hhmm.push(label(anchor + bucket * n));
            out.o.push(o);
            out.h.push(h);
            out.l.push(l);
            out.c.push(c);
            out.v.push(v);
            out.start.push(i);
            out.end.push(i);
        }
    }
    out
}

/// For each entry of `m.dates`, the index of the same date in the daily rows (`x.r`), or None when the daily series
/// has no such date. Daily rows must be in date order.
pub fn day_index(m: &Minutes, daily: &[crate::Row]) -> Vec<Option<usize>> {
    m.dates.iter().map(|d| daily.binary_search_by(|r| r.date.as_str().cmp(d.as_str())).ok()).collect()
}

/// `basket:ieodfo:SYM` -> SYM (only that basket carries intraday data).
pub fn symbol_of(ticker: &str) -> Option<&str> {
    ticker.strip_prefix(BASKET_PREFIX).filter(|s| !s.is_empty() && !s.contains(['/', '\\', ':']) && !matches!(*s, "." | ".."))
}

fn root() -> Option<PathBuf> {
    std::env::var_os("INTRADAY_DATA_DIR").filter(|v| !v.is_empty()).map(PathBuf::from)
}

pub fn file_path(root: &Path, symbol: &str) -> PathBuf {
    let first = symbol.chars().next().unwrap_or('_');
    let sub = if first.is_ascii_alphanumeric() { first.to_ascii_lowercase().to_string() } else { "_".into() };
    root.join(sub).join(symbol)
}

/// Is there an intraday file for this ticker (cheap check, no load)? Err explains why not.
pub fn available(ticker: &str) -> Result<(), String> {
    let symbol = symbol_of(ticker).ok_or_else(|| format!("{ticker} is not an F&O futures symbol with intraday data (basket ieodfo)"))?;
    let root = root().ok_or("intraday data is not configured on this engine (INTRADAY_DATA_DIR is not set)")?;
    let path = file_path(&root, symbol);
    match std::fs::metadata(&path) {
        Ok(m) if m.is_file() && m.len() > HEADER as u64 => std::fs::File::open(&path).map(|_| ()).map_err(|_| format!("the intraday file for {symbol} is not readable by the engine (check its file permissions)")),
        Ok(_) => Err(format!("the intraday file for {symbol} holds no bars")),
        Err(_) => Err(format!("no intraday file for {symbol} in the intraday database")),
    }
}

static CACHE: Mutex<Vec<(String, Arc<Minutes>)>> = Mutex::new(Vec::new());

/// The symbol's minute bars (lazy load, bounded cache). None when the ticker is not `basket:ieodfo:*`, intraday data
/// is not configured, or the file is missing / unreadable / has no usable bars.
pub fn minutes(ticker: &str) -> Option<Arc<Minutes>> {
    let symbol = symbol_of(ticker)?;
    let root = root()?;
    load_cached(&root, symbol)
}

fn load_cached(root: &Path, symbol: &str) -> Option<Arc<Minutes>> {
    let key = file_path(root, symbol).to_string_lossy().into_owned();
    {
        let mut cache = CACHE.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(pos) = cache.iter().position(|(k, _)| *k == key) {
            let hit = cache.remove(pos);
            let m = hit.1.clone();
            cache.push(hit);
            return Some(m);
        }
    }
    let m = Arc::new(read_file(&file_path(root, symbol), symbol).ok()?);
    if m.is_empty() {
        return None;
    }
    let mut cache = CACHE.lock().unwrap_or_else(|e| e.into_inner());
    cache.retain(|(k, _)| *k != key);
    if cache.len() >= CACHE_SYMBOLS {
        cache.remove(0);
    }
    cache.push((key, m.clone()));
    Some(m)
}

/// Read one AmiBroker format-5 file (see the module header for the layout and cleaning).
pub fn read_file(path: &Path, symbol: &str) -> Result<Minutes, String> {
    let meta = std::fs::metadata(path).map_err(|_| format!("intraday file for {symbol} unavailable"))?;
    if meta.len() > MAX_BYTES {
        return Err(format!("intraday file for {symbol} exceeds the memory admission limit"));
    }
    let b = std::fs::read(path).map_err(|_| format!("intraday file for {symbol} unreadable"))?;
    parse(&b, symbol)
}

pub fn parse(b: &[u8], symbol: &str) -> Result<Minutes, String> {
    if b.len() < HEADER || &b[..8] != b"BROKDAt5" {
        return Err(format!("{symbol}: not an AmiBroker format-5 file"));
    }
    let count = u32::from_le_bytes(b[HEADER - 4..HEADER].try_into().unwrap()) as usize;
    let n = count.min((b.len() - HEADER) / REC);
    let mut m = Minutes { symbol: symbol.into(), ..Default::default() };
    let mut dates: BTreeMap<String, u32> = BTreeMap::new();
    let mut last: Option<u64> = None;
    for k in 0..n {
        let r = &b[HEADER + k * REC..HEADER + (k + 1) * REC];
        let d = u64::from_le_bytes(r[..8].try_into().unwrap());
        let f = |j: usize| f32::from_le_bytes(r[8 + 4 * j..12 + 4 * j].try_into().unwrap());
        let (c, o, h, l, v) = (f(0), f(1), f(2), f(3), f(4));
        let (year, month, day, hour, minute) = (d >> 52, (d >> 48) & 0xF, (d >> 43) & 0x1F, (d >> 38) & 0x1F, (d >> 32) & 0x3F);
        if year < 1990 {
            continue;
        }
        if !(1..=12).contains(&month) || !(1..=31).contains(&day) || hour > 23 || minute > 59 {
            m.dropped += 1;
            continue;
        }
        let stamp = (year << 20) | (month << 16) | (day << 11) | (hour << 6) | minute;
        if last.is_some_and(|p| stamp < p) {
            return Err(format!("{symbol}: minute bars are out of time order"));
        }
        if last == Some(stamp) || [o, h, l, c].iter().any(|p| !p.is_finite() || *p <= 0.0) {
            m.dropped += 1;
            continue;
        }
        last = Some(stamp);
        let (hi, lo) = (h.max(o).max(c), l.min(o).min(c));
        if hi != h || lo != l {
            m.widened += 1;
        }
        let date = format!("{year:04}-{month:02}-{day:02}");
        let next = dates.len() as u32;
        let di = match dates.get(&date) {
            Some(&i) => i,
            None => {
                dates.insert(date.clone(), next);
                m.dates.push(date);
                next
            }
        };
        m.day.push(di);
        m.hhmm.push((hour * 100 + minute) as u16);
        m.o.push(o);
        m.h.push(hi);
        m.l.push(lo);
        m.c.push(c);
        m.v.push(if v.is_finite() { v.max(0.0) } else { 0.0 });
    }
    Ok(m)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static NEXT: AtomicUsize = AtomicUsize::new(0);

    pub(crate) fn pack(y: u64, mo: u64, d: u64, h: u64, mi: u64, s: u64) -> u64 {
        (y << 52) | (mo << 48) | (d << 43) | (h << 38) | (mi << 32) | (s << 26)
    }
    /// A synthetic AmiBroker format-5 file: (packed date, o, h, l, c, v) records.
    pub(crate) fn ab_file(bars: &[(u64, f32, f32, f32, f32, f32)]) -> Vec<u8> {
        let mut b = vec![0u8; HEADER];
        b[..8].copy_from_slice(b"BROKDAt5");
        b[HEADER - 4..HEADER].copy_from_slice(&(bars.len() as u32).to_le_bytes());
        for &(d, o, h, l, c, v) in bars {
            b.extend_from_slice(&d.to_le_bytes());
            for x in [c, o, h, l, v, 0.0, 0.0, 0.0] {
                b.extend_from_slice(&x.to_le_bytes());
            }
        }
        b
    }
    pub(crate) fn temp_root() -> PathBuf {
        let p = std::env::temp_dir().join(format!("guru-intraday-{}-{}", std::process::id(), NEXT.fetch_add(1, Ordering::SeqCst)));
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn reads_cleans_and_maps_the_amibroker_layout() {
        let d = |day, h, mi, s| pack(2022, 3, day, h, mi, s);
        let bars = vec![
            (pack(1980, 1, 1, 0, 0, 0), 1.0, 1.0, 1.0, 1.0, 0.0), // placeholder record: skipped, not counted
            (d(1, 9, 15, 59), 100.0, 101.0, 99.0, 100.5, 10.0),
            (d(1, 9, 16, 59), 100.5, 100.0, 99.5, 101.0, 5.0), // close above high: widened
            (d(1, 9, 16, 59), 100.5, 102.0, 99.5, 101.0, 5.0), // duplicate minute: dropped
            (d(1, 9, 17, 0), 0.0, 102.0, 99.5, 101.0, 5.0),    // zero price: dropped
            (d(2, 9, 15, 0), 102.0, 103.0, 101.0, 102.5, -3.0),
        ];
        let root = temp_root();
        std::fs::create_dir_all(root.join("s")).unwrap();
        std::fs::write(file_path(&root, "SBIN"), ab_file(&bars)).unwrap();
        assert_eq!(file_path(&root, "M&M"), root.join("m").join("M&M"));
        assert_eq!(file_path(&root, "3IINFOTECH 2010-12"), root.join("3").join("3IINFOTECH 2010-12"));
        let m = read_file(&file_path(&root, "SBIN"), "SBIN").unwrap();
        assert_eq!(m.len(), 3);
        assert_eq!((m.dropped, m.widened), (2, 1));
        assert_eq!(m.dates, vec!["2022-03-01", "2022-03-02"]);
        assert_eq!(m.hhmm, vec![915, 916, 915]);
        assert_eq!((m.h[1], m.c[1]), (101.0, 101.0));
        assert_eq!(m.v[2], 0.0);
        assert_eq!(m.date(2), "2022-03-02");
        assert_eq!(m.sessions(), vec![(0, 1), (2, 2)]);
        assert!(m.first_of_day(0) && !m.first_of_day(1) && m.first_of_day(2));
        assert!(!m.last_of_day(0) && m.last_of_day(1) && m.last_of_day(2));
        // the daily carrier's dates map each intraday date to its daily bar
        let row = |date: &str| crate::Row { ticker: "SBIN".into(), date: date.into(), open: 1.0, high: 1.0, low: 1.0, close: 1.0, volume: None };
        let daily = vec![row("2022-02-28"), row("2022-03-01"), row("2022-03-02")];
        assert_eq!(day_index(&m, &daily), vec![Some(1), Some(2)]);
        assert_eq!(day_index(&m, &daily[..2]), vec![Some(1), None]);
        // cache: the same symbol is served from memory
        let a = load_cached(&root, "SBIN").unwrap();
        let b = load_cached(&root, "SBIN").unwrap();
        assert!(Arc::ptr_eq(&a, &b));
        assert!(load_cached(&root, "NOPE").is_none());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn rejects_non_amibroker_and_unordered_files() {
        assert!(parse(b"not a file", "X").is_err());
        let d = |mi| pack(2022, 3, 1, 9, mi, 0);
        let bad = ab_file(&[(d(16), 1.0, 1.0, 1.0, 1.0, 0.0), (d(15), 1.0, 1.0, 1.0, 1.0, 0.0)]);
        assert!(parse(&bad, "X").unwrap_err().contains("order"));
    }

    #[test]
    fn tickers_resolve_only_in_the_ieodfo_basket() {
        assert_eq!(symbol_of("basket:ieodfo:BANKNIFTY"), Some("BANKNIFTY"));
        assert_eq!(symbol_of("basket:ieodfo:ABAN 2007-12"), Some("ABAN 2007-12"));
        for t in ["basket:nse500:SBIN", "BANKNIFTY FUT", "NIFTY 50", "basket:ieodfo:", "basket:ieodfo:../x", "basket:ieodfo:.."] {
            assert!(symbol_of(t).is_none(), "{t}");
        }
        assert!(available("basket:nse500:SBIN").is_err());
    }

    fn minutes_at(times: &[(u32, u16, f32)]) -> Minutes {
        let mut m = Minutes { symbol: "T".into(), dates: vec!["2022-03-01".into(), "2022-03-02".into()], ..Default::default() };
        for &(day, t, c) in times {
            m.day.push(day);
            m.hhmm.push(t);
            m.o.push(c - 0.5);
            m.h.push(c + 1.0);
            m.l.push(c - 1.0);
            m.c.push(c);
            m.v.push(1.0);
        }
        m
    }

    #[test]
    fn compress_anchors_at_the_session_start_and_labels_the_start_time() {
        let mut t: Vec<(u32, u16, f32)> = (0..12).map(|k| (0, label(555 + k), 100.0 + k as f32)).collect(); // 09:15..09:26
        t.extend([(0, 1520, 200.0), (0, 1524, 201.0), (0, 1525, 202.0), (0, 1530, 203.0)]);
        t.extend([(1, 915, 300.0), (1, 919, 301.0)]);
        let m = minutes_at(&t);
        let c5 = compress(&m, 5);
        assert_eq!(c5.hhmm, vec![915, 920, 925, 1520, 1525, 1530, 915]);
        assert_eq!(c5.day, vec![0, 0, 0, 0, 0, 0, 1]);
        // 09:15 candle: open of 09:15, close of 09:19, high/low extremes, volume summed
        assert_eq!((c5.o[0], c5.c[0], c5.h[0], c5.l[0], c5.v[0]), (99.5, 104.0, 105.0, 99.0, 5.0));
        assert_eq!((c5.start[0], c5.end[0]), (0, 4));
        assert_eq!((c5.c[3], c5.end[3]), (201.0, 13)); // 15:20 candle closes at the 15:24 minute
        assert!(c5.last_of_day(5) && c5.first_of_day(6) && !c5.first_of_day(5));
        // pre-09:15 minutes fall into earlier buckets of the same grid; 25-minute candles start at 09:15
        let m2 = minutes_at(&[(0, 900, 1.0), (0, 914, 2.0), (0, 915, 3.0), (0, 939, 4.0), (0, 940, 5.0)]);
        assert_eq!(compress(&m2, 5).hhmm, vec![900, 910, 915, 935, 940]);
        assert_eq!(compress(&m2, 25).hhmm, vec![850, 915, 940]);
        assert_eq!(compress(&m2, 1).len(), 5);
        assert_eq!((price(18808.35_f32), price(100.05_f32)), (18808.3496, 100.05));
    }
}
