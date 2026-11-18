//! Walter T. Downs, "Combining Statistical and Pattern Analysis" (Technical Analysis of Stocks & Commodities, Oct 1998; the MetaStock 6.5
//! Equis 'Shark - 32' expert), as shipped in the research share package 'USB HDD 1/MetaStock/@Systems & Indicators/Shark System (Walter T.
//! Downs)/Shark System (Walter Downs).txt' (MetaStock formulas plus the forum commentary). Book id `dwshark`. Lane CV1 (J batch part B).
//! Pattern ('shark fin', printed formula): three bars of progressively smaller, nested ranges, the third bar's midpoint inside the middle
//! of the first bar's range (Symmetry .28). Buy: the close crosses above the first bar's high within 25 days of the pattern, first cross only
//! ('Buyok', 'Chk', 'ValidChk'). Exit: the printed 'Shark Sell' symbol formula (Sellok = Cross(low of the latest pattern's first bar, C), first cross only, Alert(Shark=1,25)).
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "dwshark";
pub const REVIEWED: &[&str] = &["dwshark_32"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Unstated details, counted unbundled (R4 #343): four, each a declared reading; the card is UNVERIFIED] (1) The package's printed 'Shark Sell' formula (a close crossing below the latest pattern's first-bar low) is used as the LONG exit; the source's own lines suggest it was written for shorts, and no exit for a long is printed. (2) Alert(Shark=1,25) is read as the signal bar being at most 24 bars after the pattern bar (25 bars counting the pattern bar). (3) The Expert's trend filter Mov(C,5)>Mov(C,20) is only a trend display on the Trends tab, not part of the Buy or Sell formulas, and is not applied. (4) Long only: the printed Buy is the only entry, and the Sell is not used to go short. Printed, so not counted: the pattern formula and Symmetry .28; first-cross-only (Chk) per pattern, counted over every cross whether or not a position is open; a newer pattern replaces the level and low (ValueWhen) and resets the counts; a Sell needs an open position to exit anything and a position without a valid Sell (after 25 bars or after the first cross) stays open to the end of the run. Vault conventions: signals at the bar's close fill at that close, one position at a time.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "dwshark_32", name: "Shark-32: close above the three-bar shark-fin pattern high buys, close below its low exits", group: "Downs Shark-32 (TASC Oct 1998)", session: 0,
        dir: "Long", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: "dwshark",
        rules: &[
            "F|\"Shark:=If((H<Ref(H,-1) AND L>Ref(L,-1) AND Ref(H,-1)<Ref(H,-2) AND Ref(L,-1)>Ref(L,-2))=1, If(apex <= (Ref(H,-2)-(WB*Symmetry)) AND Apex >= (Ref(L,-2)+(WB*Symmetry)),1,0),0)\" with Symmetry:=.28, Apex:=(H+L)/2, WB:=Ref(H,-2)-Ref(L,-2)",
            "E|\"Buyok:=Cross(C,ValueWhen(1,Shark=1,Ref(H,-2))); Buy:= Buyok=1 AND Ref(Chk,-1)=0 AND ValidChk=1\" with ValidChk:=Alert(Shark=1,25): the first close above the pattern's first-bar high within 25 days",
            "X|\"Sellok:=Cross(ValueWhen(1,Shark=1,Ref(L,-2)),C); Sell:= Sellok=1 AND Ref(Chk,-1)=0 AND ValidChk=1\" ('Shark Sell' symbol formula), used as the long exit [declared reading]"],
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
    // the printed formulas: levels = ValueWhen(1, Shark=1, Ref(H,-2)) / Ref(L,-2); Buyok = Cross(C, levelH); Sellok = Cross(levelL, C);
    // Chk = Cum(ok) - ValueWhen(1, Shark=1, Cum(ok)) (crosses since the latest pattern); signal = ok AND Ref(Chk,-1)=0 AND Alert(Shark=1,25)
    let (mut lvl, mut cb, mut cs) = (None::<(usize, f64, f64)>, 0usize, 0usize);
    let (mut out, mut cur) = (vec![], None::<(usize, f64)>);
    for t in from + 2..=to {
        let prev = lvl;
        if shark(t) { lvl = Some((t, h[t - 2], l[t - 2])); cb = 0; cs = 0; }
        let Some((pb, lh, ll)) = lvl else { continue };
        let (ph, pl) = prev.map_or((f64::NAN, f64::NAN), |q| (q.1, q.2));
        let valid = t - pb < days;
        let sellok = ll > c[t] && pl <= c[t - 1];
        let buyok = c[t] > lh && c[t - 1] <= ph;
        if sellok {
            if cs == 0 && valid {
                if let Some((e, ep)) = cur.take() {
                    out.push(Trade { dir: 1., legs: vec![Leg { i: e, px: ep, w: 1. }], exit_i: t, exit_px: c[t], reason: "Shark Sell".into(), entry_kind: "close", signal_i: t, open: false, exits: vec![] });
                }
            }
            cs += 1;
        }
        if buyok {
            if cb == 0 && valid && cur.is_none() { cur = Some((t, c[t])); }
            cb += 1;
        }
    }
    if let Some((e, ep)) = cur {
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
