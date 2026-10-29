//! Art Collins, "Beating the Financial Futures Market" (Wiley, 2006) and its TradeStation code
//! (book appendix pp.205-238 = TradeStationFormulas.doc). Transcription, page refs, verification: library/acbff/.
//!
//! Every Live strategy is the book's own EasyLanguage code run bar by bar (code wins over prose; where the two
//! disagree the module says so). The book tested full-size S&P / NASDAQ / Russell / Dow futures (plus bonds and
//! currencies); here the index (SPX, NIFTY 50, ...) replaces the futures contract (instrument swap only).
//!
//! Order model (TradeStation daily-bar backtest, class B where the book is silent):
//! * the script runs at each bar's close; "next bar" orders live for the following bar only;
//! * market = next open; stop/limit fill at the open when the open is already through the level, else at the level;
//! * intrabar order: open -> nearer extreme -> other extreme -> close (TradeStation's bar-path assumption;
//!   high first when H-O <= O-L); orders fill in the order the price meets them;
//! * an entry while flat opens, an opposite entry reverses, an entry in the held direction is ignored;
//! * "setexitonclose" exits at that bar's close; "this bar on close" orders fill at the signal bar's close;
//! * EasyLanguage variables keep their last value when no "if" assigns them (code literal).
//! 1 tick (minmove/pricescale) = 0.10 index points (full S&P futures) on US data, 0.05 (NSE) on Indian data.
use crate::connors::{Claim, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "acbff";
/// Admitted ids (authors/collins/admission/SOURCE-CONTRACT.md).
pub const REVIEWED: &[&str] = &["acbff_cup_overnight", "acbff_cup_3day", "acbff_eight_lim25", "acbff_eight_lim25_3", "acbff_2hl", "acbff_2hl_tick", "acbff_2hl_4", "acbff_rx15", "acbff_short_dom", "acbff_dom", "acbff_moy", "acbff_moy_dom", "acbff_moy_dom_day", "acbff_cal3", "acbff_cal2_day", "acbff_cal_eight", "acbff_3rd_fade", "acbff_5v15", "acbff_3v30", "acbff_5d_avg_fade", "acbff_nday_10stop", "acbff_fade_rev", "acbff_rx4day", "acbff_mom66", "acbff_contract", "acbff_15_30", "acbff_wk_rx", "acbff_2v5", "acbff_5c_day", "acbff_c3_124", "acbff_c3_125", "acbff_c3_134", "acbff_c3_135", "acbff_c3_145", "acbff_c3_234", "acbff_c3_235", "acbff_c3_245", "acbff_c3_345", "acbff_c40", "acbff_cup_day", "acbff_dom_day", "acbff_dom_day_cc", "acbff_dow", "acbff_eight", "acbff_eight_3", "acbff_eight_stop33", "acbff_eight_stop33_3", "acbff_eight_stop66", "acbff_ext50", "acbff_fade23oc", "acbff_fade2c", "acbff_fade2oc", "acbff_fade_oc", "acbff_five", "acbff_five_4of5", "acbff_hl15", "acbff_monday", "acbff_nee5", "acbff_range10", "acbff_six", "acbff_six_q", "acbff_six_q66", "acbff_six_q66_v2", "acbff_six_stop66", "acbff_sr20", "acbff_three", "acbff_tuesday", "acbff_wed_fri" /* intraday wave 2b: daily bars decide every trade */];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }
const NAN: f64 = f64::NAN;

// ---------------------------------------------------------------- order engine

