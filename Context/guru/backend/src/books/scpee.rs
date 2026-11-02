//! M.H. Pee, "Trend Trigger Factor", Technical Analysis of Stocks & Commodities V.22:12 (December 2004) pp.28-36 (research share
//! Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V22/C12/263PEE.pdf). Book id `scpee`.
//! Lane K13 (titles loop, 2 Nov 2026), to-code row 212; independent check authors/titles-1030/K13w/k13_ref.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::lc3::{hi, lo};
use super::sctraub::next_open;

pub const REVIEWED: &[&str] = &["scpee_ttf"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] Day 1 = today: 15-day buy power = highest high of days 1-15 - lowest low of days 16-30; sell power = highest high of days 16-30 - lowest low of days 1-15; TTF = 100 (buy power - sell power) / (0.5 (buy power + sell power)). Above +100 be long, below -100 be short, in between keep the position; flat until the first signal; orders at the next open as printed. Article (35 futures markets, 2 January 1980 - 30 April 2003, $75 a trade): 15 days, 3,126 trades, 39.1% winners, profitable in 30 of 35 markets; 10/12/18/22 days also tested (options). Runs on indices, ETFs and stocks, US and India, are a declared application.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "scpee_ttf", name: "Pee Trend Trigger Factor (15): always in, long above +100, short below -100, at the next open", group: "S&C: Trend Trigger Factor (Pee, V.22:12)", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "scpee", rules: &[
            "F|\"15-day buy power = Highest high of (day 1 to day 15 inclusive) - Lowest low of (day 16 to day 30 inclusive) 15-day sell power = Highest high of (day 16 to day 30) - Lowest low of (day 1 to day 15) ... 15-day TTF = ((Buy power - sell power)/(0.5*(Buy power + sell power))) * 100\" (V.22:12 p.28)",
            "E|\"if you are currently short and the value of the TTF is above 100, reverse your position by entering long the next day at the open. Similarly, if you are holding long positions as of the close today and the value of the TTF happens to be below -100, then revert to short positions at the open the following trading day.\" (p.31)",
            "X|\"if it is between -100 and 100, you should remain with the current position\" (p.31); \"at any particular time, I was either long or short but never flat\""],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let id = REVIEWED.iter().find(|r| **r == id)?;
    Some((*id, "Pee Trend Trigger Factor (S&C)", "15-day TTF"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    reviewed(id).then(|| vec![ParamDef { key: "n", label: "TTF days (15 default; tested 10, 12, 18, 22)", def: 15., min: 5., max: 30., step: 1. }])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let n = p.get("n").copied().unwrap_or(15.0).round().max(1.0) as usize;
    Some(next_open(x, from, to, "TTF reversal", |t, pos| {
        if t + 1 < 2 * n { return pos; }
        let (h1, l1) = (hi(&x.h, t + 1 - n, t), lo(&x.l, t + 1 - n, t));
        let (h2, l2) = (hi(&x.h, t + 1 - 2 * n, t - n), lo(&x.l, t + 1 - 2 * n, t - n));
        let (bp, sp) = (h1 - l2, h2 - l1);
        if bp + sp == 0.0 { return pos; }
        let ttf = 100.0 * (bp - sp) / (0.5 * (bp + sp));
        if ttf > 100.0 { 1.0 } else if ttf < -100.0 { -1.0 } else { pos }
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ttf_reverses() {
        let r = super::super::sctraub::tests::bars(3000);
        let x = Ctx::new(&r, &r, &[]);
        let t = run("scpee_ttf", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(t.len() > 3);
        for w in t.windows(2) { assert_eq!(w[0].exit_i, w[1].legs[0].i); }
    }
}

// Instrument admission: S&C Pee TTF 2026-11-02 (titles lane K13).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "scpee" && reviewed(def.id),
    label: "M.H. Pee tested the TTF reversal system on 35 futures markets (1980-2003). Runs on indices, ETFs and stocks, US and India, are a declared application.",
    basis: "Technical Analysis of Stocks & Commodities V.22:12 pp.28-36 (Trend Trigger Factor); independent check authors/titles-1030/K13w/k13_ref.py",
};
