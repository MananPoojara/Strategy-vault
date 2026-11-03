//! LTC Swing system for the S&P 500 (accuratetradingsystems.com), TradeStation code "LTC SWING.txt" (research share USB HDD 1/
//! TradeStation Systems & Indicators/AATS Swing System for S&P500 Index (accuratetradingsystems.com)/LTC SWING.txt). Book id `tsltc`.
//! Lane K18 (titles loop, 2 Nov 2026), to-code row 332; independent check authors/titles-1030/K18w/k18_ref.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::rrsim::{o, run_ts, xavg, A, P};

pub const REVIEWED: &[&str] = &["tsltc_swing"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] TradeStation code as printed (Length1 5, Length2 26, AboveBelowPts 0, ConfirmBars 5). ECO = XAverage(XAverage(Close - Open, 5), 26) / XAverage(XAverage(High - Low, 5), 26) x 100 (TradeStation exponential averages, seeded with the first value). BBar is reset to 0 when ECO crosses above 0 and then counts +1 every bar (the cross bar is 1); SBar the same for a cross below 0. Buy this bar at the close when BBar crosses above 5 (5 bars after the cross) and ECO > 0; sell short this bar at the close when SBar crosses above 5 and ECO < 0. No other exits: each signal reverses the position. The counters start at 0 on the first bar, as the code's variables do. The vendor's date lock (Date > 1060427, i.e. after 27 April 2006) is a licence check, not a rule, and is not applied; the chart text objects are display only. Fills on the vault's TradeStation daily-bar engine (rrsim). Written for the S&P 500; no results are printed. Runs on other indices, ETFs and stocks, US and India, are a declared application.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "tsltc_swing", name: "LTC Swing: ECO (smoothed close-open over high-low) stays across zero 5 bars after crossing -> enter at that close, stop and reverse", group: "TradeStation code: LTC Swing (accuratetradingsystems.com)", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "tsltc", rules: &[
            "F|ECO = XAverage(XAverage(Close-Open,Length1),Length2)/XAverage(XAverage(High-Low,Length1),Length2)*100 (5, 26)",
            "E|If BBar crosses above ConfirmBars and ECO>AboveBelowPts then Buy This bar close; If SBar crosses above ConfirmBars and ECO<-AboveBelowPts then SellShort This bar close (ConfirmBars 5, AboveBelowPts 0)",
            "X|No other exit: the opposite entry reverses the position."],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { reviewed(id).then_some(("tsltc_swing", "LTC Swing (TradeStation)", "5 / 26 / 5")) }

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    reviewed(id).then(|| vec![p("l1", "Length1 (5)", 5., 1., 50., 1.), p("l2", "Length2 (26)", 26., 2., 100., 1.), p("conf", "ConfirmBars (5)", 5., 1., 30., 1.)])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let g = |k: &str, d: f64| p.get(k).copied().unwrap_or(d).round().max(1.) as usize;
    let (l1, l2, conf) = (g("l1", 5.), g("l2", 26.), g("conf", 5.) as f64);
    let n = x.c.len();
    let co: Vec<f64> = (0..n).map(|i| x.c[i] - x.o[i]).collect();
    let hl: Vec<f64> = (0..n).map(|i| x.h[i] - x.l[i]).collect();
    let (a, b) = (xavg(&xavg(&co, l1), l2), xavg(&xavg(&hl, l1), l2));
    let eco: Vec<f64> = (0..n).map(|i| a[i] / b[i] * 100.).collect();
    let (mut bb, mut sb) = (0., 0.);
    Some(run_ts(x, from, to, |i, _st, now, _next| {
        let (pb, ps) = (bb, sb);
        if i > 0 && eco[i] > 0. && eco[i - 1] <= 0. { bb = 0.; }
        bb += 1.;
        if i > 0 && eco[i] < 0. && eco[i - 1] >= 0. { sb = 0.; }
        sb += 1.;
        if i > 0 && bb > conf && pb <= conf && eco[i] > 0. { now.push(o(A::Buy, P::Mkt, "LE")); }
        if i > 0 && sb > conf && ps <= conf && eco[i] < 0. { now.push(o(A::Sell, P::Mkt, "SE")); }
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ltc_swing_reverses() {
        let r = super::super::sctraub::tests::bars(2000);
        let x = Ctx::new(&r, &r, &[]);
        let t = run("tsltc_swing", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(t.len() > 5 && t.iter().any(|z| z.dir > 0.) && t.iter().any(|z| z.dir < 0.));
        for w in t.windows(2) { assert_eq!(w[0].exit_i, w[1].legs[0].i); }
        assert!(run("tsltc_x", &x, 0, 5, &Params::new()).is_none());
    }
}

// Instrument admission: LTC Swing 2026-11-02 (titles lane K18).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "tsltc" && reviewed(def.id),
    label: "TradeStation swing system written for the S&P 500, with no printed results. Runs on other indices, ETFs and stocks, US and India, are a declared application.",
    basis: "LTC SWING.txt (accuratetradingsystems.com, printed code); independent check authors/titles-1030/K18w/k18_ref.py",
};
