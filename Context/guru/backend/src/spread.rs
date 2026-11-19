//! Two-leg spread positions (lane P1, 13 Nov 2026; R4 #182 (2), #263, #293, #297).
//!
//! A spread position holds one leg long and the other short in stated quantities; both legs are entered on the same bar
//! and exited on the same bar. The legs are joined by exact date on the run's calendar: a bar where either leg has no
//! price of its own gives no signal and no fill (nothing is carried forward). P&L = the sum of the two legs' money P&L;
//! the trade return is that P&L as a share of the GROSS exposure at entry (long notional + short notional, each leg at its
//! unadjusted entry price). Costs: the vault fee and slippage setting per side on that gross exposure, i.e. on each leg's
//! own notional. The account is options::account (one allocation per position, valued every day): with one position at a
//! time it compounds the trade returns like the single-position engine. A bar inside a position where a leg has no price
//! keeps the last joint mark in the equity line (a mark, never a fill).
use crate::options::{Account, Pos};

/// One leg on the run's calendar. `qty` = units held when the spread is LONG (signed: + long, - short); `px` = the leg's
/// prices (NaN where it has no bar on that date; back-adjusted for continuous futures); `base` = the price its exposure is
/// measured at (the unadjusted price; the same series as `px` for stocks); `mult` = money per 1.00 of the quote.
pub struct Leg<'a> { pub qty: f64, pub px: &'a [f64], pub base: &'a [f64], pub mult: f64 }

/// The bars where every series has its own finite value (exact-date join on the run's calendar).
pub fn joint(series: &[&[f64]]) -> Vec<usize> {
    let n = series.iter().map(|s| s.len()).min().unwrap_or(0);
    (0..n).filter(|&i| series.iter().all(|s| s[i].is_finite())).collect()
}

/// Population mean and z-score of the last `n` values ending at `m` (z = (v[m] - mean) / sd, sd over n); None before n
/// values or when sd = 0.
pub fn zscore(v: &[f64], m: usize, n: usize) -> Option<f64> {
    if n == 0 || m + 1 < n { return None; }
    let w = &v[m + 1 - n..=m];
    let mean = w.iter().sum::<f64>() / n as f64;
    let sd = (w.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / n as f64).sqrt();
    (sd > 0. && sd.is_finite()).then(|| (v[m] - mean) / sd)
}

/// A spread position: `dir` +1 = long the spread (each leg as signed by its qty), -1 = short. Entered at the close of bar
/// `e`, exited at the close of bar `x` (both must be joint bars). None if the entry has no positive gross exposure.
pub fn position(legs: &[Leg], dir: f64, e: usize, x: usize, reason: String, open: bool) -> Option<Pos> {
    if x < e || legs.iter().any(|l| !(l.px[e].is_finite() && l.px[x].is_finite())) { return None; }
    let s0: f64 = legs.iter().map(|l| l.qty.abs() * l.mult * l.base[e]).sum();
    if !(s0 > 0. && s0.is_finite()) { return None; }
    let mut last = 0.;
    let pnl = (e..=x).map(|k| {
        if legs.iter().all(|l| l.px[k].is_finite()) { last = legs.iter().map(|l| dir * l.qty * l.mult * (l.px[k] - l.px[e])).sum(); }
        last
    }).collect();
    Some(Pos { dir, s0, entry_i: e, exit_i: x, pnl, reason, open })
}

/// A spread position whose legs are reset to fixed money amounts at every joint close (daily rebalanced, as Chan's
/// `pnl = sum(lag1(positions) .* dailyret)`): `legs` = (prices, money in the leg when the spread is long, signed). A day's P&L is
/// the sum of dir x money x (px[k] / px[previous joint bar] - 1) over the legs, and the trade's P&L is the plain sum of its days
/// (the money in each leg stays the same, it is not compounded). s0 = gross money (sum of |money|).
pub fn rebalanced(legs: &[(&[f64], f64)], dir: f64, e: usize, x: usize, reason: String, open: bool) -> Option<Pos> {
    let ok = |k: usize| legs.iter().all(|l| l.0[k].is_finite() && l.0[k] > 0.);
    if x < e || !ok(e) || !ok(x) { return None; }
    let s0: f64 = legs.iter().map(|l| l.1.abs()).sum();
    if !(s0 > 0. && s0.is_finite()) { return None; }
    let (mut prev, mut cum) = (e, 0.);
    let pnl = (e..=x).map(|k| {
        if k > e && ok(k) { cum += legs.iter().map(|l| dir * l.1 * (l.0[k] / l.0[prev] - 1.)).sum::<f64>(); prev = k; }
        cum
    }).collect();
    Some(Pos { dir, s0, entry_i: e, exit_i: x, pnl, reason, open })
}

