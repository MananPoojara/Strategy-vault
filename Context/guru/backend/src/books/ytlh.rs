//! YT Strategies · Lower High and Lower Low: the team's AmiBroker AFLs in "Old Topics/Lower High and Lower Low/AFLs",
//! daily long-only index strategies that buy after N consecutive lower lows and/or lower highs:
//! - "LowerHigh & LowerLow.afl": the plain pattern with an optional RSI threshold and a candle-count exit;
//! - "Combinations with 2_11.afl": the pattern taken only inside a month-turn window (entry from the n-th bar of the
//!   month, exit n bars later), with optional hard stop, profit booking and trailing stop;
//! - "Combinations with 5DP.afl": the same inside the "5 DP" window (last n bars of a month to the n-th bar of the
//!   next; 31-day and 30-day months have their own n).
//! Coded from the AFLs as written, with their own default settings; there is no book PDF.
//!
//! AmiBroker behaviour the code follows (see `ytsw.rs` for the shared conventions):
//! - `Sum(cond, n) >= n` is "the last n bars all meet cond" and is false before n bars exist; `L < Ref(L, -1)` is
//!   false on bar 0;
//! - RSI is AmiBroker's (Wilder, seeded with the simple average of the first n changes, Null before bar n); a
//!   comparison with a Null RSI is false;
//! - `EOM = Month() != Ref(Month(), 1)` is the last bar of a calendar month, and true on the last bar of the data
//!   (the comparison with the Null next month is true);
//! - the AFLs set `SetBarsRequired(100000, 0)`, so the range end is the last bar the AFL sees (no look past it);
//! - each AFL's own bar loop sets Buy / Sell at the close (stop fills as the AFL prices them); a position still open
//!   at the end is closed on the last bar.
#![allow(dead_code)]
use crate::books::ytwh::{bound, collect, Ev};
use crate::books::yteng::rsi;
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "yt_lh";
pub const BASE: &str = "ytlh_lower_high_lower_low";
pub const TWO_11: &str = "ytlh_2_11_lower_high_lower_low";
pub const FIVE_DP: &str = "ytlh_5dp_lower_high_lower_low";
pub const IDS: [&str; 3] = [BASE, TWO_11, FIVE_DP];

fn def(id: &'static str, name: &'static str, rules: &'static [&'static str], notes: &'static str) -> StrategyDef {
    StrategyDef { id, name, group: "YT Strategies · Lower High and Lower Low", session: 0, dir: "Long", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim: None }
}

