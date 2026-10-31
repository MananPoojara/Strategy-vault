//! Chuck Dukas, "The TRENDadvisor Guide to Breakthrough Profits" (Wiley, 2006; research share USB HDD 2/tRADING KNO-2/Chuck
//! Dukas - The TrendAdvisor Guide to Breakthrough Profits/). Book id `cdtag`. One rule: buying strength in the Bullish Phase
//! (p.97-98) and its mirror, selling weakness in the Bearish Phase (p.163-164), out when the phase changes. Lane K8 (titles
//! loop, 1 Nov 2026); independent check authors/titles-1030/K8w/k8_ref.py (header).
use crate::connors::{sma, Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::mkits::{hhv, llv};
use super::rbtie::mk;

/// Admitted ids (authors/titles-1030/K8w/k8_ref.py (header)).
pub const REVIEWED: &[&str] = &["cdtag_phase"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] Toolkit settings from ch.1: 50- and 200-period simple averages of the close, 60-period high/low channel, 21-period Rate of Change in percent, slow stochastic = the 3-bar average of 100 x (close - 21-bar lowest low) / (21-bar highest high - 21-bar lowest low), volume oscillator = 5-period average of volume minus the 21-period average. 'Breaking out to new 60 period highs' = today's high above the highest high of the previous 60 bars; 'sloping up' = above yesterday's value (the book: 'the last period is greater than the previous period'); 'accelerating' Rate of Change = above yesterday's value; stochastic 'rising' = above yesterday's. The volume test is applied only on series that carry volume (a bar without volume then fails it). Buy stop at the setup day's high, valid the next day only: the book's '1/8 to 3/8 above yesterday's high' is a stock-price fraction 'based on price and volatility', so the default offset is 0 and a percentage offset is an option; a gap above the stop fills at the open. Exit at the close on the first day the stock is no longer in the Bullish Phase ('Sell remaining position when the phase changes'; the initial stop 'a close below the 50 PMA' is one such change), checked from the entry day. The half-position target is not coded: it is 'based on your trading plan' (no number), so the trailing stop at entry that follows it has no trigger either. Shorts mirror in the Bearish Phase (p.163-164); long only is an option. The market/sector filters (MSE) and 'sell at less than target when price stalls' are judgement. The book trades US stocks; ETFs, US/India indices and India stocks are a declared application. No printed results.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "cdtag_phase", name: "TRENDadvisor phase breakout: buy strength in the Bullish Phase on a stop above yesterday's high, out when the phase changes (short mirror)", group: "Dukas TRENDadvisor phases",
        session: 0, dir: "Long/Short", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: "cdtag", rules: &[
            "F|Bullish Phase: \"Price closes are above the 50 period moving average. Price closes are above the 200 period moving average. 50 period moving average is above the 200 period moving average\" (p.98)",
            "E|\"Price is breaking out to new 60 period highs. The 50 and 200 moving averages are sloping up ... The Rate of Change indicator is above the zero line and accelerating ... The stochastics indicator is above the 50 zone and rising ... volume oscillator moves above the zero line\"; \"buy when the stock price trades 1/8 to 3/8 above yesterday's high\" (p.98)",
            "X|\"a close below the 50 PMA, which would constitute a phase failure\"; \"Sell remaining position when the phase changes\" (p.98)",
            "E|Short mirror in the Bearish Phase: new 60-period lows, falling averages, ROC below zero and falling, stochastic below 50 and declining; \"Short when the stock price trades one-eighth to three-eighths below yesterday's low\" (p.163)",
            "X|Cover on \"a close above the 50 PMA\" / when the Bearish Phase ends (p.164)"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    (id == "cdtag_phase").then_some(("cdtag_phase", "Dukas phase breakout", "50/200/60, ROC 21, stochastic 21"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    (id == "cdtag_phase").then(|| vec![p("off", "Stop offset beyond yesterday's high/low, % (the book: 1/8 to 3/8 of a dollar; default 0)", 0., 0., 3., 0.1),
        p("side", "1 = long only (default 0: long and short)", 0., 0., 1., 1.)])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(f64::NAN, |d| d.def));
    let (off, long_only) = (g("off") / 100., g("side") >= 1.);
    let (h, l, c, n) = (&x.h, &x.l, &x.c, x.c.len());
    let (m50, m200) = (sma(c, 50), sma(c, 200));
    let roc: Vec<f64> = (0..n).map(|i| if i >= 21 { (c[i] / c[i - 21] - 1.) * 100. } else { f64::NAN }).collect();
    let (hh, ll) = (hhv(h, 21), llv(l, 21));
    let fk: Vec<f64> = (0..n).map(|i| if hh[i].is_finite() && hh[i] != ll[i] { 100. * (c[i] - ll[i]) / (hh[i] - ll[i]) } else { f64::NAN }).collect();
    let sk = sma(&fk, 3);
    let v: Vec<f64> = x.r.iter().map(|r| r.volume.filter(|v| *v > 0.).unwrap_or(f64::NAN)).collect();
    let (v5, v21) = (sma(&v, 5), sma(&v, 21));
    let bull = |i: usize| c[i] > m50[i] && c[i] > m200[i] && m50[i] > m200[i];
    let bear = |i: usize| c[i] < m50[i] && c[i] < m200[i] && m50[i] < m200[i];
    let vol_ok = |i: usize| !x.has_volume || v5[i] - v21[i] > 0.;
    let max60 = |i: usize| h[i - 60..i].iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let min60 = |i: usize| l[i - 60..i].iter().copied().fold(f64::INFINITY, f64::min);
    let (mut out, mut pos, mut pend) = (vec![], None::<(f64, usize, f64)>, None::<(f64, f64)>);
    for i in from..=to {
        if let (None, Some((d, stp))) = (pos, pend) {
            if d > 0. && h[i] > stp { pos = Some((1., i, x.o[i].max(stp))); }
            else if d < 0. && l[i] < stp { pos = Some((-1., i, x.o[i].min(stp))); }
        }
        pend = None;
        if let Some((d, e, ep)) = pos {
            if (d > 0. && !bull(i)) || (d < 0. && !bear(i)) {
                out.push(mk(d, e, ep, "stop", i, c[i], "phase changed", false, i));
                pos = None;
            }
        }
        if pos.is_none() && i < to && i >= 60 {
            if bull(i) && h[i] > max60(i) && m50[i] > m50[i - 1] && m200[i] > m200[i - 1] && roc[i] > 0. && roc[i] > roc[i - 1]
                && sk[i] > 50. && sk[i] > sk[i - 1] && vol_ok(i) { pend = Some((1., h[i] * (1. + off))); }
            else if !long_only && bear(i) && l[i] < min60(i) && m50[i] < m50[i - 1] && m200[i] < m200[i - 1] && roc[i] < 0.
                && roc[i] < roc[i - 1] && sk[i] < 50. && sk[i] < sk[i - 1] && vol_ok(i) { pend = Some((-1., l[i] * (1. - off))); }
        }
    }
    if let Some((d, e, ep)) = pos { out.push(mk(d, e, ep, "stop", to, c[to], "open", true, to)); }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn breakout_entry_and_phase_exit() {
        // 260 bars rising 0.5% a day with a bump every third bar (new highs, accelerating ROC), then a 20% drop
        let mut c: Vec<f64> = (0..260).map(|i| 100. * 1.005f64.powi(i) * if i % 3 == 0 { 1.004 } else { 1. }).collect();
        let z = c[259];
        c.extend([z * 0.8, z * 0.75, z * 0.7]);
        let r: Vec<Row> = c.iter().enumerate().map(|(i, &v)| Row { ticker: "T".into(), date: format!("2000-{:02}-{:02}", 1 + i / 25, 1 + i % 25), open: v, high: v * 1.002, low: v * 0.998, close: v, volume: None }).collect();
        let x = Ctx::new(&r, &r, &[]);
        let t = run("cdtag_phase", &x, 0, r.len() - 1, &Params::new()).unwrap();
        // (this series is periodic in ratio terms, so the stochastic's "rising" test sits on rounding noise: check the shape only)
        let z = t.last().unwrap();
        assert_eq!((z.dir, z.exit_i, z.open), (1., 260, false));
        assert!(t.iter().all(|z| z.legs[0].px >= x.h[z.legs[0].i - 1] && z.legs[0].i > 200));
    }
}

// Instrument admission: Chuck Dukas (TRENDadvisor) 2026-11-01 (titles lane K8).
// Admission only for the module REVIEWED lists (authors/titles-1030/K8w/k8_ref.py (header)).
// Runs on actual indices, ETFs and stocks, US and India (index_etf_or_stock, nifty50 basket, declared application).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "cdtag" && crate::books::cdtag::reviewed(def.id),
    label: "A stock-trading method with chart examples and no printed results; ETFs, US/India indices and India stocks are a declared application.",
    basis: "Chuck Dukas, The TRENDadvisor Guide to Breakthrough Profits (Wiley 2006) p.17-23 (toolkit), 97-98 (Bullish Phase), 163-164 (Bearish Phase); independent check authors/titles-1030/K8w/k8_ref.py",
};
