//! YT strategies "Holidays · Long" and "Holidays · Short" (not from a book). Sources: the team's AmiBroker AFLs
//! "Holidays_Updated_with ST_Long.afl" and "Holidays_Updated_with ST_Short.afl". Both choose one event from the AFL's
//! "Select Holiday" list (45 events with hard-coded dates, generated into `ythol_events.rs`); each side uses its own AFL's
//! dates. Defaults are the settings the team verified:
//! Long = Ganesh Chaturthi with the BASE 2 bull filter, from the workbook "Base 2 Bull_Long_Ganesh_Chaturthi_En_Before_3
//! day_Ex_STR (1,2) Fresh_Running_ATR (5,1) (5,2) Live_Close" (all 462 trades of its 33 sheets reproduced);
//! Short = USA : Washington's Birthday (President's Day), from "TS_WOF_Short_President's day Holidays_EN_After-0
//! days_EX_N candles-9 days_EX_ATR" (all 207 trades of its 9 sheets reproduced).
//!
//! Conventions: daily bars, one position at a time, entry at the close of the signal bar. Kept from the AFLs: the holiday
//! entry rule (N bars before or after the holiday bar; when the holiday is not a trading day, counted from the next bar),
//! the BASE 2 bull filter and its exit on each period's exit date (Long), the upper-circuit entry block and lower-circuit
//! exit deferral (Long), the upper-circuit cover deferral (Short). Long exits: N candles, ATR trailing stop on
//! (High+Low)/2, hard stop at the entry candle's low (Live or Close) and the Supertrend exit (period 5 on (High+Low)/2,
//! both lines with the same factor; Fresh = cross below, Running = close below). Short exits: N bars from the signal, the
//! AFL's Dual ATR stop on daily bars and an optional hard stop at the entry candle's high. Dropped: BASE 1 filters,
//! Supertrend entry, RSI exits, dead-stock, penny, turnover and Aux filters.
#![allow(dead_code)]
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "yt_hol";
pub const ID: &str = "ythol_long";
pub const SHORT_ID: &str = "ythol_short";

#[path = "ythol_events.rs"]
mod events;
/// Default events: Ganesh Chaturthi (Long) and USA : Washington's Birthday / President's Day (Short), as verified.
pub const LONG_DEFAULT_EVENT: f64 = 17.0;
pub const SHORT_DEFAULT_EVENT: f64 = 27.0;

/// The selected event's dates (1-based index into the AFL's "Select Holiday" list).
fn event_dates(list: &'static [&'static str; events::COUNT], event: f64) -> &'static str {
    list[(event.round().max(1.0) as usize).min(events::COUNT) - 1]
}
/// Name of event `n` (1-based), for labels and tests.
pub fn event_name(n: usize) -> &'static str {
    events::NAMES[n.clamp(1, events::COUNT) - 1]
}

/// AFL `GaneshChaturthi` dates (dd-mm-yyyy), kept for the tests.
const GANESH_CHATURTHI: &str = "01-09-2000, 22-08-2001, 10-09-2002, 31-08-2003, 18-09-2004, 07-09-2005, 27-08-2006, 15-09-2007, 03-09-2008, 23-08-2009, 11-09-2010, 01-09-2011, 19-09-2012, 09-09-2013, 29-08-2014, 17-09-2015, 05-09-2016, 25-08-2017, 13-09-2018, 02-09-2019, 22-08-2020, 10-09-2021, 31-08-2022, 19-09-2023, 07-09-2024, 27-08-2025, 14-09-2026";
/// AFL (Short) `USAWashingtonBirthday` dates (dd-mm-yyyy), the "President's day" holiday, kept for the tests.
const PRESIDENTS_DAY: &str = "21-02-2000, 19-02-2001, 18-02-2002, 17-02-2003, 16-02-2004, 21-02-2005, 20-02-2006, 19-02-2007, 18-02-2008, 16-02-2009, 15-02-2010, 21-02-2011, 20-02-2012, 18-02-2013, 17-02-2014, 16-02-2015, 15-02-2016, 20-02-2017, 19-02-2018, 18-02-2019, 17-02-2020, 15-02-2021, 21-02-2022, 20-02-2023, 19-02-2024, 17-02-2025, 16-02-2026";
/// AFL `BASE2_EntryDates` / `BASE2_ExitDates`: bull periods [entry, exit).
const BASE2_ENTRY: &str = "01-01-2008,21-11-2008, 26-09-2011, 31-05-2012, 30-08-2013, 29-02-2016, 23-12-2016, 28-02-2019, 30-08-2019, 24-03-2020, 24-06-2022, 28-02-2023, 28-02-2025,20-03-2026";
const BASE2_EXIT: &str = "31-01-2008,31-12-2010, 25-11-2011, 01-03-2013, 31-08-2015, 11-11-2016, 09-02-2018, 10-05-2019, 28-02-2020, 30-11-2021, 23-12-2022, 25-10-2024, 23-01-2026,25-09-2026";

