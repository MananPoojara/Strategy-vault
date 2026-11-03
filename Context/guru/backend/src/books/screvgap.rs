//! Stephane Reverre, "Trade Against The Gap", Technical Analysis of Stocks & Commodities V.17:12 (December 1999) pp.555-560
//! (research share Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V17/C12/100GAP.pdf).
//! Book id `screvgap`. Lane K16 (titles loop, 2 Nov 2026), to-code row 255; independent check authors/titles-1030/K16w/k16_ref.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::tselhs::Book;

pub const REVIEWED: &[&str] = &["screvgap_3"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] The printed EasyLanguage with its printed inputs (gaplong 3, gapshort 3, holdlong 1, holdshort 2): gap = 100 x (close - previous close) / previous close. With no position: a gap of -3% or less buys at this bar's close; +3% or more sells short at this bar's close. A long is closed at the close one bar after entry, a short at the close two bars after entry. No stops. A bar that closes a trade does not open a new one (MarketPosition is read before the exit). The article's limit-order variant placed before the open is not coded. Article: GE and the Dow stocks (distribution study), a 1-day hold for longs and 2 days for shorts were the best holding periods. The author tested stocks; runs on indices and ETFs, US and India, are a declared application.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "screvgap_3", name: "Reverre trade against the gap: buy a 3% down close for one day, short a 3% up close for two days", group: "S&C: Trade Against The Gap (Reverre, V.17:12)", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "screvgap", rules: &[
            "F|\"inputs: gaplong(3), gapshort(3), holdlong(1), holdshort(2); ... gap = 100*(close - close[1])/close[1];\" (V.17:12, PDF p.4)",
            "E|\"if gap <= -gaplong and MarketPosition = 0 then buy this bar at close; ... if gap >= gapshort and MarketPosition = 0 then sell this bar at close;\" (PDF p.4)",
            "X|\"if BarsSinceEntry = holdlong and MarketPosition = 1 then exitlong at close; ... if BarsSinceEntry = holdshort and MarketPosition = -1 then exitshort at close;\" (PDF p.4)"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    reviewed(id).then_some(("screvgap_3", "Reverre trade against the gap (S&C)", "3% gaps, 1/2-day holds"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    reviewed(id).then(|| vec![p("gl", "Down gap to buy, %", 3., 0.5, 10., 0.5), p("gs", "Up gap to short, %", 3., 0.5, 10., 0.5),
        p("hl", "Long hold, bars", 1., 1., 10., 1.), p("hs", "Short hold, bars", 2., 1., 10., 1.)])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let g = |k: &str, d: f64| p.get(k).copied().unwrap_or(d);
    let (gl, gs) = (g("gl", 3.), g("gs", 3.));
    let (hl, hs) = (g("hl", 1.).round().max(1.) as usize, g("hs", 2.).round().max(1.) as usize);
    let mut pos = Book::new();
    for t in from.max(1)..=to {
        let gap = 100.0 * (x.c[t] - x.c[t - 1]) / x.c[t - 1];
        match (pos.side(), pos.entry_bar()) {
            (d, Some(e)) if d > 0.0 => { if t - e == hl { pos.exit(t, x.c[t], "hold") } }
            (d, Some(e)) if d < 0.0 => { if t - e == hs { pos.exit(t, x.c[t], "hold") } }
            _ => if gap <= -gl { pos.enter(1.0, t, x.c[t], "close") } else if gap >= gs { pos.enter(-1.0, t, x.c[t], "close") },
        }
    }
    Some(pos.done(to, x.c[to]))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn gap_fade_holds() {
        let mut r = super::super::sctraub::tests::bars(400);
        for i in (20..380).step_by(17) { let k = if i % 2 == 0 { 0.95 } else { 1.05 }; r[i].close = r[i - 1].close * k; }
        let x = Ctx::new(&r, &r, &[]);
        let t = run("screvgap_3", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(t.len() > 5 && t.iter().all(|z| z.exit_i - z.legs[0].i == if z.dir > 0.0 { 1 } else { 2 } || z.open));
    }
}

// Instrument admission: S&C Reverre gap 2026-11-02 (titles lane K16).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "screvgap" && reviewed(def.id),
    label: "Stephane Reverre studied GE and the Dow stocks. Runs on indices and ETFs, US and India, are a declared application.",
    basis: "Technical Analysis of Stocks & Commodities V.17:12 pp.555-560 (Trade Against The Gap), printed EasyLanguage; independent check authors/titles-1030/K16w/k16_ref.py",
};
