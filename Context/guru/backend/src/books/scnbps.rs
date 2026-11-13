//! Nauzer Balsara, "Using Probability Stops In Trading", Technical Analysis of Stocks & Commodities V.10:5 (1992) pp.225-228
//! (research share Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V10/C05/USINGPR.pdf).
//! Book id `scnbps`: dual moving-average crossovers with the article's drawdown ('probability') stops. Lane PG (13 Nov 2026,
//! handover of lane PA1's row; PA1 had filed verifier questions, answered here under R4 #297); independent check
//! authors/pg/w/pg_fut_ref.py. Runs on the five Norgate continuous contracts of Figures 1-5 (futures::CARDS_PG).
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::msws::sma;
use super::tselhs::Book;

pub const REVIEWED: &[&str] = &["scnbps_cross"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

/// The four crossover pairs (Figures 1-5, in printed order).
pub const PAIRS: [(usize, usize); 4] = [(6, 27), (9, 33), (12, 39), (15, 45)];
/// Stops in ticks per contract and pair (Figures 1-5, 'Profit/(Loss) using N-tick stop').
pub const STOPS: &[(&str, [f64; 4])] = &[("&GE", [18., 25., 22., 19.]), ("&6S", [57., 120., 53., 45.]), ("&SP", [130., 192., 115., 167.]), ("&LE", [85., 115., 162., 249.]), ("&CL", [61., 35., 54., 43.])];

const NOTE: &str = "[Adapted: 2 unstated details, R4 #297; unverified] 'A dual moving average crossover system generates a buy signal when the shorter of two moving averages exceeds the longer one; a sell signal is generated when the shorter moving average falls below the longer moving average.' Each crossover reverses the position at that day's close ('All the testing was based on only the closing prices'). The probability stop is the article's drawdown cutoff for the contract and pair (Figures 1-5, in ticks: Eurodollar 18/25/22/19, Swiss franc 57/120/53/45, S&P 500 130/192/115/167, live cattle 85/115/162/249, crude oil 61/35/54/43 for the 6/27, 9/33, 12/39 and 15/45 pairs), checked on closes: out at the close on which the close is that many ticks or more against the entry close. 'Once stopped out of a trade, the system stays neutral until a reversal signal is generated.' Unstated: (1) simple or exponential averages - vault reading: simple; (2) a loss of exactly the stop - vault reading: stopped (the cutoff is reached). Default pair 6/27 (the first printed; the article picked all four at random and prints no preferred one), options pair = 2-4, stop = ticks (0 = the article's no-stop column). The stops were fitted on 1988-89 and tested on 1990-01..1991-11 (Figures 1-5). Runs only on the five Norgate continuous contracts of the article (Eurodollar, Swiss franc, S&P 500, live cattle, crude oil; R4 #293): signals on the back-adjusted series, each trade priced in the contract held at entry (#301). Norgate continuous futures, ends 2020-09-11; roll method not stated by the vendor. Closes only: no same-day order (#240 n/a). On crude oil the run ends on 2020-04-17, the day before NYMEX crude settled at -37.63 (no % base; R4 #316/#326).";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "scnbps_cross", name: "Balsara moving-average crossover with probability stops: 6/27-day cross reverses at the close, out on the article's tick drawdown stop (five futures)", group: "S&C: using probability stops in trading (Balsara, V.10:5)", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "scnbps", rules: &[
            "E|\"A dual moving average crossover system generates a buy signal when the shorter of two moving averages exceeds the longer one; a sell signal is generated when the shorter moving average falls below the longer moving average.\" (V.10:5, PDF p.2)",
            "X|\"The optimal unrealized loss level is set equal to the maximum unrealized loss registered on 90% of all winning trades. These drawdown cutoff rules are then applied to data for the period of January 1990 to November 1991\" (PDF p.2); \"Once stopped out of a trade, the system stays neutral until a reversal signal is generated.\" (PDF p.2)",
            "F|\"All the testing was based on only the closing prices, and each of the parameters of the moving average crossover systems were randomly selected. The stop-loss points were selected based on testing from January 1988 to December 1989.\" (Figure 1 note)",
            "F|Figures 1-5 (1990-01..1991-11, with stops): Eurodollar 6/27 5 winners 12 losers $(425), 9/33 5/10 $(2,550), 12/39 6/5 $6,050, 15/45 3/6 $425; Swiss franc 9/14 $20,238, 9/10 $19,287, 5/12 $8,500, 3/6 $27,938; S&P 500 6/18 $5,175, 7/15 $(9,225), 5/13 $7,475, 3/12 $(13,125); live cattle 6/21 $(2,776), 9/12 $1,484, 4/13 $(3,756), 3/12 $(6,220); crude oil 9/12 $15,950, 5/13 $(2,610), 8/10 $29,340, 6/4 $16,540"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { reviewed(id).then_some(("scnbps_cross", "Balsara crossover + probability stop (S&C)", "6/27")) }
pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    reviewed(id).then(|| vec![
        ParamDef { key: "pair", label: "Crossover pair: 1 = 6/27, 2 = 9/33, 3 = 12/39, 4 = 15/45", def: 1., min: 1., max: 4., step: 1. },
        ParamDef { key: "stop", label: "Stop in ticks (-1 = the article's stop for this contract and pair; 0 = no stop)", def: -1., min: -1., max: 1000., step: 1. }])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from > to || to >= x.c.len() { return None; }
    let k = (p.get("pair").copied().unwrap_or(1.).round().clamp(1., 4.) as usize) - 1;
    let sym = crate::futures::ctx_market(x).map(|m| m.sym).unwrap_or("");
    let given = p.get("stop").copied().unwrap_or(-1.);
    let ticks = if given >= 0. { given } else { STOPS.iter().find(|s| s.0 == sym).map_or(0., |s| s.1[k]) };
    let stop = ticks * x.market_tick();
    let (f, s) = (sma(&x.c, PAIRS[k].0), sma(&x.c, PAIRS[k].1));
    let side = |t: usize| if f[t] > s[t] { 1. } else if f[t] < s[t] { -1. } else { 0. };
    let mut pos = Book::new();
    let mut last = 0.; // last nonzero crossover side seen
    for t in 0..=to {
        let sd = side(t);
        let cross = sd != 0. && last != 0. && sd != last;
        if sd != 0. { last = sd; }
        if t < from { continue; }
        let cur = pos.side();
        if cur != 0. && stop > 0. && cur * (pos.entry_px() - x.c[t]) >= stop - 1e-9 { pos.exit(t, x.c[t], "probability stop"); }
        if cross && t < to { pos.enter(sd, t, x.c[t], "close"); }
    }
    Some(pos.done(to, x.c[to]))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn crossovers_reverse_and_stops_wait_for_a_reversal() {
        let r = super::super::sctraub::tests::bars(2000);
        let x = Ctx::new(&r, &r, &[]);
        let mut p = Params::new();
        p.insert("stop".into(), 150.); // 1.50 points at the 0.01 tick
        let t = run("scnbps_cross", &x, 0, r.len() - 1, &p).unwrap();
        assert!(t.iter().any(|z| z.reason == "probability stop") && t.iter().any(|z| z.reason == "reversed"));
        for w in t.windows(2) { if w[0].reason == "probability stop" { assert!(w[1].dir == -w[0].dir, "after a stop only the opposite crossover re-enters"); } }
        for z in t.iter().filter(|z| z.reason == "probability stop") { assert!(z.dir * (z.legs[0].px - z.exit_px) >= 1.5 - 1e-9); }
    }
    #[test]
    fn crude_run_stops_before_a_negative_unadjusted_price() {
        // R4 #326: on &CL the card's run ends the day before the first non-positive unadjusted price
        let r: Vec<crate::Row> = (0..6).map(|i| crate::Row { ticker: "Crude Oil (Norgate &CL)".into(), date: format!("2020-04-{:02}", i + 14), open: 20., high: 20., low: 20., close: 20., volume: None }).collect();
        let mut x = Ctx::new(&r, &[], &[]);
        for k in ["RAW.Open", "RAW.High", "RAW.Low"] { x.extra.insert(k.into(), vec![25.; 6]); }
        x.extra.insert("RAW.Close".into(), vec![25., 25., 25., 25., -37.63, 10.]);
        assert_eq!(crate::futures::first_nonpositive("scnbps_cross", &x), Some(4));
    }
}
