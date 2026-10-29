//! Smart Money Concepts group (2026-10-29). Sources: Smart Money Concepts [LuxAlgo] (TradingView open source
//! PUB;6daafb2cabe6419d98ae25229d2327f8, CC BY-NC-SA 4.0; Pine mirror RouZuoMong2003/SmartMoneyConcepts-LuxAlgo), LuxAlgo
//! Library pages (SMC, Optimal Trade Entry, Breaker Blocks), Inversion Fair Value Gaps (IFVG) [LuxAlgo] (TradingView open
//! source, CC BY-NC-SA 4.0; Pine mirrors sappbr/pine-code and austindd/trading-view-scripts) and joshyattridge/smart-money-concepts
//! smc.py (MIT, master @ 1b62fd6c, swing_highs_lows and ob). Formulas and causal adaptations: see each definition string.
//! Every output is causal: a swing that needs right-side bars is reported on its confirmation bar, never back-dated.
//! Not implemented, UNCONFIRMED: Balanced Price Range (no published source fixes an executable rule; ICT material leaves the
//! time gap between the two FVGs unquantified, and the open-source BPR scripts could not be retrieved for review) and
//! Supply/Demand zones (no single primary definition of base/departure detection). Mitigation blocks: not defined by smc.py
//! or the cited LuxAlgo sources, so skipped.
use super::bars::Bars;
use super::catalogue::{int, n, p, real, IndicatorDef, Needs, Output, ParamSpec, Params, Placement};
use super::ta::{self, NA};

