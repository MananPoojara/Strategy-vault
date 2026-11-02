//! ProScan S&P Scalping System (Jan Arps, janarps.com), TradeStation code "Code.txt" (research share USB HDD 1/TradeStation Systems &
//! Indicators/Jan Arps - ProScan S&P Scalping System (janarps.com)/Code.txt). Book id `tsprosc`. Lane K18 (titles loop, 2 Nov 2026),
//! to-code row 333; independent check authors/titles-1030/K18w/k18_ref.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::rrsim::{o, run_ts, A, P};

pub const REVIEWED: &[&str] = &["tsprosc_slowd"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] TradeStation code written for intraday S&P bars and run on daily bars as a declared application (decision 126: real variable names, no restorer header). Inputs as printed: MINBARS 44, STRENGTH 1; the PASSWORD input (5112362) feeds only the commented-out licence check and is unused. OSC = SlowD(14): FastK(14) = 100 (Close - Lowest Low) / (Highest High - Lowest Low) (0 when the range is zero), SlowK and SlowD with TradeStation's legacy smoothing (new = (2 x previous + input) / 3, seeded with the first value), as the vault's other TradeStation stochastic cards read it; option smooth 1 = 3-bar simple averages. SwingLowBar(1, OSC, 1, 2) = 1 when yesterday's OSC is at or below the bar before it and below today's (TradeStation swing rule: at least as low on the left, lower on the right); swing highs mirrored. A swing low is stored (the previous stored one moves to second place) when it is below the stored low or the stored low is at least MINBARS bars old; when it was just stored and the second stored low is at or above it, CONDITION6 buys this bar at the close. A swing high is stored the same way; CONDITION5 sells short this bar at the close when the stored high was just stored and the second one is at or below it (as printed, this test sits outside the swing-high block). Stored values start at 0, as the code's variables do, so the first stored swing waits 45 bars. The price swing variables (H1, L1 ...) are computed in the code but used by no order and are left out. Stop and reverse; no stop or target. Fills on the vault's TradeStation daily-bar engine (rrsim). No results are printed. Runs on indices, ETFs and stocks, US and India, are a declared application.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "tsprosc_slowd", name: "ProScan: SlowD(14) swing low at or below the previous stored one -> buy the close, swing high at or above it -> short (stop and reverse)", group: "TradeStation code: ProScan S&P Scalper (Jan Arps)", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "tsprosc", rules: &[
            "F|OSC = SlowD (14); VALUE1 = SwingLowBar (1, OSC, STRENGTH, STRENGTH+1); IF VALUE21 < TDL1 OR TDL1BAR-STRENGTH >= MINBARS THEN (store the swing low) (MINBARS 44, STRENGTH 1)",
            "E|IF TDL2 >= TDL1 AND TDL2BAR <>-1 AND TDL1BAR = STRENGTH THEN CONDITION6 ... BUY (\"ProScan Buy\") this bar at CLOSE",
            "E|IF TDH2 <= TDH1 AND TDH2BAR <>-1 AND TDH1BAR = STRENGTH THEN CONDITION5 ... SELLSHORT (\"ProScan SELL\") this bar at CLOSE",
            "X|No stop or target: each signal reverses the position (stop and reverse)."],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { reviewed(id).then_some(("tsprosc_slowd", "ProScan Scalper (TradeStation)", "SlowD 14, MINBARS 44")) }

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    reviewed(id).then(|| vec![p("minbars", "MINBARS (44)", 44., 2., 200., 1.), p("smooth", "0 = legacy TradeStation SlowK/SlowD smoothing, 1 = 3-bar averages", 0., 0., 1., 1.)])
}

