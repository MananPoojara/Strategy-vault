//! Anthony W. Warren, alpha-beta trend following, Technical Analysis of Stocks & Commodities:
//! V3:3 "A Trend Following Method for All Seasons, Part 1" (CONC.PDF), V3:8 "Part II. Implementation
//! Using Alpha-Beta Filtering" with the BASIC listing (IMPL.PDF), V4:9 "revisited" (ALPA.PDF),
//! V7:4 "Version II, tuning for performance" (ALPHABE.PDF) with its QuickBASIC listing "Alpha-beta, Version II" (SIDEBA.pdf in the
//! same V07/C04 folder of the S&C DVD; read and coded by lane K41, 4 Nov 2026, decision 205; independent check
//! authors/titles-1030/K41w/k41_ref.py).
//! Source copies: Books/Anthony W. Warren/. Notes: library/awab/README.md.
//!
//! The 1985 BASIC routine is run at every decision point on the data up to that point (as the author
//! did, V3:8 p.1/p.5; V7:4 explains the routine fitted the channel on the whole file, so only the
//! value at the last point is free of future data). Only that last point is used.
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

const BOOK: &str = "awab";
/// Admitted ids (authors/lanea/singles/decisions.json).
pub const REVIEWED_LANEA: &[&str] = &["awab_ls", "awab_long", "awab_v2"];
pub fn reviewed_lanea(id: &str) -> bool { REVIEWED_LANEA.contains(&id) }

