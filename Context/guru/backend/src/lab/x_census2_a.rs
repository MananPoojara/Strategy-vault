//! Census 2 group "a" (2026-10-27): indicators from trading_indicator_census.html that the lab did not have.
//! Formulas: see each definition string. Listed-only entries carry "not available yet — needs …" (data the lab does not hold)
//! or "not available yet — formula not published (…)" (proprietary, or no primary text found; nothing invented).
//! Not built and not listed (no primary text reachable, nothing invented): Mobility Oscillator (Widner, S&C Feb 1996; the only
//! open port, Moormann's [CC] Pine, declares its own changes), Divergence Index (Storz, S&C Jan 1996; CQG/TradeStation code not
//! reachable). Swami Stochastics is a SwamiChart heatmap (a stochastic for every length 12..48 drawn as colours), not one series.
#![allow(dead_code)]
use super::bars::Bars;
use super::catalogue::{get, int, len, n, p, real, run, IndicatorDef, Needs, Output, ParamSpec, Params, Placement};
use super::ta::{self, NA};

#[allow(clippy::too_many_arguments)]
const fn def(id: &'static str, name: &'static str, category: &'static str, params: &'static [ParamSpec], outputs: &'static [&'static str], placement: Placement,
             needs: Needs, definition: &'static str, first: &'static [usize], sparse: bool, f: fn(&Bars, &Params) -> Output) -> IndicatorDef {
    IndicatorDef { id, name, category, params, outputs, placement, needs, definition, first, sparse, compute: Some(f), note: "" }
}
/// Listed in the picker, not computed: the data it needs is not admitted (the app shows the note as "data not available").
const fn listed(id: &'static str, name: &'static str, category: &'static str, needs: Needs, definition: &'static str, note: &'static str) -> IndicatorDef {
    IndicatorDef { id, name, category, params: &[], outputs: &["value"], placement: Placement::Pane, needs, definition,
                   first: &[usize::MAX], sparse: false, compute: None, note }
}
const M: &str = "Momentum";
const PN: Placement = Placement::Pane;
const NO: Needs = Needs::NONE;
const PAIR: Needs = Needs { second_symbol: true, ..Needs::NONE };

fn call(id: &str, b: &Bars, kv: &[(&str, f64)]) -> Result<Vec<Vec<f64>>, String> {
    run(&get(id).ok_or_else(|| format!("{id} missing"))?, b, &kv.iter().map(|(k, v)| (k.to_string(), *v)).collect())
}

const DY: &[ParamSpec] = &[int("sd_length", "Standard deviation length", 5., 2., 2000.), len("avg_length", "Average of the standard deviation length", 10.),
    real("base_length", "Base RSI length", 14., 1., 2000., 1.), len("min_length", "Shortest RSI length", 5.), len("max_length", "Longest RSI length", 30.)];
fn dymi(b: &Bars, prm: &Params) -> Output {
    let x = &b.close;
    let sd = ta::stdev(x, n(prm, "sd_length"));
    let avg = ta::sma(&sd, n(prm, "avg_length"));
    let (lo, hi, base) = (n(prm, "min_length"), n(prm, "max_length"), p(prm, "base_length"));
    let top = lo.max(hi); // the RSI window always fits from here on, so no gaps after the first value
    Ok(vec![(0..x.len()).map(|t| {
        let vi = sd[t] / avg[t];
        if t < top || vi.is_nan() { return NA; } // warm-up, or 0/0 volatility (flat prices)
        let raw = base / vi; // vi 0 (five equal closes after volatile ones): 14/0 → the longest length
        let td = if raw.is_finite() { (raw as usize).max(lo).min(hi) } else { hi };
        let (mut up, mut dn) = (0., 0.);
        for k in t + 1 - td..=t { let d = x[k] - x[k - 1]; if d > 0. { up += d } else { dn -= d } }
        if up + dn > 0. { 100. * up / (up + dn) } else { NA }
    }).collect()])
}

const HC: &[ParamSpec] = &[len("tema_length", "TEMA length", 55.), len("ema_length", "Close EMA length", 60.), real("candle_factor", "Candle size factor", 1.1, 0.01, 10., 0.01)];
fn tema(x: &[f64], k: usize) -> Vec<f64> {
    let e1 = ta::ema(x, k);
    let e2 = ta::ema(&e1, k);
    let e3 = ta::ema(&e2, k);
    (0..x.len()).map(|i| 3. * (e1[i] - e2[i]) + e3[i]).collect()
}
fn hacolt(b: &Bars, prm: &Params) -> Output {
    let (o, h, l, c, m): (&[f64], &[f64], &[f64], &[f64], usize) = (&b.open, &b.high, &b.low, &b.close, b.len());
    let (k, f) = (n(prm, "tema_length"), p(prm, "candle_factor"));
    let q = b.ohlc4();
    let (mut hao, mut hac) = (vec![NA; m], vec![NA; m]);
    for i in 0..m {
        hao[i] = if i == 0 || hao[i - 1].is_nan() { q[i] } else { (hao[i - 1] + q[i]) / 2. };
        hac[i] = (hao[i] + h[i].max(hao[i]) + l[i].min(hao[i]) + q[i]) / 4.;
    }
    let zl = |x: &[f64]| { let t1 = tema(x, k); let t2 = tema(&t1, k); (0..m).map(|i| 2. * t1[i] - t2[i]).collect::<Vec<f64>>() };
    let (has, hls) = (zl(&hac), zl(&b.hl2()));
    let ema = ta::ema(c, n(prm, "ema_length"));
    let mut out = vec![NA; m];
    // Pine semantics: a comparison with a missing value (bar 0's [1], warm-up) is false
    let (mut kn1p, mut ka1p, mut kn2p, mut ka2p, mut utrp, mut dtrp, mut held, mut neu) = (false, false, false, false, false, false, false, false);
    for i in 0..m {
        let pv = |x: &[f64]| if i > 0 { x[i - 1] } else { NA };
        let short = (c[i] - o[i]).abs() < (h[i] - l[i]) * f;
        let kn1 = (hac[i] >= hao[i] && pv(&hac) >= pv(&hao)) || c[i] >= hac[i] || h[i] > pv(h) || l[i] > pv(l) || hls[i] >= has[i];
        let ka1 = kn1 || (kn1p && c[i] >= o[i]) || c[i] >= pv(c);
        let utr = ka1 || (ka1p && short && h[i] >= pv(l));
        let kn2 = (hac[i] < hao[i] && pv(&hac) < pv(&hao)) || hls[i] < has[i];
        let ka2 = kn2 || (kn2p && c[i] < o[i]) || c[i] < pv(c);
        let dtr = ka2 || (ka2p && short && l[i] <= pv(h));
        let upw = !dtr && dtrp && utr;
        let dnw = !utr && utrp && dtr;
        if upw != dnw { held = upw; }
        let buy = upw || (!dnw && held);
        neu = buy || (!(c[i] < ema[i]) && neu);
        if has[i].is_finite() && hls[i].is_finite() && ema[i].is_finite() { out[i] = if buy { 1. } else if neu { 0. } else { -1. }; }
        (kn1p, ka1p, kn2p, ka2p, utrp, dtrp) = (kn1, ka1, kn2, ka2, utr, dtr);
    }
    Ok(vec![out])
}

