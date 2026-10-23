//! Cross-platform indicators, Volume and Breadth (2026-10-19 build list: TradingView, AmiBroker, MetaTrader, cTrader, NinjaTrader, TradeStation, thinkorswim, Sierra Chart, MetaStock). Formulas: see each definition string.
use super::bars::{Bars, Breadth};
use super::catalogue::{int, len, n, p, real, IndicatorDef, Needs, Output, ParamSpec, Params, Placement};
use super::ta::{self, NA};

const VOL: &str = "Volume";
const BRD: &str = "Breadth";
const V: Needs = Needs { volume: true, ..Needs::NONE };
const NO: Needs = Needs::NONE;
const BR: Needs = Needs { breadth: true, ..Needs::NONE };
const PN: Placement = Placement::Pane;

fn div(a: f64, b: f64) -> f64 { if b == 0. || a.is_nan() || b.is_nan() { NA } else { a / b } }
fn vol(b: &Bars) -> &[f64] { b.volume.as_deref().unwrap_or(&[]) }
fn br(b: &Bars) -> &Breadth { b.breadth.as_ref().expect("catalogue::run checks needs.breadth") }
fn zip(a: &[f64], b: &[f64], f: impl Fn(f64, f64) -> f64) -> Vec<f64> { a.iter().zip(b).map(|(&x, &y)| f(x, y)).collect() }
fn map(n: usize, f: impl Fn(usize) -> f64) -> Vec<f64> { (0..n).map(f).collect() }
/// Running total; a NaN term outputs NaN and the total continues.
fn cum(x: impl Iterator<Item = f64>) -> Vec<f64> { let mut s = 0.; x.map(|v| if v.is_nan() { NA } else { s += v; s }).collect() }
fn sign(x: f64) -> f64 { if x > 0. { 1. } else if x < 0. { -1. } else { x } }
fn bin(c: bool) -> f64 { if c { 1. } else { 0. } }
/// 1/0 flag that stays NaN while its input is NaN.
fn flag(x: f64, c: impl Fn(f64) -> bool) -> f64 { if x.is_nan() { NA } else { bin(c(x)) } }
fn prev(x: &[f64], i: usize) -> f64 { if i == 0 { NA } else { x[i - 1] } }
/// sign(close − close[1])·volume, NaN on bar 0 (our obv terms).
fn obv_terms(b: &Bars) -> Vec<f64> { zip(&ta::change(&b.close, 1), vol(b), |d, v| sign(d) * v) }
/// pandas rolling(k).max/min: NaN unless the last k bars are all non-NaN (MetaStock HHV/LLV, TradeStation Highest/Lowest).
fn roll(x: &[f64], k: usize, max: bool) -> Vec<f64> {
    map(x.len(), |i| if i + 1 < k { NA } else {
        let w = &x[i + 1 - k..=i];
        if w.iter().any(|v| v.is_nan()) { NA } else if max { w.iter().cloned().fold(f64::MIN, f64::max) } else { w.iter().cloned().fold(f64::MAX, f64::min) }
    })
}
/// Dickover's 1..10 rescale over k bars; max = min → 1 (MetaStock form).
fn norm110(x: &[f64], k: usize) -> Vec<f64> {
    let (lo, hi) = (roll(x, k, false), roll(x, k, true));
    map(x.len(), |i| if lo[i].is_nan() { NA } else if hi[i] == lo[i] { 1. } else { 1. + 9. * (x[i] - lo[i]) / (hi[i] - lo[i]) })
}
/// (x − sma(x, k))/stdev(x, k) (population); zero deviation → 0 (TradeStation RelativeVolume / FoM).
fn zscore(x: &[f64], k: usize) -> Vec<f64> {
    let (m, s) = (ta::sma(x, k), ta::stdev(x, k));
    map(x.len(), |i| if s[i].is_nan() { NA } else if s[i] == 0. { 0. } else { (x[i] - m[i]) / s[i] })
}

