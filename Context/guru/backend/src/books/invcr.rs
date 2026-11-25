//! INVESTools, "Advanced Currency Trader Course Manual" (2006), Section 1 "System Development and Testing" (research share Trading Knowledge I/@investools.com/Advanced Currency Trader Manual/INVESTools ADVANCED_CURRENCY_TRADER_MANUAL.pdf,
//! text layer, read in full). Book id `invcr`. The manual's worked trending system: an 8-17-9 MACD and a 21-day simple moving average; "generates a buy signal when the MACD histogram crosses above the
//! signal line while the moving average is moving higher ... a sell signal when [it] crosses below the signal line while the moving average is moving lower"; exit "when it crosses back over the signal
//! line in the opposite direction" with "a backup stop at 100 pips". (Its non-trending RSI system, stop 50 / limit 200 pips, is not published here: see the gate note.) Lane K1 (24 Nov 2026, queue Q381 row 83).
//! Independent check authors/conv-1123/K1/invcr_ref.py.
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "invcr";
pub const REVIEWED: &[&str] = &["invcr_macd"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared adaptation, R4 #340/#380] The manual's currency example is run on the NSE indices NIFTY 50 and NIFTY BANK (and the US indices) on DAILY bars (the manual's moving average is a '21-day' average). It is a course example, prints no results, nothing is compared, UNVERIFIED. [Declared readings, six card-specific unstated choices, R4 #400/#404] (1) MACD(8,17,9) = EMA 8 minus EMA 17 of the close with a 9-period EMA signal line; the EMAs are seeded with the first close and the first 40 bars only warm up. (2) 'The histogram crosses above the signal line' = the MACD line crosses above its signal line (histogram turns positive). (3) 'The moving average is moving higher/lower' = the 21-day SMA is above/below its value one bar earlier on the signal bar. (4) The exit is the opposite cross, whatever the SMA does. (5) A qualifying opposite entry reverses at the same open as the exit. (6) The backup stop is tested from the entry bar itself, after the open fill. Honest total: 6 card-specific choices. Engine-wide defaults / declared adaptations (labelled, not counted, R4 #404): signals are read at a close and filled at the NEXT bar's open; the 100-pip backup stop is scaled by ONE rule, 1 pip = 0.01 percent of price (the manual prints no price): 1.0 percent from the entry, a gap through it fills at the open.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "invcr_macd", name: "INVESTools trending system: MACD 8-17-9 cross with the 21-day SMA direction, exit on the opposite cross, 100-pip backup stop (daily, indices)",
        group: "INVESTools Advanced Currency Trader", session: 0, dir: "Long/Short", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: "invcr",
        rules: &[
            "E|\"The trending system - which is based on an 8-17-9 moving average convergence divergence (MACD) indicator and a 21-day simple moving average - generates a buy signal when the MACD histogram crosses above the signal line while the moving average is moving higher. It generates a sell signal when the MACD histogram crosses below the signal line while the moving average is moving lower.\"",
            "X|\"The histogram will generate an exit signal when it crosses back over the signal line in the opposite direction. ... you should also have a backup stop loss ... a 'Stop' at 100 pips.\""],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match id { "invcr_macd" => Some(("invcr", "INVESTools trending system (MACD 8-17-9 + SMA 21)", "backup stop 1 percent")), _ => None }
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    match id {
        "invcr_macd" => Some(vec![p("fast", "MACD fast EMA (manual 8)", 8., 3., 30., 1.), p("slow", "MACD slow EMA (manual 17)", 17., 5., 60., 1.), p("sig", "Signal EMA (manual 9)", 9., 3., 30., 1.),
            p("sma", "Moving average length (manual 21)", 21., 5., 100., 1.), p("stop", "Backup stop, percent of the entry (100 pips scaled at 1 pip = 0.01 percent)", 1.0, 0.1, 10.0, 0.1)]),
        _ => None,
    }
}

