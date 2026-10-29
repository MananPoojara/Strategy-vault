//! Les Crane, "WaveSurfer" (2002, scanned e-book; book id `lcws`; printed pages, PDF = printed + 10). Reading:
//! claude-continuation-20261020/authors/lcrane (admission/ws_ref.py). Live: the Impulse Ratio rule - the daily change
//! of the 10-day average of closes (p.32-34) - buy after 10 or more days with a negative ratio once it closes at
//! +0.010 or more (p.42-43), sell when it falls below +0.010 (p.45); the short mirror (p.46) is an option.
//! Book: NYSE stocks (Ford example p.36-37); indices, ETFs and US/India stocks here are a declared application.
use crate::connors::{sma, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};
/// Admitted ids (authors/lcrane/admission/ws_ref.py (header)).
pub const REVIEWED: &[&str] = &["lcws_impulse"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] Impulse Ratio = (10-day average of closes today - the same average yesterday) / yesterday's average (spreadsheet p.32-34). A run of 10 or more days with a negative ratio, then the first positive ratio arms the buy; while the ratio stays at or above zero but below the entry level the trade waits (\"wait another day\", p.42-43); a negative ratio disarms it and a new run of 10 negative days is needed. Signals are read at the close and the order goes in the next day (p.43): fill at the next day's open. The exit is checked from the entry day's close: a ratio below the exit level sells at the next open. The book's noon or 3:58 pm timing and limit orders at the bid/ask (p.43-44) are not modelled. The 'flattens out below +0.020' sell (p.45) is judgement; the exit level 0.020 is an option. Short (side 1): the mirror after 10 positive days (p.46); the book's 'whole market negative' check is not coded. A signal on the last bar is not filled; a position still open on the last bar is marked at its close.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "lcws_impulse", name: "WaveSurfer Impulse Ratio: buy +0.010 after 10 negative days, sell below +0.010", group: "Crane WaveSurfer",
        session: 0, dir: "Long", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: "lcws",
        rules: &["F|10-day average = @SUM(B2.B11)/10 of the closes; ratio = @SUM(C12-C11)/C11, \"the percentage amount of today's 10 day running average's Impulse Ratio, above or below, yesterday's 10-day moving average\" (p.32-34)",
            "E|\"Wait until you see the 10 day, Impulse Ratio cross-over from negative to positive...AFTER HAVING BEEN 10 DAYS WITH THE OPPOSITE, NEGATIVE (-) SIGN ... do nothing, until after the close. If the Impulse Ratio closes (+.010) or more, this is a strong signal to buy ... If the Impulse Ratio is less than (+.010), wait another day\" (p.42-43); \"enter your buy or sell order the next day\" (p.43)",
            "X|\"A decline of the ratio to below (+ or -.010), or less, is a firm and absolute, signal to sell\" (p.45)",
            "F|\"Everything previously said ... concerning buying stocks for a long position, can be reversed and used as a guide for selling stock for a short position\" (p.46): side 1"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match id { "lcws_impulse" => Some(("lcws_impulse", "Crane WaveSurfer Impulse Ratio", "10 days, +0.010")), _ => None }
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    match id {
        "lcws_impulse" => Some(vec![p("side", "Side: 0 = long (book), 1 = short mirror (p.46)", 0., 0., 1., 1.),
            p("run", "Days with the opposite sign before the cross (book 10)", 10., 1., 30., 1.),
            p("entry", "Entry level, ratio (book 0.010; 0.015 mentioned p.43)", 0.010, 0.0, 0.05, 0.001),
            p("exit", "Exit level, ratio (book 0.010; 0.020 = the 'flattens out' reading)", 0.010, 0.0, 0.05, 0.001)]),
        _ => None,
    }
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(f64::NAN, |d| d.def));
    let d = if g("side").round() >= 1. { -1. } else { 1. };
    let (need, lin, lout) = (g("run").round().max(1.) as usize, g("entry"), g("exit"));
    let s = sma(&x.c, 10);
    // signed ratio: the short mirror flips the sign so one long-side rule serves both
    let r: Vec<f64> = (0..s.len()).map(|i| if i == 0 { f64::NAN } else { d * (s[i] / s[i - 1] - 1.) }).collect();
    let mk = |ei: usize, xi: usize, xp: f64, why: &str, si: usize, open: bool| Trade { dir: d, legs: vec![Leg { i: ei, px: x.o[ei], w: 1. }],
        exit_i: xi, exit_px: xp, reason: why.into(), entry_kind: "open", signal_i: si, open, exits: vec![] };
    let (mut out, mut cur, mut neg, mut armed) = (vec![], None::<usize>, 0usize, false);
    for t in 0..to {
        let v = r[t];
        if let Some(e) = cur {
            if t >= e && !(v >= lout) { out.push(mk(e, t + 1, x.o[t + 1], "ratio below the exit level", t, false)); cur = None; }
        }
        // the sign run is read from the ratio alone, in or out of a trade
        if v.is_nan() || v < 0. {
            armed = false;
            neg = if v.is_nan() { 0 } else { neg + 1 };
            continue;
        }
        if v > 0. && neg >= need { armed = true; }
        neg = 0;
        if armed && cur.is_none() && v >= lin && t >= from { cur = Some(t + 1); armed = false; }
    }
    if let Some(e) = cur { out.push(mk(e, to, x.c[to], "open", to, true)); }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn hand_checked_impulse_trades() {
        // 25 falling closes (ratio negative from bar 10 to 24), then a jump: ratio +0.031 at bar 25 -> buy the open of 26;
        // the ratio stays >= 0.010 until bar 33 (+0.003) -> sell the open of 34.
        let mut cl: Vec<f64> = (0..25).map(|i| 100. - i as f64).collect();
        cl.extend([110., 112., 113., 113., 113., 100., 90., 90., 80., 70.]);
        let r: Vec<Row> = cl.iter().enumerate().map(|(i, &c)| Row { ticker: "T".into(), date: format!("2020-{:02}-{:02}", 1 + i / 28, 1 + i % 28), open: c + 0.5, high: c + 1., low: c - 1., close: c, volume: None }).collect();
        let x = Ctx::new(&r, &r, &[]);
        let t = run("lcws_impulse", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert_eq!(t.len(), 1);
        assert_eq!((t[0].legs[0].i, t[0].legs[0].px, t[0].signal_i), (26, 112.5, 33));
        assert_eq!((t[0].exit_i, t[0].exit_px, t[0].open), (34, 70.5, false));
        assert!(run("lcws_x", &x, 0, r.len() - 1, &Params::new()).is_none());
    }
}
