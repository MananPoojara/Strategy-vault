//! YT Strategies · HH Short Strategy: the team's AmiBroker AFL in "Old Topics/HH Short Strategy" (different MAs + RSI
//! + ADX short with hard, trailing and higher-high stops), run on one series at a time on daily bars. Coded from the
//! AFL as written, with the AFL's own default settings; there is no book PDF.
//!
//! AmiBroker behaviour followed:
//! - `SetBarsRequired(100000, 0)` loads the whole history, so the loop runs from the data's first bar and the
//!   backtester opens trades only in the test window; shorts and covers at the close unless the AFL sets the price,
//!   which is kept inside the bar's low-high range;
//! - the loop is an if / else-if chain: no entry on a cover bar, no cover on the entry bar;
//! - the AFL keeps its stop levels in arrays written only on some bars (`HARDSTOPLOSS[i]`, `TRAILSL[i]`, `HIGHSL[i]`,
//!   `SLLOW[i]`, `DAYCOUNT[i]`, `HIGHCOUNT[i]`) and reads the previous bar's element; an element never written keeps
//!   the array's starting value (999999, counts 0). This is followed literally: e.g. a holding bar whose High equals
//!   the previous High leaves the higher-high stop at 999999 for the next bar;
//! - in the loop a Null element is -1e10 (no entry while MA / RSI / ADX do not exist yet);
//! - the default (filter Off) entry needs a fresh RSI cross below the level since the last cover (the AFL's `abc`
//!   flag); the %Swing filter version has no such flag (as written);
//! - the ST filter (version On + ST) is the team's Supertrend loop on the series itself, default monthly ST(5, 2) on
//!   the mid point, expanded with expandLast; the %Swing filter is the double % swing on daily Closes (up band
//!   Close + 45%, down band Close − 40%);
//! - the dead-stock toggle defaults to NO (no short on bar 0, no forced cover) and is not offered.
//!
//! The file name's "22-44% Double % Swing" does not match the AFL's swing defaults (bear 40%, bull 45%); the
//! defaults are used, and the swing is used only by the non-default %Swing version.
#![allow(dead_code)]
use crate::books::ytd7::ma;
use crate::books::ytds::{backtest, st_trend, LoopSig};
use crate::books::ytroc::rsi_opt;
use crate::books::ytsw::swing;
use crate::books::ytwh::loop_val;
use crate::connors::{dmi, Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "yt_hh";
pub const MA_RSI_ADX_SHORT: &str = "ythh_ma_rsi_adx_short";
pub const IDS: [&str; 1] = [MA_RSI_ADX_SHORT];

const GROUP: &str = "YT Strategies · HH Short Strategy";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef {
        id: MA_RSI_ADX_SHORT,
        name: "Different MA + RSI + ADX Short with Trailing and Higher-High Stops",
        group: GROUP,
        session: 0,
        dir: "Short",
        status: Status::Live,
        needs_volume: false,
        needs_vix: false,
        needs_series: &[],
        book: BOOK,
        rules: &[
            "F|Arm: RSI(8) crosses below 35 (below now, above on the previous bar); disarmed by each cover",
            "F|Close < MA(Low, 6)",
            "F|MA(Low, 6) < MA(Close, 3)",
            "F|RSI(8) < 35",
            "F|ADX(6) > 18",
            "F|Flat; not on the first bar",
            "F|Optional filter (default off): ST filter down (monthly ST(5, 2), mid point) or the double % swing down (+45% / −40%)",
            "E|Short at the Close",
            "X|Hard stop on the bar after entry only: High > entry Close + 8%; at it, or at Open + 0.25% when the Open is above it",
            "X|Higher-high stop: after a bar with a higher High, cover when a later High exceeds that High (same fill rule)",
            "X|Profit target: Low ≤ entry Close − 17%; at it, or at Open + 0.25% when the Open is below it",
            "X|Cover at the Close when Close ≥ MA(High, 1) (closes at its High) or Close > MA(Close, 3)",
            "X|Options (default off): % trailing stop above the lowest Low, Low + n% close stop",
        ],
        notes: "Differnt MA + RSI + ADX Entry Exit V2 With trail sl_Final AFL - 22-44% Double % Swing.afl (HH Short Strategy), coded as the AFL is written (version Off, its default). The hard stop is checked only while DAYCOUNT[i-1] ≤ HARD SL DAYS (1): the bar after entry. The higher-high stop level resets to 999999 on a holding bar whose High equals the previous High (the AFL leaves that element unwritten). RSI is AmiBroker's Wilder RSI; ADX is the Wilder ADX seeded with simple averages. Runs on one series at a time; the team's sizing (10% of equity, 10 positions) is not modelled.",
        claim: None,
    }]
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    if id != MA_RSI_ADX_SHORT {
        return None;
    }
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    Some(vec![
        p("version", "Filter (0 off, 1 on: ST filter, 2 on: double % swing)", 0.0, 0.0, 2.0, 1.0),
        p("pt_pct", "Profit target % (0 off)", 17.0, 0.0, 20.0, 0.05),
        p("high_ma", "High MA period", 1.0, 1.0, 300.0, 1.0),
        p("low_ma", "Low MA period", 6.0, 3.0, 10.0, 1.0),
        p("close_ma", "Close MA period", 3.0, 2.0, 10.0, 1.0),
        p("adx_period", "ADX period", 6.0, 1.0, 60.0, 2.0),
        p("adx_value", "ADX above", 18.0, 0.0, 50.0, 2.0),
        p("rsi_period", "RSI period", 8.0, 2.0, 22.0, 1.0),
        p("rsi_value", "RSI short value", 35.0, 15.0, 40.0, 5.0),
        p("hard_sl", "Hard stop % (0 off)", 8.0, 0.0, 10.0, 1.0),
        p("hard_sl_days", "Hard stop days", 1.0, 0.0, 2.0, 1.0),
        p("trail_pct", "Trailing stop % above the lowest Low (0 off)", 0.0, 0.0, 8.0, 1.0),
        p("low_n_pct", "Close above lowest Low + n% (0 off)", 0.0, 0.0, 50.0, 1.0),
        p("hh_days", "Higher-high days (0 off)", 1.0, 0.0, 2.0, 1.0),
        p("st_tf", "ST filter time frame (0 daily, 1 weekly, 2 monthly)", 2.0, 0.0, 2.0, 1.0),
        p("st_point", "ST calculation point (0 mid point, 1 close, 2 pivot)", 0.0, 0.0, 2.0, 1.0),
        p("st_period", "ST period", 5.0, 5.0, 25.0, 1.0),
        p("st_factor", "ST factor", 2.0, 1.0, 7.0, 1.0),
        p("bear_pct", "Swing % for bear market (down band)", 40.0, 10.0, 55.0, 1.0),
        p("bull_pct", "Swing % for bull market (up band)", 45.0, 10.0, 60.0, 1.0),
    ])
}

