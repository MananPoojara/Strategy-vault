//! Matt Blackman with Derek Stukuls, short-term (daily) KST, Technical Analysis of Stocks & Commodities V.21:7 (July 2003) pp.82-87
//! (research share Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V21/C07/157BLAK.pdf).
//! Book id `scstkst`. Lane K16 (titles loop, 2 Nov 2026), to-code row 272; independent check authors/titles-1030/K16w/k16_ref.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::msws::sma;
use super::sctraub::next_open;

pub const REVIEWED: &[&str] = &["scstkst_daily"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] The printed MetaStock formula: KST = Mov(ROC(C,10,%),10,S) x 1 + Mov(ROC(C,15,%),10,S) x 2 + Mov(ROC(C,20,%),10,S) x 3 + Mov(ROC(C,30,%),15,S) x 4; the printed system is long and short: Enter Long and Close Short on Cross(KST, Mov(KST,10,S)), Close Long and Enter Short on Cross(Mov(KST,10,S), KST), so after the first cross it is always in, reversing on each cross. 'For the purposes of this test, we used open with a delay of one day': fills at the next open. Option long = 1 trades the long side only (out to cash on the cross down). Article (daily test, February 2002 - February 2003, $14.95 commission each way; the instrument is not named, its daily chart is the S&P 500): 11 trades, 3 winners and 8 losers, +1.79% for the year against -21.47% buy and hold; a 12-period KST average (optimised 3-50) gave 18%. Pring's own KST cards in the vault (long-term daily KST vs its 130-day average, weekly and monthly KST) are other rules. Runs on indices, ETFs and stocks, US and India, are a declared application.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "scstkst_daily", name: "Short-term daily KST crosses its 10-day SMA: long on the cross up, short on the cross down, next open", group: "S&C: daily KST (Blackman/Stukuls, V.21:7)", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "scstkst", rules: &[
            "F|\"KST:=(Mov(ROC(Close,10,%),10,S)*1) + (Mov(ROC(Close,15,%),10,S)*2) + (Mov(ROC(Close,20,%),10,S)*3) + (Mov(ROC(Close,30,%),15,S)*4);\" (V.21:7, PDF p.2)",
            "E|\"Enter Long ... Cross(KST,Mov(KST,10,S))\" (PDF p.2)", "E|\"Enter Short ... Cross(Mov(KST,10,S),KST)\" (PDF p.3)", "X|\"Close Long ... Cross(Mov(KST,10,S),KST)\"; \"Close Short ... Cross(KST,Mov(KST,10,S))\" (PDF p.3); \"we used open with a delay of one day\" (PDF p.3)"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { reviewed(id).then_some(("scstkst_daily", "Short-term daily KST (S&C)", "10-day signal")) }
pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    reviewed(id).then(|| vec![ParamDef { key: "long", label: "1 = long side only (out to cash on the cross down); 0 = long and short as printed", def: 0., min: 0., max: 1., step: 1. }])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let roc = |n: usize| -> Vec<f64> { (0..x.c.len()).map(|i| if i >= n { 100.0 * (x.c[i] / x.c[i - n] - 1.0) } else { f64::NAN }).collect() };
    let (a, b, c, d) = (sma(&roc(10), 10), sma(&roc(15), 10), sma(&roc(20), 10), sma(&roc(30), 15));
    let k: Vec<f64> = (0..x.c.len()).map(|i| a[i] + 2.0 * b[i] + 3.0 * c[i] + 4.0 * d[i]).collect();
    let s = sma(&k, 10);
    let dn = if p.get("long").copied().unwrap_or(0.) > 0.5 { 0.0 } else { -1.0 };
    Some(next_open(x, from, to, "KST below its average", |t, side| {
        if t == 0 || !s[t - 1].is_finite() { return side; }
        if k[t] > s[t] && k[t - 1] <= s[t - 1] { 1.0 } else if k[t] < s[t] && k[t - 1] >= s[t - 1] { dn } else { side }
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn kst_cross() {
        let r = super::super::sctraub::tests::bars(1500);
        let x = Ctx::new(&r, &r, &[]);
        let both = run("scstkst_daily", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(both.iter().any(|z| z.dir < 0.0) && both.windows(2).all(|w| w[0].dir != w[1].dir && w[0].exit_i == w[1].legs[0].i));
        let mut p = Params::new(); p.insert("long".into(), 1.);
        assert!(run("scstkst_daily", &x, 0, r.len() - 1, &p).unwrap().iter().all(|z| z.dir > 0.0));
    }
}

// Instrument admission: S&C daily KST 2026-11-02 (titles lane K16).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "scstkst" && reviewed(def.id),
    label: "Blackman and Stukuls showed the daily KST on S&P 500 charts (no results table). Runs on indices, ETFs and stocks, US and India, are a declared application.",
    basis: "Technical Analysis of Stocks & Commodities V.21:7 pp.82-87, MetaStock KST formula; independent check authors/titles-1030/K16w/k16_ref.py",
};
