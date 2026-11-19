//! Lane K82 (13 Nov 2026; authors/titles-1030/K82.tsv; reader row authors/o2/O2.tsv): Russell Arthur Lockhart, "Hints & Helps for
//! the Active Trader" (Part I, The Lockhart Press 1999, updates to Jan 2000; research share 'USB HDD 2/tRADING KNO-2/Russell
//! A.Lockhard - Hints & Helps For Daytraders/Lockhart Russell Arthur - Hints & Helps For Daytraders [133] .pdf', 115 PDF pages,
//! text layer; printed page = PDF page - 1). Book id `rlhhd`.
//! Coded: the three-price break (3PB) traded mechanically (p.24-38, Tables 1-2) and the 3PB with Fibonacci exits and re-entries
//! (p.41-44, Table 3). Held: Fibonacci price potential targets and NTU trading.
//! Not Pring's mpcse_3lb: Lockhart's reversal box starts at the lower (upper) edge of the last box, his chart starts at the test's
//! first close and enters on the first new price, and a reversal broken before confirmation restores the old trend (p.28 fn.20, p.36).
//! Independent reference: authors/titles-1030/K82w/k82_ref.py (written from the pages, not from this file).
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::tselhs::Book;

pub const REVIEWED: &[&str] = &["rlhhd_3pb", "rlhhd_3pb_fib"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const SRC: &str = "Lockhart, Hints & Helps for the Active Trader, Part I (1999-2000); printed pages cited.";
const READ: &str = "[Declared readings] Closing prices only; orders at the signal close (the log's Long@ / Short@ columns are the closes, p.29-30). The chart starts at the first close of the test ('a 3PB chart can be started at any time', p.26). A new price is a close strictly beyond the last new price; the break price is the lowest (highest) edge of the last three boxes, and a close equal to it is not a break (p.27). Each new price adds a box from the end of the last one; a break box starts at the lower (upper) edge of the last box of the old trend and runs to the break close (p.35), which gives the log's break prices (e.g. 9,328.19 at the third new low, p.36). A new trend broken after only one new price is not a new trend: 'the original trend is resumed, and the trend count continued as if there were no break at all' (p.36, p.28 fn.20); the box is removed and the close becomes the old trend's next new price. The breaks of that one-price trend are the same either way; only the later break prices differ. The 2- and 4-price breaks (p.32 fn.22) are an option. The DJIA (DJI) is the book's market; other indices, ETFs and stocks, US and India, are a declared application.";

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, group, status, rules, notes| StrategyDef { id, name, group, session: 0, dir: "Long/Short", status, needs_volume: false, needs_vix: false,
        needs_series: &[], book: "rlhhd", rules, notes, claim: None };
    vec![
        d("rlhhd_3pb", "Lockhart three-price break, always in: reverse at the close that breaks the last three boxes", "Part I · The Three-Price Break Method (p.24-38)", Status::Live,
          &["E|\"So, on July 22, when the Dow closed at 9,128.91, a price below the break price of 9,174.97, the 3PB trader liquidates the long position, and goes short\" (p.35); the first trade is the first new price after the start price (Table 1, p.29)",
            "F|\"to determine where the break price is, we count backwards three boxes\" (p.32); \"If the closing price is the same as the break price, it is not considered a break\" (p.27)",
            "X|\"the method itself is always in the market, either long or short\" (p.27); Table 2 (p.38): DJIA June-October 1998, 4 trades, +299.52, +554.13, -408.86, -426.11 points (net 18.68)"],
          READ),
        d("rlhhd_3pb_fib", "Lockhart 3PB with Fibonacci exits: take profit at the 3rd, 5th, 8th, 13th... new price, re-enter at the next new price", "Part I · Fibonacci Price Points and the Three-Price Break (p.41-44)", Status::Live,
          &["E|Enter at the first new price of a 3PB trend (Table 3, p.44); \"the method tells us to re-enter if and when the fourth new high is achieved\" (p.43): after an exit, re-enter at the next new price",
            "X|\"once in a trade, all subsequent Fibonacci price levels (3, 5, 8, 13, 21, 34, ...) are action prices in terms of trade exit\" (p.41-42); a 3PB break exits and reverses (Table 3: '3pb down', '3pb up')",
            "F|Table 3 (p.44): DJIA June-October 1998, 10 trades, net +584.29 points"],
          "[Declared readings] The 3PB chart, break prices and the one-price-trend rule are those of rlhhd_3pb (see its card). Exit at a close that is the 3rd, 5th, 8th, 13th, 21st, 34th, ... new price of the trend; flat after it; re-enter at the next new price that is not itself an exit count; a break always closes the position and opens the new trend's side at the break close. When a one-price trend is broken and the old trend resumes (p.36), the position is reversed into the old trend unless that count is an exit count. Book check: Table 3's second row re-enters at 8,935.58 (the 3rd new high, the text calls it the 4th, p.43); the 4th new high in Table 1 is 8,944.54, so that row differs by 8.96 points, a source error; the first row's gain is printed 106.18 for 8,935.58 - 8,829.46 = 106.12. The book's 100-point stop (p.44 fn.28, 'imagine') is not applied. The DJIA (DJI) is the book's market; other indices, ETFs and stocks, US and India, are a declared application."),
        d("rlhhd_ppt", "Fibonacci price potential targets (PPT) and time expectation points", "Part I · Price Potential Target calculators (p.45-94)", Status::NotTestable("Held: the targets are levels from several calculators (growth factor, unit growth, spiral tables, p.55-94) applied to a start price the trader picks, and the text uses them for 'consideration' with the 3PB; no single entry or exit rule is stated (the full rules are deferred to Part 2, p.41, 42 fn.26)."),
          &[], SRC),
        d("rlhhd_ntu", "NTU (natural trading unit) trading", "Part I · NTU Trading (p.95-104)", Status::NotTestable("Held: the NTU size is 'some function of the Fibonacci price spiral' chosen per market and time frame by a 'variety of methods' (p.102); the worked example is a 13-minute Dow chart with an NTU picked for it (p.98-101)."),
          &[], SRC),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match id {
        "rlhhd_3pb" => Some(("rlhhd_3pb", "Lockhart three-price break", "3 boxes")),
        "rlhhd_3pb_fib" => Some(("rlhhd_3pb_fib", "Lockhart 3PB with Fibonacci exits", "3 boxes")),
        "rlhhd_ppt" | "rlhhd_ntu" => catalogue().into_iter().find(|d| d.id == id).map(|d| (d.id, d.name, "")),
        _ => None,
    }
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    reviewed(id).then(|| vec![ParamDef { key: "boxes", label: "Boxes counted back for the break (book 3; 2 and 4 named, p.32)", def: 3., min: 2., max: 4., step: 1. }])
}

#[derive(Clone)]
struct Chart { boxes: Vec<(f64, f64)>, dir: f64, n: usize, prev: Option<Box<Chart>> }

/// One close through the chart: Some((trend side, new-price count, trend changed)) on a new price or a break, None otherwise.
fn step(ch: &mut Chart, x: f64, start: f64, nb: usize) -> Option<(f64, usize, bool)> {
    if ch.dir == 0.0 {
        if x == start { return None; }
        ch.dir = if x > start { 1.0 } else { -1.0 };
        ch.boxes.push((start.min(x), start.max(x)));
        ch.n = 1;
        return Some((ch.dir, 1, true));
    }
    let last = *ch.boxes.last().unwrap();
    if (ch.dir > 0.0 && x > last.1) || (ch.dir < 0.0 && x < last.0) {
        ch.boxes.push(if ch.dir > 0.0 { (last.1, x) } else { (x, last.0) });
        ch.n += 1;
        return Some((ch.dir, ch.n, false));
    }
    let k = ch.boxes.len().saturating_sub(nb);
    let w = &ch.boxes[k..];
    let broke = if ch.dir > 0.0 { x < w.iter().map(|b| b.0).fold(f64::INFINITY, f64::min) } else { x > w.iter().map(|b| b.1).fold(f64::NEG_INFINITY, f64::max) };
    if !broke { return None; }
    if ch.n == 1 && ch.prev.is_some() {
        // p.36: broken before confirmation -> the old trend resumes and its count continues
        let mut old = *ch.prev.take().unwrap();
        let l = *old.boxes.last().unwrap();
        old.boxes.push(if old.dir > 0.0 { (l.1, x) } else { (x, l.0) });
        old.n += 1;
        *ch = old;
        return Some((ch.dir, ch.n, true));
    }
    let mut keep = ch.clone();
    keep.prev = None;
    let b = if ch.dir > 0.0 { (x, last.0) } else { (last.1, x) };
    *ch = Chart { boxes: vec![last, b], dir: -ch.dir, n: 1, prev: Some(Box::new(keep)) };
    Some((ch.dir, 1, true))
}

fn fib_exit(n: usize) -> bool {
    let (mut a, mut b) = (2usize, 3usize);
    while b < n { let c = a + b; a = b; b = c; }
    b == n
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let nb = p.get("boxes").copied().unwrap_or(3.).round().clamp(2., 4.) as usize;
    let fib = id == "rlhhd_3pb_fib";
    let start = x.c[from];
    let mut ch = Chart { boxes: vec![], dir: 0.0, n: 0, prev: None };
    let mut pos = Book::new();
    for t in from + 1..=to {
        let Some((d, n, turn)) = step(&mut ch, x.c[t], start, nb) else { continue };
        if !fib {
            if turn { pos.enter(d, t, x.c[t], "close"); }
            continue;
        }
        if turn && pos.side() != 0.0 && pos.side() != d { pos.exit(t, x.c[t], "3pb break"); }
        if n >= 3 && fib_exit(n) {
            if pos.side() == d { pos.exit(t, x.c[t], "Fibonacci new-price count"); }
        } else if pos.side() == 0.0 { pos.enter(d, t, x.c[t], "close"); }
    }
    Some(pos.done(to, x.c[to]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    fn rows(c: &[f64]) -> Vec<Row> {
        c.iter().enumerate().map(|(i, &v)| Row { ticker: "T".into(), date: format!("2001-01-{:02}", i + 1), open: v, high: v, low: v, close: v, volume: None }).collect()
    }
    #[test]
    fn book_table_1_break_prices_and_table_2_trades() {
        // Lockhart Table 1 (p.29-30), DJIA closes 16 Jun - 31 Aug 1998 (non-events kept where they matter)
        let c = [8665.29, 8829.46, 8813.01, 8712.87, 8711.13, 8828.46, 8923.87, 8935.58, 8944.54, 8997.36, 8952.02, 9048.67, 9025.26, 9091.77,
            9085.04, 9174.97, 9089.78, 9105.74, 9096.21, 9245.54, 9234.47, 9328.19, 9337.97, 9295.75, 9190.19, 9128.98, 8932.98, 8937.36,
            9028.24, 8934.78, 8914.96, 9026.95, 8883.29, 8786.74, 8487.31, 8546.78, 8577.68, 8598.02, 8574.85, 8462.85, 8552.96, 8459.50,
            8425.00, 8574.85, 8714.65, 8693.28, 8611.41, 8533.65, 8566.61, 8602.65, 8523.35, 8165.99, 8051.68, 7539.07];
        let r = rows(&c);
        let x = Ctx::new(&r, &r, &[]);
        let t = run("rlhhd_3pb", &x, 0, r.len() - 1, &Params::new()).unwrap();
        let pts: Vec<f64> = t.iter().filter(|z| !z.open).map(|z| ((z.dir * (z.exit_px - z.legs[0].px)) * 100.).round() / 100.).collect();
        assert_eq!(pts, vec![299.52, 554.13, -408.86]);
        let f = run("rlhhd_3pb_fib", &x, 0, r.len() - 1, &Params::new()).unwrap();
        let ex: Vec<(f64, f64)> = f.iter().map(|z| (z.legs[0].px, z.exit_px)).collect();
        assert_eq!(&ex[..4], &[(8829.46, 8935.58), (8944.54, 8997.36), (9048.67, 9174.97), (9245.54, 9128.98)]);
        assert_eq!(&ex[4..], &[(9128.98, 8914.96), (8883.29, 8786.74), (8487.31, 8459.50), (8425.00, 8574.85), (8574.85, 8165.99), (8165.99, 7539.07)]);
    }
    #[test]
    fn unconfirmed_reversal_restores_the_old_trend() {
        // up 10..13, break to 9.5 (one new low), back above 13 -> old up trend resumes as its 5th new high
        let r = rows(&[10., 11., 12., 13., 14., 9.5, 14.5, 10.9]);
        let x = Ctx::new(&r, &r, &[]);
        let mut ch = Chart { boxes: vec![], dir: 0.0, n: 0, prev: None };
        let ev: Vec<_> = (1..r.len()).map(|t| step(&mut ch, x.c[t], 10., 3)).collect();
        assert_eq!(ev[4], Some((-1.0, 1, true)));
        assert_eq!(ev[5], Some((1.0, 5, true)));
        // break of the resumed trend: lowest of the last three boxes (12-13, 13-14, 14-14.5) is 12; 10.9 breaks
        assert_eq!(ev[6], Some((-1.0, 1, true)));
    }
}

// Instrument admission: Lockhart three-price break 2026-11-13 (lane K82).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "rlhhd" && reviewed(def.id),
    label: "Russell Lockhart's worked example is the Dow Jones Industrial Average, daily closes, June-October 1998 ('all markets and all time frames'). Other indices, ETFs and stocks, US and India, are a declared application.",
    basis: "Hints & Helps for the Active Trader, Part I (1999) p.24-44, Tables 1-3; independent check authors/titles-1030/K82w/k82_ref.py",
};
