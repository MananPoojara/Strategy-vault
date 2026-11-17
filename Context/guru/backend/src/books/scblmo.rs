//! Matt Blackman, "Working Money: Do P/E Ratios Really Matter?", Technical Analysis of Stocks & Commodities V.22:7 (Jul 2004) pp.36-40
//! (research share Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V22/C07/143BLAC.pdf).
//! Book id `scblmo`. Lane C7 (second pass, 17 Nov 2026); independent check authors/convert-1117/C7/ref_bl/c7bl_ref.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::msws::sma;
use super::rbtie::target_trades;

pub const REVIEWED: &[&str] = &["scblmo_5_21"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Adapted: 3 unstated details, R4 #297 / #339] The article's DJIA trader: 'A buy was generated when the shorter-period signal line crossed above a longer-period base line. The position was exited when the signal line crossed below the base line.' with 'a five- and 21-month simple moving average (SMA) crossover'. Long only, in or out ('the major benefit ... was not in telling the trader when to go short, but rather in getting him or her out of the market'). [Unstated details] (1) the monthly series is the close of the last trading day of each calendar month (the article says only 'DJIA values'), and the month the data ends in is not used until it is complete; (2) the crossing is read on the month's closing value: buy at that month-end close when the 5-month average is above the 21-month average and was at or below it the month before, sell when it is below and was at or above it (equal averages do not cross); (3) trades are placed at that month-end close (no fill rule is given). [Book result] DJIA 1920-2004 (monthly): 22 buy signals and 21 exits, 5.47% a year buy-and-hold against 9.33% a year while in the market (49.8 years in the market); the vault's DJI starts in 1970, so the 1924 start is not reproduced and no like-for-like match is claimed. The article's second system, the same crossover on Shiller's S&P 500 10-year P/E, needs a P/E series the vault does not hold and is not run. The card runs on any actual index, ETF or stock. UNVERIFIED.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "scblmo_5_21", name: "Blackman monthly crossover: buy when the 5-month average crosses above the 21-month average, sell when it crosses below (DJIA trader)", group: "S&C: Do P/E Ratios Really Matter? (Blackman, V.22:7)", session: 0, dir: "Long", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "scblmo", rules: &[
            "F|\"I tested a simple moving crossover system on the DJIA ... a good combination was a five- and 21-month simple moving average (SMA) crossover.\" (V.22:7 p.37)",
            "E|\"A buy was generated when the shorter-period signal line crossed above a longer-period base line.\" (p.37)",
            "X|\"The position was exited when the signal line crossed below the base line.\" Long only: \"The major benefit of a simple moving average crossover system was therefore not in telling the trader when to go short, but rather in getting him or her out of the market.\" (pp.37-38)"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    reviewed(id).then_some(("scblmo_5_21", "Blackman 5/21-month crossover (S&C)", "5 and 21 months"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    reviewed(id).then(|| vec![p("fast", "Signal line, months (article 5)", 5., 2., 24., 1.), p("slow", "Base line, months (article 21)", 21., 6., 60., 1.)])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from > to || to >= x.c.len() { return None; }
    let g = |k: &str, d: f64| p.get(k).copied().unwrap_or(d).round().max(1.) as usize;
    let (nf, ns) = (g("fast", 5.), g("slow", 21.));
    let n = x.r.len();
    // month-end bars: the last bar of a calendar month that a later bar follows
    let me: Vec<usize> = (0..n).filter(|&i| i + 1 < n && x.r[i].date[..7] != x.r[i + 1].date[..7]).collect();
    let mc: Vec<f64> = me.iter().map(|&i| x.c[i]).collect();
    let (a, b) = (sma(&mc, nf), sma(&mc, ns));
    let mut tgt = vec![0i8; to + 1];
    let (mut pos, mut k) = (0i8, 0usize);
    for i in 0..=to {
        while k < me.len() && me[k] < i { k += 1; }
        if k < me.len() && me[k] == i && k > 0 && a[k].is_finite() && b[k].is_finite() && a[k - 1].is_finite() && b[k - 1].is_finite() {
            if a[k] > b[k] && a[k - 1] <= b[k - 1] { pos = 1; }
            else if a[k] < b[k] && a[k - 1] >= b[k - 1] { pos = 0; }
        }
        tgt[i] = pos;
    }
    Some(target_trades(x, &tgt, from, to, false, "monthly SMA crossover"))
}

pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "scblmo" && reviewed(def.id),
    label: "Matt Blackman tested the 5/21-month crossover on the DJIA (1920-2004); runs on actual indices, ETFs and stocks, US and India, are a declared application, not the author's test, with the unstated details read as labelled on the card.",
    basis: "Technical Analysis of Stocks & Commodities V.22:7 pp.36-40 (Working Money: Do P/E Ratios Really Matter?); independent check authors/convert-1117/C7/ref_bl/c7bl_ref.py",
};

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn monthly_cross_has_catalogue_and_params() {
        assert!(family_of("scblmo_5_21").is_some() && params("scblmo_5_21").is_some());
        let r = super::super::sctraub::tests::bars(900);
        let x = Ctx::new(&r, &r, &[]);
        assert!(run("scblmo_5_21", &x, 0, r.len() - 1, &Params::new()).is_some());
    }
}
