//! Cross-platform indicators, Momentum (part 1) (2026-10-19 build list: TradingView, AmiBroker, MetaTrader, cTrader, NinjaTrader, TradeStation, thinkorswim, Sierra Chart, MetaStock). Formulas: see each definition string.
use super::bars::Bars;
use super::catalogue::{int, len, n, p, real, IndicatorDef, Needs, Output, ParamSpec, Params, Placement};
use super::ta::{self, NA};

#[allow(clippy::too_many_arguments)]
const fn def(id: &'static str, name: &'static str, category: &'static str, params: &'static [ParamSpec], outputs: &'static [&'static str], placement: Placement,
             needs: Needs, definition: &'static str, first: &'static [usize], sparse: bool, f: fn(&Bars, &Params) -> Output) -> IndicatorDef {
    IndicatorDef { id, name, category, params, outputs, placement, needs, definition, first, sparse, compute: Some(f), note: "" }
}
const M: &str = "Momentum";
const PN: Placement = Placement::Pane;
const NO: Needs = Needs::NONE;

fn div(a: f64, b: f64) -> f64 { if b == 0. || a.is_nan() || b.is_nan() { NA } else { a / b } }
fn zip(a: &[f64], b: &[f64], f: impl Fn(f64, f64) -> f64) -> Vec<f64> { a.iter().zip(b).map(|(&x, &y)| f(x, y)).collect() }
fn map(a: &[f64], f: impl Fn(f64) -> f64) -> Vec<f64> { a.iter().map(|&x| f(x)).collect() }
/// max(v, 0) keeping NaN (f64::max would turn NaN into 0).
fn pos(v: f64) -> f64 { if v.is_nan() { NA } else { v.max(0.) } }
/// Wilder RSI exactly as our `rsi` (rma seeded with the SMA; dn 0 -> 100, up 0 -> 0).
fn rsi(x: &[f64], k: usize) -> Vec<f64> {
    let d = ta::change(x, 1);
    super::osc2::rsi_like(&map(&d, pos), &map(&d, |v| pos(-v)), k)
}
/// 100·(x − lo)/(hi − lo); `zero` where the range is 0.
fn stoch(x: &[f64], hi: &[f64], lo: &[f64], zero: f64) -> Vec<f64> {
    (0..x.len()).map(|i| if x[i].is_nan() || hi[i].is_nan() || lo[i].is_nan() { NA } else if hi[i] == lo[i] { zero } else { 100. * (x[i] - lo[i]) / (hi[i] - lo[i]) }).collect()
}
fn hl_stoch(b: &Bars, k: usize, zero: f64) -> Vec<f64> { stoch(&b.close, &ta::highest(&b.high, k), &ta::lowest(&b.low, k), zero) }
/// (eˣ − 1)/(eˣ + 1) = tanh(x/2), written without overflow.
fn ift(x: f64) -> f64 { (x / 2.).tanh() }
/// One value per bar from bar 1 on (bar 0 has no previous bar).
fn from1(b: &Bars, f: impl Fn(usize) -> f64) -> Vec<f64> { (0..b.len()).map(|i| if i == 0 { NA } else { f(i) }).collect() }

