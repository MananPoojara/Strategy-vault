//! YT Strategies · Close above PMH: the team's AmiBroker AFLs in "Old Topics/Close above PMH"
//! (FuturesOrStocks_Long_UpTrending V1-V6), long only, on a daily spot index. Coded from the AFLs as written, with the
//! AFLs' own default settings; there is no book PDF. V7 is not coded: it uses variables it never defines.
//!
//! Two kinds of AFL:
//! - V1-V3 are written for the chart periodicity ("Weekly/Monthly"; the topic's name, previous month high, is the
//!   monthly run). They are run here on calendar months built from the daily bars (weekly as an option): every rule,
//!   the Supertrend filter and the ATR exit is evaluated on those period bars, and fills are at the period close,
//!   i.e. the close of the period's last daily bar.
//! - V4-V6 run on daily bars and take the previous month's high, month-end closes and month counters from
//!   `TimeFrameSet(inMonthly)`: the previous month's high reaches every day of the month (`expandFirst`), the monthly
//!   tests reach only the month's last daily bar (`expandPoint`).
//!
//! AmiBroker behaviour followed (see ytsw.rs): fills at the signal bar's close (or the AFL's SellPrice), no delay;
//! `ATR(1)` is Null on the first bar; a Null compared in the AFL loops counts as -1e10; the AFL's last-bar exit
//! (deadstocksellsig) is the open position marked at `to`, and no buy is taken on the run's last bar (`to`) or the
//! series' last bar: AmiBroker loads bars only up to the run's end, so the team's trade lists have no buy there.
//! The AFL's Supertrend (`CalculateAvg` from the team's include, assumed as in the team's other AFLs): the average
//! of the last `period` ATR(1) values around the chosen price (default Open), the loop from bar `period`, the lower
//! band only rising in an up trend and the upper only falling in a down trend, with no reset on a flip.
#![allow(dead_code)]
use crate::books::ytsw::{compress, cross_down, cross_up, trade, true_range, Bars};
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "yt_pmh";
pub const V1: &str = "ytpmh_v1";
pub const V1_ATR: &str = "ytpmh_v1_atr";
pub const V1_BEAR: &str = "ytpmh_v1_bear_continue";
pub const V2: &str = "ytpmh_v2";
pub const V2_BEAR: &str = "ytpmh_v2_bear_continue";
pub const V3: &str = "ytpmh_v3";
pub const V3_BEAR: &str = "ytpmh_v3_bear_continue";
pub const V4: &str = "ytpmh_v4";
pub const V5: &str = "ytpmh_v5";
pub const V6: &str = "ytpmh_v6";
pub const IDS: [&str; 10] = [V1, V1_ATR, V1_BEAR, V2, V2_BEAR, V3, V3_BEAR, V4, V5, V6];

fn def(id: &'static str, name: &'static str, rules: &'static [&'static str], notes: &'static str) -> StrategyDef {
    StrategyDef { id, name, group: "YT Strategies · Close above PMH", session: 0, dir: "Long", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim: None }
}

