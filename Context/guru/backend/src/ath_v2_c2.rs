//! ATH sub-variants of batch c2: 52-week-high / ATHC RESHUFFLING (16 ids) and the long-term-high AFLs (4 ids).
//!
//! Reshuffling family (one parameterised core, a `Spec` per id): every watchlist stock is ranked per date by `diff = Close / ATHC`
//! (`ATHC` = `HHV(High, DAYSHIGH)` for the 52-week files, `HHV(Close, BarCount)` for the ATHC files; 0 for dead stock, penny, failed turnover,
//! outside the 10 % range, Supertrend down, ... depending on the file); on the reshuffle bar of each symbol (`resufflpd`, evaluated on the bar
//! calendar of the symbol: Monthly = last bar of the month = `Month() != Ref(Month(), 1)`, Weekly = `eow`, Quarterly = `Ref(eom, -entryday)` in
//! March/June/September/December, Half Yearly, Yearly) the stock is bought when `rank <= RANKHELD` and sold when `rank > EXITRANK`; the AFL keeps
//! two latched states (`buysig2` / `sellsig2`) that re-buy / re-sell on every later bar until the other side fires (no `ExRem`, raw backtest mode:
//! the portfolio ignores a Buy while the symbol is held and a Sell while it is not).
//! `StaticVarGenerateRanks(mode 0)`: rank 1 = highest value, ties by watchlist order (the AFL breaks ties with a seeded shuffle, not reproducible).
//! List parameters are 0-based indexes (as in the other ATH modules): deadstock 0 YES / 1 NO, turnflt 0 NO / 1 YES, resf 0 Monthly / 1 Weekly /
//! 2 Quarterly / 3 Half Yearly / 4 Yearly, stflt 0 CLOSE / 1 MID POINT / 2 PIVOT, stflt_2 (the filter files' second list) 0 MID POINT / 1 CLOSE /
//! 2 PIVOT, mktflt 0 NONE / 1 STOCK / 2 INDEX / 3 BOTH, flttf 0 MONTHLY / 1 WEEKLY / 2 DAILY, sflt / iflt 0 ST / 1 SMA, prflt 0 NSENIFTY / 1 NIFTY /
//! 2 NSE500 (NIFTY is not in the database: the filter is then never true).
//! Long-term-high files: `ytath_3_5_year_highest_close_buy_with(_2)`, `ytath_52_week_high_buy_with_pctswing_exit`, `ytath_250_day_highest_close_breakout_ranked`.
//! `Optimize()` parameters are not exposed: constants at the AFL default clamped into the Optimize range.
//! Verification: `ATH/v2/<id>/VERIFY.md`.
#![allow(dead_code)]
#[allow(unused_imports)]
use crate::ath_core::*;
use std::collections::{HashMap, HashSet, VecDeque};

// ---------------------------------------------------------------- AmiBroker Null and small helpers

/// AmiBroker's Null (EMPTY_VAL): arithmetic with it gives Null, a comparison sees the number.
const NULL: f32 = -1.0e10;
fn is_null(x: f32) -> bool {
    x <= -9.0e9
}
fn nadd(a: f32, b: f32) -> f32 {
    if is_null(a) || is_null(b) { NULL } else { a + b }
}
fn nsub(a: f32, b: f32) -> f32 {
    if is_null(a) || is_null(b) { NULL } else { a - b }
}
fn nmul(a: f32, b: f32) -> f32 {
    if is_null(a) || is_null(b) { NULL } else { a * b }
}

/// `Volsig` of the reshuffling / "loss" AFLs (compared with V*C).
fn volsig_a(y: i32) -> f32 {
    match y {
        y if y < 2000 => 30_000.0,
        2000..=2004 => 50_000.0,
        2005..=2009 => 1_800_000.0,
        2010..=2014 => 2_600_000.0,
        2015..=2019 => 6_000_000.0,
        _ => 13_000_000.0,
    }
}

/// `Volsig` of the long yearly table (3/5-year %swing, 52-week buy, 250-day files).
fn volsig_b(y: i32) -> f32 {
    match y {
        y if y < 2000 => 30_000.0,
        2000..=2003 => 50_000.0,
        2004 => 200_000.0,
        2005 => 400_000.0,
        2006 => 600_000.0,
        2007..=2009 => 800_000.0,
        2010 => 1_000_000.0,
        2011 => 2_000_000.0,
        2012 => 2_500_000.0,
        2013 => 3_000_000.0,
        2014 => 3_500_000.0,
        2015 | 2016 => 4_000_000.0,
        2017 => 5_000_000.0,
        2018 => 6_000_000.0,
        2019 => 7_000_000.0,
        2020 => 8_000_000.0,
        _ => 9_000_000.0,
    }
}

fn year(d: i32) -> i32 {
    d / 10000
}
fn month(d: i32) -> i32 {
    d / 100 % 100
}
fn day(d: i32) -> i32 {
    d % 100
}
fn days_from_civil(y: i32, m: i32, d: i32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y } as i64;
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = (m as i64 + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}
/// AmiBroker `DayOfWeek()`: 0 Sunday .. 6 Saturday.
fn dow(d: i32) -> i32 {
    (days_from_civil(year(d), month(d), day(d)) + 4).rem_euclid(7) as i32
}

/// `HHV(a, n)`: highest of the last n bars including the bar (the window is what exists at the start of the array).
fn hhv_n(a: &[f32], n: usize) -> Vec<f32> {
    let n = n.max(1);
    let mut dq: VecDeque<usize> = VecDeque::new();
    let mut out = vec![0f32; a.len()];
    for i in 0..a.len() {
        while let Some(&b) = dq.back() {
            if a[b] <= a[i] { dq.pop_back(); } else { break; }
        }
        dq.push_back(i);
        while let Some(&f) = dq.front() {
            if f + n <= i { dq.pop_front(); } else { break; }
        }
        out[i] = a[*dq.front().unwrap()];
    }
    out
}

/// `HHV(C, BarCount)`: highest close up to the bar.
fn hhv_all(c: &[f32]) -> Vec<f32> {
    let mut out = vec![0f32; c.len()];
    for i in 0..c.len() {
        out[i] = if i == 0 || c[i] > out[i - 1] { c[i] } else { out[i - 1] };
    }
    out
}

/// O,H,L,C of the bar equal those of each of the NEXT 30 bars (`Ref(x, k)`, k = 1..30 looks forward; beyond the last bar it is Null: false).
fn flat30(s: &Series) -> Vec<bool> {
    let n = s.bars.len();
    let same = |i: usize| {
        let (a, b) = (&s.bars[i], &s.bars[i + 1]);
        a.o == b.o && a.h == b.h && a.l == b.l && a.c == b.c
    };
    let mut out = vec![false; n];
    let mut run = 0usize;
    for i in (0..n).rev() {
        if i + 1 < n && same(i) { run += 1; } else { run = 0; }
        out[i] = run >= 30;
    }
    out
}

/// (UPcircuit, locircuit) with `Prec(x, 2)` equality.
fn circuits(s: &Series) -> (Vec<bool>, Vec<bool>) {
    let n = s.bars.len();
    let (mut up, mut lo) = (vec![false; n], vec![false; n]);
    for i in 0..n {
        let b = &s.bars[i];
        up[i] = prec2(b.o) == prec2(b.c) && prec2(b.o) == prec2(b.h);
        lo[i] = prec2(b.o) == prec2(b.c) && prec2(b.o) == prec2(b.l);
    }
    (up, lo)
}

fn true_range(h: &[f32], l: &[f32], c: &[f32]) -> Vec<f32> {
    (0..h.len()).map(|i| if i == 0 { h[i] - l[i] } else { h[i].max(c[i - 1]) - l[i].min(c[i - 1]) }).collect()
}

/// STCAL: 0 MID POINT, 1 CLOSE, 2 PIVOT.
fn stcal_of(mode: u8, h: &[f32], l: &[f32], c: &[f32]) -> Vec<f32> {
    (0..c.len())
        .map(|i| match mode {
            0 => (h[i] + l[i]) / 2.0,
            2 => (h[i] + l[i] + c[i]) / 3.0,
            _ => c[i],
        })
        .collect()
}

/// The AFL's `ST(...)` function: `Up = stcal + 1 * satr`, `Dn = stcal - factor * satr`, `satr = MA(ATR(1), period)`; returns the trend (+1 / -1).
fn st_trend(stcal: &[f32], h: &[f32], l: &[f32], c: &[f32], period: usize, factor: f32) -> Vec<i8> {
    let n = c.len();
    let t = true_range(h, l, c);
    let period = period.max(1);
    let mut sum = 0f64;
    let satr: Vec<f32> = (0..n)
        .map(|i| {
            sum += t[i] as f64;
            if i >= period { sum -= t[i - period] as f64; }
            if i + 1 >= period { (sum / period as f64) as f32 } else { f32::NAN }
        })
        .collect();
    let mut up: Vec<f32> = (0..n).map(|i| stcal[i] + 1.0 * satr[i]).collect();
    let mut dn: Vec<f32> = (0..n).map(|i| stcal[i] - factor * satr[i]).collect();
    let mut trend = vec![1i8; n];
    for i in 1..n {
        let t1 = if c[i] > up[i - 1] { 1 } else if c[i] < dn[i - 1] { -1 } else { trend[i - 1] };
        trend[i] = t1;
        let flag1 = t1 < 0 && trend[i - 1] > 0;
        let flagh1 = t1 > 0 && trend[i - 1] < 0;
        if t1 > 0 && dn[i] < dn[i - 1] { dn[i] = dn[i - 1]; }
        if t1 < 0 && up[i] > up[i - 1] { up[i] = up[i - 1]; }
        if flag1 { up[i] = stcal[i] + 1.0 * satr[i]; }
        if flagh1 { dn[i] = stcal[i] - factor * satr[i]; }
    }
    trend
}

