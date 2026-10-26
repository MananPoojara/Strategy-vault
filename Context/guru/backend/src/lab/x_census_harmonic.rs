//! Census group "harmonic" (2026-10-26, research: sv-census-work/research_harmonic.md). Formulas: see each definition string.
//! One causal swing engine: confirmed pivots (price.rs::pivots, TradingView tie rules) folded into an alternating peak/valley list;
//! each time a swing is confirmed the last k swings are tested as one pattern; a match fires on that bar (or, with confirm 1, on
//! the first close beyond the completion bar). Ratios follow LuxAlgo Library / pyharmonics; the tolerance form is pyharmonics'.
//! Not built (UNCONFIRMED or no source number; nothing invented): Carney's book page numbers and his numeric tolerance (books not
//! reachable; tol 0.03 is pyharmonics'); targets / stops (0.382/0.618 AD not in the fetched sources); time symmetry for Three
//! Drives and AB=CD ("equivalent time", no number); the Three Drives "fourth push" failure rule (no level); pyharmonics'
//! any-swing (non-adjacent) point search (a search strategy, not a rule; the last k consecutive swings are used); Alternate Bat,
//! Deep Butterfly, Cypher (not in this census); the PRZ band outputs (optional in research 0.6).
use super::bars::Bars;
use super::catalogue::{int, len, n, p, real, IndicatorDef, Needs, Output, ParamSpec, Params, Placement};
use super::price::pivots;
use super::ta::NA;

const CAT: &str = "Harmonic patterns";
const fn def(id: &'static str, name: &'static str, params: &'static [ParamSpec], definition: &'static str, f: fn(&Bars, &Params) -> Output) -> IndicatorDef {
    IndicatorDef { id, name, category: CAT, params, outputs: &["signal", "level"], placement: Placement::Pane, needs: Needs::NONE, definition,
                   first: &[0, usize::MAX], sparse: true, compute: Some(f), note: "" }
}
const INF: f64 = f64::INFINITY;
type Sw = (usize, f64, bool); // (bar, price, peak)

/// |num| / |den|; NaN (no match) when den = 0.
fn q(num: f64, den: f64) -> f64 { if den == 0. { NA } else { (num / den).abs() } }
/// pyharmonics get_pattern_definition: lo·(1 − tol) ≤ r ≤ hi·(1 + tol).
fn win(r: f64, (lo, hi): (f64, f64), tol: f64) -> bool { r >= lo * (1. - tol) && r <= hi * (1. + tol) }
fn var(prm: &Params) -> usize { prm.get("variant").map_or(0, |v| *v as usize) }
fn on(prm: &Params, k: &str) -> bool { prm.get(k) == Some(&1.) }
/// Each swing is beyond the one before it and the extreme high (peak) / low (valley) of bars (previous swing, this swing].
fn legs(b: &Bars, e: &[Sw]) -> bool {
    e.windows(2).all(|w| { let ((a, av, _), (z, zv, hi)) = (w[0], w[1]);
        z > a && if hi { zv > av && b.high[a + 1..=z].iter().all(|&h| !(h > zv)) } else { zv < av && b.low[a + 1..=z].iter().all(|&l| !(l < zv)) } })
}
fn emit(o: &mut [Vec<f64>], t: usize, g: f64, lv: f64) { if o[0][t] == 0. { (o[0][t], o[1][t]) = (g, lv); } }
/// The swing engine. `det` gets the last k swing prices oriented bullish (negated when the last swing is a peak) and returns
/// Some(kill) on a match: the oriented close level that cancels a confirm-1 candidate (−INF none).
fn scan(b: &Bars, prm: &Params, k: usize, det: impl Fn(&[f64]) -> Option<f64>) -> Output {
    let (m, r, cf) = (b.len(), n(prm, "right"), on(prm, "confirm"));
    let mw = match n(prm, "max_wait") { 0 => r, w => w };
    let (ph, pl) = (pivots(&b.high, n(prm, "left"), r, true), pivots(&b.low, n(prm, "left"), r, false));
    let mut out = vec![vec![0.; m], vec![NA; m]];
    let (mut s, mut pend): (Vec<Sw>, Vec<(f64, f64, f64, f64, usize)>) = (vec![], vec![]);
    for t in 0..m {
        for (hi, v) in [(true, ph[t]), (false, pl[t])] { // a bar that is both: peak first
            if v.is_nan() { continue; }
            match s.last_mut() {
                Some(z) if z.2 == hi => { if (hi && v > z.1) || (!hi && v < z.1) { *z = (t - r, v, hi); } else { continue; } }
                _ => s.push((t - r, v, hi)),
            }
            let Some(e) = s.len().checked_sub(k).map(|i| &s[i..]) else { continue };
            if !legs(b, e) { continue; }
            let g = if hi { -1. } else { 1. };
            let Some(kill) = det(&e.iter().map(|w| g * w.1).collect::<Vec<_>>()) else { continue };
            if cf { pend.push((g, v, if hi { b.low[t - r] } else { b.high[t - r] }, g * kill, t + mw)); } else { emit(&mut out, t, g, v); }
        }
        pend.retain(|&(g, d, trig, kill, until)| {
            if t > until { return false; }
            let c = b.close[t];
            if g * (c - trig) > 0. { emit(&mut out, t, g, d); return false; }
            !(g * ((if g > 0. { b.low[t] } else { b.high[t] }) - d) < 0. || g * (c - kill) < 0.)
        });
    }
    Ok(out)
}

