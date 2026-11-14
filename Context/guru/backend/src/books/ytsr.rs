//! YT Strategies · Stochastic+RSI System: the team's AmiBroker AFLs in "Old Topics/Stochastic+RSI System" (a
//! Stochastic %K or IBS threshold cross together with an RSI threshold cross, long only). They are written on the
//! team's stock template (penny, turnover, circuit and dead-stock rules) and trade one symbol on its own bars, so
//! they run on an index or an individual stock's daily bars. Coded from the AFLs as written, with the AFLs' own
//! default settings; there is no book PDF.
//!
//! Coded:
//! - `ytsr_below_threshold`: "BreakDown System/RSI-Stochastic Below Thrshold (Any TimeFrame).afl": %K(30, 1)
//!   crosses below 90 and RSI(30) crosses below 60 on the same bar. "AFLs/RSI-Stochastic Below Thrshold.afl" is the
//!   same rules without the time-frame, RSI-drop, Supertrend and up-close options (all off by default), so it is
//!   this id with the defaults;
//! - `ytsr_stoch_ibs_rsi`: "Stochastic or IBS + RSI/Stochastic or IBS + RSI.afl": %K(30, 3) crosses above 90 or
//!   IBS > 0.9, and RSI(30) crosses above 60.
//! With their defaults neither AFL has an exit that fires before the last bar of the data (HSL / PB 0, exit candle
//! count 0, RSI-drop / Supertrend / ATR exits off), so a run holds the first position to the end: coded as written.
//!
//! AmiBroker behaviour followed:
//! - `SetBarsRequired(100000, 0)`: the loops run over the whole history; positions opened inside the test range are
//!   reported. Dead stock: no entry on the last bar, the last bar of the data sells;
//! - `StochK(p, s)` = 100 x Sum(C - LLV(L, p), s) / Sum(HHV(H, p) - LLV(L, p), s), with HHV / LLV / Sum over the
//!   bars available on the first bars; RSI is Wilder's (Null for the first `p` bars); `Cross` reading a Null value
//!   is false;
//! - a time-frame selection other than Default (the chart, daily here) is computed on the compressed weekly
//!   (Monday-Sunday) or monthly bars and brought back as the AFLs do it: `Ref(x, -1)`, `TimeFrameExpand(..,
//!   expandFirst)`, `Ref(x, 1)`: a day sees the previous period's value, the period's last day its own value, the
//!   last bar of the data Null. The AFLs' intraday choices (5min .. 4Hourly) are not offered (daily data only);
//! - the Supertrend / ATR exit lines: mean of the last `period` ATR(1) values (ATR(1) Null on the first bar) around
//!   the chosen price, flipping on a Close beyond the previous bar's line; the lower line only rises in an up trend
//!   (the Supertrend upper line only falls in a down trend; the ATR exit has no upper ratchet); Null reads -1e10 in
//!   the loops;
//! - fills at the close; with "Exit using HSL/PB" = Live, a hit at the level or the Open (less the buffer, not below
//!   the Low) when the Open is beyond it; prices kept inside the bar's range; an exit on a bar with Open = Low =
//!   Close (OLC, lower circuit) waits for the next non-OLC bar; no entry on a bar whose High = Close (UC); penny 0
//!   and turnover off.
//!
//! Not coded: "AFLs/RSI-Stochastic Cross Thrshold (Combined Version).afl": its entry reads `Stochastic_Threshold`
//! and `RSI_Threshold`, which this version no longer defines (it declares upper / lower thresholds instead), so
//! AmiBroker stops with an uninitialised-variable error. The description's "Ranking by RSI / %K" has no
//! PositionScore in the code.
#![allow(dead_code)]
use crate::books::ytmr::day_number;
use crate::books::ytsw::{compress, Bars};
use crate::books::ytwh::{bound, collect, hhv, llv, loop_val, Ev};
use crate::connors::{rsi, Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "yt_sr";
pub const BELOW: &str = "ytsr_below_threshold";
pub const IBS: &str = "ytsr_stoch_ibs_rsi";
pub const IDS: [&str; 2] = [BELOW, IBS];

const GROUP: &str = "YT Strategies · Stochastic+RSI System";

fn def(id: &'static str, name: &'static str, rules: &'static [&'static str], notes: &'static str) -> StrategyDef {
    StrategyDef { id, name, group: GROUP, session: 0, dir: "Long", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim: None }
}

