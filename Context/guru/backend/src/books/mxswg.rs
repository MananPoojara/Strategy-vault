//! SwingTrader (MX Capital Trading, mxcapital.com, 1997-98). Source (research drive, read in place, read-only):
//! `USB HDD 1/TradeStation Systems & Indicators/SwingTrader System For TS (mxcapital.com)/Code.NewVersion.rar` ->
//! `SwingTrader.Demo.txt`, `SwinTrader.Demo2.txt`; `Code.OldVersion.rar` -> the same Demo strategy plus the user functions
//! SwingBuy/SwingSell/SwingShort/SwingCover (extracted as data). Book id `mxswg`. Notes: library/mxswg/README.md. Engine: books::rrsim.
//!
//! AND/OR precedence (the code mixes them without brackets): EasyLanguage evaluates AND before OR — G. Pruitt & J. Hill,
//! "Building Winning Trading Systems with TradeStation" (2003) ch.1 "Precedence of Operators", pp.6-7: "1. Parentheses ...
//! 4. <, >, =, <=, >=, <> 5. AND 6. OR" (research drive `Trading Books/George Pruitt - Building Winning Trading Systems with
//! Tradestation (with CD)/`). SwingLow/SwingHigh: TradeStation's swing-bar search as copied in the AmCan `S5_SwingLowBO` /
//! `S5_SwingHBO` functions on the drive (older bars may tie, newer bars must be strictly beyond; -1 when not found).
use crate::books::rrsim::{g, o, pd, run_ts, sdef, A, P};
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "mxswg";
/// Admitted ids (authors/lanea/vendor2/decisions.json).
pub const REVIEWED_LANEA: &[&str] = &["mxswg_demo", "mxswg_demo2"];
pub fn reviewed_lanea(id: &str) -> bool { REVIEWED_LANEA.contains(&id) }

const NOTE: &str = "Code literal (EasyLanguage, 'Buy/Sell/Sell Short/Buy to Cover Next Bar at Market', one unit). AND binds tighter than OR \
(Pruitt & Hill pp.6-7), so in Demo 'trend AND P1 OR P2' = (trend AND P1) OR P2 and in every exit/short test 'S1 OR S2a AND S2b ...' = S1 OR (S2a AND S2b ...). \
Hard-coded numbers stay as printed: 'lowest(L 2 bars ago,e)', 'highest(H of 2 bars ago,8)', 'h of 10 bars ago'. The code runs from the first bar on which \
every value it reads exists (TradeStation MaxBarsBack = longest lookback). Market orders fill at the next open, exits before entries; an entry reverses an \
opposite position. Timeframe: the vendor letter describes swings 'of short- to intermediate-term' in securities, no bar size; run on daily bars. \
India: instrument swap; index shorts hypothetical.";

const R_DEMO: &[&str] = &[
    "F|Inputs b(1), e(6), x(3), y(65), shrtma(26), lngma(65); condition1 = average(c,shrtma) > average(c,lngma); condition2 = average(c,shrtma) < average(c,lngma) (SwingTrader.Demo)",
    "E|\"IF (condition1=true and condition2=false) and ((L<=lowest(L b bar ago,e))AND(C>OPEN)AND(OPEN>L)AND((C-L)>(H-C))AND(L<low e bars ago))OR(((low b bar ago<lowest(L 2 bars ago,e))AND(c b bar ago<=o b bar ago)AND(o b bar ago>l b bar ago)AND((c b bar ago-l b bar ago)>=(h b bar ago-c b bar ago))AND(l b bar ago<l e bars ago)AND(L>l b bar ago)AND(H>h b bar ago)AND(C>OPEN)AND(C>o b bar ago)AND(OPEN>=L)AND(H>=C)AND(OPEN>=c b bar ago))) Then Buy Next Bar 1 share at Market\" — AND before OR: the second pattern buys without the trend test",
    "X|\"IF condition2=true or ((H>=highest(H of b bar ago,e))AND(C<OPEN)AND(OPEN<=H)AND(H>H of e bars ago))OR(h of b bar ago>highest(H of 2 bars ago,8))AND(c of b bar ago>=o of b bar ago)AND(o of b bar ago<h of b bar ago)AND(h of b bar ago>h of 10 bars ago)AND(H<h of b bar ago)AND(L<l of b bar ago)AND(C<OPEN)AND(C<o of b bar ago)AND(OPEN<=H)AND(L<=C)AND(OPEN<h of b bar ago) Then Sell Next Bar at Market\"",
    "E|\"IF (condition2=true and condition1=false) and (((H>highest(H of b bar ago,e))AND(C<OPEN)AND(OPEN<=H)AND(H>H of e bars ago))OR(h of b bar ago>highest(H of 2 bars ago,8))AND ... AND(OPEN<h of b bar ago)) and (SwingLow(1,l,x,y)<SwingLow(2,l,x,y)) and (Swinghigh(1,h,x,y)<Swinghigh(2,h,x,y)) Then Sell Short Next Bar 1 share at Market\"",
    "X|\"IF (((L<lowest(L b bar ago,e))AND(C>OPEN)AND(OPEN>L)AND((C-L)>(H-C))AND(L<low e bars ago))OR((...second buy pattern...))) or ((SwingLow(1,l,x,y)>SwingLow(2,l,x,y))) or condition1=true Then Buy to Cover Next Bar at Market\" (note L<lowest, strict, here)",
];
const R_DEMO2: &[&str] = &[
    "F|Inputs b(1), e(5), x(7), y(60), shrtma(18), lngma(100) (x, y unused); condition1/condition2 as in Demo (SwinTrader.Demo2 = 'a version with a tighter trend filter', SwingDemo.doc)",
    "E|\"IF (condition1=true and condition2=false) and ( ((L<=lowest(L b bar ago,e))AND(C>OPEN)AND(OPEN>L)AND((C-L)>(H-C))AND(L<low e bars ago))OR((...second buy pattern...))) Then Buy Next Bar at Market\" (fully bracketed: the trend test covers both patterns; same as the SwingBuy function)",
    "X|\"IF condition2=true or (((H>=highest(H of b bar ago,e))AND(C<OPEN)AND(OPEN<=H)AND(H>H of e bars ago))OR(h of b bar ago>highest(H of 2 bars ago,8))AND ... AND(OPEN<h of b bar ago)) Then Sell Next Bar at Market\"",
    "E|\"IF (condition2=true and condition1=false) and (((H>highest(H of b bar ago,e))AND(C<OPEN)AND(OPEN<=H)AND(H>H of e bars ago))OR(h of b bar ago>highest(H of 2 bars ago,8))AND ... AND(OPEN<h of b bar ago)) Then Sell Short Next Bar at Market\"",
    "X|\"IF ( (P1 with L<=lowest) OR (second buy pattern) ) or condition1=true Then Buy to Cover Next Bar at Market\"",
];

