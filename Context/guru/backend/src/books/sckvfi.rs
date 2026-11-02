//! Markos Katsanos, "Volume Flow Indicator Performance", Technical Analysis of Stocks & Commodities V.22:7 (July 2004) pp.50-60,
//! with the MetaStock sidebar pp.58-59 (research share Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008)
//! (traders.com)/VOLUMES/V22/C07/146KATS.pdf and 146SB.pdf). Book id `sckvfi`: Test 1 (weekly VFI > 0) and Test 2 (daily VFI
//! divergence). Lane K13 (titles loop, 2 Nov 2026), to-code row 223; independent check authors/titles-1030/K13w/k13_ref.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::msws::sma;
use super::sctraub::{ema0, next_open, period_ends, week_groups};

pub const REVIEWED: &[&str] = &["sckvfi_t1", "sckvfi_t2"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] The sidebar's MetaStock code: INTER = Log(Typ) - Log(Ref(Typ,-1)); VINTER = Stdev(INTER,30) (population); CUTOFF = .2 x VINTER x C; VAVE = Ref(Mov(V,PERIOD,S),-1); VC = If(V < 2.5 VAVE, V, 2.5 VAVE); VFI = Mov(Sum(+VC if Typ - Typ[-1] > CUTOFF, -VC if < -CUTOFF, else 0, PERIOD) / VAVE, 3, E). MetaStock EMAs are seeded with the first value. Test 1 runs on weekly bars built from each calendar week's daily bars (volume = the week's total; a week with a missing day's volume has no value), PERIOD 26: buy when Mov(VFI,7,E) > 0, sell when < 0. Test 2 is daily, PERIOD 130: P1 = VFI + Abs(Lowest(VFI)) + 10 (Lowest = since the first bar), LRSI / LRS = 120-bar LinRegSlope of P1 / of C divided by the value 120 bars ago x 100, DIVERG = Mov(LRSI - LRS, 3, E) x 100; buy when yesterday's DIVERG > 100, today's is lower and LRSI > 0; sell when DIVERG < 0 OR LRSI < 0 (the printed code; the text says 'and', an option). 'All orders were executed at next day's opening price' (weekly: the next week's first open). Long only; no stops ('a stop-loss condition should also be applied. This was not included here'). Needs the instrument's own volume (decision 79). Article: Test 1 Corning (GLW) weekly 1995-2003, 4 trades, all winners, +$345,951 on $10,000 compounded; Test 2 Foamex (FMXI) daily, 3 trades, +$123,112. Tests 3 (break from base) and 4 (VFI moving-average cross, optimised 11/25) are in the sidebar but not coded here. Runs on indices, ETFs and stocks with volume, US and India, are a declared application.";

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, rules| StrategyDef { id, name, group: "S&C: Volume Flow Indicator Performance (Katsanos, V.22:7)", session: 0, dir: "Long", status: Status::Live,
        needs_volume: true, needs_vix: false, needs_series: &[], book: "sckvfi", rules, notes: NOTE, claim: None };
    vec![
        d("sckvfi_t1", "Katsanos VFI Test 1 (weekly): long while the 7-week EMA of the 26-week VFI is above zero", &[
            "F|\"PERIOD:=26; VCOEF:=2.5; INTER:=Log(TYPICAL())-Log(Ref(TYPICAL(),-1)); VINTER:=Stdev(INTER,30); CUTOFF:=.2*VINTER*C; VAVE:=Ref(Mov(V,PERIOD,S),-1); VMAX:=VAVE*VCOEF; VC:=If(V<VMAX,V,VMAX); MF:=TYPICAL()-Ref(TYPICAL(),-1); VFI:=Sum(If(MF>CUTOFF, +VC, If(MF <-CUTOFF, -VC,0)), PERIOD)/VAVE; VFI:=Mov(VFI,3,E);\" (sidebar p.58)",
            "E|\"Enter Long ... MOV(VFI,7,E)>0\" (sidebar, Test 1; weekly periodicity)", "X|\"Sell Order ... MOV(VFI,7,E)<0\" (sidebar, Test 1)"]),
        d("sckvfi_t2", "Katsanos VFI Test 2 (daily divergence): buy when the 130-day VFI's slope divergence over 100 turns down with a rising VFI slope; out when it goes negative", &[
            "F|\"P1:=VFI+Abs(Lowest(VFI))+10; LRSI:=LinRegSlope(P1,D1)/Abs(Ref(P1,-D1))*100; LRS:=LinRegSlope(C,D1)/Abs(Ref(C,-D1))*100; DIVERG:=LRSI-LRS; DIVERG:=Mov(DIVERG,3,E)*100;\" (sidebar p.58; PERIOD 130, 120-day divergence)",
            "E|\"REF(diverg,-1)>100 AND DIVERG<REF(diverg,-1) and LRSI>0\" (sidebar, Test 2)", "X|\"diverg<0 OR LRSI<0\" (sidebar, Test 2; the text: 'Divergence < 0 and LRSI < 0')"]),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let v = match id { "sckvfi_t1" => "Test 1: weekly VFI > 0", "sckvfi_t2" => "Test 2: daily divergence", _ => return None };
    Some(("sckvfi", "Katsanos Volume Flow Indicator (S&C 2004)", v))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    match id {
        "sckvfi_t1" => Some(vec![]),
        "sckvfi_t2" => Some(vec![ParamDef { key: "and", label: "Exit (0 = DIVERG < 0 OR LRSI < 0, the code; 1 = AND, the text)", def: 0., min: 0., max: 1., step: 1. }]),
        _ => None,
    }
}

