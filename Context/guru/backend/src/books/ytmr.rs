//! YT strategy "Monday Reversal, Type 1" (not from a book). Source: the team's AmiBroker AFL
//! "Buy_on_Monday_Base2_ATR (2)_Modified.afl" (Buy on Monday, Base 2 Filter + Dual ATR Exit), reduced to the rules the
//! team specified for the YT Strategies tab.
//!
//! Conventions: daily bars, long only, one position at a time, signals read on the close of bar i with data <= i.
//! A Friday/Monday pair is the Monday bar whose previous bar is the Friday three calendar days earlier; any trading day
//! between them (e.g. a Saturday session) or a missing Friday/Monday skips the pair. Entry at the Monday close.
//! Exits are checked from the bar after entry: the Hard SL first (fills at the stop, or at the open less the buffer
//! when the open gaps through it), then the selected exit and the Days exit at the close. No new entry on an exit bar
//! (AFL `i != exitIndex`). Dropped from the AFL: Profit Target, SuperTrend/ATHC filters, Base 2 dates, candle colour,
//! upper-circuit, penny and turnover filters, weekly/monthly ATR timeframes and the "N Candles Exit Only" counter.
#![allow(dead_code)]
use crate::connors::{true_range, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "yt_mr";
pub const ID: &str = "ytmr_monday_reversal_t1";

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
Exit type: 1 = N up-close candles (exit at the close of the Nth bar after entry that closes above the previous close); 2 = Dual ATR on daily bars. \
ATR = simple average of the 1-bar true range over the period (AFL sum of ATR(1)); reference price (High+Low)/2. Initial stop = mid - ATR x mult, raised only; \
favour band upper = mid + ATR' x mult', lowered only, and once a close exceeds it the favour stop mid - ATR' x mult' (raised only) replaces the initial stop. \
A close below the previous bar's active stop exits at that close. Entries wait until the ATR values exist. \
Hard SL (off by default): long stop pct% below the entry close, active for the first N bars after entry; fills at the stop, or at the open less the buffer % when the open is at or below it. \
Days exit (off by default): exit at the close of the Nth bar after entry. \
Dropped from the AFL: Profit Target, SuperTrend/ATHC filters, Base 2 dates, candle colour, upper-circuit/penny/turnover filters, weekly/monthly ATR timeframes. Results are simulated on the selected instrument; no published figures exist for this strategy.";

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
            "E|Buy at the Monday close; no entry on a bar where a trade exited",
            "X|Exit type 1: exit at the close of the Nth up-close candle after entry (close above the previous close)",
            "X|Exit type 2: Dual ATR trailing stop on daily bars; exit at the close when the close falls below the active stop",
            "X|Optional: Hard SL % for the first N bars after entry; Days exit after N bars",
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
        p("exit_type", "Exit type (1 = N up-close candles, 2 = Dual ATR)", 1.0, 1.0, 2.0, 1.0),
        p("upclose_n", "Exit: N up-close candles", 1.0, 1.0, 10.0, 1.0),
        p("atr_n", "Initial ATR period", 5.0, 5.0, 15.0, 5.0),
        p("atr_mult", "Initial ATR multiplier", 2.0, 1.0, 6.0, 1.0),
        p("fav_n", "Favour ATR period", 5.0, 5.0, 15.0, 5.0),
        p("fav_mult", "Favour ATR multiplier", 2.0, 1.0, 8.0, 1.0),
        p("hsl_on", "Hard SL (0 = off, 1 = on)", 0.0, 0.0, 1.0, 1.0),
        p("hsl_pct", "Hard SL %", 5.0, 0.5, 20.0, 0.5),
        p("hsl_days", "Hard SL active for N bars", 1.0, 1.0, 4.0, 1.0),
        p("sl_buffer", "Gap fill buffer % (open through the SL)", 0.25, 0.0, 10.0, 0.05),
        p("days_on", "Days exit (0 = off, 1 = on)", 0.0, 0.0, 1.0, 1.0),
        p("days_n", "Days exit after N bars", 2.0, 1.0, 10.0, 1.0),
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
    let atr_exit = g("exit_type").round() as i64 == 2;
    let n_up = u("upclose_n");
    let (m1, m2) = (g("atr_mult"), g("fav_mult"));
    let (hsl_on, hsl_pct, hsl_days, buffer) = (g("hsl_on") >= 0.5, g("hsl_pct"), u("hsl_days"), g("sl_buffer") / 100.0);
    let (days_on, days_n) = (g("days_on") >= 0.5, u("days_n"));
    let (atr1, atr2) = if atr_exit { (avg_true_range(x, u("atr_n")), avg_true_range(x, u("fav_n"))) } else { (Vec::new(), Vec::new()) };
    let mid = |i: usize| (x.h[i] + x.l[i]) / 2.0;

    let mut trades = Vec::new();
    let mut cur: Option<(Trade, f64, usize, usize, Dual)> = None; // trade, hard-SL level, bars held, up-closes, ATR state
    let mut exit_bar = usize::MAX;
    for i in from.max(1)..=to {
        if let Some((mut t, hsl, held, ups, mut d)) = cur.take() {
            let held = held + 1;
            let ups = ups + usize::from(x.c[i] > x.c[i - 1] && x.c[i - 1] != 0.0);
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
            } else if !atr_exit && ups >= n_up {
                Some((x.c[i], format!("{n_up} up-close candle{}", if n_up == 1 { "" } else { "s" })))
            } else if atr_exit && !d.favour && x.c[i] < prev_stop {
                Some((x.c[i], "Initial ATR stop".into()))
            } else if atr_exit && d.favour && x.c[i] < prev_fav {
                Some((x.c[i], "Favour ATR stop".into()))
            } else if days_on && held >= days_n {
                Some((x.c[i], format!("Days exit ({days_n} bars)")))
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
                None => cur = Some((t, hsl, held, ups, d)),
            }
            continue;
        }
        if i == to || i == exit_bar || !friday_monday(x, i) || !entry_signal(x, i, kind) || (kind2 != 0 && !entry_signal(x, i, kind2)) {
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
        cur = Some((t, hsl, 0, 0, dual));
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
    fn hard_sl_fills_at_stop_or_gapped_open_and_days_exit() {
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
        // off by default: the same bar does not stop out; with Days exit 2 the trade leaves on bar 4's close
        let mut b = entry.to_vec();
        b.extend([(90.0, 92.0, 89.0, 91.0), (91.0, 91.0, 88.0, 89.0), (89.0, 89.0, 87.0, 88.0)]);
        let r = bars(&b);
        let x = Ctx::new(&r, &r, &[]);
        let t = run(ID, &x, 0, r.len() - 1, &ps(&[("days_on", 1.0), ("upclose_n", 10.0)])).unwrap();
        assert_eq!((t[0].exit_i, t[0].exit_px), (4, 89.0));
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
        let t = run(ID, &x, 0, r.len() - 1, &ps(&[("exit_type", 2.0)])).unwrap();
        assert_eq!((t[0].legs[0].i, t[0].exit_i, t[0].exit_px, t[0].reason.as_str()), (7, 8, 95.0, "Initial ATR stop"));
        // Upper favour band = 104 + 4 x 2 = 112 (only lowered). Close 114 > 112 switches to the favour stop
        // (max(mid 109.5 - ATR 5.8 x 2, 96) = 97.9); the next close 91 < 97.9 exits.
        let mut b2 = b[..8].to_vec();
        b2.push((104.0, 115.0, 104.0, 114.0));
        b2.push((112.0, 112.0, 90.0, 91.0));
        let r = bars(&b2);
        let x = Ctx::new(&r, &r, &[]);
        let t = run(ID, &x, 0, r.len() - 1, &ps(&[("exit_type", 2.0)])).unwrap();
        assert_eq!((t[0].legs[0].i, t[0].exit_i, t[0].reason.as_str()), (7, 9, "Favour ATR stop"));
        // bar 8 closed up, but up-close candles do not exit in ATR mode
    }
}
