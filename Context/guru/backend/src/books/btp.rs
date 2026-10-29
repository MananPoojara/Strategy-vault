//! The Best of the Professional Traders Journal: Best Trading Patterns, Volumes I and II (Larry Connors,
//! M. Gordon Publishing 1998/1999). Book id "btp". Volume I ids start "btp_", Volume II ids "btp2_".
//! Seven PTJ articles per volume. Most of them were also reprinted in Advanced Trading Strategies (book "ats").
//!
//! The book's entries are stop orders ("1 tick above the day-three high", valid for N days), which the shared
//! simulator does not have, so this module carries a small simulator (`sim`) that returns the same `Trade`s.
//! Conventions (also in the report):
//!   * a buy stop fills when High >= level at max(Open, level); a sell stop when Low <= level at min(Open, level)
//!   * protective stop: a gap through it fills at the open; otherwise at the stop price
//!   * entry bar of a stop order: the protective stop counts only if the bar opened on the safe side of it
//!     (if the bar opened beyond the stop, its extreme came before the entry: the WREG setup); the
//!     8-Day reversal's condition-3 bar (book: the low first, then the breakout) is not stopped by its range; in
//!     both cases the stop still counts (fill at the stop price) when the entry bar CLOSES beyond it, because
//!     price then certainly crossed the stop after the fill
//!   * a stop and a target on the same bar: the stop is assumed first
//!   * half exits (2-for-1, CBP's 20% cover, Gipsons' HV exit) are folded into one trade whose exit price is the
//!     average of the two halves, so the trade's return is exact (the equity curve between the two halves is
//!     marked on the full position)
//!   * one position at a time; no new entry on the bar of an exit
//!   * where the book says only "trail your stops", the exit is a labelled parameter set that is NOT from the
//!     book: a trailing stop at the prior N-day low/high (default), plus optional, off by default: the 2-for-1
//!     money management of Connors' Advanced Trading Strategies ch. 28 (default on only where the book names it:
//!     Turtle Thrusts rule 4, and Momentum MAs' "scale out") and a maximum hold.
#![allow(unused_imports, dead_code)]
use crate::connors::*;
use crate::Row;

pub const BOOK: &str = "btp";

/// Daily rules admitted after the 2026-10-21 source review (btp-admission/SOURCE-CONTRACT.md) and an independent
/// chronological Python ledger check on actual index/ETF prices. Declared application: the book tested or
/// illustrated these on futures and US stocks. Held (not listed): CBP (IBD RS stock ranking, gate withheld),
/// Bottom Reversals and Momentum Gaps (setup/stop not quantified), Large-Range Days (options / study),
/// 10% Oops x3 and WREG sells (gate withheld: same-day path).
pub const REVIEWED_DAILY: &[&str] = &[
    "btp_dvmtm_short", "btp_triple_day_pullback_adx25", "btp_triple_day_pullback_adx30", "btp_turtle_thrust",
    "btp2_smtp_long", "btp2_smtp_short", "btp2_1234_long", "btp2_1234_short",
    "btp2_8day_hl_reversal_long", "btp2_8day_hl_reversal_short", "btp2_momentum_ma_long", "btp2_momentum_ma_short",
    "btp2_gipsons_long", "btp2_gipsons_short", "btp2_wide_range_exhaustion_gap_long",
];
pub fn reviewed_daily(id: &str) -> bool { REVIEWED_DAILY.contains(&id) }
/// Book-shown single-stock rules (Volume I/II articles use US stocks with 1/8-dollar ticks) also admitted on actual
/// stocks with the stock's real tick (user decision 2026-10-21); Momentum MAs (Merrill Lynch example, open entry, no
/// tick). Triple-Day Pullbacks and Turtle Thrusts are shown on futures only, so they keep index/ETF only.
pub const STOCK_ADMITTED: &[&str] = &[
    "btp_dvmtm_short", "btp2_smtp_long", "btp2_smtp_short", "btp2_1234_long", "btp2_1234_short",
    "btp2_8day_hl_reversal_long", "btp2_8day_hl_reversal_short", "btp2_gipsons_long", "btp2_gipsons_short",
    "btp2_wide_range_exhaustion_gap_long", "btp2_momentum_ma_long", "btp2_momentum_ma_short",
];
/// Admitted rules whose fills or setups read the traded instrument's open (stop orders, gaps, open entries).
/// Only Triple-Day Pullbacks (close fills, ADX from high/low/close) does not.
pub fn reads_open(id: &str) -> bool { reviewed_daily(id) && !id.starts_with("btp_triple_day_pullback") }

// ---------------------------------------------------------------- local helpers

/// Wilder DMI with both lines: (+DI, -DI, ADX). Same maths as `connors::dmi`, which does not return -DI.
fn dmi3(r: &[Row], n: usize) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
    let len = r.len();
    let (mut pdm, mut mdm, mut tr) = (vec![f64::NAN; len], vec![f64::NAN; len], vec![f64::NAN; len]);
    let trs = true_range(r);
    for i in 1..len {
        let up = r[i].high - r[i - 1].high;
        let dn = r[i - 1].low - r[i].low;
        pdm[i] = if up > dn && up > 0.0 { up } else { 0.0 };
        mdm[i] = if dn > up && dn > 0.0 { dn } else { 0.0 };
        tr[i] = trs[i];
    }
    let (sp, sm, st) = (wilder(&pdm, n), wilder(&mdm, n), wilder(&tr, n));
    let (mut p, mut m, mut dx) = (vec![f64::NAN; len], vec![f64::NAN; len], vec![f64::NAN; len]);
    for i in 0..len {
        if st[i] > 0.0 {
            p[i] = 100.0 * sp[i] / st[i];
            m[i] = 100.0 * sm[i] / st[i];
            dx[i] = if p[i] + m[i] > 0.0 { 100.0 * (p[i] - m[i]).abs() / (p[i] + m[i]) } else { 0.0 };
        }
    }
    (p, m, wilder(&dx, n))
}

/// Days since 1970-01-01 for "YYYY-MM-DD" (Howard Hinnant's days_from_civil).
fn days(d: &str) -> i64 {
    let (y, m, dd): (i64, i64, i64) = (d[0..4].parse().unwrap_or(1970), d[5..7].parse().unwrap_or(1), d[8..10].parse().unwrap_or(1));
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * (m + if m > 2 { -3 } else { 9 }) + 2) / 5 + dd - 1;
    era * 146097 + yoe * 365 + yoe / 4 - yoe / 100 + doy - 719468
}

/// Weekly bars (Monday-Friday weeks) and, for each, the index of its last daily bar.
fn weekly(r: &[Row]) -> (Vec<Row>, Vec<usize>) {
    let (mut w, mut last): (Vec<Row>, Vec<usize>) = (vec![], vec![]);
    let mut key = i64::MIN;
    for (i, x) in r.iter().enumerate() {
        let k = (days(&x.date) + 3).div_euclid(7); // 1970-01-01 was a Thursday: +3 starts weeks on Monday
        if k != key {
            key = k;
            w.push(x.clone());
            last.push(i);
        } else {
            let b = w.last_mut().unwrap();
            b.high = b.high.max(x.high);
            b.low = b.low.min(x.low);
            b.close = x.close;
            *last.last_mut().unwrap() = i;
        }
    }
    (w, last)
}

fn range(x: &Ctx, i: usize) -> f64 {
    x.h[i] - x.l[i]
}

// ---------------------------------------------------------------- local simulator

#[derive(Clone, Copy)]
enum Fill {
    Close,     // on the close of bar `from`
    Open,      // at the open of bar `from`
    Stop(f64), // stop order live on bars from..=last
}

#[derive(Clone, Copy)]
struct Ord {
    dir: f64,
    fill: Fill,
    setup: usize, // bar whose close completed the setup (equals `from` for the 8-Day reversal's condition-3 bar)
    from: usize,
    last: usize,
    stop: f64, // initial protective stop, NaN = none
}

struct Pos {
    dir: f64,
    i: usize,
    px: f64,
    stop: f64,
    risk: f64, // |entry - initial stop|, 0 = no stop
    half: Option<(usize, f64)>, // (bar, price) of the half taken off
    why_half: String,
    kind: &'static str,
}

enum Act {
    Close(String),
    NextOpen(String),
    HalfClose(String),
}

type Rule<'a> = &'a dyn Fn(usize, &Pos) -> Option<Act>;

#[derive(Default)]
struct Ex<'a> {
    hold: Option<usize>,      // exit on the close N bars after entry (0 = the entry bar)
    trail: Option<usize>,     // trailing stop at the prior N-bar low (long) / high (short), ratchets only
    trail_after_half: bool,   // trailing starts only after the half exit (Gipsons "tighten the other half")
    half_at_r: bool,          // 2-for-1: half off at entry +/- initial risk
    half_at_pct: Option<f64>, // CBP: half off at this fractional profit
    target_pct: Option<f64>,  // Gipsons (b): whole position off at this fractional profit
    rule: Option<Rule<'a>>,   // close-of-bar rule
}

