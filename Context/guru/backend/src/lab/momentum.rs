//! Momentum / oscillator family (TradingView built-ins).
use super::bars::Bars;
use super::catalogue::{int, len, n, p, IndicatorDef, Needs, Output, ParamSpec, Params, Placement};
use super::ta::{self, NA};

const CAT: &str = "Momentum";
const NO: Needs = Needs::NONE;

fn zip(a: &[f64], b: &[f64], f: impl Fn(f64, f64) -> f64) -> Vec<f64> { a.iter().zip(b).map(|(&x, &y)| f(x, y)).collect() }
fn div(a: f64, b: f64) -> f64 { if b == 0. { NA } else { a / b } }
/// 100*(x - x[k])/x[k]
fn roc_s(x: &[f64], k: usize) -> Vec<f64> { (0..x.len()).map(|i| if i >= k { div(100. * (x[i] - x[i - k]), x[i - k]) } else { NA }).collect() }
fn rsi_s(x: &[f64], len: usize) -> Vec<f64> {
    let d = ta::change(x, 1);
    let up = ta::rma(&d.iter().map(|v| if v.is_nan() { NA } else { v.max(0.) }).collect::<Vec<_>>(), len);
    let dn = ta::rma(&d.iter().map(|v| if v.is_nan() { NA } else { (-v).max(0.) }).collect::<Vec<_>>(), len);
    zip(&up, &dn, |u, d| if u.is_nan() || d.is_nan() { NA } else if d == 0. { 100. } else if u == 0. { 0. } else { 100. - 100. / (1. + u / d) })
}
/// raw stochastic of `x` against the `hi`/`lo` window extremes; NaN when the range is 0.
fn stoch_s(x: &[f64], hi: &[f64], lo: &[f64]) -> Vec<f64> {
    (0..x.len()).map(|i| div(100. * (x[i] - lo[i]), hi[i] - lo[i])).collect()
}
fn macd_like(x: &[f64], f: usize, s: usize, sig: usize, pct: bool) -> Vec<Vec<f64>> {
    let (ef, es) = (ta::ema(x, f), ta::ema(x, s));
    let m = zip(&ef, &es, |a, b| if pct { div(100. * (a - b), b) } else { a - b });
    let signal = ta::ema(&m, sig);
    let hist = zip(&m, &signal, |a, b| a - b);
    vec![m, signal, hist]
}

