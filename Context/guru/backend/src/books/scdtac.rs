//! Walter T. Downs, "The Tactical Trader", Technical Analysis of Stocks & Commodities V.17:1 (January 1999) pp.27-37
//! (research share Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V17/C01/003TACT.pdf).
//! Book id `scdtac`. Lane PA1 (13 Nov 2026, pool A of authors/a1/A1.tsv; R4 #293, #297). Each card runs on the Norgate
//! continuous contract the article tests (futures.rs CARDS_PA1). Independent check authors/pa1/pa1_ref.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::lc3::{hi, lo};
use super::sctraub::week_groups;
use super::tselhs::Book;

pub const QUADS: &[(&str, &str, f64)] = &[("scdtac_quads_zb", "US T-Bond", 1.), ("scdtac_quads_zs", "Soybeans", -1.), ("scdtac_quads_dx", "US Dollar Index", 0.), ("scdtac_quads_sp", "S&P 500", 1.)];
pub const REVIEWED: &[&str] = &["scdtac_gaps_zs", "scdtac_quads_zb", "scdtac_quads_zs", "scdtac_quads_dx", "scdtac_quads_sp"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

fn def(id: &'static str, name: &'static str, dir: &'static str, notes: &'static str, rules: &'static [&'static str]) -> StrategyDef {
    StrategyDef { id, name, group: "S&C: The Tactical Trader (Downs, V.17:1)", session: 0, dir, status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: "scdtac", rules, notes, claim: None }
}
const R_GAPS: &[&str] = &["E|\"On a weekly chart of soybeans, the opening price of the week must gap higher than the previous week's high. This gap week becomes our setup bar.\" \"In the week following the gap week, if the market opens below the gap week high and then exceeds the high of the gap week, we will go long the market.\" (V.17:1 p.28)",
    "X|\"We will exit our long positions on the opening of the following week.\" (p.28); \"No protective stops were used.\" (p.30)"];
const R_QUADS: &[&str] = &["E|\"On a monthly chart, if the following mathematical condition is true, then our setup month is complete: H < H[n] - (Range[n]*R) AND L > L[n]+(Range[n]*R) ... In this tactical pattern, n = 1 and R = 0.01.\" (p.31)",
    "E|\"US Treasury bonds have a bullish pattern; if prices exceed the high of the setup month, we will go long the market. Soybeans have a bearish pattern; if prices exceed the low of the setup month, we will go short the market. And finally, the US dollar has a symmetrical pattern.\" (p.31); S&P 500: 'a winning percentage of 89%' on long trades (Figures 11-12)",
    "X|\"We will exit all positions at the end of the month.\" (p.31)"];

pub fn catalogue() -> Vec<StrategyDef> {
    let mut v = vec![def("scdtac_gaps_zs", "Downs soybean gaps: after a weekly up-gap, buy the next week's move back above the gap week's high, out at the next week's open", "Long",
        note("scdtac_gaps_zs"), R_GAPS)];
    for (id, _, _) in QUADS {
        let (name, dir) = match *id {
            "scdtac_quads_zb" => ("Downs equilateral quads · T-bonds: after an inside month (1% of range), buy above its high, out at month end", "Long"),
            "scdtac_quads_zs" => ("Downs equilateral quads · soybeans: after an inside month (1% of range), sell below its low, out at month end", "Short"),
            "scdtac_quads_dx" => ("Downs equilateral quads · US dollar: after an inside month (1% of range), trade its breakout either way, out at month end", "Long/Short"),
            _ => ("Downs equilateral quads · S&P 500: after an inside month (1% of range), buy above its high, out at month end", "Long"),
        };
        v.push(def(id, name, dir, note(id), R_QUADS));
    }
    v
}

