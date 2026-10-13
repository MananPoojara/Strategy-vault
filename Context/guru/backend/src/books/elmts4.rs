//! Elm Trader, *The Dow TS4 System* guide (elmtrader.co.uk, (c) 2005, 27 pages, text layer).
//! Source copy: Books/Elm Trader/The Dow TS4 System by ElmTrader (elmtrader.co.uk).pdf. Notes: library/elmts4/README.md.
//! Page refs are PDF pages. One mechanical rule: buy the Dow at the close of the last trading day of each month
//! except July (no trade for the first trading day of August), sell at the close of the first trading day of the
//! next month, with a 150-point stop. Defaults are the guide's literal numbers.
use crate::connors::{claim, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

const BOOK: &str = "elmts4";

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, dir, status, rules, notes, claim| StrategyDef {
        id, name, group: "Dow TS4 System (monthly)", session: 0, dir, status, needs_volume: false, needs_vix: false,
        needs_series: &[], book: BOOK, rules, notes, claim,
    };
    vec![
        d("elmts4_first_day", "Dow TS4: long over the first trading day of each month (no August)", "Long", Status::Live, &[
            "F|\"the most successful day is the first trading day of each month ... August is weak and no position should be opened for that month. Thus no trade is entered for the first trading day of August.\" (pdf p.11)",
            "E|\"Trades should be opened at the end of the trading day prior to the first working day of the month to which they relate ... at the close of business on this day that the new position should be opened.\" (pdf p.11-12)",
            "X|\"the position is closed at the end of the day\" [the first trading day of the month] (pdf p.12; example 28 Nov 2003 close -> next day's close, p.12)",
            "X|\"a stop loss should be used of 150 points ... set at 150 points below the opening level.\" (pdf p.15)",
        ], concat!(
            "Instrument: the guide's research is on the Dow cash index (p.12; trades are placed in Dow futures / spread bets); here DJI daily bars. ",
            "Entry at the close of the bar before the month's first bar; exit at the close of the month's first bar. ",
            "Stop: 150 index points below the entry close, live during the first trading day; a low at or below it fills at the stop, or at the open if the day opens through it [book: a 'guaranteed' spread-bet stop that caps the loss at exactly 150 even on a gap (p.15); a cash index has no such fill, so a gap fills at the open; the published table shows -150 on 3 trades]. ",
            "Optional p.janfeb=1 skips the January and February trades (\"Traders may wish to ... resist trading during January and February\", p.17) [non-default]. ",
            "Staking (+£1 after a loss, back to £1 after a win, p.16) is money management and is not applied (one unit per trade). ",
            "India: the same calendar rule on NIFTY 50 with a literal 150-point stop (instrument swap)."),
            claim("49", "63% (31/49)", "+38.1 Dow points (total +1,865)", "1 day", "Jan 2001-May 2005, Dow futures (Appendix B, p.26-27)")),
        d("elmts4_fixed_odds", "Dow TS4 fixed-odds 'Intraday Up Bet' on the first trading day", "Long",
            Status::NotTestable("a binary fixed-odds bet: the payout depends on the bookmaker's quoted price, and the time of day the bet is placed on the first trading day (its strike level) is not stated (p.20-22)"), &[
            "E|\"the date of opening a new bet ... would be on the first trading day of the month and not the last trading day\" (pdf p.21)",
            "X|\"Fixed-odds bets ... expire automatically at the selected expiry date i.e. at the end of the day.\" (pdf p.22)",
        ], "Needs fixed-odds bet prices (BetOnMarkets quotes); not an index trade.", claim("", "65%", "", "1 day", "\"historic performance\" (p.22)")),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match id {
        "elmts4_first_day" => Some(("elmts4_ts4", "Dow TS4 System", "Spread bet · close-to-close, 150-pt stop")),
        "elmts4_fixed_odds" => Some(("elmts4_ts4", "Dow TS4 System", "Fixed-odds bet")),
        _ => None,
    }
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    match id {
        "elmts4_first_day" => Some(vec![
            p("stop_pts", "Stop loss below the entry close (index points)", 150., 10., 1000., 10.),
            p("skip_aug", "Skip the August trade (1 = book)", 1., 0., 1., 1.),
            p("janfeb", "Also skip January and February (optional, p.17)", 0., 0., 1., 1.),
        ]),
        "elmts4_fixed_odds" => Some(vec![]),
        _ => None,
    }
}

fn month(x: &Ctx, i: usize) -> u32 { x.r[i].date[5..7].parse().unwrap_or(0) }

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if id != "elmts4_first_day" { return None; }
    let schema = params(id)?;
    if from > to || to >= x.r.len() { return None; }
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(0., |d| d.def));
    let (stop_pts, skip_aug, janfeb) = (g("stop_pts"), g("skip_aug") >= 0.5, g("janfeb") >= 0.5);
    let mut out = vec![];
    for i in (from + 1)..=to {
        let m = month(x, i);
        if m == month(x, i - 1) { continue; } // i = first trading day of month m
        if (skip_aug && m == 8) || (janfeb && (m == 1 || m == 2)) { continue; }
        let e = i - 1;
        let px = x.c[e];
        let stop = px - stop_pts;
        let (exit_px, reason) = if x.l[i] <= stop { (x.o[i].min(stop), "150-point stop") } else { (x.c[i], "close of the first trading day") };
        out.push(Trade { dir: 1., legs: vec![Leg { i: e, px, w: 1. }], exit_i: i, exit_px, reason: reason.into(),
            entry_kind: "close", signal_i: i, open: false, exits: vec![] });
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    fn row(date: &str, o: f64, h: f64, l: f64, c: f64) -> Row {
        Row { ticker: "T".into(), date: date.into(), open: o, high: h, low: l, close: c, volume: None }
    }
    #[test]
    fn enters_last_day_exits_first_day_skips_august_and_stops() {
        let r = vec![
            row("2003-06-27", 100., 100., 100., 1000.), row("2003-06-30", 1000., 1010., 990., 1000.),
            row("2003-07-01", 1000., 1060., 990., 1050.), // July trade: +50
            row("2003-07-31", 1050., 1050., 1050., 1050.),
            row("2003-08-01", 1050., 1100., 800., 900.),  // August: skipped
            row("2003-08-29", 900., 900., 900., 900.),
            row("2003-09-02", 880., 900., 700., 720.),    // stop 750, opens above -> fills 750
            row("2003-09-30", 720., 720., 720., 720.),
            row("2003-10-01", 500., 560., 490., 550.),    // gap through stop 570 -> open 500
        ];
        let x = Ctx::new(&r, &r, &[]);
        let ts = run("elmts4_first_day", &x, 0, r.len() - 1, &Params::new()).unwrap();
        let v: Vec<(usize, f64, usize, f64)> = ts.iter().map(|t| (t.legs[0].i, t.legs[0].px, t.exit_i, t.exit_px)).collect();
        assert_eq!(v, vec![(1, 1000., 2, 1050.), (5, 900., 6, 750.), (7, 720., 8, 500.)]);
        assert!(catalogue().iter().all(|d| d.id.starts_with("elmts4_") && family_of(d.id).is_some()));
    }
}
