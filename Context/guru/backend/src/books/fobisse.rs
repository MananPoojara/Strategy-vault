//! Stephan Bisse, "The power of X: The MAX moving average", Futures & Options Trader Aug 2009 pp.10-13 (reprint of Active Trader
//! Jun 2005). Research share Trading Knowledge I/@Newsletters/Futures & Options Trader Magazine 2005 - 2010 (futuresandoptionstrader.com)/
//! 2009/Futures & Options Trader. 2009-08.pdf; text authors/titles-1030/K15w/src/fo0908.txt. Book id `fobisse`. Lane K15 (titles loop,
//! 2 Nov 2026), to-code row 302, lead decision 109; independent check authors/titles-1030/K15w/k15_ref.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::tselhs::Book;

pub const REVIEWED: &[&str] = &["fobisse_max"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

/// Table 1 (p.12), best MAX settings 1985-2000: (look-back, power) for S&P 500, British pound, US 10-year note, Eurodollar, crude
/// oil, soybeans, gold.
pub const SETS: [(usize, f64); 7] = [(85, 1.0), (5, 0.75), (40, 2.0), (30, 0.75), (100, 0.75), (25, 0.5), (15, 0.5)];

const NOTE: &str = "[Declared readings] MAX(n, p) = the weighted average of the last n closes with weights k^p, the newest close k = n, the oldest k = 1 (p = 1 is the ordinary WMA). Always in the market, no stop: when MAX crosses above its own value one bar earlier (it rises after not rising) buy at the next open, reversing a short; when it crosses below, sell short at the next open. Default = the article's S&P 500 set, 85 bars and power 1.0 (lead decision 109); p.set 1-6 = the other published sets (British pound 5/0.75, US 10-year note 40/2.0, Eurodollar 30/0.75, crude oil 100/0.75, soybeans 25/0.5, gold 15/0.5). The article optimised every set in-sample (1985-2000, WMA look-back 5-100 then power 0.5-2.5) and warns that in-sample results fall away out of sample; its S&P set made $32,950 in-sample and lost $36,750 out of sample (2000-2005). Daily indices, ETFs and US/India stocks are a declared application; the vault's standard costs.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "fobisse_max", name: "MAX moving average (Bisse): power-weighted average of 85 closes turns up / down, always in", group: "F&O Trader Aug 2009: The power of X, the MAX moving average (Bisse)", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "fobisse", rules: &[
            "F|\"The idea behind MAX is to raise the weighting factor to a user-specified power that can be optimized.\" (F&OT Aug 2009 p.11); e.g. MAX3: \"the most recent price is weighted by 10^3 (1000), the second most recent price is weighted by 9^3 (729), etc., all the way to the oldest data-point, which is weighted [1]\" (p.10)",
            "E|\"use a moving average as a smoothed proxy for price and a one-period lagged value of the same as a signal line: When the moving average crosses over its value of 1 period ago to the upside, a buy signal is produced, while a crossing to the downside produces a sell signal.\" (p.12)",
            "X|\"the closing price was used to calculate the WMA and the MAX, and all trades were executed the following day on the open ... No stop-loss orders were used ... the system was always in the market\" (p.11)"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    (id == "fobisse_max").then_some(("fobisse_max", "Bisse MAX moving average", "85 bars, power 1"))
}
pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    reviewed(id).then(|| vec![ParamDef { key: "set", label: "Published set: 0 = S&P 500 85/1.0; 1 = British pound 5/0.75; 2 = US 10-year note 40/2.0; 3 = Eurodollar 30/0.75; 4 = crude oil 100/0.75; 5 = soybeans 25/0.5; 6 = gold 15/0.5", def: 0., min: 0., max: 6., step: 1. }])
}

pub fn max_ma(c: &[f64], n: usize, p: f64) -> Vec<f64> {
    let w: Vec<f64> = (1..=n).map(|k| (k as f64).powf(p)).collect();
    let sw: f64 = w.iter().sum();
    (0..c.len()).map(|i| if i + 1 < n { f64::NAN } else { (0..n).map(|k| w[k] * c[i + 1 - n + k]).sum::<f64>() / sw }).collect()
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let (n, pw) = SETS[(p.get("set").copied().unwrap_or(0.).round().max(0.) as usize).min(6)];
    let m = max_ma(&x.c, n, pw);
    let mut pos = Book::new();
    for t in from.max(2)..to {
        if !(m[t - 2].is_finite()) { continue; }
        let d = if m[t] > m[t - 1] && m[t - 1] <= m[t - 2] { 1. } else if m[t] < m[t - 1] && m[t - 1] >= m[t - 2] { -1. } else { 0. };
        if d != 0. && pos.side() != d { pos.enter(d, t + 1, x.o[t + 1], "open"); }
    }
    Some(pos.done(to, x.c[to]))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn max_weights_and_trades() {
        let v = max_ma(&[1., 2., 3.], 3, 1.0);
        assert!((v[2] - (1. + 4. + 9.) / 6.).abs() < 1e-12);
        let r = super::super::ctm::tests::walk(2000);
        let x = Ctx::new(&r, &r, &[]);
        let t = run("fobisse_max", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(t.len() > 5 && t.windows(2).all(|w| w[1].legs[0].i == w[0].exit_i && w[1].dir == -w[0].dir));
    }
}

// Instrument admission: Bisse MAX moving average 2026-11-02 (titles lane K15).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "fobisse" && reviewed(def.id),
    label: "Bisse optimised and tested seven futures markets; runs on daily indices, ETFs and stocks, US and India, are a declared application.",
    basis: "Futures & Options Trader Aug 2009 pp.10-13 (Active Trader Jun 2005); independent check authors/titles-1030/K15w/k15_ref.py",
};
