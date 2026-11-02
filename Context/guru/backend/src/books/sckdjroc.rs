//! Jay Kaeppel, "A DJIA Long-Term Momentum Indicator", Technical Analysis of Stocks & Commodities V.13:5 (May 1995) pp.200-203
//! (research share Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V13/C05/ADJIA.PDF).
//! Book id `sckdjroc`. Lane K13 (titles loop, 2 Nov 2026), to-code row 202, lead decision 76; independent check
//! authors/titles-1030/K13w/k13_ref.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::sctraub::period_ends;
use super::tselhs::Book;

pub const REVIEWED: &[&str] = &["sckdjroc_djroc"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] Month-end closes (the last trading day of each month). MROC = (close / close 11 months earlier - 1) x 100 (the article's December 1994 / January 1994 example); DJROC = (10 x this month's MROC + 9 x last month's ... + 1 x nine months ago) / 55. Buy at the month-end close when last month's DJROC was -1.0 or below and this month's is higher ('declines to -1.0 or below and then rises for a month'). Sell at the month-end close of the first month DJROC declines, or 12 months after the buy; buy signals while long are ignored. Long or cash (no interest). The article's book index is the DJIA (DJI is held, decision 76): 1915-1994, in the market 270 of 918 months, 2,772 of 3,752 DJIA points, 77% of trades profitable, average +16.4%. Runs on other indices, ETFs and stocks, US and India, are a declared application. The vault's msws_gm9_djroc_system (Morris) prints no exit; this article does.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "sckdjroc_djroc", name: "Kaeppel DJROC: buy when the weighted 11-month rate of change turns up from -1 or below; out on its first decline or after 12 months", group: "S&C: A DJIA Long-Term Momentum Indicator (Kaeppel, V.13:5)", session: 0, dir: "Long", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "sckdjroc", rules: &[
            "F|\"1. Each month, divide the DJIA's closing price for the month by the closing price 11 months ago. Subtract 1 and multiply by 100 ... (MROC). 2. ... a 10-period weighted moving average. First, multiply this month's MROC by 10, last month's MROC by 9 ... Add up each of these monthly calculations and then divide by 55.\" (V.13:5 p.201)",
            "E|\"A buy signal occurs when the DJROC declines to -1.0 or below and then rises for a month.\" (p.201)",
            "X|\"hold for 12 months after a buy signal or until the DJROC records a monthly decline, whichever comes first.\" (p.202)"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let id = REVIEWED.iter().find(|r| **r == id)?;
    Some((*id, "Kaeppel DJROC (S&C)", "Weighted 11-month ROC"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> { reviewed(id).then(Vec::new) }

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, _p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let me = period_ends(x, true);
    let mc: Vec<f64> = me.iter().map(|&i| x.c[i]).collect();
    let r: Vec<f64> = (0..mc.len()).map(|k| if k >= 11 { 100.0 * (mc[k] / mc[k - 11] - 1.0) } else { f64::NAN }).collect();
    let dj: Vec<f64> = (0..mc.len()).map(|k| if k >= 20 { (0..10).map(|j| (10 - j) as f64 * r[k - j]).sum::<f64>() / 55.0 } else { f64::NAN }).collect();
    let mut pos = Book::new();
    let mut ke = 0usize;
    for (k, &i) in me.iter().enumerate() {
        if i < from || i > to || k < 21 { continue; }
        if pos.side() > 0.0 && (dj[k] < dj[k - 1] || k - ke >= 12) { pos.exit(i, x.c[i], if dj[k] < dj[k - 1] { "DJROC declined" } else { "12 months" }); }
        if pos.side() == 0.0 && dj[k - 1] <= -1.0 && dj[k] > dj[k - 1] { pos.enter(1.0, i, x.c[i], "close"); ke = k; }
    }
    Some(pos.done(to, x.c[to]))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn djroc_monthly() {
        let r = super::super::sctraub::tests::bars(6000);
        let x = Ctx::new(&r, &r, &[]);
        let t = run("sckdjroc_djroc", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(!t.is_empty() && t.iter().all(|z| z.dir > 0.0));
        assert!(family_of("sckdjroc_djroc").is_some());
    }
}

// Instrument admission: S&C Kaeppel DJROC 2026-11-02 (titles lane K13).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "sckdjroc" && reviewed(def.id),
    label: "Jay Kaeppel tested the DJROC on DJIA monthly closes (1915-1994); DJI is the article's own index. Runs on other indices, ETFs and stocks, US and India, are a declared application.",
    basis: "Technical Analysis of Stocks & Commodities V.13:5 pp.200-203 (A DJIA Long-Term Momentum Indicator); independent check authors/titles-1030/K13w/k13_ref.py",
};