/// Katsanos VFI (smoothed) of the given bars.
fn vfi(h: &[f64], l: &[f64], c: &[f64], v: &[f64], p: usize) -> Vec<f64> {
    let n = c.len();
    let typ: Vec<f64> = (0..n).map(|i| (h[i] + l[i] + c[i]) / 3.0).collect();
    let inter: Vec<f64> = (0..n).map(|i| if i > 0 { typ[i].ln() - typ[i - 1].ln() } else { f64::NAN }).collect();
    let vin: Vec<f64> = (0..n).map(|i| if i >= 30 {
        let w = &inter[i - 29..=i];
        let m = w.iter().sum::<f64>() / 30.0;
        (w.iter().map(|x| (x - m).powi(2)).sum::<f64>() / 30.0).sqrt()
    } else { f64::NAN }).collect();
    let mv = sma(v, p);
    let vave: Vec<f64> = (0..n).map(|i| if i > 0 { mv[i - 1] } else { f64::NAN }).collect();
    let term: Vec<f64> = (0..n).map(|i| {
        if i == 0 || !(vave[i].is_finite() && v[i].is_finite() && vin[i].is_finite()) { return f64::NAN; }
        let vc = if v[i] < 2.5 * vave[i] { v[i] } else { 2.5 * vave[i] };
        let (mf, cut) = (typ[i] - typ[i - 1], 0.2 * vin[i] * c[i]);
        if mf > cut { vc } else if mf < -cut { -vc } else { 0.0 }
    }).collect();
    let raw: Vec<f64> = (0..n).map(|i| {
        if i + 1 < p || !vave[i].is_finite() { return f64::NAN; }
        let w = &term[i + 1 - p..=i];
        if w.iter().all(|x| x.is_finite()) { w.iter().sum::<f64>() / vave[i] } else { f64::NAN }
    }).collect();
    ema0(&raw, 3)
}

