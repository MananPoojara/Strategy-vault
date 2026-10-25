//! Price Headley, "Big Trends in Trading" (Wiley 2002). Only the rules the book prints with an exit are Live:
//! the VIX Bollinger system (p.39-40, TradeStation code), the weekly VIX > 45 buy (p.44-46), Acceleration Bands
//! (p.91-95, TradeStation formula) and the QQQ/SPY volume Bollinger system (p.50-55). Sentiment systems that need
//! put/call, Rydex or survey data are NeedsData; judgement material is NotTestable. Sources: library/phbt/.
use crate::connors::{sma, claim, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};
/// Admitted ids (authors/headley/admission/SOURCE-CONTRACT.md).
pub const REVIEWED: &[&str] = &["phbt_vix_bb", "phbt_vix45_weekly", "phbt_accel_daily", "phbt_accel_weekly", "phbt_accel_monthly"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const BOOK: &str = "phbt";

pub fn catalogue() -> Vec<StrategyDef> {
    let mk = |id, name, group, dir, status, rules, notes, claim| StrategyDef {
        id, name, group, session: 0, dir, status, needs_volume: false, needs_vix: false,
        needs_series: &[], book: BOOK, rules, notes, claim,
    };
    let accel_rules: &'static [&'static str] = &[
        "F|UpperBand = Average(high x (1 + 2 x ((((high - low)/((high + low)/2)) x 1000) x .001)), 20) (book p.92, TradeStation EasyLanguage)",
        "E|\"Once I see two consecutive closes above the upper Acceleration Band, I get a buy signal.\" (p.92-94). Two consecutive bar closes above that bar's upper band; buy at the next bar's open (\"with the entry coming at the next morning's open\", p.94; weekly: \"The entry the following Monday morning\", p.94).",
        "X|\"One close back into the Acceleration Band signals a traditional exit of the trade\" (p.94): first close below the upper band; exit at the next bar's open (\"the system's exit signal came on the next morning's opening\", p.94).",
        "F|Long only: the book gives no exit for its remark that breaks back under the band \"can often prove to be great shorting points\" (p.95), so no short side is coded.",
    ];
    let accel_note = "Book: stocks (CMGI daily 11/18/1999 entry 63.25 -> exit 12/31/1999 open 135.25; SONE weekly 1999) and indexes (Russell 2000, QQQ, Nasdaq, S&P 500 charts p.97-101, no trade lists). Run on the vault's indexes/ETFs and NSE basket stocks. Bars: weekly = Monday-Friday calendar weeks, monthly = calendar months, built from the daily rows; signals on the period's last daily close, fills at the next trading day's open. Band factor 2 and length 20 are the book's (p.92); [Book silent] equality (close = band) is neither above nor back in.";
    let vix_rules: &'static [&'static str] = &[
        "E|LONG ENTRY: If Close > Average(Close,10) and Price1[5] < BollingerBand(Close of Data2,21,2)[5] and Price[6] > BollingerBand(Close of Data2,21,2)[6] Then Buy on Close (p.39). Data2 = the VIX: \"the VIX closes above the upper Bollinger Band, followed the next day by a close below the upper band ... confirmed five days later by a close in the OEX above the 10-day simple moving average\" (p.39).",
        "X|LONG EXIT: If BarsSinceEntry=5 and c < c[5] Then ExitLong on Close (p.39-40) - \"if it is unprofitable exactly five days after entry\"; or \"if it is reversed by a sell signal\" (p.39).",
        "E|SHORT ENTRY: If c < Average(c,10) and Price1[10] > BollingerBand(Close of Data2,21,-2)[10] and Price1[11] < BollingerBand(Close of Data2,21,-2)[11] Then Sell on Close (p.39-40) - VIX close below the lower band, next close back above it, sell signal 10 trading days later with a close below the 10-day SMA (p.39).",
        "X|SHORT EXIT: If BarsSinceEntry=5 and c > c[5] Then ExitShort on Close (p.40), or reversed by a buy signal (p.39).",
        "F|BollingerBand = 21-bar simple average of the VIX close +/- 2 x population standard deviation (TradeStation StdDev). A signal in the direction already held is ignored; an opposite signal closes and reverses at the same close. side = 1 long only / -1 short only: an opposite signal still exits (\"reversed by a sell signal\").",
    ];
    vec![
        mk("phbt_vix_bb", "VIX Bollinger Band system (OEX vs VIX)", "Ch. 3 · The CBOE Volatility Index", "Long/Short", Status::Live, vix_rules,
            "Book: S&P 100 (OEX) daily 01/02/1990-09/24/2001 with the CBOE VIX of the time (OEX-based, today's VXO: src = 1 reads VXO_OHLC; default src = 0 = the server's VIX: CBOE VIX in the US, India VIX in India). OEX is not in the vault: verified on SPX (the S&P 500 has the same constituents' top end; point P&L differs, counts and win% comparable). India: NIFTY 50 + India VIX (from 2008/2009 in the vault). Figure 3.5's MACD trend-filter variant is not coded (filter not specified).",
            claim("50", "32%", "9.99 OEX points (net 499.56; avg win 50.03 / avg loss -8.85)", "132 bars winners / 8 bars losers", "OEX daily 01/02/1990-09/24/2001, Fig 3.4 p.43")),
        mk("phbt_vix_bb_macd", "VIX Bollinger system with long-term (MACD) trend filter", "Ch. 3 · The CBOE Volatility Index", "Long/Short",
            Status::NotTestable("the filter is only described as \"buy parameters on higher VIX readings if the S&P 100 Index (OEX) has a MACD signal that momentum is favorable\" (p.42-43); MACD settings and the 'favorable' condition (and any sell-side filter) are not given"),
            &["F|\"only follow buy parameters on higher VIX readings if the S&P 100 Index (OEX) has a Moving Average Convergence Divergence (MACD - see Chapter 4) signal that momentum is favorable\" (p.42-43)", "E|Otherwise as phbt_vix_bb (p.39-40)"],
            "Book result Fig 3.5 p.43: 55 trades, 49% profitable, net 285.95 OEX points, OEX daily 01/02/1990-09/24/2001.", None),
        mk("phbt_vix45_weekly", "Buy on weekly VIX spikes over 45, hold 35 weeks", "Ch. 3 · The CBOE Volatility Index", "Long", Status::Live,
            &["E|\"CBOE Volatility Index - Buy on weekly spikes over 45 percent\" (p.44). The week the VIX \"shot up above the 45 percent mark\" gives \"a buy signal on the end of the week's close\" (p.44); system label \"vix > 45\" (Fig 3.8 p.46).",
              "F|Spike = the week's highest VIX reading > 45 (basis = 0, default). Read from the book's own signals: Oct 1997 \"spike in the VIX ... to the 55 level that week\", Sep 1998 \"spiked over the 50 mark that week\", Sep 2001 \"reaching as high as 57.31\" (p.44-45) - intraday highs; the old VIX's weekly closes stayed under 45 in Oct 1997. basis = 1 = weekly close > 45 [non-default reading].",
              "X|\"The optimal hold time on this signal historically has been around 35 weeks\"; exit at the week's close 35 weeks after the buy (\"The exit signal came in May of 1999, 35 weeks after the buy signal\", p.44-45).",
              "F|Weeks = Monday-Friday calendar weeks built from the daily rows; entry and exit at the last daily close of the week. No new signal while a position is held."],
            "Book: OEX weekly 01/05/1990-09/28/2001 with the old VIX (= VXO; src = 1 reads VXO_OHLC, which reproduces the book's weekly highs: 55.48 on 10/28/1997, 53.43 on 9/1/1998). Printed trades: buy week of 10/31/1997 at OEX 436.71 -> exit early July 1998 at 559.37 (+28.1%); buy week of 9/4/1998 at 477.43 -> exit May 1999 at 681.74 (+42.8%); open signal 9/21/2001 at 491.70 (p.44-45). India: India VIX (rarely above 45 except 2008/2020).",
            claim("2", "100%", "163.49 OEX points (+28.1% / +42.8%)", "35 weeks", "OEX weekly 01/05/1990-09/28/2001, Fig 3.8 p.46")),
        mk("phbt_accel_daily", "Acceleration Bands breakout (daily)", "Ch. 7 · Acceleration Bands", "Long", Status::Live, accel_rules, accel_note, None),
        mk("phbt_accel_weekly", "Acceleration Bands breakout (weekly)", "Ch. 7 · Acceleration Bands", "Long", Status::Live, accel_rules, accel_note, None),
        mk("phbt_accel_monthly", "Acceleration Bands breakout (monthly)", "Ch. 7 · Acceleration Bands", "Long", Status::Live, accel_rules, accel_note, None),
        StrategyDef { needs_volume: true, ..mk("phbt_volume_bb", "QQQ / SPY volume Bollinger Band system", "Ch. 4 · Volume indicators", "Long/Short", Status::Live,
            &["E|Long Entry: Volume of Data2 > BollingerBand(Volume of Data2,21,2) and Volume of Data2[1] < BollingerBand(Volume of Data,21,2)[1] or Volume of Data2[2] [?] Bollingerband(Volume of Data2,21,2)[2] or Volume of Data2[3] < BollingerBand(Volume of Data2,21,2)[3] (p.55, printed with a missing operator).",
              "F|Read as: today's volume above its upper 21-day 2-sd band AND at least one of the previous 3 days below its own upper band (missing operator = '<' as in the two printed terms; grouping A and (B or C or D)). Basis: \"The surges to the upper bands indicate volume was unduly high, usually right near important market bottoms\"; \"low volume ... under the lower 21-day Bollinger Band ... often occurs right near short-term peaks\" (p.50-51).",
              "X|Long Exit: BarsSinceEntry = 10 (p.55).",
              "E|Short Entry: Volume of Data2[2] > BollingerBand(Volume of Data2,21,-2)[2] and Volume of Data2[3] [?] BollingerBand(Volume of Data2,21,-2)[3] or Volume of Data2[4] < BollingerBand(Volume of Data2,21,-2)[4] (p.55). Read as: volume 2 days ago above its lower band AND (3 or 4 days ago below its lower band).",
              "X|Short Exit: BarsSinceEntry = 2 (p.55).",
              "F|Orders: the code gives no 'on Close'; this book's sentiment systems fill \"on the following morning's opening price\" (p.10), = TradeStation's default next-bar market order. Signals on the close, entries and time exits at the next open; opposite signal reverses; same-direction signal while in position ignored. Data2 = the traded ETF's own volume."],
            "Book: Fig 4.5 QQQ 03/10/1999-09/24/2001 24 trades 67% net 32.38 points; Fig 4.6b SPY 02/03/1993-09/24/2001 84 trades 71% net 67.95 points (p.52-53). Fig 4.8 (QQQ traded on combined QQQ+SPY volume) is not coded. India: NIFTY 50 index volume (where the vault has volume).",
            claim("84", "71%", "0.81 SPY points (net 67.95)", "9 bars winners / 9 bars losers", "SPY daily 02/03/1993-09/24/2001, Fig 4.6b p.53")) },
        mk("phbt_pc_bb_long", "CBOE equity put/call Bollinger buy (15-day hold)", "Ch. 1 · The CBOE Equity Put/Call Ratio", "Long",
            Status::NeedsData("CBOE equity put/call ratio, daily (not in the vault; CBOE publishes it from Nov 2003). No NSE equity-option put/call history either."),
            &["E|Long Entry: Close < BollingerBand(Close,21,2) and Close[1] > BollingerBand(Close,21,2)[1] or Close[2] > BollingerBand(Close,21,2)[2] or Close[3] > BollingerBand(Close,21,2)[3] (p.14; Close = the put/call ratio)", "E|\"you implement your buy order on the following morning's opening price\" (p.10)", "X|Long Exit: BarsSinceEntry=15 (p.14)"],
            "Book Fig 1.7 p.13 (with a 50-day MA filter whose exact rule is not printed): Nasdaq Composite 01/02/1990-09/21/2001, 114 trades, 70%, net 3532.37 points. The printed short side (p.14) contradicts the text (exit after 2 bars vs 'eight trading days' on p.8, +2 band instead of -2) and is not listed as testable.", None),
        mk("phbt_pc_weekly80", "Weekly equity put/call > 80% buy, 10-week minimum, 10-week MA exit", "Ch. 1 · The CBOE Equity Put/Call Ratio", "Long",
            Status::NeedsData("CBOE weekly equity put/call ratio from 1990 (not in the vault)"),
            &["F|\"when the weekly equity put/call ratio finished over 80 percent on a given week\" (p.15)", "E|\"Once the market confirms this with a technical breakout as defined by a weekly close above the high of five weeks ago, you should buy\" (p.15; the confirmation window is not stated)", "X|\"You hold this trade for 10 weeks at a minimum ... then trail with a 10-week moving average so that any weekly close in the Nasdaq Composite under the 10-week average after the first 10 weeks have passed will take you out\" (p.15)"],
            "Book trade list Fig 1.8/1.10 p.15-17 (Nasdaq weekly 1990-2001): 8 trades, 7 winners; e.g. buy 11/16/90 350.85 -> 5/17/91 481.37.", claim("8", "88% (7/8)", "", "", "Nasdaq Composite weekly 07/30/1982-09/21/2001, Fig 1.10 p.17"))
        ,
        mk("phbt_rydex_bb", "Rydex (Nova+OTC)/Ursa ratio Bollinger system", "Ch. 2 · Rydex mutual fund flows", "Long/Short",
            Status::NeedsData("Rydex Nova, OTC and Ursa fund asset totals, daily (not in the vault; no Indian equivalent)"),
            &["E|Buy if the RYDEX (Nova+OTC)/Ursa ratio has a close below the lower 21-day Bollinger Band (-2 sd) followed by a close back above it (p.29)", "X|Hold 30 days, or close if unprofitable after five trading days, or on a sell signal (p.29)", "E|Sell on a close above the upper band (+2 sd) followed by a close back below (p.29)", "X|Hold the short 15 days, or close if unprofitable after five days, or on a new buy signal (p.29)"],
            "Book Fig 2.2 p.29: Nasdaq Composite 03/18/1994-09/24/2001, 73 trades, 51%, net 1474.89.", claim("73", "51%", "20.20 Nasdaq points", "9 / 7 bars", "Nasdaq daily 03/18/1994-09/24/2001, Fig 2.2 p.29")),
        mk("phbt_ii_bb", "Investors Intelligence bulls, 21-week Bollinger", "Ch. 5 · Surveys", "Long/Short",
            Status::NeedsData("Investors Intelligence weekly % bulls (subscription data, not in the vault)"),
            &["E|Buy on a close under the lower 21-period (weekly) Bollinger Band; exit after 50 weeks (p.61)", "E|Sell after a cross over the upper 21-week band followed by a cross back below; hold bearish trades seven weeks (p.61)"],
            "Book Fig 5.3 p.63: DJIA weekly 01/09/1970-09/21/2001, 59 trades, 58%, net 5806 points. The cross-back condition for buys is not restated; read as for Rydex.", None),
        mk("phbt_ii_cross", "Investors Intelligence bulls cross under bears", "Ch. 5 · Surveys", "Long",
            Status::NeedsData("Investors Intelligence weekly % bulls and % bears (not in the vault)"),
            &["E|\"Buy when the Percentage of Bulls crosses under the percentage of bears\" (p.63)", "X|\"exit the bullish position when the percentage of bulls becomes greater than 2.5 times the size of the percentage of bears\" (p.63)"],
            "Book Fig 5.4 p.63: DJIA weekly 01/09/1970-09/21/2001, 25 trades, 72%, net 5026 points.", None),
        mk("phbt_consensus_bb", "Consensus Inc. bulls, 21-week Bollinger (1.5 / -2 sd)", "Ch. 5 · Surveys", "Long/Short",
            Status::NeedsData("Consensus Inc. weekly % bullish (not in the vault)"),
            &["E|Long Entry: Close > Average(close,20) and close of Data2[1] < BollingerBand(close,21,-2)[1] and close of Data2 > BollingerBand(close,21,-2) (p.65)", "X|Long Exit: BarsSinceEntry=39 (p.65)", "E|Short Entry: Close < Average(close,20) and close of Data2[1] > BollingerBand(close,21,1.5)[1] and close of Data2 < BollingerBand(close,21,1.5) (p.65)", "X|Short Exit: BarsSinceEntry=2 (p.65)"],
            "Book Fig 5.6 p.66: DJIA weekly 07/22/1983-09/07/2001, 30 trades, 57%, net 4843.70.", None),
        mk("phbt_surveys_other", "AAII, Market Vane and composite survey Bollinger systems", "Ch. 5 · Surveys", "Long/Short",
            Status::NotTestable("results only (Figs 5.8, 5.10, 5.12); band widths and holding periods are given only as \"26 to 52 weeks\" / \"two weeks\" / \"seven weeks\" ranges (p.70)"),
            &["E|Bollinger band signals on AAII / Market Vane / averaged % bulls (p.67-70)", "X|\"Bullish trades produce better trades with longer holding periods of 26 to 52 weeks, while bearish trades generally have a short rope of two weeks except for the AAII survey, where they were allowed to be held just seven weeks\" (p.69-70)"],
            "Seen; not a complete rule.", None),
        mk("phbt_rs_lines", "Relative strength 5/10/20/50-week line alignment", "Ch. 9 · Relative strength", "Long",
            Status::NeedsData("stock / Nasdaq Composite relative strength per stock: the engine gives a module no benchmark close for basket stocks, and the vault has no Nasdaq Composite series for NSE stocks"),
            &["F|RS = stock close / Nasdaq Composite (or S&P 500) close; smoothed with 5, 10, 20 and 50-week simple averages (p.129)", "E|Ideal: 5-week RS line above the 10-week, above the 20-week, and the 20-week above the 50-week; buy signal when the last one moves into line (QCOM week ending 4/2/1999 at 17.125, p.130-131)", "X|\"The breakdown point ... is signaled by the first cross of a fast RS line under a slower RS line\" (QCOM 5-week under 10-week, week ending 2/11/2000 at 132.00, p.132)"],
            "Stock-selection tool; no performance table.", None),
        mk("phbt_judgement", "Other indicators in the book (seen, not testable)", "Ch. 1-12", "Long/Short",
            Status::NotTestable("no complete exit or the rule is judgement: put/call 80/40 levels (no exit, p.5), total put/call > 1.00 held 'X days' (p.19), rally-day put/call (no exit, p.21), 10/21-day put/call MA cross 'after reaching a past historical extreme' (p.24), 10-day put/call > 60% (forward returns only, p.25-26), Nasdaq/NYSE and Nasdaq/Dow volume ratios (no exit, p.55-57), historical-volatility V-reversals at chart-chosen levels (p.46-48), ADX exits 'if it goes flat' (p.80-82), Elephant Trunk 'extension in price' (p.85), Moving Average Envelopes (no exit, p.102-108), Momentum Divergence indicator (indicator code only, strategy rules not printed, p.109-127), options and money management chapters 10-12"),
            &["F|See the README for page references."], "Seen and classified; see library/phbt/README.md.", None),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "phbt_vix_bb" => ("phbt_vix", "Headley VIX timing", "21-day Bollinger, 5/10-day lag"),
        "phbt_vix_bb_macd" => ("phbt_vix", "Headley VIX timing", "with MACD trend filter (not specified)"),
        "phbt_vix45_weekly" => ("phbt_vix", "Headley VIX timing", "weekly VIX > 45, 35-week hold"),
        "phbt_accel_daily" => ("phbt_accel", "Acceleration Bands", "daily"),
        "phbt_accel_weekly" => ("phbt_accel", "Acceleration Bands", "weekly"),
        "phbt_accel_monthly" => ("phbt_accel", "Acceleration Bands", "monthly"),
        "phbt_volume_bb" => ("phbt_volume", "Volume Bollinger", "QQQ / SPY own volume"),
        "phbt_pc_bb_long" | "phbt_pc_weekly80" => ("phbt_pc", "Put/call ratio", if id == "phbt_pc_bb_long" { "daily Bollinger buy" } else { "weekly > 80%" }),
        "phbt_rydex_bb" => ("phbt_flows", "Rydex flows", "Bollinger"),
        "phbt_ii_bb" | "phbt_ii_cross" | "phbt_consensus_bb" | "phbt_surveys_other" => ("phbt_surveys", "Sentiment surveys", match id {
            "phbt_ii_bb" => "Investors Intelligence Bollinger", "phbt_ii_cross" => "II bulls/bears cross", "phbt_consensus_bb" => "Consensus Bollinger", _ => "AAII / Market Vane / composite" }),
        "phbt_rs_lines" => ("phbt_rs", "Relative strength", "5/10/20/50-week lines"),
        "phbt_judgement" => ("phbt_other", "Other indicators", "not testable"),
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    match id {
        "phbt_vix_bb" => Some(vec![
            p("bb_n", "VIX Bollinger length", 21., 10., 40., 1.), p("bb_k", "VIX Bollinger standard deviations", 2., 1., 3., 0.25),
            p("ma_n", "Index confirmation SMA", 10., 5., 30., 1.), p("lag_long", "Buy lag after the VIX close back inside (bars)", 5., 0., 15., 1.),
            p("lag_short", "Sell lag after the VIX close back inside (bars)", 10., 0., 20., 1.), p("hold", "Exit if unprofitable exactly N bars after entry", 5., 2., 15., 1.),
            p("src", "Volatility index: 0 = server VIX, 1 = VXO_OHLC (book's OEX VIX)", 0., 0., 1., 1.), p("side", "0 long and short (book), 1 long only, -1 short only", 0., -1., 1., 1.),
        ]),
        "phbt_vix45_weekly" => Some(vec![
            p("level", "VIX level", 45., 30., 60., 1.), p("hold", "Holding period (weeks)", 35., 5., 52., 1.),
            p("basis", "0 = weekly high above level (book's examples), 1 = weekly close", 0., 0., 1., 1.),
            p("src", "Volatility index: 0 = server VIX, 1 = VXO_OHLC (book's OEX VIX)", 0., 0., 1., 1.),
        ]),
        "phbt_accel_daily" | "phbt_accel_weekly" | "phbt_accel_monthly" => Some(vec![
            p("len", "Band average length (bars)", 20., 10., 40., 1.), p("factor", "Band width factor", 2., 1., 4., 0.5),
            p("confirm", "Consecutive closes above the upper band", 2., 1., 4., 1.),
        ]),
        "phbt_volume_bb" => Some(vec![
            p("bb_n", "Volume Bollinger length", 21., 10., 40., 1.), p("bb_k", "Volume Bollinger standard deviations", 2., 1., 3., 0.25),
            p("hold_long", "Long exit: BarsSinceEntry", 10., 2., 20., 1.), p("hold_short", "Short exit: BarsSinceEntry", 2., 1., 10., 1.),
            p("side", "0 long and short (book), 1 long only, -1 short only", 0., -1., 1., 1.),
        ]),
        _ => None,
    }
}

