//! Census group "misc" (2026-10-26, research: sv-census-work/research_misc.md). Formulas: see each definition string.
//! Skipped as UNCONFIRMED (no primary text, nothing invented): Demand Index after Sibbet's original and the MetaStock built-in
//! (only Sierra Chart's is built); Drummond Geometry envelopes, push, refresh and exhaust (proprietary course material; only
//! the PLdot is built); MIDAS Topfinder/Bottomfinder (the fitted volume D is set by eye and the intra-bar interpolation is in
//! an unread book appendix; only the MIDAS S/R curve is built); the Dalton reading of One Time Framing (Mind Over Markets
//! not retrieved; only Sierra Chart's is built); Sierra's Zig Zag mode 2 (bars + amount; its ZigZag2 source was not read).
use super::bars::{Bars, Breadth};
use super::catalogue::{get, int, len, n, p, real, run, IndicatorDef, Needs, Output, ParamSpec, Params, Placement};
use super::ta::{self, nz, NA};

#[allow(clippy::too_many_arguments)]
const fn def(id: &'static str, name: &'static str, category: &'static str, params: &'static [ParamSpec], outputs: &'static [&'static str], placement: Placement,
             needs: Needs, definition: &'static str, first: &'static [usize], sparse: bool, f: fn(&Bars, &Params) -> Output) -> IndicatorDef {
    IndicatorDef { id, name, category, params, outputs, placement, needs, definition, first, sparse, compute: Some(f), note: "" }
}
const PR: Placement = Placement::Price;
const PN: Placement = Placement::Pane;
const NO: Needs = Needs::NONE;
const V: Needs = Needs { volume: true, ..Needs::NONE };
const BR: Needs = Needs { breadth: true, ..Needs::NONE };
const BRD: &str = "Breadth";

fn br(b: &Bars) -> &Breadth { b.breadth.as_ref().expect("catalogue::run checks needs.breadth") }
fn div(a: f64, b: f64) -> f64 { if b == 0. || a.is_nan() || b.is_nan() { NA } else { a / b } }
fn zip(a: &[f64], b: &[f64], f: impl Fn(f64, f64) -> f64) -> Vec<f64> { a.iter().zip(b).map(|(&x, &y)| f(x, y)).collect() }

const ZZ: &[ParamSpec] = &[int("mode", "Calculation mode (Sierra Chart 0 mode 1 reversal %, 1 mode 3 reversal amount)", 0., 0., 1.),
    real("percent", "Reversal % (mode 1)", 0.5, 0.001, 100., 0.001), real("amount", "Reversal amount (mode 3, points)", 0.01, 0.0001, 1e9, 0.0001)];
fn zigzag_cum_volume(b: &Bars, prm: &Params) -> Output {
    let (m, v, h, l) = (b.len(), b.volume.as_deref().unwrap_or(&[]), &b.high, &b.low);
    let (pct, amt) = if p(prm, "mode") == 0. { (p(prm, "percent") / 100., 0.) } else { (0., p(prm, "amount")) };
    let (mut out, mut dir) = (vec![NA; m], vec![NA; m]);
    let Some(s) = (0..m).find(|&i| h[i].is_finite() && l[i].is_finite()) else { return Ok(vec![out, dir]) };
    // peak = (type +1 high / −1 low, value, bar), the Sierra peak that may still extend; conf = (bar, type) of the last final peak
    let (mut peak, mut conf, mut sum): (Option<(f64, f64, usize)>, Option<(usize, f64)>, f64) = (None, None, 0.);
    for i in s + 1..m {
        let (hi, lo, was) = (h[i], l[i], conf);
        match peak {
            None if hi > h[s] && lo > l[s] => { peak = Some((1., hi, i)); conf = Some((s, -1.)); }
            None if hi < h[s] && lo < l[s] => { peak = Some((-1., lo, i)); conf = Some((s, 1.)); }
            Some((t, pv, pi)) if t > 0. => {
                if lo < if pct > 0. { pv - pv * pct } else { pv - amt } { conf = Some((pi, 1.)); peak = Some((-1., lo, i)); } else if hi >= pv { peak = Some((1., hi, i)); }
            }
            Some((_, pv, pi)) => {
                if hi > if pct > 0. { pv + pv * pct } else { pv + amt } { conf = Some((pi, -1.)); peak = Some((1., hi, i)); } else if lo <= pv { peak = Some((-1., lo, i)); }
            }
            None => {}
        }
        let Some((q, t)) = conf else { continue };
        // a new confirmation re-sums the bars since that pivot, all of them already closed
        if was.map(|w| w.0) != Some(q) { sum = v[q + 1..=i].iter().map(|&x| nz(x)).sum(); } else { sum += nz(v[i]); }
        if v[i].is_finite() && hi.is_finite() && lo.is_finite() { out[i] = sum; dir[i] = -t; }
    }
    Ok(vec![out, dir])
}

