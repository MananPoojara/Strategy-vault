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
//! entry / exit dates (`day_index`) and prices are the intraday fill prices, so returns, the whole-unit account and the
//! stats are computed from the actual fills; open positions are marked at the daily close of the same series. Each
//! module also records a `TradeTiming` per trade (candle stamps, fill minutes, candles held, candle and 1-minute
//! excursion prices) that the run report attaches to the trade (`capture_timings` / `record`).
//!
//! Candle alignment follows AmiBroker's time compression (`Align`, `compress_aligned`), daily filters use
//! TimeFrameExpand's expandLast ("the compressed value is expanded starting from the last bar within a given period",
//! https://www.amibroker.com/guide/afl/timeframeexpand.html), and the instrument is the team's continuous-futures IEOD
//! database (`INSTRUMENT_BASIS`).
#![allow(dead_code)] // session / candle helpers are the API for the intraday YT modules; the pilot uses part of it
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

pub const BASKET_PREFIX: &str = "basket:ieodfo:";
pub const CACHE_SYMBOLS: usize = 4;
pub(crate) fn validate_research_params(schema:&[crate::connors::ParamDef],p:&crate::connors::Params)->Result<(),String> {
    for (key,value) in p {
        let d=schema.iter().find(|d|d.key==key.as_str()).ok_or_else(||format!("unknown research parameter {key}"))?;
        if !value.is_finite() || *value<d.min || *value>d.max {return Err(format!("research parameter {key} is outside its finite admitted range"));}
    }
    Ok(())
}
/// Year-dependent OpenInt thresholds used by the team intraday AFLs. The hourly
/// 2D files contain their own rounded table rather than dividing at runtime.
pub fn open_interest_threshold(date:&str,hourly:bool)->Option<f64> {
    let y=date.get(..4)?.parse::<u32>().ok()?;
    Some(match (y,hourly) {
        (0..=1999,_)=>30000.,(2000..=2003,false)=>50000.,(2000..=2003,true)=>7100.,
        (2004,false)=>200000.,(2004,true)=>28600.,(2005,false)=>400000.,(2005,true)=>57100.,
        (2006,false)=>600000.,(2006,true)=>85700.,(2007..=2009,false)=>800000.,(2007..=2009,true)=>114300.,
        (2010,false)=>1000000.,(2010,true)=>142900.,(2011,false)=>2000000.,(2011,true)=>285700.,
        (2012,false)=>2500000.,(2012,true)=>357100.,(2013,false)=>3000000.,(2013,true)=>428600.,
        (2014,false)=>3500000.,(2014,true)=>500000.,(2015..=2016,false)=>4000000.,(2015..=2016,true)=>571400.,
        (2017,false)=>5000000.,(2017,true)=>714300.,(2018,false)=>6000000.,(2018,true)=>857100.,
        (2019,false)=>7000000.,(2019,true)=>1000000.,(2020,false)=>8000000.,(2020,true)=>1142900.,_=>9000000.,
    })
}
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
    /// Source OpenInt; unavailable or invalid values remain absent rather than becoming zero.
    pub oi: Vec<Option<f32>>,
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
    /// Last source OpenInt observation in each compressed candle.
    pub oi: Vec<Option<f64>>,
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

/// How N-minute candles are aligned, as AmiBroker's time compression offers it.
///
/// AmiBroker documents (Preferences, Intraday tab, https://www.amibroker.com/guide/w_preferences.html):
/// "Align custom minute bars to regular market hours — when checked, AmiBroker will trim pre-market custom interval
/// bars so a new bar will begin exactly when trading hours start ... Let's say that we have 45-minute bars. Without
/// this setting, we would get bars starting at 9:00, 9:45, 10:30, 11:15 etc. When this is turned on and trading
/// starts at 9:30 ... 8:45, 9:30, 10:15, 11:00". So without the option the grid is multiples of the interval counted
/// from midnight (9:00 = 12 × 45 min) — `Midnight`, the behaviour of an AmiBroker install without that box checked
/// and of its built-in intervals; with it the grid runs from the trading-hours start (09:15 for NSE F&O) — `Session`.
/// The guide does not state the box's default; the team's own setting is unknown, so it is the `bar_align` setting.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Align {
    /// Multiples of N minutes from 00:00 (AmiBroker without "Align custom minute bars to regular market hours").
    #[default]
    Midnight,
    /// Multiples of N minutes from the 09:15 trading start ("Align custom minute bars to regular market hours" on).
    Session,
}

