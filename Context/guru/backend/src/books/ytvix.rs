//! YT Strategies · India VIX: the team's AmiBroker AFLs in "Old Topics/India VIX". They trade the chart symbol (spot
//! Nifty on daily bars) and read the India VIX only as a signal, through Foreign("VIX" / "INDIAVIX", "C"). Coded from
//! the AFLs as written, with their own default settings; there is no book PDF. No AFL in the folder trades options:
//! "WITH SHORT OPTION" means the option to go short on the same symbol after a losing long.
//!
//! Coded (6 AFL files, 3 distinct strategies):
//! - `ytvix_long`: INDIA VIX LONG TRADING.afl (= Final Folder copy, which differs only in Param ranges), INDIA VIX
//!   LONG TRADING_1.afl and INDIA VIX LONG TRADING+TRAIL SL WITH HIGHEST HIGH.afl. The three trade alike at their
//!   defaults (hard and trailing stops off); they differ only in the stop handling, so they are one id with the
//!   option `afl` (0 LONG TRADING, 1 LONG TRADING_1, 2 TRAIL SL WITH HIGHEST HIGH).
//! - `ytvix_short`: INDIA VIX SHORT TRADING VER2.afl.
//! - `ytvix_long_short`: INDIA VIX LONG TRADING WITH SHORT OPTION.afl ("LONG TRADING &SHORT LOSS MAKING" folder).
//!
//! AmiBroker behaviour followed:
//! - Foreign() aligns the VIX to the chart's dates and fills a missing VIX day with the previous value;
//! - the AFLs set no SetBarsRequired, so the loop runs over the bars AmiBroker loads: the test window plus the
//!   formula's look-back (the ROC period; and the ATR period for the long-short AFL) before it; loop state starts there;
//! - the first two AFLs' loops only keep state (count, bars in trade, stop levels); their Buy / Sell (Short / Cover)
//!   arrays are built after the loop and pass through ExRem; the long-short AFL writes the signals in its loop (no ExRem);
//! - AllowSameBarExit is on; the backtester exits first and then enters; fills are kept inside the bar's range;
//! - a comparison with a Null stop level is false (in the loops and in the arrays).
#![allow(dead_code)]
use crate::books::ytroc::atr_wilder;
use crate::books::ytsw::exrem;
use crate::books::ytwh::bound;
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "yt_vix";
pub const LONG: &str = "ytvix_long";
pub const SHORT: &str = "ytvix_short";
pub const LONG_SHORT: &str = "ytvix_long_short";
pub const IDS: [&str; 3] = [LONG, SHORT, LONG_SHORT];

