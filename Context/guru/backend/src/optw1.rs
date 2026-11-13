//! Lane F2 (13 Nov 2026, R4 #293): NIFTY weekly options on real 1-minute prices, for Singhal's option rules (books/as51ts.rs).
//!
//! Data: `<options dir>/../weekly1m/<TICKER>.w1.csv` + `.w1idx.csv` (Date,offset,bytes,expiry), built read-only from the
//! user's StockMock store (#Data/stores/NIFTY_full) by authors/titles-1030/F2w/build_w1.py: per trading date from 2019-02-11 the
//! nearest expiry's 1-minute bars (padded / zero-volume minutes dropped: a minute without a trade has no bar) of the strikes
//! within 300 points of the spot range of the day and the two trading days before, the out-of-the-money strikes beyond it that traded at
//! 10 or more that day, and the highest-open-interest call and put at 09:19; plus the NIFTY 50 spot minutes (contract 0).
//! Day block: `#,<date>,<expiry>,<n>`, `K,<ci>,<strike>,<CE|PE>,<oi at 09:19>`, then runs `R,<ci>,<m0>` of rows `a,b,c,d` in
//! paise (a = open - previous close, the run's first row: the open; b = high - open; c = open - low; d = close - open);
//! m = minutes after 09:15 (the bar stamped 09:15 = 0, 15:29 = 374). The engine reads one day at a time.
//!
//! Prices (the store's README): a bar stamped 09:16 covers 09:16:00-09:16:59, so the price at a clock time is that bar's OPEN;
//! when the contract did not trade in that minute, its last traded price (the close of its last earlier bar).
//! Orders (`Leg`): a timed order fills at the open of the contract's first traded minute at or after its time; a level order
//! (a trigger, a stop or a target) fills at its level when the price path reaches it, or at a minute's open when the price
//! jumps through it (no trade in between). Inside a minute the path runs open, the extreme nearer the open (ties: the high),
//! the other extreme, close (the vault's minute convention, intraday_ep6). A minute whose fills differ between its two extreme
//! orders is AMBIGUOUS (#175); it takes the convention and the leg is flagged.
use std::collections::BTreeMap;
use std::io::{Read, Seek, SeekFrom};
use std::sync::OnceLock;

/// Folder of the w1 files (set by main from the India options folder: its sibling `weekly1m`).
pub static DIR: OnceLock<String> = OnceLock::new();
static STORES: OnceLock<BTreeMap<String, Store>> = OnceLock::new();
static NEXT: OnceLock<BTreeMap<String, Store>> = OnceLock::new();
pub const SPOT: usize = 0;
pub const MINUTES: usize = 375;

pub struct Store { idx: BTreeMap<String, (u64, usize)>, path: std::path::PathBuf }

impl Store {
    pub fn load(dir: &str, ticker: &str) -> Option<Store> { Store::load_named(dir, ticker, "w1") }
    /// `kind` w1 = the nearest expiry of each date; w1n = the next weekly expiry on expiry days (lane F2, 6.13)
    pub fn load_named(dir: &str, ticker: &str, kind: &str) -> Option<Store> {
        let t = ticker.replace(' ', "_");
        let text = std::fs::read_to_string(format!("{dir}/{t}.{kind}idx.csv")).ok()?;
        let idx: BTreeMap<String, (u64, usize)> = text.lines().skip(1).filter_map(|l| {
            let mut f = l.split(',');
            Some((f.next()?.to_string(), (f.next()?.parse().ok()?, f.next()?.parse().ok()?)))
        }).collect();
        (!idx.is_empty()).then(|| Store { idx, path: format!("{dir}/{t}.{kind}.csv").into() })
    }
    pub fn first(&self) -> &str { self.idx.keys().next().map_or("", |s| s.as_str()) }
    pub fn last(&self) -> &str { self.idx.keys().next_back().map_or("", |s| s.as_str()) }
    pub fn has(&self, date: &str) -> bool { self.idx.contains_key(date) }
    /// The store's next `n` dates after `date` (its trading days).
    pub fn next_dates(&self, date: &str, n: usize) -> Vec<String> {
        self.idx.range::<str, _>((std::ops::Bound::Excluded(date), std::ops::Bound::Unbounded)).take(n).map(|(k, _)| k.clone()).collect()
    }
    pub fn day(&self, date: &str) -> Option<Day> {
        let &(off, n) = self.idx.get(date)?;
        let mut buf = vec![0u8; n];
        std::fs::File::open(&self.path).and_then(|mut f| { f.seek(SeekFrom::Start(off))?; f.read_exact(&mut buf) }).ok()?;
        Day::parse(std::str::from_utf8(&buf).ok()?)
    }
}

