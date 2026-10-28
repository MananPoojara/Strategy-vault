//! YT Strategies · Negative Expiry: the team's AmiBroker AFLs in "Old Topics/Negative Expiry" that trade an index on
//! daily bars ("ST 52 54": entry Supertrend 5/2, exit Supertrend 5/4, long only). Coded from the AFLs as written, with
//! the AFLs' own default settings; there is no book PDF. The "Stock Filter" AFLs that rank a stock universe are not
//! here (they need actual stocks); the one "Stock Filter" AFL kept here (re-entry until minimum profit) filters on the
//! traded symbol's own bars and was run on the spot indices.
//!
//! AmiBroker behaviour follows `books::ytsw` (fills at the signal bar's close, ExRem resetting on the same bar, exit
//! then entry on one bar, expandLast for higher time frames, ATR(1) Null on the first bar). Further points from these
//! AFLs:
//! - expiry days are derived from the calendar by the AFLs' `Lastthursday()` function, not from an NSE expiry list:
//!   days in month = 31 for Jan/Mar/May/Jul/Aug/Oct/Dec, 30 otherwise, February 29 only when the year is divisible
//!   by 4 and not by 100 (so February 2000 counts as 28 days); a Thursday within the month's last 7 days is an
//!   expiry, and a Wednesday in the last 7-8 days is one when the next bar's weekday shows the Thursday was a holiday;
//!   that Wednesday test reads the next bar (one bar of look-ahead);
//! - month end = the last trading day of the month, found from the NEXT bar's day number (one bar of look-ahead);
//!   the 5-day AFL takes the bar `entry day` bars before it (`Ref(eom, entryday)`, entryday + 1 bars of look-ahead);
//! - the circuit checks use `Prec(x, 2)`, which truncates to two decimals.
#![allow(dead_code)]
use crate::books::ytmr::day_number;
use crate::books::ytsw::{backtest, sma_filter, st_filter, supertrend, Sig};
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "yt_ne";
pub const EXPIRY: &str = "ytne_expiry";
pub const MONTH_END: &str = "ytne_month_end";
pub const EXPIRY_FLT: &str = "ytne_expiry_index_filter";
pub const MONTH_END_FLT: &str = "ytne_month_end_index_filter";
pub const FIVE_DAY_FLT: &str = "ytne_five_day_index_filter";
pub const REENTRY: &str = "ytne_expiry_reentry_min_profit";
pub const IDS: [&str; 6] = [EXPIRY, MONTH_END, EXPIRY_FLT, MONTH_END_FLT, FIVE_DAY_FLT, REENTRY];

fn def(id: &'static str, name: &'static str, rules: &'static [&'static str], notes: &'static str) -> StrategyDef {
    StrategyDef { id, name, group: "YT Strategies · Negative Expiry", session: 0, dir: "Long", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim: None }
}

