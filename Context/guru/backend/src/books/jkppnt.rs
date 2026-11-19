//! Joe Krutsinger, Position Point 2003 (joekrut.com), TradeStation code "Open Code.txt" (research share USB HDD 1/TradeStation Systems
//! & Indicators/Joe Krutsinger - Position Point for TS  (joekrut.com)/Open Code.rar -> Open Code.txt). Book id `jkppnt`. Lane K18
//! (titles loop, 2 Nov 2026), to-code row 353; independent check authors/titles-1030/K18w/k18_ref.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::rrsim::{o, sma, A, P};
use super::tstango::run_ts_pp_ep6;

pub const REVIEWED: &[&str] = &["jkppnt_code"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] TradeStation code as printed (BuyBO 2, SellBO 6, Walk 10). Range = High - Low of the signal bar. When not long, no entry filled on the signal bar (TradesToday(date) < 1, read as the vault's other Krutsinger cards read it), Close <= Close[2] and Close > Average(Close, 40): buy next bar on a stop at the close + Range x 0.10 x BuyBO; when not short, no entry filled that bar, Close > Close[2] and Close <= Average(Close, 40): sell short on a stop at the close - Range x 0.10 x SellBO. Exits as printed, with EasyLanguage precedence (AND before OR): '(MarketPosition = -1 and BarsSinceEntry >= Walk) or Close > Close[2]' -> buy to cover next bar on a stop at the close + Range x 0.10 x BuyBO; '(MarketPosition = 1 and BarsSinceEntry >= Walk) or Close <= Close[2]' -> sell next bar on a stop at the close - Range x 0.10 x SellBO (an exit acts only on a position of its side, also one entered earlier on the same bar). BarsSinceEntry = 0 on the entry bar. HELD: the $740 contract stop (SetStopContract / SetStopLoss(Amount), a dollar amount, decision 19). The JoeKrut Diff / Switcher indicators in the same file plot the equity curve and place no orders. Fills on the vault's TradeStation daily-bar engine conventions (rrsim). No results are printed. Runs on daily indices, ETFs and stocks, US and India, are a declared application.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "jkppnt_code", name: "Krutsinger Position Point: pullback (close at or below 2 bars ago) above the 40 average -> buy stop 20% of the range over the close; short side mirrored", group: "Krutsinger Position Point 2003 (code)", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "jkppnt", rules: &[
            "E|IF MarketPosition<>1 and TradesToday(date)<1 and C <= C[2] and C > Average(C,40) Then Buy next bar at C + (Range * 0.10*BuyBO) Stop; (BuyBO 2)",
            "E|IF MarketPosition<>-1 and TradesToday(date)<1 and C > C[2] and C <= Average(C,40) Then Sell Short next bar at C - (Range * 0.10*SellBO) Stop; (SellBO 6)",
            "X|IF MarketPosition=1 and (BarsSinceEntry >=Walk) OR (C <= C[2]) Then Sell next bar at C - (Range * 0.10*SellBO) Stop; cover mirrored (Walk 10). $740 SetStopLoss held."],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { reviewed(id).then_some(("jkppnt_code", "Krutsinger Position Point (code)", "2 / 6 / 10")) }

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    let mut v = reviewed(id).then(|| vec![p("buybo", "BuyBO (2)", 2., 0., 20., 1.), p("sellbo", "SellBO (6)", 6., 0., 20., 1.), p("walk", "Walk (10)", 10., 1., 50., 1.)])?;
    crate::intraday_ep6::add_param(id, &mut v); // EP6 (#243)
    Some(v)
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let Some((from, to)) = crate::intraday_ep6::window(id, x, from, to, p) else { return Some(vec![]) }; // EP6 #243 (c)
    let g = |k: &str, d: f64| p.get(k).copied().unwrap_or(d);
    let (bbo, sbo, walk) = (g("buybo", 2.), g("sellbo", 6.), g("walk", 10.));
    let c = &x.c;
    let a40 = sma(c, 40);
    Some(run_ts_pp_ep6(x, from, to, crate::intraday_ep6::on(id, p), |i, st, next| {
        if i < 2 { return; }
        let r = x.h[i] - x.l[i];
        let (up, dn) = (c[i] + r * 0.10 * bbo, c[i] - r * 0.10 * sbo);
        if st.pos != 1 && st.entries_today < 1 && c[i] <= c[i - 2] && c[i] > a40[i] { next.push(o(A::Buy, P::Stop(up), "LE")); }
        if st.pos != -1 && st.entries_today < 1 && c[i] > c[i - 2] && c[i] <= a40[i] { next.push(o(A::Sell, P::Stop(dn), "SE")); }
        let bse = (i - st.entry_i) as f64;
        if (st.pos == -1 && bse >= walk) || c[i] > c[i - 2] { next.push(o(A::Xs, P::Stop(up), "SX")); }
        if (st.pos == 1 && bse >= walk) || c[i] <= c[i - 2] { next.push(o(A::Xl, P::Stop(dn), "LX")); }
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn position_point_trades() {
        let r = super::super::sctraub::tests::bars(2500);
        let x = Ctx::new(&r, &r, &[]);
        let t = run("jkppnt_code", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(t.len() > 5 && t.iter().any(|z| z.dir > 0.) && t.iter().any(|z| z.dir < 0.));
        for w in t.windows(2) { assert!(w[1].legs[0].i >= w[0].exit_i); }
        assert!(run("jkppnt_x", &x, 0, 5, &Params::new()).is_none());
    }
}

// Instrument admission: Krutsinger Position Point 2026-11-02 (titles lane K18).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "jkppnt" && reviewed(def.id),
    label: "Krutsinger's packaged futures system code, with no printed results. Runs on daily indices, ETFs and stocks, US and India, are a declared application.",
    basis: "Joe Krutsinger, Position Point 2003, Open Code.txt (printed code); independent check authors/titles-1030/K18w/k18_ref.py",
};