pub fn catalogue() -> Vec<StrategyDef> {
    vec![
        def(BELOW, "RSI-Stochastic Below Threshold", &[
            "E|Stochastic %K(30, smoothing 1) crosses below 90 and RSI(30) crosses below 60 on the same bar: buy at the Close",
            "F|Indicator time frames: chart (daily) by default; weekly or monthly as the AFL brings them back",
            "X|With the defaults only the last bar of the data sells (the position is held)",
            "X|Options (off): HSL % / PB % (at the Close, or Live at the level / Open less the buffer), exit after N bars, exit after N up Closes",
            "X|RSI-drop option (off): once RSI(30) crosses above 60 after entry it tracks its peak; sell when RSI falls more than 5 below the peak",
            "X|Supertrend option (off, period 5, factor 2 on (H+L)/2): sell on a fresh Close below the line after a Close above it",
            "X|An exit on an Open = Low = Close bar waits for the next bar that is not",
            "F|No entry on a bar whose High = Close or on the last bar; exit then entry on one bar allowed",
        ], "RSI-Stochastic Below Thrshold (Any TimeFrame).afl (Stochastic+RSI System/BreakDown System), coded as the AFL is written; AFLs/RSI-Stochastic Below Thrshold.afl is the same rules without the time-frame, RSI-drop, Supertrend and up-close options. Defaults as the AFL: the %K threshold 90 lies outside its Param range 5-25, the RSI threshold 60 outside 20-50, both periods 30 outside 8-22, and the exit-candle count 0 below its minimum 1: all used as written. The AFL header names a weekly chart; here the chart is daily and the indicator time frames are the weekly option."),
        def(IBS, "Stochastic or IBS + RSI", &[
            "E|(Stochastic %K(30, smoothing 3) crosses above 90 or IBS = (C - L) / (H - L) > 0.9) and RSI(30) crosses above 60: buy at the Close",
            "F|Indicator time frames: chart (daily) by default; with a weekly (monthly) choice the entry is only on the week's (month's) last trading day",
            "X|With the defaults only the last bar of the data sells (the position is held)",
            "X|Options (off): HSL % / PB % (at the Close, or Live at the level / Open less the buffer)",
            "X|ATR exit option (off; period 5, multiplier 1 on the Open): sell when the Close falls below the previous bar's ATR trail line",
            "X|An exit on an Open = Low = Close bar waits for the next bar that is not",
            "F|No entry on a bar whose High = Close or on the last bar; no exit and re-entry on one bar",
        ], "Stochastic or IBS + RSI.afl (Stochastic+RSI System/Stochastic or IBS + RSI), coded as the AFL is written. Defaults as the AFL: %K period 30 (StochK's default smoothing 3), threshold 90, RSI period 30, threshold 60, IBS threshold 0.90; the exit buffer default 0 lies below its minimum 0.1 and is used as written."),
    ]
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    let exits = |v: &mut Vec<ParamDef>| {
        v.extend([
            p("exit_live", "Exit using HSL/PB (0 Close, 1 Live)", 0.0, 0.0, 1.0, 1.0),
            p("hsl_pct", "HSL % (0 off)", 0.0, 0.0, 100.0, 0.1),
            p("pb_pct", "PB % (0 off)", 0.0, 0.0, 100.0, 0.1),
            p("exit_buffer", "Exit buffer %", 0.0, 0.0, 100.0, 0.01),
        ])
    };
    Some(match id {
        BELOW => {
            let mut v = vec![
                p("stoch_prd", "Stochastic %K period", 30.0, 1.0, 100.0, 1.0),
                p("stoch_thr", "Stochastic %K threshold", 90.0, 1.0, 100.0, 1.0),
                p("stoch_avg", "Stochastic %K smoothing", 1.0, 1.0, 1000.0, 1.0),
                p("stoch_tf", "Stochastic time frame (0 chart/daily, 1 weekly, 2 monthly)", 0.0, 0.0, 2.0, 1.0),
                p("rsi_prd", "RSI period", 30.0, 1.0, 100.0, 1.0),
                p("rsi_thr", "RSI threshold", 60.0, 1.0, 100.0, 1.0),
                p("rsi_tf", "RSI time frame (0 chart/daily, 1 weekly, 2 monthly)", 0.0, 0.0, 2.0, 1.0),
                p("num_weeks", "Exit after N candles (0 off)", 0.0, 0.0, 150.0, 1.0),
                p("num_upclose", "Exit after N up Closes (0 off)", 0.0, 0.0, 100.0, 1.0),
            ];
            exits(&mut v);
            v.extend([
                p("rsi_exit", "RSI-drop exit (0 off, 1 on)", 0.0, 0.0, 1.0, 1.0),
                p("exit_rsi_tf", "Exit RSI time frame (0 chart/daily, 1 weekly, 2 monthly)", 0.0, 0.0, 2.0, 1.0),
                p("exit_rsi_thr", "Exit RSI threshold", 60.0, 20.0, 100.0, 5.0),
                p("rsi_drop", "RSI drop value", 5.0, 1.0, 100.0, 1.0),
                p("st", "Supertrend exit (0 off, 1 on)", 0.0, 0.0, 1.0, 1.0),
                p("st_input", "ST price (0 O, 1 H, 2 L, 3 C, 4 (O+C)/2, 5 (H+L)/2, 6 HLC/3, 7 OHLC/4)", 5.0, 0.0, 7.0, 1.0),
                p("st_tf", "ST time frame (0 chart/daily, 1 weekly, 2 monthly)", 0.0, 0.0, 2.0, 1.0),
                p("st_period", "ST period", 5.0, 1.0, 25.0, 1.0),
                p("st_mult", "ST multiplier", 2.0, 0.5, 5.0, 0.5),
            ]);
            v
        }
        IBS => {
            let mut v = vec![
                p("stoch_tf", "Stochastic time frame (0 chart/daily, 1 weekly, 2 monthly)", 0.0, 0.0, 2.0, 1.0),
                p("stoch_prd", "Stochastic %K period", 30.0, 1.0, 100.0, 1.0),
                p("stoch_thr", "Stochastic %K threshold", 90.0, 1.0, 100.0, 0.1),
                p("rsi_tf", "RSI time frame (0 chart/daily, 1 weekly, 2 monthly)", 0.0, 0.0, 2.0, 1.0),
                p("rsi_prd", "RSI period", 30.0, 1.0, 100.0, 1.0),
                p("rsi_thr", "RSI threshold", 60.0, 1.0, 100.0, 1.0),
                p("ibs_tf", "IBS time frame (0 chart/daily, 1 weekly, 2 monthly)", 0.0, 0.0, 2.0, 1.0),
                p("ibs_thr", "IBS threshold", 0.9, 0.01, 1.0, 0.01),
            ];
            exits(&mut v);
            v.extend([
                p("atr", "ATR exit (0 off, 1 on)", 0.0, 0.0, 1.0, 1.0),
                p("atr_tf", "ATR time frame (0 chart/daily, 1 weekly, 2 monthly)", 0.0, 0.0, 2.0, 1.0),
                p("atr_avg", "ATR price (0 O, 1 H, 2 L, 3 C, 4 (O+C)/2, 5 (H+L)/2, 6 HLC/3, 7 OHLC/4)", 0.0, 0.0, 7.0, 1.0),
                p("atr_prd", "ATR period", 5.0, 1.0, 100.0, 1.0),
                p("atr_mult", "ATR multiplier", 1.0, 0.1, 100.0, 0.1),
            ]);
            v
        }
        _ => return None,
    })
}

