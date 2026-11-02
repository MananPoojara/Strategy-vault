//! TradingView built-in indicators the lab did not have (2026-10-30 gap check against TradingView's built-in list): the
//! zigzag-based "auto" drawings, computed bar by bar. Sources: the built-ins' own Pine source as TradingView serves it to the
//! Pine editor (pine-facade, fetched 2026-10-30): STD;Zig_Zag v44 ("Zigzag"), STD;Auto%1Fib%1Extension%1 v30, STD;Auto%1Fib%1Retracement%1
//! v40, STD;Auto%1Pitchfork v13, and the TradingView/ZigZag library they import (https://www.tradingview.com/script/bzIRuGXC-ZigZag/,
//! v7 for the auto drawings, v9 for Zigzag; the confirmed-pivot logic is the same in both). Each value is what the built-in
//! draws when bar t is the chart's last bar, so every output is causal. Not computed (already listed in listed.rs as
//! auto_key_levels and auto_trendlines): Auto key levels (STD;Auto_Key_Levels: TradingView refuses its source, "User is not
//! allowed to see source code of pine") and Auto trendlines (not a Pine built-in: absent from TradingView's standard script list).
use super::bars::Bars;
use super::catalogue::{int, n, p, real, IndicatorDef, Needs, Output, ParamSpec, Params, Placement};
use super::ta::{self, NA};

#[allow(clippy::too_many_arguments)]
const fn def(id: &'static str, name: &'static str, category: &'static str, params: &'static [ParamSpec], outputs: &'static [&'static str],
             definition: &'static str, first: &'static [usize], f: fn(&Bars, &Params) -> Output) -> IndicatorDef {
    IndicatorDef { id, name, category, params, outputs, placement: Placement::Price, needs: Needs::NONE, definition, first, sparse: true, compute: Some(f), note: "" }
}
const X: usize = usize::MAX;
fn flag(b: bool) -> f64 { if b { 1. } else { 0. } }

/// A TradingView/ZigZag library pivot: is_high, start (bar, price) = the previous pivot, end (bar, price) = this pivot.
#[derive(Clone, Copy)]
struct Pivot { high: bool, start: (usize, f64), end: (usize, f64) }
#[derive(PartialEq)]
enum Ev { None, New, Moved }
/// The library's confirmed-pivot state (projection pivots, drawn on the last bar only, are not part of it).
struct ZigZag { dev: f64, legs: usize, piv: Vec<Pivot> }
impl ZigZag {
    fn new(dev: f64, depth: usize) -> Self { ZigZag { dev, legs: 2.max(depth / 2), piv: Vec::new() } }
    /// findPivotPoint: the bar t − d is a pivot when the d bars after it are not beyond it and the d bars before it are strictly inside.
    fn find(src: &[f64], t: usize, d: usize, high: bool) -> Option<(usize, f64)> {
        if 2 * d > t { return None; }
        let v = src[t - d];
        let beyond = |x: f64, tie: bool| if high { x > v || (tie && x == v) } else { x < v || (tie && x == v) };
        if (0..d).any(|i| beyond(src[t - i], false)) || (d + 1..=2 * d).any(|i| beyond(src[t - i], true)) { return None; }
        Some((t - d, v))
    }
    /// newPivotPointFound: extend the last pivot of the same kind, or add the opposite kind once it deviates by dev %.
    fn found(&mut self, high: bool, pt: (usize, f64)) -> Ev {
        let only = self.piv.len() == 1;
        let Some(last) = self.piv.last_mut() else { self.piv.push(Pivot { high, start: pt, end: pt }); return Ev::New; };
        if last.high == high {
            if (high && pt.1 > last.end.1) || (!high && pt.1 < last.end.1) {
                if only { last.start = pt; }
                last.end = pt;
                return Ev::Moved;
            }
            return Ev::None;
        }
        let dev = 100. * (pt.1 - last.end.1) / last.end.1.abs();
        if (!last.high && dev >= self.dev) || (last.high && dev <= -self.dev) {
            let start = last.end;
            self.piv.push(Pivot { high, start, end: pt });
            return Ev::New;
        }
        Ev::None
    }
    /// update(): high pivot first, then low pivot (allowZigZagOnOneBar = true in every caller).
    fn update(&mut self, b: &Bars, t: usize) -> (Ev, Ev) {
        let h = Self::find(&b.high, t, self.legs, true).map_or(Ev::None, |pt| self.found(true, pt));
        let l = Self::find(&b.low, t, self.legs, false).map_or(Ev::None, |pt| self.found(false, pt));
        (h, l)
    }
}

