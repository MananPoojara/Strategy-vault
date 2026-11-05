//! Robert Steelman, "Seasonal Patterns In The Markets", Technical Analysis of Stocks & Commodities V.24:2 (February 2006) pp.72-77
//! (research share Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V24/C02/034STEE.pdf).
//! Book id `scsteel`. Lane K40 (4 Nov 2026, decision #204); independent check authors/titles-1030/K40w/k40_ref.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::lc3::Pos;
use super::sctraub::{days, weekday, week_id};

pub const REVIEWED: &[&str] = &["scsteel_seasonal"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] Five calendar day trades, each entered at the open and closed at the close of the same day ('enter on the open of the pattern day and exit at the close of that day. The only exception is Christmas, when you exit at the close of the next day'): (1) long the first trading day of every month; (2) short the ninth calendar day of the month, only when the 9th is a trading day; (3) long expiration-week Monday = the first trading day of the calendar week that holds the month's third Friday (the article: 'In January and February, because of the holidays, this trade could take place on a Tuesday'); (4) short the last trading day of November; (5) long the last two trading days before 25 December, in at the open of the first, out at the close of the second. Default pattern = 0 trades all five (the article's combined strategy; 'All of the featured patterns are independent'); options pattern = 1-5 trade one pattern, skip = 1 leaves January and August out of pattern 1 (the article's variant, 512 points / 200 days). Last trading days are read from the next bar's date (the exchange calendar). The article skipped expiration-week Monday in September 2001 ('extraordinary events'); the rule here takes it (17 September 2001 was a trading day). Book (SPX 1985-2004, index points): first day 477.41 / 240 trades, ninth day 299.42 / 169, expiration Monday 406.26 / 240 (239 taken), last November day 86.54 / 20, Christmas 98.38 / 20 (40 days); all patterns 1,368 points in 708 days (Figure 6); on S&P e-mini futures 11/20/1997-12/31/2004: 232 trades, 151 (65%) positive (Figures 7-8). SPX opens are real only from 2014, so SPX results start there and the 1985-2004 table is compared on SPY. Runs on other indices, ETFs and stocks, US and India, are a declared application.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "scsteel_seasonal", name: "Steelman seasonal day trades: long the 1st trading day, short the 9th, long expiration-week Monday, short the last November day, long the 2 days before Christmas", group: "S&C: Seasonal Patterns In The Markets (Steelman, V.24:2)", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "scsteel", rules: &[
            "F|\"I am going to use just one day, the most profitable one, which is the first trading day of the month.\"; \"The second pattern is based on the ninth calendar day of the month ... it exhibits itself only when the ninth calendar day happens to be a trading day.\"; \"It is the Monday preceding the third Friday of the month.\"; \"The fourth pattern involves just one day of the year, the last trading day in November.\"; \"be in the market for two days in a row before Christmas\" (pp.72-75)",
            "E|\"The strategy is very simple: enter on the open of the pattern day\" (long: first trading day, expiration-week Monday, Christmas; short: ninth calendar day, last trading day of November; Figures 1-5) (p.76)",
            "X|\"and exit at the close of that day. The only exception is Christmas, when you exit at the close of the next day.\" (p.76)"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { reviewed(id).then_some(("scsteel_seasonal", "Steelman seasonal day trades (S&C)", "five calendar patterns")) }

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    reviewed(id).then(|| vec![p("pattern", "Pattern: 0 = all five (article); 1 first day, 2 ninth day short, 3 expiration Monday, 4 last November day short, 5 Christmas", 0., 0., 5., 1.),
        p("skip", "Pattern 1 without January and August (article variant): 1 = yes", 0., 0., 1., 1.)])
}

fn ymd(d: &str) -> (i64, i64, i64) { (d[0..4].parse().unwrap_or(0), d[5..7].parse().unwrap_or(1), d[8..10].parse().unwrap_or(1)) }
/// Day number of the month's third Friday.
fn third_friday(y: i64, m: i64) -> i64 { let d1 = days(&format!("{y:04}-{m:02}-01")); d1 + (4 - weekday(&format!("{y:04}-{m:02}-01"))).rem_euclid(7) + 14 }

/// The pattern of bar t (1-5; 5 = the first of the two Christmas days), 0 = none.
pub fn pattern(x: &Ctx, t: usize) -> u8 {
    let n = x.r.len();
    let d = &x.r[t].date;
    let (y, m, dd) = ymd(d);
    let next = |k: usize| (k + 1 < n).then(|| ymd(&x.r[k + 1].date));
    let last_before_xmas = |k: usize| { let (_, mk, dk) = ymd(&x.r[k].date); mk == 12 && dk < 25 && next(k).is_some_and(|(_, m2, d2)| m2 != 12 || d2 >= 25) };
    if t + 1 < n && last_before_xmas(t + 1) && m == 12 { return 5; }
    if t > 0 && ymd(&x.r[t - 1].date).1 != m { return 1; }
    if dd == 9 { return 2; }
    let w = week_id(d);
    if w == third_friday(y, m) - 4 && (t == 0 || week_id(&x.r[t - 1].date) != w) { return 3; }
    if m == 11 && next(t).is_some_and(|(_, m2, _)| m2 != 11) { return 4; }
    0
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let (only, skip) = (p.get("pattern").copied().unwrap_or(0.).round() as u8, p.get("skip").copied().unwrap_or(0.).round() == 1.);
    let mut pos = Pos::new();
    let mut t = from;
    while t <= to {
        let k = pattern(x, t);
        let m = ymd(&x.r[t].date).1;
        if k != 0 && (only == 0 || only == k) && !(k == 1 && skip && (m == 1 || m == 8)) {
            let (d, last) = (if k == 2 || k == 4 { -1. } else { 1. }, if k == 5 { t + 1 } else { t });
            if last <= to {
                pos.enter(d, t, x.o[t], "open");
                pos.exit(last, x.c[last], ["", "first trading day", "ninth calendar day", "expiration-week Monday", "last trading day of November", "before Christmas"][k as usize]);
                t = last;
            }
        }
        t += 1;
    }
    Some(pos.done(to, x.c[to]))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn calendar_patterns() {
        assert_eq!(super::super::sctraub::civil(third_friday(2026, 11)), (2026, 11, 20));
        let r = super::super::sctraub::tests::bars(1500);
        let x = Ctx::new(&r, &r, &[]);
        let t = run("scsteel_seasonal", &x, 0, r.len() - 1, &Params::new()).unwrap();
        let got: Vec<_> = t.iter().filter(|z| r[z.legs[0].i].date.starts_with("2001-1")).map(|z| (r[z.legs[0].i].date.as_str(), r[z.exit_i].date.as_str(), z.dir)).collect();
        // Oct 2001: 1st (Mon) long, 9th (Tue) short, week of the 19th: Mon 15th long; Nov: 1st, 9th, 12th (third Friday 16th), 30th short; Dec: 3rd (the 9th is a Sunday), 17th (third Friday 21st), 21st-24th
        assert_eq!(got, vec![("2001-10-01", "2001-10-01", 1.), ("2001-10-09", "2001-10-09", -1.), ("2001-10-15", "2001-10-15", 1.),
            ("2001-11-01", "2001-11-01", 1.), ("2001-11-09", "2001-11-09", -1.), ("2001-11-12", "2001-11-12", 1.), ("2001-11-30", "2001-11-30", -1.),
            ("2001-12-03", "2001-12-03", 1.), ("2001-12-17", "2001-12-17", 1.), ("2001-12-21", "2001-12-24", 1.)]);
        assert!(t.iter().all(|z| z.legs[0].px == x.o[z.legs[0].i] && z.exit_px == x.c[z.exit_i]));
    }
}

// Instrument admission: S&C Steelman seasonal day trades 2026-11-04 (titles lane K40).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "scsteel" && reviewed(def.id),
    label: "Robert Steelman measured the patterns on the S&P 500 index (1985-2004) and traded them on S&P e-mini futures (1997-2004). Runs on other indices, ETFs and stocks, US and India, are a declared application.",
    basis: "Technical Analysis of Stocks & Commodities V.24:2 pp.72-77 (Seasonal Patterns In The Markets), Figures 1-8; independent check authors/titles-1030/K40w/k40_ref.py",
};
