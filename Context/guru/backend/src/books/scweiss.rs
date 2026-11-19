//! Lane PC (13 Nov 2026; authors/pc/PC.tsv; R4 #297 "up to two unstated details"; pool C row of authors/a1/A1.tsv):
//! Richard L. Weissman, "Boosting Rates Of Return With Noncorrelated Systems", Technical Analysis of Stocks & Commodities
//! V.22:1 (2004) pp.56-58 (research share 'Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/
//! V22/C01/012WEIS.pdf'). Book id `scweiss`.
//! Coded: the 7/29-day moving average crossover and the RSI mean-reversion system with its fail-safe stop.
//! Independent reference: authors/pc/w/pc_ref.py (written from the article, not from this file).
use crate::connors::{rsi, Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::tselhs::Book;

pub const REVIEWED: &[&str] = &["scweiss_rsi", "scweiss_ma7_29"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const RSI_NOTE: &str = "[Adapted: 2 unstated details, R4 #297; unverified] Wilder's RSI of the daily closes. Flat at a close: RSI below 30 buys and RSI above 70 sells short at that close. A long is closed at the first close with RSI above 35, a short at the first close with RSI below 65. Fail-safe stop (the article: 7.5% for the S&P 500, 20% for dated Brent): a resting stop that far from the entry price, checked from the day after entry, filled at the stop or at the open when the day gaps through it; after any exit the rule may enter again at the same close. The article's combined S&P 500 + dated Brent test (3/1993-3/2003): 125 trades, 40% winners; per-market figures are not printed, so the card stays UNVERIFIED. Unstated in the article: (1) the RSI length; vault reading: Wilder's 14 days; (2) when signals are filled; vault reading: at the signal close (option: RSI length).";
const MA_NOTE: &str = "[Adapted: 2 unstated details, R4 #297; unverified] Simple averages of the last 7 and 29 daily closes; the 7-day average crossing above the 29-day average buys (reversing a short), crossing below sells short (reversing a long); always in the market after the first crossing. The article tested dated Brent and spot yen 3/1993-3/2003 (98 and 103 trades; $75 per round turn deducted); neither market is held, so indices, ETFs and stocks, US and India, are a declared application. Unstated in the article: (1) simple or exponential averages; vault reading: simple; (2) when crossings are filled; vault reading: at the crossing close.";

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, dir, rules, notes| StrategyDef { id, name, group: "S&C: Boosting Rates Of Return With Noncorrelated Systems (Weissman, V.22:1)",
        session: 0, dir, status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: "scweiss", rules, notes, claim: None };
    vec![
        d("scweiss_rsi", "Weissman RSI: buy RSI below 30, short above 70; out when RSI recovers through 35 / 65 or at a 7.5% fail-safe stop", "Long/Short",
            &["E|\"it is unrealistic to simply buy whenever RSI closes below 30 and sell when it closes above 70. As a result, to turn this indicator into a mechanical trading system, two issues were addressed: Exiting with profits and exiting with losses.\" (PDF p.2)",
              "X|\"exit long positions whenever the RSI closes above 35 and exit short positions whenever the RSI closes below 65\" (PDF p.2)",
              "X|\"you need a fail-safe exit strategy ... used 7.5% for the Standard & Poor's 500 and 20% for dated Brent to allow for normal market fluctuations\"; \"Fail-safe stop levels: S&P 500 — 7.5% Dated Brent — 20%\" (PDF p.2)"][..], RSI_NOTE),
        d("scweiss_ma7_29", "Weissman 7/29-day moving average crossover, stop and reverse", "Long/Short",
            &["E|\"Using a seven- and 29-day moving average crossover trading system, the following results were achieved through the trading of dated Brent crude oil\" (PDF p.1)",
              "X|\"One of the simplest examples of a mechanical trading system is the two-moving average crossover system.\" (PDF p.1); typical duration \"zero to four months\" (PDF p.1)"][..], MA_NOTE),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match id { "scweiss_rsi" => Some(("scweiss_rsi", "Weissman RSI 30/70 (S&C)", "RSI 14, exits 35/65, 7.5% stop")),
        "scweiss_ma7_29" => Some(("scweiss_ma7_29", "Weissman 7/29 crossover (S&C)", "7/29")), _ => None }
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    match id {
        "scweiss_rsi" => Some(vec![p("n", "RSI length, days (not printed; reading 14)", 14., 2., 50., 1.), p("stop", "Fail-safe stop, % (book: S&P 500 7.5, Brent 20)", 7.5, 1., 30., 0.5)]),
        "scweiss_ma7_29" => Some(vec![p("fast", "Fast average, days (book 7)", 7., 2., 50., 1.), p("slow", "Slow average, days (book 29)", 29., 3., 200., 1.)]),
        _ => None,
    }
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let g = |k: &str, d: f64| p.get(k).copied().unwrap_or(d);
    let mut pos = Book::new();
    if id == "scweiss_ma7_29" {
        let (f, s) = (super::msws::sma(&x.c, g("fast", 7.).round().max(1.) as usize), super::msws::sma(&x.c, g("slow", 29.).round().max(2.) as usize));
        for t in from.max(1)..=to {
            if !(f[t].is_finite() && s[t].is_finite() && f[t - 1].is_finite() && s[t - 1].is_finite()) { continue; }
            if f[t] > s[t] && f[t - 1] <= s[t - 1] { pos.enter(1., t, x.c[t], "close"); }
            else if f[t] < s[t] && f[t - 1] >= s[t - 1] { pos.enter(-1., t, x.c[t], "close"); }
        }
        return Some(pos.done(to, x.c[to]));
    }
    let r = rsi(&x.c, g("n", 14.).round().max(2.) as usize);
    let sp = g("stop", 7.5).max(0.) / 100.;
    for t in from..=to {
        let s = pos.side();
        if s != 0. && pos.entry_bar().is_some_and(|e| t > e) {
            let stop = pos.entry_px() * (1. - s * sp);
            if s > 0. && x.l[t] <= stop { pos.exit(t, x.o[t].min(stop), "fail-safe stop"); }
            else if s < 0. && x.h[t] >= stop { pos.exit(t, x.o[t].max(stop), "fail-safe stop"); }
        }
        if !r[t].is_finite() { continue; }
        let s = pos.side();
        if s > 0. && r[t] > 35. { pos.exit(t, x.c[t], "RSI above 35"); } else if s < 0. && r[t] < 65. { pos.exit(t, x.c[t], "RSI below 65"); }
        if pos.side() == 0. { if r[t] < 30. { pos.enter(1., t, x.c[t], "close"); } else if r[t] > 70. { pos.enter(-1., t, x.c[t], "close"); } }
    }
    Some(pos.done(to, x.c[to]))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn weissman_rsi_and_crossover() {
        let r = super::super::sctraub::tests::bars(3000);
        let x = Ctx::new(&r, &r, &[]);
        let rs = rsi(&x.c, 14);
        let t = run("scweiss_rsi", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(t.iter().any(|z| z.dir > 0.) && t.iter().any(|z| z.dir < 0.));
        for z in &t {
            let e = z.legs[0].i;
            assert!(if z.dir > 0. { rs[e] < 30. } else { rs[e] > 70. });
            if !z.open && z.reason != "fail-safe stop" { assert!(if z.dir > 0. { rs[z.exit_i] > 35. } else { rs[z.exit_i] < 65. }); }
        }
        let mut p = Params::new(); p.insert("stop".into(), 1.);
        assert!(run("scweiss_rsi", &x, 0, r.len() - 1, &p).unwrap().iter().any(|z| z.reason == "fail-safe stop"));
        let m = run("scweiss_ma7_29", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(m.len() > 5);
        for w in m.windows(2) { assert_eq!(w[1].legs[0].i, w[0].exit_i); assert_eq!(w[1].dir, -w[0].dir); }
    }
}

// Instrument admission: Weissman RSI and 7/29 crossover daily application 2026-11-13 (lane PC, R4 #297).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "scweiss" && reviewed(def.id),
    label: "Richard Weissman tested the RSI system on the spot S&P 500 and dated Brent and the 7/29 crossover on dated Brent and spot yen (1993-2003). Daily bars of indices, ETFs and stocks, US and India, are a declared application; two unstated details per card are read as labelled (R4 #297).",
    basis: "Technical Analysis of Stocks & Commodities V.22:1 pp.56-58; R4 #297; independent check authors/pc/w/pc_ref.py",
};
