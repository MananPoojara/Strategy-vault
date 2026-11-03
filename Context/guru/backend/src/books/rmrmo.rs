//! Rahul Mohindar (Viratech India), RMO Trade Model - 'Trade with the Trend' (Mastering MetaStock 10, ch.6, printed
//! p.53-56; research share USB HDD 1/MetaStock/@Systems & Indicators/Rahul Mohindar Oscillator (RMO) Indicators for
//! Metastock/MasteringMetaStock10_CH6-RMO.pdf) with the formulas of the MetaStock pack in that folder (MS90FORM.DTA,
//! EXPERTS/EXPT0001.DTA). Book id `rmrmo`. Lane K6 (titles loop, 31 Oct 2026); independent check
//! authors/titles-1030/K6w/k6_ref.py (header).
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::mkits::{ema, hhv, llv};
use super::rbtie::mk;
use super::svtvc::sma_exact as sma;

/// Admitted ids (authors/titles-1030/K6w/k6_ref.py (header)).
pub const REVIEWED: &[&str] = &["rmrmo_trend"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] Formulas as in MS90FORM.DTA: SwingTrd1 = 100 x (C - the average of the ten nested 2-day simple averages of C)/(HHV(C,10) - LLV(C,10)) (no value when the 10-day range is 0); SwingTrd2 = Mov(SwingTrd1,30,E), SwingTrd3 = Mov(SwingTrd2,30,E), RMO = Mov(SwingTrd1,81,E), MetaStock averages seeded with the simple average of their first values. Expert: blue bar = SwingTrd2 > 0, red bar = SwingTrd2 < 0; buy arrow = Cross(SwingTrd2, SwingTrd3), sell arrow = Cross(SwingTrd3, SwingTrd2). 'Trade with the Trend' (p.55-56): a buy arrow on a blue bar with RMO > 0 sets a buy stop at that bar's high for the next bar only (filled at the stop, or at the open if the open is already above it; 'buy only above the High' = the next high must exceed it); the stop loss is the low of the arrow bar (the 'previous bar's low' to the entry bar), checked from the bar after entry and filled at the stop or a lower open; exit at the next open after a bar that has both a sell arrow and a red bar. Shorts mirrored (sell stop below the arrow bar's low, stop at its high; out after a buy arrow on a blue bar); option: long only. Not coded: the Exit Swing Indicator (not in the formula file) and the discretionary 'support level' stop. Any timeframe per the author; daily bars here. Indian author; NSE stocks and NIFTY are a declared application like the US instruments (the chapter shows no test).";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "rmrmo_trend", name: "RMO Trade with the Trend: buy arrow on a blue bar with RMO above 0, stop at the arrow bar, out on red bar + red arrow", group: "Mohindar RMO Trade Model",
        session: 0, dir: "Long/Short", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: "rmrmo", rules: &[
            "F|SwingTrd 1: \"100 * (CLOSE - ((Mov(C,2,S) + Mov(Mov(C,2,S),2,S) + ... [10 nested]) / 10)) / (HHV(C,10) - LLV(C,10))\"; SwingTrd 2 = Mov(SwingTrd 1,30,E); SwingTrd 3 = Mov(SwingTrd 2,30,E); RMO = \"Mov(Fml(\"Swingtrd 1\"),81,E)\" (MS90FORM.DTA)",
            "E|Buy arrow \"Cross( Fml( \"swingTrd 2\"),Fml( \"swingTrd 3\") )\" on a blue bar \"(Mov(Fml(\"SwingTrd 1\"),30,E)) > 0\" with RMO > 0; \"buy only above the High of the signal/arrow bar, and be sure to set a stop loss point - either at the previous bar's low\" (p.55-56)",
            "X|\"exit long either by seeing a combination of red bars and a red arrow ... or by getting stopped out\" (p.56)",
            "E|\"Likewise, sell only below the low of a sell arrow bar, stop above the previous bar's high\" (p.56)"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    (id == "rmrmo_trend").then_some(("rmrmo_trend", "Mohindar RMO trade with the trend", "SwingTrd 30/30, RMO 81"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    (id == "rmrmo_trend").then(|| vec![ParamDef { key: "side", label: "0 = long and short (book), 1 = long only", def: 0., min: 0., max: 1., step: 1. }])
}

/// (SwingTrd2, SwingTrd3, RMO).
pub fn rmo(c: &[f64]) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
    let mut m = sma(c, 2);
    let mut sum = m.clone();
    for _ in 1..10 { m = sma(&m, 2); for (s, v) in sum.iter_mut().zip(&m) { *s += v; } }
    let (hh, ll) = (hhv(c, 10), llv(c, 10));
    let st1: Vec<f64> = (0..c.len()).map(|i| { let d = hh[i] - ll[i]; if d == 0. || !d.is_finite() { f64::NAN } else { 100. * (c[i] - sum[i] / 10.) / d } }).collect();
    let st2 = ema(&st1, 30);
    let st3 = ema(&st2, 30);
    (st2, st3, ema(&st1, 81))
}

fn cross(a: &[f64], b: &[f64], i: usize) -> bool { i > 0 && a[i] > b[i] && a[i - 1] <= b[i - 1] }

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let long_only = p.get("side").copied().unwrap_or(0.).round() >= 1.;
    let (o, h, l) = (&x.o, &x.h, &x.l);
    let (s2, s3, r) = rmo(&x.c);
    let up = |i: usize| cross(&s2, &s3, i) && s2[i] > 0.;   // buy arrow on a blue bar
    let dn = |i: usize| cross(&s3, &s2, i) && s2[i] < 0.;   // sell arrow on a red bar
    let mut out = vec![];
    let mut pos: Option<(f64, usize, f64, f64)> = None; // (dir, entry bar, entry price, stop)
    let mut pend: Option<(f64, f64, f64)> = None;       // (dir, trigger, stop) for this bar only
    let mut exit_open = false;
    for i in from..=to {
        if let Some((d, e, ep, st)) = pos {
            if exit_open {
                out.push(mk(d, e, ep, "stop", i, o[i], "red arrow on a red bar", false, i - 1)); pos = None; exit_open = false;
            } else if i > e && (if d > 0. { l[i] < st } else { h[i] > st }) {
                let px = if d > 0. { o[i].min(st) } else { o[i].max(st) };
                out.push(mk(d, e, ep, "stop", i, px, "stop loss", false, i)); pos = None;
            }
        } else if let Some((d, tr, st)) = pend.take() {
            if d > 0. && h[i] > tr { pos = Some((d, i, o[i].max(tr), st)); }
            if d < 0. && l[i] < tr { pos = Some((d, i, o[i].min(tr), st)); }
        }
        pend = None;
        match pos {
            Some((d, ..)) => exit_open = i < to && (if d > 0. { dn(i) } else { up(i) }),
            None if i < to => {
                if up(i) && r[i] > 0. { pend = Some((1., h[i], l[i])); } else if !long_only && dn(i) && r[i] < 0. { pend = Some((-1., l[i], h[i])); }
            }
            None => {}
        }
    }
    if let Some((d, e, ep, _)) = pos { out.push(mk(d, e, ep, "stop", to, x.c[to], "open", true, to)); }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn swingtrd_formula_and_trades_follow_the_setup() {
        let c: Vec<f64> = (0..300).map(|i| 100. + 10. * (i as f64 / 15.).sin() + i as f64 * 0.05).collect();
        let (s2, s3, r) = rmo(&c);
        // SwingTrd1 needs 10 nested averages (first at bar 10) and HHV/LLV(10); SwingTrd2 = EMA30 seeded on bars 10-39
        assert!(s2[38].is_nan() && s2[39].is_finite() && s3[68].is_finite() && r[90].is_finite());
        let rows: Vec<Row> = c.iter().enumerate().map(|(i, &v)| Row { ticker: "T".into(), date: format!("{}", i), open: v, high: v + 1., low: v - 1., close: v, volume: None }).collect();
        let x = Ctx::new(&rows, &rows, &[]);
        let t = run("rmrmo_trend", &x, 0, rows.len() - 1, &Params::new()).unwrap();
        for z in &t { let (a, b) = if z.dir > 0. { (&s2, &s3) } else { (&s3, &s2) }; assert!(cross(a, b, z.legs[0].i - 1)); }
        assert!(run("rmrmo_x", &x, 0, 5, &Params::new()).is_none());
    }
}

// Instrument admission: Rahul Mohindar (RMO Trade Model) 2026-10-31 (titles lane K6).
// Admission only for the module REVIEWED lists (authors/titles-1030/K6w/k6_ref.py (header)).
// Runs on actual indices, ETFs and stocks, US and India (index_etf_or_stock, nifty50 basket, declared application).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "rmrmo" && crate::books::rmrmo::reviewed(def.id),
    label: "Rahul Mohindar's MetaStock trade model for any security and timeframe, with no printed test; indices, ETFs and US/India stocks are a declared application.",
    basis: "Rahul Mohindar, RMO Trade Model, Mastering MetaStock 10 ch.6 p.53-56; formulas MS90FORM.DTA and expert EXPT0001.DTA in the same folder",
};
