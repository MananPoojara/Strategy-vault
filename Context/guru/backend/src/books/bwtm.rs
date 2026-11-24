//! Anonymous, "The Black or White Trading Model" (three Word documents: "Black-White Pg 1.doc", "Black-White Pg2.doc"
//! with hand-drawn close-channel examples, "Black-White pg 3.doc" weekly version). Book id `bwtm`. Source: research drive
//! "USB HDD 2/tRADING KNO-2/Black or White Trading Model/".
//! The colour rules are mechanical (Pg2 examples 1-3 show support/resistance = the close of the most recent
//! counter-trend close), but the signals are not fully specified, so both models are NotTestable (see AMB).
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "bwtm";

const AMB: &str = "rules incomplete/ambiguous: (1) 'Sell' is never defined as exit-only or stop-and-reverse (no short rule, no long exit other than the sell signal); (2) no rule for when the close after the colour change is not higher (lower): wait for a later pair of consecutive higher closes, or skip the signal; (3) 'use [unchanged closes] when applicable to generate buy and sell signals' does not say whether an unchanged close counts toward or breaks the 'second consecutive' count; (4) 'most recent support/resistance of the last five days': what happens when no counter-trend close occurred in the last five days. No code, trade list or results resolve these";

const DAILY: &[&str] = &[
    "F|Pg2 #1: \"With each higher close, the market is indicating the most recent resistance, with each additional higher close (channel) being white.\"",
    "F|Pg2 #2: \"When the market closes lower this becomes the most recent Support; when the market closes below the most recent Support of the last five days, the close (channel) changes to black.\"",
    "F|Pg2 #3: \"When the market reverses and closes above the most recent resistance of the last five days, the close (channel) changes back to white.\" (example: 80.90, 80.50, 80.00, 80.30 black, 80.70 white = above the 80.30 counter-trend close)",
    "F|Pg2 #4: \"Treat unchanged closings as if they were the same color as the day prior and use when applicable for buy and sell signals.\"",
    "E|Pg1: \"Buy on the second consecutive higher white close after the market has changed from black to white. I would start or include the day it changed from black to white as day one, so if the next day (2nd day) it closed higher.. that would be my buy day on the close.\"",
    "X|Pg2: \"SELL on the Second consecutive lower black close after the market has changed from white to black.\" (Pg1 prints 'lower white close'; Pg2 example: 42.80 support, 42.50 black, sell 42.30)",
];
const WEEKLY: &[&str] = &[
    "F|pg3: \"If prices are moving in a WHITE channel (uptrend) and a weekly close closes lower than the previous week then that lower close is support. Then if the following week closes thru that support to the downside then the channel turns BLACK.\"",
    "X|pg3: \"The second consecutive lower weekly close after the channel turns BLACK would be trend change to the downside and time to sell.\"",
    "E|pg3: \"A BUY signal is just the opposite.\"",
];

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, rules, status| StrategyDef { id, name, group: "Black or White Trading Model", session: 0, dir: "Long/Short",
        status, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules,
        notes: "Close-only channel model; no market, test period or results are given. Would run on daily (weekly = Friday close) index closes if the signal rules were complete.", claim: None };
    vec![
        d("bwtm_daily", "Black or White · daily close model (Pg 1-2)", DAILY, Status::Live),
        d("bwtm_weekly", "Black or White · weekly (Friday close) model (pg 3)", WEEKLY, Status::NotTestable(AMB)),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match id {
        "bwtm_daily" => Some(("bwtm_black_white", "Black or White Trading Model", "Daily closes")),
        "bwtm_weekly" => Some(("bwtm_black_white", "Black or White Trading Model", "Weekly · not testable")),
        _ => None,
    }
}
pub const REVIEWED: &[&str] = &["bwtm_daily"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }
pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    reviewed(id).then(|| vec![ParamDef { key: "n", label: "Support / resistance window, closes (document 5)", def: 5., min: 2., max: 20., step: 1. }])
}

