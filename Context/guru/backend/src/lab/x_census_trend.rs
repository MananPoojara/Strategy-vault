//! Census group "trend" (2026-10-26, research: sv-census-work/research_trend.md). Formulas: see each definition string.
//! Skipped as UNCONFIRMED (no published formula, nothing invented): Chande Trend Meter (StockCharts; weights and scoring
//! not published), Traffic Light (StockCharts; a user-level shading tool, compared series and boundary rules not stated).
use super::bars::Bars;
use super::catalogue::{get, int, len, n, p, real, run, IndicatorDef, Needs, Output, ParamSpec, Params, Placement};
use super::ta::{self, nz, NA};

#[allow(clippy::too_many_arguments)]
const fn def(id: &'static str, name: &'static str, category: &'static str, params: &'static [ParamSpec], outputs: &'static [&'static str], placement: Placement,
             needs: Needs, definition: &'static str, first: &'static [usize], f: fn(&Bars, &Params) -> Output) -> IndicatorDef {
    IndicatorDef { id, name, category, params, outputs, placement, needs, definition, first, sparse: false, compute: Some(f), note: "" }
}
const MA: &str = "Moving averages";
const TR: &str = "Trend";
const PR: Placement = Placement::Price;
const PN: Placement = Placement::Pane;
const NO: Needs = Needs::NONE;

fn call(id: &str, b: &Bars, kv: &[(&str, f64)]) -> Result<Vec<Vec<f64>>, String> {
    run(&get(id).ok_or_else(|| format!("{id} missing"))?, b, &kv.iter().map(|(k, v)| (k.to_string(), *v)).collect())
}
/// Sierra Chart SimpleMovAvg: the length shrinks to the bars available while warming up.
fn sc_sma(x: &[f64], k: usize) -> Vec<f64> {
    (0..x.len()).map(|i| { let w = &x[(i + 1).saturating_sub(k)..=i]; w.iter().sum::<f64>() / w.len() as f64 }).collect()
}

const MV: &[ParamSpec] = &[len("min_period", "Minimum period", 2.), len("max_period", "Maximum period", 30.), int("ma_type", "Average type (TA-Lib MAType 0 SMA, 1 EMA, 2 WMA)", 0., 0., 2.)];
fn mavp(b: &Bars, prm: &Params) -> Output {
    let per = super::pairs::second(b)?;
    let (lo, hi, t, c) = (n(prm, "min_period"), n(prm, "max_period"), n(prm, "ma_type"), &b.close);
    if lo > hi { return Err("mavp: minimum period must not exceed maximum period".into()); }
    let mut out = vec![NA; c.len()];
    let s = hi - 1; // TA-Lib lookback of the maximum period
    let mut cache = std::collections::BTreeMap::new();
    for i in s..c.len() {
        let v = per[i];
        let k = if !(v >= lo as f64) { lo } else if v > hi as f64 { hi } else { v as usize };
        let ma = cache.entry(k).or_insert_with(|| match t {
            // TA-Lib starts every period's EMA at the max-period lookback: SMA seed over the k bars ending there
            1 => { let mut e = vec![NA; s + 1 - k]; e.extend(ta::ema(&c[s + 1 - k..], k)); e }
            2 => ta::wma(c, k),
            _ => ta::sma(c, k),
        });
        out[i] = ma[i];
    }
    Ok(vec![out])
}

const JM: &[ParamSpec] = &[int("length", "Length", 7., 2., 2000.), real("phase", "Phase", 0., -100., 100., 1.)];
fn jma(b: &Bars, prm: &Params) -> Output {
    let (k, c) = (n(prm, "length"), &b.close);
    let m = c.len();
    let mut out = vec![NA; m];
    if m == 0 { return Ok(vec![out]); }
    let (half, pr) = (0.5 * (k as f64 - 1.), 1.5 + 0.01 * p(prm, "phase"));
    let len1 = (half.sqrt().ln() / 2f64.ln() + 2.).max(0.);
    let pow1 = (len1 - 2.).max(0.5);
    let len2 = len1 * half.sqrt();
    let (bet, beta) = (len2 / (len2 + 1.), 0.45 * (k as f64 - 1.) / (0.45 * (k as f64 - 1.) + 2.));
    let (mut volty, mut vsum) = (vec![0.; m], vec![0.; m]);
    let (mut ma1, mut ub, mut lb, mut det0, mut det1, mut j) = (c[0], c[0], c[0], 0., 0., c[0]);
    for i in 1..m {
        let x = c[i];
        let (d1, d2) = (x - ub, x - lb);
        volty[i] = if d1.abs() != d2.abs() { d1.abs().max(d2.abs()) } else { 0. };
        vsum[i] = vsum[i - 1] + (volty[i] - volty[i.saturating_sub(10)]) / 10.;
        let w = &vsum[i.saturating_sub(65)..=i];
        let avg = w.iter().sum::<f64>() / w.len() as f64;
        let rv = len1.powf(1. / pow1).min(if avg == 0. { 0. } else { volty[i] / avg }).max(1.);
        let pw = rv.powf(pow1);
        let kv = bet.powf(pw.sqrt());
        ub = if d1 > 0. { x } else { x - kv * d1 };
        lb = if d2 < 0. { x } else { x - kv * d2 };
        let alpha = beta.powf(pw);
        ma1 = (1. - alpha) * x + alpha * ma1;
        det0 = (x - ma1) * (1. - beta) + beta * det0;
        det1 = (ma1 + pr * det0 - j) * (1. - alpha) * (1. - alpha) + alpha * alpha * det1;
        j += det1;
        if i + 1 >= k { out[i] = j; }
    }
    Ok(vec![out])
}

const GD: &[ParamSpec] = &[len("length", "Length", 5.), real("vfactor", "Volume factor", 0.7, 0., 1., 0.01)];
fn gdema(b: &Bars, prm: &Params) -> Output {
    let (k, v) = (n(prm, "length"), p(prm, "vfactor"));
    let e1 = ta::ema(&b.close, k);
    let e2 = ta::ema(&e1, k);
    Ok(vec![e1.iter().zip(&e2).map(|(a, c)| a * (1. + v) - c * v).collect()])
}

const BL: &[ParamSpec] = &[len("atr_length", "ATR length", 10.), real("mult1", "Box multiplier 1", 1., 0.01, 100., 0.01), real("mult2", "Box multiplier 2", 2., 0.01, 100., 0.01)];
fn block_ma(b: &Bars, prm: &Params) -> Output {
    let (k, m) = (n(prm, "atr_length"), b.len());
    let atr = sc_sma(&ta::tr(b, true), k);
    let mut out = vec![vec![NA; m]; 4];
    for (j, key) in ["mult1", "mult2"].iter().enumerate() {
        let v = p(prm, key);
        let (mut top, mut mid, mut bot, mut dir) = (0., 0., 0., 0.); // Sierra arrays start at zero, so bar 0 opens an up box
        for i in 0..m {
            let h = 0.5 * v * atr[i];
            if b.high[i] > top { dir = 1.; top = b.high[i]; mid = top - h; bot = top - 2. * h; }
            else if b.low[i] < bot { dir = -1.; bot = b.low[i]; mid = bot + h; top = bot + 2. * h; }
            if i + 1 >= k { out[j][i] = mid; out[j + 2][i] = dir; }
        }
    }
    Ok(out)
}