fn sim(x: &Ctx, mut orders: Vec<Ord>, ex: &Ex, from: usize, to: usize) -> Vec<Trade> {
    orders.sort_by_key(|o| (o.from, o.setup));
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let mut out = Vec::new();
    let mut pos: Option<Pos> = None;
    let mut exit_open: Option<String> = None;
    let mut first = 0;
    let done = |p: Pos, j: usize, sig: usize, px: f64, why: String, out: &mut Vec<Trade>, open: bool| {
        let exit_px = p.half.map_or(px, |(_, hp)| 0.5 * hp + 0.5 * px);
        let exits = p.half.map_or(Vec::new(), |(hi, hp)| vec![(hi, hp, 0.5), (j, px, 0.5)]);
        let reason = if p.half.is_some() { format!("{}; rest: {why}", p.why_half) } else { why };
        out.push(Trade { dir: p.dir, legs: vec![Leg { i: p.i, px: p.px, w: 1.0 }], exit_i: j, exit_px, reason, entry_kind: p.kind, signal_i: sig, open, exits });
    };
    // Manage an open position on bar j; Some((price, reason)) = closed on this bar.
    let manage = |j: usize, p: &mut Pos, entry_bar: bool, setup_bar: bool, exit_open: &mut Option<String>| -> Option<(f64, String)> {
        let long = p.dir > 0.0;
        if !entry_bar {
            if let Some(n) = ex.trail.filter(|&n| n > 0 && j >= n && (!ex.trail_after_half || p.half.is_some())) {
                let lv = if long { l[j - n..j].iter().cloned().fold(f64::INFINITY, f64::min) } else { h[j - n..j].iter().cloned().fold(f64::NEG_INFINITY, f64::max) };
                p.stop = if long { p.stop.max(lv) } else { p.stop.min(lv) }; // NaN stop -> lv
            }
        }
        if p.stop.is_finite() {
            let mut hit = if long { l[j] <= p.stop } else { h[j] >= p.stop };
            let gapped = if long { o[j] <= p.stop } else { o[j] >= p.stop };
            if entry_bar {
                // exempt entry bar (opened beyond the stop / 8-Day condition-3 bar): the extreme may have come
                // before the fill, but a close beyond the stop after a fill on the safe side means the stop was
                // certainly crossed after the fill -> out at the stop
                let safe_fill = if long { p.px >= p.stop } else { p.px <= p.stop };
                let closed_beyond = if long { c[j] < p.stop } else { c[j] > p.stop };
                hit &= (!gapped && !setup_bar) || (safe_fill && closed_beyond);
            }
            if hit {
                return Some((if gapped && !entry_bar { o[j] } else { p.stop }, "stop".into()));
            }
        }
        let reach = |tg: f64| if long { h[j] >= tg } else { l[j] <= tg };
        let at = |tg: f64| if entry_bar { tg } else if long { o[j].max(tg) } else { o[j].min(tg) };
        if let Some(t) = ex.target_pct {
            let tg = p.px * (1.0 + p.dir * t);
            if reach(tg) {
                return Some((at(tg), format!("{:.0}% profit target", t * 100.0)));
            }
        }
        if p.half.is_none() {
            let tg = if ex.half_at_r && p.risk > 0.0 {
                Some((p.px + p.dir * p.risk, "half off at +1R (2-for-1)".to_string()))
            } else {
                ex.half_at_pct.map(|f| (p.px * (1.0 + p.dir * f), format!("half covered at {:.0}% profit", f * 100.0)))
            };
            if let Some((tg, why)) = tg {
                if reach(tg) {
                    p.half = Some((j, at(tg)));
                    p.why_half = why;
                }
            }
        }
        if let Some(n) = ex.hold {
            if j - p.i >= n {
                return Some((c[j], format!("{n} bars held, out on the close")));
            }
        }
        if let Some(rule) = ex.rule {
            match rule(j, p) {
                Some(Act::Close(w)) => return Some((c[j], w)),
                Some(Act::NextOpen(w)) if j < to => *exit_open = Some(w),
                Some(Act::HalfClose(w)) if p.half.is_none() => {
                    p.half = Some((j, c[j]));
                    p.why_half = w;
                }
                _ => {}
            }
        }
        None
    };
    for j in from..=to {
        if let Some(mut p) = pos.take() {
            if let Some(w) = exit_open.take() {
                done(p, j, j - 1, o[j], w, &mut out, false);
            } else if let Some((px, w)) = manage(j, &mut p, false, false, &mut exit_open) {
                done(p, j, j, px, w, &mut out, false);
            } else {
                pos = Some(p);
            }
            continue;
        }
        while first < orders.len() && orders[first].from + 64 < j {
            first += 1;
        }
        for od in &orders[first..] {
            if od.from > j {
                break;
            }
            if j > od.last {
                continue;
            }
            let long = od.dir > 0.0;
            let px = match od.fill {
                Fill::Close => (j == od.from).then(|| c[j]),
                Fill::Open => (j == od.from).then(|| o[j]),
                Fill::Stop(lv) => {
                    let through = if long { o[j] >= lv } else { o[j] <= lv };
                    if j == od.setup && through {
                        None // the setup bar opened beyond the level: the breakout came before the setup
                    } else if long && h[j] >= lv {
                        Some(o[j].max(lv))
                    } else if !long && l[j] <= lv {
                        Some(o[j].min(lv))
                    } else {
                        None
                    }
                }
            };
            let Some(px) = px else { continue };
            let kind = match od.fill { Fill::Close => "close", Fill::Open => "open", Fill::Stop(_) => "stop" };
            let risk = if od.stop.is_finite() { (px - od.stop).abs() } else { 0.0 };
            let mut p = Pos { dir: od.dir, i: j, px, stop: od.stop, risk, half: None, why_half: String::new(), kind };
            if matches!(od.fill, Fill::Close) {
                pos = Some(p);
            } else if let Some((xp, w)) = manage(j, &mut p, true, j == od.setup, &mut exit_open) {
                done(p, j, j, xp, w, &mut out, false);
            } else {
                pos = Some(p);
            }
            break;
        }
    }
    if let Some(p) = pos {
        done(p, to, to, c[to], "open".into(), &mut out, true);
    }
    out
}

// ---------------------------------------------------------------- catalogue

