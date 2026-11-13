//! Scott Barrie, "Quasi-Seasonal Tendencies in Bond Futures", Technical Analysis of Stocks & Commodities V.14:1 (1996) pp.19-25
//! (research share Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V14/C01/QUASISE.pdf).
//! Book id `scsbqs`. Lane PA2 (13 Nov 2026); independent check authors/pa2/w/pa2_fut_ref.py (written from the article).
//! Three printed tests on CBOT T-bond futures 1979-1994, run on the Norgate T-bond continuous contract only (R4 #293 (1);
//! futures::CARDS_PA2): turnaround Tuesday (Figure 4), the month-end windows (Figure 7) and the post-April-15 short (Figure 10).
//! Policy: futures::CARDS_PA2 (the named contract), like lane F3's T-bond cards; no ADMISSION entry.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::sctraub::days;
use super::tselhs::Book;

pub const REVIEWED: &[&str] = &["scsbqs_tuesday", "scsbqs_month_end", "scsbqs_april15"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

macro_rules! fut { () => { " Runs only on the Norgate T-bond continuous contract (the article's 'CBOT long bond futures contract from 1979 to 1994'): signals and point P&L on the back-adjusted series, each trade priced in the contract held at entry (R4 #293, #301). Norgate continuous futures, ends 2020-09-11; roll method not stated by the vendor. No stops, slippage or commissions, as in the article." } }
const NOTE_TUE: &str = concat!("[Adapted: 1 unstated detail, R4 #297; VERIFIED on Figure 4, R4 #168] 'If Monday's close were greater than the previous Friday's settlement, a sell order was executed as market on close (MOC) on Monday and the short was covered MOC on Tuesday's close ... The reverse was tested for long trades, with the trigger being a lower settlement on Monday.' Unstated: holiday weeks - vault reading: Monday's close against the previous session's close (Thursday when Friday was a holiday), covered at the next session's close (Wednesday when Tuesday is a holiday); a week without a Monday session has no trade; an unchanged Monday has no trade. Option side = 1 / -1 trades only the long / short half (Figure 4 prints both halves).", fut!());
const NOTE_ME: &str = concat!("[Adapted: 1 unstated detail, R4 #297; VERIFIED on Figure 7, R4 #168] Figure 7, 'best results of all 10-day periods centered on the end of month', traded from the close of the first day to the close of the second: January -5 to +5 short, February L to +5 short, May +2 to +4 long, July -3 to -2 long, August -4 to +2 long, September L to +3 long, October L to +5 long, November -5 to F short, December -4 to +5 short; March, April and June 'No significant bias' (no trade). L = the month's last trading day, F = the first trading day of the next month (+1), +k = k trading days after L. Unstated (the article gives two definitions of '-k'): 'negative numbers represent days before the last trading day of the month' or '(that is, -5 is five trading days before the first day of the next month)'. Vault reading, decided on the article's own table (R4 #297 (3), lead decision): k trading days before F, i.e. -1 = L (default neg = 1): with it the 1979-1994 win rates of Figure 7 reproduce in 6 of 9 months exactly and within one trade in the other 3 (January, February, May, August, September, November equal; July, October, December one trade off), against 4 of 9 for the other reading (option neg = 0: k trading days before L, which makes -5 to +5 the title's 10-day period). The table was picked as the best of all windows on the same 1979-1994 data (an in-sample choice by the author). Option month = 1-12 trades one month's window only (0 = all nine).", fut!());
const NOTE_APR: &str = concat!("[Adapted: 1 unstated detail, R4 #297; VERIFIED on Figure 10, R4 #168] 'selling June bond futures two trading days after April 15 and covering the short four trading days after that day. If this strategy were to be followed every year from 1979 to 1994, 4 1/8 points would have been the profit, with the average trade netting 0-10 ticks and making a profit 81% of the time.' Entry = the second trading day after April 15 (the second session dated after April 15), exit = four trading days later. Unstated: the time of day - vault reading: both at the close (the article's other tests trade market-on-close). The continuous contract holds the June bond from the March roll, as the article's trade does.", fut!());

fn card(id: &'static str, name: &'static str, group: &'static str, dir: &'static str, rules: &'static [&'static str], notes: &'static str) -> StrategyDef {
    StrategyDef { id, name, group, session: 0, dir, status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: "scsbqs", rules, notes, claim: None }
}

