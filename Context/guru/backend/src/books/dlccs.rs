//! David Linton, "Cloud Charts: Trading Success with the Ichimoku Technique" (Updata, 2010; book id `dlccs`; PDF page cited,
//! the scan has no printed numbers in the text layer). Reading: claude-continuation-20261020/authors/linton
//! (admission/cloud_ref.py). Live: the price / cloud stop and reverse the book back-tests in Ch.14 (PDF p.101-102), with
//! the book's signal delay (0-5 days) and the lagging-line version as options. Not coded: per-instrument optimised delays
//! and periods (judgement), the weekly-cloud trend filter (p.104-105), the Flip-Flop optimised percentage stop (p.100),
//! cloud breadth (Ch.15, needs breadth data). Book: Euro, S&P 500 stocks, FTSE 100 stocks, currencies; indices, ETFs and
//! US/India stocks here are a declared application.
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};
/// Admitted ids (authors/linton/admission/cloud_ref.py (header)).
pub const REVIEWED: &[&str] = &["dlccs_cloud_reversal"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] Cloud (Ch.8): turning line = midpoint of the 9-bar high-low range, standard line = midpoint of the 26-bar range; Cloud Span A = their average and Cloud Span B = midpoint of the 52-bar range, both drawn 26 bars ahead; lagging line = the close drawn 26 bars back. Above the cloud = close above the higher span, below = close under the lower span; a close inside or on the edge changes nothing. The last outside position is read from all chart history before the test; a trade needs a change of side inside the test. Signal delay d (book tests 0-5 days): the new side counts only after d+1 closes in a row on it (a close back inside restarts the count); d = 0 is the book's plain rule and the default. Lagging-line version: today's close against the cloud drawn 26 bars back, the trade taken today ('the actual trades will occur 26 bars forward where the price is at that time'). Signals on the close, orders at the next open (the book does not state the fill). Long only is an option, not from the book.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "dlccs_cloud_reversal", name: "Cloud chart: stop and reverse on closes through the cloud", group: "Cloud Chart back-tests",
        session: 0, dir: "Long/Short", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: "dlccs",
        rules: &["F|Cloud Span A = average of the turning line (9) and standard line (26), Cloud Span B = midpoint of the 52-bar range, both plotted 26 bars ahead (Ch.8); the cloud construction periods of 9, 26 and 52 (Ch.14 PDF p.101).",
            "E|\"A buy signal (long entry, short exit) is a close above the cloud, where the last 'price outside cloud position' was below the cloud. A sell signal (long exit, short entry) is where the price crosses below the cloud\" (PDF p.101).",
            "F|Signal delay: \"should we wait a number of bars after a cross?\" ... \"testing waiting a day, two days, and so on up to five days\" (PDF p.101); lagging-line version: \"the actual trades will occur 26 bars forward where the price is at that time\" (PDF p.101).",
            "X|Stop and reverse: always in the market after the first signal; the opposite signal exits and reverses (PDF p.101)."],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match id { "dlccs_cloud_reversal" => Some(("dlccs_cloud_reversal", "Linton cloud stop and reverse", "price, delay 0")), _ => None }
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    match id {
        "dlccs_cloud_reversal" => Some(vec![p("delay", "Signal delay in days (book tests 0-5; default 0 = plain rule)", 0., 0., 5., 1.),
            p("line", "Signal line: 0 = price (default), 1 = lagging line", 0., 0., 1., 1.),
            p("turning", "Turning line bars (book 9)", 9., 2., 60., 1.), p("standard", "Standard line bars (book 26)", 26., 2., 120., 1.),
            p("spanb", "Cloud Span B bars (book 52)", 52., 2., 240., 1.), p("offset", "Cloud / lagging offset bars (book 26)", 26., 1., 120., 1.),
            p("side", "Sides: 0 = stop and reverse (book), 1 = long only [not from the book]", 0., 0., 1., 1.)]),
        _ => None,
    }
}

fn midpoint(x: &Ctx, i: usize, n: usize) -> f64 {
    if n == 0 || i + 1 < n { return f64::NAN; }
    let hh = x.h[i + 1 - n..=i].iter().cloned().fold(f64::MIN, f64::max);
    let ll = x.l[i + 1 - n..=i].iter().cloned().fold(f64::MAX, f64::min);
    (hh + ll) / 2.
}

