//! Equis International, "MetaStock Formula Primer" (2002; research share USB HDD 1/MetaStock/@Knowledge/Equis International -
//! Formula Primer Book/Formula Primer Metastock.pdf, PDF p.77-79 = printed p.72-74; text authors/batch-e/E7w/
//! equis_formula_primer_p68-78.txt). Book id `eqfp`. Ch.7's System Tester worked example, a labelled teaching example (R4
//! decisions 49, 50). Lane K11 (titles loop, 1 Nov 2026), to-code row 180; independent check authors/titles-1030/K11w/k11_ref.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::msws::{simulate, sma, Exec, Sig};

/// Admitted ids (authors/titles-1030/K11w/k11_ref.py (header)).
pub const REVIEWED: &[&str] = &["eqfp_ma200_21_40"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] The primer's System Tester example (printed p.73): Enter Long when the close is above its 200-day simple average and the 21-day average crosses above the 40-day; Exit Long when the 21-day crosses back below the 40-day; Enter Short when the close is below the 200-day average and the 21-day crosses below the 40-day; Exit Short when the 21-day crosses back above. Cross(a,b) = a above b today and at or below it yesterday. The primer does not state the tester's fill: the MetaStock System Tester default is used (the signal day's close, no delay), as on the other MetaStock cards. One position: the cross that exits a long also enters a short on the same close when the close is below the 200-day average (and the mirror), as the tester processes an exit and the opposite entry. The primer's Formula 39 'sell signal' on p.72 prints C > Mov(C,200,S), the System Tester listing on p.73 prints C < Mov(C,200,S); the listing is used. Option p.len = 1: the lengths 150, 26 and 9 of p.74 ('assume you find the optimum lengths for this system are 150, 26 and 9'), i.e. 9 crossing 26 with a 150-day filter, an illustration and not a tested result. No stop or target. No instrument or bar size is named: daily bars on indices, ETFs and US/India stocks are a declared application. A teaching example: the primer teaches formula writing and does not recommend the system (lead decision 49). No printed results; the vault's standard costs (decision 50).";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "eqfp_ma200_21_40", name: "Formula Primer system test: 21/40-day average cross in the direction of the 200-day average, long and short", group: "MetaStock Formula Primer (System Tester example)",
        session: 0, dir: "Long/Short", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: "eqfp", rules: &[
            "E|\"Enter Long: C > Mov( C, 200, S ) AND Cross( Mov( C, 21, S ), Mov( C, 40, S ) )\"; \"Enter Short: C < Mov( C, 200, S ) AND Cross( Mov( C, 40, S ), Mov( C, 21, S ) )\" (p.73)",
            "X|\"Exit Long: Cross( Mov( C, 40, S ), Mov( C, 21, S ) )\"; \"Exit Short: Cross( Mov( C, 21, S ), Mov( C, 40, S ) )\" (p.73)"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    (id == "eqfp_ma200_21_40").then_some(("eqfp_ma200_21_40", "MetaStock Formula Primer system test", "200 filter, 21/40 cross"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    reviewed(id).then(|| vec![ParamDef { key: "len", label: "Lengths: 0 = 200/21/40 (system test), 1 = 150/9/26 (p.74 optimisation illustration)", def: 0., min: 0., max: 1., step: 1. }])
}

fn cross(a: &[f64], b: &[f64], i: usize) -> bool { i > 0 && a[i] > b[i] && a[i - 1] <= b[i - 1] }

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let c = &x.c;
    let (lt, s, m) = if p.get("len").copied().unwrap_or(0.) > 0.5 { (150, 9, 26) } else { (200, 21, 40) };
    let (t, a, b) = (sma(c, lt), sma(c, s), sma(c, m));
    let all = |f: &dyn Fn(usize) -> bool| -> Vec<bool> { (0..c.len()).map(f).collect() };
    let (up, dn) = (all(&|i| cross(&a, &b, i)), all(&|i| cross(&b, &a, i)));
    let sig = Sig { el: all(&|i| c[i] > t[i] && up[i]), xl: dn.clone(), es: all(&|i| c[i] < t[i] && dn[i]), xs: up };
    Some(simulate(x, from, to, &sig, Exec::default()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn primer_system_test_at_the_close() {
        let r: Vec<Row> = (0..2500).map(|i| { let f = i as f64; let c = 100. + 30. * (f / 300.).sin() + 6. * (f / 23.).sin() + 0.01 * f;
            Row { ticker: "T".into(), date: format!("{:05}", i), open: c, high: c + 1., low: c - 1., close: c, volume: None } }).collect();
        let x = Ctx::new(&r, &r, &[]);
        for prm in [Params::new(), Params::from([("len".to_string(), 1.)])] {
            let t = run("eqfp_ma200_21_40", &x, 0, r.len() - 1, &prm).unwrap();
            assert!(t.len() > 3 && t.iter().any(|z| z.dir > 0.) && t.iter().any(|z| z.dir < 0.));
            for z in &t { assert_eq!(z.legs[0].px, x.c[z.legs[0].i]); }
        }
        assert!(family_of("eqfp_ma200_21_40").is_some() && run("eqfp_x", &x, 0, 5, &Params::new()).is_none());
    }
}

// Instrument admission: Equis International, MetaStock Formula Primer 2026-11-01 (titles lane K11).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "eqfp" && reviewed(def.id),
    label: "A teaching example from Equis International's MetaStock Formula Primer, with no instrument and no results. Runs on daily indices, ETFs and stocks, US and India, are a declared application.",
    basis: "MetaStock Formula Primer (Equis, 2002), ch.7 Formulas 38-39 and the System Tester listing, PDF p.77-79 (printed p.72-74); independent check authors/titles-1030/K11w/k11_ref.py",
};
