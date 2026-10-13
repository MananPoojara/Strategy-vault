//! Shared TradeStation-style daily-bar order engine and TS functions for the RAR vendor-code modules of
//! rar-strategy-code.tsv rows 152-226 (dtwxp, tchnl, stfun, jhsto, lwinb, bfmmx, tcsyn, zstrt, tttt1).
//! No strategies of its own. Notes: library/rrb3/README.md.
//!
//! Engine conventions (the stad engine's TradeStation rules, without STAD's book-specific 10% bounce):
//! * the code runs at each bar's close; "this bar on close" orders fill at that close; "next bar" orders live for
//!   the next bar only; MarketPosition / EntryPrice read by the code are the values before this bar's close orders;
//! * next bar: market orders fill at the open (exits first, then entries); a stop or limit the open already
//!   satisfies fills at the open; then the bar runs open -> nearer extreme -> other extreme -> close (open nearer
//!   the high -> O-H-L-C, otherwise O-L-H-C) and each stop / limit fills at its level when the path reaches it;
//! * Buy / SellShort reverse an opposite position; an entry in the direction already held is ignored (no
//!   pyramiding, the TradeStation default); an exit acts on a position of its side that exists when its price is
//!   reached (also one entered earlier on the same bar); `from` names the entry an exit is tied to ("FROM ENTRY").
use crate::connors::{Ctx, Leg, ParamDef, Params, StrategyDef, Trade};