fn accdist_open(b: &Bars, prm: &Params) -> Output {
    let pc = p(prm, "prev_close") == 1.;
    let v = vol(b);
    let ad = cum((0..b.len()).map(|i| {
        let r = if pc { prev(&b.close, i) } else { b.open[i] };
        if r.is_nan() { NA } else if b.high[i] == b.low[i] { 0. } else { v[i] * (b.close[i] - r) / (b.high[i] - b.low[i]) }
    }));
    let ma = ta::sma(&ad, n(prm, "ma_length"));
    Ok(vec![ad, ma])
}
fn aobv(b: &Bars, prm: &Params) -> Output {
    let (mut f, mut s) = (n(prm, "fast"), n(prm, "slow"));
    if s < f { std::mem::swap(&mut f, &mut s); }
    let obv = cum(obv_terms(b).into_iter());
    let (fast, slow) = (ta::ema(&obv, f), ta::ema(&obv, s));
    let (df, ds) = (ta::change(&fast, 2), ta::change(&slow, 2));
    let run = |up: bool| map(b.len(), |i| if df[i].is_nan() || ds[i].is_nan() { NA } else { bin((if up { df[i] > 0. } else { df[i] < 0. }) && ds[i] != 0.) });
    let (lo, hi) = (roll(&obv, n(prm, "min_lookback"), false), roll(&obv, n(prm, "max_lookback"), true));
    let (lr, sr) = (run(true), run(false));
    Ok(vec![obv, lo, hi, fast, slow, lr, sr])
}
fn bar_up_down_volume(b: &Bars, prm: &Params) -> Output {
    let open = p(prm, "basis") == 1.;
    let v = vol(b);
    let r: Vec<f64> = map(b.len(), |i| if open { b.open[i] } else { prev(&b.close, i) });
    let up = map(b.len(), |i| if r[i].is_nan() { NA } else if b.close[i] > r[i] { v[i] } else { 0. });
    let dn = map(b.len(), |i| if r[i].is_nan() { NA } else if b.close[i] < r[i] { v[i] } else { 0. });
    let net = zip(&up, &dn, |u, d| u - d);
    Ok(vec![up, dn, net])
}
fn dvi(b: &Bars, _: &Params) -> Output {
    let v = vol(b);
    Ok(vec![cum((0..b.len()).map(|i| { let m = (b.high[i] + b.low[i]) / 2.; v[i] * div(b.close[i] - m, m) }))])
}
fn freedom_of_movement(b: &Bars, prm: &Params) -> Output {
    let k = n(prm, "length");
    let am = map(b.len(), |i| div(b.close[i] - prev(&b.close, i), prev(&b.close, i)).abs());
    let vbym = zip(&norm110(&zscore(vol(b), k), k), &norm110(&am, k), |v, m| v / m);
    let fom = zscore(&vbym, k);
    let sd = p(prm, "sd");
    let spike = fom.iter().map(|&x| flag(x, |x| x > sd)).collect();
    Ok(vec![fom, spike])
}
fn iii(b: &Bars, prm: &Params) -> Output {
    let (v, k) = (vol(b), n(prm, "length"));
    let ii = map(b.len(), |i| { let r = b.high[i] - b.low[i]; if r == 0. { 0. } else { v[i] * (2. * b.close[i] - b.high[i] - b.low[i]) / r } });
    let pct = zip(&ta::sum(&ii, k), &ta::sum(v, k), |a, s| div(100. * a, s));
    Ok(vec![ii, pct])
}
fn mfo(b: &Bars, prm: &Params) -> Output {
    let (v, k, h, l) = (vol(b), n(prm, "length"), &b.high, &b.low);
    let mfv = map(b.len(), |i| if i == 0 { NA } else {
        let (a, c) = (h[i] - l[i - 1], h[i - 1] - l[i]);
        let m = if h[i] < l[i - 1] { -1. } else if l[i] > h[i - 1] { 1. } else if a + c == 0. { 0. } else { (a - c) / (a + c) };
        m * v[i]
    });
    let vv = map(b.len(), |i| if i == 0 { NA } else { v[i] });
    Ok(vec![zip(&ta::sum(&mfv, k), &ta::sum(&vv, k), div)])
}
fn money_flow_cum(b: &Bars, _: &Params) -> Output {
    Ok(vec![cum(obv_terms(b).iter().zip(&b.close).map(|(t, c)| t * c))])
}
fn obv_modified(b: &Bars, prm: &Params) -> Output {
    let t = obv_terms(b);
    let obvm = ta::ema(&cum(t.iter().cloned()), n(prm, "smooth"));
    let sig = ta::ema(&obvm, n(prm, "signal"));
    Ok(vec![obvm, sig, ta::sum(&t, n(prm, "st_length"))])
}
fn pvr(b: &Bars, _: &Params) -> Output {
    let v = vol(b);
    Ok(vec![map(b.len(), |i| if i == 0 { NA } else {
        match (b.close[i] >= b.close[i - 1], v[i] >= v[i - 1]) { (true, true) => 1., (true, false) => 2., (false, false) => 3., (false, true) => 4. }
    })])
}
fn tsv(b: &Bars, prm: &Params) -> Output {
    let (v, oc) = (vol(b), p(prm, "variant") == 1.);
    let x = map(b.len(), |i| v[i] * (b.close[i] - if oc { b.open[i] } else { prev(&b.close, i) }));
    let t = ta::sum(&x, n(prm, "length"));
    let sig = ta::sma(&t, n(prm, "signal"));
    Ok(vec![t, sig])
}
fn tvi(b: &Bars, prm: &Params) -> Output {
    let (v, mt) = (vol(b), p(prm, "min_tick"));
    let mut dir = NA;
    Ok(vec![cum((0..b.len()).map(|i| {
        let d = b.close[i] - prev(&b.close, i);
        if d > mt { dir = 1. } else if d < -mt { dir = -1. }
        dir * v[i]
    }))])
}
fn twiggs(b: &Bars, prm: &Params) -> Output {
    let (v, k) = (vol(b), n(prm, "length"));
    let ad = map(b.len(), |i| if i == 0 { NA } else {
        let (hi, lo, c) = (b.high[i].max(b.close[i - 1]), b.low[i].min(b.close[i - 1]), b.close[i]);
        if hi == lo { 0. } else { v[i] * ((c - lo) - (hi - c)) / (hi - lo) }
    });
    let vv = map(b.len(), |i| if i == 0 { NA } else { v[i] });
    Ok(vec![zip(&ta::rma(&ad, k), &ta::rma(&vv, k), div)])
}
fn vfi(b: &Bars, prm: &Params) -> Output {
    let (v, k) = (vol(b), n(prm, "length"));
    let tp = b.hlc3();
    let inter = map(b.len(), |i| tp[i].ln() - prev(&tp, i).ln());
    let vin = ta::stdev(&inter, 30);
    let ma = ta::sma(v, k);
    let (coef, vcoef) = (p(prm, "coef"), p(prm, "vcoef"));
    let vave = map(b.len(), |i| prev(&ma, i));
    let vcp = map(b.len(), |i| {
        let (cut, va) = (coef * vin[i] * b.close[i], vave[i]);
        if cut.is_nan() || va.is_nan() { return NA; }
        let vc = if v[i] < va * vcoef { v[i] } else { va * vcoef };
        let mf = tp[i] - tp[i - 1];
        if mf > cut { vc } else if mf < -cut { -vc } else { 0. }
    });
    let f = ta::sma(&zip(&ta::sum(&vcp, k), &vave, div), n(prm, "smooth"));
    let sig = ta::ema(&f, n(prm, "signal"));
    Ok(vec![f, sig])
}
fn volume_ratio_jp(b: &Bars, prm: &Params) -> Output {
    let (v, k) = (vol(b), n(prm, "length"));
    let pick = |s: f64| map(b.len(), |i| if i == 0 { NA } else if sign(b.close[i] - b.close[i - 1]) == s { v[i] } else { 0. });
    let (u, d, s) = (ta::sum(&pick(1.), k), ta::sum(&pick(-1.), k), ta::sum(&pick(0.), k));
    Ok(vec![map(b.len(), |i| div(100. * (u[i] + s[i] / 2.), d[i] + s[i] / 2.))])
}
fn volume_zscore(b: &Bars, prm: &Params) -> Output {
    let z = zscore(vol(b), n(prm, "length"));
    let t = p(prm, "spike");
    let s = z.iter().map(|&x| flag(x, |x| x > t)).collect();
    Ok(vec![z, s])
}
fn vpn(b: &Bars, prm: &Params) -> Output {
    let (v, k) = (vol(b), n(prm, "length"));
    let tp = b.hlc3();
    let atr = ta::sma(&ta::tr(b, true), k);
    let kk = p(prm, "k");
    let side = |up: bool| map(b.len(), |i| if i == 0 || atr[i].is_nan() { NA } else {
        let d = tp[i] - tp[i - 1];
        if (up && d > kk * atr[i]) || (!up && d < -kk * atr[i]) { v[i] } else { 0. }
    });
    let (vp, vn, tv) = (ta::sum(&side(true), k), ta::sum(&side(false), k), ta::sum(v, k));
    let x = map(b.len(), |i| div(100. * (vp[i] - vn[i]), tv[i]));
    Ok(vec![ta::ema(&x, n(prm, "smooth"))])
}
fn vroc(b: &Bars, prm: &Params) -> Output {
    let (v, k) = (vol(b), n(prm, "length"));
    let x = map(b.len(), |i| if i >= k { div(100. * (v[i] - v[i - k]), v[i - k]) } else { NA });
    Ok(vec![ta::sma(&x, n(prm, "smooth"))])
}
fn vzo(b: &Bars, prm: &Params) -> Output {
    let k = n(prm, "length");
    let vv = map(b.len(), |i| if i == 0 { NA } else { vol(b)[i] }); // both averages start at bar 1, as R does
    Ok(vec![zip(&ta::ema(&obv_terms(b), k), &ta::ema(&vv, k), |a, t| div(100. * a, t))])
}
fn wad(b: &Bars, prm: &Params) -> Output {
    let ab = p(prm, "unchanged") == 1.;
    Ok(vec![cum((0..b.len()).map(|i| if i == 0 { 0. } else {
        let (c, c1) = (b.close[i], b.close[i - 1]);
        let (hi, lo) = (b.high[i].max(c1), b.low[i].min(c1));
        if c > c1 { c - lo } else if c < c1 || ab { c - hi } else { 0. }
    }))])
}
fn ad_share(b: &Bars, scale: f64) -> Vec<f64> { let r = br(b); zip(&r.adv, &r.dec, |a, d| div(scale * a, a + d)) }
fn breadth_thrust(b: &Bars, prm: &Params) -> Output {
    let (x, k) = (ad_share(b, 1.), n(prm, "length"));
    Ok(vec![if p(prm, "ma_type") == 1. { ta::sma(&x, k) } else { ta::ema(&x, k) }])
}
fn mcclellan(b: &Bars, prm: &Params) -> Output {
    let r = br(b);
    let net = if p(prm, "ratio_adjusted") == 1. { zip(&r.adv, &r.dec, |a, d| div(1000. * (a - d), a + d)) } else { zip(&r.adv, &r.dec, |a, d| a - d) };
    let osc = zip(&ta::ema(&net, n(prm, "fast")), &ta::ema(&net, n(prm, "slow")), |f, s| f - s);
    let start = p(prm, "start");
    let sum = cum(osc.iter().cloned()).into_iter().map(|s| s + start).collect();
    Ok(vec![osc, sum])
}
fn stix(b: &Bars, prm: &Params) -> Output { Ok(vec![ta::ema(&ad_share(b, 100.), n(prm, "length"))]) }

