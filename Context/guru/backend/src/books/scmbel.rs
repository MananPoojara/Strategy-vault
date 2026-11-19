//! Matt Blackman, "Elections As The Most Powerful Short-Term Cycle" (Working Money), Technical Analysis of Stocks & Commodities
//! V.26:2 (2008) pp.46-51 (research share Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/
//! V26/C02/027BLAK.pdf). Book id `scmbel`. Lane PA2 (13 Nov 2026); independent check authors/pa2/w/pa2_ref.py.
//! Admission: sclang.rs (PA2_BOOKS).
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::tselhs::Book;

pub const REVIEWED: &[&str] = &["scmbel_cycle"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Calendar rule, no unstated detail; unverified] 'Our hypothetical election trader and buy & hold investor both bought all Dow 30 stocks on the last trading day of September 1994. Our election trader exited on the last trading day of December 1996': long from the close of the last trading day of September of each US midterm year (1994, 1998, 2002, ...) to the close of the last trading day of December of the following presidential election year (27 months), in cash for the 21 months in between (January of the post-election year to September of the midterm year). The last trading day of a month is the bar whose next bar falls in another month; a position still open at the end of the data is shown open. The article holds the 30 Dow stocks in equal dollar amounts with commissions (1994-2006: the election trader at risk 56% of the days); the vault runs the calendar on one instrument - the Dow index or its ETF, and other indices, ETFs and stocks, US and India - as a declared application. Three complete cycles in the article's span, so it stays UNVERIFIED (too few trades for #168).";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "scmbel_cycle", name: "Blackman election cycle: long from the last September close of each midterm year to the last December close of the election year", group: "S&C: elections as the most powerful short-term cycle (Blackman, V.26:2)", session: 0, dir: "Long", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "scmbel", rules: &[
            "E|\"Our hypothetical election trader and buy & hold investor both bought all Dow 30 stocks on the last trading day of September 1994.\" (V.26:2, PDF p.2)",
            "X|\"Our election trader exited on the last trading day of December 1996\" (PDF p.2); \"While our election trader was out of the market over the next 22 months post-election from January 2, 1997, to September 28, 1998\" (PDF p.2)",
            "F|\"BEST TWO YEARS OF THE ELECTION CYCLE\" (PDF p.2): 'While the buy & hold investor had his money in the market and exposed to risk for all 4,383 trading days in 13 years, the election trader was at risk for 2,463 days or 56% of the time.'"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { reviewed(id).then_some(("scmbel_cycle", "Blackman election cycle (S&C)", "Sep midterm - Dec election")) }
pub fn params(id: &str) -> Option<Vec<ParamDef>> { reviewed(id).then(Vec::new) }

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from > to || to >= x.r.len() { let _ = p; return None; }
    let n = x.r.len();
    let ym = |i: usize| -> (i64, u32) { let d = &x.r[i].date; (d[0..4].parse().unwrap_or(0), d[5..7].parse().unwrap_or(0)) };
    let month_end = |i: usize| i + 1 < n && ym(i + 1).1 != ym(i).1;
    let mut pos = Book::new();
    for i in from..=to {
        let (y, m) = ym(i);
        if pos.side() == 0. && m == 9 && y.rem_euclid(4) == 2 && month_end(i) && i < to { pos.enter(1., i, x.c[i], "close"); continue; }
        if pos.side() > 0. && m == 12 && y.rem_euclid(4) == 0 && month_end(i) { pos.exit(i, x.c[i], "last close of December of the election year"); }
    }
    Some(pos.done(to, x.c[to]))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn midterm_september_to_election_december() {
        let r = super::super::sctraub::tests::bars(2600); // 2001-2010
        let x = Ctx::new(&r, &r, &[]);
        let t = run("scmbel_cycle", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert_eq!(t.len(), 3); // Sep 2002 - Dec 2004, Sep 2006 - Dec 2008, Sep 2010 - open
        assert_eq!(&r[t[0].legs[0].i].date[..7], "2002-09"); assert_eq!(&r[t[0].exit_i].date[..7], "2004-12");
        assert_eq!(&r[t[1].legs[0].i].date[..7], "2006-09"); assert!(!t[1].open && t[2].open);
    }
}