fn brar(b: &Bars, prm: &Params) -> Output {
    let k = n(prm, "length");
    let s = |v: Vec<f64>| ta::sum(&v, k);
    let ar = zip(&s((0..b.len()).map(|i| b.high[i] - b.open[i]).collect()), &s((0..b.len()).map(|i| b.open[i] - b.low[i]).collect()), |a, c| div(100. * a, c));
    let br = zip(&s(from1(b, |i| (b.high[i] - b.close[i - 1]).max(0.))), &s(from1(b, |i| (b.close[i - 1] - b.low[i]).max(0.))), |a, c| div(100. * a, c));
    Ok(vec![ar, br])
}
fn composite_index(b: &Bars, prm: &Params) -> Output {
    let ci = zip(&ta::change(&rsi(&b.close, n(prm, "rsi_long")), n(prm, "mom")), &ta::sma(&rsi(&b.close, n(prm, "rsi_short")), n(prm, "rsi_avg")), |a, c| a + c);
    let (f, s) = (ta::sma(&ci, n(prm, "fast")), ta::sma(&ci, n(prm, "slow")));
    Ok(vec![ci, f, s])
}
fn demarker(b: &Bars, prm: &Params) -> Output {
    let k = n(prm, "length");
    let mx = ta::sma(&from1(b, |i| (b.high[i] - b.high[i - 1]).max(0.)), k);
    let mn = ta::sma(&from1(b, |i| (b.low[i - 1] - b.low[i]).max(0.)), k);
    Ok(vec![zip(&mx, &mn, |a, c| if a.is_nan() || c.is_nan() { NA } else if a + c == 0. { 0. } else { a / (a + c) })])
}
fn dosc(b: &Bars, prm: &Params) -> Output {
    let ds = ta::ema(&ta::ema(&rsi(&b.close, n(prm, "length")), n(prm, "ema1")), n(prm, "ema2"));
    Ok(vec![zip(&ds, &ta::sma(&ds, n(prm, "signal")), |a, c| a - c)])
}
fn double_stoch(b: &Bars, prm: &Params) -> Output {
    let (k, s1, s2) = (n(prm, "length"), n(prm, "smooth1"), n(prm, "smooth2"));
    let dss = if p(prm, "variant") == 1. {
        let e1 = ta::ema(&hl_stoch(b, k, 0.), s1);
        let e2 = ta::ema(&stoch(&e1, &ta::highest(&e1, k), &ta::lowest(&e1, k), 0.), s1);
        ta::ema(&e2, s2)
    } else {
        let (hh, ll) = (ta::highest(&b.high, k), ta::lowest(&b.low, k));
        let ee = |x: Vec<f64>| ta::ema(&ta::ema(&x, s1), s2);
        // thinkorswim: if divisor == 0 then 0
        zip(&ee(zip(&b.close, &ll, |c, l| c - l)), &ee(zip(&hh, &ll, |h, l| h - l)), |a, c| if c == 0. { 0. } else { div(100. * a, c) })
    };
    let sig = ta::ema(&dss, s2);
    Ok(vec![dss, sig])
}
fn gap_momentum(b: &Bars, prm: &Params) -> Output {
    let k = n(prm, "length");
    let g = from1(b, |i| b.open[i] - b.close[i - 1]);
    let r = zip(&ta::sum(&map(&g, pos), k), &ta::sum(&map(&g, |v| pos(-v)), k), |u, d| if u.is_nan() || d.is_nan() { NA } else if d == 0. { 1. } else { 100. * u / d });
    let sig = ta::sma(&r, n(prm, "signal"));
    Ok(vec![r, sig])
}
fn hhlls(b: &Bars, prm: &Params) -> Output {
    let (k, s) = (n(prm, "length"), n(prm, "smooth"));
    let (hh, hl, lh, ll) = (ta::highest(&b.high, k), ta::lowest(&b.high, k), ta::highest(&b.low, k), ta::lowest(&b.low, k));
    let hs = from1(b, |i| if hh[i].is_nan() { NA } else if b.high[i] > b.high[i - 1] { div(b.high[i] - hl[i], hh[i] - hl[i]) } else { 0. });
    let ls = from1(b, |i| if lh[i].is_nan() { NA } else if b.low[i] < b.low[i - 1] { div(lh[i] - b.low[i], lh[i] - ll[i]) } else { 0. });
    Ok(vec![map(&ta::ema(&hs, s), |v| 100. * v), map(&ta::ema(&ls, s), |v| 100. * v)])
}
fn ift_rsi(b: &Bars, prm: &Params) -> Output {
    let w = ta::wma(&map(&rsi(&b.close, n(prm, "length")), |r| 0.1 * (r - 50.)), n(prm, "wma"));
    Ok(vec![map(&w, |v| ift(2. * v))])
}
fn imi(b: &Bars, prm: &Params) -> Output {
    let k = n(prm, "length");
    let up = ta::sum(&(0..b.len()).map(|i| (b.close[i] - b.open[i]).max(0.)).collect::<Vec<_>>(), k);
    let dn = ta::sum(&(0..b.len()).map(|i| (b.open[i] - b.close[i]).max(0.)).collect::<Vec<_>>(), k);
    Ok(vec![zip(&up, &dn, |u, d| if u.is_nan() || d.is_nan() { NA } else if u + d == 0. { 50. } else { 100. * u / (u + d) })])
}
fn impulse_macd(b: &Bars, prm: &Params) -> Output {
    let k = n(prm, "length");
    let (hi, lo) = (ta::rma(&b.high, k), ta::rma(&b.low, k));
    let e1 = ta::ema(&b.hlc3(), k);
    let mi = zip(&e1, &ta::ema(&e1, k), |a, c| 2. * a - c);
    let md: Vec<f64> = (0..b.len()).map(|i| if mi[i].is_nan() || hi[i].is_nan() { NA } else if mi[i] > hi[i] { mi[i] - hi[i] } else if mi[i] < lo[i] { mi[i] - lo[i] } else { 0. }).collect();
    let sig = ta::sma(&md, n(prm, "signal"));
    let hist = zip(&md, &sig, |a, c| a - c);
    Ok(vec![md, sig, hist])
}
fn inertia(b: &Bars, prm: &Params) -> Output {
    let rvi = super::osc2::DEFS.iter().find(|d| d.id == "rvi").and_then(|d| d.compute).ok_or("rvi missing")?;
    let k = p(prm, "rvi_length");
    let r = rvi(b, &[("length".to_string(), k), ("ema".to_string(), k)].into_iter().collect())?.remove(0);
    Ok(vec![ta::linreg(&r, n(prm, "length"), 0)])
}
fn kdj(b: &Bars, prm: &Params) -> Output {
    let k = ta::rma(&hl_stoch(b, n(prm, "length"), NA), n(prm, "k_smooth"));
    let d = ta::rma(&k, n(prm, "d_smooth"));
    let j = zip(&k, &d, |k, d| 3. * k - 2. * d);
    Ok(vec![k, d, j])
}
fn laguerre_rsi(b: &Bars, prm: &Params) -> Output {
    let g = p(prm, "gamma");
    let (mut l, mut out) = ([0.; 4], 0.);
    Ok(vec![b.close.iter().map(|&c| {
        let n0 = (1. - g) * c + g * l[0];
        let n1 = -g * n0 + l[0] + g * l[1];
        let n2 = -g * n1 + l[1] + g * l[2];
        let n3 = -g * n2 + l[2] + g * l[3];
        l = [n0, n1, n2, n3];
        let (mut cu, mut cd) = (0., 0.);
        for d in [n0 - n1, n1 - n2, n2 - n3] { if d >= 0. { cu += d } else { cd -= d } }
        if cu + cd != 0. { out = cu / (cu + cd); }
        out
    }).collect()])
}
fn macd_s(b: &Bars, prm: &Params) -> Vec<f64> { zip(&ta::ema(&b.close, n(prm, "fast")), &ta::ema(&b.close, n(prm, "slow")), |a, c| a - c) }
fn macd_bb(b: &Bars, prm: &Params) -> Output {
    let (m, k, mult) = (macd_s(b, prm), n(prm, "band_length"), p(prm, "mult"));
    let (basis, sd) = (ta::ema(&m, k), ta::stdev(&m, k));
    let up = zip(&basis, &sd, |a, s| a + mult * s);
    let lo = zip(&basis, &sd, |a, s| a - mult * s);
    Ok(vec![m, up, lo])
}
fn macd_leader(b: &Bars, prm: &Params) -> Output {
    let resid = |k: usize| { let e = ta::ema(&b.close, k); ta::ema(&zip(&b.close, &e, |c, e| c - e), k) };
    let m = macd_s(b, prm);
    let (r1, r2) = (resid(n(prm, "fast")), resid(n(prm, "slow")));
    Ok(vec![(0..b.len()).map(|i| m[i] + r1[i] - r2[i]).collect(), m])
}
fn macd_v(b: &Bars, prm: &Params) -> Output {
    let v = zip(&macd_s(b, prm), &ta::rma(&ta::tr(b, true), n(prm, "atr_length")), |m, a| div(100. * m, a));
    let sig = ta::ema(&v, n(prm, "signal"));
    let hist = zip(&v, &sig, |a, c| a - c);
    Ok(vec![v, sig, hist])
}
/// Ehlers' Hann-windowed FIR: Σ_{c=1..k}(1 − cos(2πc/(k+1)))·x[t−c+1] / Σ weights.
fn hann(x: &[f64], k: usize) -> Vec<f64> {
    let w: Vec<f64> = (1..=k).map(|c| 1. - (2. * std::f64::consts::PI * c as f64 / (k + 1) as f64).cos()).collect();
    let coef: f64 = w.iter().sum();
    (0..x.len()).map(|t| if t + 1 < k { NA } else { w.iter().enumerate().map(|(j, wj)| wj * x[t - j]).sum::<f64>() / coef }).collect()
}
fn madh(b: &Bars, prm: &Params) -> Output {
    let s = n(prm, "short");
    let lg = (2 * s + n(prm, "dominant")) / 2; // IntPortion(ShortLength + DominantCycle/2)
    Ok(vec![zip(&hann(&b.close, s), &hann(&b.close, lg), |f1, f2| div(100. * (f1 - f2), f2))])
}
fn sign(x: f64) -> f64 { if x > 0. { 1. } else if x < 0. { -1. } else { 0. } }
fn myrsi_net(b: &Bars, prm: &Params) -> Output {
    let (k, nn, c) = (n(prm, "rsi_length"), n(prm, "net_length"), &b.close);
    let mut prev = 0.; // TradeStation variable seed; held while CU + CD = 0
    let my: Vec<f64> = (0..c.len()).map(|i| if i < k { NA } else {
        let (mut cu, mut cd) = (0., 0.);
        for j in 0..k { let d = c[i - j] - c[i - j - 1]; if d > 0. { cu += d } else if d < 0. { cd -= d } }
        if cu + cd != 0. { prev = (cu - cd) / (cu + cd); }
        prev
    }).collect();
    let den = 0.5 * nn as f64 * (nn as f64 - 1.);
    let net = (0..c.len()).map(|i| if i + 1 < k + nn { NA } else {
        let x = |q: usize| my[i + 1 - q]; // X[q] = MyRSI[q − 1], q = 1 newest
        let mut num = 0.;
        for q in 2..=nn { for r in 1..q { num -= sign(x(q) - x(r)); } }
        num / den
    }).collect();
    Ok(vec![my, net])
}
fn pgo(b: &Bars, prm: &Params) -> Output {
    let k = n(prm, "length");
    Ok(vec![zip(&zip(&b.close, &ta::sma(&b.close, k), |c, s| c - s), &ta::ema(&ta::tr(b, true), k), div)])
}
fn polychrome_mom(b: &Bars, prm: &Params) -> Output {
    let (k, c) = (n(prm, "length"), &b.close);
    let den: f64 = (1..=k).map(|i| (i as f64).sqrt()).sum();
    Ok(vec![(0..c.len()).map(|t| if t < k { NA } else { (1..=k).map(|i| (c[t] - c[t - i]) / (i as f64).sqrt()).sum::<f64>() / den }).collect()])
}
fn preferred_stoch(b: &Bars, prm: &Params) -> Output {
    let fk = hl_stoch(b, n(prm, "length"), NA);
    let fd = ta::ema(&fk, n(prm, "fast_d"));
    let sd = ta::rma(&fd, n(prm, "slow_d"));
    Ok(vec![fk, fd, sd])
}
fn premier(k: Vec<f64>, s: usize) -> Vec<f64> { map(&ta::ema(&ta::ema(&map(&k, |v| 0.1 * (v - 50.)), s), s), ift) }
fn premier_rsi(b: &Bars, prm: &Params) -> Output {
    let (r, sl) = (rsi(&b.close, n(prm, "length")), n(prm, "stoch_length"));
    Ok(vec![premier(stoch(&r, &ta::highest(&r, sl), &ta::lowest(&r, sl), NA), n(prm, "smooth"))])
}
fn premier_stoch(b: &Bars, prm: &Params) -> Output { Ok(vec![premier(hl_stoch(b, n(prm, "length"), 50.), n(prm, "smooth"))]) }

