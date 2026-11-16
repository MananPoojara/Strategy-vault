//! Anonymous, "The Black or White Trading Model" (three Word documents: "Black-White Pg 1.doc", "Black-White Pg2.doc"
//! with hand-drawn close-channel examples, "Black-White pg 3.doc" weekly version). Book id `bwtm`. Source: research drive
//! "USB HDD 2/tRADING KNO-2/Black or White Trading Model/".
//! The colour rules are mechanical (Pg2 examples 1-3 show support/resistance = the close of the most recent
//! counter-trend close), but the signals are not fully specified, so both models are NotTestable (see AMB).
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "bwtm";

const DNOTE: &str = "[R1-1116, R4 #297/#339: adapted, 4 unstated details, UNVERIFIED] Close-only model; no market, test period or result is given. Stated: support / resistance = the close of the most recent lower / higher close of the last five days (Pg2 examples 1-3); the channel turns black when a close is below that support, white when a close is above that resistance. Buy at the close of the second consecutive higher close counting the colour-change day as day one; sell at the close of the second consecutive lower close after the change to black (Pg2; Pg1 prints 'lower white close', read as the typo). Unstated in the book, vault readings: (1) 'Sell' = stop-and-reverse to short (p.short = 0: exit only, long-only); (2) if the close after the colour change is not higher (lower), the signal is skipped (no later pair is awaited); (3) an unchanged close neither counts toward nor breaks the 'second consecutive' count and does not change the colour; (4) if no counter-trend close occurred in the last five days there is no support / resistance and the colour stays. Warm-up (a convention, not counted): the colour is undefined until the first close beyond the support (black) or the resistance (white), and no trade is taken before it.";
const WNOTE: &str = "[R1-1116, R4 #297/#339: adapted, 4 unstated details, UNVERIFIED] Same colour model on weekly closes (the last session of each calendar week; a final week with no later session is not a close). Stated (pg3): a weekly close lower than the week before is the support; a later weekly close below it turns the channel black; the second consecutive lower weekly close after the change is the sell; a buy is the opposite. Unstated, vault readings: (1) sell = stop-and-reverse to short (p.short = 0: exit only); (2) the colour-change week counts as week one, as in the daily model (the daily text says so; pg3 does not); (3) a week that does not continue the new colour's direction cancels the signal, an unchanged week neither counts nor cancels; (4) the support / resistance is the latest lower / higher weekly close with no five-day limit (pg3 has none); with none, the colour stays. Warm-up convention (not counted): the colour is undefined until the first change. Fills at the signal week's last close; no market, period or result is given.";
const AMB: &str = "rules incomplete/ambiguous: (1) 'Sell' is never defined as exit-only or stop-and-reverse (no short rule, no long exit other than the sell signal); (2) no rule for when the close after the colour change is not higher (lower): wait for a later pair of consecutive higher closes, or skip the signal; (3) 'use [unchanged closes] when applicable to generate buy and sell signals' does not say whether an unchanged close counts toward or breaks the 'second consecutive' count; (4) 'most recent support/resistance of the last five days': what happens when no counter-trend close occurred in the last five days. No code, trade list or results resolve these";