const NOTES: &str = "YT strategy, not a book method. Rules from the team's AFL \"Holidays_Updated_with ST_Long\" with the settings of the team workbook \
\"Base 2 Bull_Long_Ganesh_Chaturthi_En_Before_3 day_Ex_STR (1,2) Fresh_Running_ATR (5,1) (5,2) Live_Close\". \
Event: choose one of the AFL's 45 events (holidays, US holidays, FOMC, RBI MPC, CPI, GDP, moon phases, equinoxes); the dates are the AFL's own lists. Default Ganesh Chaturthi, the event the team verified. \
Entry: buy at the close N trading days before the event (or after, when Entry side is 1); when the event is not a trading day the count starts from the next trading day, as in the AFL. \
BASE 2 filter (1 Bull, the default): entries only inside the team's BASE 2 bull periods, and an open trade exits on each period's exit date; 2 Bear (AFL BASE 2 BEAR filter): entries only outside those periods, and an open trade exits on any bar inside a bull period; 0 off. No entry on an upper-circuit close (High = Close on an up close). \
Exits (each with its own switch; the first one that fires closes the trade at that bar's close): N candles after entry (0 off); \
ATR trailing stop: stop = (High+Low)/2 - ATR x multiplier, raised only, ATR = simple average of the 1-bar true range; exit when the close is below the stop; \
hard stop at the entry candle's low: Live exits when the low breaks it, at the stop or at the open when the open is already below it; Close exits when the close is below it; \
Supertrend (period 5 on (High+Low)/2, both AFL lines with the same factor): Fresh exits when the close crosses below the previous bar's line, Running when it closes below it; once a close is above the line, any close below it exits. \
An exit on a lower-circuit bar (Low = Close on a down close) moves to the next bar. \
Team workbook check (Ganesh Chaturthi, BASE 2 bull, before 3 days): all 462 trades of its 33 sheets (NIFTY 50, NIFTYBANK, NIFTYMIDCAP100; N candle 7, ATR (5,1)/(5,2) with or without the Live/Close hard stop, Supertrend 1/2 Fresh/Running) are reproduced. Other events use the same rules on the AFL's dates and have no team workbook to compare. \
Dropped from the AFL: BASE 1 filters, Supertrend entry, RSI exits, dead-stock, penny, turnover and Aux filters.";

