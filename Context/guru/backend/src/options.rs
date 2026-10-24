//! Options phase 2: real end-of-day prices of the exact option contracts a rule picks, read one day at a time, and a
//! small account that values every open option position at the held contract's own price each day.
//!
//! Data (built offline by library/options/build_{india,us}.py, staging only): `<dir>/<TICKER>.opt.csv` sorted by
//! Date, Expiry, Type, Strike, and `<dir>/<TICKER>.optidx.csv` (Date,offset,bytes). Monthly expiries only.
//!   India (NSE F&O bhavcopy): Date,Expiry,Type,Strike,Open,High,Low,Close,Settle,Contracts,OI
//!   US (SPY/QQQ/IWM 16:00 ET snapshot): Date,Expiry,Type,Strike,Bid,Ask,Last,Volume,OI
//! Price rules (user rules, 2026-10-24):
//!   * a FILL needs a row that traded: India Contracts > 0 (price = that day's Close, or Open for next-open rules);
//!     US volume > 0 and a two-sided quote 0 < bid <= ask (price = mid (bid+ask)/2, labelled; no bid/ask edge added).
//!     An untraded row never fills: India's printed Close of an untraded row is not a price and is never read.
//!   * a MARK (daily valuation of an open position) uses the fill price when the contract traded, else India's exchange
//!     settlement price / the US two-sided mid (both labelled), else the last mark (carried, counted).
//!   * the last trading day of a contract with no traded price: final settlement at intrinsic value against the
//!     underlying's official close (NSE index options are cash-settled on the index close; SPY options are
//!     physically settled at the close, same value). No model price is ever used.
//! Memory: only the day index is held; `Window` keeps the last few dozen parsed days of one run.
use crate::connors::{Ctx, Leg, Trade};
use std::collections::{BTreeMap, VecDeque};
use std::io::{Read, Seek, SeekFrom};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Q {
    pub expiry: u32,
    pub put: bool,
    pub strike: f64,
    pub us: bool,
    pub open: f64,
    pub high: f64,
    pub close: f64,
    pub settle: f64,
    pub bid: f64,
    pub ask: f64,
    pub volume: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Mark { Traded, Settlement, Quote }

impl Q {
    pub fn traded(&self) -> bool { self.volume > 0.0 }
    fn mid(&self) -> Option<f64> { (self.bid > 0.0 && self.ask >= self.bid).then(|| (self.bid + self.ask) / 2.0) }
    /// Fill at the day's close (US: the 16:00 mid).
    pub fn fill_close(&self) -> Option<f64> {
        if !self.traded() { return None; }
        if self.us { self.mid() } else { (self.close > 0.0).then_some(self.close) }
    }
    /// Fill at the day's open (India only; the US file has no option open).
    pub fn fill_open(&self) -> Option<f64> { (!self.us && self.traded() && self.open > 0.0).then_some(self.open) }
    /// The day's traded high (India only).
    pub fn day_high(&self) -> Option<f64> { (!self.us && self.traded() && self.high > 0.0).then_some(self.high) }
    pub fn mark(&self) -> Option<(f64, Mark)> {
        if let Some(p) = self.fill_close() { return Some((p, Mark::Traded)); }
        if self.us { self.mid().map(|p| (p, Mark::Quote)) } else { (self.settle > 0.0).then_some((self.settle, Mark::Settlement)) }
    }
}

pub struct Chain { idx: BTreeMap<String, (u64, usize)>, path: std::path::PathBuf, us: bool }

impl Chain {
    /// None when the ticker has no option files (every option rule then reports a data gap).
    pub fn load(dir: &str, ticker: &str) -> Option<Chain> {
        let t = ticker.replace(' ', "_");
        let path = std::path::PathBuf::from(format!("{dir}/{t}.opt.csv"));
        let mut head = String::new();
        std::io::BufRead::read_line(&mut std::io::BufReader::new(std::fs::File::open(&path).ok()?), &mut head).ok()?;
        let us = head.contains(",Bid,");
        let text = std::fs::read_to_string(format!("{dir}/{t}.optidx.csv")).ok()?;
        let idx: BTreeMap<String, (u64, usize)> = text.lines().skip(1).filter_map(|l| {
            let mut f = l.split(',');
            Some((f.next()?.to_string(), (f.next()?.parse().ok()?, f.next()?.parse().ok()?)))
        }).collect();
        (!idx.is_empty()).then_some(Chain { idx, path, us })
    }
    pub fn first(&self) -> &str { self.idx.keys().next().map_or("", |s| s.as_str()) }
    pub fn last(&self) -> &str { self.idx.keys().next_back().map_or("", |s| s.as_str()) }
    /// The day's chain sorted by (expiry, put/call, strike); empty when the day has no rows.
    pub fn day(&self, date: &str) -> Vec<Q> {
        let Some(&(off, n)) = self.idx.get(date) else { return vec![] };
        let mut buf = vec![0u8; n];
        let ok = std::fs::File::open(&self.path).and_then(|mut f| { f.seek(SeekFrom::Start(off))?; f.read_exact(&mut buf) });
        if ok.is_err() { return vec![]; }
        let us = self.us;
        let mut v: Vec<Q> = std::str::from_utf8(&buf).unwrap_or("").lines().filter_map(|l| {
            let f: Vec<&str> = l.split(',').collect();
            if f.first() != Some(&date) { return None; }
            let n = |k: usize| f.get(k).and_then(|s| s.parse::<f64>().ok()).unwrap_or(f64::NAN);
            let (expiry, put, strike) = (f.get(1)?.parse().ok()?, *f.get(2)? == "P", n(3));
            Some(if us {
                Q { expiry, put, strike, us, open: f64::NAN, high: f64::NAN, close: f64::NAN, settle: f64::NAN, bid: n(4), ask: n(5), volume: n(7) }
            } else {
                Q { expiry, put, strike, us, open: n(4), high: n(5), close: n(7), settle: n(8), bid: f64::NAN, ask: f64::NAN, volume: n(9) }
            })
        }).collect();
        v.sort_by(|a, b| (a.expiry, a.put).cmp(&(b.expiry, b.put)).then(a.strike.total_cmp(&b.strike)));
        v
    }
}

/// The parsed chains of the last `keep` bars of one run (each day is read once).
pub struct Window<'a> { chain: &'a Chain, x: &'a Ctx<'a>, keep: usize, days: VecDeque<(usize, Vec<Q>)> }

