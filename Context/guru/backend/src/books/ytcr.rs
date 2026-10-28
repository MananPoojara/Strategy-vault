//! YT Strategies · Candle Range Decrease and Volume Increase: the team's AmiBroker AFLs in "Old Topics/Candle
//! Range Decrease and Volume Increase" that trade individual stocks on daily bars (long only). Coded from the AFLs
//! as written, with the AFLs' own default settings; there is no book PDF. Each strategy runs on one stock at a time;
//! the team ran them over NIFTY 500 cash stocks (10 / 20 open positions); portfolio limits and sizing are not
//! modelled.
//!
//! Conventions as in `ytsw.rs` / `ytwh.rs` and the stock template in `ytbb.rs` (loop signals read by AmiBroker's
//! backtester, LC bars delaying exits, turnover = close x volume, dead-stock toggle default NO, trade prices kept
//! inside the bar's range). Several of the AFLs' Param defaults lie outside their own min-max ranges (e.g. trail
//! 2 with range 18-30); the defaults are used.
//!
//! Not coded separately: "Candle Range AFL_Sukun/Candle Range Decrease and Volume Increase.afl" is the original AFL
//! with different Param ranges only (same defaults).
#![allow(dead_code)]
use crate::books::ytbb::{amibt, dead_sigs, turnover, Sig4};
use crate::books::ytsw::{sma_filter, st_filter};
use crate::books::ytwh::bound;
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "yt_cr";
pub const ORIGINAL: &str = "ytcr_original";
pub const FLT: &str = "ytcr_filter";
pub const FLT_HSL: &str = "ytcr_filter_hard_stop";
pub const FLT_HSL_LOW: &str = "ytcr_filter_hard_stop_low";
pub const IDS: [&str; 4] = [ORIGINAL, FLT, FLT_HSL, FLT_HSL_LOW];

fn def(id: &'static str, name: &'static str, rules: &'static [&'static str], notes: &'static str) -> StrategyDef {
    StrategyDef { id, name, group: "YT Strategies · Candle Range Decrease and Volume Increase", session: 0, dir: "Long", status: Status::Live, needs_volume: true, needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim: None }
}

