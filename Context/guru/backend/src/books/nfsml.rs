//! Norman G. Fosback, "Stock Market Logic" (1976; research share Seasonality Top Reads/02-Stock-Market-Logic/Reading.pdf,
//! chapter 'The Seasonality Indicator', printed p.155-164, Tables 50-51). Book id `nfsml`. One rule: the complete
//! Seasonality Indicator (Month-End + Pre-Holiday components, overlaps counted once). The components are live separately
//! elsewhere (kftsm_month_end, kpss_hol_d2) with other definitions; their union is new. Excerpt
//! authors/titles-1030/R3w/fosback_seasonality.txt. Lane K5 (titles loop, 31 Oct 2026); independent check
//! authors/fosback/admission/k5_ref.py (header). US instruments only (instrument_policy fosback_symbol_issue): no official
//! NSE holiday list is held, and holidays are not inferred from data gaps (lead decision R3 #4).
use crate::books::mzwws::{dnum, wday, ymd};
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

/// Admitted ids (authors/fosback/admission/k5_ref.py (header)).
pub const REVIEWED: &[&str] = &["nfsml_seasonality"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }
/// Set by US exchange holidays: US instruments only.
pub const US_ONLY: &[&str] = &["nfsml_seasonality"];

const NOTE: &str = "[Declared readings] Long only on Seasonality Indicator days, cash otherwise. Month-End component = the last trading day of each month and the first four trading days of the next month (a month counts only when its first trading day and, for the last day, the next month's first bar are in the data). Pre-Holiday component = the two trading days immediately before a holiday market closing of the nine NYSE holidays of Table 50 (Washington's Birthday / Presidents' Day, Good Friday, Memorial Day, Independence Day, Labor Day, Election Day, Thanksgiving, Christmas, New Year's), each at its rule date (Saturday -> Friday, Sunday -> Monday; Washington's Birthday and Memorial Day on their Monday dates from 1971). A holiday counts as a closing only when that weekday has no bar in the instrument's own calendar (so Election Day counts only in the years the NYSE closed, to 1980, and a Saturday holiday whose Friday traded does not count). Closings for other reasons (Martin Luther King Day from 1998, Juneteenth from 2022, national days of mourning, weather) are not among the book's nine; p.newer = 1 adds the two newer holidays as an option. The two components are combined with overlaps counted once (Table 51). Close to close: bought at the close of the day before a run of indicator days, sold at the close of its last day. No costs (the book: 'assuming no commissions'). Holiday dates are US exchange rules, so the rule runs on US indices, ETFs and stocks only; India needs an official NSE holiday list, which the vault does not hold. A position still open at the window end is marked at the last close.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "nfsml_seasonality", name: "The complete Seasonality Indicator: long over each month-end (last day + first four) and the two days before each holiday closing, cash otherwise", group: "Fosback Stock Market Logic",
        session: 0, dir: "Long", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: "nfsml", rules: &[
            "E|\"The first hypothesizes superior stock market performance on the last trading day of each month and on the first four trading days of each month.\" (p.155) \"Put together, this continuous span of five trading days constitutes the 'Month-End' component\" (p.157)",
            "E|\"The Pre-Holiday component hypothesizes greater than average returns on the two trading days immediately preceding a holiday market closing.\" (p.155); the nine holidays of Table 50 (p.160)",
            "E|\"The complete Seasonality Indicator consists of the two day 'Pre-Holiday' and the five day 'Month-End' components ... Table 51 traces the overall seasonal strategy result for the combined pre-holiday and month-end periods, eliminating all overlapping.\" (p.163)",
            "X|\"buys stocks ... at the beginning of each five day month-end period, sells out at the end of the five day period, and holds cash until the next month-end period\" (p.157) = in at the close before, out at the close of the last day"],
        notes: NOTE, claim: None }]
}
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    (id == "nfsml_seasonality").then_some(("nfsml_seasonality", "Fosback Seasonality Indicator", "month-end + pre-holiday"))
}
pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max| ParamDef { key, label, def, min, max, step: 1. };
    (id == "nfsml_seasonality").then(|| vec![p("first", "Month-End: first trading days of the month (book 4)", 4., 0., 10.), p("last", "Month-End: last trading days of the month (book 1)", 1., 0., 5.),
        p("pre", "Pre-Holiday: trading days before a closing (book 2)", 2., 0., 5.), p("newer", "1 = also Martin Luther King Day and Juneteenth (not in the 1976 book)", 0., 0., 1.)])
}

