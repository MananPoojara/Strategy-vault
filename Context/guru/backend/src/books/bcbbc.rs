//! Barry Collette, "Bollinger Bands" course manual (c.2003; research share USB HDD 2/tRADING KNO-2/Barry Collette Bollinger
//! Bands Course (114 MB)/Bollinger Bands.pdf). Book id `bcbbc`. One rule: the Squeeze Breakout (printed p.41-48) with the
//! course's Parabolic SAR trailing stop. Lane K8 (titles loop, 1 Nov 2026); independent check
//! authors/titles-1030/K8w/k8_ref.py (header). Related held cards: swhoi_bollinger (Collette's chapter in High Octane
//! Investing, no threshold or exit there) and bobb_m1_squeeze (Bollinger's own Method I).
use crate::connors::{sma, Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::mkits::stdev;
use super::rbtie::mk;

/// Admitted ids (authors/titles-1030/K8w/k8_ref.py (header)).
pub const REVIEWED: &[&str] = &["bcbbc_squeeze"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] Bollinger Bands 20 days, 2 standard deviations (population), as the course's default ('20-Day Moving Average with 2 Sigma'). BandWidth W = 100 x (upper - lower) / middle band. The squeeze = W below 10 on the day before the breakout ('The squeeze should be tight, less than 10 and preferably less than 5'; 5 is an option). The breakout = a close above the upper band (long) or below the lower band (short) ('we actually buy when price is at the upper band and we short when it is at the lower band'), filled at the next open. Exit = 'Use Parabolic SAR as a trailing stop': Wilder's 0.02 step, 0.20 maximum, starting on the entry day at the breakout day's low (high for a short) with that day's high (low) as the first extreme; a low below the SAR (high above it) exits at the SAR, or at the open on a gap. One position at a time. Not applied (judgement): the volume, RSI and candle confirmations, 'we make sure from historical there's not a habit of head fakes', and the stops from the stock's 'personality'. Long only is an option. The course shows US stock charts and no results; ETFs, US/India indices and India stocks are a declared application.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "bcbbc_squeeze", name: "Bollinger Squeeze Breakout: BandWidth under 10, close outside the band, Parabolic SAR trailing stop", group: "Collette Bollinger Bands course",
        session: 0, dir: "Long/Short", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: "bcbbc", rules: &[
            "F|BandWidth W = \"(Upper Bollinger Band - lower Bollinger Band)/moving avg.\" (p.41); bands \"20-Day Moving Average with 2 Sigma\" (p.36)",
            "E|\"The squeeze should be tight, less than 10 and preferably less than 5\"; \"we actually buy when price is at the upper band and we short when it is at the lower band\" (p.43)",
            "X|\"Use Parabolic SAR as a trailing stop\" (p.48)"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    (id == "bcbbc_squeeze").then_some(("bcbbc_squeeze", "Collette squeeze breakout", "BB 20/2, W < 10, SAR"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    (id == "bcbbc_squeeze").then(|| vec![p("w", "Squeeze: BandWidth below this, % (10; 'preferably less than 5')", 10., 1., 30., 0.5),
        p("len", "Band length, days (20)", 20., 5., 100., 1.), p("k", "Standard deviations (2)", 2., 0.5, 4., 0.1),
        p("side", "1 = long only (default 0: long and short)", 0., 0., 1., 1.)])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(f64::NAN, |d| d.def));
    let (n, k, wmax, long_only) = (g("len").round().max(2.) as usize, g("k"), g("w"), g("side") >= 1.);
    let (h, l, c) = (&x.h, &x.l, &x.c);
    let (m, sd) = (sma(c, n), stdev(c, n));
    let up: Vec<f64> = m.iter().zip(&sd).map(|(a, b)| a + k * b).collect();
    let lo: Vec<f64> = m.iter().zip(&sd).map(|(a, b)| a - k * b).collect();
    let w: Vec<f64> = (0..c.len()).map(|i| if m[i].is_finite() && m[i] != 0. { 100. * (up[i] - lo[i]) / m[i] } else { f64::NAN }).collect();
    // (dir, entry bar, entry price, SAR, extreme, acceleration)
    let (mut out, mut pos, mut pend) = (vec![], None::<(f64, usize, f64, f64, f64, f64)>, None::<f64>);
    for i in from..=to {
        if let (None, Some(d)) = (pos, pend) {
            let s = i - 1;
            pos = Some((d, i, x.o[i], if d > 0. { l[s] } else { h[s] }, if d > 0. { h[s] } else { l[s] }, 0.02));
        }
        pend = None;
        if let Some((d, e, ep, sar, mut xp, mut af)) = pos {
            if d > 0. && l[i] < sar { out.push(mk(1., e, ep, "open", i, x.o[i].min(sar), "Parabolic SAR", false, i)); pos = None; }
            else if d < 0. && h[i] > sar { out.push(mk(-1., e, ep, "open", i, x.o[i].max(sar), "Parabolic SAR", false, i)); pos = None; }
            else {
                if d > 0. && h[i] > xp { xp = h[i]; af = (af + 0.02).min(0.2); } else if d < 0. && l[i] < xp { xp = l[i]; af = (af + 0.02).min(0.2); }
                let nx = sar + af * (xp - sar);
                let nx = if d > 0. { nx.min(l[i]).min(l[i - 1]) } else { nx.max(h[i]).max(h[i - 1]) };
                pos = Some((d, e, ep, nx, xp, af));
            }
        }
        if pos.is_none() && i < to && i >= 1 && w[i - 1] < wmax {
            if c[i] > up[i] { pend = Some(1.); } else if !long_only && c[i] < lo[i] { pend = Some(-1.); }
        }
    }
    if let Some((d, e, ep, ..)) = pos { out.push(mk(d, e, ep, "open", to, c[to], "open", true, to)); }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn squeeze_breakout_then_sar_exit() {
        // 30 quiet bars around 100, a breakout close at 103, two up days, then a drop through the SAR
        let mut c: Vec<f64> = (0..30).map(|i| 100. + if i % 2 == 0 { 0.2 } else { -0.2 }).collect();
        c.extend([103., 104., 105., 100.]);
        let r: Vec<Row> = c.iter().enumerate().map(|(i, &v)| Row { ticker: "T".into(), date: format!("2001-{:02}-{:02}", 1 + i / 28, 1 + i % 28), open: v, high: v + 0.5, low: v - 0.5, close: v, volume: None }).collect();
        let x = Ctx::new(&r, &r, &[]);
        let t = run("bcbbc_squeeze", &x, 0, r.len() - 1, &Params::new()).unwrap();
        // signal at bar 30, long at bar 31's open 104, SAR 102.5 (bar 30's low), then 102.5 (capped by that low), 102.68;
        // bar 33 opens at 100, below the SAR: out at the open
        assert_eq!(t.iter().map(|z| (z.dir, z.legs[0].i, z.legs[0].px, z.exit_i, z.exit_px)).collect::<Vec<_>>(), vec![(1., 31, 104., 33, 100.)]);
    }
}

// Instrument admission: Barry Collette (Bollinger Bands course) 2026-11-01 (titles lane K8).
// Admission only for the module REVIEWED lists (authors/titles-1030/K8w/k8_ref.py (header)).
// Runs on actual indices, ETFs and stocks, US and India (index_etf_or_stock, nifty50 basket, declared application).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "bcbbc" && crate::books::bcbbc::reviewed(def.id),
    label: "A course method shown on US stock charts with no results; ETFs, US/India indices and India stocks are a declared application.",
    basis: "Barry Collette, Bollinger Bands course manual (c.2003) printed p.36, 41-48; independent check authors/titles-1030/K8w/k8_ref.py",
};
