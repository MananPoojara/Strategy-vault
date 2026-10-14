//! Single-instrument simulation (spec §4, Customize §5). One position at a time; book-format `connors::Trade`s so
//! trade_validation, equity and stats are reused read-only.
//! Order of events on bar i: (1) at the open a pending exit fills, then a pending market entry, then a resting limit
//! entry (low/high touch, filled at the better of open and limit); (2) at the close, exit conditions (any) fill at
//! this close or set a next-open exit, then entry conditions (all) fill at this close, set a next-open market order
//! or place a limit for the next bar.
use super::bars::Bars;
use super::rules::Fill;
use super::series::Compiled;
use crate::connors::{Leg, Trade};

struct Pos { e: usize, px: f64, kind: &'static str }
impl Pos {
    fn trade(&self, x: usize, px: f64, reason: String, signal_i: usize, d: f64) -> Trade {
        Trade { dir: d, legs: vec![Leg { i: self.e, px: self.px, w: 1. }], exit_i: x, exit_px: px, reason, entry_kind: self.kind, signal_i, open: false, exits: vec![] }
    }
}

pub fn simulate(c: &Compiled, b: &Bars, from: usize, to: usize) -> Vec<Trade> {
    let x = &c.exec;
    let d = if x.short { -1. } else { 1. };
    // v1 rule kept: a next-open market entry may share the open with a next-open exit. Close fills and limit orders
    // follow the books instead: no new order on a bar that held a position at its close or exited after its open.
    let shares_open = x.limit.is_none() && x.fill == Fill::NextOpen;
    let mut trades = Vec::new();
    let mut pos: Option<Pos> = None;
    let mut pending_entry: Option<usize> = None;
    let mut pending_limit: Option<f64> = None;
    let mut pending_exit: Option<(usize, String)> = None;
    for i in from..=to {
        if let Some((sig, why)) = pending_exit.take() {
            if let Some(p) = pos.take() { trades.push(p.trade(i, b.open[i], why, sig, d)); }
        }
        if pending_entry.take().is_some() { pos = Some(Pos { e: i, px: b.open[i], kind: "next_open" }); }
        if let Some(lim) = pending_limit.take() {
            let touched = if d > 0. { b.low[i] <= lim } else { b.high[i] >= lim };
            if touched { pos = Some(Pos { e: i, px: if d > 0. { b.open[i].min(lim) } else { b.open[i].max(lim) }, kind: "limit" }); }
        }
        let mut exited = false; // after the open: blocks a same-bar order under book rules
        let last = i == to;
        if let Some(p) = &pos {
            if let Some(r) = c.exit.iter().find(|r| r.holds(i, Some(p.e))) {
                match x.exit_fill {
                    Fill::Close => { trades.push(pos.take().unwrap().trade(i, b.close[i], r.label.clone(), i, d)); exited = true; }
                    Fill::NextOpen if !last => pending_exit = Some((i, r.label.clone())),
                    Fill::NextOpen => {} // no next open inside the window; the position is marked open below
                }
            }
        }
        if last { break; } // a signal on the last bar has no fill inside the window
        let may_enter = if shares_open { pos.is_none() || pending_exit.is_some() } else { pos.is_none() && !exited };
        if may_enter && c.entry.iter().all(|r| r.holds(i, None)) {
            match (x.limit, x.fill) {
                (Some(l), _) => pending_limit = Some(b.close[i] - d * c.dist(l, b.close[i], i)),
                (None, Fill::Close) => pos = Some(Pos { e: i, px: b.close[i], kind: "close" }),
                (None, Fill::NextOpen) => pending_entry = Some(i),
            }
        }
    }
    if let Some(p) = pos { trades.push(Trade { open: true, ..p.trade(to, b.close[to], "open".into(), to, d) }); }
    trades
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lab::rules::Op;
    use crate::lab::series::{RCond, Val};
    use crate::lab::testdata::from_ohlc;

    fn on(bars: &[usize]) -> Vec<f64> { (0..10).map(|i| if bars.contains(&i) { 1. } else { 0. }).collect() }
    fn rule(signal: Vec<f64>) -> RCond { RCond { lhs: Val::Arr(signal), op: Op::Gt, rhs: Val::Const(0.5), label: "rule".into() } }
    fn held_at_least(n: f64) -> RCond { RCond { lhs: Val::Held, op: Op::Ge, rhs: Val::Const(n), label: format!("bars_held >= {n}") } }
    /// Open = 100 + i, close = 100.5 + i, so fills are identifiable by price.
    fn bars() -> Bars { from_ohlc(&(0..10).map(|i| { let o = 100. + i as f64; (o, o + 1., o - 1., o + 0.5) }).collect::<Vec<_>>()) }

    #[test] fn entry_signal_at_close_fills_next_open() {
        let c = Compiled::new(vec![rule(on(&[2]))], vec![held_at_least(2.)]);
        let t = simulate(&c, &bars(), 0, 9);
        assert_eq!(t.len(), 1);
        assert_eq!((t[0].legs[0].i, t[0].legs[0].px), (3, 103.)); // signal bar 2 → open of bar 3
        // bars_held is 0 on the fill bar: 2 at the close of bar 5 → exit at the open of bar 6
        assert_eq!((t[0].signal_i, t[0].exit_i, t[0].exit_px, t[0].open), (5, 6, 106., false));
        assert_eq!(t[0].reason, "bars_held >= 2");
    }
    #[test] fn exit_fills_before_a_same_bar_reentry() {
        let c = Compiled::new(vec![rule(on(&[1, 4]))], vec![rule(on(&[4]))]);
        let t = simulate(&c, &bars(), 0, 9);
        assert_eq!(t.len(), 2);
        assert_eq!((t[0].legs[0].i, t[0].exit_i), (2, 5));
        assert_eq!((t[1].legs[0].i, t[1].legs[0].px), (5, 105.)); // re-entry at the same open, after the exit
        assert!(crate::trade_validation::check(&t, 0, 9, 0.0005).is_ok());
    }
    #[test] fn entries_need_all_exits_any_and_no_pyramiding() {
        let c = Compiled::new(vec![rule(on(&[1, 2, 3, 6])), rule(on(&[1, 3, 6]))], vec![rule(on(&[7])), rule(on(&[4]))]);
        let t = simulate(&c, &bars(), 0, 9);
        assert_eq!(t.iter().map(|t| (t.legs[0].i, t.exit_i)).collect::<Vec<_>>(), vec![(2, 5), (7, 8)]);
        assert_eq!(t[0].reason, "rule");
    }
    #[test] fn open_position_at_the_end_is_marked_open() {
        let c = Compiled::new(vec![rule(on(&[7]))], vec![rule(on(&[]))]);
        let t = simulate(&c, &bars(), 0, 9);
        assert_eq!((t[0].legs[0].i, t[0].exit_i, t[0].exit_px, t[0].open, t[0].reason.as_str()), (8, 9, 109.5, true, "open"));
    }
    #[test] fn last_bar_signal_and_pre_window_signal_do_not_trade() {
        let c = Compiled::new(vec![rule(on(&[1, 9]))], vec![rule(on(&[]))]);
        assert!(simulate(&c, &bars(), 2, 9).is_empty());
    }
    #[test] fn gaps_fill_at_the_gapped_open() {
        let mut b = bars();
        b.open[6] = 90.; b.low[6] = 89.; // gap down through the prior lows
        let c = Compiled::new(vec![rule(on(&[2]))], vec![rule(on(&[5]))]);
        let t = simulate(&c, &b, 0, 9);
        assert_eq!((t[0].exit_i, t[0].exit_px), (6, 90.));
    }
    // ---- Customize: close fills, next-open exits, short, limit entries, price exits (spec 2026-10-16 §5)
    use crate::lab::rules::{Dist, Exec, Fill};
    fn with(entry: &[usize], exit: &[usize], exec: Exec) -> Compiled { Compiled { exec, ..Compiled::new(vec![rule(on(entry))], vec![rule(on(exit))]) } }
    fn close_fill() -> Exec { Exec { fill: Fill::Close, exit_fill: Fill::Close, ..Exec::default() } }
    fn fills(t: &[Trade]) -> Vec<(usize, f64, usize, f64)> { t.iter().map(|t| (t.legs[0].i, t.legs[0].px, t.exit_i, t.exit_px)).collect() }

    #[test] fn close_fill_uses_signal_closes_and_skips_exits_on_the_entry_bar() {
        // exit rule true on the entry bar 2 is ignored (book rule); it fires at bar 4's close
        let t = simulate(&with(&[2, 4, 6], &[2, 4], close_fill()), &bars(), 0, 9);
        // no re-entry at the close that exited (bar 4); the next entry is bar 6, still open at the end
        assert_eq!(fills(&t), vec![(2, 102.5, 4, 104.5), (6, 106.5, 9, 109.5)]);
        assert_eq!((t[0].entry_kind, t[0].signal_i, t[1].open), ("close", 4, true));
        assert!(crate::trade_validation::check(&t, 0, 9, 0.0005).is_ok());
    }
    #[test] fn close_fill_with_next_open_exit_may_reenter_at_the_exit_bars_close() {
        let exec = Exec { exit_fill: Fill::NextOpen, ..close_fill() };
        let t = simulate(&with(&[1, 3, 4], &[3], exec), &bars(), 0, 9);
        // exit signal at bar 3's close (no entry that bar: the position is still open) → open of 4; entry at 4's close
        assert_eq!(fills(&t), vec![(1, 101.5, 4, 104.), (4, 104.5, 9, 109.5)]);
        assert_eq!(t[0].signal_i, 3);
    }
    #[test] fn exit_on_the_last_bar_close_is_inside_the_window() {
        let t = simulate(&with(&[2], &[9], close_fill()), &bars(), 0, 9);
        assert_eq!((t[0].exit_i, t[0].exit_px, t[0].open), (9, 109.5, false));
    }
    #[test] fn short_positions_mirror_long_ones() {
        let t = simulate(&with(&[2], &[5], Exec { short: true, ..Exec::default() }), &bars(), 0, 9);
        assert_eq!((t[0].dir, t[0].legs[0].px, t[0].exit_px), (-1., 103., 106.));
        assert!(t[0].ret(0.) < 0.); // price rose against the short
    }
    #[test] fn limit_entry_fills_on_a_touch_at_the_better_price_and_otherwise_expires() {
        let exec = Exec { limit: Some(Dist::Pct(1.)), exit_fill: Fill::Close, ..Exec::default() };
        // bar 2 close 102.5 → limit 101.475; bar 3 low 102 does not touch: the order expires (no carry-over)
        assert!(simulate(&with(&[2], &[6], exec.clone()), &bars(), 0, 9).is_empty());
        let mut b = bars();
        b.low[3] = 101.;
        let t = simulate(&with(&[2], &[6], exec.clone()), &b, 0, 9);
        assert_eq!((t[0].legs[0].i, t[0].legs[0].px, t[0].entry_kind), (3, 101.475, "limit"));
        b.open[3] = 100.5; b.low[3] = 100.; // gap below the limit: filled at the open
        assert_eq!(simulate(&with(&[2], &[6], exec.clone()), &b, 0, 9)[0].legs[0].px, 100.5);
        // short limits sit above the close and fill on the high
        let mut s = bars();
        s.high[3] = 104.;
        let t = simulate(&with(&[2], &[6], Exec { short: true, ..exec }), &s, 0, 9);
        assert_eq!((t[0].dir, t[0].legs[0].px), (-1., 103.525));
    }
    #[test] fn a_limit_fill_bar_can_exit_at_its_close_and_blocks_a_new_order() {
        let mut b = bars();
        b.low[3] = 101.;
        let exec = Exec { limit: Some(Dist::Pct(1.)), exit_fill: Fill::Close, ..Exec::default() };
        let t = simulate(&with(&[2, 3], &[3], exec), &b, 0, 9);
        assert_eq!(fills(&t), vec![(3, 101.475, 3, 103.5)]); // no new limit from bar 3's entry signal
    }
    #[test] fn a_limit_without_atr_yet_places_no_order() {
        let mut c = with(&[2], &[6], Exec { limit: Some(Dist::Atr { mult: 1., length: 14 }), exit_fill: Fill::Close, ..Exec::default() });
        c.atr.insert(14, vec![f64::NAN; 10]);
        let mut b = bars();
        b.low[3] = 0.5;
        assert!(simulate(&c, &b, 0, 9).is_empty());
    }
}
