//! YT strategy "Monday Reversal, Type 1" (not from a book). Source: the team's AmiBroker AFL
//! "Buy_on_Monday_Base2_ATR (2)_Modified.afl" (Buy on Monday, Base 2 Filter + Dual ATR Exit), reduced to the rules the
//! team specified for the YT Strategies tab.
//!
//! Conventions: daily bars, long only, one position at a time, signals read on the close of bar i with data <= i.
//! A Friday/Monday pair is the Monday bar whose previous bar is the Friday three calendar days earlier; any trading day
//! between them (e.g. a Saturday session) or a missing Friday/Monday skips the pair. Entry at the Monday close.
//! Exits are checked from the bar after entry: the Hard SL first (fills at the stop, or at the open less the buffer
//! when the open gaps through it), then the Candle exit, the Dual ATR exit and the Weekday exit at the close; each has its
//! own switch, as in the AFL. No new entry on an exit bar (AFL `i != exitIndex`). The BASE 2 filter uses this AFL's own
//! bull-period dates. Dropped from the AFL: Profit Target,
//! SuperTrend/ATHC filters, candle colour, upper-circuit, penny and turnover filters, weekly/monthly ATR
//! and candle timeframes, ATR reference prices other than (High+Low)/2 and the "N Candles Exit Only" counter.
#![allow(dead_code)]
use super::ythol::base2_periods;
use crate::connors::{true_range, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "yt_mr";
pub const ID: &str = "ytmr_monday_reversal_t1";

/// AFL `BASE15DP_EntryDates` / `BASE1_ExitDates`: bull periods [entry, exit), as listed in this AFL.
const BASE2_ENTRY: &str = "01-01-2008, 21-11-2008, 26-09-2011, 31-05-2012, 30-08-2013, 29-02-2016, 23-12-2016, 28-02-2019, 30-08-2019, 24-03-2020, 24-06-2022, 28-02-2023, 28-02-2025";
const BASE2_EXIT: &str = "31-01-2008, 31-12-2010, 25-11-2011, 01-03-2013, 31-08-2015, 11-11-2016, 09-02-2018, 10-05-2019, 28-02-2020, 30-11-2021, 23-12-2022, 25-10-2024, 23-01-2026";

/// Entry comparisons in the order of the entry-condition toggle (1-based).
pub const ENTRY: [&str; 5] = [
    "Friday open > Monday high",
    "Friday low > Monday low",
    "Friday low > Monday close",
    "Friday close > Monday close",
    "Friday high > Monday high",
];

const NOTES: &str = "YT strategy, not a book method. Rules from the team's AFL \"Buy_on_Monday_Base2_ATR (2)_Modified\" as specified for the YT Strategies tab. \
Entry condition (1-5): 1 Friday open > Monday high; 2 Friday low > Monday low; 3 Friday low > Monday close; 4 Friday close > Monday close; 5 Friday high > Monday high (strict >). \
Entry condition 2 (0 = off, default; 1-5 as above): when on, both selected conditions must hold on the same Friday/Monday pair (condition 1 AND condition 2). \
BASE 2 filter (0 = off, default): 1 Bull (AFL BASE 2 5-DP BULL filter) = entries only inside the AFL's 13 bull periods (01-01-2008 to 23-01-2026), and an open trade exits at the close of the period's exit date; \
2 Bear (AFL BASE 2 5-DP BEAR filter) = entries only outside them, and an open trade exits at the close of the first bar of a bull period. The AFL's two toggles are one setting here (both on together allow no entry). \
Candle exit (on by default; its switch or N = 0 turns it off): exit at the close of the Nth candle of the selected type after entry; type 1 N Candle counts every bar, \
2 Green Candle counts a close above the open, 3 UpClose (default) counts a close above the previous close. \
Dual ATR exit (off by default, on daily bars; independent of the Candle exit, whichever fires first): ATR = simple average of the 1-bar true range over the period (AFL sum of ATR(1)); reference price (High+Low)/2. Initial stop = mid - ATR x mult, raised only; \
favour band upper = mid + ATR' x mult', lowered only, and once a close exceeds it the favour stop mid - ATR' x mult' (raised only) replaces the initial stop. \
A close below the previous bar's active stop exits at that close. Entries wait until the ATR values exist. \
Hard SL (off by default): long stop pct% below the entry close, active for the first N bars after entry; fills at the stop, or at the open less the buffer % when the open is at or below it. \
Weekday exit (off by default; not in the AFL, added by the team): 1 Wednesday = exit at the close of the first Wednesday after entry, or the next trading day when that Wednesday is a holiday; \
2 Friday = exit at the close of the first Friday after entry, or the trading day before it when that Friday is a holiday (if no trading day falls between the entry and that Friday, the next trading day). \
Dropped from the AFL: Profit Target, SuperTrend/ATHC filters, candle colour, upper-circuit/penny/turnover filters, weekly/monthly ATR and candle timeframes, ATR reference prices other than (High+Low)/2, N Candles Exit Only. Results are simulated on the selected instrument; no published figures exist for this strategy.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef {
        id: ID,
        name: "Monday Reversal, Type 1",
        group: "YT Strategies · Monday Reversal",
        session: 0,
        dir: "Long",
        status: Status::Live,
        needs_volume: false,
        needs_vix: false,
        needs_series: &[],
        book: BOOK,
        rules: &[
            "F|Use a Friday and the Monday right after it; skip the pair if any trading day falls between them",
            "E|Selected entry condition (one of): Friday open > Monday high; Friday low > Monday low; Friday low > Monday close; Friday close > Monday close; Friday high > Monday high",
            "E|Optional entry condition 2 (same five choices, off by default): when on, both conditions must hold together",
            "F|Optional BASE 2 filter (off by default): Bull = only inside the team's BASE 2 bull periods, Bear = only outside them",
            "E|Buy at the Monday close; no entry on a bar where a trade exited",
            "X|Candle exit (on by default; switch off or N = 0): exit at the close of the Nth candle after entry of the selected type (every candle, green candle or up close)",
            "X|Dual ATR exit (off by default): trailing stop on daily bars; exit at the close when the close falls below the active stop",
            "X|Optional: Hard SL % for the first N bars after entry",
            "X|Optional Weekday exit (off by default): close of the first Wednesday after entry (holiday: next trading day) or the first Friday after entry (holiday: trading day before)",
            "X|BASE 2 exit (with the filter on): Bull = the bull period's exit date, Bear = the first bar of a bull period, at the close",
        ],
        notes: NOTES,
        claim: None,
    }]
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    if id != ID {
        return None;
    }
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    Some(vec![
        p("entry", "Entry condition (1 Fri O>Mon H, 2 Fri L>Mon L, 3 Fri L>Mon C, 4 Fri C>Mon C, 5 Fri H>Mon H)", 1.0, 1.0, 5.0, 1.0),
        p("entry2", "Entry condition 2, both must hold (0 off, 1 Fri O>Mon H, 2 Fri L>Mon L, 3 Fri L>Mon C, 4 Fri C>Mon C, 5 Fri H>Mon H)", 0.0, 0.0, 5.0, 1.0),
        p("base2", "BASE 2 filter (0 off, 1 Bull, 2 Bear)", 0.0, 0.0, 2.0, 1.0),
        p("nc_on", "Candle exit (0 = off, 1 = on)", 1.0, 0.0, 1.0, 1.0),
        p("candle_type", "Type of exit candle (1 N Candle, 2 Green Candle, 3 UpClose)", 3.0, 1.0, 3.0, 1.0),
        p("upclose_n", "N candles of that type for exit (0 off)", 1.0, 0.0, 10.0, 1.0),
        p("atr_on", "Dual ATR exit (0 = off, 1 = on)", 0.0, 0.0, 1.0, 1.0),
        p("atr_n", "Initial ATR period", 5.0, 5.0, 15.0, 5.0),
        p("atr_mult", "Initial ATR multiplier", 2.0, 1.0, 6.0, 1.0),
        p("fav_n", "Favour ATR period", 5.0, 5.0, 15.0, 5.0),
        p("fav_mult", "Favour ATR multiplier", 2.0, 1.0, 8.0, 1.0),
        p("hsl_on", "Hard SL (0 = off, 1 = on)", 0.0, 0.0, 1.0, 1.0),
        p("hsl_pct", "Hard SL %", 5.0, 0.5, 20.0, 0.5),
        p("hsl_days", "Hard SL active for N bars", 1.0, 1.0, 4.0, 1.0),
        p("sl_buffer", "Gap fill buffer % (open through the SL)", 0.25, 0.0, 10.0, 0.05),
        p("wd_on", "Weekday exit (0 = off, 1 = on)", 0.0, 0.0, 1.0, 1.0),
        p("wd_day", "Weekday exit day (1 Wednesday, 2 Friday)", 1.0, 1.0, 2.0, 1.0),
    ])
}