const ZZ: &[ParamSpec] = &[real("deviation", "Price deviation for reversals (%)", 5., 0.00001, 100., 0.5), int("depth", "Pivot legs", 10., 2., 500.)];
fn tv_zigzag(b: &Bars, prm: &Params) -> Output {
    let m = b.len();
    let mut z = ZigZag::new(p(prm, "deviation"), n(prm, "depth"));
    let mut o = vec![vec![NA; m]; 7]; // pivot, prev_pivot, direction, bars_since, new_high, new_low, moved
    for t in 0..m {
        let (h, l) = z.update(b, t);
        o[4][t] = flag(h == Ev::New); o[5][t] = flag(l == Ev::New); o[6][t] = flag(h == Ev::Moved || l == Ev::Moved);
        if let Some(last) = z.piv.last() {
            o[0][t] = last.end.1;
            o[2][t] = if last.high { -1. } else { 1. };
            o[3][t] = (t - last.end.0) as f64;
        }
        if z.piv.len() > 1 { o[1][t] = z.piv[z.piv.len() - 2].end.1; }
    }
    Ok(o)
}

/// The default-on level set of both TradingView auto fib built-ins (0.65, 1.272, 1.414, 1.65, 2.65, 3.65, 4.618 and the negatives are off).
const FIB: [f64; 11] = [0., 0.236, 0.382, 0.5, 0.618, 0.786, 1., 1.618, 2.618, 3.618, 4.236];
const FIB_OUT: &[&str] = &["level_0", "level_236", "level_382", "level_500", "level_618", "level_786", "level_1000", "level_1618", "level_2618", "level_3618", "level_4236"];

const AFR: &[ParamSpec] = &[real("deviation", "Deviation (x ATR(10) % of close)", 3., 0., 100., 0.5), int("depth", "Depth", 10., 2., 500.), int("reverse", "Reverse (1 = yes)", 0., 0., 1.)];
fn tv_auto_fib_retracement(b: &Bars, prm: &Params) -> Output {
    let (mult, rev, m) = (p(prm, "deviation"), p(prm, "reverse") == 1., b.len());
    let atr = ta::rma(&ta::tr(b, true), 10); // Pine ta.atr(10)
    let mut z = ZigZag::new(NA, n(prm, "depth"));
    let mut o = vec![vec![NA; m]; FIB.len()];
    for t in 0..m {
        z.dev = atr[t] / b.close[t] * 100. * mult;
        z.update(b, t);
        if z.piv.len() < 2 { continue; } // TradingView: "Not enough data" error
        let last = z.piv[z.piv.len() - 1];
        let (s, e) = if rev { (last.start.1, last.end.1) } else { (last.end.1, last.start.1) };
        let height = if s > e { -(s - e).abs() } else { (s - e).abs() };
        for (k, r) in FIB.iter().enumerate() { o[k][t] = s + height * r; }
    }
    Ok(o)
}

