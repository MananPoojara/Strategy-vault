//! Rayner Teo, "The Ultimate Guide to Trend Following" (TradingwithRayner e-book; research share Prasad/ALL BOOKS/The-ultimate-
//! guide-to-Trend-Following.pdf, PDF p.34). Book id `rtutf`. One rule: the 'Systematic Trend Following Strategy', Rayner's summary
//! of Andreas Clenow's rule in Following the Trend (that book is not in the research share). Lane K8 (titles loop, 1 Nov 2026);
//! independent check authors/titles-1030/K8w/k8_ref.py (header).
use crate::connors::{atr, Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::mkits::ema;
use super::rbtie::mk;

/// Admitted ids (authors/titles-1030/K8w/k8_ref.py (header)).
pub const REVIEWED: &[&str] = &["rtutf_clenow"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] Source: Rayner Teo's four-line summary of Andreas Clenow's rule (Clenow is the rule's originator; his book is not in the research share, so this card follows Rayner's wording). Exponential averages use 2/(n+1) and start from the simple average of the first n closes. 'Price close above the 50 day high' = today's close above the highest high of the 50 days before today. 'A stoploss of 3 ATR away from its peak reading' = the highest close since entry minus 3 x ATR (ATR period not printed: Wilder's 14 days is the labelled reading, 20 and 100 are options), checked on the close and filled at the next open. Shorts mirrored ('Vice versa for shorts': 50 EMA below 100 EMA, close below the lowest low of the previous 50 days, stop = lowest close since entry + 3 x ATR); long only is an option. One position at a time; no profit target. Diversified futures in the source; ETFs, US/India indices and stocks are a declared application. No printed results. Rayner's own strategy on p.28 is held (a 'test' of the moving-average zone is not defined).";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "rtutf_clenow", name: "Systematic trend following: 50 EMA over 100 EMA and a close above the 50-day high, 3-ATR trailing stop from the peak close (short mirror)", group: "Teo Ultimate Guide to Trend Following",
        session: 0, dir: "Long/Short", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: "rtutf", rules: &[
            "F|\"If 50 ema is above 100 ema, then look to long\" (p.34)",
            "E|\"wait for price to close above the 50 day high. If price closes above the 50 day high, then enter your trade at the next open\" (p.34)",
            "X|\"have a stoploss of 3 ATR away from its peak reading. Vice versa for shorts\" (p.34)"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    (id == "rtutf_clenow").then_some(("rtutf_clenow", "Clenow systematic trend following (Teo)", "EMA 50/100, 50-day high, 3 ATR"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    (id == "rtutf_clenow").then(|| vec![p("atr", "ATR days (not printed; 14 = Wilder's default, 20 and 100 options)", 14., 5., 100., 1.),
        p("mult", "ATR multiple (3)", 3., 1., 6., 0.5), p("side", "1 = long only (default 0: long and short)", 0., 0., 1., 1.)])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(f64::NAN, |d| d.def));
    let (e50, e100, a) = (ema(&x.c, 50), ema(&x.c, 100), atr(x.r, g("atr").round().max(1.) as usize));
    let (mult, long_only) = (g("mult"), g("side") >= 1.);
    let (h, l, c) = (&x.h, &x.l, &x.c);
    // (dir, entry bar, entry price, peak close)
    let (mut out, mut pos, mut pend_in, mut pend_out) = (vec![], None::<(f64, usize, f64, f64)>, None::<f64>, false);
    for i in from..=to {
        if pend_out { if let Some((d, e, ep, _)) = pos.take() { out.push(mk(d, e, ep, "open", i, x.o[i], "3-ATR stop from the peak close", false, i - 1)); } }
        if let Some(d) = pend_in { pos = Some((d, i, x.o[i], c[i])); }
        (pend_in, pend_out) = (None, false);
        if let Some((d, e, ep, pk)) = pos {
            let pk = if d > 0. { pk.max(c[i]) } else { pk.min(c[i]) };
            pos = Some((d, e, ep, pk));
            pend_out = i < to && if d > 0. { c[i] < pk - mult * a[i] } else { c[i] > pk + mult * a[i] };
        } else if i < to && i >= 50 {
            let hh = h[i - 50..i].iter().copied().fold(f64::NEG_INFINITY, f64::max);
            let ll = l[i - 50..i].iter().copied().fold(f64::INFINITY, f64::min);
            if e50[i] > e100[i] && c[i] > hh { pend_in = Some(1.); } else if !long_only && e50[i] < e100[i] && c[i] < ll { pend_in = Some(-1.); }
        }
    }
    if let Some((d, e, ep, _)) = pos { out.push(mk(d, e, ep, "open", to, c[to], "open", true, to)); }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn breakout_then_trailing_stop() {
        let mut c: Vec<f64> = (0..200).map(|i| 100. + i as f64 * 0.3).collect();
        c.extend([150., 140., 130.]);
        let r: Vec<Row> = c.iter().enumerate().map(|(i, &v)| Row { ticker: "T".into(), date: format!("2001-{:02}-{:02}", 1 + i / 20, 1 + i % 20), open: v, high: v + 0.2, low: v - 0.2, close: v, volume: None }).collect();
        let x = Ctx::new(&r, &r, &[]);
        let t = run("rtutf_clenow", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert_eq!(t.len(), 1);
        // first bar with both EMAs defined (bar 99) is a breakout: long at bar 100's open; the drop at bar 200 exits at bar 201's open
        assert_eq!((t[0].dir, t[0].legs[0].i, t[0].exit_i, t[0].exit_px), (1., 100, 201, 140.));
    }
}

// Instrument admission: Rayner Teo (Clenow's rule) 2026-11-01 (titles lane K8).
// Admission only for the module REVIEWED lists (authors/titles-1030/K8w/k8_ref.py (header)).
// Runs on actual indices, ETFs and stocks, US and India (index_etf_or_stock, nifty50 basket, declared application).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "rtutf" && crate::books::rtutf::reviewed(def.id),
    label: "A futures trend-following rule summarised in an e-book with no results; ETFs, US/India indices and stocks are a declared application.",
    basis: "Rayner Teo, The Ultimate Guide to Trend Following, PDF p.34 (summary of Clenow, Following the Trend); independent check authors/titles-1030/K8w/k8_ref.py",
};
