//! Peter Aan, "Channel breakout", Technical Analysis of Stocks & Commodities V.7:9 (September 1989) pp.295-297 (research share
//! Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V07/C09/CHANNEL.pdf). Book id
//! `scaancb`. Lane K16 (titles loop, 2 Nov 2026), to-code row 239, decision 73; independent check authors/titles-1030/K16w/k16_ref.py.
//! Also holds `sar_stops`, the stop-and-reverse fill loop the lane-K16 Aan stop systems share.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::lc3::{hi, high_first, lo};
use super::tselhs::Book;

pub const REVIEWED: &[&str] = &["scaancb_8"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

/// Always-in stop and reverse on intraday stops: `lv(i)` = (buy stop, sell stop) for day i. Fills on days from+1..=to, one a
/// day: short (or flat) -> buy stop at max(open, level); long (or flat) -> sell stop at min(open, level); flat with both hit ->
/// the stop the open is through, else the extreme nearer the open first.
pub fn sar_stops(x: &Ctx, from: usize, to: usize, lv: impl Fn(usize) -> Option<(f64, f64)>) -> Vec<Trade> {
    let mut pos = Book::new();
    for i in from + 1..=to {
        let Some((bs, ss)) = lv(i) else { continue };
        let (o, d) = (x.o[i], pos.side());
        let (mut hb, mut hs) = (d != 1.0 && x.h[i] >= bs, d != -1.0 && x.l[i] <= ss);
        if hb && hs {
            if o >= bs { hs = false } else if o <= ss { hb = false } else if high_first(o, x.h[i], x.l[i]) { hs = false } else { hb = false }
        }
        if hb { pos.enter(1.0, i, o.max(bs), "stop"); } else if hs { pos.enter(-1.0, i, o.min(ss), "stop"); }
    }
    pos.done(to, x.c[to])
}

const NOTE: &str = "[Declared readings] The article's example, an 8-day channel, is the default (decision 73; other lengths are options). 'The most recent n days' for a day's stops = the n trading days before it. Intraday method (default): if long, sell and reverse on a stop at the lowest low of the channel; if short, buy and reverse on a stop at its highest high; a stop fills at its price or at the open when the open is already beyond it; with no position yet, the first stop touched opens the trade (both touched: the one the open is through, else the extreme nearer the open). Close method (option close = 1): reverse at today's close when it is at or beyond the channel high or low of the n days before today. The article found the intraday method better for 10 of 12 futures markets (about 5.5 years, results as a chart). Other vault channel cards are other authors' rules. Runs on indices, ETFs and stocks, US and India, are a declared application.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "scaancb_8", name: "Aan channel breakout: stop and reverse at the 8-day high / low", group: "S&C: Channel breakout (Aan, V.7:9)", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "scaancb", rules: &[
            "E|\"1. If long, sell and reverse on a stop at the lowest low for the most recent n days. 2. If short, buy and reverse on a stop at the highest high for the most recent n days.\" (V.7:9, PDF p.1)",
            "X|Option, close only: \"If long, sell and reverse on the close today ... if today's close is at or below the lowest low for the most recent n days. If short, buy and reverse on the close today if today's close is at or above the highest high for the most recent n days.\" (PDF p.1)",
            "F|\"if you are using an 8-day channel you will examine the highs and lows for the most recent eight trading days to find the highest high and the lowest low, which are the reversal points\" (PDF p.1)"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    reviewed(id).then_some(("scaancb_8", "Aan channel breakout (S&C)", "8 days"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    reviewed(id).then(|| vec![p("n", "Channel days (example 8)", 8., 2., 60., 1.), p("close", "1 = close-only method", 0., 0., 1., 1.)])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let n = p.get("n").copied().unwrap_or(8.).round().max(1.) as usize;
    if p.get("close").copied().unwrap_or(0.) < 0.5 {
        return Some(sar_stops(x, from, to, |i| (i >= n).then(|| (hi(&x.h, i - n, i - 1), lo(&x.l, i - n, i - 1)))));
    }
    let mut pos = Book::new();
    for t in from.max(n)..=to {
        let (c, d) = (x.c[t], pos.side());
        if d != 1.0 && c >= hi(&x.h, t - n, t - 1) { pos.enter(1.0, t, c, "close"); }
        else if d != -1.0 && c <= lo(&x.l, t - n, t - 1) { pos.enter(-1.0, t, c, "close"); }
    }
    Some(pos.done(to, x.c[to]))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn channel_reverses() {
        let r = super::super::sctraub::tests::bars(2000);
        let x = Ctx::new(&r, &r, &[]);
        for cl in [0.0, 1.0] {
            let mut p = Params::new();
            p.insert("close".into(), cl);
            let t = run("scaancb_8", &x, 0, r.len() - 1, &p).unwrap();
            assert!(t.len() > 5);
            for w in t.windows(2) { assert_eq!(w[0].exit_i, w[1].legs[0].i); }
        }
    }
}

// Instrument admission: S&C Aan channel breakout 2026-11-02 (titles lane K16).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "scaancb" && reviewed(def.id),
    label: "Peter Aan tested the channel on 12 futures markets (about 5.5 years, results as a chart). Runs on indices, ETFs and stocks, US and India, are a declared application.",
    basis: "Technical Analysis of Stocks & Commodities V.7:9 pp.295-297 (Channel breakout); independent check authors/titles-1030/K16w/k16_ref.py",
};