/// Days since 1970-01-01 for a YYYY-MM-DD date (proleptic Gregorian).
pub(crate) fn day_number(date: &str) -> Option<i64> {
    let y: i64 = date.get(0..4)?.parse().ok()?;
    let m: i64 = date.get(5..7)?.parse().ok()?;
    let d: i64 = date.get(8..10)?.parse().ok()?;
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * (m + if m > 2 { -3 } else { 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    Some(era * 146097 + doe - 719468)
}

/// 0 = Sunday .. 6 = Saturday.
fn weekday(day: i64) -> i64 {
    (day + 4).rem_euclid(7)
}

/// Bar i is a Monday whose previous bar is the Friday three calendar days earlier.
pub fn friday_monday(x: &Ctx, i: usize) -> bool {
    if i == 0 {
        return false;
    }
    match (day_number(&x.r[i].date), day_number(&x.r[i - 1].date)) {
        (Some(mon), Some(fri)) => weekday(mon) == 1 && mon - fri == 3,
        _ => false,
    }
}

fn entry_signal(x: &Ctx, i: usize, kind: usize) -> bool {
    let f = i - 1;
    match kind {
        1 => x.o[f] > x.h[i],
        2 => x.l[f] > x.l[i],
        3 => x.l[f] > x.c[i],
        4 => x.c[f] > x.c[i],
        5 => x.h[f] > x.h[i],
        _ => false,
    }
}

/// AFL `ATR(1)` summed over `n` bars / n: a simple average of the true range (NaN until n bars exist).
fn avg_true_range(x: &Ctx, n: usize) -> Vec<f64> {
    let tr = true_range(x.r);
    let mut out = vec![f64::NAN; tr.len()];
    let mut sum = 0.0;
    for i in 0..tr.len() {
        sum += tr[i];
        if i >= n {
            sum -= tr[i - n];
        }
        if i + 1 >= n {
            out[i] = sum / n as f64;
        }
    }
    out
}

/// Dual ATR levels carried through one trade (AFL ATRLongPlot, ATRLongPlot1, ATRLongPlot2 and Flag_ATR).
struct Dual {
    stop: f64,
    upper: Option<f64>,
    fav_stop: f64,
    favour: bool,
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(0.0, |d| d.def));
    let u = |k: &str| g(k).round().max(1.0) as usize;
    let kind = g("entry").round() as usize;
    let kind2 = g("entry2").round().max(0.0) as usize;
    let candle = g("candle_type").round() as i64;
    // Candle exit: off with its switch or with N = 0
    let n_candle = if g("nc_on") >= 0.5 { g("upclose_n").round().max(0.0) as usize } else { 0 };
    let atr_exit = g("atr_on") >= 0.5;
    let (m1, m2) = (g("atr_mult"), g("fav_mult"));
    let (hsl_on, hsl_pct, hsl_days, buffer) = (g("hsl_on") >= 0.5, g("hsl_pct"), u("hsl_days"), g("sl_buffer") / 100.0);
    // Weekday exit: 3 = Wednesday (holiday: next trading day), 5 = Friday (holiday: the trading day before)
    let wd_exit = (g("wd_on") >= 0.5).then(|| if g("wd_day").round() as i64 == 2 { 5 } else { 3 });
    let day = |i: usize| day_number(&x.r[i].date).unwrap_or(i64::MIN);
    let (atr1, atr2) = if atr_exit { (avg_true_range(x, u("atr_n")), avg_true_range(x, u("fav_n"))) } else { (Vec::new(), Vec::new()) };
    let mid = |i: usize| (x.h[i] + x.l[i]) / 2.0;
    // BASE 2: 1 Bull (enter inside a bull period, exit on its exit date), 2 Bear (enter outside, exit when one starts)
    let base_mode = g("base2").round() as i64;
    let (inside, base_end) = if base_mode != 0 { base2_periods(x, BASE2_ENTRY, BASE2_EXIT) } else { (Vec::new(), Vec::new()) };
    let base_ok = |i: usize| match base_mode {
        1 => inside[i],
        2 => !inside[i],
        _ => true,
    };

    let mut trades = Vec::new();
    // AFL candle flags: N Candle counts every bar, Green Candle a close above the open, UpClose a close above a non-zero previous close
    let counts = |i: usize| match candle {
        1 => true,
        2 => x.c[i] > x.o[i],
        _ => x.c[i] > x.c[i - 1] && x.c[i - 1] != 0.0,
    };
    let candle_word = match candle {
        1 => "candle",
        2 => "green candle",
        _ => "up-close candle",
    };
    let mut cur: Option<(Trade, f64, usize, usize, Dual, i64)> = None; // trade, hard-SL level, bars held, counted candles, ATR state, weekday-exit date
    let mut exit_bar = usize::MAX;
    for i in from.max(1)..=to {
        if let Some((mut t, hsl, held, ups, mut d, wd_date)) = cur.take() {
            let held = held + 1;
            let ups = ups + usize::from(counts(i));
            let (prev_stop, prev_fav) = (d.stop, d.fav_stop);
            if atr_exit {
                d.stop = (mid(i) - atr1[i] * m1).max(d.stop);
                if let Some(up) = d.upper {
                    let up = (mid(i) + atr2[i] * m2).min(up);
                    d.upper = Some(up);
                    if x.c[i] > up {
                        d.favour = true;
                        d.upper = None;
                    }
                }
                d.fav_stop = (mid(i) - atr2[i] * m2).max(d.fav_stop);
            }
            let sl_hit = hsl_on && hsl > 0.0 && held <= hsl_days && x.l[i] <= hsl;
            let why = if sl_hit {
                let fill = if x.o[i] <= hsl { x.o[i] * (1.0 - buffer) } else { hsl };
                Some((fill, format!("Hard SL {hsl_pct}%")))
            } else if base_mode == 1 && base_end[i] {
                Some((x.c[i], "BASE 2 period exit".into()))
            } else if base_mode == 2 && inside[i] && !inside[i - 1] {
                Some((x.c[i], "BASE 2 bull period starts".into()))
            } else if n_candle > 0 && ups >= n_candle {
                Some((x.c[i], format!("{n_candle} {candle_word}{}", if n_candle == 1 { "" } else { "s" })))
            } else if atr_exit && !d.favour && x.c[i] < prev_stop {
                Some((x.c[i], "Initial ATR stop".into()))
            } else if atr_exit && d.favour && x.c[i] < prev_fav {
                Some((x.c[i], "Favour ATR stop".into()))
            } else if wd_exit.is_some_and(|w| {
                // that day, or the first trading day after it; for Friday also the last trading day before it
                day(i) >= wd_date || w == 5 && i + 1 < x.r.len() && day(i + 1) > wd_date
            }) {
                Some((x.c[i], if wd_exit == Some(3) { "Wednesday exit".into() } else { "Friday exit".into() }))
            } else {
                None
            };
            match why {
                Some((px, reason)) => {
                    t.exit_i = i;
                    t.exit_px = px;
                    t.signal_i = i;
                    t.reason = reason;
                    trades.push(t);
                    exit_bar = i;
                }
                None => cur = Some((t, hsl, held, ups, d, wd_date)),
            }
            continue;
        }
        if i == to || i == exit_bar || !friday_monday(x, i) || !entry_signal(x, i, kind) || (kind2 != 0 && !entry_signal(x, i, kind2)) || !base_ok(i) {
            continue;
        }
        let dual = if atr_exit {
            let (a1, a2) = (atr1[i], atr2[i]);
            if !(a1.is_finite() && a2.is_finite()) {
                continue;
            }
            Dual { stop: mid(i) - a1 * m1, upper: Some(mid(i) + a2 * m2), fav_stop: mid(i) - a2 * m2, favour: false }
        } else {
            Dual { stop: f64::NAN, upper: None, fav_stop: f64::NAN, favour: false }
        };
        let px = x.c[i];
        let hsl = if hsl_on { (1.0 - hsl_pct / 100.0) * px } else { 0.0 };
        let t = Trade {
            dir: 1.0,
            legs: vec![Leg { i, px, w: 1.0 }],
            exit_i: i,
            exit_px: px,
            reason: String::new(),
            entry_kind: "close",
            signal_i: i,
            open: false,
            exits: Vec::new(),
        };
        // the first Wednesday / Friday after the entry date
        let wd_date = wd_exit.map_or(i64::MAX, |w| { let e = day(i); e + (w - weekday(e) + 6).rem_euclid(7) + 1 });
        cur = Some((t, hsl, 0, 0, dual, wd_date));
    }
    if let Some((mut t, ..)) = cur {
        t.exit_i = to;
        t.exit_px = x.c[to];
        // A position still held on the last bar of the selected period closes at that bar's close (team convention:
        // the AFL tradesheets end the last trade on the period's final bar). Entered on that bar: nothing to close.
        t.open = t.legs[0].i == to;
        t.reason = if t.open { "open".into() } else { "End of period".into() };
        t.signal_i = to;
        trades.push(t);
    }
    Some(trades)
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    (id == ID).then_some(("ytmr", "Monday Reversal", "Type 1"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;

    /// Consecutive weekdays from Thursday 2024-01-04; each bar is (open, high, low, close).
    fn bars(ohlc: &[(f64, f64, f64, f64)]) -> Vec<Row> {
        let mut day = day_number("2024-01-04").unwrap();
        ohlc.iter()
            .map(|&(open, high, low, close)| {
                let date = civil(day);
                day += if weekday(day) == 5 { 3 } else { 1 };
                Row { ticker: "T".into(), date, open, high, low, close, volume: None }
            })
            .collect()
    }
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
    fn ps(kv: &[(&str, f64)]) -> Params {
        kv.iter().map(|(k, v)| (k.to_string(), *v)).collect()
    }
    const FLAT: (f64, f64, f64, f64) = (100.0, 101.0, 99.0, 100.0);

    #[test]
    fn calendar_helpers() {
        assert_eq!(weekday(day_number("2024-01-08").unwrap()), 1); // Monday
        assert_eq!(weekday(day_number("2024-01-05").unwrap()), 5); // Friday
        assert_eq!(civil(day_number("2009-04-29").unwrap()), "2009-04-29");
        let r = bars(&[FLAT, FLAT, FLAT]);
        assert_eq!((r[1].date.as_str(), r[2].date.as_str()), ("2024-01-05", "2024-01-08"));
    }

    /// Thu, Fri (open 110), Mon (high 105 < 110: condition 1) -> buy Monday close 102; Tue closes up -> N=1 exit.
    #[test]
    fn friday_open_above_monday_high_enters_and_upclose_exits() {
        let r = bars(&[FLAT, (110.0, 111.0, 104.0, 106.0), (104.0, 105.0, 100.0, 102.0), (102.0, 104.0, 101.0, 103.0), FLAT]);
        let x = Ctx::new(&r, &r, &[]);
        let t = run(ID, &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert_eq!(t.len(), 1);
        assert_eq!((t[0].legs[0].i, t[0].legs[0].px, t[0].exit_i, t[0].exit_px, t[0].open), (2, 102.0, 3, 103.0, false));
        // condition 5 (Friday high 111 > Monday high 105) also fires; condition 4 (Friday close 106 > Monday close 102) too
        for kind in [4.0, 5.0] {
            assert_eq!(run(ID, &x, 0, r.len() - 1, &ps(&[("entry", kind)])).unwrap()[0].legs[0].i, 2);
        }
        // condition 3: Friday low 104 > Monday close 102 fires; condition 2: Friday low 104 > Monday low 100 fires
        assert_eq!(run(ID, &x, 0, r.len() - 1, &ps(&[("entry", 3.0)])).unwrap().len(), 1);
        assert_eq!(run(ID, &x, 0, r.len() - 1, &ps(&[("entry", 2.0)])).unwrap().len(), 1);
    }

    #[test]
    fn conditions_are_strict_and_independent() {
        // Friday open 105 == Monday high 105: condition 1 must not fire (strict >)
        let r = bars(&[FLAT, (105.0, 106.0, 100.0, 101.0), (101.0, 105.0, 100.5, 102.0), FLAT]);
        let x = Ctx::new(&r, &r, &[]);
        assert!(run(ID, &x, 0, r.len() - 1, &Params::new()).unwrap().is_empty());
        // condition 2 needs Friday low 100 > Monday low 100.5: false
        assert!(run(ID, &x, 0, r.len() - 1, &ps(&[("entry", 2.0)])).unwrap().is_empty());
        // condition 5: Friday high 106 > Monday high 105 fires
        assert_eq!(run(ID, &x, 0, r.len() - 1, &ps(&[("entry", 5.0)])).unwrap().len(), 1);
    }

    /// A Saturday session between Friday and Monday breaks the pair.
    #[test]
    fn trading_day_between_friday_and_monday_skips_the_pair() {
        let mut r = bars(&[FLAT, (110.0, 111.0, 104.0, 106.0), (104.0, 105.0, 100.0, 102.0), (102.0, 104.0, 101.0, 103.0)]);
        let mut sat = r[1].clone();
        sat.date = "2024-01-06".into();
        r.insert(2, sat);
        let x = Ctx::new(&r, &r, &[]);
        assert!(!friday_monday(&x, 3));
        assert!(run(ID, &x, 0, r.len() - 1, &ps(&[("entry", 5.0)])).unwrap().is_empty());
        // and a Monday holiday (Friday -> Tuesday) is not a pair either
        let mut r = bars(&[FLAT, (110.0, 111.0, 104.0, 106.0), (104.0, 105.0, 100.0, 102.0), FLAT]);
        r[2].date = "2024-01-09".into();
        let x = Ctx::new(&r, &r, &[]);
        assert!(!friday_monday(&x, 2));
    }

    /// No entry on the bar a trade exits (AFL i != exitIndex): exit on Monday blocks that Monday's signal.
    #[test]
    fn skip_entry_on_exit_bar() {
        // Fri 1 / Mon 1 enter; Tue..Fri flat (no up-close); Mon 2 closes up -> exit there, and Mon 2 also has a signal
        let sig_fri = (110.0, 111.0, 104.0, 106.0);
        let r = bars(&[FLAT, sig_fri, (104.0, 105.0, 100.0, 102.0), (102.0, 102.0, 101.0, 102.0), (102.0, 102.0, 101.0, 102.0),
            (102.0, 102.0, 101.0, 102.0), (120.0, 121.0, 101.0, 102.0), (101.0, 104.0, 100.0, 103.0), (103.0, 104.0, 102.0, 104.0)]);
        let x = Ctx::new(&r, &r, &[]);
        assert!(friday_monday(&x, 7) && entry_signal(&x, 7, 1));
        let t = run(ID, &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert_eq!(t.len(), 1);
        assert_eq!((t[0].legs[0].i, t[0].exit_i), (2, 7));
    }

    /// Entry condition 2 off by default; when on, both conditions must hold (condition 1 AND condition 2).
    #[test]
    fn second_entry_condition_requires_both() {
        // Fri (O 105, H 106, L 100, C 101), Mon (O 101, H 105, L 100.5, C 102): 1 false, 2 false, 3 false, 4 false, 5 true
        let r = bars(&[FLAT, (105.0, 106.0, 100.0, 101.0), (101.0, 105.0, 100.5, 102.0), FLAT]);
        let x = Ctx::new(&r, &r, &[]);
        let n = |kv: &[(&str, f64)]| run(ID, &x, 0, r.len() - 1, &ps(kv)).unwrap().len();
        assert_eq!(n(&[("entry", 5.0)]), 1);
        assert_eq!(n(&[("entry", 5.0), ("entry2", 0.0)]), 1);
        assert_eq!(n(&[("entry", 5.0), ("entry2", 5.0)]), 1);
        assert_eq!(n(&[("entry", 5.0), ("entry2", 2.0)]), 0);
        assert_eq!(n(&[("entry", 2.0), ("entry2", 5.0)]), 0);
        // Fri (O 110, H 111, L 104, C 106), Mon (H 105, L 100, C 102): all five hold, so any pair enters
        let r = bars(&[FLAT, (110.0, 111.0, 104.0, 106.0), (104.0, 105.0, 100.0, 102.0), (102.0, 104.0, 101.0, 103.0), FLAT]);
        let x = Ctx::new(&r, &r, &[]);
        assert_eq!(run(ID, &x, 0, r.len() - 1, &ps(&[("entry", 1.0), ("entry2", 3.0)])).unwrap()[0].legs[0].i, 2);
    }

    #[test]
    fn upclose_count_n() {
        // after entry: down, up, flat, up -> N=2 exits on the second up close (bar 6)
        let r = bars(&[FLAT, (110.0, 111.0, 104.0, 106.0), (104.0, 105.0, 100.0, 102.0), (101.0, 102.0, 100.0, 101.0),
            (101.0, 103.0, 100.0, 102.0), (102.0, 103.0, 101.0, 102.0), (102.0, 104.0, 101.0, 103.0), FLAT]);
        let x = Ctx::new(&r, &r, &[]);
        let t = run(ID, &x, 0, r.len() - 1, &ps(&[("upclose_n", 2.0)])).unwrap();
        assert_eq!((t[0].exit_i, t[0].exit_px), (6, 103.0));
    }

    #[test]
    fn hard_sl_fills_at_stop_or_gapped_open() {
        let entry = [FLAT, (110.0, 111.0, 104.0, 106.0), (104.0, 105.0, 100.0, 100.0)];
        // Tuesday trades down to 94 (5% stop = 95), opens above it: fill 95
        let mut b = entry.to_vec();
        b.extend([(99.0, 99.0, 94.0, 96.0), FLAT]);
        let x_rows = bars(&b);
        let x = Ctx::new(&x_rows, &x_rows, &[]);
        let t = run(ID, &x, 0, x_rows.len() - 1, &ps(&[("hsl_on", 1.0)])).unwrap();
        assert_eq!((t[0].exit_i, t[0].exit_px, t[0].reason.as_str()), (3, 95.0, "Hard SL 5%"));
        // opens at 90, below the stop: fill 90 x (1 - 0.25%)
        let mut b = entry.to_vec();
        b.extend([(90.0, 92.0, 89.0, 91.0), FLAT]);
        let r = bars(&b);
        let x = Ctx::new(&r, &r, &[]);
        let t = run(ID, &x, 0, r.len() - 1, &ps(&[("hsl_on", 1.0)])).unwrap();
        assert!((t[0].exit_px - 90.0 * 0.9975).abs() < 1e-9);
        // off by default: the same bar does not stop out
        let mut b = entry.to_vec();
        b.extend([(90.0, 92.0, 89.0, 91.0), (91.0, 91.0, 88.0, 89.0), (89.0, 89.0, 87.0, 88.0)]);
        let r = bars(&b);
        let x = Ctx::new(&r, &r, &[]);
        // Hard SL only for the first N bars: a later breach is ignored
        let t = run(ID, &x, 0, r.len() - 1, &ps(&[("hsl_on", 1.0), ("hsl_pct", 11.5), ("upclose_n", 10.0)])).unwrap();
        // stop 88.5: bar 3 low 89 stays above it; bar 4 low 88 is past the 1-bar window, so the trade runs to the end
        assert_eq!((t[0].exit_i, t[0].exit_px, t[0].open, t[0].reason.as_str()), (5, 88.0, false, "End of period"));
    }

    #[test]
    fn dual_atr_initial_and_favour_stops() {
        // Thu..Thu flat (true range 2), then Friday (bar 6) and Monday (bar 7)
        let mut b = vec![FLAT; 6];
        b.push((110.0, 111.0, 104.0, 106.0)); // Friday: TR = max(7, |111-100|, |104-100|) = 11
        b.push((104.0, 105.0, 103.0, 104.0)); // Monday: TR = max(2, |105-106|, |103-106|) = 3
        let r0 = bars(&b);
        let x0 = Ctx::new(&r0, &r0, &[]);
        assert!(friday_monday(&x0, 7));
        let mon_atr = avg_true_range(&x0, 5)[7];
        assert!((mon_atr - (2.0 + 2.0 + 2.0 + 11.0 + 3.0) / 5.0).abs() < 1e-12);
        // Initial stop = mid 104 - ATR 4 x 2 = 96; Tuesday closes at 95 -> exit at that close
        b.push((103.0, 103.5, 94.0, 95.0));
        b.push(FLAT);
        let r = bars(&b);
        let x = Ctx::new(&r, &r, &[]);
        let t = run(ID, &x, 0, r.len() - 1, &ps(&[("atr_on", 1.0), ("upclose_n", 0.0)])).unwrap();
        assert_eq!((t[0].legs[0].i, t[0].exit_i, t[0].exit_px, t[0].reason.as_str()), (7, 8, 95.0, "Initial ATR stop"));
        // Upper favour band = 104 + 4 x 2 = 112 (only lowered). Close 114 > 112 switches to the favour stop
        // (max(mid 109.5 - ATR 5.8 x 2, 96) = 97.9); the next close 91 < 97.9 exits.
        let mut b2 = b[..8].to_vec();
        b2.push((104.0, 115.0, 104.0, 114.0));
        b2.push((112.0, 112.0, 90.0, 91.0));
        let r = bars(&b2);
        let x = Ctx::new(&r, &r, &[]);
        let t = run(ID, &x, 0, r.len() - 1, &ps(&[("atr_on", 1.0), ("upclose_n", 0.0)])).unwrap();
        assert_eq!((t[0].legs[0].i, t[0].exit_i, t[0].reason.as_str()), (7, 9, "Favour ATR stop"));
        // bar 8 closed up, but the Candle exit is off (N = 0); with it on as well, that up close exits first
        let t = run(ID, &x, 0, r.len() - 1, &ps(&[("atr_on", 1.0)])).unwrap();
        assert_eq!((t[0].exit_i, t[0].reason.as_str()), (8, "1 up-close candle"));
    }

    /// The Candle and Dual ATR exits are independent switches: both on, whichever fires first closes the trade.
    #[test]
    fn candle_and_atr_exits_combine() {
        let mut b = vec![FLAT; 6];
        b.push((110.0, 111.0, 104.0, 106.0));
        b.push((104.0, 105.0, 103.0, 104.0));
        b.push((103.0, 103.5, 94.0, 95.0)); // close 95 < initial stop 96, and a down close
        b.push((95.0, 97.0, 94.0, 96.0)); // up close
        b.push(FLAT);
        let r = bars(&b);
        let x = Ctx::new(&r, &r, &[]);
        // Candle exit alone waits for the up close on bar 9; with the Dual ATR exit on, the stop exits on bar 8
        let t = run(ID, &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert_eq!((t[0].exit_i, t[0].reason.as_str()), (9, "1 up-close candle"));
        let t = run(ID, &x, 0, r.len() - 1, &ps(&[("atr_on", 1.0)])).unwrap();
        assert_eq!((t[0].exit_i, t[0].reason.as_str()), (8, "Initial ATR stop"));
        // both off: the trade runs to the end of the period
        let t = run(ID, &x, 0, r.len() - 1, &ps(&[("upclose_n", 0.0)])).unwrap();
        assert_eq!((t[0].exit_i, t[0].reason.as_str()), (10, "End of period"));
        // the Candle exit switch off does the same as N = 0
        let t = run(ID, &x, 0, r.len() - 1, &ps(&[("nc_on", 0.0)])).unwrap();
        assert_eq!((t[0].exit_i, t[0].reason.as_str()), (10, "End of period"));
    }

    /// Weekdays from a Thursday; bars `sig` + 1 (Friday) and `sig` + 2 (Monday) carry the condition-1 signal, the rest are flat.
    fn dated(start: &str, len: usize, sig: &[usize]) -> Vec<Row> {
        let mut day = day_number(start).unwrap();
        (0..len)
            .map(|k| {
                let (open, high, low, close) = if sig.iter().any(|&s| k == s + 1) {
                    (110.0, 111.0, 104.0, 106.0)
                } else if sig.iter().any(|&s| k == s + 2) {
                    (104.0, 105.0, 100.0, 102.0)
                } else {
                    FLAT
                };
                let date = civil(day);
                day += if weekday(day) == 5 { 3 } else { 1 };
                Row { ticker: "T".into(), date, open, high, low, close, volume: None }
            })
            .collect()
    }

    /// BASE 2 Bull: 2008-01-01..2008-01-31 is a bull period; Bear: entries only outside one, exit when 2008-11-21 starts one.
    #[test]
    fn base2_bull_and_bear() {
        // Mon 2008-01-28 (bar 2) is inside the period, Mon 2008-02-04 (bar 7) after it; Thu 2008-01-31 (bar 5) is its exit date
        let r = dated("2008-01-24", 12, &[0, 5]);
        assert_eq!((r[2].date.as_str(), r[5].date.as_str(), r[7].date.as_str()), ("2008-01-28", "2008-01-31", "2008-02-04"));
        let x = Ctx::new(&r, &r, &[]);
        let go = |kv: &[(&str, f64)]| -> Vec<(usize, usize, String)> {
            run(ID, &x, 0, r.len() - 1, &ps(kv)).unwrap().into_iter().map(|t| (t.legs[0].i, t.exit_i, t.reason)).collect()
        };
        assert_eq!(go(&[("upclose_n", 0.0)]), vec![(2, 11, "End of period".into())]);
        assert_eq!(go(&[("upclose_n", 0.0), ("base2", 1.0)]), vec![(2, 5, "BASE 2 period exit".into())]);
        assert_eq!(go(&[("upclose_n", 0.0), ("base2", 2.0)]), vec![(7, 11, "End of period".into())]);
        // Bear: Mon 2008-11-17 is outside every period; Fri 2008-11-21 (bar 6) starts one and closes the trade
        let r = dated("2008-11-13", 9, &[0]);
        assert_eq!(r[6].date, "2008-11-21");
        let x = Ctx::new(&r, &r, &[]);
        let t = run(ID, &x, 0, r.len() - 1, &ps(&[("upclose_n", 0.0), ("base2", 2.0)])).unwrap();
        assert_eq!((t[0].legs[0].i, t[0].exit_i, t[0].reason.as_str()), (2, 6, "BASE 2 bull period starts"));
    }

    /// Weekday exit: Wednesday (holiday: next trading day) or Friday (holiday: the trading day before).
    #[test]
    fn weekday_exit_wednesday_and_friday() {
        // Thu 2024-01-04, Fri 01-05, Mon 01-08 (entry, bar 2), Tue 01-09 .. Fri 01-12, Mon 01-15
        let r = dated("2024-01-04", 8, &[0]);
        let x = Ctx::new(&r, &r, &[]);
        let go = |r: &[Row], kv: &[(&str, f64)]| {
            let x = Ctx::new(r, r, &[]);
            let t = run(ID, &x, 0, r.len() - 1, &ps(&[&[("upclose_n", 0.0), ("wd_on", 1.0)], kv].concat())).unwrap();
            (r[t[0].exit_i].date.clone(), t[0].reason.clone())
        };
        // off by default: runs to the end
        assert_eq!(run(ID, &x, 0, r.len() - 1, &ps(&[("upclose_n", 0.0)])).unwrap()[0].reason, "End of period");
        assert_eq!(go(&r, &[]), ("2024-01-10".into(), "Wednesday exit".into()));
        assert_eq!(go(&r, &[("wd_day", 2.0)]), ("2024-01-12".into(), "Friday exit".into()));
        // Wednesday 01-10 a holiday: the next trading day (Thursday)
        let mut h = r.clone();
        h.remove(4);
        assert_eq!(go(&h, &[]), ("2024-01-11".into(), "Wednesday exit".into()));
        // Wednesday and Thursday holidays: Friday
        h.remove(4);
        assert_eq!(go(&h, &[]), ("2024-01-12".into(), "Wednesday exit".into()));
        // Friday 01-12 a holiday: the trading day before (Thursday); Thursday too: Wednesday
        let mut h = r.clone();
        h.remove(6);
        assert_eq!(go(&h, &[("wd_day", 2.0)]), ("2024-01-11".into(), "Friday exit".into()));
        h.remove(5);
        assert_eq!(go(&h, &[("wd_day", 2.0)]), ("2024-01-10".into(), "Friday exit".into()));
        // Tuesday to Friday all holidays: the next trading day (Monday 01-15)
        let mut h = r.clone();
        h.drain(3..7);
        assert_eq!(go(&h, &[("wd_day", 2.0)]), ("2024-01-15".into(), "Friday exit".into()));
        // an earlier exit wins: the up close on Tuesday exits before Wednesday
        let mut h = r.clone();
        h[3].close = 103.0;
        assert_eq!(go(&h, &[("upclose_n", 1.0)]), ("2024-01-09".into(), "1 up-close candle".into()));
    }

    #[test]
    fn candle_types_n_candle_and_green() {
        // after entry: green down close (O 100 C 101 < prev 102), red up close (O 104 C 103 > 101), flat
        let r = bars(&[FLAT, (110.0, 111.0, 104.0, 106.0), (104.0, 105.0, 100.0, 102.0), (100.0, 102.0, 99.0, 101.0),
            (104.0, 105.0, 102.0, 103.0), FLAT, FLAT]);
        let x = Ctx::new(&r, &r, &[]);
        let exit = |kv: &[(&str, f64)]| { let t = run(ID, &x, 0, r.len() - 1, &ps(kv)).unwrap(); (t[0].exit_i, t[0].reason.clone()) };
        assert_eq!(exit(&[]), (4, "1 up-close candle".into()));
        assert_eq!(exit(&[("candle_type", 2.0)]), (3, "1 green candle".into()));
        assert_eq!(exit(&[("candle_type", 1.0), ("upclose_n", 3.0)]), (5, "3 candles".into()));
    }
}
