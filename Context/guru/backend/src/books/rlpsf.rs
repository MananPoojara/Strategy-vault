//! Roy Larsen, "Trade Equity" for MetaStock (research share USB HDD 1/MetaStock/@Systems & Indicators/Trade Equity USer´s
//! Guide and Formulas for MS/TE LE Explorations Using GlobalVar.doc, 22 Aug 2002; same code in 'Trade Equity User
//! Guide.doc', section 'Sample Entry and Exit Code'). Book id `rlpsf`. One rule: the 'PS Fractal Entry' / 'PS Fractal
//! Exit' sample price formulas run through the Trade Equity LE (long, fixed position) logic with stop prices.
//! Lane K3 (titles loop, 30 Oct 2026); independent check authors/rlarsen/admission/k3f_ref.py (header).
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

/// Admitted ids (authors/rlarsen/admission/k3f_ref.py (header)).
pub const REVIEWED: &[&str] = &["rlpsf_fractal"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] The author calls these formulas samples (User Guide: 'PS Fractal Entry {sample entry price formula}'). Code read literally: F1 = the high two bars back once that bar is higher than the two bars before and the two bars after it (MetaStock ValueWhen, so F1 is updated on the bar that confirms it); the entry signal is the first bar (Alert(A=0,2)) with high >= F1 and open < F1, bought at Max(O,F1) = F1; F2 = the mirror low; the exit signal is the first bar with low <= F2, sold at Min(O,F2). Trade Equity LE logic: one long position; signals while in (entry) or out (exit) of a position are ignored; an entry and an exit signal on the same bar are a same-bar round trip, F1 in and Min(O,F2) out, as Trade Equity books it. [Reading, open question] Trade Equity's own defaults fill at the close (Input 2); the stop option (5) with these price formulas is used here because the formulas return stop prices. No commissions. Daily bars; built for any MetaStock security: indices, ETFs and US/India stocks are all a declared application.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "rlpsf_fractal", name: "PS Fractal breakout: buy the break of the last fractal high, sell the break of the last fractal low", group: "Roy Larsen Trade Equity samples",
        session: 0, dir: "Long", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: "rlpsf", rules: &[
            "E|\"{PS Fractal Entry} F1:=ValueWhen(1,H<Ref(H,-2) AND Ref(H,-1)<Ref(H,-2) AND Ref(H,-3)<Ref(H,-2) AND Ref(H,-4)<Ref(H,-2),Ref(H,-2)); A:=H>=F1 AND O<F1; If(A>0 AND Alert(A=0,2),Max(O,F1),0);\"",
            "X|\"{PS Fractal Exit} F2:=ValueWhen(1,L>Ref(L,-2) AND Ref(L,-1)>Ref(L,-2) AND Ref(L,-3)>Ref(L,-2) AND Ref(L,-4)>Ref(L,-2),Ref(L,-2)); B:=L<=F2; If(B>0 AND Alert(B=0,2),Min(O,F2),0);\"",
            "F|Trade Equity GV LE V2.7: one long position, entry and exit at the formula prices (stop option 5)"],
        notes: NOTE, claim: None }]
}
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    (id == "rlpsf_fractal").then_some(("rlpsf_fractal", "Trade Equity PS Fractal sample", "2-bar fractals, stops"))
}
pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    (id == "rlpsf_fractal").then(|| vec![ParamDef { key: "side", label: "Bars each side of a fractal (formula 2)", def: 2., min: 1., max: 5., step: 1. }])
}

/// (signal, price) per bar for a fractal breakout: up = entry (highs), else exit (lows).
fn signals(x: &Ctx, n: usize, up: bool) -> Vec<Option<f64>> {
    let (v, o) = (if up { &x.h } else { &x.l }, &x.o);
    let beyond = |a: f64, b: f64| if up { a < b } else { a > b }; // a is 'inside' b
    let mut f = f64::NAN;
    let mut prev = false;
    (0..v.len()).map(|i| {
        if i >= 2 * n && (0..=2 * n).filter(|&j| j != n).all(|j| beyond(v[i - j], v[i - n])) { f = v[i - n]; }
        let a = f.is_finite() && if up { v[i] >= f && o[i] < f } else { v[i] <= f };
        let s = (a && !prev).then(|| if up { o[i].max(f) } else { o[i].min(f) });
        prev = a;
        s
    }).collect()
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let n = p.get("side").copied().unwrap_or(2.).round().max(1.) as usize;
    let (en, ex) = (signals(x, n, true), signals(x, n, false));
    let mk = |e: usize, ep: f64, xi: usize, xp: f64, why: &str, open: bool| Trade { dir: 1., legs: vec![Leg { i: e, px: ep, w: 1. }], exit_i: xi, exit_px: xp, reason: why.into(), entry_kind: "stop", signal_i: xi, open, exits: vec![] };
    let (mut out, mut cur) = (vec![], None::<(usize, f64)>);
    for i in from..=to {
        match (cur, en[i], ex[i]) {
            (Some((e, ep)), _, Some(xp)) => { out.push(mk(e, ep, i, xp, "PS Fractal Exit stop", false)); cur = None; }
            (None, Some(ep), Some(xp)) => out.push(mk(i, ep, i, xp, "PS Fractal Exit stop (same bar)", false)),
            (None, Some(ep), None) => cur = Some((i, ep)),
            _ => {}
        }
    }
    if let Some((e, ep)) = cur { out.push(mk(e, ep, to, x.c[to], "open", true)); }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn fractal_high_break_in_fractal_low_break_out() {
        // (open, high, low, close)
        let b = [(10., 11., 9., 10.), (10., 12., 9.5, 11.), (11., 15., 10., 12.), (12., 13., 8., 9.), (9., 12., 8.5, 10.),
            (10., 11., 9., 10.), (10., 16., 9.5, 15.), (15., 17., 14., 16.), (16., 16.5, 7., 7.5)];
        let r: Vec<Row> = b.iter().enumerate().map(|(i, &(o, h, l, c))| Row { ticker: "T".into(), date: format!("2001-01-{:02}", i + 1), open: o, high: h, low: l, close: c, volume: None }).collect();
        let x = Ctx::new(&r, &r, &[]);
        let t = run("rlpsf_fractal", &x, 0, r.len() - 1, &Params::new()).unwrap();
        // F1 = 15 (bar 2's high, confirmed at bar 4); bar 6 opens 10 < 15 with high 16 -> buy at 15.
        // F2 = 8 (bar 3's low, confirmed at bar 5); bar 8's low 7 <= 8 -> sell at min(open 16, 8) = 8.
        assert_eq!(t.iter().map(|z| (z.legs[0].i, z.legs[0].px, z.exit_i, z.exit_px, z.open)).collect::<Vec<_>>(), vec![(6, 15., 8, 8., false)]);
        assert!(run("rlpsf_x", &x, 0, r.len() - 1, &Params::new()).is_none());
    }
}
