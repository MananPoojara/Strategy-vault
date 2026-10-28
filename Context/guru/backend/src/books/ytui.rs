//! YT Strategies · Ulcer Performance Index: the team's AmiBroker AFLs in "Old Topics/Ulcer Performance Index" that
//! trade one index on daily bars (spot Nifty / Bank Nifty). Coded from the AFLs as written, with the AFLs' own
//! default settings; there is no book PDF.
//!
//! Ulcer line: src = close (or open / high / low), HH = HHV(src, period), DD = 100 x (src - HH) / HH,
//! Ulcer = sqrt(MA(DD^2, smoothing)), Signal = MA (or EMA) of the Ulcer line. Fresh mode: the long cross is
//! Cross(Signal, Ulcer) (the Ulcer line falls through its signal) or Ulcer <= Signal on the signal's first bar;
//! the short cross is Cross(Ulcer, Signal) or Ulcer > Signal on that first bar. Running mode: the state itself.
//!
//! As in the ROC module (whose AmiBroker helpers this file shares): the AFL loop runs on the whole history and
//! writes Buy / Sell / Short / Cover with its own exit prices; the AFLs' `dateCheck` (the bar inside the Analysis
//! range) is the test window; the backtester trades the arrays long and short (an opposite entry reverses at the
//! close), exits first and then enters on the same bar, ignores exits while flat; the forced exit on the data's
//! last bar counts as still open. The team Supertrend and the weekly / monthly Ref(-1) / expandFirst / Ref(+1)
//! pattern are those of the ROC module. Null values before an array's first value make a condition false.
#![allow(dead_code)]
use crate::books::ytroc::{avg_of, backtest, cross, gt, htf, lt, mean_tr, open_first, rsi_opt, st_daily, team_st, tf_bars, Px, LAST_BAR};
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "yt_ui";
pub const FINAL: &str = "ytui_ulcer_index_final";
pub const EXIT_ULCER: &str = "ytui_st_or_ulcer_exit_only_ulcer";
pub const EXIT_OWN: &str = "ytui_st_or_ulcer_exit_own_or_fresh_opp";
pub const IDS: [&str; 3] = [FINAL, EXIT_ULCER, EXIT_OWN];

fn def(id: &'static str, name: &'static str, rules: &'static [&'static str], notes: &'static str) -> StrategyDef {
    StrategyDef { id, name, group: "YT Strategies · Ulcer Performance Index", session: 0, dir: "Long/Short", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim: None }
}