const ST_F: &str = "F|Supertrend filter on the same bars (default on; period 5, factor 2, on the Open; running: close above the ST line); Bearish filter type swaps the buy and sell sides";
const RISK_X: &str = "X|Optional hard stop, profit booking and trailing stop % (all 0 = off by default), at the close or live; ST sell signal (close below the ST line) when the filter is on";
const ATR_X: &str = "X|Optional ATR exit (off by default): close below a trailing line of price (Open) minus 1 x the 5-bar ATR(1) average";
const NOBUY: &str = "F|No buy on a bar whose high equals its close; a sell on a bar whose open, low and close are equal waits for the next bar";
const MODE: &str = "Bars are calendar months built from the daily bars (weekly as an option), as the AFL's monthly run; fills at the month's last close.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![
        def(V1, "Close above PMH V1 (Version 1)", &[
            "E|Buy at the month's close when it is at least the buffer % (default 0.5) above the previous month's high",
            ST_F, NOBUY,
            "X|Optional counter exits (all 0 = off): n months held, n green months, n up closes; when the count is reached on a month that meets the entry rule, the counts restart",
            RISK_X,
        ], "Close above PMH/Version 1/FuturesOrStocks_Long_UpTrending_V1.afl, coded as the AFL is written. Entry is tested before exits, so no sell and buy on the same month. This copy has no ATR exit. With live exits a trailing-stop hit still fills at the close, and a profit booking that gaps above the level fills at the open less the exit buffer (both as written)."),
        def(V1_ATR, "Close above PMH V1 (exit first, ATR exit)", &[
            "E|Buy at the month's close when it is at least the buffer % (default 0.5) above the previous month's high",
            ST_F, NOBUY,
            "X|Optional counter exits as Version 1 (counts restart on a month that meets the entry rule)",
            RISK_X, ATR_X,
        ], "Close above PMH/All AFLs/FuturesOrStocks_Long_UpTrending_V1.afl (From_2011 copy is the same rules), coded as the AFL is written. Exits are tested before entries, so a month can sell and buy again. Live trailing-stop hits fill at the close, as written."),
        def(V1_BEAR, "Close above PMH V1 (Bear Continue)", &[
            "E|Buy at the month's close when it is at or above the previous month's high (buffer default 0)",
            ST_F, NOBUY,
            "X|Optional counter exits as Version 1",
            RISK_X, ATR_X,
        ], "Close above PMH/All AFLs/Continue Bearish Trade in Bull Phase AFLs/FuturesOrStocks_Long_UpTrending_V1_Bear Continue.afl, coded as the AFL is written: Version 1 (entry tested first) with the ATR exit and buffer 0. Nothing in the code continues a trade across a filter flip; the name refers to how the team set it."),
        def(V2, "Close above PMH V2 (exit first)", &[
            "E|Buy at the month's close when it is at least the buffer % (default 0.5) above the previous month's high",
            ST_F, NOBUY,
            "X|Optional counter exits: when a count is reached on a month that meets the entry rule, wait for a month closing below the previous month's high",
            RISK_X, ATR_X,
        ], "Close above PMH/All AFLs/FuturesOrStocks_Long_UpTrending_V2.afl (From_2011 copy the same), coded as the AFL is written. Exits before entries. As written, the wait only holds while each later month still meets the entry rule: the counts stay reached, so the next month that does not meet it exits whether or not it closes below the previous high."),
        def(V2_BEAR, "Close above PMH V2 (entry first)", &[
            "E|Buy at the month's close when it is at least the buffer % (default 0.5) above the previous month's high",
            ST_F, NOBUY,
            "X|Optional counter exits with the wait as in V2",
            RISK_X, ATR_X,
        ], "Close above PMH/All AFLs/Continue Bearish Trade in Bull Phase AFLs/FuturesOrStocks_Long_UpTrending_V2_Bear Continue.afl (Version 2 copy the same rules), coded as the AFL is written: V2 with the entry tested before exits."),
        def(V3, "Close above PMH V3 (exit first)", &[
            "E|Buy at the month's close when it is at least the buffer % (default 0.5) above the previous month's high",
            ST_F, NOBUY,
            "X|Sell when the month closes below the previous month's high (toggle, default on)",
            RISK_X, ATR_X,
        ], "Close above PMH/All AFLs/FuturesOrStocks_Long_UpTrending_V3.afl, coded as the AFL is written. Exits before entries."),
        def(V3_BEAR, "Close above PMH V3 (entry first)", &[
            "E|Buy at the month's close when it is at least the buffer % (default 0.5) above the previous month's high",
            ST_F, NOBUY,
            "X|Sell when the month closes below the previous month's high (toggle, default on)",
            RISK_X, ATR_X,
        ], "Close above PMH/All AFLs/Continue Bearish Trade in Bull Phase AFLs/FuturesOrStocks_Long_UpTrending_V3_Bear Continue.afl (Version 3 copy the same rules), coded as the AFL is written: V3 with the entry tested before exits."),
        def(V4, "Buy above PMH V4 (daily)", &[
            "E|Buy at the daily close when it is at least the buffer % (default 0.5) above the previous month's high; not on a month's last trading day, not in the first month",
            "F|Optional Supertrend filter (default off) on daily, weekly or monthly bars (period 5, factor 2, on the Open, running)",
            NOBUY,
            "X|Optional monthly counter exits (all 0 = off), counted on month-end bars after the entry month: n months, n green months, n up closes; counts restart on a month closing at least the buffer % above its previous month's high",
            "X|Optional (default off): sell at the entry month's last bar if that month closes below the previous month's high",
            RISK_X, ATR_X,
        ], "Close above PMH/All AFLs/FuturesOrStocks_Long_UpTrending_V4.afl, coded as the AFL is written. With the AFL defaults nothing but the end of the data closes a trade. The month-close exit is only checked in the entry month (as written). Higher-timeframe ST signals reach a day on the period's last bar (the AFL's Ref/expandFirst/Ref steps); the AFL's intraday ST choices do not apply to daily data and 'Default' is the daily chart."),
        def(V5, "Buy above PMH V5 (daily, wait)", &[
            "E|Buy at the daily close when it is at least the buffer % (default 0.5) above the previous month's high; not on a month's last trading day",
            "F|Supertrend filter (default on; daily, period 5, factor 2, on the Open, running)",
            NOBUY,
            "X|Optional monthly counter exits as V4, but a count reached on a month meeting the entry rule waits for a month-end close below the previous month's high",
            "X|Optional (default off): sell at the entry month's last bar if that month closes below the previous month's high",
            RISK_X, ATR_X,
        ], "Close above PMH/All AFLs/FuturesOrStocks_Long_UpTrending_V5.afl, coded as the AFL is written. As in V2 the wait ends on the next month-end that does not meet the entry rule, since the counts stay reached."),
        def(V6, "Buy above PMH V6 (daily, month close exit)", &[
            "E|Buy at the daily close when it is at least the buffer % (default 0.5) above the previous month's high; not on a month's last trading day",
            "F|Supertrend filter (default on; daily, period 5, factor 2, on the Open, running)",
            NOBUY,
            "X|Sell at a month's last bar when that month closes below the previous month's high, except in the entry month",
            "X|Optional (default off): the same exit also in the entry month",
            RISK_X, ATR_X,
        ], "Close above PMH/All AFLs/FuturesOrStocks_Long_UpTrending_V6.afl, coded as the AFL is written. Its 'Exit when current close < previous high' toggle is declared but never used, so the month-close exit is always on (not offered as a setting)."),
    ]
}

