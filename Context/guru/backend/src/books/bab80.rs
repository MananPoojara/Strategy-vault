//! Bruce Babcock, "The 80% Solution: S&P Systems" (Reality Based Trading Co., 1997). Book id `bab80`.
//! Source copy: Books/Bruce Babcock/Bruce Babcock - The 80% Solution S&P Systems.pdf (21 scanned pages; printed
//! page = PDF page - 1). Transcription and checks: library/bab80/.
//!
//! The book tested one S&P 500 futures contract (TradeStation, continuous back-adjusted data, $500 per index
//! point: "$3,000 ... is 600 points in the S&P", i.e. 6.00 index points, p.3). Dollar stops are converted at the
//! book's own $500/point (`pt_usd`); on other instruments the same POINT distance is used (instrument swap only).
//!
//! Daily-bar order for the entry day (stop entry + protective stop on one bar): TradeStation's default bar path,
//! O -> nearer extreme -> other extreme -> C, as in `lwlts`. A stop gapped through fills at the open.
use crate::connors::{claim, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "bab80";
/// Admitted ids (authors/babcock/laneb/SOURCE-CONTRACT.md).
pub const REVIEWED: &[&str] = &["bab80_novapr"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const SP: &str = "Book: one S&P 500 futures contract (and NYFE), TradeStation, continuous back-adjusted data, $100 per trade deducted (p.6). $3,000 = 6.00 index points at $500/point (p.3); the same points are used on any index (instrument swap only). Back-adjusted futures prices cannot be compared with cash index prices, so only trade dates and counts are comparable. SPX cash opens before ~1999 are often stale (equal to the prior close), which changes the open-based setup and stop-entry fills on SPX in the book's window.";

const DT_R: &[&str] = &[
    "F|\"These systems never trade from the short side\" (p.2)",
    "F|\"1. Yesterday's 11-day Momentum must be greater than the 11-day Momentum of two days ago\" (p.2); Momentum = \"the difference between today's close and the close n days ago\" (p.2): C[1]-C[12] > C[2]-C[13] (book example p.4: March 6 close minus February 19 close)",
    "F|\"2. Today's Opening price must be greater than the average of yesterday's High, Low and Close\" (p.2)",
    "E|\"place an order to enter on stop if the market reaches a point equal to yesterday's True Range added to yesterday's Close\" (p.3); True Range = greater of the prior close or the high minus the lesser of the prior close or the low (p.3). A gap open above the level fills at the open",
    "X|\"a money management stop of $3,000 ... Subtract 600 points from the system entry point and place your stop there. Do not subtract from your actual fill but rather the entry price calculated by the system\" (p.3): 6.00 points below the entry level, intraday, from the entry bar (the book's 5/28/87 and 8/21/92 trades are stopped on the entry day)",
    "X|\"If you are day trading, exit your position on the close\" (p.3)",
    "X|[intraday wave 2b] Default p.path=1: real SPX 1-minute bars decide the entry fill and whether the 6.00-point stop was reached after it (fill at the level, or the minute's open when it opened through it). Fill minute that also reaches the stop: filled then stopped, unless that minute opened under the stop, then stopped only if it closed under it [convention]. SPX only: the stop is the book's fixed S&P points (no conversion to SPY or NSE prices is invented); from 2014, where SPX daily opens are real, to 2026-08. p.path=0 = the old daily bar path [Not from the book].",
];
const ST_R: &[&str] = &[
    "F|\"These systems never trade from the short side\" (p.2)",
    "F|\"1. Yesterday's 11-day Momentum must be greater than the 11-day Momentum of two days ago\" (p.2); Momentum = \"the difference between today's close and the close n days ago\" (p.2): C[1]-C[12] > C[2]-C[13] (book example p.4: March 6 close minus February 19 close)",
    "F|\"2. Today's Opening price must be greater than the average of yesterday's High, Low and Close\" (p.2)",
    "E|\"place an order to enter on stop if the market reaches a point equal to yesterday's True Range added to yesterday's Close\" (p.3); True Range = greater of the prior close or the high minus the lesser of the prior close or the low (p.3). A gap open above the level fills at the open",
    "X|\"a money management stop of $3,000 ... Subtract 600 points from the system entry point and place your stop there. Do not subtract from your actual fill but rather the entry price calculated by the system\" (p.3): 6.00 points below the entry level, intraday, from the entry bar (the book's 5/28/87 and 8/21/92 trades are stopped on the entry day)",
    "X|\"hold the position until the first profitable Open ... check the Open each day. If exiting would create a profitable trade, exit on the Open. (Don't include slippage or commissions in the calculation.) If not, hold the position for another day with the same protective stop\" (p.4): from the bar after entry, exit at the open if open > entry fill",
    "F|One position at a time; a new setup may fill on the day of a first-profitable-open exit (book 11/1/82 -> 11/2/82)"
];

const CAL: &str = "E|\"My system buys futures on the opening of the Monday of the first full week in November\" (p.16): open of the first trading day on or after the first Monday of November (the first week whose Monday is in November is the first full week)";
const CAL_X: &str = "X|\"holds (if not stopped out) until the Friday of the last complete week in April\" (p.16): exit on the close of the last Friday of April (last trading day of that week if Friday is a holiday). Read: close, since the test used weekly bars (p.16) whose last price is Friday's close; SPX check: 1983-04-29 close 164.43 vs book 164.65";
const CAL_N: &str = "Book tested S&P futures 'weekly continuation contracts' Nov 1982-Apr 1997, no costs (p.16-19). Table prices are actual futures prices (1982 entry 135.00), so they are comparable with SPX within the futures premium. Stopped trades print exactly the stop loss, so the book filled stops at the stop price on weekly bars; here a daily gap through the stop fills at the open (TradeStation convention). Run on any index; same points.";

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, group, rules, notes, claim| StrategyDef {
        id, name, group, session: 0, dir: "Long", status: Status::Live, needs_volume: false, needs_vix: false,
        needs_series: &[], book: BOOK, rules, notes, claim,
    };
    vec![
        d("bab80_daytrade", "80% Solution day trading system", "Day trading system (p.2-3)",
          DT_R,
          SP, claim("236", "59%", "$236 (after $100)", "same day", "S&P 7/1/82-6/27/97 (p.10)")),
        d("bab80_shortterm", "80% Solution short-term system", "Short-term system (p.4)",
          ST_R,
          SP, claim("227", "82%", "$412 (after $100)", "1-2 days", "S&P 7/1/82-6/27/97 (p.10-15)")),
        d("bab80_novapr", "Stock index calendar system Nov-Apr · no stop", "Long-term system (p.16-17)",
          &[CAL, CAL_X, "X|No stop-loss (first results page, p.17)"],
          CAL_N, claim("15", "80% (12 of 15)", "$13,933/yr ($209,000 total, no costs)", "~6 months", "S&P futures Nov 1982-Apr 1997 (p.17)")),
        d("bab80_novapr_stop10k", "Stock index calendar system Nov-Apr · $10,000 stop", "Long-term system (p.16, 18)",
          &[CAL, CAL_X, "X|\"The second page shows the system with a $10,000 stop\" (p.16): 20.00 points below the entry, intraday"],
          CAL_N, claim("15", "73% (11 of 15)", "$13,268/yr ($199,025 total)", "~6 months", "S&P futures Nov 1982-Apr 1997 (p.18)")),
        d("bab80_novapr_stop5k", "Stock index calendar system Nov-Apr · $5,000 stop", "Long-term system (p.16, 19)",
          &[CAL, CAL_X, "X|\"The third page shows the system with a $5,000 stop\" (p.16): 10.00 points below the entry, intraday"],
          CAL_N, claim("15", "47% (7 of 15)", "$7,575/yr ($113,625 total)", "~6 months", "S&P futures Nov 1982-Apr 1997 (p.19)")),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match id {
        "bab80_daytrade" => Some(("bab80_mom", "80% Solution momentum breakout", "Day trade")),
        "bab80_shortterm" => Some(("bab80_mom", "80% Solution momentum breakout", "Short-term (first profitable open)")),
        "bab80_novapr" => Some(("bab80_cal", "Stock index calendar Nov-Apr", "No stop")),
        "bab80_novapr_stop10k" => Some(("bab80_cal", "Stock index calendar Nov-Apr", "$10,000 stop")),
        "bab80_novapr_stop5k" => Some(("bab80_cal", "Stock index calendar Nov-Apr", "$5,000 stop")),
        _ => None,
    }
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    let pt = || p("pt_usd", "Dollars per index point (book: S&P $500)", 500.0, 50.0, 1000.0, 50.0);
    Some(match id {
        "bab80_daytrade" | "bab80_shortterm" => vec![
            p("mom", "Momentum length (book 11; tables p.7-8 test 10-35)", 11.0, 2.0, 40.0, 1.0),
            p("stop_usd", "Money-management stop, $ (book 3,000; p.8 tests 1,000-10,000)", 3000.0, 0.0, 10000.0, 500.0), pt()]
            .into_iter().chain((id == "bab80_daytrade").then(|| p("path", "1 = real 1-minute bars (default; SPX only; days without minute data make no trade), 0 = [Not from the book] daily bar path", 1.0, 0.0, 1.0, 1.0))).collect(), // intraday wave 2b
        "bab80_novapr" => vec![p("stop_usd", "Stop, $ (0 = none; book pages: none, 10,000, 5,000)", 0.0, 0.0, 20000.0, 1000.0), pt()],
        "bab80_novapr_stop10k" => vec![p("stop_usd", "Stop, $ (book 10,000)", 10000.0, 0.0, 20000.0, 1000.0), pt()],
        "bab80_novapr_stop5k" => vec![p("stop_usd", "Stop, $ (book 5,000)", 5000.0, 0.0, 20000.0, 1000.0), pt()],
        _ => return None,
    })
}

fn trade(entry_i: usize, px: f64, exit_i: usize, exit_px: f64, why: &str, kind: &'static str) -> Trade {
    Trade { dir: 1.0, legs: vec![Leg { i: entry_i, px, w: 1.0 }], exit_i, exit_px, reason: why.into(), entry_kind: kind,
        signal_i: exit_i, open: false, exits: vec![] }
}

/// Day-trade / short-term momentum breakout (long only).
fn momentum(x: &Ctx, from: usize, to: usize, n: usize, stop_pts: f64, daytrade: bool) -> Vec<Trade> {
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let mut out = vec![];
    let mut pos: Option<(usize, f64, f64)> = None; // entry bar, fill, stop level
    for i in from.max(n + 2)..=to {
        // position from an earlier bar (short-term only): first profitable open, else the stop
        if let Some((ei, px, st)) = pos {
            if o[i] > px + 1e-9 { out.push(trade(ei, px, i, o[i], "first profitable open", "stop")); pos = None; }
            else if o[i] <= st { out.push(trade(ei, px, i, o[i], "stop (gap at the open)", "stop")); pos = None; }
            else if l[i] <= st { out.push(trade(ei, px, i, st, "stop", "stop")); pos = None; continue; }
            else { continue; }
        }
        let tr = h[i - 1].max(c[i - 2]) - l[i - 1].min(c[i - 2]);
        let setup = c[i - 1] - c[i - 1 - n] > c[i - 2] - c[i - 2 - n] && o[i] > (h[i - 1] + l[i - 1] + c[i - 1]) / 3.0;
        let lv = c[i - 1] + tr;
        if !setup || h[i] < lv { continue; }
        let gap = o[i] >= lv;
        let px = if gap { o[i] } else { lv };
        let st = if stop_pts > 0.0 { lv - stop_pts } else { f64::NEG_INFINITY };
        // entry bar: TradeStation path. Gap fill or O->H->L->C: the low comes after the entry; O->L->H->C: only the close does.
        let after = if gap || h[i] - o[i] <= o[i] - l[i] { l[i] } else { c[i] };
        if after <= st { out.push(trade(i, px, i, st, "stop (entry day)", if gap { "open" } else { "stop" })); continue; }
        if daytrade { out.push(trade(i, px, i, c[i], "exit on the close", if gap { "open" } else { "stop" })); continue; }
        pos = Some((i, px, st));
    }
    if let Some((ei, px, _)) = pos {
        let mut t = trade(ei, px, to, c[to], "open", "stop");
        t.open = true;
        out.push(t);
    }
    out
}

// ---- intraday wave 2b block: the day trade on real 1-minute bars (SPX only, the book's 6.00 S&P points)
/// Same setup on the daily bars (momentum, today's daily open above yesterday's pivot); the buy stop at yesterday's close +
/// true range fills on the first minute that reaches it (at the level, or the minute's open when it opened through it); the
/// stop 6.00 points under the system entry level (p.3: not from the fill) counts on later minutes [fill-minute convention as
/// wave 1]; otherwise out on the official close. Days without minute data make no trade.
fn daytrade_minutes(x: &Ctx, from: usize, to: usize, n: usize, stop_pts: f64) -> Vec<Trade> {
    use crate::intraday::{stop_entry_day, Level};
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let mut out = vec![];
    for i in from.max(n + 2)..=to {
        let tr = h[i - 1].max(c[i - 2]) - l[i - 1].min(c[i - 2]);
        if !(c[i - 1] - c[i - 1 - n] > c[i - 2] - c[i - 2 - n] && o[i] > (h[i - 1] + l[i - 1] + c[i - 1]) / 3.0) { continue; }
        let lv = c[i - 1] + tr;
        let Some(b) = crate::intraday_w2b::day(x, i) else { continue };
        let stop = |_: usize, _: f64| (stop_pts > 0.0).then(|| Level { up: false, px: lv - stop_pts });
        let Some((_, px, st)) = stop_entry_day(&b, 0, &[Level { up: true, px: lv }], &stop) else { continue };
        out.push(match st {
            Some(xp) => trade(i, px, i, xp, "stop", "stop"),
            None => trade(i, px, i, c[i], "exit on the close", "stop"),
        });
    }
    out
}
// ---- end intraday wave 2b block ----

fn ymd(d: &str) -> (i64, i64, i64) { (d[0..4].parse().unwrap_or(0), d[5..7].parse().unwrap_or(0), d[8..10].parse().unwrap_or(0)) }

/// Day of the month of the first Monday of November and of the last Friday of April in year `y`.
fn first_monday_nov(y: i64) -> i64 {
    let w = crate::books::lwlts::dow(&format!("{y:04}-11-01")) as i64; // 0 = Sunday
    1 + (8 - w) % 7
}
fn last_friday_apr(y: i64) -> i64 {
    let w = crate::books::lwlts::dow(&format!("{y:04}-04-30")) as i64;
    30 - (w + 2) % 7
}

/// November-April calendar system (long only).
fn calendar(x: &Ctx, from: usize, to: usize, stop_pts: f64) -> Vec<Trade> {
    let (o, l, c) = (&x.o, &x.l, &x.c);
    let mut out = vec![];
    let mut i = from.max(1);
    while i <= to {
        let (y, m, d) = ymd(&x.r[i].date);
        let (py, pm, pd) = ymd(&x.r[i - 1].date);
        let fm = first_monday_nov(y);
        // first bar on or after the first Monday of November (previous bar before it)
        let is_entry = m == 11 && d >= fm && !(py == y && pm == 11 && pd >= fm);
        if !is_entry { i += 1; continue; }
        let (ey, lf) = (y + 1, last_friday_apr(y + 1));
        let px = o[i];
        let st = if stop_pts > 0.0 { px - stop_pts } else { f64::NEG_INFINITY };
        let mut j = i;
        let mut done = None;
        loop {
            let (jy, jm, jd) = ymd(&x.r[j].date);
            if j > i && o[j] <= st { done = Some((j, o[j], "stop (gap at the open)")); break; }
            if l[j] <= st { done = Some((j, st, "stop")); break; }
            // last bar of the week ending on April's last Friday
            let last = jy == ey && jm == 4 && jd <= lf && jd > lf - 5
                && (j + 1 >= x.r.len() || { let (ny, nm, nd) = ymd(&x.r[j + 1].date); !(ny == ey && nm == 4 && nd <= lf) });
            if last { done = Some((j, c[j], "last Friday of April, on the close")); break; }
            if (jy, jm) > (ey, 4) { done = Some((j - 1, c[j - 1], "last bar before May (no April-Friday bar)")); break; }
            if j >= to { break; }
            j += 1;
        }
        match done {
            Some((k, ex, why)) => { out.push(trade(i, px, k, ex, why, "open")); i = k + 1; }
            None => { let mut t = trade(i, px, to, c[to], "open", "open"); t.open = true; out.push(t); break; }
        }
    }
    out
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    if from > to || to >= x.r.len() { return None; }
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(f64::NAN, |d| d.def));
    let stop_pts = { let v = g("stop_usd") / g("pt_usd"); if v.is_finite() && v > 0.0 { v } else { 0.0 } };
    Some(match id {
        "bab80_daytrade" if g("path") >= 0.5 => daytrade_minutes(x, from, to, g("mom").round().max(1.0) as usize, stop_pts), // intraday wave 2b
        "bab80_daytrade" | "bab80_shortterm" => momentum(x, from, to, g("mom").round().max(1.0) as usize, stop_pts, id == "bab80_daytrade"),
        _ => calendar(x, from, to, stop_pts),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    fn bar(d: &str, o: f64, h: f64, l: f64, c: f64) -> Row {
        Row { ticker: "T".into(), date: d.into(), open: o, high: h, low: l, close: c, volume: None }
    }
    #[test]
    fn calendar_dates() {
        assert_eq!(first_monday_nov(1982), 1);
        assert_eq!(first_monday_nov(1983), 7);
        assert_eq!(last_friday_apr(1983), 29);
        assert_eq!(last_friday_apr(1997), 25);
    }
    #[test]
    fn book_example_march_1997() {
        // Book p.5 spreadsheet, 970218..970311: the 3/7/97 entry at 824.85 and the 3/11 first profitable open 831.95.
        let rows = [("1997-02-18", 830.25, 836.05, 824.80, 834.90), ("1997-02-19", 833.85, 836.95, 829.05, 830.85),
            ("1997-02-20", 828.55, 830.75, 818.35, 820.90), ("1997-02-21", 820.55, 823.95, 818.35, 821.00),
            ("1997-02-24", 817.05, 829.95, 816.65, 829.10), ("1997-02-25", 829.85, 831.55, 825.35, 830.70),
            ("1997-02-26", 830.35, 830.65, 811.55, 820.30), ("1997-02-27", 823.55, 823.95, 808.55, 809.05),
            ("1997-02-28", 809.05, 814.35, 806.55, 806.95), ("1997-03-03", 804.05, 812.85, 803.55, 811.80),
            ("1997-03-04", 813.55, 816.55, 806.65, 808.60), ("1997-03-05", 811.35, 821.20, 810.25, 820.80),
            ("1997-03-06", 822.05, 822.35, 814.65, 817.15), ("1997-03-07", 819.55, 826.55, 817.55, 823.75),
            ("1997-03-10", 823.85, 832.05, 820.65, 831.55), ("1997-03-11", 831.95, 832.55, 827.45, 827.85)];
        let r: Vec<Row> = rows.iter().map(|&(d, o, h, l, c)| bar(d, o, h, l, c)).collect();
        let x = Ctx::new(&r, &r, &[]);
        let t = run("bab80_shortterm", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert_eq!(t.len(), 1);
        assert_eq!((x.r[t[0].legs[0].i].date.as_str(), t[0].legs[0].px), ("1997-03-07", 824.85));
        // 3/10 open 823.85 < 824.85 holds; 3/11 open 831.95 exits (book: 3/11/97 831.95)
        assert!((t[0].exit_px - 831.95).abs() < 1e-9 && x.r[t[0].exit_i].date == "1997-03-11");
        let t = run("bab80_daytrade", &x, 0, r.len() - 1, &Params::from([("path".to_string(), 0.0)])).unwrap();
        assert_eq!((t[0].exit_px, t[0].exit_i), (823.75, 13)); // "a loss on the trade of March 7th" (p.6)
    }
}
