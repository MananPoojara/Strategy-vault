//! Lane PC (13 Nov 2026; authors/pc/PC.tsv; R4 #297 "up to two unstated details"; pool C row of authors/a1/A1.tsv):
//! Gilbert Raff, "The Relative Strength Ratio-MACD Crossover", Technical Analysis of Stocks & Commodities V.12:11 (1994)
//! pp.451-454 (research share 'Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V12/C11/
//! RELATIV.PDF'). Book id `scraff`.
//! Coded: the weekly MACD of the relative strength ratio (security / S&P 500) against its 9-week EMA trigger line, the fund
//! rule and the stock rule (buy only with the MACD above zero).
//! Independent reference: authors/pc/w/pc_ref.py (written from the article, not from this file).
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::tselhs::Book;

pub const REVIEWED: &[&str] = &["scraff_rs_macd", "scraff_rs_macd_stock"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

macro_rules! common { () => { " Weekly bars: each week's last trading day closes the week. Relative strength ratio = the security's weekly close / the market benchmark's close on the same day, or its last earlier close when the benchmark has no bar (#223; withheld above 5%) (S&P 500 for US instruments as printed; NIFTY 50 for Indian ones, a declared application). MACD = 12-week EMA - 26-week EMA of the ratio; trigger = 9-week EMA of the MACD (EMAs: alpha 2/(n+1), seeded with the first value, MetaStock style). Long only; out (cash) after a sell. Unstated in the article: (1) the MACD averages; vault reading: the classic 12 and 26 (options); (2) when a signal is filled; vault reading: at the close of the signal week. The article shows charts only (no trade table), so the card stays UNVERIFIED; the benchmark itself has a flat ratio and gives no signals." } }
const FUND: &str = concat!("[Adapted: 2 unstated details, R4 #297; unverified] Fund rule: long while the MACD is above its trigger line (buy at the first weekly close above it, sell at the first weekly close below it).", common!());
const STOCK: &str = concat!("[Adapted: 2 unstated details, R4 #297; unverified] Stock rule: as the fund rule, but a buy also needs the MACD above zero ('buy signals incorporate the added rule that the MACD line be in positive territory'); the sell is unchanged (MACD below its trigger line).", common!());

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, rules, notes| StrategyDef { id, name, group: "S&C: The Relative Strength Ratio-MACD Crossover (Raff, V.12:11)",
        session: 0, dir: "Long", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &["BENCH.C"], book: "scraff", rules, notes, claim: None };
    vec![
        d("scraff_rs_macd", "Raff relative strength MACD (fund rule): weekly MACD of the ratio to the S&P 500 above its 9-week EMA = long",
            &["F|\"The first step is to divide the weekly close of the mutual fund in question by the weekly close of the S&P 500 index, and you'll have a graph of the relative strength ratio of that fund\" (PDF p.1)",
              "E|\"apply the classic moving average convergence/divergence (MACD) indicator to this graph. When the MACD is over its nine-week exponential moving average (EMA), called the trigger line, buy the mutual fund.\" (PDF p.1)",
              "X|\"When the MACD is below the trigger line, move into a money market fund if you are conservative\" (PDF p.1)"][..], FUND),
        d("scraff_rs_macd_stock", "Raff relative strength MACD (stock rule): as the fund rule, buying only with the MACD above zero",
            &["E|\"Equities tend to be more volatile than mutual funds, and therefore, buy signals incorporate the added rule that the MACD line be in positive territory.\" (Figure 3 caption, PDF p.4)",
              "X|\"you have the option of switching to money market funds or Treasury bills after sell signals\" (PDF p.3)"][..], STOCK),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    reviewed(id).then_some(("scraff_rs_macd", "Raff RS-ratio MACD (S&C)", "12/26/9 weeks"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    reviewed(id).then(|| vec![p("fast", "MACD fast EMA, weeks (not printed; reading 12)", 12., 2., 40., 1.), p("slow", "MACD slow EMA, weeks (not printed; reading 26)", 26., 3., 80., 1.)])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let Some(b) = x.series("BENCH.C") else { return Some(vec![]) };
    let g = |k: &str, d: f64| p.get(k).copied().unwrap_or(d).round().max(1.) as usize;
    let we: Vec<usize> = super::swsp::week_ends(x).into_iter().filter(|&i| x.c[i].is_finite() && b[i].is_finite() && b[i] != 0.).collect();
    let rs: Vec<f64> = we.iter().map(|&i| x.c[i] / b[i]).collect();
    let ema = super::msws::ema;
    let (f, s) = (ema(&rs, g("fast", 12.)), ema(&rs, g("slow", 26.)));
    let macd: Vec<f64> = f.iter().zip(&s).map(|(a, b)| a - b).collect();
    let trig = ema(&macd, 9);
    let stock = id == "scraff_rs_macd_stock";
    let mut pos = Book::new();
    for (k, &t) in we.iter().enumerate() {
        if t < from || t > to || !(macd[k].is_finite() && trig[k].is_finite()) { continue; }
        if pos.side() > 0. && macd[k] < trig[k] { pos.exit(t, x.c[t], "MACD below trigger"); }
        else if pos.side() == 0. && macd[k] > trig[k] && (!stock || macd[k] > 0.) { pos.enter(1., t, x.c[t], "close"); }
    }
    Some(pos.done(to, x.c[to]))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn raff_weekly_rs_macd() {
        let r = super::super::sctraub::tests::bars(3000);
        let mut x = Ctx::new(&r, &r, &[]);
        assert!(run("scraff_rs_macd", &x, 0, r.len() - 1, &Params::new()).unwrap().is_empty()); // no benchmark series
        x.extra.insert("BENCH.C".into(), (0..r.len()).map(|i| 100. + 0.01 * i as f64).collect());
        let we = super::super::swsp::week_ends(&x);
        let a = run("scraff_rs_macd", &x, 0, r.len() - 1, &Params::new()).unwrap();
        let s = run("scraff_rs_macd_stock", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(a.len() > 3 && s.len() <= a.len());
        for z in a.iter().chain(&s) { assert!(z.dir > 0. && we.contains(&z.legs[0].i) && (z.open || we.contains(&z.exit_i))); }
    }
}

// Instrument admission: Raff RS-ratio MACD weekly application 2026-11-13 (lane PC, R4 #297).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "scraff" && reviewed(def.id),
    label: "Gilbert Raff applied the relative strength ratio MACD to mutual funds and stocks against the S&P 500 (weekly charts). Weekly bars of ETFs, indices (other than the benchmark itself, #225) and stocks against SPX (US) or NIFTY 50 (India) are a declared application; two unstated details are read as labelled (R4 #297).",
    basis: "Technical Analysis of Stocks & Commodities V.12:11 pp.451-454; R4 #297; independent check authors/pc/w/pc_ref.py",
};