const ST_AVG: &str = "ST price (1 Open, 2 High, 3 Low, 4 Close, 5 (Open+Close)/2, 6 (High+Low)/2, 7 (High+Low+Close)/3, 8 (Open+High+Low+Close)/4)";
const ATR_AVG: &str = "ATR price (1 Open, 2 High, 3 Low, 4 Close, 5 (Open+Close)/2, 6 (High+Low)/2, 7 (High+Low+Close)/3, 8 (Open+High+Low+Close)/4)";

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    if !IDS.contains(&id) {
        return None;
    }
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    let monthly = matches!(id, V1 | V1_ATR | V1_BEAR | V2 | V2_BEAR | V3 | V3_BEAR);
    let mut v = vec![];
    if monthly {
        v.push(p("tf", "Bar periodicity (1 monthly, 2 weekly)", 1.0, 1.0, 2.0, 1.0));
    }
    let (buf, bmin, bmax, bstep) = match id {
        V1 | V1_ATR => (0.5, 0.5, 2.5, 0.5),
        V1_BEAR => (0.0, 0.0, 2.5, 0.5),
        _ => (0.5, 0.0, 10.0, 0.01),
    };
    v.push(p("buffer_pct", "Minimum % of the close above the previous high", buf, bmin, bmax, bstep));
    v.extend([
        p("exit_at", "Exit at (0 close, 1 live)", 0.0, 0.0, 1.0, 1.0),
        p("hsl_pct", "HSL % (0 off)", 0.0, 0.0, 100.0, 0.1),
        p("pb_pct", "PB % (0 off)", 0.0, 0.0, 100.0, 0.1),
        p("tsl_pct", "TSL % (0 off)", 0.0, 0.0, 100.0, 0.1),
        p("exit_buffer", "Exit buffer % (live gap fills)", 0.25, 0.01, 1.0, 0.01),
    ]);
    let cmax = if matches!(id, V1 | V1_ATR) { 5.0 } else { 100.0 };
    match id {
        V3 | V3_BEAR => v.push(p("exit_below", "Exit when the close is below the previous high (0 off, 1 on)", 1.0, 0.0, 1.0, 1.0)),
        V6 => {}
        _ => v.extend([
            p("n_green", "N green candle exit (0 off)", 0.0, 0.0, cmax, 1.0),
            p("n_up", "N up close exit (0 off)", 0.0, 0.0, cmax, 1.0),
            p("n_candles", "N candles exit (0 off)", 0.0, 0.0, cmax, 1.0),
        ]),
    }
    match id {
        V4 | V5 => v.push(p("exit_if", "Exit if the entry month closes below the previous month's high (0 off, 1 on)", 0.0, 0.0, 1.0, 1.0)),
        V6 => v.push(p("exit_if", "Exit if a month (entry month too) closes below the previous month's high (0 off, 1 on)", 0.0, 0.0, 1.0, 1.0)),
        _ => {}
    }
    v.push(p("st_on", "ST filter (0 off, 1 on)", if id == V4 { 0.0 } else { 1.0 }, 0.0, 1.0, 1.0));
    if !monthly {
        v.push(p("st_tf", "ST time frame (1 daily, 2 weekly, 3 monthly)", 1.0, 1.0, 3.0, 1.0));
    }
    let fmax = if id == V1 { 5.0 } else { 100.0 };
    v.extend([
        p("st_period", "ST period", 5.0, 1.0, 100.0, 1.0),
        p("st_factor", "ST factor", 2.0, 1.0, fmax, 0.5),
        p("st_avg", ST_AVG, 1.0, 1.0, 8.0, 1.0),
        p("st_entry", "ST entry type (0 fresh cross, 1 running)", 1.0, 0.0, 1.0, 1.0),
        p("st_filter", "ST filter type (0 bullish, 1 bearish)", 0.0, 0.0, 1.0, 1.0),
    ]);
    if id != V1 {
        let (pmin, pmax, pstep, mmin, mmax, mstep) = if matches!(id, V1_ATR | V1_BEAR) { (5.0, 20.0, 5.0, 1.0, 5.0, 0.5) } else { (1.0, 100.0, 1.0, 0.1, 100.0, 0.1) };
        v.extend([
            p("atr_on", "ATR exit (0 off, 1 on)", 0.0, 0.0, 1.0, 1.0),
            p("atr_period", "ATR period", 5.0, pmin, pmax, pstep),
            p("atr_mult", "ATR multiplier", 1.0, mmin, mmax, mstep),
            p("atr_avg", ATR_AVG, 1.0, 1.0, 8.0, 1.0),
        ]);
    }
    Some(v)
}

// ---------------------------------------------------------------- bars and indicators

struct Ohlc {
    o: Vec<f64>,
    h: Vec<f64>,
    l: Vec<f64>,
    c: Vec<f64>,
}

/// Period bars (with their open) from the daily bars; tf 'M' calendar months, 'W' weeks.
fn period_bars(x: &Ctx, tf: char) -> (Ohlc, Bars) {
    let b = compress(x, tf);
    let o = (0..b.c.len()).map(|g| x.o[if g == 0 { 0 } else { b.last[g - 1] + 1 }]).collect();
    (Ohlc { o, h: b.h.clone(), l: b.l.clone(), c: b.c.clone() }, b)
}

/// AFL CalculateAvg choice.
fn avg_of(b: &Ohlc, kind: i64) -> Vec<f64> {
    (0..b.c.len())
        .map(|i| match kind {
            2 => b.h[i],
            3 => b.l[i],
            4 => b.c[i],
            5 => (b.o[i] + b.c[i]) / 2.0,
            6 => (b.h[i] + b.l[i]) / 2.0,
            7 => (b.h[i] + b.l[i] + b.c[i]) / 3.0,
            8 => (b.o[i] + b.h[i] + b.l[i] + b.c[i]) / 4.0,
            _ => b.o[i],
        })
        .collect()
}

/// Sum of the last `period` ATR(1) values / period (Null until they all exist).
fn satr(b: &Ohlc, period: usize) -> Vec<Option<f64>> {
    let tr = true_range(&b.h, &b.l, &b.c);
    let p = period.max(1);
    (0..tr.len()).map(|i| if i + 1 < p { None } else { tr[i + 1 - p..=i].iter().try_fold(0.0, |s, v| v.map(|v| s + v)).map(|s| s / p as f64) }).collect()
}