pub fn catalogue() -> Vec<StrategyDef> {
    vec![
        def(BASE, "N Lower Lows / Lower Highs, Candle Exit", &[
            "F|Entry type (default LL): LL = the last N bars (default 4) each have Low < previous Low",
            "F|LH = the last N bars (default 4) each have High < previous High; LL & LH = both",
            "F|Lower RSI = the last N bars (default 4) each have RSI(14) < previous RSI",
            "F|Optional RSI(14) threshold (off by default): RSI ≥ threshold (Above, default 60) or RSI ≤ threshold (Below)",
            "E|Buy at the Close when flat (never on the last bar)",
            "X|Count bars after entry: every bar (Num of Candles, default), Up Close bars (Close ≥ previous Close) or green bars (Close ≥ Open)",
            "X|When the count reaches N (default 5): if the entry signal is true on that bar, reset the count and stay in",
            "X|Otherwise sell at the Close",
            "X|Sell on the last bar of the data",
        ], "LowerHigh & LowerLow.afl, coded as the AFL is written. The exit-count Param has default 5 with a 1-3 range in the AFL; 5 is used. Position size and brokerage are backtest settings."),
        def(TWO_11, "2-11 Window with N Lower Lows / Lower Highs", &[
            "F|Window: starts on the bar Entry Day bars (default 1) after a month's last bar",
            "F|Window lasts Exit Day bars (default 1); the exit signal is on the next bar",
            "F|Pattern (default Lower Highs & Lower Lows): the last N bars (default 2) each have a lower Low and/or a lower High",
            "F|Optional RSI(14) threshold (off by default): Above = RSI ≥ threshold, Below = RSI ≤ threshold",
            "F|Not on the last bar of the data",
            "E|Buy at the Close when flat inside the window",
            "X|Sell at the Close on the window's exit bar or the last bar of the data",
            "X|Optional hard stop % below entry (Close, or Low with Live exit), profit booking % above entry (Close, or High with Live exit)",
            "X|Optional trailing stop % below the highest High (checked against the previous bar's level)",
            "X|Live exits fill at the stop level, or at the Open (less the exit buffer) when the Open is beyond it",
        ], "Combinations with 2_11.afl, coded as the AFL is written (Entry Day and Exit Day defaults 1 / 1). The trailing stop exit fills at the Close even with Live exits, as in the AFL."),
        def(FIVE_DP, "5 DP Window with N Lower Lows / Lower Highs", &[
            "F|Window opens on the n-th last bar of a month (default 5; separate n for 31-day and 30-day months)",
            "F|Window closes on the n-th bar of the next month (default 2; the n of the month just ended); exit signal on that bar",
            "F|Pattern (default Lower Highs & Lower Lows): the last N bars (default 2) each have a lower Low and/or a lower High",
            "F|Optional RSI(14) threshold (off by default)",
            "E|Buy at the Close when flat inside the window",
            "X|Sell at the Close on the window's exit bar or the last bar of the data",
            "X|Optional hard stop, profit booking and trailing stop as in the 2-11 AFL",
        ], "Combinations with 5DP.afl, coded as the AFL is written. A month with 31 calendar days uses the 31-day settings for its entry bar; the exit uses the settings of the month that just ended."),
    ]
}

fn common(v: &mut Vec<ParamDef>) {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    v.extend([
        p("entry_type", "Entry condition (1 Lower Highs, 2 Lower Lows, 3 both)", 3.0, 1.0, 3.0, 1.0),
        p("n", "N lower highs / lower lows (0 to disable)", 2.0, 0.0, 100.0, 1.0),
        p("rsi_on", "Check RSI threshold (0 off, 1 on)", 0.0, 0.0, 1.0, 1.0),
        p("rsi_period", "RSI period", 14.0, 1.0, 100.0, 1.0),
        p("rsi_threshold", "RSI threshold", 60.0, 1.0, 100.0, 1.0),
        p("rsi_below", "RSI threshold side (0 above, 1 below)", 0.0, 0.0, 1.0, 1.0),
        p("exit_live", "Exit at (0 close, 1 live)", 0.0, 0.0, 1.0, 1.0),
        p("hsl", "HSL %", 0.0, 0.0, 100.0, 0.1),
        p("pb", "PB %", 0.0, 0.0, 100.0, 0.1),
        p("tsl", "TSL %", 0.0, 0.0, 100.0, 0.1),
        p("exit_buffer", "Exit buffer %", 0.0, 0.0, 1.0, 0.05),
    ]);
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    Some(match id {
        BASE => vec![
            p("entry_type", "Entry criteria (1 LL, 2 LH, 3 LL & LH, 4 Lower RSI)", 1.0, 1.0, 4.0, 1.0),
            p("ll_n", "Number of LL consecutive candles", 4.0, 1.0, 20.0, 1.0),
            p("lh_n", "Number of LH consecutive candles", 4.0, 1.0, 20.0, 1.0),
            p("lrsi_n", "Number of consecutive lower RSI", 4.0, 1.0, 100.0, 1.0),
            p("lrsi_period", "(Consecutive lower) RSI period", 14.0, 1.0, 100.0, 1.0),
            p("rsi_on", "Check RSI threshold (0 off, 1 on)", 0.0, 0.0, 1.0, 1.0),
            p("rsi_period", "RSI period", 14.0, 1.0, 100.0, 1.0),
            p("rsi_threshold", "RSI threshold", 60.0, 1.0, 100.0, 1.0),
            p("rsi_below", "RSI threshold side (0 above, 1 below)", 0.0, 0.0, 1.0, 1.0),
            p("exit_type", "Exit criteria (1 Num of candles, 2 Up close, 3 Num of green candles)", 1.0, 1.0, 3.0, 1.0),
            p("exit_n", "Number of candles for the exit", 5.0, 1.0, 20.0, 1.0),
        ],
        TWO_11 => {
            let mut v = vec![p("entry_day", "Entry day (bars after the month's last bar)", 1.0, 1.0, 100.0, 1.0), p("exit_day", "Exit day (bars in the window)", 1.0, 1.0, 100.0, 1.0)];
            common(&mut v);
            v
        }
        FIVE_DP => {
            let mut v = vec![
                p("entry31", "Entry at last n days of the month (31 day)", 5.0, 1.0, 8.0, 1.0),
                p("entry30", "Entry at last n days of the month (30 day)", 5.0, 1.0, 8.0, 1.0),
                p("exit31", "Exit at n days of the start of the month (31 day)", 2.0, 1.0, 8.0, 1.0),
                p("exit30", "Exit at n days of the start of the month (30 day)", 2.0, 1.0, 8.0, 1.0),
            ];
            common(&mut v);
            v
        }
        _ => return None,
    })
}

