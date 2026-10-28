//! YT Strategies · % Swing (stocks): the team's AmiBroker AFLs in "Old Topics/% Swing" that trade individual stocks,
//! each run here on one stock's own daily bars. Coded from the AFLs as written, with the AFLs' own default settings;
//! there is no book PDF.
//!
//! AmiBroker behaviour follows `books::ytsw` (fills at the signal bar's close, ExRem resetting on the same bar, exit
//! then entry on one bar, expandLast for higher time frames, ATR(1) Null on the first bar, leading Nulls outside
//! comparisons). Further points:
//! - the AFLs that write Buy / Sell in a bar loop with their own SellPrice are backtested as `books::ytroc` does it
//!   (loop over the whole history, trades opened only in the test window, loop prices kept inside the bar's range);
//! - `ApplyStop(stopTypeLoss, stopModePercent, ..)` with ActivateStopsImmediately off is checked from the bar after
//!   entry, at the stop level or the open when the open gaps through it (`books::ytd7::fills`);
//! - a market index read with SetForeign / Foreign (NSE500 by default in the trailing-SL flipper) is the market
//!   benchmark NIFTY 50 aligned to the stock's dates (`BENCH.*`), carried forward over the stock's dates the index
//!   lacks; before the index's first bar the filter is off (no buys);
//! - the turnover filters (OpenInt, where the team stored turnover, or V x C) are close x volume here;
//! - `Prec(x, 2)` truncates to two decimals; UC = high equals close, LC = low equals close.
#![allow(dead_code)]
use crate::books::ytd7::fills;
use crate::books::ytroc::{backtest as px_backtest, Px};
use crate::books::ytsw::{backtest, close_swing, compress, cross_up, expand_last, sma, sma_filter, st_filter, supertrend, Bars, Sig};
use crate::books::ytwh::{bound, prec2};
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "yt_sws";
pub const FLIP20: &str = "ytsws_flipper_20";
pub const FLIP_TRAIL: &str = "ytsws_flipper_index_trailing_sl";
pub const DP5: &str = "ytsws_5dp_long";
pub const SINGLE: &str = "ytsws_single_trade";
pub const IDS: [&str; 4] = [FLIP20, FLIP_TRAIL, DP5, SINGLE];

const GROUP: &str = "YT Strategies · % Swing (stocks)";
const BENCH_HLC: &[&str] = &["BENCH.High", "BENCH.Low", "BENCH.Close"];

fn def(id: &'static str, name: &'static str, rules: &'static [&'static str], notes: &'static str, volume: bool, series: &'static [&'static str]) -> StrategyDef {
    StrategyDef { id, name, group: GROUP, session: 0, dir: "Long", status: Status::Live, needs_volume: volume, needs_vix: false, needs_series: series, book: BOOK, rules, notes, claim: None }
}

