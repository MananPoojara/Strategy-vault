//! Single-instrument simulation (spec §4, Customize §5). One position at a time; book-format `connors::Trade`s so
//! trade_validation, equity and stats are reused read-only.
//! Order of events on bar i (Phase 7 spec §6.2–§6.3), one or two sides (`direction: both`):
//! (1) at the open a pending (possibly delayed) exit of the current position fills, then a pending market entry, then,
//! while flat, resting limit / stop / stop-limit orders (one per side; two touched on one bar fill nothing);
//! (2) inside the bar, events gapped at the open first, then, by `execution.intrabar`: `stop_first` (default) takes the
//! adverse events (the protective level: tightest of stop-loss, armed trailing, breakeven; a reversal stop / stop-limit)
//! nearest the open first, then the favourable events (take-profits, target, a reversal limit) nearest the open first;
//! `ohlc_path` takes the same groups in the order of the O→H→L or O→L→H path;
//! (3) at the close, rule exits (after min_bars) fill at this close or set a next-open exit, then the other side's
//! entry reverses / exits / is ignored, then entries fill at this close, set a next-open order or rest an order.
use super::bars::Bars;
use super::rules::{Dist, Fill, Opposite};
use super::series::Compiled;
use crate::connors::{Leg, Trade};

struct Pos {
    e: usize, px: f64, kind: &'static str, stop: f64, target: f64, best: f64, limit_bar: bool,
    /// trailing arms once the best price reaches `arm` (NaN: always armed); breakeven moves the stop to `be` once the best
    /// price reaches `be_after` (NaN: no breakeven); take-profits (level, fraction, filled); scale-out pieces filled so far.
    arm: f64, be_after: f64, be: f64, tps: Vec<(f64, f64, bool)>, out: Vec<(usize, f64, f64)>,
}
impl Pos {
    /// Levels are fixed from the entry price and ATR at `sig`, the bar whose close produced the order.
    fn open(c: &Compiled, i: usize, px: f64, sig: usize, kind: &'static str, d: f64) -> Pos {
        let lvl = |x: Option<Dist>, side: f64| x.map_or(f64::NAN, |x| px + side * d * c.dist(x, px, sig));
        let x = &c.exec;
        Pos { e: i, px, kind, stop: lvl(x.stop_loss, -1.), target: lvl(x.target, 1.), best: px, limit_bar: kind == "limit",
              arm: lvl(x.trail_activate, 1.), be_after: lvl(x.breakeven.map(|b| b.0), 1.),
              be: x.breakeven.map_or(f64::NAN, |b| b.1.map_or(px, |p| px + d * c.dist(p, px, sig))),
              tps: x.take_profits.iter().map(|&(t, size)| (lvl(Some(t), 1.), size, false)).collect(), out: vec![] }
    }
    /// The trade, with the scale-out ledger when take-profits filled: pieces plus the final exit, exit_px their weighted average.
    fn trade(&self, x: usize, px: f64, reason: String, signal_i: usize, d: f64) -> Trade {
        let (exit_px, exits) = if self.out.is_empty() { (px, vec![]) } else {
            let mut v = self.out.clone();
            v.push((x, px, 1. - self.out.iter().map(|e| e.2).sum::<f64>()));
            (v.iter().map(|e| e.1 * e.2).sum(), v)
        };
        Trade { dir: d, legs: vec![Leg { i: self.e, px: self.px, w: 1. }], exit_i: x, exit_px, reason, entry_kind: self.kind, signal_i, open: false, exits }
    }
    /// Stop-loss / breakeven / trailing / take-profits / target and a resting reversal order `rev` (fill price, adverse
    /// side) inside bar i (a close-fill entry is never checked on its own bar). Partial take-profit fills go to `out`.
    fn price_exit(&mut self, c: &Compiled, b: &Bars, i: usize, d: f64, rev: Option<(f64, bool)>) -> Option<Hit> {
        // Trailing and breakeven levels from the best price through the previous bar and ATR at the previous close: known before bar i opens.
        let armed = |at: f64| !at.is_nan() && d * (self.best - at) >= 0.;
        let trail = c.exec.trailing.filter(|_| self.arm.is_nan() || armed(self.arm)).map_or(f64::NAN, |t| self.best - d * c.dist(t, self.best, i - 1));
        let be = if armed(self.be_after) { self.be } else { f64::NAN };
        // the protective level is the tightest of stop-loss, trailing and breakeven (ties keep the earlier name)
        let (stop, why) = [(self.stop, "exit.stop_loss"), (trail, "exit.trailing"), (be, "exit.breakeven")].into_iter()
            .filter(|(l, _)| !l.is_nan()).fold((f64::NAN, ""), |a, (l, w)| if a.0.is_nan() || d * (l - a.0) > 0. { (l, w) } else { a });
        let (o, adverse, favour) = (b.open[i], if d > 0. { b.low[i] } else { b.high[i] }, if d > 0. { b.high[i] } else { b.low[i] });
        // the path starts at the open: levels gapped through fill there (protective, adverse reversal, take-profits, target, limit reversal)
        if self.e < i {
            if d * (o - stop) <= 0. { return Some(Hit::Exit(o, format!("{why} (gap at the open)"))); }
            if rev.is_some_and(|(px, adv)| adv && px == o) { return Some(Hit::Reverse); }
            self.take(i, |l| d * (o - l) >= 0., Some(o));
            if d * (o - self.target) >= 0. { return Some(Hit::Exit(o, "exit.target (gap at the open)".into())); }
            if rev.is_some_and(|(px, adv)| !adv && px == o) { return Some(Hit::Reverse); }
        }
        // On a limit-fill bar the high/low may precede the fill, so only adverse events are assumed reachable.
        let favour_ok = !(self.limit_bar && self.e == i);
        // events reached inside the bar: (level, rank, take-profit index); rank 0 protective, 1 adverse reversal,
        // 2 take-profit, 3 target, 4 limit reversal (also the tie order)
        let mut adv: Vec<(f64, u8, usize)> = vec![];
        if d * (adverse - stop) <= 0. { adv.push((stop, 0, 0)); }
        if let Some((px, true)) = rev { adv.push((px, 1, 0)); }
        let mut fav: Vec<(f64, u8, usize)> = vec![];
        if favour_ok {
            fav.extend(self.tps.iter().enumerate().filter(|(_, t)| !t.2 && d * (favour - t.0) >= 0.).map(|(k, t)| (t.0, 2, k)));
            if d * (favour - self.target) >= 0. { fav.push((self.target, 3, 0)); }
            if let Some((px, false)) = rev { fav.push((px, 4, 0)); }
        }
        // inside each group the level a move from the open reaches first comes first, in both modes (ties by rank)
        let by_path = |v: &mut Vec<(f64, u8, usize)>, toward: f64| v.sort_by(|a, b| (toward * d * (a.0 - o)).total_cmp(&(toward * d * (b.0 - o))).then(a.1.cmp(&b.1)));
        by_path(&mut adv, -1.);
        by_path(&mut fav, 1.);
        // TradingView path: the open's nearer extreme comes first (a tie goes O → L → H → C); default: adverse first.
        let up_first = b.high[i] - o < o - b.low[i];
        let favour_first = c.exec.ohlc_path && favour_ok && (up_first == (d > 0.));
        let path = if favour_first { [fav, adv] } else { [adv, fav] };
        for (lvl, rank, k) in path.into_iter().flatten() {
            match rank {
                0 => return Some(Hit::Exit(lvl, why.into())),
                2 => { self.tps[k].2 = true; self.out.push((i, lvl, self.tps[k].1)); }
                3 => return Some(Hit::Exit(lvl, "exit.target".into())),
                _ => return Some(Hit::Reverse),
            }
        }
        None
    }
    /// Fills every take-profit the bar reaches, at its level (or at `at`, the gapped open).
    fn take(&mut self, i: usize, reached: impl Fn(f64) -> bool, at: Option<f64>) {
        for tp in self.tps.iter_mut().filter(|t| !t.2 && reached(t.0)) { tp.2 = true; self.out.push((i, at.unwrap_or(tp.0), tp.1)); }
    }
}

/// What bar i does to an open position: a full exit (price, reason), or the other side's resting order reverses it.
enum Hit { Exit(f64, String), Reverse }

