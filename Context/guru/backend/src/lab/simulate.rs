//! Long-only v1 simulation (spec §4): signals at a bar's close fill at the next bar's open; on each bar the
//! exit fill comes before the entry fill; entries need all conditions, exits any one. Produces book-format
//! `connors::Trade`s so trade_validation, equity and stats are reused read-only.
use super::bars::Bars;
use super::series::Compiled;
use crate::connors::{Leg, Trade};

pub fn simulate(c: &Compiled, b: &Bars, from: usize, to: usize) -> Vec<Trade> {
    let mut trades = Vec::new();
    let mut held: Option<usize> = None; // fill bar of the open position
    let mut entry_signal = false;
    let mut exit_signal: Option<(usize, String)> = None;
    let trade = |e: usize, x: usize, px: f64, reason: String, signal_i: usize, open: bool| Trade {
        dir: 1., legs: vec![Leg { i: e, px: b.open[e], w: 1. }], exit_i: x, exit_px: px, reason, entry_kind: "next_open", signal_i, open, exits: vec![],
    };
    for i in from..=to {
        if let (Some((sig, reason)), Some(e)) = (exit_signal.take(), held) {
            trades.push(trade(e, i, b.open[i], reason, sig, false));
            held = None;
        }
        if std::mem::take(&mut entry_signal) { held = Some(i); }
        if i == to { break; } // a signal on the last bar has no next open inside the window
        if let Some(e) = held { exit_signal = c.exit.iter().find(|r| r.holds(i, Some(e))).map(|r| (i, r.label.clone())); }
        // A bar whose close signals an exit may also signal a new entry; both fill at the next open, exit first.
        if (held.is_none() || exit_signal.is_some()) && c.entry.iter().all(|r| r.holds(i, None)) { entry_signal = true; }
    }
    if let Some(e) = held { trades.push(trade(e, to, b.close[to], "open".into(), to, true)); }
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
        let c = Compiled { entry: vec![rule(on(&[2]))], exit: vec![held_at_least(2.)] };
        let t = simulate(&c, &bars(), 0, 9);
        assert_eq!(t.len(), 1);
        assert_eq!((t[0].legs[0].i, t[0].legs[0].px), (3, 103.)); // signal bar 2 → open of bar 3
        // bars_held is 0 on the fill bar: 2 at the close of bar 5 → exit at the open of bar 6
        assert_eq!((t[0].signal_i, t[0].exit_i, t[0].exit_px, t[0].open), (5, 6, 106., false));
        assert_eq!(t[0].reason, "bars_held >= 2");
    }
    #[test] fn exit_fills_before_a_same_bar_reentry() {
        let c = Compiled { entry: vec![rule(on(&[1, 4]))], exit: vec![rule(on(&[4]))] };
        let t = simulate(&c, &bars(), 0, 9);
        assert_eq!(t.len(), 2);
        assert_eq!((t[0].legs[0].i, t[0].exit_i), (2, 5));
        assert_eq!((t[1].legs[0].i, t[1].legs[0].px), (5, 105.)); // re-entry at the same open, after the exit
        assert!(crate::trade_validation::check(&t, 0, 9, 0.0005).is_ok());
    }
    #[test] fn entries_need_all_exits_any_and_no_pyramiding() {
        let c = Compiled { entry: vec![rule(on(&[1, 2, 3, 6])), rule(on(&[1, 3, 6]))], exit: vec![rule(on(&[7])), rule(on(&[4]))] };
        let t = simulate(&c, &bars(), 0, 9);
        assert_eq!(t.iter().map(|t| (t.legs[0].i, t.exit_i)).collect::<Vec<_>>(), vec![(2, 5), (7, 8)]);
        assert_eq!(t[0].reason, "rule");
    }
    #[test] fn open_position_at_the_end_is_marked_open() {
        let c = Compiled { entry: vec![rule(on(&[7]))], exit: vec![rule(on(&[]))] };
        let t = simulate(&c, &bars(), 0, 9);
        assert_eq!((t[0].legs[0].i, t[0].exit_i, t[0].exit_px, t[0].open, t[0].reason.as_str()), (8, 9, 109.5, true, "open"));
    }
    #[test] fn last_bar_signal_and_pre_window_signal_do_not_trade() {
        let c = Compiled { entry: vec![rule(on(&[1, 9]))], exit: vec![rule(on(&[]))] };
        assert!(simulate(&c, &bars(), 2, 9).is_empty());
    }
    #[test] fn gaps_fill_at_the_gapped_open() {
        let mut b = bars();
        b.open[6] = 90.; b.low[6] = 89.; // gap down through the prior lows
        let c = Compiled { entry: vec![rule(on(&[2]))], exit: vec![rule(on(&[5]))] };
        let t = simulate(&c, &b, 0, 9);
        assert_eq!((t[0].exit_i, t[0].exit_px), (6, 90.));
    }
}