/// XABCD windows: xab = |A−B|/|A−X|, abc = |B−C|/|A−B|, bcd = |C−D|/|C−B|, xad = |A−D|/|A−X|, abcd = |C−D|/|A−B| (abcd = 1 only).
struct X5 { xab: (f64, f64), abc: (f64, f64), bcd: (f64, f64), xad: (f64, f64), abcd: (f64, f64) }
const ABC: (f64, f64) = (0.382, 0.886);
const NO: (f64, f64) = (0., INF);
fn xabcd(b: &Bars, prm: &Params, rs: &[X5]) -> Output {
    let (tol, ab, u) = (p(prm, "tol"), on(prm, "abcd"), &rs[var(prm).min(rs.len() - 1)]);
    scan(b, prm, 5, |v| {
        let &[x, a, bb, c, d] = v else { return None };
        (win(q(a - bb, a - x), u.xab, tol) && win(q(c - bb, a - bb), u.abc, tol) && win(q(c - d, c - bb), u.bcd, tol) && win(q(a - d, a - x), u.xad, tol)
            && (!ab || win(q(c - d, a - bb), u.abcd, tol))).then_some(-INF)
    })
}
const GART: &[X5] = &[X5 { xab: (0.618, 0.618), abc: ABC, bcd: (1.27, 1.618), xad: (0.786, 0.786), abcd: (1., 1.) },
                      X5 { xab: (0.618, 0.618), abc: ABC, bcd: (1.13, 1.618), xad: (0.786, 0.786), abcd: (1., 1.) }];
const BAT: &[X5] = &[X5 { xab: (0.382, 0.5), abc: ABC, bcd: (1.618, 2.618), xad: (0.886, 0.886), abcd: (1.27, 1.27) }];
const BFLY: &[X5] = &[X5 { xab: (0.786, 0.786), abc: ABC, bcd: (1.618, 2.618), xad: (1.27, 1.27), abcd: (1., 1.618) },
                      X5 { xab: (0.786, 0.786), abc: ABC, bcd: (1.618, 2.618), xad: (1.27, 1.618), abcd: (1., 1.618) },
                      X5 { xab: (0.786, 0.786), abc: ABC, bcd: (1.618, 2.618), xad: (1.272, 1.414), abcd: (1., 1.618) }];
const CRAB: &[X5] = &[X5 { xab: (0.382, 0.618), abc: ABC, bcd: (2.24, 3.618), xad: (1.618, 1.618), abcd: NO },
                      X5 { xab: (0.382, 0.618), abc: ABC, bcd: (2.618, 3.618), xad: (1.618, 1.618), abcd: NO }];
const DCRAB: &[X5] = &[X5 { xab: (0.886, 0.886), abc: ABC, bcd: (2.24, 3.618), xad: (1.618, 1.618), abcd: NO },
                       X5 { xab: (0.886, 0.886), abc: ABC, bcd: (2.227, 3.618), xad: (1.618, 1.618), abcd: NO }];
