//! Dave Wooding, "The Stock Rocket Stock Trading System" (The Wooding Trading Company, 2001; book id `dwsr`; PDF pages).
//! Reading: claude-continuation-20261020/authors/wooding (admission/sr_ref.py). Live: the whole system of Ch.2 (PDF 9-26)
//! with the appendix scan (PDF 56): high-volume 52-week-high signal day, buy stop one tick over the signal high (next day,
//! after an inside day, or after a three-day pullback), stop one tick under the signal low, half sold at one times the
//! risk, the rest trailed under confirmed higher lows. Not coded: the case-study variations (selling a third, a stop under
//! a later higher low, re-entries after new highs, PDF 31-50), float and news (stated as "not requirements", PDF 16) and
//! the 3% position size (equal weight here). Book: US stocks 1999-2000; $10 and 500,000-share limits make it US only.
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};
/// Admitted ids (authors/wooding/admission/sr_ref.py (header)).
pub const REVIEWED: &[&str] = &["dwsr_stock_rocket"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const TICK: f64 = 0.01;
const NOTE: &str = "[Declared readings] Signal day exactly as the appendix scan (PDF 56): volume above 10 times the average of the previous 50 days and above 500,000 shares, close above $10, high above the highest high of the previous 250 days, range larger than each of the previous 9 days, close above the open, close in the top third and open in the bottom third of the range. A day with no volume in the 50-day window cannot signal. A tick is $0.01. Day 2 (the day after the signal): if it opens at or below the signal high, buy stop one tick above it. Day 3: only if day 2 was an inside day (high below and low above the signal day's), the same order. Pullback: if days 2, 3 and 4 each make a lower high than the day before, from day 5 a buy stop one tick above the previous day's high on every day that opens at or below that high, until filled. A low below the signal low cancels the setup that day (no fill that day: the order inside the day is unknown). A newer signal day replaces a waiting setup; signals during a trade are ignored. From the day after the fill: sell stop one tick under the signal low; half the position is sold at entry + (entry - stop), at that level or the open if higher; a higher low = a low with two higher lows before and after it, formed on or after the entry day, moves the stop to one tick under it two days later (stop only rises). The rest is sold when the low reaches the stop, at the stop or the open if lower. If the stop and the half target are both reached on one day, the stop is taken for the whole position. Equal weight, not the book's 3% risk sizing.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "dwsr_stock_rocket", name: "Stock Rocket: 10x-volume 52-week-high day, buy stop over its high, half at 1R, trail under higher lows",
        group: "Wooding Stock Rocket", session: 0, dir: "Long", status: Status::Live, needs_volume: true, needs_vix: false, needs_series: &[],
        book: "dwsr", rules: &[
            "F|\"Today's volume is greater than ten times the average volume of the previous 50 days ... greater than 500,000 shares ... Today's high is a new fifty two week high ... open in the bottom third of the signal day's range and close in the top third ... largest range of the past two weeks ... close is greater than $10\" (PDF 9, 13; scan formula PDF 56)",
            "E|\"If the next day's open is less than or equal to the signal day's high, then place a buy stop limit order one tick above the signal day's high\" (PDF 19); inside day: the same order the second day (PDF 20); pullback: \"Day 2, 3, and 4 all make lower highs ... starting on day 5, place a buy stop above the most recent high ... until filled OR the stock trades below the signal day's low\" (PDF 21)",
            "X|\"place a protective sell stop ... one tick below the signal day's low\" (PDF 23); \"Sell half of your position when your open profits equal your initial risk\" (PDF 24)",
            "X|\"adjust your protective sell stop to one tick below the most recent higher low ... a low preceded and followed by at least two higher lows ... two days after the higher low is made\" (PDF 25); \"Sell remaining half ... as the stock trades below the most recent higher low\" (PDF 26)"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    (id == "dwsr_stock_rocket").then_some(("dwsr_stock_rocket", "Wooding Stock Rocket", "book rules"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> { (id == "dwsr_stock_rocket").then(Vec::new) }

/// Appendix scan (PDF 56) on bar t.
fn signal(x: &Ctx, t: usize) -> bool {
    if t < 250 { return false; }
    let (o, h, l, c) = (x.o[t], x.h[t], x.l[t], x.c[t]);
    let vol = |i: usize| x.r[i].volume.filter(|v| *v > 0.);
    let Some(v) = vol(t) else { return false };
    let mut sum = 0.;
    for i in t - 50..t { match vol(i) { Some(w) => sum += w, None => return false } }
    let rg = |i: usize| x.h[i] - x.l[i];
    let r = h - l;
    v > 10. * sum / 50. && v > 500_000. && c > 10. && h > x.h[t - 250..t].iter().cloned().fold(f64::MIN, f64::max)
        && (1..=9).all(|k| r > rg(t - k)) && c > o && c > 2. / 3. * r + l && o < r / 3. + l
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, _p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    struct Pos { ei: usize, ep: f64, stop: f64, target: f64, half: Option<(usize, f64)> }
    let close = |p: &Pos, xi: usize, xp: f64, why: &str, open: bool| {
        let (exit_px, exits, reason) = match p.half {
            Some((hi, hp)) => (0.5 * hp + 0.5 * xp, vec![(hi, hp, 0.5), (xi, xp, 0.5)], format!("half at 1R; rest: {why}")),
            None => (xp, vec![], why.to_string()),
        };
        Trade { dir: 1., legs: vec![Leg { i: p.ei, px: p.ep, w: 1. }], exit_i: xi, exit_px, reason, entry_kind: "stop", signal_i: xi, open, exits }
    };
    let (mut setup, mut pos, mut out) = (None::<usize>, None::<Pos>, vec![]);
    for t in 0..=to {
        if let Some(mut p) = pos.take() {
            if l[t] <= p.stop {
                out.push(close(&p, t, o[t].min(p.stop), "stop", false));
                continue; // nothing else on the exit bar
            }
            if p.half.is_none() && h[t] >= p.target { p.half = Some((t, o[t].max(p.target))); }
            // a higher low at t-2, confirmed today, moves tomorrow's stop
            if t >= 4 && t - 2 >= p.ei {
                let k = t - 2;
                if [k - 2, k - 1, k + 1, k + 2].iter().all(|&j| l[j] > l[k]) { p.stop = p.stop.max(l[k] - TICK); }
            }
            pos = Some(p);
            continue; // signals during a trade are ignored
        }
        if let Some(s) = setup {
            if l[t] < l[s] { setup = None; }
            else if t >= from && t < to {
                let n = t - s; // day number = n + 1
                let lvl = if n == 1 || (n == 2 && h[s + 1] < h[s] && l[s + 1] > l[s]) {
                    (o[t] <= h[s]).then_some(h[s] + TICK)
                } else if n >= 4 && (1..=3).all(|k| h[s + k] < h[s + k - 1]) {
                    (o[t] <= h[t - 1]).then_some(h[t - 1] + TICK)
                } else { None };
                if let Some(lv) = lvl.filter(|lv| h[t] >= *lv) {
                    let ep = o[t].max(lv);
                    let stop = l[s] - TICK;
                    pos = Some(Pos { ei: t, ep, stop, target: ep + (ep - stop), half: None });
                    setup = None;
                    continue;
                }
            }
        }
        if signal(x, t) { setup = Some(t); }
    }
    if let Some(p) = pos { out.push(close(&p, to, c[to], "open", true)); }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_reviewed_ids_run() {
        assert!(reviewed("dwsr_stock_rocket") && !reviewed("dwsr_x"));
        assert_eq!(catalogue().len(), 1);
        assert!(catalogue()[0].needs_volume);
    }
}
