//! YT Strategies · Negative Expiry (stocks): the team's AmiBroker AFLs in "Old Topics/Negative Expiry/Stock Filter"
//! that trade individual stocks ("ST 52 54": entry Supertrend 5/2, exit Supertrend 5/4, long only), each run here on
//! one stock's own daily bars. Coded from the AFLs as written, with the AFLs' own default settings; there is no book
//! PDF. The one "Stock Filter" AFL already in `books::ytne` (re-entry until minimum profit, run on the indices) is not
//! repeated here.
//!
//! AmiBroker behaviour follows `books::ytsw` (fills at the signal bar's close, ExRem resetting on the same bar, exit
//! then entry on one bar, expandLast for higher time frames, ATR(1) Null on the first bar) and the calendar rules of
//! `books::ytne` (expiry days from the AFLs' `Lastthursday()`, with its hard-coded dates, February rule and one bar
//! of look-ahead for the Wednesday fallback; month end from the next bar's day number). Further points:
//! - `HHV(C, BarCount)` (the all-time highest close) and `HHV(C, 365)` use the bars available near the start;
//! - the "STOCK" filter (default NONE) is the stock's own Supertrend / SMA on the chosen timeframe;
//! - the AFLs compute PositionScore ranking modes (default NONE) and, in the DUAL RANKING AFL, watch-list ranks with
//!   StaticVarGenerateRanks; none of these enter Buy or Sell, so each stock's rules are complete without them;
//! - the turnover, penny and hard-stop settings are declared but never used in Buy or Sell; the dead-stock default
//!   (NO) only blocks a buy on the first bar.
#![allow(dead_code)]
use crate::books::ytne::{expiry_days, month_end, probsig, ref_closes};
use crate::books::ytsw::{backtest, cross_up, sma_filter, st_filter, supertrend, Sig};
use crate::books::ytwh::hhv;
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "yt_nes";
pub const EXPIRY: &str = "ytnes_expiry";
pub const ATH_DIST: &str = "ytnes_expiry_ath_dist";
pub const ATH_DIST_NEW: &str = "ytnes_expiry_ath_dist_new_buy";
pub const Y365_DIST: &str = "ytnes_expiry_365_dist";
pub const ATH_BREAK: &str = "ytnes_expiry_ath_breakout";
pub const Y365_BREAK: &str = "ytnes_expiry_365_breakout";
pub const REENTRY_ATH: &str = "ytnes_expiry_reentry_ath_dist";
pub const MONTH_END: &str = "ytnes_month_end";
pub const FIVE_DAY: &str = "ytnes_five_day";
pub const FIVE_END_START: &str = "ytnes_five_day_end_to_start";
pub const EXPIRY_FIVE: &str = "ytnes_expiry_to_five_day";
pub const IDS: [&str; 11] = [EXPIRY, ATH_DIST, ATH_DIST_NEW, Y365_DIST, ATH_BREAK, Y365_BREAK, REENTRY_ATH, MONTH_END, FIVE_DAY, FIVE_END_START, EXPIRY_FIVE];

const GROUP: &str = "YT Strategies · Negative Expiry (stocks)";

fn def(id: &'static str, name: &'static str, rules: &'static [&'static str], notes: &'static str) -> StrategyDef {
    StrategyDef { id, name, group: GROUP, session: 0, dir: "Long", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim: None }
}

