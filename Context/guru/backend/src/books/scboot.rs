//! Martin Boot, "Trend-Following With The ADX", Technical Analysis of Stocks & Commodities V.18:9 (September 2000) pp.64-69
//! (research share Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V18/C09/084ADX.pdf).
//! Book id `scboot`: the single-entry ADX system. Lane K13 (titles loop, 2 Nov 2026), to-code row 206; independent check
//! authors/titles-1030/K13w/k13_ref.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::sctraub::dmi;
use super::tselhs::Book;

pub const REVIEWED: &[&str] = &["scboot_adx"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] Wilder ADX(14). Buy at the close when ADX(14) < 21 and above yesterday's ADX (the printed MetaStock line). The printed exit Cross(35,ADX(14)) is MetaStock's 'first series crosses above the second': 35 rises above the ADX, i.e. the ADX falls back under 35 (default; the article: 'If the point of the slowdown can be determined, this could be used as the exit point'). The ADX crossing up through 35 is an option (the reader's first reading). Fills at the signal close (MetaStock's default; the article gives none); the 10 points per contract cost is not applied; one position (the article's multiple-entry variants, up to 50 open positions, are not coded). Article, DJIA 1995 to 3 August 1999: six trades, five winners, 2,776 points, open trade +142.5; ADX < 15 from 17 August 1995 to 5 August 1999: four trades, three winners, 1,640 points. Our DJI runs: default from 1996, 7 closed trades (6 winners) and the open trade +142.5; the ADX < 15 test gives 4 trades, all winners, 3,195 points (default exit) or 3 of 4 winners, 1,806 points (cross-up exit). The book's AEX is not held. Runs on indices, ETFs and stocks, US and India, are a declared application.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "scboot_adx", name: "Boot ADX trend system: buy when ADX(14) under 21 turns up; sell when the ADX falls back under 35", group: "S&C: Trend-Following With The ADX (Boot, V.18:9)", session: 0, dir: "Long", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "scboot", rules: &[
            "E|\"Enter long: ADX(14) < 21 and ADX(14) > ref(ADX(14),-1)\" (V.18:9 p.65)",
            "X|\"Close long: Cross(35,ADX(14))\" (p.65); \"Normally, ADX should move between 20 and 40, so I took 35 as the exit point.\""],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let id = REVIEWED.iter().find(|r| **r == id)?;
    Some((*id, "Boot ADX trend-following (S&C)", "ADX < 21 turning up, exit at 35"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    reviewed(id).then(|| vec![p("lvl", "Entry: ADX below (21 printed; 15 = the article's tighter test)", 21., 10., 30., 1.),
        p("xup", "Exit (0 = ADX falls under 35, MetaStock Cross(35,ADX); 1 = ADX rises through 35)", 0., 0., 1., 1.)])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let (_, _, a) = dmi(x, 14);
    let lvl = p.get("lvl").copied().unwrap_or(21.0);
    let xup = p.get("xup").copied().unwrap_or(0.0) >= 0.5;
    let mut pos = Book::new();
    for i in from.max(1)..=to {
        if !(a[i].is_finite() && a[i - 1].is_finite()) { continue; }
        let out = if xup { a[i] > 35.0 && 35.0 >= a[i - 1] } else { a[i] < 35.0 && 35.0 <= a[i - 1] };
        if pos.side() > 0.0 && out { pos.exit(i, x.c[i], "ADX crossed 35"); }
        if pos.side() == 0.0 && a[i] < lvl && a[i] > a[i - 1] { pos.enter(1.0, i, x.c[i], "close"); }
    }
    Some(pos.done(to, x.c[to]))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn adx_system_long_only() {
        let r = super::super::sctraub::tests::bars(3000);
        let x = Ctx::new(&r, &r, &[]);
        let t = run("scboot_adx", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(!t.is_empty() && t.iter().all(|z| z.dir > 0.0));
    }
}

// Instrument admission: S&C Boot ADX 2026-11-02 (titles lane K13).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "scboot" && reviewed(def.id),
    label: "Martin Boot tested the ADX system on the AEX and the DJIA (1995-1999). Runs on other indices, ETFs and stocks, US and India, are a declared application.",
    basis: "Technical Analysis of Stocks & Commodities V.18:9 pp.64-69 (Trend-Following With The ADX); independent check authors/titles-1030/K13w/k13_ref.py",
};