fn slope(w: &[f64]) -> f64 {
    let n = w.len() as f64;
    let mx = (n - 1.0) / 2.0;
    let my = w.iter().sum::<f64>() / n;
    let num = w.iter().enumerate().fold(0.0, |s, (j, y)| s + (j as f64 - mx) * (y - my));
    let den = (0..w.len()).fold(0.0, |s, j| s + (j as f64 - mx).powi(2));
    num / den
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let vol: Vec<f64> = x.r.iter().map(|r| r.volume.filter(|v| *v > 0.0).unwrap_or(f64::NAN)).collect();
    if id == "sckvfi_t1" {
        let g = week_groups(x);
        let wh: Vec<f64> = g.iter().map(|&(s, e)| super::lc3::hi(&x.h, s, e)).collect();
        let wl: Vec<f64> = g.iter().map(|&(s, e)| super::lc3::lo(&x.l, s, e)).collect();
        let wc: Vec<f64> = g.iter().map(|&(_, e)| x.c[e]).collect();
        let wv: Vec<f64> = g.iter().map(|&(s, e)| if vol[s..=e].iter().all(|v| v.is_finite()) { vol[s..=e].iter().sum() } else { f64::NAN }).collect();
        let e7 = ema0(&vfi(&wh, &wl, &wc, &wv, 26), 7);
        let mut sig = vec![f64::NAN; x.r.len()];
        let ends = period_ends(x, false);
        for (k, &(_, e)) in g.iter().enumerate() { if ends.binary_search(&e).is_ok() { sig[e] = e7[k]; } }
        return Some(next_open(x, from, to, "Mov(VFI,7,E) < 0", |t, pos| {
            let v = sig[t];
            if !v.is_finite() { pos } else if v > 0.0 && pos == 0.0 { 1.0 } else if v < 0.0 && pos > 0.0 { 0.0 } else { pos }
        }));
    }
    let n = x.c.len();
    let f = vfi(&x.h, &x.l, &x.c, &vol, 130);
    let mut p1 = vec![f64::NAN; n];
    let mut lo: Option<f64> = None;
    for i in 0..n { if f[i].is_finite() { let m = lo.map_or(f[i], |m| m.min(f[i])); lo = Some(m); p1[i] = f[i] + m.abs() + 10.0; } }
    let (mut lrsi, mut lrs) = (vec![f64::NAN; n], vec![f64::NAN; n]);
    for i in 120..n {
        let w = &p1[i - 119..=i];
        if w.iter().all(|z| z.is_finite()) && p1[i - 120].is_finite() { lrsi[i] = slope(w) / p1[i - 120].abs() * 100.0; }
        lrs[i] = slope(&x.c[i - 119..=i]) / x.c[i - 120].abs() * 100.0;
    }
    let dv: Vec<f64> = ema0(&(0..n).map(|i| lrsi[i] - lrs[i]).collect::<Vec<_>>(), 3).into_iter().map(|z| z * 100.0).collect();
    let both = p.get("and").copied().unwrap_or(0.0) >= 0.5;
    Some(next_open(x, from, to, "divergence exit", |t, pos| {
        if t < 1 || !(dv[t].is_finite() && dv[t - 1].is_finite() && lrsi[t].is_finite()) { return pos; }
        if pos > 0.0 {
            let out = if both { dv[t] < 0.0 && lrsi[t] < 0.0 } else { dv[t] < 0.0 || lrsi[t] < 0.0 };
            return if out { 0.0 } else { 1.0 };
        }
        if dv[t - 1] > 100.0 && dv[t] < dv[t - 1] && lrsi[t] > 0.0 { 1.0 } else { 0.0 }
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn vfi_tests_run() {
        let r = super::super::sctraub::tests::bars(4000);
        let x = Ctx::new(&r, &r, &[]);
        for id in REVIEWED {
            let t = run(id, &x, 0, r.len() - 1, &Params::new()).unwrap();
            assert!(t.iter().all(|z| z.dir > 0.0 && z.legs[0].px == x.o[z.legs[0].i]), "{id}");
        }
        assert!((slope(&[1.0, 3.0, 5.0, 7.0]) - 2.0).abs() < 1e-12);
    }
}

// Instrument admission: S&C Katsanos VFI 2026-11-02 (titles lane K13).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "sckvfi" && reviewed(def.id),
    label: "Markos Katsanos tested the VFI on single stocks (Corning weekly, Foamex daily). Runs on other stocks, indices and ETFs with their own volume, US and India, are a declared application.",
    basis: "Technical Analysis of Stocks & Commodities V.22:7 pp.50-60 and MetaStock sidebar pp.58-59 (Volume Flow Indicator Performance); independent check authors/titles-1030/K13w/k13_ref.py",
};