const SHORT_NOTES: &str = "YT strategy, not a book method. Rules from the team's AFL \"Holidays_Updated_with ST_Short\" with the settings of the team workbook \
\"TS_WOF_Short_President's day Holidays_EN_After-0 days_EX_N candles-9 days_EX_ATR\" (no filter). \
Event: choose one of the Short AFL's 45 events (same list as Long; its dates differ only in Bakri Id 2007 and Washington's Birthday 2012). Default USA : Washington's Birthday (President's Day), the event the team verified. \
Entry: sell short at the close of the event bar, or N bars before or after it; when the holiday is not a trading day, the AFL counts from the next trading day (one bar earlier for entries before the holiday). \
Exits (each with its own switch; the first one that fires covers at that bar's close): N candles after the entry signal (0 off); \
Dual ATR on daily bars, reference (High+Low)/2, ATR = simple average of the 1-bar true range: initial stop = mid + ATR x multiplier and favour stop = mid + ATR' x multiplier', both lowered only; \
the favour stop takes over once a close falls below the favour band (mid - ATR' x multiplier' at entry). As in the AFL, that switch is never reset, so it carries into later trades; with the team settings (favour = initial) both stops are the same. \
Optional hard stop at the entry candle's high, Live (cover at the stop, or at the open when it opens above) or Close. \
Supertrend exit (period 5 on (High+Low)/2, both AFL lines at the same factor): Fresh covers when the close crosses above the previous bar's line, Running when it closes above it. \
BASE 2 filter (0 off by default): 1 Bull = entries only inside the team's BASE 2 bull periods, covering on each period's exit date; 2 Bear = entries only outside them, covering on any bar inside a bull period. \
A cover on an upper-circuit bar (High = Close on an up close) moves to the next bar. \
Team workbook check (President's Day, no filter, 0 days): all 207 trades of its 9 sheets (NIFTY 50, NIFTYBANK, NIFTYMIDCAP100; N candles 9, Dual ATR (5,1) and (5,2) with the favour ATR equal to the initial one) are reproduced. The hard stop is not in that workbook; other events have no team workbook to compare. \
Dropped from the AFL: BASE 1 filters, Supertrend entry, RSI exits, dead-stock, penny, turnover and Aux filters.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef {
        id: SHORT_ID,
        name: "Short",
        group: "YT Strategies · Holidays",
        session: 0,
        dir: "Short",
        status: Status::Live,
        needs_volume: false,
        needs_vix: false,
        needs_series: &[],
        book: BOOK,
        rules: &[
            "E|Sell short at the close of the selected event's bar, or N bars before or after it (default President's Day, 0 days)",
            "X|N candle exit: N bars after the entry signal (default 9)",
            "X|Dual ATR stop on daily bars, (High+Low)/2 + ATR x multiplier, lowered only; cover when the close rises above it",
            "X|Optional hard stop at the entry candle's high, Live or Close",
            "X|Supertrend exit (period 5, factor 1 or 2): Fresh = close crosses above the line, Running = close above it",
            "F|BASE 2 filter (optional): Bull = only inside the team's BASE 2 bull periods (cover on the period's exit date), Bear = only outside them (cover when one starts)",
            "X|A cover on an upper-circuit bar moves to the next bar",
        ],
        notes: SHORT_NOTES,
        claim: None,
    }, StrategyDef {
        id: ID,
        name: "Long",
        group: "YT Strategies · Holidays",
        session: 0,
        dir: "Long",
        status: Status::Live,
        needs_volume: false,
        needs_vix: false,
        needs_series: &[],
        book: BOOK,
        rules: &[
            "F|BASE 2 filter: Bull = only inside the team's BASE 2 bull periods, Bear = only outside them; no entry on an upper-circuit close",
            "E|Buy at the close N trading days before the selected event (default Ganesh Chaturthi, 3 days)",
            "X|N candle exit: after N bars (default 7)",
            "X|ATR trailing stop on (High+Low)/2, raised only; exit when the close falls below it",
            "X|Hard stop at the entry candle's low, Live (intraday) or on the Close",
            "X|Supertrend exit (period 5, factor 1 or 2), Fresh cross or Running close below",
            "X|BASE 2 exit: Bull = the period's exit date, Bear = when a bull period starts; an exit on a lower-circuit bar moves to the next bar",
        ],
        notes: NOTES,
        claim: None,
    }]
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    if id == SHORT_ID {
        return Some(vec![
            p("event", events::LABEL, SHORT_DEFAULT_EVENT, 1.0, events::COUNT as f64, 1.0),
            p("days", "Entry days from the holiday", 0.0, 0.0, 15.0, 1.0),
            p("after", "Entry side (0 before, 1 after)", 0.0, 0.0, 1.0, 1.0),
            p("nc_n", "N candle exit (0 off)", 9.0, 0.0, 40.0, 1.0),
            p("atr_on", "Dual ATR exit (0 off, 1 on)", 0.0, 0.0, 1.0, 1.0),
            p("atr_n", "Initial ATR period", 5.0, 1.0, 100.0, 1.0),
            p("atr_mult", "Initial ATR multiplier", 1.0, 1.0, 4.0, 1.0),
            p("fav_n", "Favour ATR period", 5.0, 1.0, 100.0, 1.0),
            p("fav_mult", "Favour ATR multiplier", 1.0, 1.0, 4.0, 1.0),
            p("hsl", "Hard stop at entry high (0 off, 1 Live, 2 Close)", 0.0, 0.0, 2.0, 1.0),
            p("base2", "BASE 2 filter (0 off, 1 Bull, 2 Bear)", 0.0, 0.0, 2.0, 1.0),
            p("st_on", "Supertrend exit (0 off, 1 on)", 0.0, 0.0, 1.0, 1.0),
            p("st_factor", "Supertrend factor", 1.0, 1.0, 4.0, 1.0),
            p("st_running", "Supertrend exit type (0 Fresh, 1 Running)", 0.0, 0.0, 1.0, 1.0),
        ]);
    }
    if id != ID {
        return None;
    }
    Some(vec![
        p("event", events::LABEL, LONG_DEFAULT_EVENT, 1.0, events::COUNT as f64, 1.0),
        p("days", "Entry days from the holiday", 3.0, 0.0, 15.0, 1.0),
        p("after", "Entry side (0 before, 1 after)", 0.0, 0.0, 1.0, 1.0),
        p("base2", "BASE 2 filter (0 off, 1 Bull, 2 Bear)", 1.0, 0.0, 2.0, 1.0),
        p("nc_n", "N candle exit (0 off)", 7.0, 0.0, 15.0, 1.0),
        p("atr_on", "ATR exit (0 off, 1 on)", 0.0, 0.0, 1.0, 1.0),
        p("atr_n", "ATR period", 5.0, 1.0, 100.0, 1.0),
        p("atr_mult", "ATR multiplier", 1.0, 1.0, 4.0, 1.0),
        p("hsl", "Hard stop at entry low (0 off, 1 Live, 2 Close)", 0.0, 0.0, 2.0, 1.0),
        p("st_on", "Supertrend exit (0 off, 1 on)", 0.0, 0.0, 1.0, 1.0),
        p("st_factor", "Supertrend factor", 1.0, 1.0, 4.0, 1.0),
        p("st_running", "Supertrend exit type (0 Fresh, 1 Running)", 0.0, 0.0, 1.0, 1.0),
    ])
}

/// "dd-mm-yyyy" -> "yyyy-mm-dd".
fn iso(s: &str) -> Option<String> {
    let mut f = s.trim().split('-');
    let (d, m, y) = (f.next()?.trim(), f.next()?.trim(), f.next()?.trim());
    Some(format!("{y:0>4}-{m:0>2}-{d:0>2}"))
}
fn dates(list: &str) -> Vec<String> {
    list.split(',').filter_map(iso).collect()
}