const L14: &[ParamSpec] = &[len("length", "Length", 14.)];
fn erma(b: &Bars, prm: &Params) -> Output {
    let lnx: Vec<f64> = b.close.iter().map(|&v| if v > 0. { v.ln() } else { NA }).collect();
    Ok(vec![ta::linreg(&lnx, n(prm, "length"), 0).iter().map(|v| v.exp()).collect()])
}

const TP: &[ParamSpec] = &[int("days", "Time period length (days)", 1., 1., 3650.)];
fn time_period_ma(b: &Bars, prm: &Params) -> Output {
    let (span, c, m) = (p(prm, "days") as i64, &b.close, b.len());
    let d: Vec<i64> = b.date.iter().map(|s| super::resample::day_number(s)).collect();
    let mut out = vec![NA; m];
    let mut s = 0;
    for t in 0..m {
        let target = d[t] - span;
        // nearest bar to date − span; a tie moves to the later bar (Sierra breaks only on a strictly larger difference)
        while s < t && (d[s + 1] - target).abs() <= (d[s] - target).abs() { s += 1; }
        if d[0] + span > d[t] { continue; }
        let (sum, cnt) = c[s..=t].iter().filter(|v| **v != 0.).fold((0., 0.), |(a, k), v| (a + v, k + 1.));
        out[t] = if cnt == 0. { 0. } else { sum / cnt };
    }
    Ok(vec![out])
}

fn continuous_avg(b: &Bars, _: &Params) -> Output {
    let mut sum = 0.;
    Ok(vec![b.close.iter().enumerate().map(|(i, v)| { sum += v; if i == 0 { NA } else { sum / (i + 1) as f64 } }).collect()])
}

fn arsi(b: &Bars, prm: &Params) -> Output {
    let r = call("rsi", b, &[("length", p(prm, "length")), ("variant", 0.)])?.swap_remove(0);
    let mut prev = NA;
    Ok(vec![r.iter().zip(&b.close).map(|(r, c)| { let a = 2. * (r / 100. - 0.5).abs(); prev = a * c + (1. - a) * nz(prev); prev }).collect()])
}

const MD: &[ParamSpec] = &[int("exponential", "Exponential MA (0 SMA, 1 EMA)", 1., 0., 1.)];
const MD_LEN: [usize; 18] = [5, 10, 15, 20, 25, 30, 35, 40, 45, 50, 55, 60, 65, 70, 75, 80, 85, 90];
fn madrid_ribbon(b: &Bars, prm: &Params) -> Output {
    let ma = |k: usize| if p(prm, "exponential") == 1. { ta::ema(&b.close, k) } else { ta::sma(&b.close, k) };
    let r = ma(100);
    let lines: Vec<Vec<f64>> = MD_LEN.iter().map(|&k| ma(k)).collect();
    let states: Vec<Vec<f64>> = lines.iter().map(|x| (0..x.len()).map(|i| {
        let d = if i == 0 { NA } else { x[i] - x[i - 1] };
        if d.is_nan() || r[i].is_nan() { NA }
        else if d >= 0. && x[i] > r[i] { 2. } else if d < 0. && x[i] > r[i] { -1. } else if d <= 0. && x[i] < r[i] { -2. } else if d >= 0. && x[i] < r[i] { 1. } else { 0. }
    }).collect()).collect();
    Ok(lines.into_iter().chain(states).collect())
}

fn plus_minus_dm(b: &Bars, prm: &Params) -> Output {
    let (k, m, h, l) = (n(prm, "length"), b.len(), &b.high, &b.low);
    let dm1 = |i: usize, plus: bool| { let (up, dn) = (h[i] - h[i - 1], l[i - 1] - l[i]); let (a, o) = if plus { (up, dn) } else { (dn, up) }; if a > 0. && a > o { a } else { 0. } };
    Ok([true, false].iter().map(|&plus| {
        let mut out = vec![NA; m];
        if k == 1 { for i in 1..m { out[i] = dm1(i, plus); } }
        else if m >= k {
            // Wilder sum seeded with the n − 1 first one-bar values (TA-Lib), not an average
            let mut s: f64 = (1..k).map(|i| dm1(i, plus)).sum();
            out[k - 1] = s;
            for i in k..m { s = s - s / k as f64 + dm1(i, plus); out[i] = s; }
        }
        out
    }).collect())
}

const SX: &[ParamSpec] = &[real("start", "Start value (0 auto, > 0 long at it, < 0 short at |it|)", 0., -1e7, 1e7, 0.01), real("offset", "Offset on reverse", 0., 0., 1., 0.001),
    real("init_long", "Long initial AF", 0.02, 0., 1., 0.001), real("step_long", "Long AF step", 0.02, 0., 1., 0.001), real("max_long", "Long AF maximum", 0.2, 0., 1., 0.001),
    real("init_short", "Short initial AF", 0.02, 0., 1., 0.001), real("step_short", "Short AF step", 0.02, 0., 1., 0.001), real("max_short", "Short AF maximum", 0.2, 0., 1., 0.001)];
fn sarext(b: &Bars, prm: &Params) -> Output {
    let (m, h, l, g) = (b.len(), &b.high, &b.low, |k: &str| p(prm, k));
    let (mut out, mut dir) = (vec![NA; m], vec![NA; m]);
    if m < 2 { return Ok(vec![out, dir]); }
    let (mxl, mxs, sv, off) = (g("max_long"), g("max_short"), g("start"), g("offset"));
    let (il, sl, is, ss) = (g("init_long").min(mxl), g("step_long").min(mxl), g("init_short").min(mxs), g("step_short").min(mxs));
    let mut long = if sv == 0. { let dn = l[0] - l[1]; !(dn > 0. && dn > h[1] - h[0]) } else { sv > 0. };
    let (mut ep, mut sar) = match (sv == 0., long) { (true, true) => (h[1], l[0]), (true, false) => (l[1], h[0]), (false, true) => (h[1], sv), _ => (l[1], sv.abs()) };
    let (mut afl, mut afs, mut ph, mut pl) = (il, is, h[1], l[1]); // TA-Lib uses bar 1 as its own previous bar on the first step
    for i in 1..m {
        let (nh, nl) = (h[i], l[i]);
        if long {
            if nl <= sar {
                long = false;
                sar = ep.max(ph).max(nh);
                if off != 0. { sar += sar * off; }
                out[i] = sar; dir[i] = -1.;
                afs = is; ep = nl;
                sar = afs.mul_add(ep - sar, sar).max(ph).max(nh);
            } else {
                out[i] = sar; dir[i] = 1.;
                if nh > ep { ep = nh; afl = (afl + sl).min(mxl); }
                sar = afl.mul_add(ep - sar, sar).min(pl).min(nl);
            }
        } else if nh >= sar {
            long = true;
            sar = ep.min(pl).min(nl);
            if off != 0. { sar -= sar * off; }
            out[i] = sar; dir[i] = 1.;
            afl = il; ep = nh;
            sar = afl.mul_add(ep - sar, sar).min(pl).min(nl);
        } else {
            out[i] = sar; dir[i] = -1.;
            if nl < ep { ep = nl; afs = (afs + ss).min(mxs); }
            sar = afs.mul_add(ep - sar, sar).max(ph).max(nh);
        }
        ph = nh; pl = nl;
    }
    Ok(vec![out, dir])
}