pub const ALIGN_KEY: &str = "bar_align";
/// The `bar_align` setting every intraday YT strategy with a candle timeframe offers.
pub fn align_param() -> crate::connors::ParamDef {
    crate::connors::ParamDef { key: ALIGN_KEY, label: "Candle alignment (0 AmiBroker time compression from midnight, 1 'Align custom minute bars to regular market hours' at 09:15)", def: 0., min: 0., max: 1., step: 1. }
}
pub fn align_of(p: &crate::connors::Params) -> Align {
    if p.get(ALIGN_KEY).is_some_and(|v| v.is_finite() && *v >= 0.5) { Align::Session } else { Align::Midnight }
}
impl Align {
    pub fn describe(self) -> &'static str {
        match self {
            Align::Midnight => "candles on AmiBroker's time-compression grid (multiples of the interval from midnight, so a 60-minute day starts with a 09:15-09:59 candle stamped 09:00)",
            Align::Session => "candles aligned to the 09:15 trading start (AmiBroker's 'Align custom minute bars to regular market hours')",
        }
    }
}

/// N-minute candles per trading day on AmiBroker's default grid (`Align::Midnight`); see `compress_aligned`.
pub fn compress(m: &Minutes, n_minutes: u16) -> Candles {
    compress_aligned(m, n_minutes, Align::Midnight)
}

/// N-minute candles per trading day, the way AmiBroker time-compresses 1-minute data: a minute bar at time t of its
/// date belongs to the bucket floor((t - anchor) / N), anchor = 00:00 (`Align::Midnight`) or 09:15 (`Align::Session`),
/// so candles never span two dates. A session that starts inside a bucket gives a shorter first candle: on the
/// midnight grid a 30-minute day starts with 09:15-09:29 (stamped 09:00), a 60-minute day with 09:15-09:59 (09:00) and
/// a 75-minute day with 09:15-09:59 (08:45, as 525 = 7 × 75). Each candle is stamped with its bucket's START time
/// (AmiBroker's "Time compressed bars show: START time of the interval": "a 30-minute bar covers 9:00:00..9:29:59 ...
/// AmiBroker will display the time of this bar as 9:00", https://www.amibroker.com/guide/w_preferences.html), so
/// `TimeNum()` / `Hour()*100+Minute()` of an AFL read that start time. Open = first minute's open, high / low =
/// extremes, close = last minute's close, volume = sum (prices via `price`). Empty buckets make no candle.
/// For N dividing 15 (1, 3, 5, 15) both grids are the same at 09:15.
pub fn compress_aligned(m: &Minutes, n_minutes: u16, align: Align) -> Candles {
    let n = n_minutes.max(1) as i32;
    let anchor = match align { Align::Midnight => 0, Align::Session => minute_of_day(SESSION_START) };
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
            out.oi[k] = m.oi.get(i).copied().flatten().map(f64::from);
            out.end[k] = i;
        } else {
            cur = Some((m.day[i], bucket));
            out.day.push(m.day[i]);
            out.hhmm.push(label((anchor + bucket * n).rem_euclid(1440)));
            out.o.push(o);
            out.h.push(h);
            out.l.push(l);
            out.c.push(c);
            out.v.push(v);
            out.oi.push(m.oi.get(i).copied().flatten().map(f64::from));
            out.start.push(i);
            out.end.push(i);
        }
    }
    out
}

