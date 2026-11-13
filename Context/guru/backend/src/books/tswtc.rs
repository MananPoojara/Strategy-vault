//! William Tell Coffee Strategy (Market Analytics, marketanalytics.net), TradeStation code "WT Coffe Strategy.txt" (research share
//! USB HDD 1/TradeStation Systems & Indicators/Market Analytics - William Tell Coffee Trading System (marketanalytics.net)/WT Coffe
//! Strategy.txt). Book id `tswtc`. Lane K18 (titles loop, 2 Nov 2026), to-code row 330; independent check
//! authors/titles-1030/K18w/k18_ref.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::rrsim::{highest, o, run_ts, A, P};
use super::sctraub::{civil, days, ticks, weekday};

pub const REVIEWED: &[&str] = &["tswtc_coffee"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Manual rules are the default (lane PG, 14 Nov 2026; R4 #293, RULES 'book rule default, old behaviour labelled'); Adapted: 2 unstated details, R4 #297] The system's user manual (Manual.pdf, same folder) completes the printed code: 'use a large stop ($2750)' - 7.35 points of coffee, as in the manual's example (sell at 175.36, buy stop 182.71); 'Exit at first profitable opening' (an open beyond the entry price); 'Ignore all new trades if currently in the market'; after a profitable-open exit a new entry the same day is allowed, after a stop-out it is not. The 13 patterns are the printed code (orders decided at the close for the next day, some reading the next open as the code does; 'highest(h,4) - 1 point' with 1 point = 1 tick, #57). Prices are not rounded (the code does not round). Unstated (labelled same-day conventions, R4 #240): (1) two orders that could both fill in one day ('take the signal that gets hit first') - the order filled at the open first, otherwise the one nearest the open, ties to the lower pattern number; (2) the stop on the entry day - checked only after an entry at the open. Option pattern = 1-13 runs one pattern alone (the manual's per-pattern tables). Option manual = 0 is the card's earlier reading of the code alone (no stop; an opposite entry reverses a position) - kept as a labelled option. Runs only on the Norgate coffee continuous contract (the manual's market; R4 #293 (1)): signals on the back-adjusted series, each trade priced in the contract held at entry (#301). Norgate continuous futures, ends 2020-09-11; roll method not stated by the vendor. No costs (the manual deducts $75 a trade).";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "tswtc_coffee", name: "William Tell Coffee: fourteen day-of-week price patterns with stop / limit / open entries, exit at the first profitable open", group: "TradeStation code: William Tell Coffee (Market Analytics)", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "tswtc", rules: &[
            "E|if marketposition=0 or (marketposition=1 and open of next bar > entryprice) or (marketposition=-1 and open of next bar < entryprice) then begin ... entries (1) to (13d) ... end",
            "X|if open of next bar > entryprice and marketposition=1 then Sell next bar at open; if open of next bar < entryprice and marketposition=-1 then Buy to Cover next bar at open",
            "X|\"use a large stop and exit a position at the first profitable opening ... By using a large stop ($2750), the odds of having a profitable opening before getting stopped out are very much in our favor.\" (Manual.pdf p.2); example \"Pattern#5: SELL at 175.36 stop / Buy Stop: 182.71\" (p.3)",
            "F|\"Exit at first profitable opening / Ignore all new trades if currently in the market\" (Manual p.3); \"Should a situation arise where you have two different signals (conflicting or otherwise) take the signal that gets hit first and ignore the other\"; \"Should the market open below 196.00, you can exit your short position with a profit and are free to enter into a new position on the same date.\" (p.4)",
            "F|Manual performance summaries ($75 a trade deducted): combined 1/1/86-4/2/97 417 trades, 85% profitable, $230,170; per pattern 1/2/80-4/2/97: #1 24 trades 88%, #2 67 87%, #3 46 91%, #4 46 89%, #5 70 84%, #6 46 91%, #7 62 87%, #8 22 95%, #9 129 85%, #10 87 83%, #11 228 78%, #12 87 83%, #13 29 86%"],
        notes: NOTE, claim: crate::connors::claim("417", "85%", "$552 a trade net of $75", "", "Coffee futures 1/1/86-4/2/97, 13 patterns combined") }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { reviewed(id).then_some(("tswtc_coffee", "William Tell Coffee (TradeStation)", "code as printed")) }

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    reviewed(id).then(|| vec![p("tick", "Rule 12 offset in ticks (1 point = 1 tick)", 1., 0., 5., 1.),
        p("manual", "1 = the manual's rules ($2,750 stop, ignore signals while in a trade); 0 = the printed code alone (no stop, opposite entries reverse)", 1., 0., 1., 1.),
        p("pattern", "Pattern (manual rules): 0 = all 13, 1-13 = one pattern alone", 0., 0., 13., 1.),
        p("stop", "Stop in points from the entry (manual $2,750 = 7.35 cents a pound)", 7.35, 0.5, 50., 0.05)])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let pt = ticks(x, p);
    if p.get("manual").copied().unwrap_or(1.) != 0. {
        let pat = p.get("pattern").copied().unwrap_or(0.).round().clamp(0., 13.) as u8;
        return Some(manual::sim(x, from, to, pat, p.get("stop").copied().unwrap_or(7.35), pt).0);
    }
    let (o_, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let n = c.len();
    Some(run_ts(x, from, to, |t, st, _now, next| {
        if t < 4 || t + 1 >= n { return; }
        let nx = o_[t + 1];
        let mp = st.pos;
        if !(mp == 0 || (mp == 1 && nx > st.entry_px) || (mp == -1 && nx < st.entry_px)) { return; }
        if nx > st.entry_px && mp == 1 { next.push(o(A::Xl, P::Mkt, "exit at a profitable open")); }
        if nx < st.entry_px && mp == -1 { next.push(o(A::Xs, P::Mkt, "exit at a profitable open")); }
        let date = x.r[t].date.as_str();
        let dw = weekday(date) + 1; // TradeStation DayOfWeek: 1 = Monday ... 5 = Friday
        let (_, m, d) = civil(days(date));
        let mut v: Vec<(A, P, &'static str)> = vec![];
        let r = h[t] - l[t];
        if (dw == 4 || dw == 5) && l[t] > h[t - 1] { v.push((A::Sell, P::Stop(h[t - 1]), "1")); }
        if m == 10 && dw == 5 { v.push((A::Buy, P::Mkt, "2")); }
        if dw != 2 && dw != 5 && l[t] > h[t - 2] && l[t - 1] >= h[t - 3] && nx < (h[t] - l[t]) * 0.3 + h[t] { v.push((A::Buy, P::Stop(h[t] + 0.3 * r), "3")); }
        if (dw == 1 || dw == 3 || dw == 4) && l[t] > l[t - 1] && l[t - 1] < l[t - 2] && l[t - 2] > l[t - 3] && l[t - 3] < l[t - 4] { v.push((A::Sell, P::Stop(l[t]), "4")); }
        if h[t - 1] > h[t - 2] && h[t - 1] > h[t] { v.push((A::Sell, P::Stop(l[t] - 1.7 * r), "5")); }
        if dw == 5 && o_[t - 1] > c[t - 2] && o_[t] < c[t - 1] { v.push((A::Buy, P::Stop(nx + 0.8 * r), "6")); }
        if (dw == 1 || dw == 2) && c[t - 3] < o_[t - 3] && c[t - 2] < o_[t - 2] && c[t - 1] < o_[t - 1] && c[t] < o_[t] { v.push((A::Buy, P::Lim(h[t] - r), "7a")); }
        if dw == 2 && c[t - 3] > o_[t - 3] && c[t - 2] < o_[t - 2] && c[t - 1] < o_[t - 1] && c[t] < o_[t] { v.push((A::Buy, P::Lim(h[t] - 0.9 * r), "7b")); }
        if dw != 2 && dw != 3 && h[t] > h[t - 1] && l[t] < l[t - 1] && c[t] > c[t - 1] { v.push((A::Sell, P::Stop(nx - 0.5 * r), "8")); }
        if dw == 1 && nx < l[t] { v.push((A::Buy, P::Stop(l[t]), "9")); }
        if dw != 1 && dw != 2 && c[t] < l[t - 1] { v.push((A::Sell, P::Stop(nx - r * 0.9), "10")); }
        if c[t] > h[t - 1] { v.push((A::Sell, P::Stop(nx - r * 0.85), "11")); }
        let h4 = highest(h, 4, t);
        if (dw == 4 || dw == 5) && nx > h4 { v.push((A::Sell, P::Stop(h4 - pt), "12")); }
        let xmas = m == 12 && ((d == 23 && dw == 5) || d == 24 || (d == 22 && dw == 5) || (d == 23 && dw == 4));
        let tg = m == 11 && dw == 3 && civil(days(date) - 20).1 == 11 && civil(days(date) - 27).1 != 11;
        if xmas || tg {
            if c[t] > o_[t] { v.push((A::Buy, P::Mkt, "13")); }
            if c[t] < o_[t] { v.push((A::Sell, P::Mkt, "13")); }
        }
        for (a, pr, tag) in v {
            let d = if a == A::Buy { 1 } else { -1 };
            if d != mp { next.push(o(a, pr, tag)); }
        }
    }))
}

/// The manual's rules (lane PG): the printed patterns with the $2,750 stop, the first-profitable-open exit, no new trade while
/// in the market, and the two labelled same-day conventions. `sim` also returns how many trades depend on a convention.
pub mod manual {
    use crate::connors::{Ctx, Trade};
    use super::super::sctraub::weekday;
    use super::super::tselhs::Book;
    /// One next-day order: (pattern, side, kind 0 market / 1 stop / 2 limit, price).
    type Order = (u8, f64, u8, f64);

    /// TradeStation dayofweek (0 Sunday .. 6 Saturday) of bar i.
    fn ts_dow(x: &Ctx, i: usize) -> i64 { (weekday(&x.r[i].date) + 1) % 7 }
    fn md(x: &Ctx, i: usize) -> (i64, i64) {
        let d = &x.r[i].date;
        (d.get(5..7).and_then(|s| s.parse().ok()).unwrap_or(0), d.get(8..10).and_then(|s| s.parse().ok()).unwrap_or(0))
    }

    /// The orders the code places at bar k's close for bar k + 1 (`no` = bar k + 1's open).
    pub fn orders(x: &Ctx, k: usize, no: f64, pt: f64) -> Vec<Order> {
        let mut v = vec![];
        if k < 4 { return v; }
        let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
        let w = ts_dow(x, k);
        let (m, d) = md(x, k);
        let rng = h[k] - l[k];
        if (w == 4 || w == 5) && l[k] > h[k - 1] { v.push((1, -1., 1, h[k - 1])); }
        if m == 10 && w == 5 { v.push((2, 1., 0, no)); }
        if w != 2 && w != 5 && l[k] > h[k - 2] && l[k - 1] >= h[k - 3] && no < rng * 0.3 + h[k] { v.push((3, 1., 1, h[k] + 0.3 * rng)); }
        if (w == 1 || w == 3 || w == 4) && l[k] > l[k - 1] && l[k - 1] < l[k - 2] && l[k - 2] > l[k - 3] && l[k - 3] < l[k - 4] { v.push((4, -1., 1, l[k])); }
        if h[k - 1] > h[k - 2] && h[k - 1] > h[k] { v.push((5, -1., 1, l[k] - 1.7 * rng)); }
        if w == 5 && o[k - 1] > c[k - 2] && o[k] < c[k - 1] { v.push((6, 1., 1, no + 0.8 * rng)); }
        if (w == 1 || w == 2) && c[k - 3] < o[k - 3] && c[k - 2] < o[k - 2] && c[k - 1] < o[k - 1] && c[k] < o[k] { v.push((7, 1., 2, h[k] - rng)); }
        if w == 2 && c[k - 3] > o[k - 3] && c[k - 2] < o[k - 2] && c[k - 1] < o[k - 1] && c[k] < o[k] { v.push((7, 1., 2, h[k] - 0.9 * rng)); }
        if w != 2 && w != 3 && h[k] > h[k - 1] && l[k] < l[k - 1] && c[k] > c[k - 1] { v.push((8, -1., 1, no - 0.5 * rng)); }
        if w == 1 && no < l[k] { v.push((9, 1., 1, l[k])); }
        if w != 1 && w != 2 && c[k] < l[k - 1] { v.push((10, -1., 1, no - 0.9 * rng)); }
        if c[k] > h[k - 1] { v.push((11, -1., 1, no - 0.85 * rng)); }
        let hh = h[k - 3..=k].iter().copied().fold(f64::MIN, f64::max);
        if (w == 4 || w == 5) && no > hh { v.push((12, -1., 1, hh - pt)); }
        let xmas = m == 12 && ((d == 23 && (w == 5 || w == 4)) || d == 24 || (d == 22 && w == 5));
        let thanks = m == 11 && w == 3 && (21..=27).contains(&d);
        if xmas || thanks {
            if c[k] > o[k] { v.push((13, 1., 0, no)); }
            if c[k] < o[k] { v.push((13, -1., 0, no)); }
        }
        v
    }

    /// Fill of an order on bar t: (price, filled at the open).
    pub fn fill(x: &Ctx, t: usize, od: &Order) -> Option<(f64, bool)> {
        let (_, side, kind, px) = *od;
        let (o, h, l) = (x.o[t], x.h[t], x.l[t]);
        match kind {
            0 => Some((o, true)),
            1 if side > 0. => if o >= px { Some((o, true)) } else if h >= px { Some((px, false)) } else { None },
            1 => if o <= px { Some((o, true)) } else if l <= px { Some((px, false)) } else { None },
            _ => if o <= px { Some((o, true)) } else if l <= px { Some((px, false)) } else { None },
        }
    }

    /// The run, plus the number of trades that depend on a same-day convention (entry choice, entry-day stop).
    pub fn sim(x: &Ctx, from: usize, to: usize, pat: u8, stop: f64, pt: f64) -> (Vec<Trade>, usize) {
        let mut pos = Book::new();
        let mut at_open = false; // the open position was entered at an open
        let mut amb = 0;
        for t in from.max(5)..=to {
            let s = pos.side();
            let mut may_enter = true;
            if s != 0. && pos.entry_bar().is_some_and(|e| e < t) {
                let ep = pos.entry_px();
                if (s > 0. && x.o[t] > ep) || (s < 0. && x.o[t] < ep) { pos.exit(t, x.o[t], "first profitable opening"); }
                else {
                    may_enter = false;
                    let lvl = ep - s * stop;
                    if (s > 0. && x.o[t] <= lvl) || (s < 0. && x.o[t] >= lvl) { pos.exit(t, x.o[t], "stop"); }
                    else if (s > 0. && x.l[t] <= lvl) || (s < 0. && x.h[t] >= lvl) { pos.exit(t, lvl, "stop"); }
                }
            }
            if !may_enter || pos.side() != 0. || t >= to { continue; }
            let cands: Vec<(Order, f64, bool)> = orders(x, t - 1, x.o[t], pt).into_iter().filter(|od| pat == 0 || od.0 == pat)
                .filter_map(|od| fill(x, t, &od).map(|(p, a)| (od, p, a))).collect();
            if cands.is_empty() { continue; }
            let opens: Vec<&(Order, f64, bool)> = cands.iter().filter(|c| c.2).collect();
            let mut dep; // this trade depends on a same-day convention
            let pick = if !opens.is_empty() {
                dep = opens.iter().any(|c| c.0 .1 != opens[0].0 .1);
                opens[0]
            } else {
                dep = cands.iter().any(|c| c.0 .1 > 0.) && cands.iter().any(|c| c.0 .1 < 0.);
                cands.iter().min_by(|a, b| (a.1 - x.o[t]).abs().partial_cmp(&(b.1 - x.o[t]).abs()).unwrap().then(a.0 .0.cmp(&b.0 .0))).unwrap()
            };
            let (od, px, op) = *pick;
            pos.enter(od.1, t, px, "stop");
            at_open = op;
            let lvl = px - od.1 * stop;
            let hit = (od.1 > 0. && x.l[t] <= lvl) || (od.1 < 0. && x.h[t] >= lvl);
            if hit && at_open { pos.exit(t, lvl, "stop"); } else if hit { dep = true; }
            if dep { amb += 1; }
        }
        (pos.done(to, x.c[to]), amb)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn coffee_patterns_trade() {
        let r = super::super::sctraub::tests::bars(2500);
        let x = Ctx::new(&r, &r, &[]);
        let t = run("tswtc_coffee", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(t.len() > 5 && t.iter().any(|z| z.dir > 0.) && t.iter().any(|z| z.dir < 0.));
        for w in t.windows(2) { assert!(w[1].legs[0].i >= w[0].exit_i); }
        assert!(run("tswtc_x", &x, 0, 5, &Params::new()).is_none());
        // manual rules (default): stop exits lose at least 7.35 points, open exits are profitable, no overlapping trades
        let (m, _) = manual::sim(&x, 0, r.len() - 1, 0, 7.35, 0.01);
        assert_eq!(m, t);
        for z in &m {
            if z.reason == "first profitable opening" { assert!(z.dir * (z.exit_px - z.legs[0].px) > 0. && z.exit_px == x.o[z.exit_i]); }
            if z.reason == "stop" { assert!(z.dir * (z.exit_px - z.legs[0].px) <= -7.35 + 1e-9); }
        }
        let mut old = Params::new();
        old.insert("manual".into(), 0.);
        assert!(run("tswtc_coffee", &x, 0, r.len() - 1, &old).unwrap() != m);
    }
}

// Instrument admission: none since lane PG (14 Nov 2026) - the card runs only on its Norgate coffee contract (futures::CARDS_PG).