// ---------------------------------------------------------------- time frames

/// Bars of the chosen time frame: the daily bars as they are (code 0) or compressed weeks (1) / months (2), with opens.
pub(crate) struct Tf {
    pub daily: bool,
    pub o: Vec<f64>,
    pub b: Bars,
}

pub(crate) fn tf_bars(x: &Ctx, code: i64) -> Tf {
    let ch = match code {
        1 => 'W',
        2 => 'M',
        _ => 'D',
    };
    let b = compress(x, ch);
    let mut o = vec![0.0; b.c.len()];
    for i in (0..x.c.len()).rev() {
        o[b.group[i]] = x.o[i];
    }
    Tf { daily: ch == 'D', o, b }
}

/// A value computed on the time frame's bars, back on the daily bars: as it is for the chart time frame; otherwise
/// `Ref(v, -1)`, `TimeFrameExpand(.., expandFirst)`, `Ref(.., 1)`.
pub(crate) fn back(v: &[Option<f64>], t: &Tf) -> Vec<Option<f64>> {
    if t.daily {
        return v.to_vec();
    }
    let n = t.b.group.len();
    (0..n)
        .map(|i| {
            if i + 1 >= n {
                return None;
            }
            let g = t.b.group[i + 1];
            if g == 0 {
                None
            } else {
                v[g - 1]
            }
        })
        .collect()
}

fn opt(v: f64) -> Option<f64> {
    if v.is_finite() {
        Some(v)
    } else {
        None
    }
}

