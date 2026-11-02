//! Klinger Volume Oscillator system: a TradeStation list post ("John", research share USB HDD 1/TradeStation Systems &
//! Indicators/Klinger Volume Oscilator for Ts/Readme.rar -> Readme.txt) whose code calls KVO(FastX, SlowX, Vol) without printing
//! it. The KVO itself is taken from Stephen J. Klinger's own article, "Identifying Trends With Volume Analysis", S&C V.15:12
//! (Dec 1997) p.556-560 (research share Trading Knowledge I/Stocks & Commodities DVD .../VOLUMES/V15/C12/IDENTIF.pdf), which
//! prints the formula and its spreadsheet (lead decisions 37 and 139). Book id `tsklvo`. Lane K20 (titles loop, 2 Nov 2026),
//! to-code row 344; independent check authors/titles-1030/K20w/k20_ref.py.
use crate::books::rrsim::{g, o, pd, run_ts, sma, true_range, A, P};
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const REVIEWED: &[&str] = &["tsklvo_kvo"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] The list post's TradeStation code (Readme.txt), tested by its author on back-adjusted Dow futures; runs on daily indices, ETFs and stocks with volume, US and India, are a declared application. \
KVO (Klinger, S&C V.15:12 p.557-558): trend T = +1 when today's High + Low + Close is above yesterday's sum, otherwise -1 (the printed spreadsheet formula =IF((B3+C3+D3)>(B2+C2+D2),1,-1); the text's 'when equality exists, the existing trend is maintained' is option p.teq = 1); \
DM = High - Low; CM = yesterday's CM + DM while the trend is unchanged, else yesterday's DM + today's DM; volume force VF = Volume x |2 x DM / CM - 1| x T x 100 (DM / CM taken as 0 when CM is 0); \
KVO = 34-day EMA of VF - 55-day EMA of VF; EMAs use 2/(n+1) and start at the first VF (the spreadsheet's 0.0571 and 0.0357 are these rounded; option p.xl = 1 uses the printed 0.0571 / 0.0357 / 0.143). \
The KVO starts at the first bar of the unbroken run of days with volume that holds the test window (never before the data's second bar); that bar's CM counts as a trend change (yesterday's DM + today's). Trigger = XAverage(KVO, 13). \
Code: Klinger = 1 when KVO > trigger, -1 when KVO < trigger, unchanged when equal. While Klinger = 1: buy tomorrow on a stop at High + 1 x 0.26 x AvgTrueRange(10); while -1: sell tomorrow on a stop at Low - 0.26 x AvgTrueRange(10) \
(AvgTrueRange = 10-bar average of the true range). TradeStation 2000i 'Sell' opens a short (lead decision 141), so the system is stop and reverse; a buy is ignored while long, a sell while short. \
The code's other fill modes are options: p.atrv = -0.2 buys/sells at the signal close, -0.1 at the next open. The post's author recommends tick volume intraday; daily volume is used here. No stop or target is printed. \
The post's own tests (back-adjusted Dow futures from 1990, about 12 years) lose with every entry variant shown; its author calls the period of his post a special case.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "tsklvo_kvo", name: "Klinger Volume Oscillator system: stop entries 0.26 ATR beyond the bar when the KVO is above / below its 13-day trigger, stop and reverse", group: "TradeStation code: Klinger Volume Oscillator (list post) + Klinger's KVO (S&C V.15:12)", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: true, needs_vix: false, needs_series: &[], book: "tsklvo", rules: &[
            "F|\"VF = V x [2 (DM / CM) - 1] x T x 100\" (absolute value in the spreadsheet: =E3*(ABS(2*(G3/H3)-1))*F3*100); T: =IF((B3+C3+D3)>(B2+C2+D2),1,-1); DM: =B3-C3; CM: =(IF(AND(F3=1,F2=1),H2+G3,(IF(AND(F3=-1,F2=-1),H2+G3,G2+G3)))) (Klinger, S&C V.15:12 p.557-558)",
            "F|\"KVO = Mov(VF( ), 34, E) - Mov(VF( ), 55, E)\" (p.558); \"Trigger = XAverage(KVO(FastX, SlowX,Vol), TrigLen);\" TrigLen 13 (Readme.txt)",
            "E|\"If KVO(FastX, SlowX,Vol)>Trigger Then Klinger= 1 else If KVO(FastX, SlowX,Vol)<Trigger Then Klinger= -1;\" \"If Klinger=1 then ... Buy tomorrow at H+Hyst*AvgTrueRange(ATRDays)*ATRv Stop;\" (ATRv 0.26, ATRDays 10, Hyst 1)",
            "E|\"If Klinger=-1 then ... Sell tomorrow at L-Hyst*AvgTrueRange(ATRDays)*ATRv Stop;\" (TS2000i Sell = enter short, reversing a long)",
            "X|No exit orders: a position closes only when the opposite stop reverses it"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { reviewed(id).then_some(("tsklvo_kvo", "Klinger Volume Oscillator system (TradeStation)", "KVO 34/55, trigger 13, 0.26 ATR stops")) }

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    reviewed(id).then(|| vec![
        pd("fast", "FastX: KVO fast EMA", 34., 2., 100., 1.), pd("slow", "SlowX: KVO slow EMA", 55., 3., 200., 1.), pd("trig", "TrigLen: trigger EMA", 13., 1., 50., 1.),
        pd("atrv", "ATRv: stop offset in ATRs (-0.2 = at the signal close, -0.1 = next open)", 0.26, -0.2, 2., 0.01), pd("atr_n", "ATRDays", 10., 1., 50., 1.), pd("hyst", "Hyst", 1., 0., 5., 0.1),
        pd("teq", "1 = an unchanged H+L+C sum keeps the trend (Klinger's text); 0 = -1 (his printed spreadsheet)", 0., 0., 1., 1.),
        pd("xl", "1 = the spreadsheet's printed EMA constants 0.0571 / 0.0357 / 0.143; 0 = 2/(n+1)", 0., 0., 1., 1.),
    ])
}