/// Symmetric Supertrend of the Ver4 fall-back (`Up = STCAL + 4 * satr`, `Dn = STCAL - 4 * satr`, satr = sum of ATR(1) over 10 bars / 10, summed
/// newest first like the AFL's loop); Null (NaN) satr while the history is short.
fn st_sym(stcal: &[f32], h: &[f32], l: &[f32], c: &[f32], period: usize, factor: f32) -> Vec<f32> {
    let n = c.len();
    let t = true_range(h, l, c);
    let satr: Vec<f32> = (0..n)
        .map(|i| {
            if period == 0 || i + 1 < period { return f32::NAN; }
            let mut v = t[i];
            for k in 1..period { v += t[i - k]; }
            v / period as f32
        })
        .collect();
    let mut up: Vec<f32> = (0..n).map(|i| stcal[i] + factor * satr[i]).collect();
    let mut dn: Vec<f32> = (0..n).map(|i| stcal[i] - factor * satr[i]).collect();
    let mut trend = vec![1f32; n];
    for i in 1..n {
        let t1 = if c[i] > up[i - 1] { 1.0 } else if c[i] < dn[i - 1] { -1.0 } else { trend[i - 1] };
        trend[i] = t1;
        let flag1 = t1 < 0.0 && trend[i - 1] > 0.0;
        let flagh1 = t1 > 0.0 && trend[i - 1] < 0.0;
        if t1 > 0.0 && dn[i] < dn[i - 1] { dn[i] = dn[i - 1]; }
        if t1 < 0.0 && up[i] > up[i - 1] { up[i] = up[i - 1]; }
        if flag1 { up[i] = stcal[i] + factor * satr[i]; }
        if flagh1 { dn[i] = stcal[i] - factor * satr[i]; }
    }
    trend
}

/// The %swing of the stock: Up = C + EXIT, Dn = C - EXIT, EXIT = C * EXITPCT / 100; returns the trend (+1 / -1).
fn pct_swing_trend(c: &[f32], exit_pct: f32) -> Vec<i8> {
    let n = c.len();
    let exit: Vec<f32> = c.iter().map(|&x| (x * exit_pct) / 100.0).collect();
    let mut up: Vec<f32> = (0..n).map(|i| c[i] + exit[i]).collect();
    let mut dn: Vec<f32> = (0..n).map(|i| c[i] - exit[i]).collect();
    let mut trend = vec![1i8; n];
    for i in 1..n {
        let t = if c[i] > up[i - 1] { 1 } else if c[i] < dn[i - 1] { -1 } else { trend[i - 1] };
        trend[i] = t;
        let flag1 = t < 0 && trend[i - 1] > 0;
        let flagh = t > 0 && trend[i - 1] < 0;
        if t > 0 && dn[i] < dn[i - 1] { dn[i] = dn[i - 1]; }
        if t < 0 && up[i] > up[i - 1] { up[i] = up[i - 1]; }
        if flag1 { up[i] = c[i] + exit[i]; }
        if flagh { dn[i] = c[i] - exit[i]; }
    }
    trend
}

fn clamp_prices(sg: &mut Signals, s: &Series) {
    for i in 0..sg.buy_price.len() {
        let b = &s.bars[i];
        sg.buy_price[i] = sg.buy_price[i].min(b.h).max(b.l);
        sg.sell_price[i] = sg.sell_price[i].min(b.h).max(b.l);
    }
}

fn dead_flags(n: usize, dead_yes: bool) -> (Vec<bool>, Vec<bool>) {
    // (deadstockbuysig, deadstocksellsig)
    if dead_yes {
        ((0..n).map(|i| i != n - 1).collect(), (0..n).map(|i| i == n - 1).collect())
    } else {
        ((0..n).map(|i| i != 0).collect(), vec![false; n])
    }
}

// ---------------------------------------------------------------- reshuffle dates

/// `eom` of the AFL: the next bar's day number is 1..5 and smaller than today's by the AFL's chain of tests.
fn eom_flags(dates: &[i32]) -> Vec<bool> {
    let n = dates.len();
    (0..n)
        .map(|i| {
            if i + 1 >= n { return false; }
            let nx = day(dates[i + 1]);
            let t = day(dates[i]);
            nx == 1
                || (nx == 2 && t != 1)
                || (nx == 3 && t != 1 && t != 2)
                || (nx == 4 && t != 1 && t != 2 && t != 3)
                || (nx == 5 && t != 1 && t != 2 && t != 3 && t != 4)
        })
        .collect()
}

/// `eow`: Sunday, or Wednesday..Saturday with the next bar's day of week smaller.
fn eow_flags(dates: &[i32]) -> Vec<bool> {
    let n = dates.len();
    (0..n)
        .map(|i| {
            let d = dow(dates[i]);
            let next_smaller = i + 1 < n && dow(dates[i + 1]) < d;
            ((d == 6 || d == 5 || d == 4 || d == 3) && next_smaller) || d == 0
        })
        .collect()
}

/// `resufflpd`: resf 0 Monthly, 1 Weekly, 2 Quarterly, 3 Half Yearly, 4 Yearly, on the bar calendar of the symbol.
fn resuffle_flags(dates: &[i32], resf: u8, entryday: usize) -> Vec<bool> {
    let n = dates.len();
    let month_end = |i: usize| i + 1 < n && month(dates[i]) != month(dates[i + 1]);
    match resf {
        1 => eow_flags(dates),
        0 => (0..n).map(month_end).collect(),
        2 => {
            let eom = eom_flags(dates);
            (0..n).map(|i| i >= entryday && eom[i - entryday] && matches!(month(dates[i]), 3 | 6 | 9 | 12)).collect()
        }
        3 => (0..n).map(|i| month_end(i) && matches!(month(dates[i]), 6 | 12)).collect(),
        _ => (0..n).map(|i| i + 1 < n && year(dates[i]) != year(dates[i + 1])).collect(),
    }
}

// ---------------------------------------------------------------- the monthly / weekly / daily stock and index filter (Ver2 / Ver3 / Ver4)

/// A symbol's price arrays aligned to the current symbol's bar dates (`SetForeign`: a missing bar repeats the previous close; before the start Null).
struct Al {
    o: Vec<f32>,
    h: Vec<f32>,
    l: Vec<f32>,
    c: Vec<f32>,
}

fn own_al(s: &Series) -> Al {
    Al { o: s.open(), h: s.high(), l: s.low(), c: s.close() }
}

fn align(idx: &Series, dates: &[i32]) -> Al {
    let m = idx.bars.len();
    let mut j = 0usize;
    let mut a = Al { o: Vec::with_capacity(dates.len()), h: Vec::with_capacity(dates.len()), l: Vec::with_capacity(dates.len()), c: Vec::with_capacity(dates.len()) };
    for &d in dates {
        while j < m && idx.bars[j].date <= d {
            j += 1;
        }
        if j == 0 {
            a.o.push(NULL);
            a.h.push(NULL);
            a.l.push(NULL);
            a.c.push(NULL);
        } else {
            let b = &idx.bars[j - 1];
            if b.date == d {
                a.o.push(b.o);
                a.h.push(b.h);
                a.l.push(b.l);
                a.c.push(b.c);
            } else {
                a.o.push(b.c);
                a.h.push(b.c);
                a.l.push(b.c);
                a.c.push(b.c);
            }
        }
    }
    a
}

fn period_key(d: i32, tf: u8) -> i64 {
    match tf {
        0 => (d / 100) as i64,
        1 => (days_from_civil(year(d), month(d), day(d)) + 3).div_euclid(7),
        _ => d as i64,
    }
}

struct Comp {
    start: Vec<usize>,
    end: Vec<usize>,
    o: Vec<f32>,
    h: Vec<f32>,
    l: Vec<f32>,
    c: Vec<f32>,
}

/// `TimeFrameSet(monthly / weekly / daily)` of an aligned array.
fn compress(dates: &[i32], al: &Al, tf: u8) -> Comp {
    let mut k = Comp { start: vec![], end: vec![], o: vec![], h: vec![], l: vec![], c: vec![] };
    let mut last_key = i64::MIN;
    for i in 0..dates.len() {
        let key = period_key(dates[i], tf);
        if key != last_key || k.start.is_empty() {
            k.start.push(i);
            k.end.push(i);
            k.o.push(al.o[i]);
            k.h.push(al.h[i]);
            k.l.push(al.l[i]);
            k.c.push(al.c[i]);
            last_key = key;
        } else {
            let g = k.start.len() - 1;
            k.end[g] = i;
            k.h[g] = k.h[g].max(al.h[i]);
            k.l[g] = k.l[g].min(al.l[i]);
            k.c[g] = al.c[i];
        }
    }
    k
}

/// `TimeFrameExpand(x, tf)` (default mode): a compressed bar's value shows on its last daily bar; the other daily bars of the period carry the
/// previous compressed bar's value.
fn expand(x: &[f32], k: &Comp, n: usize) -> Vec<f32> {
    let mut out = vec![NULL; n];
    for g in 0..k.start.len() {
        for j in k.start[g]..k.end[g] {
            out[j] = if g > 0 { x[g - 1] } else { NULL };
        }
        out[k.end[g]] = x[g];
    }
    out
}

/// `MA(a, n)` on a compressed array: Null for the first n-1 bars.
fn ma_null(a: &[f32], n: usize) -> Vec<f32> {
    let n = n.max(1);
    let mut out = vec![NULL; a.len()];
    let mut sum = 0f64;
    for i in 0..a.len() {
        sum += a[i] as f64;
        if i >= n { sum -= a[i - n] as f64; }
        if i + 1 >= n { out[i] = (sum / n as f64) as f32; }
    }
    out
}

