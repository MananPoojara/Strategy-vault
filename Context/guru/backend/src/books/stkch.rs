//! StockCharts.com ChartSchool articles, 1999-2001 ("Stock Charts Full Articles", research drive
//! Trading Knowledge II/@NewsLetters-Mags/Stock Charts Full Articles/*.pdf; copy in Book-to-rust/Books/StockCharts.com
//! ChartSchool (Stock Charts Full Articles)/). Book id `stkch`. Refs are article file + PDF page ("n of m" footer).
//! Most of the 80 articles are indicator / chart-pattern education with no complete rule set; only the systems with a
//! stated entry AND exit are Live. Stocks in the articles -> run on the index (instrument swap only).
//! Sources, checks and the list of not-testable articles: library/stkch/.
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "stkch";
/// Admitted ids (authors/newsletters/admission/SOURCE-CONTRACT.md).
pub const REVIEWED: &[&str] = &["stkch_ema_30_100", "stkch_ppo_20_60", "stkch_k39", "stkch_gap_fgd_long" /* lane K73, R4 #276 (2) */, "stkch_murphy_macd" /* lane R2A, R4 #297 */];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const EOD: &str = "E|Fill: the article gives no order type for these end-of-day chart signals; the signal is acted on at the close of the signal day [book silent]";
const EMA_DEF: &str = "F|EMA: \"The 10-period simple moving average is used for the first calculation only. After that the previous period's EMA is used\" (indic_movingAvg.pdf p.4), weight 2/(n+1)";
const GAP_TRAIL: &str = "X|\"Once a position is entered, you calculate and set an 8% trailing stop to exit a long position, and a 4% trailing stop to exit a short position\"; examples trail from the close: \"8% below the final close of $58.50\", \"trailing stop would be set at $80 (8% less than the $87 close)\" (gapStrategies1.pdf p.2-3)";
/// Lane K79 (R4 #290 (2)(3)): the first-hour stop entries, settled on the 1-minute bars.
const GAP_AFTER: &str = "E|The stop is placed \"after 10:30 am\": it is live from the first minute at or after the end of the first hour (US 10:30 ET; NSE 10:15 IST) on that day's real 1-minute bars and fills at its level, or at a minute's open when that minute opened beyond it (lane K79, decision #290); no minute reaching it, no entry; a day without minutes takes no entry";
const GAP_TICK: &str = "F|\"A 'tick' is defined as the bid/ask spread, usually 1/8 to 1/4 point\" (gapStrategies1.pdf p.2); here (decisions #57, #203, #276 (2)) 'two ticks' = 2 ticks of the traded instrument (US 0.01, NSE 0.05), labelled; option p.ticks = 0 places the stop at the level itself";

fn mk(dir: f64, i: usize, px: f64, sig: usize, kind: &'static str) -> Trade {
    Trade { dir, legs: vec![Leg { i, px, w: 1. }], exit_i: i, exit_px: px, reason: String::new(), entry_kind: kind, signal_i: sig, open: false, exits: vec![] }
}
fn fin(mut t: Trade, i: usize, px: f64, sig: usize, why: &str) -> Trade { t.exit_i = i; t.exit_px = px; t.signal_i = sig; t.reason = why.into(); t }
fn mark_open(out: &mut Vec<Trade>, cur: Option<Trade>, x: &Ctx, to: usize) {
    if let Some(mut t) = cur { t.exit_i = to; t.exit_px = x.c[to]; t.signal_i = to; t.reason = "open".into(); t.open = true; out.push(t); }
}

/// StockCharts EMA: seeded with the SMA of the first n values, weight 2/(n+1).
fn ema(v: &[f64], n: usize) -> Vec<f64> {
    let mut out = vec![f64::NAN; v.len()];
    if n == 0 || v.len() < n { return out; }
    let k = 2. / (n as f64 + 1.);
    let mut e = v[..n].iter().sum::<f64>() / n as f64;
    out[n - 1] = e;
    for i in n..v.len() { e += k * (v[i] - e); out[i] = e; }
    out
}

/// Signal-state driver: st[t] = +1 buy signal in force, -1 sell signal in force (decided on bar t's close).
/// A position opens/flips only on a change of state inside [from, to], filled at that close.
/// rev = false: long only (sell signal = exit); rev = true: sell signal also sells short.
fn states(x: &Ctx, from: usize, to: usize, st: &[i8], rev: bool, why: &str) -> Vec<Trade> {
    let mut out = vec![];
    let mut cur: Option<Trade> = None;
    for t in from.max(1)..=to {
        if st[t] == st[t - 1] || st[t] == 0 { continue; }
        if let Some(tr) = cur.take() { out.push(fin(tr, t, x.c[t], t, why)); }
        if st[t] > 0 || rev { cur = Some(mk(st[t] as f64, t, x.c[t], t, "close")); }
    }
    mark_open(&mut out, cur, x, to);
    out
}

/// Murphy's MACD law on a close series: +1 = cross up with both lines below zero, -1 = cross down with both above zero.
fn macd_signals(c: &[f64], fa: usize, sl: usize, sg: usize) -> Vec<i8> {
    let n = c.len();
    let (f, s) = (ema(c, fa), ema(c, sl));
    let m: Vec<f64> = (0..n).map(|t| f[t] - s[t]).collect();
    let mut sig = vec![f64::NAN; n];
    if let Some(a) = m.iter().position(|v| v.is_finite()) { for (t, v) in ema(&m[a..], sg).into_iter().enumerate() { sig[a + t] = v; } }
    (0..n).map(|t| {
        if t == 0 || !(sig[t].is_finite() && sig[t - 1].is_finite()) { return 0; }
        if m[t] > sig[t] && m[t - 1] <= sig[t - 1] && m[t] < 0. && sig[t] < 0. { 1 }
        else if m[t] < sig[t] && m[t - 1] >= sig[t - 1] && m[t] > 0. && sig[t] > 0. { -1 } else { 0 }
    }).collect()
}

