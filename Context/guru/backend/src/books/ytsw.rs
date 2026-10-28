//! YT Strategies · % Swing: the team's AmiBroker AFLs in "Old Topics/% Swing" that trade an index on daily bars
//! (spot Nifty / Bank Nifty; the single and monthly-filter % swing AFLs were also run on index futures). Coded from
//! the AFLs as written, with the AFLs' own default settings; there is no book PDF.
//!
//! AmiBroker behaviour the code follows (worked out against the team's workbooks for these AFLs):
//! - fills are at the signal bar's close, no trade delay; a position still open on the last bar is marked there;
//! - `ExRem(a, b)` keeps the first `a` after each `b`, and a `b` on the same bar already resets it; the AFLs apply
//!   `Buy = ExRem(Buy, Sell)` and then `Sell = ExRem(Sell, Buy)` with the new Buy;
//! - the backtester exits first and then enters on the same bar, even while the exit signal is still true; in
//!   long-short mode an opposite entry reverses the position;
//! - `TimeFrameExpand(.., expandLast)`: a weekly / monthly value reaches the daily bars on the period's last bar;
//!   before that they carry the previous period's value, and the first period carries its own;
//! - `ATR(1)` is Null on the first bar (no previous close), so ST bands start one bar later;
//! - Null in an array before its first value is outside the array (no comparison there); a Null after that
//!   compares as -1e10 (so `Cross(C, line)` fires when a line that is Null on up-trend bars ends).
#![allow(dead_code)]
use crate::books::ytmr::day_number;
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "yt_sw";
pub const SINGLE: &str = "ytsw_single";
pub const BULL: &str = "ytsw_monthly_bull";
pub const BEAR: &str = "ytsw_monthly_bear";
pub const BULL_BUY: &str = "ytsw_monthly_bull_buy";
pub const DIFF: &str = "ytsw_diff_entry_exit";
pub const DIFF_LS: &str = "ytsw_diff_entry_exit_ls";
pub const SWING_ST: &str = "ytsw_swing_entry_st_exit";
pub const ST_SWING_LONG: &str = "ytsw_st_entry_swing_exit_long";
pub const ST_SWING_SHORT: &str = "ytsw_st_entry_swing_exit_short";
pub const DIP: &str = "ytsw_buy_on_dip";
pub const DIP_FLT: &str = "ytsw_buy_on_dip_filter";
pub const REGIM: &str = "ytsw_regim";
pub const IDS: [&str; 12] = [SINGLE, BULL, BEAR, BULL_BUY, DIFF, DIFF_LS, SWING_ST, ST_SWING_LONG, ST_SWING_SHORT, DIP, DIP_FLT, REGIM];
/// The ones the team also ran on index futures (their FO workbooks); the rest were run on the spot indices only.
pub const FUTURES_TOO: [&str; 4] = [SINGLE, BULL, BEAR, BULL_BUY];

