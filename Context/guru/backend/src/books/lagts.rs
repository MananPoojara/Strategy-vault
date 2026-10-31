//! Luiz Alvim, The Global Trading System (alphafin.com; Word 6 manual, ch.3 'Making the Trade Decision', sec.3.2 Rules 1-7,
//! sec.3.2.1-3.2.4; research share USB HDD 2/tRADING KNO-2/Luiz Alvin - The Global System (alphafin.com)/Global 1-10.doc, with
//! the indicator settings in Notes.txt in that folder). Book id `lagts`. Lane K7 (titles loop, 1 Nov 2026); rule text
//! authors/batch-1101/B1w/alvim_global.txt; independent check authors/titles-1030/K7w/k7_ref.py (header).
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::mkits::ema;
use super::rbtie::mk;

/// Admitted ids (authors/titles-1030/K7w/k7_ref.py (header)).
pub const REVIEWED: &[&str] = &["lagts_global"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] Indicators from the folder notes: 5/5 Range Channel = 5-bar exponential averages of the highs and of the lows; Slow Alpha Indicator (ASI) = MACD 19,39,1, i.e. EMA19(C) - EMA39(C) (signal length 1 = the line itself); averages seeded with the simple average of their first values. The ASI direction is up on a bar where it rises, down where it falls, unchanged when equal; a 'turn' (top or bottom) is a change of direction; the stretch before the first turn gives no signal. Rule 1 sell: the close is lower than each of the two previous closes, lies inside the channel (5-bar EMA of lows <= close <= 5-bar EMA of highs), and the ASI is falling after a top; buy mirrored. One entry per ASI swing: the first such 3-bar reversal after each ASI turn (the author counts nine signals for nine ASI reversals on silver); option: every qualifying reversal while flat and the ASI keeps its direction. Rule 6: enter at the next day's open (position trading). Rule 2: stay until the ASI turns the other way, out at the next open (option: that close); Rule 5: the reverse trade comes only when Rule 1 holds in the new direction. Rule 3 stop: 3 ticks below the lowest low (above the highest high for shorts) of the five bars up to and including the signal bar, fixed at entry and checked from the entry day, filled at the stop or a worse open; a tick is the instrument's own (0.01 US, 0.05 NSE; option 0 ticks). Not applied: the $1,000-per-contract stop cap (needs a contract size), Rule 4 adding contracts (single position here), the discretionary 'non-mechanical' judgement the author allows, ch.4 'Maximizing the Profits' and the ch.5 stock adaptations (not in the folder). Option: the beginner filter (trade only in the direction of the 40-bar EMA slope). The author traded futures (silver, Swiss franc) on any time frame; daily bars on indices, ETFs and stocks, US and India, are a declared application. The manual shows chart counts only, no test.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "lagts_global", name: "Global Trading System: 3-bar reversal close inside the 5/5 channel after a Slow Alpha (MACD 19/39) turn, out when the Slow Alpha turns back",
        group: "Alvim Global Trading System", session: 0, dir: "Long/Short", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: "lagts", rules: &[
            "F|\"5/5 Range Channel: 5 bar Exponential MA of the highs and a 5 bar Exponential MA of the lows\"; \"The Alpha Oscillators: Are simply MACD. The slow one is set to 19,39,1\" (Notes.txt)",
            "E|Rule 1: \"Sell at the market after the third lowest bar Close is inside the 5/5 Range Channel and the Slow Alpha Indicator line turned down after making a top; Buy at the market after the third highest bar Close is inside the 5/5 Range Channel and the Slow Alpha Indicator line has turned up after making a bottom\"; Rule 6: \"Enter all orders at the market immediately after the signal (open of next trading day if position trading)\" (ch.3 sec.3.2)",
            "X|Rule 2: \"Stay with that trade until the Slow Alpha Indicator reverses to the opposite direction\"; Rule 5: \"Reverse the position when the Slow Alpha Indicator turns in the opposite direction of the trade ... within Rule 1 requirements\" (sec.3.2)",
            "X|Rule 3: \"If going short place a stop three ticks above the highest High of the previous five bars or $1,000 from the fill, whichever is lower; If going long place a stop three ticks below the lowest Low of the previous five bars\" (sec.3.2; the $1,000 cap is not applied)"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    (id == "lagts_global").then_some(("lagts_global", "Alvim Global Trading System", "5/5 channel, Slow Alpha 19/39"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    (id == "lagts_global").then(|| vec![
        ParamDef { key: "every", label: "0 = first 3-bar reversal after each Slow Alpha turn (reading), 1 = every qualifying reversal while flat", def: 0., min: 0., max: 1., step: 1. },
        ParamDef { key: "exit_close", label: "Slow Alpha turn exit: 0 = next open (reading), 1 = that close", def: 0., min: 0., max: 1., step: 1. },
        ParamDef { key: "ticks", label: "Stop offset beyond the 5-bar low/high, in ticks (book 3)", def: 3., min: 0., max: 20., step: 1. },
        ParamDef { key: "beginner", label: "1 = beginner filter: only in the direction of the 40-bar EMA slope", def: 0., min: 0., max: 1., step: 1. },
    ])
}

/// Slow Alpha direction per bar: 1 rising, -1 falling, the previous direction when equal, 0 while undefined;
/// and the swing number (0 = before the first turn).
pub fn asi_swings(c: &[f64]) -> (Vec<i8>, Vec<u32>) {
    let (e19, e39) = (ema(c, 19), ema(c, 39));
    let a: Vec<f64> = e19.iter().zip(&e39).map(|(x, y)| x - y).collect();
    let (mut dir, mut sw) = (vec![0i8; c.len()], vec![0u32; c.len()]);
    for i in 1..c.len() {
        let prev = dir[i - 1];
        dir[i] = if !(a[i].is_finite() && a[i - 1].is_finite()) { 0 } else if a[i] > a[i - 1] { 1 } else if a[i] < a[i - 1] { -1 } else { prev };
        sw[i] = sw[i - 1] + (prev != 0 && dir[i] != 0 && dir[i] != prev) as u32;
    }
    (dir, sw)
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let g = |k: &str, d: f64| p.get(k).copied().unwrap_or(d);
    let (every, exit_close, beginner) = (g("every", 0.) >= 0.5, g("exit_close", 0.) >= 0.5, g("beginner", 0.) >= 0.5);
    let off = g("ticks", 3.).max(0.).round() * x.market_tick();
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let (eh, el, e40) = (ema(h, 5), ema(l, 5), ema(c, 40));
    let (dir, sw) = asi_swings(c);
    // Rule 1 direction on bar i (0 = none)
    let rule1 = |i: usize| -> i8 {
        if i < 4 || sw[i] == 0 || !(el[i] <= c[i] && c[i] <= eh[i]) { return 0; }
        let d = dir[i];
        let rev = if d > 0 { c[i] > c[i - 1] && c[i] > c[i - 2] } else { c[i] < c[i - 1] && c[i] < c[i - 2] };
        let slope = !beginner || (d > 0 && e40[i] > e40[i - 1]) || (d < 0 && e40[i] < e40[i - 1]);
        if rev && slope { d } else { 0 }
    };
    // first qualifying bar of each swing, over all bars (an indicator of the data, not of the window)
    let mut is_first = vec![false; c.len()];
    if !every { let mut last = u32::MAX; for i in 0..c.len() { if sw[i] != last && rule1(i) != 0 { is_first[i] = true; last = sw[i]; } } }
    let mut out = vec![];
    let mut pos: Option<(f64, usize, f64, f64)> = None; // (dir, entry bar, entry price, stop)
    let (mut exit_open, mut pend): (bool, Option<(f64, f64)>) = (false, None); // pending entry (dir, stop)
    for i in from..=to {
        if exit_open {
            if let Some((d, e, ep, _)) = pos.take() { out.push(mk(d, e, ep, "open", i, o[i], "Slow Alpha turned", false, i - 1)); }
            exit_open = false;
        }
        if let Some((d, st)) = pend.take() { pos = Some((d, i, o[i], st)); }
        if let Some((d, e, ep, st)) = pos {
            if (d > 0. && l[i] <= st) || (d < 0. && h[i] >= st) {
                let px = if d > 0. { o[i].min(st) } else { o[i].max(st) };
                out.push(mk(d, e, ep, "open", i, px, "stop loss", false, i)); pos = None;
            } else if dir[i] as f64 == -d {
                if exit_close { out.push(mk(d, e, ep, "open", i, c[i], "Slow Alpha turned", false, i)); pos = None; }
                else if i < to { exit_open = true; }
            }
        }
        if i < to && (pos.is_none() || exit_open) {
            let s = rule1(i);
            if s != 0 && (every || is_first[i]) {
                let d = s as f64;
                let st = if d > 0. { l[i - 4..=i].iter().copied().fold(f64::INFINITY, f64::min) - off } else { h[i - 4..=i].iter().copied().fold(f64::NEG_INFINITY, f64::max) + off };
                pend = Some((d, st));
            }
        }
    }
    if let Some((d, e, ep, _)) = pos { out.push(mk(d, e, ep, "open", to, c[to], "open", true, to)); }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn entries_follow_rule1_and_exits_follow_the_slow_alpha() {
        let c: Vec<f64> = (0..400).map(|i| 100. + 8. * (i as f64 / 9.).sin() + 3. * (i as f64 / 2.3).cos() + i as f64 * 0.03).collect();
        let (dir, sw) = asi_swings(&c);
        assert!(dir[38] == 0 && dir[39] != 0 && sw[39] == 0 && *sw.last().unwrap() > 3);
        let rows: Vec<Row> = c.iter().enumerate().map(|(i, &v)| Row { ticker: "T".into(), date: format!("{}", i), open: v, high: v + 1., low: v - 1., close: v, volume: None }).collect();
        let x = Ctx::new(&rows, &rows, &[]);
        let t = run("lagts_global", &x, 0, rows.len() - 1, &Params::new()).unwrap();
        assert!(!t.is_empty());
        for z in &t {
            let s = z.legs[0].i - 1;
            assert_eq!(dir[s] as f64, z.dir);
            assert!(if z.dir > 0. { c[s] > c[s - 1] && c[s] > c[s - 2] } else { c[s] < c[s - 1] && c[s] < c[s - 2] });
        }
        // one entry per swing by default
        let mut seen = std::collections::BTreeSet::new();
        for z in &t { assert!(seen.insert(sw[z.legs[0].i - 1])); }
        let mut p = Params::new(); p.insert("every".into(), 1.);
        assert!(run("lagts_global", &x, 0, rows.len() - 1, &p).unwrap().len() >= t.len());
        assert!(run("lagts_x", &x, 0, 5, &Params::new()).is_none());
    }
}

// Instrument admission: Luiz Alvim (The Global Trading System) 2026-11-01 (titles lane K7).
// Admission only for the module REVIEWED lists (authors/titles-1030/K7w/k7_ref.py (header)).
// Runs on actual indices, ETFs and stocks, US and India (index_etf_or_stock, nifty50 basket, declared application).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "lagts" && crate::books::lagts::reviewed(def.id),
    label: "Luiz Alvim traded these rules on futures (silver, Swiss franc) on any time frame, with chart examples only; daily indices, ETFs and US/India stocks are a declared application.",
    basis: "Luiz Alvim, The Global Trading System (alphafin.com), ch.3 sec.3.2 Rules 1-7 and sec.3.2.1-3.2.4 (Global 1-10.doc), indicator settings in Notes.txt",
};
