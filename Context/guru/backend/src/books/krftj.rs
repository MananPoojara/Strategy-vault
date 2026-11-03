//! Robert Krausz, Fibonacci Trader Journal Vol.1 Issue 3, "The Professional Gann Swing Plan" (rules p.3-4, trade examples p.5-7;
//! research share Trading Books/Robert Krausz - Fibonacci Traders Journals, No 1-14/FTJ03.pdf; text authors/titles-1030/K15w/src/
//! ftj03.txt). Book id `krftj`. It builds on the Basic plan of book module krgann (swing chart and HiLo Activator, same
//! definitions; krgann is not changed). Lane K15 (titles loop, 2 Nov 2026), to-code row 303, lead decision 107; independent check
//! authors/titles-1030/K15w/k15_ref.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::ctm::{ent, ext, orders, K};
use super::lc3::{hi, high_first, lo, week_no};

pub const REVIEWED: &[&str] = &["krftj_pro"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] Daily bars with the weekly ('Next') HiLo Activator. Swing chart as krgann_basic: the swing turns up on two consecutive higher highs and down on two consecutive lower lows; the peak (valley) is the highest high (lowest low) of the finished swing; the chart is solid (uptrend) once a bar trades 2 ticks above the nearest peak, dashed once 2 ticks below the nearest valley. Daily HiLo lines = 3-day averages of the highs / lows up to the day before; it flips up on a close 2 ticks above the high line, down on a close 2 ticks below the low line. Weekly HiLo the same on Monday-based weeks; the line shown on a day is the low line while the weekly activator is up, the high line while it is down (from the three weeks before the current one). One position, stop and reverse. TREND: solid chart and the daily HiLo flips up on a close (2 ticks) -> long at that close; a buy stop 2 ticks above the nearest peak (also the 'stop & reverse 2 ticks past nearest peak' rule). CONTRA TREND: dashed chart and a close above the high line -> a buy stop at that bar's high, kept until it trades or the chart turns solid (a later such close replaces it). SPECIAL: a close 3 ticks below the weekly line -> a buy stop 3 ticks above the weekly line until it trades. Shorts mirrored. A buy stop is placed only while the close is below it (a sell stop above it). PROFIT PROTECTION (exit to flat, whichever is hit first): a stop at the 38.2% retracement of the current swing (from the valley to the highest high since, for a long) and a stop 2 ticks through the daily HiLo low line (high line for a short). RE-ENTRY: the standing peak/valley and contra-trend stops (the journal's re-entry rules 2 and 3); rule 1 ('most recent isolated high or low') is not applied. Orders for the next day: a stop at its price or the open if the open is through it; one fill per day, the price nearer the open first. Ticks = the instrument's tick (0.01 US, 0.05 NSE; the journal's T-bond 2 and 3 ticks, lead decision 57; p.ticks). HELD / NOT APPLIED: the 10/20-minute opening rule (intraday), the three-contract targets (1/3 at the swing-slope change or the weekly HiLo) and the 1/3 pyramids (one position). Book: daily T-bonds Jan 1 - Oct 7 1998, 30 trades, 19 winners, +$26,437.50 with three contracts; T-bond futures are not held: daily indices, ETFs and US/India stocks are a declared application; the vault's standard costs.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "krftj_pro", name: "Professional Gann Swing Plan (FTJ03, daily/weekly): trend, contra-trend and weekly-HiLo entries, 38.2% / HiLo profit protection", group: "Fibonacci Trader Journal 3: Professional Gann Swing Plan (Krausz)", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "krftj", rules: &[
            "E|TREND TRADING: \"Qualifier To Buy 1. Gann Swing Chart should show a solid (green) line. Entry Rule (to Buy) 1. Go long on close above HiLo Activator by two ticks for the T-bonds. 2. Go long if prices surpass the previous peak.\" (FTJ03 p.3)",
            "E|CONTRA TREND: \"Qualifiers To Buy 1. Gann Swing Chart should be a Dashed line. 2. Close above the HiLo Activator. Entry Rule (to Buy) 1. Go long when prices pass the high of the bar that closed above the HiLo Activator. (This does not have to be the next day's bar.)\" (p.3)",
            "E|SPECIAL ENTRY: \"The daily bar closes 3 ticks below the Next HiLo Activator ... Go long on the next bar that reverses by a 3 tick penetration back above the Next HiLo Activator (the Weekly).\" (p.3)",
            "X|\"If long, stop & reverse 2 ticks past nearest valley, unless normal entry rule is hit\"; \"PROFIT PROTECTION RULES (use whichever is hit first) 1. Any 38.2% retracement of the previous swing. 2. Any 2 tick penetration of the Daily HiLo Activator.\" (p.4)"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    (id == "krftj_pro").then_some(("krftj_pro", "Krausz Gann swing plan", "Professional plan (FTJ03)"))
}
pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    reviewed(id).then(|| vec![ParamDef { key: "ticks", label: "Ticks beyond the HiLo / peak / valley (journal: 2 for T-bonds; the special entry uses this + 1)", def: 2., min: 0., max: 5., step: 1. }])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let n = x.r.len();
    let tk = p.get("ticks").copied().unwrap_or(2.).round().max(0.);
    let (off, off3) = (tk * x.market_tick(), (tk + 1.) * x.market_tick());
    // swing levels known at each close: (peak, valley, valley bar, peak bar), as krgann
    let (mut sw, mut pk, mut vl, mut pki, mut vli) = (0i8, f64::NAN, f64::NAN, 0usize, 0usize);
    let mut lv = Vec::with_capacity(n);
    for i in 0..n {
        if i >= 2 && h[i] > h[i - 1] && h[i - 1] > h[i - 2] && sw != 1 {
            if sw == -1 { let mut j = pki; for k in pki..i { if l[k] < l[j] { j = k; } } vl = l[j]; vli = j; }
            sw = 1;
        } else if i >= 2 && l[i] < l[i - 1] && l[i - 1] < l[i - 2] && sw != -1 {
            if sw == 1 { let mut j = vli; for k in vli..i { if h[k] > h[j] { j = k; } } pk = h[j]; pki = j; }
            sw = -1;
        }
        lv.push((pk, vl, vli, pki));
    }
    // daily HiLo lines for bar i (from the 3 bars before)
    let hb: Vec<f64> = (0..n).map(|i| if i >= 3 { (h[i - 3] + h[i - 2] + h[i - 1]) / 3. } else { f64::NAN }).collect();
    let hl: Vec<f64> = (0..n).map(|i| if i >= 3 { (l[i - 3] + l[i - 2] + l[i - 1]) / 3. } else { f64::NAN }).collect();
    // weekly line for each daily bar
    let mut wks: Vec<(usize, usize)> = vec![];
    let mut s0 = 0;
    for i in 0..n { if i == n - 1 || week_no(&x.r[i + 1].date) != week_no(&x.r[i].date) { wks.push((s0, i)); s0 = i + 1; } }
    let (wh, wl, wc): (Vec<f64>, Vec<f64>, Vec<f64>) = (wks.iter().map(|w| hi(h, w.0, w.1)).collect(), wks.iter().map(|w| lo(l, w.0, w.1)).collect(), wks.iter().map(|w| c[w.1]).collect());
    let mut wline = vec![f64::NAN; n];
    let mut ws = 0i8;
    for j in 0..wks.len() {
        // line shown during week j: from weeks j-3..j-1 and the state after week j-1
        if j >= 4 {
            let v = if ws == 1 { (wl[j - 3] + wl[j - 2] + wl[j - 1]) / 3. } else if ws == -1 { (wh[j - 3] + wh[j - 2] + wh[j - 1]) / 3. } else { f64::NAN };
            for i in wks[j].0..=wks[j].1 { wline[i] = v; }
        }
        if j >= 3 {
            let (b, s) = ((wh[j - 3] + wh[j - 2] + wh[j - 1]) / 3., (wl[j - 3] + wl[j - 2] + wl[j - 1]) / 3.);
            if ws != 1 && wc[j] > b { ws = 1; } else if ws != -1 && wc[j] < s { ws = -1; }
        }
    }
    // signal states from bar 3 (independent of the position): trend, HiLo state and its flip at each close, arms
    let (mut tr, mut hs) = (0i8, 0i8);
    let (mut cl, mut cs, mut sl, mut ss) = (f64::NAN, f64::NAN, false, false);
    let mut st: Vec<(i8, i8, f64, f64, bool, bool)> = vec![(0, 0, f64::NAN, f64::NAN, false, false); n]; // (tr, flip, cl, cs, sl, ss) after close i
    for i in 3..n {
        let (pk, vl, _, _) = lv[i - 1];
        let (cu, cd) = (pk.is_finite() && h[i] > pk + off, vl.is_finite() && l[i] < vl - off);
        match (cu, cd) { (true, true) => tr = if high_first(o[i], h[i], l[i]) { -1 } else { 1 }, (true, false) => tr = 1, (false, true) => tr = -1, _ => {} }
        if h[i] > cl { cl = f64::NAN; }
        if l[i] < cs { cs = f64::NAN; }
        if sl && h[i] > wline[i] + off3 { sl = false; }
        if ss && l[i] < wline[i] - off3 { ss = false; }
        let mut flip = 0i8;
        if hs != 1 && c[i] > hb[i] + off { hs = 1; flip = 1; } else if hs != -1 && c[i] < hl[i] - off { hs = -1; flip = -1; }
        if tr == 1 { cl = f64::NAN; } else if tr == -1 && c[i] > hb[i] { cl = h[i]; }
        if tr == -1 { cs = f64::NAN; } else if tr == 1 && c[i] < hl[i] { cs = l[i]; }
        if c[i] < wline[i] - off3 { sl = true; }
        if c[i] > wline[i] + off3 { ss = true; }
        st[i] = (tr, flip, cl, cs, sl, ss);
    }
    Some(orders(x, from, to, |t, pos| {
        if t < 3 { return vec![]; }
        let (tr, flip, cl, cs, sl, ss) = st[t];
        if flip != 0 && flip == tr && pos.side() != flip as f64 { pos.enter(flip as f64, t, c[t], "close through the HiLo"); }
        if t + 1 >= n { return vec![]; }
        let sd = pos.side();
        let (pk, vl, vli, pki) = lv[t];
        let mut z = vec![];
        if sd != 1. {
            for px in [pk + off, cl, if sl { wline[t + 1] + off3 } else { f64::NAN }] { if px.is_finite() && c[t] < px { z.push(ent(1., K::Stop, px)); } }
        }
        if sd != -1. {
            for px in [vl - off, cs, if ss { wline[t + 1] - off3 } else { f64::NAN }] { if px.is_finite() && c[t] > px { z.push(ent(-1., K::Stop, px)); } }
        }
        if sd > 0. {
            z.push(ext(-1., K::Stop, (l[t - 2] + l[t - 1] + l[t]) / 3. - off));
            if vl.is_finite() { let top = hi(h, vli, t); if top > vl { z.push(ext(-1., K::Stop, top - 0.382 * (top - vl))); } }
        } else if sd < 0. {
            z.push(ext(1., K::Stop, (h[t - 2] + h[t - 1] + h[t]) / 3. + off));
            if pk.is_finite() { let bot = lo(l, pki, t); if bot < pk { z.push(ext(1., K::Stop, bot + 0.382 * (pk - bot))); } }
        }
        z
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn professional_plan_trades_both_sides() {
        let r = super::super::ctm::tests::walk(3000);
        let x = Ctx::new(&r, &r, &[]);
        let t = run("krftj_pro", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(t.len() > 10 && t.iter().any(|z| z.dir > 0.) && t.iter().any(|z| z.dir < 0.));
        assert!(family_of("krftj_pro").is_some() && params("krftj_pro").is_some() && run("krftj_x", &x, 0, 5, &Params::new()).is_none());
    }
}

// Instrument admission: Krausz FTJ03 Professional Gann Swing Plan 2026-11-02 (titles lane K15).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "krftj" && reviewed(def.id),
    label: "Krausz traded the plan on T-bond futures; runs on daily indices, ETFs and stocks, US and India, are a declared application.",
    basis: "Fibonacci Trader Journal Vol.1 Issue 3 (FTJ03.pdf) pp.3-7, building on krgann_basic's definitions; independent check authors/titles-1030/K15w/k15_ref.py",
};
