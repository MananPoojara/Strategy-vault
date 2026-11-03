//! INVESTools, The 5-Step Online Investing Formula (DVD course manual, 2005), Step 4 Technical Analysis: the three arrow
//! indicators (p.6-14), buy signals (p.18-20, 28-29), sell stop orders (p.32-35), sell signals and the 3 and 3 rule
//! (p.39-41); research share USB HDD 1/TRADING KNOWLEDGE-2/_InvesTools.com/5-Step Online Investing Formula DVD Course &
//! Manual (Video 3.91 GB)/Manual/6.- Step 4 - Technical Analysis.pdf. Book id `it5sf`. Lane K7 (titles loop, 1 Nov 2026);
//! rule text authors/batch-c/C1w/investools-5step-step4.txt; independent check authors/titles-1030/K7w/k7_ref.py (header).
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::mkits::{ema, hhv, llv};
use super::mpts::sma;
use super::rbtie::mk;

/// Admitted ids (authors/titles-1030/K7w/k7_ref.py (header)).
pub const REVIEWED: &[&str] = &["it5sf_three_arrows"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] Arrow colours per bar: moving average green while the close is above the 30-day simple average, red while below; MACD green while the MACD line is above its signal line, red while below, with MACD 8/17/9 (the INVESTools Prophet setting printed in the companion Advanced Technical Analysis manual, Fig. 6.1; not printed in this manual); stochastics %K over 14 days smoothed by a 3-day average (the smoothing is not printed): between the 25 and 75 lines its colour follows its slope (up green, down red, the 'ghost' arrows of p.14 and 29), outside them it keeps its last colour, so a green arrow is the rise back above 25 and a red arrow the drop back below 75. Buy at the close of the bar on which the most recent arrow of all three indicators becomes green ('as long as the MOST RECENT arrow on each individual indicator is green, it is a valid buy signal', p.28); after a sale, the next buy needs a new set of three greens. Initial sell stop 3% below the lowest low of the two weeks (ten bars, the signal bar included) before the buy signal (p.32-33), filled at the stop or a lower open from the next bar. Sell at the close of the bar on which all three become red (p.39); option: the '3 and 3 Rule' instead (p.39-41): at each new set of three reds move the stop up to 3% below the lowest low from the first red arrow of the set to the last, and sell only on the stop. Option: next-open fills. Long only. Not applied: 'an uptrending stock', the volume surge for sideways stocks and the Phase 2 fundamental grade (judgement and fundamentals). The manual teaches on US stocks with chart examples only; indices, ETFs and India are a declared application.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "it5sf_three_arrows", name: "Three green arrows: buy when the 30-day average, MACD and stochastics are all green; stop 3% under the two-week low; sell on three reds",
        group: "INVESTools 5-Step Formula", session: 0, dir: "Long", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: "it5sf", rules: &[
            "F|30-day average: \"The green arrow appears when the stock has closed above the moving average\"; MACD: \"When an 'up' green arrow appears, this indicates the MACD has moved above the center signal line\"; stochastics (14 days): green \"only if the indicator has been below the 25% line and then rallies above it\", red when it \"has been above the 75% line and then drops below it\"; \"When the stochastics is between the 25% and 75% lines, look at the slope\" (p.6-14)",
            "E|\"as long as the MOST RECENT arrow on each individual indicator is green, it is a valid buy signal\" (p.28); \"place your initial sell stop 3% below the lowest price in the two weeks prior to the buy signal\" (p.33)",
            "X|\"Sell on 3 Red Arrows\" (p.39); option: \"move your sell stop UP to a price that is 3% below the lowest price at which the stock traded during the formation of the three reds\" (3 and 3 Rule, p.39-41)"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    (id == "it5sf_three_arrows").then_some(("it5sf_three_arrows", "INVESTools three arrows", "SMA 30, MACD 8/17/9, stochastics 14"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    (id == "it5sf_three_arrows").then(|| vec![
        ParamDef { key: "exit", label: "0 = sell on three reds (p.39), 1 = 3 and 3 Rule stop only", def: 0., min: 0., max: 1., step: 1. },
        ParamDef { key: "fill", label: "0 = at the signal close (reading), 1 = next open", def: 0., min: 0., max: 1., step: 1. },
    ])
}

/// Colour runs per indicator: 1 green, -1 red, 0 undefined.
fn keep(prev: i8, v: i8) -> i8 { if v != 0 { v } else { prev } }

/// (moving-average, MACD, stochastics) colours per bar.
pub fn arrows(x: &Ctx) -> [Vec<i8>; 3] {
    let (c, n) = (&x.c, x.c.len());
    let ma = sma(c, 30);
    let (e8, e17) = (ema(c, 8), ema(c, 17));
    let m: Vec<f64> = e8.iter().zip(&e17).map(|(a, b)| a - b).collect();
    let sg = ema(&m, 9);
    let (hh, ll) = (hhv(&x.h, 14), llv(&x.l, 14));
    let raw: Vec<f64> = (0..n).map(|i| { let d = hh[i] - ll[i]; if d > 0. { 100. * (c[i] - ll[i]) / d } else { f64::NAN } }).collect();
    let k = sma(&raw, 3);
    let (mut a, mut b, mut s) = (vec![0i8; n], vec![0i8; n], vec![0i8; n]);
    for i in 0..n {
        let (pa, pb, ps) = if i > 0 { (a[i - 1], b[i - 1], s[i - 1]) } else { (0, 0, 0) };
        a[i] = keep(pa, if c[i] > ma[i] { 1 } else if c[i] < ma[i] { -1 } else { 0 });
        b[i] = keep(pb, if m[i] > sg[i] { 1 } else if m[i] < sg[i] { -1 } else { 0 });
        s[i] = if i > 0 && k[i].is_finite() && k[i - 1].is_finite() && (25. ..=75.).contains(&k[i]) {
            keep(ps, if k[i] > k[i - 1] { 1 } else if k[i] < k[i - 1] { -1 } else { 0 })
        } else { ps };
    }
    [a, b, s]
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let rule33 = p.get("exit").copied().unwrap_or(0.) >= 0.5;
    let nxt = p.get("fill").copied().unwrap_or(0.) >= 0.5;
    let (o, l, c) = (&x.o, &x.l, &x.c);
    let ar = arrows(x);
    let all = |i: usize, v: i8| ar.iter().all(|s| s[i] == v);
    let set = |i: usize, v: i8| all(i, v) && (i == 0 || !all(i - 1, v));
    // first bar of the latest run of each colour series ending at i (the arrow day)
    let first_red = |i: usize| ar.iter().map(|s| { let mut j = i; while j > 0 && s[j - 1] == s[i] { j -= 1; } j }).min().unwrap();
    let low10 = |i: usize| l[i.saturating_sub(9)..=i].iter().copied().fold(f64::INFINITY, f64::min);
    let mut out = vec![];
    let mut pos: Option<(usize, f64, f64)> = None; // (entry bar, price, stop)
    let (mut pend_in, mut pend_out): (Option<f64>, bool) = (None, false);
    for i in from..=to {
        if pend_out { if let Some((e, ep, _)) = pos.take() { out.push(mk(1., e, ep, "open", i, o[i], "three red arrows", false, i - 1)); } pend_out = false; }
        if let Some(st) = pend_in.take() { pos = Some((i, o[i], st)); }
        if let Some((e, ep, st)) = pos {
            if i > e && l[i] <= st { out.push(mk(1., e, ep, if nxt { "open" } else { "close" }, i, o[i].min(st), "sell stop", false, i)); pos = None; }
            else if i > e && all(i, -1) {
                if rule33 { if i == e + 1 || !all(i - 1, -1) { let ns = 0.97 * l[first_red(i)..=i].iter().copied().fold(f64::INFINITY, f64::min); if ns > st { pos = Some((e, ep, ns)); } } }
                else if nxt { if i < to { pend_out = true; } }
                else { out.push(mk(1., e, ep, "close", i, c[i], "three red arrows", false, i)); pos = None; }
            }
        } else if set(i, 1) && i >= 9 {
            let st = 0.97 * low10(i);
            if nxt { if i < to { pend_in = Some(st); } } else { pos = Some((i, c[i], st)); }
        }
    }
    if let Some((e, ep, _)) = pos { out.push(mk(1., e, ep, if nxt { "open" } else { "close" }, to, c[to], "open", true, to)); }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn buys_on_three_greens_and_sells_on_three_reds_or_the_stop() {
        let c: Vec<f64> = (0..500).map(|i| 50. + 6. * (i as f64 / 13.).sin() + 2. * (i as f64 / 3.1).cos() + i as f64 * 0.01).collect();
        let rows: Vec<Row> = c.iter().enumerate().map(|(i, &v)| Row { ticker: "T".into(), date: format!("{}", i), open: v, high: v + 0.5, low: v - 0.5, close: v, volume: None }).collect();
        let x = Ctx::new(&rows, &rows, &[]);
        let ar = arrows(&x);
        let t = run("it5sf_three_arrows", &x, 0, rows.len() - 1, &Params::new()).unwrap();
        assert!(!t.is_empty());
        for z in &t {
            let e = z.legs[0].i;
            assert!(ar.iter().all(|s| s[e] == 1) && !ar.iter().all(|s| s[e - 1] == 1));
            assert!(z.open || z.reason == "sell stop" || ar.iter().all(|s| s[z.exit_i] == -1));
        }
        assert!(run("it5sf_x", &x, 0, 5, &Params::new()).is_none());
    }
}

// Instrument admission: INVESTools (5-Step Online Investing Formula; Advanced Technical Analysis Workshop) 2026-11-01 (K7).
// Admission only for the module REVIEWED lists (authors/titles-1030/K7w/k7_ref.py (header)).
// Runs on actual indices, ETFs and stocks, US and India (index_etf_or_stock, nifty50 basket, declared application).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| (def.book == "it5sf" && reviewed(def.id)) || (def.book == "itatw" && super::itatw::reviewed(def.id)),
    label: "INVESTools taught these rules on US stocks with chart examples (the workshop's backtest example: General Electric, 2004); indices, ETFs and US/India stocks are a declared application.",
    basis: "INVESTools, 5-Step Online Investing Formula, Step 4 Technical Analysis manual p.6-41; Advanced Technical Analysis Workshop Manual ch.6 p.148-151",
};
