//! "Product Review: NeuroShell Trader Professional", Technical Analysis of Stocks & Commodities V.18:1 (2000) pp.83-87 (research share
//! Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/PR/V18/140PR.pdf): the review's demonstration
//! system, a 40-day EMA with its slope, stop and reverse. Book id `scnsema`. Lane K41 (batch T7a, 4 Nov 2026); independent check
//! authors/titles-1030/K41w/k41_ref.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::sctraub::{ema0, next_open};

pub const REVIEWED: &[&str] = &["scnsema_ema40"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] EMA alpha 2/(n+1) seeded with the first close. Long when the close is above the 40-day EMA and the EMA rose from yesterday; short when the close is below it and the EMA fell; otherwise the position is held ('a stop-and-reverse (SAR) system, closing the old position simultaneously with the opening of the new position'). The review offers market orders at the next open or close; fills at the next open. Review example (labelled; 'no optimization'): Apple Computer, 4 January 1994 to 27 September 1999, 122 trades, +$722. Runs on indices, ETFs and stocks, US and India, are a declared application.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "scnsema_ema40", name: "Close above a rising 40-day EMA long, below a falling one short (stop and reverse), next open", group: "S&C: NeuroShell Trader review system (V.18:1)", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "scnsema", rules: &[
            "E|\"We used a very simple system, which enters long when the close is above the 40-day exponential moving average (EMA) of closing prices and the slope of the EMA is positive. The slope is defined as yesterday's EMA value subtracted from today's EMA value. The system enters short when the close is below the 40-day EMA and the EMA slope is below zero.\" (V.18:1, PDF p.3)",
            "X|\"The system is a stop-and-reverse (SAR) system, closing the old position simultaneously with the opening of the new position.\" (PDF p.3); \"market (open or close) ... the day following the signal\" (PDF p.3)",
            "F|\"Figure 6 is the result of backtesting the system with no optimization on Apple Computer during the period January 4, 1994, to September 27, 1999 ... after placing 122 trades, the system made $722.\" (PDF p.3-4)"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { reviewed(id).then_some(("scnsema_ema40", "40-day EMA slope SAR (S&C review)", "Stop and reverse")) }
pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    reviewed(id).then(|| vec![ParamDef { key: "n", label: "EMA days", def: 40., min: 5., max: 200., step: 1. }])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let n = p.get("n").copied().unwrap_or(40.).round().max(1.) as usize;
    let e = ema0(&x.c, n);
    Some(next_open(x, from, to, "Stop and reverse", |t, side| {
        if t == 0 { return side; }
        let sl = e[t] - e[t - 1];
        if x.c[t] > e[t] && sl > 0.0 { 1.0 } else if x.c[t] < e[t] && sl < 0.0 { -1.0 } else { side }
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ema_slope_sar() {
        let r = super::super::sctraub::tests::bars(1500);
        let x = Ctx::new(&r, &r, &[]);
        let t = run("scnsema_ema40", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(t.len() > 5 && t.windows(2).all(|w| w[0].dir != w[1].dir && w[0].exit_i == w[1].legs[0].i));
    }
}
