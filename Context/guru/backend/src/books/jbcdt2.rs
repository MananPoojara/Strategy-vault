//! Jake Bernstein, "The Compleat Day Trader II" (McGraw-Hill, 1998). Book id `jbcdt2`.
//! Source: a 260-page scan, read as page images (printed page = PDF page - 17). Transcription and checks: library/jbcdt2/.
//! Most methods run on 5- to 30-minute bars; the daily-bar rules are the DBO day-of-week breakouts (ch.5), the
//! high-probability calendar dates and pre-holiday day trades (ch.5) and the Monday pattern (ch.5).
//! Also holds the shared DBO / weekday / US-holiday helpers used by `jbeft`.
use super::jb624::{bar_path, finish, getp, mark_open, new_trade, stop_fill, week_changes};
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "jbcdt2";

macro_rules! def {
    ($id:expr, $name:expr, $group:expr, $dir:expr, $status:expr, $rules:expr, $notes:expr) => {
        StrategyDef { id: $id, name: $name, group: $group, session: 0, dir: $dir, status: $status, needs_volume: false,
            needs_vix: false, needs_series: &[], book: BOOK, rules: $rules, notes: $notes, claim: None }
    };
}

/// S&P dates of Figure 5-6 (p.70): "High probability of close higher than open".
pub const SP_DATES: [(u32, u32); 11] = [(1, 5), (1, 14), (2, 2), (4, 12), (4, 16), (5, 19), (6, 1), (7, 14), (8, 27), (11, 24), (12, 13)];