/// Bollinger band (mean +/- k x population sd) over n bars; NaN if any value in the window is NaN.
pub(crate) fn bands(v: &[f64], n: usize, k: f64) -> (Vec<f64>, Vec<f64>) {
    let m = sma(v, n);
    let mut up = vec![f64::NAN; v.len()];
    let mut lo = vec![f64::NAN; v.len()];
    for i in 0..v.len() {
        if !m[i].is_finite() { continue; }
        let w = &v[i + 1 - n..=i];
        let sd = (w.iter().map(|x| (x - m[i]).powi(2)).sum::<f64>() / n as f64).sqrt();
        up[i] = m[i] + k * sd;
        lo[i] = m[i] - k * sd;
    }
    (up, lo)
}

/// Days since 1970-01-01 for "YYYY-MM-DD" (proleptic Gregorian).
fn days(d: &str) -> i64 {
    let (y, m, dd): (i64, i64, i64) = (d[0..4].parse().unwrap_or(0), d[5..7].parse().unwrap_or(1), d[8..10].parse().unwrap_or(1));
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + dd - 1;
    era * 146097 + yoe * 365 + yoe / 4 - yoe / 100 + doy - 719468
}

/// Period key per bar: 0 daily (bar index), 1 Monday-Friday week, 2 calendar month.
fn key(x: &Ctx, i: usize, tf: u8) -> i64 {
    let d = &x.r[i].date;
    match tf { 0 => i as i64, 1 => (days(d) + 3).div_euclid(7), _ => d[0..4].parse::<i64>().unwrap_or(0) * 12 + d[5..7].parse::<i64>().unwrap_or(0) }
}

