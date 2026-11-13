//! John Sweeney and Steven Notis, "TechniFilter Plus Version 6" (product review), Technical Analysis of Stocks & Commodities
//! V.6:7 (1988) pp.272-274 (research share Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/
//! PR/V06-07/063tech.pdf; Figure 1, the strategy screen, is an image: read with authors/ocr2, PDF p.3). Book id `scsntf`: the
//! reviewers' "combined MACD" system. Lane PA2 (13 Nov 2026); independent check authors/pa2/w/pa2_ref.py. Admission: sclang.rs.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::sctraub::ema0;
use super::tselhs::Book;

pub const REVIEWED: &[&str] = &["scsntf_cmacd"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Adapted: 2 unstated details, R4 #297; unverified] Figure 1 formulas: F2 = CX12 - CX25 (daily MACD: 12- minus 25-day exponential average of the close), F3 = 9-day exponential average of F2, F4 = CX60 - CX125 ('the weekly MACD'), F5 = 45-day exponential average of F4. Long strategy: OPEN F4 > F5 OR (F4 < F5 AND F2 > F3); Close F4 < F5 AND F2 < F3. Short strategy: Open F4 < F5 AND F2 < F3; Close F2 > F3. 'Stop Loss %: 5' ('The system was set up to incorporate a 5% trailing stop'). TechniFilter works on closes ('there is no opening price'), so every signal fills at the close. Exponential averages: alpha 2/(n+1), seeded with the first value; no trade before bar 125 (the longest average). Unstated in the review: (1) the trailing stop - vault reading: TechniFilter's stop is 'a percent of current closing price', so a long is closed at the close that is 5% or more below the highest close since entry (a short: 5% above the lowest close); (2) after a stop - vault reading: the position reopens at a later close where the OPEN condition holds (TechniFilter re-tests the strategy each day). Default side = 1 (the long strategy, 'annual percentage return of 80.8% on the long side'); side = -1 the short strategy; side = 0 both on one account (their opens never overlap). The test (Fidelity Select American Gold, FSAW-X, 08/19/86-11/27/87: long 12 positions, 6 winners; short 7, 4 winners) uses a fund we do not hold, so it stays UNVERIFIED. Runs on indices, ETFs and stocks, US and India, are a declared application.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "scsntf_cmacd", name: "TechniFilter combined MACD: weekly-length MACD 60/125/45 with daily MACD 12/25/9, 5% trailing stop, at the close", group: "S&C: TechniFilter Plus review, combined MACD (Sweeney & Notis, V.6:7)", session: 0, dir: "Long", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "scsntf", rules: &[
            "F|\"Formula 1 (abbreviated as F1) is \"C\" (the close) and F2 is CX12-CX25, which is the formula for the daily MACD ... F3 is the 9-day exponential average of F2. F4 is the weekly MACD (a 60-day minus a 125-day exponential moving average). F5 is the 45-day exponential average of F4.\" (V.6:7, PDF p.2)",
            "E|Figure 1 (PDF p.3, ocr2): \"Long Strategy OPEN: F4>F5 OR (F4<F5 AND F2>F3) Close: F4<F5 AND F2<F3\"; \"Short Strategy Open: F4<F5 AND F2<F3 Close: F2>F3\"",
            "X|Figure 1: \"Stop Loss %: 5\"; \"The system was set up to incorporate a 5% trailing stop\" (PDF p.2); \"it would be nice to be able to express the built-in stop as a fixed price or certain number of points or dollars rather than as a percent of current closing price\" (PDF p.2)",
            "F|Figure 1: FSAW-X 08/19/86 to 11/27/87: long 12 positions (6 wins, 6 losses), annualized 80.8%; short 7 positions (4 wins, 3 losses), annualized 20.7%"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { reviewed(id).then_some(("scsntf_cmacd", "TechniFilter combined MACD (S&C review)", "Side")) }
pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    reviewed(id).then(|| vec![
        ParamDef { key: "side", label: "1 = long strategy (default); -1 = short strategy; 0 = both", def: 1., min: -1., max: 1., step: 1. },
        ParamDef { key: "stop", label: "Trailing stop % from the best close since entry (Figure 1: 5; 0 = none)", def: 5., min: 0., max: 50., step: 0.5 }])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from > to || to >= x.r.len() { return None; }
    let side = p.get("side").copied().unwrap_or(1.).round();
    let stop = p.get("stop").copied().unwrap_or(5.).max(0.) / 100.;
    let c = &x.c;
    let sub = |a: &[f64], b: &[f64]| -> Vec<f64> { a.iter().zip(b).map(|(u, v)| u - v).collect() };
    let f2 = sub(&ema0(c, 12), &ema0(c, 25));
    let f3 = ema0(&f2, 9);
    let f4 = sub(&ema0(c, 60), &ema0(c, 125));
    let f5 = ema0(&f4, 45);
    let mut pos = Book::new();
    let mut best = f64::NAN;
    for t in from..=to {
        if t < 125 { continue; }
        let lo = f4[t] > f5[t] || (f4[t] < f5[t] && f2[t] > f3[t]);
        let lc = f4[t] < f5[t] && f2[t] < f3[t];
        let so = lc;
        let sc = f2[t] > f3[t];
        let s = pos.side();
        if s > 0. {
            best = best.max(c[t]);
            if lc { pos.exit(t, c[t], "long close"); } else if stop > 0. && c[t] <= best * (1. - stop) { pos.exit(t, c[t], "5% trailing stop"); continue; } else { continue; }
        } else if s < 0. {
            best = best.min(c[t]);
            if sc { pos.exit(t, c[t], "short close"); } else if stop > 0. && c[t] >= best * (1. + stop) { pos.exit(t, c[t], "5% trailing stop"); continue; } else { continue; }
        }
        // flat (or just closed by its own rule): open when allowed
        if side >= 0. && lo && t < to { pos.enter(1., t, c[t], "close"); best = c[t]; }
        else if side <= 0. && so && t < to { pos.enter(-1., t, c[t], "close"); best = c[t]; }
    }
    Some(pos.done(to, c[to]))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn combined_macd_long_short_and_stop() {
        let r = super::super::sctraub::tests::bars(2000);
        let x = Ctx::new(&r, &r, &[]);
        let t = run("scsntf_cmacd", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(t.len() > 3 && t.iter().all(|z| z.dir > 0. && z.legs[0].i >= 125));
        let both = run("scsntf_cmacd", &x, 0, r.len() - 1, &Params::from([("side".into(), 0.)])).unwrap();
        assert!(both.iter().any(|z| z.dir < 0.) && both.iter().any(|z| z.dir > 0.));
        for z in t.iter().filter(|z| z.reason.contains("trailing")) {
            let best = x.c[z.legs[0].i..=z.exit_i].iter().copied().fold(f64::MIN, f64::max);
            assert!(z.exit_px <= best * 0.95 + 1e-9);
        }
    }
}