fn hh_loop(x: &Ctx, g: &dyn Fn(&str) -> f64) -> LoopSig {
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let n = c.len();
    let gi = |k: &str| g(k).round() as i64;
    let gu = |k: &str| g(k).round().max(1.0) as usize;
    let lv = |v: Vec<Option<f64>>| v.into_iter().map(loop_val).collect::<Vec<f64>>();
    let high_ma = lv(ma(h, gu("high_ma")));
    let low_ma = lv(ma(l, gu("low_ma")));
    let close_ma = lv(ma(c, gu("close_ma")));
    let rsi = lv(rsi_opt(c, gu("rsi_period")));
    let adx: Vec<f64> = dmi(x.r, gu("adx_period")).1.into_iter().map(|v| if v.is_nan() { -1e10 } else { v }).collect();
    let version = gi("version");
    let mtrend = if version == 1 {
        let tf = match gi("st_tf") {
            0 => 'D',
            1 => 'W',
            _ => 'M',
        };
        st_trend(x, tf, gi("st_point") + 1, gu("st_period"), g("st_factor"))
    } else {
        vec![0; n]
    };
    let trendflt = if version == 2 {
        let up: Vec<f64> = c.iter().map(|v| v * (1.0 + g("bull_pct") / 100.0)).collect();
        let dn: Vec<f64> = c.iter().map(|v| v * (1.0 - g("bear_pct") / 100.0)).collect();
        swing(c, &up, &dn, 0.0).0
    } else {
        vec![0; n]
    };
    let (rsi_v, adx_v) = (g("rsi_value"), g("adx_value"));
    let (pt_pct, hard_pct, ndays, slpct, low_n, hdays) = (g("pt_pct"), g("hard_sl"), g("hard_sl_days"), g("trail_pct"), g("low_n_pct"), g("hh_days"));
    const BIG: f64 = 999_999.0;
    let mut hard = vec![BIG; n];
    let mut sllow = vec![BIG; n];
    let mut trailsl = vec![BIG; n];
    let mut highsl = vec![BIG; n];
    let mut daycount = vec![0.0f64; n];
    let mut highcount = vec![0.0f64; n];
    // LowTrailSL is recomputed before each read, so it is a local below
    let (mut ll_after, mut pt) = (BIG, 0.0f64);
    let mut abc = true;
    let mut position = false;
    let gap_up = |i: usize, lvl: f64| if o[i] > lvl { o[i] + o[i] * 0.25 / 100.0 } else { lvl };
    let mut s = LoopSig::new(-1.0, n);
    for i in 1..n {
        if version != 2 && abc && rsi[i] < rsi_v && rsi[i - 1] > rsi_v {
            abc = false;
        }
        let filter = match version {
            1 => mtrend[i] == -1,
            2 => trendflt[i] == -1,
            _ => true,
        };
        let armed = version == 2 || !abc;
        let mut covered: Option<(f64, &'static str)> = None;
        if !position && armed && filter && c[i] < low_ma[i] && low_ma[i] < close_ma[i] && rsi[i] < rsi_v && adx[i] > adx_v {
            s.entry[i] = Some(c[i]);
            ll_after = l[i];
            sllow[i] = l[i];
            if pt_pct > 0.0 {
                pt = (1.0 - pt_pct * 0.01) * c[i];
            }
            hard[i] = if hard_pct > 0.0 { c[i] + c[i] * hard_pct / 100.0 } else { BIG };
            daycount[i] = daycount[i - 1] + 1.0;
            highsl[i] = BIG;
            position = true;
        } else if position && h[i] > hard[i - 1] && daycount[i - 1] <= ndays {
            covered = Some((gap_up(i, hard[i - 1]), if o[i] > hard[i - 1] { "Hard stop (gap)" } else { "Hard stop" }));
        } else if position && h[i] > trailsl[i - 1] {
            covered = Some((gap_up(i, trailsl[i - 1]), if o[i] > trailsl[i - 1] { "Trailing stop (gap)" } else { "Trailing stop" }));
        } else if position && h[i] > highsl[i - 1] {
            covered = Some((gap_up(i, highsl[i - 1]), if o[i] > highsl[i - 1] { "Higher-high stop (gap)" } else { "Higher-high stop" }));
        } else if position && l[i] <= pt {
            covered = Some((if o[i] < pt { o[i] + o[i] * 0.25 / 100.0 } else { pt }, if o[i] < pt { "Profit target (gap)" } else { "Profit target" }));
        } else if position && (c[i] >= high_ma[i] || c[i] > close_ma[i]) {
            covered = Some((c[i], if c[i] > close_ma[i] { "Close above MA(Close)" } else { "Close at or above MA(High)" }));
        } else if position {
            daycount[i] = daycount[i - 1] + 1.0;
            hard[i] = hard[i - 1];
            sllow[i] = l[i].min(sllow[i - 1]);
            trailsl[i] = if slpct > 0.0 { if sllow[i] < sllow[i - 1] { sllow[i] + sllow[i] * slpct / 100.0 } else { trailsl[i - 1] } } else { BIG };
            if hdays > 0.0 {
                if h[i] > h[i - 1] {
                    highcount[i] = highcount[i - 1] + 1.0;
                    highsl[i] = if highcount[i] >= hdays { h[i] } else { highsl[i - 1] };
                } else if h[i] < h[i - 1] {
                    highsl[i] = highsl[i - 1];
                    highcount[i] = 0.0;
                }
            }
            if low_n > 0.0 {
                ll_after = l[i].min(ll_after);
                let low_trail = (1.0 + low_n * 0.01) * ll_after;
                if c[i] > low_trail {
                    covered = Some((c[i], "Close above lowest Low + n%"));
                }
            }
        }
        if let Some(e) = covered {
            s.exit[i] = Some(e);
            hard[i] = BIG;
            trailsl[i] = BIG;
            highsl[i] = BIG;
            ll_after = BIG;
            daycount[i] = 0.0;
            highcount[i] = 0.0;
            pt = 0.0;
            position = false;
            abc = true;
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
    Some(backtest(x, &hh_loop(x, &g), from, to))
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let d = catalogue().into_iter().find(|d| d.id == id)?;
    Some((d.id, d.name, "HH Short Strategy"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::books::ytwh::testbars::{bars, ps, smoke, wave};

    /// 30 rising bars with long lower wicks (ADX high, RSI high), then bar 30 drops to 110: RSI(8) 100 -> 27,
    /// Close 110 < MA(Low, 6) 119.9 < MA(Close, 3) 122.3: short at 110 (hard stop 118.8, target 91.3).
    fn setup() -> Vec<(f64, f64, f64, f64)> {
        let mut v: Vec<(f64, f64, f64, f64)> = (0..30).map(|k| { let c = 100.0 + k as f64; (c - 0.5, c + 0.5, c - 5.0, c) }).collect();
        v.push((128.0, 129.3, 109.5, 110.0));
        v
    }

    fn go(kv: &[(&str, f64)], v: &[(f64, f64, f64, f64)]) -> Vec<Trade> {
        let r = bars(v);
        let x = Ctx::new(&r, &r, &[]);
        run(MA_RSI_ADX_SHORT, &x, 0, r.len() - 1, &ps(kv)).unwrap()
    }

    #[test]
    fn shorts_after_the_rsi_cross_and_covers_on_close_above_ma3() {
        let mut v = setup();
        v.push((110.0, 113.0, 109.8, 112.5)); // 112.5 < MA(C, 3) 117.2: holds
        v.push((112.0, 114.0, 111.5, 113.5)); // 113.5 > MA(C, 3) 112.0: cover at the close
        let t = go(&[], &v);
        assert_eq!(t.len(), 1);
        assert_eq!((t[0].dir, t[0].legs[0].i, t[0].exit_i), (-1.0, 30, 32));
        assert!((t[0].legs[0].px - 110.0).abs() < 1e-9);
        assert_eq!(t[0].reason, "Close above MA(Close)");
        assert!((t[0].exit_px - 113.5).abs() < 1e-9);
    }

    #[test]
    fn hard_stop_on_the_bar_after_entry() {
        let mut v = setup();
        v.push((121.0, 123.0, 120.5, 122.0)); // gap above 118.8: Open + 0.25%
        let t = go(&[], &v);
        assert_eq!((t[0].legs[0].i, t[0].exit_i), (30, 31));
        assert_eq!(t[0].reason, "Hard stop (gap)");
        assert!((t[0].exit_px - 121.0 * 1.0025).abs() < 1e-9);
        // with the hard stop off the same bar covers on Close > MA(C, 3) (122 > 120.3)
        let t = go(&[("hard_sl", 0.0)], &v);
        assert_eq!(t[0].reason, "Close above MA(Close)");
    }

    #[test]
    fn higher_high_stop_covers_above_the_last_higher_high() {
        let mut v = setup();
        v.push((110.0, 130.0, 109.0, 111.0)); // higher High 130; Close below MA(C, 3) and the High
        v.push((112.0, 131.0, 111.0, 112.0)); // High above 130: cover at 130
        let t = go(&[("hard_sl", 0.0)], &v);
        assert_eq!(t[0].exit_i, 32);
        assert_eq!(t[0].reason, "Higher-high stop");
        assert!((t[0].exit_px - 130.0).abs() < 1e-9);
    }

    #[test]
    fn rsi_arm_needed_again_after_a_cover() {
        // after the cover on bar 32 the setup holds again on bar 33, but RSI has not crossed below 35 again
        let mut v = setup();
        v.push((110.0, 113.0, 109.8, 112.5));
        v.push((112.0, 114.0, 111.5, 113.5));
        v.push((113.0, 113.2, 95.0, 96.0));
        let t = go(&[], &v);
        assert_eq!(t.len(), 1);
    }

    #[test]
    fn every_strategy_runs() {
        smoke(run, &IDS);
        let r = wave(420);
        let x = Ctx::new(&r, &r, &[]);
        assert!(params(MA_RSI_ADX_SHORT).is_some() && family_of(MA_RSI_ADX_SHORT).is_some());
        for kv in [vec![("version", 1.0)], vec![("version", 2.0), ("bear_pct", 10.0), ("bull_pct", 10.0)], vec![("trail_pct", 2.0), ("low_n_pct", 3.0), ("hh_days", 2.0)], vec![("version", 1.0), ("st_tf", 0.0)]] {
            for (from, to) in [(0, 419), (50, 300)] {
                let t = run(MA_RSI_ADX_SHORT, &x, from, to, &ps(&kv)).unwrap();
                for t in &t {
                    assert!(t.legs[0].i >= from && t.exit_i >= t.legs[0].i && t.exit_i <= to);
                    assert!(t.exit_px >= x.l[t.exit_i] - 1e-9 && t.exit_px <= x.h[t.exit_i] + 1e-9);
                }
                assert!(t.windows(2).all(|w| w[1].legs[0].i >= w[0].exit_i));
            }
        }
        let t = run(MA_RSI_ADX_SHORT, &x, 0, 419, &Params::new()).unwrap();
        assert!(!t.is_empty());
        assert_eq!(catalogue().len(), IDS.len());
    }
}