pub const MODEL: &str = "spread_two_leg_v1";
pub const BASIS: &str = "Two-leg spread account (lane P1, R4 #293, #297). Each position is long one leg and short the other in the rule's stated quantities; both legs are filled at the same bar's close and closed together. The two price series are joined by exact date: a date either leg lacks gives no signal and no fill, and no price is carried forward. Trade return = the sum of the two legs' money P&L divided by the gross exposure at entry (long notional + short notional at the unadjusted entry prices). Positions are valued every day; a day on which a leg has no bar keeps the last joint value (a mark, never a fill). Costs: the vault fee and slippage setting per side on each leg's notional (0 by default); no borrow fee or margin interest is modelled.";

/// Every rule that runs on this account (books::vanvliet, books::ecqt).
pub fn is_rule(id: &str) -> bool { crate::books::vanvliet::is_spread(id) || id == crate::books::ecqt::PAIR }
/// The account of a spread rule (None = not a spread rule; Err = not on this context or data missing).
pub fn run(id: &str, x: &crate::connors::Ctx, from: usize, to: usize, cost: f64, p: &crate::connors::Params) -> Option<Result<Account, String>> {
    let pos = crate::books::vanvliet::positions(id, x, from, to, p).or_else(|| crate::books::ecqt::positions(id, x, from, to, p))?;
    Some(pos.map(|pos| account(pos, from, to, cost)))
}
/// The run's account: options::account with this model's name and basis.
pub fn account(pos: Vec<Pos>, from: usize, to: usize, cost: f64) -> Account {
    let mut a = crate::options::account(pos, from, to, cost);
    a.model = MODEL;
    a.basis = BASIS;
    a
}

#[cfg(test)]
mod tests {
    use super::*;
    const N: f64 = f64::NAN;
    #[test]
    fn exact_date_join_skips_a_bar_either_leg_lacks() {
        assert_eq!(joint(&[&[1., N, 3., 4.], &[1., 2., N, 4.]]), vec![0, 3]);
    }
    #[test]
    fn population_zscore_over_the_window() {
        let v = [1., 2., 3., 4., 10.];
        assert_eq!(zscore(&v, 1, 3), None);
        let w = [2., 3., 4., 10.]; // last 4 ending at 4
        let mean = 19. / 4.;
        let sd = (w.iter().map(|x| (x - mean) * (x - mean)).sum::<f64>() / 4.).sqrt();
        assert!((zscore(&v, 4, 4).unwrap() - (10. - mean) / sd).abs() < 1e-12);
        assert_eq!(zscore(&[5., 5., 5.], 2, 3), None);
    }
    #[test]
    fn pnl_is_the_sum_of_the_legs_on_gross_exposure_with_marks_held_over_gaps() {
        // long spread = short 1 A (x50), long 2 B (x20): A 100 -> 102, B 50 -> 52 (B missing on bar 1)
        let (a, b) = ([100., 101., 102.], [50., N, 52.]);
        let legs = [Leg { qty: -1., px: &a, base: &a, mult: 50. }, Leg { qty: 2., px: &b, base: &b, mult: 20. }];
        let p = position(&legs, 1., 0, 2, "x".into(), false).unwrap();
        assert_eq!(p.s0, 100. * 50. + 2. * 50. * 20.);
        assert_eq!(p.pnl, vec![0., 0., -2. * 50. + 2. * 2. * 20.]);
        let s = position(&legs, -1., 0, 2, "x".into(), false).unwrap();
        assert_eq!(s.pnl[2], 2. * 50. - 2. * 2. * 20.);
        assert!(position(&legs, 1., 0, 1, "x".into(), false).is_none()); // exit on a bar the leg lacks: refused
        // the account compounds the return and charges the cost per side on the gross exposure
        let acc = account(vec![p], 0, 2, 0.001);
        assert!((acc.trades[0].ret(0.) + 20. / 7000.).abs() < 1e-12);
        assert!((acc.equity[2] - (1. - 20. / 7000. - 0.002)).abs() < 1e-12);
        assert_eq!(acc.model, MODEL);
    }
    #[test]
    fn rebalanced_legs_sum_daily_returns_over_joint_bars() {
        // +1 in A, -1 in B (long): day returns A +10% then -5%, B 0% then +10% (B missing on bar 1: one 2-bar return)
        let (a, b) = ([100., 110., 104.5], [50., N, 55.]);
        let p = rebalanced(&[(&a, 1.), (&b, -1.)], 1., 0, 2, "x".into(), false).unwrap();
        assert_eq!(p.s0, 2.);
        assert!((p.pnl[1] - 0.).abs() < 1e-12); // a leg has no bar: the mark is kept
        assert!((p.pnl[2] - ((104.5 / 100. - 1.) - (55. / 50. - 1.))).abs() < 1e-12);
    }
}