/// +1 / -1 accepted-side change on bar t (0 = none), from all history up to `to`.
fn signals(x: &Ctx, to: usize, per: [usize; 4], delay: usize, lagging: bool) -> Vec<i8> {
    let [tn, sn, bn, off] = per;
    let lag = if lagging { off } else { 0 };
    let (mut acc, mut run, mut cnt) = (0i8, 0i8, 0usize);
    let mut sig = vec![0i8; to + 1];
    for t in 0..=to {
        let s = if t < off + lag { 0 } else {
            let j = t - off - lag;
            let a = (midpoint(x, j, tn) + midpoint(x, j, sn)) / 2.;
            let b = midpoint(x, j, bn);
            if a.is_nan() || b.is_nan() { 0 } else if x.c[t] > a.max(b) { 1 } else if x.c[t] < a.min(b) { -1 } else { 0 }
        };
        sig[t] = step(&mut acc, &mut run, &mut cnt, s, delay);
    }
    sig
}

/// One close: s = +1 above / -1 below / 0 inside. Returns the new side when it is accepted after `delay`+1 closes in a
/// row and there was an earlier outside position (the first one only sets the state).
fn step(acc: &mut i8, run: &mut i8, cnt: &mut usize, s: i8, delay: usize) -> i8 {
    if s == 0 { *run = 0; *cnt = 0; } else if s == *run { *cnt += 1; } else { *run = s; *cnt = 1; }
    if *run == 0 || *cnt <= delay || *run == *acc { return 0; }
    let had = *acc != 0;
    *acc = *run;
    if had { *run } else { 0 }
}

fn run_cloud(x: &Ctx, from: usize, to: usize, per: [usize; 4], delay: usize, lagging: bool, long_only: bool) -> Vec<Trade> {
    let sig = signals(x, to, per, delay, lagging);
    let mk = |d: f64, ei: usize, ep: f64, xi: usize, xp: f64, why: &str, open: bool, si: usize| Trade { dir: d, legs: vec![Leg { i: ei, px: ep, w: 1. }], exit_i: xi, exit_px: xp, reason: why.into(), entry_kind: "open", signal_i: si, open, exits: vec![] };
    let (mut out, mut cur) = (vec![], None::<(f64, usize, f64)>);
    for t in from..to {
        if sig[t] == 0 { continue; }
        let (d, f) = (sig[t] as f64, t + 1);
        let px = x.o[f];
        if let Some((cd, ei, ep)) = cur {
            if cd != d { out.push(mk(cd, ei, ep, f, px, if long_only { "exit" } else { "reverse" }, false, t)); cur = None; }
        }
        if cur.is_none() && (!long_only || d > 0.) { cur = Some((d, f, px)); }
    }
    if let Some((d, ei, ep)) = cur { out.push(mk(d, ei, ep, to, x.c[to], "open", true, to)); }
    out
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(f64::NAN, |d| d.def));
    let u = |k: &str| g(k).round().max(1.) as usize;
    Some(match id {
        "dlccs_cloud_reversal" => run_cloud(x, from, to, [u("turning"), u("standard"), u("spanb"), u("offset")],
            g("delay").round().clamp(0., 5.) as usize, g("line").round() >= 1., g("side").round() >= 1.),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn delay_counts_consecutive_closes() {
        let hits = |closes: &[i8], d: usize| { let (mut a, mut r, mut c) = (0i8, 0i8, 0usize);
            closes.iter().map(|&s| step(&mut a, &mut r, &mut c, s, d)).collect::<Vec<_>>() };
        assert_eq!(hits(&[-1, 1, 0, 1, 1], 0), vec![0, 1, 0, 0, 0]); // first outside sets state; one close above = buy
        assert_eq!(hits(&[-1, -1, 1, 0, 1, 1], 1), vec![0, 0, 0, 0, 0, 1]); // delay 1: a one-day breach is ignored
        assert_eq!(hits(&[1, -1, 1], 0), vec![0, -1, 1]);
    }
}

// Instrument admission: David Linton 2026-10-29 (lane B) block, moved from instrument_policy.rs on 31 Oct 2026 (lane ST).
// Admission only for the module REVIEWED lists (authors/linton/admission/cloud_ref.py (header)); kept in one block to merge.
// Runs on actual indices, ETFs and stocks, US and India (index_etf_or_stock, nifty50 basket, declared application).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book=="dlccs" && crate::books::dlccs::reviewed(def.id),
    label: "David Linton back-tests the cloud stop and reverse on the Euro, currencies and S&P 500 / FTSE 100 stocks; runs on indices, ETFs and US/India stocks are a declared application, not the author's test. Cash-index shorts are hypothetical.",
    basis: "David Linton, Cloud Charts (2010), Ch.8 cloud construction and Ch.14 back-testing (PDF p.100-105)",
};
