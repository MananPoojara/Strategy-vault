//! YT Strategies · Rank From Lowest Low: the team's AmiBroker AFLs in "Old Topics/Rank From Lowest Low To ST Buy of
//! Nifty and Buy Stocks" (folder "AFLs", sub-folders "For FO" and "For Nifty50 Stocks_Aux AFL"), which buy stocks
//! when NIFTY's daily Supertrend turns up and rank them by their rise from the stock's low on the day NIFTY made
//! its lowest low of the down leg. Coded from the AFLs as written, with the AFLs' own default settings; there is
//! no book PDF and nothing here is checked against the team's workbooks.
//!
//! What runs per stock: NIFTY's ST buy signal (the AFLs' Supertrend, `SetForeign("NSENIFTY")` / `"NIFTY"`; here
//! NIFTY 50, the market benchmark, aligned to the stock's dates with missing days carried forward as AmiBroker's
//! Foreign does), the stock's own rise since NIFTY's lowest-low date (used as an entry threshold by the Threshold
//! AFLs), the bar guards and the exits. The ranking (PositionScore = 100 + that rise) and the one-position limit
//! (MAXIMUM POSITION 1) only choose between stocks and are not modelled.
//!
//! AmiBroker behaviour followed:
//! - the team's Supertrend: ATRVal = sum of the last `period` ATR(1) / period (ATR(1) Null on the first bar),
//!   bands avg +/- factor x ATRVal, IsUp starts true, from bar `period`: up when C > Up[i-1], down when
//!   C < Dn[i-1], bands ratchet without a reset on a flip, ST = IsUp ? Dn : Up (`ytroc::team_st`); a Null read in
//!   the bar loop is -1e10;
//! - the AFL bar loop runs over the whole history and writes Buy / Sell with its own SellPrice; the backtester
//!   opens trades only in the test window, exits first and then enters on the same bar; a Sell while flat is
//!   ignored; the loop's sell on the data's last bar is reported as a position still open; exit prices are kept
//!   inside the bar's range (price bound check).
#![allow(dead_code)]
use crate::books::ytroc::{avg_of, mean_tr};
use crate::books::ytsw::{cross_up, trade};
use crate::books::ytwh::bound;
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "yt_rank";
pub const THRESHOLD: &str = "ytrank_threshold";
pub const FINAL: &str = "ytrank_final";
pub const BASIC: &str = "ytrank_basic";
pub const IDS: [&str; 3] = [THRESHOLD, FINAL, BASIC];

const NULL: f64 = -1e10;

fn def(id: &'static str, name: &'static str, rules: &'static [&'static str], notes: &'static str) -> StrategyDef {
    StrategyDef {
        id,
        name,
        group: "YT Strategies · Rank From Lowest Low",
        session: 0,
        dir: "Long",
        status: Status::Live,
        needs_volume: false,
        needs_vix: false,
        needs_series: &["BENCH.High", "BENCH.Low", "BENCH.Close"],
        book: BOOK,
        rules,
        notes,
        claim: None,
    }
}