const CBP: &[&str] = &[
    "F|\"Identify a stock whose weekly ADX reading is above 60.\" (period not stated: Wilder 14 on weekly bars)",
    "F|\"The stocks price should be above 20, as I have found this setup to work even better on higher priced stocks.\"",
    "E|\"After the weekly ADX reading reaches 60, wait for the first lower ADX weekly reading to occur.\"",
    "E|\"When the above 3 rules are met, short the stock and risk no more than 15 percent.\" Short on the close of the downtick week",
    "X|\"I recommend a 15 percent stop-out point\": protective buy stop at entry × 1.15",
    "X|\"Cover one-half the position upon a 20 percent profit and use trailing stops on the other half.\"",
    "X|[Not from the book] the trailing stop for the other half: the highest high of the prior 20 days",
];
const DVMTM: &[&str] = &[
    "F|\"A stock must be trading near or at a three-month calendar-day high.\" Read: today's high ≥ the highest high of the prior 63 days",
    "F|\"Today's volume must be double its 15-day average volume.\" (average of the prior 15 days)",
    "F|\"Either today, tomorrow, or the next day, the stock must close below its open.\"",
    "E|\"When Rule 3 is met, over the next two days sell 1/8 under the Rule 3 day low.\"",
    "X|\"Your initial protective stop should be the top of the Rule 3 day bar\"",
    "X|\"and trailing stops should be used to lock-in profits.\"",
    "X|[Not from the book] the trail: a trailing stop at the prior 3-day high (trail_n 3). Optional, off by default: mm = 1 adds the 2-for-1 half exit at +1× initial risk (ATS ch. 28), max_hold > 0 a time cap",
];
const BOTREV: &[&str] = &[
    "F|\"A market must be in a strong downtrend. Ideally, I would like to see it trading both under its 50-day and 20-day simple moving average.\"",
    "F|\"The market should come off its bottom and have a brief, one to three day rally.\" Read: a 10-day low, then 1-3 higher closes",
    "F|\"The market will then have a bottom range reversal bar. This means its close will be below the opening and the close will be near the bottom of the days range.\" Read: close in the bottom 25%",
    "E|\"The next day, the market will strongly reverse and we will go long 1 tick above the previous day's high.\"",
    "X|\"Keep a fairly tight stop\" Read: the reversal bar's low",
    "X|\"and trail your gains for the next one to five days.\" Out on the close of day 5 at the latest",
    "X|[Not from the book] the trail: a trailing stop at the prior 3-day low (trail_n 3) inside the book's 5 days. Optional, off by default: mm = 1 adds the 2-for-1 half exit at +1× initial risk (ATS ch. 28)",
];
const LRD_COMBO: &[&str] = &[
    "F|\"a market which closed two standard deviations ... from its previous day's close\" (1 SD = 100-day HV / √256 × price; the box prints √265)",
    "F|\"in strongly trending markets (ADX greater than 25), this principle does not hold true.\"",
    "E|\"sell the overpriced options via a naked combination. You can sell the calls that are one standard deviation from the large day move's high and the puts that are one standard deviation from the big move's low.\"",
    "X|\"If the market moves sideways the premiums on both sides will collapse.\" No expiry, stop or exit given",
];
const LRD_STUDY: &[&str] = &[
    "F|A close two standard deviations from the previous close: \"this occurs approximately once out of every twenty trading days\"",
    "F|\"in strongly trending markets (ADX greater than 25), this principle does not hold true.\"",
    "X|Claim: \"you could flip a coin as to whether the move would be up or down the next day\"",
    "X|Claim: \"the market many times closed near the closing price of the large move day, three to four days later.\"",
];
const MGAP: &[&str] = &[
    "F|\"The 14 day ADX must be above 25 and the 14 day +DI must be greater than the 14 day –DI.\" (sells reversed)",
    "E|\"Today, the market gaps higher.\" Read: open above yesterday's close (parameter: above yesterday's high)",
    "E|\"Enter the market near the close.\"",
    "X|\"Use a protective stop\" [Not from the book] the level: the gap day's low (high for sells)",
    "X|\"and trail the position as it rises.\"",
    "X|[Not from the book] the trail: a trailing stop at the prior 3-day low (trail_n 3). Optional, off by default: mm = 1 adds the 2-for-1 half exit at +1× initial risk (ATS ch. 28), max_hold > 0 a time cap",
];
const TDP25: &[&str] = &[
    "F|\"Test 1: ADX > 25, +DI > –DI.\" (14-day; sells reversed: –DI > +DI)",
    "E|\"When the market has three consecutive down days, we bought on the close of the third day.\" (sells: three up closes, sell the close)",
    "X|\"We mechanically exited after two trading days and we did not use protective stops.\"",
];
const TDP30: &[&str] = &[
    "F|\"Test 2: ADX > 30, +DI > –DI.\" (14-day; sells reversed: –DI > +DI)",
    "E|\"When the market has three consecutive down days, we bought on the close of the third day.\" (sells: three up closes, sell the close)",
    "X|\"We mechanically exited after two trading days and we did not use protective stops.\"",
];
const TURTLE: &[&str] = &[
    "F|\"A commodity must be trading under both its 20-day moving average and its 50-day moving average.\" (sells: above both)",
    "F|\"Today's range should be the largest or the second largest of the past 10 days. Also, its close must be above the open (thrust day).\"",
    "E|\"Tomorrow or the next day, buy 1 tick above the thrust-day high.\" (sells: 1 tick under the thrust-day low)",
    "X|\"Risk no more than the range of the thrust day or less.\" Stop = entry level − the thrust-day range",
    "X|\"Trail your stop to lock in profits.\" [Not from the book] the trail: the whole position's stop ratchets to the prior 3-day low (shorts: high) from the bar after entry (trail_n 3). No time cap (max_hold 0; > 0 adds one)",
    "X|\"Also, if you are inclined, apply the 2-for-1 Money Management Strategy to the method.\" Optional, so off by default (mm 0); mm = 1 takes half off at +1× initial risk (ATS ch. 28) and the trail continues on the rest. [Not from the book] previous reading: mm = 1 by default",
];
const SMTP_L: &[&str] = &[
    "F|\"Today the market must make a 10-period low.\"",
    "F|\"Today's range must be the largest range of the past 10 bars.\"",
    "F|\"Today's close must be in the top 25 percent of today's range.\"",
    "E|\"Tomorrow, or the trading day after tomorrow, buy one tick (1/8 for stocks) above today's high.\"",
    "X|\"If filled, place an initial protective sell stop at today's low\"",
    "X|\"and trail it appropriately to lock in profits.\"",
    "X|[Not from the book] the trail: a trailing stop at the prior 3-day low (trail_n 3). Optional, off by default: mm = 1 adds the 2-for-1 half exit at +1× initial risk (ATS ch. 28), max_hold > 0 a time cap",
];
const SMTP_S: &[&str] = &[
    "F|(Sells are reversed) \"Today's high is a 10-period high\"",
    "F|\"the range is the largest of the past 10 trading sessions\"",
    "F|\"the close is in the bottom 25 percent of its range\"",
    "E|\"A sell short stop is triggered 1/8 below yesterday's low\" (tomorrow or the day after)",
    "X|\"our protective stop is at yesterday's high\"",
    "X|\"trail it appropriately to lock in profits.\"",
    "X|[Not from the book] the trail: a trailing stop at the prior 3-day high (trail_n 3). Optional, off by default: mm = 1 adds the 2-for-1 half exit at +1× initial risk (ATS ch. 28), max_hold > 0 a time cap",
];
const R1234_L: &[&str] = &[
    "F|\"Identify a market whose 14-day ADX is greater than 30. The higher the ADX, the better.\"",
    "F|\"The 14-day +DI reading must be greater than the 14-day –DI reading.\"",
    "F|\"Wait for the market to have a 1-2-3 correction. This means that the market must make three consecutive intraday lower lows or any combination of two lower lows and an inside day.\"",
    "E|\"On day four only, buy 1 tick (1/8 for equities) above the day-three high.\"",
    "X|\"When filled, your initial protective stop should be near the day-three low.\"",
    "X|\"As the position moves in your favor, you should trail your stops.\"",
    "X|[Not from the book] the trail: a trailing stop at the prior 3-day low (trail_n 3). Optional, off by default: mm = 1 adds the 2-for-1 half exit at +1× initial risk (ATS ch. 28), max_hold > 0 a time cap",
];
const R1234_S: &[&str] = &[
    "F|\"ADX > 30. The higher the ADX, the better.\"",
    "F|\"The –DI must be greater than the +DI.\"",
    "F|\"Wait for a 1-2-3 rally. Three higher highs or any combination or two higher highs and an inside day.\"",
    "E|\"On day four only, sell (short for equities) 1 tick (1/8) below the day-three low.\"",
    "X|\"Your initial protective stop should be near the day-three high.\"",
    "X|\"As the position moves in your favor, trail your stops.\"",
    "X|[Not from the book] the trail: a trailing stop at the prior 3-day high (trail_n 3). Optional, off by default: mm = 1 adds the 2-for-1 half exit at +1× initial risk (ATS ch. 28), max_hold > 0 a time cap",
];
const D8_L: &[&str] = &[
    "F|\"Day one must be an eight-day low.\"",
    "F|\"Day two must trade above the day-one high.\"",
    "F|\"Days three or four or five or six must trade under the day-two low (this can be a new low).\"",
    "E|\"When condition three is satisfied, we buy one tick (1/8 for equities) above the day-two high as long as the one-tick breakout occurs within four trading days of condition three.\"",
    "X|\"Our stop goes one tick below the day-two low.\"",
    "X|[Not from the book] the trail: a trailing stop at the prior 3-day low (trail_n 3). Optional, off by default: mm = 1 adds the 2-for-1 half exit at +1× initial risk (ATS ch. 28), max_hold > 0 a time cap",
];
const D8_S: &[&str] = &[
    "F|\"Day one must be an eight-day high.\"",
    "F|\"Day two must trade below the day-one low.\"",
    "F|\"Days three, four, five, or six must trade above the day-two high (this can be a new high).\"",
    "E|\"When condition three is satisfied, we sell (short) one tick (1/8 for equities) below the day-two low as long as it occurs within four trading days.\"",
    "X|\"Our stop is placed one tick above the day-two high.\"",
    "X|[Not from the book] the trail: a trailing stop at the prior 3-day high (trail_n 3). Optional, off by default: mm = 1 adds the 2-for-1 half exit at +1× initial risk (ATS ch. 28), max_hold > 0 a time cap",
];
const OOPS_L: &[&str] = &[
    "F|\"Day one—A commodity or stock must close in the bottom 10 percent of its daily range.\"",
    "E|\"Day two—The next morning, it must gap lower (night data is omitted).\" Read: open below the day-one low",
    "E|\"If Rule 2a is met, buy at the day-one low.\"",
    "X|\"Your initial protective stop should be near the day-two opening.\" (cannot be sequenced on daily bars; the published test used none)",
    "X|\"Lock in profits with a trailing stop or sell MOC.\" Tested as the book's mechanical test: \"with no stops\", sell MOC",
];
const OOPS_S: &[&str] = &[
    "F|\"Day one—A commodity or stock must close in the top 10 percent of its range.\"",
    "E|\"Day two—The next morning it must gap higher.\" Read: open above the day-one high",
    "E|\"If Rule 2a is met, sell at day-one high.\"",
    "X|\"Your initial protective stop should be near the day-two opening.\" (the published test used none)",
    "X|\"Buy MOC or take profits via a trailing stop.\" Tested: cover MOC",
];
const OOPS_B: &[&str] = &[
    "F|Buys: \"Day one—A commodity or stock must close in the bottom 10 percent of its daily range.\" Sells: \"close in the top 10 percent of its range.\"",
    "E|\"Day two—The next morning, it must gap lower\" (sells: gap higher). Read: open beyond the day-one low / high",
    "E|\"buy at the day-one low\" / \"sell at day-one high\"",
    "X|\"Here is how the 10% Oops strategy fared in 1996 as a mechanical system with no stops\": out MOC the same day",
];
const MMA_L: &[&str] = &[
    "F|\"The 5-day moving average of a market must cross above the 20-day moving average. This signifies an up trend.\"",
    "F|\"The first time (only) that prices close below the 5-day moving average gives you the first piece of the signal.\"",
    "E|\"If prices then close at or above the 5-day moving average within 3 days, buy the market on the opening the next day, otherwise the signal is canceled.\"",
    "X|\"Your initial protective stop should be near the low of the day before the entry.\"",
    "X|\"Trail your stops and scale out as the position becomes more profitable.\"",
    "X|[Not from the book] \"scale out\" read as the 2-for-1 (mm 1): half off at +1× initial risk; trail the rest at the prior 3-day low. No time cap (max_hold 0; > 0 adds one)",
];
const MMA_S: &[&str] = &[
    "F|(Sells are reversed) The 5-day moving average crosses below the 20-day moving average",
    "F|The first time (only) that prices close above the 5-day moving average",
    "E|\"The next day it closes at the 5-day moving average, triggering a signal.\" A close at or below the 5-day MA within 3 days: sell on the next day's opening",
    "X|Initial protective stop near the high of the day before the entry",
    "X|\"Trail your stops and scale out as the position becomes more profitable.\"",
    "X|[Not from the book] \"scale out\" read as the 2-for-1 (mm 1): cover half at +1× initial risk; trail the rest at the prior 3-day high. No time cap (max_hold 0; > 0 adds one)",
];
const GIP_L: &[&str] = &[
    "F|\"I use a 14 period ADX reading and I look for readings above 25. ... I also identify the trend with +DI and –DI.\" Long: +DI > –DI",
    "F|\"I then wait for its 6/100 day historical volatility reading to drop under .40. This means the six-day reading is less than 40 percent of the 100-day reading.\"",
    "E|\"A long entry is triggered for uptrending markets when the high of the signal day is exceeded\" ([Not from the book] the order lives the next day only; the book gives no order life. A new qualifying day becomes the new signal day)",
    "X|\"For uptrending markets, my initial protective stop is usually near the low of the signal day\"",
    "X|\"c. My recommendation: Exit half your position and tighten your stops on the other half when the 6-day H.V. reading reaches the 100-day reading.\" (default exit)",
    "X|\"b. ... for stocks you should look to exit at profit levels of at least 5 to 10 percent\" (exit mode 2); \"a. Follow the price with a trailing stop\" (exit mode 3)",
    "X|[Not from the book] \"tighten\" / \"trailing stop\" = the prior 3-day low",
];
const GIP_S: &[&str] = &[
    "F|\"I use a 14 period ADX reading and I look for readings above 25.\" Short: –DI > +DI",
    "F|\"I then wait for its 6/100 day historical volatility reading to drop under .40.\"",
    "E|\"a short entry is triggered for downtrending markets when the low of the signal day is taken out.\" ([Not from the book] the order lives the next day only)",
    "X|\"for downtrending markets, my initial protective stop is near the high of the signal day.\"",
    "X|\"c. My recommendation: Exit half your position and tighten your stops on the other half when the 6-day H.V. reading reaches the 100-day reading.\" (default exit)",
    "X|\"b. ... exit at profit levels of at least 5 to 10 percent\" (exit mode 2); \"a. Follow the price with a trailing stop\" (exit mode 3)",
    "X|[Not from the book] \"tighten\" / \"trailing stop\" = the prior 3-day high",
];
const WREG_L: &[&str] = &[
    "F|\"Day 1: Identify a market or stock that has had at least a two standard deviation daily move to the downside.\" 1 SD = price × daily SD of 100-day log returns (the box: annualised HV / √256, its arithmetic; it prints √265)",
    "E|\"Day 2: Tomorrow, if this market gaps lower (night data is omitted), place a buy stop near the previous day's close (not its low!).\" Read: open below the day-one low, buy stop at the day-one close",
    "X|\"Your initial protective stop should be placed at the day-one low.\"",
    "X|\"give your stop some additional breathing room\" (when the close is near the low): parameter breath_sd, default 0; [Not from the book] the amount, N × 1 SD below the day-one close",
    "X|Entry day: the stop counts only if day two closes below it (the open was already under it, so the low may have come before the fill); then out at the stop",
    "X|\"If the position closes in the top 10 to 15 percent of its daily range, carry it into the next morning. Otherwise, lock in your profits near the close.\" Read: carried trades exit on the next open (the book gives no exit time)",
];
const WREG_S: &[&str] = &[
    "F|(Reversed) Day 1: at least a two standard deviation daily move to the upside",
    "E|Day 2: if the market gaps higher, place a sell stop near the previous day's close (\"The stock gaps to 41 3/4 and our short sale is triggered near yesterday's close at 39 7/8\")",
    "X|Initial protective stop at the day-one high (breathing room: parameter breath_sd, default 0; [Not from the book] the amount, N × 1 SD above the day-one close)",
    "X|Entry day: the stop counts only if day two closes above it; then out at the stop",
    "X|If the position closes in the bottom 10 to 15 percent of its daily range, carry it into the next morning. Otherwise, lock in your profits near the close. Read: carried trades exit on the next open (the book gives no exit time)",
];

