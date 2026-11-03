//! Peter Aan, "A simple variation of the moving average", Technical Analysis of Stocks & Commodities V.8:5 (May 1990)
//! pp.185-186 (research share Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V08/C05/
//! SIMPVAR.pdf). Book id `scaanmad`: the Moving Average Direction System. Lane K16 (titles loop, 2 Nov 2026), to-code row 241,
//! decisions 19, 57, 73; independent check authors/titles-1030/K16w/k16_ref.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::msws::sma;
use super::sctraub::ticks;
use super::tselhs::Book;

pub const REVIEWED: &[&str] = &["scaanmad_30"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] The article's example, a 30-day simple average of closes, is the default (decision 73; it tested 5-50 days, options). At each close: if long (or flat) and today's average is below yesterday's, a sell stop one tick below today's low is placed for tomorrow; if short (or flat) and today's average is above yesterday's, a buy stop one tick above today's high. 'One point' is read as one minimum tick of the instrument (NSE 0.05, US 0.01, decision 57); 0 ticks is an option. A stop fills at its level or at the open when the open is beyond it; an unfilled stop is replaced by the next day's signal (or lapses). HELD: the $1,500 liquidation stop is a dollar amount and is not applied or converted (decisions 19, 63), so the card is always in the market once the first stop fills. Article results (futures, 5.5 years, System Writer Plus) are a chart only. Runs on indices, ETFs and stocks, US and India, are a declared application.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "scaanmad_30", name: "Aan MA direction: 30-day SMA turns, enter on a stop 1 tick beyond today's range, stop and reverse", group: "S&C: A simple variation of the moving average (Aan, V.8:5)", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "scaanmad", rules: &[
            "E|\"Compare today's n-day MA with the n-day MA from yesterday. A sell signal for the long side is generated when today's MA is less than yesterday's MA. If short, a buy signal is generated when today's MA is greater than yesterday's MA.\" (V.8:5, PDF p.1)",
            "E|\"Once a signal is generated, enter the market tomorrow on a one-point penetration of today's range. For example, in the case of a buy signal, place a stop one point above today's high price ... A sell signal reverses on a sell stop placed one point below today's low.\" (PDF p.1)",
            "X|Held: \"Place a liquidation stop to keep losses to a certain dollar amount, for instance $1,500.\" (PDF p.1) - dollar stop not applied (decision 19)."],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    reviewed(id).then_some(("scaanmad_30", "Aan MA direction system (S&C)", "30-day SMA"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    reviewed(id).then(|| vec![p("n", "SMA days (example 30; tested 5-50)", 30., 2., 60., 1.), p("tick", "Stop offset in ticks ('one point'; 0 = none)", 1., 0., 10., 1.)])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let m = sma(&x.c, p.get("n").copied().unwrap_or(30.).round().max(1.) as usize);
    let k = ticks(x, p);
    let mut pos = Book::new();
    let mut st: Option<(f64, f64)> = None; // (direction, level) for the next day
    for i in from..=to {
        if let Some((d, lv)) = st.take() {
            if d > 0.0 && x.h[i] >= lv { pos.enter(1.0, i, x.o[i].max(lv), "stop"); }
            else if d < 0.0 && x.l[i] <= lv { pos.enter(-1.0, i, x.o[i].min(lv), "stop"); }
        }
        if i == to || i == 0 || !m[i - 1].is_finite() { continue; }
        let s = pos.side();
        if s != -1.0 && m[i] < m[i - 1] { st = Some((-1.0, x.l[i] - k)); } else if s != 1.0 && m[i] > m[i - 1] { st = Some((1.0, x.h[i] + k)); }
    }
    Some(pos.done(to, x.c[to]))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ma_direction_reverses() {
        let r = super::super::sctraub::tests::bars(2000);
        let x = Ctx::new(&r, &r, &[]);
        let t = run("scaanmad_30", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(t.len() > 5);
        for w in t.windows(2) { assert_eq!(w[0].exit_i, w[1].legs[0].i); }
    }
}

// Instrument admission: S&C Aan MA direction 2026-11-02 (titles lane K16).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "scaanmad" && reviewed(def.id),
    label: "Peter Aan tested the MA direction system on futures (5.5 years, results as a chart). Runs on indices, ETFs and stocks, US and India, are a declared application.",
    basis: "Technical Analysis of Stocks & Commodities V.8:5 pp.185-186 (A simple variation of the moving average); independent check authors/titles-1030/K16w/k16_ref.py",
};