const ST_NIFTY: &str = "F|NIFTY ST: the team's daily Supertrend of NIFTY 50 (period 5, factor 1, on (H+L)/2, ATR = simple average of ATR(1), bands ratchet without a reset on a flip)";
const ROC: &str = "F|Rise from the lowest low: in each NIFTY down leg (NIFTY close below its ST until it closes above again) find NIFTY's lowest low; the stock's low on the day NIFTY made that low is the base; on NIFTY ST buy days the rise = (stock close / base - 1) x 100";
const EXIT_LOOP: &str = "X|Optional exits (all off by default): the stock's own ST (5, 1, (H+L)/2) with the close below it, hard SL %, profit booking %, trailing SL % (from the closes), live (at the level, or the open less the buffer % on a gap, not below the low) or at the close; with the defaults only the end of the data closes a trade";
const NOTE: &str = " Runs on one stock at a time; the team ran it over F&O stocks (For FO) and NIFTY 50 stocks (For Nifty50 Stocks_Aux AFL); the ranking (PositionScore = 100 + the rise) and the one-position limit are not modelled. NIFTY 50 (the market benchmark) stands for the AFL's NSENIFTY / NIFTY; only the daily timeframe of the AFL's ST timeframe choice is offered (the default).";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![
        def(THRESHOLD, "NIFTY ST Buy, Stock Rise From Lowest Low Above Threshold", &[
            ST_NIFTY, ROC,
            "F|NIFTY filter: NIFTY close above its ST (Bullish, default; also Bearish or Any)",
            "E|Buy the stock at the close on the bar NIFTY's close crosses above its ST, when the filter holds, the rise is above the threshold (5%) and the stock's open differs from its high, low and close; only when flat",
            EXIT_LOOP,
            "X|Dead-stock rule as written: if the stock's close equals each of the next 4 closes, the bar takes no entry and an open position is sold at that close (the rule reads up to 4 bars ahead)",
        ], concat!("RankingFromLowestLowSuperTrendSell_Final (Threshold).afl (For FO) and RankingFromLowestLowSuperTrendSell for AUX (Threshold).afl (same logic), coded as the AFL is written. The AFL's Penny Stock Price and Turnover parameters are declared but unused.", " Runs on one stock at a time; the team ran it over F&O stocks (For FO) and NIFTY 50 stocks (For Nifty50 Stocks_Aux AFL); the ranking (PositionScore = 100 + the rise) and the one-position limit are not modelled. NIFTY 50 (the market benchmark) stands for the AFL's NSENIFTY / NIFTY; only the daily timeframe of the AFL's ST timeframe choice is offered (the default).", " The loop runs over the whole history, so a position bought before the test window holds until its exit and blocks entries meanwhile (with the default exits, until the end of the data).")),
        def(FINAL, "NIFTY ST Buy, Stocks Ranked by Rise From Lowest Low", &[
            ST_NIFTY, ROC,
            "F|NIFTY filter: NIFTY close above its ST (Bullish, default; also Bearish or Any)",
            "E|Buy the stock at the close on the bar NIFTY's close crosses above its ST, when the filter holds and the stock's open differs from its high, low and close; only when flat (no threshold; the rise only ranks)",
            EXIT_LOOP,
            "X|Dead-stock rule as written: if the stock's close equals each of the next 4 closes, the bar takes no entry and an open position is sold at that close (reads up to 4 bars ahead)",
        ], concat!("RankingFromLowestLowSuperTrendSell_Final.afl (For FO) and RankingFromLowestLowSuperTrendSell for AUX.afl (same logic), coded as the AFL is written.", " Runs on one stock at a time; the team ran it over F&O stocks (For FO) and NIFTY 50 stocks (For Nifty50 Stocks_Aux AFL); the ranking (PositionScore = 100 + the rise) and the one-position limit are not modelled. NIFTY 50 (the market benchmark) stands for the AFL's NSENIFTY / NIFTY; only the daily timeframe of the AFL's ST timeframe choice is offered (the default).", " The loop runs over the whole history, so a position bought before the test window holds until its exit and blocks entries meanwhile.")),
        def(BASIC, "NIFTY ST Buy, Stocks Ranked by Rise From Lowest Low (Basic)", &[
            ST_NIFTY,
            "E|Buy the stock at the close on the bar NIFTY's close crosses above its ST, when the stock's close differs from its open, high and low; only when flat and inside the test window (no NIFTY filter)",
            "X|Optional exits (all off by default): the stock's own ST with the close below it, hard SL %, profit booking %, trailing SL %; with the defaults only the end of the test window / data closes a trade",
        ], concat!("RankingFromLowestLowSuperTrendSell (Basic).afl (For FO), coded as the AFL is written; the rise from the lowest low only ranks here. Quirks as written: the stock ST takes its ATR period and price point from the NIFTY ST settings; the trailing-SL gap fill compares the open with the hard-SL level. Buy / Sell pass through ExRem, so a re-entry on an exit bar is dropped.", " Runs on one stock at a time; the team ran it over F&O stocks; the ranking (PositionScore = 100 + the rise) and the one-position limit are not modelled. NIFTY 50 (the market benchmark) stands for the AFL's NIFTY; only the daily timeframe of the AFL's ST timeframe choice is offered (the default).")),
    ]
}


pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    let exits = |v: &mut Vec<ParamDef>| {
        v.extend([
            p("exit_close", "Exit at (0 live, 1 close)", 0.0, 0.0, 1.0, 1.0),
            p("tsl_close", "Exit at for TSL (0 live, 1 close)", 0.0, 0.0, 1.0, 1.0),
            p("pb_pct", "Profit booking % (0 off)", 0.0, 0.0, 100.0, 0.1),
            p("hsl_pct", "Hard SL % (0 off)", 0.0, 0.0, 100.0, 0.1),
            p("tsl_pct", "Trailing SL % (0 off)", 0.0, 0.0, 100.0, 0.1),
            p("buffer_pct", "Buffer %", 0.0, 0.0, 10.0, 0.1),
        ])
    };
    Some(match id {
        THRESHOLD | FINAL => {
            let mut v = vec![
                p("flt_type", "NIFTY filter (0 bullish, 1 bearish, 2 any)", 0.0, 0.0, 2.0, 1.0),
                p("flt_period", "Filter ST period", 5.0, 3.0, 9.0, 2.0),
                p("flt_factor", "Filter ST factor", 1.0, 1.0, 16.0, 1.0),
                p("flt_avg", "Filter ST price point (0 C, 1 O, 2 H, 3 L, 4 (O+C)/2, 5 (H+L)/2, 6 (H+L+C)/3, 7 (O+H+L+C)/4)", 5.0, 0.0, 7.0, 1.0),
                p("entry_period", "Entry ST period", 5.0, 3.0, 9.0, 2.0),
                p("entry_factor", "Entry ST factor", 1.0, 1.0, 16.0, 1.0),
                p("entry_avg", "Entry ST price point (0 C, 1 O, 2 H, 3 L, 4 (O+C)/2, 5 (H+L)/2, 6 (H+L+C)/3, 7 (O+H+L+C)/4)", 5.0, 0.0, 7.0, 1.0),
                p("buy_only", "Take ticker in stock ST buy only (0 off, 1 on)", 0.0, 0.0, 1.0, 1.0),
            ];
            if id == THRESHOLD {
                v.push(p("roc_thr", "ROC threshold %", 5.0, -100.0, 100.0, 0.1));
                v.push(p("thr_below", "Threshold comparison (0 above, 1 below)", 0.0, 0.0, 1.0, 1.0));
            }
            v.extend([
                p("exit_st", "Exit on the stock ST (0 off, 1 on)", 0.0, 0.0, 1.0, 1.0),
                p("exit_period", "Stock ST period", 5.0, 3.0, 9.0, 2.0),
                p("exit_factor", "Stock ST factor", 1.0, 1.0, 16.0, 1.0),
                p("exit_avg", "Stock ST price point (0 C, 1 O, 2 H, 3 L, 4 (O+C)/2, 5 (H+L)/2, 6 (H+L+C)/3, 7 (O+H+L+C)/4)", 5.0, 0.0, 7.0, 1.0),
            ]);
            exits(&mut v);
            v
        }
        BASIC => {
            let mut v = vec![
                p("st_period", "NIFTY ST period", 5.0, 3.0, 9.0, 2.0),
                p("st_factor", "NIFTY ST factor", 1.0, 1.0, 16.0, 1.0),
                p("st_avg", "NIFTY ST price point, also the stock ST's (0 C, 1 O, 2 H, 3 L, 4 (O+C)/2, 5 (H+L)/2, 6 (H+L+C)/3, 7 (O+H+L+C)/4)", 5.0, 0.0, 7.0, 1.0),
                p("buy_only", "Take ticker in stock ST buy only (0 off, 1 on)", 0.0, 0.0, 1.0, 1.0),
                p("exit_st", "SuperTrend exit on the stock (0 off, 1 on)", 0.0, 0.0, 1.0, 1.0),
                p("exit_period", "Stock ST period (loop start; the ATR uses the NIFTY ST period)", 5.0, 3.0, 9.0, 2.0),
                p("exit_factor", "Stock ST factor", 1.0, 1.0, 16.0, 1.0),
            ];
            exits(&mut v);
            v
        }
        _ => return None,
    })
}

