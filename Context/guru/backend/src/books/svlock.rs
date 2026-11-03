//! Sylvain Vervoort, LOCKIT System for MetaStock (traderslibrary.com): the System Tester record 'SVE_ST_Cross_ZLCl_ZLHa' in
//! MS10PRFT.DTA of Vervoort.exe (research share USB HDD 1/MetaStock/@Systems & Indicators/Sylvain Vervoort's LOCKIT System
//! (traderslibrary.com)/Vervoort.exe; text and slot dump authors/batch-x/keep/1850/). Book id `svlock`. Lane K30 (titles loop,
//! 3 Nov 2026), to-code row 371, lead decision 181; independent check authors/titles-1030/K30w/k30_ref.py.
//! The record uses PREV and Tema, which the shared msws interpreter lacks, so it is written out here; fills are msws's.
use crate::books::msws;
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::rrsim::sdef;

pub const REVIEWED: &[&str] = &["svlock_zlcl_zlha"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] The record's two formulas, as stored. avg = 65. Zero-lag TEMA of the typical price: E1 = Tema((H+L+C)/3, 65), E2 = Tema(E1, 65), ZlCl = E1 + (E1 - E2). Heikin-Ashi: haOpen = (yesterday's (O+H+L+C)/4 + PREV)/2, haC = ((O+H+L+C)/4 + haOpen + Max(H, haOpen) + Min(L, haOpen))/4; ZlHa = the same zero-lag TEMA of haC. Buy when ZlCl crosses above ZlHa, sell (close the long) when ZlHa crosses above ZlCl. Of the record's ten 2,500-byte order fields, slot 0 (Buy) and slot 3 (Sell) hold these formulas; slot 6 (the Sell Short order, by the field order of the BTTK 2.0 record of the same format) holds only '100', read as a MetaStock default value and not applied; the rest are empty. So a long-only test. 'a:=opt1' is unused (OPT1 1..1). Tema = MetaStock TEMA = 3 EMA - 3 EMA(EMA) + EMA(EMA(EMA)), each EMA = Mov(x,65,E) of the msws engine (seeded with its first value, shown from its 65th bar); PREV = 0 on the first bar of haOpen (bar 2 of the data, the first with a yesterday). The Heikin-Ashi average reads the open. Fills: the vault's MetaStock System Tester convention (msws engine): signal and fill at the close; the long closes only by the Sell formula, from the bar after entry; one position, one unit. No stops; no test printed. Runs on daily indices, ETFs and stocks, US and India, are a declared application.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![sdef("svlock", "svlock_zlcl_zlha", "Vervoort LOCKIT SVE_ST_Cross_ZLCl_ZLHa: long when the zero-lag 65 TEMA of the typical price crosses above the zero-lag 65 TEMA of the Heikin-Ashi close, out on the cross back",
        "LOCKIT System for MetaStock (Vervoort, traderslibrary.com)", "Long", Status::Live, &[
            r##"E|Buy (slot 0): "a:=opt1; avg:= 65; EMA1:= Tema((H+L+C)/3,avg); EMA2:= Tema(EMA1,avg); Difference:= EMA1 - EMA2; ZlCl:= EMA1 + Difference; haOpen:=(Ref((O+H+L+C)/4,-1) + PREV)/2; haC:=((O+H+L+C)/4+haOpen+Max(H,haOpen)+Min(L,haOpen))/4; EMA1:= Tema(haC,avg); EMA2:= Tema(EMA1,avg); Difference:= EMA1 - EMA2; ZlHa:= EMA1 + Difference; Cross(ZlCl,ZlHa)""##,
            r##"X|Sell (slot 3): the same lines (without a:=opt1) ending "Cross(ZlHa,ZlCl)""##,
            r##"F|Slot 6 holds only "100" (the Sell Short order field; not applied); the other order fields are empty"##],
        NOTE)]
}
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    reviewed(id).then_some(("svlock_zlcl_zlha", "Vervoort ZLCl / ZLHa TEMA cross (LOCKIT)", "TEMA 65"))
}
pub fn params(id: &str) -> Option<Vec<ParamDef>> { reviewed(id).then(Vec::new) }

