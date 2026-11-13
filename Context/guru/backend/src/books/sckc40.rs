//! Jay Kaeppel, "The 40-Week Cycle In The Stock Market", Technical Analysis of Stocks & Commodities V.22:1 (January 2004) pp.32-36
//! (research share Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V22/C01/006KAEP.pdf).
//! Book id `sckc40`. Lane PD (A1 pool D, 13 Nov 2026), batch-h H2 to-code row; independent check authors/pd/w/pd_ref.py.
//! R4 #326 (lane X5, review authors/rev/PD.md B1): the same rule as the live kpss_c40w (Kaeppel, Seasonal Stock Market Trends ch.6:
//! 21 Apr 1967 + 280n, and 15 May 1970 = 21 Apr 1967 + 4 x 280 days; same holiday reading), so the card is marked
//! "Duplicate of kpss_c40w" (NotTestable, no ADMISSION) and counted once; the article is cited on kpss_c40w's gate row.
//! run() is kept for the record and its test.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::atm03::{civil, day_num};
use super::tselhs::Book;

pub const REVIEWED: &[&str] = &["sckc40_bull"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const DUP: &str = "Duplicate of kpss_c40w (R4 #326): Kaeppel's 40-week cycle from S&C V.22:1 (15 May 1970 + 280n days, long for 140 calendar days, a non-trading date uses the last close before it) is the same calendar as the live card kpss_c40w (21 Apr 1967 + 280n; 15 May 1970 is 4 x 280 days later) from his book Seasonal Stock Market Trends; counted once, not run as a separate strategy";
const NOTE: &str = "Duplicate of kpss_c40w (R4 #326); not run twice. Readings kept for the record: [Declared readings] Cycle start dates = 15 May 1970 + n x 280 calendar days (n = 0, 1, 2 ...); the bullish phase ends 140 calendar days after each start. Buy at the close on each start date and sell at the close on its 140th day. When a date is not a trading day the close of the last trading day before it is used: the article's own table does this (start 1/1/88 = S&P 247.09, the 31 Dec 1987 close; mid date 12/25/92 = 439.77, the 24 Dec 1992 close), so the exchange calendar is read as known in advance. Cash between phases earns nothing here (the article's $23,339 figure credits 3% interest; its $14,215 figure does not). Long only. Article (S&P 500, 43 cycles 15 May 1970 - 2 May 2003): bullish phase up in 33 of 43 (77%), average +6.83%, $1,000 -> $14,215 with no interest; the bearish phase $1,000 -> $851. The vault's SPX history starts in 1983, so 26 of the 43 cycles can be checked; the gate row lists that check. Runs on other indices, ETFs and stocks, US and India, are a declared application of the article's calendar. Known limit of this record run() (review M3): a bar owns the calendar days up to the next bar's date, which is right for scheduled holidays and weekends (the calendar is known in advance) but would place a cycle date on the bar before a data gap, and at the window's last bar reads the next bar's date.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "sckc40_bull", name: "Kaeppel 40-week cycle: buy at the close on each 280-day cycle start from 15 May 1970, sell 140 calendar days later", group: "S&C: The 40-Week Cycle In The Stock Market (Kaeppel, V.22:1)", session: 0, dir: "Long", status: Status::NotTestable(DUP),
        needs_volume: false, needs_vix: false, needs_series: &[], book: "sckc40", rules: &[
            "F|\"Our analysis of the 40-week cycle starts on May 15, 1970, and makes the assumption that a new 40-week cycle starts on this date and a new 40-week cycle starts every 280 calendar days after that.\" (V.22:1 p.32)",
            "E|\"The simplest way to use this information is to buy an S&P 500 fund at the close on the first date of each new cycle\" (p.33)",
            "X|\"and then sell it 140 calendar days later, then earn interest until the next new cycle begins.\" (p.33); a date that is not a trading day uses the last close before it, as the article's table does (Figure 1: 1/1/88 at 247.09)"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    reviewed(id).then_some(("sckc40_bull", "Kaeppel 40-week cycle (S&C)", "Duplicate of kpss_c40w"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> { reviewed(id).then(Vec::new) }

/// Is there a day d in [lo, hi] with d - 15 May 1970 = 280 n + r for some n >= 0?
fn cycle_day(lo: i64, hi: i64, r: i64) -> bool {
    let a = civil(1970, 5, 15);
    let v = (lo - a).max(r);
    let v = v + (r - v).rem_euclid(280);
    v <= hi - a
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, _p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let dn: Vec<i64> = x.r.iter().map(|r| day_num(&r.date)).collect();
    let mut pos = Book::new();
    for i in from..=to {
        // bar i stands for every calendar day from its own date to the day before the next trading day
        let hi = if i + 1 < dn.len() { dn[i + 1] - 1 } else { dn[i] };
        if pos.side() > 0.0 && cycle_day(dn[i], hi, 140) { pos.exit(i, x.c[i], "140 calendar days"); }
        if pos.side() == 0.0 && cycle_day(dn[i], hi, 0) { pos.enter(1.0, i, x.c[i], "close"); }
    }
    Some(pos.done(to, x.c[to]))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn forty_week_cycle_dates() {
        // 26 Jul 2002 is a cycle start in the article's table; 13 Dec 2002 its 140th day
        assert!(cycle_day(day_num("2002-07-26"), day_num("2002-07-26"), 0));
        assert!(cycle_day(day_num("2002-12-13"), day_num("2002-12-13"), 140));
        assert!(!cycle_day(day_num("2002-07-27"), day_num("2002-12-12"), 0));
        // 1 Jan 1988 (holiday) belongs to the 31 Dec 1987 bar
        assert!(cycle_day(day_num("1987-12-31"), day_num("1988-01-03"), 0));
        let r = super::super::sctraub::tests::bars(3000);
        let x = Ctx::new(&r, &r, &[]);
        let t = run("sckc40_bull", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(t.len() > 5 && t.iter().all(|z| z.dir > 0.0));
        for z in t.iter().filter(|z| !z.open) {
            let d = day_num(&r[z.exit_i].date) - day_num(&r[z.legs[0].i].date);
            assert!((138..=140).contains(&d), "{d}");
        }
        assert!(family_of("sckc40_bull").is_some());
    }
}

// No ADMISSION: duplicate of kpss_c40w (R4 #326).