const EM: &[ParamSpec] = &[len("slow_length", "Slow EMA length (r)", 20.), len("fast_length", "Fast EMA length (s)", 5.), len("smooth_length", "Third EMA length (u)", 3.),
    len("signal_length", "Signal EMA length (ul)", 3.)];
fn ergodic_macd(b: &Bars, prm: &Params) -> Output {
    let (f, s) = (ta::ema(&b.close, n(prm, "fast_length")), ta::ema(&b.close, n(prm, "slow_length")));
    let macd: Vec<f64> = f.iter().zip(&s).map(|(a, b)| a - b).collect();
    let erg = ta::ema(&macd, n(prm, "smooth_length"));
    let sig = ta::ema(&erg, n(prm, "signal_length"));
    Ok(vec![erg, sig])
}
const ED: &[ParamSpec] = &[len("ema_length", "Price EMA length (r)", 20.), len("smooth1", "Second EMA length (s)", 5.), len("smooth2", "Third EMA length (u)", 3.),
    len("signal_length", "Signal EMA length (ul)", 3.)];
fn ergodic_mdi(b: &Bars, prm: &Params) -> Output {
    let e = ta::ema(&b.close, n(prm, "ema_length"));
    let md: Vec<f64> = b.close.iter().zip(&e).map(|(c, e)| c - e).collect();
    let mdi = ta::ema(&ta::ema(&md, n(prm, "smooth1")), n(prm, "smooth2"));
    let sig = ta::ema(&mdi, n(prm, "signal_length"));
    Ok(vec![mdi, sig])
}

const RK: &[ParamSpec] = &[len("length", "Relative strength lookback", 90.), len("ema_length", "EMA length", 3.)];
fn rsmk(b: &Bars, prm: &Params) -> Output {
    let s = super::pairs::second(b)?;
    let lv: Vec<f64> = b.close.iter().zip(s).map(|(&c, &r)| if c > 0. && r > 0. { (c / r).ln() } else { NA }).collect();
    let d = ta::change(&lv, n(prm, "length"));
    Ok(vec![ta::ema(&d, n(prm, "ema_length")).into_iter().map(|v| 100. * v).collect()])
}

const RD: &[ParamSpec] = &[len("rsi_length", "RSI length", 14.), len("di_length", "DI length", 14.), len("adx_smoothing", "ADX smoothing", 14.)];
fn rdx(b: &Bars, prm: &Params) -> Output {
    let rsi = call("rsi", b, &[("length", p(prm, "rsi_length"))])?.swap_remove(0);
    let adx = call("dmi", b, &[("di_length", p(prm, "di_length")), ("adx_smoothing", p(prm, "adx_smoothing"))])?.swap_remove(2);
    let v = rsi.iter().zip(&adx).map(|(r, a)| r + if *a > 20. { (a - 20.) / 5. } else if a.is_nan() { NA } else { 0. }).collect();
    Ok(vec![v, rsi])
}

const SD: &[ParamSpec] = &[len("lookback", "Lookback period", 200.), len("length", "Distance length (n periods)", 12.), len("ema_length", "Smoothing EMA length", 3.)];
fn sdo(b: &Bars, prm: &Params) -> Output {
    let (x, k, lb) = (&b.close, n(prm, "length"), n(prm, "lookback"));
    let dist: Vec<f64> = (0..x.len()).map(|t| if t >= k { (x[t] - x[t - k]).abs() } else { NA }).collect();
    let sd: Vec<f64> = (0..x.len()).map(|t| {
        if t + 1 < k + lb { return NA; }
        let w = &dist[t + 1 - lb..=t];
        if w.iter().any(|v| v.is_nan()) { return NA; }
        let (lo, hi) = (w.iter().cloned().fold(f64::MAX, f64::min), w.iter().cloned().fold(f64::MIN, f64::max));
        let dd = if hi > lo { (dist[t] - lo) / (hi - lo) } else { 0. }; // Pine nz(0/0, 0)
        if x[t] > x[t - k] { dd } else if x[t] < x[t - k] { -dd } else { 0. }
    }).collect();
    Ok(vec![ta::ema(&sd, n(prm, "ema_length")).into_iter().map(|v| 100. * v).collect()])
}

const WA: &[ParamSpec] = &[len("wma_length", "WMA length", 4.), len("ema1_length", "First EMA length", 13.), len("ema2_length", "Second EMA length", 13.)];
fn wami(b: &Bars, prm: &Params) -> Output {
    let w = ta::wma(&ta::change(&b.close, 1), n(prm, "wma_length"));
    Ok(vec![ta::ema(&ta::ema(&w, n(prm, "ema1_length")), n(prm, "ema2_length"))])
}

const AZ: &[ParamSpec] = &[int("short_length", "Short z-score length", 10., 2., 2000.), int("long_length", "Long z-score length", 200., 2., 2000.)];
fn zs(x: &[f64], k: usize) -> Vec<f64> {
    let (m, s) = (ta::sma(x, k), ta::stdev(x, k));
    (0..x.len()).map(|i| if s[i] > 0. { (x[i] - m[i]) / s[i] } else { NA }).collect()
}
fn aggz(b: &Bars, prm: &Params) -> Output {
    let (a, l) = (zs(&b.close, n(prm, "short_length")), zs(&b.close, n(prm, "long_length")));
    Ok(vec![a.iter().zip(&l).map(|(s, l)| (-s + l) / 2.).collect()])
}