#[allow(clippy::too_many_arguments)]
const fn def(id: &'static str, name: &'static str, category: &'static str, params: &'static [ParamSpec], outputs: &'static [&'static str],
             needs: Needs, definition: &'static str, first: &'static [usize], f: fn(&Bars, &Params) -> Output) -> IndicatorDef {
    IndicatorDef { id, name, category, params, outputs, placement: PN, needs, definition, first, sparse: false, compute: Some(f), note: "" }
}
const NONE: &[ParamSpec] = &[];
const ADO: &[ParamSpec] = &[int("prev_close", "Use previous close instead of open (0/1)", 0., 0., 1.), len("ma_length", "MA length", 10.)];
const AOBV: &[ParamSpec] = &[len("fast", "Fast EMA", 4.), len("slow", "Slow EMA", 12.), len("max_lookback", "Max lookback", 2.), len("min_lookback", "Min lookback", 2.)];
const BUD: &[ParamSpec] = &[int("basis", "Basis (0 close vs prior close, 1 close vs open)", 0., 0., 1.)];
const FOM: &[ParamSpec] = &[len("length", "Length", 60.), real("sd", "Spike standard deviations", 2., 0.1, 10., 0.1)];
const III: &[ParamSpec] = &[len("length", "II% length", 21.)];
const L20: &[ParamSpec] = &[len("length", "Length", 20.)];
const L21: &[ParamSpec] = &[len("length", "Length", 21.)];
const L14: &[ParamSpec] = &[len("length", "Length", 14.)];
const OBVM: &[ParamSpec] = &[len("smooth", "OBV EMA", 7.), len("signal", "Signal EMA", 10.), len("st_length", "Short-term window", 10.)];
const TSV: &[ParamSpec] = &[len("length", "Length", 18.), len("signal", "Signal SMA", 10.), int("variant", "Variant (0 close change, 1 close - open)", 0., 0., 1.)];
const TVI: &[ParamSpec] = &[real("min_tick", "Minimum tick value", 0.05, 0., 1e6, 0.01)];
const VFI: &[ParamSpec] = &[len("length", "Length", 130.), real("coef", "Cutoff coefficient", 0.2, 0., 5., 0.01), real("vcoef", "Volume cutoff", 2.5, 0.1, 10., 0.1),
    len("signal", "Signal EMA", 5.), len("smooth", "Smoothing SMA", 3.)];
