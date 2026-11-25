//! Paul Evans, "Forex Soloist Method" (forexvirtuoso.com; research share USB HDD 2/Trading Knowledge 3/Forex Virtuoso Trading Method (forexvirtuoso.com)/soloist1.pdf and soloist3.pdf,
//! text layer, read in full). Book id `fxsolo`. Indicators 80/21/13/5/3 EMA and RSI(21). Long when the 3 EMA crosses the 5 EMA upward, both cross the 13/21 channel, RSI is above 50 and the
//! 21 and 13 EMAs are above the 80 EMA; exit when the 3 EMA crosses below the 5 EMA or RSI < 50; short mirrored. The guide uses 1H or longer; its sixth chart example is on the DAILY chart.
//! Lane K1 (23 Nov 2026, queue Q381 rows 375/376). Independent check authors/conv-1123/K1/fxsolo_ref.py.
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "fxsolo";
pub const REVIEWED: &[&str] = &["fxsolo_daily"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared adaptation, R4 #340/#380] The method is a forex (EUR/USD, GBP/USD, USD/JPY) system on 1-hour or longer charts; this card runs its rules on the NSE indices NIFTY 50 and NIFTY BANK (and the US indices) on DAILY bars, the timeframe of the guide's own sixth example. The guide prints no results; nothing is compared, UNVERIFIED. [Declared readings, each an unstated detail] Card-specific unstated choices (3), each independent: (1) 'both 3 EMA and 5 EMA cross a channel of 13 and 21 EMA' = on the signal bar both the 3 and the 5 EMA are above (short: below) both the 13 and the 21 EMA; (2) RSI(21) is Wilder's, seeded with the mean of the first 21 changes; (3) the 20-30 pip stop is currency-pair specific and has no daily value, so it is an optional percentage stop, default 0 = none. Honest total: 3 card-specific choices. (Audit count 5; its items 2 (EMA seed) and 4 (one position, same-direction entry ignored) are engine defaults per #404 and are moved out; the audit's item 5 (stop omitted) is (3) here.) Engine-wide defaults / declared adaptations (labelled, not counted, R4 #404): the EMAs are seeded with the first close; no signal before bar max(80, 22), the longest indicator warm-up; one position at a time, an entry in the direction already held is ignored; an opposite signal reverses the position at the same open that exits the old one; the signal is read at a close and filled at the NEXT bar's open; indices only (stocks and ETFs refused, #364); NIFTY 50 / NIFTY BANK and US indices on daily bars as the declared adaptation of the forex system (#340/#380); no costs.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "fxsolo_daily", name: "Forex Soloist: 3/5 EMA cross with the 13/21 EMA channel, RSI(21) and the 80 EMA trend filter (daily, indices)",
        group: "Forex Soloist EMA/RSI", session: 0, dir: "Long/Short", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: "fxsolo",
        rules: &[
            "E|\"We enter a long trade (buy) when 3 EMA crosses 5 EMA upward in an uptrend market; both 3 EMA and 5 EMA cross a channel of 13 and 21 EMA; RSI is above 50; both 21 EMA and 13 EMA are above 80 EMA\" (short: the mirror image)",
            "X|\"We exit our trade when 3 EMA crosses below 5 EMA or RSI < 50\" (short: 3 EMA crosses above 5 EMA or RSI > 50)",
            "X|\"Stop loss = 20 - 30 pips ... If you use other timeframes, tweak the stop loss accordingly\" (optional here)"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match id { "fxsolo_daily" => Some(("fxsolo", "Forex Soloist (EMA 3/5/13/21/80 + RSI 21)", "daily, no stop")), _ => None }
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    match id {
        "fxsolo_daily" => Some(vec![p("rsi", "RSI length (guide 21)", 21., 5., 50., 1.), p("trend", "Trend EMA length (guide 80)", 80., 20., 200., 1.),
            p("stop", "Optional stop, percent of the entry price (0 = none; the guide's 20-30 pips is pair-specific)", 0., 0., 10., 0.25)]),
        _ => None,
    }
}

fn ema(v: &[f64], n: usize) -> Vec<f64> {
    let a = 2.0 / (n as f64 + 1.0);
    let mut o = Vec::with_capacity(v.len());
    for (i, &x) in v.iter().enumerate() { o.push(if i == 0 { x } else { o[i - 1] + a * (x - o[i - 1]) }); }
    o
}
fn rsi(c: &[f64], n: usize) -> Vec<f64> {
    let mut o = vec![f64::NAN; c.len()];
    if c.len() <= n { return o; }
    let (mut g, mut l) = (0.0, 0.0);
    for i in 1..=n { let d = c[i] - c[i - 1]; if d > 0.0 { g += d } else { l -= d } }
    let (mut ag, mut al) = (g / n as f64, l / n as f64);
    o[n] = if al == 0.0 { 100.0 } else { 100.0 - 100.0 / (1.0 + ag / al) };
    for i in n + 1..c.len() {
        let d = c[i] - c[i - 1];
        ag = (ag * (n as f64 - 1.0) + d.max(0.0)) / n as f64; al = (al * (n as f64 - 1.0) + (-d).max(0.0)) / n as f64;
        o[i] = if al == 0.0 { 100.0 } else { 100.0 - 100.0 / (1.0 + ag / al) };
    }
    o
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let s = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| s.iter().find(|d| d.key == k).unwrap().def);
    let (nr, nt, stop) = (g("rsi").round() as usize, g("trend").round() as usize, g("stop") / 100.0);
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let (e3, e5, e13, e21, e80) = (ema(c, 3), ema(c, 5), ema(c, 13), ema(c, 21), ema(c, nt));
    let rs = rsi(c, nr);
    let warm = nt.max(nr + 1);
    let mut out = vec![];
    let mut pos: Option<(f64, usize, f64)> = None;
    let (mut pend_exit, mut pend_entry) = (false, 0.0f64);
    let mk = |d: f64, e: usize, ep: f64, xi: usize, xp: f64, why: &str, open: bool| Trade { dir: d, legs: vec![Leg { i: e, px: ep, w: 1. }], exit_i: xi, exit_px: xp, reason: why.into(), entry_kind: "open", signal_i: e.saturating_sub(1), open, exits: vec![] };
    for t in 0..=to {
        if t < from { continue; }
        if pend_exit { if let Some((d, e, ep)) = pos.take() { out.push(mk(d, e, ep, t, o[t], "3/5 EMA cross back or RSI through 50", false)); } pend_exit = false; }
        if pend_entry != 0.0 { if pos.is_none() { pos = Some((pend_entry, t, o[t])); } pend_entry = 0.0; }
        if let Some((d, e, ep)) = pos {
            if stop > 0.0 {
                let lv = ep * (1.0 - d * stop);
                if (d > 0.0 && l[t] <= lv) || (d < 0.0 && h[t] >= lv) {
                    out.push(mk(d, e, ep, t, if d > 0.0 { o[t].min(lv) } else { o[t].max(lv) }, "stop", false)); pos = None;
                }
            }
        }
        if t < warm || t == 0 || t >= to || rs[t].is_nan() { continue; }
        let up = e3[t - 1] <= e5[t - 1] && e3[t] > e5[t];
        let dn = e3[t - 1] >= e5[t - 1] && e3[t] < e5[t];
        let (hi, lo) = (e13[t].max(e21[t]), e13[t].min(e21[t]));
        let long = up && e3[t].min(e5[t]) > hi && rs[t] > 50.0 && lo > e80[t];
        let short = dn && e3[t].max(e5[t]) < lo && rs[t] < 50.0 && hi < e80[t];
        let sig = if long { 1.0 } else if short { -1.0 } else { 0.0 };
        if let Some((d, _, _)) = pos {
            let ex = if d > 0.0 { dn || rs[t] < 50.0 } else { up || rs[t] > 50.0 };
            if sig != 0.0 && sig != d { pend_exit = true; pend_entry = sig; } else if ex { pend_exit = true; }
        } else if sig != 0.0 { pend_entry = sig; }
    }
    if let Some((d, e, ep)) = pos { out.push(mk(d, e, ep, to, c[to], "open", true)); }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn trades_and_rejects_other_ids() {
        let r: Vec<Row> = (0..400).map(|i| { let c = 100. + (i as f64 * 0.09).sin() * 12. + i as f64 * 0.1; Row { ticker: "NIFTY 50".into(), date: format!("2010-{:03}", i), open: c - 0.2, high: c + 1., low: c - 1., close: c, volume: None } }).collect();
        let x = Ctx::new(&r, &r, &[]);
        assert!(run("fxsolo_x", &x, 0, 399, &Params::default()).is_none());
        let t = run("fxsolo_daily", &x, 0, 399, &Params::default()).unwrap();
        for z in &t { assert!(z.exit_i >= z.legs[0].i); }
    }
}

// Instrument admission: indices only (never single stocks or ETFs), declared adaptation.
pub fn index_only() -> &'static str { "Forex Soloist (forex EMA/RSI system): run on actual indices only (NIFTY 50 / NIFTY BANK and the US indices) as a declared adaptation (R4 #340/#380); UNVERIFIED" }