/// AFL StochK(p, s) on the given bars.
pub(crate) fn stochk(h: &[f64], l: &[f64], c: &[f64], p: usize, s: usize) -> Vec<Option<f64>> {
    let n = c.len();
    let (hh, ll) = (hhv(h, p), llv(l, p));
    let s = s.max(1);
    (0..n)
        .map(|i| {
            let k = (i + 1).saturating_sub(s);
            let num: f64 = (k..=i).map(|j| c[j] - ll[j]).sum();
            let den: f64 = (k..=i).map(|j| hh[j] - ll[j]).sum();
            if den != 0.0 {
                opt(100.0 * num / den)
            } else {
                None
            }
        })
        .collect()
}

fn rsi_of(c: &[f64], p: usize) -> Vec<Option<f64>> {
    rsi(c, p.max(1)).into_iter().map(opt).collect()
}

/// AFL CalculateAvg(input) on the time frame's bars.
fn price(t: &Tf, kind: i64, i: usize) -> f64 {
    let (o, h, l, c) = (t.o[i], t.b.h[i], t.b.l[i], t.b.c[i]);
    match kind {
        0 => o,
        1 => h,
        2 => l,
        4 => (o + c) / 2.0,
        5 => (h + l) / 2.0,
        6 => (h + l + c) / 3.0,
        7 => (o + h + l + c) / 4.0,
        _ => c,
    }
}

/// The AFLs' Supertrend (`upper_ratchet`) / ATR-trail loop on the time frame's bars. Returns the Supertrend line
/// (lower line in an up trend, upper line in a down trend) or, for the ATR trail, the lower line.
pub(crate) fn trail_line(t: &Tf, kind: i64, period: usize, mult: f64, upper_ratchet: bool) -> Vec<Option<f64>> {
    let (h, l, c) = (&t.b.h, &t.b.l, &t.b.c);
    let n = c.len();
    let p = period.max(1);
    let tr: Vec<Option<f64>> = (0..n).map(|i| if i == 0 { None } else { Some(h[i].max(c[i - 1]) - l[i].min(c[i - 1])) }).collect();
    let satr: Vec<Option<f64>> = (0..n).map(|i| if i + 1 < p { None } else { tr[i + 1 - p..=i].iter().try_fold(0.0, |s, v| v.map(|v| s + v)) }).collect();
    let mut up: Vec<Option<f64>> = (0..n).map(|i| satr[i].map(|s| price(t, kind, i) + mult * s / p as f64)).collect();
    let mut dn: Vec<Option<f64>> = (0..n).map(|i| satr[i].map(|s| price(t, kind, i) - mult * s / p as f64)).collect();
    let mut is_up = vec![true; n];
    let mx = |a: Option<f64>, b: Option<f64>| match (a, b) {
        (None, None) => None,
        _ => Some(loop_val(a).max(loop_val(b))),
    };
    let mn = |a: Option<f64>, b: Option<f64>| match (a, b) {
        (Some(a), Some(b)) => Some(a.min(b)),
        _ => None, // Min with the -1e10 of a Null is Null
    };
    for i in p.max(1)..n {
        is_up[i] = if c[i] > loop_val(up[i - 1]) { true } else if c[i] < loop_val(dn[i - 1]) { false } else { is_up[i - 1] };
        if is_up[i] {
            dn[i] = mx(dn[i], dn[i - 1]);
        } else if upper_ratchet {
            up[i] = mn(up[i], up[i - 1]);
        }
    }
    if upper_ratchet {
        (0..n).map(|i| if is_up[i] { dn[i] } else { up[i] }).collect()
    } else {
        dn
    }
}

/// AFL Cross(a, b) on bar i (false when a value is Null).
fn cross(a: &[Option<f64>], b: &[Option<f64>], i: usize) -> bool {
    match (a[i], b[i], a[i - 1], b[i - 1]) {
        (Some(a0), Some(b0), Some(a1), Some(b1)) => a0 > b0 && a1 <= b1,
        _ => false,
    }
}

fn konst(n: usize, v: f64) -> Vec<Option<f64>> {
    vec![Some(v); n]
}

// ---------------------------------------------------------------- the exits shared by both AFLs

#[derive(Clone, Copy)]
pub(crate) struct Stops {
    pub live: bool,
    pub hsl_pct: f64,
    pub pb_pct: f64,
    pub buf: f64,
}

fn olc(x: &Ctx, i: usize) -> bool {
    x.l[i] == x.c[i] && x.l[i] == x.o[i]
}

