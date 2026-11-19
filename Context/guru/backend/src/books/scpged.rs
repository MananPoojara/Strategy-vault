//! George Panagakis, "Trading The Eurodollar", Technical Analysis of Stocks & Commodities V.13:3 (1995) pp.102-106 (research
//! share Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V13/C03/EURODLR.pdf).
//! Book id `scpged`: the weekly two-week breakout indicator on Eurodollar futures. Lane PG (13 Nov 2026, pool B row 392 of
//! authors/a1/A1.tsv); independent check authors/pg/w/pg_fut_ref.py. Runs on the Norgate Eurodollar continuous contract only
//! (futures::CARDS_PG, R4 #293 (1)). Not the same rule as live scpanag_2close (Panagakis' T-bond/yen two-close system).
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::sctraub::{period_ends, week_groups};
use super::tselhs::Book;

pub const REVIEWED: &[&str] = &["scpged_weekly"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Literal printed rule, 0 unstated details; unverified] Weeks are calendar weeks of the daily bars; the settlement price of the week is the close of its last trading day (the data's last week counts only when it ends on a Friday). When that settlement is above the highest high of the two previous weeks' daily bars the position becomes long, when it is below their lowest low it becomes short, at that settlement price (reversing any opposite position, 'we reversed our short position and went long the market from this price'); otherwise the position is kept. Always in the market after the first signal. The article trades the June and December contracts and rolls on the first weekly close of the expiry month ('This new position will be put on as if we were already trading the new contract'); the vault runs the Norgate Eurodollar continuous contract (signals on the back-adjusted series, each trade priced in the contract held at entry; R4 #293, #301). Norgate continuous futures, ends 2020-09-11; roll method not stated by the vendor. Closes only: no same-day order (#240 n/a). #168 against Figure 5 (fiscal years 1982-1993): 99 closed entries including about two contract rolls a year, net +1,481 basis points (1 basis point = 0.01 = $25).";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "scpged_weekly", name: "Panagakis Eurodollar indicator: the week's settlement beyond the previous two weeks' high or low reverses the position (Eurodollar futures)", group: "S&C: trading the Eurodollar (Panagakis, V.13:3)", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "scpged", rules: &[
            "F|\"In my Eurodollar indicator, basically the only work you have to do is keep track of the daily high, low and settlement price of each trading day. The most important price is the settlement price of the last trading day of the week, usually Friday.\" (V.13:3, PDF p.2)",
            "E|\"Whenever the settlement price of the week is higher than the highest price of the previous two weeks, we will initiate a long position in the market. Whenever the settlement price of the week is lower than the lowest price of the previous two weeks, we will initiate a short position in the market. It's that simple.\" (PDF p.2-3)",
            "X|\"This price was higher than the highest price of the previous two weeks, so we reversed our short position and went long the market from this price. We maintained this position until we received a sell signal.\" (PDF p.3)",
            "F|Figure 5 (1982-93 trade tables, fiscal years, June/December contracts): yearly totals 8 trades +170, 12 +49, 7 +276, 9 -16, 5 +241, 8 +274, 5 +252, 8 +148, 8 -18, 10 -50, 10 +42, 9 +113 basis points (roll closes counted as trades)"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { reviewed(id).then_some(("scpged_weekly", "Panagakis Eurodollar weekly breakout (S&C)", "2 weeks")) }
pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    reviewed(id).then(|| vec![ParamDef { key: "weeks", label: "Previous weeks in the range (article 2)", def: 2., min: 1., max: 12., step: 1. }])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from > to || to >= x.c.len() { return None; }
    let n = p.get("weeks").copied().unwrap_or(2.).round().max(1.) as usize;
    let ends: std::collections::BTreeSet<usize> = period_ends(x, false).into_iter().collect();
    let g = week_groups(x);
    let mut pos = Book::new();
    for j in n..g.len() {
        let e = g[j].1;
        if e > to { break; }
        if e < from || e == to || !ends.contains(&e) { continue; }
        let (a, b) = (g[j - n].0, g[j - 1].1);
        let hh = x.h[a..=b].iter().copied().fold(f64::MIN, f64::max);
        let ll = x.l[a..=b].iter().copied().fold(f64::MAX, f64::min);
        if x.c[e] > hh { pos.enter(1., e, x.c[e], "close"); } else if x.c[e] < ll { pos.enter(-1., e, x.c[e], "close"); }
    }
    Some(pos.done(to, x.c[to]))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn weekly_reversals_at_week_closes() {
        let r = super::super::sctraub::tests::bars(1500);
        let x = Ctx::new(&r, &r, &[]);
        let t = run("scpged_weekly", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(t.len() > 5 && t.windows(2).all(|w| w[1].legs[0].i == w[0].exit_i && w[1].dir == -w[0].dir));
        let ends = period_ends(&x, false);
        assert!(t.iter().all(|z| ends.contains(&z.legs[0].i)));
    }
}
