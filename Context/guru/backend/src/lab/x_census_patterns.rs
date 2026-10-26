//! Census group "chart_patterns" (2026-10-26, research: sv-census-work/research_chart_patterns.md). Formulas: see each definition string.
//! One causal swing engine serves every multi-swing pattern: confirmed pivots (price.rs::pivots, TradingView tie rules) folded into an
//! alternating peak/valley list; a candidate is found when a swing is confirmed and fires on the first close beyond its line.
//! Bottom/bearish mirrors run the same code on negated bars (high' = −low, low' = −high, close' = −close) and flip the sign.
//! Skipped as UNCONFIRMED (nothing invented): Rounding Bottom (no published shape rule; without it the pattern is only "a close above
//! an old peak"); the Bump and Run lead-in/bump angle rules (degrees depend on chart scaling) and its target (book only); the cup
//! U-shape test; Bulkowski's "white space" fill tests (triangles, broadening); Adam/Eve labels; the Dead Cat Bounce bounce and
//! decline (statistics, not rules; only its event day is built); TradingView's built-in auto chart patterns and Bulkowski Patternz
//! (closed source); the Trendoscope two-line classifier (a separate detector, research 0.6) and the rectangle max/min level (no
//! source names it). Optional ADAPT filters with no source number (prior trend, time symmetry, measured-move close-above-B entry)
//! are not offered: their "off" default equals what is built.
use std::borrow::Cow;
use super::bars::Bars;
use super::catalogue::{int, len, n, p, real, IndicatorDef, Needs, Output, ParamSpec, Params, Placement};
use super::price::pivots;
use super::ta::{self, NA};

const CAT: &str = "Chart patterns";
const fn def(id: &'static str, name: &'static str, params: &'static [ParamSpec], definition: &'static str, f: fn(&Bars, &Params) -> Output) -> IndicatorDef {
    IndicatorDef { id, name, category: CAT, params, outputs: &["signal", "level", "target"], placement: Placement::Pane, needs: Needs::NONE, definition,
                   first: &[0, usize::MAX, usize::MAX], sparse: true, compute: Some(f), note: "" }
}
const INF: f64 = f64::INFINITY;

#[derive(Clone, Copy)]
struct Sw { p: usize, v: f64, hi: bool }
/// y(t) = y + s·(t − x)
#[derive(Clone, Copy)]
struct Ln { x: f64, y: f64, s: f64 }
impl Ln { fn at(&self, t: usize) -> f64 { self.y + self.s * (t as f64 - self.x) } }
fn ln(a: Sw, b: Sw) -> Ln { Ln { x: a.p as f64, y: a.v, s: (b.v - a.v) / (b.p as f64 - a.p as f64) } }
fn flat(y: f64) -> Ln { Ln { x: 0., y, s: 0. } }
/// Target height: fixed, the channel width at the event bar, or none.
#[derive(Clone, Copy)]
enum Tg { H(f64), Width, No }
/// A pending pattern: close > up(t) or close < dn(t) ends it (an event only when `emit` allows that side and t ≥ from); high > cap
/// or low < floor drops it; `now` ≠ 0 fires on the detection bar (no breakout in the source). `pierce` tests high/low instead of
/// close; a reported break must clear the line by `pen` × |line| on `hold` consecutive bars (`cnt` counts them).
struct Cand { piv: Vec<usize>, up: Ln, dn: Ln, emit: (bool, bool), tg: Tg, from: usize, until: usize, cap: f64, floor: f64, now: i8,
              pierce: bool, pen: f64, hold: usize, cnt: usize }
fn cand(e: &[Sw], up: Ln, dn: Ln, emit: (bool, bool), tg: Tg, until: usize) -> Cand {
    Cand { piv: e.iter().map(|w| w.p).collect(), up, dn, emit, tg, from: 0, until, cap: INF, floor: -INF, now: 0, pierce: false, pen: 0., hold: 1, cnt: 0 }
}
fn fire(out: &mut [Vec<f64>], t: usize, sg: i8, c: &Cand) {
    if out[0][t] != 0. { return; } // one event per bar: the first candidate wins
    let (u, d) = (c.up.at(t), c.dn.at(t));
    let lv = if sg > 0 { u } else { d };
    let h = match c.tg { Tg::H(h) => h, Tg::Width => u - d, Tg::No => NA };
    (out[0][t], out[1][t], out[2][t]) = (sg as f64, lv, lv + sg as f64 * h);
}
/// The swing engine: `det(t, swings, new)` runs on each confirmed swing; then every pending candidate is tested on bar t.
fn scan(b: &Bars, prm: &Params, mut det: impl FnMut(usize, &[Sw], &mut Vec<Cand>)) -> Vec<Vec<f64>> {
    let (m, r) = (b.len(), n(prm, "right"));
    let (ph, pl) = (pivots(&b.high, n(prm, "left"), r, true), pivots(&b.low, n(prm, "left"), r, false));
    let mut out = vec![vec![0.; m], vec![NA; m], vec![NA; m]];
    let (mut s, mut pend, mut new): (Vec<Sw>, Vec<Cand>, Vec<Cand>) = (vec![], vec![], vec![]);
    for t in 0..m {
        for (hi, v) in [(true, ph[t]), (false, pl[t])] { // a bar that is both: peak first
            if v.is_nan() { continue; }
            let w = Sw { p: t - r, v, hi };
            match s.last_mut() {
                Some(z) if z.hi == hi => { if (hi && v > z.v) || (!hi && v < z.v) { *z = w; } else { continue; } }
                _ => s.push(w),
            }
            det(t, &s, &mut new);
        }
        for c in new.drain(..) {
            if c.now != 0 { fire(&mut out, t, c.now, &c); } else if !pend.iter().any(|q| q.piv == c.piv) { pend.push(c); }
        }
        let (mut hit, cl) = (vec![], b.close[t]);
        pend.retain_mut(|c| {
            if t > c.until { return false; }
            let ((xu, xd), (u, d)) = (if c.pierce { (b.high[t], b.low[t]) } else { (cl, cl) }, (c.up.at(t), c.dn.at(t)));
            let sg = if xu > u { 1 } else if xd < d { -1 } else { 0 };
            if sg != 0 {
                if t < c.from || !((sg > 0 && c.emit.0) || (sg < 0 && c.emit.1)) { return false; }
                let deep = if sg > 0 { xu - u > c.pen * u.abs() } else { d - xd > c.pen * d.abs() };
                c.cnt = if deep { c.cnt + 1 } else { 0 };
                if c.cnt >= c.hold { fire(&mut out, t, sg, c); hit.extend_from_slice(&c.piv); return false; }
            } else { c.cnt = 0; }
            !(b.high[t] > c.cap || b.low[t] < c.floor)
        });
        if !hit.is_empty() { pend.retain(|c| !c.piv.iter().any(|q| hit.contains(q))); }
    }
    out
}
fn neg(b: &Bars) -> Bars {
    let f = |x: &[f64]| x.iter().map(|v| -v).collect::<Vec<_>>();
    Bars { open: f(&b.open), high: f(&b.low), low: f(&b.high), close: f(&b.close), ..b.clone() }
}
fn flip(o: Vec<Vec<f64>>) -> Vec<Vec<f64>> { o.into_iter().map(|s| s.into_iter().map(|v| if v == 0. { 0. } else { -v }).collect()).collect() }
fn mirror(b: &Bars, f: impl Fn(&Bars) -> Vec<Vec<f64>>) -> Output { Ok(flip(f(&neg(b)))) }
/// Both directions: the pattern and its mirror; on a bar where both fire, the unmirrored one wins.
fn both(b: &Bars, f: impl Fn(&Bars) -> Vec<Vec<f64>>) -> Output {
    let (mut a, z) = (f(b), flip(f(&neg(b))));
    for t in 0..b.len() { if a[0][t] == 0. && z[0][t] != 0. { for k in 0..3 { a[k][t] = z[k][t]; } } }
    Ok(a)
}

/// Every value within tol × |mean| of the mean (Lo-Mamaysky-Wang "within x% of their average").
fn near(v: &[f64], tol: f64) -> bool { let m = v.iter().sum::<f64>() / v.len() as f64; m != 0. && v.iter().all(|x| (x - m).abs() <= tol * m.abs()) }
fn mean(v: &[Sw]) -> f64 { v.iter().map(|w| w.v).sum::<f64>() / v.len() as f64 }
fn vals(v: &[Sw]) -> Vec<f64> { v.iter().map(|w| w.v).collect() }
/// The last k swings, if every peak is strictly above its neighbouring valleys.
fn tail(s: &[Sw], k: usize) -> Option<&[Sw]> {
    let e = &s[s.len().checked_sub(k)?..];
    e.windows(2).all(|w| if w[0].hi { w[0].v > w[1].v } else { w[0].v < w[1].v }).then_some(e)
}
fn sides(e: &[Sw]) -> (Vec<Sw>, Vec<Sw>) { e.iter().copied().partition(|w| w.hi) }
fn rising(v: &[Sw]) -> bool { v.windows(2).all(|w| w[1].v > w[0].v) }
fn falling(v: &[Sw]) -> bool { v.windows(2).all(|w| w[1].v < w[0].v) }
/// Line through the first and last point; a middle point must lie within tol × |line| of it (ADAPT touch rule).
fn edge(v: &[Sw], tol: f64) -> Option<Ln> {
    let l = ln(v[0], v[v.len() - 1]);
    v[1..v.len() - 1].iter().all(|w| (w.v - l.at(w.p)).abs() <= tol * l.at(w.p).abs()).then_some(l)
}
/// Where the two lines meet, and the last bar before it.
fn meet(u: Ln, d: Ln) -> f64 { (d.y - u.y + u.s * u.x - d.s * d.x) / (u.s - d.s) }
fn apex(u: Ln, d: Ln) -> usize { let x = meet(u, d); if x >= 1. { (x.ceil() - 1.) as usize } else { 0 } }
fn span(e: &[Sw]) -> usize { e[e.len() - 1].p - e[0].p }
/// Expiry bars after detection: max_wait, or the pattern's span when 0 (ADAPT, research 0.4).
fn wait(prm: &Params, sp: usize) -> usize { match n(prm, "max_wait") { 0 => sp, w => w } }
/// src 1 (Lo-Mamaysky-Wang): swings and every high/low test on the close.
fn src<'a>(b: &'a Bars, prm: &Params) -> Cow<'a, Bars> {
    if prm.get("src") == Some(&1.) { Cow::Owned(Bars { high: b.close.clone(), low: b.close.clone(), ..b.clone() }) } else { Cow::Borrowed(b) }
}