/// The filters' trend loop (Supertrend of the compressed bars). `init` is the value of `trend[0]` and of the per-bar default before the tests:
/// 1 (index, Ver2 stock), 0 (Ver3 stock) or Null (Ver4 stock). Returns (trend, TrendUp line, TrendDown line) with the AFL's `changeOfTrend` back-fill.
fn swing_f(close: &[f32], mut up: Vec<f32>, mut dn: Vec<f32>, init: f32) -> (Vec<f32>, Vec<f32>, Vec<f32>) {
    let n = close.len();
    let (uf, df) = (up.clone(), dn.clone());
    let mut trend = vec![init; n];
    let mut tup = vec![NULL; n];
    let mut tdn = vec![NULL; n];
    let mut chg = false;
    for i in 1..n {
        let mut t = init;
        if close[i] > up[i - 1] {
            t = 1.0;
            if trend[i - 1] == -1.0 { chg = true; }
        } else if close[i] < dn[i - 1] {
            t = -1.0;
            if trend[i - 1] == 1.0 { chg = true; }
        } else if trend[i - 1] == 1.0 {
            t = 1.0;
            chg = false;
        } else if trend[i - 1] == -1.0 {
            t = -1.0;
            chg = false;
        }
        trend[i] = t;
        let flag1 = t < 0.0 && trend[i - 1] > 0.0;
        let flagh = t > 0.0 && trend[i - 1] < 0.0;
        if t > 0.0 && dn[i] < dn[i - 1] { dn[i] = dn[i - 1]; }
        if t < 0.0 && up[i] > up[i - 1] { up[i] = up[i - 1]; }
        if flag1 { up[i] = uf[i]; }
        if flagh { dn[i] = df[i]; }
        if t == 1.0 {
            tup[i] = dn[i];
            if chg {
                tup[i - 1] = tdn[i - 1];
                chg = false;
            }
        } else if t == -1.0 {
            tdn[i] = up[i];
            if chg {
                tdn[i - 1] = tup[i - 1];
                chg = false;
            }
        }
    }
    (trend, tup, tdn)
}

/// Parameters of one filter leg (stock or index).
#[derive(Clone, Copy)]
struct Leg {
    sma: bool,
    pd: usize,
    fact: f32,
    mapd: usize,
}

/// One leg evaluated on daily bars: the expanded trend, "both ST lines Null", the expanded compressed close and the expanded `Ref(MA, -1)`.
struct SubOut {
    trend: Vec<f32>,
    no_lines: Vec<bool>,
    ic: Vec<f32>,
    pma: Vec<f32>,
}

fn sub_filter(src: &Al, dates: &[i32], tf: u8, point: u8, leg: &Leg, init: f32, ma50_fallback: bool) -> SubOut {
    let n = dates.len();
    let k = compress(dates, src, tf);
    let m = k.start.len();
    let cal: Vec<f32> = (0..m)
        .map(|g| {
            if is_null(k.h[g]) || is_null(k.l[g]) || is_null(k.c[g]) {
                NULL
            } else {
                match point {
                    0 => (k.h[g] + k.l[g]) / 2.0,
                    2 => (k.h[g] + k.l[g] + k.c[g]) / 3.0,
                    _ => k.c[g],
                }
            }
        })
        .collect();
    // ATR(1) of the compressed bars and the AFL's summed satr
    let tr: Vec<f32> = (0..m)
        .map(|g| {
            if g == 0 {
                nsub(k.h[g], k.l[g])
            } else if is_null(k.h[g]) || is_null(k.l[g]) || is_null(k.c[g - 1]) {
                NULL
            } else {
                k.h[g].max(k.c[g - 1]) - k.l[g].min(k.c[g - 1])
            }
        })
        .collect();
    let pd = leg.pd;
    let satr: Vec<f32> = (0..m)
        .map(|g| {
            if pd == 0 || g + 1 < pd { return NULL; }
            let mut s = tr[g];
            for i in 1..pd { s = nadd(s, tr[g - i]); }
            if is_null(s) { NULL } else { s / pd as f32 }
        })
        .collect();
    let up: Vec<f32> = (0..m).map(|g| nadd(cal[g], nmul(leg.fact, satr[g]))).collect();
    let dn: Vec<f32> = (0..m).map(|g| nsub(cal[g], nmul(leg.fact, satr[g]))).collect();
    let (trend, tup, tdn) = swing_f(&k.c, up, dn, init);
    let trend_x = expand(&trend, &k, n);
    let tup_x = expand(&tup, &k, n);
    let tdn_x = expand(&tdn, &k, n);
    let no_lines: Vec<bool> = (0..n).map(|i| is_null(tup_x[i]) && is_null(tdn_x[i])).collect();
    // SMA leg: IC and MA(C, period) (Ver4: Null elements replaced by MA(C, 50))
    let mut mav = ma_null(&k.c, leg.mapd);
    if ma50_fallback {
        let m50 = ma_null(&k.c, 50);
        for g in 0..m {
            if is_null(mav[g]) { mav[g] = m50[g]; }
        }
    }
    let mut prev = vec![NULL; m];
    for g in 1..m {
        prev[g] = mav[g - 1];
    }
    SubOut { trend: trend_x, no_lines, ic: expand(&k.c, &k, n), pma: expand(&prev, &k, n) }
}

#[derive(Clone, Copy)]
struct FltP {
    mode: u8, // 0 NONE, 1 STOCK, 2 INDEX, 3 BOTH
    tf: u8,
    point: u8, // stflt_2: 0 MID, 1 CLOSE, 2 PIVOT (= stcal_of mode)
    stock: Leg,
    idx: Leg,
    prflt: usize,
}

const FLT_INDEX_NAMES: [&str; 3] = ["NSENIFTY", "NIFTY", "NSE500"];

/// (FLTBuy, FLTShort) of one filter configuration on the daily bars of the symbol.
/// `stock_src`: the price arrays of the stock leg (the stock itself; for BOTH the index, because the AFL's stock block runs while the index is still
/// the Foreign symbol); `idx_al`: the index aligned to the symbol (None = index not in the database).
fn flt_compute(dates: &[i32], stock_own: &Al, idx_al: Option<&Al>, fp: &FltP, init_stock: f32, ver4: bool, btrend: Option<&[f32]>) -> (Vec<bool>, Vec<bool>) {
    let n = dates.len();
    let sub_idx = |ix: &Al| sub_filter(ix, dates, fp.tf, fp.point, &fp.idx, 1.0, false);
    match fp.mode {
        1 => {
            let s = sub_filter(stock_own, dates, fp.tf, fp.point, &fp.stock, init_stock, ver4);
            if fp.stock.sma {
                ((0..n).map(|i| s.ic[i] > s.pma[i]).collect(), (0..n).map(|i| s.ic[i] < s.pma[i]).collect())
            } else {
                let t: Vec<f32> = (0..n).map(|i| if ver4 && s.no_lines[i] { btrend.map_or(NULL, |b| b[i]) } else { s.trend[i] }).collect();
                ((0..n).map(|i| t[i] == 1.0).collect(), (0..n).map(|i| t[i] == -1.0).collect())
            }
        }
        2 => {
            let Some(ix) = idx_al else { return (vec![false; n], vec![false; n]) };
            let s = sub_idx(ix);
            if fp.idx.sma {
                ((0..n).map(|i| s.ic[i] > s.pma[i]).collect(), (0..n).map(|i| s.ic[i] < s.pma[i]).collect())
            } else {
                ((0..n).map(|i| s.trend[i] == 1.0).collect(), (0..n).map(|i| s.trend[i] == -1.0).collect())
            }
        }
        3 => {
            let Some(ix) = idx_al else { return (vec![false; n], vec![false; n]) };
            let ii = sub_idx(ix);
            let ss = sub_filter(ix, dates, fp.tf, fp.point, &fp.stock, init_stock, ver4);
            let (sb, sh): (Vec<bool>, Vec<bool>);
            let (ib, ih): (Vec<bool>, Vec<bool>);
            // stock leg (on the index data) and index leg, combined as the AFL's four branches
            match (fp.stock.sma, fp.idx.sma) {
                (false, false) => {
                    sb = (0..n).map(|i| ss.trend[i] == 1.0).collect();
                    sh = (0..n).map(|i| ss.trend[i] == -1.0).collect();
                    ib = (0..n).map(|i| ii.trend[i] == 1.0).collect();
                    ih = (0..n).map(|i| ii.trend[i] == -1.0).collect();
                }
                (false, true) => {
                    // FLTBuy = ST up AND IC > Ref(SFLTMA, -1) (the AFL uses the STOCK moving average for the index close)
                    sb = (0..n).map(|i| ss.trend[i] == 1.0).collect();
                    sh = (0..n).map(|i| ss.trend[i] == -1.0).collect();
                    ib = (0..n).map(|i| ii.ic[i] > ss.pma[i]).collect();
                    ih = (0..n).map(|i| ii.ic[i] < ss.pma[i]).collect();
                }
                (true, true) => {
                    sb = (0..n).map(|i| ss.ic[i] > ss.pma[i]).collect();
                    sh = (0..n).map(|i| ss.ic[i] < ss.pma[i]).collect();
                    ib = (0..n).map(|i| ii.ic[i] > ii.pma[i]).collect();
                    ih = (0..n).map(|i| ii.ic[i] < ii.pma[i]).collect();
                }
                (true, false) => {
                    sb = (0..n).map(|i| ss.ic[i] > ss.pma[i]).collect();
                    sh = (0..n).map(|i| ss.ic[i] < ss.pma[i]).collect();
                    ib = (0..n).map(|i| ii.trend[i] == 1.0).collect();
                    ih = (0..n).map(|i| ii.trend[i] == -1.0).collect();
                }
            }
            ((0..n).map(|i| sb[i] && ib[i]).collect(), (0..n).map(|i| sh[i] || ih[i]).collect())
        }
        _ => (vec![true; n], vec![false; n]),
    }
}

// ---------------------------------------------------------------- the reshuffle core

/// Which filter version a file has (the three are told apart by where the filter is evaluated and by the initial value of the stock trend).
#[derive(Clone, Copy, PartialEq)]
enum FV {
    No,
    /// VER2: both legs inside the rank loop (FLTBuy zeroes the rank value); BUYSIG has no `AND fltbuy`. (stock-trend initial value in the rank loop, in the main part)
    V2(f32, f32),
    /// VER3: only the STOCK filter in the rank loop (initial 0), index in the main part, BUYSIG has `AND fltbuy`
    V3,
    /// VER4: VER3 with Null initial trend, daily ST(10,4) fall-back and MA(C,50) fall-back
    V4,
}