/// The next-weekly-expiry blocks of expiry days (`<TICKER>.w1n.csv`; None: not built).
pub fn store_next(ticker: &str) -> Option<&'static Store> {
    NEXT.get_or_init(|| {
        let mut m = BTreeMap::new();
        if let Some(d) = DIR.get() { for t in ["NIFTY 50"] { if let Some(s) = Store::load_named(d, t, "w1n") { m.insert(t.to_string(), s); } } }
        m
    }).get(ticker)
}

/// The store of one underlying (None: no w1 files for it, or the folder is not set).
pub fn store(ticker: &str) -> Option<&'static Store> {
    STORES.get_or_init(|| {
        let mut m = BTreeMap::new();
        if let Some(d) = DIR.get() { for t in ["NIFTY 50"] { if let Some(s) = Store::load(d, t) { m.insert(t.to_string(), s); } } }
        m
    }).get(ticker)
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct B { pub o: f64, pub h: f64, pub l: f64, pub c: f64 }

pub struct Day {
    pub expiry: u32,
    /// contract 1..=n: (strike, put, open interest at 09:19); index 0 = spot (unused entry)
    pub k: Vec<(f64, bool, f64)>,
    /// bars[ci][m]
    pub bars: Vec<Vec<Option<B>>>,
}

impl Day {
    pub fn parse(text: &str) -> Option<Day> {
        let mut lines = text.lines();
        let h: Vec<&str> = lines.next()?.split(',').collect();
        if h.first() != Some(&"#") { return None; }
        let n: usize = h.get(3)?.parse().ok()?;
        let mut d = Day { expiry: h.get(2)?.replace('-', "").parse().ok()?, k: vec![(f64::NAN, false, 0.0); n + 1],
            bars: vec![vec![None; MINUTES]; n + 1] };
        let (mut ci, mut m, mut pc) = (0usize, 0usize, 0i64);
        let mut first = true;
        // a malformed line makes the whole day unreadable (None): the caller reports it, nothing is guessed
        for l in lines {
            let f: Vec<&str> = l.split(',').collect();
            match f[0] {
                "K" => {
                    if f.len() != 5 { return None; }
                    let i: usize = f[1].parse().ok()?;
                    if i == 0 || i > n { return None; }
                    d.k[i] = (f[2].parse().ok()?, f[3] == "PE", f[4].parse().ok()?);
                }
                "R" => { if f.len() != 3 { return None; } ci = f[1].parse().ok()?; m = f[2].parse().ok()?; first = true; if ci > n { return None; } }
                _ => {
                    let v: Vec<i64> = f.iter().map(|s| s.parse().ok()).collect::<Option<_>>()?;
                    if v.len() != 4 { return None; }
                    let o = if first { v[0] } else { pc + v[0] };
                    let c = o + v[3];
                    if m < MINUTES { d.bars[ci][m] = Some(B { o: o as f64 / 100.0, h: (o + v[1]) as f64 / 100.0, l: (o - v[2]) as f64 / 100.0, c: c as f64 / 100.0 }); }
                    pc = c; m += 1; first = false;
                }
            }
        }
        Some(d)
    }
    /// A short day: fewer than 300 spot minutes inside 09:15-15:29 (a special session such as a Muhurat hour, or missing spot
    /// minutes: only 2026-09-30 in the store, 140 minutes). The option rules skip such days and say so on the next position.
    pub fn special(&self) -> bool { self.bars[SPOT].iter().filter(|z| z.is_some()).count() < 300 }
    /// Price known at clock minute `m`: the open of the bar at `m`, else the close of the last earlier bar.
    pub fn price_at(&self, ci: usize, m: usize) -> Option<f64> {
        let b = &self.bars[ci];
        if let Some(z) = b.get(m).copied().flatten() { return Some(z.o); }
        (0..m.min(MINUTES)).rev().find_map(|j| b[j]).map(|z| z.c)
    }
    pub fn last_close(&self, ci: usize) -> Option<f64> { self.bars[ci].iter().rev().find_map(|z| *z).map(|z| z.c) }
    pub fn ci(&self, strike: f64, put: bool) -> Option<usize> { (1..self.k.len()).find(|&i| self.k[i].0 == strike && self.k[i].1 == put) }
    /// Listed strikes of one type, ascending, with their contract index.
    pub fn strikes(&self, put: bool) -> Vec<(f64, usize)> {
        let mut v: Vec<(f64, usize)> = (1..self.k.len()).filter(|&i| self.k[i].1 == put).map(|i| (self.k[i].0, i)).collect();
        v.sort_by(|a, b| a.0.total_cmp(&b.0));
        v
    }
    /// At-the-money strike at minute m: of the strikes listed as both call and put, the one nearest the spot (tie: lower).
    pub fn atm(&self, m: usize) -> Option<f64> {
        let s = self.price_at(SPOT, m)?;
        let puts = self.strikes(true);
        self.strikes(false).into_iter().map(|x| x.0).filter(|k| puts.iter().any(|p| p.0 == *k))
            .fold(None, |b: Option<f64>, k| match b { Some(b) if (b - s).abs() <= (k - s).abs() => Some(b), _ => Some(k) })
    }
    /// Contract of type `put` whose price at minute m is nearest `target` (tie: the lower strike); `otm`: only strikes out of
    /// the money at that minute's spot (call above it, put below it).
    pub fn nearest_premium(&self, put: bool, m: usize, target: f64, otm: bool) -> Option<usize> {
        let s = self.price_at(SPOT, m)?;
        self.strikes(put).into_iter().filter(|(k, _)| !otm || if put { *k < s } else { *k > s })
            .filter_map(|(_, i)| self.price_at(i, m).map(|p| ((p - target).abs(), i)))
            .fold(None, |b: Option<(f64, usize)>, x| match b { Some(b) if b.0 <= x.0 => Some(b), _ => Some(x) }).map(|x| x.1)
    }
}

