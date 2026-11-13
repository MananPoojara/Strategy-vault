//! M.H. Pee's indicator reversal systems in Technical Analysis of Stocks & Commodities (research share Trading Knowledge I/
//! Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/): "Trend Continuation Factor" V.20:3 (March 2002)
//! pp.58-64 (V20/C03/050TREN.pdf, book `scpeetcf`), "Trend Intensity Index" V.20:6 (June 2002) pp.32-35 (V20/C06/113TRND.pdf,
//! book `scpeetii`), "Phase Change Index" V.22:5 (May 2004) pp.28-37 (V22/C05/094PEE.pdf, book `scpeepci`). His Trend
//! Trigger Factor is `scpee`. Lane PA1 (13 Nov 2026, pool A of authors/a1/A1.tsv). Independent check authors/pa1/pa1_ref.py.
use crate::connors::{sma, Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::tselhs::Book;

pub const REVIEWED: &[&str] = &["scpee_tcf", "scpee_tii", "scpee_pci"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const TCF: &str = "[Complete rule (the sidebar's Excel formulas); unverified] +change = today's close - yesterday's when the close rose, else 0; -change = yesterday's close - today's when it fell, else 0. +CF = 0 when +change is 0, else +change + yesterday's +CF; -CF likewise. 35-day +TCF = sum of +change over the last 35 days - sum of -CF over the last 35 days; 35-day -TCF = sum of -change - sum of +CF. Long at tomorrow's open when today's +TCF is positive, short at tomorrow's open when today's -TCF is positive; held until the opposite signal (a reversal system). Article (15 futures, continuous contracts, 1/4/1982-12/31/1998, one contract, $75 a trade): e.g. T-bonds 49 trades, 38.78% winners; British pound 55, 32.73%; crude oil 43, 44.19%; gold 54, 35.19%; yen 34, 64.71%; Swiss franc 47, 44.68%; silver 52, 42.31% (Figure 1). Runs on indices, ETFs and stocks, US and India, are a declared application.";
const TII: &str = "[Complete rule (the sidebar's Excel formulas); unverified] 60-day simple average of the close (today's value); deviations of each of the last 30 closes from it; SD+ = sum of the up deviations, SD- = sum of the down deviations; 30-day TII = SD+ / (SD+ + SD-) x 100 (the sidebar: TII = ((G + H) / 2) / G x 100 with G = the sum of absolute deviations, H = their signed sum). Position for tomorrow (the sidebar's column K, 'IF(J61>80,1,IF(J61<20,-1,K60))'): long when TII is above 80, short when below 20, else unchanged; trades at the next open. Article (15 futures, 1/4/1982-12/31/1998, one contract, $75 a trade): 697 trades in all, 40.6% winners (Figure 1). Runs on indices, ETFs and stocks, US and India, are a declared application.";
const PCI: &str = "[Adapted: 1 unstated detail, R4 #297; unverified] Over the last 35 closes (E1 the oldest, E35 today): slope F = (E35 - E1) / 34, gradient line E1 + k x F; D+ / D- = the sums of the closes' up / down deviations from that line; 35-day PCI = D+ / (D+ + D-) x 100 (the sidebar's Excel). Long at tomorrow's open when the 35-day momentum is positive and PCI is below 20; short at tomorrow's open when momentum is negative and PCI is above 80; otherwise the position stays (a reversal system). Unstated in the article: (1) the momentum's look-back, written 'the close 35 days ago subtracted from the current day's close' while its spreadsheet uses the 35-close window (E35 - E1, 34 days apart) that defines the gradient line whose slope 'the 35-day momentum line measures'; vault reading: E35 - E1 (the sign of the line's slope). Article (35 futures, 1/2/1980-4/30/2003, one contract, $75 a trade): 26 of 35 markets profitable with the 35-day PCI (Figures 8-9). Runs on indices, ETFs and stocks, US and India, are a declared application.";

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, group, book, notes, rules| StrategyDef { id, name, group, session: 0, dir: "Long/Short", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book, rules, notes, claim: None };
    vec![
        d("scpee_tcf", "Pee Trend Continuation Factor (35): long while +TCF turns positive, short when -TCF does, next open", "S&C: Trend Continuation Factor (Pee, V.20:3)", "scpeetcf", TCF, &[
            "F|\"35-day + TCF = Sum of (+change) for the last 35 days - sum of (-CF) for the last 35 days; 35-day - TCF = Sum of (-change) for the last 35 days - sum of (+CF) of the last 35 days\" (V.20:3 p.59)",
            "E|\"initiate your long positions at tomorrow's open if today's +TCF value is positive. Enter short positions at tomorrow's open if the -TCF is positive today\" (p.60)",
            "X|\"the TCF is tested as a reversal system; once a position is initiated, you will stay with it until a signal to enter in the opposite is issued\" (p.60)"]),
        d("scpee_tii", "Pee Trend Intensity Index (30/60): long above 80, short below 20, next open, reversal", "S&C: Trend Intensity Index (Pee, V.20:6)", "scpeetii", TII, &[
            "F|\"30-day TII = (SD+)/((SD+)+(SD-))*100\"; deviations of the recent 30 closes from the 60-day moving average (V.20:6 p.32)",
            "E|\"make a long trade at the open on the following trading day only when the TII value today is above 80. Similarly, you should open a short trade next day if the value today is below 20.\" (p.33)",
            "X|\"Then remain with the position until the TII issues a signal to enter in the opposite direction.\" (p.33)"]),
        d("scpee_pci", "Pee Phase Change Index (35): long on rising momentum with PCI under 20, short on falling momentum above 80, next open", "S&C: Phase Change Index (Pee, V.22:5)", "scpeepci", PCI, &[
            "F|\"35-day PCI = [(D+) / ((D+) + (D-))] * 100\" with deviations from the gradient line connecting the starting and ending closes (V.22:5 p.30); \"=(E35-E1)/34\" (sidebar)",
            "E|\"enter long at the open tomorrow if today's 35-day momentum is positive and the 35-day PCI value is less than 20. Enter short at the open tomorrow if today's 35-day momentum is negative and the 35-day PCI has a value greater than 80. Otherwise, let your current position remain.\" (p.30-31)",
            "X|\"The PCI was tested as a reversal system\": a long is closed when a short signal reverses it, and vice versa (p.31)"]),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match id { "scpee_tcf" => Some(("scpee_tcf", "Pee Trend Continuation Factor (S&C)", "35")), "scpee_tii" => Some(("scpee_tii", "Pee Trend Intensity Index (S&C)", "30/60")),
        "scpee_pci" => Some(("scpee_pci", "Pee Phase Change Index (S&C)", "35")), _ => None }
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    match id {
        "scpee_tcf" => Some(vec![p("n", "TCF days (article 35)", 35., 5., 100., 1.)]),
        "scpee_tii" => Some(vec![p("n", "TII days (article 30)", 30., 5., 100., 1.), p("ma", "Moving average days (article 60)", 60., 10., 200., 1.)]),
        "scpee_pci" => Some(vec![p("n", "PCI days (article 35; Figure 9 also 21-56)", 35., 5., 100., 1.)]),
        _ => None,
    }
}

/// Target position after each close (1, -1, 0 = unchanged).
fn signals(id: &str, x: &Ctx, p: &Params) -> Vec<f64> {
    let g = |k: &str, d: f64| p.get(k).copied().unwrap_or(d);
    let c = &x.c;
    let m = c.len();
    let mut out = vec![0.; m];
    match id {
        "scpee_tcf" => {
            let n = g("n", 35.).round().max(1.) as usize;
            let (mut up, mut dn, mut pcf, mut ncf) = (vec![0.; m], vec![0.; m], vec![0.; m], vec![0.; m]);
            for i in 1..m {
                up[i] = (c[i] - c[i - 1]).max(0.); dn[i] = (c[i - 1] - c[i]).max(0.);
                pcf[i] = if up[i] == 0. { 0. } else { up[i] + pcf[i - 1] };
                ncf[i] = if dn[i] == 0. { 0. } else { dn[i] + ncf[i - 1] };
            }
            for i in n..m {
                let s = |v: &[f64]| v[i + 1 - n..=i].iter().sum::<f64>();
                if s(&up) - s(&ncf) > 0. { out[i] = 1.; } else if s(&dn) - s(&pcf) > 0. { out[i] = -1.; }
            }
        }
        "scpee_tii" => {
            let (n, man) = (g("n", 30.).round().max(1.) as usize, g("ma", 60.).round().max(1.) as usize);
            let ma = sma(c, man);
            for i in (n.max(man) - 1)..m {
                if !ma[i].is_finite() { continue; }
                let (mut gs, mut hs) = (0., 0.);
                for k in i + 1 - n..=i { gs += (c[k] - ma[i]).abs(); hs += c[k] - ma[i]; }
                if gs == 0. { continue; }
                let tii = (gs + hs) / 2. / gs * 100.;
                if tii > 80. { out[i] = 1.; } else if tii < 20. { out[i] = -1.; }
            }
        }
        _ => {
            let n = g("n", 35.).round().max(3.) as usize;
            for i in (n - 1)..m {
                let e1 = c[i + 1 - n];
                let f = (c[i] - e1) / (n - 1) as f64;
                let (mut gs, mut hs) = (0., 0.);
                for k in 1..n - 1 { let dv = c[i + 1 - n + k] - e1 - k as f64 * f; gs += dv.abs(); hs += dv; }
                if gs == 0. { continue; }
                let pci = (gs + hs) / 2. / gs * 100.;
                let mom = c[i] - e1;
                if mom > 0. && pci < 20. { out[i] = 1.; } else if mom < 0. && pci > 80. { out[i] = -1.; }
            }
        }
    }
    out
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let s = signals(id, x, p);
    let mut pos = Book::new();
    for t in from..to { if s[t] != 0. { pos.enter(s[t], t + 1, x.o[t + 1], "next open"); } }
    Some(pos.done(to, x.c[to]))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pee_reversal_systems() {
        let r = super::super::sctraub::tests::bars(3000);
        let x = Ctx::new(&r, &r, &[]);
        for id in REVIEWED {
            let t = run(id, &x, 0, r.len() - 1, &Params::new()).unwrap();
            assert!(t.len() > 4 && t.iter().any(|z| z.dir < 0.), "{id}");
            let s = signals(id, &x, &Params::new());
            for z in &t { let e = z.legs[0].i; assert_eq!(s[e - 1], z.dir); assert_eq!(z.legs[0].px, x.o[e]); }
            for w in t.windows(2) { assert_eq!(w[1].legs[0].i, w[0].exit_i); }
        }
        // the article's five-day PCI example (Figure 7): 7.23
        let c = [35.41, 35.01, 35.62, 36.88, 37.21];
        let f = (c[4] - c[0]) / 4.;
        let (mut g, mut h) = (0., 0.);
        for k in 1..4 { let d = c[k] - c[0] - k as f64 * f; g += d.abs(); h += d; }
        assert!(((g + h) / 2. / g * 100. - 7.23).abs() < 0.01);
    }
}

// Instrument admission: S&C Pee TCF / TII / PCI 2026-11-13 (lane PA1).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| matches!(def.book, "scpeetcf" | "scpeetii" | "scpeepci") && reviewed(def.id),
    label: "M.H. Pee tested these reversal systems on baskets of futures (continuous contracts); indices, ETFs and stocks, US and India, are a declared application (no fixed amounts).",
    basis: "Technical Analysis of Stocks & Commodities V.20:3 pp.58-64 (Trend Continuation Factor), V.20:6 pp.32-35 (Trend Intensity Index), V.22:5 pp.28-37 (Phase Change Index); independent check authors/pa1/pa1_ref.py",
};