/// `Sum(cond, n) >= n`: the last n bars (this one included) all meet cond; false before n bars exist. n = 0 is true.
fn last_n(cond: &[bool], n: usize) -> Vec<bool> {
    let mut run = 0usize;
    cond.iter()
        .enumerate()
        .map(|(i, &c)| {
            run = if c { run + 1 } else { 0 };
            n == 0 || (i + 1 >= n && run >= n)
        })
        .collect()
}

fn month(x: &Ctx, i: usize) -> u32 {
    x.r[i].date.get(5..7).and_then(|s| s.parse().ok()).unwrap_or(0)
}

/// AFL `Month() != Ref(Month(), 1)`. On the last bar the AFL sees, `Ref(Month(), 1)` is Null and the comparison is
/// true (the team's 5 DP trade list to 31-07-2024 enters on 25-07-2024 from that bar).
fn eom(x: &Ctx, n: usize) -> Vec<bool> {
    (0..n).map(|i| i + 1 >= n || month(x, i) != month(x, i + 1)).collect()
}

/// RSI threshold filter: `IIf(on, IIf(below, RSI <= t, RSI >= t), 1)`, false where the RSI is Null.
fn rsi_filter(x: &Ctx, on: bool, period: usize, t: f64, below: bool) -> Vec<bool> {
    if !on {
        return vec![true; x.c.len()];
    }
    rsi(&x.c, period).into_iter().map(|v| v.is_some_and(|r| if below { r <= t } else { r >= t })).collect()
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(0.0, |d| d.def));
    let gu = |k: &str| g(k).round().max(0.0) as usize;
    let n = x.c.len();
    if n == 0 || from > to || to >= n {
        return Some(Vec::new());
    }
    let rf = rsi_filter(x, g("rsi_on") > 0.5, gu("rsi_period"), g("rsi_threshold"), g("rsi_below") > 0.5);
    // SetBarsRequired(100000, 0): AmiBroker loads no bars after the range end, so the AFL's last bar (its dead-stock
    // exit, `EOM` and the forward `Ref(EOM, n - 1)`) is the bar at `to`
    let evs = if id == BASE { base(x, &g, &rf, to + 1) } else { window(id, x, &g, &rf, to + 1) };
    Some(collect(x, evs, from, to))
}