const EXP: &str = "F|Expiry day from the calendar: 29 Apr 2009 and 28 Mar 2018 (hard-coded), a Thursday in the month's last 7 days, or a Wednesday in its last 8 days when the next bar is a Friday or Monday of the same month";
const EXP_DROP: &str = "F|The latest expiry close is more than 1% below the previous expiry's close, and the previous expiry close is no longer the stock's first close";
const FLT: &str = "F|Optional filter on the stock's own bars (off by default): Supertrend (5, 1, mid point) or SMA (200) on monthly (default), weekly or daily bars; buys only while it is up";
const ST_E: &str = "E|Buy at the close while that holds and the entry Supertrend (period 5, factor 2, on the mid point) is up; no buy on the first bar or on an upper-circuit bar (open = high = close at two decimals)";
const ST_X: &str = "X|Sell at the close when the entry and exit (period 5, factor 4) Supertrends are both down, or when the filter is down; no sell on a lower-circuit bar (open = low = close)";
const ATH8: &str = "F|The close is within 8% of the all-time highest close (HHV(C, BarCount), the bars so far)";
const BREAK: &str = "F|Breakout flag: on when the close exceeds the previous bar's highest close; off when a Supertrend on the close (period 10, upper band factor 1, lower band factor 4) is down; otherwise it carries";
macro_rules! common {
    () => {
        " Runs on one stock at a time; the team ran it over stock universes (NSE500 cash-and-positional and F&O stocks); ranking and portfolio limits (MaxOpenPositions 20, sizing) are not modelled. Supertrend: the team's loop (band ATR = mean of the last 5 ATR(1) values); the '3DAYS MA' calculation point falls back to the mid point (the AFL compares it against the misspelt '3DYAS MA'). The filter's ST point gives the pivot only when the DAILY point is the pivot (the AFL tests the wrong variable). Turnover (V x C), penny and hard-stop settings are declared but not used in Buy or Sell."
    };
}

