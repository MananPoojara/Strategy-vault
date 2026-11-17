//! Richard W. Arms Jr., "Cross Your Arms", Technical Analysis of Stocks & Commodities V.9:5 (May 1991) pp.177-179
//! (research share Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V09/C05/CROSS.pdf).
//! Book id `scarmx`. Lane C5 (second pass, 17 Nov 2026); independent check authors/convert-1117/C5/ref/c5_ref.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::msws::sma;
use super::rbtie::target_trades;

pub const REVIEWED: &[&str] = &["scarmx_21_55", "scarmx_21_55_nse"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }
pub const SERIES_NSE: &[&str] = &["NSE_ADV_DEC.Advances", "NSE_ADV_DEC.Declines", "NSE_ADV_DEC.AdvVolume", "NSE_ADV_DEC.DecVolume"];
pub const SERIES: &[&str] = &["NYSE_ADV_DEC.Advances", "NYSE_ADV_DEC.Declines", "NYSE_ADV_DEC.AdvVolume", "NYSE_ADV_DEC.DecVolume"];

const NOTE: &str = "[Adapted: 1 unstated detail, R4 #297] Arms Index = (advancing issues / declining issues) / (advancing volume / declining volume), the article's own definition, on NYSE issues and volume (vault NYSE_ADV_DEC, 1965-03-01..2020-02-10). The rule: 'if the 21-day line is below the 55-day line you should be long and if it is above the 55-day line you should be short, or at least out of the market if you cannot tolerate the short side'; both lines are simple arithmetic averages of the daily Arms Index. The card is always in the market (long below, short above) after the first day both averages exist; trading only the long side (flat above) is the article's printed alternative and is not run. [Unstated detail] The article places the trades at chart points on the S&P 500 and gives no fill rule; signals act at the close of the day the averages cross (equal averages keep the position). The article gives no results table (Figures 1-2 mark crossovers on the S&P 500 in 1990), so there is no book figure to match. Runs on the US indices and ETFs only (NYSE breadth).";

const NOTE_NSE: &str = "[Declared adaptation, R4 #340 / #347, lane C7 17 Nov 2026; UNVERIFIED] The article crosses a 21-day and a 55-day simple average of the NYSE Arms Index. This twin computes the same Arms Index, (advancing issues / declining issues) / (advancing volume / declining volume), from the real NSE advances, declines and traded volume the vault holds (extra_data/india/NSE_ADV_DEC.csv, from 1999) on the NIFTY indices, with 21 and 55 unchanged; it is not the author's test. The rule has no issue-count threshold (it compares two averages of one ratio), so nothing is rescaled. Measured over 1999-11-09 to 2020-02-10: the 21-day average is below the 55-day (long) on 50.9% of NSE days against 50.7% of NYSE days; crossovers 365 on NSE (18.3 a year) against 432 on NYSE (21.3 a year); the mean Arms reading is 0.94 on NSE against 1.22 on NYSE. Same readings as the NYSE card: always in the market, long below and short above, equal averages keep the position; [Unstated detail] signals act at the close of the day the averages cross. No book figure applies.";

fn mk(id: &'static str, name: &'static str, series: &'static [&'static str], notes: &'static str) -> StrategyDef {
    StrategyDef { id, name, group: "S&C: Cross Your Arms (Arms, V.9:5)", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: series, book: "scarmx", rules: &[
            "F|\"Advances / Declines = Arms Index, Advancing volume / Declining volume\"; \"the 21-day moving average of the Arms Index superimposed upon the 55-day moving average of the Arms Index. It is a simple arithmetic moving average in both cases.\" (V.9:5 pp.177-178)",
            "E|\"The rule is simple: if the 21-day line is below the 55-day line you should be long and if it is above the 55-day line you should be short, or at least out of the market if you cannot tolerate the short side of the market.\" (p.178)",
            "X|\"Consider selling the stock market when the 21-day moving average rises above the 55-day moving average (A, C and E). Buy the stock market when the 21-day moving average drops below the 55-day moving average (B and D).\" (Figure 1 caption): stop and reverse"],
        notes, claim: None }
}

