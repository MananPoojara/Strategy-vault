//! YT strategy "January Barometer 5 Days" (not from a book). Source: the team's AmiBroker AFL
//! "January_Barometer_5_Days_CumReturn.afl", reduced to the rules the team specified for the YT Strategies tab.
//!
//! Conventions: daily bars, long only, one position at a time, entry and exits at the close of the signal bar.
//! Calendar months and years come from the bar dates. A month-end bar is one whose NEXT bar is in another month (AFL
//! `Month() != Ref(Month(), 1)`), so the last bar of the data is never a month-end. Weekly bars are calendar weeks
//! (Monday to Sunday) and monthly bars are calendar months. A higher-timeframe value reaches the daily bars the way
//! AFL `TimeFrameExpand` does: `expandLast` for the Dual ATR (the period's value appears on its last bar; earlier
//! bars of the period carry the previous period's value) and `expandPoint` for candle counts (only on the period's
//! last bar). Kept from the AFL: the five entry conditions, the upper-circuit entry block (High = Close on an up
//! close) and the lower-circuit exit deferral (an exit on a bar with Low = Close on a down close moves to the next bar
//! that is not one). Dropped: HSL, Supertrend, ST double filter, BASE 2 filters, dead-stock, penny, turnover and Aux
//! filters.
#![allow(dead_code)]
use crate::books::ytmr::day_number;
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "yt_jb";
pub const ID: &str = "ytjb_january_barometer_5d";

const NOTES: &str = "YT strategy, not a book method. Rules from the team's AFL \"January_Barometer_5_Days_CumReturn\" as specified for the YT Strategies tab. \
Entry conditions (each 0 = off, 1 = on; every condition that is on must hold, and at least one must be on): \
A1 each close of the first 5 January trading days > the previous December month-end close; A2 the 5th January day's close > that December close; \
B the January month-end close > that December close; C last year negative (December close of Y-1 < December close of Y-2); \
D cumulative return on the 5th January day > 0% (5th-day close / close of the bar before the 1st January day - 1). \
Entry at the close of the 5th January trading day, or of the January month-end bar when B is on. No entry when the signal bar closes at its high on an up close (upper circuit). \
Exits (each with its own switch; the first one that fires closes the trade, at that bar's close): \
December exit: the entry year's December month-end bar. \
Dual ATR on Daily, Weekly or Monthly bars (1/2/3): ATR = simple average of the 1-bar true range over the period on that timeframe; reference price (High+Low)/2. \
Initial stop = mid - ATR x mult (raised only); favour band = mid + ATR' x mult' (lowered only); favour stop = mid - ATR' x mult' (raised only). \
When the timeframe close is above both the previous favour band and the previous initial stop, the favour stop takes over. The trade exits when the timeframe close is below the previous active stop. \
N Candle exit on Daily, Weekly or Monthly candles: count every candle (1), green candles (2, close > open) or up-close candles (3, close > previous close) after entry and exit when the count reaches N (0 = off); \
the separate cap exits after that many daily bars (0 = off). \
An exit on a lower-circuit bar (Low = Close on a down close) waits for the next bar that is not one. \
Dropped from the AFL: HSL, Supertrend and ST double filters, BASE 2 dates, dead-stock, penny, turnover and Aux filters. Results are simulated on the selected instrument; no published figures exist for this strategy.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef {
        id: ID,
        name: "January Barometer 5 Days",
        group: "YT Strategies · January Barometer",
        session: 0,
        dir: "Long",
        status: Status::Live,
        needs_volume: false,
        needs_vix: false,
        needs_series: &[],
        book: BOOK,
        rules: &[
            "F|Compare January with the previous December month-end close; every condition switched on must hold",
            "E|Conditions: each of the first 5 January closes above the December close; the 5th January close above it; the January month-end close above it; last year negative; 5th-day cumulative return above 0%",
            "E|Buy at the close of the 5th January trading day, or of the January month-end bar when the month-end condition is on; no entry on an upper-circuit close",
            "X|December exit: close of the entry year's last December trading day",
            "X|Dual ATR trailing stop on daily, weekly or monthly bars; exit at the close when the timeframe close falls below the active stop",
            "X|N Candle exit: after N counted candles (any, green or up-close) or after a cap of N daily bars",
        ],
        notes: NOTES,
        claim: None,
    }]
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    if id != ID {
        return None;
    }
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    Some(vec![
        p("cond_a1", "A1: each of the first 5 Jan closes > Dec close (0 off, 1 on)", 0.0, 0.0, 1.0, 1.0),
        p("cond_a2", "A2: 5th Jan day close > Dec close (0 off, 1 on)", 0.0, 0.0, 1.0, 1.0),
        p("cond_b", "B: Jan month-end close > Dec close, entry at Jan month-end (0 off, 1 on)", 0.0, 0.0, 1.0, 1.0),
        p("cond_c", "C: last year negative (0 off, 1 on)", 0.0, 0.0, 1.0, 1.0),
        p("cond_d", "D: 5th Jan day cumulative return > 0% (0 off, 1 on)", 1.0, 0.0, 1.0, 1.0),
        p("dec_exit", "December month-end exit (0 off, 1 on)", 1.0, 0.0, 1.0, 1.0),
        p("atr_on", "Dual ATR exit (0 off, 1 on)", 0.0, 0.0, 1.0, 1.0),
        p("atr_tf", "Dual ATR timeframe (1 Daily, 2 Weekly, 3 Monthly)", 2.0, 1.0, 3.0, 1.0),
        p("atr_n", "Initial ATR period", 5.0, 1.0, 50.0, 1.0),
        p("atr_mult", "Initial ATR multiplier", 1.0, 1.0, 4.0, 1.0),
        p("fav_n", "Favour ATR period", 5.0, 1.0, 50.0, 1.0),
        p("fav_mult", "Favour ATR multiplier", 1.0, 1.0, 4.0, 1.0),
        p("nc_on", "N Candle exit (0 off, 1 on)", 1.0, 0.0, 1.0, 1.0),
        p("nc_type", "Candle type (1 N Candle, 2 Green, 3 UpClose)", 1.0, 1.0, 3.0, 1.0),
        p("nc_tf", "Candle timeframe (1 Daily, 2 Weekly, 3 Monthly)", 1.0, 1.0, 3.0, 1.0),
        p("nc_n", "N candles (0 off)", 0.0, 0.0, 50.0, 1.0),
        p("nc_cap", "N candles exit only, daily bars (0 off)", 0.0, 0.0, 10.0, 1.0),
    ])
}

