//! Steve Moore (Moore Research Center), interview "Speaking of Seasonals" (Thom Hartle), Technical Analysis of Stocks & Commodities
//! V.12:10 (Oct 1994) pp.420-429 (research share Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/
//! VOLUMES/V12/C10/MOORE.PDF, Figures 2, 4 and 6 on PDF pp.6-8). Book id `scmoore`. Lane C6 (second pass, 17 Nov 2026);
//! independent check authors/convert-1117/C6/ref/c6_fut_ref.py. Three printed seasonal trades, each run on the Norgate continuous
//! contract of the printed market only (futures::CARDS_C6, R4 #293 (1)).
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::tselhs::Book;

pub const REVIEWED: &[&str] = &["scmoore_crude", "scmoore_sp", "scmoore_zb"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

macro_rules! note { ($m:expr, $stop:expr, $pv:expr) => { concat!("[Declared readings, 3 unstated details, R4 #339; unverified unless the book check on the card says otherwise] ", $m, " Printed: the trade table (every year's buy or sell date, exit date and prices) and the summary line 'Protective stop (", $stop, ")'. Readings: (1) 'approximately' dates: the entry is the close of the first trading day on or after the printed date and the exit the close of the last trading day on or before the printed exit date (this is what the printed tables show every year, e.g. crude 10/16/91 -> 10/25/91 because 10/27/91 was a Sunday); (2) the stop is the printed US$ amount per contract (the engine's point value in force on the entry date, ", $pv, "), taken from the entry price, checked from the bar after entry, filled at the stop price or at the open if the bar opens beyond it; the exit bar's stop is tested before its close; (3) no re-entry after a stop within the same year. Runs on the Norgate continuous contract (the book's trades are in the December contract), signals and point P&L on the back-adjusted series.") }; }
const NOTE_CL: &str = note!("Moore's seasonal for December crude oil: sell on approximately 10/16, exit on approximately 10/27.", "1250", "crude $1,000 per $1: stop 1.25");
const NOTE_SP: &str = note!("Moore's seasonal for December S&P 500: buy on approximately 10/27, exit on approximately 11/02.", "2600", "S&P $500 per point to 1997-11-02, $250 after: stop 5.2 points, 10.4 from 1997-11-03");
const NOTE_ZB: &str = note!("Moore's seasonal for December 30-year T-bonds: buy on approximately 10/28, exit on approximately 11/12.", "2600", "T-bonds $1,000 per point: stop 2.6");

fn card(id: &'static str, name: &'static str, dir: &'static str, rules: &'static [&'static str], notes: &'static str) -> StrategyDef {
    StrategyDef { id, name, group: "S&C: Speaking of Seasonals (Moore Research, V.12:10)", session: 0, dir, status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: "scmoore", rules, notes, claim: None }
}

pub fn catalogue() -> Vec<StrategyDef> {
    vec![
        card("scmoore_crude", "Moore seasonal, crude oil: sell the close on or after 10/16, buy back the last close on or before 10/27, $1.25 protective stop", "Short", &[
            "E|\"CRUDE OIL(NYM)-DECEMBER Sell on approximately 10/16; exit on approximately 10/27\" (V.12:10 Figure 2, PDF p.6)",
            "X|\"exit on approximately 10/27\"; \"Protective stop (1250)\" (Figure 2 trade summary)",
            "F|Figure 2 trade summary: winners 10, losers 1, total trades 11 (1983-1993), percentage correct 91, average net profit per trade $730.91"], NOTE_CL),
        card("scmoore_sp", "Moore seasonal, S&P 500: buy the close on or after 10/27, sell the last close on or before 11/02, 5.2-point protective stop", "Long", &[
            "E|\"S&P 500(CME)-December Buy on approximately 10/27; exit on approximately 11/02\" (Figure 4, PDF p.7)",
            "X|\"exit on approximately 11/02\"; \"Protective stop (2600)\" (Figure 4 trade summary)",
            "F|Figure 4 trade summary: winners 11, losers 1, total trades 12, percentage correct 92, average net profit per trade $2,695.83"], NOTE_SP),
        card("scmoore_zb", "Moore seasonal, T-bonds: buy the close on or after 10/28, sell the last close on or before 11/12, 2.6-point protective stop", "Long", &[
            "E|\"30-YEAR T-BONDS (CBT)-DECEMBER Buy on approximately 10/28 - Exit on approximately 11/12\" (Figure 6, PDF p.8)",
            "X|\"Exit on approximately 11/12\"; \"Protective stop (2600)\" (Figure 6 trade summary)",
            "F|Figure 6 trade summary: winners 13, losers 2, total trades 15, percentage correct 87, average net profit per trade $1,602.08"], NOTE_ZB),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match id {
        "scmoore_crude" => Some(("scmoore_crude", "Moore crude oil seasonal (S&C)", "10/16-10/27 short")),
        "scmoore_sp" => Some(("scmoore_sp", "Moore S&P 500 seasonal (S&C)", "10/27-11/02 long")),
        "scmoore_zb" => Some(("scmoore_zb", "Moore T-bond seasonal (S&C)", "10/28-11/12 long")),
        _ => None,
    }
}
pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    reviewed(id).then(|| vec![p("stop", "Protective stop in US$ per contract (0 = none; printed: crude 1250, S&P 2600, T-bonds 2600)", default_stop(id), 0., 10000., 50.)])
}
fn default_stop(id: &str) -> f64 { match id { "scmoore_crude" => 1250., _ => 2600. } }

/// (direction, entry month-day, exit month-day)
fn spec(id: &str) -> (f64, &'static str, &'static str) {
    match id { "scmoore_crude" => (-1., "10-16", "10-27"), "scmoore_sp" => (1., "10-27", "11-02"), _ => (1., "10-28", "11-12") }
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from > to || to >= x.r.len() { return None; }
    let stop = p.get("stop").copied().unwrap_or(default_stop(id));
    let (d, ent, ex) = spec(id);
    let n = x.r.len();
    let mut pos = Book::new();
    let mut y0 = x.r[0].date[0..4].parse::<i32>().unwrap_or(0);
    let y1 = x.r[n - 1].date[0..4].parse::<i32>().unwrap_or(0);
    while y0 <= y1 {
        let (ed, xd) = (format!("{y0}-{ent}"), format!("{y0}-{ex}"));
        y0 += 1;
        let Some(e) = (0..n).find(|&i| x.r[i].date >= ed) else { continue };
        let Some(q) = (0..n).rev().find(|&i| x.r[i].date <= xd) else { continue };
        if x.r[e].date > xd || q <= e || e < from || e >= to { continue; }
        let sp = if stop > 0. { match crate::futures::usd_points(x, e, stop) { Some(v) => v, None => continue } } else { 0. };
        pos.enter(d, e, x.c[e], "close");
        let level = x.c[e] - d * sp;
        let mut out = None;
        if sp > 0. {
            for j in e + 1..=q.min(to) {
                let hit = if d > 0. { x.l[j] <= level } else { x.h[j] >= level };
                if hit { let gap = if d > 0. { x.o[j] <= level } else { x.o[j] >= level }; out = Some((j, if gap { x.o[j] } else { level })); break; }
            }
        }
        match out {
            Some((j, px)) => pos.exit(j, px, "protective stop"),
            None if q <= to => pos.exit(q, x.c[q], "seasonal exit date"),
            None => break,
        }
    }
    Some(pos.done(to, x.c[to]))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn seasonal_windows_enter_after_and_exit_before_the_printed_dates() {
        let r = super::super::sctraub::tests::bars(2600);
        let x = Ctx::new(&r, &r, &[]);
        for id in REVIEWED {
            let mut p = Params::new(); p.insert("stop".into(), 0.); // the US$ stop needs a futures context (futures::usd_points)
            let t = run(id, &x, 0, r.len() - 1, &p).unwrap();
            let (d, ent, ex) = spec(id);
            assert!(t.len() >= 8, "{id}");
            for z in &t {
                let (e, q) = (&r[z.legs[0].i].date, &r[z.exit_i].date);
                assert!(z.dir == d && e[5..] >= *ent && q[5..] <= *ex || z.reason == "protective stop", "{id} {e} {q}");
            }
        }
    }
}
