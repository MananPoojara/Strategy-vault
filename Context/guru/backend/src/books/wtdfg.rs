//! Walter T. Downs, "The Forex Gambit" (e-book; book id `wtdfg`; PDF pages cited, the book prints none).
//! Reading: claude-continuation-20261020/authors/downs (admission/downs_ref.py). Live: the daily Bollinger pullback system
//! (PDF 7-9): inside-down bar closing in its upper half above the 30-day middle line, middle line level or rising 10 days;
//! buy at the close, stop at the bar's low, exit after a close above the 2 SD band followed by a lower close, breakeven after
//! four days, out after 16 days. The betting algorithm (PDF 4-6) is position sizing and is not applied.
//! Book: currency pairs; indices, ETFs and stocks are a declared application. Trading for Tigers (.doc) was not read.
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

/// Admitted ids (authors/downs/admission/downs_ref.py (header)).
pub const REVIEWED: &[&str] = &["wtdfg_bb_pullback"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] Bollinger middle = 30-day simple average of closes, bands = population standard deviation of the same 30 closes. 'Level or rising for 10 consecutive days' = the middle line at or above the day before on each of the last 10 days. Entry at the signal day's close (the book: at the market 5-10 minutes before the close). The book's '2 points' beyond the bar are currency pips: here the stop is the signal bar's low (short: high); it is live from the next day and fills at the level or a worse open. Breakeven: once the 4th day after entry has closed, the stop moves to the entry price. Time exit at the close of the 16th day after entry. The band exit needs a close beyond the 2 SD band on a day in the trade (the entry day counts) and exits at the next day's lower (short: higher) close. No new trades on Fridays. Not applied: the vague 'moved only slightly' and 'does not move significantly for 3 days' exits, the 180-pip risk cap, choosing among several currency pairs, and the betting algorithm. One position at a time; a new entry can come from the bar after an exit.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "wtdfg_bb_pullback", name: "Forex Gambit: Bollinger pullback (inside-down bar above the 30-day middle line)", group: "The System (PDF 7-9)",
        session: 0, dir: "Long/Short", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: "wtdfg",
        rules: &["F|\"Use Daily time frame ... Apply a 30 period Bollinger Band to your chart, with bands of 1(one) and 2(two) standard deviations.\" (PDF 7)",
            "E|\"1. A Low lower than the previous Low. 2. A High Lower than the previous High 3. A Close greater than the midpoint of the bar range. 4. A Close greater than the central line of the Bollinger Band. 5. A Low that less than the 1(one) standard deviation upper line ... 6. The central line ... has been level or rising for 10(ten) consecutive days.\" (PDF 7; sell pattern mirrored, PDF 8)",
            "E|\"Within 5 to 10 minutes of the close of trading for the day, BUY at the market, and place your stop-loss orders 2(two) points below the Low of today's bar.\" \"DO NOT OPEN ANY NEW POSITIONS ON FRIDAY.\" (PDF 7-8)",
            "X|\"Exit Long positions when you get a closing price greater than the 2 standard deviation upper line of the Bollinger Band, followed by a lower close on the following day.\" (PDF 9)",
            "X|\"Move your trade to the breakeven point ... after four days.\" \"Exit all positions that have been open for 16 days.\" (PDF 9)"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match id { "wtdfg_bb_pullback" => Some(("wtdfg_bb_pullback", "Downs Forex Gambit", "BB(30) pullback")), _ => None }
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    match id {
        "wtdfg_bb_pullback" => Some(vec![p("be", "Breakeven stop after four days (1 = on, book)", 1., 0., 1., 1.),
            p("maxd", "Time exit after N days (book 16)", 16., 1., 1000., 1.)]),
        _ => None,
    }
}