const STS: &[ParamSpec] = &[real("mult", "ATR multiplier", 2., 0.01, 100., 0.01), len("atr_length", "ATR period", 3.), len("median_length", "Median period", 3.)];
fn super_trend_stop(b: &Bars, prm: &Params) -> Output {
    let (v, c, m) = (p(prm, "mult"), &b.close, b.len());
    let atr = sc_sma(&ta::tr(b, true), n(prm, "atr_length"));
    let (hl, km) = (b.hl2(), n(prm, "median_length"));
    let med: Vec<f64> = (0..m).map(|i| {
        let mut w = hl[(i + 1).saturating_sub(km)..=i].to_vec();
        w.sort_by(f64::total_cmp);
        let k = w.len();
        if k % 2 == 1 { w[k / 2] } else { (w[k / 2 - 1] + w[k / 2]) / 2. }
    }).collect();
    let (mut stop, mut trend) = (vec![NA; m], vec![NA; m]);
    if m == 0 { return Ok(vec![stop, trend]); }
    stop[0] = c[0]; trend[0] = 1.;
    for i in 1..m {
        let (s1, t1) = (stop[i - 1], trend[i - 1]);
        (stop[i], trend[i]) = if c[i] > s1 { let ns = med[i] - v * atr[i - 1]; (if t1 < 0. { ns } else { ns.max(s1) }, 1.) }
            else if c[i] < s1 { let ns = med[i] + v * atr[i - 1]; (if t1 > 0. { ns } else { ns.min(s1) }, -1.) }
            else { (s1, t1) };
    }
    Ok(vec![stop, trend])
}

const KW: &[ParamSpec] = &[len("atr_length", "ATR length", 9.), real("factor", "ATR factor", 2.5, 0.1, 20., 0.1), int("vervoort", "Vervoort variation (0 off, 1 on)", 0., 0., 1.), real("vervoort_max", "Vervoort maximum range", 1.5, 0.1, 10., 0.1)];
fn kiwis_trailing_stop(b: &Bars, prm: &Params) -> Output {
    let (k, f, m, h, l, c) = (n(prm, "atr_length"), p(prm, "factor"), b.len(), &b.high, &b.low, &b.close);
    let (mut stop, mut dir) = (vec![NA; m], vec![NA; m]);
    if m == 0 { return Ok(vec![stop, dir]); }
    let mut ave = vec![NA; m];
    if p(prm, "vervoort") == 0. {
        // Tillson T3 (b = 0.5) of the true range, all six EMAs seeded with H − L of bar 0
        let (bb, w) = (0.5f64, 2. / (k as f64 + 1.));
        let (c1, c2, c3, c4) = (-bb.powi(3), 3. * (bb * bb + bb.powi(3)), -3. * (2. * bb * bb + bb + bb.powi(3)), 1. + 3. * bb + bb.powi(3) + 3. * bb * bb);
        let mut e = [h[0] - l[0]; 6];
        for i in 1..m {
            let mut x = (h[i] - l[i]).max(h[i] - c[i - 1]).max(c[i - 1] - l[i]);
            for ej in e.iter_mut() { *ej = w * x + (1. - w) * *ej; x = *ej; }
            ave[i] = c1 * e[5] + c2 * e[4] + c3 * e[3] + c4 * e[2];
        }
    } else {
        let hl: Vec<f64> = (0..m).map(|i| h[i] - l[i]).collect();
        let cap = sc_sma(&hl, k);
        let vm = p(prm, "vervoort_max");
        let mut diff = vec![hl[0]; m]; // Sierra shares this subgraph with an array seeded H − L at bar 0
        let mut a = hl[0];
        for i in 1..m {
            let href = if l[i] <= h[i - 1] { h[i] - c[i - 1] } else { (h[i] - c[i - 1]) - (l[i] - h[i - 1]) / 2. };
            let lref = if h[i] >= l[i - 1] { c[i - 1] - l[i] } else { (c[i - 1] - l[i]) - (l[i - 1] - h[i]) / 2. };
            diff[i] = lref.max(hl[i].min(vm * cap[i]).max(href));
            // Sierra Wilders: a zero previous value is re-seeded with the zero-skipping mean of the last n inputs
            if a == 0. { let (s, cnt) = diff[i.saturating_sub(k)..i].iter().filter(|v| **v != 0.).fold((0., 0.), |(s, c), v| (s + v, c + 1.)); a = if cnt > 0. { s / cnt } else { 0. }; }
            if a != 0. { a += (diff[i] - a) / k as f64; }
            ave[i] = a;
        }
    }
    let (mut d, mut lo, mut hi) = (1., c[0], c[0]);
    for i in 1..m {
        let s = f * ave[i];
        if d == 1. && c[i] < lo { d = -1.; hi = c[i] + s; lo = 0.; }
        else if d == -1. && c[i] > hi { d = 1.; lo = c[i] - s; hi = 0.; }
        else if d == 1. { if c[i] - s > lo { lo = c[i] - s; } }
        else if c[i] + s < hi { hi = c[i] + s; }
        stop[i] = if d == 1. { lo } else { hi };
        dir[i] = d;
    }
    Ok(vec![stop, dir])
}

const PM: &[ParamSpec] = &[len("atr_length", "ATR length", 10.), real("mult", "ATR multiplier", 3., 0.1, 20., 0.1),
    int("ma_type", "MA type (0 SMA, 1 EMA, 2 WMA, 3 TMA, 4 VAR, 5 WWMA, 6 ZLEMA, 7 TSF)", 1., 0., 7.), len("ma_length", "MA length", 10.),
    int("change_atr", "ATR method (0 sma(tr), 1 Wilder atr)", 1., 0., 1.), int("normalize", "Normalize ATR by close (0/1)", 0., 0., 1.)];
