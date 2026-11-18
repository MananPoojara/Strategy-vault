//! John Forman, "The Essentials of Trading: From the Basics to Building a Winning Strategy" (Wiley 2006), ch.7 "System Design and Testing"
//! (printed p.163-200, 'Test IV - SAR Enter/SMA Exit', p.194-198; research share 'USB HDD 2/tRADING KNO-2/John Forman - The Essentials of
//! Trading/...pdf', text layer). Book id `fmess`. The book runs it on spot forex (EUR/USD style, daily, MetaStock, from 1999). Rule (p.196-197):
//! "Go Long when 0.02/0.20 setting SAR line moves below Close. Exit Long when Close crosses below the 60-day moving average. Go Short when
//! 0.02/0.20 setting SAR line moves above Close. Exit Short when Close crosses above the 60-day moving average." Entry rules supersede exit
//! rules (p.195); entries are discrete crossings, so after an SMA exit there is no new entry until the next SAR flip (p.195).
use super::as51ts::psar;
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "fmess";
pub const REVIEWED: &[&str] = &["fmess_sar_sma"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared adaptation, R4 #340] The book trades spot forex (daily, from 1999); the vault runs the rule on indices, ETFs and stocks it holds, always UNVERIFIED (the book's Table 7.12 results are for its forex series and are not compared). [Declared readings, each an unstated detail] (1) Parabolic SAR: the book prints only SAR(i) = SAR(i-1) + AF x (EP - SAR(i-1)) and 'the common default 0.02 step, 0.20 cap' (p.184-185); the reversal, start and clamping rules are taken from the published Wilder definition as implemented by TradingView ta.sar (already used by the vault's as51ts card), computed from the first bar of the data. (2) 'SAR moves below / above Close' = the SAR flips to the other side of price on that bar (price traded through it); the order is filled at that bar's close. (3) 'Close crosses below / above the 60-day moving average' = the close is below (above) the 60-bar simple average of closes (including that bar) and was not on the previous bar, filled at that close; the average must be complete inside the run window. (4) Entry beats exit on the same bar (p.195) and a flip entry reverses an open position. Vault conventions: one position at a time; no trade before the 60-bar average exists.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "fmess_sar_sma", name: "Parabolic SAR flip enters, the 60-bar average cross exits (long and short)", group: "Forman Essentials of Trading", session: 0,
        dir: "Long/Short", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: "fmess",
        rules: &[
            "F|\"SAR(i) = SAR(i-1) + ACCELERATION x (EPRICE(i-1) - SAR(i-1))\"; \"the common default is to start the Acceleration step at 0.02 and have it capped at 0.20\" (p.184-185)",
            "E|\"Go Long when 0.02/0.20 setting SAR line moves below Close ... Go Short when 0.02/0.20 setting SAR line moves above Close\" (p.196-197)",
            "X|\"Exit Long when Close crosses below the 60-day moving average ... Exit Short when Close crosses above the 60-day moving average\" (p.197); entry rules supersede exits (p.195)"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match id { "fmess_sar_sma" => Some(("fmess_sar_sma", "Forman SAR enter / SMA exit", "SAR 0.02/0.20, SMA 60")), _ => None }
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    match id {
        "fmess_sar_sma" => Some(vec![p("step", "SAR acceleration step (book 0.02)", 0.02, 0.01, 0.1, 0.01), p("cap", "SAR acceleration cap (book 0.20)", 0.2, 0.1, 1., 0.1),
            p("sma", "Exit average length (book 60)", 60., 5., 100., 5.)]),
        _ => None,
    }
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(f64::NAN, |d| d.def));
    let (step, cap, n) = (g("step"), g("cap"), g("sma").round().max(2.) as usize);
    let (_, below, _) = psar(&x.h, &x.l, &x.c, step, step, cap);
    let sma = |t: usize| -> f64 { x.c[t + 1 - n..=t].iter().sum::<f64>() / n as f64 };
    let (mut out, mut cur) = (vec![], None::<(f64, usize, f64)>);
    let first = from + n; // the average of bar t-1 and bar t must both lie inside the window
    for t in first..=to {
        let (m0, m1) = (sma(t - 1), sma(t));
        let flip_up = below[t] && !below[t - 1];
        let flip_dn = !below[t] && below[t - 1];
        let want = if flip_up { 1. } else if flip_dn { -1. } else { 0. };
        let xl = x.c[t] < m1 && x.c[t - 1] >= m0;
        let xs = x.c[t] > m1 && x.c[t - 1] <= m0;
        if let Some((d, e, ep)) = cur {
            let ex = if want != 0. && want != d { Some("SAR flip") } else if want == 0. && ((d > 0. && xl) || (d < 0. && xs)) { Some("SMA cross") } else { None };
            if let Some(w) = ex {
                out.push(Trade { dir: d, legs: vec![Leg { i: e, px: ep, w: 1. }], exit_i: t, exit_px: x.c[t], reason: w.into(), entry_kind: "close", signal_i: t, open: false, exits: vec![] });
                cur = None;
            }
        }
        if cur.is_none() && want != 0. { cur = Some((want, t, x.c[t])); }
    }
    if let Some((d, e, ep)) = cur {
        out.push(Trade { dir: d, legs: vec![Leg { i: e, px: ep, w: 1. }], exit_i: to, exit_px: x.c[to], reason: "open".into(), entry_kind: "close", signal_i: to, open: true, exits: vec![] });
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn rejects_other_ids_and_short_windows() {
        let r: Vec<Row> = (0..5).map(|i| Row { ticker: "T".into(), date: format!("2001-01-{:02}", i + 1), open: 1., high: 1., low: 1., close: 1., volume: None }).collect();
        let x = Ctx::new(&r, &r, &[]);
        let p = Params::default();
        assert!(run("fmess_x", &x, 0, 4, &p).is_none());
        assert_eq!(run("fmess_sar_sma", &x, 0, 4, &p).unwrap().len(), 0);
    }
}

// Instrument admission: Forman (Essentials of Trading) 2026-11-19 (lane CV1c). Admission only for the module REVIEWED lists.
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "fmess" && crate::books::fmess::reviewed(def.id),
    label: "Forman tests the rule on spot forex from 1999 and prints its results for that series only; runs on indices, ETFs and US/India stocks are a declared application (R4 #340), not the author's test.",
    basis: "John Forman, The Essentials of Trading (Wiley 2006), ch.7 Test IV, p.194-198",
};
