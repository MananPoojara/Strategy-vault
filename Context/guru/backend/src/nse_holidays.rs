//! Derived NSE market closures (R4 #364, 19 Nov 2026): the weekdays with no NIFTY 50 daily bar between its first and last bar
//! in the engine's own price file. No holiday list is held, so closures are read from the data and nothing is edited or
//! filled. A data gap and a holiday cannot be told apart; runs of more than 2 consecutive weekday closures are dropped as gaps. Used only by the calendar rules
//! that count trading days around "every market closure" (nfsml, stkch_preholiday, kpss_hol_d1/d2/3x3, sckcal_si2).
use crate::{connors::Ctx, Row};
use std::sync::OnceLock;
static CLOSURES: OnceLock<Vec<i64>> = OnceLock::new();
fn dnum(d: &str) -> i64 { let (y, m, dd) = crate::books::mzwws::ymd(d); crate::books::mzwws::dnum(y, m, dd) }
/// Weekday day numbers strictly between the first and last bar that have no bar. A run of MORE THAN TWO consecutive weekday
/// closures (a weekend between two weekdays still counts as consecutive) is a data gap, not holidays (lead rule, R4 #364): the
/// whole run is left out. The excluded runs and the unusual single/pair closures are listed in R1/nse-holidays-flagged.tsv.
pub fn derive(rows: &[Row]) -> Vec<i64> {
    let dn: Vec<i64> = rows.iter().map(|r| dnum(&r.date)).collect();
    let next = |g: i64| if crate::books::mzwws::wday(g) == 4 { g + 3 } else { g + 1 };
    let mut all = vec![];
    for w in dn.windows(2) { for g in w[0] + 1..w[1] { if crate::books::mzwws::wday(g) < 5 { all.push(g); } } }
    let (mut out, mut run): (Vec<i64>, Vec<i64>) = (vec![], vec![]);
    for g in all {
        if run.last().is_some_and(|&l| next(l) != g) { if run.len() <= 2 { out.extend(&run); } run.clear(); }
        run.push(g);
    }
    if run.len() <= 2 { out.extend(&run); }
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