pub fn catalogue() -> Vec<StrategyDef> {
    let make = |id, name, dir, status, rules: &'static [&'static str], notes| StrategyDef {
        id, name, group: "S&C 1985-1989 · Alpha-beta trend following", session: 0, dir, status,
        needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim: None,
    };
    const R: [&str; 7] = [
        "F|Weekly closing prices (\"MS=4 and ML=12 usually provide good trial values for weekly sampled stock and mutual fund data\", V3:8 p.3). Week close = close of the week's last trading day.",
        "F|Alpha-beta filter (V3:8 p.2, listing lines 270-290, 340-350): RE = z - XF; XF += AL*RE; DF += BE*RE; ...; XF += DF, with AL = 2/(M+1.5), BE = AL/(M+0.5); both filters start from a least-squares line over the first ML points (lines 200-220).",
        "F|Trend channels (lines 280-320): Y(i-MS) = XF - MS*DF, last MS points forecast Y(N+1-k) = x(N) - (k-1)v(N); SIG = YS*SQR(SUM(z-Y)^2/NUM); TU = Y + SIG, TL = Y - SIG. Defaults MS=4, YS=2 (V3:8 p.5; line 180 forces YS >= 2).",
        "F|Trading filter (line 350): Y(i) = XF - ML*DF, ML=12 (V3:8 p.5).",
        "E|\"If the trading filter lies below the lower trend channel, an uptrend market is in progress and one should buy or hold long\"; \"above the upper trend channel ... sell or hold short\" (V3:8 p.1; V3:3 p.1; V4:9 p.1; V7:4 p.1)",
        "X|\"If the trading filter lies between the trend channels, no trend is evident and one should liquidate any current market positions.\" (V3:8 p.1)",
        "X|The routine is re-run on the data up to each week's close and only its last point is read; the order fills at the next trading day's open [book silent on fill]. No trades in the first MS+ML weeks of the file (\"exclude the first MS and ML data points\", V7:4 p.2). The file starts at the selected 'from' date.",
    ];
    vec![
        make("awab_ls", "Alpha-beta trend channel, long/short (1985 routine)", "Long/Short", Status::Live, &R,
            "Author's cases: weekly Fidelity funds, Tandy, S&P 400 (no performance statistics printed; signal dates only, see README). Run on the index instead of the fund/S&P 400. Short side is in every statement of the rules; V7:4 says shorts may need shorter filters (no values given), so the same filters are used. Non-default: p.daily=1 samples daily closes (\"daily or weekly closing price\", V7:4 p.1)."),
        make("awab_long", "Alpha-beta trend channel, long only (1985 routine)", "Long", Status::Live, &R,
            "Same rules with the short recommendations ignored, as the author did for mutual funds (\"because mutual funds cannot be traded short, the short recommendations are ignored here\", V7:4 p.1)."),
        make("awab_v2", "Alpha-beta Version II (1989 revised code), long/short", "Long/Short", Status::Live,
            &["F|Weekly closing prices; defaults \"XMS = 6!: XML = 15!: YS = 2.5\" (SIDEBA.pdf p.2, line 145; \"A good default for weekly stock data is MS=6 ... ML=15 ... YS=2.5\", ALPHABE.PDF p.5). Week close = close of the week's last trading day.",
              "F|Lines 200-220: least-squares line over the first ML points; lines 270-290: alpha-beta filter over the first MS+ML points (AL = 2/(MS+1.5), BE = AL/(MS+.5)), Y(I-MS) = XF - MS*DF; line 310: VAR = mean of (X-Y)^2 over the first ML points; line 330: AP = 1/ML, Y(T1) = XF - J*DF for the next MS points, VAR = VAR + AP*((X-Y)^2 - VAR) (SIDEBA.pdf p.2-3)",
              "F|Line 334: over the remaining points RE = X - XF: XF = XF + AL*RE: DF = DF + BE*RE: Y(I) = XF: XF = XF + DF; VAR = VAR + AP*((X(I) - Y(I))^2 - VAR): SIG = YS*SQR(VAR); upper/lower trend line = Y +/- SIG (SIDEBA.pdf p.3)",
              "F|Lines 340-350: trading filter = ML-lag alpha-beta filter, X(IT,I) = XF - ML*DF with AL = 2/(ML+1.5), BE = AL/(ML+.5) (SIDEBA.pdf p.3)",
              "E|\"If the trading filter lies below the lower trend channel, then an uptrend is in progress and you should buy or remain long ... If the trading filter lies above the upper trend channel a downtrend is in progress and you should sell or remain short.\" (ALPHABE.PDF p.2)",
              "X|\"If the trading filter lies between the trend channels, then no trend is evident and you should be out.\" (ALPHABE.PDF p.2); \"you should exclude the first MS and ML data points from a profitability analysis\" (p.5); the order fills at the next trading day's open [book silent on fill]"],
            "Version II computes the channel from the data up to each point only (\"The revised alpha-beta method gives realistic trading signals based on data up to the time of decision\", ALPHABE.PDF p.5), so one pass over the file is free of future data after the first MS+ML points. Line 180 floors YS at 1. One set of filters for both sides: the article advises separate (shorter) filters for short/cover decisions but gives no values. Author's case: S&P 400 weekly, June 1986 - December 1988, ML = 15: buy 16 January 1987, sell 9 October 1987, no further signals (ALPHABE.PDF p.5). Non-default: p.daily = 1 samples daily closes."),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let v = match id {
        "awab_ls" => "1985 routine · long/short",
        "awab_long" => "1985 routine · long only",
        "awab_v2" => "Version II · 1989 listing",
        _ => return None,
    };
    Some(("awab_alphabeta", "Warren alpha-beta trend channel", v))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    match id {
        "awab_ls" | "awab_long" => Some(vec![
            p("ms", "MS: lags for trend channels", 4., 2., 12., 1.),
            p("ml", "ML: lags for trading filter (>= MS)", 12., 2., 30., 1.),
            p("ys", "YS: channel width in sigma (>= 2)", 2., 2., 4., 0.25),
            p("daily", "Sample daily closes instead of weekly (0 = weekly, book default)", 0., 0., 1., 1.),
        ]),
        "awab_v2" => Some(vec![
            p("ms", "MS: lags for trend channels", 6., 2., 12., 1.),
            p("ml", "ML: lags for trading filter (>= MS)", 15., 2., 30., 1.),
            p("ys", "YS: channel width in sigma (>= 1)", 2.5, 1., 4., 0.25),
            p("daily", "Sample daily closes instead of weekly (0 = weekly, book default)", 0., 0., 1., 1.),
        ]),
        _ => None,
    }
}

/// Days since 1970-01-01 for "YYYY-MM-DD".
fn day_number(date: &str) -> i64 {
    let (y, m, d): (i64, i64, i64) = (date[0..4].parse().unwrap_or(0), date[5..7].parse().unwrap_or(1), date[8..10].parse().unwrap_or(1));
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * (m + if m > 2 { -3 } else { 9 }) + 2) / 5 + d - 1;
    era * 146_097 + (yoe * 365 + yoe / 4 - yoe / 100 + doy) - 719_468
}
fn week_id(date: &str) -> i64 { (day_number(date) + 3).div_euclid(7) }

/// Last trading day of its week: next bar in another week; the last bar of the data only if a Friday.
fn week_end(x: &Ctx, i: usize) -> bool {
    match x.r.get(i + 1) {
        Some(n) => week_id(&n.date) != week_id(&x.r[i].date),
        None => (day_number(&x.r[i].date) + 3).rem_euclid(7) == 4,
    }
}

