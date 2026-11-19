//! J.R. Davis, "Comparing Three Indicators", Technical Analysis of Stocks & Commodities V.11:11 (1993) pp.472-476, with the
//! editor's sidebars "Random walk index spreadsheet" (Thom Hartle) and "Building a random walk index" (research share Trading
//! Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V11/C11/COMPARI.pdf).
//! Book id `scjrd`: the always-in random walk index system. Lane PG (13 Nov 2026, handover of lane PA2's 'not done' row);
//! independent check authors/pg/w/pg_ref.py. This module also holds lane PG's instrument admission entry (PG_BOOKS).
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::sctraub::next_open;

pub const REVIEWED: &[&str] = &["scjrd_rwi"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Adapted: 2 unstated details, R4 #297; unverified] Sidebar 'Building a random walk index': the lowest low of the prior 65 days (today excluded), which occurred k days ago; RWI up = (today's high - that low) / (sqrt(k) x the average range of the last k days); RWI down = (the highest high of the prior 65 days, j days ago - today's low) / (sqrt(j) x the average range of the last j days). The range is the editor's printed true range, =MAX(high - low, yesterday's close - low) (Hartle sidebar, cell F11; the same column serves the highs: 'substitute today's high minus the low n days ago in the numerator'). Long when RWI up >= 1 and RWI down < 1, short when RWI down >= 1 and RWI up < 1, otherwise hold: always in the market once the first signal comes ('Like the XMA and the swing chart, the RWI proved continuously to be in the market'). 'If an entry signal was generated one day, then the next day's opening price was used for the entry price'; the same for exits. Unstated: (1) whether 'the last k days' of the average range include today - vault reading: yes (the usual average-true-range convention; option today = 0 ends it yesterday, as in Hartle's spreadsheet layout); (2) equal lows (highs) in the 65 days - vault reading: the most recent one sets k. Davis tested seven futures contracts (T-bonds, Deutschemarks, S&P 500, corn, wheat, gold, hogs; the last 150 days of each contract, 1975-1990); the vault runs the rule on daily bars of indices, ETFs and stocks, US and India, as a declared application, so the article's per-contract tables (Figures 1-2) are not a comparable result: UNVERIFIED.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "scjrd_rwi", name: "Davis random walk index system: always in, long when RWI up >= 1 and RWI down < 1, short on the reverse (65-day maximum lookback), next open", group: "S&C: comparing three indicators (Davis, V.11:11)", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "scjrd", rules: &[
            "F|\"The maximum RWI lookback period was set at 65 days. For the RWI, I used the algorithm that if both the RWI down and the RWI up were greater than 1, no signal was given. A signal was generated only when one index was greater than 1, while the other was less than 1. Like the XMA and the swing chart, the RWI proved continuously to be in the market.\" (V.11:11, PDF p.2)",
            "F|\"To create a random walk index, find the lowest low for the prior n days (65 days in the example used in 'Comparing three indicators'). That low occurred k days ago. Then find the average range for the last k days. If the difference between today's high and the lowest low is greater than or equal to the square root of k multiplied by the average range (this quantity is the RWI up), then the market is considered to be trending up. The RWI down is found in a similar fashion using the highest high of the last n days and comparing it with today's low.\" (sidebar 'Building a random walk index')",
            "F|\"The average range uses the true range, which is the greatest of two comparisons: today's high minus today's low or yesterday's close minus today's low. For cell F11 (true range), the formula is: =MAX(B11-C11,D10-C11)\" (Hartle sidebar)",
            "E|\"If the RWI up was >= 1 and the RWI down < 1, then the contract was bought. The long position was held until the RWI down >= 1, while the RWI up < 1, at which time the system liquidated the long position and went short.\" (sidebar)",
            "X|\"In all cases, signals generated one day were acted upon the next. If an entry signal was generated one day, then the next day's opening price was used for the entry price. If an exit signal was generated one day, then the next day's open was used for the exit price.\" (PDF p.2)"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { reviewed(id).then_some(("scjrd_rwi", "Davis random walk index system (S&C)", "65")) }
pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    reviewed(id).then(|| vec![
        ParamDef { key: "n", label: "Maximum RWI lookback days (article 65)", def: 65., min: 2., max: 250., step: 1. },
        ParamDef { key: "today", label: "1 = the average range of the last k days includes today; 0 = ends yesterday", def: 1., min: 0., max: 1., step: 1. }])
}