const DIP: &[ParamSpec] = &[len("bs_length", "Buy/sell power length", 19.), len("bs_ma_length", "Buy/sell power average length", 19.), len("di_ma_length", "Demand Index average length", 30.)];
fn demand_index(b: &Bars, prm: &Params) -> Output {
    let (m, v, c) = (b.len(), b.volume.as_deref().unwrap_or(&[]), &b.close);
    let pr: Vec<f64> = (0..m).map(|i| b.high[i] + b.low[i] + 2. * c[i]).collect();
    let r0 = if m > 0 { b.high[0] - b.low[0] } else { NA }; // Sierra divides by RangeMA[0], which is the first bar's range
    let va = ta::ema(v, n(prm, "bs_length"));
    let (mut bp, mut sp) = (vec![NA; m], vec![NA; m]);
    for i in 1..m {
        let base = div(v[i], va[i]);
        if base.is_nan() || !(r0 > 0.) { continue; }
        let (a, z) = (pr[i], pr[i - 1]);
        let k = 0.375 * (a + z) / r0;
        bp[i] = base; sp[i] = base;
        if a < z { bp[i] /= (k * (z - a) / a).exp(); } else if a > z { sp[i] /= (k * (a - z) / z).exp(); }
    }
    let k = n(prm, "bs_ma_length");
    let di = zip(&ta::ema(&bp, k), &ta::ema(&sp, k), |bm, sm| if bm.is_nan() || sm.is_nan() { NA } else if bm > sm { 100. * (1. - sm / bm) } else if bm < sm { -100. * (1. - bm / sm) } else if bm == 0. { NA } else { 0. });
    let ma = ta::sma(&di, n(prm, "di_ma_length"));
    Ok(vec![di, ma])
}

const RP: &[ParamSpec] = &[len("window", "Window (bars)", 20.), int("offset", "Offset (bars)", 0., 0., 2000.),
    int("type", "Type (Skender PivotPointType 0 Standard, 1 Camarilla, 2 Demark, 3 Fibonacci, 4 Woodie)", 0., 0., 4.)];
fn rolling_pivots(b: &Bars, prm: &Params) -> Output {
    let (w, o, t, m) = (n(prm, "window"), n(prm, "offset"), n(prm, "type"), b.len());
    let lab = [0, 5, 4, 1, 2][t]; // periods::levels types: Traditional, Camarilla, DM, Fibonacci, Woodie
    let mut out = vec![vec![NA; m]; 9];
    for i in w + o..m {
        let win = i - w - o..i - o;
        if win.clone().any(|j| b.high[j].is_nan() || b.low[j].is_nan()) { continue; }
        let (hh, ll) = win.fold((f64::MIN, f64::MAX), |(a, z), j| (a.max(b.high[j]), z.min(b.low[j])));
        let (c, op) = (b.close[i - o - 1], b.open[i]);
        let lv = super::periods::levels(lab, hh, ll, c, op, op); // Skender's DM compares the close with the current bar's open
        let r4 = if t == 1 { (lv[4], lv[9]) } else { (NA, NA) };
        for (k, x) in [if t == 1 { c } else { lv[0] }, lv[1], lv[2], lv[3], r4.0, lv[6], lv[7], lv[8], r4.1].into_iter().enumerate() { out[k][i] = x; }
    }
    Ok(out)
}

fn pldot(b: &Bars, _: &Params) -> Output {
    let next = ta::sma(&b.hlc3(), 3);
    Ok(vec![std::iter::once(NA).chain(next.iter().copied()).take(b.len()).collect(), next])
}

const MD: &[ParamSpec] = &[int("anchor_date", "Launch date (YYYYMMDD; from the first bar on or after it)", 20000101., 19000101., 21001231.), int("source", "Price (0 hl2, 1 close)", 0., 0., 1.)];
fn midas(b: &Bars, prm: &Params) -> Output {
    let mut q = prm.clone();
    q.insert("anchor".into(), 5.); q.insert("mult".into(), 1.);
    let src = if p(prm, "source") == 1. { b.close.clone() } else { b.hl2() };
    Ok(vec![super::volume2::vwap_src(b, &q, &src)?.swap_remove(0)])
}

const LS: &[ParamSpec] = &[len("length", "Swing length (pivot bars each side)", 5.), int("mode", "Mode (LuxAlgo 0 Only Wicks, 1 Only Outbreaks & Retest, 2 Wicks + Outbreaks & Retest)", 0., 0., 2.)];
struct Level { prc: f64, bix: usize, brk: bool, wic: bool }
/// Records a sweep on bar i; with several levels swept on one bar the one nearest the close is reported.
fn hit(out: &mut [Vec<f64>], side: usize, i: usize, lvl: f64, c: f64) {
    out[side][i] = 1.;
    let cur = out[side + 2][i];
    if cur.is_nan() || (lvl - c).abs() < (cur - c).abs() { out[side + 2][i] = lvl; }
}
fn liquidity_sweep(b: &Bars, prm: &Params) -> Output {
    let (k, mode, m) = (n(prm, "length"), n(prm, "mode"), b.len());
    let (ow, oo) = (mode == 0, mode == 1);
    let piv = [super::price::pivots(&b.high, k, k, true), super::price::pivots(&b.low, k, k, false)];
    let mut out = vec![vec![NA; m]; 4]; // bull, bear, bull_level, bear_level
    let mut lists: [Vec<Level>; 2] = [Vec::new(), Vec::new()];
    for i in 0..m {
        if i >= 2 * k { out[0][i] = 0.; out[1][i] = 0.; }
        let (h, l, c) = (b.high[i], b.low[i], b.close[i]);
        for (s, list) in lists.iter_mut().enumerate() {
            if !piv[s][i].is_nan() { list.push(Level { prc: piv[s][i], bix: i - k, brk: false, wic: false }); }
            let hi = s == 0; // a pivot high's wick sweep is bearish (side 1), its retest from above bullish (side 0)
            list.retain_mut(|g| {
                let (beyond, back) = if hi { (c > g.prc, c < g.prc) } else { (c < g.prc, c > g.prc) };
                let (pierce, retest) = if hi { (h > g.prc, l < g.prc) } else { (l < g.prc, h > g.prc) };
                let mut gone = false;
                if !g.brk {
                    if beyond { if ow { gone = true; } else { g.brk = true; } }
                    if !oo && !g.wic && pierce && back { hit(&mut out, if hi { 1 } else { 0 }, i, g.prc, c); g.wic = true; }
                } else {
                    if back { gone = true; }
                    if !ow && retest && beyond { hit(&mut out, if hi { 0 } else { 1 }, i, g.prc, c); gone = true; }
                }
                !(gone || i - g.bix > 2000)
            });
        }
    }
    Ok(out)
}