/// HSL / PB hits on bar i and the AFL's SellPrice for them (SL overriding PB).
fn stop_hits(x: &Ctx, i: usize, k: &Stops, hsl: Option<f64>, pb: Option<f64>) -> (bool, bool, f64) {
    let (o, h, l, c) = (x.o[i], x.h[i], x.l[i], x.c[i]);
    let sl_hit = k.hsl_pct > 0.0 && (if k.live { l } else { c }) < loop_val(hsl);
    let pb_hit = k.pb_pct > 0.0 && (if k.live { h } else { c }) > loop_val(pb);
    let mut px = c;
    if pb_hit && k.live {
        let pb = loop_val(pb);
        px = if o > pb { l.max(o * (1.0 - k.buf / 100.0)) } else { pb };
    }
    if sl_hit && k.live {
        let hsl = loop_val(hsl);
        px = if o < hsl { l.max(o * (1.0 - k.buf / 100.0)) } else { hsl };
    }
    (sl_hit, pb_hit, px)
}

fn levels(c: f64, k: &Stops) -> (Option<f64>, Option<f64>) {
    (if k.hsl_pct > 0.0 { Some(c * (1.0 - k.hsl_pct / 100.0)) } else { None }, if k.pb_pct > 0.0 { Some(c * (1.0 + k.pb_pct / 100.0)) } else { None })
}

fn set_exit(x: &Ctx, evs: &mut [Ev], i: usize, px: f64, r: &'static str) {
    if let Some(v) = evs.last_mut() {
        v.exit = Some((i, bound(x, i, px), i, r));
    }
}

// ---------------------------------------------------------------- Below Threshold (Any TimeFrame)

#[derive(Clone, Copy)]
pub(crate) struct BelowCfg {
    pub num_weeks: i64,
    pub num_upclose: i64,
    pub stops: Stops,
    pub rsi_exit: bool,
    pub exit_rsi_thr: f64,
    pub rsi_drop: f64,
    pub st: bool,
}

/// The AFL's bar loop: exits first, then the entry on the same bar.
pub(crate) fn below_loop(x: &Ctx, entry: &[bool], exit_rsi: &[Option<f64>], st: &[Option<f64>], k: &BelowCfg) -> Vec<Ev> {
    let c = &x.c;
    let n = c.len();
    let last = n.saturating_sub(1);
    let mut evs: Vec<Ev> = Vec::new();
    let (mut pos, mut skip) = (false, false);
    let (mut hsl, mut pb) = (None, None);
    let (mut weeks, mut ups, mut flag, mut st_buy, mut st_sell) = (0i64, 0i64, 0.0f64, false, false);
    for i in 1..n {
        let dead = i == last;
        if pos {
            let (sl_hit, pb_hit, px) = stop_hits(x, i, &k.stops, hsl, pb);
            weeks += 1;
            let (rv, rvp) = (loop_val(exit_rsi[i]), loop_val(exit_rsi[i - 1]));
            if (flag == 0.0 && rvp < k.exit_rsi_thr && rv > k.exit_rsi_thr) || (flag != 0.0 && rv > flag) {
                flag = rv;
            }
            if k.st {
                if c[i] > loop_val(st[i]) && !st_buy {
                    st_buy = true;
                }
                if c[i - 1] > loop_val(st[i - 1]) && c[i] < loop_val(st[i]) && st_buy {
                    st_sell = true;
                }
            }
            if c[i] > c[i - 1] {
                ups += 1;
            }
            let rsi_hit = k.rsi_exit && rv < flag - k.rsi_drop;
            let cond = pb_hit || sl_hit || dead || weeks == k.num_weeks || rsi_hit || (k.st && st_sell) || (k.num_upclose > 0 && ups == k.num_upclose);
            if cond {
                if !olc(x, i) || dead {
                    let r = if sl_hit {
                        "HSL"
                    } else if pb_hit {
                        "PB"
                    } else if weeks == k.num_weeks {
                        "Candle count"
                    } else if rsi_hit {
                        "RSI drop"
                    } else if k.st && st_sell {
                        "Supertrend"
                    } else if k.num_upclose > 0 && ups == k.num_upclose {
                        "Up closes"
                    } else {
                        "Last bar"
                    };
                    set_exit(x, &mut evs, i, px, r);
                    pos = false;
                    hsl = None;
                    pb = None;
                    skip = false;
                } else {
                    skip = true;
                }
            }
            if skip && (!olc(x, i) || dead) {
                set_exit(x, &mut evs, i, c[i], "Blocked exit, next bar");
                pos = false;
                skip = false;
                hsl = None;
                pb = None;
            }
        }
        if !pos && entry[i] {
            (hsl, pb) = levels(c[i], &k.stops);
            pos = true;
            weeks = 0;
            ups = 0;
            flag = 0.0;
            st_buy = false;
            st_sell = false;
            evs.push(Ev { dir: 1.0, e: i, epx: c[i], exit: None });
        }
    }
    evs
}

