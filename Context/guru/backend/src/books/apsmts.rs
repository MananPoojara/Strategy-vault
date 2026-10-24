//! Gerald Appel & Martin E. Hitschler, Stock Market Trading Systems (Dow Jones-Irwin, 1980).
//! Moving-average, channel and breadth systems with complete buy/sell rules are Live; the
//! Time-Trend-Momentum system ("signals may prove open to some subjectivity", p.138), the 10-week
//! swing system, cycles and zero balance are NotTestable. Printed page = PDF page - 13.
//! Sources, transcription and checks: library/apsmts/.
use super::apta::bars;
use super::apwms::{a_minus_d, ema_a, moving_total, run_ls, sma, wma, NSE_AD, NYSE_AD};
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

/// Admitted 2026-10-24 (authors/appel/admission/SOURCE-CONTRACT.md): independent ledgers, US and India, 0 mismatches.
pub const REVIEWED: &[&str] = &["apsmts_pen", "apsmts_confirm", "apsmts_shift", "apsmts_mmacd", "apsmts_lead", "apsmts_chord", "apsmts_ad_obos", "apsmts_ad_obos_nse", "apsmts_channel"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

pub const BOOK: &str = "apsmts";

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, group, dir, status, series: &'static [&'static str], rules, notes| StrategyDef {
        id, name, group, session: 0, dir, status, needs_volume: false, needs_vix: false, needs_series: series,
        book: BOOK, rules, notes, claim: None,
    };
    use Status::*;
    let nt = |id, name, group, why| d(id, name, group, "Long", NotTestable(why), &[], &[], "");
    vec![
        d("apsmts_pen", "Basic moving average penetration system (150-day exponential)", "Ch 2 · The basic moving average system", "Long/Short", Live, &[],
            &["E|\"A buy signal for the NYSE Index was presumed to have taken place whenever the level of the index rose on a daily closing basis from below to above the moving average employed.\" (p.15) Buy (and cover shorts) on the close.",
              "X|\"A sell signal was presumed to have taken place whenever the level of the index fell from above to below the moving average employed.\" (p.15) Sell and sell short on the close (\"Although 66 trades are shown for the 150-day exponential average, you would have made only 33 trades had you traded mutual funds, since mutual funds cannot be sold short\", p.20).",
              "F|Default = the 150-day exponential average the book recommends (\"the 150-day exponential average produced a gain of 556 points between 1970 and 1977\", p.19). p.kind: 0 straight (simple), 1 weighted (weights 1..N, p.6-7), 2 exponential (smoothing constant 2/(N+1), started from the first close and valid after N days, p.7-9).",
              "F|p.filt = penetration filter in % (Ch 3, p.22-27; book tests 1% on IBM), default 0 = no filter."],
            "Book: Dow Industrials 1970-77, Table 2-3 (p.20) for 10/50/150-day straight, weighted and exponential averages; Table 2-2 (p.18) 150-day straight 1970-78. Our DJI starts 1970-01-02, so averages are not yet formed in early 1970. p.kind=0 (straight) is the rule already in the vault as kftsm_hh_ma (that one also skips days with high = low). p.side=1 long only.",
        ),
        d("apsmts_confirm", "Confirming crossover system (0.15 and 0.04 exponential averages)", "Ch 4 · Confirming moving averages", "Long", Live, &[],
            &["E|\"1. Buy signals are rendered when the index or stock price employed crosses above both the shorter and longer term moving average\" (p.34)",
              "X|\"2. Sell signals are rendered when the index or stock price crosses below either the shorter term or the longer term moving average.\" (p.34)",
              "X|Short side (p.side=0 only): \"3. ... sell short and remain short only when the stock price ... crossed below both\" (p.34-35), cover when above either (Table 4-4).",
              "F|Exponential averages with smoothing constants 0.15 and 0.04 (Table 4-5: \"buy and sell signals only; (no sell short signals)\", p.36); started from the first close."],
            "Book: NYSE Index Feb 1970-Jan 1978 (Table 4-5, p.36): 0.15 alone 112 trades, 41% profitable, 45.67 points; 0.15 confirmed by 0.04: 57 trades, 24 profitable (42%), 31.04 points, 1.1%/trade. 0.08 & 0.04: 55 trades, 21 profitable, 38.83 units (p.36). NYSE Composite is not in the vault: run on SPX/DJI/NIFTY. Long only by default (as tested).",
        ),
        d("apsmts_shift", "Shifting moving average system (0.15 and 0.075 exponential averages)", "Ch 4 · Shifting moving average system", "Long", Live, &[],
            &["E|\"Buy when the NYSE Index crosses from below to above its 0.15 exponential average.\" (Table 4-6, p.39; rule 2, p.37)",
              "X|\"Sell when the NYSE Index crosses from above to below its 0.15 exponential average\" (Table 4-6) \"except when the sell signal occurs at a price level higher than the preceding sell signal\" (rule 3, p.37)",
              "X|\"4. If your sell signal occurs at a higher price than the preceding sell signal, then you disregard the sell signal unless and until the price ... crosses below both the shorter term and the longer term moving average.\" (p.37)",
              "F|\"Preceding sell signal\" = the previous downside crossing of the 0.15 average, acted on or disregarded (p.prev=1: the previous executed sale). The first sale has no preceding signal and is taken."],
            "Book: NYSE Index 1970-78 (Table 4-6, p.39): 105 trades, 40 profitable (38.1%), 54.08 points, 1.10% and .52 points per trade. Long only (\"Buy ... Sell\"). The caption says \"await a crossing from above to below the 0.075 exponential average\"; the text rule 4 (below both) is coded - they differ only when the close is already under the 0.075 average on the disregarded day.",
        ),
        d("apsmts_mmacd", "Multiple moving average convergence-divergence trading system (MMACDTS)", "Ch 5 · MMACDTS", "Long", Live, &[],
            &["F|\"You may simply plot the difference between the short term moving average and the longer term with the formula, Short Term M.A. - Long Term M.A.\" (p.42); 0.15 and 0.075 exponential averages (Table 5-1, Figure 5-2)",
              "E|\"As long as your plot is rising, you will be on a buy signal.\" (p.42) Buy on the close of a day the difference rises.",
              "X|\"When your plot falls, you will be on a sell signal.\" (p.42) Sell on the close of a day the difference falls."],
            "Book: NYSE Index 1970-78 (Table 5-1, p.43): 0.15-0.075: 152 trades, 72 profitable, 55.70 points, .37/trade, .61%; 0.08-0.04: 141 trades, 55 profitable, 55.49 points. The book does not say whether sells were shorted; long only by default, p.side=0 long and short.",
        ),
        d("apsmts_lead", "Moving average plotted with a lead time (8-day, 1-day lead)", "Ch 6 · Lead time", "Long/Short", Live, &[],
            &["E|\"Buy signals are generated when the market average or stock price crosses from below to above the moving average plot that appears on the same time line in the plot as the most recent data.\" (p.53)",
              "X|\"Sell signals are rendered via a downside penetration ... if we are employing an 8-day moving average with a 1-day lead time, the latest price entry ... will have to cross above or below that one day old moving average to render an action signal.\" (p.53)"],
            "Book: NYSE Index 1970-78, Table 6-2 (p.54): 8-day basic vs 8-day/1-day lead, 50-day basic vs 50-day/15-day lead, trades and points per year (NYSE Composite not in the vault). Straight averages. Long and short like the basic system it is compared with (Ch 2); p.side=1 long only.",
        ),
        d("apsmts_chord", "Moving average/chord trading system (15-week average, 3-week chord)", "Ch 6 · Moving average/chord (Dunn & Hargitt)", "Long", Live, &[],
            &["F|Weekly closes on Wednesday (\"Weekly stock prices ... using mid-week daily close on Wednesday\", Table 6-3 note, p.57); straight moving average (p.55)",
              "E|\"Buy signals are rendered whenever the slope of the chord you are employing in conjunction with your selected moving average turns positive. This means the latest posting of the moving average has turned higher than its reading a chord length previous.\" (p.56)",
              "X|\"Sell signals are rendered when the slope of the chord employed turns negative.\" (p.56)",
              "F|Default = Figure 6-3: \"a 15-week moving average, coupled with a 3-week chord\" (p.56)."],
            "Book: 219 optionable stocks 1967-78 (Table 6-3, p.57), long only and long & short, excess annual return vs buy and hold; cannot be compared on an index. Long only by default (first half of Table 6-3); p.side=0 long and short. p.wd=1 uses Friday closes.",
        ),
        d("apsmts_ad_obos", "A-D overbought-oversold oscillator (10-day total, +-2,000) · NYSE", "Ch 7 · Advance-decline overbought-oversold indicator", "Long", Live, NYSE_AD,
            &["F|\"Maintain a 10-day moving total of the daily differences between advancing issues and declining issues\" on the NYSE; overbought +2,000, oversold -2,000 (p.67-68; Trendline scale +-4.00 x 50 x 10)",
              "E|\"Buy signals are rendered when this indicator falls to below the oversold line then emerges to above the oversold line.\" (p.67)",
              "X|\"Sell signals are rendered when this indicator rises to above the overbought line then falls to below the overbought line.\" (p.67)"],
            "Book's signal list (p.68): buys early June 1977, November 1977, February 1978, December 1978, February 1979; sells November 1977, May 1978, August 1978, January 1979, March 1979. Long only (buy/sell).",
        ),
        d("apsmts_ad_obos_nse", "A-D overbought-oversold oscillator (10-day total, +-2,000) · NSE", "Ch 7 · Advance-decline overbought-oversold indicator", "Long", Live, NSE_AD,
            &["F|10-day moving total of NSE advances minus declines; +-2,000 lines (p.67-68)",
              "E|Buy when it falls below -2,000 then emerges above it (p.67)", "X|Sell when it rises above +2,000 then falls below it (p.67)"],
            "India mapping: NSE breadth; the +-2,000 lines stay as printed (labelled p.level to vary).",
        ),
        d("apsmts_channel", "Calendar channel reverse system on closing levels (25-day)", "Ch 9 · Channel reversal", "Long/Short", Live, &[],
            &["E|\"2. A buy signal is rendered whenever the stock (or stock market average) rises above the highest price level attained within the specified reversal period.\" (p.100)",
              "X|\"A sell signal is rendered whenever the stock ... falls below the lowest low point attained during the specified reversal period.\" (p.100)",
              "F|\"3. In the case of market averages ... closing penetrations of the highest and lowest closing averages for the periods employed\" (p.100); e.g. \"whenever the NYSE Index closes at a level which surpasses the highest closing level of the previous four trading days ... sell when the NYSE Index falls to a level below the lowest close of the previous four trading days\" (p.100)",
              "F|Default 25 days (\"the 25-day reversal system\", Table 9-1 and the 5-6 week rules p.135-136); p.n=4 = Table 9-1's short-term column."],
            "Book: NYSE Index 1970 - 11/17/1978 (Table 9-1, p.103): 4-day 290 trades (148 profitable), 136 points; 25-day 58 trades (25 profitable), 39 points. Honeywell 29-day and IBM 30-day long and short (Table 9-2). Long and short by default; p.side=1 long only.",
        ),
        d("apsmts_cats_dogs", "Cats and dogs relative strength major trend indicator", "Ch 14 · Major trend systems", "Long/Short", NeedsData("needs weekly closes of the OTC (Nasdaq) Composite and the NYSE Composite Index (not in the vault)"), &[],
            &["F|\"A relative strength ratio was calculated at the end of each trading week by dividing the closing level of the Over-the-Counter (OTC) Composite Index by the closing level of the NYSE Index.\" .20 exponential average of the ratio, rounded to two decimals; ties keep the current signal (p.182)",
              "E|\"A buy signal is rendered when the relative strength ratio ... crosses from below to above its .20 exponential average.\" (p.182)",
              "X|\"A sell (and, if you choose, sell short) signal is rendered when the relative strength ratio ... crosses from above to below its .20 exponential average.\" (p.183)"],
            "Book: Table 14-1 (p.184), Value Line Composite traded 1972-79, 20 trades, +76.11/-33.09 points.",
        ),
        nt("apsmts_ttm", "Time-Trend-Momentum Trading System", "Ch 12-13 · Time-Trend-Momentum", "discretionary: \"at times signals may prove open to some subjectivity\" (p.138); double bottoms, \"clearly defined pattern of rising bottoms\", \"starts to turn down\" and the fail-safe \"unless you believe that a bull market is in effect\" (p.151) are judgement. Tables 12-1/12-2 (p.154-155) list 40 buys and 40 short sales on the NYSE Index 1970-79."),
        nt("apsmts_swing10wk", "10-week moving average swing system", "Ch 8 · 10-week swing", "discretionary: buy/sell codes rely on \"spikes\", \"shortly following\" and \"a bit of subjectivity\"; \"the 10-week moving average swing system does involve both some judgment\" (p.82). Tables 8-1/8-2 list NYSE trades 1966-79."),
        nt("apsmts_ma_cross", "Basic multiple moving average crossing system (3/10, 3/30, 3/50, 10/30, 10/50)", "Ch 4 · Multiple moving averages", "duplicate of kftsm_hh_cross (shorter simple average crosses the longer, long and short, p.28-30); results only for NYSE, Honeywell and IBM (Tables 4-2, 4-3) - run kftsm_hh_cross with p.fast/p.slow"),
        nt("apsmts_direction", "Change of direction of a moving average (10- and 34-day)", "Ch 6 · Change of direction", "duplicate of kftsm_ma (buy when the simple average turns up, sell when it turns down, p.50-51; run kftsm_ma with p.period=10 or 34); Table 6-1 is NYSE only"),
        nt("apsmts_filter5", "Five percent filter (quoted from Merrill)", "Ch 9 · Channel reversal", "quoted, not tested by the authors, and \"latest low point\" is not defined (p.99)"),
        nt("apsmts_excess", "Stock excess bands (+-6 points from the 150-day average), % of stocks above 30-week average, specialist short sales", "Ch 7 · Market excesses", "per-stock levels chosen by eye; % above 30-week: \"simple and effective buy and sell signals could [not] be developed\" (p.69-71)"),
        nt("apsmts_roc", "5-, 12- and 25-day rate-of-change oscillators", "Ch 10 · Market momentum", "\"too sensitive to generate valid buy and sell signals on its own\" (p.114); 12-day rules are expectations (\"expect only ...\", p.120)"),
        nt("apsmts_cycles", "Time cycles and phases", "Ch 11 · Time cycles", "cycle lows and phases are judged on charts (p.125-136)"),
        nt("apsmts_zero_balance", "Zero balance major trend trading system", "Ch 14 · Zero balance", "discretionary: reversal points are located by eye on a chart (p.186-191); Table 14-2 S&P 500 1971-79"),
        nt("apsmts_market_logic", "Market Logic daily composite", "Ch 15 · One final trading system", "probabilities of a rise next day, no trade rule (\"will probably not suffice for in and out trading\", p.204)"),
        nt("apsmts_options_funds", "Option scale trading, discount brokers, switch funds", "Ch 15 · Portfolio management", "not market-timing rules (p.194-202)"),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "apsmts_pen" | "apsmts_lead" => ("apsmts_ma", "Moving average penetration systems", if id == "apsmts_pen" { "Basic" } else { "Lead time" }),
        "apsmts_confirm" | "apsmts_shift" | "apsmts_mmacd" => ("apsmts_two_ma", "Two-average systems", match id { "apsmts_confirm" => "Confirming", "apsmts_shift" => "Shifting", _ => "MMACDTS" }),
        "apsmts_chord" => ("apsmts_ma", "Moving average penetration systems", "Chord"),
        "apsmts_ad_obos" => ("apsmts_ad", "A-D overbought-oversold", "NYSE"),
        "apsmts_ad_obos_nse" => ("apsmts_ad", "A-D overbought-oversold", "NSE"),
        "apsmts_channel" => ("apsmts_channel", "Channel reversal", "Closing levels"),
        s if s.starts_with("apsmts_") => ("apsmts_other", "Other Stock Market Trading Systems material", ""),
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    let side = |def| p("side", "1 = long only (sell = flat); 0 = long and short", def, 0., 1., 1.);
    Some(match id {
        "apsmts_pen" => vec![p("n", "Moving average days", 150., 2., 250., 1.), p("kind", "0 straight, 1 weighted, 2 exponential", 2., 0., 2., 1.),
            p("filt", "Penetration filter % (Ch 3)", 0., 0., 3., 0.25), side(0.)],
        "apsmts_confirm" => vec![p("a1", "Shorter exponential constant", 0.15, 0.05, 0.3, 0.01), p("a2", "Longer exponential constant", 0.04, 0.01, 0.1, 0.01), side(1.)],
        "apsmts_shift" => vec![p("a1", "Shorter exponential constant", 0.15, 0.05, 0.3, 0.01), p("a2", "Longer exponential constant", 0.075, 0.02, 0.15, 0.005),
            p("prev", "Preceding sell = 0 any downside crossing, 1 last executed sale", 0., 0., 1., 1.)],
        "apsmts_mmacd" => vec![p("a1", "Shorter exponential constant", 0.15, 0.05, 0.3, 0.01), p("a2", "Longer exponential constant", 0.075, 0.02, 0.15, 0.005), side(1.)],
        "apsmts_lead" => vec![p("n", "Moving average days", 8., 2., 100., 1.), p("lead", "Lead days", 1., 1., 30., 1.), side(0.)],
        "apsmts_chord" => vec![p("n", "Moving average weeks", 15., 3., 30., 1.), p("chord", "Chord weeks", 3., 1., 20., 1.), p("wd", "Weekly close: 0 Wednesday (book), 1 Friday", 0., 0., 1., 1.), side(1.)],
        "apsmts_ad_obos" | "apsmts_ad_obos_nse" => vec![p("n", "Moving total days", 10., 5., 20., 1.), p("level", "Overbought/oversold line", 2000., 500., 5000., 100.)],
        "apsmts_channel" => vec![p("n", "Reversal period (trading days)", 25., 2., 60., 1.), side(0.)],
        _ => return None,
    })
}

