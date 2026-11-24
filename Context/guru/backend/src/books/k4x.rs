//! Lane K4 (code pass, 23 Nov 2026) shared helper for TradeStation / MetaTrader intraday systems run on the NIFTY 50 / NIFTYBANK
//! futures 1-minute series (Ctx::fut, lane DI) as N-minute session-aligned bars (R4 #340 declared adaptation). No strategies here.
//! `Fb` = the N-minute bar series across days (EP2: one continuous series, indicators carry overnight, no bar spans two sessions).
//! `run` = the TradeStation bar-model order engine of rrsim (market orders at the next open, a stop/limit the open already satisfies
//! fills at the open, then the bar walks open -> nearer extreme -> other extreme -> close and each level fills where the walk reaches
//! it; an entry in the held direction is ignored, an opposite entry reverses) with one addition the rules need: `St::last_entry`, the
//! bar of the most recent entry even when that trade is already closed (EasyLanguage EntryDate(1)).
use crate::connors::{Ctx, ParamDef, Params, StrategyDef, Trade};

pub fn catalogue() -> Vec<StrategyDef> { vec![] }
pub fn params(_id: &str) -> Option<Vec<ParamDef>> { None }
pub fn run(_id: &str, _x: &Ctx, _from: usize, _to: usize, _p: &Params) -> Option<Vec<Trade>> { None }
pub fn family_of(_id: &str) -> Option<(&'static str, &'static str, &'static str)> { None }

/// Ids of the K4 cards that run on the NIFTY / BANKNIFTY futures minute series (registered in intraday_ep::M1 by `K4`).
pub const K4: &[&str] = &["sdrbk_rbreaker", "ymgap_fade", "ndt1_neural_dt1", "ndt60_neural_dt60", "kho5_sd_scalp", "dturbo_ft", "levva_value_area_80", "dmpub_epfft5", "dmpub_epfft1"];
pub fn k4(id: &str) -> bool { K4.contains(&id) }

/// One N-minute bar series. `day[k]` = the daily-row index (Ctx row) of bar k; `stamp[k]` = TradeStation time stamp (bucket end,
/// minute of day) of bar k; `last[k]` = bar k is the last bar of its session.
pub struct Fb { pub day: Vec<usize>, pub stamp: Vec<u16>, pub last: Vec<bool>, pub o: Vec<f64>, pub h: Vec<f64>, pub l: Vec<f64>, pub c: Vec<f64>, pub v: Vec<f64>, pub date: Vec<String> }

/// hhmm of a minute of day (TradeStation `Time` stamp: 12:29 -> 1229).
pub fn hhmm(m: u16) -> i64 { (m / 60 * 100 + m % 60) as i64 }

/// N-minute bars of the futures minutes of daily rows `from..=to` (days without futures minutes are skipped). None: no futures file.
pub fn fut_bars(x: &Ctx, from: usize, to: usize, n: u16) -> Option<Fb> {
    let f = x.fut?;
    let mut b = Fb { day: vec![], stamp: vec![], last: vec![], o: vec![], h: vec![], l: vec![], c: vec![], v: vec![], date: vec![] };
    for i in from..=to.min(x.r.len().saturating_sub(1)) {
        let r = &x.r[i];
        let Some((m, mv)) = f.fut_day(&r.date) else { continue };
        let (open, close) = (crate::intraday::session_open(&r.ticker, &r.date), crate::intraday_ep::session_close(&r.ticker));
        let bars = crate::intraday_ep::nbars(&m, open, n, close);
        let nb = bars.len();
        for (k, z) in bars.iter().enumerate() {
            b.day.push(i); b.stamp.push(z.e); b.last.push(k + 1 == nb); b.o.push(z.o); b.h.push(z.h); b.l.push(z.l); b.c.push(z.c);
            b.v.push(mv[z.a..z.b].iter().sum()); b.date.push(r.date.clone());
        }
    }
    (!b.o.is_empty()).then_some(b)
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum A { Buy, Sell, Xl, Xs }
#[derive(Clone, Copy, Debug)]
pub enum P { Mkt, Stop(f64), Lim(f64) }
#[derive(Clone, Copy, Debug)]
pub struct O { pub a: A, pub p: P, pub tag: &'static str }
pub fn o(a: A, p: P, tag: &'static str) -> O { O { a, p, tag } }

/// Position state seen by the code at a bar's close: `pos` (MarketPosition), `ep` (EntryPrice), `ei` (bar of the open trade's entry),
/// `last_entry` (bar of the most recent entry, closed or not).
#[derive(Clone, Copy, Default, Debug)]
pub struct St { pub pos: i8, pub ep: f64, pub ei: usize, pub last_entry: Option<usize> }

struct Cur { d: i8, ei: usize, ep: f64, kind: &'static str }

/// Run `f(bar, state, close_orders, next_bar_orders)` on every bar; returns the trades (daily-row indices, prices in index points).
/// Orders `close_orders` execute at that bar's close, `next_bar_orders` live for the next bar only. The position open on the last
/// bar is marked to its close (open = true).
pub fn run_fb(fb: &Fb, mut f: impl FnMut(usize, &St, &mut Vec<O>, &mut Vec<O>)) -> Vec<Trade> {
    let n = fb.o.len();
    let (mut out, mut cur, mut last_entry): (Vec<Trade>, Option<Cur>, Option<usize>) = (vec![], None, None);
    let mut pending: Vec<O> = vec![];
    let close = |out: &mut Vec<Trade>, cur: &mut Option<Cur>, k: usize, px: f64, why: &str| {
        if let Some(c) = cur.take() { out.push(crate::intraday::day_trade(c.d as f64, fb.day[c.ei], c.ep, fb.day[k], px, why, c.kind, false)); }
    };
    for k in 0..n {
        if !pending.is_empty() {
            let (op, hi, lo, cl) = (fb.o[k], fb.h[k], fb.l[k], fb.c[k]);
            let mut done = vec![false; pending.len()];
            let pos = |cur: &Option<Cur>| cur.as_ref().map_or(0i8, |c| c.d);
            let valid = |od: &O, cur: &Option<Cur>| match od.a { A::Buy => pos(cur) != 1, A::Sell => pos(cur) != -1, A::Xl => pos(cur) == 1, A::Xs => pos(cur) == -1 };
            let mut exec = |od: &O, px: f64, kind: &'static str, out: &mut Vec<Trade>, cur: &mut Option<Cur>, last_entry: &mut Option<usize>| match od.a {
                A::Buy | A::Sell => {
                    let d: i8 = if od.a == A::Buy { 1 } else { -1 };
                    if pos(cur) == -d { close(out, cur, k, px, &format!("reversed by {}", od.tag)); }
                    *cur = Some(Cur { d, ei: k, ep: px, kind }); *last_entry = Some(k);
                }
                A::Xl | A::Xs => close(out, cur, k, px, od.tag),
            };
            for pass in 0..2 {
                for (j, od) in pending.iter().enumerate() {
                    let is_exit = matches!(od.a, A::Xl | A::Xs);
                    if matches!(od.p, P::Mkt) && (pass == 0) == is_exit && !done[j] && valid(od, &cur) { done[j] = true; exec(od, op, "open", &mut out, &mut cur, &mut last_entry); }
                }
            }
            let rising = |a: A| matches!(a, A::Buy | A::Xs);
            loop {
                let hit = pending.iter().enumerate().find(|(j, od)| !done[*j] && valid(od, &cur) && match od.p {
                    P::Stop(s) => if rising(od.a) { op >= s } else { op <= s },
                    P::Lim(s) => if rising(od.a) { op <= s } else { op >= s },
                    P::Mkt => false,
                }).map(|(j, _)| j);
                match hit { Some(j) => { done[j] = true; let od = pending[j]; exec(&od, op, "open", &mut out, &mut cur, &mut last_entry); } None => break }
            }
            let path = if hi - op < op - lo { [hi, lo, cl] } else { [lo, hi, cl] };
            let (mut at, mut seg) = (op, 0);
            while seg < 3 {
                let t = path[seg];
                let up = t > at;
                let mut best: Option<(usize, f64)> = None;
                for (j, od) in pending.iter().enumerate() {
                    if done[j] || !valid(od, &cur) { continue; }
                    let lv = match od.p { P::Stop(s) if rising(od.a) == up => s, P::Lim(s) if rising(od.a) != up => s, _ => continue };
                    let inside = if up { lv > at && lv <= t } else { lv < at && lv >= t };
                    if inside && best.map_or(true, |(_, b)| (lv - at).abs() < (b - at).abs()) { best = Some((j, lv)); }
                }
                match best {
                    Some((j, lv)) => { done[j] = true; let od = pending[j]; exec(&od, lv, if matches!(od.p, P::Stop(_)) { "stop" } else { "limit" }, &mut out, &mut cur, &mut last_entry); at = lv; }
                    None => { at = t; seg += 1; }
                }
            }
        }
        pending.clear();
        let st = St { pos: cur.as_ref().map_or(0, |c| c.d), ep: cur.as_ref().map_or(0.0, |c| c.ep), ei: cur.as_ref().map_or(0, |c| c.ei), last_entry };
        let (mut now, mut next) = (vec![], vec![]);
        f(k, &st, &mut now, &mut next);
        for od in now.iter() {
            let ok = match od.a { A::Buy => st_pos(&cur) != 1, A::Sell => st_pos(&cur) != -1, A::Xl => st_pos(&cur) == 1, A::Xs => st_pos(&cur) == -1 };
            if !ok { continue; }
            match od.a {
                A::Buy | A::Sell => {
                    let d: i8 = if od.a == A::Buy { 1 } else { -1 };
                    if st_pos(&cur) == -d { close(&mut out, &mut cur, k, fb.c[k], &format!("reversed by {}", od.tag)); }
                    cur = Some(Cur { d, ei: k, ep: fb.c[k], kind: "close" }); last_entry = Some(k);
                }
                A::Xl | A::Xs => close(&mut out, &mut cur, k, fb.c[k], od.tag),
            }
        }
        if k + 1 < n { pending = next; }
    }
    if let Some(c) = cur.take() { out.push(crate::intraday::day_trade(c.d as f64, fb.day[c.ei], c.ep, fb.day[n - 1], fb.c[n - 1], "open", c.kind, true)); }
    out
}
fn st_pos(cur: &Option<Cur>) -> i8 { cur.as_ref().map_or(0, |c| c.d) }

#[cfg(test)]
mod tests {
    use super::*;
    fn fb(bars: &[(f64, f64, f64, f64)]) -> Fb {
        let n = bars.len();
        Fb { day: (0..n).map(|k| k / 2).collect(), stamp: (0..n).map(|k| 600 + 10 * (k as u16 % 2)).collect(), last: (0..n).map(|k| k % 2 == 1).collect(),
            o: bars.iter().map(|b| b.0).collect(), h: bars.iter().map(|b| b.1).collect(), l: bars.iter().map(|b| b.2).collect(), c: bars.iter().map(|b| b.3).collect(),
            v: vec![1.0; n], date: (0..n).map(|k| format!("2024-01-{:02}", 1 + k / 2)).collect() }
    }
    #[test]
    fn stop_target_walk_and_last_entry() {
        // bar0 buys at market next open; bar1 (position long, entry 10) places a sell stop 8 and a limit 13 for bar2 which opens nearer the low
        let b = fb(&[(10., 11., 9., 10.), (10., 11., 9., 10.), (10., 14., 7., 13.5), (10., 11., 9., 10.)]);
        let mut seen = vec![];
        let t = run_fb(&b, |k, st, _now, next| {
            seen.push((k, st.pos, st.last_entry));
            if k == 0 { next.push(o(A::Buy, P::Mkt, "LE")); }
            if k == 1 { next.push(o(A::Xl, P::Stop(8.), "SX")); next.push(o(A::Xl, P::Lim(13.), "LX")); }
        });
        assert_eq!(t.len(), 1);
        assert_eq!((t[0].legs[0].px, t[0].exit_px, t[0].reason.as_str()), (10., 8., "SX")); // open 10 is nearer the low: O-L-H-C, the stop 8 is reached first
        assert_eq!(seen[2], (2, 0, Some(1))); // closed trade: flat, but the last entry (bar 1) is remembered
    }
    /// Hand-run dump of every K4 card on the real files for the pandas references
    /// (K4_CSV=<idx_data.csv> K4_FUT_DIR=<data/intraday> K4_OUT=<file> cargo test --release k4_dump -- --ignored).
    #[test]
    #[ignore]
    fn k4_dump() {
        use std::io::Write;
        let (csv, dir, out) = (std::env::var("K4_CSV").unwrap(), std::env::var("K4_FUT_DIR").unwrap(), std::env::var("K4_OUT").unwrap());
        let data = crate::load_csv(&csv).unwrap();
        let mut f = std::io::BufWriter::new(std::fs::File::create(out).unwrap());
        writeln!(f, "id,ticker,variant,dir,entry_date,entry_px,exit_date,exit_px,reason").unwrap();
        for t in ["NIFTY 50", "NIFTYBANK"] {
            let rows = &data[t];
            let mut x = crate::connors::Ctx::new(rows, &[], &[]);
            x.fut = crate::intraday::Minutes::scan_fut(&format!("{dir}/{}.FUT1.m1.csv", t.replace(' ', "_"))).map(|m| &*Box::leak(Box::new(m)));
            for id in K4 {
                let tr = crate::books::run(id, &x, 0, rows.len() - 1, &Params::new()).unwrap();
                for z in tr { writeln!(f, "{id},{t},,{},{},{},{},{},{}", z.dir, rows[z.legs[0].i].date, z.legs[0].px, rows[z.exit_i].date, z.exit_px, z.reason).unwrap(); }
            }
        }
    }
}
