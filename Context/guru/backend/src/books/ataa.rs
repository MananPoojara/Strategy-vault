//! Australian Technical Analysts Association (ATAA) Journal / Newsletter, Aug 1990 - Nov 1998. Book id `ataa`.
//! Source: research drive "Trading Knowledge II/@NewsLetters-Mags/Australian Tech Analyst Assoc (ATAA) Journal
//! 1990 - 1998/" (multi-volume RAR SFX of scanned PDFs, one per issue; OCR'd for reading). Refs = issue + printed page.
//! Almost all articles are commentary, forecasts (Elliott, Gann, cycles) or indicator interpretation; only rules with
//! a complete entry and exit are Live. Futures/currency rules run on the index (instrument swap only).
//! Sources and checks: library/ataa/.
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "ataa";

fn mk(dir: f64, i: usize, px: f64, kind: &'static str) -> Trade {
    Trade { dir, legs: vec![Leg { i, px, w: 1. }], exit_i: i, exit_px: px, reason: String::new(), entry_kind: kind, signal_i: i, open: false, exits: vec![] }
}

fn days(d: &str) -> i64 {
    let (y, m, dd): (i64, i64, i64) = (d[0..4].parse().unwrap_or(1970), d[5..7].parse().unwrap_or(1), d[8..10].parse().unwrap_or(1));
    let y2 = if m <= 2 { y - 1 } else { y };
    let era = y2.div_euclid(400);
    let yoe = y2 - era * 400;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + dd - 1;
    era * 146097 + yoe * 365 + yoe / 4 - yoe / 100 + doy - 719468
}

/// True new / full moon (Meeus, Astronomical Algorithms ch.49, main periodic terms), JDE (TT).
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

/// +1 on the first bar dated on/after a new-moon (UTC) date, -1 for a full moon.
fn moon_bars(x: &Ctx) -> Vec<i8> {
    let n = x.r.len();
    let mut out = vec![0i8; n];
    if n == 0 { return out; }
    let dd: Vec<i64> = x.r.iter().map(|r| days(&r.date)).collect();
    let k0 = ((dd[0] as f64 / 365.2425 + 1970. - 2000.) * 12.3685).floor() as i64 - 2;
    let k1 = ((dd[n - 1] as f64 / 365.2425 + 1970. - 2000.) * 12.3685).ceil() as i64 + 2;
    for k in k0..=k1 {
        for (off, tag) in [(0.0, 1i8), (0.5, -1i8)] {
            let day = (phase_jde(k as f64 + off, off > 0.) - 2440587.5).floor() as i64;
            if let Some(i) = dd.iter().position(|&v| v >= day) { if i > 0 || dd[0] == day { out[i] = tag; } }
        }
    }
    out
}