fn pmax(b: &Bars, prm: &Params) -> Output {
    let (ka, mult, k, m) = (n(prm, "atr_length"), p(prm, "mult"), n(prm, "ma_length"), b.len());
    let src = b.hl2();
    let atr = if p(prm, "change_atr") == 1. { ta::rma(&ta::tr(b, true), ka) } else { ta::sma(&ta::tr(b, false), ka) };
    let mavg = match n(prm, "ma_type") {
        0 => ta::sma(&src, k), 1 => ta::ema(&src, k), 2 => ta::wma(&src, k),
        3 => ta::sma(&ta::sma(&src, (k as f64 / 2.).ceil() as usize), k / 2 + 1),
        4 => super::x_trend::ott_s(&src, k, 0.).swap_remove(1),
        5 => { let mut v = 0.; src.iter().map(|x| { v = x / k as f64 + (1. - 1. / k as f64) * v; v }).collect() }
        6 => { let lag = k / 2; ta::ema(&(0..m).map(|i| if i >= lag { 2. * src[i] - src[i - lag] } else { NA }).collect::<Vec<_>>(), k) }
        _ => { let (a, a1) = (ta::linreg(&src, k, 0), ta::linreg(&src, k, 1)); (0..m).map(|i| 2. * a[i] - a1[i]).collect() }
    };
    let norm = p(prm, "normalize") == 1.;
    let (mut pm, mut dir) = (vec![NA; m], vec![NA; m]);
    let (mut ls1, mut ss1, mut d) = (NA, NA, 1.);
    for i in 0..m {
        let a = if norm { mult * atr[i] / b.close[i] } else { mult * atr[i] };
        let (mut ls, mut ss) = (mavg[i] - a, mavg[i] + a);
        let (lp, sp) = (if ls1.is_nan() { ls } else { ls1 }, if ss1.is_nan() { ss } else { ss1 }); // nz(x[1], x)
        if mavg[i] > lp { ls = ls.max(lp); }
        if mavg[i] < sp { ss = ss.min(sp); }
        d = if d == -1. && mavg[i] > sp { 1. } else if d == 1. && mavg[i] < lp { -1. } else { d };
        ls1 = ls; ss1 = ss;
        if !ls.is_nan() { pm[i] = if d == 1. { ls } else { ss }; dir[i] = d; }
    }
    Ok(vec![pm, mavg, dir])
}

fn gann_swing_osc(b: &Bars, _: &Params) -> Output {
    let g = call("gann_trend_osc", b, &[])?.swap_remove(0); // +1 higher high and low, −1 lower high and low
    let (h, l) = (&b.high, &b.low);
    let mut v = 0.;
    Ok(vec![(0..b.len()).map(|i| {
        if i >= 2 {
            let s = i - 2; // swing bar s: down bar then up bar (or the reverse), confirmed by a third bar
            if g[s] == -1. && g[s + 1] == 1. && h[s + 1] < h[i] { v = 1.; }
            else if g[s] == 1. && g[s + 1] == -1. && l[s + 1] > l[i] { v = -1.; }
        }
        v
    }).collect()])
}

const SW: &[ParamSpec] = &[len("lsma_length", "LSMA length", 25.), len("ema_length", "EMA length", 34.), real("trend_threshold", "Trending threshold", 100., 0., 360., 1.),
    real("normal_threshold", "Normal threshold", 30., 0., 360., 1.), real("tick", "Tick size (Sierra Chart uses the chart's tick)", 0.05, 0.0001, 1000., 0.0001)];
fn sidewinder(b: &Bars, prm: &Params) -> Output {
    let (ke, tick, tt, nt, c, m) = (n(prm, "ema_length"), p(prm, "tick"), p(prm, "trend_threshold"), p(prm, "normal_threshold"), &b.close, b.len());
    let lsma = ta::linreg(c, n(prm, "lsma_length"), 0);
    // Sierra ExponentialMovAvg as the study calls it from bar 2: bar 0 stays 0, bar 1 = close, alpha 2/(min(n, i + 1) + 1)
    let mut ema = vec![0.; m];
    if m > 1 { ema[1] = c[1]; }
    for i in 2..m { let a = 2. / (ke.min(i + 1) as f64 + 1.); ema[i] = a * c[i] + (1. - a) * ema[i - 1]; }
    let ang = |x: &[f64], i: usize| ((x[i] - (x[i - 1] + x[i - 2]) / 2.) / tick).atan().to_degrees();
    Ok(vec![(0..m).map(|i| {
        if i < 2 { return NA; }
        let (la, ea) = (ang(&lsma, i), ang(&ema, i));
        if la.is_nan() { return NA; }
        let s = ea + la;
        if ea.abs() >= 15. && s >= tt { 2. } else if ea.abs() >= 15. && s <= -tt { -2. }
        else if ea > 5. && la > 0. && s >= nt { 1. } else if ea < 5. && la < 0. && s <= -nt { -1. } else { 0. }
    }).collect()])
}

const MD_OUT: &[&str] = &["ma5", "ma10", "ma15", "ma20", "ma25", "ma30", "ma35", "ma40", "ma45", "ma50", "ma55", "ma60", "ma65", "ma70", "ma75", "ma80", "ma85", "ma90",
    "state5", "state10", "state15", "state20", "state25", "state30", "state35", "state40", "state45", "state50", "state55", "state60", "state65", "state70", "state75", "state80", "state85", "state90"];
