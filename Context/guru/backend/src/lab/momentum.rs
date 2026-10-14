//! Momentum / oscillator family (TradingView built-ins).
use super::bars::Bars;
use super::catalogue::{len, n, IndicatorDef, Needs, Output, ParamSpec, Params, Placement};
use super::ta::{self, NA};

const CAT: &str = "Momentum";
const NO: Needs = Needs { volume: false, intraday: false };

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

fn rsi(b: &Bars, prm: &Params) -> Output { Ok(vec![rsi_s(&b.close, n(prm, "length"))]) }
fn macd(b: &Bars, prm: &Params) -> Output { Ok(macd_like(&b.close, n(prm, "fast"), n(prm, "slow"), n(prm, "signal"), false)) }
fn ppo(b: &Bars, prm: &Params) -> Output { Ok(macd_like(&b.close, n(prm, "fast"), n(prm, "slow"), n(prm, "signal"), true)) }
fn stoch(b: &Bars, prm: &Params) -> Output {
    let k = n(prm, "k");
    let raw = stoch_s(&b.close, &ta::highest(&b.high, k), &ta::lowest(&b.low, k));
    let kk = ta::sma(&raw, n(prm, "smooth_k"));
    let d = ta::sma(&kk, n(prm, "d"));
    Ok(vec![kk, d])
}
fn stochrsi(b: &Bars, prm: &Params) -> Output {
    let (r, sl) = (rsi_s(&b.close, n(prm, "rsi_length")), n(prm, "stoch_length"));
    let k = ta::sma(&stoch_s(&r, &ta::highest(&r, sl), &ta::lowest(&r, sl)), n(prm, "k"));
    let d = ta::sma(&k, n(prm, "d"));
    Ok(vec![k, d])
}
fn cci(b: &Bars, prm: &Params) -> Output {
    let (s, l) = (b.hlc3(), n(prm, "length"));
    let (ma, dv) = (ta::sma(&s, l), ta::dev(&s, l));
    Ok(vec![(0..b.len()).map(|i| div(s[i] - ma[i], 0.015 * dv[i])).collect()])
}
fn mom(b: &Bars, prm: &Params) -> Output { Ok(vec![ta::change(&b.close, n(prm, "length"))]) }
fn roc(b: &Bars, prm: &Params) -> Output { Ok(vec![roc_s(&b.close, n(prm, "length"))]) }
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
    IndicatorDef { id, name, category: CAT, params, outputs: out, placement: Placement::Pane, needs, definition, first, sparse: false, compute: Some(f) }
}
const LEN14: &[ParamSpec] = &[len("length", "Length", 14.)];
const MACD: &[ParamSpec] = &[len("fast", "Fast length", 12.), len("slow", "Slow length", 26.), len("signal", "Signal length", 9.)];