fn gartley(b: &Bars, prm: &Params) -> Output { xabcd(b, prm, GART) }
fn bat(b: &Bars, prm: &Params) -> Output { xabcd(b, prm, BAT) }
fn butterfly(b: &Bars, prm: &Params) -> Output { xabcd(b, prm, BFLY) }
fn crab(b: &Bars, prm: &Params) -> Output { xabcd(b, prm, CRAB) }
fn deep_crab(b: &Bars, prm: &Params) -> Output { xabcd(b, prm, DCRAB) }

fn shark(b: &Bars, prm: &Params) -> Output {
    let (tol, ph) = (p(prm, "tol"), var(prm) == 1);
    scan(b, prm, 5, |v| {
        let &[o, x, a, bb, c] = v else { return None };
        let (xa, xc) = (q(x - a, x - o), q(x - c, x - o));
        let leg = if ph { (win(xa, (0.382, 0.618), tol) && win(xc, (0.886, 0.886), tol)) || (win(xa, (0.618, 0.618), tol) && win(xc, (1.13, 1.13), tol)) }
                  else { a > o && win(xc, (0.886, 1.13), tol) };
        (leg && win(q(bb - a, x - a), (1.13, 1.618), tol) && win(q(bb - c, bb - a), (1.618, 2.24), tol)).then_some(x - 1.13 * (x - o))
    })
}
fn three_drives(b: &Bars, prm: &Params) -> Output {
    let tol = p(prm, "tol");
    let (ret, ext) = ((0.618, 0.786), (1.27, 1.618));
    scan(b, prm, 6, |v| {
        let &[x, d1, a, d2, bb, d3] = v else { return None };
        (d2 < d1 && d3 < d2 && win(q(a - d1, x - d1), ret, tol) && win(q(a - d2, a - d1), ext, tol) && win(q(bb - d2, a - d2), ret, tol)
            && win(q(bb - d3, bb - d2), ext, tol)).then_some(-INF)
    })
}
/// pyharmonics ABCD reciprocal pairs (abc, bcd).
const PAIRS: [(f64, f64); 7] = [(0.382, 2.24), (0.382, 2.618), (0.5, 2.), (0.618, 1.618), (0.707, 1.414), (0.786, 1.272), (0.886, 1.13)];
fn abcd(b: &Bars, prm: &Params) -> Output {
    let (tol, te, k, ph) = (p(prm, "tol"), p(prm, "tol_eq"), p(prm, "cd_ratio"), var(prm) == 1);
    scan(b, prm, 4, |v| {
        let &[a, bb, c, d] = v else { return None };
        let (abc, bcd) = (q(c - bb, a - bb), q(c - d, c - bb));
        (if ph { PAIRS.iter().any(|&(r, e)| win(abc, (r, r), tol) && win(bcd, (e * k, e * k), tol)) }
         else { win(abc, (0.618, 0.786), tol) && win(bcd, (1.27 * k, 1.618 * k), tol) && win(q(c - d, a - bb), (k, k), te) }).then_some(-INF)
    })
}
fn five_o(b: &Bars, prm: &Params) -> Output {
    let (tol, ab) = (p(prm, "tol"), on(prm, "abcd"));
    scan(b, prm, 6, |v| {
        let &[o, x, a, bb, c, d] = v else { return None };
        (a < o && win(q(a - bb, a - x), (1.13, 1.618), tol) && win(q(c - bb, a - bb), (1.618, 2.24), tol) && win(q(c - d, c - bb), (0.5, 0.5), tol)
            && (!ab || win(q(c - d, a - bb), (1., 1.), tol))).then_some(c - 0.618 * (c - bb))
    })
}