const ST_E: &str = "E|Buy at the close while that condition holds and the entry Supertrend (period 5, factor 2, on the mid point) is up";
const ST_X: &str = "X|Sell at the close when the entry Supertrend and the exit Supertrend (period 5, factor 4) are both down";
const EXP_V1: &str = "F|Expiry day from the calendar: a Thursday in the month's last 7 days, or a Wednesday in its last 8 days (not the 30th) when the next bar is a Friday";
const EXP_V2: &str = "F|Expiry day from the calendar: 29 Apr 2009 and 28 Mar 2018 (hard-coded), a Thursday in the month's last 7 days, or a Wednesday in its last 8 days when the next bar is a Friday or Monday of the same month";
const EOM: &str = "F|Month end: the last trading day of the month (read from the next bar's date)";
const FLT: &str = "F|Optional market filter (off by default): Supertrend (5, 1, mid point) or SMA (200) of the traded index on monthly (default), weekly or daily bars; entries only while it is up";
pub fn catalogue() -> Vec<StrategyDef> {
    vec![
        def(EXPIRY, "Negative Expiry ST 5/2-5/4", &[
            EXP_V1,
            "F|The latest expiry close is more than 1% below the previous expiry's close (holds until the next expiry)",
            ST_E, ST_X,
        ], "st 52 54 long only with expiry day close (Negative Expiry / Without filter / ST 52 54 long only when EXPIRY day close is negative), coded as the AFL is written. This AFL's Wednesday fallback needs the next bar to be a Friday and skips the 30th; no hard-coded dates. Before the second expiry both reference closes are the first bar's close, so no signal. Expiry days come from the AFL's calendar rule (no NSE expiry list), with its February rule (Feb 2000 counted as 28 days); the Wednesday fallback reads the next bar (one bar of look-ahead). Supertrend: the team's loop (band ATR = mean of the last 5 ATR(1) values); the '3DAYS MA' calculation point falls back to the mid point (the AFL compares it against the misspelt '3DYAS MA')."),
        def(MONTH_END, "Negative Month-End ST 5/2-5/4", &[
            EOM,
            "F|The latest month-end close is more than 1% below the previous month-end close (holds until the next month end)",
            ST_E, ST_X,
        ], "st 52 54 long only with monthly close (Negative Expiry / Without filter / ST 52 54 long only when MONTHLY day close is negative), coded as the AFL is written. The month end is found from the next bar's day number (one bar of look-ahead; the last bar of the data is never a month end). The % difference Param is declared twice; the second (default 1, minimum 0.05) is the one AmiBroker uses. Supertrend: the team's loop; the '3DAYS MA' calculation point falls back to the mid point (misspelt comparison in the AFL)."),
        def(EXPIRY_FLT, "Negative Expiry ST 5/2-5/4 with Index Filter", &[
            EXP_V2,
            "F|The latest expiry close is more than 1% below the previous expiry's close, and the previous expiry close is no longer the first bar's close",
            FLT, ST_E,
            "X|Sell at the close when the entry and exit Supertrends are both down, or when the market filter is down",
        ], "st 52 54 long only with expiry day close WITH INDEX FLT - Copy (3) (Negative Expiry / Index Filter), coded as the AFL is written. Expiry days come from the AFL's calendar rule, including its hard-coded dates (29 Apr 2009, 28 Mar 2018) and February rule (Feb 2000 counted as 28 days); the Wednesday fallback reads the next bar (one bar of look-ahead). The AFL's index filter is NONE by default; its NSENIFTY / NSE500 / BANKNIFTY choice reads another symbol, which the engine does not load, so when turned on the filter uses the traded index's own bars. The filter ST calculation point gives the pivot only when the DAILY point is the pivot (the AFL tests the wrong variable). No buy on an upper-circuit bar (open = high = close at two decimals) and no sell on a lower-circuit bar. Ranking, R-multiple and expectancy code does not change the signals of a single index."),
        def(MONTH_END_FLT, "Negative Month-End ST 5/2-5/4 with Index Filter", &[
            EOM,
            "F|The latest month-end close is more than 1% below the previous month-end close",
            FLT,
            "E|Buy at the close while that holds, the entry Supertrend (5, 2) is up and the filter is up, from the 32nd bar of the data",
            "X|Sell at the close when the entry and exit Supertrends are both down (the filter does not force a sell in this AFL)",
        ], "st 52 54 long only with monthly close WITH INDEX FLT (Negative Expiry / Index Filter), coded as the AFL is written: unlike the expiry and 5-day versions, its Sell does not include the filter, and it has no first-close guard but needs BarIndex() > 30. The month end is found from the next bar's day number (one bar of look-ahead). The AFL's index filter is NONE by default; its NSENIFTY / NSE500 / BANKNIFTY choice reads another symbol, which the engine does not load, so when turned on the filter uses the traded index's own bars (filter ST point quirk as in the AFL). No buy on an upper-circuit bar, no sell on a lower-circuit bar."),
        def(FIVE_DAY_FLT, "5 Days Before Month-End ST 5/2-5/4 with Index Filter", &[
            "F|Reference day: the bar 5 trading days before the month's last trading day (entry day 5)",
            "F|The latest reference-day close is more than 1% below the previous month's reference-day close",
            FLT,
            "E|Buy at the close while that holds, the entry Supertrend (5, 2) is up and the filter is up, from the 32nd bar of the data",
            "X|Sell at the close when the entry and exit Supertrends are both down, or when the market filter is down",
        ], "st 52 54 long only with 5 day phenomena WITH INDEX FLT (Negative Expiry / Index Filter), coded as the AFL is written. The reference day is Ref(eom, entryday): it is known only from the month end entryday bars later, and the month end itself from the bar after that, so the AFL looks ahead entryday + 1 bars (coded the same way, look-ahead included). The AFL's index filter is NONE by default; its NSENIFTY / NSE500 / BANKNIFTY choice reads another symbol, which the engine does not load, so when turned on the filter uses the traded index's own bars (filter ST point quirk as in the AFL). No buy on an upper-circuit bar, no sell on a lower-circuit bar."),
        def(REENTRY, "Negative Expiry ST 5/2-5/4, Re-entry until Minimum Profit", &[
            EXP_V2,
            "F|The latest expiry close is more than 1% below the previous expiry's close, and the previous expiry close is no longer the first bar's close",
            "F|Optional filter on the index's own bars (off by default): Supertrend (5, 1, mid point) or SMA (200), monthly, weekly or daily",
            "E|Buy at the close when the entry Supertrend (5, 2) is up and the filter is up, and either the expiry condition holds or the last trade of the AFL's own trade loop made less than the minimum profit (5%)",
            "X|Sell at the close when the entry and exit Supertrends are both down, or when the filter is down",
        ], "st 52 54 long only with expiry day close WITH stock FLT with probability (Negative Expiry / Stock Filter), coded as the AFL is written; the team ran it on the spot indices. The AFL first runs its own trade loop: after a loop trade closes below the minimum profit (net of its BROKERAGE %, default 0.65, which the AFL charges at the exit price on both legs), it may re-buy whenever the entry ST is up and the filter is up, without the expiry condition, until a trade reaches the minimum profit; the loop's re-entry does not check the upper circuit. The actual Buy then uses the loop's flag from the previous bar. Expiry days come from the AFL's calendar rule, including its hard-coded dates (29 Apr 2009, 28 Mar 2018) and February rule (Feb 2000 counted as 28 days); the Wednesday fallback reads the next bar (one bar of look-ahead). The filter here is the AFL's STOCK filter on the traded series itself (default NONE). No buy on an upper-circuit bar, no sell on a lower-circuit bar."),
    ]
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    if !IDS.contains(&id) {
        return None;
    }
    let mut v = vec![
        p("st_point", "Daily ST calculation point (0 mid point, 1 close, 2 pivot (H+L+C)/3, 3 average of mid point, pivot and close, 4 3-day MA: the AFL's misspelt test makes it the mid point, 5 5-day MA of the close)", 0.0, 0.0, 5.0, 1.0),
        p("entry_period", "ST period for entry", 5.0, 1.0, 20.0, 1.0),
        p("entry_factor", "ST factor for entry", 2.0, 1.0, 20.0, 1.0),
        p("exit_period", "ST period for exit", 5.0, 1.0, 20.0, 1.0),
        p("exit_factor", "ST factor for exit", 4.0, 1.0, 20.0, 1.0),
        p("diff", "% difference below the previous reference close", 1.0, if id == EXPIRY { 0.0 } else { 0.05 }, 20.0, 0.25),
    ];
    if id == FIVE_DAY_FLT {
        v.push(p("entry_day", "Entry day: reference bar this many trading days before the month's last trading day", 5.0, 1.0, 10.0, 1.0));
    }
    if id == REENTRY {
        v.push(p("min_profit", "Minimum profit % (re-entries continue until a trade reaches it)", 5.0, 1.0, 15.0, 1.0));
        v.push(p("comm", "Brokerage % used by the AFL's own trade loop", 0.65, 0.01, 1.0, 0.01));
    }
    if id != EXPIRY && id != MONTH_END {
        let what = if id == REENTRY { "Filter on the traded index's own bars (0 none, 1 on: the AFL's STOCK choice)" } else { "Index filter (0 none, 1 on: uses the traded index itself in place of the AFL's NSENIFTY / NSE500 / BANKNIFTY)" };
        v.extend([
            p("flt", what, 0.0, 0.0, 1.0, 1.0),
            p("flt_type", "Filter type (0 Supertrend, 1 SMA)", 0.0, 0.0, 1.0, 1.0),
            p("flt_tf", "Filter timeframe (0 monthly, 1 weekly, 2 daily)", 0.0, 0.0, 2.0, 1.0),
            p("flt_period", "Filter ST period", 5.0, 0.0, 20.0, 1.0),
            p("flt_factor", "Filter ST factor", 1.0, 0.0, 20.0, 1.0),
            p("flt_point", "Filter ST calculation point (0 mid point, 1 close, 2 pivot: gives the close unless the daily point is the pivot, as in the AFL)", 0.0, 0.0, 2.0, 1.0),
            p("sma", "Filter SMA period", 200.0, 1.0, 300.0, 1.0),
        ]);
    }
    Some(v)
}