const MD_FIRST: &[usize] = &[4, 9, 14, 19, 24, 29, 34, 39, 44, 49, 54, 59, 64, 69, 74, 79, 84, 89, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99];
pub const DEFS: &[IndicatorDef] = &[
    def("mavp", "Moving Average with Variable Period (TA-Lib MAVP)", MA, MV, &["mavp"], PR, Needs { second_symbol: true, ..Needs::NONE }, "TA-Lib MAVP(close, periods, minperiod, maxperiod, matype); periods = the aligned close of the second instrument (vs); per bar k = periods < min (or no value) ? min : periods > max ? max : trunc(periods), at least 1; mavp = MA_k(close) at that bar, every period's MA running from the max-period lookback (EMA: SMA seed over the k bars ending at bar max − 1, as TA-Lib); output from bar max − 1 (TA-Lib lookback); defaults min 2, max 30, SMA; DEMA/TEMA/TRIMA/KAMA/MAMA/T3 types not offered", &[29], mavp),
    def("jma", "Jurik Moving Average (pandas-ta reconstruction)", MA, JM, &["jma"], PR, NO, "pandas-ta jma (the true Jurik Research JMA is proprietary): len = (L − 1)/2, pr = 1.5 + phase/100, len1 = max(log2(√len) + 2, 0), pow1 = max(len1 − 2, 0.5), bet = len1·√len/(len1·√len + 1), beta = 0.45(L − 1)/(0.45(L − 1) + 2); from bar 0 (jma = ma1 = bands = close): volty = max(|close − upper|, |close − lower|) (0 when equal), vsum += (volty − volty[10])/10, avg = mean of the last 66 vsum, rv = max(1, min(len1^(1/pow1), volty/avg)), kv = bet^√(rv^pow1), bands pulled to close by kv, alpha = beta^(rv^pow1); ma1 = (1 − alpha)·close + alpha·ma1; det0 = (close − ma1)(1 − beta) + beta·det0; det1 = (ma1 + pr·det0 − jma[1])(1 − alpha)² + alpha²·det1; jma = jma[1] + det1; first L − 1 bars blank; defaults L 7, phase 0", &[6], jma),
    def("gdema", "Generalized DEMA (Tillson GD)", MA, GD, &["gd"], PR, NO, "Tillson (TASC Jan 1998), as in TA-Lib's T3 header: gd = ema(close, n)·(1 + v) − ema(ema(close, n), n)·v (SMA-seeded EMAs as TA-Lib); v 1 = DEMA, v 0 = EMA; first bar 2(n − 1); defaults n 5, v 0.7", &[8], gdema),
    def("block_ma", "Moving Average - Block (Sierra Chart)", MA, BL, &["mid1", "mid2", "dir1", "dir2"], PR, NO, "Sierra Chart Moving Average - Block (ACSIL source): atr = sma(tr, n) with Sierra's shrinking warm-up length (tr bar 0 = H − L); per box i, half = 0.5·mult_i·atr; H > top[1] → dir 1, top = H, mid = top − half, bottom = top − 2·half; else L < bottom[1] → dir −1, bottom = L, mid = bottom + half, top = bottom + 2·half; else all carried (box state starts at 0 on bar 0, so bar 0 opens an up box); shown from bar n − 1 (Sierra DataStartIndex); defaults n 10 (simple ATR), mult 1 and 2", &[9, 9, 9, 9], block_ma),
    def("erma", "Moving Average - Exponential Regression (Sierra Chart)", MA, L14, &["erma"], PR, NO, "Sierra Chart Moving Average - Exponential Regression: least-squares fit of ln(close) on x = 1..n over the last n bars, erma = exp(fitted value at the newest bar) = exp(linreg(ln close, n, 0)); a close ≤ 0 in the window gives no value; length 14 is the lab's default (Sierra's default is not documented)", &[13], erma),
    def("time_period_ma", "Moving Average - Time Period (Sierra Chart)", MA, TP, &["avg"], PR, NO, "Sierra Chart Moving Average - Time Period (ACSIL source), days unit: window = from the bar whose date is nearest to date − days (a tie takes the later bar) to this bar; avg = mean of the non-zero closes in it (0 if none); blank until date − days reaches the first bar (lab choice, so early windows are not short); the bar count varies with weekends and holidays; default 1 day", &[1], time_period_ma),
    def("continuous_avg", "Average - Continuous (Sierra Chart)", MA, &[], &["avg"], PR, NO, "Sierra Chart Average - Continuous (ACSIL source), no daily reset: avg = Σ_{0..t} close/(t + 1), the expanding mean; written from bar 1 as Sierra (bar 0 is never written)", &[1], continuous_avg),
    def("arsi", "Adaptive RSI Moving Average (everget)", MA, L14, &["arsi"], PR, NO, "everget Pine Adaptive RSI: alpha = 2·|rsi(close, n)/100 − 0.5| (TradingView RSI); arsi = alpha·close + (1 − alpha)·nz(arsi[1]); the nz seed is 0, so the line ramps up from 0 and needs a long warm-up (about 10·n bars) to settle, as in Pine; default n 14", &[14], arsi),
    def("madrid_ribbon", "Madrid Moving Average Ribbon", MA, MD, MD_OUT, PR, NO, "Madrid Pine (2014, v4 2021): ma_k = ema(close, k) (sma if exponential 0) for k = 5, 10, …, 90, reference ma100; state_k with d = ma_k − ma_k[1], tested in this order: d ≥ 0 and ma_k > ma100 → 2 (lime, uptrend); d < 0 and ma_k > ma100 → −1 (maroon, uptrend reversal warning); d ≤ 0 and ma_k < ma100 → −2 (red, downtrend); d ≥ 0 and ma_k < ma100 → 1 (green, downtrend reversal warning); else 0 (gray); states from bar 99 (ma100)", MD_FIRST, madrid_ribbon),
    def("plus_minus_dm", "Directional Movement +DM / −DM (TA-Lib)", TR, L14, &["plus_dm", "minus_dm"], PN, NO, "TA-Lib PLUS_DM / MINUS_DM (Wilder 1978): up = H − H[1], dn = L[1] − L; +DM1 = up if up > 0 and up > dn else 0, −DM1 = dn if dn > 0 and dn > up else 0 (tie → both 0); n 1: the one-bar values from bar 1; n > 1: Wilder sum, seed Σ of the first n − 1 one-bar values at bar n − 1, then s = s − s/n + DM1 (a sum, not our dmi's average); default n 14", &[13, 13], plus_minus_dm),
    def("sarext", "Parabolic SAR - Extended (TA-Lib SAREXT)", TR, SX, &["sar", "direction"], PR, NO, "TA-Lib SAREXT: start 0 → short if −DM of bar 1 > 0 (L[0] − L[1] > 0 and > H[1] − H[0]), else long; long seed ep = H[1], sar = L[0]; short seed ep = L[1], sar = H[0]; start > 0 long at start, < 0 short at |start|; AF init/step above their maximum are cut to it; from bar 1: long and L ≤ sar → reverse, sar = max(ep, H[1], H)·(1 + offset), af = init short, ep = L, next sar = max(sar + af(ep − sar), H[1], H); long otherwise: new high → ep = H, af = min(af + step, max); next sar = min(sar + af(ep − sar), L[1], L); short mirrors (H ≥ sar reverses, ·(1 − offset)); on bar 1 the previous bar is bar 1 itself (TA-Lib); sar = level as a price, direction +1 long / −1 short (TA-Lib's output = direction·sar); penetration is non-strict, unlike our TradingView sar; defaults start 0, offset 0, AF 0.02/0.02/0.2 both sides", &[1, 1], sarext),
    def("super_trend_stop", "Super Trend Stop (Sierra Chart)", TR, STS, &["stop", "trend"], PR, NO, "Sierra Chart SuperTrend Stop (2021 ACSIL source): median = moving median of hl2 (length shrinks while warming up; even lengths average the two middle values, where Sierra's partial sort may pair another element); atr = sma(tr, n) with Sierra's shrinking warm-up (tr bar 0 = H − L); bar 0: stop = close, trend 1; close > stop[1] → trend 1, new = median − mult·atr[1], stop = trend[1] < 0 ? new : max(new, stop[1]); close < stop[1] → trend −1, new = median + mult·atr[1], stop = trend[1] > 0 ? new : min(new, stop[1]); equal → carried (exact compare; Sierra compares at the price format); the current Sierra documentation adds an ATR average type (Hull) not in this source; defaults mult 2, ATR 3, median 3", &[0, 0], super_trend_stop),
    def("kiwis_trailing_stop", "Kiwi's Trailing Stop (Sierra Chart)", TR, KW, &["stop", "direction"], PR, NO, "Sierra Chart Kiwi's Trailing Stop (2021 ACSIL source): vervoort 0 → ave = Tillson T3 (b 0.5, w = 2/(n + 1)) of tr = max(H − L, H − C[1], C[1] − L), the six EMAs seeded with H − L of bar 0 (about 6n bars to settle); vervoort 1 → ave = Wilders(max(lref, min(H − L, max_range·sma(H − L, n)), href)) with Vervoort's href/lref, seeded with H − L of bar 0; s = factor·ave; bar 0 long with loclose = hiclose = close; long and close < loclose[1] → short, hiclose = close + s; short and close > hiclose[1] → long, loclose = close − s; else long: loclose = max(loclose[1], close − s) (strict), short: hiclose = min(hiclose[1], close + s); stop = long ? loclose : hiclose (from bar 1; Sierra shows from bar 10); direction +1/−1; defaults n 9, factor 2.5, max range 1.5", &[1, 1], kiwis_trailing_stop),
    def("pmax", "Profit Maximizer (PMax)", TR, PM, &["pmax", "mavg", "direction"], PR, NO, "KivancOzbilgic Pine v4: src = hl2; atr = change_atr ? atr(n) (rma of tr) : sma(tr, n); mavg by type: SMA, EMA, WMA, TMA = sma(sma(src, ceil(L/2)), floor(L/2) + 1), VAR = our ott VAR (9-bar CMO VIDYA from 0), WWMA = src/L + (1 − 1/L)·nz(WWMA[1]), ZLEMA = ema(2·src − src[floor(L/2)], L), TSF = 2·linreg(src, L, 0) − linreg(src, L, 1); a = mult·atr (normalize: mult·atr/close, as the script); longStop = mavg − a, raised to max(longStop, longStop[1]) while mavg > longStop[1] (nz(x[1], x)); shortStop mirrors; direction (start 1) → 1 when mavg > shortStop[1], −1 when mavg < longStop[1]; pmax = direction 1 ? longStop : shortStop; signals: crossover/crossunder(mavg, pmax) in rules; defaults ATR 10, mult 3, EMA 10, Wilder ATR", &[9, 9, 9], pmax),
    def("gann_swing_osc", "Gann Swing Oscillator (Sierra Chart)", TR, &[], &["gso"], PN, NO, "Sierra Chart Gann Swing Oscillator (Krausz, TASC V16:2; ACSIL source, causal form): up bar = H > H[1] and L > L[1], down bar = H < H[1] and L < L[1] (our gann_trend_osc); upswing at s = down bar s then up bar s + 1; gso = 1 on bar s + 2 when upswing at s and H[s + 1] < H[s + 2]; −1 on bar s + 2 when downswing at s (up then down) and L[s + 1] > L[s + 2]; else gso[1]; starts at 0 (bar 0 has no previous bar, so it is neither up nor down)", &[0], gann_swing_osc),
    def("sidewinder", "SideWinder (Sierra Chart)", TR, SW, &["state"], PN, NO, "Sierra Chart Sidewinder (2021 ACSIL source, Woodies CCI): lsma = linreg(close, 25, 0); ema = Sierra EMA from bar 1 (= close[1], alpha 2/(min(34, i + 1) + 1)); angle(x) = atan((x − (x[1] + x[2])/2)/tick) in degrees; with s = ema angle + lsma angle, in order: |ema angle| ≥ 15 and s ≥ trend → 2; |ema angle| ≥ 15 and s ≤ −trend → −2; ema angle > 5 and lsma angle > 0 and s ≥ normal → 1; ema angle < 5 and lsma angle < 0 and s ≤ −normal → −1 (source uses < 5); else 0; tick: Sierra uses the chart tick, the lab default 0.05 (NSE equity tick) is our adaptation; defaults trend 100, normal 30", &[26], sidewinder),
];
/// (id, platform names, AmiBroker names): one row per id in DEFS.
pub const SOURCES: &[(&str, &str, &str)] = &[
    ("mavp", "TA-Lib: MAVP", ""),
    ("jma", "pandas-ta: jma (reconstruction; Jurik Research JMA is proprietary)", ""),
    ("gdema", "TA-Lib: GD stage of T3 (Tillson)", ""),
    ("block_ma", "Sierra Chart: Moving Average - Block", ""),
    ("erma", "Sierra Chart: Moving Average - Exponential Regression", ""),
    ("time_period_ma", "Sierra Chart: Moving Average - Time Period", ""),
    ("continuous_avg", "Sierra Chart: Average - Continuous", ""),
    ("arsi", "TradingView (community): Adaptive RSI by everget", ""),
    ("madrid_ribbon", "TradingView (community): Madrid Moving Average Ribbon by Madrid", ""),
    ("plus_minus_dm", "TA-Lib: PLUS_DM; TA-Lib: MINUS_DM", ""),
    ("sarext", "TA-Lib: SAREXT", ""),
    ("super_trend_stop", "Sierra Chart: Super Trend Stop", ""),
    ("kiwis_trailing_stop", "Sierra Chart: Kiwis Trailing Stop", ""),
    ("pmax", "TradingView (community): Profit Maximizer (PMax) by KivancOzbilgic", ""),
    ("gann_swing_osc", "Sierra Chart: Gann Swing Oscillator", ""),
    ("sidewinder", "Sierra Chart: SideWinder", ""),
];

