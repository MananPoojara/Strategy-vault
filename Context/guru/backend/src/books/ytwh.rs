//! YT Strategies · 52 Week High: the team's AmiBroker AFLs in "Old Topics/52 Week High" that trade an index on daily
//! bars (spot Nifty / Bank Nifty). Coded from the AFLs as written, with the AFLs' own default settings; there is no
//! book PDF. The AmiBroker conventions are those in `ytsw.rs`; in addition:
//! - `HHV` / `LLV` near the start of the data use the bars available (no warm-up), as found for the Kora Reddy AFLs;
//! - inside a `for` loop an AFL array element that is Null is the number -1e10 (so `C[i] > MA[i]` is true while the
//!   average is still Null); array-level comparisons with a leading Null are false (the `ytsw.rs` rule);
//! - `Prec(x, 2)` truncates to 2 decimals;
//! - prices a loop assigns (BuyPrice / SellPrice / CoverPrice) are kept inside the bar's low-high range, as
//!   AmiBroker's price bound checking does;
//! - `ApplyStop` (not provided by the engine) is implemented as AmiBroker runs it with trades at the close and
//!   `ActivateStopsImmediately` off: checked from the bar after entry, the loss stop before the profit stop, a stop
//!   level gapped through fills at the open, and a stop exit takes precedence over a Sell signal on the same bar.
//!
//! The 52-Week Highest Close ST Exit folder holds a copy of the ST and profit booking AFL (Param instead of Optimize,
//! same defaults); it is not coded separately. The Optimization folder's AFL is a stock-universe run and is not here.
#![allow(dead_code)]
use crate::books::ytsw::{close_swing, compress, exrem, expand_last, st_filter};
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "yt_wh";
pub const SWING_EXIT: &str = "ytwh_swing_exit";
pub const ST_PB: &str = "ytwh_st_profit_booking";
pub const IDS: [&str; 2] = [SWING_EXIT, ST_PB];