#[derive(Clone, Copy)]
struct Spec {
    /// ATHC files: Hcl = HHV(C, BarCount); 52-week files: HHV(H, DAYSHIGH)
    athc: bool,
    /// % range from the high: ATHDIFF zeroing in the rank, `C > HCLDIFF` in BUYSIG and in the re-buy path
    range: bool,
    /// SELLSIG also `C < hclDIFF`
    range_sell: bool,
    /// the rank value is 0 while the stock's own ST is down (rbtrend1 == -1)
    rank_st: bool,
    /// the rank value is 0 unless the stock is in ATHC buy mode (Rbuysig1)
    rank_rbuy: bool,
    st_buy: bool,
    st_sell: bool,
    st_path2: bool,
    /// the sellsig2 latch also fires on `rankdiff <= 0 AND resufflpd`
    s2_rankdiff: bool,
    flt: FV,
    /// BFactor1 is an Optimize() constant (no engine key)
    bf_const: Option<f32>,
    k_tstart: bool,
    k_bpd: bool,
    k_bf: bool,
    k_days: bool,
    k_diff: bool,
    /// EXITRANK (default, min, max, step)
    ex: (f64, f64, f64, f64),
}

const BASE: Spec = Spec {
    athc: false,
    range: false,
    range_sell: false,
    rank_st: false,
    rank_rbuy: false,
    st_buy: false,
    st_sell: false,
    st_path2: false,
    s2_rankdiff: false,
    flt: FV::No,
    bf_const: None,
    k_tstart: false,
    k_bpd: false,
    k_bf: false,
    k_days: false,
    k_diff: false,
    ex: (10.0, 1.0, 50.0, 1.0),
};

struct Pm {
    dead_yes: bool,
    hard: f32,
    penny: f32,
    turn: bool,
    point1: u8, // stflt (first list) as stcal_of mode
    bpd: usize,
    bf: f32,
    resf: u8,
    entryday: usize,
    days: usize,
    diff: f32,
    rankheld: f32,
    exitrank: f32,
    flt: FltP,
}

fn pm_of(g: G, sp: &Spec) -> Pm {
    let first_list = |x: f64| match x.round() as i32 { 0 => 1u8, 1 => 0u8, _ => 2u8 };
    let leg = |k: &str, pd: &str, f: &str, m: &str| Leg { sma: g(k).round() == 1.0, pd: g(pd).round().max(0.0) as usize, fact: g(f) as f32, mapd: g(m).round().max(1.0) as usize };
    let with_flt = sp.flt != FV::No;
    Pm {
        dead_yes: g("deadstock").round() == 0.0,
        hard: g("hardstop") as f32,
        penny: g("pennyflt") as f32,
        turn: g("turnflt").round() == 1.0,
        point1: first_list(g("stflt")),
        bpd: if sp.k_bpd { g("bpd1").round().max(1.0) as usize } else { 10 },
        bf: if let Some(c) = sp.bf_const { if g("opt_raw").round() == 1.0 { 4.0 } else { c } } else if sp.k_bf { g("bfactor1") as f32 } else { 4.0 },
        resf: g("resf").round() as u8,
        entryday: g("entryday").round().max(0.0) as usize,
        days: if sp.k_days { g("dayshigh").round().max(1.0) as usize } else { 252 },
        diff: if sp.k_diff { g("difffromhigh") as f32 } else { 10.0 },
        rankheld: g("rankheld") as f32,
        exitrank: g("exitrank") as f32,
        flt: if with_flt {
            FltP {
                mode: g("mktflt").round() as u8,
                tf: g("flttf").round() as u8,
                point: g("stflt_2").round() as u8,
                stock: leg("sflt", "sfltpd", "sfltfact", "sfltmapd"),
                idx: leg("iflt", "ifltpd", "ifltfact", "ifltmapd"),
                prflt: (g("prflt").round() as usize).min(2),
            }
        } else {
            FltP { mode: 0, tf: 0, point: 0, stock: Leg { sma: false, pd: 5, fact: 1.0, mapd: 200 }, idx: Leg { sma: false, pd: 5, fact: 1.0, mapd: 200 }, prflt: 0 }
        },
    }
}

/// Everything per symbol that does not depend on the other symbols.
struct Pre {
    hcl: Vec<f32>,
    resf: Vec<bool>,
    flat: Vec<bool>,
    trend: Vec<i8>,
    /// main-part filter (FLTBuy, FLTShort)
    fbuy: Vec<bool>,
    fshort: Vec<bool>,
    /// the rank value (ValuesToSort)
    diff: Vec<f32>,
}

fn prepare(s: &Series, env: &Env, sp: &Spec, pm: &Pm) -> Pre {
    let n = s.bars.len();
    let (c, o, h, l, v) = (s.close(), s.open(), s.high(), s.low(), s.volume());
    let dates: Vec<i32> = s.bars.iter().map(|b| b.date).collect();
    let hcl = if sp.athc { hhv_all(&c) } else { hhv_n(&h, pm.days) };
    let flat = flat30(s);
    let resf = resuffle_flags(&dates, pm.resf, pm.entryday);
    let need_st = sp.rank_st || sp.rank_rbuy || sp.st_buy || sp.st_sell || sp.st_path2;
    let trend = if need_st { st_trend(&stcal_of(pm.point1, &h, &l, &c), &h, &l, &c, pm.bpd, pm.bf) } else { vec![1i8; n] };
    // Rbuysig1: ATHC buy mode of the stock (state set by a new all-time-high close, cleared when the stock's ST turns down)
    let rbuy: Vec<bool> = if sp.rank_rbuy {
        let athc = hhv_all(&c);
        let mut r = vec![false; n];
        for i in 1..n {
            r[i] = if c[i] > athc[i - 1] { true } else if trend[i] == -1 { false } else { r[i - 1] };
        }
        r
    } else {
        vec![true; n]
    };
    // filters
    let (mut fbuy, mut fshort) = (vec![true; n], vec![false; n]);
    let mut rank_flt: Option<Vec<bool>> = None;
    if sp.flt != FV::No {
        let own = own_al(s);
        let idx_series = env.get(FLT_INDEX_NAMES[pm.flt.prflt]);
        let idx_al = idx_series.map(|ix| align(ix, &dates));
        let ver4 = sp.flt == FV::V4;
        let btrend: Option<Vec<f32>> = if ver4 { Some(st_sym(&stcal_of(pm.flt.point, &h, &l, &c), &h, &l, &c, 10, 4.0)) } else { None };
        let fp = &pm.flt;
        // BOTH: the AFL's stock block runs on the index data (see the module notes)
        let stock_src_main: &Al = if fp.mode == 3 { idx_al.as_ref().unwrap_or(&own) } else { &own };
        match sp.flt {
            FV::V2(rank_init, main_init) => {
                let (rb, _) = flt_compute(&dates, stock_src_main, idx_al.as_ref(), fp, rank_init, false, None);
                rank_flt = Some(rb);
                let (mb, ms) = flt_compute(&dates, stock_src_main, idx_al.as_ref(), fp, main_init, false, None);
                fbuy = mb;
                fshort = ms;
            }
            FV::V3 | FV::V4 => {
                let init = if ver4 { NULL } else { 0.0 };
                // rank loop: only the STOCK filter is evaluated; NONE / INDEX leave FLTBuy TRUE; BOTH leaves it undefined (IsNull: rank value 0)
                rank_flt = Some(match fp.mode {
                    1 => flt_compute(&dates, &own, idx_al.as_ref(), fp, init, ver4, btrend.as_deref()).0,
                    3 => vec![false; n],
                    _ => vec![true; n],
                });
                let (mb, ms) = flt_compute(&dates, stock_src_main, idx_al.as_ref(), fp, init, ver4, btrend.as_deref());
                fbuy = mb;
                fshort = ms;
            }
            FV::No => {}
        }
    }
    // the rank value
    let athdiff: Vec<f32> = hcl.iter().map(|&a| a - (a * pm.diff) / 100.0).collect();
    let diff: Vec<f32> = (0..n)
        .map(|i| {
            let dead_next = i + 1 < n && flat[i + 1];
            let turn_fail = pm.turn && !((v[i] * c[i]) > volsig_a(year(dates[i])));
            let zero = dead_next
                || c[i] < pm.penny
                || turn_fail
                || (sp.range && c[i] < athdiff[i])
                || (sp.rank_st && trend[i] == -1)
                || (sp.rank_rbuy && !rbuy[i])
                || rank_flt.as_ref().map_or(false, |f| !f[i]);
            if zero { 0.0 } else { c[i] / hcl[i] }
        })
        .collect();
    let _ = o;
    Pre { hcl, resf, flat, trend, fbuy, fshort, diff }
}

/// `StaticVarGenerateRanks(mode 0)` at the reshuffle dates: rank 1 = highest value, ties by universe order. Returns (rank, value) per symbol bar
/// (NaN where the bar's date is not a reshuffle date of any symbol).
fn rank_at_reshuffles(series: &[Series], pre: &[Pre]) -> Vec<Vec<f32>> {
    let mut dset: HashSet<i32> = HashSet::new();
    for (k, s) in series.iter().enumerate() {
        for (i, b) in s.bars.iter().enumerate() {
            if pre[k].resf[i] { dset.insert(b.date); }
        }
    }
    let mut by_date: HashMap<i32, Vec<(f32, usize, usize)>> = HashMap::new();
    for (k, s) in series.iter().enumerate() {
        for (i, b) in s.bars.iter().enumerate() {
            if dset.contains(&b.date) {
                let x = pre[k].diff[i];
                if x.is_finite() { by_date.entry(b.date).or_default().push((x, k, i)); }
            }
        }
    }
    let mut out: Vec<Vec<f32>> = series.iter().map(|s| vec![f32::NAN; s.bars.len()]).collect();
    for (_, mut list) in by_date {
        list.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap().then(a.1.cmp(&b.1)));
        for (r, &(_, k, i)) in list.iter().enumerate() {
            out[k][i] = (r + 1) as f32;
        }
    }
    out
}

