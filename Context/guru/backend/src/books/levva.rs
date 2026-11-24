//! Larry Levin, "Secrets of Floor Traders" course (1999 / 2001 editions), Rules.pdf chapter 2 "The Value Area & The 80% Rule" pp.11-31 (scanned; RapidOCR text saved in
//! authors/conv-1123/K1/levin/rules_ocr.txt, all 14 pages read). Book id `levva`. Lane K1 (24 Nov 2026, R4 #397 tier 5): the 80% rule on the NIFTY 50 / NIFTYBANK futures
//! 1-minute series with volume (lane DI) as 30-minute brackets, a labelled R4 #340 adaptation; one scaling rule R4 #380. Helper books::k4x. Reference authors/code-1123/ref/levva_ref.py.
use super::k4x::{fut_bars, o, run_fb, A, P};
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use std::collections::HashMap;

const ID: &str = "levva_value_area_80";
const NOTE: &str = "[Declared adaptation, R4 #340 / #380 / #339] Larry Levin's Value Area 80% Rule (Secrets of Floor Traders, Rules.pdf pp.11-31, e-mini S&P). FROM THE BOOK: the Value Area is the range of prices where 70% of yesterday's volume took place; when the market is outside it (opened outside, or left it during the day, p.27) and then is inside it for two consecutive 30-minute brackets it has an 80% chance of filling the area; go long as near the bottom of the area as possible (short near the top) with a stop a few ticks beyond it (printed example: area 68500-68720, stop 68480, i.e. 20 index units) and cover at the other edge; reversal only with other indicators (not coded). Run on the NIFTY 50 / NIFTYBANK futures 1-minute bars with volume. The stop uses ONE scaling rule (#380): 20/68,500 of the edge price. UNSTATED DETAILS, four, each a separate choice: (1) the Value Area, which the course does not compute (it defers to Market Profile software), is the central 70% of the previous session's volume: each 1-minute bar's volume is put at its close on the 0.05 price grid and the area runs from the 15th to the 85th cumulative-volume percentile; (2) the two consecutive 30-minute brackets are read on bar closes (two consecutive 30-minute bars from the session open that close inside the area, after an earlier bar of the session reached beyond it) and the signal is acted on at the second bar's close, the book's wording (the next bar opens inside) not being testable on bar data without look-ahead; (3) the entry is a limit at the edge re-issued each bar until it fills or the other edge (the target) has traded, the side being that of the most recent bar to reach beyond the area (a bar reaching both sides gives no signal), and after an exit a fresh two-bracket signal is needed; (4) the position is closed at the last bar of the session. No profit figure is printed: 'about 8 out of 10'.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef {
        id: ID, name: "Levin Value Area 80% Rule (previous-day 70% volume area, two 30-minute brackets back inside, fill the area; NIFTY / BANKNIFTY futures)", group: "Secrets of Floor Traders (Larry Levin)", session: 0,
        dir: "Long/Short", status: Status::Live, needs_volume: true, needs_vix: false, needs_series: &[], book: "levva",
        rules: &[
            "F|\"The Value Area: Is the range of prices where 70% of yesterday's volume took place.\" \"The 80% Rule: Is when the market gets (or opens) above or below the Value Area, and then gets in the Value Area for two consecutive half hour periods. The market then has an 80% chance of filling the Value Area.\" (p.11); the market need not open outside the area (p.27)",
            "E|\"I would then look to get long as close to the bottom of the Value Area as possible (but only after the market has been in the Value Area for two consecutive brackets). I would put a sell stop just below the Value Area ... 68480\"; the mirror for a short with a buy stop just above (pp.22-23)",
            "X|\"I will usually look to cover this position at the top of the Value Area\" (long) / the bottom (short); stop a few ticks beyond the area; reversal \"only if your other indicators are showing\" the opposite signal (pp.22-23, p.30)",
        ],
        notes: NOTE, claim: None,
    }]
}
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { (id == ID).then_some(("levva", "Levin Value Area 80% Rule", "30-minute NIFTY / BANKNIFTY futures")) }
pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    (id == ID).then(|| vec![p("va", "Value Area share of volume (percent)", 70., 50., 90., 5.), p("brackets", "Consecutive brackets inside", 2., 1., 4., 1.), p("stop", "Stop beyond the area (units of 68,500)", 20., 5., 100., 5.)])
}