fn def(id: &'static str, name: &'static str, dir: &'static str, rules: &'static [&'static str], notes: &'static str) -> StrategyDef {
    StrategyDef { id, name, group: "YT Strategies · 52 Week High", session: 0, dir, status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim: None }
}

pub fn catalogue() -> Vec<StrategyDef> {
    vec![
        def(SWING_EXIT, "52-Week High Close Breakout, % Swing Exit", "Long", &[
            "F|Weekly bars: HWC = highest weekly close of the last 52 weeks; the level is the previous week's HWC, expanded to daily bars (on a week's last day the level includes the week just before, on earlier days it lags one more week)",
            "E|Buy at the close when the daily close is above that level; not on the first bar and not on an upper-circuit bar (open = high = close to 2 decimals)",
            "X|Sell at the close when the daily close % swing (default 10%) turns down, unless open = low = close (lower circuit)",
            "X|Optional AmiBroker stops (off by default): hard stop-loss % and profit booking %",
        ], "52 week high with %swing exit_FINAL.afl (52 Week High and % Swing Exit Final), coded as the AFL is written. The header comment says SuperTrend 5/2 but the code exits on a 10% close swing (the code is followed). Buy is not gated by the swing, so a buy and a sell can fall on the same bar; ExRem and the backtester then sell and buy again as AmiBroker does. Dead-stock, penny and turnover options are stock-template settings left at their defaults (off). The R-multiple / SQN reporting part of the AFL has no trading effect."),
        def(ST_PB, "52-Week High, ST Re-arm and Profit Booking", "Long", &[
            "F|Supertrend of the series on the filter timeframe (default daily, period 20, factor 1, mid point; ATR = simple average of ATR(1)), expanded to daily bars",
            "F|Re-arm flag: set on a bar where the ST turns up after being down the previous bar; stays set until an entry uses it",
            "E|Buy at the close when flat, the flag is set and the close is above the highest close of the previous 260 bars; never on the last bar of the data",
            "X|Sell when the ST is down (at the close), when the high exceeds the profit level (buy price + 1%: at that level, or at the open less 0.25% if the open is above it), or on the last bar of the data",
        ], "52-WEEEK HIGH + ST AND PROFIT BOOKING.afl, coded as the AFL is written (its loop; no ExRem). The default profit % of 1 is the AFL's. Penny, turnover and circuit filters are ParamList options defaulting to OFF and are not offered. The AFL's comment calls the exit a % swing; the code uses the Supertrend."),
    ]
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    Some(match id {
        SWING_EXIT => vec![
            p("exit_pct", "Exit % swing", 10.0, 5.0, 25.0, 1.0),
            p("hard_stop", "Hard stop-loss % (0 off)", 0.0, 0.0, 50.0, 1.0),
            p("pb_on", "Profit booking stop (0 no, 1 yes)", 0.0, 0.0, 1.0, 1.0),
            p("pb_pct", "Profit booking %", 20.0, 5.0, 100.0, 5.0),
        ],
        ST_PB => vec![
            p("flt_tf", "Filter timeframe (1 daily, 2 weekly, 3 monthly)", 1.0, 1.0, 3.0, 1.0),
            p("st_point", "ST calculation point (1 mid point, 2 close, 3 pivot)", 1.0, 1.0, 3.0, 1.0),
            p("st_period", "ST period", 20.0, 2.0, 20.0, 2.0),
            p("st_factor", "ST factor", 1.0, 1.0, 5.0, 1.0),
            p("profit", "Profit booking % (0 off)", 1.0, 0.0, 100.0, 1.0),
        ],
        _ => return None,
    })
}

// ---------------------------------------------------------------- shared by the yt_wh / yt_at / yt_vt / yt_pin AFLs

/// AFL HHV(v, n) using the bars available near the start.
pub(crate) fn hhv(v: &[f64], n: usize) -> Vec<f64> {
    let n = n.max(1);
    (0..v.len()).map(|i| v[(i + 1).saturating_sub(n)..=i].iter().cloned().fold(f64::NEG_INFINITY, f64::max)).collect()
}
/// AFL LLV(v, n) using the bars available near the start.
pub(crate) fn llv(v: &[f64], n: usize) -> Vec<f64> {
    let n = n.max(1);
    (0..v.len()).map(|i| v[(i + 1).saturating_sub(n)..=i].iter().cloned().fold(f64::INFINITY, f64::min)).collect()
}
/// AFL Prec(v, 2): truncated to 2 decimals.
pub(crate) fn prec2(v: f64) -> f64 {
    (v * 100.0).trunc()
}
/// AmiBroker price bound checking: a trade price outside the bar's range is moved to the low or the high.
pub(crate) fn bound(x: &Ctx, i: usize, px: f64) -> f64 {
    px.max(x.l[i]).min(x.h[i])
}
/// The AFL loop Null in a comparison: -1e10.
pub(crate) fn loop_val(v: Option<f64>) -> f64 {
    v.unwrap_or(-1e10)
}

/// One position from a loop AFL run over the whole history: entry bar and price, and the exit (bar, price, bar on
/// whose close the exit rule fired, reason) if the AFL exits it.
pub(crate) struct Ev {
    pub dir: f64,
    pub e: usize,
    pub epx: f64,
    pub exit: Option<(usize, f64, usize, &'static str)>,
}

/// AmiBroker range backtest of loop signals: positions entered in from..=to; one still open after `to` is marked
/// there at the close.
pub(crate) fn collect(x: &Ctx, evs: Vec<Ev>, from: usize, to: usize) -> Vec<Trade> {
    evs.into_iter()
        .filter(|v| v.e >= from && v.e <= to)
        .map(|v| {
            let (exit_i, exit_px, signal_i, reason, open) = match v.exit {
                Some((i, px, s, r)) if i <= to => (i, px, s, r, false),
                _ => (to, x.c[to], to, "open", true),
            };
            Trade { dir: v.dir, legs: vec![Leg { i: v.e, px: v.epx, w: 1.0 }], exit_i, exit_px, reason: reason.into(), entry_kind: "close", signal_i, open, exits: Vec::new() }
        })
        .collect()
}

/// AmiBroker long backtest at the close of ExRem'd Buy / Sell arrays with optional ApplyStop loss and profit stops
/// (percent of the entry price; 0 = off), checked from the bar after entry.
fn backtest_stops(x: &Ctx, buy: &[bool], sell: &[bool], loss: f64, profit: f64, from: usize, to: usize) -> Vec<Trade> {
    let buy = exrem(buy, sell);
    let sell = exrem(sell, &buy);
    let mut evs = Vec::new();
    let mut pos: Option<(usize, f64)> = None;
    for i in from..=to {
        if let Some((e, epx)) = pos {
            let mut out: Option<(f64, &'static str)> = None;
            if i > e && loss > 0.0 {
                let lv = epx * (1.0 - loss / 100.0);
                if x.l[i] <= lv {
                    out = Some((if x.o[i] < lv { x.o[i] } else { lv }, "Hard stop"));
                }
            }
            if out.is_none() && i > e && profit > 0.0 {
                let lv = epx * (1.0 + profit / 100.0);
                if x.h[i] >= lv {
                    out = Some((if x.o[i] > lv { x.o[i] } else { lv }, "Profit booking"));
                }
            }
            if out.is_none() && sell[i] {
                out = Some((x.c[i], "Sell signal"));
            }
            if let Some((px, r)) = out {
                evs.push(Ev { dir: 1.0, e, epx, exit: Some((i, px, i, r)) });
                pos = None;
            }
        }
        if pos.is_none() && buy[i] {
            pos = Some((i, x.c[i]));
        }
    }
    if let Some((e, epx)) = pos {
        evs.push(Ev { dir: 1.0, e, epx, exit: None });
    }
    collect(x, evs, from, to)
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
    Some(match id {
        SWING_EXIT => {
            // TimeFrameSet(inWeekly); HWC = HHV(C, 52); TimeFrameExpand(Ref(HWC, -1), inWeekly)
            let wk = compress(x, 'W');
            let hw = hhv(&wk.c, 52);
            let lvl_w: Vec<Option<f64>> = (0..wk.c.len()).map(|k| if k == 0 { None } else { Some(hw[k - 1]) }).collect();
            let lvl = expand_last(&lvl_w, &wk);
            let tr = close_swing(c, g("exit_pct"));
            let upc = |i: usize| prec2(o[i]) == prec2(c[i]) && prec2(o[i]) == prec2(h[i]);
            let loc = |i: usize| prec2(o[i]) == prec2(c[i]) && prec2(o[i]) == prec2(l[i]);
            let buy: Vec<bool> = (0..n).map(|i| lvl[i].map_or(false, |v| c[i] > v) && i != 0 && !upc(i)).collect();
            let sell: Vec<bool> = (0..n).map(|i| tr[i] == -1 && !loc(i)).collect();
            let pb = if gi("pb_on") == 1 { g("pb_pct") } else { 0.0 };
            backtest_stops(x, &buy, &sell, g("hard_stop"), pb, from, to)
        }
        ST_PB => {
            let tf = match gi("flt_tf") { 2 => 'W', 3 => 'M', _ => 'D' };
            let m = st_filter(x, tf, gi("st_point"), gu("st_period"), g("st_factor"));
            let (fb, fs): (Vec<bool>, Vec<bool>) = (m.iter().map(|&v| v == 1).collect(), m.iter().map(|&v| v == -1).collect());
            let hwc = hhv(c, 260);
            let profit = g("profit");
            let mut trendj = vec![0u8; n];
            let mut pb = vec![0.0f64; n];
            let mut pos = false;
            let mut evs = Vec::new();
            for i in 1..n {
                trendj[i] = if fs[i - 1] && fb[i] { 1 } else if trendj[i - 1] == 1 { 1 } else { 0 };
                if !pos && c[i] > hwc[i - 1] && c[i] > 0.0 && i != n - 1 && trendj[i] == 1 {
                    pb[i] = if profit > 0.0 { c[i] * (1.0 + profit / 100.0) } else { 99_999_999.0 };
                    pos = true;
                    trendj[i] = 0;
                    evs.push(Ev { dir: 1.0, e: i, epx: c[i], exit: None });
                } else if pos && (fs[i] || h[i] > pb[i - 1] || i == n - 1) {
                    let (px, r) = if o[i] > pb[i - 1] {
                        (o[i] - o[i] * 0.0025, "Profit booking (gap)")
                    } else if h[i] > pb[i - 1] {
                        (pb[i - 1], "Profit booking")
                    } else if fs[i] {
                        (c[i], "ST down")
                    } else {
                        (c[i], "Last bar")
                    };
                    if let Some(v) = evs.last_mut() {
                        v.exit = Some((i, bound(x, i, px), i, r));
                    }
                    pos = false;
                } else {
                    pb[i] = pb[i - 1];
                }
            }
            collect(x, evs, from, to)
        }
        _ => return None,
    })
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let name = catalogue().into_iter().find(|d| d.id == id)?.name;
    Some((match id {
        SWING_EXIT => "ytwh_swing_exit",
        ST_PB => "ytwh_st_profit_booking",
        _ => return None,
    }, name, "52 Week High"))
}

/// Synthetic daily bars for the yt_wh / yt_at / yt_vt / yt_pin tests.
#[cfg(test)]
pub(crate) mod testbars {
    use crate::books::ytmr::day_number;
    use crate::connors::{Ctx, Params, Trade};
    use crate::Row;

    pub fn civil(z: i64) -> String {
        let z = z + 719468;
        let era = z.div_euclid(146097);
        let doe = z - era * 146097;
        let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let d = doy - (153 * mp + 2) / 5 + 1;
        let m = if mp < 10 { mp + 3 } else { mp - 9 };
        format!("{:04}-{:02}-{:02}", yoe + era * 400 + i64::from(m <= 2), m, d)
    }
    /// Consecutive weekdays from Monday 2024-01-01; each bar is (open, high, low, close).
    pub fn bars(ohlc: &[(f64, f64, f64, f64)]) -> Vec<Row> {
        let mut d = day_number("2024-01-01").unwrap();
        ohlc.iter()
            .map(|&(open, high, low, close)| {
                let z = d;
                d += if (z + 4).rem_euclid(7) == 5 { 3 } else { 1 };
                Row { ticker: "T".into(), date: civil(z), open, high, low, close, volume: None }
            })
            .collect()
    }
    /// A wavy series of `n` bars with ranges around the close.
    pub fn wave(n: usize) -> Vec<Row> {
        let v: Vec<(f64, f64, f64, f64)> = (0..n)
            .map(|k| {
                let k = k as f64;
                let c = 100.0 + 15.0 * (k / 9.0).sin() + 6.0 * (k / 23.0).cos() + k * 0.05;
                let o = c - 1.5 * (k / 3.0).sin();
                (o, o.max(c) + 1.0 + (k / 5.0).sin().abs(), o.min(c) - 1.0 - (k / 7.0).cos().abs(), c)
            })
            .collect();
        bars(&v)
    }
    pub fn ps(kv: &[(&str, f64)]) -> Params {
        kv.iter().map(|(k, v)| (k.to_string(), *v)).collect()
    }
    pub fn go(run: fn(&str, &Ctx, usize, usize, &Params) -> Option<Vec<Trade>>, id: &str, r: &[Row], kv: &[(&str, f64)]) -> Vec<(usize, f64, usize, f64, bool)> {
        let x = Ctx::new(r, r, &[]);
        run(id, &x, 0, r.len() - 1, &ps(kv)).unwrap().into_iter().map(|t| (t.legs[0].i, t.legs[0].px, t.exit_i, t.exit_px, t.open)).collect()
    }
    pub fn smoke(run: fn(&str, &Ctx, usize, usize, &Params) -> Option<Vec<Trade>>, ids: &[&str]) {
        let r = wave(400);
        let x = Ctx::new(&r, &r, &[]);
        for id in ids {
            for (from, to) in [(0, r.len() - 1), (50, 300)] {
                let t = run(id, &x, from, to, &Params::new()).unwrap();
                for t in &t {
                    assert!(t.legs[0].i >= from && t.legs[0].i <= to && t.exit_i >= t.legs[0].i && t.exit_i <= to, "{id}");
                    assert!(t.exit_px.is_finite() && t.exit_px >= x.l[t.exit_i] - 1e-9 && t.exit_px <= x.h[t.exit_i] + 1e-9, "{id}");
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::testbars::*;
    use super::*;

    fn flat(c: f64) -> (f64, f64, f64, f64) {
        (c, c + 1.0, c - 1.0, c)
    }

    #[test]
    fn swing_exit_buys_above_previous_weeks_high_close() {
        // 3 weeks at 100 (week 0 and 1), week 2 Monday closes 105 > highest weekly close of week 0/1 -> buy;
        // then a fall of more than 10% below the swing band sells
        let mut v: Vec<(f64, f64, f64, f64)> = (0..10).map(|_| flat(100.0)).collect();
        v.extend([flat(100.0), flat(105.0), flat(106.0), flat(107.0), flat(90.0), flat(91.0)]);
        let r = bars(&v);
        let t = go(run, SWING_EXIT, &r, &[]);
        // bar 10 is Monday of week 2: its level is week 0's HWC (100, lag one week before the last day) -> no buy at 100
        assert_eq!(t[0].0, 11);
        assert_eq!(t[0].1, 105.0);
        assert_eq!((t[0].2, t[0].3, t[0].4), (14, 90.0, false));
        // a 5% profit stop books at 105 x 1.05 = 110.25 if the high reaches it
        let mut v2 = v.clone();
        v2[13] = (107.0, 111.0, 106.0, 107.0);
        let t = go(run, SWING_EXIT, &bars(&v2), &[("pb_on", 1.0), ("pb_pct", 5.0)]);
        assert_eq!((t[0].2, t[0].3), (13, 105.0 * 1.05));
        // a 3% hard stop gapped through fills at the open
        let mut v3 = v.clone();
        v3[12] = (100.0, 100.5, 99.0, 100.0);
        let t = go(run, SWING_EXIT, &bars(&v3), &[("hard_stop", 3.0)]);
        assert_eq!((t[0].2, t[0].3), (12, 100.0));
    }

    #[test]
    fn st_profit_booking_needs_rearm() {
        // ST period 2 factor 1 on the close: a down move then a turn up arms the flag; a close above the 260-bar high buys
        let mut v: Vec<(f64, f64, f64, f64)> = vec![flat(100.0), flat(100.0), flat(100.0), flat(90.0), flat(85.0), flat(84.0)];
        v.extend([flat(95.0), flat(101.0), (101.0, 104.0, 100.0, 102.0)]);
        let r = bars(&v);
        let x = Ctx::new(&r, &r, &[]);
        let m = st_filter(&x, 'D', 2, 2, 1.0);
        assert_eq!(m[3], -1);
        assert_eq!(m[6], 1);
        let t = go(run, ST_PB, &r, &[("st_point", 2.0), ("st_period", 2.0)]);
        // armed at bar 6 (ST turns up) but 95 < 100; bar 7 closes 101 > 100 -> buy; bar 8 high 104 > 101 x 1.01 -> book
        assert_eq!((t[0].0, t[0].1), (7, 101.0));
        assert_eq!(t[0].2, 8);
        assert!((t[0].3 - 101.0 * 1.01).abs() < 1e-9);
        // no rearm without a down-to-up turn: a rising series never buys
        let up: Vec<_> = (0..40).map(|k| flat(100.0 + k as f64)).collect();
        assert!(go(run, ST_PB, &bars(&up), &[]).is_empty());
    }

    #[test]
    fn every_strategy_runs() {
        smoke(run, &IDS);
    }
}