/// Last daily bar of each completed period (the final bar of the data is never a period end: its period may continue).
fn period_ends(x: &Ctx, tf: u8) -> Vec<usize> {
    (0..x.r.len().saturating_sub(1)).filter(|&i| key(x, i, tf) != key(x, i + 1, tf)).collect()
}

fn open_t(dir: f64, i: usize, px: f64, kind: &'static str) -> Trade {
    Trade { dir, legs: vec![Leg { i, px, w: 1. }], exit_i: i, exit_px: px, reason: String::new(), entry_kind: kind, signal_i: i, open: false, exits: vec![] }
}
fn close_t(mut t: Trade, i: usize, px: f64, sig: usize, reason: &str) -> Trade {
    t.exit_i = i; t.exit_px = px; t.signal_i = sig; t.reason = reason.into(); t
}
fn finish(out: &mut Vec<Trade>, cur: Option<Trade>, x: &Ctx, to: usize) {
    if let Some(t) = cur {
        let mut t = close_t(t, to, x.c[to], to, "open");
        t.open = true;
        out.push(t);
    }
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    if from > to || to >= x.r.len() { return None; }
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).unwrap().def);
    let u = |k: &str| g(k).round().max(0.) as usize;
    let vol_name = |field: &str| if g("src") >= 0.5 { format!("VXO_OHLC.{field}") } else { format!("VIX.{field}") };
    let mut out = vec![];
    match id {
        "phbt_vix_bb" => {
            let Some(v) = x.series(&vol_name("Close")) else { return Some(out) };
            let (bn, ma_n, ll, ls, hold) = (u("bb_n").max(2), u("ma_n").max(1), u("lag_long"), u("lag_short"), u("hold").max(1));
            let side = g("side").round();
            let (up, lo) = bands(v, bn, g("bb_k"));
            let ma = sma(&x.c, ma_n);
            let mut cur: Option<Trade> = None;
            for i in from.max(ll.max(ls) + 1)..=to {
                if let Some(t) = cur.as_ref() {
                    let e = t.legs[0].i;
                    if i - e == hold && t.dir * (x.c[i] - x.c[e]) < 0. {
                        out.push(close_t(cur.take().unwrap(), i, x.c[i], i, "unprofitable exactly 5 bars after entry"));
                    }
                }
                let long = x.c[i] > ma[i] && v[i - ll] < up[i - ll] && v[i - ll - 1] > up[i - ll - 1];
                let short = x.c[i] < ma[i] && v[i - ls] > lo[i - ls] && v[i - ls - 1] < lo[i - ls - 1];
                let dir = if long { 1. } else if short { -1. } else { continue };
                if cur.as_ref().is_some_and(|t| t.dir == dir) { continue; }
                if let Some(t) = cur.take() {
                    out.push(close_t(t, i, x.c[i], i, if dir > 0. { "reversed by a VIX buy signal" } else { "reversed by a VIX sell signal" }));
                }
                if i < to && (side == 0. || side == dir) { cur = Some(open_t(dir, i, x.c[i], "close")); }
            }
            finish(&mut out, cur, x, to);
        }
        "phbt_vix45_weekly" => {
            let field = if g("basis") >= 0.5 { "Close" } else { "High" };
            let Some(v) = x.series(&vol_name(field)) else { return Some(out) };
            let (level, hold) = (g("level"), u("hold").max(1));
            let ends = period_ends(x, 1);
            let mut cur: Option<(Trade, usize)> = None; // trade, entry week number in `ends`
            let mut start = 0; // first daily bar of the current week
            for (w, &e) in ends.iter().enumerate() {
                let s = start;
                start = e + 1;
                if e < from || e > to { continue; }
                if let Some((_, k)) = cur.as_ref() {
                    if w - k == hold {
                        let (t, _) = cur.take().unwrap();
                        out.push(close_t(t, e, x.c[e], e, "35 weeks after the buy"));
                    }
                }
                let wk = if field == "High" { v[s..=e].iter().copied().filter(|z| z.is_finite()).fold(f64::NAN, f64::max) } else { v[e] };
                if cur.is_none() && e < to && wk > level {
                    cur = Some((open_t(1., e, x.c[e], "close"), w));
                }
            }
            finish(&mut out, cur.map(|c| c.0), x, to);
        }
        "phbt_accel_daily" | "phbt_accel_weekly" | "phbt_accel_monthly" => {
            let tf = match id { "phbt_accel_daily" => 0, "phbt_accel_weekly" => 1, _ => 2 };
            let (len, fac, conf) = (u("len").max(1), g("factor"), u("confirm").max(1));
            let ends = period_ends(x, tf);
            let (mut hs, mut ls, mut cs) = (vec![], vec![], vec![]);
            let mut s = 0;
            for &e in &ends {
                hs.push(x.h[s..=e].iter().copied().fold(f64::NEG_INFINITY, f64::max));
                ls.push(x.l[s..=e].iter().copied().fold(f64::INFINITY, f64::min));
                cs.push(x.c[e]);
                s = e + 1;
            }
            let raw: Vec<f64> = hs.iter().zip(&ls).map(|(&h, &l)| h * (1. + fac * ((((h - l) / ((h + l) / 2.)) * 1000.) * 0.001))).collect();
            let upper = sma(&raw, len);
            let mut cur: Option<Trade> = None;
            for (j, &e) in ends.iter().enumerate() {
                if e < from || e >= to { continue; }
                if cur.is_some() {
                    if cs[j] < upper[j] {
                        out.push(close_t(cur.take().unwrap(), e + 1, x.o[e + 1], e, "close back below the upper Acceleration Band"));
                    }
                } else if j + 1 >= conf && (j + 1 - conf..=j).all(|k| cs[k] > upper[k]) {
                    let mut t = open_t(1., e + 1, x.o[e + 1], "open");
                    t.signal_i = e;
                    cur = Some(t);
                }
            }
            finish(&mut out, cur, x, to);
        }
        "phbt_volume_bb" => {
            let v: Vec<f64> = x.r.iter().map(|r| r.volume.filter(|z| *z > 0.).unwrap_or(f64::NAN)).collect();
            let (bn, hl, hsh) = (u("bb_n").max(2), u("hold_long").max(1), u("hold_short").max(1));
            let side = g("side").round();
            let (up, lo) = bands(&v, bn, g("bb_k"));
            let mut cur: Option<Trade> = None;
            for i in from.max(4)..to {
                // decisions on the close of i, fills at the open of i + 1
                if let Some(t) = cur.as_ref() {
                    let hold = if t.dir > 0. { hl } else { hsh };
                    if i - t.legs[0].i == hold {
                        out.push(close_t(cur.take().unwrap(), i + 1, x.o[i + 1], i, "BarsSinceEntry exit"));
                    }
                }
                let long = v[i] > up[i] && (v[i - 1] < up[i - 1] || v[i - 2] < up[i - 2] || v[i - 3] < up[i - 3]);
                let short = v[i - 2] > lo[i - 2] && (v[i - 3] < lo[i - 3] || v[i - 4] < lo[i - 4]);
                let dir = match (long, short) { (true, false) => 1., (false, true) => -1., _ => continue };
                if cur.as_ref().is_some_and(|t| t.dir == dir) { continue; }
                if let Some(t) = cur.take() {
                    out.push(close_t(t, i + 1, x.o[i + 1], i, "reversed by an opposite volume signal"));
                }
                if side == 0. || side == dir {
                    let mut t = open_t(dir, i + 1, x.o[i + 1], "open");
                    t.signal_i = i;
                    cur = Some(t);
                }
            }
            finish(&mut out, cur, x, to);
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
        (0..n).map(|i| {
            let d = days("2020-01-01") + i as i64;
            let (y, m, dd) = civil(d);
            Row { ticker: "T".into(), date: format!("{y:04}-{m:02}-{dd:02}"), open: 100., high: 101., low: 99., close: 100., volume: Some(1000.) }
        }).collect()
    }
    fn civil(z: i64) -> (i64, i64, i64) {
        let z = z + 719468; let era = z.div_euclid(146097); let doe = z - era * 146097;
        let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365; let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153; let d = doy - (153 * mp + 2) / 5 + 1; let m = if mp < 10 { mp + 3 } else { mp - 9 };
        (yoe + era * 400 + if m <= 2 { 1 } else { 0 }, m, d)
    }
    #[test]
    fn days_and_weeks() {
        assert_eq!(days("1970-01-01"), 0);
        assert_eq!(days("2000-03-01"), 11017);
        assert_eq!(civil(11017), (2000, 3, 1));
        // Friday 2024-01-05 and Monday 2024-01-08 fall in different Monday-Friday weeks
        assert_ne!((days("2024-01-05") + 3).div_euclid(7), (days("2024-01-08") + 3).div_euclid(7));
        assert_eq!((days("2024-01-08") + 3).div_euclid(7), (days("2024-01-12") + 3).div_euclid(7));
    }
    #[test]
    fn accel_daily_two_closes_above_then_exit_on_close_back() {
        let mut r = rows(60);
        for k in 40..43 { r[k].close = 120.; r[k].high = 121.; } // flat band ~ 101 x 1.04 = 105
        r[43].close = 100.;
        let x = Ctx::new(&r, &r, &[]);
        let ts = run("phbt_accel_daily", &x, 0, 59, &Params::new()).unwrap();
        assert_eq!(ts.len(), 1);
        assert_eq!(ts[0].legs[0].i, 42); // 2nd close above on bar 41 -> next open
        assert_eq!(ts[0].exit_i, 44); // close back below on bar 43 -> next open
    }
    #[test]
    fn bands_population_sd() {
        let (u, l) = bands(&[1., 2., 3., 4.], 4, 2.);
        let sd = (1.25f64).sqrt();
        assert!((u[3] - (2.5 + 2. * sd)).abs() < 1e-12 && (l[3] - (2.5 - 2. * sd)).abs() < 1e-12);
        assert!(u[2].is_nan());
    }
}
