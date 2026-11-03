//! Mark Shipman, Big Money, Little Effort (Kogan Page 2008), ch.14 'The rules' (printed p.75-76), ch.15 operational guide
//! (p.85), ch.18 S&P 500 trade ledger (p.124-147); research share USB HDD 2/tRADING KNO-2/Mark Shipman - Big Money Little
//! Efford/Big Money Little Effort.PDF. Book id `msbml`. Lane K7 (titles loop, 1 Nov 2026); rule text
//! authors/batch-1101/B2w/shipman_bmle.txt; independent check authors/titles-1030/K7w/k7_ref.py (header).
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::mpts::{period_ends, sma};
use super::rbtie::target_trades;

/// Admitted ids (authors/titles-1030/K7w/k7_ref.py (header)).
pub const REVIEWED: &[&str] = &["msbml_30_50w"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] Weekly bar = the last trading day of each Monday-based week (the last data bar counts only if it is a Friday); the moving averages are simple averages of those weekly closes (\"the average weekly closing price for the last 30 weeks\", p.75). Buy signal active while the 30-week average is above the 50-week, sell signal while the 50-week is above the 30-week; equal values change nothing. Long or cash (the book: an interest-bearing savings account; interest is not modelled). Fill at the signal week's close, as the ch.18 ledger prices every trade (e.g. '30 November 1984 Opened position @ 166.90'); option: the next week's first open ('what action is required when the market reopens at the beginning of the next week', p.85). Same mechanics as Russell's 13/50-week rule in Pring (mptae_russell), with Shipman's own 30/50 weeks and printed ledger. The book's instrument is a major stock index (S&P 500 1951-2007 in ch.18; also Dow, Nasdaq-100, FTSE, DAX, CAC, Nikkei, Hang Seng); NIFTY, ETFs and stocks are a declared application.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "msbml_30_50w", name: "Long-Term Investment System: invested while the 30-week average is above the 50-week, cash when below",
        group: "Shipman Long-Term Investment System", session: 0, dir: "Long", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: "msbml", rules: &[
            "E|\"A 'buy (invest in the stock market)' signal is active whenever the most recent 30-week moving average figure is greater (higher) than the most recent 50-week moving average figure.\" (p.75)",
            "X|\"A 'sell (savings account investment)' signal is active whenever the most recent 50-week moving average figure is greater (higher) than the most recent 30-week moving average figure.\" (p.75)",
            "F|\"the average weekly closing price for the last 30 weeks ... the last 50 weeks\" (p.75); analysis after the week's close (p.85)"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    (id == "msbml_30_50w").then_some(("msbml_30_50w", "Shipman Long-Term Investment System", "30/50-week averages"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    (id == "msbml_30_50w").then(|| vec![
        ParamDef { key: "fast", label: "Fast average (weeks, book 30)", def: 30., min: 2., max: 60., step: 1. },
        ParamDef { key: "slow", label: "Slow average (weeks, book 50)", def: 50., min: 5., max: 100., step: 1. },
        ParamDef { key: "fill", label: "0 = signal week's close (the ledger), 1 = next week's first open", def: 0., min: 0., max: 1., step: 1. },
    ])
}

/// f applied to the week-end values of v, written back on the week-end bars (NaN elsewhere).
pub fn on_weeks(ends: &[bool], v: &[f64], f: impl Fn(&[f64]) -> Vec<f64>) -> Vec<f64> {
    let idx: Vec<usize> = (0..ends.len()).filter(|&i| ends[i]).collect();
    let r = f(&idx.iter().map(|&i| v[i]).collect::<Vec<_>>());
    let mut out = vec![f64::NAN; ends.len()];
    for (k, &i) in idx.iter().enumerate() { out[i] = r[k]; }
    out
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let g = |k: &str, d: f64| p.get(k).copied().unwrap_or(d).round().max(1.) as usize;
    let wk = period_ends(x, false);
    let (f, s) = (on_weeks(&wk, &x.c, |v| sma(v, g("fast", 30.))), on_weeks(&wk, &x.c, |v| sma(v, g("slow", 50.))));
    let mut tgt = vec![0i8; x.c.len()];
    for i in 0..x.c.len() {
        let prev = if i > 0 { tgt[i - 1] } else { 0 };
        tgt[i] = if f[i] > s[i] { 1 } else if s[i] > f[i] { 0 } else { prev };
    }
    Some(target_trades(x, &tgt, from, to, p.get("fill").copied().unwrap_or(0.) >= 0.5, "30/50-week averages"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn invested_only_while_the_fast_weekly_average_is_above_the_slow() {
        let mut rows = vec![];
        let mut d = 0;
        for k in 0..900 { // weekdays only from Monday 2001-01-01
            let date = chrono_like(d); d += if k % 5 == 4 { 3 } else { 1 };
            let v = 100. + 20. * (k as f64 / 150.).sin();
            rows.push(Row { ticker: "T".into(), date, open: v, high: v + 1., low: v - 1., close: v, volume: None });
        }
        let x = Ctx::new(&rows, &rows, &[]);
        let wk = period_ends(&x, false);
        assert!(wk[4] && !wk[3] && wk[9]);
        let t = run("msbml_30_50w", &x, 0, rows.len() - 1, &Params::new()).unwrap();
        assert!(t.len() >= 2 && t.iter().all(|z| z.dir > 0. && wk[z.legs[0].i]));
        assert!(run("msbml_x", &x, 0, 5, &Params::new()).is_none());
    }
    fn chrono_like(d: i64) -> String { // days after 2001-01-01 as YYYY-MM-DD
        let (mut y, mut d) = (2001, d);
        loop { let n = if y % 4 == 0 { 366 } else { 365 }; if d < n { break; } d -= n; y += 1; }
        let ml = [31, if y % 4 == 0 { 29 } else { 28 }, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
        let mut m = 0; while d >= ml[m] { d -= ml[m]; m += 1; }
        format!("{y}-{:02}-{:02}", m + 1, d + 1)
    }
}

// Instrument admission: Mark Shipman (Big Money, Little Effort; The Next Big Investment Boom) 2026-11-01 (titles lane K7).
// Admission only for the module REVIEWED lists (authors/titles-1030/K7w/k7_ref.py (header)).
// Runs on actual indices, ETFs and stocks, US and India (index_etf_or_stock, nifty50 basket, declared application).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| (def.book == "msbml" && reviewed(def.id)) || (def.book == "msnib" && super::msnib::reviewed(def.id)),
    label: "Mark Shipman's weekly rules for major stock indices (Big Money, Little Effort) and commodity funds and the S&P 500 (The Next Big Investment Boom); ETFs, NIFTY and US/India stocks are a declared application.",
    basis: "Mark Shipman, Big Money, Little Effort (2008) ch.14 p.75-76, ch.15 p.85, ch.18 ledger p.124-147; The Next Big Investment Boom ch.5 p.69-86",
};
