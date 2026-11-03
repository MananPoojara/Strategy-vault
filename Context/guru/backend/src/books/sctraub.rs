//! Daniel J. Traub, "Three Market Timing Strategies", Technical Analysis of Stocks & Commodities V.12:12 (December 1994)
//! pp.507-510 (research share Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V12/C12/
//! THREE.PDF). Book id `sctraub`: the trend-following strategy (strategy 1 of 3). Lane K13 (titles loop, 2 Nov 2026), to-code
//! row 201; independent check authors/titles-1030/K13w/k13_ref.py.
//! This module also holds the small calendar / indicator helpers the other lane-K13 S&C article modules share.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::msws::sma;
use super::tselhs::Book;

const NAN: f64 = f64::NAN;

// ------------------------------------------------------------------ shared helpers (lane K13 S&C article modules)
/// Days since 1970-01-01 for "YYYY-MM-DD" (proleptic Gregorian).
pub fn days(date: &str) -> i64 {
    let y: i64 = date.get(0..4).and_then(|s| s.parse().ok()).unwrap_or(1970);
    let m: i64 = date.get(5..7).and_then(|s| s.parse().ok()).unwrap_or(1);
    let d: i64 = date.get(8..10).and_then(|s| s.parse().ok()).unwrap_or(1);
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * ((m + 9) % 12) + 2) / 5 + d - 1;
    era * 146097 + yoe * 365 + yoe / 4 - yoe / 100 + doy - 719468
}
/// (year, month, day) of a day number.
pub fn civil(z: i64) -> (i64, i64, i64) {
    let z = z + 719468;
    let era = z.div_euclid(146097);
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (yoe + era * 400 + (m <= 2) as i64, m, d)
}
/// 0 = Monday ... 6 = Sunday.
pub fn weekday(date: &str) -> i64 { (days(date) + 3).rem_euclid(7) }
/// Day number of the Monday of the date's calendar week.
pub fn week_id(date: &str) -> i64 { days(date) - weekday(date) }
pub fn month_of(date: &str) -> i64 { date.get(5..7).and_then(|s| s.parse().ok()).unwrap_or(1) }
fn month_id(date: &str) -> i64 { date.get(0..4).and_then(|s| s.parse::<i64>().ok()).unwrap_or(1970) * 12 + month_of(date) }
fn days_in_month(date: &str) -> i64 {
    let y: i64 = date.get(0..4).and_then(|s| s.parse().ok()).unwrap_or(1970);
    match month_of(date) { 4 | 6 | 9 | 11 => 30, 2 => if (y % 4 == 0 && y % 100 != 0) || y % 400 == 0 { 29 } else { 28 }, _ => 31 }
}
/// Bars that close a week (`monthly` false) or a month: the next bar is in a later period; the last bar of the data closes its
/// week only on a Friday, its month only when no weekday of that month is left.
pub fn period_ends(x: &Ctx, monthly: bool) -> Vec<usize> {
    let n = x.r.len();
    let id = |i: usize| if monthly { month_id(&x.r[i].date) } else { week_id(&x.r[i].date) };
    (0..n).filter(|&i| if i + 1 < n { id(i + 1) != id(i) } else {
        let d = &x.r[i].date;
        let wd = weekday(d);
        if monthly { let day: i64 = d.get(8..10).and_then(|s| s.parse().ok()).unwrap_or(1); day + match wd { 4 => 3, 5 => 2, _ => 1 } > days_in_month(d) } else { wd == 4 }
    }).collect()
}
/// Calendar-week groups of bars: (first bar, last bar).
pub fn week_groups(x: &Ctx) -> Vec<(usize, usize)> {
    let mut g: Vec<(usize, usize)> = vec![];
    for i in 0..x.r.len() {
        match g.last_mut() { Some(w) if week_id(&x.r[w.1].date) == week_id(&x.r[i].date) => w.1 = i, _ => g.push((i, i)) }
    }
    g
}
/// EMA, alpha 2/(n+1), seeded with the first finite value; a NaN input keeps the last value and gives NaN.
pub fn ema0(v: &[f64], n: usize) -> Vec<f64> {
    let k = 2.0 / (n as f64 + 1.0);
    let mut e: Option<f64> = None;
    v.iter().map(|&x| if !x.is_finite() { NAN } else { let y = match e { None => x, Some(p) => p + k * (x - p) }; e = Some(y); y }).collect()
}
/// Mean of v[start..start+n], then Wilder smoothing (prev*(n-1)+x)/n.
pub fn wilder_from(v: &[f64], n: usize, start: usize) -> Vec<f64> {
    let mut out = vec![NAN; v.len()];
    if v.len() < start + n { return out; }
    let mut e = v[start..start + n].iter().sum::<f64>() / n as f64;
    out[start + n - 1] = e;
    for i in start + n..v.len() { e = (e * (n as f64 - 1.0) + v[i]) / n as f64; out[i] = e; }
    out
}
/// Wilder DMI: (+DI, -DI, ADX), smoothing from bar 1.
pub fn dmi(x: &Ctx, n: usize) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
    let len = x.c.len();
    let (mut tr, mut pm, mut mm) = (vec![0.0; len], vec![0.0; len], vec![0.0; len]);
    for i in 1..len {
        tr[i] = (x.h[i] - x.l[i]).max((x.h[i] - x.c[i - 1]).abs()).max((x.l[i] - x.c[i - 1]).abs());
        let (u, d) = (x.h[i] - x.h[i - 1], x.l[i - 1] - x.l[i]);
        pm[i] = if u > d && u > 0.0 { u } else { 0.0 };
        mm[i] = if d > u && d > 0.0 { d } else { 0.0 };
    }
    let (st, sp, sm) = (wilder_from(&tr, n, 1), wilder_from(&pm, n, 1), wilder_from(&mm, n, 1));
    let (mut p, mut m, mut dx) = (vec![NAN; len], vec![NAN; len], vec![NAN; len]);
    for i in 0..len {
        if st[i].is_finite() && st[i] > 0.0 {
            p[i] = 100.0 * sp[i] / st[i];
            m[i] = 100.0 * sm[i] / st[i];
            dx[i] = if p[i] + m[i] > 0.0 { 100.0 * (p[i] - m[i]).abs() / (p[i] + m[i]) } else { 0.0 };
        }
    }
    let adx = match dx.iter().position(|v| v.is_finite()) { Some(f) => wilder_from(&dx, n, f), None => vec![NAN; len] };
    (p, m, adx)
}
/// Position changes decided at closes from..to-1 and filled at the next open: `want(t, side)` = the side wanted (0 = flat).
pub fn next_open(x: &Ctx, from: usize, to: usize, why: &str, mut want: impl FnMut(usize, f64) -> f64) -> Vec<Trade> {
    let mut pos = Book::new();
    let mut pend: Option<f64> = None;
    for t in from..=to {
        if let Some(d) = pend.take() { if d == 0.0 { pos.exit(t, x.o[t], why) } else { pos.enter(d, t, x.o[t], "open") } }
        if t < to { let d = want(t, pos.side()); if d != pos.side() { pend = Some(d); } }
    }
    pos.done(to, x.c[to])
}
/// Instrument tick times the `tick` parameter (default 1).
pub fn ticks(x: &Ctx, p: &Params) -> f64 { p.get("tick").copied().unwrap_or(1.0) * x.market_tick() }

