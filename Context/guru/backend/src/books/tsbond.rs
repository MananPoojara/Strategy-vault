//! "Bond System for TS", five-line TradeStation code "Open Code.txt" (research share USB HDD 1/TradeStation Systems & Indicators/
//! Bond System for TS/Open Code.rar -> Open Code.txt; no author named). Book id `tsbond`. Lane K18 (titles loop, 2 Nov 2026),
//! to-code row 341; independent check authors/titles-1030/K18w/k18_ref.py.
use crate::connors::{rsi, Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::rrsim::{lowest, o, run_ts, sma, true_range, A, P};
use super::sctraub::{dmi, ticks};

pub const REVIEWED: &[&str] = &["tsbond_adx"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] TradeStation code as printed, written for Treasury-bond futures; the vault holds no bond futures, so it runs on daily indices, ETFs and stocks as a declared application. ADX(14), DMIPlus(14), DMIMinus(14) = Wilder's indicators; RSI(Close, 4) = Wilder's RSI; AvgTrueRange(30) = 30-bar average of the true range. At a close with ADX > 20, +DI > -DI and RSI(4) < 55: buy next bar on a stop at the close + 18 points, read as 18 ticks of the instrument (decisions 57 and 127; option tick 0 = at the close). While long (exit stops for the next bar): BarsSinceEntry < 30 (the entry bar is 0) -> sell stop at the lowest low of 30 bars; >= 30 -> at the lowest low of 2 bars; and once the close is more than 5 x ATR(30) above the entry price, 'Keep Profits!' at the lowest low of 2 bars. The suggested $2,000-3,000 money-management stop is a dollar amount and is held (decision 19). The short side is commented out in the code: long only. Fills on the vault's TradeStation daily-bar engine (rrsim). No results are printed. Runs on indices, ETFs and stocks, US and India, are a declared application.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "tsbond_adx", name: "Bond System: ADX above 20 with +DI over -DI and RSI(4) under 55 -> buy stop 18 ticks over the close, 30-bar / 2-bar low exit stops", group: "TradeStation code: Bond System (ADX/DMI + RSI pullback)", session: 0, dir: "Long", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "tsbond", rules: &[
            "E|If ADX(14) > 20 and DMIplus(14) > DMIminus(14) and RSI(close,4) < 55 then Buy Next Bar close + 18 points stop; (1 point = 1 tick)",
            "X|If barssinceentry < 30 then Sell (\"Exit 1\") Next Bar lowest(low, 30) stop; If barssinceentry >= 30 then Sell (\"Exit 2\") Next Bar lowest(low, 2) stop;",
            "X|If close > entryprice + (5*Avgtruerange(30)) then Sell (\"Keep Profits!\") Next Bar lowest(low,2) stop; $2,000-3,000 money stop held."],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { reviewed(id).then_some(("tsbond_adx", "Bond System (TradeStation)", "ADX 14 / RSI 4")) }

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    reviewed(id).then(|| vec![p("tick", "Entry offset in ticks (18 points)", 18., 0., 100., 1.), p("adx", "ADX above (20)", 20., 5., 50., 1.), p("rsi", "RSI(4) below (55)", 55., 10., 90., 1.)])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let mut q = p.clone();
    q.entry("tick".into()).or_insert(18.);
    let off = ticks(x, &q);
    let (al, rl) = (p.get("adx").copied().unwrap_or(20.), p.get("rsi").copied().unwrap_or(55.));
    let (pdi, mdi, adx) = dmi(x, 14);
    let r4 = rsi(&x.c, 4);
    let atr = sma(&true_range(x), 30);
    Some(run_ts(x, from, to, |i, st, _now, next| {
        if adx[i] > al && pdi[i] > mdi[i] && r4[i] < rl { next.push(o(A::Buy, P::Stop(x.c[i] + off), "LE")); }
        if st.pos == 1 {
            if st.bse(i) < 30 { next.push(o(A::Xl, P::Stop(lowest(&x.l, 30, i)), "Exit 1")); }
            if st.bse(i) >= 30 { next.push(o(A::Xl, P::Stop(lowest(&x.l, 2, i)), "Exit 2")); }
            if x.c[i] > st.entry_px + 5. * atr[i] { next.push(o(A::Xl, P::Stop(lowest(&x.l, 2, i)), "Keep Profits!")); }
        }
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bond_system_long_only() {
        let r = super::super::sctraub::tests::bars(2500);
        let x = Ctx::new(&r, &r, &[]);
        let t = run("tsbond_adx", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(t.len() > 3 && t.iter().all(|z| z.dir > 0.));
        assert!(run("tsbond_x", &x, 0, 5, &Params::new()).is_none());
    }
}

// Instrument admission: Bond System 2026-11-02 (titles lane K18).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "tsbond" && reviewed(def.id),
    label: "TradeStation system written for Treasury-bond futures, with no printed results. Runs on indices, ETFs and stocks, US and India, are a declared application.",
    basis: "Bond System for TS, Open Code.txt (printed code); independent check authors/titles-1030/K18w/k18_ref.py",
};