const CM: &[ParamSpec] = &[len("r", "First EMA length (r)", 20.), len("s", "Second EMA length (s)", 5.), len("u", "Third EMA length (u)", 3.)];
fn blau_cmi(b: &Bars, prm: &Params) -> Output {
    let (r, s, u) = (n(prm, "r"), n(prm, "s"), n(prm, "u"));
    let tri = |x: &[f64]| ta::ema(&ta::ema(&ta::ema(x, r), s), u);
    let body: Vec<f64> = b.close.iter().zip(&b.open).map(|(c, o)| c - o).collect();
    let (num, den) = (tri(&body), tri(&body.iter().map(|v| v.abs()).collect::<Vec<_>>()));
    Ok(vec![num.iter().zip(&den).map(|(a, d)| if *d > 0. { 100. * a / d } else if *d == 0. { 0. } else { NA }).collect()])
}

const DN: &[ParamSpec] = &[len("rsi_length", "RSI length", 14.), len("ma_length", "Moving average length", 21.), real("midpoint", "Oscillator midpoint (Mc)", 50., -1000., 1000., 1.)];
fn dynamo(b: &Bars, prm: &Params) -> Output {
    let o = call("rsi", b, &[("length", p(prm, "rsi_length"))])?.swap_remove(0);
    let ma = ta::sma(&o, n(prm, "ma_length"));
    let mc = p(prm, "midpoint");
    Ok(vec![o.iter().zip(&ma).map(|(o, a)| mc - (a - o)).collect()])
}

const RT: &[ParamSpec] = &[int("trend_length", "Trend length", 100., 50., 2000.), int("sensitivity", "Sensitivity (%)", 95., 50., 98.), len("signal_length", "Signal EMA length", 20.)];
fn rti(b: &Bars, prm: &Params) -> Output {
    let (c, k, sens) = (&b.close, n(prm, "trend_length"), p(prm, "sensitivity"));
    let sd = ta::stdev(c, 2);
    // Pine math.round is half away from zero, as f64::round; length ≥ 50 and sensitivity ≤ 98 keep both indices inside the window
    let ui = ((sens / 100. * k as f64).round() as usize).saturating_sub(1);
    let li = (((100. - sens) / 100. * k as f64).round() as usize).saturating_sub(1);
    let v: Vec<f64> = (0..c.len()).map(|t| {
        if t + 1 < k || sd[t + 1 - k..=t].iter().any(|v| v.is_nan()) { return NA; }
        let mut up: Vec<f64> = (t + 1 - k..=t).map(|j| c[j] + sd[j]).collect();
        let mut lo: Vec<f64> = (t + 1 - k..=t).map(|j| c[j] - sd[j]).collect();
        up.sort_by(f64::total_cmp);
        lo.sort_by(f64::total_cmp);
        let (ut, lt) = (up[ui], lo[li]);
        let r = 100. * (c[t] - lt) / (ut - lt);
        if r.is_finite() { r } else { NA }
    }).collect();
    let sig = ta::ema(&v, n(prm, "signal_length"));
    Ok(vec![v, sig])
}

const fn unpublished(id: &'static str, name: &'static str, definition: &'static str, note: &'static str) -> IndicatorDef { listed(id, name, M, NO, definition, note) }

