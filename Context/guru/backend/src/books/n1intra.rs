//! Lane K46 (4-5 Nov 2026, decision #216; authors/titles-1030/K46.tsv): intraday rules of batch N1 (authors/batch-n/N1.tsv) on
//! N-minute bars built from the vault's 1-minute file (intraday_ep::day_bars, session-aligned), one series across days (EP2).
//! Four sources, four books:
//! - `hhoct` Octane System (hungryhippo.com, version 2002.2.15), research share USB HDD 1/TradeStation Systems & Indicators/
//!   Octane System for TS (hungryhippo.com)/Open Code.rar::Open Code.txt (whole code).
//! - `bbgf` Bob Buran, G-Force (Buran Systems Pack for TS), .../Bob Buran Systems Pack for TS/Buran G-Force System.rar::
//!   Buran G-Force System.txt (whole code; long side only, as its header advises).
//! - `twsifp` SP Instant Futures Profit (Trade With Success LLC, 2003), .../SP Instant Futures Profit System for TS + Manual
//!   (tradewithsuccess.com)/Open Code.rar::Open Code.txt (whole code).
//! - `jraeb` Andy's E-mini Bar, 60-minute (Joe Ross / Trading Educators, 2005), Trading Courses,Seminars,Videos/Joe Ross -
//!   Andy's EMini Bar - 60 Min System/jross60min.pdf, 'Trading Rules' (PDF p.5-6).
//! Independent reference: authors/titles-1030/K46w/k46_ref.py (written from the same sources).
use crate::connors::{wilder, Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use crate::intraday::{day_trade, Bar1};

macro_rules! def {
    ($id:expr, $book:expr, $name:expr, $group:expr, $dir:expr, $rules:expr, $notes:expr) => {
        StrategyDef { id: $id, name: $name, group: $group, session: 0, dir: $dir, status: Status::Live, needs_volume: false,
            needs_vix: false, needs_series: &[], book: $book, rules: $rules, notes: $notes, claim: None }
    };
}
const MIN: &str = "A|Minute path [convention]: a stop or limit fills at its level, or at the minute's open when the minute opens through it; inside a minute the price runs open, the extreme nearer the open, the farther extreme, close (a tie: the extreme against the position first; flat: low first). A minute whose fills differ between the two orders is counted as ambiguous (decision 175: held above 5%).";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![
        def!("hhoct_octane", "hhoct", "Octane pivot breakout (5-minute S&P)", "Octane System (hungryhippo.com)", "Long/Short",
            &["F|\"If Date<>Date[1] then begin Pivot = (HighToday+LowToday+C[1])/3 ; Value1 = 2*Pivot ; S1 = (Value1 - HighToday); R1 = (Value1 - LowToday); HighToday = H ; LowToday = L ; TradeCounter = 0 ; End;\" (Open Code.txt)",
              "F|\"If ADX(14)> 25 then begin Condition1 = ( HighToday >= R1 or LowToday <= S1 ) ; If H > HighToday then HighToday = H ; If L < LowToday then LowToday = L ; Monkey = t >= (sess1starttime+70) and t <= (sess1endtime-100) ;\" - as printed, the running high/low is updated only while ADX(14) > 25",
              "E|\"If Close > R1 and marketposition <> 1 then begin Buy(\"Go_Long\") Next Bar at Open\"; \"If Close < S1 and marketposition <> -1 then begin Sell Short (\"Go_Short\") Next Bar at Open\"; TradeCounter <= 0: one trade a day (Open Code.txt)",
              "X|\"setexitonclose\": out at the day's last 5-minute close",
              "A|Read: \"Use 5 Minute Bars Day Session Data\": session-aligned 5-minute bars from the 1-minute file (09:30-16:00 ET), ADX(14) = Wilder ADX on the one continuous 5-minute series [Declared reading]. TradeStation HHMM arithmetic makes the window 10:00-15:00 ET (the comment says 3:15; the code is followed); NSE: from 30 minutes after the open to 60 minutes before the close [Declared reading]. An entry signal on a day's last bar has no next bar that day and is dropped [Declared reading]. The vendor names the S&P; no fixed amounts, so it runs on SPY, SPX, NIFTY 50 and NIFTYBANK as a declared application.", MIN],
            "No published results. No costs in the vault's run. Cash-index shorts are hypothetical."),
        def!("bbgf_gforce", "bbgf", "G-Force spike long (10-minute bars)", "Buran Systems Pack", "Long",
            &["F|\"Time2 = CalcTime(Sess1StartTime, 130); {end of first third of trading day-11:40EST} Time3 = CalcTime(Sess1StartTime, 260); Time5 = CalcTime(Sess1StartTime, 390)\" (Buran G-Force System.txt)",
              "E|\"If Time = Time2 then begin Spike = (AvLo- LowestLow)/ (Average(Range, LkBck))*100; If Spike >= SpikTrig and Close > Close[4] and LowestLow < OpenD(0) then buy (\"SpikeLE\") market\"; AvLo = Average(Low, 13), LowestLow = Lowest(Low, 13), SpikTrig 150",
              "X|\"If Time > Time2 and Time <= Time3 ... exitlong (\"SLLX2\") at AvLo - SLM*(Average(Range, 13)) stop\" (SLM .3); \"If Time > Time3 and Time < Time5 ... exitlong (\"SLLX3\") at entryprice stop\"; \"If Time >= Time5 {leaves 15 minutes to exit} ... exitlong (\"MOCLX\")\"",
              "A|Read: \"Data1 = 10 min bars\": session-aligned 10-minute bars from the 1-minute file, one continuous series; Time2/3/5 = 130/260/390 minutes after the session open on every market; the market order fills at the next bar's open and each stop works through the next bar [Declared reading]. Long only: the header says \"Use system for long only\" and the short entry is commented out. The final exit is at the session's last 10-minute close: the code's comments put it before the close (\"leaves 15 minutes to exit\"), while a market order on the 16:00 bar would fill at the next session's open [Declared reading]. No fixed amounts and no market named: SPY, SPX, NIFTY 50, NIFTYBANK as a declared application.", MIN],
            "Data2 (60-minute) and Data3 (daily) feed only the commented-out short side. No published results. No costs in the vault's run."),
        def!("twsifp_cci", "twsifp", "Instant Futures Profit CCI/ADX entry (30-minute S&P)", "SP Instant Futures Profit (Trade With Success)", "Long/Short",
            &["E|\"if time>=ss and time<=ee then begin if c>opend(0) and CCI(cci_length) crosses above 0 and c>xaverage(c,xma_length) and adx(15)<25 then Buy size contracts next bar at c limit\"; short mirror (\"c<opend(0) and CCI(cci_length) crosses below 0 and c<xaverage(c,xma_length) and adx(15)<25 then Sell Short ... next bar at c limit\"); ss 0930, ee 1200, xma_length 14, cci_length 14 (Open Code.txt)",
              "X|\"if marketposition>0 and cci(cci_length)<0 then Sell this bar close; if marketposition<0 and cci(cci_length)>0 then Buy to Cover this bar close; if time>=extrade then begin; Sell this bar close; Buy to Cover this bar close\"; extrade 1600; setexitonclose",
              "A|Read: \"SETUP- 30 MIN BARS, ES DAY SESSION\": session-aligned 30-minute bars from the 1-minute file, one continuous series; CCI = typical-price CCI with the mean deviation, XAverage seeded with the first close, ADX = Wilder ADX(15); a cross = beyond 0 now and at or on the other side of 0 on the bar before [Declared reading]. The limit order lives for the next bar only; on NSE the window is the open to 150 minutes after it and the day ends at the close [Declared reading]. Fixed-fractional sizing is off by default (ffs false): one unit. No fixed amounts: SPY, SPX, NIFTY 50, NIFTYBANK as a declared application.", MIN],
            "The vendor's manual is not in the folder; no published results. No costs in the vault's run. Cash-index shorts are hypothetical."),
        def!("jraeb_60min", "jraeb", "Andy's E-mini Bar 60-minute fade (S&P points)", "Andy's E-mini Bar (Trading Educators)", "Long/Short",
            &["F|\"The 60 minute bar we use for the entry and target is called 'Trigger Bar'. This is the bar from 8:30 am till 9:29:59 am U.S. Central Time. The Trigger Bar must show a distance between the high and the low of at least 12 ticks!\" (jross60min.pdf, Trading Rules)",
              "E|\"Long Signal: Buy at the low of the Trigger Bar using a Limit Order - but not before 9:30 am and not later then 10:29:59 am, U.S. Central Time. Short Signal: Sell at the high of the Trigger Bar using a Limit Order\"; \"Do NOT enter any positions after 10:30 am U.S. Central Time. Cancel your orders if not filled!\"",
              "X|\"If you are long: Place a sell limit order 8 ticks below the high of the Trigger Bar. If you are short: Place a buy limit order 8 ticks above the low of the Trigger Bar. Time Stop: If not filled by 12:30 pm U.S. Central Time, exit the trade via a market order\"; no protective stop is given; \"We do NOT hold any position overnight!\"",
              "A|Read: ES ticks are 0.25 S&P index points, so 12 ticks = 3.00 and 8 ticks = 2.00 S&P points, run on SPX only (decision 196(2)); Central Time + 1 hour = the 09:30-10:29 ET trigger bar, entries 10:30-11:29 ET, the time stop at the 13:30 ET minute's open [Declared reading]. One trade a day: the first entry fill cancels the other entry order [Declared reading].", MIN],
            "The manual's equity curve (Nov 2004 - Nov 2005, one ES contract: over $8,700, more than 70% winners, largest loss $500) is before the vault's SPX minute history (2008-). No costs in the vault's run. Cash-index shorts are hypothetical."),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "hhoct_octane" => ("hhoct", "Octane System (hungryhippo.com)", "5-minute pivot breakout"),
        "bbgf_gforce" => ("bbgf", "G-Force (Bob Buran)", "10-minute spike long"),
        "twsifp_cci" => ("twsifp", "SP Instant Futures Profit", "30-minute CCI/ADX limit entry"),
        "jraeb_60min" => ("jraeb", "Andy's E-mini Bar", "60-minute trigger bar fade"),
        _ => return None,
    })
}

