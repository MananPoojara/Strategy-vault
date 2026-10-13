//! Commodity Traders Club News (CTCN, webtrading.com), member newsletter issues 01-77 (1993-2003). Book id `ctcn`.
//! Source: research drive "Trading Knowledge II/@NewsLetters-Mags/Commodity Futures Traders Club (CTCN) Issues 01 - 77
//! (webtrading.com)/Commodity Traders Club News.rar" (one .doc with all issues); copy in Book-to-rust/Books/Commodity
//! Traders Club News/. Page refs are issue number + contributor (the .doc has no page numbers). Most contributions are
//! opinion; only the four complete mechanical rules below are Live. Instrument swap only: futures rules run on the index.
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "ctcn";

fn mk(dir: f64, i: usize, px: f64, sig: usize, kind: &'static str) -> Trade {
    Trade { dir, legs: vec![Leg { i, px, w: 1. }], exit_i: i, exit_px: px, reason: String::new(), entry_kind: kind, signal_i: sig, open: false, exits: vec![] }
}
fn fin(mut t: Trade, i: usize, px: f64, sig: usize, why: &str) -> Trade { t.exit_i = i; t.exit_px = px; t.signal_i = sig; t.reason = why.into(); t }
fn mark_open(out: &mut Vec<Trade>, cur: Option<Trade>, x: &Ctx, to: usize) {
    if let Some(mut t) = cur { t.exit_i = to; t.exit_px = x.c[to]; t.signal_i = to; t.reason = "open".into(); t.open = true; out.push(t); }
}
fn hi(v: &[f64]) -> f64 { v.iter().copied().fold(f64::NEG_INFINITY, f64::max) }
fn lo(v: &[f64]) -> f64 { v.iter().copied().fold(f64::INFINITY, f64::min) }
fn ymd(d: &str) -> (i64, i64, i64) { (d[0..4].parse().unwrap_or(1970), d[5..7].parse().unwrap_or(1), d[8..10].parse().unwrap_or(1)) }
fn days(d: &str) -> i64 {
    let (y, m, dd) = ymd(d);
    let y2 = if m <= 2 { y - 1 } else { y };
    let era = y2.div_euclid(400); let yoe = y2 - era * 400;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + dd - 1;
    era * 146097 + yoe * 365 + yoe / 4 - yoe / 100 + doy - 719468
}

/// Always-in reversal driver: sig[t] = +1/-1 decided on bar t's close, filled at `px(t)` on bar `at(t)`.
fn reversal(x: &Ctx, from: usize, to: usize, sig: &[i8], next_open: bool, why: &str) -> Vec<Trade> {
    let mut out = vec![];
    let mut cur: Option<Trade> = None;
    for t in from..=to {
        let d = sig[t] as f64;
        if d == 0. || cur.as_ref().is_some_and(|c| c.dir == d) { continue; }
        let (i, px) = if next_open { if t >= to { continue; } (t + 1, x.o[t + 1]) } else { (t, x.c[t]) };
        if let Some(tr) = cur.take() { out.push(fin(tr, i, px, t, why)); }
        cur = Some(mk(d, i, px, t, if next_open { "open" } else { "close" }));
    }
    mark_open(&mut out, cur, x, to);
    out
}

