//! Lane K70 (8 Nov 2026, decision #268; authors/titles-1030/K70.tsv): Aseem Singhal, '51 Trading Strategies' (ZebraLearn 2023,
//! ISBN 978-81-963735-7-3), read from the research share's 218 web-reader screenshots ('Trading Books/51- Trading Strategies/
//! Trading Strategies - All Images.pdf'; OCR authors/rnew/ocr, page map authors/rnew/pagemap.txt). Pages below are the reader's
//! printed page numbers (N / 434) with the PDF page in brackets. All 51 strategies are cards: 9 coded, 42 held with the reason.
//! Daily rules (US and India indices, ETFs, stocks; admission below): 1.7 Ichimoku, 3.4 fractals + SMA 50, 3.6 Donchian pullback,
//! 4.2 Supertrend + RSI, 4.4 RSI M & W. Intraday rules on the vault's 1-minute files (intraday_ep lists, block K70): 2.2
//! Supertrend + pivot R1/S1 (5-minute), 2.6 double RSI (5-minute + 1-hour), 5.1 Heikin Ashi + Parabolic SAR + RSI (3-minute),
//! 5.5 first-hour EMA scalp (5-minute). Indicators follow TradingView's built-ins (the book's charts are TradingView's).
//! Independent reference: authors/titles-1030/K70w/k70_ref.py (written from the pages, not from this file).
//! Lane K71 (8 Nov 2026, decision #271, audit-1108/E1.tsv): 4.4 needs the RSI M / W shape, 5.1's candle has no lower / upper
//! wick, 4.2 enters on a stop at the green candle's high; reference authors/titles-1030/K71w/k71_ref.py.
//! Lane K72 (8 Nov 2026, decision #273; authors/titles-1030/K72.tsv): the minute rules' next-bar orders expire after their bar
//! (an unfilled order no longer blocks the next bar's signal); 4.2's initial stop, printed on the step card as 'the swing low'
//! (undefined), placed at the example's level, the signal candle's Supertrend;
//! reference authors/titles-1030/K72w/k72_ref.py.
//! Lane F2 (13 Nov 2026, R4 #293; authors/titles-1030/F2.tsv): five option rules (6.2, 6.3, 6.9, 6.10, 6.11) on real NIFTY weekly
//! 1-minute option prices (src/optw1.rs; the user's StockMock store), NIFTY 50 only; 6.4-6.7 need BANKNIFTY weekly minutes (not
//! extracted), 6.12 prints no exit; reference authors/titles-1030/F2w/f2_ref.py.
//! Lane F4 (13 Nov 2026, R4 #300 (6); authors/titles-1030/F4.tsv): 6.4 Supertrend selling, 6.5 straddle + VWAP, 6.6 momentum
//! buying and 6.7 expiry decay on real BANKNIFTY weekly 1-minute option prices (NIFTYBANK.w1.csv, from the user's AmiBroker
//! database; src/optw1.rs), NIFTYBANK only; reference authors/titles-1030/F4w/f4_ref.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use crate::intraday::{day_trade, Bar1};
use crate::intraday_ep6::{self as ep6, Ev, Seg};

pub const BOOK: &str = "as51ts";
/// Coded daily rules (admitted by ADMISSION below).
pub const DAILY: &[&str] = &["as51_ichimoku", "as51_fractal_sma", "as51_donchian_pb", "as51_st_rsi", "as51_rsi_mw"];
/// Coded minute rules (admitted through intraday_ep::K70 on the four minute tickers).
pub const INTRA: &[&str] = &["as51_st_pivot", "as51_double_rsi", "as51_ha_psar", "as51_ema_scalp"];
pub fn reviewed(id: &str) -> bool { DAILY.contains(&id) }

const SRC: &str = "Aseem Singhal, 51 Trading Strategies (ZebraLearn 2023), web-reader screenshots in the research share";
const NO_TABLE: &str = "The book prints no performance table: worked chart examples only (NIFTY 50, BANKNIFTY and NSE stocks, 2020-2023). No costs in the vault's run. Cash-index shorts are hypothetical (the book shorts through futures).";

macro_rules! def {
    ($id:expr, $name:expr, $dir:expr, $status:expr, $rules:expr, $notes:expr) => {
        StrategyDef { id: $id, name: $name, group: "51 Trading Strategies (Singhal)", session: 0, dir: $dir, status: $status, needs_volume: false,
            needs_vix: false, needs_series: &[], book: BOOK, rules: $rules, notes: $notes, claim: None }
    };
}
macro_rules! held {
    ($id:expr, $name:expr, $rule:expr, $why:expr) => { def!($id, $name, "Long/Short", Status::NotTestable($why), &[$rule], SRC) };
}
macro_rules! data {
    ($id:expr, $name:expr, $rule:expr, $why:expr) => { def!($id, $name, "Long/Short", Status::NeedsData($why), &[$rule], SRC) };
}
/// Lane F2 (R4 #293): the five NIFTY option rules on real weekly 1-minute option prices (src/optw1.rs).
pub const OPT_IDS: &[&str] = &["as51_opt_multi_sell", "as51_opt_oi_buy", "as51_opt_theta", "as51_opt_btst", "as51_opt_3pm", "as51_opt_straddle",
    "as51_opt_st_sell", "as51_opt_vwap", "as51_opt_mom_buy", "as51_opt_expiry"];