fn def(id: &'static str, name: &'static str, dir: &'static str, rules: &'static [&'static str], notes: &'static str) -> StrategyDef {
    StrategyDef { id, name, group: "YT Strategies · India VIX", session: 0, dir, status: Status::Live, needs_volume: false, needs_vix: true, needs_series: &["VIX.Close"], book: BOOK, rules, notes, claim: None }
}

pub fn catalogue() -> Vec<StrategyDef> {
    vec![
        def(LONG, "India VIX Spike Long", "Long", &[
            "F|India VIX close up more than 10% from the previous day (ROC(VIX, 1) > 10)",
            "F|Option: today's index close up more than CLOSE PERCENTAGE DIFF % (0 = off, the default)",
            "E|Buy the index at the Close when not already in a trade (the loop's count was 0 yesterday)",
            "X|Sell at the Close on the 5th bar after the entry bar (EXIT DAYS)",
            "X|Hard SL % and trailing SL % (both 0 = off by default): Low below yesterday's level sells at the level, or at Open − 0.25% when the Open is below it",
            "X|Trailing SL: TRAILING SL PCT below the highest Close since entry (afl 2: the highest High after the entry bar)",
            "X|Buy and Sell pass through ExRem; AllowSameBarExit",
        ], "INDIA VIX LONG TRADING.afl (and its Final Folder copy, same logic, other Param ranges), INDIA VIX LONG TRADING_1.afl and INDIA VIX LONG TRADING+TRAIL SL WITH HIGHEST HIGH.afl, coded as the AFLs are written; option afl 0 / 1 / 2 picks the AFL. All three default to hard SL 0 and trailing SL 0, so at the defaults they are the same strategy: buy on a 10% VIX jump, sell at the close five bars later. afl 0 sells a stop exit with the older SellPrice formula, afl 1 and 2 with the longer one; afl 2 trails from the highest High after the entry bar and does not clear its stop levels after an exit (the stale hard SL can make the loop restart its count, as written). The loop gives up the trade one bar after the Sell (its count passes EXIT DAYS), so a new buy can come two bars after a time exit. The template's turnover, penny and dead-stock settings are off at their defaults.",
        ),
        def(SHORT, "India VIX Spike Short", "Short", &[
            "F|India VIX close up more than 10% from the previous day (ROC(VIX, 1) > 10)",
            "F|Index close down on the day (ROC(C, 1) < −CLOSE PERCENTAGE DIFF, 0 by default)",
            "E|Short the index at the Close (Short = ExRem(signal, Cover))",
            "X|Cover at the Close on the 5th bar after the loop's entry bar (EXIT DAYS)",
            "X|Hard SL % (above the short price) and trailing SL % (above the lower of today's and yesterday's Low), both 0 = off by default: High above yesterday's level covers at the level, or at Open + 0.25% when the Open is above it",
        ], "INDIA VIX SHORT TRADING VER2.afl, coded as the AFL is written. The Short array is the raw signal (not gated by the loop's count) passed through ExRem with Cover, so a short taken while the loop still counts an earlier trade is covered by that trade's 5th-bar or stop signal, as in AmiBroker. The trailing level uses Min(L, Ref(L, -1)), not the lowest Low since entry, as written. With the default stops off, the stop levels are Null; a comparison with Null is taken as false (otherwise the loop would end every trade on its entry bar and never cover).",
        ),
        def(LONG_SHORT, "India VIX Spike Long, Short After a Losing Long", "Long/Short", &[
            "F|India VIX close up more than 10% from the previous day; option: index close up more than CLOSE PERCENTAGE DIFF %",
            "E|Buy the index at the Close when flat",
            "X|ATR stop: Close ≤ yesterday's (highest Close since entry − 1 × ATR(14)): sell at the Close",
            "X|Hard SL % (0 = off): Low ≤ yesterday's level: sell at the level, or at Open − 0.25% when the Open is below it",
            "E|After a stop sell, when the long's return net of the 0.65% brokerage each way is below 1% (MIN PROFIT PCT FOR LONG), short at the sell price",
            "X|Otherwise sell on the 6th bar after entry (bar count > EXIT DAYS 5) or at the trailing SL (0 = off)",
            "X|Short: cover at the Close when Close ≥ yesterday's (lowest Close since the short + 1 × ATR(14)), or at the hard SL (99999999 when off) / Open + 0.25%",
        ], "INDIA VIX LONG TRADING WITH SHORT OPTION.afl (folder INDIA VIX LONG TRADING &SHORT LOSS MAKING), coded as the AFL is written: the short is on the same index, not an option. Quirks kept: the time-exit branch assigns the whole SellPrice array (SellPrice = IIf(.., C[i])), so every long sell recorded before the last time exit in the run fills at that bar's close (kept inside each exit bar's range by AmiBroker's price bound checking); only stop sells after it keep their own price. After a stop sell or a cover the loop leaves the hard-SL and ATR levels at 99999999 on that bar, so a buy on the next bar is sold on its entry bar (and shorted when the loss rule holds). Commission in the loss rule is the AFL's BROKERAGE 0.65%. The ATR is AmiBroker's Wilder ATR over the whole history.",
        ),
    ]
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    let mut v = vec![
        p("lookback", "Look back period for VIX % change", 1.0, 1.0, 50.0, 1.0),
        p("pct_change", "VIX % change", 10.0, 1.0, 50.0, 1.0),
        p("exit_days", "Exit days", 5.0, 0.0, 100.0, 1.0),
        p("cl_pct", "Close % diff (0 off)", 0.0, 0.0, 10.0, 0.1),
        p("trail_pct", "Trailing SL % (0 off)", 0.0, 0.0, 20.0, 0.5),
        p("hard_pct", "Hard stoploss % (0 off)", 0.0, 0.0, 50.0, 0.5),
    ];
    match id {
        LONG => v.push(p("afl", "AFL (0 LONG TRADING, 1 LONG TRADING_1, 2 TRAIL SL WITH HIGHEST HIGH)", 0.0, 0.0, 2.0, 1.0)),
        SHORT => {}
        LONG_SHORT => {
            v.push(p("atr_period", "ATR period for SL", 14.0, 1.0, 100.0, 1.0));
            v.push(p("atr_factor", "ATR factor for SL", 1.0, 1.0, 100.0, 1.0));
            v.push(p("min_profit_pct", "Min profit % for long (short below it)", 1.0, 0.0, 10.0, 0.25));
            v.push(p("brokerage_pct", "Brokerage % (loss rule)", 0.65, 0.0, 1.0, 0.01));
        }
        _ => return None,
    }
    Some(v)
}

fn le(a: f64, b: Option<f64>) -> bool {
    b.is_some_and(|b| a <= b)
}
fn lt(a: f64, b: Option<f64>) -> bool {
    b.is_some_and(|b| a < b)
}
fn ge(a: f64, b: Option<f64>) -> bool {
    b.is_some_and(|b| a >= b)
}
fn gt(a: f64, b: Option<f64>) -> bool {
    b.is_some_and(|b| a > b)
}
fn gto(a: Option<f64>, b: Option<f64>) -> bool {
    matches!((a, b), (Some(a), Some(b)) if a > b)
}
fn lto(a: Option<f64>, b: Option<f64>) -> bool {
    matches!((a, b), (Some(a), Some(b)) if a < b)
}

/// Foreign("VIX", "C") on this instrument's bars: a missing VIX day takes the previous value.
fn vix_close(x: &Ctx) -> Option<Vec<f64>> {
    let v = x.series("VIX.Close")?;
    let mut last = f64::NAN;
    Some(v.iter().map(|&a| { if a.is_finite() { last = a; } last }).collect())
}

/// ROC(v, n) on the loaded bars (from `start`): Null until n bars are loaded.
fn roc(v: &[f64], n: usize, start: usize, i: usize) -> Option<f64> {
    (i >= start + n && v[i].is_finite() && v[i - n].is_finite() && v[i - n] != 0.0).then(|| (v[i] - v[i - n]) / v[i - n] * 100.0)
}

/// Signal arrays as the backtester reads them.
struct Sigs {
    buy: Vec<bool>,
    sell: Vec<bool>,
    short: Vec<bool>,
    cover: Vec<bool>,
    sell_px: Vec<f64>,
    short_px: Vec<f64>,
    cover_px: Vec<f64>,
    sell_why: Vec<&'static str>,
    cover_why: Vec<&'static str>,
}

impl Sigs {
    fn new(x: &Ctx) -> Sigs {
        let n = x.c.len();
        Sigs { buy: vec![false; n], sell: vec![false; n], short: vec![false; n], cover: vec![false; n], sell_px: x.c.clone(), short_px: x.c.clone(), cover_px: x.c.clone(), sell_why: vec![""; n], cover_why: vec![""; n] }
    }
}

fn mk(dir: f64, e: usize, epx: f64, i: usize, px: f64, why: &str, open: bool) -> Trade {
    Trade { dir, legs: vec![Leg { i: e, px: epx, w: 1.0 }], exit_i: i, exit_px: px, reason: why.into(), entry_kind: "close", signal_i: i, open, exits: Vec::new() }
}

/// AmiBroker backtest of the arrays: entries only in from..=to; exit first, then entry; AllowSameBarExit lets a
/// position opened on a bar exit on that bar (once); after a long's sell a short on the same bar is entered.
fn backtest(x: &Ctx, s: &Sigs, from: usize, to: usize) -> Vec<Trade> {
    let mut out = Vec::new();
    let mut pos: Option<(f64, usize, f64)> = None;
    for i in from..=to {
        let mut exited = false;
        if let Some((d, e, epx)) = pos {
            if d > 0.0 && s.sell[i] {
                out.push(mk(d, e, epx, i, bound(x, i, s.sell_px[i]), s.sell_why[i], false));
                pos = None;
                exited = true;
            } else if d < 0.0 && s.cover[i] {
                out.push(mk(d, e, epx, i, bound(x, i, s.cover_px[i]), s.cover_why[i], false));
                pos = None;
                exited = true;
            }
        }
        if pos.is_none() && s.buy[i] {
            let epx = x.c[i];
            if s.sell[i] && !exited {
                out.push(mk(1.0, i, epx, i, bound(x, i, s.sell_px[i]), s.sell_why[i], false));
            } else {
                pos = Some((1.0, i, epx));
            }
        }
        if pos.is_none() && s.short[i] {
            let epx = bound(x, i, s.short_px[i]);
            if s.cover[i] && !exited {
                out.push(mk(-1.0, i, epx, i, bound(x, i, s.cover_px[i]), s.cover_why[i], false));
            } else {
                pos = Some((-1.0, i, epx));
            }
        }
    }
    if let Some((d, e, epx)) = pos {
        out.push(mk(d, e, epx, to, x.c[to], "open", true));
    }
    out
}

/// INDIA VIX LONG TRADING (afl 0), _1 (afl 1), TRAIL SL WITH HIGHEST HIGH (afl 2).
fn long(x: &Ctx, vix: &[f64], g: &dyn Fn(&str) -> f64, start: usize, to: usize) -> Sigs {
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let n = c.len();
    let afl = g("afl").round() as i64;
    let lb = g("lookback").round().max(1.0) as usize;
    let (pct, clpct, tp, hs) = (g("pct_change"), g("cl_pct"), g("trail_pct"), g("hard_pct"));
    let exit_days = g("exit_days").round() as i64;
    let day_exit = if exit_days == 0 { 1_000_000_000 } else { exit_days };
    let buysig = |i: usize| {
        let clroc = if clpct > 0.0 { roc(c, 1, start, i) } else { Some(100.0) };
        gto(roc(vix, lb, start, i), Some(pct)) && gto(clroc, Some(clpct))
    };
    let (mut hard, mut trail, mut sllow): (Vec<Option<f64>>, Vec<Option<f64>>, Vec<Option<f64>>) = (vec![None; n], vec![None; n], vec![None; n]);
    let (mut count, mut bcount) = (vec![0i64; n], vec![-1i64; n]);
    let hard_at = |i: usize| (hs > 0.0).then(|| c[i] - c[i] * hs / 100.0);
    let trail_at = |i: usize| (tp > 0.0).then(|| c[i] - c[i] * tp / 100.0);
    for i in start + 1..=to {
        if buysig(i) && count[i - 1] == 0 {
            count[i] = 1;
            hard[i] = hard_at(i);
            if afl == 2 {
                trail[i] = (tp > 0.0).then_some(0.0);
                sllow[i] = Some(0.0);
            } else {
                trail[i] = trail_at(i);
                sllow[i] = Some(c[i]);
            }
        } else {
            count[i] = count[i - 1];
            hard[i] = hard[i - 1];
            let px = if afl == 2 { h[i] } else { c[i] };
            let s = sllow[i - 1].map_or(px, |v| v.max(px));
            sllow[i] = Some(s);
            trail[i] = (tp > 0.0 && count[i] == 1).then(|| s - s * tp / 100.0);
        }
        bcount[i] = if count[i] == 1 { bcount[i - 1] + 1 } else { -1 };
        if bcount[i] > day_exit || le(l[i], hard[i - 1]) || le(l[i], trail[i - 1]) {
            if buysig(i) && count[i - 1] == 0 {
                count[i] = 1;
                hard[i] = hard_at(i);
                trail[i] = trail_at(i);
                sllow[i] = Some(c[i]);
                bcount[i] = 1;
            } else {
                count[i] = 0;
                bcount[i] = -1;
                if afl != 2 {
                    hard[i] = None;
                    trail[i] = None;
                }
            }
        }
    }
    let mut s = Sigs::new(x);
    let (mut buy, mut sell) = (vec![false; n], vec![false; n]);
    for i in start + 1..=to {
        let (hs1, ts1) = (hard[i - 1], trail[i - 1]);
        buy[i] = buysig(i) && count[i - 1] == 0;
        let (days, ts_hit, hs_hit) = (bcount[i] == exit_days, lt(l[i], ts1), lt(l[i], hs1));
        sell[i] = days || ts_hit || hs_hit;
        let gap = o[i] - o[i] * 0.0025;
        s.sell_px[i] = if afl == 0 {
            if ts_hit && lto(hs1, ts1) {
                if lt(o[i], ts1) { gap } else { ts1.unwrap() }
            } else if hs_hit {
                if lt(o[i], hs1) { gap } else { hs1.unwrap() }
            } else {
                c[i]
            }
        } else if lt(o[i], hs1) && ts_hit || hs_hit && lt(o[i], ts1) {
            gap
        } else if hs_hit && ts_hit && gto(hs1, ts1) {
            hs1.unwrap()
        } else if hs_hit && ts_hit && lto(hs1, ts1) {
            ts1.unwrap()
        } else if lt(o[i], ts1) {
            gap
        } else if ts_hit {
            ts1.unwrap()
        } else if hs_hit {
            hs1.unwrap()
        } else if lt(o[i], hs1) {
            gap
        } else {
            c[i]
        };
        s.sell_why[i] = if hs_hit { "Hard SL" } else if ts_hit { "Trailing SL" } else { "Exit days" };
    }
    s.buy = exrem(&buy, &sell);
    s.sell = exrem(&sell, &s.buy);
    s
}

/// INDIA VIX SHORT TRADING VER2.
fn short(x: &Ctx, vix: &[f64], g: &dyn Fn(&str) -> f64, start: usize, to: usize) -> Sigs {
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let n = c.len();
    let lb = g("lookback").round().max(1.0) as usize;
    let (pct, clpct, tp, hs) = (g("pct_change"), g("cl_pct"), g("trail_pct"), g("hard_pct"));
    let exit_days = g("exit_days").round() as i64;
    let sig = |i: usize| gto(roc(vix, lb, start, i), Some(pct)) && lto(roc(c, 1, start, i), Some(-clpct));
    let (mut hard, mut trail): (Vec<Option<f64>>, Vec<Option<f64>>) = (vec![None; n], vec![None; n]);
    let (mut count, mut bcount) = (vec![0i64; n], vec![-1i64; n]);
    for i in start + 1..=to {
        if sig(i) && count[i - 1] == 0 {
            count[i] = 1;
            hard[i] = (hs > 0.0).then(|| c[i] + c[i] * hs / 100.0);
            trail[i] = (tp > 0.0).then(|| l[i] + l[i] * tp / 100.0);
        } else {
            count[i] = count[i - 1];
            hard[i] = hard[i - 1];
            let s = l[i].min(l[i - 1]);
            trail[i] = (tp > 0.0 && count[i] == 1).then(|| s + s * tp / 100.0);
        }
        bcount[i] = if count[i] == 1 { bcount[i - 1] + 1 } else { -1 };
        if bcount[i] > exit_days || ge(h[i], hard[i - 1]) || ge(h[i], trail[i - 1]) {
            count[i] = 0;
            bcount[i] = -1;
            hard[i] = None;
            trail[i] = None;
        }
    }
    let mut s = Sigs::new(x);
    let (mut sh, mut cv) = (vec![false; n], vec![false; n]);
    for i in start + 1..=to {
        let (hs1, ts1) = (hard[i - 1], trail[i - 1]);
        sh[i] = sig(i);
        let (hs_hit, ts_hit) = (gt(h[i], hs1), gt(h[i], ts1));
        cv[i] = bcount[i] == exit_days || ts_hit || hs_hit;
        let gap = o[i] + o[i] * 0.25 / 100.0;
        s.cover_px[i] = if hs_hit {
            if gt(o[i], hs1) { gap } else { hs1.unwrap() }
        } else if ts_hit {
            if gt(o[i], ts1) { gap } else { ts1.unwrap() }
        } else {
            c[i]
        };
        s.cover_why[i] = if hs_hit { "Hard SL" } else if ts_hit { "Trailing SL" } else { "Exit days" };
    }
    s.short = exrem(&sh, &cv);
    s.cover = exrem(&cv, &s.short);
    s
}

const BIG: f64 = 99_999_999.0;

/// INDIA VIX LONG TRADING WITH SHORT OPTION: Buy / Sell / Short / Cover written in the loop.
fn long_short(x: &Ctx, vix: &[f64], g: &dyn Fn(&str) -> f64, start: usize, to: usize) -> Sigs {
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let n = c.len();
    let lb = g("lookback").round().max(1.0) as usize;
    let (pct, clpct, tp, hs) = (g("pct_change"), g("cl_pct"), g("trail_pct"), g("hard_pct"));
    let exit_days = g("exit_days").round() as i64;
    let day_exit = if exit_days == 0 { 1_000_000_000 } else { exit_days };
    let (af, min_profit, comm) = (g("atr_factor"), g("min_profit_pct"), g("brokerage_pct"));
    let atr = atr_wilder(x, g("atr_period").round().max(1.0) as usize);
    let buysig = |i: usize| {
        let clroc = if clpct > 0.0 { roc(c, 1, start, i) } else { Some(100.0) };
        gto(roc(vix, lb, start, i), Some(pct)) && gto(clroc, Some(clpct))
    };
    let (mut hard, mut trail, mut sllow) = (vec![0.0f64; n], vec![0.0f64; n], vec![0.0f64; n]);
    let mut shortsl: Vec<Option<f64>> = vec![Some(0.0); n];
    let mut atrsl: Vec<Option<f64>> = vec![None; n];
    let (mut count, mut bcount) = (vec![0i64; n], vec![-1i64; n]);
    let mut bp = c.clone();
    let mut s = Sigs::new(x);
    let mut position = 0i64;
    for i in start + 1..=to {
        if buysig(i) && count[i - 1] == 0 && position == 0 {
            count[i] = 1;
            hard[i] = if hs > 0.0 { c[i] - c[i] * hs / 100.0 } else { 0.0 };
            trail[i] = if tp > 0.0 { c[i] - c[i] * tp / 100.0 } else { 0.0 };
            atrsl[i] = atr[i].map(|a| c[i] - a * af);
            sllow[i] = c[i];
            s.buy[i] = true;
            bp[i] = c[i];
            position = 1;
        } else if position == 1 {
            count[i] = count[i - 1];
            hard[i] = hard[i - 1];
            sllow[i] = c[i].max(sllow[i - 1]);
            trail[i] = if tp > 0.0 && count[i] == 1 { sllow[i] - sllow[i] * tp / 100.0 } else { 0.0 };
            atrsl[i] = atr[i].map(|a| sllow[i] - a * af);
            bp[i] = bp[i - 1];
        }
        bcount[i] = if count[i] == 1 { bcount[i - 1] + 1 } else { -1 };
        if (l[i] <= hard[i - 1] || le(c[i], atrsl[i - 1])) && position == 1 {
            s.sell[i] = true;
            position = 0;
            let hs1 = hard[i - 1];
            if l[i] <= hs1 {
                s.sell_px[i] = if l[i] < hs1 && o[i] < hs1 { o[i] - o[i] * 0.0025 } else { hs1 };
                s.sell_why[i] = "Hard SL";
            } else {
                s.sell_px[i] = c[i];
                s.sell_why[i] = "ATR stop";
            }
            let sp = s.sell_px[i];
            let loss_pct = ((sp - sp * comm / 100.0) - (bp[i] + bp[i] * comm / 100.0)) / bp[i] * 100.0;
            if loss_pct < min_profit {
                s.short[i] = true;
                s.short_px[i] = sp;
                position = -1;
            }
            count[i] = 0;
            bcount[i] = -1;
            trail[i] = 0.0;
            sllow[i] = c[i];
            shortsl[i] = atr[i].map(|a| c[i] + a * af);
            hard[i] = if hs > 0.0 { c[i] + c[i] * hs / 100.0 } else { BIG };
        } else if (bcount[i] > day_exit || l[i] <= trail[i - 1]) && position == 1 {
            s.sell[i] = true;
            let ts1 = trail[i - 1];
            // SellPrice = IIf(..): the whole array takes this bar's value (scalar), or O x 0.9975 on every bar
            if l[i] <= ts1 && o[i] < ts1 {
                for j in start..=to {
                    s.sell_px[j] = o[j] - o[j] * 0.0025;
                }
            } else {
                let v = if l[i] <= ts1 { ts1 } else { c[i] };
                for j in start..=to {
                    s.sell_px[j] = v;
                }
            }
            s.sell_why[i] = if l[i] <= ts1 { "Trailing SL" } else { "Exit days" };
            position = 0;
            count[i] = 0;
            bcount[i] = -1;
            hard[i] = 0.0;
            trail[i] = 0.0;
            atrsl[i] = Some(0.0);
        } else if (h[i] >= hard[i - 1] || ge(c[i], shortsl[i - 1])) && position == -1 {
            s.cover[i] = true;
            let hs1 = hard[i - 1];
            if h[i] >= hs1 {
                s.cover_px[i] = if o[i] > hs1 { o[i] + o[i] * 0.0025 } else { hs1 };
                s.cover_why[i] = "Hard SL";
            } else {
                s.cover_px[i] = c[i];
                s.cover_why[i] = "ATR stop";
            }
            position = 0;
            hard[i] = BIG;
            trail[i] = BIG;
            atrsl[i] = Some(BIG);
            shortsl[i] = Some(BIG);
        } else if position == -1 {
            hard[i] = hard[i - 1];
            sllow[i] = c[i].min(sllow[i - 1]);
            shortsl[i] = atr[i].map(|a| sllow[i] + a * af);
        }
    }
    s
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(0.0, |d| d.def));
    let n = x.c.len();
    if n == 0 || from > to || to >= n {
        return Some(Vec::new());
    }
    let Some(vix) = vix_close(x) else { return Some(Vec::new()) };
    let lb = g("lookback").round().max(1.0) as usize;
    let lead = if id == LONG_SHORT { lb.max(g("atr_period").round().max(1.0) as usize) } else { lb };
    let start = from.saturating_sub(lead);
    let s = match id {
        LONG => long(x, &vix, &g, start, to),
        SHORT => short(x, &vix, &g, start, to),
        LONG_SHORT => long_short(x, &vix, &g, start, to),
        _ => return None,
    };
    Some(backtest(x, &s, from, to))
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let d = catalogue().into_iter().find(|d| d.id == id)?;
    Some((d.id, d.name, "India VIX"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::books::ytwh::testbars::*;
    use crate::Row;

    fn ctx<'a>(r: &'a [Row], vix: &[f64]) -> Ctx<'a> {
        let mut x = Ctx::new(r, r, &[]);
        x.extra.insert("VIX.Close".into(), vix.to_vec());
        x
    }
    fn go(id: &str, r: &[Row], vix: &[f64], kv: &[(&str, f64)]) -> Vec<Trade> {
        let x = ctx(r, vix);
        run(id, &x, 0, r.len() - 1, &ps(kv)).unwrap()
    }
    fn flat(n: usize) -> Vec<(f64, f64, f64, f64)> {
        (0..n).map(|k| { let c = 100.0 + (k % 3) as f64 * 0.1; (c, c + 1.0, c - 1.0, c) }).collect()
    }

    #[test]
    fn long_buys_on_vix_jump_and_sells_five_bars_later() {
        let r = bars(&flat(20));
        let mut vix = vec![15.0; 20];
        vix[5] = 17.0; // +13.3% -> buy at bar 5's close
        vix[6] = 19.0; // +11.8%: the loop is in a trade, no new buy
        let t = go(LONG, &r, &vix, &[]);
        assert_eq!(t.len(), 1);
        assert_eq!((t[0].legs[0].i, t[0].exit_i, t[0].exit_px), (5, 10, r[10].close));
        // a missing VIX day takes the previous value (no jump from NaN)
        let mut v2 = vix.clone();
        v2[4] = f64::NAN;
        assert_eq!(go(LONG, &r, &v2, &[])[0].legs[0].i, 5);
        // the three AFLs trade alike at the defaults
        for afl in [1.0, 2.0] {
            let u = go(LONG, &r, &vix, &[("afl", afl)]);
            assert_eq!((u[0].legs[0].i, u[0].exit_i), (5, 10));
        }
        // VIX up 9%: no signal
        let mut v3 = vec![10.0; 20];
        v3[5] = 10.9;
        assert!(go(LONG, &r, &v3, &[]).is_empty());
    }

    #[test]
    fn long_hard_stop_fills_at_level_or_gap_open() {
        let mut b = flat(20);
        b[7] = (99.0, 99.5, 97.0, 98.0); // low 97 < 100.2 x 0.98 = 98.196 -> sell at the level
        let r = bars(&b);
        let mut vix = vec![15.0; 20];
        vix[5] = 17.0;
        let entry = r[5].close;
        let t = go(LONG, &r, &vix, &[("hard_pct", 2.0)]);
        assert_eq!((t[0].legs[0].i, t[0].exit_i), (5, 7));
        assert!((t[0].exit_px - entry * 0.98).abs() < 1e-9);
        let mut b2 = flat(20);
        b2[7] = (97.5, 98.0, 96.0, 97.0); // opens below the level -> open less 0.25%
        let t = go(LONG, &bars(&b2), &vix, &[("hard_pct", 2.0)]);
        assert!((t[0].exit_px - 97.5 * 0.9975).abs() < 1e-9);
    }

    #[test]
    fn short_on_vix_jump_and_down_close() {
        let mut b = flat(20);
        b[5] = (100.0, 100.5, 98.5, 99.0); // down close on the VIX jump -> short at 99
        let r = bars(&b);
        let mut vix = vec![15.0; 20];
        vix[5] = 17.0;
        let t = go(SHORT, &r, &vix, &[]);
        assert_eq!(t.len(), 1);
        assert_eq!((t[0].dir, t[0].legs[0].i, t[0].legs[0].px, t[0].exit_i), (-1.0, 5, 99.0, 10));
        // no short on an up close
        let mut vix2 = vec![15.0; 20];
        vix2[4] = 17.0; // bar 4 closes 100.1 > 100.0
        assert!(go(SHORT, &r, &vix2, &[]).is_empty());
    }

    #[test]
    fn long_short_shorts_after_a_losing_atr_stop() {
        // 30 quiet bars (ATR(14) about 2), a VIX jump at bar 20, then a fall below the ATR stop
        let mut b = flat(30);
        b[22] = (99.0, 99.2, 96.5, 97.0); // close 97 <= highest close (100.2) - ATR (2.0) -> sell at the close, loss -> short at 97
        let r = bars(&b);
        let mut vix = vec![15.0; 30];
        vix[20] = 17.0;
        let t = go(LONG_SHORT, &r, &vix, &[]);
        assert!(t.len() >= 2);
        assert_eq!((t[0].dir, t[0].legs[0].i, t[0].exit_i, t[0].exit_px), (1.0, 20, 22, 97.0));
        assert_eq!((t[1].dir, t[1].legs[0].i, t[1].legs[0].px), (-1.0, 22, 97.0));
        // the short covers when the close rises to the lowest close since the short + ATR: the flat 100 bars do it at once
        assert_eq!(t[1].exit_i, 23);
    }

    #[test]
    fn long_short_time_exit_sets_the_whole_sellprice_array() {
        let mut b = flat(40);
        b[26] = (101.0, 101.5, 100.5, 101.0);
        let r = bars(&b);
        let mut vix = vec![15.0; 40];
        vix[5] = 17.0; // buy 5, time exit on bar 11 (bar count 6 > 5)
        vix[20] = 19.0; // buy 20, time exit on bar 26
        let t = go(LONG_SHORT, &r, &vix, &[]);
        assert_eq!(t.len(), 2);
        assert_eq!((t[0].legs[0].i, t[0].exit_i, t[1].legs[0].i, t[1].exit_i), (5, 11, 20, 26));
        // both sells fill at bar 26's close (the last SellPrice = C[i] assignment), kept in each bar's range
        assert_eq!(t[1].exit_px, 101.0);
        assert_eq!(t[0].exit_px, 101.0); // not bar 11's own close 100.2 (101 lies inside bar 11's range)
        assert!((r[11].close - 100.2).abs() < 1e-9);
    }

    #[test]
    fn no_vix_series_no_trades() {
        let r = wave(100);
        let x = Ctx::new(&r, &r, &[]);
        for id in IDS {
            assert!(run(id, &x, 0, 99, &Params::new()).unwrap().is_empty());
        }
    }

    #[test]
    fn every_strategy_runs() {
        let r = wave(400);
        let vix: Vec<f64> = (0..400).map(|k| 15.0 + 4.0 * ((k as f64) / 3.0).sin() + if k % 37 == 0 { 3.0 } else { 0.0 }).collect();
        let x = ctx(&r, &vix);
        for id in IDS {
            for kv in [vec![], vec![("hard_pct", 1.0), ("trail_pct", 1.0)], vec![("cl_pct", 0.5), ("exit_days", 0.0)], vec![("afl", 2.0), ("trail_pct", 2.0), ("hard_pct", 2.0)]] {
                for (from, to) in [(0, r.len() - 1), (50, 300)] {
                    let t = run(id, &x, from, to, &ps(&kv)).unwrap();
                    for t in &t {
                        assert!(t.legs[0].i >= from && t.legs[0].i <= to && t.exit_i >= t.legs[0].i && t.exit_i <= to, "{id}");
                        assert!(t.exit_px.is_finite() && t.exit_px >= x.l[t.exit_i] - 1e-9 && t.exit_px <= x.h[t.exit_i] + 1e-9, "{id}");
                    }
                }
            }
        }
    }
}