/// Black or White daily close model (Pg 1-2). Readings (lane I2, R4 #339/#343/#348): (1) the most recent support (resistance) of the last
/// five days = the lowest (highest) of the previous five closes; (2) the change day is day one (printed on Pg1), so the SELL (BUY) is the first
/// close that is the second lower (higher) close of a run counted from the change day, once per colour phase; (3) a SELL reverses into a
/// short, a BUY into a long (the model is symmetric); (4) signals are traded at the signal close.
pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.c.len() { return None; }
    let n = p.get("n").copied().unwrap_or(5.).round().max(2.) as usize;
    let c = &x.c;
    let mut out: Vec<Trade> = vec![];
    let mut pos: Option<(f64, usize, f64)> = None;
    let mut white: Option<bool> = None;
    let (mut run_dn, mut run_up, mut done) = (0usize, 0usize, true);
    let mk = |dir: f64, e: usize, ep: f64, xi: usize, xp: f64, open: bool| Trade { dir, legs: vec![crate::connors::Leg { i: e, px: ep, w: 1.0 }], exit_i: xi, exit_px: xp, reason: if open { "open".into() } else { "reversed on the opposite signal".into() }, entry_kind: "close", signal_i: xi, open, exits: Vec::new() };
    for t in n.max(from + 1)..=to {
        let w = &c[t - n..t];
        let (lo, hi) = (w.iter().copied().fold(f64::INFINITY, f64::min), w.iter().copied().fold(f64::NEG_INFINITY, f64::max));
        let prev = white;
        if c[t] < lo { white = Some(false); } else if c[t] > hi { white = Some(true); }
        let changed = white != prev && prev.is_some() || (prev.is_none() && white.is_some());
        if c[t] < c[t - 1] { run_dn += 1; } else { run_dn = 0; }
        if c[t] > c[t - 1] { run_up += 1; } else { run_up = 0; }
        if changed { done = false; run_dn = if c[t] < c[t - 1] { 1 } else { 0 }; run_up = if c[t] > c[t - 1] { 1 } else { 0 }; }
        let sig = match white { Some(false) if !done && run_dn >= 2 => -1.0, Some(true) if !done && run_up >= 2 => 1.0, _ => 0.0 };
        if sig != 0.0 {
            done = true;
            if let Some((d, e, ep)) = pos { if d != sig { out.push(mk(d, e, ep, t, c[t], false)); pos = None; } }
            if pos.is_none() { pos = Some((sig, t, c[t])); }
        }
    }
    if let Some((d, e, ep)) = pos { out.push(mk(d, e, ep, to, c[to], true)); }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn daily_is_live_weekly_is_not() {
        let cat = catalogue();
        assert!(matches!(cat[0].status, Status::Live) && matches!(cat[1].status, Status::NotTestable(_)));
        let r = super::super::sctraub::tests::bars(1500);
        let x = Ctx::new(&r, &r, &[]);
        let t = run("bwtm_daily", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(t.len() > 5);
        for w in t.windows(2) { assert_eq!(w[0].exit_i, w[1].legs[0].i); assert!(w[0].dir == -w[1].dir); }
    }
    /// hand-run dump for the pandas reference: Q385_CSV=<idx_data.csv> Q385_OUT8=<file> cargo test --release bwtm_dump -- --ignored
    #[test]
    #[ignore]
    fn bwtm_dump() {
        use std::io::Write;
        let data = crate::load_csv(&std::env::var("Q385_CSV").unwrap()).unwrap();
        let mut f = std::io::BufWriter::new(std::fs::File::create(std::env::var("Q385_OUT8").unwrap()).unwrap());
        writeln!(f, "id,ticker,dir,entry_date,entry_px,exit_date,exit_px,open").unwrap();
        for (tk, rows) in data.iter() {
            let x = Ctx::new(rows, &[], &[]);
            for z in run("bwtm_daily", &x, 0, rows.len() - 1, &Params::new()).unwrap() {
                writeln!(f, "bwtm_daily,{tk},{},{},{},{},{},{}", z.dir, rows[z.legs[0].i].date, z.legs[0].px, rows[z.exit_i].date, z.exit_px, z.open).unwrap();
            }
        }
    }
}

pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "bwtm" && reviewed(def.id),
    label: "The Black or White model gives no market, test period or results; the vault runs the printed daily close rule on the five NSE index series (NIFTY 50, NIFTYBANK, NIFTYIT, NIFTYMIDCAP100, NIFTYMIDCAPSELECT), a declared adaptation, indices only.",
    basis: "Black or White Trading Model Word documents Pg1-Pg3 (research share USB HDD 2/tRADING KNO-2/Black or White Trading Model/); independent check authors/conv-1123/i2w/bwtm_ref.py",
};