// ---------------------------------------------------------------- Supertrend and NIFTY series

/// The team's ST line with separate ATR period and loop start (the Basic AFL mixes them; otherwise both = period).
pub(crate) fn st_line(avg: &[f64], h: &[f64], l: &[f64], c: &[f64], atr_p: usize, start: usize, factor: f64) -> Vec<Option<f64>> {
    let n = c.len();
    let atr = mean_tr(h, l, c, atr_p.max(1));
    let mut up: Vec<Option<f64>> = (0..n).map(|i| atr[i].map(|a| avg[i] + a * factor)).collect();
    let mut dn: Vec<Option<f64>> = (0..n).map(|i| atr[i].map(|a| avg[i] - a * factor)).collect();
    let mut is_up = vec![true; n];
    for i in start.max(1)..n {
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

/// NIFTY 50 OHLC on the stock's dates, missing days carried forward; `s` = first bar with a NIFTY close.
struct Bench {
    o: Vec<f64>,
    h: Vec<f64>,
    l: Vec<f64>,
    c: Vec<f64>,
    s: usize,
}

fn ffill(v: &[f64]) -> Vec<f64> {
    let mut last = f64::NAN;
    v.iter().map(|&x| { if x.is_finite() { last = x; } last }).collect()
}

fn bench(x: &Ctx, need_open: bool) -> Option<Bench> {
    let c = ffill(x.series("BENCH.Close")?);
    let h = ffill(x.series("BENCH.High")?);
    let l = ffill(x.series("BENCH.Low")?);
    let o = match x.series("BENCH.Open") {
        Some(v) => ffill(v),
        None if need_open => return None,
        None => vec![f64::NAN; c.len()],
    };
    let s = c.iter().position(|v| v.is_finite())?;
    Some(Bench { o, h, l, c, s })
}

impl Bench {
    /// The team's ST of NIFTY from its first bar on (None before).
    fn st(&self, kind: i64, period: usize, factor: f64) -> Vec<Option<f64>> {
        let s = self.s;
        let avg = avg_of(&self.o[s..], &self.h[s..], &self.l[s..], &self.c[s..], kind);
        let st = st_line(&avg, &self.h[s..], &self.l[s..], &self.c[s..], period, period, factor);
        let mut out = vec![None; s];
        out.extend(st.into_iter().map(|v| v.filter(|v| v.is_finite())));
        out
    }
}

fn uses_open(kind: i64) -> bool {
    matches!(kind, 1 | 4 | 7)
}

/// NIFTY's lowest low of each down leg (close below its ST until a close above it), and the bars whose NIFTY low
/// equals that value, scanned backwards as the AFL does (SwingLowFlag).
fn swing_low_flags(b: &Bench, st: &[Option<f64>]) -> Vec<bool> {
    let n = b.c.len();
    const START: f64 = 9_999_999_999.0;
    let mut fin = vec![0.0f64; n];
    let (mut lowest, mut flag) = (START, START);
    for i in 1..n.saturating_sub(1) {
        let s = st[i].unwrap_or(NULL);
        if b.c[i] < s && flag != 1.0 {
            flag = 1.0;
        }
        if flag == 1.0 && b.l[i] < lowest {
            lowest = b.l[i];
        }
        if b.c[i] > s && flag == 1.0 {
            fin[i] = lowest;
            lowest = START;
            flag = START;
        }
    }
    let mut out = vec![false; n];
    let (mut value, mut on) = (0.0, false);
    for i in (0..n).rev() {
        if fin[i] != 0.0 {
            value = fin[i];
            on = true;
        }
        if on && value == b.l[i] {
            out[i] = true;
        }
    }
    out
}

// ---------------------------------------------------------------- backtest of the loop's arrays

struct Sigs {
    buy: Vec<bool>,
    sell: Vec<bool>,
    px: Vec<f64>,
    why: Vec<&'static str>,
}

const LAST: &str = "Last bar";

fn backtest(x: &Ctx, s: &Sigs, from: usize, to: usize) -> Vec<Trade> {
    let mut out = Vec::new();
    let mut pos: Option<usize> = None;
    for i in from..=to {
        if let Some(e) = pos {
            if s.sell[i] {
                if s.why[i] == LAST {
                    out.push(trade(1.0, e, x.c[e], i, x.c[i], "open", true));
                } else {
                    out.push(trade(1.0, e, x.c[e], i, bound(x, i, s.px[i]), s.why[i], false));
                }
                pos = None;
            }
        }
        if pos.is_none() && s.buy[i] {
            pos = Some(i);
        }
    }
    if let Some(e) = pos {
        out.push(trade(1.0, e, x.c[e], to, x.c[to], "open", true));
    }
    out
}

struct Exits {
    exit_close: bool,
    tsl_close: bool,
    pb: f64,
    hsl: f64,
    tsl: f64,
    buf: f64,
}

// ---------------------------------------------------------------- run

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(0.0, |d| d.def));
    let gi = |k: &str| g(k).round() as i64;
    let gu = |k: &str| g(k).round().max(1.0) as usize;
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let n = c.len();
    if n == 0 || from > to || to >= n {
        return Some(Vec::new());
    }
    let ex = Exits { exit_close: gi("exit_close") == 1, tsl_close: gi("tsl_close") == 1, pb: g("pb_pct"), hsl: g("hsl_pct"), tsl: g("tsl_pct"), buf: g("buffer_pct") };
    let mut s = Sigs { buy: vec![false; n], sell: vec![false; n], px: c.to_vec(), why: vec![""; n] };
    // the AFL's live-exit price of a hit level (Max(Low, Open less the buffer) on a gap)
    let gap = |i: usize| (o[i] * (1.0 - ex.buf * 0.01)).max(l[i]);
    if id == BASIC {
        let kind = gi("st_avg");
        let Some(b) = bench(x, uses_open(kind)) else { return Some(Vec::new()) };
        let nst = b.st(kind, gu("st_period"), g("st_factor"));
        let cross = cross_up(&b.c, &nst);
        let nifty_buy: Vec<bool> = (0..n).map(|i| cross[i] && c[i] != o[i] && c[i] != h[i] && c[i] != l[i]).collect();
        let st = st_line(&avg_of(o, h, l, c, kind), h, l, c, gu("st_period"), gu("exit_period"), g("exit_factor"));
        let stv = |i: usize| st[i].unwrap_or(NULL);
        let (exit_st, buy_only) = (gi("exit_st") == 1, gi("buy_only") == 1);
        let mut pos = false;
        let (mut pb, mut hsl, mut tsl) = (NULL, NULL, NULL);
        for i in 1..n {
            let in_range = i >= from && i <= to;
            if pos {
                let tsl_hit = ex.tsl > 0.0 && (if ex.tsl_close { c[i] } else { l[i] }) <= tsl;
                let hsl_hit = ex.hsl > 0.0 && (if ex.exit_close { c[i] } else { l[i] }) <= hsl;
                let pb_hit = ex.pb > 0.0 && (if ex.exit_close { c[i] } else { h[i] }) >= pb;
                let st_hit = exit_st && c[i] < stv(i);
                if st_hit || hsl_hit || tsl_hit || pb_hit || i == n - 1 || !in_range {
                    s.sell[i] = true;
                    s.px[i] = c[i];
                    s.why[i] = if st_hit { "Close below the stock ST" } else if !in_range { "Outside the test range" } else { LAST };
                    if !ex.exit_close && ex.pb > 0.0 && pb_hit {
                        s.px[i] = if o[i] > pb { gap(i) } else { pb };
                        s.why[i] = "Profit booking";
                    }
                    if !ex.exit_close && ex.hsl > 0.0 && hsl_hit {
                        s.px[i] = if o[i] < hsl { gap(i) } else { hsl };
                        s.why[i] = "Hard SL";
                    }
                    if !ex.tsl_close && ex.tsl > 0.0 && tsl_hit {
                        // as written, the gap test reads the hard-SL level
                        s.px[i] = if o[i] < hsl { gap(i) } else { tsl };
                        s.why[i] = "Trailing SL";
                    }
                    if ex.exit_close && (pb_hit || hsl_hit) || ex.tsl_close && tsl_hit {
                        s.why[i] = if tsl_hit { "Trailing SL (close)" } else if hsl_hit { "Hard SL (close)" } else { "Profit booking (close)" };
                    }
                    pb = NULL;
                    hsl = NULL;
                    tsl = NULL;
                    pos = false;
                }
                if ex.tsl != 0.0 {
                    tsl = (c[i] * (1.0 - ex.tsl * 0.01)).max(tsl);
                }
            }
            if nifty_buy[i] && !pos && (!buy_only || c[i] > stv(i)) && in_range {
                s.buy[i] = true;
                pos = true;
                pb = if ex.pb > 0.0 { c[i] * (1.0 + ex.pb * 0.01) } else { NULL };
                hsl = if ex.hsl > 0.0 { c[i] * (1.0 - ex.hsl * 0.01) } else { NULL };
                tsl = if ex.tsl > 0.0 { c[i] * (1.0 - ex.tsl * 0.01) } else { NULL };
            }
        }
        // Buy = ExRem(Buy, Sell); Sell = ExRem(Sell, Buy)
        let buy = crate::books::ytsw::exrem(&s.buy, &s.sell);
        let sell = crate::books::ytsw::exrem(&s.sell, &buy);
        s.buy = buy;
        s.sell = sell;
        return Some(backtest(x, &s, from, to));
    }
    // Threshold / Final
    let (fk, ek) = (gi("flt_avg"), gi("entry_avg"));
    let Some(b) = bench(x, uses_open(fk) || uses_open(ek)) else { return Some(Vec::new()) };
    let fst = b.st(fk, gu("flt_period"), g("flt_factor"));
    let ftype = gi("flt_type");
    let filter: Vec<bool> = (0..n)
        .map(|i| match ftype {
            1 => fst[i].map_or(false, |v| b.c[i] < v),
            2 => true,
            _ => fst[i].map_or(false, |v| b.c[i] > v),
        })
        .collect();
    let est = b.st(ek, gu("entry_period"), g("entry_factor"));
    let entry = cross_up(&b.c, &est);
    let swing = swing_low_flags(&b, &est);
    let mut roc = vec![0.0; n];
    let mut iv = 0.0;
    for i in 0..n {
        if swing[i] {
            iv = l[i];
        }
        if entry[i] && iv != 0.0 {
            roc[i] = (c[i] / iv - 1.0) * 100.0;
        }
    }
    let xst = st_line(&avg_of(o, h, l, c, gi("exit_avg")), h, l, c, gu("exit_period"), gu("exit_period"), g("exit_factor"));
    let bear = |i: usize| xst[i].map_or(false, |v| c[i] < v);
    let (exit_st, buy_only) = (gi("exit_st") == 1, gi("buy_only") == 1);
    let thr = if id == THRESHOLD { Some((g("roc_thr"), gi("thr_below") == 1)) } else { None };
    let differs = |i: usize| (1..=4).any(|k| i + k >= n || c[i] != c[i + k]);
    let ohlc_ok = |i: usize| o[i] != h[i] && o[i] != c[i] && o[i] != l[i];
    let mut pos = false;
    let (mut pb, mut hsl, mut tsl) = (NULL, NULL, NULL);
    for i in 1..n {
        if differs(i) {
            if pos {
                let hsl_hit = ex.hsl > 0.0 && (if ex.exit_close { c[i] } else { l[i] }) <= hsl;
                let pb_hit = ex.pb > 0.0 && (if ex.exit_close { c[i] } else { h[i] }) >= pb;
                let tsl_hit = ex.tsl > 0.0 && (if ex.tsl_close { c[i] } else { l[i] }) <= tsl;
                let st_hit = exit_st && bear(i);
                if st_hit || hsl_hit || pb_hit || tsl_hit || i == n - 1 {
                    s.sell[i] = true;
                    s.px[i] = c[i];
                    s.why[i] = if st_hit { "Close below the stock ST" } else if hsl_hit { "Hard SL (close)" } else if pb_hit { "Profit booking (close)" } else if tsl_hit { "Trailing SL (close)" } else { LAST };
                    if !ex.exit_close && ex.pb > 0.0 && pb_hit {
                        s.px[i] = if o[i] > pb { gap(i) } else { pb };
                        s.why[i] = "Profit booking";
                    }
                    if !ex.exit_close && ex.hsl > 0.0 && hsl_hit {
                        s.px[i] = if o[i] < hsl { gap(i) } else { hsl };
                        s.why[i] = "Hard SL";
                    }
                    if !ex.tsl_close && ex.tsl > 0.0 && tsl_hit {
                        s.px[i] = if o[i] < tsl { gap(i) } else { tsl };
                        s.why[i] = "Trailing SL";
                    }
                    pb = NULL;
                    hsl = NULL;
                    tsl = NULL;
                    pos = false;
                }
                if ex.tsl > 0.0 && pos {
                    tsl = (c[i] * (1.0 - ex.tsl * 0.01)).max(tsl);
                }
            }
            let thr_ok = match thr {
                Some((t, true)) => roc[i] < t,
                Some((t, false)) => roc[i] > t,
                None => true,
            };
            if !pos && entry[i] && filter[i] && (!buy_only || !bear(i)) && thr_ok && ohlc_ok(i) {
                s.buy[i] = true;
                pos = true;
                pb = if ex.pb > 0.0 { c[i] * (1.0 + ex.pb * 0.01) } else { NULL };
                hsl = if ex.hsl > 0.0 { c[i] * (1.0 - ex.hsl * 0.01) } else { NULL };
                tsl = if ex.tsl > 0.0 { c[i] * (1.0 - ex.tsl * 0.01) } else { NULL };
            }
        } else if pos {
            s.sell[i] = true;
            s.px[i] = c[i];
            s.why[i] = "Dead stock (close unchanged for the next 4 bars)";
            pos = false;
        }
    }
    Some(backtest(x, &s, from, to))
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let d = catalogue().into_iter().find(|d| d.id == id)?;
    Some((d.id, d.name, "Rank From Lowest Low To ST Buy of Nifty and Buy Stocks"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::books::ytd7::tests::{bars, ps, wave};
    use crate::Row;

    /// NIFTY: 20 flat bars, a fall to a low of 80 on bar 27, then a rally that turns its ST up.
    fn nifty() -> Vec<(f64, f64, f64, f64)> {
        let mut v = Vec::new();
        for _ in 0..20 {
            v.push((100.0, 101.0, 99.0, 100.0));
        }
        for k in 0..8 {
            let c = 98.0 - 2.5 * k as f64;
            v.push((c + 1.0, c + 1.5, c - 1.0, c));
        }
        for k in 0..12 {
            let c = 82.0 + 3.0 * k as f64;
            v.push((c - 1.0, c + 1.0, c - 1.5, c));
        }
        v
    }
    fn with_bench(x: &mut Ctx, b: &[(f64, f64, f64, f64)]) {
        x.extra.insert("BENCH.Open".into(), b.iter().map(|v| v.0).collect());
        x.extra.insert("BENCH.High".into(), b.iter().map(|v| v.1).collect());
        x.extra.insert("BENCH.Low".into(), b.iter().map(|v| v.2).collect());
        x.extra.insert("BENCH.Close".into(), b.iter().map(|v| v.3).collect());
    }

    #[test]
    fn swing_low_and_rise_threshold() {
        let nb = nifty();
        let n = nb.len();
        let mut bx = Vec::new();
        for (k, v) in nb.iter().enumerate() {
            // the stock: low 50 on NIFTY's low day (bar 27), rallying to 60 (rise > 5%)
            let c = if k == 27 { 51.0 } else if k > 27 { 52.0 + k as f64 * 0.3 } else { 60.0 };
            let low = if k == 27 { 50.0 } else { c - 1.0 };
            bx.push((c - 0.5, c + 1.0, low, c));
            let _ = v;
        }
        let r = bars(&bx);
        let mut x = Ctx::new(&r, &r, &[]);
        with_bench(&mut x, &nb);
        let b = bench(&x, false).unwrap();
        let est = b.st(5, 5, 1.0);
        let sw = swing_low_flags(&b, &est);
        assert!(sw[27], "NIFTY's lowest-low bar is flagged");
        let entry = cross_up(&b.c, &est);
        let e = entry.iter().position(|&v| v).expect("NIFTY ST turns up");
        assert!(e > 27);
        let t = run(THRESHOLD, &x, 0, n - 1, &ps(&[])).unwrap();
        assert_eq!(t.len(), 1);
        assert_eq!(t[0].legs[0].i, e);
        assert!(t[0].open, "no exit but the end of the data");
        // a threshold above the rise blocks the entry; Final has none
        let rise = (x.c[e] / 50.0 - 1.0) * 100.0;
        assert!(run(THRESHOLD, &x, 0, n - 1, &ps(&[("roc_thr", rise + 1.0)])).unwrap().is_empty());
        assert_eq!(run(FINAL, &x, 0, n - 1, &ps(&[("flt_type", 2.0)])).unwrap().len(), 1);
        // a 1% hard stop exits at the stop level on a later low
        let t = run(FINAL, &x, 0, n - 1, &ps(&[("hsl_pct", 1.0)])).unwrap();
        assert!(t.iter().all(|t| t.open || t.exit_px >= x.l[t.exit_i]));
    }

    #[test]
    fn dead_stock_rule_reads_ahead() {
        let mut nb = nifty();
        nb.extend((0..10).map(|k| { let c = 118.0 + k as f64; (c - 1.0, c + 1.0, c - 1.5, c) }));
        let n = nb.len();
        let bx: Vec<(f64, f64, f64, f64)> = (0..n).map(|k| if k >= 36 { (49.0, 51.0, 48.0, 50.0) } else { let c = 50.0 + (k % 3) as f64; (c - 0.5, c + 1.0, c - 1.0, c) }).collect();
        let r = bars(&bx);
        let mut x = Ctx::new(&r, &r, &[]);
        with_bench(&mut x, &nb);
        let t = run(FINAL, &x, 0, n - 1, &ps(&[])).unwrap();
        assert_eq!(t.len(), 1);
        assert!(!t[0].open && t[0].reason.starts_with("Dead stock"), "{:?}", t[0].reason);
        assert_eq!(t[0].exit_i, 36);
    }

    #[test]
    fn missing_bench_gives_no_trades() {
        let r = wave(120);
        let x = Ctx::new(&r, &r, &[]);
        for id in IDS {
            assert!(run(id, &x, 0, r.len() - 1, &ps(&[])).unwrap().is_empty());
        }
    }

    #[test]
    fn every_strategy_runs() {
        let r: Vec<Row> = wave(400);
        for bench in [false, true] {
            let mut x = Ctx::new(&r, &r, &[]);
            if bench {
                let b: Vec<(f64, f64, f64, f64)> = r.iter().enumerate().map(|(k, b)| {
                    let c = 200.0 + 30.0 * ((k as f64) / 25.0).sin();
                    let _ = b;
                    (c - 1.0, c + 2.0, c - 2.0, c)
                }).collect();
                with_bench(&mut x, &b);
                // a gap in NIFTY data is carried forward
                x.extra.get_mut("BENCH.Close").unwrap()[100] = f64::NAN;
            }
            for id in IDS {
                for kv in [vec![], vec![("exit_st", 1.0), ("hsl_pct", 2.0), ("tsl_pct", 3.0), ("pb_pct", 5.0), ("roc_thr", -100.0)]] {
                    let t = run(id, &x, 30, r.len() - 1, &ps(&kv)).unwrap();
                    assert!(t.iter().all(|t| t.exit_i >= t.legs[0].i && t.legs[0].i >= 30), "{id}");
                    for w in t.windows(2) {
                        assert!(w[1].legs[0].i >= w[0].exit_i, "{id}: overlapping trades");
                    }
                    if !bench {
                        assert!(t.is_empty());
                    }
                }
            }
        }
        assert_eq!(catalogue().len(), IDS.len());
        for id in IDS {
            assert!(params(id).is_some() && family_of(id).is_some());
        }
    }
}