// ---------------------------------------------------------------- Stochastic or IBS + RSI

/// The AFL's bar loop: the entry is tested first, the exits in the `else` branch.
pub(crate) fn ibs_loop(x: &Ctx, entry: &[bool], atr_line: Option<&[Option<f64>]>, k: &Stops) -> Vec<Ev> {
    let c = &x.c;
    let n = c.len();
    let last = n.saturating_sub(1);
    let mut evs: Vec<Ev> = Vec::new();
    let (mut pos, mut skip) = (false, false);
    let (mut hsl, mut pb) = (None, None);
    for i in 1..n {
        let dead = i == last;
        if !pos && entry[i] {
            (hsl, pb) = levels(c[i], k);
            pos = true;
            evs.push(Ev { dir: 1.0, e: i, epx: c[i], exit: None });
        } else if pos && !skip {
            let (sl_hit, pb_hit, px) = stop_hits(x, i, k, hsl, pb);
            let atr_hit = atr_line.map_or(false, |a| c[i] < loop_val(a[i - 1]));
            if pb_hit || sl_hit || atr_hit || dead {
                if !olc(x, i) || dead {
                    let r = if sl_hit { "HSL" } else if pb_hit { "PB" } else if atr_hit { "ATR trail" } else { "Last bar" };
                    set_exit(x, &mut evs, i, px, r);
                    pos = false;
                    hsl = None;
                    pb = None;
                    skip = false;
                } else {
                    skip = true;
                }
            }
        }
        if skip && pos && (!olc(x, i) || dead) {
            set_exit(x, &mut evs, i, c[i], "Blocked exit, next bar");
            pos = false;
            skip = false;
            hsl = None;
            pb = None;
        }
    }
    evs
}

