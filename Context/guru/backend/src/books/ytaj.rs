//! YT Strategies · AJ Short Strategy: the team's AmiBroker AFL in "Old Topics/AJ Short Strategy" (Aakash-Jimit RSI +
//! Bollinger band bear-market short), run on one series at a time. Coded from the AFL as written, with the AFL's own
//! default settings; there is no book PDF.
//!
//! The AFL's header says it was built for 75-minute candles; its formula reads no interval of its own (only the
//! optional market filter sets a weekly / monthly / daily time frame), and the engine has daily bars only, so it runs
//! here on daily bars: a declared adaptation of the interval, not the team's 75-minute run.
//!
//! AmiBroker behaviour followed:
//! - `SetBarsRequired(100000, 0)` loads the whole history; the loop runs from bar RSI period (42) of the data and the
//!   backtester opens trades only in the test window;
//! - RSI and the Bollinger band are on (H + L + C) / 3 (RSIa / BBandBot, population SD); array comparisons with Null
//!   are false;
//! - `ShortPrice = Low - 0.05` (or Open less the 0.25% buffer when the Open is below that, which cannot happen) lies
//!   below the bar, so AmiBroker's price bound check fills the short at the bar's Low: the signal uses the bar's Close
//!   and fills at its Low (look-ahead as written). The hard stop is computed from the unbounded Low - 0.05;
//! - the "Entry within next N candles" Param and its counter are never advanced in the loop (count stays 0), so the
//!   entry is always on the signal bar itself; the Param is not offered;
//! - a cover on the entry bar (only possible with the hard stop at the close or the "Lowest Low Including Today"
//!   trailing point) is a Short and a Cover on the same bar; AmiBroker's backtester exits first and enters second, so
//!   that cover is lost and the short stays open until the loop's next Cover; this is kept;
//! - the market filter (default None) is the team's Supertrend loop on the traded series itself (no SetForeign),
//!   default weekly ST(15, 3) on the mid point, expanded with expandLast; the dead-stock toggle (default NO) has no
//!   effect on this AFL's entries and is not offered.
#![allow(dead_code)]
use crate::books::ytds::{backtest, expand_last_null, LoopSig};
use crate::books::ytroc::{atr_wilder, rsi_opt};
use crate::books::ytsw::{compress, supertrend};
use crate::books::ytwh::loop_val;
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "yt_aj";
pub const RSI_BB_SHORT: &str = "ytaj_rsi_bb_bear_short";
pub const IDS: [&str; 1] = [RSI_BB_SHORT];

