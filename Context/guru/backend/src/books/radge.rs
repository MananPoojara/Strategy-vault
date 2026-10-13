//! Nick Radge: "Nick Radge's Trading Course" (.doc, c.2000), "Successful Stock Trading" (2012 e-book),
//! "The Futures Markets - Lessons From An Insider" (Reef Capital course, 1999-2001; contract mechanics only, no systems).
//! Only rules the texts state exactly are Live. Sources, page refs and the independent oracle: library/radge/.
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "radge";

const PAT: &str = "chart pattern drawn by eye (swing points / trendlines / consolidation boundaries are not defined by any formula in the course)";

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, group, dir, status, rules, notes| StrategyDef {
        id, name, group, session: 0, dir, status, needs_volume: false, needs_vix: false,
        needs_series: &[], book: BOOK, rules, notes, claim: None,
    };
    let nt = Status::NotTestable;
    vec![
        d("radge_ma220", "2/20 Moving Average Breakout", "Trading Course · Trend Breakout", "Long/Short", Status::Live,
          &["F|\"we shall use a 20-day simple moving average\" (Moving Average Breakout, The Setup)",
            "F|Filter 1: \"In an uptrend, prices must move up through the moving average and post 2 lows above the average. In a downtrend, prices must down through the moving average and post 2 highs below the average.\"",
            "E|Filter 2 = entry: \"taking the highest high of those two day's, we add on 10 ticks ... This is where we place the order to buy on stop.\" Short: \"taking the lowest low of those two days, subtract 10 ticks ... an order to sell on stop.\"",
            "X|\"Place the initial stop at the moving average line or 1 tick on the other side.\"",
            "X|\"the exit will be a trailing stop using the exact same moving average ... in an uptrend, place the trailing stop exit 1 tick back below the moving average. In a downtrend, place the stop 1 tick above the moving average.\"",
            "X|[Book silent, neutral] Stop level for a bar = prior close's SMA -/+ 1 tick (orders are known before the open; the course's EMA stop uses \"the closing price of the day prior\"). Gap through a stop fills at the open; a stop touched on the entry bar counts as hit (conservative). Entry stops are re-issued each bar while the last two lows (highs) are above (below) the SMA; one position at a time, entries only when flat."],
          "Course text p. 'Moving Average Breakout' (Examples 40-50). Tick: the book's '10 ticks' uses the instrument's minimum move; here tick = 0.05 for NSE indices (NSE tick), 0.01 otherwise (index quote step); 'tick' param overrides. The book's only result is TradeStation 'Mov Avg X 2/20' on CL 63/00 crude-oil futures, Oct 1998-Mar 2000, 1 contract: 51 trades, 49.02% win, net $6,000, PF 1.40, avg win/loss 1.45 (Example 50) - a futures contract the vault does not have, so it is not reproduced. The course's 'From My Experience' note mentions a 20-day EMA; the rule text says simple, which is the default. Run on the cash index (vault convention); shorting a cash index is hypothetical."),
        d("radge_oc_long", "Buy every open, sell the close", "Successful Stock Trading · Ch.1", "Long", Status::Live,
          &["E|\"I then told the computer to buy at the open of every single day - all 1353 of them -\"",
            "X|\"and sell on the close each night.\""],
          "SST printed p.16 (Ch.1 'The win/loss ratio'). Book: 'a major stock index over a 5½ year period' (unnamed; index +41.5%): 1353 trades, 53.66% profitable, avg win/avg loss 0.96, PF 1.11. The index is not named, so the result cannot be matched; run on NIFTY 50 / SPX as an instrument swap. Days with no real open in the data (open = close) give flat trades."),
        d("radge_oc_long_stop", "Buy every open, sell the close, N-point stop", "Successful Stock Trading · Appendix A", "Long", Status::Live,
          &["E|\"The computer was told to buy at every open\"",
            "X|\"and close at the end of each day - or get stopped out at the various protective stop-loss levels.\" \"The number of points used as the protective stop was tested between one and 50\""],
          "SST Appendix A, printed p.58-60. The book tabulates every stop from 1 to 50 points and names no single one; default 1 = the table's first row, stop_pts 1..50 reproduces the other rows. Stop = open - N index points (points of the instrument run, literally; the book's instrument is unnamed, likely an Australian index contract, so its point scale is not NIFTY's or SPX's). Stop filled at its level (entry is at the open, so no gap through)."),
        d("radge_oc_short_stop", "Short every open, cover the close, N-point stop", "Successful Stock Trading · Appendix B", "Short", Status::Live,
          &["E|\"short selling on every open,\"",
            "X|\"closing the trade at the end of each day, or getting stopped out at the various protective stop-loss levels.\" (stops 1 to 50 points)"],
          "SST Appendix B, printed p.61-63: 2506 trades per row on the unnamed instrument. Default 1 = first tabulated row; stop = open + N points. Cash-index shorting is hypothetical."),
        d("radge_triangle", "Triangles", "Trading Course · Trend Continuation", "Long/Short", nt(PAT),
          &["F|30-day EMA higher today than yesterday (uptrend)", "F|4 distinct internal swings: 1 high, 2 retracement, 3 fails to clear 1, 4 falls short of 2",
            "E|Buy stop 1 tick outside the 1-3 trend line", "X|Initial stop just below the low of swing 4; trailing stop exit"],
          "Swings and the 1-3 trendline are identified on the chart; no swing definition is given, and the trailing-stop exit is left to the reader."),
        d("radge_flag", "Flags", "Trading Course · Trend Continuation", "Long/Short", nt(PAT),
          &["F|30-day EMA rising; 4 swings, swing 4 lower than swing 2", "E|Stop 1 tick outside the 1-3 line", "X|Stop just below swing 4; trailing stop"],
          "Same swing-count pattern as the Triangle; swings drawn by eye."),
        d("radge_cont_gap", "Continuation Gap", "Trading Course · Trend Continuation", "Long/Short",
          nt("needs judgement: gap must be 'during a sustained trend' and 'not a part of a pullback or retracement ... such as in a consolidation area'; trend 'if it's not already obvious'"),
          &["F|Gap away from preceding bar P in trend direction; filled within 4 days; P low not breached", "E|Stop 1 tick above the bar that fills the gap, lowered with each lower high", "X|Initial stop 1 tick below the entry bar; trailing stop"],
          "The 4-day fill and P-bar rules are mechanical but the qualifying gap needs a discretionary trend/consolidation call; exit is an unspecified trailing stop."),
        d("radge_smash", "Smash Bars", "Trading Course · Trend Momentum", "Long/Short",
          nt("trend EMA length is given only as 'somewhere around 10 to 15 days'; no exact value"),
          &["F|Trend: EMA pointing up/down, length 'somewhere around 10 to 15 days'", "F|Uptrend: close in the lowest quartile of the day (downtrend: highest quartile)",
            "E|Next day only: buy stop 1 tick above the Smash bar high (short: 1 tick below its low)", "X|Initial stop 1 tick beyond the other end of the Smash bar", "X|Exit on the first lower low (short: first higher high)"],
          "Everything but the EMA length is mechanical; it becomes Live the moment one exact length is chosen by the user (would then be labelled as not from the book)."),
        d("radge_swing_target", "Swing Target Trading", "Trading Course · Trend Momentum", "Long/Short",
          nt("swing high/low points (HP/LP/R) are chart-identified; 'a strong trend or thrust' is required; target/risk filter '2.5 to 3 times'"),
          &["F|Retracement of at least 30% of the prior swing", "E|Buy stop 1 tick above bar R", "X|Stop 1 tick below R; target T = HP + 0.618 x (HP - R)"],
          "Swings 'take anywhere from 2 to 15 days' - no rule for picking them."),
        d("radge_spring", "Springs", "Trading Course · Trend Momentum", "Long/Short",
          nt("'a large sideways consolidation area' of at least 10 days is drawn by eye; no boundary formula"),
          &["F|False break of a >=10-day consolidation that closes back inside", "E|Buy stop 1 tick above the Spring day", "X|Stop 1 tick below the Spring bar; Exposure Stop exit"], ""),
        d("radge_divergence", "Type A Divergence", "Trading Course · Trend Reversal", "Long/Short",
          nt("peaks/lows are chart-picked ('should not be more than 8 to 15 days apart'); exit chosen 'based on the price action after the entry'"),
          &["F|Price higher secondary peak, 14-bar Slow Stochastic lower peak", "E|Sell stop 1 tick below the setup bar low", "X|Stop 1 tick above the setup bar high"], ""),
        d("radge_wedge", "Rising/Falling Wedge", "Trading Course · Trend Reversal", "Long/Short", nt(PAT),
          &["F|5 internal swings forming a wedge after a sustained trend", "E|Aggressive: stop 1 tick beyond point-5 bar; passive: break of the 2-4 trendline", "X|Stop 1 tick beyond point 5; trailing stop"], "'Ensure that the Wedge shape is apparent.'"),
        d("radge_spike_ledge", "Spike and Ledge", "Trading Course · Trend Reversal", "Long/Short", nt(PAT),
          &["F|Climax wide-range bar then a 5-15 day Ledge", "E|Stop 1 tick beyond the Ledge boundary", "X|Stop 1 tick beyond the opposite boundary; longer-term trailing exit"], ""),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "radge_ma220" => ("radge_ma220", "2/20 MA breakout", "Long/Short"),
        "radge_oc_long" => ("radge_oc", "Open-to-close day trade", "Long · no stop (Ch.1)"),
        "radge_oc_long_stop" => ("radge_oc", "Open-to-close day trade", "Long · point stop (App. A)"),
        "radge_oc_short_stop" => ("radge_oc", "Open-to-close day trade", "Short · point stop (App. B)"),
        _ => catalogue().into_iter().find(|d| d.id == id).map(|d| (d.id, d.name, "Not testable"))?,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    Some(match id {
        "radge_ma220" => vec![p("ma", "Simple moving average (days)", 20., 10., 60., 5.), p("days", "Lows/highs beyond the MA (days)", 2., 1., 4., 1.),
                              p("ticks", "Entry ticks beyond the 2-day high/low", 10., 0., 20., 1.),
                              p("tick", "Tick size (0 = instrument: 0.05 NSE, 0.01 other)", 0., 0., 1., 0.01)],
        "radge_oc_long" => vec![],
        "radge_oc_long_stop" | "radge_oc_short_stop" => vec![p("stop_pts", "Protective stop (index points; book table 1-50)", 1., 1., 50., 1.)],
        _ => return None,
    })
}