const EXITS: &str = "X|Also: hard SL %, profit booking %, trailing SL % (on the previous bar's level), the close crossing the trailing ATR line (longs only), the low / high beyond the lowest low / highest high of the Xth candles before entry; live mode fills HSL / PB at the level or at the open +/- the buffer % when the open is through it";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![
        def(FINAL, "Ulcer Index Crossover", &[
            "F|Ulcer Index of the close (HHV 14, smoothing 14) against its 52-bar SMA signal",
            "E|Long at the close on the Nth candle (default 0: the same bar) after the Ulcer line falls through its signal; short after it rises through it; trade type long (default), short or any",
            "X|Sell when the Ulcer line rises through its signal; cover when it falls through it",
            EXITS,
        ], "Futures_UlcerIndex_Final.afl (Ulcer Performance Index/AFLs), coded as the AFL is written, with its ExRem lines. ATR exit average default Open (the ParamList has no default, so its first item). CalculateAvg comes from an include that is not in the folder; it is taken to be the inline version of the sibling AFLs."),
        def(EXIT_ULCER, "ST or Ulcer Entry, Exit Only on Ulcer", &[
            "E|Long as in the Ulcer crossover, or (ST entry on) when the close crosses above the entry Supertrend; optional RSI level and bull / bear ST filters on the long entry",
            "E|Short, as the AFL's operator precedence reads it: (flat, Nth candle after the Ulcer rise through the signal) OR (ST condition AND trade type short / any AND not the last bar), so Ulcer shorts are taken even with trade type Long",
            "X|Sell when the Ulcer line rises through its signal; cover when it falls through it (the Supertrend is not an exit)",
            EXITS,
        ], "ST or Ulcer WEE_Exit only Ulcer_UlcerIndex_ST_Final.afl, coded as the AFL is written. Known quirk kept: the short-entry condition lacks brackets (AND binds before OR), so with the default trade type Long the AFL still writes Short signals on Ulcer rises and they trade (long and short backtest); with trade type Short / Any and the ST entry off, the second branch is true on every bar. Its ExRem lines are commented out. The bias ST filter and ATR average use GetTimeframe / CalculateAvg from a missing include, taken to match the inline versions."),
        def(EXIT_OWN, "ST or Ulcer Entry, Exit on Own or Fresh Opposite Signal", &[
            "E|Long on the Ulcer crossover (Nth candle or later) or on the close crossing above the entry Supertrend, whichever comes first; short the mirror; optional RSI and bull / bear ST filters",
            "X|A trade exits on the opposite signal of either kind; when the opposite signal of the other kind came first it waits for the next one, and an ST sell on a bar with a fresh Ulcer long cross prolongs the trade; shorts also cover on any Ulcer long cross; the bull / bear filter turning against the trade exits",
            EXITS,
        ], "ST or Ulcer WEE_Exit Own or Fresh of Opp_UlcerIndex_ST_Final.afl, coded as the AFL is written, including its second exit pass after the entries on every bar. The AFL default 'Period for Highest Src' is 0, below its Param minimum 6; it is kept, and HHV over 0 bars is taken as empty, so the Ulcer line and every Ulcer signal are Null and the defaults trade only through the ST entry (off by default): no trades at the defaults. The team's workbooks optimised it over 6-42. On a cover the AFL resets 'currentStatus' (a typo), so the short side's wait state carries into later shorts. No ExRem."),
    ]
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    let mut v = vec![p("src", "Highest source value (0 Close, 1 Open, 2 High, 3 Low)", 0.0, 0.0, 3.0, 1.0)];
    match id {
        FINAL => v.extend([
            p("hh_period", "Period for highest src", 14.0, 1.0, 100.0, 1.0),
            p("smooth", "Smoothing length", 14.0, 1.0, 100.0, 1.0),
            p("sig_type", "Signal smoothing type (0 SMA, 1 EMA)", 0.0, 0.0, 1.0, 1.0),
            p("sig_len", "Signal smoothing length", 52.0, 1.0, 100.0, 1.0),
        ]),
        EXIT_ULCER => v.extend([
            p("hh_period", "Period for highest src", 14.0, 6.0, 42.0, 4.0),
            p("smooth", "Smoothing length", 14.0, 6.0, 42.0, 4.0),
            p("sig_type", "Signal smoothing type (0 SMA, 1 EMA)", 0.0, 0.0, 1.0, 1.0),
            p("sig_len", "Signal smoothing length", 52.0, 1.0, 200.0, 1.0),
        ]),
        EXIT_OWN => v.extend([
            p("hh_period", "Period for highest src (AFL default 0, below its Param minimum 6: no Ulcer signals)", 0.0, 0.0, 42.0, 4.0),
            p("smooth", "Smoothing length", 14.0, 6.0, 42.0, 4.0),
            p("sig_type", "Signal smoothing type (0 SMA, 1 EMA)", 0.0, 0.0, 1.0, 1.0),
            p("sig_len", "Signal smoothing length", 52.0, 1.0, 500.0, 1.0),
        ]),
        _ => return None,
    }
    v.extend([
        p("cross_mode", "Cross over for entry (0 fresh, 1 running)", 0.0, 0.0, 1.0, 1.0),
        p("trade_type", "Trade type (0 long, 1 short, 2 any)", 0.0, 0.0, 2.0, 1.0),
        p("nth", "Nth candle for entry", 0.0, 0.0, 100.0, 1.0),
        p("xth", "Xth candle lowest low / highest high as SL (0 off)", 0.0, 0.0, 100.0, 1.0),
        p("exit_at", "Exit at (0 close, 1 live high / low)", 0.0, 0.0, 1.0, 1.0),
        p("hsl_pct", "HSL % (0 off)", 0.0, 0.0, 100.0, 0.1),
        p("pb_pct", "PB % (0 off)", 0.0, 0.0, 100.0, 0.1),
        p("tsl_pct", "TSL % (0 off)", 0.0, 0.0, 100.0, 0.1),
        p("buffer", "Exit buffer %", 0.25, 0.01, 1.0, 0.01),
        p("atr_on", "ATR exit (0 off, 1 on)", 0.0, 0.0, 1.0, 1.0),
        p("atr_period", "ATR period", 5.0, 1.0, 100.0, 1.0),
        p("atr_mult", "ATR multiplier", 2.0, 1.0, 100.0, 0.5),
        p("atr_avg", "ATR average (0 Open, 1 High, 2 Low, 3 Close, 4 (O+C)/2, 5 (H+L)/2, 6 (H+L+C)/3, 7 (O+H+L+C)/4)", 0.0, 0.0, 7.0, 1.0),
    ]);
    match id {
        EXIT_ULCER => v.extend([
            p("bias_on", "ST filter (bull / bear phase) (0 off, 1 on)", 0.0, 0.0, 1.0, 1.0),
            p("bias_tf", "ST filter timeframe (0 daily, 1 weekly, 2 monthly)", 0.0, 0.0, 2.0, 1.0),
            p("bias_period", "ST filter period", 5.0, 5.0, 20.0, 5.0),
            p("bias_factor", "ST filter factor", 1.0, 0.5, 3.0, 0.5),
            p("bias_avg", "ST filter average (0 Close, 1 Open, 2 High, 3 Low, 4 (O+C)/2, 5 (H+L)/2, 6 (H+L+C)/3, 7 (O+H+L+C)/4)", 5.0, 0.0, 7.0, 1.0),
            p("bias_side", "ST filter entry (0 bullish: close above the ST, 1 bearish)", 0.0, 0.0, 1.0, 1.0),
            p("rsi_on", "RSI filter (0 off, 1 on)", 0.0, 0.0, 1.0, 1.0),
            p("rsi_period", "RSI period", 14.0, 5.0, 30.0, 1.0),
            p("rsi_side", "RSI value (0 below the threshold, 1 above)", 0.0, 0.0, 1.0, 1.0),
            p("rsi_thr", "RSI threshold", 1.0, 1.0, 90.0, 1.0),
            p("ste_on", "Supertrend entry (0 off, 1 on)", 0.0, 0.0, 1.0, 1.0),
            p("ste_tf", "Supertrend entry timeframe (0 daily, 1 weekly, 2 monthly)", 0.0, 0.0, 2.0, 1.0),
            p("ste_period", "Supertrend entry period", 5.0, 5.0, 5.0, 1.0),
            p("ste_factor", "Supertrend entry factor", 1.0, 1.0, 4.0, 1.0),
            p("ste_avg", "Supertrend entry average (0 Close, 1 Open, 2 High, 3 Low, 4 (O+C)/2, 5 (H+L)/2, 6 (H+L+C)/3, 7 (O+H+L+C)/4)", 5.0, 0.0, 7.0, 1.0),
            p("ste_side", "Supertrend entry filter (0 bullish: long on the close crossing above, 1 bearish)", 0.0, 0.0, 1.0, 1.0),
        ]),
        EXIT_OWN => v.extend([
            p("ste_on", "ST entry (0 off, 1 on)", 0.0, 0.0, 1.0, 1.0),
            p("ste_tf", "ST entry timeframe (0 daily, 1 weekly, 2 monthly)", 0.0, 0.0, 2.0, 1.0),
            p("ste_period", "ST entry period", 5.0, 1.0, 5.0, 1.0),
            p("ste_factor", "ST entry factor", 1.0, 1.0, 4.0, 1.0),
            p("ste_avg", "ST entry average (0 Close, 1 Open, 2 High, 3 Low, 4 (O+C)/2, 5 (H+L)/2, 6 (H+L+C)/3, 7 (O+H+L+C)/4)", 5.0, 0.0, 7.0, 1.0),
            p("bias_on", "Bull-bear ST filter (0 off, 1 on)", 0.0, 0.0, 1.0, 1.0),
            p("bias_tf", "ST filter timeframe (0 daily, 1 weekly, 2 monthly)", 0.0, 0.0, 2.0, 1.0),
            p("bias_period", "ST filter period", 5.0, 5.0, 20.0, 5.0),
            p("bias_factor", "ST filter factor", 1.0, 0.5, 3.0, 0.5),
            p("bias_avg", "ST filter average (0 Close, 1 Open, 2 High, 3 Low, 4 (O+C)/2, 5 (H+L)/2, 6 (H+L+C)/3, 7 (O+H+L+C)/4)", 5.0, 0.0, 7.0, 1.0),
            p("bias_side", "ST entry type (0 bullish: longs above the ST, 1 bearish)", 0.0, 0.0, 1.0, 1.0),
            p("rsi_on", "RSI filter (0 off, 1 on)", 0.0, 0.0, 1.0, 1.0),
            p("rsi_period", "RSI period", 14.0, 2.0, 16.0, 1.0),
            p("rsi_side", "RSI value (0 below the threshold, 1 above)", 0.0, 0.0, 1.0, 1.0),
            p("rsi_thr", "RSI threshold", 1.0, 1.0, 90.0, 1.0),
        ]),
        _ => {}
    }
    Some(v)
}

