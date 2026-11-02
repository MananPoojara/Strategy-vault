//! Bullseye Futures (bullseyefutures.com) DMI / XAverage signal, two-line TradeStation code in CODE_SIGNAL.TXT (research share
//! USB HDD 1/TradeStation Systems & Indicators/Bullseye (bullseyefutures.com)/CODE_SIGNAL.TXT). Book id `tsbull`. Lane K18 (titles
//! loop, 2 Nov 2026), to-code row 331; independent check authors/titles-1030/K18w/k18_ref.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::rrsim::{o, run_ts, xavg, A, P};
use super::sctraub::{dmi, ticks};

pub const REVIEWED: &[&str] = &["tsbull_dmi"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] TradeStation code as printed (Length 12, Length1 2, s 35). DMIPlus / DMIMinus(12) = Wilder's directional indicators (Wilder smoothing from the second bar); XAverage(Close, 2) = TradeStation's exponential average (alpha 2/3, seeded with the first close). At a close with +DI > -DI and the close above its XAverage: buy next bar on a stop at the close + 35 points, read as 35 ticks of the instrument (decision 57; option tick 0 = at the close); at a close with +DI < -DI and the close below its XAverage: 'Sell at Market'. The code is in the legacy (TradeStation 2000i) syntax, where Sell opens a short and reverses a long, so the default is stop and reverse at the next open, as the vault's other TradeStation 2000i cards read it; option side 1 reads Sell as an exit only (long only). There is no stop or target. Fills on the vault's TradeStation daily-bar engine (rrsim). The vendor's market and bar size are not named and no results are printed. Runs on daily indices, ETFs and stocks, US and India, are a declared application.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "tsbull_dmi", name: "Bullseye DMI signal: +DI above -DI and the close above its 2-bar XAverage -> buy stop 35 ticks over the close, sell at market when both turn (stop and reverse)", group: "TradeStation code: Bullseye Futures DMI signal", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "tsbull", rules: &[
            "E|IF DMIPlus(LENGTH) > DMIMinus(LENGTH) and Close > XAverage(Close,LENGTH1) Then Buy close + s points Stop; (Length 12, Length1 2, s 35; 1 point = 1 tick)",
            "E|IF DMIPlus(LENGTH) < DMIMinus(LENGTH) and Close < XAverage(Close,LENGTH1) Then Sell at Market; (TradeStation 2000i: Sell opens a short and reverses a long)",
            "X|No stop or target: the buy stop and the market sell reverse each other (option side 1: the sell only closes the long)."],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { reviewed(id).then_some(("tsbull_dmi", "Bullseye DMI signal (TradeStation)", "12 / 2 / 35")) }

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    reviewed(id).then(|| vec![p("n", "DMI length (12)", 12., 2., 50., 1.), p("tick", "Stop offset in ticks (code s = 35 points)", 35., 0., 100., 1.), p("side", "1 = Sell only exits the long (long only)", 0., 0., 1., 1.)])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let n = p.get("n").copied().unwrap_or(12.).round().max(1.) as usize;
    let mut q = p.clone();
    q.entry("tick".into()).or_insert(35.);
    let off = ticks(x, &q);
    let (pdi, mdi, _) = dmi(x, n);
    let xa = xavg(&x.c, 2);
    let exit_only = p.get("side").copied().unwrap_or(0.) > 0.5;
    Some(run_ts(x, from, to, |i, _st, _now, next| {
        if pdi[i] > mdi[i] && x.c[i] > xa[i] { next.push(o(A::Buy, P::Stop(x.c[i] + off), "LE")); }
        if pdi[i] < mdi[i] && x.c[i] < xa[i] { next.push(if exit_only { o(A::Xl, P::Mkt, "LX") } else { o(A::Sell, P::Mkt, "SE") }); }
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bullseye_reverses() {
        let r = super::super::sctraub::tests::bars(2000);
        let x = Ctx::new(&r, &r, &[]);
        let t = run("tsbull_dmi", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(t.len() > 5 && t.iter().any(|z| z.dir < 0.));
        let mut q = Params::new(); q.insert("side".into(), 1.);
        assert!(run("tsbull_dmi", &x, 0, r.len() - 1, &q).unwrap().iter().all(|z| z.dir > 0.));
        assert!(run("tsbull_x", &x, 0, 5, &Params::new()).is_none());
    }
}

// Instrument admission: Bullseye DMI signal 2026-11-02 (titles lane K18).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "tsbull" && reviewed(def.id),
    label: "Two-line TradeStation futures signal with no named market and no published results. Runs on daily indices, ETFs and stocks, US and India, are a declared application.",
    basis: "Bullseye Futures CODE_SIGNAL.TXT (printed code); independent check authors/titles-1030/K18w/k18_ref.py",
};
