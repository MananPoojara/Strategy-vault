//! Russell Sands, "The Original Turtle Trading Rules" (slide booklet; book id `rsotr`; PDF page cited). Reading:
//! claude-continuation-20261020/authors/sands (admission/stb_ref.py). Live: "The Simple Turtle Breakout System" (PDF 2):
//! buy above the highest high of the past 20 bars (sell below the lowest low), exit below the lowest low of the past 10 bars
//! (shorts above the highest high), a stop at 2% of the account, no profit target. The rest of the booklet (PDF 3-61) is
//! chart and quote-screen slides without text. Book: futures; indices, ETFs and US/India stocks are a declared application.
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};
/// Admitted ids (authors/sands/admission/stb_ref.py (header)).
pub const REVIEWED: &[&str] = &["rsotr_simple_breakout"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] 'Past 20 bars' = the 20 days before today (today excluded); 'goes higher than' = today's high above that level (short: low below). Entry is a stop order: filled at the level, or at the open when the day opens beyond it. If both the 20-day high and the 20-day low are crossed on one day and the open is beyond neither, the order is unknown on daily bars and no trade is taken. Exit: the low goes below the lowest low of the 10 days before today (short: high above the 10-day highest high), filled at that level or at a worse open. 2% of account: with the vault's one position of the whole account, a 2% account loss is a 2% price move, so the stop is entry x 0.98 (short: x 1.02), hit when the low touches it, filled at the stop or a worse open. When the stop and the 10-day exit fall on the same day the higher one (short: lower) fills first. Exits are checked from the day after entry. One position at a time; a new entry can come from the day after an exit (a breakout the other way on the exit day is not taken). Not applied: account-size dollar sizing.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "rsotr_simple_breakout", name: "Simple Turtle Breakout: 20-bar breakout entry, 10-bar exit, 2% stop", group: "The Simple Turtle Breakout System",
        session: 0, dir: "Long/Short", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: "rsotr",
        rules: &["E|\"If a market goes higher than the highest high of the past 20 bars, you want to buy it.\" \"If a market goes lower than the lowest low of the past 20 bars, you want to sell it.\" (Entry Rule, PDF 2)",
            "X|\"If the market goes below the lowest low of the past 10 bars, exit your longs.\" \"If the market goes above the highest high of the past 10 bars, exit your shorts.\" (Exit Rule, PDF 2)",
            "X|\"Place a stop at 2% of your account size to protect from the market going against you.\" \"Do not set a profit target. The only way you get out of the market is if your stop or exit rule is met.\" (Money Management Rule, PDF 2)"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match id { "rsotr_simple_breakout" => Some(("rsotr_simple_breakout", "Sands Simple Turtle Breakout", "20 / 10 / 2%")), _ => None }
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    match id {
        "rsotr_simple_breakout" => Some(vec![p("side", "Sides: 0 = long and short (book), 1 = long only, 2 = short only", 0., 0., 2., 1.),
            p("entry", "Entry: highest high / lowest low of the past N bars (book 20)", 20., 2., 200., 1.),
            p("exit", "Exit: lowest low / highest high of the past N bars (book 10)", 10., 2., 200., 1.),
            p("stop", "Stop, % of the account = % of the entry price (book 2; 0 = off)", 2., 0., 50., 0.5)]),
        _ => None,
    }
}

fn run_stb(x: &Ctx, from: usize, to: usize, side: u8, ne: usize, nx: usize, stop_pct: f64) -> Vec<Trade> {
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let hh = |i: usize, n: usize| h[i - n..i].iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let ll = |i: usize, n: usize| l[i - n..i].iter().cloned().fold(f64::INFINITY, f64::min);
    let mk = |d: f64, ei: usize, ep: f64, xi: usize, xp: f64, why: &str, open: bool| Trade { dir: d, legs: vec![Leg { i: ei, px: ep, w: 1. }], exit_i: xi, exit_px: xp, reason: why.into(), entry_kind: "stop", signal_i: xi, open, exits: vec![] };
    let (mut out, mut cur) = (vec![], None::<(f64, usize, f64)>); // dir, entry bar, entry price
    let start = from.max(ne.max(nx));
    let k = stop_pct / 100.;
    for i in start..=to {
        if let Some((d, ei, ep)) = cur {
            // exits from the day after entry: 2% stop (touch) and the 10-bar channel (strict)
            let lv = if d > 0. { ll(i, nx) } else { hh(i, nx) };
            let st = if k > 0. { Some(ep * (1. - d * k)) } else { None };
            // the level met first: the higher one for a long, the lower one for a short
            let mut best: Option<(f64, &str)> = None;
            let hit_x = if d > 0. { l[i] < lv } else { h[i] > lv };
            if hit_x { best = Some((lv, "channel exit")); }
            if let Some(s) = st {
                let hit_s = if d > 0. { l[i] <= s } else { h[i] >= s };
                if hit_s && best.map_or(true, |(b, _)| if d > 0. { s >= b } else { s <= b }) { best = Some((s, "stop")); }
            }
            if let Some((lvl, why)) = best {
                let px = if (d > 0. && o[i] < lvl) || (d < 0. && o[i] > lvl) { o[i] } else { lvl };
                out.push(mk(d, ei, ep, i, px, why, false)); cur = None;
            }
            continue;
        }
        let (up, dn) = (hh(i, ne), ll(i, ne));
        let (ok_l, ok_s) = (side != 2, side != 1);
        let (bl, bs) = (ok_l && h[i] > up, ok_s && l[i] < dn);
        let (gl, gs) = (ok_l && o[i] > up, ok_s && o[i] < dn);
        if gl { cur = Some((1., i, o[i])); }
        else if gs { cur = Some((-1., i, o[i])); }
        else if bl && !bs { cur = Some((1., i, up)); }
        else if bs && !bl { cur = Some((-1., i, dn)); }
    }
    if let Some((d, ei, ep)) = cur { out.push(mk(d, ei, ep, to, c[to], "open", true)); }
    out
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(f64::NAN, |d| d.def));
    Some(match id {
        "rsotr_simple_breakout" => run_stb(x, from, to, g("side").clamp(0., 2.) as u8, g("entry").max(2.) as usize, g("exit").max(2.) as usize, g("stop").max(0.)),
        _ => return None,
    })
}

// Instrument admission: Russell Sands 2026-10-26 (lane B) block, moved from instrument_policy.rs on 31 Oct 2026 (lane ST).
// Admission only for the module REVIEWED lists (authors/sands/admission/stb_ref.py (header)); kept in one block to merge.
// Runs on actual indices, ETFs and stocks, US and India (index_etf_or_stock, nifty50 basket, declared application).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book=="rsotr" && crate::books::rsotr::reviewed(def.id),
    label: "Russell Sands states the Simple Turtle Breakout System for futures markets; runs on indices, ETFs and US/India stocks are a declared application, not the author's test. Cash-index shorts are hypothetical.",
    basis: "Russell Sands, The Original Turtle Trading Rules (slide booklet), The Simple Turtle Breakout System (PDF 2)",
};