/// Auto fib extension's own pivots(src, length, isHigh): bar t − length is a pivot when no bar of t − 2·length ..= t (those
/// that exist) is strictly beyond it; ties are allowed on both sides.
fn fe_pivot(src: &[f64], t: usize, len: usize, high: bool) -> Option<(usize, f64)> {
    if t < len { return None; }
    let v = src[t - len];
    let beyond = |x: f64| if high { x > v } else { x < v };
    if (t.saturating_sub(2 * len)..=t).any(|i| beyond(src[i])) { None } else { Some((t - len, v)) }
}
/// The built-in's barstate.islast search on copies of the pivot lists: returns (C, B − A) or None ("Not enough data").
fn fe_pattern(ph: &[(usize, f64)], pl: &[(usize, f64)]) -> Option<(f64, f64)> {
    let (mut hc, mut lc) = (ph.to_vec(), pl.to_vec());
    while let (Some(mut lh), Some(ll)) = (hc.pop(), lc.pop()) {
        let piv = if lh.0 > ll.0 { &mut hc } else { &mut lc };
        // drop or absorb same-kind pivots not older than the last low (the Pine loop compares with lastH for both kinds)
        while let Some(&q) = piv.last() {
            if q.0 < ll.0 { break; }
            piv.pop();
            if q.1 > lh.1 { lh = q; }
        }
        if hc.is_empty() || lc.is_empty() { break; }
        let high_last = lh.0 > ll.0;
        let a = if high_last { hc[hc.len() - 1].1 } else { lc[lc.len() - 1].1 };
        if high_last {
            let (bb, d) = (ll.1, (a - ll.1).abs());
            if lh.1 > bb + d || lh.1 < bb + d * 0.236 { lc.push(ll); continue; }
            return Some((lh.1, -d));
        }
        let (bb, d) = (lh.1, (a - lh.1).abs());
        if ll.1 < bb - d || ll.1 > bb - d * 0.236 { hc.push(lh); continue; }
        return Some((ll.1, d));
    }
    None
}
const AFE: &[ParamSpec] = &[int("depth", "Depth", 10., 2., 500.), int("reverse", "Reverse (1 = yes)", 0., 0., 1.)];
fn tv_auto_fib_extension(b: &Bars, prm: &Params) -> Output {
    let (len, sign, m) = (n(prm, "depth") / 2, if p(prm, "reverse") == 1. { -1. } else { 1. }, b.len());
    let (mut ph, mut pl): (Vec<(usize, f64)>, Vec<(usize, f64)>) = (Vec::new(), Vec::new());
    let mut o = vec![vec![NA; m]; FIB.len()];
    for t in 0..m {
        let (mut h, mut l) = (fe_pivot(&b.high, t, len, true), fe_pivot(&b.low, t, len, false));
        if let (Some(&lh), Some(&ll)) = (ph.last(), pl.last()) {
            if lh.0 > ll.0 {
                if let Some(x) = h.take() { if x.1 > lh.1 { *ph.last_mut().unwrap() = x; } }
            } else if let Some(x) = l.take() { if x.1 < ll.1 { *pl.last_mut().unwrap() = x; } }
        }
        ph.extend(h); pl.extend(l);
        if let Some((c, d)) = fe_pattern(&ph, &pl) { for (k, r) in FIB.iter().enumerate() { o[k][t] = c + sign * d * r; } }
    }
    Ok(o)
}

const APF: &[ParamSpec] = &[int("depth", "Depth", 10., 2., 500.), int("type", "Type: 0 Original, 1 Schiff, 2 Modified Schiff, 3 Inside", 0., 0., 3.)];
fn tv_auto_pitchfork(b: &Bars, prm: &Params) -> Output {
    let (typ, m) = (n(prm, "type"), b.len());
    let mut z = ZigZag::new(0., n(prm, "depth"));
    let mut o = vec![vec![NA; m]; 5]; // median, upper_050, lower_050, upper_100, lower_100
    let off = |s: (f64, f64), e: (f64, f64)| (s.1 - e.1) * 0.5 / (s.0 - e.0 - 0.5).abs(); // _calcOffsetPrice
    let odd = |a: usize, c: usize| a.abs_diff(c) % 2 != 0;
    for t in 0..m {
        z.update(b, t);
        let k = z.piv.len();
        if k < 3 { continue; }
        // the built-in reads pivots[k−2] (end = lastP, start = prevP) and pivots[k−3].start (prev2P): the newest pivot is left out
        let (last, prev, prev2) = (z.piv[k - 2].end, z.piv[k - 2].start, z.piv[k - 3].start);
        let f = |q: (usize, f64)| (q.0 as f64, q.1);
        let (lp, pp, p2) = (f(last), f(prev), f(prev2));
        let mut em = (((pp.0 + lp.0) / 2.).floor(), (pp.1 + lp.1) / 2.);
        let need = odd(last.0, prev.0);
        let sm = match typ {
            0 | 1 => {
                let sm = if typ == 0 { p2 } else { (p2.0, (pp.1 + p2.1) / 2.) };
                if need { em.1 += off(sm, em); }
                sm
            }
            2 => {
                let mut sm = (((pp.0 + p2.0) / 2.).floor(), (pp.1 + p2.1) / 2.);
                let d = off(sm, em);
                if odd(prev2.0, prev.0) { sm.1 += d; }
                if need { em.1 += d; }
                sm
            }
            _ => {
                let si = ((pp.1 + p2.1) / 2. - lp.1) / ((pp.0 + p2.0) / 2. - lp.0);
                let sm = (lp.0, si * (lp.0 - (pp.0 + lp.0) / 2.) + em.1);
                if need { em.1 -= off(sm, em); }
                sm
            }
        };
        let slope = (em.1 - sm.1) / (em.0 - sm.0);
        let line = |x0: f64, y0: f64| { let v = y0 + slope * (t as f64 - x0); if v.is_finite() { v } else { NA } };
        o[0][t] = line(sm.0, sm.1);
        // drawLevel: each parallel passes through the handle point mid ± level·half of prevP→lastP, with the median's slope
        let (idiff, pdiff, imid, pmid, up) = ((pp.0 - lp.0).abs() / 2., (pp.1 - lp.1).abs() / 2., (pp.0 + lp.0) / 2., (pp.1 + lp.1) / 2., pp.1 > lp.1);
        for (j, (lev, sg)) in [(0.5, -1.), (0.5, 1.), (1., -1.), (1., 1.)].into_iter().enumerate() {
            let x = imid + (if up { sg * idiff } else { -sg * idiff }) * lev;
            o[1 + j][t] = line(x, pmid - sg * pdiff * lev);
        }
    }
    Ok(o)
}

