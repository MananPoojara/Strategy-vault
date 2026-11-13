//! Harley D. Wilbur, "A Twist On The Arms Index", Technical Analysis of Stocks & Commodities V.10:11 (November 1992)
//! pp.449-453 (research share Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V10/C11/
//! TWISTON.PDF). Book id `scwtrin`. Lane PA1 (13 Nov 2026, pool A of authors/a1/A1.tsv). Independent check authors/pa1/pa1_ref.py.
use crate::connors::{sma, Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::tselhs::Book;

pub const REVIEWED: &[&str] = &["scwtrin_wtrin10"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const AD: &[&str] = &["NYSE_ADV_DEC.Advances", "NYSE_ADV_DEC.Declines", "NYSE_ADV_DEC.AdvVolume", "NYSE_ADV_DEC.DecVolume"];

const NOTE: &str = "[Complete rule; unverified] Open TRIN10 = (10-day average of NYSE advancing issues / 10-day average of declining issues) / (10-day average of advancing volume / 10-day average of declining volume) (the sidebar's cell G20). Bollinger bands = its 10-day average +/- 2 population standard deviations (STDEVP) of the last 10 TRIN10 values; normalised value = 100 x (upper band - TRIN10) / (upper band - lower band); WTRIN10 = its five-day weighted average (weights 5,4,3,2,1 / 15). Buy at the close of the first day WTRIN10 rises after it has been below 20 while flat; sell at the close of the first day it falls after it has been above 80 while long ('Buy when WTRIN10 has an uptick after falling below 20, hold until it goes above 80, and then sell on its first downtick'; 'the trades are made at the close on the day of the signal'). Long only. Breadth: the vault's NYSE advances / declines / up and down volume (NYSE_ADV_DEC, 1965-03-01 to 2020-02-10), so results end in February 2020. The article trades the S&P 500 (an index fund) and prints its 14 trades of May 1990 - June 1992 (p.451-452); the breadth timing is applied to other US indices, ETFs and stocks as a declared application. India is refused (NYSE breadth).";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "scwtrin_wtrin10", name: "Wilbur WTRIN10: buy the first uptick of the banded open TRIN10 after it dips under 20, sell the first downtick after 80", group: "S&C: A Twist On The Arms Index (Wilbur, V.10:11)", session: 0, dir: "Long", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: AD, book: "scwtrin", rules: &[
            "F|\"=(AVERAGE(C11:C20)/AVERAGE(D11:D20))/(AVERAGE(E11:E20)/AVERAGE(F11:F20))\"; \"=STDEVP(G11:G20)\"; \"=100*(J20-G20)/(J20-K20)\"; \"=((5*L20)+(4*L19)+(3*L18)+(2*L17)+L16)/15\" (V.10:11 sidebar, p.453)",
            "E|\"Buy when WTRIN10 has an uptick after falling below 20\" (p.451)",
            "X|\"hold until it goes above 80, and then sell on its first downtick\"; \"the trades are made at the close on the day of the signal\" (p.451)"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    reviewed(id).then_some(("scwtrin_wtrin10", "Wilbur weighted open TRIN (S&C)", "20 / 80"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    reviewed(id).then(|| vec![p("buy", "Buy zone: below (article 20)", 20., 0., 50., 1.), p("sell", "Sell zone: above (article 80)", 80., 50., 100., 1.)])
}

/// WTRIN10 per the sidebar spreadsheet; NaN where the breadth is missing or the bands have no width.
pub fn wtrin10(adv: &[f64], dec: &[f64], av: &[f64], dv: &[f64]) -> Vec<f64> {
    let (a, d, u, w) = (sma(adv, 10), sma(dec, 10), sma(av, 10), sma(dv, 10));
    let g: Vec<f64> = (0..adv.len()).map(|i| (a[i] / d[i]) / (u[i] / w[i])).collect();
    let l: Vec<f64> = (0..g.len()).map(|i| {
        if i < 9 { return f64::NAN; }
        let s = &g[i - 9..=i];
        if s.iter().any(|v| !v.is_finite()) { return f64::NAN; }
        let m = s.iter().sum::<f64>() / 10.;
        let sd = (s.iter().map(|v| (v - m).powi(2)).sum::<f64>() / 10.).sqrt();
        if sd == 0. { f64::NAN } else { 100. * ((m + 2. * sd) - g[i]) / (4. * sd) }
    }).collect();
    (0..l.len()).map(|i| if i < 4 { f64::NAN } else { (5. * l[i] + 4. * l[i - 1] + 3. * l[i - 2] + 2. * l[i - 3] + l[i - 4]) / 15. }).collect()
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let Some(s) = AD.iter().map(|k| x.series(k)).collect::<Option<Vec<&[f64]>>>() else { return Some(vec![]) }; // no breadth: no trades
    let wt = wtrin10(s[0], s[1], s[2], s[3]);
    let (lo, hi) = (p.get("buy").copied().unwrap_or(20.), p.get("sell").copied().unwrap_or(80.));
    let mut pos = Book::new();
    let mut armed = false;
    for t in from.max(1)..=to {
        let (w, w1) = (wt[t], wt[t - 1]);
        if !w.is_finite() { continue; }
        if pos.side() == 0. {
            if w < lo { armed = true; }
            if armed && w1.is_finite() && w > w1 { pos.enter(1., t, x.c[t], "close"); armed = false; }
        } else {
            if w > hi { armed = true; }
            if armed && w1.is_finite() && w < w1 { pos.exit(t, x.c[t], "WTRIN10 downtick after 80"); armed = false; }
        }
    }
    Some(pos.done(to, x.c[to]))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn wtrin_cycle() {
        let r = super::super::sctraub::tests::bars(1500);
        let mut x = Ctx::new(&r, &r, &[]);
        let n = r.len();
        let f = |k: f64, p: f64| (0..n).map(|i| 1000. + 400. * ((i as f64) / p + k).sin()).collect::<Vec<f64>>();
        x.extra.insert(AD[0].into(), f(0., 9.)); x.extra.insert(AD[1].into(), f(1.5, 9.));
        x.extra.insert(AD[2].into(), f(0.3, 13.)); x.extra.insert(AD[3].into(), f(2., 7.));
        let t = run("scwtrin_wtrin10", &x, 0, n - 1, &Params::new()).unwrap();
        let s: Vec<&[f64]> = AD.iter().map(|k| x.series(k).unwrap()).collect();
        let w = wtrin10(s[0], s[1], s[2], s[3]);
        assert!(t.len() > 5);
        for z in &t {
            let e = z.legs[0].i; assert!(w[e] > w[e - 1] && z.legs[0].px == x.c[e]);
            assert!((0..e).rev().take_while(|&k| k > 0).any(|k| w[k] < 20.));
            if !z.open { assert!(w[z.exit_i] < w[z.exit_i - 1] && (e + 1..z.exit_i).any(|k| w[k] > 80.)); }
        }
        x.extra.remove(AD[3]);
        assert!(run("scwtrin_wtrin10", &x, 0, n - 1, &Params::new()).unwrap().is_empty());
    }
}

// Instrument admission: S&C Wilbur WTRIN10 2026-11-13 (lane PA1).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "scwtrin" && reviewed(def.id),
    label: "Harley Wilbur times the S&P 500 with NYSE breadth (advances, declines, up and down volume); other US indices, ETFs and stocks are a declared application of the same timing. India is refused: NYSE breadth has no India counterpart in the vault.",
    basis: "Technical Analysis of Stocks & Commodities V.10:11 pp.449-453 (A Twist On The Arms Index); independent check authors/pa1/pa1_ref.py",
};
