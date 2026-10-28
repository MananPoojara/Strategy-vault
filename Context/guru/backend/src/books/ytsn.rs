//! YT Strategies · Sniper System: the team's AmiBroker AFL "Old Topics/Sniper System/New Optims/SNIPER.afl", long only
//! on daily bars of an index. Coded from the AFL as written, with its own default settings; there is no book PDF.
//!
//! AmiBroker behaviour as in `books::ytsw` (fills at the close unless a live HSL / PB price applies, exit then entry
//! on one bar, ATR(1) Null on the first bar). Specific to this AFL:
//! - its Supertrend: band ATR = mean of the last `period` ATR(1) values; the trend loop starts at bar `period`
//!   (earlier bars are up), and a band only ratchets (no reset on a flip);
//! - a weekly / monthly ST is computed on compressed bars, shifted back one period, expanded with expandFirst and
//!   then shifted FORWARD one daily bar (`Ref(x, 1)`): each day of a period sees the previous completed period,
//!   except the period's last trading day, which already sees its own period (known at that close; the forward
//!   shift reads the next bar's date), and the last bar of the data gets Null;
//! - Null read in the bar loop is AmiBroker's -1e10.
#![allow(dead_code)]
use crate::books::ytsw::{compress, cross_up, trade};
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "yt_sn";
pub const SNIPER: &str = "ytsn_sniper";
pub const IDS: [&str; 1] = [SNIPER];

const NULL: f64 = -1e10;

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef {
        id: SNIPER,
        name: "Sniper (Monthly ST Cross)",
        group: "YT Strategies · Sniper System",
        session: 0,
        dir: "Long",
        status: Status::Live,
        needs_volume: false,
        needs_vix: false,
        needs_series: &[],
        book: BOOK,
        rules: &[
            "F|Monthly Supertrend (period 5, factor 2, on (H+L)/2) of the traded index; until a month's last trading day the daily bars see the previous month's ST",
            "E|Buy at the close when the daily close crosses above the monthly ST line while the last month closed below its ST (default: daily cross; options: wait for the month end, or only the monthly close crossing the ST)",
            "E|No entry on a bar whose high equals its close, or on the last bar of the data",
            "X|Exits, each off by default: % swing below the highest close since entry, hard stop %, profit booking %, a number of bars, or the daily ST pair (5, 1) and (5, 2) turning down",
            "X|With every exit off (the AFL's defaults) the position is held to the end of the data",
        ],
        notes: "SNIPER.afl (Sniper System / New Optims), coded as the AFL is written. On the daily entry time frame the entry can never fire (Cross(C, ST) and C < ST on the same bar), as written. ParamList defaults are list indices: entry ST time frame 12 = Monthly, exit ST time frames 10 = Daily, ST average 5 = (High+Low)/2, entry type 0 = Daily Cross. The time-frame list's 'Default' and minute entries are not matched by the AFL's getTimeframe, so on daily data they are the daily chart (option 0 here). The higher-time-frame lines are shifted forward one bar with Ref(x, 1), so on a period's last trading day they already use that period's close (look-ahead of the next bar's date only, as in the AFL). HSL % and PB % default 0 (below their declared minimum 5), % swing 0 and number of days 0 (the counter is tested after it is incremented, so 0 never fires), ST exit off: with the defaults the AFL only exits on the last bar, shown here as an open position. The turnover filter uses open interest and is off by default; penny filter 0. The <parmar/...> includes are not available and do not drive the signals.",
        claim: None,
    }]
}