/// TradeStation SlowD(n): legacy smoothing (or 3-bar simple averages).
pub fn slow_d(x: &Ctx, n: usize, avg3: bool) -> Vec<f64> { slow_kd(x, n, avg3).1 }
/// TradeStation (SlowK, SlowD)(n), legacy smoothing (or 3-bar simple averages).
pub fn slow_kd(x: &Ctx, n: usize, avg3: bool) -> (Vec<f64>, Vec<f64>) {
    let len = x.c.len();
    let mut fk = vec![f64::NAN; len];
    for i in n.saturating_sub(1)..len {
        let hh = x.h[i + 1 - n..=i].iter().copied().fold(f64::NEG_INFINITY, f64::max);
        let ll = x.l[i + 1 - n..=i].iter().copied().fold(f64::INFINITY, f64::min);
        fk[i] = if hh > ll { (x.c[i] - ll) / (hh - ll) * 100. } else { 0. };
    }
    let sm = |v: &[f64]| -> Vec<f64> {
        let mut o = vec![f64::NAN; v.len()];
        for i in 0..v.len() {
            if !v[i].is_finite() { continue; }
            o[i] = if avg3 { if i >= 2 && v[i - 2].is_finite() { (v[i - 2] + v[i - 1] + v[i]) / 3. } else { f64::NAN } }
                   else if i > 0 && o[i - 1].is_finite() { (2. * o[i - 1] + v[i]) / 3. } else { v[i] };
        }
        o
    };
    let k = sm(&fk);
    let d = sm(&k);
    (k, d)
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let minbars = p.get("minbars").copied().unwrap_or(44.).round();
    let osc = slow_d(x, 14, p.get("smooth").copied().unwrap_or(0.) > 0.5);
    let (mut tdl1, mut tdl2, mut tdl1b, mut tdl2b) = (0., 0., 0., 0.);
    let (mut tdh1, mut tdh2, mut tdh1b, mut tdh2b) = (0., 0., 0., 0.);
    let (mut c6, mut c5) = (false, false);
    Some(run_ts(x, from, to, |i, _st, now, _next| {
        let swing_low = i >= 2 && osc[i - 1] <= osc[i - 2] && osc[i - 1] < osc[i];
        let swing_high = i >= 2 && osc[i - 1] >= osc[i - 2] && osc[i - 1] > osc[i];
        if swing_low {
            let v = osc[i - 1];
            if v < tdl1 || tdl1b - 1. >= minbars { tdl2b = tdl1b; tdl2 = tdl1; tdl1b = 1.; tdl1 = v; }
            if tdl2 >= tdl1 && tdl2b != -1. && tdl1b == 1. { c6 = true; c5 = false; }
        }
        if swing_high {
            let v = osc[i - 1];
            if v > tdh1 || tdh1b - 1. >= minbars { tdh2b = tdh1b; tdh2 = tdh1; tdh1b = 1.; tdh1 = v; }
        }
        if tdh2 <= tdh1 && tdh2b != -1. && tdh1b == 1. { c5 = true; c6 = false; }
        tdh1b += 1.; tdh2b += 1.; tdl1b += 1.; tdl2b += 1.;
        if c6 { c6 = false; now.push(o(A::Buy, P::Mkt, "ProScan Buy")); }
        if c5 { c5 = false; now.push(o(A::Sell, P::Mkt, "ProScan SELL")); }
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn proscan_reverses() {
        let r = super::super::sctraub::tests::bars(2500);
        let x = Ctx::new(&r, &r, &[]);
        let t = run("tsprosc_slowd", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(t.len() > 3 && t.iter().any(|z| z.dir > 0.) && t.iter().any(|z| z.dir < 0.));
        assert!(run("tsprosc_x", &x, 0, 5, &Params::new()).is_none());
    }
}

// Instrument admission: ProScan Scalper 2026-11-02 (titles lane K18).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "tsprosc" && reviewed(def.id),
    label: "TradeStation scalping code written for intraday S&P bars, with no printed results. Runs on daily indices, ETFs and stocks, US and India, are a declared application.",
    basis: "Jan Arps ProScan Code.txt (printed code); independent check authors/titles-1030/K18w/k18_ref.py",
};
