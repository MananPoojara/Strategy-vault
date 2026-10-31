//! "Dennis RSI Trading System for Trading Stock Index Futures" (author named only as Dennis; research share USB HDD 2/tRADING
//! KNO-2/Dennis RSI Trading System for Trading Stock Index Futures/rsi2.tif - rsi8.tif, scanned manual pages 2-8; notes
//! authors/batch-c/C6w/dennis-rsi.txt, page images batch-c/C6w/drsi/). Book id `dnrsi`. Lane K10 (titles loop, 1 Nov 2026),
//! to-code row 165; independent check authors/titles-1030/K10w/k10_ref.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::lc3::Pos;

/// Admitted ids (authors/titles-1030/K10w/k10_ref.py (header)).
pub const REVIEWED: &[&str] = &["dnrsi_5day"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Stop/target held] The manual's 600-point ($3,000) S&P futures profit objective and 600-point stop are not applied and not converted (lead decisions 19 and 46); without them the system is the alternating long/short reversal on the RSI signals. [Declared readings] Each day's path runs previous close -> open -> high and low -> close: a close below the midrange (high + low) / 2 means the high came first (open, high, low, close), a close above it means the low came first; at the midrange the high comes first when the open is at or above the midrange, the low first when below (open = close = midrange: high first). Up movement = the sum of the rising legs, down movement = the sum of the falling legs (p.6 example: 3.50 up, 4.00 down). Dennis RSI = 100 x 5-day total up / (5-day total up + 5-day total down), today included. Signal at today's close, fill at tomorrow's open: long when the RSI is 40 or less, short when it is 60 or more; a signal for the side already held is ignored, the opposite signal reverses. The first bar has no previous close and no legs. The source instrument is S&P 500 futures (results appendix not on the drive); indices, ETFs and US/India stocks are a declared application.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "dnrsi_5day", name: "Dennis RSI: 5-day intraday-path RSI, long at 40 or less and short at 60 or more on the next open (stop/target held)", group: "Dennis RSI system (S&P futures manual)",
        session: 0, dir: "Long/Short", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: "dnrsi", rules: &[
            "F|p.5-6: close below the midrange -> path open, high, low, close; close above -> open, low, high, close; close at the midrange -> by the open vs the midrange; up = sum of rising legs from the previous close, down = sum of falling legs",
            "F|\"Dennis RSI = 100 x (5-day total up movement) / (5-day total up + 5-day total down movement)\" (p.7)",
            "E|\"1. Go long on tomorrow's opening if the 5 day RSI calculated today is equal to or less than 40. 2. Go short on tomorrow's opening if the 5 day RSI calculated today is equal to or greater than 60.\" (p.7)",
            "X|\"we will maintain the position until (1) we hit the profit objective, (2) we get stopped out, or (3) we get a signal to reverse our position. With this system, our position will always alternate\" (p.8); 600-point objective and stop held"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    (id == "dnrsi_5day").then_some(("dnrsi_5day", "Dennis RSI", "5-day 40/60, stop/target held"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    reviewed(id).then(|| vec![p("n", "RSI days (5)", 5., 2., 30., 1.), p("lo", "Long at or below (40)", 40., 5., 50., 1.), p("hi", "Short at or above (60)", 60., 50., 95., 1.)])
}

/// Up and down movement of bar i along the simulated path from the previous close.
pub fn legs(o: f64, h: f64, l: f64, c: f64, pc: f64) -> (f64, f64) {
    let mid = (h + l) / 2.;
    let high_first = c < mid || (c == mid && o >= mid);
    let path = if high_first { [pc, o, h, l, c] } else { [pc, o, l, h, c] };
    path.windows(2).fold((0., 0.), |(u, d), w| if w[1] > w[0] { (u + w[1] - w[0], d) } else { (u, d + w[0] - w[1]) })
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).unwrap().def);
    let (n, lo, hi) = (g("n").round().max(1.) as usize, g("lo"), g("hi"));
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let ud: Vec<(f64, f64)> = (0..c.len()).map(|i| if i == 0 { (f64::NAN, f64::NAN) } else { legs(o[i], h[i], l[i], c[i], c[i - 1]) }).collect();
    let mut pos = Pos::new();
    for i in from.max(n)..to {
        let (u, d) = ud[i + 1 - n..=i].iter().fold((0., 0.), |a, b| (a.0 + b.0, a.1 + b.1));
        if !(u + d > 0.) { continue; }
        let r = 100. * u / (u + d);
        if r <= lo { pos.enter(1., i + 1, o[i + 1], "open"); } else if r >= hi { pos.enter(-1., i + 1, o[i + 1], "open"); }
    }
    Some(pos.done(to, c[to]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn page_six_example_and_alternation() {
        assert_eq!(legs(186.5, 188., 184., 184.5, 185.), (3.5, 4.));
        let r: Vec<Row> = (0..300).map(|i| { let f = i as f64; let c = 100. + 6. * (f / 4.).sin();
            Row { ticker: "T".into(), date: format!("{:05}", i), open: c - 0.4 * (f / 3.).cos(), high: c + 1., low: c - 1.2, close: c, volume: None } }).collect();
        let x = Ctx::new(&r, &r, &[]);
        let t = run("dnrsi_5day", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(t.len() > 4);
        for w in t.windows(2) { assert_eq!(w[0].exit_i, w[1].legs[0].i); assert!(w[0].dir != w[1].dir); }
        for z in &t { assert_eq!(z.legs[0].px, x.o[z.legs[0].i]); }
        assert!(run("dnrsi_x", &x, 0, 5, &Params::new()).is_none());
    }
}

// Instrument admission: Dennis RSI Trading System (stock index futures manual) 2026-11-01 (titles lane K10).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "dnrsi" && reviewed(def.id),
    label: "The Dennis RSI manual trades S&P 500 futures; its 600-point stop and target are held. Runs on indices, ETFs and stocks, US and India, are a declared application.",
    basis: "Dennis RSI Trading System for Trading Stock Index Futures, manual p.2-8 (rsi2.tif - rsi8.tif); independent check authors/titles-1030/K10w/k10_ref.py",
};