/// AFL holiday loop: the entry bar is the holiday bar -/+ `days`; when the holiday has no bar, the first later bar
/// stands in, less one more bar for entries before the holiday.
pub fn holiday_signals(x: &Ctx, holidays: &str, days: usize, after: bool) -> Vec<bool> {
    let n = x.c.len();
    let mut sig = vec![false; n];
    for h in dates(holidays) {
        let (lo, hi) = if after { (0, n.saturating_sub(days)) } else { (days + 1, n) };
        for j in lo..hi {
            let d = x.r[j].date.as_str();
            if d >= h.as_str() {
                let k = match (after, d == h.as_str()) {
                    (true, _) => j + days,
                    (false, true) => j - days,
                    (false, false) => j - days - 1,
                };
                sig[k] = true;
                break;
            }
        }
    }
    sig
}

/// AFL BASE2SigFunc: (inside a bull period [entry, exit), exit-date bar). The scan resumes from the last exit bar found.
pub fn base2(x: &Ctx) -> (Vec<bool>, Vec<bool>) {
    base2_periods(x, BASE2_ENTRY, BASE2_EXIT)
}

/// BASE2SigFunc on an AFL's own entry/exit date lists (dd-mm-yyyy, comma separated).
pub fn base2_periods(x: &Ctx, entry: &str, exit_dates: &str) -> (Vec<bool>, Vec<bool>) {
    let n = x.c.len();
    let (mut inside, mut exit) = (vec![false; n], vec![false; n]);
    let mut idx = 0;
    for (a, b) in dates(entry).into_iter().zip(dates(exit_dates)) {
        for k in idx..n {
            let d = x.r[k].date.as_str();
            if d >= a.as_str() && d < b.as_str() {
                inside[k] = true;
            }
            if d == b.as_str() {
                exit[k] = true;
                idx = k;
                break;
            }
        }
    }
    (inside, exit)
}

fn true_range(x: &Ctx) -> Vec<f64> {
    (0..x.c.len()).map(|i| if i == 0 { x.h[0] - x.l[0] } else { x.h[i].max(x.c[i - 1]) - x.l[i].min(x.c[i - 1]) }).collect()
}
fn sma(v: &[f64], n: usize) -> Vec<f64> {
    (0..v.len()).map(|i| if i + 1 >= n { v[i + 1 - n..=i].iter().sum::<f64>() / n as f64 } else { f64::NAN }).collect()
}