/// The AFLs' Supertrend line (STPlot): lower band in an up trend, upper band in a down trend.
fn st_line(b: &Ohlc, period: usize, factor: f64, avg: i64) -> Vec<Option<f64>> {
    let n = b.c.len();
    let a = satr(b, period);
    let av = avg_of(b, avg);
    let mut up: Vec<Option<f64>> = (0..n).map(|i| a[i].map(|s| av[i] + s * factor)).collect();
    let mut dn: Vec<Option<f64>> = (0..n).map(|i| a[i].map(|s| av[i] - s * factor)).collect();
    let mut is_up = vec![true; n];
    for i in period.max(1)..n {
        // a Null band compares as -1e10
        is_up[i] = if b.c[i] > up[i - 1].unwrap_or(-1e10) { true } else if b.c[i] < dn[i - 1].unwrap_or(-1e10) { false } else { is_up[i - 1] };
        if is_up[i] {
            dn[i] = opt_max(dn[i], dn[i - 1]);
        } else {
            up[i] = match (up[i], up[i - 1]) {
                (Some(a), Some(b)) => Some(a.min(b)),
                _ => None,
            };
        }
    }
    (0..n).map(|i| if is_up[i] { dn[i] } else { up[i] }).collect()
}

fn opt_max(a: Option<f64>, b: Option<f64>) -> Option<f64> {
    match (a, b) {
        (Some(a), Some(b)) => Some(a.max(b)),
        (a, None) => a,
        (None, b) => b,
    }
}

struct StCfg {
    on: bool,
    period: usize,
    factor: f64,
    avg: i64,
    running: bool,
    bearish: bool,
}

/// (STBuySig, STSellSig) on the given bars.
fn st_signals(b: &Ohlc, s: &StCfg) -> (Vec<bool>, Vec<bool>) {
    let n = b.c.len();
    if !s.on {
        return (vec![true; n], vec![false; n]);
    }
    let line = st_line(b, s.period, s.factor, s.avg);
    let (bull, bear) = if s.running {
        (
            (0..n).map(|i| line[i].map_or(false, |v| b.c[i] > v)).collect::<Vec<_>>(),
            (0..n).map(|i| line[i].map_or(false, |v| b.c[i] < v)).collect::<Vec<_>>(),
        )
    } else {
        (cross_up(&b.c, &line), cross_down(&b.c, &line))
    };
    if s.bearish {
        (bear, bull)
    } else {
        (bull, bear)
    }
}

/// ATR exit line: price minus the multiple of the ATR average (Null until it exists).
fn atr_line(b: &Ohlc, period: usize, mult: f64, avg: i64) -> Vec<Option<f64>> {
    let a = satr(b, period);
    let av = avg_of(b, avg);
    (0..b.c.len()).map(|i| a[i].map(|s| av[i] - s * mult)).collect()
}

// ---------------------------------------------------------------- the AFL loop

struct Risk {
    live: bool,
    hsl: f64,
    pb: f64,
    tsl: f64,
    buf: f64,
}

#[derive(Clone, Copy, PartialEq)]
enum CountMode {
    Reset,
    Wait,
}

/// Monthly context for the daily AFLs (V4-V6): month of each bar and the month's last bar.
struct Monthly<'a> {
    group: &'a [usize],
    is_last: &'a [bool],
    exit_if: bool,
}

enum Rule<'a> {
    /// n candles / green / up close counters (V1, V2 on period bars; V4, V5 on month ends).
    Count {
        mode: CountMode,
        n: [i64; 3],
        up: &'a [bool],
        green: &'a [bool],
        entry: &'a [bool],
        below: &'a [bool],
        monthly: Option<Monthly<'a>>,
        st: [i64; 3],
        wait: bool,
        check: bool,
        check1: bool,
        entry_group: usize,
    },
    /// close below the previous high (V3).
    Below(&'a [bool]),
    /// month close below the previous month's high, not in the entry month (V6).
    MonthClose { below: &'a [bool], m: Monthly<'a>, check1: bool, entry_group: usize },
}

impl Rule<'_> {
    fn reset(&mut self, i: usize) {
        match self {
            Rule::Count { st, wait, check, check1, entry_group, monthly, .. } => {
                *st = [0; 3];
                *wait = false;
                *check = false;
                *check1 = false;
                *entry_group = monthly.as_ref().map_or(0, |m| m.group[i]);
            }
            Rule::Below(_) => {}
            Rule::MonthClose { m, check1, entry_group, .. } => {
                *check1 = false;
                *entry_group = m.group[i];
            }
        }
    }
    fn step(&mut self, i: usize) -> bool {
        match self {
            Rule::Count { mode, n, up, green, entry, below, monthly, st, wait, check, check1, entry_group } => {
                if let Some(m) = monthly {
                    if m.group[i] == *entry_group {
                        *st = [0; 3];
                        if m.exit_if && below[i] && m.is_last[i] {
                            *check1 = true;
                        }
                        return *check || *check1;
                    }
                    if !m.is_last[i] {
                        return *check || *check1;
                    }
                }
                st[0] += 1;
                st[1] += green[i] as i64;
                st[2] += up[i] as i64;
                *check = (n[0] > 0 && st[0] >= n[0]) || (n[1] > 0 && st[1] >= n[1]) || (n[2] > 0 && st[2] >= n[2]);
                if *check && entry[i] {
                    if *mode == CountMode::Reset {
                        *st = [0; 3];
                    } else {
                        *wait = true;
                    }
                    *check = false;
                } else if *wait && below[i] {
                    *wait = false;
                    *check = true;
                }
                *check || *check1
            }
            Rule::Below(b) => b[i],
            Rule::MonthClose { below, m, check1, entry_group } => {
                if m.exit_if && m.is_last[i] && below[i] {
                    *check1 = true;
                }
                (below[i] && m.group[i] != *entry_group) || *check1
            }
        }
    }
    fn name(&self) -> &'static str {
        match self {
            Rule::Count { monthly: Some(_), .. } => "Monthly counter / month close exit",
            Rule::Count { .. } => "Counter exit",
            Rule::Below(_) => "Close below previous high",
            Rule::MonthClose { .. } => "Month close below previous month high",
        }
    }
}