/// LowerHigh & LowerLow.afl.
fn base(x: &Ctx, g: &dyn Fn(&str) -> f64, rf: &[bool], n: usize) -> Vec<Ev> {
    let gu = |k: &str| g(k).round().max(1.0) as usize;
    let ll: Vec<bool> = (0..n).map(|i| i > 0 && x.l[i] < x.l[i - 1]).collect();
    let lh: Vec<bool> = (0..n).map(|i| i > 0 && x.h[i] < x.h[i - 1]).collect();
    let r = rsi(&x.c, gu("lrsi_period"));
    let lr: Vec<bool> = (0..n).map(|i| i > 0 && matches!((r[i], r[i - 1]), (Some(a), Some(b)) if a < b)).collect();
    let (ll_ok, lh_ok, lr_ok) = (last_n(&ll, gu("ll_n")), last_n(&lh, gu("lh_n")), last_n(&lr, gu("lrsi_n")));
    let et = g("entry_type").round() as i64;
    let sig: Vec<bool> = (0..n)
        .map(|i| (!(et == 1 || et == 3) || ll_ok[i]) && (!(et == 2 || et == 3) || lh_ok[i]) && (et != 4 || lr_ok[i]) && rf[i])
        .collect();
    let xt = g("exit_type").round() as i64;
    let xn = gu("exit_n");
    let mut evs: Vec<Ev> = Vec::new();
    let (mut pos, mut cnt) = (false, 0usize);
    for i in 1..n {
        if pos {
            let up = x.c[i] >= x.c[i - 1];
            let green = x.c[i] >= x.o[i];
            if (xt == 2 && up) || (xt == 3 && green) || xt == 1 {
                cnt += 1;
            }
            if i == n - 1 || cnt == xn {
                if sig[i] {
                    cnt = 0;
                } else {
                    if let Some(v) = evs.last_mut() {
                        v.exit = Some((i, x.c[i], i, if i == n - 1 { "Last bar" } else { "Candle count exit" }));
                    }
                    pos = false;
                }
            }
        }
        if !pos && sig[i] && i != n - 1 {
            evs.push(Ev { dir: 1.0, e: i, epx: x.c[i], exit: None });
            pos = true;
            cnt = 0;
        }
    }
    evs
}

/// The month-turn window (phenomena filter) and its exit signal for the 2-11 and 5 DP AFLs.
fn phenomena(id: &str, x: &Ctx, g: &dyn Fn(&str) -> f64, n: usize) -> (Vec<bool>, Vec<bool>) {
    let e = eom(x, n);
    let gi = |k: &str| g(k).round() as i64;
    let (mut ph, mut ex) = (vec![false; n], vec![false; n]);
    if id == TWO_11 {
        let (ed, xd) = (gi("entry_day").max(0) as usize, gi("exit_day"));
        let mut flag = false;
        let mut left = xd;
        for i in 0..n {
            if i >= ed && e[i - ed] {
                flag = true;
                left = xd;
            }
            if flag && left == 0 {
                ex[i] = true;
                flag = false;
            }
            if flag {
                left -= 1;
                ph[i] = true;
            }
        }
    } else {
        let long_month = |i: usize| matches!(month(x, i), 1 | 3 | 5 | 7 | 8 | 10 | 12);
        // Ref(EOM, k - 1): the bar k - 1 bars before a month's last bar (false where that bar is past the data)
        let fwd = |i: usize, k: i64| {
            let j = i as i64 + k - 1;
            j >= 0 && (j as usize) < n && e[j as usize]
        };
        let (e31, e30, x31, x30) = (gi("entry31"), gi("entry30"), gi("exit31"), gi("exit30"));
        let entry: Vec<bool> = (0..n).map(|i| if long_month(i) { fwd(i, e31) } else { fwd(i, e30) }).collect();
        let (mut counter, mut flag31) = (0i64, false);
        for i in 1..n {
            counter += 1;
            ph[i] = ph[i - 1];
            if entry[i] {
                ph[i] = true;
            }
            if e[i] {
                counter = 0;
                flag31 = long_month(i);
            }
            if counter == if flag31 { x31 } else { x30 } {
                ph[i] = false;
                ex[i] = true;
            }
        }
    }
    (ph, ex)
}