/// AFL Supertrend loop (period `n`, factor `f`, reference (High+Low)/2): the line on each bar (NaN before it exists).
pub fn supertrend(x: &Ctx, n: usize, f: f64) -> Vec<f64> {
    let len = x.c.len();
    let atr = sma(&true_range(x), n);
    let mid: Vec<f64> = (0..len).map(|i| (x.h[i] + x.l[i]) / 2.0).collect();
    let mut up: Vec<f64> = (0..len).map(|i| mid[i] + atr[i] * f).collect();
    let mut dn: Vec<f64> = (0..len).map(|i| mid[i] - atr[i] * f).collect();
    let mut is_up = vec![true; len];
    for i in n.max(2)..len {
        let c = x.c[i];
        is_up[i] = if c > up[i - 1] && c < dn[i - 1] && dn[i - 1] == dn[i - 2] {
            false
        } else if c > up[i - 1] {
            true
        } else if c < dn[i - 1] {
            false
        } else {
            is_up[i - 1]
        };
        if is_up[i] && is_up[i - 1] {
            dn[i] = dn[i].max(dn[i - 1]);
        } else if !is_up[i] && !is_up[i - 1] {
            up[i] = up[i].min(up[i - 1]);
        }
    }
    (0..len).map(|i| if is_up[i] { dn[i] } else { up[i] }).collect()
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(0.0, |d| d.def));
    let on = |k: &str| g(k) >= 0.5;
    let whole = |k: &str| g(k).round().max(0.0) as usize;
    if id == SHORT_ID {
        return Some(run_short(x, from, to, &g));
    }
    let sig = holiday_signals(x, event_dates(&events::LONG, g("event")), whole("days"), on("after"));
    // BASE 2 filter: 1 Bull (inside the bull periods, exit on the period's exit date), 2 Bear (outside them, exit when one starts)
    let base_mode = g("base2").round() as i64;
    let (inside, base_exit) = base2(x);
    let (atr_on, atr_mult) = (on("atr_on"), g("atr_mult"));
    let atr = sma(&true_range(x), whole("atr_n").max(1));
    let mid = |i: usize| (x.h[i] + x.l[i]) / 2.0;
    let hsl = g("hsl").round() as i64;
    let nc_n = whole("nc_n");
    let st_on = on("st_on");
    let running = on("st_running");
    // Ref(ST, -1): the previous bar's line, compared with today's close
    let st: Vec<f64> = if st_on { supertrend(x, 5, g("st_factor")) } else { Vec::new() };
    let prev_st = |i: usize| if i >= 1 && !st[i - 1].is_nan() && st[i - 1] != 0.0 { Some(st[i - 1]) } else { None };
    let st_exit = |i: usize| match (prev_st(i), running) {
        (Some(l), true) => x.c[i] < l,
        (Some(l), false) => x.c[i] < l && i >= 2 && st[i - 2].is_finite() && x.c[i - 1] >= st[i - 2],
        _ => false,
    };
    let uc = |i: usize| x.h[i] == x.c[i] && x.c[i] > x.c[i - 1];
    let lc = |i: usize| x.l[i] == x.c[i] && x.c[i] < x.c[i - 1];

    struct Open {
        t: Trade,
        stop: f64,
        low: f64,
        count: usize,
        above: bool,
        deferred: bool,
    }
    let mut trades = Vec::new();
    let mut cur: Option<Open> = None;
    for i in from.max(1)..=to {
        if let Some(mut s) = cur.take() {
            if atr_on && !atr[i].is_nan() {
                s.stop = if s.stop.is_nan() { s.stop } else { s.stop.max(mid(i) - atr[i] * atr_mult) };
            }
            s.count += 1;
            let hsl_hit = match hsl {
                1 => x.l[i] < s.low,
                2 => x.c[i] < s.low,
                _ => false,
            };
            let atr_hit = atr_on && !s.stop.is_nan() && x.c[i] < s.stop;
            // AFL dual-line flag: after a close above the line, any close below it exits
            if st_on && !s.above && prev_st(i).is_some_and(|l| x.c[i] > l) {
                s.above = true;
            }
            let st_hit = st_on && if s.above { prev_st(i).is_some_and(|l| x.c[i] < l) } else { st_exit(i) };
            let why = if st_hit {
                Some("Supertrend exit".to_string())
            } else if nc_n > 0 && s.count == nc_n {
                Some(format!("{nc_n} candle{}", if nc_n == 1 { "" } else { "s" }))
            } else if base_mode == 1 && base_exit[i] {
                Some("BASE 2 period exit".to_string())
            } else if base_mode == 2 && inside[i] {
                Some("BASE 2 bull period starts".to_string())
            } else if s.deferred {
                Some("Exit deferred from a lower-circuit bar".to_string())
            } else if atr_hit {
                Some("ATR stop".to_string())
            } else if hsl_hit {
                Some("Hard stop at entry low".to_string())
            } else {
                None
            };
            match why {
                Some(_) if lc(i) => {
                    s.deferred = true;
                    cur = Some(s);
                }
                Some(reason) => {
                    let mut px = x.c[i];
                    if hsl == 1 && hsl_hit {
                        px = if x.o[i] < s.low { x.l[i].max(x.o[i]) } else { s.low };
                    }
                    s.t.exit_i = i;
                    s.t.exit_px = px;
                    s.t.signal_i = i;
                    s.t.reason = if hsl == 1 && hsl_hit { "Hard stop at entry low".into() } else { reason };
                    trades.push(s.t);
                }
                None => cur = Some(s),
            }
            continue;
        }
        if !sig[i] || (base_mode == 1 && !inside[i]) || (base_mode == 2 && inside[i]) || uc(i) || x.c[i] <= 0.0 {
            continue;
        }
        let px = x.c[i];
        let stop = if atr_on { mid(i) - atr[i] * atr_mult } else { f64::NAN };
        let t = Trade { dir: 1.0, legs: vec![Leg { i, px, w: 1.0 }], exit_i: i, exit_px: px, reason: String::new(), entry_kind: "close", signal_i: i, open: false, exits: Vec::new() };
        cur = Some(Open { t, stop, low: x.l[i], count: 0, above: false, deferred: false });
    }
    if let Some(Open { mut t, .. }) = cur {
        t.exit_i = to;
        t.exit_px = x.c[to];
        // Still held on the last bar of the selected period: closes there (End of period), unless entered on that bar.
        t.open = t.legs[0].i == to;
        t.reason = if t.open { "open".into() } else { "End of period".into() };
        t.signal_i = to;
        trades.push(t);
    }
    Some(trades)
}