const VZS: &[ParamSpec] = &[len("length", "Length", 60.), real("spike", "Spike threshold", 2., 0.1, 10., 0.1)];
const VPN: &[ParamSpec] = &[len("length", "Length", 30.), len("smooth", "EMA smoothing", 3.), real("k", "ATR fraction", 0.1, 0., 5., 0.01)];
const VROC: &[ParamSpec] = &[len("length", "Length", 14.), len("smooth", "SMA smoothing (NT8 3)", 1.)];
const WAD: &[ParamSpec] = &[int("unchanged", "Unchanged close (0 adds 0 - MT5/TA-Lib, 1 adds C - TRH - AmiBroker)", 0., 0., 1.)];
const BT: &[ParamSpec] = &[len("length", "Length", 10.), int("ma_type", "MA type (0 EMA, 1 SMA)", 0., 0., 1.)];
const MCC: &[ParamSpec] = &[len("fast", "Fast EMA", 19.), len("slow", "Slow EMA", 39.), int("ratio_adjusted", "Ratio-adjusted (0/1)", 0., 0., 1.),
    real("start", "Summation start", 0., -1e6, 1e6, 1.)];

pub const DEFS: &[IndicatorDef] = &[
    def("accdist_open", "Accumulation/Distribution Flow (open-based)", VOL, ADO, &["ad", "ma"], V,
        "ad = cumulative volume·(close − open)/(high − low) (prev_close 1: close − close[1], bar 0 NaN); high = low → 0; ma = sma(ad, ma_length) (TOS AccumDistPrVol; Sierra Chart A/D Flow)", &[0, 9], accdist_open),
    def("aobv", "Archer On-Balance Volume", VOL, AOBV, &["obv", "obv_min", "obv_max", "fast", "slow", "long_run", "short_run"], V,
        "obv = our obv (bar 0 NaN); obv_min/obv_max = rolling min/max of obv over min/max_lookback (pandas rolling: NaN until the window is full); fast/slow = ema(obv, fast/slow) (our SMA-seeded EMA; slow < fast swaps them); run length 2 (pandas-ta default): long_run = fast − fast[2] > 0 and slow − slow[2] ≠ 0, short_run = fast − fast[2] < 0 and slow − slow[2] ≠ 0 (1/0; NaN until both differences exist — pandas-ta prints 0) (pandas_ta/volume/aobv.py, trend/long_run.py, trend/short_run.py)",
        &[1, 2, 2, 4, 12, 14, 14], aobv),
    def("bar_up_down_volume", "Up/Down Volume (bar direction)", VOL, BUD, &["up", "down", "net"], V,
        "up = close > ref ? volume : 0; down = close < ref ? volume : 0; net = up − down; ref = close[1] (basis 0, bar 0 NaN) or open (basis 1). Daily-bar adaptation of Sierra Chart Volume-Up/Volume-Down; the tick-based up_down_volume / net_volume stay listed", &[1, 1, 1], bar_up_down_volume),
    def("dvi", "Daily Volume Indicator", VOL, NONE, &["dvi"], V,
        "mid = (high + low)/2; dvi = cumulative volume·(close − mid)/mid (AmiBroker Formulas/Indicators/DVI - Daily Volume Indicator.afl)", &[0], dvi),
    def("freedom_of_movement", "Freedom of Movement (Dickover)", VOL, FOM, &["fom", "spike"], V,
        "am = |close/close[1] − 1|; rv = (volume − sma(volume, n))/stdev(volume, n); norm(x) = 1 + 9·(x − llv(x, n))/(hhv(x, n) − llv(x, n)) over a full n-bar window (hhv = llv → 1, MetaStock); r = norm(rv)/norm(am); fom = (r − sma(r, n))/stdev(r, n); population stdev, zero stdev → 0 (TradeStation); negative values kept; spike = fom > sd (1/0) (Dickover, TASC Apr 2014 Traders' Tips TradeStation FoM and MetaStock; Sierra Chart ID 515 uses the signed % move)",
        &[177, 177], freedom_of_movement),
    def("iii", "Intraday Intensity (II and II%)", VOL, III, &["ii", "ii_pct"], V,
        "ii = volume·(2·close − high − low)/(high − low), high = low → 0 (TA4J/Chartink; Pine ta.iii instead divides by volume: (2C − H − L)/((H − L)·V)); ii_pct = 100·sum(ii, n)/sum(volume, n), zero volume sum → NaN", &[0, 20], iii),
    def("mfo", "Money Flow Oscillator (Apirine)", VOL, L20, &["mfo"], V,
        "a = high − low[1], c = high[1] − low; mult = high < low[1] ? −1 : low > high[1] ? 1 : (a − c)/(a + c) (a + c = 0 → 0); mfo = sum(mult·volume, n)/sum(volume, n) over bars from 1 (NT8 MoneyFlowOscillator.cs without its 2- and 3-decimal rounding; Apirine TASC Oct 2015)", &[20], mfo),
    def("money_flow_cum", "Money Flow (cumulative)", VOL, NONE, &["mf"], V,
        "mf = cumulative sign(close − close[1])·close·volume; unchanged close adds 0; bar 0 NaN (TOS MoneyFlow, price = close)", &[1], money_flow_cum),
    def("obv_modified", "On Balance Volume Modified / Short-Term OBV", VOL, OBVM, &["obvm", "signal", "obv_st"], V,
        "obvm = ema(obv, smooth); signal = ema(obvm, signal) (Apirine TASC Apr 2020, TOS OnBalanceVolumeModified, EMA default); obv_st = sum(sign(close − close[1])·volume, st_length) = obv − obv[st_length] (Sierra Chart OBV-Short Term)", &[7, 16, 10], obv_modified),
    def("pvr", "Price-Volume Rank", VOL, NONE, &["rank"], V,
        "up = close ≥ close[1], vup = volume ≥ volume[1] (pandas-ta ties); rank 1 up & vup, 2 up & volume down, 3 close down & volume down, 4 close down & vup (thinkorswim PriceVolumeRank numbering — pandas_ta/volume/pvr.py swaps 3 and 4); bar 0 NaN", &[1], pvr),
    def("tsv", "Time Segmented Volume", VOL, TSV, &["tsv", "signal"], V,
        "tsv = sum(volume·(close − close[1]), n) (pandas_ta/volume/tsv.py; variant 1: close − open, TA4J); signal = sma(tsv, signal)", &[18, 27], tsv),
    def("tvi", "Trade Volume Index", VOL, TVI, &["tvi"], V,
        "dir = close − close[1] > min_tick ? 1 : < −min_tick ? −1 : dir[1] (NaN until the first move beyond min_tick); tvi = cumulative dir·volume (AmiBroker TVI - Trade Volume Index.afl, ValueWhen; min tick is in price units: AmiBroker 0.1, ChartIQ 0.5, 0.05 = NSE tick)", &[1], tvi),
    def("twiggs", "Twiggs Money Flow", VOL, L21, &["tmf"], V,
        "trh = max(close[1], high); trl = min(close[1], low); ad = volume·((close − trl) − (trh − close))/(trh − trl) (zero range → 0) from bar 1; tmf = rma(ad, n)/rma(volume, n) (Wilder, SMA seed; ChartIQ calculateTwiggsMoneyFlow decays plain sums by (n−1)/n — same ratio after its seed)", &[21], twiggs),
    def("vfi", "Volume Flow Indicator (Katsanos)", VOL, VFI, &["vfi", "signal"], V,
        "tp = hlc3; cutoff = coef·stdev(ln tp − ln tp[1], 30)·close; vave = sma(volume, n)[1]; vc = min(volume, vcoef·vave); mf = tp − tp[1]; vcp = mf > cutoff ? vc : mf < −cutoff ? −vc : 0 (NaN until vave and cutoff exist); vfi = sma(sum(vcp, n)/vave, smooth); signal = ema(vfi, signal) (LazyBear Pine; smooth 1 = its default smoothVFI off; TOS/Katsanos MetaStock smooth with EMA 3)", &[261, 265], vfi),
    def("volume_ratio_jp", "Volume Ratio (Japan)", VOL, L20, &["vr"], V,
        "U/D/S = sum over n of volume on up/down/unchanged closes (vs close[1], from bar 1); vr = 100·(U + S/2)/(D + S/2), zero denominator → NaN (Japanese VR as Rakuten MarketSpeed / moomoo; TradeStation Volume Ratio JP does not publish its unchanged-bar rule)", &[20], volume_ratio_jp),
    def("volume_zscore", "Relative Volume Standard Deviation", VOL, VZS, &["z", "spike"], V,
        "z = (volume − sma(volume, n))/stdev(volume, n), population stdev, zero stdev → 0 (Dickover RelativeVolume, TASC Apr 2014; TOS RelativeVolumeStDev; negative values kept); spike = z > threshold (1/0)", &[59, 59], volume_zscore),
    def("vpn", "Volume Positive-Negative (Katsanos)", VOL, VPN, &["vpn"], V,
        "tp = hlc3; atr = sma(true range, n) (TradeStation AvgTrueRange); vp/vn = sum(tp − tp[1] > k·atr / < −k·atr ? volume : 0, n) (NaN until atr exists); vpn = ema(100·(vp − vn)/sum(volume, n), smooth) (Katsanos TASC Apr 2021 TradeStation code; MetaStock uses ≥ and Wilder ATR)", &[60], vpn),
    def("vroc", "Volume Rate of Change", VOL, VROC, &["vroc"], V,
        "x = 100·(volume − volume[n])/volume[n] (MetaStock; volume[n] = 0 → NaN); vroc = sma(x, smooth) (NT8 VROC smooths with 3 and uses volume[n−1])", &[14], vroc),
    def("vzo", "Volume Zone Oscillator", VOL, L14, &["vzo"], V,
        "vzo = 100·ema(sign(close − close[1])·volume, n)/ema(volume, n) (both from bar 1; Khalil & Steckler TASC May 2011; our SMA-seeded EMA, TradeStation XAverage seeds with the first value). Levels ±60, ±40, ±15", &[14], vzo),
    def("wad", "Williams Accumulation/Distribution", VOL, WAD, &["wad"], NO,
        "trh = max(high, close[1]); trl = min(low, close[1]); ad = close > close[1] ? close − trl : close < close[1] ? close − trh : 0 (unchanged 1: close − trh, AmiBroker AD Price); wad = cumulative ad, bar 0 = 0 (Pine ta.wad; Tulip wad.c starts at bar 1). No volume", &[0], wad),
    def("breadth_thrust", "Breadth Thrust (Zweig)", BRD, BT, &["thrust"], BR,
        "thrust = ema(advances/(advances + declines), n) (AmiBroker Breadth Thrust.afl; ma_type 1 = sma, MetaStock); A + D = 0 or missing breadth → NaN. Signal: below 0.40 then above 0.615", &[9], breadth_thrust),
    def("mcclellan", "McClellan Oscillator and Summation Index", BRD, MCC, &["osc", "summation"], BR,
        "net = advances − declines (ratio_adjusted 1: 1000·(A − D)/(A + D)); osc = ema(net, fast) − ema(net, slow) (NT8 McClellanOscillator.cs; 19/39 = 10%/5% smoothing; our SMA seed); summation = start + cumulative osc (TOS cumulative mode; classic ratio-adjusted start 1000); days without breadth NaN, sums continue", &[38, 38], mcclellan),
    def("stix", "STIX", BRD, L21, &["stix"], BR,
        "stix = ema(100·advances/(advances + declines), n) (AmiBroker STIX.afl, n = 21); A + D = 0 or missing breadth → NaN", &[20], stix),
];