fn pd(key: &'static str, label: &'static str, def: f64, min: f64, max: f64, step: f64) -> ParamDef { ParamDef { key, label, def, min, max, step } }

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    Some(match id {
        "hhoct_octane" => vec![pd("adx_len", "ADX length (bars)", 14., 2., 100., 1.), pd("adx_min", "ADX above", 25., 0., 100., 1.)],
        "bbgf_gforce" => vec![pd("spike", "SpikTrig", 150., 1., 1000., 1.), pd("lookback", "LkBck (bars)", 13., 2., 100., 1.), pd("slm", "SLM: stop multiple of the average range", 0.3, 0., 5., 0.05)],
        "twsifp_cci" => vec![pd("cci_len", "CCI length (bars)", 14., 2., 100., 1.), pd("xma_len", "XAverage length (bars)", 14., 2., 100., 1.), pd("adx_len", "ADX length (bars)", 15., 2., 100., 1.), pd("adx_max", "ADX below", 25., 0., 100., 1.)],
        "jraeb_60min" => vec![pd("min_range", "Trigger bar range at least (S&P points; 12 ES ticks)", 3.0, 0.25, 50., 0.25), pd("target", "Target inside the far end (S&P points; 8 ES ticks)", 2.0, 0.25, 50., 0.25)],
        _ => return None,
    })
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if params(id).is_none() || to >= x.c.len() || from > to { return None; }
    let g = |k: &str| super::jb624::getp(id, p, k);
    let u = |k: &str| g(k).round().max(1.0) as usize;
    Some(match id {
        "hhoct_octane" => octane(x, from, to, u("adx_len"), g("adx_min")),
        "bbgf_gforce" => gforce(x, from, to, g("spike"), u("lookback"), g("slm")),
        "twsifp_cci" => ifp(x, from, to, u("cci_len"), u("xma_len"), u("adx_len"), g("adx_max")),
        _ => andy60(x, from, to, g("min_range"), g("target")),
    })
}

