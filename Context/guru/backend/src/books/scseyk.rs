//! Ed Seykota & Dave Druz, "Determining Optimal Risk", Technical Analysis of Stocks & Commodities V.11:3 (March 1993) pp.122-124,
//! sidebar "System test" (research share Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/
//! V11/C03/SIDESYS.pdf, parent DETERMI.pdf). Book id `scseyk`. Lane K40 (4 Nov 2026, decision #203); independent check
//! authors/titles-1030/K40w/k40_ref.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::lc3::Pos;
use super::scfavw::{ge, le};
use super::sctraub::week_groups;

pub const REVIEWED: &[&str] = &["scseyk_box"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] Weekly boxes from calendar weeks (Monday-based): during a week the 20-week range = the highest high and lowest low of the 20 completed weeks before it, the 3-week range the same over 3 weeks ('updated weekly'). Entry 'on stop close only, 2 ticks beyond the 20 week price range': a daily close at or above the 20-week high + 2 ticks buys at that close, a close at or below the 20-week low - 2 ticks sells short at that close. Exit 'on stop 2 ticks beyond the three-week price range': a plain stop (the sidebar says 'close only' for the entry and not for the exit) at the 3-week low - 2 ticks for a long, the 3-week high + 2 ticks for a short, from the day after the entry, filled at the level or the open beyond it; option exit = 1 exits only on a close beyond the level, at that close. After an exit, the same close may enter again. Ticks = 2 ticks of the traded instrument (0.01 US, 0.05 NSE; decision #203). The sizing rule (equity x heat / instruments / three-week risk) is a money-management overlay and is not applied (one unit). Original test: soybean oil, live cattle, sugar, gold and Swiss francs, 12/19/1979-1/28/1992, a heat test with no per-trade statistics, so no book comparison. Runs on indices, ETFs and stocks, US and India, are a declared application.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "scseyk_box", name: "Seykota / Druz 20-week / 3-week box: enter on a close 2 ticks beyond the 20-week range, exit on a stop 2 ticks beyond the 3-week range", group: "S&C: Determining Optimal Risk (Seykota & Druz, V.11:3)", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "scseyk", rules: &[
            "F|\"Trading system: Enter positions on stop close only, 2 ticks beyond the 20 week price range, updated weekly.\" (sidebar 'System test', p.122)",
            "E|\"Enter positions on stop close only, 2 ticks beyond the 20 week price range\"; \"(2-tick breakout of the 20-week range SCO)\" (sidebar)",
            "X|\"Exit on stop 2 ticks beyond the three-week price range, also updated weekly.\"; \"the three-week low is $2,500 per contract away\" (entry risk, sidebar)"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { reviewed(id).then_some(("scseyk_box", "Seykota / Druz box system (S&C)", "20-week / 3-week, 2 ticks")) }

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    reviewed(id).then(|| vec![p("entry", "Entry box, weeks (book 20)", 20., 2., 104., 1.),
        p("exit_w", "Exit box, weeks (book 3)", 3., 1., 52., 1.),
        p("tick", "Ticks beyond the box (book 2)", 2., 0., 20., 1.),
        p("exit", "Exit: 0 = plain stop (sidebar); 1 = stop close only", 0., 0., 1., 1.)])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(f64::NAN, |d| d.def));
    let (ne, nx, tk, close_exit) = (g("entry").round().max(1.) as usize, g("exit_w").round().max(1.) as usize, g("tick") * x.market_tick(), g("exit").round() == 1.);
    let wg = week_groups(x);
    let wh: Vec<f64> = wg.iter().map(|&(a, b)| x.h[a..=b].iter().copied().fold(f64::NEG_INFINITY, f64::max)).collect();
    let wl: Vec<f64> = wg.iter().map(|&(a, b)| x.l[a..=b].iter().copied().fold(f64::INFINITY, f64::min)).collect();
    let box_of = |k: usize, n: usize| (k >= n).then(|| (wh[k - n..k].iter().copied().fold(f64::NEG_INFINITY, f64::max), wl[k - n..k].iter().copied().fold(f64::INFINITY, f64::min)));
    let mut pos = Pos::new();
    for (k, &(a, b)) in wg.iter().enumerate() {
        if a > to { break; }
        let (eb, xb) = (box_of(k, ne), box_of(k, nx));
        for t in a.max(from)..=b.min(to) {
            if let Some((xh, xl)) = xb {
                let (ls, ss) = (xl - tk, xh + tk);
                let held = pos.entry_bar().is_some_and(|e| e < t);
                if held && pos.side() > 0. {
                    if close_exit { if le(x.c[t], ls) { pos.exit(t, x.c[t], "close below the 3-week low"); } }
                    else if le(x.l[t], ls) { pos.exit(t, x.o[t].min(ls), "below the 3-week low"); }
                } else if held && pos.side() < 0. {
                    if close_exit { if ge(x.c[t], ss) { pos.exit(t, x.c[t], "close above the 3-week high"); } }
                    else if ge(x.h[t], ss) { pos.exit(t, x.o[t].max(ss), "above the 3-week high"); }
                }
            }
            if let Some((eh, el)) = eb {
                if pos.side() == 0. && t < to {
                    if ge(x.c[t], eh + tk) { pos.enter(1., t, x.c[t], "close"); }
                    else if le(x.c[t], el - tk) { pos.enter(-1., t, x.c[t], "close"); }
                }
            }
        }
    }
    Some(pos.done(to, x.c[to]))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn box_entries_on_closes_and_exits_beyond_the_three_week_range() {
        let r = super::super::sctraub::tests::bars(3000);
        let x = Ctx::new(&r, &r, &[]);
        let t = run("scseyk_box", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(t.len() > 3 && t.iter().any(|z| z.dir > 0.) && t.iter().any(|z| z.dir < 0.));
        for z in &t { assert_eq!(z.legs[0].px, x.c[z.legs[0].i]); assert!(z.exit_i > z.legs[0].i || z.open); }
        for w in t.windows(2) { assert!(w[1].legs[0].i >= w[0].exit_i); }
    }
}

// Instrument admission: S&C Seykota / Druz box system 2026-11-04 (titles lane K40).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "scseyk" && reviewed(def.id),
    label: "Ed Seykota and Dave Druz tested the 20-week / 3-week box system on a five-futures portfolio (soybean oil, cattle, sugar, gold, Swiss franc; 1979-1992). Runs on indices, ETFs and stocks, US and India, are a declared application.",
    basis: "Technical Analysis of Stocks & Commodities V.11:3 pp.122-124 (Determining Optimal Risk), sidebar 'System test'; independent check authors/titles-1030/K40w/k40_ref.py",
};