/// A leg's order state.
#[derive(Clone, Debug, PartialEq)]
pub enum St {
    /// a timed order: fills at the open of the first traded minute >= `at` (no fill after `until`)
    Timed { at: usize, until: usize },
    /// a level order: fills when the price reaches `lv` (rising: `up`), from minute `from` to `until` (inclusive)
    Trigger { lv: f64, up: bool, from: usize, until: usize },
    /// open position
    Open { entry: f64, m: usize },
    /// closed at `px` in minute `m`, why
    Done { px: f64, m: usize, why: String },
    /// the entry never filled
    Gone,
}

/// One option leg on one contract: quantity q (+1 bought, -1 sold), stop / target / trailing levels as fractions of the
/// entry price, a clock exit.
#[derive(Clone, Debug)]
pub struct Leg {
    pub ci: usize,
    pub q: f64,
    pub st: St,
    /// stop at entry x (1 + stop) for a sold leg, entry x (1 - stop) for a bought leg (NaN: none)
    pub stop: f64,
    /// target at entry x (1 + target) for a bought leg (NaN: none)
    pub target: f64,
    /// sold leg: once the price reaches entry x trail_at, the stop moves to entry x trail_to (NaN: none)
    pub trail_at: f64,
    pub trail_to: f64,
    /// clock exit at the open of the first traded minute >= exit_m (MINUTES: none today)
    pub exit_m: usize,
    pub exit_why: &'static str,
    pub amb: bool,
    /// minutes held: the clock exit is set at the fill (exit_m = fill minute + hold)
    pub hold: Option<usize>,
    /// the entry fill (price, minute), kept after the leg closes
    pub fill: Option<(f64, usize)>,
    /// bought leg: once started (start_trail), the stop follows frac x the highest price since (NaN: off)
    pub trail_frac: f64,
    /// last traded price of the contract seen by the leg (a level order's reference price at its start)
    pub prev: f64,
    /// working levels once open
    stop_lv: f64,
    target_lv: f64,
    trail_lv: f64,
}