/// The 1985 BASIC routine on z[0..N]; returns (TL, TU, trading filter) at the last point N.
pub fn routine(z: &[f64], ms: usize, ml: usize, ys: f64) -> (f64, f64, f64) {
    let n = z.len();
    // lines 200-220: least-squares line over the first ML points, value at point 1 and slope
    let c = (ml as f64 + 1.) / 2.;
    let (mut xb, mut dx) = (0., 0.);
    for i in 1..=ml {
        xb += z[i - 1];
        dx += z[i - 1] * (i as f64 - c);
    }
    let u = ml as f64 * (ml as f64 * ml as f64 - 1.) / 12.;
    xb /= ml as f64;
    dx /= u;
    xb += (1. - c) * dx;
    // lines 270-290
    let al = 2. / (ms as f64 + 1.5);
    let be = al / (ms as f64 + 0.5);
    let (mut xf, mut df) = (xb, dx);
    let mut y = vec![0.; n];
    for i in 1..=n {
        let re = z[i - 1] - xf;
        xf += al * re;
        df += be * re;
        if i > ms { y[i - ms - 1] = xf - ms as f64 * df; }
        xf += df;
    }
    for i in (1..=ms).rev() { y[n - i] = xf - i as f64 * df; }
    // lines 310-320
    let sig = ys * (z.iter().zip(&y).map(|(a, b)| (a - b).powi(2)).sum::<f64>() / n as f64).sqrt();
    // lines 340-350
    let al = 2. / (ml as f64 + 1.5);
    let be = al / (ml as f64 + 0.5);
    let (mut xf, mut df, mut tf) = (xb, dx, f64::NAN);
    for &zi in z {
        let re = zi - xf;
        xf += al * re;
        df += be * re;
        tf = xf - ml as f64 * df;
        xf += df;
    }
    (y[n - 1] - sig, y[n - 1] + sig, tf)
}

