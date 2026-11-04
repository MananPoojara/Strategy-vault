//! Multi-unit position account (lane K43, 4 Nov 2026, decision #211, engine piece EP3).
//! The single-position engine (main.rs equity_curve + trade_validation) refuses a position larger than one unit and
//! overlapping positions of one rule. The rules here need exactly that: pyramids (units added on rules), unequal lots
//! (a 200-share entry after a 100-share one), units that leave the position at different prices, and independent
//! positions that overlap (one contract per signal). Their book modules already produce the positions (`Trade`s whose
//! legs are the units); this account sizes and marks every unit on one account and lists each unit as its own trade.
//!
//! Sizing (stated once, the same for every rule):
//! * a position (one `Trade` of the module) is sized when its first unit fills, at the account value at that moment:
//!   cash after the bar's earlier exits, plus every open unit marked at the fill price (all units are on the same
//!   instrument). With one position at a time this is exactly the single-position engine's compounding;
//! * a unit's allocation = its weight x that starting value. Weights are read as the module prints them: `Qty` = the
//!   printed contract/share count relative to the position's first unit (one more contract = the same quantity again,
//!   so its money weight is count x its fill / the first fill); `Money` = a money fraction of the starting value
//!   (rcmm's unit = 1% of equity at risk over 2 ATR, which already makes every unit the same quantity);
//! * vixr_all is one position for the whole run ("one S&P for each signal ... no further leverage or increased account
//!   size", p.82): every signal adds the same quantity, sized once from the starting value, never compounded;
//! * same-bar order: the bar's exits of units held from earlier bars are settled before the bar's new positions are sized
//!   (no rule here opens a new position earlier in a bar than another position's exit on that bar: the overlapping rules
//!   enter on the close, the others hold one position at a time and reverse at the exit price);
//! * a unit's P&L = allocation x direction x (price / its fill - 1); costs = the vault's side rate on each unit's
//!   allocation at entry and at exit (0 by default); no financing, margin or borrowing cost is modelled.
//! Equity is marked at every close. If it reaches 0 the account is wiped out: open units end that day at the close and
//! later units are dropped. The detail JSON reuses the multi-position shape of `options::Account` (per-trade allocation).
use crate::connors::{Ctx, Leg, Params, Trade};

pub const MODEL: &str = "multi_unit_position_v1";
pub const BASIS: &str = "Multi-unit position account (lane K43, decision 211). Each unit of a position is listed as its own trade. A position is sized when its first unit fills, at the account value at that moment (cash plus every open unit marked at the fill price); each unit's allocation is its printed size relative to the position's first unit (contracts or shares, or the book's own risk-based unit) times that value, so a pyramided or doubled position is leveraged exactly as printed. Units are marked at every close; overlapping positions are carried together. Costs: the vault's fee and slippage settings per side on each unit's allocation (0 by default); no margin, financing or borrowing cost is modelled. If equity reaches 0 the account is wiped out and the run ends that day.";

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum W { Qty, Money }

/// The rules this account runs: (id, weight reading, the module lists one exit piece per leg in leg order, every unit of
/// the run is one position: a fixed quantity per signal, never resized).
const RULES: &[(&str, W, bool, bool)] = &[
    ("bwntd_fractal", W::Qty, false, false), ("bwtc2_fractal_sao", W::Qty, false, false),
    ("rcmm_20", W::Money, true, false), ("rcmm_55", W::Money, true, false),
    ("stad2_obv", W::Qty, false, false), ("stad2_stoch", W::Qty, false, false),
    ("stad8_stampede", W::Qty, false, false), ("stad9_luxor", W::Qty, false, false), ("stad13_luxor", W::Qty, false, false),
    ("tcdt_ut1", W::Qty, false, false), ("tcdt_ut2", W::Qty, false, false), ("tcdt_ut3", W::Qty, false, false), ("tcdt_ut4", W::Qty, false, false), ("tcdt_ut5", W::Qty, false, false),
    // Section Four p.82: "one S&P ... for each signal" and "using no further leverage or increased account size"
    ("vixr_all", W::Qty, false, true),
    // lane K44: Active Trader TSL Jun 2000 p.49, "Add the same number of shares/contracts as in the initial entry"
    ("atm00_fourplay", W::Qty, false, false),
];
pub fn is_units(id: &str) -> bool { RULES.iter().any(|r| r.0 == id) }