fn days(d: &str) -> i64 {
    let (y, m, dd): (i64, i64, i64) = (d[0..4].parse().unwrap_or(1970), d[5..7].parse().unwrap_or(1), d[8..10].parse().unwrap_or(1));
    let y2 = if m <= 2 { y - 1 } else { y };
    let era = y2.div_euclid(400);
    let yoe = y2 - era * 400;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + dd - 1;
    era * 146097 + yoe * 365 + yoe / 4 - yoe / 100 + doy - 719468
}

/// Highest / lowest close of the previous week for every bar. cal = true: the previous calendar week (Mon-Sun) that
/// has bars; cal = false: the 5 bars before this one.
fn prev_week_closes(x: &Ctx, cal: bool) -> (Vec<f64>, Vec<f64>) {
    let n = x.c.len();
    let (mut hi, mut lo) = (vec![f64::NAN; n], vec![f64::NAN; n]);
    if !cal {
        for t in 5..n {
            let w = &x.c[t - 5..t];
            hi[t] = w.iter().copied().fold(f64::NEG_INFINITY, f64::max);
            lo[t] = w.iter().copied().fold(f64::INFINITY, f64::min);
        }
        return (hi, lo);
    }
    // 1970-01-05 was a Monday: week number = floor((days - 4) / 7)
    let wk: Vec<i64> = x.r.iter().map(|r| (days(&r.date) - 4).div_euclid(7)).collect();
    let (mut ph, mut pl) = (f64::NAN, f64::NAN); // previous week's extremes
    let (mut ch, mut cl) = (f64::NEG_INFINITY, f64::INFINITY); // current week so far
    for t in 0..n {
        if t > 0 && wk[t] != wk[t - 1] { ph = ch; pl = cl; ch = f64::NEG_INFINITY; cl = f64::INFINITY; }
        hi[t] = ph;
        lo[t] = pl;
        ch = ch.max(x.c[t]);
        cl = cl.min(x.c[t]);
    }
    (hi, lo)
}

/// Gap strategies (Scott McCormick, gapStrategies1/2.pdf). kind: 1 = full gap up (open > prior high),
/// -1 = full gap down (open < prior low). dir = trade direction. lvl(t) = the stop-entry price on the gap day
/// (None = data missing). One position at a time; entry stop is a day order on the gap day. Trailing stop from the
/// closes after entry, checked from the next bar (open beyond the stop fills at the open).
/// `after` = minutes past the session open from which the entry stop is live (0: from the open, on the daily bar; 60: after the
/// first hour, settled on the day's 1-minute bars only, lane K79).
fn gap(x: &Ctx, from: usize, to: usize, kind: i8, dir: f64, trail: f64, after: u16, order: bool, lvl: impl Fn(usize) -> Option<f64>) -> Vec<Trade> {
    let mut out = vec![];
    let mut cur: Option<(Trade, f64)> = None; // trade, best close (or entry) so far
    for t in from.max(1)..=to {
        let mut cut = None; // lane K75 (#240, R4 #281 (3)): the trailing stop level when it was hit inside this bar (not at the open)
        if let Some((tr, best)) = cur.take() {
            let stop = if dir > 0. { best * (1. - trail) } else { best * (1. + trail) };
            let hit = if dir > 0. { x.l[t] <= stop } else { x.h[t] >= stop };
            if hit {
                let px = if dir > 0. { x.o[t].min(stop) } else { x.o[t].max(stop) };
                if if dir > 0. { x.o[t] > stop } else { x.o[t] < stop } { cut = Some(stop); }
                out.push(fin(tr, t, px, t, if dir > 0. { "8% trailing stop" } else { "4% trailing stop" }));
            } else {
                let b = if dir > 0. { best.max(x.c[t]) } else { best.min(x.c[t]) };
                cur = Some((tr, b));
                continue;
            }
        }
        let g = if x.o[t] > x.h[t - 1] { 1 } else if x.o[t] < x.l[t - 1] { -1 } else { 0 };
        if g != kind { continue; }
        let Some(lv) = lvl(t) else { continue };
        if !lv.is_finite() { continue; }
        let cut = cut.filter(|_| order);
        let px = if after > 0 {
            // lane K79 (#290 (2)(3)): an order placed after the first hour is settled on the day's minutes only; no minutes, no entry
            let Some(b) = crate::intraday_w2b::day(x, t).filter(|b| !b.is_empty()) else { continue };
            match same_day(x, t, &b, dir, cut, lv, after) { Some(Some(p)) => p, Some(None) => continue, None => lv }
        } else {
            if !(if dir > 0. { x.h[t] >= lv } else { x.l[t] <= lv }) { continue; }
            match cut.and_then(|sp| crate::intraday_w2b::day(x, t).map(|b| same_day(x, t, &b, dir, Some(sp), lv, 0))) { Some(Some(Some(p))) => p, Some(Some(None)) => continue, _ => lv }
        };
        let tr = mk(dir, t, px, t, "stop");
        let best = if dir > 0. { px.max(x.c[t]) } else { px.min(x.c[t]) };
        cur = Some((tr, best));
    }
    mark_open(&mut out, cur.map(|c| c.0), x, to);
    out
}

/// The entry stop `lv` on the day's 1-minute bars `b` (intraday_w2b::day: R4 #285 skips the rejected NIFTY 50 minute days), live
/// from the first minute at or after `after` minutes past the session open (lane K79, #290 (2)(3)), filled at its level or at the
/// minute's open when it opened through it: Some(Some(fill)) = entered, Some(None) = not touched (no entry that day).
/// Lane K75 (#240, R4 #281 (3)): `stop` = the trailing stop when it was hit inside this bar, so the daily bar does not say whether
/// the exit or the new entry came first: the entry is live only from the minute after the stop's first minute. None = the stop is
/// not on the minutes (with `after` = 0) or the stop and the live entry fall in one minute (unknown order): the caller's daily
/// convention (out at the stop, then in at the entry level). With `after` > 0 a stop not on the minutes leaves the entry
/// on the live minutes.
fn same_day(x: &Ctx, t: usize, b: &[crate::intraday::Bar1], dir: f64, stop: Option<f64>, lv: f64, after: u16) -> Option<Option<f64>> {
    use crate::intraday::{first_touch, session_open, touched, Level, Touch};
    let (live, en) = (session_open(&x.r[t].ticker, &x.r[t].date) + after, Level { up: dir > 0., px: lv });
    let mut k = b.iter().position(|z| z.m >= live).unwrap_or(b.len());
    if let Some(sp) = stop {
        match first_touch(b, 0, &[Level { up: dir < 0., px: sp }]) {
            Touch::Hit { bar: e, .. } => {
                if b[e].m >= live && touched(&b[e], en) { return None; }
                k = k.max(e + 1);
            }
            _ if after == 0 => return None,
            _ => {}
        }
    }
    Some(match first_touch(b, k, &[en]) { Touch::Hit { px, .. } => Some(px), _ => None })
}

