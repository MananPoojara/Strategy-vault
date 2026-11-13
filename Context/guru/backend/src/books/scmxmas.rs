//! John L. Momsen, "Christmas Comes Early With The DJIA", Technical Analysis of Stocks & Commodities V.19:1 (January 2001)
//! pp.32-37 (research share Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V19/C01/
//! 006XMAS.pdf). Book id `scmxmas`. Lane PA1 (13 Nov 2026, pool A of authors/a1/A1.tsv; R4 #297). Independent check
//! authors/pa1/pa1_ref.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::lc3::lo;
use super::tselhs::Book;

pub const REVIEWED: &[&str] = &["scmxmas_dip"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Adapted: 1 unstated detail, R4 #297; unverified] Hirsch's 'mid-December new lows' trade as Momsen tests it on the Dow 30 industrials: from December 15 (the next trading day if the 15th is not one) a buy limit one tick (0.01) below the previous day's low is placed each day until filled ('continuously setting your buy orders one tick below yesterday's lows until you get filled'); a limit fills at its level, or at the open when the day opens below it. Exit on January 7 (the next trading day if the 7th is not one; Momsen's choice over Hirsch's February 15, option exit = 2). Protective stop 4.5% below the entry price, filled at the stop or at a lower open; it works from the entry day (the limit is touched before any lower price that day, so the stop after the entry is not an ordering guess). One trade a year ('I allow only one trade per year'); no order is placed on or after the exit day. Unstated in the article: (1) the exit fill on the exit day; vault reading: that day's close. Options: n = buy one tick below the lowest low of the previous n days (Figure 2 tests 1-20; 1 = the article's choice); stop = 0 turns the stop off. Article (Dow 30 industrials cash, with the 4.5% stop, January 7 exit): in-sample 1920-89 79% profitable, average +2.73% a year; out-of-sample 1990-99 80% profitable, average +3.06% a year, average win +3.83%, average loss -1.12%; cash S&P 500 1982-99 78% profitable, average +2.46% (tables p.36-37). Runs on other indices, ETFs and stocks, US and India, are a declared application.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "scmxmas_dip", name: "Momsen Christmas dip: from Dec 15 buy one tick under yesterday's low, out Jan 7, 4.5% stop", group: "S&C: Christmas Comes Early With The DJIA (Momsen, V.19:1)", session: 0, dir: "Long", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "scmxmas", rules: &[
            "E|\"On December 15, I began to place orders one tick under yesterday's low\"; \"the greatest profits are made from continuously setting your buy orders one tick below yesterday's lows until you get filled\" (V.19:1 p.33, Figure 2)",
            "X|\"I prefer to enter trades with the highest degree of probability, so I set my exit date at January 7\" (p.35); \"if either the entry or exit date fell during a nontrading day, the action was initiated on the next trading day\" (p.33)",
            "X|\"This 4 1/2% risk is an excellent point for a protective stop\"; \"Due to the entry and protective stop used, I allow only one trade per year\" (p.36)"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    reviewed(id).then_some(("scmxmas_dip", "Momsen Christmas dip (S&C)", "Dec 15 - Jan 7"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    reviewed(id).then(|| vec![p("n", "Buy one tick under the lowest low of the previous n days (article 1)", 1., 1., 20., 1.),
        p("stop", "Protective stop, % below entry (article 4.5; 0 = off)", 4.5, 0., 20., 0.5),
        p("exit", "Exit date: 1 = January 7 (Momsen), 2 = February 15 (Hirsch)", 1., 1., 2., 1.),
        p("tick", "One tick (article 0.01)", 0.01, 0., 1., 0.01)])
}

/// (year, month, day) of an ISO date.
fn ymd(d: &str) -> (i64, i64, i64) {
    let f = |a: usize, b: usize| d.get(a..b).and_then(|s| s.parse().ok()).unwrap_or(0);
    (f(0, 4), f(5, 7), f(8, 10))
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let g = |k: &str, d: f64| p.get(k).copied().unwrap_or(d);
    let n = g("n", 1.).round().max(1.) as usize;
    let (stop, tick) = (g("stop", 4.5).max(0.) / 100., g("tick", 0.01).max(0.));
    let exit_md = if g("exit", 1.).round() >= 2. { (2, 15) } else { (1, 7) };
    let mut pos = Book::new();
    let mut traded_season: Option<i64> = None; // the December year whose trade is taken or open
    for i in from.max(n)..=to {
        let (y, m, d) = ymd(&x.r[i].date);
        // season year: Dec 15.. belongs to y; Jan..Feb to y-1
        let season = if m == 12 { y } else { y - 1 };
        let in_entry = (m == 12 && d >= 15) || ((m, d) < exit_md && m <= 2);
        let exit_day = (m <= 2 && (m, d) >= exit_md) || (3..=11).contains(&m);
        if pos.side() > 0. {
            let sl = pos.entry_px() * (1. - stop);
            if stop > 0. && x.l[i] <= sl { pos.exit(i, x.o[i].min(sl), "4.5% stop"); continue; }
            if exit_day { pos.exit(i, x.c[i], "exit date"); }
            continue;
        }
        if !in_entry || traded_season == Some(season) || i == 0 { continue; }
        let lv = lo(&x.l, i - n, i - 1) - tick;
        if x.l[i] <= lv {
            let fill = x.o[i].min(lv);
            pos.enter(1., i, fill, "limit");
            traded_season = Some(season);
            let sl = fill * (1. - stop);
            if stop > 0. && x.l[i] <= sl { pos.exit(i, sl, "4.5% stop"); }
        }
    }
    Some(pos.done(to, x.c[to]))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn one_december_dip_a_year() {
        let r = super::super::sctraub::tests::bars(4000);
        let x = Ctx::new(&r, &r, &[]);
        let t = run("scmxmas_dip", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(t.len() >= 8);
        let mut years = std::collections::BTreeSet::new();
        for z in &t {
            let e = z.legs[0].i;
            let (y, m, d) = ymd(&x.r[e].date);
            assert!((m == 12 && d >= 15) || (m == 1 && d < 7));
            assert!(years.insert(if m == 12 { y } else { y - 1 }));
            assert!(z.legs[0].px <= x.l[e - 1] - 0.01 + 1e-9);
            if !z.open && z.reason == "exit date" { let (_, m2, d2) = ymd(&x.r[z.exit_i].date); assert!(m2 == 1 && d2 >= 7); assert_eq!(z.exit_px, x.c[z.exit_i]); }
            if z.reason == "4.5% stop" { assert!(z.exit_px <= z.legs[0].px * 0.955 + 1e-9); }
        }
    }
}

// Instrument admission: S&C Momsen Christmas dip 2026-11-13 (lane PA1, R4 #297).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "scmxmas" && reviewed(def.id),
    label: "John Momsen tested the mid-December dip on the Dow 30 industrials (1920-99) and the S&P 500 (cash 1960-99, futures 1982-99); other indices, ETFs and stocks, US and India, are a declared application with one labelled reading (R4 #297).",
    basis: "Technical Analysis of Stocks & Commodities V.19:1 pp.32-37 (Christmas Comes Early With The DJIA); R4 #297; independent check authors/pa1/pa1_ref.py",
};