#[allow(clippy::too_many_arguments)]
const fn def(id: &'static str, name: &'static str, params: &'static [ParamSpec], outputs: &'static [&'static str], definition: &'static str,
             first: &'static [usize], f: fn(&Bars, &Params) -> Output) -> IndicatorDef {
    IndicatorDef { id, name, category: SM, params, outputs, placement: Placement::Price, needs: Needs::NONE, definition, first, sparse: true, compute: Some(f), note: "" }
}
const SM: &str = "Smart Money";
const X: usize = usize::MAX;
fn flag(b: bool) -> f64 { if b { 1. } else { 0. } }
fn hi_of(x: &[f64]) -> f64 { x.iter().copied().fold(f64::MIN, f64::max) }
fn lo_of(x: &[f64]) -> f64 { x.iter().copied().fold(f64::MAX, f64::min) }

/// LuxAlgo leg(size) per bar: +1 = swing low low[t − size] confirmed on bar t, −1 = swing high high[t − size], 0 = none.
fn legs(h: &[f64], l: &[f64], size: usize) -> Vec<i8> {
    let (mut leg, mut ev) = (0i8, vec![0i8; h.len()]);
    for t in size..h.len() {
        let prev = leg;
        if h[t - size] > hi_of(&h[t + 1 - size..=t]) { leg = 0; } else if l[t - size] < lo_of(&l[t + 1 - size..=t]) { leg = 1; }
        ev[t] = leg - prev;
    }
    ev
}
/// Pine ta.atr(n): RMA of the true range (bar 0 = high − low), seeded at bar n − 1 with the mean of the first n.
fn atr_pine(b: &Bars, len: usize) -> Vec<f64> {
    let tr = ta::tr(b, true);
    let (mut out, mut r) = (vec![NA; tr.len()], NA);
    for t in 0..tr.len() {
        if t + 1 == len { r = tr[..len].iter().sum::<f64>() / len as f64; } else if !r.is_nan() { r = (tr[t] + (len - 1) as f64 * r) / len as f64; }
        out[t] = r;
    }
    out
}

const LEN: &[ParamSpec] = &[int("length", "Swing length", 50., 2., 500.)];
fn smc_swings(b: &Bars, prm: &Params) -> Output {
    let (s, m) = (n(prm, "length"), b.len());
    let ev = legs(&b.high, &b.low, s);
    let mut o = vec![vec![NA; m]; 4];
    let (mut hi, mut lo) = (NA, NA);
    for t in 0..m {
        if ev[t] == 1 { lo = b.low[t - s]; } else if ev[t] == -1 { hi = b.high[t - s]; }
        if t >= s { o[0][t] = flag(ev[t] == 1); o[1][t] = flag(ev[t] == -1); }
        o[2][t] = hi; o[3][t] = lo;
    }
    Ok(o)
}

fn smc_bos_choch(b: &Bars, prm: &Params) -> Output {
    let (s, m, c) = (n(prm, "length"), b.len(), &b.close);
    let ev = legs(&b.high, &b.low, s);
    let mut o = vec![vec![NA; m]; 7]; // bull_bos, bull_choch, bear_bos, bear_choch, bull_level, bear_level, trend
    let (mut hi, mut lo, mut hx, mut lx, mut bias) = (NA, NA, false, false, 0.);
    let (mut ph, mut pl) = (NA, NA); // the levels as they stood on the previous bar (ta.crossover)
    for t in 0..m {
        if ev[t] == 1 { lo = b.low[t - s]; lx = false; } else if ev[t] == -1 { hi = b.high[t - s]; hx = false; }
        let mut e = [0.; 4];
        if t > 0 && c[t] > hi && c[t - 1] <= ph && !hx {
            e[if bias == -1. { 1 } else { 0 }] = 1.;
            hx = true; bias = 1.; o[4][t] = hi;
        }
        if t > 0 && c[t] < lo && c[t - 1] >= pl && !lx {
            e[if bias == 1. { 3 } else { 2 }] = 1.;
            lx = true; bias = -1.; o[5][t] = lo;
        }
        (ph, pl) = (hi, lo);
        if t >= s { for k in 0..4 { o[k][t] = e[k]; } o[6][t] = bias; }
    }
    Ok(o)
}

const EQ: &[ParamSpec] = &[int("length", "Bars confirmation", 3., 1., 100.), real("threshold", "Threshold (x ATR)", 0.1, 0., 0.5, 0.1),
    int("atr_length", "ATR length", 200., 1., 500.)];
fn smc_equal_hl(b: &Bars, prm: &Params) -> Output {
    let (s, thr, al, m) = (n(prm, "length"), p(prm, "threshold"), n(prm, "atr_length"), b.len());
    let (ev, atr) = (legs(&b.high, &b.low, s), atr_pine(b, al));
    let mut o = vec![vec![NA; m]; 4]; // bull_eql, bear_eqh, eql_level, eqh_level
    let (mut ph, mut pl, warm) = (NA, NA, s.max(al - 1));
    for t in 0..m {
        let (mut bu, mut be) = (0., 0.);
        if ev[t] == 1 {
            let lv = b.low[t - s];
            if (pl - lv).abs() < thr * atr[t] { bu = 1.; o[2][t] = lv; }
            pl = lv;
        } else if ev[t] == -1 {
            let lv = b.high[t - s];
            if (ph - lv).abs() < thr * atr[t] { be = 1.; o[3][t] = lv; }
            ph = lv;
        }
        if t >= warm { o[0][t] = bu; o[1][t] = be; }
    }
    Ok(o)
}

const OB: &[ParamSpec] = &[int("swing_length", "Swing length (bars each side)", 50., 1., 200.), int("close_mitigation", "Mitigate on body (1) or wick (0)", 0., 0., 1.)];
struct Block { top: f64, bottom: f64, breaker: bool }
/// smc.ob state machine, causal: [bull, bear, bull_mitigated, bear_mitigated, OB bull top/bottom, OB bear top/bottom,
/// breaker bull top/bottom (failed bearish OBs), breaker bear top/bottom (failed bullish OBs)].
fn ob_engine(b: &Bars, prm: &Params) -> Vec<Vec<f64>> {
    let (k, cm, m) = (n(prm, "swing_length"), p(prm, "close_mitigation") == 1., b.len());
    let (o, h, l, c) = (&b.open, &b.high, &b.low, &b.close);
    let mut out = vec![vec![0.; m]; 4];
    out.extend(vec![vec![NA; m]; 8]);
    let (mut sh, mut sl, mut crossed): (Option<usize>, Option<usize>, Vec<bool>) = (None, None, vec![false; m]);
    let (mut bulls, mut bears): (Vec<Block>, Vec<Block>) = (Vec::new(), Vec::new());
    for t in 0..m {
        // 1) existing blocks: a breaker is dropped on the opposite wick, an active block mitigated becomes a breaker
        let (dn, up) = if cm { (o[t].min(c[t]), o[t].max(c[t])) } else { (l[t], h[t]) };
        bulls.retain_mut(|z| if z.breaker { !(h[t] > z.top) } else { if dn < z.bottom { z.breaker = true; out[2][t] = 1.; } true });
        bears.retain_mut(|z| if z.breaker { !(l[t] < z.bottom) } else { if up > z.top { z.breaker = true; out[3][t] = 1.; } true });
        // 2) swing at p = t − k confirmed by the k bars after it (window p − k + 1 ..= t); a high wins over a low
        if t + 1 >= 2 * k {
            let pv = t - k;
            if h[pv] == hi_of(&h[pv + 1 - k..=t]) { sh = Some(pv); } else if l[pv] == lo_of(&l[pv + 1 - k..=t]) { sl = Some(pv); }
        }
        // 3) a close through the newest unused swing makes a block on the extreme candle since it (last on ties)
        if let Some(s) = sh.filter(|&s| c[t] > h[s] && !crossed[s]) {
            crossed[s] = true;
            let j = (s + 1..t).fold(None, |a: Option<usize>, i| if a.is_none_or(|a| l[i] <= l[a]) { Some(i) } else { a }).unwrap_or(t - 1);
            bulls.push(Block { top: h[j], bottom: l[j], breaker: false });
            out[0][t] = 1.;
        }
        if let Some(s) = sl.filter(|&s| c[t] < l[s] && !crossed[s]) {
            crossed[s] = true;
            let j = (s + 1..t).fold(None, |a: Option<usize>, i| if a.is_none_or(|a| h[i] >= h[a]) { Some(i) } else { a }).unwrap_or(t - 1);
            bears.push(Block { top: h[j], bottom: l[j], breaker: false });
            out[1][t] = 1.;
        }
        // 4) newest block of each kind
        for (col, list, brk) in [(4, &bulls, false), (6, &bears, false), (8, &bears, true), (10, &bulls, true)] {
            if let Some(z) = list.iter().rev().find(|z| z.breaker == brk) { out[col][t] = z.top; out[col + 1][t] = z.bottom; }
        }
    }
    out
}
fn order_block(b: &Bars, prm: &Params) -> Output { let mut e = ob_engine(b, prm); e.truncate(8); Ok(e) }
fn breaker_block(b: &Bars, prm: &Params) -> Output {
    let mut e = ob_engine(b, prm);
    Ok([3, 2, 8, 9, 10, 11].iter().map(|&k| std::mem::take(&mut e[k])).collect())
}

const IF: &[ParamSpec] = &[real("atr_multi", "ATR multiplier (gap filter)", 0.25, 0., 5., 0.25), int("wick", "Signal on wicks (1) or close (0)", 0., 0., 1.)];
struct Gap { top: f64, bot: f64, dir: i8, armed: bool }
const BUFFER: usize = 100; // LuxAlgo: FVGs kept in memory per array
fn ifvg(b: &Bars, prm: &Params) -> Output {
    let (mult, wick, m) = (p(prm, "atr_multi"), p(prm, "wick") == 1., b.len());
    let (o, h, l, c) = (&b.open, &b.high, &b.low, &b.close);
    let atr = atr_pine(b, 200);
    let mut out = vec![vec![0.; m]; 2];
    out.extend(vec![vec![NA; m]; 4]);
    let (mut bull_fvg, mut bear_fvg, mut bull_inv, mut bear_inv): (Vec<Gap>, Vec<Gap>, Vec<Gap>, Vec<Gap>) = Default::default();
    let mut cum = 0.;
    for t in 0..m {
        cum += h[t] - l[t];
        let thr = if atr[t].is_nan() { cum / (t + 1) as f64 } else { atr[t] * mult };
        let (ctop, cbot) = (o[t].max(c[t]), o[t].min(c[t]));
        if t >= 2 {
            if l[t] > h[t - 2] && c[t - 1] > h[t - 2] && (l[t] - h[t - 2]).abs() > thr { bull_fvg.push(Gap { top: l[t], bot: h[t - 2], dir: 1, armed: false }); }
            if h[t] < l[t - 2] && c[t - 1] < l[t - 2] && (l[t - 2] - h[t]).abs() > thr { bear_fvg.push(Gap { top: l[t - 2], bot: h[t], dir: -1, armed: false }); }
        }
        for (ary, inv) in [(&mut bull_fvg, &mut bull_inv), (&mut bear_fvg, &mut bear_inv)] {
            if ary.len() >= BUFFER { ary.remove(0); }
            for i in (0..ary.len()).rev() {
                if (ary[i].dir == 1 && cbot < ary[i].bot) || (ary[i].dir == -1 && ctop > ary[i].top) { inv.push(ary.remove(i)); }
            }
        }
        // inverted bullish FVGs act as resistance (bear signals), inverted bearish FVGs as support (bull signals)
        for (ary, sig) in [(&mut bull_inv, 1), (&mut bear_inv, 0)] {
            if ary.len() >= BUFFER { ary.remove(0); }
            let mut fire = false;
            for i in (0..ary.len()).rev() {
                let Gap { top, bot, dir: d, armed } = ary[i];
                if !armed { ary[i].armed = true; ary[i].dir = -d; continue; } // a fresh zone only arms this bar
                if d == -1 {
                    let r = if wick { h[t] } else { c[t - 1] };
                    fire |= c[t] < bot && bot <= r && r < top;
                } else {
                    let r = if wick { l[t] } else { c[t - 1] };
                    fire |= c[t] > top && bot < r && r <= top;
                }
                if (d == -1 && ctop > top) || (d == 1 && cbot < bot) { ary.remove(i); }
            }
            out[sig][t] = flag(fire);
        }
        for (col, list) in [(2, &bear_inv), (4, &bull_inv)] {
            if let Some(z) = list.last() { out[col][t] = z.top; out[col + 1][t] = z.bot; }
        }
    }
    Ok(out)
}

const PD: &[ParamSpec] = &[int("length", "Swing length", 50., 5., 200.)];
fn premium_discount(b: &Bars, prm: &Params) -> Output {
    let (s, m) = (n(prm, "length"), b.len());
    let ev = legs(&b.high, &b.low, s);
    let mut o = vec![vec![NA; m]; 7];
    let (mut top, mut bot) = (NA, NA);
    for t in 0..m {
        if !top.is_nan() { top = b.high[t].max(top); }
        if !bot.is_nan() { bot = b.low[t].min(bot); }
        if !top.is_nan() && !bot.is_nan() {
            for (k, v) in [top, 0.95 * top + 0.05 * bot, 0.525 * top + 0.475 * bot, 0.525 * bot + 0.475 * top, 0.95 * bot + 0.05 * top, bot, (top + bot) / 2.].into_iter().enumerate() { o[k][t] = v; }
        }
        // the swing step runs after the zones, so a swing confirmed on bar t first moves them on bar t + 1
        if ev[t] == 1 { bot = b.low[t - s]; } else if ev[t] == -1 { top = b.high[t - s]; }
    }
    Ok(o)
}

fn ote(b: &Bars, prm: &Params) -> Output {
    let (s, m) = (n(prm, "length"), b.len());
    let ev = legs(&b.high, &b.low, s);
    let mut o = vec![vec![NA; m]; 6]; // bull_top, bull_bottom, bull_mid, bear_top, bear_bottom, bear_mid
    let (mut hi, mut lo, mut last_high) = (NA, NA, false);
    for t in 0..m {
        if ev[t] == 1 { lo = b.low[t - s]; last_high = false; } else if ev[t] == -1 { hi = b.high[t - s]; last_high = true; }
        if hi.is_nan() || lo.is_nan() { continue; }
        let r = hi - lo;
        if last_high { (o[0][t], o[1][t], o[2][t]) = (hi - 0.62 * r, hi - 0.79 * r, hi - 0.705 * r); }
        else { (o[3][t], o[4][t], o[5][t]) = (lo + 0.79 * r, lo + 0.62 * r, lo + 0.705 * r); }
    }
    Ok(o)
}

/// Strong / Weak High-Low: the trailing top / bottom of premium_discount and the swing trend as it stood before bar t
/// (LuxAlgo draws the labels in updateTrailingExtremes / drawHighLowSwings, before getCurrentStructure / displayStructure).
fn smc_strong_weak(b: &Bars, prm: &Params) -> Output {
    let pd = premium_discount(b, prm)?;
    let trend = smc_bos_choch(b, prm)?.swap_remove(6);
    let bias = (0..b.len()).map(|t| if t == 0 { NA } else { trend[t - 1] }).collect();
    Ok(vec![pd[0].clone(), pd[5].clone(), bias])
}

pub const DEFS: &[IndicatorDef] = &[
    def("smc_swings", "SMC Swing Points (LuxAlgo)", LEN, &["bull", "bear", "swing_high", "swing_low"], "LuxAlgo Smart Money Concepts leg() swings (TradingView open source, CC BY-NC-SA 4.0). size = length. State leg starts at 0. On each bar t >= size: newLegHigh = high[t-size] > max(high[t-size+1..t]); newLegLow = low[t-size] < min(low[t-size+1..t]); if newLegHigh then leg = 0 (bearish), else if newLegLow then leg = 1 (bullish); otherwise leg is kept. If leg changed from 0 to 1 on bar t, a swing low is confirmed with level low[t-size]. If it changed from 1 to 0, a swing high is confirmed with level high[t-size]. bull = 1 on a swing-low confirmation bar, else 0. bear = 1 on a swing-high confirmation bar, else 0. Both are blank before bar size. swing_high and swing_low hold the latest confirmed level, carried forward (NaN until the first one). Pine runs this on bar t, so the level appears on its confirmation bar t, not on bar t-size. There is no left-side condition, and the swing is the bar `size` back, not the most extreme bar of the leg. Both are as published. Adaptation: the LuxAlgo input minimum of 10 is lowered to 2 for daily bars. Sources: Smart Money Concepts [LuxAlgo] (TradingView open-source script PUB;6daafb2cabe6419d98ae25229d2327f8, functions leg/startOfNewLeg/getCurrentStructure; https://www.tradingview.com/script/CnB3fSph-Smart-Money-Concepts-LUX/, CC BY-NC-SA 4.0); LuxAlgo Library: Smart Money Concepts (SMC) (https://www.luxalgo.com/library/indicator/smart-money-concepts-smc/).", &[50, 50, X, X], smc_swings),
    def("smc_bos_choch", "SMC Break of Structure / CHoCH (LuxAlgo)", LEN, &["bull_bos", "bull_choch", "bear_bos", "bear_choch", "bull_level", "bear_level", "trend"], "LuxAlgo Smart Money Concepts displayStructure() swing layer. Swing pivots come from the smc_swings leg rule with the same length. When a swing high is confirmed, hi = high[t-size] and hi_crossed = false. When a swing low is confirmed, lo = low[t-size] and lo_crossed = false. bias starts at 0. On each bar t, after updating the pivots: Bull: if close[t] > hi[t] and close[t-1] <= hi[t-1] (ta.crossover against the level as it stood on each bar; any NaN comparison is false) and not hi_crossed, then the event is a CHoCH if bias == -1, else a BOS (including bias 0). Set hi_crossed = true, bias = +1, bull_level[t] = hi. Bear, checked after bull on the same bar: if close[t] < lo[t] and close[t-1] >= lo[t-1] and not lo_crossed, then the event is a CHoCH if bias == +1, else a BOS. Set lo_crossed = true, bias = -1, bear_level[t] = lo. bull_bos, bull_choch, bear_bos and bear_choch are 1 on their event bar, else 0, and blank before bar size. bull_level and bear_level are the broken level on the event bar, else NaN. trend = bias after the bar (+1, -1 or 0). The script is causal as published (bar-by-bar Pine). Adaptation: the LuxAlgo input minimum of 10 is lowered to 2. Only the swing layer is reproduced: LuxAlgo's 5-bar internal layer, its extra filters and order-block storage are not part of this indicator. Sources: Smart Money Concepts [LuxAlgo] (TradingView open-source script PUB;6daafb2cabe6419d98ae25229d2327f8, functions getCurrentStructure/displayStructure; https://www.tradingview.com/script/CnB3fSph-Smart-Money-Concepts-LUX/, CC BY-NC-SA 4.0); LuxAlgo Library: Smart Money Concepts (SMC), BOS = break with trend, CHoCH = break against trend (https://www.luxalgo.com/library/indicator/smart-money-concepts-smc/).", &[50, 50, 50, 50, X, X, 50], smc_bos_choch),
    def("smc_equal_hl", "SMC Equal Highs / Lows (LuxAlgo)", EQ, &["bull_eql", "bear_eqh", "eql_level", "eqh_level"], "LuxAlgo Smart Money Concepts getCurrentStructure(equalHighsLowsLength, equalHighLow=true). It runs its own leg() with size = length (default 3), using the same rule as smc_swings. ATR = Pine ta.atr(atr_length): true range tr[0] = high-low, else max(high-low, |high-close[t-1]|, |low-close[t-1]|). Smoothing is RMA: seeded at bar atr_length-1 with the SMA of the first atr_length true ranges, then r = (tr + (n-1)*r_prev)/n. ATR is NaN before the seed. On a swing-low confirmation at bar t with level L = low[t-size]: if |prevEqualLow - L| < threshold*ATR[t], then bull_eql = 1 and eql_level[t] = L. In every case prevEqualLow = L afterwards. On a swing-high confirmation with H = high[t-size]: if |prevEqualHigh - H| < threshold*ATR[t], then bear_eqh = 1 and eqh_level[t] = H. Then prevEqualHigh = H. A NaN previous level or NaN ATR never triggers, and the inequality is strict. Signals are 1 on the event bar, else 0, and blank before bar max(length, atr_length-1). Levels are the new pivot level on the event bar, else NaN. EQH counts as bear and EQL as bull, matching the LuxAlgo label colours. The script is causal as published: the event appears on the confirmation bar t. Adaptation: atr_length is exposed as a parameter; LuxAlgo hard-codes 200. Liquidity sweep or invalidation of the pool is not part of the published EQH/EQL rule and is not added; see the existing liquidity_sweep. Sources: Smart Money Concepts [LuxAlgo] (TradingView open-source script PUB;6daafb2cabe6419d98ae25229d2327f8, getCurrentStructure equalHighLow branch, atrMeasure = ta.atr(200), inputs Bars Confirmation 3 / Threshold 0.1; https://www.tradingview.com/script/CnB3fSph-Smart-Money-Concepts-LUX/, CC BY-NC-SA 4.0); LuxAlgo Library: Smart Money Concepts (SMC) (https://www.luxalgo.com/library/indicator/smart-money-concepts-smc/).", &[199, 199, X, X], smc_equal_hl),
    def("order_block", "Order Blocks (SMC)", OB, &["bull", "bear", "bull_mitigated", "bear_mitigated", "bull_top", "bull_bottom", "bear_top", "bear_bottom"], "Inputs o,h,l,c; n = swing_length. Process each bar t in order 0..N-1 in exactly these steps (this order is taken from smc.ob). Step 1, update existing OBs using bar t. Each OB keeps its own state {top, bottom, breaker=false}. Active bullish OB (breaker=false): if (close_mitigation ? min(o[t],c[t]) : l[t]) < bottom, set breaker=true and bull_mitigated[t]=1. Bullish OB that is already a breaker: if h[t] > top, drop it. Active bearish OB: if (close_mitigation ? max(o[t],c[t]) : h[t]) > top, set breaker=true and bear_mitigated[t]=1. Bearish OB that is already a breaker: if l[t] < bottom, drop it. An OB created on bar t is never tested on bar t. Step 2, swing confirmation. Let p = t-n. If p >= 0 and p-n+1 >= 0: if h[p] == max(h[p-n+1..t]) (ties count), set last_sh = p; else if l[p] == min(l[p-n+1..t]), set last_sl = p. A high takes precedence over a low on the same p. Step 3, new OBs. Bullish: if last_sh exists, c[t] > h[last_sh] and last_sh is not yet crossed, mark last_sh crossed. Let k = the last bar (largest index on ties) with the minimum low among bars last_sh+1..t-1; if that range is empty, k = t-1. Create a bullish OB {top=h[k], bottom=l[k]} and set bull[t]=1. Bearish (mirror): if last_sl exists, c[t] < l[last_sl] and last_sl is not crossed, mark it crossed. Let k = the last bar with the maximum high in last_sl+1..t-1. Create {top=h[k], bottom=l[k]} and set bear[t]=1. Only the newest confirmed swing is tested. Once crossed, a swing never makes another OB. One crossed-set covers both swing highs and swing lows. Step 4, levels. bull_top/bull_bottom are top/bottom of the most recently created bullish OB whose breaker is still false, else NaN. bear_top/bear_bottom are the same for bearish OBs. Signals are 1 on the event bar, else 0. Adaptation (causality): smc.py finds swings with high == high.shift(-n).rolling(2n).max(). That looks n bars ahead (window p-n+1..p+n). Here the swing is used only from its confirmation bar p+n on. smc.py's alternation filter (drop consecutive same-type swings, keep the more extreme) and its first/last endpoint flip need future swings, so they are not applied. smc.py writes OB values back on candle k and zeroes them when the breaker is invalidated, so its output repaints. Here the event is reported on breakout bar t and never erased. smc.py stores state in per-candle arrays, so a bullish and a bearish OB on the same candle share one breaker flag, and a new OB on a candle that is already a breaker (same side, e.g. mitigated on the same breakout bar) is born as a breaker. Here each OB has its own state and every new OB starts with breaker=false, for either case. The volume outputs (OBVolume, Percentage) are dropped because the data is daily OHLC with no volume. Never back-dated: c[p+n] <= h[p+n] <= h[p], so the breakout bar is always after confirmation. Sources: smart-money-concepts smartmoneyconcepts/smc.py: swing_highs_lows (lines 137-220) and ob (lines 376-571), master @ 1b62fd6c (https://github.com/joshyattridge/smart-money-concepts/blob/master/smartmoneyconcepts/smc.py, MIT); LuxAlgo Smart Money Concepts Pine v5 source, cross-check only (storeOrdeBlock/deleteOrderBlocks/displayStructure; https://www.tradingview.com/script/CnB3fSph-Smart-Money-Concepts-SMC-LuxAlgo/, CC BY-NC-SA 4.0).", &[0, 0, 0, 0, X, X, X, X], order_block),
    def("breaker_block", "Breaker Blocks (SMC)", OB, &["bull", "bear", "bull_top", "bull_bottom", "bear_top", "bear_bottom"], "Uses exactly the same per-bar order-block state machine as order_block (same params, same step order, same causal swings). A breaker is an order block that failed (smc.ob internal 'breaker' flag). bear[t]=1 on the bar where an active bullish OB is mitigated: (close_mitigation ? min(o[t],c[t]) : l[t]) < its bottom. That OB becomes a bearish breaker zone [bottom, top] = [l[k], h[k]] of its OB candle. On a later bar (never the mitigation bar itself), the bearish breaker is removed when h[t] > top. This test always uses the wick, whatever close_mitigation is, as in smc.py. bull[t]=1 on the bar where an active bearish OB is mitigated: (close_mitigation ? max(o[t],c[t]) : h[t]) > its top. It becomes a bullish breaker [bottom, top], removed on a later bar when l[t] < bottom. Within bar t, existing breakers are tested for removal before OBs are tested for mitigation, and new OBs are created after both. bull_top/bull_bottom (bear_top/bear_bottom) are the newest still-live bullish (bearish) breaker, else NaN. Signals are 1 on the event bar, else 0. Identity check: bear == order_block.bull_mitigated and bull == order_block.bear_mitigated. Adaptation: all order_block causality adaptations apply: swings are used from confirmation bar p+n, no alternation filter, no back-dating, no repainting reset. The side naming (a failed bullish OB is a bearish breaker, and the reverse) follows the ICT/LuxAlgo +BB/-BB convention. smc.py tracks the breaker state but does not label a side. Sources: smart-money-concepts smartmoneyconcepts/smc.py: ob() breaker / mitigated_index logic (lines 376-571), master @ 1b62fd6c (https://github.com/joshyattridge/smart-money-concepts/blob/master/smartmoneyconcepts/smc.py, MIT); LuxAlgo Breaker Blocks with Signals, description only (naming convention +BB/-BB; https://www.luxalgo.com/library/indicator/breaker-blocks-with-signals/).", &[0, 0, X, X, X, X], breaker_block),
    def("ifvg", "Inversion Fair Value Gaps (LuxAlgo)", IF, &["bull", "bear", "bull_top", "bull_bottom", "bear_top", "bear_bottom"], "LuxAlgo Inversion Fair Value Gaps (IFVG), TradingView open source (CC BY-NC-SA 4.0); the published script is already causal (it reads only bars t, t-1, t-2), so no adaptation is needed. Per bar t, in this order: (1) threshold thr = ATR(200)*atr_multi, where ATR(200) = Wilder RMA of true range (bar 0 TR = high-low; first value at bar 199 = simple mean of TR[0..199]); while ATR(200) is undefined (bars 0..198) thr = sum(high-low over bars 0..t)/(t+1), NOT multiplied. (2) Detection (t>=2): bullish FVG if low[t] > high[t-2] and close[t-1] > high[t-2] and |low[t]-high[t-2]| > thr -> append {top=low[t], bot=high[t-2]} to the bullish FVG list; bearish FVG if high[t] < low[t-2] and close[t-1] < low[t-2] and |low[t-2]-high[t]| > thr -> append {top=low[t-2], bot=high[t]} to the bearish FVG list. (3) Inversion, bullish list then bearish list: if the list holds >=100 items drop its oldest; then scan newest to oldest: a bullish FVG with min(open,close)[t] < bot, or a bearish FVG with max(open,close)[t] > top, is removed and appended to that side's inverted list (bullish -> 'bearish IFVG' list, bearish -> 'bullish IFVG' list); the new state starts as 'fresh'. (4) Signals, bearish-IFVG list then bullish-IFVG list: if the list holds >=100 items drop its oldest; scan newest to oldest reading each zone's state before changing it: a fresh zone becomes armed (no signal, no removal this bar); for an armed bearish IFVG (former bullish FVG): bear signal if close[t] < bot and bot <= R < top, R = high[t] when wick=1 else close[t-1]; it is removed if max(open,close)[t] > top; for an armed bullish IFVG (former bearish FVG): bull signal if close[t] > top and bot < R <= top, R = low[t] when wick=1 else close[t-1]; it is removed if min(open,close)[t] < bot. Outputs: bull/bear = 1 if any zone of that list fired on bar t, else 0 (from bar 0); bull_top/bull_bottom = top/bot of the newest (last appended) still-listed bullish IFVG, bear_top/bear_bottom likewise for bearish IFVGs, NaN when the list is empty; values reflect the state after all four steps of bar t. Buffer 100 per list is the published constant; the 'Show Last' input is drawing only. Sources: Inversion Fair Value Gaps (IFVG) [LuxAlgo], TradingView open-source script (https://www.tradingview.com/script/8FHucjY6-Inversion-Fair-Value-Gaps-IFVG-LuxAlgo/, CC BY-NC-SA 4.0); Pine mirrors sappbr/pine-code indicators/lux-ifvg.pine (core logic identical to the second mirror; its trailing 'Connector Outputs' block is a third-party addition) and austindd/trading-view-scripts fair-value-gaps/lux-algo-ifvg-script.ps.", &[0, 0, X, X, X, X], ifvg),
    def("premium_discount", "Premium / Discount / Equilibrium Zones (LuxAlgo SMC)", PD, &["premium_top", "premium_bottom", "eq_top", "eq_bottom", "discount_top", "discount_bottom", "equilibrium"], "LuxAlgo Smart Money Concepts premium/discount/equilibrium zones (TradingView open source, CC BY-NC-SA 4.0), using the script's own per-bar order updateTrailingExtremes -> drawPremiumDiscountZones -> getCurrentStructure(length). State: leg = 0 (bearish) at start; T (trailing top) and B (trailing bottom) undefined. Per bar t: (1) if T defined, T = max(high[t], T); if B defined, B = min(low[t], B). (2) Outputs from T and B as they stand now (all NaN until both are defined): premium_top = T, premium_bottom = 0.95*T + 0.05*B; eq_top = 0.525*T + 0.475*B, eq_bottom = 0.525*B + 0.475*T; discount_top = 0.95*B + 0.05*T, discount_bottom = B; equilibrium = (T+B)/2. (3) Swing step (LuxAlgo leg(size)), only when t >= length: if high[t-length] > max(high[t-length+1..t]) then leg = 0, else if low[t-length] < min(low[t-length+1..t]) then leg = 1; if leg changed this bar: change to 1 (new swing low) sets B = low[t-length], change to 0 (new swing high) sets T = high[t-length]. Because step 3 runs after step 2, a swing confirmed on bar t first affects the zones on bar t+1, exactly as LuxAlgo draws them. Causality: a swing at bar t-length is only known after length later bars and is applied on its confirmation bar, never back-dated. Adaptation: LuxAlgo initialises trailing.top/bottom as na; this rule keeps each extreme undefined until its first confirmed swing (zones NaN until a swing high and a swing low have both been confirmed). Note the LuxAlgo premium and discount boxes are the outer 5% bands of the range; the ICT-style split (price above / below 50%) is available by comparing price with equilibrium. Default length 50 is the published Swing Structure default (LuxAlgo minval 10; 5 is its internal-structure size). Sources: Smart Money Concepts (SMC) [LuxAlgo], TradingView open-source script (https://www.tradingview.com/script/CnB3fSph-Smart-Money-Concepts-SMC-LuxAlgo/, CC BY-NC-SA 4.0); Pine mirror RouZuoMong2003/SmartMoneyConcepts-LuxAlgo SmartMoneyConcepts_LuxAlgo.pine (leg(), getCurrentStructure(), updateTrailingExtremes(), drawPremiumDiscountZones()).", &[X; 7], premium_discount),
    def("smc_strong_weak", "SMC Strong / Weak High-Low (LuxAlgo)", PD, &["trailing_high", "trailing_low", "bias"], "LuxAlgo Smart Money Concepts Strong/Weak High/Low (TradingView open source, CC BY-NC-SA 4.0): updateTrailingExtremes / drawHighLowSwings. trailing_high and trailing_low are premium_discount's T and B with the same length: per bar t, T = max(high[t], T) and B = min(low[t], B) first, then a swing high (low) confirmed on bar t by the leg(length) rule resets T (B) to its level for bar t + 1; NaN until a swing high and a swing low have both been confirmed. bias = the swing trend of smc_bos_choch with the same length as it stood at the end of bar t - 1 (+1, -1 or 0; NaN on bar 0), because LuxAlgo draws the labels before this bar's structure update. Reading (drawing only, at the last bar): the top is the Strong High when bias = -1, else the Weak High; the bottom is the Strong Low when bias = +1, else the Weak Low; LuxAlgo draws only the latest top and bottom, from the bar that made each extreme to the right edge. Causal as published: every value uses bars 0..t only. Adaptation: the trailing values stay NaN until both swings exist (LuxAlgo starts them as na). Sources: Smart Money Concepts [LuxAlgo] (TradingView open-source script PUB;6daafb2cabe6419d98ae25229d2327f8, functions updateTrailingExtremes, drawHighLowSwings; https://www.tradingview.com/script/CnB3fSph-Smart-Money-Concepts-SMC-LuxAlgo/, CC BY-NC-SA 4.0).", &[X; 3], smc_strong_weak),
    def("ote", "Optimal Trade Entry Zone (ICT 0.62-0.79)", PD, &["bull_top", "bull_bottom", "bull_mid", "bear_top", "bear_bottom", "bear_mid"], "OTE band = 0.62 to 0.79 retracement of the latest swing leg, 0.705 midpoint (ICT convention; LuxAlgo library: anchor swing low -> swing high for longs, swing high -> swing low for shorts). Swings are the LuxAlgo Smart Money Concepts leg(size) pivots: state leg = 0 at start; per bar t with t >= length: if high[t-length] > max(high[t-length+1..t]) then leg = 0, else if low[t-length] < min(low[t-length+1..t]) then leg = 1; a change of leg to 1 confirms a swing low L = low[t-length], a change to 0 confirms a swing high H = high[t-length]; pivots therefore alternate. Per bar t, after the swing step: if both an H and an L have been confirmed and the most recent confirmed pivot is a high (up leg L -> H, R = H - L): bull_top = H - 0.62*R, bull_bottom = H - 0.79*R, bull_mid = H - 0.705*R, bear_* = NaN; if the most recent is a low (down leg H -> L): bear_bottom = L + 0.62*R, bear_top = L + 0.79*R, bear_mid = L + 0.705*R, bull_* = NaN; all NaN before both exist. A zone stays until the next confirmed pivot replaces it. Adaptation: the pivot at bar t-length is confirmed by length right-side bars and is reported on its confirmation bar t, never back-dated; the swing detector is borrowed from LuxAlgo SMC because no OTE source fixes one; no displacement/structure-break/sweep prerequisite and no touch signal are applied, since no published rule fixes them (rules can compare price with the levels). The LuxAlgo page treats OTE as a setup only after an impulse leg that breaks structure, usually following a liquidity sweep; this band is drawn on every leg() swing leg, so a band alone is not a source-qualified OTE setup. Sources: LuxAlgo Library - Optimal Trade Entry (0.62/0.705/0.79 ICT convention; anchoring; https://www.luxalgo.com/library/concept/optimal-trade-entry/); Pine mirror RouZuoMong2003/SmartMoneyConcepts-LuxAlgo SmartMoneyConcepts_LuxAlgo.pine (leg() swing detector, CC BY-NC-SA 4.0).", &[X; 6], ote),
];
/// (id, platform names, AmiBroker names): one row per id in DEFS.
pub const SOURCES: &[(&str, &str, &str)] = &[
    ("smc_swings", "TradingView (community): Smart Money Concepts [LuxAlgo] swing points", ""),
    ("smc_bos_choch", "TradingView (community): Smart Money Concepts [LuxAlgo] BOS / CHoCH", ""),
    ("smc_equal_hl", "TradingView (community): Smart Money Concepts [LuxAlgo] equal highs / lows", ""),
    ("order_block", "smart-money-concepts (Python, MIT): smc.ob order blocks", ""),
    ("breaker_block", "smart-money-concepts (Python, MIT): smc.ob breaker blocks", ""),
    ("ifvg", "TradingView (community): Inversion Fair Value Gaps (IFVG) [LuxAlgo]", ""),
    ("premium_discount", "TradingView (community): Smart Money Concepts [LuxAlgo] premium / discount zones", ""),
    ("smc_strong_weak", "TradingView (community): Smart Money Concepts [LuxAlgo] strong / weak high-low", ""),
    ("ote", "LuxAlgo Library: Optimal Trade Entry (ICT)", ""),
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lab::catalogue::run;
    use crate::lab::testdata::{assert_series, compiles, cut, from_ohlc, go, walk};
    const N: f64 = f64::NAN;
    fn bars(o: &[f64], h: &[f64], l: &[f64], c: &[f64]) -> Bars { from_ohlc(&(0..c.len()).map(|i| (o[i], h[i], l[i], c[i])).collect::<Vec<_>>()) }
    fn prm(kv: &[(&str, f64)]) -> Params { kv.iter().map(|(k, v)| (k.to_string(), *v)).collect() }
    fn check(got: &[Vec<f64>], want: &[&[f64]]) { assert_eq!(got.len(), want.len()); for (g, w) in got.iter().zip(want) { assert_series(g, w); } }
    // length 2: swing low 0.5 (bar 0) on bar 2, swing high 5 (bar 2) on bar 4, swing low 1 (bar 4) on bar 6, swing high 6 (bar 7) on bar 9
    const H: [f64; 10] = [1., 2., 5., 3., 2., 3., 4., 6., 3., 0.8];
    const L: [f64; 10] = [0.5, 1., 4., 2., 1., 2.5, 3., 4., 0.5, 0.2];
    const C: [f64; 10] = [0.8, 1.5, 4.5, 2.5, 1.5, 2.8, 3.5, 5.5, 0.7, 0.5];

    #[test] fn swings_are_reported_on_their_confirmation_bar() {
        let o = go("smc_swings", &bars(&C, &H, &L, &C), &[("length", 2.)]);
        check(&o, &[&[N, N, 1., 0., 0., 0., 1., 0., 0., 0.], &[N, N, 0., 0., 1., 0., 0., 0., 0., 1.],
            &[N, N, N, N, 5., 5., 5., 5., 5., 6.], &[N, N, 0.5, 0.5, 0.5, 0.5, 1., 1., 1., 1.]]);
    }
    #[test] fn bos_then_choch_once_per_level() {
        // bar 7 closes 5.5 over 5 with bias 0 → BOS; bar 8 closes 0.7 under 1 while bullish → CHoCH; bar 9 no repeat
        let o = go("smc_bos_choch", &bars(&C, &H, &L, &C), &[("length", 2.)]);
        let z = [N, N, 0., 0., 0., 0., 0., 0., 0., 0.];
        check(&o, &[&[N, N, 0., 0., 0., 0., 0., 1., 0., 0.], &z, &z, &[N, N, 0., 0., 0., 0., 0., 0., 1., 0.],
            &[N, N, N, N, N, N, N, 5., N, N], &[N, N, N, N, N, N, N, N, 1., N], &[N, N, 0., 0., 0., 0., 0., 1., -1., -1.]]);
        // a bearish break first, then a bullish break against it is a bullish CHoCH
        let (h, l) = ([5., 6., 9., 6., 5., 4., 3., 6., 9.5, 10.], [4., 5., 8., 3., 2., 2.5, 1.5, 5., 8., 9.]);
        let c = [4.5, 5.5, 8.5, 4., 2.5, 3., 1.8, 5.5, 9.2, 9.5];
        let o = go("smc_bos_choch", &bars(&c, &h, &l, &c), &[("length", 2.)]);
        assert_series(&o[2], &[N, N, 0., 0., 1., 0., 0., 0., 0., 0.]); // swing low 4 (bar 0) from bar 2, bar 4 closes 2.5 < 4 (bar 3 at 4) → bear BOS
        assert_series(&o[1], &[N, N, 0., 0., 0., 0., 0., 0., 1., 0.]); // swing high 9 (bar 2) from bar 4, close 9.2 > 9 → CHoCH
        assert_series(&o[6], &[N, N, 0., 0., -1., -1., -1., -1., 1., 1.]);
    }
    #[test] fn equal_highs_within_a_tenth_of_atr() {
        let (h, l) = ([1., 5., 3., 5.05, 3., 2.], [0.5, 4., 2., 4., 2., 1.]);
        let b = bars(&h, &h, &l, &h);
        let o = go("smc_equal_hl", &b, &[("length", 1.), ("atr_length", 2.)]);
        check(&o, &[&[N, 0., 0., 0., 0., 0.], &[N, 0., 0., 0., 1., 0.], &[N; 6], &[N, N, N, N, 5.05, N]]);
        assert_series(&go("smc_equal_hl", &b, &[("length", 1.), ("atr_length", 2.), ("threshold", 0.)])[1], &[N, 0., 0., 0., 0., 0.]); // strict <
        let flat = from_ohlc(&[(10., 11., 9., 10.); 4]);
        assert_series(&atr_pine(&flat, 3), &[N, N, 2., 2.]);
    }
    // swing_length 2: swing high 12 (bar 1) from bar 3; bar 5 closes 12.5 > 12 → bull OB on the lowest low of bars 2–4 (bar 3, [7, 10.5]);
    // bar 6's low 6.5 < 7 mitigates it (a bearish breaker); bar 7's high 11 > 10.5 drops the breaker
    const OO: [f64; 8] = [9.5, 10., 11., 8.5, 7.6, 10., 11., 9.5];
    const OH: [f64; 8] = [10., 12., 11., 10.5, 11., 13., 12., 11.];
    const OL: [f64; 8] = [9., 10., 8., 7., 7.5, 10., 6.5, 9.];
    const OC: [f64; 8] = [9.8, 11.5, 8.5, 7.5, 10., 12.5, 7., 10.];
    #[test] fn order_block_forms_on_the_break_and_mitigates_on_the_wick() {
        let b = bars(&OO, &OH, &OL, &OC);
        let z = [0.; 8];
        check(&go("order_block", &b, &[("swing_length", 2.)]), &[&[0., 0., 0., 0., 0., 1., 0., 0.], &z, &[0., 0., 0., 0., 0., 0., 1., 0.], &z,
            &[N, N, N, N, N, 10.5, N, N], &[N, N, N, N, N, 7., N, N], &[N; 8], &[N; 8]]);
        // body mitigation: bar 6's body bottom 7 is not below 7
        let o = go("order_block", &b, &[("swing_length", 2.), ("close_mitigation", 1.)]);
        assert_series(&o[2], &z);
        assert_series(&o[4], &[N, N, N, N, N, 10.5, 10.5, 10.5]);
    }
    #[test] fn bearish_order_block_mirrors() {
        let neg = |x: &[f64]| x.iter().map(|v| -v).collect::<Vec<_>>();
        let b = bars(&neg(&OO), &neg(&OL), &neg(&OH), &neg(&OC));
        let o = go("order_block", &b, &[("swing_length", 2.)]);
        assert_series(&o[1], &[0., 0., 0., 0., 0., 1., 0., 0.]);
        assert_series(&o[3], &[0., 0., 0., 0., 0., 0., 1., 0.]);
        assert_series(&o[6], &[N, N, N, N, N, -7., N, N]);
        assert_series(&o[7], &[N, N, N, N, N, -10.5, N, N]);
        let bb = go("breaker_block", &b, &[("swing_length", 2.)]);
        assert_series(&bb[0], &[0., 0., 0., 0., 0., 0., 1., 0.]);
        assert_series(&bb[2], &[N, N, N, N, N, N, -7., N]);
    }
    #[test] fn breaker_is_the_failed_order_block() {
        let b = bars(&OO, &OH, &OL, &OC);
        check(&go("breaker_block", &b, &[("swing_length", 2.)]), &[&[0.; 8], &[0., 0., 0., 0., 0., 0., 1., 0.], &[N; 8], &[N; 8],
            &[N, N, N, N, N, N, 10.5, N], &[N, N, N, N, N, N, 7., N]]);
        let w = walk(600, 3, false);
        for sl in [2., 5.] {
            let (ob, bb) = (go("order_block", &w, &[("swing_length", sl)]), go("breaker_block", &w, &[("swing_length", sl)]));
            assert_series(&bb[1], &ob[2]); assert_series(&bb[0], &ob[3]);
        }
    }
    #[test] fn ifvg_inverts_then_signals_on_the_retest() {
        // bull FVG [10.2, 12.4] on bar 2; bar 3's body closes under 10.2 → bearish IFVG (armed on bar 4); bar 6 closes 9.5 under it
        // with close[1] 10.6 inside → bear; bar 7's body top 12.6 > 12.4 removes it and inverts the bearish FVG of bar 4 [9.5, 12.4]
        let o = [10., 10.5, 12.5, 12.0, 9.0, 11.0, 10.4, 10.0];
        let c = [10., 12.0, 13.0, 9.0, 9.3, 10.6, 9.5, 12.6];
        let h = [10.2, 12.2, 13.2, 12.1, 9.5, 11.2, 10.6, 12.8];
        let l = [9.8, 10.4, 12.4, 8.9, 8.8, 10.3, 9.4, 9.9];
        for wk in [0., 1.] {
            check(&go("ifvg", &bars(&o, &h, &l, &c), &[("atr_multi", 0.), ("wick", wk)]), &[&[0.; 8], &[0., 0., 0., 0., 0., 0., 1., 0.],
                &[N, N, N, N, N, N, N, 12.4], &[N, N, N, N, N, N, N, 9.5], &[N, N, N, 12.4, 12.4, 12.4, 12.4, N], &[N, N, N, 10.2, 10.2, 10.2, 10.2, N]]);
        }
    }
    #[test] fn premium_discount_moves_the_bar_after_a_swing() {
        // length 2 (below the input minimum, so called directly): swing low 1 (bar 2) on bar 4, swing high 9 (bar 4) on bar 6
        let (h, l) = ([5., 4., 3., 5., 9., 6., 5., 6., 7.], [4., 3., 1., 2., 7., 5., 4., 5., 6.]);
        let m: Vec<f64> = h.iter().zip(&l).map(|(a, b)| (a + b) / 2.).collect();
        let o = premium_discount(&bars(&m, &h, &l, &m), &prm(&[("length", 2.)])).unwrap();
        let w = |v: f64| { let mut s = vec![N; 7]; s.extend([v, v]); s };
        for (k, v) in [9., 8.6, 5.2, 4.8, 1.4, 1., 5.].into_iter().enumerate() { assert_series(&o[k], &w(v)); }
    }
    #[test] fn ote_band_of_the_latest_leg() {
        // length 2: swing low 1 on bar 4, swing high 9 on bar 6 (up leg, R 8), swing low 3 (bar 7) on bar 9 (down leg, R 6)
        let (h, l) = ([5., 4., 3., 5., 9., 6., 5., 4.5, 6., 7.], [4., 3., 1., 2., 7., 5., 4., 3., 5., 6.]);
        let m: Vec<f64> = h.iter().zip(&l).map(|(a, b)| (a + b) / 2.).collect();
        let o = ote(&bars(&m, &h, &l, &m), &prm(&[("length", 2.)])).unwrap();
        let up = |v: f64| [N, N, N, N, N, N, v, v, v, N];
        let dn = |v: f64| { let mut s = [N; 10]; s[9] = v; s };
        check(&o, &[&up(9. - 0.62 * 8.), &up(9. - 0.79 * 8.), &up(9. - 0.705 * 8.), &dn(3. + 0.79 * 6.), &dn(3. + 0.62 * 6.), &dn(3. + 0.705 * 6.)]);
    }
    #[test] fn defaults_produce_events_on_long_history() {
        let w = walk(1500, 8, false);
        for (id, k) in [("smc_swings", 0), ("smc_swings", 1), ("smc_bos_choch", 6), ("order_block", 0), ("order_block", 1), ("ifvg", 2), ("premium_discount", 6), ("ote", 0)] {
            assert!(go(id, &w, &[])[k].iter().any(|v| v.is_finite() && *v != 0.), "{id}.{k}");
        }
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
        // value on bar k-1 must not change when bars k.. are removed; rounded copy forces equal highs/lows (ties)
        let w = walk(900, 5, false);
        let mut r = w.clone();
        for v in [&mut r.open, &mut r.high, &mut r.low, &mut r.close] { for x in v.iter_mut() { *x = x.round(); } }
        let small = [("length", 5.), ("swing_length", 2.), ("close_mitigation", 1.), ("wick", 1.), ("atr_multi", 0.), ("atr_length", 14.), ("threshold", 0.5)];
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
    #[test] fn strong_weak_is_the_trailing_range_with_the_previous_bar_trend() {
        let w = walk(600, 3, false);
        let q = prm(&[("length", 5.)]);
        let (sw, pd, bc) = (go("smc_strong_weak", &w, &[("length", 5.)]), go("premium_discount", &w, &[("length", 5.)]), go("smc_bos_choch", &w, &[("length", 5.)]));
        assert_series(&sw[0], &pd[0]); assert_series(&sw[1], &pd[5]);
        assert!(sw[2][0].is_nan()); assert_series(&sw[2][1..], &bc[6][..599]);
        assert!(sw[0].iter().zip(&sw[1]).filter(|(a, b)| a.is_finite() && b.is_finite()).all(|(a, b)| a >= b));
        assert!(sw[2].iter().any(|v| *v == 1.) && sw[2].iter().any(|v| *v == -1.), "{q:?}: both trends occur");
    }
    #[test] fn rules_can_read_every_output() {
        for d in DEFS { for o in d.outputs { compiles(d.id, o).unwrap_or_else(|e| panic!("{}.{o}: {e:?}", d.id)); } }
    }
    /// SMC_BARS=<csv date,open,high,low,close> SMC_OUT=<json> cargo test --lib lab::x_smc::tests::smc_dump -- --ignored --exact
    #[test] #[ignore] fn smc_dump() {
        let text = std::fs::read_to_string(std::env::var("SMC_BARS").expect("SMC_BARS")).expect("read SMC_BARS");
        let mut b = Bars::default();
        for (i, line) in text.lines().skip(1).filter(|s| !s.trim().is_empty()).enumerate() {
            let f: Vec<&str> = line.split(',').map(str::trim).collect();
            let x = |k: usize| f[k].parse::<f64>().unwrap_or(f64::NAN);
            b.date.push(f[0].to_string());
            b.open.push(x(1)); b.high.push(x(2)); b.low.push(x(3)); b.close.push(x(4)); b.end.push(i);
        }
        let num = |v: f64| if v.is_finite() { format!("{v:?}") } else { "null".into() };
        let body: Vec<String> = DEFS.iter().map(|d| {
            let out = run(d, &b, &Params::new()).unwrap_or_else(|e| panic!("{}: {e}", d.id));
            let cols: Vec<String> = d.outputs.iter().zip(&out).map(|(k, s)| format!("\"{k}\":[{}]", s.iter().map(|&v| num(v)).collect::<Vec<_>>().join(","))).collect();
            format!("\"{}\":{{{}}}", d.id, cols.join(","))
        }).collect();
        std::fs::write(std::env::var("SMC_OUT").expect("SMC_OUT"), format!("{{{}}}", body.join(","))).expect("write SMC_OUT");
    }
}
