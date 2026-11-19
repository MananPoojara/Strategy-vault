//! Bill Poulos, Market Mastery Protege Program (Profits Run, Inc.), Method 4 "bearish / bullish divergence" (module 5 transcript,
//! research share Trading Courses,Seminars,Videos/Bill Poulos - Market Mastery Course (marketmastery.com)/Module 5/Method 4.pdf,
//! text layer; slide numbers cited). Book id `mmpoulos`. Stock method: runs only on actual stocks (US S&P 500 and NSE Nifty 50
//! baskets); an index or ETF cannot stand in. Lane CV5 (19 Nov 2026); independent check authors/convert-1118/I-scan/poulos/ref/mmp_ref.py.
use crate::connors::{dmi, sma, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "mmpoulos";
pub const REVIEWED: &[&str] = &["mmp4_div"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Unstated details, 4 (R4 #343)] (1) The 8% gap stand-aside (slide 9: a gap that leaves a gap above 8% of the setup-day close; the downtrend mirror, slide 6) prints no window; read as any gap in the 40 bars that make the 40-day extreme. (2) Day-10 step (slide 19, 'not hit within ten days after entry'): applied at the close of the 10th day after the entry day, effective the next bar. (3) That step is a single change of the stop to the HH4 (LL4) plus (minus) the buffer; the slide does not say whether it is re-trailed daily afterwards, so it is not (a later half-target trail HH2 / LL2 still applies and only tightens: M1 slide 19, M2 slide 18, 'we never lower stops'). (4) Inside one daily bar the stop is tested before the profit target (stop first; a fill at or beyond the stop on the entry bar is no trade). Labelled, not counted: a gap is a low above the previous high (a high below the previous low); the search criteria of slides 31-36, which spell the Point 1/2/3 picture as N-days-ago tests and include the 50-day average above its value 5 days ago, are the rule. Book-literal: limit entry at the setup-day 20-day SMA for one day only; initial stop at the HH4 (LL4) of the setup day (Point 3) plus (minus) 0.5% of that level (price >= 50) or 25 cents; half covered at the 50-day SMA of the prior close (moves daily); the rest trails the HH2 / LL2 plus / minus the buffer. ADX(14,14) is Wilder's; slow stochastic 8-3-3 is the usual built-in (fast %K over 8 bars, slow %K = 3-bar SMA) (R4 #339). US stocks use the printed price 10-200 and 50-day volume >= 500,000 screens and the wide-range screen max high(65) - min low(65) > 10 dollars or that / close > .30 (M1 line 1570, M2 slides 26 and 34, M3 slide 29: printed .30). India (Nifty 50) is a declared adaptation (R4 #340): the course says to change the price range and the volume level for local markets and gives no values, so the dollar price band and the absolute 10 are dropped, the 500,000-share volume and the .30 ratio are kept as printed, and the 50-dollar buffer switch, the 25-cent buffer and the 10 / 200 band act on the basket's rupee prices; read India results as indicative only. UNVERIFIED: the course prints chart examples, not results.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "mmp4_div", name: "Poulos Method 4: sell short a bearish divergence (buy a bullish one) at the 20-day SMA limit, stop beyond Point 3, half at the 50-day SMA, rest on a 2-bar trail", group: "Market Mastery Protege Program (Poulos)", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: true, needs_vix: false, needs_series: &[], book: BOOK, rules: &[
            "F|Bearish setup day: SMA50 today > SMA50 yesterday and 5 days ago; no gap up above 8% of the close in the last 40 days (slide 9); max high(4) = max high(40); high < max of the 3 prior highs, or (high > prior high and low < prior low and close below the middle of the range); slow %K(8,3,3) between 65 and 92; %K yesterday minus %K today >= 3; %K yesterday < %K N days ago - 4 for some N in 8..26; close > SMA20; %K < 65 N days ago for some N in 3..13; ADX(14,14) > 20 (slides 31-33). Bullish: the mirror (SMA50 falling, no gap down above 8%) with %K between 8 and 35, hooking up >= 3, %K yesterday > %K N days ago + 4, %K > 35 N days ago (slides 34-36).",
            "E|Next day only: sell short (buy) at the setup-day 20-day SMA on a limit; if not filled that day, stand aside (slide 11, slide 10 of the bullish part).",
            "X|Stop at the setup-day HH4 (LL4) = Point 3 plus (minus) 0.5% of it (price >= 50) or 25 cents; half covered at the 50-day SMA on a limit; the rest at HH2 (LL2) plus (minus) the buffer on a stop; if the target is not hit within 10 days the stop is changed (once) to the HH4 (LL4) plus (minus) the buffer (slides 13, 17-19, 12)."],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    reviewed(id).then(|| ("mmp4_div", "Poulos Method 4 divergence", "20-day limit, 50-day target"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    reviewed(id).then(|| vec![p("screen", "1 = price / volume / wide-range screens of the course (0 = off)", 1., 0., 1., 1.), p("days", "Days to the stop tightening (10)", 10., 2., 30., 1.)])
}

fn hi(v: &[f64], i: usize, n: usize) -> f64 { v[i + 1 - n..=i].iter().cloned().fold(f64::NEG_INFINITY, f64::max) }
fn lo(v: &[f64], i: usize, n: usize) -> f64 { v[i + 1 - n..=i].iter().cloned().fold(f64::INFINITY, f64::min) }
fn buf(level: f64) -> f64 { if level >= 50.0 { level * 0.005 } else { 0.25 } }

/// Slow stochastic %K (n, 3): fast %K over n bars of high / low, then its 3-bar simple average.
pub fn slow_k(x: &Ctx, n: usize) -> Vec<f64> {
    let len = x.c.len();
    let mut f = vec![f64::NAN; len];
    for i in n - 1..len {
        let (h, l) = (hi(&x.h, i, n), lo(&x.l, i, n));
        f[i] = if h > l { (x.c[i] - l) / (h - l) * 100.0 } else { 50.0 };
    }
    sma(&f, 3)
}

/// Indicators and filters shared by the methods.
struct Ind { s7: Vec<f64>, k: Vec<f64>, s20: Vec<f64>, s50: Vec<f64>, v50: Vec<f64>, adx: Vec<f64>, us: bool, screen: bool }
fn ind(x: &Ctx, screen: bool) -> Ind {
    let vol: Vec<f64> = x.r.iter().map(|r| r.volume.filter(|v| v.is_finite() && *v > 0.0).unwrap_or(f64::NAN)).collect();
    let (_, adx) = dmi(x.r, 14);
    Ind { s7: sma(&x.c, 7), k: slow_k(x, 8), s20: sma(&x.c, 20), s50: sma(&x.c, 50), v50: sma(&vol, 50), adx, us: !super::tselhs::india(x), screen }
}
/// The course's price / volume / wide-range screens (M1 slide 27, common to the four methods), on setup day t.
fn screens(x: &Ctx, n: &Ind, t: usize) -> bool {
    if !n.screen { return true; }
    let c = x.c[t];
    let rng = hi(&x.h, t, 65) - lo(&x.l, t, 65);
    n.v50[t] >= 500000.0 && (!n.us || (c >= 10.0 && c <= 200.0)) && ((n.us && rng > 10.0) || rng / c > 0.30)
}
/// Method 4 setup on day t: (direction, Point 3 level).
fn m4_setup(x: &Ctx, n: &Ind, t: usize) -> Option<(f64, f64)> {
    let (h, l, c, k) = (&x.h, &x.l, &x.c, &n.k);
    if t < 66 || !k[t].is_finite() || !k[t - 1].is_finite() || !n.adx[t].is_finite() || !n.s50[t].is_finite() || !n.s50[t - 1].is_finite() || !n.s20[t].is_finite() { return None; }
    if !screens(x, n, t) || n.adx[t] <= 20.0 { return None; }
    let outside = h[t] > h[t - 1] && l[t] < l[t - 1];
    let mid = (h[t] - l[t]) / 2.0 + l[t];
    let any = |a: usize, b: usize, f: &dyn Fn(f64) -> bool| (a..=b).any(|m| k[t - m].is_finite() && f(k[t - m]));
    let gap_up = (t - 39..=t).any(|i| l[i] > h[i - 1] && l[i] - h[i - 1] > 0.08 * c[t]);
    let gap_dn = (t - 39..=t).any(|i| h[i] < l[i - 1] && l[i - 1] - h[i] > 0.08 * c[t]);
    if n.s50[t] > n.s50[t - 1] && n.s50[t] > n.s50[t - 5] && !gap_up && hi(h, t, 4) == hi(h, t, 40)
        && (h[t] < hi(h, t - 1, 3) || (outside && c[t] < mid))
        && k[t] < 92.0 && k[t] > 65.0 && k[t - 1] - k[t] >= 3.0
        && any(8, 26, &|v| k[t - 1] < v - 4.0) && c[t] > n.s20[t] && any(3, 13, &|v| v < 65.0) { return Some((-1.0, hi(h, t, 4))); }
    if n.s50[t] < n.s50[t - 1] && n.s50[t] < n.s50[t - 5] && !gap_dn && lo(l, t, 4) == lo(l, t, 40)
        && (l[t] > lo(l, t - 1, 3) || (outside && c[t] > mid))
        && k[t] > 8.0 && k[t] < 35.0 && k[t] - k[t - 1] >= 3.0
        && any(8, 26, &|v| k[t - 1] > v + 4.0) && c[t] < n.s20[t] && any(3, 13, &|v| v > 35.0) { return Some((1.0, lo(l, t, 4))); }
    None
}

/// Method 4 walk: its trades, plus a block for every divergence setup (direction, setup day, last bar of the stand-aside: the
/// first half-position target or the exit, the next day if the order was not filled).
fn m4_walk(x: &Ctx, n: &Ind, from: usize, to: usize, days: usize) -> (Vec<Trade>, Vec<(f64, usize, usize)>) {
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let (s20, s50) = (&n.s20, &n.s50);
    let mut out: Vec<Trade> = Vec::new();
    let mut blocks = Vec::new();
    let mut t = from.max(66);
    while t < to {
        let Some((d, p3)) = m4_setup(x, n, t) else { t += 1; continue };
        let j = t + 1;
        let lim = s20[t];
        let fill = if d > 0.0 { (l[j] <= lim).then(|| o[j].min(lim)) } else { (h[j] >= lim).then(|| o[j].max(lim)) };
        let Some(e) = fill else { blocks.push((d, t, j)); t += 1; continue };
        let mut stop = if d > 0.0 { p3 - buf(p3) } else { p3 + buf(p3) };
        if d * (e - stop) <= 0.0 { blocks.push((d, t, j)); t += 1; continue; } // filled at or beyond the stop: no trade
        let mut exits: Vec<(usize, f64, f64)> = Vec::new();
        let (mut half, mut last, mut closed, mut why) = (false, j, false, "open");
        let mut end = to;
        for b in j..=to {
            last = b;
            let tgt = s50[b - 1];
            let hit = if d > 0.0 { l[b] <= stop } else { h[b] >= stop };
            if hit {
                let px = if d > 0.0 { o[b].min(stop) } else { o[b].max(stop) };
                exits.push((b, px, if half { 0.5 } else { 1.0 }));
                why = if half { "HH2/LL2 trailing stop" } else { "Stop" };
                closed = true;
                if !half { end = b; }
                break;
            }
            if !half && tgt.is_finite() {
                let reach = if d > 0.0 { h[b] >= tgt } else { l[b] <= tgt };
                if reach {
                    let px = if d > 0.0 { o[b].max(tgt) } else { o[b].min(tgt) };
                    exits.push((b, px, 0.5));
                    half = true;
                    end = b;
                }
            }
            if half {
                let tr = if d > 0.0 { lo(l, b, 2) - buf(lo(l, b, 2)) } else { hi(h, b, 2) + buf(hi(h, b, 2)) };
                stop = if d > 0.0 { stop.max(tr) } else { stop.min(tr) };
            } else if b == j + days {
                stop = if d > 0.0 { lo(l, b, 4) - buf(lo(l, b, 4)) } else { hi(h, b, 4) + buf(hi(h, b, 4)) }; // slide 19: the stop is changed to the HH4 / LL4 level
            }
        }
        blocks.push((d, t, end));
        out.push(scale_trade(d, j, e, t, last, closed, why, exits, c[last], to, c[to], "limit"));
        if !closed { break; }
        t = last;
    }
    (out, blocks)
}

#[allow(clippy::too_many_arguments)]
fn scale_trade(d: f64, j: usize, e: f64, sig: usize, last: usize, closed: bool, why: &str, exits: Vec<(usize, f64, f64)>, cl: f64, to: usize, cto: f64, kind: &'static str) -> Trade {
    let mut tr = Trade { dir: d, legs: vec![Leg { i: j, px: e, w: 1.0 }], exit_i: last, exit_px: cl, reason: why.into(), entry_kind: kind, signal_i: sig, open: !closed, exits: vec![] };
    if closed {
        let (sw, sp): (f64, f64) = exits.iter().fold((0.0, 0.0), |a, z| (a.0 + z.2, a.1 + z.1 * z.2));
        tr.exit_px = sp / sw;
        tr.signal_i = last;
        tr.exits = if exits.len() > 1 { exits } else { vec![] };
    } else {
        let done: f64 = exits.iter().map(|z| z.2).sum();
        if done > 0.0 {
            let mut ex = exits.clone();
            ex.push((to, cto, 1.0 - done));
            tr.exit_px = ex.iter().map(|z| z.1 * z.2).sum();
            tr.exits = ex;
        }
    }
    tr
}

/// Methods 1-3 position management after the fill on bar j: initial stop, half at 10%, LL4 / HH4 trail (2-bar after a gap
/// to a new 20-day extreme), stop first inside a bar, stops only tightened. Returns (exits, last bar, closed, reason).
pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let g = |k: &str, d: f64| p.get(k).copied().unwrap_or(d);
    let (screen, days) = (g("screen", 1.) > 0.5, g("days", 10.).round() as usize);
    let n = ind(x, screen);
    Some(m4_walk(x, &n, from, to, days).0)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn runs_and_orders_are_sane() {
        let r = super::super::sctraub::tests::bars(3000);
        let x = Ctx::new(&r, &r, &[]);
        let k = slow_k(&x, 8);
        assert!(k[20..].iter().all(|v| (0.0..=100.0).contains(v)));
        let t = run("mmp4_div", &x, 0, r.len() - 1, &[("screen".to_string(), 0.)].into_iter().collect()).unwrap();
        for z in &t { assert!(z.exit_i >= z.legs[0].i); }
        for w in t.windows(2) { assert!(w[1].legs[0].i > w[0].exit_i); }
    }
}