pub fn catalogue() -> Vec<StrategyDef> {
    use Status::*;
    let d = |id, name, group, dir, status, rules, notes, claim| StrategyDef {
        id, name, group, session: 0, dir, status, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim,
    };
    let t = "Triple-Day Pullbacks (Vol. I)";
    let tdp = "Book: 15 futures over 'this decade' (~1990-98), one row per market; the vault has no futures, so the reproduction runs SPY/QQQ/MDY. Both sides in one test, one position at a time. The streak must be exactly 3 on the entry day ('three consecutive down days'), so a 4th or 5th down close does not re-enter. Not in Advanced Trading Strategies. India: runs directly on the NSE indices.";
    vec![
        d("btp_cbp_short", "Crash, Burn, and Profit (CBP)", "Crash, Burn, and Profit (Vol. I)", "Short", Live, CBP,
          "Weekly bars are built from the daily file (Monday-Friday weeks); weekly ADX = Wilder 14. Fires once per excursion above 60 (re-arms when ADX falls back under 60). The book screens single high-flying stocks (IBD RS 98-99); a weekly ADX above 60 is very rare on indices and ETFs, so expect few or no trades. Same article as ATS (ats_cbp), same wording. The half cover at 20% is the book's; the trail on the other half is not (it starts after the half cover; trail_whole 1 = the earlier reading, whole position). US data only: the $20 filter is in dollars (connors-final 26 Oct 2026).", None),
        StrategyDef { needs_volume: true, ..d("btp_dvmtm_short", "Double-Volume Market Top (DVMTM)", "Double-Volume Market Top (Vol. I)", "Short", Live, DVMTM,
          "Book: single stocks. Needs volume (SPY/QQQ; NIFTY 50, BANK, MIDCAP 100). 'Three-month calendar-day high' = 63 trading days, tolerance parameter (book reading 0% = a new high, as in every example). Same article as ATS (ats_dvmtm), same wording. The exit after the initial stop is a labelled non-book set.", None) },
        d("btp_bottom_reversal_long", "Bottom Reversals", "Bottom Reversals (Vol. I)", "Long", Live, BOTREV,
          "The 10-day 'bottom', the 25% close location and the reversal-bar-low stop are programmer readings of 'strong downtrend ... bottom', 'near the bottom' and 'fairly tight stop'. Book: futures and stocks; run on index ETFs / NSE indices. Not in ATS.", None),
        d("btp_large_range_day_strangle", "Large-Range Days · naked combination", "Large-Range Days (Vol. I)", "Long/Short", NotTestable("needs option chains"), LRD_COMBO,
          "Options trade with no expiry or exit. Same article as ATS ch. 27 (ats_large_range_day_combo) and the TradingMarkets reprint (lrd). The book lists 44 dated two-SD T-bond days (1/13/95-10/29/96) without outcomes; the vault has no bond futures.", None),
        d("btp_large_range_day_study", "Large-Range Days · follow-through study", "Large-Range Days (Vol. I)", "Long/Short", NotTestable("event study: claims, not entry and exit rules"), LRD_STUDY,
          "A base-rate observation, not a trading system: no direction, entry or exit to backtest.", None),
        d("btp_momentum_gap", "Momentum Gaps", "Momentum Gaps (Vol. I)", "Long/Short", Live, MGAP,
          "Both sides in one test. The gap-day-low stop and the exit set are not from the book ('use a protective stop and trail'); the book warns that 1-5 day pullbacks after the gap are common. Book: futures and stocks; run on index ETFs / NSE indices. Not in ATS.", None),
        d("btp_triple_day_pullback_adx25", "Triple-Day Pullbacks · ADX > 25", t, "Long/Short", Live, TDP25, tdp,
          claim("598 (364 correct)", "60.87%", "-", "2 days", "~1990-1998, 15 futures; SP row 42 trades, 57%")),
        d("btp_triple_day_pullback_adx30", "Triple-Day Pullbacks · ADX > 30", t, "Long/Short", Live, TDP30, tdp,
          claim("377 (237 correct)", "62.86%", "-", "2 days", "~1990-1998, 15 futures; SP row 32 trades, 56%")),
        d("btp_turtle_thrust", "Turtle Thrusts", "Turtle Thrusts (Vol. I)", "Long/Short", Live, TURTLE,
          "Both sides. SMA (the volume says 'simple' elsewhere). The author recommends it for futures only ('trend followers ... play only a small role in the equities markets'); run on index ETFs / NSE indices. 2-for-1 is the book's optional suggestion (on by default); the trail on the second half is not from the book; no time cap by default (max_hold > 0 adds one, [Not from the book]). Not in ATS.", None),
        d("btp2_smtp_long", "Spent Market Trading Pattern · buys", "Spent Market Trading Pattern (Vol. II)", "Long", Live, SMTP_L,
          "Same article as ATS (ats_smtp), same wording. 10-period low = lowest low of 10 bars incl. today; range = high − low. Exit after the stop is a labelled non-book set. Book: stocks and futures; run on index ETFs / NSE indices.", None),
        d("btp2_smtp_short", "Spent Market Trading Pattern · sells", "Spent Market Trading Pattern (Vol. II)", "Short", Live, SMTP_S,
          "Sells reversed, wording from the Compaq / US Bond examples. Same as ATS (ats_smtp).", None),
        d("btp2_1234_long", "1-2-3-4 · buys", "1-2-3-4 (Vol. II)", "Long", Live, R1234_L,
          "Same article as ATS (ats_1234). Lower low = L < L[1]; inside day = H < H[1] and L > L[1]; days 1-3 each a lower low or an inside day, at most one inside day; ADX/DI on day 3. A day-4 gap above the level fills at the open (Gucci example). Run on index ETFs / NSE indices.", None),
        d("btp2_1234_short", "1-2-3-4 · sells", "1-2-3-4 (Vol. II)", "Short", Live, R1234_S, "Mirror of the buys. Same as ATS (ats_1234).", None),
        d("btp2_8day_hl_reversal_long", "8-Day High/Low Reversal · buys", "8-Day High/Low Reversal (Vol. II)", "Long", Live, D8_L,
          "Same article as ATS (ats_8day_hl_reversal). Entry window = the condition-3 bar plus the next four (S&P example); a same-bar condition 3 + breakout is allowed (Microsoft example) unless that bar opened above the level, and is not stopped out that bar. The book gives only the stop; the rest of the exit is a labelled non-book set. Run on index ETFs / NSE indices.", None),
        d("btp2_8day_hl_reversal_short", "8-Day High/Low Reversal · sells", "8-Day High/Low Reversal (Vol. II)", "Short", Live, D8_S, "Mirror of the buys (IBM example). Same as ATS.", None),
        d("btp2_10pct_oops_long", "10% Oops · buys", "10% Oops (Vol. II)", "Long", Live, OOPS_L,
          "Same article as ATS (ats_10pct_oops). Gap = open below the day-one low (Oops convention). Fill at the day-one low, out MOC. Run on index ETFs / NSE indices; check the index open quality before trusting gaps. Intraday wave 1: run on real 1-minute bars by default (p.path=1): the entry fills on the first minute that reaches it, at the level or at that minute's open when it opened through it; out on the official daily close. Instruments: SPY, SPX (from 2014, real daily opens), NIFTY 50 and NIFTYBANK, the only ones with cleaned minute data (declared application); days without minute data make no trade. p.path=0 = the old daily-bar reading [Not from the book].", None),
        d("btp2_10pct_oops_short", "10% Oops · sells", "10% Oops (Vol. II)", "Short", Live, OOPS_S, "Mirror of the buys. Same as ATS. Intraday wave 1: run on real 1-minute bars by default (p.path=1): the entry fills on the first minute that reaches it, at the level or at that minute's open when it opened through it; out on the official daily close. Instruments: SPY, SPX (from 2014, real daily opens), NIFTY 50 and NIFTYBANK, the only ones with cleaned minute data (declared application); days without minute data make no trade. p.path=0 = the old daily-bar reading [Not from the book].", None),
        d("btp2_10pct_oops", "10% Oops · buys and sells (the 1996 test)", "10% Oops (Vol. II)", "Long/Short", Live, OOPS_B,
          "Both sides, as in the book's S&P futures table (ATS prints the same table). The table's 'signal date' is day two (the trade day). $19,880 is after $100 per trade; at $500/point the printed points give $20,375, a $495 gap the book does not explain. SPY opens differ from S&P futures day-session opens, so the dates match only partly. Intraday wave 1: run on real 1-minute bars by default (p.path=1): the entry fills on the first minute that reaches it, at the level or at that minute's open when it opened through it; out on the official daily close. Instruments: SPY, SPX (from 2014, real daily opens), NIFTY 50 and NIFTYBANK, the only ones with cleaned minute data (declared application); days without minute data make no trade. p.path=0 = the old daily-bar reading [Not from the book].",
          claim("15 (12 profitable / 3 unprofitable)", "80%", "+2.92 S&P pts (43.75 pts in all)", "same day (MOC)", "S&P futures 1/1/96-12/31/96, no stops; $19,880 after $100/trade")),
        d("btp2_momentum_ma_long", "Momentum Moving Averages · buys", "Momentum Moving Averages (Vol. II)", "Long", Live, MMA_L,
          "SMA. 'First time (only)': one signal per 5/20 up-cross regime; the regime ends when the 5-day crosses back under. Not in ATS. Run on index ETFs / NSE indices.", None),
        d("btp2_momentum_ma_short", "Momentum Moving Averages · sells", "Momentum Moving Averages (Vol. II)", "Short", Live, MMA_S, "Mirror of the buys (Hogs / CLZ8 examples).", None),
        d("btp2_gipsons_long", "Gipsons · long", "Gipsons (Vol. II)", "Long", Live, GIP_L,
          "Same article as ATS (ats_gipsons), same wording. HV = sample stdev of the N−1 most recent log returns, the PTJ definition printed in the ATS appendix (the ratio is annualisation-free); it puts the S&P example's signal on Nov 13, 1996 as in the book. [Not from the book] The order lives the next day only (the book says only \"when the high of the signal day is exceeded\"); each new qualifying day re-arms it (Pediatrix example). Default exit is the author's recommendation (c). Run on index ETFs / NSE indices.", None),
        d("btp2_gipsons_short", "Gipsons · short", "Gipsons (Vol. II)", "Short", Live, GIP_S, "Mirror of the long. Same as ATS.", None),
        d("btp2_wide_range_exhaustion_gap_long", "Wide-Range Exhaustion Gap Reversal · buys", "Wide-Range Exhaustion Gap Reversals (Vol. II)", "Long", Live, WREG_L,
          "Same article as ATS (ats_wide_range_exhaustion_gap_reversal). 2-SD move = close to close, HV100 measured the day before the move. Gap = open below the day-one low. The market opened under the stop, so the day-two low is taken to come before the entry and the entry day is not stopped out by its low; if day two closes below the stop, price certainly crossed it after the fill and the trade exits at the stop (the ATS extraction suggests a fully conservative reading, which makes every trade a loser). 1 SD uses the daily SD of log returns (the box's annualised HV ÷ √256); the old √252/√256 mix (1.984 SD) is gone. Default tick = one real tick per market [Not from the book]: 0.05 NSE, 0.01 US (the book's 1/8 = tick 0.125). Run on index ETFs / NSE indices.", None),
        d("btp2_wide_range_exhaustion_gap_short", "Wide-Range Exhaustion Gap Reversal · sells", "Wide-Range Exhaustion Gap Reversals (Vol. II)", "Short", Live, WREG_S,
          "Mirror of the buys (America Online example). Same as ATS. Intraday wave 1: run on real 1-minute bars by default (p.path=1): the entry fills on the first minute that reaches it, at the level or at that minute's open when it opened through it; out on the official daily close. Instruments: SPY, SPX (from 2014, real daily opens), NIFTY 50 and NIFTYBANK, the only ones with cleaned minute data (declared application); days without minute data make no trade. p.path=0 = the old daily-bar reading [Not from the book]. A minute that fills and also reaches the protective stop counts as filled then stopped, unless it opened beyond the stop: then only if it closed beyond it [convention]. Carried positions exit at the next day's open. The buys stay on their admitted daily reading (p.path=1 is not offered there).", None),
    ]
}

