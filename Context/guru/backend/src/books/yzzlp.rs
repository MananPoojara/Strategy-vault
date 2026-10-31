//! Kathy Yuan, Lu Zheng and Qiaoqiao Zhu, "Are Investors Moonstruck? Lunar Phases and Stock Returns" (working paper, 2001;
//! research share USB HDD 2/tRADING KNO-2/Kathy Yuan - Lunar Phases and Stock Returns (Article)/). Book id `yzzlp`. One rule:
//! the paper's section III.C trading strategy, long in new-moon periods and short in full-moon periods. Lane K8 (titles loop,
//! 1 Nov 2026); independent check authors/titles-1030/K8w/k8_ref.py (header).
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::kets::{jdn, lunar_event, ymd};
use super::rbtie::target_trades;
use std::collections::HashSet;

/// Admitted ids (authors/titles-1030/K8w/k8_ref.py (header)).
pub const REVIEWED: &[&str] = &["yzzlp_lunar"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] A period is N calendar days before the moon day + the moon day + N days after (p.12), N = 7 (the '15-day window' used for the trading strategy; N = 3 is an option). The paper's dummy marks the full-moon period, so where the two windows touch the full-moon period wins (footnote 4: 'a new moon period can be less than 15 days'); a day in neither window (several days each cycle with N = 3, now and then one day with N = 7) is flat. New and full moon dates come from the vault's own astronomical routine, the one used for Katz's lunar tests (Numerical Recipes flmoon in Katz's LunarEventDates, US Eastern civil date; the paper used a GMT table from life-cycles-destiny.com, so a phase near midnight can fall one day apart). The position for each trading day is taken at the previous close (the paper measures each day's close-to-close return), so the switch happens at the close of the last day of a period; filling at the next open is an option. Long only (new-moon periods, cash otherwise) is an option. The paper trades an equal-weighted portfolio of 48 country indices (4.2% a year, 15-day window); one index, ETF or stock at a time is a declared application, and that figure is not comparable.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "yzzlp_lunar", name: "Lunar phases: long in new-moon periods, short in full-moon periods (15-day windows), at the close", group: "Yuan, Zheng & Zhu lunar phases",
        session: 0, dir: "Long/Short", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: "yzzlp", rules: &[
            "F|\"We define a full moon period as N days before the full moon day + the full moon day + N days after the full moon day (N = 3 or 7). Similarly, we define a new moon period\" (p.12)",
            "E|\"We then short this global portfolio during full moon periods and long this portfolio during new moon periods\" (p.17)",
            "X|The position switches when the period changes; \"we trade about 24 times a year\" (p.18)"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    (id == "yzzlp_lunar").then_some(("yzzlp_lunar", "Yuan-Zheng-Zhu lunar phases", "15-day windows"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    (id == "yzzlp_lunar").then(|| vec![p("n", "Days before and after the moon day (7 = the 15-day window; 3 = the 7-day window)", 7., 1., 7., 1.),
        p("side", "1 = long only in new-moon periods (default 0: also short in full-moon periods)", 0., 0., 1., 1.),
        p("open", "1 = fill at the next open (default 0: at the close)", 0., 0., 1., 1.)])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(f64::NAN, |d| d.def));
    let n = g("n").round().max(0.) as i64;
    let days: Vec<i64> = x.r.iter().map(|r| { let (y, m, d) = ymd(&r.date); jdn(y, m, d) }).collect();
    let k = |d: i64| ((d as f64 - 2415020.76) / 7.38265).floor() as i64;
    let (mut new, mut full) = (HashSet::new(), HashSet::new());
    for q in k(days[0]) - 12..=k(days[days.len() - 1]) + 12 {
        match q.rem_euclid(4) { 0 => { new.insert(lunar_event(q)); } 2 => { full.insert(lunar_event(q)); } _ => {} }
    }
    let short = if g("side") >= 1. { 0 } else { -1 };
    let state = |d: i64| if (-n..=n).any(|j| full.contains(&(d + j))) { short } else if (-n..=n).any(|j| new.contains(&(d + j))) { 1 } else { 0 };
    let tgt: Vec<i8> = (0..days.len()).map(|i| state(*days.get(i + 1).unwrap_or(&days[i]))).collect();
    Some(target_trades(x, &tgt, from, to, g("open") >= 1., "lunar period changed"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn new_moon_long_full_moon_short() {
        // new moon 2000-01-06, full moon 2000-01-20 (21 Jan 04:40 UT = 20 Jan US Eastern; Katz's routine)
        let r: Vec<Row> = (1..=31).map(|d| Row { ticker: "T".into(), date: format!("2000-01-{d:02}"), open: 100., high: 101., low: 99., close: 100. + d as f64, volume: None }).collect();
        let x = Ctx::new(&r, &r, &[]);
        let t = run("yzzlp_lunar", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert_eq!(t[0].dir, 1.);
        assert_eq!(t[1].dir, -1.);
        // the full-moon window is Jan 13-27: the short starts at Jan 12's close and ends at Jan 27's (Jan 28 is in neither window)
        assert_eq!((r[t[1].legs[0].i].date.as_str(), r[t[1].exit_i].date.as_str()), ("2000-01-12", "2000-01-27"));
    }
}

// Instrument admission: Yuan, Zheng & Zhu (lunar phases) 2026-11-01 (titles lane K8).
// Admission only for the module REVIEWED lists (authors/titles-1030/K8w/k8_ref.py (header)).
// Runs on actual indices, ETFs and stocks, US and India (index_etf_or_stock, nifty50 basket, declared application).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "yzzlp" && crate::books::yzzlp::reviewed(def.id),
    label: "Tested in the paper on an equal-weighted portfolio of 48 country indices; single indices, ETFs and US/India stocks are a declared application.",
    basis: "Yuan, Zheng & Zhu, Are Investors Moonstruck? (2001) p.12 (periods) and p.17-18 (III.C trading strategy); moon dates from Katz's LunarEventDates routine (books/kets.rs); independent check authors/titles-1030/K8w/k8_ref.py",
};