/// Lane F4 (R4 #300 (6)): the BANKNIFTY option rules (6.4-6.7); they run on NIFTYBANK only, the others on NIFTY 50 only.
pub const BNF_IDS: &[&str] = &["as51_opt_st_sell", "as51_opt_vwap", "as51_opt_mom_buy", "as51_opt_expiry"];
const OPTNOTE_BN: &str = "The book prints trade lists from its own backtests (another data vendor; 6.4 and 6.6 over 6 years, 6.7 from 2017); the vault runs the rule on its own data; where printed rows can be read they are compared in authors/titles-1030/F4.tsv. No costs in the vault's run.";
const OPTMIN_BN: &str = "A|Real BANKNIFTY weekly options, 1-minute (R4 #300 (6), lane F4): the user's AmiBroker database DATABASE/zerodha_N50_BN_FN__MIDCP_Options_From_01-02-2017_To_08-09-2025 (one file per contract, the same vendor files as the Globaldatafeed folders, extended to 2025) and its NIFTY BANK index minutes; the nearest weekly expiry, 2017-02-01 to 2024-11-13 (NSE ended BANKNIFTY weekly options after 13 Nov 2024); src/optw1.rs. The database repeats a contract's last bar in minutes without a trade (same open, high, low, close and volume): such repeats are dropped, so a minute without a trade has no bar. A price at a clock time is the open of that minute's bar, else the contract's last traded price; a level order fills at its level when the 1-minute path reaches it, or at a minute's open when the price jumps through it; inside a minute the path is open, nearer extreme (ties: high), other extreme, close; a minute whose fills differ between its two extreme orders is ambiguous (#175). Each leg is one option per unit of BANKNIFTY on the options account (options::account): P&L in index points / the spot at entry. No costs (the book gives none; the vault cost setting applies). Days with fewer than 300 index minutes (trading halts, special Saturday and Muhurat sessions) are not traded, and each is noted on the next trade.";
const OPTNOTE: &str = "The book prints trade lists from its own backtests (another data vendor, 2019-2023); the vault runs the rule on its own data; where printed rows can be read they are compared in authors/titles-1030/F2.tsv. No costs in the vault's run.";
const OPTMIN: &str = "A|Real NIFTY weekly options, 1-minute (R4 #293): the user's StockMock store (#Data/stores/NIFTY_full, nearest expiry, 2019-02-11 on; src/optw1.rs). A price at a clock time is the open of that minute's bar (the bar covers the minute; the store's README), else the contract's last traded price; a level order fills at its level when the 1-minute path reaches it, or at a minute's open when the price jumps through it; inside a minute the path is open, nearer extreme (ties: high), other extreme, close; a minute whose fills differ between its two extreme orders is ambiguous (#175). Each leg is one option per unit of NIFTY on the options account (options::account): P&L in index points / the spot at entry. No costs (the book gives none; the vault cost setting applies). Days without option minutes in the store (2026-09-16 to 2026-09-28: the source has no spot minutes) and days with fewer than 300 spot minutes (2026-09-30) are not traded, and each is noted on the next trade. The rejection of the vault's own NIFTY 50 minute file from 2026-08-11 (frozen 15:15-15:19 bars, then a spike; intraday_w2b::NIFTY_BAD_FROM) is not applied: this store's spot is another source, and it shows neither defect on any day of Jul-Oct 2026 (lane F2 check).";
const OPT: &str = "NIFTY weekly 1-minute option prices are now held (lane F2, R4 #293), but this rule manages two legs as one position (a combined or linked stop, or a spread across two expiries), which the vault's option account does not run yet (#263, #182 (2)).";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![
        held!("as51_bb_ema9", "1.1 Bollinger Bands + 9 EMA (30-minute)", "E|Price takes support on the lower band, bullish candle, buy above its high; stop below the touching candle; target the upper band (p.15-29 [PDF 8-16])",
            "Held (decision #268 (2)): 'in an uptrend' is never defined (only band-to-EMA distances are illustrated), so the setup is not mechanical (p.15-29)."),
        held!("as51_willr_sma", "1.2 Williams %R + SMA duo (30-minute swing)", "E|Price above SMA 14, %R(14) from overbought back to -50 and up: buy above the green candle; stop at the swing low (p.29-35 [PDF 17-20])",
            "Held (decision #268 (2)): no exit or target rule (only 'more than 1:2' in an example) and the swing low is undefined (p.29-35)."),
        held!("as51_macd_fib", "1.3 MACD + Fibonacci retracement swings (30-minute)", "E|Pullback to a Fibonacci level, MACD crosses up, green candle: buy above its close; stop the previous swing low (p.35-41 [PDF 20-23])",
            "Held (decision #268): the Fibonacci anchors (swing low and high) are chosen by eye (p.35-41)."),
        held!("as51_triangle_vol", "1.4 Triangle breakout with volume (daily)", "E|Rally, triangle with drying volume, high-volume breakout candle: buy above its close; stop below the lower trendline (p.41-47 [PDF 23-26])",
            "Held (decision #268): the triangle and its trendlines are drawn by eye (p.41-47)."),
        held!("as51_gap_zones", "1.5 Institutional gap demand / supply zones (daily)", "E|Lower lows into a prior demand zone, gap-up green day: buy above its high; stop below the gap (p.51-63 [PDF 27-33])",
            "Held (decision #268): demand and supply zones are marked by eye (p.51-63)."),
        held!("as51_bb_squeeze", "1.6 Bollinger squeeze breakout with Band Width (30-minute)", "E|After a squeeze with BBW flat, buy the break of the upper band with BBW rising; exit a close below the middle band (p.63-71 [PDF 33-37])",
            "Held (decision #268 (1)): the book gives no threshold for 'squeeze / Band Width flat'; a trader-chosen threshold is not a complete rule (#230) (p.63-71)."),
        def!("as51_ichimoku", "1.7 Ichimoku Cloud swing: Tenkan above Kijun and a close above the cloud; stop Span A, target 1:3", "Long/Short", Status::Live,
            &["F|Ichimoku (9, 26, 52, 26): \"Conversion Line ... midpoint of the last 9 periods\"; \"Base Line ... midpoint of the last 26 periods\"; \"Leading Span A ... the average of the Conversion Line and the Base line ... plotted 26 periods ahead\"; \"Leading Span B ... highest high and the lowest low of the last 52 periods ... plotted 26 periods ahead\" (p.73-75 [PDF 38-39])",
              "E|\"Look for buying opportunities when the Conversion Line is above the Base Line and vice versa for selling. Only buy when the price has closed above the cloud and only sell when the price has closed below the cloud\"; \"a 1-hour or a daily trading timeframe\" (p.75); \"After a few candles, the nifty closed above the cloud ... This is where we are going to take a bullish entry\" (p.77)",
              "X|\"keeping the stoploss at the same level as Leading Span A ... you can easily watch out for targets with a 1:3 ratio\" (p.77); short: \"Keep the stoploss as same level as the Leading Span A\" (p.77)",
              "A|Read: daily bars; TradingView's built-in Ichimoku (the book's caption 'Ichimoku (9, 26, 52, 26)'), whose spans are drawn 25 bars ahead (offset = displacement - 1), so the cloud at a bar is Span A / Span B computed 25 bars earlier; entry at the close of the first bar on which Conversion > Base AND the close is above both spans (both conditions were not true on the bar before), short the mirror; the stop is the cloud's Span A at the entry bar, fixed; target = entry + 3 x (entry - stop); the optional trailing is not modelled; one position at a time, signals while in a position are ignored. Stop and target are working levels on later bars; a bar that reaches both takes the stop first (daily-bar convention) [Declared readings]. The book's examples are NIFTY 50 1-hour charts; daily indices, ETFs and stocks, US and India, are a declared application."],
            NO_TABLE),
        held!("as51_sma200_fib", "2.1 200 SMA + Fibonacci (5-minute)", "E|Above a rising 200 SMA, Fibonacci from swing low to high, bullish candle at a level: buy its high; stop the next lower level (p.81-89 [PDF 42-46])",
            "Held (decision #268): the Fibonacci anchors (swing low and high) are chosen by eye (p.81-89)."),
        def!("as51_st_pivot", "2.2 Supertrend (7, 3) + daily pivot R1/S1 (5-minute): buy above the close of a bullish candle above R1 and the Supertrend", "Long/Short", Status::Live,
            &["F|\"Apply Supertrend to the chart. Set the ATR range to 7.\" (chart label 'Supertrend 7 3'); \"Apply Standard Pivot Points with only S1 and R1 points enabled\" (chart 'Pivots Traditional Auto') (p.93-95 [PDF 48-49])",
              "E|\"Wait for the price to break the R1. And let the price stay above the Supertrend. Once both conditions are met, wait for a bullish candle to form. BUY above the closing of the bullish candle.\" Sell: \"break the S1 ... stay below the Supertrend ... wait for a bearish candle ... SELL below the closing of the bearish candle\" (p.95-97 [PDF 49-50])",
              "X|\"STOPLOSS may be placed below the Supertrend. The TARGET is at the trader's discretion or exit when the price closes below Supertrend\"; \"Since we are using Supertrend, it acts like the stoploss and the trade is exited at the closing of the candle below the Supertrend\" (p.95); \"since this was an intraday trade we needed to close the trade at 3:15 PM\" (p.97)",
              "A|Read: session-aligned 5-minute bars from the 1-minute file, one continuous series (TradingView); Supertrend = TradingView ta.supertrend(3, 7) on (H+L)/2 with Wilder's ATR; pivots = Traditional from the previous daily bar: P = (H+L+C)/3, R1 = 2P - L, S1 = 2P - H. Setup at a bar's close: close above R1, Supertrend bullish, close above open; then a buy stop at that close for the next bar only (short: the mirror below S1); exit at the close of a bar on which the Supertrend turns bearish (the close below it); square-off at the close of the bar ending 15 minutes before the session close (15:15 NSE, 15:45 US); no new order from that bar on [Declared readings]. The 'discretionary' target is not modelled. NIFTY 50 examples; SPY, SPX and NIFTYBANK are a declared application.", MIN],
            NO_TABLE),
        data!("as51_vwap_sd", "2.3 VWAP + 2-standard-deviation band reversal (5-minute)", "E|Bearish close below the lower 2-SD VWAP band, next candle bullish: buy above its high; stop its low; targets VWAP and the upper band (p.97-107 [PDF 50-55])",
            "Held (decisions #268 (3), #271(5)): the rule needs real traded volume for the session VWAP, and the vault's 1-min bar files carry no volume data (p.97-107)."),
        data!("as51_rsi_vo", "2.4 RSI + Volume Oscillator (5-minute)", "E|Volume Oscillator(5,10) near -30% with RSI(14) at 30 in an uptrend dip: buy; stop the swing low; target RSI overbought (p.107-115 [PDF 55-59])",
            "Held (decision #268): needs volume data (the vault's 1-min bar files carry none), and 'near -30%', 'in tandem', the trend and the swing low are undefined (p.107-115)."),
        held!("as51_pivot_pullback", "2.5 Pivot-point pullback (5-minute)", "E|Pullback without breaking a pivot = reversal trade, after breaking it = continuation; bullish candle after the pullback; stop its low; target the next pivot (p.115-125 [PDF 59-64])",
            "Held (decision #268): 'pullback' has no definition (p.115-125)."),
        def!("as51_double_rsi", "2.6 Double RSI: 5-minute RSI(14) oversold + bullish candle with the 1-hour RSI(14) above 50; exit on the 70 re-cross", "Long/Short", Status::Live,
            &["F|\"The double RSI method involves two RSI, one with its default settings, and one with an hourly timeframe on a 5-minute chart\" (p.127 [PDF 65]); \"The RSI with hourly chart timeframe gives buy signal when it is above the 50 level and gives sell signal when it is below the 50 level\" (p.129 [PDF 66])",
              "E|\"Look for the second RSI to be above the 50 level. Meanwhile, look for the first RSI to enter the oversold zone and for the price to form a bullish candle. BUY at the high of the green candle.\" Sell: \"second RSI to be below the 50 level ... first RSI to enter the overbought zone and the price to form a bearish candle. SELL at the low of the red candle\" (p.131-133 [PDF 67-68])",
              "X|\"STOPLOSS is below the low of the green candle. TARGET is booked when the first RSI crosses the 70 level from above after entering the overbought zone\"; sell: \"STOPLOSS is above the high of the red candle. TARGET is booked when the first RSI crosses the 30 level from below after entering into the oversold zone\" (p.131-133)",
              "A|Read: session-aligned 5-minute bars, one continuous series; RSI(14) is TradingView's ta.rsi; the hourly RSI is RSI(14) of session-aligned 60-minute bars, its value at a 5-minute bar being that of the last hourly bar completed by then (TradingView request.security without lookahead). Setup at a bar's close: hourly RSI > 50, 5-minute RSI below 30 on that bar or the bar before ('enters the oversold zone' and the bullish candle may be the turn), close above open; a buy stop at its high for the next bar only, protective stop at its low; exit at the close of a bar whose 5-minute RSI falls back to 70 or below after a bar above 70; the stop and the entry work on the real 1-minute path; flat at the session close (an intraday strategy) [Declared readings]. Short = mirror. NIFTY 50 examples; SPY, SPX and NIFTYBANK are a declared application.", MIN],
            NO_TABLE),
        held!("as51_cpr_trend", "2.7 CPR with trend following (5-minute)", "E|Narrow CPR: buy the break of a level, stop below the candle, target the next pivot; wide CPR: buy support, sell resistance (p.133-147 [PDF 68-75])",
            "Held (decision #268): no threshold separates a narrow from a wide CPR, and 'takes support' is undefined (p.133-147)."),
        held!("as51_dow_swings", "3.1 Dow theory swing breaks", "E|Higher highs and lows: buy at the top of the candle that breaks the previous high; stop below the swing low (p.149-157 [PDF 76-80])",
            "Held (decision #268): swing highs and lows are not defined (p.149-157)."),
        held!("as51_smc_ob", "3.2 Smart Money Concept order blocks (1-minute)", "E|Mark the order block / fair-value zone after an aggressive move, buy above a reversal candle on the retrace (p.157-173 [PDF 80-88])",
            "Held (decision #268): zones are marked by eye (p.157-173)."),
        held!("as51_elliott", "3.3 Elliott Wave entries", "E|Buy at the end of wave 2 or 4 on a bullish candle's high; stop below the swing low; 1:2 (p.173-181 [PDF 88-92])",
            "Held (decision #268): wave counts are made by eye (p.173-181)."),
        def!("as51_fractal_sma", "3.4 Williams fractals (4/4) + SMA 50: a green candle closing through the last fractal high above the SMA; stop the last fractal low, target 1:2", "Long/Short", Status::Live,
            &["F|\"I prefer the 4/4 setting, meaning 4 candles on each side of the fractal\"; a fractal high: \"4 candles are preceding it and 4 are following it. All of which have failed to surpass the price level marked by the fractal\"; a fractal low the mirror (p.185 [PDF 94]); \"apply a 50-Day Simple Moving Average\" (p.187 [PDF 95])",
              "E|\"When the price is above the 50-period SMA, we look for buy trades, and when it's below, we focus on sell trades ... Buy when the previous fractal high is broken with a green candle for the entry\" (p.189 [PDF 96]); \"BUY when the price breaks the previous fractal high\"; sell: \"Wait for the price to fall below the 50-day Simple Moving Average ... SELL when the price breaks the previous fractal low\" (p.191-193 [PDF 97-98])",
              "X|\"you can place the stoploss either at the fractal level or, if the fractal is too distant, at the nearest significant swing high or low\" (p.189); \"The ideal Risk to Reward ratio is 1:2\" (p.191-193)",
              "A|Read: daily bars; a fractal bar's high is >= the highs of the 4 bars on each side ('failed to surpass'; low mirror), known once the 4th later bar has closed. Long: at a close above the SMA(50), above the open (green) and above the last confirmed fractal high, entered at that close; each fractal level is broken once (its first close beyond it). The step card's 'Look for an uptrend and points where the bear run has exhausted' (p.191; sell card p.193 the mirror) is read as the chapter's own direction filter, the SMA (p.189: 'The SMA acts as a directional filter'), and the 'bear run has exhausted' as the fractal reversal itself, the close through the last fractal high (decision #271(4)). Stop = the last confirmed fractal low (the 'fractal level'; the alternative swing level is a judgement), target = entry + 2 x (entry - stop); no trade when the stop is not below the entry. Short = mirror. A bar that reaches both stop and target takes the stop first (daily-bar convention) [Declared readings]. NIFTY 50 1-hour / 5-minute examples; daily indices, ETFs and stocks, US and India, are a declared application."],
            NO_TABLE),
        held!("as51_renko", "3.5 Renko (ATR 14) + RSI + Stochastic RSI (15-minute)", "E|Sell at a marked long-term resistance on a red brick with RSI and Stoch RSI overbought; 1:3 (p.195-201 [PDF 99-102])",
            "Held (decision #268): the support and resistance levels are marked by eye (p.195-201)."),
        def!("as51_donchian_pb", "3.6 Donchian (20) pullback: touch the upper band, pull back 3-4 candles, break it again; stop the middle band, trail, target 1:2", "Long/Short", Status::Live,
            &["F|\"The upper band marks the highest price of the security over N periods ... The lower band marks the lowest price ... The middle band takes the average of both\" (p.205 [PDF 104]); \"Apply Donchian Channel with default settings\" (TradingView: 20)",
              "E|\"I observed that a pullback of 3 to 4 candles before a sharp move for the breakout is important to confirm the trade\" (p.205); \"Let the price touch the upper band and take a pullback. The pullback should be followed by a rise in the price and a breaking of the upper band. Once this pattern is spotted, the BUY trade is triggered\" (p.209 [PDF 106]); sell the mirror at the lower band (p.211 [PDF 107])",
              "X|\"Place STOPLOSS below the middle band: trail the SL to the first candle's low when the market makes two bullish candles in a row. TARGET is typically 1:2 Risk to Reward ratio\" (p.209); sell: \"STOPLOSS above the middle band: trail the SL to the first candle's high when the market makes two bullish candles in a row\" (p.211)",
              "A|Read: daily bars; TradingView's Donchian (20): bands over the last 20 bars including the current one. A touch = a bar whose high is the 20-bar high (low mirror); the break = a later bar whose high exceeds the previous bar's upper band, with 3 or 4 bars between the last touch and it ('a pullback of 3 to 4 candles'; option pb_max widens it); entry at the break bar's close; stop = the middle band at entry, fixed, then raised to the low of the first of two consecutive bullish bars (close above open) from the entry bar on; target = entry + 2 x (entry - initial stop); no trade when the stop is not below the entry. Short = mirror; the sell card's 'two bullish candles' is read as bearish, as its example says ('formed two bearish candles in a row', p.211). A bar that reaches both stop and target takes the stop first (daily-bar convention) [Declared readings]. NIFTY 50 intraday examples; daily indices, ETFs and stocks, US and India, are a declared application."],
            NO_TABLE),
        held!("as51_gann_linreg", "3.7 Gann fan + linear regression", "E|Draw a regression line and Gann fan on a chosen leg; buy support at a fan line in an uptrend (p.211-217 [PDF 107-110])",
            "Held (decision #268): the price leg is chosen by eye (p.211-217)."),
        held!("as51_monthly_pivots", "4.1 Moving with macro trends: pivot support + confirming candle (daily)", "E|\"Look for an uptrend. Wait for the price to take support on P, S1 or S2. Wait for a bullish candle to form. BUY above the high of the confirming candle\"; stop below the support area; target the next pivot (p.227-229 [PDF 115-116])",
            "Held (lane K70, 8 Nov 2026): the printed step card includes 'Look for an uptrend' (p.227 [PDF 115], read on the page image; the sell card 'Look for a downtrend', p.229) and the chapter never defines the trend; decision #268 (2) holds 1.1 for the same undefined 'uptrend'. 'Takes support' and 'below the support area' are also undefined."),
        def!("as51_st_rsi", "4.2 Supertrend (10, 3) + RSI (14) crossing 60 / 40 (daily): buy stop at the green candle's high; exit on a close through the Supertrend", "Long/Short", Status::Live,
            &["F|Chart labels 'Supertrend 10 3' and 'RSI 14 close' (p.231-233 [PDF 117-118]); \"When the RSI is above the 60 level indicates the market is bullish ... RSI below 40 level indicates that the market is bearish\" (p.233)",
              "E|\"Look for stocks where the RSI is moving towards the 60 level and the Supertrend is turning bullish. If the price stays above Supertrend and the RSI crosses the 60 level, the BUY entry is triggered.\" Sell: \"If the price stays below Supertrend and the RSI crosses the 40 level, the SELL entry is triggered\" (p.235-237 [PDF 119-120])",
              "X|\"Place STOPLOSS at the swing low, keep trailing it at the Supertrend level. TARGET is on a trailing basis - exit when the price closes below Supertrend\"; short: \"exit when the price closes above the Supertrend\" (p.235-237); example: \"the initial stoploss was at the Supertrend\" (p.235)",
              "A|Read: daily bars; Supertrend = TradingView ta.supertrend(3, 10) on (H+L)/2 with Wilder's ATR; RSI(14) Wilder. Setup: a bar on which the Supertrend is bullish and the RSI crosses above 60 (above 60 now, at or below 60 on the bar before). Entry (decision #271(3)): the steps give no entry order and the only example enters 'at the high of the green candle' (p.235), so the setup bar must be green (close above open) and a buy stop at its high works on the next bar only (filled at the high, or at the open when the bar opens above it), then is cancelled; option stop_entry = 0: entry at the setup bar's close with no candle colour required (the K70 reading). Initial stop (decision #273(2)): the step card prints it ('Place STOPLOSS at the swing low', p.235) but never defines the swing low, so its level is the example's: 'the initial stoploss was at the Supertrend' (p.235), the Supertrend line at the signal candle, a fixed working stop from the entry fill on (filled at the stop, or at the open when a later bar opens beyond it); a bar that fills the stop entry inside its range and also reaches the stop is read as stopped and counted (#240); option init_stop = 0: no initial stop (the K71 reading). Exit at the close of the bar on which the Supertrend turns bearish (the close below it), which is the trailed stop at the Supertrend ('We kept trailing it and exited when the price closed below the Supertrend', p.235). Short = mirror: RSI crossing below 40 on a red bar, a sell stop at its low, the initial stop at the Supertrend above. One position at a time [Declared readings]. NSE stock and index examples; daily indices, ETFs and stocks, US and India, are a declared application (the book shorts through F&O)."],
            NO_TABLE),
        held!("as51_sector_rs", "4.3 Sector relative strength vs NIFTY 50", "E|When a sector index's relative-strength line crosses above NIFTY 50, buy the sector's highest-weight stock (p.237-245 [PDF 120-124])",
            "Held (decision #268): no exit or stop rule; it also needs NSE sector index history and constituent weights (p.237-245)."),
        def!("as51_rsi_mw", "4.4 RSI (14) M & W: buy the W's second re-entry above 30, sell the M's second re-entry below 70; exit at the opposite level", "Long/Short", Status::Live,
            &["F|\"I am using the 14-period RSI and looking for RSI to give overbought (above 70) or oversold levels (below 30)\" (p.247 [PDF 125])",
              "E|\"I am going to sell the stock when the RSI goes above the 70 levels and comes back (below the 70) while forming an M type structure. Conversely, I am going to buy the stock when the RSI goes below the 30 levels and bounce back (above the 30) while forming a W type structure\" (p.247-249 [PDF 125-126]); \"I will buy the stock when the RSI makes the W like structure and re-enters above the 30 level\" (p.251 [PDF 127])",
              "X|\"The stoploss will be close to the nearest price swing near the entry point. I am going to book profits when the RSI reaches the opposite overbought/oversold levels. 30 in the case of a bearish trade and 70 in the case of a bullish trade\" (p.249)",
              "A|Read (decision #271(1), the shape the p.249 figure draws: two RSI peaks above 70 with the SELL at the second fall): daily bars; RSI(14) Wilder. Short at the close of the bar on which the RSI crosses back below 70 (at or above 70 on the bar before) for the second time: the first re-cross below 70 arms the M, the second fires it and the count starts again; no bar limit between the two peaks (the book sets none) and no other reset. Long = the mirror W around 30 (dip below 30, back above, below again, back above: buy at that close). The count runs on the RSI whether or not a position is open; a signal while in a position is ignored. Option mw = 0: every re-cross is a signal (the K70 reading). Long stop = the lowest low from the first bar of the second RSI dip through the entry bar (the 'nearest price swing'), a working stop on later bars (filled at the stop, or the open when it gaps through); exit at the close of the first bar with RSI at or above 70. Short stop: the highest high of the second overbought run through the entry bar; short exit at the close of the first bar with RSI at or below 30. One position at a time [Declared readings]. NSE stock examples; daily indices, ETFs and stocks, US and India, are a declared application."],
            NO_TABLE),
        def!("as51_ha_psar", "5.1 Heikin Ashi + Parabolic SAR + RSI 50 (3-minute scalp): buy above the high of a bullish candle with no lower wick; stop the SAR, target 1:2", "Long/Short", Status::Live,
            &["F|\"SAR 0.02 0.02 0.2\", \"RSI 14 close\", chart 'NIFTY BANK · 3 · NSE · Heikin Ashi' (p.255-259 [PDF 129-131]); \"A Heiken Ashi candlestick with no lower body is a bullish candle\" (p.255)",
              "E|\"Wait for the candles to turn bullish and Parabolic SAR below the candlestick. With the above two confluences, wait for the RSI to move above the 50 level. Once all these confirmations hold true, the buy signal is triggered. BUY above the high of the bullish candle\"; sell the mirror (p.259-261 [PDF 131-132])",
              "X|\"Place STOPLOSS at the Parabolic SAR. TARGET is 1:2. If trading in multiple quantities, close half the position at 1:1\" (p.259); \"the idea was to exit from the trade when the price met the Parabolic SAR\" (p.259)",
              "A|Read: session-aligned 3-minute bars, one continuous series. The book's chart is a Heikin Ashi chart, on which TradingView computes the RSI and the SAR from the Heikin Ashi candles and shows the Heikin Ashi high, so they are used here: HA close = (O+H+L+C)/4, HA open = (previous HA open + previous HA close)/2, HA high/low include them; SAR = TradingView ta.sar(0.02, 0.02, 0.2). Setup at a bar's close: HA close above HA open and no lower wick, the book's bullish candle ('A Heiken Ashi candlestick with no lower body is a bullish candle', p.255; decision #271(2)): HA low = HA open, tested within 1e-9 x the HA open because the HA open is a running average held in binary floating point (the independent reference tests it in exact 50-digit decimals; a bearish candle has no upper wick, HA high = HA open); SAR below the candle, RSI crossing above 50 (above now, at or below on the bar before); a buy stop at the HA high for the next bar only, placed when the SAR level for that bar is below it; the stop is the SAR level of each bar (trailing; when the SAR has flipped above, the next bar's open), the target = entry + 2 x (entry - the SAR at the entry bar); orders fill on the real 1-minute path; one quantity (the half-off at 1:1 applies to several lots only); flat at the session close [Declared readings]. Short = mirror. BANKNIFTY examples; SPY, SPX and NIFTY 50 are a declared application.", MIN],
            NO_TABLE),
        held!("as51_rsi_div_bb", "5.2 RSI divergence + Bollinger Bands scalp (1-minute)", "E|Bullish RSI divergence at the lower band, buy at the green candle; stop below it; target the upper band (p.261-271 [PDF 132-137])",
            "Held (decisions #268, #271(5)): divergence itself is defined ('Price is making higher highs and the oscillator is failing to make higher highs ... and vice versa', p.265-266), but which swing highs or lows it compares is not ('form lower lows near the lower Bollinger Band', p.269), so the signal is not mechanical (p.261-271)."),
        data!("as51_rsi_vwap", "5.3 RSI + VWAP scalp (1-minute)", "E|Overbought and rejected at VWAP: sell; stop above VWAP; target the lower VWAP band (p.271-279 [PDF 137-141])",
            "Held (decision #268): 'rejection' is undefined and the session VWAP needs volume data, which the vault's 1-min bar files do not have (p.271-279)."),
        held!("as51_1m_consol", "5.4 1-minute consolidation breakout scalp", "E|After a trend, 4-5 small candles form a range: buy the break of its high; stop beyond the breakout candle; 1:3; close after 10 minutes (p.279-287 [PDF 141-145])",
            "Held (decision #268): the trend and the range are defined by eye (p.279-287)."),
        def!("as51_ema_scalp", "5.5 First-hour EMA scalp (5-minute): fade the day's 2nd candle when it closes beyond the EMA without touching it; 1:3", "Long/Short", Status::Live,
            &["F|\"STEP 1: Apply a 5 or 7 period EMA (Both will work) on a 5-minute timeframe\" (p.287 [PDF 145])",
              "E|\"STEP 2: In case of a bearish trade, I am looking for the 2nd candle to close above the moving average ... STEP 3: I will sell the stock at the low of the 2nd candle, which has also closed above the EMA. Remember, the second candle should not touch the EMA in any way\"; \"I will not execute any trades after the first hour\" (p.287)",
              "X|\"I will keep the stoploss on the high of this candle. STEP 5: I will keep my R:R ratio to be at least at 1:3 or 1:4\" (p.287); \"Traders using this strategy should always go for a 1:3 risk to reward ratio\" (p.291 [PDF 147])",
              "A|Read: session-aligned 5-minute bars, one continuous series; EMA(5) TradingView (option 7). Bearish: the day's 2nd bar closes above the EMA with its low above the EMA; a sell stop at its low working from the 3rd bar until the end of the first hour (bars starting before open + 60 minutes), then cancelled; protective stop at its high; target = entry - 3 x (stop - entry). Bullish = mirror (2nd bar closes below the EMA with its high below it; buy stop at its high). STEP 4's discretionary moving of the entry to a later candle is not modelled; orders fill on the real 1-minute path; flat at the session close; one setup a day [Declared readings]. NIFTY 50 examples; SPY, SPX and NIFTYBANK are a declared application.", MIN],
            NO_TABLE),
        held!("as51_martingale", "5.6 Martingale 5 & 14 EMA mean reversion (1-minute)", "E|After a down move, buy the high of a bullish reversal candle; stop its low; target the 14 EMA; double the size after a loss (p.291-299 [PDF 147-151])",
            "Held (decision #268): the 'reversal sign' and the 'move' are undefined, and doubling after losses is a sizing scheme the vault does not run (p.291-299)."),
        held!("as51_opt_calendar", "6.1 Weekly hedged calendar spread", "E|Sell current-week CE/PE near 2x India VIX, buy the same strikes next week; adjustments; exit on expiry day (p.303-315 [PDF 153-159])", "Held (lane F2, R4 #294 (4) re-read, 13 Nov 2026): the data exist in the user's StockMock store (NIFTY weekly 1-minute options of both weekly expiries), but the rule is not complete: no entry time is printed (the example uses the 24 May 2023 close, p.305); the hedge legs are bought 'a bit expensive to the sold options' (p.309-311; the example pays 34.05 and 28.75 against 28.6 and 27.1); the adjustments start when the market 'quickly moves to' or 'gets near' a sold strike (p.313, undefined); and the stop closes all legs at 75% of the strategy's maximum loss (p.315), a value read from the broker's modelled payoff chart, not from prices."),
        def!("as51_opt_multi_sell", "6.2 Multi-time option selling: sell the NIFTY weekly OTM call and put nearest Rs 40 at 9:16, 10:16, 11:16 and 12:16; stop 100% of premium, halved at 50% decay; out 3:15 pm", "Long/Short", Status::Live,
            &["E|\"This option selling strategy involves selling out of money Nifty weekly calls and puts that are priced anywhere around Rs. 40\" (p.317 [PDF 160]); \"I have sold both the call and put option at 4 different times of the day - 9:16 a.m.; 10:16 a.m.; 11:16 a.m.; and 12:16 p.m.\" (p.318); \"split one lot of calls and put options across all the 4-time frames each\" (p.321 [PDF 162])",
              "X|\"Our initial stoploss will be set at 100% of the premium received. Meaning, if we sell a call option for Rs. 42, our initial stoploss would be placed at Rs. 84\"; \"Once the trade starts turning in our favor and the option premium declines by 50% from the entry price and we will reduce the stoploss by 50%. For example, if the call option's premium drops to Rs. 21, we will trail our stoploss from Rs. 84 to Rs. 42\"; \"all the trades will be exited by 3:15 p.m. on the day of entry\" (p.317 [PDF 160])",
              "A|Read: the nearest weekly expiry (on expiry day the contract expiring that day, as the book's 8 Jun 2023 rows); at each of the four times, the call and the put whose price at that minute is nearest Rs 40 (tie: lower strike), sold at that minute's open. The text says 'out of money', but the book's own 10:16 trade list (p.318, 2-21 Jun 2023) sells the strike nearest Rs 40 even when it is at or slightly in the money (expiry days 7, 8 and 15 Jun): on the vault's data that reading picks the book's strike in 25 of 25 rows, the out-of-the-money reading in 21, so the default follows the printed trades (as #264 (1)); option otm = 1: out-of-the-money strikes only (call above, put below the spot); eight independent legs a day (the multi-time version, p.321); stop at 2 x entry, moved to the entry once the price reaches half the entry (option sl: the stop in % of premium; the moved stop is half of it); out at the open of 15:15 [Declared readings].", OPTMIN],
            OPTNOTE),
        def!("as51_opt_oi_buy", "6.3 Option buying from open interest: buy the highest-OI NIFTY weekly call or put when it rises 10% over its 9:20 price; stop 20%, target 1:2, out after 30 minutes", "Long/Short", Status::Live,
            &["E|\"We will analyze the Nifty 50 option chain at market open, somewhere around 9:20 a.m. Next, we have to look at the put and call option strikes with the highest open interest\" (p.325 [PDF 164]); \"we will look for a 10% increase in any of the above premiums. So, if the 19,300 put option rises to Rs.50 or the 19,500 call option rises to Rs.16.50, we will buy that option contract\" (p.326)",
              "X|\"Our stoploss will be at the 20% of the entry premium. For example, for a 19,300 put option of Rs.50, the stoploss will be Rs.10. We will cover positions in profit when the trade has given us a risk to reward ratio of 1:2. Meaning, if we are risking Rs. 10, the aim will be to earn Rs.20\"; \"the holding period will be for 30 minutes only ... there are 3 defined exits in this strategy - Stoploss, target or time-based exit in 30 minutes\" (p.326); \"an intraday strategy which will be executed and exited in the early half of the market itself\" (p.321 [PDF 162])",
              "A|Read: the nearest weekly expiry; open interest = the last printed OI of each contract at or before the 09:19 minute (the chain as seen at 9:20); the call and the put with the highest OI; their reference prices = the price at 9:20; a buy stop at 1.1 x that price working from 9:20 to 12:21 (the session's first half ends 12:22:30; option until), each contract at most once a day; stop at 0.8 x entry, target at 1.4 x entry (twice the Rs 10 risk on Rs 50); out at the open of the first traded minute 30 minutes after the fill [Declared readings].", OPTMIN],
            OPTNOTE),
        def!("as51_opt_st_sell", "6.4 Supertrend option selling: on the 5-minute BANKNIFTY chart sell the weekly ATM put while the Supertrend (7, 3) is green and the ATM call while it is red, switching at each colour change; no new trade after 3:15 pm", "Long/Short", Status::Live,
            &["E|\"This intraday strategy applies the Supertrend indicator on a 5-minute Bank Nifty chart for selling the ATM call and puts options of Bank Nifty based on the directional signals generated by the Supertrend indicator\"; \"at the 9:20 a.m. the price is above the Supertrend indicator and it is also green ... At this point, we will sell an at the money put option of the weekly expiry\" (p.333 [PDF 168]); the chart: NIFTY BANK, 5-minute, \"Supertrend 7 3\" (p.331 [PDF 167])",
              "X|\"We will cover our the option whenever the Supertrend changes its color into red. This provides with another trading opportunity and we can sell an at the money call\"; \"we will continue to sell ATM calls and puts whenever the Supertrend indicator changes its colors. This systematic alternating of calls and puts will continue till 3:15 p.m. We will stop trading after 3:15 p.m.\"; \"The dynamic bands of the Supertrend indicator acts as the exit trigger instead of rigid stoplosses\" (p.333); the book's trade table enters at 9:15 bars and exits each day's last trade at the 15:25 bar (p.334 [PDF 168])",
              "A|Read: Supertrend (ATR 7, factor 3; TradingView's built-in, as as51_st_pivot) on session 5-minute bars of the NIFTY BANK index minutes (09:15-09:19 ... 15:25-15:29), one continuous series across days as on the chart (a run starts it 5 trading days before its window); a bar's colour acts at its close, the open of the next minute (the 9:15 bar at 9:20, the book's example); each day the first evaluated bar opens a position in its colour (green: sell the ATM put; red: the ATM call); every colour change buys it back and sells the other type; ATM = the strike listed as call and put nearest the spot at that minute (tie: lower); the nearest weekly expiry (on expiry day the contract expiring that day); no new position on a bar closing after 15:15; the day's last position leaves at its last traded price of the day (the book's table exits at the 15:25 bar) [Declared readings]. The book's table and text disagree on the contract: its 16-22 Dec 2022 rows carry the prices of the 29 Dec 2022 monthly contract (5 rows checked in the source files: within Rs 4), not of the nearest weekly (22 Dec), while the text says 'the weekly expiry'; the vault runs the text's nearest weekly contract (its store holds no other expiry). On 23-29 Dec 2022, where the two are the same contract, the vault's entries match the table within Rs 5.40 and its exits within Rs 5.55, and 21 of the 22 legible rows have a vault leg at the same minute (authors/titles-1030/F4w/book.out).", OPTMIN_BN],
            OPTNOTE_BN),
        def!("as51_opt_vwap", "6.5 Option + VWAP: sell the BANKNIFTY weekly ATM straddle (strike fixed at 9:15) when its 5-minute premium is 3% below the straddle's VWAP; buy it back when the premium touches the VWAP; at most 3 trades a day, none after 3 pm", "Short", Status::Live,
            &["E|\"we are going to sell an At-The-Money straddle of the Bank Nifty at the open and use VWAP as a tool to manage our risks. We are going to use a 5-minute chart and ... the combined straddle premium is plotted on the 5-minute chart\"; \"the entry signal is only triggered when the premium value declines 3% below the VwAP line. This allows us to set a risk buffer and the VWAP acts as the stoploss level\" (p.339 [PDF 171]); the example chart: BANKNIFTY 13JUL23 44900 call + 44900 put, 5min, with its VWAP (p.339)",
              "X|\"we will square off our trade as soon as the premium value touches the VwAP line. We can only take a maximum number of 3 trades per day. We will not take a trade after 3 p.m.\" (p.341 [PDF 172])",
              "A|Adapted: 2 unstated details (#297). Unstated in the book: (1) the straddle's VWAP volume: the book's VWAP is cumulative price x volume over cumulative volume (p.339), but a two-leg straddle has no volume of its own; vault reading: each 5-minute bar's straddle price = the call's close + the put's close (their last traded prices at the bar's end), its volume = the two legs' traded volume added, accumulated from 9:15; (2) the exit of a position still open late in the day: none printed; vault reading: out at the open of 15:15, the book's intraday exit time in 6.2, 6.6, 6.8 and 6.9 (option eod). Read: BANKNIFTY (the chapter's straddle); the nearest weekly expiry; the ATM strike at 9:15 (listed as call and put nearest the spot; tie: lower), kept all day (the book's chart plots one strike); signals at 5-minute closes, acting at the open of the next minute; sell when the straddle's close is at or below 0.97 x VWAP, buy back when it is at or above the VWAP; at most 3 entries a day; no entry on a bar closing after 15:00 [Declared readings].", OPTMIN_BN],
            OPTNOTE_BN),
        def!("as51_opt_mom_buy", "6.6 Momentum option buying: buy the BANKNIFTY weekly ATM call or put when it rises 15% over its 9:30 price; stop 25%, target 100%; out 3:15 pm", "Long/Short", Status::Live,
            &["E|\"Traders would typically execute this strategy by purchasing At-the-Money call and put options based on the spot prices at 9:30 in the morning\" (p.347 [PDF 175]); \"if the call price at 9:30 is 100, traders would wait for the price to increase by 15% (reaching 115) before initiating the trade. Similarly, for the put option priced at 120, a 15% increase (reaching 138) would serve as the trigger for entry\" (p.347)",
              "X|\"we maintain the 25% stoploss and a 100% target, both calculated from the entry price\" (p.349 [PDF 176]); \"set stoploss and target levels for each position and wait for the trade to either hit the stoploss, reach the target, or exit at 3:15 p.m.\" (p.347); the example, BANKNIFTY 24 Apr 2017: only the call reached its trigger (p.349)",
              "A|Adapted: 1 unstated detail (#297). Unstated in the book: how long the 15% trigger works; vault reading: until the 3:15 pm exit (the order works 9:30 to 15:14; option until). Read: BANKNIFTY (the chapter's example and results); the nearest weekly expiry; ATM at 9:30 (the strike listed as call and put nearest the spot; tie: lower); each leg's reference = its price at 9:30; a buy stop at 1.15 x that price, each contract at most once a day; stop at 0.75 x entry, target at 2 x entry; out at the open of 15:15 [Declared readings].", OPTMIN_BN],
            OPTNOTE_BN),
        def!("as51_opt_expiry", "6.7 Expiry decay: two trading days before the BANKNIFTY weekly expiry, at 10:00 sell the call 100 points above and the put 100 points below the rounded spot; stop 90% of premium per leg; out 9:55 am on expiry day", "Short", Status::Live,
            &["E|\"This is typically done around 10:00 a.m. when the initial volatility has settled. We choose to implement this strategy 2 days prior to the expiry date\" (p.351 [PDF 177]); \"Once we have the Bank Nifty spot value, we round it off and select one Out-of-the-Money (OTM) call option and one OTM put option to sell. For instance, if the spot value is 41,181, we round it to 41,200. Therefore, we would look for a Bank Nifty 41,300 call (100 points OTM) and a Bank Nifty 41,100 put (100 points OTM)\"; \"an initial stoploss is set at 90% of the premium for each leg\" (p.351)",
              "X|\"if the premium is 100, the initial stoploss would be 90 (Sell price- 100, Stoploss- 190)\"; \"it is recommended to exit the strategy at 9:55 a.m. on the expiry day instead of waiting for the expiry time\" (p.353 [PDF 178])",
              "A|Adapted: 1 unstated detail (#297). Unstated in the book: the rounding of a spot exactly halfway between hundreds; vault reading: up. Read: '2 days prior' = two trading days before the expiry day (the book's table enters on Mondays when the expiry is on a Wednesday or follows a holiday, p.353); trading days = the store's days with a full session; the spot at 10:00 = the open of the 10:00 minute, rounded to the nearest 100; each leg sold at the open of its first traded minute from 10:00; stop at 1.9 x entry per leg, working through the two nights (a gap above it fills at the open); out at the open of 9:55 on the expiry day; the advice to skip weeks of very high VIX (p.353) is not modelled [Declared readings].", OPTMIN_BN],
            OPTNOTE_BN),
        held!("as51_opt_comb_sl", "6.8 Combined stoploss straddle", "E|Short straddle / strangle with a combined premium stop (p.355-359 [PDF 179-181])", "Held (lane F2, R4 #294 (4) re-read, 13 Nov 2026): the data are held (NIFTY weekly 1-minute options), but the strikes are not defined: the chapter sells 'out-of-the-money (OTM) call and put options' at 9:30 with a combined stop of 40 points and an exit at 3:15 (p.355-356), and never says how far out of the money; its only example sells a 19800 call and a 19800 put of 28 Sep 2023 (p.355), the same strike for both legs. Choosing the distance would be our value (#36)."),
        def!("as51_opt_theta", "6.9 Theta decay: sell the NIFTY weekly ATM call and put at 9:30, stop 25% each, re-sell the new ATM leg after a stop (at most 3 trades a day); out 3:15 pm", "Long/Short", Status::Live,
            &["E|\"The strategy typically begins at 9:30 a.m. ... Traders identify the current spot price of a particular index or stock, such as the Nifty, and select the ATM call and put options accordingly. One lot of each option is sold to initiate the strategy\" (p.359 [PDF 181]); \"if either the call or put option hits the predetermined stoploss level, the trader exits that particular leg of the trade. Subsequently, at 10:34 a.m., the trader reenters the trade by selling the new ATM call or put option, placing another stop-loss order at 25% above the entry price\" (p.360)",
              "X|\"a stoploss order is placed at 25% above the entry price for both the call and put options\" (p.359); \"Throughout the day, a maximum of 3 trades can be executed, and all positions are closed by 3:15 p.m. since it is an intraday strategy\" (p.360)",
              "A|Read: NIFTY (the text's example; the printed table, p.361, is a BANKNIFTY run, which the vault cannot price: BANKNIFTY weekly minutes exist only in the AmiBroker sets); the nearest weekly expiry; ATM = the strike listed as call and put nearest the spot at that minute (tie: lower); both legs sold at the open of 9:30; stop 1.25 x entry; a stopped leg is re-sold at the next minute's open as the ATM contract of the same type at that minute; 'a maximum of 3 trades' = the opening straddle and at most 2 re-entries (the printed table shows up to 4 legs a day, p.361); stops in the same minute: the call first; out at the open of 15:15. Option prem > 0: the contracts nearest that premium (the book's 125 variant, p.361) [Declared readings].", OPTMIN],
            OPTNOTE),
        def!("as51_opt_btst", "6.10 BTST option buying: buy the NIFTY weekly call two strikes in the money when it rises 20% over its 11:00 price; stop 30%; out next day 9:30", "Long", Status::Live,
            &["E|\"The BTST strategy involves buying an in-the-money call option, specifically selecting a call option two strikes in the money, based on the at-the-money price at 11:00 a.m.\"; \"If the call option exhibits a 20% upward move from the entry point ... the trader enters a buying position\" (p.364 [PDF 183]); \"suppose the at-the-money call option is initially priced at 100, and it reaches 120 after 11:00 a.m. ... the trader buys the call\" (p.365 [PDF 184]); \"historical data ... specifically focusing on the Nifty index ... excluding exceptional cases such as Thursdays without trades\" (p.365)",
              "X|\"SL - 30% below the entry price\" (p.364); \"the stoploss level would be 84 (30% of 120 is 36) ... The exit can be either on the same day if the stoploss is triggered or the next morning at 9:30 a.m. to avoid the impact of Theta decay\" (p.365)",
              "A|Read: NIFTY, as the text says (the printed table's strikes, p.365, are BANKNIFTY's); the nearest weekly expiry; ATM at 11:00 (strike listed as call and put nearest the spot; tie: lower), the call two listed strikes below it (in the money; the text's 'two strikes above' read as deeper in the money); a buy stop at 1.2 x its 11:00 price working from 11:00 to the close; no entry on the contract's expiry day (the book's 'Thursdays without trades'); stop at 0.7 x entry, working the same day and the next morning (a gap below it fills at the open); out at the open of 9:30 the next trading day [Declared readings].", OPTMIN],
            OPTNOTE),
        def!("as51_opt_3pm", "6.11 The 3:00 PM NIFTY strategy: at 3 pm sell the ATM call / put and the call / put nearest Rs 150 once each drops 5%; 9% stop on the Rs 150 legs; out 3:29 pm", "Long/Short", Status::Live,
            &["E|\"We may find the CMP of ATM CE and PE at 3 PM. We will wait for the CMP to reduce by 5%. We will short sell the option whose premium reduces by 5%. Similarly we will find the CE and PE options which are trading around Rs 150 and we will short sell when it reduces by 5%\" (p.367 [PDF 185])",
              "X|\"since there is just 30 minutes remaining, we will put the stop loss on only the premium legs (Rs 150). We can put a small stoploss of 9% of the entry price. We may even do trailing of the stoploss ... We may exit all legs at 3:29 PM\" (p.367)",
              "A|Read: the nearest weekly expiry; at 15:00 the ATM strike (listed as call and put nearest the spot; tie: lower) and, per type, the contract whose 15:00 price is nearest Rs 150 (tie: lower strike; when it is the ATM contract both legs are kept); each leg sold when its price falls to 0.95 x its 15:00 price (a sell order working 15:00 to 15:28); stop 1.09 x entry on the Rs 150 legs only; the optional trailing ('We may even do trailing') is not modelled; out at the open of 15:29 [Declared readings].", OPTMIN],
            OPTNOTE),
        held!("as51_opt_mom_sell", "6.12 Momentum option selling", "E|Intraday momentum option selling (p.371-377 [PDF 187-190])", "Held (lane F2, 13 Nov 2026): the NIFTY weekly 1-minute prices it needs are now held, but the chapter prints no exit time and no target: the 40% stop and the stop moved to the entry are the only exits (p.371-373), and the vault does not invent an exit."),
        def!("as51_opt_straddle", "6.13 Swing buying: buy the NIFTY weekly ATM straddle at 3:30 pm Thursday; 50% stop per leg, trail the survivor at half its premium; out on the 2nd trading day", "Long/Short", Status::Live,
            &["E|\"Buying straddle with a week-to-week an at-the-money (ATM) expiry is the tactic in question. The rationale for initiating the trade around 3:30 p.m. on a Thursday\" (p.377 [PDF 190]); \"Holding Period - 2-3 days; Execution- 3:30 p.m. on a Thursday\" (p.378)",
              "X|\"The initial for both the options will be 50% of premium. For example: If the ATM call option is of 100, the stoploss will be at 50\"; \"If one stoploss order is triggered while the other option is still active, it is recommended to trail the stoploss order of the active option. The trailing stoploss level need increase by half the amount of option premium. If the call option's stoploss level is triggered, and the put option's premium is 170, the new following stoploss level for the put option is 85\" (p.379 [PDF 191])",
              "A|Read: NIFTY (the text's example chain, 19,600-20,100, p.378; the printed results, p.379, are a BANKNIFTY run, strikes 32,600-43,400 in 2022, which the vault cannot price); every Thursday, at the open of the 15:29 minute (the session ends at 15:30), buy the call and the put of the ATM strike (listed as both, nearest the spot) of the next weekly expiry after that day (on an expiry Thursday the following week's contract); stop at 0.5 x entry per leg; when one leg stops, the other's stop rises to half its price at that minute and then to half of each new high ('trail'); the holding period '2-3 days' is a range (#36), and the book's own table exits unstopped trades at 3:30 pm on the second trading day after the Thursday (Monday, or Tuesday after a Monday holiday), so the default is the open of 15:29 on the 2nd trading day (option hold); each leg is its own position on the options account [Declared readings].", OPTMIN],
            OPTNOTE),
        held!("as51_ema_9_21", "7.1 9 and 21 EMA support and resistance (5-minute)", "E|After a consolidation, buy the 9/21 EMA cross at the breakout candle's high; stop the swing low; target a projected supply zone (p.383-393 [PDF 193-198])",
            "Held (decision #268): the consolidation and the target zone are judged by eye (p.383-393)."),
        held!("as51_rally_breakout", "7.2 Rally, consolidation and volume breakout (daily)", "E|After a 15-20% rally and a quiet consolidation, buy the high of a big high-volume green breakout candle; pyramid (p.393-399 [PDF 198-201])",
            "Held (decision #268): the range and the volume conditions are judged by eye (p.393-399)."),
        held!("as51_pin_bar", "7.3 Pin bar at range extremes", "E|Buy a pin bar at the bottom of a range, sell at the top (p.399-405 [PDF 201-204])",
            "Held (decision #268): the range is drawn by eye (p.399-405)."),
        held!("as51_retest", "7.4 Breakout retest (pullback)", "E|After a range breakout, buy the retest of the old resistance when the pullback candle's high breaks; 1:3-1:4 (p.405-409 [PDF 204-206])",
            "Held (decision #268): the range and the resistance are drawn by eye (p.405-409)."),
        data!("as51_repo_rate", "7.5 Trading on RBI repo-rate decisions (NIFTY 50)", "E|Buy NIFTY 50 at the close of an RBI policy day that cuts the repo rate, short when it hikes, none when unchanged; exit after one month (p.409-413 [PDF 206-208])",
            "Held (decision #268): a complete rule, but it needs the RBI policy dates and decisions, which the vault does not hold (p.409-413)."),
        held!("as51_vcp", "7.6 Volatility contraction pattern", "E|Minervini-style contractions, then the breakout (p.415-421 [PDF 209-212])",
            "Held (decision #268): the pattern is judged by eye (p.415-421)."),
        held!("as51_two_leg_pb", "7.7 Two-legged pullback", "E|In a trend of higher highs and lows, wait for a two-legged pullback and buy above the green candle's high (p.421-427 [PDF 212-215])",
            "Held (decision #268): the swings and legs are judged by eye (p.421-427)."),
    ]
}