fn easter(y: i64) -> i64 {
    let (a, b, c) = (y % 19, y / 100, y % 100);
    let (d, e, f) = (b / 4, b % 4, (b + 8) / 25);
    let h = (19 * a + b - d - (b - f + 1) / 3 + 15) % 30;
    let l = (32 + 2 * e + 2 * (c / 4) - h - c % 4) % 7;
    let m = (a + 11 * h + 22 * l) / 451;
    dnum(y, (h + l - 7 * m + 114) / 31, (h + l - 7 * m + 114) % 31 + 1)
}
/// k-th weekday wd (0 = Monday) of month m
fn nth(y: i64, m: i64, wd: i64, k: i64) -> i64 { let f = dnum(y, m, 1); f + (wd - wday(f)).rem_euclid(7) + 7 * (k - 1) }
fn last_mon_may(y: i64) -> i64 { let l = dnum(y, 5, 31); l - wday(l) }
fn observed(z: i64) -> i64 { match wday(z) { 5 => z - 1, 6 => z + 1, _ => z } }
/// Rule dates of the holidays of year y (Table 50; `newer` adds MLK Day from 1998 and Juneteenth from 2022).
fn holidays(y: i64, newer: bool) -> Vec<i64> {
    let mut v = vec![
        observed(dnum(y, 1, 1)),
        if y >= 1971 { nth(y, 2, 0, 3) } else { observed(dnum(y, 2, 22)) },
        easter(y) - 2,
        if y >= 1971 { last_mon_may(y) } else { observed(dnum(y, 5, 30)) },
        observed(dnum(y, 7, 4)),
        nth(y, 9, 0, 1),
        nth(y, 11, 0, 1) + 1,
        nth(y, 11, 3, 4),
        observed(dnum(y, 12, 25)),
    ];
    if newer && y >= 1998 { v.push(nth(y, 1, 0, 3)); }
    if newer && y >= 2022 { v.push(observed(dnum(y, 6, 19))); }
    v
}

