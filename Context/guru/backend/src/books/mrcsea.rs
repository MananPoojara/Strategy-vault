//! Moore Research Center seasonal trade sheet, as printed in "Research Your Trade" (Steve Moore with Linda Raschke, TAG XV, 1993;
//! research share Trading Knowledge II/@INO (ino.com)/Linda Raschke & Moore - Researching your Trade/Raschke - Moore - Researching Your Trade.pdf,
//! 20-page scan, PDF page 14 "Moore September Trades"; p.17 "Mechanical vs. Filtered Trading"). Book id `mrcsea`. Lane CV2 (18 Nov 2026).
//! The sheet lists dated seasonal trades by contract; only the S&P 500 line is on a series the index engine holds. Other lines
//! (gold, yen, Swiss franc, bonds, grains, energy) need continuous futures and are not coded here.
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const REVIEWED: &[&str] = &["mrcsea_sp_sep_short"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Complete rule, labelled readings; UNVERIFIED (R4 #339/#343)] Printed on the September trade sheet (PDF p.14): '2 Sell S&P 500(CME)-Dec  9/03/93  9/12/93  82% win, 9 winning years, 2 losing years, 11 years, total profit 1966, average profit per day 196.59, average percent of daily margin 1.64'. p.17: 'Each trade may be entered at the close of the indicated date of entry, protected with a stop, and exited on the appropriate close.' Readings: (1) the 1993 dates are used as the calendar dates of every year (entry Sep 3, exit Sep 12); (2) a date that is not a trading day moves to the next trading day (Sep 12 1993 was a Sunday; the Trade Review sheet adjusts for weekends); (3) the p.17 stop is not given an amount for this trade (the $2,000 protective stop is printed only on the heating-oil strategy sheet, p.15), so no stop is applied; (4) the book trades the December S&P 500 future, here the cash index or ETF (instrument swap): the book's average profit of 1966 dollars at $500 per index point is 3.93 points. Book window: the sheet's 11 years of S&P futures, which started in April 1982 (contracts 1982-1992).";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "mrcsea_sp_sep_short", name: "Moore Research seasonal: short the S&P 500 at the Sep 3 close, cover at the Sep 12 close",
        group: "Research Your Trade (Moore/Raschke, TAG 1993) - September trade sheet", session: 0, dir: "Short", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: "mrcsea",
        rules: &["E|\"Sell S&P 500 (CME) - Dec, entry date 9/03/93\" (September trades sheet, PDF p.14); \"Each trade may be entered at the close of the indicated date of entry\" (p.17)",
            "X|\"exit date 9/12/93\"; \"exited on the appropriate close\" (p.17)"],
        notes: NOTE, claim: None }]
}
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    reviewed(id).then_some(("mrcsea_sp_sep_short", "Moore seasonal S&P September short", "Sep 3 - Sep 12"))
}
pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    reviewed(id).then(|| vec![p("em", "Entry month (book 9)", 9., 1., 12., 1.), p("ed", "Entry day of month (book 3)", 3., 1., 31., 1.),
        p("xm", "Exit month (book 9)", 9., 1., 12., 1.), p("xd", "Exit day of month (book 12)", 12., 1., 31., 1.)])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let g = |k: &str, d: f64| p.get(k).copied().unwrap_or(d).round() as u32;
    let (em, ed, xm, xd) = (g("em", 9.), g("ed", 3.), g("xm", 9.), g("xd", 12.));
    let md = |i: usize| -> (u32, u32) { let d = &x.r[i].date; (d[5..7].parse().unwrap_or(0), d[8..10].parse().unwrap_or(0)) };
    let year = |i: usize| &x.r[i].date[..4];
    let mut out: Vec<Trade> = vec![];
    let mut i = from;
    while i <= to {
        let y = year(i).to_string();
        let (mut e, mut xi, mut k) = (None, None, i);
        while k <= to && year(k) == y {
            if e.is_none() && md(k) >= (em, ed) { e = Some(k); }
            else if let Some(en) = e { if xi.is_none() && k > en && md(k) >= (xm, xd) { xi = Some(k); } }
            k += 1;
        }
        if let Some(en) = e {
            let (xj, open) = match xi { Some(v) => (v, false), None => (k - 1, true) };
            if xj > en {
                out.push(Trade { dir: -1., legs: vec![Leg { i: en, px: x.c[en], w: 1. }], exit_i: xj, exit_px: x.c[xj], reason: if open { "open".into() } else { "seasonal exit date (close)".into() },
                    entry_kind: "close", signal_i: xj, open, exits: vec![] });
            }
        }
        i = k;
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sept_short() {
        let r = super::super::sctraub::tests::bars(3000);
        let x = Ctx::new(&r, &r, &[]);
        let t = run("mrcsea_sp_sep_short", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(t.len() >= 5 && t.iter().all(|z| z.dir < 0. && x.r[z.legs[0].i].date[5..7] == *"09"));
    }
}

// Instrument admission: Moore Research seasonal sheet 2026-11-18 (lane CV2).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "mrcsea" && reviewed(def.id),
    label: "Moore Research trades the December S&P 500 future; the cash index (SPX), ETFs and other equity indices, US and India, are a declared application (instrument swap).",
    basis: "Moore/Raschke, Research Your Trade (TAG XV 1993), September trade sheet; independent check authors/convert-1118/CV2/ref/mrcsea_ref.py",
};