/// One bar of the continuous series: daily row, stamp, OHLC, its minutes, first/last bar of its day, session open minute.
/// (pub(crate) since lane K55, which reuses the series for books/jbcdt.rs, jbudt.rs and stad.rs; no change here.)
pub(crate) struct Bn { pub i: usize, pub s: u16, pub o: f64, pub h: f64, pub l: f64, pub c: f64, pub m: Vec<Bar1>, pub first: bool, pub last: bool, pub open: u16, pub close: u16 }

pub(crate) fn series(x: &Ctx, from: usize, to: usize, n: u16) -> Vec<Bn> {
    let mut out = vec![];
    for i in from..=to {
        let Some((open, close, m, v)) = crate::intraday_ep::day_bars(x, i, n) else { continue };
        let k = v.len();
        out.extend(v.into_iter().enumerate().map(|(j, z)| Bn { i, s: z.e, o: z.o, h: z.h, l: z.l, c: z.c, m: m[z.a..z.b].to_vec(), first: j == 0, last: j + 1 == k, open, close }));
    }
    out
}

/// Wilder ADX(n) on the bar series (DM and TR smoothed by connors::wilder; NaN until defined).
fn adx(b: &[Bn], n: usize) -> Vec<f64> {
    let k = b.len();
    let (mut pdm, mut mdm, mut tr) = (vec![f64::NAN; k], vec![f64::NAN; k], vec![f64::NAN; k]);
    for j in 1..k {
        let (up, dn) = (b[j].h - b[j - 1].h, b[j - 1].l - b[j].l);
        pdm[j] = if up > dn && up > 0.0 { up } else { 0.0 };
        mdm[j] = if dn > up && dn > 0.0 { dn } else { 0.0 };
        tr[j] = (b[j].h - b[j].l).max((b[j].h - b[j - 1].c).abs()).max((b[j].l - b[j - 1].c).abs());
    }
    let (sp, sm, st) = (wilder(&pdm, n), wilder(&mdm, n), wilder(&tr, n));
    let dx: Vec<f64> = (0..k).map(|j| if st[j] > 0.0 { let (p, m) = (100.0 * sp[j] / st[j], 100.0 * sm[j] / st[j]); if p + m > 0.0 { 100.0 * (p - m).abs() / (p + m) } else { 0.0 } } else { f64::NAN }).collect();
    wilder(&dx, n)
}