struct Loop<'a> {
    b: &'a Ohlc,
    entry: &'a [bool],
    st_buy: &'a [bool],
    st_sell: &'a [bool],
    atr: Option<&'a [Option<f64>]>,
    can_buy: &'a [bool],
    /// the series' last bar in this bar space (the AFL's deadstock bar), if processed
    data_last: Option<usize>,
    /// bars 1..end are processed
    end: usize,
    entry_first: bool,
    risk: Risk,
}

/// (entry bar, entry price, exit bar, exit price, reason); the open position at the end, if any.
type Done = (usize, f64, usize, f64, &'static str);

#[derive(Default)]
struct State {
    pos: Option<(usize, f64)>,
    skip: bool,
    hsl: Option<f64>,
    pb: Option<f64>,
    tsl: Option<f64>,
    atrl: Option<f64>,
    out: Vec<Done>,
}

impl Loop<'_> {
    fn olc(&self, i: usize) -> bool {
        self.b.l[i] == self.b.c[i] && self.b.l[i] == self.b.o[i]
    }
    fn try_buy(&self, s: &mut State, rule: &mut Rule, i: usize) -> bool {
        let b = self.b;
        if s.pos.is_some() || !self.can_buy[i] || b.h[i] == b.c[i] || !self.entry[i] || !self.st_buy[i] {
            return false;
        }
        let px = b.c[i];
        let r = &self.risk;
        s.pos = Some((i, px));
        s.hsl = (r.hsl > 0.0).then(|| px * (1.0 - r.hsl * 0.01));
        s.pb = (r.pb > 0.0).then(|| px * (1.0 + r.pb * 0.01));
        s.tsl = (r.tsl > 0.0).then(|| px * (1.0 - r.tsl * 0.01));
        s.atrl = self.atr.and_then(|a| a[i]);
        rule.reset(i);
        true
    }
    fn close(s: &mut State, i: usize, px: f64, reason: &'static str) {
        if let Some((e, epx)) = s.pos.take() {
            s.out.push((e, epx, i, px, reason));
        }
        s.skip = false;
        s.hsl = None;
        s.pb = None;
        s.tsl = None;
        s.atrl = None;
    }
    fn exit_block(&self, s: &mut State, rule: &mut Rule, i: usize, tsl_prev: Option<f64>, atr_prev: Option<f64>) {
        let b = self.b;
        let r = &self.risk;
        let sig = rule.step(i);
        if !self.olc(i) || Some(i) == self.data_last {
            let (lo, hi) = if r.live { (b.l[i], b.h[i]) } else { (b.c[i], b.c[i]) };
            let hsl_hit = r.hsl > 0.0 && s.hsl.map_or(false, |v| lo < v);
            let pb_hit = r.pb > 0.0 && s.pb.map_or(false, |v| hi > v);
            let tsl_hit = tsl_prev.map_or(false, |v| v != 0.0 && lo < v);
            let atr_hit = self.atr.is_some() && atr_prev.map_or(false, |v| b.c[i] < v);
            if hsl_hit || pb_hit || tsl_hit || atr_hit || self.st_sell[i] || sig {
                let mut px = b.c[i];
                if r.live && hsl_hit {
                    let v = s.hsl.unwrap();
                    px = if b.o[i] < v { b.l[i].max(b.o[i] * (1.0 - r.buf * 0.01)) } else { v };
                }
                if r.live && pb_hit {
                    let v = s.pb.unwrap();
                    px = if b.o[i] > v { b.l[i].max(b.o[i] * (1.0 - r.buf * 0.01)) } else { v };
                }
                let reason = if hsl_hit {
                    "Hard stop"
                } else if pb_hit {
                    "Profit booking"
                } else if tsl_hit {
                    "Trailing stop"
                } else if atr_hit {
                    "ATR exit"
                } else if self.st_sell[i] {
                    "ST sell"
                } else {
                    rule.name()
                };
                Self::close(s, i, px, reason);
            }
        } else {
            s.skip = true;
        }
        if s.pos.is_some() {
            if let Some(t) = tsl_prev.filter(|&t| t != 0.0) {
                s.tsl = Some((b.h[i] * (1.0 - r.tsl * 0.01)).max(t));
            }
        }
        s.atrl = match (s.pos.is_some(), self.atr) {
            (true, Some(a)) => opt_max(a[i], atr_prev),
            _ => None,
        };
    }
    fn run(&self, rule: &mut Rule) -> (Vec<Done>, Option<(usize, f64)>) {
        let mut s = State::default();
        for i in 1..self.end {
            let (tsl_prev, atr_prev) = (s.tsl, s.atrl);
            if self.entry_first {
                if !self.try_buy(&mut s, rule, i) && s.pos.is_some() && !s.skip {
                    self.exit_block(&mut s, rule, i, tsl_prev, atr_prev);
                }
            } else if s.pos.is_some() && !s.skip {
                self.exit_block(&mut s, rule, i, tsl_prev, atr_prev);
            }
            if s.skip && !self.olc(i) {
                Self::close(&mut s, i, self.b.c[i], "Exit after an open = low = close bar");
            }
            if !self.entry_first {
                self.try_buy(&mut s, rule, i);
            }
        }
        (s.out, s.pos)
    }
}