// ---------------------------------------------------------------- Ulcer line

fn window_opt(v: &[Option<f64>], i: usize, n: usize) -> Option<Vec<f64>> {
    if n == 0 || i + 1 < n {
        return None;
    }
    v[i + 1 - n..=i].iter().copied().collect()
}

/// (long cross, short cross) of the Ulcer line against its signal.
fn ulcer_cross(x: &Ctx, src: i64, hh_p: usize, smooth: usize, ema: bool, len: usize, running: bool) -> (Vec<bool>, Vec<bool>) {
    let n = x.c.len();
    let s: Vec<f64> = match src {
        1 => x.o.clone(),
        2 => x.h.clone(),
        3 => x.l.clone(),
        _ => x.c.clone(),
    };
    let sv: Vec<Option<f64>> = s.iter().map(|&v| Some(v)).collect();
    // HHV over 0 bars (the own-or-fresh AFL default) is taken as empty
    let hh: Vec<Option<f64>> = (0..n).map(|i| window_opt(&sv, i, hh_p).map(|w| w.into_iter().fold(f64::MIN, f64::max))).collect();
    let dd2: Vec<Option<f64>> = (0..n).map(|i| hh[i].filter(|&h| h != 0.0).map(|h| (100.0 * (s[i] - h) / h).powi(2))).collect();
    let sm = smooth.max(1);
    let ulcer: Vec<Option<f64>> = (0..n).map(|i| window_opt(&dd2, i, sm).map(|w| (w.iter().sum::<f64>() / sm as f64).sqrt())).collect();
    let len = len.max(1);
    let signal: Vec<Option<f64>> = if ema {
        // EMA seeded with the simple average of the first `len` Ulcer values
        let a = 2.0 / (len as f64 + 1.0);
        let mut out = vec![None; n];
        let mut prev: Option<f64> = None;
        for i in 0..n {
            prev = match (prev, ulcer[i]) {
                (Some(p), Some(u)) => Some(p + a * (u - p)),
                (None, Some(_)) => window_opt(&ulcer, i, len).map(|w| w.iter().sum::<f64>() / len as f64),
                _ => None,
            };
            out[i] = prev;
        }
        out
    } else {
        (0..n).map(|i| window_opt(&ulcer, i, len).map(|w| w.iter().sum::<f64>() / len as f64)).collect()
    };
    let nz = |v: Option<f64>| v.unwrap_or(0.0);
    let le = |a: Option<f64>, b: Option<f64>| matches!((a, b), (Some(a), Some(b)) if a <= b);
    if running {
        let sx = (0..n).map(|i| gt(ulcer[i], signal[i]) && nz(signal[i]) != 0.0).collect();
        let lx = (0..n).map(|i| le(ulcer[i], signal[i]) && nz(signal[i]) != 0.0).collect();
        (lx, sx)
    } else {
        let (cu, cd) = (cross(&ulcer, &signal), cross(&signal, &ulcer));
        let first = |i: usize| nz(if i == 0 { None } else { signal[i - 1] }) == 0.0;
        let sx = (0..n).map(|i| cu[i] || (gt(ulcer[i], signal[i]) && first(i))).collect();
        let lx = (0..n).map(|i| cd[i] || (le(ulcer[i], signal[i]) && first(i))).collect();
        (lx, sx)
    }
}

// ---------------------------------------------------------------- the loop's position bookkeeping

