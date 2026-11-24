//! Anna Krivelyova and Cesare Robotti, "Playing the Field: Geomagnetic Storms and International Stock Markets", Federal Reserve Bank of Atlanta
//! Working Paper 2003-5, Strategy II (pp.17-18) (research share USB HDD 2/tRADING KNO-2/Anna Krivelyova, C.Robotti - Playing the Field Geomagnetic Storms
//! and International Stock Markets (Article)/...pdf). Book id `krgms`. Lane K2 (24 Nov 2026, claimed P-tier row VB1025 / P85, full read).
//! Independent check authors/conv-1123/k2w/qp/krgms_ref.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::tselhs::Book;

pub const REVIEWED: &[&str] = &["krgms_calendar"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "A|[Declared readings, lane K2, 2] (1) The paper states the portfolio by calendar months, not by switch days, so the position changes at the close of the last trading day of the previous month (a position held for whole calendar months); the month of the next bar is calendar knowledge, not a price. (2) The T-bill leg of the short portfolio ('100% short in the market and 100% long in T-bills') earns nothing here: only the market leg is traded, the cash return is not added and no costs are charged. The paper tests the rule as a hedge against geomagnetic-storm seasonality and finds it profitable only in Canada; it is not a recommended system. Strategy I (long in 'good' days, T-bills in 'bad' days) needs the geomagnetic Ap series, which the vault does not hold, and is not coded. Run on the NSE indices as a labelled #340 adaptation, UNVERIFIED.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "krgms_calendar", name: "Geomagnetic-storm seasonal rule: short March-April and September-October, long the other eight months", group: "Krivelyova-Robotti geomagnetic storms (Strategy II)", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "krgms", rules: &[
            "E|\"a portfolio, 100% short in the market and 100% long in T-bills during March-April and September-October, and 100% long in the market the remaining eight months\" (Strategy II, p.17)",
            "X|The position is reversed at each month boundary of that calendar: no other exit, stop or target is printed (p.17-18)",
            NOTE], notes: NOTE, claim: None }]
}
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { reviewed(id).then_some(("krgms", "Geomagnetic-storm calendar (Atlanta Fed WP 2003-5)", "Mar-Apr, Sep-Oct short")) }
pub fn params(id: &str) -> Option<Vec<ParamDef>> { reviewed(id).then(Vec::new) }

fn month(d: &str) -> Option<u32> { d.get(5..7)?.parse().ok() }

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, _p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let side_of = |m: u32| if matches!(m, 3 | 4 | 9 | 10) { -1.0 } else { 1.0 };
    let mut pos = Book::new();
    for t in from..=to {
        // the position held from this close on: the month of the next bar (this bar's own month on the last bar)
        let m = month(&x.r[if t < to { t + 1 } else { t }].date)?;
        let want = side_of(m);
        if want != pos.side() {
            if pos.side() != 0.0 { pos.exit(t, x.c[t], "month boundary"); }
            pos.enter(want, t, x.c[t], "close");
        }
    }
    Some(pos.done(to, x.c[to]))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn switches_at_month_ends() {
        let r = super::super::sctraub::tests::bars(1500);
        let x = Ctx::new(&r, &r, &[]);
        let t = run("krgms_calendar", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(t.len() > 8);
        for w in t.windows(2) { assert_eq!(w[0].exit_i, w[1].legs[0].i); assert!(w[0].dir != w[1].dir); }
        for z in &t { if z.exit_i + 1 < r.len() { assert_ne!(month(&r[z.exit_i].date), month(&r[z.exit_i + 1].date)); } }
    }
    /// hand-run dump for the pandas reference: KRGMS_CSV=<idx_data.csv> KRGMS_OUT=<file> cargo test --release krgms_dump -- --ignored
    #[test]
    #[ignore]
    fn krgms_dump() {
        use std::io::Write;
        let data = crate::load_csv(&std::env::var("KRGMS_CSV").unwrap()).unwrap();
        let mut f = std::io::BufWriter::new(std::fs::File::create(std::env::var("KRGMS_OUT").unwrap()).unwrap());
        writeln!(f, "id,ticker,dir,entry_date,entry_px,exit_date,exit_px,open").unwrap();
        for (t, rows) in data.iter() {
            let x = Ctx::new(rows, &[], &[]);
            for z in run("krgms_calendar", &x, 0, rows.len() - 1, &Params::new()).unwrap() {
                writeln!(f, "krgms_calendar,{t},{},{},{},{},{},{}", z.dir, rows[z.legs[0].i].date, z.legs[0].px, rows[z.exit_i].date, z.exit_px, z.open).unwrap();
            }
        }
    }
}

// Instrument admission: Krivelyova-Robotti geomagnetic calendar 2026-11-24 (lane K2, P-tier).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "krgms" && reviewed(def.id),
    label: "The paper tests the rule on international stock indices against buy-and-hold (profitable only for Canada); run only on the five NSE index series the independent reference covers (the gate row allows only those, R4 #378), as a declared adaptation.",
    basis: "Krivelyova and Robotti, Playing the Field: Geomagnetic Storms and International Stock Markets, Atlanta Fed WP 2003-5, Strategy II pp.17-18; independent check authors/conv-1123/k2w/qp/krgms_ref.py",
};
