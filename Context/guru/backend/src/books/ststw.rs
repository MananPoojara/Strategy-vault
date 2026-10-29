//! Larry Connors & Cesar Alvarez, "Short Term Trading Strategies That Work" (TradingMarkets, 2008). Book id "ststw".
//! Index/SPY timing rules from Chapters 2, 7, 9, 10 and 12 coded bar by bar (all orders at the close unless stated).
//! Rules already in the vault with identical wording are NOT repeated here (see library/ststw/README.md):
//! TRIN strategy = stc9 `trin3_10`, 2-day RSI(3) < 45 = stc9 `rsi_3`. The VIX RSI strategy (p.96-98) has the same
//! wording as stc9 `vix2` and was added here on 2026-10-21 as `ststw_vix_rsi` so the book carries its own rule and claim.
//! Stock-universe tests (Ch. 4, 6, 8, 9, 11, 13) need a US stock universe and are listed as NeedsData.
#![allow(dead_code)]
use crate::connors::*;

pub const BOOK: &str = "ststw";

/// Admitted after the 2026-10-21 source review (connors/newrules-misc/SOURCE-CONTRACT.md) and an independent ledger check.
pub const REVIEWED_DAILY: &[&str] = &["ststw_vix_rsi"];
/// connors-final (2026-10-26): the Ch 6 / 8 / 13 stock tests coded from the book text and checked with an independent
/// ledger (authors/connors-final). One position at a time per stock; basket runs replay them across a stock universe.
/// The Ch 8 drops tests filter price > $5: US instruments only (user rule: $-amount rules run on US data only).
pub const STOCK_STUDIES: &[&str] = &["ststw_intraday_drops_highs", "ststw_intraday_drops_lows", "ststw_stops_hurt", "ststw_exit_study"];
pub fn stock_study(id: &str) -> bool { STOCK_STUDIES.contains(&id) }
pub fn usd_filter(id: &str) -> bool { matches!(id, "ststw_intraday_drops_highs" | "ststw_intraday_drops_lows") }
pub fn reviewed_daily(id: &str) -> bool {
    REVIEWED_DAILY.contains(&id)
}
/// The VIX RSI rule times the S&P 500 (SPY, "or use the SPX") and reads the VIX open: US only (India VIX opens repeat
/// the previous close).
pub fn admitted_symbol(id: &str, ticker: &str) -> bool {
    reviewed_daily(id) && matches!(ticker, "SPX" | "SPY")
}
/// The served CBOE VIX history has close-only bars before this date.
pub const REAL_VIX_OHLC_FROM: &str = "1992-01-01";

/// [Not from the book] Price-vs-SMA tie tolerance (relative), as in ws5/cwin: a close can equal its SMA exactly in decimal
/// arithmetic but differ by float rounding; within 1e-9 x |SMA| is a tie, which is never "above" or "below".
const TIE: f64 = 1e-9;
fn above(a: f64, ma: f64) -> bool {
    a > ma + TIE * ma.abs()
}
fn below(a: f64, ma: f64) -> bool {
    a < ma - TIE * ma.abs()
}

const STOCKS: &str = "needs a US stock universe (every NYSE/Nasdaq stock 1995-2007 with 100-day average volume >= 250,000 shares and price > $5, incl. delisted names); the vault has index data only";
const IDX: &str = " Book tested the S&P 500 cash index (SPX) / SPY; the vault runs the same rules on NIFTY 50, NIFTY BANK, NIFTY IT, NIFTY MIDCAP 100 (instrument swap only) and SPX/NDX/DJI/SPY/QQQ in the US check. One position at a time, no costs in the book.";