const L: ParamSpec = len("left", "Pivot left bars (ADAPT 10: pyharmonics peak_spacing; Carney gives no swing rule)", 10.);
const R: ParamSpec = len("right", "Pivot right bars (ADAPT 10: pyharmonics peak_spacing)", 10.);
const TOL: ParamSpec = real("tol", "Ratio tolerance: lo·(1 − tol) ≤ r ≤ hi·(1 + tol) (ADAPT 0.03: pyharmonics fib_tolerance)", 0.03, 0., 1., 0.005);
const CF: ParamSpec = int("confirm", "Event bar (ADAPT: 0 the bar the last point is confirmed, 1 the first close beyond that point's bar high (bullish) / low (bearish))", 0., 0., 1.);
const MW: ParamSpec = int("max_wait", "confirm 1: bars after detection before the candidate expires (ADAPT; 0 = right)", 0., 0., 5000.);
const AB: ParamSpec = int("abcd", "Also require Carney's AB=CD leg (0 off: LuxAlgo, pyharmonics; 1 on: Harmonic Trader)", 0., 0., 1.);
const BASE: [ParamSpec; 5] = [L, R, TOL, CF, MW];
const GART_P: &[ParamSpec] = &[L, R, TOL, CF, MW, AB, int("variant", "BC projection (0 Harmonic Trader / pyharmonics 1.27–1.618, 1 LuxAlgo 1.13–1.618)", 0., 0., 1.)];
const BAT_P: &[ParamSpec] = &[L, R, TOL, CF, MW, AB];
const BFLY_P: &[ParamSpec] = &[L, R, TOL, CF, MW, AB, int("variant", "XA projection of D (0 Harmonic Trader 1.27, 1 LuxAlgo 1.27–1.618, 2 pyharmonics 1.272–1.414)", 0., 0., 2.)];
const CRAB_P: &[ParamSpec] = &[L, R, TOL, CF, MW, int("variant", "BC projection (0 Harmonic Trader 2.24–3.618, 1 LuxAlgo / pyharmonics 2.618–3.618)", 0., 0., 1.)];
const DCRAB_P: &[ParamSpec] = &[L, R, TOL, CF, MW, int("variant", "BC projection (0 LuxAlgo 2.24–3.618, 1 pyharmonics 2.227–3.618)", 0., 0., 1.)];
const SHARK_P: &[ParamSpec] = &[L, R, TOL, CF, MW, int("variant", "Rules (0 Carney 88.6%/113% / LuxAlgo: A between 0 and X, C at 0.886–1.13 of 0X; 1 pyharmonics Shark: A at 0.382–0.618 of 0X and C at 0.886, or Deep Shark: A at 0.618 and C at 1.13)", 0., 0., 1.)];
const ABCD_P: &[ParamSpec] = &[L, R, TOL, CF, MW, int("variant", "Rules (0 Harmonic Trader: C at 0.618–0.786 of AB, BC projection 1.27–1.618, CD = AB; 1 pyharmonics reciprocal pairs)", 0., 0., 1.),
    real("cd_ratio", "CD/AB (Harmonic Trader 1.0 = AB=CD; LuxAlgo / pyharmonics alternate 1.27 or 1.618)", 1., 0.1, 5., 0.001),
    real("tol_eq", "CD/AB equality tolerance, variant 0 (ADAPT 0.03 = tol)", 0.03, 0., 1., 0.005)];
const FIVE_P: &[ParamSpec] = &[L, R, TOL, CF, MW, int("abcd", "Also require the reciprocal AB=CD, CD/AB = 1 (0 off: LuxAlgo; 1 on: LuxAlgo / Carney Vol. Two \"CD equal in AB rib length\")", 0., 0., 1.)];

macro_rules! sw { () => { "Swings: ta.pivothigh(high)/ta.pivotlow(low) with left/right bars (default 10/10: ADAPT, pyharmonics peak_spacing = 10; Carney gives no swing detector), each known right bars after it, folded into an alternating peak/valley list (a pivot of the same kind as the last swing replaces it only when strictly more extreme). Each time a swing is confirmed the last k consecutive swings are the pattern points (ADAPT; pyharmonics also searches non-adjacent swings); each point must be beyond the point before it and the extreme high (peak) / low (valley) of the bars after that point up to and including its own bar. Ratios r = |leg| / |leg| of pivot prices pass when lo·(1 − tol) ≤ r ≤ hi·(1 + tol), a single number being lo = hi (pyharmonics get_pattern_definition; tol 0.03 = pyharmonics fib_tolerance, ADAPT: Carney's public text gives none); a zero denominator never passes. Bullish = the last point is a valley (+1, a buy at D), bearish = the mirror (−1). confirm 0: the event is the bar the last point is confirmed (its pivot bar + right; no lookahead, never earlier, never retracted; a later more extreme last point is a new candidate). confirm 1 (ADAPT, Carney gives no trigger bar): the first bar from that bar on whose close is strictly beyond the last point's bar high (bullish) / low (bearish); dropped first when a low below (bullish) / high above (bearish) the last point, a close beyond the pattern's invalidation level, or more than max_wait (0 = right) bars after detection. One event per bar (the first wins). Outputs: signal ±1 on the event bar, else 0; level = the last point's price on the event bar, else NaN. Targets and stops are not built (not in the fetched sources). " } }

