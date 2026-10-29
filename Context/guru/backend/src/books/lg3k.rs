//! Lee E. Gettess, "The $3000 Secret" (Windsor Books; 24-page booklet, examples dated 1991). Book id `lg3k`.
//! Source copy: Books/Lee E. Gettess/gettess $3000 secret.doc (scanned pages; refs = printed page numbers).
//! Transcription / testability / oracle: library/lg3k/.
//!
//! Also holds the small daily helpers shared by the other Gettess / Aan modules (lgvp, lgtt30, aanwr):
//! * `roundup(v, tick)`: "always round your final figures UP to the next whole number" of ticks (p.9-10).
//! * `reversal()`: always-in stop-and-reverse on daily bars. Each day only the opposite stop is working (book p.10:
//!   "since we are already long ... we will only look at the SELL point"), so at most one reversal per day.
//!   While flat (start of data) both stops work; if both are inside the day's range the intrabar path decides
//!   (vault convention, as jkbwts: open -> extreme nearer the open -> other extreme; tie = low first).
//! * `oco_day()`: day trade with two stop orders; the first one filled is the trade, the other becomes its stop
//!   (path decides which came first), otherwise exit at the close.
//! A stop already through at the open fills at the open.
use crate::connors::{claim, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "lg3k";
/// Admitted ids (authors/gettess/admission/SOURCE-CONTRACT.md); lg3k_tr_day stays gate-withheld.
pub const REVIEWED: &[&str] = &["lg3k_ar_rev", "lg3k_tr_rev", "lg3k_sp_rev", "lg3k_tr_day" /* intraday wave 2b: daily bars decide every trade */];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

pub(crate) fn roundup(v: f64, tick: f64) -> f64 {
    if tick <= 0.0 { return v; }
    (v / tick - 1e-9).ceil() * tick
}
pub(crate) fn tr(dir: f64, ei: usize, epx: f64, xi: usize, xpx: f64, reason: &str, kind: &'static str, open: bool) -> Trade {
    Trade { dir, legs: vec![Leg { i: ei, px: epx, w: 1.0 }], exit_i: xi, exit_px: xpx, reason: reason.into(), entry_kind: kind,
        signal_i: xi, open, exits: vec![] }
}
/// true when the intrabar path visits the low before the high (open nearer the low; tie = low first).
pub(crate) fn low_first(o: f64, h: f64, l: f64) -> bool { o - l <= h - o }

/// Stop-and-reverse. `lv(i)` = (buy stop, sell stop) working on bar i (NaN = none), computed from data up to i-1 plus o[i].
pub(crate) fn reversal(x: &Ctx, from: usize, to: usize, tag: &str, lv: impl Fn(usize) -> (f64, f64)) -> Vec<Trade> {
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let mut out = vec![];
    let mut pos: Option<(f64, usize, f64)> = None; // dir, entry bar, entry px
    for i in from..=to {
        let (b, s) = lv(i);
        let bh = b.is_finite() && h[i] >= b;
        let sh = s.is_finite() && l[i] <= s;
        let bpx = b.max(o[i]);
        let spx = s.min(o[i]);
        match pos {
            None => {
                let d = if bh && sh { if low_first(o[i], h[i], l[i]) { -1.0 } else { 1.0 } } else if bh { 1.0 } else if sh { -1.0 } else { 0.0 };
                if d != 0.0 { pos = Some((d, i, if d > 0.0 { bpx } else { spx })); }
            }
            Some((d, ei, epx)) => {
                if d > 0.0 && sh {
                    out.push(tr(1.0, ei, epx, i, spx, &format!("{tag}: reversed at the sell stop"), "stop", false));
                    pos = Some((-1.0, i, spx));
                } else if d < 0.0 && bh {
                    out.push(tr(-1.0, ei, epx, i, bpx, &format!("{tag}: reversed at the buy stop"), "stop", false));
                    pos = Some((1.0, i, bpx));
                }
            }
        }
    }
    if let Some((d, ei, epx)) = pos { out.push(tr(d, ei, epx, to, c[to], "open", "stop", true)); }
    out
}

/// One-cancels-other day trade: first stop filled = the trade, the other = its stop; else exit at the close.
pub(crate) fn oco_day(x: &Ctx, from: usize, to: usize, lv: impl Fn(usize) -> (f64, f64)) -> Vec<Trade> {
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let mut out = vec![];
    for i in from..=to {
        let (b, s) = lv(i);
        let bh = b.is_finite() && h[i] >= b;
        let sh = s.is_finite() && l[i] <= s;
        let (bpx, spx) = (b.max(o[i]), s.min(o[i]));
        let atopen_b = b.is_finite() && b <= o[i];
        let atopen_s = s.is_finite() && s >= o[i];
        if bh && sh {
            let long_first = if atopen_b { true } else if atopen_s { false } else { !low_first(o[i], h[i], l[i]) };
            if long_first { out.push(tr(1.0, i, bpx, i, s, "Stopped by the sell point", "stop", false)); }
            else { out.push(tr(-1.0, i, spx, i, b, "Stopped by the buy point", "stop", false)); }
        } else if bh { out.push(tr(1.0, i, bpx, i, c[i], "Market on close", "stop", false)); }
        else if sh { out.push(tr(-1.0, i, spx, i, c[i], "Market on close", "stop", false)); }
    }
    out
}

const NOTE_TB: &str = "Worked example on 1991 T-Bond day-session data (p.8-12); the book presents it as the general volatility method, parameters for illustration. Run on the index (instrument swap). Ranges in index points, figure rounded UP to the tick (book: 'always round your final figures UP'). Each day only the opposite stop works (p.10); at the data start the first breakout either way starts the position.";

pub fn catalogue() -> Vec<StrategyDef> {
    use Status::*;
    let d = |id, name, group, dir, status, rules, notes, claim| StrategyDef { id, name, group, session: 0, dir, status,
        needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim };
    vec![
        d("lg3k_ar_rev", "Volatility reversal · 4-day actual range × 45%", "Volatility breakout (p.5-10)", "Long/Short", Live,
          &["F|\"We are going to average the last four days of actual range, then use 45% of this figure as our breakout points, displacing it from the opening price\" (p.9); actual range = high minus low (p.5)",
            "F|\"Always round the final volatility factor UP\" (p.9-10)",
            "E|\"adding eight ticks to tomorrow's opening price to get our BUY price, and subtracting eight ticks from the open to get our SELL price\" (p.10)",
            "X|\"hold the position until you get a signal IN THE OPPOSITE DIRECTION ... you SELL twice as many contracts ... always positioned in the market either long or short\" (p.6); \"since we are already long, we will only look at the SELL point\" (p.10)"],
          NOTE_TB, None),
        d("lg3k_tr_rev", "Volatility reversal · 4-day true range × 55%", "Volatility breakout (p.5-10)", "Long/Short", Live,
          &["F|True range: \"When prices gap so that the day's low is higher than the previous close, you measure the true range as the distance from the high to the previous close ... When the high of a day is lower than the previous close you measure true range as the distance to that day's low from the previous close\" (p.11)",
            "F|\"Let's use a volatility factor of 55%\" of the four-day average true range, rounded up (p.11)",
            "E|Buy stop = open + factor, sell stop = open - factor (p.11: \"Add 7 to the 93-08 open on 7/24, and your buy becomes 93-15\")",
            "X|Stop and reverse (p.11: \"We then stay long until 8/01, when we go short at 94-23\")"],
          NOTE_TB, None),
        d("lg3k_tr_day", "Volatility day trade · 4-day true range × 55%, exit on the close", "Volatility breakout (p.5-10)", "Long/Short", Live,
          &["F|\"It can be used for day trading, whereby you look each day for both the buys and the sells, then exit your position by the close. Using that method with 55% of the four day average true range\" (p.12)",
            "E|Buy stop = open + factor, sell stop = open - factor (rounded up); \"use both buys and sells each day\" (p.12)",
            "X|Exit on the close of the entry day (p.12)"],
          "Book example p.12 (T-Bonds 7/24-8/01/91). Both sides are taken when both are hit (book: 'use both buys and sells each day'); each exits on the close. No stop is given by the book. Run on the index (instrument swap).", None),
        d("lg3k_sp_rev", "Volatility reversal · S&P 500, 50% of 2-day average range", "Recent examples (p.16-21)", "Long/Short", Live,
          &["F|\"We will use 120% of one day's actual range for T-Bonds, and 50% of a two day average actual range for the S&P 500. These parameters were arrived at via optimization\" (p.16)",
            "E|\"a volatility reversal system that is always in the market, positioned either long or short\" (p.16); points displaced from the open, rounded up (p.5-10)",
            "X|Stop and reverse"],
          "Book trade list p.19: June 1991 S&P 500 futures (SPM), 3/11-6/7/1991, 25 reversals + exit, 3215 points = $16,075. Run on SPX/SPY/NIFTY (instrument swap). SPX daily opens before ~1999 often equal the prior close in the vault data, so a cash-index 1991 replication is approximate.",
          claim("26 (list)", "", "3215 pts total", "", "June 1991 S&P futures 3/11-6/7/1991 (p.19)")),
        d("lg3k_tb_rev", "Volatility reversal · T-Bonds, 120% of one day's range", "Recent examples (p.16-21)", "Long/Short",
          NeedsData("T-Bond futures daily data (parameter optimised for T-Bonds; the vault has no bond futures)"),
          &["F|\"120% of one day's actual range for T-Bonds\" (p.16)", "X|Stop and reverse"],
          "Book trade list p.17: September 1991 T-Bonds 6/5-8/29/1991, 86 points = $2,687.50.", claim("11 (list)", "", "86 pts total", "", "Sept 1991 T-Bonds (p.17)")),
        d("lg3k_vol_filter", "Minimum / maximum volatility filters", "Filters & optimisation (p.12-15)", "Long/Short",
          NotTestable("no threshold values are given; the filter is described only as an idea"),
          &["F|\"this filter can be either a minimum or maximum measure that must be reached or else a trade is bypassed\" (p.12-13)"], "", None),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "lg3k_ar_rev" => ("lg3k_vol", "$3000 Secret volatility system", "4-day actual range 45%, reversal"),
        "lg3k_tr_rev" => ("lg3k_vol", "$3000 Secret volatility system", "4-day true range 55%, reversal"),
        "lg3k_tr_day" => ("lg3k_vol", "$3000 Secret volatility system", "4-day true range 55%, day trade"),
        "lg3k_sp_rev" => ("lg3k_vol", "$3000 Secret volatility system", "S&P: 2-day range 50%, reversal"),
        "lg3k_tb_rev" => ("lg3k_vol", "$3000 Secret volatility system", "T-Bonds: 1-day range 120%"),
        _ => return catalogue().into_iter().find(|d| d.id == id).map(|d| (d.id, d.name, "")),
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    let tick = p("tick", "Rounding increment: 0 = the instrument's own tick (book: \"round your final figures UP\" to the next tick, p.9-10); e.g. 0.05 = S&P futures / NSE tick", 0.0, 0.0, 1.0, 0.01);
    let (n, pct, tre) = match id {
        "lg3k_ar_rev" => (4., 45., 0.),
        "lg3k_tr_rev" | "lg3k_tr_day" => (4., 55., 1.),
        "lg3k_sp_rev" => (2., 50., 0.),
        _ => return None,
    };
    Some(vec![p("days", "Days of range averaged", n, 1., 10., 1.), p("pct", "Percent of the average range added to / subtracted from the open", pct, 10., 200., 5.),
        p("true_range", "Range type: 0 = actual (high-low), 1 = true range", tre, 0., 1., 1.), tick])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    if from > to || to >= x.c.len() { return None; }
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(f64::NAN, |d| d.def));
    let n = g("days").max(1.0).round() as usize;
    // Book p.9-10: round UP to the traded instrument's tick; 0 (default) = that tick. Before 2026-10-25 the default was a fixed 0.05.
    let (pct, tick) = (g("pct") / 100.0, if g("tick") > 0.0 { g("tick") } else { x.market_tick() });
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let rng: Vec<f64> = (0..c.len()).map(|i| if g("true_range") >= 0.5 && i > 0 { h[i].max(c[i - 1]) - l[i].min(c[i - 1]) } else { h[i] - l[i] }).collect();
    let fig = |i: usize| -> f64 {
        if i < n + 1 { return f64::NAN; }
        roundup(rng[i - n..i].iter().sum::<f64>() / n as f64 * pct, tick)
    };
    let lv = |i: usize| { let f = fig(i); (o[i] + f, o[i] - f) };
    Some(match id {
        "lg3k_tr_day" => {
            let mut v = vec![];
            for i in from..=to {
                let (b, s) = lv(i);
                if !b.is_finite() { continue; }
                if h[i] >= b { v.push(tr(1.0, i, b.max(o[i]), i, c[i], "Exit on the close", "stop", false)); }
                if l[i] <= s { v.push(tr(-1.0, i, s.min(o[i]), i, c[i], "Exit on the close", "stop", false)); }
            }
            v
        }
        _ => reversal(x, from, to, "Volatility reversal", lv),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    fn rows(v: &[(f64, f64, f64, f64)]) -> Vec<Row> {
        v.iter().enumerate().map(|(k, &(o, h, l, c))| Row { ticker: "T".into(), date: format!("2024-01-{:02}", k + 1), open: o, high: h, low: l, close: c, volume: None }).collect()
    }
    #[test]
    fn roundup_book_examples() {
        // p.9: 16.75 * .45 = 7.5375 -> 8 (ticks of 1); p.11: 12 * .55 = 6.6 -> 7; Volpat p.8: 3.37 -> 3.40 at 0.05
        assert_eq!(roundup(7.5375, 1.0), 8.0); assert_eq!(roundup(6.6, 1.0), 7.0);
        assert!((roundup(3.37, 0.05) - 3.40).abs() < 1e-9); assert!((roundup(8.855, 0.05) - 8.90).abs() < 1e-9);
    }
    #[test]
    fn reversal_one_per_day() {
        // ranges 2 each; 50% of 2-day avg = 1. bar2: open 100 -> buy 101 / sell 99; high 102 -> long 101.
        // bar3: open 101 -> sell 100; low 99.5 -> reverse short at 100 (buy 102 not looked at).
        let r = rows(&[(100., 101., 99., 100.), (100., 101., 99., 100.), (100., 102., 100., 101.5), (101., 101.5, 99.5, 100.)]);
        let x = Ctx::new(&r, &r, &[]);
        let t = reversal(&x, 0, 3, "t", |i| if i < 2 { (f64::NAN, f64::NAN) } else { (x.o[i] + 1.0, x.o[i] - 1.0) });
        assert_eq!(t.len(), 2);
        assert_eq!((t[0].dir, t[0].legs[0].px, t[0].exit_px), (1.0, 101.0, 100.0));
        assert!(t[1].open && t[1].dir < 0.0);
    }
}