/// (id, platform names, AmiBroker names): one row per id in DEFS.
pub const SOURCES: &[(&str, &str, &str)] = &[
    ("accdist_open", "thinkorswim: AccumDistPrVol; Sierra Chart: Accumulation/Distribution Flow", ""),
    ("aobv", "pandas-ta: aobv", ""),
    ("bar_up_down_volume", "Sierra Chart: Volume-Up / Volume-Down (daily-bar adaptation)", ""),
    ("dvi", "AmiBroker: DVI - Daily Volume Indicator", "DVI - Daily Volume Indicator (Formulas/Indicators)"),
    ("freedom_of_movement", "Sierra Chart: Freedom of Movement; thinkorswim: FreedomOfMovement; TradeStation: FoM (TASC Apr 2014)", ""),
    ("iii", "TradingView: ta.iii; TA4J: IIIIndicator; Chartink: iii", ""),
    ("mfo", "NinjaTrader 8: MoneyFlowOscillator; thinkorswim: MoneyFlowOscillator", ""),
    ("money_flow_cum", "thinkorswim: MoneyFlow", ""),
    ("obv_modified", "thinkorswim: OnBalanceVolumeModified; Sierra Chart: OBV-Short Term", ""),
    ("pvr", "pandas-ta: pvr; thinkorswim: PriceVolumeRank", ""),
    ("tsv", "pandas-ta: tsv; TA4J: TimeSegmentedVolumeIndicator", ""),
    ("tvi", "AmiBroker: TVI - Trade Volume Index; MetaStock: tvi(); cTrader: TradeVolumeIndex; Sierra Chart: Trade Volume Index; thinkorswim: TradeVolumeIndex; Zerodha Kite (ChartIQ): Trade Vol", "TVI - Trade Volume Index (Formulas/Indicators)"),
    ("twiggs", "Zerodha Kite (ChartIQ): Twiggs Money Flow; Zerodha Streak: Twiggs Money Flow", ""),
    ("vfi", "thinkorswim: VolumeFlowIndicator; TradingView community: Volume Flow Indicator [LazyBear]", ""),
    ("volume_ratio_jp", "TradeStation: Volume Ratio JP", ""),
    ("volume_zscore", "Sierra Chart: Relative Volume Standard Deviation; thinkorswim: RelativeVolumeStDev; TA4J: RelativeVolumeStandardDeviationIndicator; pandas-ta: vhm", ""),
    ("vpn", "Sierra Chart: VPN; thinkorswim: VPNIndicator", ""),
    ("vroc", "MetaStock: Volume ROC; NinjaTrader 8: VROC; MetaTrader 5: VROC; cTrader: VolumeROC; TradeStation: Volume Rate of Chg; MultiCharts: Volume Rate of Chg; thinkorswim: VolumeRateOfChange", ""),
    ("vzo", "thinkorswim: VolumeZoneOscillator; Sierra Chart: Volume Zone Oscillator", ""),
    ("wad", "TradingView: ta.wad; TA-Lib: WAD; Tulip: wad; MetaTrader 5: W_AD; cTrader: Williams Accumulation Distribution; TradeStation: Williams AD; MultiCharts: Williams AD; thinkorswim: AccumDistBuyPr; Sierra Chart: A/D-Williams; MetaStock: willa(); Zerodha Kite (ChartIQ)", "AD Price - Williams Advance Decline of Price"),
    ("breadth_thrust", "AmiBroker: Breadth Thrust; MetaStock: Breadth Thrust", "Breadth Thrust (Formulas/Indicators)"),
    ("mcclellan", "NinjaTrader 8: McClellanOscillator; thinkorswim: McClellanOscillator, McClellanSummationIndex; MetaStock; Sierra Chart: McClellan; TradeStation: McClellan Osc; MultiCharts: McClellan Osc", ""),
    ("stix", "AmiBroker: STIX; Sierra Chart: STIX; MetaStock", "STIX (Formulas/Indicators)"),
];