pub const DEFS: &[IndicatorDef] = &[
    def("harmonic_gartley", "Harmonic Pattern - Gartley", GART_P, concat!(sw!(), "Gartley (Carney, The Harmonic Trader; LuxAlgo Library; pyharmonics gartley): k = 5 swings X, A, B, C, D (bullish X valley, A peak, B valley, C peak, D valley): xab = |A−B|/|A−X| = 0.618, abc = |B−C|/|A−B| in 0.382–0.886, bcd = |C−D|/|C−B| in 1.27–1.618 (variant 1 LuxAlgo 1.13–1.618), xad = |A−D|/|A−X| = 0.786; abcd 1: also |C−D|/|A−B| = 1.0 (Carney \"should possess a distinct AB=CD\")"), gartley),
    def("harmonic_bat", "Harmonic Pattern - Bat", BAT_P, concat!(sw!(), "Bat (Carney, The Harmonic Trader; LuxAlgo Library; pyharmonics bat): k = 5 swings X, A, B, C, D: xab = |A−B|/|A−X| in 0.382–0.50, abc = |B−C|/|A−B| in 0.382–0.886, bcd = |C−D|/|C−B| in 1.618–2.618, xad = |A−D|/|A−X| = 0.886; abcd 1: also |C−D|/|A−B| = 1.27 (Carney \"usually requires a 1.27AB=CD\")"), bat),
    def("harmonic_butterfly", "Harmonic Pattern - Butterfly", BFLY_P, concat!(sw!(), "Butterfly (Carney, The Harmonic Trader; LuxAlgo Library; pyharmonics butterfly): k = 5 swings X, A, B, C, D: xab = |A−B|/|A−X| = 0.786, abc = |B−C|/|A−B| in 0.382–0.886, bcd = |C−D|/|C−B| in 1.618–2.618, xad = |A−D|/|A−X| = 1.27 (variant 1 LuxAlgo 1.27–1.618, 2 pyharmonics 1.272–1.414); abcd 1: also |C−D|/|A−B| in 1.0–1.618 (Carney \"must include an AB=CD pattern\", frequently 1.27 or 1.618)"), butterfly),
    def("harmonic_crab", "Harmonic Pattern - Crab", CRAB_P, concat!(sw!(), "Crab (Carney, The Harmonic Trader; LuxAlgo Library; pyharmonics crab): k = 5 swings X, A, B, C, D: xab = |A−B|/|A−X| in 0.382–0.618, abc = |B−C|/|A−B| in 0.382–0.886, bcd = |C−D|/|C−B| in 2.24–3.618 (Carney \"2.24, 2.618, 3.14, 3.618\"; variant 1 LuxAlgo / pyharmonics 2.618–3.618), xad = |A−D|/|A−X| = 1.618"), crab),
    def("harmonic_deep_crab", "Harmonic Pattern - Deep Crab", DCRAB_P, concat!(sw!(), "Deep Crab (LuxAlgo Library; pyharmonics deep crab; Carney's text not reached): k = 5 swings X, A, B, C, D: xab = |A−B|/|A−X| = 0.886, abc = |B−C|/|A−B| in 0.382–0.886 (pyharmonics), bcd = |C−D|/|C−B| in 2.24–3.618 (variant 1 pyharmonics 2.227–3.618), xad = |A−D|/|A−X| = 1.618"), deep_crab),
    def("harmonic_shark", "Harmonic Pattern - Shark", SHARK_P, concat!(sw!(), "Shark (Carney, Harmonic Trading Vol. 3 \"88.6% retracement and the 113% Reciprocal Ratio\"; LuxAlgo Library; pyharmonics shark / deep shark): k = 5 swings 0, X, A, B, C (bullish 0 valley, X peak, A valley, B peak, C valley; C is the completion point): |B−A|/|X−A| in 1.13–1.618 (B beyond X), |B−C|/|B−A| in 1.618–2.24; variant 0: A between 0 and X (ADAPT, no ratio in LuxAlgo) and |X−C|/|X−0| in 0.886–1.13; variant 1 (pyharmonics): |X−A|/|X−0| in 0.382–0.618 with |X−C|/|X−0| = 0.886, or |X−A|/|X−0| = 0.618 with |X−C|/|X−0| = 1.13. Invalidation (LuxAlgo, confirm 1): a close beyond the 1.13 extension of 0X, X − 1.13·(X − 0)"), shark),
    def("harmonic_three_drives", "Harmonic Pattern - Three Drives", &BASE, concat!(sw!(), "Three Drives (Carney, The Harmonic Trader; LuxAlgo Library; TradingView Three Drives scripts): k = 6 swings X, D1, A, D2, B, D3 (bullish: X peak, D1 valley, A peak, D2 valley, B peak, D3 valley; three drives down): D2 < D1, D3 < D2; drives |A−D2|/|A−D1| and |B−D3|/|B−D2| in 1.27–1.618 (Carney \"each drive completes at either a 1.27 or a 1.618\"); corrections |A−D1|/|X−D1| and |B−D2|/|A−D2| in 0.618–0.786 (TradingView / LuxAlgo; not in Carney's text). Carney's \"equivalent time periods\" has no number and is not built"), three_drives),
    def("harmonic_abcd", "Harmonic Pattern - AB=CD", ABCD_P, concat!(sw!(), "AB=CD (Carney, The Harmonic Trader; LuxAlgo Library; pyharmonics ABCD): k = 4 swings A, B, C, D (bullish A peak, B valley, C peak, D valley): variant 0 (Carney): abc = |B−C|/|A−B| in 0.618–0.786, bcd = |C−D|/|C−B| in 1.27·cd_ratio–1.618·cd_ratio (CD/AB = abc × bcd), |C−D|/|A−B| within tol_eq of cd_ratio (1.0 = \"each price leg is equivalent\"; alternate AB=CD 1.27 or 1.618, LuxAlgo / pyharmonics); variant 1 (pyharmonics): abc and bcd/cd_ratio each within tol of one reciprocal pair 0.382/2.24, 0.382/2.618, 0.5/2.0, 0.618/1.618, 0.707/1.414, 0.786/1.272, 0.886/1.13. Time symmetry (\"often preferred\", no number) is not built"), abcd),
    def("harmonic_5_0", "Harmonic Pattern - 5-0", FIVE_P, concat!(sw!(), "5-0 (Carney, Harmonic Trading Vol. Two, as summarised by LuxAlgo Library and TradingView; not in pyharmonics): k = 6 swings 0, X, A, B, C, D (bullish 0 peak, X valley, A peak, B valley, C peak, D valley): A does not break 0 (bullish A < 0); |A−B|/|A−X| in 1.13–1.618 (B beyond X); |C−B|/|A−B| in 1.618–2.24; |C−D|/|C−B| = 0.50; abcd 1: also |C−D|/|A−B| = 1.0. Invalidation (LuxAlgo, confirm 1): a close through the 0.618 retracement of BC, C − 0.618·(C − B)"), five_o),
];

