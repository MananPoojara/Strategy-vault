//! Austin Passamonte, "Working Money: Simply Profitable", Technical Analysis of Stocks & Commodities V.22:11 (November 2004)
//! pp.88-91 (research share Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V22/C11/
//! 250PASS.pdf). Book id `scpass`: the VIX 5/20 moving-average swing rule. Lane K13 (titles loop, 2 Nov 2026), to-code row 219,
//! lead decision 81; independent check authors/titles-1030/K13w/k13_ref.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::msws::sma;
use super::sctraub::ema0;
use super::tselhs::Book;
use std::collections::HashMap;
use std::sync::OnceLock;

pub const REVIEWED: &[&str] = &["scpass_vix"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] The article states both signals, 'go long' and 'go short', so the rule is always in after the first signal (decision 81). VIX = the daily VIX closes of the engine's volatility file (US: CBOE VIX; India: India VIX for Indian instruments), kept on this instrument's trading days; 5-period EMA (seeded with the first VIX close) and 20-period SMA of those closes. A cross = the EMA on the other side of the SMA than on the instrument's previous day; 'price action below both' = the VIX close below both averages. Signals and fills at the instrument's close. No stops or targets are given for the daily swing rule. The article shows chart examples on the S&P 500 and the Russell 2000 e-minis (2004), no test table. Runs on indices, ETFs and stocks, US and India, with their market's own VIX, are a declared application.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "scpass_vix", name: "Passamonte VIX swing: long when the VIX 5-day EMA crosses under its 20-day SMA with the VIX below both; short on the opposite cross", group: "S&C: Working Money, Simply Profitable (Passamonte, V.22:11)", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: false, needs_vix: true, needs_series: &[], book: "scpass", rules: &[
            "E|\"Go long when the VIX daily-chart five-period moving average crosses below the 20-period moving average with price action below both.\" (V.22:11 p.89)",
            "X|\"Go short when the VIX daily-chart five-period moving average crosses above the 20-period moving average with price action above both.\" (p.89)",
            "F|Chart legend: \"5-period DMA (exponential) 20-period DMA (simple)\" (Figure 1)"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let id = REVIEWED.iter().find(|r| **r == id)?;
    Some((*id, "Passamonte VIX moving averages (S&C)", "VIX 5 EMA / 20 SMA cross"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> { reviewed(id).then(Vec::new) }

/// VIX closes by date from the volatility directory the server was started with (argument 3), as hfm.rs does.
/// ponytail: re-reads the file once per process because `VixAligned` keeps its closes private.
fn vix_rows() -> &'static Vec<(String, f64)> {
    static V: OnceLock<Vec<(String, f64)>> = OnceLock::new();
    V.get_or_init(|| match std::env::args().nth(3) { Some(dir) => crate::load_vix_dir(&dir).into_iter().map(|r| (r.date, r.close)).collect(), None => vec![] })
}

/// Long/short signals from VIX closes aligned to the bars (`vx`, NaN where missing) and their averages.
fn signals(x: &Ctx, from: usize, to: usize, v: &[f64], e: &[f64], s: &[f64]) -> Vec<Trade> {
    let mut pos = Book::new();
    for i in from.max(1)..=to {
        if ![v[i], e[i], s[i], e[i - 1], s[i - 1]].iter().all(|z| z.is_finite()) { continue; }
        if e[i] < s[i] && e[i - 1] >= s[i - 1] && v[i] < e[i] && v[i] < s[i] { pos.enter(1.0, i, x.c[i], "close"); }
        else if e[i] > s[i] && e[i - 1] <= s[i - 1] && v[i] > e[i] && v[i] > s[i] { pos.enter(-1.0, i, x.c[i], "close"); }
    }
    pos.done(to, x.c[to])
}

fn aligned(x: &Ctx, vix: &[(String, f64)]) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
    let first = x.r.first().map_or("", |r| r.date.as_str());
    let days: std::collections::HashSet<&str> = x.r.iter().map(|r| r.date.as_str()).collect();
    let kept: Vec<&(String, f64)> = vix.iter().filter(|(d, _)| d.as_str() < first || days.contains(d.as_str())).collect();
    let vc: Vec<f64> = kept.iter().map(|z| z.1).collect();
    let (e5, s20) = (ema0(&vc, 5), sma(&vc, 20));
    let at: HashMap<&str, usize> = kept.iter().enumerate().map(|(k, z)| (z.0.as_str(), k)).collect();
    let n = x.r.len();
    let (mut v, mut e, mut s) = (vec![f64::NAN; n], vec![f64::NAN; n], vec![f64::NAN; n]);
    for (i, r) in x.r.iter().enumerate() { if let Some(&k) = at.get(r.date.as_str()) { v[i] = vc[k]; e[i] = e5[k]; s[i] = s20[k]; } }
    (v, e, s)
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, _p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let (v, e, s) = aligned(x, vix_rows());
    Some(signals(x, from, to, &v, &e, &s))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn vix_cross_signals() {
        let r = super::super::sctraub::tests::bars(1500);
        let x = Ctx::new(&r, &r, &[]);
        let vix: Vec<(String, f64)> = r.iter().enumerate().map(|(i, z)| (z.date.clone(), 20. + 6. * (i as f64 / 9.).sin())).collect();
        let (v, e, s) = aligned(&x, &vix);
        let t = signals(&x, 0, r.len() - 1, &v, &e, &s);
        assert!(t.len() > 5 && t.iter().any(|z| z.dir > 0.0) && t.iter().any(|z| z.dir < 0.0));
        assert!(family_of("scpass_vix").is_some());
    }
}

// Instrument admission: S&C Passamonte VIX 2026-11-02 (titles lane K13).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "scpass" && reviewed(def.id),
    label: "Austin Passamonte showed the VIX rule on S&P 500 and Russell 2000 e-mini charts (2004). Runs on indices, ETFs and stocks, US (CBOE VIX) and India (India VIX), are a declared application.",
    basis: "Technical Analysis of Stocks & Commodities V.22:11 pp.88-91 (Working Money: Simply Profitable); independent check authors/titles-1030/K13w/k13_ref.py",
};
