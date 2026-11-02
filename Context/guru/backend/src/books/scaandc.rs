//! Peter Aan, "Delayed Channel Breakout System", Technical Analysis of Stocks & Commodities V.8:9 (September 1990) pp.334-335
//! (research share Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V08/C09/DELAY.pdf).
//! Book id `scaandc`. Lane K13 (titles loop, 2 Nov 2026), to-code row 199, lead decision 73; independent check
//! authors/titles-1030/K13w/k13_ref.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::lc3::{hi, high_first, lo};
use super::sctraub::ticks;
use super::tselhs::Book;

pub const REVIEWED: &[&str] = &["scaandc_8_20"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] The article's worked example, an eight-day channel delayed 20 days, is the default (decision 73); it tested channels of 4-14 days and delays of 15-50 'optimised per market', which are options. At each close the stops for the next day: the highest high (+ 1 tick) and lowest low (- 1 tick) of the x days ending y days ago ('first count back 20 market days from today and examine the highs and lows of that day and the previous seven days'). '1 point' is read as one minimum tick of the instrument (NSE 0.05, US 0.01, decision 57); 0 ticks is an option. A stop fills at its price, or at the open when the open is already through it. Always in once the first stop fills; one fill a day (with no position, when both stops are hit, the one the open is through, else the extreme nearer the open). The article's tests are futures over 5.5 years with no printed table. Runs on indices, ETFs and stocks, US and India, are a declared application.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "scaandc_8_20", name: "Aan delayed channel breakout: stop and reverse 1 tick beyond the 8-day high/low of 20 days ago", group: "S&C: Delayed Channel Breakout System (Aan, V.8:9)", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "scaandc", rules: &[
            "E|\"Exit shorts and buy long on an intraday stop one point above the highest high of x days, with the most recent day being y days ago.\" (V.8:9 p.334)",
            "X|\"Exit longs and sell short on an intraday stop one point below the lowest low of x days, with the most recent day being y days ago.\" (p.334)",
            "F|\"let's say you are using an eight-day channel with a 20-day delay. To find the buy or sell points for tomorrow, first count back 20 market days from today and examine the highs and lows of that day and the previous seven days.\" (p.334)"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let id = REVIEWED.iter().find(|r| **r == id)?;
    Some((*id, "Aan delayed channel breakout (S&C)", "8-day channel, 20-day delay"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    reviewed(id).then(|| vec![p("x", "Channel length, days (example 8; tested 4-14)", 8., 4., 14., 1.), p("y", "Delay, days (example 20; tested 15-50)", 20., 15., 50., 1.),
        p("tick", "Stop offset in ticks ('one point'; 0 = none)", 1., 0., 10., 1.)])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let g = |k: &str, d: f64| p.get(k).copied().unwrap_or(d).round().max(1.0) as usize;
    let (cx, cy, k) = (g("x", 8.), g("y", 20.), ticks(x, p));
    let mut pos = Book::new();
    for t in from..to {
        if t + 1 < cx + cy { continue; }
        let (a, b) = (t + 1 - cy - cx, t - cy);
        let (bs, ss) = (hi(&x.h, a, b) + k, lo(&x.l, a, b) - k);
        let (i, d) = (t + 1, pos.side());
        let (o, h, l) = (x.o[i], x.h[i], x.l[i]);
        let (mut hb, mut hs) = (d != 1.0 && h >= bs, d != -1.0 && l <= ss);
        if hb && hs {
            if o >= bs { hs = false } else if o <= ss { hb = false } else if high_first(o, h, l) { hs = false } else { hb = false }
        }
        if hb { pos.enter(1.0, i, o.max(bs), "stop"); } else if hs { pos.enter(-1.0, i, o.min(ss), "stop"); }
    }
    Some(pos.done(to, x.c[to]))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn delayed_channel_reverses() {
        let r = super::super::sctraub::tests::bars(3000);
        let x = Ctx::new(&r, &r, &[]);
        let t = run("scaandc_8_20", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(t.len() > 5 && t.iter().any(|z| z.dir > 0.0) && t.iter().any(|z| z.dir < 0.0));
        for w in t.windows(2) { assert_eq!(w[0].exit_i, w[1].legs[0].i); }
    }
}

// Instrument admission: S&C Aan delayed channel 2026-11-02 (titles lane K13).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "scaandc" && reviewed(def.id),
    label: "Peter Aan tested the delayed channel on futures (5.5 years, no printed table). Runs on indices, ETFs and stocks, US and India, are a declared application.",
    basis: "Technical Analysis of Stocks & Commodities V.8:9 pp.334-335 (Delayed Channel Breakout System); independent check authors/titles-1030/K13w/k13_ref.py",
};
