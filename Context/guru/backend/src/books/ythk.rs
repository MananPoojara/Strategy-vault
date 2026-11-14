//! YT Strategies · Heikin Ashi: the team's AmiBroker AFLs in "Old Topics/Hikenashi" ("ST BUY SELL ON HEIKIN ASHI
//! CHART WITH REAL PRICE.afl" and its "- Copy.afl"), a daily long-only Supertrend computed on Heikin Ashi candles and
//! traded at the real close. Coded from the AFLs as written, with their own default settings; there is no book PDF and
//! the folder has no trade sheets to check against.
//!
//! AmiBroker behaviour the code follows (see `ytsw.rs` for the shared conventions):
//! - HaClose = (O + H + L + C) / 4; HaOpen = AMA(Ref(HaClose, -1), 0.5), which starts on bar 1 at HaClose[0];
//!   HaHigh / HaLow take the real High / Low and the Heikin Ashi open and close (Null on bar 0);
//! - the true range uses the Heikin Ashi high, low and previous close (Null on bar 0), and the ATR is its simple
//!   average (`MA(TR, period)`), first available on bar `period`;
//! - the Supertrend is the AFL's loop from bar `period`; the band and Supertrend arrays are 0 before it (the AFL
//!   initialises them to 0, not Null), and the four-way IIf leaves 0 when none of its branches applies;
//! - the AFL's own bar loop sets Buy / Sell at the real close: buy when flat on a buy signal, otherwise sell when long
//!   on a sell signal (so no exit and re-entry on one bar). The header's SMA / ST filters, the turnover, penny,
//!   circuit and dead-stock pieces are not used by the signals and have no effect.
#![allow(dead_code)]
use crate::books::ytwh::{collect, Ev};
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "yt_hk";
pub const HA_ST: &str = "ythk_heikin_ashi_st";
pub const HA_ST_COPY: &str = "ythk_heikin_ashi_st_copy";
pub const IDS: [&str; 2] = [HA_ST, HA_ST_COPY];

fn def(id: &'static str, name: &'static str, rules: &'static [&'static str], notes: &'static str) -> StrategyDef {
    StrategyDef { id, name, group: "YT Strategies · Heikin Ashi", session: 0, dir: "Long", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim: None }
}

pub fn catalogue() -> Vec<StrategyDef> {
    vec![
        def(HA_ST, "Supertrend on Heikin Ashi, Real-Price Trades", &[
            "F|Heikin Ashi candles: HaClose = (O+H+L+C)/4, HaOpen = AMA(previous HaClose, 0.5), HaHigh = max(H, HaOpen, HaClose), HaLow = min(L, HaOpen, HaClose)",
            "F|ATR = simple average of the Heikin Ashi true range over the ATR period (default 5)",
            "F|Supertrend of the Heikin Ashi mid point (HaHigh+HaLow)/2 ± multiplier (default 1) × ATR, the AFL's band loop",
            "F|Buy signal: yesterday's HaClose < Supertrend and today's HaClose > Supertrend",
            "E|Buy at the real Close when flat",
            "X|Sell signal: yesterday's HaClose > Supertrend and today's HaClose < Supertrend",
            "X|Sell at the real Close",
        ], "ST BUY SELL ON HEIKIN ASHI CHART WITH REAL PRICE.afl, coded as the AFL is written. The SMA / ST filters listed in the AFL header are not coded in it; the turnover, penny, circuit and dead-stock pieces are computed but not used by the signals. The folder has no trade sheets, so nothing is checked against the team's runs."),
        def(HA_ST_COPY, "Supertrend on Heikin Ashi, Copy AFL (Exit Next Bar)", &[
            "F|Same Heikin Ashi candles, ATR and Supertrend as the main AFL",
            "F|Buy signal: yesterday's HaClose < Supertrend and today's HaClose > Supertrend",
            "E|Buy at the real Close when flat",
            "X|Sell signal: yesterday's HaClose > Supertrend OR today's HaClose < Supertrend (dead-stock exit off by default)",
            "X|Because the buy bar closes above the Supertrend, the sell signal is true on the next bar: every trade exits at the next bar's real Close",
        ], "ST BUY SELL ON HEIKIN ASHI CHART WITH REAL PRICE - Copy.afl, coded as written. It differs from the main AFL only in a dead-stock toggle (default NO, no effect) and the sell line, which uses OR instead of AND; as written that sells on the bar after every buy."),
    ]
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    if !IDS.contains(&id) {
        return None;
    }
    Some(vec![
        p("atr_mult", "ATR multiplier", 1.0, 1.0, 25.0, 1.0),
        p("atr_period", "ATR period", 5.0, 1.0, 25.0, 1.0),
    ])
}

