//! Lane K53 (5 Nov 2026, decision #229; batch-h3/H3.tsv; authors/titles-1030/K53.tsv): held H3 intraday cards run on the vault's
//! 1-minute file (intraday_ep::day_bars for N-minute bars, the K46 minute walker n1intra::minute for orders inside a bar).
//! No new cards: the catalogue rows stay in their books (jb30d, stad2, mlhpt, baboops, rwmts) and run here.
//! - `jb30d_or30` Bernstein, 30 Days to Market Mastery Lesson 18 pp.151-154 (= 6-2-4 report pp.51-55, jb624_or30).
//! - `stad7_idbo` / `stad7_fibret` Omega Research STAD Vol 7 ch.3 pp.47-53 / pp.60-68 (printed EasyLanguage).
//! - `mlhpt_ma3` Link, High Probability Trading pp.108-109.
//! - `baboops_stoops_rules` Babcock, The Best Mechanical Day Trading System I Know p.13 (rules 1-3), SPX only (S&P points).
//! - `rwmts_*` Weissman, Mechanical Trading Systems ch.5 pp.94-102 (the Nasdaq-100 tables 5.5-5.14) on the Chapter 4 code.
//! Independent reference: authors/titles-1030/K53w/k53_ref.py (written from the same sources).
use crate::books::n1intra::{minute, Ord};
use crate::connors::{rsi, sma, wilder, Ctx, ParamDef, Params, StrategyDef, Trade};
use crate::intraday::{day_trade, Bar1};

pub fn catalogue() -> Vec<StrategyDef> { vec![] }
pub fn family_of(_: &str) -> Option<(&'static str, &'static str, &'static str)> { None }

fn pd(key: &'static str, label: &'static str, def: f64, min: f64, max: f64, step: f64) -> ParamDef { ParamDef { key, label, def, min, max, step } }

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let stop = |d: f64| pd("stop", "Fail-safe stop % of entry", d, 0.25, 6., 0.25);
    Some(match id {
        "jb30d_or30" => vec![pd("last", "No trigger in the last N minutes of the session (book: the last hour)", 60., 0., 180., 30.)],
        "stad7_idbo" => vec![pd("init", "InitMin: minutes of the initial range", 90., 10., 240., 10.)],
        "stad7_fibret" => vec![pd("start", "StartTme as minutes after the session open (12:00 ET = 150)", 150., 30., 330., 5.),
            pd("r66", "R66: limit at the .66 level (code default off)", 0., 0., 1., 1.), pd("r50", "R50: limit at the .50 level", 1., 0., 1., 1.),
            pd("r33", "R33: limit at the .33 level", 1., 0., 1., 1.)],
        "mlhpt_ma3" => vec![pd("fast", "Fast average", 10., 2., 50., 1.), pd("mid", "Middle average", 35., 5., 100., 1.), pd("slow", "Slow (monitoring) average", 50., 10., 200., 1.)],
        "baboops_stoops_rules" => vec![pd("gap", "Minimum opening gap, ticks (rule 1: 15; the book's July 1997 notice: 30)", 15., 0., 60., 5.),
            pd("early", "Enter this many ticks before the gap closes (rule 1: 2)", 2., -10., 10., 1.), pd("cap", "Stop cap in S&P points ($1,500 at $500 a point)", 3., 0.5, 20., 0.25),
            pd("tick", "Tick in index points (S&P 0.05)", 0.05, 0.01, 1., 0.01)],
        "rwmts_2h_rsi400" | "rwmts_60m_rsi200h" => vec![stop(2.5)],
        "rwmts_30m_rsi100h" => vec![stop(1.5)],
        "rwmts_15m_5m_rsi" => vec![pd("bars", "Bar minutes: 15 (Table 5.13, 50-hour MA) or 5 (Table 5.14, 16.67-hour MA)", 15., 5., 15., 10.), stop(1.)],
        "rwmts_2h_stoch_cci" => vec![stop(1.5), pd("time", "Exit when bars since entry exceed (30 hours = 15 two-hour bars)", 14., 3., 40., 1.)],
        "rwmts_60m_seven" => vec![pd("tgt", "Profit target % of entry", 1., 0.25, 5., 0.25), stop(1.)],
        "rwmts_60m_rsi_cross" => vec![pd("tgt", "Profit target % of entry", 3., 0.25, 6., 0.25), stop(1.)],
        "rwmts_30m_rsi_cross" => vec![pd("tgt", "Profit target % of entry", 1., 0.25, 6., 0.25), stop(1.)],
        _ => return None,
    })
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if params(id).is_none() || to >= x.c.len() || from > to { return None; }
    let g = |k: &str| super::jb624::getp(id, p, k);
    let rw = |n: u16, sys: Sys| rwmts(x, from, to, n, sys, g("stop") / 100.0);
    Some(match id {
        "jb30d_or30" => or30(x, from, to, g("last").round() as u16),
        "stad7_idbo" => idbo(x, from, to, g("init").round() as u16),
        "stad7_fibret" => fibret(x, from, to, g("start").round() as u16, [g("r66") >= 0.5, g("r50") >= 0.5, g("r33") >= 0.5]),
        "mlhpt_ma3" => ma3(x, from, to, g("fast").round() as usize, g("mid").round() as usize, g("slow").round() as usize),
        "baboops_stoops_rules" => { let t = g("tick"); stoops(x, from, to, g("gap") * t, g("early") * t, g("cap"), t) }
        "rwmts_2h_rsi400" => rw(120, Sys::RsiExt),
        "rwmts_60m_rsi200h" => rw(60, Sys::RsiExt),
        "rwmts_30m_rsi100h" => rw(30, Sys::RsiExt),
        "rwmts_15m_5m_rsi" => rw(if g("bars") < 10. { 5 } else { 15 }, Sys::RsiExt),
        "rwmts_2h_stoch_cci" => rw(120, Sys::StochCci(g("time").round() as usize)),
        "rwmts_60m_seven" => rw(60, Sys::Seven(g("tgt") / 100.0)),
        _ => rw(if id == "rwmts_60m_rsi_cross" { 60 } else { 30 }, Sys::RsiCross(g("tgt") / 100.0)),
    })
}