pub fn catalogue() -> Vec<StrategyDef> {
    use Status::*;
    let d = |id, name, group, dir, status, rules, notes, claim| StrategyDef { id, name, group, session: 0, dir, status, needs_volume: false,
        needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim };
    vec![
        d("ataa_moon", "Sheer Lunacy: sell the close of the Full Moon, buy the close of the New Moon (Ivan Krastins)", "Dec 1990 · Sheer Lunacy (p.11-12)", "Long/Short", Live,
          &["E|\"Based on selling on the close on the day of the Full Moon and then reversing to long on the close of the day of the New Moon, a true stop and reverse system\" (Dec 1990 p.11)",
            "X|\"No rules, other than to buy/sell on the close of the appropriate moon phase day, have been used.\" (p.11); always in after the first phase",
            "F|Phase dates: Meeus, Astronomical Algorithms ch.49 (true phases), as UTC calendar dates; a phase on a non-trading day acts on the next trading day's close [book silent]",
            "F|p.flip=-1 (labelled option): buy the full moon / sell the new moon"],
          "Book test: AUD/USD futures (IMM), chart Feb-Aug 1990; 'hypothetical ... slippage, margin calls and brokerage have not been factored'. Comparison in the same article: best 7-day EMA price-penetration system 13 wins / 24 losses, net -0.42 points. No AUD futures data in the vault: run on the indices (instrument swap). Related but different: ctcn_moon (Pugh: buy full, sell new, at the open).",
          crate::connors::claim("16 (8 win / 8 loss)", "50%", "+0.32 pts (gain/loss ratio 1.61)", "", "AUD/USD IMM futures, 1990 (chart Feb-Aug)")),
        d("ataa_ma3_5_15_30", "Three moving averages 5/15/30: long after the 5 then the 15 cross above the 30, out when the 5 crosses the 15 (Vic Nevill)", "Nov 1998 · Moving Averages (p.5-8)", "Long/Short", Live,
          &["E|\"probably the most popular is the 5, 15, 30 period moving average. Using this you would go long after firstly, the 5 line and secondly, the 15 line, have crossed from below to above the 30 line. You would go short when the reverse occurred.\" (Nov 1998 p.8): long when the 15 crosses above the 30 while the 5 is already above it; short mirrored",
            "X|\"you would close a long position at the first cross down, that is, when the 5 crosses below the 15 and close a short position at the first cross up, that is, when the 5 crosses above the 15\" (p.8)",
            "F|Simple moving averages of the close (the article's MOVAVS chart, p.7) on weekly bars (the article's only example uses 5 and 30 week averages, p.6-7); p.daily=1 (labelled option) = daily bars",
            "F|Signals on the week's close are filled at the next week's first open [book silent]; a new entry needs a new 15/30 cross (\"don't go long until all moving averages are telling you to go long\")"],
          "The article also advises not to 'rely merely on the filtered moving average signal in isolation, but consider other factors ... price reversal signals, support and resistance' (p.8): general advice, not part of the stated model. Book example: NAB weekly (no statistics). Run on the indices (instrument swap).",
          None),
        d("ataa_coppock", "Coppock Indicator buy (negative & ticks up) / sell (positive & ticks down), monthly (Colin Nicholson)", "May 1995 · Coppock Indicator Revisited (p.5-8)", "Long", Live,
          &["F|\"Calculate a 14 month Rate of Change; Calculate an 11 month Rate of Change; Add the ROC(14) and the ROC(11); Calculate a 10 month Weighted Moving Average of ROC(14)+ROC(11)\"; MetaStock: MOV(roc(CLOSE,14,%)+roc(Close,11,%),10,WEIGHTED) (Mar 1995 p.4)",
            "E|\"Buy signals are generated when the Coppock Indicator is in negative territory and ticks upward.\" (May 1995 p.6)",
            "X|\"a sell signal is given when the Coppock Indicator is in positive territory and ticks down\" (p.6); the author tested it on the DJIA and All Ordinaries (p.8)",
            "F|\"He only calculated it on monthly observations\" (p.5): month-end closes; the signal is known at the month's close and filled at the next month's first open [book silent on fills]"],
          "Coppock himself gave buy signals only; the sell rule is the article's 'opposite to a buy signal', which the author found gave 'so many false signals' (p.8). Book result (DJIA, monthly, 1904-1995): buy signals followed by an advance in ... 1970, 1975, 1978, 1982, 1985, 1988 (22 good, 9 poor, none poor after 1957); no trade statistics printed. Run on DJI/SPX and the Indian indices (instrument swap).",
          None),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "ataa_moon" => ("ataa_moon", "Moon phases (ATAA)", ""),
        "ataa_coppock" => ("ataa_coppock", "Coppock Indicator (ATAA)", ""),
        "ataa_ma3_5_15_30" => ("ataa_ma3_5_15_30", "Three moving averages 5/15/30 (ATAA)", ""),
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    Some(match id {
        "ataa_moon" => vec![p("flip", "1 = book (sell full / buy new); -1 = inverted", 1., -1., 1., 2.)],
        "ataa_ma3_5_15_30" => vec![p("a", "Short MA", 5., 2., 20., 1.), p("b", "Middle MA", 15., 5., 40., 1.), p("c", "Long MA", 30., 10., 100., 1.),
                                   p("daily", "0 = weekly bars (book example); 1 = daily bars", 0., 0., 1., 1.)],
        "ataa_coppock" => vec![p("roc1", "ROC months (long)", 14., 3., 36., 1.), p("roc2", "ROC months (short)", 11., 3., 36., 1.),
                               p("wma", "Weighted MA months", 10., 2., 24., 1.)],
        _ => return None,
    })
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    if from > to || to >= x.r.len() { return None; }
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(0., |d| d.def));
    Some(match id {
        "ataa_moon" => {
            let ph = moon_bars(x);
            let f: i8 = if g("flip") < 0. { -1 } else { 1 };
            let mut out = vec![];
            let mut cur: Option<Trade> = None;
            for t in from..=to {
                let s = ph[t] * f;
                if s == 0 || cur.as_ref().is_some_and(|c| c.dir == s as f64) { continue; }
                if let Some(mut tr) = cur.take() {
                    tr.exit_i = t; tr.exit_px = x.c[t]; tr.signal_i = t; tr.reason = "opposite moon phase · close".into();
                    out.push(tr);
                }
                cur = Some(mk(s as f64, t, x.c[t], "close"));
            }
            if let Some(mut tr) = cur { tr.exit_i = to; tr.exit_px = x.c[to]; tr.signal_i = to; tr.reason = "open".into(); tr.open = true; out.push(tr); }
            out
        }
        "ataa_ma3_5_15_30" => {
            let daily = g("daily") >= 0.5;
            // period closes and the bar on which each period's signal is filled
            let (pc, fill): (Vec<f64>, Vec<usize>) = if daily { (x.c.clone(), (0..x.r.len()).map(|t| t + 1).collect()) } else {
                let wk: Vec<i64> = x.r.iter().map(|r| (days(&r.date) - 4).div_euclid(7)).collect();
                let (mut pc, mut fill) = (vec![], vec![]);
                for t in 0..x.r.len() {
                    if t > 0 && wk[t] != wk[t - 1] { fill.push(t); }
                    if t == 0 || wk[t] != wk[t - 1] { pc.push(x.c[t]); } else { *pc.last_mut().unwrap() = x.c[t]; }
                }
                fill.push(usize::MAX); // the current week is not complete
                (pc, fill)
            };
            let sma = |n: usize| crate::connors::sma(&pc, n.max(1));
            let (a, b, c) = (sma(g("a").round() as usize), sma(g("b").round() as usize), sma(g("c").round() as usize));
            let mut out = vec![];
            let mut cur: Option<Trade> = None;
            for k in 1..pc.len() {
                let t = fill[k];
                if t > to || t >= x.r.len() { break; }
                if t < from { continue; }
                let px = x.o[t];
                if let Some(tr) = cur.as_ref() {
                    let out_sig = if tr.dir > 0. { a[k] < b[k] && a[k - 1] >= b[k - 1] } else { a[k] > b[k] && a[k - 1] <= b[k - 1] };
                    if out_sig {
                        let mut tr = cur.take().unwrap();
                        tr.exit_i = t; tr.exit_px = px; tr.signal_i = t - 1;
                        tr.reason = (if tr.dir > 0. { "5 crosses below 15 · next open" } else { "5 crosses above 15 · next open" }).into();
                        out.push(tr);
                    }
                }
                if cur.is_none() {
                    let up = b[k] > c[k] && b[k - 1] <= c[k - 1] && a[k] > c[k];
                    let dn = b[k] < c[k] && b[k - 1] >= c[k - 1] && a[k] < c[k];
                    if up || dn { let mut tr = mk(if up { 1. } else { -1. }, t, px, "open"); tr.signal_i = t - 1; cur = Some(tr); }
                }
            }
            if let Some(mut tr) = cur { tr.exit_i = to; tr.exit_px = x.c[to]; tr.signal_i = to; tr.reason = "open".into(); tr.open = true; out.push(tr); }
            out
        }
        "ataa_coppock" => {
            let (cop, first) = coppock_monthly(x, g("roc1").round() as usize, g("roc2").round() as usize, g("wma").round() as usize);
            let mut out = vec![];
            let mut cur: Option<Trade> = None;
            for (m, &t) in first.iter().enumerate() {
                if m < 2 || t < from || t > to { continue; }
                let (c0, c1) = (cop[m - 1], cop[m - 2]); // last completed month and the one before
                if cur.is_none() && c0 < 0. && c0 > c1 { cur = Some(mk(1., t, x.o[t], "open")); cur.as_mut().unwrap().signal_i = t - 1; }
                else if cur.is_some() && c0 > 0. && c0 < c1 {
                    let mut tr = cur.take().unwrap();
                    tr.exit_i = t; tr.exit_px = x.o[t]; tr.signal_i = t - 1; tr.reason = "Coppock positive and ticks down · next open".into();
                    out.push(tr);
                }
            }
            if let Some(mut tr) = cur { tr.exit_i = to; tr.exit_px = x.c[to]; tr.signal_i = to; tr.reason = "open".into(); tr.open = true; out.push(tr); }
            out
        }
        _ => return None,
    })
}