pub fn catalogue() -> Vec<StrategyDef> {
    use Status::*;
    let d = |id, name, group, dir, status, needs_series, rules, notes| StrategyDef { id, name, group, session: 0, dir, status, needs_volume: false,
        needs_vix: false, needs_series, book: BOOK, rules, notes, claim: None };
    vec![
        d("stkch_ema_30_100", "Arthur Hill 30/100 EMA crossover (Xircom)", "Trading Strategies · Arthur Hill On Moving Average Crossovers", "Long", Live, &[],
          &["E|\"For Xircom, a 30/100 exponential moving average crossover was used to generate signals. When the 30-day EMA moves above the 100-day EMA, a buy signal is in force.\" (AHillMAcrossover.pdf p.2)",
            "X|\"When the 30-day EMA declines below the 100-day EMA, a sell signal is in force.\" (p.2); \"an exit strategy based on the moving average crossover would have given back some of those profits\" (p.2) -> the sell signal closes the long",
            EMA_DEF, EOD,
            "F|p.rev=1 (labelled option, not the default): the sell signal also sells short (the general description p.1 says only 'a sell signal')"],
          "Long entries are taken on a change to a buy signal inside the test window (no entry on the first bar of a window that starts mid-signal). Article prints no statistics (chart only). Stock example -> index.", ),
        d("stkch_ppo_20_60", "Arthur Hill 20/60 EMA crossover with +/-2% PPO band (3Com)", "Trading Strategies · Arthur Hill On Moving Average Crossovers", "Long", Live, &[],
          &["F|\"a 20/60 EMA crossover system ... The plot below the price is the 20/60 EMA differential, which is shown as a percent ... Percentage Price Oscillator (PPO) set at (20,60,1)\" (AHillMAcrossover.pdf p.2-3); PPO = (EMA20 - EMA60) / EMA60 x 100",
            "E|\"the buy signal was set just above the zero line (at +2%) ... When the 20-day EMA is more than 2% above the 60-day EMA, a buy signal is in force.\" (p.3)",
            "X|\"the sell signal was set just below the zero line (at -2%) ... When the 20-day EMA is more than 2% below the 60-day EMA, a sell signal is in force.\" (p.3)",
            EMA_DEF, EOD,
            "F|p.rev=1 (labelled option): the sell signal also sells short"],
          "Between +2% and -2% the last signal stays in force. The article's remarks that 'tight stop-losses would have been required' and 'a trailing stop or use of the parabolic SAR might have helped' (p.3) are commentary on the 3Com chart with no size; nothing is added. No statistics printed.", ),
        d("stkch_k39", "The \"Last\" Stochastic: unsmoothed 39-period %K crossing 50 (Scott McCormick)", "Trading Strategies · The \"Last\" Stochastic Technique", "Long/Short", Live, &[],
          &["F|\"an unsmoothed 39 period stochastic oscillator (K = 39, no signal line)\"; K = (C - L) / (H - L) over n periods (lastStochastic.pdf p.1-2)",
            "E|\"A buy signal is generated when K crosses above 50% and the closing price is above the previous week's high close.\" (p.2)",
            "X|\"Sell and/or sell short signals are created when the K line crosses below 50% and the closing price is below the previous week's low close.\" (p.2); default = sell and sell short (always in after the first signal); p.mode=1 = sell only (long/flat)",
            "F|\"previous week's high close\" = highest close of the previous calendar week (p.wk=0); p.wk=1 (labelled option) = the 5 bars before the signal bar",
            "F|p.buy_lvl / p.sell_lvl (labelled options): \"most Tech stocks have evidenced a tendency to signal entry at a K crossover above 40% and a sell with K crossing below 60%\" (p.6)",
            EOD],
          "Signals only on the bar where K crosses the level AND the close condition holds the same day. 'Weekly, Daily and Hourly ... can all be used' (p.3): run on daily bars. The OBV confirmation (p.5-6) is a separate not-testable entry (stkch_k39_obv). Source study: Colby & Meyers, Encyclopedia of Technical Market Indicators (cf. mpmm_cm_stoch39w, the weekly rule as printed there; this article's daily rule uses the previous week's high/low close instead).", ),
        d("stkch_gap_fgd_long", "Full Gap Down: Long (buy stop 2 ticks over yesterday's low, 8% trailing stop)", "Trading Strategies · Gap Trading Strategies", "Long", Live, &[],
          &["F|\"A Full Gap Down occurs when the opening price is less than yesterday's low.\" (gapStrategies1.pdf p.1)",
            "E|\"If a stock's opening price is less than yesterday's low, set a long stop equal to two ticks more than yesterday's low.\" (p.3); filled on the gap day when the high reaches the stop (the open is below it)",
            GAP_TRAIL, GAP_TICK,
            "X|Stop is 8% under the entry price, then 8% under the highest close since entry; checked from the day after entry; an open under the stop fills at the open [book silent on gaps through the stop]"],
          "Entry order lives for the gap day only [book silent]. The TWTC example's 'immediate entry ... at open as long as it has been preceded by another gap signal' (p.3) is not a stated rule and is not coded. 'one would normally reduce the sell stop to 5% or less to protect profit' (p.2) has no trigger: not coded. No statistics printed. Lane K75 (#240, R4 #281 (3)): a bar where the trailing stop is hit inside the day and a new gap entry also triggers (25% of SPY entries) is ordered on that day's real 1-minute bars; because the daily convention changed the result materially (#243 (c)), results are published only over the minute windows of SPY, SPX, NIFTY 50 and NIFTYBANK. p.order = 0: the daily convention (out, then in).", ),
        d("stkch_gap_fgu_long", "Full Gap Up: Long (buy stop 2 ticks over the first-hour high)", "Trading Strategies · Gap Trading Strategies", "Long", Live, &["OPEN60_HIGH"],
          &["F|\"A Full Gap Up occurs when the opening price is greater than yesterday's high.\" (gapStrategies1.pdf p.1)",
            "E|\"If a stock's opening price is greater than yesterday's high, revisit the 1-minute chart after 10:30 am and set a long (buy) stop two ticks above the high achieved in the first hour of trading.\" (p.2)",
            GAP_TRAIL, GAP_TICK,
            "F|OPEN60_HIGH = high of the first 60 minutes of the session (US 9:30-10:30 ET; NSE 9:15-10:15 IST), per ticker", GAP_AFTER],
          "Lane K75 (R4 #279 (2)): runs on the minute tickers (SPY, SPX, NIFTY 50, NIFTYBANK) over the minute years; OPEN60_HIGH is built from the real 1-minute bars (intraday::session_series). Day order on the gap day [book silent]. A bar where the trailing stop is hit inside the day and a new gap entry also triggers is ordered on that day's 1-minute bars (p.order = 0: the daily convention, out then in).", ),
        d("stkch_gap_fgu_short", "Full Gap Up: Short (sell stop 2 ticks under the first-hour low)", "Trading Strategies · Gap Trading Strategies", "Short", Live, &["OPEN60_LOW"],
          &["E|\"If a stock's opening price is greater than yesterday's high, revisit the 1-minute chart after 10:30 am and set a short stop equal to two ticks below the low achieved in the first hour of trading.\" (gapStrategies1.pdf p.2)",
            GAP_TRAIL, GAP_TICK,
            "F|OPEN60_LOW = low of the first 60 minutes of the session, per ticker", GAP_AFTER],
          "Lane K75 (R4 #279 (2)): runs on the minute tickers (SPY, SPX, NIFTY 50, NIFTYBANK) over the minute years; OPEN60_LOW is built from the real 1-minute bars (intraday::session_series). Day order on the gap day [book silent]. A bar where the trailing stop is hit inside the day and a new gap entry also triggers is ordered on that day's 1-minute bars (p.order = 0: the daily convention, out then in).", ),
        d("stkch_gap_fgd_short", "Full Gap Down: Short (sell stop 2 ticks under the first-hour low)", "Trading Strategies · Gap Trading Strategies", "Short", Live, &["OPEN60_LOW"],
          &["E|\"If a stock's opening price is less than yesterday's low, revisit the 1-minute chart after 10:30 am and set a short stop equal to two ticks below the low achieved in the first hour of trading.\" (gapStrategies1.pdf p.3)",
            GAP_TRAIL, GAP_TICK, "F|OPEN60_LOW = low of the first 60 minutes of the session, per ticker", GAP_AFTER],
          "Lane K75 (R4 #279 (2)): runs on the minute tickers (SPY, SPX, NIFTY 50, NIFTYBANK) over the minute years; OPEN60_LOW is built from the real 1-minute bars (intraday::session_series). Day order on the gap day [book silent]. A bar where the trailing stop is hit inside the day and a new gap entry also triggers is ordered on that day's 1-minute bars (p.order = 0: the daily convention, out then in).", ),
        d("stkch_gap_partial", "Partial Gap Up / Down, long and short (first-hour breakouts)", "Trading Strategies · Gap Trading Strategies", "Long/Short",
          NotTestable("the partial-gap entries depend on a 'volume requirement' that Part 1 never defines ('If the volume requirement is not met, the safest way to play a partial gap is to wait until the price breaks the previous high'); Part 2's only volume rule is a 'projected daily volume' of 2x the 5-day average, which needs an intraday projection; the article also suggests 'closer trailing stops of 5-6%' without fixing one"), &[],
          &["E|Partial Gap Up (open > yesterday's close, not > yesterday's high): buy stop two ticks above / short stop two ticks below the first-hour range (gapStrategies1.pdf p.3-4)",
            "E|Partial Gap Down (open < yesterday's close, not < yesterday's low): same first-hour breakouts (p.4)",
            "F|\"If the volume requirement is not met, the safest way to play a partial gap is to wait until the price breaks the previous high (on a long trade) or low (on a short trade).\" (p.4)"],
          "Not testable; nothing added.", ),
        d("stkch_gap_modified", "Gap Modified Trading Method", "Trading Strategies · Gap Trading Strategies", "Long/Short",
          NotTestable("entry needs 'projected daily volume be double the 5-day average' (an intraday projection) and exits are 'mental stops ... instead of hard stops' with no size"), &[],
          &["E|\"set a long stop equal to the average of the open price and the high price achieved in the first hour of trading. This method recommends that the projected daily volume be double the 5-day average.\" (gapStrategies2.pdf p.1)",
            "X|\"Since heavy volume trading can experience quick reversals, mental stops are usually used instead of hard stops.\" (p.1)"],
          "Not testable.", ),
        d("stkch_k39_obv", "Last Stochastic with OBV / 30-day MA confirmation", "Trading Strategies · The \"Last\" Stochastic Technique", "Long",
          NotTestable("the confirmation window is not stated: the OBV crossover of its 30-day MA came 'soon after' the K signal (p.6); no rule for how long to wait"), &[],
          &["F|\"one should pair it with a volume assessment for trade confirmation ... the On Balance Volume (OBV) indicator has been added along with a 30 day MA as a signal line\" (lastStochastic.pdf p.5)"],
          "Not testable.", ),
        d("stkch_preholiday", "Pre-Holiday Effect (buy one or two days before nine US holidays)", "Trading Strategies · The Pre-Holiday Effect", "Long",
          NotTestable("entry price is not stated ('buy near open' in one example, 'buy near close' in another), 'one or two days prior' is left open and the short-term exit is 'just after the holiday'; the year-end variant is a 1/9-of-capital portfolio schedule over nine US holiday closings"), &[],
          &["E|\"The general strategy is to purchase equities one or two days prior to a holiday. Short term traders would look to sell just after the holiday while longer-term investors would wait until year end.\" (preHolidayEffect.pdf p.1)"],
          "Published table (S&P 500 1928-1975, buy 2 / 1 days before, sell at year end) cannot be matched to a mechanical rule. Close relatives in the vault: kpss_hol_d1/d2, mzwws_preholiday, jbcdt2_preholiday_oc.", ),
        d("stkch_murphy_macd", "Murphy's Law 'Trade MACD' (12/26/9): buy a cross up below zero, sell a cross down above zero, weekly signal first", "Trading Strategies · John Murphy's Ten Laws of Technical Trading", "Long",
          Live, &[],
          &["E|\"A buy signal occurs when the faster line crosses above the slower and both lines are below zero.\" (MurphysLaws.pdf p.2-3, law 8)",
            "X|\"A sell signal takes place when the faster line crosses below the slower from above the zero line.\" (same): closes a long (long or flat by default, like the module's other sell signals; p.mode = 0 also sells short)",
            "F|\"Weekly signals take precedence over daily signals.\" (same)",
            "F|MACD: \"the traditional 12/26 MACD\" = 12-day EMA - 26-day EMA of closes, \"Usually, a 9-day EMA of MACD is plotted ... as a trigger\" (indic_MACD1.pdf p.1, the same ChartSchool collection): faster line = MACD, slower line = its 9-period EMA",
            EMA_DEF, EOD],
          concat!("Adapted: 1 unstated detail (R4 #297). Unstated in the article: how the weekly signal takes precedence; vault reading: the weekly signal in force (the last buy or sell signal of the same MACD rule on weekly closes, known at the close of each week's last trading day) is a filter and an exit: a daily buy is taken only while the weekly buy is in force, a daily sell only while the weekly sell is in force, and a new weekly signal against the open position closes it at that close (p.wk = 0 trades the daily signals alone). ",
            "'Crosses above' = MACD > signal today and MACD <= signal yesterday; 'both lines below zero' / 'from above the zero line' are read on the signal day. Weeks = Monday-Sunday calendar weeks; the data's last bar ends a week only on a Friday. The signal EMA starts from the first full MACD value (the collection's SMA seed). Murphy's examples are generic; run on indices, ETFs and stocks (instrument swap). No statistics printed."), ),
        d("stkch_discretionary", "Chart-pattern, candlestick, Dow Theory and indicator-interpretation articles", "ChartSchool · Chart Analysis / Indicators / Market Analysis", "Long/Short",
          NotTestable("pattern recognition and indicator reading (divergences, support/resistance, trendlines, 'in conjunction with other aspects of technical analysis'); no complete entry + exit"), &[],
          &["F|Chart patterns: bump-and-run, cup with handle, double/triple tops and bottoms, flags, head and shoulders, wedges, triangles, rectangles, channels, measured moves, rounding bottom",
            "F|Candlestick articles, Dow Theory 1-3, Elliott Wave, support/resistance, trendlines, Richard Rhodes' trading rules, Arthur Hill's goals/style/strategy (support/resistance with weekly trend, divergences)",
            "F|Indicator articles (A/D line, ADX, ATR, Bollinger Bands double tops/bottoms, CCI, Chaikin MF/oscillator, MACD 1-4, PPO, PVO, RSI, StochRSI, Stochastics, Williams %R, TRIN, VIX 20/30 extremes, ZigZag): signals described as interpretations without exits or with judgement filters ('if an uptrend has been identified')"],
          "Duplicates skipped: Lambert CCI +/-100 (indic_CCI.pdf p.2) = lebcafm_cci_lambert / rwmts_cci_trend; price crossing the 100-day SMA (indic_movingAvg2.pdf, Enron) = apta_ma100_dji.", ),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "stkch_ema_30_100" => ("stkch_hill_ma", "Arthur Hill MA crossovers (StockCharts)", "30/100 EMA"),
        "stkch_ppo_20_60" => ("stkch_hill_ma", "Arthur Hill MA crossovers (StockCharts)", "20/60 EMA +/-2%"),
        "stkch_k39" | "stkch_k39_obv" => ("stkch_k39", "Last Stochastic K39 (StockCharts)", if id == "stkch_k39" { "" } else { "OBV confirmation" }),
        "stkch_gap_fgd_long" => ("stkch_gap", "Gap trading strategies (StockCharts)", "full gap down · long"),
        "stkch_gap_fgu_long" => ("stkch_gap", "Gap trading strategies (StockCharts)", "full gap up · long"),
        "stkch_gap_fgu_short" => ("stkch_gap", "Gap trading strategies (StockCharts)", "full gap up · short"),
        "stkch_gap_fgd_short" => ("stkch_gap", "Gap trading strategies (StockCharts)", "full gap down · short"),
        "stkch_gap_partial" => ("stkch_gap", "Gap trading strategies (StockCharts)", "partial gaps"),
        "stkch_gap_modified" => ("stkch_gap", "Gap trading strategies (StockCharts)", "modified method"),
        "stkch_preholiday" => ("stkch_preholiday", "Pre-holiday effect (StockCharts)", ""),
        "stkch_murphy_macd" => ("stkch_murphy_macd", "Murphy MACD law (StockCharts)", ""),
        "stkch_discretionary" => ("stkch_discretionary", "ChartSchool discretionary material", ""),
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    let gap = |trail: f64| vec![p("ticks", "Entry offset in ticks of the traded instrument (US 0.01, NSE 0.05): 2 = the article's two ticks; 0 = no offset", 2., 0., 10., 1.),
                                p("trail", "Trailing stop %", trail, 1., 20., 0.5),
                                p("order", "A day with a trailing-stop exit inside the bar and a new gap entry: 1 = ordered on the real 1-minute bars where they exist (default, lane K75, #240); 0 = daily convention (out, then in)", 1., 0., 1., 1.)];
    Some(match id {
        "stkch_ema_30_100" => vec![p("fast", "Fast EMA", 30., 5., 60., 1.), p("slow", "Slow EMA", 100., 20., 250., 5.),
                                   p("rev", "0 = long/flat (book); 1 = also short on the sell signal", 0., 0., 1., 1.)],
        "stkch_ppo_20_60" => vec![p("fast", "Fast EMA", 20., 5., 60., 1.), p("slow", "Slow EMA", 60., 20., 250., 5.),
                                  p("band", "PPO buy / sell band %", 2., 0., 10., 0.25),
                                  p("rev", "0 = long/flat (book); 1 = also short on the sell signal", 0., 0., 1., 1.)],
        "stkch_k39" => vec![p("n", "Stochastic period", 39., 5., 100., 1.), p("buy_lvl", "K crosses above (buy)", 50., 10., 90., 5.),
                            p("sell_lvl", "K crosses below (sell)", 50., 10., 90., 5.),
                            p("wk", "Previous week: 0 = calendar week (book); 1 = prior 5 bars", 0., 0., 1., 1.),
                            p("mode", "0 = sell and sell short (book); 1 = sell only", 0., 0., 1., 1.)],
        "stkch_murphy_macd" => vec![p("fast", "MACD fast EMA (collection: 12)", 12., 2., 50., 1.), p("slow", "MACD slow EMA (collection: 26)", 26., 5., 100., 1.),
                                    p("sig", "Signal line EMA of MACD (collection: 9)", 9., 2., 30., 1.),
                                    p("wk", "Weekly precedence: 1 = weekly signal filters and closes (vault reading); 0 = daily signals alone", 1., 0., 1., 1.),
                                    p("mode", "1 = long only (default); 0 = the sell signal also sells short", 1., 0., 1., 1.)],
        "stkch_gap_fgd_long" | "stkch_gap_fgu_long" => gap(8.),
        "stkch_gap_fgu_short" | "stkch_gap_fgd_short" => gap(4.),
        _ => return None,
    })
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    if from > to || to >= x.r.len() { return None; }
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(0., |d| d.def));
    let n = x.r.len();
    Some(match id {
        "stkch_ema_30_100" | "stkch_ppo_20_60" => {
            let (f, s) = (ema(&x.c, (g("fast").round() as usize).max(1)), ema(&x.c, (g("slow").round() as usize).max(1)));
            let band = if id == "stkch_ppo_20_60" { g("band") } else { 0. };
            let mut st = vec![0i8; n];
            for t in 0..n {
                let ppo = (f[t] - s[t]) / s[t] * 100.;
                st[t] = if ppo > band { 1 } else if ppo < -band { -1 } else if t > 0 { st[t - 1] } else { 0 };
            }
            let why = if band > 0. { "PPO below -band (sell signal) · close" } else { "fast EMA below slow EMA (sell signal) · close" };
            states(x, from, to, &st, g("rev") >= 0.5, why)
        }
        "stkch_murphy_macd" => {
            let u = |k: &str| (g(k).round() as usize).max(1);
            let (fa, sl, sg) = (u("fast"), u("slow"), u("sig"));
            let (wk_on, shorts) = (g("wk") >= 0.5, g("mode") < 0.5);
            let dsig = macd_signals(&x.c, fa, sl, sg);
            // weekly closes: a week ends on its last bar (the data's last bar only if it is a Friday)
            let wn: Vec<i64> = x.r.iter().map(|r| (days(&r.date) - 4).div_euclid(7)).collect();
            let ends: Vec<usize> = (0..n).filter(|&t| if t + 1 < n { wn[t + 1] != wn[t] } else { (days(&x.r[t].date) + 3).rem_euclid(7) == 4 }).collect();
            let wsig = macd_signals(&ends.iter().map(|&t| x.c[t]).collect::<Vec<_>>(), fa, sl, sg);
            let mut ws = vec![0i8; n];
            let (mut k, mut cur) = (0usize, 0i8);
            for t in 0..n {
                let mut new_w = 0i8;
                if k < ends.len() && ends[k] == t { if wsig[k] != 0 { cur = wsig[k]; new_w = wsig[k]; } k += 1; }
                ws[t] = if new_w != 0 { 2 * new_w } else { cur }; // +-2 = a new weekly signal on this bar
            }
            let mut st = vec![0i8; n];
            for t in 1..n {
                let (prev, w) = (st[t - 1], ws[t].signum());
                let d = dsig[t];
                let mut nx = prev;
                if wk_on {
                    if ws[t].abs() == 2 && prev != 0 && w != prev { nx = 0; }
                    if d != 0 && d == w { nx = d; }
                } else if d != 0 { nx = d; }
                if nx < 0 && !shorts { nx = 0; }
                st[t] = nx;
            }
            let mut out = vec![];
            let mut cur_t: Option<Trade> = None;
            for t in from.max(1)..=to {
                if st[t] == st[t - 1] { continue; }
                if let Some(tr) = cur_t.take() { out.push(fin(tr, t, x.c[t], t, if st[t] == 0 { "weekly signal against the position · close" } else { "opposite MACD signal · close" })); }
                if st[t] != 0 { cur_t = Some(mk(st[t] as f64, t, x.c[t], t, "close")); }
            }
            mark_open(&mut out, cur_t, x, to);
            out
        }
        "stkch_k39" => {
            let k = (g("n").round() as usize).max(2);
            let kk: Vec<f64> = (0..n).map(|t| if t + 1 < k { f64::NAN } else {
                let hh = x.h[t + 1 - k..=t].iter().copied().fold(f64::NEG_INFINITY, f64::max);
                let ll = x.l[t + 1 - k..=t].iter().copied().fold(f64::INFINITY, f64::min);
                if hh > ll { 100. * (x.c[t] - ll) / (hh - ll) } else { f64::NAN } }).collect();
            let (wh, wl) = prev_week_closes(x, g("wk") < 0.5);
            let (bl, sl) = (g("buy_lvl"), g("sell_lvl"));
            let mut st = vec![0i8; n];
            for t in 1..n {
                st[t] = st[t - 1];
                if kk[t] > bl && kk[t - 1] <= bl && x.c[t] > wh[t] { st[t] = 1; }
                else if kk[t] < sl && kk[t - 1] >= sl && x.c[t] < wl[t] { st[t] = -1; }
            }
            states(x, from, to, &st, g("mode") < 0.5, "K below the sell level and close under last week's low close · close")
        }
        "stkch_gap_fgd_long" => {
            let Some((from, to)) = crate::intraday_ep6::window(id, x, from, to, p) else { return Some(vec![]) }; // K75: #243 (c) minute window
            let tk = g("ticks").round().max(0.) * x.market_tick(); gap(x, from, to, -1, 1., g("trail") / 100., 0, g("order") >= 0.5, |t| Some(x.l[t - 1] + tk)) }
        "stkch_gap_fgu_long" | "stkch_gap_fgu_short" | "stkch_gap_fgd_short" => {
            let tk = g("ticks").round().max(0.) * x.market_tick();
            let (kind, dir, name) = match id { "stkch_gap_fgu_long" => (1, 1., "OPEN60_HIGH"), "stkch_gap_fgu_short" => (1, -1., "OPEN60_LOW"), _ => (-1, -1., "OPEN60_LOW") };
            let Some(s) = x.series(name) else { return Some(vec![]) };
            gap(x, from, to, kind, dir, g("trail") / 100., 60, g("order") >= 0.5, |t| Some(if dir > 0. { s[t] + tk } else { s[t] - tk }))
        }
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    fn row(d: &str, o: f64, h: f64, l: f64, c: f64) -> Row { Row { ticker: "T".into(), date: d.into(), open: o, high: h, low: l, close: c, volume: None } }

    #[test]
    fn full_gap_down_long_and_8pct_trailing_stop() {
        let r = vec![row("2001-01-02", 100., 101., 99., 100.), row("2001-01-03", 97., 103., 96., 102.),
                     row("2001-01-04", 102., 110., 101., 110.), row("2001-01-05", 109., 109., 100., 101.)];
        let x = Ctx::new(&r, &r, &[]);
        let ts = run("stkch_gap_fgd_long", &x, 0, 3, &Params::new()).unwrap();
        assert_eq!(ts.len(), 1);
        assert_eq!(ts[0].legs[0].i, 1);
        assert!((ts[0].legs[0].px - 99.02).abs() < 1e-9); // yesterday's low 99 + 2 ticks of 0.01 (US; #276 (2))
        // best close 110 -> stop 101.2 hit on day 3 (low 100), open 109 above it -> fill 101.2
        assert_eq!(ts[0].exit_i, 3);
        assert!((ts[0].exit_px - 101.2).abs() < 1e-9);
        let z = run("stkch_gap_fgd_long", &x, 0, 3, &[("ticks".to_string(), 0.)].into_iter().collect()).unwrap();
        assert!((z[0].legs[0].px - 99.).abs() < 1e-9); // the 0-offset option
        let r2: Vec<Row> = r.iter().map(|w| Row { ticker: "NIFTY 50".into(), ..w.clone() }).collect();
        let n = run("stkch_gap_fgd_long", &Ctx::new(&r2, &r2, &[]), 0, 3, &Params::new()).unwrap();
        assert!((n[0].legs[0].px - 99.10).abs() < 1e-9); // NSE: 2 x 0.05
        assert!(reviewed("stkch_gap_fgd_long") && !reviewed("stkch_gap_fgu_long"));
    }

    #[test]
    fn same_day_stop_exit_and_new_gap_entry_ordered_on_minutes() {
        // lane K75 (#240, R4 #281 (3)): day 3 is a full gap down whose trailing stop (95.68) is hit inside the bar and whose
        // entry stop (103.52) also triggers; its minutes show the entry level first (still long), then the stop: no re-entry
        let mk = |d: &str, o: f64, h: f64, l: f64, c: f64| Row { ticker: "SPY".into(), date: d.into(), open: o, high: h, low: l, close: c, volume: None };
        let r = vec![mk("2001-01-02", 100., 101., 99., 100.), mk("2001-01-03", 98., 103., 97., 103.), mk("2001-01-04", 104., 104.5, 103.5, 104.),
                     mk("2001-01-05", 103., 104., 95., 95.5)];
        let mut x = Ctx::new(&r, &r, &[]);
        let conv = run("stkch_gap_fgd_long", &x, 0, 3, &[("order".to_string(), 0.)].into_iter().collect()).unwrap();
        assert_eq!(conv.iter().map(|t| (t.legs[0].i, t.exit_i, t.open)).collect::<Vec<_>>(), vec![(1, 3, false), (3, 3, true)]);
        let dir = std::env::temp_dir().join(format!("stkchk75{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let body = "Date,Time,Open,High,Low,Close\n2001-01-02,09:30,100,101,99,100\n2001-01-05,09:30,103,104,102,103.4\n2001-01-05,09:31,103.4,103.4,95,95.5\n";
        std::fs::write(dir.join("SPY.m1.csv"), body).unwrap();
        std::fs::write(dir.join("SPY.m1idx.csv"), format!("Date,offset,bytes\n2001-01-02,30,32\n2001-01-05,62,{}\n", body.len() - 62)).unwrap(); // minute window 01-02..01-05
        x.minutes = crate::intraday::Minutes::load(dir.to_str().unwrap(), "SPY").map(|m| &*Box::leak(Box::new(m)));
        let res = run("stkch_gap_fgd_long", &x, 0, 3, &Params::new()).unwrap();
        assert_eq!(res.iter().map(|t| (t.legs[0].i, t.exit_i, t.open)).collect::<Vec<_>>(), vec![(1, 3, false)]);
        assert!((res[0].exit_px - 104. * 0.92).abs() < 1e-9);
        std::fs::remove_dir_all(dir).unwrap();
    }

    /// `name`'s 1-minute file for ticker T: (date, ["HH:MM,o,h,l,c", ...]).
    fn minutes(name: &str, days: &[(&str, &[&str])]) -> &'static crate::intraday::Minutes {
        let dir = std::env::temp_dir().join(format!("stkch{name}{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let (mut body, mut idx) = (String::from("Date,Time,Open,High,Low,Close\n"), String::from("Date,offset,bytes\n"));
        for (d, lines) in days {
            let start = body.len();
            for l in *lines { body.push_str(&format!("{d},{l}\n")); }
            idx.push_str(&format!("{d},{start},{}\n", body.len() - start));
        }
        std::fs::write(dir.join("T.m1.csv"), &body).unwrap();
        std::fs::write(dir.join("T.m1idx.csv"), &idx).unwrap();
        Box::leak(Box::new(crate::intraday::Minutes::load(dir.to_str().unwrap(), "T").unwrap()))
    }
    /// A full gap up on 2001-01-03 (open 102 > 101) with the given first-hour high / low and minutes.
    fn gap_up(name: &str, h1: f64, l1: f64, mins: &'static [&'static str]) -> Ctx<'static> {
        let r: &'static [Row] = Box::leak(vec![row("2001-01-02", 100., 101., 99., 100.), row("2001-01-03", 102., 106., 101.5, 105.), row("2001-01-04", 105., 107., 95., 96.)].into_boxed_slice());
        let mut x = Ctx::new(r, r, &[]);
        x.extra.insert("OPEN60_HIGH".into(), vec![f64::NAN, h1, f64::NAN]);
        x.extra.insert("OPEN60_LOW".into(), vec![f64::NAN, l1, f64::NAN]);
        x.minutes = Some(minutes(name, &[("2001-01-03", mins)]));
        x
    }

    #[test]
    fn full_gap_up_first_hour_breakouts() {
        let x = gap_up("fh", 104., 101.8, &["09:30,102,104,101.8,103", "10:30,103,104.5,102.5,104.4", "11:00,102,102.2,101.5,102"]);
        let l = run("stkch_gap_fgu_long", &x, 0, 2, &Params::new()).unwrap();
        assert_eq!((l[0].legs[0].i, l[0].exit_i), (1, 2));
        assert!((l[0].legs[0].px - 104.02).abs() < 1e-9 && (l[0].exit_px - 96.6).abs() < 1e-9);
        let s = run("stkch_gap_fgu_short", &x, 0, 2, &Params::new()).unwrap();
        assert!((s[0].legs[0].px - 101.78).abs() < 1e-9 && (s[0].exit_px - 101.78 * 1.04).abs() < 1e-9);
        // no minutes that day: no entry (the daily high says nothing about the time)
        let mut y = gap_up("fh0", 104., 101.8, &["09:30,102,104,101.8,103"]);
        y.minutes = Some(minutes("fh00", &[("2001-01-02", &["09:30,100,101,99,100"])]));
        assert!(run("stkch_gap_fgu_long", &y, 0, 2, &Params::new()).unwrap().is_empty());
    }

    /// Lane K79 (R4 #290 (2)): with p.ticks = 0 the stop is the first-hour high itself, which the daily high always reaches; the
    /// high was made at 09:45 and no minute from 10:30 reaches it again: no trade.
    #[test]
    fn first_hour_high_not_retouched_after_1030_no_trade() {
        let x = gap_up("k79a", 106., 101.5, &["09:30,102,103,101.5,102.5", "09:45,102.5,106,102.4,103", "10:30,103,105.9,102,103.5", "15:59,103.5,105,103,105"]);
        let p: Params = [("ticks".to_string(), 0.)].into_iter().collect();
        assert!(run("stkch_gap_fgu_long", &x, 0, 2, &p).unwrap().is_empty());
    }

    /// Lane K79 (R4 #290 (3)): a minute after 10:30 opening at 106 over the 105.02 stop fills at 106, not at the stop.
    #[test]
    fn minute_opening_through_the_stop_fills_at_its_open() {
        let x = gap_up("k79b", 105., 101.6, &["09:30,102,105,101.6,104", "10:30,104.5,105,104,105", "10:31,106,106.5,105.8,106.2"]);
        let l = run("stkch_gap_fgu_long", &x, 0, 2, &Params::new()).unwrap();
        assert_eq!(l[0].legs[0].i, 1);
        assert_eq!(l[0].legs[0].px, 106.);
    }

    #[test]
    fn previous_calendar_week_close_extremes() {
        // Thu 4 Jan, Fri 5 Jan (week 1); Mon 8 Jan (week 2) sees week 1's closes
        let r = vec![row("2001-01-04", 1., 1., 1., 5.), row("2001-01-05", 1., 1., 1., 7.), row("2001-01-08", 1., 1., 1., 6.)];
        let x = Ctx::new(&r, &r, &[]);
        let (h, l) = prev_week_closes(&x, true);
        assert!(h[1].is_nan());
        assert_eq!((h[2], l[2]), (7., 5.));
    }
    #[test]
    fn murphy_macd_entries_follow_the_signal_rule() {
        // lane R2A (R4 #297): with p.wk = 0 every long entry is a daily buy signal (MACD crosses its 9-EMA, both below zero), at that close,
        // and every exit is a daily sell signal; the weekly filter (default) only removes trades.
        let r: Vec<Row> = (0..600).map(|i| { let v = 100. + 10. * (i as f64 / 15.).sin();
            let d = 7305 + i as i64 + 2 * (i as i64 / 5); // weekdays from 1990-01-01 (a Monday)
            let (y, m, dd) = { let z = d + 719468; let era = z.div_euclid(146097); let doe = z - era * 146097; let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
                let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); let mp = (5 * doy + 2) / 153; let dd = doy - (153 * mp + 2) / 5 + 1; let m = if mp < 10 { mp + 3 } else { mp - 9 };
                (yoe + era * 400 + if m <= 2 { 1 } else { 0 }, m, dd) };
            row(&format!("{y:04}-{m:02}-{dd:02}"), v, v + 1., v - 1., v) }).collect();
        let x = Ctx::new(&r, &r, &[]);
        let sig = macd_signals(&x.c, 12, 26, 9);
        let daily: Params = [("wk".to_string(), 0.)].into_iter().collect();
        let t = run("stkch_murphy_macd", &x, 0, 599, &daily).unwrap();
        assert!(t.len() >= 3);
        for tr in &t { assert!(tr.dir == 1. && sig[tr.legs[0].i] == 1 && tr.legs[0].px == x.c[tr.legs[0].i]); if !tr.open { assert_eq!(sig[tr.exit_i], -1); } }
        let w = run("stkch_murphy_macd", &x, 0, 599, &Params::new()).unwrap();
        assert!(w.iter().all(|tr| sig[tr.legs[0].i] == 1) && w.len() <= t.len());
    }
}