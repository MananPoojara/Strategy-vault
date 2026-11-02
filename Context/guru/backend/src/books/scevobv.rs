//! Stuart Evens, "On-Balance Volume", Technical Analysis of Stocks & Commodities V.17:5 (May 1999) pp.234-240 (research share
//! Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V17/C05/036OBV.pdf). Book id `scevobv`:
//! Granville's net field trend, the printed spreadsheet columns F-P. Lane K16 (titles loop, 2 Nov 2026), to-code row 252;
//! independent check authors/titles-1030/K16w/k16_ref.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::sctraub::next_open;

pub const REVIEWED: &[&str] = &["scevobv_nft"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] The printed spreadsheet (PDF p.3), started on the first day of the test window as the spreadsheet is ('Cells prior to the ones designated below are zero-filled with the exception of cell F1'): F = OBV (unchanged close keeps it, a lower close subtracts the day's volume, a higher close adds it); G/H mark a local OBV valley/peak one day late; I/J hold the last valley/peak value; K = -1 when OBV falls below the previous last-valley value, +1 when it rises above the previous last-peak value; L/M count the down/up designations to two (the formulas never reset them); N/O = whether the last valley/peak value fell (-1) or rose (+1); P, the net field trend, = 1 when both rose, -1 when both fell, else 0. 'we will go long and close short positions when column P is a 1 and go short and close long positions when it is a -1 ... and go flat when column P is a zero', at 'the opening price on the day following the signal'. The volume scaling (column E = MetaStock volume x 100) does not change any signal. The minor components (velocity, relative volume) are not used, as in the spreadsheet. Article: Merck, 1/1993-12/1998, 161 trades signalled. Needs daily volume; runs on indices, ETFs and stocks with volume, US and India, as a declared application.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "scevobv_nft", name: "Evens Granville OBV net field trend: long while it rises, short while it falls, flat when undetermined, next open", group: "S&C: On-Balance Volume (Evens, V.17:5)", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: true, needs_vix: false, needs_series: &[], book: "scevobv", rules: &[
            "F|\"F2: =IF(B1=B2,F1,IF(B1>B2,F1-E2,F1+E2)) G3: =IF(AND(F2<F1,F2<F3),-1,0) H3: =IF(AND(F2>F1,F2>F3),1,0) I4: =IF(G4=-1,F3,I3) J4: =IF(H4=1,F3,J3) K3: =IF(AND(OR(I3>0,I3<0),OR(J3>0,J3<0),F3<I2),-1, IF(...,F3>J2),1,0))\" (V.17:5, PDF p.3)",
            "F|\"L2: =IF(AND(K2=-1,L1=0),1,IF(AND(K2=-1,L1=1),2,L1)) ... N2: =IF(AND(L2=2,M2=2,I2<I1),-1, IF(AND(L2=2,M2=2,I2>I1),1,N1)) ... P2: =IF(AND(N2=1,O2=1),1,IF(AND(N2=-1,O2=-1),-1,0))\" (PDF p.3)",
            "E|\"we will go long and close short positions when column P is a 1 and go short and close long positions when it is a -1.\" (PDF p.3)",
            "X|\"We will close all open positions, and go flat when column P is a zero.\" Trades \"at the opening price on the day following the signal\" (PDF p.3)"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    reviewed(id).then_some(("scevobv_nft", "Evens OBV net field trend (S&C)", "Spreadsheet columns F-P"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> { reviewed(id).then(Vec::new) }

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, _p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let n = to - from + 1;
    let v = |i: usize| x.r[from + i].volume.unwrap_or(f64::NAN);
    let c = |i: usize| x.c[from + i];
    let mut f = vec![0.0; n];
    f[0] = v(0);
    for i in 1..n { f[i] = if c(i - 1) == c(i) { f[i - 1] } else if c(i - 1) > c(i) { f[i - 1] - v(i) } else { f[i - 1] + v(i) }; }
    let (mut g, mut h, mut iv, mut jv, mut k) = (vec![0i8; n], vec![0i8; n], vec![0.0; n], vec![0.0; n], vec![0i8; n]);
    let (mut l, mut m, mut nn, mut o, mut pp) = (vec![0u8; n], vec![0u8; n], vec![0i8; n], vec![0i8; n], vec![0i8; n]);
    for i in 1..n {
        if i >= 2 {
            g[i] = if f[i - 1] < f[i - 2] && f[i - 1] < f[i] { -1 } else { 0 };
            h[i] = if f[i - 1] > f[i - 2] && f[i - 1] > f[i] { 1 } else { 0 };
        }
        if i >= 3 {
            iv[i] = if g[i] == -1 { f[i - 1] } else { iv[i - 1] };
            jv[i] = if h[i] == 1 { f[i - 1] } else { jv[i - 1] };
        }
        if i >= 2 {
            let both = iv[i] != 0.0 && jv[i] != 0.0;
            k[i] = if both && f[i] < iv[i - 1] { -1 } else if both && f[i] > jv[i - 1] { 1 } else { 0 };
        }
        l[i] = if k[i] == -1 && l[i - 1] == 0 { 1 } else if k[i] == -1 && l[i - 1] == 1 { 2 } else { l[i - 1] };
        m[i] = if k[i] == 1 && m[i - 1] == 0 { 1 } else if k[i] == 1 && m[i - 1] == 1 { 2 } else { m[i - 1] };
        let cl = l[i] == 2 && m[i] == 2;
        nn[i] = if cl && iv[i] < iv[i - 1] { -1 } else if cl && iv[i] > iv[i - 1] { 1 } else { nn[i - 1] };
        o[i] = if cl && jv[i] < jv[i - 1] { -1 } else if cl && jv[i] > jv[i - 1] { 1 } else { o[i - 1] };
        pp[i] = if nn[i] == 1 && o[i] == 1 { 1 } else if nn[i] == -1 && o[i] == -1 { -1 } else { 0 };
    }
    Some(next_open(x, from, to, "net field trend undetermined", |t, _| pp[t - from] as f64))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn net_field_trend_trades() {
        let r = super::super::sctraub::tests::bars(3000);
        let x = Ctx::new(&r, &r, &[]);
        let t = run("scevobv_nft", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(t.iter().all(|z| z.legs[0].px == x.o[z.legs[0].i]));
    }
}

// Instrument admission: S&C Evens OBV 2026-11-02 (titles lane K16).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "scevobv" && reviewed(def.id),
    label: "Stuart Evens tested Granville's net field trend on Merck stock (1993-1998). Runs on indices, ETFs and other stocks with volume, US and India, are a declared application.",
    basis: "Technical Analysis of Stocks & Commodities V.17:5 pp.234-240 (On-Balance Volume), printed spreadsheet formulas; independent check authors/titles-1030/K16w/k16_ref.py",
};