// ---------------------------------------------------------------- calendar

fn ymd(date: &str) -> Option<(i64, i64, i64)> {
    Some((date.get(0..4)?.parse().ok()?, date.get(5..7)?.parse().ok()?, date.get(8..10)?.parse().ok()?))
}

/// AFL `DayOfWeek()`: 0 Sunday .. 6 Saturday (-1 for an unreadable date).
fn dow(date: &str) -> i64 {
    day_number(date).map_or(-1, |d| (d + 4).rem_euclid(7))
}

/// The AFL's days in month: 31 for months 1, 3, 5, 7, 8, 10, 12, otherwise 30; February 29 only when the year is
/// divisible by 4 and not by 100 (2000 gets 28).
pub(crate) fn afl_days_in_month(y: i64, m: i64) -> i64 {
    match m {
        2 => if y % 4 == 0 && y % 100 != 0 { 29 } else { 28 },
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        _ => 30,
    }
}

/// `Lastthursday()` as written. `v2` = the Index Filter / Stock Filter version (hard-coded 2009-04-29 and
/// 2018-03-28, Wednesday when the next bar is a Friday or Monday of the same month); otherwise the Without-filter
/// version (Wednesday, not the 30th, when the next bar is a Friday). The Wednesday tests read the next bar.
pub(crate) fn expiry_days(x: &Ctx, v2: bool) -> Vec<bool> {
    let n = x.c.len();
    (0..n)
        .map(|i| {
            let d = &x.r[i].date;
            let Some((y, m, day)) = ymd(d) else { return false };
            let left = afl_days_in_month(y, m) - day;
            let wd = dow(d);
            let next = if i + 1 < n { Some((dow(&x.r[i + 1].date), ymd(&x.r[i + 1].date).map(|t| t.1))) } else { None };
            if v2 {
                let hard = d.as_str() == "2009-04-29" || d.as_str() == "2018-03-28";
                let wed = left <= 7 && wd == 3 && next.map_or(false, |(nd, nm)| (nd == 5 || nd == 1) && nm == Some(m));
                hard || (left < 7 && wd == 4) || wed
            } else {
                let wed = left <= 7 && day != 30 && wd == 3 && next.map_or(false, |(nd, _)| nd == 5);
                (left < 7 && wd == 4) || wed
            }
        })
        .collect()
}

