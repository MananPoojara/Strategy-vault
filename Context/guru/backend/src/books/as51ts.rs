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
const OPT: &str = "Needs weekly NIFTY / BANKNIFTY option prices during the day; the vault holds monthly-expiry end-of-day options only (decision #268).";

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
            "Held (decisions #268 (3), #271(5)): the rule needs real traded volume for the session VWAP, and the vault's minute files carry no volume (p.97-107)."),
        data!("as51_rsi_vo", "2.4 RSI + Volume Oscillator (5-minute)", "E|Volume Oscillator(5,10) near -30% with RSI(14) at 30 in an uptrend dip: buy; stop the swing low; target RSI overbought (p.107-115 [PDF 55-59])",
            "Held (decision #268): needs volume (none in the minute files), and 'near -30%', 'in tandem', the trend and the swing low are undefined (p.107-115)."),
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
              "A|Read: daily bars; Supertrend = TradingView ta.supertrend(3, 10) on (H+L)/2 with Wilder's ATR; RSI(14) Wilder. Setup: a bar on which the Supertrend is bullish and the RSI crosses above 60 (above 60 now, at or below 60 on the bar before). Entry (decision #271(3)): the steps give no entry order and the only example enters 'at the high of the green candle' (p.235), so the setup bar must be green (close above open) and a buy stop at its high works on the next bar only (filled at the high, or at the open when the bar opens above it), then is cancelled; option stop_entry = 0: entry at the setup bar's close with no candle colour required (the K70 reading). Exit at the close of the bar on which the Supertrend turns bearish (the close below it), which is the trailed stop at the Supertrend; the initial 'swing low' is not modelled (the example uses the Supertrend). Exits are at closes only, so a stop entry and its exit never fall in an unknown order on one daily bar (#240). Short = mirror: RSI crossing below 40 on a red bar, a sell stop at its low. One position at a time [Declared readings]. NSE stock and index examples; daily indices, ETFs and stocks, US and India, are a declared application (the book shorts through F&O)."],
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
            "Held (decision #268): 'rejection' is undefined and the session VWAP needs volume, which the minute files do not have (p.271-279)."),
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
        data!("as51_opt_calendar", "6.1 Weekly hedged calendar spread", "E|Sell current-week CE/PE near 2x India VIX, buy the same strikes next week; adjustments; exit on expiry day (p.303-315 [PDF 153-159])", OPT),
        data!("as51_opt_multi_sell", "6.2 Multi-time option selling", "E|Sell NIFTY weekly OTM CE and PE near Rs 40 at 9:16, 10:16, 11:16, 12:16; stop 100% of premium; exit 3:15 pm (p.315-321 [PDF 159-162])", OPT),
        data!("as51_opt_oi_buy", "6.3 Option buying from open interest", "E|Intraday option buying from open-interest changes (p.321-329 [PDF 162-166])", "Needs weekly NIFTY / BANKNIFTY option prices and open interest during the day; the vault holds monthly-expiry end-of-day options only (decision #268)."),
        data!("as51_opt_st_sell", "6.4 Supertrend option selling", "E|Intraday option selling on Supertrend signals (p.329-335 [PDF 166-169])", OPT),
        data!("as51_opt_vwap", "6.5 Option + VWAP", "E|Intraday option trades with VWAP on the option chart (p.335-345 [PDF 169-174])", OPT),
        data!("as51_opt_mom_buy", "6.6 Momentum option buying", "E|Intraday momentum option buying (p.345-351 [PDF 174-177])", OPT),
        data!("as51_opt_expiry", "6.7 Expiry-day decay", "E|Expiry-day option selling for theta decay (p.351-355 [PDF 177-179])", OPT),
        data!("as51_opt_comb_sl", "6.8 Combined stoploss straddle", "E|Short straddle / strangle with a combined premium stop (p.355-359 [PDF 179-181])", OPT),
        data!("as51_opt_theta", "6.9 Theta decay", "E|Intraday option selling for theta (p.359-363 [PDF 181-183])", OPT),
        data!("as51_opt_btst", "6.10 BTST option buying", "E|At 11:00 pick the CE two strikes ITM, buy on a 20% rise; stop 30%; exit next day 9:30 (p.363-367 [PDF 183-185])", OPT),
        data!("as51_opt_3pm", "6.11 The 3:00 PM NIFTY strategy", "E|At 3 pm short ATM CE/PE after a 5% premium drop; stop 9%; exit 3:29 pm (p.367-371 [PDF 185-187])", OPT),
        data!("as51_opt_mom_sell", "6.12 Momentum option selling", "E|Intraday momentum option selling (p.371-377 [PDF 187-190])", OPT),
        data!("as51_opt_straddle", "6.13 Swing buying: weekly ATM straddle", "E|Buy the weekly ATM straddle 3:30 pm Thursday; stop 50% per leg; trail the survivor; hold 2-3 days (p.377-383 [PDF 190-193])", OPT),
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
            pd("stop_entry", "Entry: 1 = stop at the green candle's high, the book's example (p.235); 0 = the signal bar's close", 1., 0., 1., 1.)],
        "as51_rsi_mw" => vec![pd("lo", "Oversold (30)", 30., 5., 50., 1.), pd("hi", "Overbought (70)", 70., 50., 95., 1.),
            pd("mw", "1 = the M / W shape, two re-crosses (figure p.249); 0 = every re-cross of 30 / 70", 1., 0., 1., 1.)],
        "as51_st_pivot" => vec![pd("atr", "Supertrend ATR length (7)", 7., 2., 50., 1.), pd("fac", "Supertrend factor (3)", 3., 0.5, 10., 0.5)],
        "as51_double_rsi" => vec![pd("lo", "5-minute oversold (30)", 30., 5., 50., 1.), pd("hi", "5-minute overbought (70)", 70., 50., 95., 1.)],
        "as51_ha_psar" => vec![pd("rr", "Target: risk multiple (1:2)", 2., 0.5, 10., 0.5)],
        "as51_ema_scalp" => vec![pd("ema", "EMA length (5; the book: 5 or 7)", 5., 2., 50., 1.), pd("rr", "Target: risk multiple (1:3)", 3., 0.5, 10., 0.5)],
        _ => return None,
    })
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if params(id).is_none() || to >= x.c.len() || from > to { return None; }
    let g = |k: &str| super::jb624::getp(id, p, k);
    let n = |k: &str| g(k).round().max(1.) as usize;
    Some(match id {
        "as51_ichimoku" => ichimoku(x, from, to, g("rr")),
        "as51_fractal_sma" => fractal(x, from, to, n("sma"), g("rr")),
        "as51_donchian_pb" => donchian(x, from, to, n("len"), n("pb_max").max(3), g("rr")),
        "as51_st_rsi" => st_rsi(x, from, to, n("atr"), g("fac"), g("hi"), g("lo"), g("stop_entry") >= 0.5),
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

fn st_rsi(x: &Ctx, from: usize, to: usize, n: usize, fac: f64, hi: f64, lo: f64, stop_entry: bool) -> Vec<Trade> {
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let (_, dir) = supertrend(h, l, c, n, fac);
    let r = rsi(c, 14);
    let up = |t: usize| t > 0 && dir[t] == -1 && r[t] > hi && r[t - 1] <= hi;
    let dn = |t: usize| t > 0 && dir[t] == 1 && r[t] < lo && r[t - 1] >= lo;
    let exit = |t: usize, d: f64| -> Option<&'static str> {
        if d > 0.0 && dir[t] == 1 { Some("close below the Supertrend") } else if d < 0.0 && dir[t] == -1 { Some("close above the Supertrend") } else { None }
    };
    if !stop_entry {
        // the option: entry at the signal bar's close, no candle colour required
        let sig = |t: usize| if up(t) { Some((1.0, f64::NAN, f64::NAN)) } else if dn(t) { Some((-1.0, f64::NAN, f64::NAN)) } else { None };
        return daily(x, from, to, false, &sig, &|t: usize, p: &DPos| exit(t, p.d), &no_trail);
    }
    // Decision #271(3), the default: a green signal bar places a buy stop at its high for the next bar only (a red one a sell
    // stop at its low); an open through the level fills at the open. Exits stay at closes, so no same-day order arises (#240).
    let mut out = vec![];
    let (mut pos, mut pend): (Option<(f64, usize, f64)>, Option<(f64, f64)>) = (None, None);
    for t in from..=to {
        if let Some((d, lv)) = pend.take() {
            let px = if d > 0.0 { if o[t] >= lv { Some(o[t]) } else if h[t] >= lv { Some(lv) } else { None } }
                else if o[t] <= lv { Some(o[t]) } else if l[t] <= lv { Some(lv) } else { None };
            if let Some(px) = px { pos = Some((d, t, px)); }
        }
        if let Some((d, ei, ep)) = pos {
            if let Some(why) = exit(t, d) { out.push(day_trade(d, ei, ep, t, c[t], why, "stop", false)); pos = None; }
        }
        if pos.is_none() {
            if up(t) && c[t] > o[t] { pend = Some((1.0, h[t])); } else if dn(t) && c[t] < o[t] { pend = Some((-1.0, l[t])); }
        }
    }
    if let Some((d, ei, ep)) = pos { out.push(day_trade(d, ei, ep, to, c[to], "open", "stop", true)); }
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
        if s.pend.is_some_and(|p| p.until < k) { s.pend = None; }
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
        assert_eq!(cat.iter().filter(|d| d.status == Status::Live).count(), DAILY.len() + INTRA.len());
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
                ("as51_st_rsi", &[]), ("as51_st_rsi", &[("stop_entry", 0.)]), ("as51_rsi_mw", &[]), ("as51_rsi_mw", &[("mw", 0.)]), ("as51_st_pivot", &[]), ("as51_double_rsi", &[]), ("as51_ha_psar", &[]), ("as51_ema_scalp", &[]), ("as51_ema_scalp", &[("ema", 7.)])];
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
}
