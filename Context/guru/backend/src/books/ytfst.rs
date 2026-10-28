//! YT Strategies · Fresh ST Buy: the team's AmiBroker AFLs in "Old Topics/Fresh ST Buy Next Day High-Close". Only the
//! basic version trades an index on daily bars without volume; the other versions need Volume > MA(Volume) and are not
//! coded. Coded from the AFL as written, with its default settings; there is no book PDF.
//!
//! AmiBroker behaviour followed: the Supertrend is the team's ST loop (ATR(1) Null on bar 0, bands Null until the
//! period's ATRs exist, trend held while they are Null), computed on the filter timeframe and expanded with
//! expandLast; the strategy loop runs from bar 1 over the full history; the sell price the AFL sets is kept inside the
//! bar's low-high range (AmiBroker's price bound check); trades are entered only in the test window.
#![allow(dead_code)]
use crate::books::ytich::{backtest, Sigs};
use crate::books::ytsw::st_filter;
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "yt_fst";
pub const BASIC: &str = "ytfst_basic";
pub const IDS: [&str; 1] = [BASIC];

fn def(id: &'static str, name: &'static str, rules: &'static [&'static str], notes: &'static str) -> StrategyDef {
    StrategyDef { id, name, group: "YT Strategies · Fresh ST Buy", session: 0, dir: "Long", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim: None }
}

pub fn catalogue() -> Vec<StrategyDef> {
    vec![def(BASIC, "Fresh ST Buy, Next-Day Close Above Setup High (Basic)", &[
        "F|Supertrend of the series (default daily, period 5, factor 4, on the mid point (H+L)/2)",
        "F|A fresh up phase: on the first up-trend bar while flat, its high becomes the setup high and the bar count starts at 1; each further up-trend bar adds 1; a down-trend bar cancels the setup",
        "E|Buy at the close when the setup is armed, the count is at most the entry count (default 2) and the close is above the setup high (and the bar's high is not its close)",
        "X|Trailing stop: the highest high since entry less the trail % (default 3); the low reaching the stop sells at the stop, or at the open less the selling buffer % (0.25) when the open is already below it; no exit on a bar whose open, low and close are equal (lower circuit)",
        "F|After an exit no new trade until the Supertrend has turned down and a new up phase starts",
    ], "Fresh ST Buy Next Day Close_Basic Version (Trash/Basic Version (Same as Close)), coded as the AFL is written. The stop tested on a bar uses the highest high up to the previous bar (the AFL updates the stop after the test). With trail % 0 the AFL never sets a stop, so a trade never exits (as written; not the default). The volume-filter versions of this family are not coded (index data has no volume).")]
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    match id {
        BASIC => Some(vec![
            p("entry_count", "Entry count (bars of the up phase allowed for entry)", 2.0, 1.0, 50.0, 1.0),
            p("trail_pct", "Trailing SL % below the highest high", 3.0, 0.0, 20.0, 0.001),
            p("buffer_pct", "Selling buffer % (gap fills)", 0.25, 0.0, 10.0, 0.05),
            p("st_tf", "ST timeframe (1 daily, 2 weekly, 3 monthly)", 1.0, 1.0, 3.0, 1.0),
            p("st_point", "ST calculation point (1 mid point, 2 close, 3 pivot)", 1.0, 1.0, 3.0, 1.0),
            p("st_period", "ST period", 5.0, 5.0, 25.0, 1.0),
            p("st_factor", "ST factor", 4.0, 1.0, 7.0, 1.0),
        ]),
        _ => None,
    }
}

struct Cfg {
    entry_count: f64,
    trail: f64,
    buf: f64,
}

/// The AFL's loop on a given (expanded) ST trend.
fn fst_signals(x: &Ctx, tr: &[i8], k: &Cfg) -> Sigs {
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let n = c.len();
    let mut s = Sigs::new(1.0, c);
    // AFL Prec(x, 2) truncates to 2 decimals
    let r2 = |v: f64| (v * 100.0).trunc();
    let lo_circuit = |i: usize| r2(o[i]) == r2(c[i]) && r2(o[i]) == r2(l[i]);
    let (mut pos, mut abc, mut count) = (false, 0u8, 0i64);
    let mut my_high = 0.0f64;
    let (mut hh, mut tsl) = (BIG, 0.0f64);
    for i in 1..n {
        if !pos && tr[i] == 1 && abc == 0 {
            my_high = h[i];
            count = 1;
            abc = 1;
        } else if !pos && count > 0 && tr[i] == 1 && abc == 1 {
            count += 1;
        } else if !pos && count > 0 && tr[i] == -1 && abc == 1 {
            abc = 0;
            my_high = 0.0;
            count = 0;
        } else if abc == 2 && tr[i] == -1 {
            abc = 0;
        }
        if !pos && abc == 1 && (count as f64) <= k.entry_count && c[i] > my_high && my_high > 0.0 && h[i] != c[i] && c[i] > 0.0 {
            s.entry[i] = true;
            if k.trail > 0.0 {
                hh = h[i];
                tsl = (1.0 - k.trail * 0.01) * hh;
            }
            my_high = 0.0;
            pos = true;
        } else if pos {
            if k.trail > 0.0 {
                hh = hh.max(h[i]);
            }
            if l[i] <= tsl && !lo_circuit(i) {
                s.exit[i] = true;
                s.exit_px[i] = if o[i] < tsl { o[i] - o[i] * k.buf / 100.0 } else { tsl };
                s.why[i] = "Trailing SL";
                pos = false;
                hh = BIG;
                count = 0;
                abc = 2;
            }
            if k.trail > 0.0 {
                tsl = (1.0 - k.trail * 0.01) * hh;
            }
        }
    }
    s
}

