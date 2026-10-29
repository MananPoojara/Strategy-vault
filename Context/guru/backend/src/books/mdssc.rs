//! Michael D. Sheimo, "Trading The Santa Claus Rally", Technical Analysis of Stocks & Commodities V.10:12 (Dec 1992) pp.517-520
//! (book id `mdssc`). Reading: claude-continuation-20261020/authors/lanec3 (text layer, read in full); independent reference
//! authors/lanec3/ref/lanec3_ref.py. Admitted: the fixed holiday window (buy the close of the day after Thanksgiving, sell the
//! close of the year's last trading day). The article's stock picks (five Dow stocks chosen by preference) are not a rule.
use crate::books::lc3::Pos;
use crate::books::lwicw::{holiday, Hol};
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const REVIEWED: &[&str] = &["mdssc_santa"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "mdssc_santa", name: "Santa Claus rally: buy the close of the day after Thanksgiving, sell the close of the year's last trading day", group: "Trading the Santa Claus Rally (S&C Dec 1992)",
        session: 0, dir: "Long", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: "mdssc",
        rules: &["E|\"We will buy the closing price for the day after Thanksgiving and hold the stock until the end of the last trading day of the year\" (p.518, article page 2)",
            "X|\"The period will be the closing price on the day after Thanksgiving (for the buy) until the closing price for the year (for the sell)\" (p.518-519, article page 3)",
            "F|\"in reality, any list of stocks can be used, as long as the group shows some history of a rally between Thanksgiving and Christmas\" (p.519)"],
        notes: "Thanksgiving = fourth Thursday of November (US calendar). 'The day after Thanksgiving' = the first trading day dated after it (normally the Friday). Exit at the close of the last bar dated in the same calendar year; a window that ends before the year's last trading day leaves the trade open. The article bought 500 shares each of five Dow stocks (Coca-Cola, Disney, Exxon, GE, IBM) in 1989-1991 with $150 commission per side; here one unit per trade, no costs, on any US index, ETF or stock (index/ETF = declared application). US only: the window is set by a US exchange holiday.",
        claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match id { "mdssc_santa" => Some(("mdssc_santa", "Sheimo Santa Claus rally", "Day after Thanksgiving to year end")), _ => None }
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    match id { "mdssc_santa" => Some(vec![]), _ => None }
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, _p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let c = &x.c;
    let mut pos = Pos::new();
    let mut tg = String::new();
    for i in from..=to {
        let d = &x.r[i].date;
        let y: i32 = d[0..4].parse().unwrap_or(0);
        if tg.get(0..4) != Some(&d[0..4]) { tg = holiday(Hol::Thanksgiving, y); }
        let first_after = d.as_str() > tg.as_str() && (i == 0 || x.r[i - 1].date.as_str() <= tg.as_str());
        let year_end = i + 1 < x.r.len() && x.r[i + 1].date[0..4] != d[0..4];
        if pos.side() > 0. && year_end { pos.exit(i, c[i], "last trading day of the year"); continue; }
        if pos.side() == 0. && first_after && !year_end { pos.enter(1., i, c[i], "close"); }
    }
    Some(pos.done(to, c[to]))
}
