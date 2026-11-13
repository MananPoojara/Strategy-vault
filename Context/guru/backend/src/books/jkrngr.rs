//! Joe Krutsinger, Ranger (joekrut.com), TradeStation code "Open Code.txt" (research share USB HDD 1/TradeStation Systems & Indicators/
//! Joe Krutsinger - Ranger for TS (joekrut.com)/Open Code.rar -> Open Code.txt; copy authors/batch-j/J3w/ranger_krut.txt). Book id
//! `jkrngr`. Lane PD (A1 pool D, 13 Nov 2026), batch-j J3 to-code row; independent check authors/pd/w/pd_ref.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::tselhs::{stop_fill, Book};

pub const REVIEWED: &[&str] = &["jkrngr_ranger"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] Orders for the next bar from each close (Range = this bar's high - low): not long and close <= the close 2 bars ago -> buy on a stop at close + 0.6 x Range; not short and close > the close 2 bars ago -> sell short on a stop at close - 0.6 x Range. A stop fills at its level, or at the open when the bar opens beyond it; the two orders never exist together. Default p.exit = 0, the code as TradeStation runs it: its two time exits are written 'IF MarketPosition=-1 and BarsSinceEntry >=1 Then Sell Next Bar at Market' and 'IF MarketPosition=1 and BarsSinceEntry >=1 Then Buy to Cover Next Bar at Market'; 'Sell' only closes a long and 'Buy to Cover' only closes a short, so neither can act and the system is stop-and-reverse (literal platform semantics are the default, R4 #218). Option p.exit = 1, the evident intent: a position is closed at the next bar's open once it has been held one bar (the close after the entry bar decides, the open after that fills); an entry stop placed at the same close can still fill later that bar. Held (R4 #19): 'SetStopContract; SetStopLoss(Amount)' with Amount 650, a dollar stop per contract of an unstated contract. The code prints no test results. Runs on indices, ETFs and stocks, US and India, are a declared application.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "jkrngr_ranger", name: "Krutsinger Ranger: after a close at or under the close 2 bars ago buy a stop 0.6 range above the close (short mirrored), stop and reverse", group: "TradeStation code: Krutsinger Ranger", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "jkrngr", rules: &[
            "E|\"IF MarketPosition<>1 and C <= C[2] Then Buy Next Bar C + (Range * 0.6) Stop;\" (Open Code.txt)",
            "E|\"IF MarketPosition<>-1 and C > C[2] Then Sell Short Next Bar C - (Range * 0.6) Stop;\" (Open Code.txt)",
            "X|\"IF MarketPosition=-1 and BarsSinceEntry >=1 Then Sell Next Bar at Market; IF MarketPosition=1 and BarsSinceEntry >=1 Then Buy to Cover Next Bar at Market;\" - as written these cannot act ('Sell' closes longs only, 'Buy to Cover' shorts only): stop and reverse by default; p.exit = 1 applies the one-bar hold",
            "X|HELD: \"SetStopContract; SetStopLoss(Amount);\" Amount(650) - a dollar amount per contract of an unstated contract (R4 #19)"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    reviewed(id).then_some(("jkrngr_ranger", "Krutsinger Ranger (code)", "Open Code.txt"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    reviewed(id).then(|| vec![
        ParamDef { key: "k", label: "Stop distance, fraction of the bar's range (0.6 printed)", def: 0.6, min: 0.1, max: 2., step: 0.1 },
        ParamDef { key: "exit", label: "Time exit: 0 = as TradeStation runs the code (the exit lines cannot act: stop and reverse), 1 = the evident intent (out at the next open after one bar held)", def: 0., min: 0., max: 1., step: 1. }])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let k = p.get("k").copied().unwrap_or(0.6);
    let timed = p.get("exit").copied().unwrap_or(0.) >= 0.5;
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let mut pos = Book::new();
    let mut order: Option<(f64, f64)> = None; // (direction, stop level) for this bar
    let mut out_at_open = false;
    for i in from.max(2)..=to {
        if out_at_open { pos.exit(i, o[i], "one bar held · next open"); out_at_open = false; }
        if let Some((d, lv)) = order.take() {
            if pos.side() != d { if let Some(px) = stop_fill(x, i, d, lv) { pos.enter(d, i, px, "stop"); } }
        }
        if i == to { break; }
        let side = pos.side();
        if timed && side != 0. && pos.entry_bar().is_some_and(|e| i > e) { out_at_open = true; }
        let rg = h[i] - l[i];
        if side != 1. && c[i] <= c[i - 2] { order = Some((1., c[i] + rg * k)); }
        else if side != -1. && c[i] > c[i - 2] { order = Some((-1., c[i] - rg * k)); }
    }
    Some(pos.done(to, c[to]))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ranger_stops() {
        let r = super::super::ctm::tests::walk(3000);
        let x = Ctx::new(&r, &r, &[]);
        let t = run("jkrngr_ranger", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(t.len() > 5 && t.iter().any(|z| z.dir > 0.) && t.iter().any(|z| z.dir < 0.));
        for w in t.windows(2) { assert_eq!(w[1].legs[0].i, w[0].exit_i, "stop and reverse"); }
        let mut p = Params::new(); p.insert("exit".into(), 1.);
        let t = run("jkrngr_ranger", &x, 0, r.len() - 1, &p).unwrap();
        assert!(t.iter().filter(|z| !z.open && z.reason.contains("one bar")).all(|z| z.exit_i == z.legs[0].i + 2));
        assert!(family_of("jkrngr_ranger").is_some());
    }
}

// Instrument admission: Krutsinger Ranger code 2026-11-13 (lane PD).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "jkrngr" && reviewed(def.id),
    label: "Krutsinger's Ranger code names no market; daily runs on indices, ETFs and stocks, US and India, are a declared application (the $650 contract stop is held).",
    basis: "Joe Krutsinger - Ranger for TS (joekrut.com)/Open Code.rar -> Open Code.txt (research share USB HDD 1); independent check authors/pd/w/pd_ref.py",
};