/// The short President's Day loop (AFL "Holidays_Updated_with ST_Short").
fn run_short(x: &Ctx, from: usize, to: usize, g: &dyn Fn(&str) -> f64) -> Vec<Trade> {
    let whole = |k: &str| g(k).round().max(0.0) as usize;
    let sig = holiday_signals(x, event_dates(&events::SHORT, g("event")), whole("days"), g("after") >= 0.5);
    let (atr_on, m1, m2) = (g("atr_on") >= 0.5, g("atr_mult"), g("fav_mult"));
    let tr = true_range(x);
    let (a1, a2) = (sma(&tr, whole("atr_n").max(1)), sma(&tr, whole("fav_n").max(1)));
    let mid = |i: usize| (x.h[i] + x.l[i]) / 2.0;
    let (nc_n, hsl) = (whole("nc_n"), g("hsl").round() as i64);
    let uc = |i: usize| x.h[i] == x.c[i] && x.c[i] > x.c[i - 1];
    let live = |v: f64| v.is_finite() && v != 0.0;
    // BASE 2 filter as in the AFL: 1 Bull (inside the bull periods, cover on the period's exit date), 2 Bear (outside them,
    // cover when one starts)
    let base_mode = g("base2").round() as i64;
    let (inside, base_exit) = base2(x);
    // Supertrend exit (AFL Shortexitsig with both lines at the same factor): Fresh = the close crosses above the previous
    // bar's line, Running = the close is above it
    let (st_on, running) = (g("st_on") >= 0.5, g("st_running") >= 0.5);
    let st: Vec<f64> = if st_on { supertrend(x, 5, g("st_factor")) } else { Vec::new() };
    let prev_st = |i: usize| if i >= 1 && st[i - 1].is_finite() && st[i - 1] != 0.0 { Some(st[i - 1]) } else { None };
    let st_hit = |i: usize| match prev_st(i) {
        Some(l) if running => x.c[i] > l,
        Some(l) => x.c[i] > l && i >= 2 && st[i - 2].is_finite() && x.c[i - 1] <= st[i - 2],
        None => false,
    };
    // AFL ATRShortPlot (initial stop), ATRShortPlot1 (favour band), ATRShortPlot2 (favour stop); Flag is never reset.
    let (mut stop, mut band, mut fav, mut flag) = (f64::NAN, f64::NAN, f64::NAN, false);
    let (mut trades, mut cur, mut last_sig, mut deferred): (Vec<Trade>, Option<Trade>, Option<usize>, bool) = (Vec::new(), None, None, false);
    for i in 1..=to {
        if sig[i] {
            last_sig = Some(i);
        }
        if cur.is_some() && atr_on && stop.is_finite() {
            stop = stop.min(mid(i) + a1[i] * m1);
        }
        if atr_on && band.is_finite() && x.c[i] < band {
            flag = true;
            if cur.is_some() {
                band = f64::NAN;
            }
        }
        if cur.is_some() && atr_on && fav.is_finite() {
            fav = fav.min(mid(i) + a2[i] * m2);
        }
        let Some(mut t) = cur.take() else {
            if i >= from && sig[i] && x.c[i] > 0.0 && !(base_mode == 1 && !inside[i]) && !(base_mode == 2 && inside[i]) {
                if atr_on {
                    (stop, band, fav) = (mid(i) + a1[i] * m1, mid(i) - a2[i] * m2, mid(i) + a2[i] * m2);
                }
                cur = Some(Trade { dir: -1.0, legs: vec![Leg { i, px: x.c[i], w: 1.0 }], exit_i: i, exit_px: x.c[i], reason: String::new(), entry_kind: "close", signal_i: i, open: false, exits: Vec::new() });
            }
            continue;
        };
        let e = t.legs[0].i;
        let hsl_hit = match hsl {
            1 => x.h[i] > x.h[e],
            2 => x.c[i] > x.h[e],
            _ => false,
        };
        let candle = nc_n > 0 && last_sig.is_some_and(|s| i - s == nc_n);
        let atr_hit = atr_on && if flag { live(fav) && x.c[i] > fav } else { live(stop) && x.c[i] > stop };
        let why = if candle {
            format!("{nc_n} candle{}", if nc_n == 1 { "" } else { "s" })
        } else if atr_hit {
            "ATR stop".into()
        } else if hsl_hit {
            "Hard stop at entry high".into()
        } else if st_on && st_hit(i) {
            "Supertrend exit".into()
        } else if base_mode == 1 && base_exit[i] {
            "BASE 2 period exit".into()
        } else if base_mode == 2 && inside[i] {
            "BASE 2 bull period starts".into()
        } else if deferred {
            "Cover deferred from an upper-circuit bar".into()
        } else {
            cur = Some(t);
            continue;
        };
        if uc(i) {
            deferred = true;
            cur = Some(t);
            continue;
        }
        t.exit_i = i;
        t.exit_px = if hsl == 1 && hsl_hit { if x.o[i] > x.h[e] { x.h[i].min(x.o[i]) } else { x.h[e] } } else { x.c[i] };
        t.signal_i = i;
        t.reason = if hsl == 1 && hsl_hit { "Hard stop at entry high".into() } else { why };
        trades.push(t);
        (stop, band, fav, deferred) = (f64::NAN, f64::NAN, f64::NAN, false);
    }
    if let Some(mut t) = cur {
        t.exit_i = to;
        t.exit_px = x.c[to];
        t.open = t.legs[0].i == to;
        t.reason = if t.open { "open".into() } else { "End of period".into() };
        t.signal_i = to;
        trades.push(t);
    }
    trades
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match id {
        ID => Some(("ythol", "Holidays", "Long")),
        SHORT_ID => Some(("ythol", "Holidays", "Short")),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::books::ytmr::day_number;
    use crate::Row;

    fn civil(z: i64) -> String {
        let z = z + 719468;
        let era = z.div_euclid(146097);
        let doe = z - era * 146097;
        let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let d = doy - (153 * mp + 2) / 5 + 1;
        let m = if mp < 10 { mp + 3 } else { mp - 9 };
        format!("{:04}-{:02}-{:02}", yoe + era * 400 + i64::from(m <= 2), m, d)
    }
    /// Weekdays from `a` to `b`, flat at close 100 (high 101, low 99) unless `set` lists the date.
    fn rows(a: &str, b: &str, set: &[(&str, (f64, f64, f64, f64))]) -> Vec<Row> {
        let (mut d, end) = (day_number(a).unwrap(), day_number(b).unwrap());
        let mut out = Vec::new();
        while d <= end {
            if (d + 4).rem_euclid(7) % 6 != 0 {
                let date = civil(d);
                let (open, high, low, close) = set.iter().find(|(k, _)| *k == date).map_or((100.0, 101.0, 99.0, 100.0), |x| x.1);
                out.push(Row { ticker: "T".into(), date, open, high, low, close, volume: None });
            }
            d += 1;
        }
        out
    }
    fn ps(kv: &[(&str, f64)]) -> Params {
        kv.iter().map(|(k, v)| (k.to_string(), *v)).collect()
    }
    fn go(r: &[Row], kv: &[(&str, f64)]) -> Vec<(String, f64, String, f64, String)> {
        let x = Ctx::new(r, r, &[]);
        run(ID, &x, 0, r.len() - 1, &ps(kv)).unwrap().into_iter()
            .map(|t| (r[t.legs[0].i].date.clone(), t.legs[0].px, r[t.exit_i].date.clone(), t.exit_px, t.reason)).collect()
    }

    #[test]
    fn enters_three_bars_before_and_exits_after_seven_candles() {
        // Ganesh Chaturthi 2024-09-07 is a Saturday: the next bar is Mon 09-09, so entry is 4 bars earlier, Tue 09-03
        let r = rows("2024-08-01", "2024-09-30", &[]);
        let t = go(&r, &[]);
        assert_eq!(t[0].0, "2024-09-03");
        assert_eq!((t[0].2.as_str(), t[0].4.as_str()), ("2024-09-12", "7 candles"));
        // 2023-09-19 is a Tuesday (a bar): entry 3 bars earlier, Thu 09-14
        let r = rows("2023-08-01", "2023-09-30", &[]);
        assert_eq!(go(&r, &[])[0].0, "2023-09-14");
    }

    #[test]
    fn base2_filter_blocks_entries_outside_bull_periods() {
        // 2018-09-13 lies outside every BASE 2 period
        let r = rows("2018-08-01", "2018-09-30", &[]);
        assert!(go(&r, &[]).is_empty());
        assert_eq!(go(&r, &[("base2", 0.0)])[0].0, "2018-09-10");
        // Bear: entries only outside the bull periods -> 2018 trades, 2024 (inside a bull period) does not
        assert_eq!(go(&r, &[("base2", 2.0)])[0].0, "2018-09-10");
        assert!(go(&rows("2024-08-01", "2024-09-30", &[]), &[("base2", 2.0)]).is_empty());
    }

    #[test]
    fn base2_exit_date_and_lower_circuit_deferral() {
        // 2024: entry 09-03; BASE 2 period ends 2024-10-25. With no other exit the trade leaves on that date.
        let r = rows("2024-08-01", "2024-11-29", &[]);
        let t = go(&r, &[("nc_n", 0.0)]);
        assert_eq!((t[0].2.as_str(), t[0].4.as_str()), ("2024-10-25", "BASE 2 period exit"));
        // the 7th candle (09-12) closes at its low on a down close: the exit moves to 09-13
        let r = rows("2024-08-01", "2024-09-30", &[("2024-09-12", (100.0, 101.0, 98.0, 98.0))]);
        let t = go(&r, &[]);
        assert_eq!((t[0].2.as_str(), t[0].4.as_str()), ("2024-09-13", "Exit deferred from a lower-circuit bar"));
    }

    #[test]
    fn hard_stop_live_fills_at_stop_or_gap_open_and_close_waits_for_the_close() {
        // entry 2024-09-03 low 99; 09-04 trades down to 98.5 but closes 100; 09-05 opens 97 below the stop
        let set = [("2024-09-04", (100.0, 101.0, 98.5, 100.0)), ("2024-09-05", (97.0, 98.0, 96.0, 97.5))];
        let r = rows("2024-08-01", "2024-09-30", &set);
        let t = go(&r, &[("hsl", 1.0)]);
        assert_eq!((t[0].2.as_str(), t[0].3), ("2024-09-04", 99.0));
        let t = go(&r, &[("hsl", 2.0)]);
        assert_eq!((t[0].2.as_str(), t[0].3), ("2024-09-05", 97.5));
        let r = rows("2024-08-01", "2024-09-30", &set[1..]);
        let t = go(&r, &[("hsl", 1.0)]);
        assert_eq!((t[0].2.as_str(), t[0].3, t[0].4.as_str()), ("2024-09-05", 97.0, "Hard stop at entry low"));
    }

    #[test]
    fn atr_stop_trails_mid_price() {
        // flat bars: TR 2, ATR(5) 2, mid 100 -> stop 98 (x1); 09-04 closes 97.5 < 98
        let r = rows("2024-08-01", "2024-09-30", &[("2024-09-04", (99.0, 99.5, 97.0, 97.5))]);
        let t = go(&r, &[("atr_on", 1.0), ("nc_n", 0.0)]);
        assert_eq!((t[0].2.as_str(), t[0].4.as_str()), ("2024-09-04", "ATR stop"));
        // multiplier 2: stop 96 holds
        assert_ne!(go(&r, &[("atr_on", 1.0), ("atr_mult", 2.0), ("nc_n", 0.0)])[0].2, "2024-09-04");
    }

    fn go_short(r: &[Row], kv: &[(&str, f64)]) -> Vec<(String, f64, String, f64, String)> {
        let x = Ctx::new(r, r, &[]);
        run(SHORT_ID, &x, 0, r.len() - 1, &ps(kv)).unwrap().into_iter()
            .map(|t| (r[t.legs[0].i].date.clone(), t.legs[0].px, r[t.exit_i].date.clone(), t.exit_px, t.reason)).collect()
    }

    #[test]
    fn presidents_day_short_nine_candles_and_atr_cover() {
        // 2024-02-19 (Monday) is President's Day: short at its close, cover 9 bars later (2024-03-01)
        let r = rows("2024-01-01", "2024-03-29", &[]);
        let t = go_short(&r, &[]);
        assert_eq!((t[0].0.as_str(), t[0].2.as_str(), t[0].4.as_str()), ("2024-02-19", "2024-03-01", "9 candles"));
        // flat bars: ATR 2, mid 100, stop 102 (x1); 02-21 closes 102.5 above it (not at its high, so no deferral)
        let r = rows("2024-01-01", "2024-03-29", &[("2024-02-21", (101.0, 103.0, 100.5, 102.5))]);
        let t = go_short(&r, &[("nc_n", 0.0), ("atr_on", 1.0)]);
        assert_eq!((t[0].2.as_str(), t[0].3, t[0].4.as_str()), ("2024-02-21", 102.5, "ATR stop"));
        // the same close at the bar's high on an up close is an upper circuit: the cover waits a bar
        let r = rows("2024-01-01", "2024-03-29", &[("2024-02-21", (101.0, 102.5, 100.5, 102.5))]);
        let t = go_short(&r, &[("nc_n", 0.0), ("atr_on", 1.0)]);
        assert_eq!((t[0].2.as_str(), t[0].4.as_str()), ("2024-02-22", "Cover deferred from an upper-circuit bar"));
    }

    #[test]
    fn event_lists_default_to_the_verified_events() {
        assert_eq!(event_name(LONG_DEFAULT_EVENT as usize), "Ganesh Chaturthi");
        assert_eq!(event_name(SHORT_DEFAULT_EVENT as usize), "USA : Washington's Birthday");
        assert_eq!(dates(event_dates(&events::LONG, LONG_DEFAULT_EVENT)), dates(GANESH_CHATURTHI));
        assert_eq!(dates(event_dates(&events::SHORT, SHORT_DEFAULT_EVENT)), dates(PRESIDENTS_DAY));
        assert_eq!(events::NAMES.len(), 48);
        assert!(events::LONG.iter().zip(events::SHORT).all(|(l, s)| !dates(l).is_empty() && !dates(s).is_empty()));
        assert!(events::LABEL.starts_with("Event::Union Budget Day|"));
    }

    #[test]
    fn event_switch_moves_the_entry() {
        // Republic Day (event 3) 2024-01-26 is a Friday: Long, 3 bars before -> 2024-01-23 (no BASE 2 filter)
        let r = rows("2023-12-01", "2024-03-29", &[]);
        assert_eq!(go(&r, &[("event", 3.0), ("base2", 0.0)])[0].0, "2024-01-23");
        // Short, event 3, 0 days: short at the 2024-01-26 close
        assert_eq!(go_short(&r, &[("event", 3.0)])[0].0, "2024-01-26");
    }

    #[test]
    fn short_supertrend_exit_and_base2_filter() {
        // flat bars: the line is 98 (up trend) and closes stay above it -> Running covers on the first bar after entry
        let r = rows("2024-01-01", "2024-03-29", &[]);
        let t = go_short(&r, &[("nc_n", 0.0), ("st_on", 1.0), ("st_running", 1.0)]);
        assert_eq!((t[0].0.as_str(), t[0].2.as_str(), t[0].4.as_str()), ("2024-02-19", "2024-02-20", "Supertrend exit"));
        // Fresh needs a cross from below: none on flat bars, so the 9-candle exit closes it
        assert_eq!(go_short(&r, &[("st_on", 1.0)])[0].4, "9 candles");
        // 2024-02-19 is inside a BASE 2 bull period: Bull keeps the trade, Bear drops it
        assert_eq!(go_short(&r, &[("base2", 1.0)]).len(), 1);
        assert!(go_short(&r, &[("base2", 2.0)]).is_empty());
        // 2018-02-19 is outside every bull period
        let r = rows("2018-01-01", "2018-03-30", &[]);
        assert_eq!(go_short(&r, &[("base2", 2.0)])[0].0, "2018-02-19");
    }

    #[test]
    fn supertrend_line_on_flat_bars() {
        let r = rows("2024-08-01", "2024-08-30", &[]);
        let x = Ctx::new(&r, &r, &[]);
        let st = supertrend(&x, 5, 1.0);
        assert!(st[3].is_nan());
        assert_eq!(st[10], 98.0); // up trend: lower line (High+Low)/2 - ATR x 1 = 100 - 2
    }
}