pub fn catalogue() -> Vec<StrategyDef> {
    let n = |s: &'static str| s;
    vec![
        def(EXPIRY, "Negative Expiry ST 5/2-5/4 (stock)", &[EXP, EXP_DROP, FLT, ST_E, ST_X], n(concat!(
            "st 52 54 long only with expiry day close WITH stock FLT - Copy (4).afl (Negative Expiry / Stock Filter), coded as the AFL is written. Same rules as the index version (ytne_expiry_index_filter) with the AFL's filter on the stock itself.",
            " Runs on one stock at a time; the team ran it over stock universes (NSE500 cash-and-positional and F&O stocks; workbooks 'FIRST TRADE AS PER EXPIRY'); ranking (PositionScore modes ROC, RSI, STD, ROC/STD, positive closes, swing; default NONE) and portfolio limits (MaxOpenPositions 20, sizing) are not modelled. Supertrend: the team's loop (band ATR = mean of the last 5 ATR(1) values); the '3DAYS MA' calculation point falls back to the mid point (misspelt comparison in the AFL). The filter's ST point gives the pivot only when the DAILY point is the pivot (the AFL tests the wrong variable). Turnover (V x C), penny and hard-stop settings are declared but not used in Buy or Sell."))),
        def(ATH_DIST, "Negative Expiry ST 5/2-5/4, near All-Time High (stock)", &[EXP, EXP_DROP, ATH8, FLT, ST_E, ST_X], n(concat!(
            "st 52 54 long only with expiry day close WITH stock FLT AND ATHC 30 PCTl - Copy (2).afl, and the same per-stock rules in st 52 54 long only with expiry day close WITH stock ATHC -30 PCT WITH DUAL RANKING.afl (Negative Expiry / Stock Filter), coded as the AFLs are written. Despite the '30 PCT' names, the '% DIFF FROM ATHC' default is 8. The DUAL RANKING AFL's watch-list loops (positive-close counts and swing distance ranked with StaticVarGenerateRanks) feed no Buy or Sell; its loop even overwrites the '% Difference' variable, but only after the buy condition was computed.",
            " Runs on one stock at a time; the team ran it over stock universes (NSE500 cash-and-positional and F&O stocks); ranking and portfolio limits (MaxOpenPositions 20, sizing) are not modelled. Supertrend: the team's loop; the '3DAYS MA' calculation point falls back to the mid point (misspelt comparison in the AFL). The filter's ST point gives the pivot only when the DAILY point is the pivot. Turnover, penny and hard-stop settings are declared but not used in Buy or Sell."))),
        def(ATH_DIST_NEW, "Negative Expiry, Fresh ST Buy near All-Time High (stock)", &[EXP, EXP_DROP, ATH8, FLT,
            "E|Buy at the close when the close crosses above the previous bar's entry Supertrend down line (5, 2, mid point) while those hold; no buy on the first bar or an upper-circuit bar",
            ST_X,
        ], n(concat!(
            "st 52 54 long only with expiry day close WITH stock FLT AND ATHC 30 PCT NEW BUY ONLYl.afl (Negative Expiry / Stock Filter), coded as the AFL is written: Buy needs Cross(C, Ref(down line, -1)) instead of an up trend. The down line is the ST loop's upper band on down-trend bars, with the bar before a flip down taking that bar's up-trend line; elsewhere it is Null, so the cross also fires when the line ends (AmiBroker Null handling). '% DIFF FROM ATHC' default 8.",
            common!()))),
        def(Y365_DIST, "Negative Expiry ST 5/2-5/4, near 365-Day High (stock)", &[EXP, EXP_DROP,
            "F|The close is within 8% of the highest close of the last 365 bars",
            FLT, ST_E, ST_X,
        ], n(concat!("st 52 54 long only with expiry day close WITH stock FLT AND 365 days -30 PCT.afl (Negative Expiry / Stock Filter), coded as the AFL is written. '% DIFF FROM ATHC' default 8; HHV(C, 365) near the start uses the bars available.", common!()))),
        def(ATH_BREAK, "Negative Expiry ST 5/2-5/4 after All-Time-High Breakout (stock)", &[EXP, EXP_DROP, BREAK, FLT,
            "E|Buy at the close while the expiry condition holds, the breakout flag is on and the entry Supertrend (5, 2, mid point) is up; no buy on the first bar or an upper-circuit bar",
            ST_X,
        ], n(concat!(
            "st 52 54 long only with expiry day close WITH stock FLT AND ATHC FLT - Copy (2).afl (Negative Expiry / Stock Filter), coded as the AFL is written. The breakout flag's off branch is written `ATHbuysig[i]==0;` (a comparison, not an assignment); the bar is then left at the array's initial 0, so the flag does go off as intended. The flag's ST (ST2) uses the close, MA(ATR(1), 10), factor 1 for the upper band and 4 for the lower band.",
            common!()))),
        def(Y365_BREAK, "Negative Expiry ST 5/2-5/4 after 365-Day-High Breakout (stock)", &[EXP, EXP_DROP,
            "F|Breakout flag: on when the close exceeds the previous bar's 365-bar highest close; off when the ST on the close (10, factors 1 / 4) is down; otherwise it carries",
            FLT,
            "E|Buy at the close while the expiry condition holds, the breakout flag is on and the entry Supertrend (5, 2, mid point) is up; no buy on the first bar or an upper-circuit bar",
            ST_X,
        ], n(concat!("st 52 54 long only with expiry day close WITH stock FLT AND 365 days FLT.afl (Negative Expiry / Stock Filter), coded as the AFL is written; the breakout flag as in the all-time-high version but on HHV(C, 365) (bars available near the start).", common!()))),
        def(REENTRY_ATH, "Negative Expiry ST 5/2-5/4, Re-entry until Minimum Profit, near ATH (stock)", &[EXP, EXP_DROP, ATH8, FLT,
            "E|Buy at the close when the entry Supertrend (5, 2) is up, the filter is up and the close is within 8% of the all-time highest close, and either the expiry condition holds or the last trade of the AFL's own trade loop made less than the minimum profit (5%)",
            ST_X,
        ], n(concat!(
            "st 52 54 long only with expiry day close WITH stock FLT with probability and athc -8pct ver2 - Copy.afl (Negative Expiry / Stock Filter), coded as the AFL is written: the re-entry AFL of ytne_expiry_reentry_min_profit with the within-8%-of-ATH condition added to the loop's re-entry and to Buy. The AFL's own trade loop charges its BROKERAGE % (default 0.65) at the exit price on both legs; after a loop trade below the minimum profit it may re-buy without the expiry condition until a trade reaches it; Buy uses the loop's flag from the previous bar.",
            common!()))),
        def(MONTH_END, "Negative Month-End ST 5/2-5/4 (stock)", &[
            "F|Month end: the last trading day of the month (read from the next bar's date)",
            "F|The latest month-end close is more than 1% below the previous month-end close, and the previous month-end close is no longer the stock's first close",
            FLT, ST_E, ST_X,
        ], n(concat!("st 52 54 long only with monthly close with stock flt.afl (Negative Expiry / Stock Filter), coded as the AFL is written: unlike the index month-end AFL it has the first-close guard, no 30-bar start, and the filter also forces a sell. The month end is found from the next bar's day number (one bar of look-ahead; the data's last bar is never a month end).", common!()))),
        def(FIVE_DAY, "5 Days Before Month-End ST 5/2-5/4 (stock)", &[
            "F|Reference day: the bar 5 trading days before the month's last trading day (entry day 5)",
            "F|The latest reference-day close is more than 1% below the previous month's reference-day close, and that previous close is no longer the stock's first close",
            FLT, ST_E, ST_X,
        ], n(concat!("st 52 54 long only with 5 day phenomena with stock flt.afl (Negative Expiry / Stock Filter), coded as the AFL is written. The reference day is Ref(eom, entryday): it is known only from the month end entryday bars later, and the month end from the bar after that, so the AFL looks ahead entryday + 1 bars (coded the same way, look-ahead included). Its ROC ranking period is an Optimize (ranking not modelled).", common!()))),
        def(FIVE_END_START, "Month-Start Close vs 5 Days Before Month-End ST 5/2-5/4 (stock)", &[
            "F|Start day: 5 trading days before the month's last trading day; end day: 2 trading days after a month end",
            "F|The start-day close is more than 1% below the latest end-day close (the close 2 days into the month), and that end-day close is no longer the stock's first close",
            FLT, ST_E, ST_X,
        ], n(concat!("st 52 54 long only with 5 day phenomena end to start.afl (Negative Expiry / Stock Filter), coded as the AFL is written: lastclose = the start-day close, lastclose1 = the end-day close held on the bar before the start day. The start day looks ahead entryday + 1 bars as in the 5-day AFL; the end day is Ref(eom, -2). Before the first end day the comparison close is the close of bar entryday - 1.", common!()))),
        def(EXPIRY_FIVE, "5 Days Before Month-End vs Expiry Close ST 5/2-5/4 (stock)", &[
            "F|Start day: 5 trading days before the month's last trading day; expiry day as in the expiry AFLs",
            "F|The start-day close is more than 1% below the latest expiry close (held on the bar before the start day), and that expiry close is no longer the stock's first close",
            FLT, ST_E, ST_X,
        ], n(concat!("st 52 54 long only with exipry to 5 day phenomenal.afl (Negative Expiry / Stock Filter), coded as the AFL is written; when a bar is both the start day and an expiry, the start day wins (the AFL's if / else). The start day looks ahead entryday + 1 bars; expiry days come from the AFL's calendar rule (one bar of look-ahead for the Wednesday fallback).", common!()))),
    ]
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    if !IDS.contains(&id) {
        return None;
    }
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    let mut v = vec![
        p("st_point", "Daily ST calculation point (0 mid point, 1 close, 2 pivot (H+L+C)/3, 3 average of mid point, pivot and close, 4 3-day MA: the AFL's misspelt test makes it the mid point, 5 5-day MA of the close)", 0.0, 0.0, 5.0, 1.0),
        p("entry_period", "ST period for entry", 5.0, 1.0, 20.0, 1.0),
        p("entry_factor", "ST factor for entry", 2.0, 1.0, 20.0, 0.5),
        p("exit_period", "ST period for exit", 5.0, 1.0, 20.0, 1.0),
        p("exit_factor", "ST factor for exit", 4.0, 1.0, 20.0, 0.5),
        p("diff", "% difference below the reference close", 1.0, 0.05, 20.0, 0.25),
    ];
    match id {
        ATH_DIST | ATH_DIST_NEW | Y365_DIST | REENTRY_ATH => v.push(p("ath_diff", "% below the highest close allowed (% DIFF FROM ATHC)", 8.0, 1.0, 50.0, 1.0)),
        ATH_BREAK | Y365_BREAK => v.extend([
            p("ath_period", "ST period for the breakout flag", 10.0, 1.0, 20.0, 1.0),
            p("ath_factor", "ST lower-band factor for the breakout flag", 4.0, 1.0, 20.0, 1.0),
        ]),
        FIVE_DAY | FIVE_END_START | EXPIRY_FIVE => v.push(p("entry_day", "Start day: this many trading days before the month's last trading day", 5.0, 1.0, 10.0, 1.0)),
        _ => {}
    }
    if id == FIVE_END_START {
        v.push(p("exit_day", "End day: this many trading days after a month end", 2.0, 1.0, 30.0, 1.0));
    }
    if id == REENTRY_ATH {
        v.push(p("min_profit", "Minimum profit % (re-entries continue until a trade reaches it)", 5.0, 1.0, 50.0, 1.0));
        v.push(p("comm", "Brokerage % used by the AFL's own trade loop", 0.65, 0.01, 1.0, 0.01));
    }
    v.extend([
        p("flt", "Filter on the stock's own bars (0 none, 1 on: the AFL's STOCK choice)", 0.0, 0.0, 1.0, 1.0),
        p("flt_type", "Filter type (0 Supertrend, 1 SMA)", 0.0, 0.0, 1.0, 1.0),
        p("flt_tf", "Filter timeframe (0 monthly, 1 weekly, 2 daily)", 0.0, 0.0, 2.0, 1.0),
        p("flt_period", "Filter ST period", 5.0, 1.0, 20.0, 1.0),
        p("flt_factor", "Filter ST factor", 1.0, 0.5, 20.0, 0.5),
        p("flt_point", "Filter ST calculation point (0 mid point, 1 close, 2 pivot: gives the close unless the daily point is the pivot, as in the AFL)", 0.0, 0.0, 2.0, 1.0),
        p("sma", "Filter SMA period", 200.0, 1.0, 300.0, 1.0),
    ]);
    Some(v)
}

