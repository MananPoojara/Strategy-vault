//! Cross-platform indicators, Cycles (2026-10-19 build list: TradingView, AmiBroker, MetaTrader, cTrader, NinjaTrader, TradeStation, thinkorswim, Sierra Chart, MetaStock). Formulas: see each definition string.
//! Ehlers conventions used throughout (his EasyLanguage takes trig arguments in degrees): a recursive filter starts on the first
//! bar whose inputs all exist and its earlier outputs count as 0 (EasyLanguage variables start at 0); a window statistic waits for
//! a full window; an output is NaN until every term it reads exists; `If x <> 0 Then y = …` keeps the previous y (initially 0).
use super::bars::Bars;
use super::catalogue::{int, n, p, real, IndicatorDef, Needs, Output, ParamSpec, Params, Placement};
use super::ta::{self, NA};
use std::f64::consts::PI;

const CAT: &str = "Cycles";
const fn def(id: &'static str, name: &'static str, params: &'static [ParamSpec], outputs: &'static [&'static str], placement: Placement,
             definition: &'static str, first: &'static [usize], f: fn(&Bars, &Params) -> Output) -> IndicatorDef {
    IndicatorDef { id, name, category: CAT, params, outputs, placement, needs: Needs::NONE, definition, first, sparse: false, compute: Some(f), note: "" }
}
const PN: Placement = Placement::Pane;
const PR: Placement = Placement::Price;

// ---------- shared Ehlers building blocks ----------
fn dg(x: f64) -> f64 { x.to_radians() }
/// First i ≥ lag with x[i−lag..=i] all finite (x.len() if none).
fn start(x: &[f64], lag: usize) -> usize { (lag..x.len()).find(|&i| x[i - lag..=i].iter().all(|v| v.is_finite())).unwrap_or(x.len()) }
/// y[i] = f(i, y[i−1], y[i−2]) from bar s on; earlier outputs count as 0 and are reported NaN.
fn rec(len: usize, s: usize, mut f: impl FnMut(usize, f64, f64) -> f64) -> Vec<f64> {
    let (mut out, mut y1, mut y2) = (vec![NA; len], 0., 0.);
    for i in s..len { let y = f(i, y1, y2); out[i] = y; y2 = y1; y1 = y; }
    out
}
/// Ehlers 2-pole SuperSmoother coefficients (c1, c2, c3): a1 = exp(−1.414·3.14159/P), c2 = 2a1·cos(1.414·180°/P), c3 = −a1², c1 = 1 − c2 − c3.
fn ss_coef(per: f64) -> (f64, f64, f64) {
    let a1 = (-1.414 * 3.14159 / per).exp(); let c2 = 2. * a1 * dg(1.414 * 180. / per).cos(); let c3 = -a1 * a1;
    (1. - c2 - c3, c2, c3)
}
fn ss(x: &[f64], per: f64) -> Vec<f64> {
    let (c1, c2, c3) = ss_coef(per);
    rec(x.len(), start(x, 1), |i, y1, y2| c1 * (x[i] + x[i - 1]) / 2. + c2 * y1 + c3 * y2)
}
/// Ehlers 2-pole high-pass (Cycle Analytics roofing listing): a = (cos(.707·360°/P) + sin(.707·360°/P) − 1)/cos(.707·360°/P).
fn hp2(x: &[f64], per: f64) -> Vec<f64> {
    let w = dg(0.707 * 360. / per); let a = (w.cos() + w.sin() - 1.) / w.cos(); let (k, b) = ((1. - a / 2.) * (1. - a / 2.), 1. - a);
    rec(x.len(), start(x, 2), |i, y1, y2| k * (x[i] - 2. * x[i - 1] + x[i - 2]) + 2. * b * y1 - b * b * y2)
}
/// Ehlers 1-pole high-pass: a = (cos(360°/P) + sin(360°/P) − 1)/cos(360°/P); hp = (1 − a/2)(x − x[1]) + (1 − a)hp[1].
fn hp1(x: &[f64], per: f64) -> Vec<f64> {
    let w = dg(360. / per); let a = (w.cos() + w.sin() - 1.) / w.cos();
    rec(x.len(), start(x, 1), |i, y1, _| (1. - a / 2.) * (x[i] - x[i - 1]) + (1. - a) * y1)
}
fn roof(x: &[f64], hp: f64, lp: f64) -> Vec<f64> { ss(&hp2(x, hp), lp) }
fn sq(x: &[f64]) -> Vec<f64> { x.iter().map(|v| v * v).collect() }
fn rms(x: &[f64], k: usize) -> Vec<f64> { ta::sma(&sq(x), k).iter().map(|v| v.sqrt()).collect() }
/// Highest/lowest over a full window of k finite values (NaN otherwise).
fn ext(x: &[f64], k: usize, max: bool) -> Vec<f64> {
    (0..x.len()).map(|i| if i + 1 < k { NA } else {
        let w = &x[i + 1 - k..=i];
        if w.iter().any(|v| v.is_nan()) { NA } else if max { w.iter().cloned().fold(f64::MIN, f64::max) } else { w.iter().cloned().fold(f64::MAX, f64::min) }
    }).collect()
}
/// (x − ll)/(hh − ll) over k; a zero range keeps the previous value (initially 0).
fn stoch01(x: &[f64], k: usize) -> Vec<f64> {
    let (hh, ll, mut prev) = (ext(x, k, true), ext(x, k, false), 0.);
    (0..x.len()).map(|i| if hh[i].is_nan() { NA } else { if hh[i] != ll[i] { prev = (x[i] - ll[i]) / (hh[i] - ll[i]); } prev }).collect()
}
/// Ehlers' Stochastic/Fisher wrappers (Cybernetic Analysis; Sierra Chart): s = (4v + 3v[1] + 2v[2] + v[3])/10 of v = stoch01;
/// stoch = 2(s − 0.5); fisher = 0.5·ln((1 + 1.98(s − 0.5))/(1 − 1.98(s − 0.5))).
fn stoch_fisher(x: &[f64], k: usize) -> (Vec<f64>, Vec<f64>) {
    let s = ta::wma(&stoch01(x, k), 4);
    (s.iter().map(|v| 2. * (v - 0.5)).collect(), s.iter().map(|v| 0.5 * ((1. + 1.98 * (v - 0.5)) / (1. - 1.98 * (v - 0.5))).ln()).collect())
}
fn lag1(x: &[f64]) -> Vec<f64> { (0..x.len()).map(|i| if i == 0 { NA } else { x[i - 1] }).collect() }
/// Ehlers' 2016–2019 band-pass coefficients (Fourier Series / Voss papers): L = cos(360°/P), G = cos(bw·360°/P), S = 1/G − sqrt(1/G² − 1).
fn bp_coef(per: f64, bw: f64) -> Result<(f64, f64), String> {
    if bw * 360. / per >= 90. { return Err(format!("bandwidth × 360 / period must stay below 90° (got {:.1}°)", bw * 360. / per)); }
    let g = dg(bw * 360. / per).cos();
    Ok((dg(360. / per).cos(), 1. / g - (1. / (g * g) - 1.).sqrt()))
}
/// BP = .5(1 − S)(close − close[2]) + L(1 + S)BP[1] − S·BP[2], forced to 0 on the first `zero` bars as in the papers.
fn bp_raw(c: &[f64], (l, s): (f64, f64), zero: usize) -> Vec<f64> {
    let mut v = vec![0.; c.len()];
    for i in zero.max(2)..c.len() { v[i] = 0.5 * (1. - s) * (c[i] - c[i - 2]) + l * (1. + s) * v[i - 1] - s * v[i - 2]; }
    v
}
fn mask(mut v: Vec<f64>, k: usize) -> Vec<f64> { for x in v.iter_mut().take(k) { *x = NA; } v }

// ---------- TA-Lib Hilbert transform family (port of ta_HT_DCPERIOD/DCPHASE/PHASOR/SINE/TRENDMODE.c, TA-Lib 0.8, unstable period 0) ----------
#[derive(Default, Clone, Copy)]
struct Hb { buf: [f64; 3], prev: f64, prev_in: f64 }
impl Hb {
    /// One Hilbert FIR stage (0.0962, 0.5769) on the odd- or even-bar circular buffers, scaled by 0.075·period[1] + 0.54.
    fn step(&mut self, idx: usize, input: f64, adj: f64) -> f64 {
        let t = 0.0962 * input;
        let mut v = -self.buf[idx]; self.buf[idx] = t; v += t; v -= self.prev;
        self.prev = 0.5769 * self.prev_in; v += self.prev; self.prev_in = input;
        v * adj
    }
}
struct Ht { period: Vec<f64>, inphase: Vec<f64>, quad: Vec<f64>, phase: Vec<f64>, sine: Vec<f64>, lead: Vec<f64>, trend: Vec<f64> }
/// `long` = the DC-phase family (34 warm-up steps after the WMA seed, lookback 63); otherwise HT_DCPERIOD/HT_PHASOR (9 steps, lookback 32).
fn ht(x: &[f64], long: bool) -> Ht {
    let len = x.len(); let lb = if long { 63 } else { 32 };
    let mut o = Ht { period: vec![NA; len], inphase: vec![NA; len], quad: vec![NA; len], phase: vec![NA; len], sine: vec![NA; len], lead: vec![NA; len], trend: vec![NA; len] };
    if len <= lb || x.iter().any(|v| !v.is_finite()) { return o; }
    let rad2deg = 45. / 1f64.atan(); let deg2rad = 1. / rad2deg; let c360 = 1f64.atan() * 8.;
    let (mut trail, mut sub, mut sum) = (0usize, x[0] + x[1] + x[2], x[0] + x[1] * 2. + x[2] * 3.);
    let (mut today, mut trail_v) = (3usize, 0.);
    // TA-Lib's incremental WMA(4): returns sum·0.1 before the sum is rolled forward
    let wma = |today: usize, trail: &mut usize, trail_v: &mut f64, sub: &mut f64, sum: &mut f64| -> f64 {
        let t = x[today]; *sub += t; *sub -= *trail_v; *sum += t * 4.; *trail_v = x[*trail]; *trail += 1;
        let s = *sum * 0.1; *sum -= *sub; s
    };
    for _ in 0..(if long { 34 } else { 9 }) { wma(today, &mut trail, &mut trail_v, &mut sub, &mut sum); today += 1; }
    let (mut ev, mut od) = ([Hb::default(); 4], [Hb::default(); 4]); // detrender, Q1, jI, jQ
    let (mut hi, mut period, mut prev_q2, mut prev_i2, mut re, mut im, mut smooth) = (0usize, 0., 0., 0., 0., 0., 0.);
    let (mut i1_ev3, mut i1_od3, mut i1_ev2, mut i1_od2) = (0., 0., 0., 0.);
    let (mut sp, mut sp_idx) = ([0f64; 50], 0usize);
    let (mut it1, mut it2, mut it3, mut days, mut dc_phase, mut sine, mut lead) = (0., 0., 0., 0i64, 0., 0., 0.);
    while today < len {
        let adj = 0.075f64.mul_add(period, 0.54);
        let sm = wma(today, &mut trail, &mut trail_v, &mut sub, &mut sum);
        sp[sp_idx] = sm;
        let (q2, i2, q1, i1);
        if today % 2 == 0 {
            let det = ev[0].step(hi, sm, adj); q1 = ev[1].step(hi, det, adj); i1 = i1_ev3;
            let ji = ev[2].step(hi, i1_ev3, adj); let jq = ev[3].step(hi, q1, adj);
            hi += 1; if hi == 3 { hi = 0; }
            q2 = 0.2f64.mul_add(q1 + ji, 0.8 * prev_q2); i2 = 0.2f64.mul_add(i1_ev3 - jq, 0.8 * prev_i2);
            i1_od3 = i1_od2; i1_od2 = det;
        } else {
            let det = od[0].step(hi, sm, adj); q1 = od[1].step(hi, det, adj); i1 = i1_od3;
            let ji = od[2].step(hi, i1_od3, adj); let jq = od[3].step(hi, q1, adj);
            q2 = 0.2f64.mul_add(q1 + ji, 0.8 * prev_q2); i2 = 0.2f64.mul_add(i1_od3 - jq, 0.8 * prev_i2);
            i1_ev3 = i1_ev2; i1_ev2 = det;
        }
        re = 0.8f64.mul_add(re, 0.2 * i2.mul_add(prev_i2, q2 * prev_q2));
        im = 0.8f64.mul_add(im, 0.2 * (i2 * prev_q2 - q2 * prev_i2));
        prev_q2 = q2; prev_i2 = i2;
        let prev = period;
        if im != 0. && re != 0. { period = 360. / ((im / re).atan() * rad2deg); }
        if period > 1.5 * prev { period = 1.5 * prev; }
        if period < 0.67 * prev { period = 0.67 * prev; }
        if period < 6. { period = 6.; } else if period > 50. { period = 50.; }
        period = 0.2f64.mul_add(period, 0.8 * prev);
        smooth = 0.67f64.mul_add(smooth, 0.33 * period);
        // dominant-cycle phase: DFT of the smoothed price over int(smoothPeriod + 0.5) bars
        let prev_phase = dc_phase;
        let k = if smooth + 0.5 >= 0. { (smooth + 0.5) as usize } else { 0 };
        let (mut rp, mut ip, mut idx) = (0., 0., sp_idx);
        for i in 0..k {
            let t = i as f64 * c360 / k as f64;
            rp += t.sin() * sp[idx]; ip += t.cos() * sp[idx];
            idx = if idx == 0 { 49 } else { idx - 1 };
        }
        if ip.abs() > 0. { dc_phase = (rp / ip).atan() * rad2deg; } else if ip.abs() <= 0.01 { if rp < 0. { dc_phase -= 90.; } else if rp > 0. { dc_phase += 90.; } }
        dc_phase += 90.; dc_phase += 360. / smooth;
        if ip < 0. { dc_phase += 180.; }
        if dc_phase > 315. { dc_phase -= 360.; }
        let (prev_sine, prev_lead) = (sine, lead);
        sine = (dc_phase * deg2rad).sin(); lead = ((dc_phase + 45.) * deg2rad).sin();
        // instantaneous trendline over the raw price (TA-Lib issue #88), then trend mode
        let mut tr = 0.;
        for j in 0..k.min(50) { if j <= today { tr += x[today - j]; } }
        if k > 0 { tr /= k as f64; }
        let trendline = (2f64.mul_add(it2, 4f64.mul_add(tr, 3. * it1)) + it3) / 10.;
        it3 = it2; it2 = it1; it1 = tr;
        let mut trend = 1.;
        if (sine > lead && prev_sine <= prev_lead) || (sine < lead && prev_sine >= prev_lead) { days = 0; trend = 0.; }
        days += 1;
        if (days as f64) < 0.5 * smooth { trend = 0.; }
        let dp = dc_phase - prev_phase;
        if smooth != 0. && dp > 0.67 * 360. / smooth && dp < 1.5 * 360. / smooth { trend = 0.; }
        if trendline != 0. && ((sp[sp_idx] - trendline) / trendline).abs() >= 0.015 { trend = 1.; }
        if today >= lb {
            o.period[today] = smooth; o.inphase[today] = i1; o.quad[today] = q1;
            o.phase[today] = dc_phase; o.sine[today] = sine; o.lead[today] = lead; o.trend[today] = trend;
        }
        sp_idx = if sp_idx == 49 { 0 } else { sp_idx + 1 };
        today += 1;
    }
    o
}
fn ht_dcperiod(b: &Bars, _: &Params) -> Output { Ok(vec![ht(&b.close, false).period]) }
fn ht_phasor(b: &Bars, _: &Params) -> Output { let h = ht(&b.close, false); Ok(vec![h.inphase, h.quad]) }
fn ht_dcphase(b: &Bars, _: &Params) -> Output { Ok(vec![ht(&b.close, true).phase]) }
fn ht_sine(b: &Bars, _: &Params) -> Output { let h = ht(&b.close, true); Ok(vec![h.sine, h.lead]) }
fn ht_trendmode(b: &Bars, _: &Params) -> Output { Ok(vec![ht(&b.close, true).trend]) }

// ---------- Ehlers filters and oscillators ----------
fn ehlers_hp(b: &Bars, prm: &Params) -> Output {
    let per = p(prm, "length");
    Ok(vec![if n(prm, "poles") == 1 { hp1(&b.close, per) } else { hp2(&b.close, per) }])
}
fn roofing(b: &Bars, prm: &Params) -> Output { Ok(vec![roof(&b.close, p(prm, "hp_length"), p(prm, "ss_length"))]) }
fn bandpass(b: &Bars, prm: &Params) -> Output {
    let bp = mask(bp_raw(&b.close, bp_coef(p(prm, "length"), p(prm, "bandwidth"))?, 3), 3);
    let trig = lag1(&bp);
    Ok(vec![bp, trig])
}
fn swak_highpass(b: &Bars, prm: &Params) -> Output {
    let (x, k, mode) = (&b.close, p(prm, "length"), n(prm, "mode"));
    let (c0, a1, a2) = match mode {
        0 => { let h = PI / k; let (sh, ch) = (h.sin(), h.cos()); let a1p = 2. * sh * (ch - sh) / (1. - 2. * sh * sh); (1. - a1p / 2., 1. - a1p, 0.) }
        1 => { let s = (PI / k).sin(); let b2p = 2.415 * (2. * s * s); let a2p = -b2p + b2p.mul_add(b2p, 2. * b2p).sqrt(); let om = 1. - a2p;
               ((1. - a2p / 2.) * (1. - a2p / 2.), 2. * om, -(om * om)) }
        _ => { let t = 4. * PI * p(prm, "delta") / k; let abp = (1. - t.sin()) / t.cos(); ((1. - abp) / 2., (2. * PI / k).cos() * (1. + abp), -abp) }
    };
    let mut out = vec![NA; x.len()];
    if x.is_empty() { return Ok(vec![out]); }
    // TA-Lib seeds both input slots with the first bar and both output slots with 0, and publishes from bar 0
    let (mut x1, mut x2, mut y1, mut y2) = (x[0], x[0], 0., 0.);
    for i in 0..x.len() {
        let y = match mode { 0 => a1.mul_add(y1, c0 * (x[i] - x1)), 1 => a1.mul_add(y1, a2.mul_add(y2, c0 * (x[i] - 2. * x1 + x2))), _ => a1.mul_add(y1, a2.mul_add(y2, c0 * (x[i] - x2))) };
        x2 = x1; x1 = x[i]; y2 = y1; y1 = y; out[i] = y;
    }
    Ok(vec![out])
}
fn ebsw(b: &Bars, prm: &Params) -> Output {
    let w = dg(360. / p(prm, "hp_length")); let a = (1. - w.sin()) / w.cos(); let c = &b.close;
    let hp = rec(c.len(), start(c, 1), |i, y1, _| 0.5 * (1. + a) * (c[i] - c[i - 1]) + a * y1);
    let f = ss(&hp, p(prm, "ss_length"));
    let (wave, pwr) = (ta::sma(&f, 3), ta::sma(&sq(&f), 3));
    Ok(vec![wave.iter().zip(&pwr).map(|(w, q)| if w.is_nan() { NA } else if *q == 0. { 0. } else { w / q.sqrt() }).collect()])
}
fn super_passband(b: &Bars, prm: &Params) -> Output {
    let (a1, a2, c) = (5. / p(prm, "fast"), 5. / p(prm, "slow"), &b.close);
    let pb = rec(c.len(), start(c, 1), |i, y1, y2| (a1 - a2) * c[i] + (a2 * (1. - a1) - a1 * (1. - a2)) * c[i - 1] + ((1. - a1) + (1. - a2)) * y1 - (1. - a1) * (1. - a2) * y2);
    let r = rms(&pb, n(prm, "rms_length"));
    let lo = r.iter().map(|v| -v).collect();
    Ok(vec![pb, r, lo])
}
fn decycler_osc(b: &Bars, prm: &Params) -> Output {
    let (per, k, c) = (p(prm, "hp_length"), p(prm, "k"), &b.close);
    let dec: Vec<f64> = c.iter().zip(&hp2(c, per)).map(|(c, h)| c - h).collect();
    Ok(vec![hp2(&dec, 0.5 * per).iter().zip(c).map(|(o, c)| if *c == 0. { NA } else { 100. * k * o / c }).collect()])
}
fn dso(b: &Bars, prm: &Params) -> Output {
    let (k, c) = (n(prm, "length"), &b.close);
    let f = ss(&ta::change(c, 2), 0.5 * k as f64);
    let r = rms(&f, k);
    let (mut scaled, mut fish) = (0., 0.);
    Ok(vec![(0..c.len()).map(|i| if r[i].is_nan() { NA } else {
        if r[i] != 0. { scaled = f[i] / r[i]; }
        if scaled.abs() < 2. { fish = 0.5 * ((1. + scaled / 2.) / (1. - scaled / 2.)).ln(); }
        fish
    }).collect()])
}
fn ehlers_am_fm(b: &Bars, prm: &Params) -> Output {
    let d: Vec<f64> = (0..b.len()).map(|i| b.close[i] - b.open[i]).collect();
    let am = ta::sma(&ta::highest(&d.iter().map(|v| v.abs()).collect::<Vec<_>>(), n(prm, "env_length")), n(prm, "avg_length"));
    let hl: Vec<f64> = d.iter().map(|v| (10. * v).clamp(-1., 1.)).collect();
    let (c1, c2, c3) = ss_coef(p(prm, "ss_length"));
    // Ehlers: If CurrentBar < 3 Then SS = Deriv
    let fm = rec(d.len(), 0, |i, y1, y2| if i < 2 { d[i] } else { c1 * (hl[i] + hl[i - 1]) / 2. + c2 * y1 + c3 * y2 });
    Ok(vec![am, fm])
}
fn ehlers_leading(b: &Bars, prm: &Params) -> Output {
    let (x, a1, a2) = (b.hl2(), p(prm, "alpha1"), p(prm, "alpha2"));
    let lead = rec(x.len(), start(&x, 1), |i, y1, _| 2. * x[i] + (a1 - 2.) * x[i - 1] + (1. - a1) * y1);
    let net = rec(x.len(), start(&lead, 0), |i, y1, _| a2 * lead[i] + (1. - a2) * y1);
    let ema = rec(x.len(), start(&x, 0), |i, y1, _| 0.5 * x[i] + 0.5 * y1);
    Ok(vec![net, ema])
}
fn ehlers_stoch(b: &Bars, prm: &Params) -> Output {
    let lp = p(prm, "ss_length");
    Ok(vec![ss(&stoch01(&roof(&b.close, p(prm, "hp_length"), lp), n(prm, "length")), lp)])
}
fn elegant_osc(b: &Bars, prm: &Params) -> Output {
    let d = ta::change(&b.close, 2); let r = rms(&d, n(prm, "rms_length"));
    // (e^{2x} − 1)/(e^{2x} + 1) written as tanh(x): identical, and finite for large |x|
    let ifish: Vec<f64> = d.iter().zip(&r).map(|(d, r)| if r.is_nan() { NA } else if *r == 0. { 0. } else { (d / r).tanh() }).collect();
    let fir6 = |x: &[f64]| -> Vec<f64> { (0..x.len()).map(|i| if i < 5 { NA } else { (x[i] + 2. * x[i - 1] + 3. * x[i - 2] + 3. * x[i - 3] + 2. * x[i - 4] + x[i - 5]) / 12. }).collect() };
    // Ehlers' published hard limiter clips the raw derivative, not the RMS-normalised one
    Ok(vec![ss(&ifish, p(prm, "ss_length")), fir6(&ifish), fir6(&d.iter().map(|v| v.clamp(-1., 1.)).collect::<Vec<_>>())])
}
fn fourier_series(b: &Bars, prm: &Params) -> Output {
    let (f, fk, bw, c) = (p(prm, "fundamental"), n(prm, "fundamental"), p(prm, "bandwidth"), &b.close);
    let (mut bps, mut pw) = (Vec::new(), Vec::new());
    for h in 1..=3 {
        let bp = bp_raw(c, bp_coef(f / h as f64, bw)?, 3);
        let q: Vec<f64> = (0..c.len()).map(|i| if i <= 3 { 0. } else { (f / 6.28) * (bp[i] - bp[i - 1]) }).collect();
        pw.push(ta::sum(&bp.iter().zip(&q).map(|(b, q)| b * b + q * q).collect::<Vec<_>>(), fk));
        bps.push(bp);
    }
    let mut w = 0.;
    let wave: Vec<f64> = (0..c.len()).map(|i| if pw[0][i].is_nan() { NA } else {
        if pw[0][i] != 0. { w = bps[0][i] + (pw[1][i] / pw[0][i]).sqrt() * bps[1][i] + (pw[2][i] / pw[0][i]).sqrt() * bps[2][i]; }
        w
    }).collect();
    let roc = ta::change(&wave, 2).iter().map(|v| f / 12.57 * v).collect();
    Ok(vec![wave, roc])
}
fn onset_trend(b: &Bars, prm: &Params) -> Output {
    let (f, k) = (roof(&b.close, p(prm, "hp_length"), p(prm, "ss_length")), p(prm, "k"));
    let (mut peak, mut x) = (0., 0.);
    Ok(vec![f.iter().map(|&v| if v.is_nan() { NA } else {
        peak *= 0.991; if v.abs() > peak { peak = v.abs(); }
        if peak != 0. { x = v / peak; }
        let den = k * x + 1.; if den == 0. { NA } else { (x + k) / den }
    }).collect()])
}
fn flex(b: &Bars, prm: &Params, reflex: bool) -> Output {
    let k = n(prm, "length");
    let f = ss(&b.close, 0.5 * k as f64);
    let mut ms = 0.;
    Ok(vec![(0..f.len()).map(|i| if i < k || f[i - k].is_nan() { NA } else {
        let slope = (f[i - k] - f[i]) / k as f64;
        let s = (1..=k).map(|j| if reflex { f[i] + j as f64 * slope - f[i - j] } else { f[i] - f[i - j] }).sum::<f64>() / k as f64;
        ms = 0.04 * s * s + 0.96 * ms;
        if ms != 0. { s / ms.sqrt() } else { 0. }
    }).collect()])
}
fn reflex(b: &Bars, prm: &Params) -> Output { flex(b, prm, true) }
fn trendflex(b: &Bars, prm: &Params) -> Output { flex(b, prm, false) }
fn reverse_ema(b: &Bars, prm: &Params) -> Output {
    let (a, c) = (p(prm, "alpha"), &b.close); let cc = 1. - a;
    let ema = rec(c.len(), start(c, 0), |i, y1, _| a * c[i] + cc * y1);
    let mut re = ema.clone();
    for k in 0..8 {
        let g = cc.powi(1 << k); let prev = re;
        re = rec(c.len(), start(&prev, 1), |i, _, _| g * prev[i] + prev[i - 1]);
    }
    Ok(vec![ema.iter().zip(&re).map(|(e, r)| e - a * r).collect()])
}
fn rmo_ehlers(b: &Bars, prm: &Params) -> Output {
    let c = &b.close; let w = dg(360. / p(prm, "lp_length")); let a = (w.cos() + w.sin() - 1.) / w.cos();
    let med: Vec<f64> = (0..c.len()).map(|i| if i < 4 { NA } else { let mut v = c[i - 4..=i].to_vec(); v.sort_by(|x, y| x.total_cmp(y)); v[2] }).collect();
    let rm = rec(c.len(), start(&med, 0), |i, y1, _| a * med[i] + (1. - a) * y1);
    Ok(vec![hp2(&rm, p(prm, "hp_length"))])
}
fn universal_osc(b: &Bars, prm: &Params) -> Output {
    let wn: Vec<f64> = ta::change(&b.close, 2).iter().map(|v| v / 2.).collect();
    // Ehlers sets Peak = 1e−7 on bar 1 and decays it .991 per bar; his Filt is 0 on bars 1–3, so bar 4 sees .991³·1e−7
    let (mut pk, mut u) = (0.0000001 * 0.991 * 0.991, 0.);
    Ok(vec![ss(&wn, p(prm, "band_edge")).iter().map(|&v| if v.is_nan() { NA } else {
        pk *= 0.991; if v.abs() > pk { pk = v.abs(); }
        if pk != 0. { u = v / pk; }
        u
    }).collect()])
}
fn voss(b: &Bars, prm: &Params) -> Output {
    let filt = bp_raw(&b.close, bp_coef(p(prm, "length"), p(prm, "bandwidth"))?, 5);
    let order = 3 * n(prm, "predict");
    let mut v = vec![0.; filt.len()];
    for i in 0..filt.len() {
        let sumc: f64 = (0..order).map(|c| if i >= order - c { (c + 1) as f64 / order as f64 * v[i - (order - c)] } else { 0. }).sum();
        v[i] = (3 + order) as f64 / 2. * filt[i] - sumc;
    }
    Ok(vec![mask(filt, 5), mask(v, 5)])
}
fn cyber(x: &[f64], alpha: impl Fn(usize) -> f64) -> Vec<f64> {
    let s: Vec<f64> = (0..x.len()).map(|i| if i < 3 { NA } else { (x[i] + 2. * x[i - 1] + 2. * x[i - 2] + x[i - 3]) / 6. }).collect();
    rec(x.len(), start(x, 2), |i, y1, y2| if i < 6 { (x[i] - 2. * x[i - 1] + x[i - 2]) / 4. } else {
        let a = alpha(i);
        (1. - 0.5 * a) * (1. - 0.5 * a) * (s[i] - 2. * s[i - 1] + s[i - 2]) + 2. * (1. - a) * y1 - (1. - a) * (1. - a) * y2
    })
}
fn cyber_cycle(b: &Bars, prm: &Params) -> Output {
    let (a, x) = (p(prm, "alpha"), b.hl2());
    let dc = if n(prm, "adaptive") == 1 { ht(&b.close, false).period } else { vec![NA; x.len()] };
    let c = cyber(&x, |i| if dc[i].is_nan() { a } else { 2. / (dc[i] + 1.) });
    let (s, f) = stoch_fisher(&c, n(prm, "stoch_length"));
    let trig = lag1(&c);
    Ok(vec![c, trig, s, f])
}
/// Window length per bar: the fixed length, or int(ht_dcperiod/2) (≥ 1) when adaptive (None until the period exists).
fn lengths(b: &Bars, prm: &Params) -> Vec<Option<usize>> {
    let k = n(prm, "length");
    if n(prm, "adaptive") == 0 { return vec![Some(k); b.len()]; }
    ht(&b.close, false).period.iter().map(|v| if v.is_nan() { None } else { Some(((v / 2.) as usize).max(1)) }).collect()
}
fn cg_ext(b: &Bars, prm: &Params) -> Output {
    let (h, ks, adapt) = (b.hl2(), lengths(b, prm), n(prm, "adaptive") == 1);
    let mut cg = 0.;
    let v: Vec<f64> = (0..h.len()).map(|t| match ks[t] { Some(k) if t + 1 >= k => {
        let (num, den) = (0..k).fold((0., 0.), |(a, d), i| (a + (i + 1) as f64 * h[t - i], d + h[t - i]));
        // Ehlers' Adaptive CG listing re-centres by (n + 1)/2 because n changes; a fixed n keeps cg_osc's form
        if den != 0. { cg = -num / den + if adapt { (k as f64 + 1.) / 2. } else { 0. }; } cg
    } _ => NA }).collect();
    let (s, f) = stoch_fisher(&v, n(prm, "stoch_length"));
    Ok(vec![v, s, f])
}
fn rvgi_ext(b: &Bars, prm: &Params) -> Output {
    let ks = lengths(b, prm);
    let sw = |f: &dyn Fn(usize) -> f64| -> Vec<f64> { (0..b.len()).map(|i| if i < 3 { NA } else { (f(i) + 2. * f(i - 1) + 2. * f(i - 2) + f(i - 3)) / 6. }).collect() };
    let (num, den) = (sw(&|i| b.close[i] - b.open[i]), sw(&|i| b.high[i] - b.low[i]));
    let mut r = 0.;
    let v: Vec<f64> = (0..b.len()).map(|t| match ks[t] { Some(k) if t >= k + 2 => {
        let (a, d) = (0..k).fold((0., 0.), |(a, d), i| (a + num[t - i], d + den[t - i]));
        if d != 0. { r = a / d; } r
    } _ => NA }).collect();
    let (s, f) = stoch_fisher(&v, n(prm, "stoch_length"));
    Ok(vec![v, s, f])
}
fn correlation_cycle(b: &Bars, prm: &Params) -> Output {
    let (k, thr, c) = (n(prm, "length"), p(prm, "threshold"), &b.close);
    let pearson = |t: usize, y: &dyn Fn(usize) -> f64| -> Option<f64> {
        let (mut sx, mut sy, mut sxx, mut sxy, mut syy) = (0., 0., 0., 0., 0.);
        for j in 0..k { let (x, y) = (c[t - j], y(j)); sx += x; sy += y; sxx += x * x; sxy += x * y; syy += y * y; }
        let (l, vx, vy) = (k as f64, k as f64 * sxx - sx * sx, k as f64 * syy - sy * sy);
        if vx > 0. && vy > 0. { Some((l * sxy - sx * sy) / (vx * vy).sqrt()) } else { None }
    };
    let len = c.len(); let (mut re, mut imv, mut ang, mut st) = (vec![NA; len], vec![NA; len], vec![NA; len], vec![NA; len]);
    let (mut r, mut im, mut a) = (0., 0., 0.);
    for t in k.saturating_sub(1)..len {
        let pa = a;
        if let Some(v) = pearson(t, &|j| dg(360. * j as f64 / k as f64).cos()) { r = v; }
        if let Some(v) = pearson(t, &|j| -dg(360. * j as f64 / k as f64).sin()) { im = v; }
        if im != 0. { a = 90. + (r / im).atan().to_degrees(); }
        if im > 0. { a -= 180.; }
        if pa - a < 270. && a < pa { a = pa; }
        re[t] = r; imv[t] = im; ang[t] = a;
        if t >= k { st[t] = if (a - pa).abs() < thr { if a < 0. { -1. } else { 1. } } else { 0. }; }
    }
    Ok(vec![re, imv, ang, st])
}

// ---------- parameters ----------
const NOP: &[ParamSpec] = &[];
const BPP: &[ParamSpec] = &[int("length", "Length", 20., 2., 2000.), real("bandwidth", "Bandwidth", 0.3, 0.01, 1., 0.01)];
const CGP: &[ParamSpec] = &[int("length", "CG length", 8., 1., 2000.), int("adaptive", "Adaptive length (0 off, 1 half dominant cycle)", 0., 0., 1.), int("stoch_length", "Stochastic length", 8., 1., 2000.)];
const CCP: &[ParamSpec] = &[int("length", "Length", 20., 4., 2000.), real("threshold", "State threshold (deg)", 9., 0., 90., 0.5)];
const CYP: &[ParamSpec] = &[real("alpha", "Alpha", 0.07, 0.01, 1., 0.01), int("adaptive", "Adaptive alpha (0 off, 1 dominant cycle)", 0., 0., 1.), int("stoch_length", "Stochastic/Fisher length", 8., 1., 2000.)];
const DOP: &[ParamSpec] = &[int("hp_length", "Decycler period", 125., 4., 2000.), real("k", "Scale k", 1., 0.01, 100., 0.01)];
const DSP: &[ParamSpec] = &[int("length", "Length", 40., 2., 2000.)];
const EBP: &[ParamSpec] = &[int("hp_length", "High-pass period", 40., 5., 2000.), int("ss_length", "SuperSmoother period", 10., 2., 2000.)]; // hp 4: cos(90°) = 0
const AMP: &[ParamSpec] = &[int("ss_length", "FM SuperSmoother period", 30., 2., 2000.), int("env_length", "AM envelope length", 4., 1., 2000.), int("avg_length", "AM average length", 8., 1., 2000.)];
const HPP: &[ParamSpec] = &[int("length", "Length", 48., 5., 2000.), int("poles", "Poles (1 or 2)", 2., 1., 2.)]; // 1-pole length 4: cos(90°) = 0
const LDP: &[ParamSpec] = &[real("alpha1", "Alpha 1", 0.25, 0.01, 1., 0.01), real("alpha2", "Alpha 2", 0.33, 0.01, 1., 0.01)];
const ESP: &[ParamSpec] = &[int("length", "Stochastic length", 20., 1., 2000.), int("hp_length", "Roofing high-pass", 48., 2., 2000.), int("ss_length", "Roofing SuperSmoother", 10., 2., 2000.)];
const ELP: &[ParamSpec] = &[int("rms_length", "RMS length", 50., 1., 2000.), int("ss_length", "Band edge (SuperSmoother period)", 20., 2., 2000.)];
const FSP: &[ParamSpec] = &[int("fundamental", "Fundamental period", 20., 6., 2000.), real("bandwidth", "Bandwidth", 0.1, 0.01, 1., 0.01)];
const OTP: &[ParamSpec] = &[int("hp_length", "High-pass period", 100., 2., 2000.), int("ss_length", "SuperSmoother period (LPPeriod)", 30., 2., 2000.), real("k", "K", 0.85, -1., 1., 0.01)];
const L20: &[ParamSpec] = &[int("length", "Length", 20., 2., 2000.)];
const REP: &[ParamSpec] = &[real("alpha", "Alpha", 0.1, 0.01, 0.99, 0.01)];
const RMP: &[ParamSpec] = &[int("lp_length", "Low-pass period", 12., 5., 2000.), int("hp_length", "High-pass period", 30., 2., 2000.)]; // lp 4: cos(90°) = 0
const RFP: &[ParamSpec] = &[int("hp_length", "High-pass period", 48., 2., 2000.), int("ss_length", "SuperSmoother period", 10., 2., 2000.)];
const RVP: &[ParamSpec] = &[int("length", "RVI length", 10., 1., 2000.), int("adaptive", "Adaptive length (0 off, 1 half dominant cycle)", 0., 0., 1.), int("stoch_length", "Stochastic length", 10., 1., 2000.)];
const SPP: &[ParamSpec] = &[int("fast", "Fast period", 40., 5., 2000.), int("slow", "Slow period", 60., 5., 2000.), int("rms_length", "RMS length", 50., 1., 2000.)]; // period < 5: alpha > 1
const SWP: &[ParamSpec] = &[int("length", "Length", 20., 5., 2000.), int("mode", "Mode (0 HP 1-pole, 1 HP 2-pole, 2 band-pass)", 0., 0., 2.), real("delta", "Band-pass delta", 0.1, 0.05, 0.5, 0.01)]; // TA-Lib's valid ranges
const UOP: &[ParamSpec] = &[int("band_edge", "Band edge", 20., 2., 2000.)];
const VSP: &[ParamSpec] = &[int("length", "Length", 20., 2., 2000.), int("predict", "Predict bars", 3., 1., 20.), real("bandwidth", "Bandwidth", 0.25, 0.01, 1., 0.01)];

pub const DEFS: &[IndicatorDef] = &[
    def("bandpass", "Ehlers Bandpass Filter", BPP, &["bp", "trigger"], PN, "L = cos(360°/P), G = cos(bw·360°/P), S = 1/G − √(1/G² − 1); bp = .5(1 − S)(close − close[2]) + L(1 + S)bp[1] − S·bp[2], 0 on the first 3 bars (Ehlers, Fourier Series Model of the Market, 2019); trigger = bp[1] (declared). Needs bw·360/P < 90°", &[3, 4], bandpass),
    def("cg_ext", "Center of Gravity: Adaptive / Stochastic / Fisher", CGP, &["cg", "stoch", "fisher"], PN, "cg = −Σ(i+1)·hl2[i]/Σhl2[i] over n (Ehlers, The CG Oscillator; adaptive: n = int(ht_dcperiod/2), declared substitute for Ehlers' cycle period); v = (cg − ll)/(hh − ll) over stoch_length (Max = Min keeps v), s = (4v + 3v[1] + 2v[2] + v[3])/10; stoch = 2(s − .5); fisher = .5·ln((1 + 1.98(s − .5))/(1 − 1.98(s − .5))) (Ehlers, Cybernetic Analysis for Stocks and Futures, 2004: Stochastic CG / Fisher CG listings, Length 8 for both; adaptive adds (n + 1)/2 as his Adaptive CG listing)", &[7, 17, 17], cg_ext),
    def("correlation_cycle", "Ehlers Correlation Cycle", CCP, &["real", "imag", "angle", "state"], PN, "real/imag = Pearson correlation of close[0..n−1] with cos(360°·j/n) and −sin(360°·j/n) (degenerate window keeps the value); angle = 90 + atan(real/imag)° (−180 if imag > 0), held when it would fall by less than 270°; state = ±1 (sign of angle) when |angle − angle[1]| < threshold else 0 (Ehlers, Correlation as a Cycle Indicator, TASC Jun 2020)", &[19, 19, 19, 20], correlation_cycle),
    def("cyber_cycle", "Ehlers Cyber Cycle", CYP, &["cycle", "trigger", "stoch", "fisher"], PN, "x = hl2; s = (x + 2x[1] + 2x[2] + x[3])/6; cycle = (1 − a/2)²(s − 2s[1] + s[2]) + 2(1 − a)cycle[1] − (1 − a)²cycle[2], (x − 2x[1] + x[2])/4 on the first 6 bars; trigger = cycle[1] (Ehlers, Cybernetic Analysis for Stocks and Futures, 2004: Cyber Cycle, Stochastic / Fisher Cyber Cycle listings, alpha .07, Len 8). Adaptive: a = 2/(ht_dcperiod + 1) once it exists (declared substitute for his cycle period). stoch/fisher: Ehlers' wrappers as cg_ext", &[2, 3, 12, 12], cyber_cycle),
    def("decycler_osc", "Ehlers Decycler Oscillator", DOP, &["osc"], PN, "dec = close − hp2(close, P); osc = 100·k·hp2(dec, P/2)/close, hp2 = Ehlers 2-pole high-pass (Ehlers, Decyclers, TASC Sep 2015, Decycler Oscillator listing, HPPeriod 125, K 1)", &[4], decycler_osc),
    def("dso", "Ehlers Deviation-Scaled Oscillator", DSP, &["dso"], PN, "f = SuperSmoother(close − close[2], n/2); x = f/√(Σf²/n) over n (RMS 0 keeps x); dso = .5·ln((1 + x/2)/(1 − x/2)) while |x| < 2, else held (Ehlers, Probability—Probably A Good Thing To Know, TASC Oct 2018: Fisherized Deviation Scaled Oscillator listing, Period 40)", &[42], dso),
    def("ebsw", "Ehlers Even Better Sinewave", EBP, &["wave"], PN, "a = (1 − sin(360°/hp))/cos(360°/hp); hp = .5(1 + a)(close − close[1]) + a·hp[1]; f = SuperSmoother(hp, ss); wave = ((f + f[1] + f[2])/3)/√((f² + f[1]² + f[2]²)/3), 0 when the power is 0 (Ehlers, Cycle Analytics for Traders, 2013, as reprinted on ProRealCode as Even better Sinewave; Duration 40, SuperSmoother 10)", &[4], ebsw),
    def("ehlers_am_fm", "Ehlers AM Detector and FM Demodulator", AMP, &["am", "fm"], PN, "d = close − open; am = sma(highest(|d|, env), avg); fm = SuperSmoother(clamp(10d, −1, 1), P), = d on the first 2 bars (Ehlers, AM and FM components, TASC May 2021)", &[10, 0], ehlers_am_fm),
    def("ehlers_hp", "Ehlers High-Pass Filter", HPP, &["hp"], PN, "1-pole: a = (cos(360°/P) + sin(360°/P) − 1)/cos(360°/P), hp = (1 − a/2)(x − x[1]) + (1 − a)hp[1]; 2-pole: a with .707·360°/P, hp = (1 − a/2)²(x − 2x[1] + x[2]) + 2(1 − a)hp[1] − (1 − a)²hp[2] (Ehlers, Predictive Indicators, TASC Jan 2014)", &[2], ehlers_hp),
    def("ehlers_leading", "Ehlers Leading Indicator", LDP, &["netlead", "ema"], PR, "x = hl2; lead = 2x + (a1 − 2)x[1] + (1 − a1)lead[1]; netlead = a2·lead + (1 − a2)netlead[1]; ema = .5x + .5ema[1] (Ehlers, Cybernetic Analysis for Stocks and Futures, 2004: Leading Indicator listing, alpha1 .25, alpha2 .33; zero-initialised as in his EasyLanguage)", &[1, 0], ehlers_leading),
    def("ehlers_stoch", "Ehlers Stochastic (roofing-filtered)", ESP, &["stoch"], PN, "f = SuperSmoother(hp2(close, hp), ss); raw = (f − ll(f, n))/(hh(f, n) − ll(f, n)) (zero range keeps raw); stoch = SuperSmoother(raw, ss) (Ehlers, Predictive And Successful Indicators, TASC Jan 2014: SuperSmoothStoch listing, Length 20, high-pass 48, SuperSmoother 10)", &[23], ehlers_stoch),
    def("elegant_osc", "Ehlers Elegant Oscillator (soft/hard limiter)", ELP, &["elegant", "soft", "hard"], PN, "d = close − close[2]; ifish = (e^{2x} − 1)/(e^{2x} + 1) of x = d/√(Σd²/n) (0 when RMS is 0); elegant = SuperSmoother(ifish, band edge); fir6(v) = (v + 2v[1] + 3v[2] + 3v[3] + 2v[4] + v[5])/12; soft = fir6(ifish); hard = fir6(clamp(d, −1, 1)), the raw derivative clipped as in Ehlers' listing (Ehlers, An Elegant Oscillator: Inverse Fisher Transform Redux, TASC Feb 2022: Elegant Oscillator and Soft And Hard Limiter Comparison listings, Traders' Tips Feb 2022; n = 50, band edge 20)", &[52, 56, 7], elegant_osc),
    def("fourier_series", "Ehlers Fourier Series Indicator", FSP, &["wave", "roc"], PN, "bp_k = Ehlers band-pass at F/k (k = 1..3, 0 on the first 3 bars), q_k = (F/6.28)(bp_k − bp_k[1]) (0 on the first 4); P_k = Σ_{F bars}(bp_k² + q_k²); wave = bp1 + √(P2/P1)bp2 + √(P3/P1)bp3 (P1 = 0 holds); roc = (F/12.57)(wave − wave[2]) (Ehlers, Fourier Series Model of the Market, 2019)", &[19, 21], fourier_series),
    def("ht_dcperiod", "Hilbert Transform Dominant Cycle Period", NOP, &["period"], PN, "TA-Lib HT_DCPERIOD ported line by line: WMA(4) price smoother, Hilbert FIR (0.0962, 0.5769)·(0.075·period[1] + 0.54) on odd/even bars, homodyne discriminator, period clamped to [0.67, 1.5]×previous and [6, 50], 0.2/0.8 then 0.33/0.67 smoothing; lookback 32, unstable period 0", &[32], ht_dcperiod),
    def("ht_dcphase", "Hilbert Transform Dominant Cycle Phase", NOP, &["phase"], PN, "TA-Lib HT_DCPHASE ported line by line: DFT of the smoothed price over int(smoothPeriod + 0.5) bars, atan(real/imag) + 90° + 360/smoothPeriod (WMA lag), +180° when imag < 0, wrapped above 315°; lookback 63, unstable period 0", &[63], ht_dcphase),
    def("ht_phasor", "Hilbert Transform Phasor Components", NOP, &["inphase", "quadrature"], PN, "TA-Lib HT_PHASOR ported line by line: inphase = detrender delayed 3 bars (I1), quadrature = Hilbert of the detrender (Q1); lookback 32, unstable period 0", &[32, 32], ht_phasor),
    def("ht_sine", "Hilbert Transform SineWave (MESA Sine Wave)", NOP, &["sine", "lead_sine"], PN, "TA-Lib HT_SINE ported line by line: sine = sin(DC phase), lead_sine = sin(DC phase + 45°); lookback 63, unstable period 0", &[63, 63], ht_sine),
    def("ht_trendmode", "Hilbert Transform Trend vs Cycle Mode", NOP, &["trend"], PN, "TA-Lib HT_TRENDMODE ported line by line: 1 = trend unless the sine/lead-sine crossed within half a cycle or the DC phase advanced 0.67–1.5×360/period; forced 1 when the smoothed price is ≥ 1.5% from the instantaneous trendline (raw-price average); lookback 63, unstable period 0", &[63], ht_trendmode),
    def("onset_trend", "Ehlers Onset Trend Detector (Quotient Transform)", OTP, &["quotient"], PN, "f = SuperSmoother(hp2(close, hp), ss); peak = max(.991·peak[1], |f|); x = f/peak; quotient = (x + k)/(k·x + 1) (Ehlers, The Quotient Transform, TASC Aug 2014, Early Onset Trend listing in Traders' Tips Aug 2014: high-pass fixed at 100, LPPeriod 30, K1 = .85 and K2 = .4)", &[3], onset_trend),
    def("reflex", "Ehlers Reflex", L20, &["reflex"], PN, "f = SuperSmoother(close, n/2); slope = (f[n] − f)/n; s = Σ_{i=1..n}(f + i·slope − f[i])/n; ms = .04s² + .96ms[1]; reflex = s/√ms (Ehlers, Reflex: A New Zero-Lag Indicator, TASC Feb 2020, _Reflex listing)", &[21], reflex),
    def("reverse_ema", "Ehlers Reverse EMA", REP, &["wave"], PN, "cc = 1 − a; ema = a·close + cc·ema[1]; re1 = cc·ema + ema[1]; re_k = cc^(2^(k−1))·re_{k−1} + re_{k−1}[1], k = 2..8; wave = ema − a·re8 (Ehlers, The Reverse EMA Indicator, TASC Sep 2017, _ReverseEMA listing; zero-initialised)", &[8], reverse_ema),
    def("rmo_ehlers", "Ehlers Recursive Median Oscillator", RMP, &["rmo"], PN, "a = (cos(360°/lp) + sin(360°/lp) − 1)/cos(360°/lp); rm = a·median(close, 5) + (1 − a)rm[1]; rmo = hp2(rm, hp) (Ehlers, Recursive Median Filters, TASC Mar 2018, Recursive Median Oscillator listing, LPPeriod 12, HPPeriod 30)", &[6], rmo_ehlers),
    def("roofing", "Ehlers Roofing Filter", RFP, &["roof"], PN, "roof = SuperSmoother(hp2(close, hp), ss): a1 = exp(−1.414·3.14159/ss), c2 = 2a1·cos(1.414·180°/ss), c3 = −a1², filt = (1 − c2 − c3)(x + x[1])/2 + c2·filt[1] + c3·filt[2] (Ehlers, Predictive Indicators, TASC Jan 2014)", &[3], roofing),
    def("rvgi_ext", "Relative Vigor Index: Adaptive / Stochastic / Fisher", RVP, &["rvi", "stoch", "fisher"], PN, "rvi = Σ_n swma(close − open)/Σ_n swma(high − low) (as rvgi; adaptive: n = int(ht_dcperiod/2), declared substitute); stoch/fisher: Ehlers' wrappers as cg_ext (Ehlers, Cybernetic Analysis for Stocks and Futures, 2004: Stochastic RVI listing; Fisher RVI applies his Fisher step to the same smoothed stochastic)", &[12, 24, 24], rvgi_ext),
    def("super_passband", "Ehlers Super Passband Filter", SPP, &["pb", "upper", "lower"], PN, "a1 = 5/fast, a2 = 5/slow; pb = (a1 − a2)x + (a2(1 − a1) − a1(1 − a2))x[1] + ((1 − a1) + (1 − a2))pb[1] − (1 − a1)(1 − a2)pb[2]; upper/lower = ±√(Σpb²/n) (Ehlers, The Super Passband Filter, TASC Jul 2016: Period1 40, Period2 60, RMS over 50)", &[1, 50, 50], super_passband),
    def("swak_highpass", "Swiss Army Knife High-Pass / Band-Pass", SWP, &["filt"], PN, "TA-Lib SWAK_HP / SWAK_2PHP / SWAK_BP ported (Ehlers, Swiss Army Knife Indicator): HP a = 2sin h(cos h − sin h)/(1 − 2sin²h), h = π/n; 2PHP b = 2.415(1 − cos(2π/n)), a = −b + √(b² + 2b); BP β = cos(2π/n), a = (1 − sin t)/cos t, t = 4π·delta/n; inputs seeded with the first bar, outputs with 0, published from bar 0", &[0], swak_highpass),
    def("trendflex", "Ehlers Trendflex", L20, &["trendflex"], PN, "f = SuperSmoother(close, n/2); s = Σ_{i=1..n}(f − f[i])/n; ms = .04s² + .96ms[1]; trendflex = s/√ms (Ehlers, Reflex: A New Zero-Lag Indicator, TASC Feb 2020, _Trendflex listing)", &[21], trendflex),
    def("universal_osc", "Ehlers Universal Oscillator", UOP, &["uo"], PN, "wn = (close − close[2])/2; f = SuperSmoother(wn, edge); pk = max(.991·pk[1], |f|), 1e−7 on bar 1; uo = f/pk (Ehlers, Whiter Is Brighter, TASC Jan 2015, Universal Oscillator listing, BandEdge 20)", &[3], universal_osc),
    def("voss", "Ehlers Voss Predictive Filter", VSP, &["filt", "voss"], PN, "filt = Ehlers band-pass(close, P, bw), 0 on the first 5 bars; order = 3·predict; voss = ((3 + order)/2)·filt − Σ_{c<order}((c + 1)/order)·voss[order − c] (Ehlers, A Peek Into the Future, TASC Aug 2019)", &[5, 5], voss),
];
/// (id, platform names, AmiBroker names): one row per id in DEFS.
pub const SOURCES: &[(&str, &str, &str)] = &[
    ("bandpass", "thinkorswim: BandpassFilter; TA-Lib: SWAK_BP", ""),
    ("cg_ext", "Sierra Chart: Adaptive / Stochastic / Fisher Center of Gravity", ""),
    ("correlation_cycle", "thinkorswim: CorrelationCycle, CorrelationAngle, MarketState", ""),
    ("cyber_cycle", "Sierra Chart: Cyber Cycle, Adaptive/Fisher/Stochastic Cyber Cycle; thinkorswim: CyberCyclesOscillator; cTrader: CyberCycle", ""),
    ("decycler_osc", "thinkorswim: EhlersDecyclerOscillator", ""),
    ("dso", "thinkorswim: DeviationScaledOscillator", ""),
    ("ebsw", "Sierra Chart: Even Better Sinewave; pandas-ta: ebsw", ""),
    ("ehlers_am_fm", "thinkorswim: AMDetector, FMDemodulator", ""),
    ("ehlers_hp", "thinkorswim: EhlersHighPassFilter; TA-Lib: SWAK_HP, SWAK_2PHP", ""),
    ("ehlers_leading", "Sierra Chart: Leading Indicator; MultiCharts: D_ELI", ""),
    ("ehlers_stoch", "thinkorswim: EhlersStochastic", ""),
    ("elegant_osc", "thinkorswim: ElegantOscillator, SoftAndHardLimiterComparison", ""),
    ("fourier_series", "thinkorswim: FourierSeries", ""),
    ("ht_dcperiod", "TA-Lib: HT_DCPERIOD; pandas-ta: ht_dcperiod; Sierra Chart: Cycle Period", ""),
    ("ht_dcphase", "TA-Lib: HT_DCPHASE; pandas-ta: ht_dcphase", ""),
    ("ht_phasor", "TA-Lib: HT_PHASOR; pandas-ta: ht_phasor", ""),
    ("ht_sine", "TA-Lib: HT_SINE; pandas-ta: ht_sine; Tulip: msw; thinkorswim: MESASineWave; Sierra Chart: Sinewave; MetaStock: MESA Sine Wave", ""),
    ("ht_trendmode", "TA-Lib: HT_TRENDMODE; pandas-ta: ht_trendmode", ""),
    ("onset_trend", "thinkorswim: OnsetTrendDetector", ""),
    ("reflex", "pandas-ta: reflex; thinkorswim: Reflex", ""),
    ("reverse_ema", "thinkorswim: ReverseEMA", ""),
    ("rmo_ehlers", "thinkorswim: EhlersRecursiveMedianOscillator", ""),
    ("roofing", "thinkorswim: EhlersRoofingFilter", ""),
    ("rvgi_ext", "Sierra Chart: Adaptive / Stochastic / Fisher RVI", ""),
    ("super_passband", "thinkorswim: SuperPassbandFilter", ""),
    ("swak_highpass", "TA-Lib: SWAK_HP, SWAK_2PHP, SWAK_BP", ""),
    ("trendflex", "pandas-ta: trendflex; thinkorswim: Trendflex", ""),
    ("universal_osc", "thinkorswim: UniversalOscillator", ""),
    ("voss", "thinkorswim: VossPredictiveFilter", ""),
];

#[cfg(test)]
mod tests {
    use crate::lab::testdata::{close_eq, closes, from_ohlc, go, walk};
    const N: f64 = f64::NAN;
    fn wave(len: usize, per: f64) -> Vec<f64> { (0..len).map(|i| 100. + (2. * std::f64::consts::PI * i as f64 / per).sin()).collect() }
    fn last(v: &[f64]) -> f64 { *v.last().unwrap() }
    fn amp(v: &[f64], from: usize) -> f64 { v[from..].iter().fold(0f64, |m, x| m.max(x.abs())) }
    fn flat(id: &str, kv: &[(&str, f64)]) -> Vec<Vec<f64>> { go(id, &closes(&[50.; 400]), kv) }
    fn near(a: f64, b: f64, tol: f64) -> bool { (a - b).abs() <= tol }

    /// TA-Lib 0.8.1 (python TA-Lib 0.8.2) on c[i] = 100 + 0.2i + 5sin(0.7i) + 3cos(0.31i), i < 120.
    #[test] fn hilbert_family_matches_talib() {
        let c: Vec<f64> = (0..120).map(|i| { let x = i as f64; 100. + 0.2 * x + 5. * (x * 0.7).sin() + 3. * (x * 0.31).cos() }).collect();
        let b = closes(&c);
        let at = [32usize, 33, 50, 63, 64, 80, 119];
        let chk = |id: &str, k: usize, want: &[f64]| { let s = &go(id, &b, &[])[k]; for (j, &i) in at.iter().enumerate() { assert!(close_eq(s[i], want[j]) || (s[i] - want[j]).abs() < 1e-9, "{id}[{k}] bar {i}: {} vs {}", s[i], want[j]); } };
        chk("ht_dcperiod", 0, &[15.646696015449784, 17.026088549201422, 18.763537247888546, 14.139703163231822, 13.80163796050556, 12.738161298691036, 12.441420256973464]);
        chk("ht_dcphase", 0, &[N, N, N, 156.5079354706431, 166.2457254168061, 163.29918263418125, 81.34668464029352]);
        chk("ht_phasor", 0, &[-0.2752491019274492, 2.889410404185521, 0.7375655133961073, 3.10034240663882, -0.8184472995353214, 5.356846342142298, -4.361231809937478]);
        chk("ht_phasor", 1, &[8.04015441052441, 6.189226644871157, 9.978158458095356, -8.6562892168096, -7.09476959070047, -4.3906171265726535, 0.8649197222892635]);
        chk("ht_sine", 0, &[N, N, N, 0.39862205219797464, 0.23775835917484273, 0.2873741838525233, 0.9886168060841263]);
        chk("ht_sine", 1, &[N, N, N, -0.366630086126013, -0.5187094755457687, -0.47407564835131893, 0.8054456420313827]);
        chk("ht_trendmode", 0, &[N, N, N, 1., 1., 0., 1.]);
        assert!(go("ht_dcperiod", &b, &[])[0][31].is_nan());
    }
    #[test] fn hilbert_period_finds_a_20_bar_cycle() {
        let p = go("ht_dcperiod", &closes(&wave(400, 20.)), &[])[0].clone();
        assert!(near(last(&p), 20., 1.), "{}", last(&p));
        let ph = go("ht_dcphase", &closes(&wave(400, 20.)), &[])[0].clone();
        let step = (ph[399] - ph[398] + 360.) % 360.; assert!(near(step, 18., 3.), "{step}");
        let ramp: Vec<f64> = (0..300).map(|i| 100. + 2. * i as f64).collect();
        assert_eq!(last(&go("ht_trendmode", &closes(&ramp), &[])[0]), 1.);
    }
    #[test] fn constant_close_gives_zero() {
        for (id, kv) in [("bandpass", &[][..]), ("decycler_osc", &[]), ("dso", &[]), ("fourier_series", &[]), ("elegant_osc", &[]), ("universal_osc", &[]), ("voss", &[]),
                         ("swak_highpass", &[("mode", 0.)]), ("swak_highpass", &[("mode", 1.)]), ("swak_highpass", &[("mode", 2.)]), ("roofing", &[]), ("ehlers_hp", &[]), ("ehlers_hp", &[("poles", 1.)]), ("cyber_cycle", &[])] {
            for s in flat(id, kv).iter().take(if id == "cyber_cycle" { 2 } else { 4 }) { assert!(s.iter().all(|v| v.is_nan() || v.abs() < 1e-9), "{id} {kv:?}"); }
        }
        // zero-initialised recursions settle to 0 on a constant
        for id in ["rmo_ehlers", "super_passband", "reverse_ema"] { assert!(last(&flat(id, &[])[0]).abs() < 1e-6, "{id}"); }
        let l = flat("ehlers_leading", &[]); assert!(near(last(&l[0]), 50., 1e-9) && near(last(&l[1]), 50., 1e-9));
    }
    #[test] fn hand_values() {
        let b = closes(&[10., 11., 13., 12., 15., 14., 16.]);
        // bandpass bar 3 = .5(1 − S)(c3 − c1); bar 4 adds L(1 + S)bp[3]
        let (l, s) = super::bp_coef(20., 0.3).unwrap();
        let bp = go("bandpass", &b, &[]);
        let b3 = 0.5 * (1. - s) * (12. - 11.); let b4 = 0.5 * (1. - s) * (15. - 13.) + l * (1. + s) * b3;
        assert!(bp[0][2].is_nan() && close_eq(bp[0][3], b3) && close_eq(bp[0][4], b4) && close_eq(bp[1][4], b3));
        // SWAK HP: bar 0 = 0, bar 1 = c0·(x1 − x0)
        let h = std::f64::consts::PI / 20.; let a1p = 2. * h.sin() * (h.cos() - h.sin()) / (1. - 2. * h.sin() * h.sin());
        let sw = go("swak_highpass", &b, &[]);
        assert!(sw[0][0] == 0. && close_eq(sw[0][1], (1. - a1p / 2.) * 1.));
        // super passband bar 1 = (a1 − a2)c1 + (a2(1 − a1) − a1(1 − a2))c0
        let (a1, a2) = (5. / 40., 5. / 60.);
        assert!(close_eq(go("super_passband", &b, &[])[0][1], (a1 - a2) * 11. + (a2 * (1. - a1) - a1 * (1. - a2)) * 10.));
        // cyber cycle seed (x − 2x[1] + x[2])/4 on hl2 = close; trigger = cycle[1]
        let cy = go("cyber_cycle", &b, &[]);
        assert!(cy[0][1].is_nan() && close_eq(cy[0][2], (13. - 22. + 10.) / 4.) && close_eq(cy[1][3], cy[0][2]));
        // AM/FM: fm = close − open on the first two bars
        let o = from_ohlc(&[(10., 12., 9., 11.), (11., 12., 10., 10.5), (10.5, 11., 10., 10.6)]);
        let am = go("ehlers_am_fm", &o, &[]); assert!(am[1][0] == 1. && am[1][1] == -0.5);
        // elegant hard limiter = fir6 of the raw close − close[2] clipped to ±1 (TASC Feb 2022 listing): d = .1 .3 −.2 −.5 .4 1.9→1
        let el = go("elegant_osc", &closes(&[10., 10.2, 10.1, 10.5, 9.9, 10., 10.3, 11.9]), &[]);
        assert!(el[2][6].is_nan() && close_eq(el[2][7], (1. + 2. * 0.4 + 3. * -0.5 + 3. * -0.2 + 2. * 0.3 + 0.1) / 12.), "{}", el[2][7]);
        let same = from_ohlc(&vec![(10., 11., 9., 10.); 40]);
        for s in go("ehlers_am_fm", &same, &[]) { assert!(s.iter().all(|v| v.is_nan() || *v == 0.)); }
    }
    #[test] fn passbands_have_unity_gain_at_the_centre() {
        let b = closes(&wave(600, 20.));
        assert!(near(amp(&go("bandpass", &b, &[])[0], 400), 1., 0.05));
        assert!(near(amp(&go("swak_highpass", &b, &[("mode", 2.)])[0], 400), 1., 0.05));
        let r = amp(&go("roofing", &b, &[])[0], 400); assert!((0.7..1.1).contains(&r), "{r}");
        // Voss leads the band-pass: its peak comes earlier within the cycle
        let v = go("voss", &b, &[]); let argmax = |s: &[f64]| (500..520).max_by(|&i, &j| s[i].total_cmp(&s[j])).unwrap();
        assert!(argmax(&v[1]) < argmax(&v[0]) || argmax(&v[0]) - argmax(&v[1]) > 10);
    }
    #[test] fn correlation_of_a_cosine() {
        let c: Vec<f64> = (0..100).map(|i| 50. + (2. * std::f64::consts::PI * i as f64 / 20.).cos()).collect();
        let o = go("correlation_cycle", &closes(&c), &[]);
        assert!(near(o[0][60], 1., 1e-9) && near(o[1][60], 0., 1e-9) && o[0][18].is_nan() && o[3][19].is_nan());
    }
    #[test] fn bounded_outputs() {
        let b = walk(600, 3, false);
        let within = |id: &str, k: usize, lo: f64, hi: f64| { for v in go(id, &b, &[])[k].iter().filter(|v| !v.is_nan()) { assert!(*v >= lo - 1e-12 && *v <= hi + 1e-12, "{id}[{k}] = {v}"); } };
        within("ebsw", 0, -1., 1.); within("universal_osc", 0, -1., 1.); for k in 0..3 { within("elegant_osc", k, -1., 1.); }
        within("cg_ext", 1, -1., 1.); within("cyber_cycle", 2, -1., 1.); within("rvgi_ext", 1, -1., 1.); within("onset_trend", 0, -1., 1.);
        within("ehlers_stoch", 0, -0.3, 1.3); within("ht_sine", 0, -1., 1.); within("ht_trendmode", 0, 0., 1.);
        let st = &go("correlation_cycle", &b, &[])[3]; assert!(st.iter().filter(|v| !v.is_nan()).all(|v| [-1., 0., 1.].contains(v)));
        // fisher is 0 exactly when the smoothed stochastic is mid-range
        let cg = go("cg_ext", &b, &[]); for i in 30..600 { if cg[1][i] == 0. { assert!(cg[2][i].abs() < 1e-12); } }
    }
    #[test] fn wrappers_reuse_the_base_indicators() {
        let b = walk(200, 5, false);
        let (rv, base) = (go("rvgi_ext", &b, &[]), go("rvgi", &b, &[]));
        for i in 0..200 { assert!(close_eq(rv[0][i], base[0][i]), "rvi bar {i}"); }
        let (cg, base) = (go("cg_ext", &b, &[]), go("cg_osc", &b, &[("length", 8.)]));
        for i in 0..200 { assert!(close_eq(cg[0][i], base[0][i]), "cg bar {i}"); }
        for id in ["cg_ext", "rvgi_ext", "cyber_cycle"] { for s in go(id, &b, &[("adaptive", 1.)]) { assert!(s[150].is_finite(), "{id} adaptive"); } }
    }
    #[test] fn ramps_reflex_and_trendflex() {
        let ramp: Vec<f64> = (0..500).map(|i| 100. + 0.5 * i as f64).collect();
        let b = closes(&ramp);
        assert!(near(last(&go("trendflex", &b, &[])[0]), 1., 1e-3));
        assert!(last(&go("reflex", &b, &[])[0]).abs() < 1e-3);
        assert!(near(last(&go("ebsw", &b, &[])[0]), 1., 1e-6));
        assert!(last(&go("ehlers_hp", &b, &[])[0]).abs() < 1e-6);
    }
    #[test] fn bad_bandwidth_is_refused() {
        let d = crate::lab::catalogue::get("bandpass").unwrap();
        let e = crate::lab::catalogue::run(&d, &closes(&[1.; 10]), &[("length".to_string(), 4.), ("bandwidth".to_string(), 1.)].into_iter().collect()).unwrap_err();
        assert!(e.contains("bandwidth"));
    }
    /// The catalogue warm-up check restricted to this file (the shared test stops at the first failing def anywhere).
    #[test] fn warmup_matches_first() {
        let bars = walk(1000, 7, true);
        for d in super::DEFS {
            let out = crate::lab::catalogue::run(d, &bars, &Default::default()).unwrap();
            for (k, s) in out.iter().enumerate() {
                let first = s.iter().position(|v| !v.is_nan()).unwrap_or(usize::MAX);
                assert_eq!(first, d.first[k], "{}.{}", d.id, d.outputs[k]);
                assert!(s[first..].iter().all(|v| v.is_finite()), "{}.{} gap", d.id, d.outputs[k]);
            }
        }
    }
    #[test] fn parameter_bounds_run() {
        for bars in [walk(3, 1, false), walk(300, 1, false)] {
            for d in super::DEFS { for max in [false, true] {
                let prm = d.params.iter().map(|p| (p.key.to_string(), if max { p.max } else { p.min })).collect();
                let out = crate::lab::catalogue::run(d, &bars, &prm).unwrap_or_else(|e| panic!("{}: {e}", d.id));
                for s in out { assert!(s.iter().all(|v| !v.is_infinite()), "{} max={max}", d.id); }
            } }
        }
    }
    #[test] fn rules_reach_cycles() { for (id, out) in [("ht_sine", "lead_sine"), ("voss", "voss"), ("correlation_cycle", "state")] { crate::lab::testdata::compiles(id, out).unwrap(); } }
}