fn def(id: &'static str, name: &'static str, dir: &'static str, rules: &'static [&'static str], notes: &'static str) -> StrategyDef {
    StrategyDef { id, name, group: "YT Strategies · % Swing", session: 0, dir, status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim: None }
}

const SIDE: &str = "Side (1 long only, 2 short only, 3 long and short)";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![
        def(SINGLE, "Single % Swing", "Long/Short", &[
            "F|% swing trend: up when the close rises more than the swing % above the trailing band, down when it falls more than the swing % below it (bands trail the close and reset on each flip)",
            "E|Long when the trend is up, short when it is down, at the close",
            "X|Exit (or reverse) when the trend flips",
        ], "5% swing afl (% Swing, Single %swing entry-exit), coded as the AFL is written. Swing % default 5; side picks the AmiBroker long / short / long-short run."),
        def(BULL, "% Swing with Monthly ST Filter (Bull)", "Long/Short", &[
            "F|Monthly Supertrend of the same series (period 5, factor 1, on the monthly mid point), expanded to daily bars",
            "E|Long when the monthly ST is up and the % swing trend is up; short when the monthly ST is down and the swing is down",
            "X|Sell when the swing turns down; cover when it turns up",
        ], "5% swing with st monthly flt for bull mkt, coded as the AFL is written. Swing % default 3 (the AFL's default)."),
        def(BEAR, "% Swing with Monthly ST Filter (Bear)", "Long/Short", &[
            "F|Monthly Supertrend of the same series (period 5, factor 1, mid point), expanded to daily bars",
            "E|Long when the monthly ST is DOWN and the swing is up; short when the monthly ST is down and the swing is down",
            "X|Sell when the monthly ST turns up or the swing turns down; cover when the swing turns up",
        ], "5% swing with st monthly flt for bear mkt, coded as the AFL is written (buys only in a monthly down trend). Swing % default 3."),
        def(BULL_BUY, "% Swing: Bull Buy, Bear Buy-Sell", "Long/Short", &[
            "E|Long whenever the % swing trend is up (no filter); short only when the monthly ST (5, 1, mid point) is down and the swing is down",
            "X|Sell when the swing turns down; cover when it turns up",
        ], "5% swing with st monthly flt bull buy bear buy sell, coded as the AFL is written. Swing % default 3."),
        def(DIFF, "Different % Swing Entry and Exit", "Long/Short", &[
            "F|Separate entry and exit % swings for each side; optional market filter: none (default), the series' own ST (monthly 5, 1, mid point) or SMA (close above its 200-period average)",
            "E|Long: if the entry % is below the exit %, when the entry swing is up; otherwise when both swings are up. Short: if the exit % is below the entry %, when both are down; otherwise when the entry swing is down",
            "X|Long: if entry % < exit %, when both swings are down; otherwise when the exit swing is down. Short: if exit % < entry %, when the exit swing is up; otherwise when both are up",
        ], "5% swing with diff entry exit - For Long only, Short Only - FINAL, coded as the AFL is written; all swing % default 3, swing calculated on the close. The team's filtered runs used ST 20, 1 (the AFL default is 5, 1)."),
        def(DIFF_LS, "Different % Swing Entry and Exit (Long-Short)", "Long/Short", &[
            "F|One entry % and one exit % for both sides; optional market filter as in the long/short-only version",
            "E|If entry % > exit %: long when the close crosses above the entry swing's down-trend line, short when it crosses below it; otherwise long when the entry swing is up, short when both swings are down",
            "X|If entry % > exit %: sell when the exit swing is down, cover when it is up; otherwise sell when both are down, cover when the entry swing is up",
        ], "5% swing with diff entry exit for long-short - FINAL, coded as the AFL is written; entry and exit % default 3. The down-trend line is Null on up-trend bars, so the cross fires as that line ends (AmiBroker Null handling)."),
        def(SWING_ST, "% Swing Entry, ST Exit", "Long", &[
            "E|Buy when the % swing trend (default 3%) is up",
            "X|Sell when the daily Supertrend (period 5, factor 2) turns down; its upper band uses factor 1 and only the lower band the ST factor, as in the AFL",
        ], "%SWING ENTRY N ST EXIT LONG ONLY - FINAL, coded as the AFL is written. ST calculation point default Close (the AFL default); the team's workbooks were run on the mid point. While the swing is up and the ST is down, the AFL's signals buy and sell on alternate bars, as AmiBroker does."),
        def(ST_SWING_LONG, "ST Entry, % Swing Exit (Long)", "Long", &[
            "F|Filter 1 (WITH filter AFL): only when the monthly ST of the series (5, 1, mid point) is up; filter 0 (Wo filter AFL): no filter",
            "E|Buy when the close crosses above the previous bar's daily Supertrend down line (period 5, factor 2, mid point)",
            "X|Sell when the % swing (default 3%) turns down or the daily ST turns down",
        ], "st entry n %swing exit - long only (Wo filter_FINAL and WITH filter - FINAL), coded as the AFLs are written."),
        def(ST_SWING_SHORT, "ST Entry, % Swing Exit (Short)", "Short", &[
            "F|Filter 1 (WITH filter AFL): only when the monthly ST of the series (5, 1, mid point) is down; filter 0: no filter",
            "E|Short when the daily Supertrend (period 5, factor 2, mid point) and the % swing (default 3%) are both down",
            "X|Cover when the swing turns up or the daily ST turns up",
        ], "ST Entry n %swing Exit short only (Wo filter_FINAL(Copy) and WITH filter - FINAL), coded as the AFLs are written."),
        def(DIP, "Buy on Dip with Scale-out", "Long", &[
            "E|Track the highest close since the last buy; buy one lot (1/5 of the allocation) at the close when it falls more than the dip % (default 5) below that; up to 5 lots, each new buy resets the reference to its close",
            "X|Each lot has a profit level, its buy price plus the booking % (default 10); a close above the highest open level sells everything, a close above a lower level sells the lots from that level down",
            "F|No buys on the first bar, and no buy or profit booking on a bar whose low equals its close (the AFL's LC rule)",
        ], "buy on dip with scaleout_Final, coded as the AFL is written, including its level insertion: a new lot's level is compared only with the next level up (the filter version sorts them). Results are on the full 5-lot allocation; unfilled lots are idle cash. A trade that buys more than 5 lots in all (re-buys after scale-outs) gives each lot 1 / lots of the allocation."),
        def(DIP_FLT, "Buy on Dip with Scale-out and Filter", "Long", &[
            "F|Filter on the series itself: ST (default; daily, period 20, factor 1, mid point), SMA (close above the previous 200-period average) or off; timeframe daily (default), weekly or monthly",
            "E|As Buy on Dip, buying only while the filter is up",
            "X|As Buy on Dip; and everything is sold when the filter turns off",
        ], "Buy on dip_with Flt_Updated_Final, coded as the AFL is written (profit levels kept in order). The team's workbooks used weekly 20-3, monthly 20-1 and monthly 5-2 filters. Lots are 1/5 of the allocation; a trade that buys more than 5 lots in all gives each lot 1 / lots."),
        def(REGIM, "Regime % Swing (Bull Long, Bear Long-Short)", "Long/Short", &[
            "F|Regime swing: up on a close more than the bull % (default 44) above its band or above every earlier close, down on a close more than the bear % (default 23) below its band",
            "F|Market filter for shorts: monthly ST of the series (5, 1, mid point; default), SMA (close above the previous 200-period average) or S% (the regime swing)",
            "E|Long when the trading % swing (default 3%) is up; short when the filter is down and the swing is down",
            "X|Sell when the swing turns down; cover when the swing turns up or the filter turns up",
        ], "REGIM - ENTRY EXIT WITH S% BL-LONG ONLY BR-LONG SHORT (To be checked folder), coded as the AFL is written; there is no team workbook for it. Like the AFL, no buy on a bar whose open, high and close are equal (upper circuit) and no sell when open, low and close are equal."),
    ]
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    let side = |d: f64| p("side", SIDE, d, 1.0, 3.0, 1.0);
    let flt_tf = |d: f64| p("flt_tf", "Filter timeframe (1 monthly, 2 weekly, 3 daily)", d, 1.0, 3.0, 1.0);
    let point = |key, d: f64| p(key, "ST calculation point (1 mid point, 2 close, 3 pivot)", d, 1.0, 3.0, 1.0);
    Some(match id {
        SINGLE => vec![p("pct", "Swing %", 5.0, 1.0, 50.0, 0.5), side(1.0)],
        BULL | BEAR | BULL_BUY => vec![
            p("pct", "Swing %", 3.0, 1.0, 50.0, 0.5),
            side(1.0),
            flt_tf(1.0),
            point("flt_point", 1.0),
            p("flt_period", "Filter ST period", 5.0, 1.0, 30.0, 1.0),
            p("flt_factor", "Filter ST factor", 1.0, 0.5, 10.0, 0.5),
        ],
        DIFF | DIFF_LS => {
            let mut v = if id == DIFF {
                vec![
                    p("long_entry", "Swing % for long entry", 3.0, 1.0, 50.0, 0.5),
                    p("long_exit", "Swing % for long exit", 3.0, 1.0, 50.0, 0.5),
                    p("short_entry", "Swing % for short entry", 3.0, 1.0, 50.0, 0.5),
                    p("short_exit", "Swing % for short exit", 3.0, 1.0, 50.0, 0.5),
                    side(1.0),
                ]
            } else {
                vec![p("entry", "Swing % for entry", 3.0, 1.0, 50.0, 0.5), p("exit", "Swing % for exit", 3.0, 1.0, 50.0, 0.5), side(3.0)]
            };
            v.extend([
                p("swing_point", "Swing % calculation point (1 close, 2 high/low, 3 mid point)", 1.0, 1.0, 3.0, 1.0),
                p("flt", "Market filter (0 none, 1 ST, 2 SMA)", 0.0, 0.0, 2.0, 1.0),
                flt_tf(1.0),
                point("flt_point", 1.0),
                p("flt_period", "Filter ST period", 5.0, 1.0, 30.0, 1.0),
                p("flt_factor", "Filter ST factor", 1.0, 0.5, 10.0, 0.5),
                p("sma", "Filter SMA period", 200.0, 1.0, 300.0, 1.0),
            ]);
            v
        }
        SWING_ST => vec![
            p("pct", "Swing % for entry", 3.0, 1.0, 50.0, 0.5),
            p("st_period", "ST period for exit", 5.0, 1.0, 50.0, 1.0),
            p("st_factor", "ST factor for exit", 2.0, 0.5, 10.0, 0.5),
            p("st_point", "ST calculation point (1 close, 2 mid point, 3 pivot)", 1.0, 1.0, 3.0, 1.0),
        ],
        ST_SWING_LONG | ST_SWING_SHORT => vec![
            p("st_period", "ST period", 5.0, 1.0, 50.0, 1.0),
            p("st_factor", "ST factor", 2.0, 0.5, 12.0, 0.5),
            p("pct", "Swing %", 3.0, 1.0, 50.0, 0.5),
            point("st_point", 1.0),
            p("flt", "Monthly ST filter (0 off: Wo filter AFL, 1 on: WITH filter AFL)", 0.0, 0.0, 1.0, 1.0),
            flt_tf(1.0),
            p("flt_period", "Filter ST period", 5.0, 1.0, 30.0, 1.0),
            p("flt_factor", "Filter ST factor", 1.0, 0.5, 10.0, 0.5),
        ],
        DIP => vec![p("buy_pct", "Buy on dip %", 5.0, 1.0, 25.0, 1.0), p("pb_pct", "Profit booking %", 10.0, 1.0, 25.0, 1.0)],
        DIP_FLT => vec![
            p("buy_pct", "Buy on dip %", 5.0, 1.0, 25.0, 1.0),
            p("pb_pct", "Profit booking %", 10.0, 1.0, 25.0, 1.0),
            p("flt", "Filter (1 ST, 2 SMA, 3 off)", 1.0, 1.0, 3.0, 1.0),
            p("flt_tf", "Filter timeframe (1 daily, 2 weekly, 3 monthly)", 1.0, 1.0, 3.0, 1.0),
            point("flt_point", 1.0),
            p("flt_period", "Filter ST period", 20.0, 1.0, 30.0, 1.0),
            p("flt_factor", "Filter ST factor", 1.0, 0.5, 10.0, 0.5),
            p("sma", "Filter SMA period", 200.0, 1.0, 2000.0, 1.0),
        ],
        REGIM => vec![
            p("bear_pct", "Swing % for bear market", 23.0, 10.0, 50.0, 1.0),
            p("bull_pct", "Swing % for bull market", 44.0, 10.0, 50.0, 1.0),
            p("pct", "Swing % for entry and exit", 3.0, 1.0, 50.0, 0.5),
            p("swing_point", "Swing % calculation point (1 close, 2 high/low, 3 mid point)", 1.0, 1.0, 3.0, 1.0),
            p("flt", "Market filter (1 ST, 2 SMA, 3 S% regime swing)", 1.0, 1.0, 3.0, 1.0),
            flt_tf(1.0),
            point("flt_point", 1.0),
            p("flt_period", "Filter ST period", 5.0, 1.0, 20.0, 1.0),
            p("flt_factor", "Filter ST factor", 1.0, 0.5, 20.0, 0.5),
            p("sma", "Filter SMA period", 200.0, 1.0, 300.0, 1.0),
            side(3.0),
        ],
        _ => return None,
    })
}