const BRAR: &[ParamSpec] = &[len("length", "Length", 26.)];
const CI: &[ParamSpec] = &[len("rsi_long", "RSI length 1", 14.), len("rsi_short", "RSI length 2", 3.), len("mom", "Momentum length", 9.), len("rsi_avg", "Short RSI average", 3.), len("fast", "Fast average", 13.), len("slow", "Slow average", 33.)];
const L14: &[ParamSpec] = &[len("length", "Length", 14.)];
const DOSC: &[ParamSpec] = &[len("length", "RSI length", 14.), len("ema1", "First EMA", 5.), len("ema2", "Second EMA", 3.), len("signal", "Signal SMA", 9.)];
const DSS: &[ParamSpec] = &[len("length", "Length", 10.), len("smooth1", "First smoothing", 5.), len("smooth2", "Second smoothing", 9.), int("variant", "Variant (0 Blau DSS, 1 Bressert)", 0., 0., 1.)];
const GAP: &[ParamSpec] = &[len("length", "Length", 40.), len("signal", "Signal SMA", 20.)];
const HHLL: &[ParamSpec] = &[int("length", "Length", 20., 2., 2000.), len("smooth", "Smoothing", 20.)]; // length 1: zero high/low range
const IFT: &[ParamSpec] = &[len("length", "RSI length", 5.), len("wma", "WMA length", 9.)];
const IMP: &[ParamSpec] = &[len("length", "MA length", 34.), len("signal", "Signal", 9.)];
const INER: &[ParamSpec] = &[len("length", "Regression length", 20.), int("rvi_length", "RVI length", 14., 2., 2000.)]; // rvi needs stdev over 2+ bars
const KDJ: &[ParamSpec] = &[len("length", "Length", 9.), len("k_smooth", "K smoothing", 3.), len("d_smooth", "D smoothing", 3.)];
const LAG: &[ParamSpec] = &[real("gamma", "Gamma", 0.5, 0., 0.99, 0.01)];
const MBB: &[ParamSpec] = &[len("fast", "Fast", 12.), len("slow", "Slow", 26.), len("band_length", "Band length", 10.), real("mult", "Multiplier", 1., 0.1, 5., 0.1)];
const MLD: &[ParamSpec] = &[len("fast", "Fast", 12.), len("slow", "Slow", 26.)];
const MV: &[ParamSpec] = &[len("fast", "Fast", 12.), len("slow", "Slow", 26.), len("atr_length", "ATR length", 26.), len("signal", "Signal", 9.)];
const MADH: &[ParamSpec] = &[len("short", "Short length", 8.), len("dominant", "Dominant cycle", 27.)];
const NET: &[ParamSpec] = &[len("rsi_length", "MyRSI length", 14.), int("net_length", "NET length", 14., 2., 2000.)]; // length 1: zero pair count
const L8: &[ParamSpec] = &[len("length", "Length", 8.)];
const PREF: &[ParamSpec] = &[len("length", "Length", 10.), len("fast_d", "Fast %D EMA", 3.), len("slow_d", "Slow %D (modified MA)", 3.)];
const PRSI: &[ParamSpec] = &[len("length", "RSI length", 14.), len("stoch_length", "Stochastic length", 8.), len("smooth", "EMA length", 5.)];
const PSO: &[ParamSpec] = &[len("length", "Stochastic length", 8.), len("smooth", "EMA length", 5.)];