// ---------------------------------------------------------------- parameters

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    let tick = || p("tick", "Entry offset: 1 tick (−1 = [Not from the book] one real tick per market: 0.05 on NSE NIFTY indices and Indian stocks (INR), 0.01 on US indices/ETFs and US stocks (USD); adaptation: the book used 1/8-dollar ticks = 0.125)", -1.0, -1.0, 0.5, 0.005);
    let kit = |mm: f64, hold: f64| [
        p("mm", "2-for-1: half off at +1× initial risk (1 = on; book suggests it only for Turtle Thrusts / Momentum MAs 'scale out')", mm, 0.0, 1.0, 1.0),
        p("trail_n", "[Not in book] trailing stop at the prior N-day low/high (0 = off)", 3.0, 0.0, 10.0, 1.0),
        p("max_hold", if hold == 5.0 { "Out on the close of day N (book: 'next one to five days')" } else { "[Not in book] out on the close after N days (0 = off)" }, hold, 0.0, 30.0, 1.0),
    ];
    let mut v: Vec<ParamDef> = match id {
        "btp_cbp_short" => vec![p("adx_len", "Weekly ADX period", 14.0, 10.0, 20.0, 1.0), p("adx_level", "Weekly ADX above", 60.0, 40.0, 70.0, 5.0),
            p("min_price", "Price above", 20.0, 0.0, 50.0, 5.0), p("stop_pct", "Protective buy stop % above entry", 15.0, 5.0, 25.0, 5.0),
            p("target_pct", "Cover half at % profit", 20.0, 10.0, 40.0, 5.0), p("trail_n", "[Not in book] trail the other half at the prior N-day high", 20.0, 5.0, 60.0, 5.0),
            p("trail_whole", "Trail: 0 = the other half after the half cover (book), 1 = [old vault reading] whole position from the day after entry", 0.0, 0.0, 1.0, 1.0)],
        "btp_dvmtm_short" => vec![p("high_lookback", "3-month high: prior N days", 63.0, 40.0, 90.0, 5.0), p("near_high_pct", "'Near' the high: % tolerance (0 = new high)", 0.0, 0.0, 3.0, 0.5),
            p("vol_avg_len", "Volume average days", 15.0, 10.0, 30.0, 5.0), p("vol_mult", "Volume ≥ N × average", 2.0, 1.5, 3.0, 0.25),
            p("rule3_window", "Close < open within N days (incl. the volume day)", 3.0, 1.0, 3.0, 1.0), p("entry_window", "Sell stop live N days", 2.0, 1.0, 3.0, 1.0), tick()],
        "btp_bottom_reversal_long" => vec![p("sma_fast", "Below SMA (fast)", 20.0, 10.0, 30.0, 5.0), p("sma_slow", "Below SMA (slow)", 50.0, 30.0, 100.0, 10.0),
            p("bottom_lookback", "Bottom = lowest low of N days (reading)", 10.0, 5.0, 20.0, 5.0), p("rally_min", "Rally days min", 1.0, 1.0, 3.0, 1.0),
            p("rally_max", "Rally days max", 3.0, 1.0, 5.0, 1.0), p("close_pos_max", "Reversal bar close in bottom fraction (reading)", 0.25, 0.1, 0.4, 0.05), tick()],
        "btp_momentum_gap" => vec![p("gap_mode", "Gap: 0 = open beyond prior close, 1 = beyond prior high/low", 0.0, 0.0, 1.0, 1.0)],
        "btp_triple_day_pullback_adx25" | "btp_triple_day_pullback_adx30" => vec![p("streak", "Consecutive down (up) closes", 3.0, 2.0, 5.0, 1.0), p("hold_days", "Exit after N trading days", 2.0, 1.0, 5.0, 1.0)],
        "btp_turtle_thrust" => vec![p("ma_fast", "SMA (fast)", 20.0, 10.0, 30.0, 5.0), p("ma_slow", "SMA (slow)", 50.0, 30.0, 100.0, 10.0),
            p("range_lookback", "Range rank over N days (incl. today)", 10.0, 5.0, 20.0, 5.0), p("range_rank", "Range rank at most", 2.0, 1.0, 3.0, 1.0),
            p("entry_window", "Buy stop live N days", 2.0, 1.0, 3.0, 1.0), p("stop_range_mult", "Stop: × thrust-day range", 1.0, 0.5, 1.0, 0.25), tick()],
        "btp2_smtp_long" | "btp2_smtp_short" => vec![p("lookback", "Extreme and largest range over N bars", 10.0, 5.0, 20.0, 1.0),
            p("close_pct", "Close in top (bottom) fraction of range", 0.25, 0.1, 0.4, 0.05), p("entry_days", "Entry stop live N days", 2.0, 1.0, 3.0, 1.0), tick()],
        "btp2_1234_long" | "btp2_1234_short" => vec![tick()],
        "btp2_8day_hl_reversal_long" | "btp2_8day_hl_reversal_short" => vec![p("lookback", "Day one: N-day low (high)", 8.0, 5.0, 15.0, 1.0),
            p("cond3_window", "Condition 3 within N bars after day two", 4.0, 2.0, 6.0, 1.0), p("entry_window", "Breakout within N days of condition 3", 4.0, 2.0, 6.0, 1.0), tick()],
        "btp2_10pct_oops_long" | "btp2_10pct_oops_short" | "btp2_10pct_oops" => vec![p("close_pct", "Day-one close in bottom (top) fraction", 0.10, 0.05, 0.25, 0.05), /* intraday wave 1 */ p("path", "1 = real 1-minute bars (book rule; fills at the level or the minute's open; days without minute data make no trade), 0 = [Not from the book] old daily-bar reading", 1.0, 0.0, 1.0, 1.0)],
        "btp2_momentum_ma_long" | "btp2_momentum_ma_short" => vec![p("fast_ma", "Fast SMA", 5.0, 3.0, 10.0, 1.0), p("slow_ma", "Slow SMA", 20.0, 10.0, 50.0, 5.0),
            p("confirm_days", "Close back across within N days", 3.0, 1.0, 5.0, 1.0)],
        "btp2_gipsons_long" | "btp2_gipsons_short" => vec![p("hv_short", "Short HV days", 6.0, 4.0, 10.0, 1.0), p("hv_long", "Long HV days", 100.0, 50.0, 200.0, 10.0),
            p("hv_ratio_max", "HV short / HV long below", 0.40, 0.30, 0.60, 0.05),
            p("hv_n1", "HV over N−1 returns (1 = PTJ/ATS appendix definition, 0 = N returns)", 1.0, 0.0, 1.0, 1.0), tick(),
            p("exit_mode", "Exit: 1 = (c) half off when HV6 ≥ HV100, 2 = (b) % target, 3 = (a) trailing stop", 1.0, 1.0, 3.0, 1.0),
            p("target_pct", "Exit (b): profit target %", 5.0, 5.0, 10.0, 1.0), p("trail_n", "[Not in book] trailing stop at the prior N-day low/high", 3.0, 1.0, 10.0, 1.0)],
        "btp2_wide_range_exhaustion_gap_long" | "btp2_wide_range_exhaustion_gap_short" => vec![p("hv_len", "HV days", 100.0, 50.0, 200.0, 10.0),
            p("sd_mult", "Day-one move ≥ N standard deviations", 2.0, 1.5, 3.0, 0.25), p("breath_sd", "Stop breathing room: stop at least N × 1 SD beyond the day-one close (book: 'additional breathing room', unquantified; 0 = day-one low/high)", 0.0, 0.0, 1.0, 0.25),
            p("carry_close_pct", "Carry overnight if close in top (bottom) fraction", 0.15, 0.10, 0.15, 0.05)],
        _ => return None,
    };
    // intraday wave 1: the WREG sells were held for their same-day path (the buys stay on the admitted daily reading)
    if id == "btp2_wide_range_exhaustion_gap_short" {
        v.push(p("path", "1 = real 1-minute bars (book rule; fills at the level or the minute's open; days without minute data make no trade), 0 = [Not from the book] old daily-bar reading", 1.0, 0.0, 1.0, 1.0));
    }
    let adx = match id {
        "btp_triple_day_pullback_adx30" | "btp2_1234_long" | "btp2_1234_short" => Some(30.0),
        "btp_momentum_gap" | "btp_triple_day_pullback_adx25" | "btp2_gipsons_long" | "btp2_gipsons_short" => Some(25.0),
        _ => None,
    };
    if let Some(a) = adx {
        let mut w = vec![p("adx_len", "ADX / DI period", 14.0, 10.0, 20.0, 1.0), p("adx_min", "ADX above", a, 15.0, 45.0, 5.0)];
        w.append(&mut v);
        v = w;
    }
    match id {
        "btp_dvmtm_short" | "btp_momentum_gap" | "btp2_smtp_long" | "btp2_smtp_short" | "btp2_1234_long" | "btp2_1234_short"
        | "btp2_8day_hl_reversal_long" | "btp2_8day_hl_reversal_short" => v.extend(kit(0.0, 0.0)),
        "btp_turtle_thrust" => v.extend(kit(0.0, 0.0)), // the 2-for-1 is optional in the book ("if you are inclined")
        "btp2_momentum_ma_long" | "btp2_momentum_ma_short" => v.extend(kit(1.0, 0.0)),
        "btp_bottom_reversal_long" => v.extend(kit(0.0, 5.0)),
        _ => {}
    }
    Some(v)
}