/// Indicator day flags for every bar.
fn indicator_days(x: &Ctx, first: usize, last: usize, pre: usize, newer: bool) -> Vec<bool> {
    let n = x.r.len();
    let ym: Vec<(i64, i64)> = x.r.iter().map(|r| { let (y, m, _) = ymd(&r.date); (y, m) }).collect();
    let dn: Vec<i64> = x.r.iter().map(|r| { let (y, m, d) = ymd(&r.date); dnum(y, m, d) }).collect();
    let mut w = vec![false; n];
    // Month-End: k-th trading day from the start (month start must be in the data) and from the end (next month in the data)
    let mut s = 0;
    while s < n {
        let mut e = s;
        while e + 1 < n && ym[e + 1] == ym[s] { e += 1; }
        if s > 0 && first > 0 { for k in s..=e.min(s + first - 1) { w[k] = true; } }
        if e + 1 < n && last > 0 { for k in e.saturating_sub(last - 1).max(s)..=e { w[k] = true; } }
        s = e + 1;
    }
    // Pre-Holiday: the `pre` bars before each closing (a weekday rule date with no bar, inside the data)
    if n > 0 && pre > 0 {
        for y in ym[0].0..=ym[n - 1].0 + 1 {
            for d in holidays(y, newer) {
                let p = dn.partition_point(|&z| z < d);
                if wday(d) > 4 || p == 0 || p >= n || dn[p] == d { continue; }
                for k in p.saturating_sub(pre)..p { w[k] = true; }
            }
        }
    }
    w
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let s = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| s.iter().find(|d| d.key == k).unwrap().def);
    let u = |k: &str| g(k).round().max(0.) as usize;
    let w = indicator_days(x, u("first"), u("last"), u("pre"), g("newer") >= 0.5);
    let (c, n) = (&x.c, x.r.len());
    let mk = |e: usize, xi: usize, why: &str, open: bool| Trade { dir: 1., legs: vec![Leg { i: e, px: c[e], w: 1. }], exit_i: xi, exit_px: c[xi], reason: why.into(), entry_kind: "close", signal_i: xi, open, exits: vec![] };
    let (mut out, mut cur) = (vec![], None::<usize>);
    for i in from..=to {
        if let Some(e) = cur {
            if i + 1 < n && !w[i + 1] { out.push(mk(e, i, "last Seasonality Indicator day", false)); cur = None; }
        } else if i < to && i + 1 < n && w[i + 1] {
            cur = Some(i);
        }
    }
    if let Some(e) = cur { out.push(mk(e, to, "open", true)); }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    fn date(z: i64) -> String {
        // civil date from day number (inverse of dnum)
        let z = z + 719468;
        let era = z.div_euclid(146097);
        let doe = z - era * 146097;
        let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let d = doy - (153 * mp + 2) / 5 + 1;
        let m = if mp < 10 { mp + 3 } else { mp - 9 };
        format!("{:04}-{:02}-{:02}", yoe + era * 400 + if m <= 2 { 1 } else { 0 }, m, d)
    }
    #[test]
    fn month_ends_and_pre_holiday_days_merge_into_trades() {
        // NYSE weekdays 2001-10-01 .. 2002-01-31, closed on Thanksgiving, Christmas, New Year's and MLK Day 2002
        let shut = [dnum(2001, 11, 22), dnum(2001, 12, 25), dnum(2002, 1, 1), dnum(2002, 1, 21)];
        let r: Vec<Row> = (dnum(2001, 10, 1)..=dnum(2002, 1, 31)).filter(|&z| wday(z) < 5 && !shut.contains(&z))
            .enumerate().map(|(i, z)| Row { ticker: "SPY".into(), date: date(z), open: 100., high: 101., low: 99., close: 100. + i as f64, volume: None }).collect();
        let x = Ctx::new(&r, &r, &[]);
        let ds = |t: &Vec<Trade>| t.iter().map(|t| (r[t.legs[0].i].date.clone(), r[t.exit_i].date.clone())).collect::<Vec<_>>();
        let s = |a: &str, b: &str| (a.to_string(), b.to_string());
        let t = run("nfsml_seasonality", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert_eq!(ds(&t), vec![s("2001-10-30", "2001-11-06"), s("2001-11-19", "2001-11-21"), s("2001-11-29", "2001-12-06"), s("2001-12-20", "2001-12-24"), s("2001-12-27", "2002-01-07")]);
        let mut p = Params::new();
        p.insert("newer".into(), 1.);
        assert_eq!(ds(&run("nfsml_seasonality", &x, 0, r.len() - 1, &p).unwrap())[5], s("2002-01-16", "2002-01-18"));
        assert_eq!(easter(2024), dnum(2024, 3, 31));
    }
}

pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "nfsml" && crate::books::nfsml::reviewed(def.id),
    label: "Fosback measured the Seasonality Indicator on the S&P 500 (1928-1975); runs on other US indices, ETFs and US stocks are a declared application. The days are set by US exchange holidays, so Indian instruments are not run.",
    basis: "Norman G. Fosback, Stock Market Logic (1976), 'The Seasonality Indicator', printed p.155-164, Tables 50-51",
};