const DAILY: &[&str] = &[
    "F|Pg2 #1: \"With each higher close, the market is indicating the most recent resistance, with each additional higher close (channel) being white.\"",
    "F|Pg2 #2: \"When the market closes lower this becomes the most recent Support; when the market closes below the most recent Support of the last five days, the close (channel) changes to black.\"",
    "F|Pg2 #3: \"When the market reverses and closes above the most recent resistance of the last five days, the close (channel) changes back to white.\" (example: 80.90, 80.50, 80.00, 80.30 black, 80.70 white = above the 80.30 counter-trend close)",
    "F|Pg2 #4: \"Treat unchanged closings as if they were the same color as the day prior and use when applicable for buy and sell signals.\"",
    "E|Pg1: \"Buy on the second consecutive higher white close after the market has changed from black to white. I would start or include the day it changed from black to white as day one, so if the next day (2nd day) it closed higher.. that would be my buy day on the close.\"",
    "X|Pg2: \"SELL on the Second consecutive lower black close after the market has changed from white to black.\" (Pg1 prints 'lower white close'; Pg2 example: 42.80 support, 42.50 black, sell 42.30)",
];
const WEEKLY: &[&str] = &[
    "F|pg3: \"If prices are moving in a WHITE channel (uptrend) and a weekly close closes lower than the previous week then that lower close is support. Then if the following week closes thru that support to the downside then the channel turns BLACK.\"",
    "X|pg3: \"The second consecutive lower weekly close after the channel turns BLACK would be trend change to the downside and time to sell.\"",
    "E|pg3: \"A BUY signal is just the opposite.\"",
];

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, rules| StrategyDef { id, name, group: "Black or White Trading Model", session: 0, dir: "Long/Short",
        status: if id == "bwtm_daily" || id == "bwtm_weekly" { Status::Live } else { Status::NotTestable(AMB) }, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules,
        notes: if id == "bwtm_daily" { DNOTE } else if id == "bwtm_weekly" { WNOTE } else { "Close-only channel model; no market, test period or results are given. Would run on daily (weekly = Friday close) index closes if the signal rules were complete." }, claim: None };
    vec![
        d("bwtm_daily", "Black or White · daily close model (Pg 1-2)", DAILY),
        d("bwtm_weekly", "Black or White · weekly (Friday close) model (pg 3)", WEEKLY),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match id {
        "bwtm_daily" => Some(("bwtm_daily", "Black or White Trading Model (daily close)", "")),
        "bwtm_weekly" => Some(("bwtm_weekly", "Black or White Trading Model (weekly close)", "")),
        _ => None,
    }
}
pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    (id == "bwtm_daily" || id == "bwtm_weekly").then(|| vec![ParamDef { key: "short", label: "Sell: 1 = reverse to short (vault reading), 0 = exit only (long-only)", def: 1., min: 0., max: 1., step: 1. }])
}
/// Signal per bar: +1 buy, -1 sell, 0 none (the colour machine runs over the whole series; trading starts at `from`).
fn signals(c: &[f64], look: usize) -> Vec<i8> {
    let n = c.len();
    let mut sig = vec![0i8; n];
    let (mut colour, mut pend, mut cnt) = (0i8, 0i8, 0u8); // colour +1 white, -1 black, 0 undefined; pend = the colour whose second close is awaited
    for i in 1..n {
        // support / resistance: the close of the most recent lower / higher close among the five days before today
        let (mut sup, mut res) = (f64::NAN, f64::NAN);
        for d in (i.saturating_sub(look).max(1)..i).rev() {
            if sup.is_nan() && c[d] < c[d - 1] { sup = c[d]; }
            if res.is_nan() && c[d] > c[d - 1] { res = c[d]; }
        }
        let new = match (colour, c[i] > res, c[i] < sup) {
            (0, true, false) => 1,
            (0, false, true) => -1,
            (-1, true, _) => 1,
            (1, _, true) => -1,
            _ => colour,
        };
        let step = if c[i] > c[i - 1] { 1i8 } else if c[i] < c[i - 1] { -1 } else { 0 };
        if new != colour {
            colour = new; pend = new; cnt = 1; // the change day is day one
        } else if pend != 0 && step != 0 {
            if step == pend && colour == pend { cnt += 1; if cnt == 2 { sig[i] = pend; pend = 0; } } else { pend = 0; } // an opposite move cancels
        }
    }
    sig
}
pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !(id == "bwtm_daily" || id == "bwtm_weekly") || from > to || to >= x.r.len() { return None; }
    for (k, v) in p { if k != "short" || !(*v == 0. || *v == 1.) { return None; } }
    let short = p.get("short").map_or(1., |v| *v) == 1.;
    let c = &x.c;
    let sig = if id == "bwtm_daily" { signals(c, 5) } else {
        // weekly: week ends = sessions whose next session is in another calendar week; the signal of the weekly series lands on that session
        let wk = |i: usize| (super::sctraub::days(&x.r[i].date) + 3).div_euclid(7);
        let ends: Vec<usize> = (0..c.len() - 1).filter(|&i| wk(i + 1) != wk(i)).collect();
        let ws = signals(&ends.iter().map(|&i| c[i]).collect::<Vec<_>>(), usize::MAX);
        let mut sig = vec![0i8; c.len()];
        for (k, &i) in ends.iter().enumerate() { sig[i] = ws[k]; }
        sig
    };
    let (mut out, mut cur): (Vec<Trade>, Option<Trade>) = (vec![], None);
    for i in from..=to {
        if sig[i] == 0 { continue; }
        if cur.as_ref().map_or(false, |t| t.dir == sig[i] as f64) { continue; }
        if cur.is_none() && sig[i] < 0 && !short { continue; }
        if let Some(mut t) = cur.take() { t.exit_i = i; t.exit_px = c[i]; t.signal_i = i; t.reason = "second consecutive close in the new colour".into(); out.push(t); }
        if i < to && (sig[i] > 0 || short) {
            cur = Some(Trade { dir: sig[i] as f64, legs: vec![Leg { i, px: c[i], w: 1. }], exit_i: i, exit_px: c[i], reason: String::new(), entry_kind: "close", signal_i: i, open: false, exits: vec![] });
        }
    }
    if let Some(mut t) = cur.take() { t.exit_i = to; t.exit_px = c[to]; t.signal_i = to; t.reason = "open".into(); t.open = true; out.push(t); }
    Some(out)
}