fn symbol_signals(s: &Series, pre: &Pre, rank: &[f32], sp: &Spec, pm: &Pm) -> Signals {
    let n = s.bars.len();
    let (c, o, h, l) = (s.close(), s.open(), s.high(), s.low());
    let v = s.volume();
    let (up, lo) = circuits(s);
    let (dbuy, dsell) = dead_flags(n, pm.dead_yes);
    let hcl = &pre.hcl;
    let hcldiff: Vec<f32> = hcl.iter().map(|&a| a - (a * pm.diff) / 100.0).collect();
    let rankdiff = &pre.diff;
    let flt_in_buy = matches!(sp.flt, FV::V3 | FV::V4);
    let flt_on = sp.flt != FV::No;
    // BUYSIG / SELLSIG
    let (mut buysig, mut sellsig) = (vec![false; n], vec![false; n]);
    for i in 0..n {
        let rk = rank[i];
        let r_ok = pre.resf[i] && rk.is_finite();
        buysig[i] = r_ok
            && rk <= pm.rankheld
            && rankdiff[i] > 0.0
            && (!sp.range || c[i] > hcldiff[i])
            && (!sp.st_buy || pre.trend[i] == 1)
            && (!flt_in_buy || pre.fbuy[i]);
        sellsig[i] = (r_ok && rk > pm.exitrank)
            || (sp.range_sell && c[i] < hcldiff[i])
            || (sp.st_sell && pre.trend[i] == -1)
            || (flt_on && pre.fshort[i] && pre.resf[i]);
    }
    // BUYRANK = ValueWhen(resufflpd, rank)
    let mut buyrank = vec![f32::NAN; n];
    let mut last = f32::NAN;
    for i in 0..n {
        if pre.resf[i] { last = rank[i]; }
        buyrank[i] = last;
    }
    // the two latches
    let (mut buysig2, mut sellsig2) = (vec![false; n], vec![false; n]);
    for i in 1..n {
        if buysig[i] {
            buysig2[i] = true;
            sellsig2[i] = false;
        } else if sellsig[i] || (sp.s2_rankdiff && rankdiff[i] <= 0.0 && pre.resf[i]) {
            sellsig2[i] = true;
            buysig2[i] = false;
        } else {
            buysig2[i] = buysig2[i - 1];
            sellsig2[i] = sellsig2[i - 1];
        }
    }
    let (mut buy, mut sell) = (vec![false; n], vec![false; n]);
    for i in 0..n {
        let turnover = !pm.turn || (v[i] * c[i]) > volsig_a(year(s.bars[i].date));
        let common = !up[i] && !lo[i] && !pre.flat[i] && c[i] >= pm.penny && turnover;
        let p1 = buysig[i] && dbuy[i] && common;
        let p2 = buysig2[i]
            && common
            && (!sp.range || c[i] >= hcldiff[i])
            && (!sp.st_path2 || pre.trend[i] == 1)
            && buyrank[i].is_finite()
            && buyrank[i] <= pm.rankheld;
        buy[i] = p1 || p2;
        sell[i] = (sellsig[i] && !lo[i]) || dsell[i] || (sellsig2[i] && !lo[i]) || pre.flat[i];
    }
    let _ = (o, h, l);
    let px = c.clone();
    let mut sg = Signals::new(buy, sell, px.clone(), px);
    // PositionScore = diff = C / HCL
    sg.score = Some((0..n).map(|i| c[i] / hcl[i]).collect());
    clamp_prices(&mut sg, s);
    sg
}

fn run(series: &[Series], env: &Env, g: G, sp: &Spec) -> Vec<Signals> {
    let pm = pm_of(g, sp);
    let pre: Vec<Pre> = series.iter().map(|s| prepare(s, env, sp, &pm)).collect();
    let rk = rank_at_reshuffles(series, &pre);
    series
        .iter()
        .enumerate()
        .map(|(k, s)| {
            let mut sg = symbol_signals(s, &pre[k], &rk[k], sp, &pm);
            if pm.hard > 0.0 {
                sg.stop = Some(StopSpec { loss_pct: pm.hard, ..StopSpec::default() });
            }
            sg
        })
        .collect()
}

fn schema_of(sp: &Spec) -> Vec<P> {
    let mut v = vec![
        p("deadstock", "DEAD STOCK BACKTEST (0 YES, 1 NO)", 1.0, 0.0, 1.0, 1.0),
        p("hardstop", "Hard Stoploss % (0 = off)", 0.0, 0.0, 50.0, 1.0),
        p("pennyflt", "Penny Stock Price", 0.0, 0.0, 100.0, 1.0),
        p("turnflt", "TURNOVER Filter (0 NO, 1 YES; V*C against the AFL's yearly table)", 0.0, 0.0, 1.0, 1.0),
        p("stflt", "ST CALCULATION POINT (0 CLOSE, 1 MID POINT, 2 PIVOT)", 0.0, 0.0, 2.0, 1.0),
    ];
    if sp.k_tstart {
        v.push(p("tstart", "TRADING START BAR (not used by the AFL's signals)", 0.0, 0.0, 500.0, 1.0));
    }
    if sp.flt != FV::No {
        v.extend([
            p("mktflt", "FILTER TYPE (0 NONE, 1 STOCK, 2 INDEX, 3 BOTH)", 0.0, 0.0, 3.0, 1.0),
            p("flttf", "FILTER TIME FRAME (0 MONTHLY, 1 WEEKLY, 2 DAILY)", 0.0, 0.0, 2.0, 1.0),
            p("stflt_2", "ST CALCULATION POINT of the filters (0 MID POINT, 1 CLOSE, 2 PIVOT)", 0.0, 0.0, 2.0, 1.0),
            p("sflt", "STOCK Filter (0 ST, 1 SMA)", 0.0, 0.0, 1.0, 1.0),
            p("sfltpd", "STOCK ST PERIOD", 5.0, 0.0, 20.0, 1.0),
            p("sfltfact", "STOCK ST FACTOR", 1.0, 0.0, 20.0, 1.0),
            p("sfltmapd", "STOCK SMA PERIOD", 200.0, 1.0, 300.0, 1.0),
            p("prflt", "INDEX (0 NSENIFTY, 1 NIFTY, 2 NSE500)", 0.0, 0.0, 2.0, 1.0),
            p("iflt", "INDEX Filter (0 ST, 1 SMA)", 0.0, 0.0, 1.0, 1.0),
            p("ifltpd", "INDEX ST PERIOD", 5.0, 0.0, 20.0, 1.0),
            p("ifltfact", "INDEX ST FACTOR", 1.0, 0.0, 20.0, 1.0),
            p("ifltmapd", "INDEX SMA PERIOD", 200.0, 1.0, 300.0, 1.0),
        ]);
    }
    if sp.k_bpd {
        v.push(p("bpd1", "ST Period", 10.0, 1.0, 20.0, 1.0));
    }
    if sp.k_bf {
        v.push(p("bfactor1", "ST Factor", 4.0, 1.0, 20.0, 1.0));
    }
    v.push(p("resf", "Resuffle Period (0 Monthly, 1 Weekly, 2 Quarterly, 3 Half Yearly, 4 Yearly)", 0.0, 0.0, 4.0, 1.0));
    v.push(p("entryday", "Resuffle day (Quarterly: eom of this many bars earlier)", 0.0, 0.0, 30.0, 1.0));
    if sp.k_days {
        v.push(p("dayshigh", "NO OF DAYS HIGHEST High", 252.0, 1.0, 1000.0, 1.0));
    }
    if sp.k_diff {
        v.push(p("difffromhigh", "% DIFF FROM HIGH", 10.0, 1.0, 50.0, 1.0));
    }
    v.push(p("rankheld", "RANK FOR ENTRY", 10.0, 1.0, 50.0, 1.0));
    v.push(p("exitrank", "RANK FOR EXIT", sp.ex.0, sp.ex.1, sp.ex.2, sp.ex.3));
    v
}

macro_rules! resh {
    ($spec:ident, $sig:ident, $sch:ident) => {
        fn $sig(series: &[Series], env: &Env, g: G) -> Vec<Signals> {
            run(series, env, g, &$spec)
        }
        fn $sch() -> Vec<P> {
            schema_of(&$spec)
        }
    };
}

// 1: 52W + ST(10,4): rank value 0 while ST is down, sell when ST is down, re-buy when ST is up
const R01: Spec = Spec { rank_st: true, st_sell: true, st_path2: true, k_tstart: true, k_bpd: true, k_bf: true, k_days: true, ..BASE };
// 2: 52W ratio rank, 10 % range, single exit (sell only at the reshuffle)
const R02: Spec = Spec { range: true, s2_rankdiff: true, k_days: true, k_diff: true, ..BASE };
// 3: as 2, plus the daily 10 % stop `C < hclDIFF`; the latches are the plain ones
const R03: Spec = Spec { range: true, range_sell: true, k_days: true, k_diff: true, ..BASE };
// 4-6: 52W with the Ver2 / Ver3 / Ver4 stock / index filters
const R04: Spec = Spec { flt: FV::V2(NULL, 1.0), k_days: true, ..BASE };
const R05: Spec = Spec { flt: FV::V3, k_days: true, ..BASE };
const R06: Spec = Spec { flt: FV::V4, k_days: true, ..BASE };
// 7: plain 52W ratio rank (ST computed but only plotted)
const R07: Spec = Spec { k_bpd: true, k_bf: true, k_days: true, ..BASE };
// 8: ATHC + ST(10,4) (as 1 with ATHC and EXITRANK 10..100 step 5)
const R08: Spec = Spec { athc: true, rank_st: true, st_sell: true, st_path2: true, k_tstart: true, k_bpd: true, k_bf: true, ex: (10.0, 10.0, 100.0, 5.0), ..BASE };
// 9: ATHC, rank only stocks in ATHC buy mode, ST(bpd1, 3.5) buy / sell
const R09: Spec = Spec { athc: true, rank_rbuy: true, st_buy: true, st_sell: true, st_path2: true, k_bpd: true, bf_const: Some(3.5), ..BASE };
// 10: ATHC 10 % range (Optimize(10,3,10)) and daily 10 % stop
const R10: Spec = Spec { athc: true, range: true, range_sell: true, ..BASE };
// 11: ATHC range + ST(10,4): buy needs ST up, sell on ST down (no C < hclDIFF)
const R11: Spec = Spec { athc: true, range: true, st_buy: true, st_sell: true, k_bpd: true, k_bf: true, k_diff: true, ..BASE };
// 12: ATHC range, single exit
const R12: Spec = Spec { athc: true, range: true, s2_rankdiff: true, k_diff: true, ..BASE };
// 13-15: ATHC with the Ver2 / Ver3 / Ver4 filters
const R13: Spec = Spec { athc: true, flt: FV::V2(1.0, NULL), ..BASE };
const R14: Spec = Spec { athc: true, flt: FV::V3, ..BASE };
const R15: Spec = Spec { athc: true, flt: FV::V4, ..BASE };
// 16: plain ATHC ratio rank (ST computed, unused)
const R16: Spec = Spec { athc: true, k_bpd: true, k_bf: true, ..BASE };