/// Value Area (low, high) of one session: 1-minute volume at each close on a 0.05 grid; the range holding the central `pct` percent of the volume
/// (from the (100-pct)/2 to the 100-(100-pct)/2 cumulative-volume percentile).
pub fn value_area(close: &[f64], vol: &[f64], pct: f64) -> Option<(f64, f64)> {
    let mut m: std::collections::BTreeMap<i64, f64> = std::collections::BTreeMap::new();
    for (c, v) in close.iter().zip(vol) { *m.entry((c * 20.0).round() as i64).or_insert(0.0) += v; }
    let total: f64 = m.values().sum();
    if total <= 0.0 { return None; }
    let tail = (100.0 - pct) / 200.0 * total;
    let (mut cum, mut lo, mut hi) = (0.0, None, None);
    for (&k, &v) in m.iter() {
        cum += v;
        if lo.is_none() && cum >= tail - 1e-9 { lo = Some(k); }
        if hi.is_none() && cum >= total - tail - 1e-9 { hi = Some(k); }
    }
    Some((lo? as f64 / 20.0, hi? as f64 / 20.0))
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if id != ID || to >= x.c.len() || from > to { return None; }
    let g = |k: &str| *p.get(k).unwrap_or(&params(id).unwrap().iter().find(|d| d.key == k).unwrap().def);
    let Some(fb) = fut_bars(x, from, to, 30) else { return Some(vec![]) };
    let (pct, need, stopu) = (g("va"), g("brackets").round() as usize, g("stop") / 68500.0);
    // Value Area of each day's predecessor (the previous day present in the bar series)
    let mut va: HashMap<usize, (f64, f64)> = HashMap::new();
    let mut prev: Option<usize> = None;
    for k in 0..fb.day.len() {
        if k > 0 && fb.day[k] == fb.day[k - 1] { continue; }
        if let Some(pi) = prev {
            if let Some((m, mv)) = x.fut.and_then(|f| f.fut_day(&x.r[pi].date)) {
                let cl: Vec<f64> = m.iter().map(|z| z.c).collect();
                if let Some(a) = value_area(&cl, &mv, pct) { va.insert(fb.day[k], a); }
            }
        }
        prev = Some(fb.day[k]);
    }
    let (mut run_in, mut last_out, mut armed): (usize, i8, i8) = (0, 0, 0);
    Some(run_fb(&fb, |k, st, now, next| {
        let new_day = k == 0 || fb.day[k] != fb.day[k - 1];
        if new_day { run_in = 0; last_out = 0; armed = 0; }
        if fb.last[k] {
            if st.pos != 0 { now.push(o(if st.pos > 0 { A::Xl } else { A::Xs }, P::Mkt, "end of day")); }
            return;
        }
        let Some(&(val, vah)) = va.get(&fb.day[k]) else { return };
        let (h, l, c) = (fb.h[k], fb.l[k], fb.c[k]);
        if st.pos != 0 { armed = 0; run_in = 0; } // a new trade needs a fresh two-bracket signal
        if st.pos == 0 {
            if l < val || h > vah { last_out = if l < val && h > vah { 0 } else if l < val { -1 } else { 1 }; } // a bar reaching both sides is no signal
            run_in = if c >= val && c <= vah { run_in + 1 } else { 0 };
            if armed == 0 && last_out != 0 && run_in >= need { armed = -last_out; } // beyond below -> long (+1)
            // the other edge (the target) already traded: the setup is gone
            if armed == 1 && h >= vah { armed = 0; }
            if armed == -1 && l <= val { armed = 0; }
            if armed == 1 { next.push(o(A::Buy, P::Lim(val), "Buy bottom of the area")); }
            if armed == -1 { next.push(o(A::Sell, P::Lim(vah), "Sell top of the area")); }
        }
        if st.pos == 1 { next.push(o(A::Xl, P::Stop(val * (1.0 - stopu)), "stop")); next.push(o(A::Xl, P::Lim(vah), "target: top of the area")); }
        if st.pos == -1 { next.push(o(A::Xs, P::Stop(vah * (1.0 + stopu)), "stop")); next.push(o(A::Xs, P::Lim(val), "target: bottom of the area")); }
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn value_area_expands_from_the_busiest_price() {
        // volume 1, 5, 3, 1 at 10.00..10.15: total 10, 15% tail = 1.5: cumulative 1, 6, 9, 10 -> low 10.05 (6 >= 1.5), high 10.10 (9 >= 8.5)
        let c = [10.0, 10.05, 10.1, 10.15];
        let (lo, hi) = value_area(&c, &[1.0, 5.0, 3.0, 1.0], 70.0).unwrap();
        assert_eq!((lo, hi), (10.05, 10.10));
        assert!(value_area(&[], &[], 70.0).is_none());
        assert_eq!(params(ID).unwrap().iter().map(|p| p.def).collect::<Vec<_>>(), vec![70., 2., 20.]);
    }
}