const AVG: &str = "ST average value (0 close, 1 open, 2 high, 3 low, 4 (O+C)/2, 5 (H+L)/2, 6 (H+L+C)/3, 7 (O+H+L+C)/4)";

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    if id != SNIPER {
        return None;
    }
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    Some(vec![
        p("entry_tf", "Entry ST time frame (0 daily: the AFL's Default / minute / Daily choices on daily data, 1 weekly, 2 monthly)", 2.0, 0.0, 2.0, 1.0),
        p("entry_period", "Entry Supertrend period", 5.0, 1.0, 100.0, 1.0),
        p("entry_avg", AVG, 5.0, 0.0, 7.0, 1.0),
        p("entry_factor", "Entry ST factor", 2.0, 1.0, 50.0, 0.5),
        p("entry_type", "Take entry at (0 daily cross, 1 daily cross with month end, 2 only month end)", 0.0, 0.0, 2.0, 1.0),
        p("exit_bars", "Number of days for exit (0 off)", 0.0, 0.0, 500.0, 1.0),
        p("pct_swing", "% swing below the highest close since entry (0 off)", 0.0, 0.0, 26.0, 1.0),
        p("exit_live", "Exit using HSL / PB at (0 close, 1 live: the level, or the open if it gaps past)", 0.0, 0.0, 1.0, 1.0),
        p("hsl_pct", "Hard stop loss % (0 off)", 0.0, 0.0, 30.0, 5.0),
        p("pb_pct", "Profit booking % (0 off)", 0.0, 0.0, 30.0, 5.0),
        p("exit_buffer", "Exit buffer % on a gap past the HSL / PB level", 0.0, 0.0, 100.0, 0.01),
        p("st_exit", "ST exit (0 off, 1 on)", 0.0, 0.0, 1.0, 1.0),
        p("st1_tf", "Exit ST1 time frame (0 daily, 1 weekly, 2 monthly)", 0.0, 0.0, 2.0, 1.0),
        p("st1_period", "Exit Supertrend period 1", 5.0, 3.0, 9.0, 2.0),
        p("st1_factor", "Exit ST factor 1", 1.0, 1.0, 6.0, 1.0),
        p("st1_avg", AVG, 5.0, 0.0, 7.0, 1.0),
        p("st2_tf", "Exit ST2 time frame (0 daily, 1 weekly, 2 monthly)", 0.0, 0.0, 2.0, 1.0),
        p("st2_period", "Exit Supertrend period 2", 5.0, 3.0, 9.0, 2.0),
        p("st2_factor", "Exit ST factor 2", 2.0, 1.0, 6.0, 1.0),
        p("st2_avg", AVG, 5.0, 0.0, 7.0, 1.0),
        p("penny", "Penny price filter (buy only above this close)", 0.0, 0.0, 100.0, 1.0),
    ])
}

/// The AFL's Supertrend on bars (o, h, l, c): returns (ST line, close < line); None where the line is Null.
pub(crate) fn sniper_st(o: &[f64], h: &[f64], l: &[f64], c: &[f64], period: usize, factor: f64, avg: i64) -> (Vec<Option<f64>>, Vec<Option<bool>>) {
    let n = c.len();
    let p = period.max(1);
    let tr: Vec<Option<f64>> = (0..n).map(|i| if i == 0 { None } else { Some(h[i].max(c[i - 1]) - l[i].min(c[i - 1])) }).collect();
    let atr: Vec<Option<f64>> = (0..n)
        .map(|i| if i + 1 < p { None } else { tr[i + 1 - p..=i].iter().try_fold(0.0, |s, v| v.map(|v| s + v)).map(|s| s / p as f64) })
        .collect();
    let av = |i: usize| match avg {
        0 => c[i],
        1 => o[i],
        2 => h[i],
        3 => l[i],
        4 => (o[i] + c[i]) / 2.0,
        5 => (h[i] + l[i]) / 2.0,
        6 => (h[i] + l[i] + c[i]) / 3.0,
        7 => (o[i] + h[i] + l[i] + c[i]) / 4.0,
        _ => c[i],
    };
    let mut up: Vec<f64> = (0..n).map(|i| atr[i].map_or(NULL, |a| av(i) + a * factor)).collect();
    let mut dn: Vec<f64> = (0..n).map(|i| atr[i].map_or(NULL, |a| av(i) - a * factor)).collect();
    let mut is_up = vec![true; n];
    for i in p..n {
        is_up[i] = if c[i] > up[i - 1] { true } else if c[i] < dn[i - 1] { false } else { is_up[i - 1] };
        if is_up[i] {
            dn[i] = dn[i].max(dn[i - 1]);
        } else {
            up[i] = up[i].min(up[i - 1]);
        }
    }
    let line: Vec<Option<f64>> = (0..n).map(|i| { let v = if is_up[i] { dn[i] } else { up[i] }; if v == NULL { None } else { Some(v) } }).collect();
    let bear = (0..n).map(|i| line[i].map(|v| c[i] < v)).collect();
    (line, bear)
}

