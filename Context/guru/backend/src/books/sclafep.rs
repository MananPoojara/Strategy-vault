//! Patrick E. Lafferty, "The End Point Moving Average", Technical Analysis of Stocks & Commodities V.13:10 (October 1995)
//! pp.413-417 (research share Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V13/C10/
//! THEENDP.pdf). Book id `sclafep`. Lane K16 (titles loop, 2 Nov 2026), to-code row 291, decisions 64, 76; independent check
//! authors/titles-1030/K16w/k16_ref.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::tselhs::Book;

pub const REVIEWED: &[&str] = &["sclafep_200"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] EPMA(n) = the end point (today's value) of the least-squares line fitted to the last n closes (MetaStock's time series forecast without projection), n = 200 as in the article's DJIA example. Buy when the EPMA moves up (above yesterday's) and the index closes above it; sell when the EPMA moves down and the index closes below it. Long or cash (the article quotes buy and sell points only, decision 64); fills at the signal close. The article's DJIA example (28 months, 1985-1987): buy 1355 October 1985, sell 1793 July 1986, buy 1906 November 1986, sell 2493 September 1987. DJI is held and runs as the article's own index (decision 76); other indices, ETFs and stocks, US and India, are a declared application.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "sclafep_200", name: "Lafferty 200-day end point moving average: long while it rises with price above it, out when it falls with price below", group: "S&C: The End Point Moving Average (Lafferty, V.13:10)", session: 0, dir: "Long", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "sclafep", rules: &[
            "F|EPMA = the end point of the least-squares line through the last 200 closes (V.13:10)",
            "E|\"1. Buy when the 200-day EPMA moves up and the value of the DJIA is higher than the value of the EPMA.\" (PDF p.6)",
            "X|\"2. Sell when the 200-day EPMA moves down and the value of the DJIA is lower than the value of the EPMA.\" (PDF p.6)"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { reviewed(id).then_some(("sclafep_200", "Lafferty end point moving average (S&C)", "200 days")) }

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    reviewed(id).then(|| vec![p("n", "EPMA days (article 200)", 200., 10., 300., 5.)])
}

/// End point of the least-squares line through the last n values.
pub fn epma(c: &[f64], n: usize) -> Vec<f64> {
    let xm = (n as f64 - 1.0) / 2.0;
    let sxx: f64 = (0..n).map(|k| (k as f64 - xm).powi(2)).sum();
    (0..c.len()).map(|i| if i + 1 < n { f64::NAN } else {
        let w = &c[i + 1 - n..=i];
        let ym = w.iter().sum::<f64>() / n as f64;
        let b = w.iter().enumerate().map(|(k, y)| (k as f64 - xm) * (y - ym)).sum::<f64>() / sxx;
        ym + b * (n as f64 - 1.0 - xm)
    }).collect()
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let e = epma(&x.c, p.get("n").copied().unwrap_or(200.).round().max(2.) as usize);
    let mut pos = Book::new();
    for t in from.max(1)..=to {
        if !e[t - 1].is_finite() { continue; }
        if pos.side() == 0.0 && e[t] > e[t - 1] && x.c[t] > e[t] { pos.enter(1.0, t, x.c[t], "close"); }
        else if pos.side() > 0.0 && e[t] < e[t - 1] && x.c[t] < e[t] { pos.exit(t, x.c[t], "EPMA down, close below"); }
    }
    Some(pos.done(to, x.c[to]))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn epma_line() {
        let v: Vec<f64> = (0..10).map(|k| 3.0 + 2.0 * k as f64).collect();
        assert!((epma(&v, 5)[9] - 21.0).abs() < 1e-9);
        let r = super::super::sctraub::tests::bars(1500);
        let x = Ctx::new(&r, &r, &[]);
        assert!(run("sclafep_200", &x, 0, r.len() - 1, &Params::new()).is_some());
    }
}

// Instrument admission: S&C Lafferty EPMA 2026-11-02 (titles lane K16).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "sclafep" && reviewed(def.id),
    label: "Patrick Lafferty showed the 200-day EPMA on the DJIA (1985-1987). Runs on indices, ETFs and stocks, US and India, are a declared application.",
    basis: "Technical Analysis of Stocks & Commodities V.13:10 pp.413-417 (The End Point Moving Average); independent check authors/titles-1030/K16w/k16_ref.py",
};
