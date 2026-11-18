//! Bill Poulos, Market Mastery Protege Program (Profits Run, Inc.), Method 4 "bearish / bullish divergence" (module 5 transcript,
//! research share Trading Courses,Seminars,Videos/Bill Poulos - Market Mastery Course (marketmastery.com)/Module 5/Method 4.pdf,
//! text layer; slide numbers cited). Book id `mmpoulos`. Stock method: runs only on actual stocks (US S&P 500 and NSE Nifty 50
//! baskets); an index or ETF cannot stand in. Lane CV5 (19 Nov 2026); independent check authors/convert-1118/I-scan/poulos/ref/mmp_ref.py.
use crate::connors::{dmi, sma, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "mmpoulos";
pub const REVIEWED: &[&str] = &["mmp4_div", "mmp3_flag", "mmp2_retrace"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings, 3] (1) The search criteria of slides 31-36 are the rule (they spell out the swing-point setup as N-days-ago tests; the Point 1/2/3 pictures of slides 6-9 are what they encode); the 8% gap stand-aside printed on slide 9 is not part of those criteria and is not applied. Screens: the M1/M2 search-criteria text is 'common to all four methods' and is applied as printed (max high(65) - min low(65) > 10 dollars, or that difference / close > .30, M2 slide 26). (2) Day-10 step: when the half-position target has not been hit by the close of the 10th day after the entry day, the stop becomes the HH4 (LL4) plus (minus) the buffer for the next bar, never loosened. (3) Inside one daily bar the stop is tested before the profit target (stop first; a fill at or beyond the stop on the entry bar is no trade). Book-literal: limit entry at the setup-day 20-day SMA for one day only; initial stop at the HH4 / LL4 of the setup day (Point 3) plus / minus 0.5% of that level (price >= 50) or 25 cents; half covered at the 50-day SMA of the prior close (moves daily); the rest trails the HH2 / LL2 plus / minus the buffer, only tightened. ADX(14,14) is Wilder's; slow stochastic 8-3-3 is the usual built-in (fast %K over 8 bars, slow %K = 3-bar SMA) (R4 #339). US stocks use the printed price 10-200 and 50-day volume >= 500,000 screens; the India run (declared adaptation, R4 #340) drops the dollar price test and the absolute 10 dollars (the course says to change the price range for rupee prices and gives none) and keeps the .30 range test and the 500,000-share volume. UNVERIFIED: the course prints chart examples, not results.";

const NOTE3: &str = "[Declared readings, 4] (1) Stand-aside: Method 3 says to stand aside while a bearish (for longs) / bullish (for shorts) divergence is pending 'until the divergence profit target is hit, or it is stopped out 10 days after entry'; read as: from a Method 4 divergence setup day (the card's own Method 4 rule, run on the same stock with its own position rule) until that hypothetical trade's first half-position target or exit, or the next day if its order never fills. (2) The initial stop is the LL4 (HH4) of the setup day minus (plus) the buffer, known before the entry as the course says ('known precisely prior to entering'); the slide examples count the four days to the entry day and cannot be told apart from the setup-day count at the printed prices. (3) Short entries are the printed stop-limit: trigger at the LL4 minus the buffer, limit 0.02 below it (slide 7: 61.64 stop, 61.62 limit); filled at the trigger when the bar opens above it and falls to it, at the open when that is between limit and trigger, otherwise no fill. (4) Inside one daily bar the stop is tested before the 10% target (stop first). Book-literal: flag / pennant setup (50- and 20-day SMAs rising, lowest low of 5 days above the 20-day SMA of that day, every high of the last 4 days below the prior 4-day high, some low of the last 6 days below the prior LL4, ADX(14,14) > 20, 11-day high = 30-day high; the short side is the printed mirror with the 22-day / 30-day low as printed); buy stop at the setup-day HH4 plus 0.5% (price >= 50) or 25 cents, re-set each day while the setup holds; half sold at 10%; the rest on the LL4 trail (LL2 after a gap of more than 0.5% / 25 cents to a new 20-day high), only tightened. Screens, ADX and stochastic as Method 4. UNVERIFIED: chart examples only.";

const NOTE2: &str = "[Declared readings, 4] (1) Stand-aside, as Method 3: from a Method 4 divergence setup day (the card's own Method 4 rule, same stock, own position rule) until that hypothetical trade's first half-position target or exit, or the next day if its order never fills. (2) The close-in-range alternative to %K is the printed arithmetic of the setup slides (long: close < max high(20) - 0.6 x (max high(20) - min low(10)); short: close > min low(20) + 0.6 x (max high(10) - min low(20))); the search-criteria slide 27 calls the long one 'the upper 40%', which contradicts that arithmetic. (3) The initial stop is the setup-day LL4 (HH4) minus (plus) the buffer, 'known precisely prior to entering'. (4) Inside one daily bar the stop is tested before the 10% target (stop first). Book-literal: retrace then confirm: 50-day SMA rising (falling) against yesterday; lowest low of 5 days below the 20-day SMA of that day (short: highest high of 5 days above it); close back through the 7-day SMA and beyond yesterday's high (low) - or the last non-inside day's when yesterday is an inside day; %K(8,3,3) <= 40 (>= 60) or the range arithmetic; ADX(14,14) > 20; buy (sell short) next day at the setup-day close on a limit, one day only; half at 10%, the rest on the LL4 / HH4 trail with the LL2 / HH2 switch after a gap to a new 20-day extreme, never loosened. The search slide also codes the 50-day SMA test as above both yesterday and 5 days ago; the spoken rule (yesterday) is used. Screens, ADX and stochastic as Method 4 (US 10-200 dollars, 500,000 shares, wide range). UNVERIFIED: chart examples only.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "mmp4_div", name: "Poulos Method 4: sell short a bearish divergence (buy a bullish one) at the 20-day SMA limit, stop beyond Point 3, half at the 50-day SMA, rest on a 2-bar trail", group: "Market Mastery Protege Program (Poulos)", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: true, needs_vix: false, needs_series: &[], book: BOOK, rules: &[
            "F|Bearish setup day: SMA50 today > SMA50 yesterday; max high(4) = max high(40); high < max of the 3 prior highs, or (high > prior high and low < prior low and close below the middle of the range); slow %K(8,3,3) between 65 and 92; %K yesterday minus %K today >= 3; %K yesterday < %K N days ago - 4 for some N in 8..26; close > SMA20; %K < 65 N days ago for some N in 3..13; ADX(14,14) > 20 (slides 31-33). Bullish: the mirror with %K between 8 and 35, hooking up >= 3, %K yesterday > %K N days ago + 4, %K > 35 N days ago (slides 34-36).",
            "E|Next day only: sell short (buy) at the setup-day 20-day SMA on a limit; if not filled that day, stand aside (slide 11, slide 10 of the bullish part).",
            "X|Stop at the setup-day HH4 (LL4) = Point 3 plus (minus) 0.5% of it (price >= 50) or 25 cents; half covered at the 50-day SMA on a limit; the rest at HH2 (LL2) plus (minus) the buffer on a stop; if the target is not hit within 10 days the stop becomes the HH4 (LL4) plus (minus) the buffer (slides 13, 17-19, 12)."],
        notes: NOTE, claim: None },
    StrategyDef { id: "mmp3_flag", name: "Poulos Method 3: buy (sell short) a break of the 4-day high (low) after a flag in a strong trend, 10% on half, LL4 trail", group: "Market Mastery Protege Program (Poulos)", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: true, needs_vix: false, needs_series: &[], book: BOOK, rules: &[
            "F|Uptrend setup day: SMA50 and SMA20 rising; the lowest low of the last 5 days above the SMA20 of that day; every high of the last 4 days below the highest high of the 4 days before it; some low of the last 6 days below the LL4 before it; ADX(14,14) > 20; max high(11) = max high(30); stand aside during a pending bearish divergence (slides 6-7). Downtrend: the mirror, with the lowest low of 22 days = that of 30 days (Method 3 downtrend slides 3-4).",
            "E|Next day buy stop at the setup-day HH4 plus 0.5% of it (price >= 50) or 25 cents; sell short on a stop-limit at the LL4 minus the buffer; the order is set again each day the setup holds (slides 9-11; downtrend slides 6-9).",
            "X|Initial stop at the LL4 (HH4) minus (plus) the buffer; half sold (covered) at 1.10 (0.90) x the entry on a limit; the rest at the LL4 (HH4) trail, LL2 (HH2) after a gap beyond 0.5% / 25 cents to a new 20-day high, never loosened (slides 12, 16-18; downtrend 10, 14)."],
        notes: NOTE3, claim: None },
    StrategyDef { id: "mmp2_retrace", name: "Poulos Method 2: buy (sell short) the close back through the 7-day SMA after a retrace, limit at that close, 10% on half, LL4 trail", group: "Market Mastery Protege Program (Poulos)", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: true, needs_vix: false, needs_series: &[], book: BOOK, rules: &[
            "F|Uptrend setup day: SMA50 rising; the lowest low of the last 5 days below the SMA20 of that day; yesterday's close below and today's close above the 7-day SMA and above yesterday's high (the last non-inside day's high if yesterday is an inside day); %K(8,3,3) <= 40 or the close in the lower part of the 20-day range by the printed arithmetic; ADX(14,14) > 20; stand aside during a pending bearish divergence (M2 slides 6-7). Downtrend: the mirror (slides 3-4 of the downtrend part).",
            "E|Next day only: buy (sell short) at the setup-day close on a limit (slide 9; downtrend slide 7).",
            "X|Initial stop at the LL4 (HH4) minus (plus) 0.5% (price >= 50) or 25 cents; half sold (covered) at 1.10 (0.90) x the entry on a limit; the rest on the LL4 (HH4) trail with the LL2 (HH2) switch after a gap beyond 0.5% / 25 cents to a new 20-day extreme, never loosened (slides 11, 15-17; downtrend 9, 13)."],
        notes: NOTE2, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    reviewed(id).then(|| if id == "mmp4_div" { ("mmp4_div", "Poulos Method 4 divergence", "20-day limit, 50-day target") } else if id == "mmp3_flag" { ("mmp3_flag", "Poulos Method 3 flag", "4-day break, 10% half") } else { ("mmp2_retrace", "Poulos Method 2 retrace", "7-day SMA close, 10% half") })
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
    if n.s50[t] > n.s50[t - 1] && hi(h, t, 4) == hi(h, t, 40)
        && (h[t] < hi(h, t - 1, 3) || (outside && c[t] < mid))
        && k[t] < 92.0 && k[t] > 65.0 && k[t - 1] - k[t] >= 3.0
        && any(8, 26, &|v| k[t - 1] < v - 4.0) && c[t] > n.s20[t] && any(3, 13, &|v| v < 65.0) { return Some((-1.0, hi(h, t, 4))); }
    if n.s50[t] < n.s50[t - 1] && lo(l, t, 4) == lo(l, t, 40)
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
                let tr = if d > 0.0 { lo(l, b, 4) - buf(lo(l, b, 4)) } else { hi(h, b, 4) + buf(hi(h, b, 4)) };
                stop = if d > 0.0 { stop.max(tr) } else { stop.min(tr) };
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
fn manage_trend(x: &Ctx, d: f64, j: usize, e: f64, stop0: f64, to: usize) -> (Vec<(usize, f64, f64)>, usize, bool, &'static str) {
    let (o, h, l) = (&x.o, &x.h, &x.l);
    let (mut stop, mut half, mut two) = (stop0, false, false);
    let mut exits = Vec::new();
    let tgt = e * (1.0 + d * 0.10);
    let mut last = j;
    for b in j..=to {
        last = b;
        let hit = if d > 0.0 { l[b] <= stop } else { h[b] >= stop };
        if hit {
            let px = if d > 0.0 { o[b].min(stop) } else { o[b].max(stop) };
            exits.push((b, px, if half { 0.5 } else { 1.0 }));
            return (exits, b, true, if half { "Trailing stop (rest)" } else if two || stop != stop0 { "Trailing stop" } else { "Initial stop" });
        }
        if !half && (if d > 0.0 { h[b] >= tgt } else { l[b] <= tgt }) {
            exits.push((b, if d > 0.0 { o[b].max(tgt) } else { o[b].min(tgt) }, 0.5));
            half = true;
        }
        if b >= 1 {
            let gap = if d > 0.0 { l[b] - h[b - 1] } else { l[b - 1] - h[b] };
            let ref_px = if d > 0.0 { h[b - 1] } else { l[b - 1] };
            let thr = if ref_px >= 50.0 { 0.005 * ref_px } else { 0.25 };
            let new_ext = if d > 0.0 { h[b] >= hi(h, b, 20) } else { l[b] <= lo(l, b, 20) };
            if gap > thr && new_ext { two = true; }
        }
        let nn = if two { 2 } else { 4 };
        let lv = if d > 0.0 { lo(l, b, nn) } else { hi(h, b, nn) };
        let tr = if d > 0.0 { lv - buf(lv) } else { lv + buf(lv) };
        stop = if d > 0.0 { stop.max(tr) } else { stop.min(tr) };
    }
    (exits, last, false, "open")
}

/// Method 3 setup on day t (flag / pennant in a strong trend): Some(direction).
fn m3_setup(x: &Ctx, n: &Ind, t: usize) -> Option<f64> {
    let (h, l, c) = (&x.h, &x.l, &x.c);
    if t < 66 || !n.adx[t].is_finite() || !n.s50[t].is_finite() || !n.s50[t - 1].is_finite() || !n.s20[t].is_finite() || !n.s20[t - 1].is_finite() || !screens(x, n, t) || n.adx[t] <= 20.0 { return None; }
    let _ = c;
    // uptrend
    if n.s50[t] > n.s50[t - 1] && n.s20[t] > n.s20[t - 1] {
        let im = (t - 4..=t).min_by(|a, b| l[*a].partial_cmp(&l[*b]).unwrap().then(b.cmp(a))).unwrap();
        if l[im] > n.s20[im] && (t - 3..=t).all(|i| h[i] < hi(h, i - 1, 4)) && (t - 5..=t).any(|i| l[i] < lo(l, i - 1, 4)) && hi(h, t, 11) == hi(h, t, 30) { return Some(1.0); }
    }
    if n.s50[t] < n.s50[t - 1] && n.s20[t] < n.s20[t - 1] {
        let im = (t - 4..=t).max_by(|a, b| h[*a].partial_cmp(&h[*b]).unwrap().then(a.cmp(b))).unwrap();
        if h[im] < n.s20[im] && (t - 3..=t).all(|i| l[i] > lo(l, i - 1, 4)) && (t - 5..=t).any(|i| h[i] > hi(h, i - 1, 4)) && lo(l, t, 22) == lo(l, t, 30) { return Some(-1.0); }
    }
    None
}
fn blocked(blocks: &[(f64, usize, usize)], d: f64, t: usize) -> bool { blocks.iter().any(|b| b.0 == -d && b.1 <= t && t <= b.2) }

fn run_m3(x: &Ctx, n: &Ind, from: usize, to: usize, days: usize) -> Vec<Trade> {
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let (_, blocks) = m4_walk(x, n, 0, x.r.len() - 1, days);
    let mut out = Vec::new();
    let mut t = from.max(66);
    while t < to {
        let Some(d) = m3_setup(x, n, t) else { t += 1; continue };
        if blocked(&blocks, d, t) { t += 1; continue; }
        let j = t + 1;
        let (lvl, lvl_stop) = if d > 0.0 { let v = hi(h, t, 4); (v, v + buf(v)) } else { let v = lo(l, t, 4); (v, v - buf(v)) };
        let _ = lvl;
        let fill = if d > 0.0 { (h[j] >= lvl_stop).then(|| o[j].max(lvl_stop)) } else {
            let lim = lvl_stop - 0.02; // stop-limit order (uptick rule): limit 2 cents beyond the trigger
            if l[j] > lvl_stop { None } else if o[j] > lvl_stop { Some(lvl_stop) } else if o[j] >= lim { Some(o[j]) } else { None }
        };
        let Some(e) = fill else { t += 1; continue };
        let istop = if d > 0.0 { let v = lo(l, t, 4); v - buf(v) } else { let v = hi(h, t, 4); v + buf(v) };
        if d * (e - istop) <= 0.0 { t += 1; continue; }
        let (exits, last, closed, why) = manage_trend(x, d, j, e, istop, to);
        out.push(scale_trade(d, j, e, t, last, closed, why, exits, c[last], to, c[to], "stop"));
        if !closed { break; }
        t = last;
    }
    out
}


/// Method 2 setup on day t (retrace to support / resistance, then a confirming close): Some(direction).
fn m2_setup(x: &Ctx, n: &Ind, t: usize) -> Option<f64> {
    let (h, l, c, k) = (&x.h, &x.l, &x.c, &n.k);
    if t < 66 || !n.adx[t].is_finite() || !n.s50[t].is_finite() || !n.s50[t - 1].is_finite() || !n.s7[t].is_finite() || !n.s7[t - 1].is_finite() || !k[t].is_finite() || !screens(x, n, t) || n.adx[t] <= 20.0 { return None; }
    let inside = |i: usize| h[i] < h[i - 1] && l[i] > l[i - 1];
    let mut q = t - 1;
    while inside(q) { q -= 1; }
    if n.s50[t] > n.s50[t - 1] {
        let im = (t - 4..=t).min_by(|a, b| l[*a].partial_cmp(&l[*b]).unwrap().then(b.cmp(a))).unwrap();
        let (h20, l10) = (hi(h, t, 20), lo(l, t, 10));
        if l[im] < n.s20[im] && c[t - 1] < n.s7[t - 1] && c[t] > n.s7[t] && c[t] > h[q] && (k[t] <= 40.0 || c[t] < h20 - 0.6 * (h20 - l10)) { return Some(1.0); }
    }
    if n.s50[t] < n.s50[t - 1] {
        let im = (t - 4..=t).max_by(|a, b| h[*a].partial_cmp(&h[*b]).unwrap().then(a.cmp(b))).unwrap();
        let (l20, h10) = (lo(l, t, 20), hi(h, t, 10));
        if h[im] > n.s20[im] && c[t - 1] > n.s7[t - 1] && c[t] < n.s7[t] && c[t] < l[q] && (k[t] >= 60.0 || c[t] > l20 + 0.6 * (h10 - l20)) { return Some(-1.0); }
    }
    None
}

fn run_m2(x: &Ctx, n: &Ind, from: usize, to: usize, days: usize) -> Vec<Trade> {
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let (_, blocks) = m4_walk(x, n, 0, x.r.len() - 1, days);
    let mut out = Vec::new();
    let mut t = from.max(66);
    while t < to {
        let Some(d) = m2_setup(x, n, t) else { t += 1; continue };
        if blocked(&blocks, d, t) { t += 1; continue; }
        let j = t + 1;
        let lim = c[t];
        let fill = if d > 0.0 { (l[j] <= lim).then(|| o[j].min(lim)) } else { (h[j] >= lim).then(|| o[j].max(lim)) };
        let Some(e) = fill else { t += 1; continue };
        let istop = if d > 0.0 { let v = lo(l, t, 4); v - buf(v) } else { let v = hi(h, t, 4); v + buf(v) };
        if d * (e - istop) <= 0.0 { t += 1; continue; }
        let (exits, last, closed, why) = manage_trend(x, d, j, e, istop, to);
        out.push(scale_trade(d, j, e, t, last, closed, why, exits, c[last], to, c[to], "limit"));
        if !closed { break; }
        t = last;
    }
    out
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let g = |k: &str, d: f64| p.get(k).copied().unwrap_or(d);
    let (screen, days) = (g("screen", 1.) > 0.5, g("days", 10.).round() as usize);
    let n = ind(x, screen);
    Some(match id {
        "mmp4_div" => m4_walk(x, &n, from, to, days).0,
        "mmp3_flag" => run_m3(x, &n, from, to, days),
        _ => run_m2(x, &n, from, to, days),
    })
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