impl Leg {
    pub fn new(ci: usize, q: f64, st: St) -> Leg {
        Leg { ci, q, st, stop: f64::NAN, target: f64::NAN, trail_at: f64::NAN, trail_to: f64::NAN, exit_m: MINUTES, exit_why: "", amb: false, hold: None, fill: None, trail_frac: f64::NAN,
            stop_lv: f64::NAN, target_lv: f64::NAN, trail_lv: f64::NAN, prev: f64::NAN }
    }
    pub fn entry(&self) -> Option<(f64, usize)> { if let St::Open { entry, m } = self.st { Some((entry, m)) } else { None } }
    pub fn active(&self) -> bool { matches!(self.st, St::Timed { .. } | St::Trigger { .. } | St::Open { .. }) }
    fn open_at(&mut self, px: f64, m: usize) {
        self.st = St::Open { entry: px, m };
        self.fill = Some((px, m));
        let s = if self.q < 0.0 { 1.0 } else { -1.0 };
        self.stop_lv = px * (1.0 + s * self.stop);
        self.target_lv = px * (1.0 + self.target);
        self.trail_lv = px * self.trail_at;
        if let Some(h) = self.hold { self.exit_m = m + h; }
    }
    /// Walk one piece of the path a -> b (gap: no trade in between, a level passed fills at b). Returns the fill events.
    fn seg(&mut self, a: f64, b: f64, gap: bool, m: usize, ev: &mut Vec<(u8, i64)>) {
        if a == b { return; }
        let up = b > a;
        let inside = |lv: f64| lv.is_finite() && if up { lv > a && lv <= b } else { lv < a && lv >= b };
        let at = |lv: f64| if gap { b } else { lv };
        match self.st.clone() {
            St::Trigger { lv, up: u, from, until } if m >= from && m <= until && u == up && inside(lv) => {
                let px = at(lv);
                ev.push((0, (px * 1e4).round() as i64));
                self.open_at(px, m);
                self.seg(px, b, false, m, ev);
            }
            St::Open { .. } => {
                let short = self.q < 0.0;
                // a sold leg: stop above (rising), trailing trigger below (falling); a bought leg: stop below, target above
                let (hit, why) = if short {
                    if up && inside(self.stop_lv) { (Some(self.stop_lv), "stop") }
                    else if !up && inside(self.trail_lv) { let t = self.trail_lv; self.stop_lv = self.trail_stop(); return self.seg(t, b, false, m, ev); }
                    else { (None, "") }
                } else if !up && inside(self.stop_lv) { (Some(self.stop_lv), "stop") } else if up && inside(self.target_lv) { (Some(self.target_lv), "target") } else {
                    if up && self.trail_frac.is_finite() { self.stop_lv = self.stop_lv.max(self.trail_frac * b); } // a new high raises the trailing stop
                    (None, "")
                };
                if let Some(lv) = hit {
                    let px = at(lv);
                    ev.push((1, (px * 1e4).round() as i64));
                    self.st = St::Done { px, m, why: why.into() };
                }
            }
            _ => {}
        }
    }
    /// The trailing trigger was reached: it is used up, and the stop moves to entry x trail_to.
    fn trail_stop(&mut self) -> f64 {
        let (entry, _) = self.entry().unwrap();
        self.trail_lv = f64::NAN;
        entry * self.trail_to
    }
    /// The bar at minute m of this leg's contract (None: no trade that minute).
    pub fn step(&mut self, m: usize, bar: Option<B>) {
        // a level order past its last minute is gone; a timed order past its window too
        match self.st {
            St::Trigger { until, .. } | St::Timed { until, .. } if m > until => { self.st = St::Gone; return; }
            _ => {}
        }
        let Some(z) = bar else { return };
        match self.st.clone() {
            St::Timed { at, .. } if m >= at => {
                self.open_at(z.o, m);
                self.prev = z.o;
            }
            St::Open { .. } if m >= self.exit_m => {
                self.st = St::Done { px: z.o, m, why: self.exit_why.into() };
                return;
            }
            St::Timed { .. } => return,
            _ => {}
        }
        if !self.active() { return; }
        // gap from the last traded price to this minute's open, then open -> nearer extreme -> other extreme -> close
        let p = if self.prev.is_finite() { self.prev } else { z.o };
        let mut ev = Vec::new();
        self.seg(p, z.o, true, m, &mut ev);
        let base = self.clone();
        let walk = |l: &mut Leg, hf: bool, ev: &mut Vec<(u8, i64)>| {
            let (x1, x2) = if hf { (z.h, z.l) } else { (z.l, z.h) };
            l.seg(z.o, x1, false, m, ev);
            l.seg(x1, x2, false, m, ev);
            l.seg(x2, z.c, false, m, ev);
        };
        let hf = z.h - z.o <= z.o - z.l;
        let (mut e1, mut e2) = (Vec::new(), Vec::new());
        let mut other = base.clone();
        walk(self, hf, &mut e1);
        walk(&mut other, !hf, &mut e2);
        if e1 != e2 { self.amb = true; }
        self.prev = z.c;
    }
    /// Start trailing a bought leg's stop at frac x its last traded price, then x every new high.
    pub fn start_trail(&mut self, frac: f64) {
        if !matches!(self.st, St::Open { .. }) || self.q < 0.0 { return; }
        self.trail_frac = frac;
        if self.prev.is_finite() { self.stop_lv = self.stop_lv.max(frac * self.prev); }
    }
    /// Close an open leg at `px` (end of the data / the day), minute m.
    pub fn close(&mut self, px: f64, m: usize, why: &str) { if matches!(self.st, St::Open { .. }) { self.st = St::Done { px, m, why: why.into() }; } }
    /// Continue an open leg on the next day (the overnight gap is walked from the previous close at the next traded minute).
    pub fn carry(&mut self) { if let St::Open { entry, .. } = self.st { self.st = St::Open { entry, m: 0 }; } }
}