pub fn note(id: &str) -> &'static str {
    match id {
        "scdtac_gaps_zs" => concat!("[Adapted: 1 unstated detail, R4 #297; unverified] Soybean gaps (weekly; weeks = calendar weeks of the daily bars): a gap week opens above the previous week's high. In the next week, if the market opens below the gap week's high and then trades above it, buy: a buy stop at the gap week's high, filled at that level or at a day's open beyond it. Exit at the open of the following week ('We will exit our long positions on the opening of the following week'). Long only, no stop ('No protective stops were used'). Unstated in the article: (1) whether 'exceeds' means a tick above the high; vault reading: the stop at the high itself (the article's printed S&P quads fills sit exactly on the setup high), option tick = 1 adds one tick. Article (soybeans, 02/02/68-08/02/96, 5 contracts, $75 a contract for costs): 46 trades, 30 winners (65%), +$128,062.50, profit factor 3.61 (Figure 1); 1986-96: 9 trades, 7 winners (Figure 3). Runs only on Soybeans (Norgate &ZS). ", "Norgate continuous futures, ends 2020-09-11; roll method not stated by the vendor (R4 #293 (5)): signals and point P&L on the back-adjusted series, each trade's % on the unadjusted entry price (#301 (1)). The article used Omega's back-adjusted '67/99' continuous series; its roll dates differ from Norgate's, so its weeks and months do not all line up with ours."),
        "scdtac_quads_zb" => concat!("[T-bonds, long only; runs only on US T-Bond (Norgate &ZB)] Article (T-bonds monthly 08/31/77-07/31/96, 5 contracts, costs subtracted): 14 trades, 13 winners (93%), +$110,218.75 (Figure 7). ", "[Adapted: 1 unstated detail, R4 #297; unverified] Equilateral quads (monthly; months = calendar months of the daily bars): the setup month is inside the previous month by 1% of that month's range: H < H[1] - Range[1] x 0.01 AND L > L[1] + Range[1] x 0.01 (n = 1, R = 0.01). In the next month a buy stop at the setup month's high (bullish markets) and/or a sell stop at its low (bearish markets) works; it fills at the level or at a day's open beyond it, and the position is closed at the month's last close ('We will exit all positions at the end of the month'). The article's biases: T-bonds bullish (long only), soybeans bearish (short only), US dollar symmetrical (both; once one side fills the other order is cancelled, and on a day that reaches both, the level nearer the open fills first), S&P 500 bullish (long only). Unstated in the article: (1) whether the setup month's stop works only in the next month; vault reading: the next month only (each month bar is a new setup test). The printed fills sit exactly on the setup high (S&P 11/29/85: +11.95 points = the Norgate November close 202.40 minus the October high 190.45). ", "Norgate continuous futures, ends 2020-09-11; roll method not stated by the vendor (R4 #293 (5)): signals and point P&L on the back-adjusted series, each trade's % on the unadjusted entry price (#301 (1)). The article used Omega's back-adjusted '67/99' continuous series; its roll dates differ from Norgate's, so its weeks and months do not all line up with ours."),
        "scdtac_quads_zs" => concat!("[Soybeans, short only; runs only on Soybeans (Norgate &ZS)] Article (soybeans monthly 02/29/68-07/31/96): 30 trades, 24 winners (80%), +$92,718.75 (Figures 9-10). ", "[Adapted: 1 unstated detail, R4 #297; unverified] Equilateral quads (monthly; months = calendar months of the daily bars): the setup month is inside the previous month by 1% of that month's range: H < H[1] - Range[1] x 0.01 AND L > L[1] + Range[1] x 0.01 (n = 1, R = 0.01). In the next month a buy stop at the setup month's high (bullish markets) and/or a sell stop at its low (bearish markets) works; it fills at the level or at a day's open beyond it, and the position is closed at the month's last close ('We will exit all positions at the end of the month'). The article's biases: T-bonds bullish (long only), soybeans bearish (short only), US dollar symmetrical (both; once one side fills the other order is cancelled, and on a day that reaches both, the level nearer the open fills first), S&P 500 bullish (long only). Unstated in the article: (1) whether the setup month's stop works only in the next month; vault reading: the next month only (each month bar is a new setup test). ", "Norgate continuous futures, ends 2020-09-11; roll method not stated by the vendor (R4 #293 (5)): signals and point P&L on the back-adjusted series, each trade's % on the unadjusted entry price (#301 (1)). The article used Omega's back-adjusted '67/99' continuous series; its roll dates differ from Norgate's, so its weeks and months do not all line up with ours."),
        "scdtac_quads_dx" => concat!("[US dollar, long and short; runs only on US Dollar Index (Norgate &DX)] Article (US dollar index monthly 11/29/85-07/31/96): long 4 trades, 4 winners, +$18,600 (Figures 13, 15); short 6 trades, 4 winners (67%), +$22,575 (Figures 14, 16). ", "[Adapted: 1 unstated detail, R4 #297; unverified] Equilateral quads (monthly; months = calendar months of the daily bars): the setup month is inside the previous month by 1% of that month's range: H < H[1] - Range[1] x 0.01 AND L > L[1] + Range[1] x 0.01 (n = 1, R = 0.01). In the next month a buy stop at the setup month's high and a sell stop at its low work; a stop fills at the level or at a day's open beyond it, and the position is closed at the month's last close ('We will exit all positions at the end of the month'). The US dollar is symmetrical (both sides): once one side fills the other order is cancelled, and on a day that reaches both, the level nearer the open fills first. Unstated in the article: (1) whether the setup month's stop works only in the next month; vault reading: the next month only. ", "Norgate continuous futures, ends 2020-09-11; roll method not stated by the vendor (R4 #293 (5)): signals and point P&L on the back-adjusted series, each trade's % on the unadjusted entry price (#301 (1)). The article used Omega's back-adjusted '67/99' continuous series; its roll dates differ from Norgate's, so its weeks and months do not all line up with ours."),
        _ => concat!("[S&P 500, long only; runs only on S&P 500 (Norgate &SP)] Article (S&P 500 index futures monthly 04/30/82-07/31/96): 9 trades, 8 winners (89%), +$135,500 (Figures 11-12; 11/29/85 buy 306.30, out 318.25 = +11.95 points, the same +11.95 as the Norgate October high 190.45 to the November close 202.40). ", "[Adapted: 1 unstated detail, R4 #297; unverified] Equilateral quads (monthly; months = calendar months of the daily bars): the setup month is inside the previous month by 1% of that month's range: H < H[1] - Range[1] x 0.01 AND L > L[1] + Range[1] x 0.01 (n = 1, R = 0.01). In the next month a buy stop at the setup month's high works; it fills at the level or at a day's open beyond it, and the position is closed at the month's last close ('We will exit all positions at the end of the month'). Unstated in the article: (1) whether the setup month's stop works only in the next month; vault reading: the next month only. ", "Norgate continuous futures, ends 2020-09-11; roll method not stated by the vendor (R4 #293 (5)): signals and point P&L on the back-adjusted series, each trade's % on the unadjusted entry price (#301 (1)). The article used Omega's back-adjusted '67/99' continuous series; its roll dates differ from Norgate's, so its weeks and months do not all line up with ours."),
    }
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match id {
        "scdtac_gaps_zs" => Some(("scdtac_gaps", "Downs soybean gaps (S&C)", "Soybeans")),
        _ => QUADS.iter().find(|q| q.0 == id).map(|q| ("scdtac_quads", "Downs equilateral quads (S&C)", q.1)),
    }
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    if id == "scdtac_gaps_zs" { return Some(vec![p("tick", "Ticks above the gap week's high (vault reading 0)", 0., 0., 4., 1.)]); }
    reviewed(id).then(|| vec![p("r", "R: inside by this fraction of last month's range (article 0.01)", 0.01, 0., 0.25, 0.01)])
}

