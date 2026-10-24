//! Linda Bradford Raschke, "Trading Techniques 2008 - One Day Workshop Manual" (LBRGroup). Book id "lbrtt".
//! Only the two rule sets the manual states completely in mechanical terms are coded. Everything else in the
//! manual (Taylor Buy/Sell days, 3/10 oscillator patterns, Grail, tape reading, Ticks/TRIN, first-hour
//! breakouts with undefined qualifiers, random-entry studies) is listed in library/lbrtt/README.md with the
//! reason it is not testable or is already in the vault (the 2-period ROC flip = ss_2_period_roc).
//! India mapping: instrument swap only (NIFTY 50 etc. instead of the futures the manual tested).
#![allow(dead_code)]
use crate::connors::*;

pub const BOOK: &str = "lbrtt";
/// Admitted ids (authors/raschke/admission/SOURCE-CONTRACT.md).
pub const REVIEWED: &[&str] = &["lbrtt_wide_range_reversal", "lbrtt_extended_run"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const WRR: &[&str] = &[
    "F|(PDF p49, \"Trading Pattern: Wide Range Reversal System\") \"Identify the widest range of the last four days. If the bar closes up from the previous day's close, only a sell can be taken.\"",
    "E|\"Enter a sell stop at the low of the wide-range bar. The signal can only be taken over the next 2-day window. If no trade is entered within 2 days, the order is cancelled.\"",
    "X|\"If a trade is entered, trail a 7-bar channel stop (one that is based on the highest high of the last 7 days).\"",
    "E|\"All rules are reversed for a buy.\" (bar closes down from the previous close -> buy stop at the high of the wide-range bar; stop = lowest low of the last 7 days)",
    "F|[Book silent, read] range = high - low; the bar's range must be >= each of the 3 prior ranges (ties count as widest); an unchanged close gives no signal.",
    "F|[Book silent, read] one position at a time; the newest setup replaces a pending one; orders work only on bars that start flat; stop/channel orders are placed at the close and work from the next bar (TradeStation 'next bar ... stop'); a gap through a stop fills at the open.",
];

const XRUN: &[&str] = &[
    "F|(PDF p137, slide 102 \"Profile of an Extended Run\") \"we will examine consecutive closes on one side of a 5 period simple moving average. 7 closes on one side of this SMA are considered to be significant.\"",
    "E|\"entering a trade on the first close on the opposite side of the SMA\" (after >= 7 closes above the SMA5: buy on the close of the first close below it; mirror for shorts) - a retracement trade in the direction of the run (slide 103 \"Retracement Trades after 'Extended Run'\")",
    "X|\"and exiting it on the first close that is in the direction of the original trend\" (default read: first close above the previous close for a long, below for a short; exit on that close)",
    "F|[Book silent, read] above = close > SMA5, below = close < SMA5, a close equal to the SMA (within 1e-9 relative) ends the run; no stop is given so none is used; no re-entry on the exit bar.",
];

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, group, rules, notes, claim| StrategyDef {
        id, name, group, session: 0, dir: "Long/Short", status: Status::Live, needs_volume: false, needs_vix: false,
        needs_series: &[], book: BOOK, rules, notes, claim,
    };
    vec![
        d("lbrtt_wide_range_reversal", "Wide Range Reversal System", "When the market goes where it shouldn't go (PDF p49)", WRR,
          "Raschke: \"profitable in just about every single market, every single year since 1985 (the start of our testing)\"; Nelson Freeburg's update to 1980 was \"not as impressive\" and she \"would never suggest trading this pattern on a mechanical basis\" (uses it as an indicator). No trade counts printed. Tested in the book on futures; here on the cash indices (instrument swap only).",
          claim("-", "-", "-", "until 7-bar channel stop", "futures 1985-~1995 (no figures printed)")),
        d("lbrtt_extended_run", "Extended Run retracement (5-SMA)", "Modeling Extended Runs (PDF p137)", XRUN,
          "Printed profile (slide 102): \"%Win = +70%; average loss ~ 2x average win; profitable in 90 - 95% of a basket of 22 markets on any given year.\" Tested on 22 futures markets; here on the cash indices. exit_mode 1 (close back on the trend side of the SMA) is a labelled non-default alternative reading.",
          claim("-", "~70%", "avg loss ~2x avg win", "until first trend-direction close", "22 futures markets (period not printed)")),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match id {
        "lbrtt_wide_range_reversal" => Some(("lbrtt_wrr", "Wide Range Reversal (Raschke)", "")),
        "lbrtt_extended_run" => Some(("lbrtt_xrun", "Extended Run retracement (Raschke)", "")),
        _ => None,
    }
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    match id {
        "lbrtt_wide_range_reversal" => Some(vec![
            p("range_n", "Widest range of the last N days", 4.0, 3.0, 10.0, 1.0),
            p("window", "Stop order valid for N days", 2.0, 1.0, 5.0, 1.0),
            p("chan", "Trailing channel stop: N-bar highest high / lowest low", 7.0, 3.0, 20.0, 1.0),
        ]),
        "lbrtt_extended_run" => Some(vec![
            p("sma_n", "SMA period", 5.0, 3.0, 20.0, 1.0),
            p("run_n", "Consecutive closes on one side (at least)", 7.0, 3.0, 15.0, 1.0),
            p("exit_mode", "Exit: 0 = first close in trend direction vs prior close (book read), 1 = first close back across the SMA (alternative read)", 0.0, 0.0, 1.0, 1.0),
        ]),
        _ => None,
    }
}

fn open_trade(dir: f64, i: usize, px: f64, kind: &'static str) -> Trade {
    Trade { dir, legs: vec![Leg { i, px, w: 1.0 }], exit_i: i, exit_px: px, reason: String::new(), entry_kind: kind, signal_i: i, open: false, exits: vec![] }
}

fn close_trade(mut t: Trade, i: usize, px: f64, reason: &str) -> Trade {
    t.exit_i = i;
    t.exit_px = px;
    t.signal_i = i;
    t.reason = reason.into();
    t
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    if from > to || to >= x.r.len() {
        return None;
    }
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).unwrap().def);
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let mut out = vec![];
    if id == "lbrtt_wide_range_reversal" {
        let (rn, win, ch) = (g("range_n").round().max(2.) as usize, g("window").round().max(1.) as usize, g("chan").round().max(1.) as usize);
        let start = from.max(rn).max(ch);
        // pending: (dir, stop price, last bar the order works)
        let mut pending: Option<(f64, f64, usize)> = None;
        let mut cur: Option<Trade> = None;
        for i in start..=to {
            if let Some(t) = cur.as_ref() {
                if i > t.legs[0].i {
                    let (hit, px) = if t.dir < 0. {
                        let s = h[i - ch..i].iter().copied().fold(f64::NEG_INFINITY, f64::max);
                        (h[i] >= s, o[i].max(s))
                    } else {
                        let s = l[i - ch..i].iter().copied().fold(f64::INFINITY, f64::min);
                        (l[i] <= s, o[i].min(s))
                    };
                    if hit {
                        out.push(close_trade(cur.take().unwrap(), i, px, &format!("{ch}-bar channel stop")));
                    }
                }
            } else if let Some((dir, stop, last)) = pending {
                if i > last {
                    pending = None;
                } else if dir < 0. && l[i] <= stop {
                    cur = Some(open_trade(-1., i, o[i].min(stop), "stop"));
                    pending = None;
                } else if dir > 0. && h[i] >= stop {
                    cur = Some(open_trade(1., i, o[i].max(stop), "stop"));
                    pending = None;
                }
            }
            // setup on this bar's close (order works on the next `win` bars)
            let r = h[i] - l[i];
            if (i - rn + 1..i).all(|k| r >= h[k] - l[k]) {
                if c[i] > c[i - 1] {
                    pending = Some((-1., l[i], i + win));
                } else if c[i] < c[i - 1] {
                    pending = Some((1., h[i], i + win));
                }
            }
        }
        if let Some(t) = cur {
            let mut t = close_trade(t, to, c[to], "open");
            t.open = true;
            out.push(t);
        }
    } else if id == "lbrtt_extended_run" {
        let (sn, rn, mode) = (g("sma_n").round().max(1.) as usize, g("run_n").round().max(1.) as usize, g("exit_mode").round() as i32);
        let ma = sma(c, sn);
        // a close within 1e-9 x SMA of the SMA is a tie (running-sum float noise, as in hpetf.rs)
        let side = |i: usize| if !ma[i].is_finite() || (c[i] - ma[i]).abs() <= 1e-9 * ma[i].abs() { 0 } else if c[i] > ma[i] { 1 } else { -1 };
        // run[i] = signed length of the run of same-side closes ending at i
        let mut run = vec![0i64; c.len()];
        for i in 0..c.len() {
            let s = side(i);
            run[i] = if s == 0 { 0 } else if i > 0 && run[i - 1].signum() == s as i64 { run[i - 1] + s as i64 } else { s as i64 };
        }
        let mut cur: Option<Trade> = None;
        for i in from.max(1)..=to {
            if let Some(t) = cur.as_ref() {
                let hit = if mode == 1 { side(i) as f64 == t.dir } else { (c[i] - c[i - 1]) * t.dir > 0. };
                if hit {
                    let why = if mode == 1 { "close back across the SMA" } else { "first close in the trend's direction" };
                    out.push(close_trade(cur.take().unwrap(), i, c[i], why));
                }
                continue;
            }
            let s = side(i);
            if s != 0 && run[i - 1].abs() >= rn as i64 && run[i - 1].signum() == -(s as i64) && i < to {
                // closes were on the -s side for >= run_n bars; today closed on the other side: trade with the run
                cur = Some(open_trade(-(s as f64), i, c[i], "close"));
            }
        }
        if let Some(t) = cur {
            let mut t = close_trade(t, to, c[to], "open");
            t.open = true;
            out.push(t);
        }
    } else {
        return None;
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    fn rows(ohlc: &[(f64, f64, f64, f64)]) -> Vec<Row> {
        ohlc.iter().enumerate().map(|(i, &(o, h, l, c))| Row { ticker: "T".into(), date: format!("{:04}-01-01", 2000 + i), open: o, high: h, low: l, close: c, volume: None }).collect()
    }
    #[test]
    fn wrr_sells_low_of_up_closing_wide_bar_and_trails_7_bar_high() {
        let mut v = vec![(100., 101., 99., 100.); 10];
        v.push((100., 106., 98., 103.)); // i=10: widest of 4, closes up -> sell stop 98 for bars 11-12
        v.push((102., 104., 99., 100.)); // 11: low 99 > 98, no fill
        v.push((99., 99.5, 97., 97.5)); // 12: fills at 98 (open 99 above stop)
        v.push((97., 98., 95., 96.)); // 13: stop = max high 6..12 = 106 not hit
        v.push((96., 107., 95., 106.)); // 14: high 107 >= 106 -> out at 106
        let r = rows(&v);
        let x = Ctx::new(&r, &r, &[]);
        let t = run("lbrtt_wide_range_reversal", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert_eq!(t.len(), 1);
        assert_eq!((t[0].dir, t[0].legs[0].i, t[0].legs[0].px, t[0].exit_i, t[0].exit_px), (-1., 12, 98., 14, 106.));
    }
    #[test]
    fn extended_run_buys_first_close_below_sma_after_7_above_and_exits_first_up_close() {
        let mut v: Vec<(f64, f64, f64, f64)> = (0..14).map(|i| { let c = 100. + i as f64; (c, c, c, c) }).collect();
        v.push((105., 105., 105., 105.)); // 14: below SMA5 after a long run above -> buy 105
        v.push((104., 104., 104., 104.)); // 15: down close, hold
        v.push((106., 106., 106., 106.)); // 16: up close -> exit 106
        let r = rows(&v);
        let x = Ctx::new(&r, &r, &[]);
        let t = run("lbrtt_extended_run", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert_eq!(t.len(), 1);
        assert_eq!((t[0].dir, t[0].legs[0].i, t[0].legs[0].px, t[0].exit_i, t[0].exit_px), (1., 14, 105., 16, 106.));
    }
}
