//! J. D. Hamon, *Spike 35 Trader's Manual* (2nd ed., 21 scanned pages). Source + audit: library/jhs35/.
//! One mechanical daily system: change of trend by HIP/LOP (rule 1) or the Thirteen Day Rule (rule 2),
//! entry on the close, exit when price touches the lower (higher) of the two previous closes (MIT),
//! re-entry in the trend direction when the four previous closes (tight congestion: four highs/lows) are passed.
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "jhs35";

const NOTE: &str = "Book market: commodity futures (daily bars; p.18-19 'trade markets that are moving'); the publisher's note (p.1) gives stock-index variances: 'S&P 500 40 pts, NYSE Composite 40 pts, Value Line 60 pts'. \
'pts' read with the book's own Table I convention (p.3: 1 pt = the quote's hundredth, e.g. Soybean Oil/Silver/Gold 1 pt = $.01): 40 pts = 0.40 index points (param variance; an absolute price amount, so on today's index levels it is a very small filter). \
Runs on the indices (instrument swap; NIFTY 50 uses the same 0.40-point variance). Fills: change-of-trend entries and opposite-signal reversals at the close of the signal day (p.13 'enter the market at the close'); \
MIT exit / stop re-entry fill at the level, or at the open if the bar gaps through it. Book silent, neutral choices: the exit stop is first active the bar after an entry, and a re-entry is first allowed the bar after an exit \
(daily bars cannot order the two touches inside one day); the first signal of either rule sets the initial trend; on the (rare) day both an up and a down change of trend would fire, nothing changes. \
HIPs closer than 'more than one day' apart (p.7, figs 11-13): the higher high survives (the earlier one on a tie). No published trade results; p.1 says the system 'usually only keeps one in the market about four days per trade'.";

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, group, dir, status, rules, notes| StrategyDef {
        id, name, group, session: 0, dir, status, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim: None,
    };
    vec![
        d("jhs35_spike35", "Spike-35 System (HIP/LOP + Thirteen Day Rule, two-close exit, four-close re-entry)", "Spike-35 · Sections II-VIII", "Long/Short", Status::Live, &[
            "F|HIP: \"A day with a lower high the day before it and the day after it\" and \"The high of the high day must be at least this amount [variance] above the highs on each side of it\"; \"the pattern need not occur in 3 days. As long as the variance amounts, front and back, are met before a new high occurs, the pattern is complete\" (p.4-5). LOP identical with lows (p.6-7).",
            "F|\"There must be more than one day between two HIP's (or two LOP's)\" - a HIP formed with a higher high one day later disqualifies the earlier one (p.7, figs 11-13).",
            "E|Change of Trend Rule 1: \"If the market reverses and passes the two previous HIP's or LOP's (on the close), this is a change of trend\"; a close exactly equal is not (p.7-9).",
            "E|Change of Trend Rule 2 (TDR): \"If the market reverses and closes beyond the previous thirteen closes (on the close), this is a change of trend\"; today is Day 0, equal close is not (p.10-12).",
            "E|Entry Signals 1 and 2: \"enter the market at the close on the day the trend changes\" (p.13-14). Optional: \"wait until the next day, and get in at the open\" (p.13; param next_open=1).",
            "X|\"Set your stop at the lower (higher if short) of the two previous closes. If this level is penetrated, close out your position. Use a market-if-touched (MIT) order. So even if price goes exactly to that point but not further, you should still close your position.\" (p.15)",
            "A|Basic Re-Entry: \"If the market penetrates the four previous closes, without a change of trend, you re-enter the market in the direction of the trend\" at \"the price level of the highest of the four previous closes\" (p.16-17); \"you must use a 'stop order'\" (p.1).",
            "A|Tight Congestion Re-Entry: \"four consecutive closes all within the variance amount of each other ... re-enter long only on penetration of the highest high of the previous 4 days\" (short: lowest low) (p.17).",
            "X|An opposite change of trend closes the position and reverses at that close (p.13-15: you 'only use the closer of the entry points in each direction').",
        ], NOTE),
        d("jhs35_trendline_exit", "Optional trendline exit", "Spike-35 · XI Optional exit", "Long/Short", Status::NotTestable(
            "discretionary: 'this rule is a bit subjective' - 'use the best of the last several HIP's or LOP's with which to draw a trend line' (p.20)"), &[
            "X|\"exit when the trend line is passed\" (p.20)"], "Book itself calls it subjective and untested ('I didn't analyze it over the long term')."),
        d("jhs35_reentry_tolerance", "Optional MIT re-entry tolerance", "Spike-35 · XII Optional re-entry", "Long/Short", Status::NotTestable(
            "no value for stock indices: the book gives only Live Hogs 15 points and Pork Bellies 30 points (p.20)"), &[
            "A|\"a tolerance level for these Market-If-Touched (MIT) orders ... fifteen points for trading Live Hogs\" (p.20)"],
            "Hamon: 'I didn't use this in my system'. Not coded: inventing an index tolerance would be a new rule."),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match id {
        "jhs35_spike35" => Some(("jhs35_spike", "Spike-35 System", "Book rules")),
        "jhs35_trendline_exit" => Some(("jhs35_spike", "Spike-35 System", "Optional trendline exit")),
        "jhs35_reentry_tolerance" => Some(("jhs35_spike", "Spike-35 System", "Optional re-entry tolerance")),
        _ => None,
    }
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    match id {
        "jhs35_spike35" => Some(vec![
            p("variance", "HIP/LOP variance in index points (S&P 500 '40 pts' = 0.40, p.1)", 0.40, 0.0, 50.0, 0.05),
            p("tdr", "Thirteen Day Rule: previous closes", 13.0, 5.0, 30.0, 1.0),
            p("exit_closes", "Exit: previous closes for the MIT stop", 2.0, 1.0, 5.0, 1.0),
            p("reentry_closes", "Re-entry: previous closes (and highs/lows in tight congestion)", 4.0, 2.0, 8.0, 1.0),
            p("tight", "Tight congestion re-entry rule on (1) / off (0)", 1.0, 0.0, 1.0, 1.0),
            p("next_open", "Enter change-of-trend at next open instead of the close (book option p.13)", 0.0, 0.0, 1.0, 1.0),
        ]),
        _ => None,
    }
}