pub const SOURCES: &[(&str, &str, &str)] = &[
    ("harmonic_gartley", "Harmonic Trader (Carney): Gartley; LuxAlgo Library: Gartley Pattern; pyharmonics: gartley; TradingView drawing tool: XABCD Pattern", ""),
    ("harmonic_bat", "Harmonic Trader (Carney): Bat; LuxAlgo Library: Bat Pattern; pyharmonics: bat; TradingView drawing tool: XABCD Pattern", ""),
    ("harmonic_butterfly", "Harmonic Trader (Carney): Butterfly; LuxAlgo Library: Butterfly Pattern; pyharmonics: butterfly; TradingView drawing tool: XABCD Pattern", ""),
    ("harmonic_crab", "Harmonic Trader (Carney): Crab; LuxAlgo Library: Crab Pattern; pyharmonics: crab; TradingView drawing tool: XABCD Pattern", ""),
    ("harmonic_deep_crab", "LuxAlgo Library: Crab Pattern (Deep Crab); pyharmonics: deep crab", ""),
    ("harmonic_shark", "Harmonic Trader (Carney): Shark; LuxAlgo Library: Shark Pattern; pyharmonics: shark, deep shark", ""),
    ("harmonic_three_drives", "Harmonic Trader (Carney): Three Drives; LuxAlgo Library: Three Drives Pattern; TradingView: Three Drives Pattern drawing tool, Three Drive Pattern Detector (LuxAlgo)", ""),
    ("harmonic_abcd", "Harmonic Trader (Carney): AB=CD; LuxAlgo Library: ABCD Pattern; pyharmonics: ABCD; TradingView drawing tool: ABCD Pattern", ""),
    ("harmonic_5_0", "Carney, Harmonic Trading Vol. Two: 5-0; LuxAlgo Library: 5-0 Pattern", ""),
];

