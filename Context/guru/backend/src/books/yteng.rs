//! YT Strategies · Engulfing Candle: the team's AmiBroker AFLs in "Old Topics/Engulfing Candle" (N red candles then a
//! bullish engulfing candle), run on an index's daily bars. Coded from the AFLs as written, with the AFLs' own default
//! settings; there is no book PDF and nothing here is checked against the team's workbooks.
//!
//! AmiBroker behaviour followed (see `ytsw.rs` and `ytd7.rs`): the AFL's bar loop sets Buy / Sell at the close (stop,
//! target and trailing levels override the price); the backtester enters only when flat and exits first, then
//! enters, on one bar; inside the loop a Null element is -1e10; whole-array comparisons are false before an array's
//! first value. The ST here is the AFL's own "IsUp" loop (no band reset on a flip): ATR = mean of the last P ATR(1)
//! values, up / down bands = base +/- factor x ATR, from bar P the trend is up on a close above the previous upper
//! band and down on a close below the previous lower band; in an up trend the lower band only rises, in a down trend
//! the upper band only falls. On a weekly / monthly timeframe the AFL takes Ref(-1) of the period values, expands
//! them with expandFirst and shifts them Ref(+1) on daily bars, so each daily bar sees the last period completed at
//! its close (the first period and the last data bar see Null).
#![allow(dead_code)]
use crate::books::ytd7::{fills, lv, tf_char, Exit, NULL};
use crate::books::ytsw::{compress, true_range};
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "yt_eng";
pub const LOW: &str = "yteng_open_below_low";
pub const CLOSE_BUF: &str = "yteng_open_below_close_buffer";
pub const IDS: [&str; 2] = [LOW, CLOSE_BUF];