/// Raw swing points. `hi`: HIPs on highs (else LOPs on lows). Returns (bar, price, confirm bar) sorted by confirm bar.
/// A point is confirmed on the first later bar that is `var` beyond it before any bar exceeds it; it also needs a bar
/// `var` beyond it on the front side before any bar exceeding it (p.4-7). The 'more than one day between' rule is
/// applied in real time by `accept`.
pub fn swing_points(v: &[f64], var: f64, hi: bool) -> Vec<(usize, f64, usize)> {
    let n = v.len();
    let beyond = |a: f64, b: f64| if hi { a > b } else { a < b };
    let far = |a: f64, p: f64| if hi { a <= p - var } else { a >= p + var };
    let mut raw: Vec<(usize, f64, usize)> = vec![];
    for i in 0..n {
        let p = v[i];
        let mut front = false;
        for j in (0..i).rev() {
            if beyond(v[j], p) { break; }
            if far(v[j], p) { front = true; break; }
        }
        if !front { continue; }
        for k in i + 1..n {
            if beyond(v[k], p) { break; }
            if far(v[k], p) { raw.push((i, p, k)); break; }
        }
    }
    raw.sort_by_key(|&(i, _, k)| (k, i));
    raw
}

/// Add a newly confirmed point: if an accepted point lies within 2 bars ('more than one day between' fails),
/// the more extreme one survives (the earlier one on a tie).
pub fn accept(acc: &mut Vec<(usize, f64, usize)>, pt: (usize, f64, usize), hi: bool) {
    if let Some(pos) = acc.iter().rposition(|q| q.0 + 2 >= pt.0 && pt.0 + 2 >= q.0) {
        let more = if hi { pt.1 > acc[pos].1 } else { pt.1 < acc[pos].1 };
        if more { acc.remove(pos); } else { return; }
    }
    acc.push(pt);
}