fn rsi(b: &Bars, prm: &Params) -> Output {
    let l = n(prm, "length");
    if p(prm, "variant") == 0. { return Ok(vec![rsi_s(&b.close, l)]); }
    let d = ta::change(&b.close, 1);
    let up = ta::sum(&d.iter().map(|v| if v.is_nan() { NA } else { v.max(0.) }).collect::<Vec<_>>(), l);
    let dn = ta::sum(&d.iter().map(|v| if v.is_nan() { NA } else { (-v).max(0.) }).collect::<Vec<_>>(), l);
    Ok(vec![zip(&up, &dn, |u, d| if u.is_nan() || d.is_nan() { NA } else if d == 0. { 100. } else { 100. * u / (u + d) })])
}
fn macd(b: &Bars, prm: &Params) -> Output {
    let (f, s, sig, ma, st) = (n(prm, "fast"), n(prm, "slow"), n(prm, "signal"), n(prm, "ma_type"), n(prm, "signal_type"));
    if ma == 0 && st == 0 { return Ok(macd_like(&b.close, f, s, sig, false)); }
    // DEMA = 2·e1 − e2, TEMA = 3·(e1 − e2) + e3 (as ma.rs dema/tema)
    let avg = |k: usize| -> Vec<f64> {
        let e1 = ta::ema(&b.close, k);
        if ma == 0 { return e1; }
        let e2 = ta::ema(&e1, k);
        if ma == 1 { return zip(&e1, &e2, |a, c| 2. * a - c); }
        let e3 = ta::ema(&e2, k);
        (0..e1.len()).map(|i| 3. * (e1[i] - e2[i]) + e3[i]).collect()
    };
    let m = zip(&avg(f), &avg(s), |a, c| a - c);
    let signal = if st == 1 { ta::sma(&m, sig) } else { ta::ema(&m, sig) };
    let hist = zip(&m, &signal, |a, c| a - c);
    Ok(vec![m, signal, hist])
}
fn ppo(b: &Bars, prm: &Params) -> Output { Ok(macd_like(&b.close, n(prm, "fast"), n(prm, "slow"), n(prm, "signal"), true)) }
fn stoch(b: &Bars, prm: &Params) -> Output {
    let k = n(prm, "k");
    let (hh, ll) = (ta::highest(&b.high, k), ta::lowest(&b.low, k));
    let kk = if p(prm, "variant") == 1. {
        let sk = n(prm, "smooth_k");
        zip(&ta::sum(&zip(&b.close, &ll, |c, l| c - l), sk), &ta::sum(&zip(&hh, &ll, |h, l| h - l), sk), |a, d| div(100. * a, d))
    } else { ta::sma(&stoch_s(&b.close, &hh, &ll), n(prm, "smooth_k")) };
    let d = ta::sma(&kk, n(prm, "d"));
    Ok(vec![kk, d])
}
fn stochrsi(b: &Bars, prm: &Params) -> Output {
    let (r, sl) = (rsi_s(&b.close, n(prm, "rsi_length")), n(prm, "stoch_length"));
    let k = ta::sma(&stoch_s(&r, &ta::highest(&r, sl), &ta::lowest(&r, sl)), n(prm, "k"));
    let d = ta::sma(&k, n(prm, "d"));
    if p(prm, "variant") == 1. { return Ok(vec![k.iter().map(|v| v / 100.).collect(), d.iter().map(|v| v / 100.).collect()]); }
    Ok(vec![k, d])
}
fn cci(b: &Bars, prm: &Params) -> Output {
    let (s, l) = (b.hlc3(), n(prm, "length"));
    let (ma, dv) = (ta::sma(&s, l), ta::dev(&s, l));
    Ok(vec![(0..b.len()).map(|i| div(s[i] - ma[i], 0.015 * dv[i])).collect()])
}
fn mom(b: &Bars, prm: &Params) -> Output {
    let (k, c) = (n(prm, "length"), &b.close);
    if p(prm, "variant") == 1. { return Ok(vec![(0..c.len()).map(|i| if i >= k { div(100. * c[i], c[i - k]) } else { NA }).collect()]); }
    Ok(vec![ta::change(c, k)])
}
fn roc(b: &Bars, prm: &Params) -> Output {
    let (k, c) = (n(prm, "length"), &b.close);
    if p(prm, "variant") == 1. { return Ok(vec![(0..c.len()).map(|i| if i >= k { div(100. * (c[i] - c[i - k]), c[i]) } else { NA }).collect()]); }
    Ok(vec![roc_s(c, k)])
}
fn willr(b: &Bars, prm: &Params) -> Output {
    let l = n(prm, "length");
    let (hh, ll) = (ta::highest(&b.high, l), ta::lowest(&b.low, l));
    Ok(vec![(0..b.len()).map(|i| div(100. * (b.close[i] - hh[i]), hh[i] - ll[i])).collect()])
}
fn mfi(b: &Bars, prm: &Params) -> Output {
    let v = b.volume.as_ref().ok_or("needs volume")?;
    let (s, l) = (b.hlc3(), n(prm, "length"));
    let d = ta::change(&s, 1);
    // NaN change (bar 0) fails both comparisons, so bar 0 counts in both sums (TradingView quirk).
    let up: Vec<f64> = (0..b.len()).map(|i| v[i] * if d[i] <= 0. { 0. } else { s[i] }).collect();
    let dn: Vec<f64> = (0..b.len()).map(|i| v[i] * if d[i] >= 0. { 0. } else { s[i] }).collect();
    let (up, dn) = (ta::sum(&up, l), ta::sum(&dn, l));
    Ok(vec![zip(&up, &dn, |u, d| if u.is_nan() || d.is_nan() { NA } else if d == 0. { 100. } else { 100. - 100. / (1. + u / d) })])
}
fn tsi(b: &Bars, prm: &Params) -> Output {
    let (lg, sh) = (n(prm, "long"), n(prm, "short"));
    let d = ta::change(&b.close, 1);
    let ad: Vec<f64> = d.iter().map(|v| v.abs()).collect();
    let (num, den) = (ta::ema(&ta::ema(&d, lg), sh), ta::ema(&ta::ema(&ad, lg), sh));
    let t = zip(&num, &den, |a, b| div(100. * a, b));
    let s = ta::ema(&t, n(prm, "signal"));
    Ok(vec![t, s])
}
fn cmo(b: &Bars, prm: &Params) -> Output {
    let d = ta::change(&b.close, 1);
    let l = n(prm, "length");
    let s1 = ta::sum(&d.iter().map(|&v| if v >= 0. { v } else if v.is_nan() { NA } else { 0. }).collect::<Vec<_>>(), l);
    let s2 = ta::sum(&d.iter().map(|&v| if v < 0. { -v } else if v.is_nan() { NA } else { 0. }).collect::<Vec<_>>(), l);
    Ok(vec![zip(&s1, &s2, |a, b| div(100. * (a - b), a + b))])
}
fn uo(b: &Bars, prm: &Params) -> Output {
    let c = &b.close;
    let lo: Vec<f64> = (0..b.len()).map(|i| if i == 0 { NA } else { b.low[i].min(c[i - 1]) }).collect();
    let bp = zip(c, &lo, |c, l| c - l);
    let tr: Vec<f64> = (0..b.len()).map(|i| if i == 0 { NA } else { b.high[i].max(c[i - 1]) - lo[i] }).collect();
    let avg = |k: &str| { let l = n(prm, k); zip(&ta::sum(&bp, l), &ta::sum(&tr, l), div) };
    let (a, m, s) = (avg("fast"), avg("mid"), avg("slow"));
    Ok(vec![(0..b.len()).map(|i| 100. * (4. * a[i] + 2. * m[i] + s[i]) / 7.).collect()])
}
fn ao(b: &Bars, prm: &Params) -> Output {
    let h = b.hl2();
    Ok(vec![zip(&ta::sma(&h, n(prm, "fast")), &ta::sma(&h, n(prm, "slow")), |a, b| a - b)])
}
fn trix(b: &Bars, prm: &Params) -> Output {
    let l = n(prm, "length");
    if p(prm, "variant") >= 1. {
        let (e, sc) = (ta::ema(&ta::ema(&ta::ema(&b.close, l), l), l), if p(prm, "variant") == 1. { 100. } else { 1. });
        return Ok(vec![(0..e.len()).map(|i| if i == 0 { NA } else { div(sc * (e[i] - e[i - 1]), e[i - 1]) }).collect()]);
    }
    let ln: Vec<f64> = b.close.iter().map(|&c| if c > 0. { c.ln() } else { NA }).collect();
    let e = ta::ema(&ta::ema(&ta::ema(&ln, l), l), l);
    Ok(vec![ta::change(&e, 1).iter().map(|d| 10000. * d).collect()])
}
fn kst(b: &Bars, prm: &Params) -> Output {
    let mut k = vec![0.; b.len()];
    for j in 1..=4 {
        let s = ta::sma(&roc_s(&b.close, n(prm, &format!("roc{j}"))), n(prm, &format!("sma{j}")));
        for i in 0..b.len() { k[i] += j as f64 * s[i]; }
    }
    let sig = ta::sma(&k, n(prm, "signal"));
    Ok(vec![k, sig])
}
fn dpo(b: &Bars, prm: &Params) -> Output {
    let l = n(prm, "length");
    if p(prm, "variant") == 1. {
        let m = ta::sma(&b.close, l / 2 + 1);
        return Ok(vec![(0..b.len()).map(|i| b.close[i] - m[i]).collect()]);
    }
    let (m, sh) = (ta::sma(&b.close, l), l / 2 + 1);
    Ok(vec![(0..b.len()).map(|i| if i >= sh { b.close[i] - m[i - sh] } else { NA }).collect()])
}
fn coppock(b: &Bars, prm: &Params) -> Output {
    let s = zip(&roc_s(&b.close, n(prm, "long")), &roc_s(&b.close, n(prm, "short")), |a, b| a + b);
    Ok(vec![ta::wma(&s, n(prm, "wma"))])
}
fn fisher(b: &Bars, prm: &Params) -> Output {
    let (h, l) = (b.hl2(), n(prm, "length"));
    let (hh, ll) = (ta::highest(&h, l), ta::lowest(&h, l));
    let (mut v, mut f) = (0., 0.);
    let mut out = vec![NA; b.len()];
    for i in 0..b.len() {
        let r = hh[i] - ll[i];
        if r.is_nan() || r == 0. { continue; }
        v = 0.66 * ((h[i] - ll[i]) / r - 0.5) + 0.67 * v;
        if v.abs() > 0.99 { v = 0.999 * v.signum(); }
        f = 0.5 * ((1. + v) / (1. - v)).ln() + 0.5 * f;
        out[i] = f;
    }
    let trig = (0..b.len()).map(|i| if i == 0 { NA } else { out[i - 1] }).collect();
    Ok(vec![out, trig])
}
/// Up/down streak length; bar 0 is -1 (TradingView artefact).
fn streak(c: &[f64]) -> Vec<f64> {
    let mut out = vec![NA; c.len()];
    let mut s = -1.;
    for i in 0..c.len() {
        if i > 0 {
            if c[i].is_nan() || c[i - 1].is_nan() { continue; }
            s = if c[i] == c[i - 1] { 0. } else if c[i] > c[i - 1] { if s <= 0. { 1. } else { s + 1. } } else if s >= 0. { -1. } else { s - 1. };
        }
        out[i] = s;
    }
    out
}
/// 100 * (count of the n previous values <= x[i]) / n; NaN members stay in the denominator.
fn percentrank(x: &[f64], k: usize) -> Vec<f64> {
    (0..x.len()).map(|i| if i < k || x[i].is_nan() { NA } else {
        100. * x[i - k..i].iter().filter(|&&v| v <= x[i]).count() as f64 / k as f64
    }).collect()
}
fn crsi(b: &Bars, prm: &Params) -> Output {
    let (r, s, p) = (rsi_s(&b.close, n(prm, "rsi")), rsi_s(&streak(&b.close), n(prm, "streak")), percentrank(&roc_s(&b.close, 1), n(prm, "rank")));
    Ok(vec![(0..b.len()).map(|i| (r[i] + s[i] + p[i]) / 3.).collect()])
}