/// The AFLs' `eom`: from the next bar's day number (1, or 2 when today is not the 1st, ... up to 5); false on the
/// last bar.
pub(crate) fn month_end(x: &Ctx) -> Vec<bool> {
    let n = x.c.len();
    let day = |i: usize| ymd(&x.r[i].date).map_or(0, |t| t.2);
    (0..n)
        .map(|i| {
            if i + 1 >= n {
                return false;
            }
            let (nd, d) = (day(i + 1), day(i));
            (1..=5).contains(&nd) && !(1..nd).contains(&d)
        })
        .collect()
}

/// Reference closes loop: lastclose = lastclose1 = C; from `start`, on an event bar lastclose = C and lastclose1 =
/// the previous lastclose, otherwise both carry.
pub(crate) fn ref_closes(c: &[f64], ev: &[bool], start: usize) -> (Vec<f64>, Vec<f64>) {
    let (mut lc, mut lc1) = (c.to_vec(), c.to_vec());
    for i in start.max(1)..c.len() {
        if ev[i] {
            lc[i] = c[i];
            lc1[i] = lc[i - 1];
        } else {
            lc[i] = lc[i - 1];
            lc1[i] = lc1[i - 1];
        }
    }
    (lc, lc1)
}

// ---------------------------------------------------------------- indicators