const GROUP: &str = "YT Strategies · AJ Short Strategy";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef {
        id: RSI_BB_SHORT,
        name: "Aakash-Jimit RSI + Bollinger Band Bear-Market Short",
        group: GROUP,
        session: 0,
        dir: "Short",
        status: Status::Live,
        needs_volume: false,
        needs_vix: false,
        needs_series: &[],
        book: BOOK,
        rules: &[
            "F|Typical price TP = (High + Low + Close) / 3",
            "F|RSI(TP, 42) crosses below 40: below 40 now, above 40 on the previous bar",
            "F|Close < lower Bollinger band of TP (16 bars, 1.5 SD)",
            "F|Upper wick (High − Open) at most 10% of the bar's range (options: off, or Open = High)",
            "F|Flat",
            "F|Optional filters (default off): ST market filter down, price range 25-500, Volume ≥ minimum",
            "E|Short at the bar's Low − 0.05, kept inside the bar: at its Low",
            "X|Trailing stop: lowest Close since entry (previous bars) + 3%; cover at the Close when the Close reaches it",
            "X|Options: ATR trailing stop (lowest point + 1 × ATR(15), only falling), the Low or Low-including-today as the point, a live-market fill",
            "X|Optional hard stop (default off): entry + 0.5%, at the Close or live",
            "X|Live fills: at the stop, or at Open + 0.25% when the Open is above it",
        ],
        notes: "Aakash_Jimit_(RSI+BB)_BearMarket _ShortOnly _NextDay_All_New.afl (AJ Short Strategy), coded as the AFL is written. The AFL header describes 75-minute candles; it runs here on daily bars (the engine has no 75-minute data), a declared adaptation. The short fills at the signal bar's Low (Low − 0.05 bounded by AmiBroker to the bar) while the signal needs that bar's Close: look-ahead as written. The 'within next N candles' entry window is never used by the loop (its counter is never advanced). The turnover filter compares the 'lakh' Param (1000) with raw Volume, as written. Runs on one series at a time; the team's sizing (10% of equity, 10 positions) is not modelled.",
        claim: None,
    }]
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    if id != RSI_BB_SHORT {
        return None;
    }
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    Some(vec![
        p("rsi_period", "RSI period (on (H+L+C)/3)", 42.0, 8.0, 50.0, 4.0),
        p("rsi_level", "RSI short benchmark", 40.0, 25.0, 75.0, 5.0),
        p("bb_period", "Bollinger band period", 16.0, 4.0, 28.0, 4.0),
        p("bb_sd", "Bollinger band standard deviations", 1.5, 0.5, 2.0, 0.25),
        p("wick_mode", "Base candle condition (0 none, 1 Open = High, 2 maximum upper wick)", 2.0, 0.0, 2.0, 1.0),
        p("wick_pct", "Maximum upper wick % of the candle", 10.0, 0.0, 100.0, 1.0),
        p("entry_buffer", "Entry buffer below the Low", 0.05, 0.0, 1.0, 0.01),
        p("open_buffer", "Open buffer % (gap fills)", 0.25, 0.0, 20.0, 0.001),
        p("trail_type", "Trailing stop type (0 percentage, 1 ATR)", 0.0, 0.0, 1.0, 1.0),
        p("trail_point", "Trailing stop point (0 previous lowest Close, 1 previous lowest Low, 2 lowest Low including today)", 0.0, 0.0, 2.0, 1.0),
        p("trail_exit", "Trailing stop exit at (0 Close, 1 live market)", 0.0, 0.0, 1.0, 1.0),
        p("trail_pct", "% trailing stop", 3.0, 0.1, 100.0, 0.5),
        p("atr_period", "ATR stop period", 15.0, 1.0, 30.0, 1.0),
        p("atr_mult", "ATR stop multiplier", 1.0, 0.0, 10.0, 0.5),
        p("hard_sl", "Hard stop (0 off, 1 on)", 0.0, 0.0, 1.0, 1.0),
        p("hard_sl_pct", "% hard stop", 0.5, 0.1, 100.0, 0.1),
        p("hard_sl_exit", "Hard stop exit at (0 Close, 1 live market)", 0.0, 0.0, 1.0, 1.0),
        p("market_filter", "Market filter (0 none, 1 ST down on the series itself)", 0.0, 0.0, 1.0, 1.0),
        p("st_tf", "Market filter time frame (0 monthly, 1 weekly, 2 daily)", 1.0, 0.0, 2.0, 1.0),
        p("st_period", "ST filter period", 15.0, 2.0, 50.0, 2.0),
        p("st_factor", "ST filter factor", 3.0, 1.0, 6.0, 0.5),
        p("st_point", "ST calculation point (0 mid, 1 close, 2 pivot, 3 mid/pivot/close average, 4 MA(C, 3), 5 MA(C, 5))", 0.0, 0.0, 5.0, 1.0),
        p("turnover", "Turnover filter (0 off, 1 on: Volume ≥ minimum)", 0.0, 0.0, 1.0, 1.0),
        p("min_turnover", "Minimum turnover (compared with Volume, as written)", 1000.0, 1.0, 1_000_000.0, 1.0),
        p("price_range", "Price range filter (0 off, 1 on)", 0.0, 0.0, 1.0, 1.0),
        p("min_price", "Minimum price", 25.0, 1.0, 1_000_000.0, 1.0),
        p("max_price", "Maximum price", 500.0, 1.0, 1_000_000.0, 1.0),
    ])
}

