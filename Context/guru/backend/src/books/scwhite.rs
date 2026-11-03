//! Adam White, "A Weekly S&P Trading System", Technical Analysis of Stocks & Commodities V.13:6 (June 1995) pp.233-237, and
//! its sidebar "MetaStock syntax" (research share Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008)
//! (traders.com)/VOLUMES/V13/C06/AWEEKLY.PDF and SIDEMET.PDF). Book id `scwhite`. Lane K19 (titles loop, 2 Nov 2026), to-code
//! row 290; independent check authors/titles-1030/K19w/k19_ref.py.
use crate::connors::{sma, Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::lc3::{hi, lo};
use super::sctraub::{week_groups, weekday};

pub const REVIEWED: &[&str] = &["scwhite_weekly"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] Weekly bars = calendar weeks of the daily bars (open of the first day, highest high, lowest low, close of the last day); the data's last week counts only when it ends on a Friday. Entry and exit rules are the sidebar's MetaStock formulas with X = 5, N = 13 and the 26-week simple average. The retracement level: the text twice gives 'one half of 1%' (0.005, the default), while the sidebar optimises it from 0.03 to 0.07 (and Morris's GM9 copy, msws_gm9_weekly_sp_system, uses 0.05); 0.05 is option ret. TAI threshold 1.2 (the text; optimised 1.0-1.4). Trades 'on the opening of the bar that immediately follows the bar that generates the trade signal' = the next week's first open. Long or cash. Article: S&P 500, 500 weeks from April 1982 to November 1991, no interest or commissions; the result table is an image in the PDF. Runs on indices, ETFs and stocks, US and India, are a declared application.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "scwhite_weekly", name: "White weekly S&P system: buy a 5-week high while the 5-week low is above the 13-week low; exit on a retracement from the 13-week highest low while the TAI is under 1.2", group: "S&C: A Weekly S&P Trading System (White, V.13:6)", session: 0, dir: "Long", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "scwhite", rules: &[
            "E|\"Enter Long: when(llv(L,5),>,llv(L,13)) AND when(H,=,hhv(H,5))\" (sidebar MetaStock syntax, p.236)",
            "X|\"Close Long: when(fml(#1),>,opt1) AND when(fml(#2),<,opt2)\"; \"Formula #1: (hhv(L,13)-L)/L\", \"Formula #2: ((hhv(mov(C,26,S),5)-llv(mov(C,26,S),5))/C)*100\" (sidebar); \"Exit if the last bar's low is more than one half of 1% of the price below the highest low of the last 13 bars, provided a five-bar/26-bar moving average TAI is below 1.2\" (p.236)",
            "F|\"trades are taken on the opening of the bar that immediately follows the bar that generates the trade signal\"; weekly bars of the S&P 500 (p.233-234)"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    reviewed(id).then_some(("scwhite_weekly", "White weekly S&P system (S&C)", "5 / 13 weeks, 0.5%, TAI 1.2"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    reviewed(id).then(|| vec![p("ret", "Retracement exit level (text 0.005; sidebar range 0.03-0.07)", 0.005, 0.001, 0.1, 0.001), p("tai", "TAI threshold (1.2)", 1.2, 0.5, 3., 0.1)])
}

/// Calendar weeks (first bar, last bar); the data's last week only when its last bar is a Friday.
pub fn complete_weeks(x: &Ctx) -> Vec<(usize, usize)> {
    let n = x.r.len();
    week_groups(x).into_iter().filter(|&(_, e)| e + 1 < n || weekday(&x.r[e].date) == 4).collect()
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let (ret, tai_max) = (p.get("ret").copied().unwrap_or(0.005), p.get("tai").copied().unwrap_or(1.2));
    let w = complete_weeks(x);
    let (wh, wl): (Vec<f64>, Vec<f64>) = w.iter().map(|&(s, e)| (hi(&x.h, s, e), lo(&x.l, s, e))).unzip();
    let wc: Vec<f64> = w.iter().map(|&(_, e)| x.c[e]).collect();
    let m = sma(&wc, 26);
    let mut sig = vec![None::<(bool, bool)>; x.r.len()];
    for k in 12..w.len() {
        let enter = lo(&wl, k - 4, k) > lo(&wl, k - 12, k) && wh[k] == hi(&wh, k - 4, k);
        let tai = if k >= 4 && m[k - 4].is_finite() { (hi(&m, k - 4, k) - lo(&m, k - 4, k)) / wc[k] * 100.0 } else { f64::NAN };
        let exit = (hi(&wl, k - 12, k) - wl[k]) / wl[k] > ret && tai < tai_max;
        sig[w[k].1] = Some((enter, exit));
    }
    Some(super::sctraub::next_open(x, from, to, "retracement exit", |t, side| match sig[t] {
        Some((e, _)) if side == 0.0 && e => 1.0,
        Some((_, q)) if side > 0.0 && q => 0.0,
        _ => side,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn weekly_entries_fill_on_mondays() {
        let r = super::super::sctraub::tests::bars(3000);
        let x = Ctx::new(&r, &r, &[]);
        let t = run("scwhite_weekly", &x, 0, r.len() - 1, &[("ret".to_string(), 0.05)].into_iter().collect()).unwrap();
        assert!(t.len() > 2);
        for z in &t { assert_eq!(weekday(&x.r[z.legs[0].i].date), 0); }
        assert!(complete_weeks(&x).windows(2).all(|g| g[0].1 + 1 == g[1].0));
    }
}

// Instrument admission: S&C White weekly S&P system 2026-11-02 (titles lane K19).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "scwhite" && reviewed(def.id),
    label: "White tested the weekly system on the S&P 500 index (April 1982 - November 1991). Runs on other indices, ETFs and stocks, US and India, are a declared application.",
    basis: "Technical Analysis of Stocks & Commodities V.13:6 pp.233-237 (A Weekly S&P Trading System) and its MetaStock syntax sidebar; independent check authors/titles-1030/K19w/k19_ref.py",
};