pub fn catalogue() -> Vec<StrategyDef> {
    vec![
        def(ORIGINAL, "Candle Range Decrease and Volume Increase", &[
            "F|Setup bar: its range (high - low) is below the previous bar's range and its volume is more than 20% above the previous bar's volume",
            "E|Buy at the close of the bar after a setup bar when the close is above the setup bar's high; not when high = close, not on the first bar",
            "X|Profit target: the high exceeds the buy price + 5% (sold at the target, or at the open less 0.5% when the open is above it)",
            "X|Trailing stop: the close is below the highest high since entry less 2% (sold at the close); an LC bar waits for the next non-LC bar",
        ], "Candle Range Decrease and Volume Increase-original.afl, coded as the AFL is written (its loop). Runs on one stock at a time; the team ran it over NIFTY 500 cash stocks (raw backtests with 10 / 20 open positions); portfolio limits not modelled. The trailing stop is raised with the bar's own high before the close is tested. Turnover = close x volume (the team stored turnover in OpenInt), off by default; dead-stock backtest default NO."),
        def(FLT, "Candle Range and Volume with ST Filter", &[
            "F|Setup bar as in the original (range down, volume up more than 20%)",
            "F|Filter on the stock itself: Supertrend (default daily, period 20, factor 1, mid point; ATR = mean of ATR(1)); SMA (close above the previous 200-period average) or off",
            "E|Buy at the close when the close is above the setup bar's high and the filter is up",
            "X|Profit target 5% and 2% trailing stop as in the original; an LC bar delays the sell only while the filter is not down",
        ], "Candle Range Decrease and Volume Increase_WITH FLT.afl, coded as the AFL is written. The filter is on the stock's own bars (no SetForeign) and is not an exit rule: a down filter only lets a sell go through on an LC bar. Runs on one stock at a time; the team ran it over NIFTY 500 cash stocks (20 open positions); portfolio limits not modelled. Turnover = close x volume, off by default; dead-stock backtest default NO."),
        def(FLT_HSL, "Candle Range and Volume with Filter and Hard Stop", &[
            "F|Setup bar and filter as in the filter AFL",
            "E|Buy at the close when the close is above the setup bar's high and the filter is up",
            "X|Profit target 5%, 2% trailing stop, and a hard stop: the close below the buy price less 1% (sold at the close)",
        ], "Candle Range Decrease and Volume Increase_WITH FLT+HSL.afl, coded as the AFL is written. Runs on one stock at a time; the team ran it over NIFTY 500 cash stocks (20 open positions); portfolio limits not modelled. Turnover = close x volume, off by default; dead-stock backtest default NO."),
        def(FLT_HSL_LOW, "Candle Range and Volume with Filter and Intraday Hard Stop", &[
            "F|Setup bar and filter as in the filter AFL",
            "E|Buy at the close when the close is above the setup bar's high and the filter is up",
            "X|Profit target 5%, 2% trailing stop, and a hard stop on the low: the low below the buy price less 1% sells at that level, or at the open less 0.5% when the open is below it",
        ], "Candle Range AFL_Sukun/Candle Range Decrease and Volume Increase_WITH FLT+HSL.afl, coded as the AFL is written (the hard stop tests the low and fills at the stop or a gap-adjusted open). A gap fill below the bar's low is moved to the low (AmiBroker price bound). Runs on one stock at a time; the team ran it over NIFTY 500 cash stocks; portfolio limits not modelled. Turnover = close x volume, off by default; dead-stock backtest default NO."),
    ]
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    let mut v = vec![
        p("vol_pct", "Volume increase %", 20.0, 0.0, 50.0, 1.0),
        p("tsl_pct", "Trailing stop % (0 off)", 2.0, 0.0, 50.0, 0.5),
        p("pt_pct", "Profit target % (0 off)", 5.0, 0.0, 70.0, 1.0),
        p("buffer_pct", "Gap buffer %", 0.5, 0.0, 5.0, 0.01),
    ];
    match id {
        ORIGINAL => {}
        FLT | FLT_HSL | FLT_HSL_LOW => {
            if id != FLT {
                v.push(p("hsl_pct", "Hard stop-loss % (0 off)", 1.0, 0.0, 55.0, 1.0));
            }
            v.extend([
                p("flt", "Filter (1 ST, 2 SMA, 3 off)", 1.0, 1.0, 3.0, 1.0),
                p("flt_tf", "Filter timeframe (1 daily, 2 weekly, 3 monthly)", 1.0, 1.0, 3.0, 1.0),
                p("st_point", "ST calculation point (1 mid point, 2 close, 3 pivot)", 1.0, 1.0, 3.0, 1.0),
                p("st_period", "ST period", 20.0, 1.0, 50.0, 1.0),
                p("st_factor", "ST factor", 1.0, 0.5, 5.0, 0.5),
                p("sma", "SMA period", 200.0, 1.0, 2000.0, 1.0),
            ]);
        }
        _ => return None,
    }
    v.extend([
        p("penny", "Penny stock price", 0.0, 0.0, 100.0, 1.0),
        p("turnover", "Turnover filter (0 no, 1 yes: close x volume)", 0.0, 0.0, 1.0, 1.0),
        p("dead_bt", "Dead stock backtest (0 no, 1 yes)", 0.0, 0.0, 1.0, 1.0),
    ]);
    Some(v)
}

const BIG: f64 = 9_999_999.0;