pub fn catalogue() -> Vec<StrategyDef> { vec![] }
pub fn params(_id: &str) -> Option<Vec<ParamDef>> { None }
pub fn run(_id: &str, _x: &Ctx, _from: usize, _to: usize, _p: &Params) -> Option<Vec<Trade>> { None }
pub fn family_of(_id: &str) -> Option<(&'static str, &'static str, &'static str)> { None }

pub const NAN: f64 = f64::NAN;
/// EasyLanguage "1 point" = 1/PriceScale; index quotes have 2 decimals (PriceScale 100), as in the other TS modules.
pub const POINT: f64 = 0.01;

// ------------------------------------------------------------------ TradeStation functions
pub fn sma(v: &[f64], n: usize) -> Vec<f64> {
    let mut out = vec![NAN; v.len()];
    if n == 0 { return out; }
    for i in n.saturating_sub(1)..v.len() { out[i] = v[i + 1 - n..=i].iter().sum::<f64>() / n as f64; }
    out
}
/// XAverage: alpha 2/(n+1), seeded with the first value.
pub fn xavg(v: &[f64], n: usize) -> Vec<f64> {
    let a = 2.0 / (n as f64 + 1.0);
    let mut out = vec![NAN; v.len()];
    for i in 0..v.len() { out[i] = if i == 0 { v[0] } else { out[i - 1] + a * (v[i] - out[i - 1]) }; }
    out
}
pub fn highest(v: &[f64], n: usize, i: usize) -> f64 {
    if i + 1 < n || n == 0 { return NAN; }
    v[i + 1 - n..=i].iter().copied().fold(f64::NEG_INFINITY, f64::max)
}
pub fn lowest(v: &[f64], n: usize, i: usize) -> f64 {
    if i + 1 < n || n == 0 { return NAN; }
    v[i + 1 - n..=i].iter().copied().fold(f64::INFINITY, f64::min)
}
/// TrueRange (first bar: high - low), TrueHigh = max(H, C[1]), TrueLow = min(L, C[1]).
pub fn true_range(x: &Ctx) -> Vec<f64> {
    (0..x.c.len()).map(|i| if i == 0 { x.h[0] - x.l[0] } else { x.h[i].max(x.c[i - 1]) - x.l[i].min(x.c[i - 1]) }).collect()
}
pub fn true_high(x: &Ctx) -> Vec<f64> { (0..x.c.len()).map(|i| if i == 0 { x.h[0] } else { x.h[i].max(x.c[i - 1]) }).collect() }
pub fn true_low(x: &Ctx) -> Vec<f64> { (0..x.c.len()).map(|i| if i == 0 { x.l[0] } else { x.l[i].min(x.c[i - 1]) }).collect() }

// ------------------------------------------------------------------ order engine
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum A { Buy, Sell, Xl, Xs }
#[allow(dead_code)]
#[derive(Clone, Copy, Debug)]
pub enum P { Mkt, Stop(f64), Lim(f64) }
#[derive(Clone, Copy, Debug)]
pub struct O { pub a: A, pub p: P, pub tag: &'static str, pub from: &'static str }
pub fn o(a: A, p: P, tag: &'static str) -> O { O { a, p, tag, from: "" } }
pub fn of(a: A, p: P, tag: &'static str, from: &'static str) -> O { O { a, p, tag, from } }

/// Position state seen by the code at a bar's close (before that bar's close orders).
#[derive(Clone, Copy, Default, Debug)]
pub struct St { pub pos: i8, pub entry_px: f64, pub entry_i: usize }
impl St {
    /// BarsSinceEntry (0 on the entry bar).
    pub fn bse(&self, i: usize) -> usize { i - self.entry_i }
}

struct Eng<'a> { x: &'a Ctx<'a>, pos: i8, cur: Option<Trade>, out: Vec<Trade>, tag: &'static str }

impl<'a> Eng<'a> {
    fn close_pos(&mut self, i: usize, px: f64, why: &str, sig: usize) {
        if let Some(mut t) = self.cur.take() { t.exit_i = i; t.exit_px = px; t.reason = why.into(); t.signal_i = sig; self.out.push(t); }
        self.pos = 0; self.tag = "";
    }
    fn exec(&mut self, od: &O, i: usize, px: f64, kind: &'static str, sig: usize) {
        match od.a {
            A::Buy | A::Sell => {
                let d: i8 = if od.a == A::Buy { 1 } else { -1 };
                if self.pos == -d { self.close_pos(i, px, &format!("reversed by {}", od.tag), sig); }
                self.pos = d; self.tag = od.tag;
                self.cur = Some(Trade { dir: d as f64, legs: vec![Leg { i, px, w: 1.0 }], exit_i: i, exit_px: px,
                    reason: String::new(), entry_kind: kind, signal_i: sig, open: false, exits: vec![] });
            }
            A::Xl | A::Xs => self.close_pos(i, px, od.tag, sig),
        }
    }
    fn valid(&self, od: &O) -> bool {
        let tag_ok = od.from.is_empty() || od.from == self.tag;
        match od.a { A::Buy => self.pos != 1, A::Sell => self.pos != -1, A::Xl => self.pos == 1 && tag_ok, A::Xs => self.pos == -1 && tag_ok }
    }
    fn fill_bar(&mut self, i: usize, ords: &[O]) {
        if ords.is_empty() { return; }
        let x = self.x;
        let (op, hi, lo, cl) = (x.o[i], x.h[i], x.l[i], x.c[i]);
        let mut done = vec![false; ords.len()];
        for pass in 0..2 {
            for (k, od) in ords.iter().enumerate() {
                let is_exit = matches!(od.a, A::Xl | A::Xs);
                if matches!(od.p, P::Mkt) && (pass == 0) == is_exit && !done[k] && self.valid(od) { done[k] = true; self.exec(od, i, op, "open", i - 1); }
            }
        }
        let rising = |a: A| matches!(a, A::Buy | A::Xs);
        loop {
            let hit = ords.iter().enumerate().find(|(k, od)| !done[*k] && self.valid(od) && match od.p {
                P::Stop(s) => if rising(od.a) { op >= s } else { op <= s },
                P::Lim(s) => if rising(od.a) { op <= s } else { op >= s },
                P::Mkt => false,
            }).map(|(k, _)| k);
            match hit { Some(k) => { done[k] = true; let od = ords[k]; self.exec(&od, i, op, kind(&od), i - 1); } None => break }
        }
        let path = if hi - op < op - lo { [hi, lo, cl] } else { [lo, hi, cl] };
        let mut cur = op;
        let mut seg = 0;
        while seg < 3 {
            let t = path[seg];
            let up = t > cur;
            let mut best: Option<(usize, f64)> = None;
            for (k, od) in ords.iter().enumerate() {
                if done[k] || !self.valid(od) { continue; }
                let lv = match od.p { P::Stop(s) if rising(od.a) == up => s, P::Lim(s) if rising(od.a) != up => s, _ => continue };
                let inside = if up { lv > cur && lv <= t } else { lv < cur && lv >= t };
                if inside && best.map_or(true, |(_, b)| (lv - cur).abs() < (b - cur).abs()) { best = Some((k, lv)); }
            }
            match best {
                Some((k, lv)) => { done[k] = true; let od = ords[k]; self.exec(&od, i, lv, kind(&od), i - 1); cur = lv; }
                None => { cur = t; seg += 1; }
            }
        }
    }
}
fn kind(od: &O) -> &'static str { match od.p { P::Mkt => "open", P::Stop(_) => "stop", P::Lim(_) => "limit" } }

/// Run strategy code `f(i, state, close_orders, next_bar_orders)` on every bar 0..=to (so the code's own variables
/// evolve from the first bar); orders are taken from `from` on. The position open at `to` is marked to its close.
pub fn run_ts(x: &Ctx, from: usize, to: usize, mut f: impl FnMut(usize, &St, &mut Vec<O>, &mut Vec<O>)) -> Vec<Trade> {
    let mut e = Eng { x, pos: 0, cur: None, out: vec![], tag: "" };
    let mut pending: Vec<O> = vec![];
    for i in 0..=to {
        if i > from { e.fill_bar(i, &pending); }
        pending.clear();
        let st = St { pos: e.pos, entry_px: e.cur.as_ref().map_or(0.0, |t| t.legs[0].px), entry_i: e.cur.as_ref().map_or(0, |t| t.legs[0].i) };
        let (mut now, mut next) = (vec![], vec![]);
        f(i, &st, &mut now, &mut next);
        if i >= from {
            for od in now.iter() { if e.valid(od) { e.exec(od, i, x.c[i], "close", i); } }
            if i < to { pending = next; }
        }
    }
    if let Some(mut t) = e.cur.take() { t.exit_i = to; t.exit_px = x.c[to]; t.signal_i = to; t.reason = "open".into(); t.open = true; e.out.push(t); }
    e.out
}

/// Parameter helper: value of `k` or the book default.
pub fn g(p: &Params, k: &str, d: f64) -> f64 { p.get(k).copied().unwrap_or(d) }
/// StrategyDef with the defaults shared by these modules.
pub fn sdef(book: &'static str, id: &'static str, name: &'static str, group: &'static str, dir: &'static str, status: crate::connors::Status,
            rules: &'static [&'static str], notes: &'static str) -> StrategyDef {
    StrategyDef { id, name, group, session: 0, dir, status, needs_volume: false, needs_vix: false, needs_series: &[], book, rules, notes, claim: None }
}
pub fn pd(key: &'static str, label: &'static str, def: f64, min: f64, max: f64, step: f64) -> ParamDef { ParamDef { key, label, def, min, max, step } }

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    fn ctx(rows: &[(f64, f64, f64, f64)]) -> Vec<Row> {
        rows.iter().enumerate().map(|(k, &(o, h, l, c))| Row { ticker: "T".into(), date: format!("2000-01-{:02}", k + 1), open: o, high: h, low: l, close: c, volume: None }).collect()
    }
    #[test]
    fn stop_reverse_and_exit_order_on_one_bar() {
        let r = ctx(&[(10., 11., 9., 10.), (10., 11., 9., 10.), (10., 14., 9.9, 13.5)]);
        let x = Ctx::new(&r, &r, &[]);
        // bar 0: sell short at market next bar; bar 1: buy stop 13 and cover stop 12 for bar 2 (open nearer the low -> O-L-H-C)
        let t = run_ts(&x, 0, 2, |i, st, _now, next| {
            if i == 0 { next.push(o(A::Sell, P::Mkt, "SE")); }
            if i == 1 { assert_eq!(st.pos, -1); next.push(o(A::Buy, P::Stop(13.), "LE")); next.push(o(A::Xs, P::Stop(12.), "SX")); }
        });
        assert_eq!(t.len(), 2);
        assert_eq!((t[0].exit_px, t[0].reason.as_str()), (12., "SX"));
        assert_eq!((t[1].dir, t[1].legs[0].px, t[1].open), (1., 13., true));
    }
}
