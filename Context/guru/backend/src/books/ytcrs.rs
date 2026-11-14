//! YT Strategies · Consecutive Red in 5254 Empty Space: the team's AmiBroker AFL in "Old Topics/Consecutive Red in
//! 5254 Empty Space" (AFLs/5-2 5-4 Sell Red Candles.afl), run on an index on daily bars. Coded from the AFL as
//! written, with its own default settings; there is no book PDF.
//!
//! Despite its file name the AFL is long only: it BUYS after N consecutive red candles while the close is below
//! both of its daily Supertrends (both "in a sell signal"), and sells on the Nth exit candle or when the close
//! crosses above the first Supertrend.
//!
//! AmiBroker behaviour followed (as in `ytroc.rs`, whose Supertrend this AFL shares):
//! - the team's Supertrend: ATRVal = sum of the last `period` ATR(1) values / period (ATR(1) is Null on the first
//!   bar), Up = avg + factor x ATRVal, Dn = avg - factor x ATRVal, IsUp starts true, from bar `period`: up when
//!   C > Up[i-1], down when C < Dn[i-1]; in an up trend Dn only rises, in a down trend Up only falls; ST = IsUp ? Dn : Up;
//! - SetBarsRequired(100000, 0): the loops run over the whole history; trades are taken only in the test window;
//! - fills at the bar's close; the loop sells first and may buy again on the same bar; no ExRem;
//! - a comparison with a Null Supertrend (its first bars) is false;
//! - the optional ST filter on a weekly / monthly timeframe uses the AFL's Ref(-1) / expandFirst / Ref(+1) pattern
//!   (each daily bar sees the previous period's value, the period's last bar its own; Null on the data's last bar).
#![allow(dead_code)]
use crate::books::ytroc::{avg_of, backtest, gt, lt, st_daily, team_st, Px};
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "yt_crs";
pub const RED: &str = "ytcrs_red_below_two_st";
pub const IDS: [&str; 1] = [RED];

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef {
        id: RED,
        name: "N Red Candles Below Two Supertrends (5-2 5-4)",
        group: "YT Strategies · Consecutive Red in 5254 Empty Space",
        session: 0,
        dir: "Long",
        status: Status::Live,
        needs_volume: false,
        needs_vix: false,
        needs_series: &[],
        book: BOOK,
        rules: &[
            "F|ST 1 = daily Supertrend(5, factor 1) and ST 2 = daily Supertrend(5, factor 2), both on (High + Low) / 2",
            "F|N consecutive red candles (Open > Close; option: down closes), N = 1 by default",
            "E|Buy at the Close when the red count is at least N and the Close is below both ST 1 and ST 2",
            "X|Exit candle count: every bar in the trade (options: green candles, up closes); 0 = off (the default)",
            "X|Sell at the Close when the count reaches N exit candles, or when the Close crosses above ST 1 (below it yesterday, above it today)",
            "X|The loop sells first and can buy again on the same bar",
            "F|Optional ST filter (off by default): buy only in its bullish (option: bearish) phase, sell when the phase ends",
        ],
        notes: "AFLs/5-2 5-4 Sell Red Candles.afl, coded as the AFL is written. The AFL is long only (Buy / Sell) although its file name says sell. Defaults as written: 'Number of Red Consecutive Candle' 1 lies below its declared minimum 2 and 'N Candles (Keep 0 for OFF)' 0 below its minimum 1; both are used as written, so at the defaults one red candle below both Supertrends buys and the only exit is the close crossing above ST 1. The two Supertrends default to factors 1 and 2 (period 5, (High+Low)/2), not the 5-2 / 5-4 of the folder name. The ST filter (Daily by default) is off by default. Position size settings are the stock template's; fills at the close.",
        claim: None,
    }]
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    if id != RED {
        return None;
    }
    const AVG: &str = "ST avg (0 Close, 1 Open, 2 High, 3 Low, 4 (O+C)/2, 5 (H+L)/2, 6 (H+L+C)/3, 7 (O+H+L+C)/4)";
    Some(vec![
        p("entry_type", "Entry candle (0 red candle, 1 down close)", 0.0, 0.0, 1.0, 1.0),
        p("num_red", "Number of consecutive red candles", 1.0, 1.0, 10.0, 1.0),
        p("st1_period", "Supertrend 1 period", 5.0, 1.0, 50.0, 1.0),
        p("st1_factor", "Supertrend 1 factor", 1.0, 0.0, 10.0, 0.5),
        p("st1_avg", AVG, 5.0, 0.0, 7.0, 1.0),
        p("st2_period", "Supertrend 2 period", 5.0, 1.0, 50.0, 1.0),
        p("st2_factor", "Supertrend 2 factor", 2.0, 0.0, 10.0, 0.5),
        p("st2_avg", AVG, 5.0, 0.0, 7.0, 1.0),
        p("exit_type", "Exit candle (0 any candle, 1 green candle, 2 up close)", 0.0, 0.0, 2.0, 1.0),
        p("exit_num", "N exit candles (0 off)", 0.0, 0.0, 10.0, 1.0),
        p("flt_on", "ST filter (0 off, 1 on)", 0.0, 0.0, 1.0, 1.0),
        p("flt_tf", "ST filter timeframe (0 daily, 1 weekly, 2 monthly)", 0.0, 0.0, 2.0, 1.0),
        p("flt_period", "ST filter period", 5.0, 1.0, 50.0, 1.0),
        p("flt_factor", "ST filter factor", 1.0, 0.0, 10.0, 0.5),
        p("flt_avg", AVG, 5.0, 0.0, 7.0, 1.0),
        p("flt_side", "ST filter entry phase (0 bullish, 1 bearish)", 0.0, 0.0, 1.0, 1.0),
    ])
}

