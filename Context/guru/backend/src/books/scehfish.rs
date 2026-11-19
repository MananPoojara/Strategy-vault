//! John F. Ehlers, "Using The Fisher Transform", Technical Analysis of Stocks & Commodities V.20:11 (2002) pp.40-42
//! (research share Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V20/C11/130fish.pdf).
//! Book id `scehfish`: the crossover system of the Fisher transform and its one-bar-delayed trigger (Figure 5 EasyLanguage).
//! Lane PA2 (13 Nov 2026); independent check authors/pa2/w/pa2_ref.py. Admission: sclang.rs (PA2_BOOKS).
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::sctraub::next_open;

pub const REVIEWED: &[&str] = &["scehfish_cross"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Adapted: 2 unstated details, R4 #297; unverified] Figure 5, as printed: Price = (H + L) / 2; MaxH / MinL = highest / lowest Price of the last Len = 10 bars; Value1 = .33 x 2 x ((Price - MinL) / (MaxH - MinL) - .5) + .67 x Value1[1], set to .999 above .99 and to -.999 below -.99; Fish = .5 x ln((1 + Value1) / (1 - Value1)) + .5 x Fish[1]; Fish and 'Fish-delayed-by-one-bar' (the trigger) 'provide a crossover system that identifies the cyclic turning points'. Both start at 0 (EasyLanguage variables) and are computed from the 10th bar. Buy when Fish crosses above the trigger (Fish > Fish[1] after Fish[1] <= Fish[2]); sell when it crosses below. Unstated in the article: (1) the fill - vault reading: the next bar's open (TradeStation's 'next bar at market', the platform of the printed code); (2) long and short - vault reading: stop and reverse (the crossings mark both tops and bottoms; option short = 0: long or flat). A 10-bar range of zero (MaxH = MinL; the code would divide by zero) adds nothing to Value1 that bar. The article prints charts only (no trade table). ehmd_osc_family (the MESA article set) lists the Fisher transform as an indicator only; this card codes the S&C article's crossover sentence. Runs on indices, ETFs and stocks, US and India, are a declared application.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "scehfish_cross", name: "Ehlers Fisher transform (10-bar, (H+L)/2) crossing its one-bar trigger: stop and reverse at the next open", group: "S&C: using the Fisher transform (Ehlers, V.20:11)", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "scehfish", rules: &[
            "F|\"Vars: Price((H+L)/2), Len(10); MaxH(0), MinL(0), Fish(0); MaxH=Highest(Price,Len); MinL=Lowest(Price,Len); Value1 = .33*2*((Price-MinL)/(MaxH-MinL)-.5)+.67*Value1[1]; If Value1 > .99 then Value1= .999; If Value1 <-.99 then Value1=-.999; Fish = .5*Log((1+Value1)/(1-Value1)) + .5*Fish[1]; Plot1(Fish, \"Fisher\"); Plot2(Fish[1], \"Trigger\");\" (V.20:11, Figure 5, PDF p.2)",
            "E|\"Both Fish and Fish-delayed-by-one-bar are plotted to provide a crossover system that identifies the cyclic turning points.\" (PDF p.2)",
            "X|\"The sharp turning points of these peak swings clearly and unambiguously identify price reversals in a timely manner.\" (PDF p.3): the opposite crossing reverses"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { reviewed(id).then_some(("scehfish_cross", "Ehlers Fisher transform crossover (S&C)", "10")) }
pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    reviewed(id).then(|| vec![
        ParamDef { key: "len", label: "Len: channel bars (article 10)", def: 10., min: 3., max: 60., step: 1. },
        ParamDef { key: "short", label: "1 = stop and reverse (vault reading); 0 = long or flat", def: 1., min: 0., max: 1., step: 1. }])
}

pub fn fish(x: &Ctx, len: usize) -> Vec<f64> {
    let n = x.c.len();
    let price: Vec<f64> = (0..n).map(|i| (x.h[i] + x.l[i]) / 2.).collect();
    let (mut v, mut f) = (vec![0.0; n], vec![0.0; n]);
    for i in 0..n {
        let (pv, pf) = if i > 0 { (v[i - 1], f[i - 1]) } else { (0., 0.) };
        if i + 1 < len { v[i] = pv; f[i] = pf; continue; }
        let w = &price[i + 1 - len..=i];
        let (mx, mn) = (w.iter().copied().fold(f64::MIN, f64::max), w.iter().copied().fold(f64::MAX, f64::min));
        let r = if mx > mn { (price[i] - mn) / (mx - mn) - 0.5 } else { 0. };
        let mut val = 0.33 * 2. * r + 0.67 * pv;
        if val > 0.99 { val = 0.999; } else if val < -0.99 { val = -0.999; }
        v[i] = val;
        f[i] = 0.5 * ((1. + val) / (1. - val)).ln() + 0.5 * pf;
    }
    f
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from > to || to >= x.r.len() { return None; }
    let len = p.get("len").copied().unwrap_or(10.).round().max(2.) as usize;
    let dn = if p.get("short").copied().unwrap_or(1.) > 0.5 { -1. } else { 0. };
    let f = fish(x, len);
    Some(next_open(x, from, to, "Fisher crossed its trigger", |t, side| {
        if t < len + 1 { return side; }
        if f[t] > f[t - 1] && f[t - 1] <= f[t - 2] { 1. } else if f[t] < f[t - 1] && f[t - 1] >= f[t - 2] { dn } else { side }
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fisher_crossovers_reverse_at_the_next_open() {
        let r = super::super::sctraub::tests::bars(1500);
        let x = Ctx::new(&r, &r, &[]);
        let f = fish(&x, 10);
        assert!(f.iter().all(|v| v.is_finite()) && f[..9].iter().all(|v| *v == 0.));
        let t = run("scehfish_cross", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(t.len() > 20 && t.iter().any(|z| z.dir < 0.));
        for z in &t { let i = z.legs[0].i; assert_eq!(z.legs[0].px, x.o[i]); assert!(if z.dir > 0. { f[i - 1] > f[i - 2] } else { f[i - 1] < f[i - 2] }); }
        assert!(run("scehfish_cross", &x, 0, r.len() - 1, &Params::from([("short".into(), 0.)])).unwrap().iter().all(|z| z.dir > 0.));
    }
}