resh!(R01, sig01, sch01);
resh!(R02, sig02, sch02);
resh!(R03, sig03, sch03);
resh!(R04, sig04, sch04);
resh!(R05, sig05, sch05);
resh!(R06, sig06, sch06);
resh!(R07, sig07, sch07);
resh!(R08, sig08, sch08);
resh!(R09, sig09, sch09);
resh!(R10, sig10, sch10);
resh!(R11, sig11, sch11);
resh!(R12, sig12, sch12);
resh!(R13, sig13, sch13);
resh!(R14, sig14, sch14);
resh!(R15, sig15, sch15);
resh!(R16, sig16, sch16);

// ---------------------------------------------------------------- long-term highs

fn turnover_oi(s: &Series, on: bool) -> Vec<bool> {
    s.bars.iter().map(|b| !on || b.oi > volsig_b(year(b.date))).collect()
}
fn turnover_vc(s: &Series, on: bool, table_b: bool) -> Vec<bool> {
    s.bars.iter().map(|b| !on || (b.v * b.c) > if table_b { volsig_b(year(b.date)) } else { volsig_a(year(b.date)) }).collect()
}

fn with_hard(mut sg: Signals, hard: f32) -> Signals {
    if hard > 0.0 {
        sg.stop = Some(StopSpec { loss_pct: hard, ..StopSpec::default() });
    }
    sg
}

fn common_lt(extra: Vec<P>, with_hard_key: bool) -> Vec<P> {
    let mut v = vec![p("deadstock", "DEAD STOCK BACKTEST (0 YES, 1 NO)", 1.0, 0.0, 1.0, 1.0)];
    if with_hard_key {
        v.push(p("hardstop", "Hard Stoploss % (0 = off)", 0.0, 0.0, 50.0, 1.0));
    }
    v.push(p("pennyflt", "Penny Stock Price", 0.0, 0.0, 100.0, 1.0));
    v.push(p("turnflt", "TURNOVER Filter (0 NO, 1 YES)", 0.0, 0.0, 1.0, 1.0));
    v.extend(extra);
    v
}

// ---- 3/5 year highest close, %swing exit
fn signals_35(s: &Series, noofdays: usize, exit_pct: f32, dead_yes: bool, penny: f32, turn: bool) -> Signals {
    let n = s.bars.len();
    let (c, o, h, l) = (s.close(), s.open(), s.high(), s.low());
    let hwc = hhv_n(&c, noofdays);
    let trend = pct_swing_trend(&c, exit_pct);
    let mut buysig = vec![false; n];
    for i in 1..n {
        buysig[i] = if c[i] > hwc[i - 1] { true } else if trend[i] == -1 && buysig[i - 1] { false } else { buysig[i - 1] };
    }
    let (up, lo) = circuits(s);
    let (dbuy, dsell) = dead_flags(n, dead_yes);
    let tov = turnover_oi(s, turn);
    let (mut buy, mut sell) = (vec![false; n], vec![false; n]);
    for i in 0..n {
        let new_high = i > 0 && c[i] > hwc[i - 1];
        buy[i] = (new_high || buysig[i]) && dbuy[i] && !up[i] && c[i] > penny && tov[i];
        sell[i] = ((trend[i] == -1 || !buysig[i]) && !lo[i]) || dsell[i];
    }
    let buy = exrem(&buy, &sell);
    let sell = exrem(&sell, &buy);
    let _ = (o, h, l);
    let mut sg = Signals::new(buy, sell, c.clone(), c);
    clamp_prices(&mut sg, s);
    sg
}

fn sig_35(series: &[Series], _env: &Env, g: G) -> Vec<Signals> {
    series
        .iter()
        .map(|s| with_hard(signals_35(s, g("noofdays").round().max(1.0) as usize, g("exitpct") as f32, g("deadstock").round() == 0.0, g("pennyflt") as f32, g("turnflt").round() == 1.0), g("hardstop") as f32))
        .collect()
}
fn sch_35() -> Vec<P> {
    common_lt(
        vec![p("noofdays", "DAYS FOR HIGHEST Close (225 DAYS = 1 YEAR)", 225.0, 1.0, 10000.0, 1.0), p("exitpct", "EXIT %", 10.0, 5.0, 35.0, 1.0)],
        true,
    )
}

// ---- 3/5 year highest close, Supertrend 10,4 exit
fn signals_35_st(s: &Series, days: usize, point: u8, bpd: usize, bf: f32, dead_yes: bool, penny: f32, turn: bool) -> Signals {
    let n = s.bars.len();
    let (c, h, l) = (s.close(), s.high(), s.low());
    let hcl = hhv_n(&c, days);
    let trend = st_trend(&stcal_of(point, &h, &l, &c), &h, &l, &c, bpd, bf);
    let mut buysig1 = vec![false; n];
    for i in 1..n {
        buysig1[i] = if c[i] > hcl[i - 1] { true } else if trend[i] == -1 { false } else { buysig1[i - 1] };
    }
    let (up, lo) = circuits(s);
    let (dbuy, dsell) = dead_flags(n, dead_yes);
    let tov = turnover_vc(s, turn, false);
    let (mut buy, mut sell) = (vec![false; n], vec![false; n]);
    for i in 0..n {
        buy[i] = buysig1[i] && dbuy[i] && !up[i] && tov[i] && c[i] >= penny;
        sell[i] = ((trend[i] == -1 || !buysig1[i]) && !lo[i]) || dsell[i];
    }
    let buy = exrem(&buy, &sell);
    let sell = exrem(&sell, &buy);
    let mut sg = Signals::new(buy, sell, c.clone(), c);
    // `diff = C / HCL` is computed by the AFL but never used (no PositionScore): equal scores, ties by symbol name
    clamp_prices(&mut sg, s);
    sg
}

fn sig_35_st(series: &[Series], _env: &Env, g: G) -> Vec<Signals> {
    let point = match g("stflt").round() as i32 { 0 => 1u8, 1 => 0u8, _ => 2u8 };
    series
        .iter()
        .map(|s| with_hard(signals_35_st(s, g("dayshigh").round().max(1.0) as usize, point, g("bpd1").round().max(1.0) as usize, g("bfactor1") as f32, g("deadstock").round() == 0.0, g("pennyflt") as f32, g("turnflt").round() == 1.0), g("hardstop") as f32))
        .collect()
}
fn sch_35_st() -> Vec<P> {
    common_lt(
        vec![
            p("dayshigh", "NO OF DAYS HIGHEST CLOSE (248 DAYS = 1 YEAR)", 252.0, 225.0, 1525.0, 100.0),
            p("stflt", "ST CALCULATION POINT (0 CLOSE, 1 MID POINT, 2 PIVOT)", 0.0, 0.0, 2.0, 1.0),
            p("tstart", "TRADING START BAR (not used by the AFL's signals)", 0.0, 0.0, 500.0, 1.0),
            p("bpd1", "ST Period", 10.0, 1.0, 20.0, 1.0),
            p("bfactor1", "ST Factor", 4.0, 1.0, 6.0, 0.5),
        ],
        true,
    )
}

// ---- 52-week high buy with %swing exit, profit booking, Bollinger upper band
/// Optimize() constants of the AFL. Default (like every Param default outside its range): the AFL default clamped into the Optimize range =
/// Days 200, EXIT % 10, PROFIT % 30, HSL % 10, band period 15, upper SD 2.5. Hidden engine option `opt_raw` = 1: the AFL defaults as typed
/// (Days 1, EXIT % 10, PROFIT % 25, HSL % 25, band period 100, upper SD 3), in case AmiBroker does not clamp Optimize() defaults.
#[derive(Clone, Copy)]
struct WkOpt {
    days: usize,
    exit: f32,
    profit: f32,
    hsl: f32,
    bbavg: usize,
    ub: f32,
}
const WK_CLAMPED: WkOpt = WkOpt { days: 200, exit: 10.0, profit: 30.0, hsl: 10.0, bbavg: 15, ub: 2.5 };
const WK_RAW: WkOpt = WkOpt { days: 1, exit: 10.0, profit: 25.0, hsl: 25.0, bbavg: 100, ub: 3.0 };

/// `BBandTop(C, n, sd)` / `BBandBot`: MA +- sd * StDev (population); Null (NaN) while the history is short.
fn bband(c: &[f32], n: usize, sd: f32, top: bool) -> Vec<f32> {
    let mut out = vec![f32::NAN; c.len()];
    for i in 0..c.len() {
        if i + 1 < n { continue; }
        let w = &c[i + 1 - n..=i];
        let m: f64 = w.iter().map(|&x| x as f64).sum::<f64>() / n as f64;
        let var: f64 = w.iter().map(|&x| (x as f64 - m).powi(2)).sum::<f64>() / n as f64;
        let b = m + if top { 1.0 } else { -1.0 } * sd as f64 * var.sqrt();
        out[i] = b as f32;
    }
    out
}

struct WkP {
    penny_on: bool,
    penny_price: f32,
    turn_on: bool,
    circuit_on: bool,
    lbt: bool,
    lbpct: f32,
    o: WkOpt,
}