/// One unit: the position (group) it belongs to, its money weight of the group's starting value, and its own trade
/// (one leg, its own exit).
#[derive(Clone, Debug)]
pub struct Unit { pub g: usize, pub w: f64, pub t: Trade }

/// Split the module's positions into units. `one`: the whole run is one position (every unit the same quantity as the
/// run's first unit, sized once from the starting value). `close` tells a unit stopped on the last bar from one still open.
pub fn units(trades: &[Trade], w: W, per_leg: bool, one: bool, close: &[f64]) -> Result<Vec<Unit>, String> {
    let mut out = Vec::new();
    let first = trades.iter().filter_map(|t| t.legs.first()).min_by_key(|l| l.i).map(|l| l.px);
    for (g, t) in trades.iter().enumerate() {
        let Some(p0) = (if one { first } else { t.legs.first().map(|l| l.px) }) else { return Err("calculation_unavailable: a position has no unit".into()) };
        let g = if one { 0 } else { g };
        let n = t.legs.len();
        if !t.exits.is_empty() && !(per_leg && t.exits.len() == n) {
            return Err("calculation_unavailable: partial exits of a multi-unit position are not mapped to its units. No performance is published.".into());
        }
        for (k, l) in t.legs.iter().enumerate() {
            let mw = match w { W::Qty => l.w * l.px / p0, W::Money => l.w };
            let (exit_i, exit_px) = if t.exits.is_empty() { (t.exit_i, t.exit_px) } else { (t.exits[k].0, t.exits[k].1) };
            let last = exit_i == t.exit_i && (t.exits.is_empty() || close.get(exit_i) == Some(&exit_px));
            let reason = if n > 1 { format!("unit {} of {} · {}", k + 1, n, t.reason) } else { t.reason.clone() };
            out.push(Unit { g, w: mw, t: Trade { dir: t.dir, legs: vec![Leg { i: l.i, px: l.px, w: 1.0 }], exit_i, exit_px, reason,
                entry_kind: t.entry_kind, signal_i: if last { t.signal_i } else { exit_i }, open: t.open && last, exits: vec![] } });
        }
    }
    Ok(out)
}

/// The account over bars from..=to (see the module doc). Units outside the window are dropped.
pub fn account(close: &[f64], units: &[Unit], from: usize, to: usize, cost: f64) -> crate::options::Account {
    let mut u: Vec<&Unit> = units.iter().filter(|u| u.t.legs[0].i >= from && u.t.exit_i <= to && u.t.exit_i >= u.t.legs[0].i).collect();
    u.sort_by_key(|u| (u.t.legs[0].i, u.g));
    let run = |cost: f64| {
        let n = to - from + 1;
        let (mut eq, mut alloc, mut e0) = (vec![0.0; n], vec![0.0; u.len()], std::collections::HashMap::new());
        let pnl = |j: usize, a: f64, px: f64| a * u[j].t.dir * (px / u[j].t.legs[0].px - 1.0);
        let mut cash = 1.0;
        let mut ruin = None;
        for k in from..=to {
            // 1. exits of units entered before this bar
            for j in 0..u.len() {
                if alloc[j] > 0.0 && u[j].t.legs[0].i < k && u[j].t.exit_i == k { cash += pnl(j, alloc[j], u[j].t.exit_px) - alloc[j] * cost; }
            }
            // 2. entries (a new position is sized at the value at its first fill), then exits on the entry bar
            for j in 0..u.len() {
                let (i, px) = (u[j].t.legs[0].i, u[j].t.legs[0].px);
                if i != k { continue; }
                let v0 = *e0.entry(u[j].g).or_insert_with(|| cash + (0..u.len())
                    .filter(|&q| alloc[q] > 0.0 && u[q].t.legs[0].i < k && u[q].t.exit_i > k).map(|q| pnl(q, alloc[q], px)).sum::<f64>()
                    + (0..j).filter(|&q| alloc[q] > 0.0 && u[q].t.legs[0].i == k && u[q].t.exit_i > k).map(|q| pnl(q, alloc[q], px)).sum::<f64>());
                if v0 <= 0.0 { continue; }
                alloc[j] = u[j].w * v0;
                cash -= alloc[j] * cost;
                if u[j].t.exit_i == k { cash += pnl(j, alloc[j], u[j].t.exit_px) - alloc[j] * cost; }
            }
            let e = cash + (0..u.len()).filter(|&j| alloc[j] > 0.0 && u[j].t.legs[0].i <= k && u[j].t.exit_i > k).map(|j| pnl(j, alloc[j], close[k])).sum::<f64>();
            eq[k - from] = e;
            if e <= 0.0 { ruin = Some(k); eq[k - from..].fill(0.0); break; }
        }
        (eq, alloc, ruin)
    };
    let (equity, alloc, ruin) = run(cost);
    let (gross, _, _) = run(0.0);
    let (mut trades, mut allocs) = (vec![], vec![]);
    for (j, x) in u.iter().enumerate() {
        if alloc[j] <= 0.0 { continue; }
        let mut t = x.t.clone();
        if let Some(k) = ruin { if t.exit_i > k { t.exit_i = k; t.exit_px = close[k]; t.signal_i = k; t.open = false; t.reason = format!("{} · account wiped out", t.reason); } }
        trades.push(t);
        allocs.push(alloc[j]);
    }
    crate::options::Account { trades, alloc: allocs, equity, gross, ruin, model: MODEL, basis: BASIS }
}

