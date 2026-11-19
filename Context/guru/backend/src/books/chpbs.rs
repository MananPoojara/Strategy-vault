//! Tushar Chande, "Developing & Implementing Pattern-Based Trading Systems" (TAG XVIII New Orleans 1996, INO workbook,
//! 48 scanned pages). Almost all of it repeats Beyond Technical Analysis (chande.rs / chbta2.rs); only what the vault
//! does not already have is here. Source + page per rule: library/chpbs/README.md. Page = PDF page of workbook.pdf.
use crate::books::chbta2::{hh1, ll, none_exit, run_sys, Kind, Pos, Sys};
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "chpbs";
/// Lane K74 (R4 #277, #196(2)): the workshop's $1,500 initial stop on the named contract SPU5 ($500 per S&P point) = 3.00 S&P
/// index points, so the rule runs on SPX only (gate list), from 2014 (real opens, clamp_open). Admitted by ADMISSION_SPX.
pub const SPX_PTS: &[&str] = &["ch_pbs_cbpb_trail3"];

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, dir, status, rules, notes| StrategyDef { id, name, group: "TAG XVIII workshop · patterns", session: 0, dir, status,
        needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim: None };
    use Status::*;
    vec![
        d("ch_pbs_cbpb_trail3", "CB-PB with 3-day-low trail after a new 20-day high", "Long", Live, &[
            "F|\"Use 20-day channel\" / \"Market must make new 5-day low within next 7-days\" (p.23, slide 34); coded as the book's TradeStation line \"If Highest Bar(High,20)[1] < 7 and Low < Lowest(Low,5)[1] then buy tomorrow on the open\" (Beyond Technical Analysis p.103)",
            "E|\"Buy on the open of next day\" (p.23); \"Long trades only!\" (p.21)",
            "X|\"CB-PB PATTERN ON SPU5 / AFTER NEW 20 DAY HIGH, SET TRAILING STOP AT 3-DAY LOW\" (p.24, chart 11)",
            "X|Read: a new 20-day high = a bar from the entry bar on whose high is above the highest high of the 20 bars before it; from that bar's close a sell stop sits at the lowest low of the last 3 bars (that bar included), re-set every bar; a gap through it fills at the open",
            "X|Initial stop: the workshop's CB-PB test strategy uses a \"$1,500 initial stop\" (p.23); on the chart's contract SPU5 (S&P 500 Sep-95, $500 per point) that is exactly 3.00 S&P index points below the entry (param stop_pts; 0 = the chart's trail only). Read [lane K74, R4 #277]: the chart prints no stop of its own, so the workshop's test stop is the default",
            "X|Read: before the trail is armed the 3.00-point stop alone protects the trade; afterwards the higher of the two stops counts"],
            "Workshop pp.21-24 (slides 33-36, chart 11: S&P-500 Sep-95 futures, Apr-Jul 1995, entries/exits shown, no statistics). SPX only (lane K74, decisions #196(2), #277): the 3.00-point stop is the $1,500 of the named contract, so it is literal S&P index points and no other instrument is published; results from 2014 (real SPX opens, the entry is at the open). One position at a time; a signal while long is ignored."),
        d("ch_pbs_bfp_weekly_stocks", "Bottom-fishing pattern on stocks (weekly)", "Long", NotTestable("rules incomplete: the slides give only (H-L) > 1 and, for one chart, a trailing 30-week-low exit; the weekly low lookback, the 'recently' window, the entry order, the stop and the Goodyear exit are not stated, and Chande says he has no systematic test data ('two well chosen examples')"), &[
            "F|\"BFP on Stocks: Use weekly data ... Don't have systematic test data ... Two 'well chosen' examples\" (p.18, slide 29)",
            "E|Caterpillar weekly: \"BOTTOM FISHING PATTERN ON STOCKS (H-L) > 1 Exit on trailing 30 week low\" (p.19, chart 8)",
            "E|Goodyear weekly: \"GENERIC BOTTOM FISHING PATTERN ON STOCKS (H-L) > 1, (C-O) > 0\" (p.20, chart 9)"],
            "Also needs individual US stocks (Caterpillar, Goodyear). The daily S&P and generic patterns are already ch_bta2_bottom_fish / ch_bta2_bottom_fish_generic."),
        d("ch_pbs_triangle", "Wheat-triangle pattern match", "Long/Short", NotTestable("the 'Wheat Triangle' match indicator (percent match of a 15-bar template) is never defined; breakout rules are bullet points with no levels or orders"), &[
            "F|\"Use an old chart to define a 'successful triangle' ... Capture this pattern and test against other markets\" (p.6)",
            "E|\"Breakout beyond 'horizontal' line / 'sloped' line\" (p.10, Trading Triangles)"],
            "Charts 3-5 (pp.7-9) show the indicator, no rule."),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "ch_pbs_cbpb_trail3" => ("ch_cbpb", "Channel breakout–pullback", "3-day-low trail after new 20-day high · workshop"),
        "ch_pbs_bfp_weekly_stocks" => ("ch_bottom", "Bottom-fishing pattern", "Weekly stocks · not testable"),
        "ch_pbs_triangle" => ("ch_pbs_triangle", "Wheat-triangle pattern", ""),
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    match id {
        "ch_pbs_cbpb_trail3" => Some(vec![
            p("high_n", "Channel (HighestBar) days", 20., 10., 40., 5.),
            p("within", "Channel high at most N-1 bars old", 7., 3., 15., 1.),
            p("low_n", "New N-day low", 5., 2., 10., 1.),
            p("nh_n", "New N-day high arms the trail", 20., 5., 40., 5.),
            p("trail_n", "Trailing stop at N-day low", 3., 1., 10., 1.),
            p("stop_pts", "Initial stop, S&P index points ($1,500 / $500 on SPU5 = 3; 0 = off)", 3., 0., 50., 1.)]),
        _ => None,
    }
}