fn ymd(s: &str) -> (i64, i64, i64) {
    (s[0..4].parse().unwrap_or(0), s[5..7].parse().unwrap_or(0), s[8..10].parse().unwrap_or(0))
}

/// Days since 1970-01-01 (Hinnant days_from_civil).
pub fn days(s: &str) -> i64 {
    let (y, m, d) = ymd(s);
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * ((m + 9) % 12) + 2) / 5 + d - 1;
    era * 146097 + yoe * 365 + yoe / 4 - yoe / 100 + doy - 719468
}

/// Mid-week bars: per Monday-Friday week, the last bar falling on Monday-Wednesday. A week whose
/// Wednesday may still be to come (data ends Monday/Tuesday) is left out.
fn wed_bars(x: &Ctx) -> Vec<usize> {
    let n = x.r.len();
    let wd = |i: usize| (days(&x.r[i].date) + 3).rem_euclid(7); // 0 = Monday
    let wk = |i: usize| (days(&x.r[i].date) + 3).div_euclid(7);
    (0..n)
        .filter(|&i| wd(i) <= 2 && (wd(i) == 2 || (i + 1 < n && (wk(i + 1) != wk(i) || wd(i + 1) > 2))))
        .collect()
}

/// EMA with smoothing constant a, started from the first close, NaN until it has run N = 2/a - 1 days (p.8-9).
fn ema_stab(v: &[f64], a: f64) -> Vec<f64> {
    let n = (2.0 / a - 1.0).round().max(1.0) as usize;
    let mut e = ema_a(v, a);
    let s = super::apwms::seen(v);
    for (i, x) in e.iter_mut().enumerate() {
        if s[i] < n {
            *x = f64::NAN;
        }
    }
    e
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, prm: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    if from > to || to >= x.r.len() {
        return None;
    }
    let g = |k: &str| prm.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).unwrap().def);
    let u = |k: &str| g(k).round().max(1.0) as usize;
    let lo = schema.iter().any(|d| d.key == "side") && g("side") > 0.5;
    let c = &x.c;
    let daily: Vec<usize> = (0..x.r.len()).collect();
    let fin = |v: f64| v.is_finite();
    Some(match id {
        "apsmts_pen" => {
            let n = u("n");
            let ma = match g("kind").round() as i64 {
                0 => sma(c, n),
                1 => wma(c, n),
                _ => ema_stab(c, 2.0 / (n as f64 + 1.0)),
            };
            let f = g("filt") / 100.0;
            run_ls(x, &daily, from, to, lo, |_, i| {
                if !fin(ma[i]) {
                    None
                } else if c[i] > ma[i] * (1.0 + f) {
                    Some((1, "close above the moving average".into()))
                } else if c[i] < ma[i] * (1.0 - f) {
                    Some((-1, "close below the moving average".into()))
                } else {
                    None
                }
            })
        }
        "apsmts_confirm" => {
            let (e1, e2) = (ema_stab(c, g("a1")), ema_stab(c, g("a2")));
            run_ls(x, &daily, from, to, lo, |_, i| {
                if !(fin(e1[i]) && fin(e2[i])) {
                    return None;
                }
                let (ab, bl) = (c[i] > e1[i] && c[i] > e2[i], c[i] < e1[i] && c[i] < e2[i]);
                Some(if ab { (1, "close above both averages".into()) } else if bl { (-1, "close below both averages".into()) } else { (0, "close between the averages".into()) })
            })
        }
        "apsmts_shift" => {
            let (e1, e2) = (ema_stab(c, g("a1")), ema_stab(c, g("a2")));
            let every = g("prev") < 0.5;
            let (mut hold, mut waiting, mut prev_sell) = (false, false, f64::NAN);
            run_ls(x, &daily, from, to, true, |_, i| {
                if i == 0 || !(fin(e1[i]) && fin(e2[i]) && fin(e1[i - 1])) {
                    return None;
                }
                let up = c[i - 1] <= e1[i - 1] && c[i] > e1[i];
                let dn = c[i - 1] >= e1[i - 1] && c[i] < e1[i];
                if !hold {
                    if up {
                        (hold, waiting) = (true, false);
                        return Some((1, "close crossed above the 0.15 average".into()));
                    }
                    return None;
                }
                if dn {
                    let higher = fin(prev_sell) && c[i] > prev_sell;
                    if every || !higher {
                        prev_sell = c[i];
                    }
                    if !higher {
                        hold = false;
                        prev_sell = c[i];
                        return Some((0, "close crossed below the 0.15 average".into()));
                    }
                    waiting = true;
                }
                if waiting && c[i] < e1[i] && c[i] < e2[i] {
                    (hold, waiting, prev_sell) = (false, false, c[i]);
                    return Some((0, "higher sell disregarded; close below both averages".into()));
                }
                None
            })
        }
        "apsmts_mmacd" => {
            let (e1, e2) = (ema_stab(c, g("a1")), ema_stab(c, g("a2")));
            let df: Vec<f64> = e1.iter().zip(&e2).map(|(a, b)| a - b).collect();
            run_ls(x, &daily, from, to, lo, |_, i| {
                if i == 0 || !(fin(df[i]) && fin(df[i - 1])) {
                    None
                } else if df[i] > df[i - 1] {
                    Some((1, "difference of the averages rising".into()))
                } else if df[i] < df[i - 1] {
                    Some((-1, "difference of the averages falling".into()))
                } else {
                    None
                }
            })
        }
        "apsmts_lead" => {
            let (ma, lead) = (sma(c, u("n")), u("lead"));
            run_ls(x, &daily, from, to, lo, |_, i| {
                if i < lead || !fin(ma[i - lead]) {
                    None
                } else if c[i] > ma[i - lead] {
                    Some((1, "close above the lead-time average".into()))
                } else if c[i] < ma[i - lead] {
                    Some((-1, "close below the lead-time average".into()))
                } else {
                    None
                }
            })
        }
        "apsmts_chord" => {
            let idx = if g("wd") > 0.5 { bars(x, 1) } else { wed_bars(x) };
            let cl: Vec<f64> = idx.iter().map(|&i| c[i]).collect();
            let (ma, ch) = (sma(&cl, u("n")), u("chord"));
            run_ls(x, &idx, from, to, lo, |k, _| {
                if k < ch || !(fin(ma[k]) && fin(ma[k - ch])) {
                    None
                } else if ma[k] > ma[k - ch] {
                    Some((1, "chord slope turned positive".into()))
                } else if ma[k] < ma[k - ch] {
                    Some((-1, "chord slope turned negative".into()))
                } else {
                    None
                }
            })
        }
        "apsmts_ad_obos" | "apsmts_ad_obos_nse" => {
            let Some(p) = a_minus_d(x, id.ends_with("_nse")) else { return Some(vec![]) };
            let (t, lv) = (moving_total(&p, u("n")), g("level"));
            let (mut below, mut above) = (false, false);
            run_ls(x, &daily, from, to, true, |_, i| {
                if !fin(t[i]) {
                    return None;
                }
                let mut out = None;
                if t[i] < -lv {
                    below = true;
                } else if below && t[i] > -lv {
                    below = false;
                    out = Some((1, "A-D total emerged above the oversold line".into()));
                }
                if t[i] > lv {
                    above = true;
                } else if above && t[i] < lv {
                    above = false;
                    out = Some((0, "A-D total fell back below the overbought line".into()));
                }
                out
            })
        }
        "apsmts_channel" => {
            let n = u("n");
            run_ls(x, &daily, from, to, lo, |_, i| {
                if i < n {
                    return None;
                }
                let w = &c[i - n..i];
                if c[i] > w.iter().copied().fold(f64::NEG_INFINITY, f64::max) {
                    Some((1, "close above the highest close of the period".into()))
                } else if c[i] < w.iter().copied().fold(f64::INFINITY, f64::min) {
                    Some((-1, "close below the lowest close of the period".into()))
                } else {
                    None
                }
            })
        }
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ema_matches_the_books_worked_example() {
        // p.8: 53.00 then 53.50 with 0.18 -> 53.09; then 52.75 -> 53.025
        let e = ema_a(&[53.0, 53.5, 52.75], 0.18);
        assert!((e[1] - 53.09).abs() < 1e-9);
        assert!((e[2] - 53.0288).abs() < 1e-4);
    }
    #[test]
    fn days_and_weekdays() {
        assert_eq!(days("1970-01-01"), 0);
        assert_eq!((days("2024-01-10") + 3).rem_euclid(7), 2); // a Wednesday
    }
}