pub fn catalogue() -> Vec<StrategyDef> {
    vec![
        def(FLIP20, "20% Flipper with Monthly ST Filter (stock)", &[
            "F|20% swing on the stock: up when the close rises more than 20% above the trailing low band (low x 1.2), down when it falls more than 20% below the trailing high band (high x 0.8); bands ratchet and reset on each flip",
            "F|Filter on the stock's own bars: monthly Supertrend (period 5, factor 1, mid point; default) or SMA (200), weekly or daily as options",
            "E|Buy at the close when the filter is up and the swing is up; not on the first bar, an upper-circuit bar (open = high = close at two decimals) or a bar closing at its high",
            "X|Sell at the close when the swing turns down or the filter turns down, unless the bar closes at its low",
        ], "20% flipper 2.afl (% Swing / %swing Buy and % Swing Exit / 20% flipper), coded as the AFL is written. Runs on one stock at a time; the team ran it over NSE500 cash-and-positional stocks (workbook NSE500C&P_20% flipper 2); ranking and portfolio limits (MaxOpenPositions 20, sizing) are not modelled. The AFL calls its filter a market filter but computes it on the stock's own bars (no Foreign). Turnover filter (OpenInt, default off) is close x volume here; penny price default 0; dead-stock default NO (no buy on the first bar, a Sell on the first bar).", false, &[]),
        def(FLIP_TRAIL, "12% Flipper with Index ST Filter and 7% Trailing SL (stock)", &[
            "F|Index filter: daily Supertrend of the index (period 20, factor 1, mid point; default) or the index close above its previous 200-day SMA",
            "F|Flip swing 12% on the stock (bands from low x 1.12 and high x 0.88) and a trailing swing 7% (low x 1.07, high x 0.93)",
            "E|Buy at the close when the index filter is up and the 12% swing is up; not on the first bar or a bar closing at its high",
            "X|While the index filter is up: sell when the 12% swing turns down; while it is down: sell when the 7% swing turns down; never on a bar closing at its low",
        ], "20_ flipper WITH ST MA  DAILY FLT AND TRALING SL WITH PARAM.afl (% Swing / %swing Buy and % Swing Exit / 20% flipper / 20% flipper), coded as the AFL is written with its Optimize defaults (flip 12%, trailing SL 7%). The AFL's index (SetForeign, default NSE500) is replaced by NIFTY 50, the market benchmark, aligned to the stock's dates. Runs on one stock at a time; the team ran it over a stock universe (its optimisation workbooks); ranking and portfolio limits (MaxOpenPositions 20, sizing) are not modelled. Unlike 20% flipper 2 it has no Prec upper-circuit test. Turnover filter (OpenInt, default off) is close x volume here; penny price default 0.", false, BENCH_HLC),
        def(DP5, "5 Days Before Month-End Long with Profit Target and Hard SL (stock)", &[
            "F|Month end: the bar whose next bar is in another month; buy day = 4 bars before it (Ref(EOM, 4)); sell day = 5 bars after a month end",
            "F|RSI(14) above 30 and ADX(8) above 20; candle colour any (default); optional filter: all-time-high breakout with a 20% swing, monthly ST (5, 1, mid point) or bull / bear % swing (44% / 23%); default off",
            "E|Buy at the close on the buy day when those hold, not on the first bar or a bar closing at its high",
            "X|Exit at the profit target (entry + 3%) when the high reaches it, at the hard SL (entry - 1%) when the low reaches it (at the open less 0.25% if the open gapped through), or at the close on the sell day or a filter sell; a bar closing at its low postpones the exit to the next bar not closing at its low",
        ], "5 DP Long Only (With & Without Filter and HSL & PT)+G R CND+ADX+RSI+RSI RANK(Bear Entry in %Swing).afl (% Swing / Bear Entry in %Swing / Checking and afl), coded as the AFL is written. Runs on one stock at a time; the team ran it over a stock universe; ranking and portfolio limits (MaxOpenPositions 20, sizing) are not modelled. The buy day reads 4 bars ahead (look-ahead as written) and the month end the next bar; the data's last bar is not a month end. The ADX period's default 8 lies below its own Param minimum 12; the default is used as written. RSI is AmiBroker's Wilder RSI; ADX is the Wilder ADX seeded with simple averages. Gap-up profit exits fill at the open less the 0.25% buffer (as written), kept inside the bar's range. Turnover (OpenInt, default off) is close x volume; penny price default 0; dead-stock default NO.", false, &[]),
        def(SINGLE, "Single Trade % Swing Long (stock)", &[
            "F|Exit swing 5% on closes; filter swing on closes starting from 8% bands whose flips reset at 5% (as written)",
            "F|Buy flag: on when the close crosses above the previous bar's filter-swing down line while the exit swing is up; off when either swing is down",
            "E|Buy at the close on that cross or while the flag is on, when the exit swing is up, the close is above 50, turnover (close x volume) exceeds the AFL's yearly threshold, and not on the first bar or an upper-circuit bar",
            "X|Sell at the close when either swing is down or the flag is off (not on a lower-circuit bar); a 5% loss stop (ApplyStop) from the bar after entry",
        ], "SINGLE TRADE WITH %SWING  long only WITH SQN N RISK OF RUIN.afl (% Swing / %swing Buy and % Swing Exit / To be checked), coded as the AFL is written; there is no team workbook for it. Runs on one stock at a time; the team's settings are for a stock universe; ranking and portfolio limits (MaxOpenPositions 20, sizing) and the SQN / R-multiple / risk-of-ruin report metrics are not modelled (they do not change trades). The turnover filter is ON by default (V x C above 30,000 before 2000, 50,000 in 2000-04, 1.8 M in 2005-09, 2.6 M in 2010-14, 6 M in 2015-19, 13 M from 2020); a bar without volume fails it. The filter swing's flip resets use the exit % (5), not the filter % (8), as written. The down line is Null on up-trend bars, so the cross also fires as it ends (AmiBroker Null handling).", true, &[]),
    ]
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    let penny = |d: f64| p("penny", "Penny stock price (buy only above it)", d, 0.0, 100.0, 1.0);
    let turnover = |d: f64| p("turnover", "Turnover filter (0 off, 1 on: close x volume above the AFL's yearly threshold)", d, 0.0, 1.0, 1.0);
    let dead = |d: f64| p("dead_stock", "Dead stock backtest (0 YES: buys allowed from the first bar, the position is sold on the run's last bar; 1 NO, the AFL default: no buy on the first bar)", d, 0.0, 1.0, 1.0);
    Some(match id {
        FLIP20 => vec![
            p("flt", "Filter (0 Supertrend, 1 SMA)", 0.0, 0.0, 1.0, 1.0),
            p("flt_tf", "Filter timeframe (0 monthly, 1 weekly, 2 daily)", 0.0, 0.0, 2.0, 1.0),
            p("flt_period", "Filter ST period", 5.0, 5.0, 20.0, 5.0),
            p("flt_factor", "Filter ST factor", 1.0, 1.0, 12.0, 1.0),
            p("flt_point", "Filter ST calculation point (0 mid point, 1 close, 2 pivot, 3 average of mid point, pivot and close, 4 3-period MA, 5 5-period MA)", 0.0, 0.0, 5.0, 1.0),
            p("sma", "Filter SMA period", 200.0, 10.0, 500.0, 1.0),
            penny(0.0),
            turnover(0.0),
            dead(1.0),
        ],
        FLIP_TRAIL => vec![
            p("flip_pct", "Flip %", 12.0, 12.0, 30.0, 1.0),
            p("trail_pct", "Trailing SL %", 7.0, 7.0, 15.0, 1.0),
            p("flt", "Index filter (0 Supertrend, 1 SMA)", 0.0, 0.0, 1.0, 1.0),
            p("flt_point", "Index ST calculation point (0 mid point, 1 close, 2 pivot)", 0.0, 0.0, 2.0, 1.0),
            p("flt_period", "Index ST period", 20.0, 1.0, 20.0, 1.0),
            p("flt_factor", "Index ST factor", 1.0, 0.5, 20.0, 0.5),
            p("sma", "Index SMA period", 200.0, 1.0, 300.0, 1.0),
            penny(0.0),
            turnover(0.0),
            dead(1.0),
        ],
        DP5 => vec![
            p("entry_day", "Entry day (buy Ref(EOM, entry day - 1))", 5.0, 2.0, 6.0, 1.0),
            p("exit_day", "Exit day (sell this many bars after a month end)", 5.0, 2.0, 6.0, 1.0),
            p("profit_pct", "Profit booking % (0 off)", 3.0, 0.0, 30.0, 3.0),
            p("hard_sl_pct", "Hard SL % (0 off)", 1.0, 0.0, 4.0, 1.0),
            p("buffer_pct", "Selling buffer % (gap fills)", 0.25, 0.0, 10.0, 0.05),
            p("candle", "Candle colour (0 green, 1 red, 2 any)", 2.0, 0.0, 2.0, 1.0),
            p("rsi_period", "RSI period", 14.0, 4.0, 24.0, 4.0),
            p("rsi_value", "RSI above", 30.0, 20.0, 55.0, 5.0),
            p("adx_period", "ADX period (AFL default 8, below its Param minimum)", 8.0, 2.0, 21.0, 1.0),
            p("adx_value", "ADX above", 20.0, 10.0, 30.0, 5.0),
            p("flt", "Filter (0 off, 1 ATHC breakout with % swing exit, 2 ST, 3 bull / bear % swing)", 0.0, 0.0, 3.0, 1.0),
            p("athc_exit_pct", "ATHC swing exit %", 20.0, 1.0, 22.0, 1.0),
            p("flt_tf", "ST filter timeframe (0 monthly, 1 weekly, 2 daily)", 0.0, 0.0, 2.0, 1.0),
            p("flt_period", "ST filter period", 5.0, 1.0, 25.0, 1.0),
            p("flt_factor", "ST filter factor", 1.0, 1.0, 5.0, 1.0),
            p("flt_point", "ST calculation point (0 mid point, 1 close, 2 pivot)", 0.0, 0.0, 2.0, 1.0),
            p("bear_pct", "Swing % for bear market", 23.0, 4.0, 24.0, 1.0),
            p("bull_pct", "Swing % for bull market", 44.0, 4.0, 50.0, 1.0),
            penny(0.0),
            turnover(0.0),
            dead(1.0),
        ],
        SINGLE => vec![
            p("swing_flt", "Swing % for the filter", 8.0, 5.0, 25.0, 1.0),
            p("exit_pct", "Swing % for exit", 5.0, 1.0, 50.0, 1.0),
            p("hard_stop", "Hard stop loss % (ApplyStop, 0 off)", 5.0, 0.0, 50.0, 1.0),
            penny(50.0),
            turnover(1.0),
            dead(1.0),
        ],
        _ => return None,
    })
}