fn out_trade(p: (f64, usize, f64), xi: usize, xp: f64, why: &str, kind: &'static str) -> Trade { day_trade(p.0, p.1, p.2, xi, xp, why, kind, false) }

/// Octane: yesterday's floor pivot from the running high/low (updated only while ADX > adx_min, as printed), one trade a
/// day at the next bar's open when a bar inside the window closes beyond R1 / S1; out at the day's last close.
fn octane(x: &Ctx, from: usize, to: usize, n: usize, amin: f64) -> Vec<Trade> {
    let b = series(x, from, to, 5);
    let a = adx(&b, n);
    let (mut out, mut ht, mut lt, mut lv, mut tc, mut pend, mut pos) = (vec![], 0.0, 0.0, None::<(f64, f64)>, 0, 0.0, None::<(f64, usize, f64)>);
    for k in 0..b.len() {
        let z = &b[k];
        if pend != 0.0 { pos = Some((pend, z.i, z.o)); pend = 0.0; }
        if z.first {
            if k > 0 { let pv = (ht + lt + b[k - 1].c) / 3.0; lv = Some((2.0 * pv - lt, 2.0 * pv - ht)); }
            ht = z.h; lt = z.l; tc = 0;
        }
        if a[k] > amin {
            let cond = lv.map_or(false, |(r1, s1)| ht >= r1 || lt <= s1);
            ht = ht.max(z.h); lt = lt.min(z.l);
            if cond && !z.last && z.s >= z.open + 30 && z.s + 60 <= z.close && tc <= 0 {
                let (r1, s1) = lv.unwrap();
                let d = pos.map_or(0.0, |p| p.0);
                if z.c > r1 && d != 1.0 { pend = 1.0; tc += 1; } else if z.c < s1 && d != -1.0 { pend = -1.0; tc += 1; }
            }
        }
        if z.last { if let Some(p) = pos.take() { out.push(out_trade(p, z.i, z.c, "end of day", "open")); } }
    }
    out
}

/// G-Force (long): at the bar stamped open+130 a low spike (average low far above the 13-bar lowest low, in average ranges)
/// with C > C[4] and the 13-bar low under today's open buys at the next bar's open; stops placed at each close for the next
/// bar; out at the day's last close.
fn gforce(x: &Ctx, from: usize, to: usize, spike: f64, lk: usize, slm: f64) -> Vec<Trade> {
    let b = series(x, from, to, 10);
    let (mut out, mut pos, mut pend, mut stop, mut dop) = (vec![], None::<(f64, usize, f64)>, false, f64::NAN, 0.0);
    for k in 0..b.len() {
        let z = &b[k];
        if z.first { pend = false; stop = f64::NAN; dop = z.o; }
        if pend { pos = Some((1.0, z.i, z.o)); pend = false; }
        if let (Some(p), true) = (pos, stop.is_finite()) {
            if let Some(q) = z.m.iter().find(|q| q.l <= stop) { out.push(out_trade(p, z.i, q.o.min(stop), "stop", "open")); pos = None; }
        }
        stop = f64::NAN;
        if z.last { if let Some(p) = pos.take() { out.push(out_trade(p, z.i, z.c, "end of day", "open")); } continue; }
        if k + 1 < lk.max(13) { continue; }
        // as printed: AvLo and the Spike's average range use LkBck; Lowest(Low,13) and the stop's Average(Range,13) are fixed 13
        let avg = |n: usize, f: &dyn Fn(&Bn) -> f64| b[k + 1 - n..=k].iter().map(|q| f(q)).sum::<f64>() / n as f64;
        let avlo = avg(lk, &|q| q.l);
        let lowl = b[k + 1 - 13..=k].iter().map(|q| q.l).fold(f64::INFINITY, f64::min);
        let (avr, avr13) = (avg(lk, &|q| q.h - q.l), avg(13, &|q| q.h - q.l));
        let (t2, t3, t5) = (z.open + 130, z.open + 260, z.open + 390);
        if z.s == t2 && pos.is_none() && avr > 0.0 && k >= 4 && (avlo - lowl) / avr * 100.0 >= spike && z.c > b[k - 4].c && lowl < dop { pend = true; }
        if let Some(p) = pos {
            if z.s > t2 && z.s <= t3 { stop = avlo - slm * avr13; } else if z.s > t3 && z.s < t5 { stop = p.2; }
        }
    }
    out
}

/// Typical-price CCI with the mean absolute deviation (0 when it is 0), a fresh window sum per bar; NaN until defined.
fn cci(b: &[Bn], n: usize) -> Vec<f64> {
    let tp: Vec<f64> = b.iter().map(|z| (z.h + z.l + z.c) / 3.0).collect();
    (0..tp.len()).map(|k| if k + 1 < n { f64::NAN } else {
        let w = &tp[k + 1 - n..=k];
        let m = w.iter().sum::<f64>() / n as f64;
        let md = w.iter().map(|v| (v - m).abs()).sum::<f64>() / n as f64;
        if md > 0.0 { (tp[k] - m) / (0.015 * md) } else { 0.0 } }).collect()
}

/// Instant Futures Profit: inside the window a CCI cross of 0 with the close beyond today's open and its XAverage, ADX under
/// adx_max, places a limit at this close for the next bar; CCI back across 0 exits on this bar's close; flat at the day's end.
fn ifp(x: &Ctx, from: usize, to: usize, nc: usize, nx: usize, na: usize, amax: f64) -> Vec<Trade> {
    let b = series(x, from, to, 30);
    let (cc, a) = (cci(&b, nc), adx(&b, na));
    let xa = super::atm03::xavg(&b.iter().map(|z| z.c).collect::<Vec<_>>(), nx);
    let (mut out, mut pos, mut lim, mut dop) = (vec![], None::<(f64, usize, f64)>, None::<(f64, f64)>, 0.0);
    for k in 0..b.len() {
        let z = &b[k];
        if z.first { dop = z.o; }
        if let Some((d, lv)) = lim.take() {
            if let Some(q) = z.m.iter().find(|q| if d > 0.0 { q.l <= lv } else { q.h >= lv }) {
                let px = if d > 0.0 { q.o.min(lv) } else { q.o.max(lv) };
                if let Some(p) = pos.filter(|p| p.0 != d) { out.push(out_trade(p, z.i, px, "reverse", "limit")); pos = None; }
                if pos.is_none() { pos = Some((d, z.i, px)); }
            }
        }
        if k >= 1 && cc[k - 1].is_finite() && a[k].is_finite() && z.s >= z.open && z.s <= z.open + 150 && !z.last {
            let d = pos.map_or(0.0, |p| p.0);
            if z.c > dop && cc[k] > 0.0 && cc[k - 1] <= 0.0 && z.c > xa[k] && a[k] < amax && d != 1.0 { lim = Some((1.0, z.c)); }
            if z.c < dop && cc[k] < 0.0 && cc[k - 1] >= 0.0 && z.c < xa[k] && a[k] < amax && d != -1.0 { lim = Some((-1.0, z.c)); }
        }
        if let Some(p) = pos { if cc[k].is_finite() && ((p.0 > 0.0 && cc[k] < 0.0) || (p.0 < 0.0 && cc[k] > 0.0)) { out.push(out_trade(p, z.i, z.c, "cci exit", "limit")); pos = None; } }
        if z.last { if let Some(p) = pos.take() { out.push(out_trade(p, z.i, z.c, "end of day", "limit")); } lim = None; }
    }
    out
}

/// A resting order for the minute walker: `up` = fills when the price reaches the level from below (buy stop, sell
/// limit), else from above (sell stop, buy limit); `tag` names it for the rule.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Ord { pub up: bool, pub lv: f64, pub tag: u8 }

/// One minute's price path (4 points) against the orders the rule's state gives (`orders`, exits first); `fill` applies a
/// fill to the state. At the open every order the open is through fills at the open; along each segment the level nearest
/// the segment's start in the direction of travel fills first (at its level); a level at the segment's start price is
/// reached only after a fill at that price. Returns the fills (tag, price).
pub fn walk<S>(path: [f64; 4], st: &mut S, orders: &dyn Fn(&S) -> Vec<Ord>, fill: &dyn Fn(&mut S, u8, f64)) -> Vec<(u8, f64)> {
    let mut fills = vec![];
    for k in 0..4 {
        let to = path[k];
        let (mut fr, mut at) = (if k == 0 { to } else { path[k - 1] }, false);
        for _ in 0..20 {
            let mut best: Option<(f64, usize, Ord, f64)> = None;
            for (j, o) in orders(st).into_iter().enumerate() {
                let (ok, dist, px) = if k == 0 { (if o.up { to >= o.lv } else { to <= o.lv }, 0.0, to) }
                    else if at && (if o.up { fr >= o.lv } else { fr <= o.lv }) { (true, 0.0, fr) } // marketable after a fill (Codex review)
                    else if to > fr { (o.up && fr <= o.lv && o.lv <= to && (o.lv > fr || at), o.lv - fr, o.lv) }
                    else if to < fr { (!o.up && to <= o.lv && o.lv <= fr && (o.lv < fr || at), fr - o.lv, o.lv) }
                    else { (false, 0.0, 0.0) };
                if ok && best.map_or(true, |b| dist < b.0 || (dist == b.0 && j < b.1)) { best = Some((dist, j, o, px)); }
            }
            let Some((_, _, o, px)) = best else { break };
            fill(st, o.tag, px);
            fills.push((o.tag, px));
            fr = px;
            at = true;
        }
    }
    fills
}

/// Runs minute q on the convention path (the nearer extreme first; a tie: `adverse_high` decides) and says whether the
/// other path would fill differently (ambiguous).
pub fn minute<S: Clone>(q: &Bar1, st: &mut S, orders: &dyn Fn(&S) -> Vec<Ord>, fill: &dyn Fn(&mut S, u8, f64), adverse_high: bool) -> bool {
    let (hl, lh) = ([q.o, q.h, q.l, q.c], [q.o, q.l, q.h, q.c]);
    let high_first = q.h - q.o < q.o - q.l || (q.h - q.o == q.o - q.l && adverse_high);
    let (mut a, mut b) = (st.clone(), st.clone());
    let amb = walk(hl, &mut a, orders, fill) != walk(lh, &mut b, orders, fill);
    *st = if high_first { a } else { b };
    amb
}

/// Andy's E-mini Bar state on one day: the position (dir, entry px), whether an entry filled, a closed trade's exit px.
#[derive(Clone)]
struct Ae { pos: Option<(f64, f64)>, done: bool, exit: Option<f64> }

/// Andy's 60-minute bar: the 09:30-10:29 ET range (at least `rng` points) is faded with limits at its ends 10:30-11:29 ET,
/// target `tgt` inside the far end, time stop at the 13:30 minute's open.
fn andy60(x: &Ctx, from: usize, to: usize, rng: f64, tgt: f64) -> Vec<Trade> {
    let mut out = vec![];
    for i in from..=to {
        let Some(m) = crate::intraday_w2b::day(x, i) else { continue };
        let f = 1.0;
        let tb: Vec<&Bar1> = m.iter().filter(|q| (570..630).contains(&q.m)).collect();
        if tb.is_empty() { continue; }
        let th = tb.iter().map(|q| q.h).fold(f64::NEG_INFINITY, f64::max);
        let tl = tb.iter().map(|q| q.l).fold(f64::INFINITY, f64::min);
        if th - tl < rng * f { continue; }
        let orders = |s: &Ae| -> Vec<Ord> {
            match s.pos { None if s.done => vec![], None => vec![Ord { up: false, lv: tl, tag: 1 }, Ord { up: true, lv: th, tag: 2 }],
                Some(_) if s.exit.is_some() => vec![],
                Some((d, _)) => vec![if d > 0.0 { Ord { up: true, lv: th - tgt * f, tag: 0 } } else { Ord { up: false, lv: tl + tgt * f, tag: 0 } }] }
        };
        let fill = |s: &mut Ae, tag: u8, px: f64| { if tag == 0 { s.exit = Some(px); } else { s.pos = Some((if tag == 1 { 1.0 } else { -1.0 }, px)); s.done = true; } };
        let (mut st, mut closed) = (Ae { pos: None, done: false, exit: None }, None::<(f64, &str)>);
        for q in &m {
            if q.m >= 810 { if st.pos.is_some() && st.exit.is_none() { closed = Some((q.o, "time stop")); } break; }
            if q.m < 630 || (q.m >= 690 && st.pos.is_none()) { continue; }
            let adv = st.pos.map_or(false, |p| p.0 < 0.0);
            minute(q, &mut st, &orders, &fill, adv); // ambiguity: counted by the reference
            if let Some(px) = st.exit { closed = Some((px, "target")); break; }
        }
        let Some((d, ep)) = st.pos else { continue };
        let (xp, why) = closed.unwrap_or((m[m.len() - 1].c, "end of day"));
        out.push(day_trade(d, i, ep, i, xp, why, "limit", false));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    fn q(m: u16, o: f64, h: f64, l: f64, c: f64) -> Bar1 { Bar1 { m, o, h, l, c } }
    #[test]
    fn walker_orders_by_path_and_flags_ambiguity() {
        // flat; buy limit 9 and sell limit 11 live; target 1 above the fill. Minute o 10 h 11.5 l 8.5 c 10: the two paths fill
        // different sides -> ambiguous; the tie (1.5 each way) with no position goes low first: long at 9, target 10 reached.
        #[derive(Clone, Debug, PartialEq)]
        struct S { pos: Option<(f64, f64)>, x: Option<f64> }
        let orders = |s: &S| match s.pos { None => vec![Ord { up: false, lv: 9.0, tag: 1 }, Ord { up: true, lv: 11.0, tag: 2 }],
            Some((d, ep)) if s.x.is_none() => vec![Ord { up: d > 0.0, lv: ep + d, tag: 0 }], _ => vec![] };
        let fill = |s: &mut S, t: u8, px: f64| if t == 0 { s.x = Some(px) } else { s.pos = Some((if t == 1 { 1.0 } else { -1.0 }, px)) };
        let mut s = S { pos: None, x: None };
        assert!(minute(&q(600, 10., 11.5, 8.5, 10.), &mut s, &orders, &fill, false));
        assert_eq!(s, S { pos: Some((1.0, 9.0)), x: Some(10.0) });
        // opening through the buy limit fills at the open; the target 9.5 + 1 is reached on the way up
        let mut s = S { pos: None, x: None };
        assert_eq!(walk([8.5, 10.6, 8.4, 9.], &mut s, &orders, &fill), vec![(1, 8.5), (0, 9.5)]);
        // a target exactly at the extreme where the entry filled is not reached before the turn
        let mut s = S { pos: None, x: None };
        assert_eq!(walk([10., 9., 10.5, 10.], &mut s, &orders, &fill), vec![(1, 9.0), (0, 10.0)]);
        // a target made marketable by the fill (target below a long fill) fills at once at the fill price
        let o2 = |s: &S| match s.pos { None => vec![Ord { up: true, lv: 10.0, tag: 1 }], Some(_) if s.x.is_none() => vec![Ord { up: true, lv: 9.0, tag: 0 }], _ => vec![] };
        let mut s = S { pos: None, x: None };
        assert_eq!(walk([9.5, 10.5, 9.4, 10.], &mut s, &o2, &fill), vec![(1, 10.0), (0, 10.0)]);
        let mut s = S { pos: None, x: None };
        assert!(!minute(&q(600, 10., 10.5, 9.5, 10.), &mut s, &orders, &fill, false));
        assert_eq!(s.pos, None);
    }
    /// The reference comparison (run by hand): VAULT_US=<us_data.csv> VAULT_IDX=<idx_data.csv> VAULT_M1_DIR=<data/intraday>
    /// K46_OUT=<csv> cargo test --release k46_dump -- --ignored; then authors/titles-1030/K46w/cmp_dump.py.
    #[test]
    #[ignore]
    fn k46_dump() {
        use std::io::Write;
        let mut f = std::io::BufWriter::new(std::fs::File::create(std::env::var("K46_OUT").unwrap()).unwrap());
        for (file, t) in [("VAULT_US", "SPY"), ("VAULT_US", "SPX"), ("VAULT_IDX", "NIFTY 50"), ("VAULT_IDX", "NIFTYBANK")] {
            let rows = crate::load_csv(&std::env::var(file).unwrap()).unwrap().remove(t).unwrap();
            let mut x = Ctx::new(&rows, &rows, &[]);
            x.minutes = Some(Box::leak(Box::new(crate::intraday::Minutes::load(&std::env::var("VAULT_M1_DIR").unwrap(), t).unwrap())));
            let from = rows.partition_point(|r| r.date.as_str() < x.minutes.unwrap().first());
            for id in ["hhoct_octane", "bbgf_gforce", "twsifp_cci", "jraeb_60min"] {
                if id == "jraeb_60min" && t != "SPX" { continue; }
                for z in run(id, &x, from, rows.len() - 1, &Params::new()).unwrap() {
                    writeln!(f, "{id},{t},{},{},{},{},{},{},{}", z.dir, rows[z.legs[0].i].date, z.legs[0].px, rows[z.exit_i].date, z.exit_px, z.reason, z.open).unwrap();
                }
            }
        }
    }
}