const BIG: f64 = 9_999_999.0;

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(0.0, |d| d.def));
    let n = x.c.len();
    if n == 0 || from > to || to >= n {
        return Some(Vec::new());
    }
    let tf = match g("st_tf").round() as i64 { 2 => 'W', 3 => 'M', _ => 'D' };
    let tr = st_filter(x, tf, g("st_point").round() as i64, g("st_period").round().max(1.0) as usize, g("st_factor"));
    let k = Cfg { entry_count: g("entry_count"), trail: g("trail_pct"), buf: g("buffer_pct") };
    Some(backtest(x, &fst_signals(x, &tr, &k), from, to))
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let d = catalogue().into_iter().find(|d| d.id == id)?;
    Some((d.id, d.name, "Fresh ST Buy Next Day High-Close"))
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
    /// Consecutive weekdays from Monday 2024-01-01; each bar is (open, high, low, close).
    fn bars(ohlc: &[(f64, f64, f64, f64)]) -> Vec<Row> {
        let mut d = day_number("2024-01-01").unwrap();
        ohlc.iter()
            .map(|&(open, high, low, close)| {
                let z = d;
                d += if (z + 4).rem_euclid(7) == 5 { 3 } else { 1 };
                Row { ticker: "T".into(), date: civil(z), open, high, low, close, volume: None }
            })
            .collect()
    }
    const K: Cfg = Cfg { entry_count: 2.0, trail: 3.0, buf: 0.25 };
    fn trades(r: &[Row], tr: &[i8], k: &Cfg) -> Vec<(usize, f64, usize, f64)> {
        let x = Ctx::new(r, r, &[]);
        backtest(&x, &fst_signals(&x, tr, k), 0, r.len() - 1).into_iter().map(|t| (t.legs[0].i, t.legs[0].px, t.exit_i, (t.exit_px * 1e6).round() / 1e6)).collect()
    }
    const B: (f64, f64, f64, f64) = (100.0, 101.0, 99.0, 100.0);
    fn sample() -> Vec<Row> {
        bars(&[
            B,
            B,                          // 1: ST down
            B,                          // 2: fresh up phase, setup high 101, count 1
            (101.0, 103.0, 100.5, 102.5), // 3: count 2, close 102.5 > 101 -> buy; stop 103 x 0.97 = 99.91
            (104.0, 106.0, 103.5, 105.5), // 4: low above the stop; then stop = 106 x 0.97 = 102.82
            (102.0, 103.0, 101.0, 101.5), // 5: low 101 <= 102.82, open 102 below it -> 102 less 0.25% = 101.745
            (103.0, 106.0, 102.0, 105.0), // 6: still up trend: no new trade before the ST turns down
        ])
    }

    #[test]
    fn buys_next_day_close_above_setup_high_and_trails() {
        let tr = [1, -1, 1, 1, 1, 1, 1];
        assert_eq!(trades(&sample(), &tr, &K), vec![(3, 102.5, 5, 101.745)]);
    }

    #[test]
    fn entry_count_limits_the_wait() {
        // the close beats the setup high only on the third bar of the up phase: count 3 > 2, no trade; count 3 allows it
        let mut r = sample();
        (r[3].open, r[3].high, r[3].low, r[3].close) = (101.0, 101.0, 99.5, 100.5);
        let tr = [1, -1, 1, 1, 1, 1, 1];
        assert!(trades(&r, &tr, &K).is_empty());
        let t = trades(&r, &tr, &Cfg { entry_count: 3.0, ..K });
        assert_eq!(t[0].0, 4);
    }

    #[test]
    fn fresh_phase_needed_after_exit() {
        // after the bar-5 exit the trend turns down on bar 7 and up on bar 8 (setup high 102): bar 9 closes 103 -> buy
        let mut v: Vec<(f64, f64, f64, f64)> = sample().iter().map(|r| (r.open, r.high, r.low, r.close)).collect();
        v.extend([B, (100.0, 102.0, 99.0, 101.0), (101.5, 104.0, 101.0, 103.0)]);
        let tr = [1, -1, 1, 1, 1, 1, 1, -1, 1, 1];
        let t = trades(&bars(&v), &tr, &K);
        assert_eq!(t.len(), 2);
        assert_eq!((t[1].0, t[1].1), (9, 103.0));
    }

    #[test]
    fn every_strategy_runs() {
        let ohlc: Vec<(f64, f64, f64, f64)> = (0..400)
            .map(|k| {
                let c = 1000.0 + 120.0 * ((k as f64) / 17.0).sin() + 40.0 * ((k as f64) / 5.0).cos() + k as f64 * 0.3;
                (c - 2.0, c + 8.0, c - 9.0, c)
            })
            .collect();
        let r = bars(&ohlc);
        let x = Ctx::new(&r, &r, &[]);
        for id in IDS {
            assert!(params(id).is_some() && family_of(id).is_some(), "{id}");
            for tf in [1.0, 2.0, 3.0] {
                let p: Params = [("st_tf".to_string(), tf)].into_iter().collect();
                let t = run(id, &x, 30, r.len() - 1, &p).unwrap();
                assert!(t.iter().all(|t| t.exit_i >= t.legs[0].i && t.legs[0].i >= 30), "{id}");
            }
            assert!(!run(id, &x, 30, r.len() - 1, &Params::new()).unwrap().is_empty(), "{id}");
        }
    }
}