/// Walk legs through one day, minute by minute (each leg on its own contract's bars); `each` runs after every minute and may
/// add legs (re-entries). Legs keep their state at the end of the day.
pub fn walk(d: &Day, legs: &mut Vec<Leg>, from: usize, each: &mut dyn FnMut(usize, &mut Vec<Leg>)) {
    for m in from..MINUTES {
        for l in legs.iter_mut() { if l.active() { let b = d.bars[l.ci][m]; l.step(m, b); } }
        each(m, legs);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn bar(o: f64, h: f64, l: f64, c: f64) -> Option<B> { Some(B { o, h, l, c }) }

    #[test]
    fn parse_delta_rows() {
        let d = Day::parse("#,2023-06-02,2023-06-08,2\nK,1,18500,CE,100\nK,2,18500,PE,200\nR,0,0\n1850000,500,1000,-200\n100,0,0,0\nR,1,1\n4000,100,50,-20\nR,2,3\n3000,0,0,10\n").unwrap();
        assert_eq!(d.expiry, 20230608);
        assert_eq!(d.bars[0][0], bar(18500.0, 18505.0, 18490.0, 18498.0));
        assert_eq!(d.bars[0][1], bar(18499.0, 18499.0, 18499.0, 18499.0));
        assert_eq!(d.bars[1][1], bar(40.0, 41.0, 39.5, 39.8));
        assert_eq!(d.price_at(1, 1), Some(40.0));
        assert_eq!(d.price_at(1, 5), Some(39.8)); // carried: last close
        assert_eq!(d.price_at(2, 2), None);
        assert_eq!(d.atm(0), Some(18500.0));
        assert_eq!(d.k[2], (18500.0, true, 200.0));
        assert!(d.special(), "2 spot minutes is not a normal session");
        // malformed blocks are unreadable, never half-read
        assert!(Day::parse("#,2023-06-02,2023-06-08,1\nK,2,18500,CE,100\n").is_none(), "contract index out of range");
        assert!(Day::parse("#,2023-06-02,2023-06-08,1\nK,1,18500,CE\n").is_none(), "short contract line");
        assert!(Day::parse("#,2023-06-02,2023-06-08,1\nR,3,0\n1,0,0,0\n").is_none(), "run of an unknown contract");
        assert!(Day::parse("#,2023-06-02,2023-06-08,1\nR,1,0\n1,0,0\n").is_none(), "short price row");
    }

    #[test]
    fn sold_leg_stop_trail_and_time_exit() {
        // sold at 40 at the open of minute 1; stop 2x = 80; at 20 (50%) the stop moves to the entry 40
        let mut l = Leg::new(1, -1.0, St::Timed { at: 1, until: 300 });
        (l.stop, l.trail_at, l.trail_to, l.exit_m, l.exit_why) = (1.0, 0.5, 1.0, 10, "15:15");
        l.step(1, bar(40.0, 41.0, 39.0, 40.0));
        assert_eq!(l.entry(), Some((40.0, 1)));
        l.step(2, bar(30.0, 31.0, 19.0, 25.0)); // reaches 20: stop -> 40
        l.step(3, bar(38.0, 41.0, 37.0, 39.0)); // 41 >= 40: stopped at 40
        assert_eq!(l.st, St::Done { px: 40.0, m: 3, why: "stop".into() });
        // gap through the stop fills at the minute's open; time exit at the open
        let mut l = Leg::new(1, -1.0, St::Timed { at: 0, until: 300 });
        (l.stop, l.exit_m, l.exit_why) = (1.0, 5, "15:15");
        l.step(0, bar(40.0, 40.0, 40.0, 40.0));
        l.step(2, bar(90.0, 95.0, 85.0, 90.0));
        assert_eq!(l.st, St::Done { px: 90.0, m: 2, why: "stop".into() });
        let mut l = Leg::new(1, -1.0, St::Timed { at: 0, until: 300 });
        (l.stop, l.exit_m, l.exit_why) = (1.0, 5, "15:15");
        l.step(0, bar(40.0, 40.0, 40.0, 40.0));
        l.step(6, bar(35.0, 36.0, 34.0, 35.0));
        assert_eq!(l.st, St::Done { px: 35.0, m: 6, why: "15:15".into() });
    }

    #[test]
    fn trigger_fills_on_the_path_and_flags_ambiguous_minutes() {
        // buy when the price rises to 55 (10% over 50); stop 20% (44), target 40% (77)
        let mk = || { let mut l = Leg::new(1, 1.0, St::Trigger { lv: 55.0, up: true, from: 5, until: 100 }); (l.stop, l.target) = (0.2, 0.4); l.prev = 50.0; l };
        let mut l = mk();
        l.step(5, bar(50.0, 56.0, 49.0, 55.5)); // high nearer the open: up to 56 fills at 55
        assert_eq!(l.entry(), Some((55.0, 5)));
        assert!(!l.amb);
        l.step(6, bar(60.0, 80.0, 59.0, 70.0)); // target 77
        assert_eq!(l.st, St::Done { px: 77.0, m: 6, why: "target".into() });
        // one minute reaching the trigger and the stop: high first (nearer) -> filled at 55, then down to 43 -> stopped at 44
        let mut l = mk();
        l.step(5, bar(50.0, 56.0, 43.0, 50.0));
        assert_eq!(l.st, St::Done { px: 44.0, m: 5, why: "stop".into() });
        assert!(l.amb, "low first would fill at 55 and never stop");
        // a gap above the trigger fills at the open
        let mut l = mk();
        l.step(7, bar(58.0, 58.0, 57.0, 57.5));
        assert_eq!(l.entry(), Some((58.0, 7)));
        // past its window the order is gone
        let mut l = mk();
        l.step(101, bar(60.0, 60.0, 60.0, 60.0));
        assert_eq!(l.st, St::Gone);
    }

    #[test]
    fn bought_leg_trailing_stop_follows_new_highs() {
        // bought at 100, stop 50%; trailing at half the price from 170, then from the new high 200 (stop 100)
        let mut l = Leg::new(1, 1.0, St::Timed { at: 0, until: 10 });
        l.stop = 0.5;
        l.step(0, bar(100.0, 100.0, 100.0, 100.0));
        l.step(1, bar(170.0, 170.0, 170.0, 170.0));
        l.start_trail(0.5);
        l.step(2, bar(180.0, 200.0, 180.0, 190.0));
        assert!(matches!(l.st, St::Open { .. }));
        l.step(3, bar(150.0, 150.0, 99.0, 120.0)); // opens at 150 (> 100), falls through 100
        assert_eq!(l.st, St::Done { px: 100.0, m: 3, why: "stop".into() });
    }
}