pub const DEFS: &[IndicatorDef] = &[
    def("brar", "BRAR (Shinohara Intensity Ratio)", M, BRAR, &["ar", "br"], PN, NO, "ar = 100·sum(high − open, n)/sum(open − low, n); br = 100·sum(max(high − close[1], 0), n)/sum(max(close[1] − low, 0), n); zero denominator → NaN (pandas-ta brar; ChartIQ Shinohara 'strong' = br)", &[25, 26], false, brar),
    def("composite_index", "Composite Index (Connie Brown)", M, CI, &["ci", "fast", "slow"], PN, NO, "ci = (rsi(close, rsi_long) − rsi(close, rsi_long)[mom]) + sma(rsi(close, rsi_short), rsi_avg); fast = sma(ci, fast); slow = sma(ci, slow) (Connie Brown's TradeStation formula: RSIMO9 = Momentum(RSI(C, 14), 9), RSI3 = Average(RSI(C, 3), 3), Plot1 = RSIMO9 + RSI3, Plot2 = Average(Plot1, 13), Plot3 = Average(Plot1, 33); Wilder RSI as TradeStation; Sierra Chart's study defaults its RSI averaging to simple — not followed)", &[23, 35, 55], false, composite_index),
    def("demarker", "DeMarker", M, L14, &["dem"], PN, NO, "demax = max(high − high[1], 0); demin = max(low[1] − low, 0); dem = sma(demax, n)/(sma(demax, n) + sma(demin, n)), 0 when the denominator is 0 (MetaTrader 5 DeMarker.mq5: SimpleMA of the DeMax/DeMin buffers, dnum != 0 ? avgmax/dnum : 0.0, period 14, drawn from bar n; 0..1; verified)", &[14], false, demarker),
    def("dosc", "Derivative Oscillator", M, DOSC, &["dosc"], PN, NO, "ds = ema(ema(rsi(close, length), ema1), ema2); dosc = ds − sma(ds, signal) (TA-Lib DOSC; Wilder RSI, SMA-seeded EMAs)", &[28], false, dosc),
    def("double_stoch", "Double Smoothed Stochastic", M, DSS, &["dss", "signal"], PN, NO, "variant 0 (Blau; thinkorswim DoubleSmoothedStochastic source: DSS = divisor == 0 ? 0 : 100·ExpAverage(ExpAverage(close − Lowest(low, length), sLength), rLength)/divisor, divisor = ExpAverage(ExpAverage(Highest(high, length) − Lowest(low, length), sLength), rLength); smooth1 = sLength 5, smooth2 = rLength 9, length 10 as thinkorswim; our EMAs seed with the SMA); variant 1 (Sierra Chart Double Stochastic - Bressert): k1 = stoch(close, n) (0 on zero range), e1 = ema(k1, smooth1), e2 = ema(stoch(e1, n), smooth1), dss = ema(e2, smooth2) (NinjaTrader 8 Double Stochastics uses 3/3 and holds the previous value on a zero range); signal = ema(dss, smooth2) is a LAB ADDITION — neither thinkorswim nor Sierra Chart plots a signal line", &[21, 29], false, double_stoch),
    def("gap_momentum", "Gap Momentum (Kaufman)", M, GAP, &["gap_ratio", "signal"], PN, NO, "g = open − close[1]; up = sum(max(g, 0), n); dn = sum(max(−g, 0), n); gap_ratio = dn = 0 ? 1 : 100·up/dn; signal = sma(gap_ratio, signal) (Kaufman, TASC Jan 2024 Traders' Tips: PineCoders Pine gapRatio = gapsDn == 0 ? 1.0 : 100.0·gapsUp/gapsDn, signal = ta.sma(gapRatio, signalPeriod), defaults 40/20; MetaQuotes and MetaStock tips identical; Kaufman's EasyLanguage sums gaps from bar 1, one bar behind — not followed)", &[40, 59], false, gap_momentum),
    def("hhlls", "Higher High Lower Low Stochastic (Apirine)", M, HHLL, &["hhs", "lls"], PN, NO, "hh = high > high[1] ? (high − ll(high, n))/(hh(high, n) − ll(high, n)) : 0; ll = low < low[1] ? (hh(low, n) − low)/(hh(low, n) − ll(low, n)) : 0; hhs = 100·ema(hh, smooth); lls = 100·ema(ll, smooth) (Apirine TASC Feb 2016 Traders' Tips EasyLanguage _HHLLS: XAverage over Length 20; our EMA seeds with the SMA, XAverage with the first value)", &[38, 38], false, hhlls),
    def("ift_rsi", "Inverse Fisher Transform of RSI", M, IFT, &["ift"], PN, NO, "v = 0.1·(rsi(close, length) − 50); w = wma(v, wma); ift = (e^{2w} − 1)/(e^{2w} + 1) (Sierra Chart Inverse Fisher Transform with RSI; Vervoort/Ehlers)", &[13], false, ift_rsi),
    def("imi", "Intraday Momentum Index", M, L14, &["imi"], PN, NO, "up = sum(max(close − open, 0), n); dn = sum(max(open − close, 0), n); imi = 100·up/(up + dn), 50 when both are 0 (TA-Lib IMI)", &[13], false, imi),
    def("impulse_macd", "Impulse MACD (LazyBear)", M, IMP, &["md", "signal", "hist"], PN, NO, "hi = rma(high, n); lo = rma(low, n); e = ema(hlc3, n); mi = 2e − ema(e, n); md = mi > hi ? mi − hi : mi < lo ? mi − lo : 0; signal = sma(md, signal); hist = md − signal (TradingView public Pine 'Impulse MACD [LazyBear]': calc_smma = na(smma[1]) ? sma : (smma[1]·(len − 1) + src)/len = our rma; calc_zlema = ema1 + (ema1 − ema(ema1)); lengths 34/9; verified line by line)", &[66, 74, 74], false, impulse_macd),
    def("inertia", "Inertia (Dorsey)", M, INER, &["inertia"], PN, NO, "linreg(rvi(close), length, 0) with our rvi (TradingView Relative Volatility Index) at stdev length = ema length = rvi_length (pandas-ta inertia default; MetaStock/Sierra Chart use Dorsey's refined RVI of high and low — variant not built); warm-up follows rvi (data-dependent)", &[usize::MAX], true, inertia),
    def("kdj", "KDJ", M, KDJ, &["k", "d", "j"], PN, NO, "rsv = 100·(close − ll(low, n))/(hh(high, n) − ll(low, n)) (zero range → NaN); k = rma(rsv, k_smooth); d = rma(k, d_smooth); j = 3k − 2d (TA-Lib/pandas-ta; rma SMA-seeded, the classic Chinese form seeds k = d = 50)", &[10, 12, 12], false, kdj),
    def("laguerre_rsi", "Laguerre RSI (Ehlers)", M, LAG, &["lrsi"], PN, NO, "l0 = (1 − γ)·close + γ·l0[1]; l1 = −γ·l0 + l0[1] + γ·l1[1]; l2, l3 likewise (all start at 0, as Ehlers' code); cu = Σ positive (l0 − l1), (l1 − l2), (l2 − l3), cd = Σ |negative|; lrsi = cu/(cu + cd), previous value when cu + cd = 0 (Ehlers, Cybernetic Analysis fig. 14.8; Sierra Chart Laguerre RSI shows 0 there)", &[0], false, laguerre_rsi),
    def("macd_bb", "MACD Bollinger Bands", M, MBB, &["macd", "upper", "lower"], PN, NO, "macd = ema(close, fast) − ema(close, slow); upper/lower = ema(macd, band_length) ± mult·stdev(macd, band_length) (population) (Sierra Chart MACD Bollinger Bands - Standard, default EMA basis)", &[25, 34, 34], false, macd_bb),
    def("macd_leader", "MACD Leader", M, MLD, &["leader", "macd"], PN, NO, "macd = ema(close, fast) − ema(close, slow); leader = macd + ema(close − ema(close, fast), fast) − ema(close − ema(close, slow), slow) (Sierra Chart MACD Leader; Siligardos)", &[50, 25], false, macd_leader),
    def("macd_v", "MACD-V (volatility-normalised MACD)", M, MV, &["macdv", "signal", "hist"], PN, NO, "macdv = 100·(ema(close, fast) − ema(close, slow))/atr(atr_length) (our Wilder atr, bar-0 TR = high − low); signal = ema(macdv, signal); hist = macdv − signal (TA4J VolatilityNormalizedMACDIndicator; Spiroglou)", &[25, 33, 33], false, macd_v),
    def("madh", "Moving Average Difference - Hann (Ehlers)", M, MADH, &["madh"], PN, NO, "long = int(short + dominant/2); hann(x, N) = Σ_{c=1..N}(1 − cos(2πc/(N+1)))·x[c−1]/Σ weights; madh = 100·(hann(close, short) − hann(close, long))/hann(close, long) (Ehlers TASC Nov 2021 Traders' Tips EasyLanguage)", &[20], false, madh),
    def("myrsi_net", "Ehlers MyRSI with Noise Elimination (NET)", M, NET, &["myrsi", "net"], PN, NO, "myrsi = (cu − cd)/(cu + cd) over the last rsi_length one-bar close changes (held when cu + cd = 0); net = Σ_{q=2..N} Σ_{r<q} −sign(X[q] − X[r]) / (N(N−1)/2), X[q] = myrsi[q−1] (Kendall tau vs time) (Ehlers TASC Dec 2020 EasyLanguage)", &[14, 27], false, myrsi_net),
    def("pgo", "Pretty Good Oscillator", M, L14, &["pgo"], PN, NO, "(close − sma(close, n))/ema(true range, n) (Mark Johnson's TradeStation code PGO = (C − Average(C, 89))/XAverage(TrueRange, 89); Trading Technologies PGO, same formula with N = 14; default 14 declared, Johnson used 89; bar-0 TR = high − low; our EMA seeds with the SMA, XAverage with the first value; pandas-ta divides by ema(atr(n), n) instead — not followed)", &[13], false, pgo),
    def("polychrome_mom", "Polychrome Momentum", M, L8, &["pmtm"], PN, NO, "Σ_{i=1..n}(close − close[i])/√i ÷ Σ_{i=1..n}√i (thinkorswim PolychromMtm source: num = fold i = 1 to length + 1 of (price − price[i])/Sqrt(i), denom = fold of Sqrt(i), PM = num/denom; default length 8 as thinkorswim; its BuyLevel 0.2 / SellLevel −0.04 lines not plotted)", &[8], false, polychrome_mom),
    def("preferred_stoch", "Preferred Stochastic (DiNapoli)", M, PREF, &["fast_k", "fast_d", "slow_d"], PN, NO, "fast_k = 100·(close − ll(low, n))/(hh(high, n) − ll(low, n)) (zero range → NaN); fast_d = ema(fast_k, fast_d); slow_d = slow_d[1] + (fast_d − slow_d[1])/slow_d (modified MA = rma, SMA seed) (Sierra Chart Preferred Stochastic - DiNapoli)", &[9, 11, 13], false, preferred_stoch),
    def("premier_rsi", "Premier RSI Oscillator", M, PRSI, &["pro"], PN, NO, "r = rsi(close, length); sk = 100·(r − ll(r, stoch_length))/(hh(r, stoch_length) − ll(r, stoch_length)) (zero range → NaN, as Pine); ss = ema(ema(0.1·(sk − 50), smooth), smooth); pro = (e^{ss} − 1)/(e^{ss} + 1) (TradingView public Pine 'Premier RSI Oscillator [LazyBear]': sk = stoch(r, r, r, 8), len = round(sqrt(smoothlen 25)) = 5 — our smooth is that EMA length directly, ss = ema(ema(0.1·(sk − 50), len), len), expss = exp(ss); verified line by line)", &[23], false, premier_rsi),
    def("premier_stoch", "Premier Stochastic Oscillator", M, PSO, &["pso"], PN, NO, "k = 100·(close − ll(low, n))/(hh(high, n) − ll(low, n)) (50 on zero range, as TA-Lib); ss = ema(ema(0.1·(k − 50), smooth), smooth); pso = (e^{ss} − 1)/(e^{ss} + 1) (TA-Lib PSO; Leibfarth TASC Aug 2008)", &[15], false, premier_stoch),
];
/// (id, platform names, AmiBroker names): one row per id in DEFS.
pub const SOURCES: &[(&str, &str, &str)] = &[
    ("brar", "pandas-ta: brar; Zerodha Kite (ChartIQ): Shinohara Intensity Ratio", ""),
    ("composite_index", "Sierra Chart: Connie Brown Composite Index", ""),
    ("demarker", "MetaTrader 4: DeMarker; MetaTrader 5: DeMarker; cTrader: DeMarker; Sierra Chart: DeMarker Oscillator; thinkorswim: DeMarkIndicator", ""),
    ("dosc", "TA-Lib: DOSC", ""),
    ("double_stoch", "NinjaTrader 8: Double Stochastics; thinkorswim: DoubleSmoothedStochastics; Sierra Chart: Double Stochastic, Double Stochastic - Bressert; TradeStation: Stochastic Double Slow", ""),
    ("gap_momentum", "thinkorswim: GapMomentum", ""),
    ("hhlls", "thinkorswim: HHLLS", ""),
    ("ift_rsi", "Sierra Chart: Inverse Fisher Transform with RSI; MetaStock: IFT RSI; thinkorswim: InverseFisherTransform RSI", ""),
    ("imi", "TA-Lib: IMI; TA4J: IntraDayMomentumIndexIndicator; MetaStock: imi(); thinkorswim: IntradayMomentumIndex; Zerodha Kite (ChartIQ): Intraday Momentum Index", ""),
    ("impulse_macd", "TradingView community scripts (India): Impulse MACD [LazyBear]", ""),
    ("inertia", "pandas-ta: inertia; MetaStock: inertia(); Sierra Chart: Inertia; thinkorswim: Inertia", ""),
    ("kdj", "TA-Lib: KDJ; pandas-ta: kdj; Sierra Chart: KDJ", ""),
    ("laguerre_rsi", "Sierra Chart: Laguerre RSI; TradingView community scripts (India): Laguerre RSI", ""),
    ("macd_bb", "Sierra Chart: MACD Bollinger Bands - Standard", ""),
    ("macd_leader", "Sierra Chart: MACD Leader", ""),
    ("macd_v", "TA4J: VolatilityNormalizedMACDIndicator", ""),
    ("madh", "thinkorswim: MADH", ""),
    ("myrsi_net", "thinkorswim: MyRSIWithNET", ""),
    ("pgo", "pandas-ta: pgo; Zerodha Kite (ChartIQ): Pretty Good Oscillator", ""),
    ("polychrome_mom", "thinkorswim: PolychromMtm", ""),
    ("preferred_stoch", "Sierra Chart: Preferred Stochastic - DiNapoli", ""),
    ("premier_rsi", "Dhan: Premier RSI Oscillator; TradingView community scripts (India): Premier RSI Oscillator [LazyBear]", ""),
    ("premier_stoch", "TA-Lib: PSO; Sierra Chart: Premier Stochastic Oscillator; Dhan: Premier Stochastic Oscillator", ""),
];

