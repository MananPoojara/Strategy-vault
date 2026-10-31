//! Keith Raphael, "How to Overlay Technical Indicators" (Telerate TAG XVII workbook, 1995; research share Trading
//! Knowledge II/@INO (ino.com)/Keith Raphael - How to Overlay Technical Indicators/workbook.pdf, PDF p.15 = printed 630,
//! read as an image). Book id `krovl`. One rule: the TRADEPLAN 'trade band' (30 Jul 91), Trade 1 by default and the
//! printed Trade 2 as an option. Lane K6 (titles loop, 31 Oct 2026); independent check
//! authors/titles-1030/K6w/k6_ref.py (header).
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::mkits::stdev;
use super::rbtie::mk;
use super::svtvc::sma_exact as sma;

/// Admitted ids (authors/titles-1030/K6w/k6_ref.py (header)).
pub const REVIEWED: &[&str] = &["krovl_trade_band"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] ma_sh = 5-day simple average of the close; vol_coef = 1.95 x the 20-day standard deviation of the close (population, TRADEPLAN's formula is not printed); hiband/loband = ma_sh +/- vol_coef; histop = hiband x 1.005, lostop = loband x 0.995. TRADEPLAN values trades at 'last', so every signal is taken and filled at the close of the bar that gives it. One position per plan; exits are tested from the bar after entry; after an exit a new entry can come from the next bar; a bar that touches both bands while flat gives no entry. Trade 1 (default): long when low <= loband, out when close >= ma_sh or high >= ma_sh or low <= lostop; short when high >= hiband, out when close <= ma_sh or low <= ma_sh or high >= histop. Trade 2 (option plan = 2) is coded exactly as printed, including its short entry 'low >= hiband' that looks like a typo. The printed equity line averages the two plans; each plan is a separate run here. per_2 (the 13-day 'ma_md') is defined but unused. Book item: Swiss franc futures (SWF.D), no printed results; indices, ETFs and US/India stocks are a declared application.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "krovl_trade_band", name: "Trade band: fade the 1.95-sigma band around the 5-day average, out at the average or a 0.5% stop", group: "Raphael How to Overlay Technical Indicators",
        session: 0, dir: "Long/Short", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: "krovl", rules: &[
            "F|\"ma_sh = mov(close, per_1)\" (5); \"vol_coef = 1.95 * stdev(close, 20)\"; \"hiband = ma_sh + vol_coef; loband = ma_sh - vol_coef\"; \"stop = .005; histop = hiband * (1 + stop); lostop = loband * (1 - stop)\" (p.630)",
            "E|Trade 1: \"enter long when low <= loband\"; \"enter short when high >= hiband\"",
            "X|Trade 1: \"exit long when close >= ma_sh or high >= ma_sh or low <= lostop\"; \"exit short when close <= ma_sh or low <= ma_sh or high >= histop\"",
            "F|Trade 2 (option): long when low <= loband, out when high >= hiband or (close <= ma_sh and low < loband); short when \"low >= hiband\", out when low <= loband or (close >= ma_sh and high < hiband)"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    (id == "krovl_trade_band").then_some(("krovl_trade_band", "Raphael trade band", "SMA 5, 1.95 x SD 20"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    (id == "krovl_trade_band").then(|| vec![ParamDef { key: "plan", label: "Trade plan: 1 (default) or 2 (as printed)", def: 1., min: 1., max: 2., step: 1. }])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let plan2 = p.get("plan").copied().unwrap_or(1.).round() >= 2.;
    let (h, l, c) = (&x.h, &x.l, &x.c);
    let (ma, sd) = (sma(c, 5), stdev(c, 20));
    let (mut out, mut cur) = (vec![], None::<(f64, usize, f64)>);
    for i in from..=to {
        let (m, hb, lb) = (ma[i], ma[i] + 1.95 * sd[i], ma[i] - 1.95 * sd[i]);
        if !(hb.is_finite() && lb.is_finite()) { continue; }
        let (hs, ls) = (hb * 1.005, lb * 0.995);
        match cur {
            Some((d, e, ep)) => {
                if i == e { continue; }
                let out_now = match (plan2, d > 0.) {
                    (false, true) => c[i] >= m || h[i] >= m || l[i] <= ls,
                    (false, false) => c[i] <= m || l[i] <= m || h[i] >= hs,
                    (true, true) => h[i] >= hb || (c[i] <= m && l[i] < lb),
                    (true, false) => l[i] <= lb || (c[i] >= m && h[i] < hb),
                };
                if out_now { out.push(mk(d, e, ep, "close", i, c[i], "trade band exit", false, i)); cur = None; }
            }
            None => {
                let (lg, sh) = (l[i] <= lb, if plan2 { l[i] >= hb } else { h[i] >= hb });
                if lg != sh { cur = Some((if lg { 1. } else { -1. }, i, c[i])); }
            }
        }
    }
    if let Some((d, e, ep)) = cur { out.push(mk(d, e, ep, "close", to, c[to], "open", true, to)); }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn band_fade_in_at_the_close_out_at_the_average() {
        // 20 bars alternating 100/102 (SD 1, band +/-1.95 around the 5-day average), then a dip, then a recovery
        let mut b: Vec<(f64, f64, f64)> = (0..20).map(|i| { let c = if i % 2 == 0 { 100. } else { 102. }; (c + 0.5, c - 0.5, c) }).collect();
        b.extend([(100., 98., 98.5), (101.5, 98.4, 101.3), (101.5, 100.5, 101.)]);
        let r: Vec<Row> = b.iter().enumerate().map(|(i, &(h, l, c))| Row { ticker: "T".into(), date: format!("{i}"), open: c, high: h, low: l, close: c, volume: None }).collect();
        let x = Ctx::new(&r, &r, &[]);
        let t = run("krovl_trade_band", &x, 0, r.len() - 1, &Params::new()).unwrap();
        // bar 20: ma 100.5, sd < 1.2 -> loband > 98.2 >= low 98 -> long at 98.5; bar 21: close 101.3 >= ma -> out
        assert_eq!(t.iter().map(|z| (z.dir, z.legs[0].i, z.exit_i)).collect::<Vec<_>>(), vec![(1., 20, 21)]);
        assert!(run("krovl_x", &x, 0, 5, &Params::new()).is_none());
    }
}

// Instrument admission: Keith Raphael (TRADEPLAN trade band) 2026-10-31 (titles lane K6).
// Admission only for the module REVIEWED lists (authors/titles-1030/K6w/k6_ref.py (header)).
// Runs on actual indices, ETFs and stocks, US and India (index_etf_or_stock, nifty50 basket, declared application).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "krovl" && crate::books::krovl::reviewed(def.id),
    label: "Keith Raphael's TRADEPLAN example runs on Swiss franc futures with no printed results; indices, ETFs and US/India stocks are a declared application.",
    basis: "Keith Raphael, How to Overlay Technical Indicators (Telerate TAG XVII, 1995), TRADEPLAN 'trade band', printed p.630 (PDF p.15)",
};
