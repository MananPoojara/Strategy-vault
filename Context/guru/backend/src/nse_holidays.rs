//! Derived NSE market closures (R4 #364, 19 Nov 2026): the weekdays with no NIFTY 50 daily bar between its first and last bar
//! in the engine's own price file. No holiday list is held, so closures are read from the data and nothing is edited or
//! filled. A data gap and a holiday cannot be told apart; the lead accepted that (#364). Used only by the calendar rules
//! that count trading days around "every market closure" (nfsml, stkch_preholiday, kpss_hol_d1/d2/3x3, sckcal_si2).
use crate::{connors::Ctx, Row};
use std::sync::OnceLock;
static CLOSURES: OnceLock<Vec<i64>> = OnceLock::new();
fn dnum(d: &str) -> i64 { let (y, m, dd) = crate::books::mzwws::ymd(d); crate::books::mzwws::dnum(y, m, dd) }
/// Weekday day numbers strictly between the first and last bar that have no bar.
pub fn derive(rows: &[Row]) -> Vec<i64> {
    let dn: Vec<i64> = rows.iter().map(|r| dnum(&r.date)).collect();
    let mut out = vec![];
    for w in dn.windows(2) { for g in w[0] + 1..w[1] { if crate::books::mzwws::wday(g) < 5 { out.push(g); } } }
    out
}
/// Called once at start-up with the loaded NIFTY 50 rows.
pub fn set_from(rows: Option<&Vec<Row>>) { if let Some(r) = rows { let _ = CLOSURES.set(derive(r)); } }
pub fn closures() -> &'static [i64] { CLOSURES.get().map_or(&[], |v| v.as_slice()) }
/// An Indian context: a native Indian stock or an Indian index.
pub fn is_india(x: &Ctx) -> bool { x.native_market == Some(crate::baskets::Market::India) || x.r.first().is_some_and(|r| r.ticker.starts_with("NIFTY")) }
/// Test hook: closures for synthetic Indian contexts.
#[cfg(test)]
pub fn set_for_test(v: Vec<i64>) { let _ = CLOSURES.set(v); }
