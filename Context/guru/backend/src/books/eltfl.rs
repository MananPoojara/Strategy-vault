//! Alexander Elder, "Trading for a Living" (Wiley 1993). Sections 25-31 (indicators), 41 Elder-ray,
//! 42 Force Index, 43 Triple Screen, 44 Parabolic, 45 Channels. Only rules with an exact entry AND exit are Live.
//! Source, extraction and verification: library/eltfl/README.md. Uses the helpers in books/elcim.rs.
use super::elcim::{ema, falling, force, macd_hist, rising, simulate, tick, weekly, Entry, Protect, Rules};
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "eltfl";
/// Elder author review (23 Oct 2026, authors/elder/admission/SOURCE-CONTRACT.md): admitted on indices, ETFs and stocks.
pub const REVIEWED: &[&str] = &["eltfl_fi2", "eltfl_ts_elderray", "eltfl_sroc"];

const FI2: &[&str] = &[
    "F|\"It pays to buy when the 2-day EMA is negative and sell when it is positive, as long as you trade in the direction of the 13-day EMA of prices.\" (p.228); trend = slope of the 13-day EMA (Fig 42-2, p.230).",
    "E|\"1. Buy when a 2-day EMA of Force Index turns negative during uptrends ... place a buy order above the high price of that day ... If prices continue to decline, your order will not be executed. Then lower your buy order to within one tick of the high of the latest bar.\" (p.231)",
    "E|\"2. Sell short when a 2-day EMA of Force Index turns positive in downtrends ... Place an order to sell short below the low of the latest price bar ... raise it daily to within a tick of the latest bar's low.\" (p.231)",
    "E|[Book silent] The order is cancelled when the 13-day EMA stops pointing in the trade's direction, by analogy with the same trailing-stop technique in Triple Screen (\"until the weekly indicator reverses and cancels its buy signal\", p.241).",
    "X|\"Once your buy stop is triggered, place a protective stop below the low of the trade day or the previous day, whichever is lower.\" Shorts: \"above the high of the latest price bar or the previous bar, whichever is higher.\" (p.231). One tick beyond (p.243, Triple Screen stop).",
    "X|\"A short-term trader who buys when this indicator is negative should sell when it turns positive. A short-term trader who goes short when this indicator is positive should cover when it turns negative.\" (p.231) Executed at the next open (default; 'at the close' = option).",
    "X|Not modelled: \"Move your stop down to a break-even level as early as possible\" (shorts, p.231) - no exact trigger is given.",
];
const TS_ER: &[&str] = &[
    "F|Screen one: \"The original system uses the slope of weekly MACD-Histogram ... The slope is defined as the relationship between the two latest bars.\" (p.237). \"you do not have to wait until Friday to get your signals ... weekly studies have to be performed each day\" (p.135): each day the latest weekly bar is the current week so far (closes at today's close) and is compared with the last completed week. Option weekly=1: completed calendar weeks only [old reading].",
    "F|Screen two: \"When the weekly trend is up, daily Elder-ray gives a buy signal when Bear Power declines below zero and then ticks back up toward the centerline. When the weekly trend is down, daily Elder-ray signals to go short when Bull Power rallies above zero and then ticks back down.\" (p.239-240). Bear Power = Low - 13-day EMA, Bull Power = High - 13-day EMA (p.221).",
    "E|Screen three: \"Place a buy order one tick above the high of the previous day ... Lower your buy order the next day to the level one tick above the latest price bar. Keep lowering your buy-stop each day until stopped in or until the weekly indicator reverses and cancels its buy signal.\" Shorts: \"one tick below the latest bar's low ... keep raising your sell order daily.\" (p.241)",
    "X|\"As soon as you buy, place a stop-loss order one tick below the low of the trade day or the previous day, whichever is lower. Once you sell short, place a protective stop-loss one tick above the high of the trade day or the previous day, whichever is higher.\" (p.243)",
    "X|\"Conservative traders should go long or short on the first signal of the Triple Screen trading system and stay with that position until the major trend reverses or until stopped out.\" (p.243) = exit at the next open after the weekly MACD-Histogram slope reverses against the trade (an unchanged weekly bar is not a reversal). An unfilled order is cancelled when the weekly indicator reverses (p.241).",
    "X|Not modelled: \"Move your stop to a break-even level as soon as the market moves in your favor. Afterwards, the rule of thumb is to move your stop to protect approximately 50 percent of paper profits\" (p.243) - no exact trigger; pyramiding (optional, p.243) not modelled.",
];
const SROC: &[&str] = &[
    "F|\"calculate a 13-day exponential moving average of closing prices and then apply a 21-day Rate of Change to it\" (p.146-147, Fig 28-4 p.151); Rate of Change \"divides the latest price by the closing price 7 days ago. If they are equal, ROC equals 1\" (p.146) - centerline 1.",
    "E|\"1. Buy when S-ROC turns up from below its centerline.\" (p.150) = S-ROC rises after a fall or flat bar, with the turning bar below 1.",
    "X|\"2. Sell when S-ROC stops rising and turns down.\" (p.150)",
    "X|[Book silent] Fills: next open after the signal close (default).",
];

