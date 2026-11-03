//! Phil Abel, "Aftermath Of The Bubble", Technical Analysis of Stocks & Commodities V.21:4 (April 2003) pp.20-24 (research share
//! Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V21/C04/075ABEL.pdf). Book id `scabel`:
//! the article's sample 21/34/55 SMA system. Lane K13 (titles loop, 2 Nov 2026), to-code row 209; independent check
//! authors/titles-1030/K13w/k13_ref.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::msws::sma;
use super::sctraub::next_open;

pub const REVIEWED: &[&str] = &["scabel_sma3"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] SMAs of the close. The rules are states: with no position, a long is opened while SMA21 > SMA34 > SMA55 and a short while SMA21 < SMA34 < SMA55; an exit day that meets the other side's entry reverses. Orders at the next open (the article's TradeStation chart shows next-bar entries; it gives no fill). One unit (the article sizes 1% of a $1 million account per trade). The author presents it as a sample system 'intended merely to illustrate a concept, not recommend a particular system'. Article: each Nasdaq-100 component since 1990, $3.53 million total, 353% (10.5% a year) through 2002, one windfall trade making most of a stock's profit. Runs on indices, ETFs and stocks, US and India, are a declared application.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "scabel_sma3", name: "Abel triple SMA (21/34/55): long when stacked up, short when stacked down, out when the 21 falls under both (rises over both)", group: "S&C: Aftermath Of The Bubble (Abel, V.21:4)", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "scabel", rules: &[
            "E|\"Buy when SMA(21) > SMA(34) and SMA(34) > SMA(55) ... Sell short when SMA(21) < SMA(34) and SMA(34) < SMA(55)\" (V.21:4 p.21)",
            "X|\"Exit long when SMA(21) < SMA(34) and SMA(21) < SMA(55) ... Exit short when SMA(21) > SMA(34) and SMA(21) > SMA(55)\" (p.21); \"Moving average calculations were based on closing prices.\""],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let id = REVIEWED.iter().find(|r| **r == id)?;
    Some((*id, "Abel triple SMA (S&C)", "21/34/55"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> { reviewed(id).then(Vec::new) }

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, _p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let (a, b, c) = (sma(&x.c, 21), sma(&x.c, 34), sma(&x.c, 55));
    Some(next_open(x, from, to, "SMA exit", |t, pos| {
        if !c[t].is_finite() { return pos; }
        let mut d = pos;
        if pos > 0.0 && a[t] < b[t] && a[t] < c[t] { d = 0.0; }
        if pos < 0.0 && a[t] > b[t] && a[t] > c[t] { d = 0.0; }
        if d == 0.0 { if a[t] > b[t] && b[t] > c[t] { d = 1.0; } else if a[t] < b[t] && b[t] < c[t] { d = -1.0; } }
        d
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn triple_sma() {
        let r = super::super::sctraub::tests::bars(3000);
        let x = Ctx::new(&r, &r, &[]);
        let t = run("scabel_sma3", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(t.iter().any(|z| z.dir > 0.0) && t.iter().any(|z| z.dir < 0.0));
    }
}

// Instrument admission: S&C Abel triple SMA 2026-11-02 (titles lane K13).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "scabel" && reviewed(def.id),
    label: "Phil Abel applied the sample system to each Nasdaq-100 component stock (1990-2002). Runs on indices, ETFs and other stocks, US and India, are a declared application.",
    basis: "Technical Analysis of Stocks & Commodities V.21:4 pp.20-24 (Aftermath Of The Bubble); independent check authors/titles-1030/K13w/k13_ref.py",
};
