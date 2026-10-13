//! James Wolf, "System Report - Bent Nail" (MetaStock system report, one page, digitally signed 2003.01.14).
//! Source: research drive MetaStock/@Systems & Indicators/BentNail for Ms/Bent Nail.pdf; copy in Books/James Wolf/Bent Nail/.
//! Book id "jwbent". Notes: library/jwbent/README.md.
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "jwbent";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef {
        id: "jwbent_bent_nail", name: "Bent Nail: RSI(5) vs -WillR(20), short filter -WillR(30) > RSI(26)", group: "System Report (p.1)",
        session: 0, dir: "Long/Short", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK,
        rules: &[
            "E|Enter long: \"a:=RSI(5)>(WillR(20)*(-1)); a=1;\"",
            "X|Close long: \"a:=RSI(5)<WillR(20)*(-1); b:=(WillR(30)*(-1))>RSI(26); a=1 AND b=1;\"",
            "E|Enter short: \"a:=RSI(5)<WillR(20)*(-1); b:=(WillR(30)*(-1))>RSI(26); a=1 AND b=1;\"",
            "X|Close short: \"a:=RSI(5)>(WillR(20)*(-1)); a=1;\"",
            "F|\"Positions Long and short; Entry trade price Close; Entry trade delay 0; Exit trade price Close; Exit trade delay 0; Entry/Exit commission 0 points\"",
        ],
        notes: "The close-long rule equals the enter-short rule and the close-short rule equals the enter-long rule, so a close and the opposite entry fill on the same close (stop-and-reverse, as the MetaStock System Tester does when the sell and sell-short rules match). The AutoQueue summary listed close long as only 'RSI(5) < -WillR(20)'; the page shows 'a=1 AND b=1'. RSI = Wilder RSI; WillR(n) = -100 x (HHV(H,n) - C) / (HHV(H,n) - LLV(L,n)) (MetaStock); a zero n-day range gives no signal. No result is printed. India: instrument swap; index shorts are hypothetical.",
        claim: None,
    }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    (id == "jwbent_bent_nail").then_some(("jwbent_bent_nail", "Bent Nail (Wolf)", "MetaStock report literal"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    (id == "jwbent_bent_nail").then(|| vec![p("rsi_fast", "RSI (signal)", 5., 2., 14., 1.), p("wr_fast", "Williams %R (signal)", 20., 5., 40., 1.),
        p("wr_slow", "Williams %R (short filter)", 30., 10., 60., 1.), p("rsi_slow", "RSI (short filter)", 26., 10., 50., 1.)])
}

/// -WillR(n) = 100 x (HHV(H,n) - C) / (HHV(H,n) - LLV(L,n)).
pub fn neg_willr(x: &Ctx, n: usize) -> Vec<f64> {
    (0..x.c.len()).map(|i| {
        if i + 1 < n { return f64::NAN; }
        let hh = x.h[i + 1 - n..=i].iter().copied().fold(f64::MIN, f64::max);
        let ll = x.l[i + 1 - n..=i].iter().copied().fold(f64::MAX, f64::min);
        if hh > ll { 100. * (hh - x.c[i]) / (hh - ll) } else { f64::NAN }
    }).collect()
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if id != "jwbent_bent_nail" || from >= to || to >= x.r.len() { return None; }
    let g = |k: &str, d: f64| (p.get(k).copied().unwrap_or(d).max(1.)) as usize;
    let (r5, w20, w30, r26) = (x.rsi_n(g("rsi_fast", 5.)), neg_willr(x, g("wr_fast", 20.)), neg_willr(x, g("wr_slow", 30.)), x.rsi_n(g("rsi_slow", 26.)));
    let long_sig = |i: usize| r5[i] > w20[i];
    let short_sig = |i: usize| r5[i] < w20[i] && w30[i] > r26[i];
    let mut out = vec![];
    let mut cur: Option<Trade> = None;
    for i in from..=to {
        let want = if long_sig(i) { 1. } else if short_sig(i) { -1. } else { continue };
        if cur.as_ref().is_some_and(|t| t.dir == want) { continue; }
        if let Some(mut t) = cur.take() {
            t.exit_i = i; t.exit_px = x.c[i]; t.signal_i = i;
            t.reason = if want < 0. { "Close long (= enter short) · close" } else { "Close short (= enter long) · close" }.into();
            out.push(t);
        }
        if i < to {
            cur = Some(Trade { dir: want, legs: vec![Leg { i, px: x.c[i], w: 1. }], exit_i: i, exit_px: x.c[i], reason: String::new(), entry_kind: "close", signal_i: i, open: false, exits: vec![] });
        }
    }
    if let Some(mut t) = cur { t.exit_i = to; t.exit_px = x.c[to]; t.signal_i = to; t.reason = "open".into(); t.open = true; out.push(t); }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn neg_willr_is_distance_from_the_high() {
        let r: Vec<Row> = (0..3).map(|i| Row { ticker: "T".into(), date: format!("{i}"), open: 10., high: 12., low: 8., close: [10., 11., 8.][i], volume: None }).collect();
        let x = Ctx::new(&r, &r, &[]);
        let w = neg_willr(&x, 2);
        assert!(w[0].is_nan()); assert_eq!(w[1], 25.); assert_eq!(w[2], 100.);
    }
}