struct Eng<'a> {
    x: &'a Ctx<'a>,
    n: usize,
    live: bool,
    hsl_pct: f64,
    pb_pct: f64,
    tsl_pct: f64,
    buf: f64,
    atr_on: bool,
    mult: f64,
    xth: usize,
    atrv: Vec<Option<f64>>,
    avg: Vec<f64>,
    llv: Vec<Option<f64>>,
    hhv: Vec<Option<f64>>,
    lpos: bool,
    spos: bool,
    lhsl: Vec<Option<f64>>,
    lpb: Vec<Option<f64>>,
    ltsl: Vec<Option<f64>>,
    shsl: Vec<Option<f64>>,
    spb: Vec<Option<f64>>,
    stsl: Vec<Option<f64>>,
    atrl: Vec<Option<f64>>,
    atrs: Vec<Option<f64>>,
    clow: Vec<Option<f64>>,
    chigh: Vec<Option<f64>>,
    s: Px,
}

fn nz(v: Option<f64>) -> f64 {
    v.unwrap_or(0.0)
}
fn max_opt(a: Option<f64>, b: Option<f64>) -> Option<f64> {
    match (a, b) {
        (Some(a), Some(b)) => Some(a.max(b)),
        (a, b) => a.or(b),
    }
}
fn min_opt(a: Option<f64>, b: Option<f64>) -> Option<f64> {
    match (a, b) {
        (Some(a), Some(b)) => Some(a.min(b)),
        (a, b) => a.or(b),
    }
}

impl<'a> Eng<'a> {
    fn new(x: &'a Ctx<'a>, g: &dyn Fn(&str) -> f64, xth_null_when_off: bool) -> Eng<'a> {
        let n = x.c.len();
        let xth = g("xth").round().max(0.0) as usize;
        let k = xth + 1;
        let llv = (0..n).map(|i| if (xth == 0 && xth_null_when_off) || i + 1 < k { None } else { Some(x.l[i + 1 - k..=i].iter().copied().fold(f64::MAX, f64::min)) }).collect();
        let hhv = (0..n).map(|i| if (xth == 0 && xth_null_when_off) || i + 1 < k { None } else { Some(x.h[i + 1 - k..=i].iter().copied().fold(f64::MIN, f64::max)) }).collect();
        let none = vec![None; n];
        Eng {
            x,
            n,
            live: g("exit_at").round() as i64 == 1,
            hsl_pct: g("hsl_pct"),
            pb_pct: g("pb_pct"),
            tsl_pct: g("tsl_pct"),
            buf: g("buffer"),
            atr_on: g("atr_on").round() as i64 == 1,
            mult: g("atr_mult"),
            xth,
            atrv: mean_tr(&x.h, &x.l, &x.c, g("atr_period").round().max(1.0) as usize),
            avg: avg_of(&x.o, &x.h, &x.l, &x.c, open_first(g("atr_avg").round() as i64)),
            llv,
            hhv,
            lpos: false,
            spos: false,
            lhsl: none.clone(),
            lpb: none.clone(),
            ltsl: none.clone(),
            shsl: none.clone(),
            spb: none.clone(),
            stsl: none.clone(),
            atrl: none.clone(),
            atrs: none.clone(),
            clow: none.clone(),
            chigh: none,
            s: Px::new(n),
        }
    }

    fn carry(&mut self, i: usize) {
        for v in [&mut self.lhsl, &mut self.lpb, &mut self.ltsl, &mut self.shsl, &mut self.spb, &mut self.stsl, &mut self.atrl, &mut self.atrs, &mut self.clow, &mut self.chigh] {
            v[i] = v[i - 1];
        }
    }

    /// The AFL's long exit check; `signal` is the strategy's own exit signal. Returns true when it sold.
    fn long_exit(&mut self, i: usize, signal: bool, why_signal: &'static str) -> bool {
        if !self.lpos {
            return false;
        }
        let x = self.x;
        let (o, h, l, c) = (x.o[i], x.h[i], x.l[i], x.c[i]);
        let lo = if self.live { l } else { c };
        let hi = if self.live { h } else { c };
        let hsl_hit = self.hsl_pct > 0.0 && lt(Some(lo), self.lhsl[i]);
        let pb_hit = self.pb_pct > 0.0 && gt(Some(hi), self.lpb[i]);
        let tsl_hit = nz(self.ltsl[i - 1]) != 0.0 && lt(Some(lo), self.ltsl[i - 1]);
        let atr_hit = nz(self.atrl[i - 1]) != 0.0 && lt(Some(c), self.atrl[i - 1]);
        let xth_hit = self.xth > 0 && lt(Some(lo), self.clow[i]);
        let last = i + 1 == self.n;
        let mut sold = false;
        if hsl_hit || pb_hit || tsl_hit || atr_hit || signal || xth_hit || last {
            let mut px = c;
            let b = self.buf * 0.01;
            if self.live && hsl_hit {
                let v = self.lhsl[i].unwrap();
                px = if o < v { l.max(o * (1.0 - b)) } else { v };
            }
            if self.live && pb_hit {
                let v = self.lpb[i].unwrap();
                px = if o > v { l.max(o * (1.0 - b)) } else { v };
            }
            let why = if hsl_hit {
                "Hard SL"
            } else if pb_hit {
                "Profit booking"
            } else if tsl_hit {
                "Trailing SL"
            } else if atr_hit {
                "ATR trail"
            } else if xth_hit {
                "Xth candle low"
            } else if signal {
                why_signal
            } else {
                LAST_BAR
            };
            self.s.sell(i, px, why);
            for v in [&mut self.lhsl, &mut self.lpb, &mut self.ltsl, &mut self.atrl, &mut self.clow] {
                v[i] = None;
            }
            self.lpos = false;
            sold = true;
        }
        if nz(self.ltsl[i - 1]) != 0.0 && self.lpos {
            self.ltsl[i] = max_opt(Some(h * (1.0 - self.tsl_pct * 0.01)), self.ltsl[i - 1]);
        }
        if self.atr_on && self.lpos {
            self.atrl[i] = max_opt(self.atrv[i].map(|a| self.avg[i] - a * self.mult), self.atrl[i - 1]);
        }
        sold
    }

    /// The AFL's short exit check (the short ATR hit is computed there but not used in the cover condition).
    fn short_exit(&mut self, i: usize, signal: bool, why_signal: &'static str) -> bool {
        if !self.spos {
            return false;
        }
        let x = self.x;
        let (o, h, l, c) = (x.o[i], x.h[i], x.l[i], x.c[i]);
        let lo = if self.live { l } else { c };
        let hi = if self.live { h } else { c };
        let hsl_hit = self.hsl_pct > 0.0 && gt(Some(hi), self.shsl[i]);
        let pb_hit = self.pb_pct > 0.0 && lt(Some(lo), self.spb[i]);
        let tsl_hit = nz(self.stsl[i - 1]) != 0.0 && gt(Some(hi), self.stsl[i - 1]);
        let xth_hit = self.xth > 0 && gt(Some(hi), self.chigh[i]);
        let last = i + 1 == self.n;
        let mut covered = false;
        if hsl_hit || pb_hit || tsl_hit || signal || xth_hit || last {
            let mut px = c;
            let b = self.buf * 0.01;
            if self.live && hsl_hit {
                let v = self.shsl[i].unwrap();
                px = if o > v { h.min(o * (1.0 + b)) } else { v };
            }
            if self.live && pb_hit {
                let v = self.spb[i].unwrap();
                px = if o < v { h.min(o * (1.0 + b)) } else { v };
            }
            let why = if hsl_hit {
                "Hard SL"
            } else if pb_hit {
                "Profit booking"
            } else if tsl_hit {
                "Trailing SL"
            } else if xth_hit {
                "Xth candle high"
            } else if signal {
                why_signal
            } else {
                LAST_BAR
            };
            self.s.cover(i, px, why);
            for v in [&mut self.shsl, &mut self.spb, &mut self.stsl, &mut self.atrs, &mut self.chigh] {
                v[i] = None;
            }
            self.spos = false;
            covered = true;
        }
        if nz(self.stsl[i - 1]) != 0.0 && self.spos {
            self.stsl[i] = min_opt(Some(l * (1.0 + self.tsl_pct * 0.01)), self.stsl[i - 1]);
        }
        if self.atr_on && self.spos {
            self.atrs[i] = min_opt(self.atrv[i].map(|a| self.avg[i] + a * self.mult), self.atrs[i - 1]);
        }
        covered
    }

    fn enter_long(&mut self, i: usize) {
        let c = self.x.c[i];
        self.s.buy[i] = true;
        self.lhsl[i] = (self.hsl_pct > 0.0).then(|| c * (1.0 - self.hsl_pct * 0.01));
        self.lpb[i] = (self.pb_pct > 0.0).then(|| c * (1.0 + self.pb_pct * 0.01));
        self.ltsl[i] = (self.tsl_pct > 0.0).then(|| c * (1.0 - self.tsl_pct * 0.01));
        self.atrl[i] = if self.atr_on { self.atrv[i].map(|a| self.avg[i] - a * self.mult) } else { None };
        self.clow[i] = self.llv[i];
        self.lpos = true;
    }

    fn enter_short(&mut self, i: usize) {
        let c = self.x.c[i];
        self.s.short[i] = true;
        self.shsl[i] = (self.hsl_pct > 0.0).then(|| c * (1.0 + self.hsl_pct * 0.01));
        self.spb[i] = (self.pb_pct > 0.0).then(|| c * (1.0 - self.pb_pct * 0.01));
        self.stsl[i] = (self.tsl_pct > 0.0).then(|| c * (1.0 + self.tsl_pct * 0.01));
        self.atrs[i] = if self.atr_on { self.atrv[i].map(|a| self.avg[i] + a * self.mult) } else { None };
        self.chigh[i] = self.hhv[i];
        self.spos = true;
    }
}

