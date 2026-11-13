//! Lane R2C (13 Nov 2026; authors/titles-1030/R2C.tsv; R4 #297 "up to two unstated details"; reader row authors/o2/O2.tsv):
//! Avery T. Horton Jr. ("TheRumpledOne"), "Trapline Trading Strategy" (e-book, 2006; research share 'USB HDD 2/tRADING KNO-2/
//! Mark Crisp - The Trapline Trading Method (Esignal & TS Code Included) (daytradeformoney.com)/TRAPLINE_TRADING_015.pdf', 95 PDF
//! pages, no printed page numbers: PDF pages are cited). Book id `athtt`.
//! Coded: the midpoint "who's in control" swing rule (ch.2 PDF 19, ch.4 PDF 22, ch.6 PDF 26-28) and the printed
//! "Trapline Statistics Filter #2" scan as a one-day trade (ch.5 PDF 24-25). Both trade at closes only.
//! Held: the open-price gap fade ($.10 offsets, quick $100 exits, intraday AAPL), the intraday exit list (ch.3: "price action
//! slowing or stalled", candle reversals, time of day: judgement), Appendix D stock screen (no exit).
//! Independent reference: authors/titles-1030/R2Cw/r2c_ref.py (written from the pages, not from this file).
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::tselhs::Book;

