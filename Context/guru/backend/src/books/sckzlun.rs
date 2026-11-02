//! Jeffrey Owen Katz with Donna L. McCormick, "Lunar Cycles And Trading", Technical Analysis of Stocks & Commodities V.15:6
//! (June 1997) pp.261-266 (research share Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/
//! VOLUMES/V15/C06/LUNARCY.pdf). Book id `sckzlun`. Lane K16 (titles loop, 2 Nov 2026), to-code row 244, decisions 12, 73;
//! moon dates by the vault's Meeus routine (books/ctcn.rs `moon_bars`); independent check authors/titles-1030/K16w/k16_ref.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::sctraub::next_open;

pub const REVIEWED: &[&str] = &["sckzlun_delay"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] The printed TradeStation system: Value1 = 1 on a bar when a full moon falls after its date and on or before the next bar's date ('there will be a ... moon overnight'), Value2 the same for a new moon; 'If Value1[L1] > 0 Then Buy At Market', 'If Value2[L1] > 0 Then Sell At Market', a reversal system with no stops. 'At market' = the next bar's open, so with L1 = 0 the trade opens on the first bar on or after the moon date. L1 = 1, the best delay in the article's wheat test, is the default (decision 73); 0-20 are options; flip = -1 trades the phases reversed (the article's NYFE stock-index test used reversed phases with a delay of 10). Moon dates (UTC) come from the Meeus (Astronomical Algorithms, ch.49) routine the vault already uses for ctcn_moon (decision 12); the article used Scientific Consultant Services' SA_MoonPhaseDate DLL. Article: wheat 1/2/1990-2/21/1997, delay 1: 170 trades, 56% profitable, +$20,600; NYFE reversed, delay 10: 170 trades, 60%, +$75,550. Runs on indices, ETFs and stocks, US and India, are a declared application.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "sckzlun_delay", name: "Katz lunar delay: buy L1 bars after a full moon, sell short L1 bars after a new moon, at the next open (L1 = 1)", group: "S&C: Lunar Cycles And Trading (Katz, V.15:6)", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "sckzlun", rules: &[
            "E|\"If there was a full moon L1 days ago, then buy at market; if there was a new moon L1 days ago, then sell at market.\" (V.15:6, PDF p.3); code: \"If Date < FullMoonDate And Date Tomorrow >= FullMoonDate Then Value1 = 1; ... If Value1[L1] > 0 Then Buy At Market;\" (Figure 1)",
            "X|Reversal system: \"No stops or filters are used to improve the trades.\" (PDF p.3)",
            "F|\"if L1 = 1, I will be buying one day after the night of a full moon\" (PDF p.3); wheat: \"As one moves away from the delay factor of 1, the performance\" falls (PDF p.4)"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    reviewed(id).then_some(("sckzlun_delay", "Katz lunar delay (S&C)", "L1 = 1"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    reviewed(id).then(|| vec![p("l1", "Delay L1, bars (wheat best 1)", 1., 0., 20., 1.), p("flip", "1 = buy full / sell new (code); -1 = phases reversed", 1., -1., 1., 2.)])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let l1 = p.get("l1").copied().unwrap_or(1.).round().max(0.) as usize;
    let fl = if p.get("flip").copied().unwrap_or(1.) < 0. { -1.0 } else { 1.0 };
    let mb = super::ctcn::moon_bars(x);
    // Value1/Value2 sit on the bar before the first bar on/after the moon date; the order goes in L1 bars later.
    let mut sig = vec![0.0f64; x.r.len()];
    for (m, &v) in mb.iter().enumerate() {
        if v != 0 && m >= 1 { let s = m - 1 + l1; if s < sig.len() { sig[s] = if v > 0 { fl } else { -fl }; } }
    }
    Some(next_open(x, from, to, "reversed", |t, side| if sig[t] != 0.0 { sig[t] } else { side }))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn lunar_delay_reverses() {
        let r = super::super::sctraub::tests::bars(1500);
        let x = Ctx::new(&r, &r, &[]);
        let t = run("sckzlun_delay", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(t.len() > 80 && t.iter().any(|z| z.dir > 0.0) && t.iter().any(|z| z.dir < 0.0));
        for w in t.windows(2) { assert_eq!(w[0].exit_i, w[1].legs[0].i); }
    }
}

// Instrument admission: S&C Katz lunar delay 2026-11-02 (titles lane K16).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "sckzlun" && reviewed(def.id),
    label: "Katz and McCormick tested the lunar delay on wheat and NYFE futures (1990-1997). Runs on indices, ETFs and stocks, US and India, are a declared application.",
    basis: "Technical Analysis of Stocks & Commodities V.15:6 pp.261-266 (Lunar Cycles And Trading), Figure 1 code; moon dates books/ctcn.rs moon_bars (Meeus ch.49); independent check authors/titles-1030/K16w/k16_ref.py",
};