/// One intraday trade's clock and excursion prices, recorded by the intraday YT modules next to the ordinary daily
/// `Trade` (whose legs stay on the daily carrier's bars so the engine's equity, whole-unit account and stats are
/// unchanged). `entry_i` / `exit_i` / prices repeat the trade's daily indices and fills so the caller can match
/// each record to its trade.
#[derive(Clone, Debug)]
pub struct TradeTiming {
    pub entry_i: usize,
    pub exit_i: usize,
    pub entry_px: f64,
    pub exit_px: f64,
    pub entry_date: String,
    /// Start-time stamp of the entry / exit candle (what an AmiBroker trade list shows for the bar).
    pub entry_bar: u16,
    pub exit_bar: u16,
    /// The 1-minute bar in which the fill happened (the candle's last minute for a close fill, its first minute for
    /// an open fill, otherwise the first minute whose range reaches the fill price).
    pub entry_fill: u16,
    pub exit_date: String,
    pub exit_fill: u16,
    /// Candles from the entry candle to the exit candle (0 = entry and exit on the same candle).
    pub candles: usize,
    pub tf_min: u16,
    pub align: Align,
    /// AmiBroker trade-list excursion inputs on the strategy's own candles (the convention of
    /// `accounting::TradeLedger::amibroker`): lowest low / highest high from the entry candle through the candle before
    /// the exit candle (the entry candle alone for a one-candle trade) and the exit candle's open when the trade spans
    /// candles and is closed.
    pub ab_low: f64,
    pub ab_high: f64,
    pub ab_exit_open: Option<f64>,
    /// Exact 1-minute path between the fills: lowest low / highest high of the minutes after the entry fill (from the
    /// entry minute itself for an open fill) up to the exit fill (the exit minute fully for a close fill, its open
    /// otherwise). None when entry and exit fill in the same minute.
    pub path_low: Option<f64>,
    pub path_high: Option<f64>,
}

impl TradeTiming {
    pub fn matches(&self, t: &crate::connors::Trade) -> bool {
        t.legs.len() == 1 && t.legs[0].i == self.entry_i && t.exit_i == self.exit_i
            && (t.legs[0].px - self.entry_px).abs() < 1e-9 && (t.exit_px - self.exit_px).abs() < 1e-9
    }
    pub fn hm(t: u16) -> String {
        format!("{:02}:{:02}", t / 100, t % 100)
    }
}

thread_local! {
    static TIMINGS: std::cell::RefCell<Option<Vec<TradeTiming>>> = const { std::cell::RefCell::new(None) };
}

/// Run `f` (a strategy run on this thread) and return the trade timings the intraday modules recorded during it.
pub fn capture_timings<R>(f: impl FnOnce() -> R) -> (R, Vec<TradeTiming>) {
    TIMINGS.with(|t| *t.borrow_mut() = Some(Vec::new()));
    let r = f();
    let out = TIMINGS.with(|t| t.borrow_mut().take()).unwrap_or_default();
    (r, out)
}

/// Record the timings of `trades` (built on candles `b` of minutes `m`; `pairs[k]` = (entry candle, exit candle) of
/// trades[k]). A no-op unless a `capture_timings` is active on this thread; a later call replaces an earlier one.
pub(crate) fn record(m: &Minutes, b: &Candles, tf_min: u16, align: Align, trades: &[crate::connors::Trade], pairs: &[(usize, usize)]) {
    if TIMINGS.with(|t| t.borrow().is_none()) {
        return;
    }
    let list = if trades.len() == pairs.len() { trades.iter().zip(pairs).filter_map(|(t, &(e, z))| timing(m, b, tf_min, align, t, e, z)).collect() } else { Vec::new() };
    TIMINGS.with(|t| *t.borrow_mut() = Some(list));
}

/// The minute index of a fill at `px` inside candle k.
fn fill_minute(m: &Minutes, b: &Candles, k: usize, px: f64, at_open: bool) -> usize {
    if at_open {
        return b.start[k];
    }
    if (px - b.c[k]).abs() < 1e-4 {
        return b.end[k];
    }
    (b.start[k]..=b.end[k]).find(|&i| price(m.l[i]) - 1e-4 <= px && px <= price(m.h[i]) + 1e-4).unwrap_or(b.end[k])
}