fn year(x: &Ctx, i: usize) -> i32 {
    x.r[i].date.get(0..4).and_then(|s| s.parse().ok()).unwrap_or(0)
}
fn month(x: &Ctx, i: usize) -> u32 {
    x.r[i].date.get(5..7).and_then(|s| s.parse().ok()).unwrap_or(0)
}

/// Bars of one higher timeframe: each period's first and last daily index (1 Daily, 2 Weekly, 3 Monthly).
fn periods(x: &Ctx, tf: i64) -> Vec<(usize, usize)> {
    let key = |i: usize| -> i64 {
        match tf {
            2 => day_number(&x.r[i].date).map_or(i as i64, |d| (d + 3).div_euclid(7)), // Monday-based weeks
            3 => year(x, i) as i64 * 12 + month(x, i) as i64,
            _ => i as i64,
        }
    };
    let mut out: Vec<(usize, usize)> = Vec::new();
    for i in 0..x.c.len() {
        match out.last_mut() {
            Some(last) if key(last.0) == key(i) => last.1 = i,
            _ => out.push((i, i)),
        }
    }
    out
}

/// Dual ATR inputs on the daily bars: (ATR over `n` periods, (High+Low)/2, close), each expanded with `expandLast`.
fn atr_inputs(x: &Ctx, ps: &[(usize, usize)], n: usize) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
    let (h, l, c): (Vec<f64>, Vec<f64>, Vec<f64>) = ps
        .iter()
        .map(|&(a, b)| {
            let hi = x.h[a..=b].iter().cloned().fold(f64::MIN, f64::max);
            let lo = x.l[a..=b].iter().cloned().fold(f64::MAX, f64::min);
            (hi, lo, x.c[b])
        })
        .fold((vec![], vec![], vec![]), |mut v, (hi, lo, cl)| {
            v.0.push(hi);
            v.1.push(lo);
            v.2.push(cl);
            v
        });
    let tr: Vec<f64> = (0..ps.len())
        .map(|k| if k == 0 { h[0] - l[0] } else { (h[k] - l[k]).max((h[k] - c[k - 1]).abs()).max((l[k] - c[k - 1]).abs()) })
        .collect();
    let atr: Vec<f64> = (0..ps.len()).map(|k| if k + 1 >= n { tr[k + 1 - n..=k].iter().sum::<f64>() / n as f64 } else { f64::NAN }).collect();
    let n_bars = x.c.len();
    let (mut a, mut m, mut cl) = (vec![f64::NAN; n_bars], vec![f64::NAN; n_bars], vec![f64::NAN; n_bars]);
    for (k, &(s, e)) in ps.iter().enumerate() {
        for i in s..=e {
            let j = if i == e { Some(k) } else { k.checked_sub(1) };
            if let Some(j) = j {
                a[i] = atr[j];
                m[i] = (h[j] + l[j]) / 2.0;
                cl[i] = c[j];
            }
        }
    }
    (a, m, cl)
}