const fn lr(k: &'static str, l: &'static str) -> ParamSpec { len(k, l, 2.) }
const L: ParamSpec = lr("left", "Pivot left bars (Bulkowski minor high: 2)");
const R: ParamSpec = lr("right", "Pivot right bars (Bulkowski minor high: 2)");
const TOUCH: ParamSpec = real("touch_tol", "Middle pivot within this fraction of its trendline (ADAPT)", 0.01, 0., 1., 0.001);
const MW: ParamSpec = int("max_wait", "Bars after detection before a pending pattern expires (ADAPT; 0 = the pattern's span)", 0., 0., 5000.);
const SRC: ParamSpec = int("src", "Swing prices (0 Bulkowski / StockCharts high and low, 1 Lo-Mamaysky-Wang close)", 0., 0., 1.);

const HS: &[ParamSpec] = &[L, R, real("sh_tol", "Shoulders within this fraction of their average (Lo-Mamaysky-Wang 0.015)", 0.015, 0., 1., 0.001),
    real("neck_tol", "Armpits within this fraction of their average (Lo-Mamaysky-Wang 0.015; 1 = off, Bulkowski)", 0.015, 0., 1., 0.001),
    int("neck_mode", "Neckline (0 Bulkowski, 1 StockCharts, 2 Lo-Mamaysky-Wang no breakout)", 0., 0., 2.), MW, SRC];
fn hs(b: &Bars, prm: &Params) -> Vec<Vec<f64>> {
    let (st, nt, mode) = (p(prm, "sh_tol"), p(prm, "neck_tol"), n(prm, "neck_mode"));
    scan(b, prm, |t, s, out| {
        let Some(e) = tail(s, 5) else { return };
        let (ls, t1, hd, t2, rs) = (e[0], e[1], e[2], e[3], e[4]);
        if !ls.hi || !(hd.v > ls.v && hd.v > rs.v) || !near(&[ls.v, rs.v], st) || !near(&[t1.v, t2.v], nt) { return; }
        let nl = ln(t1, t2);
        let dn = if mode == 0 && t2.v < t1.v { flat(t2.v) } else { nl };
        out.push(Cand { now: if mode == 2 { -1 } else { 0 }, ..cand(e, flat(hd.v), dn, (false, true), Tg::H(hd.v - nl.at(hd.p)), t + wait(prm, span(e))) });
    })
}
fn hs_top(b: &Bars, prm: &Params) -> Output { Ok(hs(&src(b, prm), prm)) }
fn hs_bottom(b: &Bars, prm: &Params) -> Output { mirror(&src(b, prm), |x| hs(x, prm)) }

const fn dbl(rule: &'static str) -> [ParamSpec; 11] {
    [L, R, real("tol", "Equal-extreme tolerance (Bulkowski / StockCharts 0.03)", 0.03, 0., 0.5, 0.001), int("rule", rule, 0., 0., 1.),
     real("depth_min", "Min move between the extremes, fraction of the outer extreme (Bulkowski 0.10)", 0.10, 0., 1., 0.01),
     int("min_sep", "Bars between the extremes must exceed (Lo-Mamaysky-Wang 22)", 22., 0., 2000.), int("max_sep", "Max bars between the extremes (0 = off)", 0., 0., 2000.),
     real("pen_pct", "Break must clear the line by this fraction of it (StockCharts price filter 0.03; 0 = off, Bulkowski)", 0., 0., 1., 0.001),
     int("hold_bars", "Consecutive closes beyond the line (StockCharts time filter 3; 0 = off, Bulkowski)", 0., 0., 100.), MW, SRC]
}
const DT: &[ParamSpec] = &dbl("Equal-top test (0 Bulkowski |P1−P2|/max(P1,P2) ≤ tol, 1 Lo-Mamaysky-Wang within tol of the average)");
const DB: &[ParamSpec] = &dbl("Equal-bottom test (0 StockCharts |B1−B2|/B1 ≤ tol, 1 Lo-Mamaysky-Wang within tol of the average)");
fn double(b: &Bars, prm: &Params, bottom: bool) -> Vec<Vec<f64>> {
    let (tol, lmw, dep, mn, mx) = (p(prm, "tol"), n(prm, "rule") == 1, p(prm, "depth_min"), n(prm, "min_sep"), n(prm, "max_sep"));
    let (pen, hold) = (p(prm, "pen_pct"), n(prm, "hold_bars").max(1));
    let (h, l) = (&b.high, &b.low);
    scan(b, prm, |t, s, out| {
        let Some(&p2) = s.last() else { return };
        if !p2.hi { return; }
        // no acceptable P1 is above this, so a higher bar between ends the search
        let stop = p2.v + p2.v.abs() * 2. * tol / (1. - tol);
        let (mut hh, mut ll, mut q) = (-INF, INF, p2.p);
        for &p1 in s.iter().rev().skip(2).step_by(2) {
            for u in p1.p + 1..q { hh = hh.max(h[u]); ll = ll.min(l[u]); }
            q = p1.p + 1;
            if hh > stop || (mx > 0 && p2.p - p1.p > mx) { break; }
            let top = p1.v.max(p2.v);
            let eq = if lmw { near(&[p1.v, p2.v], tol) } else { (p1.v - p2.v).abs() <= tol * if bottom { p1.v.abs() } else { p1.v.abs().max(p2.v.abs()) } };
            if p2.p - p1.p > mn && eq && hh <= top && top - ll >= dep * top.abs() {
                out.push(Cand { pen, hold, ..cand(&[p1, p2], flat(top), flat(ll), (false, true), Tg::H(top - ll), t + wait(prm, p2.p - p1.p)) });
                break;
            }
        }
    })
}
fn double_top(b: &Bars, prm: &Params) -> Output { Ok(double(&src(b, prm), prm, false)) }
fn double_bottom(b: &Bars, prm: &Params) -> Output { mirror(&src(b, prm), |x| double(x, prm, true)) }

const TRP: &[ParamSpec] = &[L, R, real("top_tol", "Three extremes within this fraction of their average (ADAPT 0.03)", 0.03, 0., 1., 0.001), MW];
fn triple(b: &Bars, prm: &Params) -> Vec<Vec<f64>> {
    let tol = p(prm, "top_tol");
    scan(b, prm, |t, s, out| {
        let Some(e) = tail(s, 5) else { return };
        let side = e[0].v.max(e[4].v);
        // a middle peak this far above the shoulders is a head and shoulders
        if !e[0].hi || !near(&[e[0].v, e[2].v, e[4].v], tol) || e[2].v - side > tol * side.abs() { return; }
        let (top, low) = (side.max(e[2].v), e[1].v.min(e[3].v));
        out.push(cand(e, flat(top), flat(low), (false, true), Tg::H(top - low), t + wait(prm, span(e))));
    })
}
fn triple_top(b: &Bars, prm: &Params) -> Output { Ok(triple(b, prm)) }
fn triple_bottom(b: &Bars, prm: &Params) -> Output { mirror(b, |x| triple(x, prm)) }

