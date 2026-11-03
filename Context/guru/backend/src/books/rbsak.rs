//! Rolf D. Burti, "The Swiss Army Knife of Trading Tools" (FastTrack TRADE program presentation, 2000; research share
//! USB HDD 2/tRADING KNO-2/Rolf D.Burti - The Swiss Army Knife Of Trading Tools/...PPT, 26 slides). Book id `rbsak`.
//! One rule: the SPSto signal - a 5-day close-only stochastic and RSI(14) on the S&P 500 switching a fund between
//! long and cash. Not coded: the Don Beasley fund-family ranking module (needs fund NAVs). Lane K6 (titles loop,
//! 31 Oct 2026); independent check authors/titles-1030/K6w/k6_ref.py (header).
use crate::connors::{rsi, Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::mkits::{hhv, llv};
use super::rbtie::mk;

/// Admitted ids (authors/titles-1030/K6w/k6_ref.py (header)).
pub const REVIEWED: &[&str] = &["rbsak_spsto"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] FastTrack data carry one price per day, so the stochastic is close-only: 100 x (C - lowest close of 5 days)/(highest close of 5 days - lowest close of 5 days), raw %K with no smoothing (the .INI's 'emadays = none' is commented out); RSI = Wilder 14 on closes. 'Nup' = today above N after yesterday at or below N; 'Ndn' = today below N after yesterday at or above N. Buy when any buy crossing happens and no sell crossing on the same day; sell when any sell crossing happens or the close is at or below 96% of the entry close (the 4% stop). Delay 0: signals and fills at that day's close; long or cash, cash earns nothing. The .INI's 'mindays = 8' has no stated meaning and the printed signal file holds trades of 1-3 days, so it is not applied (option: a minimum hold in days before a sell signal counts). Buy level 14 as printed (annotated '11 to 14'). The book switched a Rydex fund (RYOCX) on the S&P 500 index signal; SPX, NIFTY, ETFs and US/India stocks each run on their own closes as a declared application. Printed sample: RYOCX AnnRet 66.7% (9/96-5/00), not compared.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "rbsak_spsto", name: "SPSto: 5-day close stochastic and RSI(14) level crossings, long or cash, 4% stop", group: "Burti Swiss Army Knife (FastTrack)",
        session: 0, dir: "Long", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: "rbsak", rules: &[
            "F|SPSTO.INI: lookback = 5 (stochastic), RSI 14; index sp-cp (S&P 500 close)",
            "E|\"Buy: StocH 14up Or 36up Or 78up Or Rsi 45up\"",
            "X|\"Sell: StocH 76dn Or 36dn Or 18dn Or Rsi 35dn Or SP -4% stop\"",
            "F|Signal pair example: RYOCX vs money market SPRXX, \"Delay 0\", \"HoldMinDays 1\""],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    (id == "rbsak_spsto").then_some(("rbsak_spsto", "Burti SPSto", "Stoch 5 close, RSI 14, 4% stop"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    (id == "rbsak_spsto").then(|| vec![p("stop", "Stop, % below the entry close (printed 4)", 4., 0.5, 20., 0.5),
        p("mindays", "Minimum hold in days before a sell signal counts (not applied: 0; the .INI says 'mindays = 8')", 0., 0., 20., 1.)])
}

pub fn stoch_close(c: &[f64], n: usize) -> Vec<f64> {
    let (hh, ll) = (hhv(c, n), llv(c, n));
    (0..c.len()).map(|i| { let d = hh[i] - ll[i]; if d > 0. { 100. * (c[i] - ll[i]) / d } else { f64::NAN } }).collect()
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(f64::NAN, |d| d.def));
    let (stop, mind) = (g("stop") / 100., g("mindays").round().max(0.) as usize);
    let c = &x.c;
    let (s, r) = (stoch_close(c, 5), rsi(c, 14));
    let up = |v: &[f64], k: f64, i: usize| i > 0 && v[i] > k && v[i - 1] <= k;
    let dn = |v: &[f64], k: f64, i: usize| i > 0 && v[i] < k && v[i - 1] >= k;
    let (mut out, mut cur) = (vec![], None::<(usize, f64)>);
    for i in from..=to {
        let sell = dn(&s, 76., i) || dn(&s, 36., i) || dn(&s, 18., i) || dn(&r, 35., i);
        match cur {
            Some((e, ep)) => {
                let stopped = c[i] <= ep * (1. - stop);
                if i > e && (stopped || (sell && i - e >= mind)) {
                    out.push(mk(1., e, ep, "close", i, c[i], if stopped { "4% stop" } else { "sell signal" }, false, i)); cur = None;
                }
            }
            None => if !sell && (up(&s, 14., i) || up(&s, 36., i) || up(&s, 78., i) || up(&r, 45., i)) { cur = Some((i, c[i])); },
        }
    }
    if let Some((e, ep)) = cur { out.push(mk(1., e, ep, "close", to, c[to], "open", true, to)); }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn stochastic_crossings_and_the_stop() {
        let c = [10., 9., 8., 7., 6., 6.5, 7., 7.5, 8., 7.9, 7.0];
        // bar 5: closes 9, 8, 7, 6, 6.5 -> (6.5 - 6)/(9 - 6) = 16.7, up through 14 from 0 -> buy 6.5;
        // bar 10: closes 7, 7.5, 8, 7.9, 7 -> 0, down through 76/36/18 from 93.3 -> sell 7
        assert!((stoch_close(&c, 5)[5] - 50. / 3.).abs() < 1e-12);
        let r: Vec<Row> = c.iter().enumerate().map(|(i, &v)| Row { ticker: "T".into(), date: format!("{i}"), open: v, high: v, low: v, close: v, volume: None }).collect();
        let x = Ctx::new(&r, &r, &[]);
        let t = run("rbsak_spsto", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert_eq!(t.iter().map(|z| (z.legs[0].i, z.legs[0].px, z.exit_i, z.exit_px)).collect::<Vec<_>>(), vec![(5, 6.5, 10, 7.)]);
        let s1: Params = [("stop".to_string(), 1.)].into_iter().collect();
        assert_eq!(run("rbsak_spsto", &x, 0, r.len() - 1, &s1).unwrap()[0].exit_i, 10, "7 > 6.5 x 0.99: no stop");
        assert!(run("rbsak_x", &x, 0, 5, &Params::new()).is_none());
    }
}

// Instrument admission: Rolf D. Burti (FastTrack SPSto) 2026-10-31 (titles lane K6).
// Admission only for the module REVIEWED lists (authors/titles-1030/K6w/k6_ref.py (header)).
// Runs on actual indices, ETFs and stocks, US and India (index_etf_or_stock, nifty50 basket, declared application).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "rbsak" && crate::books::rbsak::reviewed(def.id),
    label: "Rolf Burti timed a Rydex fund on the S&P 500 index signal; SPX, NIFTY, ETFs and US/India stocks each on their own closes are a declared application.",
    basis: "Rolf D. Burti, The Swiss Army Knife of Trading Tools (FastTrack, 2000), slides 'SPSTO SIGNAL REPORT', 'SPSTO SIGNAL FILE', 'SINGLE FUND TRADING STRATEGY', 'SPSTO.INI FILE'",
};