// ---------------------------------------------------------------- rules

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    let g0 = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).unwrap_or_else(|| panic!("{id} has no param {k}")).def);
    // tick < 0 = one real tick of the market: 0.05 NSE (the vault's Indian tickers all start "NIFTY"), 0.01 US decimals.
    // A native stock uses its trusted market's tick (NSE 0.05 INR, US 0.01 USD; adaptation: the book used 1/8-dollar ticks).
    let g = |k: &str| match g0(k) { t if k == "tick" && t < 0.0 => x.market_tick(), v => v };
    let u = |k: &str| g(k).round().max(1.0) as usize;
    let n = x.c.len();
    if n < 3 {
        return Some(vec![]);
    }
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let ord = |dir: f64, fill: Fill, setup: usize, from: usize, last: usize, stop: f64| Ord { dir, fill, setup, from, last: last.min(n - 1), stop };
    let kit = || Ex {
        half_at_r: g("mm") >= 0.5,
        trail: Some(g("trail_n").round() as usize).filter(|&t| t > 0),
        hold: Some(g("max_hold").round() as usize).filter(|&t| t > 0),
        ..Default::default()
    };
    let long_only = id.ends_with("_long");
    let short_only = id.ends_with("_short");
    let mut orders: Vec<Ord> = Vec::new();

    let trades = match id {
        "btp_cbp_short" => {
            let (wk, last) = weekly(x.r);
            let (_, _, adx) = dmi3(&wk, u("adx_len"));
            let (lvl, minp, stop, tgt) = (g("adx_level"), g("min_price"), 1.0 + g("stop_pct") / 100.0, g("target_pct") / 100.0);
            let mut armed = true;
            for w in 1..wk.len() {
                if adx[w] < lvl {
                    armed = true; // re-arm once ADX is back under the level
                }
                if armed && adx[w - 1] > lvl && adx[w] < adx[w - 1] && wk[w].close > minp {
                    armed = false;
                    let i = last[w];
                    orders.push(ord(-1.0, Fill::Close, i, i, i, c[i] * stop));
                }
            }
            // connors-final 2026-10-26: the article trails "the other half", i.e. after the half cover (book default);
            // trail_whole 1 = the earlier vault reading (trail the whole position from the day after entry), labelled.
            sim(x, orders, &Ex { half_at_pct: Some(tgt), trail: Some(u("trail_n")), trail_after_half: g("trail_whole") < 0.5, ..Default::default() }, from, to)
        }
        "btp_dvmtm_short" => {
            let vol: Vec<f64> = x.r.iter().map(|r| r.volume.unwrap_or(f64::NAN)).collect();
            let (lb, tol, va, vm, r3, ew, tk) = (u("high_lookback"), g("near_high_pct") / 100.0, u("vol_avg_len"), g("vol_mult"), u("rule3_window"), u("entry_window"), g("tick"));
            for d in lb.max(va)..n {
                let hh = h[d - lb..d].iter().cloned().fold(f64::NEG_INFINITY, f64::max);
                let avg = vol[d - va..d].iter().sum::<f64>() / va as f64;
                if !(avg > 0.0 && vol[d] >= vm * avg && h[d] >= hh * (1.0 - tol)) {
                    continue;
                }
                if let Some(r) = (d..(d + r3).min(n)).find(|&k| c[k] < o[k]) {
                    orders.push(ord(-1.0, Fill::Stop(l[r] - tk), r, r + 1, r + ew, h[r]));
                }
            }
            sim(x, orders, &kit(), from, to)
        }
        "btp_bottom_reversal_long" => {
            let (f, s) = (x.sma_n(u("sma_fast")), x.sma_n(u("sma_slow")));
            let (bl, rmin, rmax, cp, tk) = (u("bottom_lookback"), u("rally_min"), u("rally_max"), g("close_pos_max"), g("tick"));
            for r in 1..n {
                if !(c[r] < o[r] && x.close_loc(r) <= cp && c[r] < f[r] && c[r] < s[r]) {
                    continue;
                }
                // bottom bar b = r-k-1 is an N-day low, then k higher closes, then the reversal bar r
                let rally = (rmin..=rmax).any(|k| r > k + 1 && x.lowest_low(r - k - 1, bl) && (r - k..r).all(|j| c[j] > c[j - 1]));
                if rally {
                    orders.push(ord(1.0, Fill::Stop(h[r] + tk), r, r + 1, r + 1, l[r]));
                }
            }
            sim(x, orders, &kit(), from, to)
        }
        "btp_momentum_gap" => {
            let (pd, md, adx) = dmi3(x.r, u("adx_len"));
            let (am, hi) = (g("adx_min"), g("gap_mode") >= 0.5);
            for i in 1..n {
                if !(adx[i] > am) {
                    continue;
                }
                if pd[i] > md[i] && o[i] > if hi { h[i - 1] } else { c[i - 1] } {
                    orders.push(ord(1.0, Fill::Close, i, i, i, l[i]));
                } else if md[i] > pd[i] && o[i] < if hi { l[i - 1] } else { c[i - 1] } {
                    orders.push(ord(-1.0, Fill::Close, i, i, i, h[i]));
                }
            }
            sim(x, orders, &kit(), from, to)
        }
        "btp_triple_day_pullback_adx25" | "btp_triple_day_pullback_adx30" => {
            let (pd, md, adx) = dmi3(x.r, u("adx_len"));
            let (am, k) = (g("adx_min"), u("streak"));
            for i in 0..n {
                if !(adx[i] > am) {
                    continue;
                }
                if pd[i] > md[i] && x.down_closes[i] == k {
                    orders.push(ord(1.0, Fill::Close, i, i, i, f64::NAN));
                } else if md[i] > pd[i] && x.up_closes[i] == k {
                    orders.push(ord(-1.0, Fill::Close, i, i, i, f64::NAN));
                }
            }
            sim(x, orders, &Ex { hold: Some(u("hold_days")), ..Default::default() }, from, to)
        }
        "btp_turtle_thrust" => {
            let (f, s) = (x.sma_n(u("ma_fast")), x.sma_n(u("ma_slow")));
            let (lb, rk, ew, m, tk) = (u("range_lookback"), u("range_rank"), u("entry_window"), g("stop_range_mult"), g("tick"));
            for t in lb..n {
                let rg = range(x, t);
                if (t + 1 - lb..=t).filter(|&k| range(x, k) > rg).count() >= rk {
                    continue; // not the largest or second largest range
                }
                if c[t] < f[t] && c[t] < s[t] && c[t] > o[t] {
                    let lv = h[t] + tk;
                    orders.push(ord(1.0, Fill::Stop(lv), t, t + 1, t + ew, lv - m * rg));
                } else if c[t] > f[t] && c[t] > s[t] && c[t] < o[t] {
                    let lv = l[t] - tk;
                    orders.push(ord(-1.0, Fill::Stop(lv), t, t + 1, t + ew, lv + m * rg));
                }
            }
            sim(x, orders, &kit(), from, to)
        }
        "btp2_smtp_long" | "btp2_smtp_short" => {
            let (lb, cp, ed, tk) = (u("lookback"), g("close_pct"), u("entry_days"), g("tick"));
            for s in lb..n {
                let rg = range(x, s);
                if rg <= 0.0 || !(s + 1 - lb..=s).all(|k| range(x, k) <= rg) {
                    continue;
                }
                if long_only && x.lowest_low(s, lb) && x.close_loc(s) >= 1.0 - cp {
                    orders.push(ord(1.0, Fill::Stop(h[s] + tk), s, s + 1, s + ed, l[s]));
                } else if short_only && x.highest_high(s, lb) && x.close_loc(s) <= cp {
                    orders.push(ord(-1.0, Fill::Stop(l[s] - tk), s, s + 1, s + ed, h[s]));
                }
            }
            sim(x, orders, &kit(), from, to)
        }
        "btp2_1234_long" | "btp2_1234_short" => {
            let (pd, md, adx) = dmi3(x.r, u("adx_len"));
            let (am, tk, long) = (g("adx_min"), g("tick"), long_only);
            let inside = |b: usize| h[b] < h[b - 1] && l[b] > l[b - 1];
            let leg = |b: usize| if long { l[b] < l[b - 1] } else { h[b] > h[b - 1] };
            for i in 3..n {
                let trend = adx[i] > am && if long { pd[i] > md[i] } else { md[i] > pd[i] };
                if trend && (i - 2..=i).all(|b| leg(b) || inside(b)) && (i - 2..=i).filter(|&b| inside(b)).count() <= 1 {
                    if long {
                        orders.push(ord(1.0, Fill::Stop(h[i] + tk), i, i + 1, i + 1, l[i]));
                    } else {
                        orders.push(ord(-1.0, Fill::Stop(l[i] - tk), i, i + 1, i + 1, h[i]));
                    }
                }
            }
            sim(x, orders, &kit(), from, to)
        }
        "btp2_8day_hl_reversal_long" | "btp2_8day_hl_reversal_short" => {
            let (lb, cw, ew, tk, long) = (u("lookback"), u("cond3_window"), u("entry_window"), g("tick"), long_only);
            for j in 0..n - 2 {
                let d2 = j + 1;
                let ok = if long { x.lowest_low(j, lb) && h[d2] > h[j] } else { x.highest_high(j, lb) && l[d2] < l[j] };
                if !ok {
                    continue;
                }
                if let Some(k) = (d2 + 1..(d2 + 1 + cw).min(n)).find(|&k| if long { l[k] < l[d2] } else { h[k] > h[d2] }) {
                    if long {
                        orders.push(ord(1.0, Fill::Stop(h[d2] + tk), k, k, k + ew, l[d2] - tk));
                    } else {
                        orders.push(ord(-1.0, Fill::Stop(l[d2] - tk), k, k, k + ew, h[d2] + tk));
                    }
                }
            }
            sim(x, orders, &kit(), from, to)
        }
        // ---- intraday wave 1: real 1-minute path (default for the OOPS ids and the WREG sells); daily readings stay as path = 0
        "btp2_10pct_oops_long" | "btp2_10pct_oops_short" | "btp2_10pct_oops" | "btp2_wide_range_exhaustion_gap_short" if g("path") >= 0.5 => {
            use crate::intraday::{day_trade, stop_entry_day, Level};
            let Some(mm) = x.minutes else { return Some(vec![]) };
            let wreg = id.contains("wide_range");
            let hvv = if wreg { hv(c, u("hv_len")) } else { vec![] };
            let mut out = vec![];
            for j in from.max(2)..=to {
                let i = j - 1; // setup day
                let (dir, lv, sp) = if wreg {
                    let (k, br) = (g("sd_mult"), g("breath_sd"));
                    let sd = c[i - 1] * hvv[i - 1] / 100.0 / 252f64.sqrt();
                    if long_only && c[i] <= c[i - 1] - k * sd && o[j] < l[i] { (1.0, c[i], l[i].min(c[i] - br * sd)) }
                    else if short_only && c[i] >= c[i - 1] + k * sd && o[j] > h[i] { (-1.0, c[i], h[i].max(c[i] + br * sd)) } else { continue }
                } else {
                    let (cp, loc) = (g("close_pct"), x.close_loc(i));
                    if !short_only && loc <= cp && o[j] < l[i] { (1.0, l[i], f64::NAN) }
                    else if !long_only && loc >= 1.0 - cp && o[j] > h[i] { (-1.0, h[i], f64::NAN) } else { continue }
                };
                let Some(b) = mm.day(&x.r[j].date) else { continue };
                let st = |_: usize, _: f64| sp.is_finite().then(|| Level { up: dir < 0.0, px: sp });
                let Some((_, px, xs)) = stop_entry_day(&b, 0, &[Level { up: dir > 0.0, px: lv }], &st) else { continue };
                let loc = x.close_loc(j);
                let strong = wreg && if dir > 0.0 { loc >= 1.0 - g("carry_close_pct") } else { loc <= g("carry_close_pct") };
                out.push(match xs {
                    Some(xp) => day_trade(dir, j, px, j, xp, "protective stop", "stop", false),
                    None if strong && j < to => day_trade(dir, j, px, j + 1, o[j + 1], "closed at the extreme: carried, out on the next open", "stop", false),
                    None if strong => day_trade(dir, j, px, j, c[j], "open", "stop", true),
                    None => day_trade(dir, j, px, j, c[j], if wreg { "locked in near the close" } else { "MOC" }, "stop", false),
                });
            }
            out
        }
        // ---- end intraday wave 1
        "btp2_10pct_oops_long" | "btp2_10pct_oops_short" | "btp2_10pct_oops" => {
            let cp = g("close_pct");
            for i in 0..n - 1 {
                let loc = x.close_loc(i);
                if !short_only && loc <= cp && o[i + 1] < l[i] {
                    orders.push(ord(1.0, Fill::Stop(l[i]), i, i + 1, i + 1, f64::NAN));
                }
                if !long_only && loc >= 1.0 - cp && o[i + 1] > h[i] {
                    orders.push(ord(-1.0, Fill::Stop(h[i]), i, i + 1, i + 1, f64::NAN));
                }
            }
            sim(x, orders, &Ex { hold: Some(0), ..Default::default() }, from, to)
        }
        "btp2_momentum_ma_long" | "btp2_momentum_ma_short" => {
            let (f, s, cd, long) = (x.sma_n(u("fast_ma")), x.sma_n(u("slow_ma")), u("confirm_days"), long_only);
            let up = |i: usize| if long { f[i] > s[i] } else { f[i] < s[i] };
            let away = |i: usize| if long { c[i] < f[i] } else { c[i] > f[i] };
            let back = |i: usize| if long { c[i] >= f[i] } else { c[i] <= f[i] };
            let mut i = 1;
            while i < n {
                if !(up(i) && !up(i - 1) && !f[i - 1].is_nan() && !s[i - 1].is_nan()) {
                    i += 1;
                    continue;
                }
                // the regime runs from the cross bar while the fast SMA stays on the trend side
                let end = (i..n).find(|&k| !up(k)).unwrap_or(n);
                if let Some(t) = (i..end).find(|&k| away(k)) {
                    if let Some(k) = (t + 1..(t + 1 + cd).min(end)).find(|&k| back(k)) {
                        orders.push(ord(if long { 1.0 } else { -1.0 }, Fill::Open, k, k + 1, k + 1, if long { l[k] } else { h[k] }));
                    }
                }
                i = end.max(i + 1);
            }
            sim(x, orders, &kit(), from, to)
        }
        "btp2_gipsons_long" | "btp2_gipsons_short" => {
            let (pd, md, adx) = dmi3(x.r, u("adx_len"));
            // PTJ / ATS appendix HV: "the unbiased standard deviation of the N minus one most recent logarithmic daily returns"
            let k = (g("hv_n1") >= 0.5) as usize;
            let (hs, hl) = (hv(c, u("hv_short") - k), hv(c, u("hv_long") - k));
            let (am, rmax, tk, long) = (g("adx_min"), g("hv_ratio_max"), g("tick"), long_only);
            for s in 0..n {
                if adx[s] > am && hs[s] / hl[s] < rmax && if long { pd[s] > md[s] } else { md[s] > pd[s] } {
                    if long {
                        orders.push(ord(1.0, Fill::Stop(h[s] + tk), s, s + 1, s + 1, l[s]));
                    } else {
                        orders.push(ord(-1.0, Fill::Stop(l[s] - tk), s, s + 1, s + 1, h[s]));
                    }
                }
            }
            let tn = u("trail_n");
            let hvx: Rule = &|j, _| (hs[j] >= hl[j]).then(|| Act::HalfClose("half off: HV6 reached HV100".into()));
            let ex = match g("exit_mode").round() as i32 {
                2 => Ex { target_pct: Some(g("target_pct") / 100.0), ..Default::default() },
                3 => Ex { trail: Some(tn), ..Default::default() },
                _ => Ex { rule: Some(hvx), trail: Some(tn), trail_after_half: true, ..Default::default() },
            };
            sim(x, orders, &ex, from, to)
        }
        "btp2_wide_range_exhaustion_gap_long" | "btp2_wide_range_exhaustion_gap_short" => {
            let hvv = hv(c, u("hv_len"));
            let (k, br, carry, long) = (g("sd_mult"), g("breath_sd"), g("carry_close_pct"), long_only);
            for i in 1..n - 1 {
                // 1 SD = price × daily SD of log returns (HV100 measured before the move). `hv` annualises with √252,
                // so divide by √252: the book's box (annualised HV ÷ √256) is the same daily SD in its own units.
                let sd = c[i - 1] * hvv[i - 1] / 100.0 / 252f64.sqrt();
                if long && c[i] <= c[i - 1] - k * sd && o[i + 1] < l[i] {
                    orders.push(ord(1.0, Fill::Stop(c[i]), i, i + 1, i + 1, l[i].min(c[i] - br * sd)));
                } else if !long && c[i] >= c[i - 1] + k * sd && o[i + 1] > h[i] {
                    orders.push(ord(-1.0, Fill::Stop(c[i]), i, i + 1, i + 1, h[i].max(c[i] + br * sd)));
                }
            }
            let rule: Rule = &|j, p| {
                let loc = x.close_loc(j);
                let strong = if p.dir > 0.0 { loc >= 1.0 - carry } else { loc <= carry };
                Some(if strong { Act::NextOpen("closed at the extreme: carried, out on the next open".into()) } else { Act::Close("locked in near the close".into()) })
            };
            sim(x, orders, &Ex { rule: Some(rule), ..Default::default() }, from, to)
        }
        _ => return None,
    };
    Some(trades)
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "btp_cbp_short" => ("btp_cbp", "Crash, Burn, and Profit (CBP)", "Short"),
        "btp_dvmtm_short" => ("btp_dvmtm", "Double-Volume Market Top (DVMTM)", "Short"),
        "btp_bottom_reversal_long" => ("btp_bottom_reversal", "Bottom Reversals", "Long"),
        "btp_large_range_day_strangle" => ("btp_large_range_days", "Large-Range Days", "Naked combination"),
        "btp_large_range_day_study" => ("btp_large_range_days", "Large-Range Days", "Follow-through study"),
        "btp_momentum_gap" => ("btp_momentum_gap", "Momentum Gaps", "Long/Short"),
        "btp_triple_day_pullback_adx25" => ("btp_triple_day_pullback", "Triple-Day Pullbacks", "ADX > 25"),
        "btp_triple_day_pullback_adx30" => ("btp_triple_day_pullback", "Triple-Day Pullbacks", "ADX > 30"),
        "btp_turtle_thrust" => ("btp_turtle_thrust", "Turtle Thrusts", "Long/Short"),
        "btp2_smtp_long" => ("btp2_smtp", "Spent Market Trading Pattern (SMTP)", "Buys"),
        "btp2_smtp_short" => ("btp2_smtp", "Spent Market Trading Pattern (SMTP)", "Sells"),
        "btp2_1234_long" => ("btp2_1234", "1-2-3-4", "Buys"),
        "btp2_1234_short" => ("btp2_1234", "1-2-3-4", "Sells"),
        "btp2_8day_hl_reversal_long" => ("btp2_8day", "8-Day High/Low Reversal", "Buys"),
        "btp2_8day_hl_reversal_short" => ("btp2_8day", "8-Day High/Low Reversal", "Sells"),
        "btp2_10pct_oops_long" => ("btp2_oops", "10% Oops", "Buys"),
        "btp2_10pct_oops_short" => ("btp2_oops", "10% Oops", "Sells"),
        "btp2_10pct_oops" => ("btp2_oops", "10% Oops", "Buys + sells (1996 test)"),
        "btp2_momentum_ma_long" => ("btp2_momentum_ma", "Momentum Moving Averages", "Buys"),
        "btp2_momentum_ma_short" => ("btp2_momentum_ma", "Momentum Moving Averages", "Sells"),
        "btp2_gipsons_long" => ("btp2_gipsons", "Gipsons", "Long"),
        "btp2_gipsons_short" => ("btp2_gipsons", "Gipsons", "Short"),
        "btp2_wide_range_exhaustion_gap_long" => ("btp2_wreg", "Wide-Range Exhaustion Gap Reversals", "Buys"),
        "btp2_wide_range_exhaustion_gap_short" => ("btp2_wreg", "Wide-Range Exhaustion Gap Reversals", "Sells"),
        _ => return None,
    })
}