/// Coppock on month-end closes. Returns (coppock per month, first bar index of each month). Month m's value is
/// complete only when month m+1 has started, so it is used at first[m+1].
fn coppock_monthly(x: &Ctx, r1: usize, r2: usize, w: usize) -> (Vec<f64>, Vec<usize>) {
    let mut first = vec![];
    let mut mc: Vec<f64> = vec![];
    for t in 0..x.r.len() {
        if t == 0 || x.r[t].date[..7] != x.r[t - 1].date[..7] { first.push(t); mc.push(x.c[t]); } else { *mc.last_mut().unwrap() = x.c[t]; }
    }
    let n = mc.len();
    let roc = |k: usize, m: usize| if m >= k && k > 0 { (mc[m] / mc[m - k] - 1.) * 100. } else { f64::NAN };
    let s: Vec<f64> = (0..n).map(|m| roc(r1, m) + roc(r2, m)).collect();
    let wsum = (w * (w + 1) / 2) as f64;
    let cop = (0..n).map(|m| if w == 0 || m + 1 < w { f64::NAN } else {
        (0..w).map(|j| s[m + 1 - w + j] * (j + 1) as f64).sum::<f64>() / wsum }).collect();
    (cop, first)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    fn row(d: &str, c: f64) -> Row { Row { ticker: "T".into(), date: d.into(), open: c, high: c, low: c, close: c, volume: None } }
    #[test]
    fn article_dates_oct_1990() {
        // article p.12: "October 4 was a Full Moon ... October 18 ... a Bullish New Moon"
        let r: Vec<Row> = [("1990-10-03", 1.), ("1990-10-04", 2.), ("1990-10-17", 3.), ("1990-10-18", 4.), ("1990-10-19", 5.)].iter().map(|(d, c)| row(d, *c)).collect();
        let x = Ctx::new(&r, &r, &[]);
        assert_eq!(moon_bars(&x), vec![0, -1, 0, 1, 0]);
        let ts = run("ataa_moon", &x, 0, 4, &Params::new()).unwrap();
        assert_eq!((ts[0].dir, ts[0].legs[0].i, ts[0].exit_i, ts[0].exit_px), (-1., 1, 3, 4.));
    }
}