/// Heikin Ashi open, high, low, close (open / high / low are None on bar 0).
pub(crate) fn heikin_ashi(x: &Ctx) -> (Vec<Option<f64>>, Vec<Option<f64>>, Vec<Option<f64>>, Vec<f64>) {
    let n = x.c.len();
    let hc: Vec<f64> = (0..n).map(|i| (x.o[i] + x.h[i] + x.l[i] + x.c[i]) / 4.0).collect();
    let mut ho = vec![None; n];
    for i in 1..n {
        ho[i] = Some(match ho[i - 1] {
            None => hc[i - 1],
            Some(prev) => 0.5 * hc[i - 1] + 0.5 * prev,
        });
    }
    let hh: Vec<Option<f64>> = (0..n).map(|i| ho[i].map(|o| x.h[i].max(hc[i].max(o)))).collect();
    let hl: Vec<Option<f64>> = (0..n).map(|i| ho[i].map(|o| x.l[i].min(hc[i].min(o)))).collect();
    (ho, hh, hl, hc)
}

/// The AFL's Supertrend on Heikin Ashi bars (0 before bar `period`, as the AFL's arrays start at 0).
pub(crate) fn ha_supertrend(x: &Ctx, period: usize, mult: f64) -> (Vec<f64>, Vec<f64>) {
    let n = x.c.len();
    let (_, hh, hl, hc) = heikin_ashi(x);
    // TR = Max(HaHigh-HaLow, Max(|HaHigh-Ref(HaClose,-1)|, |HaLow-Ref(HaClose,-1)|)), Null on bar 0
    let tr: Vec<Option<f64>> = (0..n)
        .map(|i| {
            if i == 0 {
                return None;
            }
            let (h, l) = (hh[i]?, hl[i]?);
            Some((h - l).max((h - hc[i - 1]).abs().max((l - hc[i - 1]).abs())))
        })
        .collect();
    let p = period.max(1);
    let atr: Vec<Option<f64>> = (0..n)
        .map(|i| {
            if i + 1 < p {
                return None;
            }
            let w = &tr[i + 1 - p..=i];
            if w.iter().any(|v| v.is_none()) {
                None
            } else {
                Some(w.iter().map(|v| v.unwrap()).sum::<f64>() / p as f64)
            }
        })
        .collect();
    let (mut fu, mut fl, mut st) = (vec![0.0; n], vec![0.0; n], vec![0.0; n]);
    for i in p..n {
        let (Some(h), Some(l)) = (hh[i], hl[i]) else { continue };
        let a = atr[i].unwrap_or(-1e10);
        let bu = (h + l) / 2.0 + mult * a;
        let bl = (h + l) / 2.0 - mult * a;
        fu[i] = if bu < fu[i - 1] || hc[i - 1] > fu[i - 1] { bu } else { fu[i - 1] };
        fl[i] = if bl > fl[i - 1] || hc[i - 1] < fl[i - 1] { bl } else { fl[i - 1] };
        st[i] = if st[i - 1] == fu[i - 1] && hc[i] <= fu[i] {
            fu[i]
        } else if st[i - 1] == fu[i - 1] && hc[i] >= fu[i] {
            fl[i]
        } else if st[i - 1] == fl[i - 1] && hc[i] >= fl[i] {
            fl[i]
        } else if st[i - 1] == fl[i - 1] && hc[i] <= fl[i] {
            fu[i]
        } else {
            0.0
        };
    }
    (st, hc)
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(0.0, |d| d.def));
    let n = x.c.len();
    if n == 0 || from > to || to >= n {
        return Some(Vec::new());
    }
    let (st, hc) = ha_supertrend(x, g("atr_period").round().max(1.0) as usize, g("atr_mult"));
    let copy = id == HA_ST_COPY;
    let buy = |i: usize| i > 0 && hc[i - 1] < st[i - 1] && hc[i] > st[i];
    let sell = |i: usize| {
        if copy {
            (i > 0 && hc[i - 1] > st[i - 1]) || hc[i] < st[i]
        } else {
            i > 0 && hc[i - 1] > st[i - 1] && hc[i] < st[i]
        }
    };
    let mut evs: Vec<Ev> = Vec::new();
    let mut pos = false;
    for i in 1..n {
        if !pos && buy(i) {
            evs.push(Ev { dir: 1.0, e: i, epx: x.c[i], exit: None });
            pos = true;
        } else if pos && sell(i) {
            if let Some(v) = evs.last_mut() {
                v.exit = Some((i, x.c[i], i, "Heikin Ashi close below the Supertrend"));
            }
            pos = false;
        }
    }
    Some(collect(x, evs, from, to))
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let name = catalogue().into_iter().find(|d| d.id == id)?.name;
    Some((match id {
        HA_ST => "ythk_heikin_ashi_st",
        HA_ST_COPY => "ythk_heikin_ashi_st_copy",
        _ => return None,
    }, name, "Hikenashi"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::books::ytwh::testbars::*;

    fn flat(c: f64) -> (f64, f64, f64, f64) {
        (c, c + 1.0, c - 1.0, c)
    }

    #[test]
    fn heikin_ashi_candles() {
        let r = bars(&[(10.0, 12.0, 9.0, 11.0), (11.0, 13.0, 10.0, 12.0), (12.0, 12.5, 8.0, 9.0)]);
        let x = Ctx::new(&r, &r, &[]);
        let (ho, hh, hl, hc) = heikin_ashi(&x);
        assert_eq!(hc[0], 10.5);
        assert_eq!(ho[0], None);
        assert_eq!(ho[1], Some(10.5));
        assert_eq!(ho[2], Some(0.5 * 11.5 + 0.5 * 10.5));
        assert_eq!(hh[1], Some(13.0));
        assert_eq!(hl[2], Some(8.0));
    }

    #[test]
    fn buys_on_cross_up_and_sells_on_cross_down() {
        // a fall, a rise and a fall again: one round trip at the real closes
        let mut v: Vec<(f64, f64, f64, f64)> = (0..10).map(|k| flat(100.0 - 2.0 * k as f64)).collect();
        v.extend((0..12).map(|k| flat(82.0 + 3.0 * k as f64)));
        v.extend((0..12).map(|k| flat(115.0 - 3.0 * k as f64)));
        let r = bars(&v);
        let t = go(run, HA_ST, &r, &[]);
        assert_eq!(t.len(), 1);
        let x = Ctx::new(&r, &r, &[]);
        let (st, hc) = ha_supertrend(&x, 5, 1.0);
        let (e, xi) = (t[0].0, t[0].2);
        assert!(hc[e - 1] < st[e - 1] && hc[e] > st[e]);
        assert!(hc[xi - 1] > st[xi - 1] && hc[xi] < st[xi]);
        assert_eq!((t[0].1, t[0].3), (x.c[e], x.c[xi]));
        // the copy AFL sells on the next bar
        let c = go(run, HA_ST_COPY, &r, &[]);
        assert_eq!((c[0].0, c[0].2), (e, e + 1));
    }

    #[test]
    fn every_strategy_runs() {
        smoke(run, &IDS);
    }
}