// ---------------------------------------------------------------- pieces

/// Daily STCAL: 0 mid point, 1 close, 2 pivot, 3 (MP + pivot + C) / 3, 4 the mid point (misspelt list test), 5 MA(C, 5)
/// (NaN for the first 4 bars: the ST bands hold their trend there).
fn stcal(x: &Ctx, kind: i64) -> Vec<f64> {
    (0..x.c.len())
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

/// The AFLs' STOCK filter on the stock's own bars: (buy allowed, filter sell). Off: (true, false).
fn filter(x: &Ctx, g: &dyn Fn(&str) -> f64) -> (Vec<bool>, Vec<bool>) {
    let n = x.c.len();
    if g("flt").round() as i64 != 1 {
        return (vec![true; n], vec![false; n]);
    }
    let tf = match g("flt_tf").round() as i64 { 1 => 'W', 2 => 'D', _ => 'M' };
    if g("flt_type").round() as i64 == 1 {
        return sma_filter(x, tf, g("sma").round().max(1.0) as usize, false);
    }
    let point = if g("flt_point").round() as i64 == 0 { 1 } else if g("st_point").round() as i64 == 2 { 3 } else { 2 };
    let m = st_filter(x, tf, point, g("flt_period").round().max(1.0) as usize, g("flt_factor"));
    (m.iter().map(|&v| v == 1).collect(), m.iter().map(|&v| v == -1).collect())
}

/// The 5-day "end to start" / "expiry to 5 day" loop: from `start_bar`, on a start bar lastclose = C and lastclose1 =
/// the previous bar's lastclose2; else on an end bar lastclose2 = C; otherwise all carry. Returns (lastclose,
/// lastclose1).
pub(crate) fn start_end_closes(c: &[f64], start: &[bool], end: &[bool], start_bar: usize) -> (Vec<f64>, Vec<f64>) {
    let (mut lc, mut lc1, mut lc2) = (c.to_vec(), c.to_vec(), c.to_vec());
    for i in start_bar.max(1)..c.len() {
        if start[i] {
            lc[i] = c[i];
            lc1[i] = lc2[i - 1];
            lc2[i] = lc2[i - 1];
        } else if end[i] {
            lc2[i] = c[i];
            lc[i] = lc[i - 1];
            lc1[i] = lc1[i - 1];
        } else {
            lc[i] = lc[i - 1];
            lc1[i] = lc1[i - 1];
            lc2[i] = lc2[i - 1];
        }
    }
    (lc, lc1)
}

/// The breakout flag: 1 when C > hcl[i-1]; else 0 when the ST2 trend is down (the AFL's `==0` leaves the initial 0);
/// else it carries.
pub(crate) fn breakout_flag(c: &[f64], hcl: &[f64], st2: &[i8]) -> Vec<bool> {
    let n = c.len();
    let mut f = vec![false; n];
    for i in 1..n {
        f[i] = if c[i] > hcl[i - 1] { true } else if st2[i] == -1 { false } else { f[i - 1] };
    }
    f
}

/// Buy and Sell arrays as each AFL builds them (before ExRem).
pub(crate) fn signals(id: &str, x: &Ctx, p: &Params) -> Option<(Vec<bool>, Vec<bool>)> {
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(0.0, |d| d.def));
    let gu = |k: &str| g(k).round().max(1.0) as usize;
    let (c, n) = (&x.c, x.c.len());
    let calc = stcal(x, g("st_point").round() as i64);
    let (t1, down1) = supertrend(&calc, &x.h, &x.l, c, gu("entry_period"), g("entry_factor"), g("entry_factor"));
    let (t2, _) = supertrend(&calc, &x.h, &x.l, c, gu("exit_period"), g("exit_factor"), g("exit_factor"));
    // reference closes
    let (lc, lc1) = match id {
        MONTH_END => ref_closes(c, &month_end(x), 1),
        FIVE_DAY | FIVE_END_START | EXPIRY_FIVE => {
            let k = gu("entry_day");
            let eom = month_end(x);
            let start: Vec<bool> = (0..n).map(|i| i + k < n && eom[i + k]).collect();
            match id {
                FIVE_DAY => ref_closes(c, &start, k),
                FIVE_END_START => {
                    let e = gu("exit_day");
                    let end: Vec<bool> = (0..n).map(|i| i >= e && eom[i - e]).collect();
                    start_end_closes(c, &start, &end, k)
                }
                _ => start_end_closes(c, &start, &expiry_days(x, true), k),
            }
        }
        _ => ref_closes(c, &expiry_days(x, true), 1),
    };
    let diff = g("diff");
    let buysig: Vec<bool> = (0..n).map(|i| lc[i] < lc1[i] - lc1[i] * diff / 100.0 && c[0] != lc1[i]).collect();
    // all-time-high / 365-day conditions
    let ath_ok: Vec<bool> = match id {
        ATH_DIST | ATH_DIST_NEW | Y365_DIST | REENTRY_ATH => {
            let hcl = hhv(c, if id == Y365_DIST { 365 } else { n.max(1) });
            let d = g("ath_diff");
            (0..n).map(|i| (hcl[i] - c[i]) / hcl[i] * 100.0 <= d).collect()
        }
        ATH_BREAK | Y365_BREAK => {
            let hcl = hhv(c, if id == Y365_BREAK { 365 } else { n.max(1) });
            let (st2, _) = supertrend(c, &x.h, &x.l, c, gu("ath_period"), 1.0, g("ath_factor"));
            breakout_flag(c, &hcl, &st2)
        }
        _ => vec![true; n],
    };
    let (fb, fs) = filter(x, &g);
    let upc: Vec<bool> = (0..n).map(|i| prec2(x.o[i]) == prec2(c[i]) && prec2(x.o[i]) == prec2(x.h[i])).collect();
    let loc: Vec<bool> = (0..n).map(|i| prec2(x.o[i]) == prec2(c[i]) && prec2(x.o[i]) == prec2(x.l[i])).collect();
    let sell: Vec<bool> = (0..n).map(|i| ((t1[i] == -1 && t2[i] == -1) || fs[i]) && !loc[i]).collect();
    let trend_ok: Vec<bool> = if id == ATH_DIST_NEW {
        let prev: Vec<Option<f64>> = (0..n).map(|i| if i == 0 { None } else { down1[i - 1] }).collect();
        cross_up(c, &prev)
    } else {
        t1.iter().map(|&t| t == 1).collect()
    };
    let base: Vec<bool> = (0..n).map(|i| trend_ok[i] && fb[i] && i != 0 && !upc[i] && ath_ok[i]).collect();
    let buy = if id == REENTRY_ATH {
        let dbuy: Vec<bool> = (0..n).map(|i| buysig[i] && base[i]).collect();
        let re_ok: Vec<bool> = (0..n).map(|i| fb[i] && i != 0 && ath_ok[i]).collect();
        let ps = probsig(c, &dbuy, &sell, &t1, &re_ok, g("comm"), g("min_profit"));
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
    use crate::books::ytwh::testbars::{bars, ps, wave};
    use crate::Row;

    fn rows_c(closes: &[f64]) -> Vec<Row> {
        bars(&closes.iter().map(|&c| (c, c * 1.01, c * 0.99, c)).collect::<Vec<_>>())
    }

    #[test]
    fn start_end_loop_takes_end_close_into_lastclose1() {
        let c = [10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0];
        let end = [false, false, true, false, false, false, false];
        let start = [false, false, false, false, true, false, false];
        let (lc, lc1) = start_end_closes(&c, &start, &end, 1);
        assert_eq!(lc, vec![10.0, 10.0, 10.0, 10.0, 14.0, 14.0, 14.0]);
        assert_eq!(lc1, vec![10.0, 10.0, 10.0, 10.0, 12.0, 12.0, 12.0]);
        // a bar that is both: the start branch wins
        let (lc, lc1) = start_end_closes(&c, &[false, false, true, false, false, false, false], &end, 1);
        assert_eq!((lc[2], lc1[2]), (12.0, 10.0));
    }

    #[test]
    fn breakout_flag_resets_on_st_down() {
        let c = [10.0, 11.0, 10.5, 10.4, 12.0, 11.0];
        let hcl = hhv(&c, 6);
        let st = [1i8, 1, 1, -1, 1, 1];
        assert_eq!(breakout_flag(&c, &hcl, &st), vec![false, true, true, false, true, true]);
    }

    #[test]
    fn ath_distance_blocks_buys_far_below_the_high() {
        // a high of 200 early, then a slide to ~100 with expiry drops: the plain version may buy, the ATH one never
        let r = wave(400);
        let mut v: Vec<(f64, f64, f64, f64)> = r.iter().map(|b| (b.open, b.high, b.low, b.close)).collect();
        v[5] = (200.0, 201.0, 199.0, 200.0);
        let r = bars(&v);
        let x = Ctx::new(&r, &r, &[]);
        let (b, _) = signals(ATH_DIST, &x, &Params::new()).unwrap();
        let hcl = hhv(&x.c, x.c.len());
        assert!((0..400).all(|i| !b[i] || (hcl[i] - x.c[i]) / hcl[i] * 100.0 <= 8.0));
        assert!((6..400).all(|i| !b[i]));
    }

    #[test]
    fn month_end_version_has_first_close_guard_and_filter_sell() {
        let r = wave(300);
        let x = Ctx::new(&r, &r, &[]);
        let kv = ps(&[("flt", 1.0), ("flt_type", 1.0), ("flt_tf", 2.0), ("sma", 3.0)]);
        let (_, s) = signals(MONTH_END, &x, &kv).unwrap();
        let (_, fs) = sma_filter(&x, 'D', 3, false);
        let n = x.c.len();
        let loc: Vec<bool> = (0..n).map(|i| prec2(x.o[i]) == prec2(x.c[i]) && prec2(x.o[i]) == prec2(x.l[i])).collect();
        assert!((0..n).all(|i| !(fs[i] && !loc[i]) || s[i]));
        // before the first month end nothing can be bought (both reference closes are the first close)
        let (b, _) = signals(MONTH_END, &x, &Params::new()).unwrap();
        let first_eom = month_end(&x).iter().position(|&v| v).unwrap();
        assert!(b[..=first_eom].iter().all(|v| !v));
    }

    #[test]
    fn expiry_drop_buys_when_st_up() {
        // Jan 2024: falling into the Jan and Feb expiries, then rising
        let closes: Vec<f64> = (0..80).map(|k| if k < 40 { 120.0 - k as f64 * 0.5 } else { 100.0 + (k - 40) as f64 * 1.5 }).collect();
        let r = rows_c(&closes);
        let x = Ctx::new(&r, &r, &[]);
        let t = run(EXPIRY, &x, 0, 79, &Params::new()).unwrap();
        assert!(!t.is_empty());
        let ev = expiry_days(&x, true);
        let second = (0..80).filter(|&i| ev[i]).nth(1).unwrap();
        assert!(t[0].legs[0].i >= second);
        // identical to the index version's rules on the same bars
        let (b1, s1) = signals(EXPIRY, &x, &Params::new()).unwrap();
        let (b2, s2) = crate::books::ytne::signals(crate::books::ytne::EXPIRY_FLT, &x, &Params::new()).unwrap();
        assert_eq!((b1, s1), (b2, s2));
    }

    #[test]
    fn every_strategy_runs() {
        let r = wave(420);
        let mut x = Ctx::new(&r, &r, &[]);
        for pass in 0..2 {
            if pass == 1 {
                for k in ["Open", "High", "Low", "Close"] {
                    x.extra.insert(format!("BENCH.{k}"), x.c.iter().map(|v| v * 0.9).collect());
                }
            }
            for id in IDS {
                assert!(params(id).is_some() && family_of(id).is_some(), "{id}");
                for kv in [vec![], vec![("flt", 1.0)], vec![("flt", 1.0), ("flt_type", 1.0), ("flt_tf", 2.0), ("sma", 10.0)], vec![("st_point", 5.0)]] {
                    for (from, to) in [(0, 419), (50, 300)] {
                        let t = run(id, &x, from, to, &ps(&kv)).unwrap();
                        assert!(t.iter().all(|t| t.exit_i >= t.legs[0].i && t.legs[0].i >= from && t.exit_i <= to), "{id}");
                        assert!(t.iter().filter(|t| t.open).all(|t| t.exit_i == to), "{id}");
                        assert!(t.windows(2).all(|w| w[1].legs[0].i >= w[0].exit_i), "{id}");
                    }
                }
            }
        }
        assert_eq!(catalogue().len(), IDS.len());
    }
}