pub const DEFS: &[IndicatorDef] = &[
    def("zigzag", "Zig Zag (TradingView built-in, confirmed pivots)", "Trend", ZZ, &["pivot", "prev_pivot", "direction", "bars_since", "new_high", "new_low", "moved"],
        "TradingView built-in Zigzag (STD;Zig_Zag v44, Pine v6), which runs the TradingView/ZigZag library v9 update() once per bar. Source rule: legs d = max(2, floor(depth/2)) (\"Pivot legs\" input, default 10, so d = 5). On bar t >= 2d, bar t-d is a pivot high if high[t-i] <= high[t-d] for i = 0..d-1 (later bars may tie) and high[t-i] < high[t-d] for i = d+1..2d (earlier bars may not tie); pivot low mirrors with low. The high is tested first, then the low (allowZigZagOnOneBar = true, so both can register on one bar). A found pivot is registered (newPivotPointFound): with no pivot yet it becomes the first pivot; if the last pivot is the same kind and the new price is strictly beyond it, the last pivot moves to the new bar and price; if the last pivot is the opposite kind, dev = 100*(price - last price)/|last price| and a new pivot is added when last is a low and dev >= deviation, or last is a high and dev <= -deviation (default 5%); otherwise nothing changes. Outputs on bar t, after that bar's update: pivot = price of the last pivot, prev_pivot = price of the one before it, direction = +1 when the last pivot is a low (the zigzag is rising from it) and -1 when it is a high, bars_since = t - bar of the last pivot; new_high/new_low = 1 on the bar a new high/low pivot is added (its confirmation bar, d bars after the pivot bar), moved = 1 on a bar where the last pivot was moved further; levels NaN before the first pivot. Adaptation: the built-in also draws projected pivots (\"Calculate projected pivots\", on by default) on the last bar only and redraws them each bar; they are not confirmed, so they are left out, and the lines and labels are drawing only. Values are the confirmed zigzag as TradingView holds it when bar t is the last bar (a moved pivot changes its level from the bar it moves, never earlier). Sources: TradingView built-in Zigzag Pine source STD;Zig_Zag v44 and library TradingView/ZigZag v9 (https://www.tradingview.com/script/bzIRuGXC-ZigZag/), both fetched 2026-10-30 from TradingView's pine-facade; source URL https://pine-facade.tradingview.com/pine-facade/get/STD%3BZig_Zag/last; Zigzag help: https://www.tradingview.com/support/solutions/43000591664-zigzag-indicator/.",
        &[X, X, X, X, 0, 0, 0], tv_zigzag),
    def("auto_fib_retracement", "Auto Fib Retracement (TradingView built-in)", "Bands & channels", AFR, FIB_OUT,
        "TradingView built-in Auto fib retracement (STD;Auto%1Fib%1Retracement%1 v40, Pine v6). Source rule: a TradingView/ZigZag library v7 zigzag with the same pivot rule as tv_zigzag (legs d = max(2, floor(depth/2)), default depth 10), whose deviation is reset before every update to ATR(10)/close*100*deviation (ATR = Pine ta.atr, Wilder RMA of true range, first value bar 9; deviation input default 3, minimum 0; while ATR is undefined no opposite pivot can register). After the update, take the last pivot (start = the pivot before it, end = the newest pivot; both from the zigzag, so the newest one may still move): start price S = end price (Reverse: start price), E = start price (Reverse: end price), height = (S > E ? -1 : 1)*|S - E|, level_x = S + height*x for the default-on ratios x = 0, 0.236, 0.382, 0.5, 0.618, 0.786, 1, 1.618, 2.618, 3.618, 4.236 (so level_0 is the newest pivot and level_1000 the pivot before it). NaN while the zigzag holds fewer than 2 pivots (TradingView stops with \"Not enough data\"). Causal as published: every value uses bars 0..t only; the drawn lines simply show the latest bar's levels. Adaptation: none to the rule; the optional levels (0.65, 1.272, 1.414, 1.65, 2.65, 3.65, 4.618, -0.236, -0.382, -0.618, -0.65, all off by default) are not output. This differs from fib_retracement, which uses the rolling length-bar high/low range instead of zigzag pivots. Sources: TradingView built-in Auto fib retracement Pine source STD;Auto%1Fib%1Retracement%1 v40 and library TradingView/ZigZag v7 (https://www.tradingview.com/script/bzIRuGXC-ZigZag/), fetched 2026-10-30 from TradingView's pine-facade (source URL https://pine-facade.tradingview.com/pine-facade/get/STD%3BAuto%251Fib%251Retracement%251/last).",
        &[X; 11], tv_auto_fib_retracement),
    def("auto_fib_extension", "Auto Fib Extension (TradingView built-in)", "Bands & channels", AFE, FIB_OUT,
        "TradingView built-in Auto fib extension (STD;Auto%1Fib%1Extension%1 v30, Pine v6). It does not use the zigzag library. Source rule: L = depth/2 (default depth 10, L = 5). On bar t >= L, bar t-L is a pivot high if no high[t-i], i = 0..2L, is strictly greater (ties allowed on both sides; bars before bar 0 do not exist and are skipped); pivot low mirrors with low. Lists PH and PL: when both are non-empty and the newest high is later than the newest low, a new pivot high replaces the newest high only if higher and is otherwise dropped (a pivot low is still appended); when the newest low is later or on the same bar, a new pivot low replaces the newest low only if lower and is otherwise dropped (a pivot high is still appended); otherwise every pivot is appended. Pattern (the built-in's last-bar search, on copies of PH and PL): pop the newest high H and low Lo; on the list of the later one, pop pivots not older than Lo, keeping the higher price as H (the built-in compares with H for either list); stop if a list is empty; A = the newest remaining pivot of the later kind. If H is later (A high, B = Lo, C = H) the pattern is valid when B + 0.236|A-B| <= C <= B + |A-B|; if Lo is later (A low, B = H, C = Lo) when B - |A-B| <= C <= B - 0.236|A-B|. When invalid, put B back on its list (C is discarded) and search again. Levels: level_x = C + (B - A)*x (Reverse: C - (B - A)*x) for the default-on ratios x = 0, 0.236, 0.382, 0.5, 0.618, 0.786, 1, 1.618, 2.618, 3.618, 4.236. Adaptation (causality): TradingView runs the pattern search on the last bar only; here it runs on every bar t with the pivots known at t, i.e. the levels TradingView draws when t is the last bar; NaN when no valid pattern exists (TradingView stops with \"Not enough data\"). Assumption, not verified: for an odd depth L = floor(depth/2) (the Pine divides an int by 2). The optional levels that are off by default are not output. Sources: TradingView built-in Auto fib extension Pine source STD;Auto%1Fib%1Extension%1 v30, fetched 2026-10-30 from TradingView's pine-facade (source URL https://pine-facade.tradingview.com/pine-facade/get/STD%3BAuto%251Fib%251Extension%251/last).",
        &[X; 11], tv_auto_fib_extension),
    def("auto_pitchfork", "Auto Pitchfork (TradingView built-in)", "Bands & channels", APF, &["median", "upper_050", "lower_050", "upper_100", "lower_100"],
        "TradingView built-in Auto pitchfork (STD;Auto%1Pitchfork v13, Pine v6). Source rule: a TradingView/ZigZag library v7 zigzag with deviation 0 and the tv_zigzag pivot rule (legs d = max(2, floor(depth/2)), default depth 10). With k >= 3 zigzag pivots, lastP = end of pivot k-2, prevP = its start (pivot k-3's point), prev2P = start of pivot k-3 (the first pivot's start is its own point); the newest pivot is not used. Bar index i, price y. endMedian = (floor((prevP.i + lastP.i)/2), (prevP.y + lastP.y)/2); off(S, E) = (S.y - E.y)*0.5/|S.i - E.i - 0.5|; need = (lastP.i - prevP.i) odd. Type 0 Original: startMedian = prev2P, endMedian.y += need ? off : 0. Type 1 Schiff: startMedian = (prev2P.i, (prevP.y + prev2P.y)/2), same end offset. Type 2 Modified Schiff: startMedian = (floor((prevP.i + prev2P.i)/2), (prevP.y + prev2P.y)/2), o = off(start, end) before either change; start.y += o if (prev2P.i - prevP.i) odd; end.y += o if need. Type 3 Inside: s = ((prevP.y + prev2P.y)/2 - lastP.y)/((prevP.i + prev2P.i)/2 - lastP.i); startMedian = (lastP.i, s*(lastP.i - (prevP.i + lastP.i)/2) + endMedian.y); endMedian.y -= need ? off : 0. slope = (end.y - start.y)/(end.i - start.i); median[t] = start.y + slope*(t - start.i). Parallels (default-on levels 0.5 and 1; 0.25, 0.382, 0.618, 0.75, 1.5, 1.75, 2 are off by default and not output): handle point = midpoint of prevP-lastP moved by level times half of that segment toward its higher-price end (upper_*) or lower-price end (lower_*); each parallel is the line with the median's slope through that point, valued at bar t (so upper_100 and lower_100 pass through prevP and lastP). NaN before 3 pivots or when the slope is undefined (pivots on one bar). Causal as published: the built-in recomputes the pitchfork from confirmed pivots on every bar; the lines extend right, so the value at t is the current pitchfork's line at bar t. Adaptation: none to the rule; the bar index is the lab's bar position (TradingView's bar_index on a gap-free chart). Sources: TradingView built-in Auto pitchfork Pine source STD;Auto%1Pitchfork v13 and library TradingView/ZigZag v7 (https://www.tradingview.com/script/bzIRuGXC-ZigZag/), fetched 2026-10-30 from TradingView's pine-facade (source URL https://pine-facade.tradingview.com/pine-facade/get/STD%3BAuto%251Pitchfork/last).",
        &[X; 5], tv_auto_pitchfork),
];
/// (id, source, licence/notes), one per entry.
pub const SOURCES: &[(&str, &str, &str)] = &[
    ("zigzag", "TradingView: Zigzag (built-in; TradingView/ZigZag library), confirmed pivots", "Zig"),
    ("auto_fib_retracement", "TradingView: Auto Fib Retracement (built-in, zigzag-based)", ""),
    ("auto_fib_extension", "TradingView: Auto Fib Extension (built-in)", ""),
    ("auto_pitchfork", "TradingView: Auto Pitchfork (built-in)", ""),
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lab::catalogue::run;
    use crate::lab::testdata::{assert_series, compiles, cut, from_ohlc, go, walk};
    const N: f64 = f64::NAN;
    fn hl(h: &[f64], l: &[f64]) -> Bars { from_ohlc(&h.iter().zip(l).map(|(&a, &b)| ((a + b) / 2., a, b, (a + b) / 2.)).collect::<Vec<_>>()) }

    // legs 2 (depth 4): low 1 (bar 2) on bar 4, high 10 (bar 5) on bar 7, high 12 (bar 9) moves it on bar 11, low 2 (bar 12) on
    // bar 14, high 8 (bar 15) on bar 17
    const H: [f64; 19] = [5., 4., 3., 4., 6., 10., 8., 7., 9., 12., 9., 7., 5., 6., 7., 8., 7., 6., 6.5];
    const L: [f64; 19] = [4., 3., 1., 2., 5., 8., 6., 6.5, 7., 10., 7., 6., 2., 4., 5., 6., 5., 4., 5.];
    #[test] fn zigzag_pivots_on_their_confirmation_bar() {
        let o = go("zigzag", &hl(&H[..16], &L[..16]), &[("depth", 4.)]);
        assert_series(&o[0], &[N, N, N, N, 1., 1., 1., 10., 10., 10., 10., 12., 12., 12., 2., 2.]);
        assert_series(&o[1], &[N, N, N, N, N, N, N, 1., 1., 1., 1., 1., 1., 1., 12., 12.]);
        assert_series(&o[2], &[N, N, N, N, 1., 1., 1., -1., -1., -1., -1., -1., -1., -1., 1., 1.]);
        assert_series(&o[3], &[N, N, N, N, 2., 3., 4., 2., 3., 4., 5., 2., 3., 4., 2., 3.]);
        let at = |k: &[usize]| (0..16).map(|i| if k.contains(&i) { 1. } else { 0. }).collect::<Vec<_>>();
        assert_series(&o[4], &at(&[7])); assert_series(&o[5], &at(&[4, 14])); assert_series(&o[6], &at(&[11]));
        // 95%: the fall 12 → 2 (−83%) does not reverse the zigzag
        let o = go("zigzag", &hl(&H[..16], &L[..16]), &[("depth", 4.), ("deviation", 95.)]);
        assert_series(&o[0][13..], &[12., 12., 12.]);
        // the same leg on high[t − d] ties: a later equal high does not stop the pivot, an earlier equal high does
        let none = |h: [f64; 5]| go("zigzag", &hl(&h, &[0.5; 5]), &[("depth", 4.)])[0][4].is_nan();
        assert!(!none([1., 2., 3., 3., 1.]) && none([1., 3., 3., 2., 1.]));
    }
    #[test] fn retracement_levels_span_the_last_leg() {
        // deviation 0: the threshold is NaN until ATR(10) exists (bar 9), so the high 10 confirmed on bar 7 cannot register
        let b = hl(&H[..16], &L[..16]);
        let o = go("auto_fib_retracement", &b, &[("depth", 4.), ("deviation", 0.)]);
        let w = |a: f64, c: f64| { let mut s = vec![N; 11]; s.extend([a; 3]); s.extend([c; 2]); s };
        assert_series(&o[0], &w(12., 2.)); assert_series(&o[6], &w(1., 12.)); assert_series(&o[3], &w(6.5, 7.));
        assert_series(&o[1], &w(12. - 0.236 * 11., 2. + 0.236 * 10.));
        let r = go("auto_fib_retracement", &b, &[("depth", 4.), ("deviation", 0.), ("reverse", 1.)]);
        assert_series(&r[0], &w(1., 12.)); assert_series(&r[7], &w(1. + 1.618 * 11., 12. - 1.618 * 10.));
    }
    #[test] fn extension_projects_ab_from_c() {
        // depth 4 (L 2): high A 10 (bar 2) on bar 4, low B 4 (bar 5) on bar 7, high C 8 (bar 8) on bar 10; C retraces 4/6 of AB
        let h = [7., 8., 10., 9., 7., 6., 7., 7.5, 8., 7., 6.5];
        let l = [6., 7., 9., 7., 5., 4., 5., 6., 7., 6., 5.5];
        let o = go("auto_fib_extension", &hl(&h, &l), &[("depth", 4.)]);
        for (k, r) in FIB.iter().enumerate() { assert!(o[k][..10].iter().all(|v| v.is_nan())); assert_series(&o[k][10..], &[8. - 6. * r]); }
        assert_series(&go("auto_fib_extension", &hl(&h, &l), &[("depth", 4.), ("reverse", 1.)])[7][10..], &[8. + 6. * 1.618]);
        // C above A (beyond 100% of AB) is not a pattern
        let mut h2 = h; h2[8] = 10.5; h2[7] = 9.;
        assert!(go("auto_fib_extension", &hl(&h2, &l), &[("depth", 4.)])[0][10].is_nan());
    }
    #[test] fn pitchfork_original_through_the_three_pivots() {
        let o = go("auto_pitchfork", &hl(&H, &L), &[("depth", 4.)]);
        // 3 pivots from bar 14 (prev2P = prevP = the first pivot, as the built-in reads pivots[0].start)
        assert!(o.iter().all(|s| s[..14].iter().all(|v| v.is_nan()) && s[14].is_finite()));
        // bar 17: pivots 1@2, 12@9, 2@12, 8@15 → prev2P 1@2, prevP 12@9, lastP 2@12 (the newest, 8@15, is left out); the median
        // runs from 1@2 through the handle midpoint 7@10.5, slope 6/8.5
        let s = 6. / 8.5;
        assert_series(&o[0][17..], &[1. + s * 15., 1. + s * 16.]);
        assert_series(&o[3][17..], &[12. + s * 8., 12. + s * 9.]); // through prevP
        assert_series(&o[4][17..], &[2. + s * 5., 2. + s * 6.]); // through lastP
        assert_series(&o[1][17..], &[9.5 + s * 7.25, 9.5 + s * 8.25]); // through 9.5@9.75
        assert_series(&o[2][17..], &[4.5 + s * 5.75, 4.5 + s * 6.75]); // through 4.5@11.25
        // Schiff starts at (2, (12 + 1)/2)
        let sc = go("auto_pitchfork", &hl(&H, &L), &[("depth", 4.), ("type", 1.)]);
        assert_series(&sc[0][17..], &[6.5 + 0.5 / 8.5 * 15., 6.5 + 0.5 / 8.5 * 16.]);
    }
    #[test] fn defaults_produce_values_on_long_history() {
        let w = walk(1500, 8, false);
        for (id, k) in [("zigzag", 0), ("zigzag", 4), ("zigzag", 5), ("auto_fib_retracement", 3), ("auto_fib_extension", 3), ("auto_pitchfork", 0)] {
            assert!(go(id, &w, &[])[k].iter().any(|v| v.is_finite() && *v != 0.), "{id}.{k}");
        }
        for t in 0..4 { assert!(go("auto_pitchfork", &w, &[("type", t as f64)])[4].iter().any(|v| v.is_finite()), "type {t}"); }
    }
    #[test] fn parameter_extremes_keep_lengths() {
        for b in [walk(3, 1, false), walk(300, 1, false)] {
            for d in DEFS {
                for hi in [false, true] {
                    let q: Params = d.params.iter().map(|p| (p.key.to_string(), if hi { p.max } else { p.min })).collect();
                    let o = run(d, &b, &q).unwrap();
                    assert!(o.iter().all(|s| s.len() == b.len() && s.iter().all(|v| !v.is_infinite())), "{} hi={hi}", d.id);
                }
            }
        }
    }
    #[test] fn every_output_is_causal_at_20_cut_points() {
        // value on bar k-1 must not change when bars k.. are removed; the rounded copy forces equal highs/lows (ties)
        let w = walk(900, 5, false);
        let mut r = w.clone();
        for v in [&mut r.open, &mut r.high, &mut r.low, &mut r.close] { for x in v.iter_mut() { *x = x.round(); } }
        let small = [("depth", 4.), ("deviation", 1.), ("type", 2.), ("reverse", 1.)];
        for b in [&w, &r] {
            for d in DEFS {
                for q in [Params::new(), d.params.iter().filter_map(|p| small.iter().find(|(k, _)| *k == p.key).map(|(k, v)| (k.to_string(), *v))).collect()] {
                    let full = run(d, b, &q).unwrap();
                    for k in (1..=20).map(|i| i * b.len() / 20) {
                        let part = run(d, &cut(b, k), &q).unwrap();
                        for (f, g) in full.iter().zip(&part) { assert_series(g, &f[..k]); }
                    }
                }
            }
        }
    }
    #[test] fn rules_can_read_every_output() {
        for d in DEFS { for o in d.outputs { compiles(d.id, o).unwrap_or_else(|e| panic!("{}.{o}: {e:?}", d.id)); } }
    }
    #[test] fn one_source_row_per_entry() {
        for d in DEFS { assert_eq!(SOURCES.iter().filter(|s| s.0 == d.id).count(), 1, "{}", d.id); }
        assert_eq!(SOURCES.len(), DEFS.len());
    }
    /// TV_BARS=<csv date,open,high,low,close> TV_OUT=<json> cargo test --bin guru lab::x_tv_auto::tests::tv_auto_dump -- --ignored --exact
    #[test] #[ignore] fn tv_auto_dump() {
        let text = std::fs::read_to_string(std::env::var("TV_BARS").expect("TV_BARS")).expect("read TV_BARS");
        let mut b = Bars::default();
        for (i, line) in text.lines().skip(1).filter(|s| !s.trim().is_empty()).enumerate() {
            let f: Vec<&str> = line.split(',').map(str::trim).collect();
            let x = |k: usize| f[k].parse::<f64>().unwrap_or(f64::NAN);
            b.date.push(f[0].to_string());
            b.open.push(x(1)); b.high.push(x(2)); b.low.push(x(3)); b.close.push(x(4)); b.end.push(i);
        }
        let num = |v: f64| if v.is_finite() { format!("{v:?}") } else { "null".into() };
        let runs: Vec<(String, &IndicatorDef, Params)> = DEFS.iter().map(|d| (d.id.to_string(), d, Params::new()))
            .chain((1..4).map(|t| (format!("tv_auto_pitchfork_type{t}"), &DEFS[3], [("type".to_string(), t as f64)].into_iter().collect()))).collect();
        let body: Vec<String> = runs.iter().map(|(name, d, q)| {
            let out = run(d, &b, q).unwrap_or_else(|e| panic!("{name}: {e}"));
            let cols: Vec<String> = d.outputs.iter().zip(&out).map(|(k, s)| format!("\"{k}\":[{}]", s.iter().map(|&v| num(v)).collect::<Vec<_>>().join(","))).collect();
            format!("\"{name}\":{{{}}}", cols.join(","))
        }).collect();
        std::fs::write(std::env::var("TV_OUT").expect("TV_OUT"), format!("{{{}}}", body.join(","))).expect("write TV_OUT");
    }
}