// ---------------------------------------------------------------- run

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(0.0, |d| d.def));
    let gi = |k: &str| g(k).round() as i64;
    let gu = |k: &str| g(k).round().max(1.0) as usize;
    let n = x.c.len();
    if n == 0 || from > to || to >= n {
        return Some(Vec::new());
    }
    let buf = g("buffer_pct");
    let risk = Risk { live: gi("exit_at") == 1, hsl: g("hsl_pct"), pb: g("pb_pct"), tsl: g("tsl_pct"), buf: g("exit_buffer") };
    let st = StCfg { on: gi("st_on") == 1, period: gu("st_period"), factor: g("st_factor"), avg: gi("st_avg"), running: gi("st_entry") == 1, bearish: gi("st_filter") == 1 };
    let atr_on = id != V1 && gi("atr_on") == 1;
    let counts = [gi("n_candles"), gi("n_green"), gi("n_up")];
    let to_trades = |done: Vec<Done>, open: Option<(usize, f64)>, map: &dyn Fn(usize) -> usize| -> Vec<Trade> {
        let mut out: Vec<Trade> = done.into_iter().map(|(e, epx, xi, xpx, why)| trade(1.0, map(e), epx, map(xi), xpx, why, false)).collect();
        if let Some((e, epx)) = open {
            out.push(trade(1.0, map(e), epx, to, x.c[to], "open", true));
        }
        out
    };

    Some(match id {
        V1 | V1_ATR | V1_BEAR | V2 | V2_BEAR | V3 | V3_BEAR => {
            let (b, bars) = period_bars(x, if gi("tf") == 2 { 'W' } else { 'M' });
            let nb = b.c.len();
            let prev_h = |k: usize| if k == 0 || b.h[k - 1] == 0.0 { None } else { Some(b.h[k - 1]) };
            let entry: Vec<bool> = (0..nb).map(|k| prev_h(k).map_or(false, |ph| 100.0 * (b.c[k] - ph) / ph >= buf)).collect();
            let below_on = !matches!(id, V3 | V3_BEAR) || gi("exit_below") == 1;
            let below: Vec<bool> = (0..nb).map(|k| below_on && prev_h(k).map_or(false, |ph| b.c[k] < ph)).collect();
            let up: Vec<bool> = (0..nb).map(|k| k > 0 && b.c[k - 1] != 0.0 && b.c[k] > b.c[k - 1]).collect();
            let green: Vec<bool> = (0..nb).map(|k| b.c[k] > b.o[k]).collect();
            let (st_buy, st_sell) = st_signals(&b, &st);
            let atr = atr_on.then(|| atr_line(&b, gu("atr_period"), g("atr_mult"), gi("atr_avg")));
            // the AFL's deadstock rule (no buy on BarIndex() == LastValue(BarIndex())): AmiBroker loads bars only up to
            // the run's end, so its last bar is the run's last bar (the team's trade lists have no buy there)
            let can_buy: Vec<bool> = (0..nb).map(|k| bars.last[k] >= from && bars.last[k] != n - 1 && bars.last[k] != to).collect();
            let end = bars.last.iter().take_while(|&&d| d <= to).count();
            let lp = Loop {
                b: &b,
                entry: &entry,
                st_buy: &st_buy,
                st_sell: &st_sell,
                atr: atr.as_deref(),
                can_buy: &can_buy,
                data_last: (bars.last[nb - 1] == n - 1 && end == nb).then_some(nb - 1),
                end,
                entry_first: matches!(id, V1 | V1_BEAR | V2_BEAR | V3_BEAR),
                risk,
            };
            let mut rule = match id {
                V3 | V3_BEAR => Rule::Below(&below),
                _ => Rule::Count {
                    mode: if matches!(id, V2 | V2_BEAR) { CountMode::Wait } else { CountMode::Reset },
                    n: counts,
                    up: &up,
                    green: &green,
                    entry: &entry,
                    below: &below,
                    monthly: None,
                    st: [0; 3],
                    wait: false,
                    check: false,
                    check1: false,
                    entry_group: 0,
                },
            };
            let (done, open) = lp.run(&mut rule);
            to_trades(done, open, &|k| bars.last[k])
        }
        V4 | V5 | V6 => {
            let d = Ohlc { o: x.o.clone(), h: x.h.clone(), l: x.l.clone(), c: x.c.clone() };
            let (m, mb) = period_bars(x, 'M');
            let grp = &mb.group;
            let is_last: Vec<bool> = (0..n).map(|i| mb.last[grp[i]] == i).collect();
            let pmh = |gm: usize| if gm == 0 || m.h[gm - 1] == 0.0 { None } else { Some(m.h[gm - 1]) };
            // month-end values (expandPoint): only on the month's last bar
            let at_end = |f: &dyn Fn(usize) -> bool| -> Vec<bool> { (0..n).map(|i| is_last[i] && f(grp[i])).collect() };
            let entry1 = at_end(&|gm| pmh(gm).map_or(false, |ph| 100.0 * (m.c[gm] - ph) / ph >= buf));
            let below = at_end(&|gm| pmh(gm).map_or(false, |ph| m.c[gm] < ph));
            let up = at_end(&|gm| gm > 0 && m.c[gm - 1] != 0.0 && m.c[gm] > m.c[gm - 1]);
            let green = at_end(&|gm| m.c[gm] > m.o[gm]);
            let entry: Vec<bool> = (0..n).map(|i| !is_last[i] && pmh(grp[i]).map_or(false, |ph| (x.c[i] - ph) / ph * 100.0 >= buf)).collect();
            let (st_buy, st_sell) = match gi("st_tf") {
                tf @ (2 | 3) if st.on => {
                    let (sb, sbars) = period_bars(x, if tf == 2 { 'W' } else { 'M' });
                    let (pb, ps) = st_signals(&sb, &st);
                    // Ref(sig, -1) -> expandFirst -> Ref(sig, 1): the period's own value on its last bar, the previous
                    // period's value on its other bars, Null (false) before that and on the series' last bar
                    let ex = |v: &[bool]| -> Vec<bool> {
                        (0..n)
                            .map(|i| {
                                let k = sbars.group[i];
                                if i == n - 1 {
                                    false
                                } else if sbars.last[k] == i {
                                    v[k]
                                } else {
                                    k > 0 && v[k - 1]
                                }
                            })
                            .collect()
                    };
                    (ex(&pb), ex(&ps))
                }
                _ => st_signals(&d, &st),
            };
            let atr = atr_on.then(|| atr_line(&d, gu("atr_period"), g("atr_mult"), gi("atr_avg")));
            // deadstock rule: no buy on the run's last bar (AmiBroker's LastValue(BarIndex()) is the run end)
            let can_buy: Vec<bool> = (0..n).map(|i| i >= from && i != n - 1 && i != to).collect();
            let lp = Loop {
                b: &d,
                entry: &entry,
                st_buy: &st_buy,
                st_sell: &st_sell,
                atr: atr.as_deref(),
                can_buy: &can_buy,
                data_last: (to == n - 1).then_some(n - 1),
                end: to + 1,
                entry_first: true,
                risk,
            };
            let mth = Monthly { group: grp, is_last: &is_last, exit_if: gi("exit_if") == 1 };
            let mut rule = if id == V6 {
                Rule::MonthClose { below: &below, m: mth, check1: false, entry_group: 0 }
            } else {
                Rule::Count {
                    mode: if id == V5 { CountMode::Wait } else { CountMode::Reset },
                    n: counts,
                    up: &up,
                    green: &green,
                    entry: &entry1,
                    below: &below,
                    monthly: Some(mth),
                    st: [0; 3],
                    wait: false,
                    check: false,
                    check1: false,
                    entry_group: 0,
                }
            };
            let (done, open) = lp.run(&mut rule);
            to_trades(done, open, &|i| i)
        }
        _ => return None,
    })
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    if !IDS.contains(&id) {
        return None;
    }
    let d = catalogue().into_iter().find(|d| d.id == id)?;
    Some((d.id, d.name, "Close above PMH"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;

    fn row(date: &str, o: f64, h: f64, l: f64, c: f64) -> Row {
        Row { ticker: "T".into(), date: date.into(), open: o, high: h, low: l, close: c, volume: None }
    }
    fn go(id: &str, r: &[Row], kv: &[(&str, f64)]) -> Vec<(usize, usize, bool)> {
        let x = Ctx::new(r, r, &[]);
        let p: Params = kv.iter().map(|(k, v)| (k.to_string(), *v)).collect();
        run(id, &x, 0, r.len() - 1, &p).unwrap().iter().map(|t| (t.legs[0].i, t.exit_i, t.open)).collect()
    }
    /// One daily bar per month, so the month bars are these bars.
    fn months(b: &[(f64, f64, f64, f64)]) -> Vec<Row> {
        b.iter().enumerate().map(|(k, &(o, h, l, c))| row(&format!("{}-{:02}-15", 2020 + k / 12, 1 + k % 12), o, h, l, c)).collect()
    }

    #[test]
    fn v1_counter_exit_restarts_on_an_entry_month() {
        let r = months(&[
            (100.0, 101.0, 99.0, 100.0),
            (101.0, 103.5, 100.0, 103.0), // 2% above 101: buy
            (103.0, 104.0, 101.0, 102.0), // 1 month held
            (102.0, 107.0, 101.0, 106.0), // 2 held but meets the entry rule: counts restart
            (106.0, 106.0, 104.0, 105.0),
            (105.0, 105.5, 103.0, 104.0), // 2 held: sell
            (104.0, 107.5, 103.0, 107.0), // buy again
            (107.0, 108.0, 106.0, 107.2),
        ]);
        let kv = [("st_on", 0.0), ("n_candles", 2.0)];
        assert_eq!(go(V1, &r, &kv), vec![(1, 5, false), (6, 7, true)]);
        assert_eq!(go(V1_ATR, &r, &kv), vec![(1, 5, false), (6, 7, true)]);
        // buffer 2.5%: month 1 (1.98%) is no entry
        assert_eq!(go(V1, &r, &[("st_on", 0.0), ("buffer_pct", 2.5)]), vec![]);
    }

    #[test]
    fn exit_first_rebuys_on_the_same_month() {
        let r = months(&[
            (100.0, 101.0, 99.0, 100.0),
            (101.0, 103.5, 100.0, 103.0), // buy 103, PB 2% = 105.06
            (103.0, 106.5, 102.0, 106.0), // PB hit and the entry rule holds
            (106.0, 107.0, 105.0, 106.5),
        ]);
        let kv = [("st_on", 0.0), ("pb_pct", 2.0)];
        assert_eq!(go(V2, &r, &kv), vec![(1, 2, false), (2, 3, true)]);
        assert_eq!(go(V2_BEAR, &r, &kv), vec![(1, 2, false)]);
    }

    #[test]
    fn v3_sells_below_previous_high() {
        let r = months(&[
            (100.0, 101.0, 99.0, 100.0),
            (101.0, 103.5, 100.0, 103.0), // buy
            (103.0, 106.5, 102.0, 104.0), // above 103.5: hold
            (104.0, 105.0, 101.0, 102.0), // below 106.5: sell
            (102.0, 106.0, 101.0, 105.6), // 0.57% above 105: buy
            (105.6, 106.5, 104.0, 106.2),
        ]);
        assert_eq!(go(V3, &r, &[("st_on", 0.0)]), vec![(1, 3, false), (4, 5, true)]);
        assert_eq!(go(V3_BEAR, &r, &[("st_on", 0.0), ("exit_below", 0.0)]), vec![(1, 5, true)]);
    }

    #[test]
    fn monthly_bars_fill_on_the_last_day() {
        // two daily bars a month; the month's close is its last day's close
        let r = vec![
            row("2020-01-02", 100.0, 101.0, 99.0, 100.0),
            row("2020-01-31", 100.0, 100.5, 99.0, 100.0),
            row("2020-02-03", 100.0, 102.0, 99.0, 101.0),
            row("2020-02-28", 101.0, 103.5, 100.0, 103.0), // Feb close 103 > Jan high 101 x 1.005
            row("2020-03-02", 103.0, 103.5, 101.0, 102.0),
            row("2020-03-31", 102.0, 103.0, 99.0, 100.0), // Mar close 100 < Feb high 103.5
            row("2020-04-01", 100.0, 101.0, 99.0, 100.5),
        ];
        assert_eq!(go(V3, &r, &[("st_on", 0.0)]), vec![(3, 5, false)]);
    }

    fn daily() -> Vec<Row> {
        vec![
            row("2024-01-02", 99.0, 100.0, 98.0, 99.0),
            row("2024-01-03", 99.0, 100.0, 98.0, 99.5),
            row("2024-01-04", 99.0, 100.0, 98.0, 99.5), // Jan high 100
            row("2024-02-01", 99.0, 99.5, 98.0, 99.0),
            row("2024-02-02", 99.0, 101.5, 98.0, 101.0), // 1% above Jan high: buy (4)
            row("2024-02-05", 101.0, 102.0, 97.0, 98.0), // Feb high 102, Feb close 98 < 100
            row("2024-03-01", 98.0, 103.5, 98.0, 103.0), // 0.98% above Feb high
            row("2024-03-04", 103.0, 103.5, 100.0, 101.0), // Mar close 101 < Feb high 102
            row("2024-04-01", 101.0, 104.5, 101.0, 104.0), // 0.48% above Mar high: no
            row("2024-04-02", 104.0, 105.5, 103.0, 105.0), // buy
            row("2024-04-03", 105.0, 105.5, 104.0, 105.2),
        ]
    }

    #[test]
    fn v6_month_close_exit_skips_the_entry_month() {
        assert_eq!(go(V6, &daily(), &[("st_on", 0.0)]), vec![(4, 7, false), (9, 10, true)]);
        // exit_if also in the entry month: Feb's close sells at Feb's last day, Mar's first day buys, Mar's end sells
        assert_eq!(go(V6, &daily(), &[("st_on", 0.0), ("exit_if", 1.0)]), vec![(4, 5, false), (6, 7, false), (9, 10, true)]);
    }

    #[test]
    fn v4_defaults_hold_and_entry_month_exit() {
        assert_eq!(go(V4, &daily(), &[]), vec![(4, 10, true)]);
        assert_eq!(go(V4, &daily(), &[("exit_if", 1.0)]), vec![(4, 5, false), (6, 7, false), (9, 10, true)]);
    }

    #[test]
    fn v5_counts_month_ends_after_the_entry_month() {
        // 1 month after the entry month: Mar's last day sells (Mar does not meet the entry rule)
        assert_eq!(go(V5, &daily(), &[("st_on", 0.0), ("n_candles", 1.0)]), vec![(4, 7, false), (9, 10, true)]);
    }

    #[test]
    fn no_entry_on_a_months_last_day_or_close_at_high() {
        let mut r = daily();
        r[4].high = r[4].close; // high = close on Feb 2: no buy; Feb 5 is month end; Mar 1 buys
        assert_eq!(go(V4, &r, &[]), vec![(6, 10, true)]);
    }

    #[test]
    fn st_line_follows_the_afl_loop() {
        let b = Ohlc { o: vec![10.0; 6], h: vec![11.0; 6], l: vec![9.0; 6], c: vec![10.0, 10.0, 10.0, 10.0, 20.0, 5.0] };
        let line = st_line(&b, 2, 1.0, 4);
        // bars 0-1 Null (ATR(1) Null on bar 0); TR 2 on bars 1-4, 11 on bar 5
        assert_eq!(&line[..4], &[None, None, Some(8.0), Some(8.0)]);
        // bar 4 closes above the upper band 12: lower band 20 - 2 = 18; bar 5 closes below 18: down, upper band
        // 5 + 6.5 = 11.5 (below the previous 22)
        assert_eq!(&line[4..], &[Some(18.0), Some(11.5)]);
    }

    #[test]
    fn every_strategy_runs() {
        let r: Vec<Row> = (0..700)
            .map(|k| {
                let c = 100.0 + 15.0 * ((k as f64) / 23.0).sin() + k as f64 * 0.08;
                row(&format!("{}-{:02}-{:02}", 2000 + k / 252, 1 + (k % 252) / 21, 1 + k % 21), c - 0.5, c + 1.0, c - 1.0, c)
            })
            .collect();
        let x = Ctx::new(&r, &r, &[]);
        let all: Params = [
            ("hsl_pct", 5.0), ("pb_pct", 8.0), ("tsl_pct", 4.0), ("exit_at", 1.0), ("atr_on", 1.0), ("n_green", 2.0), ("n_up", 2.0),
            ("n_candles", 3.0), ("exit_if", 1.0), ("st_on", 1.0), ("st_entry", 0.0), ("st_avg", 6.0),
        ]
        .iter()
        .map(|(k, v)| (k.to_string(), *v))
        .collect();
        for id in IDS {
            assert!(catalogue().iter().any(|d| d.id == id) && family_of(id).is_some(), "{id}");
            for p in [Params::new(), all.clone()] {
                for (from, to) in [(0, r.len() - 1), (100, 500)] {
                    let mut p = p.clone();
                    for tf in [1.0, 2.0, 3.0] {
                        p.insert("st_tf".into(), tf);
                        p.insert("tf".into(), tf.min(2.0));
                        let t = run(id, &x, from, to, &p).unwrap();
                        assert!(t.iter().all(|t| t.exit_i >= t.legs[0].i && t.legs[0].i >= from && t.exit_i <= to), "{id}");
                        assert!(t.iter().filter(|t| t.open).count() <= 1, "{id}");
                    }
                }
            }
            assert!(!run(id, &x, 0, r.len() - 1, &Params::new()).unwrap().is_empty(), "{id}");
        }
    }
}