/// An ST on the chosen time frame as the daily bars see it: (line, bear signal, period close).
fn st_on(x: &Ctx, tf: i64, period: usize, factor: f64, avg: i64) -> (Vec<Option<f64>>, Vec<Option<bool>>, Vec<Option<f64>>) {
    let n = x.c.len();
    if tf != 1 && tf != 2 {
        let (line, bear) = sniper_st(&x.o, &x.h, &x.l, &x.c, period, factor, avg);
        return (line, bear, x.c.iter().map(|&v| Some(v)).collect());
    }
    let b = compress(x, if tf == 2 { 'M' } else { 'W' });
    let m = b.c.len();
    // open of each period = open of its first daily bar
    let mut o = vec![0.0; m];
    for i in (0..n).rev() {
        o[b.group[i]] = x.o[i];
    }
    let (line, bear) = sniper_st(&o, &b.h, &b.l, &b.c, period, factor, avg);
    // Ref(x, -1) on the periods, expandFirst, then Ref(x, 1) on the days: day i sees period group[i + 1] - 1
    let see = |i: usize| -> Option<usize> { if i + 1 >= n { None } else { b.group[i + 1].checked_sub(1) } };
    (
        (0..n).map(|i| see(i).and_then(|g| line[g])).collect(),
        (0..n).map(|i| see(i).and_then(|g| bear[g])).collect(),
        (0..n).map(|i| see(i).map(|g| b.c[g])).collect(),
    )
}

/// AFL Cross(a, b) with both possibly Null: no cross before both have values, Null as -1e10 after that.
fn cross_opt(a: &[Option<f64>], b: &[Option<f64>]) -> Vec<bool> {
    let first = (0..a.len()).find(|&i| a[i].is_some() && b[i].is_some()).unwrap_or(a.len());
    let v = |z: Option<f64>| z.unwrap_or(NULL);
    (0..a.len()).map(|i| i > first && v(a[i]) > v(b[i]) && v(a[i - 1]) <= v(b[i - 1])).collect()
}