#[cfg(test)]
mod tests {
    use crate::lab::testdata::{assert_series, closes, d1, from_ohlc, go, walk};
    const N: f64 = f64::NAN;
    fn ramp(k: usize) -> crate::lab::bars::Bars { closes(&(0..k).map(|i| 100. + i as f64).collect::<Vec<_>>()) }
    fn near(a: f64, b: f64) -> bool { (a - b).abs() < 1e-9 }
    fn flat(k: usize, o: f64, h: f64, l: f64, c: f64) -> crate::lab::bars::Bars { from_ohlc(&vec![(o, h, l, c); k]) }

    #[test] fn brar_symmetric_bars_give_100() {
        let b = from_ohlc(&(0..5).map(|i| { let o = 10. + i as f64; (o, o + 1., o - 1., o) }).collect::<Vec<_>>());
        let o = go("brar", &b, &[("length", 3.)]);
        assert_series(&o[0], &[N, N, 100., 100., 100.]);
        // H − C[1] = 2, C[1] − L = 0 on this +1 staircase → br denominator 0 → NaN
        assert_series(&o[1], &[N, N, N, N, N]);
    }
    #[test] fn composite_index_on_a_steady_uptrend_is_100() {
        let o = go("composite_index", &ramp(80), &[]);
        assert!(o[0][22].is_nan() && near(o[0][23], 100.) && near(o[1][35], 100.) && near(o[2][55], 100.));
    }
    #[test] fn demarker_hand_values() {
        // d1 highs 11,12,12.5,12.2,11.5,11,12,13 → demax 1,.5,0,0,0,1,1; lows 9,10,11,10.8,10,9.5,10.5,11.5 → demin 0,0,.2,.8,.5,0,0
        assert_series(&go("demarker", &d1(), &[("length", 2.)])[0], &[N, N, 1., 0.25 / 0.35, 0., 0., 0.5 / 0.75, 1.]);
    }
    #[test] fn dosc_zero_on_constant_rsi() {
        let o = go("dosc", &ramp(60), &[]);
        assert!(o[0][27].is_nan() && o[0][28..].iter().all(|v| near(*v, 0.)));
    }
    #[test] fn double_stoch_close_at_high() {
        // close = high every bar on a rise: close − ll = hh − ll → Blau 100
        let b = from_ohlc(&(0..60).map(|i| { let c = 100. + i as f64; (c - 0.5, c, c - 10., c) }).collect::<Vec<_>>());
        let o = go("double_stoch", &b, &[]);
        assert!(near(o[0][59], 100.) && near(o[1][59], 100.));
        // Bressert: k1 pinned at 100 → e1 flat → second stochastic has zero range → 0 (Sierra Chart rule)
        let o = go("double_stoch", &b, &[("variant", 1.)]);
        assert!(near(o[0][59], 0.) && near(o[1][59], 0.));
        // thinkorswim: zero high-low range → divisor 0 → DSS 0 (defaults 10/5/9: first value bar 21)
        let o = go("double_stoch", &closes(&[5.; 40]), &[]);
        assert!(o[0][20].is_nan() && o[0][21..].iter().all(|v| *v == 0.) && o[1][28].is_nan() && near(o[1][29], 0.));
    }
    #[test] fn gap_momentum_equal_gaps_is_100() {
        let b = closes(&[10., 11., 10., 11., 10.]); // gaps +1, −1, +1, −1
        let o = go("gap_momentum", &b, &[("length", 2.), ("signal", 2.)]);
        assert_series(&o[0], &[N, N, 100., 100., 100.]);
        assert_series(&o[1], &[N, N, N, 100., 100.]);
    }
    #[test] fn hhlls_new_high_every_bar() {
        // high at the n-bar high every bar → hh = 1 → hhs 100; lows never fall → lls 0
        let o = go("hhlls", &ramp(50), &[("length", 5.), ("smooth", 3.)]);
        assert!(o[0][5].is_nan() && near(o[0][6], 100.) && near(o[1][6], 0.) && near(o[0][49], 100.));
    }
    #[test] fn ift_rsi_saturates() {
        let o = go("ift_rsi", &ramp(30), &[]);
        assert!(o[0][12].is_nan() && near(o[0][13], (10f64.exp() - 1.) / (10f64.exp() + 1.)));
        assert!(go("ift_rsi", &closes(&[5.; 30]), &[])[0][13..].iter().all(|v| v.abs() > 0.99)); // flat → rsi 100 by our rule
    }
    #[test] fn imi_hand_values() {
        // d1 close − open: .5, 1, .5, −1, −.8, .6, 1.1, .7; n = 2
        assert_series(&go("imi", &d1(), &[("length", 2.)])[0], &[N, 100., 100., 100. * 0.5 / 1.5, 0., 100. * 0.6 / 1.4, 100., 100.]);
        assert!(near(go("imi", &closes(&[5.; 20]), &[])[0][19], 50.));
    }
    #[test] fn impulse_macd_zero_inside_band() {
        let o = go("impulse_macd", &closes(&[50.; 120]), &[]);
        assert!(o[0][65].is_nan() && near(o[0][66], 0.) && near(o[2][119], 0.));
    }
    #[test] fn inertia_is_linreg_of_rvi() {
        let b = walk(200, 3, false);
        let r = go("rvi", &b, &[("length", 14.), ("ema", 14.)]);
        assert_series(&go("inertia", &b, &[])[0], &crate::lab::ta::linreg(&r[0], 20, 0));
    }
    #[test] fn kdj_constant_rsv() {
        let o = go("kdj", &flat(20, 9., 10., 5., 9.), &[]); // rsv = 80
        assert!(o[0][9].is_nan() && near(o[0][10], 80.) && near(o[1][12], 80.) && near(o[2][19], 80.));
    }
    #[test] fn laguerre_rsi_bounded_and_pinned_by_uptrend() {
        assert!(go("laguerre_rsi", &walk(300, 2, false), &[])[0].iter().all(|v| (0. ..=1.).contains(v)));
        assert!(near(go("laguerre_rsi", &ramp(200), &[])[0][199], 1.));
        // bar 0 from the zero state, γ = .5, close 8: l = (4, −2, 1, −.5) → cu = 6 + 1.5, cd = 3
        assert!(near(go("laguerre_rsi", &closes(&[8.]), &[])[0][0], 7.5 / 10.5));
    }
    #[test] fn macd_bb_collapses_on_constant_macd() {
        let o = go("macd_bb", &closes(&[7.; 60]), &[]);
        assert!(o[1][33].is_nan() && near(o[1][34], 0.) && near(o[2][59], 0.) && near(o[0][59], 0.));
    }
    #[test] fn macd_leader_on_a_ramp() {
        // on a ramp the residual EMAs settle to constants, so leader − macd settles to a constant
        let o = go("macd_leader", &ramp(400), &[]);
        assert!(((o[0][399] - o[1][399]) - (o[0][398] - o[1][398])).abs() < 1e-6);
    }
    #[test] fn macd_v_is_macd_over_atr() {
        let b = walk(120, 5, false);
        let (m, a, v) = (go("macd", &b, &[]), go("atr", &b, &[("length", 26.)]), go("macd_v", &b, &[]));
        for i in 25..120 { assert!(near(v[0][i], 100. * m[0][i] / a[0][i])); }
    }
    #[test] fn madh_hand_and_flat() {
        assert!(go("madh", &closes(&[5.; 30]), &[])[0][20..].iter().all(|v| near(*v, 0.)));
        // short 1, dominant 2 → long 2: hann(1) = close; hann(2) weights (1 − cos 120°, 1 − cos 240°) = (1.5, 1.5) → mean of 2
        assert_series(&go("madh", &closes(&[10., 12., 9.]), &[("short", 1.), ("dominant", 2.)])[0], &[N, 100. / 11., 100. * -1.5 / 10.5]);
    }
    #[test] fn myrsi_net_hand() {
        let o = go("myrsi_net", &ramp(40), &[("rsi_length", 3.), ("net_length", 3.)]);
        assert!(near(o[0][3], 1.) && near(o[1][5], 0.)); // all-up → myrsi 1; flat X → net 0
        let o = go("myrsi_net", &closes(&[0., 1., 1., 0., 1., 2., 3., 4.]), &[("rsi_length", 3.), ("net_length", 3.)]);
        // myrsi: bar3 (+1, 0, −1) → 0; bar4 (0, −1, +1) → 0; bar5 (−1, +1, +1) → 1/3; bar6 → 1
        assert_series(&o[0][3..7], &[0., 0., 1. / 3., 1.]);
        // net bar5: X = (1/3, 0, 0) newest first → pairs (2,1) +1, (3,1) +1, (3,2) 0 → 2/3; bar6 rising → 1
        assert!(near(o[1][5], 2. / 3.) && near(o[1][6], 1.));
    }
    #[test] fn pgo_zero_when_close_on_sma() {
        assert!(go("pgo", &closes(&[4.; 20]), &[])[0].iter().all(|v| v.is_nan())); // zero true range → NaN
        assert!(go("pgo", &flat(20, 4., 5., 3., 4.), &[])[0][13..].iter().all(|v| near(*v, 0.)));
    }
    #[test] fn polychrome_hand() {
        assert!(go("polychrome_mom", &closes(&[2.; 15]), &[])[0][10..].iter().all(|v| near(*v, 0.)));
        assert_series(&go("polychrome_mom", &closes(&[1., 2., 4.]), &[("length", 2.)])[0], &[N, N, (2. + 3. / 2f64.sqrt()) / (1. + 2f64.sqrt())]);
    }
    #[test] fn preferred_stoch_constant_k() {
        let o = go("preferred_stoch", &flat(30, 9., 10., 5., 7.), &[]); // fast_k = 40
        assert!(near(o[0][9], 40.) && near(o[1][11], 40.) && o[2][12].is_nan() && near(o[2][13], 40.));
    }
    #[test] fn premier_rsi_bounded() {
        assert!(go("premier_rsi", &walk(300, 8, false), &[])[0].iter().filter(|v| !v.is_nan()).all(|v| v.abs() < 1.));
    }
    #[test] fn premier_stoch_midrange_is_zero() {
        let o = &go("premier_stoch", &flat(30, 7., 10., 4., 7.), &[])[0];
        assert!(o[14].is_nan() && o[15..].iter().all(|v| near(*v, 0.)));
        assert!(go("premier_stoch", &walk(300, 8, false), &[])[0].iter().filter(|v| !v.is_nan()).all(|v| v.abs() < 1.));
    }
}
