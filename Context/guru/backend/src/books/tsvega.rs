//! Vega System, TradeStation code "Open Code.txt" (research share USB HDD 1/TradeStation Systems & Indicators/Vega System for TS/
//! Open Code.rar -> Open Code.txt; no author named). Book id `tsvega`. Lane K18 (titles loop, 2 Nov 2026), to-code row 346;
//! independent check authors/titles-1030/K18w/k18_ref.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::rrsim::{highest, lowest, o, run_ts_ep6, A, P};
use super::sctraub::ticks;
use super::tsprosc::slow_kd;

pub const REVIEWED: &[&str] = &["tsvega_stoch"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] TradeStation code as printed (Length 12, OBought 70, OSold 30, SetUpLen 6). KVal / DVal = SlowKCustom / SlowDCustom(High, Low, Close, 12): FastK(12) = 100 (Close - Lowest Low) / (Highest High - Lowest Low) (0 when the range is zero), SlowK and SlowD with TradeStation's legacy smoothing (new = (2 x previous + input) / 3, seeded with the first value), as the vault's other TradeStation stochastic cards read it; option smooth 1 = 3-bar simple averages. Buy set-up: KVal < 30 and KVal crosses above DVal -> BuySetup = highest high of 2 bars, BuyStop = the bar's low, BuyPeriod = 0; sell set-up mirrored (KVal > 70 crossing below; lowest low of 2 bars, the bar's high). Each bar BuyPeriod = 7 while long, else + 1 (SellPeriod likewise); while BuyPeriod <= 6 buy next bar on a stop 1 point over BuySetup, while SellPeriod <= 6 sell short on a stop 1 point under SellSetup (1 point = 1 tick, decision 57; option tick 0). KVal crossing below DVal above 30 sets BuyPeriod = 6 and sells the long next bar at market; crossing above below 70 sets SellPeriod = 6 and covers at market. A sell stop at BuyStop and a buy-to-cover stop at SellStop are issued every bar. Fills on the vault's TradeStation daily-bar engine (rrsim). No market is named and no results are printed. Runs on daily indices, ETFs and stocks, US and India, are a declared application.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "tsvega_stoch", name: "Vega: slow stochastic (12) turns up under 30 -> buy stop over the 2-bar high for 6 bars, out on the next turn or under the set-up low (shorts mirrored)", group: "TradeStation code: Vega System (stochastic set-ups)", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "tsvega", rules: &[
            "E|IF KVal < OSold AND KVal Crosses Above DVal Then BuySetup = Highest(High, 2), BuyStop = Low, BuyPeriod = 0; IF BuyPeriod <= SetUpLen Then Buy Next Bar at BuySetup + 1 Point Stop (Length 12, OSold 30, SetUpLen 6)",
            "E|IF KVal > OBought AND KVal Crosses Below DVal Then SellSetup = Lowest(Low, 2), SellStop = High ... Sell Short Next Bar at SellSetup - 1 Point Stop (OBought 70)",
            "X|IF KVal Crosses Below DVal AND KVal > OSold Then Sell Next Bar at Market; Sell Next Bar at BuyStop Stop (short side mirrored)"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { reviewed(id).then_some(("tsvega_stoch", "Vega System (TradeStation)", "Stochastic 12, 70 / 30")) }

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    let mut v = reviewed(id).then(|| vec![p("len", "Length (12)", 12., 3., 50., 1.), p("tick", "1 point = ticks (1)", 1., 0., 10., 1.), p("smooth", "0 = legacy TradeStation SlowK/SlowD smoothing, 1 = 3-bar averages", 0., 0., 1., 1.)])?;
    crate::intraday_ep6::add_param(id, &mut v); // EP6 (#243)
    Some(v)
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let Some((from, to)) = crate::intraday_ep6::window(id, x, from, to, p) else { return Some(vec![]) }; // EP6 #243 (c)
    let len = p.get("len").copied().unwrap_or(12.).round().max(1.) as usize;
    let pt = ticks(x, p);
    let (k, d) = slow_kd(x, len, p.get("smooth").copied().unwrap_or(0.) > 0.5);
    let (ob, os, sul) = (70., 30., 6.);
    let (mut bsu, mut ssu, mut bp, mut sp, mut bstop, mut sstop) = (0., 0., 99., 99., 0., 0.);
    Some(run_ts_ep6(x, from, to, crate::intraday_ep6::on(id, p), |i, st, _now, next| {
        let up = i > 0 && k[i] > d[i] && k[i - 1] <= d[i - 1];
        let dn = i > 0 && k[i] < d[i] && k[i - 1] >= d[i - 1];
        if k[i] < os && up { bsu = highest(&x.h, 2, i); bstop = x.l[i]; bp = 0.; }
        if k[i] > ob && dn { ssu = lowest(&x.l, 2, i); sstop = x.h[i]; sp = 0.; }
        if st.pos == 1 { bp = sul + 1. } else { bp += 1. }
        if st.pos == -1 { sp = sul + 1. } else { sp += 1. }
        if bp <= sul { next.push(o(A::Buy, P::Stop(bsu + pt), "LE")); }
        if sp <= sul { next.push(o(A::Sell, P::Stop(ssu - pt), "SE")); }
        if dn && k[i] > os { bp = sul; next.push(o(A::Xl, P::Mkt, "LX")); }
        if up && k[i] < ob { sp = sul; next.push(o(A::Xs, P::Mkt, "SX")); }
        next.push(o(A::Xl, P::Stop(bstop), "LX stop"));
        next.push(o(A::Xs, P::Stop(sstop), "SX stop"));
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn vega_trades() {
        let r = super::super::sctraub::tests::bars(2500);
        let x = Ctx::new(&r, &r, &[]);
        let t = run("tsvega_stoch", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(t.len() > 3 && t.iter().any(|z| z.dir > 0.) && t.iter().any(|z| z.dir < 0.));
        assert!(run("tsvega_x", &x, 0, 5, &Params::new()).is_none());
    }
}

// Instrument admission: Vega System 2026-11-02 (titles lane K18).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "tsvega" && reviewed(def.id),
    label: "TradeStation stochastic set-up system with no named market and no published results. Runs on daily indices, ETFs and stocks, US and India, are a declared application.",
    basis: "Vega System for TS, Open Code.txt (printed code); independent check authors/titles-1030/K18w/k18_ref.py",
};
