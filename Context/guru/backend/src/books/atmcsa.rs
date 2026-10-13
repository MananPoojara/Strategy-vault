//! Currency Trader "Currency System Analysis, Vol. 1: 2005" binder (Wealth-Lab columns, Jan-Dec 2005), from the Active Trader
//! "Misc Numbers" binders (research drive Trading Knowledge II/@NewsLetters-Mags/Active Trader Magazine/Misc Numbers/
//! Binder - Currency System Analysis Vol 1.pdf, read in place). Book id "atmcsa". Notes, page refs, oracle: library/atmcsa/README.md.
//! FX / currency-futures rules run on the indices (instrument swap). Uses the atm03 helpers (own simulator; no jkbwts::sim).
use crate::books::atm03::{by_open, day_num, done, ema, finish, hh, ll, new_trade, stop_dn, stop_up};
use crate::connors::{atr, sma, Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use Status::*;

pub const BOOK: &str = "atmcsa";

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, group, dir, status, rules, notes, claim| StrategyDef { id, name, group, session: 0, dir, status, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim };
    let c = crate::connors::claim;
    vec![
        d("atmcsa_bo20", "Basic currency trend system · close beyond the 20-day high/low, stop and reverse", "CSA Jan 2005 (binder p.3-5)", "Long/Short", Live,
          &["E|\"1. Enter long and cover shorts on the next bar's open when price closes above the 20-day high.\"",
            "E|\"2. Enter short and exit longs on the next bar's open when price closes below the 20-day low.\"",
            "X|Stop and reverse: rule 1 \"cover[s] shorts\" and rule 2 \"exit[s] longs\" at the same open"],
          "20-day high/low = the 20 bars before the signal bar (a close can never exceed a high that includes its own bar). Always in after the first signal. Position sizing (2% risk) not modelled. Similar to ch_bta2_cbo_ref, which fills at the signal close instead of the next open.",
          c("294", "37.76%", "", "48.79 days", "6 FX pairs 12/1994-12/2004: +145.43%, max DD -25.89%")),
        d("atmcsa_bo20_mod", "Modified currency trend system · 20-day breakout with prior-bar filter, 10-day exit", "CSA Jan 2005 (binder p.5)", "Long/Short", Live,
          &["E|\"1. Enter long on the next bar's open when price closes above the 20-day high IF the bar preceding the breakout bar closed in the lower 50-percent of its range.\"",
            "X|\"2. Exit long on the next bar's open when price closes below the 10-day low.\"",
            "E|\"3. Enter short on the next bar's open when price closes below the 20-day low IF the bar preceding the breakout bar closed in the upper 50 percent of its range.\"",
            "X|\"4. Cover short on the next bar's open when price closes above the 10-day high.\""],
          "High/low windows end the bar before the signal bar. 'Lower 50 percent' = (close - low)/(high - low) <= 0.5, 'upper' >= 0.5 [B: midpoint counts for both]. A short entry while long also exits the long at the same open (a close below the 20-day low is below the 10-day low).",
          c("195", "36.92%", "", "26.13 days", "6 FX pairs 12/1994-12/2004: +46.73%, max DD -21.05%")),
        d("atmcsa_ma_trend", "Moving average trend-following system · 5-day SMA of highs/lows vs 28-day EMA, 20% stop", "CSA Feb 2005 (binder p.6-8)", "Long/Short", Live,
          &["E|\"Enter long and exit short on the next day's open if the trend is bullish, which means the 5-day SMA of highs crosses above the 28-day EMA of closing prices, the latest close is above the 5-day SMA of highs, and the 5-day moving average's current value is also higher than its value the prior day.\"",
            "E|\"Enter short and exit long on the next day's open if the trend is bearish, which means the 5-day SMA of lows crosses below the 28-day EMA of closing prices, the latest close is below the 5-day SMA of lows, and the 5-day SMA moving average's current value is also below the previous day's value.\"",
            "X|\"Exit any trade if there is a trend change or if a position loses 20 percent.\""],
          "'Trend change' = the rules' own definition of a bullish/bearish trend (the opposite entry signal); all three conditions on the signal bar. 20% loss = stop order at entry x 0.80 (x 1.20 short) from the bar after entry, gap = open [B]. EMA = Wealth-Lab EMA (2/(n+1), SMA seed) [B].",
          c("396", "38.89%", "", "32.37 days", "6 FX pairs 12/1994-12/2004: +292.83%, max DD -39.73%")),
        d("atmcsa_psar_weekly", "Parabolic FX (weekly bars) · Wilder parabolic stop and reverse", "CSA Mar 2005 (binder p.9-11)", "Long/Short", Live,
          &["F|\"SAR price level tomorrow = (SAR price today) + (Acceleration Factor * (Extreme Price - SAR level today))\" — \"We chose the original parameters Wilder used in his book\" (AF 0.02, +0.02, max 0.20)",
            "E|\"1. Enter long when price hits today's upper parabolic stop value. 2. Enter short trade when price hits today's lower parabolic stop value.\"",
            "X|\"3. Exit long trade when price hits today's lower parabolic stop value. 4. Cover short trade when price hits today's upper parabolic stop value.\"",
            "F|\"we then applied the same rules to weekly data\""],
          "Weekly bars (Monday-Sunday) built from the daily bars; each week's SAR comes from the completed weeks (Wilder: SAR never inside the last two bars). The fill is taken on the first day of the week that touches the SAR, at the SAR or the open if that day gaps through it [B]. Flat until the first reversal inside the window. The daily version is a duplicate of rwmts_psar / kftsm_parabolic (not repeated).",
          c("249", "42.23%", "", "11.45 (weekly bars)", "6 FX pairs 12/1994-12/2004, weekly: +122.06%, max DD -31.41% (daily: -52.45%)")),
        d("atmcsa_macd", "MACD system · signal-line cross above zero / zero-line cross above the signal", "CSA Apr 2005 (binder p.12-14)", "Long/Short", Live,
          &["E|\"1. If the MACD line is above zero and it crosses above the signal line, buy tomorrow at the open. 2. If today's MACD line is above yesterday's signal line and if today's MACD crosses above the zero line, buy tomorrow at the open.\"",
            "X|\"1. Exit long positions if the MACD line crosses below the signal line. 2. Exit long positions if the MACD line crosses below the zero line.\"",
            "F|\"Short trades: Invert the long-side signals for short trades.\" \"Additional stops beside the system exit rules are not used.\""],
          "MACD = EMA(12) - EMA(26), signal = EMA(9) of MACD (book text; Wealth-Lab EMA with SMA seed [B]). Exits at the next open like the entries [B: the exit rules give no order]. A short entry while long exits the long and reverses at the same open.",
          c("748", "38.50%", "", "10.92 days", "6 FX pairs 12/1994-12/2004: +74.65%, max DD -36.37%")),
        d("atmcsa_vk_bands", "VK Bands for FX · close beyond the 104-day VK Band, 2% stop", "CSA May 2005 (binder p.15-17)", "Long/Short", Live,
          &["F|\"1) divide a 104-day look-back period into two segments of 52 days; 2) determine the highest high and the lowest low of each 52-day period; 3) calculate a two-period weighted moving average of both the highest highs and lowest lows.\"",
            "E|\"1. Go long tomorrow at the open if price closes above upper VK Band. 2. Go short tomorrow at the open if price closes below the lower VK Band.\"",
            "X|\"3. Sell (exit long) if price closes below the lower VK Band. 4. Buy to cover (exit short) if price closes above the upper VK Band.\" \"the system does not simultaneously reverse position when a stop is hit; it waits for a 'fresh' signal\"",
            "X|\"1. Stop-loss: Exit with a loss when a position is down 2 percent.\" (\"The single trade loss larger than -2 percent was the result of a gap.\")"],
          "Upper = (HH(older 52) + 2 x HH(recent 52)) / 3, lower the same with lows (2-period WMA, newest weight 2). The 104-day window ends at the signal bar [B; p.lag = 1 ends it the bar before]. Band exits at the next open; the exit bar's close is not an entry. 2% stop = stop order at entry x 0.98 (1.02 short) from the bar after entry, gap = open.",
          c("84", "33.33%", "+2.64%", "125.55 days", "6 FX pairs 12/1994-12/2004: +239.93%, max DD -32.82%")),
        d("atmcsa_pharos", "Pharos FX system · weekly-shifted 5-day high/low stop and reverse", "CSA Jun 2005 (binder p.18-20)", "Long/Short", Live,
          &["E|\"1. Go long and cover short with a buy stop at tomorrow's open if price crosses above the highest high of the past five trading days, including the current day (first trading day of the week). The buy stop should equal this value and be valid until (and including) the first trading day of next week.\"",
            "E|\"2. Go short and exit long with a sell stop at tomorrow's open if price crosses below the lowest low of the past five trading days, including the current day. The sell stop should equal this value and be valid until (and including) the first trading day of next week.\"",
            "X|Stop and reverse: the buy stop also covers a short (\"Go long and cover short\"), the sell stop also exits a long (\"Go short and exit long\")"],
          "Levels set at the close of each week's first session (Monday-Sunday weeks; a Monday holiday makes Tuesday the first session), active from the next session through the next first session. Stop fills at the level, or the open if it gaps through. One reversal per bar; when flat and both levels are hit, the one nearer the open fills [B].",
          c("1,155", "38.35%", "+0.10%", "12.06 days", "6 FX pairs 12/1994-12/2004: +497.38%, max DD -41.64%")),
        d("atmcsa_donchian_counter", "Donchian counter-channel system · buy when the lower 20-day line turns up", "CSA Oct 2005 (binder p.21-22)", "Long/Short", Live,
          &["E|\"1. Go long next day at market if the 20-day lower Donchian channel line turns up. 2. Go short next day at market if the upper 20-day Donchian channel line turns down.\"",
            "X|\"3. Exit long with a stop at the lower 20-day Donchian channel line. 4. Exit short with a stop at the upper 20-day Donchian channel line.\""],
          "Channel = lowest low / highest high of the 20 bars ending at the bar; 'turns up' = higher than the bar before. Entry at the next open; the stop (the channel line of the previous bar) works from the bar after entry, gap = open. One position at a time: signals are taken only when flat; a bar where both lines turn is skipped [B]. Book: Euro FX futures 1/1999-1/2005, one contract.",
          c("31", "54.8% (17 of 31)", "", "37 bars", "Euro FX futures 1/1999-1/2005: +320.76% on $20,000")),
        d("atmcsa_dual_stoch", "Dual timeframe stochastics · daily and weekly %K cross 70/30 within 10 days", "CSA Nov 2005 (binder p.23-25)", "Long/Short",
          NotTestable("the second oscillator is a 'two-week' slow stochastic: its look-back (two weekly bars, or ten?) and how a weekly reading is aligned to the daily bars are not given; the %K/%D roles of the 'five-day smoothings' are also unstated"),
          &["E|\"Use 10-day and two-week slow stochastic oscillators, both of which use five-day smoothings instead of the usual three-day smoothing: 1. Go long and exit short the next day at market if both indicators (%K lines) cross below 70 within 10 trading days. 2. Go short and exit long the next day at market if both indicators (%K lines) cross above 30 within 10 trading days.\"",
            "X|\"3. Place a stop-loss above or below the entry price a distance of the 10-day average true range (ATR).\""],
          "Book: BP, EC, JY, SF futures 1/1990-1/2005: 149 trades, 21.48% winners, +0.79%/trade, +13.46%/yr, max DD -42.57%.", None),
        d("atmcsa_tct_trend", "Trend/countertrend system · trend part: 20-day stop breakout, reverse at the opposite channel, 3 x ATR(10) stop", "CSA Dec 2005 (binder p.26-28)", "Long/Short", Live,
          &["E|\"1. Go long with a buy stop at the highest high of the past 20 days.\"", "E|\"2. Exit long and go short at stop at the lowest low of the past 20 days.\"",
            "X|\"3. Exit at stop loss at three times the average true range (ATR) of the past 10 days.\""],
          "Channel = the 20 bars before the order day; stops fill at the level or the open. The loss stop sits 3 x Wilder ATR(10) (value at the signal bar) from the entry price [B]; when both sell stops are hit the higher one fills first (a loss-stop exit is flat, the channel stop reverses); one transaction per bar; when flat and both entry stops are hit, the one nearer the open fills [B]. The combined system holds the trend and countertrend trades at the same time in opposite directions, which one trade list cannot show; the book also tested both parts on their own (trend part +9.48%/yr).",
          c("400 (combined)", "38.50%", "+0.29%", "31.26 bars", "BP, EC, JY, SF futures 1/1990-1/2005: combined +10.38%/yr, trend part alone +9.48%/yr")),
        d("atmcsa_tct_counter", "Trend/countertrend system · countertrend part: fade an open beyond yesterday's range at open +/- 0.2 x ATR(10), exit at the close", "CSA Dec 2005 (binder p.26-28)", "Long/Short", Live,
          &["E|\"1. If the open is above the previous day's high, sell short at limit at the open plus 20 percent of the 10-day ATR. Exit at the close.\"",
            "E|\"2. If the open is below the previous day's low, go long at limit at the open minus 20 percent of the 10-day ATR. Exit at the close.\"",
            "X|\"Exit at the close.\" (both rules)"],
          "Same-day trades: the limit fills when the day's range reaches it (the open is on the other side of it, so the fill is the limit price). ATR = Wilder ATR(10) of the previous day [B].",
          c("n/a", "n/a", "", "1 day", "BP, EC, JY, SF futures 1/1990-1/2005: countertrend part alone +0.78%/yr")),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let v: &'static str = catalogue().into_iter().find(|d| d.id == id)?.name;
    match id { s if s.starts_with("atmcsa_") => Some(("atmcsa", "Currency System Analysis 2005 (Wealth-Lab)", v)), _ => None }
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    Some(match id {
        "atmcsa_bo20" => vec![p("n", "Breakout days", 20., 5., 100., 1.)],
        "atmcsa_bo20_mod" => vec![p("n", "Breakout days", 20., 5., 100., 1.), p("x", "Exit channel days", 10., 3., 50., 1.), p("loc", "Prior-bar close location (fraction of range)", 0.5, 0.1, 0.9, 0.05)],
        "atmcsa_ma_trend" => vec![p("fast", "SMA of highs/lows", 5., 2., 20., 1.), p("slow", "EMA of closes", 28., 10., 100., 1.), p("stop", "Loss stop %", 20., 1., 50., 1.)],
        "atmcsa_psar_weekly" => vec![p("af", "Acceleration factor start/step", 0.02, 0.01, 0.1, 0.01), p("afmax", "Acceleration factor max", 0.2, 0.1, 0.5, 0.05)],
        "atmcsa_macd" => vec![p("fast", "Fast EMA", 12., 3., 30., 1.), p("slow", "Slow EMA", 26., 10., 60., 1.), p("sig", "Signal EMA", 9., 3., 20., 1.)],
        "atmcsa_vk_bands" => vec![p("seg", "Segment length (days)", 52., 10., 120., 1.), p("stop", "Loss stop %", 2., 0.5, 10., 0.5), p("lag", "Window ends this many bars before the signal bar (0 = includes it)", 0., 0., 1., 1.)],
        "atmcsa_pharos" => vec![p("n", "High/low days", 5., 2., 20., 1.)],
        "atmcsa_donchian_counter" => vec![p("n", "Channel days", 20., 5., 60., 1.)],
        "atmcsa_dual_stoch" => vec![],
        "atmcsa_tct_trend" => vec![p("n", "Channel days", 20., 5., 100., 1.), p("atr_n", "ATR days", 10., 5., 30., 1.), p("k", "Loss stop (x ATR)", 3., 1., 6., 0.5)],
        "atmcsa_tct_counter" => vec![p("atr_n", "ATR days", 10., 5., 30., 1.), p("k", "Limit offset (x ATR)", 0.2, 0., 1., 0.05)],
        _ => return None,
    })
}

/// Next-open state machine with an optional fixed-percent protective stop. want(i, pos) at the close of bar i returns the
/// position wanted at the open of i+1 (a change fills there; a reversal closes and opens there). The stop works from the bar
/// after entry (Wealth-Lab bar loop), at entry x (1 -/+ pct), gap = open. Signal bars are from..to-1, as in by_open.
fn open_sim(x: &Ctx, from: usize, to: usize, stop_pct: f64, want: impl Fn(usize, i8) -> Option<i8>) -> Vec<Trade> {
    let (mut out, mut cur) = (vec![], None::<Trade>);
    for j in from + 1..=to {
        let i = j - 1;
        let pos = cur.as_ref().map_or(0, |t| t.dir as i8);
        if let Some(w) = want(i, pos) {
            if w != pos {
                if let Some(t) = cur.take() { out.push(done(t, j, x.o[j], i, if w == 0 { "exit at the open" } else { "reverse at the open" })); }
                if w != 0 { cur = Some(new_trade(w as f64, j, x.o[j], "open")); }
            }
        }
        if stop_pct > 0. {
            if let Some(t) = cur.as_ref() {
                if j > t.legs[0].i {
                    let e = t.legs[0].px;
                    let f = if t.dir > 0. { stop_dn(x, j, e * (1. - stop_pct / 100.)) } else { stop_up(x, j, e * (1. + stop_pct / 100.)) };
                    if let Some(px) = f { let t = cur.take().unwrap(); out.push(done(t, j, px, j, "loss stop")); }
                }
            }
        }
    }
    finish(&mut out, cur, x, to);
    out
}

/// Monday-based week number of a YYYY-MM-DD date (day 4 since 1970-01-01 is a Monday).
pub fn week_of(date: &str) -> i64 { (day_num(date) - 4).div_euclid(7) }

fn crosses_above(a: &[f64], b: &[f64], i: usize) -> bool { i > 0 && a[i] > b[i] && a[i - 1] <= b[i - 1] }
fn crosses_below(a: &[f64], b: &[f64], i: usize) -> bool { i > 0 && a[i] < b[i] && a[i - 1] >= b[i - 1] }

/// Wilder parabolic on weekly bars built from the daily bars 0..=to; returns the reversal fills (day, new dir, price).
fn weekly_psar_flips(x: &Ctx, to: usize, af0: f64, afmax: f64) -> Vec<(usize, i8, f64)> {
    let (o, h, l) = (&x.o, &x.h, &x.l);
    // weeks: (first day, last day, high, low)
    let mut wk: Vec<(usize, usize, f64, f64)> = vec![];
    let mut prev = i64::MIN;
    for i in 0..=to {
        let w = week_of(&x.r[i].date);
        if w != prev { wk.push((i, i, h[i], l[i])); prev = w; } else { let k = wk.last_mut().unwrap(); k.1 = i; k.2 = k.2.max(h[i]); k.3 = k.3.min(l[i]); }
    }
    let mut ev = vec![];
    if wk.len() < 2 { return ev; }
    let (wh, wl): (Vec<f64>, Vec<f64>) = wk.iter().map(|k| (k.2, k.3)).unzip();
    // Wilder: start long with SAR = first week's low, EP = its high. SAR(next) = SAR + AF x (EP - SAR); AF +step on each new EP
    // (max afmax); never inside the last two bars. On a reversal the next SAR = the old trade's EP, AF resets.
    let (mut pos, mut af, mut sar, mut ep) = (1i8, af0, wl[0], wh[0]);
    for w in 1..wk.len() {
        let s = sar; // the SAR that applies to week w
        let flip = if pos == 1 && wl[w] <= s { Some(-1i8) } else if pos == -1 && wh[w] >= s { Some(1i8) } else { None };
        if let Some(nd) = flip {
            // the first day of the week that touches the level
            let d = (wk[w].0..=wk[w].1).find(|&d| if nd < 0 { l[d] <= s } else { h[d] >= s }).unwrap();
            let px = if nd < 0 { o[d].min(s) } else { o[d].max(s) };
            ev.push((d, nd, px));
            sar = if nd < 0 { ep.max(wh[w]).max(wh[w - 1]) } else { ep.min(wl[w]).min(wl[w - 1]) };
            pos = nd; af = af0; ep = if nd < 0 { wl[w] } else { wh[w] };
        } else if pos == 1 {
            if wh[w] > ep { ep = wh[w]; af = (af + af0).min(afmax); }
            sar = (s + af * (ep - s)).min(wl[w]).min(wl[w - 1]);
        } else {
            if wl[w] < ep { ep = wl[w]; af = (af + af0).min(afmax); }
            sar = (s + af * (ep - s)).max(wh[w]).max(wh[w - 1]);
        }
    }
    ev
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    let n = x.r.len();
    if !p.keys().all(|k| schema.iter().any(|d| d.key == k)) || from > to || to >= n { return None; }
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(0., |d| d.def));
    let u = |k: &str| g(k).max(1.) as usize;
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    Some(match id {
        "atmcsa_bo20" => {
            let nn = u("n");
            by_open(x, from, to, |i, _, _| {
                if i < 1 { return None; }
                let (up, dn) = (hh(h, i - 1, nn), ll(l, i - 1, nn));
                if up.is_nan() { None } else if c[i] > up { Some(1) } else if c[i] < dn { Some(-1) } else { None }
            })
        }
        "atmcsa_bo20_mod" => {
            let (nn, xn, loc) = (u("n"), u("x"), g("loc"));
            let pl = |k: usize| if h[k] > l[k] { (c[k] - l[k]) / (h[k] - l[k]) } else { 0.5 };
            open_sim(x, from, to, 0., |i, pos| {
                if i < 2 { return None; }
                let (up, dn, ux, dx) = (hh(h, i - 1, nn), ll(l, i - 1, nn), hh(h, i - 1, xn), ll(l, i - 1, xn));
                if up.is_nan() || ux.is_nan() { return None; }
                let long = c[i] > up && pl(i - 1) <= loc;
                let short = c[i] < dn && pl(i - 1) >= 1. - loc;
                match pos {
                    1 => if short { Some(-1) } else if c[i] < dx { Some(0) } else { None },
                    -1 => if long { Some(1) } else if c[i] > ux { Some(0) } else { None },
                    _ => if long { Some(1) } else if short { Some(-1) } else { None },
                }
            })
        }
        "atmcsa_ma_trend" => {
            let (fh, fl, e) = (sma(h, u("fast")), sma(l, u("fast")), ema(c, u("slow")));
            let bull = |i: usize| crosses_above(&fh, &e, i) && c[i] > fh[i] && fh[i] > fh[i - 1];
            let bear = |i: usize| crosses_below(&fl, &e, i) && c[i] < fl[i] && fl[i] < fl[i - 1];
            open_sim(x, from, to, g("stop"), |i, pos| {
                if i < 1 || e[i - 1].is_nan() || fh[i - 1].is_nan() { return None; }
                let (b, s) = (bull(i), bear(i));
                match pos { 1 => s.then_some(-1), -1 => b.then_some(1), _ => if b { Some(1) } else if s { Some(-1) } else { None } }
            })
        }
        "atmcsa_psar_weekly" => {
            let ev = weekly_psar_flips(x, to, g("af"), g("afmax"));
            let (mut out, mut cur) = (vec![], None::<Trade>);
            for (d, nd, px) in ev {
                if d <= from { continue; }
                if let Some(t) = cur.take() { out.push(done(t, d, px, d, "weekly SAR hit: reverse")); }
                cur = Some(new_trade(nd as f64, d, px, "stop"));
            }
            finish(&mut out, cur, x, to);
            out
        }
        "atmcsa_macd" => {
            let (ef, es) = (ema(c, u("fast")), ema(c, u("slow")));
            let m: Vec<f64> = (0..n).map(|i| ef[i] - es[i]).collect();
            let s = ema(&m, u("sig"));
            let z = vec![0.; n];
            open_sim(x, from, to, 0., |i, pos| {
                if i < 1 || s[i - 1].is_nan() { return None; }
                let long = (m[i] > 0. && crosses_above(&m, &s, i)) || (m[i] > s[i - 1] && crosses_above(&m, &z, i));
                let short = (m[i] < 0. && crosses_below(&m, &s, i)) || (m[i] < s[i - 1] && crosses_below(&m, &z, i));
                let xl = crosses_below(&m, &s, i) || crosses_below(&m, &z, i);
                let xs = crosses_above(&m, &s, i) || crosses_above(&m, &z, i);
                match pos {
                    1 => if short { Some(-1) } else if xl { Some(0) } else { None },
                    -1 => if long { Some(1) } else if xs { Some(0) } else { None },
                    _ => if long { Some(1) } else if short { Some(-1) } else { None },
                }
            })
        }
        "atmcsa_vk_bands" => {
            let (sg, lag) = (u("seg"), g("lag").max(0.) as usize);
            let band = |i: usize| -> (f64, f64) {
                if i + 1 < 2 * sg + lag { return (f64::NAN, f64::NAN); }
                let e = i - lag; // last bar of the window
                let (hn, ho) = (hh(h, e, sg), hh(h, e - sg, sg));
                let (ln, lo) = (ll(l, e, sg), ll(l, e - sg, sg));
                ((ho + 2. * hn) / 3., (lo + 2. * ln) / 3.)
            };
            open_sim(x, from, to, g("stop"), |i, pos| {
                let (up, dn) = band(i);
                if up.is_nan() { return None; }
                match pos { 1 => (c[i] < dn).then_some(0), -1 => (c[i] > up).then_some(0), _ => if c[i] > up { Some(1) } else if c[i] < dn { Some(-1) } else { None } }
            })
        }
        "atmcsa_pharos" => {
            let nn = u("n");
            let wk: Vec<i64> = x.r.iter().map(|r| week_of(&r.date)).collect();
            let (mut out, mut cur) = (vec![], None::<Trade>);
            let (mut up, mut dn) = (f64::NAN, f64::NAN);
            for j in 1..=to {
                let f = j - 1;
                if f == 0 || wk[f] != wk[f - 1] { up = hh(h, f, nn); dn = ll(l, f, nn); }
                if j <= from || up.is_nan() { continue; }
                let pos = cur.as_ref().map_or(0, |t| t.dir as i8);
                let (fu, fd) = (if pos <= 0 { stop_up(x, j, up) } else { None }, if pos >= 0 { stop_dn(x, j, dn) } else { None });
                let hit = match (fu, fd) {
                    (Some(a), Some(b)) => if o[j] >= up { Some((1, a)) } else if o[j] <= dn { Some((-1, b)) } else if up - o[j] <= o[j] - dn { Some((1, a)) } else { Some((-1, b)) },
                    (Some(a), None) => Some((1, a)),
                    (None, Some(b)) => Some((-1, b)),
                    _ => None,
                };
                if let Some((d, px)) = hit {
                    if let Some(t) = cur.take() { out.push(done(t, j, px, j, if d > 0 { "buy stop: reverse long" } else { "sell stop: reverse short" })); }
                    cur = Some(new_trade(d as f64, j, px, "stop"));
                }
            }
            finish(&mut out, cur, x, to);
            out
        }
        "atmcsa_donchian_counter" => {
            let nn = u("n");
            let lo: Vec<f64> = (0..n).map(|i| ll(l, i, nn)).collect();
            let hi: Vec<f64> = (0..n).map(|i| hh(h, i, nn)).collect();
            let (mut out, mut cur) = (vec![], None::<Trade>);
            for j in from + 1..=to {
                let i = j - 1;
                if cur.is_none() && i >= 1 && !lo[i - 1].is_nan() {
                    let (lu, ud) = (lo[i] > lo[i - 1], hi[i] < hi[i - 1]);
                    if lu != ud { cur = Some(new_trade(if lu { 1. } else { -1. }, j, o[j], "open")); }
                }
                if let Some(t) = cur.as_ref() {
                    if j > t.legs[0].i {
                        let f = if t.dir > 0. { stop_dn(x, j, lo[i]) } else { stop_up(x, j, hi[i]) };
                        if let Some(px) = f { let t = cur.take().unwrap(); out.push(done(t, j, px, j, "channel stop")); }
                    }
                }
            }
            finish(&mut out, cur, x, to);
            out
        }
        "atmcsa_tct_trend" => {
            let (nn, k) = (u("n"), g("k"));
            let a = atr(x.r, u("atr_n"));
            let (mut out, mut cur, mut ls) = (vec![], None::<Trade>, f64::NAN);
            for j in from + 1..=to {
                let i = j - 1;
                let (up, dn) = (hh(h, i, nn), ll(l, i, nn));
                if up.is_nan() || a[i].is_nan() { continue; }
                let pos = cur.as_ref().map_or(0, |t| t.dir as i8);
                // (new dir: 0 = loss-stop exit, +-1 = enter/reverse; fill price) for this bar
                let ev: Option<(i8, f64)> = match pos {
                    1 => {
                        let (first, lvl) = if ls >= dn { (0, ls) } else { (-1, dn) };
                        stop_dn(x, j, lvl).map(|px| (first, px))
                    }
                    -1 => {
                        let (first, lvl) = if ls <= up { (0, ls) } else { (1, up) };
                        stop_up(x, j, lvl).map(|px| (first, px))
                    }
                    _ => match (stop_up(x, j, up), stop_dn(x, j, dn)) {
                        (Some(pa), Some(pb)) => if o[j] >= up { Some((1, pa)) } else if o[j] <= dn { Some((-1, pb)) } else if up - o[j] <= o[j] - dn { Some((1, pa)) } else { Some((-1, pb)) },
                        (Some(pa), None) => Some((1, pa)),
                        (None, Some(pb)) => Some((-1, pb)),
                        _ => None,
                    },
                };
                if let Some((nd, px)) = ev {
                    if let Some(t) = cur.take() { out.push(done(t, j, px, j, if nd == 0 { "3 x ATR loss stop" } else { "opposite 20-day stop: reverse" })); }
                    if nd != 0 { cur = Some(new_trade(nd as f64, j, px, "stop")); ls = px - nd as f64 * k * a[i]; }
                }
            }
            finish(&mut out, cur, x, to);
            out
        }
        "atmcsa_tct_counter" => {
            let (a, k) = (atr(x.r, u("atr_n")), g("k"));
            let mut out = vec![];
            for j in from.max(1) + 1..=to {
                let i = j - 1;
                if a[i].is_nan() { continue; }
                let t = if o[j] > h[i] { let lim = o[j] + k * a[i]; (h[j] >= lim).then(|| new_trade(-1., j, lim, "limit")) }
                        else if o[j] < l[i] { let lim = o[j] - k * a[i]; (l[j] <= lim).then(|| new_trade(1., j, lim, "limit")) } else { None };
                if let Some(t) = t { out.push(done(t, j, c[j], j, "exit at the close")); }
            }
            out
        }
        "atmcsa_dual_stoch" => vec![],
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    fn ctx(b: &[(f64, f64, f64, f64)]) -> Vec<Row> {
        b.iter().enumerate().map(|(i, &(o, h, l, c))| Row { ticker: "T".into(), date: format!("2001-01-{:02}", i + 1), open: o, high: h, low: l, close: c, volume: None }).collect()
    }
    #[test]
    fn counter_fade_hand_checked() {
        // ATR(1) of bar 1 = true range of bar 1 = 2; bar 2 opens above bar 1's high (11): short limit 12 + 0.5 x 2 = 13, exit 11.5
        let r = ctx(&[(10., 10.5, 9.5, 10.), (10., 11., 9., 10.), (12., 13.5, 11., 11.5)]);
        let x = Ctx::new(&r, &r, &[]);
        let p: Params = [("atr_n".to_string(), 1.), ("k".to_string(), 0.5)].into_iter().collect();
        let t = run("atmcsa_tct_counter", &x, 0, 2, &p).unwrap();
        assert_eq!(t.len(), 1);
        assert_eq!((t[0].dir, t[0].legs[0].i, t[0].legs[0].px, t[0].exit_i, t[0].exit_px), (-1., 2, 13., 2, 11.5));
    }
    #[test]
    fn week_numbers() {
        assert_eq!(week_of("2001-01-01"), week_of("2001-01-07")); // Mon..Sun
        assert_ne!(week_of("2001-01-07"), week_of("2001-01-08"));
    }
}
