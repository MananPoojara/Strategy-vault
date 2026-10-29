//! Active Trader Magazine, 2004 issues (research drive Trading Knowledge II/@NewsLetters-Mags/Active Trader Magazine/2004/,
//! copies in Books/Active Trader Magazine/2004/). Book id "atm04": Trading System Lab (stocks + futures) columns and the
//! fully specified strategy columns of 2004. Notes, page refs, oracle: library/atm04/README.md.
//! Uses the helpers of atm03 (own simulator; no jkbwts::sim).
use crate::books::atm03::{by_open, cci_of, day_num, done, ema, finish, fold, hh, lim_up, ll, new_trade, stdev, stop_dn, stop_up, weekday, wma, Pos};
use crate::connors::{atr, rsi, sma, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};
use Status::*;

pub const BOOK: &str = "atm04";
/// Admitted ids (authors/activetrader/admission/SOURCE-CONTRACT.md).
pub const REVIEWED: &[&str] = &["atm04_glitch", "atm04_monthly_bo", "atm04_cci_weekly", "atm04_wma", "atm04_55_trail", "atm04_55_simple", "atm04_four_pct", "atm04_rsi_agita"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, group, dir, status, vol: bool, rules, notes, claim| StrategyDef { id, name, group, session: 0, dir, status, needs_volume: vol, needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim };
    let c = crate::connors::claim;
    vec![
        d("atm04_glitch", "Glitch Index · buy GI < -2, exit GI > +2", "TSL Feb 2004 (p.27)", "Long", Live, false,
          &["F|\"Glitch Index = (Diff/Closing Price) * 100 where SMA = 30-period simple moving average, Diff = Closing Price - SMAMult, RocSMA = Rate of Change(SMA) * 0.1 + 1, SMAMult = SMA * RocSMA\" (figure label: \"One-day ROC of 30-day SMA\")",
            "E|\"1. The Glitch Index must be less than -2. 2. The highest Glitch Index reading within the last 30 bars must be less than +5. 3. If these two conditions are met, buy the next bar at the market.\"",
            "X|\"1. If the Glitch Index is greater than +2, exit next bar at the market.\""],
          "ROC = Wealth-Lab ROC in percent: 100 x (SMA/SMA[1] - 1). The 30-bar window includes the current bar [B]. One position at a time.",
          c("~600", ">70%", "", "", "AT 18-stock portfolio 11/1993-11/2003: +86.98%, max DD -19.28%")),
        d("atm04_monthly_bo", "Monthly breakout (T-bond walk-forward study) · buy 1-month high, sell 2-month low", "TSL Futures Mar 2004", "Long", Live, false,
          &["E|\"1. Entry: Buy at the highest high of the last x months.\"", "X|\"2. Exit: Sell at the lowest low of the last y months.\"",
            "F|In-sample best: \"buy at the highest high of the past month and sell at the lowest low of the past two months\" (x = 1, y = 2; out-of-sample best 1/8)"],
          "Stops on daily bars (\"we will use end-of-day data to take opening gaps into consideration\"); levels = high/low of the last x / y completed calendar months [B]; gap = open. One T-bond contract, long only; run on the indices.",
          c("19 (in-sample)", "n/a", "", "", "T-bonds 1985-1995: profit $40,012 vs buy & hold $44,006")),
        d("atm04_cci_weekly", "Commodity Channel Index system (weekly) · CCI(20) +/-100 crosses", "TSL Futures Apr 2004", "Long/Short", Live, false,
          &["E|\"Long entry: Enter long on the next bar's open if the 20-bar CCI crosses below +100 and there is no position (long or short) currently open.\"",
            "X|\"Long exit: Exit the long position on the next bar's open if the 20-bar CCI crosses below -100.\"",
            "E|\"Short entry: Enter short on the next bar's open if the 20-bar CCI crosses above -100 and there is no position currently open.\" \"Short exit: ... crosses above 100.\"",
            "F|Tested \"on weekly data\" (daily results \"too volatile\")"],
          "Weekly bars built from the daily bars (Monday-Sunday weeks); 'next bar's open' = open of the first session of the next week. CCI = (TP - SMA(TP))/(0.015 x mean deviation). Book: 19 futures 1/1994-8/2003.",
          c("190", "46%", "winners +15.39%", "52.16 wks winners / 18.72 losers", "AT futures portfolio 1/1994-8/2003: +258.63%, max DD 39.61%")),
        d("atm04_wma", "Short-term WMA crossover (inverted) · buy WMA(7) crosses below WMA(10)", "TSL May 2004", "Long", Live, false,
          &["E|\"1. Enter long next bar at market open if the seven-day weighted moving average, WMA(7), crosses below the 10-day weighted moving average, WMA(10).\"",
            "X|\"2. Exit long next bar at market open if the seven-day weighted moving average, WMA(7), crosses above the 10-day weighted moving average, WMA(10).\""],
          "Book: 18-stock portfolio 11/1993-11/2003, 10% of equity per trade; Table 1 run 27: 1,845 trades, 67.86%, +0.83%/trade.",
          c("1,845", "67.86%", "+0.83%", "5.84 days winners / 14.56 losers", "AT 18-stock portfolio 11/1993-11/2003: +306.93%, max DD -28.54%")),
        d("atm04_55_trail", "55-day breakout with time-tightening ATR exit", "TSL Futures Jun 2004", "Long", Live, false,
          &["E|\"1. Enter long on the next bar's open if today's high is greater than the highest high of the last 55 days.\"",
            "X|\"2. Exit on the next day's open if the lowest low is less than the lowest low of the last 55 days plus 10 percent of the actual average true range for each day in the market.\" (\"the 20-day average true range (ATR) is multiplied by 10 percent, and then multiplied by the number of days the trade has been open\")"],
          "Today's low vs lowest low of the previous 55 days + 0.1 x ATR(20) x days in trade (entry day = day 1) [B: day count]. Wilder ATR. Long only. Book: 19 futures 3/1994-8/2003, 351 trades.",
          c("351", "n/a", "$963.77/trade", "40 days", "AT futures portfolio 3/1994-8/2003: +135.31%, max DD -28.70%")),
        d("atm04_55_simple", "55-day breakout with 55-day-low exit (comparison system)", "TSL Futures Jun 2004", "Long", Live, false,
          &["E|Same entry: today's high above the highest high of the last 55 days -> buy next open", "X|\"the simple 55-day low exit rule\": exit next open when today's low is below the lowest low of the last 55 days"],
          "Book comparison run: 208 trades, $3,073 average, 95-day average hold, max DD 36.65%.",
          c("208", "n/a", "$3,073/trade", "95 days", "AT futures portfolio 3/1994-8/2003: net $639,204")),
        d("atm04_qqq_crash", "QQQ Crash System (Altucher) · close below 10-bar 1.5-SD Bollinger Band of lows", "TSL Jun 2004", "Long", Live, false,
          &["E|\"1. A 'QQQ crash' is defined as a close below the lower Bollinger Band with the following parameters: calculations based on daily low (rather than closing) prices; 10-bar look-back period; and 1.5 standard deviations for the bands. 2. Buy the next bar at the market after a QQQ crash.\"",
            "X|\"1. Exit on the close if the closing price is above the entry price of the trade. 2. Exit after 20 days if the first exit condition has not been achieved.\""],
          "Lower band = SMA(low,10) - 1.5 x population SD(low,10). The close exit can fire on the entry day [B]. Time exit at the close of the 20th session held (entry day = 1) [B]. Book: QQQ 3/10/1999-3/16/2004, 100% equity.",
          c("48", "91.7% (44 of 48)", "winners +2.33%", "1.92 days", "QQQ 3/10/1999-3/16/2004: +163.26%, PF 25.04, max DD -10.74%")),
        d("atm04_four_pct", "Four-percent breakout (Altucher) · buy +4% above yesterday's close, exit next open", "TSL Sep 2004", "Long", Live, false,
          &["E|\"Entry --- Buy today if price gains four percent from the previous trading day's closing price.\"", "X|\"Exit --- Exit on the open of the next trading day.\"",
            "F|Variation (p.down2 = 1, non-default): \"Price must be down two percent on the day before entering a trade.\""],
          "Buy stop at 1.04 x previous close; an open above it fills at the open (\"Price gapped up at the market open, beyond the four-percent threshold, and the entry order was filled\"). Book: QQQ 3/1999-6/2004 (+121.09%, max DD -11.93%).",
          c("n/a", "n/a", "", "1 day", "QQQ 3/1999-6/2004: +121.09%, PF 1.82, exposure 7.81%")),
        d("atm04_rsi_agita", "RSI Agita · scale in on RSI(20) crosses below 30/25/20/15/10/5, exit RSI > 55", "TSL Oct 2004", "Long", Live, false,
          &["E|\"1. Enter long on the next day's open when the 20-period RSI crosses below any of the following values: 30, 25, 20, 15, 10 or 5. 2. If the RSI crosses below more than one level in one move, enter one long position for each value it crosses below.\"",
            "X|\"3. Exit all open positions at the next day's open after the 20-period RSI crosses above 55.\"", "F|\"Invest 8 percent of equity per trade.\" No stop-loss"],
          "Wilder RSI(20). Each crossed level adds a leg of weight p.w = 0.08 (book). Book: 18 stocks 7/1994-6/2004.",
          c("n/a", "n/a", "", "30.87 days winners / 66.42 losers", "AT 18-stock portfolio 7/1994-6/2004: +146.43%, PF 3.56, max DD -21%")),
        d("atm04_blues", "50-percent blues · 125-day low close, add at -50%, 80% target per position", "TSL Nov 2004", "Long", Live, false,
          &["E|\"1. Enter long tomorrow if today's closing price is the lowest close of the last 125 days.\"", "A|\"2. Enter another position the next day when the closing price falls 50 percent below previous entry price.\"",
            "X|\"3. Exit long if position reaches an 80-percent profit target.\"", "F|\"Invest $9,000 in each trade. This strategy does not use any stop-losses and can take multiple positions in any stock.\""],
          "Reading [C]: rule 1 applies only when no position is open (book: 61 trades in 10 years on 18 stocks, average hold 654 days, which rules out a new position on every 125-day low). Entries at the next open; each position has its own limit at 1.8 x entry from the next bar; positions are folded into one trade per episode (weight p.w = 0.09 each).",
          c("61", "70%", "+40.7%", "654 days", "AT 18-stock portfolio 6/1994-6/2004: +222.78%, PF 5.22, max DD -34.02%")),
        d("atm04_vomund", "Volume-spike screen (Vomund) · volume >= 2 x avg(21), upper-half close, RSI(14) < 30, hold 5 days", "Trading Strategies Mar 2004", "Long", Live, true,
          &["E|\"Volume if [volume] >= [volume esa] * 2. Price if [close]>([high]+[low])/2. RSI if [RSI Wilder] <30. Allworks if Volume and price and RSI.\" (AIQ code); \"daily volume must be at least twice its 21-day average\"",
            "X|\"a five-day holding period --- that is, exiting on the close of the fifth day after the entry day\""],
          "AIQ 'esa' = exponential average, 21 days per the text [B]. Entry at the close of the signal day [B: the article does not name the entry order]. One position at a time. Book: S&P 500 index +0.55% per trade (no count given); stocks 2,014 trades +2.33%.",
          c("n/a", "n/a", "+0.55% (S&P 500 index)", "5 days", "1/1/1999-7/15/2003")),
        d("atm04_60min", "60-minute breakout (first-hour range, third 30-minute bar)", "TSL Jan 2004 (stocks + SPY/QQQ)", "Long/Short",
          NeedsData("30-minute bars (first-hour high/low and the close of the third 30-minute bar)"), false,
          &["E|\"Buy if the closing price of the third 30-minute bar is above the high of the first 60 minutes of the day\"; short below the low", "X|\"Exit all positions on signals in the opposite direction or at the end of the day.\""],
          "Intraday data not in the vault. Book: SPY+QQQ 10/2001-10/2003, 892 trades, +19.88%.", None),
        d("atm04_mtpg", "Monthly top percent gainers (MTPG) / losers (MTPL) rotation", "TSL Jul-Aug 2004", "Long",
          NeedsData("the 18-stock Active Trader portfolio (cross-sectional monthly ranking); the engine runs one instrument at a time"), false,
          &["E|\"Buy the top-five percent-gaining stocks at the opening of the second trading day of each month\" (MTPL: the five worst)", "X|\"Sell those stocks ... that are no long among the top-five\""],
          "Needs a stock universe; not coded.", None),
        d("atm04_lazy_gainer", "Lazy Gainer (HV < 20, above EMA(70), hold 250 days)", "TSL Mar 2004", "Long",
          NotTestable("the historical-volatility lookback is not given (the original's 14-bar standard deviation was replaced by 'historical volatility' with no period)"), false,
          &["E|\"1. The historical volatility (HV) of the stock must be below 20. 2. The HV of SPY must be below 20. 3. The stock must be above its 70-bar EMA. 4. If conditions 1 through 3 have been satisfied for at least five consecutive trading days, enter long the next bar at the market.\"", "X|\"5. Exit the trade after 250 trading days.\""],
          "Not completed (HV period unknown).", None),
        d("atm04_oddball", "Another oddball system (Mark Brown) · 38-bar 3-SD bands of a 10-bar AMA of highs/lows", "TSL Futures May 2004", "Long/Short",
          NotTestable("the Kaufman AMA fast/slow constants are not given and it is not stated how the bands are built from the AMA of highs and the AMA of lows"), false,
          &["E|\"Go long and cover a short position on the next bar's open when price closes above the upper band of 38-bar, three-standard-deviation Bollinger Bands based on an 10-bar AMA of highs and lows.\""],
          "Not completed.", None),
        d("atm04_other", "Other 2004 columns: intraday (Meyers, Carter, Kerr, Hartle %b), COT, put/call-VXO, earnings, chart patterns", "2004 issues", "Long/Short",
          NeedsData("intraday bars, CFTC COT, CBOE put/call + VXO or earnings dates; chart-pattern columns are discretionary"), false,
          &["E|e.g. Kaeppel Aug 2004: \"Buy when the PCVXO falls below 110.0 after previously rising to 110.0 or above. Sell when the PCVXO rises above 90.0 after previously falling to 90.0 or below.\""],
          "See README.", None),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let v: &'static str = catalogue().into_iter().find(|d| d.id == id)?.name;
    Some(match id {
        "atm04_glitch" | "atm04_rsi_agita" | "atm04_blues" | "atm04_wma" | "atm04_vomund" => ("atm04_counter", "Active Trader 2004 · countertrend", v),
        "atm04_monthly_bo" | "atm04_cci_weekly" | "atm04_55_trail" | "atm04_55_simple" | "atm04_oddball" => ("atm04_trend", "Active Trader 2004 · trend", v),
        "atm04_qqq_crash" | "atm04_four_pct" => ("atm04_altucher", "Altucher (Trade Like a Hedge Fund)", v),
        "atm04_60min" | "atm04_mtpg" | "atm04_lazy_gainer" | "atm04_other" => ("atm04_other", "Active Trader 2004 · not testable / needs data", v),
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    Some(match id {
        "atm04_glitch" => vec![p("ma", "SMA length", 30., 5., 100., 1.), p("in", "Buy below", -2., -10., 0., 0.5), p("cap", "Highest GI in window must be below", 5., 1., 20., 0.5), p("win", "Window (bars)", 30., 5., 100., 1.), p("out", "Exit above", 2., 0., 10., 0.5)],
        "atm04_monthly_bo" => vec![p("xm", "Entry: months for the highest high", 1., 1., 8., 1.), p("ym", "Exit: months for the lowest low", 2., 1., 8., 1.)],
        "atm04_cci_weekly" => vec![p("n", "CCI length (weeks)", 20., 5., 50., 1.), p("lvl", "Level", 100., 50., 200., 10.)],
        "atm04_wma" => vec![p("fast", "Fast WMA", 7., 2., 20., 1.), p("slow", "Slow WMA", 10., 3., 40., 1.)],
        "atm04_55_trail" => vec![p("n", "Breakout / low lookback", 55., 10., 100., 1.), p("atr_n", "ATR length", 20., 5., 50., 1.), p("f", "ATR fraction per day", 0.1, 0.01, 0.5, 0.01)],
        "atm04_55_simple" => vec![p("n", "Breakout / low lookback", 55., 10., 100., 1.)],
        "atm04_qqq_crash" => vec![p("n", "Band lookback (lows)", 10., 5., 30., 1.), p("k", "Standard deviations", 1.5, 0.5, 3., 0.1), p("max", "Maximum days held", 20., 2., 60., 1.)],
        "atm04_four_pct" => vec![p("pct", "Gain from previous close %", 4., 1., 10., 0.5), p("down2", "1 = also require the prior day down >= 2% (variation)", 0., 0., 1., 1.)],
        "atm04_rsi_agita" => vec![p("n", "RSI length", 20., 5., 50., 1.), p("exit", "Exit when RSI crosses above", 55., 40., 70., 1.), p("w", "Leg weight (fraction of equity)", 0.08, 0.01, 1., 0.01)],
        "atm04_blues" => vec![p("n", "Lowest close lookback", 125., 10., 250., 5.), p("add", "Add when close is this % below the previous entry", 50., 10., 80., 5.), p("tgt", "Profit target % per position", 80., 10., 200., 5.), p("w", "Position weight (fraction of equity)", 0.09, 0.01, 1., 0.01)],
        "atm04_vomund" => vec![p("vn", "Volume average length", 21., 5., 50., 1.), p("vk", "Volume multiple", 2., 1., 5., 0.25), p("rn", "RSI length", 14., 2., 30., 1.), p("rl", "RSI below", 30., 10., 50., 1.), p("hold", "Days held after entry", 5., 1., 20., 1.)],
        "atm04_60min" | "atm04_mtpg" | "atm04_lazy_gainer" | "atm04_oddball" | "atm04_other" => vec![],
        _ => return None,
    })
}

/// Weekly bars: (first daily index, last daily index, open, high, low, close) per Monday-Sunday week.
fn weeks(x: &Ctx) -> Vec<(usize, usize, f64, f64, f64, f64)> {
    let mut out: Vec<(usize, usize, f64, f64, f64, f64)> = vec![];
    let mut key = i64::MIN;
    for (i, r) in x.r.iter().enumerate() {
        let dn = day_num(&r.date);
        let k = dn - (weekday(dn) + 6) % 7;
        if k != key { out.push((i, i, x.o[i], x.h[i], x.l[i], x.c[i])); key = k; }
        else { let w = out.last_mut().unwrap(); w.1 = i; w.3 = w.3.max(x.h[i]); w.4 = w.4.min(x.l[i]); w.5 = x.c[i]; }
    }
    out
}
/// Calendar-month id per bar.
fn month_id(d: &str) -> i64 { d[0..4].parse::<i64>().unwrap() * 12 + d[5..7].parse::<i64>().unwrap() }

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    let n = x.r.len();
    if !p.keys().all(|k| schema.iter().any(|d| d.key == k)) || from > to || to >= n { return None; }
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(0., |d| d.def));
    let u = |k: &str| g(k).max(1.) as usize;
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    Some(match id {
        "atm04_glitch" => {
            let s = sma(c, u("ma"));
            let gi: Vec<f64> = (0..n).map(|i| if i == 0 || s[i - 1].is_nan() { f64::NAN } else {
                let roc = 100. * (s[i] / s[i - 1] - 1.);
                (c[i] - s[i] * (roc * 0.1 + 1.)) / c[i] * 100. }).collect();
            let (gin, cap, win, gout) = (g("in"), g("cap"), u("win"), g("out"));
            by_open(x, from, to, |i, pos, _| {
                if pos == 0 { let full = i + 1 >= win && gi[i + 1 - win..=i].iter().all(|v| !v.is_nan()); let m = hh(&gi, i, win); if full && gi[i] < gin && m < cap { Some(1) } else { None } } else if gi[i] > gout { Some(0) } else { None }
            })
        }
        "atm04_monthly_bo" => {
            let (xm, ym) = (u("xm") as i64, u("ym") as i64);
            let mid: Vec<i64> = x.r.iter().map(|r| month_id(&r.date)).collect();
            // monthly highs / lows by month id
            let mut mh = std::collections::BTreeMap::<i64, (f64, f64)>::new();
            for i in 0..n { let e = mh.entry(mid[i]).or_insert((f64::MIN, f64::MAX)); e.0 = e.0.max(h[i]); e.1 = e.1.min(l[i]); }
            let level = |m: i64, k: i64, hi: bool| -> f64 {
                let mut v = if hi { f64::MIN } else { f64::MAX };
                for q in m - k..m { match mh.get(&q) { Some(&(a, b)) => v = if hi { v.max(a) } else { v.min(b) }, None => return f64::NAN } }
                v
            };
            let (mut out, mut cur) = (vec![], None::<Trade>);
            for j in from..=to {
                match cur.take() {
                    Some(t) => match stop_dn(x, j, level(mid[j], ym, false)) { Some(px) => out.push(done(t, j, px, j, "y-month low stop")), None => cur = Some(t) },
                    None => if let Some(px) = stop_up(x, j, level(mid[j], xm, true)) { cur = Some(new_trade(1., j, px, "stop")); },
                }
            }
            finish(&mut out, cur, x, to);
            out
        }
        "atm04_cci_weekly" => {
            let w = weeks(x);
            let (wh, wl, wc): (Vec<f64>, Vec<f64>, Vec<f64>) = (w.iter().map(|b| b.3).collect(), w.iter().map(|b| b.4).collect(), w.iter().map(|b| b.5).collect());
            let cc = cci_of(&wh, &wl, &wc, u("n"));
            let lv = g("lvl");
            let (mut out, mut cur, mut pos) = (vec![], None::<Trade>, 0i8);
            for k in 1..w.len().saturating_sub(1) {
                let j = w[k + 1].0; // first session of next week
                if w[k].1 < from || j > to { continue; }
                let (a, b) = (cc[k - 1], cc[k]);
                if a.is_nan() || b.is_nan() { continue; }
                let want = match pos {
                    0 => if b < lv && a >= lv { 1 } else if b > -lv && a <= -lv { -1 } else { 0 },
                    1 => if b < -lv && a >= -lv { 0 } else { 1 },
                    _ => if b > lv && a <= lv { 0 } else { -1 },
                };
                if want == pos { continue; }
                if let Some(t) = cur.take() { out.push(done(t, j, o[j], w[k].1, "weekly CCI exit at next week's open")); }
                if want != 0 { cur = Some(new_trade(want as f64, j, o[j], "open")); }
                pos = want;
            }
            finish(&mut out, cur, x, to);
            out
        }
        "atm04_wma" => {
            let (f, s) = (wma(c, u("fast")), wma(c, u("slow")));
            by_open(x, from, to, |i, pos, _| {
                if i == 0 || s[i - 1].is_nan() { return None; }
                if pos == 0 && f[i] < s[i] && f[i - 1] >= s[i - 1] { Some(1) } else if pos == 1 && f[i] > s[i] && f[i - 1] <= s[i - 1] { Some(0) } else { None }
            })
        }
        "atm04_55_trail" | "atm04_55_simple" => {
            let nn = u("n");
            let trail = id == "atm04_55_trail";
            let a = atr(x.r, if trail { u("atr_n") } else { 14 });
            let f = if trail { g("f") } else { 0. };
            by_open(x, from, to, |i, pos, t| {
                if i < nn { return None; }
                if pos == 0 { if h[i] > hh(h, i - 1, nn) { Some(1) } else { None } } else {
                    let days = (i + 1 - t.unwrap().legs[0].i) as f64;
                    if l[i] < ll(l, i - 1, nn) + f * a[i] * days { Some(0) } else { None }
                }
            })
        }
        "atm04_qqq_crash" => {
            let (m, sd, k, mx) = (sma(l, u("n")), stdev(l, u("n")), g("k"), u("max"));
            let (mut out, mut i) = (vec![], from);
            while i < to {
                if m[i].is_nan() || !(c[i] < m[i] - k * sd[i]) { i += 1; continue; }
                let e = i + 1;
                let t = new_trade(1., e, o[e], "open");
                let mut j = e;
                let mut t = Some(t);
                while j <= to {
                    if c[j] > o[e] { out.push(done(t.take().unwrap(), j, c[j], j, "close above entry")); break; }
                    if j + 1 - e >= mx { out.push(done(t.take().unwrap(), j, c[j], j, "20-day time exit")); break; }
                    j += 1;
                }
                if t.is_some() { finish(&mut out, t, x, to); break; }
                i = j; // a new crash on the exit day can be bought the next morning
            }
            out
        }
        "atm04_four_pct" => {
            let (f, d2) = (1. + g("pct") / 100., g("down2") > 0.5);
            let mut out = vec![];
            for j in from.max(2)..=to {
                if d2 && !(c[j - 1] <= c[j - 2] * 0.98) { continue; }
                if let Some(px) = stop_up(x, j, c[j - 1] * f) {
                    let t = new_trade(1., j, px, "stop");
                    if j < to { out.push(done(t, j + 1, o[j + 1], j, "exit next open")); } else { finish(&mut out, Some(t), x, to); }
                }
            }
            out
        }
        "atm04_rsi_agita" => {
            let (r, ex, w) = (rsi(c, u("n")), g("exit"), g("w"));
            const LV: [f64; 6] = [30., 25., 20., 15., 10., 5.];
            let (mut out, mut cur) = (vec![], None::<Trade>);
            for i in from.max(1)..to {
                let j = i + 1;
                if r[i - 1].is_nan() { continue; }
                if let Some(t) = cur.take() {
                    if r[i] > ex && r[i - 1] <= ex { out.push(done(t, j, o[j], i, "RSI crossed above 55: exit all next open")); continue; }
                    cur = Some(t);
                }
                let k = LV.iter().filter(|&&v| r[i - 1] >= v && r[i] < v).count();
                for _ in 0..k {
                    match cur.as_mut() { Some(t) => t.legs.push(Leg { i: j, px: o[j], w }), None => { let mut t = new_trade(1., j, o[j], "open"); t.legs[0].w = w; cur = Some(t); } }
                }
            }
            finish(&mut out, cur, x, to);
            out
        }
        "atm04_blues" => {
            let (nn, add, tg, w) = (u("n"), 1. - g("add") / 100., 1. + g("tgt") / 100., g("w"));
            let mut open: Vec<Pos> = vec![];
            let mut ps = vec![];
            let mut last_px = f64::NAN;
            for i in from..=to {
                // exits on bar i for positions entered before i
                let mut k = 0;
                while k < open.len() {
                    if open[k].i < i { if let Some(px) = lim_up(x, i, open[k].px * tg) { let mut q = open.remove(k); q.xi = i; q.xpx = px; q.why = "80% target".into(); q.open = false; ps.push(q); continue; } }
                    k += 1;
                }
                if i == to { break; }
                let e = i + 1;
                let sig = if open.is_empty() { i + 1 >= nn && c[i] <= ll(c, i, nn) } else { c[i] <= last_px * add };
                if sig { open.push(Pos { dir: 1., i: e, px: o[e], w, xi: to, xpx: c[to], why: "open".into(), open: true }); last_px = o[e]; }
            }
            ps.extend(open);
            fold(ps)
        }
        "atm04_vomund" => {
            let v: Vec<f64> = x.r.iter().map(|r| r.volume.unwrap_or(f64::NAN)).collect();
            let (va, vk, r, rl, hold) = (ema(&v, u("vn")), g("vk"), rsi(c, u("rn")), g("rl"), u("hold"));
            let (mut out, mut i) = (vec![], from);
            while i <= to {
                if !(v[i] >= va[i] * vk && c[i] > (h[i] + l[i]) / 2. && r[i] < rl) { i += 1; continue; }
                let t = new_trade(1., i, c[i], "close");
                let xi = i + hold;
                if xi > to { finish(&mut out, Some(t), x, to); break; }
                out.push(done(t, xi, c[xi], xi, "5-day hold: exit at the close"));
                i = xi;
            }
            out
        }
        "atm04_60min" | "atm04_mtpg" | "atm04_lazy_gainer" | "atm04_oddball" | "atm04_other" => vec![],
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn four_pct_gap_fills_at_open() {
        let b = [(100., 100., 100., 100.), (100., 101., 99., 100.), (105., 106., 104., 105.), (104., 104., 103., 103.)];
        let r: Vec<Row> = b.iter().enumerate().map(|(i, &(o, h, l, c))| Row { ticker: "T".into(), date: format!("2001-01-{:02}", i + 1), open: o, high: h, low: l, close: c, volume: None }).collect();
        let x = Ctx::new(&r, &r, &[]);
        let t = run("atm04_four_pct", &x, 0, 3, &Params::new()).unwrap();
        assert_eq!(t.len(), 1);
        assert_eq!((t[0].legs[0].i, t[0].legs[0].px, t[0].exit_i, t[0].exit_px), (2, 105., 3, 104.));
    }
}