pub const MIN: &str = "A|Minute path [convention, intraday_ep6]: within a bar the orders work on the real 1-minute bars: a stop or limit fills at its level, or at the minute's open when the minute opens through it; inside a minute the price runs open, the extreme nearer the open, the other extreme, close. A minute that reaches two levels without opening through exactly one of them is ambiguous; its trade is counted (decision 175: held above 5%).";

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let d = catalogue().into_iter().find(|d| d.id == id)?;
    Some((d.id, d.name, "51 Trading Strategies (Singhal)"))
}

fn pd(key: &'static str, label: &'static str, def: f64, min: f64, max: f64, step: f64) -> ParamDef { ParamDef { key, label, def, min, max, step } }

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    Some(match id {
        "as51_ichimoku" => vec![pd("rr", "Target: risk multiple (1:3)", 3., 0.5, 10., 0.5)],
        "as51_fractal_sma" => vec![pd("sma", "SMA length (50)", 50., 5., 300., 1.), pd("rr", "Target: risk multiple (1:2)", 2., 0.5, 10., 0.5)],
        "as51_donchian_pb" => vec![pd("len", "Donchian length (TradingView default 20)", 20., 5., 100., 1.), pd("pb_max", "Pullback: at most this many bars (3 to 4 printed)", 4., 3., 50., 1.),
            pd("rr", "Target: risk multiple (1:2)", 2., 0.5, 10., 0.5)],
        "as51_st_rsi" => vec![pd("atr", "Supertrend ATR length (10)", 10., 2., 50., 1.), pd("fac", "Supertrend factor (3)", 3., 0.5, 10., 0.5),
            pd("hi", "RSI buy level (60)", 60., 50., 90., 1.), pd("lo", "RSI sell level (40)", 40., 10., 50., 1.),
            pd("stop_entry", "Entry: 1 = stop at the green candle's high, the book's example (p.235); 0 = the signal bar's close", 1., 0., 1., 1.),
            pd("init_stop", "Initial stop: 1 = at the Supertrend of the signal candle (step card p.235, the example's level); 0 = none (exits at closes only)", 1., 0., 1., 1.)],
        "as51_rsi_mw" => vec![pd("lo", "Oversold (30)", 30., 5., 50., 1.), pd("hi", "Overbought (70)", 70., 50., 95., 1.),
            pd("mw", "1 = the M / W shape, two re-crosses (figure p.249); 0 = every re-cross of 30 / 70", 1., 0., 1., 1.)],
        "as51_st_pivot" => vec![pd("atr", "Supertrend ATR length (7)", 7., 2., 50., 1.), pd("fac", "Supertrend factor (3)", 3., 0.5, 10., 0.5)],
        "as51_double_rsi" => vec![pd("lo", "5-minute oversold (30)", 30., 5., 50., 1.), pd("hi", "5-minute overbought (70)", 70., 50., 95., 1.)],
        "as51_ha_psar" => vec![pd("rr", "Target: risk multiple (1:2)", 2., 0.5, 10., 0.5)],
        "as51_ema_scalp" => vec![pd("ema", "EMA length (5; the book: 5 or 7)", 5., 2., 50., 1.), pd("rr", "Target: risk multiple (1:3)", 3., 0.5, 10., 0.5)],
        // lane F2 (R4 #293): option rules on NIFTY weekly 1-minute prices
        "as51_opt_multi_sell" => vec![pd("prem", "Premium sought, Rs (around 40)", 40., 5., 500., 5.), pd("sl", "Stop, % of premium (100)", 100., 10., 500., 5.),
            pd("otm", "1 = out-of-the-money strikes only (the text, p.317); 0 = any strike, as the book's own trade list (p.318-320)", 0., 0., 1., 1.)],
        "as51_opt_oi_buy" => vec![pd("rise", "Entry: rise over the 9:20 price, % (10)", 10., 1., 100., 1.), pd("sl", "Stop, % of entry (20)", 20., 5., 90., 1.),
            pd("rr", "Target: risk multiple (1:2)", 2., 0.5, 10., 0.5), pd("hold", "Time exit: minutes after the fill (30)", 30., 1., 360., 1.),
            pd("until", "Last entry minute after 9:15 (186 = 12:21, the session's first half)", 186., 6., 359., 1.)],
        "as51_opt_theta" => vec![pd("sl", "Stop, % above entry (25)", 25., 5., 200., 1.), pd("max_trades", "Trades a day: the straddle + re-entries (3)", 3., 1., 20., 1.),
            pd("prem", "0 = ATM (the rule); > 0 = the contracts nearest this premium (the book's 125 variant)", 0., 0., 500., 5.)],
        "as51_opt_btst" => vec![pd("rise", "Entry: rise over the 11:00 price, % (20)", 20., 1., 100., 1.), pd("sl", "Stop, % below entry (30)", 30., 5., 90., 1.),
            pd("itm", "Strikes in the money (2)", 2., 0., 10., 1.)],
        "as51_opt_3pm" => vec![pd("drop", "Entry: fall below the 15:00 price, % (5)", 5., 1., 50., 1.), pd("sl", "Stop on the premium legs, % (9)", 9., 1., 100., 1.),
            pd("prem", "Premium legs: price sought at 15:00, Rs (150)", 150., 10., 1000., 10.)],
        "as51_opt_straddle" => vec![pd("sl", "Stop, % below entry (50)", 50., 5., 95., 5.), pd("trail", "Survivor's trailing stop, % of its price (50)", 50., 5., 95., 5.),
            pd("hold", "Exit on this trading day after entry (2; the book: 2-3 days, its table exits on the 2nd)", 2., 1., 5., 1.)],
        // lane F4 (R4 #300 (6)): BANKNIFTY option rules on weekly 1-minute prices
        "as51_opt_st_sell" => vec![pd("atr", "Supertrend ATR length (7, the book's chart)", 7., 2., 50., 1.), pd("fac", "Supertrend factor (3)", 3., 0.5, 10., 0.5),
            pd("last", "Last new position, minutes after 9:15 (360 = 15:15)", 360., 5., 370., 5.)],
        "as51_opt_vwap" => vec![pd("below", "Entry: premium below the VWAP, % (3)", 3., 0.5, 50., 0.5), pd("max_trades", "Trades a day (3)", 3., 1., 20., 1.),
            pd("eod", "Time exit, minutes after 9:15 (360 = 15:15; unstated in the book)", 360., 300., 374., 1.)],
        "as51_opt_mom_buy" => vec![pd("rise", "Entry: rise over the 9:30 price, % (15)", 15., 1., 100., 1.), pd("sl", "Stop, % below entry (25)", 25., 5., 90., 1.),
            pd("tgt", "Target, % above entry (100)", 100., 5., 1000., 5.), pd("until", "Last entry minute after 9:15 (359 = 15:14; unstated in the book)", 359., 16., 359., 1.)],
        "as51_opt_expiry" => vec![pd("sl", "Stop, % of premium (90)", 90., 10., 500., 5.), pd("otm", "Points out of the money (100)", 100., 0., 2000., 100.)],
        _ => return None,
    })
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if params(id).is_none() || OPT_IDS.contains(&id) || to >= x.c.len() || from > to { return None; }
    let g = |k: &str| super::jb624::getp(id, p, k);
    let n = |k: &str| g(k).round().max(1.) as usize;
    Some(match id {
        "as51_ichimoku" => ichimoku(x, from, to, g("rr")),
        "as51_fractal_sma" => fractal(x, from, to, n("sma"), g("rr")),
        "as51_donchian_pb" => donchian(x, from, to, n("len"), n("pb_max").max(3), g("rr")),
        "as51_st_rsi" => st_rsi(x, from, to, n("atr"), g("fac"), g("hi"), g("lo"), g("stop_entry") >= 0.5, g("init_stop") >= 0.5),
        "as51_rsi_mw" => rsi_mw(x, from, to, g("lo"), g("hi"), g("mw") >= 0.5),
        "as51_st_pivot" => st_pivot(x, from, to, n("atr"), g("fac")),
        "as51_double_rsi" => double_rsi(x, from, to, g("lo"), g("hi")),
        "as51_ha_psar" => ha_psar(x, from, to, g("rr")),
        _ => ema_scalp(x, from, to, n("ema"), g("rr")),
    })
}

