//! YT Strategies · Gap Down: the team's AmiBroker AFLs in "Old Topics/Gap Down" that trade individual stocks on
//! daily bars. Coded from the AFLs as written, with the AFLs' own default settings; there is no book PDF and nothing
//! here is checked against the team's workbooks.
//!
//! Coded (one stock at a time, each trade opened and closed on the same daily bar, `AllowSameBarExit` on):
//! - INTRADAY GAP DOWN BUY.afl: the open is more than GAP % (1) below the previous bar's low (option: close); buy at
//!   the open + 0.25% and sell at that bar's close (`Sell = BUYSIG AND NOT LOCIRCUIT`, true on every entry bar);
//! - INTRADAY GAP DOWN BUY_With SMA & ROC filter.afl: the same with the previous close above its 200-day average;
//! - INTRADAY GAP DOWN SHORT WHEN PREVIOUS BULLISH CANDLE.afl (Shock system): after a bullish bar (close in the top
//!   20% of its range, open in the bottom 20%), short at the open - 0.25% when the open is below the previous low
//!   + 1%, with the stock's own monthly SuperTrend(20, 1, mid point) down; cover at the close (or at the optional
//!   trailing level, off by default) on the same bar (`Cover = ... OR shortsig`).
//!
//! AmiBroker behaviour followed (see `ytsw.rs` / `ytwh.rs` / `ytnr.rs`):
//! - `Ref(x, -1)` is Null on the first bar (no trade there); `MA(C, 200)` is Null until 200 bars exist, and the
//!   array comparison with it is false there;
//! - the lower-circuit test is `Prec(O,2) == Prec(C,2) AND Prec(O,2) == Prec(L,2)` on the entry bar itself and the
//!   penny filter is `C > 0` on the entry bar (both read that bar's close, as written);
//! - trade prices are kept inside the bar's low-high range (AmiBroker's price bound check);
//! - the optional `ApplyStop(stopTypeLoss, percent)` (default 0 = off) runs with `ActivateStopsImmediately`, so it
//!   is live on the entry bar and is taken before the close exit, at entry x (1 - SL%) (kept within the bar);
//! - the monthly SuperTrend is `books::ytsw::st_filter` (the same "Harshit" ST loop; ATR(1) Null on the first
//!   bar; `TimeFrameExpand` expandLast); the trend starts at 1 (up), so no short until the monthly bands exist;
//! - the turnover filter (off) is close x volume against the AFL's year table; a missing volume fails it.
//! Not modelled: PositionScore ranking (`Ref(100 - ROC(C, 1), -1)` in the SMA AFL), Maximum Open Positions (20) and
//! position sizing, which only choose among stocks.
//!
//! Not coded:
//! - Gap down strategy/GAP DOWN_Intra day time frame/Intraday GapDown Buy with Time Condition.afl and the four
//!   "Intraday GapDown Buy with Single OR Double Confirmation and Time Condition Exit" AFLs ('+' Normal ROC / '-'
//!   Nick Radge ROC, with and without filter): 1-minute bars (entry at the 9:20 bar's open, confirmation candles,
//!   exit at 15:20); the engine has daily bars only;
//! - Shock system/INTRADAY/GAP DOWN SHORT WHEN PREVIOUS BULLISH CANDLE with intraday trailing sl.afl: intraday bars
//!   (first-bar entry, trailing SL from the 2nd candle's low, 15:15 / 15:25 exits);
//! - Single and Double Confirmation - FO Intraday Candle/Intraday GapDown Buy ... ('-' Nick Radge ROC).afl at the
//!   folder root: only the standard "Price" chart section (no trading rules).
#![allow(dead_code)]
use crate::books::ytd7::{ma, prec2};
use crate::books::ytsw::{st_filter, tf_of};
use crate::books::ytwh::bound;
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "yt_gd";
pub const BUY: &str = "ytgd_buy";
pub const BUY_SMA: &str = "ytgd_buy_sma";
pub const SHOCK: &str = "ytgd_shock_short";
pub const IDS: [&str; 3] = [BUY, BUY_SMA, SHOCK];

