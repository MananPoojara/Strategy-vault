//! YT Strategies · Anti Turtle: the team's AmiBroker AFL in "Old Topics/Anti Turtle" (run on spot Nifty / Bank Nifty
//! daily bars). Coded from the AFL as written, with its own default settings; there is no book PDF. AmiBroker
//! conventions as in `ytsw.rs` and `ytwh.rs`. The "Anti Turtle System" sub-folder holds a copy of the same AFL.
#![allow(dead_code)]
use crate::books::ytsw::{compress, cross_down, true_range};
use crate::books::ytwh::{bound, collect, llv, Ev};
use crate::connors::{rsi, Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "yt_at";
pub const ANTI_TURTLE: &str = "ytat_anti_turtle";
pub const IDS: [&str; 1] = [ANTI_TURTLE];

fn def(id: &'static str, name: &'static str, dir: &'static str, rules: &'static [&'static str], notes: &'static str) -> StrategyDef {
    StrategyDef { id, name, group: "YT Strategies · Anti Turtle", session: 0, dir, status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim: None }
}

const SRC: &str = "(1 close, 2 open, 3 high, 4 low, 5 (O+C)/2, 6 (H+L)/2, 7 (H+L+C)/3, 8 (O+H+L+C)/4)";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![def(ANTI_TURTLE, "Anti Turtle (N-Day Low Breakdown Buy)", "Long", &[
        "E|Buy at the close when the close is below the lowest low (price choice; default low) of the previous 10 bars; never on the last bar of the data",
        "F|Optional monthly Supertrend filter (default ANY = off; period 5, factor 1, (H+L)/2, ATR = simple average of ATR(1), bands ratchet without a reset on a flip): Bullish needs the previous month's close above its ST, Bearish below; on a month's last day the month just ending already counts",
        "X|Exit on the first of (all off by default): profit booking %, hard SL %, trailing SL % (from the highest close-based level), N candles, N green candles, N up closes, RSI crossing a threshold, RSI falling a set drop below its peak, the daily ST (5, 1, (H+L)/2) crossing above the close after a close above it, or the filter no longer matching; and always on the last bar of the data",
        "X|Live mode fills a hit level at the level, or at the open less the buffer when the open is beyond it (not below the low); close mode tests the levels on the close",
        "X|If an exit falls on a bar that is itself a new entry signal (and no live-mode price level was hit), the position simply continues: no sell and no new buy, with stops and counters restarted from that close",
    ], "Anti Turtle.afl, coded as the AFL is written. Exit price as written: only the live-mode price levels set SellPrice; every other exit (candle counts, RSI, ST, filter, close mode, last bar) keeps the SellPrice carried from the previous bar, which is the fill price of the last price-level exit of an earlier trade (moved into the bar's low-high range by AmiBroker's price bound checking). Before any price-level exit has happened SellPrice is Null; that case is filled at the close here (AmiBroker's default trade price) and is a coding choice, not something the AFL states. With every exit at its default (off) the only exit is the last bar of the data; the team's runs set exits. The PB % and HSL % Param defaults (0) lie below their minimum (1) and are taken as 0 = off. The ParamStr filter timeframe is unused: the filter is always monthly. Monthly warm-up bars (filter Null) count as not bullish."),
    ]
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    if id != ANTI_TURTLE {
        return None;
    }
    Some(vec![
        p("days", "Number of days for the lowest price", 10.0, 2.0, 30.0, 2.0),
        p("price", "Lookback price (1 open, 2 high, 3 low, 4 close)", 3.0, 1.0, 4.0, 1.0),
        p("flt_type", "Monthly ST filter (1 ANY, 2 bullish, 3 bearish)", 1.0, 1.0, 3.0, 1.0),
        p("flt_period", "Filter ST period", 5.0, 1.0, 9.0, 1.0),
        p("flt_factor", "Filter ST factor", 1.0, 1.0, 6.0, 1.0),
        p("flt_src", "Filter ST average value (1 close, 2 open, 3 high, 4 low, 5 (O+C)/2, 6 (H+L)/2, 7 (H+L+C)/3, 8 (O+H+L+C)/4)", 6.0, 1.0, 8.0, 1.0),
        p("exit_at", "Exit at (0 live, 1 close)", 0.0, 0.0, 1.0, 1.0),
        p("pb_pct", "Profit booking % (0 off)", 0.0, 0.0, 7.0, 1.0),
        p("hsl_pct", "Hard SL % (0 off)", 0.0, 0.0, 7.0, 1.0),
        p("tsl_pct", "Trailing SL % (0 off)", 0.0, 0.0, 100.0, 0.1),
        p("buffer_pct", "Buffer % on gap fills", 0.0, 0.0, 10.0, 0.1),
        p("n_candles", "Exit after N candles (0 off)", 0.0, 0.0, 50.0, 1.0),
        p("n_green", "Exit after N green candles (0 off)", 0.0, 0.0, 50.0, 1.0),
        p("n_upclose", "Exit after N up closes (0 off)", 0.0, 0.0, 50.0, 1.0),
        p("rsi_period", "RSI period", 14.0, 1.0, 80.0, 1.0),
        p("rsi_thr_exit", "RSI threshold exit (0 off, 1 on)", 0.0, 0.0, 1.0, 1.0),
        p("rsi_cross", "RSI crosses the threshold (0 below, 1 above)", 0.0, 0.0, 1.0, 1.0),
        p("rsi_thr", "RSI threshold", 1.0, 1.0, 100.0, 1.0),
        p("rsi_trail_exit", "RSI trailing exit (0 off, 1 on)", 0.0, 0.0, 1.0, 1.0),
        p("rsi_drop", "RSI drop value", 20.0, 1.0, 100.0, 1.0),
        p("st_exit", "Daily ST exit (0 off, 1 on)", 0.0, 0.0, 1.0, 1.0),
        p("st_period", "Exit ST period", 5.0, 1.0, 9.0, 1.0),
        p("st_factor", "Exit ST factor", 1.0, 1.0, 6.0, 1.0),
        p("st_src", "Exit ST average value (same list as the filter)", 6.0, 1.0, 8.0, 1.0),
    ])
}

/// AFL CalculateAvg: 1 close, 2 open, 3 high, 4 low, 5 (O+C)/2, 6 (H+L)/2, 7 (H+L+C)/3, 8 (O+H+L+C)/4.
fn avg_val(kind: i64, o: f64, h: f64, l: f64, c: f64) -> f64 {
    match kind {
        2 => o,
        3 => h,
        4 => l,
        5 => (o + c) / 2.0,
        6 => (h + l) / 2.0,
        7 => (h + l + c) / 3.0,
        8 => (o + h + l + c) / 4.0,
        _ => c,
    }
}

/// AFL getSuperTrend: satr = mean of the last `period` ATR(1) (Null until they exist); LineUp / LineDn = src +/- f x
/// satr; from bar `period`: up if C > LineUp[i-1], down if C < LineDn[i-1], else unchanged; while up LineDn =
/// Max(LineDn, LineDn[i-1]), while down LineUp = Min(LineUp, LineUp[i-1]) (no reset on a flip). ST = up ? LineDn :
/// LineUp. A Null previous line (the first band bar) leaves the comparison and the ratchet out.
fn st_line(src: &[f64], h: &[f64], l: &[f64], c: &[f64], period: usize, f: f64) -> Vec<Option<f64>> {
    let n = c.len();
    let p = period.max(1);
    let trr = true_range(h, l, c);
    let satr: Vec<Option<f64>> = (0..n)
        .map(|i| if i + 1 < p { None } else { trr[i + 1 - p..=i].iter().try_fold(0.0, |s, v| v.map(|v| s + v)).map(|s| s / p as f64) })
        .collect();
    let mut up: Vec<Option<f64>> = (0..n).map(|i| satr[i].map(|s| src[i] + s * f)).collect();
    let mut dn: Vec<Option<f64>> = (0..n).map(|i| satr[i].map(|s| src[i] - s * f)).collect();
    let mut is_up = vec![true; n];
    for i in p..n {
        is_up[i] = match (up[i - 1], dn[i - 1]) {
            (Some(u), _) if c[i] > u => true,
            (_, Some(d)) if c[i] < d => false,
            (None, _) => true, // C > Null (-1e10) in the loop
            _ => is_up[i - 1],
        };
        if is_up[i] {
            if let (Some(a), Some(b)) = (dn[i], dn[i - 1]) {
                dn[i] = Some(a.max(b));
            }
        } else if let (Some(a), Some(b)) = (up[i], up[i - 1]) {
            up[i] = Some(a.min(b));
        }
    }
    (0..n).map(|i| if is_up[i] { dn[i] } else { up[i] }).collect()
}

/// Monthly ST filter as the AFL builds it: IsBull = monthly C > ST, Ref(-1) on months, expandFirst to days, then
/// Ref(+1) on days. Leading Nulls and the last bar (Ref +1 past the end) are not bullish.
fn monthly_bull(x: &Ctx, period: usize, f: f64, src: i64) -> Vec<bool> {
    let n = x.c.len();
    let b = compress(x, 'M');
    let m = b.c.len();
    let mut mo = vec![0.0; m];
    for i in (0..n).rev() {
        mo[b.group[i]] = x.o[i];
    }
    let s: Vec<f64> = (0..m).map(|g| avg_val(src, mo[g], b.h[g], b.l[g], b.c[g])).collect();
    let st = st_line(&s, &b.h, &b.l, &b.c, period, f);
    let bull: Vec<bool> = (0..m).map(|g| st[g].map_or(false, |v| b.c[g] > v)).collect();
    let prev = |g: usize| g > 0 && bull[g - 1];
    (0..n).map(|i| i + 1 < n && prev(b.group[i + 1])).collect()
}

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
    let days = gu("days");
    let price: Vec<f64> = match gi("price") { 1 => o.clone(), 2 => h.clone(), 4 => c.clone(), _ => l.clone() };
    let lo = llv(&price, days);
    let entry: Vec<bool> = (0..n).map(|i| i > 0 && c[i] < lo[i - 1]).collect();
    let ftype = gi("flt_type");
    let bull = if ftype == 2 || ftype == 3 { monthly_bull(x, gu("flt_period"), g("flt_factor"), gi("flt_src")) } else { vec![false; n] };
    let flt_ok = |i: usize| match ftype { 2 => bull[i], 3 => !bull[i], _ => true };
    let r = rsi(c, gu("rsi_period"));
    let thr = g("rsi_thr");
    let rsi_sig: Vec<bool> = (0..n)
        .map(|i| i > 0 && !r[i].is_nan() && !r[i - 1].is_nan() && if gi("rsi_cross") == 1 { r[i] > thr && r[i - 1] <= thr } else { thr > r[i] && thr <= r[i - 1] })
        .collect();
    let st_src: Vec<f64> = (0..n).map(|i| avg_val(gi("st_src"), o[i], h[i], l[i], c[i])).collect();
    let st = st_line(&st_src, h, l, c, gu("st_period"), g("st_factor"));
    let st_bull: Vec<bool> = (0..n).map(|i| st[i].map_or(false, |v| c[i] > v)).collect();
    let st_bear = cross_down(c, &st); // Cross(STPlot, Close)

    let live = gi("exit_at") == 0;
    let (pb_pct, hsl_pct, tsl_pct, buf) = (g("pb_pct"), g("hsl_pct"), g("tsl_pct"), g("buffer_pct"));
    let (n_c, n_g, n_u) = (gi("n_candles"), gi("n_green"), gi("n_upclose"));
    let (thr_on, trail_on, st_on, drop) = (gi("rsi_thr_exit") == 1, gi("rsi_trail_exit") == 1, gi("st_exit") == 1, g("rsi_drop"));

    let mut evs: Vec<Ev> = Vec::new();
    let mut pos = false;
    let (mut pb, mut hsl, mut tsl): (Option<f64>, Option<f64>, Option<f64>) = (None, None, None);
    let mut sell_px: Option<f64> = None; // the AFL's SellPrice array, carried bar to bar
    let (mut cnt, mut green, mut upc) = (0i64, 0i64, 0i64);
    let mut fresh = false;
    let mut trail_rsi = 0.0f64;
    let mut cont = false;
    for i in days..n {
        let tsl_prev = tsl;
        if pos {
            let lp = if live { l[i] } else { c[i] };
            let hp = if live { h[i] } else { c[i] };
            let hsl_hit = hsl_pct > 0.0 && hsl.map_or(false, |v| lp <= v);
            let tsl_hit = tsl_pct > 0.0 && tsl.map_or(false, |v| lp <= v);
            let pb_hit = pb_pct > 0.0 && pb.map_or(false, |v| hp >= v);
            cnt += 1;
            if c[i] >= o[i] {
                green += 1;
            }
            if c[i] >= c[i - 1] {
                upc += 1;
            }
            if !fresh && st_bull[i] {
                fresh = true;
            }
            let reason = if i == n - 1 {
                Some("Last bar")
            } else if hsl_hit || tsl_hit || pb_hit {
                Some(if tsl_hit { "Trailing SL" } else if hsl_hit { "Hard SL" } else { "Profit booking" })
            } else if n_c > 0 && cnt >= n_c {
                Some("N candles")
            } else if n_g > 0 && green >= n_g {
                Some("N green candles")
            } else if n_u > 0 && upc >= n_u {
                Some("N up closes")
            } else if thr_on && rsi_sig[i] {
                Some("RSI threshold")
            } else if st_on && fresh && st_bear[i] {
                Some("ST exit")
            } else if trail_on && r[i] < trail_rsi {
                Some("RSI trail")
            } else if ftype != 1 && !flt_ok(i) {
                Some("Filter")
            } else {
                None
            };
            if let Some(reason) = reason {
                if entry[i] && i != n - 1 && flt_ok(i) && !(live && (tsl_hit || hsl_hit || pb_hit)) {
                    cont = true;
                } else {
                    let gap = |lv: f64, beyond: bool| if beyond { l[i].max(o[i] * (1.0 - buf * 0.01)) } else { lv };
                    if pb_hit && live {
                        let v = pb.unwrap_or(0.0);
                        sell_px = Some(gap(v, o[i] > v));
                    }
                    if hsl_hit && live {
                        let v = hsl.unwrap_or(0.0);
                        sell_px = Some(gap(v, o[i] < v));
                    }
                    if tsl_hit && live {
                        let v = tsl.unwrap_or(0.0);
                        sell_px = Some(gap(v, o[i] < v));
                    }
                    let px = bound(x, i, sell_px.unwrap_or(c[i]));
                    if let Some(v) = evs.last_mut() {
                        v.exit = Some((i, px, i, reason));
                    }
                }
                pos = false;
                trail_rsi = 0.0;
                pb = None;
                hsl = None;
                tsl = None;
            }
            if tsl_pct > 0.0 {
                let v = c[i] * (1.0 - tsl_pct * 0.01);
                tsl = Some(tsl_prev.map_or(v, |t| v.max(t)));
            }
            if trail_on {
                trail_rsi = trail_rsi.max(r[i] - drop);
            }
        }
        if !pos && entry[i] && i != n - 1 && flt_ok(i) {
            if !cont {
                evs.push(Ev { dir: 1.0, e: i, epx: c[i], exit: None });
            }
            pos = true;
            hsl = if hsl_pct > 0.0 { Some(c[i] * (1.0 - hsl_pct * 0.01)) } else { None };
            tsl = if tsl_pct > 0.0 { Some(c[i] * (1.0 - tsl_pct * 0.01)) } else { None };
            pb = if pb_pct > 0.0 { Some(c[i] * (1.0 + pb_pct * 0.01)) } else { None };
            cnt = 0;
            green = 0;
            upc = 0;
            trail_rsi = r[i] - drop;
            cont = false;
            fresh = st_bull[i];
        }
    }
    Some(collect(x, evs, from, to))
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let name = catalogue().into_iter().find(|d| d.id == id)?.name;
    Some((match id {
        ANTI_TURTLE => "ytat_anti_turtle",
        _ => return None,
    }, name, "Anti Turtle"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::books::ytwh::testbars::*;

    fn flat(c: f64) -> (f64, f64, f64, f64) {
        (c, c + 1.0, c - 1.0, c)
    }

    #[test]
    fn buys_breakdown_and_exits_after_n_candles() {
        // 10 bars at 100 (lows 99), bar 10 closes 97 < 99 -> buy; 3-candle exit on bar 13 at the close (no SellPrice yet)
        let mut v: Vec<_> = (0..10).map(|_| flat(100.0)).collect();
        v.extend([flat(97.0), flat(98.0), flat(99.0), flat(100.0), flat(101.0), flat(101.0)]);
        let r = bars(&v);
        let t = go(run, ANTI_TURTLE, &r, &[("n_candles", 3.0)]);
        assert_eq!(t[0], (10, 97.0, 13, 100.0, false));
        // defaults: every exit off, the only exit is the last bar of the data
        let t = go(run, ANTI_TURTLE, &r, &[]);
        assert_eq!((t[0].0, t[0].2, t[0].4), (10, 15, false));
    }

    #[test]
    fn non_price_exit_carries_previous_sell_price() {
        // trade 1 books profit at 97 x 1.02 = 98.94 (live level); trade 2 exits on N candles and is filled at the
        // carried SellPrice 98.94, moved into that bar's range
        let mut v: Vec<_> = (0..10).map(|_| flat(100.0)).collect();
        v.extend([flat(97.0), (97.0, 99.5, 96.5, 98.0)]); // bar 11: high 99.5 >= 98.94
        v.extend([flat(98.0), flat(98.0), flat(98.0), flat(98.0), flat(98.0), flat(98.0), flat(98.0), flat(98.0)]);
        v.extend([flat(90.0), flat(90.0), flat(90.5), flat(95.0), flat(99.0)]); // bar 20 breaks down
        let r = bars(&v);
        let t = go(run, ANTI_TURTLE, &r, &[("pb_pct", 2.0), ("n_candles", 2.0)]);
        assert_eq!((t[0].0, t[0].2), (10, 11));
        assert!((t[0].3 - 98.94).abs() < 1e-9);
        assert_eq!((t[1].0, t[1].2), (20, 22));
        // bar 22 range 89.5..91.5: 98.94 is above the high, so the fill is the high
        assert_eq!(t[1].3, 91.5);
    }

    #[test]
    fn exit_on_a_new_entry_signal_continues_the_trade() {
        // bar 10 buys at 97; bar 11 closes 95 < previous lows (another entry signal) on the 1-candle exit -> continue;
        // bar 12 exit (not an entry signal) sells: one trade from bar 10 to bar 12
        let mut v: Vec<_> = (0..10).map(|_| flat(100.0)).collect();
        v.extend([flat(97.0), flat(95.0), flat(96.0), flat(97.0)]);
        let t = go(run, ANTI_TURTLE, &bars(&v), &[("n_candles", 1.0)]);
        assert_eq!(t.len(), 1);
        assert_eq!((t[0].0, t[0].2), (10, 12));
    }

    #[test]
    fn every_strategy_runs() {
        smoke(run, &IDS);
        let r = wave(400);
        let x = Ctx::new(&r, &r, &[]);
        let all = ps(&[("flt_type", 2.0), ("pb_pct", 3.0), ("hsl_pct", 2.0), ("tsl_pct", 2.0), ("rsi_thr_exit", 1.0), ("rsi_thr", 60.0),
            ("rsi_trail_exit", 1.0), ("st_exit", 1.0), ("n_green", 4.0), ("buffer_pct", 0.5)]);
        for t in run(ANTI_TURTLE, &x, 0, r.len() - 1, &all).unwrap() {
            assert!(t.exit_px >= x.l[t.exit_i] - 1e-9 && t.exit_px <= x.h[t.exit_i] + 1e-9);
        }
        let t = run(ANTI_TURTLE, &x, 0, r.len() - 1, &ps(&[("flt_type", 3.0), ("exit_at", 1.0), ("hsl_pct", 2.0)])).unwrap();
        assert!(t.iter().all(|t| t.exit_i >= t.legs[0].i));
    }
}