#[cfg(test)]
mod tests {
    use crate::lab::testdata::{assert_series, close_eq, closes, compiles, from_ohlc, go, walk};
    const N: f64 = f64::NAN;
    fn hl(rows: &[(f64, f64)]) -> crate::lab::bars::Bars { from_ohlc(&rows.iter().map(|&(h, l)| ((h + l) / 2., h, l, (h + l) / 2.)).collect::<Vec<_>>()) }
    fn pair(c: &[f64], per: &[f64]) -> crate::lab::bars::Bars { let mut b = closes(c); b.second = Some(per.to_vec()); b }

    #[test] fn mavp_picks_each_bars_period_from_the_second_series() {
        let c: Vec<f64> = (1..=8).map(f64::from).collect();
        let o = go("mavp", &pair(&c, &[9., 9., 9., 2., 3., N, 9., 2.9]), &[("min_period", 2.), ("max_period", 4.)]);
        assert_series(&o[0], &[N, N, N, 3.5, 4., 5.5, 5.5, 7.5]); // 2; 3; NaN -> min; 9 -> max; 2.9 -> 2
        // EMA: TA-Lib seeds each period at the max-period lookback (bar 3: sma of bars 2..3), not at bar k − 1
        let e = go("mavp", &pair(&[1., 5., 2., 8., 3., 9.], &[2.; 6]), &[("min_period", 2.), ("max_period", 4.), ("ma_type", 1.)]);
        assert_series(&e[0], &[N, N, N, 5., 5. - 4. / 3., 11. / 3. + 2. / 3. * (9. - 11. / 3.)]);
        let d = crate::lab::catalogue::get("mavp").unwrap();
        let kv = |a: f64, b: f64| [("min_period".to_string(), a), ("max_period".to_string(), b)].into_iter().collect();
        assert!(crate::lab::catalogue::run(&d, &pair(&c, &[2.; 8]), &kv(5., 3.)).is_err());
        assert_eq!(crate::lab::catalogue::run(&d, &closes(&c), &kv(2., 4.)).unwrap_err(), "needs a second instrument");
    }
    #[test] fn mavp_is_causal() {
        let mut full = walk(200, 5, false);
        full.second = Some((0..200).map(|i| 2. + (i % 29) as f64).collect());
        for t in [0., 1., 2.] {
            let want = go("mavp", &full, &[("ma_type", t)]);
            for cut in (40..200).step_by(9) {
                let mut pre = crate::lab::testdata::cut(&full, cut);
                pre.second = Some(full.second.as_ref().unwrap()[..cut].to_vec());
                let got = go("mavp", &pre, &[("ma_type", t)]);
                for i in 0..cut { assert!(close_eq(got[0][i], want[0][i]), "type {t} bar {i} cut {cut}"); }
            }
        }
    }
    #[test] fn jma_matches_the_pandas_ta_loop() {
        let c = [10., 10.5, 11.2, 10.8, 11.5, 12.3, 12.0, 11.4, 11.9, 12.8, 13.1, 12.7];
        // python port of pandas-ta-classic _jma_loop (L 4, phase 30)
        assert_series(&go("jma", &closes(&c), &[("length", 4.), ("phase", 30.)])[0], &[N, N, N, 10.93117507938126, 11.384233806522852, 12.132573947531993,
            12.120099148690874, 11.634518529304964, 11.712429388434035, 12.51537851948214, 12.980014778825113, 12.973941931930915]);
        assert!(go("jma", &closes(&[5.; 20]), &[])[0][6..].iter().all(|&v| v == 5.));
    }
    #[test] fn gdema_hand_values_and_its_limits() {
        // n 2: ema1 = 1.5, 2.5, 3.5 from bar 1; ema2 = 2, 3 from bar 2
        assert_series(&go("gdema", &closes(&[1., 2., 3., 4.]), &[("length", 2.)])[0], &[N, N, 2.5 * 1.7 - 2. * 0.7, 3.5 * 1.7 - 3. * 0.7]);
        let b = walk(200, 2, false);
        assert_series(&go("gdema", &b, &[("vfactor", 1.)])[0], &go("dema", &b, &[("length", 5.)])[0]);
        let e = go("ema", &b, &[("length", 5.)])[0].clone();
        assert_series(&go("gdema", &b, &[("vfactor", 0.)])[0][8..], &e[8..]);
    }
    #[test] fn block_ma_boxes_move_only_on_breaks() {
        let b = from_ohlc(&[(10., 11., 9., 10.), (10., 12., 10., 11.), (11., 11.5, 10.5, 11.), (11., 11., 8., 8.5)]);
        // tr 2, 2, 1, 3 -> Sierra shrinking sma(2): 2, 2, 1.5, 2
        let o = go("block_ma", &b, &[("atr_length", 2.)]);
        assert_series(&o[0], &[N, 11., 11., 9.]); // box 1: bar 3 low 8 < bottom 10 -> down box, mid 8 + 1
        assert_series(&o[2], &[N, 1., 1., -1.]);
        assert_series(&o[1], &[N, 10., 10., 10.]); // box 2: bottom 8, low 8 is not strictly below
        assert_series(&o[3], &[N, 1., 1., 1.]);
    }
    #[test] fn erma_follows_an_exponential_exactly() {
        let c: Vec<f64> = (0..10).map(|i| 2f64.powi(i)).collect();
        let o = go("erma", &closes(&c), &[("length", 3.)]);
        assert_series(&o[0][2..], &c[2..]);
        assert!(go("erma", &closes(&[1., -1., 2., 3., 4.]), &[("length", 2.)])[0][..3].iter().all(|v| v.is_nan()));
    }
    #[test] fn time_period_ma_nearest_start_bar_and_zero_skip() {
        let mut b = closes(&[10., 20., 0., 40., 50.]);
        b.date = ["2024-01-01", "2024-01-02", "2024-01-03", "2024-01-05", "2024-01-08"].map(String::from).to_vec();
        // 01-03: from 01-01 -> (10 + 20)/2 (zero skipped); 01-05: nearest to 01-03 is 01-03 -> 40; 01-08: nearest to 01-06 is 01-05
        assert_series(&go("time_period_ma", &b, &[("days", 2.)])[0], &[N, N, 15., 40., 45.]);
        let mut t = closes(&[1., 2., 4.]);
        t.date = ["2024-01-01", "2024-01-03", "2024-01-04"].map(String::from).to_vec();
        assert_series(&go("time_period_ma", &t, &[("days", 2.)])[0], &[N, 1.5, 3.]); // 01-02 is 1 day from both: the later bar
    }
    #[test] fn continuous_avg_is_the_expanding_mean() {
        assert_series(&go("continuous_avg", &closes(&[2., 4., 6., 8.]), &[])[0], &[N, 3., 4., 5.]);
    }
    #[test] fn arsi_hand_values() {
        // rsi(2): 100, 50, 83.33 -> alpha 1, 0, 2/3; nz seed 0
        assert_series(&go("arsi", &closes(&[10., 11., 12., 11., 13.]), &[("length", 2.)])[0], &[N, N, 12., 12., 2. / 3. * 13. + 12. / 3.]);
    }
    #[test] fn madrid_states_on_steady_trends() {
        let up: Vec<f64> = (0..150).map(|i| 100. + i as f64).collect();
        let o = go("madrid_ribbon", &closes(&up), &[]);
        assert_eq!(o.len(), 36);
        assert!(o[18..].iter().all(|s| s[99..].iter().all(|&v| v == 2.) && s[98].is_nan()));
        let dn: Vec<f64> = up.iter().rev().cloned().collect();
        assert!(go("madrid_ribbon", &closes(&dn), &[("exponential", 0.)])[18..].iter().all(|s| s[99..].iter().all(|&v| v == -2.)));
    }
    #[test] fn plus_minus_dm_wilder_sums() {
        let b = hl(&[(10., 9.), (11., 9.5), (10.5, 8.), (12., 10.), (11., 9.)]);
        // one-bar +DM 1, 0, 1.5, 0; −DM 0, 1.5, 0, 1
        let o = go("plus_minus_dm", &b, &[("length", 3.)]);
        assert_series(&o[0], &[N, N, 1., 1. - 1. / 3. + 1.5, (1. - 1. / 3. + 1.5) * 2. / 3.]);
        assert_series(&o[1], &[N, N, 1.5, 1., 1. - 1. / 3. + 1.]);
        assert_series(&go("plus_minus_dm", &b, &[("length", 1.)])[0], &[N, 1., 0., 1.5, 0.]);
    }
    #[test] fn sarext_hand_trace() {
        let b = hl(&[(10., 9.), (11., 9.5), (12., 10.5), (11.5, 9.8), (10., 8.)]);
        let o = go("sarext", &b, &[]);
        // long (−DM 0): sar 9; 9 + 0.02·2; new high 12 -> af 0.04; low 8 ≤ 9.272064 -> short at max(ep 12, 11.5, 10)
        assert_series(&o[0], &[N, 9., 9.04, 9.1584, 12.]);
        assert_series(&o[1], &[N, 1., 1., 1., -1.]);
        let s = go("sarext", &hl(&[(10., 9.), (9.5, 8.)]), &[]); // −DM 1 > 0 -> short at high[0]
        assert_series(&s[0], &[N, 10.]);
        assert_series(&s[1], &[N, -1.]);
        assert!(close_eq(go("sarext", &b, &[("offset", 0.1)])[0][4], 13.2));
    }
    #[test] fn super_trend_stop_hand_trace() {
        let b = from_ohlc(&[(10., 11., 9., 10.), (10., 12., 10., 11.5), (11., 13., 11., 12.5), (12., 12.5, 9., 9.5), (9.5, 10., 9., 9.2)]);
        let o = go("super_trend_stop", &b, &[("mult", 1.), ("atr_length", 1.), ("median_length", 1.)]);
        // atr[i − 1] = tr of the previous bar: 2, 2, 2, 3.5
        assert_series(&o[0], &[10., 10., 10., 10.75 + 2., 12.75]);
        assert_series(&o[1], &[1., 1., 1., -1., -1.]);
        let m = go("super_trend_stop", &b, &[("mult", 1.), ("atr_length", 1.), ("median_length", 2.)]);
        assert_eq!(m[0][3], (12. + 10.75) / 2. + 2.); // even median: mean of hl2 12 and 10.75 on the flip bar
    }
    #[test] fn kiwis_trailing_stop_matches_the_acsil_loop() {
        let (h, l, c) = ([11., 12., 12.5, 12.2, 11.5, 11.0, 12.0, 13.0, 13.5, 13.2], [9., 10., 11., 10.8, 10., 9.5, 10.5, 11.5, 12.5, 12.0], [10.5, 11.5, 12., 11., 10.2, 10.8, 11.9, 12.6, 13.3, 12.4]);
        let b = from_ohlc(&(0..10).map(|i| (c[i], h[i], l[i], c[i])).collect::<Vec<_>>());
        // python port of scsf_KiwisTrailingStop (n 3, factor 1)
        let o = go("kiwis_trailing_stop", &b, &[("atr_length", 3.), ("factor", 1.)]);
        assert_series(&o[0], &[N, 10.5, 10.5, 10.5, 11.788623046875, 11.788623046875, 10.4024169921875, 11.11103515625, 11.933416748046875, 11.933416748046875]);
        assert_series(&o[1], &[N, 1., 1., 1., -1., -1., 1., 1., 1., 1.]);
        let v = go("kiwis_trailing_stop", &b, &[("atr_length", 3.), ("factor", 1.), ("vervoort", 1.)]);
        assert_series(&v[0], &[N, 10.5, 10.5, 10.5, 11.825925925925924, 11.825925925925924, 10.344032921810701, 11.062688614540466, 11.941792409693646, 11.941792409693646]);
    }
    #[test] fn pmax_hand_trace_and_default_mavg() {
        let b = from_ohlc(&[(10., 11., 9., 10.), (10., 12., 10., 11.), (11., 11., 7., 8.), (8., 8., 6., 7.), (7., 6., 2., 3.)]);
        let o = go("pmax", &b, &[("atr_length", 1.), ("mult", 1.), ("ma_type", 0.), ("ma_length", 1.)]);
        // mavg = hl2 10, 11, 9, 7, 4; atr = tr 2, 2, 4, 2, 5; bar 4: 4 < longStop[1] 5 -> short at shortStop 9
        assert_series(&o[0], &[8., 9., 5., 5., 9.]);
        assert_series(&o[2], &[1., 1., 1., 1., -1.]);
        let w = walk(300, 3, false);
        assert_series(&go("pmax", &w, &[])[1], &crate::lab::ta::ema(&w.hl2(), 10));
        for t in 0..8 { assert!(go("pmax", &w, &[("ma_type", t as f64)])[0][299].is_finite(), "type {t}"); }
    }
    #[test] fn gann_swing_needs_the_swing_then_a_third_bar() {
        let b = hl(&[(10., 9.), (9.5, 8.5), (10., 9.), (10.5, 9.5), (11., 10.), (10.5, 9.6), (10., 9.2)]);
        // down bar 1, up bar 2, higher high on bar 3 -> +1; up bar 4, down bar 5, lower low on bar 6 -> −1
        assert_series(&go("gann_swing_osc", &b, &[])[0], &[0., 0., 0., 1., 1., 1., -1.]);
    }
    #[test] fn sidewinder_hand_angles() {
        let b = closes(&[10., 10., 10., 13.]);
        // lsma(2) = close; Sierra ema(2): 10, 10, 2/3·13 + 10/3 = 12; angles atan(3) = 71.57°, atan(2) = 63.43°, sum 135
        let kv = [("lsma_length", 2.), ("ema_length", 2.), ("tick", 1.)];
        assert_series(&go("sidewinder", &b, &kv)[0], &[N, N, N, 2.]);
        assert_eq!(go("sidewinder", &b, &[kv[0], kv[1], kv[2], ("trend_threshold", 140.)])[0][3], 1.);
        assert!(go("sidewinder", &closes(&[10.; 40]), &[])[0][26..].iter().all(|&v| v == 0.));
    }
    #[test] fn parameter_bounds_never_panic() {
        for mut bars in [walk(3, 1, true), walk(300, 1, true)] {
            bars.second = Some(vec![7.; bars.len()]);
            for d in super::DEFS {
                for pick in [|p: &crate::lab::catalogue::ParamSpec| p.min, |p: &crate::lab::catalogue::ParamSpec| p.max] {
                    let kv: Vec<(&str, f64)> = d.params.iter().map(|p| (p.key, pick(p))).collect();
                    assert!(go(d.id, &bars, &kv).iter().all(|s| s.len() == bars.len()), "{}", d.id);
                }
            }
        }
    }
    #[test] fn rules_can_read_every_output() {
        for d in super::DEFS.iter().filter(|d| !d.needs.second_symbol) { for o in d.outputs { compiles(d.id, o).unwrap_or_else(|e| panic!("{}.{o}: {e:?}", d.id)); } }
    }
}