/// (RWI up, RWI down) per bar.
pub fn rwi(x: &Ctx, n: usize, today: bool) -> (Vec<f64>, Vec<f64>) {
    let len = x.c.len();
    let tr: Vec<f64> = (0..len).map(|i| if i == 0 { f64::NAN } else { (x.h[i] - x.l[i]).max(x.c[i - 1] - x.l[i]) }).collect();
    let avg = |t: usize, k: usize| -> f64 { let e = if today { t } else { t - 1 }; if e + 1 < k { return f64::NAN; } tr[e + 1 - k..=e].iter().sum::<f64>() / k as f64 };
    let (mut up, mut dn) = (vec![f64::NAN; len], vec![f64::NAN; len]);
    for t in n..len {
        let (mut il, mut ih) = (t - n, t - n);
        for i in t - n..t { if x.l[i] <= x.l[il] { il = i; } if x.h[i] >= x.h[ih] { ih = i; } }
        let (k, j) = (t - il, t - ih);
        up[t] = (x.h[t] - x.l[il]) / ((k as f64).sqrt() * avg(t, k));
        dn[t] = (x.h[ih] - x.l[t]) / ((j as f64).sqrt() * avg(t, j));
    }
    (up, dn)
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from > to || to >= x.c.len() { return None; }
    let n = p.get("n").copied().unwrap_or(65.).round().max(2.) as usize;
    let (up, dn) = rwi(x, n, p.get("today").copied().unwrap_or(1.) != 0.);
    Some(next_open(x, from, to, "opposite RWI signal", |t, s| if up[t] >= 1. && dn[t] < 1. { 1. } else if dn[t] >= 1. && up[t] < 1. { -1. } else { s }))
}

pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| PG_BOOKS.contains(&def.book) && pg_reviewed(def.book, def.id),
    label: "Stocks & Commodities articles read in lane PG (13 Nov 2026), tested by their authors on futures contracts or weekly index closes; runs on daily bars of indices, ETFs and stocks, US and India, are a declared application, not the authors' tests (any unstated detail is a labelled reading on the card, R4 #297).",
    basis: "Lane PG (handover rows of lanes PA1/PA2 and pool B of authors/a1/A1.tsv, R4 #297): each card cites its article pages; independent check authors/pg/w/pg_ref.py",
};
/// Books admitted by the entry above.
pub const PG_BOOKS: &[&str] = &["scjrd", "scpmdm"];
fn pg_reviewed(book: &str, id: &str) -> bool {
    match book { "scjrd" => reviewed(id), "scpmdm" => super::scpmdm::reviewed(id), _ => false }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rwi_definition_and_always_in() {
        let r = super::super::sctraub::tests::bars(1500);
        let x = Ctx::new(&r, &r, &[]);
        let (up, dn) = rwi(&x, 65, true);
        // hand check of one bar against the sidebar's definition
        let t = 700;
        let il = (t - 65..t).rev().min_by(|&a, &b| x.l[a].partial_cmp(&x.l[b]).unwrap()).unwrap();
        let k = t - il;
        let a: f64 = (t + 1 - k..=t).map(|i| (x.h[i] - x.l[i]).max(x.c[i - 1] - x.l[i])).sum::<f64>() / k as f64;
        assert!((up[t] - (x.h[t] - x.l[il]) / ((k as f64).sqrt() * a)).abs() < 1e-9 && dn[t].is_finite());
        let tr = run("scjrd_rwi", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(tr.len() > 5 && tr.windows(2).all(|w| w[1].legs[0].i == w[0].exit_i && w[1].dir == -w[0].dir));
        assert!(tr.iter().all(|z| z.entry_kind == "open"));
    }
}
