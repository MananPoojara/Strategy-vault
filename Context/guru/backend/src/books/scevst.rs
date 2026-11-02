//! Stuart Evens, "Stochastics", Technical Analysis of Stocks & Commodities V.17:9 (September 1999) pp.392-398 (research share
//! Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V17/C09/067STOC.pdf). Book id `scevst`:
//! comparisons 1-5 of the printed MetaStock code. Lane K16 (titles loop, 2 Nov 2026), to-code row 254; independent check
//! authors/titles-1030/K16w/k16_ref.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::lc3::{hi, lo};
use super::msws::sma;
use super::scevbb::ms_book;

pub const REVIEWED: &[&str] = &["scevst_1", "scevst_2", "scevst_3", "scevst_4"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

/// MetaStock Stoch(n, s): 100 * Sum(C - LLV(L,n), s) / Sum(HHV(H,n) - LLV(L,n), s).
pub fn ms_stoch(x: &Ctx, n: usize, s: usize) -> Vec<f64> {
    let len = x.c.len();
    let (mut a, mut b) = (vec![f64::NAN; len], vec![f64::NAN; len]);
    for i in n - 1..len { let l = lo(&x.l, i + 1 - n, i); a[i] = x.c[i] - l; b[i] = hi(&x.h, i + 1 - n, i) - l; }
    let (sa, sb) = (sma(&a, s), sma(&b, s));
    (0..len).map(|i| if sb[i].is_finite() && sb[i] > 0.0 { 100.0 * sa[i] / sb[i] } else { f64::NAN }).collect()
}

const NOTE: &str = "[Declared readings] The printed MetaStock code, %K = Stoch(5, slowing) and %D = Mov(%K, 3, S), Cross() as MetaStock (above today, at or below yesterday); System Tester trades 'at the opening price on the day following the signal', long and short, no stops. 1: open long Cross(%K, %D), open short Cross(%D, %K), each closing the other side. 2: as 1 but open long only when LLV(%K, 2) < 20 and open short only when HHV(%K, 2) > 80 (the code; the list says '%K must be below 20'); close on the opposite cross at any level. 3: as 2 but close long whenever %K > 80 and short whenever %K < 20. 4: entries and exits need the cross with %K or %D beyond 20/80. Slowing 1 (no slowing) is the default for each card; slowing 5 is the article's 'A' version and slowing 3 on card 4 is its comparison 5. Article: 32 US stocks, about five to six years to 1999, results by stock (points); comparison 6 is buy and hold. Runs on indices, ETFs and other stocks, US and India, are a declared application.";

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, rules| StrategyDef { id, name, group: "S&C: Stochastics (Evens, V.17:9)", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "scevst", rules, notes: NOTE, claim: None };
    vec![
        d("scevst_1", "Evens stochastics 1: %K(5) crosses %D(3), stop and reverse at the next open", &[
            "E|\"1 Go long when %K crosses %D to the upside and go short when crossing to the downside. No %K slowing.\" (V.17:9, PDF p.2)",
            "F|\"OL: Cross(Stoch(5,1),Mov(Stoch(5,1),3,S)) CL: Cross(Mov(Stoch(5,1),3,S), Stoch(5,1)) OS: Cross(Mov(Stoch(5,1),3,S), Stoch(5,1)) CS: Cross(Stoch(5,1),Mov(Stoch(5,1),3,S))\" (PDF p.2)", "X|Stop and reverse on the opposite cross (the close rules equal the other side's open rules)."]),
        d("scevst_2", "Evens stochastics 2: open on the cross only from below 20 / above 80, close on any opposite cross", &[
            "E|\"2 Same as 1, but to open long, %K must be below 20 and to open short, %K must be above 80. Closing position is on crossover at any level.\" (PDF p.2); code \"OL: Cross(Stoch(5,1),Mov(Stoch(5,1),3,S)) AND LLV(Stoch(5,1), 2)<20 ... OS: ... AND HHV(Stoch(5,1),2)>80\"", "X|\"Closing position is on crossover at any level.\" (PDF p.2)"]),
        d("scevst_3", "Evens stochastics 3: as 2, but close longs whenever %K > 80 and shorts whenever %K < 20", &[
            "E|\"3 Same as 2, but close long anytime %K is above 80 and close short anytime %K is below 20.\" (PDF p.2); code \"CL: Stoch(5,1)>80 ... CS: Stoch(5,1)<20\"", "X|Close long: %K above 80; close short: %K below 20 (PDF p.2)."]),
        d("scevst_4", "Evens stochastics 4: crosses with %K or %D beyond 20/80 to open and to close", &[
            "E|\"OL: Cross(Stoch(5,1),Mov(Stoch(5,1),3,S)) AND (Stoch(5,1)<20 OR Mov(Stoch(5,1),3,S)<20) CL: Cross(Mov(Stoch(5,1),3,S), Stoch(5,1)) AND (Stoch(5,1)>80 OR Mov(Stoch(5,1),3,S)>80)\" (PDF p.2), shorts mirrored; \"5 Same as 4A, but with 3 for %K slowing.\"", "X|Close long on Cross(%D, %K) with %K or %D above 80; close short on Cross(%K, %D) with %K or %D below 20 (PDF p.2)."]),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let v = match id { "scevst_1" => "1: cross", "scevst_2" => "2: cross from 20/80", "scevst_3" => "3: 2 + exit beyond 80/20", "scevst_4" => "4: %K or %D beyond 20/80", _ => return None };
    Some(("scevst", "Evens stochastics comparisons (S&C)", v))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    reviewed(id).then(|| vec![p("slow", "%K slowing (1 = none; 5 = 'A' version; 3 = comparison 5)", 1., 1., 5., 1.)])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let k = ms_stoch(x, 5, p.get("slow").copied().unwrap_or(1.).round().clamp(1., 20.) as usize);
    let d = sma(&k, 3);
    let plan = id.as_bytes()[7];
    Some(ms_book(x, from, to, true, |t| {
        if t == 0 || !d[t - 1].is_finite() || !k[t - 1].is_finite() { return (false, false, false, false); }
        let up = k[t] > d[t] && k[t - 1] <= d[t - 1];
        let dn = d[t] > k[t] && d[t - 1] <= k[t - 1];
        let (low, high) = (k[t].min(k[t - 1]) < 20.0, k[t].max(k[t - 1]) > 80.0);
        let (kd_lo, kd_hi) = (k[t] < 20.0 || d[t] < 20.0, k[t] > 80.0 || d[t] > 80.0);
        match plan {
            b'1' => (up, dn, dn, up),
            b'2' => (up && low, dn, dn && high, up),
            b'3' => (up && low, k[t] > 80.0, dn && high, k[t] < 20.0),
            _ => (up && kd_lo, dn && kd_hi, dn && kd_hi, up && kd_lo),
        }
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn stochastic_plans() {
        let r = super::super::sctraub::tests::bars(2000);
        let x = Ctx::new(&r, &r, &[]);
        for id in REVIEWED {
            let t = run(id, &x, 0, r.len() - 1, &Params::new()).unwrap();
            assert!(!t.is_empty() && t.iter().all(|z| z.legs[0].px == x.o[z.legs[0].i]), "{id}");
        }
    }
}

// Instrument admission: S&C Evens stochastics 2026-11-02 (titles lane K16).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "scevst" && reviewed(def.id),
    label: "Stuart Evens compared the stochastic rules on 32 US stocks (1993-1999). Runs on indices, ETFs and other stocks, US and India, are a declared application.",
    basis: "Technical Analysis of Stocks & Commodities V.17:9 pp.392-398 (Stochastics), printed MetaStock code; independent check authors/titles-1030/K16w/k16_ref.py",
};