// ---------------------------------------------------------------- the AFLs' % swing and Supertrend loops

/// AFL % swing loop: Up = up_base x (1 + pct/100), Dn = dn_base x (1 - pct/100); up when C > Up[i-1], down when
/// C < Dn[i-1]; in an up trend Dn only rises, in a down trend Up only falls; a flip resets the other band.
/// Returns (trend, down-trend line = Up on down-trend bars, None elsewhere).
pub(crate) fn swing(c: &[f64], up_base: &[f64], dn_base: &[f64], pct: f64) -> (Vec<i8>, Vec<Option<f64>>) {
    let n = c.len();
    let mut up: Vec<f64> = (0..n).map(|i| up_base[i] * (1.0 + pct / 100.0)).collect();
    let mut dn: Vec<f64> = (0..n).map(|i| dn_base[i] * (1.0 - pct / 100.0)).collect();
    let mut tr = vec![1i8; n];
    let mut line = vec![None; n];
    for i in 1..n {
        tr[i] = if c[i] > up[i - 1] { 1 } else if c[i] < dn[i - 1] { -1 } else { tr[i - 1] };
        if tr[i] > 0 && dn[i] < dn[i - 1] {
            dn[i] = dn[i - 1];
        }
        if tr[i] < 0 && up[i] > up[i - 1] {
            up[i] = up[i - 1];
        }
        if tr[i] < 0 && tr[i - 1] > 0 {
            up[i] = up_base[i] * (1.0 + pct / 100.0);
        }
        if tr[i] > 0 && tr[i - 1] < 0 {
            dn[i] = dn_base[i] * (1.0 - pct / 100.0);
        }
        if tr[i] < 0 {
            line[i] = Some(up[i]);
        }
    }
    (tr, line)
}

pub(crate) fn close_swing(c: &[f64], pct: f64) -> Vec<i8> {
    swing(c, c, c, pct).0
}

/// AFL ATR(1): max(H, prev C) - min(L, prev C); Null on the first bar.
pub(crate) fn true_range(h: &[f64], l: &[f64], c: &[f64]) -> Vec<Option<f64>> {
    (0..c.len()).map(|i| if i == 0 { None } else { Some(h[i].max(c[i - 1]) - l[i].min(c[i - 1])) }).collect()
}

