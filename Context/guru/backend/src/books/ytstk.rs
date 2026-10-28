//! YT Strategies · 52 Week High and ROC (stocks): the team's AmiBroker stock AFLs in "Old Topics/52 Week High" and
//! "Old Topics/ROC" that are not in `books::ytwh` / `books::ytroc`, each run here on one stock's own daily bars.
//! Coded from the AFLs as written, with the AFLs' own default settings; there is no book PDF.
//!
//! Both AFLs write Buy / Sell (Short / Cover) in a bar loop with their own exit prices; they are backtested as
//! `books::ytroc` does it: the loop runs over the whole history, trades open only in the test window, the loop's
//! forced exit on the data's last bar (dead-stock rule) counts as the position still being open, and prices the loop
//! assigns are kept inside the bar's range (AmiBroker price bound checking). Further points:
//! - `HHV(C, 260)` near the start uses the bars available;
//! - inside a loop an array element that is Null is -1e10 (so `MA_7 / MA_65 <= 0.95` holds while the 65-bar average
//!   does not exist yet, and a Null ATR makes the ATR trailing stop sit far below the close);
//! - the ROC short AFL ranks the watch list by ROC (StaticVarGenerateRanks, PositionScore = rank); that only picks
//!   among simultaneous short signals and is not modelled;
//! - turnover filters (V x C) are close x volume here.
#![allow(dead_code)]
use crate::books::ytroc::{atr_wilder, backtest, Px, LAST_BAR};
use crate::books::ytsw::{close_swing, sma};
use crate::books::ytsws::turnover_ok;
use crate::books::ytwh::{bound, hhv, loop_val};
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "yt_stk";
pub const WH_SWING_PB: &str = "ytstk_52wh_swing_profit_booking";
pub const ROC_SHORT: &str = "ytstk_roc_trend_intensity_short";
pub const IDS: [&str; 2] = [WH_SWING_PB, ROC_SHORT];

const GROUP: &str = "YT Strategies · 52 Week High and ROC (stocks)";