/// BBandBot(v, n, k): MA(v, n) − k × population SD of the last n values; None until n bars.
fn bband_bot(v: &[f64], n: usize, k: f64) -> Vec<Option<f64>> {
    let n = n.max(1);
    (0..v.len())
        .map(|i| {
            if i + 1 < n {
                return None;
            }
            let w = &v[i + 1 - n..=i];
            let m = w.iter().sum::<f64>() / n as f64;
            let sd = (w.iter().map(|x| (x - m).powi(2)).sum::<f64>() / n as f64).sqrt();
            Some(m - k * sd)
        })
        .collect()
}

/// The AFL's ST calculation point on the filter bars; MA(C, 3/5) before it exists is the loop's Null (-1e10).
fn st_calc(h: &[f64], l: &[f64], c: &[f64], kind: i64) -> Vec<f64> {
    let ma = |n: usize| crate::books::ytsw::sma(c, n).into_iter().map(loop_val).collect::<Vec<f64>>();
    match kind {
        1 => c.to_vec(),
        2 => (0..c.len()).map(|i| (h[i] + l[i] + c[i]) / 3.0).collect(),
        3 => (0..c.len()).map(|i| ((h[i] + l[i]) / 2.0 + (h[i] + l[i] + c[i]) / 3.0 + c[i]) / 3.0).collect(),
        4 => ma(3),
        5 => ma(5),
        _ => (0..c.len()).map(|i| (h[i] + l[i]) / 2.0).collect(),
    }
}

