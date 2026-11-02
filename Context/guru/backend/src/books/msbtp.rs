//! Balance Trend Point, "Sistema ZT" (an anonymous Spanish MetaStock formula set; research share USB HDD 1/MetaStock/@Systems &
//! Indicators/Balance Trend Point for MS/, formulas MS70FORM.DTA `_BTP-largos`, `_BTP-cortos`, `_posicion` and expert
//! experts/EXPT0001.DTA "BTP-Sistem"; text authors/batch-g/G1w/btp.txt). Book id `msbtp` (`btp` is Connors' Best Trading Patterns).
//! A close-only stop-and-reverse, coded as a labelled vendor sample. Lane K12 (titles loop, 2 Nov 2026), to-code row 192;
//! independent check authors/titles-1030/K12w/k12_ref.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::rbtie::target_trades;

/// Admitted ids (authors/titles-1030/K12w/k12_ref.py (header)).
pub const REVIEWED: &[&str] = &["msbtp_zt"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const ZT: &str = "[Declared readings] The MetaStock formulas, as written: FM = C - Ref(C,-2); BTPL = If(Ref(FM,-1) < FM, Ref(C,-1) + Ref(FM,-1), Ref(C,-1) + FM); BTPC = If(Ref(FM,-1) > FM, Ref(C,-1) + Ref(FM,-1), Ref(C,-1) + FM); BTP = If(C > PREV, BTPL, BTPC) with PREV = yesterday's BTP; posicion = If(BTP = BTPL, 1, -1), 1 = 'Comprado' (long), -1 = 'Vendido' (short). The expert reports the position at today's close and says it switches when a later close is below (long) / above (short) the BTP level. Read: the position changes at the close that shows the change, filled at that close (p.open = 1: the next open); always in, long and short (p.side = 1: long only, flat instead of short). PREV starts at 0 on the first bar where BTPL is defined (the fourth bar of the history, as MetaStock does), so that bar is long; when BTPL = BTPC the position is long, as the formula gives. A run starts with the first position change inside it. Not applied: the expert's stop (X - TR / X + TR) and target (2X - L / 2X - H) levels, which it prints as commentary only (X = (H + L + C) / 3, TR = true range). An anonymous MetaStock formula pack with no author, test or result; daily bars on indices, ETFs and US/India stocks are a declared application; one unit; the vault's standard costs.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "msbtp_zt", name: "Balance Trend Point (Sistema ZT): long while the close is above yesterday's BTP level, short below it, at the close", group: "Balance Trend Point MetaStock formulas (Spanish, anonymous)", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "msbtp", rules: &[
            "F|\"FM:=C-Ref(C,-2); BTPL:=If(Ref(FM,-1)<FM,Ref(C,-1)+Ref(FM,-1),Ref(C,-1)+FM); BTPC:=If(Ref(FM,-1)>FM,Ref(C,-1)+Ref(FM,-1),Ref(C,-1)+FM); BTP:=If(C>PREV,BTPL,BTPC);\" (MS70FORM.DTA)",
            "E|\"posicion:=If(btp=btpl,1,-1);\" 1 = Comprado (long), -1 = Vendido (short) (MS70FORM.DTA, expert BTP-Sistem)",
            "X|Stop and reverse: the position changes at the close where posicion changes (the expert: a close below the BTP level while long, above it while short)."],
        notes: ZT, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let id = REVIEWED.iter().find(|r| **r == id)?;
    Some((*id, "Balance Trend Point (MetaStock)", "close-only stop and reverse"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    reviewed(id).then(|| vec![p("open", "Fill: 0 = the close that shows the change, 1 = the next open", 0., 0., 1., 1.),
        p("side", "Sides: 0 = long and short (the formula), 1 = long only", 0., 0., 1., 1.)])
}

/// posicion per bar (0 before the fourth bar).
pub fn posicion(c: &[f64]) -> Vec<i8> {
    let mut out = vec![0i8; c.len()];
    let mut prev = 0.0;
    for i in 3..c.len() {
        let (fm, fm1) = (c[i] - c[i - 2], c[i - 1] - c[i - 3]);
        let btpl = if fm1 < fm { c[i - 1] + fm1 } else { c[i - 1] + fm };
        let btpc = if fm1 > fm { c[i - 1] + fm1 } else { c[i - 1] + fm };
        let btp = if c[i] > prev { btpl } else { btpc };
        out[i] = if btp == btpl { 1 } else { -1 };
        prev = btp;
    }
    out
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let get = |k: &str| p.get(k).copied().unwrap_or(0.) > 0.5;
    let mut tgt = posicion(&x.c);
    if get("side") { for t in tgt.iter_mut() { *t = (*t).max(0); } }
    Some(target_trades(x, &tgt, from, to, get("open"), "BTP switch"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn balance_trend_point() {
        // closes 10, 11, 13, 12: FM[2] = 3, FM[3] = 1: BTPL = 13 + 1 = 14, BTPC = 13 + 3 = 16; C = 12 > PREV 0 -> BTPL -> long
        assert_eq!(posicion(&[10., 11., 13., 12.]), vec![0, 0, 0, 1]);
        // bar 4: C = 11, FM = -2, FM[-1] = 1: BTPL = 12 - 2 = 10, BTPC = 12 + 1 = 13; 11 > PREV 14? no -> BTPC -> short
        assert_eq!(posicion(&[10., 11., 13., 12., 11.])[4], -1);
        let r: Vec<Row> = (0..800).map(|i| { let f = i as f64; let c = 100. + 9. * (f / 17.).sin() + 4. * (f / 5.3).cos();
            Row { ticker: "T".into(), date: format!("{:05}", i), open: c - 0.5, high: c + 1., low: c - 1., close: c, volume: None } }).collect();
        let x = Ctx::new(&r, &r, &[]);
        let t = run("msbtp_zt", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(t.len() > 10 && t.iter().any(|z| z.dir > 0.) && t.iter().any(|z| z.dir < 0.));
        for w in t.windows(2) { assert_eq!(w[1].legs[0].i, w[0].exit_i); }
        let mut lo = Params::new(); lo.insert("side".into(), 1.);
        assert!(run("msbtp_zt", &x, 0, r.len() - 1, &lo).unwrap().iter().all(|z| z.dir > 0.));
        assert!(family_of("msbtp_zt").is_some() && params("msbtp_zt").is_some() && run("msbtp_x", &x, 0, 5, &Params::new()).is_none());
    }
}

// Instrument admission: Balance Trend Point MetaStock formulas 2026-11-02 (titles lane K12).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "msbtp" && reviewed(def.id),
    label: "An anonymous Spanish MetaStock formula pack (Balance Trend Point, 'Sistema ZT') with no test or result. Runs on indices, ETFs and stocks, US and India, are a declared application.",
    basis: "Balance Trend Point for MS: MS70FORM.DTA formulas _BTP-largos, _BTP-cortos, _posicion and expert EXPT0001.DTA 'BTP-Sistem'; independent check authors/titles-1030/K12w/k12_ref.py",
};