/// Daily STCAL: 0 mid point, 1 close, 2 pivot, 3 (MP + pivot + C) / 3, 4 the mid point (misspelt list test), 5 MA(C, 5)
/// (Null, passed as NaN, for the first 4 bars: the ST bands hold their trend there).
fn stcal(x: &Ctx, kind: i64) -> Vec<f64> {
    let n = x.c.len();
    (0..n)
        .map(|i| {
            let (h, l, c) = (x.h[i], x.l[i], x.c[i]);
            match kind {
                1 => c,
                2 => (h + l + c) / 3.0,
                3 => ((h + l) / 2.0 + (h + l + c) / 3.0 + c) / 3.0,
                5 => if i >= 4 { x.c[i - 4..=i].iter().sum::<f64>() / 5.0 } else { f64::NAN },
                _ => (h + l) / 2.0,
            }
        })
        .collect()
}

/// AFL `Prec(v, 2)`: truncated to two decimals.
fn prec2(v: f64) -> f64 {
    (v * 100.0 + 1e-6).floor()
}

/// The market / own-symbol filter: (buy allowed, filter sell). Off: (true, false).
fn filter(x: &Ctx, g: &dyn Fn(&str) -> f64) -> (Vec<bool>, Vec<bool>) {
    let n = x.c.len();
    if g("flt").round() as i64 != 1 {
        return (vec![true; n], vec![false; n]);
    }
    let tf = match g("flt_tf").round() as i64 { 1 => 'W', 2 => 'D', _ => 'M' };
    if g("flt_type").round() as i64 == 1 {
        return sma_filter(x, tf, g("sma").round().max(1.0) as usize, false);
    }
    // IMSTCAL inside the filter block: mid point if chosen, else pivot when the DAILY point is the pivot, else close
    let point = if g("flt_point").round() as i64 == 0 { 1 } else if g("st_point").round() as i64 == 2 { 3 } else { 2 };
    let m = st_filter(x, tf, point, g("flt_period").round().max(1.0) as usize, g("flt_factor"));
    (m.iter().map(|&v| v == 1).collect(), m.iter().map(|&v| v == -1).collect())
}

/// The re-entry AFL's own trade loop; returns PROBSIG1 AND PROBSIG2 per bar (both mean: the last loop trade made
/// less than the minimum profit).
pub(crate) fn probsig(c: &[f64], dbuy: &[bool], dsell: &[bool], t1: &[i8], fb: &[bool], comm: f64, min_profit: f64) -> Vec<bool> {
    let n = c.len();
    let (mut p1, mut p2) = (vec![false; n], vec![false; n]);
    let mut count = 0;
    let mut bp = 0.0;
    for i in 1..n {
        if (dbuy[i] && count == 0) || (p1[i - 1] && p2[i - 1] && t1[i] == 1 && fb[i] && count == 0) {
            bp = c[i];
            count = 1;
            p1[i] = p1[i - 1];
            p2[i] = p2[i - 1];
        } else if dsell[i] && count == 1 {
            let s = c[i];
            let pct = ((s - s * comm / 100.0) - (bp + s * comm / 100.0)) / bp * 100.0;
            p1[i] = pct < 0.0 || pct < min_profit;
            p2[i] = if pct < 0.0 { true } else { pct < min_profit };
            count = 0;
        } else {
            p1[i] = p1[i - 1];
            p2[i] = p2[i - 1];
        }
    }
    (0..n).map(|i| p1[i] && p2[i]).collect()
}