const WHY_SX: &str = "Ulcer rose through its signal";
const WHY_LX: &str = "Ulcer fell through its signal";

fn crosses(x: &Ctx, g: &dyn Fn(&str) -> f64) -> (Vec<bool>, Vec<bool>) {
    let gi = |k: &str| g(k).round() as i64;
    ulcer_cross(x, gi("src"), gi("hh_period").max(0) as usize, gi("smooth").max(1) as usize, gi("sig_type") == 1, gi("sig_len").max(1) as usize, gi("cross_mode") == 1)
}

/// RSI level filter as the AFLs write it; `nonzero` adds the own-or-fresh AFL's `Nz(rsi) != 0`.
fn rsi_ok(x: &Ctx, g: &dyn Fn(&str) -> f64, null_below_2: bool, nonzero: bool) -> Vec<bool> {
    let n = x.c.len();
    if g("rsi_on").round() as i64 != 1 {
        return vec![true; n];
    }
    let period = g("rsi_period").round() as i64;
    let rv = if null_below_2 && period <= 1 { vec![None; n] } else { rsi_opt(&x.c, period.max(1) as usize) };
    let thr = Some(g("rsi_thr"));
    let above = g("rsi_side").round() as i64 == 1;
    (0..n).map(|i| (if above { gt(rv[i], thr) } else { lt(rv[i], thr) }) && (!nonzero || nz(rv[i]) != 0.0)).collect()
}

// ---------------------------------------------------------------- Futures_UlcerIndex_Final and "exit only Ulcer"