fn signals_wk(s: &Series, p: &WkP) -> Signals {
    let n = s.bars.len();
    let (c, o, h, l) = (s.close(), s.open(), s.high(), s.low());
    let o_ = p.o;
    let hwc = hhv_n(&c, o_.days);
    let trend = pct_swing_trend(&c, o_.exit);
    let ub = bband(&c, o_.bbavg, o_.ub, true);
    let lb: Vec<f32> = if p.lbt { bband(&c, o_.bbavg, p.lbpct, false) } else { vec![1.0; n] };
    let tov = turnover_vc(s, p.turn_on, true);
    let (dbuy, dsell) = dead_flags(n, true);
    // CIRCUIT0..3 (exact float equality, as the AFL)
    let circ = |i: usize| -> (bool, bool, bool, bool) {
        if !p.circuit_on { return (false, false, false, false); }
        let flat = o[i] == c[i] && o[i] == h[i] && o[i] == l[i];
        let prev = if i > 0 { c[i - 1] } else { f32::NAN };
        (flat && c[i] > prev, flat && c[i] < prev, c[i] == h[i], l[i] == c[i])
    };
    let (mut buy, mut sell) = (vec![false; n], vec![false; n]);
    let mut bp = c.clone();
    let mut sp = c.clone();
    let mut pos = vec![false; n];
    let mut trendj = vec![false; n];
    // PB / HSL arrays: elements the loop never assigns stay at their default (0); only assigned on a buy bar / carried while in a position
    let mut pb = vec![0f32; n];
    let mut hsl = vec![0f32; n];
    for i in 1..n {
        if trend[i - 1] == -1 && trend[i] > 0 {
            trendj[i] = true;
        } else if trendj[i - 1] {
            trendj[i] = true;
        } else if trend[i] == -1 {
            trendj[i] = false;
        }
        let (c0, c1, c2, c3) = circ(i);
        let penny_ok = if p.penny_on { c[i] > p.penny_price } else { c[i] > 0.0 };
        if !pos[i - 1] && c[i] > ub[i] && c[i] > hwc[i - 1] && tov[i] && penny_ok && !c0 && !c2 && dbuy[i] && trendj[i] {
            buy[i] = true;
            bp[i] = c[i];
            pb[i] = if o_.profit > 0.0 { bp[i] + (bp[i] * o_.profit) / 100.0 } else { 99_999_999.0 };
            hsl[i] = if o_.hsl > 0.0 { bp[i] - (bp[i] * o_.hsl) / 100.0 } else { 0.0 };
            pos[i] = true;
            trendj[i] = false;
        } else if pos[i - 1] && (trend[i] == -1 || c[i] < lb[i] || l[i] < hsl[i - 1] || h[i] > pb[i - 1] || dsell[i]) && !c1 && !c3 {
            sell[i] = true;
            // the AFL's second SellPrice assignment replaces the first one
            sp[i] = if o[i] < hsl[i - 1] { o[i] - o[i] * 0.0025 } else if l[i] < hsl[i - 1] { hsl[i - 1] } else { c[i] };
            pos[i] = false;
        } else {
            pos[i] = pos[i - 1];
            pb[i] = pb[i - 1];
        }
    }
    let mut sg = Signals::new(buy, sell, bp, sp);
    clamp_prices(&mut sg, s);
    sg
}

fn sig_wk(series: &[Series], _env: &Env, g: G) -> Vec<Signals> {
    // ParamList items ON|OFF: 0 ON, 1 OFF (default 1); LBt ParamToggle OFF|ON (default 0)
    let p = WkP { penny_on: g("penny").round() == 0.0, penny_price: g("pennyprice") as f32, turn_on: g("turnflt").round() == 0.0, circuit_on: g("circuit_filter").round() == 0.0, lbt: g("lbt").round() == 1.0, lbpct: g("lbpct") as f32, o: if g("opt_raw").round() == 1.0 { WK_RAW } else { WK_CLAMPED } };
    series.iter().map(|s| signals_wk(s, &p)).collect()
}
fn sch_wk() -> Vec<P> {
    vec![
        p("penny", "PENNY_FILTER (0 ON, 1 OFF)", 1.0, 0.0, 1.0, 1.0),
        p("pennyprice", "PENNY_PRICE", 1.0, 1.0, 100.0, 0.5),
        p("turnflt", "TURNOVER Filter (0 ON, 1 OFF; V*C against the AFL's yearly table)", 1.0, 0.0, 1.0, 1.0),
        p("circuit_filter", "CIRCUIT (0 ON, 1 OFF)", 1.0, 0.0, 1.0, 1.0),
        p("lbt", "Lower Bollinger (0 OFF, 1 ON)", 0.0, 0.0, 1.0, 1.0),
        p("lbpct", "LOWER BAND SD %", 1.0, 0.5, 10.5, 0.5),
    ]
}

// ---- 250-day highest-close breakout ranked by the gap between the all-time-high close and the 250-day high close
fn signals_250(s: &Series, prd: usize, exit_pct: f32, dead_yes: bool, penny: f32, turn: bool) -> Signals {
    let n = s.bars.len();
    let c = s.close();
    let highest = hhv_all(&c);
    let w52 = hhv_n(&c, prd);
    let trend = pct_swing_trend(&c, exit_pct);
    let ranking: Vec<f32> = (0..n).map(|i| ((highest[i] - w52[i]) / highest[i]) * 100.0).collect();
    let mut buysig = vec![false; n];
    for i in 1..n {
        buysig[i] = if c[i] > w52[i - 1] { true } else if trend[i] == -1 && buysig[i - 1] { false } else { buysig[i - 1] };
    }
    let tov = turnover_oi(s, turn);
    let (dbuy, dsell) = dead_flags(n, dead_yes);
    let uc: Vec<bool> = s.bars.iter().map(|b| b.h == b.c).collect();
    let lc: Vec<bool> = s.bars.iter().map(|b| b.l == b.c).collect();
    let bcf: Vec<bool> = (0..n).map(|i| !uc[i] && tov[i] && c[i] > penny && dbuy[i]).collect();
    let (mut buy, mut sell) = (vec![false; n], vec![false; n]);
    let (mut position, mut skipbuy, mut skipsell) = (false, false, false);
    for i in prd..n {
        if !position && (c[i] > w52[i - 1] || buysig[i]) && !skipbuy {
            if bcf[i] {
                buy[i] = true;
                position = true;
            } else {
                skipbuy = true;
            }
        } else if skipbuy && bcf[i] {
            buy[i] = true;
            position = true;
            skipbuy = false;
        } else if (position && !skipsell && trend[i] == -1) || (position && dsell[i]) {
            if !lc[i] || dsell[i] {
                sell[i] = true;
                position = false;
            } else {
                skipsell = true;
            }
        } else if (skipsell && !lc[i]) || dsell[i] {
            sell[i] = true;
            position = false;
            skipsell = false;
        }
    }
    let mut sg = Signals::new(buy, sell, c.clone(), c);
    sg.score = Some((0..n).map(|i| 1000.0 - ranking[i]).collect());
    clamp_prices(&mut sg, s);
    sg
}

fn sig_250(series: &[Series], _env: &Env, g: G) -> Vec<Signals> {
    series.iter().map(|s| signals_250(s, g("prd").round().max(1.0) as usize, g("exitpct") as f32, g("deadstock").round() == 0.0, g("pennyflt") as f32, g("turnflt").round() == 1.0)).collect()
}
fn sch_250() -> Vec<P> {
    common_lt(vec![p("exitpct", "EXIT %", 10.0, 1.0, 20.0, 1.0), p("prd", "250 Days High Close", 250.0, 1.0, 400.0, 1.0)], false)
}

// ---------------------------------------------------------------- the 20 variants

pub fn variants() -> Vec<VariantDef> {
    let d = Data::Cash22 { watchlist: "NSE 500 C&P.tls" };
    vec![
        VariantDef { id: "ytath_52_week_high_reshuffling_close_52w", data: d, schema: sch01, signals: sig01 },
        VariantDef { id: "ytath_52_week_high_reshuffling_ratio_rank", data: d, schema: sch02, signals: sig02 },
        VariantDef { id: "ytath_52_week_high_reshuffling_ratio_rank_2", data: d, schema: sch03, signals: sig03 },
        VariantDef { id: "ytath_52_week_high_reshuffling_ratio_rank_3", data: d, schema: sch04, signals: sig04 },
        VariantDef { id: "ytath_52_week_high_reshuffling_ratio_rank_4", data: d, schema: sch05, signals: sig05 },
        VariantDef { id: "ytath_52_week_high_reshuffling_ratio_rank_5", data: d, schema: sch06, signals: sig06 },
        VariantDef { id: "ytath_52_week_high_reshuffling_ranking_on", data: d, schema: sch07, signals: sig07 },
        VariantDef { id: "ytath_reshuffling_ranking_on_close_athc", data: d, schema: sch08, signals: sig08 },
        VariantDef { id: "ytath_reshuffling_rank_only_stocks_in_athc", data: d, schema: sch09, signals: sig09 },
        VariantDef { id: "ytath_reshuffling_on_close_athc_ratio_10pct", data: d, schema: sch10, signals: sig10 },
        VariantDef { id: "ytath_reshuffling_on_close_athc_ratio_10pct_2", data: d, schema: sch11, signals: sig11 },
        VariantDef { id: "ytath_reshuffling_on_close_athc_ratio_10pct_3", data: d, schema: sch12, signals: sig12 },
        VariantDef { id: "ytath_reshuffling_on_close_athc_ratio_with", data: d, schema: sch13, signals: sig13 },
        VariantDef { id: "ytath_reshuffling_on_close_athc_ratio_with_2", data: d, schema: sch14, signals: sig14 },
        VariantDef { id: "ytath_reshuffling_on_close_athc_ratio_with_3", data: d, schema: sch15, signals: sig15 },
        VariantDef { id: "ytath_reshuffling_on_close_athc_ratio", data: d, schema: sch16, signals: sig16 },
        VariantDef { id: "ytath_3_5_year_highest_close_buy_with", data: d, schema: sch_35, signals: sig_35 },
        VariantDef { id: "ytath_3_5_year_highest_close_buy_with_2", data: d, schema: sch_35_st, signals: sig_35_st },
        VariantDef { id: "ytath_52_week_high_buy_with_pctswing_exit", data: d, schema: sch_wk, signals: sig_wk },
        VariantDef { id: "ytath_250_day_highest_close_breakout_ranked", data: d, schema: sch_250, signals: sig_250 },
    ]
}

// ---------------------------------------------------------------- tests

#[cfg(test)]
mod tests {
    use super::*;

    fn bar(date: i32, o: f32, h: f32, l: f32, c: f32) -> Bar {
        Bar { date, c, o, h, l, v: 1.0e6, oi: 1.0e9, aux1: 1.0, aux2: 0.0 }
    }