/// Month groups (first, last daily index) by calendar month.
pub fn month_groups(x: &Ctx) -> Vec<(usize, usize)> {
    let mut g: Vec<(usize, usize)> = vec![];
    for i in 0..x.r.len() {
        match g.last_mut() { Some(m) if x.r[m.1].date.get(0..7) == x.r[i].date.get(0..7) => m.1 = i, _ => g.push((i, i)) }
    }
    g
}

fn gaps(x: &Ctx, from: usize, to: usize, p: &Params) -> Vec<Trade> {
    let tick = p.get("tick").copied().unwrap_or(0.).round().max(0.) * x.market_tick();
    let w = week_groups(x);
    let mut pos = Book::new();
    for k in 2..w.len() {
        let (s1, e1) = w[k - 1];
        let (s, e) = w[k];
        if s > to { break; }
        // exit at the first open of the week after the entry week
        if pos.side() > 0. && pos.entry_bar().is_some_and(|b| b < s) && s >= from { pos.exit(s, x.o[s], "next week's open"); }
        if pos.side() != 0. || s1 < from { continue; }
        let (s0, e0) = w[k - 2];
        // week k-1 is a gap week (opened above week k-2's high), week k opens below its high
        let gap_hi = hi(&x.h, s1, e1);
        if !(x.o[s1] > hi(&x.h, s0, e0)) || !(x.o[s] < gap_hi) { continue; }
        let lv = gap_hi + tick;
        for i in s..=e.min(to) { if x.h[i] >= lv { pos.enter(1., i, x.o[i].max(lv), "stop"); break; } }
    }
    pos.done(to, x.c[to])
}