pub fn catalogue() -> Vec<StrategyDef> {
    use Status::*;
    let d = |id, name, group, dir, status, rules, notes, claim| StrategyDef { id, name, group, session: 0, dir, status, needs_volume: false, needs_vix: false,
        needs_series: &[], book: BOOK, rules, notes, claim };
    vec![
        d("ctcn_dc40", "Donchian 40-day high/low reversal, next open (David Bryant)", "Issue 28 · Method Above/Below X-Days High/Low", "Long/Short", Live,
          &["E|\"if today's high exceeds the highest high of the past x days, you buy on tomorrow's open. Likewise, if today's low is less than the lowest low of the past x days, you sell on tomorrow's open\" (Issue 28, D. Bryant)",
            "X|\"Pure reversal system -- no stops, no exits, no delays, and the same value for x for both buys and sells\"; x = 40"],
          "Past x days exclude today. If both a new high and a new low occur on the same bar the bar is ignored (book silent). Book: one contract per commodity, 4/83-3/93, results table 'in print copy' only (not on the drive).", None),
        d("ctcn_trident", "Trident-style DP/TRP swing reversal with 4-day extremes (Brian Radke)", "Issue 04 · TRIDENT ... A Trading Strategy", "Long/Short", Live,
          &["F|\"Take nine days of prices, with today being day #1.\" #1 = (lowest low of days 3-6 - highest high of days 6-9) x 0.685 + highest high of days 1-4; #2 = (lowest low of days 3-6 - highest high of days 1-4) x 0.795 + highest high of days 1-4; #3 = (highest high of days 3-6 - lowest low of days 6-9) x 0.675 + lowest low of days 1-4; #4 = (highest high of days 3-6 - lowest low of days 1-4) x 0.795 + lowest low of days 1-4 (Issue 04, B. Radke)",
            "E|\"#1 goes above #2 and #3 above #4 and today's high is a NEW FOUR DAY HIGH, sell the next open. The opposite is true of course for long trades.\"",
            "X|\"Use this as a reversal system\""],
          "Readings (text): 'goes above' = the first day on which both #1 > #2 and #3 > #4 hold (not both on the previous day); new four-day high = today's high above the highs of the previous three days. Multipliers are the author's printed values (he notes Trident's own 0.625 can be changed).", None),
        d("ctcn_gann1428", "Range of the 14th/28th breakout on the close (Max Robinson)", "Issue 20 · Trading advice from a trader of 30+ yrs", "Long/Short", Live,
          &["E|\"Simply watch the market establish a range on the 14th or 28th. Then buy a close that closes above that range.\" \"Do the opposite to sell.\" (Issue 20, M. Robinson)",
            "X|\"Use a stop that is the first daily close under that range.\"",
            "F|\"If the 14th or 28th occurs on Saturday or Sunday, use the previous Friday's range.\""],
          "Entries and stops act on the close. The range day is the last trading day on or before the 14th/28th (the weekend rule extended to exchange holidays: book silent). A position's stop uses the range it was entered from; the newest range sets new entries. A stop-out close below the range also sells (the opposite rule).", None),
        d("ctcn_moon", "Full moon buy / new moon sell-and-reverse at the open (Harold Uney, after B.G. Pugh 1929)", "Issue 30 · Does the Moon help?", "Long/Short", Live,
          &["E|\"Buy the open on the day of the full moon and sell and reverse on the open on the day of the new moon.\" (Issue 30, H. Uney; from Burton G. Pugh, Trader's Instruction Book, 1929)",
            "X|Reversal at the next opposite phase; \"there are no commissions, slide or stops\""],
          "Phase dates from Meeus (Astronomical Algorithms ch.49, main periodic terms, accuracy ~minutes) as UTC dates; a phase on a non-trading day acts on the next trading day's open (book silent). Book result (Feb 1996 pork bellies) 'in print copy' only.", None),
        d("ctcn_thrust", "Thrust method for runaway markets (C.R., from Commodity Closeup)", "Issue 10", "Long/Short",
          NotTestable("entry only in a 'runaway' bull/bear market, which is a chart judgement, and no exit beyond the initial stop"),
          &["E|\"After the first down day, a buy stop is placed over the high of the down day ... lower the buy stop to just over the high of the next down day as long as prices retreat\" (Issue 10)"], "", None),
        d("ctcn_marcantel", "Marcantel candlestick validation rules", "Issue 49", "Long/Short", NotTestable("rules depend on candlestick judgement and 'projected extreme pivot points' not defined"),
          &["X|\"Rule 1: Price action must dictate a valid signal upon the day of entry or the trade is exited on the open the following day.\" (Issue 49)"], "", None),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "ctcn_dc40" => ("ctcn_dc40", "Donchian reversal (CTCN)", ""),
        "ctcn_trident" => ("ctcn_trident", "Trident DP/TRP (CTCN)", ""),
        "ctcn_gann1428" => ("ctcn_gann1428", "14th/28th range (CTCN)", ""),
        "ctcn_moon" => ("ctcn_moon", "Moon phases (CTCN)", ""),
        "ctcn_thrust" => ("ctcn_thrust", "Thrust method (CTCN)", ""),
        "ctcn_marcantel" => ("ctcn_marcantel", "Candlestick rules (CTCN)", ""),
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    Some(match id {
        "ctcn_dc40" => vec![p("x", "Look-back x (days)", 40., 5., 100., 5.)],
        "ctcn_trident" => vec![p("k1", "#1 multiplier", 0.685, 0.3, 1., 0.005), p("k2", "#2 multiplier", 0.795, 0.3, 1., 0.005), p("k3", "#3 multiplier", 0.675, 0.3, 1., 0.005),
                               p("k4", "#4 multiplier", 0.795, 0.3, 1., 0.005), p("ext", "New N-day high/low", 4., 2., 10., 1.)],
        "ctcn_gann1428" => vec![p("d1", "First range day of the month", 14., 1., 28., 1.), p("d2", "Second range day of the month", 28., 1., 31., 1.)],
        "ctcn_moon" => vec![p("flip", "1 = book (buy full / sell new); -1 = inverted", 1., -1., 1., 2.)],
        _ => return None,
    })
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    if from > to || to >= x.r.len() { return None; }
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(0., |d| d.def));
    let n = x.r.len();
    Some(match id {
        "ctcn_dc40" => {
            let xn = (g("x").round() as usize).max(1);
            let sig: Vec<i8> = (0..n).map(|t| if t < xn { 0 } else {
                let (up, dn) = (x.h[t] > hi(&x.h[t - xn..t]), x.l[t] < lo(&x.l[t - xn..t]));
                if up && !dn { 1 } else if dn && !up { -1 } else { 0 } }).collect();
            reversal(x, from, to, &sig, true, "opposite x-day breakout · next open")
        }
        "ctcn_trident" => {
            let ext = (g("ext").round() as usize).max(2);
            let w = |t: usize, a: usize, b: usize, h: bool| { // days a..=b, day 1 = t
                let s = if h { &x.h[t + 1 - b..=t + 1 - a] } else { &x.l[t + 1 - b..=t + 1 - a] }; if h { hi(s) } else { lo(s) } };
            let st: Vec<i8> = (0..n).map(|t| if t < 8 { 0 } else {
                let c1 = (w(t, 3, 6, false) - w(t, 6, 9, true)) * g("k1") + w(t, 1, 4, true);
                let c2 = (w(t, 3, 6, false) - w(t, 1, 4, true)) * g("k2") + w(t, 1, 4, true);
                let c3 = (w(t, 3, 6, true) - w(t, 6, 9, false)) * g("k3") + w(t, 1, 4, false);
                let c4 = (w(t, 3, 6, true) - w(t, 1, 4, false)) * g("k4") + w(t, 1, 4, false);
                if c1 > c2 && c3 > c4 { 1 } else if c1 < c2 && c3 < c4 { -1 } else { 0 } }).collect();
            let sig: Vec<i8> = (0..n).map(|t| if t < 9.max(ext) { 0 } else if st[t] == 1 && st[t - 1] != 1 && x.h[t] > hi(&x.h[t + 1 - ext..t]) { -1 }
                else if st[t] == -1 && st[t - 1] != -1 && x.l[t] < lo(&x.l[t + 1 - ext..t]) { 1 } else { 0 }).collect();
            reversal(x, from, to, &sig, true, "opposite Trident signal · next open")
        }
        "ctcn_gann1428" => gann(x, from, to, g("d1") as i64, g("d2") as i64),
        "ctcn_moon" => {
            let ph = moon_bars(x);
            let f = if g("flip") < 0. { -1 } else { 1 };
            // phase on bar t's date -> act at bar t's open: express as a signal on bar t-1 filled at the next open
            let mut sig = vec![0i8; n];
            for t in 1..n { sig[t - 1] = ph[t] * f; }
            if from > 0 { reversal(x, from - 1, to, &sig, true, "opposite moon phase · open") } else { reversal(x, 0, to, &sig, true, "opposite moon phase · open") }
        }
        _ => return None,
    })
}

