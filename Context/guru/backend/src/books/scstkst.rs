//! Matt Blackman with Derek Stukuls, short-term (daily) KST, Technical Analysis of Stocks & Commodities V.21:7 (July 2003) pp.82-87
//! (research share Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V21/C07/157BLAK.pdf).
//! Book id `scstkst`. Lane K16 (titles loop, 2 Nov 2026), to-code row 272; independent check authors/titles-1030/K16w/k16_ref.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::msws::sma;
use super::sctraub::next_open;

pub const REVIEWED: &[&str] = &["scstkst_daily"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] The printed MetaStock formula: KST = Mov(ROC(C,10,%),10,S) x 1 + Mov(ROC(C,15,%),10,S) x 2 + Mov(ROC(C,20,%),10,S) x 3 + Mov(ROC(C,30,%),15,S) x 4; enter long on Cross(KST, Mov(KST,10,S)), sell on the cross back below. Long only; MetaStock System Tester fills at the next open. The article shows signals on daily S&P 500 charts without a results table. Pring's own KST cards in the vault (long-term daily KST vs its 130-day average, weekly and monthly KST) are other rules. Runs on indices, ETFs and stocks, US and India, are a declared application.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "scstkst_daily", name: "Short-term daily KST crosses its 10-day SMA: long, out on the cross back, next open", group: "S&C: daily KST (Blackman/Stukuls, V.21:7)", session: 0, dir: "Long", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "scstkst", rules: &[
            "F|\"KST:=(Mov(ROC(Close,10,%),10,S)*1) + (Mov(ROC(Close,15,%),10,S)*2) + (Mov(ROC(Close,20,%),10,S)*3) + (Mov(ROC(Close,30,%),15,S)*4);\" (V.21:7, PDF p.2)",
            "E|\"Enter Long ... Cross(KST,Mov(KST,10,S))\" (PDF p.2)", "X|Sell when the KST crosses back below its 10-period SMA (Cross(Mov(KST,10,S),KST))."],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { reviewed(id).then_some(("scstkst_daily", "Short-term daily KST (S&C)", "10-day signal")) }
pub fn params(id: &str) -> Option<Vec<ParamDef>> { reviewed(id).then(Vec::new) }

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, _p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let roc = |n: usize| -> Vec<f64> { (0..x.c.len()).map(|i| if i >= n { 100.0 * (x.c[i] / x.c[i - n] - 1.0) } else { f64::NAN }).collect() };
    let (a, b, c, d) = (sma(&roc(10), 10), sma(&roc(15), 10), sma(&roc(20), 10), sma(&roc(30), 15));
    let k: Vec<f64> = (0..x.c.len()).map(|i| a[i] + 2.0 * b[i] + 3.0 * c[i] + 4.0 * d[i]).collect();
    let s = sma(&k, 10);
    Some(next_open(x, from, to, "KST below its average", |t, side| {
        if t == 0 || !s[t - 1].is_finite() { return side; }
        if k[t] > s[t] && k[t - 1] <= s[t - 1] { 1.0 } else if k[t] < s[t] && k[t - 1] >= s[t - 1] { 0.0 } else { side }
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn kst_cross() {
        let r = super::super::sctraub::tests::bars(1500);
        let x = Ctx::new(&r, &r, &[]);
        assert!(run("scstkst_daily", &x, 0, r.len() - 1, &Params::new()).unwrap().iter().all(|z| z.dir > 0.0));
    }
}

// Instrument admission: S&C daily KST 2026-11-02 (titles lane K16).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "scstkst" && reviewed(def.id),
    label: "Blackman and Stukuls showed the daily KST on S&P 500 charts (no results table). Runs on indices, ETFs and stocks, US and India, are a declared application.",
    basis: "Technical Analysis of Stocks & Commodities V.21:7 pp.82-87, MetaStock KST formula; independent check authors/titles-1030/K16w/k16_ref.py",
};