fn tema(v: &[f64], n: usize) -> Vec<f64> {
    let e1 = msws::ema(v, n); let e2 = msws::ema(&e1, n); let e3 = msws::ema(&e2, n);
    (0..v.len()).map(|i| 3.0 * e1[i] - 3.0 * e2[i] + e3[i]).collect()
}
fn zero_lag(v: &[f64], n: usize) -> Vec<f64> {
    let e1 = tema(v, n); let e2 = tema(&e1, n);
    (0..v.len()).map(|i| e1[i] + (e1[i] - e2[i])).collect()
}
/// (ZlCl, ZlHa) of the record.
pub fn lines(x: &Ctx) -> (Vec<f64>, Vec<f64>) {
    let n = x.c.len();
    let tp: Vec<f64> = (0..n).map(|i| (x.h[i] + x.l[i] + x.c[i]) / 3.0).collect();
    let a4: Vec<f64> = (0..n).map(|i| (x.o[i] + x.h[i] + x.l[i] + x.c[i]) / 4.0).collect();
    let mut ha = vec![f64::NAN; n];
    for i in 1..n { ha[i] = (a4[i - 1] + if i >= 2 { ha[i - 1] } else { 0.0 }) / 2.0; }
    let hac: Vec<f64> = (0..n).map(|i| (a4[i] + ha[i] + x.h[i].max(ha[i]) + x.l[i].min(ha[i])) / 4.0).collect();
    (zero_lag(&tp, 65), zero_lag(&hac, 65))
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, _p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let (zc, zh) = lines(x);
    let n = x.c.len();
    let f = |a: &[f64], b: &[f64]| -> Vec<bool> { (0..n).map(|i| i >= 1 && [a[i], b[i], a[i - 1], b[i - 1]].iter().all(|v| v.is_finite()) && a[i] > b[i] && a[i - 1] <= b[i - 1]).collect() };
    let sig = msws::Sig { el: f(&zc, &zh), xl: f(&zh, &zc), es: vec![false; n], xs: vec![false; n] };
    Some(msws::simulate(x, from, to, &sig, msws::EXEC_DEFAULT))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn zlcl_zlha_cross() {
        let r = super::super::nijat::tests::bars(3000);
        let x = Ctx::new(&r, &r, &[]);
        let t = run("svlock_zlcl_zlha", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(t.len() >= 3 && t.iter().all(|z| z.dir > 0. && z.legs[0].px == r[z.legs[0].i].close));
        let (zc, zh) = lines(&x);
        // TEMA of TEMA: undefined until 6 x 64 bars after the first input; haOpen from bar 1 seeded with PREV = 0
        assert!(zc[383].is_nan() && zc[384].is_finite() && zh[384].is_nan() && zh[385].is_finite());
        for z in &t { let i = z.legs[0].i; assert!(zc[i] > zh[i] && zc[i - 1] <= zh[i - 1]); }
        assert!(run("svlock_zlcl_zlha", &x, 5, 5, &Params::new()).is_none());
    }
}

// Instrument admission: Vervoort LOCKIT system test 2026-11-03 (titles lane K30).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "svlock" && reviewed(def.id),
    label: "Sylvain Vervoort's LOCKIT System Tester record names no market and prints no test; runs on daily indices, ETFs and stocks, US and India, are a declared application.",
    basis: "LOCKIT System for MetaStock (Vervoort, traderslibrary.com): MS10PRFT.DTA record 'SVE_ST_Cross_ZLCl_ZLHa' in Vervoort.exe (research share USB HDD 1/MetaStock/@Systems & Indicators; slots read by batch X, authors/batch-x/keep/1850/); independent check authors/titles-1030/K30w/k30_ref.py",
};