fn quads(x: &Ctx, side: f64, from: usize, to: usize, p: &Params) -> Vec<Trade> {
    let r = p.get("r").copied().unwrap_or(0.01).max(0.);
    let m = month_groups(x);
    let mut pos = Book::new();
    for k in 2..m.len() {
        let (s, e) = m[k];
        if s > to { break; }
        let (s1, e1) = m[k - 1];
        let (s0, e0) = m[k - 2];
        if s1 < from { continue; } // the setup month lies inside the window
        let (h1, l1, h0, l0) = (hi(&x.h, s1, e1), lo(&x.l, s1, e1), hi(&x.h, s0, e0), lo(&x.l, s0, e0));
        let rg = h0 - l0;
        if !(h1 < h0 - rg * r && l1 > l0 + rg * r) { continue; }
        for i in s..=e.min(to) {
            if pos.side() != 0. { break; }
            let up = side >= 0. && x.h[i] >= h1;
            let dn = side <= 0. && x.l[i] <= l1;
            match (up, dn) {
                (true, false) => pos.enter(1., i, x.o[i].max(h1), "stop"),
                (false, true) => pos.enter(-1., i, x.o[i].min(l1), "stop"),
                (true, true) => if h1 - x.o[i] <= x.o[i] - l1 { pos.enter(1., i, x.o[i].max(h1), "stop") } else { pos.enter(-1., i, x.o[i].min(l1), "stop") },
                _ => {}
            }
        }
        if pos.side() != 0. && e <= to { pos.exit(e, x.c[e], "month end"); }
    }
    pos.done(to, x.c[to])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    if id == "scdtac_gaps_zs" { return Some(gaps(x, from, to, p)); }
    let side = QUADS.iter().find(|q| q.0 == id)?.2;
    Some(quads(x, side, from, to, p))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn quads_and_gaps() {
        let r = super::super::sctraub::tests::bars(5000);
        let x = Ctx::new(&r, &r, &[]);
        let m = month_groups(&x);
        let mi = |i: usize| m.iter().position(|&(s, e)| s <= i && i <= e).unwrap();
        for id in ["scdtac_quads_zb", "scdtac_quads_zs", "scdtac_quads_dx"] {
            let mut p = Params::new(); p.insert("r".into(), 0.);
            let t = run(id, &x, 0, r.len() - 1, &p).unwrap();
            assert!(!t.is_empty(), "{id}");
            for z in &t {
                let k = mi(z.legs[0].i);
                assert!(id != "scdtac_quads_zb" || z.dir > 0.);
                assert!(id != "scdtac_quads_zs" || z.dir < 0.);
                let (h1, l1) = (hi(&x.h, m[k - 1].0, m[k - 1].1), lo(&x.l, m[k - 1].0, m[k - 1].1));
                assert!(h1 < hi(&x.h, m[k - 2].0, m[k - 2].1) && l1 > lo(&x.l, m[k - 2].0, m[k - 2].1));
                if !z.open { assert_eq!(z.exit_i, m[k].1); }
                assert!(z.legs[0].px == if z.dir > 0. { x.o[z.legs[0].i].max(h1) } else { x.o[z.legs[0].i].min(l1) });
            }
        }
        let w = week_groups(&x);
        for z in gaps(&x, 0, r.len() - 1, &Params::new()).iter().filter(|z| !z.open) {
            let k = w.iter().position(|&(s, e)| s <= z.legs[0].i && z.legs[0].i <= e).unwrap();
            assert!(x.o[w[k - 1].0] > hi(&x.h, w[k - 2].0, w[k - 2].1) && x.o[w[k].0] < hi(&x.h, w[k - 1].0, w[k - 1].1));
            assert_eq!(z.exit_i, w[k + 1].0);
        }
    }
}
// Admission: futures.rs CARDS_PA1 (each card on its Norgate contract, R4 #293 (1)), like the other futures cards.