// ---------------------------------------------------------------- indicators (TradingView built-ins)

fn highest(v: &[f64], n: usize, t: usize) -> f64 { if t + 1 < n { f64::NAN } else { v[t + 1 - n..=t].iter().copied().fold(f64::MIN, f64::max) } }
fn lowest(v: &[f64], n: usize, t: usize) -> f64 { if t + 1 < n { f64::NAN } else { v[t + 1 - n..=t].iter().copied().fold(f64::MAX, f64::min) } }

/// TradingView ta.rsi: rma (alpha 1/n, seeded with the simple average of the first n values) of the gains and of the losses;
/// 100 when the average loss is 0, 0 when the average gain is 0.
pub fn rsi(c: &[f64], n: usize) -> Vec<f64> {
    let (m, a) = (c.len(), 1.0 / n as f64);
    let mut out = vec![f64::NAN; m];
    let (mut up, mut dn) = (0.0f64, 0.0f64);
    for t in n..m {
        let ch = c[t] - c[t - 1];
        if t == n {
            up = (1..=n).map(|j| (c[j] - c[j - 1]).max(0.0)).sum::<f64>() / n as f64;
            dn = (1..=n).map(|j| (c[j - 1] - c[j]).max(0.0)).sum::<f64>() / n as f64;
        } else {
            up = a * ch.max(0.0) + (1.0 - a) * up;
            dn = a * (-ch).max(0.0) + (1.0 - a) * dn;
        }
        out[t] = if dn == 0.0 { 100.0 } else if up == 0.0 { 0.0 } else { 100.0 - 100.0 / (1.0 + up / dn) };
    }
    out
}

/// TradingView ta.ema: alpha 2/(n+1), seeded with the simple average of the first n values.
fn ema(c: &[f64], n: usize) -> Vec<f64> {
    let a = 2.0 / (n as f64 + 1.0);
    let mut out = vec![f64::NAN; c.len()];
    for t in 0..c.len() {
        out[t] = if t + 1 < n { f64::NAN } else if t + 1 == n { c[..n].iter().sum::<f64>() / n as f64 } else { a * c[t] + (1.0 - a) * out[t - 1] };
    }
    out
}

/// TradingView ta.supertrend(factor, atrPeriod): returns (line, direction) with direction -1 = up (line below), +1 = down.
pub fn supertrend(h: &[f64], l: &[f64], c: &[f64], n: usize, fac: f64) -> (Vec<f64>, Vec<i8>) {
    let m = c.len();
    // ta.atr = ta.rma(ta.tr(true), n): the first bar's true range is high - low; rma seeded with the simple average
    let tr: Vec<f64> = (0..m).map(|t| if t == 0 { h[t] - l[t] } else { (h[t] - l[t]).max((h[t] - c[t - 1]).abs()).max((l[t] - c[t - 1]).abs()) }).collect();
    let mut atr = vec![f64::NAN; m];
    for t in 0..m {
        atr[t] = if t + 1 < n { f64::NAN } else if t + 1 == n { tr[..n].iter().sum::<f64>() / n as f64 } else { (1.0 / n as f64) * tr[t] + (1.0 - 1.0 / n as f64) * atr[t - 1] };
    }
    let (mut up, mut lo, mut st, mut dir) = (vec![f64::NAN; m], vec![f64::NAN; m], vec![f64::NAN; m], vec![1i8; m]);
    for t in 0..m {
        let src = (h[t] + l[t]) / 2.0;
        let (mut u, mut d) = (src + fac * atr[t], src - fac * atr[t]);
        let nz = |v: f64| if v.is_nan() { 0.0 } else { v };
        let (pu, pl) = if t > 0 { (nz(up[t - 1]), nz(lo[t - 1])) } else { (0.0, 0.0) };
        let pc = if t > 0 { c[t - 1] } else { f64::NAN };
        if !(d > pl || pc < pl) { d = pl; }
        if !(u < pu || pc > pu) { u = pu; }
        if atr[t].is_nan() { u = f64::NAN; d = f64::NAN; }
        up[t] = u; lo[t] = d;
        dir[t] = if t == 0 || atr[t - 1].is_nan() { 1 } else if st[t - 1] == up[t - 1] { if c[t] > u { -1 } else { 1 } } else if c[t] < d { 1 } else { -1 };
        st[t] = if dir[t] == -1 { d } else { u };
    }
    (st, dir)
}

// ---------------------------------------------------------------- daily rules

#[derive(Clone, Copy)]
struct DPos { d: f64, ei: usize, ep: f64, stop: f64, tgt: f64 }

/// Daily walk: entries at closes; a working stop / target on later bars (both on one bar: the stop first, the daily-bar
/// convention, or the EP6 minute resolver when the id is in intraday_ep6::IDS); `close_exit` at closes; `trail` after them.
fn daily(x: &Ctx, from: usize, to: usize, on: bool, signal: &dyn Fn(usize) -> Option<(f64, f64, f64)>,
    close_exit: &dyn Fn(usize, &DPos) -> Option<&'static str>, trail: &dyn Fn(usize, &mut DPos)) -> Vec<Trade> {
    let mut out = vec![];
    let mut pos: Option<DPos> = None;
    for t in from..=to {
        if let Some(mut p) = pos {
            let conv = || -> Option<(f64, &'static str)> {
                let (o, hi, lo) = (x.o[t], x.h[t], x.l[t]);
                let hit_stop = |v: f64| if p.d > 0.0 { v <= p.stop } else { v >= p.stop };
                let hit_tgt = |v: f64| p.tgt.is_finite() && if p.d > 0.0 { v >= p.tgt } else { v <= p.tgt };
                if hit_stop(o) { return Some((o, "stop")); }
                if hit_tgt(o) { return Some((o, "target")); }
                if hit_stop(if p.d > 0.0 { lo } else { hi }) { return Some((p.stop, "stop")); }
                if hit_tgt(if p.d > 0.0 { hi } else { lo }) { return Some((p.tgt, "target")); }
                None
            };
            let (hit, day) = ep6::stop_target(x, t, p.d, p.stop, p.tgt, on, ("stop", "target"), conv);
            if let Some((px, why)) = hit {
                let mut z = day_trade(p.d, p.ei, p.ep, t, px, why, "close", false);
                ep6::mark(&mut z, day);
                out.push(z);
                pos = None;
            } else if let Some(why) = close_exit(t, &p) {
                out.push(day_trade(p.d, p.ei, p.ep, t, x.c[t], why, "close", false));
                pos = None;
            } else {
                trail(t, &mut p);
                pos = Some(p);
            }
        }
        if pos.is_none() {
            if let Some((d, stop, tgt)) = signal(t) { pos = Some(DPos { d, ei: t, ep: x.c[t], stop, tgt }); }
        }
    }
    if let Some(p) = pos { out.push(day_trade(p.d, p.ei, p.ep, to, x.c[to], "open", "close", true)); }
    out
}
fn none_exit(_: usize, _: &DPos) -> Option<&'static str> { None }
fn no_trail(_: usize, _: &mut DPos) {}

fn ichimoku(x: &Ctx, from: usize, to: usize, rr: f64) -> Vec<Trade> {
    let (h, l, c) = (&x.h, &x.l, &x.c);
    let mid = |n: usize, t: usize| (highest(h, n, t) + lowest(l, n, t)) / 2.0;
    let m = c.len();
    let conv: Vec<f64> = (0..m).map(|t| mid(9, t)).collect();
    let base: Vec<f64> = (0..m).map(|t| mid(26, t)).collect();
    let lead1: Vec<f64> = (0..m).map(|t| (conv[t] + base[t]) / 2.0).collect();
    let lead2: Vec<f64> = (0..m).map(|t| mid(52, t)).collect();
    let a = |t: usize| if t >= 25 { lead1[t - 25] } else { f64::NAN };
    let b = |t: usize| if t >= 25 { lead2[t - 25] } else { f64::NAN };
    let long = |t: usize| conv[t] > base[t] && c[t] > a(t) && c[t] > b(t);
    let short = |t: usize| conv[t] < base[t] && c[t] < a(t) && c[t] < b(t);
    let sig = |t: usize| -> Option<(f64, f64, f64)> {
        if t == 0 { return None; }
        let s = a(t);
        if long(t) && !long(t - 1) { return Some((1.0, s, c[t] + rr * (c[t] - s))); }
        if short(t) && !short(t - 1) { return Some((-1.0, s, c[t] - rr * (s - c[t]))); }
        None
    };
    daily(x, from, to, ep6::on("as51_ichimoku", &Params::new()), &sig, &none_exit, &no_trail)
}

fn fractal(x: &Ctx, from: usize, to: usize, n: usize, rr: f64) -> Vec<Trade> {
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let sma = crate::connors::sma(c, n);
    let m = c.len();
    // the last confirmed fractal high / low at each close: (level, broken)
    let mut fh: Vec<Option<f64>> = vec![None; m];
    let mut fl: Vec<Option<f64>> = vec![None; m];
    let (mut ch, mut cl): (Option<f64>, Option<f64>) = (None, None);
    let mut used = vec![(false, false); m];
    let (mut uh, mut ul) = (false, false);
    for t in 0..m {
        if t >= 8 {
            let k = t - 4;
            if (k - 4..=k + 4).all(|j| h[j] <= h[k]) { ch = Some(h[k]); uh = false; }
            if (k - 4..=k + 4).all(|j| l[j] >= l[k]) { cl = Some(l[k]); ul = false; }
        }
        fh[t] = ch; fl[t] = cl;
        used[t] = (uh, ul);
        if let Some(v) = ch { if c[t] > v { uh = true; } }
        if let Some(v) = cl { if c[t] < v { ul = true; } }
    }
    let sig = |t: usize| -> Option<(f64, f64, f64)> {
        if let (Some(fhv), Some(flv)) = (fh[t], fl[t]) {
            if !used[t].0 && c[t] > fhv && c[t] > sma[t] && c[t] > o[t] && flv < c[t] { return Some((1.0, flv, c[t] + rr * (c[t] - flv))); }
            if !used[t].1 && c[t] < flv && c[t] < sma[t] && c[t] < o[t] && fhv > c[t] { return Some((-1.0, fhv, c[t] - rr * (fhv - c[t]))); }
        }
        None
    };
    daily(x, from, to, ep6::on("as51_fractal_sma", &Params::new()), &sig, &none_exit, &no_trail)
}

fn donchian(x: &Ctx, from: usize, to: usize, n: usize, pb_max: usize, rr: f64) -> Vec<Trade> {
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let m = c.len();
    let up: Vec<f64> = (0..m).map(|t| highest(h, n, t)).collect();
    let lo: Vec<f64> = (0..m).map(|t| lowest(l, n, t)).collect();
    // the last touch before each bar
    let (mut lu, mut ll) = (vec![None; m], vec![None; m]);
    let (mut a, mut b): (Option<usize>, Option<usize>) = (None, None);
    for t in 0..m {
        lu[t] = a; ll[t] = b;
        if h[t] >= up[t] { a = Some(t); }
        if l[t] <= lo[t] { b = Some(t); }
    }
    let pb = |k: Option<usize>, t: usize| k.is_some_and(|k| (3..=pb_max).contains(&(t - k - 1)));
    let sig = |t: usize| -> Option<(f64, f64, f64)> {
        if t == 0 || up[t - 1].is_nan() { return None; }
        let mid = (up[t] + lo[t]) / 2.0;
        if h[t] > up[t - 1] && pb(lu[t], t) && mid < c[t] { return Some((1.0, mid, c[t] + rr * (c[t] - mid))); }
        if l[t] < lo[t - 1] && pb(ll[t], t) && mid > c[t] { return Some((-1.0, mid, c[t] - rr * (mid - c[t]))); }
        None
    };
    let trail = |t: usize, p: &mut DPos| {
        if t < p.ei + 1 { return; }
        if p.d > 0.0 && c[t] > o[t] && c[t - 1] > o[t - 1] { p.stop = p.stop.max(l[t - 1]); }
        if p.d < 0.0 && c[t] < o[t] && c[t - 1] < o[t - 1] { p.stop = p.stop.min(h[t - 1]); }
    };
    daily(x, from, to, ep6::on("as51_donchian_pb", &Params::new()), &sig, &none_exit, &trail)
}

