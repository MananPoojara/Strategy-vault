//! "Guida all'uso di MetaStock" (Italian MetaStock user guide, no author named), Cap.8, section 8.3 "Come utilizzare l'esperto -
//! Versione Avanzata", steps e-g: the TrendRSI Expert Advisor worked example (research share USB HDD 1/MetaStock/@Knowledge/Manual
//! Metastock (Italiano)/Cap.8°.pdf, PDF p.6; ocr2 text authors/batch-e/E5w/ms_it_cap8_p5-6.txt). Book id `msitg`. A teaching
//! example (R4 decision 49). Lane K10 (titles loop, 1 Nov 2026), to-code row 177; independent check authors/titles-1030/K10w/k10_ref.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::msws::{rsi, simulate, Exec, Sig};

/// Admitted ids (authors/titles-1030/K10w/k10_ref.py (header)).
pub const REVIEWED: &[&str] = &["msitg_trendrsi"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] The guide's TrendRSI expert marks a 'Buy' arrow where RSI(14) < 30 and a 'Sell' arrow where RSI(14) > 70. Read as a long-only pair: buy at the close of a day with RSI(14) below 30 when flat, sell at the close of the first later day with RSI(14) above 70; repeat Buy arrows while long are ignored. The expert only marks bars, so the fill is the MetaStock System Tester default (the signal day's close, no delay); the close rule is checked from the day after entry. RSI = Wilder's (MetaStock). The 'Trend' ribbon (close above/below the 50-day average) and the RSI highlights are display only, not filters. No stop or target in the source. Option p.side = 1 adds the short side (sell short at RSI above 70, cover at RSI below 30, stop and reverse). A teaching example with no instrument and no results; indices, ETFs and US/India stocks are a declared application; vault standard costs.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "msitg_trendrsi", name: "MetaStock guide TrendRSI expert: buy at RSI(14) below 30, sell at RSI(14) above 70", group: "Guida all'uso di MetaStock (Expert Advisor)",
        session: 0, dir: "Long", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: "msitg", rules: &[
            "E|simbolo \"Buy\" con la formula: RSI(14)<30 e la grafica denominata \"Buy Arrow\" (Cap.8 p.6)",
            "X|simbolo \"Sell\" con la formula RSI(14)>70 e la grafica denominata \"Sell Arrow\" (Cap.8 p.6)"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    (id == "msitg_trendrsi").then_some(("msitg_trendrsi", "MetaStock guide TrendRSI", "RSI 14, 30/70"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    reviewed(id).then(|| vec![ParamDef { key: "side", label: "Sides: 0 = long only (default), 1 = long and short", def: 0., min: 0., max: 1., step: 1. }])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let r = rsi(&x.c, 14);
    let (buy, sell): (Vec<bool>, Vec<bool>) = (r.iter().map(|v| *v < 30.).collect(), r.iter().map(|v| *v > 70.).collect());
    let both = p.get("side").copied().unwrap_or(0.) > 0.5;
    let none = vec![false; buy.len()];
    let sig = Sig { el: buy.clone(), xl: sell.clone(), es: if both { sell } else { none.clone() }, xs: if both { buy } else { none } };
    Some(simulate(x, from, to, &sig, Exec::default()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn rsi_30_70_long_only_at_the_close() {
        let r: Vec<Row> = (0..600).map(|i| { let f = i as f64; let c = 100. + 10. * (f / 20.).sin();
            Row { ticker: "T".into(), date: format!("{:05}", i), open: c, high: c + 1., low: c - 1., close: c, volume: None } }).collect();
        let x = Ctx::new(&r, &r, &[]);
        let t = run("msitg_trendrsi", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(t.len() > 3 && t.iter().all(|z| z.dir > 0.));
        for z in &t { assert_eq!(z.legs[0].px, x.c[z.legs[0].i]); }
        let b = run("msitg_trendrsi", &x, 0, r.len() - 1, &Params::from([("side".to_string(), 1.)])).unwrap();
        assert!(b.iter().any(|z| z.dir < 0.));
        assert!(run("msitg_x", &x, 0, 5, &Params::new()).is_none());
    }
}

// Instrument admission: Italian MetaStock user guide (Guida all'uso di MetaStock) 2026-11-01 (titles lane K10).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "msitg" && reviewed(def.id),
    label: "A MetaStock Expert Advisor teaching example with no instrument and no results. Runs on indices, ETFs and stocks, US and India, are a declared application.",
    basis: "Guida all'uso di MetaStock, Cap.8 section 8.3 steps e-g (Cap.8°.pdf PDF p.6); independent check authors/titles-1030/K10w/k10_ref.py",
};