pub fn catalogue() -> Vec<StrategyDef> {
    use Status::*;
    let d = |id, name, group, dir, status, rules, notes, claim| StrategyDef {
        id, name, group, session: 0, dir, status, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim,
    };
    let g2 = "Ch 2 · Rule 1 Buy Pullbacks, Not Breakouts";
    let g9 = "Ch 9 · The 2-period RSI";
    let g12 = "Ch 12 · 5 Strategies to Time the Market";
    vec![
        d("ststw_10d_high_buy", "Buy 10-day highs, exit below 10-day MA", g2, "Long", Live,
          &["E|(p11-13) \"bought every new 10-period high\": today's close is the highest close of the last 10 days (incl. today); buy on the close",
            "X|\"used a trailing stop of exiting when prices closed under their 10-period moving average\": sell on the close when Close < SMA(10)"],
          concat!("p.11-13, Question #1 / answer: SPX 1/1/1995-12/31/2007, lost ~100 S&P points, right 41.1% of the time. 'New 10-period high' read as a closing high (p.hl=1: intraday high): the closing reading gives SPX 169 trades / 41.4% / -81 pts, the intraday reading 39.3% / -299 pts, so the closing reading is the one that fits the book.", " Book tested the S&P 500 cash index; the vault runs the same rules on Indian and US indices (instrument swap only). One position at a time, no costs in the book."),
          claim("-", "41.1%", "-100 S&P pts total (\"nearly\")", "-", "1/1/95 - 12/31/07, SPX")),
        d("ststw_10d_high_short", "Short 10-day highs, cover below 10-day MA", g2, "Short", Live,
          &["E|(p12, p14) \"You SOLD the market as it made new highs\": today's close is the highest close of the last 10 days; sell short on the close",
            "X|\"you exited when the market crossed under its 10-period MA\": cover on the close when Close < SMA(10)"],
          concat!("p.12-14, Question #2: \"made almost 100 points and was correct 59% of the time\". Same entries as ststw_10d_high_buy, mirrored.", " Book tested the S&P 500 cash index; the vault runs the same rules on Indian and US indices (instrument swap only). One position at a time, no costs in the book."),
          claim("-", "59%", "+100 S&P pts total (\"almost\")", "-", "1/1/95 - 12/31/07, SPX")),
        d("ststw_10d_low_buy", "Buy 10-day lows, exit above 10-day MA", g2, "Long", Live,
          &["E|(p12, p14) \"You only bought when the market made new 10-day lows\": today's close is the lowest close of the last 10 days; buy on the close",
            "X|\"exiting when the market closed above its 10-period moving average\": sell on the close when Close > SMA(10)"],
          concat!("p.12-14, Question #3: \"made 1048.70 S&P points being invested only 28.81% of the time. Plus, 73.5% of your trades would have been profitable\".", " Book tested the S&P 500 cash index; the vault runs the same rules on Indian and US indices (instrument swap only). One position at a time, no costs in the book."),
          claim("-", "73.5%", "1,048.70 S&P pts total", "28.81% of the time invested", "1/1/95 - 12/31/07, SPX")),
        d("ststw_overnight", "Overnight: buy the close, sell the next open", "Ch 7 · Rule 6 It Pays to Hold Positions Overnight", "Long", Live,
          &["E|(p37) \"Overnight - Buy the SPY on the close\" (every day)", "X|\"and sell on the next day's open\""],
          concat!("p.36-38, Fig. 7.1 (SPY 1995-2007) and 7.2 (QQQQ 1999-2007): all the market's gains came overnight. The overnight bar value in Fig. 7.1 is not legible in the scan; only the Day bar (-70.88 SPY points) and QQQQ Day (-58.33) are printed.", " Book tested the S&P 500 cash index; the vault runs the same rules on Indian and US indices (instrument swap only). One position at a time, no costs in the book."),
          None),
        d("ststw_intraday", "Day session: buy the open, sell the same close", "Ch 7 · Rule 6 It Pays to Hold Positions Overnight", "Long", Live,
          &["E|(p37) \"Day - Buy the SPY on the open\" (every day)", "X|\"and sell on the same day's close\""],
          concat!("p.37-38: SPY 1995-2007 Day = -70.88 SPY points; QQQQ 1999-2007 Day = -58.33 points.", " Book tested the S&P 500 cash index; the vault runs the same rules on Indian and US indices (instrument swap only). One position at a time, no costs in the book."),
          claim("-", "-", "-70.88 SPY pts total", "same day", "1995 - 2007, SPY")),
        d("ststw_rsi2_under5", "2-period RSI under 5 on the S&P 500", g9, "Long", Live,
          &["F|(p62) \"The S&P 500 index is above its 200-day moving average.\"", "E|\"The 2-period RSI of the S&P 500 Index closes below 5.\"", "E|\"Buy the S&P on the close.\"",
            "X|\"Exit when the S&P closes above its 5-period moving average.\""],
          concat!("p.61-65. Book trade list (p.62) 6/7/07 1490.72 -> 6/13/07 1515.67, 7/27/07 1458.95 -> 8/2/07 1472.2, 10/19/07 1500.63 -> 10/25/07 1514.4 (SPX).", " Book tested the S&P 500 cash index; the vault runs the same rules on Indian and US indices (instrument swap only). One position at a time, no costs in the book."),
          claim("49", "83.6%", "522.92 S&P pts total", "3 days", "1995 - 2007, SPX")),
        d("ststw_cum_rsi_35", "Cumulative RSI · 2-day RSI(2) sum < 35", g9, "Long", Live,
          &["F|(p67) \"The security being used is above its 200-day moving average.\"", "E|\"Use a 2-Period RSI. Take the past X days of the 2-period RSI and add them up.\" X = 2",
            "E|\"Buy if the Cumulative RSI is below Y\" Y = 35; buy on the close", "X|\"Exit when the 2-period RSI closes above 65\""],
          "p.66-68: SPY mid-Jan 1993 - 12/31/2007, X=2, Y=35: 50 signals, 88%, 65.53 SPY points, 1.26% avg, 3.7 days. Defaults are the printed rule (RSI period 2). [Book-vs-results gap] The printed figures are not reproduced with RSI(2) (SPY 113 trades / 84.1% / 0.84%); they are with RSI(3) summed (p.rsi_n=3: SPY 45 / 88.9% / 1.33% / 64.2 pts), the indicator of the book's own Ch 12 cumulative-RSI strategy; kept as a labelled parameter, not the default. Book tested SPY; the vault runs it on Indian and US indices (instrument swap only).",
          claim("50", "88%", "1.26%", "3.7 days", "1/93 - 12/31/07, SPY")),
        d("ststw_cum_rsi_50", "Cumulative RSI · 2-day RSI(2) sum < 50", g9, "Long", Live,
          &["F|(p67) \"The security being used is above its 200-day moving average.\"", "E|2-day sum of the 2-period RSI (X = 2) \"only had to add up to 50\" (Y = 50); buy on the close",
            "X|\"Exit when the 2-period RSI closes above 65\""],
          "p.67-68: SPY 1993-2007, X=2, Y=50: 105 signals, 85.47%, 105.95 SPY points, 1.05% avg, 3.57 days. Same gap as ststw_cum_rsi_35: RSI(2) gives SPY 159 / 79.9% / 0.60%; p.rsi_n=3 gives 95 / 85.3% / 1.06% / 101.7 pts. Book tested SPY; the vault runs it on Indian and US indices (instrument swap only).",
          claim("105", "85.47%", "1.05%", "3.57 days", "1/93 - 12/31/07, SPY")),
        d("ststw_double7", "Double 7's", "Ch 10 · Double 7's Strategy", "Long", Live,
          &["F|(p76) \"The SPY is above its 200-day moving average\"", "E|\"If the SPY closes at a 7-day low, buy.\" (lowest close of the last 7 closes incl. today; buy on the close)",
            "X|\"If the SPY closes at a 7-day high, sell your long position.\" (highest close of the last 7 incl. today; sell on the close)"],
          "p.75-80. SPY 1/29/93 inception - book date: 153 trades, 0.85% avg, 80.4% correct, 122.36 points; QQQQ (3/10/99): 68 / 0.93% / 79.4% / 31.30; FXI 26 / 1.41% / 76.9%; EWZ 63 / 1.82% / 81.0%. \"Double 5's ... Double 10's\" hold up too (p.80): p.n varies both lookbacks. Book tested SPY/QQQQ; the vault runs it on Indian and US indices (instrument swap only).",
          claim("153", "80.4%", "0.85%", "<25% of the time invested", "1/29/93 - 2007, SPY")),
        StrategyDef { needs_vix: true, ..d("ststw_vix_stretch", "VIX Stretches", g12, "Long", Live,
          &["F|(p93) \"The SPY is above its 200-day moving average (you can use the SPX also).\"", "E|\"The VIX is stretched 5% or more above its 10-day MA for 3 or more days. If this occurs, we'll buy the market on the close.\" (VIX close >= 1.05 x SMA(VIX close, 10))",
            "X|\"Exit when the SPY closes above a 2-period RSI reading of 65 or more.\" (RSI(2) > 65)"],
          "p.93-95: 1995-2007, 33 trades, 84.85%, 363.90 SPX points, under five days. India: India VIX for the VIX (window starts 2015). [Book-vs-results gap] The printed rules give SPX 73 trades / 78.1% / +446 pts; the printed 33 trades are reproduced only with an extra RSI(2) < 10 on the trigger day (SPX 34 / 88.2% / +346 pts), the filter of the same strategy in the TradingMarkets course (vault stc9 'vix200'). The default is the printed rule; p.rsi_max=10 applies the course filter [Not from the book].",
          claim("33", "84.85%", "363.90 SPX pts total", "< 5 days", "1995 - 2007, SPX")) },
        StrategyDef { needs_vix: true, ..d("ststw_vix_rsi", "VIX RSI Strategy", g12, "Long", Live,
          &["F|(p96) \"1. The SPY is above its 200-period moving average. ... For those of you who want to keep things simpler, use the SPX.\"",
            "E|\"2. The 2-period RSI of the VIX is greater than 90.\"", "E|\"3. Today's VIX open is greater than yesterday's close\" (the VIX's close)",
            "E|\"4. The 2-period RSI of the SPY is below 30\"", "E|\"5. Buy on the close.\"", "X|\"6. Exit when the 2-period RSI of the SPY closes above 65.\""],
          "p.96-98. Same wording as the TradingMarkets course rule stc9 'vix2'. Reads the VIX open: runs on SPX/SPY with the CBOE VIX only (India VIX opens repeat the previous close; the served CBOE VIX has real opens from 1992). The VIX bar of the instrument's previous trading day is 'yesterday'. Book: 1995-2007 (Ch.12 studies), SPX points.",
          claim("92", "79.35%", "879.46 SPX pts total", "< 5 days", "1995 - 2007, SPX")) },
        d("ststw_sp_short", "The S&P Short Strategy", g12, "Short", Live,
          &["F|(p106) \"The SPY (or the SPX) is below the 200-day MA.\"", "E|\"The market closes up 4 or more days in a row.\"", "E|\"Sell on the close.\"",
            "X|\"Cover your short position when the SPY closes under its 5-period moving average.\""],
          concat!("p.106-108: 1995-2007, 16 trades, 68.75%, 169.91 S&P points, under five days. Differs from stc9 'short_4up' (the course adds Close >= 5-day SMA).", " Book tested the S&P 500 cash index; the vault runs the same rules on Indian and US indices (instrument swap only). One position at a time, no costs in the book."),
          claim("16", "68.75%", "169.91 S&P pts total", "< 5 days", "1995 - 2007, SPX")),
        // ---- stock-universe tests: rules listed, never run on an index
        StrategyDef { needs_volume: true, ..d("ststw_intraday_drops_highs", "Buying into further strength intraday (stocks)", "Ch 8 · Trading with Intra-day Drops", "Long", Live,
          &["F|(p43-44) \"1. Stock closes at a 10-period high and is above its 200-day simple moving average\"", "F|\"2. Average volume over the past 100 days is at least 250,000 shares per day.\" \"3. Price is greater than $5 per share.\"",
            "E|\"4. Buy on the close.\" or (p44) \"Buy on a limit order the next day 1% above the previous day's close. We also tested buying 3% higher, 5% higher, 7% higher and 10% higher\" (param pct; 0 = buy on the close)",
            "X|\"5. Exit on the close 5 trading days later.\""],
          "p.43-46, every stock 1995-2007: close entry 1,264,571 trades / 50% / +0.06%; +1% 597,063 / 49% / +0.02%; +3% 201,156 / 47% / -0.18%; +5% 87,606 / 45% / -0.42%; +7% 44,131 / 43% / -0.62%; +10% 18,784 / 42% / -0.90%. Vault reading: 10-period high = today's close is the highest close of the last 10 days (incl. today); the next-day order to buy x% ABOVE the close is read as a buy stop (\"buy into further strength\"): it fills when the day's high reaches the level, at the level or at the open if the stock opens above it, and is cancelled at that day's close if not reached [declared reading]. Exit at the close 5 trading days after the entry day. One position at a time per stock (a signal during a trade is skipped). Average volume = mean of the last 100 daily share volumes incl. the signal day; price = the signal-day close (split/dividend-adjusted vault prices). US stocks / US index and ETF data only ($ price filter).", None) },
        StrategyDef { needs_volume: true, ..d("ststw_intraday_drops_lows", "Buying into further weakness intraday (stocks)", "Ch 8 · Trading with Intra-day Drops", "Long", Live,
          &["F|(p46) \"1. Stock closes at a 10-period low and is above its 200-day simple moving average\"", "F|\"2. Average volume over the past 100-days is at least 250,000 shares per day.\" \"3. Price is greater than $5 per share.\"",
            "E|\"4. Buy on the close.\" or (p47) \"buying on a limit the next day 1% lower\" (also 3, 5, 7, 10%; param pct, 0 = buy on the close)", "X|\"5. Exit on the close 5 trading days later.\""],
          "p.46-48: close 661,570 / 54% / +0.61%; -1% 342,091 / 55% / +0.62%; -3% 134,436 / 57% / +1.17%; -5% 63,154 / 59% / +1.97%; -7% 33,831 / 62% / +2.95%; -10% 14,939 / 63% / +4.35%. The next-day buy limit x% below the signal close fills when the day's low reaches it, at the limit or at the open if the stock opens below it; cancelled at that day's close. Exit at the close 5 trading days after the entry day. One position at a time per stock. Same filters as the 10-day-high test. US data only ($ price filter).", None) },
        d("ststw_stops_hurt", "Stop test: 10-day low, exit above 10-day MA or x% stop (stocks)", "Ch 6 · Rule 5 Stops Hurt", "Long", Live,
          &["F|(p32) \"A stock is above its 200-day moving average", "E|and closes at a 10-day low.\" Buy on the close [declared: the book's tests buy on the close]", "X|\"The exit is a close above its 10-day moving average, or when a stop is hit x% below the entry\" (param stop: 0 = none, the book's first row; 1, 3, 5, 7, 10, 20, 50)"],
          "p.32 table, stocks: none 236,237 trades / +0.58% / 7.74 bars / 69.81%; 1% 394,480 / 0.19% / 2.89 / 26.89%; 3% 321,824 / 0.20%; 5% 286,991 / 0.20%; 7% 268,410 / 0.22%; 10% 253,863 / 0.27%; 20% 239,605 / 0.39%; 50% 236,337 / 0.56% / 7.70 / 69.81%. The trade count rises with tighter stops, so the test re-enters after a stop: one position at a time per stock. Stop checked from the day after entry on the day's low, filled at the stop or at the open if the stock opens below it; a stop day cannot also be a new entry. The text names no volume/price filter for this test: none by default (param universe 1 = the book's standard stock universe of p.43, 100-day average volume >= 250,000 and price > $5, US data only).", None),
        d("ststw_cum_rsi_stocks", "Cumulative RSI for stocks (sum < 10)", g9, "Long", NeedsData(STOCKS),
          &["F|(p71) Stocks above the 200-day MA, average daily volume 250,000, price > $5", "E|Cumulative (2-day) 2-period RSI below 10; buy on the close", "X|Exit when the 2-period RSI closes above 65"],
          "p.71: 77,068 trades since 1995, 69% profitable.", None),
        d("ststw_exit_study", "Exit comparison: 10-day low + 5% limit (stocks)", "Ch 13 · Exit Strategies", "Long", Live,
          &["F|(p114) \"all stocks above the 200 day MA ... which had closed at a 10-day low", "E|and entered the next day on a 5% limit.\" (a buy limit 5% below the signal close, good for that day)",
            "X|\"We then tested various exits\" (param exit): 1 = 1-day hold, 2 = 3-day hold, 3 = 5-day hold, 4 = 7-day high, 5 = C > MA5, 6 = C > MA10, 7 = RSI2 > 65, 8 = RSI2 > 70, 9 = RSI2 > 75"],
          "p.114 table, 63,101 trades each: 1 day 0.61% / 56%, 3 day 1.76% / 60%, 5 day 1.97% / 59%, 7-day high 2.05% / 58%, C>MA5 2.65% / 70%, C>MA10 2.80% / 69%, RSI2>65 2.76% / 70%, RSI2>70 2.83% / 70%, RSI2>75 2.93% / 69%. The table has no default row: the default is its first row (1-day hold) [declared]. N-day hold = exit at the close N trading days after the entry day; 7-day high = the first close that is the highest close of the last 7 days; MA and RSI2 exits at the close (checked from the entry day's close, as an exit 'on the close'). The same count for every exit means the book counted every signal even while a trade was open; the vault holds one position at a time per stock, so its counts differ by exit. No filter named for this test (param universe 1 = the p.43 standard universe, US data only).", None),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let f = ("ststw_rule1", "Rule 1 · Pullbacks vs breakouts (STSTW)");
    let o = ("ststw_overnight", "Rule 6 · Overnight vs day session (STSTW)");
    let c = ("ststw_cumrsi", "Cumulative RSI (STSTW)");
    Some(match id {
        "ststw_10d_high_buy" => (f.0, f.1, "Buy 10-day highs"),
        "ststw_10d_high_short" => (f.0, f.1, "Short 10-day highs"),
        "ststw_10d_low_buy" => (f.0, f.1, "Buy 10-day lows"),
        "ststw_overnight" => (o.0, o.1, "Overnight"),
        "ststw_intraday" => (o.0, o.1, "Day session"),
        "ststw_rsi2_under5" => ("ststw_rsi2_under5", "2-period RSI under 5 (STSTW)", ""),
        "ststw_cum_rsi_35" => (c.0, c.1, "Sum < 35"),
        "ststw_cum_rsi_50" => (c.0, c.1, "Sum < 50"),
        "ststw_cum_rsi_stocks" => (c.0, c.1, "Stocks, sum < 10 · data missing"),
        "ststw_double7" => ("ststw_double7", "Double 7's (STSTW)", ""),
        "ststw_vix_stretch" => ("ststw_vix_stretch", "VIX Stretches (STSTW)", ""),
        "ststw_vix_rsi" => ("ststw_vix_rsi", "VIX RSI Strategy (STSTW)", ""),
        "ststw_sp_short" => ("ststw_sp_short", "S&P Short Strategy (STSTW)", ""),
        "ststw_intraday_drops_highs" => ("ststw_drops", "Intra-day drops, stocks (STSTW)", "10-day highs"),
        "ststw_intraday_drops_lows" => ("ststw_drops", "Intra-day drops, stocks (STSTW)", "10-day lows"),
        "ststw_stops_hurt" => ("ststw_stops_hurt", "Stops Hurt test, stocks (STSTW)", ""),
        "ststw_exit_study" => ("ststw_exit_study", "Exit comparison, stocks (STSTW)", ""),
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    Some(match id {
        "ststw_10d_high_buy" | "ststw_10d_high_short" | "ststw_10d_low_buy" => vec![
            p("n", "New N-day high/low", 10.0, 5.0, 20.0, 1.0), p("ma", "Exit moving average", 10.0, 5.0, 20.0, 1.0),
            p("hl", "Entry on 0 = closing high/low, 1 = intraday high/low", 0.0, 0.0, 1.0, 1.0)],
        "ststw_overnight" | "ststw_intraday" => vec![],
        "ststw_rsi2_under5" => vec![p("trend", "Trend SMA", 200.0, 50.0, 300.0, 10.0), p("rsi_n", "RSI period", 2.0, 2.0, 5.0, 1.0),
            p("entry", "Buy when RSI closes below", 5.0, 1.0, 20.0, 1.0), p("exit_ma", "Exit: close above SMA", 5.0, 3.0, 10.0, 1.0)],
        "ststw_cum_rsi_35" | "ststw_cum_rsi_50" => vec![p("trend", "Trend SMA", 200.0, 50.0, 300.0, 10.0),
            p("rsi_n", "RSI period summed (book 2; 3 reproduces the printed results)", 2.0, 2.0, 5.0, 1.0),
            p("days", "Days summed (X)", 2.0, 1.0, 5.0, 1.0), p("sum", "Buy when the sum is below (Y)", if id.ends_with("35") { 35.0 } else { 50.0 }, 10.0, 100.0, 5.0),
            p("exit", "Exit when RSI(2) closes above", 65.0, 50.0, 80.0, 5.0)],
        "ststw_double7" => vec![p("trend", "Trend SMA", 200.0, 50.0, 300.0, 10.0), p("n", "N-day closing low (entry)", 7.0, 5.0, 10.0, 1.0),
            p("x", "N-day closing high (exit)", 7.0, 5.0, 10.0, 1.0)],
        "ststw_vix_stretch" => vec![p("trend", "Trend SMA", 200.0, 50.0, 300.0, 10.0), p("pct", "VIX % above its 10-day SMA", 5.0, 3.0, 15.0, 1.0),
            p("days", "for at least N days", 3.0, 1.0, 6.0, 1.0), p("exit", "Exit when RSI(2) closes above", 65.0, 50.0, 80.0, 5.0),
            p("rsi_max", "[Not from the book] also RSI(2) below this on the trigger day (course filter; 100 = off)", 100.0, 5.0, 100.0, 5.0)],
        "ststw_vix_rsi" => vec![p("trend", "Trend SMA", 200.0, 50.0, 300.0, 10.0), p("vix_rsi", "VIX RSI(2) above", 90.0, 70.0, 98.0, 2.0),
            p("entry", "RSI(2) below", 30.0, 10.0, 40.0, 5.0), p("exit", "Exit when RSI(2) closes above", 65.0, 50.0, 80.0, 5.0)],
        "ststw_sp_short" => vec![p("trend", "Trend SMA", 200.0, 50.0, 300.0, 10.0), p("up", "Up closes in a row, at least", 4.0, 2.0, 7.0, 1.0),
            p("exit_ma", "Cover: close below SMA", 5.0, 3.0, 10.0, 1.0)],
        "ststw_intraday_drops_highs" | "ststw_intraday_drops_lows" => vec![p("pct", "Next-day order % beyond the signal close (0 = buy on the close)", 0.0, 0.0, 10.0, 1.0)],
        "ststw_stops_hurt" => vec![p("stop", "Stop % below the entry (0 = none)", 0.0, 0.0, 50.0, 1.0), p("universe", "1 = p.43 universe filters (US data only)", 0.0, 0.0, 1.0, 1.0)],
        "ststw_exit_study" => vec![p("exit", "Exit: 1 1-day, 2 3-day, 3 5-day hold, 4 7-day high, 5 C>MA5, 6 C>MA10, 7 RSI2>65, 8 >70, 9 >75", 1.0, 1.0, 9.0, 1.0),
            p("universe", "1 = p.43 universe filters (US data only)", 0.0, 0.0, 1.0, 1.0)],
        _ => return None,
    })
}

/// Ch 6 / 8 / 13 stock tests (connors-final). `signal(i)` decides on bar i's close; entry on that close (pct 0) or a
/// next-day order; `exit(i, t)` closes at bar i's close, except the Stops Hurt stop (intraday, from the day after entry).
fn stock_study_run(id: &str, x: &Ctx, from: usize, to: usize, g: &dyn Fn(&str) -> f64) -> Vec<Trade> {
    let c = &x.c;
    let (ma200, ma5, ma10) = (x.sma_n(200), x.sma_n(5), x.sma_n(10));
    let r2 = &x.rsi2;
    let filtered = usd_filter(id) || g("universe") >= 0.5;
    let liquid = |i: usize| -> bool {
        !filtered || (i + 1 >= 100 && c[i] > 5.0 && x.r[i + 1 - 100..=i].iter().map(|r| r.volume.unwrap_or(0.0)).sum::<f64>() / 100.0 >= 250_000.0)
    };
    let high10 = |i: usize| i + 1 >= 10 && c[i + 1 - 10..=i].iter().all(|&v| c[i] >= v);
    let signal = |i: usize| -> bool {
        above(c[i], ma200[i]) && liquid(i) && if id == "ststw_intraday_drops_highs" { high10(i) } else { x.lowest_close(i, 10) }
    };
    let pct = match id { "ststw_intraday_drops_highs" | "ststw_intraday_drops_lows" => g("pct"), "ststw_exit_study" => 5.0, _ => 0.0 };
    let (stop, exit) = (g("stop"), g("exit").round() as i64);
    let hold = |e: usize, i: usize| -> Option<String> {
        let held = i - e;
        match id {
            "ststw_intraday_drops_highs" | "ststw_intraday_drops_lows" => (held >= 5).then(|| "close 5 trading days later".into()),
            "ststw_stops_hurt" => above(c[i], ma10[i]).then(|| "Close > 10-day SMA".into()),
            _ => match exit {
                1 | 2 | 3 => { let n = [1, 3, 5][(exit - 1) as usize]; (held >= n).then(|| format!("{n}-day hold")) }
                4 => (i + 1 >= 7 && c[i + 1 - 7..=i].iter().all(|&v| c[i] >= v)).then(|| "7-day closing high".into()),
                5 => above(c[i], ma5[i]).then(|| "Close > 5-day SMA".into()),
                6 => above(c[i], ma10[i]).then(|| "Close > 10-day SMA".into()),
                k => { let lv = [65.0, 70.0, 75.0][((k - 7).clamp(0, 2)) as usize]; (r2[i] > lv).then(|| format!("RSI(2) {:.1} > {lv}", r2[i])) }
            },
        }
    };
    let mut out = vec![];
    let mut i = from;
    while i <= to {
        if !signal(i) || i == to { i += 1; continue; }
        // entry
        let (e, px, kind) = if pct == 0.0 { (i, c[i], "close") } else {
            let k = i + 1;
            let fill = if id == "ststw_intraday_drops_highs" {
                let lvl = c[i] * (1.0 + pct / 100.0); (x.h[k] >= lvl).then(|| x.o[k].max(lvl))
            } else {
                let lvl = c[i] * (1.0 - pct / 100.0); (x.l[k] <= lvl).then(|| x.o[k].min(lvl))
            };
            match fill { Some(px) => (k, px, if id == "ststw_intraday_drops_highs" { "stop" } else { "limit" }), None => { i += 1; continue; } }
        };
        let mut t = Trade { dir: 1.0, legs: vec![Leg { i: e, px, w: 1.0 }], exit_i: to, exit_px: c[to], reason: "open".into(), entry_kind: kind, signal_i: to, open: true, exits: vec![] };
        // a next-day limit fill can exit at that day's close (time exits count trading days after the entry day)
        let first = if kind == "close" { e + 1 } else { e };
        for k in first..=to {
            if id == "ststw_stops_hurt" && stop > 0.0 && k > e {
                let lvl = t.legs[0].px * (1.0 - stop / 100.0);
                if x.l[k] <= lvl { t.exit_i = k; t.exit_px = x.o[k].min(lvl); t.reason = format!("{stop}% stop"); t.signal_i = k; t.open = false; break; }
            }
            if let Some(why) = hold(e, k) { t.exit_i = k; t.exit_px = c[k]; t.reason = why; t.signal_i = k; t.open = false; break; }
        }
        i = t.exit_i + 1;
        out.push(t);
    }
    out
}

fn trade(dir: f64, i: usize, px: f64, kind: &'static str) -> Trade {
    Trade { dir, legs: vec![Leg { i, px, w: 1.0 }], exit_i: i, exit_px: px, reason: String::new(), entry_kind: kind, signal_i: i, open: false, exits: vec![] }
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    if from > to || to >= x.c.len() {
        return Some(vec![]);
    }
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(0.0, |d| d.def));
    let u = |k: &str| g(k).round().max(1.0) as usize;
    let c = &x.c;
    let close = |dir: f64, entry: &dyn Fn(usize) -> bool, exit: &dyn Fn(usize) -> Option<String>| {
        let e = |i: usize| entry(i).then_some(Entry::Close);
        let xf = |i: usize, _: &Trade| exit(i).map(Exit::Close);
        simulate(x, &Spec { dir, weights: &[1.0], entry: &e, add: &no_add, exit: &xf }, from, to)
    };
    if stock_study(id) {
        return Some(stock_study_run(id, x, from, to, &g));
    }
    Some(match id {
        "ststw_10d_high_buy" | "ststw_10d_high_short" | "ststw_10d_low_buy" => {
            let (n, ma, hl) = (u("n"), x.sma_n(u("ma")), g("hl") >= 0.5);
            let high = |i: usize| if hl { x.highest_high(i, n) } else { i + 1 >= n && c[i + 1 - n..=i].iter().all(|&v| c[i] >= v) };
            let low = |i: usize| if hl { x.lowest_low(i, n) } else { x.lowest_close(i, n) };
            match id {
                "ststw_10d_high_buy" => close(1.0, &|i| high(i), &|i| below(c[i], ma[i]).then(|| "Close < 10-day SMA".into())),
                "ststw_10d_high_short" => close(-1.0, &|i| high(i), &|i| below(c[i], ma[i]).then(|| "Close < 10-day SMA".into())),
                _ => close(1.0, &|i| low(i), &|i| above(c[i], ma[i]).then(|| "Close > 10-day SMA".into())),
            }
        }
        "ststw_overnight" => (from..to)
            .map(|i| Trade { exit_i: i + 1, exit_px: x.o[i + 1], signal_i: i + 1, reason: "next open".into(), ..trade(1.0, i, c[i], "close") })
            .collect(),
        "ststw_intraday" => (from..=to)
            .map(|i| Trade { exit_px: c[i], reason: "same-day close".into(), ..trade(1.0, i, x.o[i], "open") })
            .collect(),
        "ststw_rsi2_under5" => {
            let (ma, rs, xm, lo) = (x.sma_n(u("trend")), x.rsi_n(u("rsi_n").max(2)), x.sma_n(u("exit_ma")), g("entry"));
            close(1.0, &|i| above(c[i], ma[i]) && rs[i] < lo, &|i| above(c[i], xm[i]).then(|| "Close > 5-day SMA".into()))
        }
        "ststw_cum_rsi_35" | "ststw_cum_rsi_50" => {
            let (ma, rs, days, y, ex) = (x.sma_n(u("trend")), x.rsi_n(u("rsi_n").max(2)), u("days"), g("sum"), g("exit"));
            let r2 = &x.rsi2;
            close(1.0, &|i| i + 1 >= days && above(c[i], ma[i]) && rs[i + 1 - days..=i].iter().sum::<f64>() < y,
                  &|i| (r2[i] > ex).then(|| format!("RSI(2) {:.1} > {ex}", r2[i])))
        }
        "ststw_double7" => {
            let (ma, n, xn) = (x.sma_n(u("trend")), u("n"), u("x"));
            close(1.0, &|i| above(c[i], ma[i]) && x.lowest_close(i, n),
                  &|i| (i + 1 >= xn && c[i + 1 - xn..=i].iter().all(|&v| c[i] >= v)).then(|| "7-day closing high".into()))
        }
        "ststw_vix_stretch" => {
            let (ma, st, days, ex, rmax) = (x.sma_n(u("trend")), x.vix.stretched(g("pct")), u("days"), g("exit"), g("rsi_max"));
            let r2 = &x.rsi2;
            close(1.0, &|i| above(c[i], ma[i]) && st[i] >= days && (rmax >= 100.0 || r2[i] < rmax),
                  &|i| (r2[i] > ex).then(|| format!("RSI(2) {:.1} > {ex}", r2[i])))
        }
        "ststw_vix_rsi" => {
            let (ma, vr, lt, ex, r2) = (x.sma_n(u("trend")), g("vix_rsi"), g("entry"), g("exit"), &x.rsi2);
            close(1.0, &|i| above(c[i], ma[i]) && x.vix.rsi2[i] > vr && x.vix.gap_over_close[i] && r2[i] < lt,
                  &|i| (r2[i] > ex).then(|| format!("RSI(2) {:.1} > {ex}", r2[i])))
        }
        "ststw_sp_short" => {
            let (ma, up, xm) = (x.sma_n(u("trend")), u("up"), x.sma_n(u("exit_ma")));
            close(-1.0, &|i| below(c[i], ma[i]) && x.up_closes[i] >= up, &|i| below(c[i], xm[i]).then(|| "Close < 5-day SMA".into()))
        }
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    fn rows(c: &[f64]) -> Vec<Row> {
        c.iter().enumerate().map(|(i, &c)| Row { ticker: "T".into(), date: format!("{:04}-01-01", 1000 + i), open: c - 0.5, high: c + 1.0, low: c - 1.0, close: c, volume: None }).collect()
    }
    /// Double 7's by hand: uptrend (above SMA200), a 7-day closing low, then a 7-day closing high.
    #[test]
    fn double7_by_hand() {
        let mut c: Vec<f64> = (0..210).map(|i| 100.0 + i as f64).collect();
        c.extend([305.0, 304.0, 303.0, 302.0, 301.0, 300.0, 299.0, 330.0]);
        let r = rows(&c);
        let x = Ctx::new(&r, &r, &[]);
        let t = run("ststw_double7", &x, 0, c.len() - 1, &Params::new()).unwrap();
        // 304 at bar 211 is the first close that is the lowest of the last 7 (bars 205..211); 330 is a 7-day closing high
        assert_eq!(t.len(), 1);
        assert_eq!((t[0].legs[0].i, t[0].legs[0].px, t[0].exit_i, t[0].exit_px), (211, 304.0, 217, 330.0));
    }
    #[test]
    fn overnight_and_day_session() {
        let c = [10.0, 11.0, 12.0];
        let r = rows(&c);
        let x = Ctx::new(&r, &r, &[]);
        let t = run("ststw_overnight", &x, 0, 2, &Params::new()).unwrap();
        assert_eq!(t.len(), 2);
        assert_eq!((t[0].legs[0].px, t[0].exit_i, t[0].exit_px), (10.0, 1, 10.5));
        let t = run("ststw_intraday", &x, 0, 2, &Params::new()).unwrap();
        assert_eq!(t.len(), 3);
        assert_eq!((t[2].legs[0].px, t[2].exit_i, t[2].exit_px), (11.5, 2, 12.0));
    }
    #[test]
    fn vix_rsi_reads_the_vix_open_gap() {
        let mut c: Vec<f64> = (0..210).map(|i| 100.0 + i as f64).collect();
        c.extend([300.0, 296.0, 292.0, 330.0]);
        let r = rows(&c);
        let vix = |gap: f64| -> Vec<Row> { r.iter().enumerate().map(|(i, x)| {
            let v = if i < 210 { 15.0 + (i % 2) as f64 } else { 15.0 + 5.0 * (i - 209) as f64 };
            Row { ticker: "CBOE VIX".into(), open: if i >= 210 { v - 5.0 + if i == 212 { gap } else { 0.0 } } else { v }, high: v + 1.0, low: v - 6.0, close: v, ..x.clone() } }).collect() };
        let v = vix(1.0);
        let x = Ctx::new(&r, &r, &v);
        let t = run("ststw_vix_rsi", &x, 0, c.len() - 1, &Params::new()).unwrap();
        assert!(x.vix.gap_over_close[212] && x.vix.rsi2[212] > 90.0 && x.rsi2[212] < 30.0);
        assert_eq!((t[0].legs[0].i, t[0].exit_i), (212, 213));
        let v = vix(-1.0); // open below yesterday's VIX close: no trade that day
        assert!(run("ststw_vix_rsi", &Ctx::new(&r, &r, &v), 0, c.len() - 1, &Params::new()).unwrap().iter().all(|t| t.legs[0].i != 212));
    }
}