/// AFL ST loop: satr = mean of the last `period` ATR(1) values (Null until they exist), Up = calc + f_up x satr,
/// Dn = calc - f_dn x satr, same flip / ratchet logic as the % swing loop; the trend holds while the bands are Null.
/// Returns (trend, down-trend line as the ST-entry AFLs set it: Up on down-trend bars and, on the bar before a
/// flip down, that bar's up-trend line).
pub(crate) fn supertrend(calc: &[f64], h: &[f64], l: &[f64], c: &[f64], period: usize, f_up: f64, f_dn: f64) -> (Vec<i8>, Vec<Option<f64>>) {
    let n = c.len();
    let trr = true_range(h, l, c);
    let p = period.max(1);
    let satr: Vec<Option<f64>> = (0..n)
        .map(|i| if i + 1 < p { None } else { trr[i + 1 - p..=i].iter().try_fold(0.0, |s, v| v.map(|v| s + v)).map(|s| s / p as f64) })
        .collect();
    let mut up: Vec<Option<f64>> = (0..n).map(|i| satr[i].map(|s| calc[i] + f_up * s)).collect();
    let mut dn: Vec<Option<f64>> = (0..n).map(|i| satr[i].map(|s| calc[i] - f_dn * s)).collect();
    let mut tr = vec![1i8; n];
    let (mut lu, mut ld) = (vec![None; n], vec![None; n]);
    for i in 1..n {
        tr[i] = match (up[i - 1], dn[i - 1]) {
            (Some(u), _) if c[i] > u => 1,
            (_, Some(d)) if c[i] < d => -1,
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
            up[i] = satr[i].map(|s| calc[i] + f_up * s);
        }
        if tr[i] > 0 && tr[i - 1] < 0 {
            dn[i] = satr[i].map(|s| calc[i] - f_dn * s);
        }
        if tr[i] == 1 {
            lu[i] = dn[i];
            if tr[i - 1] == -1 {
                lu[i - 1] = ld[i - 1];
            }
        } else {
            ld[i] = up[i];
            if tr[i - 1] == 1 {
                ld[i - 1] = lu[i - 1];
            }
        }
    }
    (tr, ld)
}

/// ST calculation point: 1 mid point (H+L)/2, 2 close, 3 pivot (H+L+C)/3.
pub(crate) fn calc_point(o: &Bars, kind: i64) -> Vec<f64> {
    (0..o.c.len())
        .map(|i| match kind {
            2 => o.c[i],
            3 => (o.h[i] + o.l[i] + o.c[i]) / 3.0,
            _ => (o.h[i] + o.l[i]) / 2.0,
        })
        .collect()
}

// ---------------------------------------------------------------- time frames

pub(crate) struct Bars {
    pub h: Vec<f64>,
    pub l: Vec<f64>,
    pub c: Vec<f64>,
    /// last daily bar of each period, and the period of each daily bar
    pub last: Vec<usize>,
    pub group: Vec<usize>,
}

/// Daily bars as they are (tf 'D'), or compressed to weeks (Monday-Sunday) or calendar months.
pub(crate) fn compress(x: &Ctx, tf: char) -> Bars {
    let n = x.c.len();
    let key = |i: usize| -> i64 {
        match tf {
            'M' => x.r[i].date.get(..7).map_or(i as i64, |s| s.replace('-', "").parse().unwrap_or(i as i64)),
            'W' => day_number(&x.r[i].date).map_or(i as i64, |d| (d + 3).div_euclid(7)),
            _ => i as i64,
        }
    };
    let mut b = Bars { h: vec![], l: vec![], c: vec![], last: vec![], group: vec![0; n] };
    let mut prev = None;
    for i in 0..n {
        let k = key(i);
        if prev != Some(k) {
            b.h.push(x.h[i]);
            b.l.push(x.l[i]);
            b.c.push(x.c[i]);
            b.last.push(i);
            prev = Some(k);
        }
        let g = b.c.len() - 1;
        b.h[g] = b.h[g].max(x.h[i]);
        b.l[g] = b.l[g].min(x.l[i]);
        b.c[g] = x.c[i];
        b.last[g] = i;
        b.group[i] = g;
    }
    b
}

pub(crate) fn expand_last<T: Copy>(v: &[T], b: &Bars) -> Vec<T> {
    (0..b.group.len()).map(|i| { let g = b.group[i]; if b.last[g] == i || g == 0 { v[g] } else { v[g - 1] } }).collect()
}

/// Market filter: ST trend of the series on the filter timeframe, expanded to daily bars (1 up, -1 down).
pub(crate) fn st_filter(x: &Ctx, tf: char, point: i64, period: usize, factor: f64) -> Vec<i8> {
    let b = compress(x, tf);
    let (tr, _) = supertrend(&calc_point(&b, point), &b.h, &b.l, &b.c, period, factor, factor);
    expand_last(&tr, &b)
}

/// Simple moving average of the period closes (None until `n` periods exist).
pub(crate) fn sma(c: &[f64], n: usize) -> Vec<Option<f64>> {
    let n = n.max(1);
    (0..c.len()).map(|i| if i + 1 < n { None } else { Some(c[i + 1 - n..=i].iter().sum::<f64>() / n as f64) }).collect()
}

/// SMA filter as `C > MA(C, n)` (diff-entry AFL, `prev` false) or `C > Ref(MA, -1)` (dip and REGIM AFLs) on the filter
/// timeframe, expanded: (filter buy, filter short). Before the average exists both are false.
pub(crate) fn sma_filter(x: &Ctx, tf: char, n: usize, prev: bool) -> (Vec<bool>, Vec<bool>) {
    let b = compress(x, tf);
    let ma = sma(&b.c, n);
    let at = |g: usize| if prev { if g == 0 { None } else { ma[g - 1] } } else { ma[g] };
    let up: Vec<bool> = (0..b.c.len()).map(|g| at(g).map_or(false, |m| b.c[g] > m)).collect();
    let dn: Vec<bool> = (0..b.c.len()).map(|g| at(g).map_or(false, |m| b.c[g] < m)).collect();
    (expand_last(&up, &b), expand_last(&dn, &b))
}

pub(crate) fn tf_of(code: i64, monthly_first: bool) -> char {
    match (code, monthly_first) {
        (1, true) | (3, false) => 'M',
        (2, _) => 'W',
        _ => 'D',
    }
}

// ---------------------------------------------------------------- AmiBroker signal handling

/// AFL Cross(a, b) on bars where `b` may be Null: before b's first value there is no cross; after it a Null b is -1e10.
pub(crate) fn cross_up(a: &[f64], b: &[Option<f64>]) -> Vec<bool> {
    let first = b.iter().position(|v| v.is_some()).unwrap_or(b.len());
    let v = |z: Option<f64>| z.unwrap_or(-1e10);
    (0..a.len()).map(|i| i > first && a[i] > v(b[i]) && a[i - 1] <= v(b[i - 1])).collect()
}
pub(crate) fn cross_down(a: &[f64], b: &[Option<f64>]) -> Vec<bool> {
    let first = b.iter().position(|v| v.is_some()).unwrap_or(b.len());
    let v = |z: Option<f64>| z.unwrap_or(-1e10);
    (0..a.len()).map(|i| i > first && v(b[i]) > a[i] && v(b[i - 1]) <= a[i - 1]).collect()
}