fn ad_percent(b: &Bars, _: &Params) -> Output {
    let r = br(b);
    Ok(vec![(0..b.len()).map(|i| div(100. * (r.adv[i] - r.dec[i]), r.adv[i] + r.dec[i] + r.unch[i])).collect()])
}
/// Advancing minus declining volume over total volume; unchanged volume counts where the source has it (US only).
fn vol_share(r: &Breadth, scale: f64) -> Vec<f64> { (0..r.adv_vol.len()).map(|i| div(scale * (r.adv_vol[i] - r.dec_vol[i]), r.adv_vol[i] + r.dec_vol[i] + nz(r.unch_vol[i]))).collect() }
fn ad_volume_percent(b: &Bars, _: &Params) -> Output { Ok(vec![vol_share(br(b), 100.)]) }
fn rana(r: &Breadth) -> Vec<f64> { zip(&r.adv, &r.dec, |a, d| div(1000. * (a - d), a + d)) }
/// DecisionPoint ITBM/ITVM: ema(McClellan oscillator + its 19-day "10%" EMA, 20), signal ema(·, 10).
fn dp_momentum(x: &[f64]) -> Vec<Vec<f64>> {
    let it = ta::ema(&zip(&ta::ema(x, 19), &ta::ema(x, 39), |a, z| 2. * a - z), 20);
    let sig = ta::ema(&it, 10);
    vec![it, sig]
}
fn itbm(b: &Bars, _: &Params) -> Output { Ok(dp_momentum(&rana(br(b)))) }
fn itvm(b: &Bars, _: &Params) -> Output { Ok(dp_momentum(&vol_share(br(b), 1000.))) }
const STO: &[ParamSpec] = &[int("basis", "Basis (DecisionPoint 0 STO-B issues, 1 STO-V volume)", 0., 0., 1.)];
fn swenlin_sto(b: &Bars, prm: &Params) -> Output {
    let r = br(b);
    let x = if p(prm, "basis") == 1. { zip(&r.adv_vol, &r.dec_vol, |a, d| div(1000. * (a - d), a + d)) } else { rana(r) };
    Ok(vec![ta::sma(&ta::ema(&x, 4), 5)])
}

fn renko_predictor(b: &Bars, prm: &Params) -> Output {
    let r = run(&get("renko").ok_or("renko missing")?, b, prm)?;
    let s = super::x_atlas::renko_box(b, prm);
    Ok(vec![zip(&r[3], &s, |h, s| h + s), zip(&r[4], &s, |l, s| l - s)])
}

fn one_time_framing(b: &Bars, _: &Params) -> Output {
    let (h, l, m) = (&b.high, &b.low, b.len());
    let mut out = vec![vec![NA; m]; 3];
    let (mut up, mut dn, mut run) = (false, false, 0.);
    for t in 2..m {
        let u = l[t] > l[t - 1] && h[t] >= h[t - 1] && (up || (h[t - 1] > h[t - 2] && l[t - 1] > l[t - 2]));
        let d = h[t] < h[t - 1] && l[t] <= l[t - 1] && (dn || (h[t - 1] < h[t - 2] && l[t - 1] < l[t - 2]));
        run = if u { if up { run + 1. } else { 1. } } else if d { if dn { run - 1. } else { -1. } } else { 0. };
        (up, dn) = (u, d);
        out[0][t] = u as u8 as f64; out[1][t] = d as u8 as f64; out[2][t] = run;
    }
    Ok(out)
}