/// A resting entry order for bars sig+1 ..= until (spec §6.2); a NaN level is an absent leg. At most one per side: placed
/// while flat, or, for the other side of a `direction: both` spec, while a position is open (a reversal order).
#[derive(Clone, Copy)]
struct Order { side: usize, sig: usize, limit: f64, stop: f64, until: usize }
impl Order {
    /// Fill price, entry kind and whether the bar's favourable extreme may precede the fill (limit-type fills), or None.
    /// A long limit fills when the low reaches it (at the better of open and limit); a long stop when the high reaches it
    /// (at the worse of open and stop); a stop-limit fills only where daily bars prove it (spec §6.2). Shorts mirror.
    fn fill(&self, b: &Bars, i: usize, d: f64) -> Option<(f64, &'static str, bool)> {
        let (o, down, up) = (b.open[i], if d > 0. { b.low[i] } else { b.high[i] }, if d > 0. { b.high[i] } else { b.low[i] });
        let reaches_limit = |p: f64| d * (p - self.limit) <= 0.;
        let reaches_stop = |p: f64| d * (p - self.stop) >= 0.;
        match (self.limit.is_nan(), self.stop.is_nan()) {
            (false, true) => reaches_limit(down).then(|| (if d > 0. { o.min(self.limit) } else { o.max(self.limit) }, "limit", true)),
            (true, false) => reaches_stop(up).then(|| (if d > 0. { o.max(self.stop) } else { o.min(self.stop) }, "stop", false)),
            (false, false) if reaches_stop(o) => {
                if reaches_limit(o) { Some((o, "stop_limit", false)) } else if reaches_limit(down) { Some((self.limit, "stop_limit", true)) } else { None }
            }
            (false, false) => (reaches_stop(up) && reaches_limit(self.stop)).then_some((self.stop, "stop_limit", false)),
            (true, true) => None,
        }
    }
}