pub const ADMISSION_R1: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == BOOK && matches!(def.id, "bwtm_daily" | "bwtm_weekly"),
    label: "The document names no market; daily bars of indices, ETFs and stocks, US and India, are a declared application. Adapted: 4 unstated details each for the daily and weekly model (sell = reverse, a non-higher close after the change, unchanged closes, no counter-trend close in five days), labelled on the card.",
    basis: "Black-White Pg 1.doc / Pg2.doc ('The Black or White Trading Model'); R4 #297/#339; independent check authors/relax-339/R1/w/r1_ref.py",
};

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn colour_examples_and_signals() {
        // Pg2 example: 80.90, 80.50, 80.00 (below support 80.50 -> black), 80.30 (resistance), 80.70 (above it -> white)
        let c = [80.0, 80.9, 80.5, 80.0, 80.3, 80.7, 80.9, 80.4, 80.0];
        let s = signals(&c, 5);
        assert_eq!(s, vec![0, 0, 0, 0, 0, 0, 1, 0, 0]); // white on 80.70 (day one), second higher close 80.90 = buy
        let c2 = [80.0, 80.9, 80.5, 80.0, 80.3, 80.7, 80.5, 80.9];
        assert!(signals(&c2, 5).iter().all(|v| *v == 0)); // the close after the change is lower: skipped
    }
    #[test]
    fn only_the_daily_model_is_live_and_it_reverses_or_exits() {
        assert!(catalogue().iter().all(|s| true == matches!(s.status, crate::connors::Status::Live) && family_of(s.id).is_some()));
        assert!(crate::books::ADMISSIONS.iter().any(|a| (a.reviewed)(&catalogue().into_iter().find(|d| d.id == "bwtm_daily").unwrap())));
        let r = super::super::sctraub::tests::bars(600);
        let x = Ctx::new(&r, &r, &[]);
        let t = run("bwtm_daily", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(t.len() > 3 && t.windows(2).all(|w| w[1].legs[0].i == w[0].exit_i && w[1].dir == -w[0].dir));
        let lo = run("bwtm_daily", &x, 0, r.len() - 1, &Params::from([("short".to_string(), 0.)])).unwrap();
        assert!(!lo.is_empty() && lo.iter().all(|t| t.dir > 0.));
        let w = run("bwtm_weekly", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(!w.is_empty() && w.windows(2).all(|p| p[1].legs[0].i == p[0].exit_i) && w.iter().all(|t| (super::super::sctraub::days(&r[t.legs[0].i + 1].date) + 3).div_euclid(7) != (super::super::sctraub::days(&r[t.legs[0].i].date) + 3).div_euclid(7)));
        assert!(run("bwtm_x", &x, 0, 9, &Params::new()).is_none());
    }
}
