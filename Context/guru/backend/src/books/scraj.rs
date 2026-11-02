//! Xavier Maria Raj, "A Simple Range-Trading System", Technical Analysis of Stocks & Commodities V.23:12 (December 2005) pp.50-52
//! (research share Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V23/C12/252RAJ.pdf).
//! Book id `scraj`: the range expansion day strategy. Lane K13 (titles loop, 2 Nov 2026), to-code row 217; independent check
//! authors/titles-1030/K13w/k13_ref.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::sctraub::ticks;
use super::tselhs::Book;

pub const REVIEWED: &[&str] = &["scraj_red"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] Range expansion day: today's high - low above the previous day's. If it closed above its open, the next day only a buy stop 1 tick above its high; if below, only a sell stop 1 tick under its low; no order when close = open. A stop fills at its price, or at the open if the open is beyond it; out at that day's close (one entry and one exit a day, so no intraday path question). One tick = the instrument's minimum tick (NSE 0.05, US 0.01; 0 is an option). The 1.5x and 2x range filters of the article are options. Article (cash indices, 1 January 1994 - 29 August 2003): Russell Midcap 834 trades, 64.6% profitable; Russell 2000 890 trades, 71.1%; with 1.5x 440 / 468 trades, 67% / 75%. Those indices are not held (IWM is a different series). Runs on indices, ETFs and stocks, US and India, are a declared application.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "scraj_red", name: "Raj range expansion day: after a wider-range up (down) day, buy (sell) a stop beyond its high (low) next day; out at that close", group: "S&C: A Simple Range-Trading System (Raj, V.23:12)", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "scraj", rules: &[
            "F|\"Long Entry : On a Range Expansion Day with Close greater than Open (D1) - Place Stop Loss Buy Order for the Next Day (D2) at High (H1) Plus One Tick\" (sidebar, V.23:12 p.52)",
            "E|\"Short Entry : On a Range Expansion Day with Close lower than Open (D1) - Place Sell Stop Loss Order for the Next Day (D2) at Low (L1) Minus One Tick\" (sidebar)",
            "X|\"Exit the position at Close.\" (sidebar)"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let id = REVIEWED.iter().find(|r| **r == id)?;
    Some((*id, "Raj range expansion day (S&C)", "Next-day stop, out at the close"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    reviewed(id).then(|| vec![p("mult", "Expansion: range above x times the previous range (1 printed; 1.5 and 2 tested)", 1., 1., 3., 0.5), p("tick", "Stop offset in ticks (1 printed; 0 = none)", 1., 0., 10., 1.)])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let (mult, k) = (p.get("mult").copied().unwrap_or(1.0), ticks(x, p));
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let mut pos = Book::new();
    for t in from.max(1)..to {
        if !(h[t] - l[t] > mult * (h[t - 1] - l[t - 1])) { continue; }
        let i = t + 1;
        if c[t] > o[t] && h[i] >= h[t] + k { pos.enter(1.0, i, o[i].max(h[t] + k), "stop"); pos.exit(i, c[i], "close"); }
        else if c[t] < o[t] && l[i] <= l[t] - k { pos.enter(-1.0, i, o[i].min(l[t] - k), "stop"); pos.exit(i, c[i], "close"); }
    }
    Some(pos.done(to, c[to]))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn range_expansion_day_trades() {
        let r = super::super::sctraub::tests::bars(3000);
        let x = Ctx::new(&r, &r, &[]);
        let t = run("scraj_red", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(t.len() > 10 && t.iter().all(|z| z.exit_i == z.legs[0].i && z.exit_px == x.c[z.exit_i]));
    }
}

// Instrument admission: S&C Raj range expansion 2026-11-02 (titles lane K13).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "scraj" && reviewed(def.id),
    label: "Xavier Maria Raj tested the rule on Russell Midcap and Russell 2000 cash index data (1994-2003). Runs on the vault's indices, ETFs and stocks, US and India, are a declared application.",
    basis: "Technical Analysis of Stocks & Commodities V.23:12 pp.50-52 and sidebar (A Simple Range-Trading System); independent check authors/titles-1030/K13w/k13_ref.py",
};
