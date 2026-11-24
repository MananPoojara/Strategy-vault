//! Connors Group, "VIX Options Trading Course" (2007) slides 73, 76, 78 (short-VIX sets RSI80, 4HC, 5HH) and 97, 99, 101 (long-VIX sets RSI20, 5LC, 5LL)
//! (research share Trading Courses ... vixclass.pdf, 119 slides). Book id `vixcc`. Lane K2 (24 Nov 2026, tier 5 row I608: the six sets no live card implements).
//! Independent check authors/conv-1123/k2w/qi/vixcc_ref.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::tselhs::Book;

pub const REVIEWED: &[&str] = &["vixcc_rsi80", "vixcc_4hc", "vixcc_5hh", "vixcc_rsi20", "vixcc_5lc", "vixcc_5ll"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "A|[Declared readings, lane K2, 3] Adaptation (R4 #340): the course trades VIX options with SPY as the filter instrument and names no equity vehicle for these signals beyond 'Long SPY'; here India VIX with NIFTY 50: the short-VIX sets (RSI80, 4HC, 5HH) buy NIFTY 50 when the SMA(200) filter is above, the long-VIX sets (RSI20, 5LC, 5LL) sell NIFTY 50 short below it. Unstated, counted: (1) RSI(2) is Wilder's, taken from the VIX close; (2) a run of N days includes the signal day, on days both the VIX and NIFTY traded; (3) entries and exits are at the close of the signal day (the slides give no order timing; Connors' convention).";

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id: &'static str, name: &'static str, rules: &'static [&'static str], dir: &'static str| StrategyDef { id, name, group: "Connors VIX Options Trading Course (slides 73-101)", session: 0, dir, status: Status::Live,
        needs_volume: false, needs_vix: true, needs_series: &[], book: "vixcc", rules, notes: NOTE, claim: None };
    vec![
        d("vixcc_rsi80", "VIX RSI(2) above 80 three days in a row, index above its 200-day SMA: buy; exit when VIX RSI(2) falls below 25", &["E|Slide 73 (RSI80): VIX RSI(2) > 80 three days in a row and SPY above its 200-day moving average", "X|Exit when VIX RSI(2) < 25 (slide 73)", NOTE], "Long"),
        d("vixcc_4hc", "VIX four higher closes in a row, index above its 200-day SMA: buy; exit when VIX RSI(2) falls below 25", &["E|Slide 76 (4HC): the VIX closes higher than the previous close four days in a row and SPY is above its 200-day moving average", "X|Exit when VIX RSI(2) < 25 (slide 76)", NOTE], "Long"),
        d("vixcc_5hh", "VIX five higher highs in a row, index above its 200-day SMA: buy; exit when VIX RSI(2) falls below 25", &["E|Slide 78 (5HH): the VIX high is above the previous day's high five days in a row and SPY is above its 200-day moving average", "X|Exit when VIX RSI(2) < 25 (slide 78)", NOTE], "Long"),
        d("vixcc_rsi20", "VIX RSI(2) below 20 three days in a row, index below its 200-day SMA: sell short; exit when VIX RSI(2) rises above 70", &["E|Slide 97 (RSI20, long-VIX set): VIX RSI(2) < 20 three days in a row and SPY below its 200-day moving average", "X|Exit when VIX RSI(2) > 70 (slide 97)", NOTE], "Short"),
        d("vixcc_5lc", "VIX five lower closes in a row, index below its 200-day SMA: sell short; exit when VIX RSI(2) rises above 70", &["E|Slide 99 (5LC, long-VIX set): the VIX closes lower than the previous close five days in a row and SPY is below its 200-day moving average", "X|Exit when VIX RSI(2) > 70 (slide 99)", NOTE], "Short"),
        d("vixcc_5ll", "VIX five lower lows in a row, index below its 200-day SMA: sell short; exit when VIX RSI(2) rises above 70", &["E|Slide 101 (5LL, long-VIX set): the VIX low is below the previous day's low five days in a row and SPY is below its 200-day moving average", "X|Exit when VIX RSI(2) > 70 (slide 101)", NOTE], "Short"),
    ]
}
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { reviewed(id).then_some(("vixcc", "Connors VIX course sets", "VIX RSI(2) / run-of-days signals")) }
pub fn params(_id: &str) -> Option<Vec<ParamDef>> { None }