/// Minute-walker state: the position (dir, entry px, entry day row, kind), trades closed in the walk, the day row, a flag
/// (or30: twice the range reached) and the orders placed for this bar.
#[derive(Clone, Default)]
struct St { pos: Option<(f64, f64, usize, &'static str)>, done: Vec<(f64, f64, usize, usize, f64, &'static str, &'static str)>, i: usize, k: bool, live: Vec<Ord> }
impl St {
    fn dir(&self) -> f64 { self.pos.map_or(0.0, |p| p.0) }
    fn enter(&mut self, d: f64, px: f64, kind: &'static str) {
        if self.dir() == d { return; }
        self.exit(px, "reverse");
        self.pos = Some((d, px, self.i, kind));
        self.k = false;
    }
    fn exit(&mut self, px: f64, why: &'static str) { if let Some((d, ep, ei, k)) = self.pos.take() { self.done.push((d, ep, ei, self.i, px, why, k)); } }
    fn walk(&mut self, q: &Bar1, orders: &dyn Fn(&St) -> Vec<Ord>, fill: &dyn Fn(&mut St, u8, f64)) { let adv = self.dir() < 0.0; minute(q, self, orders, fill, adv); }
    fn flush(&mut self, out: &mut Vec<Trade>) {
        for (d, ep, ei, xi, xp, why, k) in self.done.drain(..) { out.push(day_trade(d, ei, ep, xi, xp, why, k, false)); }
    }
    fn finish(mut self, out: &mut Vec<Trade>, i: usize, px: f64) {
        self.flush(out);
        if let Some((d, ep, ei, k)) = self.pos.take() { out.push(day_trade(d, ei, ep, i, px, "open", k, true)); }
    }
}
fn ord(up: bool, lv: f64, tag: u8) -> Ord { Ord { up, lv, tag } }

/// One N-minute bar of a continuous series: day row, session open, stamp, OHLC, its minutes, first bar of its day.
struct B { i: usize, open: u16, e: u16, o: f64, h: f64, l: f64, c: f64, m: Vec<Bar1>, first: bool }
fn series(x: &Ctx, from: usize, to: usize, n: u16) -> Vec<B> {
    let mut out = vec![];
    for i in from..=to {
        let Some((open, _, m, v)) = crate::intraday_ep::day_bars(x, i, n) else { continue };
        out.extend(v.into_iter().enumerate().map(|(j, z)| B { i, open, e: z.e, o: z.o, h: z.h, l: z.l, c: z.c, m: m[z.a..z.b].to_vec(), first: j == 0 }));
    }
    out
}

/// Bernstein's advanced 30-minute breakout, one contract: a later 30-minute bar ENDING beyond the first bar's high (low)
/// enters at that close (no trigger in the last `last` minutes); the stop is a 30-minute close beyond the opposite side,
/// which reverses once a day; once the trade has gained twice the first bar's range a stop sits at the first target
/// (entry +/- range); out at the close.
fn or30(x: &Ctx, from: usize, to: usize, last: u16) -> Vec<Trade> {
    let mut out = vec![];
    for i in from..=to {
        let Some((_, close, m, b)) = crate::intraday_ep::day_bars(x, i, 30) else { continue };
        let (h1, l1) = (b[0].h, b[0].l);
        let r = h1 - l1;
        if b.len() < 2 || !(r > 0.0) { continue; }
        let orders = |s: &St| -> Vec<Ord> {
            match s.pos { Some((d, ep, ..)) if !s.k => vec![ord(d > 0.0, ep + d * 2.0 * r, 1)], Some((d, ep, ..)) => vec![ord(d < 0.0, ep + d * r, 2)], None => vec![] }
        };
        let fill = |s: &mut St, t: u8, px: f64| if t == 1 { s.k = true } else { s.exit(px, "profit stop (first target)") };
        let (mut st, mut n, mut done) = (St { i, ..St::default() }, 0, false);
        for (j, z) in b.iter().enumerate().skip(1) {
            if st.pos.is_some() { for q in &m[z.a..z.b] { st.walk(q, &orders, &fill); } if st.pos.is_none() { done = true; } }
            if j + 1 == b.len() { st.exit(z.c, "end of day (MOC)"); break; }
            let want = if z.c > h1 { 1.0 } else if z.c < l1 { -1.0 } else { 0.0 };
            let ok = z.e + last <= close;
            match st.pos {
                None if !done && n == 0 && want != 0.0 && ok => { st.enter(want, z.c, "close"); n = 1; }
                Some((d, ..)) if want == -d => {
                    if n == 1 && ok { st.enter(want, z.c, "close"); n = 2; } else { st.exit(z.c, "stop (30-minute close beyond the other side)"); done = true; }
                }
                _ => {}
            }
        }
        st.flush(&mut out);
    }
    out
}

/// STAD7 Intraday Breakout on 10-minute bars (printed EasyLanguage): range of the bars stamped <= open + InitMin; later
/// (not the session's last bar) "Close Crosses Above SetHigh" -> Buy this bar on close (LongFlag), below SetLow -> Sell
/// (ShortFlag); MarketPosition read at a close is the value before that close's orders: long -> LongFlag false and
/// ExitLong next bar at SetLow stop (short mirror); "Close all trades at end of day session".
fn idbo(x: &Ctx, from: usize, to: usize, init: u16) -> Vec<Trade> {
    let mut out = vec![];
    for i in from..=to {
        let Some((open, _, m, b)) = crate::intraday_ep::day_bars(x, i, 10) else { continue };
        let (mut sh, mut sl, mut lf, mut sf, mut prev) = (f64::NAN, f64::NAN, true, true, f64::NAN);
        let mut st = St { i, ..St::default() };
        let mut stop: Option<(f64, f64)> = None; // (direction it protects, level), next bar only
        for (j, z) in b.iter().enumerate() {
            if let Some((d, lv)) = stop.take() {
                if st.dir() == d {
                    if let crate::intraday::Touch::Hit { px, .. } = crate::intraday::first_touch(&m[z.a..z.b], 0, &[crate::intraday::Level { up: d < 0.0, px: lv }]) { st.exit(px, "stop (other side of the range)"); }
                }
            }
            let mp = st.dir();
            if j + 1 == b.len() { st.exit(z.c, "end of day session"); break; }
            if z.e <= open + init {
                if j == 0 { sh = z.h; sl = z.l; lf = true; sf = true; } else { sh = sh.max(z.h); sl = sl.min(z.l); }
            } else {
                if lf && z.c > sh && prev <= sh { st.enter(1.0, z.c, "close"); }
                if sf && z.c < sl && prev >= sl { st.enter(-1.0, z.c, "close"); }
            }
            if mp > 0.0 { lf = false; stop = Some((1.0, sl)); }
            if mp < 0.0 { sf = false; stop = Some((-1.0, sh)); }
            prev = z.c;
        }
        st.flush(&mut out);
    }
    out
}

/// STAD7 Fibonacci Retracement on 5-minute bars (printed EasyLanguage), one continuous series (no end-of-day exit in the
/// code): the range of the bars stamped <= StartTme; after it, flat and no trade today, buy limits at the .50/.33 levels
/// (.66 off) when the range high came later, else sell limits there (next bar); the stop EntryPrice - 2 x Average(Range,4)
/// on the first bar of the position, then stop + (Low - stop)/2 each bar (short mirror), next bar.
fn fibret(x: &Ctx, from: usize, to: usize, start: u16, on: [bool; 3]) -> Vec<Trade> {
    let mut out = vec![];
    let bars = series(x, from, to, 5);
    let orders = |s: &St| -> Vec<Ord> { if s.pos.is_none() { s.live.iter().copied().filter(|o| o.tag < 10).collect() } else { s.live.iter().copied().filter(|o| o.tag == 10).collect() } };
    let fill = |s: &mut St, t: u8, px: f64| if t == 10 { s.exit(px, "trailing stop") } else { s.enter(if t < 4 { 1.0 } else { -1.0 }, px, "limit") };
    let mut st = St::default();
    let (mut nh, mut nl, mut nht, mut nlt, mut ntt, mut mp1) = (f64::NAN, f64::NAN, 0u16, 0u16, true, 0.0);
    let (mut p66, mut p50, mut p33, mut lx) = (f64::NAN, f64::NAN, f64::NAN, f64::NAN);
    let mut next: Vec<Ord> = vec![];
    for (k, z) in bars.iter().enumerate() {
        st.i = z.i;
        st.live = std::mem::take(&mut next);
        for q in &z.m { st.walk(q, &orders, &fill); }
        st.flush(&mut out);
        if z.first { nh = z.h; nl = z.l; nht = z.e; nlt = z.e; ntt = true; }
        let mp = st.dir();
        let c1 = z.e > z.open + start;
        if mp != 0.0 { ntt = false; }
        if !c1 {
            if z.h > nh { nh = z.h; nht = z.e; }
            if z.l < nl { nl = z.l; nlt = z.e; }
            p66 = nl + (nh - nl) * 0.66; p50 = (nl + nh) / 2.0; p33 = nl + (nh - nl) * 0.33;
        }
        if c1 && ntt {
            let long = nht > nlt;
            for (j, lv) in [p66, p50, p33].into_iter().enumerate() { if on[j] { next.push(ord(!long, lv, if long { 1 + j as u8 } else { 4 + j as u8 })); } }
        }
        if mp != 0.0 {
            let ep = st.pos.unwrap().1;
            lx = if mp1 != mp { let a = k.saturating_sub(3); ep - mp * 2.0 * bars[a..=k].iter().map(|b| b.h - b.l).sum::<f64>() / (k + 1 - a) as f64 }
                else if mp > 0.0 { lx + (z.l - lx) / 2.0 } else { lx - (lx - z.h) / 2.0 };
            next.push(ord(mp < 0.0, lx, 10));
        }
        mp1 = mp;
    }
    if let Some(z) = bars.last() { st.finish(&mut out, z.i, z.c); }
    out
}

/// Link's three averages on 5-minute bars (simple averages, orders at the bar's close, one series across days): long when
/// close > slow and fast > mid and fast > slow; a long is closed when close > slow but fast < mid (no short); mirror.
fn ma3(x: &Ctx, from: usize, to: usize, nf: usize, nm: usize, ns: usize) -> Vec<Trade> {
    let mut out = vec![];
    let bars = series(x, from, to, 5);
    let c: Vec<f64> = bars.iter().map(|b| b.c).collect();
    let (f, m, s) = (sma(&c, nf), sma(&c, nm), sma(&c, ns));
    let mut st = St::default();
    for (k, z) in bars.iter().enumerate() {
        st.i = z.i;
        if s[k].is_nan() || m[k].is_nan() || f[k].is_nan() { continue; }
        let d = st.dir();
        if (d > 0.0 && c[k] > s[k] && f[k] < m[k]) || (d < 0.0 && c[k] < s[k] && f[k] > m[k]) { st.exit(z.c, "exit (fast average back through the middle)"); }
        if c[k] > s[k] && f[k] > m[k] && f[k] > s[k] { st.enter(1.0, z.c, "close"); }
        if c[k] < s[k] && f[k] < m[k] && f[k] < s[k] { st.enter(-1.0, z.c, "close"); }
        st.flush(&mut out);
    }
    if let Some(z) = bars.last() { st.finish(&mut out, z.i, z.c); }
    out
}

/// Babcock's STOOPS as traded (rules 1-3, p.13), SPX points: an opening gap of at least `gap` beyond yesterday's extreme;
/// a stop `early` before the gap closes; protective stop one tick beyond the day's opposite extreme at the time of entry or
/// `cap` from the entry, whichever is nearer; out at the close if not profitable there, else at the next open.
fn stoops(x: &Ctx, from: usize, to: usize, gap: f64, early: f64, cap: f64, tick: f64) -> Vec<Trade> {
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let mut out = vec![];
    for i in from.max(1)..=to {
        let (d, lv) = if o[i] <= l[i - 1] - gap + 1e-9 { (1.0, l[i - 1] - early) } else if o[i] >= h[i - 1] + gap - 1e-9 { (-1.0, h[i - 1] + early) } else { continue };
        let Some(m) = crate::intraday_w2b::day(x, i) else { continue };
        let Some((px, stop, mut k, hit)) = stoops_fill(&m, d, lv, cap, tick) else { continue };
        let mut xp = if hit { Some(stop) } else { None };
        while xp.is_none() && k + 1 < m.len() {
            k += 1;
            let q = &m[k];
            if d * (q.o - stop) <= 0.0 { xp = Some(q.o); } else if d * ((if d > 0.0 { q.l } else { q.h }) - stop) <= 0.0 { xp = Some(stop); }
        }
        if let Some(v) = xp { out.push(day_trade(d, i, px, i, v, "protective stop", "stop", false)); }
        else if d * (c[i] - px) <= 0.0 { out.push(day_trade(d, i, px, i, c[i], "not profitable at the close", "stop", false)); }
        else if i < to { out.push(day_trade(d, i, px, i + 1, o[i + 1], "exit on the next open", "stop", false)); }
        else { out.push(day_trade(d, i, px, i, c[i], "open", "stop", true)); }
    }
    out
}
/// The entry minute of STOOPS: (fill, protective stop, minute, stopped in that minute). The day's extreme "at the time of
/// entry" includes the fill minute's extreme only when the minute path reaches it before the fill (open, nearer extreme,
/// farther extreme, close; a tie: low first, as for a flat position).
fn stoops_fill(m: &[Bar1], d: f64, lv: f64, cap: f64, tick: f64) -> Option<(f64, f64, usize, bool)> {
    let mut ex = f64::NAN; // the adverse extreme so far (low for a long, high for a short)
    for (k, q) in m.iter().enumerate() {
        let (adv, fav) = if d > 0.0 { (q.l, q.h) } else { (q.h, q.l) };
        if d * (fav - lv) < 0.0 { ex = if ex.is_nan() { adv } else if d > 0.0 { ex.min(adv) } else { ex.max(adv) }; continue; }
        let worse = |a: f64, b: f64| if a.is_nan() { b } else if d > 0.0 { a.min(b) } else { a.max(b) };
        let stop_at = |e: f64, px: f64| if d > 0.0 { (e - tick).max(px - cap) } else { (e + tick).min(px + cap) };
        if d * (q.o - lv) >= 0.0 { // opened through: fill at the open; the rest of the minute may reach the stop
            let s = stop_at(worse(ex, q.o), q.o);
            return Some((q.o, s, k, d * (adv - s) <= 0.0));
        }
        let adv_first = (q.h - q.o - (q.o - q.l)) * d > 0.0 || (q.h - q.o == q.o - q.l && d > 0.0); // tie while flat: low first
        if adv_first { let s = stop_at(worse(ex, adv), lv); return Some((lv, s, k, d * (q.c - s) <= 0.0)); }
        let s = stop_at(worse(ex, q.o), lv);
        return Some((lv, s, k, d * (adv - s) <= 0.0));
    }
    None
}

#[derive(Clone, Copy)]
enum Sys { RsiExt, StochCci(usize), Seven(f64), RsiCross(f64) }

/// Weissman's Chapter 4/5 CQG systems on one continuous series of session-aligned N-minute bars (day session): signals on
/// the close of bar s fill at the open of bar s+1 (its first minute's open); an opposite signal reverses; condition exits
/// on the close fill at the next open; the percentage stop/target work in every bar of the position from its entry bar on,
/// in minute order.
fn rwmts(x: &Ctx, from: usize, to: usize, n: u16, sys: Sys, stp: f64) -> Vec<Trade> {
    let mut out = vec![];
    let bars = series(x, from, to, n);
    let c: Vec<f64> = bars.iter().map(|b| b.c).collect();
    let nb = c.len();
    let (ri, ro, ma) = (rsi(&c, 9), rsi(&c, 14), sma(&c, 200));
    let (ssd, cc) = if matches!(sys, Sys::StochCci(_)) {
        let fk: Vec<f64> = (0..nb).map(|i| if i < 13 { f64::NAN } else {
            let hh = bars[i - 13..=i].iter().map(|b| b.h).fold(f64::MIN, f64::max);
            let ll = bars[i - 13..=i].iter().map(|b| b.l).fold(f64::MAX, f64::min);
            if hh > ll { 100.0 * (c[i] - ll) / (hh - ll) } else { 50.0 } }).collect();
        let tp: Vec<f64> = bars.iter().map(|b| (b.h + b.l + b.c) / 3.0).collect();
        let tm = sma(&tp, 10);
        let cc: Vec<f64> = (0..nb).map(|i| if tm[i].is_nan() { f64::NAN } else {
            let md = tp[i - 9..=i].iter().map(|v| (v - tm[i]).abs()).sum::<f64>() / 10.0;
            if md > 0.0 { (tp[i] - tm[i]) / (0.015 * md) } else { 0.0 } }).collect();
        (wilder(&wilder(&fk, 3), 3), cc)
    } else { (vec![], vec![]) };
    let xa = |v: &[f64], lv: f64, s: usize| s > 0 && v[s] > lv && v[s - 1] <= lv;
    let xb = |v: &[f64], lv: f64, s: usize| s > 0 && v[s] < lv && v[s - 1] >= lv;
    let ent = |s: usize| -> f64 {
        match sys {
            Sys::RsiExt => if ri[s] < 35.0 && c[s] > ma[s] { 1.0 } else if ri[s] > 65.0 && c[s] < ma[s] { -1.0 } else { 0.0 },
            Sys::StochCci(_) => if xb(&ssd, 15.0, s) && cc[s] < -100.0 { 1.0 } else if xa(&ssd, 85.0, s) && cc[s] > 100.0 { -1.0 } else { 0.0 },
            Sys::Seven(_) => if s < 7 { 0.0 } else if (s - 7..s - 1).all(|k| c[k] > c[k + 1]) && c[s - 1] < c[s] { 1.0 }
                else if (s - 7..s - 1).all(|k| c[k] < c[k + 1]) && c[s - 1] > c[s] { -1.0 } else { 0.0 },
            Sys::RsiCross(_) => if s > 0 && ro[s - 1] < 25.0 && xa(&ro, 25.0, s) { 1.0 } else if s > 0 && ro[s - 1] > 75.0 && xb(&ro, 75.0, s) { -1.0 } else { 0.0 },
        }
    };
    let cx = |s: usize, d: f64, ei: usize| -> Option<&'static str> {
        match sys {
            Sys::RsiExt => ((d > 0.0 && xa(&ro, 60.0, s)) || (d < 0.0 && xb(&ro, 40.0, s))).then_some("RSI(14) exit cross"),
            Sys::StochCci(b) => if (d > 0.0 && xa(&ssd, 30.0, s)) || (d < 0.0 && xb(&ssd, 70.0, s)) { Some("SSD exit cross") } else if s >= ei && s - ei > b { Some("time exit") } else { None },
            Sys::Seven(_) => (s >= 6 && (s - 6..s).all(|k| if d > 0.0 { c[k] < c[k + 1] } else { c[k] > c[k + 1] })).then_some("seven-period reversal exit"),
            Sys::RsiCross(_) => None,
        }
    };
    let tgt = match sys { Sys::Seven(t) | Sys::RsiCross(t) => t, _ => f64::NAN };
    let orders = |s: &St| -> Vec<Ord> {
        let Some((d, ep, ..)) = s.pos else { return vec![] };
        let mut v = vec![ord(d < 0.0, ep * (1.0 - d * stp), 1)];
        if tgt.is_finite() { v.push(ord(d > 0.0, ep * (1.0 + d * tgt), 2)); }
        v
    };
    let fill = |s: &mut St, t: u8, px: f64| s.exit(px, if t == 1 { "stop" } else { "target" });
    let (mut st, mut ei) = (St::default(), 0usize);
    for t in 1..nb {
        let (s, z) = (t - 1, &bars[t]);
        st.i = z.i;
        if let Some((d, ..)) = st.pos { if let Some(why) = cx(s, d, ei) { st.exit(z.o, why); } }
        let e = ent(s);
        if e != 0.0 && st.dir() != e { st.enter(e, z.o, "open"); ei = t; }
        if st.pos.is_some() { for q in &z.m { st.walk(q, &orders, &fill); } }
        st.flush(&mut out);
    }
    if let Some(z) = bars.last() { st.finish(&mut out, z.i, z.c); }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    fn q(m: u16, o: f64, h: f64, l: f64, c: f64) -> Bar1 { Bar1 { m, o, h, l, c } }
    fn hm(m: u16) -> String { format!("{:02}:{:02}", m / 60, m % 60) }
    fn minutes(name: &str, days: &[(&str, Vec<String>)]) -> &'static crate::intraday::Minutes {
        let dir = std::env::temp_dir().join(format!("k53{name}{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let (mut body, mut idx) = (String::from("Date,Time,Open,High,Low,Close\n"), String::from("Date,offset,bytes\n"));
        for (d, lines) in days {
            let start = body.len();
            for l in lines { body.push_str(&format!("{d},{l}\n")); }
            idx.push_str(&format!("{d},{start},{}\n", body.len() - start));
        }
        std::fs::write(dir.join("T.m1.csv"), &body).unwrap();
        std::fs::write(dir.join("T.m1idx.csv"), &idx).unwrap();
        Box::leak(Box::new(crate::intraday::Minutes::load(dir.to_str().unwrap(), "T").unwrap()))
    }
    #[test]
    fn or30_by_hand() {
        // day 1 rises 0.01 a minute from 100: first bar 99.995..100.295 (range 0.30); the 10:00-10:29 bar closes 100.59 above
        // it -> long at that close; twice the range (101.19) is reached, the stop at 100.89 never; out at the 15:59 close.
        // Day 2 falls: short at 99.41, out at 96.11.
        let row = |d: &str| crate::Row { ticker: "T".into(), date: d.into(), open: 1., high: 1., low: 1., close: 1., volume: None };
        let r = vec![row("2020-01-02"), row("2020-01-03")];
        let mut x = Ctx::new(&r, &r, &[]);
        let path = |s: f64| -> Vec<String> { (570..960u16).map(|m| { let p = 100. + s * (m - 570) as f64 * 0.01; format!("{},{p},{},{},{p}", hm(m), p + 0.005, p - 0.005) }).collect() };
        x.minutes = Some(minutes("or30", &[("2020-01-02", path(1.)), ("2020-01-03", path(-1.))]));
        let t = crate::books::run("jb30d_or30", &x, 0, 1, &Params::new()).unwrap();
        let v: Vec<(f64, usize, f64, f64)> = t.iter().map(|z| (z.dir, z.legs[0].i, (z.legs[0].px * 100.).round() / 100., (z.exit_px * 100.).round() / 100.)).collect();
        assert_eq!(v, vec![(1., 0, 100.59, 103.89), (-1., 1, 99.41, 96.11)]);
    }
    #[test]
    fn stoops_entry_minute_by_hand() {
        // long buy stop 99.90: minute 1 never reaches it (low 98.00); minute 2's low is nearer its open, so its low 99.10 comes
        // before the fill: day low at entry 98.00 -> stop max(97.95, 96.90) = 97.95, not hit in the minute.
        let m = [q(570, 99.0, 99.5, 98.0, 99.2), q(571, 99.2, 100.2, 99.1, 100.0)];
        assert_eq!(stoops_fill(&m, 1.0, 99.9, 3.0, 0.05).map(|v| (v.0, (v.1 * 100.0).round() / 100.0, v.2, v.3)), Some((99.9, 97.95, 1, false)));
        // the cap is nearer: a minute opening through the level fills at its open 100.00; day low 95.00 -> stop 100.00 - 3.00 = 97.00, hit by its low
        let m = [q(570, 96.0, 96.5, 95.0, 96.2), q(571, 100.0, 100.2, 96.0, 99.0)];
        assert_eq!(stoops_fill(&m, 1.0, 99.9, 3.0, 0.05).map(|v| (v.0, (v.1 * 100.0).round() / 100.0, v.3)), Some((100.0, 97.0, true)));
    }
    /// The reference comparison (run by hand): VAULT_US=<us_data.csv> VAULT_IDX=<idx_data.csv> VAULT_M1_DIR=<data/intraday>
    /// K53_OUT=<csv> cargo test --release k53_dump -- --ignored; then authors/titles-1030/K53w/cmp_dump.py.
    #[test]
    #[ignore]
    fn k53_dump() {
        use std::io::Write;
        let mut f = std::io::BufWriter::new(std::fs::File::create(std::env::var("K53_OUT").unwrap()).unwrap());
        for (file, t) in [("VAULT_US", "SPY"), ("VAULT_US", "SPX"), ("VAULT_IDX", "NIFTY 50"), ("VAULT_IDX", "NIFTYBANK")] {
            let rows = crate::load_csv(&std::env::var(file).unwrap()).unwrap().remove(t).unwrap();
            let mut x = Ctx::new(&rows, &rows, &[]);
            x.minutes = Some(Box::leak(Box::new(crate::intraday::Minutes::load(&std::env::var("VAULT_M1_DIR").unwrap(), t).unwrap())));
            let from = rows.partition_point(|r| r.date.as_str() < x.minutes.unwrap().first());
            let opts: &[(&str, &[(&str, f64)])] = &[("jb30d_or30", &[]), ("stad7_idbo", &[]), ("stad7_fibret", &[]), ("stad7_fibret", &[("r66", 1.)]), ("mlhpt_ma3", &[]),
                ("baboops_stoops_rules", &[]), ("baboops_stoops_rules", &[("gap", 30.)]), ("jchr1_lizard_long", &[]), ("jchr1_lizard_short", &[]), ("tdperl_rebo", &[]),
                ("rwmts_2h_rsi400", &[]), ("rwmts_2h_stoch_cci", &[]), ("rwmts_60m_rsi200h", &[]), ("rwmts_60m_seven", &[]), ("rwmts_60m_rsi_cross", &[]),
                ("rwmts_30m_rsi100h", &[]), ("rwmts_30m_rsi100h", &[("stop", 2.5)]), ("rwmts_30m_rsi_cross", &[]), ("rwmts_15m_5m_rsi", &[]), ("rwmts_15m_5m_rsi", &[("bars", 5.)])];
            for (id, kv) in opts {
                if *id == "baboops_stoops_rules" && t != "SPX" { continue; }
                if id.starts_with("jchr1") && t != "SPY" { continue; }
                let mut p = Params::new();
                for (k, v) in kv.iter() { p.insert(k.to_string(), *v); }
                let tag = kv.iter().map(|(k, v)| format!("{k}={v}")).collect::<Vec<_>>().join(";");
                let lo = if id.starts_with("jchr1") || *id == "tdperl_rebo" || *id == "baboops_stoops_rules" { from.max(1) } else { from };
                for z in crate::books::run(id, &x, lo, rows.len() - 1, &p).unwrap() {
                    writeln!(f, "{id}{}{tag},{t},{},{},{},{},{},\"{}\",{}", if tag.is_empty() { "" } else { "|" }, z.dir, rows[z.legs[0].i].date, z.legs[0].px, rows[z.exit_i].date, z.exit_px, z.reason, z.open).unwrap();
                }
            }
        }
    }
}