fn gann(x: &Ctx, from: usize, to: usize, d1: i64, d2: i64) -> Vec<Trade> {
    let n = x.r.len();
    // range bar: last trading day on or before the 14th/28th of each month
    let mut is_rng = vec![false; n];
    for t in 0..n {
        let (y, m, d) = ymd(&x.r[t].date);
        for dd in [d1, d2] {
            if d > dd { continue; }
            let nxt = if t + 1 < n { Some(ymd(&x.r[t + 1].date)) } else { None };
            // t is the range bar if the next bar is past day dd of this month (or in a later month)
            if nxt.is_some_and(|(y2, m2, d2_)| (y2, m2) > (y, m) || d2_ > dd) { is_rng[t] = true; }
        }
    }
    let mut out = vec![];
    let mut cur: Option<(Trade, f64)> = None; // trade, stop level (range low for longs / high for shorts)
    let mut rng: Option<(f64, f64)> = None;
    for t in 0..=to {
        if let Some((tr, st)) = cur.take() {
            let hit = if tr.dir > 0. { x.c[t] < st } else { x.c[t] > st };
            if hit && t > tr.legs[0].i { out.push(fin(tr, t, x.c[t], t, "first close beyond the range · close")); } else { cur = Some((tr, st)); }
        }
        if let (Some((h, l)), true, None) = (rng, t >= from, cur.as_ref()) {
            if x.c[t] > h { cur = Some((mk(1., t, x.c[t], t, "close"), l)); } else if x.c[t] < l { cur = Some((mk(-1., t, x.c[t], t, "close"), h)); }
        }
        if is_rng[t] { rng = Some((x.h[t], x.l[t])); }
    }
    mark_open(&mut out, cur.map(|c| c.0), x, to);
    out
}