fn friday(d: &str) -> bool {
    // Sakamoto's day-of-week (0 = Sunday) on YYYY-MM-DD
    let (y, m, dd): (i64, i64, i64) = (d[0..4].parse().unwrap_or(0), d[5..7].parse().unwrap_or(1), d[8..10].parse().unwrap_or(1));
    let t = [0, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4];
    let y = if m < 3 { y - 1 } else { y };
    (y + y / 4 - y / 100 + y / 400 + t[(m - 1) as usize] + dd) % 7 == 5
}

fn boll(c: &[f64], n: usize) -> (Vec<f64>, Vec<f64>) {
    let (mut mid, mut sd) = (vec![f64::NAN; c.len()], vec![f64::NAN; c.len()]);
    for i in n - 1..c.len() {
        let w = &c[i + 1 - n..=i];
        let mut s = 0.0; for v in w { s += v; }
        let m = s / n as f64;
        let mut q = 0.0; for v in w { q += (v - m) * (v - m); }
        mid[i] = m; sd[i] = (q / n as f64).sqrt();
    }
    (mid, sd)
}

fn run_gambit(x: &Ctx, from: usize, to: usize, be: bool, maxd: usize) -> Vec<Trade> {
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let (mid, sd) = boll(c, 30);
    let mk = |d: f64, ei: usize, ep: f64, xi: usize, xp: f64, why: &str, open: bool| Trade { dir: d, legs: vec![Leg { i: ei, px: ep, w: 1. }], exit_i: xi, exit_px: xp, reason: why.into(), entry_kind: "close", signal_i: xi, open, exits: vec![] };
    let (mut out, mut cur) = (vec![], None::<(f64, usize, f64, f64)>); // dir, entry bar, entry price, stop
    for i in from.max(40)..=to {
        if let Some((d, ei, ep, mut s)) = cur {
            if be && i >= ei + 5 { s = if d > 0. { s.max(ep) } else { s.min(ep) }; }
            cur = Some((d, ei, ep, s));
            if (d > 0. && o[i] <= s) || (d < 0. && o[i] >= s) { out.push(mk(d, ei, ep, i, o[i], "stop", false)); cur = None; continue; }
            if (d > 0. && l[i] <= s) || (d < 0. && h[i] >= s) { out.push(mk(d, ei, ep, i, s, "stop", false)); cur = None; continue; }
            if d > 0. && c[i - 1] > mid[i - 1] + 2. * sd[i - 1] && c[i] < c[i - 1] { out.push(mk(d, ei, ep, i, c[i], "2 SD close, then a lower close", false)); cur = None; continue; }
            if d < 0. && c[i - 1] < mid[i - 1] - 2. * sd[i - 1] && c[i] > c[i - 1] { out.push(mk(d, ei, ep, i, c[i], "2 SD close, then a higher close", false)); cur = None; continue; }
            if i - ei >= maxd { out.push(mk(d, ei, ep, i, c[i], "time exit", false)); cur = None; }
            continue;
        }
        if i >= to || friday(&x.r[i].date) { continue; }
        if l[i] < l[i - 1] && h[i] < h[i - 1] && c[i] > (h[i] + l[i]) / 2. && c[i] > mid[i] && l[i] < mid[i] + sd[i] && (i - 9..=i).all(|j| mid[j] >= mid[j - 1]) {
            cur = Some((1., i, c[i], l[i]));
        } else if h[i] > h[i - 1] && l[i] > l[i - 1] && c[i] < (h[i] + l[i]) / 2. && c[i] < mid[i] && h[i] > mid[i] - sd[i] && (i - 9..=i).all(|j| mid[j] <= mid[j - 1]) {
            cur = Some((-1., i, c[i], h[i]));
        }
    }
    if let Some((d, ei, ep, _)) = cur { out.push(mk(d, ei, ep, to, c[to], "open", true)); }
    out
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(f64::NAN, |d| d.def));
    Some(match id {
        "wtdfg_bb_pullback" => run_gambit(x, from, to, g("be") > 0.5, g("maxd").max(1.) as usize),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn friday_check() { assert!(super::friday("2026-10-23")); assert!(!super::friday("2026-10-22")); assert!(super::friday("1999-12-31")); }
}