fn timing(m: &Minutes, b: &Candles, tf_min: u16, align: Align, t: &crate::connors::Trade, e: usize, z: usize) -> Option<TradeTiming> {
    if t.legs.len() != 1 || e >= b.len() || z >= b.len() || z < e || b.end[z] >= m.len() {
        return None;
    }
    let (entry_px, exit_px) = (t.legs[0].px, t.exit_px);
    let entry_open = t.entry_kind == "open";
    let ef = fill_minute(m, b, e, entry_px, entry_open);
    let exit_close = t.open || (exit_px - b.c[z]).abs() < 1e-4;
    let xf = if t.open { b.end[z] } else { fill_minute(m, b, z, exit_px, false) };
    let (mut ab_low, mut ab_high) = (f64::INFINITY, f64::NEG_INFINITY);
    for k in e..z.max(e + 1) {
        ab_low = ab_low.min(b.l[k]);
        ab_high = ab_high.max(b.h[k]);
    }
    let first = if entry_open { ef } else { ef + 1 };
    let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
    for i in first..xf.max(first) {
        lo = lo.min(price(m.l[i]));
        hi = hi.max(price(m.h[i]));
    }
    if xf > ef || (entry_open && xf == ef) {
        let (l, h) = if exit_close { (price(m.l[xf]), price(m.h[xf])) } else { (price(m.o[xf]), price(m.o[xf])) };
        lo = lo.min(l);
        hi = hi.max(h);
    }
    let path = lo.is_finite().then_some((lo.min(exit_px), hi.max(exit_px)));
    Some(TradeTiming {
        entry_i: t.legs[0].i,
        exit_i: t.exit_i,
        entry_px,
        exit_px,
        entry_date: m.date(ef).to_string(),
        entry_bar: b.hhmm[e],
        exit_bar: b.hhmm[z],
        entry_fill: m.hhmm[ef],
        exit_date: m.date(xf).to_string(),
        exit_fill: m.hhmm[xf],
        candles: z - e,
        tf_min,
        align,
        ab_low,
        ab_high,
        ab_exit_open: (!t.open && z > e).then_some(b.o[z]),
        path_low: path.map(|p| p.0),
        path_high: path.map(|p| p.1),
    })
}

/// The instrument these intraday results are computed on, stated with every intraday run.
pub const INSTRUMENT_BASIS: &str = "Instrument: the continuous front-month futures series of the team's IEOD F&O AmiBroker database (1-minute bars to 07-02-2023) - the same database the team ran these AFLs on in AmiBroker, so it is the instrument their backtests traded. Contract roll-overs appear as price jumps in the series, exactly as in AmiBroker; no contract-level ledger, lot size or margin is modelled.";
/// How intraday trades are accounted, stated with every intraday run.
pub const ACCOUNTING_BASIS: &str = "Accounting: fills are the AFL's intraday prices on the compressed candles; each trade carries its entry / exit candle stamp and fill minute and its bars held in candles. Returns, the whole-unit account and statistics use those fill prices. The equity curve is marked once a day: a position still open at a day's end is marked at that day's close of the same continuous series, and several trades in one day compound into that day's mark. MAE / MFE follow AmiBroker's trade-list convention on the strategy's candles (entry candle through the candle before the exit, plus the exit candle's open and the exit fill); the exact 1-minute path between the fills is reported alongside.";