#[cfg(test)]
mod tests {
    use crate::lab::testdata::{assert_series, closes, compiles, from_ohlc, go};
    const N: f64 = f64::NAN;
    const LR1: [(&str, f64); 2] = [("left", 1.), ("right", 1.)];
    fn with<'a>(kv: &[(&'a str, f64)]) -> Vec<(&'a str, f64)> { [&LR1[..], kv].concat() }
    fn one(o: &[Vec<f64>], at: usize, sg: f64, level: f64) {
        let mut want = vec![vec![0.; o[0].len()], vec![N; o[0].len()]];
        (want[0][at], want[1][at]) = (sg, level);
        for k in 0..2 { assert_series(&o[k], &want[k]); }
    }
    fn none(o: &[Vec<f64>]) { assert!(o[0].iter().all(|v| *v == 0.) && o[1].iter().all(|v| v.is_nan())); }
    /// Bullish on closes and bearish on the mirror (1000 − close).
    fn both(id: &str, c: &[f64], kv: &[(&str, f64)], at: usize, d: f64) {
        one(&go(id, &closes(c), &with(kv)), at, 1., d);
        one(&go(id, &closes(&c.iter().map(|x| 1000. - x).collect::<Vec<_>>()), &with(kv)), at, -1., 1000. - d);
    }
    fn no(id: &str, c: &[f64], kv: &[(&str, f64)]) { none(&go(id, &closes(c), &with(kv))); }

    #[test] fn gartley_ratios_variant_and_ties() {
        // X 100, A 200, B 138.2 (xab .618), C 176.4 (abc .618), D 121.4 (xad .786, bcd 55/38.2 = 1.44), D confirmed on bar 6
        let c = [110., 100., 200., 138.2, 176.4, 121.4, 130.];
        both("harmonic_gartley", &c, &[], 6, 121.4);
        no("harmonic_gartley", &[110., 100., 200., 145., 176.4, 121.4, 130.], &[]); // xab .55 < .618·.97
        no("harmonic_gartley", &c, &[("abcd", 1.)]); // CD/AB = 55/61.8 = .89 < .97
        // equal low right of D cancels that pivot; the tie bar becomes D (left ties allowed) and fires a bar later
        one(&go("harmonic_gartley", &closes(&[110., 100., 200., 138.2, 176.4, 121.4, 121.4, 130.]), &with(&[])), 7, 1., 121.4);
        // bcd 70.7/57.9 = 1.221: below 1.27·.97, inside LuxAlgo 1.13·.97 (xab .635, abc .912, xad .763 at the tolerance edges)
        let e = [110., 100., 200., 136.5, 194.4, 123.7, 130.];
        no("harmonic_gartley", &e, &[]);
        both("harmonic_gartley", &e, &[("variant", 1.)], 6, 123.7);
    }
    #[test] fn bat_butterfly_crab_deep_crab() {
        both("harmonic_bat", &[110., 100., 200., 150., 180., 111.4, 120.], &[], 6, 111.4); // xab .5, abc .6, bcd 68.6/30, xad .886
        let f = [110., 100., 200., 121.4, 171.4, 73., 80.]; // xab .786, abc 50/78.6, bcd 98.4/50, xad 1.27
        both("harmonic_butterfly", &f, &[], 6, 73.);
        let g = [110., 100., 200., 121.4, 171.4, 50., 60.]; // xad 1.5, bcd 2.428
        no("harmonic_butterfly", &g, &[]);
        both("harmonic_butterfly", &g, &[("variant", 1.)], 6, 50.);
        no("harmonic_butterfly", &g, &[("variant", 2.)]); // 1.5 > 1.414·1.03
        both("harmonic_crab", &[110., 100., 200., 138.2, 178.2, 38.2, 50.], &[], 6, 38.2); // xab .618, abc .647, bcd 3.5, xad 1.618
        both("harmonic_deep_crab", &[110., 100., 200., 111.4, 151.4, 38.2, 50.], &[], 6, 38.2); // xab .886, abc .4515, bcd 2.83
        no("harmonic_crab", &[110., 100., 200., 111.4, 151.4, 38.2, 50.], &[]); // xab .886 is outside the Crab's
    }
    #[test] fn shark_three_drives_abcd_five_o() {
        // 0 100, X 200, A 150, B 220 (ext 1.4), C 100 (0X 1.0, BC/AB 120/70)
        let s = [110., 100., 200., 150., 220., 100., 105.];
        both("harmonic_shark", &s, &[], 6, 100.);
        no("harmonic_shark", &s, &[("variant", 1.)]); // pyharmonics: C at 1.0 is neither 0.886 nor 1.13
        // X 200, D1 100, A 170 (ret .7), D2 72 (ext 1.4), B 140.6 (ret .7), D3 44.56 (ext 1.4)
        let t = [190., 200., 100., 170., 72., 140.6, 44.56, 50.];
        both("harmonic_three_drives", &t, &[], 7, 44.56);
        no("harmonic_three_drives", &[190., 200., 100., 170., 72., 140.6, 60., 70.], &[]); // ext3 1.17
        // A 200, B 100, C 161.8 (abc .618), D 61.8 (bcd 1.618, CD = AB)
        both("harmonic_abcd", &[190., 200., 100., 161.8, 61.8, 70.], &[], 5, 61.8);
        let h = [190., 200., 100., 150., 50., 60.]; // abc .5, bcd 2.0: only the pyharmonics pair
        no("harmonic_abcd", &h, &[]);
        both("harmonic_abcd", &h, &[("variant", 1.)], 5, 50.);
        let x = [190., 200., 100., 161.8, 34.8, 40.]; // CD = 127 = 1.27·AB, bcd 2.055
        no("harmonic_abcd", &x, &[]);
        both("harmonic_abcd", &x, &[("cd_ratio", 1.27)], 5, 34.8);
        // 0 200, X 100, A 180, B 76 (ext 1.3), C 284 (BC/AB 2.0), D 180 (bcd .5, CD = AB)
        let f = [190., 200., 100., 180., 76., 284., 180., 190.];
        both("harmonic_5_0", &f, &[("abcd", 1.)], 7, 180.);
        // A 210, C 344, D 210 (ext 1.218, BC/AB 2.0, bcd .5): passes with 0 at 220, not with 0 at 200 (A breaks 0)
        both("harmonic_5_0", &[190., 220., 100., 210., 76., 344., 210., 220.], &[], 7, 210.);
        no("harmonic_5_0", &[190., 200., 100., 210., 76., 344., 210., 220.], &[]);
    }
    #[test] fn confirm_mode_waits_for_the_close_beyond_d() {
        let row = |c: f64| (c, c, c, c);
        // D bar: high 125, low 121.4; bars 6, 7 close below 125 (lows above D); bar 8 closes 126
        let mut r = vec![row(110.), row(100.), row(200.), row(138.2), row(176.4), (123., 125., 121.4, 123.), (123., 124., 122., 123.5), (123.5, 124.5, 122.5, 124.), (124., 126., 123., 126.)];
        let b = from_ohlc(&r);
        one(&go("harmonic_gartley", &b, &with(&[])), 6, 1., 121.4);
        none(&go("harmonic_gartley", &b, &with(&[("confirm", 1.)]))); // expires after right = 1 bar
        one(&go("harmonic_gartley", &b, &with(&[("confirm", 1.), ("max_wait", 2.)])), 8, 1., 121.4);
        (r[7].2, r[8].2) = (121., 120.9); // a low below D drops it (bar 8 lower still, so bar 7 is no new D)
        none(&go("harmonic_gartley", &from_ohlc(&r), &with(&[("confirm", 1.), ("max_wait", 2.)])));
    }
    #[test] fn every_output_compiles() {
        for id in ["harmonic_gartley", "harmonic_bat", "harmonic_butterfly", "harmonic_crab", "harmonic_deep_crab", "harmonic_shark", "harmonic_three_drives", "harmonic_abcd", "harmonic_5_0"] {
            for o in ["signal", "level"] { compiles(id, o).unwrap(); }
        }
    }
}
