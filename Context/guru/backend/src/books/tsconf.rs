//! Confluence system (Gary Fritz's Omega-list post, November 1998) on Dale Legan's Confluence function, TradeStation code printed
//! in full in conflu.txt (research share USB HDD 1/TradeStation Systems & Indicators/_Misc/_196/Dale Legan's Confluence function
//! for TS/conflu.txt). Book id `tsconf`. Lane K18 (titles loop, 2 Nov 2026), to-code row 325; independent check
//! authors/titles-1030/K18w/k18_ref.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::rrsim::{o, run_ts, sma, A, P, NAN};

pub const REVIEWED: &[&str] = &["tsconf_legan"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] TradeStation code as printed (inputs Price = Close, Harmonic 4, AggStops false). Confluence function (Dale Legan): STL = 2H - 1, ITL = 2 STL - 1, LTL = 2 ITL - 1 (7, 13, 25), offsets H/2, STL/2, ITL/2; DerivativeMA(p, L) = L (2 Avg(p, L) - Avg(p, L)[1]) - (L - 1) Avg(p, L - 1); value5/6/7 = (Sum(p, L - 1) + DerivativeMA(p, L)) / L, value13 uses Sum(AvgPrice, LTL - 1) as printed (AvgPrice = (O + H + L + C) / 4); sine and cosine in degrees (EasyLanguage); ErrNum, MomNum and TCNum keep their last value when no printed case applies (EasyLanguage variables); Confluence = their sum when its sign agrees with TC, else the sum / 10. System: Signal = 1 when Confluence >= 1, -1 when <= -1; entering bull mode sets a buy stop at the High + Trigger, leaving it an exit-long stop at the Low - Trigger (bear mode mirrored, sell short stop at the Low - Trigger, exit-short stop at the High + Trigger); each stop is re-issued every bar until the price trades through it. The code's Trigger (5) is a fixed price amount for 30-minute SPX, so it is held (decision 19): the default offset is zero; option trig = the code's 5 price points, literal. Values are computed from the first bar on which they exist (the post's MaxBarsBack 150 is not applied). Fills on the vault's TradeStation daily-bar engine (rrsim): stops at their level or the open beyond it; a buy stop reverses a short and a sell stop a long. The post's 30-minute SPX report is not printed in the file. Runs on daily indices, ETFs and stocks, US and India, are a declared application.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "tsconf_legan", name: "Confluence system: Legan's Confluence enters bull (bear) mode -> stop at the bar's high (low), exit stop when the mode ends", group: "TradeStation code: Confluence (Legan / Fritz, Omega list 1998)", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "tsconf", rules: &[
            "F|Confluence(Price, Harmonic): ErrNum + MomNum + TCNum (-9..+9) when its sign agrees with TC = value5 - value13, else divided by 10 (Dale Legan's function, conflu.txt)",
            "E|if (Signal = 1) and (Signal[1] <> 1) then Lstop = High + Trigger ... buy at Lstop stop; if (Signal = -1) and (Signal[1] <> -1) then Sstop = Low - Trigger ... sell at Sstop stop",
            "X|if (Signal <> 1) and (Signal[1] = 1) then Lexit = Low - Trigger ... exitlong at Lexit stop (short side mirrored); stops cleared once the price trades through them. Trigger (5 price points) held: zero offset."],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { reviewed(id).then_some(("tsconf_legan", "Confluence system (Legan)", "Harmonic 4")) }

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    reviewed(id).then(|| vec![p("harm", "Harmonic (4)", 4., 2., 15., 1.), p("trig", "Trigger in price points (code 5; held, 0)", 0., 0., 50., 0.5), p("agg", "1 = AggStops (tighten the stops)", 0., 0., 1., 1.)])
}

fn shift(v: &[f64], k: usize, i: usize) -> f64 { if i >= k { v[i - k] } else { NAN } }
fn summ(v: &[f64], n: usize) -> Vec<f64> { (0..v.len()).map(|i| if i + 1 >= n { v[i + 1 - n..=i].iter().sum::<f64>() } else { NAN }).collect() }
fn est(p: &[f64], sp: &[f64], l: usize) -> Vec<f64> {
    let (a, a1, s) = (sma(p, l), sma(p, l - 1), summ(sp, l - 1));
    let lf = l as f64;
    (0..p.len()).map(|i| (s[i] + (lf * (2. * a[i] - shift(&a, 1, i)) - (lf - 1.) * a1[i])) / lf).collect()
}
fn sc(v: f64) -> f64 { v.to_radians().sin() + v.to_radians().cos() }
fn count(v: &[f64], sig: &[f64], i: usize, num: &mut f64) {
    let (x, p, s) = (v[i], shift(v, 1, i), sig[i]);
    if x > 0. {
        if x < p && x < s { *num = 1.; }
        if x < p && x > s { *num = 2.; }
        if x > p && x < s { *num = 2.; }
        if x > p && x > s { *num = 3.; }
    }
    if x < 0. {
        if x > p && x > s { *num = -1.; }
        if x < p && x > s { *num = -2.; }
        if x > p && x < s { *num = -2.; }
        if x < p && x < s { *num = -3.; }
    }
}