fn vol_ok(x: &Ctx, i: usize) -> bool { x.r[i].volume.is_some_and(|v| v.is_finite() && v > 0.) }

/// KVO and trigger from bar `s` on (NaN before it).
pub fn kvo(x: &Ctx, s: usize, fast: usize, slow: usize, trig: usize, teq: bool, xl: bool) -> (Vec<f64>, Vec<f64>) {
    let n = x.c.len();
    let (mut k, mut t) = (vec![f64::NAN; n], vec![f64::NAN; n]);
    if s == 0 || s >= n { return (k, t); }
    let alpha = |m: usize, printed: f64| if xl { printed } else { 2. / (m as f64 + 1.) };
    let (af, asl, at) = (alpha(fast, 0.0571), alpha(slow, 0.0357), alpha(trig, 0.143));
    let sum = |i: usize| x.h[i] + x.l[i] + x.c[i];
    let (mut tr_prev, mut cm, mut ef, mut es, mut tg) = (0i8, 0., 0., 0., 0.);
    for i in s..n {
        let dm = x.h[i] - x.l[i];
        let tr: i8 = if sum(i) > sum(i - 1) { 1 } else if teq && sum(i) == sum(i - 1) && i > s { tr_prev } else { -1 };
        cm = if i > s && tr == tr_prev { cm + dm } else { x.h[i - 1] - x.l[i - 1] + dm };
        let ratio = if cm == 0. { 0. } else { dm / cm };
        let vf = x.r[i].volume.unwrap_or(f64::NAN) * (2. * ratio - 1.).abs() * tr as f64 * 100.;
        if i == s { ef = vf; es = vf; tg = 0.; } else { ef += af * (vf - ef); es += asl * (vf - es); }
        let kv = ef - es;
        if i > s { tg += at * (kv - tg); }
        k[i] = kv; t[i] = tg;
        tr_prev = tr;
    }
    (k, t)
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let n = |k: &str, d: f64| g(p, k, d).round().max(1.) as usize;
    // start of the unbroken run of volume days that holds `from`
    let mut s = from;
    while s > 0 && vol_ok(x, s - 1) { s -= 1; }
    let s = s.max(1);
    let (kv, tg) = kvo(x, s, n("fast", 34.), n("slow", 55.), n("trig", 13.), g(p, "teq", 0.) >= 0.5, g(p, "xl", 0.) >= 0.5);
    let atr = sma(&true_range(x), n("atr_n", 10.));
    let (atrv, hy) = (g(p, "atrv", 0.26), g(p, "hyst", 1.));
    let (at_close, at_open) = ((atrv + 0.2).abs() < 1e-9, (atrv + 0.1).abs() < 1e-9);
    let mut kl = 0i8;
    Some(run_ts(x, from, to, |i, _st, now, next| {
        if kv[i].is_nan() { return; }
        if kv[i] > tg[i] { kl = 1 } else if kv[i] < tg[i] { kl = -1 }
        if kl == 0 { return; }
        let a = if kl == 1 { A::Buy } else { A::Sell };
        let tag = if kl == 1 { "LE" } else { "SE" };
        if at_close { now.push(o(a, P::Mkt, tag)); }
        else if at_open { next.push(o(a, P::Mkt, tag)); }
        else if !atr[i].is_nan() {
            let px = if kl == 1 { x.h[i] + hy * atr[i] * atrv } else { x.l[i] - hy * atr[i] * atrv };
            next.push(o(a, P::Stop(px), tag));
        }
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn kvo_reproduces_klingers_spreadsheet() {
        // S&C V.15:12 p.558, rows 1-8 (10/15/93-10/26/93), printed EMA constants; row 1 seeds the EMAs (our bar s = 1)
        let d = [(27.5, 26.5, 27., 16518.), (28., 26.75, 27.875, 19030.), (27.75, 26.25, 26.5, 20836.), (27., 25.75, 26.25, 9652.),
                 (27.5, 25.5, 27.375, 10080.), (28., 26., 26.125, 14651.), (26.75, 25.5, 26.125, 12267.), (26.5, 25.75, 26.375, 8787.)];
        // a bar before row 1 makes row 1 a down day with DM 1 and CM 1.75 (CM = previous DM 0.75 + 1)
        let mut r = vec![Row { ticker: "T".into(), date: "0".into(), open: 28., high: 28.5, low: 27.75, close: 28., volume: Some(1.) }];
        r.extend(d.iter().enumerate().map(|(i, &(h, l, c, v))| Row { ticker: "T".into(), date: format!("{}", i + 1), open: c, high: h, low: l, close: c, volume: Some(v) }));
        let x = Ctx::new(&r, &r, &[]);
        let (k, t) = kvo(&x, 1, 34, 55, 13, false, true);
        let want = [(0.0, 0.0), (9574.70, 1369.18), (9682.41, 2557.97), (6068.42, 3059.97), (15493.78, 4838.00), (19054.12, 6870.91), (8501.76, 7104.12), (17341.58, 8568.08)];
        for (j, (wk, wt)) in want.iter().enumerate() {
            assert!((k[j + 1] - wk).abs() < 0.02 && (t[j + 1] - wt).abs() < 0.02, "row {}: {} {}", j + 1, k[j + 1], t[j + 1]);
        }
    }
    #[test]
    fn klinger_stop_and_reverse() {
        let mut r = super::super::sctraub::tests::bars(2500);
        for (i, b) in r.iter_mut().enumerate() { b.volume = Some(1e6 + 3e5 * ((i as f64) / 5.).sin()); }
        let x = Ctx::new(&r, &r, &[]);
        let t = run("tsklvo_kvo", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(t.len() > 10 && t.iter().any(|z| z.dir > 0.) && t.iter().any(|z| z.dir < 0.));
        for w in t.windows(2) { assert_eq!((w[0].exit_i, w[0].exit_px), (w[1].legs[0].i, w[1].legs[0].px)); }
        assert!(run("tsklvo_x", &x, 0, 5, &Params::new()).is_none());
    }
}

// Instrument admission: Klinger KVO system 2026-11-02 (titles lane K20).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "tsklvo" && reviewed(def.id),
    label: "A TradeStation list-post system tested on Dow futures, with Klinger's published KVO; runs on indices, ETFs and stocks with volume, US and India, are a declared application.",
    basis: "Klinger Volume Oscilator for Ts/Readme.rar -> Readme.txt (code and the post's results); KVO from S&C V.15:12 p.556-560 (Klinger, IDENTIF.pdf); independent check authors/titles-1030/K20w/k20_ref.py",
};
