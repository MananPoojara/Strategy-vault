//! Jack L. Weinberg, "The Range Indicator" (Technical Analysis of Stocks & Commodities V.13:6, June 1995, p.238-242; research share Trading Knowledge I/
//! Stocks & Commodities DVD 11.26/VOLUMES/V13/C06/RANGEIN.PDF, text layer). Book id `jwtri`. The article builds the stochastic range and TRI (Figures 2-3),
//! tests a three-average EMA system on four currencies (q=3, m=10, r=30, $2,000 stop, 1984-94; Fig 5-6), then adds TRI as an entry filter (Fig 7) and as
//! an exit filter (Fig 8). One method here, `jwtri_ema`; p.tri = 0 base system, 1 entry filter, 2 entry and exit filter (default: the article's final system).
//! Lane K1 (23 Nov 2026, queue A5017 / to-code row 419). Independent check authors/conv-1123/K1/jwtri_ref.py.
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "jwtri";
pub const REVIEWED: &[&str] = &["jwtri_ema"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared adaptation, R4 #340] The article tests four currency futures (Swiss franc, Deutschemark, pound, yen; continuous contracts 31 Mar 1984-31 Mar 1994, $100 a trade for costs, $2,000 money stop) and prints only chart pictures (Figures 6-8: 32% winners for the base system, 39% with the entry filter, no per-trade list in the text), so no figure is compared. The vault runs the rule on the indices, ETFs and stocks it holds, always UNVERIFIED; the $2,000 stop is a US-dollar futures amount and is not applied. Default = the article's final system (TRI as entry and exit filter, q 3, m 10, r 30, low zone 20, high zone 70); p.tri 0 gives the base system and 1 the entry filter only. [Declared readings, each an unstated detail] (1) The Figure 3 code prints XAverage(StochRange(m), q) while the text says an average over m of the stochastic range over q; the text is followed (q = 3 for the stochastic, m = 10 for the average). (2) The exit rule with TRI prints 'EMA(q) < EMA(r)' as a state next to 'TRI crosses above the high zone'; both are read on the same bar, the base exit keeps its printed cross. (3) One position at a time: an entry signal is taken only when flat (the article does not say whether a signal reverses an open position). Averages are exponential (XAverage, alpha 2/(n+1), seeded with the first close of the series); every fill is at the signal close ('Buy on the close'). Vault convention: no trade before 90 bars of the series.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "jwtri_ema", name: "Three-average EMA trend system with the Range Indicator (TRI) as entry and exit filter (long and short)",
        group: "Weinberg Range Indicator", session: 0, dir: "Long/Short", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: "jwtri",
        rules: &[
            "F|\"Value1 = TrueRange/(Close-Close[1]) ... unless Close-Close[1] < 0 then Value1 = TrueRange\"; StochRange = 100 x (Value1 - Lowest(Value1,q))/(Highest - Lowest); TRI = XAverage(StochRange, m); q = 3, m = 10, low zone 20, high zone 70 (p.239-240)",
            "E|Buy on the close when EMA(Close,q) is above EMA(Close,m), Close is above EMA(Close,r) and EMA(Close,m), TRI is above yesterday's TRI and crosses above the low zone; the short side is the mirror, the TRI condition as is (Fig 7 text); the base system uses an EMA(q) cross of EMA(m) with no TRI condition (Fig 5)",
            "X|Exit long on the close when BarsSinceEntry > q, EMA(q) is below EMA(r) and TRI crosses above the high zone (Fig 8 text); base exit: BarsSinceEntry > q and EMA(q) crosses below EMA(r); q, m, r = 3, 10, 30"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match id { "jwtri_ema" => Some(("jwtri_ema", "Weinberg Range Indicator system", "EMA 3/10/30, TRI 20/70")), _ => None }
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    match id {
        "jwtri_ema" => Some(vec![p("q", "Short EMA and stochastic range length q (article 3)", 3., 2., 20., 1.), p("m", "Medium EMA and TRI smoothing m (article 10)", 10., 3., 50., 1.),
            p("r", "Long EMA r (article 30)", 30., 5., 100., 1.), p("low", "TRI low zone (article 20)", 20., 5., 50., 5.), p("high", "TRI high zone (article 70)", 70., 50., 95., 5.),
            p("tri", "TRI use: 0 = none (base system), 1 = entry filter, 2 = entry and exit filter (final system)", 2., 0., 2., 1.), p("shorts", "Short side: 1 = yes (article), 0 = long only", 1., 0., 1., 1.)]),
        _ => None,
    }
}

fn ema(c: &[f64], n: usize) -> Vec<f64> {
    let a = 2.0 / (n as f64 + 1.0);
    let mut out = Vec::with_capacity(c.len());
    for (i, &v) in c.iter().enumerate() { out.push(if i == 0 { v } else { a * v + (1.0 - a) * out[i - 1] }); }
    out
}

/// TRI per the article text: stochastic of Value1 over q bars, then an EMA over m (NaN until q bars of Value1 exist).
fn tri(x: &Ctx, q: usize, m: usize) -> Vec<f64> {
    let (h, l, c) = (&x.h, &x.l, &x.c);
    let n = c.len();
    let v1: Vec<f64> = (0..n).map(|i| {
        if i == 0 { return f64::NAN; }
        let t = (h[i] - l[i]).max((h[i] - c[i - 1]).abs()).max((l[i] - c[i - 1]).abs());
        if c[i] - c[i - 1] > 0.0 { t / (c[i] - c[i - 1]) } else { t }
    }).collect();
    let st: Vec<f64> = (0..n).map(|i| {
        if i < q { return f64::NAN; }
        let w = &v1[i + 1 - q..=i];
        let (lo, hi) = (w.iter().cloned().fold(f64::INFINITY, f64::min), w.iter().cloned().fold(f64::NEG_INFINITY, f64::max));
        if hi - lo > 0.0 { 100.0 * (v1[i] - lo) / (hi - lo) } else { 100.0 * (v1[i] - lo) }
    }).collect();
    let a = 2.0 / (m as f64 + 1.0);
    let mut out = vec![f64::NAN; n];
    for i in q..n { out[i] = if i == q { st[i] } else { a * st[i] + (1.0 - a) * out[i - 1] }; }
    out
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let s = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| s.iter().find(|d| d.key == k).unwrap().def);
    let (q, m, r) = (g("q").round() as usize, g("m").round() as usize, g("r").round() as usize);
    let (lo, hi, mode, shorts) = (g("low"), g("high"), g("tri").round() as i32, g("shorts") > 0.5);
    let c = &x.c;
    let (eq, em, er, t) = (ema(c, q), ema(c, m), ema(c, r), tri(x, q, m));
    let (mut out, mut cur) = (vec![], None::<(f64, usize, f64)>);
    for i in from.max(90).max(1)..=to {
        if let Some((d, e, ep)) = cur {
            let cross_r = if d > 0. { eq[i] < er[i] && eq[i - 1] >= er[i - 1] } else { eq[i] > er[i] && eq[i - 1] <= er[i - 1] };
            let state_r = if d > 0. { eq[i] < er[i] } else { eq[i] > er[i] };
            let hit = i - e > q && if mode == 2 { state_r && t[i] > hi && t[i - 1] <= hi } else { cross_r };
            if hit {
                out.push(Trade { dir: d, legs: vec![Leg { i: e, px: ep, w: 1. }], exit_i: i, exit_px: c[i], reason: if mode == 2 { "EMA below/above the long average and TRI crossed the high zone" } else { "EMA(q) crossed the long average" }.into(), entry_kind: "close", signal_i: i, open: false, exits: vec![] });
                cur = None;
            }
            continue;
        }
        let long = c[i] > er[i] && c[i] > em[i] && if mode == 0 { eq[i] > em[i] && eq[i - 1] <= em[i - 1] } else { eq[i] > em[i] };
        let short = c[i] < er[i] && c[i] < em[i] && if mode == 0 { eq[i] < em[i] && eq[i - 1] >= em[i - 1] } else { eq[i] < em[i] };
        let tri_ok = mode == 0 || (t[i] > t[i - 1] && t[i - 1] <= lo && t[i] > lo);
        if tri_ok && long { cur = Some((1., i, c[i])); } else if tri_ok && short && shorts { cur = Some((-1., i, c[i])); }
    }
    if let Some((d, e, ep)) = cur {
        out.push(Trade { dir: d, legs: vec![Leg { i: e, px: ep, w: 1. }], exit_i: to, exit_px: c[to], reason: "open".into(), entry_kind: "close", signal_i: to, open: true, exits: vec![] });
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn rejects_other_ids_and_runs() {
        let r: Vec<Row> = (0..300).map(|i| { let v = 100.0 + (i as f64 * 0.3).sin() * 10.0 + i as f64 * 0.05; Row { ticker: "T".into(), date: format!("2001-{:03}", i), open: v, high: v + 1., low: v - 1., close: v, volume: None } }).collect();
        let x = Ctx::new(&r, &r, &[]);
        let p = Params::default();
        assert!(run("jwtri_x", &x, 0, 299, &p).is_none());
        for z in run("jwtri_ema", &x, 0, 299, &p).unwrap() { assert!(z.exit_i >= z.legs[0].i); }
    }
}

// Instrument admission: Weinberg Range Indicator 2026-11-23 (lane K1). Admission only for the module REVIEWED lists.
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "jwtri" && crate::books::jwtri::reviewed(def.id),
    label: "Weinberg tests the rule on four currency futures (1984-94) and prints charts only; runs on indices, ETFs and US/India stocks are a declared application (R4 #340), not the author's test.",
    basis: "Jack L. Weinberg, The Range Indicator, Technical Analysis of Stocks & Commodities V.13:6 (1995), p.238-242",
};
