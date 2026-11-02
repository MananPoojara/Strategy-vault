//! David K. Moy, "OEX Options Expiration Trading Patterns", Technical Analysis of Stocks & Commodities V.13:10 (October 1995)
//! pp.438-442 (research share Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V13/C10/
//! OEXOPT.pdf). Book id `scmoy`. Lane K13 (titles loop, 2 Nov 2026), to-code row 221; independent check
//! authors/titles-1030/K13w/k13_ref.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::sctraub::{civil, days, period_ends, weekday};
use super::tselhs::Book;

pub const REVIEWED: &[&str] = &["scmoy_oex"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] Options expiration week = the calendar week (Monday-Friday) holding the month's third Friday (the Friday falls on day 15-21; a holiday Friday still marks the week); the post-expiration week is the next week. Long through expiration and non-expiration weeks, short through the post-expiration week; every switch at the close of the last trading day of the week before ('All transactions would be at market close the last day of the preceding week'). No risk control, as the article says. The S&P 100 (OEX) is not held: SPX and the usual instruments, US and India (India's own expiries are not used), are declared applications. Article (OEX, 17 January 1986 - 12 May 1995, before costs): buy and hold +298.60 points, this strategy +657.84 points.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "scmoy_oex", name: "Moy options-expiration cycle: long the market except short through the week after monthly options expiration", group: "S&C: OEX Options Expiration Trading Patterns (Moy, V.13:10)", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "scmoy", rules: &[
            "F|\"These contracts cease trading on the third Friday of each month.\" (V.13:10 p.438)",
            "E|\"The simple aggressive weekly trading strategy would be to buy the options expiration week and sell the post-expiration week; the short would be converted into a long position during the nonexpiration weeks.\" (p.441)",
            "X|\"All transactions would be at market close the last day of the preceding week.\" (p.441)"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let id = REVIEWED.iter().find(|r| **r == id)?;
    Some((*id, "Moy options expiration cycle (S&C)", "Short the post-expiration week"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> { reviewed(id).then(Vec::new) }

/// Day of month of the Friday of the date's calendar week.
fn friday_day(date: &str) -> i64 { civil(days(date) - weekday(date) + 4).2 }

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, _p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let mut pos = Book::new();
    for i in period_ends(x, false) {
        if i < from || i > to { continue; }
        let f = friday_day(&x.r[i].date);
        let d = if (15..=21).contains(&f) { -1.0 } else { 1.0 };
        pos.enter(d, i, x.c[i], "close");
    }
    Some(pos.done(to, x.c[to]))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn expiration_week_short() {
        assert_eq!(friday_day("2026-11-16"), 20);
        assert_eq!(friday_day("2026-10-28"), 30);
        assert_eq!(friday_day("2026-12-29"), 1);
        let r = super::super::sctraub::tests::bars(1500);
        let x = Ctx::new(&r, &r, &[]);
        let t = run("scmoy_oex", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(t.iter().filter(|z| z.dir < 0.0).all(|z| z.exit_i - z.legs[0].i == 5));
    }
}

// Instrument admission: S&C Moy 2026-11-02 (titles lane K13).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "scmoy" && reviewed(def.id),
    label: "David Moy tested the weekly options-expiration pattern on the S&P 100 (OEX, not held, 1986-1995). Runs on SPX and other indices, ETFs and stocks, US and India, are a declared application.",
    basis: "Technical Analysis of Stocks & Commodities V.13:10 pp.438-442 (OEX Options Expiration Trading Patterns); independent check authors/titles-1030/K13w/k13_ref.py",
};