fn st_rsi(x: &Ctx, from: usize, to: usize, n: usize, fac: f64, hi: f64, lo: f64, stop_entry: bool, init_stop: bool) -> Vec<Trade> {
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let (line, dir) = supertrend(h, l, c, n, fac);
    let r = rsi(c, 14);
    // no setup before the Supertrend exists (its direction reads 'down' during the ATR warm-up; matters only for atr > 14)
    let up = |t: usize| t > 0 && line[t].is_finite() && dir[t] == -1 && r[t] > hi && r[t - 1] <= hi;
    let dn = |t: usize| t > 0 && line[t].is_finite() && dir[t] == 1 && r[t] < lo && r[t - 1] >= lo;
    let exit = |t: usize, d: f64| -> Option<&'static str> {
        if d > 0.0 && dir[t] == 1 { Some("close below the Supertrend") } else if d < 0.0 && dir[t] == -1 { Some("close above the Supertrend") } else { None }
    };
    // Decision #273(2): the printed step card places an initial stop ('Place STOPLOSS at the swing low', p.235); its level is the
    // example's, the Supertrend at the signal candle (p.235 'the initial stoploss was at the Supertrend'), a fixed working stop.
    let sl = |t: usize| if init_stop { line[t] } else { f64::NAN };
    if !stop_entry {
        // the option: entry at the signal bar's close, no candle colour required; the initial stop works from the next bar
        let sig = |t: usize| if up(t) { Some((1.0, sl(t), f64::NAN)) } else if dn(t) { Some((-1.0, sl(t), f64::NAN)) } else { None };
        return daily(x, from, to, false, &sig, &|t: usize, p: &DPos| exit(t, p.d), &no_trail);
    }
    // Decision #271(3), the default: a green signal bar places a buy stop at its high for the next bar only (a red one a sell
    // stop at its low); an open through the level fills at the open. The initial stop works from the entry fill on: on the
    // entry bar after the fill (a bar that fills the entry inside its range and reaches the stop is counted for #240 and
    // read as stopped), on later bars at the stop or the open beyond it; then the exits at closes.
    let mut out = vec![];
    let (mut pos, mut pend): (Option<(f64, usize, f64, f64)>, Option<(f64, f64, f64)>) = (None, None);
    for t in from..=to {
        if let Some((d, lv, st)) = pend.take() {
            let px = if d > 0.0 { if o[t] >= lv { Some(o[t]) } else if h[t] >= lv { Some(lv) } else { None } }
                else if o[t] <= lv { Some(o[t]) } else if l[t] <= lv { Some(lv) } else { None };
            if let Some(px) = px { pos = Some((d, t, px, st)); }
        }
        if let Some((d, ei, ep, st)) = pos {
            let gap = t > ei && if d > 0.0 { o[t] <= st } else { o[t] >= st };
            let touch = if d > 0.0 { l[t] <= st } else { h[t] >= st };
            if gap || touch { out.push(day_trade(d, ei, ep, t, if gap { o[t] } else { st }, "initial stop", "stop", false)); pos = None; }
            else if let Some(why) = exit(t, d) { out.push(day_trade(d, ei, ep, t, c[t], why, "stop", false)); pos = None; }
        }
        if pos.is_none() {
            if up(t) && c[t] > o[t] { pend = Some((1.0, h[t], sl(t))); } else if dn(t) && c[t] < o[t] { pend = Some((-1.0, l[t], sl(t))); }
        }
    }
    if let Some((d, ei, ep, _)) = pos { out.push(day_trade(d, ei, ep, to, c[to], "open", "stop", true)); }
    out
}

fn rsi_mw(x: &Ctx, from: usize, to: usize, lo: f64, hi: f64, shape: bool) -> Vec<Trade> {
    let (h, l) = (&x.h, &x.l);
    let r = rsi(&x.c, 14);
    // Decision #271(1): the M / W shape. A re-cross of 30 (70) is a signal only when it is the second one: the first re-cross
    // arms the W (M), the second fires and disarms. The count runs on the RSI alone, in or out of a position, with no bar limit.
    let (mut w, mut mm) = (vec![!shape; r.len()], vec![!shape; r.len()]);
    if shape {
        let (mut aw, mut am) = (false, false);
        for t in 1..r.len() {
            if r[t - 1] <= lo && r[t] > lo { w[t] = aw; aw = !aw; }
            if r[t - 1] >= hi && r[t] < hi { mm[t] = am; am = !am; }
        }
    }
    let sig = |t: usize| -> Option<(f64, f64, f64)> {
        if t == 0 { return None; }
        if r[t - 1] <= lo && r[t] > lo && w[t] {
            let mut k = t - 1;
            while k > 0 && r[k - 1] <= lo { k -= 1; }
            return Some((1.0, (k..=t).map(|j| l[j]).fold(f64::MAX, f64::min), f64::NAN));
        }
        if r[t - 1] >= hi && r[t] < hi && mm[t] {
            let mut k = t - 1;
            while k > 0 && r[k - 1] >= hi { k -= 1; }
            return Some((-1.0, (k..=t).map(|j| h[j]).fold(f64::MIN, f64::max), f64::NAN));
        }
        None
    };
    let ex = |t: usize, p: &DPos| -> Option<&'static str> {
        if p.d > 0.0 && r[t] >= hi { Some("RSI reached overbought") } else if p.d < 0.0 && r[t] <= lo { Some("RSI reached oversold") } else { None }
    };
    daily(x, from, to, false, &sig, &ex, &no_trail)
}

// ---------------------------------------------------------------- intraday rules (1-minute file)

/// One N-minute bar of the continuous series.
struct B { i: usize, s: u16, e: u16, open: u16, close: u16, o: f64, h: f64, l: f64, c: f64, m: Vec<Bar1>, first: bool, last: bool }

fn series(x: &Ctx, from: usize, to: usize, n: u16) -> Vec<B> {
    let mut out = vec![];
    for i in from..=to {
        let Some((open, close, m, v)) = crate::intraday_ep::day_bars(x, i, n) else { continue };
        let k = v.len();
        out.extend(v.into_iter().enumerate().map(|(j, z)| B { i, s: z.s, e: z.e, open, close, o: z.o, h: z.h, l: z.l, c: z.c, m: m[z.a..z.b].to_vec(), first: j == 0, last: j + 1 == k }));
    }
    out
}

/// A working entry order: direction, level, protective stop, target risk multiple (NaN: none), last bar index it works on.
#[derive(Clone, Copy, Debug)]
struct Pend { d: f64, lv: f64, stop: f64, rr: f64, until: usize }
#[derive(Clone, Copy, Debug)]
struct IPos { d: f64, ei: usize, ep: f64, stop: f64, tgt: f64, amb: bool }
#[derive(Clone, Default)]
struct W { pend: Option<Pend>, pos: Option<IPos>, done: Vec<(IPos, f64, &'static str)> }

/// A level reached on segment g by a move in direction `up` (already beyond at the start counts): the fill price.
fn reach(g: &Seg, lv: f64, up: bool) -> Option<f64> {
    if !lv.is_finite() { return None; }
    let beyond = if up { g.a >= lv } else { g.a <= lv };
    if beyond { return Some(if g.gap { g.b } else { g.a }); }
    let hit = if up { g.b >= lv } else { g.b <= lv };
    hit.then(|| if g.gap { g.b } else { lv })
}

/// Walk one path piece by piece: entry, then the stop and the target of the new or held position.
fn walk(s: &mut W, segs: &[Seg], i: usize) -> Vec<Ev> {
    let mut ev = vec![];
    for g in segs {
        let mut g = *g;
        loop {
            if s.pos.is_none() {
                if let Some(p) = s.pend {
                    if let Some(px) = reach(&g, p.lv, p.d > 0.0) {
                        let tgt = if p.rr.is_finite() { px + p.rr * (px - p.stop) } else { f64::NAN };
                        s.pos = Some(IPos { d: p.d, ei: i, ep: px, stop: p.stop, tgt, amb: false });
                        s.pend = None;
                        ev.push(Ev { k: 0, px, up: p.d > 0.0 });
                        g = Seg { a: px, b: g.b, gap: false };
                        continue;
                    }
                }
                break;
            }
            let q = s.pos.unwrap();
            let st = reach(&g, q.stop, q.d < 0.0);
            let tg = reach(&g, q.tgt, q.d > 0.0);
            let (px, k, why) = match (st, tg) {
                (Some(a), _) => (a, 1u8, "stop"),
                (None, Some(b)) => (b, 2u8, "target"),
                _ => break,
            };
            s.done.push((q, px, why));
            s.pos = None;
            ev.push(Ev { k, px, up: k == 2 && q.d > 0.0 || k == 1 && q.d < 0.0 });
            g = Seg { a: px, b: g.b, gap: false };
        }
    }
    ev
}

/// Drive a bar list: `place(k, &W) -> Option<Pend>` at bar k's close (flat, no order), `stop_of(k, &IPos) -> f64` the stop
/// working on bar k (trailing), `close_exit(k, &IPos)` at bar k's close, `flat(k)` = a square-off bar.
fn drive(x: &Ctx, v: &[B], place: &dyn Fn(usize) -> Option<Pend>, stop_of: &dyn Fn(usize, &IPos) -> f64,
    close_exit: &dyn Fn(usize, &IPos) -> Option<&'static str>, flat: &dyn Fn(usize) -> bool, to: usize) -> Vec<Trade> {
    let mut out = vec![];
    let mut s = W::default();
    let mut prev = f64::NAN;
    let push = |out: &mut Vec<Trade>, q: IPos, xi: usize, px: f64, why: &str, amb: bool| {
        let mut z = day_trade(q.d, q.ei, q.ep, xi, px, why, "stop", false);
        if amb || q.amb { z.reason.push_str(AMB); }
        out.push(z);
    };
    for (k, b) in v.iter().enumerate() {
        if b.first { prev = b.o; }
        if let Some(q) = s.pos.as_mut() { let st = stop_of(k, q); q.stop = st; }
        if s.pend.is_some() || s.pos.is_some() {
            let (amb, w) = ep6::walk_minutes(&b.m, prev, std::mem::take(&mut s), &|st: &mut W, segs: &[Seg]| walk(st, segs, b.i));
            s = w;
            for (q, px, why) in std::mem::take(&mut s.done) { push(&mut out, q, b.i, px, why, amb); }
            if amb { if let Some(q) = s.pos.as_mut() { q.amb = true; } }
        }
        prev = b.c;
        if let Some(q) = s.pos {
            let why = close_exit(k, &q).or_else(|| (b.last || flat(k)).then_some("square-off"));
            if let Some(why) = why { push(&mut out, q, b.i, b.c, why, false); s.pos = None; }
        }
        if b.last || flat(k) { s.pend = None; continue; }
        // Decision #273(1): an order expires after its last bar (`until`), so an unfilled next-bar order never blocks a fresh
        // signal at this bar's close; every bar is evaluated (TradeStation semantics, #184/#218).
        if s.pend.is_some_and(|p| p.until <= k) { s.pend = None; }
        if s.pos.is_none() && s.pend.is_none() { s.pend = place(k); }
    }
    if let Some(q) = s.pos {
        let mut z = day_trade(q.d, q.ei, q.ep, to, x.c[to], "open", "stop", true);
        if q.amb { z.reason.push_str(AMB); }
        out.push(z);
    }
    out
}
pub const AMB: &str = " · ambiguous minute";

fn st_pivot(x: &Ctx, from: usize, to: usize, n: usize, fac: f64) -> Vec<Trade> {
    let v = series(x, from.max(1), to, 5);
    let (h, l, c): (Vec<f64>, Vec<f64>, Vec<f64>) = (v.iter().map(|b| b.h).collect(), v.iter().map(|b| b.l).collect(), v.iter().map(|b| b.c).collect());
    let (_, dir) = supertrend(&h, &l, &c, n, fac);
    let cut = |k: usize| v[k].e >= v[k].close - 15;
    let place = |k: usize| -> Option<Pend> {
        let b = &v[k];
        let (ph, pl, pc) = (x.h[b.i - 1], x.l[b.i - 1], x.c[b.i - 1]);
        let p = (ph + pl + pc) / 3.0;
        let (r1, s1) = (2.0 * p - pl, 2.0 * p - ph);
        if b.c > r1 && dir[k] == -1 && b.c > b.o { return Some(Pend { d: 1.0, lv: b.c, stop: f64::NAN, rr: f64::NAN, until: k + 1 }); }
        if b.c < s1 && dir[k] == 1 && b.c < b.o { return Some(Pend { d: -1.0, lv: b.c, stop: f64::NAN, rr: f64::NAN, until: k + 1 }); }
        None
    };
    let ex = |k: usize, q: &IPos| -> Option<&'static str> {
        if q.d > 0.0 && dir[k] == 1 { Some("close below the Supertrend") } else if q.d < 0.0 && dir[k] == -1 { Some("close above the Supertrend") } else { None }
    };
    drive(x, &v, &place, &|_, q| q.stop, &ex, &cut, to)
}

fn double_rsi(x: &Ctx, from: usize, to: usize, lo: f64, hi: f64) -> Vec<Trade> {
    let v = series(x, from, to, 5);
    let c: Vec<f64> = v.iter().map(|b| b.c).collect();
    let r5 = rsi(&c, 14);
    // hourly bars (session-aligned 60-minute) and their RSI; the value at a 5-minute bar = the last hourly bar completed by it
    let hv = series(x, from, to, 60);
    let rh = rsi(&hv.iter().map(|b| b.c).collect::<Vec<_>>(), 14);
    let mut r60 = vec![f64::NAN; v.len()];
    let mut j = 0usize;
    let mut cur = f64::NAN;
    for (k, b) in v.iter().enumerate() {
        while j < hv.len() && (hv[j].i < b.i || (hv[j].i == b.i && hv[j].e <= b.e)) { cur = rh[j]; j += 1; }
        r60[k] = cur;
    }
    let place = |k: usize| -> Option<Pend> {
        if k == 0 { return None; }
        let b = &v[k];
        if r60[k] > 50.0 && r5[k].min(r5[k - 1]) < lo && b.c > b.o { return Some(Pend { d: 1.0, lv: b.h, stop: b.l, rr: f64::NAN, until: k + 1 }); }
        if r60[k] < 50.0 && r5[k].max(r5[k - 1]) > hi && b.c < b.o { return Some(Pend { d: -1.0, lv: b.l, stop: b.h, rr: f64::NAN, until: k + 1 }); }
        None
    };
    let ex = |k: usize, q: &IPos| -> Option<&'static str> {
        if q.d > 0.0 && r5[k - 1] > hi && r5[k] <= hi { Some("RSI back below 70") } else if q.d < 0.0 && r5[k - 1] < lo && r5[k] >= lo { Some("RSI back above 30") } else { None }
    };
    drive(x, &v, &place, &|_, q| q.stop, &ex, &|_| false, to)
}

/// TradingView ta.sar(start, inc, max) on (h, l, c): returns (sar, below, next) where next[t] = the level the SAR will have
/// on bar t+1 before that bar's flip check (known at bar t's close).
pub fn psar(h: &[f64], l: &[f64], c: &[f64], start: f64, inc: f64, max: f64) -> (Vec<f64>, Vec<bool>, Vec<f64>) {
    let m = c.len();
    let (mut out, mut bel, mut nxt) = (vec![f64::NAN; m], vec![false; m], vec![f64::NAN; m]);
    let (mut res, mut mm, mut acc, mut below) = (f64::NAN, f64::NAN, f64::NAN, false);
    let clamp = |r: f64, below: bool, t: usize| -> f64 {
        let mut r = r;
        if below { r = r.min(l[t - 1]); if t > 1 { r = r.min(l[t - 2]); } } else { r = r.max(h[t - 1]); if t > 1 { r = r.max(h[t - 2]); } }
        r
    };
    for t in 1..m {
        let mut first = false;
        if t == 1 {
            if c[1] > c[0] { below = true; mm = h[1]; res = l[0]; } else { below = false; mm = l[1]; res = h[0]; }
            first = true;
            acc = start;
        }
        res += acc * (mm - res);
        if below {
            if res > l[t] { first = true; below = false; res = h[t].max(mm); mm = l[t]; acc = start; }
        } else if res < h[t] { first = true; below = true; res = l[t].min(mm); mm = h[t]; acc = start; }
        if !first {
            if below { if h[t] > mm { mm = h[t]; acc = (acc + inc).min(max); } } else if l[t] < mm { mm = l[t]; acc = (acc + inc).min(max); }
        }
        res = clamp(res, below, t);
        out[t] = res; bel[t] = below;
        if t + 1 < m { nxt[t] = clamp(res + acc * (mm - res), below, t + 1); }
    }
    (out, bel, nxt)
}

fn ha_psar(x: &Ctx, from: usize, to: usize, rr: f64) -> Vec<Trade> {
    let v = series(x, from, to, 3);
    let m = v.len();
    let (mut ho, mut hc, mut hh, mut hl) = (vec![0.0; m], vec![0.0; m], vec![0.0; m], vec![0.0; m]);
    for (k, b) in v.iter().enumerate() {
        hc[k] = (b.o + b.h + b.l + b.c) / 4.0;
        ho[k] = if k == 0 { (b.o + b.c) / 2.0 } else { (ho[k - 1] + hc[k - 1]) / 2.0 };
        hh[k] = b.h.max(ho[k]).max(hc[k]);
        hl[k] = b.l.min(ho[k]).min(hc[k]);
    }
    let r = rsi(&hc, 14);
    let (_, below, nxt) = psar(&hh, &hl, &hc, 0.02, 0.02, 0.2);
    // Decision #271(2): the book's bullish candle has no lower wick (HA low = HA open), the bearish one no upper wick (HA high =
    // HA open), p.255. The HA open is a running average, so equality is tested within 1e-9 x the HA open (float rounding only).
    let flat = |a: f64, b: f64, k: usize| (a - b).abs() <= 1e-9 * ho[k].abs();
    let place = |k: usize| -> Option<Pend> {
        if k == 0 || k + 1 >= m { return None; }
        let stop = nxt[k];
        let (bull, bear) = (flat(hl[k], ho[k], k), flat(hh[k], ho[k], k));
        if hc[k] > ho[k] && bull && below[k] && r[k - 1] <= 50.0 && r[k] > 50.0 && stop < hh[k] { return Some(Pend { d: 1.0, lv: hh[k], stop, rr, until: k + 1 }); }
        if hc[k] < ho[k] && bear && !below[k] && r[k - 1] >= 50.0 && r[k] < 50.0 && stop > hl[k] { return Some(Pend { d: -1.0, lv: hl[k], stop, rr, until: k + 1 }); }
        None
    };
    // the stop working on bar k is the SAR level for bar k (from bar k-1's close)
    drive(x, &v, &place, &|k, _| nxt[k - 1], &|_, _| None, &|_| false, to)
}

