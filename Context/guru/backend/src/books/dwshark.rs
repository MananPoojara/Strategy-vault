//! Walter T. Downs, "Combining Statistical and Pattern Analysis" (Technical Analysis of Stocks & Commodities, Oct 1998; the MetaStock 6.5
//! Equis 'Shark - 32' expert), as shipped in the research share package 'USB HDD 1/MetaStock/@Systems & Indicators/Shark System (Walter T.
//! Downs)/Shark System (Walter Downs).txt' (MetaStock formulas plus the forum commentary). Book id `dwshark`. Lane CV1 (J batch part B).
//! Pattern ('shark fin', printed formula): three bars of progressively smaller, nested ranges, the third bar's midpoint inside the middle
//! of the first bar's range (Symmetry .28). Buy: the close crosses above the first bar's high within 25 days of the pattern, first cross only
//! ('Buyok', 'Chk', 'ValidChk'). Exit (commentary): 'if it breaks below the low point in the pattern, there's a sell signal'.
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "dwshark";
pub const REVIEWED: &[&str] = &["dwshark_32"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Counted unstated details (R4 #343), each a labelled reading; the card is UNVERIFIED] (1) The exit is only described in prose ('breaks below the low point in the pattern'): read as a close below the lowest low of the three pattern bars (the first bar's low), filled at that close. (2) 'ValidChk = Alert(Shark=1, 25)': read as the cross bar being at most 24 bars after the pattern bar (25 bars counting the pattern bar). (3) The Expert's trend filter (5-bar over 20-bar average) is only a display on the highlights tab, not part of the buy formula, and is not applied. (4) A new pattern replaces the older one's level and low; a cross that happens while a position is open still uses up the pattern's single buy. Printed and not counted: the pattern formula, Symmetry 0.28, the first-cross-only rule. Vault conventions: signals at the bar's close fill at that close, one position at a time, long only (the book prints no short side), indicators complete inside the run window. Shipped parameters: Symmetry 0.28, 25 days.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "dwshark_32", name: "Shark-32: close above the three-bar shark-fin pattern high buys, close below its low exits", group: "Downs Shark-32 (TASC Oct 1998)", session: 0,
        dir: "Long", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: "dwshark",
        rules: &[
            "F|\"Shark:=If((H<Ref(H,-1) AND L>Ref(L,-1) AND Ref(H,-1)<Ref(H,-2) AND Ref(L,-1)>Ref(L,-2))=1, If(apex <= (Ref(H,-2)-(WB*Symmetry)) AND Apex >= (Ref(L,-2)+(WB*Symmetry)),1,0),0)\" with Symmetry:=.28, Apex:=(H+L)/2, WB:=Ref(H,-2)-Ref(L,-2)",
            "E|\"Buyok:=Cross(C,ValueWhen(1,Shark=1,Ref(H,-2))); Buy:= Buyok=1 AND Ref(Chk,-1)=0 AND ValidChk=1\" with ValidChk:=Alert(Shark=1,25): the first close above the pattern's first-bar high within 25 days",
            "X|\"The system gives a signal if and when the stock breaks above the high point in the pattern over the next 25 days ... if it breaks below the low point in the pattern, there's a sell signal\" (forum commentary in the package text)"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match id { "dwshark_32" => Some(("dwshark_32", "Downs Shark-32", "Symmetry .28, 25 days")), _ => None }
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    match id {
        "dwshark_32" => Some(vec![p("sym", "Symmetry (printed 0.28)", 0.28, 0.05, 0.5, 0.01), p("days", "Days a pattern stays valid (printed 25)", 25., 5., 60., 1.)]),
        _ => None,
    }
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(f64::NAN, |d| d.def));
    let (sym, days) = (g("sym"), g("days").round().max(2.) as usize);
    let (h, l, c) = (&x.h, &x.l, &x.c);
    let shark = |t: usize| -> bool {
        if t < from + 2 { return false; }
        let (wb, apex) = (h[t - 2] - l[t - 2], (h[t] + l[t]) / 2.);
        h[t] < h[t - 1] && l[t] > l[t - 1] && h[t - 1] < h[t - 2] && l[t - 1] > l[t - 2] && apex <= h[t - 2] - wb * sym && apex >= l[t - 2] + wb * sym
    };
    // latest pattern so far: (bar, level = first-bar high, low = first-bar low); crosses counted since that pattern
    let (mut pat, mut crosses) = (None::<(usize, f64, f64)>, 0usize);
    let (mut out, mut cur) = (vec![], None::<(usize, f64, f64)>); // open trade: (entry bar, entry px, pattern low)
    for t in from + 2..=to {
        let prev_level = pat.map(|q| q.1);
        if shark(t) { pat = Some((t, h[t - 2], l[t - 2])); crosses = 0; }
        if let (Some((pb, lv, lo)), true) = (pat, t > from + 2) {
            if let Some((e, ep, plo)) = cur {
                if c[t] < plo {
                    out.push(Trade { dir: 1., legs: vec![Leg { i: e, px: ep, w: 1. }], exit_i: t, exit_px: c[t], reason: "below pattern low".into(), entry_kind: "close", signal_i: t, open: false, exits: vec![] });
                    cur = None;
                }
            }
            let pl = prev_level.unwrap_or(f64::NAN);
            let cross = c[t] > lv && c[t - 1] <= if pb == t { pl } else { lv };
            if cross {
                if crosses == 0 && t - pb < days && cur.is_none() { cur = Some((t, c[t], lo)); }
                crosses += 1;
            }
        }
    }
    if let Some((e, ep, _)) = cur {
        out.push(Trade { dir: 1., legs: vec![Leg { i: e, px: ep, w: 1. }], exit_i: to, exit_px: c[to], reason: "open".into(), entry_kind: "close", signal_i: to, open: true, exits: vec![] });
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn shark_pattern_then_cross_buys_and_pattern_low_exits() {
        // bars: wide bar (H 20, L 10), nested bar (H 18, L 12), nested third (H 16, L 13: apex 14.5 inside 10+.28*10=12.8..20-2.8=17.2), then close 21 > 20, then close 9 < 10
        let b = [(15., 20., 10., 15.), (15., 18., 12., 15.), (15., 16., 13., 15.), (15., 22., 14., 21.), (20., 21., 8., 9.)];
        let r: Vec<Row> = b.iter().enumerate().map(|(i, &(o, h, l, c))| Row { ticker: "T".into(), date: format!("2001-01-{:02}", i + 1), open: o, high: h, low: l, close: c, volume: None }).collect();
        let x = Ctx::new(&r, &r, &[]);
        let t = run("dwshark_32", &x, 0, 4, &Params::default()).unwrap();
        assert_eq!(t.len(), 1);
        assert_eq!((t[0].legs[0].i, t[0].legs[0].px, t[0].exit_i, t[0].exit_px), (3, 21., 4, 9.));
        assert!(run("dwshark_x", &x, 0, 4, &Params::default()).is_none());
    }
}

// Instrument admission: Walter Downs Shark-32, 2026-11-19 (lane CV1, J part B). Admission only for the module REVIEWED lists.
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "dwshark" && crate::books::dwshark::reviewed(def.id),
    label: "Downs prints the formulas for MetaStock stocks and indices and no test results; runs on indices, ETFs and US/India stocks are a declared application, not the author's test.",
    basis: "Walter T. Downs, Shark-32 (TASC Oct 1998) MetaStock text 'Shark System (Walter Downs).txt'",
};
