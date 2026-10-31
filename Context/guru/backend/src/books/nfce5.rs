//! Nelson Freeburg, "CD Handouts" (Chicago 2005 Traders Expo presentation CD; research drive USB HDD 2/tRADING KNO-2/
//! Chicago 2005 Traders Expo Presentation CD/AutoPlay/Docs/CD Handouts-Nelson Freeburg.pdf). Book id `nfce5`. Two weekly
//! S&P 500 timing models, rules printed on PDF p.34 (Allen) and p.39 (Traub); results on the image slides p.30 and p.36.
//! Lane K2 (titles loop, 30 Oct 2026); independent check authors/niksan-k2/admission/k2b_ref.py (header).
use crate::books::swsp::week_ends;
use crate::connors::{sma, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

/// Admitted ids (authors/niksan-k2/admission/k2b_ref.py (header)).
pub const REVIEWED: &[&str] = &["nfce5_allen", "nfce5_traub"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] Weekly bars: a week's close is its last trading day. The handout gives no fill: signals and fills at the weekly close. Long or in cash; start in cash at the window start and buy at the first weekly close that meets the buy rule. Averages are simple averages of weekly closes. Book instrument: the S&P 500 index; ETFs, other indices and single stocks are a declared application.";

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id: &'static str, name: &'static str, rules: &'static [&'static str], claim| StrategyDef { id, name, group: "Freeburg Chicago 2005 handout",
        session: 0, dir: "Long", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: "nfce5", rules, notes: NOTE, claim };
    vec![
        d("nfce5_allen", "Allen Stock Market Timing Model: in while the S&P is above its 20- or 40-week average, out below both", &[
            "E|\"Buy When S&P 500 > 20-Week SMA OR S&P 500 > 40-Week SMA\" (PDF p.34)",
            "X|\"Sell When S&P 500 < 20-Week SMA AND S&P < 40-Week SMA\" (PDF p.34)"],
            crate::connors::claim("75 (30 winners)", "40%", "CAR 11.2%; $10,000 -> $425,100; max drawdown 41.6%; 72% invested", "", "S&P 500, 1/2/1970-6/3/2005 (PDF p.30)")),
        d("nfce5_traub", "Traub Timing Model: S&P above the 5-week average of its 50-week average and above its 9-week high", &[
            "F|\"Calculate 50-Week SMA of S&P 500\" (PDF p.39)",
            "E|\"Buy When S&P > 5-Week SMA of 50-Week SMA AND S&P > Highest Close of Past 9 Weeks\" (PDF p.39); [reading] past 9 weeks = the 9 weeks before this one",
            "X|\"Sell When S&P 500 < 5-Week SMA of 50-Week SMA AND S&P < Lowest Close of Past 9 Weeks\" (PDF p.39)"],
            crate::connors::claim("", "", "$10,000 -> $522,500 (S&P 500 buy and hold $406,000)", "", "S&P 500, Jan 1970-Jun 2005 (PDF p.36)")),
    ]
}
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match id { "nfce5_allen" => Some(("nfce5_allen", "Allen timing model", "20 / 40 weeks")), "nfce5_traub" => Some(("nfce5_traub", "Traub timing model", "5 of 50 weeks, 9-week channel")), _ => None }
}
pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max| ParamDef { key, label, def, min, max, step: 1. };
    match id {
        "nfce5_allen" => Some(vec![p("fast", "Shorter average, weeks (handout 20)", 20., 2., 100.), p("slow", "Longer average, weeks (handout 40)", 40., 3., 200.)]),
        "nfce5_traub" => Some(vec![p("base", "Base average, weeks (handout 50)", 50., 2., 200.), p("smooth", "Average of the base average, weeks (handout 5)", 5., 1., 50.), p("chan", "Channel, past weeks (handout 9)", 9., 1., 52.)]),
        _ => None,
    }
}
pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let s = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| s.iter().find(|d| d.key == k).unwrap().def).round().max(1.) as usize;
    let we: Vec<usize> = week_ends(x).into_iter().filter(|&i| i <= to).collect();
    let wc: Vec<f64> = we.iter().map(|&i| x.c[i]).collect();
    let n = wc.len();
    // (buy, sell) per week; undefined weeks give neither
    let sig: Vec<(bool, bool)> = if id == "nfce5_allen" {
        let (a, b) = (sma(&wc, g("fast")), sma(&wc, g("slow")));
        (0..n).map(|k| if a[k].is_finite() && b[k].is_finite() { (wc[k] > a[k] || wc[k] > b[k], wc[k] < a[k] && wc[k] < b[k]) } else { (false, false) }).collect()
    } else {
        let (m, ch) = (sma(&sma(&wc, g("base")), g("smooth")), g("chan"));
        (0..n).map(|k| if k >= ch && m[k].is_finite() {
            let w = &wc[k - ch..k];
            let (hi, lo) = (w.iter().copied().fold(f64::MIN, f64::max), w.iter().copied().fold(f64::MAX, f64::min));
            (wc[k] > m[k] && wc[k] > hi, wc[k] < m[k] && wc[k] < lo)
        } else { (false, false) }).collect()
    };
    let c = &x.c;
    let mk = |e: usize, xi: usize, why: &str, open: bool| Trade { dir: 1., legs: vec![Leg { i: e, px: c[e], w: 1. }], exit_i: xi, exit_px: c[xi], reason: why.into(), entry_kind: "close", signal_i: xi, open, exits: vec![] };
    let (mut out, mut cur) = (vec![], None::<usize>);
    for k in 0..n {
        let i = we[k];
        if let Some(e) = cur { if i > e && sig[k].1 { out.push(mk(e, i, "sell rule · weekly close", false)); cur = None; continue; } }
        if cur.is_none() && i >= from && i < to && sig[k].0 { cur = Some(i); }
    }
    if let Some(e) = cur { out.push(mk(e, to, "open", true)); }
    Some(out)
}

// Instrument admission: Nelson Freeburg (Chicago 2005 handout) 2026-10-30 (titles lane K2) block, moved from instrument_policy.rs on 31 Oct 2026 (lane ST).
// Admission only for the module REVIEWED lists (authors/niksan-k2/admission/k2b_ref.py (header)); kept in one block to merge.
// Runs on actual indices, ETFs and stocks, US and India (index_etf_or_stock, nifty50 basket, declared application).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book=="nfce5" && crate::books::nfce5::reviewed(def.id),
    label: "Nelson Freeburg presents these timing models on the S&P 500 index; ETFs, other indices and single stocks (US and India) are a declared application, not the author's test.",
    basis: "Nelson Freeburg, CD Handouts, Chicago 2005 Traders Expo, PDF p.30-39",
};
