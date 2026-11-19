//! Lane K81 (13 Nov 2026; authors/titles-1030/K81.tsv): Patrick Lafferty, "Single Stock Futures" (McGraw-Hill, (c) 2002). Book id "plssf".
//! Source copy: research share (reader row authors/o3/O3.tsv idx 307; scanned, OCR authors/o3/ocr/307_*.txt). Read: contents
//! (PDF p.3-8) and ch.9 'Daily Reversal Patterns' (printed p.78-87, PDF p.91-100). Printed page = PDF page - 13.
//! One rule is coded: the daily reversal trade as 'Defining the Trade for Short-term Trading' states it (p.83).
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

const BOOK: &str = "plssf";
const SRC: &str = "Lafferty, Single Stock Futures (2002), ch.9 'Daily Reversal Patterns'; printed pages (PDF page = printed + 13).";
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == BOOK && def.id == "plssf_reversal",
    label: "Lafferty trades the daily reversal pattern in single stock futures (and shows it on stocks and the Dow cash index). It runs on the stocks themselves and on US and Indian indices and ETFs as a declared application; stock prices stand in for the futures (no futures leverage or roll).",
    basis: "Patrick Lafferty, Single Stock Futures (2002) ch.9 p.80-84; independent check authors/titles-1030/K81w/k81_ref.py (lafferty).",
};

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, dir, status, rules| StrategyDef { id, name, group: "Ch.9 · Daily reversal patterns", session: 0, dir, status, needs_volume: false,
        needs_vix: false, needs_series: &[], book: BOOK, rules, notes: concat!(
            "Lafferty, Single Stock Futures (2002), ch.9 'Daily Reversal Patterns'; printed pages (PDF page = printed + 13). ",
            "Reversal bottom: the day's low is below the lows of the previous 6 trading days and the close is above the previous close; reversal top: the high is above the previous 6 highs and the close is below the previous close (p.80-81, 83). ",
            "Default: enter at that close (p.84 offers the close or the next day's open: option p.next_open = 1 also takes the pattern exits at the next open). ",
            "Exit long the first day the price trades below the previous day's low, filled at that low or at the open when the day opens below it (reading: 'just below' = any price under it, no tick offset); exit short above the previous day's high. ",
            "A reversal pattern the other way closes the position at its close and opens the other side ('if you are long and a daily reversal top is identified, exit the position', p.83; that pattern is itself an entry). ",
            "Not applied: the heavier-volume 'exclamation point' (a note, not a condition, p.84), the outside-day limit entry (no price is given, p.85-86) and the trend-line stop drawn by eye (p.84). ",
            "Single stock futures in the book; here on stocks, indices and ETFs as a declared application."), claim: None };
    vec![
        d("plssf_reversal", "Daily reversal: 6-day low with a higher close = long, 6-day high with a lower close = short; out beyond the previous day's low / high", "Long/Short", Status::Live, &[
            "E|\"The daily reversal bottom trade. Enter long the market when the daily low surpasses the previous 6 days' low and then closes above the previous day's close.\" (p.83)",
            "E|\"The daily reversal top trade. Enter short the market when the daily high surpasses the previous 6 days' high and then closes below the previous day's close.\" (p.83)",
            "X|\"Maintain your position until there is a price move just below the previous day's low or until a daily reversal top pattern is signaled.\" (p.83)",
            "E|option next_open=1: \"you can initiate an entry order either on the market's close ... or on the following day's open\" (p.84)"]),
        d("plssf_outside_day", "Outside-day (key) reversal entered with a limit order", "Long/Short",
            Status::NotTestable("Held: 'I recommend using a limit order to enter a new position based on an outside day pattern' (p.85) gives no limit price, and the stop is the trend line drawn by eye (p.84)."), &[
            "E|\"If a reversal day's trading range exceeds both the high and the low of the previous day, then we refer to it as a key reversal, or outside day reversal\" (p.85)"]),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let d = catalogue().into_iter().find(|d| d.id == id)?;
    Some((d.id, d.name, "Lafferty, Daily Reversal Patterns"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    match id {
        "plssf_reversal" => Some(vec![ParamDef { key: "next_open", label: "Enter (and take pattern exits) at the next open instead of the close (p.84)", def: 0., min: 0., max: 1., step: 1. }]),
        "plssf_outside_day" => Some(vec![]),
        _ => None,
    }
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if id != "plssf_reversal" { return None; }
    let n = x.r.len();
    if n == 0 || from > to || to >= n { return Some(vec![]); }
    let nxt = p.get("next_open").copied().unwrap_or(0.) >= 0.5;
    let sig = |t: usize| -> f64 {
        if t < 6 { return 0.; }
        let (lo, hi) = (x.l[t - 6..t].iter().copied().fold(f64::INFINITY, f64::min), x.h[t - 6..t].iter().copied().fold(f64::NEG_INFINITY, f64::max));
        let bottom = x.l[t] < lo && x.c[t] > x.c[t - 1];
        let top = x.h[t] > hi && x.c[t] < x.c[t - 1];
        if bottom && !top { 1. } else if top && !bottom { -1. } else { 0. }
    };
    let open = |dir: f64, i: usize, px: f64, kind| Trade { dir, legs: vec![Leg { i, px, w: 1. }], exit_i: i, exit_px: f64::NAN, reason: String::new(), entry_kind: kind,
        signal_i: i, open: false, exits: vec![] };
    let mut out = vec![];
    let mut cur: Option<Trade> = None;
    let mut pending: f64 = 0.; // next_open: the pattern seen at yesterday's close
    for t in from.max(1)..=to {
        if nxt && pending != 0. {
            let s = pending; pending = 0.;
            if let Some(mut q) = cur.take() {
                if q.dir == s { cur = Some(q); } else {
                    (q.exit_i, q.exit_px, q.signal_i, q.reason) = (t, x.o[t], t - 1, "reversal pattern the other way: out at the next open".into());
                    out.push(q);
                }
            }
            if cur.is_none() { cur = Some(open(s, t, x.o[t], "open")); }
        }
        // the trailing stop: from the day after a close entry, from the entry day itself after an open entry
        if let Some(q) = cur.as_ref() {
            let ei = q.legs[0].i;
            if t > ei || (q.entry_kind == "open" && t == ei) {
                let hit = if q.dir > 0. { x.l[t] < x.l[t - 1] } else { x.h[t] > x.h[t - 1] };
                if hit {
                    let lv = if q.dir > 0. { x.l[t - 1] } else { x.h[t - 1] };
                    let px = if q.dir > 0. { x.o[t].min(lv) } else { x.o[t].max(lv) };
                    let mut q = cur.take().unwrap();
                    (q.exit_i, q.exit_px, q.signal_i, q.reason) = (t, px, t, "traded beyond the previous day's low / high".into());
                    out.push(q);
                }
            }
        }
        let s = sig(t);
        if s == 0. { continue; }
        if nxt { if t < to { pending = s; } continue; }
        match cur.take() {
            Some(q) if q.dir == s => cur = Some(q),
            Some(mut q) => {
                (q.exit_i, q.exit_px, q.signal_i, q.reason) = (t, x.c[t], t, "reversal pattern the other way: out at the close".into());
                out.push(q);
                cur = Some(open(s, t, x.c[t], "close"));
            }
            None => cur = Some(open(s, t, x.c[t], "close")),
        }
    }
    if let Some(mut q) = cur { (q.exit_i, q.exit_px, q.signal_i, q.open, q.reason) = (to, x.c[to], to, true, "open".into()); out.push(q); }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    fn r(d: usize, o: f64, h: f64, l: f64, c: f64) -> Row { Row { ticker: "T".into(), date: format!("2020-01-{:02}", d + 1), open: o, high: h, low: l, close: c, volume: None } }
    #[test]
    fn six_day_low_with_higher_close_buys_and_the_previous_low_trails() {
        let mut v: Vec<Row> = (0..6).map(|d| r(d, 100., 101., 99., 100.)).collect();
        v.push(r(6, 99., 100., 97., 99.5));   // 6: low 97 < 99 (six-day low), close 99.5 < 100: no signal
        v.push(r(7, 99., 101., 96., 100.5));  // 7: low 96 < 97, close 100.5 > 99.5: reversal bottom, long at 100.5
        v.push(r(8, 100.5, 102., 97., 101.)); // 8: low 97 > 96: held
        v.push(r(9, 96.5, 99., 95., 98.));    // 9: opens 96.5 below yesterday's low 97: out at the open
        let x = Ctx::new(&v, &v, &[]);
        let t = run("plssf_reversal", &x, 0, v.len() - 1, &Params::new()).unwrap();
        assert_eq!(t.iter().map(|t| (t.dir, t.legs[0].i, t.legs[0].px, t.exit_i, t.exit_px)).collect::<Vec<_>>(), vec![(1., 7, 100.5, 9, 96.5)]);
        let o = run("plssf_reversal", &x, 0, v.len() - 1, &[("next_open".to_string(), 1.)].into_iter().collect()).unwrap();
        assert_eq!(o.iter().map(|t| (t.legs[0].i, t.legs[0].px, t.exit_i, t.exit_px)).collect::<Vec<_>>(), vec![(8, 100.5, 9, 96.5)]);
        assert!(crate::gate::get("plssf_reversal").is_some());
    }
}