/// Combinations with 2_11.afl / Combinations with 5DP.afl.
fn window(id: &str, x: &Ctx, g: &dyn Fn(&str) -> f64, rf: &[bool], n: usize) -> Vec<Ev> {
    let (ph, ex) = phenomena(id, x, g, n);
    let k = g("n").round().max(0.0) as usize;
    let ll: Vec<bool> = (0..n).map(|i| i > 0 && x.l[i] < x.l[i - 1] && x.l[i - 1] != 0.0).collect();
    let lh: Vec<bool> = (0..n).map(|i| i > 0 && x.h[i] < x.h[i - 1] && x.h[i - 1] != 0.0).collect();
    let (llc, lhc) = (last_n(&ll, k), last_n(&lh, k));
    let et = g("entry_type").round() as i64;
    let entry: Vec<bool> = (0..n)
        .map(|i| {
            let pat = match et {
                1 => lhc[i],
                2 => llc[i],
                _ => llc[i] && lhc[i],
            };
            i != n - 1 && x.c[i] > 0.0 && ph[i] && pat && rf[i]
        })
        .collect();
    let live = g("exit_live") > 0.5;
    let (hsl_p, pb_p, tsl_p, buf) = (g("hsl"), g("pb"), g("tsl"), g("exit_buffer"));
    let mut evs: Vec<Ev> = Vec::new();
    let mut pos = false;
    let (mut hsl, mut pb, mut tsl): (Option<f64>, Option<f64>, Option<f64>) = (None, None, None);
    for i in 1..n {
        let tsl_prev = tsl;
        if entry[i] && !pos {
            let c = x.c[i];
            evs.push(Ev { dir: 1.0, e: i, epx: c, exit: None });
            pos = true;
            hsl = (hsl_p > 0.0).then(|| c * (1.0 - hsl_p * 0.01));
            pb = (pb_p > 0.0).then(|| c * (1.0 + pb_p * 0.01));
            tsl = (tsl_p > 0.0).then(|| c * (1.0 - tsl_p * 0.01));
        } else if pos {
            let lo = if live { x.l[i] } else { x.c[i] };
            let hi = if live { x.h[i] } else { x.c[i] };
            let hsl_hit = hsl_p > 0.0 && hsl.is_some_and(|v| lo < v);
            let pb_hit = pb_p > 0.0 && pb.is_some_and(|v| hi > v);
            let tsl_hit = tsl_prev.is_some_and(|v| v != 0.0 && lo < v);
            let last = i == n - 1;
            if ex[i] || last || hsl_hit || pb_hit || tsl_hit {
                let mut px = x.c[i];
                let mut why = if ex[i] { "Window exit" } else if last { "Last bar" } else if tsl_hit { "Trailing stop" } else { "Close exit" };
                if live && hsl_hit {
                    let s = hsl.unwrap();
                    px = if x.o[i] < s { x.l[i].max(x.o[i] * (1.0 - buf * 0.01)) } else { s };
                }
                if live && pb_hit {
                    let s = pb.unwrap();
                    px = if x.o[i] > s { x.l[i].max(x.o[i] * (1.0 - buf * 0.01)) } else { s };
                }
                if hsl_hit {
                    why = "Hard stop";
                }
                if pb_hit {
                    why = "Profit booking";
                }
                if let Some(v) = evs.last_mut() {
                    v.exit = Some((i, bound(x, i, px), i, why));
                }
                pos = false;
                hsl = None;
                pb = None;
                tsl = None;
            }
            if let (Some(t), true) = (tsl_prev, pos) {
                if t != 0.0 {
                    tsl = Some((x.h[i] * (1.0 - tsl_p * 0.01)).max(t));
                }
            }
        }
    }
    evs
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let name = catalogue().into_iter().find(|d| d.id == id)?.name;
    Some((match id {
        BASE => "ytlh_lower_high_lower_low",
        TWO_11 => "ytlh_2_11_lower_high_lower_low",
        FIVE_DP => "ytlh_5dp_lower_high_lower_low",
        _ => return None,
    }, name, "Lower High and Lower Low"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::books::ytwh::testbars::*;

    fn bar(c: f64) -> (f64, f64, f64, f64) {
        (c, c + 1.0, c - 1.0, c)
    }

    #[test]
    fn last_n_needs_n_consecutive() {
        assert_eq!(last_n(&[true, true, false, true, true, true], 2), vec![false, true, false, false, true, true]);
        assert_eq!(last_n(&[false, true], 0), vec![true, true]);
    }

    #[test]
    fn base_buys_after_n_lower_lows_and_exits_after_n_bars() {
        // bars 1-4 make 4 lower lows -> buy bar 4; 5 bars later (bar 9) sell unless the pattern repeats
        let mut v: Vec<_> = (0..5).map(|k| bar(100.0 - k as f64)).collect();
        v.extend((0..8).map(|k| bar(97.0 + k as f64)));
        let t = go(run, BASE, &bars(&v), &[]);
        assert_eq!((t[0].0, t[0].1, t[0].2, t[0].3), (4, 96.0, 9, 101.0));
        // exit after 1 candle, LL & LH with N = 2
        let t = go(run, BASE, &bars(&v), &[("entry_type", 3.0), ("ll_n", 2.0), ("lh_n", 2.0), ("exit_n", 1.0)]);
        // bars 2, 3, 4 all qualify: bar 2 buys, bar 3 reaches the count but the signal is true -> stays; bar 5 sells
        assert_eq!((t[0].0, t[0].2), (2, 5));
    }

    #[test]
    fn two_11_window_and_stops() {
        // 2024-01-01 is a Monday; January has 23 weekdays (bars 0-22), February starts at bar 23
        let mut v: Vec<_> = (0..40).map(|k| bar(100.0 + (k % 3) as f64)).collect();
        // bars 21, 22, 23 fall: bar 23 (1st bar of February) has 2 lower lows and highs
        v[21] = bar(99.0);
        v[22] = bar(98.0);
        v[23] = bar(97.0);
        let r = bars(&v);
        let x = Ctx::new(&r, &r, &[]);
        assert!(eom(&x, r.len())[22] && !eom(&x, r.len())[21]);
        let t = go(run, TWO_11, &r, &[]);
        assert_eq!((t[0].0, t[0].2), (23, 24));
        // a longer window keeps the trade to its exit bar; a 1% profit booking at the close exits earlier
        v[24] = bar(99.0);
        let t = go(run, TWO_11, &bars(&v), &[("exit_day", 5.0)]);
        assert_eq!((t[0].0, t[0].2), (23, 28));
        let t = go(run, TWO_11, &bars(&v), &[("exit_day", 5.0), ("pb", 1.0)]);
        assert_eq!((t[0].2, t[0].3), (24, 99.0));
        // live: bar 24 opens at 99, above the 97.97 booking level -> fill at the open; an open below it fills at the level
        let t = go(run, TWO_11, &bars(&v), &[("exit_day", 5.0), ("pb", 1.0), ("exit_live", 1.0)]);
        assert_eq!((t[0].2, t[0].3), (24, 99.0));
        v[24] = (97.5, 99.0, 97.0, 98.5);
        let t = go(run, TWO_11, &bars(&v), &[("exit_day", 5.0), ("pb", 1.0), ("exit_live", 1.0)]);
        assert_eq!(t[0].2, 24);
        assert!((t[0].3 - 97.0 * 1.01).abs() < 1e-9);
    }

    #[test]
    fn five_dp_window() {
        // January (31 days): entry on the 5th last bar (bar 18) to the 2nd bar of February (bar 24)
        let v: Vec<_> = (0..45).map(|k| bar(100.0 - k as f64)).collect();
        let r = bars(&v);
        let x = Ctx::new(&r, &r, &[]);
        let g = |k: &str| params(FIVE_DP).unwrap().into_iter().find(|d| d.key == k).unwrap().def;
        let (ph, ex) = phenomena(FIVE_DP, &x, &g, r.len());
        assert!(!ph[17] && ph[18] && ph[23] && !ph[24]);
        assert!(ex[24]);
        let t = go(run, FIVE_DP, &r, &[]);
        assert_eq!((t[0].0, t[0].2), (18, 24));
    }

    #[test]
    fn every_strategy_runs() {
        smoke(run, &IDS);
    }
}