/// EMA value if `x` were appended after `vals` (whose EMA is `ema_v`): seeding as in `ema` (SMA of the first n finite values).
fn ema_next(vals: &[f64], ema_v: &[f64], n: usize, x: f64) -> f64 {
    if !x.is_finite() {
        return f64::NAN;
    }
    if let Some(&p) = ema_v.last().filter(|p| p.is_finite()) {
        let k = 2.0 / (n as f64 + 1.0);
        return x * k + p * (1.0 - k);
    }
    let run: Vec<f64> = vals.iter().rev().take_while(|v| v.is_finite()).copied().collect();
    if run.len() + 1 == n { (run.iter().sum::<f64>() + x) / n as f64 } else { f64::NAN }
}

/// TFL p.135 "weekly studies have to be performed each day": slope of weekly MACD-Histogram 12-26-9 at the close of each
/// day, where the latest weekly bar is the current calendar week so far (close = today's close) and the bar before it is
/// the last completed week. +1 rising, -1 falling, 0 unchanged/unknown.
fn weekly_hist_today(x: &Ctx, wk: &super::elcim::Weekly) -> Vec<i8> {
    let w = &wk.close;
    let (e12, e26) = (ema(w, 12), ema(w, 26));
    let line: Vec<f64> = e12.iter().zip(&e26).map(|(a, b)| a - b).collect();
    let sig = ema(&line, 9);
    let hist: Vec<f64> = line.iter().zip(&sig).map(|(a, b)| a - b).collect();
    (0..x.c.len()).map(|i| {
        // completed weeks before day i's own week (day i completes a week when `at` changes on it)
        let ends_week = wk.at[i].is_some() && (i == 0 || wk.at[i] != wk.at[i - 1]);
        let k = wk.at[i].map_or(0, |a| if ends_week { a } else { a + 1 });
        if k == 0 { return 0; }
        let lp = ema_next(&w[..k], &e12[..k], 12, x.c[i]) - ema_next(&w[..k], &e26[..k], 26, x.c[i]);
        let hp = lp - ema_next(&line[..k], &sig[..k], 9, lp);
        let prev = hist[k - 1];
        if hp > prev { 1 } else if hp < prev { -1 } else { 0 }
    }).collect()
}

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, group, dir, status, vol, rules, notes| StrategyDef {
        id, name, group, session: 0, dir, status, needs_volume: vol, needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim: None,
    };
    use Status::*;
    let minor = "protective stop 'below the latest minor low' / 'above the latest minor high' is not defined, and exits are by divergence or judgement";
    vec![
        d("eltfl_fi2", "Force Index 2-day EMA · 13-day EMA trend, trailing stop entry", "Sec 42 · Force Index", "Long/Short", Live, true, FI2,
          "pp.227-232. Book prints no test results. 'Turns negative' = 2-day EMA of Force Index crosses from >= 0 to < 0 on the close while the 13-day EMA rose that day. Buy stop fills at the open on a gap, else at the stop; protective stop is active from the day after entry. Needs volume (NIFTY 50 from 2000). India = instrument swap only."),
        d("eltfl_ts_elderray", "Triple Screen (1993) · weekly MACD-Histogram + daily Elder-ray", "Sec 43 · Triple Screen", "Long/Short", Live, false, TS_ER,
          "pp.135, 235-243. Book prints no test results. Weekly MACD-Histogram 12-26-9 on calendar weeks, updated daily with the current week so far (p.135; completed weeks only = option weekly=1); a weekly reversal cancels pending orders and exits open trades at the next open. Re-entry after a stop-out is allowed on the next screen-two signal ('first signal' read per signal). Initial tight stop is fixed (break-even/50% trailing not exact, not modelled)."),
        d("eltfl_sroc", "Smoothed Rate of Change (S-ROC 13/21) · long", "Sec 28 · Momentum", "Long", Live, false, SROC,
          "p.150 (indicator by Fred G. Schutzman, presented with these rules by Elder). Book prints no test results. Turn-up = S[i] > S[i-1] and S[i-1] <= S[i-2] with S[i-1] < 1; exit when S[i] < S[i-1]. Fills at the next open (book silent). The section gives no protective stop; none is added (Elder's general advice to use stops gives no level for this rule)."),
        d("eltfl_sroc_short", "Smoothed Rate of Change · short", "Sec 28 · Momentum", "Short", NotTestable("no cover rule: only 'Sell short when S-ROC turns down from above its centerline' (p.150); the book never states when to cover except by divergence"),
          false, &["E|\"Sell short when S-ROC turns down from above its centerline.\" (p.150)"], "p.150."),
        d("eltfl_ts_fi", "Triple Screen (1993) · Force Index second screen", "Sec 43 · Triple Screen", "Long/Short", NotTestable("second-screen rule 'as long as it does not fall to a new multiweek low' gives no number of weeks; the 2002 version without this clause is elcim_ts_fi"),
          true, &["F|\"When the weekly MACD-Histogram rises, the 2-day EMA of Force Index gives buy signals when it falls below its centerline, as long as it does not fall to a new multiweek low.\" (p.239)", "X|\"if a trader is long and Force Index becomes positive ... he may sell and take profits\" (p.242)"], "pp.239-242."),
        d("eltfl_ts_stoch", "Triple Screen (1993) · Stochastic / Williams %R second screen", "Sec 43 · Triple Screen", "Long/Short", NotTestable("Stochastic version not fixed (Fast or Slow, window 5 or 14-21, which line; Sec 30 uses 20/80 lines, Sec 43 uses 30/70); Williams %R window '4- or 5-day' with no levels"),
          false, &["F|\"When weekly MACD-Histogram rises but daily Stochastic falls below 30, it identifies an oversold area, a buying opportunity.\" (p.240)", "X|\"Stochastic rises to 70 percent, he may sell and take profits\" (p.243)"], "pp.157-161, 240-243."),
        d("eltfl_elderray", "Elder-ray stand-alone", "Sec 41 · Elder-ray", "Long/Short", NotTestable(minor),
          false, &["E|\"When Bear Power ticks up, place a buy order above the high of the last two days ... Once long, place a protective stop below the latest minor low.\" (p.225)", "X|\"A sell signal is given when prices reach a new high but Bull Power reaches a lower peak\" (p.226)"], "pp.220-227."),
        d("eltfl_indicator_signals", "MA / MACD / MACD-Histogram / Directional / ROC / RSI / OBV signals", "Sec 25-33 · Indicators", "Long/Short", NotTestable(minor),
          false, &["E|MACD: \"When the fast MACD line crosses above the slow Signal line, it gives a buy signal. Go long, and place a protective stop below the latest minor low.\" (p.129)", "E|MACD-Histogram: \"Buy when MACD-Histogram stops falling and ticks up. Place a protective stop below the latest minor low.\" (p.132)", "E|EMA: \"Buy when prices dip near or slightly below the moving average\" (p.125)"],
          "pp.121-205. The 'old mechanical' MA system (p.125) gives no MA length and is described as obsolete."),
        d("eltfl_parabolic", "Parabolic (Wilder) as an exit", "Sec 44 · Parabolic", "Long/Short", NotTestable("Elder uses Parabolic only to trail stops in a trade 'entered using any method' and says 'Do not use it as an automatic trading method' (p.246)"),
          false, &["X|\"AF increases by 0.02 each day the rally reaches a new high ... up to the maximum of 0.20\" (p.244-245)"], "pp.244-247."),
        d("eltfl_channels", "Moving-average channels / Bollinger bands", "Sec 45 · Channels", "Long/Short", NotTestable("channel coefficient is fitted 'until a channel contains 90 percent to 95 percent of the price action' and readjusted; Bollinger rules use 'very narrow' bands"),
          false, &["E|\"Go long below the moving average when the channel is rising, and take profits at the upper channel line.\" (p.251)"], "pp.247-253."),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "eltfl_fi2" => ("el_force_index", "Elder Force Index", "13-day EMA trend, trailing stop (1993)"),
        "eltfl_ts_elderray" => ("el_triple_screen", "Elder Triple Screen", "1993 · Elder-ray"),
        "eltfl_ts_fi" => ("el_triple_screen", "Elder Triple Screen", "1993 · Force Index (not testable)"),
        "eltfl_ts_stoch" => ("el_triple_screen", "Elder Triple Screen", "1993 · Stochastic (not testable)"),
        "eltfl_sroc" | "eltfl_sroc_short" => ("eltfl_sroc", "Smoothed Rate of Change", if id == "eltfl_sroc" { "Long" } else { "Short (not testable)" }),
        "eltfl_elderray" | "eltfl_indicator_signals" | "eltfl_parabolic" | "eltfl_channels" => ("eltfl_tools", "Elder indicator signals", "Not testable"),
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    match id {
        "eltfl_fi2" => Some(vec![
            p("ema", "Trend EMA length", 13., 5., 30., 1.),
            p("fi", "Force Index EMA length", 2., 1., 13., 1.),
            p("ticks", "Ticks beyond the bar for orders and stops", 1., 0., 2., 1.),
            p("next_open", "Indicator exit: 1 = next open (default), 0 = at the signal close", 1., 0., 1., 1.),
        ]),
        "eltfl_ts_elderray" => Some(vec![
            p("ema", "Elder-ray EMA length", 13., 5., 30., 1.),
            p("ticks", "Ticks beyond the bar for orders and stops", 1., 0., 2., 1.),
            p("weekly", "Weekly bar: 0 = updated daily with the current week (book, p.135), 1 = completed weeks only", 0., 0., 1., 1.),
        ]),
        "eltfl_sroc" => Some(vec![
            p("ema", "S-ROC EMA length", 13., 5., 30., 1.),
            p("roc", "S-ROC rate-of-change length", 21., 5., 40., 1.),
        ]),
        _ => None,
    }
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    if super::elcim::validate_with(&schema, p).is_some() || from > to || to >= x.c.len() {
        return None;
    }
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).unwrap().def);
    let u = |k: &str| g(k) as usize;
    let both: &[f64] = &[1.0, -1.0];
    let never = |_: usize, _: f64| false;
    match id {
        "eltfl_fi2" => {
            let e = ema(&x.c, u("ema"));
            let fi = ema(&force(x), u("fi"));
            let off = g("ticks") * tick(x);
            let trend = |i: usize, d: f64| if d > 0.0 { rising(&e, i) } else { falling(&e, i) };
            let trig = |i: usize, d: f64| i > 0 && trend(i, d) && if d > 0.0 { fi[i] < 0.0 && fi[i - 1] >= 0.0 } else { fi[i] > 0.0 && fi[i - 1] <= 0.0 };
            let exit = |i: usize, d: f64| if d > 0.0 { fi[i] > 0.0 } else { fi[i] < 0.0 };
            Some(simulate(x, from, to, &Rules { sides: both, trig: &trig, keep: &trend, exit: &exit, entry: Entry::Stop(off),
                exit_next_open: u("next_open") == 1, protect: Protect::TwoBar(off), exit_reason: "2-day Force Index crossed zero" }))
        }
        "eltfl_ts_elderray" => {
            let e = ema(&x.c, u("ema"));
            let bear: Vec<f64> = x.l.iter().zip(&e).map(|(l, m)| l - m).collect();
            let bull: Vec<f64> = x.h.iter().zip(&e).map(|(h, m)| h - m).collect();
            let wk = weekly(x);
            let wm = macd_hist(&wk.close, 12, 26, 9);
            let off = g("ticks") * tick(x);
            let live = if u("weekly") == 0 { Some(weekly_hist_today(x, &wk)) } else { None };
            let slope = |i: usize| live.as_ref().map_or_else(|| wk.slope(&wm, i), |v| v[i]);
            let tide = |i: usize, d: f64| slope(i) == d as i8;
            let reversed = |i: usize, d: f64| slope(i) == -(d as i8);
            let alive = |i: usize, d: f64| !reversed(i, d);
            let trig = |i: usize, d: f64| i > 0 && tide(i, d) && if d > 0.0 { bear[i - 1] < 0.0 && bear[i] < 0.0 && bear[i] > bear[i - 1] } else { bull[i - 1] > 0.0 && bull[i] > 0.0 && bull[i] < bull[i - 1] };
            Some(simulate(x, from, to, &Rules { sides: both, trig: &trig, keep: &alive, exit: &reversed, entry: Entry::Stop(off),
                exit_next_open: true, protect: Protect::TwoBar(off), exit_reason: "weekly MACD-Histogram reversed" }))
        }
        "eltfl_sroc" => {
            let e = ema(&x.c, u("ema"));
            let n = u("roc");
            let s: Vec<f64> = (0..e.len()).map(|i| if i >= n { e[i] / e[i - n] } else { f64::NAN }).collect();
            let trig = |i: usize, _: f64| i > 1 && s[i] > s[i - 1] && s[i - 1] <= s[i - 2] && s[i - 1] < 1.0;
            let exit = |i: usize, _: f64| falling(&s, i);
            Some(simulate(x, from, to, &Rules { sides: &[1.0], trig: &trig, keep: &never, exit: &exit, entry: Entry::NextOpen,
                exit_next_open: true, protect: Protect::None, exit_reason: "S-ROC turned down" }))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn weekly_histogram_updated_daily_equals_completed_week_on_its_last_day() {
        // 400 weekdays of a wavy series: on the last day of each week the daily-updated weekly slope (TFL p.135) must equal
        // the completed-week slope; inside a week it compares the week so far with the last completed week.
        let mut d = 0i64; // 2001-01-01 is a Monday
        let r: Vec<Row> = (0..400).map(|i| {
            let c = 100.0 + 10.0 * ((i as f64) / 23.0).sin() + i as f64 * 0.05;
            let date = { let t = 11323 + d; d += if i % 5 == 4 { 3 } else { 1 }; super::super::elcim::tests_date(t) };
            Row { ticker: "SPY".into(), date, open: c, high: c + 1.0, low: c - 1.0, close: c, volume: Some(1.0) }
        }).collect();
        let x = Ctx::new(&r, &r, &[]);
        let wk = weekly(&x);
        let wm = macd_hist(&wk.close, 12, 26, 9);
        let live = weekly_hist_today(&x, &wk);
        let mut checked = 0;
        for i in 1..r.len() {
            if wk.at[i] != wk.at[i - 1] && wk.at[i].is_some_and(|k| k > 0 && wm[k - 1].is_finite()) {
                assert_eq!(live[i], wk.slope(&wm, i), "bar {i}");
                checked += 1;
            }
        }
        assert!(checked > 40);
        // mid-week day: compare by hand with the series where today's close is appended to the completed weeks
        let i = 302; // a Wednesday
        let k = wk.at[i].unwrap() + 1;
        let mut v = wk.close[..k].to_vec(); v.push(x.c[i]);
        let h = macd_hist(&v, 12, 26, 9);
        let want = if h[k] > h[k - 1] { 1 } else if h[k] < h[k - 1] { -1 } else { 0 };
        assert_eq!(live[i], want);
    }
    #[test]
    fn fi2_trailing_buy_stop_tick_stop_and_exit() {
        // Uptrend, then one down day turns the 2-day Force Index negative; next day lower high (order trails),
        // then a breakout fills at the trailed stop; protective stop = lower low of entry/prior bar - 1 tick.
        let mut r: Vec<Row> = (0..40).map(|i| {
            let c = 100.0 + i as f64;
            Row { ticker: "SPX".into(), date: format!("2001-{:02}-{:02}", 1 + i / 28, 1 + i % 28), open: c, high: c + 1.0, low: c - 1.0, close: c, volume: Some(10.0) }
        }).collect();
        // bar 30: down close -> FI negative (EMA13 still rising)
        r[30].close = 125.0; r[30].open = 128.0; r[30].high = 129.5; r[30].low = 124.0;
        // bar 31: lower high, no fill at 129.51; order trails to 31's high + tick
        r[31].open = 125.0; r[31].high = 126.0; r[31].low = 123.0; r[31].close = 124.5;
        // bar 32: trades through 126.01 -> long
        r[32].open = 125.5; r[32].high = 131.0; r[32].low = 125.0; r[32].close = 130.0;
        let x = Ctx::new(&r, &r, &[]);
        let ts = run("eltfl_fi2", &x, 15, 39, &Params::new()).unwrap();
        let t = &ts[0];
        assert_eq!(t.legs[0].i, 32);
        assert!((t.legs[0].px - 126.01).abs() < 1e-9);
        assert_eq!(t.dir, 1.0);
        // FI2 positive at close of 32 -> exit at open of 33
        assert_eq!(t.exit_i, 33);
        assert_eq!(t.exit_px, r[33].open);
    }
}