pub fn catalogue() -> Vec<StrategyDef> {
    use Status::{Live, NeedsData, NotTestable};
    vec![
        StrategyDef { claim: crate::connors::claim("397", "77%", "$409.32 (1 S&P contract)", "2 bars (winners)", "S&P futures 4/21/82-1/09/98, PF 1.61"),
          ..def!("jbcdt2_dbo_mon", "DBO Monday · 3 ticks above / 26 ticks below Friday, $3,400 stop, FPO", "Ch.5 · Day of the Week Patterns", "Long/Short", Live,
            &["F|\"Trade only on the indicated day or week\"; \"the DBO history on Mondays (or the first trading day of the week)\" (p.73)",
              "E|\"Buy on a stop x ticks above the previous daily high if previous close was greater than previous open\" (p.73)",
              "E|\"Sell short on a stop x ticks below the previous daily low if previous close was less than previous open\" (p.73)",
              "E|Figure 5-8 (p.73): \"Long entry, three ticks, above Friday's high; short entry, 26 ticks, below Friday's low\" = 0.15 / 1.30 S&P points (0.05-point tick of the $500 contract)",
              "X|\"Exit on a predetermined risk management stop loss\"; Figure 5-8: \"Stop loss $3400\" = 6.8 S&P points at $500 per point (p.73-74)",
              "X|\"Exit on the first profitable opening (FPO)\" (p.74)"],
            "Same rule as EFT ch.8 (1982-97 inputs, p.127) and as jb624_fri_mon with other inputs. Points kept as printed on every index (instrument swap only). Week boundary: the next bar falls in a later calendar week. Entry-day stop counted only if the bar path (open, nearer extreme, other extreme, close) reaches it after the entry (the book's own caveat, p.74); p.entry_day=1 counts any touch. One position at a time; FPO = any later opening better than the entry price.") },
        StrategyDef { claim: crate::connors::claim("395", "78%", "$363.67 (1 S&P contract)", "2 bars (winners)", "S&P futures 4/21/82-1/09/98, PF 1.56"),
          ..def!("jbcdt2_dbo_thu", "DBO Thursday · 6 ticks above / 21 ticks below Wednesday, $3,500 stop, FPO", "Ch.5 · Day of the Week Patterns", "Long/Short", Live,
            &["F|Figure 5-9 (p.74): \"DBO signals on Thursdays\": setup = Wednesday's close versus Wednesday's open (p.73 rules)",
              "E|\"Long entry six ticks above Wednesday's high; short entry 21 ticks below Wednesday's low\" (p.74) = 0.30 / 1.05 S&P points",
              "X|\"Stop loss $3500. Exit on first profitable opening\" (p.74) = 7.0 S&P points at $500 per point"],
            "Needs a Wednesday bar followed by a Thursday bar (a holiday on either day = no trade). Same fills and FPO as jbcdt2_dbo_mon.") },
        def!("jbcdt2_dbo_wed", "DBO Wednesday (referenced to Tuesday's range)", "Ch.5 · Day of the Week Patterns", "Long/Short", NotTestable("the penetration ticks and stop for the Wednesday test (Figure 5-10, p.75) are not given"),
            &["E|\"using the DBO method on Wednesdays (i.e., referenced to Tuesday's range) is a losing proposition in spite of its 68 percent accuracy\" (Figure 5-10, p.75)"], ""),
        def!("jbcdt2_dates_oc", "S&P high-probability dates · buy the open, exit the close", "Ch.5 · Day of the Week Patterns", "Long", Live,
            &["F|Figure 5-6 (p.70) \"High probability of close higher than open: dates\" for S&P: 1/5, 1/14, 2/2, 4/12, 4/16, 5/19, 6/1, 7/14, 8/27, 11/24, 12/13",
              "E|\"these are pure day trades, since they would be entered on the open and exited on the close\" (p.69)",
              "X|Exit on the close of the same day"],
            "A date that is not a trading day gives no trade (the book's per-date counts, e.g. 11 years for 1/5, are years in which the date traded). The table also lists soybeans, T-bonds and yen dates (futures: no data). On SPX before 2001 many opens equal the prior close (stale index opens), so open-to-close results there are not the futures' results."),
        def!("jbcdt2_preholiday_oc", "Pre-holiday day trade · buy the open, exit the close", "Ch.5 · Day of the Week Patterns", "Long", Live,
            &["F|Merrill's holidays (Figure 5-4, p.68): Good Friday, Memorial Day, Independence Day, Labor Day, Thanksgiving Day, Christmas, New Year's Day",
              "E|\"Another way would be to buy on the opening of the target day and exit on the close. While this would be a true day trade, it would not be a trade made exactly in accordance with the Merrill research\" (p.68-69); target day = the trading day before the holiday",
              "X|Exit on the close of the target day"],
            "Holiday dates by the US rules (Easter algorithm for Good Friday; Memorial Day = last Monday of May from 1971, May 30 before; Labor Day = first Monday of September; Thanksgiving = fourth Thursday of November; July 4 / Christmas / New Year's observed on Friday when on Saturday and on Monday when on Sunday; a Saturday New Year's Day closes no day, so its pre-holiday day is the Friday before). Target = the last bar dated before the (observed) holiday. p.set=1 keeps only Christmas, New Year's and Independence Day, the three S&P holidays of Figure 5-5. The other variant (\"a long position could be entered on the close of trading the day prior to the target day ... closed out the next day\") is the vault's mzwws_preholiday / kpss_hol_d1."),
        def!("jbcdt2_mon_cc", "Monday pattern · buy Friday's close, exit Monday's close", "Ch.5 · The Monday Pattern", "Long", Live,
            &["E|\"Yet another way to take advantage of the Monday pattern is to buy on the close Friday and exit on the close Monday\" (p.72)",
              "X|Exit on Monday's close"],
            "Friday = last trading day of the week, Monday = the first trading day of the next week (p.73: \"Mondays (or the first trading day of the week)\"). The other Monday variant (\"buying on the opening on Mondays and exiting on the close\") is the vault's lwlts_mon_oc / pbwts_dow_oc; the buy-signals-only filter (p.71) has no system attached."),
        def!("jbcdt2_or30", "30-minute breakout (30MBO)", "Ch.4 · The 30-Minute Breakout", "Long/Short", NeedsData("30-minute bars (first-half-hour range and half-hour closes)"),
            &["E|\"After the first 30 minutes, buy if the ending price of the 30 minutes is greater than the high of the first 30-minute price bar by a predetermined number of ticks\" (p.46)",
              "X|\"Exit your trade at the end of the day\"; S&P: 12 ticks each side, $2,050 initial stop, $1,000 floor, 50% trail (p.48-49)"], "Daily-data relatives: jb30d_or30 / jb624_or30 (also intraday)."),
        def!("jbcdt2_inside_day", "Inside day (ID) breakout, exit first profitable opening", "Ch.7 · Power of the Inside Day", "Long/Short", NotTestable("the x ticks and the stop loss behind the S&P record (Figure 7-5) are not given: \"The optimum number of ticks ... is a function of the market\""),
            &["E|\"After an ID, buy on a stop above the high of the ID plus x ticks\"; \"go short on a stop below the low of the ID minus x ticks\" (p.83)",
              "X|\"Use a risk management stop loss\"; \"Exit on the first profitable opening or at the stop loss, whichever comes first\" (p.83-86)"], "Book: 430 trades, 69%, $219.78 per trade, S&P 1982-1998 (p.85)."),
        def!("jbcdt2_cor", "COR open/close relationship method", "Ch.11 · Closings and Openings", "Long/Short", NeedsData("20- and 30-minute bars (entries on the next bar's open, exits MOC or nth profitable open)"),
            &["E|\"Following x consecutive price bars where the close is greater than the open, a buy signal is triggered and a long position initiated on the opening of the next price bar\" (p.123)",
              "X|Initial stop, floor amount, percent trail, nth profitable opening or MOC (p.124); inputs per test in Figures 11-9 to 11-13"], ""),
        def!("jbcdt2_gap", "Basic gap (oops) day trade", "Ch.12 · Gap Methods for Day Trading", "Long/Short", NotTestable("duplicate: the vault already has it (jbcdt_gap_go, jbudt_gap_*, ss_study_oops); the 10/12-tick optimum has no stated stop"),
            &["E|\"A gap buy signal is generated once the market has opened on a gap lower and then comes back up to penetrate the previous daily low by a given number of ticks\" (p.142)"], ""),
        def!("jbcdt2_multigap", "Multiple-day gap (multigap)", "Ch.12 · Gap Methods for Day Trading", "Long/Short", NotTestable("only the windows are given (9 days buy, 2 days sell); the penetration ticks and stop loss are \"a few more parameters\" not printed (p.148)"),
            &["E|\"If a market opens below its lowest low of the last x days, then buy on a penetration back up through the low by x ticks\" (p.147)",
              "X|\"Exit on a predetermined risk management stop loss, on the close of the day, or on the nth profitable opening\" (p.148)"], "Book: 177 trades 60% $482 (day trade, Fig 12-15); 169 trades 85% $780 (FPO, Fig 12-18)."),
        def!("jbcdt2_add", "Accumulation/distribution derivative (ADD) day trade", "Ch.13 · A/D Oscillator Derivative", "Long/Short", NeedsData("intraday bars (10-minute; all trades exit at the end of the day session)"),
            &["E|\"A buy signal occurs when the A/D line crosses above its moving average after being below it\"; 28-period simple MA; \"the crossover must remain in effect for at least two postings\" (p.164)",
              "X|\"All trades are exited at the end of the day session\" (p.164)"], "The daily ADD is jbsmsw_add."),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "jbcdt2_dbo_mon" => ("jbcdt2_dbo", "DBO day-of-week breakout (CDT II)", "Monday · 3/26 ticks, $3,400"),
        "jbcdt2_dbo_thu" => ("jbcdt2_dbo", "DBO day-of-week breakout (CDT II)", "Thursday · 6/21 ticks, $3,500"),
        "jbcdt2_dbo_wed" => ("jbcdt2_dbo", "DBO day-of-week breakout (CDT II)", "Wednesday · not testable"),
        "jbcdt2_dates_oc" => ("jbcdt2_calendar", "Calendar day trades (CDT II)", "S&P dates, open to close"),
        "jbcdt2_preholiday_oc" => ("jbcdt2_calendar", "Calendar day trades (CDT II)", "Pre-holiday, open to close"),
        "jbcdt2_mon_cc" => ("jbcdt2_calendar", "Calendar day trades (CDT II)", "Friday close to Monday close"),
        "jbcdt2_or30" | "jbcdt2_cor" | "jbcdt2_add" => ("jbcdt2_intraday", "CDT II intraday methods", "Needs intraday bars"),
        "jbcdt2_inside_day" | "jbcdt2_gap" | "jbcdt2_multigap" => ("jbcdt2_other", "CDT II other methods", "Not testable"),
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    let dbo = |b, s, st| vec![p("buy_off", "Buy stop above the setup day's high (points)", b, 0., 10., 0.05), p("sell_off", "Sell stop below the setup day's low (points)", s, 0., 10., 0.05),
        p("stop", "Stop loss from the entry (points)", st, 1., 40., 0.1), p("entry_day", "Entry-day stop: 0 = bar path O-near-far-C, 1 = any touch", 0., 0., 1., 1.)];
    Some(match id {
        "jbcdt2_dbo_mon" => dbo(0.15, 1.3, 6.8),
        "jbcdt2_dbo_thu" => dbo(0.3, 1.05, 7.0),
        "jbcdt2_preholiday_oc" => vec![p("set", "Holidays: 0 = Merrill's seven (Fig. 5-4), 1 = Christmas/New Year's/July 4 (Fig. 5-5)", 0., 0., 1., 1.)],
        "jbcdt2_dates_oc" | "jbcdt2_mon_cc" => vec![],
        _ => return None,
    })
}

// ---------------------------------------------------------------- calendar helpers

/// Days since 1970-01-01 for a YYYY-MM-DD date (proleptic Gregorian).
pub fn days(d: &str) -> i64 {
    let y: i64 = d[0..4].parse().unwrap_or(1970);
    let m: i64 = d[5..7].parse().unwrap_or(1);
    let dd: i64 = d[8..10].parse().unwrap_or(1);
    days_ymd(y, m, dd)
}

pub fn days_ymd(y: i64, m: i64, d: i64) -> i64 {
    let (y2, m2) = if m <= 2 { (y - 1, m + 12) } else { (y, m) };
    365 * y2 + y2 / 4 - y2 / 100 + y2 / 400 + (153 * (m2 - 3) + 2) / 5 + d - 719469
}

/// 0 = Monday .. 6 = Sunday
pub fn weekday(d: &str) -> i64 {
    (days(d) + 3).rem_euclid(7)
}

fn wd(y: i64, m: i64, d: i64) -> i64 {
    (days_ymd(y, m, d) + 3).rem_euclid(7)
}

/// n-th (1-based) given weekday of a month; n = 0 = last.
fn nth_wd(y: i64, m: i64, w: i64, n: i64) -> i64 {
    if n == 0 {
        let last = days_ymd(if m == 12 { y + 1 } else { y }, if m == 12 { 1 } else { m + 1 }, 1) - 1;
        last - ((last + 3).rem_euclid(7) - w).rem_euclid(7)
    } else {
        let first = days_ymd(y, m, 1);
        first + (w - (first + 3).rem_euclid(7)).rem_euclid(7) + 7 * (n - 1)
    }
}

fn easter(y: i64) -> i64 {
    let a = y % 19;
    let b = y / 100;
    let c = y % 100;
    let d = b / 4;
    let e = b % 4;
    let f = (b + 8) / 25;
    let g = (b - f + 1) / 3;
    let h = (19 * a + b - d - g + 15) % 30;
    let i = c / 4;
    let k = c % 4;
    let l = (32 + 2 * e + 2 * i - h - k) % 7;
    let m = (a + 11 * h + 22 * l) / 451;
    let month = (h + l - 7 * m + 114) / 31;
    let day = (h + l - 7 * m + 114) % 31 + 1;
    days_ymd(y, month, day)
}

/// Observed closure day: Saturday -> Friday, Sunday -> Monday; a Saturday New Year's Day closes no day (NYSE), so the
/// Saturday itself is kept and the pre-holiday day is the Friday before it.
fn observed(y: i64, m: i64, d: i64, new_year: bool) -> i64 {
    let k = days_ymd(y, m, d);
    match wd(y, m, d) {
        5 => if new_year { k } else { k - 1 },
        6 => k + 1,
        _ => k,
    }
}

/// US market-holiday closure days (days since 1970) for year y: (closure day, holiday tag).
/// Tags: 0 Good Friday, 1 Memorial, 2 Independence, 3 Labor, 4 Thanksgiving, 5 Christmas, 6 New Year's.
pub fn us_holidays(y: i64) -> Vec<(i64, u8)> {
    let mut v = vec![(easter(y) - 2, 0u8)];
    v.push((if y >= 1971 { nth_wd(y, 5, 0, 0) } else { observed(y, 5, 30, false) }, 1));
    v.push((observed(y, 7, 4, false), 2));
    v.push((nth_wd(y, 9, 0, 1), 3));
    v.push((nth_wd(y, 11, 3, 4), 4));
    v.push((observed(y, 12, 25, false), 5));
    v.push((observed(y, 1, 1, true), 6));
    v
}

// ---------------------------------------------------------------- shared DBO simulator (also used by jbeft)

/// Day-of-week breakout. `thursday` = false: setup = last bar of a calendar week, entry on the next bar (first of the
/// next week); true: setup = a Wednesday bar followed by a Thursday bar. Buy stop setup high + buy_off when the setup
/// closed above its open, sell stop setup low - sell_off when it closed below. Exit: first later opening better than the
/// entry price, or the stop `stop` points from the entry (resting from the day after entry; on the entry day only if the
/// bar path reaches it after the fill, or any touch with entry_any).
#[allow(clippy::too_many_arguments)]
pub fn dbo(x: &Ctx, from: usize, to: usize, thursday: bool, bo: f64, so: f64, sl: f64, entry_any: bool) -> Vec<Trade> {
    let (mut out, mut cur): (Vec<Trade>, Option<Trade>) = (Vec::new(), None);
    let mut stop = f64::NAN;
    for i in from..=to {
        if let Some(t) = cur.as_ref().filter(|t| t.legs[0].i < i) {
            let (d, e) = (t.dir, t.legs[0].px);
            if (x.o[i] - e) * d > 0.0 {
                out.push(finish(cur.take().unwrap(), i, x.o[i], i, "First profitable opening"));
            } else if let Some(px) = stop_fill(x, i, -d, stop) {
                out.push(finish(cur.take().unwrap(), i, px, i, "Stop loss"));
            }
        }
        if cur.is_some() || i >= to {
            continue;
        }
        let setup = if thursday { weekday(&x.r[i].date) == 2 && weekday(&x.r[i + 1].date) == 3 } else { week_changes(x, i) };
        if !setup {
            continue;
        }
        let j = i + 1;
        let d = if x.c[i] > x.o[i] { 1.0 } else if x.c[i] < x.o[i] { -1.0 } else { continue };
        let lvl = if d > 0.0 { x.h[i] + bo } else { x.l[i] - so };
        if let Some(px) = stop_fill(x, j, d, lvl) {
            stop = px - d * sl;
            let t = new_trade(d, j, px, "stop", i);
            let hit = if entry_any { if d > 0.0 { x.l[j] <= stop } else { x.h[j] >= stop } } else { hit_after(x, j, d, lvl, stop) };
            if hit { out.push(finish(t, j, stop, j, "Stop loss (entry day)")); } else { cur = Some(t); }
        }
    }
    mark_open(&mut out, cur, x, to);
    out
}

/// After a stop entry in direction d at `lvl` on bar i, does the bar path (O, nearer extreme, other extreme, C) reach `stop`?
fn hit_after(x: &Ctx, i: usize, d: f64, lvl: f64, stop: f64) -> bool {
    let p = bar_path(x, i);
    let Some(k) = (0..4).find(|&k| (p[k] - lvl) * d >= 0.0) else { return false };
    (k..4).any(|m| (p[m] - stop) * d <= 0.0)
}

// ---------------------------------------------------------------- run

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !id.starts_with("jbcdt2_") || params(id).is_none() || to >= x.c.len() || from > to {
        return None;
    }
    let g = |k: &str| getp(id, p, k);
    let mut out = Vec::new();
    match id {
        "jbcdt2_dbo_mon" | "jbcdt2_dbo_thu" => {
            return Some(dbo(x, from, to, id == "jbcdt2_dbo_thu", g("buy_off"), g("sell_off"), g("stop"), g("entry_day") >= 0.5));
        }
        "jbcdt2_dates_oc" => {
            for i in from..=to {
                let d = &x.r[i].date;
                let (m, dd): (u32, u32) = (d[5..7].parse().unwrap_or(0), d[8..10].parse().unwrap_or(0));
                if SP_DATES.contains(&(m, dd)) {
                    out.push(finish(new_trade(1.0, i, x.o[i], "open", i), i, x.c[i], i, "Close of the date"));
                }
            }
        }
        "jbcdt2_preholiday_oc" => {
            let three = g("set") >= 0.5;
            let y0: i64 = x.r[from].date[0..4].parse().unwrap_or(1970);
            let y1: i64 = x.r[to].date[0..4].parse().unwrap_or(1970);
            let mut targets: Vec<i64> = Vec::new();
            for y in y0..=y1 + 1 {
                for (k, tag) in us_holidays(y) {
                    if !three || matches!(tag, 2 | 5 | 6) {
                        targets.push(k);
                    }
                }
            }
            targets.sort();
            targets.dedup();
            // target bar = last bar dated before the closure day
            for i in from..=to {
                let di = days(&x.r[i].date);
                let next = if i + 1 < x.r.len() { days(&x.r[i + 1].date) } else { i64::MAX };
                if targets.iter().any(|&k| di < k && next >= k) {
                    out.push(finish(new_trade(1.0, i, x.o[i], "open", i), i, x.c[i], i, "Close of the pre-holiday day"));
                }
            }
        }
        "jbcdt2_mon_cc" => {
            for i in from..to {
                if week_changes(x, i) {
                    out.push(finish(new_trade(1.0, i, x.c[i], "close", i), i + 1, x.c[i + 1], i + 1, "Monday close"));
                }
            }
        }
        _ => {}
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    fn row(d: &str, o: f64, h: f64, l: f64, c: f64) -> Row {
        Row { ticker: "T".into(), date: d.into(), open: o, high: h, low: l, close: c, volume: None }
    }
    #[test]
    fn holidays_known_years() {
        let k = |y, m, d| days_ymd(y, m, d);
        let h = us_holidays(2004);
        assert!(h.contains(&(k(2004, 4, 9), 0))); // Good Friday 2004
        assert!(h.contains(&(k(2004, 5, 31), 1)));
        assert!(h.contains(&(k(2004, 7, 5), 2))); // July 4 on Sunday -> Monday
        assert!(h.contains(&(k(2004, 9, 6), 3)));
        assert!(h.contains(&(k(2004, 11, 25), 4)));
        assert!(h.contains(&(k(2004, 12, 24), 5))); // Christmas on Saturday -> Friday
        assert!(us_holidays(2005).contains(&(k(2005, 1, 1), 6))); // Saturday New Year's: kept on the Saturday
        assert_eq!(weekday("2026-10-11"), 6);
    }
    #[test]
    fn dbo_thursday_hand_checked() {
        // Wed up close; Thu trades through Wed high + 0.30 -> long; Fri opens above entry -> FPO
        let r = vec![
            row("2024-01-09", 100., 101., 99., 100.),
            row("2024-01-10", 100., 102., 99.5, 101.), // Wednesday, close > open
            row("2024-01-11", 101.5, 104., 101., 103.), // Thursday: buy stop 102.30
            row("2024-01-12", 103.5, 104., 103., 103.8), // Friday open 103.5 > 102.3
        ];
        let x = Ctx::new(&r, &r, &[]);
        let ts = run("jbcdt2_dbo_thu", &x, 0, 3, &Params::new()).unwrap();
        assert_eq!(ts.len(), 1);
        assert!((ts[0].legs[0].px - 102.3).abs() < 1e-9);
        assert_eq!(ts[0].exit_i, 3);
        assert!((ts[0].exit_px - 103.5).abs() < 1e-9);
    }
}