pub fn catalogue() -> Vec<StrategyDef> {
    vec![
        sdef(BOOK, "mxswg_demo", "SwingTrader Demo (26/65 trend, swing-low/high short filter)", "SwingTrader (mxcapital.com)", "Long/Short", Status::Live, R_DEMO, NOTE),
        sdef(BOOK, "mxswg_demo2", "SwingTrader Demo2 (18/100 'tighter trend filter')", "SwingTrader (mxcapital.com)", "Long/Short", Status::Live, R_DEMO2, NOTE),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match id {
        "mxswg_demo" => Some(("mxswg", "SwingTrader (MX Capital)", "Demo")),
        "mxswg_demo2" => Some(("mxswg", "SwingTrader (MX Capital)", "Demo2")),
        _ => None,
    }
}

fn defs(id: &str) -> Option<[f64; 6]> {
    match id { "mxswg_demo" => Some([1., 6., 3., 65., 26., 65.]), "mxswg_demo2" => Some([1., 5., 7., 60., 18., 100.]), _ => None }
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let d = defs(id)?;
    let mut v = vec![pd("b", "b: swing strength (bars ago)", d[0], 1., 5., 1.), pd("e", "e: swing length", d[1], 2., 15., 1.)];
    if id == "mxswg_demo" { v.push(pd("x", "x: SwingLow/SwingHigh strength", d[2], 1., 10., 1.)); v.push(pd("y", "y: SwingLow/SwingHigh length", d[3], 10., 200., 5.)); }
    v.push(pd("shrtma", "Short moving average", d[4], 5., 100., 1.));
    v.push(pd("lngma", "Long moving average", d[5], 20., 250., 5.));
    Some(v)
}

/// TradeStation SwingLowBar/SwingHighBar search (S5_SwingLowBO / S5_SwingHBO text): price of the `occur`-th swing within
/// `length` bars (strength bars each side; older bars may tie, newer bars must be strictly beyond), or -1.
pub fn swing(p: &[f64], i: usize, occur: usize, strength: usize, length: usize, low: bool) -> f64 {
    if strength == 0 { return -1.; }
    let at = |k: usize| p[i - k];
    let mut j = strength;
    let mut counter = 0;
    while j < length {
        let p1 = at(j);
        let mut truth = true;
        let mut x = j + 1;
        while truth && x - j <= strength { if if low { p1 > at(x) } else { p1 < at(x) } { truth = false; } x += 1; }
        let mut x = j as i64 - 1;
        while truth && j as i64 - x <= strength as i64 { let v = at(x as usize); if if low { p1 >= v } else { p1 <= v } { truth = false; } x -= 1; }
        if truth { counter += 1; }
        if counter >= occur { return p1; }
        j += 1;
    }
    -1.
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let d = defs(id)?;
    if from >= to || to >= x.r.len() { return None; }
    let demo = id == "mxswg_demo";
    let b = g(p, "b", d[0]).max(1.) as usize;
    let e = g(p, "e", d[1]).max(1.) as usize;
    let sx = g(p, "x", d[2]).max(1.) as usize;
    let sy = g(p, "y", d[3]).max(2.) as usize;
    let (ns, nl) = (g(p, "shrtma", d[4]).max(1.) as usize, g(p, "lngma", d[5]).max(1.) as usize);
    let (o_, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let ms = crate::books::rrsim::sma(c, ns);
    let ml = crate::books::rrsim::sma(c, nl);
    // first bar on which every value read exists
    let mut mbb = [nl - 1, ns - 1, b + e - 1, e + 1, 9, 10, e].into_iter().max().unwrap();
    if demo { mbb = mbb.max(sy - 1 + sx); }
    let lo = |v: &[f64], i: usize, back: usize, n: usize| (0..n).map(|k| v[i - back - k]).fold(f64::INFINITY, f64::min);
    let hi = |v: &[f64], i: usize, back: usize, n: usize| (0..n).map(|k| v[i - back - k]).fold(f64::NEG_INFINITY, f64::max);
    Some(run_ts(x, from, to, |i, _st, _now, next| {
        if i < mbb { return; }
        let (c1, c2) = (ms[i] > ml[i], ms[i] < ml[i]);
        let (o0, h0, l0, cc) = (o_[i], h[i], l[i], c[i]);
        let (ob, hb, lb, cb) = (o_[i - b], h[i - b], l[i - b], c[i - b]);
        let low_e = l[i - e];
        let p1 = |strict: bool| (if strict { l0 < lo(l, i, b, e) } else { l0 <= lo(l, i, b, e) }) && cc > o0 && o0 > l0 && (cc - l0) > (h0 - cc) && l0 < low_e;
        let p2 = lb < lo(l, i, 2, e) && cb <= ob && ob > lb && (cb - lb) >= (hb - cb) && lb < low_e && l0 > lb && h0 > hb && cc > o0 && cc > ob && o0 >= l0 && h0 >= cc && o0 >= cb;
        let s1 = |strict: bool| (if strict { h0 > hi(h, i, b, e) } else { h0 >= hi(h, i, b, e) }) && cc < o0 && o0 <= h0 && h0 > h[i - e];
        let s2 = hb > hi(h, i, 2, 8) && cb >= ob && ob < hb && hb > h[i - 10] && h0 < hb && l0 < lb && cc < o0 && cc < ob && o0 <= h0 && l0 <= cc && o0 < hb;
        let buy = if demo { (c1 && !c2 && p1(false)) || p2 } else { c1 && !c2 && (p1(false) || p2) };
        let sell = c2 || s1(false) || s2;
        let (sl1, sl2, sh1, sh2) = if demo {
            (swing(l, i, 1, sx, sy, true), swing(l, i, 2, sx, sy, true), swing(h, i, 1, sx, sy, false), swing(h, i, 2, sx, sy, false))
        } else { (0., 0., 0., 0.) };
        let short = c2 && !c1 && (s1(true) || s2) && (!demo || (sl1 < sl2 && sh1 < sh2));
        let cover = if demo { p1(true) || p2 || sl1 > sl2 || c1 } else { p1(false) || p2 || c1 };
        if buy { next.push(o(A::Buy, P::Mkt, "Buy")); }
        if sell { next.push(o(A::Xl, P::Mkt, "Sell")); }
        if short { next.push(o(A::Sell, P::Mkt, "Sell Short")); }
        if cover { next.push(o(A::Xs, P::Mkt, "Buy to Cover")); }
    }))
}

#[cfg(test)]
mod tests {
    use super::swing;
    #[test]
    fn swing_low_ties_and_instances() {
        // bars oldest -> newest; current bar is the last. strength 1, length 6.
        let p = [5., 3., 4., 3., 2., 6., 7.];
        let i = p.len() - 1;
        // j=1: 6 vs older 2 -> 6>2 not a low; j=2: 2 (older 3, newer 6) -> swing low #1 = 2; j=3: 3 vs older 4 ok, newer 2: 3>=2 fails;
        // j=4: 4 vs older 3: 4>3 fails; j=5: 3 vs older 5 ok, newer 4 ok -> #2 = 3
        assert_eq!(swing(&p, i, 1, 1, 6, true), 2.);
        assert_eq!(swing(&p, i, 2, 1, 6, true), 3.);
        assert_eq!(swing(&p, i, 3, 1, 6, true), -1.);
        // a tie with the OLDER neighbour is allowed, with the NEWER one is not
        let q = [4., 2., 2., 5., 6.];
        assert_eq!(swing(&q, 4, 1, 1, 4, true), 2.); // j=1 is 5 (older 2 lower); j=2 = 2 with an equal OLDER neighbour and newer 5 -> swing low 2
        let r = [4., 3., 2., 2., 6.];
        assert_eq!(swing(&r, 4, 1, 1, 4, true), 2.); // j=1: the newer of two equal lows (older tie allowed, newer 6) is the swing low
        assert_eq!(swing(&r, 4, 2, 1, 4, true), -1.); // the older 2 has an equal NEWER neighbour -> not a swing low
    }
}