/// AFL ExRem(a, b): the first a after each b; a b on that same bar already resets it.
pub(crate) fn exrem(a: &[bool], b: &[bool]) -> Vec<bool> {
    let mut on = false;
    (0..a.len())
        .map(|i| {
            let mut out = false;
            if !on && a[i] {
                out = true;
                on = !b[i];
            } else if on && b[i] {
                on = false;
            }
            out
        })
        .collect()
}

pub(crate) struct Sig {
    pub buy: Vec<bool>,
    pub sell: Vec<bool>,
    pub short: Vec<bool>,
    pub cover: Vec<bool>,
}

pub(crate) fn trade(dir: f64, e: usize, px: f64, exit: usize, exit_px: f64, reason: &str, open: bool) -> Trade {
    Trade { dir, legs: vec![Leg { i: e, px, w: 1.0 }], exit_i: exit, exit_px, reason: reason.into(), entry_kind: "close", signal_i: exit, open, exits: Vec::new() }
}

/// AmiBroker regular backtest at the close. side 1 long only, 2 short only, 3 long and short (reversing).
pub(crate) fn backtest(x: &Ctx, s: Sig, side: i64, from: usize, to: usize) -> Vec<Trade> {
    let buy = exrem(&s.buy, &s.sell);
    let sell = exrem(&s.sell, &buy);
    let short = exrem(&s.short, &s.cover);
    let cover = exrem(&s.cover, &short);
    let (longs, shorts) = (side != 2, side != 1);
    let mut out = Vec::new();
    let mut pos: Option<(f64, usize)> = None;
    for i in from..=to {
        if let Some((d, e)) = pos {
            let exit = if d > 0.0 { sell[i] || (side == 3 && short[i]) } else { cover[i] || (side == 3 && buy[i]) };
            if exit {
                out.push(trade(d, e, x.c[e], i, x.c[i], if d > 0.0 { "Sell signal" } else { "Cover signal" }, false));
                pos = None;
            }
        }
        if pos.is_none() {
            if longs && buy[i] {
                pos = Some((1.0, i));
            } else if shorts && short[i] {
                pos = Some((-1.0, i));
            }
        }
    }
    if let Some((d, e)) = pos {
        out.push(trade(d, e, x.c[e], to, x.c[to], "open", true));
    }
    out
}

// ---------------------------------------------------------------- buy on dip

const BIG: f64 = 9_999_999.0;
enum Dip {
    In,
    Out(usize),
    Sell,
}

/// The buy-on-dip loop as written. `filter` = Some(fltbuy) for the filter AFL (sorted profit levels, and a full sell
/// on every bar the filter is off), None for the no-filter AFL.
fn dip_events(x: &Ctx, buy_pct: f64, pb_pct: f64, filter: Option<&[bool]>) -> Vec<Option<Dip>> {
    let (c, l) = (&x.c, &x.l);
    let n = c.len();
    let lc: Vec<bool> = (0..n).map(|i| l[i] == c[i]).collect();
    let buyflt: Vec<bool> = (0..n).map(|i| i != 0 && c[i] > 0.0 && !lc[i]).collect();
    let fb = |i: usize| filter.map_or(true, |f| f[i]);
    let mut close = c.to_vec();
    let mut pb = vec![vec![BIG; n]; 6];
    let mut buylevel: Vec<f64> = (0..n).map(|i| close[i] * (1.0 - buy_pct / 100.0)).collect();
    let mut ev: Vec<Option<Dip>> = (0..n).map(|_| None).collect();
    let mut pos = 0usize;
    for i in 1..n {
        if pos == 0 && fb(i) {
            if c[i] > close[i - 1] {
                close[i] = c[i];
                pb[1][i] = pb[1][i - 1];
            } else if c[i] < buylevel[i - 1] && buyflt[i] {
                ev[i] = Some(Dip::In);
                close[i] = c[i];
                pb[1][i] = c[i] * (1.0 + pb_pct / 100.0);
                pos = 1;
            } else {
                close[i] = close[i - 1];
                pb[1][i] = pb[1][i - 1];
            }
        } else if pos >= 1 {
            // the AFL tests the levels from the highest (level 1) down; level 1 sells all, level k books lots k..pos
            let hit = (1..=pos).find(|&k| c[i] > pb[k][i - 1] && !lc[i]);
            match hit {
                Some(1) => {
                    ev[i] = Some(Dip::Sell);
                    close[i] = c[i];
                    pos = 0;
                }
                Some(k) => {
                    ev[i] = Some(Dip::Out(pos - k + 1));
                    close[i] = c[i];
                    for j in 1..k {
                        pb[j][i] = pb[j][i - 1];
                    }
                    pos = k - 1;
                }
                None if c[i] > close[i - 1] => {
                    close[i] = c[i];
                    for j in 1..=pos {
                        pb[j][i] = pb[j][i - 1];
                    }
                }
                None if pos < 5 && c[i] < buylevel[i - 1] && buyflt[i] && fb(i) => {
                    ev[i] = Some(Dip::In);
                    close[i] = c[i];
                    insert_level(&mut pb, i, pos, c[i] * (1.0 + pb_pct / 100.0), filter.is_some());
                    pos += 1;
                }
                None => {
                    close[i] = close[i - 1];
                    for j in 1..=pos {
                        pb[j][i] = pb[j][i - 1];
                    }
                }
            }
        }
        if filter.is_some() && !fb(i) {
            ev[i] = Some(Dip::Sell);
            for j in 1..=5 {
                pb[j][i] = BIG;
            }
            pos = 0;
            close[i] = c[i];
        }
        buylevel[i] = close[i] * (1.0 - buy_pct / 100.0);
    }
    ev
}

/// A new lot's profit level joins levels 1..pos (level 1 the highest). The filter AFL inserts it in order; the
/// no-filter AFL compares it only with level `pos` (and, at position 2, then level 2 with level 1), as written.
fn insert_level(pb: &mut [Vec<f64>], i: usize, pos: usize, new: f64, sorted: bool) {
    if sorted {
        let mut v: Vec<f64> = (1..=pos).map(|k| pb[k][i - 1]).collect();
        v.push(new);
        v.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));
        for (k, val) in v.into_iter().enumerate() {
            pb[k + 1][i] = val;
        }
        return;
    }
    for k in 1..pos {
        pb[k][i] = pb[k][i - 1];
    }
    pb[pos + 1][i] = new;
    if new > pb[pos][i - 1] {
        pb[pos][i] = new;
        pb[pos + 1][i] = pb[pos][i - 1];
        if pos == 2 {
            if pb[2][i] > pb[1][i - 1] {
                pb[1][i] = pb[2][i];
                pb[2][i] = pb[1][i - 1];
            } else {
                pb[1][i] = pb[1][i - 1];
            }
        }
    } else {
        pb[pos][i] = pb[pos][i - 1];
    }
}

