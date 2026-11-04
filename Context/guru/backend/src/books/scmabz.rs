//! Mark Angel, "Modeling the markets with Bezier curves", Technical Analysis of Stocks & Commodities V.8:2 (1990) pp.47-55, with the
//! sidebars "Test parameters" and "Bezier equations" (research share Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008)
//! (traders.com)/VOLUMES/V08/C02/MODEL.PDF, SIDEBT.pdf, SIDEBB.pdf). Book id `scmabz`: Bezier system 1 (closes). Lane K41 (batch T7a,
//! 4 Nov 2026); independent check authors/titles-1030/K41w/k41_ref.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::sctraub::next_open;

pub const REVIEWED: &[&str] = &["scmabz_close"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

/// Control points: closes this many bars ago, control point 0 first (sidebar "Test parameters").
const LAGS: [usize; 7] = [28, 23, 18, 13, 8, 5, 0];
const SEGMENTS: usize = 30;

const NOTE: &str = "[Declared readings] Price coordinate of the Bezier curve of 7 control points (closes 28, 23, 18, 13, 8, 5 and 0 bars ago, LASTPOINT 6, 30 segments): P(u) = sum C(6,k) u^k (1-u)^(6-k) P_k. The Figure 1 PASCAL routine stores the point for u = i/30 (i = 0..30) at array index i + 1 and then copies index 2 over index 1, so 'segment s' (the s-th point) is u = (s-1)/30, and segment 1 equals segment 2. Default segment 13, the article's example ('the 13th point on a 30-point Bezier curve'); Figure 4 tested segments 2, 7, 12, 17 and 22 (options). Long when the close is above the segment value, short when below (always in after the first signal). The article does not give the fill: orders fill at the next open. Systems 2-4 (high/low curves, moving-average and RSI curves) are not coded: their segment choices are not fixed. Article tests: daily bonds, Value Line, yen, Swiss franc, heating oil, gold, soybeans (Figure 4, a chart). Runs on indices, ETFs and stocks, US and India, are a declared application.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "scmabz_close", name: "Bezier curve of 7 closes: long above the 13th of 30 curve points, short below, next open", group: "S&C: Bezier curves (Angel, V.8:2)", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "scmabz", rules: &[
            "F|\"Control point 6 is the most recent price value, Control point 5 was the value 5 periods ago ... 4 ... 8 ... 3 ... 13 ... 2 ... 18 ... 1 ... 23 ... 0 was the value 28 periods ago\"; \"All the article's systems are based on 7 vertices ... Number of segments ... The value used is 30\" (SIDEBT.pdf p.1)",
            "F|\"P(u) = sum Pi B(i,n)(u) ... B(i,n)u = C(n,i) u^i (1-u)^(n-i), C(n,i) = n!/(i!(n-i)!)\" (SIDEBB.pdf p.1); PASCAL routine: uu[i] := i/segments, bez_curve[ary_count] from ary_count 1, then bez_curve[1] := bez_curve[2] (MODEL.PDF p.3, Figure 1)",
            "E|\"Should we choose an intermediate segment, say number 13, the trading system will monitor prices against the 13th point on a 30-point Bezier curve. ... If the market is now above the Bezier value, the system is long. If the close falls below the 13th point on the current Bezier curve, the system is short.\" (MODEL.PDF p.9)",
            "X|The close crossing to the other side of the curve point reverses the position (always in); fills at the next open [book silent on fill]"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { reviewed(id).then_some(("scmabz_close", "Bezier curve system (S&C)", "Closes, segment 13")) }
pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    reviewed(id).then(|| vec![ParamDef { key: "seg", label: "Curve point (1-31; 13 = the article's example; Figure 4 tested 2, 7, 12, 17, 22)", def: 13., min: 1., max: 31., step: 1. }])
}

/// Price coordinate of curve point `seg` (1-based, the routine's array index) for the bar t.
pub fn bezier(c: &[f64], t: usize, seg: usize) -> f64 {
    let i = seg.max(2) - 1;
    let u = i as f64 / SEGMENTS as f64;
    let binom = [1., 6., 15., 20., 15., 6., 1.];
    LAGS.iter().enumerate().map(|(k, &lag)| binom[k] * u.powi(k as i32) * (1.0 - u).powi(6 - k as i32) * c[t - lag]).sum()
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let seg = p.get("seg").copied().unwrap_or(13.).round().clamp(1., 31.) as usize;
    Some(next_open(x, from, to, "Close crossed the Bezier value", |t, side| {
        if t < 28 { return side; }
        let b = bezier(&x.c, t, seg);
        if x.c[t] > b { 1.0 } else if x.c[t] < b { -1.0 } else { side }
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bezier_end_points_and_reversals() {
        let c: Vec<f64> = (0..40).map(|i| i as f64).collect();
        // all weights on control point 6 at u = 1 (segment 31), on control point 0 at u = 0 (segments 1/2 -> u = 1/30)
        assert!((bezier(&c, 39, 31) - 39.0).abs() < 1e-9);
        let u: f64 = 1.0 / 30.0;
        let want: f64 = [28usize, 23, 18, 13, 8, 5, 0].iter().enumerate().map(|(k, l)| [1., 6., 15., 20., 15., 6., 1.][k] * u.powi(k as i32) * (1. - u).powi(6 - k as i32) * (39 - l) as f64).sum();
        assert!((bezier(&c, 39, 1) - want).abs() < 1e-9 && (bezier(&c, 39, 2) - want).abs() < 1e-9);
        let r = super::super::sctraub::tests::bars(1500);
        let x = Ctx::new(&r, &r, &[]);
        let t = run("scmabz_close", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(t.len() > 5 && t.windows(2).all(|w| w[0].dir != w[1].dir && w[0].exit_i == w[1].legs[0].i));
    }
}