// ------------------------------------------------------------------ Traub trend-following strategy
pub const REVIEWED: &[&str] = &["sctraub_trend"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] Weekly closes = the close of each calendar week's last trading day (the article acts 'on the Friday of the week that the signals are generated'; a holiday Friday uses that week's last day). '40-week SMA ... including the current week'; its three-week SMA; 'five-week high ... the highest week-ending close over the prior five weeks' and 'the lowest week-ending close over the prior 10 weeks' exclude the current week. Long or cash; fills at the signal week's last close. HELD: strategy 2 (prime-rate changes, no prime-rate series is held) and strategy 3 (55-week oscillator with -15 / +50 S&P index-point thresholds, which are not converted, decision 19). Article (S&P 500, 1972 to June 1994, capital gain only): 22 round trips, 397.0% total, 7.4% a year, 67.9% of the time invested; exit on 25 March 1994. Our SPX run (data from 1983) also exits on 25 March 1994. The vault's nfce5_traub is a different Traub model (5-week average of a 50-week average, 9-week high). Runs on indices, ETFs and stocks, US and India, are a declared application.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "sctraub_trend", name: "Traub weekly trend timing: buy above the 3-week average of the 40-week average and the prior 5-week high, sell below it and the prior 10-week low", group: "S&C: Three Market Timing Strategies (Traub, V.12:12)", session: 0, dir: "Long", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "sctraub", rules: &[
            "F|\"1 40-week simple moving average of the S&P 500, including the current week ... 2 Three-week simple moving average of calculation 1 ... 3 Five-week high ... the highest week-ending close over the prior five weeks. 4 10-week low ... the lowest week-ending close over the prior 10 weeks.\" (V.12:12 p.508)",
            "E|\"Buy when the current week's close of the S&P 500 is greater than three-week SMA [40-week SMA [S&P 500]] and also greater than the five-week high [S&P 500].\" (p.508)",
            "X|\"Sell when the current week's close of the S&P 500 is less than three-week SMA [40-week SMA [S&P 500]] and also less than 10-week low [S&P 500].\" (p.508) Action 'on the Friday of the week that the signals are generated'."],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let id = REVIEWED.iter().find(|r| **r == id)?;
    Some((*id, "Traub market timing (S&C)", "Weekly trend-following"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> { reviewed(id).then(Vec::new) }

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, _p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let we = period_ends(x, false);
    let w: Vec<f64> = we.iter().map(|&i| x.c[i]).collect();
    let m = sma(&sma(&w, 40), 3);
    let mut pos = Book::new();
    for (k, &i) in we.iter().enumerate() {
        if i < from || i > to || k < 10 || !m[k].is_finite() { continue; }
        let hi5 = w[k - 5..k].iter().copied().fold(f64::NEG_INFINITY, f64::max);
        let lo10 = w[k - 10..k].iter().copied().fold(f64::INFINITY, f64::min);
        if pos.side() == 0.0 && w[k] > m[k] && w[k] > hi5 { pos.enter(1.0, i, x.c[i], "close"); }
        else if pos.side() > 0.0 && w[k] < m[k] && w[k] < lo10 { pos.exit(i, x.c[i], "below the 3-week average and the 10-week low"); }
    }
    Some(pos.done(to, x.c[to]))
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::Row;
    /// Daily bars on real weekdays from 2001-01-01 (a Monday), a wavy series.
    pub fn bars(n: usize) -> Vec<Row> {
        let mut out = vec![];
        let mut d = days("2001-01-01");
        while out.len() < n {
            let wd = (d + 3).rem_euclid(7);
            if wd < 5 {
                let f = out.len() as f64;
                let c = 100. + 9. * (f / 17.).sin() + 4. * (f / 5.3).cos() + 6. * (f / 61.).sin() + 0.02 * f;
                let (y, m, dd) = super::civil(d);
                out.push(Row { ticker: "T".into(), date: format!("{y:04}-{m:02}-{dd:02}"), open: c - 0.6 * (f / 2.1).sin(), high: c + 1.3 + 0.4 * (f / 3.7).sin(), low: c - 1.1 - 0.3 * (f / 4.1).cos(), close: c, volume: Some(1e8 * (1.5 + (f / 7.).sin())) });
            }
            d += 1;
        }
        out
    }
    #[test]
    fn calendar_helpers() {
        assert_eq!(weekday("2026-11-02"), 0);
        assert_eq!(weekday("1970-01-01"), 3);
        let r = bars(400);
        let x = Ctx::new(&r, &r, &[]);
        let we = period_ends(&x, false);
        assert!(we.iter().all(|&i| weekday(&r[i].date) == 4));
        assert!(period_ends(&x, true).iter().all(|&i| i + 1 == r.len() || month_of(&r[i + 1].date) != month_of(&r[i].date)));
        assert_eq!(week_groups(&x).len(), 80);
    }
    #[test]
    fn traub_weekly_trend() {
        let r = bars(3000);
        let x = Ctx::new(&r, &r, &[]);
        let t = run("sctraub_trend", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(t.len() > 3 && t.iter().all(|z| z.dir > 0.0 && weekday(&r[z.legs[0].i].date) == 4));
        assert!(family_of("sctraub_trend").is_some() && run("sctraub_x", &x, 0, 9, &Params::new()).is_none());
    }
}

// Instrument admission: S&C Traub 2026-11-02 (titles lane K13).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "sctraub" && reviewed(def.id),
    label: "Daniel Traub tested the rule on S&P 500 weekly closes (1972-1994). Runs on indices, ETFs and stocks, US and India, are a declared application.",
    basis: "Technical Analysis of Stocks & Commodities V.12:12 pp.507-510 (Three Market Timing Strategies); independent check authors/titles-1030/K13w/k13_ref.py",
};