/// The 1989 Version II routine (SIDEBA.pdf) over z[0..N]: (TL, TU, trading filter) at every point; NaN before point ML+MS.
pub fn routine2(z: &[f64], ms: usize, ml: usize, ys: f64) -> Vec<(f64, f64, f64)> {
    let n = z.len();
    let mut out = vec![(f64::NAN, f64::NAN, f64::NAN); n];
    if n < ml + ms || ml < 2 { return out; }
    let c = (ml as f64 + 1.) / 2.;
    let (mut xb, mut dx) = (0., 0.);
    for i in 1..=ml { xb += z[i - 1]; dx += z[i - 1] * (i as f64 - c); }
    dx /= ml as f64 * (ml as f64 * ml as f64 - 1.) / 12.;
    xb = xb / ml as f64 + (1. - c) * dx;
    let (al, be) = (2. / (ms as f64 + 1.5), 2. / (ms as f64 + 1.5) / (ms as f64 + 0.5));
    let (mut xf, mut df) = (xb, dx);
    let mut y = vec![0.; n + 1]; // 1-based
    for i in 1..=ml + ms {
        let re = z[i - 1] - xf; xf += al * re; df += be * re;
        if i > ms { y[i - ms] = xf - ms as f64 * df; }
        xf += df;
    }
    let mut var = (1..=ml).map(|i| (z[i - 1] - y[i]).powi(2)).sum::<f64>() / ml as f64;
    let t = ml + ms;
    let ap = 1. / ml as f64;
    let mut sig = vec![f64::NAN; n + 1];
    for i in 1..=ms {
        let t1 = t + i - ms;
        y[t1] = xf - (ms - i + 1) as f64 * df;
        var += ap * ((z[t1 - 1] - y[t1]).powi(2) - var);
        sig[t1] = ys * var.sqrt();
    }
    for i in t + 1..=n {
        let re = z[i - 1] - xf; xf += al * re; df += be * re;
        y[i] = xf; xf += df;
        var += ap * ((z[i - 1] - y[i]).powi(2) - var);
        sig[i] = ys * var.sqrt();
    }
    let (al, be) = (2. / (ml as f64 + 1.5), 2. / (ml as f64 + 1.5) / (ml as f64 + 0.5));
    let (mut xf, mut df) = (xb, dx);
    for i in 1..=n {
        let re = z[i - 1] - xf; xf += al * re; df += be * re;
        let tf = xf - ml as f64 * df;
        xf += df;
        if i >= t { out[i - 1] = (y[i] - sig[i], y[i] + sig[i], tf); }
    }
    out
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    if from > to || to >= x.r.len() { return None; }
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).unwrap().def);
    let v2 = id == "awab_v2";
    let (ms, ml, ys) = (g("ms").round().max(2.) as usize, g("ml").round().max(2.) as usize, if v2 { g("ys").max(1.) } else { g("ys").max(2.) });
    if ms > ml { return None; } // listing line 170
    let (daily, long_only) = (g("daily") >= 0.5, id == "awab_long");
    let pts: Vec<usize> = (from..=to).filter(|&i| daily || week_end(x, i)).collect();
    let all: Vec<f64> = pts.iter().map(|&i| x.c[i]).collect();
    let bands = if v2 { routine2(&all, ms, ml, ys) } else { vec![] };
    let mut z: Vec<f64> = vec![];
    let mut out = vec![];
    let mut cur: Option<Trade> = None;
    for (k, &i) in pts.iter().enumerate() {
        z.push(x.c[i]);
        if z.len() < ml + ms || i >= to { continue; }
        let (tl, tu, tf) = if v2 { bands[k] } else { routine(&z, ms, ml, ys) };
        let want = if tf < tl { 1. } else if tf > tu && !long_only { -1. } else { 0. };
        let have = cur.as_ref().map_or(0., |t| t.dir);
        if want == have { continue; }
        let (j, px) = (i + 1, x.o[i + 1]);
        if let Some(mut t) = cur.take() {
            t.exit_i = j; t.exit_px = px; t.signal_i = i;
            t.reason = if want == 0. { "Filter inside the channel · next open".into() } else { "Filter crossed the opposite channel · next open".into() };
            out.push(t);
        }
        if want != 0. {
            cur = Some(Trade { dir: want, legs: vec![Leg { i: j, px, w: 1. }], exit_i: j, exit_px: px, reason: String::new(),
                entry_kind: "open", signal_i: i, open: false, exits: vec![] });
        }
    }
    if let Some(mut t) = cur {
        t.exit_i = to; t.exit_px = x.c[to]; t.signal_i = to; t.reason = "open".into(); t.open = true;
        out.push(t);
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn routine_tracks_a_straight_line_exactly() {
        // On a straight line the least-squares start is exact, every residual is 0: channel = price, filter lags by ML*slope
        let z: Vec<f64> = (0..40).map(|i| 10. + 2. * i as f64).collect();
        let (tl, tu, tf) = routine(&z, 4, 12, 2.);
        assert!((tl - 88.).abs() < 1e-9 && (tu - 88.).abs() < 1e-9);
        assert!((tf - (88. - 24.)).abs() < 1e-9);
    }
    #[test]
    fn version2_tracks_a_straight_line_exactly() {
        let z: Vec<f64> = (0..60).map(|i| 10. + 2. * i as f64).collect();
        let b = routine2(&z, 6, 15, 2.5);
        assert!(b[19].0.is_nan() && !b[20].0.is_nan());
        for k in 20..60 { let (tl, tu, tf) = b[k]; assert!((tl - z[k]).abs() < 1e-9 && (tu - z[k]).abs() < 1e-9 && (tf - (z[k] - 30.)).abs() < 1e-9); }
    }
    #[test]
    fn first_step_matches_hand_calculation() {
        // ML=2 fit over [1,3]: mean 2, C=1.5, U=0.5, DX=(1*-0.5+3*0.5)/0.5=2, XB=2-0.5*2=1 (line at point 1)
        let z = [1., 3., 2., 7.];
        let (al, be) = (2. / 3.5, (2. / 3.5) / 2.5);
        let (mut xf, mut df) = (1., 2.);
        let mut tf = 0.;
        for &v in &z { let re = v - xf; xf += al * re; df += be * re; tf = xf - 2. * df; xf += df; }
        assert!((routine(&z, 2, 2, 2.).2 - tf).abs() < 1e-12);
    }
}

// Instrument admission: Anthony W. Warren (awab, lane A) 2026-10-26 (lane B) block, moved from instrument_policy.rs on 31 Oct 2026 (lane ST).
// Admission only for the module REVIEWED lists (authors/lanea/singles/decisions.json); kept in one block to merge.
// Runs on actual indices, ETFs and stocks, US and India (index_etf_or_stock, nifty50 basket, declared application).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book=="awab" && crate::books::awab::reviewed_lanea(def.id),
    label: "Warren's alpha-beta trend channel was written for weekly mutual-fund, stock and S&P 400 prices; Runs on indices, ETFs and single stocks (US and India) are a declared application, one unit per trade.",
    basis: "Anthony W. Warren / Technical Analysis of Stocks & Commodities V3:3, V3:8, V7:4 (1985-1989)",
};
