//! Peter Pontikis, CMC Markets Education "Technical Analysis" course notes (research share USB HDD 2/tRADING KNO-2/Peter Pontikis -
//! Technical Analysis (cmcmarkets.com.au)/pdfs/strategy_course_notes.pdf p.44-45 and 55-57, mechanics_course_notes.pdf RSI section;
//! text authors/batch-e/E10w/pontikis.txt). Book id `pptca`. Two teaching examples (R4 decisions 49, 56): percentage-band fades on a
//! 40-week average, and RSI(14) 30/70 signals with the ADX as an off switch. Lane K11 (titles loop, 1 Nov 2026), to-code row 179;
//! independent check authors/titles-1030/K11w/k11_ref.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::lc3::bars;
use super::msws::{dmi, rsi, sma};
use super::tselhs::Book;

/// Admitted ids (authors/titles-1030/K11w/k11_ref.py (header)).
pub const REVIEWED: &[&str] = &["pptca_band40w", "pptca_rsi_adx"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

macro_rules! common { () => { " Long and short, always in after the first signal (a buy closes a short and goes long, a sell closes a long and goes short); p.long = 1 trades the long side only. Fill at the signal close (the course uses the closing price as the trigger, footnote 9). No stop or target: the course places stops by eye at support and resistance. Teaching examples from a broker's education course (CMC Markets), shown on charts with no results (lead decision 49); indices, ETFs and US/India stocks are a declared application; the vault's standard costs." } }

const BAND: &str = concat!("[Declared readings] Percentage bands (strategy notes p.44-45, Figure 5.1 'Dow Jones, with a 40 week average with 10% bands'): weekly bars (Monday-based calendar weeks, each week's last trading day; the last week of a run may be partial), band = the 40-week simple average of weekly closes x 1.10 and x 0.90. 'Buy signals occur when price drops below the lower band level and then crosses back above it. Conversely Sell signals occur when the price moves over the upper band level and then crosses back below it': a weekly close above the lower band after a close at or below it buys; a weekly close below the upper band after a close at or above it sells. The text says 'average' (the software offers simple or exponential): simple is the reading. The author says no band width is best (3% short-term, 10-20% long-term): p.band = 3 or 20 are options.", common!());
const RSIADX: &str = concat!("[Declared readings] RSI with the ADX as a filter (strategy notes p.55-57, mechanics notes RSI section): RSI(14), Wilder ('Normally the x is set at 14 days'); 'Buy signals occur when the RSI reading drops below the 30 RSI level and then crosses back above it. Conversely Sell signals occur when the RSI reading moves over the 70 RSI level and then crosses back below it' = RSI above 30 today after at or below 30 yesterday (buy), below 70 after at or above 70 (sell). 'When the ADX is rising, we ignore any signals that the RSI may generate ... When the ADX is quiet or declining we then accept the RSI signals': a signal is ignored when ADX today > ADX yesterday, and also before the ADX has two values. ADX(14) = Wilder's published definition (the course cites Wilder 1978; lead decision 56).", common!());

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, rules, notes| StrategyDef { id, name, group: "CMC Markets Technical Analysis course (Pontikis)", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "pptca", rules, notes, claim: None };
    vec![
        d("pptca_band40w", "Weekly 40-week average with 10% bands: buy a close back above the lower band, sell a close back below the upper band", &[
            "F|\"Figure 5.1 Dow Jones, with a 40 week average with 10% bands applied\" (strategy notes p.45)",
            "E|\"Buy signals occur when price drops below the lower band level and then crosses back above it. Conversely Sell signals occur when the price moves over the upper band level and then crosses back below it.\" (p.45)",
            "X|Always in: a sell signal closes the long and sells short, a buy signal covers and buys; no stop or target (stops are placed by eye in the course)."], BAND),
        d("pptca_rsi_adx", "RSI(14) leaving 30 buys, leaving 70 sells, signals ignored while the ADX(14) is rising", &[
            "E|\"Buy signals occur when the RSI reading drops below the 30 RSI level and then crosses back above it. Conversely Sell signals occur when the RSI reading moves over the 70 RSI level and then crosses back below it.\" (mechanics notes p.26)",
            "F|\"when the ADX is rising, we 'ignore' any signals that the RSI may generate ... When the ADX is quiet or declining we then accept the RSI signals\" (strategy notes p.57)",
            "X|Always in: an accepted sell signal closes the long and sells short, an accepted buy signal covers and buys; no stop or target."], RSIADX),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let f = match id {
        "pptca_band40w" => ("Pontikis percentage bands", "40-week, 10%"),
        "pptca_rsi_adx" => ("Pontikis RSI with ADX filter", "RSI 14, ADX 14"),
        _ => return None,
    };
    let id = REVIEWED.iter().find(|r| **r == id)?;
    Some((*id, f.0, f.1))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    let long = p("long", "Sides: 0 = long and short (default), 1 = long only", 0., 0., 1., 1.);
    Some(match id {
        "pptca_band40w" => vec![p("band", "Band width, % (10; the course also names 3 and 20)", 10., 1., 30., 1.), long],
        "pptca_rsi_adx" => vec![long],
        _ => return None,
    })
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let long_only = p.get("long").copied().unwrap_or(0.) > 0.5;
    let mut pos = Book::new();
    let act = |pos: &mut Book, d: f64, i: usize, px: f64| {
        if d > 0. { pos.enter(1., i, px, "close"); } else if long_only { if pos.side() > 0. { pos.exit(i, px, "sell signal"); } } else { pos.enter(-1., i, px, "close"); }
    };
    match id {
        "pptca_band40w" => {
            let k = p.get("band").copied().unwrap_or(10.) / 100.;
            let w = bars(x, to, true);
            let wc: Vec<f64> = w.iter().map(|b| b.5).collect();
            let m = sma(&wc, 40);
            let (lb, ub): (Vec<f64>, Vec<f64>) = (m.iter().map(|v| v * (1. - k)).collect(), m.iter().map(|v| v * (1. + k)).collect());
            for j in 1..w.len() {
                let i = w[j].1;
                if i < from { continue; }
                if wc[j] > lb[j] && wc[j - 1] <= lb[j - 1] { act(&mut pos, 1., i, x.c[i]); }
                else if wc[j] < ub[j] && wc[j - 1] >= ub[j - 1] { act(&mut pos, -1., i, x.c[i]); }
            }
        }
        "pptca_rsi_adx" => {
            let r = rsi(&x.c, 14);
            let (_, _, adx) = dmi(x, 14);
            for i in from.max(1)..=to {
                if !(adx[i].is_finite() && adx[i - 1].is_finite()) || adx[i] > adx[i - 1] { continue; }
                if r[i] > 30. && r[i - 1] <= 30. { act(&mut pos, 1., i, x.c[i]); }
                else if r[i] < 70. && r[i - 1] >= 70. { act(&mut pos, -1., i, x.c[i]); }
            }
        }
        _ => return None,
    }
    Some(pos.done(to, x.c[to]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn pontikis_examples() {
        let r: Vec<Row> = (0..4000).map(|i| { let f = i as f64; let c = 100. + 25. * (f / 150.).sin() + 6. * (f / 9.).sin();
            let date = format!("{:04}-{:02}-{:02}", 2000 + i / 336, i % 336 / 28 + 1, i % 28 + 1);
            Row { ticker: "T".into(), date, open: c, high: c + 1.5, low: c - 1.5, close: c, volume: None } }).collect();
        let x = Ctx::new(&r, &r, &[]);
        for id in REVIEWED {
            let t = run(id, &x, 0, r.len() - 1, &Params::new()).unwrap();
            assert!(!t.is_empty(), "{id}");
            for z in &t { assert_eq!(z.legs[0].px, x.c[z.legs[0].i]); }
            let lo = run(id, &x, 0, r.len() - 1, &Params::from([("long".to_string(), 1.)])).unwrap();
            assert!(lo.iter().all(|z| z.dir > 0.), "{id}");
            assert!(family_of(id).is_some() && params(id).is_some());
        }
        assert!(run("pptca_x", &x, 0, 5, &Params::new()).is_none());
    }
}

// Instrument admission: Peter Pontikis, CMC Markets Technical Analysis course 2026-11-01 (titles lane K11).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "pptca" && reviewed(def.id),
    label: "Teaching examples from Peter Pontikis's CMC Markets course, shown on the Dow Jones, sterling and Canadian dollar charts with no results. Runs on indices, ETFs and stocks, US and India, are a declared application.",
    basis: "CMC Markets Technical Analysis course: strategy notes p.44-45 (percentage bands) and p.55-57 (ADX as a filter for the RSI), mechanics notes RSI section; independent check authors/titles-1030/K11w/k11_ref.py",
};