fn basic(x: &Ctx, g: &dyn Fn(&str) -> f64, from: usize, to: usize, st_variant: bool) -> Px {
    let n = x.c.len();
    let gi = |k: &str| g(k).round() as i64;
    let gu = |k: &str| g(k).round().max(1.0) as usize;
    let (lx, sx) = crosses(x, g);
    let tt = gi("trade_type");
    let (t_long, t_short) = (tt == 0 || tt == 2, tt == 1 || tt == 2);
    let nth = g("nth");
    let dc = |i: usize| from <= i && i <= to;
    // the "exit only Ulcer" AFL's filters (all off in the Final AFL)
    let (mut rsi_f, mut bias_ok, mut ste_on, mut ste_cross, mut ste_bear) = (vec![true; n], vec![true; n], false, vec![false; n], false);
    if st_variant {
        rsi_f = rsi_ok(x, g, true, false);
        if gi("bias_on") == 1 {
            let (st, cm) = st_daily(x, gi("bias_tf"), gu("bias_period"), g("bias_factor"), gi("bias_avg"));
            let bear = gi("bias_side") == 1;
            bias_ok = (0..n).map(|i| gt(cm[i], st[i]) != bear).collect();
        }
        ste_on = gi("ste_on") == 1;
        if ste_on {
            let (st, cm) = st_daily(x, gi("ste_tf"), gu("ste_period"), g("ste_factor"), gi("ste_avg"));
            ste_cross = cross(&cm, &st);
        }
        ste_bear = gi("ste_side") == 1;
    }
    let mut e = Eng::new(x, g, false);
    let mut count = 0.0;
    let (mut lflag, mut sflag) = (false, false);
    for i in 1..n {
        e.carry(i);
        let last = i + 1 == n;
        if e.lpos || e.spos {
            if e.long_exit(i, sx[i], WHY_SX) {
                lflag = false;
            }
            if e.short_exit(i, lx[i], WHY_LX) {
                sflag = false;
            }
        }
        if (lx[i] && !lflag && !e.lpos && dc(i)) || (sx[i] && !sflag && !e.spos && dc(i)) {
            count = 0.0;
            lflag = lx[i];
            sflag = sx[i];
        } else {
            count += 1.0;
        }
        if !e.lpos || !e.spos {
            let ulcer_long = !e.lpos && dc(i) && count == nth && lflag && rsi_f[i] && bias_ok[i] && t_long && !last;
            let st_long = st_variant && !e.lpos && dc(i) && ste_on && (ste_cross[i] != ste_bear) && bias_ok[i] && rsi_f[i] && t_long && !last;
            if ulcer_long || st_long {
                e.enter_long(i);
            }
            let ulcer_short = !e.spos && dc(i) && count == nth && sflag;
            let go_short = if st_variant {
                // AFL: A AND B AND C AND shortFlag OR (ST cond) AND (trade type short / any) AND not the last bar
                ulcer_short || ((!ste_on || (ste_cross[i] == ste_bear)) && t_short && !last)
            } else {
                ulcer_short && t_short && !last
            };
            if go_short {
                e.enter_short(i);
            }
        }
    }
    let mut s = e.s;
    if !st_variant {
        s.exrem_all();
    }
    s
}

// ---------------------------------------------------------------- "exit own or fresh of opposite"

