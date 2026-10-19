//! Cross-platform indicators, Trend (2026-10-19 build list: TradingView, AmiBroker, MetaTrader, cTrader, NinjaTrader, TradeStation, thinkorswim, Sierra Chart, MetaStock). Formulas: see each definition string.
use super::bars::Bars;
use super::catalogue::{get, int, len, n, p, real, run, IndicatorDef, Needs, Output, ParamSpec, Params, Placement};
use super::ta::{self, nz, NA};

const CAT: &str = "Trend";
#[allow(clippy::too_many_arguments)]
const fn def(id: &'static str, name: &'static str, params: &'static [ParamSpec], outputs: &'static [&'static str], placement: Placement,
             needs: Needs, definition: &'static str, first: &'static [usize], sparse: bool, f: fn(&Bars, &Params) -> Output) -> IndicatorDef {
    IndicatorDef { id, name, category: CAT, params, outputs, placement, needs, definition, first, sparse, compute: Some(f), note: "" }
}
const PR: Placement = Placement::Price;
const PN: Placement = Placement::Pane;
const NO: Needs = Needs::NONE;

/// Another catalogue indicator on `b` (reuses its exact code: rsi, mfi, kama, supertrend).
fn call(id: &str, b: &Bars, kv: &[(&str, f64)]) -> Result<Vec<Vec<f64>>, String> {
    run(&get(id).ok_or_else(|| format!("{id} missing"))?, b, &kv.iter().map(|(k, v)| (k.to_string(), *v)).collect())
}
/// Pine `ta.sma(ta.tr, n)` (tr NaN on bar 0); equals MetaQuotes iATR from bar n.
fn atr_sma(b: &Bars, k: usize) -> Vec<f64> { ta::sma(&ta::tr(b, false), k) }
/// Pine `ta.atr(n)` = rma(tr(true), n).
fn atr_rma(b: &Bars, k: usize) -> Vec<f64> { ta::rma(&ta::tr(b, true), k) }
/// Kivanc's nz-seeded ATR ratchet (AlphaTrend, Trend Magic): bull ? (low − m·atr < x[1] ? x[1] : low − m·atr) : (high + m·atr > x[1] ? x[1] : high + m·atr).
/// Pine holds its nz(x[1]) = 0 seed until the first bullish bar; that seed is reported as NaN, not 0.
fn kivanc_line(b: &Bars, atr: &[f64], m: f64, bull: impl Fn(usize) -> bool) -> Vec<f64> {
    let mut out = vec![NA; b.len()];
    let (mut prev, mut live) = (NA, false);
    for i in 0..b.len() {
        let (up, dn, p1) = (b.low[i] - atr[i] * m, b.high[i] + atr[i] * m, nz(prev));
        let v = if bull(i) { if up < p1 { p1 } else { up } } else if dn > p1 { p1 } else { dn };
        prev = v;
        if v.is_finite() && v != 0. { live = true; }
        if live { out[i] = v; }
    }
    out
}

const AT: &[ParamSpec] = &[len("length", "Length", 14.), real("coeff", "Multiplier", 1., 0.1, 10., 0.1)];
fn alphatrend(b: &Bars, prm: &Params) -> Output {
    let (k, c) = (n(prm, "length"), p(prm, "coeff"));
    let momo = call(if b.volume.is_some() { "mfi" } else { "rsi" }, b, &[("length", k as f64)])?.swap_remove(0);
    let at = kivanc_line(b, &atr_sma(b, k), c, |i| momo[i] >= 50.);
    let sig = (0..b.len()).map(|i| {
        if i < 3 || at[i].is_nan() || at[i - 3].is_nan() { return NA; }
        if at[i] > at[i - 2] && at[i - 1] <= at[i - 3] { 1. } else if at[i] < at[i - 2] && at[i - 1] >= at[i - 3] { -1. } else { 0. }
    }).collect();
    Ok(vec![at, sig])
}

const ATS: &[ParamSpec] = &[len("atr_length", "ATR length", 5.), real("factor", "ATR factor", 3.5, 0.1, 20., 0.1), int("modified", "Modified true range (0/1)", 1., 0., 1.)];
fn atr_trailing_stop(b: &Bars, prm: &Params) -> Output {
    let (k, f, m) = (n(prm, "atr_length"), p(prm, "factor"), b.len());
    let (h, l, c) = (&b.high, &b.low, &b.close);
    let tr = if p(prm, "modified") == 1. {
        let hl: Vec<f64> = (0..m).map(|i| h[i] - l[i]).collect();
        let cap = ta::sma(&hl, k);
        (0..m).map(|i| if i == 0 || cap[i].is_nan() { NA } else {
            let href = if l[i] <= h[i - 1] { h[i] - c[i - 1] } else { (h[i] - c[i - 1]) - 0.5 * (l[i] - h[i - 1]) };
            let lref = if h[i] >= l[i - 1] { c[i - 1] - l[i] } else { (c[i - 1] - l[i]) - 0.5 * (l[i - 1] - h[i]) };
            hl[i].min(1.5 * cap[i]).max(href).max(lref)
        }).collect()
    } else { ta::tr(b, false) };
    let loss = ta::rma(&tr, k);
    let (mut stop, mut dir) = (vec![NA; m], vec![NA; m]);
    let (mut s1, mut d1) = (NA, NA);
    for i in 0..m {
        if loss[i].is_nan() { continue; }
        let lo = f * loss[i];
        let (s, d) = if d1.is_nan() { (c[i] - lo, 1.) } // TOS firstTrade = long
            else if d1 == 1. { if c[i] > s1 { (s1.max(c[i] - lo), 1.) } else { (c[i] + lo, -1.) } }
            else if c[i] < s1 { (s1.min(c[i] + lo), -1.) } else { (c[i] - lo, 1.) };
        stop[i] = s; dir[i] = d; s1 = s; d1 = d;
    }
    Ok(vec![stop, dir])
}

fn clear_method(b: &Bars, _: &Params) -> Output {
    let m = b.len();
    let (mut line, mut dir) = (vec![NA; m], vec![NA; m]);
    // Sierra Chart zero-initialised state; its highest-high/lowest-low arrays never reach the line, so they are not kept
    let (mut hl, mut lh, mut up) = (0., 0., false);
    for i in 0..m {
        let (h, l) = (b.high[i], b.low[i]);
        if up {
            if l > hl { hl = l; }
            if h < hl { up = false; lh = h; }
        }
        if !up {
            if h < lh { lh = h; }
            if l > lh { up = true; hl = l; }
        }
        line[i] = if up { hl } else { lh };
        dir[i] = if up { 1. } else { -1. };
    }
    Ok(vec![line, dir])
}

const L14: &[ParamSpec] = &[len("length", "Length", 14.)];
fn dmh(b: &Bars, prm: &Params) -> Output {
    let (k, m) = (n(prm, "length"), b.len());
    let sf = 1. / k as f64;
    let mut e = vec![0.; m]; // EasyLanguage vars start at 0
    for i in 1..m {
        let (up, dn) = (b.high[i] - b.high[i - 1], b.low[i - 1] - b.low[i]);
        let dm = if up > dn && up > 0. { up } else if dn > up && dn > 0. { -dn } else { 0. };
        e[i] = sf * dm + (1. - sf) * e[i - 1];
    }
    let w: Vec<f64> = (1..=k).map(|j| 1. - (2. * std::f64::consts::PI * j as f64 / (k as f64 + 1.)).cos()).collect();
    let ws: f64 = w.iter().sum();
    Ok(vec![(0..m).map(|t| if t < k { NA } else { w.iter().enumerate().map(|(j, wj)| wj * e[t - j]).sum::<f64>() / ws }).collect()])
}

const L10: &[ParamSpec] = &[len("length", "Length", 10.)];
fn abs_sum(c: &[f64], k: usize) -> Vec<f64> { ta::sum(&ta::change(c, 1).iter().map(|v| v.abs()).collect::<Vec<_>>(), k) }
fn er(b: &Bars, prm: &Params) -> Output {
    let (k, c) = (n(prm, "length"), &b.close);
    let path = abs_sum(c, k);
    Ok(vec![(0..c.len()).map(|i| if path[i].is_nan() { NA } else if path[i] == 0. { 1. } else { ((c[i] - c[i - k]).abs() / path[i]).min(1.) }).collect()])
}

const GH: &[ParamSpec] = &[len("high_length", "High SMA", 3.), len("low_length", "Low SMA", 3.)];
fn gann_hilo(b: &Bars, prm: &Params) -> Output {
    let (hma, lma, m) = (ta::sma(&b.high, n(prm, "high_length")), ta::sma(&b.low, n(prm, "low_length")), b.len());
    let (mut hilo, mut dir) = (vec![NA; m], vec![NA; m]);
    let mut d = NA;
    for i in 1..m {
        if hma[i - 1].is_nan() || lma[i - 1].is_nan() { continue; }
        if b.close[i] > hma[i - 1] { d = 1. } else if b.close[i] < lma[i - 1] { d = -1. }
        if d.is_nan() { continue; }
        dir[i] = d;
        hilo[i] = if d == 1. { lma[i] } else { hma[i] };
    }
    Ok(vec![hilo, dir])
}

const ST: &[ParamSpec] = &[len("atr_length", "ATR length", 10.), real("factor", "Factor", 3., 0.5, 20., 0.5)];
/// Heikin-Ashi bars (TradingView heikinashi): close = ohlc4, open = (open[1] + close[1])/2 seeded (o + c)/2.
fn heikin_ashi(b: &Bars) -> Bars {
    let mut h = b.clone();
    let c = b.ohlc4();
    let mut o = vec![NA; b.len()];
    for i in 0..b.len() {
        o[i] = if i == 0 { (b.open[0] + b.close[0]) / 2. } else { (o[i - 1] + c[i - 1]) / 2. };
        h.high[i] = b.high[i].max(o[i]).max(c[i]);
        h.low[i] = b.low[i].min(o[i]).min(c[i]);
    }
    h.open = o; h.close = c;
    h
}
fn ha_supertrend(b: &Bars, prm: &Params) -> Output { call("supertrend", &heikin_ashi(b), &[("atr_length", p(prm, "atr_length")), ("factor", p(prm, "factor"))]) }

const HT: &[ParamSpec] = &[len("amplitude", "Amplitude", 2.), real("channel_dev", "Channel deviation", 2., 0., 10., 0.1)];
fn halftrend(b: &Bars, prm: &Params) -> Output {
    let (amp, cd, m) = (n(prm, "amplitude"), p(prm, "channel_dev"), b.len());
    let (hp, lp) = (ta::highest(&b.high, amp), ta::lowest(&b.low, amp));
    let (hma, lma) = (ta::sma(&b.high, amp), ta::sma(&b.low, amp));
    let atr2: Vec<f64> = atr_rma(b, 100).iter().map(|v| v / 2.).collect();
    let mut out = vec![vec![NA; m]; 4];
    if m == 0 { return Ok(out); }
    let (mut trend, mut next) = (0., 0.);
    let (mut max_low, mut min_high) = (b.low[0], b.high[0]); // var … = nz(low[1], low)
    let (mut up, mut down) = (0., 0.);
    for i in 0..m {
        let prev_trend = trend;
        let (pl, ph) = (b.low[i.saturating_sub(1)], b.high[i.saturating_sub(1)]); // nz(low[1], low)
        if next == 1. {
            // highest/lowest are NaN before bar amplitude − 1; nextTrend cannot be 1 that early (lowma is NaN), so this guard never fires
            if !lp[i].is_nan() { max_low = lp[i].max(max_low); }
            if hma[i] < max_low && b.close[i] < pl { trend = 1.; next = 0.; min_high = hp[i]; }
        } else {
            if !hp[i].is_nan() { min_high = hp[i].min(min_high); } // Pine's partial-window highestbars ≥ high[0] = minHigh: same result
            if lma[i] > min_high && b.close[i] > ph { trend = 0.; next = 1.; max_low = lp[i]; }
        }
        if trend == 0. {
            up = if i > 0 && prev_trend != 0. { down } else if i == 0 { max_low } else { max_low.max(up) };
        } else {
            down = if i > 0 && prev_trend != 1. { up } else { min_high.min(down) };
        }
        let ht = if trend == 0. { up } else { down };
        out[0][i] = ht; out[1][i] = trend; out[2][i] = ht + cd * atr2[i]; out[3][i] = ht - cd * atr2[i];
    }
    Ok(out)
}

const BW: &[ParamSpec] = &[len("length", "Length", 10.), len("fast", "Fast", 2.), len("slow", "Slow", 30.), real("pct", "Filter percent", 25., 0., 100., 1.)];
fn kama_binary_wave(b: &Bars, prm: &Params) -> Output {
    let k = n(prm, "length");
    let ama = call("kama", b, &[("length", k as f64), ("fast", p(prm, "fast")), ("slow", p(prm, "slow"))])?.swap_remove(0);
    let sd = ta::stdev(&ama, k);
    let pct = p(prm, "pct") / 100.;
    let (mut lo, mut hi) = (NA, NA);
    Ok(vec![(0..ama.len()).map(|i| {
        let a = ama[i];
        if a.is_nan() { return NA; }
        if lo.is_nan() { lo = a; hi = a; } else { if a < ama[i - 1] { lo = a; } if a > ama[i - 1] { hi = a; } }
        if sd[i].is_nan() { return NA; }
        let f = pct * sd[i];
        if a - lo > f { 1. } else if hi - a > f { -1. } else { 0. }
    }).collect()])
}

const NR: &[ParamSpec] = &[len("atr_length", "ATR length", 40.), real("k", "ATR multiplier", 2., 0.1, 10., 0.1)];
fn nrtr(b: &Bars, prm: &Params) -> Output {
    let (k, mult, m) = (n(prm, "atr_length"), p(prm, "k"), b.len());
    let atr = atr_sma(b, k);
    let (h, l, c) = (&b.high, &b.low, &b.close);
    let (mut buy, mut sell, mut trend) = (vec![NA; m], vec![NA; m], vec![NA; m]);
    if m < k { return Ok(vec![buy, sell, trend]); }
    let s = k - 1; // MT5 seeds on bar n − 1, where its ATR buffer is still 0
    let (mut ceil, mut floor) = (NA, NA);
    if c[s] > l[s] { trend[s] = 1.; ceil = c[s]; buy[s] = c[s]; } else { trend[s] = -1.; floor = c[s]; sell[s] = c[s]; }
    for i in s + 1..m {
        let a = mult * atr[i];
        let (t, b1, s1) = (trend[i - 1], buy[i - 1], sell[i - 1]);
        if t > 0. {
            if l[i] > ceil { ceil = c[i]; floor = NA; buy[i] = c[i] - a; trend[i] = 1.; continue; }
            if c[i] < b1 { trend[i] = -1.; floor = c[i]; sell[i] = c[i] + a; ceil = NA; continue; }
        } else {
            if h[i] < floor { floor = c[i]; ceil = NA; sell[i] = c[i] + a; trend[i] = -1.; continue; }
            if c[i] > s1 { trend[i] = 1.; ceil = c[i]; buy[i] = c[i] - a; floor = NA; continue; }
        }
        buy[i] = b1; sell[i] = s1; trend[i] = t;
    }
    Ok(vec![buy, sell, trend])
}

const OT: &[ParamSpec] = &[len("length", "VAR length", 2.), real("percent", "Percent", 1.4, 0.1, 50., 0.1)];
/// Kivanc OTT on `c`: [ott, mavg, direction].
fn ott_s(c: &[f64], k: usize, pct: f64) -> Vec<Vec<f64>> {
    let m = c.len();
    let a = 2. / (k as f64 + 1.);
    let ud: Vec<f64> = (0..m).map(|i| if i > 0 && c[i] > c[i - 1] { c[i] - c[i - 1] } else { 0. }).collect();
    let dd: Vec<f64> = (0..m).map(|i| if i > 0 && c[i] < c[i - 1] { c[i - 1] - c[i] } else { 0. }).collect();
    let (su, sd) = (ta::sum(&ud, 9), ta::sum(&dd, 9));
    let mut var = vec![0.; m];
    for i in 0..m {
        let cmo = nz((su[i] - sd[i]) / (su[i] + sd[i])).abs();
        var[i] = nz(a * cmo * c[i]) + (1. - a * cmo) * if i > 0 { var[i - 1] } else { 0. };
    }
    let (mut ott, mut dir) = (vec![NA; m], vec![NA; m]);
    let (mut ls1, mut ss1, mut d) = (NA, NA, 1.);
    for i in 0..m {
        let mv = var[i];
        let fark = mv * pct / 100.;
        let (lp, sp) = (if ls1.is_nan() { mv - fark } else { ls1 }, if ss1.is_nan() { mv + fark } else { ss1 });
        let ls = if mv > lp { (mv - fark).max(lp) } else { mv - fark };
        let ss = if mv < sp { (mv + fark).min(sp) } else { mv + fark };
        d = if d == -1. && mv > sp { 1. } else if d == 1. && mv < lp { -1. } else { d };
        let mt = if d == 1. { ls } else { ss };
        ott[i] = if mv > mt { mt * (200. + pct) / 200. } else { mt * (200. - pct) / 200. };
        dir[i] = d; ls1 = ls; ss1 = ss;
    }
    vec![ott, var, dir]
}
fn ott(b: &Bars, prm: &Params) -> Output { Ok(ott_s(&b.close, n(prm, "length"), p(prm, "percent"))) }

const PF: &[ParamSpec] = &[int("length", "Length", 10., 2., 2000.), len("smooth", "EMA smoothing", 5.)]; // length 1: no path
fn pfe(b: &Bars, prm: &Params) -> Output {
    let (k, c) = (n(prm, "length") - 1, &b.close);
    let step: Vec<f64> = (0..c.len()).map(|i| if i == 0 { NA } else { ((c[i] - c[i - 1]).powi(2) + 1.).sqrt() }).collect();
    let den = ta::sum(&step, k);
    let raw: Vec<f64> = (0..c.len()).map(|i| if den[i].is_nan() { NA } else {
        let d = c[i] - c[i - k];
        (if d > 0. { 100. } else { -100. }) * (d * d + (k * k) as f64).sqrt() / den[i]
    }).collect();
    Ok(vec![ta::ema(&raw, n(prm, "smooth"))])
}

const PP: &[ParamSpec] = &[len("pivot_period", "Pivot period", 2.), real("factor", "ATR factor", 3., 0.1, 20., 0.1), len("atr_length", "ATR length", 10.)];
fn pp_supertrend(b: &Bars, prm: &Params) -> Output {
    let (prd, f, m) = (n(prm, "pivot_period"), p(prm, "factor"), b.len());
    let (ph, pl) = (super::price::pivots(&b.high, prd, prd, true), super::price::pivots(&b.low, prd, prd, false));
    let atr = atr_rma(b, n(prm, "atr_length"));
    let (mut line, mut trend, mut center) = (vec![NA; m], vec![NA; m], vec![NA; m]);
    let (mut ctr, mut tup1, mut tdn1, mut t1) = (NA, NA, NA, NA);
    for i in 0..m {
        // Pine v4 `ph ? ph : pl ? pl : na`: na and 0 are false
        let last = if !ph[i].is_nan() && ph[i] != 0. { ph[i] } else if !pl[i].is_nan() && pl[i] != 0. { pl[i] } else { NA };
        if !last.is_nan() { ctr = if ctr.is_nan() { last } else { (ctr * 2. + last) / 3. }; }
        let (up, dn) = (ctr - f * atr[i], ctr + f * atr[i]);
        let pc = if i > 0 { b.close[i - 1] } else { NA };
        let tup = if pc > tup1 { up.max(tup1) } else { up };
        let tdn = if pc < tdn1 { dn.min(tdn1) } else { dn };
        let t = if b.close[i] > tdn1 { 1. } else if b.close[i] < tup1 { -1. } else if t1.is_nan() { 1. } else { t1 };
        line[i] = if t == 1. { tup } else { tdn }; trend[i] = t; center[i] = ctr;
        tup1 = tup; tdn1 = tdn; t1 = t;
    }
    Ok(vec![line, trend, center])
}

fn price_time_trend(b: &Bars, _: &Params) -> Output {
    let v = b.volume.as_ref().ok_or("needs volume")?;
    let m = b.len();
    let mut out = vec![vec![NA; m]; 3];
    let (mut t, mut vol, mut cnt) = (NA, 0., 0.);
    for i in 1..m {
        let nt = if b.close[i] > b.high[i - 1] { 1. } else if b.close[i] < b.low[i - 1] { -1. } else { t };
        if nt.is_nan() { continue; }
        if nt != t { vol = 0.; cnt = 0.; }
        t = nt; vol += v[i]; cnt += 1.;
        out[0][i] = t; out[1][i] = t * vol; out[2][i] = t * cnt;
    }
    Ok(out)
}

const RF: &[ParamSpec] = &[len("fast", "Fast period", 27.), real("fast_mult", "Fast multiplier", 1.6, 0.1, 10., 0.1), len("slow", "Slow period", 55.), real("slow_mult", "Slow multiplier", 2., 0.1, 10., 0.1)];
fn range_filter(b: &Bars, prm: &Params) -> Output {
    let (c, m) = (&b.close, b.len());
    let ac: Vec<f64> = ta::change(c, 1).iter().map(|v| v.abs()).collect();
    let sr = |t: usize, mu: f64| -> Vec<f64> { ta::ema(&ta::ema(&ac, t), 2 * t - 1).iter().map(|v| v * mu).collect() };
    let (r1, r2) = (sr(n(prm, "fast"), p(prm, "fast_mult")), sr(n(prm, "slow"), p(prm, "slow_mult")));
    let mut out = vec![vec![NA; m]; 4];
    let (mut f1, mut d) = (NA, NA);
    for i in 0..m {
        let r = (r1[i] + r2[i]) / 2.;
        let p1 = nz(f1);
        let f = if c[i] > p1 { if c[i] - r < p1 { p1 } else { c[i] - r } } else if c[i] + r > p1 { p1 } else { c[i] + r };
        if f > f1 { d = 1. } else if f < f1 { d = -1. }
        out[0][i] = f; out[1][i] = f + r; out[2][i] = f - r; out[3][i] = d;
        f1 = f;
    }
    Ok(out)
}

fn ssl(b: &Bars, prm: &Params) -> Output {
    let k = n(prm, "length");
    let (sh, sl, m) = (ta::sma(&b.high, k), ta::sma(&b.low, k), b.len());
    let mut out = vec![vec![NA; m]; 3];
    let mut d = NA;
    for i in 0..m {
        if b.close[i] > sh[i] { d = 1. } else if b.close[i] < sl[i] { d = -1. }
        out[0][i] = if d < 0. { sl[i] } else { sh[i] };
        out[1][i] = if d < 0. { sh[i] } else { sl[i] };
        out[2][i] = d;
    }
    Ok(out)
}

const SF: &[ParamSpec] = &[len("length", "Stiffness length", 60.), len("ma_length", "MA length", 100.), real("k", "Stdev factor", 0.2, 0., 5., 0.1), len("smooth", "EMA smoothing", 3.)];
fn stiffness(b: &Bars, prm: &Params) -> Output {
    let (k, ml, c) = (n(prm, "length"), n(prm, "ma_length"), &b.close);
    let (ma, sd, kk) = (ta::sma(c, ml), ta::stdev(c, ml), p(prm, "k"));
    let above: Vec<f64> = (0..c.len()).map(|i| if ma[i].is_nan() { NA } else if c[i] > ma[i] - kk * sd[i] { 1. } else { 0. }).collect();
    let s: Vec<f64> = ta::sum(&above, k).iter().map(|v| 100. * v / k as f64).collect();
    Ok(vec![ta::ema(&s, n(prm, "smooth"))])
}

const TM: &[ParamSpec] = &[len("cci_length", "CCI length", 20.), len("atr_length", "ATR length", 5.), real("mult", "ATR multiplier", 1., 0.1, 10., 0.1)];
fn trend_magic(b: &Bars, prm: &Params) -> Output {
    let cci = super::osc2::cci_s(&b.close, n(prm, "cci_length"));
    let magic = kivanc_line(b, &atr_sma(b, n(prm, "atr_length")), p(prm, "mult"), |i| cci[i] >= 0.);
    Ok(vec![magic, cci.iter().map(|v| if v.is_nan() { NA } else if *v >= 0. { 1. } else { -1. }).collect()])
}

const UT: &[ParamSpec] = &[real("key", "Key value", 1., 0.1, 20., 0.1), len("atr_length", "ATR length", 10.), int("heikin", "Use Heikin-Ashi close (0/1)", 0., 0., 1.)];
fn utbot(b: &Bars, prm: &Params) -> Output {
    let src = if p(prm, "heikin") == 1. { b.ohlc4() } else { b.close.clone() };
    let atr = atr_rma(b, n(prm, "atr_length"));
    let (key, m) = (p(prm, "key"), b.len());
    let mut out = vec![vec![NA; m]; 3];
    let mut s1 = NA;
    for i in 0..m {
        let nl = key * atr[i];
        if nl.is_nan() { s1 = NA; continue; } // Pine max/min with na → na in every branch
        let (x, x1, p1) = (src[i], if i > 0 { src[i - 1] } else { NA }, nz(s1));
        let s = if x > p1 && x1 > p1 { p1.max(x - nl) } else if x < p1 && x1 < p1 { p1.min(x + nl) } else if x > p1 { x - nl } else { x + nl };
        out[0][i] = s;
        out[1][i] = if x > s && x1 <= s1 { 1. } else { 0. };
        out[2][i] = if x < s && x1 >= s1 { 1. } else { 0. };
        s1 = s;
    }
    Ok(out)
}

const VH: &[ParamSpec] = &[int("length", "Length", 28., 2., 2000.)];
fn vhf(b: &Bars, prm: &Params) -> Output {
    let (k, c) = (n(prm, "length"), &b.close);
    let (hh, ll, path) = (ta::highest(c, k), ta::lowest(c, k), abs_sum(c, k));
    Ok(vec![(0..c.len()).map(|i| if path[i].is_nan() { NA } else if path[i] == 0. { 0. } else { (hh[i] - ll[i]) / path[i] }).collect()])
}

pub const DEFS: &[IndicatorDef] = &[
    def("alphatrend", "AlphaTrend", AT, &["alphatrend", "signal"], PR, NO, "KivancOzbilgic Pine: atr = sma(tr, n); cond = volume present ? mfi(hlc3, n) ≥ 50 : rsi(close, n) ≥ 50; at = cond ? (low − coeff·atr < nz(at[1]) ? nz(at[1]) : low − coeff·atr) : (high + coeff·atr > nz(at[1]) ? nz(at[1]) : high + coeff·atr); Pine's nz seed 0 is held until the first bullish bar and shown as NaN here (warm-up depends on the data); signal = +1 on crossover(at, at[2]), −1 on crossunder, else 0 (raw crosses, without the script's alternate-signal filter)", &[usize::MAX, usize::MAX], true, alphatrend),
    def("atr_trailing_stop", "ATR Trailing Stop (Vervoort modified ATR)", ATS, &["stop", "direction"], PR, NO, "thinkorswim ATRTrailingStop (Vervoort, TASC 2009): modified tr = max(min(H − L, 1.5·sma(H − L, n)), href, lref), href = L ≤ H[1] ? H − C[1] : (H − C[1]) − 0.5·(L − H[1]), lref = H ≥ L[1] ? C[1] − L : (C[1] − L) − 0.5·(L[1] − H); modified 0 → plain true range; loss = factor·rma(tr, n) (Wilder, SMA seed as TradingView); first trade long: stop = C − loss; long: C > stop[1] ? max(stop[1], C − loss) : flip short at C + loss; short mirrors; direction +1 long / −1 short", &[8, 8], false, atr_trailing_stop),
    def("clear_method", "Clear Method Swing Line (Ron Black)", &[], &["line", "direction"], PR, NO, "Sierra Chart Clear Method Swing Line (ACSIL source): up-swing tracks the highest low HL and turns down when H < HL (LH = H); down-swing tracks the lowest high LH and turns up when L > LH (HL = L); state starts at zero, so bar 0 opens an up-swing; line = HL in an up-swing, LH in a down-swing; direction +1 up / −1 down", &[0, 0], false, clear_method),
    def("dmh", "Ehlers DMH (Hann-windowed directional movement)", L14, &["dmh"], PN, NO, "Ehlers (TASC Dec 2021, author's code): +DM/−DM as Wilder (larger positive move survives); ema = (1/n)·(+DM − −DM) + (1 − 1/n)·ema[1], starting from 0; dmh = Σ_{k=1..n} (1 − cos(2πk/(n+1)))·ema[k−1] / Σ (1 − cos(2πk/(n+1))); first value once n bars of ema exist", &[14], false, dmh),
    def("er", "Kaufman Efficiency Ratio", L10, &["er"], PN, NO, "|close − close[n]| / Σ_{n} |close − close[1]| (TA-Lib ER: flat window → 1, capped at 1)", &[10], false, er),
    def("gann_hilo", "Gann HiLo Activator", GH, &["hilo", "direction"], PR, NO, "hma = sma(high, high_length), lma = sma(low, low_length); direction = close > hma[1] ? 1 : close < lma[1] ? −1 : direction[1] (NaN until the first break; warm-up depends on the data); hilo = direction 1 ? lma : hma (current average; Sierra Chart plots the previous bar's average, pandas-ta holds the old value between breaks)", &[usize::MAX, usize::MAX], true, gann_hilo),
    def("ha_supertrend", "Heikin-Ashi SuperTrend", ST, &["supertrend", "direction"], PR, NO, "our supertrend computed on Heikin-Ashi bars (ha_close = ohlc4, ha_open = (ha_open[1] + ha_close[1])/2 seeded (open + close)/2, ha_high/low include ha_open/ha_close); hl2, close and the rma true range all from the HA bars (LonesomeTheBlue); fills still use real prices", &[9, 9], false, ha_supertrend),
    def("halftrend", "HalfTrend", HT, &["halftrend", "direction", "upper", "lower"], PR, NO, "everget Pine: high/low price = highest(high)/lowest(low) over amplitude, highma/lowma = sma(high/low, amplitude); nextTrend 1: maxLow = max(lowPrice, maxLow), flip to trend 1 when highma < maxLow and close < low[1] (minHigh = highPrice); else minHigh = min(highPrice, minHigh), flip to trend 0 when lowma > minHigh and close > high[1] (maxLow = lowPrice); trend 0: up = flip ? down[1] : max(maxLow, up[1]); trend 1: down = flip ? up[1] : min(minHigh, down[1]); maxLow/minHigh start at low/high of bar 0 (var … = nz(low[1], low)); before bar amplitude − 1 no flip is possible and min(highPrice, minHigh) stays high[0], so halftrend = low[0] there, as everget's code (Pine v4 2021 and v6 2025 versions, same logic); direction = trend (0 up, 1 down); upper/lower = halftrend ± channel_dev·atr(100)/2", &[0, 0, 99, 99], false, halftrend),
    def("kama_binary_wave", "Adaptive Binary Wave (Kaufman)", BW, &["wave"], PN, NO, "Sierra Chart MA-Adaptive Binary Wave (ACSIL source): ama = our kama(close, n, fast, slow); ama_low/ama_high = ama when it falls/rises, else held (start at the first ama); filter = pct/100·stdev(ama, n) (stdev of the AMA itself, as the source); wave = ama − ama_low > filter ? 1 : ama_high − ama > filter ? −1 : 0", &[19], false, kama_binary_wave),
    def("nrtr", "NRTR Channel", NR, &["buy", "sell", "trend"], PR, NO, "MetaTrader 5 NRTR_Channel.mq5: atr = sma(tr, n) (iATR); seed bar n−1: trend up if close > low (ceiling = close, buy = close) else down (floor = close, sell = close), MT5's ATR buffer being 0 there; uptrend: low > ceiling[1] → ceiling = close, buy = close − k·atr; else close < buy[1] → trend −1, floor = close, sell = close + k·atr; downtrend mirrors; otherwise carry; buy is NaN in downtrends and sell in uptrends (their first bar depends on the data)", &[usize::MAX, usize::MAX, 39], true, nrtr),
    def("ott", "Optimized Trend Tracker", OT, &["ott", "mavg", "direction"], PR, NO, "KivancOzbilgic Pine: mavg = VAR: cmo = nz((Σ9 up − Σ9 dn)/(Σ9 up + Σ9 dn)), VAR = α·|cmo|·close + (1 − α·|cmo|)·nz(VAR[1]), α = 2/(length + 1) (starts at 0 and ramps up, as in Pine; allow a warm-up); fark = mavg·percent/100; longStop = mavg > longStop[1] ? max(mavg − fark, longStop[1]) : mavg − fark; shortStop mirrors; direction (start 1) → 1 when mavg > shortStop[1], −1 when mavg < longStop[1]; mt = direction 1 ? longStop : shortStop; ott = mavg > mt ? mt·(200 + percent)/200 : mt·(200 − percent)/200 (Kivanc plots ott[2]; use ott[2] in rules for parity)", &[0, 0, 0], false, ott),
    def("pfe", "Polarized Fractal Efficiency", PF, &["pfe"], PN, NO, "MetaStock/Hannula form, k = n − 1: raw = ±100·√((close − close[k])² + k²)/Σ_{i=0..k−1} √((close[i] − close[i+1])² + 1), + if close > close[k] else −; pfe = ema(raw, smooth) (NinjaTrader uses k = n and omits ×100)", &[13], false, pfe),
    def("pp_supertrend", "Pivot Point SuperTrend", PP, &["line", "trend", "center"], PR, NO, "LonesomeTheBlue Pine: pivots = pivothigh(high, prd, prd)/pivotlow(low, prd, prd) dated at the confirming bar; center = first pivot, then (center·2 + pivot)/3; up = center − factor·atr, dn = center + factor·atr (atr = rma(tr)); tup = close[1] > tup[1] ? max(up, tup[1]) : up; tdown = close[1] < tdown[1] ? min(dn, tdown[1]) : dn; trend = close > tdown[1] ? 1 : close < tup[1] ? −1 : nz(trend[1], 1); line = trend 1 ? tup : tdown (line and center: warm-up depends on the first pivot)", &[usize::MAX, 0, usize::MAX], true, pp_supertrend),
    def("price_time_trend", "Price-Time Filtering (trend volume and bar count)", &[], &["trend", "trend_volume", "trend_bars"], PN, Needs { volume: true, ..Needs::NONE }, "thinkorswim PriceTimeFilteringAccVolume / BarCount: trend = close > high[1] ? 1 : close < low[1] ? −1 : trend[1] (NaN until the first break); trend_volume = trend·Σ volume and trend_bars = trend·bar count since the last trend change (both restart at the change bar; warm-up depends on the data)", &[usize::MAX, usize::MAX, usize::MAX], true, price_time_trend),
    def("range_filter", "Range Filter (Twin Range Filter)", RF, &["filt", "upper", "lower", "direction"], PR, NO, "colinmck Pine Twin Range Filter: smoothrng(t, m) = ema(ema(|close − close[1]|, t), 2t − 1)·m; r = (smoothrng(fast) + smoothrng(slow))/2; filt = close > nz(filt[1]) ? (close − r < nz(filt[1]) ? nz(filt[1]) : close − r) : (close + r > nz(filt[1]) ? nz(filt[1]) : close + r), so the first value is close − r; upper/lower = filt ± r; direction = +1 after filt rises, −1 after it falls, held while flat (NaN before the first move; its first bar depends on the data)", &[163, 163, 163, usize::MAX], true, range_filter),
    def("ssl", "SSL Channel", L10, &["ssl_up", "ssl_down", "direction"], PR, NO, "ErwinBeckers Pine: sma_h = sma(high, n), sma_l = sma(low, n); direction = close > sma_h ? 1 : close < sma_l ? −1 : direction[1] (NaN until the first break; warm-up depends on the data); ssl_down = direction < 0 ? sma_h : sma_l; ssl_up = direction < 0 ? sma_l : sma_h", &[9, 9, usize::MAX], true, ssl),
    def("stiffness", "Stiffness (Katsanos)", SF, &["stiffness"], PN, NO, "Katsanos (TASC Nov 2018): above = close > sma(close, ma_length) − k·stdev(close, ma_length) (population stdev); stiffness = ema(100·Σ_{length} above/length, smooth); level 90", &[160], false, stiffness),
    def("trend_magic", "Trend Magic", TM, &["magic", "direction"], PR, NO, "KivancOzbilgic Pine: atr = sma(tr, atr_length); c = cci(close, cci_length) (source close, not hlc3); magic = c ≥ 0 ? (low − mult·atr < nz(magic[1]) ? nz(magic[1]) : low − mult·atr) : (high + mult·atr > nz(magic[1]) ? nz(magic[1]) : high + mult·atr); Pine's nz seed 0 is held until the first c ≥ 0 bar and shown as NaN here (warm-up depends on the data); direction = c ≥ 0 ? 1 : −1", &[usize::MAX, 19], true, trend_magic),
    def("utbot", "UT Bot (ATR trailing stop)", UT, &["stop", "buy", "sell"], PR, NO, "QuantNomad Pine UT Bot Alerts: src = close (heikin 1: Heikin-Ashi close = ohlc4); nloss = key·atr(n) (rma of the real bars' true range); stop = src > stop[1] and src[1] > stop[1] ? max(stop[1], src − nloss) : src < stop[1] and src[1] < stop[1] ? min(stop[1], src + nloss) : src > stop[1] ? src − nloss : src + nloss (stop[1] nz → 0); buy = src > stop and src crosses above stop (1/0); sell mirrors", &[9, 9, 9], false, utbot),
    def("vhf", "Vertical Horizontal Filter", VH, &["vhf"], PN, NO, "(highest(close, n) − lowest(close, n)) / Σ_{n} |close − close[1]| (TA-Lib VHF: flat window → 0; thinkorswim uses high/low in the numerator)", &[28], false, vhf),
];
/// (id, platform names, AmiBroker names): one row per id in DEFS.
pub const SOURCES: &[(&str, &str, &str)] = &[
    ("alphatrend", "TradingView (community): AlphaTrend by KivancOzbilgic; pandas-ta: alphatrend; Dhan: AlphaTrend", ""),
    ("atr_trailing_stop", "thinkorswim: ATRTrailingStop", ""),
    ("clear_method", "Sierra Chart: Clear Method Swing Line", ""),
    ("dmh", "thinkorswim: DMH", ""),
    ("er", "TA-Lib: ER; pandas-ta: er; thinkorswim: EfficiencyRatio; Sierra Chart: Kaufman Efficiency Ratio", ""),
    ("gann_hilo", "Sierra Chart: Gann HiLo Activator; TradeStation: Gann HiLo Activator; MultiCharts: Gann HiLo Activator; thinkorswim: HiLoActivator; pandas-ta: hilo", ""),
    ("ha_supertrend", "TradingView (community): Supertrend MTF Heikin Ashi by LonesomeTheBlue; Dhan: Heikin-Ashi SuperTrend", ""),
    ("halftrend", "TradingView (community): HalfTrend by everget; Sierra Chart: HalfTrend; Dhan: HalfTrend", ""),
    ("kama_binary_wave", "Sierra Chart: Moving Average - Adaptive Binary Wave", ""),
    ("nrtr", "MetaTrader 5: NRTR Channel", ""),
    ("ott", "TradingView (community): Optimized Trend Tracker by KivancOzbilgic", ""),
    ("pfe", "MetaStock: PFE; NinjaTrader 8: PFE; TradeStation: Polarized Fractal Efficiency; thinkorswim: PolarizedFractalEfficiency; Sierra Chart: Polarized Fractal Efficiency", ""),
    ("pp_supertrend", "TradingView (community): Pivot Point SuperTrend by LonesomeTheBlue", ""),
    ("price_time_trend", "thinkorswim: PriceTimeFilteringAccVolume; thinkorswim: PriceTimeFilteringBarCount", ""),
    ("range_filter", "TradingView (community): Twin Range Filter by colinmck", ""),
    ("ssl", "TradingView (community): SSL Channel by ErwinBeckers; Dhan: SSL Channel", ""),
    ("stiffness", "thinkorswim: Stiffness", ""),
    ("trend_magic", "TradingView (community): Trend Magic by KivancOzbilgic", ""),
    ("utbot", "TradingView (community): UT Bot Alerts by QuantNomad; Dhan: UT Bot", ""),
    ("vhf", "TA-Lib: VHF; Tulip: vhf; pandas-ta: vhf; TradeStation: VHF; thinkorswim: VerticalHorizontalFilter; Sierra Chart: Vertical Horizontal Filter; MetaStock: VHF; cTrader: Vertical Horizontal Filter; Zerodha Kite (ChartIQ): VT HZ Filter", ""),
];


#[cfg(test)]
mod tests {
    use crate::lab::testdata::{assert_series, close_eq, closes, compiles, d1, from_ohlc, go, walk};
    const N: f64 = f64::NAN;
    /// Bars around `c` with high = c + 0.5, low = c − 0.5.
    fn band(c: &[f64]) -> crate::lab::bars::Bars { from_ohlc(&c.iter().map(|&x| (x, x + 0.5, x - 0.5, x)).collect::<Vec<_>>()) }
    fn rising(n: usize) -> Vec<f64> { (0..n).map(|i| 100. + i as f64).collect() }
    fn non_decreasing(x: &[f64]) -> bool { x.windows(2).all(|w| w[1] >= w[0]) }
    fn from(x: &[f64]) -> &[f64] { &x[x.iter().position(|v| !v.is_nan()).unwrap()..] }

    #[test] fn alphatrend_ratchets_up_while_bullish() {
        let o = go("alphatrend", &band(&rising(40)), &[]); // no volume -> rsi(close) = 100
        assert_eq!(o[0].iter().position(|v| !v.is_nan()), Some(14));
        assert!(non_decreasing(from(&o[0])));
        assert!(from(&o[1]).iter().all(|&s| s == 0.)); // a steadily rising line never re-crosses its own value 2 bars back
    }
    #[test] fn atr_trailing_stop_plain_tr_uses_wilder_atr() {
        let o = go("atr_trailing_stop", &d1(), &[("atr_length", 2.), ("modified", 0.)]);
        // tr1 = 2, tr2 = 1.5 -> rma 1.75; first trade long: 12 − 3.5·1.75
        assert_series(&o[0][..3], &[N, N, 5.875]);
        assert_eq!(o[1][2], 1.);
        assert_eq!(go("atr_trailing_stop", &d1(), &[("atr_length", 2.)])[0].iter().position(|v| !v.is_nan()), Some(2)); // modified: sma(H − L, 2) from bar 1, then rma(·, 2)
    }
    #[test] fn clear_method_rising_never_flips_then_turns() {
        let mut c = rising(10); c.push(95.); // high 95.5 < highest low 108.5
        let o = go("clear_method", &band(&c), &[]);
        assert!(o[1][..10].iter().all(|&d| d == 1.));
        assert_eq!(o[0][9], 108.5);
        assert_eq!((o[0][10], o[1][10]), (95.5, -1.));
    }
    #[test] fn dmh_up_moves_positive_and_rising() {
        let o = go("dmh", &band(&rising(40)), &[("length", 5.)]);
        let v = from(&o[0]);
        assert!(v.iter().all(|&x| x > 0.) && v.windows(2).all(|w| w[1] > w[0]));
        assert!(v[v.len() - 1] < 1.); // the ema of +DM = 1 approaches 1 from below
    }
    #[test] fn efficiency_ratio_extremes() {
        assert_series(&go("er", &closes(&[1., 2., 3., 2., 2., 2.]), &[("length", 2.)])[0], &[N, N, 1., 0., 1., 1.]);
    }
    #[test] fn gann_hilo_switches_side_on_breaks() {
        let b = from_ohlc(&[(9., 10., 9., 9.5), (10., 11.2, 10., 11.), (11., 11.5, 10.5, 10.8), (10., 10.4, 8., 9.)]);
        let o = go("gann_hilo", &b, &[("high_length", 1.), ("low_length", 1.)]);
        assert_series(&o[1], &[N, 1., 1., -1.]); // 11 > 10; 10.8 inside; 9 < 10.5
        assert_series(&o[0], &[N, 10., 10.5, 10.4]);
    }
    #[test] fn ha_supertrend_is_supertrend_on_heikin_ashi_bars() {
        let b = walk(300, 2, false);
        let (a, e) = (go("ha_supertrend", &b, &[]), go("supertrend", &super::heikin_ashi(&b), &[]));
        for k in 0..2 { assert_series(&a[k], &e[k]); }
        let h = super::heikin_ashi(&d1());
        assert!((h.close[0] - 10.125).abs() < 1e-12 && (h.open[1] - (10.25 + 10.125) / 2.).abs() < 1e-12);
    }
    #[test] fn halftrend_uptrend_ratchets() {
        let o = go("halftrend", &band(&rising(150)), &[]);
        assert!(o[1].iter().all(|&d| d == 0.));
        assert!(non_decreasing(&o[0]));
        assert_eq!(o[2].iter().position(|v| !v.is_nan()), Some(99));
        assert!((o[2][120] - o[0][120] - 2. * (o[0][120] - o[3][120]) / 2.).abs() < 1e-9);
    }
    #[test] fn halftrend_warmup_sits_on_the_first_low() {
        // amplitude 3: bars 0..1 cannot flip (lowma NaN) and minHigh stays high[0] → ht = up = low[0] = 9.5, trend 0 (everget Pine)
        let o = go("halftrend", &band(&[10., 12., 11., 13., 14.]), &[("amplitude", 3.)]);
        assert_series(&o[0][..2], &[9.5, 9.5]);
        assert_series(&o[1][..2], &[0., 0.]);
    }
    #[test] fn binary_wave_rising_is_plus_one() {
        let o = go("kama_binary_wave", &closes(&rising(60)), &[]);
        assert_eq!(o[0].iter().position(|v| !v.is_nan()), Some(19));
        assert!(from(&o[0]).iter().all(|&w| w == 1.));
        let mut f = rising(40); f.reverse();
        assert!(from(&go("kama_binary_wave", &closes(&f), &[])[0]).iter().all(|&w| w == -1.));
    }
    #[test] fn nrtr_uptrend_never_flips() {
        let o = go("nrtr", &band(&rising(30)), &[("atr_length", 3.)]);
        assert_eq!(o[2][2], 1.);
        assert!(o[2][2..].iter().all(|&t| t == 1.) && o[1].iter().all(|v| v.is_nan()));
        assert_eq!(o[0][2], 102.); // seed: close (MT5 ATR buffer 0)
        assert_eq!(o[0][3], 103. - 2. * 1.5); // tr = 1.5 on these bars
        assert!(non_decreasing(&o[0][3..])); // the seed bar sits at close because MT5's ATR is 0 there
    }
    #[test] fn ott_zero_percent_collapses_onto_mavg() {
        let c: Vec<f64> = walk(200, 3, false).close;
        let o = super::ott_s(&c, 2, 0.);
        assert_series(&o[0], &o[1]);
        // VAR on 10 bars: zero until the 9-bar CMO sum exists (Pine nz)
        let v = super::ott_s(&[1., 2., 3., 4., 5., 6., 7., 8., 9., 10.], 2, 1.4);
        assert_series(&v[1][..10], &[0., 0., 0., 0., 0., 0., 0., 0., 9. * 2. / 3., 9. * 2. / 3. / 3. + 10. * 2. / 3.]);
    }
    #[test] fn pfe_straight_line_is_100() {
        let up: Vec<f64> = (0..20).map(|i| 50. + 2. * i as f64).collect();
        let o = go("pfe", &closes(&up), &[("length", 4.), ("smooth", 2.)]);
        assert_eq!(o[0].iter().position(|v| !v.is_nan()), Some(4));
        assert!(from(&o[0]).iter().all(|v| (v - 100.).abs() < 1e-9));
        let dn: Vec<f64> = up.iter().rev().cloned().collect();
        assert!(from(&go("pfe", &closes(&dn), &[("length", 4.), ("smooth", 2.)])[0]).iter().all(|v| (v + 100.).abs() < 1e-9));
    }
    #[test] fn pp_supertrend_waits_for_a_confirmed_pivot() {
        let o = go("pp_supertrend", &band(&rising(30)), &[]);
        assert!(o[0].iter().all(|v| v.is_nan()) && o[2].iter().all(|v| v.is_nan()) && o[1].iter().all(|&t| t == 1.));
        let c = [10., 11., 15., 11., 10., 10.5, 10.2, 10.4, 10.3, 10.6, 10.5, 10.7, 10.6];
        let o = go("pp_supertrend", &band(&c), &[("atr_length", 2.)]);
        assert_eq!(o[2].iter().position(|v| !v.is_nan()), Some(4)); // pivot high 15.5 at bar 2, confirmed on bar 4
        assert_eq!(o[2][4], 15.5);
        assert_eq!(o[2][6], (15.5 * 2. + 9.5) / 3.); // pivot low 9.5 (bar 4) confirmed on bar 6
    }
    #[test] fn price_time_inside_bar_keeps_trend_and_adds_volume() {
        let mut b = from_ohlc(&[(9., 10., 9., 9.5), (10., 11.2, 10., 11.), (11., 11.5, 10.5, 10.8), (10., 10.4, 8., 9.)]);
        b.volume = Some(vec![5., 10., 20., 40.]);
        let o = go("price_time_trend", &b, &[]);
        assert_series(&o[0], &[N, 1., 1., -1.]);
        assert_series(&o[1], &[N, 10., 30., -40.]);
        assert_series(&o[2], &[N, 1., 2., -1.]);
    }
    #[test] fn range_filter_holds_inside_the_range() {
        let b = walk(500, 6, false);
        let o = go("range_filter", &b, &[]);
        let mut held = 0;
        for i in 164..500 {
            let (f1, r) = (o[0][i - 1], o[1][i] - o[0][i]);
            if b.close[i] - r < f1 && b.close[i] + r > f1 { assert_eq!(o[0][i], f1); held += 1; }
            if o[0][i] > f1 { assert_eq!(o[3][i], 1.); } else if o[0][i] < f1 { assert_eq!(o[3][i], -1.); } else { assert!(close_eq(o[3][i], o[3][i - 1])); }
        }
        assert!(held > 50);
    }
    #[test] fn ssl_between_the_averages_holds_direction() {
        let b = from_ohlc(&[(10., 11., 9., 10.), (10., 12., 10., 11.8), (12., 13., 11., 11.)]);
        let o = go("ssl", &b, &[("length", 2.)]);
        // bar 1: sma_h 11.5, sma_l 9.5, close 11.8 -> +1; bar 2: sma_h 12.5, sma_l 10.5, close 11 inside -> +1 held
        assert_series(&o[2], &[N, 1., 1.]);
        assert_series(&o[0], &[N, 11.5, 12.5]);
        assert_series(&o[1], &[N, 9.5, 10.5]);
    }
    #[test] fn stiffness_always_above_is_100() {
        let o = go("stiffness", &closes(&rising(40)), &[("length", 4.), ("ma_length", 5.), ("smooth", 2.)]);
        assert_eq!(o[0].iter().position(|v| !v.is_nan()), Some(4 + 3 + 1));
        assert!(from(&o[0]).iter().all(|&v| v == 100.));
    }
    #[test] fn trend_magic_bullish_never_decreases() {
        let o = go("trend_magic", &band(&rising(60)), &[]);
        assert_eq!(o[0].iter().position(|v| !v.is_nan()), Some(19)); // first cci >= 0 bar; before it Pine sits on its 0 seed
        assert!(non_decreasing(from(&o[0])) && from(&o[1]).iter().all(|&d| d == 1.));
        assert_eq!(o[0][19], 118.5 - 1.5); // low − sma(tr = 1.5, 5)
    }
    #[test] fn utbot_one_buy_after_the_turn_then_ratchets() {
        let mut c: Vec<f64> = (0..20).map(|i| 150. - 2. * i as f64).collect();
        c.extend((0..30).map(|i| 112. + 2. * i as f64));
        let o = go("utbot", &band(&c), &[]);
        assert_eq!(o[0].iter().position(|v| !v.is_nan()), Some(9));
        let buys: Vec<usize> = (0..c.len()).filter(|&i| o[1][i] == 1.).collect();
        assert_eq!(buys.len(), 1);
        assert!(non_decreasing(&o[0][buys[0]..]) && o[2][buys[0]..].iter().all(|&s| s == 0.));
    }
    #[test] fn vhf_windows_follow_ta_lib() {
        // range over n closes, path over n changes: a straight line gives (n − 1)/n; flat -> 0
        assert_series(&go("vhf", &closes(&[1., 2., 3., 4., 4., 4., 4.]), &[("length", 3.)])[0], &[N, N, N, 2. / 3., 1. / 2., 0., 0.]);
    }
    #[test] fn parameter_bounds_never_panic() {
        for bars in [walk(3, 1, true), walk(300, 1, true)] {
            for d in super::DEFS {
                for pick in [|p: &crate::lab::catalogue::ParamSpec| p.min, |p: &crate::lab::catalogue::ParamSpec| p.max] {
                    let kv: Vec<(&str, f64)> = d.params.iter().map(|p| (p.key, pick(p))).collect();
                    assert!(go(d.id, &bars, &kv).iter().all(|s| s.len() == bars.len()), "{}", d.id);
                }
            }
        }
    }
    /// Sparse entries skip the shared warm-up check: fixed warm-ups must still hold, data-dependent ones (usize::MAX)
    /// must start somewhere, and nothing may gap after its start except nrtr's one-sided buy/sell lines.
    #[test] fn own_warmup_fixed_or_data_dependent() {
        let bars = walk(1000, 7, true);
        for d in super::DEFS {
            for (k, s) in go(d.id, &bars, &[]).iter().enumerate() {
                let first = s.iter().position(|v| !v.is_nan()).unwrap_or_else(|| panic!("{}.{} never starts", d.id, d.outputs[k]));
                if d.first[k] != usize::MAX { assert_eq!(first, d.first[k], "{}.{}", d.id, d.outputs[k]); }
                if d.id != "nrtr" || k == 2 { assert!(s[first..].iter().all(|v| v.is_finite()), "{}.{} gap", d.id, d.outputs[k]); }
            }
        }
    }
    #[test] fn rules_can_read_every_output() {
        for d in super::DEFS { for o in d.outputs { compiles(d.id, o).unwrap_or_else(|e| panic!("{}.{o}: {e:?}", d.id)); } }
    }
}