    /// consecutive calendar days from 2020-01-01 skipping weekends
    fn dates(n: usize) -> Vec<i32> {
        let mut out = vec![];
        let (mut y, mut m, mut d) = (2020, 1, 1);
        while out.len() < n {
            if dow(y * 10000 + m * 100 + d) != 0 && dow(y * 10000 + m * 100 + d) != 6 { out.push(y * 10000 + m * 100 + d); }
            d += 1;
            let dim = [31, if y % 4 == 0 { 29 } else { 28 }, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31][(m - 1) as usize];
            if d > dim { d = 1; m += 1; if m > 12 { m = 1; y += 1; } }
        }
        out
    }

    fn series_from(name: &str, closes: &[f32]) -> Series {
        let ds = dates(closes.len());
        Series { name: name.into(), bars: closes.iter().enumerate().map(|(i, &c)| bar(ds[i], c, c + 0.5, c - 0.5, c)).collect() }
    }

    #[test]
    fn day_of_week_and_month_end() {
        assert_eq!(dow(20220331), 4); // Thursday
        assert_eq!(dow(20200101), 3); // Wednesday
        let ds = vec![20200129, 20200130, 20200131, 20200203, 20200204];
        let m = resuffle_flags(&ds, 0, 0);
        assert_eq!(m, vec![false, false, true, false, false], "Monthly: last bar of the month");
        let w = resuffle_flags(&[20200130, 20200131, 20200203, 20200207, 20200210], 1, 0);
        assert_eq!(w, vec![false, true, false, true, false], "Weekly: Friday (next bar's weekday is smaller)");
        let q = resuffle_flags(&[20200327, 20200331, 20200401], 2, 0);
        assert_eq!(q, vec![false, true, false], "Quarterly: end of March");
        let y = resuffle_flags(&[20201230, 20201231, 20210104], 4, 0);
        assert_eq!(y, vec![false, true, false]);
    }

    #[test]
    fn eom_needs_the_next_day_number_to_be_small() {
        // the next bar is the 8th: the AFL's eom chain (next day 1..5) does not see a month end
        assert_eq!(eom_flags(&[20200131, 20200208]), vec![false, false]);
        assert_eq!(eom_flags(&[20200131, 20200203]), vec![true, false]);
    }

    #[test]
    fn hhv_window_and_cumulative_max() {
        assert_eq!(hhv_n(&[1.0, 5.0, 2.0, 3.0, 1.0], 2), vec![1.0, 5.0, 5.0, 3.0, 3.0]);
        assert_eq!(hhv_all(&[1.0, 5.0, 2.0, 6.0]), vec![1.0, 5.0, 5.0, 6.0]);
    }

    #[test]
    fn rank_one_is_the_highest_value_and_ties_follow_the_universe_order() {
        let a = series_from("A", &[10.0; 3]);
        let b = series_from("B", &[10.0; 3]);
        let mk = |v: f32| Pre { hcl: vec![1.0; 3], resf: vec![false, false, true], flat: vec![false; 3], trend: vec![1; 3], fbuy: vec![true; 3], fshort: vec![false; 3], diff: vec![v; 3] };
        let pre = vec![mk(0.9), mk(0.95)];
        let r = rank_at_reshuffles(&[a, b], &pre);
        assert_eq!(r[1][2], 1.0);
        assert_eq!(r[0][2], 2.0);
        assert!(r[0][0].is_nan(), "ranks only at reshuffle dates");
    }

    #[test]
    fn reshuffle_buys_the_top_ranks_and_the_latch_rebuys_until_sold() {
        // a rising stock: month ends are reshuffle bars, rank 1 of 1 -> buy; the latch keeps Buy true afterwards
        let closes: Vec<f32> = (0..80).map(|i| 100.0 + i as f32).collect();
        let s = series_from("A", &closes);
        let pm = Pm { dead_yes: false, hard: 0.0, penny: 0.0, turn: false, point1: 1, bpd: 10, bf: 4.0, resf: 0, entryday: 0, days: 252, diff: 10.0, rankheld: 10.0, exitrank: 10.0, flt: pm_none() };
        let pre = prepare(&s, &Env::empty(), &BASE, &pm);
        let rk = rank_at_reshuffles(std::slice::from_ref(&s), std::slice::from_ref(&pre));
        let sg = symbol_signals(&s, &pre, &rk[0], &BASE, &pm);
        let first = (0..80).find(|&i| sg.buy[i]).expect("a buy");
        assert!(pre.resf[first], "the first buy is on a reshuffle bar");
        assert!(sg.buy[first + 1], "raw mode: the buysig2 latch keeps signalling");
        assert!(sg.sell.iter().all(|&x| !x), "no sell while the stock stays rank 1");
    }

    fn pm_none() -> FltP {
        FltP { mode: 0, tf: 0, point: 0, stock: Leg { sma: false, pd: 5, fact: 1.0, mapd: 200 }, idx: Leg { sma: false, pd: 5, fact: 1.0, mapd: 200 }, prflt: 0 }
    }

    #[test]
    fn rank_beyond_exitrank_sells_on_the_reshuffle_bar() {
        // two stocks, EXITRANK 1: the weaker one (ratio < the other's) is rank 2 at the reshuffle and is sold
        let mut c1: Vec<f32> = (0..70).map(|i| 100.0 + i as f32).collect();
        let c2: Vec<f32> = (0..70).map(|i| 100.0 + 0.2 * i as f32).collect();
        c1[69] = 169.0;
        let s1 = series_from("A", &c1);
        let s2 = series_from("B", &c2);
        let pm = Pm { dead_yes: false, hard: 0.0, penny: 0.0, turn: false, point1: 1, bpd: 10, bf: 4.0, resf: 0, entryday: 0, days: 252, diff: 10.0, rankheld: 2.0, exitrank: 1.0, flt: pm_none() };
        let series = vec![s1, s2];
        let pre: Vec<Pre> = series.iter().map(|s| prepare(s, &Env::empty(), &BASE, &pm)).collect();
        let rk = rank_at_reshuffles(&series, &pre);
        let sb = symbol_signals(&series[1], &pre[1], &rk[1], &BASE, &pm);
        // B is bought at a month end (rank 2 <= RANKHELD 2) and sold at the next one (rank 2 > EXITRANK 1)
        let b0 = (0..70).find(|&i| sb.buy[i]).expect("B bought");
        let s0 = (b0 + 1..70).find(|&i| sb.sell[i]).expect("B sold");
        assert!(pre[1].resf[s0] || sb.sell[s0]);
    }

    #[test]
    fn supertrend_flips_down_on_a_collapse() {
        let mut c: Vec<f32> = (0..40).map(|i| 100.0 + i as f32).collect();
        for i in 30..40 { c[i] = 60.0; }
        let h: Vec<f32> = c.iter().map(|x| x + 1.0).collect();
        let l: Vec<f32> = c.iter().map(|x| x - 1.0).collect();
        let t = st_trend(&c, &h, &l, &c, 10, 4.0);
        assert_eq!(t[29], 1);
        assert_eq!(t[39], -1);
    }

    #[test]
    fn filter_trend_starts_at_the_chosen_value() {
        let close = vec![10.0f32; 4];
        let (t0, _, _) = swing_f(&close, vec![NULL; 4], vec![NULL; 4], 0.0);
        // Null bands: Close > Up[i-1] (-1e10) is true -> trend 1 from bar 1; bar 0 keeps the initial value
        assert_eq!(t0[0], 0.0);
        assert_eq!(t0[1], 1.0);
        let (tn, _, _) = swing_f(&close, vec![20.0; 4], vec![5.0; 4], NULL);
        assert!(is_null(tn[1]) && is_null(tn[3]), "inside the bands a Null trend stays Null");
    }

    #[test]
    fn long_term_35_buys_a_new_n_day_high_and_sells_on_the_swing() {
        let mut c: Vec<f32> = (0..30).map(|i| 100.0 + i as f32).collect();
        c.extend([80.0, 79.0, 78.0]);
        let s = series_from("A", &c);
        let sg = signals_35(&s, 10, 10.0, false, 0.0, false);
        assert!(sg.buy.iter().filter(|&&b| b).count() == 1, "ExRem: one entry");
        let sell = (1..c.len()).find(|&i| sg.sell[i]).expect("a sell");
        assert!(sell >= 30, "the sell comes with the collapse");
    }

    #[test]
    fn week_high_buy_loop_needs_a_swing_flip_and_a_bollinger_break() {
        // flat at 100, a one-bar fall to 70 (swing down), a 21 % jump (swing flips up: TRENDJ), a flat base at 85, then a break to 102
        let mut c: Vec<f32> = vec![100.0; 210];
        c.extend([70.0; 6]);
        c.push(85.0);
        c.extend([85.0; 19]);
        c.push(102.0);
        c.push(103.0); // the last bar never buys (deadstockbuysig)
        let s = series_from("A", &c);
        let p = WkP { penny_on: false, penny_price: 1.0, turn_on: false, circuit_on: false, lbt: false, lbpct: 1.0, o: WK_CLAMPED };
        let sg = signals_wk(&s, &p);
        assert!(sg.buy.iter().any(|&b| b), "a buy after the flip");
        let b = (0..c.len()).find(|&i| sg.buy[i]).unwrap();
        assert_eq!(b, c.len() - 2, "the break of the band and of the 200-bar high after the flip");
    }

    #[test]
    fn breakout_250_state_machine_buys_once_and_sells_on_the_swing() {
        let mut c: Vec<f32> = (0..300).map(|i| 100.0 + 0.1 * i as f32).collect();
        c.extend([60.0, 59.0]);
        let s = series_from("A", &c);
        let sg = signals_250(&s, 250, 10.0, false, 0.0, false);
        assert_eq!(sg.buy.iter().filter(|&&b| b).count(), 1);
        assert!(sg.sell[300]);
        assert!(sg.score.as_ref().unwrap()[290] > 900.0);
    }

    #[test]
    fn all_twenty_ids_are_registered_with_distinct_names() {
        let v = variants();
        assert_eq!(v.len(), 20);
        let ids: HashSet<&str> = v.iter().map(|x| x.id).collect();
        assert_eq!(ids.len(), 20);
        for x in &v {
            let keys: HashSet<&str> = (x.schema)().iter().map(|k| k.key).collect();
            assert_eq!(keys.len(), (x.schema)().len(), "{}: duplicate key", x.id);
        }
    }
}
