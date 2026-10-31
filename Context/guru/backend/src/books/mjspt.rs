//! Michael Johnson, 'Sign Post Trading' (the 3-day moving-average changeover with a 25-point target and a 20-point stop),
//! a chapter in Stephen Pierce (ed.), Amazing Trading Plans (Impulsive Profits, 2004), PDF p.60-69; research share USB HDD 2/
//! tRADING KNO-2/Stephen A.Pierce - Amazing Trading Plans (536 MB) (amazingtradingplans.com)/CD5 - Amazing Trading Plans
//! Manual/Amazing Manual.pdf. Book id `mjspt`. Lane K7 (titles loop, 1 Nov 2026); rule text
//! authors/batch-c/C2w/pierce_signpost.txt; independent check authors/titles-1030/K7w/k7_ref.py (header).
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::mpts::sma;
use super::rbtie::mk;

/// Admitted ids (authors/titles-1030/K7w/k7_ref.py (header)).
pub const REVIEWED: &[&str] = &["mjspt_3dma"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] 3dma = the simple average of the last three closes ('always based on the close and is simple', p.60; his rounding to whole points is not applied). Changeover up = today's 3dma above yesterday's after yesterday's was at or below the day before; down mirrored. Each changeover enters in its direction at the next day's open (he enters at the next session's open, p.61), closing a trade still open from the other direction at the same price ('you exit one trade and take the opposite one at the same time', p.64). Exits from the entry day: a target 25 points and a stop 20 points from the entry, one cancels the other (p.61-62), filled at the level or a worse/better open; when both are inside one day the stop is taken (conservative). The points are SPI (S&P/ASX 200) index points at about 3,500 (2004), so here they are percentages of the entry price: target 0.714% (25/3500), stop 0.571% (20/3500), labelled; other widths are options. Not coded: his trailing stop and 'stop a trade at any time it looks to be doubtful' (judgement), and the optional slow-stochastic trend filter (p.66-67). The author trades SPI futures; US and India indices, ETFs and stocks are a declared application. His SPI table (30 Mar-28 Apr 2004: +25, +25, -20, +1, -4, +25) is on an index the vault does not hold.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "mjspt_3dma", name: "Sign Post Trading: trade each 3-day moving-average changeover, 25-point target / 20-point stop (as % of entry)",
        group: "Johnson Sign Post Trading", session: 0, dir: "Long/Short", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: "mjspt", rules: &[
            "F|\"3dma for my intents and purposes is always based on the close and is simple\"; \"Add the 'close' for the last three days and divide your total by 3\" (p.60)",
            "E|\"my entry point is the changeover from one direction to another on the 3dma\" (p.60); \"Entry Points are change of 3dma direction\" (p.65)",
            "X|\"I Buy at 3500 ... 1. Sell at 3525. 2. Sell at 3480 on stop. 3. OCO GAM\" (p.61); \"play the 20 Point stop loss 25 point stop profit every time\" (p.62); \"Exit Points are + 25 or - 20 or new changeover of direction\" (p.65)"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    (id == "mjspt_3dma").then_some(("mjspt_3dma", "Johnson Sign Post Trading", "3dma changeover, 25/20 points as %"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    (id == "mjspt_3dma").then(|| vec![
        ParamDef { key: "target", label: "Target (% of entry; 25 of 3,500 points = 0.714)", def: 25. / 35., min: 0.05, max: 10., step: 0.01 },
        ParamDef { key: "stop", label: "Stop (% of entry; 20 of 3,500 points = 0.571)", def: 20. / 35., min: 0.05, max: 10., step: 0.01 },
    ])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let tg = p.get("target").copied().unwrap_or(25. / 35.) / 100.;
    let sl = p.get("stop").copied().unwrap_or(20. / 35.) / 100.;
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let m = sma(c, 3);
    let turn = |i: usize| -> f64 {
        if i < 2 { return 0.; }
        if m[i] > m[i - 1] && m[i - 1] <= m[i - 2] { 1. } else if m[i] < m[i - 1] && m[i - 1] >= m[i - 2] { -1. } else { 0. }
    };
    let mut out = vec![];
    let mut pos: Option<(f64, usize, f64)> = None;
    let mut pend = 0.;
    for i in from..=to {
        if pend != 0. {
            if let Some((d, e, ep)) = pos.take() { out.push(mk(d, e, ep, "open", i, o[i], "3dma changeover", false, i - 1)); }
            pos = Some((pend, i, o[i])); pend = 0.;
        }
        if let Some((d, e, ep)) = pos {
            let (st, tp) = (ep * (1. - d * sl), ep * (1. + d * tg));
            if (d > 0. && l[i] <= st) || (d < 0. && h[i] >= st) {
                out.push(mk(d, e, ep, "open", i, if d > 0. { o[i].min(st) } else { o[i].max(st) }, "stop", false, i)); pos = None;
            } else if (d > 0. && h[i] >= tp) || (d < 0. && l[i] <= tp) {
                out.push(mk(d, e, ep, "open", i, if d > 0. { o[i].max(tp) } else { o[i].min(tp) }, "target", false, i)); pos = None;
            }
        }
        if i < to { pend = turn(i); }
    }
    if let Some((d, e, ep)) = pos { out.push(mk(d, e, ep, "open", to, c[to], "open", true, to)); }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn every_changeover_enters_and_exits_are_target_stop_or_the_next_changeover() {
        // the book's own 3dma table (p.60): up from day 3, down from day 7
        let c = [3500., 3520., 3530., 3540., 3555., 3540., 3520., 3500., 3510., 3530., 3550.];
        let m = sma(&c, 3);
        assert!(m[6] < m[5] && m[5] > m[4]);
        let rows: Vec<Row> = c.iter().enumerate().map(|(i, &v)| Row { ticker: "T".into(), date: format!("{}", i), open: v, high: v + 5., low: v - 5., close: v, volume: None }).collect();
        let x = Ctx::new(&rows, &rows, &[]);
        let t = run("mjspt_3dma", &x, 0, rows.len() - 1, &Params::new()).unwrap();
        // turn down at day 7 (index 6) -> short at index 7's open; turn up at index 9 -> long at index 10
        assert_eq!(t[0].dir, -1.); assert_eq!(t[0].legs[0].i, 7);
        assert!(t.iter().any(|z| z.dir > 0. && z.legs[0].i == 10));
        assert!(run("mjspt_x", &x, 0, 5, &Params::new()).is_none());
    }
}

// Instrument admission: Michael Johnson (Sign Post Trading, in Amazing Trading Plans) 2026-11-01 (titles lane K7).
// Admission only for the module REVIEWED lists (authors/titles-1030/K7w/k7_ref.py (header)).
// Runs on actual indices, ETFs and stocks, US and India (index_etf_or_stock, nifty50 basket, declared application).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "mjspt" && reviewed(def.id),
    label: "Michael Johnson traded the 3dma changeover on SPI (S&P/ASX 200) futures with point targets; here the points are a percentage of the entry, and indices, ETFs and US/India stocks are a declared application.",
    basis: "Michael Johnson, Sign Post Trading, in Stephen Pierce (ed.), Amazing Trading Plans (2004), PDF p.60-69",
};