// ---------------------------------------------------------------- tests

#[cfg(test)]
mod tests {
    use super::*;

    fn rows(b: &[(f64, f64, f64, f64)]) -> Vec<Row> {
        b.iter()
            .enumerate()
            .map(|(i, &(o, h, l, c))| Row { ticker: "T".into(), date: format!("2020-01-{:02}", i + 1), open: o, high: h, low: l, close: c, volume: Some(1e6) })
            .collect()
    }

    #[test]
    fn oops_buys_day_one_low_sells_moc() {
        // day 0 closes in the bottom 10% (100..110, close 100.5); day 1 gaps to 98, trades up through 100, closes 103
        let r = rows(&[(105.0, 110.0, 100.0, 100.5), (98.0, 104.0, 97.0, 103.0), (103.0, 104.0, 102.0, 103.5)]);
        let x = Ctx::new(&r, &r, &[]);
        let t = run("btp2_10pct_oops_long", &x, 0, 2, &Params::from([("path".to_string(), 0.0)])).unwrap();
        assert_eq!(t.len(), 1);
        assert_eq!((t[0].legs[0].i, t[0].legs[0].px, t[0].exit_i, t[0].exit_px), (1, 100.0, 1, 103.0));
        // no gap below the day-one low -> no trade
        let r = rows(&[(105.0, 110.0, 100.0, 100.5), (100.2, 104.0, 99.0, 103.0), (103.0, 104.0, 102.0, 103.5)]);
        let x = Ctx::new(&r, &r, &[]);
        assert!(run("btp2_10pct_oops_long", &x, 0, 2, &Params::from([("path".to_string(), 0.0)])).unwrap().is_empty());
    }

