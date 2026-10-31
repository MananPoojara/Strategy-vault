//! Buff Dormeier, "Buff Up Your Moving Averages" (Las Vegas 2005 Traders Expo handout; research share USB HDD 2/tRADING KNO-2/
//! Las Vegas 2005 Traders Expo Presentation CD/AutoPlay/Docs/Dormeier Handout.pdf, p.9 VWMA, p.12 test, p.14-18 results; text
//! authors/batch-c/C6w/dormeier-vwma.txt). Book id `bdvwma`. Lane K10 (titles loop, 1 Nov 2026), to-code row 163; independent
//! check authors/titles-1030/K10w/k10_ref.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::lc3::Pos;
use super::msws::sma;

/// Admitted ids (authors/titles-1030/K10w/k10_ref.py (header)).
pub const REVIEWED: &[&str] = &["bdvwma_cross"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] '5/20day crossing system w/long & short positions' = the 5-day average crossing the 20-day average of the same type, stop and reverse: long when the 5-day crosses above the 20-day (above today, at or below yesterday), short when it crosses below. Default = Dormeier's volume-weighted averages, VWMA(n) = sum(close x volume, n) / sum(volume, n) (p.9 worked example: 10 on 100,000 shares and 12 on 300,000 shares give 11.5); p.kind = 0 runs his control test with simple averages. The first position opens on the first cross inside the test window. Fills at the signal day's close (option: next day's open). Needs daily volume, so indices without volume are refused. The VPCI-confirmed version (p.30-33) is held: its periods and how VPCI confirms the crossover are not stated. His test: 60 US stocks in 12 size/volume/volatility groups, 2000 trading days (VWMA reliability 60% vs SMA 28%, average drawdown 16 vs 44); the 1999/2004 stock lists are not the vault's baskets, so no result is compared. Indices with volume, ETFs and US/India stocks are a declared application.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "bdvwma_cross", name: "Buff Up Your Moving Averages: 5/20-day volume-weighted average crossover, long and short", group: "Volume-weighted averages (Traders Expo 2005)",
        session: 0, dir: "Long/Short", status: Status::Live, needs_volume: true, needs_vix: false, needs_series: &[], book: "bdvwma", rules: &[
            "F|\"Volume weighted average = Sum {Closing Price (I) * [Volume (I)/(total range)]; I = given days action}\" ... \"VWMA = 11.5 [(10*100/400) + (12*300/400)]\" (p.9)",
            "E|\"Test: 2000 Trading days (approx 7 1/2 years) 5/20day crossing system w/long & short positions ... 1st group used SMA, 2nd group used VWMA\" (p.12)",
            "X|stop and reverse on the opposite 5/20 cross (long & short positions)"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    (id == "bdvwma_cross").then_some(("bdvwma_cross", "Dormeier VWMA crossover", "5/20"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    reviewed(id).then(|| vec![p("kind", "Averages: 1 = volume-weighted (default), 0 = simple (his control test)", 1., 0., 1., 1.),
        p("fast", "Fast average (5)", 5., 2., 50., 1.), p("slow", "Slow average (20)", 20., 5., 200., 1.),
        p("open", "Fills: 0 = signal day's close (default), 1 = next day's open", 0., 0., 1., 1.)])
}

fn vwma(x: &Ctx, n: usize) -> Vec<f64> {
    let v: Vec<f64> = x.r.iter().map(|r| r.volume.filter(|v| v.is_finite() && *v > 0.).unwrap_or(f64::NAN)).collect();
    let cv: Vec<f64> = (0..v.len()).map(|i| x.c[i] * v[i]).collect();
    let (a, b) = (sma(&cv, n), sma(&v, n));
    (0..v.len()).map(|i| if b[i] > 0. { a[i] / b[i] } else { f64::NAN }).collect()
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).unwrap().def);
    let (nf, ns) = (g("fast").round().max(1.) as usize, g("slow").round().max(1.) as usize);
    let (f, s) = if g("kind") > 0.5 { (vwma(x, nf), vwma(x, ns)) } else { (sma(&x.c, nf), sma(&x.c, ns)) };
    let nxt = g("open") > 0.5;
    let mut pos = Pos::new();
    for i in from.max(1)..=to {
        if nxt && i == to { break; }
        let ok = [f[i], s[i], f[i - 1], s[i - 1]].iter().all(|v| v.is_finite());
        let d = if ok && f[i] > s[i] && f[i - 1] <= s[i - 1] { 1. } else if ok && f[i] < s[i] && f[i - 1] >= s[i - 1] { -1. } else { 0. };
        if d == 0. { continue; }
        if nxt { pos.enter(d, i + 1, x.o[i + 1], "open"); } else { pos.enter(d, i, x.c[i], "close"); }
    }
    Some(pos.done(to, x.c[to]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn vwma_example_and_stop_and_reverse() {
        let r: Vec<Row> = (0..400).map(|i| { let f = i as f64; let c = 100. + 10. * (f / 15.).sin();
            Row { ticker: "T".into(), date: format!("{:05}", i), open: c, high: c + 1., low: c - 1., close: c, volume: Some(1000. + 500. * (f / 7.).cos().abs()) } }).collect();
        let mut two = r[..2].to_vec(); two[0].close = 10.; two[0].volume = Some(1e5); two[1].close = 12.; two[1].volume = Some(3e5);
        let x2 = Ctx::new(&two, &two, &[]);
        assert!((vwma(&x2, 2)[1] - 11.5).abs() < 1e-12);
        let x = Ctx::new(&r, &r, &[]);
        let t = run("bdvwma_cross", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(t.len() > 4);
        for w in t.windows(2) { assert_eq!(w[0].exit_i, w[1].legs[0].i); assert!(w[0].dir != w[1].dir); }
        assert!(run("bdvwma_x", &x, 0, 5, &Params::new()).is_none());
    }
}

// Instrument admission: Buff Dormeier, Buff Up Your Moving Averages 2026-11-01 (titles lane K10).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "bdvwma" && reviewed(def.id),
    label: "Buff Dormeier tested the 5/20-day crossover on 60 US stocks (2000 trading days). Runs on indices with volume, ETFs and stocks, US and India, are a declared application.",
    basis: "Buff Dormeier, Buff Up Your Moving Averages, Las Vegas 2005 Traders Expo handout p.9, 12, 14-18 (Dormeier Handout.pdf); independent check authors/titles-1030/K10w/k10_ref.py",
};