fn def(id: &'static str, name: &'static str, dir: &'static str, rules: &'static [&'static str], notes: &'static str) -> StrategyDef {
    StrategyDef { id, name, group: "YT Strategies · Engulfing Candle", session: 0, dir, status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim: None }
}

macro_rules! common_note { () => { " With every exit at its default (stops 0, N candles 0, filters off) the only exit is the AFL's forced sell on the last data bar, shown here as an open position: a run needs an exit switched on. An exit on a bar that has a new entry signal keeps the position and restarts its levels. If the ST entry filter then blocks the re-entry, the AFL keeps a position the backtester no longer holds, and its next sells are ignored. Unlike the classifier summary, the AFL does not block entries on the last bar (its dead-stock flag is never used)." }; }

pub fn catalogue() -> Vec<StrategyDef> {
    vec![
        def(LOW, "N Red Candles + Engulf (open below previous low)", "Long", &[
            "F|Optional ST filter (off by default; Daily/Weekly/Monthly, period 5, factor 1, mid point): bullish mode buys only while the close is above the ST",
            "E|Buy at the close when the previous N candles (default 1) were red and today opens below the previous low and closes above the previous high; or (candle type 1) after N up closes",
            "X|Any switched-on exit: N-candle / green / up-close count, % profit booking, hard SL, trailing SL, RSI crossing a level, RSI trail, the ST filter turning off, or the close crossing below an exit ST",
        ], concat!("N_Redcandle_Previous_ENGULF.afl, coded as the AFL is written.", common_note!(), " In Bearish ST-filter mode the entry needs the close below the ST and the exit fires above it.")),
        def(CLOSE_BUF, "N Red Candles + Engulf (open below previous close, buffer)", "Long", &[
            "F|Optional ST filter as in the open-below-low version",
            "E|Buy at the close when the previous N candles (default 1) were red, today opens below the previous close and closes above the previous high; with a buffer % above 0, a close within that % below the previous high (or above it) is enough",
            "X|The same switchable exits as the open-below-low version",
        ], concat!("N_Redcandle_Previous_ENGULF_Buffer.afl, coded as the AFL is written (N red candles is an Optimize() with default 1).", common_note!())),
    ]
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    if !IDS.contains(&id) {
        return None;
    }
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    let src = "ST price (0 close, 1 open, 2 high, 3 low, 4 (O+C)/2, 5 (H+L)/2, 6 (H+L+C)/3, 7 (O+H+L+C)/4)";
    let mut v = vec![
        p("st_filter_on", "ST filter (0 off, 1 on)", 0.0, 0.0, 1.0, 1.0),
        p("st_filter_tf", "ST filter timeframe (0 daily, 1 weekly, 2 monthly)", 0.0, 0.0, 2.0, 1.0),
        p("st_filter_period", "ST filter period", 5.0, 5.0, 5.0, 1.0),
        p("st_filter_factor", "ST filter factor", 1.0, 1.0, 6.0, 0.5),
        p("st_filter_src", src, 5.0, 0.0, 7.0, 1.0),
        p("st_entry_mode", "ST entry filter (0 bullish, 1 bearish)", 0.0, 0.0, 1.0, 1.0),
        p("n_red", "N red candles", 1.0, 1.0, if id == LOW { 100.0 } else { 5.0 }, 1.0),
    ];
    if id == CLOSE_BUF {
        v.push(p("engulf_buffer_pct", "Engulf buffer % below the previous high (0 off)", 0.0, 0.0, 10.0, 0.01));
    }
    v.extend([
        p("candle_type", "Entry candle (0 engulf, 1 N up closes)", 0.0, 0.0, 1.0, 1.0),
        p("n_upclose", "N up closes (candle type 1)", 1.0, 1.0, 100.0, 1.0),
        p("st_exit_on", "ST exit (0 off, 1 on)", 0.0, 0.0, 1.0, 1.0),
        p("st_exit_tf", "ST exit timeframe (0 daily, 1 weekly, 2 monthly)", 0.0, 0.0, 2.0, 1.0),
        p("st_exit_period", "ST exit period", 5.0, 5.0, 5.0, 1.0),
        p("st_exit_factor", "ST exit factor", 1.0, 1.0, 6.0, 0.5),
        p("st_exit_src", src, 5.0, 0.0, 7.0, 1.0),
        p("rsi_exit_on", "RSI exit (0 off, 1 on)", 0.0, 0.0, 1.0, 1.0),
        p("rsi_period", "RSI period", 14.0, 1.0, 80.0, 1.0),
        p("rsi_dir", "RSI exit when it crosses (0 below, 1 above) the threshold", 0.0, 0.0, 1.0, 1.0),
        p("rsi_thr", "RSI threshold", 1.0, 1.0, 100.0, 1.0),
        p("rsi_trail_on", "RSI trail exit (0 off, 1 on)", 0.0, 0.0, 1.0, 1.0),
        p("rsi_drop", "RSI drop value", 20.0, 1.0, 100.0, 1.0),
        p("exit_candle_type", "Count exit candle (0 every candle, 1 green candle, 2 up close)", 0.0, 0.0, 2.0, 1.0),
        p("n_exit_candles", "N candles for the count exit (0 off)", 0.0, 0.0, 500.0, 1.0),
        p("exit_at", "Stop / target test (0 live: high / low, 1 close)", 0.0, 0.0, 1.0, 1.0),
        p("pb_pct", "Profit booking % (0 off)", 0.0, 0.0, 50.0, 0.1),
        p("hsl_pct", "Hard SL % (0 off)", 0.0, 0.0, 20.0, 0.1),
        p("tsl_pct", "Trailing SL % (0 off)", 0.0, 0.0, 100.0, 0.1),
        p("buffer_pct", "Buffer % on a gap through a level", 0.0, 0.0, 10.0, 0.1),
    ]);
    Some(v)
}

// ---------------------------------------------------------------- indicators

/// AFL RSI(n): Wilder smoothing seeded with the simple average of the first n changes; Null before bar n
/// (and everywhere when n <= 1, as the AFL only computes it for periods above 1).
pub(crate) fn rsi(c: &[f64], n: usize) -> Vec<Option<f64>> {
    let len = c.len();
    let mut out = vec![None; len];
    if n <= 1 || len <= n {
        return out;
    }
    let (mut au, mut ad) = (0.0, 0.0);
    for i in 1..len {
        let d = c[i] - c[i - 1];
        let (u, dn) = (d.max(0.0), (-d).max(0.0));
        if i <= n {
            au += u / n as f64;
            ad += dn / n as f64;
        } else {
            au = (au * (n as f64 - 1.0) + u) / n as f64;
            ad = (ad * (n as f64 - 1.0) + dn) / n as f64;
        }
        if i >= n {
            out[i] = Some(if au + ad == 0.0 { 50.0 } else { 100.0 * au / (au + ad) });
        }
    }
    out
}

/// AFL Cross(a, b) on arrays that may hold Null: no cross before both have a value; later Nulls are -1e10.
pub(crate) fn cross(a: &[Option<f64>], b: &[Option<f64>]) -> Vec<bool> {
    let first = (0..a.len()).position(|i| a[i].is_some() && b[i].is_some()).unwrap_or(a.len());
    (0..a.len()).map(|i| i > first && lv(a[i]) > lv(b[i]) && lv(a[i - 1]) <= lv(b[i - 1])).collect()
}

/// The AFL's ST "IsUp" loop on (o, h, l, c) bars: the ST line (lower band in an up trend, upper band in a down trend).
fn st_line(o: &[f64], h: &[f64], l: &[f64], c: &[f64], period: usize, factor: f64, src: i64) -> Vec<Option<f64>> {
    let m = c.len();
    let p = period.max(1);
    let tr = true_range(h, l, c);
    let atr: Vec<Option<f64>> = (0..m)
        .map(|i| if i + 1 < p { None } else { tr[i + 1 - p..=i].iter().try_fold(0.0, |s, v| v.map(|v| s + v)).map(|s| s / p as f64) })
        .collect();
    let base = |i: usize| match src {
        1 => o[i],
        2 => h[i],
        3 => l[i],
        4 => (o[i] + c[i]) / 2.0,
        6 => (h[i] + l[i] + c[i]) / 3.0,
        7 => (o[i] + h[i] + l[i] + c[i]) / 4.0,
        0 => c[i],
        _ => (h[i] + l[i]) / 2.0,
    };
    let mut up: Vec<f64> = (0..m).map(|i| atr[i].map_or(NULL, |a| base(i) + a * factor)).collect();
    let mut dn: Vec<f64> = (0..m).map(|i| atr[i].map_or(NULL, |a| base(i) - a * factor)).collect();
    let mut is_up = vec![true; m];
    for i in p..m {
        is_up[i] = if c[i] > up[i - 1] { true } else if c[i] < dn[i - 1] { false } else { is_up[i - 1] };
        if is_up[i] {
            dn[i] = dn[i].max(dn[i - 1]);
        } else {
            up[i] = up[i].min(up[i - 1]);
        }
    }
    (0..m).map(|i| atr[i].map(|_| if is_up[i] { dn[i] } else { up[i] })).collect()
}

/// (ST line, Close_M) on daily bars for timeframe code 0 daily, 1 weekly, 2 monthly.
fn st_daily(x: &Ctx, tf: i64, period: usize, factor: f64, src: i64) -> (Vec<Option<f64>>, Vec<Option<f64>>) {
    let n = x.c.len();
    if tf_char(tf) == 'D' {
        return (st_line(&x.o, &x.h, &x.l, &x.c, period, factor, src), x.c.iter().map(|&v| Some(v)).collect());
    }
    let b = compress(x, tf_char(tf));
    let mut o = vec![0.0; b.c.len()];
    for i in (0..n).rev() {
        o[b.group[i]] = x.o[i];
    }
    let st = st_line(&o, &b.h, &b.l, &b.c, period, factor, src);
    // Ref(-1) on the periods, expandFirst, Ref(+1) on the days
    let at = |i: usize, v: &dyn Fn(usize) -> Option<f64>| -> Option<f64> {
        if i + 1 >= n {
            return None;
        }
        let g = b.group[i + 1];
        if g == 0 { None } else { v(g - 1) }
    };
    ((0..n).map(|i| at(i, &|g| st[g])).collect(), (0..n).map(|i| at(i, &|g| Some(b.c[g]))).collect())
}

// ---------------------------------------------------------------- the AFL loop

fn signals(id: &str, x: &Ctx, g: &dyn Fn(&str) -> f64) -> (Vec<Option<f64>>, Vec<Exit>) {
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let n = c.len();
    let gi = |k: &str| g(k).round() as i64;
    let gu = |k: &str| g(k).round().max(1.0) as usize;
    let nred = gu("n_red");
    let nup = gu("n_upclose");
    // Sum(C < O, nRed) == nRed, false before the window exists
    let red_ok: Vec<bool> = (0..n).map(|i| i + 1 >= nred && (i + 1 - nred..=i).all(|k| c[k] < o[k])).collect();
    let up_ok: Vec<bool> = (0..n).map(|i| i >= nup && (i + 1 - nup..=i).all(|k| c[k] > c[k - 1])).collect();
    let buf = if id == CLOSE_BUF { g("engulf_buffer_pct") } else { 0.0 };
    let entry: Vec<bool> = (0..n)
        .map(|i| {
            if gi("candle_type") == 1 {
                return i >= nup && red_ok[i - nup] && up_ok[i];
            }
            if i == 0 || !red_ok[i - 1] {
                return false;
            }
            if id == LOW {
                c[i] > h[i - 1] && o[i] < l[i - 1]
            } else if buf == 0.0 {
                c[i] > h[i - 1] && o[i] < c[i - 1]
            } else {
                o[i] < c[i - 1] && (h[i - 1] - c[i]) / h[i - 1] * 100.0 < buf
            }
        })
        .collect();
    let st_on = gi("st_filter_on") == 1;
    let bear_mode = gi("st_entry_mode") == 1;
    let st_bull: Vec<bool> = if st_on {
        let (st, cm) = st_daily(x, gi("st_filter_tf"), gu("st_filter_period"), g("st_filter_factor"), gi("st_filter_src"));
        let first = (0..n).position(|i| st[i].is_some() && cm[i].is_some()).unwrap_or(n);
        (0..n).map(|i| i >= first && lv(cm[i]) > lv(st[i])).collect()
    } else {
        vec![false; n]
    };
    let st_exit: Vec<bool> = if gi("st_exit_on") == 1 {
        let (st, cm) = st_daily(x, gi("st_exit_tf"), gu("st_exit_period"), g("st_exit_factor"), gi("st_exit_src"));
        cross(&st, &cm)
    } else {
        vec![false; n]
    };
    let rv = rsi(c, g("rsi_period").round().max(0.0) as usize);
    let thr = vec![Some(g("rsi_thr")); n];
    let rsi_x = if gi("rsi_dir") == 0 { cross(&thr, &rv) } else { cross(&rv, &thr) };
    let (rsi_on, trail_on, drop) = (gi("rsi_exit_on") == 1, gi("rsi_trail_on") == 1, g("rsi_drop"));
    let (ctype, nexit, at_close) = (gi("exit_candle_type"), g("n_exit_candles").round(), gi("exit_at") == 1);
    let (pbp, hslp, tslp, bufp) = (g("pb_pct"), g("hsl_pct"), g("tsl_pct"), g("buffer_pct"));
    let (mut pb, mut hsl, mut tsl, mut trail) = (vec![NULL; n], vec![NULL; n], vec![NULL; n], vec![0.0; n]);
    let mut buy = vec![None; n];
    let mut sell: Vec<Exit> = vec![None; n];
    let (mut pos, mut flag, mut count) = (false, false, 0.0);
    for i in nred + 1..n {
        pb[i] = pb[i - 1];
        hsl[i] = hsl[i - 1];
        tsl[i] = tsl[i - 1];
        trail[i] = trail[i - 1];
        if pos {
            let lo = if at_close { c[i] } else { l[i] };
            let hi = if at_close { c[i] } else { h[i] };
            let hsl_hit = hslp > 0.0 && lo <= hsl[i];
            let pb_hit = pbp > 0.0 && hi >= pb[i];
            let tsl_hit = tslp > 0.0 && lo <= tsl[i];
            match ctype {
                1 if o[i] < c[i] => count += 1.0,
                2 if c[i] > c[i - 1] => count += 1.0,
                0 => count += 1.0,
                _ => {}
            }
            let cnt_exit = count == nexit;
            let flt_out = st_on && if bear_mode { st_bull[i] } else { !st_bull[i] };
            let rule = cnt_exit || hsl_hit || pb_hit || tsl_hit || (rsi_on && rsi_x[i]) || flt_out || st_exit[i] || (trail_on && lv(rv[i]) < trail[i]);
            if i == n - 1 || rule {
                if entry[i] && !flag {
                    flag = true;
                } else {
                    let gap = |lvl: f64, above: bool| if (above && o[i] > lvl) || (!above && o[i] < lvl) { l[i].max(o[i] * (1.0 - bufp * 0.01)) } else { lvl };
                    let mut px = (c[i], if rule { "Exit rule" } else { "Last bar" });
                    if pb_hit && !at_close {
                        px = (gap(pb[i], true), "Profit booking");
                    }
                    if hsl_hit && !at_close {
                        px = (gap(hsl[i], false), "Hard SL");
                    }
                    if tsl_hit && !at_close {
                        px = (gap(tsl[i], false), "Trailing SL");
                    }
                    sell[i] = Some(px);
                    pos = false;
                    trail[i] = 0.0;
                    pb[i] = NULL;
                    hsl[i] = NULL;
                    tsl[i] = NULL;
                }
            }
            if tslp > 0.0 {
                tsl[i] = if tsl[i] > 0.0 { (h[i] * (1.0 - tslp * 0.01)).max(tsl[i - 1]) } else { NULL };
            }
            if trail_on {
                trail[i] = trail[i].max(lv(rv[i]) - drop);
            }
        }
        let flt_in = !st_on || if bear_mode { !st_bull[i] } else { st_bull[i] };
        if (!pos || flag) && entry[i] && flt_in {
            if !flag {
                buy[i] = Some(c[i]);
            }
            hsl[i] = if hslp > 0.0 { c[i] * (1.0 - hslp * 0.01) } else { NULL };
            pb[i] = if pbp > 0.0 { c[i] * (1.0 + pbp * 0.01) } else { NULL };
            tsl[i] = if tslp > 0.0 { c[i] * (1.0 - tslp * 0.01) } else { NULL };
            trail[i] = lv(rv[i]) - drop;
            pos = true;
            count = 0.0;
            flag = false;
        }
    }
    (buy, sell)
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(0.0, |d| d.def));
    let n = x.c.len();
    if n == 0 || from > to || to >= n {
        return Some(Vec::new());
    }
    let (b, s) = signals(id, x, &g);
    Some(fills(x, 1.0, &b, &s, from, to, "close", false, 0.0))
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let name = catalogue().into_iter().find(|d| d.id == id)?.name;
    Some((match id {
        LOW => "yteng_low",
        _ => "yteng_close_buffer",
    }, name, "Engulfing Candle"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::books::ytd7::tests::{bars, ps, wave};

    fn go(id: &str, ohlc: &[(f64, f64, f64, f64)], kv: &[(&str, f64)]) -> Vec<(usize, f64, usize, f64, bool)> {
        let r = bars(ohlc);
        let x = Ctx::new(&r, &r, &[]);
        run(id, &x, 0, r.len() - 1, &ps(kv)).unwrap().into_iter().map(|t| (t.legs[0].i, t.legs[0].px, t.exit_i, t.exit_px, t.open)).collect()
    }
    const G: (f64, f64, f64, f64) = (100.0, 102.0, 99.0, 101.0); // green
    const R: (f64, f64, f64, f64) = (101.0, 101.5, 99.0, 99.5); // red: low 99, high 101.5, close 99.5

    #[test]
    fn engulf_entry_and_profit_booking() {
        // bar 2 red; bar 3 opens 98.5 (< low 99 and < close 99.5) and closes 102 > 101.5 -> buy at 102
        let v = [G, G, R, (98.5, 102.5, 98.0, 102.0), (102.0, 103.0, 101.0, 102.5), (102.5, 106.0, 102.0, 105.0)];
        // defaults: no exit but the last bar -> open
        assert_eq!(go(LOW, &v, &[]), vec![(3, 102.0, 5, 105.0, true)]);
        // 3% profit booking: level 105.06, hit on bar 5 at the level
        let t = go(LOW, &v, &[("pb_pct", 3.0)]);
        assert_eq!((t[0].2, t[0].4), (5, false));
        assert!((t[0].3 - 102.0 * 1.03).abs() < 1e-9);
        // the buffer AFL with buffer 0 also needs only an open below the previous close
        let mut w = v.to_vec();
        w[3] = (99.2, 102.5, 98.0, 102.0); // open 99.2: above the low 99, below the close 99.5
        assert!(go(LOW, &w, &[]).is_empty());
        assert_eq!(go(CLOSE_BUF, &w, &[]).len(), 1);
        // buffer 1%: close 101.0 is within 1% of the previous high 101.5
        w[3] = (99.2, 101.2, 98.0, 101.0);
        assert!(go(CLOSE_BUF, &w, &[]).is_empty());
        assert_eq!(go(CLOSE_BUF, &w, &[("engulf_buffer_pct", 1.0)]).len(), 1);
    }

    #[test]
    fn count_exit_and_stop_gap() {
        let v = [G, G, R, (98.5, 102.5, 98.0, 102.0), (102.0, 103.0, 101.0, 102.5), (102.5, 104.0, 102.0, 103.0), (103.0, 104.0, 102.0, 103.5)];
        // exit after 2 candles: bar 5's close
        assert_eq!(go(LOW, &v, &[("n_exit_candles", 2.0)]), vec![(3, 102.0, 5, 103.0, false)]);
        // 1% hard SL (100.98); bar 4 opens 99 below it: exit at max(low, open) = 99
        let mut w = v.to_vec();
        w[4] = (99.0, 100.0, 98.5, 99.5);
        let t = go(LOW, &w, &[("hsl_pct", 1.0)]);
        assert_eq!((t[0].2, t[0].3), (4, 99.0));
    }

    #[test]
    fn rsi_matches_wilder() {
        let c: Vec<f64> = (0..20).map(|k| 100.0 + (k as f64).sin() * 3.0).collect();
        let r = rsi(&c, 14);
        assert!(r[13].is_none() && r[14].is_some());
        assert!(r.iter().flatten().all(|v| (0.0..=100.0).contains(v)));
    }

    #[test]
    fn every_strategy_runs() {
        let r = wave(400);
        let x = Ctx::new(&r, &r, &[]);
        for id in IDS {
            for (k, v) in [("st_filter_on", 0.0), ("st_filter_on", 1.0), ("st_exit_on", 1.0)] {
                for tf in [0.0, 1.0, 2.0] {
                    let kv = [(k, v), ("st_filter_tf", tf), ("st_exit_tf", tf), ("n_exit_candles", 3.0), ("rsi_exit_on", 1.0), ("rsi_trail_on", 1.0), ("tsl_pct", 2.0)];
                    let t = run(id, &x, 30, r.len() - 1, &ps(&kv)).unwrap();
                    assert!(t.iter().all(|t| t.exit_i >= t.legs[0].i && t.legs[0].i >= 30), "{id}");
                }
            }
            assert!(family_of(id).is_some());
        }
    }
}