fn own(x: &Ctx, g: &dyn Fn(&str) -> f64, from: usize, to: usize) -> Px {
    let n = x.c.len();
    let gi = |k: &str| g(k).round() as i64;
    let gu = |k: &str| g(k).round().max(1.0) as usize;
    let (lx, sx) = crosses(x, g);
    let tt = gi("trade_type");
    let (t_long, t_short) = (tt == 0 || tt == 2, tt == 1 || tt == 2);
    let nth = g("nth");
    let dc = |i: usize| from <= i && i <= to;
    let ste_on = gi("ste_on") == 1;
    let (st_buy, st_sell): (Vec<bool>, Vec<bool>) = if ste_on {
        // the crosses on the timeframe's own bars, then Ref(-1) / expandFirst / Ref(+1)
        let t = tf_bars(x, gi("ste_tf"));
        let st = team_st(&avg_of(&t.o, &t.h, &t.l, &t.c, gi("ste_avg")), &t.h, &t.l, &t.c, gu("ste_period"), g("ste_factor"));
        let cl: Vec<Option<f64>> = t.c.iter().map(|&v| Some(v)).collect();
        let b: Vec<Option<bool>> = cross(&cl, &st).into_iter().map(Some).collect();
        let s: Vec<Option<bool>> = cross(&st, &cl).into_iter().map(Some).collect();
        (htf(&b, &t).into_iter().map(|v| v.unwrap_or(false)).collect(), htf(&s, &t).into_iter().map(|v| v.unwrap_or(false)).collect())
    } else {
        (vec![false; n], vec![false; n])
    };
    let bias_on = gi("bias_on") == 1;
    let (f_buy, f_sell): (Vec<bool>, Vec<bool>) = if bias_on {
        let t = tf_bars(x, gi("bias_tf"));
        let st = team_st(&avg_of(&t.o, &t.h, &t.l, &t.c, gi("bias_avg")), &t.h, &t.l, &t.c, gu("bias_period"), g("bias_factor"));
        let bull: Vec<Option<bool>> = (0..t.c.len()).map(|k| Some(gt(Some(t.c[k]), st[k]))).collect();
        let bear: Vec<Option<bool>> = (0..t.c.len()).map(|k| Some(lt(Some(t.c[k]), st[k]))).collect();
        let (bull, bear): (Vec<bool>, Vec<bool>) = (htf(&bull, &t).into_iter().map(|v| v.unwrap_or(false)).collect(), htf(&bear, &t).into_iter().map(|v| v.unwrap_or(false)).collect());
        if gi("bias_side") == 0 { (bull, bear) } else { (bear, bull) }
    } else {
        (vec![false; n], vec![false; n])
    };
    let rsi_f = rsi_ok(x, g, false, true);

    let mut e = Eng::new(x, g, true);
    let mut count = 0.0;
    let (mut lflag, mut sflag) = (false, false);
    let (mut l_type, mut s_type) = (0i64, 0i64);
    let (mut l_chk, mut s_chk) = (false, false);
    let (mut l_cur, mut s_cur): (Option<bool>, Option<bool>) = (None, None);
    let (mut l_wait, mut s_wait) = (false, false);
    let (mut l_pro, mut s_pro) = (false, false);

    for i in 1..n {
        e.carry(i);
        let last = i + 1 == n;
        // the AFL runs the same exit block before and after the entries
        let exits = |e: &mut Eng, lflag: &mut bool, sflag: &mut bool, st: &mut [&mut bool; 6], l_type: &mut i64, s_type: &mut i64, l_cur: &mut Option<bool>, s_cur: &mut Option<bool>| {
            let [l_chk, s_chk, l_wait, s_wait, l_pro, s_pro] = st;
            if e.lpos {
                if **l_pro {
                    **l_chk = st_sell[i] || sx[i];
                }
                if *l_type == 1 && *l_cur != Some(true) {
                    *l_cur = Some(st_sell[i]);
                } else if *l_type == 2 && *l_cur != Some(true) {
                    *l_cur = Some(sx[i]);
                }
                let cur = *l_cur == Some(true);
                if cur && !**l_wait {
                    **l_wait = true;
                } else if !cur || **l_wait {
                    **l_chk = st_sell[i] || sx[i];
                }
                if st_sell[i] && lx[i] {
                    **l_pro = true;
                    **l_chk = false;
                }
                let why = if **l_chk { if st_sell[i] { "ST sell signal" } else { WHY_SX } } else { "Bull-bear ST filter" };
                if e.long_exit(i, **l_chk || (bias_on && f_sell[i]), why) {
                    *l_cur = None;
                    **l_wait = false;
                    **l_chk = false;
                    **l_pro = false;
                    *lflag = false;
                    *l_type = 0;
                }
            }
            if e.spos {
                if **s_pro {
                    **s_chk = st_buy[i] || lx[i];
                }
                if *s_type == 1 && *s_cur != Some(true) {
                    *s_cur = Some(st_buy[i]);
                } else if *s_type == 2 && *s_cur != Some(true) {
                    *s_cur = Some(lx[i]);
                }
                let cur = *s_cur == Some(true);
                if cur && !**s_wait {
                    **s_wait = true;
                } else if !cur || **s_wait {
                    **s_chk = st_buy[i] || lx[i];
                }
                if st_buy[i] && sx[i] {
                    **s_pro = true;
                    **s_chk = false;
                }
                let why = if **s_chk { if st_buy[i] { "ST buy signal" } else { WHY_LX } } else if lx[i] { WHY_LX } else { "Bull-bear ST filter" };
                if e.short_exit(i, **s_chk || lx[i] || (bias_on && f_buy[i]), why) {
                    // the AFL resets 'currentStatus' here, not shortCurrentStatus, and never shortWait
                    *s_type = 0;
                    *sflag = false;
                    **s_chk = false;
                    **s_pro = false;
                }
            }
        };
        exits(&mut e, &mut lflag, &mut sflag, &mut [&mut l_chk, &mut s_chk, &mut l_wait, &mut s_wait, &mut l_pro, &mut s_pro], &mut l_type, &mut s_type, &mut l_cur, &mut s_cur);
        let fb = |i: usize| !bias_on || f_buy[i];
        let fs = |i: usize| !bias_on || f_sell[i];
        if (lx[i] && !lflag && !e.lpos && fb(i) && dc(i) && t_long) || (sx[i] && !sflag && !e.spos && fs(i) && dc(i) && t_short) {
            count = 0.0;
            if !lflag {
                lflag = lx[i];
            }
            if !sflag {
                sflag = sx[i];
            }
        } else {
            count += 1.0;
        }
        if !e.lpos || !e.spos {
            if !e.lpos && dc(i) && count >= nth && (lflag || (ste_on && st_buy[i])) && rsi_f[i] && fb(i) && t_long && !last {
                e.enter_long(i);
                l_type = if st_buy[i] { 2 } else if lflag { 1 } else { 0 };
            }
            if !e.spos && dc(i) && count >= nth && (sflag || (ste_on && st_sell[i])) && rsi_f[i] && fs(i) && t_short && !last {
                e.enter_short(i);
                s_type = if sflag { 1 } else if st_sell[i] { 2 } else { 0 };
            }
        }
        exits(&mut e, &mut lflag, &mut sflag, &mut [&mut l_chk, &mut s_chk, &mut l_wait, &mut s_wait, &mut l_pro, &mut s_pro], &mut l_type, &mut s_type, &mut l_cur, &mut s_cur);
    }
    e.s
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
        FINAL => basic(x, &g, from, to, false),
        EXIT_ULCER => basic(x, &g, from, to, true),
        EXIT_OWN => own(x, &g, from, to),
        _ => return None,
    };
    Some(backtest(x, &s, from, to))
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let name = catalogue().into_iter().find(|d| d.id == id)?.name;
    Some((match id {
        FINAL => "ytui_final",
        EXIT_ULCER => "ytui_exit_only_ulcer",
        EXIT_OWN => "ytui_exit_own_or_fresh",
        _ => return None,
    }, name, "Ulcer Performance Index"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::books::ytroc::tests::{bars, wave};
    use crate::Row;

    fn go(id: &str, r: &[Row], kv: &[(&str, f64)]) -> Vec<Trade> {
        let x = Ctx::new(r, r, &[]);
        let p: Params = kv.iter().map(|(k, v)| (k.to_string(), *v)).collect();
        run(id, &x, 0, r.len() - 1, &p).unwrap()
    }
    /// Rally, sharp drop, recovery: the Ulcer line rises through its signal in the drop and falls back after.
    fn swing_rows() -> Vec<Row> {
        let mut v = Vec::new();
        let mut c = 100.0;
        for k in 0..40 {
            c += if k < 15 { 1.0 } else if k < 22 { -3.0 } else { 1.5 };
            v.push((c, c + 0.5, c - 0.5, c));
        }
        bars(&v)
    }
    const SMALL: [(&str, f64); 3] = [("hh_period", 3.0), ("smooth", 2.0), ("sig_len", 3.0)];

    #[test]
    fn ulcer_cross_marks_the_drop_and_recovery() {
        let r = swing_rows();
        let x = Ctx::new(&r, &r, &[]);
        let (lx, sx) = ulcer_cross(&x, 0, 3, 2, false, 3, false);
        let first_s = sx.iter().position(|&b| b).unwrap();
        let first_l = lx.iter().position(|&b| b).unwrap();
        // the signal's first bar (index 2 + 1 + 2 = 5) gives the initial state (flat rally: ulcer 0 <= signal 0)
        assert_eq!(first_l, 5);
        assert!(first_s >= 15 && first_s <= 17, "{first_s}");
        // HHV over 0 bars: nothing
        let (lx0, sx0) = ulcer_cross(&x, 0, 0, 2, false, 3, false);
        assert!(lx0.iter().chain(&sx0).all(|&b| !b));
    }

    #[test]
    fn final_long_only_enters_on_long_cross_and_exits_on_short_cross() {
        let r = swing_rows();
        let t = go(FINAL, &r, &SMALL);
        assert!(!t.is_empty() && t.iter().all(|t| t.dir > 0.0));
        let x = Ctx::new(&r, &r, &[]);
        let (lx, sx) = ulcer_cross(&x, 0, 3, 2, false, 3, false);
        assert!(lx[t[0].legs[0].i]);
        assert!(t[0].open || sx[t[0].exit_i]);
    }

    #[test]
    fn exit_only_ulcer_shorts_even_with_trade_type_long() {
        let r = swing_rows();
        let t = go(EXIT_ULCER, &r, &SMALL);
        assert!(t.iter().any(|t| t.dir < 0.0), "the AFL's precedence bug writes Short signals");
        assert!(t.iter().any(|t| t.dir > 0.0));
    }

    #[test]
    fn own_defaults_give_no_trades_and_do_not_panic() {
        let r = wave(300);
        assert!(go(EXIT_OWN, &r, &[]).is_empty());
        let t = go(EXIT_OWN, &r, &[("hh_period", 6.0)]);
        assert!(t.iter().all(|t| t.dir > 0.0));
    }

    #[test]
    fn live_hsl_fill_for_a_long() {
        // long at the close of the first long cross; HSL 1% hit live
        let r = swing_rows();
        let base = go(FINAL, &r, &SMALL);
        let e = base[0].legs[0].i;
        let mut v: Vec<(f64, f64, f64, f64)> = r.iter().map(|b| (b.open, b.high, b.low, b.close)).collect();
        let c = v[e].3;
        v[e + 1] = (c, c + 0.5, c * 0.98, c);
        let r2 = bars(&v);
        let t = go(FINAL, &r2, &[SMALL[0], SMALL[1], SMALL[2], ("hsl_pct", 1.0), ("exit_at", 1.0)]);
        let tr = t.iter().find(|t| t.legs[0].i == e).unwrap();
        assert_eq!(tr.exit_i, e + 1);
        assert!((tr.exit_px - c * 0.99).abs() < 1e-9);
    }

    #[test]
    fn every_strategy_runs() {
        let r = wave(400);
        let x = Ctx::new(&r, &r, &[]);
        let sets: [&[(&str, f64)]; 4] = [
            &[],
            &[("hh_period", 6.0), ("trade_type", 2.0), ("nth", 1.0), ("xth", 2.0), ("exit_at", 1.0), ("hsl_pct", 2.0), ("pb_pct", 3.0), ("tsl_pct", 2.0), ("atr_on", 1.0)],
            &[("hh_period", 10.0), ("sig_type", 1.0), ("cross_mode", 1.0), ("trade_type", 1.0), ("ste_on", 1.0), ("ste_tf", 1.0), ("bias_on", 1.0), ("bias_tf", 2.0), ("rsi_on", 1.0), ("rsi_thr", 60.0)],
            &[("hh_period", 14.0), ("trade_type", 2.0), ("ste_on", 1.0), ("ste_side", 1.0), ("rsi_on", 1.0), ("rsi_side", 1.0), ("rsi_thr", 30.0), ("bias_on", 1.0), ("bias_side", 1.0)],
        ];
        for id in IDS {
            for kv in sets {
                let p: Params = kv.iter().map(|(k, v)| (k.to_string(), *v)).collect();
                let t = run(id, &x, 60, r.len() - 1, &p).unwrap();
                assert!(t.iter().all(|t| t.exit_i >= t.legs[0].i && t.legs[0].i >= 60 && t.exit_px.is_finite()), "{id}");
            }
            assert!(family_of(id).is_some() && params(id).is_some());
        }
        assert_eq!(catalogue().len(), IDS.len());
    }
}
