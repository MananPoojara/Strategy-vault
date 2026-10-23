//! Martin J. Pring, "How to Select Stocks Using Technical Analysis" (pring.com course with a MetaStock plug-in CD).
//! Book id `mpsel`. The rules are the CD's MetaStock System Tester file (MS70PRFT.DTA, 108 systems, decoded in
//! claude-continuation-20261020/authors/pring/text/mpsel_systems_ms70prft.txt) and the explorations of the same names;
//! the course text itself (a 156 MB archive on the share) was not opened. Reading: authors/pring/agents/mpsel/.
//! Formulas run as printed with MetaStock semantics: Cross(a,b) = a > b today and a <= b yesterday; Mov(x,n,S) simple,
//! Mov(x,n,E) exponential (2/(n+1)); RSI = Wilder; ROC(C,n,%) = 100 (C / C n bars ago - 1); fills at the signal close.
use crate::connors::{rsi, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};
use crate::books::mpts::sma;

pub const BOOK: &str = "mpsel";

/// Admitted after the 2026-10-23 reading and independent ledgers (authors/pring/admission/SOURCE-CONTRACT.md).
pub const REVIEWED: &[&str] = &["mpsel_kst_lt", "mpsel_wellinvested", "mpsel_cmo7", "mpsel_adv_ma"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const CONV: &str = "MetaStock System Tester formulas from the course CD, run as printed: signals at the close, filled at that close (MetaStock default; the files do not state fills) [declared]. An exit and the opposite entry on the same bar reverse the position. An exit condition is checked from the bar after the entry [declared]. History before the selected start warms the indicators; a run starts flat and waits for the next entry cross. The CD has no instrument list beyond its stock scans; indices, ETFs and stocks in the US and India are a declared application.";

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, group, dir, rules, notes| StrategyDef { id, name, group, session: 0, dir, status: Status::Live, needs_volume: false, needs_vix: false,
        needs_series: &[], book: BOOK, rules, notes, claim: None };
    vec![
        d("mpsel_kst_lt", "Pring Long-term Momentum System: daily long-term KST vs its 130-day average", "CD · pRING SYSTEM TEST", "Long",
          &["F|Daily long-term KST = (Mov(ROC(C,180,%),120,S) x1 + Mov(ROC(C,240,%),120,S) x2 + Mov(ROC(C,360,%),120,S) x3 + Mov(ROC(C,480,%),180,S) x4) / 10 (system 'pRING SYSTEM TEST')",
            "E|ENTER LONG: Cross(KST, Mov(KST, opt1, S)); the CD stores opt1 only as an optimisation range 5-140; the exploration of the same name fixes 130 ('KST/10 > Mov(KST/10,130,S)') and the exit uses 130, so 130 is the default",
            "X|CLOSE LONG: Cross(Mov(KST,130,S), KST); the short slots are empty (long only)",
            "F|p.rsi = 1: 'pRING SYSTEM TEST WITH RSI': buy when KST > its 130-day MA AND Mov(RSI(9),10,E) crosses above Mov(Mov(RSI(9),10,E),10,E); sell when it crosses back below (exploration: 'a short-term momentum buy signal when the long-term momentum is positive')"],
          "Long only. The scan description also says 'where the long-term KST is below zero', which no formula tests [not used]. Needs about 790 daily bars before the first signal. EMA seeded with the first value [declared]. A different daily KST in the 'Combo Daily Mo' scan (195/265/390/530) has no exit and is not run. ".to_string().leak()),
        d("mpsel_wellinvested", "Wellinvested NASDAQ System: 10-day MA crossover with smoothed-RSI exits", "CD · Wellinvested NASDAQ System", "Long/Short",
          &["E|ENTER LONG: Cross(C, Mov(C,10,S)); ENTER SHORT: Cross(Mov(C,10,S), C)",
            "X|CLOSE LONG: Cross(Mov(C,10,S), C) OR Cross(65, Mov(RSI(9),8,S)) - the 8-day average of RSI(9) falls back below 65",
            "X|CLOSE SHORT: Cross(C, Mov(C,10,S)) OR Cross(35, Mov(RSI(9),8,S)) - as printed, the smoothed RSI falls below 35"],
          CONV),
        d("mpsel_cmo7", "MA crossover with a 7-day CMO profit exit (27/7/38)", "CD · ma cross cmo 27/7/38/-38", "Long/Short",
          &["E|ENTER LONG: Cross(C, Mov(C,27,S)); ENTER SHORT: Cross(Mov(C,27,S), C)",
            "X|CLOSE LONG: Cross(Mov(C,27,S), C) OR When(CMO(C,7) > 38); CLOSE SHORT: Cross(C, Mov(C,27,S)) OR When(CMO(C,7) < -38)",
            "F|p.ma = 24, p.lvl = 48: the CD's 'ma cross cmo mark 24/7/48/-48' system"],
          "CMO = Chande Momentum Oscillator, 100 x (sum of up closes - sum of down closes) / (sum of both) over the last 7 changes (MetaStock CMO). Same structure as Trading Systems Explained Ch.14 with the CMO as the oscillator. ".to_string().leak()),
        d("mpsel_adv_ma", "Advanced MA crossover: close vs the 10-day MA of 5 days ago", "CD · Advanced MA Crossover", "Long/Short",
          &["E|ENTER LONG: Cross(C, Ref(Mov(C,10,S),-5)); ENTER SHORT: Cross(Ref(Mov(C,10,S),-5), C)",
            "X|Always in: each entry closes the opposite position"],
          CONV),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "mpsel_kst_lt" => ("mpsel_kst", "Pring Long-term Momentum (daily KST)", "KST vs 130-day MA"),
        "mpsel_wellinvested" => ("mpsel_ma", "Pring CD MA systems", "10-day MA + RSI 65/35"),
        "mpsel_cmo7" => ("mpsel_ma", "Pring CD MA systems", "27-day MA + CMO 7 / 38"),
        "mpsel_adv_ma" => ("mpsel_ma", "Pring CD MA systems", "10-day MA advanced 5"),
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    Some(match id {
        "mpsel_kst_lt" => vec![p("ma", "Entry MA of the KST (days; CD range 5-140)", 130., 5., 140., 5.), p("rsi", "Smoothed-RSI timing version (1)", 0., 0., 1., 1.)],
        "mpsel_wellinvested" => vec![p("ma", "MA (days)", 10., 2., 60., 1.), p("ob", "Long exit: smoothed RSI below", 65., 50., 90., 1.), p("os", "Short exit: smoothed RSI below", 35., 10., 50., 1.)],
        "mpsel_cmo7" => vec![p("ma", "MA (days)", 27., 2., 60., 1.), p("cmo", "CMO span", 7., 2., 30., 1.), p("lvl", "CMO exit level (+/-)", 38., 10., 90., 1.)],
        "mpsel_adv_ma" => vec![p("ma", "MA (days)", 10., 2., 60., 1.), p("adv", "Advanced by (days)", 5., 0., 20., 1.)],
        _ => return None,
    })
}

/// MetaStock exponential average: 2/(n+1), seeded with the first finite value.
pub fn ema_ms(v: &[f64], n: usize) -> Vec<f64> {
    let a = 2. / (n as f64 + 1.);
    let mut e = f64::NAN;
    v.iter().map(|&x| { if x.is_finite() { e = if e.is_finite() { e + a * (x - e) } else { x }; } e }).collect()
}

fn roc_pct(c: &[f64], n: usize) -> Vec<f64> { (0..c.len()).map(|i| if i >= n { 100. * (c[i] / c[i - n] - 1.) } else { f64::NAN }).collect() }

/// MetaStock Cross(a, b) on bar i.
fn cross(a: &[f64], b: &[f64], i: usize) -> bool {
    i > 0 && [a[i], b[i], a[i - 1], b[i - 1]].iter().all(|v| v.is_finite()) && a[i] > b[i] && a[i - 1] <= b[i - 1]
}

pub fn cmo(c: &[f64], n: usize) -> Vec<f64> { crate::books::mpts::cmo(c, n) }

pub fn kst_daily(c: &[f64]) -> Vec<f64> {
    let parts = [(180, 120, 1.), (240, 120, 2.), (360, 120, 3.), (480, 180, 4.)];
    let sm: Vec<Vec<f64>> = parts.iter().map(|&(r, s, _)| sma(&roc_pct(c, r), s)).collect();
    (0..c.len()).map(|i| parts.iter().zip(&sm).map(|(&(_, _, w), s)| w * s[i]).sum::<f64>() / 10.).collect()
}

fn trade(dir: f64, i: usize, px: f64) -> Trade {
    Trade { dir, legs: vec![Leg { i, px, w: 1. }], exit_i: i, exit_px: px, reason: String::new(), entry_kind: "close", signal_i: i, open: false, exits: vec![] }
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    if from > to || to >= x.r.len() { return None; }
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).unwrap().def);
    let u = |k: &str| g(k).round().max(1.) as usize;
    let c = &x.c;
    let n = c.len();
    // per bar: (enter long, enter short, close long, close short) and the reasons
    let mut el = vec![false; n]; let mut es = vec![false; n]; let mut xl = vec![false; n]; let mut xs = vec![false; n];
    let mut why_l = vec![""; n]; let mut why_s = vec![""; n];
    match id {
        "mpsel_kst_lt" => {
            let k = kst_daily(c);
            let ma_in = sma(&k, u("ma"));
            let ma_out = sma(&k, 130);
            if g("rsi") >= 0.5 {
                let r1 = ema_ms(&rsi(c, 9), 10);
                let r2 = ema_ms(&r1, 10);
                for i in 0..n {
                    el[i] = k[i].is_finite() && ma_in[i].is_finite() && k[i] > ma_in[i] && cross(&r1, &r2, i);
                    xl[i] = cross(&r2, &r1, i); why_l[i] = "smoothed RSI crossed below its average";
                }
            } else {
                for i in 0..n { el[i] = cross(&k, &ma_in, i); xl[i] = cross(&ma_out, &k, i); why_l[i] = "KST crossed below its 130-day average"; }
            }
        }
        "mpsel_wellinvested" | "mpsel_cmo7" | "mpsel_adv_ma" => {
            let base = sma(c, u("ma"));
            let adv = if id == "mpsel_adv_ma" { g("adv").round().max(0.) as usize } else { 0 };
            let ma: Vec<f64> = (0..n).map(|i| if i >= adv { base[i - adv] } else { f64::NAN }).collect();
            let (osc_l, osc_s): (Vec<bool>, Vec<bool>) = match id {
                "mpsel_wellinvested" => {
                    let r = sma(&rsi(c, 9), 8);
                    let (ob, os) = (vec![g("ob"); n], vec![g("os"); n]);
                    ((0..n).map(|i| cross(&ob, &r, i)).collect(), (0..n).map(|i| cross(&os, &r, i)).collect())
                }
                "mpsel_cmo7" => {
                    let m = cmo(c, u("cmo"));
                    let l = g("lvl");
                    ((0..n).map(|i| m[i].is_finite() && m[i] > l).collect(), (0..n).map(|i| m[i].is_finite() && m[i] < -l).collect())
                }
                _ => (vec![false; n], vec![false; n]),
            };
            for i in 0..n {
                el[i] = cross(c, &ma, i); es[i] = cross(&ma, c, i);
                xl[i] = es[i] || osc_l[i]; xs[i] = el[i] || osc_s[i];
                why_l[i] = if es[i] { "close crossed below the average" } else if id == "mpsel_cmo7" { "CMO above its level" } else { "smoothed RSI fell below 65" };
                why_s[i] = if el[i] { "close crossed above the average" } else if id == "mpsel_cmo7" { "CMO below its level" } else { "smoothed RSI fell below 35" };
            }
        }
        _ => return None,
    }
    let mut out = vec![];
    let mut cur: Option<Trade> = None;
    for i in from..=to {
        if let Some(t) = cur.as_ref() {
            let long = t.dir > 0.;
            if i > t.legs[0].i && (if long { xl[i] } else { xs[i] }) {
                let mut t = cur.take().unwrap();
                t.exit_i = i; t.exit_px = c[i]; t.signal_i = i; t.reason = (if long { why_l[i] } else { why_s[i] }).into();
                out.push(t);
            }
        }
        if cur.is_none() {
            if el[i] { cur = Some(trade(1., i, c[i])); } else if es[i] { cur = Some(trade(-1., i, c[i])); }
        }
    }
    if let Some(mut t) = cur { t.exit_i = to; t.exit_px = c[to]; t.signal_i = to; t.reason = "open".into(); t.open = true; out.push(t); }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn metastock_cross_and_ema() {
        assert!(cross(&[1., 2.], &[1., 1.], 1) && !cross(&[2., 2.], &[1., 1.], 1) && !cross(&[1., 1.], &[1., 1.], 1));
        let e = ema_ms(&[f64::NAN, 2., 4.], 3);
        assert!(e[0].is_nan() && e[1] == 2. && (e[2] - 3.).abs() < 1e-12);
    }
}
