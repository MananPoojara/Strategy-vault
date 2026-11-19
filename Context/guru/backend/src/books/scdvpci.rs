//! Buff Pelz Dormeier, "Between Price And Volume" (VPCI), Technical Analysis of Stocks & Commodities V.25:7 (July 2007)
//! pp.20-30, and the TradeStation VPCI strategy of that issue's Traders' Tips pp.74-75 (research share Trading Knowledge I/
//! Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V25/C07/129DORM.pdf and 142TIPS.pdf). Book id `scdvpci`.
//! Lane PA1 (13 Nov 2026, pool A of authors/a1/A1.tsv). The book version (bdvwma, Investing with Volume Analysis) keeps its
//! VPCI-confirmed crossover held (periods not stated there); this card runs the printed Traders' Tips strategy.
//! Independent check authors/pa1/pa1_ref.py.
use crate::connors::{sma, Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::sctraub::ema0;
use super::tselhs::Book;

pub const REVIEWED: &[&str] = &["scdvpci_macd"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Complete rule (printed EasyLanguage, Traders' Tips V.25:7 p.74-75, TradeStation's strategy for the article); unverified] MACD = XAverage(close,12) - XAverage(close,26) (TradeStation exponential averages, seeded with the first close), signal = 9-bar simple average of MACD. VPCI (Length1 5, Length2 20): VP = 20-bar volume-weighted average close - 20-bar simple average close; VPR = 5-bar volume-weighted average close / 5-bar simple average of the LOW (as the TradeStation code prints it); VM = 5-bar / 20-bar average volume; VPCI = VP x VPR x VM; smoothed VPCI = its 20-bar simple average. Buy next bar at market when MACD crosses over its signal and VPCI is above its smoothed value; sell short next bar at market when MACD crosses under its signal and VPCI is below its smoothed value (each reverses the other: always in after the first signal). The code's ADX and OBV confirmations are switched off by default (UseADX 0, UseOBV 0) and are not offered. Option shorts = 0: long only, a sell signal closes the long (the article's own test was long only: SPY Feb 1993-2006, VPCI 8.11% a year, 65.15% profitable, profit factor 2.47, invested 35.63% of the time, Figure 6; its exact entry combination with ADX(7) is not fully stated, so no book check). Fills at the next open. Runs on indices, ETFs and stocks with volume, US and India, are a declared application.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "scdvpci_macd", name: "Dormeier VPCI: MACD(12,26,9) crossovers confirmed by VPCI(5,20) against its 20-bar average, stop and reverse", group: "S&C: Between Price And Volume (Dormeier, V.25:7)", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: true, needs_vix: false, needs_series: &[], book: "scdvpci", rules: &[
            "F|\"VP = VWMA2 - Average( Price, Length2 ) ; VPR = VWMA1 / Average( Low, Length1 ) ; VM = Average( VolValue, Length1 ) / Average( VolValue, Length2 ) ; VPCI = VP * VPR * VM ; AvgVPCI = Average( VPCI, VPCIAvgLen ) ;\" Length1 5, Length2 20, VPCIAvgLen 20 (V.25:7 Traders' Tips p.74)",
            "E|\"if MACDValue crosses over AvgMACD and ( ( VPCI > AvgVPCI and UseVPCI = 1 ) or UseVPCI <> 1 ) ... then Buy next bar at market\" (p.75)",
            "X|\"else if MACDValue crosses under AvgMACD and ( ( VPCI < AvgVPCI and UseVPCI = 1 ) or UseVPCI <> 1 ) ... then SellShort next bar at market\" (p.75)"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    reviewed(id).then_some(("scdvpci_macd", "Dormeier VPCI-confirmed MACD (S&C)", "5/20"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    reviewed(id).then(|| vec![p("l1", "VPCI short length (code 5)", 5., 2., 50., 1.), p("l2", "VPCI long length (code 20)", 20., 5., 200., 1.),
        p("shorts", "1 = stop and reverse (code), 0 = long only", 1., 0., 1., 1.)])
}

/// VPCI per the Traders' Tips code; NaN where volume or history is missing.
pub fn vpci(x: &Ctx, l1: usize, l2: usize) -> Vec<f64> {
    let n = x.c.len();
    let v: Vec<f64> = x.r.iter().map(|r| r.volume.filter(|v| *v > 0.).unwrap_or(f64::NAN)).collect();
    let pv: Vec<f64> = (0..n).map(|i| x.c[i] * v[i]).collect();
    let (spv1, sv1, spv2, sv2) = (sma(&pv, l1), sma(&v, l1), sma(&pv, l2), sma(&v, l2));
    let (sc2, sl1) = (sma(&x.c, l2), sma(&x.l, l1));
    (0..n).map(|i| {
        let (w1, w2) = (spv1[i] / sv1[i], spv2[i] / sv2[i]);
        (w2 - sc2[i]) * (w1 / sl1[i]) * (sv1[i] / sv2[i])
    }).collect()
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let g = |k: &str, d: f64| p.get(k).copied().unwrap_or(d);
    let (l1, l2) = (g("l1", 5.).round().max(1.) as usize, g("l2", 20.).round().max(1.) as usize);
    let shorts = g("shorts", 1.) >= 0.5;
    let (e12, e26) = (ema0(&x.c, 12), ema0(&x.c, 26));
    let m: Vec<f64> = (0..x.c.len()).map(|i| e12[i] - e26[i]).collect();
    let ms = sma(&m, 9);
    let vp = vpci(x, l1, l2);
    let va = sma(&vp, 20);
    let mut pos = Book::new();
    for t in from.max(1)..to {
        let up = m[t] > ms[t] && m[t - 1] <= ms[t - 1];
        let dn = m[t] < ms[t] && m[t - 1] >= ms[t - 1];
        let i = t + 1;
        if up && vp[t] > va[t] { pos.enter(1., i, x.o[i], "next open"); }
        else if dn && vp[t] < va[t] { if shorts { pos.enter(-1., i, x.o[i], "next open"); } else if pos.side() > 0. { pos.exit(i, x.o[i], "sell signal"); } }
    }
    Some(pos.done(to, x.c[to]))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn vpci_confirmed_macd() {
        let r = super::super::sctraub::tests::bars(3000);
        let x = Ctx::new(&r, &r, &[]);
        let t = run("scdvpci_macd", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(t.len() > 10 && t.iter().any(|z| z.dir < 0.));
        let (vp, va) = { let v = vpci(&x, 5, 20); let a = sma(&v, 20); (v, a) };
        for z in &t { let s = z.legs[0].i - 1; assert!(z.dir * (vp[s] - va[s]) > 0. && z.legs[0].px == x.o[s + 1]); }
        let mut p = Params::new(); p.insert("shorts".into(), 0.);
        assert!(run("scdvpci_macd", &x, 0, r.len() - 1, &p).unwrap().iter().all(|z| z.dir > 0.));
    }
}

// Instrument admission: S&C Dormeier VPCI (Traders' Tips strategy) 2026-11-13 (lane PA1).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "scdvpci" && reviewed(def.id),
    label: "Buff Dormeier tested VPCI confirmation on SPY (1993-2006); TradeStation's printed strategy runs on indices, ETFs and stocks with volume, US and India, as a declared application.",
    basis: "Technical Analysis of Stocks & Commodities V.25:7 pp.20-30 (Between Price And Volume) and Traders' Tips pp.74-75 (TradeStation VPCI strategy); independent check authors/pa1/pa1_ref.py",
};