pub const DEFS: &[IndicatorDef] = &[
    def("rsi", &["rsi"], "Relative Strength Index", LEN14, NO, "up=rma(max(change,0)), dn=rma(max(-change,0)); dn==0 -> 100, up==0 -> 0, else 100-100/(1+up/dn)", &[14], rsi),
    def("macd", &["macd", "signal", "hist"], "MACD", MACD, NO, "macd=ema(close,fast)-ema(close,slow); signal=ema(macd,signal); hist=macd-signal", &[25, 33, 33], macd),
    def("stoch", &["k", "d"], "Stochastic", &[len("k", "%K length", 14.), len("smooth_k", "%K smoothing", 1.), len("d", "%D smoothing", 3.)], NO,
        "raw=100*(close-lowest(low,k))/(highest(high,k)-lowest(low,k)) (NaN if range 0); k=sma(raw,smooth_k); d=sma(k,d)", &[13, 15], stoch),
    def("stochrsi", &["k", "d"], "Stochastic RSI", &[len("k", "%K smoothing", 3.), len("d", "%D smoothing", 3.), len("rsi_length", "RSI length", 14.), len("stoch_length", "Stochastic length", 14.)], NO,
        "r=rsi(close,rsi_length); k=sma(100*(r-lowest(r,s))/(highest(r,s)-lowest(r,s)),k) with highest/lowest window restarting after NaN; d=sma(k,d)", &[17, 19], stochrsi),
    def("cci", &["cci"], "Commodity Channel Index", &[len("length", "Length", 20.)], NO, "(hlc3-sma(hlc3,n))/(0.015*dev(hlc3,n)); dev 0 -> NaN", &[19], cci),
    def("mom", &["mom"], "Momentum", &[len("length", "Length", 10.)], NO, "close - close[n]", &[10], mom),
    def("roc", &["roc"], "Rate of Change", &[len("length", "Length", 9.)], NO, "100*(close-close[n])/close[n]", &[9], roc),
    def("willr", &["willr"], "Williams %R", LEN14, NO, "100*(close-highest(high,n))/(highest(high,n)-lowest(low,n)); NaN if range 0", &[13], willr),
    def("mfi", &["mfi"], "Money Flow Index", LEN14, Needs { volume: true, intraday: false },
        "src=hlc3; up=sum(v*(change<=0?0:src),n), dn=sum(v*(change>=0?0:src),n) (bar 0 counts in both); dn==0 -> 100 else 100-100/(1+up/dn)", &[13], mfi),
    def("tsi", &["tsi", "signal"], "True Strength Index", &[len("long", "Long length", 25.), len("short", "Short length", 13.), len("signal", "Signal length", 13.)], NO,
        "100*ema(ema(change,long),short)/ema(ema(|change|,long),short); signal=ema(tsi,signal)", &[37, 49], tsi),
    def("cmo", &["cmo"], "Chande Momentum Oscillator", &[len("length", "Length", 9.)], NO, "S1=sum of gains, S2=sum of losses over n; 100*(S1-S2)/(S1+S2); 0/0 -> NaN", &[9], cmo),
    def("uo", &["uo"], "Ultimate Oscillator", &[len("fast", "Fast length", 7.), len("mid", "Middle length", 14.), len("slow", "Slow length", 28.)], NO,
        "bp=close-min(low,close[1]); tr=max(high,close[1])-min(low,close[1]); A_k=sum(bp,k)/sum(tr,k); 100*(4*A_fast+2*A_mid+A_slow)/7", &[28], uo),
    def("ao", &["ao"], "Awesome Oscillator", &[len("fast", "Fast length", 5.), len("slow", "Slow length", 34.)], NO, "sma(hl2,fast)-sma(hl2,slow)", &[33], ao),
    def("ppo", &["ppo", "signal", "hist"], "Percentage Price Oscillator", MACD, NO, "100*(ema(close,fast)-ema(close,slow))/ema(close,slow); signal=ema(ppo,signal); hist=ppo-signal", &[25, 33, 33], ppo),
    def("trix", &["trix"], "TRIX", &[len("length", "Length", 18.)], NO, "10000*change(ema(ema(ema(ln(close),n),n),n))", &[52], trix),
    def("kst", &["kst", "signal"], "Know Sure Thing", &[len("roc1", "ROC 1", 10.), len("roc2", "ROC 2", 15.), len("roc3", "ROC 3", 20.), len("roc4", "ROC 4", 30.),
        len("sma1", "SMA 1", 10.), len("sma2", "SMA 2", 10.), len("sma3", "SMA 3", 10.), len("sma4", "SMA 4", 15.), len("signal", "Signal length", 9.)], NO,
        "sum_j j*sma(roc(close,roc_j),sma_j), j=1..4; signal=sma(kst,signal)", &[44, 52], kst),
    def("dpo", &["dpo"], "Detrended Price Oscillator", &[len("length", "Length", 21.)], NO, "close - sma(close,n)[floor(n/2)+1] (non-centered)", &[31], dpo),
    def("coppock", &["coppock"], "Coppock Curve", &[len("wma", "WMA length", 10.), len("long", "Long ROC", 14.), len("short", "Short ROC", 11.)], NO, "wma(roc(close,long)+roc(close,short),wma)", &[23], coppock),
    def("fisher", &["fisher", "trigger"], "Fisher Transform", &[len("length", "Length", 9.)], NO,
        "v=0.66*((hl2-ll)/(hh-ll)-0.5)+0.67*nz(v[1]) clamped to +-0.999 when |v|>0.99; fisher=0.5*ln((1+v)/(1-v))+0.5*nz(fisher[1]); trigger=fisher[1]", &[8, 9], fisher),
    def("crsi", &["crsi"], "Connors RSI", &[len("rsi", "RSI length", 3.), len("streak", "Streak RSI length", 2.), len("rank", "ROC rank length", 100.)], NO,
        "avg(rsi(close,rsi), rsi(streak,streak), percentrank(roc(close,1),rank)); streak bar 0 = -1", &[100], crsi),
];

#[cfg(test)]
mod tests {
    use super::streak;
    use crate::lab::{catalogue::{run, get, Params}, testdata::{assert_series, from_ohlc}};
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
}
