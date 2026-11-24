//! Thom Hartle, "The Average Directional Movement Index (ADX)", Technical Analysis of Stocks & Commodities V.9:3 (March 1991) pp.101-102
//! (research share Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V09/C03/ADX.pdf). Book id `schdadx`.
//! Lane K2 (24 Nov 2026, Q396: an article the 30 Oct keyword scan had marked NO-RULES; the full read finds the printed trading rule on p.2).
//! Independent check authors/conv-1123/k2w/q396/schdadx_ref.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::sctraub::dmi;
use super::tselhs::Book;

pub const REVIEWED: &[&str] = &["schdadx_di14"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "A|[Declared readings, lane K2, 3] (1) The article gives the rule in one sentence ('go long when the +DI14 crosses above -DI14 and go short when the -DI14 crosses over the +DI14') and no stop, so the system is always in the market after the first cross (stop and reverse). (2) Fills: the cross is known at the close, so the order fills at the NEXT bar's close (R4 #374). (3) 'Only trade those markets that are highest on the ADX scale' ranks markets against each other and has no cutoff, so it is not coded. The 14-day smoothed sums with the 13/14 decay are the article's own construction (Wilder); the sign of +DI minus -DI does not depend on the sum-versus-average scaling. Run on indices and stocks as a declared application (the figures show Microsoft and Apple); the article prints no results.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "schdadx_di14", name: "Hartle ADX system: +DI(14) crosses -DI(14), stop and reverse", group: "S&C: The Average Directional Movement Index (Hartle, V.9:3)", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "schdadx", rules: &[
            "F|\"+Directional Indicator14 = +DM14 / TR14; -Directional Indicator14 = -DM14 / TR14\"; the 14-day sums are carried forward as previous x 13/14 plus today's value (PDF p.2)",
            "E|\"The general rule for trading based on ADX is to go long when the +DI14 crosses above -DI14 and go short when the -DI14 crosses over the +DI14\" (PDF p.2, Figure 7)",
            "X|No separate exit: the opposite cross reverses the position (Figure 7: buy when +DI crosses above -DI, sell when it crosses back) [reading]",
            NOTE], notes: NOTE, claim: None }]
}
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { reviewed(id).then_some(("schdadx", "Hartle ADX / DI crossover (S&C)", "DI(14)")) }
pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    reviewed(id).then(|| vec![ParamDef { key: "n", label: "DI length, days (article: 14)", def: 14., min: 5., max: 40., step: 1. }])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let n = p.get("n").copied().unwrap_or(14.).round().max(2.) as usize;
    let (pd, md, _) = dmi(x, n);
    let mut pos = Book::new();
    let mut pend = 0.0;
    for t in from..=to {
        if pend != 0.0 { pos.exit(t, x.c[t], "reversed"); pos.enter(pend, t, x.c[t], "close"); pend = 0.0; }
        if t > from && t < to && pd[t].is_finite() && md[t].is_finite() && pd[t - 1].is_finite() && md[t - 1].is_finite() {
            let d = if pd[t] > md[t] && pd[t - 1] <= md[t - 1] { 1.0 } else if md[t] > pd[t] && md[t - 1] <= pd[t - 1] { -1.0 } else { 0.0 };
            if d != 0.0 && d != pos.side() { pend = d; }
        }
    }
    Some(pos.done(to, x.c[to]))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn crosses_reverse_at_the_next_close() {
        let r = super::super::sctraub::tests::bars(1500);
        let x = Ctx::new(&r, &r, &[]);
        let t = run("schdadx_di14", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(t.len() > 10);
        for z in &t { assert_eq!(z.legs[0].px, r[z.legs[0].i].close); }
        for w in t.windows(2) { assert_eq!(w[0].exit_i, w[1].legs[0].i); assert!(w[0].dir != w[1].dir); }
        assert!(run("schdadx_di14", &x, 5, 5, &Params::new()).is_none());
    }
    /// hand-run dump for the pandas reference: SCHDADX_CSV=<idx_data.csv> SCHDADX_OUT=<file> cargo test --release schdadx_dump -- --ignored
    #[test]
    #[ignore]
    fn schdadx_dump() {
        use std::io::Write;
        let data = crate::load_csv(&std::env::var("SCHDADX_CSV").unwrap()).unwrap();
        let mut f = std::io::BufWriter::new(std::fs::File::create(std::env::var("SCHDADX_OUT").unwrap()).unwrap());
        writeln!(f, "id,ticker,dir,entry_date,entry_px,exit_date,exit_px,open").unwrap();
        for (t, rows) in data.iter() {
            let x = Ctx::new(rows, &[], &[]);
            for z in run("schdadx_di14", &x, 0, rows.len() - 1, &Params::new()).unwrap() {
                writeln!(f, "schdadx_di14,{t},{},{},{},{},{},{}", z.dir, rows[z.legs[0].i].date, z.legs[0].px, rows[z.exit_i].date, z.exit_px, z.open).unwrap();
            }
        }
    }
}

// Instrument admission: S&C Hartle ADX system 2026-11-24 (lane K2, Q396).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "schdadx" && reviewed(def.id),
    label: "Hartle's figures show Microsoft and Apple and the text speaks of markets in general; the article prints no results. Runs on indices, ETFs and stocks, US and India, are a declared application.",
    basis: "Technical Analysis of Stocks & Commodities V.9:3 pp.101-102 (The Average Directional Movement Index); independent check authors/conv-1123/k2w/q396/schdadx_ref.py",
};