/// Candle-exit flags on the daily bars (`expandPoint`): (period ends, green candle, up-close candle).
fn candle_flags(x: &Ctx, ps: &[(usize, usize)]) -> (Vec<bool>, Vec<bool>, Vec<bool>) {
    let n = x.c.len();
    let (mut any, mut green, mut up) = (vec![false; n], vec![false; n], vec![false; n]);
    for (k, &(s, e)) in ps.iter().enumerate() {
        any[e] = true;
        green[e] = x.c[e] > x.o[s];
        up[e] = k > 0 && x.c[ps[k - 1].1] != 0.0 && x.c[e] > x.c[ps[k - 1].1];
    }
    (any, green, up)
}

/// AFL max/min with Null propagation (a Null level stays Null).
fn mx(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() { f64::NAN } else { a.max(b) }
}
fn mn(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() { f64::NAN } else { a.min(b) }
}

/// Entry signal and December month-end flags for every bar (the AFL January Barometer loop).
pub fn signals(x: &Ctx, on: [bool; 5]) -> (Vec<bool>, Vec<bool>) {
    let n = x.c.len();
    let [a1_on, a2_on, b_on, c_on, d_on] = on;
    let (mut entry, mut dec_end) = (vec![false; n], vec![false; n]);
    let (mut last_dec, mut prev_dec, mut last_dec_y, mut prev_dec_y) = (0.0, 0.0, 0, 0);
    let (mut count, mut dec_ok, mut all_pos, mut a2, mut c_flag, mut d, mut base) = (0, false, false, false, false, false, 0.0);
    for i in 0..n {
        let (y, m, c) = (year(x, i), month(x, i), x.c[i]);
        let month_end = i + 1 < n && month(x, i + 1) != m;
        if m == 12 {
            if y != last_dec_y {
                prev_dec = last_dec;
                prev_dec_y = last_dec_y;
            }
            last_dec = c;
            last_dec_y = y;
            dec_end[i] = month_end;
        }
        if m != 1 {
            count = 0;
            continue;
        }
        if i == 0 || month(x, i - 1) != 1 || year(x, i - 1) != y {
            count = 1;
            dec_ok = last_dec_y == y - 1 && last_dec > 0.0;
            c_flag = dec_ok && prev_dec_y == y - 2 && prev_dec > 0.0 && last_dec < prev_dec;
            all_pos = dec_ok && c > last_dec;
            a2 = false;
            d = false;
            base = if i > 0 { x.c[i - 1] } else { 0.0 };
        } else {
            count += 1;
            if count <= 5 {
                all_pos = all_pos && dec_ok && c > last_dec;
            }
        }
        if count == 5 {
            a2 = dec_ok && c > last_dec;
            if base > 0.0 {
                d = dec_ok && (c / base - 1.0) * 100.0 > 0.0;
            }
        }
        let late = count >= 5;
        let b = month_end && dec_ok && c > last_dec;
        let ok = (!a1_on || (late && all_pos)) && (!a2_on || (late && a2)) && (!b_on || b) && (!c_on || c_flag) && (!d_on || (late && d));
        let bar = if b_on { month_end } else { count == 5 };
        entry[i] = on.iter().any(|&f| f) && bar && ok;
    }
    (entry, dec_end)
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(0.0, |d| d.def));
    let on = |k: &str| g(k) >= 0.5;
    let whole = |k: &str| g(k).round().max(0.0) as usize;
    let (entry, dec_end) = signals(x, [on("cond_a1"), on("cond_a2"), on("cond_b"), on("cond_c"), on("cond_d")]);
    let dec_on = on("dec_exit");
    let atr_on = on("atr_on");
    let (m1, m2) = (g("atr_mult"), g("fav_mult"));
    let ((a1, mid1, aclose), (a2, mid2, _)) = if atr_on {
        let ps = periods(x, g("atr_tf").round() as i64);
        (atr_inputs(x, &ps, whole("atr_n").max(1)), atr_inputs(x, &ps, whole("fav_n").max(1)))
    } else {
        Default::default()
    };
    let nc_on = on("nc_on");
    let (nc_type, nc_n, nc_cap) = (g("nc_type").round() as i64, whole("nc_n"), whole("nc_cap"));
    let (cand, green, up) = candle_flags(x, &periods(x, g("nc_tf").round() as i64));
    let uc = |i: usize| x.h[i] == x.c[i] && x.c[i] > x.c[i - 1];
    let lc = |i: usize| x.l[i] == x.c[i] && x.c[i] < x.c[i - 1];

    struct Open {
        t: Trade,
        year: i32,
        stop: f64,
        upper: f64,
        fav: f64,
        favour: bool,
        count: usize,
        capped: usize,
        held: usize,
        deferred: bool,
    }
    let mut trades = Vec::new();
    let mut cur: Option<Open> = None;
    let mut exit_bar = usize::MAX;
    for i in from.max(1)..=to {
        if let Some(mut s) = cur.take() {
            let (prev_stop, prev_upper, prev_fav) = (s.stop, s.upper, s.fav);
            if atr_on {
                s.stop = mx(mid1[i] - a1[i] * m1, s.stop);
                s.upper = mn(mid2[i] + a2[i] * m2, s.upper);
                s.fav = mx(mid2[i] - a2[i] * m2, s.fav);
                if aclose[i] > prev_upper && aclose[i] > prev_stop {
                    s.favour = true;
                    s.upper = f64::NAN;
                }
            }
            let mut candle_hit = false;
            if nc_on {
                let counted = match nc_type {
                    1 => cand[i],
                    2 => green[i],
                    3 => up[i],
                    _ => false,
                };
                s.count += usize::from(counted);
                s.capped += usize::from(nc_type != 1 && nc_cap > 0 && cand[i]);
                candle_hit = (nc_n > 0 && s.count == nc_n) || (nc_cap > 0 && s.capped >= nc_cap);
            }
            s.held += 1;
            let atr_hit = atr_on && if s.favour { aclose[i] < prev_fav } else { aclose[i] < prev_stop };
            let dec_hit = dec_on && dec_end[i] && year(x, i) == s.year;
            let cap_hit = nc_on && nc_cap > 0 && s.held >= nc_cap;
            let why = if atr_hit {
                Some(if s.favour { "Favour ATR stop" } else { "Initial ATR stop" }.to_string())
            } else if candle_hit {
                Some(format!("{nc_n} candle{}", if nc_n == 1 { "" } else { "s" }))
            } else if dec_hit {
                Some("December month-end".to_string())
            } else if s.deferred {
                Some("Exit deferred from a lower-circuit bar".to_string())
            } else if cap_hit {
                Some(format!("{nc_cap}-bar cap"))
            } else {
                None
            };
            match why {
                Some(_) if lc(i) => {
                    s.deferred = true;
                    cur = Some(s);
                }
                Some(reason) => {
                    s.t.exit_i = i;
                    s.t.exit_px = x.c[i];
                    s.t.signal_i = i;
                    s.t.reason = reason;
                    trades.push(s.t);
                    exit_bar = i;
                }
                None => cur = Some(s),
            }
            continue;
        }
        if i == exit_bar || !entry[i] || uc(i) || x.c[i] <= 0.0 {
            continue;
        }
        let (stop, upper, fav) = if atr_on {
            (mid1[i] - a1[i] * m1, mid2[i] + a2[i] * m2, mid2[i] - a2[i] * m2)
        } else {
            (f64::NAN, f64::NAN, f64::NAN)
        };
        let px = x.c[i];
        let t = Trade { dir: 1.0, legs: vec![Leg { i, px, w: 1.0 }], exit_i: i, exit_px: px, reason: String::new(), entry_kind: "close", signal_i: i, open: false, exits: Vec::new() };
        cur = Some(Open { t, year: year(x, i), stop, upper, fav, favour: false, count: 0, capped: 0, held: 0, deferred: false });
    }
    if let Some(Open { mut t, .. }) = cur {
        t.exit_i = to;
        t.exit_px = x.c[to];
        t.reason = "open".into();
        t.signal_i = to;
        t.open = true;
        trades.push(t);
    }
    Some(trades)
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    (id == ID).then_some(("ytjb", "January Barometer", "5 Days"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::books::ytmr::day_number;
    use crate::Row;

    fn civil(z: i64) -> String {
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
    /// Weekdays 2022-12-01 .. 2025-01-10, flat at close 100 (high 101, low 99) unless `set` lists the date.
    fn rows(set: &[(&str, (f64, f64, f64, f64))]) -> Vec<Row> {
        let (mut d, end) = (day_number("2022-12-01").unwrap(), day_number("2025-01-10").unwrap());
        let mut out = Vec::new();
        while d <= end {
            if (d + 4).rem_euclid(7) % 6 != 0 {
                let date = civil(d);
                let (open, high, low, close) = set.iter().find(|(k, _)| *k == date).map_or((100.0, 101.0, 99.0, 100.0), |x| x.1);
                out.push(Row { ticker: "T".into(), date, open, high, low, close, volume: None });
            }
            d += 1;
        }
        out
    }
    fn bar(c: f64) -> (f64, f64, f64, f64) {
        (c, c + 1.0, c - 1.0, c)
    }
    /// Jan 2024: closes 101..105 on the first five trading days (Jan 1-5) over a December close of 100.
    fn jan(extra: &[(&'static str, (f64, f64, f64, f64))]) -> Vec<Row> {
        let mut s = extra.to_vec(); // overrides first: `rows` takes the first match
        s.extend([("2024-01-01", bar(101.0)), ("2024-01-02", bar(102.0)), ("2024-01-03", bar(103.0)), ("2024-01-04", bar(104.0)), ("2024-01-05", bar(105.0))]);
        rows(&s)
    }
    fn ps(kv: &[(&str, f64)]) -> Params {
        kv.iter().map(|(k, v)| (k.to_string(), *v)).collect()
    }
    fn go(r: &[Row], kv: &[(&str, f64)]) -> Vec<(String, f64, String, f64, String)> {
        let x = Ctx::new(r, r, &[]);
        run(ID, &x, 0, r.len() - 1, &ps(kv)).unwrap().into_iter()
            .map(|t| (r[t.legs[0].i].date.clone(), t.legs[0].px, r[t.exit_i].date.clone(), t.exit_px, t.reason)).collect()
    }
    fn at(r: &[Row], date: &str) -> usize {
        r.iter().position(|x| x.date == date).unwrap()
    }

    #[test]
    fn default_d_enters_on_fifth_january_day_and_exits_at_december_month_end() {
        let r = jan(&[]);
        let t = go(&r, &[]);
        assert_eq!(t, vec![("2024-01-05".into(), 105.0, "2024-12-31".into(), 100.0, "December month-end".into())]);
        // A1, A2 hold too; with the December exit off the trade stays open to the last bar
        assert_eq!(go(&r, &[("cond_d", 0.0), ("cond_a1", 1.0)]).len(), 1);
        assert_eq!(go(&r, &[("cond_d", 0.0), ("cond_a2", 1.0)]).len(), 1);
        assert_eq!(go(&r, &[("dec_exit", 0.0)])[0].4, "open");
        // all conditions off: no entry
        assert!(go(&r, &[("cond_d", 0.0)]).is_empty());
    }

    #[test]
    fn a1_needs_every_one_of_the_first_five_closes() {
        let r = jan(&[("2024-01-02", bar(99.0))]);
        assert!(go(&r, &[("cond_d", 0.0), ("cond_a1", 1.0)]).is_empty());
        assert_eq!(go(&r, &[("cond_d", 0.0), ("cond_a2", 1.0)]).len(), 1);
    }

    #[test]
    fn b_moves_entry_to_january_month_end_and_c_needs_a_down_year() {
        let r = jan(&[]);
        assert!(go(&r, &[("cond_b", 1.0)]).is_empty()); // Jan 31 close 100 is not above 100
        let r = jan(&[("2024-01-31", bar(110.0))]);
        assert_eq!(go(&r, &[("cond_b", 1.0)])[0].0, "2024-01-31");
        assert!(go(&r, &[("cond_c", 1.0)]).is_empty()); // Dec 2023 100 is not below Dec 2022 100
        let r = jan(&[("2022-12-30", (120.0, 121.0, 99.0, 120.0))]);
        assert_eq!(go(&r, &[("cond_c", 1.0)])[0].0, "2024-01-05");
    }

    #[test]
    fn upper_circuit_blocks_entry_and_lower_circuit_defers_exit() {
        let r = jan(&[("2024-01-05", (105.0, 105.0, 104.0, 105.0))]);
        assert!(go(&r, &[]).is_empty());
        let r = jan(&[("2024-12-31", (100.0, 101.0, 99.0, 99.0))]);
        let t = go(&r, &[]);
        assert_eq!((t[0].2.as_str(), t[0].3, t[0].4.as_str()), ("2025-01-01", 100.0, "Exit deferred from a lower-circuit bar"));
    }

    #[test]
    fn n_candle_types_cap_and_weekly_candles() {
        // after Friday Jan 5: Mon 100 (down), Tue 101 (up), Wed 100, Thu 101 (up)
        let up = (100.0, 102.0, 99.0, 101.0);
        let r = jan(&[("2024-01-09", up), ("2024-01-11", up)]);
        assert_eq!(go(&r, &[("nc_n", 3.0)])[0].2, "2024-01-10");
        assert_eq!(go(&r, &[("nc_n", 2.0), ("nc_type", 3.0)])[0].2, "2024-01-11");
        assert_eq!(go(&r, &[("nc_n", 1.0), ("nc_type", 2.0)])[0].2, "2024-01-09"); // open 100 -> close 101 is green
        assert_eq!(go(&r, &[("nc_cap", 2.0), ("nc_type", 3.0)])[0].2, "2024-01-09");
        assert_eq!(go(&r, &[("nc_n", 1.0), ("nc_tf", 2.0)])[0].2, "2024-01-12"); // next week's last bar
        assert_eq!(go(&r, &[("nc_n", 2.0), ("nc_on", 0.0)])[0].2, "2024-12-31");
    }

    #[test]
    fn weekly_values_expand_from_the_last_bar_of_each_week() {
        let r = jan(&[]);
        let x = Ctx::new(&r, &r, &[]);
        let (_, mid, close) = atr_inputs(&x, &periods(&x, 2), 1);
        let (wed, fri, mon) = (at(&r, "2024-01-03"), at(&r, "2024-01-05"), at(&r, "2024-01-08"));
        assert_eq!((close[wed], close[fri], close[mon]), (100.0, 105.0, 105.0)); // Wed sees last week's close
        assert_eq!(mid[fri], (106.0 + 100.0) / 2.0); // week Jan 1-5: high 106 (Fri), low 100 (Mon)
        let (any, _, _) = candle_flags(&x, &periods(&x, 3));
        assert!(any[at(&r, "2024-01-31")] && !any[at(&r, "2024-01-30")]);
    }

    #[test]
    fn daily_dual_atr_initial_stop() {
        // Jan 5: TR = max(106-104, |106-104|, |104-104|) = 2, mid 105 -> stop 105 - 2x1 = 103; Jan 8 closes 102 < 103
        let r = jan(&[("2024-01-08", (103.0, 103.5, 101.5, 102.0))]);
        let t = go(&r, &[("atr_on", 1.0), ("atr_tf", 1.0), ("atr_n", 1.0), ("fav_n", 1.0)]);
        assert_eq!((t[0].2.as_str(), t[0].3, t[0].4.as_str()), ("2024-01-08", 102.0, "Initial ATR stop"));
    }
}