const fn cups(lo: &'static str, hi: &'static str, handle: ParamSpec) -> [ParamSpec; 6] {
    [L, R, real("rim_tol", "Rims: |A − C|/max(A, C) ≤ this (ADAPT 0.05)", 0.05, 0., 0.5, 0.001), int("cup_min", lo, 35., 1., 2000.), int("cup_max", hi, 325., 1., 5000.), handle]
}
const CUP: &[ParamSpec] = &{
    let c = cups("Min cup bars A..C (Bulkowski 7 weeks = 35)", "Max cup bars A..C (Bulkowski 65 weeks = 325)", real("handle_max", "Max handle retrace, fraction of the cup height (Bulkowski 0.5 upper half; StockCharts 1/3)", 0.5, 0., 1., 0.001));
    [c[0], c[1], c[2], c[3], c[4], c[5], real("depth_max", "Max cup depth / prior advance (StockCharts 1/3; 1 = off)", 1. / 3., 0., 1., 0.001),
     int("handle_min", "Min bars from the right rim to the breakout (Bulkowski 1 week = 5)", 5., 0., 2000.)]
};
const ICUP: &[ParamSpec] = &cups("Min cup bars A..C (ADAPT 35: the cup's; Bulkowski gives none)", "Max cup bars A..C (ADAPT 325: the cup's; Bulkowski gives none)", real("handle_max", "Max handle retrace, fraction of the cup height (Bulkowski 0.6)", 0.6, 0., 1., 0.001));
/// Cup with handle; the inverted cup is the same scan on negated bars without the depth, handle-time and rim-cap rules.
fn cup(b: &Bars, prm: &Params, inv: bool) -> Vec<Vec<f64>> {
    let (rim, cmin, cmax, hmax) = (p(prm, "rim_tol"), n(prm, "cup_min"), n(prm, "cup_max"), p(prm, "handle_max"));
    let (dmax, hmin) = if inv { (1., 0) } else { (p(prm, "depth_max"), n(prm, "handle_min")) };
    let (h, l) = (&b.high, &b.low);
    scan(b, prm, |t, s, out| {
        let Some(&c) = s.last() else { return };
        if !c.hi { return; }
        let stop = c.v + c.v.abs() * rim / (1. - rim);
        let (mut hh, mut ll, mut q) = (-INF, INF, c.p);
        for k in (0..s.len().saturating_sub(2)).rev().step_by(2) {
            let a = s[k];
            for u in a.p + 1..q { hh = hh.max(h[u]); ll = ll.min(l[u]); }
            q = a.p + 1;
            let (w, top) = (c.p - a.p, a.v.max(c.v));
            if hh > stop || w > cmax { break; }
            if w < cmin || (a.v - c.v).abs() > rim * a.v.abs().max(c.v.abs()) || hh > top { continue; }
            // StockCharts depth: (A − B)/(A − Z), Z = the valley before A
            if dmax < 1. && !(k > 0 && a.v > s[k - 1].v && a.v - ll <= dmax * (a.v - s[k - 1].v)) { continue; }
            let (cap, floor) = (if inv { INF } else { c.v }, c.v - hmax * (c.v - ll));
            // the handle rules also cover the bars between C and its confirmation bar t
            if (c.p + 1..t).all(|u| h[u] <= cap && l[u] >= floor) {
                out.push(Cand { from: c.p + hmin, cap, floor, ..cand(&[a, c], flat(c.v), flat(-INF), (true, false), Tg::H(c.v - ll), usize::MAX) });
            }
            break;
        }
    })
}
fn cup_handle(b: &Bars, prm: &Params) -> Output { Ok(cup(b, prm, false)) }
fn inv_cup_handle(b: &Bars, prm: &Params) -> Output { mirror(b, |x| cup(x, prm, true)) }

const TRI: &[ParamSpec] = &[L, R, TOUCH, int("timing", "Breakout window (0 Bulkowski any bar before the apex, 1 StockCharts 1/2 to 3/4 of the way from the first pivot to the apex)", 0., 0., 1.), SRC];
fn tri_sym(b: &Bars, prm: &Params) -> Output {
    let (tt, sc, b) = (p(prm, "touch_tol"), n(prm, "timing") == 1, src(b, prm));
    Ok(scan(&b, prm, |_, s, out| {
        let Some(e) = tail(s, 5) else { return };
        let (pk, vl) = sides(e);
        if !falling(&pk) || !rising(&vl) { return; }
        let (Some(u), Some(d)) = (edge(&pk, tt), edge(&vl, tt)) else { return };
        let mut c = cand(e, u, d, (true, true), Tg::H(u.at(e[0].p) - d.at(e[0].p)), apex(u, d));
        if sc { let (x0, w) = (e[0].p as f64, meet(u, d) - e[0].p as f64); (c.from, c.until) = ((x0 + 0.5 * w).ceil() as usize, (x0 + 0.75 * w).floor() as usize); }
        out.push(c);
    }))
}
const ASC: &[ParamSpec] = &[L, R, real("flat_tol", "Flat side: every pivot within this fraction of their average (ADAPT 0.01)", 0.01, 0., 1., 0.001), TOUCH];
fn asc(b: &Bars, prm: &Params) -> Vec<Vec<f64>> {
    let (ft, tt) = (p(prm, "flat_tol"), p(prm, "touch_tol"));
    scan(b, prm, |_, s, out| {
        let Some(e) = tail(s, 5) else { return };
        let (pk, vl) = sides(e);
        if !near(&vals(&pk), ft) || !rising(&vl) { return; }
        let (Some(d), r) = (edge(&vl, tt), mean(&pk)) else { return };
        out.push(cand(e, flat(r), d, (true, true), Tg::H(r - vl[0].v), apex(flat(r), d)));
    })
}
fn tri_asc(b: &Bars, prm: &Params) -> Output { Ok(asc(b, prm)) }
fn tri_desc(b: &Bars, prm: &Params) -> Output { mirror(b, |x| asc(x, prm)) }

const WDG: &[ParamSpec] = &[L, R, TOUCH, int("min_span", "Min bars first..last pivot (Bulkowski 3 weeks = 15)", 15., 0., 2000.),
    int("dir", "Breakout (0 Bulkowski either way, 1 StockCharts rising down only / falling up only)", 0., 0., 1.)];
fn wedge(b: &Bars, prm: &Params) -> Vec<Vec<f64>> {
    let (tt, ms, either) = (p(prm, "touch_tol"), n(prm, "min_span"), n(prm, "dir") == 0);
    scan(b, prm, |_, s, out| {
        let Some(e) = tail(s, 5) else { return };
        let (pk, vl) = sides(e);
        if !rising(&pk) || !rising(&vl) || span(e) < ms { return; }
        let (Some(u), Some(d)) = (edge(&pk, tt), edge(&vl, tt)) else { return };
        if !(u.s > 0. && d.s > u.s) { return; }
        out.push(cand(e, u, d, (either, true), Tg::H(u.at(e[0].p) - d.at(e[0].p)), apex(u, d)));
    })
}
fn wedge_rising(b: &Bars, prm: &Params) -> Output { Ok(wedge(b, prm)) }
fn wedge_falling(b: &Bars, prm: &Params) -> Output { mirror(b, |x| wedge(x, prm)) }

const fn poles(tail_: [ParamSpec; 2]) -> [ParamSpec; 7] {
    [L, R, int("pole_max", "Max pole bars (ADAPT 15)", 15., 1., 2000.), real("pole_atr", "Min pole height in ATR(14) at its end (ADAPT 3; 0 = off)", 3., 0., 100., 0.1),
     int("flag_max", "Max bars from the pole end to the last pivot (Bulkowski 3 weeks = 15; StockCharts up to 60)", 15., 1., 2000.), tail_[0], tail_[1]]
}
const DIR: ParamSpec = int("dir", "Breakout (0 Bulkowski either way, 1 StockCharts with the pole only)", 1., 0., 1.);
const FLAG: &[ParamSpec] = &{ let c = poles([real("par_tol", "Parallel: |s_up − s_lo| ≤ this × max(|s_up|, |s_lo|) (ADAPT 0.3)", 0.3, 0., 10., 0.01), DIR]);
    [c[0], c[1], c[2], c[3], c[4], c[5], c[6], MW] };
const PEN: &[ParamSpec] = &{ let c = poles([DIR, DIR]); [c[0], c[1], c[2], c[3], c[4], c[5]] };
/// Bull flag/pennant: pole valley A → peak B, then valley, peak, valley; the bear one is the mirror.
fn pole(b: &Bars, prm: &Params, pennant: bool) -> Vec<Vec<f64>> {
    let (pm, k, fm, pt, pole_only) = (n(prm, "pole_max"), p(prm, "pole_atr"), n(prm, "flag_max"), prm.get("par_tol").copied().unwrap_or(0.), n(prm, "dir") == 1);
    let atr = ta::rma(&ta::tr(b, true), 14);
    scan(b, prm, |t, s, out| {
        let Some(e) = tail(s, 5) else { return };
        let (a, bb, v1, p2, v2) = (e[0], e[1], e[2], e[3], e[4]);
        if a.hi || bb.p - a.p > pm || (k > 0. && !(bb.v - a.v >= k * atr[bb.p])) || v2.p - bb.p > fm { return; }
        let (u, d) = (ln(bb, p2), ln(v1, v2));
        let until = if pennant {
            if !(p2.v < bb.v && v2.v > v1.v) { return; }
            apex(u, d)
        } else {
            if (u.s - d.s).abs() > pt * u.s.abs().max(d.s.abs()) { return; }
            t + wait(prm, span(e))
        };
        out.push(cand(e, u, d, (true, !pole_only), Tg::H(bb.v - a.v), until));
    })
}
fn flag(b: &Bars, prm: &Params) -> Output { both(b, |x| pole(x, prm, false)) }
fn pennant(b: &Bars, prm: &Params) -> Output { both(b, |x| pole(x, prm, true)) }

const RECT: &[ParamSpec] = &[L, R, real("tol", "Tops and bottoms each within this fraction of their average (Lo-Mamaysky-Wang 0.0075)", 0.0075, 0., 1., 0.0001),
    int("min_span", "Min bars first..last pivot (StockCharts 3 weeks = 15)", 15., 0., 2000.), MW, SRC];
