//! Donald W. Pendergast Jr., "The Keys To Trend-Following", Technical Analysis of Stocks & Commodities V.26:11 (November 2008)
//! pp.26-30 (research share Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V26/C11/
//! 201PEND.pdf). Book id `scpend`: the weekly 10/40 EMA framework. Lane K13 (titles loop, 2 Nov 2026), to-code row 220;
//! independent check authors/titles-1030/K13w/k13_ref.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::sctraub::{ema0, period_ends};
use super::tselhs::Book;

pub const REVIEWED: &[&str] = &["scpend_ema1040"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] Weekly closes (each week's last trading day); 10- and 40-week EMAs seeded with the first weekly close, used from the 40th week. A cross is confirmed at the week's close (10-week above the 40-week now, at or below it the week before; mirror for the exit); the trade is made at the close of the next trading day ('the first trading day after the moving average crossover has been confirmed on the weekly chart'). Long only. Article (17 Fidelity Select sector funds, January 2002 - 6 April 2007, open trades included): average trade +14.05%, 50.98% winners; fund NAVs are not held. Runs on indices, ETFs and stocks, US and India, are a declared application.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "scpend_ema1040", name: "Pendergast weekly 10/40 EMA trend-following: in the day after the 10-week EMA crosses above the 40-week, out the day after it crosses below", group: "S&C: The Keys To Trend-Following (Pendergast, V.26:11)", session: 0, dir: "Long", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "scpend", rules: &[
            "E|\"Enter after the 10-week exponential moving average (EMA) crosses the 40-week EMA to the upside. Enter the market at the close of the first trading day after the moving average crossover has been confirmed on the weekly chart.\" (V.26:11 p.27)",
            "X|\"Exit after the 10-week EMA crosses the 40-week EMA to the downside. Exit the market at the close of the first trading day after the moving average crossover has been confirmed on the weekly chart.\" (p.27)"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let id = REVIEWED.iter().find(|r| **r == id)?;
    Some((*id, "Pendergast weekly EMA trend-following (S&C)", "10/40-week EMA"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> { reviewed(id).then(Vec::new) }

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, _p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let we = period_ends(x, false);
    let w: Vec<f64> = we.iter().map(|&i| x.c[i]).collect();
    let (e10, e40) = (ema0(&w, 10), ema0(&w, 40));
    let mut pos = Book::new();
    for (k, &i) in we.iter().enumerate() {
        if k < 40 || i < from || i >= to { continue; }
        if e10[k] > e40[k] && e10[k - 1] <= e40[k - 1] && pos.side() == 0.0 { pos.enter(1.0, i + 1, x.c[i + 1], "close"); }
        else if e10[k] < e40[k] && e10[k - 1] >= e40[k - 1] && pos.side() > 0.0 { pos.exit(i + 1, x.c[i + 1], "10-week EMA under the 40-week"); }
    }
    Some(pos.done(to, x.c[to]))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn weekly_ema_cross_next_day() {
        let r = super::super::sctraub::tests::bars(4000);
        let x = Ctx::new(&r, &r, &[]);
        let t = run("scpend_ema1040", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(!t.is_empty() && t.iter().all(|z| super::super::sctraub::weekday(&r[z.legs[0].i].date) == 0));
    }
}

// Instrument admission: S&C Pendergast 2026-11-02 (titles lane K13).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "scpend" && reviewed(def.id),
    label: "Donald Pendergast tested the weekly framework on Fidelity Select sector funds (not held). Runs on indices, ETFs and stocks, US and India, are a declared application.",
    basis: "Technical Analysis of Stocks & Commodities V.26:11 pp.26-30 (The Keys To Trend-Following); independent check authors/titles-1030/K13w/k13_ref.py",
};