pub const DEFS: &[IndicatorDef] = &[
    def("dymi", "Dynamic Momentum Index (Chande)", M, DY, &["dymi"], PN, NO, "Chande & Kroll (S&C May 1993; The New Technical Trader, 1994); devexperts dxCharts and LuxAlgo Library restatements (MetaStock Stdev(C,5)/Mov(Stdev(C,5),10,S)): sd = stdev(close, 5) (population); vi = sd/sma(sd, 10); td = int(14/vi) (truncated) limited to 5..30 (vi 0 with a non-zero average: the longest length); dymi = RSI of close over the last td bars (MotiveWave's listing differs: round(14/sma(sd, 10)), the average itself, not the ratio); declared adaptation: the RSI is the window form 100·Σup/(Σup + Σdown) over the last td closes (a Wilder recursion has no single definition for a length that changes every bar, and platforms differ); no value before bar max(min, max length) (30), on 0/0 volatility (flat prices: sd and its average both 0) or when the window has no change; defaults 5, 10, 14, 5, 30; guide lines 30/70", &[30], false, dymi),
    def("hacolt", "Heikin-Ashi Candlestick Oscillator Long Term (Vervoort)", M, HC, &["hacolt"], PN, NO, "Vervoort (S&C July 2012, from his HACO, S&C Dec 2008), as LazyBear's open-source TradingView port (4zuhGaAU): haOpen = (haOpen[1] + ohlc4)/2 (bar 0: ohlc4; this bar's ohlc4, as the port); haClose = (haOpen + max(H, haOpen) + min(L, haOpen) + ohlc4)/4; tema(x) = 3(e1 − e2) + e3 of nested ema(·, 55); haS = 2·tema(haClose) − tema(tema(haClose)), hlS likewise for hl2; short = |C − O| < (H − L)·1.1; keepn1 = (haC ≥ haO and haC[1] ≥ haO[1]) or C ≥ haC or H > H[1] or L > L[1] or hlS ≥ haS; keepall1 = keepn1 or (keepn1[1] and C ≥ O) or C ≥ C[1]; utr = keepall1 or (keepall1[1] and short and H ≥ L[1]); keepn2 = (haC < haO and haC[1] < haO[1]) or hlS < haS; keepall2 = keepn2 or (keepn2[1] and C < O) or C < C[1]; dtr = keepall2 or (keepall2[1] and short and L ≤ H[1]); upw = not dtr and dtr[1] and utr; dnw = not utr and utr[1] and dtr; held = upw when upw ≠ dnw, else held[1] (from false); buy = upw or (not dnw and held); neutral = buy or (C ≥ ema(close, 60) and neutral[1]); hacolt = 1 (long) when buy, 0 (flat) when neutral, else −1 (short) (thinkorswim draws 100/50/0); comparisons with a missing value are false (Pine), the state runs from bar 0 and is shown once both smoothings and the EMA exist (bar 6·(55 − 1)); defaults 55, 60, 1.1", &[324], false, hacolt),
    def("ergodic_macd", "Ergodic MACD Oscillator (Blau)", M, EM, &["ergodic", "signal"], PN, NO, "Blau (Momentum, Direction, and Divergence, 1995), MQL5 Blau_Ergodic_MACD (code 376) / Blau_MACD (375): ergodic = ema(ema(close, s) − ema(close, r), u); signal = ema(ergodic, ul); EMAs alpha 2/(n + 1) seeded with the SMA of their first n values; defaults r 20, s 5, u 3, ul 3", &[21, 23], false, ergodic_macd),
    def("ergodic_mdi", "Ergodic Mean Deviation Index (Blau)", M, ED, &["mdi", "signal"], PN, NO, "Blau (Momentum, Direction, and Divergence, 1995), MQL5 Blau_Ergodic_MDI (code 374): md = close − ema(close, r); mdi = ema(ema(md, s), u); signal = ema(mdi, ul); EMAs alpha 2/(n + 1) seeded with the SMA of their first n values; price units (not normalised); defaults r 20, s 5, u 3, ul 3", &[25, 27], false, ergodic_mdi),
    def("rsmk", "RSMK Relative Strength (Katsanos)", M, RK, &["rsmk"], PN, PAIR, "Katsanos (S&C March 2020, Traders' Tips TradeStation/MetaStock code): lv = ln(close/close of the second instrument); rsmk = 100·ema(lv − lv[90], 3); positive when the symbol outperforms the reference; a price ≤ 0 gives no value; EMA seeded with the SMA of its first 3 values; defaults 90, 3", &[92], false, rsmk),
    def("rdx", "RDX (RSI + ADX)", M, RD, &["rdx", "rsi"], PN, NO, "traderharikrishna, TradingView open-source RDX (CYqbdAgU): rsi = rsi(close, 14); adx from dmi(14, 14); rdx = rsi + (adx > 20 ? (adx − 20)/5 : 0) (the script's filled line); no rdx until the ADX exists (Pine treats a missing ADX as 0 strength); default lengths 14", &[27, 14], false, rdx),
    def("stochastic_distance_osc", "Stochastic Distance Oscillator (Apirine)", M, SD, &["sdo"], PN, NO, "Apirine (S&C June 2023), Traders' Tips TradingView (PineCodersTASC) / MetaQuotes code: dist = |close − close[n]|; dd = (dist − lowest(dist, lb))/(highest(dist, lb) − lowest(dist, lb)), 0 when the range is 0; sd = dd when close > close[n], −dd when below, else 0; sdo = 100·ema(sd, 3) (EMA seeded with its first-3 SMA), −100..100; no value until n + lb bars exist (Pine's nz() reads missing closes as 0); TradeStation's listing takes the numerator low over n, a misprint the other listings do not share; defaults lb 200, n 12, 3; guide lines ±40", &[213], false, sdo),
    def("wami", "WAMI Oscillator (Warren)", M, WA, &["wami"], PN, NO, "Warren (S&C 1994), eSignal WAMI.efs / MotiveWave WAMI: wami = ema(ema(wma(close − close[1], 4), 13), 13); EMAs alpha 2/(n + 1) seeded with the SMA of their first n values; defaults 4, 13, 13", &[28], false, wami),
    def("aggz", "Aggregate Z-Score (AggZ, Varadi)", M, AZ, &["aggz"], PN, NO, "Varadi (CSS Analytics, 19 March 2010, as quoted in the Wealth-Lab forum thread 30490): z(n) = (close − sma(close, n))/stdev(close, n); the post does not say sample or population: population here (as a TradeStation reply on the post, StandardDev(c, len, 1)); the Wealth-Lab demo uses the sample deviation (n − 1), which scales each z by √((n − 1)/n); aggz = (−z(10) + z(200))/2; zero deviation gives no value; above 0 bullish; defaults 10, 200", &[199], false, aggz),
    def("blau_cmi", "Candlestick Momentum Index (Blau)", M, CM, &["cmi"], PN, NO, "Blau (Momentum, Direction, and Divergence, 1995), MQL5 article 190 / Blau_CMI (q 1): cmi = 100·ema(ema(ema(close − open, r), s), u)/ema(ema(ema(|close − open|, r), s), u), −100..100; a zero denominator gives 0 (as the MQL5 article 190 states); EMAs alpha 2/(n + 1) seeded with the SMA of their first n values; defaults r 20, s 5, u 3; guide lines ±25", &[25], false, blau_cmi),
    def("dynamo", "DYNAMO Dynamic Momentum Oscillator (Wall)", M, DN, &["dynamo"], PN, NO, "Wall (Futures, July 1996), MetaStock form 50 − (Mov(RSI(14), 21, S) − RSI(14)) as in M. Yalcin's open-source TradingView script (YwtYfzlv): dynamo = Mc − (sma(O, m) − O) with O = rsi(close, n) (lab rsi, Wilder); defaults n 14, m 21, Mc 50", &[34], false, dynamo),
    def("relative_trend_index", "Relative Trend Index (Zeiierman)", M, RT, &["rti", "signal"], PN, NO, "Zeiierman, TradingView open-source RTI (VwmUNNwp): s = stdev(close, 2) (population); over the last L bars sort close + s and close − s; upper = sorted(close + s)[round(p/100·L) − 1], lower = sorted(close − s)[round((100 − p)/100·L) − 1] (round half away from zero); rti = 100·(close − lower)/(upper − lower) (not bounded); signal = ema(rti, 20); equal upper and lower give no value; L ≥ 50 here (the script allows 10, where an index can fall outside the window); defaults L 100, p 95, signal 20; guide lines 20/50/80", &[100, 119], false, rti),
    listed("sctr", "StockCharts Technical Rank (SCTR)", M, PAIR, "StockCharts SCTR: score = 30% close/ema(200) − 1, 30% roc(125), 15% close/ema(50) − 1, 15% roc(20), 5% 3-day slope of the PPO(12, 26, 9) histogram, 5% rsi(14); SCTR = the score's percentile rank 0..99.9 within its peer group (large, mid or small caps); listed only", "not available yet — needs a ranked stock universe (every peer's score on the same day); the rank is the indicator"),
    listed("relative_currency_strength", "Relative Currency Strength", M, PAIR, "currency strength: each currency's average change against the other currencies of a basket of pairs; listed only", "not available yet — needs a basket of currency-pair prices (all crosses of the currencies compared)"),
    listed("stress_indicator", "Stress Indicator (Kaufman)", M, PAIR, "Kaufman (S&C March 2014, thinkorswim StressIndicator): stochastic of the stock and of the index over n (highest high and lowest low of each), then a stochastic of their difference over n; 0..100, guide lines 10/90; listed only", "not available yet — needs the second instrument's high and low (the lab holds its close only)"),
    unpublished("rrg_relative_strength", "RRG Relative Strength (JdK RS-Ratio / RS-Momentum)", "RRG Research (de Kempenaer): RS-Ratio = normalised trend of close/benchmark around 100, RS-Momentum = its rate of change around 100; lengths and normalisation unpublished", "not available yet — formula not published (RRG Research)"),
    unpublished("weighted_alpha", "Weighted Alpha (Barchart)", "Barchart: one-year price change with more weight on recent activity; the weighting is not published", "not available yet — formula not published (Barchart)"),
    unpublished("market_forecast", "Market Forecast (Investools)", "Investools Market Forecast: momentum, intermediate and near-term stochastic-style lines; the lengths and smoothing are not published", "not available yet — formula not published (Investools)"),
    unpublished("ttm_wave", "TTM Wave (Carter)", "John Carter / Simpler Trading TTM Wave A, B, C: MACD-like waves of undisclosed lengths", "not available yet — formula not published (Simpler Trading, John Carter)"),
    unpublished("d_momentum_pullback", "D Momentum Pullback", "a D-series proprietary momentum pullback study", "not available yet — formula not published (D-series vendor)"),
    unpublished("gonogo_oscillator", "GoNoGo Oscillator", "GoNoGo Charts: a composite momentum oscillator of undisclosed components", "not available yet — formula not published (GoNoGo Charts)"),
    unpublished("swami_intraday_fisher", "Swami Intraday Fisher", "SwamiCharts (Ehlers & Way; thinkorswim, eSignal): a Fisher-transform heatmap over a range of lookbacks for intraday prices; the computation is not published", "not available yet — formula not published (SwamiCharts, Ehlers & Way)"),
    unpublished("swami_intraday_impulse", "Swami Intraday Impulse", "SwamiCharts (Ehlers & Way; thinkorswim, eSignal): the impulse response of intraday prices as a heatmap over lookbacks (digital signal processing, unpublished)", "not available yet — formula not published (SwamiCharts, Ehlers & Way)"),
    unpublished("swami_relative_performance", "Swami Relative Performance", "SwamiCharts (Ehlers & Way; thinkorswim, eSignal): relative performance against a reference as a heatmap over lookbacks; the computation is not published", "not available yet — formula not published (SwamiCharts, Ehlers & Way)"),
    unpublished("swami_swing_wave", "Swami Swing Wave", "SwamiCharts (Ehlers & Way; thinkorswim, eSignal): short-term wave peaks and troughs isolated by digital signal processing, shown over lookbacks; the computation is not published", "not available yet — formula not published (SwamiCharts, Ehlers & Way)"),
    unpublished("kase_peak_oscillator", "Kase Peak Oscillator", "Kase (Trading with the Odds, 1996): momentum from volatility-normalised log up- and down-moves; Kase & Company does not publish the statistics (only community reconstructions exist)", "not available yet — formula not published (Kase & Company)"),
    unpublished("king_oscillator", "King Oscillator", "Valvank Tomar, TradingView protected-source script: an EMA(13) of a 'King Force Index'; the code is hidden", "not available yet — formula not published (protected TradingView script, Valvank Tomar)"),
    unpublished("optimized_balance_conviction", "Optimized Balance Conviction (TradeStation)", "TradeStation OBC: running total of volume·ohlc4 added on higher closes and subtracted on lower closes, with intensity weighting and compression above a threshold that the help page does not define", "not available yet — formula not published (TradeStation; intensity and compression undefined)"),
    unpublished("rohit_momentum", "Rohit's Momentum Indicator", "strike.money commercial momentum indicator; the formula is not published", "not available yet — formula not published (strike.money)"),
    unpublished("prime_number_oscillator", "Prime Number Oscillator", "Modulus Financial Engineering: distance from price to a nearby prime number; the owner's rule is not published, and the community ports disagree (nearest vs farther prime, a tolerance band) and only work on whole-number prices", "not available yet — formula not published (Modulus Financial Engineering)"),
    unpublished("spectrum_bars", "Spectrum Bars", "thinkorswim SpectrumBars study; no calculation is published", "not available yet — formula not published (thinkorswim)"),
    unpublished("turbo_stochastic", "Turbo Stochastic", "Omega Research (1996) Turbo Stochastic Fast/Slow, restated by MotiveWave: a linear-regression projection of %K and %D by a 'turbo' number of bars through an undefined linRegLine helper", "not available yet — formula not published (Omega Research, 1996)"),
    unpublished("fast_slow_rsi", "Fast and Slow RSI (FSRS)", "a fast and a slow RSI pair; no primary text defines FSRS (the lab has rsi and Apirine's slow_rsi)", "not available yet — formula not published (no primary source found)"),
    unpublished("move_tracker", "Move Tracker", "no primary text found", "not available yet — formula not published (no primary source found)"),
    unpublished("harmonic_oscillator", "Harmonic Oscillator", "no primary text found (not the lab's harmonic XABCD patterns)", "not available yet — formula not published (no primary source found)"),
    unpublished("high_low_momentum_index", "High Low Momentum Index", "no primary text found", "not available yet — formula not published (no primary source found)"),
    unpublished("price_cycle_oscillator", "Price Cycle Oscillator (PCO)", "no primary text found", "not available yet — formula not published (no primary source found)"),
    unpublished("price_strength_index", "Price Strength Index", "no primary text found", "not available yet — formula not published (no primary source found)"),
    unpublished("relative_conviction_momentum", "Relative Conviction Momentum", "no primary text found", "not available yet — formula not published (no primary source found)"),
    unpublished("rondeau_reversion", "Rondeau's Reversion", "no primary text found", "not available yet — formula not published (no primary source found)"),
    unpublished("shadow_rate_indicator", "Shadow Rate Indicator (SRI)", "no primary text found", "not available yet — formula not published (no primary source found)"),
    unpublished("support_resistance_oscillator", "Support and Resistance Oscillator", "no primary text found", "not available yet — formula not published (no primary source found)"),
    unpublished("wick_oscillator", "Wick Oscillator", "no primary text found", "not available yet — formula not published (no primary source found)"),
    unpublished("twiggs_momentum", "Twiggs Momentum Oscillator", "Colin Twiggs (Incredible Charts): a smoothed rate of change, 21 days, ±20% levels; the smoothing is proprietary", "not available yet — formula not published (Incredible Charts, Colin Twiggs)"),
    unpublished("twiggs_smoothed_momentum", "Twiggs Smoothed Momentum", "Colin Twiggs (Incredible Charts): a further-smoothed Twiggs Momentum; the smoothing is proprietary", "not available yet — formula not published (Incredible Charts, Colin Twiggs)"),
];
/// (id, platform names, AmiBroker names): one row per id in DEFS.
pub const SOURCES: &[(&str, &str, &str)] = &[
    ("dymi", "MotiveWave: Dynamic Momentum Index; Infront: Dynamic Momentum Index", ""),
    ("hacolt", "TradingView (LazyBear): HACOLT; thinkorswim: HACOLT", ""),
    ("ergodic_macd", "MQL5: Blau_Ergodic_MACD", ""),
    ("ergodic_mdi", "MQL5: Blau_Ergodic_MDI", ""),
    ("rsmk", "TradeStation / MetaStock (S&C Traders' Tips Mar 2020): RSMK; thinkorswim: RSMK", ""),
    ("rdx", "TradingView (traderharikrishna): RDX", ""),
    ("stochastic_distance_osc", "TradingView (PineCodersTASC): TASC 2023.06 Stochastic Distance Oscillator; thinkorswim: StochasticDistanceOsc", ""),
    ("wami", "eSignal: WAMI; MotiveWave: WAMI; MultiCharts: WAMI", ""),
    ("aggz", "Wealth-Lab: AggZ", ""),
    ("blau_cmi", "MQL5 / FXCodeBase: Blau_CMI", ""),
    ("dynamo", "TradingView (M. Yalcin): Dynamic Momentum Oscillator", ""),
    ("relative_trend_index", "TradingView (Zeiierman): Relative Trend Index (RTI)", ""),
    ("sctr", "StockCharts: SCTR", ""),
    ("relative_currency_strength", "census: Relative Currency Strength", ""),
    ("stress_indicator", "thinkorswim: StressIndicator", ""),
    ("rrg_relative_strength", "StockCharts: RRG Relative Strength", ""),
    ("weighted_alpha", "Barchart: Weighted Alpha", ""),
    ("market_forecast", "Investools: Market Forecast", ""),
    ("ttm_wave", "thinkorswim / Simpler Trading: TTM_Wave", ""),
    ("d_momentum_pullback", "census: D Momentum Pullback", ""),
    ("gonogo_oscillator", "GoNoGo Charts: GoNoGo Oscillator", ""),
    ("swami_intraday_fisher", "thinkorswim: SwamiIntradayFisher", ""),
    ("swami_intraday_impulse", "thinkorswim: SwamiIntradayImpulse", ""),
    ("swami_relative_performance", "thinkorswim: SwamiRelativePerformance", ""),
    ("swami_swing_wave", "thinkorswim: SwamiSwingWave", ""),
    ("kase_peak_oscillator", "ProRealCode (community): Kase Peak Oscillator", ""),
    ("king_oscillator", "TradingView (protected): Kings Oscillator", ""),
    ("optimized_balance_conviction", "TradeStation: Optimized Balance Conviction", ""),
    ("rohit_momentum", "strike.money: Rohit Momentum Indicator", ""),
    ("prime_number_oscillator", "TradingView / FXCodeBase (community ports): Prime Number Oscillator", ""),
    ("spectrum_bars", "thinkorswim: SpectrumBars", ""),
    ("turbo_stochastic", "MotiveWave: Turbo Stochastic Fast / Slow", ""),
    ("fast_slow_rsi", "census: Fast and Slow RSI", ""),
    ("move_tracker", "census: Move Tracker", ""),
    ("harmonic_oscillator", "census: Harmonic Oscillator", ""),
    ("high_low_momentum_index", "census: High Low Momentum Index", ""),
    ("price_cycle_oscillator", "census: Price Cycle Oscillator", ""),
    ("price_strength_index", "census: Price Strength Index", ""),
    ("relative_conviction_momentum", "census: Relative Conviction Momentum", ""),
    ("rondeau_reversion", "census: Rondeau's Reversion", ""),
    ("shadow_rate_indicator", "census: Shadow Rate Indicator", ""),
    ("support_resistance_oscillator", "census: Support and Resistance Oscillator", ""),
    ("wick_oscillator", "census: Wick Oscillator", ""),
    ("twiggs_momentum", "Incredible Charts: Twiggs Momentum Oscillator", ""),
    ("twiggs_smoothed_momentum", "Incredible Charts: Twiggs Smoothed Momentum", ""),
];

