//! Jeffrey Owen Katz with Donna L. McCormick, "Trading Stocks With A Cyclical System", Technical Analysis of Stocks & Commodities
//! V.17:2 (February 1999) pp.63-67 (research share Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/
//! VOLUMES/V17/C02/010TRAD.pdf). Book id `sckzcyc`. Lane K16 (titles loop, 2 Nov 2026), to-code row 250, decision 73;
//! independent check authors/titles-1030/K16w/k16_ref.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::scchvid::pstd;
use super::tselhs::Book;

pub const REVIEWED: &[&str] = &["sckzcyc_thrust"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] The printed EasyLanguage (Figure 1) with its printed inputs THRESH 4, K 3, M 63, HLD 10 as the default (decision 73); the article's AAPL optimum (thresh 3.25, k 2, m 64, hld 5) is an option set. Value2 = C[M] - C[M+K] (a K-day thrust that happened M days ago, about one quarter), Value1 = StdDev of Value2 over 20 bars (TradeStation's StdDev, the population form). Buy at market when Value2 > THRESH x Value1, sell short at market when Value2 < -THRESH x Value1; 'If BarsSinceEntry(0) > Hld' exit at market. 'At market' = the next bar's open; a signal in the held direction is ignored (no pyramiding), an opposite one reverses. Article: AAPL 7 December 1990 - 6 November 1998 with the optimum: 17 trades, 71% profitable, +$1,832. The authors tested stocks; runs on indices and ETFs, US and India, are a declared application.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "sckzcyc_thrust", name: "Katz cyclical stock system: follow a 3-day thrust of 63 days ago larger than 4 standard deviations, exit after 10 bars", group: "S&C: Trading Stocks With A Cyclical System (Katz, V.17:2)", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "sckzcyc", rules: &[
            "F|\"INPUTS: THRESH(4), K(3), M(63), HLD(10); Value1 = StdDev (C[M]-C[M+K], 20); Value2 = C[M] - C[M+K];\" (V.17:2, PDF p.3, Figure 1)",
            "E|\"If Value2 > Thresh * Value1 Then Buy At Market; If Value2 < -Thresh * Value1 Then Sell At Market;\" (PDF p.3)",
            "X|\"If BarsSinceEntry(0) > Hld Then Begin If MarketPosition > 0 Then ExitLong At Market; If MarketPosition < 0 Then ExitShort At Market;\" (PDF p.3)"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    reviewed(id).then_some(("sckzcyc_thrust", "Katz cyclical stock system (S&C)", "4 / 3 / 63 / 10"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    reviewed(id).then(|| vec![p("thresh", "THRESH (printed 4; AAPL optimum 3.25)", 4., 0.5, 8., 0.25), p("k", "K, thrust days (3; optimum 2)", 3., 1., 10., 1.),
        p("m", "M, days ago (63; optimum 64)", 63., 1., 260., 1.), p("hld", "HLD, bars (10; optimum 5)", 10., 1., 60., 1.)])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let g = |k: &str, d: f64| p.get(k).copied().unwrap_or(d);
    let th = g("thresh", 4.);
    let (k, m, hld) = (g("k", 3.).round().max(1.) as usize, g("m", 63.).round().max(0.) as usize, g("hld", 10.).round().max(0.) as usize);
    let v2: Vec<f64> = (0..x.c.len()).map(|i| if i >= m + k { x.c[i - m] - x.c[i - m - k] } else { f64::NAN }).collect();
    let v1 = pstd(&v2, 20);
    let mut pos = Book::new();
    let mut pend: Option<f64> = None;
    for t in from..=to {
        if let Some(d) = pend.take() { if d == 0.0 { pos.exit(t, x.o[t], "time exit") } else { pos.enter(d, t, x.o[t], "open") } }
        if t == to { break; }
        let side = pos.side();
        let sig = if v2[t] > th * v1[t] { 1.0 } else if v2[t] < -th * v1[t] { -1.0 } else { 0.0 };
        let w = if sig != 0.0 && sig != side { sig } else if side != 0.0 && t - pos.entry_bar().unwrap_or(t) > hld { 0.0 } else { side };
        if w != side { pend = Some(w); }
    }
    Some(pos.done(to, x.c[to]))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn thrust_trades_exit_on_time() {
        let r = super::super::sctraub::tests::bars(3000);
        let x = Ctx::new(&r, &r, &[]);
        let mut p = Params::new();
        p.insert("thresh".into(), 1.0);
        let t = run("sckzcyc_thrust", &x, 0, r.len() - 1, &p).unwrap();
        assert!(!t.is_empty() && t.iter().all(|z| z.legs[0].px == x.o[z.legs[0].i]));
        assert!(t.iter().filter(|z| z.reason == "time exit").all(|z| z.exit_i - z.legs[0].i == 12));
    }
}

// Instrument admission: S&C Katz cyclical stock system 2026-11-02 (titles lane K16).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "sckzcyc" && reviewed(def.id),
    label: "Katz and McCormick tested the cyclical system on AAPL (1990-1998). Runs on indices, ETFs and other stocks, US and India, are a declared application.",
    basis: "Technical Analysis of Stocks & Commodities V.17:2 pp.63-67 (Trading Stocks With A Cyclical System), Figure 1 EasyLanguage; independent check authors/titles-1030/K16w/k16_ref.py",
};
