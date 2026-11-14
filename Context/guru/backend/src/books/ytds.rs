//! YT Strategies · Date Sum (Odd / Even): the team's AmiBroker AFLs in "Old Topics/Miscelleneous/Buy when Date's sum
//! is Odd number" and "Old Topics/Miscelleneous/Short when Date's sum is Even number", run on one series at a time on
//! daily bars. Coded from the AFLs as written, with the AFLs' own default settings; there is no book PDF.
//!
//! The date sum is Day() + Month() + Year() of the bar (2024-01-02: 2 + 1 + 2024 = 2027, odd). The long AFL buys on a
//! bar whose sum is odd, the short AFL shorts on a bar whose sum is even, both at the close, with an ATR profit target
//! and ATR stop from the previous bar's ATR(5); the "(With & without filter)" AFLs add a Supertrend filter (both) and an
//! all-time-high-close filter (long only), chosen by a ParamList whose default is "Without Filter".
//!
//! One id per AFL pair: the "(Without filter)" AFLs are the same loop as the "(With & without filter)" AFLs set to
//! "Without Filter" (same defaults), so the filter is a parameter. The copies under "Short when Date's sum is Even
//! number/Odd-Even Date - Buy Sell/" differ from the top-level files only in Param / Optimize ranges, a "Checked By"
//! comment and chart text (md5 differs; defaults identical) and are not coded separately.
//!
//! AmiBroker behaviour followed:
//! - `SetBarsRequired(100000, 0)` loads the whole history, so the loop (and the all-time-high close HHV(C, BarCount))
//!   runs from the data's first bar; the backtester opens trades only in the test window;
//! - the loop is an if / else-if chain: no entry on an exit bar and no exit on the entry bar; the backtester then
//!   exits first and enters second on a bar; trade prices are kept inside the bar's low-high range;
//! - in the loop a Null element is -1e10 (`MyATR > 0` fails while ATR(5) does not exist yet);
//! - the AFLs' circuit rules are kept: no buy on a bar closing at its high (no short on one closing at its low); a
//!   target / stop exit on a lower-circuit bar (Prec(O,2) = Prec(C,2) = Prec(L,2); upper circuit for the short) is
//!   skipped and taken at the next non-circuit bar's open ∓ the 0.25% buffer; the same for a filter exit on a bar
//!   closing at its low (high);
//! - the ST filter is the team's Supertrend loop on the filter time frame (default monthly, period 5, factor 1,
//!   mid point; ATR = mean of the last 5 ATR(1)), expanded with TimeFrameExpand's default expandLast (a period's value
//!   reaches the daily bars at the period's last bar; the first period has none);
//! - the dead-stock toggle defaults to NO (no buy on bar 0, no forced sell) and is not offered; penny price 0 and the
//!   turnover filter (V x C against the AFL's yearly threshold, close x volume here) default off.
//!
//! Not coded: nothing else in these folders. "Candles Range Decrease and Volume Increase" (same Miscelleneous folder)
//! is `books::ytcr`.
#![allow(dead_code)]
use crate::books::ytbb::turnover;
use crate::books::ytroc::atr_wilder;
use crate::books::ytsw::{calc_point, close_swing, compress, supertrend, trade};
use crate::books::ytwh::{bound, hhv, loop_val, prec2};
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "yt_ds";
pub const BUY_ODD: &str = "ytds_buy_odd_date_sum";
pub const SHORT_EVEN: &str = "ytds_short_even_date_sum";
pub const IDS: [&str; 2] = [BUY_ODD, SHORT_EVEN];

const GROUP: &str = "YT Strategies · Date Sum (Odd / Even)";