pub fn catalogue() -> Vec<StrategyDef> {
    const G: &str = "S&C: quasi-seasonal tendencies in bond futures (Barrie, V.14:1)";
    vec![
        card("scsbqs_tuesday", "Barrie turnaround Tuesday (T-bond futures): fade Monday's close-to-close move from Monday's close to Tuesday's close", G, "Long/Short", &[
            "E|\"If Monday's close were greater than the previous Friday's settlement, a sell order was executed as market on close (MOC) on Monday and the short was covered MOC on Tuesday's close. No stops were used, and neither slippage nor commissions were applied. The reverse was tested for long trades, with the trigger being a lower settlement on Monday.\" (V.14:1, PDF p.3; Figure 4)",
            "X|\"the short was covered MOC on Tuesday's close\" (PDF p.3): out at the next session's close",
            "F|Figure 4 'Results of trading turnaround Tuesday' (CBOT long bond futures 1979-1994): all trades 867, wins 489 (56.4%), losers 348, scratch 3.5%, sum P&L 66-29; buys 446 trades, 266 wins (59.6%); sells 421 trades, 223 wins (52.97%)"], concat_note(0)),
        card("scsbqs_month_end", "Barrie month-end windows (T-bond futures): the nine printed 10-day periods around the month end, long or short close to close", G, "Long/Short", &[
            "F|\"I examined the front-contract bond future for the last five days of a particular month continuing through the first five days of the following month ... Figure 7 presents the best results of all 10-day periods centered on the end of month listed.\" (PDF p.3)",
            "F|\"negative numbers represent days before the last trading day of the month (that is, -5 is five trading days before the first day of the next month), L represents the last trading day of the month, F signifies the first trading day of the following month and positive numbers represent the numbers of days past the last trading of the month\" (PDF p.3-4)",
            "E|\"Trade results are based on entry on the close (the first number presented under time) and exit on the close (the second number presented under time) with no commission or slippage applied. An example would be January: enter five days prior to the end of January and exit five days into February.\" (PDF p.4)",
            "X|\"exit on the close (the second number presented under time)\" (PDF p.4): out at the close of the window's last day",
            "F|Figure 7 (PDF p.4): January -5 to +5 Short +7-13 62.5%; February L to +5 Short +5-27 62.5; March, April, June no significant bias; May +2 to +4 Long +3-08 62.5; July -3 to -2 Long +1-19 56.3; August -4 to +2 Long +3-03 50.0; September L to +3 Long +4-28 56.3; October L to +5 Long +5-04 56.3; November -5 to F Short +3-09 68.8; December -4 to +5 Short +6-31 75.0"], concat_note(1)),
        card("scsbqs_april15", "Barrie tax-day short (T-bond futures): sell two trading days after April 15, cover four trading days later, at the close", G, "Short", &[
            "E|\"Figure 10 presents the results of selling June bond futures two trading days after April 15 and covering the short four trading days after that day.\" (PDF p.4)",
            "X|\"covering the short four trading days after that day\" (PDF p.4): cover at the close four trading days after entry",
            "F|\"If this strategy were to be followed every year from 1979 to 1994, 4 1/8 points would have been the profit, with the average trade netting 0-10 ticks and making a profit 81% of the time.\" (PDF p.4)"], concat_note(2)),
    ]
}
fn concat_note(k: usize) -> &'static str { [NOTE_TUE, NOTE_ME, NOTE_APR][k] }

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    // three printed rules (Figures 4, 7, 10), three families (review R4 #322 M1)
    match id {
        "scsbqs_tuesday" => Some(("scsbqs_tuesday", "Barrie T-bond turnaround Tuesday (S&C)", "Fade Monday")),
        "scsbqs_month_end" => Some(("scsbqs_month_end", "Barrie T-bond month-end windows (S&C)", "Figure 7")),
        "scsbqs_april15" => Some(("scsbqs_april15", "Barrie T-bond tax-day short (S&C)", "April 15 + 2")),
        _ => None,
    }
}
pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    match id {
        "scsbqs_tuesday" => Some(vec![p("side", "0 = both halves (article); 1 = long half only; -1 = short half only", 0., -1., 1., 1.)]),
        "scsbqs_month_end" => Some(vec![p("neg", "'-k' = k days before the next month's first day (1, vault reading: matches Figure 7) or before the last trading day (0)", 1., 0., 1., 1.),
            p("month", "0 = all nine printed months; 1-12 = that month's window only", 0., 0., 12., 1.)]),
        "scsbqs_april15" => Some(vec![]),
        _ => None,
    }
}

