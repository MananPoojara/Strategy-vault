//! TrendChannel Pro System for TS + Manual (trendchannel.com, John Tolan / Scientific Trading Solutions).
//! Sources (research drive, read in place, read-only): `USB HDD 1/TradeStation Systems & Indicators/TrendChannel Pro System
//! for TS + Manual (trendchannel.com)/CODE.rar` -> `TrendChannel System.txt` (extracted as data), and
//! `TrendChannel Operating Manual.pdf` (p.2-3: rules and the "TradeStation6 EasyLanguage" RSI(34) formula).
//! Book id `tchnl`. Notes: library/tchnl/README.md. Engine: books::rrsim.
use crate::books::rrsim::{g, o, pd, run_ts, sdef, sma, xavg, A, P};
use crate::connors::{rsi, Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "tchnl";
const BANDS: &str = "tchnl_bands";
const RSI: &str = "tchnl_rsi34";
const NOTE: &str = "Manual: 'All calculations ... are based on end-of-day settlement prices'; 'Positions are entered or exited \"at the market\" on the next open'; 'No intra-day stops are used'. \
Fills: next open; Buy/Sell Short reverse an opposite position, and an exit plus an opposite entry on one open give a reversal (exits fill first). India: instrument swap (NIFTY 50 instead of futures); index shorts hypothetical.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![
        sdef(BOOK, BANDS, "TrendChannel bands (XAverage 67 +/- 4 x average |change|)", "TrendChannel (CODE.rar)", "Long/Short", Status::Live, &[
            "F|\"Value1=XAverage(close,len); Value2=Average(AbsValue(c-c[1]),len)*PB; Value3=Value1+Value2; Value4=Value1-Value2\" with len 67, PB 4",
            "E|\"If MarketPosition<1 and close>Value3 then Buy(\\\"LE\\\") tomorrow at the market\"",
            "E|\"If MarketPosition>-1 and close<Value4 then Sell Short(\\\"SE\\\") tomorrow at the market\"",
            "X|\"If MarketPosition>0 and close<Value1 then Sell(\\\"LX\\\") tomorrow at the market\"",
            "X|\"If MarketPosition<0 and close>Value1 then Buy to Cover(\\\"SX\\\") tomorrow at the market\"",
        ], NOTE),
        sdef(BOOK, RSI, "TrendChannel RSI(34) formula (manual, 55 / 45 / 50)", "TrendChannel (Operating Manual p.3)", "Long/Short", Status::Live, &[
            "F|\"Set the daily bar RSI Parameter at 34\"; \"If currentbar > 1 then begin\"",
            "E|\"If MarketPosition<1 and Rsi(close,34)> 55 then Buy (\\\"LE\\\") tomorrow at the market\"",
            "E|\"If MarketPosition>-1 and Rsi(close,34) < 45 then Sell Short (\\\"SE\\\") tomorrow at the market\"",
            "X|\"If MarketPosition>0 and Rsi(close,34) < 50 then Sell(\\\"LX\\\") tomorrow at the market\"",
            "X|\"If MarketPosition<0 and Rsi(close,34) crosses above 50 then Buy to Cover(\\\"SX\\\") tomorrow at the market\"",
        ], "The manual's own TradeStation 6 formula; the prose says the short liquidation is 'RSI > 50', the code says 'crosses above 50' (code kept). RSI = Wilder (TradeStation RSI). \
Manual: 'All calculations ... end-of-day settlement prices'; entries/exits at the next open; no intraday stops. India: instrument swap; index shorts hypothetical."),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match id { BANDS => Some(("tchnl", "TrendChannel (trendchannel.com)", "CODE.rar bands")), RSI => Some(("tchnl", "TrendChannel (trendchannel.com)", "manual RSI(34)")), _ => None }
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    match id {
        BANDS => Some(vec![pd("len", "len (XAverage and average-change length)", 67., 10., 200., 1.), pd("pb", "PB (band multiple)", 4., 1., 8., 0.25)]),
        RSI => Some(vec![pd("rsi_len", "RSI length", 34., 5., 60., 1.), pd("up", "Long above", 55., 50., 70., 1.), pd("dn", "Short below", 45., 30., 50., 1.), pd("mid", "Exit level", 50., 40., 60., 1.)]),
        _ => None,
    }
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if from >= to || to >= x.r.len() { return None; }
    let c = &x.c;
    match id {
        BANDS => {
            let n = g(p, "len", 67.).max(1.) as usize; let pb = g(p, "pb", 4.);
            let mid = xavg(c, n);
            let ch: Vec<f64> = (0..c.len()).map(|i| if i > 0 { (c[i] - c[i - 1]).abs() } else { f64::NAN }).collect();
            let w: Vec<f64> = sma(&ch, n).iter().map(|v| v * pb).collect();
            Some(run_ts(x, from, to, |i, st, _now, next| {
                if !w[i].is_finite() { return; }
                if st.pos < 1 && c[i] > mid[i] + w[i] { next.push(o(A::Buy, P::Mkt, "LE")); }
                if st.pos > -1 && c[i] < mid[i] - w[i] { next.push(o(A::Sell, P::Mkt, "SE")); }
                if st.pos > 0 && c[i] < mid[i] { next.push(o(A::Xl, P::Mkt, "LX")); }
                if st.pos < 0 && c[i] > mid[i] { next.push(o(A::Xs, P::Mkt, "SX")); }
            }))
        }
        RSI => {
            let r = rsi(c, g(p, "rsi_len", 34.).max(1.) as usize);
            let (up, dn, mid) = (g(p, "up", 55.), g(p, "dn", 45.), g(p, "mid", 50.));
            Some(run_ts(x, from, to, |i, st, _now, next| {
                if i < 1 || !r[i].is_finite() { return; }
                if st.pos < 1 && r[i] > up { next.push(o(A::Buy, P::Mkt, "LE")); }
                if st.pos > -1 && r[i] < dn { next.push(o(A::Sell, P::Mkt, "SE")); }
                if st.pos > 0 && r[i] < mid { next.push(o(A::Xl, P::Mkt, "LX")); }
                if st.pos < 0 && r[i] > mid && r[i - 1] <= mid { next.push(o(A::Xs, P::Mkt, "SX")); }
            }))
        }
        _ => None,
    }
}