/// HighestBar(High,n)[1] < within and Low < Lowest(Low,m)[1] (BTA p.103). HighestBar on yesterday's n-bar window,
/// most recent tie wins (TradeStation), same reading as chande.rs.
pub(crate) fn cbpb(x: &Ctx, i: usize, n: usize, within: usize, m: usize) -> bool {
    if i < n.max(m) { return false; }
    let mut best = i - 1;
    for j in (i - n..i).rev() { if x.h[j] > x.h[best] { best = j; } }
    i - 1 - best < within && x.l[i] < x.l[i - m..i].iter().copied().fold(f64::INFINITY, f64::min)
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if id != "ch_pbs_cbpb_trail3" { return None; }
    let schema = params(id)?;
    if from > to || to >= x.r.len() { return None; }
    let g = |k: &str| p.get(k).copied().filter(|v| v.is_finite()).unwrap_or_else(|| schema.iter().find(|d| d.key == k).unwrap().def);
    let u = |k: &str| g(k).round().max(1.) as usize;
    let (n, w, m, nh, tn) = (u("high_n"), u("within"), u("low_n"), u("nh_n"), u("trail_n"));
    // last bar (<= i) that made a new nh-day high
    let mut last_nh = vec![usize::MAX; x.r.len()];
    let mut cur = usize::MAX;
    for i in 0..x.r.len() {
        if i >= nh && x.h[i] > hh1(x, i, nh) { cur = i; }
        last_nh[i] = cur;
    }
    let entry = |i: usize, _: f64| [cbpb(x, i, n, w, m).then_some((1., Kind::Open)), None];
    let trail = |i: usize, q: &Pos| if last_nh[i] != usize::MAX && last_nh[i] >= q.i { ll(x, i, tn) } else { f64::NAN };
    Some(run_sys(x, from, to, &Sys { entry: &entry, exit_close: &none_exit, exit_open: &none_exit, trail: &trail,
        stop_pts: g("stop_pts").max(0.), oneway: false }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    fn rows() -> Vec<Row> {
        (0..60).map(|i| Row { ticker: "T".into(), date: format!("{:04}-01-01", 2000 + i), open: 100., high: 101., low: 99., close: 100., volume: None }).collect()
    }
    #[test]
    fn trail_arms_only_after_a_new_20_day_high_in_the_trade() {
        let mut r = rows();
        r[20].high = 110.;                        // channel high (age 4 at bar 25)
        r[25].low = 98.;                          // new 5-day low -> buy bar 26 open
        r[26].open = 100.5;
        r[30].high = 111.; r[30].low = 99.5;      // new 20-day high on bar 30 -> stop = min(L[28..30]) = 99 for bar 31
        r[31].open = 100.; r[31].low = 98.5;      // hits 99
        let x = Ctx::new(&r, &r, &[]);
        let t = run("ch_pbs_cbpb_trail3", &x, 0, 59, &stop0()).unwrap();
        assert_eq!(t[0].legs[0].i, 26); assert_eq!(t[0].legs[0].px, 100.5);
        assert_eq!(t[0].exit_i, 31); assert_eq!(t[0].exit_px, 99.);
        // without the new high the trade never exits
        let mut r2 = r.clone(); r2[30].high = 101.;
        let x2 = Ctx::new(&r2, &r2, &[]);
        let t2 = run("ch_pbs_cbpb_trail3", &x2, 0, 59, &stop0()).unwrap();
        assert_eq!(t2.len(), 1); assert!(t2[0].open);
        // the default 3.00-point initial stop (workshop $1,500 / $500 per point) protects it before the trail is armed
        let mut r3 = r2.clone(); r3[28].low = 97.;
        let x3 = Ctx::new(&r3, &r3, &[]);
        let t3 = run("ch_pbs_cbpb_trail3", &x3, 0, 59, &Params::new()).unwrap();
        assert_eq!((t3[0].exit_i, t3[0].exit_px, t3[0].reason.as_str()), (28, 97.5, "Initial stop"));
        assert!(crate::gate::get("ch_pbs_cbpb_trail3").is_some_and(|g| g.allowed == Some(&["SPX"][..]) && g.clamp_open));
    }
    fn stop0() -> Params { [("stop_pts".to_string(), 0.0)].into_iter().collect() }
}

// Lane K74 (11 Nov 2026, R4 #277 / #196(2)): CB-PB with the 3-day-low trail, SPX only through the gate list.
pub const ADMISSION_SPX: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == BOOK && SPX_PTS.contains(&def.id),
    label: "Tushar Chande showed this CB-PB trade on S&P 500 futures (SPU5, 1995) and tested CB-PB with a $1,500 initial stop per contract, which on that contract ($500 per index point) is 3.00 points. It runs on the S&P 500 index (SPX) only, the 3.00 points literal, from 2014 (real opens), as a declared application of the futures rule to the cash index. Other instruments would need a conversion the workshop does not give.",
    basis: "Tushar Chande: Developing & Implementing Pattern-Based Trading Systems (TAG XVIII workshop 1996, INO workbook) p.21-24 (CB-PB definition, test strategy with the $1,500 initial stop, chart 11 'CB-PB pattern on SPU5 after new 20 day high, set trailing stop at 3-day low'), Beyond Technical Analysis p.103 (the CB-PB TradeStation line); decisions #196(2), #240, #277; independent check authors/titles-1030/K74w (k74_spx.py).",
};