fn aj_loop(x: &Ctx, g: &dyn Fn(&str) -> f64) -> LoopSig {
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let n = c.len();
    let gi = |k: &str| g(k).round() as i64;
    let gu = |k: &str| g(k).round().max(1.0) as usize;
    let tp: Vec<f64> = (0..n).map(|i| (h[i] + l[i] + c[i]) / 3.0).collect();
    let rsi = rsi_opt(&tp, gu("rsi_period"));
    let bbl = bband_bot(&tp, gu("bb_period"), g("bb_sd"));
    let atr: Vec<f64> = atr_wilder(x, gu("atr_period")).into_iter().map(loop_val).collect();
    let lvl = g("rsi_level");
    let rsi_breach = |i: usize| i > 0 && matches!((rsi[i], rsi[i - 1]), (Some(a), Some(b)) if a < lvl && b > lvl);
    let below_band = |i: usize| bbl[i].map_or(false, |b| b > c[i]);
    let wick = g("wick_pct");
    let wick_ok = |i: usize| match gi("wick_mode") {
        1 => h[i] == o[i],
        2 => h[i] - o[i] <= (h[i] - l[i]) * (wick / 100.0),
        _ => true,
    };
    let market: Vec<bool> = if gi("market_filter") == 1 {
        let tf = match gi("st_tf") {
            0 => 'M',
            2 => 'D',
            _ => 'W',
        };
        let b = compress(x, tf);
        let calc = st_calc(&b.h, &b.l, &b.c, gi("st_point"));
        let (tr, _) = supertrend(&calc, &b.h, &b.l, &b.c, gu("st_period"), g("st_factor"), g("st_factor"));
        expand_last_null(&tr, x, tf).into_iter().map(|t| t == -1).collect()
    } else {
        vec![true; n]
    };
    let vol_ok = |i: usize| gi("turnover") != 1 || x.r[i].volume.map_or(false, |v| g("min_turnover") <= v);
    let price_ok = |i: usize| gi("price_range") != 1 || (g("min_price") <= c[i] && g("max_price") >= c[i]);

    let point = gi("trail_point");
    let look: &[f64] = if point == 0 { c } else { l };
    let atr_type = gi("trail_type") == 1;
    let (tsl_pct, mult) = (g("trail_pct"), g("atr_mult"));
    let trail_of = |min: f64, a: f64| if atr_type { min + a * mult } else { min * (1.0 + tsl_pct / 100.0) };
    let trail_live = gi("trail_exit") == 1;
    let trail_watch: &[f64] = if trail_live { h } else { c };
    let hard_on = gi("hard_sl") == 1;
    let hard_live = gi("hard_sl_exit") == 1;
    let hard_watch: &[f64] = if hard_live { h } else { c };
    let buf = g("open_buffer") / 100.0;
    let eb = g("entry_buffer");
    let hard_hit = |hsl: f64, watch: f64| hard_on && hsl <= watch;
    let same_day_ll = |trail: f64, i: usize| point == 2 && trail <= c[i];
    let gap = |i: usize, lvl: f64| if o[i] > lvl { o[i] + o[i] * buf } else { lvl };
    // getCoverPrice
    let cover_px = |i: usize, trail: f64, hsl: f64, entry_bar: bool| -> (f64, &'static str) {
        if hard_on && hard_live && h[i] >= hsl && !entry_bar {
            if trail_live && trail <= hsl { (gap(i, trail), "Trailing stop (live)") } else { (gap(i, hsl), "Hard stop (live)") }
        } else if trail_live && h[i] >= trail && !entry_bar {
            if hard_on && hard_live && trail >= hsl { (gap(i, hsl), "Hard stop (live)") } else { (gap(i, trail), "Trailing stop (live)") }
        } else {
            (c[i], "Stop at the Close")
        }
    };

    let mut s = LoopSig::new(-1.0, n);
    let mut position = false;
    let (mut min_price, mut trail, mut hsl) = (0.0f64, 0.0f64, 0.0f64);
    for i in gu("rsi_period")..n {
        if !position && market[i] && rsi_breach(i) && below_band(i) && wick_ok(i) && price_ok(i) && vol_ok(i) {
            let short_px = if o[i] < l[i] - eb { o[i] - o[i] * buf } else { l[i] - eb };
            s.entry[i] = Some(short_px);
            min_price = look[i];
            trail = trail_of(min_price, atr[i]);
            hsl = short_px * (1.0 + g("hard_sl_pct") / 100.0);
            position = true;
            if hard_hit(hsl, c[i]) || same_day_ll(trail, i) {
                s.exit[i] = Some(cover_px(i, trail, hsl, true));
                position = false;
            }
        } else if position {
            if min_price > look[i] && point == 2 {
                min_price = look[i];
                trail = trail_of(min_price, atr[i]);
            }
            if trail <= trail_watch[i] || hard_hit(hsl, hard_watch[i]) || same_day_ll(trail, i) {
                s.exit[i] = Some(cover_px(i, trail, hsl, false));
                position = false;
            } else if min_price > look[i] && !atr_type {
                min_price = look[i];
                trail = trail_of(min_price, atr[i]);
            } else if atr_type {
                min_price = look[i];
                let new = trail_of(min_price, atr[i]);
                if new < trail {
                    trail = new;
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
    Some(backtest(x, &aj_loop(x, &g), from, to))
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let d = catalogue().into_iter().find(|d| d.id == id)?;
    Some((d.id, d.name, "AJ Short Strategy"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::books::ytwh::testbars::{bars, ps, smoke, wave};

    /// 60 gently rising bars (RSI high), then a sharp drop bar with a small upper wick.
    fn setup() -> Vec<(f64, f64, f64, f64)> {
        let mut v: Vec<(f64, f64, f64, f64)> = (0..60)
            .map(|k| {
                let c = 100.0 + k as f64 * 0.2 + if k % 2 == 0 { 0.3 } else { -0.3 };
                (c, c + 0.5, c - 0.5, c)
            })
            .collect();
        // bar 60: opens at its high, falls hard: TP far below the band, RSI crosses below 40
        v.push((105.0, 105.0, 80.0, 80.5));
        v
    }

    #[test]
    fn shorts_at_the_low_and_trails_lowest_close_by_3pct() {
        let mut v = setup();
        v.push((80.0, 81.0, 78.0, 78.5)); // new lowest close 78.5 -> trail 80.855
        v.push((79.0, 81.5, 78.8, 80.8)); // close 80.8 < 80.855: holds
        v.push((81.0, 82.0, 80.5, 81.5)); // close 81.5 >= 80.855: cover at the close
        let r = bars(&v);
        let x = Ctx::new(&r, &r, &[]);
        let t = run(RSI_BB_SHORT, &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert_eq!(t.len(), 1);
        assert_eq!((t[0].dir, t[0].legs[0].i, t[0].exit_i), (-1.0, 60, 63));
        // Low - 0.05 is below the bar: filled at the Low
        assert!((t[0].legs[0].px - 80.0).abs() < 1e-9);
        assert!((t[0].exit_px - 81.5).abs() < 1e-9);
    }

    #[test]
    fn upper_wick_filter_blocks_a_long_wick() {
        let mut v = setup();
        v[60] = (95.0, 105.0, 80.0, 80.5); // wick 10 of range 25 > 10%
        v.push((80.0, 81.0, 78.0, 78.5));
        let r = bars(&v);
        let x = Ctx::new(&r, &r, &[]);
        assert!(run(RSI_BB_SHORT, &x, 0, r.len() - 1, &Params::new()).unwrap().iter().all(|t| t.legs[0].i != 60));
        assert!(run(RSI_BB_SHORT, &x, 0, r.len() - 1, &ps(&[("wick_mode", 0.0)])).unwrap().iter().any(|t| t.legs[0].i == 60));
    }

    #[test]
    fn live_trailing_stop_fills_at_the_level() {
        let mut v = setup();
        v.push((80.0, 82.0, 79.0, 80.0)); // High 82 below the trail 80.5 x 1.03; new lowest close 80 -> trail 82.4
        v.push((82.0, 84.0, 81.0, 82.5)); // High 84 >= 82.4 with the Open below it -> cover at 82.4
        let r = bars(&v);
        let x = Ctx::new(&r, &r, &[]);
        let t = run(RSI_BB_SHORT, &x, 0, r.len() - 1, &ps(&[("trail_exit", 1.0)])).unwrap();
        assert_eq!(t[0].exit_i, 62);
        assert!((t[0].exit_px - 80.0 * 1.03).abs() < 1e-9);
    }

    #[test]
    fn every_strategy_runs() {
        smoke(run, &IDS);
        let r = wave(420);
        let x = Ctx::new(&r, &r, &[]);
        assert!(params(RSI_BB_SHORT).is_some() && family_of(RSI_BB_SHORT).is_some());
        let opts = [
            vec![("rsi_period", 8.0), ("rsi_level", 50.0), ("wick_mode", 0.0)],
            vec![("rsi_period", 8.0), ("rsi_level", 50.0), ("wick_mode", 0.0), ("trail_type", 1.0), ("trail_point", 2.0), ("hard_sl", 1.0), ("hard_sl_exit", 1.0), ("trail_exit", 1.0)],
            vec![("rsi_period", 8.0), ("rsi_level", 50.0), ("wick_mode", 0.0), ("market_filter", 1.0), ("st_point", 4.0)],
            vec![("rsi_period", 8.0), ("rsi_level", 50.0), ("wick_mode", 0.0), ("trail_point", 1.0), ("hard_sl", 1.0), ("turnover", 1.0)],
        ];
        for kv in opts {
            for (from, to) in [(0, 419), (50, 300)] {
                let t = run(RSI_BB_SHORT, &x, from, to, &ps(&kv)).unwrap();
                for t in &t {
                    assert!(t.legs[0].i >= from && t.exit_i >= t.legs[0].i && t.exit_i <= to);
                    assert!(t.exit_px >= x.l[t.exit_i] - 1e-9 && t.exit_px <= x.h[t.exit_i] + 1e-9);
                }
                assert!(t.windows(2).all(|w| w[1].legs[0].i >= w[0].exit_i));
            }
        }
        assert_eq!(catalogue().len(), IDS.len());
    }
}