pub fn catalogue() -> Vec<StrategyDef> {
    vec![mk("scarmx_21_55", "Arms Index 21/55 crossover: long when the 21-day average is below the 55-day average, short when above", SERIES, NOTE),
         mk("scarmx_21_55_nse", "Arms Index 21/55 crossover on NSE breadth: long when the 21-day average is below the 55-day average, short when above", SERIES_NSE, NOTE_NSE)]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    reviewed(id).then_some(("scarmx_21_55", "Arms Index 21/55 crossover (S&C)", "21 and 55 days"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    reviewed(id).then(|| vec![p("fast", "Short average, days (article 21)", 21., 2., 60., 1.), p("slow", "Long average, days (article 55)", 55., 5., 150., 1.)])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from > to || to >= x.c.len() { return None; }
    let g = |k: &str, d: f64| p.get(k).copied().unwrap_or(d).round().max(1.) as usize;
    let s: Vec<&[f64]> = match (if id.ends_with("_nse") { SERIES_NSE } else { SERIES }).iter().map(|n| x.series(n)).collect::<Option<Vec<_>>>() { Some(v) => v, None => return Some(vec![]) };
    let arms: Vec<f64> = (0..=to).map(|i| { let v = (s[0][i] / s[1][i]) / (s[2][i] / s[3][i]); if v.is_finite() { v } else { f64::NAN } }).collect();
    let (a, b) = (sma(&arms, g("fast", 21.)), sma(&arms, g("slow", 55.)));
    let mut tgt = vec![0i8; to + 1];
    let mut pos = 0i8;
    for i in 0..=to {
        if a[i].is_finite() && b[i].is_finite() { if a[i] < b[i] { pos = 1; } else if a[i] > b[i] { pos = -1; } }
        tgt[i] = pos;
    }
    Some(target_trades(x, &tgt, from, to, false, "Arms 21/55 cross"))
}

pub const ADMISSION_NSE: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "scarmx" && def.id == "scarmx_21_55_nse",
    label: "NSE-breadth twin (R4 #340 / #347): Richard Arms' 21/55 crossover of the Arms Index, computed from the real NSE advancing and declining issues and volume, on the NIFTY indices only. Measured 1999-11-09 to 2020-02-10: long (21 below 55) on 50.9% of NSE days against 50.7% of NYSE days; 18.3 crossovers a year on NSE against 21.3 on NYSE. A declared adaptation, not the author's test, UNVERIFIED.",
    basis: "Technical Analysis of Stocks & Commodities V.9:5 pp.177-179 (Cross Your Arms), NSE-breadth adaptation; independent check authors/convert-1117/C7/ref_nse/c7_ref.py",
};

pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "scarmx" && def.id == "scarmx_21_55",
    label: "Richard Arms showed the 21/55 crossover on the S&P 500 in 1990 (chart points, no results table); it reads NYSE advancing and declining issues and volume and runs on the US indices and ETFs SPX, SPY, DJI, DIA, NDX and QQQ only (NYSE breadth, R4 #306), with the unstated detail read as labelled on the card.",
    basis: "Technical Analysis of Stocks & Commodities V.9:5 pp.177-179 (Cross Your Arms); independent check authors/convert-1117/C5/ref/c5_ref.py",
};

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn arms_cross_has_catalogue_and_params() {
        assert!(family_of("scarmx_21_55").is_some() && params("scarmx_21_55").is_some());
        let r = super::super::sctraub::tests::bars(300);
        let x = Ctx::new(&r, &r, &[]);
        assert_eq!(run("scarmx_21_55", &x, 0, r.len() - 1, &Params::new()).unwrap().len(), 0); // no NYSE series: no trades
        assert_eq!(run("scarmx_21_55_nse", &x, 0, r.len() - 1, &Params::new()).unwrap().len(), 0);
    }
}