/// The AFL's two loops: the red-candle signal and the position loop; Buy / Sell arrays at the close.
fn signals(x: &Ctx, g: &dyn Fn(&str) -> f64) -> Px {
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let n = c.len();
    let gi = |k: &str| g(k).round() as i64;
    let gu = |k: &str| g(k).round().max(1.0) as usize;
    let st1 = team_st(&avg_of(o, h, l, c, gi("st1_avg")), h, l, c, gu("st1_period"), g("st1_factor"));
    let st2 = team_st(&avg_of(o, h, l, c, gi("st2_avg")), h, l, c, gu("st2_period"), g("st2_factor"));
    let down_close = gi("entry_type") == 1;
    let num_red = g("num_red");
    let mut sig = vec![false; n];
    let mut count_red = 0.0;
    for i in 0..n {
        let red = if down_close { i > 0 && c[i] < c[i - 1] } else { o[i] > c[i] };
        count_red = if red { count_red + 1.0 } else { 0.0 };
        sig[i] = count_red >= num_red && lt(Some(c[i]), st1[i]) && lt(Some(c[i]), st2[i]);
    }
    let flt_on = gi("flt_on") == 1;
    let bull: Vec<bool> = if flt_on {
        let (st, cm) = st_daily(x, gi("flt_tf"), gu("flt_period"), g("flt_factor"), gi("flt_avg"));
        (0..n).map(|i| gt(cm[i], st[i])).collect()
    } else {
        vec![false; n]
    };
    let bearish = gi("flt_side") == 1;
    let entry: Vec<bool> = (0..n).map(|i| sig[i] && (!flt_on || if bearish { !bull[i] } else { bull[i] })).collect();
    let etype = gi("exit_type");
    let exit_num = g("exit_num");

    let mut s = Px::new(n);
    let mut pos = false;
    let mut count = 0.0;
    for i in 1..n {
        if pos {
            match etype {
                1 => {
                    if o[i] < c[i] {
                        count += 1.0
                    }
                }
                2 => {
                    if c[i] > c[i - 1] {
                        count += 1.0
                    }
                }
                _ => count += 1.0,
            }
            let st_buy = lt(Some(c[i - 1]), st1[i - 1]) && gt(Some(c[i]), st1[i]);
            if st_buy {
                count = exit_num;
            }
            let flt_exit = flt_on && if bearish { bull[i] } else { !bull[i] };
            if count == exit_num || flt_exit {
                let why = if st_buy {
                    "Close crossed above ST 1"
                } else if count == exit_num {
                    "N exit candles"
                } else {
                    "ST filter phase ended"
                };
                s.sell(i, c[i], why);
                pos = false;
            }
        }
        if !pos && entry[i] {
            s.buy[i] = true;
            pos = true;
            count = 0.0;
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
    Some(backtest(x, &signals(x, &g), from, to))
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let d = catalogue().into_iter().find(|d| d.id == id)?;
    Some((d.id, d.name, "Consecutive Red in 5254 Empty Space"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::books::ytwh::testbars::*;

    /// A steady rise (ST in an up trend, close above both lines), then a sharp fall that puts the close under both
    /// Supertrends on red candles, then a rebound above ST 1.
    fn series() -> Vec<(f64, f64, f64, f64)> {
        let mut v: Vec<(f64, f64, f64, f64)> = (0..12).map(|k| { let c = 100.0 + k as f64; (c - 0.5, c + 0.5, c - 1.0, c) }).collect();
        // bars 12..14: red candles falling hard
        v.push((110.0, 110.5, 104.0, 104.5));
        v.push((104.0, 104.2, 99.0, 99.5));
        v.push((99.0, 99.2, 95.0, 95.5));
        // bars 15..: rebound on green candles
        v.push((95.5, 100.0, 95.0, 99.8));
        v.push((100.0, 106.0, 99.5, 105.5));
        v.push((105.5, 112.0, 105.0, 111.5));
        v
    }

    fn lines(r: &[crate::Row]) -> (Vec<Option<f64>>, Vec<Option<f64>>, Vec<f64>) {
        let x = Ctx::new(r, r, &[]);
        let st1 = team_st(&avg_of(&x.o, &x.h, &x.l, &x.c, 5), &x.h, &x.l, &x.c, 5, 1.0);
        let st2 = team_st(&avg_of(&x.o, &x.h, &x.l, &x.c, 5), &x.h, &x.l, &x.c, 5, 2.0);
        (st1, st2, x.c.clone())
    }

    #[test]
    fn buys_red_below_both_sts_and_sells_on_st1_cross() {
        let r = bars(&series());
        let (st1, st2, c) = lines(&r);
        let t = go(run, RED, &r, &[]);
        assert!(!t.is_empty());
        let (e, _, x, _, _) = t[0];
        // the entry: red candle, close below both lines, first such bar
        assert!(r[e].open > r[e].close);
        assert!(c[e] < st1[e].unwrap() && c[e] < st2[e].unwrap());
        assert!((e..r.len()).len() > 0 && (12..e).all(|j| !(c[j] < st1[j].unwrap_or(-1e10) && c[j] < st2[j].unwrap_or(-1e10) && r[j].open > r[j].close)));
        // the exit: close below ST 1 yesterday and above it today
        assert!(c[x - 1] < st1[x - 1].unwrap() && c[x] > st1[x].unwrap());
        assert_eq!(t[0].3, c[x]);
    }

    #[test]
    fn exit_after_n_candles() {
        let r = bars(&series());
        let t = go(run, RED, &r, &[]);
        let e = t[0].0;
        // one exit candle: sells on the bar after the entry (unless the ST 1 cross comes first, which it cannot on
        // that bar: the close is still falling)
        let t1 = go(run, RED, &r, &[("exit_num", 1.0)]);
        assert_eq!((t1[0].0, t1[0].2), (e, e + 1));
        // two red candles in a row needed: the entry moves at least one bar later
        let t2 = go(run, RED, &r, &[("num_red", 2.0)]);
        assert!(t2[0].0 > e && r[t2[0].0 - 1].open > r[t2[0].0 - 1].close);
    }

    #[test]
    fn every_strategy_runs() {
        smoke(run, &IDS);
        let r = wave(400);
        let x = Ctx::new(&r, &r, &[]);
        for kv in [vec![("flt_on", 1.0)], vec![("flt_on", 1.0), ("flt_tf", 2.0), ("flt_side", 1.0)], vec![("entry_type", 1.0), ("exit_type", 1.0), ("exit_num", 3.0)]] {
            let t = run(RED, &x, 0, r.len() - 1, &ps(&kv)).unwrap();
            assert!(t.iter().all(|t| t.dir > 0.0 && t.exit_i >= t.legs[0].i));
        }
    }
}