#[derive(Clone, Copy, PartialEq, Debug)]
enum A { Buy, Sell, Short, Cover }
#[derive(Clone, Copy, Debug)]
enum T { Mkt, Stop(f64), Lim(f64), Close }
#[derive(Clone, Copy, Debug)]
struct O { a: A, t: T, tag: &'static str, tp: f64 }
#[derive(Clone, Copy, Debug, Default)]
pub struct Pos { mp: i8, ei: usize, ep: f64 }
impl Pos { fn bse(&self, i: usize) -> usize { i.saturating_sub(self.ei) } }

fn buy_side(a: A) -> bool { matches!(a, A::Buy | A::Cover) }
fn o(a: A, t: T, tag: &'static str) -> O { O { a, t, tag, tp: NAN } }
/// Push an order unless its price is not yet defined (indicator warm-up).
fn put(v: &mut Vec<O>, a: A, t: T, tag: &'static str) {
    let ok = match t { T::Stop(p) | T::Lim(p) => p.is_finite(), _ => true };
    if ok { v.push(o(a, t, tag)); }
}
fn day_exit(v: &mut Vec<O>) { v.push(o(A::Sell, T::Close, "exit on close")); v.push(o(A::Cover, T::Close, "exit on close")); }

struct Eng<'a, 'b> { x: &'a Ctx<'b>, pos: Pos, cur: Option<Trade>, out: Vec<Trade> }
impl<'a, 'b> Eng<'a, 'b> {
    fn close(&mut self, px: f64, j: usize, sig: usize, tag: &str) {
        let mut t = self.cur.take().unwrap();
        t.exit_i = j; t.exit_px = px; t.signal_i = sig; t.reason = tag.to_string();
        self.out.push(t); self.pos.mp = 0;
    }
    /// Try to execute `od` at `px` on bar j; false when the order does not apply to the current position.
    fn act(&mut self, od: &O, px: f64, j: usize, sig: usize) -> bool {
        let kind = match od.t { T::Mkt => "open", T::Stop(_) => "stop", T::Lim(_) => "limit", T::Close => "close" };
        match od.a {
            A::Buy | A::Short => {
                let d: i8 = if od.a == A::Buy { 1 } else { -1 };
                if self.pos.mp == d { return false; }
                if self.pos.mp != 0 { self.close(px, j, sig, "reversed by opposite entry"); }
                self.cur = Some(Trade { dir: d as f64, legs: vec![Leg { i: j, px, w: 1.0 }], exit_i: j, exit_px: px,
                    reason: String::new(), entry_kind: kind, signal_i: sig, open: false, exits: vec![] });
                self.pos = Pos { mp: d, ei: j, ep: px };
                true
            }
            A::Sell => { if self.pos.mp != 1 { return false; } self.close(px, j, sig, od.tag); true }
            A::Cover => { if self.pos.mp != -1 { return false; } self.close(px, j, sig, od.tag); true }
        }
    }
    fn after_fill(&self, od: &O, px: f64, pend: &mut Vec<O>, tried: &mut Vec<bool>) {
        if od.tp.is_finite() && matches!(od.a, A::Buy | A::Short) {
            let (a, p) = if od.a == A::Buy { (A::Sell, px + od.tp) } else { (A::Cover, px - od.tp) };
            pend.push(o(a, T::Lim(p), "profit target")); tried.push(false);
        }
    }
    fn bar(&mut self, j: usize, mut pend: Vec<O>) {
        let x = self.x;
        let (op, hi, lo, cl) = (x.o[j], x.h[j], x.l[j], x.c[j]);
        let sig = j.saturating_sub(1);
        let mut tried = vec![false; pend.len()];
        // at the open
        let mut k = 0;
        while k < pend.len() {
            let od = pend[k];
            let hit = match od.t {
                T::Mkt => true,
                T::Stop(p) => if buy_side(od.a) { op >= p } else { op <= p },
                T::Lim(p) => if buy_side(od.a) { op <= p } else { op >= p },
                T::Close => false,
            };
            if hit && self.act(&od, op, j, sig) {
                pend.remove(k); tried.remove(k);
                self.after_fill(&od, op, &mut pend, &mut tried);
                continue;
            }
            if matches!(od.t, T::Mkt) { pend.remove(k); tried.remove(k); continue; }
            k += 1;
        }
        // along the bar path
        let pts = if hi - op <= op - lo { [op, hi, lo, cl] } else { [op, lo, hi, cl] };
        for s in 0..3 {
            let (a, b) = (pts[s], pts[s + 1]);
            if a == b { continue; }
            let up = b > a;
            for t in tried.iter_mut() { *t = false; }
            let mut cur = a;
            loop {
                let mut best: Option<(usize, f64)> = None;
                for (idx, od) in pend.iter().enumerate() {
                    if tried[idx] { continue; }
                    let p = match od.t {
                        T::Stop(p) if up == buy_side(od.a) => p,
                        T::Lim(p) if up != buy_side(od.a) => p,
                        _ => continue,
                    };
                    let inside = if up { p > a && p >= cur && p <= b } else { p < a && p <= cur && p >= b };
                    if !inside { continue; }
                    let better = match best { None => true, Some((_, bp)) => if up { p < bp } else { p > bp } };
                    if better { best = Some((idx, p)); }
                }
                let Some((idx, p)) = best else { break };
                tried[idx] = true;
                let od = pend[idx];
                if self.act(&od, p, j, sig) {
                    pend.remove(idx); tried.remove(idx);
                    self.after_fill(&od, p, &mut pend, &mut tried);
                }
                cur = p;
            }
        }
        // at the close
        for od in pend.iter().filter(|od| matches!(od.t, T::Close)) {
            let od = *od;
            self.act(&od, cl, j, j);
        }
    }
}

/// Run a strategy script. `s(i, pos, next, this_close)` is called at the close of bar i (position as of the
/// script run); `next` holds orders for bar i+1, `this_close` orders fill at bar i's close.
fn engine<F: FnMut(usize, &Pos, &mut Vec<O>, &mut Vec<O>)>(x: &Ctx, from: usize, to: usize, mut s: F) -> Vec<Trade> {
    let mut e = Eng { x, pos: Pos::default(), cur: None, out: vec![] };
    let mut pend: Vec<O> = vec![];
    if from > 0 {
        let (mut nx, mut tc) = (vec![], vec![]);
        s(from - 1, &e.pos, &mut nx, &mut tc);
        pend = nx;
    }
    for j in from..=to {
        e.bar(j, std::mem::take(&mut pend));
        if j < to {
            let (mut nx, mut tc) = (vec![], vec![]);
            let snap = e.pos;
            s(j, &snap, &mut nx, &mut tc);
            for od in &tc { e.act(od, x.c[j], j, j); }
            pend = nx;
        }
    }
    if let Some(mut t) = e.cur.take() {
        t.exit_i = to; t.exit_px = x.c[to]; t.signal_i = to; t.reason = "open".into(); t.open = true; e.out.push(t);
    }
    e.out
}

// ---------------------------------------------------------------- indicators (TradeStation definitions)

fn mean(v: &[f64], n: usize) -> Vec<f64> {
    (0..v.len()).map(|i| if n == 0 || i + 1 < n { NAN } else { v[i + 1 - n..=i].iter().sum::<f64>() / n as f64 }).collect()
}
fn hiw(v: &[f64], n: usize) -> Vec<f64> {
    (0..v.len()).map(|i| if n == 0 || i + 1 < n { NAN } else { v[i + 1 - n..=i].iter().copied().fold(f64::MIN, f64::max) }).collect()
}
fn low(v: &[f64], n: usize) -> Vec<f64> {
    (0..v.len()).map(|i| if n == 0 || i + 1 < n { NAN } else { v[i + 1 - n..=i].iter().copied().fold(f64::MAX, f64::min) }).collect()
}
/// HighestBar / LowestBar: offset of the extreme in the last n bars, most recent one on ties (Extremes, strict compare).
fn xbar(v: &[f64], n: usize, high: bool) -> Vec<f64> {
    (0..v.len()).map(|i| {
        if n == 0 || i + 1 < n { return NAN; }
        let (mut best, mut off) = (v[i], 0usize);
        for k in 1..n { let p = v[i - k]; if (high && p > best) || (!high && p < best) { best = p; off = k; } }
        off as f64
    }).collect()
}
/// NthHighest(2, v, n) / NthLowest(2, v, n), duplicates counted.
fn nth2(v: &[f64], i: usize, n: usize, high: bool) -> f64 {
    if n < 2 || i + 1 < n { return NAN; }
    let mut w: Vec<f64> = v[i + 1 - n..=i].to_vec();
    w.sort_by(|a, b| a.partial_cmp(b).unwrap());
    if high { w[n - 2] } else { w[1] }
}
fn at(v: &[f64], i: usize, k: usize) -> f64 { if i >= k { v[i - k] } else { NAN } }
fn persist(n: usize, f: impl Fn(usize) -> (bool, bool)) -> Vec<f64> {
    let (mut v, mut s) = (vec![0.0; n], 0.0);
    for i in 0..n { let (p, q) = f(i); if p { s = 1.0; } if q { s = -1.0; } v[i] = s; }
    v
}
fn rng(x: &Ctx) -> Vec<f64> { x.h.iter().zip(&x.l).map(|(h, l)| h - l).collect() }

/// (day of week 0=Sun..6=Sat, day of month, month) from "YYYY-MM-DD".
fn ymd(d: &str) -> (i64, u32, u32) {
    let y: i64 = d[0..4].parse().unwrap_or(1970);
    let m: u32 = d[5..7].parse().unwrap_or(1);
    let dd: u32 = d[8..10].parse().unwrap_or(1);
    (y, m, dd)
}
fn days(y: i64, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (m as i64 + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}
struct Cal { dow: Vec<u32>, dom: Vec<u32>, mon: Vec<u32>, week: Vec<i64> }
fn cal(x: &Ctx) -> Cal {
    let mut c = Cal { dow: vec![], dom: vec![], mon: vec![], week: vec![] };
    for r in x.r {
        let (y, m, d) = ymd(&r.date);
        let n = days(y, m, d);
        c.dow.push(((n + 4).rem_euclid(7)) as u32); // 1970-01-01 = Thursday (4)
        c.dom.push(d); c.mon.push(m); c.week.push((n + 3).div_euclid(7)); // Monday-based week number
    }
    c
}

/// The five "either-or" building blocks (persisting variables as coded) and the other daily signals.
struct Ind { q: Vec<f64>, u: Vec<f64>, xo: Vec<f64>, y: Vec<f64>, z: Vec<f64> }
fn five(x: &Ctx, e: usize, n: usize, p: usize, qn: usize, m: usize, hl: usize, lr: bool) -> Ind {
    let c = &x.c; let len = c.len();
    let ae = mean(c, e); let (an, ap) = (mean(c, n), mean(c, p));
    let (hb, lb) = (xbar(c, qn, true), xbar(c, qn, false));
    let r = rng(x); let ar = mean(&r, m);
    let (ah, al) = (mean(&x.h, hl), mean(&x.l, hl));
    Ind {
        q: persist(len, |i| (c[i] > ae[i], c[i] < ae[i])),
        u: persist(len, |i| (an[i] < ap[i], an[i] > ap[i])),
        xo: persist(len, |i| (hb[i] > lb[i], hb[i] < lb[i])),
        y: persist(len, |i| {
            let c1 = at(c, i, 1);
            if lr { // EasyLanguage left-to-right and/or (labelled option for the unparenthesised 17.x form)
                (r[i] > ar[i] && c[i] < c1, r[i] > ar[i] && c[i] > c1)
            } else {
                ((r[i] < ar[i] && c[i] > c1) || (r[i] > ar[i] && c[i] < c1), (r[i] < ar[i] && c[i] < c1) || (r[i] > ar[i] && c[i] > c1))
            }
        }),
        z: persist(len, |i| { let mid = (ah[i] + al[i]) / 2.0; (c[i] > mid, c[i] < mid) }),
    }
}
/// Fade two same-way open-to-closes: +1 / -1 / 0 (else 0 as coded).
fn fade2oc(x: &Ctx) -> Vec<f64> {
    (0..x.c.len()).map(|i| {
        if i < 1 { return 0.0; }
        if x.c[i] > x.o[i] && x.c[i - 1] > x.o[i - 1] { -1.0 } else if x.c[i] < x.o[i] && x.c[i - 1] < x.o[i - 1] { 1.0 } else { 0.0 }
    }).collect()
}
/// Cups (+1) and caps (-1), Table 15.1 code.
fn cupcap(x: &Ctx) -> Vec<f64> {
    let (h, l, c) = (&x.h, &x.l, &x.c);
    (0..c.len()).map(|i| {
        if i < 4 { return 0.0; }
        let ll3 = l[i - 2].min(l[i - 3]).min(l[i - 4]);
        let hh3 = h[i - 2].max(h[i - 3]).max(h[i - 4]);
        let cup = l[i] > l[i - 1] && l[i - 1] < ll3 && c[i] > c[i - 1] && c[i - 1] < c[i - 2];
        let cap = h[i] < h[i - 1] && h[i - 1] > hh3 && c[i] < c[i - 1] && c[i - 1] > c[i - 2];
        if cap { -1.0 } else if cup { 1.0 } else { 0.0 }
    }).collect()
}
/// Three-day 20% support (+1) / resistance (-1) — (long, short) conditions, Table 16.1 code.
fn sr(x: &Ctx, k: f64) -> (Vec<bool>, Vec<bool>) {
    let (hh, lh, hl, ll) = (hiw(&x.h, 3), low(&x.h, 3), hiw(&x.l, 3), low(&x.l, 3));
    let n = x.c.len();
    ((0..n).map(|i| hl[i] - ll[i] <= k * (hh[i] - ll[i])).collect(), (0..n).map(|i| hh[i] - lh[i] <= k * (hh[i] - ll[i])).collect())
}
fn sr_val(x: &Ctx) -> Vec<f64> { let (b, s) = sr(x, 0.2); b.iter().zip(&s).map(|(b, s)| if *s { -1.0 } else if *b { 1.0 } else { 0.0 }).collect() }
/// Five-closes patterns used in chapters 36 and 43: `k` = number of one-way closes before the opposite close.
fn run_rev(c: &[f64], i: usize, start: usize, k: usize, down_then_up: bool) -> bool {
    // c[start] vs c[start+1] is the opposite move, then k same-way closes c[start+1..start+1+k]
    if i < start + k + 1 { return false; }
    let cc = |o: usize| c[i - o];
    let opp = if down_then_up { cc(start) > cc(start + 1) } else { cc(start) < cc(start + 1) };
    opp && (start + 1..start + 1 + k).all(|o| if down_then_up { cc(o) < cc(o + 1) } else { cc(o) > cc(o + 1) })
}
/// Non-either-or block aa (5 closes, c[1..6]) and bb (c[1..7]) from Table 43.1, +1/-1/0.
fn nee_ab(c: &[f64], i: usize, n: usize) -> f64 {
    // code: c[1]<c[2] and c[2]>c[3] ... (n-1 comparisons after the first) -> +1 ; mirror -> -1
    if i < n + 1 { return 0.0; }
    let cc = |o: usize| c[i - o];
    let up = cc(1) < cc(2) && (2..=n).all(|o| cc(o) > cc(o + 1));
    let dn = cc(1) > cc(2) && (2..=n).all(|o| cc(o) < cc(o + 1));
    if dn { -1.0 } else if up { 1.0 } else { 0.0 }
}
/// Open-close sign e (persisting when the code has no else) and the fade-2-of-3 vote ff (persisting).
fn ff23(x: &Ctx, e_persist: bool) -> Vec<f64> {
    let n = x.c.len();
    let mut e = vec![0.0; n]; let mut s = 0.0;
    for i in 0..n {
        if !e_persist { s = 0.0; }
        if x.c[i] > x.o[i] { s = 1.0; }
        if x.c[i] < x.o[i] { s = -1.0; }
        e[i] = s;
    }
    let a = mean(&e, 3);
    persist(n, |i| (a[i] < 0.0, a[i] > 0.0))
}
/// Day-of-week signal g (Tables 26.5 / 28.5 / 29.1); `else0` = 29.1 code (e/f reset to 0), else persisting.
fn dow_sig(x: &Ctx, k: &Cal, else0: bool) -> (Vec<bool>, Vec<bool>) {
    let n = x.c.len();
    let (mut e, mut f) = (vec![0.0; n], vec![0.0; n]);
    let (mut se, mut sf) = (0.0, 0.0);
    for i in 0..n {
        if else0 { se = 0.0; sf = 0.0; }
        if x.c[i] > x.o[i] { se = x.c[i] - x.o[i]; }
        if x.c[i] < x.o[i] { sf = x.o[i] - x.c[i]; }
        e[i] = se; f[i] = sf;
    }
    let (e2, e3, e4, f2, f3, f4) = (hiw(&e, 2), hiw(&e, 3), hiw(&e, 4), hiw(&f, 2), hiw(&f, 3), hiw(&f, 4));
    let c = &x.c;
    let b = (0..n).map(|i| { let c1 = at(c, i, 1); let d = k.dow[i];
        (d == 5 && c[i] > c1) || (d == 1 && c[i] < c1) || (d == 2 && e2[i] < f2[i]) || (d == 3 && e3[i] < f3[i]) || (d == 4 && e4[i] < f4[i]) }).collect();
    let s = (0..n).map(|i| { let c1 = at(c, i, 1); let d = k.dow[i];
        (d == 5 && c[i] < c1) || (d == 1 && c[i] > c1) || (d == 2 && e2[i] > f2[i]) || (d == 3 && e3[i] > f3[i]) || (d == 4 && e4[i] > f4[i]) }).collect();
    (b, s)
}
/// Month-of-year e (persisting): +1 from the first bar after month n ends, -1 after month p ends.
fn moy(k: &Cal, n: u32, p: u32) -> Vec<f64> {
    persist(k.mon.len(), |i| { let ch = i > 0 && k.mon[i] != k.mon[i - 1]; (ch && k.mon[i - 1] == n, ch && k.mon[i - 1] == p) })
}
fn domf(k: &Cal, i: usize) -> f64 { let d = k.dom[i]; if d >= 21 || d < 6 { 1.0 } else { -1.0 } }

// ---------------------------------------------------------------- catalogue

macro_rules! sd {
    ($id:expr, $name:expr, $grp:expr, $dir:expr, $st:expr, $rules:expr, $notes:expr, $claim:expr) => {
        StrategyDef { id: $id, name: $name, group: $grp, session: 0, dir: $dir, status: $st, needs_volume: false,
            needs_vix: false, needs_series: &[], book: BOOK, rules: $rules, notes: $notes, claim: $claim }
    };
}
fn cl(t: &'static str, w: &'static str, a: &'static str, h: &'static str, p: &'static str) -> Option<Claim> {
    Some(Claim { trades: t, win: w, avg: a, hold: h, period: p })
}
const SP: &str = "S&P full futures 01-02-01 to 11-19-05";
const MAP: &str = "Book tested full-size S&P/NASDAQ/Russell futures (and bonds/currencies); run on the index instead (instrument swap). Day trade: in at the next open, out at that day's close (setexitonclose). Claim = book's S&P row; book P&L is in dollars per contract ($250 per S&P point).";
const SNOTE: &str = "Instrument swap only (index for the S&P futures contract). Order fills per the module's TradeStation bar model; 1 tick = 0.10 pt US / 0.05 pt NSE.";
const IDN: &str = "Needs intraday bars (5/10/30/60-minute or 1-minute day-session bars) that the vault does not have; the rules read intraday time stamps and intraday highs/lows.";
const DS: &str = "Needs intraday (5-minute) day-session bars of the index AND of the Dow futures as data2 (the 'Dow-Spoo' net-change comparison runs bar by bar intraday). Not in the vault.";

pub fn catalogue() -> Vec<StrategyDef> {
    use Status::*;
    let g5 = "Ch 5-13 · Either-or building blocks";
    let g14 = "Ch 14-16 · Open-close, cups/caps, support-resistance";
    let g17 = "Ch 17-20 · Eight indicator system";
    let g21 = "Ch 21-22 · Second high/low exit";
    let g25 = "Ch 25 · Range expansion (sectors)";
    let g26 = "Ch 26-29 · Index calendar biases";
    let g30 = "Ch 24, 30-35, 39 · Intraday";
    let g33 = "Ch 33, 36-38, 40 · Switch, extremes, fades, N day";
    let g41 = "Ch 41 · Potpourri";
    let g42 = "Ch 42-44 · Six signal + non-either-or";
    let g4 = "Ch 4, 6, 23, 32, 45 · Other";
    vec![
        sd!("acbff_c40", "Close vs 40-day average (Table 5.2)", g5, "Long/Short", Live,
            &["E|\"if c > average(c,n) then buy next bar at market; if c < average(c,n) then sell short next bar at market; setexitonclose\" n=40 (App. p.205; text p.14)", "X|exit on the close of the entry day"],
            MAP, cl("1,228", "50%", "$43.31/trade (net $53,187, n=40)", "day", SP)),
        sd!("acbff_forever_long", "Forever Long S&P (Table 6.1)", g4, "Long", Live,
            &["E|\"if c < lowest(c,p)[1] then buy next bar at l limit\" p=37 (App. p.205): close below the previous 37 closes, buy tomorrow at today's low on a limit", "X|\"setprofittarget (1250)\" = five S&P points (text p.19: \"a five point profit—$1,250 per contract\"); no stop, no short"],
            "The book's own 'don't let this happen to you' demonstration (contract inception 1982-11-19-05). Target = 5 index points literal on every instrument. The built-in target is live from the fill onward, including the rest of the entry bar along the bar path.",
            cl("102", "100%", "$1,275.74 (13.18 bars)", "multi-day", "S&P full futures 1982 to 11-19-05")),
        sd!("acbff_2v5", "Two-day vs five-day average (Table 7.1)", g5, "Long/Short", Live,
            &["E|\"if average(c,2) < average(c,5) then buy next bar at market; if average(c,2) > average(c,5) then sell short next bar at market; setexitonclose\" (App. p.206; text p.25)", "X|setexitonclose: out at the close of the entry day"],
            MAP, cl("1,226", "52.28%", "$81.68 (net $100,137.50)", "day", SP)),
        sd!("acbff_fade2c", "Fade two like closes (Table 7.4)", g5, "Long/Short", Live,
            &["E|\"if c < c[1] and c[1] < c[2] then buy next bar at market; if c > c[1] and c[1] > c[2] then sell short next bar at market; setexitonclose\" (App. p.206; text p.25)", "X|setexitonclose: out at the close of the entry day"],
            "Book table 7.4 title says 'open-to-closes' but the code and text (p.25 \"After two like closes in a row\") use close-to-close; code used. ", cl("568", "51.76%", "$86.97 (net $49,400)", "day", SP)),
        sd!("acbff_ext50", "50-day order of extreme closes (Table 8.2)", g5, "Long/Short", Live,
            &["E|\"if highestbar(c,m) > lowestbar(c,m) then buy next bar at market; ... < ... sell short\" m=50; setexitonclose (App. p.206; text p.29)", "X|setexitonclose: out at the close of the entry day"],
            "HighestBar/LowestBar = bars ago of the extreme close, most recent on ties. Book's 50-day S&P row is in Table 8.1 (10-19-05 end date as printed).", cl("1,228", "52%", "$75.82 (net $93,113, m=50)", "day", "S&P full futures 01-02-01 to 10-19-05")),
        sd!("acbff_three", "First three either-or indicators combined (Table 9.1)", g5, "Long/Short", Live,
            &["F|x=+1/-1 c vs average(c,e); y=+1 if average(c,n)<average(c,p), -1 if >; z=+1 if highestbar(c,q)>lowestbar(c,q), -1 if < (variables keep their value otherwise); e=40 n=2 p=5 q=50 (App. p.206)", "E|\"if x+y+z > 0 then buy next bar at market; if x+y+z < 0 then sell short\"; setexitonclose", "X|setexitonclose: out at the close of the entry day"],
            "Tables 23.2/23.3 are this code with other (e,n,p,q): set the params to reproduce them (e.g. 50/2/10/30).", cl("1,228", "53.75%", "$119.98 (net $147,338)", "day", SP)),
        sd!("acbff_range10", "Range vs 10-day average range (Table 10.5)", g5, "Long/Short", Live,
            &["E|\"if (range < average(range,n) and c > c[1]) or (range > average(range,n) and c < c[1]) then buy next bar at market\"; mirror for sell short; n=10; setexitonclose (App. p.206; text p.35-37)", "F|p.half=1: only the narrow-range half (Table 10.3), p.half=2: only the wide-range half (Table 10.4) — book: \"Delete appropriate sides of the 'or' equations to individually test each half\"", "X|setexitonclose: out at the close of the entry day"],
            "range = high - low. ", cl("1,216", "52.80%", "$155.07 (net $188,563)", "day", SP)),
        sd!("acbff_hl15", "15-day high/low average (Table 11.1)", g5, "Long/Short", Live,
            &["E|\"if c > (average(h,15)+average(l,15))/2 then buy next bar at market; if c < ... sell short\"; setexitonclose (App. p.207; text p.38)", "X|setexitonclose: out at the close of the entry day"],
            MAP, cl("1,228", "49.92%", "$77.03 (net $94,588)", "day", SP)),
        sd!("acbff_five", "Five either-or indicators combined (Table 12.1)", g5, "Long/Short", Live,
            &["F|q (c vs 40-day avg), u (2 vs 5 avg), x (50-day extreme order), y (10-day range), z (15-day hi/lo mid); persisting +1/-1 (App. p.207)", "E|\"if q+u+x+y+z > 0 then buy next bar at market; if q+u+x+y+z < 0 then sell short\"; setexitonclose", "X|setexitonclose: out at the close of the entry day"],
            "The separate .doc file prints '>=3 / <-3' for 12.1 — a copy slip; the book appendix (p.207) and text (simple majority) say >0/<0, used here.", cl("1,228", "53.34%", "$169.33 (net $207,938)", "day", SP)),
        sd!("acbff_five_4of5", "Five indicators, four of five agree (Table 12.2)", g5, "Long/Short", Live,
            &["E|\"if q+u+x+y+z >= 3 then buy next bar at market; if q+u+x+y+z <= -3 then sell short\"; setexitonclose (App. p.207)", "X|setexitonclose: out at the close of the entry day"],
            MAP, cl("288", "60.07%", "$469.49 (net $135,213)", "day", SP)),
        sd!("acbff_c3_124", "Three of five combo 124 (Table 13.1)", g5, "Long/Short", Live, &["E|q+u+y > 0 buy / < 0 sell short next open; setexitonclose (App. p.207: plug three of the five variables into 12.1)", "X|setexitonclose: out at the close of the entry day"], MAP, cl("1,228", "52.77%", "$117.25", "day", SP)),
        sd!("acbff_c3_125", "Three of five combo 125 (Table 13.1)", g5, "Long/Short", Live, &["E|q+u+z > 0 buy / < 0 sell short next open; setexitonclose", "X|setexitonclose: out at the close of the entry day"], MAP, cl("1,228", "51.22%", "$68.07", "day", SP)),
        sd!("acbff_c3_134", "Three of five combo 134 (Table 13.1)", g5, "Long/Short", Live, &["E|q+x+y > 0 buy / < 0 sell short next open; setexitonclose", "X|setexitonclose: out at the close of the entry day"], MAP, cl("1,228", "54.40%", "$196.94", "day", SP)),
        sd!("acbff_c3_135", "Three of five combo 135 (Table 13.1)", g5, "Long/Short", Live, &["E|q+x+z > 0 buy / < 0 sell short next open; setexitonclose", "X|setexitonclose: out at the close of the entry day"], MAP, cl("1,228", "50.57%", "$78.41", "day", SP)),
        sd!("acbff_c3_145", "Three of five combo 145 (Table 13.1)", g5, "Long/Short", Live, &["E|q+y+z > 0 buy / < 0 sell short next open; setexitonclose", "X|setexitonclose: out at the close of the entry day"], MAP, cl("1,228", "51.55%", "$125.93", "day", SP)),
        sd!("acbff_c3_234", "Three of five combo 234 (Table 13.1)", g5, "Long/Short", Live, &["E|u+x+y > 0 buy / < 0 sell short next open; setexitonclose", "X|setexitonclose: out at the close of the entry day"], MAP, cl("1,228", "54.07%", "$180.53", "day", SP)),
        sd!("acbff_c3_235", "Three of five combo 235 (Table 13.1)", g5, "Long/Short", Live, &["E|u+x+z > 0 buy / < 0 sell short next open; setexitonclose", "X|setexitonclose: out at the close of the entry day"], MAP, cl("1,228", "53.26%", "$151.01", "day", SP)),
        sd!("acbff_c3_245", "Three of five combo 245 (Table 13.1)", g5, "Long/Short", Live, &["E|u+y+z > 0 buy / < 0 sell short next open; setexitonclose", "X|setexitonclose: out at the close of the entry day"], MAP, cl("1,228", "52.77%", "$167.62", "day", SP)),
        sd!("acbff_c3_345", "Three of five combo 345 (Table 13.1)", g5, "Long/Short", Live, &["E|x+y+z > 0 buy / < 0 sell short next open; setexitonclose", "X|setexitonclose: out at the close of the entry day"], MAP, cl("1,228", "52.93%", "$133.24", "day", SP)),
        sd!("acbff_fade_oc", "Fade yesterday's open-to-close (Table 14.1)", g14, "Long/Short", Live,
            &["E|\"if c < o then buy next bar at market; if c > o then sell short next bar at market; setexitonclose\" (App. p.207; text p.47)", "X|setexitonclose: out at the close of the entry day"], MAP, cl("1,226", "53.26%", "$129.15 (net $158,338)", "day", SP)),
        sd!("acbff_fade2oc", "Fade two same-way open-to-closes (Table 14.2)", g14, "Long/Short", Live,
            &["E|\"if c < o and c[1] < o[1] then buy next bar at market; if c > o and c[1] > o[1] then sell short\"; setexitonclose (App. p.208)", "X|setexitonclose: out at the close of the entry day"], MAP, cl("570", "55.96%", "$250.39 (net $142,725)", "day", SP)),
        sd!("acbff_cup_day", "Cups and caps · day (Table 15.1)", g14, "Long/Short", Live,
            &["F|cup: \"l > l[1] and l[1] < lowest(l,3)[2] and c > c[1] and c[1] < c[2]\"; cap mirror (App. p.208; text p.49)", "E|buy (cup) / sell short (cap) next bar at market; setexitonclose", "X|setexitonclose: out at the close of the entry day"], MAP, cl("183", "51.37%", "-$16.39 (net -$3,000)", "day", SP)),
        sd!("acbff_cup_overnight", "Cups and caps · overnight (Table 15.2)", g14, "Long/Short", Live,
            &["E|\"... then buy this bar on close; sell next bar at market\"; cap: \"sell short this bar on close; buy to cover next bar at market\" (App. p.208)", "X|\"sell next bar at market\" / \"buy to cover next bar at market\": out at the next open"],
            "Entry at the signal day's close, exit at the next open.", cl("708", "53.39%", "$50.26 (net $35,587)", "close to next open", "S&P full futures, 20 years to 11-19-05")),
        sd!("acbff_cup_3day", "Cups and caps · overnight with stops and targets (Table 15.3)", g14, "Long/Short", Live,
            &["E|cup: buy this bar on close; cap: sell short this bar on close (App. p.208)", "X|\"sell next bar at highest(h,3)+.25*average(range,3) limit; sell next bar at lowest(l,3)-(minmove/pricescale) stop; if marketposition=1 and barssinceentry=1 then sell next bar at market\"; mirror for shorts (text p.53: target/stop the day after entry, else out on the Day 3 open)"],
            SNOTE, cl("183", "55.19%", "$237.23 (net $43,413)", "2 days", SP)),
        sd!("acbff_sr20", "Three-day 20% support-resistance (Table 16.1)", g14, "Long/Short", Live,
            &["E|\"if highest(l,3)-lowest(l,3) <= n*(highest(h,3)-lowest(l,3)) then buy next bar at market; if highest(h,3)-lowest(h,3) <= n*(highest(h,3)-lowest(l,3)) then sell short\" n=.2; setexitonclose (App. p.208; text p.56)", "X|setexitonclose: out at the close of the entry day"],
            "When both conditions hold the code places both market orders: the buy fills and the short reverses it at the same open (a zero-P&L long is recorded), as TradeStation would.", cl("239", "47.70%", "$392.99 (net $93,925)", "day", SP)),
        sd!("acbff_eight", "Eight indicators · simple majority (Table 17.1)", g17, "Long/Short", Live,
            &["F|aa 2/5 avg, bb 40 avg, cc 50-day extreme order, dd 10-day range, ee 15-day hi/lo mid (persisting); ff fade 2 open-closes, gg cups/caps, hh 3-day 20% S/R (else 0) (App. p.209; text p.58)", "E|\"if aa+...+hh > 0 then buy next bar at market; < 0 sell short\"; setexitonclose", "X|setexitonclose: out at the close of the entry day"],
            "dd is printed without parentheses: '(range<avg) and c>c[1] or (range>avg) and c<c[1]'. Default reads AND before OR, which is the text's rule 4 (p.58). p.prec=1 = strict left-to-right evaluation (labelled option: TradeStation documents and/or at one precedence level).",
            cl("908", "56.28%", "$293.54 (net $266,537.50)", "day", "S&P full futures 01-04-01 to 11-19-05")),
        sd!("acbff_eight_3", "Eight indicators · +/-3 or beyond (Table 17.7)", g17, "Long/Short", Live,
            &["E|\"if aa+...+hh >= 3 then buy next bar at market; <= -3 sell short\"; setexitonclose (App. p.209)", "X|setexitonclose: out at the close of the entry day"], MAP, cl("230", "63.48%", "$638.64 (net $146,888)", "day", "S&P full futures 01-04-01 to 11-19-05")),
        sd!("acbff_eight_stop33", "Eight indicators · 33% entry stop (Table 18.4)", g17, "Long/Short", Live,
            &["E|\"if aa+...+hh > 0 then buy next bar at o of tomorrow+.33*average(range,3) stop; < 0 sell short next bar at o of tomorrow-.33*average(range,3) stop\"; setexitonclose (App. p.209; text p.66)", "X|setexitonclose: out at the close of the entry day"], SNOTE, cl("578", "55.54%", "$237.20 (net $137,100)", "day", "S&P full futures 01-04-01 to 11-19-05")),
        sd!("acbff_eight_stop33_3", "Eight indicators · 33% entry stop, +/-3 (Table 18.5)", g17, "Long/Short", Live,
            &["E|sum >= 3: buy at o of tomorrow+.33*average(range,3) stop; sum <= -3: sell short at o-.33*average(range,3) stop; setexitonclose (App. p.210)", "X|setexitonclose: out at the close of the entry day"], SNOTE, cl("155", "61.29%", "$563.39 (net $87,325)", "day", "S&P full futures 01-04-01 to 11-19-05")),
        sd!("acbff_eight_lim25", "Eight indicators · 25% limit entry (code Table 19.2)", g17, "Long/Short", Live,
            &["E|sum > 0: \"buy next bar at o of tomorrow-.25*average(range,3) limit\"; sum < 0: sell short at o+.25*average(range,3) limit; setexitonclose (App. p.210)", "X|setexitonclose: out at the close of the entry day"], "S&P result is Table 19.1 row X=0.25.", cl("586", "53%", "$142.49 (net $83,500)", "day", "S&P full futures 1-02-01 to 11-19-05")),
        sd!("acbff_eight_lim25_3", "Eight indicators · 25% limit entry, +/-3 (code Table 19.3)", g17, "Long/Short", Live,
            &["E|sum >= 3: buy at o-.25*average(range,3) limit; sum <= -3: sell short at o+.25*average(range,3) limit; setexitonclose (App. p.210)", "X|setexitonclose: out at the close of the entry day"], "S&P result is Table 19.4.", cl("136", "58.82%", "$343.57 (net $46,725)", "day", "S&P full futures 1-02-01 to 11-18-05")),
        sd!("acbff_eight_stop66", "Eight indicators · 66% stop loss (Table 20.3)", g17, "Long/Short", Live,
            &["E|sum > 0 buy / < 0 sell short next bar at market (App. p.210)", "X|\"sell next bar at o of tomorrow-.66*average(range,3) stop\"; \"buy to cover next bar at o of tomorrow+.66*average(range,3) stop\"; setexitonclose"], SNOTE, cl("908", "53.30%", "$304.01 (net $276,038)", "day", "S&P full futures 01-04-01 to 11-19-05")),
        sd!("acbff_2hl", "Second high/low exit (Table 21.1)", g21, "Long/Short", Live,
            &["E|\"if q+u+x+y+z > 0 and c > average(c,20) then buy next bar at o of tomorrow-.33*average(range,3) limit\"; mirror short (App. p.211)", "X|hh = entry-day high; for barssinceentry = 2..14: \"if highest(h,k) > hh and nthhighest(2,h,k) > hh then sell next bar at highest(h,k) limit\"", "X|\"if c < average(c,20) then sell next bar at market\"; mirror for shorts"],
            "Text (p.77) says the stop is a close under the 20-day average of the LOWS; the code uses average(c,20); code used. No profit-target orders after 14 bars in the trade (code literal).", cl("199", "54.27%", "$1,112.94 (net $221,475)", "multi-day", SP)),
        sd!("acbff_2hl_tick", "Second high/low exit · two-tick penalty (Table 21.2)", g21, "Long/Short", Live,
            &["E|as 21.1 but the entry limit is 1 tick worse: \"(o of tomorrow-(.33*average(range,3)))-(minmove/pricescale) limit\" (App. p.213)", "X|targets 1 tick beyond: \"(highest(h,k))+(minmove/pricescale) limit\"; average(c,20) market exit"], SNOTE, cl("199", "54.27%", "$1,112.94 (book docks $50/trade S&P)", "multi-day", SP)),
        sd!("acbff_2hl_2", "2d high/low exit #2 (Table 22.1)", g21, "Long/Short", Live,
            &["E|\"if c > average(c,25) then buy next bar at o of tomorrow+.5*average(range,3) stop\"; \"if c < average(c,25) then sell short next bar at o of tomorrow-(.5*average(range,3)) stop\" (App. p.215)", "X|second-high/low targets as 21.1; \"if c < average(c,25) then sell next bar at market\"; mirror"],
            "Text (p.83) says close below the 25-day average of the lows; code uses average(c,25); code used.", cl("234", "43.59%", "$710.47 (net $166,250)", "multi-day", SP)),
        sd!("acbff_2hl_3", "2d high/low exit #3 (Table 22.2)", g21, "Long/Short", Live,
            &["E|\"if c > h[1] and c > average(c,40) then buy next bar at o of tomorrow+.5*average(range,3) stop\"; mirror with c < l[1] (App. p.216)", "X|second-high/low targets; \"if c < average(c,40) then sell next bar at market\"; mirror"],
            "Text (p.83) says 'buy at market'; code uses the 50% stop; code used.", cl("97", "55.67%", "$595.88 (net $57,800)", "multi-day", SP)),
        sd!("acbff_2hl_4", "2d high/low exit #4 · limit entries (Table 22.3)", g21, "Long/Short", Live,
            &["E|\"if c > h[1] and c > average(c,40) then buy next bar at o of tomorrow-.5*average(range,3) limit\"; mirror (App. p.218)", "X|second-high/low targets; average(c,40) market exits"], SNOTE, cl("99", "63.64%", "$1,856.31 (net $183,775)", "multi-day", SP)),
        sd!("acbff_wk_rx", "One-week range expansion (Table 25.1/25.3)", g25, "Long/Short", Live,
            &["E|weekly bars: \"buy next bar at o of tomorrow+range stop; sell short next bar at o of tomorrow-range stop\" (App. p.219; text p.104: Monday's open +/- previous week's range)", "X|stop and reverse only, always in the market"],
            "Built for yen; book also prints S&P/NASDAQ/Russell (Table 25.3, contract inception). Levels = this week's first open +/- last complete week's high-low range; evaluated on the daily bars inside the week (class B: daily bars resolve which stop is hit first where a weekly bar cannot).",
            cl("189", "57.81%", "-$1,058.00 (net -$199,925)", "multi-week", "S&P full futures inception to 11-19-05")),
        sd!("acbff_rx15", "1.5 x daily range expansion (Table 25.4/25.5)", g25, "Long/Short", Live,
            &["E|\"if c > average(c,n) then buy next bar at o of tomorrow+(q*range) stop; if c < average(c,n) then sell short next bar at o of tomorrow-(q*range) stop\" q=1.5 n=25 (App. p.220)", "X|\"sell next bar at o of tomorrow-(q*range) stop\"; \"buy to cover next bar at o of tomorrow+(q*range) stop\""],
            "Bond system; S&P row Table 25.5 (contract inception).", cl("229", "36.24%", "$237.99 (net $54,500)", "multi-day", "S&P full futures inception to 11-19-05")),
        sd!("acbff_monday", "Monday rule (Table 26.1)", g26, "Long/Short", Live,
            &["E|\"if dayofweek(date)=5 and c > c[1] then buy next bar at market; if dayofweek(date)=5 and c < c[1] then sell short\"; setexitonclose (doc Table 26.1 / App. 26.5 lines)", "X|setexitonclose: out at the close of the entry day"], MAP, cl("244", "46.72%", "$120.49 (net $29,400)", "day", SP)),
        sd!("acbff_tuesday", "Turnaround Tuesday (Table 26.3)", g26, "Long/Short", Live,
            &["E|\"if dayofweek(date)=1 and c < c[1] then buy next bar at market; if dayofweek(date)=1 and c > c[1] then sell short\"; setexitonclose", "X|setexitonclose: out at the close of the entry day"], MAP, cl("229", "50.66%", "$94.10 (net $21,550)", "day", SP)),
        sd!("acbff_wed_fri", "Wednesday-Friday fade biggest open-close (Table 26.4)", g26, "Long/Short", Live,
            &["F|\"if c > o then e=c-o; if c < o then f=o-c\" (e, f keep their last value)", "E|Tue/Wed/Thu signal (dayofweek 2/3/4): highest(e,k) < highest(f,k) buy, > sell short, k = 2/3/4; setexitonclose", "X|setexitonclose: out at the close of the entry day"], MAP, cl("749", "54.34%", "$262.93 (net $196,938)", "day", SP)),
        sd!("acbff_dow", "Days of the week indicator · complete (Table 26.5)", g26, "Long/Short", Live,
            &["E|all five day rules together (App. p.220 Table 26.5)", "X|setexitonclose"], MAP, cl("1,222", "52.13%", "$202.85 (net $247,888)", "day", SP)),
        sd!("acbff_short_dom", "Perennially short S&P (Table 27.1)", g26, "Short", Live,
            &["E|\"if dayofmonth(date) >= p and dayofmonth(date) <= (p+3) then sell short next bar at market\" p=6 (App. p.220)", "X|\"if dayofmonth(date) >= n and dayofmonth(date) <= (n+3) then buy to cover next bar at market\" n=21"],
            "Calendar rule; perpetual short half.", cl("280", "48.21%", "$288.13 (net $80,675)", "~11.5 bars", "S&P full futures inception to 11-19-05")),
        sd!("acbff_dom", "Days of month indicator · perpetual (Table 27.3)", g26, "Long/Short", Live,
            &["E|\"if dayofmonth(date) >= n and dayofmonth(date) <= (n+3) then buy next bar at market; if dayofmonth(date) >= p and dayofmonth(date) <= (p+3) then sell short next bar at market\" n=21 p=6 (App. p.221)", "X|stop and reverse on the opposite signal (no other exit in the code)"], "Always long or short.", cl("116", "60.34%", "$1,565.73 (net $181,625)", "~15 days", SP)),
        sd!("acbff_dom_day", "Days of month · day trades (Table 27.4)", g26, "Long/Short", Live,
            &["E|\"if dayofmonth(date) >= n or dayofmonth(date) < p then buy next bar at market; if dayofmonth(date) >= p and dayofmonth(date) < n then sell short\"; setexitonclose (App. p.221)", "X|setexitonclose: out at the close of the entry day"], MAP, cl("1,228", "50.90%", "$100.95 (net $123,963)", "day", SP)),
        sd!("acbff_dom_day_cc", "Days of month · buy after lower closes, sell after higher (Table 27.5)", g26, "Long/Short", Live,
            &["E|\"if c < c[1] and dayofmonth(date) >= n or dayofmonth(date) < p then buy next bar at market; if c > c[1] and dayofmonth(date) >= p and dayofmonth(date) < n then sell short\"; setexitonclose (App. p.221)", "X|setexitonclose: out at the close of the entry day"],
            "As printed the lower-close filter binds only to the dom>=21 days (either and/or precedence gives this); text p.119 says \"Take longs only after lower closes\": p.close_all=1 (labelled option) applies it to every long day.", cl("583", "52.14%", "$178.28 (net $103,938)", "day", SP)),
        sd!("acbff_moy", "Index month of year (Table 28.1)", g26, "Long/Short", Live,
            &["E|\"m=month(date); if m <> m[1] and m[1]=n then e=1; if m <> m[1] and m[1]=p then e=-1; if e=1 then buy next bar at market; if e=-1 then sell short\" n=10 p=4 (App. p.221)", "X|stop and reverse on the opposite signal (no other exit in the code)"], "Long from the second November bar to the second May bar, then short.", cl("46", "54.35%", "$4,407 (net $202,713)", "~6 months", "S&P full futures inception to 11-19-05")),
        sd!("acbff_moy_dom", "Month of year + day of month (Table 28.3)", g26, "Long/Short", Live,
            &["F|f=+1 if dayofmonth >= 21 or < 6, -1 if 6..20 (App. p.222)", "E|\"if e+f=2 then buy next bar at market; if e+f=-2 then sell short\"", "X|\"if e+f < 2 then sell next bar at market; if e+f > -2 then buy to cover\""], "", cl("300", "56.67%", "$910.17 (net $273,050)", "multi-day", "S&P full futures inception to 11-19-05")),
        sd!("acbff_moy_dom_day", "Month of year + day of month · day system (Table 28.4)", g26, "Long/Short", Live,
            &["E|\"if o of tomorrow < c and e+f=2 then buy next bar at market; if o of tomorrow > c and e+f=-2 then sell short\"; setexitonclose (App. p.222)", "X|setexitonclose: out at the close of the entry day"], MAP, cl("263", "54.75%", "$360.17 (net $94,725)", "day", SP)),
        sd!("acbff_cal3", "Three calendar indicators agree (Table 28.5)", g26, "Long/Short", Live,
            &["F|g = day-of-week signal (persisting), x = month of year, u = day of month (App. p.222)", "E|\"if x+u+g=3 then buy next bar at market; if x+u+g=-3 then sell short\"", "X|\"if x+u+g < 3 then sell next bar at market; if x+u+g > -3 then buy to cover\""], "", cl("120", "60.83%", "$687.71 (net $82,525)", "multi-day", SP)),
        sd!("acbff_cal2_day", "Two of three calendars · day system (Table 28.6)", g26, "Long/Short", Live,
            &["E|\"if o of tomorrow < c and x+u+g > 0 then buy next bar at market; if o of tomorrow > c and x+u+g < 0 then sell short\"; setexitonclose (App. p.223)", "X|setexitonclose: out at the close of the entry day"], MAP, cl("547", "53.02%", "$298.45 (net $163,250)", "day", SP)),
        sd!("acbff_cal_eight", "Monthly date + day of week + five indicators (Table 29.1)", g26, "Long/Short", Live,
            &["F|aaa day of week (e,f reset to 0), bbb day of month, ccc = sign of the five either-or indicators (persisting) (App. p.223)", "E|\"if aaa+bbb+ccc > 0 then buy next bar at market; if aaa+bbb+ccc < 0 then sell short\" — always in the market", "X|stop and reverse on the opposite signal (no other exit in the code)"],
            "Text calls it 'previous eight indicators' but the code combines the five; code used. x (range) printed without parentheses, AND-first default (p.prec=1 left-to-right).", cl("376", "58.51%", "$1,066.36 (net $400,950)", "4.26 bars", SP)),
        sd!("acbff_rx_filter", "Range expansion with two-same-way filter (Table 33.1)", g33, "Long/Short", Live,
            &["F|x=1 if two closes in a row the same way, else 0; filter average(x,n) >= u, n=20 u=.6 (App. p.227)", "E|\"if average(x,n) >= u then buy next bar at o of tomorrow+(.5*average(range,3)) stop\"; mirror short", "X|\"sell next bar at o of tomorrow-(.5*average(range,3)) stop\"; \"buy to cover next bar at o of tomorrow+(.5*average(range,3)) stop\""],
            "p.u=0 = the book's 'without filter' row (text p.148: perpetual breakout every day).", cl("216", "40.28%", "$279.75 (net $60,425)", "multi-day", "S&P full futures 01-02-96 to 11-19-05")),
        sd!("acbff_5c_day", "First contrary close after five same-way closes · day (Table 36.1)", g33, "Long/Short", Live,
            &["E|\"if c > c[1] and c[1] < c[2] and c[2] < c[3] and c[3] < c[4] and c[4] < c[5] and c[5] < c[6] then buy next bar at market\"; mirror; setexitonclose (App. p.229)", "X|setexitonclose: out at the close of the entry day"], MAP, cl("59", "54.24%", "$552.33 (net $32,588)", "day", "S&P full futures 1-2-96 to 11-19-05")),
        sd!("acbff_5c_perp", "Five same-way closes then opposite · perpetual (Table 36.2)", g33, "Long/Short", Live,
            &["E|\"if c < c[1] and c[1] > c[2] and ... c[5] > c[6] then buy next bar at market\"; mirror sell short; no other exit (App. p.229)", "X|stop and reverse on the opposite signal (no other exit in the code)"], "Stop and reverse only.", cl("24", "58.33%", "$11,286 (net $270,863)", "long-term", "S&P full futures 1-2-96 to 11-19-05")),
        sd!("acbff_5c1_perp", "Five one-way, one opposite, one irrelevant · perpetual (Table 36.3)", g33, "Long/Short", Live,
            &["E|\"if c[1] < c[2] and c[2] > c[3] and ... c[6] > c[7] then buy next bar at market\"; mirror (App. p.230)", "X|stop and reverse on the opposite signal (no other exit in the code)"], "Stop and reverse only.", cl("24", "66.67%", "$13,351.56 (net $320,438)", "long-term", "S&P full futures 1-2-96 to 11-19-05")),
        sd!("acbff_5c1_day", "Five one-way, one opposite, one irrelevant · day (Table 36.3)", g33, "Long/Short", Live,
            &["E|as 36.3 perpetual + \"setexitonclose\" (App. p.230: type it at the end for the day version)", "X|setexitonclose: out at the close of the entry day"], MAP, cl("59", "64.41%", "$760.59 (net $44,875)", "day", "S&P full futures 1-2-96 to 11-19-05")),
        sd!("acbff_3rd_fade", "Third day fade (Table 37.1)", g33, "Long/Short", Live,
            &["E|\"if marketposition < 1 and c < l[1] and c[1] < l[2] then buy next bar at o of tomorrow-.33*average(range,3) limit\"; mirror (App. p.230)", "X|\"if marketposition=1 then sell next bar at h limit; ... sell next bar at l-(minmove/pricescale) stop; ... setexitonclose\": the day after entry, target = entry-day high, stop 1 tick under entry-day low, else out on that close (text p.163)"],
            "Text says open minus 50% of the 3-day range; code says .33; code used. Exit timing = text (hold overnight, out on the next day's close).", cl("219", "51.14%", "$487.87 (net $106,188)", "2 days", "S&P full futures 1-2-96 to 11-19-05")),
        sd!("acbff_5d_avg_fade", "Closes beyond 5-day low/high averages (Table 37.2)", g33, "Long/Short", Live,
            &["E|\"x=1 if c < average(l,5); if marketposition < 1 and lowest(x,3)=1 and c < c[1] and c[1] < c[2] then buy next bar at market\"; mirror with c > average(h,5) (App. p.230)", "X|\"if marketposition=1 and barssinceentry > 0 then sell next bar at h limit / at l-(minmove/pricescale) stop\"; exit on the close of the third day (text p.164: \"Stay in the position over two nights. On the third day ... Failure to hit either means exit on the close\")"],
            "Code's 'barssinceentry > 1 then setexitonclose' would exit a day later under next-bar timing; default follows the text (out on the 3rd day's close). p.exit_late=1 = one day later (labelled option).", cl("187", "55.98%", "$969.69 (net $181,313)", "3 days", "S&P full futures 1-2-96 to 11-19-05")),
        sd!("acbff_nday_10stop", "N day with 10-day stops (Table 38.3)", g33, "Long/Short", Live,
            &["E|\"if marketposition < 1 then buy next bar at highest(h,20) stop; if marketposition > -1 then sell short next bar at lowest(l,20) stop\" (App. p.231)", "X|\"if barssinceentry >= 10 and (c[10] > highest(c,10) or h[10] > highest(h,10)) then sell next bar at market\"; mirror (text p.169: no new high and high close vs 11 days back)"],
            "The separate .doc prints c[10] twice; the book appendix has h[10]; book used. The plain 20-day N day (Table 38.1) is a vault duplicate of rwmts_cb20 and is not catalogued again.", cl("52", "40.38%", "$1,438.46 (net $74,800)", "multi-week", SP)),
        sd!("acbff_fade_rev", "Fading reversals (Table 40.1)", g33, "Long/Short", Live,
            &["E|\"if marketposition < 1 and h > h[1] and c < c[1] then buy next bar at market\"; \"if marketposition > -1 and l < l[1] and c > c[1] then sell short\" (App. p.231)", "X|\"if (h <= h[1] or c >= c[1]) then sell next bar at hh limit\" (hh = entry-day high); stop 1 tick under the entry-day low from the day after entry (text p.175); exit orders not placed on a same-direction signal day"],
            "Code uses ll (the SHORT side's entry-day low) for the long stop and hh for the short stop — a swapped variable. Default follows the text (entry-day low / high); p.stop_ref=1 = the code's swapped variables (labelled option).", cl("325", "58.15%", "$192.46 (net $62,550)", "multi-day", "S&P full futures 01-01-01 to 11-19-05")),
        sd!("acbff_rx4day", "200% range expansion · four days in trade (Table 41.1)", g41, "Long/Short", Live,
            &["E|\"buy next bar at o of tomorrow+2*average(range,3) stop; sell short next bar at o of tomorrow-(2*average(range,3)) stop\" (App. p.232)", "X|\"if barssinceentry=2 then sell next bar at h limit / at l stop / setexitonclose\"; mirror (text p.176: fourth day in the trade: previous day's high target, low stop, else close)"], SNOTE, cl("153", "49.02%", "$35.21 (net $5,388)", "4 days", "S&P full futures 1-2-86 to 11-19-05")),
        sd!("acbff_mom66", "Nine-day 66% momentum (Table 41.3)", g41, "Long/Short", Live,
            &["F|hc = highest(h,9)-c; lc = c-lowest(l,9); xx = the bigger (App. p.232)", "E|\"if mp < 1 and hc > lc then buy next bar at o of tomorrow+(.66*lc) stop; if mp > -1 and lc > hc then sell short next bar at o of tomorrow-(.66*hc) stop\"", "X|\"if mp=1 and barssinceentry > 0 then sell next bar at entryprice-(1.32*xx) stop\"; mirror"], SNOTE, cl("188", "62.77%", "$952.79 (net $179,125)", "7.3 bars", SP)),
        sd!("acbff_contract", "Contracting range · fade close on momentum (Table 41.6)", g41, "Long/Short", Live,
            &["E|\"if (average(range,10) < average(range,25)) and (c < c[1] and c < average(c,5)) then buy next bar (o of tomorrow+(.2*range)) stop\"; mirror (App. p.232)", "X|\"if (c > average(c,10) and c > average(c,25)) then sell next bar at (o of tomorrow-(.2*range)) stop; if barssinceentry > 0 then sell next bar at entryprice-average(range,25) stop\"; mirror"],
            "Text's conditional stop uses the previous close and 5-day average; the code uses the 10- and 25-day averages; code used.", cl("257", "46.69%", "$540.37 (net $138,875)", "multi-day", SP)),
        sd!("acbff_5v15", "5 vs 15-day closing averages (Table 41.7)", g41, "Long/Short", Live,
            &["E|5-day average above the 15-day for 3 days after being below: buy next bar at market; mirror sell short (App. p.233)", "X|stop and reverse only (\"No stops\", text p.181)"], "", cl("76", "40.79%", "$651.97 (net $49,500)", "multi-week", SP)),
        sd!("acbff_3v30", "3-day / 30-day crossover (Table 41.11)", g41, "Long/Short", Live,
            &["E|average(c,3) below average(c,30) 3 bars ago and above for the last 3 bars: buy next bar at market; mirror (App. p.233)", "X|\"if average(c,3) < average(c,30) then sell next bar at market\"; mirror"], "", cl("52", "36.54%", "$1,720.67 (net $89,475)", "multi-week", SP)),
        sd!("acbff_15_30", "15-30 day close indicator (Table 41.12)", g41, "Long/Short", Live,
            &["E|\"if c > c[15] and c > c[30] then buy next bar at market; if c < c[15] and c < c[30] then sell short\" (App. p.234)", "X|\"if c < c[15] or c < c[30] then sell next bar at market; if c > c[15] or c > c[30] then buy to cover\""], "", cl("114", "44.74%", "$396.71 (net $45,225)", "multi-day", SP)),
        sd!("acbff_fade23oc", "Fade two of three open-to-closes (Table 42.1)", g42, "Long/Short", Live,
            &["E|\"e=1 if c>o, -1 if c<o (kept otherwise); if average(e,3) < 0 then buy next bar at market; if > 0 sell short\"; setexitonclose (App. p.234)", "X|setexitonclose: out at the close of the entry day"], MAP, cl("1,228", "53.75%", "$165.48 (net $203,213)", "day", SP)),
        sd!("acbff_six", "Six indicator · simple majority (Table 42.2)", g42, "Long/Short", Live,
            &["F|the five either-or indicators + ff = fade 2-of-3 open-closes (App. p.234)", "E|\"if aa+bb+cc+dd+ee+ff > 0 then buy next bar at market; < 0 sell short\"; setexitonclose", "X|setexitonclose: out at the close of the entry day"], "dd printed without parentheses; AND-first default (p.prec=1 left-to-right).", cl("666", "57.81%", "$389.25 (net $259,238)", "day", SP)),
        sd!("acbff_six_stop66", "Six indicator · 66% stop loss (Table 42.3)", g42, "Long/Short", Live,
            &["E|six indicator entries at market (App. p.235)", "X|\"sell next bar at o of tomorrow-.66*average(range,3) stop; buy to cover next bar at o of tomorrow+.66*average(range,3) stop\"; setexitonclose"], SNOTE, cl("666", "57.81%", "$363.76 (net $242,263)", "day", SP)),
        sd!("acbff_nee5", "Five combined non-either-or indicators (Table 43.1)", g42, "Long/Short", Live,
            &["F|aa (5 closes pattern c[1..6]), bb (c[1..7]), cc cups/caps, dd 3-day 20% S/R, ee fade 2 open-closes; each +1/-1/0 (App. p.235)", "E|\"if aa+bb+cc+dd+ee > 0 then buy next bar at market; < 0 sell short\"; setexitonclose", "X|setexitonclose: out at the close of the entry day"], MAP, cl("767", "54.37%", "$248.73 (net $190,775)", "day", SP)),
        sd!("acbff_six_q", "Six indicator with five non-either-or qualifier (Table 44.1)", g42, "Long/Short", Live,
            &["E|\"if aa+...+ff > 0 and aaa+bbb+ccc+ddd+eee >= 0 then buy next bar at market; if aa+...+ff < 0 and aaa+...+eee <= 0 then sell short\"; setexitonclose (App. p.236; e reset to 0 on doji days as coded)", "X|setexitonclose: out at the close of the entry day"], MAP, cl("522", "60.73%", "$549.21 (net $286,688)", "day", SP)),
        sd!("acbff_six_q66", "Filtered six indicator · 66% stop (Table 44.2)", g42, "Long/Short", Live,
            &["E|as 44.1 (App. p.237)", "X|66% of the 3-day average range stop from the open; setexitonclose"], SNOTE, cl("522", "57.66%", "$531.01 (net $277,188)", "day", SP)),
        sd!("acbff_six_q66_v2", "Filter indicator · 66% stop · version 2 (Table 44.3)", g42, "Long/Short", Live,
            &["E|\"(six > 0 and five >= 0) or (six = 0 and five > 0) then buy next bar at market\"; mirror (App. p.237)", "X|66% stop; setexitonclose"], SNOTE, cl("897", "54.07%", "$338.39 (net $303,538)", "day", SP)),
        // ---- needs data
        sd!("acbff_mega_1min", "One-minute 'Mega System' (Table 24.1)", g30, "Long/Short", NeedsData(IDN), &["E|1-minute e-mini S&P bars 8:45-14:55: buy at open-average(o-l,5) limit, sell short at open+average(h-o,5) limit; exit 15:00 (App. p.219)"], "", None),
        sd!("acbff_dowspoo_perp", "Dow-Spoo perpetual (Table 30.1)", g30, "Long/Short", NeedsData(DS), &["E|5-min bars: long if the S&P's net change % since yesterday's close beats the Dow's, else short (App. p.224)"], "", None),
        sd!("acbff_dowspoo_day", "Dow-Spoo basic day trade (Table 30.2)", g30, "Long/Short", NeedsData(DS), &["E|5-min bars 8:45-14:45 in the Dow-Spoo direction; exit 15:00 (App. p.224)"], "", None),
        sd!("acbff_dowspoo_3bar", "Dow-Spoo with 3-bar requirement (Table 30.5)", g30, "Long/Short", NeedsData(DS), &["E|9:30-14:45, last 3 five-minute bars agree; exit 15:00 (App. p.224)"], "", None),
        sd!("acbff_dowspoo_50", "Dow-Spoo with 50-bar average (Figure 30.8)", g30, "Long/Short", NeedsData(DS), &["E|5-min close crosses its 50-bar average with Dow-Spoo agreeing (App. p.224)"], "", None),
        sd!("acbff_dowspoo_6stop", "Dow-Spoo 6-bar high/low stop (Table 30.10)", g30, "Long/Short", NeedsData(DS), &["E|6 bars agree: buy at the 6-bar high stop; exit 15:00 (App. p.225)"], "", None),
        sd!("acbff_dowspoo_6lim", "Dow-Spoo 6-bar high/low limit (Table 30.11)", g30, "Long/Short", NeedsData(DS), &["E|c > average(h,6) and Dow-Spoo up: buy at c-.5*range limit; exit 15:00 (App. p.225)"], "", None),
        sd!("acbff_dowspoo_dyn", "Dynamic Dow-Spoo (Table 30.12)", g30, "Long/Short", NeedsData(DS), &["E|12-48 bar Dow-Spoo averages by half hour 9:30-13:00; exit 15:00 (App. p.225)"], "", None),
        sd!("acbff_open_rel", "Entering re daily opening (Table 31.2)", g30, "Long/Short", NeedsData(IDN), &["E|30-min bars: at 9:30 index time, long if above the day's open, short if below; out 15:15 (App. p.226)"], "", None),
        sd!("acbff_2bar_open", "Two bars in a row vs open · 60-min (Table 31.3)", g30, "Long/Short", NeedsData(IDN), &["E|60-min bars 10:30-14:30 (App. p.226)"], "", None),
        sd!("acbff_final_third", "Final third of day (Table 31.4)", g30, "Long/Short", NeedsData(IDN), &["E|135-min bars: at 13:00 both bars above the open buy, below sell short; out 15:15 (App. p.226)"], "", None),
        sd!("acbff_1230", "12:30 entry system (Table 34.1)", g30, "Long/Short", NeedsData(IDN), &["E|5-min bars: at 12:30 open in lowest 25% of range and highs rising since 9:30/11:30; exit 15:05 (App. p.227)"], "", None),
        sd!("acbff_9oc", "Majority of 9 open-closes · 5-min (Table 35.1)", g30, "Long/Short", NeedsData(IDN), &["E|5-min bars 9:15-14:45 (App. p.228)"], "", None),
        sd!("acbff_midpoint", "Daily midpoint demarcation · 5-min (Table 35.2)", g30, "Long/Short", NeedsData(IDN), &["E|5-min bars 9:15-14:45 vs (day high+day low)/2 (App. p.228)"], "", None),
        sd!("acbff_4ind", "Four indicator intraday system (Tables 35.3-35.6)", g30, "Long/Short", NeedsData(DS), &["E|Dow-Spoo, vs open, 9 open-closes, midpoint all agree; 35.6 adds the two-bar exit (App. pp.228-229)"], "", None),
        sd!("acbff_rsi_5min", "Five-minute RSI go-with (Table 39.1)", g30, "Long/Short", NeedsData(IDN), &["E|RSI(9) on 5-min bars > 85 buy, < 15 sell short 9:15-14:45; exit 15:10 (App. p.231)"], "", None),
        sd!("acbff_bond_index", "Bonds lead the indexes (Table 41.14)", g30, "Long/Short", NeedsData("Needs 10-minute e-mini S&P bars AND 10-minute 30-year T-bond futures bars (data2) for the 10:00 entry / 15:10 exit."), &["E|10:00: S&P above its open and bonds in the top/bottom 25% of their range (App. p.234)"], "", None),
        sd!("acbff_bond_basis", "Basis-point bond system (Ch 4)", g4, "Long/Short", NeedsData("Needs 30-year T-bond futures daily prices in 32nds (and the pre-1990s 8:00 CBOT open); the rule keys off whole-point bond prices."), &["E|open in a different point than the previous close: stop entry on crossing 00 back into the original point (p.11)", "X|day 2: day-1 low target (short), high+1 tick stop, else out on the close"], "", None),
        sd!("acbff_bean_dual", "Fading closes · dual entry soybeans (Ch 4)", g4, "Long/Short", NeedsData("Needs soybean futures daily data."), &["E|after a down close: open below the low -> buy stop at the close; open between low and close -> buy at the high; exits as the bond system (p.12)"], "", None),
        sd!("acbff_bean_switch", "Soybean switch (Table 32.1)", g4, "Long/Short", NeedsData("Soybean futures system (Tables 32.1-32.2 list only grains, metals, softs and currencies)."), &["E|avg(range,5) >= 2*avg(range,20)[5] or close in top 10% of the 20-day range: buy at open+range stop, sell short at open-range stop (App. p.227)", "X|target open+.5*20-day range, stop open-.4*range"], "", None),
        sd!("acbff_bean_march", "Soybean seasonal · buy in March (Table 41.8)", g41, "Long", NeedsData("Needs November soybean futures (dollar target $2,000 / stop $4,000)."), &["E|first trading day of March buy (App. p.233)", "X|$2,000 target, $4,000 stop, else out in July"], "", None),
        sd!("acbff_bean_july", "Soybean seasonal · sell in July (Table 41.10)", g41, "Short", NeedsData("Needs November soybean futures (dollar target $4,000 / stop $5,000)."), &["E|first trading day of July sell short (App. p.233)", "X|$4,000 target, $5,000 stop, else out in November"], "", None),
        // ---- not testable
        sd!("acbff_ch4_sp_range", "S&P range sell stop (Black Monday, Ch 4)", g4, "Short", NotTestable("Entry only (low minus previous range sell stop, p.12); the book gives no exit."), &["E|subtract the previous daily range from the low for a sell stop (p.12)"], "", None),
        sd!("acbff_walkforward", "Annual re-optimisation (Ch 23)", g4, "Long/Short", NotTestable("A parameter-selection procedure (best ROA over 600 (e,n,p,q) combinations per 5-year block), not a trading rule; the fixed-parameter rows (Tables 23.2/23.3) are acbff_three with those params."), &["F|optimise e 5-50, n 2-4, p 5/10, q 5-50 on the prior five years, trade the next year (p.88)"], "", None),
        sd!("acbff_quiz_20ma", "Quiz example: 20-day average + 75% range stop (Ch 45)", g4, "Long/Short", NotTestable("Quiz illustration with no code: whether 'the previous 20-day closing average' includes yesterday's close is not stated (p.198)."), &["E|close above the previous 20-day average: buy next open + 75% of the previous day's range on a stop; vice versa; perpetual"], "", None),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    if !id.starts_with("acbff_") { return None; }
    let d = catalogue().into_iter().find(|d| d.id == id)?;
    let fam = match d.group {
        g if g.starts_with("Ch 5-13") => ("acbff_eitheror", "Collins either-or building blocks"),
        g if g.starts_with("Ch 14") => ("acbff_patterns", "Collins open-close / cups / S-R"),
        g if g.starts_with("Ch 17") => ("acbff_eight", "Collins eight indicator system"),
        g if g.starts_with("Ch 21") => ("acbff_2hl", "Collins second high/low exit"),
        g if g.starts_with("Ch 25") => ("acbff_rx", "Collins range expansion"),
        g if g.starts_with("Ch 26") => ("acbff_calendar", "Collins index calendar biases"),
        g if g.starts_with("Ch 24") => ("acbff_intraday", "Collins intraday systems"),
        g if g.starts_with("Ch 33") => ("acbff_extremes", "Collins switch / extremes / fades"),
        g if g.starts_with("Ch 41") => ("acbff_potpourri", "Collins potpourri"),
        g if g.starts_with("Ch 42") => ("acbff_six", "Collins six signal system"),
        _ => ("acbff_other", "Collins other"),
    };
    Some((fam.0, fam.1, d.name))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    let tick = || p("tick", "Tick (0 = auto: 0.10 US / 0.05 NSE)", 0.0, 0.0, 5.0, 0.05);
    let prec = || p("prec", "dd and/or: 0 AND-first (text), 1 left-to-right", 0.0, 0.0, 1.0, 1.0);
    if !id.starts_with("acbff_") { return None; }
    let v = match id {
        "acbff_c40" => vec![p("n", "Average length", 40., 5., 100., 5.)],
        "acbff_forever_long" => vec![p("p", "Lowest-close lookback", 37., 1., 60., 1.), p("target", "Profit target (index points)", 5., 0.5, 100., 0.5)],
        "acbff_ext50" => vec![p("m", "Extreme lookback", 50., 5., 100., 5.)],
        "acbff_three" => vec![p("e", "Close average", 40., 5., 100., 5.), p("n", "Short average", 2., 1., 10., 1.), p("p", "Long average", 5., 2., 20., 1.), p("q", "Extreme lookback", 50., 5., 100., 5.)],
        "acbff_range10" => vec![p("n", "Range average", 10., 3., 50., 1.), p("half", "0 both halves, 1 narrow (10.3), 2 wide (10.4)", 0., 0., 2., 1.)],
        "acbff_hl15" => vec![p("n", "High/low average", 15., 3., 50., 1.)],
        "acbff_sr20" => vec![p("n", "Fraction of the 3-day range", 0.2, 0.05, 0.5, 0.05)],
        "acbff_eight" | "acbff_eight_3" | "acbff_six" | "acbff_six_q" | "acbff_cal_eight" => vec![prec()],
        "acbff_eight_stop33" | "acbff_eight_stop33_3" => vec![p("k", "Entry stop x 3-day avg range", 0.33, 0., 1.5, 0.01), prec()],
        "acbff_eight_lim25" | "acbff_eight_lim25_3" => vec![p("k", "Entry limit x 3-day avg range", 0.25, 0., 1.5, 0.01), prec()],
        "acbff_eight_stop66" | "acbff_six_stop66" | "acbff_six_q66" | "acbff_six_q66_v2" => vec![p("k", "Stop x 3-day avg range", 0.66, 0.1, 1.5, 0.01), prec()],
        "acbff_2hl" => vec![p("k", "Entry limit x 3-day avg range", 0.33, 0., 1.5, 0.01), p("ma", "Exit average", 20., 5., 60., 1.)],
        "acbff_2hl_tick" => vec![p("k", "Entry limit x 3-day avg range", 0.33, 0., 1.5, 0.01), p("ma", "Exit average", 20., 5., 60., 1.), tick()],
        "acbff_2hl_2" => vec![p("k", "Entry stop x 3-day avg range", 0.5, 0., 1.5, 0.05), p("ma", "Average", 25., 5., 60., 1.)],
        "acbff_2hl_3" | "acbff_2hl_4" => vec![p("k", "Entry x 3-day avg range", 0.5, 0., 1.5, 0.05), p("ma", "Average", 40., 5., 80., 1.)],
        "acbff_wk_rx" => vec![p("q", "Weekly range multiple", 1., 0.25, 3., 0.25)],
        "acbff_rx15" => vec![p("q", "Range multiple", 1.5, 0.25, 3., 0.25), p("n", "Average", 25., 5., 60., 1.)],
        "acbff_short_dom" | "acbff_dom" | "acbff_dom_day" => vec![p("n", "Long start day", 21., 1., 28., 1.), p("p", "Short start day", 6., 1., 28., 1.)],
        "acbff_dom_day_cc" => vec![p("n", "Long start day", 21., 1., 28., 1.), p("p", "Short start day", 6., 1., 28., 1.), p("close_all", "Lower-close filter on all long days (text p.119)", 0., 0., 1., 1.)],
        "acbff_moy" | "acbff_moy_dom" | "acbff_moy_dom_day" | "acbff_cal3" | "acbff_cal2_day" => vec![p("n", "Month ending the short (long from next)", 10., 1., 12., 1.), p("p", "Month ending the long", 4., 1., 12., 1.)],
        "acbff_rx_filter" => vec![p("u", "Filter threshold (0 = no filter)", 0.6, 0., 1., 0.05), p("n", "Filter days", 20., 5., 50., 1.), p("k", "Stop x 3-day avg range", 0.5, 0.1, 2., 0.05)],
        "acbff_cup_3day" | "acbff_3rd_fade" => vec![tick()],
        "acbff_5d_avg_fade" => vec![tick(), p("exit_late", "1 = exit one day later (code under next-bar timing)", 0., 0., 1., 1.)],
        "acbff_fade_rev" => vec![tick(), p("stop_ref", "0 text (entry-day low/high), 1 code's swapped variables", 0., 0., 1., 1.)],
        "acbff_nday_10stop" => vec![p("n", "Breakout days", 20., 5., 60., 1.), p("m", "Failure lookback", 10., 3., 30., 1.)],
        "acbff_rx4day" => vec![p("k", "Range multiple", 2., 0.5, 4., 0.25)],
        "acbff_mom66" => vec![p("n", "Lookback", 9., 3., 30., 1.), p("k", "Entry fraction", 0.66, 0.1, 1.5, 0.01), p("s", "Stop multiple", 1.32, 0.5, 3., 0.01)],
        "acbff_15_30" => vec![p("a", "Near close back", 15., 2., 60., 1.), p("b", "Far close back", 30., 4., 120., 1.)],
        "acbff_5c_day" => vec![p("swap", "Sides: 0 = book (Table 36.1: buy the up close after five down closes), 1 = [Not from the book] old swapped sides", 0., 0., 1., 1.)], // intraday wave 2b
        _ => vec![],
    };
    Some(v)
}

// ---------------------------------------------------------------- run

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !id.starts_with("acbff_") { return None; }
    let defs = catalogue();
    let d = defs.iter().find(|d| d.id == id)?;
    if d.status != Status::Live || from > to || to >= x.r.len() { return None; }
    let schema = params(id).unwrap_or_default();
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map(|d| d.def).unwrap_or(0.0));
    let u = |k: &str| g(k).round().max(1.0) as usize;
    let india = x.r.first().is_some_and(|r| { let t = r.ticker.to_uppercase(); t.starts_with("NIFTY") || t.starts_with("BASKET") || t.contains("INDIA") || t.contains("SENSEX") });
    let tick = if g("tick") > 0.0 { g("tick") } else if india { 0.05 } else { 0.10 };
    let lr = g("prec") >= 0.5;
    let (c, op, h, l) = (&x.c, &x.o, &x.h, &x.l);
    let n = c.len();
    let r = rng(x);
    let ar3 = mean(&r, 3);
    use A::*;
    // generic day trade from a signal: +1 buy / -1 short / 2 both (buy then short, code order)
    let day = |sig: &dyn Fn(usize) -> (bool, bool)| -> Vec<Trade> {
        engine(x, from, to, |i, _, nx, _| {
            let (b, s) = sig(i);
            if b { put(nx, Buy, T::Mkt, "buy"); }
            if s { put(nx, Short, T::Mkt, "sell short"); }
            day_exit(nx);
        })
    };
    let perp = |sig: &dyn Fn(usize) -> (bool, bool)| -> Vec<Trade> {
        engine(x, from, to, |i, _, nx, _| {
            let (b, s) = sig(i);
            if b { put(nx, Buy, T::Mkt, "buy"); }
            if s { put(nx, Short, T::Mkt, "sell short"); }
        })
    };
    let c1 = |i: usize| at(c, i, 1);
    let tr = match id {
        "acbff_c40" => { let a = mean(c, u("n")); day(&|i| (c[i] > a[i], c[i] < a[i])) }
        "acbff_forever_long" => {
            let pp = u("p"); let tp = g("target"); let lw = low(c, pp);
            engine(x, from, to, |i, ps, nx, _| {
                if i >= 1 && c[i] < lw[i - 1] { nx.push(O { a: Buy, t: T::Lim(l[i]), tag: "buy", tp }); }
                if ps.mp == 1 { put(nx, Sell, T::Lim(ps.ep + tp), "profit target"); }
            })
        }
        "acbff_2v5" => { let (a2, a5) = (mean(c, 2), mean(c, 5)); day(&|i| (a2[i] < a5[i], a2[i] > a5[i])) }
        "acbff_fade2c" => day(&|i| (i >= 2 && c[i] < c[i - 1] && c[i - 1] < c[i - 2], i >= 2 && c[i] > c[i - 1] && c[i - 1] > c[i - 2])),
        "acbff_ext50" => { let m = u("m"); let (hb, lb) = (xbar(c, m, true), xbar(c, m, false)); day(&|i| (hb[i] > lb[i], hb[i] < lb[i])) }
        "acbff_three" => { let f = five(x, u("e"), u("n"), u("p"), u("q"), 10, 15, false); day(&|i| { let s = f.q[i] + f.u[i] + f.xo[i]; (s > 0.0, s < 0.0) }) }
        "acbff_range10" => {
            let ar = mean(&r, u("n")); let half = g("half").round() as i32;
            day(&|i| {
                let (nb, ns) = (r[i] < ar[i] && c[i] > c1(i), r[i] < ar[i] && c[i] < c1(i));
                let (wb, ws) = (r[i] > ar[i] && c[i] < c1(i), r[i] > ar[i] && c[i] > c1(i));
                match half { 1 => (nb, ns), 2 => (wb, ws), _ => (nb || wb, ns || ws) }
            })
        }
        "acbff_hl15" => { let k = u("n"); let (ah, al) = (mean(h, k), mean(l, k)); day(&|i| { let m = (ah[i] + al[i]) / 2.0; (c[i] > m, c[i] < m) }) }
        "acbff_five" | "acbff_five_4of5" | "acbff_c3_124" | "acbff_c3_125" | "acbff_c3_134" | "acbff_c3_135" | "acbff_c3_145"
        | "acbff_c3_234" | "acbff_c3_235" | "acbff_c3_245" | "acbff_c3_345" => {
            let f = five(x, 40, 2, 5, 50, 10, 15, false);
            let comb: &str = id.strip_prefix("acbff_c3_").unwrap_or("12345");
            let thr = if id == "acbff_five_4of5" { 3.0 } else { 0.5 };
            let vals = [&f.q, &f.u, &f.xo, &f.y, &f.z];
            let idx: Vec<usize> = comb.bytes().map(|b| (b - b'1') as usize).collect();
            day(&|i| { let s: f64 = idx.iter().map(|&k| vals[k][i]).sum(); (s >= thr, s <= -thr) })
        }
        "acbff_fade_oc" => day(&|i| (c[i] < op[i], c[i] > op[i])),
        "acbff_fade2oc" => { let f = fade2oc(x); day(&|i| (f[i] > 0.0, f[i] < 0.0)) }
        "acbff_cup_day" => { let k = cupcap(x); day(&|i| (k[i] > 0.0, k[i] < 0.0)) }
        "acbff_cup_overnight" => {
            let k = cupcap(x);
            engine(x, from, to, |i, _, nx, tc| {
                if k[i] > 0.0 { put(tc, Buy, T::Close, "buy on close"); }
                put(nx, Sell, T::Mkt, "next open");
                if k[i] < 0.0 { put(tc, Short, T::Close, "sell short on close"); }
                put(nx, Cover, T::Mkt, "next open");
            })
        }
        "acbff_cup_3day" => {
            let k = cupcap(x); let (h3, l3) = (hiw(h, 3), low(l, 3));
            engine(x, from, to, |i, ps, nx, tc| {
                if k[i] > 0.0 { put(tc, Buy, T::Close, "buy on close"); }
                if ps.mp == 1 && ps.bse(i) == 1 { put(nx, Sell, T::Mkt, "day 3 open"); }
                put(nx, Sell, T::Lim(h3[i] + 0.25 * ar3[i]), "target");
                put(nx, Sell, T::Stop(l3[i] - tick), "stop");
                if k[i] < 0.0 { put(tc, Short, T::Close, "sell short on close"); }
                if ps.mp == -1 && ps.bse(i) == 1 { put(nx, Cover, T::Mkt, "day 3 open"); }
                put(nx, Cover, T::Lim(l3[i] - 0.25 * ar3[i]), "target");
                put(nx, Cover, T::Stop(h3[i] + tick), "stop");
            })
        }
        "acbff_sr20" => { let (b, s) = sr(x, g("n")); day(&|i| (b[i], s[i])) }
        "acbff_eight" | "acbff_eight_3" | "acbff_eight_stop33" | "acbff_eight_stop33_3" | "acbff_eight_lim25" | "acbff_eight_lim25_3" | "acbff_eight_stop66" => {
            let f = five(x, 40, 2, 5, 50, 10, 15, lr);
            let (ff, gg, hh) = (fade2oc(x), cupcap(x), sr_val(x));
            let sum: Vec<f64> = (0..n).map(|i| f.u[i] + f.q[i] + f.xo[i] + f.y[i] + f.z[i] + ff[i] + gg[i] + hh[i]).collect();
            let thr = if id.ends_with("_3") { 3.0 } else { 0.5 };
            let k = g("k");
            let mode = if id.contains("stop33") { 1 } else if id.contains("lim25") { 2 } else if id.contains("stop66") { 3 } else { 0 };
            engine(x, from, to, |i, _, nx, _| {
                let (b, s) = (sum[i] >= thr, sum[i] <= -thr);
                let o1 = op[i + 1];
                match mode {
                    1 => { if b { put(nx, Buy, T::Stop(o1 + k * ar3[i]), "buy stop"); } if s { put(nx, Short, T::Stop(o1 - k * ar3[i]), "short stop"); } }
                    2 => { if b { put(nx, Buy, T::Lim(o1 - k * ar3[i]), "buy limit"); } if s { put(nx, Short, T::Lim(o1 + k * ar3[i]), "short limit"); } }
                    3 => {
                        if b { put(nx, Buy, T::Mkt, "buy"); }
                        put(nx, Sell, T::Stop(o1 - k * ar3[i]), "66% stop");
                        if s { put(nx, Short, T::Mkt, "sell short"); }
                        put(nx, Cover, T::Stop(o1 + k * ar3[i]), "66% stop");
                    }
                    _ => { if b { put(nx, Buy, T::Mkt, "buy"); } if s { put(nx, Short, T::Mkt, "sell short"); } }
                }
                day_exit(nx);
            })
        }
        "acbff_2hl" | "acbff_2hl_tick" | "acbff_2hl_2" | "acbff_2hl_3" | "acbff_2hl_4" => {
            let pen = if id == "acbff_2hl_tick" { tick } else { 0.0 };
            let k = g("k"); let ma = mean(c, u("ma"));
            let f = five(x, 40, 2, 5, 50, 10, 15, false);
            let (mut hh, mut ll) = (0.0f64, 0.0f64);
            engine(x, from, to, move |i, ps, nx, _| {
                if ps.mp == 1 && ps.bse(i) == 0 { hh = h[i]; }
                if ps.mp == -1 && ps.bse(i) == 0 { ll = l[i]; }
                let o1 = op[i + 1];
                let s5 = f.q[i] + f.u[i] + f.xo[i] + f.y[i] + f.z[i];
                let (lb, sb) = match id {
                    "acbff_2hl" | "acbff_2hl_tick" => (s5 > 0.0 && c[i] > ma[i], s5 < 0.0 && c[i] < ma[i]),
                    "acbff_2hl_2" => (c[i] > ma[i], c[i] < ma[i]),
                    _ => (c[i] > at(h, i, 1) && c[i] > ma[i], c[i] < at(l, i, 1) && c[i] < ma[i]),
                };
                let stop_entry = id == "acbff_2hl_2" || id == "acbff_2hl_3";
                if lb { if stop_entry { put(nx, Buy, T::Stop(o1 + k * ar3[i]), "buy stop"); } else { put(nx, Buy, T::Lim(o1 - k * ar3[i] - pen), "buy limit"); } }
                let bs = ps.bse(i);
                if ps.mp == 1 && (2..=14).contains(&bs) {
                    let hk = hiw(&h[i + 1 - bs..=i], bs)[bs - 1];
                    if hk > hh && nth2(h, i, bs, true) > hh { put(nx, Sell, T::Lim(hk + pen), "second high target"); }
                }
                if c[i] < ma[i] { put(nx, Sell, T::Mkt, "close below average"); }
                if sb { if stop_entry { put(nx, Short, T::Stop(o1 - k * ar3[i]), "short stop"); } else { put(nx, Short, T::Lim(o1 + k * ar3[i] + pen), "short limit"); } }
                if ps.mp == -1 && (2..=14).contains(&bs) {
                    let lk = low(&l[i + 1 - bs..=i], bs)[bs - 1];
                    if lk < ll && nth2(l, i, bs, false) < ll { put(nx, Cover, T::Lim(lk - pen), "second low target"); }
                }
                if c[i] > ma[i] { put(nx, Cover, T::Mkt, "close above average"); }
            })
        }
        "acbff_wk_rx" => {
            let kc = cal(x); let q = g("q");
            // first open of each week and the high-low range of the previous complete week
            let mut wopen = vec![NAN; n]; let mut prevr = vec![NAN; n];
            let (mut s, mut last_r) = (0usize, NAN);
            while s < n {
                let mut e = s; while e + 1 < n && kc.week[e + 1] == kc.week[s] { e += 1; }
                let wr = h[s..=e].iter().copied().fold(f64::MIN, f64::max) - l[s..=e].iter().copied().fold(f64::MAX, f64::min);
                for j in s..=e { wopen[j] = op[s]; prevr[j] = last_r; }
                last_r = wr; s = e + 1;
            }
            engine(x, from, to, |i, _, nx, _| {
                let j = i + 1;
                put(nx, Buy, T::Stop(wopen[j] + q * prevr[j]), "weekly range buy stop");
                put(nx, Short, T::Stop(wopen[j] - q * prevr[j]), "weekly range sell stop");
            })
        }
        "acbff_rx15" => {
            let q = g("q"); let ma = mean(c, u("n"));
            engine(x, from, to, |i, _, nx, _| {
                let o1 = op[i + 1]; let d = q * r[i];
                if c[i] > ma[i] { put(nx, Buy, T::Stop(o1 + d), "buy stop"); }
                put(nx, Sell, T::Stop(o1 - d), "exit stop");
                if c[i] < ma[i] { put(nx, Short, T::Stop(o1 - d), "short stop"); }
                put(nx, Cover, T::Stop(o1 + d), "exit stop");
            })
        }
        "acbff_monday" => { let k = cal(x); day(&|i| (k.dow[i] == 5 && c[i] > c1(i), k.dow[i] == 5 && c[i] < c1(i))) }
        "acbff_tuesday" => { let k = cal(x); day(&|i| (k.dow[i] == 1 && c[i] < c1(i), k.dow[i] == 1 && c[i] > c1(i))) }
        "acbff_wed_fri" | "acbff_dow" => {
            let k = cal(x); let (b, s) = dow_sig(x, &k, false);
            let all = id == "acbff_dow";
            day(&|i| { let d = k.dow[i]; let ok = all || (2..=4).contains(&d); (ok && b[i], ok && s[i]) })
        }
        "acbff_short_dom" | "acbff_dom" => {
            let k = cal(x); let (nn, pp) = (g("n").round() as u32, g("p").round() as u32);
            let short_only = id == "acbff_short_dom";
            engine(x, from, to, |i, _, nx, _| {
                let d = k.dom[i];
                let lw = d >= nn && d <= nn + 3; let sw = d >= pp && d <= pp + 3;
                if short_only { if sw { put(nx, Short, T::Mkt, "sell short"); } if lw { put(nx, Cover, T::Mkt, "cover"); } }
                else { if lw { put(nx, Buy, T::Mkt, "buy"); } if sw { put(nx, Short, T::Mkt, "sell short"); } }
            })
        }
        "acbff_dom_day" | "acbff_dom_day_cc" => {
            let k = cal(x); let (nn, pp) = (g("n").round() as u32, g("p").round() as u32);
            let cc = id == "acbff_dom_day_cc"; let all = g("close_all") >= 0.5;
            day(&|i| {
                let d = k.dom[i];
                if !cc { return (d >= nn || d < pp, d >= pp && d < nn); }
                let dn = c[i] < c1(i);
                let b = if all { dn && (d >= nn || d < pp) } else { (dn && d >= nn) || d < pp };
                (b, c[i] > c1(i) && d >= pp && d < nn)
            })
        }
        "acbff_moy" | "acbff_moy_dom" | "acbff_moy_dom_day" | "acbff_cal3" | "acbff_cal2_day" => {
            let k = cal(x); let e = moy(&k, g("n").round() as u32, g("p").round() as u32);
            let (b, s) = dow_sig(x, &k, false);
            let gsig = persist(n, |i| (b[i], s[i]));
            match id {
                "acbff_moy" => perp(&|i| (e[i] == 1.0, e[i] == -1.0)),
                "acbff_moy_dom" | "acbff_cal3" => {
                    let three = id == "acbff_cal3";
                    engine(x, from, to, |i, _, nx, _| {
                        let sm = e[i] + domf(&k, i) + if three { gsig[i] } else { 0.0 };
                        let full = if three { 3.0 } else { 2.0 };
                        if sm == full { put(nx, Buy, T::Mkt, "buy"); }
                        if sm < full { put(nx, Sell, T::Mkt, "condition changed"); }
                        if sm == -full { put(nx, Short, T::Mkt, "sell short"); }
                        if sm > -full { put(nx, Cover, T::Mkt, "condition changed"); }
                    })
                }
                "acbff_moy_dom_day" => engine(x, from, to, |i, _, nx, _| {
                    let sm = e[i] + domf(&k, i);
                    if op[i + 1] < c[i] && sm == 2.0 { put(nx, Buy, T::Mkt, "buy"); }
                    if op[i + 1] > c[i] && sm == -2.0 { put(nx, Short, T::Mkt, "sell short"); }
                    day_exit(nx);
                }),
                _ => engine(x, from, to, |i, _, nx, _| {
                    let sm = e[i] + domf(&k, i) + gsig[i];
                    if op[i + 1] < c[i] && sm > 0.0 { put(nx, Buy, T::Mkt, "buy"); }
                    if op[i + 1] > c[i] && sm < 0.0 { put(nx, Short, T::Mkt, "sell short"); }
                    day_exit(nx);
                }),
            }
        }
        "acbff_cal_eight" => {
            let k = cal(x); let (b, s) = dow_sig(x, &k, true);
            let aaa = persist(n, |i| (b[i], s[i]));
            let f = five(x, 40, 2, 5, 50, 10, 15, lr);
            let ccc = persist(n, |i| { let t = f.u[i] + f.q[i] + f.xo[i] + f.y[i] + f.z[i]; (t > 0.0, t < 0.0) });
            perp(&|i| { let t = aaa[i] + domf(&k, i) + ccc[i]; (t > 0.0, t < 0.0) })
        }
        "acbff_rx_filter" => {
            let xs: Vec<f64> = (0..n).map(|i| if i >= 2 && ((c[i] > c[i - 1] && c[i - 1] > c[i - 2]) || (c[i] < c[i - 1] && c[i - 1] < c[i - 2])) { 1.0 } else { 0.0 }).collect();
            let ax = mean(&xs, u("n")); let (uu, k) = (g("u"), g("k"));
            engine(x, from, to, |i, _, nx, _| {
                let o1 = op[i + 1]; let d = k * ar3[i];
                if ax[i] >= uu { put(nx, Buy, T::Stop(o1 + d), "buy stop"); }
                put(nx, Sell, T::Stop(o1 - d), "exit stop");
                if ax[i] >= uu { put(nx, Short, T::Stop(o1 - d), "short stop"); }
                put(nx, Cover, T::Stop(o1 + d), "exit stop");
            })
        }
        // intraday wave 2b: code-vs-book fix (Table 36.1 printed code: BUY = up close after five down closes); swap = 1 keeps the old sides
        "acbff_5c_day" => { let sw = g("swap") >= 0.5; day(&|i| (run_rev(c, i, 0, 5, !sw), run_rev(c, i, 0, 5, sw))) }
        "acbff_5c_perp" => perp(&|i| (run_rev(c, i, 0, 5, true), run_rev(c, i, 0, 5, false))),
        "acbff_5c1_perp" => perp(&|i| (run_rev(c, i, 1, 5, true), run_rev(c, i, 1, 5, false))),
        "acbff_5c1_day" => day(&|i| (run_rev(c, i, 1, 5, true), run_rev(c, i, 1, 5, false))),
        "acbff_3rd_fade" => engine(x, from, to, |i, ps, nx, _| {
            let o1 = op[i + 1];
            if ps.mp < 1 && i >= 2 && c[i] < l[i - 1] && c[i - 1] < l[i - 2] { put(nx, Buy, T::Lim(o1 - 0.33 * ar3[i]), "buy limit"); }
            if ps.mp == 1 { put(nx, Sell, T::Lim(h[i]), "target"); put(nx, Sell, T::Stop(l[i] - tick), "stop"); put(nx, Sell, T::Close, "exit on close"); }
            if ps.mp > -1 && i >= 2 && c[i] > h[i - 1] && c[i - 1] > h[i - 2] { put(nx, Short, T::Lim(o1 + 0.33 * ar3[i]), "short limit"); }
            if ps.mp == -1 { put(nx, Cover, T::Lim(l[i]), "target"); put(nx, Cover, T::Stop(h[i] + tick), "stop"); put(nx, Cover, T::Close, "exit on close"); }
        }),
        "acbff_5d_avg_fade" => {
            let (al, ah) = (mean(l, 5), mean(h, 5));
            let xs: Vec<f64> = (0..n).map(|i| if c[i] < al[i] { 1.0 } else { 0.0 }).collect();
            let ys: Vec<f64> = (0..n).map(|i| if c[i] > ah[i] { 1.0 } else { 0.0 }).collect();
            let (lx, ly) = (low(&xs, 3), low(&ys, 3));
            let late = if g("exit_late") >= 0.5 { 2 } else { 1 };
            engine(x, from, to, |i, ps, nx, _| {
                let bs = ps.bse(i);
                if ps.mp < 1 && lx[i] == 1.0 && i >= 2 && c[i] < c[i - 1] && c[i - 1] < c[i - 2] { put(nx, Buy, T::Mkt, "buy"); }
                if ps.mp == 1 && bs > 0 { put(nx, Sell, T::Lim(h[i]), "target"); put(nx, Sell, T::Stop(l[i] - tick), "stop"); }
                if ps.mp == 1 && bs >= late { put(nx, Sell, T::Close, "exit on close"); }
                if ps.mp > -1 && ly[i] == 1.0 && i >= 2 && c[i] > c[i - 1] && c[i - 1] > c[i - 2] { put(nx, Short, T::Mkt, "sell short"); }
                if ps.mp == -1 && bs > 0 { put(nx, Cover, T::Lim(l[i]), "target"); put(nx, Cover, T::Stop(h[i] + tick), "stop"); }
                if ps.mp == -1 && bs >= late { put(nx, Cover, T::Close, "exit on close"); }
            })
        }
        "acbff_nday_10stop" => {
            let (nb, m) = (u("n"), u("m"));
            let (hh, ll, hc, lc, hm, lm) = (hiw(h, nb), low(l, nb), hiw(c, m), low(c, m), hiw(h, m), low(l, m));
            engine(x, from, to, |i, ps, nx, _| {
                let bs = ps.bse(i);
                if ps.mp < 1 { put(nx, Buy, T::Stop(hh[i]), "20-day high stop"); }
                if ps.mp != 0 && bs >= m && (at(c, i, m) > hc[i] || at(h, i, m) > hm[i]) { put(nx, Sell, T::Mkt, "no new high in 10 days"); }
                if ps.mp > -1 { put(nx, Short, T::Stop(ll[i]), "20-day low stop"); }
                if ps.mp != 0 && bs >= m && (at(c, i, m) < lc[i] || at(l, i, m) < lm[i]) { put(nx, Cover, T::Mkt, "no new low in 10 days"); }
            })
        }
        "acbff_fade_rev" => {
            let code = g("stop_ref") >= 0.5;
            let (mut hh, mut ll, mut lo_e, mut hi_e) = (0.0f64, 0.0f64, 0.0f64, 0.0f64);
            engine(x, from, to, move |i, ps, nx, _| {
                if ps.mp == 1 && ps.bse(i) == 0 { hh = h[i]; lo_e = l[i]; }
                if ps.mp == -1 && ps.bse(i) == 0 { ll = l[i]; hi_e = h[i]; }
                let (h1, l1) = (at(h, i, 1), at(l, i, 1));
                if ps.mp < 1 && h[i] > h1 && c[i] < c1(i) { put(nx, Buy, T::Mkt, "buy"); }
                if h[i] <= h1 || c[i] >= c1(i) { put(nx, Sell, T::Lim(hh), "target: entry-day high"); }
                if ps.mp == 1 && ps.bse(i) > 0 { put(nx, Sell, T::Stop((if code { ll } else { lo_e }) - tick), "stop"); }
                if ps.mp > -1 && l[i] < l1 && c[i] > c1(i) { put(nx, Short, T::Mkt, "sell short"); }
                if l[i] >= l1 || c[i] <= c1(i) { put(nx, Cover, T::Lim(ll), "target: entry-day low"); }
                if ps.mp == -1 && ps.bse(i) > 0 { put(nx, Cover, T::Stop((if code { hh } else { hi_e }) + tick), "stop"); }
            })
        }
        "acbff_rx4day" => {
            let k = g("k");
            engine(x, from, to, |i, ps, nx, _| {
                let o1 = op[i + 1]; let bs2 = ps.mp != 0 && ps.bse(i) == 2;
                put(nx, Buy, T::Stop(o1 + k * ar3[i]), "buy stop");
                if bs2 { put(nx, Sell, T::Lim(h[i]), "target"); put(nx, Sell, T::Stop(l[i]), "stop"); put(nx, Sell, T::Close, "exit on close"); }
                put(nx, Short, T::Stop(o1 - k * ar3[i]), "short stop");
                if bs2 { put(nx, Cover, T::Lim(l[i]), "target"); put(nx, Cover, T::Stop(h[i]), "stop"); put(nx, Cover, T::Close, "exit on close"); }
            })
        }
        "acbff_mom66" => {
            let (nn, k, sm) = (u("n"), g("k"), g("s"));
            let (h9, l9) = (hiw(h, nn), low(l, nn));
            let mut xx = 0.0f64;
            engine(x, from, to, move |i, ps, nx, _| {
                let (hc, lc) = (h9[i] - c[i], c[i] - l9[i]);
                if hc > lc { xx = hc; } if hc < lc { xx = lc; }
                let o1 = op[i + 1];
                if ps.mp < 1 && hc > lc { put(nx, Buy, T::Stop(o1 + k * lc), "buy stop"); }
                if ps.mp == 1 && ps.bse(i) > 0 { put(nx, Sell, T::Stop(ps.ep - sm * xx), "stop"); }
                if ps.mp > -1 && lc > hc { put(nx, Short, T::Stop(o1 - k * hc), "short stop"); }
                if ps.mp == -1 && ps.bse(i) > 0 { put(nx, Cover, T::Stop(ps.ep + sm * xx), "stop"); }
            })
        }
        "acbff_contract" => {
            let (a10, a25, a5, c10, c25) = (mean(&r, 10), mean(&r, 25), mean(c, 5), mean(c, 10), mean(c, 25));
            engine(x, from, to, |i, ps, nx, _| {
                let o1 = op[i + 1]; let d = 0.2 * r[i];
                if a10[i] < a25[i] && c[i] < c1(i) && c[i] < a5[i] { put(nx, Buy, T::Stop(o1 + d), "buy stop"); }
                if c[i] > c10[i] && c[i] > c25[i] { put(nx, Sell, T::Stop(o1 - d), "conditional stop"); }
                if ps.mp == 1 && ps.bse(i) > 0 { put(nx, Sell, T::Stop(ps.ep - a25[i]), "straight stop"); }
                if a10[i] < a25[i] && c[i] > c1(i) && c[i] > a5[i] { put(nx, Short, T::Stop(o1 - d), "short stop"); }
                if c[i] < c10[i] && c[i] < c25[i] { put(nx, Cover, T::Stop(o1 + d), "conditional stop"); }
                if ps.mp == -1 && ps.bse(i) > 0 { put(nx, Cover, T::Stop(ps.ep + a25[i]), "straight stop"); }
            })
        }
        "acbff_5v15" => {
            let (a5, a15) = (mean(c, 5), mean(c, 15));
            let gt = |i: usize, k: usize| i >= k && a5[i - k] > a15[i - k];
            let lt = |i: usize, k: usize| i >= k && a5[i - k] < a15[i - k];
            perp(&|i| (gt(i, 0) && gt(i, 1) && gt(i, 2) && lt(i, 3), lt(i, 0) && lt(i, 1) && lt(i, 2) && gt(i, 3)))
        }
        "acbff_3v30" => {
            let (a3, a30) = (mean(c, 3), mean(c, 30));
            let gt = |i: usize, k: usize| i >= k && a3[i - k] > a30[i - k];
            let lt = |i: usize, k: usize| i >= k && a3[i - k] < a30[i - k];
            engine(x, from, to, |i, _, nx, _| {
                if lt(i, 3) && gt(i, 2) && gt(i, 1) && gt(i, 0) { put(nx, Buy, T::Mkt, "buy"); }
                if lt(i, 0) { put(nx, Sell, T::Mkt, "3-day below 30-day"); }
                if gt(i, 3) && lt(i, 2) && lt(i, 1) && lt(i, 0) { put(nx, Short, T::Mkt, "sell short"); }
                if gt(i, 0) { put(nx, Cover, T::Mkt, "3-day above 30-day"); }
            })
        }
        "acbff_15_30" => {
            let (ka, kb) = (u("a"), u("b"));
            engine(x, from, to, |i, _, nx, _| {
                let (ca, cb) = (at(c, i, ka), at(c, i, kb));
                if c[i] > ca && c[i] > cb { put(nx, Buy, T::Mkt, "buy"); }
                if c[i] < ca || c[i] < cb { put(nx, Sell, T::Mkt, "below a marker"); }
                if c[i] < ca && c[i] < cb { put(nx, Short, T::Mkt, "sell short"); }
                if c[i] > ca || c[i] > cb { put(nx, Cover, T::Mkt, "above a marker"); }
            })
        }
        "acbff_fade23oc" => { let ff = ff23(x, true); day(&|i| (ff[i] > 0.0, ff[i] < 0.0)) }
        "acbff_six" | "acbff_six_stop66" | "acbff_nee5" | "acbff_six_q" | "acbff_six_q66" | "acbff_six_q66_v2" => {
            let q44 = id.starts_with("acbff_six_q");
            let f = five(x, 40, 2, 5, 50, 10, 15, lr);
            let ff = ff23(x, !q44);
            let six: Vec<f64> = (0..n).map(|i| f.u[i] + f.q[i] + f.xo[i] + f.y[i] + f.z[i] + ff[i]).collect();
            let (cc, dd, ee) = (cupcap(x), sr_val(x), fade2oc(x));
            let nee: Vec<f64> = (0..n).map(|i| nee_ab(c, i, 5) + nee_ab(c, i, 6) + cc[i] + dd[i] + ee[i]).collect();
            let stop = id.contains("66") || id == "acbff_six_stop66";
            let k = g("k");
            engine(x, from, to, |i, _, nx, _| {
                let (b, s) = match id {
                    "acbff_nee5" => (nee[i] > 0.0, nee[i] < 0.0),
                    "acbff_six" | "acbff_six_stop66" => (six[i] > 0.0, six[i] < 0.0),
                    "acbff_six_q66_v2" => ((six[i] > 0.0 && nee[i] >= 0.0) || (six[i] == 0.0 && nee[i] > 0.0), (six[i] < 0.0 && nee[i] <= 0.0) || (six[i] == 0.0 && nee[i] < 0.0)),
                    _ => (six[i] > 0.0 && nee[i] >= 0.0, six[i] < 0.0 && nee[i] <= 0.0),
                };
                let o1 = op[i + 1];
                if b { put(nx, Buy, T::Mkt, "buy"); }
                if stop { put(nx, Sell, T::Stop(o1 - k * ar3[i]), "66% stop"); }
                if s { put(nx, Short, T::Mkt, "sell short"); }
                if stop { put(nx, Cover, T::Stop(o1 + k * ar3[i]), "66% stop"); }
                day_exit(nx);
            })
        }
        _ => return None,
    };
    Some(tr)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    fn rows(v: &[(f64, f64, f64, f64)]) -> Vec<Row> {
        v.iter().enumerate().map(|(i, &(o, h, l, c))| Row { ticker: "SPX".into(), date: format!("2020-01-{:02}", i + 1), open: o, high: h, low: l, close: c, volume: None }).collect()
    }
    #[test]
    fn fade_two_closes_day_trade_and_stop_path() {
        let r = rows(&[(10., 11., 9., 10.), (10., 10., 8., 9.), (9., 9., 7., 8.), (8., 12., 7.5, 11.), (11., 11., 11., 11.)]);
        let x = Ctx::new(&r, &r, &[]);
        let t = run("acbff_fade2c", &x, 0, 4, &Params::new()).unwrap();
        assert_eq!(t.len(), 1);
        assert_eq!((t[0].dir, t[0].legs[0].i, t[0].legs[0].px, t[0].exit_i, t[0].exit_px), (1.0, 3, 8.0, 3, 11.0));
        // engine: buy stop above the open and sell stop below on the same bar, open nearer the low -> low first
        let mut e = Eng { x: &x, pos: Pos::default(), cur: None, out: vec![] };
        e.bar(3, vec![o(A::Buy, T::Stop(9.0), "b"), o(A::Sell, T::Stop(7.8), "s")]);
        assert_eq!(e.pos.mp, 1); // low touched first while flat (sell ignored), then the buy stop fills
        assert_eq!(e.cur.as_ref().unwrap().legs[0].px, 9.0);
    }
    #[test]
    fn calendar_helpers() {
        assert_eq!(days(1970, 1, 1), 0);
        let r = rows(&[(1., 1., 1., 1.); 3]); let x = Ctx::new(&r, &r, &[]); let k = cal(&x);
        assert_eq!(k.dow[0], 3); // 2020-01-01 was a Wednesday
        assert_eq!(k.dom[2], 3);
    }
    #[test]
    fn every_live_strategy_runs() {
        let v: Vec<(f64, f64, f64, f64)> = (0..400).map(|i| { let b = 100.0 + (i as f64 * 0.37).sin() * 10.0 + i as f64 * 0.05; (b, b + 1.5, b - 1.4, b + ((i * 7) % 5) as f64 * 0.3 - 0.6) }).collect();
        let mut r = rows(&v);
        for (i, row) in r.iter_mut().enumerate() { let n = days(2018, 1, 1) + i as i64 * 7 / 5; let _ = n; row.date = civil(days(2018, 1, 1) + (i as i64 / 5) * 7 + (i as i64 % 5)); }
        let x = Ctx::new(&r, &r, &[]);
        for d in catalogue().iter().filter(|d| d.status == Status::Live) {
            let t = run(d.id, &x, 60, 399, &Params::new()).unwrap();
            for tr in &t { assert!(tr.exit_i >= tr.legs[0].i, "{}", d.id); }
        }
    }
    fn civil(z: i64) -> String {
        let z = z + 719468; let era = z.div_euclid(146097); let doe = z - era * 146097;
        let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365; let y = yoe + era * 400;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); let mp = (5 * doy + 2) / 153;
        let d = doy - (153 * mp + 2) / 5 + 1; let m = if mp < 10 { mp + 3 } else { mp - 9 };
        format!("{:04}-{:02}-{:02}", if m <= 2 { y + 1 } else { y }, m, d)
    }
}