fn rectangle(b: &Bars, prm: &Params) -> Output {
    let (tol, ms, b) = (p(prm, "tol"), n(prm, "min_span"), src(b, prm));
    Ok(scan(&b, prm, |t, s, out| {
        let Some(e) = tail(s, 5) else { return };
        let (pk, vl) = sides(e);
        let (lo_top, hi_bot) = (pk.iter().map(|w| w.v).fold(INF, f64::min), vl.iter().map(|w| w.v).fold(-INF, f64::max));
        if !near(&vals(&pk), tol) || !near(&vals(&vl), tol) || lo_top <= hi_bot || span(e) < ms { return; }
        let (r, sp) = (mean(&pk), mean(&vl));
        out.push(cand(e, flat(r), flat(sp), (true, true), Tg::H(r - sp), t + wait(prm, span(e))));
    }))
}

const CHAN: &[ParamSpec] = &[L, R, real("par_tol", "Parallel: |s_p − s_v| ≤ this × max(|s_p|, |s_v|) (ADAPT 0.3)", 0.3, 0., 10., 0.01),
    real("flat_pct", "Not flat: |s_v| × span / V1 must exceed this (ADAPT 0.02)", 0.02, 0., 10., 0.001),
    int("points", "Lines (0 Bulkowski / StockCharts 4-point: two peaks, two valleys; 1 Bulkowski 3-point: main line through two lows or highs, parallel line through the swing between)", 0., 0., 1.), MW];
fn channel(b: &Bars, prm: &Params) -> Output {
    let (pt, fp, three) = (p(prm, "par_tol"), p(prm, "flat_pct"), n(prm, "points") == 1);
    Ok(scan(b, prm, |t, s, out| {
        if three {
            let Some(e) = tail(s, 3) else { return };
            let (m, par) = (ln(e[0], e[2]), Ln { x: e[1].p as f64, y: e[1].v, ..ln(e[0], e[2]) });
            // bull: rising lows carry the main line; bear: falling highs
            let (u, d) = if !e[0].hi && m.s > 0. { (par, m) } else if e[0].hi && m.s < 0. { (m, par) } else { return };
            if m.s.abs() * span(e) as f64 <= fp * e[0].v.abs() { return; }
            return out.push(cand(e, u, d, (true, true), Tg::No, t + wait(prm, span(e))));
        }
        let Some(e) = tail(s, 4) else { return };
        let (pk, vl) = sides(e);
        let (u, d) = (ln(pk[0], pk[1]), ln(vl[0], vl[1]));
        if u.s * d.s <= 0. || (u.s - d.s).abs() > pt * u.s.abs().max(d.s.abs()) || d.s.abs() * span(e) as f64 <= fp * vl[0].v.abs() { return; }
        out.push(cand(e, u, d, (true, true), Tg::No, t + wait(prm, span(e))));
    }))
}

const BRD: &[ParamSpec] = &[L, R, TOUCH, int("pierce", "Breakout (0 close beyond the line: Bulkowski confirmation, StockCharts; 1 Bulkowski \"pierces\": high above / low below)", 0., 0., 1.), MW, SRC];
fn broadening(b: &Bars, prm: &Params) -> Output {
    let (tt, pierce, b) = (p(prm, "touch_tol"), n(prm, "pierce") == 1, src(b, prm));
    Ok(scan(&b, prm, |t, s, out| {
        let Some(e) = tail(s, 5) else { return };
        let (pk, vl) = sides(e);
        if !rising(&pk) || !falling(&vl) { return; }
        let (Some(u), Some(d)) = (edge(&pk, tt), edge(&vl, tt)) else { return };
        out.push(Cand { pierce, ..cand(e, u, d, (true, true), Tg::Width, t + wait(prm, span(e))) });
    }))
}

const BARR: &[ParamSpec] = &[L, R, int("lead_min", "Min lead-in bars V1..V2 (Bulkowski a month = 22)", 22., 1., 2000.),
    real("bump_ratio", "Bump height / lead-in height at least (Bulkowski, StockCharts 2)", 2., 0., 100., 0.1), MW];
/// Bump-and-run top: lead-in line through rising valleys V1, V2; bump peak P ≥ ratio × lead-in height above it; the bottom is the mirror.
fn barr(b: &Bars, prm: &Params) -> Vec<Vec<f64>> {
    let (lm, ratio, h) = (n(prm, "lead_min"), p(prm, "bump_ratio"), &b.high);
    scan(b, prm, |t, s, out| {
        let Some(&pk) = s.last() else { return };
        if !pk.hi || s.len() < 4 { return; }
        let v2 = s[s.len() - 2];
        if h[v2.p..=t].iter().any(|&x| x > pk.v) { return; }
        let Some(&v1) = s[..s.len() - 2].iter().rev().skip(1).step_by(2).find(|v1| v2.p - v1.p >= lm && v1.v < v2.v) else { return };
        let line = ln(v1, v2);
        // ADAPT lead-in height: the causal stand-in for Bulkowski's "first quarter of the pattern"
        let h1 = (v1.p..=v2.p).map(|u| h[u] - line.at(u)).fold(-INF, f64::max);
        if pk.v - line.at(pk.p) >= ratio * h1 { out.push(cand(&[v1, v2, pk], flat(INF), line, (false, true), Tg::No, t + wait(prm, pk.p - v1.p))); }
    })
}
fn bump_run(b: &Bars, prm: &Params) -> Output { both(b, |x| barr(x, prm)) }

const MM: &[ParamSpec] = &[L, R, real("r_min", "Min retrace (B − C)/(B − A) (StockCharts 0.33; Bulkowski automated 0.70)", 0.33, 0., 1., 0.01),
    real("r_max", "Max retrace (StockCharts 0.67; Bulkowski automated < 1)", 0.67, 0., 1., 0.01)];
fn mm(b: &Bars, prm: &Params) -> Vec<Vec<f64>> {
    let (lo, hi, mut done) = (p(prm, "r_min"), p(prm, "r_max"), (usize::MAX, usize::MAX));
    scan(b, prm, |t, s, out| {
        let Some(e) = tail(s, 3) else { return };
        let (a, bb, c) = (e[0], e[1], e[2]);
        let r = (bb.v - c.v) / (bb.v - a.v);
        // a later, more extreme C is the same A–B leg: it fired already
        if a.hi || c.v <= a.v || !(r >= lo && r <= hi) || done == (a.p, bb.p) { return; }
        done = (a.p, bb.p);
        out.push(Cand { now: 1, ..cand(e, flat(c.v), flat(-INF), (true, false), Tg::H(bb.v - a.v), t) });
    })
}
fn measured_move(b: &Bars, prm: &Params) -> Output { both(b, |x| mm(x, prm)) }

const DCB: &[ParamSpec] = &[real("drop_min", "Min one-day close drop (Bulkowski 0.15)", 0.15, 0., 1., 0.01), int("gap", "Gap required: high < prior low (0 no, 1 yes)", 0., 0., 1.)];
fn dead_cat(b: &Bars, prm: &Params) -> Output {
    let (k, gap, (h, l, c)) = (p(prm, "drop_min"), n(prm, "gap") == 1, (&b.high, &b.low, &b.close));
    Ok(vec![(0..b.len()).map(|i| if i == 0 { NA } else if c[i] <= (1. - k) * c[i - 1] && (!gap || h[i] < l[i - 1]) { -1. } else { 0. }).collect()])
}

macro_rules! sw { () => { "Swings: ta.pivothigh(high)/ta.pivotlow(low) with left/right bars (default 2/2, Bulkowski minor high \"2 days before to 2 days after\"), each known right bars after it, folded into an alternating peak/valley list (a pivot of the same kind as the last swing replaces it only when strictly more extreme: ADAPT, as Lo-Mamaysky-Wang extrema alternate); the swings used must each have every peak strictly above its neighbouring valleys. A bar that is both a peak and a valley adds the peak first (so ids run on negated bars add the valley first). The pattern is tested on the newest swings each time a swing is confirmed; a candidate with the same pivots as a pending one is not added. Breakout: the first bar t from that bar on whose close is strictly beyond the line (Bulkowski: a close outside the trendline); on one bar the breakout close is tested before the high/low drop rules. A candidate fires once; a pending candidate sharing a pivot with one that fired is dropped (ADAPT, research 0.4); one event per bar: a second candidate breaking on a bar that already has an event is dropped and its pivots count as used. Expiry \"after N bars\" means N bars after the detection bar, N = max_wait when > 0 (ADAPT, research 0.4). Outputs: signal ±1 on the event bar, else 0; level = the line value and target = level ± height on the event bar, else NaN. " } }