/// One trade per position, from the first lot to the full exit. Each lot is a leg of 1/5 of the allocation; a
/// scale-out of k lots closes k of the trade's lots (fraction k / lots bought), the final sell the rest.
fn dip_trades(x: &Ctx, ev: &[Option<Dip>], from: usize, to: usize) -> Vec<Trade> {
    let mut out = Vec::new();
    let mut cur: Option<(Vec<Leg>, Vec<(usize, f64, usize)>)> = None;
    let mut held = 0usize;
    let finish = |mut legs: Vec<Leg>, pieces: Vec<(usize, f64, usize)>, open: bool| -> Trade {
        let lots = legs.len() as f64;
        // a trade that re-buys after scale-outs can buy more than 5 lots in all; its lots then share the allocation
        if lots > 5.0 {
            for l in legs.iter_mut() {
                l.w = 1.0 / lots;
            }
        }
        let exits: Vec<(usize, f64, f64)> = pieces.iter().map(|&(i, px, k)| (i, px, k as f64 / lots)).collect();
        let exit_px = exits.iter().map(|e| e.1 * e.2).sum::<f64>();
        let last = exits.last().map_or(0, |e| e.0);
        Trade { dir: 1.0, legs, exit_i: last, exit_px, reason: if open { "open".into() } else { "Profit booking".into() }, entry_kind: "close", signal_i: last, open, exits }
    };
    for i in from..=to {
        match &ev[i] {
            Some(Dip::In) => {
                let t = cur.get_or_insert_with(|| (Vec::new(), Vec::new()));
                t.0.push(Leg { i, px: x.c[i], w: 0.2 });
                held += 1;
            }
            Some(Dip::Out(k)) if held > 0 => {
                let k = (*k).min(held);
                if let Some(t) = cur.as_mut() {
                    t.1.push((i, x.c[i], k));
                }
                held -= k;
                if held == 0 {
                    let (legs, pieces) = cur.take().unwrap();
                    out.push(finish(legs, pieces, false));
                }
            }
            Some(Dip::Sell) if held > 0 => {
                let (legs, mut pieces) = cur.take().unwrap();
                pieces.push((i, x.c[i], held));
                held = 0;
                out.push(finish(legs, pieces, false));
            }
            _ => {}
        }
    }
    if let Some((legs, mut pieces)) = cur.take() {
        pieces.push((to, x.c[to], held));
        out.push(finish(legs, pieces, true));
    }
    out
}