pub const REVIEWED: &[&str] = &["athtt_control", "athtt_mid_cross_1d"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE_CONTROL: &str = "[Adapted: 2 unstated details, R4 #297; unverified] Midpoint = (high + low) / 2 of a day. Buyers are in control at a close above BOTH the previous day's midpoint (the 'trapline') and the current day's midpoint; sellers at a close below both; otherwise no one is in control (PDF 19). 'You enter the trade with whoever is in control and you exit when they lose control' (PDF 22): flat, a close with buyers in control buys at that close, a close with sellers in control sells short at that close; a long is closed at the first close where the buyers are no longer in control (the book's 'conservative' choice in its AAPL 09/06/2006 example, PDF 27), and the same close opens a short if the sellers are in control. Unstated in the book: (1) which bar sizes - the book offers pairs (weekly midpoints to enter and daily to exit for swing traders, daily/60-minute and 1/5-minute for day traders) and says 'Some swing traders only look at the daily chart. The trapline results are the same' (PDF 28); vault reading: daily midpoints to enter and to exit. (2) the fill - the book enters 'at the trapline' during the day, but control needs the day's own midpoint, known only at the close; vault reading: decided and filled at the close. Option exit=1: hold until the other side takes control (the book's 'let it ride' choice, PDF 27). The book's AAPL tables (PDF 29-37) are 'potential' profit measured to each day's high or low, not trades, so there is nothing to verify against. A stock method applied to indices and ETFs, US and India, as a declared application.";

const NOTE_STAT: &str = "[Adapted: 2 unstated details, R4 #297; unverified] The book's 'TRAPLINE STATISTICS FILTER #2' (PDF 25) counts a day as a long trigger when the close 2 days ago was below the midpoint (high + low) / 2 of 3 days ago and the close 1 day ago was above the midpoint of 2 days ago, and counts a 'win' when the next close is above the trigger day's close ('the profit percentage when you enter the trade the day the price crosses above the midpoint trapline', PDF 24). Coded as a trade: buy at the close of the day that closes above the previous day's midpoint after a close below the midpoint before it; sell at the next close. Long only, as printed. Unstated in the book: (1) that the statistic is traded as a one-day hold - the book prints it as a scan column to choose stocks ('Trade stocks with the best win percent'); vault reading: entry at the trigger close, exit at the next close, exactly the scan's win test. (2) the scan's stock filters (30-day average daily point range > 1.00, volume and 5/30-day average volume > 1,000,000, NASDAQ only); vault reading: not applied, the rule runs on each instrument on its own. The printed table (23 NASDAQ stocks, last 100 days to an unstated 2006 date) cannot be dated, so there is no book comparison.";

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, group, dir, rules, notes| StrategyDef { id, name, group, session: 0, dir, status: Status::Live, needs_volume: false, needs_vix: false,
        needs_series: &[], book: "athtt", rules, notes, claim: None };
    vec![
        d("athtt_control", "Horton Trapline: trade with whoever is in control of both day midpoints; exit when they lose control", "Who's in control (ch.2, ch.4, ch.6)", "Long/Short",
          &["E|\"If the price is above both the previous and current days' midpoints the buyers are in control. If the price is below both the previous and current days' midpoints the sellers are in control. If the price is above one and below the other, no one is in control.\" (PDF 19); \"ALWAYS KNOW WHO IS IN CONTROL AND BE ON THAT SIDE OF THE TRADE.\" (PDF 19)",
            "X|\"But it is always the same. You enter the trade with whoever is in control and you exit when they lose control.\" (PDF 22); 09/06/2006: \"AAPL closed above the trapline $70.03 but is closed below the current day's midpoint $70.69. We can either: Close the entire position and take the profit - conservative ...\" (PDF 27)",
            "F|\"If you are a swing trader, then you might use weekly midpoints to enter and daily to exit the trade.\" (PDF 22); \"Some swing traders only look at the daily chart. The trapline results are the same.\" (PDF 28)"],
          NOTE_CONTROL),
        d("athtt_mid_cross_1d", "Horton Trapline Statistics Filter #2: buy the close that crosses above the previous day's midpoint, sell the next close", "Trapline statistics (ch.5)", "Long",
          &["E|\"Set{LgTrigger, count( close 2 days ago below MidPt 3 days ago, 1 ) * count( close 1 day ago above MidPt 2 day2 ago, 1 ) }\" with \"Set{MidPta, high + low } Set{MidPt, MidPta * .50 }\" (PDF 25)",
            "X|\"Lgwin100 is the number of 'wins' - the close was above the previous day's close.\" (PDF 25): the trade is closed at the next close",
            "F|\"Here's a list of stocks showing the profit percentage when you enter the trade the day the price crosses above the midpoint trapline\" (PDF 24)"],
          NOTE_STAT),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match id {
        "athtt_control" => Some(("athtt_control", "Horton Trapline who's in control", "daily")),
        "athtt_mid_cross_1d" => Some(("athtt_mid_cross_1d", "Horton Trapline midpoint cross, one day", "1 day")),
        _ => None,
    }
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    match id {
        "athtt_control" => Some(vec![ParamDef { key: "exit", label: "Exit: 0 = buyers/sellers lose control (book, conservative), 1 = only when the other side takes control", def: 0., min: 0., max: 1., step: 1. }]),
        "athtt_mid_cross_1d" => Some(vec![]),
        _ => None,
    }
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let mid: Vec<f64> = (0..x.r.len()).map(|i| (x.h[i] + x.l[i]) / 2.).collect();
    let mut pos = Book::new();
    if id == "athtt_mid_cross_1d" {
        for t in from.max(2)..=to {
            if pos.side() > 0. && pos.entry_bar().is_some_and(|e| t == e + 1) { pos.exit(t, x.c[t], "next close"); }
            if pos.side() == 0. && x.c[t - 1] < mid[t - 2] && x.c[t] > mid[t - 1] { pos.enter(1., t, x.c[t], "close"); }
        }
        return Some(pos.done(to, x.c[to]));
    }
    let ride = p.get("exit").copied().unwrap_or(0.) >= 0.5;
    for t in from.max(1)..=to {
        let ctl = if x.c[t] > mid[t - 1] && x.c[t] > mid[t] { 1. } else if x.c[t] < mid[t - 1] && x.c[t] < mid[t] { -1. } else { 0. };
        let side = pos.side();
        if side != 0. && ctl != side && (!ride || ctl == -side) { pos.exit(t, x.c[t], if ctl == 0. { "no one in control" } else { "other side in control" }); }
        if pos.side() == 0. && ctl != 0. { pos.enter(ctl, t, x.c[t], "close"); }
    }
    Some(pos.done(to, x.c[to]))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn control_trades_follow_both_midpoints() {
        let r = super::super::sctraub::tests::bars(2500);
        let x = Ctx::new(&r, &r, &[]);
        let m = |i: usize| (x.h[i] + x.l[i]) / 2.;
        let t = run("athtt_control", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(t.iter().any(|z| z.dir > 0.) && t.iter().any(|z| z.dir < 0.));
        for z in &t {
            let e = z.legs[0].i;
            assert!(z.dir * (x.c[e] - m(e - 1)) > 0. && z.dir * (x.c[e] - m(e)) > 0.);
            if !z.open { let k = z.exit_i; assert!(!(z.dir * (x.c[k] - m(k - 1)) > 0. && z.dir * (x.c[k] - m(k)) > 0.)); }
        }
        for w in t.windows(2) { assert!(w[1].legs[0].i >= w[0].exit_i); }
        let mut p = Params::new(); p.insert("exit".into(), 1.);
        assert!(run("athtt_control", &x, 0, r.len() - 1, &p).unwrap().len() <= t.len());
        let s = run("athtt_mid_cross_1d", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(!s.is_empty());
        for z in &s { let e = z.legs[0].i; assert!(z.dir > 0. && x.c[e - 1] < m(e - 2) && x.c[e] > m(e - 1) && (z.open || z.exit_i == e + 1)); }
    }
}

// Instrument admission: Horton Trapline daily application 2026-11-13 (lane R2C, R4 #297).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "athtt" && reviewed(def.id),
    label: "Avery Horton's trapline rules are shown on intraday and daily AAPL and NASDAQ stock charts; daily bars of indices, ETFs and stocks, US and India, are a declared application, with two unstated details read as labelled on each card (R4 #297).",
    basis: "Trapline Trading Strategy (e-book, 2006) PDF 19-28; R4 #297; independent check authors/titles-1030/R2Cw/r2c_ref.py",
};