/// The run of a multi-unit rule (None = not one of them).
pub fn run(id: &str, x: &Ctx, from: usize, to: usize, cost: f64, p: &Params) -> Option<Result<crate::options::Account, String>> {
    let &(_, w, per_leg, one) = RULES.iter().find(|r| r.0 == id)?;
    Some((|| {
        let trades = crate::connors::run(id, x, from, to, p).ok_or("Strategy implementation unavailable")?;
        let u = units(&trades, w, per_leg, one, &x.c)?;
        if let Some(j) = u.iter().position(|u| { let (l, t) = (&u.t.legs[0], &u.t);
            l.i < from || t.exit_i > to || t.exit_i < l.i || ![-1.0, 1.0].contains(&t.dir) || !(l.px.is_finite() && l.px > 0.0 && t.exit_px.is_finite() && t.exit_px > 0.0 && u.w.is_finite() && u.w > 0.0) }) {
            return Err(format!("calculation_unavailable: Unit{}: invalid prices, weight, direction or dates. No performance is published.", j + 1));
        }
        Ok(account(&x.c, &u, from, to, cost))
    })())
}

/// The ids are listed with their books' own reviews (stad, stad2, tcdt, rcmm); bwntd/bwtc2 and vixr keep their own entries.
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| matches!((def.book, def.id), ("stad", "stad2_obv" | "stad2_stoch") | ("stad2", "stad8_stampede" | "stad9_luxor" | "stad13_luxor")
        | ("tcdt", "tcdt_ut1" | "tcdt_ut2" | "tcdt_ut3" | "tcdt_ut4" | "tcdt_ut5") | ("rcmm", "rcmm_20" | "rcmm_55") | ("atm00", "atm00_fourplay")),
    label: "The books tested these systems on futures or stocks (Omega Research STAD 1998-2000, Crabel T-Bond upthrusts 1978-87, Market Millions, Active Trader's Fourplay on the Dow stocks 1990-2000); runs on indices, ETFs and single stocks (US and India) are a declared application. Positions hold more than one unit, sized as printed (multi-unit account).",
    basis: "Lane K43 (decision 211): the printed rules of STAD Vol 2 pp.29-48 (OBV, Stochastics: 100/200 and 200/400 shares, OBV Properties tab 'Allow multiple entries', 4 entries), Vol 8 pp.89-99 (Stampede, MaximumEntries 10), Vol 9 pp.29-38 and Vol 13 pp.75-86 (LUXOR, 3 entries); Crabel pp.261-265 (every upthrust counted, overlapping three-day shorts); Market Millions pp.36-49 (1% risk units, 4-unit 1/2 ATR matrix); Active Trader Trading System Lab Jun 2000 p.49 (Fourplay: 100 shares at the 50-day cross, 100 more at +1%, +2%, +4% ... from the entry; lane K44, check K44w/k44_ref.py). Run on the multi-unit account (books::units); independent check authors/titles-1030/K43w/k43_ref.py.",
};

