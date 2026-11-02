//! Joe Krutsinger, Queen and Traders Delight (joekrut.com), TradeStation open code. Research share USB HDD 1/TradeStation Systems
//! & Indicators/Joe Krutsinger - Queen for TS (joekrut.com)/Open Code.rar::Open Code.txt and .../Joe Krutsinger - Traders Delight
//! for TS  (joekrut.com)/Open Code.rar::Open Code.txt (copies authors/batch-j/J1w/queen.txt, tradersdelight.txt). Book id `jkqtd`.
//! Lane K17 (titles loop, 2 Nov 2026), to-code rows 313-314, lead decision 19; independent check authors/titles-1030/K17w/k17_ref.py.
//! Orders run in `rrsim`.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::rrsim::{g, o, pd, run_ts, sdef, A, P};

pub const REVIEWED: &[&str] = &["jkqtd_queen", "jkqtd_delight"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

macro_rules! dt { () => { " Inputs DT(1) as printed: 'If DT=1 then SetExitOnClose', so every trade is a day trade, entered on the stop during the day (or at the open if the open is already through it) and closed at that day's close; p.dt = 0 holds the position overnight instead. STOP HELD: SetStopContract; SetStopLoss($300 per contract) is a dollar amount, not converted (lead decision 19). The JoeKrut equity-curve indicators in the same file are money-management displays, not rules. One unit. The code names no market; daily indices, ETFs and US/India stocks are a declared application; the vault's standard costs." } }
const QN: &str = concat!("[Declared readings] The printed code: if Close < Close[1], buy next bar at Close[1] stop (yesterday's close, above today's); otherwise (Close >= Close[1]) sell short next bar at Close[1] stop. 'IF MarketPosition=1 Then Sell Next Bar at Market' (short mirrored): with DT = 0 an open position is closed at the next open, and the new stop order can then fill later that day.", dt!());
const TD: &str = concat!("[Declared readings] The printed code: if Close < Close[2], buy next bar at today's High stop; otherwise (Close >= Close[2]) sell short next bar at today's Low stop. The code has no other exit: with DT = 0 a position is held until an opposite stop reverses it.", dt!());

pub fn catalogue() -> Vec<StrategyDef> {
    vec![
        sdef("jkqtd", "jkqtd_queen", "Queen (Krutsinger): after a down close buy a stop at yesterday's close, after an up close sell a stop there; out at the close",
            "Queen (joekrut.com, TradeStation code)", "Long/Short", Status::Live, &[
                "E|\"IF c<c[1] Then Buy Next Bar c[1] Stop; IF c>=c[1] Then Sell Short Next Bar c[1] Stop;\"",
                "X|\"IF MarketPosition=1 Then Sell Next Bar at Market; IF MarketPosition=-1 Then Buy to Cover Next Bar at Market;\" \"Inputs:Amt(300),DT(1);\" \"If DT=1 then setexitonclose;\" (SetStopLoss(Amt) held)"],
            QN),
        sdef("jkqtd", "jkqtd_delight", "Traders Delight (Krutsinger): close under the close 2 days ago buys a stop at today's high, otherwise sells a stop at today's low; out at the close",
            "Traders Delight (joekrut.com, TradeStation code)", "Long/Short", Status::Live, &[
                "E|\"IF close < close[2] Then Buy Next Bar High Stop; IF close >=close[2] Then Sell Short Next Bar Low Stop;\"",
                "X|\"Inputs:AMOUNT(300),DT(1);\" \"If DT=1 then SetExitOnClose;\" (SetStopLoss(Amount) held)"],
            TD),
    ]
}
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match id { "jkqtd_queen" => Some(("jkqtd_queen", "Krutsinger Queen", "stop at yesterday's close, day trade")),
        "jkqtd_delight" => Some(("jkqtd_delight", "Krutsinger Traders Delight", "high/low stops, day trade")), _ => None }
}
pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    reviewed(id).then(|| vec![pd("dt", "DT: 1 = day trade, exit at the close (code); 0 = hold overnight", 1., 0., 1., 1.)])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let dt = g(p, "dt", 1.) >= 0.5;
    let queen = id == "jkqtd_queen";
    let (c, h, l) = (&x.c, &x.h, &x.l);
    Some(run_ts(x, from, to, |i, st, now, next| {
        if queen {
            if i >= 1 {
                if c[i] < c[i - 1] { next.push(o(A::Buy, P::Stop(c[i - 1]), "LE")); } else { next.push(o(A::Sell, P::Stop(c[i - 1]), "SE")); }
            }
            if !dt && st.pos == 1 { next.insert(0, o(A::Xl, P::Mkt, "LX")); }
            if !dt && st.pos == -1 { next.insert(0, o(A::Xs, P::Mkt, "SX")); }
        } else if i >= 2 {
            if c[i] < c[i - 2] { next.push(o(A::Buy, P::Stop(h[i]), "LE")); } else { next.push(o(A::Sell, P::Stop(l[i]), "SE")); }
        }
        if dt && st.pos == 1 { now.push(o(A::Xl, P::Mkt, "EOD")); }
        if dt && st.pos == -1 { now.push(o(A::Xs, P::Mkt, "EOD")); }
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn queen_and_delight() {
        super::super::nijat::tests::sane(run, REVIEWED);
        let r = super::super::nijat::tests::bars(800);
        let x = Ctx::new(&r, &r, &[]);
        for id in REVIEWED {
            let t = run(id, &x, 0, r.len() - 1, &Params::new()).unwrap();
            assert!(t.iter().all(|z| z.open || (z.exit_i == z.legs[0].i && z.exit_px == r[z.exit_i].close)), "{id}: day trades");
        }
    }
}

// Instrument admission: Krutsinger Queen / Traders Delight 2026-11-02 (titles lane K17).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "jkqtd" && reviewed(def.id),
    label: "Krutsinger's open TradeStation day-trade code with no market or test named ($300 stop held); runs on daily indices, ETFs and stocks, US and India, are a declared application.",
    basis: "Queen for TS and Traders Delight for TS (joekrut.com), Open Code.txt; independent check authors/titles-1030/K17w/k17_ref.py",
};