/// days in a row (ending at t) with cur[k] > prev (up) or < prev (down); NaN breaks the run
fn run_len(v: &[f64], t: usize, up: bool) -> usize {
    let mut n = 0;
    while t >= n + 1 && v[t - n].is_finite() && v[t - n - 1].is_finite() && (if up { v[t - n] > v[t - n - 1] } else { v[t - n] < v[t - n - 1] }) { n += 1; }
    n
}
fn all_in_a_row(v: &[f64], t: usize, n: usize, f: &dyn Fn(f64) -> bool) -> bool { t + 1 >= n && v[t + 1 - n..=t].iter().all(|&z| z.is_finite() && f(z)) }

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, _p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let ma = x.sma_n(200);
    let r = &x.vix.rsi2;
    let mut pos = Book::new();
    for t in from..=to {
        if !ma[t].is_finite() || !r[t].is_finite() && pos.side() == 0. { continue; }
        let (long, up) = (matches!(id, "vixcc_rsi80" | "vixcc_4hc" | "vixcc_5hh"), x.c[t] > ma[t]);
        if pos.side() != 0. {
            if (long && r[t] < 25.) || (!long && r[t] > 70.) { pos.exit(t, x.c[t], "VIX RSI(2) exit"); }
            continue;
        }
        let trend = if long { up } else { x.c[t] < ma[t] };
        let sig = match id {
            "vixcc_rsi80" => all_in_a_row(r, t, 3, &|z| z > 80.),
            "vixcc_4hc" => run_len(&x.vix.close, t, true) >= 4,
            "vixcc_5hh" => run_len(&x.vix.high, t, true) >= 5,
            "vixcc_rsi20" => all_in_a_row(r, t, 3, &|z| z < 20.),
            "vixcc_5lc" => run_len(&x.vix.close, t, false) >= 5,
            _ => run_len(&x.vix.low, t, false) >= 5,
        };
        if sig && trend && t < to { pos.enter(if long { 1. } else { -1. }, t, x.c[t], "close"); }
    }
    Some(pos.done(to, x.c[to]))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn runs_count_consecutive_moves_and_nan_breaks_them() {
        let v = [1., 2., 3., 4., 5., f64::NAN, 6., 7.];
        assert_eq!(run_len(&v, 4, true), 4);
        assert_eq!(run_len(&v, 7, true), 1);
        assert_eq!(run_len(&v, 4, false), 0);
    }
    /// hand-run dump for the pandas reference: VIXCC_CSV=<idx_data.csv> VIXCC_VIX=<india_vix dir> VIXCC_OUT=<file> cargo test --release vixcc_dump -- --ignored
    #[test]
    #[ignore]
    fn vixcc_dump() {
        use std::io::Write;
        let data = crate::load_csv(&std::env::var("VIXCC_CSV").unwrap()).unwrap();
        let vix = crate::load_vix_dir(&std::env::var("VIXCC_VIX").unwrap());
        let mut f = std::io::BufWriter::new(std::fs::File::create(std::env::var("VIXCC_OUT").unwrap()).unwrap());
        writeln!(f, "id,ticker,dir,entry_date,entry_px,exit_date,exit_px,open").unwrap();
        for (t, rows) in data.iter() {
            if t != "NIFTY 50" { continue; }
            let x = Ctx::new(rows, rows, &vix);
            for id in REVIEWED { for z in run(id, &x, 0, rows.len() - 1, &Params::new()).unwrap() {
                writeln!(f, "{id},{t},{},{},{},{},{},{}", z.dir, rows[z.legs[0].i].date, z.legs[0].px, rows[z.exit_i].date, z.exit_px, z.open).unwrap();
            } }
        }
    }
}

// Instrument admission: Connors VIX course sets 2026-11-24 (lane K2, tier 5).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "vixcc" && reviewed(def.id),
    label: "The course prints no results for these sets; run on NIFTY 50 with India VIX (from 2009-03-02) as a labelled adaptation of SPY and the CBOE VIX; the gate rows allow NIFTY 50 only (R4 #378).",
    basis: "Connors Group, VIX Options Trading Course (2007) slides 73, 76, 78, 97, 99, 101; independent check authors/conv-1123/k2w/qi/vixcc_ref.py",
};
