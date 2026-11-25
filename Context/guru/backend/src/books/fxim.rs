//! Richard Stevenson, "The Forex Impulse Trading System" (theforexmagicmachine.com, 2009; research share Trading Knowledge I/Richard Stevenson - The Forex Impulse
//! Trading System/foreximpulse.pdf, 5 pp. of rules, text layer, read in full). Book id `fxim`. Daily Slow Stochastic (13,5,5) %K above %D allows longs (below: shorts);
//! on the 4-hour chart EMA4 crosses EMA50 first, then EMA13 crosses EMA50 = entry; stop 50 pips from the entry; exit when EMA4 reverses and crosses EMA13.
//! Lane K1 (24 Nov 2026, Q381 row 143). Independent check authors/conv-1123/K1/fxim_ref.py.
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};
use crate::intraday::Bar1;

pub const BOOK: &str = "fxim";
pub const INTRA: &[&str] = &["fxim_h4"];
pub fn reviewed(id: &str) -> bool { INTRA.contains(&id) }

const NOTE: &str = "[Declared adaptation, R4 #340/#380] The guide trades GBP/USD (4-hour and daily charts); this card runs it on 4-hour bars built from the real 1-minute bars of SPX / NIFTY 50 / NIFTYBANK (SPY via the standard minute route) with the daily bars of the same index. Card-specific unstated choices (4), each independent: (1) the 4-hour chart = 240-minute bars from the session open, the last bar of a session being the shorter remainder; (2) 'EMA4 first crosses above EMA50, followed by EMA13 crosses above EMA50' = EMA13 crosses above EMA50 on a closed bar while EMA4 is above EMA50 on that bar and the one before (mirrored for shorts); (3) the daily Slow Stochastic (13,5,5) is smoothed with simple averages: Slow %K = SMA5 of the 13-day raw %K, %D = SMA5 of Slow %K; (4) the daily %K / %D used are those of the previous completed day (no look-ahead). Honest total: 4 card-specific choices. (Audit count 5; its items 4 (EMAs one continuous series seeded with the first close, no signal before the 50th bar) and 5 (one position, a signal during a position ignored) are engine defaults per #404 and are moved out; the simple-average smoothing of the Slow Stochastic, not in the audit list, is counted as (3).) Engine-wide defaults / declared adaptations (labelled, not counted, R4 #404): the EMAs are one continuous series across sessions (not reset each day), seeded with the first close, with no signal before the 50th 4-hour bar; one position at a time, a signal during a position is ignored; 50 pips = 0.2710 percent of the entry price (the guide's long example: entry 1.8450, stop 1.8400; point-to-percent scaling); signals at a closed bar fill at the next bar's open (#373); the stop is tested on the 1-minute path with a gap filling at the minute's open (#175/#240); the exit fills at the next bar's open after EMA4 crosses back through EMA13; a session without cleaned 1-minute bars is skipped.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "fxim_h4", name: "Forex Impulse: daily Slow Stochastic (13,5,5) filter, 4-hour EMA4 then EMA13 cross of EMA50, stop 0.271 percent, exit EMA4 back across EMA13 (indices, 1-minute path)",
        group: "Stevenson Forex Impulse", session: 0, dir: "Long/Short", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: "fxim",
        rules: &[
            "E|Daily chart: Slow %K above Slow %D = longs only (below = shorts only). 4-hour chart: \"EMA4 first crosses above the EMA50; follow by EMA13 crosses above the EMA50\" (mirrored for shorts) (pp.2-3)",
            "X|\"Place your stop loss 50 pips from the entry point\" (pp.2-3)",
            "X|\"When EMA4 reverse and cross the EMA13 on the 4-Hour chart\" (pp.2-3)"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    reviewed(id).then_some(("fxim", "Forex Impulse (4-hour EMA cross)", "EMA 4/13/50, stochastic (13,5,5), stop 0.271 percent"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    reviewed(id).then(|| vec![p("stop", "Stop, percent of the entry (50 pips at the guide's 1.8450 example)", 0.271, 0.05, 10.0, 0.001)])
}

fn ema(c: &[f64], n: usize) -> Vec<f64> {
    let a = 2.0 / (n as f64 + 1.0);
    let mut v = Vec::with_capacity(c.len());
    for (i, &z) in c.iter().enumerate() { v.push(if i == 0 { z } else { a * z + (1.0 - a) * v[i - 1] }); }
    v
}

fn sma(x: &[f64], n: usize) -> Vec<f64> {
    (0..x.len()).map(|i| if i + 1 < n || x[i + 1 - n..=i].iter().any(|v| v.is_nan()) { f64::NAN } else { x[i + 1 - n..=i].iter().sum::<f64>() / n as f64 }).collect()
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let s = params(id)?;
    let stp = p.get("stop").copied().unwrap_or(s[0].def) / 100.0;
    // daily Slow Stochastic (13,5,5)
    let nd = to + 1;
    let raw: Vec<f64> = (0..nd).map(|i| if i < 12 { f64::NAN } else {
        let hh = x.h[i - 12..=i].iter().cloned().fold(f64::MIN, f64::max); let ll = x.l[i - 12..=i].iter().cloned().fold(f64::MAX, f64::min);
        if hh > ll { 100.0 * (x.c[i] - ll) / (hh - ll) } else { 50.0 } }).collect();
    let sk = sma(&raw, 5); let sd = sma(&sk, 5);
    // 4-hour bars
    let mut mins: Vec<Option<Vec<Bar1>>> = Vec::with_capacity(nd);
    let mut bars: Vec<(usize, usize, usize, f64)> = vec![]; // (day, first minute, last minute, close)
    for i in 0..nd {
        match crate::intraday_ep::day_bars(x, i, 240) {
            Some((_, _, m, b)) => { for z in &b { bars.push((i, z.a, z.b - 1, z.c)); } mins.push(Some(m)); }
            None => mins.push(None),
        }
    }
    let n = bars.len();
    let cl: Vec<f64> = bars.iter().map(|b| b.3).collect();
    let (e4, e13, e50) = (ema(&cl, 4), ema(&cl, 13), ema(&cl, 50));
    let open_of = |k: usize| mins[bars[k].0].as_ref().unwrap()[bars[k].1].o;
    let mut out: Vec<Trade> = vec![];
    let mut pos: Option<(f64, f64, f64, usize, usize)> = None; // dir, entry px, stop, entry day, signal day
    let mut wait: Option<(f64, usize)> = None; // entry signal (dir, signal bar) to fill at the next bar's open
    for k in 0..n {
        if let Some((dir, sg)) = wait.take() {
            let d = bars[k].0;
            if d >= from && d <= to { let px = open_of(k); pos = Some((dir, px, px * (1.0 - dir * stp), d, bars[sg].0)); }
        }
        let mut exit_next = false;
        if let Some((dir, px, st, ed, sg)) = pos {
            let (d, m) = (bars[k].0, mins[bars[k].0].as_ref().unwrap());
            for j in bars[k].1..=bars[k].2 {
                let b = &m[j];
                if (dir > 0.0 && b.l <= st) || (dir < 0.0 && b.h >= st) {
                    out.push(Trade { dir, legs: vec![Leg { i: ed, px, w: 1. }], exit_i: d, exit_px: if dir > 0.0 { b.o.min(st) } else { b.o.max(st) }, reason: "stop".into(), entry_kind: "open", signal_i: sg, open: false, exits: vec![] });
                    pos = None; break;
                }
            }
            if let Some((dir, ..)) = pos { if k >= 1 { exit_next = if dir > 0.0 { e4[k] < e13[k] && e4[k - 1] >= e13[k - 1] } else { e4[k] > e13[k] && e4[k - 1] <= e13[k - 1] }; } }
        }
        if exit_next {
            let (dir, px, _, ed, sg) = pos.take().unwrap();
            if k + 1 < n { out.push(Trade { dir, legs: vec![Leg { i: ed, px, w: 1. }], exit_i: bars[k + 1].0, exit_px: open_of(k + 1), reason: "EMA4 crossed EMA13".into(), entry_kind: "open", signal_i: sg, open: false, exits: vec![] }); }
            else { pos = Some((dir, px, 0.0, ed, sg)); }
            continue;
        }
        if pos.is_none() && k >= 50 && k + 1 < n {
            let d = bars[k].0;
            if d == 0 || d > to { continue; }
            let (kk, dd) = (sk[d - 1], sd[d - 1]);
            if kk.is_nan() || dd.is_nan() { continue; }
            let up = e13[k] > e50[k] && e13[k - 1] <= e50[k - 1] && e4[k] > e50[k] && e4[k - 1] > e50[k - 1];
            let dn = e13[k] < e50[k] && e13[k - 1] >= e50[k - 1] && e4[k] < e50[k] && e4[k - 1] < e50[k - 1];
            if up && kk > dd { wait = Some((1.0, k)); } else if dn && kk < dd { wait = Some((-1.0, k)); }
        }
    }
    if let Some((dir, px, _, ed, sg)) = pos {
        out.push(Trade { dir, legs: vec![Leg { i: ed, px, w: 1. }], exit_i: to, exit_px: x.c[to], reason: "open".into(), entry_kind: "open", signal_i: sg, open: true, exits: vec![] });
    }
    Some(out)
}