fn def(id: &'static str, name: &'static str, dir: &'static str, rules: &'static [&'static str], notes: &'static str) -> StrategyDef {
    StrategyDef { id, name, group: GROUP, session: 0, dir, status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim: None }
}

pub fn catalogue() -> Vec<StrategyDef> {
    vec![
        def(BUY_ODD, "Buy when the Date's Sum is Odd", "Long", &[
            "F|Date sum = Day + Month + Year of the bar is odd",
            "F|Previous bar's ATR(5) exists (> 0)",
            "F|The bar does not close at its High",
            "F|Flat (not on an exit bar)",
            "F|Optional filter (default off): ST filter up (default monthly ST(5, 1), mid point)",
            "F|Or the all-time-high-close flag: set by a Close above the previous highest Close, cleared when the 10% Close swing turns down",
            "E|Buy at the Close",
            "X|Target: High ≥ Close + 2 × ATR: at the target, or at Open − 0.25% when the Open is above it",
            "X|Stop: Low ≤ Close − 1 × ATR: at the stop, or at Open − 0.25% when the Open is below it",
            "X|With a filter: sell at the Close when it turns off (ST down / flag cleared)",
            "X|A target / stop bar at the lower circuit (O = C = L) waits; sold at the next non-circuit bar's Open − 0.25%",
            "X|A filter exit on a bar closing at its Low waits the same way",
        ], "Buy when Date's sum is Odd number (With & without filter).afl (Miscelleneous/Buy when Date's sum is Odd number), coded as the AFL is written; its \"(Without filter).afl\" twin is the same loop with the filter off (the default here), and the copies in \"Short when Date's sum is Even number/Odd-Even Date - Buy Sell/Buy when Date's sum is Odd number/\" differ only in Param ranges. The AFL's Swing EXIT % default 10 lies below its own Param minimum 11; the default is used. ATR is AmiBroker's Wilder ATR(5) of the previous bar. A target sell when the Open gaps above the target fills at Open − 0.25% (below the Open, as written), kept inside the bar's range. The AFL is a stock template (penny / turnover filters, 20 positions) but trades one series at a time; portfolio limits are not modelled."),
        def(SHORT_EVEN, "Short when the Date's Sum is Even", "Short", &[
            "F|Date sum = Day + Month + Year of the bar is even",
            "F|Previous bar's ATR(5) exists (> 0)",
            "F|The bar does not close at its Low",
            "F|Flat (not on an exit bar)",
            "F|Optional filter (default off): ST filter down (default monthly ST(5, 1), mid point)",
            "E|Short at the Close",
            "X|Target: Low ≤ Close − 2 × ATR: at the target, or at Open + 0.25% when the Open is below it",
            "X|Stop: High ≥ Close + 1 × ATR: at the stop, or at Open + 0.25% when the Open is above it",
            "X|With the filter: cover at the Close when the ST turns up",
            "X|A target / stop bar at the upper circuit (O = C = H) waits; covered at the next non-circuit bar's Open + 0.25%",
            "X|A filter exit on a bar closing at its High waits the same way",
        ], "Short when Date's sum is Even number (With & without filter).afl (Miscelleneous/Short when Date's sum is Even number), coded as the AFL is written; its \"(Without filter).afl\" twin is the same loop with the filter off (the default here), and the copies in \"Odd-Even Date - Buy Sell/Short when Date's sum is Even number/\" differ only in Param / Optimize ranges and chart text. The short AFL has no all-time-high filter. ATR is AmiBroker's Wilder ATR(5) of the previous bar. A target cover when the Open gaps below the target fills at Open + 0.25% (as written), kept inside the bar's range. The AFL is a stock template (penny / turnover filters, 20 positions) but trades one series at a time; portfolio limits are not modelled."),
    ]
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    let mut v = vec![
        p("atr_period", "ATR period", 5.0, 1.0, 100.0, 1.0),
        p("pt_mult", "ATR profit target multiplier (0 off)", 2.0, 0.0, 20.0, 1.0),
        p("sl_mult", "ATR stop multiplier (0 off)", 1.0, 0.0, 20.0, 1.0),
        p("buffer_pct", "Buffer % (gap and circuit fills)", 0.25, 0.0, 5.0, 0.01),
    ];
    match id {
        BUY_ODD => v.insert(0, p("filter", "Filter (0 without, 1 ST filter, 2 all-time-high close filter)", 0.0, 0.0, 2.0, 1.0)),
        SHORT_EVEN => v.insert(0, p("filter", "Filter (0 without, 1 ST filter)", 0.0, 0.0, 1.0, 1.0)),
        _ => return None,
    }
    v.extend([
        p("st_tf", "ST filter time frame (0 monthly, 1 weekly, 2 daily)", 0.0, 0.0, 2.0, 1.0),
        p("st_period", "ST filter period", 5.0, 1.0, 20.0, 1.0),
        p("st_factor", "ST filter factor", 1.0, 0.0, 20.0, 1.0),
        p("st_point", "ST calculation point (0 mid point, 1 close, 2 pivot)", 0.0, 0.0, 2.0, 1.0),
    ]);
    if id == BUY_ODD {
        v.push(p("swing_pct", "All-time-high filter: Close swing % (AFL default 10, below its Param minimum)", 10.0, 1.0, 200.0, 1.0));
    }
    v.extend([
        p("penny", "Penny stock price (enter only above it)", 0.0, 0.0, 100.0, 1.0),
        p("turnover", "Turnover filter (0 off, 1 on: close x volume above the AFL's yearly threshold)", 0.0, 0.0, 1.0, 1.0),
    ]);
    Some(v)
}

// ---------------------------------------------------------------- shared with ytaj / ythh

/// The AFL loop's Short / Cover (or Buy / Sell) arrays with the loop's own prices (None = no signal).
pub(crate) struct LoopSig {
    pub dir: f64,
    pub entry: Vec<Option<f64>>,
    pub exit: Vec<Option<(f64, &'static str)>>,
}

impl LoopSig {
    pub(crate) fn new(dir: f64, n: usize) -> Self {
        LoopSig { dir, entry: vec![None; n], exit: vec![None; n] }
    }
}

/// AmiBroker's regular backtester on loop arrays (no ExRem): in from..=to an exit is taken before an entry on a bar;
/// an exit while flat and an entry while in a position are ignored; prices are kept inside the bar's low-high range.
/// A position still open at `to` is marked there at the close.
pub(crate) fn backtest(x: &Ctx, s: &LoopSig, from: usize, to: usize) -> Vec<Trade> {
    let mut out = Vec::new();
    let mut pos: Option<(usize, f64)> = None;
    for i in from..=to {
        if let (Some((e, epx)), Some((px, why))) = (pos, s.exit[i]) {
            out.push(trade(s.dir, e, epx, i, bound(x, i, px), why, false));
            pos = None;
        }
        if let (None, Some(px)) = (pos, s.entry[i]) {
            pos = Some((i, bound(x, i, px)));
        }
    }
    if let Some((e, epx)) = pos {
        out.push(trade(s.dir, e, epx, to, x.c[to], "open", true));
    }
    out
}

/// TimeFrameExpand(v, tf) with the default expandLast: a period's value reaches the daily bars at the period's last
/// bar and holds until the next period's last bar; the bars before the first period ends have none (0).
pub(crate) fn expand_last_null(v: &[i8], x: &Ctx, tf: char) -> Vec<i8> {
    let b = compress(x, tf);
    (0..b.group.len())
        .map(|i| {
            let g = b.group[i];
            if b.last[g] == i {
                v[g]
            } else if g == 0 {
                0
            } else {
                v[g - 1]
            }
        })
        .collect()
}

/// The team's Supertrend filter loop (MTREND) on the time frame `tf` ('M', 'W', 'D') with the calculation point
/// `point` (ytsw::calc_point: 1 mid, 2 close, 3 pivot), expanded to daily bars (1 up, -1 down, 0 none yet).
pub(crate) fn st_trend(x: &Ctx, tf: char, point: i64, period: usize, factor: f64) -> Vec<i8> {
    let b = compress(x, tf);
    let (tr, _) = supertrend(&calc_point(&b, point), &b.h, &b.l, &b.c, period, factor, factor);
    expand_last_null(&tr, x, tf)
}

/// Day() + Month() + Year() of a "YYYY-MM-DD" date.
pub(crate) fn date_sum(date: &str) -> i64 {
    date.split('-').filter_map(|s| s.parse::<i64>().ok()).sum()
}

// ---------------------------------------------------------------- the loop

fn date_loop(x: &Ctx, long: bool, g: &dyn Fn(&str) -> f64) -> LoopSig {
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let n = c.len();
    let gu = |k: &str| g(k).round().max(1.0) as usize;
    let gi = |k: &str| g(k).round() as i64;
    let atr = atr_wilder(x, gu("atr_period"));
    // MyATR = Ref(ATR(ATR_Prd), -1), as the loop reads it
    let my_atr: Vec<f64> = (0..n).map(|i| if i == 0 { -1e10 } else { loop_val(atr[i - 1]) }).collect();
    let (pt_m, sl_m, buf) = (g("pt_mult"), g("sl_mult"), g("buffer_pct") / 100.0);
    let filter = gi("filter");
    let mtrend = if filter == 1 {
        let tf = match gi("st_tf") {
            1 => 'W',
            2 => 'D',
            _ => 'M',
        };
        st_trend(x, tf, gi("st_point") + 1, g("st_period").round().max(1.0) as usize, g("st_factor"))
    } else {
        vec![0; n]
    };
    // all-time-high-close flag (long AFL): HHV(C, BarCount) is the highest close so far
    let athc: Vec<bool> = if long && filter == 2 {
        let hc = hhv(c, n);
        let t1 = close_swing(c, g("swing_pct"));
        let mut a = vec![false; n];
        for i in 1..n {
            a[i] = if c[i] > hc[i - 1] { true } else if t1[i] == -1 && a[i - 1] { false } else { a[i - 1] };
        }
        a
    } else {
        vec![false; n]
    };
    let turn = turnover(x, gi("turnover") == 1);
    let penny = g("penny");
    let circuit = |i: usize| prec2(o[i]) == prec2(c[i]) && prec2(o[i]) == prec2(if long { l[i] } else { h[i] });
    // UC (H = C) blocks a buy, LC (L = C) a short; a filter exit waits on LC (long) / UC (short)
    let at_extreme = |i: usize| if long { h[i] == c[i] } else { l[i] == c[i] };
    let exit_wait = |i: usize| if long { l[i] == c[i] } else { h[i] == c[i] };
    let parity_ok = |i: usize| (date_sum(&x.r[i].date) % 2 != 0) == long;
    let filter_ok = |i: usize| match filter {
        1 => mtrend[i] == if long { 1 } else { -1 },
        2 => athc[i],
        _ => true,
    };
    let filter_off = |i: usize| match filter {
        1 => mtrend[i] == if long { -1 } else { 1 },
        2 => !athc[i],
        _ => false,
    };
    let d = if long { 1.0 } else { -1.0 };
    const BIG: f64 = 9_999_999.0;
    // long: target starts at 9999999 and the stop at 0; short: target 0 and stop 9999999
    let (pt0, sl0) = if long { (BIG, 0.0) } else { (0.0, BIG) };
    let mut s = LoopSig::new(d, n);
    let (mut position, mut skip, mut pt, mut sl) = (false, false, pt0, sl0);
    for i in 1..n {
        if !position && my_atr[i] > 0.0 && parity_ok(i) && !at_extreme(i) && c[i] > penny && turn[i] && filter_ok(i) {
            s.entry[i] = Some(c[i]);
            if pt_m > 0.0 {
                pt = c[i] + d * my_atr[i] * pt_m;
            }
            if sl_m > 0.0 {
                sl = c[i] - d * my_atr[i] * sl_m;
            }
            position = true;
        } else if position && !skip {
            let hit_pt = if long { h[i] >= pt } else { l[i] <= pt };
            let hit_sl = if long { l[i] <= sl } else { h[i] >= sl };
            let mut out: Option<(f64, &'static str)> = None;
            if hit_pt {
                let gap = if long { o[i] > pt } else { o[i] < pt };
                if circuit(i) {
                    skip = true;
                } else {
                    out = Some((if gap { o[i] - d * o[i] * buf } else { pt }, if gap { "ATR target (gap)" } else { "ATR target" }));
                }
            } else if hit_sl {
                let gap = if long { o[i] < sl } else { o[i] > sl };
                if circuit(i) {
                    skip = true;
                } else {
                    out = Some((if gap { o[i] - d * o[i] * buf } else { sl }, if gap { "ATR stop (gap)" } else { "ATR stop" }));
                }
            } else if filter_off(i) {
                if !exit_wait(i) {
                    out = Some((c[i], if filter == 1 { "ST filter turned" } else { "All-time-high flag cleared" }));
                } else {
                    skip = true;
                }
            }
            if let Some(e) = out {
                s.exit[i] = Some(e);
                position = false;
                pt = pt0;
                sl = sl0;
            }
        } else if skip && !circuit(i) {
            s.exit[i] = Some((o[i] - d * o[i] * buf, "Circuit bar skipped: next bar's Open"));
            position = false;
            skip = false;
            pt = pt0;
            sl = sl0;
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
    let s = date_loop(x, id == BUY_ODD, &g);
    Some(backtest(x, &s, from, to))
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let d = catalogue().into_iter().find(|d| d.id == id)?;
    Some((d.id, d.name, "Date Sum (Odd / Even)"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::books::ytwh::testbars::{bars, ps, smoke, wave};

    fn flat(c: f64) -> (f64, f64, f64, f64) {
        (c, c + 1.0, c - 1.0, c - 0.2)
    }

    #[test]
    fn date_sum_parity() {
        assert_eq!(date_sum("2024-01-01"), 2026);
        assert_eq!(date_sum("2024-01-02"), 2027);
    }

    #[test]
    fn buys_on_odd_sum_and_exits_at_atr_target() {
        // bars from Mon 2024-01-01 (sum 2026, even), 01-02 odd, ... 01-09 (bar 6) even, 01-10 (bar 7) odd;
        // TR = 2 on every bar, ATR(5) exists from bar 5, so MyATR (previous bar) from bar 6
        let mut v: Vec<(f64, f64, f64, f64)> = (0..12).map(|_| flat(100.0)).collect();
        v[8] = (100.0, 104.0, 99.0, 103.5);
        let r = bars(&v);
        let x = Ctx::new(&r, &r, &[]);
        let t = run(BUY_ODD, &x, 0, r.len() - 1, &Params::new()).unwrap();
        // bar 7: buy at 99.8, target 99.8 + 2 x 2 = 103.8, reached on bar 8
        assert_eq!(t[0].legs[0].i, 7);
        assert!((t[0].legs[0].px - 99.8).abs() < 1e-9);
        assert_eq!(t[0].exit_i, 8);
        assert!((t[0].exit_px - 103.8).abs() < 1e-9);
        // no entry on the exit bar (bar 8 is even anyway); the next odd bar re-enters
        assert_eq!(t[1].legs[0].i, 9);
    }

    #[test]
    fn short_on_even_sum_and_stop_at_atr() {
        let mut v: Vec<(f64, f64, f64, f64)> = (0..12).map(|_| (100.0, 101.0, 99.0, 100.2)).collect();
        v[7] = (100.0, 103.0, 99.5, 100.5);
        let r = bars(&v);
        let x = Ctx::new(&r, &r, &[]);
        let t = run(SHORT_EVEN, &x, 0, r.len() - 1, &Params::new()).unwrap();
        // bar 6 (2024-01-09, sum 2034): short at 100.2, stop 102.2, target 96.2; bar 7 reaches the stop
        assert_eq!((t[0].dir, t[0].legs[0].i, t[0].exit_i), (-1.0, 6, 7));
        assert!((t[0].exit_px - 102.2).abs() < 1e-9);
        assert_eq!(t[1].legs[0].i, 8);
    }

    #[test]
    fn lower_circuit_bar_defers_the_sell_to_next_open() {
        let mut v: Vec<(f64, f64, f64, f64)> = (0..12).map(|_| flat(100.0)).collect();
        // buy bar 7 at 99.8, stop 97.8; bar 8 is a lower-circuit bar (O = C = L) below the stop
        v[8] = (97.0, 97.0, 97.0, 97.0);
        v[9] = (98.0, 99.0, 97.5, 98.5);
        let r = bars(&v);
        let x = Ctx::new(&r, &r, &[]);
        let t = run(BUY_ODD, &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert_eq!((t[0].legs[0].i, t[0].exit_i), (7, 9));
        assert!((t[0].exit_px - 98.0 * (1.0 - 0.0025)).abs() < 1e-9);
    }

    #[test]
    fn st_filter_exit_at_close() {
        // daily ST filter: a steady rise keeps it up; a collapse turns it down and sells at the close
        let mut v: Vec<(f64, f64, f64, f64)> = (0..30).map(|k| { let c = 100.0 + k as f64 * 0.1; (c - 0.05, c + 1.0, c - 1.0, c) }).collect();
        v[20] = (101.0, 101.5, 90.0, 90.5);
        let r = bars(&v);
        let x = Ctx::new(&r, &r, &[]);
        let t = run(BUY_ODD, &x, 0, r.len() - 1, &ps(&[("filter", 1.0), ("st_tf", 2.0), ("pt_mult", 0.0), ("sl_mult", 0.0)])).unwrap();
        let k = t.iter().find(|t| t.exit_i == 20).expect("ST exit on bar 20");
        assert_eq!(k.reason, "ST filter turned");
        assert!((k.exit_px - 90.5).abs() < 1e-9);
    }

    #[test]
    fn every_strategy_runs() {
        smoke(run, &IDS);
        let r = wave(420);
        let x = Ctx::new(&r, &r, &[]);
        for id in IDS {
            assert!(params(id).is_some() && family_of(id).is_some(), "{id}");
            for kv in [vec![("filter", 1.0)], vec![("filter", 2.0)], vec![("filter", 1.0), ("st_tf", 1.0)], vec![("pt_mult", 0.0), ("sl_mult", 0.0), ("filter", 1.0), ("st_tf", 2.0)], vec![("turnover", 1.0)]] {
                for (from, to) in [(0, 419), (50, 300)] {
                    let t = run(id, &x, from, to, &ps(&kv)).unwrap();
                    for t in &t {
                        assert!(t.legs[0].i >= from && t.exit_i >= t.legs[0].i && t.exit_i <= to, "{id}");
                        assert!(t.exit_px >= x.l[t.exit_i] - 1e-9 && t.exit_px <= x.h[t.exit_i] + 1e-9, "{id}");
                    }
                    assert!(t.windows(2).all(|w| w[1].legs[0].i >= w[0].exit_i), "{id}");
                }
            }
        }
        assert_eq!(catalogue().len(), IDS.len());
    }
}
