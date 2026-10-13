//! StockCharts.com ChartSchool articles, 1999-2001 ("Stock Charts Full Articles", research drive
//! Trading Knowledge II/@NewsLetters-Mags/Stock Charts Full Articles/*.pdf; copy in Book-to-rust/Books/StockCharts.com
//! ChartSchool (Stock Charts Full Articles)/). Book id `stkch`. Refs are article file + PDF page ("n of m" footer).
//! Most of the 80 articles are indicator / chart-pattern education with no complete rule set; only the systems with a
//! stated entry AND exit are Live. Stocks in the articles -> run on the index (instrument swap only).
//! Sources, checks and the list of not-testable articles: library/stkch/.
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "stkch";

const EOD: &str = "E|Fill: the article gives no order type for these end-of-day chart signals; the signal is acted on at the close of the signal day [book silent]";
const EMA_DEF: &str = "F|EMA: \"The 10-period simple moving average is used for the first calculation only. After that the previous period's EMA is used\" (indic_movingAvg.pdf p.4), weight 2/(n+1)";
const GAP_TRAIL: &str = "X|\"Once a position is entered, you calculate and set an 8% trailing stop to exit a long position, and a 4% trailing stop to exit a short position\"; examples trail from the close: \"8% below the final close of $58.50\", \"trailing stop would be set at $80 (8% less than the $87 close)\" (gapStrategies1.pdf p.2-3)";
const GAP_TICK: &str = "F|\"A 'tick' is defined as the bid/ask spread, usually 1/8 to 1/4 point\" (gapStrategies1.pdf p.2); index tick here = p.tick (0.05, the NSE tick)";

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
fn gap(x: &Ctx, from: usize, to: usize, kind: i8, dir: f64, trail: f64, lvl: impl Fn(usize) -> Option<f64>) -> Vec<Trade> {
    let mut out = vec![];
    let mut cur: Option<(Trade, f64)> = None; // trade, best close (or entry) so far
    for t in from.max(1)..=to {
        if let Some((tr, best)) = cur.take() {
            let stop = if dir > 0. { best * (1. - trail) } else { best * (1. + trail) };
            let hit = if dir > 0. { x.l[t] <= stop } else { x.h[t] >= stop };
            if hit {
                let px = if dir > 0. { x.o[t].min(stop) } else { x.o[t].max(stop) };
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
        let fill = if dir > 0. { x.h[t] >= lv } else { x.l[t] <= lv };
        if !fill { continue; }
        let tr = mk(dir, t, lv, t, "stop");
        let best = if dir > 0. { lv.max(x.c[t]) } else { lv.min(x.c[t]) };
        cur = Some((tr, best));
    }
    mark_open(&mut out, cur.map(|c| c.0), x, to);
    out
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
          "Entry order lives for the gap day only [book silent]. The TWTC example's 'immediate entry ... at open as long as it has been preceded by another gap signal' (p.3) is not a stated rule and is not coded. 'one would normally reduce the sell stop to 5% or less to protect profit' (p.2) has no trigger: not coded. No statistics printed.", ),
        d("stkch_gap_fgu_long", "Full Gap Up: Long (buy stop 2 ticks over the first-hour high)", "Trading Strategies · Gap Trading Strategies", "Long", Live, &["OPEN60_HIGH"],
          &["F|\"A Full Gap Up occurs when the opening price is greater than yesterday's high.\" (gapStrategies1.pdf p.1)",
            "E|\"If a stock's opening price is greater than yesterday's high, revisit the 1-minute chart after 10:30 am and set a long (buy) stop two ticks above the high achieved in the first hour of trading.\" (p.2)",
            GAP_TRAIL, GAP_TICK,
            "F|OPEN60_HIGH = high of the first 60 minutes of the session (US 9:30-10:30 ET; NSE 9:15-10:15 IST), per ticker. A day high above stop level means the break came after the first hour; fill at the stop"],
          "Needs first-hour intraday data (OPEN60_HIGH); runs where the series exists. Day order on the gap day [book silent].", ),
        d("stkch_gap_fgu_short", "Full Gap Up: Short (sell stop 2 ticks under the first-hour low)", "Trading Strategies · Gap Trading Strategies", "Short", Live, &["OPEN60_LOW"],
          &["E|\"If a stock's opening price is greater than yesterday's high, revisit the 1-minute chart after 10:30 am and set a short stop equal to two ticks below the low achieved in the first hour of trading.\" (gapStrategies1.pdf p.2)",
            GAP_TRAIL, GAP_TICK,
            "F|OPEN60_LOW = low of the first 60 minutes of the session, per ticker"],
          "Needs first-hour intraday data (OPEN60_LOW). Day order on the gap day [book silent].", ),
        d("stkch_gap_fgd_short", "Full Gap Down: Short (sell stop 2 ticks under the first-hour low)", "Trading Strategies · Gap Trading Strategies", "Short", Live, &["OPEN60_LOW"],
          &["E|\"If a stock's opening price is less than yesterday's low, revisit the 1-minute chart after 10:30 am and set a short stop equal to two ticks below the low achieved in the first hour of trading.\" (gapStrategies1.pdf p.3)",
            GAP_TRAIL, GAP_TICK, "F|OPEN60_LOW = low of the first 60 minutes of the session, per ticker"],
          "Needs first-hour intraday data (OPEN60_LOW). Day order on the gap day [book silent].", ),
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
        d("stkch_murphy_macd", "Murphy's Law 'Trade MACD' (crosses below / above zero)", "Trading Strategies · John Murphy's Ten Laws of Technical Trading", "Long/Short",
          NotTestable("'Weekly signals take precedence over daily signals' is part of the rule but how the two time frames combine is not stated; the article also gives no MACD periods"), &[],
          &["E|\"A buy signal occurs when the faster line crosses above the slower and both lines are below zero. A sell signal takes place when the faster line crosses below the slower from above the zero line. Weekly signals take precedence over daily signals.\" (MurphysLaws.pdf p.3)"],
          "Not testable.", ),
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
    let gap = |trail: f64| vec![p("tick", "Tick added to the stop (2 ticks)", 0.05, 0.01, 1., 0.01),
                                p("trail", "Trailing stop %", trail, 1., 20., 0.5)];
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
        "stkch_gap_fgd_long" => { let tk = 2. * g("tick"); gap(x, from, to, -1, 1., g("trail") / 100., |t| Some(x.l[t - 1] + tk)) }
        "stkch_gap_fgu_long" | "stkch_gap_fgu_short" | "stkch_gap_fgd_short" => {
            let tk = 2. * g("tick");
            let (kind, dir, name) = match id { "stkch_gap_fgu_long" => (1, 1., "OPEN60_HIGH"), "stkch_gap_fgu_short" => (1, -1., "OPEN60_LOW"), _ => (-1, -1., "OPEN60_LOW") };
            let Some(s) = x.series(name) else { return Some(vec![]) };
            gap(x, from, to, kind, dir, g("trail") / 100., |t| Some(if dir > 0. { s[t] + tk } else { s[t] - tk }))
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
        assert!((ts[0].legs[0].px - 99.10).abs() < 1e-9); // yesterday's low 99 + 2 x 0.05
        // best close 110 -> stop 101.2 hit on day 3 (low 100), open 109 above it -> fill 101.2
        assert_eq!(ts[0].exit_i, 3);
        assert!((ts[0].exit_px - 101.2).abs() < 1e-9);
    }

    #[test]
    fn full_gap_up_first_hour_breakouts() {
        let r = vec![row("2001-01-02", 100., 101., 99., 100.), row("2001-01-03", 102., 106., 101.5, 105.), row("2001-01-04", 105., 107., 95., 96.)];
        let mut x = Ctx::new(&r, &r, &[]);
        x.extra.insert("OPEN60_HIGH".into(), vec![f64::NAN, 104., f64::NAN]);
        x.extra.insert("OPEN60_LOW".into(), vec![f64::NAN, 101.8, f64::NAN]);
        let l = run("stkch_gap_fgu_long", &x, 0, 2, &Params::new()).unwrap();
        assert_eq!((l[0].legs[0].i, l[0].exit_i), (1, 2));
        assert!((l[0].legs[0].px - 104.1).abs() < 1e-9 && (l[0].exit_px - 96.6).abs() < 1e-9);
        let s = run("stkch_gap_fgu_short", &x, 0, 2, &Params::new()).unwrap();
        assert!((s[0].legs[0].px - 101.7).abs() < 1e-9 && (s[0].exit_px - 101.7 * 1.04).abs() < 1e-9);
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
}