/// AFL EntryDay of the IBS AFL: the month's (week's) last trading day when any indicator is monthly (weekly).
fn entry_day(x: &Ctx, i: usize, monthly: bool, weekly: bool) -> bool {
    let n = x.c.len();
    if !monthly && !weekly {
        return true;
    }
    if i + 1 >= n {
        return false;
    }
    let (a, b) = (&x.r[i].date, &x.r[i + 1].date);
    if monthly {
        return a.get(5..7) != b.get(5..7);
    }
    let dow = |d: &str| day_number(d).map(|z| (z + 4).rem_euclid(7));
    match (dow(a), dow(b)) {
        (Some(p), Some(q)) => p > q,
        _ => false,
    }
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(0.0, |d| d.def));
    let gi = |k: &str| g(k).round() as i64;
    let gu = |k: &str| g(k).round().max(1.0) as usize;
    let n = x.c.len();
    if n == 0 || from > to || to >= n {
        return Some(Vec::new());
    }
    let last = n - 1;
    let buy_ok = |i: usize| i != last && x.h[i] != x.c[i] && x.c[i] > 0.0;
    let stops = Stops { live: gi("exit_live") == 1, hsl_pct: g("hsl_pct"), pb_pct: g("pb_pct"), buf: g("exit_buffer") };
    let stoch = |code: i64, s: usize| {
        let t = tf_bars(x, code);
        back(&stochk(&t.b.h, &t.b.l, &t.b.c, gu("stoch_prd"), s), &t)
    };
    let rsi_on = |code: i64| {
        let t = tf_bars(x, code);
        back(&rsi_of(&t.b.c, gu("rsi_prd")), &t)
    };
    let evs = match id {
        BELOW => {
            let k = stoch(gi("stoch_tf"), gu("stoch_avg"));
            let r = rsi_on(gi("rsi_tf"));
            let (kt, rt) = (konst(n, g("stoch_thr")), konst(n, g("rsi_thr")));
            let entry: Vec<bool> = (0..n).map(|i| i > 0 && cross(&kt, &k, i) && cross(&rt, &r, i) && buy_ok(i)).collect();
            let exit_rsi = rsi_on(gi("exit_rsi_tf"));
            let st_on = gi("st") == 1;
            let st = if st_on {
                let t = tf_bars(x, gi("st_tf"));
                back(&trail_line(&t, gi("st_input"), gu("st_period"), g("st_mult"), true), &t)
            } else {
                vec![None; n]
            };
            let cfg = BelowCfg { num_weeks: gi("num_weeks"), num_upclose: gi("num_upclose"), stops, rsi_exit: gi("rsi_exit") == 1, exit_rsi_thr: g("exit_rsi_thr"), rsi_drop: g("rsi_drop"), st: st_on };
            below_loop(x, &entry, &exit_rsi, &st, &cfg)
        }
        IBS => {
            let (st, rt, it) = (gi("stoch_tf"), gi("rsi_tf"), gi("ibs_tf"));
            let k = stoch(st, 3);
            let r = rsi_on(rt);
            let ibs = {
                let t = tf_bars(x, it);
                let v: Vec<Option<f64>> = (0..t.b.c.len()).map(|j| if t.b.h[j] != t.b.l[j] { Some((t.b.c[j] - t.b.l[j]) / (t.b.h[j] - t.b.l[j])) } else { None }).collect();
                back(&v, &t)
            };
            let monthly = st == 2 || rt == 2 || it == 2;
            let weekly = st == 1 || rt == 1 || it == 1;
            let (kt, rt_) = (konst(n, g("stoch_thr")), konst(n, g("rsi_thr")));
            let ibs_thr = g("ibs_thr");
            let entry: Vec<bool> = (0..n)
                .map(|i| i > 0 && entry_day(x, i, monthly, weekly) && (cross(&k, &kt, i) || ibs[i].map_or(false, |v| v > ibs_thr)) && cross(&r, &rt_, i) && buy_ok(i))
                .collect();
            let atr = if gi("atr") == 1 {
                let t = tf_bars(x, gi("atr_tf"));
                Some(back(&trail_line(&t, gi("atr_avg"), gu("atr_prd"), g("atr_mult"), false), &t))
            } else {
                None
            };
            ibs_loop(x, &entry, atr.as_deref(), &stops)
        }
        _ => return None,
    };
    Some(collect(x, evs, from, to))
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let d = catalogue().into_iter().find(|d| d.id == id)?;
    Some((d.id, d.name, "Stochastic+RSI System"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::books::ytwh::testbars::*;

    fn quiet(n: usize) -> Vec<(f64, f64, f64, f64)> {
        (0..n).map(|k| { let c = 100.0 + k as f64 * 0.1; (c, c + 1.0, c - 1.0, c) }).collect()
    }
    fn stops() -> Stops {
        Stops { live: false, hsl_pct: 0.0, pb_pct: 0.0, buf: 0.0 }
    }
    fn cfg() -> BelowCfg {
        BelowCfg { num_weeks: 0, num_upclose: 0, stops: stops(), rsi_exit: false, exit_rsi_thr: 60.0, rsi_drop: 5.0, st: false }
    }
    fn ev(e: &[Ev]) -> Vec<(usize, Option<usize>, &'static str)> {
        e.iter().map(|v| (v.e, v.exit.map(|t| t.0), v.exit.map_or("", |t| t.3))).collect()
    }
    fn at(n: usize, on: &[usize]) -> Vec<bool> {
        (0..n).map(|i| on.contains(&i)).collect()
    }

    #[test]
    fn stochk_and_cross() {
        // bar 2: close at the top of the 3-bar range = 100, then at the bottom = 0
        let (h, l, c) = ([10.0, 11.0, 12.0, 12.0], [9.0, 9.5, 10.0, 9.0], [9.5, 10.0, 12.0, 9.0]);
        let k = stochk(&h, &l, &c, 3, 1);
        assert_eq!(k[2], Some(100.0));
        assert_eq!(k[3], Some(0.0));
        let thr = konst(4, 90.0);
        assert!(cross(&thr, &k, 3) && !cross(&thr, &k, 2));
        // smoothing 3: sums of the numerators over the denominators
        let k3 = stochk(&h, &l, &c, 3, 3);
        let num = (9.5 - 9.0) + (10.0 - 9.0) + (12.0 - 9.0);
        let den = (10.0 - 9.0) + (11.0 - 9.0) + (12.0 - 9.0);
        assert!((k3[2].unwrap() - 100.0 * num / den).abs() < 1e-9);
    }

    #[test]
    fn higher_time_frame_values_come_back_as_the_afl_does() {
        let r = bars(&quiet(15));
        let x = Ctx::new(&r, &r, &[]);
        let t = tf_bars(&x, 1);
        let v: Vec<Option<f64>> = (0..t.b.c.len()).map(|g| Some(g as f64)).collect();
        let b = back(&v, &t);
        // week 0 days see Null, its Friday its own value 0; week 1 days see 0, its Friday 1; the last bar is Null
        assert_eq!(&b[..6], &[None, None, None, None, Some(0.0), Some(0.0)]);
        assert_eq!(b[9], Some(1.0));
        assert_eq!(b[14], None);
        assert_eq!(t.o[1], x.o[5]);
    }

    #[test]
    fn below_holds_to_the_last_bar_and_reenters_on_an_exit_bar() {
        let r = bars(&quiet(8));
        let x = Ctx::new(&r, &r, &[]);
        let none = vec![None; 8];
        assert_eq!(ev(&below_loop(&x, &at(8, &[2]), &none, &none, &cfg())), vec![(2, Some(7), "Last bar")]);
        // candle count 2: sold on bar 4, and a signal on that bar buys again
        let e = below_loop(&x, &at(8, &[2, 4]), &none, &none, &BelowCfg { num_weeks: 2, ..cfg() });
        assert_eq!(ev(&e), vec![(2, Some(4), "Candle count"), (4, Some(6), "Candle count")]);
    }

    #[test]
    fn below_rsi_drop_and_live_pb() {
        let mut v = quiet(8);
        let r0 = bars(&v);
        let x0 = Ctx::new(&r0, &r0, &[]);
        let rs: Vec<Option<f64>> = [50.0, 50.0, 50.0, 62.0, 70.0, 66.0, 64.0, 64.0].iter().map(|&a| Some(a)).collect();
        let none = vec![None; 8];
        let e = below_loop(&x0, &at(8, &[2]), &rs, &none, &BelowCfg { rsi_exit: true, ..cfg() });
        assert_eq!(ev(&e), vec![(2, Some(6), "RSI drop")]);
        // PB 5% live: the open gaps above the target -> sold at the open (less a 0 buffer)
        v[4] = (110.0, 111.0, 109.0, 110.5);
        let r = bars(&v);
        let x = Ctx::new(&r, &r, &[]);
        let k = BelowCfg { stops: Stops { live: true, pb_pct: 5.0, ..stops() }, ..cfg() };
        let e = below_loop(&x, &at(8, &[2]), &none, &none, &k);
        assert_eq!(ev(&e), vec![(2, Some(4), "PB")]);
        assert!((e[0].exit.unwrap().1 - 110.0).abs() < 1e-9);
    }

    #[test]
    fn ibs_loop_olc_defers_and_no_same_bar_reentry() {
        let mut v = quiet(8);
        v[4] = (95.0, 96.0, 95.0, 95.0); // Open = Low = Close: the HSL exit waits
        let r = bars(&v);
        let x = Ctx::new(&r, &r, &[]);
        let k = Stops { hsl_pct: 2.0, ..stops() };
        let e = ibs_loop(&x, &at(8, &[2, 5]), None, &k);
        // the signal on the exit bar 5 is not taken (the entry is tested before the exit)
        assert_eq!(ev(&e), vec![(2, Some(5), "Blocked exit, next bar")]);
    }

    #[test]
    fn ibs_entry_day_and_atr_line() {
        let r = bars(&quiet(12));
        let x = Ctx::new(&r, &r, &[]);
        // weekly: Friday (bar 4) is the week's last day, Thursday is not
        assert!(entry_day(&x, 4, false, true) && !entry_day(&x, 3, false, true));
        let t = tf_bars(&x, 0);
        let line = trail_line(&t, 3, 2, 1.0, false);
        assert!(line[0].is_none() && line[1].is_none() && line[2].is_some());
        // ATR line under a steadily rising close never falls
        assert!(line[2..].windows(2).all(|w| w[1].unwrap() >= w[0].unwrap()));
    }

    #[test]
    fn every_strategy_runs() {
        smoke(run, &IDS);
        let r = wave(400);
        let x = Ctx::new(&r, &r, &[]);
        let on: Params = [("st", 1.0), ("st_tf", 1.0), ("rsi_exit", 1.0), ("stoch_tf", 1.0), ("rsi_tf", 2.0), ("atr", 1.0), ("atr_tf", 2.0), ("hsl_pct", 3.0), ("pb_pct", 5.0), ("exit_live", 1.0), ("num_upclose", 3.0)]
            .iter()
            .map(|(k, v)| (k.to_string(), *v))
            .collect();
        for id in IDS {
            let t = run(id, &x, 0, r.len() - 1, &on).unwrap();
            assert!(t.iter().all(|t| t.exit_i >= t.legs[0].i), "{id}");
            assert!(params(id).unwrap().iter().all(|d| d.def >= d.min && d.def <= d.max), "{id}");
        }
    }
}