pub const DEFS: &[IndicatorDef] = &[
    def("chart_hs_top", "Head and Shoulders (Top)", HS, concat!(sw!(), "Head and Shoulders top (Bulkowski hst; StockCharts Head and Shoulders Top; Lo-Mamaysky-Wang 2000 Def. 1): last 5 swings LS, T1, Head, T2, RS = peak, valley, peak, valley, peak; Head > LS and Head > RS; LS, RS within sh_tol (0.015) of their average; T1, T2 within neck_tol (0.015) of theirs. Neckline T1–T2. Breakout −1: neck_mode 0 (Bulkowski) close below the neckline when T2 ≥ T1, else below the horizontal T2 (\"right armpit\"); 1 (StockCharts) below the T1–T2 line; 2 (Lo-Mamaysky-Wang, no breakout) −1 on the bar RS is confirmed, level = neckline there. Dropped on a close above Head or after max_wait (0 = RS − LS) bars. src 1 (LMW): swings on the close. Target = level − (Head − neckline at the Head bar)"), hs_top),
    def("chart_hs_bottom", "Inverse Head and Shoulders", HS, concat!(sw!(), "Inverse Head and Shoulders (Bulkowski hsb; StockCharts Head and Shoulders Bottom; Lo-Mamaysky-Wang Def. 1 IHS): the mirror of chart_hs_top: valley, peak, valley, peak, valley; Head < both shoulders; same tolerances. Breakout +1: neck_mode 0 (Bulkowski) close above the neckline when T2 ≤ T1, else above the horizontal T2; 1 (StockCharts) above the line; 2 (Lo-Mamaysky-Wang) +1 when the right shoulder is confirmed. Dropped on a close below Head or after max_wait (0 = RS − LS) bars. src 1 (LMW): swings on the close. Target = level + (neckline at the Head bar − Head)"), hs_bottom),
    def("chart_double_top", "Double Top", DT, concat!(sw!(), "Double Top (Bulkowski aadt/aedt; StockCharts Double Top Reversal; Lo-Mamaysky-Wang Def. 5): when a peak P2 is confirmed, the nearest earlier peak P1 in the swing list with P2.bar − P1.bar > min_sep (22, LMW), ≤ max_sep (0 = off), equal tops (rule 0 Bulkowski |P1 − P2|/max(P1, P2) ≤ tol 0.03; rule 1 LMW both within tol of their average), no high strictly between above max(P1, P2), and (max − V)/max ≥ depth_min (0.10, Bulkowski) with V = the lowest low strictly between. Breakout −1: close < V; StockCharts filters: the close must be below V − pen_pct × V (0; StockCharts 0.03) on hold_bars consecutive bars (0 = first close; StockCharts 3). Dropped on a close above max(P1, P2) or after max_wait (0 = P2 − P1) bars. src 1 (LMW): swings and the between-extremes tests on the close. Target = V − (max − V)"), double_top),
    def("chart_double_bottom", "Double Bottom", DB, concat!(sw!(), "Double Bottom (Bulkowski aadb; StockCharts Double Bottom Reversal; Lo-Mamaysky-Wang Def. 5 DBOT): mirror of chart_double_top: valleys B1, B2 > min_sep (22) bars apart; equal bottoms rule 0 (StockCharts \"within 3% of the previous low\") |B1 − B2|/B1 ≤ tol 0.03, rule 1 LMW within tol of their average; no low between below min(B1, B2); (Pk − min)/min ≥ depth_min 0.10 with Pk = the highest high strictly between. Breakout +1: close > Pk, with the same pen_pct / hold_bars filters. Dropped on a close below min(B1, B2) or after max_wait (0 = B2 − B1) bars. src 1 (LMW) as for the top. Target = Pk + (Pk − min)"), double_bottom),
    def("chart_triple_top", "Triple Top", TRP, concat!(sw!(), "Triple Top (Bulkowski tt; StockCharts Triple Top Reversal): last 5 swings peak, valley, peak, valley, peak; the three peaks within top_tol of their average (ADAPT 0.03: the sources say \"near the same price\" with no number); not a head and shoulders: middle peak − max(outer peaks) ≤ top_tol × max(outer peaks). Breakout −1: close below the lowest valley (Bulkowski, StockCharts). Dropped on a close above the highest peak or after max_wait (0 = last − first pivot) bars. Target = Vmin − (Pmax − Vmin)"), triple_top),
    def("chart_triple_bottom", "Triple Bottom", TRP, concat!(sw!(), "Triple Bottom (Bulkowski tb; StockCharts Triple Bottom Reversal): mirror of chart_triple_top: three valleys within top_tol (ADAPT 0.03) of their average, middle valley not more than top_tol below the outer ones. Breakout +1: close above the highest peak between the valleys. Dropped on a close below the lowest valley or after max_wait (0 = last − first pivot) bars. Target = Pmax + (Pmax − Bmin)"), triple_bottom),
    def("chart_cup_handle", "Cup and Handle", CUP, concat!(sw!(), "Cup with Handle (Bulkowski cup; StockCharts Cup with Handle after O'Neil): when a peak C (right rim) is confirmed, the nearest earlier peak A (left rim) with cup_min ≤ C.bar − A.bar ≤ cup_max (35..325 bars = Bulkowski 7..65 weeks; StockCharts 1–6 months ≈ 22..130), |A − C|/max(A, C) ≤ rim_tol (ADAPT 0.05), no high strictly between above max(A, C); B = the lowest low strictly between; depth (A − B)/(A − Z) ≤ depth_max (StockCharts 1/3; 1 = off) with Z = the swing valley before A. Handle: dropped when a low < C − handle_max × (C − B) (Bulkowski 0.5 = upper half of the cup; StockCharts 1/3) or a high > C without a breakout close, on any bar after C (the bars before C is confirmed are checked at detection). Breakout +1: close > C on a bar ≥ C.bar + handle_min (Bulkowski 1 week = 5). No expiry (Bulkowski: no handle maximum). The U-shape test is not built (no source number). Target = C + (C − B)"), cup_handle),
    def("chart_inv_cup_handle", "Inverted Cup and Handle", ICUP, concat!(sw!(), "Inverted Cup with Handle (Bulkowski icup): when a valley C (right rim) is confirmed, the nearest earlier valley A with cup_min ≤ C.bar − A.bar ≤ cup_max (ADAPT: the cup's 35..325), |A − C|/max(A, C) ≤ rim_tol (ADAPT 0.05), no low strictly between below min(A, C); B = the highest high strictly between. Handle: dropped when a high > C + handle_max × (B − C) (Bulkowski: retraces 30%–60% and never above the cup top; 0.6) on any bar after C. Breakout −1: close < C (\"closes below the right cup lip\"). No expiry. Target = C − (B − C)"), inv_cup_handle),
    def("chart_triangle_sym", "Triangle (Symmetrical)", TRI, concat!(sw!(), "Symmetrical Triangle (Bulkowski st; StockCharts Symmetrical Triangle; Lo-Mamaysky-Wang Def. 3 TTOP/TBOT): last 5 swings, peaks strictly falling and valleys strictly rising; upper line through the first and last peak, lower through the first and last valley, the middle pivot of the 3-pivot side within touch_tol (ADAPT 0.01) of its line value. Breakout +1 close > upper(t), −1 close < lower(t), until the bar before the apex: timing 0 (Bulkowski) any such bar; timing 1 (StockCharts \"1/2 to 3/4 of the way\") only bars t with x0 + 0.5 × (apex − x0) ≤ t ≤ x0 + 0.75 × (apex − x0), x0 = the first pivot bar; a close beyond a line before that window ends it silently. src 1 (LMW): swings on the close. Target = level ± (upper − lower at the first swing) (StockCharts method 1)"), tri_sym),
    def("chart_triangle_asc", "Ascending Triangle", ASC, concat!(sw!(), "Ascending Triangle (Bulkowski at; StockCharts Ascending Triangle): last 5 swings, peaks within flat_tol (ADAPT 0.01) of their average R, valleys strictly rising (StockCharts: an equal or lower reaction low invalidates); lower line through the first and last valley (middle within touch_tol). Breakout +1 close > R, −1 close < lower(t), until the bar before the lower line reaches R. Target = level ± (R − first valley)"), tri_asc),
    def("chart_triangle_desc", "Descending Triangle", ASC, concat!(sw!(), "Descending Triangle (Bulkowski dt; StockCharts Descending Triangle): mirror of chart_triangle_asc: valleys within flat_tol (ADAPT 0.01) of their average S, peaks strictly falling, upper line through the first and last peak. Breakout −1 close < S, +1 close > upper(t) (Bulkowski: \"closes outside one of the trendlines\"), until the apex. Target = level ± (first peak − S)"), tri_desc),
    def("chart_wedge_rising", "Rising Wedge", WDG, concat!(sw!(), "Rising Wedge (Bulkowski risewedge; StockCharts Rising Wedge): last 5 swings, peaks and valleys each strictly rising; lines through the first and last peak / valley (middle within touch_tol), 0 < upper slope < lower slope (converging); last − first pivot ≥ min_span (Bulkowski 3 weeks = 15). Breakout dir 0 (Bulkowski) +1 close > upper(t) or −1 close < lower(t); dir 1 (StockCharts) −1 only (an upper break just ends it); until the apex. Target (ADAPT, no source rule) = level ± (upper − lower at the first swing)"), wedge_rising),
    def("chart_wedge_falling", "Falling Wedge", WDG, concat!(sw!(), "Falling Wedge (Bulkowski fallwedge; StockCharts Falling Wedge): mirror of chart_wedge_rising: peaks and valleys each strictly falling, upper slope < lower slope < 0 (converging: the upper line falls faster), span ≥ min_span (15). Breakout dir 0 (Bulkowski) either way; dir 1 (StockCharts) +1 only; until the apex. Target (ADAPT) = level ± (upper − lower at the first swing)"), wedge_falling),
    def("chart_flag", "Flag", FLAG, concat!(sw!(), "Flag (Bulkowski flags; StockCharts Flag): bull: last 5 swings valley A, peak B, valley V1, peak P2, valley V2; pole B.bar − A.bar ≤ pole_max (ADAPT 15) and B − A ≥ pole_atr (ADAPT 3; 0 = off) × ATR(14) at B (Wilder RMA of the true range; Bulkowski \"unusually steep\", no number); flag lines B–P2 and V1–V2 parallel: |s_up − s_lo| ≤ par_tol (ADAPT 0.3) × max(|s_up|, |s_lo|); V2.bar − B.bar ≤ flag_max (Bulkowski 3 weeks = 15). Breakout +1 close > upper(t); −1 close < lower(t) when dir 0 (Bulkowski, either way), only ends it when dir 1 (default, research 14: StockCharts, with the pole). Dropped after max_wait (0 = V2 − A) bars. Target = level ± (B − A) (StockCharts: the pole from the break). Bear flag mirrored, merged into the same output (the bull one wins a shared bar)"), flag),
    def("chart_pennant", "Pennant", PEN, concat!(sw!(), "Pennant (Bulkowski pennants; StockCharts Pennant): pole and swings as chart_flag (pole_max 15, pole_atr 3, flag_max 15, all ADAPT except Bulkowski's 3 weeks), consolidation converging: P2 < B and V2 > V1 (\"a short symmetrical triangle\"), lines B–P2 and V1–V2. Breakout +1 close > upper(t); −1 close < lower(t) when dir 0 (Bulkowski), only ends it when dir 1 (default; StockCharts); until the apex. Target = level ± (B − A). Bear pennant mirrored into the same output"), pennant),
    def("chart_rectangle", "Rectangle", RECT, concat!(sw!(), "Rectangle (Bulkowski recttops/rectbots; StockCharts Rectangle; Lo-Mamaysky-Wang Def. 4 RTOP/RBOT): last 5 swings, tops within tol (0.0075, LMW) of their average R, bottoms within tol of their average S, lowest top > highest bottom, last − first pivot ≥ min_span (StockCharts 3 weeks = 15). Breakout +1 close > R, −1 close < S. Dropped after max_wait (0 = last − first pivot) bars. src 1 (LMW): swings on the close. Target = level ± (R − S)"), rectangle),
    def("chart_channel", "Channel", CHAN, concat!(sw!(), "Channel (Bulkowski channels; StockCharts Price Channel): last 4 swings, line through the two peaks (slope s_p) and through the two valleys (s_v); same sign, non-zero; parallel |s_p − s_v| ≤ par_tol (ADAPT 0.3) × max(|s_p|, |s_v|); not flat |s_v| × (last − first pivot bars) > flat_pct (ADAPT 0.02) × first valley (a flat one is a rectangle). points 1 (Bulkowski 3PointChannels): last 3 swings; bull: valley, peak, valley with rising valleys, main line through the valleys, upper line parallel through the peak; bear: peak, valley, peak with falling peaks, main line through the peaks, lower line parallel through the valley; not flat |s| × span > flat_pct × first pivot. Breakout +1 close > upper(t), −1 close < lower(t) (Bulkowski: any direction). Dropped after max_wait (0 = last − first pivot) bars. No target rule in the sources: target NaN"), channel),
    def("chart_broadening", "Broadening Formation", BRD, concat!(sw!(), "Broadening Top/Bottom (Bulkowski bt, broadb; StockCharts Broadening Top; Lo-Mamaysky-Wang Def. 2 BTOP/BBOT): last 5 swings, peaks strictly rising and valleys strictly falling; lines through the first and last peak / valley, the middle pivot within touch_tol (ADAPT 0.01). Breakout +1 close > upper(t), −1 close < lower(t); pierce 1 (Bulkowski \"pierces a trendline\"): high > upper(t) or low < lower(t), the upper test first. Dropped after max_wait (0 = last − first pivot) bars. src 1 (LMW): swings on the close. Target (ADAPT, no source rule) = level ± (upper − lower) at the event bar"), broadening),
    def("chart_barr", "Bump and Run Reversal", BARR, concat!(sw!(), "Bump and Run Reversal (Bulkowski barrt, barrb; StockCharts Bump and Run Reversal): top: when a peak P is confirmed and no high since the swing valley V2 before it is above P, V1 = the nearest earlier swing valley with V2.bar − V1.bar ≥ lead_min (Bulkowski a month = 22) and V1 < V2; lead-in line V1–V2; lead-in height h1 = max(high − line) over V1..V2 (ADAPT causal stand-in for \"the first quarter\"); bump P − line(P.bar) ≥ bump_ratio (2) × h1. Event −1: close < line(t) (Bulkowski: confirms on a close below the trendline). Dropped after max_wait (0 = P − V1) bars. Bottom mirrored (+1, line through falling peaks). The 30–60 degree angle rules are not built (degrees depend on chart scaling). Target NaN (book only)"), bump_run),
    def("chart_measured_move", "Measured Move", MM, concat!(sw!(), "Measured Move up/down (Bulkowski mmu/mmd automated algorithm; StockCharts Measured Move): up: last 3 swings valley A, peak B, valley C with C > A and retrace r = (B − C)/(B − A) in [r_min, r_max] (StockCharts 0.33..0.67; Bulkowski automated 0.70..<1); +1 on the bar C is confirmed (Bulkowski: buy once the second leg begins; no breakout in either source), level = C, target = C + (B − A); a later, more extreme C on the same A–B leg does not fire again. Down mirrored (−1, target C − (A − B))"), measured_move),
    IndicatorDef { id: "chart_dead_cat", name: "Dead Cat Bounce (event day)", category: CAT, params: DCB, outputs: &["signal"], placement: Placement::Pane, needs: Needs::NONE,
        definition: "Dead Cat Bounce event day (Bulkowski dcb: \"price usually gaps downward, closing 15% to 70% lower than the prior day\"): −1 when close ≤ (1 − drop_min) × close[1] (drop_min 0.15), and with gap = 1 also high < low[1]; else 0; NaN on bar 0. Only the event day is a rule: the bounce and later decline are Bulkowski's statistics and are not built",
        first: &[1], sparse: false, compute: Some(dead_cat), note: "" },
];