/// The AFL's future references, for the run notice. The engine runs these as written: AmiBroker evaluates a future
/// reference with the real later bars of the loaded data, so the team's runs contained the same look-ahead.
pub fn look_ahead_notice(id: &str) -> Option<&'static str> {
    use crate::books::{yt2ds, yt30m, ytgd, ytnth, ytorb_draft};
    Some(match id {
        ytnth::ID => "This AFL references future bars: entrydaycon = TimeFrameExpand(Ref(today, 1), inDaily) reads the NEXT trading day's day counter (on a day's last candle, the next day's value; on its earlier candles, the current day's), and ENDBAR = Day() != Ref(Day(), 1) reads the next candle. Results contain look-ahead as in the team's AmiBroker run.",
        ytorb_draft::ID => "This AFL references future bars: its gap / lap and narrow-range conditions are TimeFrameExpand(Ref(x, 1), inDaily), the NEXT day's daily values, and lc = DateNum() != Ref(DateNum(), 1) reads the next candle. Results contain look-ahead as in the team's AmiBroker run.",
        ytgd::MINUTE_TIME | ytgd::CONFIRM_POS | ytgd::CONFIRM_NEG | ytgd::CONFIRM_NONE => "This AFL references future bars: its sell condition MyDay != Ref(MyDay, 2) OR TimeNow != Ref(TimeNow, 1) reads the next minutes, is true on almost every minute and so sells at the close of the 09:20 entry minute itself; TimeFrameGetPrice(\"O\", inDaily, 0) is the day's own open (known at 09:20). Results contain look-ahead as in the team's AmiBroker run.",
        ytgd::SHOCK_IEOD => "This AFL references future bars: the day-end exit Day() != Ref(Day(), 1) reads the next candle. Results contain this look-ahead as in the team's AmiBroker run.",
        _ if yt2ds::INTRADAY_IDS.contains(&id) => "This AFL references future bars: the month-end candle is found with Ref(Month(), 1), the next candle's month. Results contain this look-ahead as in the team's AmiBroker run.",
        _ if yt30m::IDS.contains(&id) => "This AFL references future bars: ENDBAR = Day() != Ref(Day(), 1) reads the next candle (the day's last candle is known on that candle), and the VER 1 / VER 2 AFLs combine the day's first candle with the last candle through ValueWhen(ENDBAR, ...), so on a day's last candle the level already includes that candle. Results contain this look-ahead as in the team's AmiBroker run.",
        _ => return None,
    })
}

/// The run notices of an intraday YT strategy: instrument, accounting, candle alignment and look-ahead.
pub fn run_notices(id: &str, p: &crate::connors::Params) -> Vec<String> {
    let mut v = vec![INSTRUMENT_BASIS.to_string(), ACCOUNTING_BASIS.to_string(), format!("Candles: {}; bars are stamped with the start time of their interval (AmiBroker's 'START time of the interval'). Daily filters computed with TimeFrameSet(inDaily) use the daily bars of the same minutes and reach the candles through TimeFrameExpand's default expandLast: a day's value from that day's last candle, the previous day's value on its earlier candles.", align_of(p).describe())];
    if let Some(n) = look_ahead_notice(id) {
        v.push(n.to_string());
    }
    v
}

/// For each entry of `m.dates`, the index of the same date in the daily rows (`x.r`), or None when the daily series
/// has no such date. Daily rows must be in date order.
pub fn day_index(m: &Minutes, daily: &[crate::Row]) -> Vec<Option<usize>> {
    m.dates.iter().map(|d| daily.binary_search_by(|r| r.date.as_str().cmp(d.as_str())).ok()).collect()
}

/// Require the full daily indicator prefix to match the same minute source.
pub(crate) fn validate_daily_carrier(m:&Minutes,daily:&[crate::Row],to:usize)->Result<(),String> {
    if daily.is_empty() || to>=daily.len() || m.dates.first()!=Some(&daily[0].date) {return Err("daily carrier and minute source have different history starts".into());}
    if daily[..=to].iter().any(|r|r.ticker!=daily[0].ticker || m.dates.binary_search(&r.date).is_err()) {
        return Err("daily carrier contains a different symbol or a date without minute bars".into());
    }
    for (first,last) in m.sessions() {
        let date=m.date(first);if date>daily[to].date.as_str(){break;}
        let j=daily.binary_search_by(|r|r.date.as_str().cmp(date)).map_err(|_|format!("daily carrier does not include intraday date {date}"))?;
        let high=m.h[first..=last].iter().copied().fold(f32::NEG_INFINITY,f32::max);
        let low=m.l[first..=last].iter().copied().fold(f32::INFINITY,f32::min);
        let observed=[price(m.o[first]),price(high),price(low),price(m.c[last])];
        let carrier=[daily[j].open,daily[j].high,daily[j].low,daily[j].close];
        if observed.iter().zip(carrier).any(|(a,b)|!b.is_finite() || (*a-b).abs()>0.00011) {return Err(format!("daily carrier differs from minute source on {date}"));}
    }
    Ok(())
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
    load_cached(&root, symbol).map(|_| ())
}