const fn def(id: &'static str, out: &'static [&'static str], name: &'static str, params: &'static [ParamSpec], needs: Needs, definition: &'static str, first: &'static [usize], f: fn(&Bars, &Params) -> Output) -> IndicatorDef {
    IndicatorDef { id, name, category: CAT, params, outputs: out, placement: Placement::Pane, needs, definition, first, sparse: false, compute: Some(f), note: "" }
}
const LEN14: &[ParamSpec] = &[len("length", "Length", 14.)];
const MACD: &[ParamSpec] = &[len("fast", "Fast length", 12.), len("slow", "Slow length", 26.), len("signal", "Signal length", 9.)];

pub const DEFS: &[IndicatorDef] = &[
    def("rsi", &["rsi"], "Relative Strength Index", &[len("length", "Length", 14.), int("variant", "Variant (0 Wilder/TradingView, 1 Cutler/TradeStation RSI JP)", 0., 0., 1.)], NO,
        "up=rma(max(change,0)), dn=rma(max(-change,0)); dn==0 -> 100, up==0 -> 0, else 100-100/(1+up/dn); variant 1 (Cutler's RSI, TradeStation RSI JP: no Wilder smoothing): 100·sum(up, n)/(sum(up, n) + sum(dn, n)), dn sum 0 → 100", &[14], rsi),
    def("macd", &["macd", "signal", "hist"], "MACD", &[len("fast", "Fast length", 12.), len("slow", "Slow length", 26.), len("signal", "Signal length", 9.),
        int("signal_type", "Signal MA (0 EMA TradingView, 1 SMA MetaTrader iMACD/iOsMA)", 0., 0., 1.), int("ma_type", "Fast/slow MA (0 EMA, 1 DEMA, 2 TEMA — MetaStock/Mulloy)", 0., 0., 2.)], NO,
        "macd=ema(close,fast)-ema(close,slow); signal=ema(macd,signal); hist=macd-signal; signal_type 1 (MetaTrader 5 MACD.mq5/OsMA.mq5): signal = sma(macd, signal), hist = OsMA = macd − signal; ma_type 1/2 (MetaStock MACD on DEMA/TEMA, Mulloy 1994): fast/slow = dema/tema(close) = 2·e1 − e2 / 3·(e1 − e2) + e3 (EMAs SMA-seeded as ours; MetaTrader seeds its EMA with the first price, so its early values differ)", &[25, 33, 33], macd),
    def("stoch", &["k", "d"], "Stochastic", &[len("k", "%K length", 14.), len("smooth_k", "%K smoothing", 1.), len("d", "%D smoothing", 3.), int("variant", "Variant (0 TradingView, 1 AmiBroker ratio of sums)", 0., 0., 1.)], NO,
        "raw=100*(close-lowest(low,k))/(highest(high,k)-lowest(low,k)) (NaN if range 0); k=sma(raw,smooth_k); d=sma(k,d); variant 1 (AmiBroker StochK/StochD, standard ratio-of-sums form; unverified): k = 100·sum(close − lowest(low, k), smooth_k)/sum(highest(high, k) − lowest(low, k), smooth_k); d = sma(k, d)", &[13, 15], stoch),
    def("stochrsi", &["k", "d"], "Stochastic RSI", &[len("k", "%K smoothing", 3.), len("d", "%D smoothing", 3.), len("rsi_length", "RSI length", 14.), len("stoch_length", "Stochastic length", 14.),
        int("variant", "Scale (0 TradingView 0–100, 1 NinjaTrader 8 0–1)", 0., 0., 1.)], NO,
        "r=rsi(close,rsi_length); k=sma(100*(r-lowest(r,s))/(highest(r,s)-lowest(r,s)),k) with highest/lowest window restarting after NaN; d=sma(k,d); variant 1 (NinjaTrader 8 StochRSI): k and d divided by 100 (0–1 scale); NinjaTrader's single unsmoothed line = k with k = 1 (NinjaTrader prints 0 where the RSI range is 0; here NaN)", &[17, 19], stochrsi),
    def("cci", &["cci"], "Commodity Channel Index", &[len("length", "Length", 20.)], NO, "(hlc3-sma(hlc3,n))/(0.015*dev(hlc3,n)); dev 0 -> NaN", &[19], cci),
    def("mom", &["mom"], "Momentum", &[len("length", "Length", 10.), int("variant", "Variant (0 TradingView difference, 1 MetaTrader/thinkorswim ratio)", 0., 0., 1.)], NO,
        "close - close[n]; variant 1 (MetaTrader 5 Momentum.mq5, default n 14; thinkorswim MomentumPercent): 100·close/close[n] (base 100; close[n] 0 → NaN)", &[10], mom),
    def("roc", &["roc"], "Rate of Change", &[len("length", "Length", 9.), int("variant", "Variant (0 TradingView/AmiBroker, 1 MetaTrader 5 ROC example)", 0., 0., 1.)], NO,
        "100*(close-close[n])/close[n]; variant 1 (MetaTrader 5 ROC.mq5 example): 100·(close − close[n])/close (divides by the current close; 0 → NaN)", &[9], roc),
    def("willr", &["willr"], "Williams %R", LEN14, NO, "100*(close-highest(high,n))/(highest(high,n)-lowest(low,n)); NaN if range 0", &[13], willr),
    def("mfi", &["mfi"], "Money Flow Index", LEN14, Needs { volume: true, ..Needs::NONE },
        "src=hlc3; up=sum(v*(change<=0?0:src),n), dn=sum(v*(change>=0?0:src),n) (bar 0 counts in both); dn==0 -> 100 else 100-100/(1+up/dn)", &[13], mfi),
    def("tsi", &["tsi", "signal"], "True Strength Index", &[len("long", "Long length", 25.), len("short", "Short length", 13.), len("signal", "Signal length", 13.)], NO,
        "100*ema(ema(change,long),short)/ema(ema(|change|,long),short); signal=ema(tsi,signal)", &[37, 49], tsi),
    def("cmo", &["cmo"], "Chande Momentum Oscillator", &[len("length", "Length", 9.)], NO, "S1=sum of gains, S2=sum of losses over n; 100*(S1-S2)/(S1+S2); 0/0 -> NaN", &[9], cmo),
    def("uo", &["uo"], "Ultimate Oscillator", &[len("fast", "Fast length", 7.), len("mid", "Middle length", 14.), len("slow", "Slow length", 28.)], NO,
        "bp=close-min(low,close[1]); tr=max(high,close[1])-min(low,close[1]); A_k=sum(bp,k)/sum(tr,k); 100*(4*A_fast+2*A_mid+A_slow)/7", &[28], uo),
    def("ao", &["ao"], "Awesome Oscillator", &[len("fast", "Fast length", 5.), len("slow", "Slow length", 34.)], NO, "sma(hl2,fast)-sma(hl2,slow)", &[33], ao),
    def("ppo", &["ppo", "signal", "hist"], "Percentage Price Oscillator", MACD, NO, "100*(ema(close,fast)-ema(close,slow))/ema(close,slow); signal=ema(ppo,signal); hist=ppo-signal", &[25, 33, 33], ppo),
    def("trix", &["trix"], "TRIX", &[len("length", "Length", 18.), int("variant", "Variant (0 TradingView log ×10000, 1 AmiBroker percent ROC, 2 MetaTrader 5 iTriX fraction)", 0., 0., 2.)], NO, "10000*change(ema(ema(ema(ln(close),n),n),n)); variant 1 (AmiBroker TRIX, standard form; unverified): 100·(e − e[1])/e[1], e = ema(ema(ema(close, n), n), n) (no log); variant 2 (MetaTrader 5 TRIX.mq5, default n 14): (e − e[1])/e[1] as a fraction (variant 1 ÷ 100; EMAs SMA-seeded as ours)", &[52], trix),
    def("kst", &["kst", "signal"], "Know Sure Thing", &[len("roc1", "ROC 1", 10.), len("roc2", "ROC 2", 15.), len("roc3", "ROC 3", 20.), len("roc4", "ROC 4", 30.),
        len("sma1", "SMA 1", 10.), len("sma2", "SMA 2", 10.), len("sma3", "SMA 3", 10.), len("sma4", "SMA 4", 15.), len("signal", "Signal length", 9.)], NO,
        "sum_j j*sma(roc(close,roc_j),sma_j), j=1..4; signal=sma(kst,signal)", &[44, 52], kst),
    def("dpo", &["dpo"], "Detrended Price Oscillator", &[len("length", "Length", 21.), int("variant", "Variant (0 TradingView, 1 MetaTrader 5 DPO)", 0., 0., 1.)], NO,
        "close - sma(close,n)[floor(n/2)+1] (non-centered); variant 1 (MetaTrader 5 DPO.mq5, default n 12): close − sma(close, floor(n/2) + 1) on the current bar (no shift; MetaTrader shows 0 before the first value). Sierra Chart Detrended Oscillator (Length L) = variant 0 with n = floor(L/2); Sierra Chart Detrended Oscillator - DiNapoli = close − sma(close, 7) = variant 1 with n = 12", &[31], dpo),
    def("coppock", &["coppock"], "Coppock Curve", &[len("wma", "WMA length", 10.), len("long", "Long ROC", 14.), len("short", "Short ROC", 11.)], NO, "wma(roc(close,long)+roc(close,short),wma)", &[23], coppock),
    def("fisher", &["fisher", "trigger"], "Fisher Transform", &[len("length", "Length", 9.)], NO,
        "v=0.66*((hl2-ll)/(hh-ll)-0.5)+0.67*nz(v[1]) clamped to +-0.999 when |v|>0.99; fisher=0.5*ln((1+v)/(1-v))+0.5*nz(fisher[1]); trigger=fisher[1]", &[8, 9], fisher),
    def("crsi", &["crsi"], "Connors RSI", &[len("rsi", "RSI length", 3.), len("streak", "Streak RSI length", 2.), len("rank", "ROC rank length", 100.)], NO,
        "avg(rsi(close,rsi), rsi(streak,streak), percentrank(roc(close,1),rank)); streak bar 0 = -1", &[100], crsi),
];

#[cfg(test)]
mod tests {
    use super::streak;
    use crate::lab::{catalogue::{run, get, Params}, testdata::{assert_series, d1, from_ohlc, go, walk}};
    const N: f64 = f64::NAN;
    fn closes(c: &[f64]) -> crate::lab::bars::Bars { from_ohlc(&c.iter().map(|&x| (x, x, x, x)).collect::<Vec<_>>()) }
    fn with(k: &str, v: f64) -> Params { let mut p = Params::new(); p.insert(k.into(), v); p }
    fn many(kv: &[(&str, f64)]) -> Params { kv.iter().map(|(k, v)| (k.to_string(), *v)).collect() }
    fn hl_rows() -> crate::lab::bars::Bars {
        from_ohlc(&[(1., 2., 0., 1.), (1., 3., 1., 2.), (1., 4., 2., 3.), (1., 5., 3., 5.), (1., 6., 4., 4.)])
    }
    #[test] fn rsi_saturates_at_100_on_monotonic_rise() {
        let b = closes(&(1..=20).map(|x| x as f64).collect::<Vec<_>>());
        let r = &run(&get("rsi").unwrap(), &b, &with("length", 14.)).unwrap()[0];
        assert!(r[13].is_nan()); assert_eq!(r[14], 100.);
    }
    #[test] fn macd_short_lengths() {
        let o = run(&get("macd").unwrap(), &closes(&[1., 2., 4., 7., 11.]), &many(&[("fast", 2.), ("slow", 3.), ("signal", 2.)])).unwrap();
        assert_series(&o[0], &[N, N, 5. / 6., 19. / 18., 38. / 27.]);
        assert_series(&o[1], &[N, N, N, 17. / 18., 203. / 162.]);
        assert_series(&o[2], &[N, N, N, 1. / 9., 25. / 162.]);
    }
    #[test] fn stoch_k_and_d() {
        let o = run(&get("stoch").unwrap(), &hl_rows(), &many(&[("k", 3.), ("smooth_k", 1.), ("d", 2.)])).unwrap();
        assert_series(&o[0], &[N, N, 75., 100., 50.]);
        assert_series(&o[1], &[N, N, N, 87.5, 75.]);
    }
    #[test] fn cci_hand_value() {
        let r = &run(&get("cci").unwrap(), &closes(&[1., 2., 6.]), &with("length", 3.)).unwrap()[0];
        assert_series(r, &[N, N, 100.]);
    }
    #[test] fn willr_hand_value() {
        let r = &run(&get("willr").unwrap(), &hl_rows(), &with("length", 3.)).unwrap()[0];
        assert_series(r, &[N, N, -25., 0., -50.]);
    }
    #[test] fn cmo_hand_value() {
        let r = &run(&get("cmo").unwrap(), &closes(&[1., 3., 2., 5.]), &with("length", 3.)).unwrap()[0];
        assert_series(r, &[N, N, N, 100. * 4. / 6.]);
    }
    #[test] fn connors_streak() {
        assert_series(&streak(&[1., 1., 2., 3., 2., 2., 1., 0.5]), &[-1., 0., 1., 2., -1., 0., -1., -2.]);
    }
    /// Values captured at the commit before the `variant` parameter existed: the default meaning must never change.
    #[test] fn defaults_are_pinned_to_pre_variant_values() {
        let b = walk(1000, 5, true);
        let at = |id: &str, kv: &[(&str, f64)], o: usize| [20usize, 57, 300, 999].map(|i| go(id, &b, kv)[o][i]);
        let pin = |got: [f64; 4], want: [f64; 4]| for (g, w) in got.iter().zip(want) { assert!((g.is_nan() && w.is_nan()) || (g - w).abs() < 1e-12, "{g} vs {w}"); };
        pin(at("stoch", &[], 0), [16.593143227045697, 80.57663186472233, 43.25793326011815, 19.842984755584773]);
        pin(at("stoch", &[], 1), [10.007393087031415, 68.76198329929049, 47.1421167722645, 34.4545533649469]);
        pin(at("stoch", &[("smooth_k", 3.)], 0), [10.007393087031415, 68.76198329929049, 47.1421167722645, 34.4545533649469]);
        pin(at("stoch", &[("smooth_k", 3.)], 1), [8.280758363728102, 65.68237906273644, 46.284972944724665, 38.89603509018648]);
        pin(at("trix", &[], 0), [f64::NAN, -4.8194953290003895, -41.15304281852694, 2.857914402154549]);
    }
    /// Bit-exact FNV-1a of every default output, captured before the cross-platform `variant` parameters were added
    /// (2026-10-19): a new platform option must never move today's default values.
    #[test] fn platform_variants_leave_defaults_bit_identical() {
        let b = walk(1000, 5, true);
        let pins: [(&str, u64); 28] = [("dmi", 0xa9e4c3108e3263a1), ("atr", 0x15e80447a62d0c3b), ("mom", 0xdab57baf2ad3ab0d), ("roc", 0x6f8cc69701a8feed), ("macd", 0x84e450bf7d12ab0f),
            ("efi", 0x87acb25ea616bc73), ("trix", 0xda67d812014e35de), ("dpo", 0xf2f60dc21a7654ec), ("pivots_std", 0xdb70b5b69fd2ab64), ("kc", 0x16026fc1c3cc47c), ("eom", 0x5da87fdc947e9943),
            ("hv", 0x66cd61320dd1fd32), ("stochrsi", 0x5d49c4930f918304), ("price_osc", 0x9e403ebd7297f2ba), ("pvo", 0x912aa7ef9828e3c8), ("rsi", 0xcdc8e9dc68c0fa4c),
            ("majority_rule", 0xf02fee5ca8fd1d03), ("klinger", 0x1e866435cd0deff1), ("tsf", 0x7030c4a53d70a504), ("bb", 0x795e66a918e6d4ce), ("supertrend", 0xfd65c569c9ab05a6),
            ("hma", 0x8e8e90ff68e1fc5c), ("envelope", 0x396231c0b7afb39d), ("vwap_period", 0x4025915c015ad8), ("cdl_kicking_bull", 0x51e78e744621f425),
            ("cdl_kicking_bear", 0x51e78e744621f425), ("median", 0x466b791e4fd77f2), ("ma_ribbon", 0xdd1ad472c78a4be1)];
        for (id, want) in pins {
            let mut h = 0xcbf2_9ce4_8422_2325u64;
            for s in go(id, &b, &[]) { for v in s { for x in v.to_bits().to_le_bytes() { h ^= u64::from(x); h = h.wrapping_mul(0x100_0000_01b3); } } }
            assert_eq!(h, want, "{id}: default output moved");
        }
    }
    #[test] fn stoch_variant_ab_is_ratio_of_sums() {
        let o = go("stoch", &d1(), &[("k", 3.), ("smooth_k", 2.), ("d", 2.), ("variant", 1.)]);
        assert_series(&o[0], &[N, N, N, 66.666666666667, 24.0, 28.846153846154, 71.153846153846, 91.666666666667]);
        assert_series(&o[1], &[N, N, N, N, 45.333333333333, 26.423076923077, 50.0, 81.410256410256]);
    }
    #[test] fn trix_variant_ab_is_percent_roc() {
        assert_series(&go("trix", &d1(), &[("length", 2.), ("variant", 1.)])[0], &[N, N, N, N, -3.045537340619, -1.317602264472, 2.716080720904, 4.83106113651]);
    }
    #[test] fn mom_and_roc_metatrader_variants() {
        let c = closes(&[1., 2., 4.]);
        assert_series(&go("mom", &c, &[("length", 1.), ("variant", 1.)])[0], &[N, 200., 200.]);
        assert_series(&go("roc", &c, &[("length", 1.), ("variant", 1.)])[0], &[N, 50., 50.]);
    }
    #[test] fn macd_metatrader_sma_signal() {
        let o = go("macd", &closes(&[1., 2., 4., 7., 11.]), &[("fast", 2.), ("slow", 3.), ("signal", 2.), ("signal_type", 1.)]);
        assert_series(&o[0], &[N, N, 5. / 6., 19. / 18., 38. / 27.]);
        assert_series(&o[1], &[N, N, N, 17. / 18., 133. / 108.]);
        assert_series(&o[2], &[N, N, N, 1. / 9., 19. / 108.]);
    }
    #[test] fn macd_on_dema_and_tema_is_the_difference_of_those_averages() {
        let b = walk(300, 5, true);
        for (t, id) in [(1., "dema"), (2., "tema")] {
            let (f, s) = (go(id, &b, &[("length", 12.)]).remove(0), go(id, &b, &[("length", 26.)]).remove(0));
            assert_series(&go("macd", &b, &[("ma_type", t)])[0], &f.iter().zip(&s).map(|(a, c)| a - c).collect::<Vec<_>>());
        }
    }
    #[test] fn trix_metatrader_fraction_and_dpo_unshifted() {
        assert_series(&go("trix", &d1(), &[("length", 2.), ("variant", 2.)])[0], &[N, N, N, N, -0.03045537340619, -0.01317602264472, 0.02716080720904, 0.0483106113651]);
        // sma(close, 2/2 + 1 = 2) = 1.5, 3, 5.5, 9 on the current bar
        assert_series(&go("dpo", &closes(&[1., 2., 4., 7., 11.]), &[("length", 2.), ("variant", 1.)])[0], &[N, 0.5, 1., 1.5, 2.]);
    }
    #[test] fn stochrsi_ninjatrader_scale_and_cutler_rsi() {
        let b = walk(300, 5, true);
        let (a, f) = (go("stochrsi", &b, &[]), go("stochrsi", &b, &[("variant", 1.)]));
        for k in 0..2 { assert_series(&f[k], &a[k].iter().map(|v| v / 100.).collect::<Vec<_>>()); }
        // changes +2, −1, +3: 100·5/6
        assert_series(&go("rsi", &closes(&[1., 3., 2., 5.]), &[("length", 3.), ("variant", 1.)])[0], &[N, N, N, 250. / 3.]);
    }
    #[test] fn variant_zero_is_unchanged() {
        let b = walk(300, 5, true);
        for id in ["stoch", "trix"] {
            let (a, z) = (go(id, &b, &[]), go(id, &b, &[("variant", 0.)]));
            for (x, y) in a.iter().zip(&z) { assert_series(x, y); }
        }
    }
}