/// (month, start offset, end offset, direction) from Figure 7; offsets count trading days from the month's last trading day L.
pub const WINDOWS: &[(u32, i64, i64, f64)] = &[(1, -5, 5, -1.), (2, 0, 5, -1.), (5, 2, 4, 1.), (7, -3, -2, 1.), (8, -4, 2, 1.), (9, 0, 3, 1.), (10, 0, 5, 1.), (11, -5, 1, -1.), (12, -4, 5, -1.)];

fn month(x: &Ctx, i: usize) -> (i64, u32) { let d = &x.r[i].date; (d[0..4].parse().unwrap_or(0), d[5..7].parse().unwrap_or(0)) }

/// One position from bar e to bar q (q may lie past `to`: the trade is then open at `to`).
fn put(pos: &mut Book, x: &Ctx, d: f64, e: usize, q: usize, to: usize, why: &str) -> bool {
    pos.enter(d, e, x.c[e], "close");
    if q <= to { pos.exit(q, x.c[q], why); true } else { false }
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from > to || to >= x.r.len() { return None; }
    let g = |k: &str, d: f64| p.get(k).copied().unwrap_or(d);
    let n = x.r.len();
    let mut pos = Book::new();
    match id {
        "scsbqs_tuesday" => {
            let side = g("side", 0.).round();
            for i in from.max(1)..to {
                if (days(&x.r[i].date) + 3).rem_euclid(7) != 0 { continue; } // Monday
                let d = if x.c[i] > x.c[i - 1] { -1. } else if x.c[i] < x.c[i - 1] { 1. } else { 0. };
                if d == 0. || (side != 0. && d != side) { continue; }
                put(&mut pos, x, d, i, i + 1, to, "Tuesday's close");
            }
        }
        "scsbqs_month_end" => {
            let shift = if g("neg", 1.) >= 0.5 { 1 } else { 0 };
            let only = g("month", 0.).round() as u32;
            let mut busy = 0usize;
            for l in 0..n - 1 {
                if month(x, l + 1).1 == month(x, l).1 { continue; } // l = the month's last trading day
                let m = month(x, l).1;
                let Some(&(_, a, b, d)) = WINDOWS.iter().find(|w| w.0 == m && (only == 0 || only == m)) else { continue };
                let off = |k: i64| if k < 0 { k + shift } else { k };
                let (e, q) = (l as i64 + off(a), l as i64 + off(b));
                if e < 0 || q >= n as i64 || q <= e { continue; }
                let (e, q) = (e as usize, q as usize);
                if e < from || e >= to || e < busy { continue; }
                if !put(&mut pos, x, d, e, q, to, "window end") { break; }
                busy = q;
            }
        }
        _ => {
            for i in 1..n {
                let (y, m) = month(x, i);
                let dd: u32 = x.r[i].date[8..10].parse().unwrap_or(0);
                // i = the first session dated after April 15
                let after = |k: usize| { let (yy, mm) = month(x, k); let d2: u32 = x.r[k].date[8..10].parse().unwrap_or(0); yy == y && (mm > 4 || (mm == 4 && d2 > 15)) };
                if !(after(i) && !(month(x, i - 1).0 == y && after(i - 1))) || m > 5 || (m == 5 && dd > 15) { continue; }
                let (e, q) = (i + 1, i + 5);
                if q >= n || e < from || e >= to { continue; }
                if !put(&mut pos, x, -1., e, q, to, "four trading days later") { break; }
            }
        }
    }
    Some(pos.done(to, x.c[to]))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tuesday_month_end_and_april_calendar() {
        let r = super::super::sctraub::tests::bars(2600);
        let x = Ctx::new(&r, &r, &[]);
        let wd = |i: usize| (days(&r[i].date) + 3).rem_euclid(7);
        let t = run("scsbqs_tuesday", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(t.len() > 300 && t.iter().all(|z| wd(z.legs[0].i) == 0 && z.exit_i == z.legs[0].i + 1));
        assert!(t.iter().all(|z| (z.dir < 0.) == (x.c[z.legs[0].i] > x.c[z.legs[0].i - 1])));
        let m = run("scsbqs_month_end", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert_eq!(m.iter().filter(|z| &r[z.legs[0].i].date[5..7] == "03").count(), 0);
        assert!(m.len() >= 9 * 9);
        let a = run("scsbqs_april15", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert_eq!(a.len(), 10);
        for z in &a { assert!(&r[z.legs[0].i].date[5..7] == "04" && z.dir < 0. && z.exit_i == z.legs[0].i + 4); }
    }
}