pub const SOURCES: &[(&str, &str, &str)] = &[
    ("chart_hs_top", "Bulkowski ThePatternSite: Head-and-Shoulders Top; StockCharts ChartSchool: Head and Shoulders Top; Lo-Mamaysky-Wang (2000): HS", ""),
    ("chart_hs_bottom", "Bulkowski ThePatternSite: Head-and-Shoulders Bottom; StockCharts ChartSchool: Head and Shoulders Bottom; Lo-Mamaysky-Wang (2000): IHS", ""),
    ("chart_double_top", "Bulkowski ThePatternSite: Double Tops (Adam & Adam, Adam & Eve); StockCharts ChartSchool: Double Top Reversal; Lo-Mamaysky-Wang (2000): DTOP", ""),
    ("chart_double_bottom", "Bulkowski ThePatternSite: Double Bottoms (Adam & Adam); StockCharts ChartSchool: Double Bottom Reversal; Lo-Mamaysky-Wang (2000): DBOT", ""),
    ("chart_triple_top", "Bulkowski ThePatternSite: Triple Tops; StockCharts ChartSchool: Triple Top Reversal", ""),
    ("chart_triple_bottom", "Bulkowski ThePatternSite: Triple Bottoms; StockCharts ChartSchool: Triple Bottom Reversal", ""),
    ("chart_cup_handle", "Bulkowski ThePatternSite: Cup with Handle; StockCharts ChartSchool: Cup with Handle", ""),
    ("chart_inv_cup_handle", "Bulkowski ThePatternSite: Cup with Handle, Inverted", ""),
    ("chart_triangle_sym", "Bulkowski ThePatternSite: Triangles, Symmetrical; StockCharts ChartSchool: Symmetrical Triangle; Lo-Mamaysky-Wang (2000): TTOP/TBOT; Trendoscope Auto Chart Patterns (TradingView): Converging Triangle", ""),
    ("chart_triangle_asc", "Bulkowski ThePatternSite: Triangles, Ascending; StockCharts ChartSchool: Ascending Triangle; Trendoscope Auto Chart Patterns (TradingView): Ascending Triangle (Contracting)", ""),
    ("chart_triangle_desc", "Bulkowski ThePatternSite: Triangles, Descending; StockCharts ChartSchool: Descending Triangle; Trendoscope Auto Chart Patterns (TradingView): Descending Triangle (Contracting)", ""),
    ("chart_wedge_rising", "Bulkowski ThePatternSite: Wedges, Rising; StockCharts ChartSchool: Rising Wedge; Trendoscope Auto Chart Patterns (TradingView): Rising Wedge (Contracting)", ""),
    ("chart_wedge_falling", "Bulkowski ThePatternSite: Wedges, Falling; StockCharts ChartSchool: Falling Wedge; Trendoscope Auto Chart Patterns (TradingView): Falling Wedge (Contracting)", ""),
    ("chart_flag", "Bulkowski ThePatternSite: Flags; StockCharts ChartSchool: Flag, Pennant", ""),
    ("chart_pennant", "Bulkowski ThePatternSite: Pennants; StockCharts ChartSchool: Flag, Pennant", ""),
    ("chart_rectangle", "Bulkowski ThePatternSite: Rectangle Tops, Rectangle Bottoms; StockCharts ChartSchool: Rectangle; Lo-Mamaysky-Wang (2000): RTOP/RBOT", ""),
    ("chart_channel", "Bulkowski ThePatternSite: Channels; StockCharts ChartSchool: Price Channel; Trendoscope Auto Chart Patterns (TradingView): Ascending/Descending Channel", ""),
    ("chart_broadening", "Bulkowski ThePatternSite: Broadening Tops, Broadening Bottoms; StockCharts ChartSchool: Broadening Top (Megaphone); Lo-Mamaysky-Wang (2000): BTOP/BBOT; Trendoscope Auto Chart Patterns (TradingView): Diverging Triangle", ""),
    ("chart_barr", "Bulkowski ThePatternSite: Bump-and-Run Reversal Tops, Bottoms; StockCharts ChartSchool: Bump and Run Reversal", ""),
    ("chart_measured_move", "Bulkowski ThePatternSite: Measured Move Up, Measured Move Down; StockCharts ChartSchool: Measured Move Bullish, Bearish", ""),
    ("chart_dead_cat", "Bulkowski ThePatternSite: Dead-Cat Bounce", ""),
];