// ---------------------------------------------------------------- shared pieces

fn year(x: &Ctx, i: usize) -> i64 {
    x.r[i].date.get(0..4).and_then(|s| s.parse().ok()).unwrap_or(0)
}

/// The 16-step turnover threshold of the flipper / 5 DP / 52-week AFLs.
pub(crate) fn volsig_fine(y: i64) -> f64 {
    match y {
        ..=1999 => 30_000.0,
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

/// The 6-step turnover threshold of the Negative Expiry / single-trade AFLs.
fn volsig_coarse(y: i64) -> f64 {
    match y {
        ..=1999 => 30_000.0,
        2000..=2004 => 50_000.0,
        2005..=2009 => 1_800_000.0,
        2010..=2014 => 2_600_000.0,
        2015..=2019 => 6_000_000.0,
        _ => 13_000_000.0,
    }
}

/// Turnover filter: close x volume above the threshold (a bar without volume fails); `on` false = always true.
pub(crate) fn turnover_ok(x: &Ctx, on: bool, fine: bool) -> Vec<bool> {
    (0..x.c.len())
        .map(|i| {
            if !on {
                return true;
            }
            let t = if fine { volsig_fine(year(x, i)) } else { volsig_coarse(year(x, i)) };
            x.r[i].volume.map_or(false, |v| v * x.c[i] > t)
        })
        .collect()
}

/// A benchmark series carried forward over missing dates (leading NaNs stay NaN).
fn bench(x: &Ctx, name: &str) -> Option<Vec<f64>> {
    let v = x.series(name)?;
    let mut out = v.to_vec();
    for i in 1..out.len() {
        if !out[i].is_finite() {
            out[i] = out[i - 1];
        }
    }
    Some(out)
}

/// The flipper's filter ST point on period bars: 0 mid point, 1 close, 2 pivot, 3 (MP + pivot + C) / 3, 4 MA(C, 3),
/// 5 MA(C, 5) (NaN before the average exists).
fn flt_point(b: &Bars, kind: i64) -> Vec<f64> {
    let m3 = sma(&b.c, 3);
    let m5 = sma(&b.c, 5);
    (0..b.c.len())
        .map(|i| {
            let (h, l, c) = (b.h[i], b.l[i], b.c[i]);
            match kind {
                1 => c,
                2 => (h + l + c) / 3.0,
                3 => ((h + l) / 2.0 + (h + l + c) / 3.0 + c) / 3.0,
                4 => m3[i].unwrap_or(f64::NAN),
                5 => m5[i].unwrap_or(f64::NAN),
                _ => (h + l) / 2.0,
            }
        })
        .collect()
}

/// A swing on closes whose bands start at `start_pct` and reset at `reset_pct` on a flip; returns (trend, down line =
/// the upper band on down-trend bars and, on the bar before a flip down, that bar's up-trend line).
pub(crate) fn swing_lines(c: &[f64], start_pct: f64, reset_pct: f64) -> (Vec<i8>, Vec<Option<f64>>) {
    let n = c.len();
    let mut up: Vec<f64> = c.iter().map(|v| v * (1.0 + start_pct / 100.0)).collect();
    let mut dn: Vec<f64> = c.iter().map(|v| v * (1.0 - start_pct / 100.0)).collect();
    let mut tr = vec![1i8; n];
    let (mut lu, mut ld): (Vec<Option<f64>>, Vec<Option<f64>>) = (vec![None; n], vec![None; n]);
    for i in 1..n {
        tr[i] = if c[i] > up[i - 1] { 1 } else if c[i] < dn[i - 1] { -1 } else { tr[i - 1] };
        if tr[i] > 0 && dn[i] < dn[i - 1] {
            dn[i] = dn[i - 1];
        }
        if tr[i] < 0 && up[i] > up[i - 1] {
            up[i] = up[i - 1];
        }
        if tr[i] < 0 && tr[i - 1] > 0 {
            up[i] = c[i] * (1.0 + reset_pct / 100.0);
        }
        if tr[i] > 0 && tr[i - 1] < 0 {
            dn[i] = c[i] * (1.0 - reset_pct / 100.0);
        }
        if tr[i] == 1 {
            lu[i] = Some(dn[i]);
            if tr[i - 1] == -1 {
                lu[i - 1] = ld[i - 1];
            }
        } else {
            ld[i] = Some(up[i]);
            if tr[i - 1] == 1 {
                ld[i - 1] = lu[i - 1];
            }
        }
    }
    (tr, ld)
}

/// The 5 DP AFL's bull / bear % swing: Up = C x (1 + bull%), Dn = C x (1 - bear%); returns (trend, Down_Trend_Filter).
fn regime_swing(c: &[f64], bull: f64, bear: f64) -> (Vec<i8>, Vec<Option<f64>>) {
    // at AmiBroker's 32-bit precision, so a close exactly on a band resolves as in the team's runs
    let n = c.len();
    let f = |v: f64| v as f32;
    let (ku, kd) = (bull as f32 / 100.0, bear as f32 / 100.0);
    let mut up: Vec<f32> = c.iter().map(|&v| f(v) + f(v) * ku).collect();
    let mut dn: Vec<f32> = c.iter().map(|&v| f(v) - f(v) * kd).collect();
    let mut tr = vec![1i8; n];
    let (mut lu, mut ld): (Vec<Option<f64>>, Vec<Option<f64>>) = (vec![None; n], vec![None; n]);
    for i in 1..n {
        tr[i] = if f(c[i]) > up[i - 1] { 1 } else if f(c[i]) < dn[i - 1] { -1 } else { tr[i - 1] };
        if tr[i] > 0 && dn[i] < dn[i - 1] {
            dn[i] = dn[i - 1];
        }
        if tr[i] < 0 && up[i] > up[i - 1] {
            up[i] = up[i - 1];
        }
        if tr[i] < 0 && tr[i - 1] > 0 {
            up[i] = f(c[i]) + f(c[i]) * ku;
        }
        if tr[i] > 0 && tr[i - 1] < 0 {
            dn[i] = f(c[i]) - f(c[i]) * kd;
        }
        if tr[i] == 1 {
            lu[i] = Some(dn[i] as f64);
            if tr[i - 1] == -1 {
                lu[i - 1] = ld[i - 1];
            }
        } else {
            ld[i] = Some(up[i] as f64);
            if tr[i - 1] == 1 {
                ld[i - 1] = lu[i - 1];
            }
        }
    }
    (tr, ld)
}

/// AFL RSI(n) (Wilder, seeded with the simple average of the first n changes), Null before bar n.
fn rsi_afl(c: &[f64], n: usize) -> Vec<Option<f64>> {
    crate::books::yteng::rsi(c, n)
}

/// Prec(x, 2) as AmiBroker evaluates it on a 32-bit price: 0.69 is stored as 0.68999999..., so it truncates to 0.68.
fn prec32(v: f64) -> f64 {
    ((v as f32) as f64 * 100.0).trunc()
}

/// The flipper AFL's Supertrend filter loop at AmiBroker's 32-bit precision (ATR(1) averaged over `period` bars, bands
/// calc +/- factor x that average), so a close exactly on a band resolves as in the team's runs. Before the average
/// exists the trend carries, as in `ytsw::supertrend`.
fn st32(b: &Bars, kind: i64, period: usize, factor: f64) -> Vec<i8> {
    let (h, l, c) = (&b.h, &b.l, &b.c);
    let n = c.len();
    let f = |v: f64| v as f32;
    // the calculation point from 32-bit prices (a 64-bit (H + L) / 2 can sit one step off the AFL's band)
    let ma = flt_point(b, kind);
    let calc: Vec<f32> = (0..n)
        .map(|i| {
            let (hi, lo, ci) = (f(h[i]), f(l[i]), f(c[i]));
            match kind {
                1 => ci,
                2 => (hi + lo + ci) / 3.0,
                3 => ((hi + lo) / 2.0 + (hi + lo + ci) / 3.0 + ci) / 3.0,
                4 | 5 => f(ma[i]),
                _ => (hi + lo) / 2.0,
            }
        })
        .collect();
    let p = period.max(1);
    let trr: Vec<Option<f32>> = (0..n).map(|i| if i == 0 { None } else { Some(f(h[i]).max(f(c[i - 1])) - f(l[i]).min(f(c[i - 1]))) }).collect();
    let satr: Vec<Option<f32>> = (0..n)
        .map(|i| if i + 1 < p { None } else { (0..p).try_fold(0.0f32, |s, k| trr[i - k].map(|v| s + v)).map(|s| s / p as f32) })
        .collect();
    let k = f(factor);
    let band = |i: usize, sign: f32| satr[i].map(|s| calc[i] + sign * (k * s));
    let mut up: Vec<Option<f32>> = (0..n).map(|i| band(i, 1.0)).collect();
    let mut dn: Vec<Option<f32>> = (0..n).map(|i| band(i, -1.0)).collect();
    let mut tr = vec![1i8; n];
    for i in 1..n {
        let ci = f(c[i]);
        tr[i] = match (up[i - 1], dn[i - 1]) {
            (Some(u), _) if ci > u => 1,
            (_, Some(d)) if ci < d => -1,
            _ => tr[i - 1],
        };
        if let (true, Some(a), Some(b)) = (tr[i] > 0, dn[i], dn[i - 1]) {
            if a < b {
                dn[i] = Some(b);
            }
        }
        if let (true, Some(a), Some(b)) = (tr[i] < 0, up[i], up[i - 1]) {
            if a > b {
                up[i] = Some(b);
            }
        }
        if tr[i] < 0 && tr[i - 1] > 0 {
            up[i] = band(i, 1.0);
        }
        if tr[i] > 0 && tr[i - 1] < 0 {
            dn[i] = band(i, -1.0);
        }
    }
    tr
}

/// The AFLs' DEAD STOCK BACKTEST toggle. NO (1, default): BarIndex() != 0 to buy, a sell on bar 0. YES (0): BarIndex() !=
/// LastValue(BarIndex()) to buy and a sell on that last bar; AmiBroker's last loaded bar is the run's last bar (`last`).
fn dead_stock(g: &dyn Fn(&str) -> f64, n: usize, last: usize) -> (Vec<bool>, Vec<bool>) {
    if g("dead_stock").round() as i64 == 0 {
        ((0..n).map(|i| i != last).collect(), (0..n).map(|i| i == last).collect())
    } else {
        ((0..n).map(|i| i != 0).collect(), (0..n).map(|i| i == 0).collect())
    }
}

/// The AFL % swing loop computed at AmiBroker's 32-bit precision (prices and bands are 32-bit floats there), so a close
/// exactly on a band resolves as in the team's runs. Up from `up_base` (the low), Dn from `dn_base` (the high).
fn swing32(c: &[f64], up_base: &[f64], dn_base: &[f64], pct: f64) -> Vec<i8> {
    let n = c.len();
    let f = |v: f64| v as f32;
    let k = pct as f32 / 100.0;
    let mut up: Vec<f32> = (0..n).map(|i| f(up_base[i]) + f(up_base[i]) * k).collect();
    let mut dn: Vec<f32> = (0..n).map(|i| f(dn_base[i]) - f(dn_base[i]) * k).collect();
    let mut tr = vec![1i8; n];
    for i in 1..n {
        tr[i] = if f(c[i]) > up[i - 1] { 1 } else if f(c[i]) < dn[i - 1] { -1 } else { tr[i - 1] };
        if tr[i] > 0 && dn[i] < dn[i - 1] {
            dn[i] = dn[i - 1];
        }
        if tr[i] < 0 && up[i] > up[i - 1] {
            up[i] = up[i - 1];
        }
        if tr[i] < 0 && tr[i - 1] > 0 {
            up[i] = f(up_base[i]) + f(up_base[i]) * k;
        }
        if tr[i] > 0 && tr[i - 1] < 0 {
            dn[i] = f(dn_base[i]) - f(dn_base[i]) * k;
        }
    }
    tr
}

// ---------------------------------------------------------------- the strategies

fn flip20(x: &Ctx, g: &dyn Fn(&str) -> f64, last: usize) -> Sig {
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let n = c.len();
    let gi = |k: &str| g(k).round() as i64;
    let tf = match gi("flt_tf") { 1 => 'W', 2 => 'D', _ => 'M' };
    let (fb, fs): (Vec<bool>, Vec<bool>) = if gi("flt") == 1 {
        sma_filter(x, tf, g("sma").round().max(1.0) as usize, false)
    } else {
        let b = compress(x, tf);
        let tr = st32(&b, gi("flt_point"), g("flt_period").round().max(1.0) as usize, g("flt_factor"));
        let m = expand_last(&tr, &b);
        (m.iter().map(|&v| v == 1).collect(), m.iter().map(|&v| v == -1).collect())
    };
    let tr = swing32(c, l, h, 20.0);
    let turn = turnover_ok(x, gi("turnover") == 1, true);
    let penny = g("penny");
    let (dbuy, dsell) = dead_stock(g, n, last);
    let buy = (0..n)
        .map(|i| {
            let upc = prec32(o[i]) == prec32(c[i]) && prec32(o[i]) == prec32(h[i]);
            fb[i] && tr[i] == 1 && turn[i] && !upc && h[i] != c[i] && c[i] > penny && dbuy[i]
        })
        .collect();
    let sell = (0..n).map(|i| ((tr[i] == -1 || fs[i]) && l[i] != c[i]) || dsell[i]).collect();
    Sig { buy, sell, short: vec![false; n], cover: vec![false; n] }
}

fn flip_trail(x: &Ctx, g: &dyn Fn(&str) -> f64, last: usize) -> Option<Sig> {
    let (h, l, c) = (&x.h, &x.l, &x.c);
    let n = c.len();
    let gi = |k: &str| g(k).round() as i64;
    let (ih, il, ic) = (bench(x, "BENCH.High")?, bench(x, "BENCH.Low")?, bench(x, "BENCH.Close")?);
    let first = ic.iter().position(|v| v.is_finite()).unwrap_or(n);
    let mut fb = vec![false; n];
    if first < n {
        let (bh, bl, bc) = (&ih[first..], &il[first..], &ic[first..]);
        if gi("flt") == 1 {
            let ma = sma(bc, g("sma").round().max(1.0) as usize);
            for k in 1..bc.len() {
                if let Some(m) = ma[k - 1] {
                    fb[first + k] = bc[k] > m;
                }
            }
        } else {
            let calc: Vec<f64> = (0..bc.len())
                .map(|k| match gi("flt_point") {
                    1 => bc[k],
                    2 => (bh[k] + bl[k] + bc[k]) / 3.0,
                    _ => (bh[k] + bl[k]) / 2.0,
                })
                .collect();
            let (tr, _) = supertrend(&calc, bh, bl, bc, g("flt_period").round().max(1.0) as usize, g("flt_factor"), g("flt_factor"));
            for k in 0..bc.len() {
                fb[first + k] = tr[k] == 1;
            }
        }
    }
    let tr = swing32(c, l, h, g("flip_pct"));
    let tr1 = swing32(c, l, h, g("trail_pct"));
    let turn = turnover_ok(x, gi("turnover") == 1, true);
    let penny = g("penny");
    let (dbuy, dsell) = dead_stock(g, n, last);
    let buy = (0..n).map(|i| fb[i] && tr[i] == 1 && turn[i] && h[i] != c[i] && c[i] > penny && dbuy[i]).collect();
    let sell = (0..n).map(|i| (if fb[i] { tr[i] == -1 } else { tr1[i] == -1 } && l[i] != c[i]) || dsell[i]).collect();
    Some(Sig { buy, sell, short: vec![false; n], cover: vec![false; n] })
}

fn dp5(x: &Ctx, g: &dyn Fn(&str) -> f64, last: usize) -> Px {
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let n = c.len();
    let gi = |k: &str| g(k).round() as i64;
    let gu = |k: &str| g(k).round().max(1.0) as usize;
    let month = |i: usize| x.r[i].date.get(5..7).and_then(|s| s.parse::<i64>().ok()).unwrap_or(0);
    // EOM = Month() != Ref(Month(), 1); the last bar (no next bar) is not counted
    let eom: Vec<bool> = (0..n).map(|i| i + 1 < n && month(i) != month(i + 1)).collect();
    let ed = gu("entry_day");
    let xd = gu("exit_day");
    let buysig: Vec<bool> = (0..n).map(|i| i + ed - 1 < n && eom[i + ed - 1]).collect();
    let mut sellsig: Vec<bool> = (0..n).map(|i| i >= xd && eom[i - xd]).collect();
    let cnd: Vec<bool> = (0..n).map(|i| match gi("candle") { 0 => c[i] > o[i], 1 => c[i] < o[i], _ => true }).collect();
    let rsi = rsi_afl(c, gu("rsi_period"));
    let adx = crate::connors::dmi(x.r, gu("adx_period")).1;
    let short1: Vec<bool> = rsi.iter().map(|v| v.map_or(false, |v| v > g("rsi_value"))).collect();
    let short2: Vec<bool> = adx.iter().map(|&v| v > g("adx_value")).collect();
    let flt = gi("flt");
    // ATHC filter
    let mut hc = c.clone();
    for i in 1..n {
        hc[i] = hc[i].max(hc[i - 1]);
    }
    let t1 = close_swing(c, g("athc_exit_pct"));
    let mut athc = vec![false; n];
    for i in 1..n {
        athc[i] = if c[i] > hc[i - 1] { true } else if t1[i] == -1 && athc[i - 1] { false } else { athc[i - 1] };
    }
    // ST filter
    let tf = match gi("flt_tf") { 1 => 'W', 2 => 'D', _ => 'M' };
    let point = match gi("flt_point") { 1 => 2, 2 => 3, _ => 1 };
    let m = st_filter(x, tf, point, gu("flt_period"), g("flt_factor"));
    // bull / bear swing
    let (tf_tr, down_tf) = regime_swing(c, g("bull_pct"), g("bear_pct"));
    let buy_flt: Vec<bool> = (0..n)
        .map(|i| match flt {
            1 => athc[i],
            2 => m[i] == 1,
            3 => down_tf[i].map_or(false, |d| (c[i] as f32) < (d as f32) && d > 0.0),
            _ => true,
        })
        .collect();
    let sell_flt: Vec<bool> = (0..n)
        .map(|i| match flt {
            1 => !athc[i],
            2 => m[i] == -1 && i > 0 && m[i - 1] == 1,
            _ => false,
        })
        .collect();
    for i in 0..n {
        sellsig[i] = sellsig[i] || sell_flt[i] || (flt == 3 && tf_tr[i] == 1);
    }
    let turn = turnover_ok(x, gi("turnover") == 1, true);
    let penny = g("penny");
    let (dbuy, _) = dead_stock(g, n, last);
    let cash: Vec<bool> = (0..n).map(|i| buy_flt[i] && h[i] != c[i] && turn[i] && c[i] > penny && dbuy[i]).collect();
    let (pt_pct, sl_pct, buf) = (g("profit_pct"), g("hard_sl_pct"), g("buffer_pct"));
    let mut s = Px::new(n);
    let (mut pos, mut skip) = (false, false);
    let (mut hsl, mut pt) = (0.0f64, 9_999_999.0f64);
    for i in 1..n {
        let lc = l[i] == c[i];
        if !pos && buysig[i] && cash[i] && cnd[i] && short1[i] && short2[i] && buy_flt[i] {
            s.buy[i] = true;
            if pt_pct > 0.0 {
                pt = (1.0 + pt_pct * 0.01) * c[i];
            }
            if sl_pct > 0.0 {
                hsl = (1.0 - sl_pct * 0.01) * c[i];
            }
            pos = true;
        } else if pos && !skip {
            if l[i] <= hsl || h[i] >= pt || sellsig[i] || sell_flt[i] {
                if !lc {
                    let (px, why) = if l[i] <= hsl {
                        (if o[i] < hsl { o[i] - o[i] * buf / 100.0 } else { hsl }, "Hard SL")
                    } else if h[i] >= pt {
                        (if o[i] > pt { o[i] - o[i] * buf / 100.0 } else { pt }, "Profit booking")
                    } else {
                        (c[i], "Sell day / filter")
                    };
                    s.sell(i, bound(x, i, px), why);
                    pos = false;
                    hsl = 0.0;
                    pt = 9_999_999.0;
                } else {
                    skip = true;
                }
            }
        } else if skip && !lc {
            s.sell(i, c[i], "Postponed exit (low = close)");
            pos = false;
            skip = false;
            hsl = 0.0;
            pt = 9_999_999.0;
        }
    }
    s
}

fn single(x: &Ctx, g: &dyn Fn(&str) -> f64, from: usize, to: usize) -> Vec<Trade> {
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let n = c.len();
    let ex = g("exit_pct");
    let t1 = close_swing(c, ex);
    let (t2, down2) = swing_lines(c, g("swing_flt"), ex);
    let prev: Vec<Option<f64>> = (0..n).map(|i| if i == 0 { None } else { down2[i - 1] }).collect();
    let dbuy = cross_up(c, &prev);
    let mut flag = vec![false; n];
    for i in 1..n {
        flag[i] = if dbuy[i] && t1[i] == 1 { true } else if t1[i] == -1 || t2[i] == -1 { false } else { flag[i - 1] };
    }
    let upc = |i: usize| prec2(o[i]) == prec2(c[i]) && prec2(o[i]) == prec2(h[i]);
    let loc = |i: usize| prec2(o[i]) == prec2(c[i]) && prec2(o[i]) == prec2(l[i]);
    let turn = turnover_ok(x, g("turnover").round() as i64 == 1, false);
    let penny = g("penny");
    let (dsbuy, _) = dead_stock(g, n, to);
    let buy: Vec<bool> = (0..n).map(|i| (dbuy[i] || flag[i]) && dsbuy[i] && !upc(i) && t1[i] == 1 && c[i] > penny && turn[i]).collect();
    let sell: Vec<bool> = (0..n).map(|i| ((t1[i] == -1 || t2[i] == -1) && !loc(i)) || (!flag[i] && !loc(i))).collect();
    let buy = crate::books::ytsw::exrem(&buy, &sell);
    let sell = crate::books::ytsw::exrem(&sell, &buy);
    let entry: Vec<Option<f64>> = (0..n).map(|i| if buy[i] { Some(c[i]) } else { None }).collect();
    let exit: Vec<Option<(f64, &'static str)>> = (0..n).map(|i| if sell[i] { Some((c[i], "Sell signal")) } else { None }).collect();
    fills(x, 1.0, &entry, &exit, from, to, "close", false, g("hard_stop"))
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(0.0, |d| d.def));
    let n = x.c.len();
    if n == 0 || from > to || to >= n {
        return Some(Vec::new());
    }
    Some(match id {
        FLIP20 => backtest(x, flip20(x, &g, to), 1, from, to),
        FLIP_TRAIL => match flip_trail(x, &g, to) {
            Some(s) => backtest(x, s, 1, from, to),
            None => Vec::new(),
        },
        DP5 => px_backtest(x, &dp5(x, &g, to), from, to),
        SINGLE => single(x, &g, from, to),
        _ => return None,
    })
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let d = catalogue().into_iter().find(|d| d.id == id)?;
    Some((d.id, d.name, "% Swing"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::books::ytwh::testbars::{bars, ps, wave};
    use crate::Row;

    fn with_volume(mut r: Vec<Row>, v: f64) -> Vec<Row> {
        for b in r.iter_mut() {
            b.volume = Some(v);
        }
        r
    }

    #[test]
    fn swing_lines_reset_at_the_reset_pct() {
        // start bands 8%: 100 -> 91 is a 9% drop (flip down); the new upper band is 91 x 1.05 = 95.55, not 98.28
        let c = [100.0, 91.0, 95.0, 96.0];
        let (tr, line) = swing_lines(&c, 8.0, 5.0);
        assert_eq!(tr, vec![1, -1, -1, 1]);
        assert!((line[1].unwrap() - 95.55).abs() < 1e-9);
        // the bar before the flip down takes that bar's up-trend line, Null on bar 0
        assert!(line[0].is_none());
    }

    #[test]
    fn flipper_buys_on_up_swing_and_sells_on_down_swing() {
        // daily SMA filter of 1 period: C > MA(C, 1) is never true, so use the ST filter on daily bars with a rising
        // series; then a 25% fall flips the 20% swing down
        let mut v: Vec<(f64, f64, f64, f64)> = (0..40).map(|k| { let c = 100.0 + k as f64; (c - 0.5, c + 1.0, c - 1.0, c) }).collect();
        for k in 0..10 {
            let c = 139.0 * (1.0 - 0.03 * (k + 1) as f64);
            v.push((c + 0.5, c + 1.0, c - 1.0, c));
        }
        let r = bars(&v);
        let x = Ctx::new(&r, &r, &[]);
        let t = run(FLIP20, &x, 0, r.len() - 1, &ps(&[("flt_tf", 2.0)])).unwrap();
        assert!(!t.is_empty());
        assert_eq!(t[0].legs[0].i, 1);
        assert!(!t[0].open && t[0].exit_i >= 40);
    }

    #[test]
    fn dp5_buys_four_bars_before_month_end_and_books_profit() {
        // weekdays from 2024-01-01; Jan's last bar is 2024-01-31 (index 22); buy day = index 18 (Jan 25)
        let mut v: Vec<(f64, f64, f64, f64)> = (0..60).map(|k| { let c = 100.0 + (k as f64 * 0.7).sin() * 3.0 + k as f64 * 0.3; (c - 0.2, c + 0.6, c - 0.6, c) }).collect();
        // the bar after the buy day reaches +3%
        let bc = v[18].3;
        v[19] = (bc, bc * 1.04, bc - 0.1, bc * 1.01);
        let r = bars(&v);
        let x = Ctx::new(&r, &r, &[]);
        let s = dp5(&x, &|k: &str| params(DP5).unwrap().iter().find(|d| d.key == k).unwrap().def, x.c.len() - 1);
        if s.buy[18] {
            assert!(s.sell[19]);
            assert!((s.sell_px[19] - bc * 1.03).abs() < 1e-9);
        }
        // the buy days are always 4 bars before a month end
        let eom: Vec<bool> = (0..60).map(|i| i + 1 < 60 && r[i].date[5..7] != r[i + 1].date[5..7]).collect();
        assert!((0..60).all(|i| !s.buy[i] || (i + 4 < 60 && eom[i + 4])));
    }

    #[test]
    fn single_trade_needs_turnover_and_penny() {
        let r = wave(300);
        let x = Ctx::new(&r, &r, &[]);
        // no volume: the default turnover filter fails every bar
        assert!(run(SINGLE, &x, 0, 299, &Params::new()).unwrap().is_empty());
        let r = with_volume(wave(300), 1e9);
        let x = Ctx::new(&r, &r, &[]);
        let t = run(SINGLE, &x, 0, 299, &Params::new()).unwrap();
        assert!(t.iter().all(|t| x.c[t.legs[0].i] > 50.0));
        assert!(!run(SINGLE, &x, 0, 299, &ps(&[("penny", 0.0)])).unwrap().is_empty() || t.is_empty());
    }

    #[test]
    fn flip_trail_needs_the_benchmark() {
        let r = wave(300);
        let mut x = Ctx::new(&r, &r, &[]);
        assert!(run(FLIP_TRAIL, &x, 0, 299, &Params::new()).unwrap().is_empty());
        for (k, f) in [("High", 1.01), ("Low", 0.99), ("Close", 1.0)] {
            x.extra.insert(format!("BENCH.{k}"), (0..300).map(|i| if i < 10 { f64::NAN } else { (100.0 + i as f64 * 0.2) * f }).collect());
        }
        let t = run(FLIP_TRAIL, &x, 0, 299, &Params::new()).unwrap();
        assert!(t.iter().all(|t| t.legs[0].i >= 10));
    }

    #[test]
    fn every_strategy_runs() {
        let r = with_volume(wave(420), 1e9);
        let mut x = Ctx::new(&r, &r, &[]);
        for pass in 0..2 {
            if pass == 1 {
                for (k, f) in [("Open", 1.0), ("High", 1.01), ("Low", 0.99), ("Close", 1.0)] {
                    x.extra.insert(format!("BENCH.{k}"), x.c.iter().enumerate().map(|(i, v)| if i % 17 == 3 { f64::NAN } else { v * 0.5 * f }).collect());
                }
            }
            for id in IDS {
                assert!(params(id).is_some() && family_of(id).is_some(), "{id}");
                for kv in [vec![], vec![("flt", 1.0)], vec![("flt", 2.0), ("flt_tf", 2.0)], vec![("flt", 3.0)], vec![("turnover", 0.0), ("penny", 0.0)]] {
                    for (from, to) in [(0, 419), (50, 300)] {
                        let t = run(id, &x, from, to, &ps(&kv)).unwrap();
                        for t in &t {
                            assert!(t.legs[0].i >= from && t.legs[0].i <= to && t.exit_i >= t.legs[0].i && t.exit_i <= to, "{id}");
                            assert!(t.exit_px.is_finite() && t.exit_px >= x.l[t.exit_i] - 1e-9 && t.exit_px <= x.h[t.exit_i] + 1e-9, "{id}");
                        }
                        assert!(t.windows(2).all(|w| w[1].legs[0].i >= w[0].exit_i), "{id}");
                    }
                }
            }
        }
        assert_eq!(catalogue().len(), IDS.len());
    }

    #[test]
    fn prec_and_supertrend_ties_follow_amibroker_32_bit_values() {
        // AEGISCHEM 09-06-2003 (old C&P): O 0.69, H 0.69, C 0.68 is an up-circuit bar to the AFL's Prec(.., 2)
        assert_eq!(prec32(0.69), 68.0);
        assert_eq!(prec32(0.68), 68.0);
        assert_eq!(prec32(16.0), 1600.0);
        // a close exactly on the up band does not turn the trend up
        let c = [10.0, 10.0, 9.0, 10.0, 10.5];
        let b = Bars { h: c.to_vec(), l: c.to_vec(), c: c.to_vec(), last: (0..5).collect(), group: (0..5).collect() };
        assert_eq!(st32(&b, 0, 1, 1.0), vec![1, 1, -1, -1, 1]);
    }

}