/// LongEntry before the position loop.
pub(crate) fn long_entry(x: &Ctx, p: &dyn Fn(&str) -> f64) -> Vec<bool> {
    let n = x.c.len();
    let gi = |k: &str| p(k).round() as i64;
    let gu = |k: &str| p(k).round().max(1.0) as usize;
    let (line, bear, close_m) = st_on(x, gi("entry_tf"), gu("entry_period"), p("entry_factor"), gi("entry_avg"));
    let cr = cross_up(&x.c, &line);
    let monthly: Vec<bool> = (0..n).map(|i| cr[i] && bear[i] == Some(true)).collect();
    let month = |i: usize| x.r[i].date.get(5..7).and_then(|s| s.parse::<i64>().ok()).unwrap_or(0);
    let final_sig: Vec<bool> = match gi("entry_type") {
        1 => {
            let mut flag = false;
            (0..n)
                .map(|i| {
                    if monthly[i] {
                        flag = true;
                    }
                    let eom = i + 1 < n && month(i) != month(i + 1);
                    if eom && flag {
                        flag = false;
                        true
                    } else {
                        false
                    }
                })
                .collect()
        }
        2 => cross_opt(&close_m, &line),
        _ => monthly,
    };
    let penny = p("penny");
    (0..n).map(|i| i + 1 != n && x.h[i] != x.c[i] && x.c[i] > penny && final_sig[i]).collect()
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(0.0, |d| d.def));
    let n = x.c.len();
    if n == 0 || from > to || to >= n {
        return Some(Vec::new());
    }
    let gi = |k: &str| g(k).round() as i64;
    let gu = |k: &str| g(k).round().max(1.0) as usize;
    let entry = long_entry(x, &g);
    let (l1, _, c1) = st_on(x, gi("st1_tf"), gu("st1_period"), g("st1_factor"), gi("st1_avg"));
    let (l2, _, c2) = st_on(x, gi("st2_tf"), gu("st2_period"), g("st2_factor"), gi("st2_avg"));
    let v = |z: Option<f64>| z.unwrap_or(NULL);
    let (hsl_pct, pb_pct, live, buf) = (g("hsl_pct"), g("pb_pct"), gi("exit_live") == 1, g("exit_buffer"));
    let (exit_bars, swing, st_on_exit) = (gi("exit_bars"), g("pct_swing"), gi("st_exit") == 1);
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let mut out = Vec::new();
    // position: entry bar, HSL, PB, bar counter, ST exit flags, trailing high close
    let mut pos: Option<usize> = None;
    let (mut hsl, mut pb): (Option<f64>, Option<f64>) = (None, None);
    let (mut counter, mut st_exit, mut flag52, mut trail) = (0i64, false, false, 0.0f64);
    for i in from.max(1)..=to {
        if let Some(e) = pos {
            let sl_hit = hsl_pct > 0.0 && (if live { l[i] } else { c[i] }) < v(hsl);
            let pb_hit = pb_pct > 0.0 && (if live { h[i] } else { c[i] }) > v(pb);
            counter += 1;
            if v(c1[i]) < v(l1[i]) && !flag52 && v(c2[i]) < v(l2[i]) {
                st_exit = true;
            }
            if v(c2[i]) > v(l2[i]) && !flag52 {
                flag52 = true;
            }
            if v(c2[i]) < v(l2[i]) && flag52 {
                st_exit = true;
            }
            if c[i] > trail {
                trail = c[i];
            }
            let swing_hit = swing > 0.0 && c[i] < trail - trail * 0.01 * swing;
            let bars_hit = counter == exit_bars;
            let last = i + 1 == n;
            if pb_hit || sl_hit || last || bars_hit || (st_exit && st_on_exit) || swing_hit {
                let mut px = c[i];
                if pb_hit && live {
                    let lv = v(pb);
                    px = if o[i] > lv { l[i].max(o[i] * (1.0 - buf * 0.01)) } else { lv };
                }
                if sl_hit && live {
                    let lv = v(hsl);
                    px = if o[i] < lv { l[i].max(o[i] * (1.0 - buf * 0.01)) } else { lv };
                }
                let reason = if sl_hit { "Hard stop" } else if pb_hit { "Profit booking" } else if st_exit && st_on_exit { "ST exit" } else if swing_hit { "% swing exit" } else if bars_hit { "Bars exit" } else { "" };
                if reason.is_empty() {
                    // only the AFL's last-bar exit: the position is still open at the end of the data
                    out.push(trade(1.0, e, c[e], i, c[i], "open", true));
                } else {
                    out.push(trade(1.0, e, c[e], i, px, reason, false));
                }
                pos = None;
                hsl = None;
                pb = None;
                trail = 0.0;
                st_exit = false;
                flag52 = false;
            }
        }
        if pos.is_none() && entry[i] {
            pos = Some(i);
            hsl = if hsl_pct > 0.0 { Some(c[i] * (1.0 - hsl_pct * 0.01)) } else { None };
            pb = if pb_pct > 0.0 { Some(c[i] * (1.0 + pb_pct * 0.01)) } else { None };
            counter = 0;
            st_exit = false;
            trail = c[i];
        }
    }
    if let Some(e) = pos {
        out.push(trade(1.0, e, c[e], to, c[to], "open", true));
    }
    Some(out)
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    if id != SNIPER {
        return None;
    }
    Some((SNIPER, "Sniper (Monthly ST Cross)", "Sniper System"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;

    fn weekdays(start: (i64, i64, i64), count: usize) -> Vec<String> {
        let (mut y, mut m, mut d) = start;
        let mut out = Vec::new();
        while out.len() < count {
            let s = format!("{y:04}-{m:02}-{d:02}");
            let wd = crate::books::ytmr::day_number(&s).map_or(-1, |k| (k + 4).rem_euclid(7));
            if wd != 0 && wd != 6 {
                out.push(s);
            }
            let dim = match m { 2 => if y % 4 == 0 { 29 } else { 28 }, 4 | 6 | 9 | 11 => 30, _ => 31 };
            d += 1;
            if d > dim {
                d = 1;
                m += 1;
                if m > 12 {
                    m = 1;
                    y += 1;
                }
            }
        }
        out
    }
    fn rows(dates: &[String], closes: &[f64]) -> Vec<Row> {
        dates.iter().zip(closes).map(|(d, &c)| Row { ticker: "T".into(), date: d.clone(), open: c, high: c * 1.01, low: c * 0.99, close: c, volume: None }).collect()
    }
    fn params_of(kv: &[(&str, f64)]) -> Params {
        kv.iter().map(|(k, v)| (k.to_string(), *v)).collect()
    }

    #[test]
    fn st_loop_starts_at_period_and_ratchets() {
        let c = [10.0, 10.0, 10.0, 10.0, 12.0, 13.0, 9.0, 8.0];
        let h: Vec<f64> = c.iter().map(|v| v + 0.5).collect();
        let l: Vec<f64> = c.iter().map(|v| v - 0.5).collect();
        let (line, bear) = sniper_st(&c, &h, &l, &c, 2, 1.0, 0);
        // ATR Null on bars 0-1; the lower band only rises in the up trend; the close of 9 breaks it at bar 6, then
        // the upper band only falls
        assert_eq!(line, vec![None, None, Some(9.0), Some(9.0), Some(10.25), Some(11.0), Some(12.0), Some(11.0)]);
        assert_eq!(bear, vec![None, None, Some(false), Some(false), Some(false), Some(false), Some(true), Some(true)]);
    }

    #[test]
    fn monthly_line_seen_from_previous_month_until_month_end() {
        let d = weekdays((2024, 1, 1), 70);
        let closes: Vec<f64> = (0..70).map(|k| 100.0 + k as f64).collect();
        let r = rows(&d, &closes);
        let x = Ctx::new(&r, &r, &[]);
        let (_, _, cm) = st_on(&x, 2, 1, 1.0, 5);
        let jan_end = d.iter().position(|s| s == "2024-01-31").unwrap();
        // January days (before its last) see no completed month; Jan 31 sees January's close; Feb days see it too
        assert_eq!(cm[0], None);
        assert_eq!(cm[jan_end - 1], None);
        assert_eq!(cm[jan_end], Some(closes[jan_end]));
        assert_eq!(cm[jan_end + 1], Some(closes[jan_end]));
        assert_eq!(cm[69], None); // the last bar of the data
    }

    /// Weekly entry ST: 150 falling bars, a 10% jump on a Wednesday (through the previous week's ST), 100 rising, 12 falling.
    fn swing_series() -> Vec<f64> {
        let mut c = vec![200.0];
        for k in 1..262 {
            let f = if k < 150 { 0.997 } else if k == 150 { 1.10 } else if k < 250 { 1.006 } else { 0.985 };
            c.push(c[k - 1] * f);
        }
        c
    }

    #[test]
    fn daily_st_entry_never_fires_as_written() {
        // on the daily time frame Cross(C, ST) and C < ST cannot hold on one bar
        let c = swing_series();
        let r = rows(&weekdays((2020, 1, 1), c.len()), &c);
        let x = Ctx::new(&r, &r, &[]);
        assert!(run(SNIPER, &x, 0, c.len() - 1, &params_of(&[("entry_tf", 0.0)])).unwrap().is_empty());
    }

    #[test]
    fn weekly_cross_entry_and_swing_exit() {
        let c = swing_series();
        let r = rows(&weekdays((2020, 1, 1), c.len()), &c);
        let x = Ctx::new(&r, &r, &[]);
        let t = run(SNIPER, &x, 0, c.len() - 1, &params_of(&[("entry_tf", 1.0), ("pct_swing", 5.0)])).unwrap();
        assert!(!t.is_empty());
        let e = t[0].legs[0].i;
        assert_eq!(e, 150);
        let (line, bear, _) = st_on(&x, 1, 5, 2.0, 5);
        assert!(c[e] > line[e].unwrap() && c[e - 1] <= line[e - 1].unwrap_or(NULL) && bear[e] == Some(true));
        // exit: first close more than 5% below the highest close since entry
        let mut top = c[e];
        let want = (e + 1..c.len()).find(|&i| { top = top.max(c[i]); c[i] < top * 0.95 }).unwrap();
        assert_eq!((t[0].exit_i, t[0].reason.as_str(), t[0].exit_px), (want, "% swing exit", c[want]));
    }

    #[test]
    fn hard_stop_live_fills_at_gap_open() {
        let c = swing_series();
        let d = weekdays((2020, 1, 1), c.len());
        let mut r = rows(&d, &c);
        let p = params_of(&[("entry_tf", 1.0)]);
        let e = { let x = Ctx::new(&r, &r, &[]); run(SNIPER, &x, 0, c.len() - 1, &p).unwrap()[0].legs[0].i };
        // the next bar opens 10% below the entry close and its low is lower still: a live 5% stop fills at the open
        let ce = c[e];
        r[e + 1].open = ce * 0.90;
        r[e + 1].low = ce * 0.89;
        let x = Ctx::new(&r, &r, &[]);
        let t = run(SNIPER, &x, 0, c.len() - 1, &params_of(&[("entry_tf", 1.0), ("hsl_pct", 5.0), ("exit_live", 1.0)])).unwrap();
        assert_eq!(t[0].legs[0].i, e);
        assert_eq!((t[0].exit_i, t[0].reason.as_str()), (e + 1, "Hard stop"));
        assert!((t[0].exit_px - ce * 0.90).abs() < 1e-9);
        // stop tested on the close instead: the close is above the stop, no exit on that bar
        let t = run(SNIPER, &x, 0, c.len() - 1, &params_of(&[("entry_tf", 1.0), ("hsl_pct", 5.0)])).unwrap();
        assert_ne!(t[0].exit_i, e + 1);
    }

    #[test]
    fn defaults_hold_to_the_end() {
        let closes: Vec<f64> = (0..500).map(|k| 100.0 + 25.0 * ((k as f64) / 40.0).sin() + k as f64 * 0.05).collect();
        let d = weekdays((2015, 1, 1), 500);
        let r = rows(&d, &closes);
        let x = Ctx::new(&r, &r, &[]);
        let t = run(SNIPER, &x, 0, 499, &Params::new()).unwrap();
        // no exits on by default: at most one trade, open at the end
        assert!(t.len() <= 1);
        assert!(t.iter().all(|t| t.open && t.exit_i == 499));
    }

    #[test]
    fn every_strategy_runs() {
        let closes: Vec<f64> = (0..600).map(|k| 100.0 + 25.0 * ((k as f64) / 40.0).sin() + k as f64 * 0.05).collect();
        let d = weekdays((2015, 1, 1), 600);
        let r = rows(&d, &closes);
        let x = Ctx::new(&r, &r, &[]);
        for id in IDS {
            assert!(family_of(id).is_some());
            for kv in [
                vec![],
                vec![("entry_type", 1.0), ("pct_swing", 5.0)],
                vec![("entry_type", 2.0), ("st_exit", 1.0)],
                vec![("entry_tf", 1.0), ("hsl_pct", 5.0), ("pb_pct", 10.0), ("exit_live", 1.0), ("exit_bars", 20.0)],
                vec![("entry_tf", 0.0), ("st1_tf", 1.0), ("st2_tf", 2.0), ("st_exit", 1.0)],
            ] {
                let t = run(id, &x, 100, 599, &params_of(&kv)).unwrap();
                assert!(t.iter().all(|t| t.exit_i >= t.legs[0].i && t.legs[0].i >= 100), "{id} {kv:?}");
                assert!(t.iter().filter(|t| t.open).all(|t| t.exit_i == 599));
            }
        }
        assert_eq!(catalogue().len(), IDS.len());
    }
}