#[cfg(test)]
mod tests {
    use crate::lab::catalogue::{get, run, ParamSpec};
    use crate::lab::testdata::{assert_series, close_eq, closes, compiles, cut, from_ohlc, go, walk};
    const N: f64 = f64::NAN;

    #[test] fn dymi_window_length_follows_volatility() {
        let kv = [("sd_length", 2.), ("avg_length", 2.), ("base_length", 3.), ("min_length", 2.), ("max_length", 5.)];
        // σ(2) = |Δ|/2: 0.5 until bar 6; bar 7: σ 1, avg 0.75, vi 4/3, td int(2.25) = 2
        let o = go("dymi", &closes(&[1., 2., 3., 4., 5., 6., 5., 3.]), &kv);
        // bar 5: td 3, all up; bar 6: +1 +1 −1; bar 7 (td 2): −1 −2 → 0 (td 3 would give 25)
        assert_series(&o[0], &[N, N, N, N, N, 100., 200. / 3., 0.]);
        assert!(go("dymi", &closes(&[7.; 40]), &[])[0].iter().all(|v| v.is_nan()));
    }
    #[test] fn hacolt_hand_trace() {
        // tema 1 and ema 1: the smoothings are the series themselves and the close never sits below its EMA
        let b = from_ohlc(&[(10., 10.5, 8.5, 9.), (9., 9.2, 7.8, 8.), (8., 10.2, 7.9, 10.), (10., 12., 9.8, 11.8)]);
        // haO 9.5, 9, 9.0125, 9.95625; haC 9.5, 8.625, 9.034375, 10.6640625; utr on every bar (bar 0 via hlS ≥ haS, 9.5 = 9.5), dtr on bars 1-2 only, so bar 3 is not dtr, dtr[1], utr → upw
        assert_series(&go("hacolt", &b, &[("tema_length", 1.), ("ema_length", 1.)])[0], &[-1., -1., -1., 1.]);
        // a rise from the first bar never turns the state (the port starts short)
        let up: Vec<(f64, f64, f64, f64)> = (0..10).map(|i| { let x = 10. + i as f64; (x, x, x, x) }).collect();
        assert!(go("hacolt", &from_ohlc(&up), &[("tema_length", 1.), ("ema_length", 1.)])[0].iter().all(|&v| v == -1.));
    }
    #[test] fn ergodic_macd_and_mdi_hand_values() {
        // ema(close, 2): bar 1 seed 2, bar 2 = 2 + (2/3)·4; close − ema: 1, 4/3
        let b = closes(&[1., 3., 6.]);
        let o = go("ergodic_macd", &b, &[("slow_length", 2.), ("fast_length", 1.), ("smooth_length", 2.), ("signal_length", 1.)]);
        assert_series(&o[0], &[N, N, 7. / 6.]);
        assert_series(&o[1], &[N, N, 7. / 6.]);
        let d = go("ergodic_mdi", &b, &[("ema_length", 2.), ("smooth1", 1.), ("smooth2", 1.), ("signal_length", 2.)]);
        assert_series(&d[0], &[N, 1., 4. / 3.]);
        assert_series(&d[1], &[N, N, 7. / 6.]);
    }
    #[test] fn rsmk_log_ratio_momentum_and_causality() {
        let mut b = closes(&[10., 20., 40.]);
        b.second = Some(vec![10., 10., 10.]);
        let l2 = 100. * 2f64.ln();
        assert_series(&go("rsmk", &b, &[("length", 1.), ("ema_length", 1.)])[0], &[N, l2, l2]);
        assert_series(&go("rsmk", &b, &[("length", 1.), ("ema_length", 2.)])[0], &[N, N, l2]);
        let d = get("rsmk").unwrap();
        assert_eq!(run(&d, &closes(&[1., 2.]), &Default::default()).unwrap_err(), "needs a second instrument");
        let mut full = walk(200, 4, false);
        full.second = Some((0..200).map(|i| 50. + (i % 17) as f64).collect());
        let want = go("rsmk", &full, &[]);
        assert!(want[0][91].is_nan() && want[0][92].is_finite());
        for k in (95..200).step_by(7) {
            let mut pre = cut(&full, k);
            pre.second = Some(full.second.as_ref().unwrap()[..k].to_vec());
            let got = go("rsmk", &pre, &[]);
            for i in 0..k { assert!(close_eq(got[0][i], want[0][i]), "bar {i} cut {k}"); }
        }
    }
    #[test] fn rdx_adds_adx_strength_above_20() {
        let b = walk(300, 3, false);
        let (o, rsi, adx) = (go("rdx", &b, &[]), go("rsi", &b, &[]).remove(0), go("dmi", &b, &[]).remove(2));
        assert_series(&o[1], &rsi);
        let mut seen = (false, false);
        for i in 27..300 {
            let want = rsi[i] + if adx[i] > 20. { seen.0 = true; (adx[i] - 20.) / 5. } else { seen.1 = true; 0. };
            assert!(close_eq(o[0][i], want), "bar {i}");
        }
        assert!(seen.0 && seen.1, "both branches covered");
        assert!(o[0][26].is_nan());
    }
    #[test] fn sdo_hand_values() {
        // n 1, lookback 2, ema 1: dist 1, 2, 1, 3 from bar 1; bar 2: (2 − 1)/1 up; bar 3: (1 − 1)/1; bar 4: (3 − 1)/2 down
        let kv = [("lookback", 2.), ("length", 1.), ("ema_length", 1.)];
        assert_series(&go("stochastic_distance_osc", &closes(&[1., 2., 4., 3., 0.]), &kv)[0], &[N, N, 100., 0., -100.]);
        assert_series(&go("stochastic_distance_osc", &closes(&[5.; 6]), &kv)[0], &[N, N, 0., 0., 0., 0.]);
    }
    #[test] fn wami_hand_values() {
        // mom 2, −1; wma(2) = (2·1 − 1·2)/3 = 0
        assert_series(&go("wami", &closes(&[1., 3., 2.]), &[("wma_length", 2.), ("ema1_length", 1.), ("ema2_length", 1.)])[0], &[N, N, 0.]);
        // mom 2, −1, 4; ema(2) seeds 0.5 then 0.5 + (2/3)·3.5
        assert_series(&go("wami", &closes(&[1., 3., 2., 6.]), &[("wma_length", 1.), ("ema1_length", 2.), ("ema2_length", 1.)])[0], &[N, N, 0.5, 0.5 + 2. / 3. * 3.5]);
    }
    #[test] fn aggz_hand_value() {
        // z(2) at bar 2 = (4 − 3)/1 = 1; z(3) = (4 − 7/3)/(√14/3) = 5/√14
        let o = go("aggz", &closes(&[1., 2., 4.]), &[("short_length", 2.), ("long_length", 3.)]);
        assert_series(&o[0], &[N, N, (-1. + 5. / 14f64.sqrt()) / 2.]);
    }
    #[test] fn blau_cmi_hand_values() {
        let b = from_ohlc(&[(10., 12., 9., 11.), (11., 11., 7., 8.)]); // bodies 1, −3
        assert_series(&go("blau_cmi", &b, &[("r", 1.), ("s", 1.), ("u", 1.)])[0], &[100., -100.]);
        assert_series(&go("blau_cmi", &b, &[("r", 2.), ("s", 1.), ("u", 1.)])[0], &[N, -50.]); // (1 − 3)/2 over (1 + 3)/2
    }
    #[test] fn dynamo_hand_values() {
        // rsi(1): 100 up, 0 down; sma(2) 50 → dynamo = 50 − (50 − rsi) = rsi
        assert_series(&go("dynamo", &closes(&[1., 2., 1., 2.]), &[("rsi_length", 1.), ("ma_length", 2.)])[0], &[N, N, 0., 100.]);
        assert_series(&go("dynamo", &closes(&[1., 2., 1., 2.]), &[("rsi_length", 1.), ("ma_length", 2.), ("midpoint", 0.)])[0], &[N, N, -50., 50.]);
    }
    #[test] fn rti_sorted_band_indices() {
        // close = bar index: s = 0.5, close ± s; at bar 50 the window holds bars 1..50
        let c: Vec<f64> = (0..60).map(f64::from).collect();
        let o = go("relative_trend_index", &closes(&c), &[("trend_length", 50.), ("signal_length", 1.)]);
        assert!(o[0][49].is_nan());
        // p 95: upper index round(47.5) − 1 = 47 → 48.5; lower index round(2.5) − 1 = 2 → 2.5
        assert!(close_eq(o[0][50], 100. * 47.5 / 46.));
        // p 50: both index 24 → 25.5 and 24.5
        assert!(close_eq(go("relative_trend_index", &closes(&c), &[("trend_length", 50.), ("sensitivity", 50.)])[0][50], 2550.));
    }
    #[test] fn dymi_zero_deviation_takes_the_longest_length() {
        // bar 6: σ(2) of 6, 6 is 0 but its average (0.5 + 0)/2 is not: vi 0, td = max 5 (bars 2..6: four rises, one flat)
        let kv = [("sd_length", 2.), ("avg_length", 2.), ("base_length", 3.), ("min_length", 2.), ("max_length", 5.)];
        assert_series(&go("dymi", &closes(&[1., 2., 3., 4., 5., 6., 6.]), &kv)[0], &[N, N, N, N, N, 100., 100.]);
    }
    #[test] fn blau_cmi_zero_denominator_is_zero() {
        let b = from_ohlc(&[(10., 11., 9., 10.), (10., 11., 9., 10.), (10., 12., 9., 11.)]); // bodies 0, 0, 1
        assert_series(&go("blau_cmi", &b, &[("r", 2.), ("s", 1.), ("u", 1.)])[0], &[N, 0., 100.]);
    }
    #[test] fn listed_entries_refuse_with_their_note() {
        for d in super::DEFS.iter().filter(|d| d.compute.is_none()) {
            let e = run(d, &walk(30, 1, false), &Default::default()).unwrap_err();
            assert!(e.starts_with("not available yet — ") || e == "needs a second instrument", "{}: {e}", d.id);
        }
    }
    #[test] fn parameter_bounds_never_panic() {
        for mut bars in [walk(3, 1, true), walk(300, 1, true)] {
            bars.second = Some(vec![7.; bars.len()]);
            for d in super::DEFS.iter().filter(|d| d.compute.is_some()) {
                for pick in [|p: &ParamSpec| p.min, |p: &ParamSpec| p.max] {
                    let kv: Vec<(&str, f64)> = d.params.iter().map(|p| (p.key, pick(p))).collect();
                    assert!(go(d.id, &bars, &kv).iter().all(|s| s.len() == bars.len()), "{}", d.id);
                }
            }
        }
    }
    #[test] fn rules_can_read_every_output() {
        for d in super::DEFS.iter().filter(|d| d.compute.is_some() && !d.needs.second_symbol) { for o in d.outputs { compiles(d.id, o).unwrap_or_else(|e| panic!("{}.{o}: {e:?}", d.id)); } }
    }
    /// CENSUS_BARS=<csv date,open,high,low,close[,volume[,second close]]> CENSUS_OUT=<json> cargo test --bin guru lab::x_census2_a::tests::census_a_dump -- --ignored --exact
    /// Writes {id: {output: [...]}} for every computed entry at default parameters (null = no value); rsmk only when the 7th column exists.
    #[test] #[ignore] fn census_a_dump() {
        use crate::lab::bars::Bars;
        let text = std::fs::read_to_string(std::env::var("CENSUS_BARS").expect("CENSUS_BARS")).expect("read CENSUS_BARS");
        let (mut b, mut vol, mut sec) = (Bars::default(), Vec::new(), Vec::new());
        for (i, line) in text.lines().skip(1).filter(|s| !s.trim().is_empty()).enumerate() {
            let f: Vec<&str> = line.split(',').map(str::trim).collect();
            let x = |k: usize| f.get(k).and_then(|s| s.parse::<f64>().ok()).unwrap_or(f64::NAN);
            b.date.push(f[0].to_string());
            b.open.push(x(1)); b.high.push(x(2)); b.low.push(x(3)); b.close.push(x(4)); b.end.push(i);
            if f.len() > 5 { vol.push(x(5)); }
            if f.len() > 6 { sec.push(x(6)); }
        }
        if vol.len() == b.len() && !vol.is_empty() { b.volume = Some(vol); }
        let paired = sec.len() == b.len() && !sec.is_empty();
        if paired { b.second = Some(sec); }
        let num = |v: f64| if v.is_finite() { format!("{v:?}") } else { "null".into() };
        let body: Vec<String> = super::DEFS.iter().filter(|d| d.compute.is_some() && (paired || !d.needs.second_symbol)).map(|d| {
            let out = run(d, &b, &Default::default()).unwrap_or_else(|e| panic!("{}: {e}", d.id));
            let cols: Vec<String> = d.outputs.iter().zip(&out).map(|(k, s)| format!("\"{k}\":[{}]", s.iter().map(|&v| num(v)).collect::<Vec<_>>().join(","))).collect();
            format!("\"{}\":{{{}}}", d.id, cols.join(","))
        }).collect();
        std::fs::write(std::env::var("CENSUS_OUT").expect("CENSUS_OUT"), format!("{{{}}}", body.join(","))).expect("write CENSUS_OUT");
    }
}