// ---------------------------------------------------------------- moon phases (Meeus ch.49)
fn phase_jde(k: f64, full: bool) -> f64 {
    let t = k / 1236.85;
    let r = |d: f64| d.to_radians();
    let jde = 2451550.09766 + 29.530588861 * k + 0.00015437 * t * t - 0.00000015 * t.powi(3) + 0.00000000073 * t.powi(4);
    let e = 1. - 0.002516 * t - 0.0000074 * t * t;
    let m = r(2.5534 + 29.1053567 * k - 0.0000014 * t * t);
    let mp = r(201.5643 + 385.81693528 * k + 0.0107582 * t * t);
    let f = r(160.7108 + 390.67050284 * k - 0.0016118 * t * t);
    let om = r(124.7746 - 1.56375588 * k + 0.0020672 * t * t);
    let (a, b, c, dd) = if full { (-0.40614, 0.17302, 0.01614, 0.01043) } else { (-0.40720, 0.17241, 0.01608, 0.01039) };
    let (e1, e2, e3) = if full { (0.00734, -0.00515, 0.00209) } else { (0.00739, -0.00514, 0.00208) };
    jde + a * mp.sin() + b * e * m.sin() + c * (2. * mp).sin() + dd * (2. * f).sin() + e1 * e * (mp - m).sin() + e2 * e * (mp + m).sin()
        + e3 * e * e * (2. * m).sin() - 0.00111 * (mp - 2. * f).sin() - 0.00057 * (mp + 2. * f).sin() + 0.00056 * e * (2. * mp + m).sin()
        - 0.00042 * (3. * mp).sin() + 0.00042 * e * (m + 2. * f).sin() + 0.00038 * e * (m - 2. * f).sin() - 0.00024 * e * (2. * mp - m).sin()
        - 0.00017 * om.sin()
}
/// +1 on the first bar dated on/after a full-moon (UTC) date, -1 for a new moon.
fn moon_bars(x: &Ctx) -> Vec<i8> {
    let n = x.r.len();
    let mut out = vec![0i8; n];
    if n == 0 { return out; }
    let dd: Vec<i64> = x.r.iter().map(|r| days(&r.date)).collect();
    let k0 = ((dd[0] as f64 / 365.2425 + 1970. - 2000.) * 12.3685).floor() as i64 - 2;
    let k1 = ((dd[n - 1] as f64 / 365.2425 + 1970. - 2000.) * 12.3685).ceil() as i64 + 2;
    for k in k0..=k1 {
        for (off, tag) in [(0.0, -1i8), (0.5, 1i8)] {
            let day = (phase_jde(k as f64 + off, off > 0.) - 2440587.5).floor() as i64;
            if let Some(i) = dd.iter().position(|&v| v >= day) { if i > 0 || dd[0] == day { out[i] = tag; } }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    fn row(d: &str, o: f64, h: f64, l: f64, c: f64) -> Row { Row { ticker: "T".into(), date: d.into(), open: o, high: h, low: l, close: c, volume: None } }
    #[test]
    fn known_moon_dates() {
        // full moon 2000-01-21 04:40 UT, new moon 2000-02-05 13:03 UT
        let r: Vec<Row> = ["2000-01-20", "2000-01-21", "2000-01-24", "2000-02-04", "2000-02-07"].iter().map(|d| row(d, 1., 1., 1., 1.)).collect();
        let x = Ctx::new(&r, &r, &[]);
        let m = moon_bars(&x);
        assert_eq!(m, vec![0, 1, 0, 0, -1]); // Feb 5 is a Saturday -> Monday Feb 7
    }
    #[test]
    fn gann_range_and_close_entries() {
        let mut r = vec![row("2001-03-13", 10., 11., 9., 10.), row("2001-03-14", 10., 12., 8., 10.), row("2001-03-15", 10., 13., 10., 12.5),
                         row("2001-03-16", 12., 12., 7., 7.5)];
        r.push(row("2001-03-19", 7., 8., 6., 7.));
        let x = Ctx::new(&r, &r, &[]);
        let ts = gann(&x, 0, 4, 14, 28);
        assert_eq!(ts[0].dir, 1.); assert_eq!(ts[0].legs[0].i, 2); assert_eq!(ts[0].exit_i, 3); assert_eq!(ts[0].exit_px, 7.5);
        assert_eq!(ts[1].dir, -1.); assert_eq!(ts[1].legs[0].i, 3);
    }
}
