//! Larry Levin, "Secrets of Floor Traders" course (1999 / 2001 editions), Rules.pdf chapter 2 "The Value Area & The 80% Rule" pp.11-31 (scanned; RapidOCR text saved in
//! authors/conv-1123/K1/levin/rules_ocr.txt, all 14 pages read). Book id `levva`. Lane K1 (24 Nov 2026, R4 #397 tier 5): the 80% rule on the NIFTY 50 / NIFTYBANK futures
//! 1-minute series with volume (lane DI) as 30-minute brackets, a labelled R4 #340 adaptation; one scaling rule R4 #380. Helper books::k4x. Reference authors/code-1123/ref/levva_ref.py.
use super::k4x::{fut_bars, o, run_fb, A, P};
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use std::collections::HashMap;

const ID: &str = "levva_value_area_80";
const NOTE: &str = "[Declared adaptation, R4 #340 / #380] Larry Levin's Value Area 80% Rule (Secrets of Floor Traders, Rules.pdf pp.11-31, e-mini S&P): the Value Area is the price range holding 70% of yesterday's volume; when the market is outside it (opened outside, or left it during the day) and then is inside it for two consecutive 30-minute brackets it has an 80% chance of filling the whole Value Area: go long as near the bottom of the area as possible (short near the top), with a sell stop a few ticks below the area (printed example: area 68500-68720, stop 68480, 20 index units) and cover at the other side of the area; reversal only with other indicators (not coded). Run on the NIFTY 50 / NIFTYBANK futures 1-minute bars with volume. Labelled readings (4): (1) the Value Area is built from the previous session's 1-minute volume placed at each minute's close price on a 0.05 grid, by the Market Profile rule the book cites (Dalton, Mind Over Markets, not printed in the course): start at the highest-volume price (ties: the lower), repeatedly add the two grid prices above or the two below, whichever pair holds more volume (ties: above), until 70% of the volume is inside; (2) 'in the Value Area for two consecutive brackets' = two consecutive 30-minute bars (from the session open) that close inside the area, after a bar of the same session reached beyond it (the market need not open outside, p.27); the earlier excursion's side (the latest bar closing beyond, else the side its extreme reached) decides long (below) or short (above); the signal is read at the close of the second bar (the book reads it when the next bar opens, not testable on bar data without look-ahead); (3) the order is a limit at the area edge (bottom for a long, top for a short), re-issued each bar while no trade is open, dropped for the day once the other edge has traded; the stop is 20/68,500 of the edge price beyond the area (the book's 20 units on its 68,500 level, ONE scaling rule #380), the target is the other edge; one trade per session; (4) the position is closed at the session's last bar. Previous session = the previous futures day in the file; no signal on a day without one. No profit figure is printed: 'about 8 out of 10'.";

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

/// Value Area (low, high) of one session: 1-minute volume at each close on a 0.05 grid, Market Profile expansion from the highest-volume price.
pub fn value_area(close: &[f64], vol: &[f64], pct: f64) -> Option<(f64, f64)> {
    let key: Vec<i64> = close.iter().map(|c| (c * 20.0).round() as i64).collect();
    let (lo0, hi0) = (*key.iter().min()?, *key.iter().max()?);
    let mut v = vec![0.0f64; (hi0 - lo0 + 1) as usize];
    for (k, x) in key.iter().zip(vol) { v[(k - lo0) as usize] += x; }
    let total: f64 = v.iter().sum();
    if total <= 0.0 { return None; }
    let poc = v.iter().enumerate().fold((0usize, -1.0f64), |b, (i, &x)| if x > b.1 { (i, x) } else { b }).0;
    let (mut lo, mut hi, mut acc) = (poc, poc, v[poc]);
    let at = |i: isize| if i >= 0 && (i as usize) < v.len() { v[i as usize] } else { 0.0 };
    while acc < total * pct / 100.0 && (lo > 0 || hi + 1 < v.len()) {
        let up = at(hi as isize + 1) + at(hi as isize + 2);
        let dn = at(lo as isize - 1) + at(lo as isize - 2);
        if (up >= dn && hi + 1 < v.len()) || lo == 0 {
            for _ in 0..2 { if hi + 1 < v.len() { hi += 1; acc += v[hi]; } }
        } else {
            for _ in 0..2 { if lo > 0 { lo -= 1; acc += v[lo]; } }
        }
    }
    Some(((lo0 + lo as i64) as f64 / 20.0, (lo0 + hi as i64) as f64 / 20.0))
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
        let entered_today = st.last_entry.map_or(false, |e| fb.date[e] == fb.date[k]);
        if st.pos == 0 && !entered_today {
            if l < val || h > vah {
                last_out = if c < val { -1 } else if c > vah { 1 } else if l < val && h <= vah { -1 } else if h > vah && l >= val { 1 } else if c < (val + vah) / 2.0 { -1 } else { 1 };
            }
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
        // volume 1 at 10.00, 5 at 10.05, 3 at 10.10, 1 at 10.15: total 10; 70% = 7 -> POC 10.05 (5), the pair above (3+1) beats the pair below (1+0): 9 >= 7
        let c = [10.0, 10.05, 10.1, 10.15];
        let (lo, hi) = value_area(&c, &[1.0, 5.0, 3.0, 1.0], 70.0).unwrap();
        assert_eq!((lo, hi), (10.05, 10.15));
        assert!(value_area(&[], &[], 70.0).is_none());
        assert_eq!(params(ID).unwrap().iter().map(|p| p.def).collect::<Vec<_>>(), vec![70., 2., 20.]);
    }
}