fn ema_scalp(x: &Ctx, from: usize, to: usize, n: usize, rr: f64) -> Vec<Trade> {
    let v = series(x, from, to, 5);
    let e = ema(&v.iter().map(|b| b.c).collect::<Vec<_>>(), n);
    let first_hour = |k: usize| v[k].s < v[k].open + 60;
    let mut until = vec![0usize; v.len()];
    for k in 0..v.len() { let mut u = k; while u + 1 < v.len() && v[u + 1].i == v[k].i && first_hour(u + 1) { u += 1; } until[k] = u; }
    let place = |k: usize| -> Option<Pend> {
        // the day's 2nd bar (k-1 is the day's first)
        if k == 0 || !v[k - 1].first || v[k].i != v[k - 1].i || until[k] <= k { return None; }
        let b = &v[k];
        if b.c > e[k] && b.l > e[k] { return Some(Pend { d: -1.0, lv: b.l, stop: b.h, rr, until: until[k] }); }
        if b.c < e[k] && b.h < e[k] { return Some(Pend { d: 1.0, lv: b.h, stop: b.l, rr, until: until[k] }); }
        None
    };
    drive(x, &v, &place, &|_, q| q.stop, &|_, _| None, &|_| false, to)
}

// ---------------------------------------------------------------- lane F2 (13 Nov 2026, R4 #293): option rules, NIFTY weekly 1-minute
// Clock minutes after 09:15 (the bar stamped 09:15 = 0).
const T0916: usize = 1;
const T0920: usize = 5;
const T0930: usize = 15;
const T1100: usize = 105;
const T1500: usize = 345;
const T1515: usize = 360;
const T1529: usize = 374;
const T0955: usize = 40;
const T1000: usize = 45;

fn hhmm(m: usize) -> String { format!("{:02}:{:02}", (555 + m) / 60, (555 + m) % 60) }

/// One finished (or still open) leg as an options-account position; `marks` = the leg's closing values on the days before
/// its exit day (overnight rules).
fn leg_pos(x: &Ctx, d: &crate::optw1::Day, i: usize, l: &crate::optw1::Leg, entry: (f64, usize), out: (usize, f64, usize, String), marks: &[f64], open: bool) -> Option<crate::options::Pos> {
    use crate::optw1::SPOT;
    let (strike, put, _) = d.k[l.ci];
    let s0 = d.price_at(SPOT, entry.1)?;
    let (exit_i, px, m, why) = out;
    let mut pnl: Vec<f64> = marks.iter().map(|v| l.q * (v - entry.0)).collect();
    pnl.push(l.q * (px - entry.0));
    let dir = if (l.q > 0.0) != put { 1.0 } else { -1.0 };
    let side = if l.q < 0.0 { "sold" } else { "bought" };
    let ty = if put { "PE" } else { "CE" };
    let exp = d.expiry;
    let amb = if l.amb { " · ambiguous minute (#175: the order inside one minute is unknown; nearer extreme first)" } else { "" };
    let when = if exit_i == i { hhmm(m) } else { format!("{} {}", x.r[exit_i].date, hhmm(m)) };
    let und = if x.r[0].ticker == "NIFTYBANK" { "BANKNIFTY" } else { "NIFTY" };
    Some(crate::options::Pos { dir, s0, entry_i: i, exit_i, pnl, open,
        reason: format!("{side} {und} {strike} {ty} exp {}-{:02}-{:02} at {} {:.2}; out {when} {px:.2} ({why}){amb}", exp / 10000, exp / 100 % 100, exp % 100, hhmm(entry.1), entry.0) })
}

/// Finished legs of one intraday day as positions (a leg still open at the end of the day leaves at its last traded price).
fn day_pos(x: &Ctx, d: &crate::optw1::Day, i: usize, legs: &mut [crate::optw1::Leg], out: &mut Vec<crate::options::Pos>) {
    use crate::optw1::St;
    for l in legs.iter_mut() {
        let Some(e) = l.fill else { continue };
        if matches!(l.st, St::Open { .. }) {
            let px = d.last_close(l.ci).unwrap_or(e.0);
            l.close(px, crate::optw1::MINUTES - 1, "no trade at or after the exit time: last traded price");
        }
        if let St::Done { px, m, why } = &l.st { if let Some(p) = leg_pos(x, d, i, l, e, (i, *px, *m, why.clone()), &[], false) { out.push(p); } }
    }
}

/// 5-minute bars of contract `ci` on one day (session-aligned: 09:15-09:19 ... 15:25-15:29): (bar no., open, high, low, close).
fn five(d: &crate::optw1::Day, ci: usize) -> Vec<(usize, f64, f64, f64, f64)> {
    (0..crate::optw1::MINUTES / 5).filter_map(|j| {
        let v: Vec<crate::optw1::B> = (5 * j..5 * j + 5).filter_map(|m| d.bars[ci][m]).collect();
        let (f, z) = (v.first()?, v.last()?);
        Some((j, f.o, v.iter().map(|b| b.h).fold(f64::MIN, f64::max), v.iter().map(|b| b.l).fold(f64::MAX, f64::min), z.c))
    }).collect()
}

/// TradingView ta.supertrend fed one bar at a time: the same arithmetic as `supertrend` (lane F4, 6.4 runs one continuous series
/// across the days it reads).
pub struct StInc { n: usize, fac: f64, t: usize, trs: f64, atr: f64, up: f64, lo: f64, st: f64, pc: f64 }
impl StInc {
    pub fn new(n: usize, fac: f64) -> StInc { StInc { n, fac, t: 0, trs: 0.0, atr: f64::NAN, up: f64::NAN, lo: f64::NAN, st: f64::NAN, pc: f64::NAN } }
    /// Add one bar; Some(direction: -1 up / green, +1 down / red) once it is evaluated (the ATR existed on the bar before).
    pub fn push(&mut self, h: f64, l: f64, c: f64) -> Option<i8> {
        let (t, n) = (self.t, self.n);
        let tr = if t == 0 { h - l } else { (h - l).max((h - self.pc).abs()).max((l - self.pc).abs()) };
        let atr = if t + 1 < n { self.trs += tr; f64::NAN } else if t + 1 == n { (self.trs + tr) / n as f64 } else { (1.0 / n as f64) * tr + (1.0 - 1.0 / n as f64) * self.atr };
        let src = (h + l) / 2.0;
        let (mut u, mut d) = (src + self.fac * atr, src - self.fac * atr);
        let nz = |v: f64| if v.is_nan() { 0.0 } else { v };
        let (pu, pl) = if t > 0 { (nz(self.up), nz(self.lo)) } else { (0.0, 0.0) };
        let pc = if t > 0 { self.pc } else { f64::NAN };
        if !(d > pl || pc < pl) { d = pl; }
        if !(u < pu || pc > pu) { u = pu; }
        if atr.is_nan() { u = f64::NAN; d = f64::NAN; }
        let evaluated = t > 0 && !self.atr.is_nan();
        let dir: i8 = if !evaluated { 1 } else if self.st == self.up { if c > u { -1 } else { 1 } } else if c < d { 1 } else { -1 };
        (self.up, self.lo, self.st, self.atr, self.pc, self.t) = (u, d, if dir == -1 { d } else { u }, atr, c, t + 1);
        evaluated.then_some(dir)
    }
}

