//! Balkrishna M. Sadekar, "How to Make Money Trading the Ichimoku System" (Vision Books 2010; book id `bsichi`; PDF pages).
//! Reading: claude-continuation-20261020/authors/sadekar (agents/bsichi) and authors/lanec/admission3/lanec3_ref.py. Admitted: the five Ch.3
//! "Basic Strategy" rules, each with the book's Kijun-close exit: T/K cross (p.54-62), Kumo breakout (p.63-67), Kijun cross
//! (p.68-76), Chikou breakout (p.77-85), Kumo twist (p.86-94).
//! Held: the Ch.4-5 "high probability" versions (flat Kumo, Chikou 'free and clear', 'small buffer' stops judged by eye) and
//! partial profit taking. Book: Indian stocks (NSE) daily; indices, ETFs and US instruments are a declared application.
use crate::books::lc3::{hi, lo, Pos};
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const REVIEWED: &[&str] = &["bsichi_tk", "bsichi_kumo", "bsichi_kijun", "bsichi_chikou", "bsichi_twist"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const F: &str = "F|Tenkan = (highest high + lowest low of 9 periods) / 2 (p.17); Kijun = same over 26 (p.22); Senkou A = (Tenkan + Kijun) / 2 and Senkou B = (highest high + lowest low of the prior 52 periods) / 2, both 'projected 26 days in the future' (p.33, 37): the cloud under today's bar is the Senkou A/B computed 26 bars ago";
const NOTE: &str = "[Declared readings] Signals and exits on the daily close, filled at that close (the book gives no order type). 'Crosses' = beyond today and not beyond yesterday. One position; an opposite entry reverses. Exit = 'Close out ... as price closes below (short: above) the Kijun', the one listed exit that needs no judgement (the 'small buffer' stop is not defined; partial profits are not modelled). The strong/weak/neutral grading by the Kumo is information, not a filter. The book's 'Alert' to check every component is judgement and is not applied. A position opened on a bar is first tested for the exit on the next bar. No costs.";

fn d(id: &'static str, name: &'static str, group: &'static str, rules: &'static [&'static str]) -> StrategyDef {
    StrategyDef { id, name, group, session: 0, dir: "Long/Short", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: "bsichi", rules, notes: NOTE, claim: None }
}

pub fn catalogue() -> Vec<StrategyDef> {
    vec![
        d("bsichi_tk", "Tenkan/Kijun cross, out on a close beyond the Kijun", "Ch.3 Trading Strategies · T/K Cross (p.54-62)",
          &["E|\"Long positions are initiated when the Tenkan crosses the Kijun to the upside. Short positions are initiated when the Tenkan crosses the Kijun to the downside.\" (p.54)",
            "X|\"Close out your position as price closes below the Kijun.\" (p.62); option p.exit=1: \"Close out of a T/K cross occurs in the opposite direction.\" (p.62)", F]),
        d("bsichi_kumo", "Kumo breakout: close beyond the cloud, out on a close beyond the Kijun", "Ch.3 Trading Strategies · Kumo Breakout (p.63-67)",
          &["E|\"Long positions are initiated when price breaks and closes above the Kumo. Short positions are initiated when prices break and close below the Kumo.\" (p.63) - read: close above the cloud top today, at or below it yesterday",
            "X|\"Close out the position as price closes above / below the Kijun.\" (p.67)", F]),
        d("bsichi_kijun", "Kijun cross: close beyond the Kijun, out (and reverse) on the opposite close", "Ch.3 Trading Strategies · Kijun Cross (p.68-76)",
          &["E|\"Initiate long position when price closes above the Kijun. Initiate short position when price closes below the Kijun.\" (p.68)",
            "X|\"Close out position when the price closes below the Kijun.\" (p.76)", F]),
        d("bsichi_chikou", "Chikou breakout: close crosses the close of 26 days ago, out on a close beyond the Kijun", "Ch.3 Trading Strategies · Chikou Breakout (p.77-85)",
          &["E|\"Initiate long position when Chikou crosses and closes above the price of 26 days ago. Initiate short position when Chikou crosses and closes below the price of 26 days ago.\" (p.77); Chikou = 'the current price shifted back 26' periods - read: today's close vs the close 26 bars ago",
            "X|\"Close out the position as the price closes above / below the Kijun, respectively, depending on whether the trade is long or short.\" (p.85)", F]),
        d("bsichi_twist", "Kumo twist: future Senkou A crosses Senkou B, out on a close beyond the Kijun", "Ch.3 Trading Strategies · Kumo Twist (p.86-94)",
          &["E|\"to enter a long position Senkou A should cross Senkou B to the upside in the future\" (p.86) - read: the Senkou A and B computed today (before the 26-bar projection)",
            "X|\"Close out position as price closes above / below the Kijun\" (p.94); option p.exit=1: \"Close out position if the future Kumo twists back in the other direction.\" (p.94)", F]),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let f = match id { "bsichi_tk" => "T/K cross", "bsichi_kumo" => "Kumo breakout", "bsichi_kijun" => "Kijun cross", "bsichi_chikou" => "Chikou breakout", "bsichi_twist" => "Kumo twist", _ => return None };
    let id = REVIEWED.iter().copied().find(|r| *r == id)?;
    Some((id, "Sadekar Ichimoku", f))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    match id {
        "bsichi_tk" => Some(vec![p("exit", "Exit: 0 close beyond the Kijun (book) / 1 opposite T/K cross (book's other exit)", 0., 0., 1., 1.)]),
        "bsichi_twist" => Some(vec![p("exit", "Exit: 0 close beyond the Kijun (book) / 1 twist back (book's other exit)", 0., 0., 1., 1.)]),
        _ if reviewed(id) => Some(vec![]),
        _ => None,
    }
}

/// (highest high + lowest low) / 2 over the n bars ending at i.
pub fn mid(x: &Ctx, n: usize) -> Vec<f64> {
    (0..x.c.len()).map(|i| if i + 1 < n { f64::NAN } else { (hi(&x.h, i + 1 - n, i) + lo(&x.l, i + 1 - n, i)) / 2. }).collect()
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let schema = params(id)?;
    let alt = p.get("exit").copied().unwrap_or_else(|| schema.iter().find(|d| d.key == "exit").map_or(0., |d| d.def)) > 0.5;
    let c = &x.c;
    let (t, k, sb) = (mid(x, 9), mid(x, 26), mid(x, 52));
    let sa: Vec<f64> = (0..c.len()).map(|i| (t[i] + k[i]) / 2.).collect();
    let lag = |v: &[f64], i: usize| if i >= 26 { v[i - 26] } else { f64::NAN };
    let top = |i: usize| lag(&sa, i).max(lag(&sb, i));
    let bot = |i: usize| lag(&sa, i).min(lag(&sb, i));
    let fin = |v: &[f64]| v.iter().all(|z| z.is_finite());
    // entry signal on bar i (+1 / -1 / 0)
    let sig = |i: usize| -> f64 {
        let (u, dn) = match id {
            "bsichi_tk" => (t[i] > k[i] && t[i - 1] <= k[i - 1], t[i] < k[i] && t[i - 1] >= k[i - 1]),
            "bsichi_kumo" => { if !fin(&[top(i), top(i - 1)]) { return 0.; } (c[i] > top(i) && c[i - 1] <= top(i - 1), c[i] < bot(i) && c[i - 1] >= bot(i - 1)) }
            "bsichi_kijun" => (c[i] > k[i] && c[i - 1] <= k[i - 1], c[i] < k[i] && c[i - 1] >= k[i - 1]),
            "bsichi_chikou" => { if i < 27 { return 0.; } (c[i] > c[i - 26] && c[i - 1] <= c[i - 27], c[i] < c[i - 26] && c[i - 1] >= c[i - 27]) }
            _ => (sa[i] > sb[i] && sa[i - 1] <= sb[i - 1], sa[i] < sb[i] && sa[i - 1] >= sb[i - 1]),
        };
        if u { 1. } else if dn { -1. } else { 0. }
    };
    let mut pos = Pos::new();
    for i in from.max(1)..=to {
        if !fin(&[t[i], t[i - 1], k[i], k[i - 1]]) { continue; }
        let s = pos.side();
        if s != 0. {
            let out = if alt && id == "bsichi_tk" { (s > 0. && t[i] < k[i] && t[i - 1] >= k[i - 1]) || (s < 0. && t[i] > k[i] && t[i - 1] <= k[i - 1]) }
                else if alt && id == "bsichi_twist" { (s > 0. && sa[i] < sb[i] && sa[i - 1] >= sb[i - 1]) || (s < 0. && sa[i] > sb[i] && sa[i - 1] <= sb[i - 1]) }
                else { (s > 0. && c[i] < k[i]) || (s < 0. && c[i] > k[i]) };
            if out { pos.exit(i, c[i], "exit rule"); }
        }
        if id == "bsichi_twist" && !fin(&[sb[i], sb[i - 1]]) { continue; }
        let e = sig(i);
        if e != 0. { pos.enter(e, i, c[i], "close"); }
    }
    Some(pos.done(to, c[to]))
}
