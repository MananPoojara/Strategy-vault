//! Lane K81 (13 Nov 2026; authors/titles-1030/K81.tsv): George Cole, "Graphs and Their Application to Speculation" (1936).
//! Book id "gcgas". Source copy: research share 'USB HDD 2/tRADING KNO-2/George Cole - Graphs, Application to Speculation/'
//! (publisher's OCR as .doc/.rtf chapters; source has no page numbers in the files, the printed page markers '_150_' etc.
//! are inside the text). Read: Contents/1-3.rtf and Chap 07 (ch.VII 'Mechanical Rules', printed p.145-168) in full.
//! Reader row: authors/o1/O1.tsv (RULES-COMPLETE).
//! One rule is coded: Plan No. 5 'Going With the Trend' (Ainsworth's plan, p.157-159). Plans 1-3 need a judged entry or the
//! averaging tables of another book, Plan 4 scalps cent offsets on wheat futures, Plan 6 trades named wheat/corn contract
//! months: held with the reason.
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

const BOOK: &str = "gcgas";
const SRC: &str = "Cole, Graphs and Their Application to Speculation (1936), ch.VII 'Mechanical Rules'; printed page markers in the publisher's text (Chap 07 .rtf files).";
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == BOOK && def.id == "gcgas_plan5",
    label: "Cole trades Plan No. 5 in wheat futures, one unit of 5,000 bushels at a time. It uses only closes (three in a row) and market orders at the next open, so it runs on US and Indian indices, ETFs and single stocks as a declared application; each unit is the same quantity as the first.",
    basis: "George Cole, Graphs and Their Application to Speculation (1936) ch.VII p.157-159 (Plan No. 5); multi-unit account books/units.rs (decision 211); independent check authors/titles-1030/K81w/k81_ref.py (cole_plan5).",
};

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, dir, status, rules, notes| StrategyDef { id, name, group: "Ch.VII · Mechanical Rules", session: 0, dir, status, needs_volume: false,
        needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim: None };
    vec![
        d("gcgas_plan5", "Plan No. 5, going with the trend: a unit at every other close in the run, reverse on the third close the other way", "Long/Short", Status::Live, &[
            "E|\"On the third day, after the market starts a movement in either direction, that is, closing three days in succession in the same direction, counting the last close in the previous run as the first day sell or buy at the opening of the next day, one unit\" (p.158)",
            "E|\"the fifth day, if the market closes higher or lower in the same direction, buy or sell on the opening the next (the sixth) day ... every other day a unit is accumulated with the trend\" (p.158)",
            "X|\"on the third closing up after the low closing has been made, we buy in our entire line and also one additional unit, making us long one unit\" (p.158)"],
          concat!("Cole, Graphs and Their Application to Speculation (1936), ch.VII 'Mechanical Rules'; printed page markers in the publisher's text (Chap 07 .rtf files). ",
            "A run counts the closes in one direction, the last close of the previous run (the low or high close) as the first: a close above the one before extends an up run, anything else starts a new run at that close (an unchanged close ends both runs; reading). ",
            "Flat: the third close of a run buys (up) or sells short (down) one unit at the next open. In the market: every odd close of a run with the position (3rd, 5th, 7th ...) adds one unit at the next open ('every other day a unit is accumulated', p.158; a new run after a one-day pause counts again from its first close, reading); the third close of a run against the position closes the whole line at the next open and opens one unit the other way. ",
            "No stop and no limit on the number of units: each unit is the same quantity as the first, so the position grows with the run (multi-unit account, decision 211; every unit is listed as its own trade). ",
            "Not applied: the author's modification (a stop 1/4 cent beyond 'the last reversing point' after 'a temporary reverse', p.159), a judged cent amount on wheat; and the switch to Plan No. 4 in congestions, which 'a study of the graph will indicate' (p.159). ",
            "Wheat futures in the book; here on indices, ETFs and single stocks as a declared application. The book's record (Oct 1931 - Jun 1932, p.161-164) mixes Plans 4 and 5 by judgement: no Plan 5 result is printed.")),
        d("gcgas_plans_1_3", "Plans No. 1-3: stop profit, stop-profit averaging and averaging-and-doubling", "Long/Short",
          Status::NotTestable("Held: each plan starts from a judged entry ('After deciding where and how you want to enter the market', p.149-150; 'a great deal depends upon judgment in entering the market', p.150), Plan 1 gives no stop or profit distance, and Plans 2-3 take their spacing from the tables of another book, Successful Speculation a Business (p.150-152)."), &[
            "E|\"you make your commitment and then immediately place a stop loss order at a certain distance ... a stop profit order at an equal distance\" (p.149-150)"],
          SRC),
        d("gcgas_plan4", "Plan No. 4, scalping against the trend: sell 1/2 cent over yesterday's high, buy 1/2 cent under its low", "Long/Short",
          Status::NotTestable("Held: the orders are cent amounts of wheat (1/2 cent beyond the previous day's high and low, the spacing widening to 3/4, 1 and 1 1/2 cents as units accumulate; Cole's own variant 3/4 cent from the close with a 1/4 cent step, p.153-157), which have no exact conversion to an index, ETF or stock (decision #19), and wheat futures are not loaded in the engine. Cole also runs it 'only while the market is in a congestion' (p.153), judged from the graph."), &[
            "E|\"Before the opening of the market, regardless of whether you are OUT or LONG or SHORT, enter an order (good only for the day) to sell 5000 bushels at one-half cent above the highest price reached on the previous day, and an order to buy 5000 bushels at one-half cent below the price reached the previous day\" (p.153)"],
          SRC),
        d("gcgas_plan6", "Plan No. 6, year-around plan: May/July/September wheat and corn bought at the previous month's low", "Long",
          Status::NotTestable("Held: the plan buys named futures contracts (May Wheat, May Corn, July Corn, September Wheat) at the previous month's low and sells on 10% / 5% advances (p.165-168): it needs each contract month's own prices, which the vault does not hold (continuous futures only), and 'judgment enters into it to a large extent' (p.165)."), &[
            "E|\"In September he will buy 10,000 bushels each of May Wheat and May Corn at the lowest price reached in August\" (p.165-166)"],
          SRC),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let d = catalogue().into_iter().find(|d| d.id == id)?;
    Some((d.id, d.name, "Cole, Mechanical Rules (ch.VII)"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    matches!(id, "gcgas_plan5" | "gcgas_plans_1_3" | "gcgas_plan4" | "gcgas_plan6").then(Vec::new)
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, _p: &Params) -> Option<Vec<Trade>> {
    if id != "gcgas_plan5" { return None; }
    let n = x.r.len();
    if n == 0 || from > to || to >= n { return Some(vec![]); }
    let (mut up, mut dn) = (1usize, 1usize); // closes in the current up / down run, its first close counted
    let mut out: Vec<Trade> = vec![];
    let mut cur: Option<Trade> = None;
    for t in (from + 1)..=to {
        if x.c[t] > x.c[t - 1] { up += 1; dn = 1; } else if x.c[t] < x.c[t - 1] { dn += 1; up = 1; } else { up = 1; dn = 1; }
        if t == to { break; } // orders go to the next open
        let o = t + 1;
        let sig = if up >= 3 && up % 2 == 1 { 1. } else if dn >= 3 && dn % 2 == 1 { -1. } else { 0. };
        if sig == 0. { continue; }
        let third = (sig > 0. && up == 3) || (sig < 0. && dn == 3);
        let new = |o: usize| Trade { dir: sig, legs: vec![Leg { i: o, px: x.o[o], w: 1. }], exit_i: o, exit_px: f64::NAN, reason: String::new(),
            entry_kind: "open", signal_i: o, open: false, exits: vec![] };
        match cur.as_ref().map(|p| p.dir) {
            Some(d) if d == sig => cur.as_mut().unwrap().legs.push(Leg { i: o, px: x.o[o], w: 1. }),
            Some(_) if third => {
                let mut q = cur.take().unwrap();
                (q.exit_i, q.exit_px, q.signal_i, q.reason) = (o, x.o[o], t, "third close the other way: reverse at the open".into());
                out.push(q);
                cur = Some(new(o));
            }
            None if third => cur = Some(new(o)),
            _ => {}
        }
    }
    if let Some(mut q) = cur { (q.exit_i, q.exit_px, q.signal_i, q.open, q.reason) = (to, x.c[to], to, true, "open".into()); out.push(q); }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn third_close_enters_odd_closes_add_and_third_close_against_reverses() {
        // closes: 10 11 12 (3rd up close -> buy the open of bar 3) 13 14 (5th -> add at bar 5) 15 13 12 (3rd down close counting 15 -> reverse at the open of bar 8) 12 12
        let c = [10., 11., 12., 13., 14., 15., 13., 12., 12., 12.];
        let r: Vec<Row> = c.iter().enumerate().map(|(k, &v)| Row { ticker: "T".into(), date: format!("2020-01-{:02}", k + 1), open: v + 0.5, high: v + 1., low: v - 1., close: v, volume: None }).collect();
        let x = Ctx::new(&r, &r, &[]);
        let t = run("gcgas_plan5", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert_eq!(t.len(), 2);
        assert_eq!(t[0].legs.iter().map(|l| l.i).collect::<Vec<_>>(), vec![3, 5]);
        assert_eq!((t[0].dir, t[0].exit_i, t[0].exit_px), (1., 8, 12.5));
        assert_eq!((t[1].dir, t[1].legs[0].i, t[1].open), (-1., 8, true));
        assert!(crate::books::units::is_units("gcgas_plan5"));
    }
}