static CACHE: Mutex<Vec<(String, Arc<Minutes>)>> = Mutex::new(Vec::new());

/// The symbol's minute bars (lazy load, bounded cache). None when the ticker is not `basket:ieodfo:*`, intraday data
/// is not configured, or the file is missing / unreadable / has no usable bars.
pub fn minutes(ticker: &str) -> Option<Arc<Minutes>> {
    let symbol = symbol_of(ticker)?;
    let root = root()?;
    load_cached(&root, symbol).ok()
}

fn load_cached(root: &Path, symbol: &str) -> Result<Arc<Minutes>, String> {
    let key = file_path(root, symbol).to_string_lossy().into_owned();
    {
        let mut cache = CACHE.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(pos) = cache.iter().position(|(k, _)| *k == key) {
            let hit = cache.remove(pos);
            let m = hit.1.clone();
            cache.push(hit);
            return Ok(m);
        }
    }
    let m = Arc::new(read_file(&file_path(root, symbol), symbol)?);
    if m.is_empty() {
        return Err(format!("the intraday file for {symbol} holds no usable bars"));
    }
    let mut cache = CACHE.lock().unwrap_or_else(|e| e.into_inner());
    cache.retain(|(k, _)| *k != key);
    if cache.len() >= CACHE_SYMBOLS {
        cache.remove(0);
    }
    cache.push((key, m.clone()));
    Ok(m)
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
    if count > (b.len() - HEADER) / REC {
        return Err(format!("{symbol}: intraday file is truncated"));
    }
    let n = count;
    let mut m = Minutes { symbol: symbol.into(), ..Default::default() };
    let mut dates: BTreeMap<String, u32> = BTreeMap::new();
    let mut last: Option<u64> = None;
    for k in 0..n {
        let r = &b[HEADER + k * REC..HEADER + (k + 1) * REC];
        let d = u64::from_le_bytes(r[..8].try_into().unwrap());
        let f = |j: usize| f32::from_le_bytes(r[8 + 4 * j..12 + 4 * j].try_into().unwrap());
        let (c, o, h, l, v, oi) = (f(0), f(1), f(2), f(3), f(4), f(5));
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
        m.oi.push((oi.is_finite() && oi>=0.).then_some(oi));
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
        assert_eq!(m.oi, vec![Some(0.0),Some(0.0),Some(0.0)]);
        let candles=compress(&m,5);
        assert_eq!(candles.oi,vec![Some(0.0),Some(0.0)]);
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
        assert!(load_cached(&root, "NOPE").is_err());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn rejects_non_amibroker_and_unordered_files() {
        assert!(parse(b"not a file", "X").is_err());
        let mut truncated = ab_file(&[(pack(2022, 3, 1, 9, 15, 0), 1.0, 1.0, 1.0, 1.0, 0.0)]);
        truncated.pop();
        assert!(parse(&truncated, "X").unwrap_err().contains("truncated"));
        let d = |mi| pack(2022, 3, 1, 9, mi, 0);
        let bad = ab_file(&[(d(16), 1.0, 1.0, 1.0, 1.0, 0.0), (d(15), 1.0, 1.0, 1.0, 1.0, 0.0)]);
        assert!(parse(&bad, "X").unwrap_err().contains("order"));
    }

    // Opt-in integration check against a local source and its independently stored daily carrier.
    // Runtime data remains outside Git; ordinary unit tests need no market-data mount.
    #[test]
    fn local_banknifty_minutes_match_daily_carrier_when_configured() {
        let (Ok(root), Ok(csv)) = (std::env::var("TEST_INTRADAY_DIR"), std::env::var("TEST_INTRADAY_DAILY")) else { return };
        let m = read_file(&file_path(Path::new(&root), "BANKNIFTY"), "BANKNIFTY").unwrap();
        let text = std::fs::read_to_string(csv).unwrap();
        let rows: Vec<_> = text.lines().skip(1).map(|line| {
            let f: Vec<_> = line.split(',').collect();
            (f[0], f[1].parse::<f64>().unwrap(), f[2].parse::<f64>().unwrap(), f[3].parse::<f64>().unwrap(), f[4].parse::<f64>().unwrap())
        }).collect();
        let sessions = m.sessions();
        assert_eq!(sessions.len(), rows.len());
        for ((start, end), &(date, open, high, low, close)) in sessions.into_iter().zip(&rows) {
            assert_eq!(m.date(start), date);
            let actual_high = m.h[start..=end].iter().copied().fold(f32::NEG_INFINITY, f32::max);
            let actual_low = m.l[start..=end].iter().copied().fold(f32::INFINITY, f32::min);
            for (actual, expected) in [(price(m.o[start]), open), (price(actual_high), high), (price(actual_low), low), (price(m.c[end]), close)] {
                assert!((actual - expected).abs() < 0.00011, "daily carrier differs on {date}");
            }
        }
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
    fn compress_aligned_to_the_session_start_labels_the_start_time() {
        let mut t: Vec<(u32, u16, f32)> = (0..12).map(|k| (0, label(555 + k), 100.0 + k as f32)).collect(); // 09:15..09:26
        t.extend([(0, 1520, 200.0), (0, 1524, 201.0), (0, 1525, 202.0), (0, 1530, 203.0)]);
        t.extend([(1, 915, 300.0), (1, 919, 301.0)]);
        let m = minutes_at(&t);
        let c5 = compress_aligned(&m, 5, Align::Session);
        assert_eq!(c5.hhmm, vec![915, 920, 925, 1520, 1525, 1530, 915]);
        assert_eq!(c5.day, vec![0, 0, 0, 0, 0, 0, 1]);
        // 09:15 candle: open of 09:15, close of 09:19, high/low extremes, volume summed
        assert_eq!((c5.o[0], c5.c[0], c5.h[0], c5.l[0], c5.v[0]), (99.5, 104.0, 105.0, 99.0, 5.0));
        assert_eq!((c5.start[0], c5.end[0]), (0, 4));
        assert_eq!((c5.c[3], c5.end[3]), (201.0, 13)); // 15:20 candle closes at the 15:24 minute
        assert!(c5.last_of_day(5) && c5.first_of_day(6) && !c5.first_of_day(5));
        // pre-09:15 minutes fall into earlier buckets of the same grid; 25-minute candles start at 09:15
        let m2 = minutes_at(&[(0, 900, 1.0), (0, 914, 2.0), (0, 915, 3.0), (0, 939, 4.0), (0, 940, 5.0)]);
        assert_eq!(compress_aligned(&m2, 5, Align::Session).hhmm, vec![900, 910, 915, 935, 940]);
        assert_eq!(compress_aligned(&m2, 25, Align::Session).hhmm, vec![850, 915, 940]);
        assert_eq!(compress_aligned(&m2, 1, Align::Session).len(), 5);
        assert_eq!((price(18808.35_f32), price(100.05_f32)), (18808.3496, 100.05));
    }

    #[test]
    fn compress_default_is_amibrokers_midnight_grid() {
        // a 09:15-15:29 session in minutes
        let t: Vec<(u32, u16, f32)> = (555..930).map(|k| (0, label(k), 100.0 + (k % 7) as f32)).collect();
        let m = minutes_at(&t);
        // 60 minutes: 09:00 (09:15-09:59), 10:00 ... 15:00 (15:00-15:29)
        let h = compress(&m, 60);
        assert_eq!(h.hhmm, vec![900, 1000, 1100, 1200, 1300, 1400, 1500]);
        assert_eq!((h.start[0], h.end[0]), (0, 44));
        assert_eq!(h.end[6] - h.start[6], 29);
        // 30 minutes: the first candle is 09:15-09:29, stamped 09:00
        let c30 = compress(&m, 30);
        assert_eq!((c30.hhmm[0], c30.hhmm[1], c30.end[0]), (900, 930, 14));
        assert_eq!(c30.len(), 13);
        // 75 minutes: 525 = 7 x 75, so 08:45 (09:15-09:59), 10:00, 11:15, 12:30, 13:45, 15:00
        assert_eq!(compress(&m, 75).hhmm, vec![845, 1000, 1115, 1230, 1345, 1500]);
        // 25 minutes: 550 = 22 x 25, so 09:10 (09:15-09:34), 09:35 ...
        let c25 = compress(&m, 25);
        assert_eq!((c25.hhmm[0], c25.hhmm[1], c25.end[0]), (910, 935, 19));
        // 5 minutes: both grids agree at 09:15
        assert_eq!(compress(&m, 5).hhmm, compress_aligned(&m, 5, Align::Session).hhmm);
        // the 45-minute example of AmiBroker's Preferences page: 9:00, 9:45, 10:30 without the option
        let m45 = minutes_at(&(540..660).map(|k| (0, label(k), 1.0)).collect::<Vec<_>>());
        assert_eq!(compress(&m45, 45).hhmm, vec![900, 945, 1030]);
        // session alignment (the option on, trading start 09:15) gives 09:15, 10:15 ... for 60 minutes
        assert_eq!(compress_aligned(&m, 60, Align::Session).hhmm, vec![915, 1015, 1115, 1215, 1315, 1415, 1515]);
        assert_eq!(align_of(&crate::connors::Params::from([(ALIGN_KEY.into(), 1.)])), Align::Session);
        assert_eq!(align_of(&crate::connors::Params::new()), Align::Midnight);
    }

    #[test]
    fn timings_record_candle_stamps_fill_minutes_and_excursions() {
        let mut t: Vec<(u32, u16, f32)> = (555..585).map(|k| (0, label(k), 100.0 + (k - 555) as f32)).collect(); // 09:15..09:44
        t.extend((555..560).map(|k| (1, label(k), 50.0)));
        let m = minutes_at(&t);
        let b = compress(&m, 15); // 09:15 (minutes 0-14), 09:30 (15-29), next day 09:15
        let trade = |e: usize, z: usize, entry_px: f64, exit_px: f64, kind: &'static str| crate::connors::Trade {
            dir: 1., legs: vec![crate::connors::Leg { i: 0, px: entry_px, w: 1. }], exit_i: b.day[z] as usize, exit_px, reason: String::new(),
            entry_kind: kind, signal_i: 0, open: false, exits: Vec::new() };
        // close entry on candle 0 (09:29 close 114), target fill at 120 inside candle 1 (minute 09:34 has 118..120)
        let trades = vec![trade(0, 1, b.c[0], 120., "close")];
        // no capture active: nothing recorded
        record(&m, &b, 15, Align::Midnight, &trades, &[(0, 1)]);
        let ((), got) = capture_timings(|| record(&m, &b, 15, Align::Midnight, &trades, &[(0, 1)]));
        assert_eq!(got.len(), 1);
        let g = &got[0];
        assert!(g.matches(&trades[0]));
        assert_eq!((g.entry_bar, g.entry_fill, g.exit_bar, g.exit_fill, g.candles), (915, 929, 930, 934, 1));
        // AmiBroker candle convention: entry candle range (99..115) and the exit candle's open (114.5)
        assert_eq!((g.ab_low, g.ab_high, g.ab_exit_open), (99.0, 115.0, Some(114.5)));
        // minute path: minutes 09:30..09:33 fully (114..119) plus the open of 09:34 (118.5) and the fill 120
        assert_eq!((g.path_low, g.path_high), (Some(114.0), Some(120.0)));
        // an open-price entry fills in the candle's first minute; mismatched lists record nothing
        let open = vec![trade(0, 0, b.o[0], b.c[0], "open")];
        let ((), o) = capture_timings(|| record(&m, &b, 15, Align::Midnight, &open, &[(0, 0)]));
        assert_eq!((o[0].entry_fill, o[0].exit_fill, o[0].path_low, o[0].path_high), (915, 929, Some(99.0), Some(115.0)));
        let ((), none) = capture_timings(|| record(&m, &b, 15, Align::Midnight, &open, &[]));
        assert!(none.is_empty());
    }
}