    #[test]
    fn two_for_one_half_exit_then_gap_stop() {
        // buy stop 101, stop 99 (R = 2): bar 2 reaches 103 (half off), bar 3 gaps under the stop and fills at its 98 open
        let r = rows(&[(100.0, 100.0, 100.0, 100.0), (100.5, 101.5, 100.2, 101.2), (101.5, 103.5, 101.0, 103.0), (98.0, 98.5, 97.0, 97.5)]);
        let x = Ctx::new(&r, &r, &[]);
        let o = vec![Ord { dir: 1.0, fill: Fill::Stop(101.0), setup: 0, from: 1, last: 1, stop: 99.0 }];
        let t = sim(&x, o, &Ex { half_at_r: true, ..Default::default() }, 0, 3);
        assert_eq!(t.len(), 1);
        assert_eq!((t[0].legs[0].px, t[0].exit_i, t[0].exit_px), (101.0, 3, 100.5));
    }

    #[test]
    fn exempt_entry_bar_closing_beyond_stop_exits_at_stop() {
        // WREG-style: day 1 low 99, close 100; day 2 opens 97 (under the stop 99), fills the buy stop at 100
        let o = || vec![Ord { dir: 1.0, fill: Fill::Stop(100.0), setup: 0, from: 1, last: 1, stop: 99.0 }];
        let ex = Ex { hold: Some(1), ..Default::default() };
        // closes 98.5 below the stop: certainly crossed after the fill -> out at 99 on the entry bar
        let r = rows(&[(101.0, 102.0, 99.0, 100.0), (97.0, 100.5, 96.0, 98.5), (98.0, 99.0, 97.0, 98.0)]);
        let t = sim(&Ctx::new(&r, &r, &[]), o(), &ex, 0, 2);
        assert_eq!((t[0].legs[0].px, t[0].exit_i, t[0].exit_px), (100.0, 1, 99.0));
        // closes back inside (100.2): the low came before the fill, the trade is carried
        let r = rows(&[(101.0, 102.0, 99.0, 100.0), (97.0, 100.5, 96.0, 100.2), (100.0, 101.0, 99.5, 100.8)]);
        let t = sim(&Ctx::new(&r, &r, &[]), o(), &ex, 0, 2);
        assert_eq!((t[0].exit_i, t[0].exit_px), (2, 100.8));
    }

    #[test]
    fn weeks_start_monday() {
        assert_eq!(days("1970-01-01"), 0);
        assert_eq!((days("2024-01-08") + 3).div_euclid(7), (days("2024-01-12") + 3).div_euclid(7)); // Mon..Fri
        assert_ne!((days("2024-01-12") + 3).div_euclid(7), (days("2024-01-15") + 3).div_euclid(7));
    }
}