#[cfg(test)]
mod tests {
    use super::*;
    fn tr(dir: f64, legs: &[(usize, f64, f64)], exit_i: usize, exit_px: f64) -> Trade {
        Trade { dir, legs: legs.iter().map(|&(i, px, w)| Leg { i, px, w }).collect(), exit_i, exit_px, reason: "x".into(), entry_kind: "stop",
            signal_i: exit_i, open: false, exits: vec![] }
    }
    fn close(v: &[f64]) -> Vec<f64> { v.to_vec() }

    /// One position at a time, one unit each: exactly the single-position engine (main.rs equity_curve), costs included.
    #[test]
    fn sequential_single_units_equal_the_single_position_engine() {
        let c = close(&[100., 102., 101., 105., 104., 103., 108., 110.]);
        let t = vec![tr(1., &[(1, 101.5, 1.)], 3, 104.), tr(-1., &[(3, 104., 1.)], 5, 102.5), tr(1., &[(6, 107., 1.)], 7, 110.)];
        for cost in [0.0, 0.001] {
            let a = account(&c, &units(&t, W::Qty, false, false, &c).unwrap(), 0, 7, cost);
            let want = crate::equity_curve(&c, &t, 0, 7, cost);
            assert!(a.equity.iter().zip(&want).all(|(x, y)| (x - y).abs() < 1e-12), "{:?} {:?}", a.equity, want);
        }
    }

    /// Hand-built pyramid: units of 1, 1 and 2 contracts at 100, 110, 120 (Qty), all out at 115.
    #[test]
    fn pyramid_units_are_sized_in_quantity_from_the_first_fill() {
        let c = close(&[100., 100., 110., 120., 115.]);
        let t = vec![tr(1., &[(1, 100., 1.), (2, 110., 1.), (3, 120., 2.)], 4, 115.)];
        let a = account(&c, &units(&t, W::Qty, false, false, &c).unwrap(), 0, 4, 0.0);
        // allocations: 1.0, 1.1, 2.4 (2 contracts at 120 vs 1 at 100); P&L at 115 = +0.15 +0.0455 -0.1 (per allocation)
        assert!(a.alloc.iter().zip([1.0, 1.1, 2.4]).all(|(x, y)| (x - y).abs() < 1e-12), "{:?}", a.alloc);
        let want = [1.0, 1.0, 1.1, 1.3, 1.1];
        assert!(a.equity.iter().zip(want).all(|(x, y)| (x - y).abs() < 1e-12), "{:?}", a.equity);
        assert_eq!(a.trades.len(), 3);
        assert!(a.trades[2].reason.starts_with("unit 3 of 3"));
        // the same position in Money weights holds 1/1/2 of the starting value instead
        let m = account(&c, &units(&t, W::Money, false, false, &c).unwrap(), 0, 4, 0.0);
        assert!(m.alloc.iter().zip([1.0, 1.0, 2.0]).all(|(x, y)| (x - y).abs() < 1e-12));
    }

    /// Hand-built overlap: a second independent position opens while the first is open; it is sized at the account value
    /// at its fill (the first marked at that price), and both are marked together.
    #[test]
    fn overlapping_positions_are_sized_at_the_value_at_their_fill_and_marked_together() {
        let c = close(&[100., 104., 106., 103., 101.]);
        let t = vec![tr(-1., &[(1, 104., 1.)], 4, 101.), tr(-1., &[(2, 106., 1.)], 3, 103.)];
        let a = account(&c, &units(&t, W::Qty, false, false, &c).unwrap(), 0, 4, 0.0);
        let v0 = 1.0 - (106. / 104. - 1.); // first short marked at 106
        assert!((a.alloc[1] - v0).abs() < 1e-12);
        let want = [1.0, 1.0, 1.0 - (106. / 104. - 1.), 1.0 - (103. / 104. - 1.) - v0 * (103. / 106. - 1.), 1.0 - (101. / 104. - 1.) - v0 * (103. / 106. - 1.)];
        assert!(a.equity.iter().zip(want).all(|(x, y)| (x - y).abs() < 1e-12), "{:?}", a.equity);
    }

