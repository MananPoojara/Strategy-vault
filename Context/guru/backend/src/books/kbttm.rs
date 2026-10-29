//! Kedrick F. Brown, "Trend Trading: Timing Market Tides" (Wiley, 2006; book id `kbttm`; printed pages, PDF = printed
//! + 19). Reading: claude-continuation-20261020/authors/kbrown (agents/trendtrading/methods.json, admission/tt_ref.py).
//! Live: the 32-day close-only Donchian long/flat rule the book tests in Ch.7 (p.187-193) and the all-time-high breakout
//! with a 96% fixed stop and a 32-day-low exit (p.167-168). Held: Bollinger and generic stochastic rules (N not given),
//! the NASDAQ Composite pilots and filter (series not held), the stochastic-average basket (book's coal stocks gone),
//! the S&P rotation (two-instrument switch), the 8-day peak breakout and the INTC/KLAC pair (see authors/kbrown notes).
//! Book: NASDAQ Composite, XOI and UTY indices and US stocks; indices, ETFs and US/India stocks here are a declared
//! application.
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};
/// Admitted ids (authors/kbrown/admission/tt_ref.py (header)).
pub const REVIEWED: &[&str] = &["kbttm_donchian32", "kbttm_ath_breakout"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE_D: &str = "[Declared readings] Highest / lowest close of the previous N days, today excluded (p.80). A close above the upper band buys, a close below the lower band sells; signals while already long (or flat) are ignored (p.80). Fill on the next day's close (p.187, the tested version); the next day's open is the book's alternative (p.81) and an option. Long or flat; the bands are read from all chart history and the first trade needs a buy inside the test window. The book's test is on index closes without costs or dividends.";
const NOTE_A: &str = "[Declared readings] All-time high = the highest high of every earlier bar the vault holds (the data start limits 'all-time'). Buy on a stop at that level when today's high exceeds it, filled at the level or at the open if the day opens above it. From the next day: a fixed stop at 96% of the fill and a stop at the lowest low of the previous 32 days (today excluded); the position is sold at whichever level is broken first (the higher one), at the level or the open if gapped below. Re-entry on the next new all-time high (not on the bar of an exit: the order inside the day is unknown). The watch-list ranking (p.167) is personal and not coded; each instrument runs on its own. Position sizing by target heat is not modelled.";

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id: &'static str, name: &'static str, rules: &'static [&'static str], notes: &'static str| StrategyDef { id, name, group: "Brown trend trades", session: 0, dir: "Long", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "kbttm", rules, notes, claim: None };
    vec![
        d("kbttm_donchian32", "32-day close Donchian: buy a close above the highest close, sell a close below the lowest", &[
            "E|\"IF the NASDAQ Composite's close today exceeds the highest close of the past 32 days, THEN invest 100 percent of your capital in the NASDAQ Composite on tomorrow's close\" (p.187)",
            "X|\"IF the NASDAQ Composite's close today is below the lowest close of the past 32 days, THEN liquidate ... on tomorrow's close\" (p.187); bands exclude the current day (p.80)",
            "F|Close-only Donchian bands, \"tomorrow's close (or alternatively tomorrow's open)\" (p.81); N = 32 in the book's test (p.187-193)"], NOTE_D),
        d("kbttm_ath_breakout", "All-time-high breakout with a 96% stop and a 32-day-low exit", &[
            "E|\"Buy the number one stock on your watch list as soon as it makes a new all-time high\" (p.168)",
            "X|\"Set a fixed stop level at 96 percent of your entry price ... Liquidate the position if price breaks down through your fixed stop level. Otherwise, liquidate the position if price makes a new 32-day low\" (p.168)",
            "F|\"Reenter the stock if it makes a subsequent new all-time high\" (p.168)"], NOTE_A),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match id {
        "kbttm_donchian32" => Some(("kbttm_donchian32", "Brown close Donchian", "32 days, next close")),
        "kbttm_ath_breakout" => Some(("kbttm_ath_breakout", "Brown all-time-high breakout", "96% stop, 32-day low")),
        _ => None,
    }
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    match id {
        "kbttm_donchian32" => Some(vec![p("n", "Band days (book test 32)", 32., 2., 250., 1.),
            p("fill", "Fill: 0 = next day's close (book test), 1 = next day's open (book's alternative, p.81)", 0., 0., 1., 1.)]),
        "kbttm_ath_breakout" => Some(vec![p("stop", "Fixed stop, % of entry (book 96)", 96., 50., 99.9, 0.5), p("low", "Exit on a new N-day low (book 32)", 32., 2., 250., 1.)]),
        _ => None,
    }
}

fn mk(ei: usize, ep: f64, kind: &'static str, xi: usize, xp: f64, why: &str, si: usize, open: bool) -> Trade {
    Trade { dir: 1., legs: vec![Leg { i: ei, px: ep, w: 1. }], exit_i: xi, exit_px: xp, reason: why.into(), entry_kind: kind, signal_i: si, open, exits: vec![] }
}

fn donchian(x: &Ctx, from: usize, to: usize, n: usize, next_open: bool) -> Vec<Trade> {
    let c = &x.c;
    let (mut out, mut cur) = (vec![], None::<(usize, f64)>);
    for t in from.max(n)..to {
        let w = &c[t - n..t];
        let (hi, lo) = (w.iter().cloned().fold(f64::MIN, f64::max), w.iter().cloned().fold(f64::MAX, f64::min));
        let px = if next_open { x.o[t + 1] } else { c[t + 1] };
        match cur {
            None if c[t] > hi => cur = Some((t + 1, px)),
            Some((ei, ep)) if c[t] < lo => { out.push(mk(ei, ep, if next_open { "open" } else { "close" }, t + 1, px, "close below the lowest close", t, false)); cur = None; }
            _ => {}
        }
    }
    if let Some((ei, ep)) = cur { out.push(mk(ei, ep, if next_open { "open" } else { "close" }, to, c[to], "open", to, true)); }
    out
}

fn ath(x: &Ctx, from: usize, to: usize, keep: f64, n: usize) -> Vec<Trade> {
    let (o, h, l) = (&x.o, &x.h, &x.l);
    let mut top = f64::MIN; // highest high before bar t
    let (mut out, mut cur) = (vec![], None::<(usize, f64)>);
    for t in 0..=to {
        if t >= from && t > 0 {
            let mut sold = false;
            if let Some((ei, ep)) = cur {
                if t > ei && t >= n {
                    let chan = l[t - n..t].iter().cloned().fold(f64::MAX, f64::min);
                    let fixed = ep * keep;
                    let lvl = fixed.max(chan);
                    if l[t] < lvl {
                        let why = if fixed >= chan { "fixed stop" } else { "new N-day low" };
                        out.push(mk(ei, ep, "stop", t, o[t].min(lvl), why, t, false)); cur = None; sold = true;
                    }
                }
            }
            if cur.is_none() && !sold && t < to && h[t] > top {
                cur = Some((t, o[t].max(top)));
            }
        }
        top = top.max(h[t]);
    }
    if let Some((ei, ep)) = cur { out.push(mk(ei, ep, "stop", to, x.c[to], "open", to, true)); }
    out
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(f64::NAN, |d| d.def));
    let u = |k: &str| g(k).round().max(1.) as usize;
    Some(match id {
        "kbttm_donchian32" => donchian(x, from, to, u("n"), g("fill").round() >= 1.),
        "kbttm_ath_breakout" => ath(x, from, to, g("stop") / 100., u("low")),
        _ => return None,
    })
}