/// One position at a time over one side (long or short) or, for `direction: both`, two sides (`c` long, `c.other` short).
pub fn simulate(c: &Compiled, b: &Bars, from: usize, to: usize) -> Vec<Trade> {
    let sides: Vec<&Compiled> = std::iter::once(c).chain(c.other.as_deref()).collect();
    let dir = |s: usize| if sides[s].exec.short { -1. } else { 1. };
    let x = &c.exec; // fill, exit_fill, opposite, delay and edge are shared by both sides
    let delay = x.delay;
    // v1 rule kept: a next-open market entry may share the open with a next-open exit. Close fills and resting orders
    // follow the books instead: no new order on a bar that held a position at its close or exited after its open.
    let shares_open = sides.iter().all(|s| !s.has_order()) && x.fill == Fill::NextOpen;
    let holds = |s: usize, i: usize| sides[s].entry.iter().all(|r| r.holds(i, None));
    let fires = |s: usize, i: usize| holds(s, i) && (!x.edge || i == 0 || !holds(s, i - 1));
    // An order's levels come from the signal bar; a configured level that is unknown (warm-up) places no order.
    let place = |s: usize, i: usize| -> Option<Order> {
        let (side, d, close) = (sides[s], dir(s), b.close[i]);
        let limit = side.limit_at.as_ref().map(|a| a[i]).or(side.exec.limit.map(|l| close - d * side.dist(l, close, i)));
        let stop = side.stop_at.as_ref().map(|a| a[i]).or(side.exec.stop.map(|st| close + d * side.dist(st, close, i)));
        (limit.map_or(true, f64::is_finite) && stop.map_or(true, f64::is_finite))
            .then_some(Order { side: s, sig: i, limit: limit.unwrap_or(f64::NAN), stop: stop.unwrap_or(f64::NAN), until: i + side.exec.valid_bars })
    };
    let mut trades = Vec::new();
    let mut pos: Option<(usize, Pos)> = None; // (side, position)
    let mut pending_entry: Option<(usize, usize, usize, &'static str)> = None; // (side, signal bar, fill bar, entry kind)
    let mut pending_exit: Option<(usize, usize, String)> = None; // (signal bar, fill bar, reason); cleared when its position closes
    let mut orders: [Option<Order>; 2] = [None, None]; // one resting order per side
    for i in from..=to {
        let mut closed_at_open = false;
        if pending_exit.as_ref().is_some_and(|e| e.1 == i) {
            let (sig, _, why) = pending_exit.take().unwrap();
            if let Some((s, p)) = pos.take() { trades.push(p.trade(i, b.open[i], why, sig, dir(s))); closed_at_open = true; }
        }
        if pending_entry.is_some_and(|e| e.2 == i) {
            let (s, sig, _, kind) = pending_entry.take().unwrap();
            // a reversal whose closing half was already done by a price exit or a reversal order is a plain entry
            let kind = if kind == "reverse" && !closed_at_open { "next_open" } else { kind };
            if pos.is_none() { pos = Some((s, Pos::open(sides[s], i, b.open[i], sig, kind, dir(s)))); }
        }
        if pos.is_none() {
            // resting orders while flat: one fill opens a position (the other side's order keeps resting); two fills on one
            // bar are ambiguous on daily bars and fill nothing
            let filled: Vec<(Order, (f64, &'static str, bool))> = orders.iter().flatten().filter_map(|o| o.fill(b, i, dir(o.side)).map(|f| (*o, f))).collect();
            if let [(o, (px, kind, limit_bar))] = filled.as_slice() {
                orders[o.side] = None;
                let mut p = Pos::open(sides[o.side], i, *px, o.sig, kind, dir(o.side));
                p.limit_bar = *limit_bar;
                pos = Some((o.side, p));
            }
        }
        let mut exited = false; // after the open: blocks a same-bar order under book rules
        if let Some((s, p)) = pos.as_mut() {
            let (s, d) = (*s, dir(*s));
            // the other side's resting order is a reversal order under `opposite: reverse` only; a limit fills on the
            // favourable side, a stop or stop-limit on the adverse side
            let rev = orders[1 - s.min(1)].filter(|o| o.side != s && x.opposite == Opposite::Reverse).and_then(|o| o.fill(b, i, dir(o.side)).map(|f| (o, f)));
            match p.price_exit(sides[s], b, i, d, rev.map(|(_, (px, kind, _))| (px, kind != "limit"))) {
                Some(Hit::Exit(px, why)) => {
                    trades.push(pos.take().unwrap().1.trade(i, px, why, i, d));
                    (exited, pending_exit) = (true, None); // a delayed exit belongs to the position it was set for
                }
                Some(Hit::Reverse) => {
                    // a reversal order fills: the position closes and the other side opens at the same price
                    let (o, (px, _, limit_bar)) = rev.unwrap();
                    trades.push(pos.take().unwrap().1.trade(i, px, format!("reverse to {}", if dir(o.side) > 0. { "long" } else { "short" }), i, d));
                    let mut p = Pos::open(sides[o.side], i, px, o.sig, "reverse", dir(o.side));
                    p.limit_bar = limit_bar;
                    pos = Some((o.side, p));
                    orders[o.side] = None;
                    pending_exit = None;
                }
                None => {}
            }
        }
        if let Some((s, _)) = &pos {
            orders[*s] = None; // a side in a position has no entry order
            if x.opposite != Opposite::Reverse { orders = [None, None]; } // nor, unless it reverses, the other side
        }
        for o in orders.iter_mut() { if o.is_some_and(|o| i >= o.until) { *o = None; } }
        let last = i == to;
        if let (Some((s, p)), None) = (&pos, &pending_exit) { // a pending (delayed) exit is not re-placed
            let s = *s;
            // rule exits count from min_bars after the fill (price exits stay active)
            if let Some(r) = sides[s].exit.iter().find(|r| i - p.e >= sides[s].exec.min_bars && r.holds(i, Some(p.e))) {
                match x.exit_fill {
                    Fill::Close => { trades.push(pos.take().unwrap().1.trade(i, b.close[i], r.label.clone(), i, dir(s))); exited = true; }
                    Fill::NextOpen if i + delay <= to => pending_exit = Some((i, i + delay, r.label.clone())),
                    Fill::NextOpen => {} // no fill inside the window; the position is marked open below
                }
            }
        }
        if let Some((s, p)) = pos.as_mut() {
            let d = dir(*s);
            if !(p.limit_bar && p.e == i) { p.best = if d > 0. { p.best.max(b.high[i]) } else { p.best.min(b.low[i]) }; }
        }
        if last { break; } // a signal on the last bar has no fill inside the window
        let want: Vec<usize> = (0..sides.len()).filter(|&s| fires(s, i)).collect();
        // The other side's entry while a position is still open (direction both): reverse, exit, or ignore.
        if let Some(&(s, _)) = pos.as_ref().filter(|_| pending_exit.is_none()) {
            let o = 1 - s.min(1);
            if sides.len() == 2 && want.contains(&o) {
                let why = if x.opposite == Opposite::Reverse { format!("reverse to {}", if dir(o) > 0. { "long" } else { "short" }) } else { "opposite entry".into() };
                match (x.opposite, x.fill, sides[o].has_order()) {
                    (Opposite::Ignore, ..) => {}
                    (Opposite::Reverse, _, true) => { if orders[o].is_none() { orders[o] = place(o, i); } }
                    // a reversal is one fill: exit and new entry at the same price, exempt from the same-bar re-entry rule
                    (Opposite::Reverse, Fill::Close, false) => {
                        trades.push(pos.take().unwrap().1.trade(i, b.close[i], why, i, dir(s)));
                        pos = Some((o, Pos::open(sides[o], i, b.close[i], i, "reverse", dir(o))));
                    }
                    (Opposite::Reverse, Fill::NextOpen, false) => if i + delay <= to { pending_exit = Some((i, i + delay, why)); pending_entry = Some((o, i, i + delay, "reverse")); },
                    (Opposite::Exit, ..) => match x.exit_fill {
                        Fill::Close => trades.push(pos.take().unwrap().1.trade(i, b.close[i], why, i, dir(s))),
                        Fill::NextOpen => if i + delay <= to { pending_exit = Some((i, i + delay, why)); },
                    },
                }
                continue;
            }
        }
        let flat = if shares_open { pos.is_none() || pending_exit.is_some() } else { pos.is_none() && !exited };
        let may_enter = flat && pending_entry.is_none();
        // Both sides firing while flat: two order sides each rest an order (a channel breakout); otherwise it is ambiguous
        // and nothing is placed (spec §6.1). One order side rests its order; one market side enters and cancels a resting
        // order of the other side.
        if may_enter {
            match *want.as_slice() {
                [a, z] if sides[a].has_order() && sides[z].has_order() => for s in [a, z] { if orders[s].is_none() { orders[s] = place(s, i); } },
                [s] if sides[s].has_order() => if orders[s].is_none() { orders[s] = place(s, i); },
                [s] => {
                    orders = [None, None];
                    let (d, side) = (dir(s), sides[s]);
                    match x.fill {
                        Fill::Close => pos = Some((s, Pos::open(side, i, b.close[i], i, "close", d))),
                        Fill::NextOpen => if i + delay <= to { pending_entry = Some((s, i, i + delay, "next_open")); },
                    }
                }
                _ => {}
            }
        }
    }
    if let Some((s, p)) = pos { trades.push(Trade { open: true, ..p.trade(to, b.close[to], "open".into(), to, dir(s)) }); }
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
    use crate::lab::rules::{Dist, Exec, Fill, Operand};
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
        let mut late = bars();
        late.low[4] = 101.; // a later touch must not fill an expired order
        assert!(simulate(&with(&[2], &[6], exec.clone()), &late, 0, 9).is_empty());
        let mut b = bars();
        b.low[3] = 101.;
        let t = simulate(&with(&[2], &[6], exec.clone()), &b, 0, 9);
        assert_eq!((t[0].legs[0].i, t[0].legs[0].px, t[0].entry_kind), (3, 101.475, "limit"));
        b.open[3] = 100.5; b.low[3] = 100.; // gap below the limit: filled at the open
        assert_eq!(simulate(&with(&[2], &[6], exec.clone()), &b, 0, 9)[0].legs[0].px, 100.5);
        // short limits sit above the close and fill on the high
        let mut s = bars();
        s.high[3] = 103.5; // the low (102) is below the limit, but only the high counts for a short
        assert!(simulate(&with(&[2], &[6], Exec { short: true, ..exec.clone() }), &s, 0, 9).is_empty());
        s.high[3] = 104.;
        let t = simulate(&with(&[2], &[6], Exec { short: true, ..exec }), &s, 0, 9);
        assert_eq!((t[0].dir, t[0].legs[0].px), (-1., 103.525));
    }
    #[test] fn a_limit_fill_bar_can_exit_at_its_close_and_blocks_a_new_order() {
        let mut b = bars();
        b.low[3] = 101.;
        b.low[4] = 102.; // would touch a limit placed from bar 3 (102.465) if one were allowed
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
    #[test] fn stop_and_target_fill_at_their_levels_or_the_gapped_open() {
        let exec = Exec { stop_loss: Some(Dist::Pct(2.)), target: Some(Dist::Pct(3.)), ..close_fill() };
        // entry close 102.5 → stop 100.45, target 105.575
        let mut b = bars();
        b.low[4] = 100.;
        let t = simulate(&with(&[2, 4], &[], exec.clone()), &b, 0, 9);
        assert_eq!((t.len(), t[0].exit_i, t[0].signal_i), (1, 4, 4));
        assert_eq!((t[0].exit_px, t[0].reason.as_str()), (100.45, "exit.stop_loss"));
        let t = simulate(&with(&[2], &[], exec.clone()), &bars(), 0, 9); // highs reach 105.575 on bar 5 (high 106)
        assert_eq!((t[0].exit_i, t[0].exit_px, t[0].reason.as_str()), (5, 105.575, "exit.target"));
        let mut g = bars();
        g.open[3] = 99.; g.low[3] = 98.; // gap through the stop
        let t = simulate(&with(&[2], &[], exec.clone()), &g, 0, 9);
        assert_eq!((t[0].exit_px, t[0].reason.as_str()), (99., "exit.stop_loss (gap at the open)"));
        let mut g = bars();
        g.open[3] = 107.; g.high[3] = 108.; // gap through the target
        let t = simulate(&with(&[2], &[], exec), &g, 0, 9);
        assert_eq!((t[0].exit_px, t[0].reason.as_str()), (107., "exit.target (gap at the open)"));
    }
    #[test] fn stop_and_target_inside_one_bar_take_the_stop() {
        let exec = Exec { stop_loss: Some(Dist::Pct(2.)), target: Some(Dist::Pct(3.)), ..close_fill() };
        let mut b = bars();
        b.low[3] = 100.; b.high[3] = 106.;
        let t = simulate(&with(&[2], &[], exec), &b, 0, 9);
        assert_eq!((t[0].exit_i, t[0].exit_px, t[0].reason.as_str()), (3, 100.45, "exit.stop_loss"));
    }
    #[test] fn a_limit_fill_bar_assumes_the_stop_but_never_the_target() {
        let exec = Exec { limit: Some(Dist::Pct(1.)), stop_loss: Some(Dist::Pct(1.)), target: Some(Dist::Pct(1.)), exit_fill: Fill::Close, ..Exec::default() };
        let mut b = bars();
        b.low[3] = 101.; b.high[3] = 110.; // limit 101.475 fills; target 102.48975 inside the bar is not taken
        let t = simulate(&with(&[2], &[], exec.clone()), &b, 0, 9);
        assert_eq!((t[0].legs[0].i, t[0].exit_i, t[0].reason.as_str()), (3, 4, "exit.target (gap at the open)"));
        b.low[3] = 100.; // low also below the stop 100.46025: the stop is assumed hit after the fill
        let t = simulate(&with(&[2], &[], exec), &b, 0, 9);
        assert_eq!((t[0].exit_i, t[0].reason.as_str()), (3, "exit.stop_loss"));
        assert!((t[0].exit_px - 100.46025).abs() < 1e-9);
        let mut b = bars();
        b.low[3] = 101.; b.open[4] = 102.; b.low[4] = 101.9; // after the fill bar the target is live again
        let t = simulate(&with(&[2], &[], Exec { limit: Some(Dist::Pct(1.)), stop_loss: Some(Dist::Pct(1.)), target: Some(Dist::Pct(1.)), exit_fill: Fill::Close, ..Exec::default() }), &b, 0, 9);
        assert_eq!((t[0].exit_i, t[0].reason.as_str()), (4, "exit.target"));
    }
    #[test] fn trailing_level_comes_from_bars_already_closed() {
        let exec = Exec { trailing: Some(Dist::Pct(2.)), ..close_fill() };
        let mut b = bars();
        // entry close 102.5 at bar 2. Bar 3 high 103.9 → level for bar 4 = 103.9 * 0.98 = 101.822.
        // Bar 3's own low 101.6 is below its own high's level but above the level known at its open (102.5 * 0.98 = 100.45).
        b.high[3] = 103.9; b.low[3] = 101.6;
        b.low[4] = 101.8; // touches 101.822 → exit at the level on bar 4
        let t = simulate(&with(&[2], &[], exec), &b, 0, 9);
        assert_eq!((t[0].exit_i, t[0].reason.as_str()), (4, "exit.trailing"));
        assert!((t[0].exit_px - 101.822).abs() < 1e-9);
    }
    #[test] fn atr_distances_are_read_at_the_signal_bar() {
        let mut c = with(&[2], &[], Exec { stop_loss: Some(Dist::Atr { mult: 2., length: 14 }), ..Exec::default() });
        c.atr.insert(14, vec![f64::NAN, f64::NAN, 1., 50., 50., 50., 50., 50., 50., 50.]);
        let mut b = bars();
        b.low[4] = 100.9; // stop = open of bar 3 (103) - 2 x ATR(bar 2 = 1) = 101; ATR 50 on the fill bar is not used
        let t = simulate(&c, &b, 0, 9);
        assert_eq!((t[0].legs[0].i, t[0].exit_i, t[0].exit_px), (3, 4, 101.));
        c.atr.insert(14, vec![f64::NAN; 10]); // ATR still warming up: no level, no stop
        assert!(simulate(&c, &b, 0, 9)[0].open);
    }
    #[test] fn short_levels_mirror_long_ones() {
        let exec = Exec { short: true, stop_loss: Some(Dist::Pct(2.)), target: Some(Dist::Pct(3.)), ..close_fill() };
        // short at the close 102.5 → stop 104.55 above, target 99.425 below; bar 4's high 105 reaches the stop
        let t = simulate(&with(&[2], &[], exec.clone()), &bars(), 0, 9);
        assert_eq!((t[0].dir, t[0].exit_i, t[0].reason.as_str()), (-1., 4, "exit.stop_loss"));
        assert!((t[0].exit_px - 104.55).abs() < 1e-9);
        let mut b = bars();
        b.low[3] = 99.;
        let t = simulate(&with(&[2], &[], exec), &b, 0, 9);
        assert_eq!((t[0].exit_i, t[0].reason.as_str()), (3, "exit.target"));
        assert!((t[0].exit_px - 99.425).abs() < 1e-9);
    }
    #[test] fn a_next_open_fill_can_stop_out_on_its_own_bar() {
        let mut b = bars();
        b.low[3] = 101.; // fill at the open 103, stop 101.97
        let t = simulate(&with(&[2], &[], Exec { stop_loss: Some(Dist::Pct(1.)), ..Exec::default() }), &b, 0, 9);
        assert_eq!((t[0].legs[0].i, t[0].exit_i, t[0].reason.as_str()), (3, 3, "exit.stop_loss"));
        assert!((t[0].exit_px - 101.97).abs() < 1e-9);
    }
    #[test] fn trailing_atr_is_read_at_the_previous_close() {
        let mut c = with(&[2], &[], Exec { trailing: Some(Dist::Atr { mult: 1., length: 14 }), ..close_fill() });
        c.atr.insert(14, vec![f64::NAN, f64::NAN, 1., 1., 50., 50., 50., 50., 50., 50.]);
        let t = simulate(&c, &bars(), 0, 9);
        assert_eq!((t[0].exit_i, t[0].exit_px, t[0].reason.as_str()), (4, 103., "exit.trailing"));
    }
    #[test] fn the_tighter_of_stop_and_trailing_wins() {
        let exec = Exec { stop_loss: Some(Dist::Pct(2.)), trailing: Some(Dist::Pct(1.)), ..close_fill() };
        let mut b = bars();
        b.low[4] = 102.5;
        let t = simulate(&with(&[2], &[], exec), &b, 0, 9);
        assert_eq!((t[0].exit_i, t[0].reason.as_str()), (4, "exit.trailing"));
        assert!((t[0].exit_px - 102.96).abs() < 1e-9);
    }
    #[test] fn a_limit_fill_bars_high_does_not_raise_the_trailing_best() {
        let exec = Exec { limit: Some(Dist::Pct(1.)), trailing: Some(Dist::Pct(2.)), exit_fill: Fill::Close, ..Exec::default() };
        let mut b = bars();
        b.low[3] = 101.; b.low[4] = 101.5;
        let t = simulate(&with(&[2], &[], exec), &b, 0, 9);
        assert_eq!((t[0].legs[0].i, t[0].open), (3, true));
    }
    #[test] fn short_trailing_tracks_lows() {
        let exec = Exec { short: true, trailing: Some(Dist::Pct(2.)), ..close_fill() };
        let t = simulate(&with(&[2], &[], exec), &bars(), 0, 9);
        assert_eq!((t[0].exit_i, t[0].reason.as_str()), (4, "exit.trailing"));
        assert!((t[0].exit_px - 104.04).abs() < 1e-9);
    }
    #[test] fn price_exits_come_before_close_rule_exits() {
        let exec = Exec { stop_loss: Some(Dist::Pct(2.)), ..close_fill() };
        let mut b = bars();
        b.low[4] = 100.;
        let t = simulate(&with(&[2], &[4], exec), &b, 0, 9);
        assert_eq!((t[0].exit_i, t[0].exit_px, t[0].reason.as_str()), (4, 100.45, "exit.stop_loss"));
    }
    #[test] fn future_bars_never_change_past_trades() {
        use crate::lab::series::{compile, Cache, Frames};
        let specs = [
            r#"{"v":1,"entry":{"all":[{"lhs":{"s":"rsi","p":{"length":2}},"op":"<","rhs":{"value":20}}]},"exit":{"any":[{"lhs":{"s":"rsi","p":{"length":2}},"op":">","rhs":{"value":70}}]}}"#,
            r#"{"v":1,"entry":{"all":[{"lhs":{"s":"rsi","p":{"length":2}},"op":"<","rhs":{"value":20}}]},"exit":{"any":[],"stop_loss":{"pct":2},"target":{"pct":3}},"execution":{"fill":"close"}}"#,
            r#"{"v":1,"entry":{"all":[{"lhs":{"s":"close"},"op":">","rhs":{"s":"sma","p":{"length":5}}}]},"exit":{"any":[{"lhs":{"s":"bars_held"},"op":">=","rhs":{"value":8}}],"trailing":{"atr":2,"atr_length":10}}}"#,
            r#"{"v":1,"direction":"short","entry":{"all":[{"lhs":{"s":"rsi","p":{"length":2}},"op":">","rhs":{"value":80}}],"limit":{"atr":0.4,"atr_length":10}},"exit":{"any":[{"lhs":{"s":"rsi","p":{"length":2}},"op":"<","rhs":{"value":40}}],"stop_loss":{"atr":3}},"execution":{"exit_fill":"close"}}"#,
            // Phase 7: two sides (reverse / opposite exit), resting stop and stop-limit orders, `at` levels, valid_bars, delay, edge
            r#"{"v":1,"direction":"both","entry":{"all":[{"lhs":{"s":"rsi","p":{"length":2}},"op":"<","rhs":{"value":15}}]},"exit":{"any":[{"lhs":{"s":"bars_held"},"op":">=","rhs":{"value":12}}]},"short_side":{"entry":{"all":[{"lhs":{"s":"rsi","p":{"length":2}},"op":">","rhs":{"value":85}}]},"exit":{"any":[{"lhs":{"s":"bars_held"},"op":">=","rhs":{"value":12}}]}}}"#,
            r#"{"v":1,"direction":"both","entry":{"all":[{"lhs":{"s":"rsi","p":{"length":2}},"op":"<","rhs":{"value":15}}]},"exit":{"any":[{"lhs":{"s":"bars_held"},"op":">=","rhs":{"value":12}}]},"short_side":{"entry":{"all":[{"lhs":{"s":"rsi","p":{"length":2}},"op":">","rhs":{"value":85}}]},"exit":{"any":[{"lhs":{"s":"bars_held"},"op":">=","rhs":{"value":12}}]}},"execution":{"opposite":"exit","delay_bars":3,"entry_on":"edge"}}"#,
            r#"{"v":1,"direction":"both","entry":{"all":[{"lhs":{"s":"close"},"op":">","rhs":{"value":0}}],"stop":{"at":{"s":"highest","p":{"length":10}}},"valid_bars":3},"exit":{"any":[],"stop_loss":{"pct":5}},"short_side":{"entry":{"all":[{"lhs":{"s":"close"},"op":">","rhs":{"value":0}}],"stop":{"at":{"s":"lowest","p":{"length":10}}},"valid_bars":3},"exit":{"any":[],"stop_loss":{"pct":5}}}}"#,
            r#"{"v":1,"entry":{"all":[{"lhs":{"s":"rsi","p":{"length":2}},"op":">","rhs":{"value":60}}],"stop":{"points":0.3},"valid_bars":2},"exit":{"any":[{"lhs":{"s":"bars_held"},"op":">=","rhs":{"value":4}}]}}"#,
            r#"{"v":1,"entry":{"all":[{"lhs":{"s":"close"},"op":">","rhs":{"s":"sma","p":{"length":5}}}],"stop":{"pct":0.5},"limit":{"pct":0},"valid_bars":2},"exit":{"any":[{"lhs":{"s":"bars_held"},"op":">=","rhs":{"value":5}}]},"execution":{"delay_bars":2,"entry_on":"edge"}}"#,
            // Phase 7 exits: trailing activation, breakeven, take-profits, min_bars, both intrabar paths, one and two sides
            r#"{"v":1,"entry":{"all":[{"lhs":{"s":"rsi","p":{"length":2}},"op":"<","rhs":{"value":20}}]},"exit":{"any":[{"lhs":{"s":"bars_held"},"op":">=","rhs":{"value":15}}],"stop_loss":{"pct":4},"trailing":{"atr":1.5,"atr_length":10,"activate":{"atr":1,"atr_length":10}},"breakeven":{"after":{"pct":1.5},"plus":{"points":0.2}},"take_profits":[{"pct":2,"size":0.3},{"atr":2,"atr_length":10,"size":0.3}],"min_bars":3},"execution":{"intrabar":"ohlc_path"}}"#,
            r#"{"v":1,"direction":"short","entry":{"all":[{"lhs":{"s":"rsi","p":{"length":2}},"op":">","rhs":{"value":80}}]},"exit":{"any":[{"lhs":{"s":"rsi","p":{"length":2}},"op":"<","rhs":{"value":30}}],"stop_loss":{"pct":3},"breakeven":{"after":{"atr":1,"atr_length":10}},"take_profits":[{"pct":1.5,"size":0.5}],"trailing":{"pct":2,"activate":{"pct":1}},"min_bars":2},"execution":{"fill":"close"}}"#,
            r#"{"v":1,"direction":"both","entry":{"all":[{"lhs":{"s":"rsi","p":{"length":2}},"op":"<","rhs":{"value":15}}]},"exit":{"any":[],"stop_loss":{"pct":3},"target":{"pct":4},"take_profits":[{"pct":2,"size":0.5}]},"short_side":{"entry":{"all":[{"lhs":{"s":"rsi","p":{"length":2}},"op":">","rhs":{"value":85}}]},"exit":{"any":[],"trailing":{"pct":2,"activate":{"pct":1}},"breakeven":{"after":{"pct":1}}}},"execution":{"intrabar":"ohlc_path"}}"#,
        ];
        let full = crate::lab::testdata::walk(400, 5, false);
        for text in specs {
            let spec = crate::lab::rules::parse_spec(text).unwrap();
            let run = |b: &Bars| simulate(&compile(&spec, &Frames::new(b.clone()), &mut Cache::new()).unwrap(), b, 0, b.len() - 1);
            let want = run(&full);
            assert!(want.len() > 3, "{text}: only {} trades", want.len());
            for k in 30usize..398 {
                let mut alt = full.clone();
                for i in k + 1..alt.len() { for v in [&mut alt.open, &mut alt.high, &mut alt.low, &mut alt.close] { v[i] *= 0.7; } }
                let done = |t: &[Trade]| fills(&t.iter().filter(|t| !t.open && t.exit_i <= k).cloned().collect::<Vec<_>>());
                assert_eq!(done(&run(&alt)), done(&want), "{text}: bars after {k} changed earlier trades");
                // An order or next-open fill on bar k+1 reads only that bar's open, high and low: moving its close (and every
                // later bar) never changes an entry made up to k+1 (close fills read the close and are skipped).
                // Likewise a price exit (stop, trailing, breakeven, target, take-profit pieces) on bar k+1 uses only levels known
                // before it opened.
                if k + 2 >= full.len() { continue; }
                alt.open[k + 1] = full.open[k + 1]; alt.high[k + 1] = full.high[k + 1]; alt.low[k + 1] = full.low[k + 1];
                alt.close[k + 1] = (full.high[k + 1] + full.low[k + 1]) / 2.;
                let got = run(&alt);
                if spec.exec.fill != crate::lab::rules::Fill::Close {
                    let entries = |t: &[Trade]| t.iter().filter(|t| t.legs[0].i <= k + 1).map(|t| (t.legs[0].i, t.legs[0].px, t.dir)).collect::<Vec<_>>();
                    assert_eq!(entries(&got), entries(&want), "{text}: the close of bar {} moved an entry filled on it", k + 1);
                }
                let priced = |t: &[Trade]| t.iter().filter(|t| !t.open && t.exit_i <= k + 1 && ["exit.stop_loss", "exit.trailing", "exit.breakeven", "exit.target"].iter().any(|p| t.reason.starts_with(p)))
                    .map(|t| (t.legs[0].i, t.legs[0].px, t.exit_i, t.exit_px, t.exits.clone())).collect::<Vec<_>>();
                assert_eq!(priced(&got), priced(&want), "{text}: the close of bar {} moved a price exit on it", k + 1);
                // A one-sided order entry on bar k+1 fills at a price set by its level and the open: widening that bar's
                // high and low outward (open fixed) keeps it filled at the same price.
                if spec.short_side.is_some() { continue; }
                let mut wide = full.clone();
                let r = full.high[k + 1] - full.low[k + 1];
                wide.high[k + 1] += r; wide.low[k + 1] -= r.min(full.low[k + 1] / 2.);
                let got = run(&wide);
                for t in want.iter().filter(|t| t.legs[0].i == k + 1 && ["limit", "stop", "stop_limit"].contains(&t.entry_kind)) {
                    let key = (t.legs[0].i, t.legs[0].px, t.entry_kind);
                    assert!(got.iter().any(|g| (g.legs[0].i, g.legs[0].px, g.entry_kind) == key), "{text}: a wider bar {} moved the order fill {key:?}", k + 1);
                }
            }
        }
    }

    // ---- Phase 7: two sides, reversal, opposite exit, conflict (spec §6.1)
    fn both(long_in: &[usize], short_in: &[usize], exec: Exec) -> Compiled {
        let mut c = with(long_in, &[], exec.clone());
        c.other = Some(Box::new(with(short_in, &[], Exec { short: true, ..exec })));
        c
    }
    #[test] fn reversal_at_the_next_open_closes_and_opens_at_one_price() {
        let t = simulate(&both(&[1], &[4], Exec::default()), &bars(), 0, 9);
        assert_eq!(fills(&t), vec![(2, 102., 5, 105.), (5, 105., 9, 109.5)]);
        assert_eq!((t[0].dir, t[0].reason.as_str(), t[1].dir, t[1].entry_kind, t[1].open), (1., "reverse to short", -1., "reverse", true));
        assert!(crate::trade_validation::check(&t, 0, 9, 0.0005).is_ok());
    }
    #[test] fn reversal_at_the_close_is_one_fill_and_exempt_from_the_reentry_block() {
        let t = simulate(&both(&[1, 6], &[4], close_fill()), &bars(), 0, 9);
        assert_eq!(fills(&t), vec![(1, 101.5, 4, 104.5), (4, 104.5, 6, 106.5), (6, 106.5, 9, 109.5)]);
        assert_eq!((t[1].dir, t[2].dir, t[2].entry_kind), (-1., 1., "reverse"));
    }
    #[test] fn opposite_exit_only_closes_and_ignore_keeps_the_position() {
        let exit = Exec { opposite: Opposite::Exit, ..Exec::default() };
        let t = simulate(&both(&[1], &[4, 6], exit), &bars(), 0, 9);
        // short entry at 4 only closes the long (open of 5); the next short signal (6) opens a short at 7's open
        assert_eq!(fills(&t), vec![(2, 102., 5, 105.), (7, 107., 9, 109.5)]);
        assert_eq!((t[0].reason.as_str(), t[1].dir), ("opposite entry", -1.));
        let t = simulate(&both(&[1], &[4], Exec { opposite: Opposite::Ignore, ..Exec::default() }), &bars(), 0, 9);
        assert_eq!(fills(&t), vec![(2, 102., 9, 109.5)]);
    }
    #[test] fn both_entries_on_one_flat_bar_enter_nothing() {
        let t = simulate(&both(&[2, 5], &[2], Exec::default()), &bars(), 0, 9);
        assert_eq!(fills(&t), vec![(6, 106., 9, 109.5)]);
    }
    #[test] fn one_sided_specs_run_the_same_path() {
        // a single long side never looks at `other`; the pinned-output test covers every template and v1 spec
        let t = simulate(&with(&[1, 4], &[3], Exec::default()), &bars(), 0, 9);
        assert_eq!(fills(&t), vec![(2, 102., 4, 104.), (5, 105., 9, 109.5)]);
    }

    // ---- Phase 7: stop, stop-limit, `at` levels, valid_bars, delay, edge, reversal orders (spec §6.1–§6.2)
    fn ordered(entry: &[usize], exit: &[usize], exec: Exec) -> Compiled { with(entry, exit, Exec { exit_fill: Fill::Close, ..exec }) }
    #[test] fn stop_entries_fill_at_the_level_or_the_gapped_open() {
        let stop = Exec { stop: Some(Dist::Pct(1.)), ..Exec::default() }; // bar 2 close 102.5 → stop 103.525
        let t = simulate(&ordered(&[2], &[6], stop.clone()), &bars(), 0, 9);
        assert_eq!((fills(&t), t[0].entry_kind), (vec![(3, 103.525, 6, 106.5)], "stop"));
        let mut g = bars();
        g.open[3] = 104.; g.high[3] = 105.; // gaps over the stop: filled at the open
        assert_eq!(simulate(&ordered(&[2], &[6], stop.clone()), &g, 0, 9)[0].legs[0].px, 104.);
        let mut short = bars();
        short.low[3] = 101.; // short stop below the close: 101.475
        let t = simulate(&ordered(&[2], &[6], Exec { short: true, ..stop }), &short, 0, 9);
        assert_eq!((t[0].dir, t[0].legs[0].px), (-1., 101.475));
    }
    #[test] fn orders_rest_for_valid_bars_then_expire() {
        let mut b = bars();
        b.high[3] = 103.4; // below the 103.525 stop
        let one = Exec { stop: Some(Dist::Pct(1.)), ..Exec::default() };
        assert!(simulate(&ordered(&[2], &[7], one.clone()), &b, 0, 9).is_empty());
        let t = simulate(&ordered(&[2], &[7], Exec { valid_bars: 2, ..one }), &b, 0, 9);
        assert_eq!(fills(&t), vec![(4, 104., 7, 107.5)]); // bar 4 opens at 104, above the stop
    }
    #[test] fn stop_limit_fills_only_where_the_bar_proves_it() {
        let sl = |limit: Vec<f64>| { let mut c = ordered(&[2], &[6], Exec { stop: Some(Dist::Pct(1.)), limit: Some(Dist::Pct(0.)), ..Exec::default() }); if !limit.is_empty() { c.exec.limit = None; c.exec.limit_at = Some(Operand::Value(0.)); c.limit_at = Some(limit); } c };
        // stop 103.525 triggers inside bar 3 but the limit (102.5, the signal close) is below it: no proof of a fill
        assert!(simulate(&sl(vec![]), &bars(), 0, 9).is_empty());
        // triggered at the open (gap to 104) and the low comes back to the limit: filled at the limit, a limit-type bar
        let mut g = bars();
        g.open[3] = 104.; g.high[3] = 105.; g.low[3] = 102.;
        let t = simulate(&sl(vec![]), &g, 0, 9);
        assert_eq!((t[0].legs[0].px, t[0].entry_kind), (102.5, "stop_limit"));
        // limit above the stop: an intrabar trigger fills at the stop
        let t = simulate(&sl(vec![104.; 10]), &bars(), 0, 9);
        assert_eq!((t[0].legs[0].i, t[0].legs[0].px), (3, 103.525));
    }
    #[test] fn at_levels_come_from_the_signal_bar_and_unknown_levels_place_nothing() {
        let mut c = ordered(&[2], &[7], Exec { stop_at: Some(Operand::Value(0.)), valid_bars: 2, ..Exec::default() });
        c.stop_at = Some(vec![f64::NAN, f64::NAN, 105., 1., 1., 1., 1., 1., 1., 1.]); // later values never move a resting order
        assert_eq!(fills(&simulate(&c, &bars(), 0, 9)), vec![(4, 105., 7, 107.5)]); // bar 3 high 104 < 105; bar 4 high 105
        c.stop_at = Some(vec![f64::NAN; 10]);
        assert!(simulate(&c, &bars(), 0, 9).is_empty());
    }
    #[test] fn delay_bars_moves_next_open_fills() {
        let t = simulate(&with(&[2, 8], &[6, 7], Exec { delay: 2, ..Exec::default() }), &bars(), 0, 9);
        // entry signal 2 → open of 4; exit signal 6 → open of 8 (bar 7's repeat does not move it); signal 8 + 2 is past the window
        assert_eq!(fills(&t), vec![(4, 104., 8, 108.)]);
    }
    #[test] fn edge_entries_need_a_false_to_true_turn() {
        let level = simulate(&with(&[1, 2, 3, 6], &[2], Exec::default()), &bars(), 0, 9);
        assert_eq!(fills(&level), vec![(2, 102., 3, 103.), (3, 103., 9, 109.5)]);
        let edge = simulate(&with(&[1, 2, 3, 6], &[2], Exec { edge: true, ..Exec::default() }), &bars(), 0, 9);
        assert_eq!(fills(&edge), vec![(2, 102., 3, 103.), (7, 107., 9, 109.5)]);
    }
    #[test] fn both_sides_rest_an_order_while_flat_and_the_touched_one_fills() {
        let mut c = with(&[2], &[], Exec { stop: Some(Dist::Pct(1.)), valid_bars: 3, ..Exec::default() });
        c.other = Some(Box::new(with(&[2], &[], Exec { short: true, stop: Some(Dist::Pct(1.)), valid_bars: 3, ..Exec::default() })));
        // bar 2 close 102.5: long stop 103.525 above, short stop 101.475 below; bar 3 (high 104, low 102) touches only the long
        let t = simulate(&c, &bars(), 0, 9);
        assert_eq!((t[0].legs[0].i, t[0].legs[0].px, t[0].dir), (3, 103.525, 1.));
        let mut wide = bars();
        wide.low[3] = 101.; // both touched on one bar: ambiguous, nothing fills; bar 4 (low 103) touches only the long
        let t = simulate(&c, &wide, 0, 9);
        assert_eq!((t[0].legs[0].i, t[0].legs[0].px, t[0].dir), (4, 104., 1.));
    }
    #[test] fn a_resting_reversal_order_flips_the_position_at_its_level() {
        let mut c = with(&[1], &[], Exec::default());
        c.other = Some(Box::new(with(&[4], &[], Exec { short: true, stop: Some(Dist::Pct(1.)), ..Exec::default() })));
        let mut b = bars();
        b.low[5] = 103.; // short stop below bar 4's close: 104.5 × 0.99 = 103.455
        let t = simulate(&c, &b, 0, 9);
        assert_eq!(fills(&t), vec![(2, 102., 5, 103.455), (5, 103.455, 9, 109.5)]);
        assert_eq!((t[0].reason.as_str(), t[1].dir, t[1].entry_kind), ("reverse to short", -1., "reverse"));
        assert!(crate::trade_validation::check(&t, 0, 9, 0.0005).is_ok());
    }

    // ---- Phase 7: trailing activation, breakeven, take-profits, min_bars, ohlc_path (spec §6.1–§6.3)
    #[test] fn trailing_arms_only_after_the_activation_level() {
        let exec = Exec { trailing: Some(Dist::Pct(2.)), trail_activate: Some(Dist::Pct(3.)), ..close_fill() };
        let mut b = bars();
        b.low[4] = 101.8; // would hit the unarmed 2% trail from bar 3's high (103.9 → 101.822), but the best (104) < 105.575
        b.high[3] = 103.9;
        let t = simulate(&with(&[2], &[], exec.clone()), &b, 0, 9);
        assert!(t[0].open);
        b.high[5] = 106.; b.low[6] = 103.8; // bar 5's high 106 ≥ 105.575 arms the trail; bar 6's level 106 × 0.98 = 103.88
        let t = simulate(&with(&[2], &[], exec), &b, 0, 9);
        assert_eq!((t[0].exit_i, t[0].reason.as_str()), (6, "exit.trailing"));
        assert!((t[0].exit_px - 103.88).abs() < 1e-9);
    }
    #[test] fn breakeven_moves_the_stop_once_the_best_price_is_far_enough() {
        let exec = Exec { stop_loss: Some(Dist::Pct(5.)), breakeven: Some((Dist::Pct(2.), Some(Dist::Points(0.5)))), ..close_fill() };
        let mut b = bars(); // entry close 102.5; after 104.55 is reached by bar 3's high 104; bar 4's high 105 arms it
        b.low[5] = 102.9; // the breakeven stop is 103 (entry + 0.5); the 5% stop (97.375) would not be touched
        let t = simulate(&with(&[2], &[], exec), &b, 0, 9);
        assert_eq!((t[0].exit_i, t[0].exit_px, t[0].reason.as_str()), (5, 103., "exit.breakeven"));
    }
    #[test] fn take_profits_scale_out_and_the_rest_exits_by_rule() {
        let exec = Exec { take_profits: vec![(Dist::Pct(1.), 0.5), (Dist::Pct(3.), 0.25)], ..close_fill() };
        // entry close 102.5: TP1 103.525 on bar 3 (high 104), TP2 105.575 on bar 5 (high 106); the rule exit closes the rest at 7's close
        let t = simulate(&with(&[2], &[7], exec), &bars(), 0, 9);
        assert_eq!(t[0].exits, vec![(3, 103.525, 0.5), (5, 105.575, 0.25), (7, 107.5, 0.25)]);
        assert!((t[0].exit_px - (103.525 * 0.5 + 105.575 * 0.25 + 107.5 * 0.25)).abs() < 1e-9);
        assert!(crate::trade_validation::check(&t, 0, 9, 0.0005).is_ok());
    }
    #[test] fn min_bars_delays_rule_exits_but_not_stops() {
        let t = simulate(&with(&[2], &[3, 4, 5], Exec { min_bars: 3, ..close_fill() }), &bars(), 0, 9);
        assert_eq!((t[0].exit_i, t[0].reason.as_str()), (5, "rule"));
        let mut b = bars();
        b.low[3] = 100.;
        let t = simulate(&with(&[2], &[], Exec { min_bars: 3, stop_loss: Some(Dist::Pct(2.)), ..close_fill() }), &b, 0, 9);
        assert_eq!((t[0].exit_i, t[0].reason.as_str()), (3, "exit.stop_loss"));
    }
    #[test] fn ohlc_path_takes_the_nearer_extreme_first() {
        let exec = Exec { stop_loss: Some(Dist::Pct(2.)), target: Some(Dist::Pct(3.)), ..close_fill() };
        let mut b = bars(); // entry close 102.5: stop 100.45, target 105.575
        b.open[3] = 105.; b.high[3] = 106.; b.low[3] = 100.; // open nearer the high: O → H → L → C, the target first
        assert_eq!(simulate(&with(&[2], &[], exec.clone()), &b, 0, 9)[0].reason, "exit.stop_loss"); // default: stop first
        let t = simulate(&with(&[2], &[], Exec { ohlc_path: true, ..exec.clone() }), &b, 0, 9);
        assert_eq!((t[0].exit_px, t[0].reason.as_str()), (105.575, "exit.target"));
        b.open[3] = 101.; // open nearer the low: O → L → H → C, the stop first in both modes
        assert_eq!(simulate(&with(&[2], &[], Exec { ohlc_path: true, ..exec }), &b, 0, 9)[0].reason, "exit.stop_loss");
    }

    // ---- Phase 7 batch 3, fix round 1
    /// Long and short stop orders (1%, valid 3 bars) from bar 2: the long fills on bar 3 (103.525); bar 4's low 101 touches
    /// the short's 101.475, which may act only under `opposite: reverse`.
    #[test] fn ignore_and_exit_never_reverse_through_a_resting_order() {
        let mk = |opposite: Opposite| {
            let mut c = with(&[2], &[], Exec { stop: Some(Dist::Pct(1.)), valid_bars: 3, opposite, ..Exec::default() });
            c.other = Some(Box::new(with(&[2], &[], Exec { short: true, stop: Some(Dist::Pct(1.)), valid_bars: 3, opposite, ..Exec::default() })));
            c
        };
        let mut b = bars();
        b.low[4] = 101.;
        let t = simulate(&mk(Opposite::Reverse), &b, 0, 9);
        assert_eq!((t.len(), t[1].legs[0].i, t[1].dir), (2, 4, -1.));
        for opposite in [Opposite::Ignore, Opposite::Exit] {
            let t = simulate(&mk(opposite), &b, 0, 9);
            assert_eq!((t.len(), t[0].dir, t[0].open, t[0].legs[0].px), (1, 1., true, 103.525), "{opposite:?}");
        }
    }
    /// Long at bar 2's open 102 (target or take-profit 1% = 103.02); short reversal stop from bar 2's close at 101.475.
    /// Bar 3 reaches both (high 104, low 101). stop_first: the adverse reversal comes first. ohlc_path: O→H→L (open 103)
    /// reaches the favourable level first, O→L→H (open 102) the reversal.
    #[test] fn an_adverse_reversal_order_comes_before_targets_and_take_profits() {
        let mk = |exec: Exec| {
            let mut c = with(&[1], &[], exec.clone());
            c.other = Some(Box::new(with(&[2], &[], Exec { short: true, stop: Some(Dist::Pct(1.)), ohlc_path: exec.ohlc_path, ..Exec::default() })));
            c
        };
        let lvl = 102. + 102. * 1. / 100.;
        let (mut hl, mut lh) = (bars(), bars());
        hl.low[3] = 101.;
        lh.low[3] = 101.; lh.open[3] = 102.;
        let target = Exec { target: Some(Dist::Pct(1.)), ..Exec::default() };
        let tp = Exec { take_profits: vec![(Dist::Pct(1.), 0.5)], ..Exec::default() };
        for (exec, b) in [(target.clone(), &hl), (tp.clone(), &hl), (Exec { ohlc_path: true, ..target.clone() }, &lh), (Exec { ohlc_path: true, ..tp.clone() }, &lh)] {
            let t = simulate(&mk(exec.clone()), b, 0, 9);
            assert_eq!(fills(&t), vec![(2, 102., 3, 101.475), (3, 101.475, 9, 109.5)], "{exec:?}");
            assert!(t[0].exits.is_empty() && t[0].reason == "reverse to short");
            assert!(crate::trade_validation::check(&t, 0, 9, 0.0005).is_ok());
        }
        // ohlc_path, O→H→L: the target closes the long at 103.02 and no short opens on that bar
        let t = simulate(&mk(Exec { ohlc_path: true, ..target }), &hl, 0, 9);
        assert_eq!((fills(&t), t[0].reason.as_str()), (vec![(2, 102., 3, lvl)], "exit.target"));
        // ohlc_path, O→H→L: the take-profit books half at 103.02, then the reversal closes the rest at 101.475
        let t = simulate(&mk(Exec { ohlc_path: true, ..tp }), &hl, 0, 9);
        assert_eq!(t[0].exits, vec![(3, lvl, 0.5), (3, 101.475, 0.5)]);
        assert_eq!((t[1].legs[0].i, t[1].legs[0].px, t[1].dir, t[1].entry_kind), (3, 101.475, -1., "reverse"));
        assert!(crate::trade_validation::check(&t, 0, 9, 0.0005).is_ok());
        // the adverse group goes by the level a move from the open (102) reaches first, in both modes (low 100):
        // a stop-loss of 1.5% (100.47) lies beyond the reversal at 101.475; one of 0.5% (101.49) comes before it
        lh.low[3] = 100.; lh.low[2] = 101.6; // the entry bar stays above both levels
        for ohlc_path in [false, true] {
            let t = simulate(&mk(Exec { stop_loss: Some(Dist::Pct(1.5)), ohlc_path, ..Exec::default() }), &lh, 0, 9);
            assert_eq!(fills(&t), vec![(2, 102., 3, 101.475), (3, 101.475, 9, 109.5)], "{ohlc_path}");
            let t = simulate(&mk(Exec { stop_loss: Some(Dist::Pct(0.5)), ohlc_path, ..Exec::default() }), &lh, 0, 9);
            assert_eq!((t.len(), t[0].exit_i, t[0].exit_px, t[0].reason.as_str()), (1, 3, 102. - 102. * 0.5 / 100., "exit.stop_loss"), "{ohlc_path}");
        }
    }
    /// N4: the open (101) gaps through the short reversal stop (101.475) but not the long's stop-loss (99.96): the
    /// reversal fills at the open, before any favourable event of the bar, in both modes.
    #[test] fn an_adverse_reversal_gapped_at_the_open_fills_there() {
        let mk = |exec: Exec| {
            let mut c = with(&[1], &[], Exec { stop_loss: Some(Dist::Pct(2.)), ..exec.clone() });
            c.other = Some(Box::new(with(&[2], &[], Exec { short: true, stop: Some(Dist::Pct(1.)), ohlc_path: exec.ohlc_path, ..Exec::default() })));
            c
        };
        let mut b = bars();
        b.open[3] = 101.; b.high[3] = 102.; b.low[3] = 99.;
        let t = simulate(&mk(Exec::default()), &b, 0, 9);
        assert_eq!((t[0].exit_i, t[0].exit_px, t[1].dir, t[1].legs[0].px), (3, 101., -1., 101.));
        // ohlc_path with the high nearer the open (102.8): the take-profit at 102.51 comes after the open's reversal
        b.high[3] = 102.8; b.high[2] = 102.5; // bar 2 (the entry bar) stays below the take-profit
        let t = simulate(&mk(Exec { ohlc_path: true, take_profits: vec![(Dist::Pct(0.5), 0.5)], ..Exec::default() }), &b, 0, 9);
        assert_eq!((t[0].exit_i, t[0].exit_px, t[0].exits.len(), t[1].dir, t[1].legs[0].px), (3, 101., 0, -1., 101.));
    }
    /// N6: delay 3, the long fills on bar 4 (stop-loss 102.96) and its exit rule on bar 5 sets an exit for bar 8. Bar 6's
    /// low 102.5 hits the stop-loss but not the short's resting stop (102.41, from bar 4); bar 7's low 102 fills it. The
    /// cleared delayed exit must not close the short on bar 8.
    #[test] fn a_price_exit_cancels_its_delayed_exit() {
        let mut c = with(&[1], &[5], Exec { delay: 3, stop_loss: Some(Dist::Pct(1.)), ..Exec::default() });
        c.other = Some(Box::new(with(&[4], &[], Exec { short: true, stop: Some(Dist::Pct(2.)), valid_bars: 5, delay: 3, ..Exec::default() })));
        let mut b = bars();
        b.low[6] = 102.5; b.low[7] = 102.;
        let t = simulate(&c, &b, 0, 9);
        assert_eq!((t[0].reason.as_str(), t[0].exit_i, t[0].exit_px), ("exit.stop_loss", 6, 104. - 104. * 1. / 100.));
        assert_eq!((t.len(), t[1].dir, t[1].open, t[1].legs[0].i, t[1].legs[0].px), (2, -1., true, 7, 104.5 - 104.5 * 2. / 100.));
    }
    /// delay 3: the long fills on bar 4, its exit rule on bar 5 sets an exit for bar 8; the short reversal stop (from bar 4,
    /// 103.455) fills on bar 6. The long's delayed exit must not close the short.
    #[test] fn a_delayed_exit_belongs_to_its_own_position() {
        let mut c = with(&[1], &[5], Exec { delay: 3, ..Exec::default() });
        c.other = Some(Box::new(with(&[4], &[], Exec { short: true, stop: Some(Dist::Pct(1.)), valid_bars: 5, delay: 3, ..Exec::default() })));
        let mut b = bars();
        b.low[6] = 103.;
        let t = simulate(&c, &b, 0, 9);
        assert_eq!(fills(&t), vec![(4, 104., 6, 103.455), (6, 103.455, 9, 109.5)]);
        assert!(t[1].open && t[1].dir == -1.);
        // delay 2: a reversal is set on bar 3 for bar 5, but the long's stop closes it on bar 4: the short is a plain entry
        let mut c = with(&[1], &[], Exec { delay: 2, stop_loss: Some(Dist::Pct(1.)), ..Exec::default() });
        c.other = Some(Box::new(with(&[3], &[], Exec { short: true, delay: 2, ..Exec::default() })));
        let mut b = bars();
        b.low[4] = 101.;
        let t = simulate(&c, &b, 0, 9);
        assert_eq!((t[0].exit_i, t[0].reason.as_str()), (4, "exit.stop_loss"));
        assert_eq!((t[1].legs[0].i, t[1].legs[0].px, t[1].dir, t[1].entry_kind), (5, 105., -1., "next_open"));
    }
    #[test] fn trailing_activation_ignores_the_current_bar() {
        let mut b = bars();
        b.high[4] = 106.; b.low[4] = 101.5; // 106 ≥ 105.575 only inside bar 4: the trail is not armed on bar 4
        let t = simulate(&with(&[2], &[], Exec { trailing: Some(Dist::Pct(2.)), trail_activate: Some(Dist::Pct(3.)), ..close_fill() }), &b, 0, 9);
        assert!(t[0].open);
    }
    #[test] fn a_limit_fill_bar_books_no_take_profit() {
        let mut b = bars();
        b.low[3] = 101.; // limit 101.475 fills on bar 3; its high 104 may precede the fill; bar 4 opens at 104 above the TP
        let t = simulate(&with(&[2], &[6], Exec { limit: Some(Dist::Pct(1.)), take_profits: vec![(Dist::Pct(1.), 0.5)], exit_fill: Fill::Close, ..Exec::default() }), &b, 0, 9);
        assert_eq!(t[0].exits[0], (4, 104., 0.5));
    }
    #[test] fn the_stop_comes_before_take_profits() {
        let mut b = bars();
        b.low[3] = 100.; // stop 100.45 and take-profit 103.525 both inside bar 3: the stop, no piece
        let t = simulate(&with(&[2], &[], Exec { stop_loss: Some(Dist::Pct(2.)), take_profits: vec![(Dist::Pct(1.), 0.5)], ..close_fill() }), &b, 0, 9);
        assert_eq!((t[0].exit_i, t[0].exit_px, t[0].exits.len()), (3, 100.45, 0));
    }
    /// One side market (long), one side orders (short stop 1%, valid 3 bars).
    #[test] fn mixed_sides_market_wins_and_conflicts_place_nothing() {
        let mk = |long_in: &[usize]| {
            let mut c = with(long_in, &[], Exec::default());
            c.other = Some(Box::new(with(&[2], &[], Exec { short: true, stop: Some(Dist::Pct(1.)), valid_bars: 3, ..Exec::default() })));
            c
        };
        let mut b = bars();
        b.low[3] = 101.; b.low[4] = 101.; // both would touch the short stop at 101.475
        // both fire on bar 2 while flat: nothing is placed
        assert!(simulate(&mk(&[2]), &b, 0, 9).is_empty());
        // the short order rests from bar 2; b.low[3] stays above it; the long's market entry on bar 3 cancels it
        b.low[3] = 102.;
        let t = simulate(&mk(&[3]), &b, 0, 9);
        assert_eq!((fills(&t), t[0].dir), (vec![(4, 104., 9, 109.5)], 1.));
    }
}