fn def(id: &'static str, name: &'static str, dir: &'static str, rules: &'static [&'static str], notes: &'static str) -> StrategyDef {
    StrategyDef { id, name, group: "YT Strategies · Gap Down", session: 0, dir, status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim: None }
}

pub fn catalogue() -> Vec<StrategyDef> {
    vec![
        def(BUY, "Intraday Gap Down Buy", "Long", &[
            "F|Open < previous Low − gap % (1) of it (gap calculation option: previous Close)",
            "F|Entry bar is not a lower circuit (Open = Close = Low to 2 decimals; option, on)",
            "F|Entry bar Close > penny price (0); optional turnover filter (off)",
            "E|Buy at the Open + 0.25% (kept within the bar)",
            "X|Optional hard stop % (off), live on the entry bar: at entry × (1 − SL%)",
            "X|Sell at the same bar's Close",
        ], concat!("INTRADAY GAP DOWN BUY.afl (folder root), coded as the AFL is written. Same rules: Gap down strategy/INTRADAY GAP DOWN BUY (No filter).afl and GAP DOWN_With & without filter/INTRADAY GAP DOWN BUY (Wo filter).afl (Param instead of Optimize, same defaults); FO Intraday - Without Filter/INTRADAY GAP DOWN BUY (No filter).afl comments out the lower-circuit test (parameter lower-circuit filter = 0).",
            " Runs on one stock at a time; the team ran it over stock lists (F&O stocks); Maximum Open Positions (20) and position sizing are not modelled. The lower-circuit and penny tests read the entry bar's own close, as written. Prices are kept within the bar's range.")),
        def(BUY_SMA, "Intraday Gap Down Buy, 200-day SMA Filter", "Long", &[
            "F|Previous Close > its 200-day SMA (as of the previous bar)",
            "F|Open < previous Low − gap % (1) of it (gap calculation option: previous Close)",
            "F|Entry bar is not a lower circuit (Open = Close = Low to 2 decimals; option, on)",
            "F|Entry bar Close > penny price (0); optional turnover filter (off)",
            "E|Buy at the Open + 0.25% (kept within the bar)",
            "X|Optional hard stop % (off), live on the entry bar: at entry × (1 − SL%)",
            "X|Sell at the same bar's Close",
        ], concat!("Gap down strategy/INTRADAY GAP DOWN BUY_With SMA & ROC filter.afl, coded as the AFL is written (same rules: the copy in GAP DOWN_With & without filter, Param instead of Optimize). The ROC(1) only feeds PositionScore (100 - ROC of the previous bar), which ranks stocks and is not modelled.",
            " Runs on one stock at a time; the team ran it over stock lists (F&O stocks); Maximum Open Positions (20) and position sizing are not modelled. The lower-circuit and penny tests read the entry bar's own close, as written. Prices are kept within the bar's range.")),
        def(SHOCK, "Gap Down Short after a Bullish Candle (Shock System)", "Short", &[
            "F|Previous bar bullish: Close > High − 20% of its range and Open < Low + 20% of its range",
            "F|Open < previous Low + gap range % (1) of it (gap calculation option: previous Close)",
            "F|The stock's own monthly SuperTrend (period 20, factor 1, mid point) is down (filter option: none)",
            "E|Short at the Open − 0.25% (kept within the bar)",
            "X|Cover at the same bar's Close",
            "X|Optional trailing SL % (0 = off): High ≥ previous Low + SL% covers at that level",
        ], concat!("INTRADAY GAP DOWN SHORT WHEN PREVIOUS BULLISH CANDLE (Shock system)/INTRADAY GAP DOWN SHORT WHEN PREVIOUS BULLISH CANDLE.afl, coded as the AFL is written on daily bars. The trailing SL % default 0 lies below its declared range 2-6 and is used as written (off). The gap test uses + 1% (the 'GAP RANGE PERCENTAGE' default) above the previous low, as written. The SuperTrend runs on the traded stock itself (no SetForeign).",
            " Runs on one stock at a time; Maximum Open Positions (20) and position sizing are not modelled. Prices are kept within the bar's range.")),
    ]
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    match id {
        BUY | BUY_SMA => {
            let mut v = vec![
                p("gap_calc", "Gap calculation (0 previous Low, 1 previous Close)", 0.0, 0.0, 1.0, 1.0),
                p("gap_pct", "Gap percentage", 1.0, 1.0, if id == BUY { 3.0 } else { 7.0 }, 0.5),
                p("buy_buff", "Buy price buffer %", 0.25, 0.0, 5.0, 0.05),
                p("lc_flt", "Lower-circuit filter (0 off, 1 on)", 1.0, 0.0, 1.0, 1.0),
                p("hard_sl", "Hard stop loss % (0 off)", 0.0, 0.0, 5.0, 0.5),
                p("penny", "Penny stock price", 0.0, 0.0, 100.0, 1.0),
                p("turnover_flt", "Turnover filter (0 off, 1 on)", 0.0, 0.0, 1.0, 1.0),
            ];
            if id == BUY_SMA {
                v.insert(0, p("ma_period", "MA periods", 200.0, 10.0, 300.0, 1.0));
            }
            Some(v)
        }
        SHOCK => Some(vec![
            p("range_pct", "Candle range %", 20.0, 1.0, 100.0, 1.0),
            p("gap_calc", "Gap calculation (0 previous Low, 1 previous Close)", 0.0, 0.0, 1.0, 1.0),
            p("gap_pct", "Gap range percentage", 1.0, -2.0, 2.0, 1.0),
            p("trail_pct", "Trailing SL % (0 off)", 0.0, 0.0, 6.0, 1.0),
            p("short_buff", "Short price buffer %", 0.25, 0.0, 2.0, 0.05),
            p("st_flt", "Filter (1 SuperTrend, 0 none)", 1.0, 0.0, 1.0, 1.0),
            p("flt_tf", "Filter time frame (1 monthly, 2 weekly, 3 daily)", 1.0, 1.0, 3.0, 1.0),
            p("st_point", "ST calculation point (1 mid point, 2 close, 3 pivot)", 1.0, 1.0, 3.0, 1.0),
            p("st_period", "Filter ST period", 20.0, 1.0, 20.0, 1.0),
            p("st_factor", "Filter ST factor", 1.0, 0.0, 20.0, 1.0),
        ]),
        _ => None,
    }
}

/// The AFLs' turnover thresholds by calendar year.
fn volsig(date: &str) -> f64 {
    let y: i64 = date.get(..4).and_then(|s| s.parse().ok()).unwrap_or(0);
    match y {
        ..=1999 => 30_000.0,
        2000..=2004 => 50_000.0,
        2005..=2009 => 1_800_000.0,
        2010..=2014 => 2_600_000.0,
        2015..=2019 => 6_000_000.0,
        _ => 13_000_000.0,
    }
}

fn trade(dir: f64, i: usize, px: f64, exit_px: f64, why: &str) -> Trade {
    Trade { dir, legs: vec![Leg { i, px, w: 1.0 }], exit_i: i, exit_px, reason: why.into(), entry_kind: "open", signal_i: i, open: false, exits: Vec::new() }
}

const BIG: f64 = 9_999_999.0;

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(0.0, |d| d.def));
    let on = |k: &str| g(k).round() as i64 == 1;
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let n = c.len();
    if n == 0 || from > to || to >= n {
        return Some(Vec::new());
    }
    let cal: &[f64] = if on("gap_calc") { c } else { l };
    let gap = g("gap_pct") / 100.0;
    let mut out = Vec::new();
    if id == SHOCK {
        let flt: Vec<bool> = if on("st_flt") {
            let tf = tf_of(g("flt_tf").round() as i64, true);
            st_filter(x, tf, g("st_point").round() as i64, g("st_period").round().max(1.0) as usize, g("st_factor")).into_iter().map(|t| t == -1).collect()
        } else {
            vec![true; n]
        };
        let hr = g("range_pct") / 100.0;
        let bull = |j: usize| { let r20 = (h[j] - l[j]) * hr; c[j] > h[j] - r20 && o[j] < l[j] + r20 };
        let (buff, trail_pct) = (g("short_buff") / 100.0, g("trail_pct"));
        for i in from.max(1)..=to {
            if !(bull(i - 1) && o[i] < cal[i - 1] + cal[i - 1] * gap && flt[i]) {
                continue;
            }
            let epx = bound(x, i, o[i] - o[i] * buff);
            // TRAILSL of the previous bar: off (9999999) at 0 %, else that (bullish) bar's low + SL%
            let trail = if trail_pct == 0.0 { BIG } else { l[i - 1] + l[i - 1] * trail_pct / 100.0 };
            let (px, why) = if h[i] >= trail { (trail, "Trailing SL") } else { (c[i], "Cover at close") };
            out.push(trade(-1.0, i, epx, bound(x, i, px), why));
        }
        return Some(out);
    }
    let ma_f: Option<Vec<Option<f64>>> = (id == BUY_SMA).then(|| ma(c, g("ma_period").round().max(1.0) as usize));
    let (buff, hs, penny) = (g("buy_buff") / 100.0, g("hard_sl"), g("penny"));
    let (lc_flt, tflt) = (on("lc_flt"), on("turnover_flt"));
    for i in from.max(1)..=to {
        let buysig = o[i] < cal[i - 1] - cal[i - 1] * gap;
        let locirc = lc_flt && prec2(o[i]) == prec2(c[i]) && prec2(o[i]) == prec2(l[i]);
        let ma_ok = ma_f.as_ref().map_or(true, |m| m[i - 1].map_or(false, |m| c[i - 1] > m));
        let turnover = !tflt || x.r[i].volume.unwrap_or(f64::NAN) * c[i] > volsig(&x.r[i].date);
        if !(buysig && ma_ok && !locirc && c[i] > penny && turnover) {
            continue;
        }
        let epx = bound(x, i, o[i] + o[i] * buff);
        let stop = epx * (1.0 - hs / 100.0);
        let (px, why) = if hs > 0.0 && l[i] <= stop { (stop, "Hard stop (ApplyStop)") } else { (c[i], "Sell at close") };
        out.push(trade(1.0, i, epx, bound(x, i, px), why));
    }
    Some(out)
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let d = catalogue().into_iter().find(|d| d.id == id)?;
    Some((d.id, d.name, "Gap Down"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::books::ytwh::testbars::*;
    use crate::Row;

    fn flat(n: usize) -> Vec<(f64, f64, f64, f64)> {
        (0..n).map(|_| (100.0, 101.0, 99.0, 100.5)).collect()
    }

    #[test]
    fn gap_down_buy_open_plus_buffer_sell_close() {
        let mut v = flat(5);
        v.push((97.0, 99.0, 96.5, 98.0)); // open 97 < 99 x 0.99 = 98.01
        v.push((98.5, 99.0, 98.0, 98.8)); // 98.5 > 96.5 x 0.99: no trade
        let t = go(run, BUY, &bars(&v), &[]);
        assert_eq!(t.len(), 1);
        assert_eq!((t[0].0, t[0].2), (5, 5));
        assert!((t[0].1 - 97.0 * 1.0025).abs() < 1e-9);
        assert!((t[0].3 - 98.0).abs() < 1e-9);
        // open 98.2 is not 1% below the previous low (98.01) -> nothing
        let mut v = flat(5);
        v.push((98.2, 99.0, 97.0, 98.0));
        assert!(go(run, BUY, &bars(&v), &[]).is_empty());
        // by the previous close (100.5 x 0.99 = 99.495) it is a gap
        assert_eq!(go(run, BUY, &bars(&v), &[("gap_calc", 1.0)]).len(), 1);
    }

    #[test]
    fn lower_circuit_and_hard_stop() {
        let mut v = flat(5);
        v.push((97.0, 98.0, 97.0, 97.0)); // open = low = close: lower circuit
        let r = bars(&v);
        assert!(go(run, BUY, &r, &[]).is_empty());
        assert_eq!(go(run, BUY, &r, &[("lc_flt", 0.0)]).len(), 1);
        let mut v = flat(5);
        v.push((97.0, 99.0, 95.0, 98.0));
        let t = go(run, BUY, &bars(&v), &[("hard_sl", 2.0)]);
        let epx = 97.0 * 1.0025;
        assert!((t[0].3 - epx * 0.98).abs() < 1e-9); // low 95 reaches the stop, taken before the close
        // a buy price above the high is kept at the high
        let mut v = flat(5);
        v.push((97.0, 97.1, 96.0, 96.5));
        assert!((go(run, BUY, &bars(&v), &[])[0].1 - 97.1).abs() < 1e-9);
    }

    #[test]
    fn sma_filter_and_turnover() {
        let mut v: Vec<(f64, f64, f64, f64)> = (0..10).map(|k| { let c = 100.0 + k as f64; (c, c + 1.0, c - 1.0, c) }).collect();
        v.push((95.0, 99.0, 94.0, 98.0));
        let r = bars(&v);
        // MA(200) Null with 10 bars: no trade; MA(10): previous close 109 > its average 104.5
        assert!(go(run, BUY_SMA, &r, &[]).is_empty());
        assert_eq!(go(run, BUY_SMA, &r, &[("ma_period", 10.0)]).len(), 1);
        assert_eq!(go(run, BUY, &r, &[]).len(), 1);
        // turnover filter: no volume fails it; enough volume passes (2024 threshold 13,000,000)
        assert!(go(run, BUY, &r, &[("turnover_flt", 1.0)]).is_empty());
        let rv: Vec<Row> = r.into_iter().map(|mut b| { b.volume = Some(200_000.0); b }).collect();
        assert_eq!(go(run, BUY, &rv, &[("turnover_flt", 1.0)]).len(), 1);
    }

    #[test]
    fn shock_short_after_bullish_bar() {
        let mut v = flat(5);
        v.push((100.0, 104.0, 99.8, 103.9)); // bullish: close in the top 20%, open in the bottom 20%
        v.push((100.5, 101.0, 99.0, 99.5)); // open 100.5 < 99.8 x 1.01 = 100.798
        let r = bars(&v);
        // with the default monthly ST filter there is no down trend in this short history: no short
        assert!(go(run, SHOCK, &r, &[]).is_empty());
        let t = go(run, SHOCK, &r, &[("st_flt", 0.0)]);
        assert_eq!(t.len(), 1);
        assert_eq!((t[0].0, t[0].2), (6, 6));
        assert!((t[0].1 - 100.5 * 0.9975).abs() < 1e-9);
        assert!((t[0].3 - 99.5).abs() < 1e-9);
        // trailing SL 1%: level 99.8 x 1.01 = 100.798; the high 101 reaches it -> covered at that level
        let t = go(run, SHOCK, &r, &[("st_flt", 0.0), ("trail_pct", 1.0)]);
        assert!((t[0].3 - 99.8 * 1.01).abs() < 1e-9);
        // the open too high (101 > 100.798): no short; a non-bullish previous bar: no short
        let mut v2 = v.clone();
        v2[6].0 = 101.0;
        assert!(go(run, SHOCK, &bars(&v2), &[("st_flt", 0.0)]).is_empty());
        let mut v3 = v.clone();
        v3[5].3 = 102.0;
        assert!(go(run, SHOCK, &bars(&v3), &[("st_flt", 0.0)]).is_empty());
    }

    #[test]
    fn shock_filter_needs_st_down() {
        // a long decline makes the daily ST(20, 1) point down; the bullish bar and gap then short
        let mut v: Vec<(f64, f64, f64, f64)> = (0..60).map(|k| { let c = 200.0 - 3.0 * k as f64; (c + 0.5, c + 1.0, c - 1.0, c - 0.9) }).collect();
        v.push((19.2, 20.5, 19.0, 20.4)); // bullish bar
        v.push((19.1, 19.5, 17.0, 18.0)); // open 19.1 < 19.0 x 1.01
        let r = bars(&v);
        let t = go(run, SHOCK, &r, &[("flt_tf", 3.0)]);
        assert_eq!(t.len(), 1);
        assert_eq!(t[0].0, 61);
        // in an up trend (rising bars) the same pattern is filtered out
        let mut u: Vec<(f64, f64, f64, f64)> = (0..60).map(|k| { let c = 20.0 + 3.0 * k as f64; (c - 0.5, c + 1.0, c - 1.0, c + 0.9) }).collect();
        u.push((199.2, 200.5, 199.0, 200.4));
        u.push((199.1, 199.5, 197.0, 198.0));
        assert!(go(run, SHOCK, &bars(&u), &[("flt_tf", 3.0)]).is_empty());
        assert_eq!(go(run, SHOCK, &bars(&u), &[("st_flt", 0.0)]).len(), 1);
    }

    #[test]
    fn every_strategy_runs() {
        smoke(run, &IDS);
        assert_eq!(catalogue().len(), IDS.len());
        for id in IDS {
            assert!(params(id).is_some() && family_of(id).is_some());
        }
        assert!(run("ytgd_none", &Ctx::new(&bars(&flat(3)), &bars(&flat(3)), &[]), 0, 2, &Params::new()).is_none());
    }
}
