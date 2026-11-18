//! John Forman, "The Essentials of Trading: From the Basics to Building a Winning Strategy" (Wiley 2006), ch.7 "System Design and Testing"
//! (printed p.163-200, 'Test IV - SAR Enter/SMA Exit', p.194-198; research share 'USB HDD 2/tRADING KNO-2/John Forman - The Essentials of
//! Trading/...pdf', text layer). Book id `fmess`. The book runs it on spot forex (EUR/USD style, daily, MetaStock, from 1999). Rule (p.196-197):
//! "Go Long when 0.02/0.20 setting SAR line moves below Close. Exit Long when Close crosses below the 60-day moving average. Go Short when
//! 0.02/0.20 setting SAR line moves above Close. Exit Short when Close crosses above the 60-day moving average." Entry rules supersede exit
//! rules (p.195); entries are discrete crossings, so after an SMA exit there is no new entry until the next SAR flip (p.195).
//! The same chapter's Tests I-III are modes of the same card: mode 1 = Test I (p.178: Close crosses the 92-day average to enter, the 50-day
//! average to exit), mode 2 = Test II (p.186: SAR-only, always in, 0.02/0.20), mode 3 = Test III (p.189: 92-day average cross enters, SAR exits;
//! the book's chosen 20-day / '0.9/0.50' version is p.192, offered only through the parameters: the text and Table 7.9 say step 0.09).
use super::as51ts::psar;
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "fmess";
pub const REVIEWED: &[&str] = &["fmess_sar_sma"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared adaptation, R4 #340] The book trades spot forex (daily, from 1999); the vault runs the rule on indices, ETFs and stocks it holds, always UNVERIFIED (the book's Table 7.12 results are for its forex series and are not compared). [Readings, counted honestly (R4 #343): the unstated details are (2) and (3); (1) is a built-in indicator definition (R4 #339), (4) is printed on p.195, (5) is exact arithmetic] (1) Parabolic SAR: the book prints only SAR(i) = SAR(i-1) + AF x (EP - SAR(i-1)) and 'the common default 0.02 step, 0.20 cap' (p.183-184); the reversal, start and clamping rules are taken from the published Wilder definition as implemented by TradingView ta.sar (already used by the vault's as51ts card), computed from the first bar of the data. (2) 'SAR moves below / above Close' = the SAR flips to the other side of price on that bar (price traded through it); the order is filled at that bar's close. (3) 'Close crosses below / above the 60-day moving average' = the close is below (above) the 60-bar simple average of closes (including that bar) and was not on the previous bar, filled at that close; the average must be complete inside the run window. (4) Entry beats exit on the same bar (p.195) and an entry signal in the other direction reverses an open position. Modes (default 0 = Test IV): 1 = Test I, 2 = Test II, 3 = Test III; modes 1 and 3 enter on a cross of the 92-bar average (p.178, p.189), mode 1 exits on the 50-bar average (p.178), mode 3 exits when the SAR flips to the other side of price (p.189), mode 2 is always in; the same four readings apply to every mode. The 20-day / 0.9/0.50 version of Test III (p.192) is only reachable by setting the parameters (sma_in 20, step 0.9, cap 0.5); the printed rule says 0.9 but the text and Table 7.9 select step 0.09, so neither is a default. (5) Closes are compared with the average at 1e-6 precision (whole-number arithmetic), so a close exactly equal to its average is an exact tie (a cross needs the strict side now and the other side or equal on the previous bar). Vault conventions: one position at a time; no trade before the averages exist.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "fmess_sar_sma", name: "Parabolic SAR flip enters, the 60-bar average cross exits (long and short)", group: "Forman Essentials of Trading", session: 0,
        dir: "Long/Short", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: "fmess",
        rules: &[
            "F|\"SAR(i) = SAR(i-1) + ACCELERATION x (EPRICE(i-1) - SAR(i-1))\" (p.184); \"the common default is to start the Acceleration step at 0.02 and have it capped at 0.20\" (p.183-184)",
            "E|\"Go Long when 0.02/0.20 setting SAR line moves below Close ... Go Short when 0.02/0.20 setting SAR line moves above Close\" (p.196-197)",
            "X|\"Exit Long when Close crosses below the 60-day moving average ... Exit Short when Close crosses above the 60-day moving average\" (p.197); entry rules supersede exits (p.195)",
            "A|Mode 1, Test I (p.178): \"Go Long when Close crosses above the 92-day moving average. Exit Long when Close crosses below the 50-day moving average\" and the short mirror",
            "A|Mode 2, Test II (p.186): \"Exit Shorts and Go Long when the 0.02/0.20 setting SAR line moves below the close ... Exit Longs and Go Short when the 0.02/0.20 setting SAR line moves above the close\" (always in)",
            "A|Mode 3, Test III (p.189): \"Go Long when Close crosses above the 92-day moving average. Exit Long when the 0.02/0.20 setting SAR line moves above Close\" and the short mirror; the book's chosen version (p.192) is a 20-day average with '0.9/0.50' SAR (text and Table 7.9: step 0.09), reachable through the parameters only"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match id { "fmess_sar_sma" => Some(("fmess_sar_sma", "Forman SAR enter / SMA exit", "SAR 0.02/0.20, SMA 60")), _ => None }
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    match id {
        "fmess_sar_sma" => Some(vec![p("step", "SAR acceleration step (book 0.02)", 0.02, 0.01, 0.1, 0.01), p("cap", "SAR acceleration cap (book 0.20)", 0.2, 0.1, 1., 0.1),
            p("sma", "Mode 0: exit average length (book 60)", 60., 5., 100., 5.),
            p("mode", "0 = Test IV SAR enter / SMA exit (default), 1 = Test I SMA enter / SMA exit, 2 = Test II SAR only, 3 = Test III SMA enter / SAR exit", 0., 0., 3., 1.),
            p("sma_in", "Modes 1 and 3: entry average length (book 92; p.192 alternative 20)", 92., 5., 200., 1.),
            p("sma_out", "Mode 1: exit average length (book 50)", 50., 5., 200., 1.)]),
        _ => None,
    }
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(f64::NAN, |d| d.def));
    let (step, cap, n) = (g("step"), g("cap"), g("sma").round().max(2.) as usize);
    let (mode, n_in, n_out) = (g("mode").round() as i32, g("sma_in").round().max(2.) as usize, g("sma_out").round().max(2.) as usize);
    let (_, below, _) = psar(&x.h, &x.l, &x.c, step, step, cap);
    // closes read at 1e-6 as whole numbers, so "close equals the average" is an exact tie, not a float accident (declared reading 5)
    let ci: Vec<i64> = x.c.iter().map(|v| (v * 1e6).round() as i64).collect();
    let sum = |t: usize, len: usize| -> i64 { ci[t + 1 - len..=t].iter().sum::<i64>() };
    // the longest average used by the mode must be complete at t-1 and t
    let first = from + match mode { 1 => n_in.max(n_out), 2 => 1, 3 => n_in, _ => n };
    let (mut out, mut cur) = (vec![], None::<(f64, usize, f64)>);
    for t in first.max(1)..=to {
        let flip_up = below[t] && !below[t - 1];
        let flip_dn = !below[t] && below[t - 1];
        let cross = |len: usize| -> (bool, bool) {
            let l = len as i64;
            let (s0, s1) = (sum(t - 1, len), sum(t, len));
            (ci[t] * l > s1 && ci[t - 1] * l <= s0, ci[t] * l < s1 && ci[t - 1] * l >= s0)
        };
        // want: the entry signal of the mode (1 long, -1 short, 0 none); ex: the exit signal against the open direction d
        let (want, xup, xdn) = match mode {
            1 => { let (u, d) = cross(n_in); let (xu, xd) = cross(n_out); (if u { 1. } else if d { -1. } else { 0. }, xu, xd) }
            3 => { let (u, d) = cross(n_in); (if u { 1. } else if d { -1. } else { 0. }, flip_up, flip_dn) }
            2 => (if flip_up { 1. } else if flip_dn { -1. } else { 0. }, false, false),
            _ => { let (xu, xd) = cross(n); (if flip_up { 1. } else if flip_dn { -1. } else { 0. }, xu, xd) }
        };
        if let Some((d, e, ep)) = cur {
            let rev = if matches!(mode, 1 | 3) { "SMA reversal" } else { "SAR flip" };
            let exit_why = if mode == 3 { "SAR flip" } else { "SMA cross" };
            let ex = if want != 0. && want != d { Some(rev) } else if want == 0. && ((d > 0. && xdn) || (d < 0. && xup)) { Some(exit_why) } else { None };
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