fn tr(dir: f64, i: usize, px: f64, kind: &'static str) -> Trade {
    Trade { dir, legs: vec![Leg { i, px, w: 1.0 }], exit_i: i, exit_px: px, reason: String::new(), entry_kind: kind, signal_i: i, open: false, exits: vec![] }
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if id != "jhs35_spike35" { return None; }
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).unwrap().def);
    let n = x.c.len();
    if n == 0 || to >= n || from > to { return Some(vec![]); }
    let var = g("variance").max(0.0);
    let tdr = (g("tdr") as usize).max(1);
    let ex = (g("exit_closes") as usize).max(1);
    let re = (g("reentry_closes") as usize).max(1);
    let tight = g("tight") >= 0.5;
    let next_open = g("next_open") >= 0.5;
    // swing points use bars up to `to` only (confirmation never looks past the bar it is used on)
    let hips = swing_points(&x.h[..=to], var, true);
    let lops = swing_points(&x.l[..=to], var, false);
    let (mut hk, mut lk) = (0usize, 0usize);
    let mut hl: Vec<f64> = vec![]; // accepted HIP prices in confirm order (with removals applied)
    let mut ll: Vec<f64> = vec![];
    let mut hacc: Vec<(usize, f64, usize)> = vec![];
    let mut lacc: Vec<(usize, f64, usize)> = vec![];
    let mut trend = 0i8;
    let mut cur: Option<Trade> = None;
    let mut active_from = 0usize; // first bar the exit stop may fire
    let mut last_exit: Option<usize> = None;
    let mut pending: Option<(i8, usize)> = None; // next-open entry (dir, signal bar)
    let mut out = vec![];
    for t in 0..=to {
        // 0. pending next-open entry (book p.19: no entry if the open is already beyond the exit level)
        if let Some((d, s)) = pending.take() {
            if t >= ex && cur.is_none() {
                let lvl = if d > 0 { (1..=ex).map(|k| x.c[t - k]).fold(f64::INFINITY, f64::min) } else { (1..=ex).map(|k| x.c[t - k]).fold(f64::NEG_INFINITY, f64::max) };
                let ok = if d > 0 { x.o[t] > lvl } else { x.o[t] < lvl };
                if ok { let mut nt = tr(d as f64, t, x.o[t], "open"); nt.signal_i = s; cur = Some(nt); active_from = t; }
            }
        }
        // 1. MIT exit at the lower (higher) of the previous `ex` closes
        if let Some(c) = cur.as_mut() {
            if t >= active_from && t >= ex {
                if c.dir > 0.0 {
                    let lvl = (1..=ex).map(|k| x.c[t - k]).fold(f64::INFINITY, f64::min);
                    if x.l[t] <= lvl { c.exit_i = t; c.exit_px = x.o[t].min(lvl); c.signal_i = t; c.reason = format!("MIT: low touched the lower of the previous {ex} closes"); }
                } else {
                    let lvl = (1..=ex).map(|k| x.c[t - k]).fold(f64::NEG_INFINITY, f64::max);
                    if x.h[t] >= lvl { c.exit_i = t; c.exit_px = x.o[t].max(lvl); c.signal_i = t; c.reason = format!("MIT: high touched the higher of the previous {ex} closes"); }
                }
                if !c.reason.is_empty() { out.push(cur.take().unwrap()); last_exit = Some(t); }
            }
        } else if trend != 0 && t >= from && t >= re && last_exit.map_or(true, |e| t > e) && pending.is_none() {
            // 2. re-entry stop in the trend direction (only after a change-of-trend trade has been exited)
            let cl = &x.c[t - re..t];
            let (cmax, cmin) = (cl.iter().copied().fold(f64::NEG_INFINITY, f64::max), cl.iter().copied().fold(f64::INFINITY, f64::min));
            let cong = tight && cmax - cmin <= var;
            if trend > 0 {
                let lvl = if cong { x.h[t - re..t].iter().copied().fold(f64::NEG_INFINITY, f64::max) } else { cmax };
                if x.h[t] >= lvl {
                    let mut nt = tr(1.0, t, x.o[t].max(lvl), "stop"); nt.signal_i = t; cur = Some(nt); active_from = t + 1;
                }
            } else {
                let lvl = if cong { x.l[t - re..t].iter().copied().fold(f64::INFINITY, f64::min) } else { cmin };
                if x.l[t] <= lvl {
                    let mut nt = tr(-1.0, t, x.o[t].min(lvl), "stop"); nt.signal_i = t; cur = Some(nt); active_from = t + 1;
                }
            }
        }
        // 3. close: change of trend
        while hk < hips.len() && hips[hk].2 <= t { accept(&mut hacc, hips[hk], true); hk += 1; }
        while lk < lops.len() && lops[lk].2 <= t { accept(&mut lacc, lops[lk], false); lk += 1; }
        hl.clear(); hl.extend(hacc.iter().rev().take(2).map(|q| q.1));
        ll.clear(); ll.extend(lacc.iter().rev().take(2).map(|q| q.1));
        let c = x.c[t];
        let up1 = hl.len() == 2 && c > hl[0].max(hl[1]);
        let dn1 = ll.len() == 2 && c < ll[0].min(ll[1]);
        let (up2, dn2) = if t >= tdr {
            let w = &x.c[t - tdr..t];
            (c > w.iter().copied().fold(f64::NEG_INFINITY, f64::max), c < w.iter().copied().fold(f64::INFINITY, f64::min))
        } else { (false, false) };
        let up = trend != 1 && (up1 || up2);
        let dn = trend != -1 && (dn1 || dn2);
        if up == dn { continue; }
        let d: i8 = if up { 1 } else { -1 };
        trend = d;
        let why = match (d, up1 || dn1, up2 || dn2) {
            (_, true, true) => "HIP/LOP and Thirteen Day Rule",
            (_, true, false) => "HIP/LOP rule",
            _ => "Thirteen Day Rule",
        };
        if let Some(mut old) = cur.take() {
            if old.dir as i8 != d {
                old.exit_i = t; old.exit_px = c; old.signal_i = t; old.reason = format!("Opposite change of trend ({why}) at the close"); out.push(old);
                last_exit = Some(t);
            } else { cur = Some(old); }
        }
        if cur.is_none() && t >= from {
            if next_open { if t < to { pending = Some((d, t)); } }
            else { let mut nt = tr(d as f64, t, c, "close"); nt.signal_i = t; cur = Some(nt); active_from = t + 1; }
        }
    }
    if let Some(mut c) = cur { c.exit_i = to; c.exit_px = x.c[to]; c.signal_i = to; c.reason = "open".into(); c.open = true; out.push(c); }
    out.retain(|t| t.legs[0].i >= from);
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn hip_needs_variance_on_both_sides_and_one_day_gap_keeps_the_higher() {
        // highs: 10, 12, 10, 12.5, 10  with var 1 -> raw HIPs at 1 and 3 (two bars apart) -> only bar 3 (higher) survives
        let h = [10.0, 12.0, 10.0, 12.5, 10.0];
        let mut acc = vec![];
        for pt in swing_points(&h, 1.0, true) { accept(&mut acc, pt, true); }
        assert_eq!(acc.iter().map(|q| q.0).collect::<Vec<_>>(), vec![3]);
        // back variance not met before a new high: 10, 12, 11.5, 13, 10 -> bar 1 is not a HIP, bar 3 is
        let h = [10.0, 12.0, 11.5, 13.0, 10.0];
        let s = swing_points(&h, 1.0, true);
        assert_eq!(s.iter().map(|q| (q.0, q.2)).collect::<Vec<_>>(), vec![(3, 4)]);
        // exactly the variance counts (fig 6)
        let h = [11.0, 12.0, 11.0];
        assert_eq!(swing_points(&h, 1.0, true).len(), 1);
    }
    #[test]
    fn thirteen_day_rule_enters_on_the_close_and_two_close_mit_exits() {
        let mut r: Vec<Row> = (0..40).map(|i| Row { ticker: "T".into(), date: format!("2000-01-{:02}", 1 + i % 28), open: 100., high: 100.5, low: 99.5, close: 100. - (i as f64) * 0.01, volume: None }).collect();
        r[20].close = 101.0; r[20].high = 101.2; // above the previous 13 closes -> long at 101
        r[21].close = 101.5; r[21].high = 101.8; r[21].low = 101.1;
        r[22].low = 100.9; r[22].open = 101.4; // lower of closes 21/20 = 101.0 -> touched -> exit at 101.0
        let x = Ctx::new(&r, &r, &[]);
        let ts = run("jhs35_spike35", &x, 0, 39, &Params::new()).unwrap();
        let t = ts.iter().find(|t| t.legs[0].i == 20).expect("long on bar 20");
        assert_eq!((t.dir, t.legs[0].px, t.exit_i, t.exit_px), (1.0, 101.0, 22, 101.0));
    }
}