pub const DEFS: &[IndicatorDef] = &[
    def("zigzag_cum_volume", "Zig Zag Cumulative Volume (Sierra Chart, causal)", "Volume", ZZ, &["volume", "direction"], PN, V, "Sierra Chart Zig Zag Cumulative Volume (doc IDs 341/133; ACSIL scsf_ZigZagCumulativeVolumeStudy, AccumulateZigZagVolume, ResettableZigZag_S), run bar by bar as a declared causal adaptation (Sierra's drawn history repaints while a leg extends; no parity with it is claimed): from the first bar s, the zigzag starts when H > H_s and L > L_s (s is a low, the bar a high peak) or H < H_s and L < L_s (mirror); with a high peak P, a bar whose low < P·(1 − r) (mode 1, r = percent/100) or P − amount (mode 3) makes P final and starts a low peak, else a high ≥ P moves the peak to that bar; a low peak mirrors this (high > P·(1 + r) reverses, low ≤ P extends); volume = Σ volume over the bars after the last final peak through this bar (on the bar that makes a peak final the sum restarts from the bar after it), direction = +1 after a final low, −1 after a final high; blank until the first peak is final (data-dependent); a missing volume counts 0 and blanks that bar; Sierra's mode 2 (bars + amount), bid/ask volume and session reset are not offered; defaults mode 1, reversal 0.5 %, amount 0.01 (mode 3)", &[usize::MAX, usize::MAX], true, zigzag_cum_volume),
    def("demand_index", "Demand Index (Sierra Chart)", "Volume", DIP, &["di", "di_ma"], PN, V, "Sierra Chart Demand Index (doc ID 139, ACSIL scsf_DemandIndex; Sibbet's original and the MetaStock built-in were not checked): P = H + L + 2C; K = 0.375·(P + P[1])/R0 with R0 = the first loaded bar's H − L (Sierra divides by RangeMA[0], which equals it; the values depend on where the history starts, R0 = 0 gives no value); VA = ema(volume, n1); from bar 1 BP = SP = volume/VA, then P < P[1]: BP /= exp(K·(P[1] − P)/P), P > P[1]: SP /= exp(K·(P − P[1])/P[1]); BPM = ema(BP, n2), SPM = ema(SP, n2); di = BPM > SPM ? 100(1 − SPM/BPM) : BPM < SPM ? −100(1 − BPM/SPM) : 0; di_ma = sma(di, n3) (always simple, as Sierra); the EMAs are the lab's SMA-seeded ones (Sierra seeds with the first value and a growing length, so the first bars differ) in f64 (Sierra float32); only Sierra's default average type, exponential, is offered; defaults n1 19, n2 19, n3 30", &[36, 65], false, demand_index),
    def("rolling_pivots", "Rolling Pivot Points (Skender)", "Price functions", RP, &["pp", "r1", "r2", "r3", "r4", "s1", "s2", "s3", "s4"], PR, NO, "Skender Stock.Indicators Rolling Pivots (ToRollingPivots, PivotPoints.GetPivotPoint): on bar i ≥ w + o, H = highest high and L = lowest low of the w bars i − w − o .. i − o − 1 (the current bar is never in the window), C = close[i − o − 1], O = open of the current bar i; Standard PP = (H + L + C)/3, R1 = 2PP − L, S1 = 2PP − H, R2 = PP + (H − L), S2 = PP − (H − L), R3 = H + 2(PP − L), S3 = L − 2(H − PP); Camarilla PP = C, R/Sn = C ± 1.1(H − L)/12, /6, /4, /2 (n = 1..4); Demark X = C < O ? H + 2L + C : C > O ? 2H + L + C : H + L + 2C, PP = X/4, R1 = X/2 − L, S1 = X/2 − H; Fibonacci PP = (H + L + C)/3 ± 0.382, 0.618, 1·(H − L); Woodie PP = (H + L + 2O)/4, R1..R3/S1..S3 as Standard; R4/S4 only for Camarilla, levels a type lacks are blank; a missing high or low in the window gives no value; defaults window 20, offset 0, Standard", &[20; 9], true, rolling_pivots),
    def("pldot", "Drummond Geometry PLdot", "Trend", &[], &["pldot", "next"], PR, NO, "Drummond Geometry PL Dot (Wikipedia, citing Hill & Pruitt, The Ultimate Trading Guide 2000, and T. Hearne, Futures June 1999): the average of the high, low and close of the last three bars, displaced forward one bar; next = sma(hlc3, 3), the dot for the next bar known at this close (from bar 2); pldot = next[1], the dot drawn on this bar (from bar 3); the envelopes, push, refresh and exhaust are proprietary course material and not offered", &[3, 2], false, pldot),
    def("midas", "MIDAS Support/Resistance Curve", "Volume", MD, &["midas"], PR, V, "MIDAS S/R curve (Paul Levine; Wikipedia citing Coles & Hawkins, Stocks & Commodities 2009 and the 2011 Wiley book, neither read): MIDAS_i = [Y(x_i) − Y(x_j)]/(x_i − x_j) with x = cumulative volume and Y = cumulative price·volume from launch bar j, computed as Σ_{k=j..i} price·volume / Σ_{k=j..i} volume, the launch bar included (= our vwap_period anchor 5 with this price); launch = the first bar dated on or after the launch date, blank before it and while Σ volume = 0; price hl2 (lab default; the book's price basis was not checked) or close; the Topfinder/Bottomfinder curve is not offered (its fitted volume D is discretionary)", &[0], false, midas),
    def("liquidity_sweep", "Liquidity Sweeps (LuxAlgo)", "Patterns", LS, &["bull", "bear", "bull_level", "bear_level"], PR, NO, "LuxAlgo Liquidity Sweeps (TradingView open source, CC BY-NC-SA; no canonical SMC/ICT definition exists): swing levels = ta.pivothigh/pivotlow(high/low, length, length), stored on their confirmation bar and checked that same bar; every stored level is checked each bar (oldest first): an unbroken pivot-high level P with close > P is removed (Only Wicks) or marked broken (other modes); else (not Only Outbreaks) if not yet wick-swept and high > P and close < P: bearish sweep, once per level; a broken pivot-high level is removed on close < P, and low < P with close > P is a bullish sweep (retest), after which it is removed; pivot lows mirror this (close < P breaks, low < P and close > P bullish sweep, a broken level with high > P and close < P bearish sweep); a level is dropped once more than 2000 bars old; equality is never a sweep; bull/bear = 1 on a sweep bar else 0 (from bar 2·length), bull_level/bear_level = the swept level (the one nearest the close if several), blank otherwise; the sweep-area boxes are drawing only; defaults length 5, Only Wicks", &[10, 10, usize::MAX, usize::MAX], true, liquidity_sweep),
    def("ad_percent", "AD Percent", BRD, &[], &["pct"], PN, BR, "StockCharts AD Percent: 100·(advances − declines)/total issues, total = advances + declines + unchanged (the 'total number of stocks in the group'), between −100 and +100; computed on the market's breadth (NSE for India, NYSE for US), not on an ETF's components as StockCharts can; zero total or missing breadth → NaN", &[0], false, ad_percent),
    def("ad_volume_percent", "AD Volume Percent", BRD, &[], &["pct"], PN, BR, "StockCharts AD Volume Percent: 100·(advancing volume − declining volume)/total volume, total = advancing + declining + unchanged volume (unchanged volume is US only; India's total is advancing + declining); market breadth (NSE/NYSE); zero total or missing breadth → NaN", &[0], false, ad_volume_percent),
    def("itbm", "DecisionPoint ITBM", BRD, &[], &["itbm", "signal"], PN, BR, "StockCharts DecisionPoint Intermediate-Term Breadth Momentum Oscillator: RANA = 1000·(A − D)/(A + D) (= our mcclellan ratio_adjusted net); MO = ema(RANA, 19) − ema(RANA, 39); itbm = ema(MO + ema(RANA, 19), 20), reading the page's 'daily 10% exponential average (Ratio-Adjusted)' as the McClellan page's 19-day (10%) EMA of RANA; signal = ema(itbm, 10) (the page's signal line); SMA-seeded EMAs ('the first EMA calculation is a simple average'), days without breadth skipped and NaN; A + D = 0 → NaN", &[57, 66], false, itbm),
    def("itvm", "DecisionPoint ITVM", BRD, &[], &["itvm", "signal"], PN, BR, "StockCharts DecisionPoint Intermediate-Term Volume Momentum Oscillator: RANV = 1000·(advancing volume − declining volume)/total volume (total includes unchanged volume where the source has it, US only); itvm = ema(2·ema(RANV, 19) − ema(RANV, 39), 20), the rest 'identical to the McClellan Oscillator and ITBM'; signal = ema(itvm, 10); SMA-seeded EMAs, days without breadth skipped; zero total → NaN", &[57, 66], false, itvm),
    def("swenlin_sto", "DecisionPoint Swenlin Trading Oscillator", BRD, STO, &["sto"], PN, BR, "StockCharts DecisionPoint Swenlin Trading Oscillator: STO-B = sma(ema(1000·(A − D)/(A + D), 4), 5); STO-V (basis 1) the same with advancing and declining volume, denominator advancing + declining volume as the page states; SMA-seeded EMA, days without breadth skipped; zero denominator → NaN; default STO-B", &[7], false, swenlin_sto),
    def("renko_predictor", "Renko Bar Predictor (Sierra Chart)", "Price functions", super::x_atlas::RENKO, &["next_up", "next_down"], PR, NO, "Sierra Chart Renko Bar Predictor (doc ID 345, ACSIL scsf_RenkoBarPredictor; prior brick up: high = C + box, low = C − 2·box; prior brick down: high = C + 2·box, low = C − box) on our bar-aligned renko: next_up = brick_high + s, next_down = brick_low − s, with s the box tested on this bar (fixed points, ATR or % of close as renko's mode; with an ATR or % box this is the next box, not the last brick's); before the first brick the anchor close ± s; the closes that would complete the next up or down brick (renko forms bricks on ≥, so Sierra's one-tick 'exceeded' offset does not apply); computed on every bar, not only the last (presentation adaptation); parameters and defaults as renko", &[13, 13], false, renko_predictor),
    def("one_time_framing", "One Time Framing (Sierra Chart)", "Trend", &[], &["up", "down", "run"], PN, NO, "Sierra Chart One Time Framing (doc ID 482; no source; Dalton's Mind Over Markets reading was not retrieved) on bars t − 2, t − 1, t, a declared reading of the doc's same-index prerequisite: up = 1 if L > L[1] and H ≥ H[1] and (up[1] or (H[1] > H[2] and L[1] > L[2])), so the uptrend bar comes first and an equal low exits; down = 1 if H < H[1] and L ≤ L[1] and (down[1] or (H[1] < H[2] and L[1] < L[2])); an inside bar exits both; run = +bars in up, −bars in down, 0 otherwise; from bar 2", &[2, 2, 2], false, one_time_framing),
];
/// (id, platform names, AmiBroker names): one row per id in DEFS.
pub const SOURCES: &[(&str, &str, &str)] = &[
    ("zigzag_cum_volume", "Sierra Chart: Zig Zag Cumulative Volume", ""),
    ("demand_index", "Sierra Chart: Demand Index", ""),
    ("rolling_pivots", "Skender Stock.Indicators: Rolling Pivot Points", ""),
    ("pldot", "Drummond Geometry: PL Dot", ""),
    ("midas", "MIDAS Technical Analysis (Levine; Coles & Hawkins): MIDAS Support/Resistance curve", ""),
    ("liquidity_sweep", "TradingView (community): Liquidity Sweeps [LuxAlgo]", ""),
    ("ad_percent", "StockCharts: AD Percent", ""),
    ("ad_volume_percent", "StockCharts: AD Volume Percent", ""),
    ("itbm", "StockCharts DecisionPoint: Intermediate-Term Breadth Momentum Oscillator (ITBM)", ""),
    ("itvm", "StockCharts DecisionPoint: Intermediate-Term Volume Momentum Oscillator (ITVM)", ""),
    ("swenlin_sto", "StockCharts DecisionPoint: Swenlin Trading Oscillator (STO-B, STO-V)", ""),
    ("renko_predictor", "Sierra Chart: Renko Bar Predictor", ""),
    ("one_time_framing", "Sierra Chart: One Time Framing", ""),
];

