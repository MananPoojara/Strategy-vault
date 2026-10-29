//! Anonymous, "24/7 Stock Indices and Futures Trading Strategy" (24_7_System.doc, Sections 1-5; eBay-sold, FTSE
//! spread-betting). Book id `ts247`. Source: research drive "USB HDD 2/tRADING KNO-2/24-7 System for Spread Betting &
//! Futures/24_7_System.doc" (+ the eBay listing .htm).
//! Section 3: on a day whose date is > 24th or < 7th be long from the previous close, stop 0.5% of the index below
//! the price, re-set at each close in the window; hold until the close of the last window day. Section 5 adds a
//! short side (date > 7th and < 25th) and a long-every-day-October-to-April variant, both with the 0.5% stop.
//! A day "in the window" is a trading day whose CALENDAR date qualifies; the entry is the close of the trading day
//! before it (the calendar of the next session is known at that close). After a stop-out inside the window the next
//! window day again means "long from the previous close", so the position is re-entered at that close.
use crate::connors::{claim, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "ts247";
/// Admitted ids (authors/lanea/elmtrader/decisions.json).
pub const REVIEWED_LANEA: &[&str] = &["ts247_long", "ts247_short", "ts247_oct_apr"];
pub fn reviewed_lanea(id: &str) -> bool { REVIEWED_LANEA.contains(&id) }

const NOTE: &str = "FTSE cash-index spread bet run on the index (instrument swap; percentage returns, one unit). The 0.5% stop is checked on every bar in the trade against the day's low (high for shorts): it fills at the stop, or at the open when the open is already through it. The stop is re-set to 0.5% from each close while the next trading day is still in the window. The book's statistics count every window day as a trade (890 trades in 9 years, ~one per window day) while its practical rule holds one position through the window; daily re-set stops make the two P&L-identical before costs. Book stats are net of a 2-point spread and carry; the vault has no FTSE data, so they cannot be reproduced.";

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, dir, rules, claim| StrategyDef { id, name, group: "24-7 System Sections 3 and 5", session: 0, dir, status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules, notes: NOTE, claim };
    vec![
        d("ts247_long", "24-7 long (date > 24th or < 7th), 0.5% stop", "Long", &[
            "E|S3: \"If todays date is greater than the 24th or the month, or less than the 7th of the month, we want to enter a 'Buy' position as close to the previous close as possible.\"",
            "X|S3: \"As soon as a trade is entered, place a stop 0.5% of the index value away from the current price.\" \"There are no target/take profit orders placed.\"",
            "X|S3: \"if today is the 27th June, we should have entered a long at the previous days close ie the 26th, and then hold this position until todays close ... we do not close this postion on the 27th, as we know that tomorrow is still in the trading window ... re-adjust our stop loss for the current trade, and recalculate it based upon the most recent close\"",
            "F|S1 (FTSE cash, Jan 1998 - Dec 2006, net of 2-point spread and carry): 890 trades, 401 winners, 45.1%, 7.2 points/trade, avg win 47, avg loss -26, PF 1.50, max DD 488 points, total 6,368 points",
        ], claim("890", "45.1%", "7.2 FTSE points (net)", "1 window day per trade", "Jan 1998 - Dec 2006, FTSE cash index")),
        d("ts247_short", "24-7 short side (date > 7th and < 25th), 0.5% stop", "Short", &[
            "E|S5: \"if todays date is greater than the 7th and less than the 25th of the month, we want to be short the market from the previous close.\"",
            "X|S5: \"Again, using a stop loss of 0.5% of the market.\" (stop re-set at each close as in S3; exit at the close of the last window day)",
            "F|S5: \"short side averages approx 6.5 pts per trade versus 7.2pts for the longs\"",
        ], claim("-", "-", "approx 6.5 FTSE points", "-", "FTSE cash index (period as S1)")),
        d("ts247_oct_apr", "24-7 variant: long every day October-April, 0.5% stop", "Long", &[
            "E|S5: \"another market tendency is for the market to rise strongly during the months of October-to april ... it is possible to adapt this system to simply be long every single day (with the 0.5% stop loss).\"",
            "X|0.5% stop re-set at each close (S3 mechanics); flat May-September (\"5 months of the year when you are not taking any trades\")",
            "F|S5: \"The results by doing this are somewhat similar to the standard 24-7 method\" (no numbers)",
        ], None),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "ts247_long" => ("ts247_24_7", "24-7 System", "Long 25th-6th (book core)"),
        "ts247_short" => ("ts247_24_7", "24-7 System", "Short 8th-24th (S5 tweak)"),
        "ts247_oct_apr" => ("ts247_24_7", "24-7 System", "Long Oct-Apr (S5 tweak)"),
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    let stop = p("stop_pct", "Stop, % of the index", 0.5, 0.25, 2.0, 0.25);
    Some(match id {
        "ts247_long" => vec![p("after_day", "Long when date > this day", 24., 20., 28., 1.), p("before_day", "... or date < this day", 7., 2., 10., 1.), stop],
        "ts247_short" => vec![p("after_day", "Short when date > this day", 7., 4., 12., 1.), p("before_day", "... and date < this day", 25., 18., 28., 1.), stop],
        "ts247_oct_apr" => vec![stop],
        _ => return None,
    })
}

fn ymd(date: &str) -> (u32, u32) {
    (date[5..7].parse().unwrap_or(0), date[8..10].parse().unwrap_or(0))
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(0.0, |d| d.def));
    let (a, b, stop_f) = (g("after_day"), g("before_day"), g("stop_pct") / 100.0);
    let dir = if id == "ts247_short" { -1.0 } else { 1.0 };
    let in_window = |i: usize| {
        let (m, d) = ymd(&x.r[i].date);
        let d = d as f64;
        match id {
            "ts247_long" => d > a || d < b,
            "ts247_short" => d > a && d < b,
            _ => matches!(m, 10 | 11 | 12 | 1 | 2 | 3 | 4),
        }
    };
    let to = to.min(x.c.len().saturating_sub(1));
    let mut out = Vec::new();
    let mut cur: Option<(Trade, f64)> = None; // trade, current stop
    for i in from..=to {
        if let Some((mut t, stop)) = cur.take() {
            let hit = if dir > 0.0 { x.l[i] <= stop } else { x.h[i] >= stop };
            if hit {
                t.exit_px = if dir > 0.0 { x.o[i].min(stop) } else { x.o[i].max(stop) };
                t.exit_i = i; t.signal_i = i - 1; t.reason = "0.5% stop".into();
                out.push(t);
            } else if i < to && in_window(i + 1) {
                cur = Some((t, x.c[i] * (1.0 - dir * stop_f)));
                continue;
            } else if i < to {
                t.exit_px = x.c[i]; t.exit_i = i; t.signal_i = i; t.reason = "Close of the last window day".into();
                out.push(t);
            } else {
                cur = Some((t, stop));
                continue;
            }
        }
        if i < to && in_window(i + 1) {
            let t = Trade { dir, legs: vec![Leg { i, px: x.c[i], w: 1.0 }], exit_i: i, exit_px: x.c[i], reason: String::new(),
                entry_kind: "close", signal_i: i, open: false, exits: vec![] };
            cur = Some((t, x.c[i] * (1.0 - dir * stop_f)));
        }
    }
    if let Some((mut t, _)) = cur {
        t.exit_i = to; t.exit_px = x.c[to]; t.signal_i = to; t.reason = "open".into(); t.open = true;
        out.push(t);
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
    fn holds_through_window_and_stops_then_reenters() {
        let r = vec![
            row("2020-01-23", 100., 101., 99., 100.), // next day 24 not in window
            row("2020-01-24", 100., 101., 99., 100.), // next day 27 in window -> buy close 100
            row("2020-01-27", 100., 102., 99.6, 101.), // low 99.6 > stop 99.5 -> hold, stop 100.495
            row("2020-01-28", 101., 101., 100., 100.8), // low 100 <= 100.495 -> stopped at 100.495; re-enter at 100.8
            row("2020-02-06", 101., 102., 100.9, 101.5), // next day 07 out -> exit close 101.5
            row("2020-02-07", 101., 102., 100., 101.),
        ];
        let x = Ctx::new(&r, &r, &[]);
        let ts = run("ts247_long", &x, 0, 5, &Params::new()).unwrap();
        assert_eq!(ts.len(), 2);
        assert_eq!((ts[0].legs[0].i, ts[0].exit_i), (1, 3));
        assert!((ts[0].exit_px - 100.495).abs() < 1e-9);
        assert_eq!((ts[1].legs[0].i, ts[1].legs[0].px, ts[1].exit_i, ts[1].exit_px), (3, 100.8, 4, 101.5));
    }
}
