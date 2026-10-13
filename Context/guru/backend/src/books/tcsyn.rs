//! Synergy System (traderstech.net). Source (research drive, read in place, read-only):
//! `USB HDD 1/TradeStation Systems & Indicators/Synergy System (traderstech.net)/_From other source/Open Code.rar` ->
//! `Open Code.txt` (extracted as data). Book id `tcsyn`. Notes: library/tcsyn/README.md. Engine: books::rrsim.
use crate::books::rrsim::{g, o, pd, run_ts, sdef, true_high, true_low, A, P};
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "tcsyn";
const ID: &str = "tcsyn_synergy";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![sdef(BOOK, ID, "Synergy trendline stop and reverse (STRENGTH 4 pivots)", "Synergy (Open Code.txt)", "Long/Short", Status::Live, &[
        "F|Pivot low: \"IF L[R]<LOWEST(TrueLow,R)\" (R = STRENGTH 4), skip equal lows (\"WHILE L[R]=L[R+E]\"), \"IF L[R]<LOWEST(TrueLow,R)[R+E]\" -> push LP[0]=L[R], LPBAR[0]=R (20-slot arrays, every LPBAR/HPBAR +1 each bar); pivot high mirror with TrueHigh",
        "F|Up line: LPOINT2 = LP[0]; LPOINT1 = the first earlier LP[Y] < LP[0] (Y < 20); if LPOINT1>0 and LPOINT1BAR>LPOINT2BAR: USLOPE=(LPOINT2-LPOINT1)/(LPOINT1BAR-LPOINT2BAR), ULINE = LPOINT1 + USLOPE x (LPOINT1BAR+1)",
        "E|\"QSELL=2*C-TrueHigh>ULINE; IF QSELL AND MARKETPOSITION>-1 AND (C>ULINE OR L>ULINE-SLOPE) THEN SELL(\\\" LINE 1 Sell\\\") ULINE STOP\" (old syntax: sell short next bar)",
        "F|Down line: POINT2 = HP[0]; POINT1 = the first earlier HP[Y] > HP[0]; SLOPE=(POINT2-POINT1)/(POINT1BAR-POINT2BAR), LINE = POINT1 + SLOPE x (POINT1BAR+1)",
        "E|\"QBUY=2*C-TrueLow<LINE; IF QBUY AND MARKETPOSITION<1 AND (C<LINE OR H<LINE-SLOPE) THEN BUY(\\\" LINE 1 Buy\\\") LINE STOP\"",
        "X|No exits: stop and reverse",
    ], "Literal: the sell test 'L>ULINE-SLOPE' uses SLOPE, the down-line slope variable, whose value at that point is the previous bar's (it is recomputed later in the code). \
Unlike Weaver-DT / Universal LT (same trendline family) there is no MAXBARSBACK-limited target, so the code is complete. Lookbacks that would reach before the first bar are skipped. \
TrueHigh/TrueLow use the prior close. Timeframe not stated; run on daily bars. India: instrument swap; index shorts hypothetical.")]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    (id == ID).then_some(("tcsyn", "Synergy (traderstech.net)", "code literal"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    (id == ID).then(|| vec![pd("strength", "STRENGTH (pivot bars)", 4., 1., 10., 1.)])
}

/// Lowest (or highest) of v over the n bars ending `back` bars before i; None if it reaches before bar `min_i`.
fn ext(v: &[f64], i: usize, back: usize, n: usize, hi: bool, min_i: usize) -> Option<f64> {
    let end = i.checked_sub(back)?;
    let start = (end + 1).checked_sub(n)?;
    if start < min_i { return None; }
    Some(if hi { v[start..=end].iter().copied().fold(f64::NEG_INFINITY, f64::max) } else { v[start..=end].iter().copied().fold(f64::INFINITY, f64::min) })
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if id != ID || from >= to || to >= x.r.len() { return None; }
    let r = g(p, "strength", 4.).max(1.) as usize;
    let (th, tl) = (true_high(x), true_low(x));
    let (h, l, c) = (&x.h, &x.l, &x.c);
    let (mut lp, mut lpb, mut hp, mut hpb) = ([0f64; 20], [0f64; 20], [0f64; 20], [0f64; 20]);
    let mut slope = 0f64;
    Some(run_ts(x, from, to, |i, st, _now, next| {
        for z in 0..20 { lpb[z] += 1.; hpb[z] += 1.; }
        // pivot low
        if i >= r {
            if let Some(m) = ext(&tl, i, 0, r, false, 1) {
                if l[i - r] < m {
                    let mut e = 1;
                    while i >= r + e && l[i - r] == l[i - r - e] { e += 1; }
                    if let Some(m2) = ext(&tl, i, r + e, r, false, 1) {
                        if l[i - r] < m2 {
                            for z in (1..20).rev() { lp[z] = lp[z - 1]; lpb[z] = lpb[z - 1]; }
                            lp[0] = l[i - r]; lpb[0] = r as f64;
                        }
                    }
                }
            }
        }
        let (lp2, lp2b) = (lp[0], lpb[0]);
        let (mut lp1, mut lp1b) = (0f64, 0f64);
        let mut y = 1;
        while lp1 == 0. && y < 20 { if lp[y] < lp[0] { lp1 = lp[y]; lp1b = lpb[y]; } y += 1; }
        if lp1 > 0. && lp1b > lp2b {
            let us = (lp2 - lp1) / (lp1b - lp2b);
            let uline = lp1 + us * (lp1b + 1.);
            let qsell = 2. * c[i] - th[i] > uline;
            if qsell && st.pos > -1 && (c[i] > uline || l[i] > uline - slope) { next.push(o(A::Sell, P::Stop(uline), " LINE 1 Sell")); }
        }
        // pivot high
        if i >= r {
            if let Some(m) = ext(&th, i, 0, r, true, 1) {
                if h[i - r] > m {
                    let mut e = 1;
                    while i >= r + e && h[i - r] == h[i - r - e] { e += 1; }
                    if let Some(m2) = ext(&th, i, r + e, r, true, 1) {
                        if h[i - r] > m2 {
                            for z in (1..20).rev() { hp[z] = hp[z - 1]; hpb[z] = hpb[z - 1]; }
                            hp[0] = h[i - r]; hpb[0] = r as f64;
                        }
                    }
                }
            }
        }
        let (p2, p2b) = (hp[0], hpb[0]);
        let (mut p1, mut p1b) = (0f64, 0f64);
        let mut y = 1;
        while p1 == 0. && y < 20 { if hp[y] > hp[0] { p1 = hp[y]; p1b = hpb[y]; } y += 1; }
        if p1 > 0. && p1b > p2b {
            slope = (p2 - p1) / (p1b - p2b);
            let line = p1 + slope * (p1b + 1.);
            let qbuy = 2. * c[i] - tl[i] < line;
            if qbuy && st.pos < 1 && (c[i] < line || h[i] < line - slope) { next.push(o(A::Buy, P::Stop(line), " LINE 1 Buy")); }
        }
    }))
}