#[cfg(test)]
mod tests {
    use super::{norm110, roll, zscore};
    use crate::lab::bars::{Bars, Breadth};
    use crate::lab::catalogue::{get, run, Params};
    use crate::lab::testdata::{assert_series, close_eq, closes, compiles, cut, from_ohlc, go, walk, with_breadth};
    const N: f64 = f64::NAN;
    fn with(rows: &[(f64, f64, f64, f64)], v: &[f64]) -> Bars { let mut b = from_ohlc(rows); b.volume = Some(v.to_vec()); b }
    fn cv(c: &[f64], v: &[f64]) -> Bars { let mut b = closes(c); b.volume = Some(v.to_vec()); b }
    fn ab(adv: &[f64], dec: &[f64]) -> Bars {
        let mut b = closes(&vec![1.; adv.len()]);
        let z = vec![0.; adv.len()];
        b.breadth = Some(Breadth { adv: adv.to_vec(), dec: dec.to_vec(), unch: z.clone(), adv_vol: z.clone(), dec_vol: z.clone(), unch_vol: z, ..Default::default() });
        b
    }

    #[test] fn helpers_rolling_rescale_and_zscore() {
        assert_series(&roll(&[N, 3., 1., 2.], 2, true), &[N, N, 3., 2.]);
        assert_series(&roll(&[N, 3., 1., 2.], 2, false), &[N, N, 1., 1.]);
        assert_series(&norm110(&[1., 2., 3., 2.], 3), &[N, N, 10., 1.]);
        assert_series(&norm110(&[2., 2.], 2), &[N, 1.]);
        assert_series(&zscore(&[1., 2., 3.], 3), &[N, N, 1. / (2f64 / 3.).sqrt()]);
        assert_series(&zscore(&[5., 5.], 2), &[N, 0.]);
    }
    #[test] fn accdist_open_and_dvi() {
        let b = with(&[(10., 12., 8., 10.), (10., 12., 8., 11.), (11., 11., 11., 11.)], &[100., 100., 100.]);
        assert_series(&go("accdist_open", &b, &[("ma_length", 2.)])[0], &[0., 25., 25.]);
        assert_series(&go("accdist_open", &b, &[("ma_length", 2.)])[1], &[N, 12.5, 25.]);
        assert_series(&go("accdist_open", &b, &[("prev_close", 1.)])[0], &[N, 25., 25.]);
        let flat = with(&[(10., 12., 8., 10.), (11., 13., 9., 11.)], &[5., 7.]); // close = open every bar
        assert_series(&go("accdist_open", &flat, &[])[0], &[0., 0.]);
        // mid 10 → close 11 adds 100·0.1; close at mid adds 0
        assert_series(&go("dvi", &b, &[])[0], &[0., 10., 10.]);
    }
    #[test] fn aobv_matches_ema_of_obv_and_runs() {
        let b = walk(200, 3, true);
        let o = go("aobv", &b, &[]);
        let obv = &go("obv", &b, &[])[0];
        assert_series(&o[0], obv);
        assert_series(&o[3], &crate::lab::ta::ema(obv, 4));
        assert_series(&o[4], &crate::lab::ta::ema(obv, 12));
        for i in 14..200 {
            let (df, ds) = (o[3][i] - o[3][i - 2], o[4][i] - o[4][i - 2]);
            assert_eq!(o[5][i], if df > 0. && ds != 0. { 1. } else { 0. }, "long_run {i}");
            assert_eq!(o[6][i], if df < 0. && ds != 0. { 1. } else { 0. }, "short_run {i}");
            assert!(o[1][i] == obv[i].min(obv[i - 1]) && o[2][i] == obv[i].max(obv[i - 1]));
        }
        let up = cv(&(1..=30).map(|x| x as f64).collect::<Vec<_>>(), &[10.; 30]);
        let o = go("aobv", &up, &[]);
        assert_eq!((o[5][29], o[6][29]), (1., 0.));
    }
    #[test] fn bar_up_down_volume_splits_by_direction() {
        let b = cv(&[1., 2., 2., 1.], &[10., 20., 30., 40.]);
        let o = go("bar_up_down_volume", &b, &[]);
        assert_series(&o[0], &[N, 20., 0., 0.]);
        assert_series(&o[1], &[N, 0., 0., 40.]);
        assert_series(&o[2], &[N, 20., 0., -40.]);
        let oc = with(&[(1., 2., 0., 1.5), (2., 2., 1., 1.), (1., 1., 1., 1.)], &[5., 6., 7.]);
        let o = go("bar_up_down_volume", &oc, &[("basis", 1.)]);
        assert_series(&o[2], &[5., -6., 0.]);
    }
    #[test] fn freedom_of_movement_is_a_zscore_with_spikes() {
        let b = walk(400, 5, true);
        let o = go("freedom_of_movement", &b, &[("length", 20.), ("sd", 1.5)]);
        assert!(o[0][..57].iter().all(|v| v.is_nan()) && o[0][57].is_finite());
        for (f, s) in o[0][57..].iter().zip(&o[1][57..]) { assert_eq!(*s, if *f > 1.5 { 1. } else { 0. }); assert!(f.abs() <= 20f64.sqrt()); }
        assert!(o[1].contains(&1.), "some spikes on a random walk");
    }
    #[test] fn intraday_intensity() {
        let b = with(&[(1., 2., 1., 2.), (2., 3., 2., 3.)], &[10., 30.]); // close at the high
        let o = go("iii", &b, &[("length", 2.)]);
        assert_series(&o[0], &[10., 30.]);
        assert_series(&o[1], &[N, 100.]);
        let mid = with(&[(1., 2., 0., 1.)], &[10.]);
        assert_series(&go("iii", &mid, &[("length", 1.)])[0], &[0.]);
    }
    #[test] fn money_flow_oscillator() {
        let b = with(&[(10., 11., 9., 10.), (10., 12., 10., 11.), (13., 14., 13., 14.), (9., 10., 8., 9.)], &[1., 100., 50., 20.]);
        // bar 1: a = 12 − 9, c = 11 − 10 → 0.5; bar 2 gaps up → 1; bar 3 gaps down → −1
        assert_series(&go("mfo", &b, &[("length", 1.)])[0], &[N, 0.5, 1., -1.]);
        assert_series(&go("mfo", &b, &[("length", 2.)])[0], &[N, N, 100. / 150., 30. / 70.]);
        let w = go("mfo", &walk(300, 9, true), &[]);
        assert!(w[0][20..].iter().all(|v| (-1. ..=1.).contains(v)));
    }
    #[test] fn money_flow_obv_modified_and_pvr() {
        let b = cv(&[10., 11., 11., 10.], &[1., 2., 3., 4.]);
        assert_series(&go("money_flow_cum", &b, &[])[0], &[N, 22., 22., -18.]);
        let w = walk(300, 2, true);
        let obv = &go("obv", &w, &[])[0];
        let o = go("obv_modified", &w, &[]);
        assert_series(&o[0], &crate::lab::ta::ema(obv, 7));
        for i in 11..300 { assert!(close_eq(o[2][i], obv[i] - obv[i - 10]), "obv_st {i}"); }
        let p = cv(&[10., 11., 11., 10., 9.], &[100., 200., 150., 150., 100.]);
        assert_series(&go("pvr", &p, &[])[0], &[N, 1., 2., 4., 3.]);
    }
    #[test] fn time_segmented_volume_and_trade_volume_index() {
        assert_series(&go("tsv", &cv(&[5.; 4], &[1., 2., 3., 4.]), &[("length", 2.), ("signal", 1.)])[0], &[N, N, 0., 0.]);
        let b = cv(&[10., 11., 11., 10., 10.], &[1., 2., 3., 4., 5.]);
        assert_series(&go("tsv", &b, &[("length", 2.), ("signal", 2.)])[0], &[N, N, 2., -4., -4.]);
        assert_series(&go("tsv", &b, &[("length", 2.), ("signal", 2.)])[1], &[N, N, N, -1., -4.]);
        // min_tick 0: like obv, but unchanged closes keep the last direction
        assert_series(&go("tvi", &b, &[("min_tick", 0.)])[0], &[N, 2., 5., 1., -4.]);
        assert_series(&go("tvi", &b, &[("min_tick", 1.)])[0], &[N, N, N, N, N]);
    }
    #[test] fn twiggs_is_one_when_closing_at_the_true_high() {
        let rows: Vec<_> = (0..10).map(|i| { let c = 10. + i as f64; (c - 1., c, c - 1.5, c) }).collect();
        let v: Vec<f64> = (0..10).map(|i| 100. + i as f64).collect();
        let o = go("twiggs", &with(&rows, &v), &[("length", 3.)]);
        assert_series(&o[0], &[N, N, N, 1., 1., 1., 1., 1., 1., 1.]);
    }
    #[test] fn volume_flow_indicator() {
        assert!(go("vfi", &cv(&[100.; 300], &[1000.; 300]), &[])[0][261..].iter().all(|&v| v == 0.), "no move → 0");
        // steady 1% rise: zero log-return stdev → cutoff 0, every bar adds vc = volume = vave → vfi = length
        let c: Vec<f64> = (0..40).map(|i| 100. * 1.01f64.powi(i)).collect();
        let o = go("vfi", &cv(&c, &[1000.; 40]), &[("length", 5.), ("smooth", 1.), ("signal", 1.)]);
        assert_series(&o[0][34..], &[5.; 6]);
        assert!(o[0][..34].iter().all(|v| v.is_nan()));
    }
    #[test] fn volume_ratio_and_zscore() {
        assert_series(&go("volume_ratio_jp", &cv(&[10., 11., 10., 11., 10.], &[7.; 5]), &[("length", 4.)])[0], &[N, N, N, N, 100.]);
        assert_series(&go("volume_ratio_jp", &cv(&[10., 11., 11., 10.], &[1., 4., 2., 3.]), &[("length", 3.)])[0], &[N, N, N, 100. * 5. / 4.]);
        let o = go("volume_zscore", &cv(&[1.; 4], &[1., 3., 2., 2.]), &[("length", 3.), ("spike", 0.5)]);
        assert_series(&o[0], &[N, N, 0., -(0.5f64).sqrt()]);
        assert_series(&o[1], &[N, N, 0., 0.]);
        assert_series(&go("volume_zscore", &cv(&[1.; 3], &[4.; 3]), &[("length", 2.)])[0], &[N, 0., 0.]);
    }
    #[test] fn vpn_vroc_vzo() {
        let c: Vec<f64> = (0..8).map(|i| 100. + 10. * i as f64).collect();
        assert_series(&go("vpn", &cv(&c, &[5.; 8]), &[("length", 3.), ("smooth", 1.)])[0], &[N, N, N, N, 100., 100., 100., 100.]);
        let v = cv(&[1.; 5], &[1., 1., 2., 2., 0.]);
        assert_series(&go("vroc", &v, &[("length", 2.)])[0], &[N, N, 100., 100., -100.]);
        assert_series(&go("vroc", &v, &[("length", 2.), ("smooth", 2.)])[0], &[N, N, N, 100., 0.]);
        assert_series(&go("vroc", &cv(&[1.; 3], &[0., 1., 1.]), &[("length", 1.)])[0], &[N, N, 0.]);
        assert_series(&go("vzo", &cv(&[1., 2., 3., 4., 5.], &[3., 1., 4., 1., 5.]), &[("length", 3.)])[0][3..], &[100., 100.]);
    }
    #[test] fn williams_ad() {
        let b = from_ohlc(&[(10., 10., 10., 10.), (10., 11.5, 9.5, 11.), (11., 12., 10.5, 11.), (11., 11.2, 10., 10.5)]);
        // bar 1 trl = 9.5 → 1.5; bar 2 unchanged → 0 (variant 1: 11 − 12); bar 3 trh = 11.2 → −0.7
        assert_series(&go("wad", &b, &[])[0], &[0., 1.5, 1.5, 0.8]);
        assert_series(&go("wad", &b, &[("unchanged", 1.)])[0], &[0., 1.5, 0.5, -0.2]);
    }
    #[test] fn breadth_thrust_stix_mcclellan() {
        let b = ab(&[300.; 50], &[300.; 50]);
        assert_series(&go("breadth_thrust", &b, &[])[0][9..], &[0.5; 41]);
        assert_series(&go("breadth_thrust", &b, &[("ma_type", 1.)])[0][9..], &[0.5; 41]);
        assert_series(&go("stix", &b, &[])[0][20..], &[50.; 30]);
        let c = ab(&[300.; 50], &[100.; 50]);
        for ra in [0., 1.] {
            let o = go("mcclellan", &c, &[("ratio_adjusted", ra), ("start", 1000.)]);
            assert!(o[0][..38].iter().all(|v| v.is_nan()));
            assert_series(&o[0][38..], &[0.; 12]);
            assert_series(&o[1][38..], &[1000.; 12]);
        }
        let g = ab(&[100., N, 300., 200.], &[100., 50., 100., 200.]);
        let o = go("mcclellan", &g, &[("fast", 1.), ("slow", 2.)]);
        // net 0, NaN, 200, 0: ema(net, 2) skips the gap: seed 100 at bar 2, then 100 + (2/3)(0 − 100)
        assert_series(&o[0], &[N, N, 100., -100. / 3.]);
        assert_series(&o[1], &[N, N, 100., 200. / 3.]);
        assert_series(&go("stix", &g, &[("length", 1.)])[0], &[50., N, 75., 50.]);
    }
    #[test] fn breadth_items_are_causal_with_exact_warmup_and_reachable() {
        let full = with_breadth(walk(400, 11, true));
        for id in ["breadth_thrust", "mcclellan", "stix"] {
            let d = get(id).unwrap();
            let want = run(&d, &full, &Params::new()).unwrap();
            for (k, s) in want.iter().enumerate() {
                assert_eq!(s.iter().position(|v| !v.is_nan()), Some(d.first[k]), "{id}.{k} first");
                assert!(s[d.first[k]..].iter().all(|v| v.is_finite()), "{id}.{k} gap");
            }
            for c in [60usize, 151, 399] {
                let got = run(&d, &cut(&full, c), &Params::new()).unwrap();
                for (k, s) in got.iter().enumerate() { for i in 0..c { assert!(close_eq(s[i], want[k][i]), "{id}.{k} bar {i}"); } }
            }
            compiles(id, d.outputs[0]).unwrap();
        }
        assert_eq!(run(&get("stix").unwrap(), &closes(&[1., 2.]), &Params::new()).unwrap_err(), "needs breadth");
    }
    #[test] fn parameter_bounds_never_panic() {
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
    #[test] fn rules_reach_volume_items() {
        for d in super::DEFS.iter().filter(|d| d.category == "Volume") { compiles(d.id, d.outputs[d.outputs.len() - 1]).unwrap(); }
    }
}