fn signals(id: &str, x: &Ctx, g: &dyn Fn(&str) -> f64) -> Sig4 {
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let n = c.len();
    let gi = |k: &str| g(k).round() as i64;
    let vol: Vec<f64> = x.r.iter().map(|r| r.volume.unwrap_or(f64::NAN)).collect();
    let vp = g("vol_pct");
    // Range_Cnd = Range < Ref(Range, -1); Volume_Cnd = V > Ref(V x (1 + pct), -1); false on bar 0
    let rc: Vec<bool> = (0..n).map(|i| i > 0 && h[i] - l[i] < h[i - 1] - l[i - 1]).collect();
    let vc: Vec<bool> = (0..n).map(|i| i > 0 && vol[i] > vol[i - 1] * (1.0 + vp / 100.0)).collect();
    let (dbs, dss) = dead_sigs(n, gi("dead_bt") == 1, false);
    let turn = turnover(x, gi("turnover") == 1);
    let penny = g("penny");
    let bcf: Vec<bool> = (0..n).map(|i| h[i] != c[i] && turn[i] && c[i] > penny && dbs[i]).collect();
    let lc: Vec<bool> = (0..n).map(|i| l[i] == c[i]).collect();
    let (fb, fs): (Vec<bool>, Vec<bool>) = if id == ORIGINAL {
        (vec![true; n], vec![false; n])
    } else {
        let tf = match gi("flt_tf") { 2 => 'W', 3 => 'M', _ => 'D' };
        match gi("flt") {
            2 => sma_filter(x, tf, g("sma").round().max(1.0) as usize, true),
            3 => (vec![true; n], vec![false; n]),
            _ => {
                let m = st_filter(x, tf, gi("st_point"), g("st_period").round().max(1.0) as usize, g("st_factor"));
                (m.iter().map(|&t| t == 1).collect(), m.iter().map(|&t| t == -1).collect())
            }
        }
    };
    let (tsl_pct, pt_pct, buf) = (g("tsl_pct"), g("pt_pct"), g("buffer_pct"));
    let hsl_pct = if id == FLT_HSL || id == FLT_HSL_LOW { g("hsl_pct") } else { 0.0 };
    let low_stop = id == FLT_HSL_LOW;
    let mut s = Sig4::new(n);
    let (mut pos, mut skip) = (false, false);
    let (mut tsl, mut hsl, mut hh, mut pt) = (0.0f64, 0.0f64, BIG, BIG);
    for i in 1..n {
        if !pos && rc[i - 1] && vc[i - 1] && c[i] > h[i - 1] && bcf[i] && fb[i] {
            s.buy[i] = true;
            if tsl_pct > 0.0 {
                hh = h[i];
                tsl = hh - hh * tsl_pct / 100.0;
            }
            if pt_pct > 0.0 {
                pt = c[i] + c[i] * pt_pct / 100.0;
            }
            if hsl_pct > 0.0 {
                hsl = c[i] - c[i] * hsl_pct / 100.0;
            }
            pos = true;
        } else if pos && !skip {
            if tsl_pct > 0.0 {
                hh = h[i].max(hh);
                tsl = hh - hh * tsl_pct / 100.0;
            }
            let stop = if low_stop { l[i] < hsl } else { c[i] < hsl };
            if h[i] > pt || c[i] < tsl || stop || dss[i] {
                if !lc[i] || dss[i] || fs[i] {
                    let (px, r) = if h[i] > pt {
                        (if o[i] > pt { o[i] - o[i] * buf / 100.0 } else { pt }, "Profit target")
                    } else if low_stop && l[i] < hsl {
                        (if o[i] < hsl { o[i] - o[i] * buf / 100.0 } else { hsl }, "Hard stop")
                    } else if c[i] < tsl {
                        (c[i], "Trailing stop")
                    } else if stop {
                        (c[i], "Hard stop")
                    } else {
                        (c[i], "Last bar")
                    };
                    s.sell[i] = Some((bound(x, i, px), r));
                    pos = false;
                    tsl = 0.0;
                    hh = BIG;
                    pt = BIG;
                } else {
                    skip = true;
                }
            }
        } else if skip && (!lc[i] || dss[i] || fs[i]) {
            s.sell[i] = Some((c[i], "Sell after LC bar"));
            pos = false;
            skip = false;
            tsl = 0.0;
            hh = BIG;
            pt = BIG;
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
    Some(amibt(x, &signals(id, x, &g), from, to))
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let name = catalogue().into_iter().find(|d| d.id == id)?.name;
    Some((match id {
        ORIGINAL => "ytcr_original",
        FLT => "ytcr_filter",
        FLT_HSL => "ytcr_filter_hard_stop",
        FLT_HSL_LOW => "ytcr_filter_hard_stop_low",
        _ => return None,
    }, name, "Candle Range Decrease and Volume Increase"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::books::ytbb::stocktest::{rows, smoke, with_volume};
    use crate::books::ytwh::testbars::ps;
    use crate::Row;

    fn go(id: &str, r: &[Row], kv: &[(&str, f64)]) -> Vec<(usize, f64, usize, f64, bool)> {
        let x = Ctx::new(r, r, &[]);
        run(id, &x, 0, r.len() - 1, &ps(kv)).unwrap().into_iter().map(|t| (t.legs[0].i, t.legs[0].px, t.exit_i, t.exit_px, t.open)).collect()
    }

    /// bar 1 is the setup (narrower range than bar 0, volume 130 > 100 x 1.2); bar 2 closes above bar 1's high
    fn setup() -> Vec<(f64, f64, f64, f64)> {
        vec![(100.0, 104.0, 96.0, 100.0), (100.0, 102.0, 98.0, 101.0), (101.0, 103.5, 100.5, 103.0)]
    }
    const VOL: [f64; 3] = [100.0, 130.0, 100.0];

    #[test]
    fn buys_after_setup_and_books_profit() {
        let mut v = setup();
        // bar 3: high 108.5 > 103 x 1.05 = 108.15 -> profit target
        v.push((104.0, 108.5, 103.5, 108.0));
        let mut vol = VOL.to_vec();
        vol.push(100.0);
        let t = go(ORIGINAL, &with_volume(rows(&v), &vol), &[]);
        assert_eq!((t[0].0, t[0].1, t[0].2), (2, 103.0, 3));
        assert!((t[0].3 - 103.0 * 1.05).abs() < 1e-9);
        // a gap above the target sells at the open less 0.5%
        v[3] = (110.0, 111.0, 109.0, 110.5);
        let t = go(ORIGINAL, &with_volume(rows(&v), &vol), &[]);
        assert!((t[0].3 - 110.0 * 0.995).abs() < 1e-9);
        // without the volume increase there is no setup
        assert!(go(ORIGINAL, &with_volume(rows(&v), &[100.0, 110.0, 100.0, 100.0]), &[]).is_empty());
    }

    #[test]
    fn trailing_stop_uses_the_bars_own_high() {
        let mut v = setup();
        // bar 3 high 106 -> trail 103.88; close 103.5 < 103.88 -> sell at the close
        v.push((104.0, 106.0, 103.0, 103.5));
        let t = go(ORIGINAL, &with_volume(rows(&v), &[100.0, 130.0, 100.0, 100.0]), &[]);
        assert_eq!((t[0].2, t[0].3), (3, 103.5));
        // the low-based hard stop (filter off): low 101.5 < 103 x 0.99 = 101.97 sells at the stop level
        let mut w = setup();
        w.push((102.5, 103.4, 101.5, 103.2));
        let t = go(FLT_HSL_LOW, &with_volume(rows(&w), &[100.0, 130.0, 100.0, 100.0]), &[("flt", 3.0), ("tsl_pct", 0.0)]);
        assert_eq!(t[0].2, 3);
        assert!((t[0].3 - 103.0 * 0.99).abs() < 1e-9);
        // the close-based hard stop does not fire on that bar
        assert!(go(FLT_HSL, &with_volume(rows(&w), &[100.0, 130.0, 100.0, 100.0]), &[("flt", 3.0), ("tsl_pct", 0.0)])[0].4);
    }

    #[test]
    fn defaults_trade_on_a_wave() {
        let r = crate::books::ytbb::stocktest::with_volume(crate::books::ytwh::testbars::wave(420), &[1.0e5, 1.6e5, 0.9e5, 2.2e5, 1.2e5]);
        let x = Ctx::new(&r, &r, &[]);
        for id in IDS {
            let t = run(id, &x, 0, r.len() - 1, &Params::new()).unwrap();
            eprintln!("{id}: {} trades", t.len());
            assert!(!t.is_empty(), "{id}");
        }
    }

    #[test]
    fn every_strategy_runs() {
        smoke(run, &IDS, &[("flt", 2.0), ("flt_tf", 2.0), ("dead_bt", 1.0)]);
        smoke(run, &IDS, &[("vol_pct", 0.0), ("tsl_pct", 5.0), ("pt_pct", 10.0), ("hsl_pct", 3.0)]);
    }
}