/// Buy and Sell arrays as each AFL builds them (before ExRem).
pub(crate) fn signals(id: &str, x: &Ctx, p: &Params) -> Option<(Vec<bool>, Vec<bool>)> {
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(0.0, |d| d.def));
    let gu = |k: &str| g(k).round().max(1.0) as usize;
    let (c, n) = (&x.c, x.c.len());
    let calc = stcal(x, g("st_point").round() as i64);
    let (t1, _) = supertrend(&calc, &x.h, &x.l, c, gu("entry_period"), g("entry_factor"), g("entry_factor"));
    let (t2, _) = supertrend(&calc, &x.h, &x.l, c, gu("exit_period"), g("exit_factor"), g("exit_factor"));
    let both_down: Vec<bool> = (0..n).map(|i| t1[i] == -1 && t2[i] == -1).collect();
    let (ev, start) = match id {
        EXPIRY => (expiry_days(x, false), 1),
        EXPIRY_FLT | REENTRY => (expiry_days(x, true), 1),
        FIVE_DAY_FLT => {
            let k = gu("entry_day");
            let eom = month_end(x);
            ((0..n).map(|i| i + k < n && eom[i + k]).collect(), k)
        }
        _ => (month_end(x), 1),
    };
    let (lc, lc1) = ref_closes(c, &ev, start);
    let diff = g("diff");
    let first_guard = matches!(id, EXPIRY_FLT | REENTRY);
    let buysig: Vec<bool> = (0..n).map(|i| lc[i] < lc1[i] - lc1[i] * diff / 100.0 && (!first_guard || c[0] != lc1[i])).collect();
    if id == EXPIRY || id == MONTH_END {
        let buy = (0..n).map(|i| buysig[i] && t1[i] == 1 && i != 0).collect();
        return Some((buy, both_down));
    }
    let (fb, fs) = filter(x, &g);
    let upc: Vec<bool> = (0..n).map(|i| prec2(x.o[i]) == prec2(c[i]) && prec2(x.o[i]) == prec2(x.h[i])).collect();
    let loc: Vec<bool> = (0..n).map(|i| prec2(x.o[i]) == prec2(c[i]) && prec2(x.o[i]) == prec2(x.l[i])).collect();
    let bar30 = matches!(id, MONTH_END_FLT | FIVE_DAY_FLT);
    let sell_flt = id != MONTH_END_FLT;
    let sell: Vec<bool> = (0..n).map(|i| (both_down[i] || (sell_flt && fs[i])) && !loc[i]).collect();
    let base: Vec<bool> = (0..n).map(|i| t1[i] == 1 && fb[i] && i != 0 && !upc[i] && (!bar30 || i > 30)).collect();
    let buy = if id == REENTRY {
        let dbuy: Vec<bool> = (0..n).map(|i| buysig[i] && base[i]).collect();
        let ps = probsig(c, &dbuy, &sell, &t1, &fb, g("comm"), g("min_profit"));
        (0..n).map(|i| (buysig[i] || (i > 0 && ps[i - 1])) && base[i]).collect()
    } else {
        (0..n).map(|i| buysig[i] && base[i]).collect()
    };
    Some((buy, sell))
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !IDS.contains(&id) {
        return None;
    }
    let n = x.c.len();
    if n == 0 || from > to || to >= n {
        return Some(Vec::new());
    }
    let (buy, sell) = signals(id, x, p)?;
    Some(backtest(x, Sig { buy, sell, short: vec![false; n], cover: vec![false; n] }, 1, from, to))
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let d = catalogue().into_iter().find(|d| d.id == id)?;
    Some((d.id, d.name, "Negative Expiry"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;

    /// Weekday dates from `start` (YYYY-MM-DD), skipping Saturdays and Sundays and any date in `skip`.
    fn weekdays(start: (i64, i64, i64), count: usize, skip: &[&str]) -> Vec<String> {
        let (mut y, mut m, mut d) = start;
        let mut out = Vec::new();
        while out.len() < count {
            let s = format!("{y:04}-{m:02}-{d:02}");
            let wd = dow(&s);
            if wd != 0 && wd != 6 && !skip.contains(&s.as_str()) {
                out.push(s);
            }
            let dim = match m { 2 => if (y % 4 == 0 && y % 100 != 0) || y % 400 == 0 { 29 } else { 28 }, 4 | 6 | 9 | 11 => 30, _ => 31 };
            d += 1;
            if d > dim {
                d = 1;
                m += 1;
                if m > 12 {
                    m = 1;
                    y += 1;
                }
            }
        }
        out
    }
    fn rows(dates: &[String], closes: &[f64]) -> Vec<Row> {
        dates.iter().zip(closes).map(|(d, &c)| Row { ticker: "T".into(), date: d.clone(), open: c, high: c * 1.01, low: c * 0.99, close: c, volume: None }).collect()
    }
    fn dates_of(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn afl_february_rule() {
        assert_eq!(afl_days_in_month(2000, 2), 28); // the AFL's quirk
        assert_eq!(afl_days_in_month(2004, 2), 29);
        assert_eq!(afl_days_in_month(2023, 2), 28);
        assert_eq!(afl_days_in_month(2024, 4), 30);
        assert_eq!(afl_days_in_month(2024, 8), 31);
    }

    #[test]
    fn expiry_calendar_both_versions() {
        // 2024-01-25 Thu (last-7 Thursday); 2024-01-18 Thu (no); 2024-03-27 Wed then Fri 29 (Thursday 28 a holiday)
        let d = dates_of(&["2024-01-18", "2024-01-25", "2024-01-26", "2024-03-26", "2024-03-27", "2024-03-29", "2024-04-01"]);
        let r = rows(&d, &[100.0; 7]);
        let x = Ctx::new(&r, &r, &[]);
        assert_eq!(expiry_days(&x, false), vec![false, true, false, false, true, false, false]);
        assert_eq!(expiry_days(&x, true), vec![false, true, false, false, true, false, false]);
        // Wednesday then Monday of the same month: only the Index Filter version (2017-06-28 Wed, next 2017-07-03 is
        // another month, so use 2015-09-23 Wed -> 2015-09-28 Mon, 30-23 = 7 <= 7); and the hard-coded 2009-04-29
        let d = dates_of(&["2015-09-23", "2015-09-28", "2009-04-28", "2009-04-29", "2009-05-04"]);
        let r = rows(&d, &[100.0; 5]);
        let x = Ctx::new(&r, &r, &[]);
        assert_eq!(expiry_days(&x, false), vec![false, false, false, false, false]);
        assert_eq!(expiry_days(&x, true), vec![true, false, false, true, false]);
        // the Without-filter version skips a Wednesday the 30th
        let d = dates_of(&["2015-12-30", "2016-01-01"]);
        let r = rows(&d, &[100.0; 2]);
        let x = Ctx::new(&r, &r, &[]);
        assert_eq!(expiry_days(&x, false), vec![false, false]);
    }

    #[test]
    fn month_end_from_next_bar() {
        let d = dates_of(&["2024-01-30", "2024-01-31", "2024-02-01", "2024-02-02", "2024-02-29", "2024-03-04", "2024-03-05"]);
        let r = rows(&d, &[100.0; 7]);
        let x = Ctx::new(&r, &r, &[]);
        // 03-04 is followed by day 5 but is itself the 4th -> not a month end; the last bar never is
        assert_eq!(month_end(&x), vec![false, true, false, false, true, false, false]);
    }

    #[test]
    fn reference_closes_and_buy_condition() {
        let c = [100.0, 101.0, 99.0, 98.0, 97.0, 96.0];
        let ev = [false, true, false, true, false, true];
        let (lc, lc1) = ref_closes(&c, &ev, 1);
        assert_eq!(lc, vec![100.0, 101.0, 101.0, 98.0, 98.0, 96.0]);
        assert_eq!(lc1, vec![100.0, 100.0, 100.0, 101.0, 101.0, 98.0]);
        // 5-day version: the loop starts at entryday, bars before keep their own close
        let (lc, lc1) = ref_closes(&c, &[false; 6], 3);
        assert_eq!(lc, vec![100.0, 101.0, 99.0, 99.0, 99.0, 99.0]);
        assert_eq!(lc1, vec![100.0, 101.0, 99.0, 99.0, 99.0, 99.0]);
    }

    #[test]
    fn five_day_event_is_shifted_before_month_end() {
        let d = weekdays((2024, 1, 1), 60, &[]);
        let closes: Vec<f64> = (0..60).map(|k| 100.0 + k as f64).collect();
        let r = rows(&d, &closes);
        let x = Ctx::new(&r, &r, &[]);
        let eom = month_end(&x);
        let jan = eom.iter().position(|&b| b).unwrap();
        assert_eq!(d[jan], "2024-01-31");
        // entry day 5: reference bar 2024-01-24, so lastclose takes that close
        let k = 5;
        let ev: Vec<bool> = (0..60).map(|i| i + k < 60 && eom[i + k]).collect();
        assert_eq!(d[ev.iter().position(|&b| b).unwrap()], "2024-01-24");
    }

    #[test]
    fn reentry_loop_flags_small_profit() {
        // loop trade 1: buy 100 at bar 1, sell 102 at bar 3 (< 5% net) -> flag; re-buy at bar 4 without the expiry
        // condition; sell 110 at bar 6 (> 5%) -> flag cleared
        let c = [100.0, 100.0, 101.0, 102.0, 100.0, 105.0, 110.0, 111.0];
        let dbuy = [false, true, false, false, false, false, false, false];
        let dsell = [false, false, false, true, false, false, true, false];
        let t1 = [1i8; 8];
        let ps = probsig(&c, &dbuy, &dsell, &t1, &[true; 8], 0.65, 5.0);
        assert_eq!(ps, vec![false, false, false, true, true, true, false, false]);
    }

    #[test]
    fn month_end_filter_does_not_force_sell_but_five_day_does() {
        let d = weekdays((2023, 1, 2), 120, &[]);
        let closes: Vec<f64> = (0..120).map(|k| 100.0 + 8.0 * ((k as f64) / 6.0).sin()).collect();
        let r = rows(&d, &closes);
        let x = Ctx::new(&r, &r, &[]);
        let kv: Params = [("flt", 1.0), ("flt_type", 1.0), ("flt_tf", 2.0), ("sma", 3.0)].iter().map(|(k, v)| (k.to_string(), *v)).collect();
        let (_, s_me) = signals(MONTH_END_FLT, &x, &kv).unwrap();
        let (_, s_5d) = signals(FIVE_DAY_FLT, &x, &kv).unwrap();
        let (_, fs) = sma_filter(&x, 'D', 3, false);
        assert!((0..120).any(|i| fs[i] && !s_me[i]));
        assert!((0..120).all(|i| !fs[i] || s_5d[i]));
        // and the month-end filter version never buys before bar 31
        let (b, _) = signals(MONTH_END_FLT, &x, &Params::new()).unwrap();
        assert!(b[..31].iter().all(|v| !v));
    }

    #[test]
    fn expiry_buys_after_lower_expiry_close_when_st_up() {
        // falling into the Jan and Feb 2024 expiries, then rising: the Feb expiry close is > 1% below Jan's
        let d = weekdays((2023, 12, 1), 80, &[]);
        let closes: Vec<f64> = (0..80).map(|k| if k < 45 { 120.0 - k as f64 * 0.5 } else { 97.5 + (k - 45) as f64 * 1.5 }).collect();
        let r = rows(&d, &closes);
        let x = Ctx::new(&r, &r, &[]);
        let t = run(EXPIRY, &x, 0, 79, &Params::new()).unwrap();
        assert!(!t.is_empty());
        let ev = expiry_days(&x, false);
        let jan = (0..80).filter(|&i| ev[i]).nth(1).unwrap(); // Dec, Jan expiries
        assert_eq!(d[jan], "2024-01-25");
        assert!(t[0].legs[0].i > 45 && t[0].legs[0].i >= jan);
        // the guarded Index Filter version needs a second expiry after the first; same entry here
        let t2 = run(EXPIRY_FLT, &x, 0, 79, &Params::new()).unwrap();
        assert_eq!(t2.first().map(|t| t.legs[0].i), t.first().map(|t| t.legs[0].i));
    }

    #[test]
    fn every_strategy_runs() {
        let d = weekdays((2022, 1, 3), 400, &["2022-03-31", "2022-06-30"]);
        let closes: Vec<f64> = (0..400).map(|k| 100.0 + 15.0 * ((k as f64) / 9.0).sin() + k as f64 * 0.05).collect();
        let r = rows(&d, &closes);
        let x = Ctx::new(&r, &r, &[]);
        for id in IDS {
            assert!(params(id).is_some() && family_of(id).is_some(), "{id}");
            for kv in [vec![], vec![("flt", 1.0)], vec![("flt", 1.0), ("flt_type", 1.0), ("flt_tf", 1.0), ("sma", 10.0)], vec![("st_point", 5.0)]] {
                let p: Params = kv.iter().map(|(k, v)| (k.to_string(), *v)).collect();
                let t = run(id, &x, 50, 399, &p).unwrap();
                assert!(t.iter().all(|t| t.exit_i >= t.legs[0].i && t.legs[0].i >= 50), "{id}");
                assert!(t.iter().filter(|t| t.open).all(|t| t.exit_i == 399), "{id}");
            }
        }
        assert_eq!(catalogue().len(), IDS.len());
    }
}