#[cfg(test)]
mod tests {
    use crate::lab::bars::{Bars, Breadth};
    use crate::lab::catalogue::{get, run, Params};
    use crate::lab::testdata::{assert_series, close_eq, closes, compiles, cut, from_ohlc, go, walk, with_breadth};
    const N: f64 = f64::NAN;
    fn hl(rows: &[(f64, f64)]) -> Bars { from_ohlc(&rows.iter().map(|&(h, l)| ((h + l) / 2., h, l, (h + l) / 2.)).collect::<Vec<_>>()) }
    fn vol(mut b: Bars, v: &[f64]) -> Bars { b.volume = Some(v.to_vec()); b }
    fn breadth(adv: &[f64], dec: &[f64], unch: &[f64], av: &[f64], dv: &[f64], uv: &[f64]) -> Bars {
        let mut b = closes(&vec![1.; adv.len()]);
        b.breadth = Some(Breadth { adv: adv.to_vec(), dec: dec.to_vec(), unch: unch.to_vec(), adv_vol: av.to_vec(), dec_vol: dv.to_vec(), unch_vol: uv.to_vec(), ..Default::default() });
        b
    }

    #[test] fn zigzag_volume_sums_since_the_last_final_peak() {
        // 10%: bar 1 starts the up leg (bar 0's low final), bars 2–3 extend the high, bar 4's low 9 < 13·0.9 makes bar 3 final,
        // bar 5 extends the low (9.5 ≤ 9.9), bar 6's high 12 > 8·1.1 makes bar 5 final
        let b = vol(hl(&[(10., 9.), (11., 10.), (12., 11.), (13., 12.), (12., 9.), (9.5, 8.), (12., 10.)]), &[1., 2., 3., 4., 5., 6., 7.]);
        let o = go("zigzag_cum_volume", &b, &[("percent", 10.)]);
        assert_series(&o[0], &[N, 2., 5., 9., 5., 11., 7.]);
        assert_series(&o[1], &[N, 1., 1., 1., -1., -1., 1.]);
        // mode 3, amount 4.5: bar 4's low 9 is not below 8.5, bar 5's 8 is → bar 3 final, the sum restarts with bars 4–5
        let a = go("zigzag_cum_volume", &b, &[("mode", 1.), ("amount", 4.5)]);
        assert_series(&a[0], &[N, 2., 5., 9., 14., 11., 18.]);
        assert_series(&a[1], &[N, 1., 1., 1., 1., -1., -1.]);
        // no trend yet: inside bars keep it blank
        assert_series(&go("zigzag_cum_volume", &vol(hl(&[(10., 9.), (10., 9.), (11., 8.)]), &[1.; 3]), &[])[0], &[N, N, N]);
    }
    #[test] fn demand_index_follows_the_sierra_steps() {
        // n 1 everywhere: VA = V, BP = SP = 1; P 40, 44, 40 with R0 = 2: bar 1 SP = 1/exp(0.375·84/2·4/40), bar 2 BP = 1/exp(0.375·84/2·4/40)
        let b = vol(from_ohlc(&[(10., 11., 9., 10.), (11., 12., 10., 11.), (10., 11., 9., 10.)]), &[5., 6., 7.]);
        let o = go("demand_index", &b, &[("bs_length", 1.), ("bs_ma_length", 1.), ("di_ma_length", 2.)]);
        let e = (-0.375f64 * 84. / 2. * 0.1).exp();
        assert_series(&o[0], &[N, 100. * (1. - e), -100. * (1. - e)]);
        assert_series(&o[1], &[N, N, 0.]);
        assert_series(&go("demand_index", &vol(closes(&[1., 2., 3.]), &[1.; 3]), &[("bs_length", 1.), ("bs_ma_length", 1.)])[0], &[N; 3]); // flat first bar: R0 = 0
    }
    #[test] fn rolling_pivots_exclude_the_current_bar_and_match_skender_types() {
        // window 2: on bar 2, H = 12, L = 9 from bars 0–1, C = close[1] = 11, O = open[2] = 10
        let b = from_ohlc(&[(10., 11., 9., 10.), (10., 12., 10., 11.), (10., 13., 8., 9.), (9., 10., 7., 8.)]);
        let o = go("rolling_pivots", &b, &[("window", 2.)]);
        let pp = 32. / 3.;
        assert_series(&o[0], &[N, N, pp, 10.]); // bar 3: H 13, L 8, C 9
        assert_series(&o[1][2..3], &[2. * pp - 9.]);
        assert_series(&o[3][2..3], &[12. + 2. * (pp - 9.)]);
        assert!(o[4].iter().all(|v| v.is_nan()));
        assert_series(&go("rolling_pivots", &b, &[("window", 2.), ("offset", 1.)])[0], &[N, N, N, pp]);
        let cam = go("rolling_pivots", &b, &[("window", 2.), ("type", 1.)]);
        assert_series(&cam[0][2..3], &[11.]);
        assert_series(&cam[4][2..3], &[11. + 1.1 * 3. / 2.]);
        assert_series(&cam[8][2..3], &[11. - 1.1 * 3. / 2.]);
        // Demark: C 11 > current open 10 → X = 2H + L + C = 44
        let dm = go("rolling_pivots", &b, &[("window", 2.), ("type", 2.)]);
        assert_series(&dm[0][2..3], &[11.]);
        assert_series(&dm[1][2..3], &[22. - 9.]);
        assert!(dm[2].iter().all(|v| v.is_nan()));
        assert_series(&go("rolling_pivots", &b, &[("window", 2.), ("type", 4.)])[0][2..3], &[(12. + 9. + 20.) / 4.]);
        assert_series(&go("rolling_pivots", &b, &[("window", 2.), ("type", 3.)])[2][2..3], &[pp + 0.618 * 3.]);
    }
    #[test] fn pldot_is_the_three_bar_hlc3_mean_shifted_one() {
        let o = go("pldot", &closes(&[1., 2., 3., 4., 5.]), &[]);
        assert_series(&o[1], &[N, N, 2., 3., 4.]);
        assert_series(&o[0], &[N, N, N, 2., 3.]);
    }
    #[test] fn midas_is_the_anchored_vwap_of_the_chosen_price() {
        let b = vol(from_ohlc(&[(1., 4., 2., 3.), (1., 6., 2., 5.), (1., 8., 4., 7.)]), &[1., 1., 2.]);
        assert_series(&go("midas", &b, &[])[0], &[3., 3.5, 4.75]);
        assert_series(&go("midas", &b, &[("source", 1.)])[0], &[3., 4., 5.5]);
        assert_series(&go("midas", &b, &[("anchor_date", 20240102.)])[0], &[N, 4., 16. / 3.]); // hl2 4, 6 from the launch bar
    }
    #[test] fn liquidity_sweep_modes() {
        // pivot high 10 at bar 1 (length 1), known on bar 2; bar 3 wicks above and closes below → bearish sweep, once
        let rows = [(5., 6., 4., 5.), (8., 10., 7., 8.), (7., 8., 6., 7.), (8., 11., 7., 9.), (9., 11., 8., 9.), (9., 12., 9., 11.), (11., 11., 9.5, 10.5)];
        let b = from_ohlc(&rows);
        let o = go("liquidity_sweep", &b, &[("length", 1.)]);
        assert_series(&o[1], &[N, N, 0., 1., 0., 0., 0.]);
        assert_series(&o[3], &[N, N, N, 10., N, N, N]);
        assert_series(&o[0], &[N, N, 0., 0., 0., 0., 0.]);
        // retest modes: bar 5 closes above 10 (broken), bar 6 dips below 10 and closes above → bullish sweep
        let w = go("liquidity_sweep", &b, &[("length", 1.), ("mode", 2.)]);
        assert_series(&w[1][..4], &[N, N, 0., 1.]);
        assert_series(&w[0], &[N, N, 0., 0., 0., 0., 1.]);
        assert_series(&w[2][6..], &[10.]);
        let oo = go("liquidity_sweep", &b, &[("length", 1.), ("mode", 1.)]);
        assert_series(&oo[1], &[N, N, 0., 0., 0., 0., 0.]);
        assert_series(&oo[0][6..], &[1.]);
    }
    #[test] fn breadth_percents_and_decisionpoint_oscillators() {
        let b = breadth(&[70., 30., 0.], &[7., 50., 0.], &[3., 20., 0.], &[36., 7., 1.], &[5., 36., 1.], &[N, 2., N]);
        assert_series(&go("ad_percent", &b, &[])[0], &[100. * 63. / 80., -20., N]);
        assert_series(&go("ad_volume_percent", &b, &[])[0], &[100. * 31. / 41., -100. * 29. / 45., 0.]);
        let full = with_breadth(walk(200, 3, true));
        let r = full.breadth.as_ref().unwrap();
        let x: Vec<f64> = (0..200).map(|i| 1000. * (r.adv[i] - r.dec[i]) / (r.adv[i] + r.dec[i])).collect();
        // ITBM = ema(MO + EMA19, 20) with MO from our mcclellan
        let mo = &go("mcclellan", &full, &[("ratio_adjusted", 1.)])[0];
        let e19 = crate::lab::ta::ema(&x, 19);
        let want = crate::lab::ta::ema(&mo.iter().zip(&e19).map(|(a, b)| a + b).collect::<Vec<_>>(), 20);
        assert_series(&go("itbm", &full, &[])[0], &want);
        assert_series(&go("swenlin_sto", &full, &[])[0], &crate::lab::ta::sma(&crate::lab::ta::ema(&x, 4), 5));
        let xv: Vec<f64> = (0..200).map(|i| 1000. * (r.adv_vol[i] - r.dec_vol[i]) / (r.adv_vol[i] + r.dec_vol[i])).collect();
        assert_series(&go("swenlin_sto", &full, &[("basis", 1.)])[0], &crate::lab::ta::sma(&crate::lab::ta::ema(&xv, 4), 5));
    }
    #[test] fn renko_predictor_is_one_box_beyond_the_brick() {
        let o = go("renko_predictor", &closes(&[50., 52., 54., 55., 56., 51.]), &[("mode", 1.), ("box", 2.)]);
        assert_series(&o[0], &[52., 54., 56., 56., 58., 56.]); // renko brick_high 50, 52, 54, 54, 56, 54
        assert_series(&o[1], &[48., 48., 50., 50., 52., 50.]); // brick_low 50, 50, 52, 52, 54, 52
    }
    #[test] fn one_time_framing_needs_an_uptrend_bar_first() {
        // bar 1 strict up bar, bar 2 higher low and equal high → up; bar 3 equal low exits; bar 4 lower bar without a
        // downtrend bar before it; bar 4 is that bar for bar 5 (lower high, equal low) → down, bar 6 stays down
        let b = hl(&[(10., 8.), (11., 9.), (11., 10.), (12., 10.), (11., 9.), (10., 9.), (9.5, 8.5)]);
        let o = go("one_time_framing", &b, &[]);
        assert_series(&o[0], &[N, N, 1., 0., 0., 0., 0.]);
        assert_series(&o[1], &[N, N, 0., 0., 0., 1., 1.]);
        assert_series(&o[2], &[N, N, 1., 0., 0., -1., -2.]);
    }
    #[test] fn breadth_items_are_causal_with_exact_warmup_and_reachable() {
        let full = with_breadth(walk(400, 11, true));
        for d in super::DEFS.iter().filter(|d| d.needs.breadth) {
            let want = run(d, &full, &Params::new()).unwrap();
            for (k, s) in want.iter().enumerate() {
                assert_eq!(s.iter().position(|v| !v.is_nan()), Some(d.first[k]), "{}.{k} first", d.id);
                assert!(s[d.first[k]..].iter().all(|v| v.is_finite()), "{}.{k} gap", d.id);
            }
            for c in [60usize, 151, 399] {
                let got = run(d, &cut(&full, c), &Params::new()).unwrap();
                for (k, s) in got.iter().enumerate() { for i in 0..c { assert!(close_eq(s[i], want[k][i]), "{}.{k} bar {i}", d.id); } }
            }
        }
        assert_eq!(run(&get("itbm").unwrap(), &closes(&[1., 2.]), &Params::new()).unwrap_err(), "needs breadth");
    }
    #[test] fn sparse_items_warm_up_where_declared() {
        let bars = walk(300, 5, true);
        let first = |id: &str, k: usize| go(id, &bars, &[])[k].iter().position(|v| !v.is_nan());
        for k in [0, 1, 2, 3, 5, 6, 7] { assert_eq!(first("rolling_pivots", k), Some(20)); }
        assert_eq!(first("liquidity_sweep", 0), Some(10));
        assert_eq!(first("liquidity_sweep", 1), Some(10));
        // data-dependent start: it starts and never gaps after
        for s in go("zigzag_cum_volume", &bars, &[]) { let f = s.iter().position(|v| !v.is_nan()).unwrap(); assert!(s[f..].iter().all(|v| v.is_finite())); }
    }
    #[test] fn parameter_extremes_keep_lengths() {
        for bars in [with_breadth(walk(3, 1, true)), with_breadth(walk(300, 1, true))] {
            for d in super::DEFS {
                for hi in [false, true] {
                    let prm: Params = d.params.iter().map(|p| (p.key.to_string(), if hi { p.max } else { p.min })).collect();
                    let o = run(d, &bars, &prm).unwrap();
                    assert!(o.iter().all(|s| s.len() == bars.len() && s.iter().all(|v| !v.is_infinite())), "{} hi={hi}", d.id);
                }
            }
        }
    }
    #[test] fn rules_can_read_every_output() {
        for d in super::DEFS { for o in d.outputs { compiles(d.id, o).unwrap_or_else(|e| panic!("{}.{o}: {e:?}", d.id)); } }
    }
}