fn trade(dir: f64, i: usize, px: f64, kind: &'static str) -> Trade {
    Trade { dir, legs: vec![Leg { i, px, w: 1. }], exit_i: i, exit_px: px, reason: String::new(), entry_kind: kind, signal_i: i, open: false, exits: vec![] }
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    if from > to || to >= x.r.len() { return None; }
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(0., |d| d.def));
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let mut out = vec![];
    match id {
        "radge_oc_long" | "radge_oc_long_stop" | "radge_oc_short_stop" => {
            let stop = if id == "radge_oc_long" { f64::INFINITY } else { g("stop_pts") };
            let dir = if id == "radge_oc_short_stop" { -1. } else { 1. };
            for i in from..=to {
                let mut t = trade(dir, i, o[i], "open");
                let lvl = o[i] - dir * stop;
                let hit = if dir > 0. { l[i] <= lvl } else { h[i] >= lvl };
                (t.exit_px, t.reason) = if hit { (lvl, format!("{stop} pt stop")) } else { (c[i], "close".into()) };
                out.push(t);
            }
        }
        "radge_ma220" => {
            let ma = x.sma_n(g("ma") as usize);
            let days = (g("days") as usize).max(1);
            let tick = if g("tick") > 0. { g("tick") } else if x.r[0].ticker.to_uppercase().contains("NIFTY") { 0.05 } else { 0.01 };
            let off = g("ticks") * tick;
            let mut cur: Option<Trade> = None;
            // pending entry order for bar i: (dir, stop level)
            let mut pend: Option<(f64, f64)> = None;
            for i in from..=to {
                if let Some((dir, lvl)) = pend.take() {
                    let fill = if dir > 0. { (h[i] >= lvl).then(|| o[i].max(lvl)) } else { (l[i] <= lvl).then(|| o[i].min(lvl)) };
                    if let Some(px) = fill { cur = Some(trade(dir, i, px, "stop")); }
                }
                if let Some(t) = cur.as_mut() {
                    // prior close's MA, 1 tick beyond; rounded to 1e-8 so a touch is not lost to rolling-sum float noise
                    let stop = ((ma[i - 1] - t.dir * tick) * 1e8).round() / 1e8;
                    let entry_bar = t.legs[0].i == i;
                    let px = if t.dir > 0. {
                        if !entry_bar && o[i] <= stop { Some(o[i]) } else if l[i] <= stop { Some(stop) } else { None }
                    } else if !entry_bar && o[i] >= stop { Some(o[i]) } else if h[i] >= stop { Some(stop) } else { None };
                    if let Some(px) = px {
                        t.exit_i = i; t.exit_px = px; t.signal_i = i;
                        t.reason = if entry_bar { "MA stop (entry bar)".into() } else { "MA trailing stop".into() };
                        out.push(cur.take().unwrap());
                    }
                }
                if cur.is_none() && i < to && i + 1 >= days {
                    let w = i + 1 - days..=i;
                    if w.clone().any(|k| !ma[k].is_finite()) { continue; }
                    if w.clone().all(|k| l[k] > ma[k]) {
                        pend = Some((1., w.clone().map(|k| h[k]).fold(f64::MIN, f64::max) + off));
                    } else if w.clone().all(|k| h[k] < ma[k]) {
                        pend = Some((-1., w.map(|k| l[k]).fold(f64::MAX, f64::min) - off));
                    }
                }
            }
            if let Some(mut t) = cur { t.exit_i = to; t.exit_px = c[to]; t.signal_i = to; t.reason = "open".into(); t.open = true; out.push(t); }
        }
        _ => return None,
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    fn rows(n: usize) -> Vec<Row> {
        (0..n).map(|i| Row { ticker: "TEST".into(), date: format!("{:04}-01-01", 1900 + i), open: 100., high: 101., low: 99., close: 100., volume: None }).collect()
    }
    #[test]
    fn ma220_enters_on_stop_above_two_day_high_and_trails_prior_ma() {
        let mut r = rows(40);
        // bars 25,26 lows above SMA20 (~100.x); highs 104,105 -> buy stop 105 + 10*0.01 = 105.10 on bar 27
        for (i, (lo, hi, cl)) in [(101., 104., 103.), (102., 105., 104.), (104., 107., 106.)].iter().enumerate() {
            r[25 + i].low = *lo; r[25 + i].high = *hi; r[25 + i].close = *cl; r[25 + i].open = *lo + 0.5;
        }
        for k in 28..40 { r[k].open = 106.; r[k].high = 107.; r[k].low = 105.; r[k].close = 106.; }
        r[33].open = 90.; r[33].low = 89.; // gap below the MA stop -> exit at open
        let x = Ctx::new(&r, &r, &[]);
        let ts = run("radge_ma220", &x, 20, 39, &Params::new()).unwrap();
        assert_eq!(ts[0].legs[0].i, 27);
        assert!((ts[0].legs[0].px - 105.10).abs() < 1e-9);
        assert_eq!(ts[0].exit_i, 33);
        assert_eq!(ts[0].exit_px, 90.);
    }
    #[test]
    fn open_close_stop_fills_at_level() {
        let mut r = rows(3);
        r[1].low = 99.5; r[1].close = 100.7;
        let x = Ctx::new(&r, &r, &[]);
        let ts = run("radge_oc_long_stop", &x, 0, 2, &Params::new()).unwrap();
        assert_eq!(ts.len(), 3);
        assert_eq!(ts[0].exit_px, 99.); // low touches open - 1 -> stopped at 99
        assert_eq!(ts[1].exit_px, 100.7); // stop untouched -> close
        let s = run("radge_oc_short_stop", &x, 0, 2, &Params::from([("stop_pts".into(), 2.)])).unwrap();
        assert_eq!(s[0].exit_px, 100.); // high 101 < 102 -> close
    }
    #[test]
    fn catalogue_ids_unique_and_prefixed() {
        let c = catalogue();
        let mut ids: Vec<_> = c.iter().map(|d| d.id).collect();
        ids.sort(); ids.dedup();
        assert_eq!(ids.len(), c.len());
        assert!(c.iter().all(|d| d.id.starts_with("radge_") && family_of(d.id).is_some()));
        assert_eq!(c.iter().filter(|d| d.status == Status::Live).count(), 4);
    }
}