fn ema(v: &[f64], n: usize) -> Vec<f64> {
    let a = 2.0 / (n as f64 + 1.0);
    let mut o = Vec::with_capacity(v.len());
    for (i, &x) in v.iter().enumerate() { o.push(if i == 0 { x } else { o[i - 1] + a * (x - o[i - 1]) }); }
    o
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let s = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| s.iter().find(|d| d.key == k).unwrap().def);
    let (nf, ns, ng, na, stop) = (g("fast").round() as usize, g("slow").round() as usize, g("sig").round() as usize, g("sma").round() as usize, g("stop") / 100.0);
    let (o, l, h, c) = (&x.o, &x.l, &x.h, &x.c);
    let macd: Vec<f64> = ema(c, nf).iter().zip(ema(c, ns).iter()).map(|(a, b)| a - b).collect();
    let sg = ema(&macd, ng);
    let sm: Vec<f64> = (0..c.len()).map(|i| if i + 1 >= na { c[i + 1 - na..=i].iter().sum::<f64>() / na as f64 } else { f64::NAN }).collect();
    let warm = 40usize.max(na + 1);
    let mut out = vec![];
    let mut pos: Option<(f64, usize, f64)> = None;
    let mut pend: Option<(bool, f64)> = None; // (exit?, entry dir 0 = none)
    let mk = |d: f64, e: usize, ep: f64, xi: usize, xp: f64, why: &str, open: bool| Trade { dir: d, legs: vec![Leg { i: e, px: ep, w: 1. }], exit_i: xi, exit_px: xp, reason: why.into(), entry_kind: "open", signal_i: e.saturating_sub(1), open, exits: vec![] };
    for t in 0..=to {
        if t < from { continue; }
        if let Some((ex, dir)) = pend.take() {
            if ex { if let Some((d, e, ep)) = pos.take() { out.push(mk(d, e, ep, t, o[t], "MACD crossed back", false)); } }
            if dir != 0.0 && pos.is_none() { pos = Some((dir, t, o[t])); }
        }
        if let Some((d, e, ep)) = pos {
            let lv = ep * (1.0 - d * stop);
            if (d > 0.0 && l[t] <= lv) || (d < 0.0 && h[t] >= lv) { out.push(mk(d, e, ep, t, if d > 0.0 { o[t].min(lv) } else { o[t].max(lv) }, "100-pip backup stop", false)); pos = None; }
        }
        if t < warm || t >= to || sm[t - 1].is_nan() { continue; }
        let up = macd[t - 1] <= sg[t - 1] && macd[t] > sg[t];
        let dn = macd[t - 1] >= sg[t - 1] && macd[t] < sg[t];
        let held = pos.map_or(0.0, |z| z.0);
        let want = if up && sm[t] > sm[t - 1] { 1.0 } else if dn && sm[t] < sm[t - 1] { -1.0 } else { 0.0 };
        let exit = (up && held < 0.0) || (dn && held > 0.0);
        if exit || (want != 0.0 && held == 0.0) { pend = Some((exit, if want != held { want } else { 0.0 })); }
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
        let r: Vec<Row> = (0..300).map(|i| { let c = 100. + (i as f64 * 0.12).sin() * 10. + i as f64 * 0.1; Row { ticker: "NIFTY 50".into(), date: format!("2010-{:03}", i), open: c - 0.2, high: c + 1., low: c - 1., close: c, volume: None } }).collect();
        let x = Ctx::new(&r, &r, &[]);
        assert!(run("invcr_x", &x, 0, 299, &Params::default()).is_none());
        let t = run("invcr_macd", &x, 0, 299, &Params::default()).unwrap();
        for z in &t { assert!(z.exit_i >= z.legs[0].i); }
    }
}

// Instrument admission: indices only (never single stocks or ETFs), declared adaptation.
pub fn index_only() -> &'static str { "INVESTools trending system (currency course example): run on actual indices only (NIFTY 50 / NIFTY BANK and the US indices) as a declared adaptation (R4 #340/#380); UNVERIFIED" }
