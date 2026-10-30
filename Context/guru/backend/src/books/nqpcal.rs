//! NasdaqProfits.com (author signs as 'Steve'), "Nasdaq Profits" ebook (web pages printed 5/28/2008; research share USB HDD
//! 2/tRADING KNO-2/Nasdaq Profits Ebook (nasdaqprofits.com)/NasdaqProfits.pdf, text layer). Book id `nqpcal`. One rule:
//! the calendar-date table (PDF p.2-11) traded on the dates with a printed 60% or better up/down rate (p.2), in at the
//! open and out at the close (futures route, p.12). Lane K3 (titles loop, 30 Oct 2026); independent check
//! authors/nqprofits/admission/k3n_ref.py (header).
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

/// Admitted ids (authors/nqprofits/admission/k3n_ref.py (header)).
pub const REVIEWED: &[&str] = &["nqpcal_60"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

/// The printed table, literally (PDF p.2-11; stats through 12/31/2007): (month, day, printed %, printed direction
/// 1 = UP, -1 = DOWN, 0 = UP/DOWN). The three HOLIDAY dates (Jan 1, Jul 4, Dec 25) have no row. Printed labels are kept
/// even where the counts disagree (Feb 7 '52% UP' 13/14, May 14 '54% UP' 12/14, Aug 10 '59% UP' 11/16 - none reaches 60%).
pub const TABLE: &[(u8, u8, u8, i8)] = &[
    (1,2,62,1), (1,3,55,1), (1,4,54,1), (1,5,65,1), (1,6,76,1), (1,7,68,1), (1,8,56,-1), (1,9,58,1), (1,10,70,1), (1,11,54,1), (1,12,54,1), (1,13,56,1),
    (1,14,84,1), (1,15,70,1), (1,16,62,1), (1,17,58,1), (1,18,68,1), (1,19,60,1), (1,20,50,0), (1,21,54,-1), (1,22,64,-1), (1,23,58,1), (1,24,56,1), (1,25,52,-1),
    (1,26,50,0), (1,27,56,1), (1,28,72,1), (1,29,64,1), (1,30,58,1), (1,31,67,1), (2,1,65,1), (2,2,69,1), (2,3,68,1), (2,4,60,1), (2,5,52,1), (2,6,69,1),
    (2,7,52,1), (2,8,59,1), (2,9,59,-1), (2,10,65,1), (2,11,54,1), (2,12,58,1), (2,13,54,-1), (2,14,67,1), (2,15,77,1), (2,16,55,1), (2,17,57,-1), (2,18,52,-1),
    (2,19,65,-1), (2,20,62,1), (2,21,67,-1), (2,22,55,-1), (2,23,55,1), (2,24,58,1), (2,25,69,1), (2,26,54,1), (2,27,58,-1), (2,28,52,1), (2,29,83,1), (3,1,70,1),
    (3,2,63,1), (3,3,58,1), (3,4,64,1), (3,5,58,1), (3,6,54,1), (3,7,52,1), (3,8,64,1), (3,9,52,-1), (3,10,65,1), (3,11,56,1), (3,12,58,-1), (3,13,69,1),
    (3,14,56,1), (3,15,61,1), (3,16,70,1), (3,17,50,0), (3,18,64,1), (3,19,50,0), (3,20,50,0), (3,21,67,-1), (3,22,57,-1), (3,23,59,1), (3,24,50,0), (3,25,54,-1),
    (3,26,58,1), (3,27,50,0), (3,28,50,0), (3,29,56,1), (3,30,52,1), (3,31,64,1), (4,1,59,1), (4,2,54,1), (4,3,54,1), (4,4,65,-1), (4,5,72,1), (4,6,54,1),
    (4,7,62,1), (4,8,58,1), (4,9,55,1), (4,10,64,1), (4,11,52,1), (4,12,81,1), (4,13,50,0), (4,14,58,1), (4,15,64,1), (4,16,56,1), (4,17,57,1), (4,18,58,1),
    (4,19,54,-1), (4,20,56,-1), (4,21,64,1), (4,22,60,1), (4,23,54,-1), (4,24,54,-1), (4,25,56,1), (4,26,64,1), (4,27,58,1), (4,28,65,1), (4,29,64,1), (4,30,64,1),
    (5,1,58,1), (5,2,78,1), (5,3,57,1), (5,4,78,1), (5,5,72,1), (5,6,56,-1), (5,7,50,0), (5,8,62,1), (5,9,52,-1), (5,10,54,-1), (5,11,56,1), (5,12,58,1),
    (5,13,60,1), (5,14,54,1), (5,15,62,1), (5,16,59,1), (5,17,50,0), (5,18,56,1), (5,19,52,-1), (5,20,56,-1), (5,21,58,-1), (5,22,65,1), (5,23,56,1), (5,24,54,1),
    (5,25,52,1), (5,26,67,1), (5,27,70,1), (5,28,70,1), (5,29,60,-1), (5,30,55,1), (5,31,68,1), (6,1,70,1), (6,2,65,1), (6,3,56,1), (6,4,69,1), (6,5,69,1),
    (6,6,56,1), (6,7,54,-1), (6,8,52,-1), (6,9,50,0), (6,10,56,1), (6,11,60,1), (6,12,50,0), (6,13,63,1), (6,14,54,1), (6,15,61,1), (6,16,65,1), (6,17,54,-1),
    (6,18,65,-1), (6,19,54,1), (6,20,56,-1), (6,21,57,1), (6,22,52,1), (6,23,50,0), (6,24,60,1), (6,25,58,-1), (6,26,54,-1), (6,27,59,1), (6,28,64,1), (6,29,59,1),
    (6,30,73,1), (7,1,56,1), (7,2,58,-1), (7,3,62,1), (7,5,55,-1), (7,6,56,-1), (7,7,54,1), (7,8,56,1), (7,9,88,1), (7,10,54,-1), (7,11,63,1), (7,12,61,1),
    (7,13,70,1), (7,14,76,1), (7,15,68,1), (7,16,50,0), (7,17,65,1), (7,18,63,-1), (7,19,54,1), (7,20,52,1), (7,21,62,-1), (7,22,56,-1), (7,23,54,-1), (7,24,54,-1),
    (7,25,67,1), (7,26,54,-1), (7,27,56,-1), (7,28,58,-1), (7,29,56,-1), (7,30,54,1), (7,31,62,1), (8,1,56,1), (8,2,61,1), (8,3,56,-1), (8,4,62,-1), (8,5,56,-1),
    (8,6,62,1), (8,7,65,1), (8,8,56,-1), (8,9,50,0), (8,10,59,1), (8,11,64,1), (8,12,52,-1), (8,13,54,1), (8,14,65,1), (8,15,56,1), (8,16,54,1), (8,17,81,1),
    (8,18,50,0), (8,19,72,-1), (8,20,62,1), (8,21,50,0), (8,22,65,1), (8,23,54,-1), (8,24,56,1), (8,25,54,1), (8,26,56,1), (8,27,50,0), (8,28,62,1), (8,29,67,1),
    (8,30,57,1), (8,31,74,1), (9,1,67,1), (9,2,75,1), (9,3,55,1), (9,4,50,0), (9,5,59,-1), (9,6,68,1), (9,7,61,1), (9,8,54,1), (9,9,56,1), (9,10,50,0),
    (9,11,72,1), (9,12,54,-1), (9,13,56,1), (9,14,62,1), (9,15,62,-1), (9,16,52,1), (9,17,54,-1), (9,18,50,0), (9,19,56,1), (9,20,61,1), (9,21,63,-1), (9,22,54,1),
    (9,23,52,-1), (9,24,54,1), (9,25,62,-1), (9,26,56,1), (9,27,59,-1), (9,28,59,1), (9,29,54,-1), (9,30,56,1), (10,1,58,1), (10,2,65,1), (10,3,52,-1), (10,4,54,1),
    (10,5,63,1), (10,6,65,1), (10,7,60,1), (10,8,58,1), (10,9,54,1), (10,10,59,1), (10,11,57,1), (10,12,59,1), (10,13,69,1), (10,14,56,1), (10,15,58,1), (10,16,58,-1),
    (10,17,56,-1), (10,18,68,1), (10,19,52,-1), (10,20,56,1), (10,21,60,1), (10,22,58,1), (10,23,58,-1), (10,24,63,-1), (10,25,68,-1), (10,26,56,-1), (10,27,58,-1), (10,28,68,1),
    (10,29,62,1), (10,30,50,0), (10,31,67,1), (11,1,57,1), (11,2,62,1), (11,3,62,1), (11,4,63,1), (11,5,81,1), (11,6,58,1), (11,7,62,-1), (11,8,54,1), (11,9,63,1),
    (11,10,54,1), (11,11,72,1), (11,12,62,1), (11,13,58,-1), (11,14,59,1), (11,15,61,-1), (11,16,52,1), (11,17,50,0), (11,18,52,1), (11,19,50,0), (11,20,54,1), (11,21,56,1),
    (11,22,68,1), (11,23,57,1), (11,24,81,1), (11,25,74,1), (11,26,64,1), (11,27,57,1), (11,28,64,1), (11,29,57,1), (11,30,56,1), (12,1,73,1), (12,2,56,1), (12,3,54,1),
    (12,4,65,1), (12,5,56,1), (12,6,57,1), (12,7,56,-1), (12,8,62,1), (12,9,56,-1), (12,10,50,0), (12,11,58,-1), (12,12,52,-1), (12,13,52,-1), (12,14,63,-1), (12,15,52,-1),
    (12,16,60,1), (12,17,54,1), (12,18,58,1), (12,19,59,-1), (12,20,54,-1), (12,21,70,1), (12,22,73,1), (12,23,72,1), (12,24,65,1), (12,26,77,1), (12,27,59,1), (12,28,62,1),
    (12,29,65,1), (12,30,68,1), (12,31,81,1),
];

const NOTE: &str = "[Declared readings] The ebook's stats are for the Nasdaq Composite, which the vault does not hold: the Nasdaq-100 (NDX), QQQ, other indices, ETFs, single stocks and India are a declared application. Each trading day whose calendar date has a printed rate of 60% or more is traded in the printed direction, in at that day's open and out at its close (p.12 futures route: 'enter the market at open', 'exit the trade at the close'); dates under 60% and dates with no session (weekends, holidays) are not traded. The table is used exactly as printed (no recomputation from its counts). The 20-25 point futures stop (p.12) is not applied (Nasdaq points; the author's preferred BetOnMarkets route has no stop). The 9.40-10.10 intraday 'bonus strategy' (p.14-16) needs intraday data and is not coded.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "nqpcal_60", name: "Nasdaq calendar days: trade the printed 60%+ up/down dates from the open to the close", group: "NasdaqProfits.com calendar stats",
        session: 0, dir: "Long/Short", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: "nqpcal", rules: &[
            "F|\"The following are the days that have a 60% or better up/down rate.\" (p.2); table of 363 dated rows, p.2-11",
            "E|\"So the play on January 2nd would be to go LONG on that day ... So the trade for this day would be a SELL/SHORT.\" (p.2) = UP dates bought, DOWN dates sold short",
            "E|\"On UP days you would enter the market at open\" (p.12)",
            "X|\"you can exit the trade at the close\" (p.12) = out at the same day's close"],
        notes: NOTE, claim: None }]
}
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    (id == "nqpcal_60").then_some(("nqpcal_60", "NasdaqProfits calendar days", "60%+ dates, open to close"))
}
pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    (id == "nqpcal_60").then(|| vec![ParamDef { key: "min_pct", label: "Trade dates with a printed rate of at least, % (ebook 60)", def: 60., min: 51., max: 84., step: 1. }])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let min = p.get("min_pct").copied().unwrap_or(60.);
    let mut out = vec![];
    for i in from..=to {
        let d = &x.r[i].date;
        let (m, day) = (d[5..7].parse::<u8>().ok()?, d[8..10].parse::<u8>().ok()?);
        let Some(&(_, _, pct, dir)) = TABLE.iter().find(|r| r.0 == m && r.1 == day) else { continue };
        if dir == 0 || (pct as f64) < min { continue; }
        out.push(Trade { dir: dir as f64, legs: vec![Leg { i, px: x.o[i], w: 1. }], exit_i: i, exit_px: x.c[i], reason: "day's close".into(), entry_kind: "open", signal_i: i, open: false, exits: vec![] });
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn table_is_literal_and_trades_60_percent_dates() {
        assert_eq!(TABLE.len(), 363);
        assert_eq!(TABLE.iter().filter(|r| r.2 >= 60 && r.3 != 0).count(), 149);
        assert_eq!(TABLE.iter().find(|r| (r.0, r.1) == (2, 19)), Some(&(2, 19, 65, -1))); // p.2 example: DOWN 65%
        let r: Vec<Row> = ["2008-01-02", "2008-01-03", "2008-02-19"].iter().map(|d| Row { ticker: "T".into(), date: d.to_string(), open: 10., high: 12., low: 9., close: 11., volume: None }).collect();
        let x = Ctx::new(&r, &r, &[]);
        let t = run("nqpcal_60", &x, 0, 2, &Params::new()).unwrap();
        assert_eq!(t.iter().map(|z| (z.dir, z.legs[0].i, z.exit_i)).collect::<Vec<_>>(), vec![(1., 0, 0), (-1., 2, 2)]);
    }
}