#[cfg(test)]
mod tests {
    use crate::lab::testdata::{assert_series, closes, compiles, from_ohlc, go};
    const N: f64 = f64::NAN;
    const LR1: [(&str, f64); 2] = [("left", 1.), ("right", 1.)];
    fn with<'a>(kv: &[(&'a str, f64)]) -> Vec<(&'a str, f64)> { [&LR1[..], kv].concat() }
    /// One event at bar `at`: signal, level, target; zero/NaN elsewhere.
    fn one(o: &[Vec<f64>], at: usize, sg: f64, level: f64, target: f64) {
        let mut want = vec![vec![0.; o[0].len()], vec![N; o[0].len()], vec![N; o[0].len()]];
        (want[0][at], want[1][at], want[2][at]) = (sg, level, target);
        for k in 0..3 { assert_series(&o[k], &want[k]); }
    }
    fn none(o: &[Vec<f64>]) { assert!(o[0].iter().all(|v| *v == 0.) && o[1].iter().all(|v| v.is_nan())); }
    fn mir(c: &[f64], k: f64) -> Vec<f64> { c.iter().map(|x| k - x).collect() }

    #[test] fn head_and_shoulders_neckline_modes() {
        // swings (left = right = 1): 12@1, 11@2, 14@3, 11@4, 12@5 confirmed on bar 6; flat neckline 11, height 3
        let c = [10., 12., 11., 14., 11., 12., 11.5, 10.9];
        one(&go("chart_hs_top", &closes(&c), &with(&[])), 7, -1., 11., 8.);
        one(&go("chart_hs_top", &closes(&c), &with(&[("neck_mode", 2.)])), 6, -1., 11., 8.);
        one(&go("chart_hs_bottom", &closes(&mir(&c, 20.)), &with(&[])), 7, 1., 9., 12.);
        // T2 10.8 < T1 11: Bulkowski breaks below the horizontal 10.8 (height 14 − 10.9); StockCharts' line is 10.5 on bar 7
        let c = [10., 12., 11., 14., 10.8, 12., 11.5, 10.6];
        one(&go("chart_hs_top", &closes(&c), &with(&[])), 7, -1., 10.8, 10.8 - 3.1);
        none(&go("chart_hs_top", &closes(&c), &with(&[("neck_mode", 1.)])));
        // a close above the head first drops it
        none(&go("chart_hs_top", &closes(&[10., 12., 11., 14., 11., 12., 11.5, 14.5, 10.]), &with(&[])));
    }
    #[test] fn double_top_and_bottom() {
        // P1 20@1, P2 19.8@5 (|0.2|/20 ≤ 0.03), lowest low between 15, depth 25%; 17@3 fails the tolerance
        let c = [10., 20., 16., 17., 15., 19.8, 18., 14.9];
        one(&go("chart_double_top", &closes(&c), &with(&[("min_sep", 1.)])), 7, -1., 15., 10.);
        none(&go("chart_double_top", &closes(&c), &with(&[])));
        // bottoms 10, 10.2: |0.2|/10 = 0.02; highest high between 15
        one(&go("chart_double_bottom", &closes(&mir(&c, 30.)), &with(&[("min_sep", 1.)])), 7, 1., 15., 20.);
    }
    #[test] fn triple_top_and_bottom() {
        let c = [10., 20., 15., 20.2, 15.5, 19.9, 17., 14.8];
        one(&go("chart_triple_top", &closes(&c), &with(&[])), 7, -1., 15., 15. - 5.2);
        one(&go("chart_triple_bottom", &closes(&mir(&c, 40.)), &with(&[])), 7, 1., 25., 25. + 5.2);
    }
    #[test] fn symmetrical_triangle_breaks_inside_the_apex() {
        // upper 30 − 2(t − 1) through 30, 26, 22; lower 10 + 0.5(t − 2); apex 9.2; bar 7 close 18.5 > 18; widest height 30 − 9.5
        let c = [15., 30., 10., 26., 11., 22., 19., 18.5];
        one(&go("chart_triangle_sym", &closes(&c), &with(&[])), 7, 1., 18., 38.5);
        // middle peak 27 is 1/26 off the line: outside touch_tol 0.01
        none(&go("chart_triangle_sym", &closes(&[15., 30., 10., 27., 11., 22., 19., 18.5]), &with(&[])));
    }
    #[test] fn ascending_and_descending_triangles() {
        // peaks 20, 20.1, 19.95 (R = mean), valleys 10, 14: lower 10 + 2(t − 2) meets R after bar 7
        let (c, r) = ([15., 20., 10., 20.1, 14., 19.95, 18.5, 20.5], 60.05 / 3.);
        one(&go("chart_triangle_asc", &closes(&c), &with(&[])), 7, 1., r, 2. * r - 10.);
        one(&go("chart_triangle_desc", &closes(&mir(&c, 40.)), &with(&[])), 7, -1., 40. - r, 40. - (2. * r - 10.));
    }
    #[test] fn wedges_and_stockcharts_direction() {
        // peaks 20, 22, 24 (slope 1), valleys 10, 14 (slope 2): apex 13; bar 7 close 19.5 < lower 20; height at bar 1 = 20 − 8
        let c = [5., 20., 10., 22., 14., 24., 21., 19.5];
        one(&go("chart_wedge_rising", &closes(&c), &with(&[("min_span", 1.)])), 7, -1., 20., 8.);
        none(&go("chart_wedge_rising", &closes(&c), &with(&[])));
        one(&go("chart_wedge_falling", &closes(&mir(&c, 40.)), &with(&[("min_span", 1.)])), 7, 1., 20., 32.);
        // an upper break: Bulkowski counts it, StockCharts only ends the pattern
        let c = [5., 20., 10., 22., 14., 24., 21., 26.5];
        one(&go("chart_wedge_rising", &closes(&c), &with(&[("min_span", 1.)])), 7, 1., 26., 38.);
        none(&go("chart_wedge_rising", &closes(&c), &with(&[("min_span", 1.), ("dir", 1.)])));
    }
    #[test] fn flag_and_pennant() {
        // pole 10@1 → 20@2; flag lines 20@2–19.5@4 and 18@3–17.5@5, both slope −0.25; bar 7 close 21 > 18.75
        let c = [12., 10., 20., 18., 19.5, 17.5, 18.8, 21.];
        one(&go("chart_flag", &closes(&c), &with(&[("pole_atr", 0.)])), 7, 1., 18.75, 28.75);
        none(&go("chart_pennant", &closes(&c), &with(&[("pole_atr", 0.)])));
        one(&go("chart_flag", &closes(&mir(&c, 40.)), &with(&[("pole_atr", 0.)])), 7, -1., 21.25, 11.25);
        // ATR(14) is not defined at bar 2, so the default pole test fails
        none(&go("chart_flag", &closes(&c), &with(&[])));
        // pennant: 20 → 19.5 falling, 15 → 15.5 rising; apex 12.5
        let c = [12., 10., 20., 15., 19.5, 15.5, 17., 19.5];
        one(&go("chart_pennant", &closes(&c), &with(&[("pole_atr", 0.)])), 7, 1., 18.75, 28.75);
        none(&go("chart_flag", &closes(&c), &with(&[("pole_atr", 0.)])));
    }
    #[test] fn rectangle_channel_broadening() {
        let (r, s) = (60.05 / 3., 10.025);
        one(&go("chart_rectangle", &closes(&[15., 20., 10., 20.1, 10.05, 19.95, 15., 20.5]), &with(&[("min_span", 1.)])), 7, 1., r, 2. * r - s);
        // channel: peaks 20@1, 22@3, valleys 12@2, 14@4 (slopes 1); bar 6 close 14.9 < lower 16
        one(&go("chart_channel", &closes(&[10., 20., 12., 22., 14., 16., 14.9]), &with(&[])), 6, -1., 16., N);
        // broadening: peaks 40, 41, 42 and valleys 32, 31; bar 7 close 29 < 29.5; height there 43 − 29.5
        one(&go("chart_broadening", &closes(&[35., 40., 32., 41., 31., 42., 37., 29.]), &with(&[])), 7, -1., 29.5, 16.);
    }
    #[test] fn cup_and_inverted_cup() {
        // A 10@1, B 6, C 9.8@5 confirmed on bar 6; floor 9.8 − 0.5 × 3.8 = 7.9; breakout allowed from bar 7
        let c = [5., 10., 7., 6., 7., 9.8, 9., 9.5, 10.2];
        let kv = with(&[("cup_min", 1.), ("depth_max", 1.), ("handle_min", 2.)]);
        one(&go("chart_cup_handle", &closes(&c), &kv), 8, 1., 9.8, 13.6);
        // too early: the high above C kills it
        none(&go("chart_cup_handle", &closes(&c), &with(&[("cup_min", 1.), ("depth_max", 1.), ("handle_min", 4.)])));
        // no valley before A: the StockCharts depth rule cannot pass
        none(&go("chart_cup_handle", &closes(&c), &with(&[("cup_min", 1.), ("handle_min", 2.)])));
        one(&go("chart_inv_cup_handle", &closes(&mir(&c, 20.)), &with(&[("cup_min", 1.)])), 8, -1., 10.2, 6.4);
    }
    #[test] fn bump_and_run_top() {
        // lead-in 10@1 → 11@3 (slope 0.5), h1 = 13 − 10.5 = 2.5; bump 20@5, h2 = 20 − 12 = 8 ≥ 5; bar 8 close 12.9 < 13.5
        let c = [12., 10., 13., 11., 12., 20., 18., 16., 12.9];
        one(&go("chart_barr", &closes(&c), &with(&[("lead_min", 2.)])), 8, -1., 13.5, N);
        none(&go("chart_barr", &closes(&c), &with(&[("lead_min", 2.), ("bump_ratio", 3.3)])));
        one(&go("chart_barr", &closes(&mir(&c, 30.)), &with(&[("lead_min", 2.)])), 8, 1., 16.5, N);
    }
    #[test] fn measured_move_fires_on_point_c() {
        let c = [12., 10., 20., 15., 17.];
        one(&go("chart_measured_move", &closes(&c), &with(&[])), 4, 1., 15., 25.);
        one(&go("chart_measured_move", &closes(&mir(&c, 30.)), &with(&[])), 4, -1., 15., 5.);
        none(&go("chart_measured_move", &closes(&c), &with(&[("r_max", 0.4)])));
    }
    #[test] fn dead_cat_event_day() {
        assert_series(&go("chart_dead_cat", &closes(&[100., 84., 80.]), &[])[0], &[N, -1., 0.]);
        let b = from_ohlc(&[(100., 101., 99., 100.), (90., 99.5, 80., 84.), (79., 79., 60., 60.)]);
        assert_series(&go("chart_dead_cat", &b, &[("gap", 1.)])[0], &[N, 0., -1.]);
    }
    fn def(id: &str) -> &'static crate::lab::catalogue::IndicatorDef { super::DEFS.iter().find(|d| d.id == id).unwrap() }
    #[test] fn falling_wedge_text_states_the_converging_rule() {
        let d = def("chart_wedge_falling").definition;
        assert!(d.contains("upper slope < lower slope < 0") && !d.contains("lower slope < upper slope"));
    }
    #[test] fn cup_handle_kill_covers_the_confirmation_window() {
        // right = 2: C 9.8@5 is confirmed on bar 7; floor 9.8 − 0.5 × 3.8 = 7.9; the bar-6 low 7.5 already breaks it
        let kv = [("left", 1.), ("right", 2.), ("cup_min", 1.), ("depth_max", 1.), ("handle_min", 2.)];
        none(&go("chart_cup_handle", &closes(&[5., 10., 7., 6., 7., 9.8, 7.5, 9., 9.5, 10.2]), &kv));
        one(&go("chart_cup_handle", &closes(&[5., 10., 7., 6., 7., 9.8, 8., 9., 9.5, 10.2]), &kv), 9, 1., 9.8, 13.6);
    }
    #[test] fn measured_move_leg_fires_once() {
        // lows: A 10@1, C 15@3 (retrace 0.5, fires on bar 4), then C' 14@5 (retrace 0.6) with no peak between: same A–B leg
        let r = [(12., 12., 12., 12.), (10., 10., 10., 10.), (20., 20., 20., 20.), (15., 19., 15., 15.), (16., 18., 16., 16.), (14., 17., 14., 14.), (17., 18.5, 17., 17.)];
        one(&go("chart_measured_move", &from_ohlc(&r), &with(&[])), 4, 1., 15., 25.);
        let m: Vec<_> = r.iter().map(|&(o, h, l, c)| (30. - o, 30. - l, 30. - h, 30. - c)).collect();
        one(&go("chart_measured_move", &from_ohlc(&m), &with(&[])), 4, -1., 15., 5.);
    }
    #[test] fn max_wait_and_engine_rules_are_labelled() {
        for d in super::DEFS.iter().filter(|d| d.id != "chart_dead_cat") {
            for k in ["sharing a pivot with one that fired is dropped (ADAPT", "adds the peak first", "same pivots as a pending one is not added",
                      "breakout close is tested before the high/low drop rules", "its pivots count as used", "max_wait when > 0 (ADAPT"] {
                assert!(d.definition.contains(k), "{}: {k}", d.id);
            }
            assert!(!d.definition.contains("after ("), "{}: unlabelled expiry", d.id);
            assert_eq!(d.definition.contains("after max_wait"), d.params.iter().any(|q| q.key == "max_wait"), "{}", d.id);
        }
        // P1 20@1, P2 19.8@5 detected on bar 6; the close first breaks 15 on bar 12: span 4 expires it, max_wait 6 keeps it
        let c = [10., 20., 16., 17., 15., 19.8, 18., 17.5, 17., 16.5, 16., 15.5, 14.9];
        none(&go("chart_double_top", &closes(&c), &with(&[("min_sep", 1.)])));
        none(&go("chart_double_top", &closes(&c), &with(&[("min_sep", 1.), ("max_wait", 5.)])));
        one(&go("chart_double_top", &closes(&c), &with(&[("min_sep", 1.), ("max_wait", 6.)])), 12, -1., 15., 10.);
    }
    #[test] fn inverted_cup_bars_are_labelled_adapt() {
        for q in def("chart_inv_cup_handle").params.iter().filter(|q| q.key.starts_with("cup_")) { assert!(q.label.contains("ADAPT") && !q.label.contains("weeks"), "{}", q.label); }
    }
    #[test] fn flag_defaults_to_with_the_pole() {
        // bull flag (lower line 18 − 0.25(t − 3)); bar 7 close 16.5 < 17 breaks against the pole
        let c = [12., 10., 20., 18., 19.5, 17.5, 18., 16.5];
        none(&go("chart_flag", &closes(&c), &with(&[("pole_atr", 0.)])));
        one(&go("chart_flag", &closes(&c), &with(&[("pole_atr", 0.), ("dir", 0.)])), 7, -1., 17., 7.);
    }
    #[test] fn platform_variants() {
        // StockCharts double-top filters: 3 closes below 15 (bars 7..9), or a close below 15 × (1 − 0.015) = 14.775 (bar 9)
        let c = [10., 20., 16., 17., 15., 19.8, 18., 14.9, 14.8, 14.7];
        one(&go("chart_double_top", &closes(&c), &with(&[("min_sep", 1.)])), 7, -1., 15., 10.);
        one(&go("chart_double_top", &closes(&c), &with(&[("min_sep", 1.), ("hold_bars", 3.)])), 9, -1., 15., 10.);
        one(&go("chart_double_top", &closes(&c), &with(&[("min_sep", 1.), ("pen_pct", 0.015)])), 9, -1., 15., 10.);
        // Lo-Mamaysky-Wang close swings: a bar-1 high of 25 spoils the high/low tops only
        let mut r: Vec<_> = c[..8].iter().map(|&x| (x, x, x, x)).collect();
        r[1] = (20., 25., 20., 20.);
        none(&go("chart_double_top", &from_ohlc(&r), &with(&[("min_sep", 1.)])));
        one(&go("chart_double_top", &from_ohlc(&r), &with(&[("min_sep", 1.), ("src", 1.)])), 7, -1., 15., 10.);
        // symmetrical triangle, apex 9.2 from x0 = 1: StockCharts window bars 6..7; bar 8 close 17 > 16 only counts for Bulkowski
        let c = [15., 30., 10., 26., 11., 22., 19., 18., 17.];
        one(&go("chart_triangle_sym", &closes(&c), &with(&[])), 8, 1., 16., 36.5);
        none(&go("chart_triangle_sym", &closes(&c), &with(&[("timing", 1.)])));
        one(&go("chart_triangle_sym", &closes(&[15., 30., 10., 26., 11., 22., 19., 18.5]), &with(&[("timing", 1.)])), 7, 1., 18., 38.5);
        // broadening: the bar-6 low 28 pierces lower(6) = 30 (height 42.5 − 30); the close breaks only on bar 7
        let mut r: Vec<_> = [35., 40., 32., 41., 31., 42., 37., 29.].iter().map(|&x| (x, x, x, x)).collect();
        r[6] = (37., 37., 28., 37.);
        one(&go("chart_broadening", &from_ohlc(&r), &with(&[])), 7, -1., 29.5, 16.);
        one(&go("chart_broadening", &from_ohlc(&r), &with(&[("pierce", 1.)])), 6, -1., 30., 17.5);
        // 3-point channel: lows 12@2, 14@4 (lower 12 + (t − 2)), upper parallel through 22@3; the 4-point lines 25→22 / 12→14 diverge
        let c = [10., 25., 12., 22., 14., 14.5];
        none(&go("chart_channel", &closes(&c), &with(&[])));
        one(&go("chart_channel", &closes(&c), &with(&[("points", 1.)])), 5, -1., 15., N);
    }
    #[test] fn every_output_compiles_in_rules() {
        for d in super::DEFS { for o in d.outputs { compiles(d.id, o).unwrap_or_else(|e| panic!("{}.{o}: {e:?}", d.id)); } }
    }
}
