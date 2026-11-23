//! Tim Hayes (Ned Davis Research), S&C V.9:8 interview, sidebar "A Composite Indicator" (p.310-315; research share Trading Knowledge I/Stocks & Commodities
//! DVD 11.26/VOLUMES/V09/C08/SIDEBC.pdf, text layer, Figure 1). Book id `thvl`. The Value Line slope indicator: this week's close against the close 30
//! weeks ago as a percentage; "if the percentage difference moves below -1% and then increases above -1%, a buy signal is flashed. If the percentage
//! increase moves above 16% and then drops below 16%, a sell signal is indicated." The sidebar's other two indicators (Investors Intelligence sentiment,
//! 26-week short-term government bond change) need data the vault does not hold and are not coded. Lane K1 (23 Nov 2026, queue A1213).
//! Independent check authors/conv-1123/K1/thvl_ref.py.
use super::atm03::{day_num, weekday};
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "thvl";
pub const REVIEWED: &[&str] = &["thvl_slope"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared adaptation, R4 #340] The sidebar uses the Value Line Composite (an equal-weighted US index the vault does not hold) and prints no trade list or result, so no figure is compared. An index rule: it runs on actual indices only (US and NSE); an index or ETF is the nearest match to the Value Line, never a single stock. UNVERIFIED. [Declared readings, each an unstated detail] (1) Weekly closes = the close of the last trading day of each calendar week (Monday-Sunday) that is followed by a bar of a later week, so a week still in progress at the end of the data is not used. (2) 'Moves below -1% and then increases above -1%' = the weekly 30-week change was at or below -1% and is above it on the next week; the sell is the mirror at 16% (at or above 16%, then below it); the order is filled at that weekly close. (3) Long or flat: a buy signal while long, or a sell while flat, is ignored (the sidebar gives no short side).";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "thvl_slope", name: "30-week momentum: buy when it rises back above -1%, sell when it falls back below +16% (weekly, long/flat)",
        group: "Hayes Value Line slope", session: 0, dir: "Long", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: "thvl",
        rules: &[
            "F|\"a Value Line slope indicator based on the 30-week momentum of the Value Line composite\"; \"Use this week's close of the Value Line Index compared with the close 30 weeks ago\" (Figure 1)",
            "E|\"If the Value Line momentum goes below -1%, turns around and moves above that, that's a buy.\"",
            "X|\"If the Value Line momentum goes above 16%, peaks out and turns down, that's a sell signal.\""],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match id { "thvl_slope" => Some(("thvl_slope", "Hayes 30-week momentum", "30 weeks, -1% / 16%")), _ => None }
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    match id {
        "thvl_slope" => Some(vec![p("weeks", "Momentum length, weeks (sidebar 30)", 30., 4., 104., 1.), p("buy", "Buy when it rises back above this % (sidebar -1)", -1., -20., 20., 1.), p("sell", "Sell when it falls back below this % (sidebar 16)", 16., 0., 60., 1.)]),
        _ => None,
    }
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let s = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| s.iter().find(|d| d.key == k).unwrap().def);
    let (k, buy, sell) = (g("weeks").round() as usize, g("buy"), g("sell"));
    let n = x.r.len();
    let wk = |i: usize| { let d = day_num(&x.r[i].date); d - (weekday(d) + 6) % 7 };
    let ends: Vec<usize> = (0..n - 1).filter(|&i| wk(i + 1) != wk(i)).collect();
    let (mut out, mut cur) = (vec![], None::<(usize, f64)>);
    for j in k + 1..ends.len() {
        let i = ends[j];
        if i < from { continue; }
        if i > to { break; }
        let m = |w: usize| 100.0 * (x.c[ends[w]] / x.c[ends[w - k]] - 1.0);
        let (a, b) = (m(j - 1), m(j));
        match cur {
            None if a <= buy && b > buy => cur = Some((i, x.c[i])),
            Some((e, ep)) if a >= sell && b < sell => {
                out.push(Trade { dir: 1., legs: vec![Leg { i: e, px: ep, w: 1. }], exit_i: i, exit_px: x.c[i], reason: "30-week change fell back below the sell level".into(), entry_kind: "close", signal_i: i, open: false, exits: vec![] });
                cur = None;
            }
            _ => {}
        }
    }
    if let Some((e, ep)) = cur {
        out.push(Trade { dir: 1., legs: vec![Leg { i: e, px: ep, w: 1. }], exit_i: to, exit_px: x.c[to], reason: "open".into(), entry_kind: "close", signal_i: to, open: true, exits: vec![] });
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn rejects_other_ids_and_runs() {
        let r: Vec<Row> = (0..700).map(|i| { let v = 100.0 + (i as f64 * 0.02).sin() * 30.0; Row { ticker: "T".into(), date: format!("{}-{:02}-{:02}", 2000 + i / 336, (i / 28) % 12 + 1, i % 28 + 1), open: v, high: v, low: v, close: v, volume: None } }).collect();
        let x = Ctx::new(&r, &r, &[]);
        let p = Params::default();
        assert!(run("thvl_x", &x, 0, 699, &p).is_none());
        for z in run("thvl_slope", &x, 0, 699, &p).unwrap() { assert!(z.exit_i >= z.legs[0].i); }
    }
}