impl<'a> Window<'a> {
    pub fn new(chain: &'a Chain, x: &'a Ctx<'a>, keep: usize) -> Self { Window { chain, x, keep, days: VecDeque::new() } }
    /// Chain of bar i (bars must be requested in increasing order; older bars inside `keep` stay readable).
    pub fn at(&mut self, i: usize) -> &[Q] {
        if !self.days.iter().any(|d| d.0 == i) {
            self.days.push_back((i, self.chain.day(&self.x.r[i].date)));
            while self.days.len() > self.keep { self.days.pop_front(); }
        }
        &self.days.iter().find(|d| d.0 == i).unwrap().1
    }
    /// The quote of one contract on bar i (None: not listed that day / outside the window).
    pub fn quote(&mut self, i: usize, c: Contract) -> Option<Q> { find(self.at(i), c).copied() }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Contract { pub expiry: u32, pub put: bool, pub strike: f64 }

pub fn find(day: &[Q], c: Contract) -> Option<&Q> {
    day.binary_search_by(|q| (q.expiry, q.put).cmp(&(c.expiry, c.put)).then(q.strike.total_cmp(&c.strike))).ok().map(|k| &day[k])
}
/// Listed monthly expiries of the day, ascending.
pub fn expiries(day: &[Q]) -> Vec<u32> {
    let mut v: Vec<u32> = day.iter().map(|q| q.expiry).collect();
    v.dedup();
    v
}
/// Listed strikes of one expiry and type, ascending.
pub fn strikes(day: &[Q], expiry: u32, put: bool) -> Vec<f64> {
    day.iter().filter(|q| q.expiry == expiry && q.put == put).map(|q| q.strike).collect()
}
/// Listed strike nearest `px` (a tie goes to the lower strike).
pub fn nearest(strikes: &[f64], px: f64) -> Option<f64> {
    strikes.iter().copied().fold(None, |best: Option<f64>, k| match best {
        Some(b) if (b - px).abs() <= (k - px).abs() => Some(b),
        _ => Some(k),
    })
}

pub fn ymd(date: &str) -> u32 { date.replace('-', "").parse().unwrap_or(0) }
/// Calendar day number of a YYYYMMDD value.
pub fn day_no(v: u32) -> i64 {
    let v = v as i64;
    let (y, m, d) = (v / 10000, v / 100 % 100, v % 100);
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * (m + if m > 2 { -3 } else { 9 }) + 2) / 5 + d - 1;
    era * 146_097 + yoe * 365 + yoe / 4 - yoe / 100 + doy
}
/// Monday (day number) of the Mon..Sun calendar week holding a YYYYMMDD date.
pub fn week_of(v: u32) -> i64 { let n = day_no(v); n - (n + 2).rem_euclid(7) }
/// Bar index of the contract's last trading day: the last underlying bar dated on or before the expiry date, known
/// only when a later bar exists or that bar is the expiry date itself.
pub fn last_day(x: &Ctx, expiry: u32) -> Option<usize> {
    let k = x.r.partition_point(|r| ymd(&r.date) <= expiry);
    (k > 0 && (k < x.r.len() || ymd(&x.r[k - 1].date) == expiry)).then(|| k - 1)
}
/// YYYYMMDD of the contract's last trading day (the expiry date itself when the data end earlier).
pub fn ltd(x: &Ctx, expiry: u32) -> u32 { last_day(x, expiry).map_or(expiry, |k| ymd(&x.r[k].date)) }
pub fn intrinsic(c: Contract, s: f64) -> f64 { if c.put { (c.strike - s).max(0.0) } else { (s - c.strike).max(0.0) } }

pub enum Exit { Close, Open }

/// Follow one option leg entered on bar i at `px0` (q = -1 sold, +1 bought) until `rule(k)` asks to leave on a later
/// bar k (Close = that day's close, Open = its open; an untraded exit day leaves at the next traded close, counted as
/// delayed), its last trading day (final settlement at intrinsic value on the underlying close), or bar `to` (open,
/// marked). Returns (exit bar, P&L points per bar i..=exit, how it ended, open, marks).
pub fn follow(w: &mut Window, x: &Ctx, c: Contract, q: f64, i: usize, px0: f64, to: usize, rule: &mut dyn FnMut(usize) -> Option<(Exit, String)>) -> (usize, Vec<f64>, String, bool, Marks) {
    let (mut pnl, mut marks, mut last) = (vec![0.0], Marks::default(), px0);
    let end = last_day(x, c.expiry);
    let mut pending: Option<String> = None;
    for k in i + 1..=to {
        let qt = w.quote(k, c);
        let m = qt.and_then(|q| q.mark());
        marks.add(m.map(|m| m.1));
        if let Some((p, _)) = m { last = p; }
        let close = qt.and_then(|q| q.fill_close());
        let mut done: Option<(f64, String)> = None;
        if let Some(why) = pending.take() {
            match close { Some(p) => done = Some((p, format!("{why} (filled on the next traded close)"))), None => pending = Some(why) }
        } else if let Some((how, why)) = rule(k) {
            match how {
                Exit::Close => match close { Some(p) => done = Some((p, why)), None => pending = Some(why) },
                Exit::Open => match (qt.and_then(|q| q.fill_open()), close) {
                    (Some(p), _) => done = Some((p, why)),
                    (None, Some(p)) => done = Some((p, format!("{why} (no traded open: filled on the close)"))),
                    _ => pending = Some(why),
                },
            }
        }
        if done.is_none() && end == Some(k) {
            let v = intrinsic(c, x.c[k]);
            done = Some((v, format!("final settlement {v:.2} on the last trading day{}", pending.take().map_or(String::new(), |w| format!(" ({w}: no traded price before it)")))));
        }
        if let Some((p, why)) = done {
            pnl.push(q * (p - px0));
            return (k, pnl, why, false, marks);
        }
        pnl.push(q * (last - px0));
        if k == to { return (k, pnl, "open".into(), true, marks); }
    }
    (i, pnl, "open".into(), true, marks)
}

/// One option position (any legs), valued in underlying price points per one unit of the underlying.
pub struct Pos {
    /// +1 when the position gains as the underlying rises (short puts, covered calls), -1 otherwise (display side).
    pub dir: f64,
    /// underlying close at entry: the position's notional (one option per unit of the underlying)
    pub s0: f64,
    pub entry_i: usize,
    pub exit_i: usize,
    /// cumulative P&L in points for bars entry_i..=exit_i (last = final; finite)
    pub pnl: Vec<f64>,
    pub reason: String,
    pub open: bool,
}

/// Mark counter for a position's reason text.
#[derive(Default)]
pub struct Marks { pub traded: usize, pub settle: usize, pub quote: usize, pub carried: usize }
impl Marks {
    pub fn add(&mut self, m: Option<Mark>) {
        match m { Some(Mark::Traded) => self.traded += 1, Some(Mark::Settlement) => self.settle += 1, Some(Mark::Quote) => self.quote += 1, None => self.carried += 1 }
    }
    pub fn text(&self) -> String {
        format!("marks: {} traded, {} settlement, {} quote-only, {} carried", self.traded, self.settle, self.quote, self.carried)
    }
}

#[derive(Clone, Debug)]
pub struct Account { pub trades: Vec<Trade>, pub alloc: Vec<f64>, pub equity: Vec<f64>, pub gross: Vec<f64>, pub ruin: Option<usize> }

pub const BASIS: &str = "Options phase 2 account. Real end-of-day prices of the exact option contracts the rule picks (NSE F&O bhavcopy for India; SPY/QQQ/IWM 16:00 ET option snapshot for the US). A fill needs a contract that traded that day: India at its close (or open), US at the 16:00 mid (bid+ask)/2 with volume > 0 (labelled; no bid/ask edge added). Open positions are valued every day at the held contract's own price: traded close, else the exchange settlement price (India) or the two-sided mid (US), else the last value; counts are in each trade's reason. A contract held to its last trading day settles at intrinsic value on the underlying's official close. Each position is one option per unit of the underlying, sized at the account equity on its entry close; overlapping positions are carried together and the equity curve, drawdown and risk statistics use these daily option values. Trade return = option points / underlying close at entry. Costs: the books give none; the vault fee and slippage settings are charged per side on each position's notional (0 by default). No option price is modelled, filled in or repaired; book performance is not reproduced (different market and period).";

/// Option positions of an option rule (None = not an options phase 2 rule; Err = data gap for this instrument).
pub fn positions(id: &str, x: &Ctx, from: usize, to: usize, p: &crate::connors::Params) -> Option<Result<Vec<Pos>, String>> {
    use crate::books::*;
    oiss::option_positions(id, x, from, to, p)
        .or_else(|| otvt::option_positions(id, x, from, to, p))
        .or_else(|| ccl08::option_positions(id, x, from, to, p))
        .or_else(|| mahrs::option_positions(id, x, from, to, p))
        .or_else(|| spmt::option_positions(id, x, from, to, p))
}
pub fn is_option_rule(id: &str) -> bool { RULES.contains(&id) }
/// Every rule that runs on real option prices (instrument_policy admits each on its own instruments).
pub const RULES: &[&str] = &["oiss_short_put", "oiss_short_call", "oiss_plus_three_put", "oiss_plus_three_call", "otvt_cvr1_sell_puts",
    "ccl08_weekly_rsi_options", "mahrs_covered_calls", "spmt_eom_puts"];

/// Overlapping positions on one account. Each position is sized at the account equity on its entry close (one option
/// per unit of the underlying, notional = that equity); its value moves with its own daily P&L / s0. Costs: `cost`
/// per side of the position's notional (the vault's user cost setting; 0 by default, books give none). If equity
/// reaches 0 the account is wiped out: later positions are dropped and open ones end that day.
pub fn account(mut pos: Vec<Pos>, from: usize, to: usize, cost: f64) -> Account {
    pos.retain(|p| p.entry_i >= from && p.exit_i <= to && p.exit_i >= p.entry_i && p.pnl.len() == p.exit_i - p.entry_i + 1 && p.pnl.iter().all(|v| v.is_finite()) && p.s0 > 0.0);
    pos.sort_by_key(|p| (p.entry_i, p.exit_i));
    let run = |cost: f64, pos: &[Pos]| {
        let n = to - from + 1;
        let (mut eq, mut alloc) = (vec![0.0; n], vec![0.0; pos.len()]);
        let mut cash = 1.0;
        let mut ruin = None;
        for k in from..=to {
            // value of positions opened earlier, then realize exits, then size today's entries
            let open_val = |alloc: &[f64]| pos.iter().enumerate()
                .filter(|(j, p)| p.entry_i < k && k <= p.exit_i && alloc[*j] > 0.0)
                .map(|(j, p)| alloc[j] * (p.pnl[k - p.entry_i] / p.s0 - cost - if k == p.exit_i { cost } else { 0.0 }))
                .sum::<f64>();
            let e = cash + open_val(&alloc);
            for (j, p) in pos.iter().enumerate() {
                if p.entry_i < k && p.exit_i == k && alloc[j] > 0.0 { cash += alloc[j] * (p.pnl[k - p.entry_i] / p.s0 - 2.0 * cost); }
            }
            let mut e2 = e;
            for (j, p) in pos.iter().enumerate() {
                if p.entry_i == k && e > 0.0 {
                    alloc[j] = e;
                    let v = e * (p.pnl.last().copied().unwrap_or(0.0) / p.s0 - 2.0 * cost);
                    if p.exit_i == k { cash += v; e2 += v; } else { e2 += e * (p.pnl[0] / p.s0 - cost); }
                }
            }
            eq[k - from] = e2;
            if e2 <= 0.0 { ruin = Some(k); eq[k - from..].fill(0.0); break; }
        }
        (eq, alloc, ruin)
    };
    let (equity, alloc, ruin) = run(cost, &pos);
    let (gross, _, _) = run(0.0, &pos);
    let mut trades = Vec::new();
    let mut allocs = Vec::new();
    for (j, p) in pos.iter().enumerate() {
        if alloc[j] <= 0.0 { continue; }
        let (exit_i, last, open) = match ruin {
            Some(k) if p.exit_i > k => (k, p.pnl[k - p.entry_i], false),
            _ => (p.exit_i, *p.pnl.last().unwrap(), p.open),
        };
        let ret = last / p.s0;
        trades.push(Trade {
            dir: p.dir, legs: vec![Leg { i: p.entry_i, px: p.s0, w: 1.0 }], exit_i, exit_px: p.s0 * (1.0 + p.dir * ret),
            reason: p.reason.clone(), entry_kind: "close", signal_i: exit_i, open, exits: Vec::new(),
        });
        allocs.push(alloc[j]);
    }
    Account { trades, alloc: allocs, equity, gross, ruin }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pos(e: usize, x: usize, pnl: &[f64]) -> Pos {
        Pos { dir: 1.0, s0: 100.0, entry_i: e, exit_i: x, pnl: pnl.to_vec(), reason: String::new(), open: false }
    }

    #[test]
    fn sequential_positions_compound_like_the_single_position_engine() {
        // +10 points then -5 points on 100: 1.10 then 1.10 * 0.95
        let a = account(vec![pos(0, 2, &[0.0, 4.0, 10.0]), pos(2, 4, &[0.0, 1.0, -5.0])], 0, 4, 0.0);
        let want = [1.0, 1.04, 1.10, 1.10 * 1.01, 1.10 * 0.95];
        assert!(a.equity.iter().zip(want).all(|(a, b)| (a - b).abs() < 1e-12), "{:?}", a.equity);
        assert_eq!(a.alloc, vec![1.0, 1.10]);
        assert!((a.trades[1].ret(0.0) + 0.05).abs() < 1e-12);
    }

    #[test]
    fn overlapping_positions_are_marked_together() {
        // second position opens on bar 1 while the first is open: allocation = equity that day (1.02)
        let a = account(vec![pos(0, 3, &[0.0, 2.0, 3.0, 6.0]), pos(1, 2, &[0.0, -1.0])], 0, 3, 0.0);
        let e1 = 1.02;
        assert!((a.equity[1] - e1).abs() < 1e-12);
        assert!((a.equity[2] - (1.0 + 0.03 - e1 * 0.01)).abs() < 1e-12);
        assert!((a.equity[3] - (1.0 + 0.06 - e1 * 0.01)).abs() < 1e-12);
    }

    #[test]
    fn wipe_out_stops_the_account() {
        let a = account(vec![pos(0, 3, &[0.0, -50.0, -120.0, -10.0]), pos(3, 4, &[0.0, 1.0])], 0, 4, 0.0);
        assert_eq!(a.ruin, Some(2));
        assert_eq!(a.trades.len(), 1);
        assert_eq!(a.trades[0].exit_i, 2);
        assert_eq!(&a.equity[2..], &[0.0, 0.0, 0.0]);
    }

    #[test]
    fn fills_never_use_untraded_rows() {
        let india = Q { expiry: 20200326, put: true, strike: 9000.0, us: false, open: 0.0, high: 0.0, close: 866.7, settle: 120.5, bid: f64::NAN, ask: f64::NAN, volume: 0.0 };
        assert_eq!(india.fill_close(), None);
        assert_eq!(india.fill_open(), None);
        assert_eq!(india.mark(), Some((120.5, Mark::Settlement)));
        let us = Q { us: true, bid: 1.0, ask: 1.2, volume: 0.0, ..india };
        assert_eq!(us.fill_close(), None);
        assert_eq!(us.mark().map(|m| m.1), Some(Mark::Quote));
        assert!((Q { volume: 3.0, ..us }.fill_close().unwrap() - 1.1).abs() < 1e-12);
        assert_eq!(Q { volume: 3.0, bid: 0.0, ..us }.fill_close(), None, "one-sided quote: no fill");
    }

    #[test]
    fn strike_and_calendar_helpers() {
        assert_eq!(nearest(&[95.0, 100.0, 105.0], 102.5), Some(100.0));
        assert_eq!(nearest(&[95.0, 100.0, 105.0], 102.6), Some(105.0));
        assert_eq!(week_of(20240315), week_of(20240311)); // Fri and Mon of one week
        assert_ne!(week_of(20240317), week_of(20240318)); // Sun / Mon
    }
}
