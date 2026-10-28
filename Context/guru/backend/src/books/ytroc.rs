//! YT Strategies · ROC: the team's AmiBroker AFLs in "Old Topics/ROC" that trade one index on daily bars (spot Nifty /
//! Bank Nifty; the team's workbooks are NIFTY_ / BANKNIFTY_ runs). Coded from the AFLs as written, with the AFLs'
//! own default settings; there is no book PDF. The ROC-ranking short AFL in the same folder trades a stock universe
//! and is not part of this module.
//!
//! The AFLs are bar loops that write Buy / Sell arrays with their own SellPrice; the AmiBroker backtester then
//! trades those arrays. The code follows that split:
//! - the loop runs over the whole history (as AmiBroker computes the AFL on every bar), and the backtester only
//!   opens trades in the test window, exits first and then enters on the same bar, and ignores a Sell while flat;
//! - the loop's forced sell on the data's last bar (`BarIndex() == LastValue(BarIndex())`) is treated as the
//!   position still being open; a position open at the window end is marked there at the close;
//! - the team's Supertrend: ATRVal = sum of the last `period` ATR(1) values / period (ATR(1) is Null on the first
//!   bar), Up = avg + factor x ATRVal, Dn = avg - factor x ATRVal, IsUp starts true, from bar `period`: up when
//!   C > Up[i-1], down when C < Dn[i-1]; in an up trend Dn only rises, in a down trend Up only falls, with no
//!   reset on a flip; ST = IsUp ? Dn : Up;
//! - a weekly / monthly ST uses the AFLs' pattern Ref(x, -1) on the period bars, TimeFrameExpand(expandFirst),
//!   Ref(x, +1) on the daily bars: every daily bar of a period sees the previous period's value, except the
//!   period's last bar, which sees its own; the data's last bar is Null;
//! - Null values before an array's first value take no part in comparisons (the condition is false).
#![allow(dead_code)]
use crate::books::ytsw::{compress, exrem, trade, true_range};
use crate::connors::{rsi, wilder, Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "yt_roc";
pub const X_DROP: &str = "ytroc_x_drop";
pub const X_DROP_CHECK: &str = "ytroc_x_drop_check";
pub const N_RED: &str = "ytroc_n_red_candles";
pub const IDS: [&str; 3] = [X_DROP, X_DROP_CHECK, N_RED];

fn def(id: &'static str, name: &'static str, dir: &'static str, rules: &'static [&'static str], notes: &'static str) -> StrategyDef {
    StrategyDef { id, name, group: "YT Strategies · ROC", session: 0, dir, status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim: None }
}

pub fn catalogue() -> Vec<StrategyDef> {
    vec![
        def(X_DROP, "X% ROC Drop", "Long", &[
            "E|Buy at the close when the close is more than the ROC threshold % (default 1) below the close N candles - 1 bars earlier (N default 1)",
            "F|Optional: every one of the last N candles fell more than the each-candle threshold % (default off); optional Supertrend entry filter (daily / weekly / monthly, default off)",
            "X|Sell at the close on any enabled exit: Nth counted candle (N candle / green candle / up close), profit booking %, hard SL %, trailing SL %, RSI crossing the threshold, RSI falling the drop value below its trailing high, ST filter turning against the trade, close crossing below the exit ST",
            "X|Live mode fills PB / HSL / TSL at the level, or at the open less the buffer % (not below the low) when the open is already through it",
            "X|If the entry condition is true on an exit bar the trade is carried forward once (no sell) and its levels restart from that close",
        ], "x%_drop_ROC.afl (ROC/AFLs), coded as the AFL is written. With the AFL defaults (N candles 1, ROC threshold 1) the entry compares the close with itself (Ref(C, -(N-1)) = C) and never fires, so the defaults give no trades; the team's runs set N above 1. The N-candle exit default 0 never fires (the count starts at 1). RSI exits use Cross(); all exits default off, so a trade ends only when the entry condition and another exit line up."),
        def(X_DROP_CHECK, "X% ROC Drop (% Close AFL Check version)", "Long", &[
            "E|As X% ROC Drop: close more than the ROC threshold % below the close N candles - 1 bars earlier",
            "F|Each-candle option needs only ONE of the last N candles to have fallen more than the threshold (the later AFL needs all of them)",
            "X|As X% ROC Drop, except the RSI exit is a level: RSI below (or above) the threshold on the bar, not a cross",
        ], "x%_drop_ROC.afl in ROC/AFLs/ROC - % Close AFL Check (the earlier copy, workbook 'ROC - % Close AFL'), coded as the AFL is written. Same defaults and the same never-firing default entry (N candles 1) as the later AFL; the two differ only in the each-candle Sum test and the RSI level vs cross exit."),
        def(N_RED, "N Red Candles", "Long", &[
            "E|Consecutive mode (default): the last N candles (default 2) each closed at or below their open (or the previous close) less the min difference %; look-back mode: at least N of the last look-back candles fell by the min difference %",
            "E|Buy at that close (or, with 'entry at one up candle', at the first higher close after the setup); no entry when the close equals the high, nor on the last bar",
            "F|Optional Supertrend filter (close at or above the ST, or below it) and SMA filter, each on the chart, weekly or monthly bars",
            "X|Sell at the close on the nth counted candle (next candle / green candle / higher close), or on profit booking %, hard SL %, or the close falling below the trailing ATR line; if the entry condition is true again on the exit bar, the count restarts and the trade continues",
        ], "N_Week_Red_V2_Extrafeature (V1).afl (ROC - C to C and O to C - Latest Work), coded as the AFL is written; the BankNifty _Index copy is the same code with other defaults. Defaults below the AFL's Param minimum are kept as AmiBroker keeps them: min difference 0 (so a close equal to the open counts as red), exit at nth candle 0 (with 'Next candle' the count exit never fires, with green / higher close it fires on the first bar that does not count), hard SL 0 (off). Exit at closing (default) checks PB / SL on the close. On an open = high = close bar the live-price sell is replaced by a sell at the close."),
    ]
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    Some(match id {
        X_DROP | X_DROP_CHECK => vec![
            p("st_flt", "ST entry filter (0 off, 1 on)", 0.0, 0.0, 1.0, 1.0),
            p("st_flt_tf", "ST filter timeframe (0 daily, 1 weekly, 2 monthly)", 0.0, 0.0, 2.0, 1.0),
            p("st_flt_period", "ST filter period", 5.0, 5.0, 5.0, 1.0),
            p("st_flt_factor", "ST filter factor", 1.0, 1.0, 6.0, 0.5),
            p("st_flt_avg", "ST filter average (0 Close, 1 Open, 2 High, 3 Low, 4 (O+C)/2, 5 (H+L)/2, 6 (H+L+C)/3, 7 (O+H+L+C)/4)", 5.0, 0.0, 7.0, 1.0),
            p("st_flt_side", "ST entry filter (0 bullish: close above the ST, 1 bearish)", 0.0, 0.0, 1.0, 1.0),
            p("each_on", "Each candle ROC condition (0 off, 1 on)", 0.0, 0.0, 1.0, 1.0),
            p("each_thr", "Each candle threshold % (1-bar fall)", 1.0, 0.0, 100.0, 0.1),
            p("n_candles", "N candles", 1.0, 1.0, 100.0, 1.0),
            p("roc_thr", "ROC threshold % (fall over the N candles)", 1.0, 0.0, 100.0, 0.1),
            p("st_exit", "ST exit (0 off, 1 on)", 0.0, 0.0, 1.0, 1.0),
            p("st_exit_tf", "ST exit timeframe (0 daily, 1 weekly, 2 monthly)", 0.0, 0.0, 2.0, 1.0),
            p("st_exit_period", "ST exit period", 5.0, 5.0, 5.0, 1.0),
            p("st_exit_factor", "ST exit factor", 1.0, 1.0, 6.0, 0.5),
            p("st_exit_avg", "ST exit average (0 Close, 1 Open, 2 High, 3 Low, 4 (O+C)/2, 5 (H+L)/2, 6 (H+L+C)/3, 7 (O+H+L+C)/4)", 5.0, 0.0, 7.0, 1.0),
            p("rsi_exit", "RSI exit (0 off, 1 on)", 0.0, 0.0, 1.0, 1.0),
            p("rsi_period", "RSI period (1 = no RSI)", 14.0, 1.0, 80.0, 1.0),
            p("rsi_side", "RSI exit when (0 below the threshold, 1 above)", 0.0, 0.0, 1.0, 1.0),
            p("rsi_thr", "RSI threshold", 1.0, 1.0, 100.0, 1.0),
            p("rsi_trail", "RSI trail exit (0 off, 1 on)", 0.0, 0.0, 1.0, 1.0),
            p("rsi_drop", "RSI drop value", 20.0, 1.0, 100.0, 1.0),
            p("exit_type", "Type of exit candle (0 N candle, 1 green candle, 2 up close)", 0.0, 0.0, 2.0, 1.0),
            p("exit_n", "N candles for exit (0 = the N-candle count never fires)", 0.0, 0.0, 500.0, 1.0),
            p("exit_at", "Exit at (0 live high / low, 1 close)", 0.0, 0.0, 1.0, 1.0),
            p("pb_pct", "PB % (0 off)", 0.0, 0.0, 50.0, 0.1),
            p("hsl_pct", "HSL % (0 off)", 0.0, 0.0, 20.0, 0.1),
            p("tsl_pct", "TSL % (0 off)", 0.0, 0.0, 100.0, 0.1),
            p("buffer_pct", "Buffer % (open through a level)", 0.0, 0.0, 10.0, 0.1),
        ],
        N_RED => vec![
            p("afl_type", "AFL type (0 consecutive, 1 look back)", 0.0, 0.0, 1.0, 1.0),
            p("entry_type", "Entry type (0 open to close, 1 previous close to close)", 0.0, 0.0, 1.0, 1.0),
            p("num_candles", "Number of red candles", 2.0, 1.0, 5.0, 1.0),
            p("lookback", "Look back candles (look back mode)", 2.0, 2.0, 10.0, 1.0),
            p("min_diff", "Min difference % per candle (AFL default 0, below its Param minimum 0.5)", 0.0, 0.0, 3.0, 0.25),
            p("one_up", "Entry at one up candle (0 off, 1 on)", 0.0, 0.0, 1.0, 1.0),
            p("exit_type", "Exit type (0 next candle, 1 green candle, 2 higher close)", 0.0, 0.0, 2.0, 1.0),
            p("exit_num", "Exit at nth candle of exit type (AFL default 0, below its Param minimum 1)", 0.0, 0.0, 5.0, 1.0),
            p("exit_at", "Exit at (0 live high / low, 1 closing)", 1.0, 0.0, 1.0, 1.0),
            p("pb_pct", "Profit booking % (0 off)", 0.0, 0.0, 50.0, 1.0),
            p("hsl_pct", "Hard SL % (0 off; AFL default 0, below its Param minimum 0.5)", 0.0, 0.0, 5.0, 0.5),
            p("buffer_pct", "Selling buffer %", 0.25, 0.0, 10.0, 0.05),
            p("atr_exit", "Exit using fresh ATR (0 off, 1 on)", 0.0, 0.0, 1.0, 1.0),
            p("atr_period", "ATR period", 5.0, 5.0, 25.0, 5.0),
            p("atr_mult", "ATR multiplier", 2.0, 0.1, 10.0, 0.1),
            p("st_on", "ST filter (0 no, 1 yes)", 0.0, 0.0, 1.0, 1.0),
            p("st_avg", "ST input (0 Open, 1 High, 2 Low, 3 Close, 4 (O+C)/2, 5 (H+L)/2, 6 (H+L+C)/3, 7 (O+H+L+C)/4)", 5.0, 0.0, 7.0, 1.0),
            p("st_tf", "ST timeframe (0 chart default, 1 daily, 2 weekly, 3 monthly; the AFL's intraday choices do not apply to daily bars)", 0.0, 0.0, 3.0, 1.0),
            p("st_period", "ST period", 5.0, 5.0, 25.0, 5.0),
            p("st_mult", "ST multiplier", 2.0, 0.5, 5.0, 0.1),
            p("st_side", "ST filter type (0 bearish: close below the ST, 1 bullish: close at or above it)", 1.0, 0.0, 1.0, 1.0),
            p("sma_on", "SMA filter (0 off, 1 on)", 0.0, 0.0, 1.0, 1.0),
            p("sma_tf", "SMA timeframe (0 chart default, 1 daily, 2 weekly, 3 monthly)", 0.0, 0.0, 3.0, 1.0),
            p("sma_period", "SMA period", 15.0, 1.0, 100.0, 1.0),
            p("sma_side", "Trade when (0 close above the SMA, 1 close below)", 0.0, 0.0, 1.0, 1.0),
        ],
        _ => return None,
    })
}

// ---------------------------------------------------------------- shared AFL pieces (also used by ytui)

/// Bars of the chosen timeframe (daily as they are, weeks Monday-Sunday, calendar months) with their opens.
pub(crate) struct Tf {
    pub o: Vec<f64>,
    pub h: Vec<f64>,
    pub l: Vec<f64>,
    pub c: Vec<f64>,
    pub last: Vec<usize>,
    pub group: Vec<usize>,
    pub daily: bool,
}

/// tf 0 daily (the chart interval: no compression, no shifts), 1 weekly, 2 monthly.
pub(crate) fn tf_bars(x: &Ctx, tf: i64) -> Tf {
    let ch = match tf {
        1 => 'W',
        2 => 'M',
        _ => 'D',
    };
    let b = compress(x, ch);
    let o = (0..b.c.len()).map(|g| x.o[if g == 0 { 0 } else { b.last[g - 1] + 1 }]).collect();
    Tf { o, h: b.h, l: b.l, c: b.c, last: b.last, group: b.group, daily: ch == 'D' }
}

/// The AFLs' higher-timeframe pattern: Ref(v, -1) on the period bars, expandFirst, Ref(+1) on the daily bars.
pub(crate) fn htf<T: Copy>(v: &[Option<T>], t: &Tf) -> Vec<Option<T>> {
    if t.daily {
        return v.to_vec();
    }
    let n = t.group.len();
    (0..n)
        .map(|i| {
            let g = t.group[i];
            if i + 1 == n {
                None
            } else if t.last[g] == i {
                v[g]
            } else if g == 0 {
                None
            } else {
                v[g - 1]
            }
        })
        .collect()
}

/// CalculateAvg by name, in the order Close, Open, High, Low, (O+C)/2, (H+L)/2, (H+L+C)/3, (O+H+L+C)/4.
pub(crate) fn avg_of(o: &[f64], h: &[f64], l: &[f64], c: &[f64], kind: i64) -> Vec<f64> {
    (0..c.len())
        .map(|i| match kind {
            1 => o[i],
            2 => h[i],
            3 => l[i],
            4 => (o[i] + c[i]) / 2.0,
            5 => (h[i] + l[i]) / 2.0,
            6 => (h[i] + l[i] + c[i]) / 3.0,
            7 => (o[i] + h[i] + l[i] + c[i]) / 4.0,
            _ => c[i],
        })
        .collect()
}

/// A ParamList index of an "Open|High|Low|Close|..." list as the avg_of kind.
pub(crate) fn open_first(idx: i64) -> i64 {
    match idx {
        0 => 1,
        1 => 2,
        2 => 3,
        3 => 0,
        k => k,
    }
}

/// satr / period: the mean of the last `period` ATR(1) values (Null until all exist).
pub(crate) fn mean_tr(h: &[f64], l: &[f64], c: &[f64], period: usize) -> Vec<Option<f64>> {
    let tr = true_range(h, l, c);
    let p = period.max(1);
    (0..c.len()).map(|i| if i + 1 < p { None } else { tr[i + 1 - p..=i].iter().try_fold(0.0, |s, v| v.map(|v| s + v)).map(|s| s / p as f64) }).collect()
}

/// The team's Supertrend line (no reset on a flip) on the given bars.
pub(crate) fn team_st(avg: &[f64], h: &[f64], l: &[f64], c: &[f64], period: usize, factor: f64) -> Vec<Option<f64>> {
    let n = c.len();
    let p = period.max(1);
    let atr = mean_tr(h, l, c, p);
    let mut up: Vec<Option<f64>> = (0..n).map(|i| atr[i].map(|a| avg[i] + a * factor)).collect();
    let mut dn: Vec<Option<f64>> = (0..n).map(|i| atr[i].map(|a| avg[i] - a * factor)).collect();
    let mut is_up = vec![true; n];
    // AFL loop values: Null is -1e10
    const NULL: f64 = -1e10;
    for i in p..n {
        is_up[i] = if c[i] > up[i - 1].unwrap_or(NULL) { true } else if c[i] < dn[i - 1].unwrap_or(NULL) { false } else { is_up[i - 1] };
        if is_up[i] {
            if let Some(d) = dn[i] {
                dn[i] = Some(d.max(dn[i - 1].unwrap_or(NULL)));
            }
        } else if let Some(u) = up[i] {
            let m = u.min(up[i - 1].unwrap_or(NULL));
            up[i] = if m == NULL { None } else { Some(m) };
        }
    }
    (0..n).map(|i| if is_up[i] { dn[i] } else { up[i] }).collect()
}

/// ST of the chosen timeframe and the matching close, both on daily bars (AFL ST_FilterPlot and Close_M).
pub(crate) fn st_daily(x: &Ctx, tf: i64, period: usize, factor: f64, avg_kind: i64) -> (Vec<Option<f64>>, Vec<Option<f64>>) {
    let t = tf_bars(x, tf);
    let st = team_st(&avg_of(&t.o, &t.h, &t.l, &t.c, avg_kind), &t.h, &t.l, &t.c, period, factor);
    let cl: Vec<Option<f64>> = t.c.iter().map(|&v| Some(v)).collect();
    (htf(&st, &t), htf(&cl, &t))
}

pub(crate) fn gt(a: Option<f64>, b: Option<f64>) -> bool {
    matches!((a, b), (Some(a), Some(b)) if a > b)
}
pub(crate) fn lt(a: Option<f64>, b: Option<f64>) -> bool {
    matches!((a, b), (Some(a), Some(b)) if a < b)
}

/// AFL Cross(a, b) where both sides may be Null (no cross unless both exist on this bar and the bar before).
pub(crate) fn cross(a: &[Option<f64>], b: &[Option<f64>]) -> Vec<bool> {
    (0..a.len())
        .map(|i| i > 0 && matches!((a[i], b[i], a[i - 1], b[i - 1]), (Some(a0), Some(b0), Some(a1), Some(b1)) if a0 > b0 && a1 <= b1))
        .collect()
}

/// AmiBroker RSI (Wilder), Null where it does not exist.
pub(crate) fn rsi_opt(c: &[f64], n: usize) -> Vec<Option<f64>> {
    rsi(c, n.max(1)).into_iter().map(|v| if v.is_nan() { None } else { Some(v) }).collect()
}

/// AmiBroker ATR(n): Wilder average of ATR(1), which is Null on the first bar.
pub(crate) fn atr_wilder(x: &Ctx, n: usize) -> Vec<Option<f64>> {
    let tr: Vec<f64> = true_range(&x.h, &x.l, &x.c).into_iter().map(|v| v.unwrap_or(f64::NAN)).collect();
    wilder(&tr, n.max(1)).into_iter().map(|v| if v.is_nan() { None } else { Some(v) }).collect()
}

pub(crate) const LAST_BAR: &str = "last bar";

/// Buy / Sell / Short / Cover arrays as the AFL loop leaves them, with the loop's exit prices and reasons.
pub(crate) struct Px {
    pub buy: Vec<bool>,
    pub sell: Vec<bool>,
    pub short: Vec<bool>,
    pub cover: Vec<bool>,
    pub sell_px: Vec<f64>,
    pub cover_px: Vec<f64>,
    pub sell_why: Vec<&'static str>,
    pub cover_why: Vec<&'static str>,
}

impl Px {
    pub fn new(n: usize) -> Px {
        Px { buy: vec![false; n], sell: vec![false; n], short: vec![false; n], cover: vec![false; n], sell_px: vec![0.0; n], cover_px: vec![0.0; n], sell_why: vec![""; n], cover_why: vec![""; n] }
    }
    pub fn sell(&mut self, i: usize, px: f64, why: &'static str) {
        self.sell[i] = true;
        self.sell_px[i] = px;
        self.sell_why[i] = why;
    }
    pub fn cover(&mut self, i: usize, px: f64, why: &'static str) {
        self.cover[i] = true;
        self.cover_px[i] = px;
        self.cover_why[i] = why;
    }
    /// The AFL's `Buy = ExRem(Buy, Sell); Sell = ExRem(Sell, Buy)` (and the same for Short / Cover).
    pub fn exrem_all(&mut self) {
        self.buy = exrem(&self.buy, &self.sell);
        self.sell = exrem(&self.sell, &self.buy);
        self.short = exrem(&self.short, &self.cover);
        self.cover = exrem(&self.cover, &self.short);
    }
}

/// AmiBroker regular backtest of the arrays (long and short allowed; an opposite entry reverses at the close):
/// entries only in from..=to, exit first then entry on the same bar, a Sell / Cover while flat is ignored.
pub(crate) fn backtest(x: &Ctx, s: &Px, from: usize, to: usize) -> Vec<Trade> {
    let n = x.c.len();
    let mut out = Vec::new();
    let mut pos: Option<(f64, usize)> = None;
    for i in from..=to.min(n - 1) {
        let mut reversed = None;
        if let Some((d, e)) = pos {
            if d > 0.0 {
                if s.sell[i] && !(i + 1 == n && s.sell_why[i] == LAST_BAR) {
                    out.push(trade(1.0, e, x.c[e], i, s.sell_px[i], s.sell_why[i], false));
                    pos = None;
                } else if s.short[i] {
                    out.push(trade(1.0, e, x.c[e], i, x.c[i], "Reversed by a short signal", false));
                    pos = None;
                    reversed = Some(-1.0);
                }
            } else if s.cover[i] && !(i + 1 == n && s.cover_why[i] == LAST_BAR) {
                out.push(trade(-1.0, e, x.c[e], i, s.cover_px[i], s.cover_why[i], false));
                pos = None;
            } else if s.buy[i] {
                out.push(trade(-1.0, e, x.c[e], i, x.c[i], "Reversed by a buy signal", false));
                pos = None;
                reversed = Some(1.0);
            }
        }
        if pos.is_none() {
            pos = match reversed {
                Some(d) => Some((d, i)),
                None if s.buy[i] => Some((1.0, i)),
                None if s.short[i] => Some((-1.0, i)),
                None => None,
            };
        }
    }
    if let Some((d, e)) = pos {
        out.push(trade(d, e, x.c[e], to, x.c[to], "open", true));
    }
    out
}

// ---------------------------------------------------------------- X% ROC drop

fn x_drop(x: &Ctx, g: &dyn Fn(&str) -> f64, check: bool) -> Px {
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let n = c.len();
    let gi = |k: &str| g(k).round() as i64;
    let gu = |k: &str| g(k).round().max(1.0) as usize;
    // entry: ((C / Ref(C, -N + 1)) - 1) x 100 < -thr
    let nc = gu("n_candles");
    let k = nc - 1;
    let thr = g("roc_thr");
    let mut entry: Vec<bool> = (0..n).map(|i| i >= k && (c[i] / c[i - k] - 1.0) * 100.0 < -thr).collect();
    if gi("each_on") == 1 {
        let et = g("each_thr");
        // ROC(C, 1) < -thr, Null on the first bar; Sum over the N candles
        for i in 0..n {
            let sum = if i + 1 < nc + 1 { None } else { Some((i + 1 - nc..=i).filter(|&j| (c[j] / c[j - 1] - 1.0) * 100.0 < -et).count()) };
            let ok = match sum {
                Some(s) if check => s != 0,
                Some(s) => s == nc,
                None => false,
            };
            entry[i] = entry[i] && ok;
        }
    }
    let st_flt = gi("st_flt") == 1;
    let bull: Vec<bool> = if st_flt {
        let (st, cm) = st_daily(x, gi("st_flt_tf"), gu("st_flt_period"), g("st_flt_factor"), gi("st_flt_avg"));
        (0..n).map(|i| gt(cm[i], st[i])).collect()
    } else {
        vec![false; n]
    };
    let flt_bear = gi("st_flt_side") == 1;
    let st_exit = gi("st_exit") == 1;
    let st_x: Vec<bool> = if st_exit {
        let (st, cm) = st_daily(x, gi("st_exit_tf"), gu("st_exit_period"), g("st_exit_factor"), gi("st_exit_avg"));
        cross(&st, &cm)
    } else {
        vec![false; n]
    };
    let rp = g("rsi_period").round() as i64;
    let rv: Vec<Option<f64>> = if rp > 1 { rsi_opt(c, rp as usize) } else { vec![None; n] };
    let rthr = Some(g("rsi_thr"));
    let thr_v = vec![rthr; n];
    let (above, below): (Vec<bool>, Vec<bool>) = if check {
        ((0..n).map(|i| gt(rv[i], rthr)).collect(), (0..n).map(|i| lt(rv[i], rthr)).collect())
    } else {
        (cross(&rv, &thr_v), cross(&thr_v, &rv))
    };
    let (rsi_exit, rsi_above) = (gi("rsi_exit") == 1, gi("rsi_side") == 1);
    let (rsi_trail, drop) = (gi("rsi_trail") == 1, g("rsi_drop"));
    let (etype, exit_n) = (gi("exit_type"), g("exit_n"));
    let close_basis = gi("exit_at") == 1;
    let (pb_pct, hsl_pct, tsl_pct, buf) = (g("pb_pct"), g("hsl_pct"), g("tsl_pct"), g("buffer_pct"));

    let mut s = Px::new(n);
    let mut pos = false;
    let mut flag = false;
    let mut count = 0.0;
    let (mut pb, mut hsl, mut tsl): (Option<f64>, Option<f64>, Option<f64>) = (None, None, None);
    let mut trail: Option<f64> = Some(0.0);
    for i in 1..n {
        if pos {
            let lo = if close_basis { c[i] } else { l[i] };
            let hi = if close_basis { c[i] } else { h[i] };
            let hsl_hit = hsl_pct > 0.0 && hsl.map_or(false, |v| lo <= v);
            let pb_hit = pb_pct > 0.0 && pb.map_or(false, |v| hi >= v);
            let tsl_hit = tsl_pct > 0.0 && tsl.map_or(false, |v| lo <= v);
            match etype {
                1 => {
                    if c[i] > o[i] {
                        count += 1.0
                    }
                }
                2 => {
                    if c[i] > c[i - 1] {
                        count += 1.0
                    }
                }
                _ => count += 1.0,
            }
            let main = count == exit_n;
            let rsi_hit = rsi_exit && if rsi_above { above[i] } else { below[i] };
            let flt_hit = st_flt && if flt_bear { bull[i] } else { !bull[i] };
            let stx_hit = st_exit && st_x[i];
            let trail_hit = rsi_trail && lt(rv[i], trail);
            let last = i + 1 == n;
            if last || main || hsl_hit || pb_hit || tsl_hit || rsi_hit || flt_hit || stx_hit || trail_hit {
                if entry[i] && !flag {
                    flag = true;
                } else {
                    let mut px = c[i];
                    let gap = |lvl: f64, through: bool| if through { l[i].max(o[i] * (1.0 - buf * 0.01)) } else { lvl };
                    if pb_hit && !close_basis {
                        let v = pb.unwrap();
                        px = gap(v, o[i] > v);
                    }
                    if hsl_hit && !close_basis {
                        let v = hsl.unwrap();
                        px = gap(v, o[i] < v);
                    }
                    if tsl_hit && !close_basis {
                        let v = tsl.unwrap();
                        px = gap(v, o[i] < v);
                    }
                    let why = if hsl_hit {
                        "Hard SL"
                    } else if tsl_hit {
                        "Trailing SL"
                    } else if pb_hit {
                        "Profit booking"
                    } else if main {
                        "Nth exit candle"
                    } else if rsi_hit {
                        "RSI threshold"
                    } else if trail_hit {
                        "RSI trail"
                    } else if flt_hit {
                        "ST filter"
                    } else if stx_hit {
                        "ST exit"
                    } else {
                        LAST_BAR
                    };
                    s.sell(i, px, why);
                    pos = false;
                    trail = Some(0.0);
                    pb = None;
                    hsl = None;
                    tsl = None;
                }
            }
            if tsl_pct > 0.0 {
                tsl = tsl.filter(|&t| t > 0.0).map(|t| (h[i] * (1.0 - tsl_pct * 0.01)).max(t));
            }
            if rsi_trail {
                trail = match (trail, rv[i].map(|r| r - drop)) {
                    (Some(a), Some(b)) => Some(a.max(b)),
                    (a, b) => a.or(b),
                };
            }
        }
        let flt_ok = !st_flt || if flt_bear { !bull[i] } else { bull[i] };
        if (!pos || flag) && entry[i] && flt_ok {
            if !flag {
                s.buy[i] = true;
            }
            hsl = (hsl_pct > 0.0).then(|| c[i] * (1.0 - hsl_pct * 0.01));
            pb = (pb_pct > 0.0).then(|| c[i] * (1.0 + pb_pct * 0.01));
            tsl = (tsl_pct > 0.0).then(|| c[i] * (1.0 - tsl_pct * 0.01));
            trail = rv[i].map(|r| r - drop);
            pos = true;
            count = 0.0;
            flag = false;
        }
    }
    s
}

// ---------------------------------------------------------------- N red candles

/// N red candles ST / SMA timeframe choice: 0 chart, 1 daily (both the daily chart), 2 weekly, 3 monthly.
fn nred_tf(code: i64) -> i64 {
    match code {
        2 => 1,
        3 => 2,
        _ => 0,
    }
}

fn n_red(x: &Ctx, g: &dyn Fn(&str) -> f64) -> Px {
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let n = c.len();
    let gi = |k: &str| g(k).round() as i64;
    let gu = |k: &str| g(k).round().max(1.0) as usize;
    let cc = gi("entry_type") == 1;
    let num = gi("num_candles").max(0);
    let md = g("min_diff");
    let cond1: Vec<bool> = if gi("afl_type") == 1 {
        let lb = gu("lookback");
        let red = |j: usize| -> Option<bool> {
            if cc {
                (j >= 1).then(|| (c[j] / c[j - 1] - 1.0) * 100.0 <= -md)
            } else {
                Some((c[j] / o[j] - 1.0) * 100.0 <= -md)
            }
        };
        (0..n)
            .map(|i| {
                if i + 1 < lb {
                    return false;
                }
                (i + 1 - lb..=i).map(red).try_fold(0i64, |s, v| v.map(|v| s + v as i64)).map_or(false, |s| s >= num)
            })
            .collect()
    } else {
        (0..n)
            .map(|i| {
                (0..num as usize).all(|k| {
                    if k > i {
                        return false;
                    }
                    let j = i - k;
                    let r = if cc {
                        if j == 0 {
                            return false;
                        }
                        c[j - 1]
                    } else {
                        o[j]
                    };
                    c[j] <= r * (1.0 - md * 0.01)
                })
            })
            .collect()
    };
    let fe: Vec<bool> = if gi("one_up") == 1 {
        let mut since: Option<usize> = None;
        let raw: Vec<bool> = (0..n)
            .map(|i| {
                if cond1[i] {
                    since = Some(i);
                }
                since.map_or(false, |s| i > s) && i >= 1 && c[i] > c[i - 1]
            })
            .collect();
        exrem(&raw, &cond1)
    } else {
        cond1
    };
    let st_ok: Vec<bool> = if gi("st_on") == 1 {
        let t = tf_bars(x, nred_tf(gi("st_tf")));
        let st = team_st(&avg_of(&t.o, &t.h, &t.l, &t.c, open_first(gi("st_avg"))), &t.h, &t.l, &t.c, gu("st_period"), g("st_mult"));
        let st = htf(&st, &t);
        let bull = gi("st_side") == 1;
        (0..n).map(|i| st[i].map_or(false, |v| if bull { c[i] >= v } else { c[i] < v })).collect()
    } else {
        vec![true; n]
    };
    let sma_ok: Vec<bool> = if gi("sma_on") == 1 {
        let t = tf_bars(x, nred_tf(gi("sma_tf")));
        let p = gu("sma_period");
        let ma: Vec<Option<f64>> = (0..t.c.len()).map(|k| (k + 1 >= p).then(|| t.c[k + 1 - p..=k].iter().sum::<f64>() / p as f64)).collect();
        let ma = htf(&ma, &t);
        let below = gi("sma_side") == 1;
        (0..n).map(|i| ma[i].map_or(false, |v| if below { c[i] < v } else { c[i] > v })).collect()
    } else {
        vec![true; n]
    };
    // DayofEntry: FinalEntry AND C > penny (0) AND NOT UC (H == C) AND not the last bar AND ST AND SMA
    let day_entry: Vec<bool> = (0..n).map(|i| fe[i] && c[i] > 0.0 && h[i] != c[i] && i + 1 != n && st_ok[i] && sma_ok[i]).collect();

    let etype = gi("exit_type");
    let exit_num = g("exit_num");
    let close_basis = gi("exit_at") == 1;
    let (pb_pct, sl_pct, buf) = (g("pb_pct"), g("hsl_pct"), g("buffer_pct"));
    let atr_on = gi("atr_exit") == 1;
    let atr = if atr_on { atr_wilder(x, gu("atr_period")) } else { vec![None; n] };
    let mult = g("atr_mult");

    let mut s = Px::new(n);
    let mut pos = false;
    let mut skip = false;
    let mut counter = 0.0;
    let (mut pb, mut hsl): (Option<f64>, Option<f64>) = (None, None);
    let mut atr_val: Vec<Option<f64>> = vec![None; n];
    for i in 1..n {
        atr_val[i] = atr_val[i - 1];
        let last = i + 1 == n;
        if pos && !skip {
            match etype {
                1 => {
                    if c[i] > o[i] {
                        counter += 1.0
                    }
                }
                2 => {
                    if c[i] > c[i - 1] {
                        counter += 1.0
                    }
                }
                _ => counter += 1.0,
            }
            let mut exit_cond = exit_num == counter;
            let sl_hit = sl_pct > 0.0 && hsl.map_or(false, |v| (if close_basis { c[i] } else { l[i] }) < v);
            let pb_hit = pb_pct > 0.0 && pb.map_or(false, |v| (if close_basis { c[i] } else { h[i] }) > v);
            let atr_hit = atr_on && atr_val[i - 1].map_or(false, |v| c[i] < v);
            if day_entry[i] && exit_cond {
                counter = 0.0;
                exit_cond = false;
            }
            if exit_cond || sl_hit || pb_hit || atr_hit || last {
                let ohc = o[i] == h[i] && h[i] == c[i];
                if !ohc || last {
                    let mut px = c[i];
                    let gap = |lvl: f64, through: bool| if through { l[i].max(o[i] * (1.0 - buf * 0.01)) } else { lvl };
                    if !close_basis && pb_pct > 0.0 && pb_hit {
                        let v = pb.unwrap();
                        px = gap(v, o[i] > v);
                    }
                    if !close_basis && sl_pct > 0.0 && sl_hit {
                        let v = hsl.unwrap();
                        px = gap(v, o[i] < v);
                    }
                    let why = if sl_hit {
                        "Hard SL"
                    } else if pb_hit {
                        "Profit booking"
                    } else if atr_hit {
                        "ATR trail"
                    } else if exit_cond {
                        "Nth exit candle"
                    } else {
                        LAST_BAR
                    };
                    s.sell(i, px, why);
                    pb = None;
                    hsl = None;
                    atr_val[i] = None;
                    pos = false;
                    counter = 0.0;
                } else {
                    skip = true;
                }
            }
        }
        if day_entry[i] && !pos {
            s.buy[i] = true;
            if pb_pct > 0.0 {
                pb = Some((1.0 + pb_pct * 0.01) * c[i]);
            }
            if sl_pct > 0.0 {
                hsl = Some((1.0 - sl_pct * 0.01) * c[i]);
            }
            if atr_on {
                atr_val[i] = atr[i].map(|a| c[i] - a * mult);
            }
            pos = true;
        }
        if atr_on && atr_val[i].is_some() {
            let new = atr[i].map(|a| c[i] - a * mult);
            atr_val[i] = match (atr_val[i - 1], new) {
                (Some(a), Some(b)) => Some(a.max(b)),
                (a, b) => b.or(a),
            };
        }
        if (skip && pos) || last {
            // the open = high = close sell, and the forced last-bar sell, at the close (it also overwrites a
            // live-price sell made on the last bar)
            let why = if s.sell[i] { s.sell_why[i] } else if skip { "Exit at the close (open = high = close bar)" } else { LAST_BAR };
            s.sell(i, c[i], why);
            pos = false;
            counter = 0.0;
            skip = false;
        }
    }
    s
}

// ---------------------------------------------------------------- run

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(0.0, |d| d.def));
    let n = x.c.len();
    if n == 0 || from > to || to >= n {
        return Some(Vec::new());
    }
    let s = match id {
        X_DROP => x_drop(x, &g, false),
        X_DROP_CHECK => x_drop(x, &g, true),
        N_RED => n_red(x, &g),
        _ => return None,
    };
    Some(backtest(x, &s, from, to))
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let name = catalogue().into_iter().find(|d| d.id == id)?.name;
    Some((match id {
        X_DROP => "ytroc_x_drop",
        X_DROP_CHECK => "ytroc_x_drop_check",
        N_RED => "ytroc_n_red",
        _ => return None,
    }, name, "ROC"))
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::Row;

    /// Weekday dates from 2024-01-01 (a Monday).
    pub(crate) fn dates(n: usize) -> Vec<String> {
        let mut out = Vec::new();
        let mut d: i64 = 19723; // 2024-01-01
        while out.len() < n {
            if (d + 4).rem_euclid(7) != 0 && (d + 4).rem_euclid(7) != 6 {
                // civil from days
                let z = d + 719468;
                let era = z.div_euclid(146097);
                let doe = z - era * 146097;
                let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
                let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
                let mp = (5 * doy + 2) / 153;
                let day = doy - (153 * mp + 2) / 5 + 1;
                let m = if mp < 10 { mp + 3 } else { mp - 9 };
                let y = yoe + era * 400 + if m <= 2 { 1 } else { 0 };
                out.push(format!("{y:04}-{m:02}-{day:02}"));
            }
            d += 1;
        }
        out
    }
    /// Bars from (open, high, low, close).
    pub(crate) fn bars(v: &[(f64, f64, f64, f64)]) -> Vec<Row> {
        let ds = dates(v.len());
        v.iter().zip(ds).map(|(&(o, h, l, c), d)| Row { ticker: "T".into(), date: d, open: o, high: h, low: l, close: c, volume: None }).collect()
    }
    pub(crate) fn wave(n: usize) -> Vec<Row> {
        let v: Vec<(f64, f64, f64, f64)> = (0..n)
            .map(|k| {
                let c = 100.0 + 15.0 * ((k as f64) / 9.0).sin() + k as f64 * 0.05;
                let o = c - 0.8 * ((k as f64) / 3.0).cos();
                (o, c.max(o) + 0.7, c.min(o) - 0.6, c)
            })
            .collect();
        bars(&v)
    }
    fn go(id: &str, r: &[Row], kv: &[(&str, f64)]) -> Vec<Trade> {
        let x = Ctx::new(r, r, &[]);
        let p: Params = kv.iter().map(|(k, v)| (k.to_string(), *v)).collect();
        run(id, &x, 0, r.len() - 1, &p).unwrap()
    }
    fn closes(cs: &[f64]) -> Vec<Row> {
        bars(&cs.iter().map(|&c| (c, c + 1.0, c - 1.0, c)).collect::<Vec<_>>())
    }

    #[test]
    fn x_drop_defaults_never_enter() {
        let r = closes(&[100.0, 98.0, 95.0, 90.0, 85.0, 80.0]);
        assert!(go(X_DROP, &r, &[]).is_empty());
    }

    #[test]
    fn x_drop_enters_on_window_fall_and_exits_on_count() {
        // N = 2: C / C[-1] - 1 < -1%: bar 2 (95 vs 98) enters; exit after 2 candles (bar 4), bar 4 not an entry
        let r = closes(&[100.0, 99.5, 95.0, 96.0, 97.0, 98.0]);
        let t = go(X_DROP, &r, &[("n_candles", 2.0), ("exit_n", 2.0)]);
        assert_eq!(t.iter().map(|t| (t.legs[0].i, t.exit_i, t.open)).collect::<Vec<_>>(), vec![(2, 4, false)]);
        assert_eq!(t[0].exit_px, 97.0);
    }

    #[test]
    fn x_drop_carries_forward_when_entry_repeats_on_exit_bar() {
        // entry bar 2; bar 3 is the count exit but also an entry -> carried, count restarts; exit on bar 4
        let r = closes(&[100.0, 99.5, 95.0, 92.0, 93.0, 94.0]);
        let t = go(X_DROP, &r, &[("n_candles", 2.0), ("exit_n", 1.0)]);
        assert_eq!(t.iter().map(|t| (t.legs[0].i, t.exit_i)).collect::<Vec<_>>(), vec![(2, 4)]);
    }

    #[test]
    fn x_drop_live_hsl_fills_at_level_or_gap() {
        // entry at 95 (bar 2), HSL 2% = 93.1; bar 3 low 92 reaches it with the open above -> fill at 93.1
        let r = bars(&[(100.0, 101.0, 99.0, 100.0), (100.0, 100.5, 99.0, 99.5), (99.0, 99.5, 94.0, 95.0), (95.0, 96.0, 92.0, 96.0), (96.0, 97.0, 95.0, 96.0)]);
        let t = go(X_DROP, &r, &[("n_candles", 2.0), ("hsl_pct", 2.0)]);
        assert_eq!((t[0].legs[0].i, t[0].exit_i), (2, 3));
        assert!((t[0].exit_px - 93.1).abs() < 1e-9);
        // gap below the level: open 90 -> max(low, open)
        let r = bars(&[(100.0, 101.0, 99.0, 100.0), (100.0, 100.5, 99.0, 99.5), (99.0, 99.5, 94.0, 95.0), (90.0, 96.0, 89.0, 96.0), (96.0, 97.0, 95.0, 96.0)]);
        let t = go(X_DROP, &r, &[("n_candles", 2.0), ("hsl_pct", 2.0)]);
        assert_eq!(t[0].exit_px, 90.0);
    }

    #[test]
    fn check_version_each_candle_needs_one_fall() {
        // N = 3: window fall from 100 to 95; only bar 4 fell more than 2% -> the check version enters, the later one not
        let r = closes(&[100.0, 100.0, 99.9, 99.0, 95.0, 96.0]);
        let kv = [("n_candles", 3.0), ("each_on", 1.0), ("each_thr", 2.0)];
        assert!(go(X_DROP, &r, &kv).is_empty());
        let t = go(X_DROP_CHECK, &r, &kv);
        assert_eq!(t[0].legs[0].i, 4);
        assert!(t[0].open);
    }

    #[test]
    fn n_red_two_red_candles_then_next_candle_exit() {
        // bars 1 and 2 close below their opens -> buy bar 2; exit_num 1 with next candle -> sell bar 3
        let r = bars(&[(100.0, 101.0, 99.0, 100.5), (101.0, 101.5, 98.0, 99.0), (99.0, 99.5, 96.0, 97.0), (97.0, 99.0, 96.5, 98.0), (98.0, 99.0, 97.5, 98.5)]);
        let t = go(N_RED, &r, &[("exit_num", 1.0)]);
        assert_eq!(t.iter().map(|t| (t.legs[0].i, t.exit_i, t.open)).collect::<Vec<_>>(), vec![(2, 3, false)]);
        // AFL default exit_num 0 with next candle: the count exit never fires -> open to the end
        let t = go(N_RED, &r, &[]);
        assert_eq!(t.iter().map(|t| (t.legs[0].i, t.open)).collect::<Vec<_>>(), vec![(2, true)]);
    }

    #[test]
    fn n_red_no_entry_when_close_is_high() {
        let r = bars(&[(100.0, 101.0, 99.0, 100.5), (101.0, 101.5, 98.0, 99.0), (99.0, 97.0, 96.0, 97.0), (97.0, 99.0, 96.5, 98.0)]);
        assert!(go(N_RED, &r, &[("exit_num", 1.0)]).is_empty());
    }

    #[test]
    fn team_st_ratchets_without_reset() {
        let r = wave(60);
        let x = Ctx::new(&r, &r, &[]);
        let st = team_st(&avg_of(&x.o, &x.h, &x.l, &x.c, 5), &x.h, &x.l, &x.c, 5, 1.0);
        assert!(st[..5].iter().all(|v| v.is_none()) && st[5..].iter().all(|v| v.is_some()));
        let t = tf_bars(&x, 1);
        let wk = htf(&t.c.iter().map(|&v| Some(v)).collect::<Vec<_>>(), &t);
        // first week Null except its last bar; the data's last bar is Null
        assert!(wk[0].is_none() && wk[4] == Some(x.c[4]) && wk[5] == Some(x.c[4]) && wk[59].is_none());
    }

    #[test]
    fn every_strategy_runs() {
        let r = wave(400);
        let x = Ctx::new(&r, &r, &[]);
        let sets: [&[(&str, f64)]; 4] = [
            &[],
            &[("n_candles", 3.0), ("exit_n", 2.0), ("st_flt", 1.0), ("st_flt_tf", 1.0), ("st_exit", 1.0), ("st_exit_tf", 2.0), ("rsi_exit", 1.0), ("rsi_thr", 40.0), ("rsi_trail", 1.0), ("tsl_pct", 2.0), ("hsl_pct", 3.0), ("pb_pct", 4.0)],
            &[("afl_type", 1.0), ("lookback", 4.0), ("one_up", 1.0), ("exit_num", 2.0), ("exit_type", 1.0), ("exit_at", 0.0), ("hsl_pct", 2.0), ("pb_pct", 3.0), ("atr_exit", 1.0), ("st_on", 1.0), ("st_tf", 2.0), ("sma_on", 1.0), ("sma_tf", 3.0)],
            &[("each_on", 1.0), ("n_candles", 2.0), ("exit_type", 2.0), ("exit_n", 1.0), ("rsi_period", 1.0)],
        ];
        for id in IDS {
            for kv in sets {
                let p: Params = kv.iter().map(|(k, v)| (k.to_string(), *v)).collect();
                let t = run(id, &x, 50, r.len() - 1, &p).unwrap();
                assert!(t.iter().all(|t| t.exit_i >= t.legs[0].i && t.legs[0].i >= 50 && t.exit_px.is_finite()), "{id}");
            }
            assert!(family_of(id).is_some() && params(id).is_some());
        }
        assert_eq!(catalogue().len(), IDS.len());
    }
}