    /// Units leaving at different prices (rcmm per-leg pieces), then a reversal at the price of the last exit.
    #[test]
    fn per_leg_exits_and_same_bar_reversal() {
        let c = close(&[100., 101., 103., 102., 99., 98.]);
        let mut t0 = tr(1., &[(1, 101., 0.5), (2, 102., 0.5)], 4, 100.75);
        t0.exits = vec![(4, 100., 0.5), (3, 101.5, 0.5)];
        let t1 = tr(-1., &[(4, 100., 1.)], 5, 98.);
        let u = units(&[t0, t1], W::Money, true, false, &c).unwrap();
        assert_eq!((u[1].t.exit_i, u[1].t.exit_px), (3, 101.5));
        let a = account(&c, &u, 0, 5, 0.0);
        let e2 = 1.0 + 0.5 * (103. / 101. - 1.) + 0.5 * (103. / 102. - 1.);
        let cash3 = 1.0 + 0.5 * (101.5 / 102. - 1.);
        let v0 = cash3 + 0.5 * (100. / 101. - 1.);
        let want = [1.0, 1.0, e2, cash3 + 0.5 * (102. / 101. - 1.), v0 * 1.01, v0 * 1.02];
        assert!(a.equity.iter().zip(want).all(|(x, y)| (x - y).abs() < 1e-12), "{:?} {:?}", a.equity, want);
        assert!((a.alloc[2] - v0).abs() < 1e-12);
    }

    /// Leverage can wipe the account out: the run ends that day, open units close at that close, later units are dropped.
    #[test]
    fn a_leveraged_loss_wipes_the_account_out() {
        let c = close(&[100., 100., 70., 60., 80.]);
        let t = vec![tr(1., &[(1, 100., 2.), (1, 100., 2.)], 3, 60.), tr(1., &[(4, 80., 1.)], 4, 80.)];
        let a = account(&c, &units(&t, W::Qty, false, false, &c).unwrap(), 0, 4, 0.0);
        assert_eq!(a.ruin, Some(2));
        assert_eq!(a.trades.len(), 2);
        assert!(a.trades.iter().all(|t| t.exit_i == 2 && t.exit_px == 70.));
        assert!(a.equity[2..].iter().all(|v| *v == 0.0));
    }

    /// One position for the whole run (vixr_all): every signal is the same quantity, never resized, even after a gain.
    #[test]
    fn one_position_runs_keep_a_fixed_quantity_per_signal() {
        let c = close(&[100., 110., 120., 120., 125.]);
        let t = vec![tr(1., &[(1, 110., 1.)], 3, 120.), tr(-1., &[(2, 120., 1.)], 4, 125.)];
        let a = account(&c, &units(&t, W::Qty, false, true, &c).unwrap(), 0, 4, 0.0);
        assert!((a.alloc[1] - 120. / 110.).abs() < 1e-12, "{:?}", a.alloc); // same quantity as the first signal
        let want = [1.0, 1.0, 1.0 + 10. / 110., 1.0 + 10. / 110., 1.0 + 10. / 110. - 5. / 110.];
        assert!(a.equity.iter().zip(want).all(|(x, y)| (x - y).abs() < 1e-12), "{:?}", a.equity);
    }

    #[test]
    fn unmapped_partial_exits_are_refused() {
        let mut t = tr(1., &[(1, 100., 1.), (2, 101., 1.)], 4, 102.);
        t.exits = vec![(3, 101., 0.5), (4, 103., 0.5)];
        assert!(units(&[t], W::Qty, false, false, &[100.; 5]).is_err());
    }
}