/// Dale Legan's Confluence function on price `p` (Close) with AvgPrice `ap`.
pub fn confluence(p: &[f64], ap: &[f64], harm: usize) -> Vec<f64> {
    let n = p.len();
    let stl = 2 * harm - 1; let itl = 2 * stl - 1; let ltl = 2 * itl - 1;
    let (hoff, soff, ioff) = (harm / 2, stl / 2, itl / 2);
    let (havg, savg, iavg, lavg) = (sma(p, harm), sma(p, stl), sma(p, itl), sma(p, ltl));
    let (v5, v6, v7, v13) = (est(p, p, harm), est(p, p, stl), est(p, p, itl), est(p, ap, ltl));
    let momsig: Vec<f64> = (0..n).map(|i| (savg[i] - shift(&havg, hoff, i)) + (iavg[i] - shift(&savg, soff, i)) + (lavg[i] - shift(&iavg, ioff, i))).collect();
    let mom: Vec<f64> = (0..n).map(|i| (v6[i] - shift(&v5, hoff, i)) + (v7[i] - shift(&v6, soff, i)) + (v13[i] - shift(&v7, ioff, i))).collect();
    let sum: Vec<f64> = (0..n).map(|i| sc(v5[i]) + sc(v6[i]) + sc(v7[i])).collect();
    let err: Vec<f64> = (0..n).map(|i| sc(havg[i]) + sc(savg[i]) + sc(iavg[i])).collect();
    let errsum: Vec<f64> = (0..n).map(|i| {
        let (s0, h0) = (shift(&sum, soff, i), shift(&havg, soff, i));
        let c2 = (sum[i] > s0 && havg[i] < h0) || (sum[i] < s0 && havg[i] > h0);
        (sum[i] - err[i]) * if c2 { -1. } else { 1. }
    }).collect();
    let errsig = sma(&errsum, soff);
    let tc: Vec<f64> = (0..n).map(|i| v5[i] - v13[i]).collect();
    let tcsig = sma(&tc, harm);
    let (mut en, mut mn, mut tn) = (0., 0., 0.);
    (0..n).map(|i| {
        count(&errsum, &errsig, i, &mut en);
        count(&mom, &momsig, i, &mut mn);
        count(&tc, &tcsig, i, &mut tn);
        let v = en + mn + tn;
        let mut c = 0.;
        if v > 0. && tc[i] > 0. { c = v; }
        if v < 0. && tc[i] < 0. { c = v; }
        if (v > 0. && tc[i] < 0.) || (v < 0. && tc[i] > 0.) { c = v / 10.; }
        c
    }).collect()
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let harm = p.get("harm").copied().unwrap_or(4.).round().max(2.) as usize;
    let trig = p.get("trig").copied().unwrap_or(0.);
    let agg = p.get("agg").copied().unwrap_or(0.) > 0.5;
    let ap: Vec<f64> = (0..x.c.len()).map(|i| (x.o[i] + x.h[i] + x.l[i] + x.c[i]) / 4.).collect();
    let conf = confluence(&x.c, &ap, harm);
    let sig: Vec<i8> = conf.iter().map(|&c| if c <= -1. { -1 } else if c >= 1. { 1 } else { 0 }).collect();
    let (mut lstop, mut lexit, mut sstop, mut sexit) = (99999., 0., 0., 99999.);
    let (h, l) = (&x.h, &x.l);
    Some(run_ts(x, from, to, |i, _st, _now, next| {
        let s1 = if i > 0 { sig[i - 1] } else { 0 };
        if sig[i] == 1 && s1 != 1 { lstop = h[i] + trig; lexit = 0.; }
        if sig[i] == -1 && s1 != -1 { sstop = l[i] - trig; sexit = 99999.; }
        if sig[i] != 1 && s1 == 1 { lexit = l[i] - trig; lstop = 99999.; }
        if sig[i] != -1 && s1 == -1 { sexit = h[i] + trig; sstop = 0.; }
        if agg {
            if lstop != 99999. { lstop = f64::min(lstop, h[i] + trig); }
            if lexit != 0. { lexit = f64::max(lexit, l[i] - trig); }
            if sstop != 0. { sstop = f64::max(sstop, l[i] - trig); }
            if sexit != 99999. { sexit = f64::min(sexit, h[i] + trig); }
        }
        if h[i] > lstop { lstop = 99999.; }
        if l[i] < lexit { lexit = 0.; }
        if l[i] < sstop { sstop = 0.; }
        if h[i] > sexit { sexit = 99999.; }
        if lstop != 99999. { next.push(o(A::Buy, P::Stop(lstop), "Confluence LE")); }
        if lexit != 0. { next.push(o(A::Xl, P::Stop(lexit), "Confluence LX")); }
        if sstop != 0. { next.push(o(A::Sell, P::Stop(sstop), "Confluence SE")); }
        if sexit != 99999. { next.push(o(A::Xs, P::Stop(sexit), "Confluence SX")); }
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn confluence_range_and_trades() {
        let r = super::super::sctraub::tests::bars(1500);
        let x = Ctx::new(&r, &r, &[]);
        let ap: Vec<f64> = (0..x.c.len()).map(|i| (x.o[i] + x.h[i] + x.l[i] + x.c[i]) / 4.).collect();
        let c = confluence(&x.c, &ap, 4);
        assert!(c.iter().all(|v| v.abs() <= 9.) && c.iter().any(|&v| v >= 1.) && c.iter().any(|&v| v <= -1.));
        let t = run("tsconf_legan", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(t.len() > 5 && t.iter().any(|z| z.dir > 0.) && t.iter().any(|z| z.dir < 0.));
        assert!(run("tsconf_x", &x, 0, 5, &Params::new()).is_none());
    }
}

// Instrument admission: Confluence system 2026-11-02 (titles lane K18).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "tsconf" && reviewed(def.id),
    label: "TradeStation system posted to the Omega list for 30-minute SPX bars, with no printed results. Runs on daily indices, ETFs and stocks, US and India, are a declared application.",
    basis: "conflu.txt (Gary Fritz's Omega-list post with Dale Legan's Confluence and DerivativeMA functions, printed code); independent check authors/titles-1030/K18w/k18_ref.py",
};