/// Lane F2 / F4 option rules (None: not one of them; Err: no weekly 1-minute option prices of the rule's market for this instrument).
pub fn option_positions(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Result<Vec<crate::options::Pos>, String>> {
    if !OPT_IDS.contains(&id) { return None; }
    let ticker = x.r.first().map_or("", |r| r.ticker.as_str());
    let bnf = BNF_IDS.contains(&id);
    // each rule trades its own market's options: NIFTY rules on NIFTY 50, the BANKNIFTY rules (lane F4) on NIFTYBANK
    let Some(st) = (ticker == if bnf { "NIFTYBANK" } else { "NIFTY 50" }).then(|| crate::optw1::store(ticker)).flatten() else {
        return Some(Err(if bnf { "needs_data: no BANKNIFTY weekly 1-minute option prices are loaded for this instrument (the rule trades BANKNIFTY weekly options)" }
            else { "needs_data: no NIFTY weekly 1-minute option prices are loaded for this instrument" }.into()));
    };
    Some(positions_in(id, x, from, to, p, st, crate::optw1::store_next(ticker)))
}

fn positions_in(id: &str, x: &Ctx, from: usize, to: usize, p: &Params, st: &crate::optw1::Store, next_store: Option<&crate::optw1::Store>) -> Result<Vec<crate::options::Pos>, String> {
    use crate::optw1::{Leg, St, MINUTES, SPOT};
    if to >= x.r.len() || from > to { return Ok(vec![]); }
    if id == "as51_opt_straddle" && next_store.is_none() {
        return Err("needs_data: the next weekly expiry's 1-minute prices on expiry days (NIFTY_50.w1n) are not loaded; 6.13 buys that contract on expiry Thursdays".into());
    }
    let g = |k: &str| super::jb624::getp(id, p, k);
    let mut out = Vec::new();
    // days the rule could not trade, said on the next position (bar index, note)
    let mut notes: Vec<(usize, String)> = Vec::new();
    // 6.4 (lane F4): the Supertrend runs on one continuous 5-minute series, started 5 trading days before the window
    let mut stinc = (id == "as51_opt_st_sell").then(|| StInc::new(g("atr").round().max(1.0) as usize, g("fac")));
    if let Some(s) = stinc.as_mut() {
        for j in from.saturating_sub(5)..from {
            let dt = x.r[j].date.as_str();
            if !st.has(dt) { continue; }
            let d = st.day(dt).ok_or_else(|| format!("calculation_unavailable: the weekly option minutes of {dt} could not be read (malformed store block); no result is published"))?;
            for (_, _, h, l, c) in five(&d, SPOT) { s.push(h, l, c); }
        }
    }
    let mut i = from;
    while i <= to {
        let date = x.r[i].date.as_str();
        if !st.has(date) {
            if date > st.first() && date < st.last() { notes.push((i, format!("{date}: no option minutes in the store"))); }
            i += 1;
            continue;
        }
        let Some(d) = st.day(date) else {
            return Err(format!("calculation_unavailable: the weekly option minutes of {date} could not be read (malformed store block); no result is published"));
        };
        // 6.4: the day's index bars feed the continuous Supertrend (every store day of the vault's calendar, short sessions too)
        let dirs: Vec<(usize, Option<i8>)> = match stinc.as_mut() { Some(s) => five(&d, SPOT).into_iter().map(|(j, _, h, l, c)| (j, s.push(h, l, c))).collect(), None => vec![] };
        if d.special() {
            notes.push((i, if BNF_IDS.contains(&id) { format!("{date}: fewer than 300 NIFTY BANK index minutes in the source (a trading halt, a special Saturday or Muhurat session), not traded") }
                else { format!("{date}: fewer than 300 NIFTY spot minutes in the store (a special session, or missing spot minutes as on 2026-09-30), not traded") }));
            i += 1;
            continue;
        }
        // a day with index minutes but no option minutes (the BANKNIFTY source lacks every contract on 2023-02-08; lane F4)
        if d.k.len() <= 1 { notes.push((i, format!("{date}: no option minutes in the source on this day, not traded"))); i += 1; continue; }
        let short = |ci: usize, stt: St| { let mut l = Leg::new(ci, -1.0, stt); l.exit_m = T1515; l.exit_why = "15:15 exit"; l };
        match id {
            "as51_opt_multi_sell" => {
                let sl = g("sl") / 100.0;
                let mut legs = Vec::new();
                for at in [T0916, T0916 + 60, T0916 + 120, T0916 + 180] {
                    for put in [false, true] {
                        let Some(ci) = d.nearest_premium(put, at, g("prem"), g("otm") >= 0.5) else { continue };
                        let mut l = short(ci, St::Timed { at, until: T1515 - 1 });
                        (l.stop, l.trail_at, l.trail_to) = (sl, 0.5, (1.0 + sl) / 2.0);
                        legs.push(l);
                    }
                }
                crate::optw1::walk(&d, &mut legs, T0916, &mut |_, _| {});
                day_pos(x, &d, i, &mut legs, &mut out);
            }
            "as51_opt_oi_buy" => {
                let (rise, sl) = (g("rise") / 100.0, g("sl") / 100.0);
                let mut legs = Vec::new();
                for put in [false, true] {
                    let best = (1..d.k.len()).filter(|&c| d.k[c].1 == put && d.k[c].2 > 0.0)
                        .fold(None, |b: Option<usize>, c| match b { Some(b) if d.k[b].2 > d.k[c].2 || (d.k[b].2 == d.k[c].2 && d.k[b].0 <= d.k[c].0) => Some(b), _ => Some(c) });
                    let Some(ci) = best else { continue };
                    let Some(p0) = d.price_at(ci, T0920) else { continue };
                    let mut l = Leg::new(ci, 1.0, St::Trigger { lv: p0 * (1.0 + rise), up: true, from: T0920, until: g("until").round() as usize });
                    (l.stop, l.target, l.hold, l.exit_why, l.prev) = (sl, sl * g("rr"), Some(g("hold").round() as usize), "time exit", p0);
                    legs.push(l);
                }
                crate::optw1::walk(&d, &mut legs, T0920, &mut |_, _| {});
                day_pos(x, &d, i, &mut legs, &mut out);
            }
            "as51_opt_theta" => {
                let (sl, prem, max_trades) = (g("sl") / 100.0, g("prem"), g("max_trades").round() as usize);
                let pick = |put: bool, m: usize| -> Option<usize> {
                    if prem > 0.0 { d.nearest_premium(put, m, prem, false) } else { d.atm(m).and_then(|k| d.ci(k, put)) }
                };
                let mut legs = Vec::new();
                for put in [false, true] {
                    if let Some(ci) = pick(put, T0930) { let mut l = short(ci, St::Timed { at: T0930, until: T1515 - 1 }); l.stop = sl; legs.push(l); }
                }
                let mut trades = 1usize;
                let mut handled: Vec<bool> = vec![false; legs.len()];
                crate::optw1::walk(&d, &mut legs, T0930, &mut |m, ls| {
                    handled.resize(ls.len(), false);
                    let n = ls.len();
                    for j in 0..n {
                        if handled[j] { continue; }
                        if ls[j].active() { continue; }
                        handled[j] = true;
                        if matches!(&ls[j].st, St::Done { why, .. } if why == "stop") && trades < max_trades && m + 1 < T1515 {
                            let put = d.k[ls[j].ci].1;
                            if let Some(ci) = pick(put, m + 1) {
                                let mut l = short(ci, St::Timed { at: m + 1, until: T1515 - 1 });
                                l.stop = sl;
                                ls.push(l);
                                trades += 1;
                            }
                        }
                    }
                });
                day_pos(x, &d, i, &mut legs, &mut out);
            }
            "as51_opt_btst" => {
                let (rise, sl, itm) = (g("rise") / 100.0, g("sl") / 100.0, g("itm").round() as usize);
                let today = crate::options::ymd(date);
                let pick = (d.expiry != today).then(|| d.atm(T1100)).flatten().and_then(|k0| {
                    let calls = d.strikes(false);
                    let j = calls.iter().position(|c| c.0 == k0)?;
                    j.checked_sub(itm).map(|j| calls[j].1)
                });
                let Some(ci) = pick else { i += 1; continue };
                let Some(p0) = d.price_at(ci, T1100) else { i += 1; continue };
                let mut l = Leg::new(ci, 1.0, St::Trigger { lv: p0 * (1.0 + rise), up: true, from: T1100, until: T1529 });
                (l.stop, l.prev) = (sl, p0);
                let mut legs = vec![l];
                crate::optw1::walk(&d, &mut legs, T1100, &mut |_, _| {});
                let mut l = legs.pop().unwrap();
                let Some(e) = l.fill else { i += 1; continue };
                if let St::Done { px, m, why } = &l.st {
                    if let Some(p) = leg_pos(x, &d, i, &l, e, (i, *px, *m, why.clone()), &[], false) { out.push(p); }
                    i += 1;
                    continue;
                }
                // still open after the close: the stop works the next morning; out at the open of 9:30
                let mark = d.last_close(l.ci).unwrap_or(e.0);
                let (strike, put, _) = d.k[l.ci];
                let next = (i < to).then(|| st.day(&x.r[i + 1].date)).flatten();
                let Some(n) = next.filter(|n| n.expiry == d.expiry) else {
                    let open = i == to;
                    let why = if open { "open".to_string() } else { "no price on the next trading day: last traded price".to_string() };
                    if let Some(p) = leg_pos(x, &d, i, &l, e, (i, mark, crate::optw1::MINUTES - 1, why), &[], open) { out.push(p); }
                    i += 1;
                    continue;
                };
                let Some(nci) = n.ci(strike, put) else { i += 1; continue };
                l.carry();
                (l.ci, l.exit_m, l.exit_why) = (nci, T0930, "9:30 next day");
                let mut legs = vec![l];
                crate::optw1::walk(&n, &mut legs, 0, &mut |_, _| {});
                let mut l = legs.pop().unwrap();
                if matches!(l.st, St::Open { .. }) { let px = n.last_close(nci).unwrap_or(mark); l.close(px, crate::optw1::MINUTES - 1, "no trade at or after 9:30: last traded price"); }
                if let St::Done { px, m, why } = &l.st {
                    let mut l0 = l.clone();
                    l0.ci = ci; // the contract's index on the entry day
                    let lp = leg_pos(x, &d, i, &l0, e, (i + 1, *px, *m, why.clone()), &[mark], false);
                    if let Some(p) = lp { out.push(p); }
                }
                i += 1; // the next day's 11:00 order does not overlap: this leg is out by 9:30
                continue;
            }
            "as51_opt_straddle" => {
                // 6.13 (R4 #294 (4), #300): Thursday 15:29, buy the ATM call and put of the next weekly expiry; 50% stops; when one
                // leg stops, the other's stop trails at half its price (then half of each new high); out at 15:29 on the 2nd trading
                // day of the store (its dates are the trading days); a leg still held on the contract's expiry day settles there at
                // intrinsic value on the NIFTY close.
                let today = crate::options::ymd(date);
                let dn = crate::options::day_no(today);
                if dn - crate::options::week_of(today) != 3 { i += 1; continue; } // Thursdays only
                let e = if d.expiry > today { d } else {
                    match next_store.and_then(|s| s.has(date).then(|| s.day(date)).flatten()) {
                        Some(n) => n,
                        None => { notes.push((i, format!("{date}: expiry Thursday without the next expiry's prices in NIFTY_50.w1n, not traded"))); i += 1; continue; }
                    }
                };
                let (sl, trail, hold) = (g("sl") / 100.0, g("trail") / 100.0, g("hold").round().max(1.0) as usize);
                let Some(k) = e.atm(T1529) else { i += 1; continue };
                let (Some(cc), Some(pc)) = (e.ci(k, false), e.ci(k, true)) else { i += 1; continue };
                let mk = |ci: usize| { let mut l = Leg::new(ci, 1.0, St::Timed { at: T1529, until: T1529 }); l.stop = sl; l };
                let mut legs = vec![mk(cc), mk(pc)];
                let mut started = false;
                let mut each = |_m: usize, ls: &mut Vec<Leg>| {
                    if started { return; }
                    let stopped: Vec<bool> = ls.iter().map(|l| matches!(&l.st, St::Done { why, .. } if why == "stop")).collect();
                    for j in 0..ls.len() {
                        if stopped[1 - j] && matches!(ls[j].st, St::Open { .. }) { ls[j].start_trail(trail); started = true; }
                    }
                };
                crate::optw1::walk(&e, &mut legs, T1529, &mut each);
                // the straddle is both legs: a day on which either leg has no trade at 15:29 is not traded (#300 review)
                if legs.iter().any(|l| l.fill.is_none()) {
                    if legs.iter().any(|l| l.fill.is_some()) { notes.push((i, format!("{date}: only one leg of the {k} straddle traded at 15:29, not traded"))); }
                    i += 1;
                    continue;
                }
                let keys = [(k, false, cc), (k, true, pc)];
                let mut marks: Vec<Vec<f64>> = vec![vec![], vec![]];
                let mut exit_j: Vec<Option<usize>> = legs.iter().map(|l| (!matches!(l.st, St::Open { .. })).then_some(i)).collect();
                for (j, l) in legs.iter().enumerate() { if exit_j[j].is_none() { marks[j].push(e.last_close(l.ci).unwrap_or(l.fill.map_or(0.0, |f| f.0))); } }
                let mut last = i;
                // the holding days are the store's next trading days, mapped to the vault's daily bars
                for (step, nd) in st.next_dates(date, hold).iter().enumerate() {
                    if !legs.iter().any(|l| matches!(l.st, St::Open { .. })) { break; }
                    let jd = x.r.partition_point(|r| r.date.as_str() < nd.as_str());
                    if jd > to { break; } // past the run's window: open
                    if jd >= x.r.len() || x.r[jd].date != *nd {
                        for (j, l) in legs.iter_mut().enumerate() {
                            if !matches!(l.st, St::Open { .. }) { continue; }
                            let p = marks[j].pop().unwrap();
                            l.close(p, crate::optw1::MINUTES - 1, "the next trading day has no NIFTY 50 daily bar: last traded price");
                            exit_j[j] = Some(last);
                        }
                        break;
                    }
                    let Some(n) = st.day(nd) else {
                        return Err(format!("calculation_unavailable: the weekly option minutes of {nd} could not be read (malformed store block); no result is published"));
                    };
                    // daily bars of the vault between two store days (no option minutes) carry the last value (one mark per bar)
                    for j in 0..2 { if exit_j[j].is_none() && legs[j].fill.is_some() { while marks[j].len() < jd - i { let v = *marks[j].last().unwrap(); marks[j].push(v); } } }
                    let final_day = step + 1 == hold;
                    let expiry_day = crate::options::ymd(nd) == e.expiry;
                    for (j, l) in legs.iter_mut().enumerate() {
                        if !matches!(l.st, St::Open { .. }) { continue; }
                        match (n.expiry == e.expiry).then(|| n.ci(keys[j].0, keys[j].1)).flatten() {
                            Some(ci) => { l.carry(); l.ci = ci; if final_day { (l.exit_m, l.exit_why) = (T1529, "15:29 exit, 2nd trading day"); } }
                            None => { let p = marks[j].pop().unwrap(); l.close(p, crate::optw1::MINUTES - 1, "contract not in the next day's data: last traded price"); exit_j[j] = Some(last); }
                        }
                    }
                    crate::optw1::walk(&n, &mut legs, 0, &mut each);
                    for (j, l) in legs.iter_mut().enumerate() {
                        if exit_j[j].is_some() || l.fill.is_none() { continue; }
                        if matches!(l.st, St::Open { .. }) {
                            if final_day {
                                let prev = *marks[j].last().unwrap();
                                l.close(n.last_close(l.ci).unwrap_or(prev), crate::optw1::MINUTES - 1, "no trade at or after the exit time: last traded price");
                                exit_j[j] = Some(jd);
                            } else if expiry_day {
                                // held into its expiry day: cash-settled at intrinsic value on the NIFTY close (no later price exists)
                                let s = n.last_close(crate::optw1::SPOT).unwrap_or(f64::NAN);
                                let v = if keys[j].1 { (keys[j].0 - s).max(0.0) } else { (s - keys[j].0).max(0.0) };
                                l.close(v, crate::optw1::MINUTES - 1, "expiry day: final settlement at intrinsic value on the NIFTY close");
                                exit_j[j] = Some(jd);
                            } else {
                                let prev = *marks[j].last().unwrap();
                                marks[j].push(n.last_close(l.ci).unwrap_or(prev));
                            }
                        } else { exit_j[j] = Some(jd); }
                    }
                    last = jd;
                    if expiry_day { break; }
                }
                for (j, l) in legs.iter().enumerate() {
                    let Some(f) = l.fill else { continue };
                    let mut l0 = l.clone();
                    l0.ci = keys[j].2;
                    let p = match (&l.st, exit_j[j]) {
                        (St::Done { px, m, why }, Some(xj)) => leg_pos(x, &e, i, &l0, f, (xj, *px, *m, why.clone()), &marks[j][..xj - i], false),
                        _ => { // still open at the end of the window or of the data
                            let mk = marks[j].pop().unwrap_or(f.0);
                            leg_pos(x, &e, i, &l0, f, (last, mk, crate::optw1::MINUTES - 1, "open".into()), &marks[j], true)
                        }
                    };
                    if let Some(p) = p { out.push(p); }
                }
                i += 1;
                continue;
            }
            "as51_opt_st_sell" => {
                // 6.4 (lane F4): each evaluated 5-minute bar's colour acts at its close (the open of the next minute); the day's first
                // evaluated bar and every colour change open a sold ATM leg of that colour (green: put, red: call), which a later
                // change buys back; no new leg after `last`; the day's last leg leaves at its last traded price of the day
                let last = g("last").round() as usize;
                let mut dec: Vec<(usize, bool)> = Vec::new();
                let mut cur: Option<i8> = None;
                for &(j, dir) in &dirs {
                    let Some(dir) = dir else { continue };
                    if cur != Some(dir) { dec.push((5 * (j + 1), dir == -1)); cur = Some(dir); }
                }
                let mut legs = Vec::new();
                for (k, &(a, put)) in dec.iter().enumerate() {
                    if a > last { break; }
                    let nxt = dec.get(k + 1).map(|z| z.0).filter(|&n| n < MINUTES);
                    let Some(ci) = d.atm(a).and_then(|kk| d.ci(kk, put)) else { continue };
                    let mut l = short(ci, St::Timed { at: a, until: nxt.map_or(T1529, |n| n - 1) });
                    (l.exit_m, l.exit_why) = match nxt { Some(n) => (n, "Supertrend colour change"), None => (MINUTES, "") };
                    legs.push(l);
                }
                crate::optw1::walk(&d, &mut legs, 0, &mut |_, _| {});
                for l in legs.iter_mut() {
                    if matches!(l.st, St::Open { .. }) && l.exit_m >= MINUTES {
                        let lm = (0..MINUTES).rev().find(|&m| d.bars[l.ci][m].is_some()).unwrap_or(T1529);
                        l.close(d.bars[l.ci][lm].map_or(f64::NAN, |z| z.c), lm, "the day's last trade (the book's table exits at the 15:25 bar)");
                    }
                }
                day_pos(x, &d, i, &mut legs, &mut out);
            }
            "as51_opt_vwap" => {
                // 6.5 (lane F4): straddle of the 9:15 ATM strike; 5-minute closes vs the straddle's VWAP (#297 reading: price = the two
                // legs' closes added, volume = their volumes added, from 9:15)
                let (below, maxt, eod) = (g("below") / 100.0, g("max_trades").round() as usize, g("eod").round() as usize);
                let ks = d.atm(0).map(|k| (d.ci(k, false), d.ci(k, true)));
                let Some((Some(cc), Some(pc))) = ks else { i += 1; continue };
                let close_by = |ci: usize, e: usize| (0..=e).rev().find_map(|m| d.bars[ci][m]).map(|z| z.c);
                let (mut pv, mut vv) = (0.0, 0.0);
                let mut trades: Vec<(usize, usize, &'static str)> = Vec::new();
                let mut open: Option<usize> = None;
                for j in 0..MINUTES / 5 {
                    let (e, a) = (5 * j + 4, 5 * j + 5);
                    let (Some(c1), Some(c2)) = (close_by(cc, e), close_by(pc, e)) else { continue };
                    let px = c1 + c2;
                    let v: f64 = (5 * j..=e).map(|m| d.vol[cc][m] + d.vol[pc][m]).sum();
                    pv += px * v;
                    vv += v;
                    if vv <= 0.0 || a >= eod { continue; }
                    let w = pv / vv;
                    match open {
                        Some(t0) => if px >= w { trades.push((t0, a, "premium back at the VWAP")); open = None; },
                        None => if trades.len() < maxt && a <= T1500 && px <= (1.0 - below) * w { open = Some(a); },
                    }
                }
                if let Some(t0) = open { trades.push((t0, eod, if eod == T1515 { "15:15 exit" } else { "time exit" })); }
                let mut legs = Vec::new();
                for &(t0, t1, why) in &trades {
                    for ci in [cc, pc] {
                        let mut l = short(ci, St::Timed { at: t0, until: t1 - 1 });
                        (l.exit_m, l.exit_why) = (t1, why);
                        legs.push(l);
                    }
                }
                crate::optw1::walk(&d, &mut legs, 0, &mut |_, _| {});
                day_pos(x, &d, i, &mut legs, &mut out);
            }
            "as51_opt_mom_buy" => {
                // 6.6 (lane F4): the 9:30 ATM call and put, each bought when it rises `rise` over its 9:30 price
                let (rise, sl, tgt) = (g("rise") / 100.0, g("sl") / 100.0, g("tgt") / 100.0);
                let mut legs = Vec::new();
                for put in [false, true] {
                    let Some(ci) = d.atm(T0930).and_then(|k| d.ci(k, put)) else { continue };
                    let Some(p0) = d.price_at(ci, T0930) else { continue };
                    let mut l = Leg::new(ci, 1.0, St::Trigger { lv: p0 * (1.0 + rise), up: true, from: T0930, until: g("until").round() as usize });
                    (l.stop, l.target, l.exit_m, l.exit_why, l.prev) = (sl, tgt, T1515, "15:15 exit", p0);
                    legs.push(l);
                }
                crate::optw1::walk(&d, &mut legs, T0930, &mut |_, _| {});
                day_pos(x, &d, i, &mut legs, &mut out);
            }
            "as51_opt_expiry" => {
                // 6.7 (lane F4): two trading days before the expiry (trading days = the vault's dates with a full session in the
                // store), at 10:00 sell the call `otm` above and the put `otm` below the spot rounded to 100; stop 1 + sl; out 9:55
                // on the expiry day
                let today = crate::options::ymd(date);
                if d.expiry <= today { i += 1; continue; }
                let mut nxt: Vec<(usize, crate::optw1::Day)> = Vec::new();
                let mut j = i + 1;
                while j < x.r.len() && crate::options::ymd(&x.r[j].date) <= d.expiry && nxt.len() < 3 {
                    let dj = x.r[j].date.as_str();
                    if st.has(dj) {
                        let n = st.day(dj).ok_or_else(|| format!("calculation_unavailable: the weekly option minutes of {dj} could not be read (malformed store block); no result is published"))?;
                        if !n.special() { nxt.push((j, n)); }
                    }
                    j += 1;
                }
                if nxt.len() != 2 || crate::options::ymd(&x.r[nxt[1].0].date) != d.expiry || nxt[1].1.expiry != d.expiry { i += 1; continue; }
                let (sl, otm) = (g("sl") / 100.0, g("otm"));
                let Some(s) = d.price_at(SPOT, T1000) else { i += 1; continue };
                let r0 = (s / 100.0 + 0.5).floor() * 100.0;
                let mut keys = Vec::new();
                let mut legs = Vec::new();
                for (k, put) in [(r0 + otm, false), (r0 - otm, true)] {
                    let Some(ci) = d.ci(k, put) else { continue };
                    let mut l = short(ci, St::Timed { at: T1000, until: T1529 });
                    (l.stop, l.exit_m, l.exit_why) = (sl, MINUTES, "");
                    legs.push(l);
                    keys.push((k, put, ci));
                }
                crate::optw1::walk(&d, &mut legs, T1000, &mut |_, _| {});
                let mut marks: Vec<Vec<f64>> = vec![Vec::new(); legs.len()];
                let mut cur: Vec<f64> = legs.iter().map(|l| d.last_close(l.ci).unwrap_or(l.fill.map_or(0.0, |f| f.0))).collect();
                let mut exit_j: Vec<Option<usize>> = legs.iter().map(|l| (!matches!(l.st, St::Open { .. })).then_some(i)).collect();
                let mut last = i;
                for (step, (jd, n)) in nxt.iter().enumerate() {
                    if *jd > to || !legs.iter().any(|l| matches!(l.st, St::Open { .. })) { break; }
                    for (q, l) in legs.iter_mut().enumerate() {
                        if !matches!(l.st, St::Open { .. }) { continue; }
                        match n.ci(keys[q].0, keys[q].1) {
                            Some(ci) => {
                                while marks[q].len() < jd - i { marks[q].push(cur[q]); } // the close of the last day, carried over vault days without minutes
                                l.carry();
                                l.ci = ci;
                                if step == 1 { (l.exit_m, l.exit_why) = (T0955, "9:55 exit, expiry day"); }
                            }
                            None => { l.close(cur[q], MINUTES - 1, "contract not in the next day's data: last traded price"); exit_j[q] = Some(last); }
                        }
                    }
                    crate::optw1::walk(n, &mut legs, 0, &mut |_, _| {});
                    for (q, l) in legs.iter_mut().enumerate() {
                        if exit_j[q].is_some() || l.fill.is_none() { continue; }
                        if let Some(c) = n.last_close(l.ci) { cur[q] = c; }
                        if matches!(l.st, St::Open { .. }) {
                            if step == 1 { l.close(cur[q], MINUTES - 1, "no trade at or after the exit time: last traded price"); exit_j[q] = Some(*jd); }
                        } else { exit_j[q] = Some(*jd); }
                    }
                    last = *jd;
                }
                for (q, l) in legs.iter().enumerate() {
                    let Some(f) = l.fill else { continue };
                    let mut l0 = l.clone();
                    l0.ci = keys[q].2;
                    let pos = match (&l.st, exit_j[q]) {
                        (St::Done { px, m, why }, Some(xj)) => leg_pos(x, &d, i, &l0, f, (xj, *px, *m, why.clone()), &marks[q][..xj - i], false),
                        _ => { // still open at the end of the window
                            while marks[q].len() < last - i { marks[q].push(cur[q]); }
                            leg_pos(x, &d, i, &l0, f, (last, cur[q], MINUTES - 1, "open".into()), &marks[q][..last - i], true)
                        }
                    };
                    if let Some(z) = pos { out.push(z); }
                }
            }
            _ => { // as51_opt_3pm
                let (drop, sl, prem) = (g("drop") / 100.0, g("sl") / 100.0, g("prem"));
                let mut legs = Vec::new();
                let atm = d.atm(T1500);
                for put in [false, true] {
                    let a = atm.and_then(|k| d.ci(k, put));
                    let b = d.nearest_premium(put, T1500, prem, false);
                    for (ci, stop) in [(a, f64::NAN), (b, sl)] {
                        let Some(ci) = ci else { continue };
                        let Some(p0) = d.price_at(ci, T1500) else { continue };
                        let mut l = Leg::new(ci, -1.0, St::Trigger { lv: p0 * (1.0 - drop), up: false, from: T1500, until: T1529 - 1 });
                        (l.stop, l.exit_m, l.exit_why, l.prev) = (stop, T1529, "15:29 exit", p0);
                        legs.push(l);
                    }
                }
                crate::optw1::walk(&d, &mut legs, T1500, &mut |_, _| {});
                day_pos(x, &d, i, &mut legs, &mut out);
            }
        }
        i += 1;
    }
    // each note goes on the first position entered after it (the days a rule skipped are never silent)
    out.sort_by_key(|q: &crate::options::Pos| q.entry_i);
    let mut k = 0;
    for (ni, note) in notes {
        while k < out.len() && out[k].entry_i <= ni { k += 1; }
        if k < out.len() { out[k].reason.push_str(&format!(" · note: {note}")); }
    }
    Ok(out)
}

// Instrument admission: the five daily rules (lane K70, 8 Nov 2026). The minute rules: intraday_ep::K70.
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "as51ts" && reviewed(def.id),
    label: "Singhal's examples are NIFTY 50, BANKNIFTY and NSE stock charts with no test results. Runs on daily indices, ETFs and stocks, US and India, are a declared application.",
    basis: "Aseem Singhal, 51 Trading Strategies (ZebraLearn 2023), research-share screenshots read in full (authors/rnew/ocr); independent check authors/titles-1030/K70w/k70_ref.py",
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    fn rows(n: usize) -> Vec<Row> {
        (0..n).map(|i| { let f = i as f64; let c = 100. + 12. * (f / 23.).sin() + 5. * (f / 6.1).sin() + 0.03 * f;
            Row { ticker: "T".into(), date: format!("2000-{:05}", i), open: c - 0.8 * (f / 2.3).sin(), high: c + 1.2 + 0.3 * (f / 3.).cos(), low: c - 1.1, close: c, volume: None } }).collect()
    }
    #[test]
    fn all_51_cards_and_daily_rules_run() {
        let cat = catalogue();
        assert_eq!(cat.len(), 51);
        assert_eq!(cat.iter().filter(|d| d.status == Status::Live).count(), DAILY.len() + INTRA.len() + OPT_IDS.len());
        for d in &cat { assert!(family_of(d.id).is_some() && (d.status == Status::Live) == params(d.id).is_some(), "{}", d.id); }
        let r = rows(900);
        let x = Ctx::new(&r, &r, &[]);
        for id in DAILY {
            let mut p = Params::new();
            if *id == "as51_donchian_pb" { p.insert("pb_max".into(), 50.); } // the smooth test series rarely pulls back only 3-4 bars
            let t = run(id, &x, 0, r.len() - 1, &p).unwrap();
            assert!(t.len() >= 3, "{id} {}", t.len());
            for z in &t { assert!(z.exit_i >= z.legs[0].i && (z.exit_i > z.legs[0].i || z.open)); }
        }
    }
    #[test]
    fn k71_mw_shape_and_stop_entry() {
        let r = rows(900);
        let x = Ctx::new(&r, &r, &[]);
        let rs = rsi(&x.c, 14);
        // #271(1): every M / W entry is an even-numbered re-cross (2nd, 4th, ...) of 30 (long) or 70 (short)
        let ord = |t: usize, lv: f64, up: bool| (1..=t).filter(|&j| if up { rs[j - 1] <= lv && rs[j] > lv } else { rs[j - 1] >= lv && rs[j] < lv }).count();
        let mw = run("as51_rsi_mw", &x, 0, r.len() - 1, &Params::new()).unwrap();
        let mut p0 = Params::new();
        p0.insert("mw".into(), 0.);
        assert!(!mw.is_empty() && mw.len() < run("as51_rsi_mw", &x, 0, r.len() - 1, &p0).unwrap().len());
        for z in &mw { let t = z.legs[0].i; assert_eq!(ord(t, if z.dir > 0. { 30. } else { 70. }, z.dir > 0.) % 2, 0, "entry {t}"); }
        // #271(3): a stop entry on the bar after a green (long) / red (short) signal bar, at its high / low or the open beyond it
        let st = run("as51_st_rsi", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(!st.is_empty());
        for z in &st {
            let (t, px) = (z.legs[0].i, z.legs[0].px);
            if z.dir > 0. { assert!(x.c[t - 1] > x.o[t - 1] && px == x.o[t].max(x.h[t - 1]) && x.h[t] >= x.h[t - 1]); }
            else { assert!(x.c[t - 1] < x.o[t - 1] && px == x.o[t].min(x.l[t - 1]) && x.l[t] <= x.l[t - 1]); }
        }
    }
    #[test]
    fn k72_order_expiry_and_initial_stop() {
        // #273(1): bar 0's next-bar order (buy stop 105) is not reached on bar 1; bar 1's own signal (buy stop 100.7) is still
        // placed at its close and fills on bar 2 (before K72 the stale order blocked it and nothing traded)
        let r = rows(900);
        let x = Ctx::new(&r, &r, &[]);
        let bar = |k: u16, o: f64, h: f64, l: f64, c: f64| B { i: 1, s: 570 + 5 * k, e: 575 + 5 * k, open: 570, close: 960, o, h, l, c,
            m: vec![Bar1 { m: 570 + 5 * k, o, h, l, c }], first: k == 0, last: k == 3 };
        let v = vec![bar(0, 100., 101., 99., 100.5), bar(1, 100.5, 100.8, 100., 100.6), bar(2, 100.6, 102., 100.5, 101.8), bar(3, 101.8, 102., 101., 101.5)];
        let place = |k: usize| match k {
            0 => Some(Pend { d: 1., lv: 105., stop: f64::NAN, rr: f64::NAN, until: 1 }),
            1 => Some(Pend { d: 1., lv: 100.7, stop: f64::NAN, rr: f64::NAN, until: 2 }),
            _ => None };
        let t = drive(&x, &v, &place, &|_, q| q.stop, &|_, _| None, &|_| false, 4);
        assert_eq!((t.len(), t[0].legs[0].px, t[0].exit_px), (1, 100.7, 101.5));
        // #273(2): an initial-stop exit is at the signal candle's Supertrend line (or the open beyond it); off = none
        // a tight Supertrend (factor 0.5) so that the smooth test series reaches the initial stop
        let mut p1 = Params::new();
        p1.insert("fac".into(), 0.5);
        let (line, _) = supertrend(&x.h, &x.l, &x.c, 10, 0.5);
        let st = run("as51_st_rsi", &x, 0, r.len() - 1, &p1).unwrap();
        assert!(st.iter().any(|z| z.reason == "initial stop" && z.dir > 0.) && st.iter().any(|z| z.reason == "initial stop" && z.dir < 0.));
        for z in st.iter().filter(|z| z.reason == "initial stop") {
            let (e, sl) = (z.legs[0].i, line[z.legs[0].i - 1]);
            assert!(z.exit_px == sl || (z.exit_i > e && z.exit_px == x.o[z.exit_i]), "{e}");
        }
        let mut p0 = p1.clone();
        p0.insert("init_stop".into(), 0.);
        assert!(run("as51_st_rsi", &x, 0, r.len() - 1, &p0).unwrap().iter().all(|z| z.reason != "initial stop"));
    }
    #[test]
    fn supertrend_and_sar_by_hand() {
        // a straight rise: the Supertrend turns up once the ATR exists and stays up; the SAR stays below the lows
        let h: Vec<f64> = (0..40).map(|i| 101. + i as f64).collect();
        let l: Vec<f64> = (0..40).map(|i| 99. + i as f64).collect();
        let c: Vec<f64> = (0..40).map(|i| 100.5 + i as f64).collect();
        let (st, dir) = supertrend(&h, &l, &c, 7, 3.);
        assert_eq!(dir[5], 1);
        assert!(dir[25..].iter().all(|&d| d == -1) && st[30] < l[30]);
        let (s, below, nxt) = psar(&h, &l, &c, 0.02, 0.02, 0.2);
        assert!(below[1..].iter().all(|&b| b) && (2..40).all(|t| s[t] <= l[t - 1]));
        // bar 1: res = l[0] = 99, first trend bar; then res = 99 + 0.02 x (h[1] - 99) = 99.06 clamped by l[0] = 99 -> 99
        assert_eq!(s[1], 99.);
        assert!((nxt[1] - (99f64 + 0.02 * (102. - 99.)).min(99.)).abs() < 1e-12 && (nxt[10] - s[11]).abs() < 1e-12);
        // ema seeded with the simple average
        let e = ema(&[1., 2., 3., 4.], 3);
        assert!(e[1].is_nan() && e[2] == 2. && e[3] == 0.5 * 4. + 0.5 * 2.);
    }
    #[test]
    fn minute_walk_entry_stop_target() {
        // a buy stop at 101 fills, then the target 101 + 2 x (101 - 100) = 103 is reached before the stop
        let mut s = W { pend: Some(Pend { d: 1., lv: 101., stop: 100., rr: 2., until: 9 }), ..Default::default() };
        let m = [Bar1 { m: 0, o: 100.5, h: 101.5, l: 100.4, c: 101.2 }, Bar1 { m: 1, o: 101.2, h: 103.5, l: 101.1, c: 103. }];
        let (amb, w) = ep6::walk_minutes(&m, 100.5, s.clone(), &|st: &mut W, g: &[Seg]| walk(st, g, 7));
        assert!(!amb && w.pos.is_none() && w.done.len() == 1);
        let (q, px, why) = w.done[0];
        assert_eq!((q.ep, q.tgt, px, why, q.ei), (101., 103., 103., "target", 7));
        // one minute through both the entry and the stop (opened through neither): ambiguous
        s.pend = Some(Pend { d: 1., lv: 101., stop: 100.8, rr: f64::NAN, until: 9 });
        let (amb, _) = ep6::walk_minutes(&[Bar1 { m: 0, o: 100.9, h: 101.5, l: 100.5, c: 101. }], 100.9, s, &|st: &mut W, g: &[Seg]| walk(st, g, 7));
        assert!(amb);
    }

    /// K70_OUT=<csv> VAULT_US=.. VAULT_IDX=.. VAULT_M1_DIR=.. K70_STOCKS=<name=path,...> cargo test --release k70_dump -- --ignored;
    /// then authors/titles-1030/K70w/cmp_dump.py.
    #[test]
    #[ignore]
    fn k70_dump() {
        use std::io::Write;
        let mut f = std::io::BufWriter::new(std::fs::File::create(std::env::var("K70_OUT").unwrap()).unwrap());
        let mut sets: Vec<(String, Vec<Row>)> = vec![];
        for (file, t) in [("VAULT_US", "SPY"), ("VAULT_US", "SPX"), ("VAULT_US", "QQQ"), ("VAULT_US", "DIA"), ("VAULT_IDX", "NIFTY 50"), ("VAULT_IDX", "NIFTYBANK")] {
            sets.push((t.into(), crate::load_csv(&std::env::var(file).unwrap()).unwrap().remove(t).unwrap()));
        }
        for kv in std::env::var("K70_STOCKS").unwrap_or_default().split(',').filter(|s| !s.is_empty()) {
            let (name, path) = kv.split_once('=').unwrap();
            let text = std::fs::read_to_string(path).unwrap();
            let r: Vec<Row> = text.lines().skip(1).map(|l| { let v: Vec<&str> = l.split(',').collect(); let g = |k: usize| v[k].parse::<f64>().unwrap();
                Row { ticker: name.into(), date: v[0].into(), open: g(1), high: g(2), low: g(3), close: g(4), volume: None } }).collect();
            sets.push((name.into(), r));
        }
        for (t, rows) in &sets {
            let mut x = Ctx::new(rows, rows, &[]);
            let minute = ["SPY", "SPX", "NIFTY 50", "NIFTYBANK"].contains(&t.as_str());
            if minute { x.minutes = Some(Box::leak(Box::new(crate::intraday::Minutes::load(&std::env::var("VAULT_M1_DIR").unwrap(), t).unwrap()))); }
            let opts: &[(&str, &[(&str, f64)])] = &[("as51_ichimoku", &[]), ("as51_fractal_sma", &[]), ("as51_donchian_pb", &[]), ("as51_donchian_pb", &[("pb_max", 10.)]),
                ("as51_st_rsi", &[]), ("as51_st_rsi", &[("stop_entry", 0.)]), ("as51_st_rsi", &[("init_stop", 0.)]), ("as51_rsi_mw", &[]), ("as51_rsi_mw", &[("mw", 0.)]), ("as51_st_pivot", &[]), ("as51_double_rsi", &[]), ("as51_ha_psar", &[]), ("as51_ema_scalp", &[]), ("as51_ema_scalp", &[("ema", 7.)])];
            for (id, kv) in opts {
                let intra = INTRA.contains(id);
                if intra && !minute { continue; }
                let from = if intra { rows.partition_point(|r| r.date.as_str() < x.minutes.unwrap().first()) } else { 0 };
                let mut p = Params::new();
                for (k, v) in kv.iter() { p.insert(k.to_string(), *v); }
                let tag = kv.iter().map(|(k, v)| format!("{k}={v}")).collect::<Vec<_>>().join(";");
                for z in run(id, &x, from, rows.len() - 1, &p).unwrap() {
                    writeln!(f, "{id}{}{tag},{t},{},{},{},{},{},\"{}\",{}", if tag.is_empty() { "" } else { "|" }, z.dir, rows[z.legs[0].i].date, z.legs[0].px, rows[z.exit_i].date, z.exit_px, z.reason, z.open).unwrap();
                }
            }
        }
    }

    /// Lane F2: F2_OUT=<csv> VAULT_IDX=<idx_data.csv> F2_W1=<weekly1m dir> cargo test --release f2_dump -- --ignored;
    /// then authors/titles-1030/F2w/f2_cmp.py.
    #[test]
    #[ignore]
    fn f2_dump() {
        use std::io::Write;
        let _ = crate::optw1::DIR.set(std::env::var("F2_W1").unwrap());
        let mut f = std::io::BufWriter::new(std::fs::File::create(std::env::var("F2_OUT").unwrap()).unwrap());
        let rows = crate::load_csv(&std::env::var("VAULT_IDX").unwrap()).unwrap().remove("NIFTY 50").unwrap();
        let x = Ctx::new(&rows, &rows, &[]);
        let st = crate::optw1::store("NIFTY 50").unwrap();
        let from = rows.partition_point(|r| r.date.as_str() < st.first());
        let to = rows.partition_point(|r| r.date.as_str() <= st.last()) - 1;
        let opts: &[(&str, &[(&str, f64)])] = &[("as51_opt_multi_sell", &[]), ("as51_opt_multi_sell", &[("otm", 1.)]), ("as51_opt_oi_buy", &[]), ("as51_opt_theta", &[]), ("as51_opt_theta", &[("prem", 125.)]),
            ("as51_opt_btst", &[]), ("as51_opt_3pm", &[]), ("as51_opt_straddle", &[])];
        for (id, kv) in opts {
            let mut p = Params::new();
            for (k, v) in kv.iter() { p.insert(k.to_string(), *v); }
            let tag = kv.iter().map(|(k, v)| format!("{k}={v}")).collect::<Vec<_>>().join(";");
            for z in option_positions(id, &x, from, to, &p).unwrap().unwrap() {
                writeln!(f, "{id}{}{tag},{},{},{},{},{},\"{}\",{}", if tag.is_empty() { "" } else { "|" }, rows[z.entry_i].date, rows[z.exit_i].date, z.s0, z.dir,
                    z.pnl.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(";"), z.reason, z.open).unwrap();
            }
        }
    }

    /// Lane F4: F4_OUT=<csv> VAULT_IDX=<idx_data.csv> F2_W1=<weekly1m dir> cargo test --release f4_dump -- --ignored;
    /// then authors/titles-1030/F4w/f4_cmp.py.
    #[test]
    #[ignore]
    fn f4_dump() {
        use std::io::Write;
        let _ = crate::optw1::DIR.set(std::env::var("F2_W1").unwrap());
        let mut f = std::io::BufWriter::new(std::fs::File::create(std::env::var("F4_OUT").unwrap()).unwrap());
        let rows = crate::load_csv(&std::env::var("VAULT_IDX").unwrap()).unwrap().remove("NIFTYBANK").unwrap();
        let x = Ctx::new(&rows, &rows, &[]);
        let st = crate::optw1::store("NIFTYBANK").unwrap();
        let from = rows.partition_point(|r| r.date.as_str() < st.first());
        let to = rows.partition_point(|r| r.date.as_str() <= st.last()) - 1;
        let opts: &[(&str, &[(&str, f64)])] = &[("as51_opt_st_sell", &[]), ("as51_opt_st_sell", &[("atr", 10.)]), ("as51_opt_vwap", &[]), ("as51_opt_vwap", &[("eod", 374.)]),
            ("as51_opt_mom_buy", &[]), ("as51_opt_expiry", &[])];
        for (id, kv) in opts {
            let mut p = Params::new();
            for (k, v) in kv.iter() { p.insert(k.to_string(), *v); }
            let tag = kv.iter().map(|(k, v)| format!("{k}={v}")).collect::<Vec<_>>().join(";");
            for z in option_positions(id, &x, from, to, &p).unwrap().unwrap() {
                writeln!(f, "{id}{}{tag},{},{},{},{},{},\"{}\",{}", if tag.is_empty() { "" } else { "|" }, rows[z.entry_i].date, rows[z.exit_i].date, z.s0, z.dir,
                    z.pnl.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(";"), z.reason, z.open).unwrap();
            }
        }
    }

    #[test]
    fn f4_incremental_supertrend_matches_the_series_version() {
        let n = 300;
        let c: Vec<f64> = (0..n).map(|i| { let f = i as f64; 1000. + 30. * (f / 17.).sin() + 12. * (f / 5.3).cos() + 0.2 * f }).collect();
        let h: Vec<f64> = c.iter().enumerate().map(|(i, v)| v + 2. + (i % 7) as f64).collect();
        let l: Vec<f64> = c.iter().enumerate().map(|(i, v)| v - 1.5 - (i % 5) as f64).collect();
        let (_, dir) = supertrend(&h, &l, &c, 7, 3.);
        let mut s = StInc::new(7, 3.);
        for t in 0..n {
            let got = s.push(h[t], l[t], c[t]);
            if t < 7 { assert_eq!(got, None, "{t}: the direction is only evaluated once the ATR existed on the bar before"); } else { assert_eq!(got, Some(dir[t]), "{t}"); }
        }
        assert!(dir.iter().any(|&d| d == 1) && dir.iter().any(|&d| d == -1));
    }

    /// Lane F4: the BANKNIFTY rules on a hand-made 3-day store (Tue, Wed, Thu expiry): 6.7 entry two trading days before the
    /// expiry, stop at 1.9 x, 9:55 exit; 6.6 trigger, target; the guards (NIFTY rules on NIFTYBANK and the reverse).
    #[test]
    fn f4_banknifty_rules_on_a_small_store() {
        // spot flat at 44,170 (rounds to 44,200: call 44,300, put 44,100); prices in paise, rows a,b,c,d[,v]
        let dir = std::env::temp_dir().join(format!("f4_store_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let day = |date: &str, ce: &[(usize, i64, i64, i64, i64)], pe: &[(usize, i64, i64, i64, i64)]| -> String {
            let mut s = format!("#,{date},2023-06-08,4\nK,1,44200,CE,0\nK,2,44300,CE,0\nK,3,44100,PE,0\nK,4,44200,PE,0\nR,0,0\n4417000,0,0,0\n");
            for _ in 1..375 { s.push_str("0,0,0,0\n"); }
            for (ci, rows) in [(2, ce), (3, pe)] {
                for &(m, o, h, l, c) in rows.iter() { s.push_str(&format!("R,{ci},{m}\n{o},{},{},{},10\n", h - o, o - l, c - o)); }
            }
            // the ATM pair, traded every minute at 300 (the 9:30 ATM for 6.6 is 44,200)
            for ci in [1, 4] { s.push_str(&format!("R,{ci},0\n30000,0,0,0,5\n")); for _ in 1..375 { s.push_str("0,0,0,0,5\n"); } }
            s
        };
        let d1 = day("2023-06-06", &[(45, 10000, 10000, 10000, 10000), (300, 9000, 9000, 9000, 9000)], &[(45, 8000, 8000, 8000, 8000)]);
        // day 2: the put jumps to 160 (stop 152 = 1.9 x 80): filled at the open 160; the call drifts
        let d2 = day("2023-06-07", &[(0, 9500, 9500, 9500, 9500)], &[(10, 16000, 16000, 16000, 16000)]);
        // day 3 (expiry): the call trades at 9:55 at 20
        let d3 = day("2023-06-08", &[(30, 3000, 3000, 3000, 3000), (40, 2000, 2000, 2000, 2000)], &[]);
        let (mut text, mut idx, mut off) = (String::new(), String::from("Date,offset,bytes,expiry\n"), 0);
        for (d, b) in [("2023-06-06", &d1), ("2023-06-07", &d2), ("2023-06-08", &d3)] { idx.push_str(&format!("{d},{off},{},2023-06-08\n", b.len())); off += b.len(); text.push_str(b); }
        std::fs::write(dir.join("NIFTYBANK.w1.csv"), text).unwrap();
        std::fs::write(dir.join("NIFTYBANK.w1idx.csv"), idx).unwrap();
        let st = crate::optw1::Store::load(dir.to_str().unwrap(), "NIFTYBANK").unwrap();
        let mut r = rows(3);
        for (k, d) in ["2023-06-06", "2023-06-07", "2023-06-08"].iter().enumerate() { r[k].date = d.to_string(); r[k].ticker = "NIFTYBANK".into(); }
        let x = Ctx::new(&r, &r, &[]);
        let ps = positions_in("as51_opt_expiry", &x, 0, 2, &Params::new(), &st, None).unwrap();
        assert_eq!(ps.len(), 2, "{:?}", ps.iter().map(|p| &p.reason).collect::<Vec<_>>());
        let call = ps.iter().find(|p| p.reason.contains("44300 CE")).unwrap();
        let put = ps.iter().find(|p| p.reason.contains("44100 PE")).unwrap();
        assert!(call.reason.contains("at 10:00 100.00") && call.reason.contains("2023-06-08 09:55 20.00 (9:55 exit, expiry day)"), "{}", call.reason);
        assert_eq!((call.entry_i, call.exit_i), (0, 2));
        assert_eq!(call.pnl, vec![100.0 - 90.0, 100.0 - 95.0, 100.0 - 20.0]);
        assert!(put.reason.contains("at 10:00 80.00") && put.reason.contains("2023-06-07 09:25 160.00 (stop)"), "{}", put.reason);
        assert_eq!(put.pnl, vec![0.0, 80.0 - 160.0]);
        // 6.7 needs two trading days before the expiry: entering on the Wednesday is not the rule
        assert!(positions_in("as51_opt_expiry", &x, 1, 2, &Params::new(), &st, None).unwrap().is_empty());
        // the rules keep to their own market
        for id in OPT_IDS {
            let bnf = BNF_IDS.contains(id);
            let mut r2 = rows(5);
            for z in r2.iter_mut() { z.ticker = if bnf { "NIFTY 50" } else { "NIFTYBANK" }.into(); }
            let e = option_positions(id, &Ctx::new(&r2, &r2, &[]), 0, 4, &Params::new());
            assert!(matches!(e, Some(Err(ref m)) if m.starts_with("needs_data") && m.contains(if bnf { "BANKNIFTY" } else { "NIFTY weekly" })), "{id}");
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn f2_option_rules_need_the_weekly_store() {
        let r = rows(30);
        let x = Ctx::new(&r, &r, &[]);
        for id in OPT_IDS {
            assert!(params(id).is_some() && run(id, &x, 0, 29, &Params::new()).is_none(), "{id}");
            assert!(matches!(option_positions(id, &x, 0, 29, &Params::new()), Some(Err(e)) if e.starts_with("needs_data")), "{id}");
        }
    }
}