// ---------------------------------------------------------------- run

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(0.0, |d| d.def));
    let gi = |k: &str| g(k).round() as i64;
    let gu = |k: &str| g(k).round().max(1.0) as usize;
    let c = &x.c;
    let n = c.len();
    if n == 0 || from > to || to >= n {
        return Some(Vec::new());
    }
    let side = gi("side");
    let bar_hl = || -> Bars { Bars { h: x.h.clone(), l: x.l.clone(), c: x.c.clone(), last: (0..n).collect(), group: (0..n).collect() } };
    // swing % calculation point: 1 close, 2 high/low (Up from the low, Dn from the high), 3 mid point
    let swing_bases = |kind: i64| -> (Vec<f64>, Vec<f64>) {
        match kind {
            2 => (x.l.clone(), x.h.clone()),
            3 => {
                let m: Vec<f64> = (0..n).map(|i| (x.h[i] + x.l[i]) / 2.0).collect();
                (m.clone(), m)
            }
            _ => (c.clone(), c.clone()),
        }
    };
    let sig = |buy: Vec<bool>, sell: Vec<bool>, short: Vec<bool>, cover: Vec<bool>| Sig { buy, sell, short, cover };
    Some(match id {
        SINGLE => {
            let tr = close_swing(c, g("pct"));
            let (up, dn): (Vec<bool>, Vec<bool>) = (tr.iter().map(|&t| t == 1).collect(), tr.iter().map(|&t| t == -1).collect());
            backtest(x, sig(up.clone(), dn.clone(), dn, up), side, from, to)
        }
        BULL | BEAR | BULL_BUY => {
            let tr = close_swing(c, g("pct"));
            let m = st_filter(x, tf_of(gi("flt_tf"), true), gi("flt_point"), gu("flt_period"), g("flt_factor"));
            let buy: Vec<bool> = (0..n).map(|i| tr[i] == 1 && match id { BULL => m[i] == 1, BEAR => m[i] == -1, _ => true }).collect();
            let sell: Vec<bool> = (0..n).map(|i| tr[i] == -1 || (id == BEAR && m[i] == 1)).collect();
            let short: Vec<bool> = (0..n).map(|i| m[i] == -1 && tr[i] == -1).collect();
            let cover: Vec<bool> = tr.iter().map(|&t| t == 1).collect();
            backtest(x, sig(buy, sell, short, cover), side, from, to)
        }
        DIFF | DIFF_LS => {
            let (ub, db) = swing_bases(gi("swing_point"));
            let (fb, fs): (Vec<bool>, Vec<bool>) = match gi("flt") {
                1 => {
                    let m = st_filter(x, tf_of(gi("flt_tf"), true), gi("flt_point"), gu("flt_period"), g("flt_factor"));
                    (m.iter().map(|&v| v == 1).collect(), m.iter().map(|&v| v == -1).collect())
                }
                2 => {
                    // diff AFL: FLTBuy = C > MA on the filter timeframe; FLTShort = not that
                    let (up, _) = sma_filter(x, tf_of(gi("flt_tf"), true), gu("sma"), false);
                    let dn = up.iter().map(|v| !v).collect();
                    (up, dn)
                }
                _ => (vec![true; n], vec![true; n]),
            };
            if id == DIFF {
                let (le, lx, se, sx) = (g("long_entry"), g("long_exit"), g("short_entry"), g("short_exit"));
                let (b1, _) = swing(c, &ub, &db, le);
                let (b2, _) = swing(c, &ub, &db, lx);
                let (s1, _) = swing(c, &ub, &db, se);
                let (s2, _) = swing(c, &ub, &db, sx);
                let buy = (0..n).map(|i| fb[i] && if le < lx { b1[i] == 1 } else { b1[i] == 1 && b2[i] == 1 }).collect();
                let sell = (0..n).map(|i| if le < lx { b1[i] == -1 && b2[i] == -1 } else { b2[i] == -1 }).collect();
                let short = (0..n).map(|i| fs[i] && if sx < se { s1[i] == -1 && s2[i] == -1 } else { s1[i] == -1 }).collect();
                let cover = (0..n).map(|i| if sx < se { s2[i] == 1 } else { s1[i] == 1 && s2[i] == 1 }).collect();
                backtest(x, sig(buy, sell, short, cover), side, from, to)
            } else {
                let (en, ex) = (g("entry"), g("exit"));
                let (b1, d1) = swing(c, &ub, &db, en);
                let (b2, _) = swing(c, &ub, &db, ex);
                let s = if en > ex {
                    let (up, dn) = (cross_up(c, &d1), cross_down(c, &d1));
                    sig((0..n).map(|i| fb[i] && up[i]).collect(), b2.iter().map(|&t| t == -1).collect(),
                        (0..n).map(|i| fs[i] && dn[i]).collect(), b2.iter().map(|&t| t == 1).collect())
                } else {
                    sig((0..n).map(|i| fb[i] && b1[i] == 1).collect(), (0..n).map(|i| b1[i] == -1 && b2[i] == -1).collect(),
                        (0..n).map(|i| fs[i] && b1[i] == -1 && b2[i] == -1).collect(), b1.iter().map(|&t| t == 1).collect())
                };
                backtest(x, s, side, from, to)
            }
        }
        SWING_ST => {
            let tr = close_swing(c, g("pct"));
            // this AFL's list is CLOSE | MID POINT | PIVOT
            let point = match gi("st_point") { 2 => 1, 3 => 3, _ => 2 };
            let (st, _) = supertrend(&calc_point(&bar_hl(), point), &x.h, &x.l, c, gu("st_period"), 1.0, g("st_factor"));
            let buy = tr.iter().map(|&t| t == 1).collect();
            let sell = st.iter().map(|&t| t == -1).collect();
            backtest(x, sig(buy, sell, vec![false; n], vec![false; n]), 1, from, to)
        }
        ST_SWING_LONG | ST_SWING_SHORT => {
            let sw = close_swing(c, g("pct"));
            let f = g("st_factor");
            let (st, down) = supertrend(&calc_point(&bar_hl(), gi("st_point")), &x.h, &x.l, c, gu("st_period"), f, f);
            let m = if gi("flt") == 1 { st_filter(x, tf_of(gi("flt_tf"), true), gi("st_point"), gu("flt_period"), g("flt_factor")) } else { vec![0; n] };
            let on = gi("flt") == 1;
            if id == ST_SWING_LONG {
                // Buy = Cross(C, Ref(ST down line, -1))
                let prev: Vec<Option<f64>> = (0..n).map(|i| if i == 0 { None } else { down[i - 1] }).collect();
                let cr = cross_up(c, &prev);
                let buy = (0..n).map(|i| cr[i] && (!on || m[i] == 1)).collect();
                let sell = (0..n).map(|i| sw[i] == -1 || st[i] == -1).collect();
                backtest(x, sig(buy, sell, vec![false; n], vec![false; n]), 1, from, to)
            } else {
                let short = (0..n).map(|i| st[i] == -1 && sw[i] == -1 && (!on || m[i] == -1)).collect();
                let cover = (0..n).map(|i| sw[i] == 1 || st[i] == 1).collect();
                backtest(x, sig(vec![false; n], vec![false; n], short, cover), 2, from, to)
            }
        }
        DIP => dip_trades(x, &dip_events(x, g("buy_pct"), g("pb_pct"), None), from, to),
        DIP_FLT => {
            let tf = tf_of(gi("flt_tf"), false);
            let fb: Vec<bool> = match gi("flt") {
                2 => sma_filter(x, tf, gu("sma"), true).0,
                3 => vec![true; n],
                _ => st_filter(x, tf, gi("flt_point"), gu("flt_period"), g("flt_factor")).iter().map(|&v| v == 1).collect(),
            };
            dip_trades(x, &dip_events(x, g("buy_pct"), g("pb_pct"), Some(&fb)), from, to)
        }
        REGIM => {
            let (bear, bull) = (g("bear_pct"), g("bull_pct"));
            // regime swing: HCL = HHV(C, BarCount), the highest close so far
            let mut hcl = c.clone();
            for i in 1..n {
                hcl[i] = hcl[i].max(hcl[i - 1]);
            }
            let mut up: Vec<f64> = (0..n).map(|i| c[i] * (1.0 + bull / 100.0)).collect();
            let mut dn: Vec<f64> = (0..n).map(|i| c[i] * (1.0 - bear / 100.0)).collect();
            let mut t1 = vec![1i8; n];
            for i in 1..n {
                t1[i] = if c[i] > up[i - 1] || c[i] > hcl[i - 1] { 1 } else if c[i] < dn[i - 1] { -1 } else { t1[i - 1] };
                if t1[i] > 0 && dn[i] < dn[i - 1] {
                    dn[i] = dn[i - 1];
                }
                if t1[i] < 0 && up[i] > up[i - 1] {
                    up[i] = up[i - 1];
                }
                if t1[i] < 0 && t1[i - 1] > 0 {
                    up[i] = hcl[i].min(c[i] * (1.0 + bull / 100.0));
                }
                if t1[i] > 0 && t1[i - 1] < 0 {
                    dn[i] = c[i] * (1.0 - bear / 100.0);
                }
            }
            let (ub, db) = swing_bases(gi("swing_point"));
            let (bt, _) = swing(c, &ub, &db, g("pct"));
            let tf = tf_of(gi("flt_tf"), true);
            let (fb, fs): (Vec<bool>, Vec<bool>) = match gi("flt") {
                2 => sma_filter(x, tf, gu("sma"), true),
                3 => (t1.iter().map(|&t| t == 1).collect(), t1.iter().map(|&t| t == -1).collect()),
                _ => {
                    let m = st_filter(x, tf, gi("flt_point"), gu("flt_period"), g("flt_factor"));
                    (m.iter().map(|&v| v == 1).collect(), m.iter().map(|&v| v == -1).collect())
                }
            };
            // AFL Prec(x, 2) truncates to 2 decimals
            let r2 = |v: f64| (v * 100.0).trunc();
            let upc: Vec<bool> = (0..n).map(|i| r2(x.o[i]) == r2(c[i]) && r2(x.o[i]) == r2(x.h[i])).collect();
            let loc: Vec<bool> = (0..n).map(|i| r2(x.o[i]) == r2(c[i]) && r2(x.o[i]) == r2(x.l[i])).collect();
            let buy = (0..n).map(|i| bt[i] == 1 && i != 0 && !upc[i]).collect();
            let sell = (0..n).map(|i| (bt[i] == -1 || i == 0) && !loc[i]).collect();
            let short = (0..n).map(|i| fs[i] && bt[i] == -1 && !loc[i] && i != 0).collect();
            let cover = (0..n).map(|i| (bt[i] == 1 || i == 0 || fb[i]) && !upc[i]).collect();
            backtest(x, sig(buy, sell, short, cover), side, from, to)
        }
        _ => return None,
    })
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let name = catalogue().into_iter().find(|d| d.id == id)?.name;
    Some((match id {
        SINGLE => "ytsw_single",
        BULL => "ytsw_bull",
        BEAR => "ytsw_bear",
        BULL_BUY => "ytsw_bull_buy",
        DIFF => "ytsw_diff",
        DIFF_LS => "ytsw_diff_ls",
        SWING_ST => "ytsw_swing_st",
        ST_SWING_LONG => "ytsw_st_swing_long",
        ST_SWING_SHORT => "ytsw_st_swing_short",
        DIP => "ytsw_dip",
        DIP_FLT => "ytsw_dip_flt",
        REGIM => "ytsw_regim",
        _ => return None,
    }, name, "% Swing"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;

    fn rows(closes: &[f64]) -> Vec<Row> {
        closes.iter().enumerate().map(|(k, &c)| Row { ticker: "T".into(), date: format!("2024-{:02}-{:02}", 1 + k / 28, 1 + k % 28), open: c, high: c * 1.01, low: c * 0.99, close: c, volume: None }).collect()
    }
    fn go(id: &str, r: &[Row], kv: &[(&str, f64)]) -> Vec<Trade> {
        let x = Ctx::new(r, r, &[]);
        let p: Params = kv.iter().map(|(k, v)| (k.to_string(), *v)).collect();
        run(id, &x, 0, r.len() - 1, &p).unwrap()
    }

    #[test]
    fn swing_flips_on_pct_moves() {
        // up 0..3, a fall of more than 5% from the trailing band flips down, a rise of more than 5% flips up
        let (tr, line) = swing(&[100.0, 103.0, 106.0, 99.0, 98.0, 104.0], &[100.0, 103.0, 106.0, 99.0, 98.0, 104.0], &[100.0, 103.0, 106.0, 99.0, 98.0, 104.0], 5.0);
        assert_eq!(tr, vec![1, 1, 1, -1, -1, 1]);
        assert!(line[3].is_some() && line[2].is_none() && line[5].is_none());
    }

    #[test]
    fn single_swing_long_and_reversing() {
        let r = rows(&[100.0, 103.0, 106.0, 99.0, 98.0, 104.0, 105.0]);
        let t = go(SINGLE, &r, &[("pct", 5.0)]);
        // long from bar 0 (trend starts up), sold at bar 3, bought again at bar 5, open at the end
        assert_eq!(t.iter().map(|t| (t.legs[0].i, t.exit_i, t.open)).collect::<Vec<_>>(), vec![(0, 3, false), (5, 6, true)]);
        let t = go(SINGLE, &r, &[("pct", 5.0), ("side", 3.0)]);
        assert_eq!(t.iter().map(|t| (t.dir, t.legs[0].i, t.exit_i)).collect::<Vec<_>>(), vec![(1.0, 0, 3), (-1.0, 3, 5), (1.0, 5, 6)]);
    }

    #[test]
    fn exrem_resets_on_the_same_bar() {
        // buy and sell both true on bars 1-3: every one of those bars keeps its buy
        let a = [false, true, true, true];
        let b = [true, true, true, true];
        assert_eq!(exrem(&a, &b), vec![false, true, true, true]);
        assert_eq!(exrem(&[true, true, false, true], &[false, false, true, false]), vec![true, false, false, true]);
    }

    #[test]
    fn expand_last_and_first_period() {
        let r: Vec<Row> = ["2024-01-30", "2024-01-31", "2024-02-01", "2024-02-29", "2024-03-01"]
            .iter().map(|d| Row { ticker: "T".into(), date: d.to_string(), open: 1.0, high: 1.0, low: 1.0, close: 1.0, volume: None }).collect();
        let x = Ctx::new(&r, &r, &[]);
        let b = compress(&x, 'M');
        assert_eq!(b.last, vec![1, 3, 4]);
        // month values 10, 20, 30: January bars carry January's own value; Feb 1 carries January's; Feb 29 its own
        assert_eq!(expand_last(&[10, 20, 30], &b), vec![10, 10, 10, 20, 30]);
    }

    #[test]
    fn cross_ignores_leading_nulls() {
        let line = [None, None, Some(10.0), None, Some(12.0), None];
        let c = [9.0, 9.0, 9.5, 11.0, 11.5, 13.0];
        // bar 3: the line ended (Null = -1e10 after the first value) -> cross; bar 2 is the line's first value -> none
        assert_eq!(cross_up(&c, &line), vec![false, false, false, true, false, true]);
    }

    #[test]
    fn buy_on_dip_scales_in_and_books() {
        // 100 then dips of more than 5%: buy at 94 and at 88 (second lot); 88 x 1.10 = 96.8 books the second lot,
        // 94 x 1.10 = 103.4 sells the rest
        let r = rows(&[100.0, 94.0, 88.0, 97.0, 104.0]);
        let t = go(DIP, &r, &[("buy_pct", 5.0), ("pb_pct", 10.0)]);
        assert_eq!(t.len(), 1);
        assert_eq!(t[0].legs.iter().map(|l| (l.i, l.px)).collect::<Vec<_>>(), vec![(1, 94.0), (2, 88.0)]);
        assert_eq!(t[0].exits, vec![(3, 97.0, 0.5), (4, 104.0, 0.5)]);
        assert!(!t[0].open);
    }

    #[test]
    fn every_strategy_runs() {
        let closes: Vec<f64> = (0..330).map(|k| 100.0 + 15.0 * ((k as f64) / 9.0).sin() + k as f64 * 0.05).collect();
        let r = rows(&closes);
        for id in IDS {
            let x = Ctx::new(&r, &r, &[]);
            let t = run(id, &x, 0, r.len() - 1, &Params::new()).unwrap();
            assert!(t.iter().all(|t| t.exit_i >= t.legs[0].i), "{id}");
        }
    }
}