fn def(id: &'static str, name: &'static str, dir: &'static str, rules: &'static [&'static str], notes: &'static str) -> StrategyDef {
    StrategyDef { id, name, group: GROUP, session: 0, dir, status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim: None }
}

pub fn catalogue() -> Vec<StrategyDef> {
    vec![
        def(WH_SWING_PB, "52-Week High with % Swing Exit and Profit Booking (stock)", "Long", &[
            "F|10% swing on closes (bands at close x 1.10 and x 0.90, ratcheting, reset on each flip); re-arm flag set when the swing flips from down to up, cleared on a buy, and cleared while the swing is down unless already set",
            "E|Buy at the close when flat, the re-armed flag is on and the close is above the previous bar's highest close of the last 260 bars; never on the data's last bar",
            "X|Sell when the swing is down (at the close), or when the high exceeds the profit level (entry + 25%; at that level, or at the open less 0.25% when the open is above it)",
        ], "52-WEEEK HIGH + % SWING AND PROFIT BOOKING.afl (52 Week High / Optimization on 52 Week high strategy), coded as the AFL is written with its Optimize defaults (exit swing 10%, profit 25%). Runs on one stock at a time; the team ran it over a cash-stock universe (workbook 'Cash Stock 20 open', 20 positions); ranking and portfolio limits (MaxOpenPositions 20, sizing) are not modelled. The penny, turnover (V x C, close x volume here) and circuit filters are ParamList options defaulting to OFF; the market-filter block is commented out in the AFL. Like ytwh_st_profit_booking, the re-arm flag means a new buy needs a swing flip up after the previous buy.")
        ,
        def(ROC_SHORT, "Trend Intensity Short with Hard, % and ATR Trailing Stops (stock)", "Short", &[
            "F|Trend intensity = MA(C, 7) / MA(C, 65); up move = C / previous C",
            "E|Short at the close when flat, trend intensity <= 0.95 and up move <= 0.96, the bar does not close at its low, and the stock is not frozen (same OHLC for the next 30 bars); not on the first bar",
            "X|Cover when the high reaches the hard SL (entry + 2%; at it, or at the open plus 0.25% if the open is at or above it), the close reaches the % trailing SL (lowest close since entry + 5%, as of the previous bar), or the close reaches the ATR trailing SL (lowest close + 5 x ATR(5) taken on the bar of that low, only falling; previous bar's value); not on a bar closing at its high, except a frozen stock",
        ], "Short Entry based on ROC stocks Ranking and Trend Intensity and Exit with Differnt SL's (Without Filter)..afl (ROC), coded as the AFL is written with its Optimize defaults (MA 7 / 65, trend intensity 0.95, up move 0.96, % trailing SL 5). Runs on one stock at a time; the team ran it over a stock watch list; the AFL's ROC(30) cross-sectional rank only sets PositionScore, so ranking and portfolio limits (MaxOpenPositions 20, sizing) are not modelled. The ATR multiplier's default 5 lies above its own Param maximum 2; the default is used as written. The frozen-stock test (deadstock1) reads the next 30 bars (look-ahead as written; never true in the data's last 30 bars). While MA(C, 65) does not exist yet, the loop reads the Null trend intensity as -1e10, so the first 64 bars need only the up-move condition. ATR is AmiBroker's Wilder ATR(5); while it is Null the ATR stop is far below the close and covers on the next bar not closing at its high. Penny price 0 and turnover (V x C, default off) as in the AFL; dead-stock default NO."),
    ]
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    Some(match id {
        WH_SWING_PB => vec![
            p("exit_pct", "Exit swing %", 10.0, 5.0, 25.0, 1.0),
            p("profit", "Profit booking % (0 off)", 25.0, 0.0, 40.0, 2.0),
            p("penny", "Penny filter (0 off, 1 on)", 0.0, 0.0, 1.0, 1.0),
            p("penny_price", "Penny price (buy only above it when on)", 1.0, 1.0, 100.0, 0.5),
            p("turnover", "Turnover filter (0 off, 1 on: close x volume above the AFL's yearly threshold)", 0.0, 0.0, 1.0, 1.0),
            p("circuit", "Circuit filter (0 off, 1 on)", 0.0, 0.0, 1.0, 1.0),
        ],
        ROC_SHORT => vec![
            p("hard_sl", "Hard SL % (0 off)", 2.0, 0.0, 5.0, 1.0),
            p("trail_pct", "% trailing SL (0 off)", 5.0, 0.0, 20.0, 5.0),
            p("atr_period", "ATR period", 5.0, 1.0, 100.0, 1.0),
            p("atr_mult", "ATR trailing SL multiplier (0 off; AFL default 5, above its Param maximum)", 5.0, 0.0, 10.0, 1.0),
            p("buffer_pct", "Buffer % (gap fills)", 0.25, 0.0, 10.0, 0.05),
            p("ma_fast", "Fast MA", 7.0, 4.0, 10.0, 1.0),
            p("ma_slow", "Slow MA", 65.0, 45.0, 80.0, 5.0),
            p("ti", "Trend intensity at most", 0.95, 0.76, 0.96, 0.02),
            p("up_move", "Up move (C / previous C) at most", 0.96, 0.76, 0.96, 0.02),
            p("penny", "Penny stock price (short only above it)", 0.0, 0.0, 100.0, 1.0),
            p("turnover", "Turnover filter (0 off, 1 on: close x volume above the AFL's yearly threshold)", 0.0, 0.0, 1.0, 1.0),
        ],
        _ => return None,
    })
}

fn wh_swing_pb(x: &Ctx, g: &dyn Fn(&str) -> f64) -> Px {
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let n = c.len();
    let gi = |k: &str| g(k).round() as i64;
    let hwc = hhv(c, 260);
    let t1 = close_swing(c, g("exit_pct"));
    let profit = g("profit");
    let turn = turnover_ok(x, gi("turnover") == 1, true);
    let circuit = gi("circuit") == 1;
    let penny_ok = |i: usize| if gi("penny") == 1 { c[i] > g("penny_price") } else { c[i] > 0.0 };
    let c0 = |i: usize| circuit && i > 0 && o[i] == c[i] && o[i] == h[i] && o[i] == l[i] && c[i] > c[i - 1];
    let c1 = |i: usize| circuit && i > 0 && o[i] == c[i] && o[i] == h[i] && o[i] == l[i] && c[i] < c[i - 1];
    let c2 = |i: usize| circuit && c[i] == h[i];
    let c3 = |i: usize| circuit && l[i] == c[i];
    let mut s = Px::new(n);
    let mut trendj = vec![0u8; n];
    let mut pb = vec![0.0f64; n];
    let mut pos = vec![false; n];
    for i in 1..n {
        trendj[i] = if t1[i - 1] == -1 && t1[i] > 0 { 1 } else if trendj[i - 1] == 1 { 1 } else { 0 };
        let last = i + 1 == n;
        if !pos[i - 1] && c[i] > hwc[i - 1] && turn[i] && penny_ok(i) && !c0(i) && !c2(i) && !last && trendj[i] == 1 {
            s.buy[i] = true;
            pb[i] = if profit > 0.0 { c[i] + c[i] * profit / 100.0 } else { 99_999_999.0 };
            pos[i] = true;
            trendj[i] = 0;
        } else if pos[i - 1] && (t1[i] == -1 || h[i] > pb[i - 1] || last) && !c1(i) && !c3(i) {
            let (px, why) = if o[i] > pb[i - 1] {
                (o[i] - o[i] * 0.0025, "Profit booking (gap)")
            } else if h[i] > pb[i - 1] {
                (pb[i - 1], "Profit booking")
            } else if t1[i] == -1 {
                (c[i], "Swing down")
            } else {
                (c[i], LAST_BAR)
            };
            s.sell(i, bound(x, i, px), why);
            pos[i] = false;
        } else {
            pos[i] = pos[i - 1];
            pb[i] = pb[i - 1];
        }
    }
    s
}

fn roc_short(x: &Ctx, g: &dyn Fn(&str) -> f64) -> Px {
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let n = c.len();
    let gu = |k: &str| g(k).round().max(1.0) as usize;
    let maf = sma(c, gu("ma_fast"));
    let mas = sma(c, gu("ma_slow"));
    // MA_7 / MA_65 is Null while either is Null; the loop reads that as -1e10
    let ti: Vec<f64> = (0..n).map(|i| match (maf[i], mas[i]) { (Some(a), Some(b)) => a / b, _ => -1e10 }).collect();
    let atr = atr_wilder(x, gu("atr_period"));
    // deadstock1: the same O, H, L, C on each of the next 30 bars
    let frozen: Vec<bool> = (0..n).map(|i| i + 30 < n && (1..=30).all(|k| o[i] == o[i + k] && h[i] == h[i + k] && l[i] == l[i + k] && c[i] == c[i + k])).collect();
    let turn = turnover_ok(x, g("turnover").round() as i64 == 1, true);
    let penny = g("penny");
    let filter: Vec<bool> = (0..n).map(|i| l[i] != c[i] && c[i] > penny && turn[i] && i != 0 && !frozen[i]).collect();
    let (hsl_pct, trail_pct, mult, buf) = (g("hard_sl"), g("trail_pct"), g("atr_mult"), g("buffer_pct"));
    let (ti_v, um_v) = (g("ti"), g("up_move"));
    const BIG: f64 = 9_999_999.0;
    let mut s = Px::new(n);
    let mut position = false;
    let (mut hsl, mut ll, mut atr_value, mut lc_af, mut trail) = (BIG, BIG, BIG, BIG, BIG);
    let mut atr_sl = vec![BIG; n];
    for i in 1..n {
        let up_move = c[i] / c[i - 1];
        if !position && ti[i] <= ti_v && up_move <= um_v && filter[i] {
            s.short[i] = true;
            if hsl_pct > 0.0 {
                hsl = (1.0 + hsl_pct * 0.01) * c[i];
            }
            if trail_pct > 0.0 {
                lc_af = c[i];
                trail = (1.0 + trail_pct * 0.01) * lc_af;
            }
            if mult > 0.0 {
                ll = c[i];
                atr_sl[i] = ll + loop_val(atr[i]) * mult;
            }
            position = true;
        } else if position {
            if trail_pct > 0.0 {
                lc_af = c[i].min(lc_af);
            }
            if mult > 0.0 {
                ll = c[i].min(ll);
                if c[i] == ll {
                    atr_value = loop_val(atr[i]);
                }
            }
            let uc = h[i] == c[i];
            if ((h[i] >= hsl || c[i] >= trail || c[i] >= atr_sl[i - 1]) && !uc) || frozen[i] {
                let (px, why) = if h[i] >= hsl {
                    (if o[i] >= hsl { o[i] + o[i] * buf / 100.0 } else { hsl }, "Hard SL")
                } else if c[i] >= trail {
                    (c[i], "% trailing SL")
                } else if c[i] >= atr_sl[i - 1] {
                    (c[i], "ATR trailing SL")
                } else {
                    (c[i], "Frozen stock")
                };
                s.cover(i, bound(x, i, px), why);
                position = false;
                hsl = BIG;
                lc_af = BIG;
                trail = BIG;
                ll = BIG;
                atr_value = BIG;
                atr_sl[i] = BIG;
            }
            if trail_pct > 0.0 {
                trail = (1.0 + trail_pct * 0.01) * lc_af;
            }
            if mult > 0.0 {
                atr_sl[i] = if atr_value > 0.0 { ll + atr_value * mult } else { atr_sl[i - 1] };
                if atr_sl[i] > atr_sl[i - 1] {
                    atr_sl[i] = atr_sl[i - 1];
                }
            }
        }
    }
    s
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(0.0, |d| d.def));
    let n = x.c.len();
    if n == 0 || from > to || to >= n {
        return Some(Vec::new());
    }
    let s = match id {
        WH_SWING_PB => wh_swing_pb(x, &g),
        ROC_SHORT => roc_short(x, &g),
        _ => return None,
    };
    Some(backtest(x, &s, from, to))
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let d = catalogue().into_iter().find(|d| d.id == id)?;
    Some((d.id, d.name, if id == ROC_SHORT { "ROC" } else { "52 Week High" }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::books::ytwh::testbars::{bars, ps, wave};

    fn flat(c: f64) -> (f64, f64, f64, f64) {
        (c, c + 0.5, c - 0.5, c)
    }

    #[test]
    fn wh_needs_rearm_then_breakout_and_books_profit() {
        // fall 15% (swing down), recover above the old high (swing up = re-arm, new high = buy), then +30%
        let mut v: Vec<(f64, f64, f64, f64)> = vec![flat(100.0), flat(100.0), flat(84.0), flat(84.0), flat(95.0), flat(101.0), flat(102.0)];
        v.push((103.0, 140.0, 102.0, 104.0));
        v.push(flat(104.0));
        let r = bars(&v);
        let x = Ctx::new(&r, &r, &[]);
        let t = run(WH_SWING_PB, &x, 0, r.len() - 1, &Params::new()).unwrap();
        // swing up flips at 95 (> 84 x 1.1 = 92.4): re-armed at bar 4 but 95 is no new high; bar 5 (101 > 100) buys
        assert_eq!(t[0].legs[0].i, 5);
        assert_eq!(t[0].exit_i, 7);
        assert!((t[0].exit_px - 101.0 * 1.25).abs() < 1e-9);
        // no new buy without another flip up, even on a new high
        assert_eq!(t.len(), 1);
    }

    #[test]
    fn roc_short_enters_on_a_drop_and_covers_at_hard_sl() {
        let mut v: Vec<(f64, f64, f64, f64)> = (0..80).map(|k| flat(100.0 - k as f64 * 0.3)).collect();
        // a 5% drop on bar 80 (trend intensity < 0.95 by then? use the early-bars Null rule instead: bar 10)
        v[10] = (v[9].3, v[9].3 + 0.2, v[9].3 * 0.94, v[9].3 * 0.945);
        let e = v[10].3;
        v[11] = (e, e * 1.03, e - 0.2, e + 0.1);
        let r = bars(&v);
        let x = Ctx::new(&r, &r, &[]);
        let t = run(ROC_SHORT, &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert_eq!(t[0].dir, -1.0);
        assert_eq!(t[0].legs[0].i, 10);
        assert_eq!(t[0].exit_i, 11);
        assert!((t[0].exit_px - e * 1.02).abs() < 1e-9);
    }

    #[test]
    fn every_strategy_runs() {
        let r = wave(420);
        let mut x = Ctx::new(&r, &r, &[]);
        for pass in 0..2 {
            if pass == 1 {
                for k in ["Open", "High", "Low", "Close"] {
                    x.extra.insert(format!("BENCH.{k}"), x.c.clone());
                }
            }
            for id in IDS {
                assert!(params(id).is_some() && family_of(id).is_some(), "{id}");
                for kv in [vec![], vec![("profit", 0.0), ("circuit", 1.0)], vec![("atr_mult", 0.0), ("trail_pct", 0.0)], vec![("turnover", 1.0)]] {
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
}
