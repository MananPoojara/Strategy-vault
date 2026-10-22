//! The Best of the Professional Traders Journal: Market Timing (Laurence A. Connors, M. Gordon Publishing).
//! Book id "ptjmt". Connors VIX Reversals I-V, the CVR IV Friday-Monday system, TRIN Reversals, TRIN Thrusts
//! and the Percent Advance/Decline Indicator (PADI), as printed in the PTJ 1997-98.
//!
//! Engine conventions, reproduced from the book's TradeStation tables (S&P 500 futures, 1 contract) and the
//! CVR I/II/III trade-by-trade lists printed in Advanced Trading Strategies (see library/ptjmt/implementation-report.md):
//!   * one position, entries on the close of the signal day;
//!   * an opposite signal exits on the close and enters the other way on the same close (TradeStation stop-and-reverse);
//!   * N-day exit = close of the N-th bar after entry; a same-direction signal on that exit bar keeps the trade
//!     open and restarts the count (TradeStation nets the exit against the new entry); a same-direction signal on
//!     any other bar is ignored ("Max contracts held 1");
//!   * % stop (TRIN Thrusts, PADI): from the entry close, checked from the bar after entry on the bar's low/high,
//!     filled at the stop or at the open if the bar gaps through it. After a stop a new signal can enter on that close.
//!
//! VIX OHLC: the engine's built-in series VIX.Open/High/Low/Close (the server's VIX dir, aligned by date).
//! Breadth: external series NYSE_TRIN / NYSE_ADV_DEC.* (US) and NSE_TRIN / NSE_ADV_DEC.* (India), see extra_data/README.md.
#![allow(unused_imports, dead_code)]
use crate::connors::*;
use crate::Row;

pub const BOOK: &str = "ptjmt";

/// Exact daily contracts admitted after the 2026-10-21 source review (connors/ptjmt-admission/SOURCE-CONTRACT.md)
/// and independent chronological Python ledgers. Declared index-timing application of an S&P-futures study.
/// CVR IV / IV Friday stay held (accuracy gate: fixed VIX points, next-open exit).
pub const REVIEWED_DAILY: &[&str] = &[
    "ptjmt_cvr1", "ptjmt_cvr2", "ptjmt_cvr3", "ptjmt_cvr5",
    "ptjmt_trin_reversals", "ptjmt_trin_reversals_20", "ptjmt_trin_thrusts", "ptjmt_padi",
    "ptjmt_trin_reversals_nse", "ptjmt_trin_reversals_20_nse", "ptjmt_trin_thrusts_nse", "ptjmt_padi_nse",
];
pub fn reviewed_daily(id: &str) -> bool { REVIEWED_DAILY.contains(&id) }
/// NYSE-breadth and CVR rules: actual SPX / SPY; CVR II and III (VIX close/high/low, scale-free thresholds) also
/// NIFTY 50 with India VIX. NSE-breadth versions (declared adaptation): NIFTY 50 only.
pub fn admitted_symbol(id: &str, ticker: &str) -> bool {
    reviewed_daily(id) && if id.ends_with("_nse") { ticker == "NIFTY 50" } else {
        matches!(ticker, "SPX" | "SPY") || (ticker == "NIFTY 50" && matches!(id, "ptjmt_cvr2" | "ptjmt_cvr3"))
    }
}
/// Rules whose % stop can fill at the traded instrument's open (refused where that index's opens are stale).
pub fn reads_open(id: &str) -> bool { matches!(id, "ptjmt_trin_thrusts" | "ptjmt_padi") }
/// The served CBOE VIX file holds close-only bars (open = high = low = close) for 1990-1991.
pub const REAL_VIX_OHLC_FROM: &str = "1992-01-01";

/// Strategy note + the shared book note (+ the VIX-open caveat for rules that read the VIX open).
macro_rules! note {
    (cvr $s:literal) => {
        concat!($s, " Book: 1 S&P 500 futures contract ($500/pt), signals on the original CBOE VIX (OEX-based, today's VXO), no stops, no slippage or commission. Here the traded index is the ticker you pick and the VIX is the server's VIX (India VIX for NIFTY, CBOE VIX for US ETFs). Long and short share one position: [Not from the book] an opposite signal stops and reverses (the rules are silent). p.src=1 reads the external series VXO_OHLC (CBOE OEX VIX, the book's index; 1986 to 2021-09-23, US only, no trades where it has no data): on SPY 1993-97 it gives CVR I 63/72 and CVR II 93/97 of the ATS-listed entries, against 19/72 and 46/97 with today's SPX VIX (the default p.src=0, so live runs work after 2021 and in India). p.restart (default 1) = a same-direction signal on the N-day exit bar keeps the trade and restarts the count [Not from the book: a TradeStation netting inference, fitted to the ATS CVR lists; 0 = exit and re-enter on that close].")
    };
    (open $s:literal) => {
        concat!(note!(cvr $s), " Reads the VIX OPEN: NSE defines the India VIX daily open as the previous close (276/276 sampled days; the vault's first-minute open is within 0.5% of it on 57% of days), so on Indian data this rule really tests 'VIX close vs yesterday's close' and its India results are NOT a meaningful test of the book's rule; kept exactly as written.")
    };
    (breadth $s:literal) => {
        concat!($s, " Book: 1 S&P 500 futures contract ($500/pt), no slippage or commission; run here on the index/ETF you pick (vault convention). Long and short share one position: [Not from the book] an opposite signal stops and reverses (the rules are silent). p.restart (default 1) = a same-direction signal on the N-day exit bar keeps the trade and restarts the count [Not from the book: a TradeStation netting inference; with it TRIN Thrusts gives 763 trades vs the book's 764, without it 836].")
    };
}

const CVR1: &[&str] = &[
    "E|Buy: \"Today the VIX must make a 15-day high.\" \"Today, the close of the VIX must be below the open.\" \"buy the S&P's (or the market) on the close of today\"",
    "E|Sell: \"Today the VIX must make a 15-day low.\" \"Today the close of the VIX must be above the open.\" \"sell the S&P's (or the market) on the close of today\"",
    "X|\"Hold the position at least one to three days.\" Coded: exit on the close 3 days after entry (book tables: avg bars 3 / 3)",
    "X|No stops in the test. [Not from the book] An opposite signal stops and reverses (the rules are silent; TradeStation stop-and-reverse, inferred from the ATS appendix trade lists)",
];
const CVR2: &[&str] = &[
    "F|\"Take a five-period RSI of the closing VIX.\"",
    "E|Buy: \"When a daily RSI reading above 70 is followed by a downtick in RSI, buy the market that day on the close.\" (coded: yesterday's RSI >= 70, today's RSI lower)",
    "E|Sell: \"When an RSI reading below 30 is followed by an uptick in RSI, sell the market that day on the close.\" (coded: yesterday's RSI <= 30, today's RSI higher)",
    "X|\"Exit five to eight trading days later\"; the published test uses \"an eight-day exit\"",
    "X|No stops in the test. [Not from the book] An opposite signal stops and reverses (the rules are silent; TradeStation stop-and-reverse, inferred from the ATS appendix trade lists)",
];
const CVR3: &[&str] = &[
    "E|Buy: \"Today, the low of the VIX must be above its 10-day moving average.\" \"Today, the VIX must close at least 10 percent above its 10-day moving average.\" \"buy the market on the close\"",
    "E|Sell: \"Today, the high of the VIX must be below its 10-day moving average.\" \"Today, the VIX must close at least 10 percent below its 10-day moving average.\" \"sell on the close\"",
    "X|Long: \"Exit (on the close) the day the VIX trades (intraday) below yesterday's 10-day moving average (reversion to the mean).\"",
    "X|Short: \"Exit (on the close) the day the VIX trades (intraday) above yesterday's 10-day moving average\"",
    "X|No time stop, no price stop. [Not from the book] An opposite signal stops and reverses (the rules are silent; TradeStation stop-and-reverse, inferred from the ATS appendix trade lists)",
];
const CVR4: &[&str] = &[
    "E|Sell: \"If the VIX closes 1.50 or more points greater than its open, sell the S&P futures MOC.\"",
    "E|Buy: \"If the VIX closes 1.50 or more points below its open, buy the S&P futures MOC.\"",
    "X|\"Cover your position on the opening the next morning (Globex is omitted).\" / \"Sell your position on the opening the next morning\"",
];
const CVR4F: &[&str] = &[
    "F|Fridays only: \"He only looked at the CVR IV signals that occurred on Fridays.\"",
    "E|Sell: the VIX closes 1.50 or more points above its open (Friday): sell MOC",
    "E|Buy: the VIX closes 1.50 or more points below its open (Friday): buy MOC",
    "X|Exit on the next session's open (Monday)",
];
const CVR5: &[&str] = &[
    "E|Buy: \"Today's VIX must open and close in the bottom third of its trading range.\" \"The close of the VIX must be above its 10-day moving average.\" \"buy the market on the close\"",
    "E|Sell: \"sells are reversed\": the VIX opens and closes in the top third of its range and closes below its 10-day moving average: sell on the close",
    "X|\"exit the next day no later than the close\" (coded: next day's close; book avg bars 1 / 1)",
    "X|[Not from the book] An opposite signal stops and reverses (the rules are silent; TradeStation stop-and-reverse, inferred from the ATS appendix trade lists)",
];
const TR: &[&str] = &[
    "F|\"Take a three-period simple moving average of the TRIN\" (NYSE closing TRIN)",
    "E|Buy: \"If yesterday's average hits a 30 trading-day high and today's average closes below yesterday's average, then buy the market on the close.\"",
    "E|Sell: \"If yesterday's average hits a 30 trading-day low and today's average closed above yesterday's average, then sell the market on the close.\"",
    "X|\"Exit seven trading days later.\"",
    "X|No stops in the test. [Not from the book] An opposite signal stops and reverses (the rules are silent; TradeStation stop-and-reverse, inferred from the ATS appendix trade lists)",
];
const TR20: &[&str] = &[
    "F|\"If you wish to be more aggressive, you can use 20-day highs/lows instead of 30.\"",
    "F|Three-period simple moving average of the NYSE closing TRIN",
    "E|Buy: yesterday's average is a 20-day high and today's average is below yesterday's: buy on the close",
    "E|Sell: yesterday's average is a 20-day low and today's average is above yesterday's: sell on the close",
    "X|Exit seven trading days later. [Not from the book] An opposite signal stops and reverses (the rules are silent; TradeStation stop-and-reverse, inferred from the ATS appendix trade lists)",
];
const TT: &[&str] = &[
    "E|Buy: \"If today's closing TRIN is 30 percent or more below yesterday's closing TRIN, buy the market on the close today.\"",
    "E|Sell: \"If today's closing TRIN is 30 percent or more above yesterday's closing TRIN, sell the market on the close today.\"",
    "X|\"Sell on the close two trading days later.\" / \"Close the position two trading days later.\"",
    "X|\"Here are the mechanical results using a 1 1/2 percent stop.\" (stop from the entry close)",
    "X|[Not from the book] An opposite signal stops and reverses (the rules are silent; TradeStation stop-and-reverse, inferred from the ATS appendix trade lists)",
];
const PADI: &[&str] = &[
    "F|Total issues = advances + declines + unchanged; advancing % = advances / total; declining % = declines / total",
    "F|\"Take the six-day average of the advancing percent and the six-day average of the declining percentage.\"",
    "E|Sell: \"If yesterday's advancing average was 40 percent or higher and today's advancing average is less than yesterday's advancing average, sell on the close (only if the stock market is trading below its 100-day moving average).\"",
    "E|Buy: \"If yesterday's declining average was 40 percent or higher and today's declining average is less than yesterday's declining average, we buy on the close (only if the market is trading above its 100-day moving average).\"",
    "X|\"Exit in five to seven days or if there is a signal in the opposite direction.\" Published test: \"a 1 1/2 percent stop with a five-day exit\"",
];

const P93: &str = "01/01/93-10/01/97, S&P 500 futures, 1 contract, no stops";
const NSE_TRIN_NOTE: &str = "India mapping: series NSE_TRIN = (advancing / declining NSE stocks) / (traded quantity of the advancing / declining stocks), close vs previous close, rebuilt from the daily CM bhavcopy on ALL series (NSE's official advance/decline universe, like the NYSE composite's all issues; median 0.3% off NSE's published counts). Same rules as the NYSE version. Book: 1 S&P 500 futures contract, here run on the index you pick. Calibration: NSE TRIN is far more volatile than 1990s NYSE TRIN (1999-2003 share volume is extremely concentrated, median TRIN about 0.7), so it moves 30% or more in a day on about 69% of NSE days (NYSE 1990-97: 47%) and TRIN Thrusts fires constantly (about 3,700 trades on NIFTY 50); with an average trade near 0.10% gross, India round-trip costs of about 6-8 bps (STT, exchange, stamp, GST) eat most of it: set the cost field. p.restart as in the NYSE version.";

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, group, rules, notes, claim, needs_series| StrategyDef {
        id, name, group, session: 0, dir: "Long/Short", status: Status::Live, needs_volume: false, needs_vix: false, needs_series, book: BOOK, rules, notes, claim,
    };
    let vx = |s: StrategyDef| StrategyDef { needs_vix: true, ..s };
    vec![
        vx(d("ptjmt_cvr1", "CVR I · 15-day VIX extreme reversing intraday", "Connors VIX Reversal I", CVR1,
            note!(open "15-day high/low = today's VIX intraday high (low) is the highest high (lowest low) of the last 15 VIX bars, today included. Exit: close of the 3rd day after entry. Same signal as Trading Connors VIX Reversals 'CVR 1' (5 days there); Advanced Trading Strategies prints this exact test trade by trade."),
            claim("72 (L 50 / S 22)", "65% (L 64% / S 68%)", "$1,130.56 per contract", "3 / 3 bars (win / loss)", P93), &[])),
        vx(d("ptjmt_cvr2", "CVR II · VIX RSI(5) extreme turns", "Connors VIX Reversal II", CVR2,
            note!(cvr "RSI = Wilder (TradeStation) RSI of the VIX close. Threshold: rule 2 says '70 or above', rule 3 'above 70'; coded >= (parameter). Exit: close of the 8th day after entry (the published test). Trading Connors VIX Reversals 'CVR 2' is the same signal with a 2-3 day exit."),
            claim("96 (L 55 / S 41) + 1 open", "61% (L 67% / S 54%)", "$1,865.63 per contract", "7 / 8 bars (win / loss)", P93), &[])),
        vx(d("ptjmt_cvr3", "CVR III · VIX 10% from its 10-day MA", "Connors VIX Reversal III", CVR3,
            note!(cvr "10-day MA = simple MA of the VIX close. The entry rules compare with TODAY's 10-day MA, as written (\"its 10-day moving average\"; only the exit says \"yesterday's\"), parameter 'Entry MA as of N days ago' = 0. [Not from the book] 1 = yesterday's MA, fitted to the ATS appendix list of this test: with today's MA 53 of its 75 listed entries appear, with yesterday's 71. Exit: first close after entry on which the VIX low (longs) / high (shorts) trades strictly below / above yesterday's 10-day MA (book \"below\" / \"above\"; a touch does not exit); no time cap (book avg bars in losers 5). Co-created with Dave Landry; same signal as Trading Connors VIX Reversals 'CVR 3'."),
            claim("75 (L 41 / S 34)", "72% (L 78% / S 65%)", "$1,609.00 per contract", "4 / 5 bars (win / loss)", P93), &[])),
        vx(d("ptjmt_cvr4", "CVR IV · VIX open-to-close 1.50 points, out next open", "Connors VIX Reversal IV", CVR4,
            note!(open "Thresholds are absolute VIX points (the book's VXO was 10-40; India VIX trades in a similar range, so the points are kept). Exit at the next bar's open of the traded index (the book: the S&P futures day-session open). Book table by threshold: 1.50 pts 105 trades 58%, 1.75 81 62%, 2.0 58 64%, 2.25 45 69%, 2.50 32 72%; 'Long/Short Profits' are average $ per trade per side. Trades never overlap. Trading Connors VIX Reversals says CVR 4 'was removed in 1998'."),
            claim("105", "58%", "$566 per trade (L $504 / S $665)", "overnight", "1993 - mid 1998, S&P 500 futures, threshold 1.50 points"), &[])),
        vx(d("ptjmt_cvr4_friday", "CVR IV Friday-Monday · Friday signals only", "Connors VIX Reversal IV", CVR4F,
            note!(open "Bill Masciarelli's filter: only CVR IV signals on a calendar Friday (a holiday Friday gives no signal), exit at the next session's (Monday's) open. Book table: 1.50 pts 29 trades 72%, 1.75 19 77%, 2.0 12 75%, 2.25 11 82%, 2.50 8 100%."),
            claim("29", "72%", "$690 per trade (L $765 / S $585)", "Friday close to Monday open", "1993 - mid 1998, S&P 500 futures, threshold 1.50 points"), &[])),
        vx(d("ptjmt_cvr5", "CVR V · VIX opens and closes in the range extreme", "Connors VIX Reversal V", CVR5,
            note!(open "Bottom third: open and close both within (high - low)/3 of the VIX low (<=); top third mirrored; no signal when high = low. 10-day MA = simple MA of the VIX close, today included. Exit: next day's close (the book's 'or earlier' profit-taking is discretionary). The book's examples (buys on 8/11/98 and 8/21/98) are reproduced. Trading Connors VIX Reversals tests the same signal with 1- and 3-day exits."),
            claim("76 (L 58 / S 18)", "63% (L 69% / S 44%)", "$851.25 per contract", "1 / 1 bars (win / loss)", "01/28/93-08/25/98, S&P 500 futures, no stops"), &[])),
        d("ptjmt_trin_reversals", "TRIN Reversals · 3-day TRIN average turns from a 30-day extreme", "TRIN Reversals", TR,
            note!(breadth "Series NYSE_TRIN = NYSE Arms index daily close = (advancing issues / declining issues) / (advancing volume / declining volume). 'Yesterday's average hits a 30 trading-day high' = yesterday's 3-day average is the highest of the last 30 averages, yesterday included. Exit: close of the 7th day after entry."),
            claim("90 (L 51 / S 39)", "69% (L 75% / S 62%)", "$2,490 per trade (L $2,741 / S $2,162)", "7 days", "01/01/92-10/15/98, S&P 500 futures, no stops"), &["NYSE_TRIN"]),
        d("ptjmt_trin_reversals_nse", "TRIN Reversals · NSE TRIN", "TRIN Reversals", TR, NSE_TRIN_NOTE, None, &["NSE_TRIN"]),
        d("ptjmt_trin_reversals_20", "TRIN Reversals · 20-day variant", "TRIN Reversals", TR20,
            note!(breadth "The book's 'more aggressive' variant: 'This will increase the number of trades by approximately 30 percent and backtesting during the same period shows it to be profitable just over 60 percent of the time.' Series NYSE_TRIN as in the 30-day version."),
            claim("about 30% more than 90", "just over 60%", "not given", "7 days", "01/01/92-10/15/98, S&P 500 futures"), &["NYSE_TRIN"]),
        d("ptjmt_trin_reversals_20_nse", "TRIN Reversals · 20-day variant · NSE TRIN", "TRIN Reversals", TR20, NSE_TRIN_NOTE, None, &["NSE_TRIN"]),
        d("ptjmt_trin_thrusts", "TRIN Thrusts · TRIN moves 30% in a day", "TRIN Thrusts", TT,
            note!(breadth "Series NYSE_TRIN (closing Arms index). Buy: TRIN <= 0.70 x yesterday's; sell: TRIN >= 1.30 x yesterday's (the book's own example .70 -> .91). Exit: close of the 2nd day after entry, or the 1.5% stop from the entry close (the book's '982 to 987' stop for a 900 market is a misprint; the percentage is used). The author rarely uses it stand-alone."),
            claim("764 (L 342 / S 422)", "51% (L 57% / S 46%)", "$244.10 per contract", "2 / 1 bars (win / loss)", "01/03/90-10/01/97, S&P 500 futures, 1.5% stop"), &["NYSE_TRIN"]),
        d("ptjmt_trin_thrusts_nse", "TRIN Thrusts · NSE TRIN", "TRIN Thrusts", TT, NSE_TRIN_NOTE, None, &["NSE_TRIN"]),
        d("ptjmt_padi", "PADI · Percent Advance/Decline Indicator", "Percent Advance/Decline Indicator", PADI,
            note!(breadth "Series NYSE_ADV_DEC with columns Advances, Declines, Unchanged (NYSE issue counts per day; rule 1's 'shares traded' means issues). Percentages 0-100, 6-day simple averages. The 100-day MA filter is a simple MA of this ticker's close, today included. Exit: close of the 5th day after entry, the 1.5% stop from the entry close, or an opposite signal."),
            claim("109 (L 83 / S 26)", "61% (L 64% / S 50%)", "$1,311.31 per contract", "6 / 5 bars (win / loss)", "01/01/90-10/03/97, S&P 500 futures, 1.5% stop, 5-day exit"),
            &["NYSE_ADV_DEC.Advances", "NYSE_ADV_DEC.Declines", "NYSE_ADV_DEC.Unchanged"]),
        d("ptjmt_padi_nse", "PADI · NSE advances/declines", "Percent Advance/Decline Indicator", PADI,
            "India mapping: series NSE_ADV_DEC with columns Advances, Declines, Unchanged = NSE stocks (all CM series, NSE's official A/D universe) closing above / below / at the previous close, rebuilt from the daily CM bhavcopy. Same rules as the NYSE version; the 100-day MA is on the index you pick. Calibration: the 6-day declining average is at or above 40% on 89% of NSE days since 2000 (NYSE 1990-97: 31%), advancing 77% (NYSE 39%), so the book's 40% level is not an extreme in India. The book's 40 stays the default; the levels that fire as often as on the NYSE are about 54 (declining) and 48 (advancing) [Not from the book: set p.level; level 54 gives 409 trades, PF 1.66 on NIFTY 50 vs 776, PF 1.38 at 40]. p.restart (default 1) = same-direction signal on the exit bar keeps the trade [Not from the book].",
            None, &["NSE_ADV_DEC.Advances", "NSE_ADV_DEC.Declines", "NSE_ADV_DEC.Unchanged"]),
    ]
}

/// Tunable numbers of each strategy (book value = default). None = not a strategy of this book.
pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    let hold = |def, max| p("hold", "Exit on the close N days after entry", def, 1.0, max, 1.0);
    let stop = || p("stop", "Protective stop % from entry (0 = off)", 1.5, 0.0, 4.0, 0.25);
    let mut v = match id {
        "ptjmt_cvr1" => vec![p("len", "VIX high/low of N days", 15.0, 3.0, 30.0, 1.0), hold(3.0, 10.0)],
        "ptjmt_cvr2" => vec![p("rsi_n", "RSI period (VIX close)", 5.0, 2.0, 14.0, 1.0), p("ob", "Buy: yesterday's RSI at or above", 70.0, 55.0, 90.0, 5.0),
            p("os", "Sell: yesterday's RSI at or below", 30.0, 10.0, 45.0, 5.0), hold(8.0, 15.0)],
        "ptjmt_cvr3" => vec![p("ma", "VIX moving average length", 10.0, 5.0, 20.0, 1.0), p("pct", "VIX close at least % beyond the MA", 10.0, 3.0, 20.0, 1.0),
            p("ma_lag", "Entry MA as of N days ago (0 = today's, book p.13; 1 = yesterday's, fitted to the ATS list)", 0.0, 0.0, 1.0, 1.0)],
        "ptjmt_cvr4" | "ptjmt_cvr4_friday" => vec![p("pts", "VIX close vs open, points", 1.5, 0.5, 4.0, 0.25)],
        "ptjmt_cvr5" => vec![p("ma", "VIX moving average length", 10.0, 5.0, 20.0, 1.0),
            p("zone", "Open & close within % of the range from the extreme", 100.0 / 3.0, 10.0, 50.0, 5.0), hold(1.0, 5.0)],
        "ptjmt_trin_reversals" | "ptjmt_trin_reversals_nse" | "ptjmt_trin_reversals_20" | "ptjmt_trin_reversals_20_nse" => vec![
            p("ma", "TRIN moving average length", 3.0, 1.0, 10.0, 1.0),
            p("look", "Yesterday's average at an N-day high/low", if id.contains("_20") { 20.0 } else { 30.0 }, 10.0, 60.0, 5.0),
            hold(7.0, 15.0)],
        "ptjmt_trin_thrusts" | "ptjmt_trin_thrusts_nse" => vec![p("thrust", "TRIN change % vs yesterday", 30.0, 10.0, 60.0, 5.0), hold(2.0, 5.0), stop()],
        "ptjmt_padi" | "ptjmt_padi_nse" => vec![p("avg", "Average of the daily % over N days", 6.0, 2.0, 15.0, 1.0),
            p("level", "Yesterday's average at or above %", 40.0, 25.0, 60.0, 1.0), p("trend", "Market moving average (days)", 100.0, 20.0, 200.0, 10.0),
            hold(5.0, 10.0), stop()],
        _ => return None,
    };
    if id.starts_with("ptjmt_cvr") {
        v.push(p("src", "Volatility index: 0 = server VIX, 1 = VXO_OHLC (book's OEX VIX)", 0.0, 0.0, 1.0, 1.0));
    }
    if !id.starts_with("ptjmt_cvr4") {
        v.push(p("restart", "Same-direction signal on the exit bar keeps the trade, count restarts (1) / exit and re-enter (0) [Not from the book]", 1.0, 0.0, 1.0, 1.0));
    }
    Some(v)
}

// ---------------------------------------------------------------- VIX OHLC

/// The VIX's bars (the engine's built-in VIX.Open/High/Low/Close series, kept only on days with a VIX bar, so
/// rolling windows and the RSI run on the VIX's own consecutive bars) + map from the instrument's bar to that VIX bar.
struct Vx {
    o: Vec<f64>,
    h: Vec<f64>,
    l: Vec<f64>,
    c: Vec<f64>,
    at: Vec<Option<usize>>,
}

impl Vx {
    /// `src` 0 = the server's VIX (built-in VIX.*), 1 = external VXO_OHLC.* (CBOE OEX VIX, the book's index).
    fn of(x: &Ctx, src: usize) -> Self {
        let n = x.c.len();
        let b = if src == 1 { "VXO_OHLC" } else { "VIX" };
        let col = |k: &str| x.series(&format!("{b}.{k}")).map(<[f64]>::to_vec).unwrap_or_else(|| vec![f64::NAN; n]);
        let (o, h, l, c) = (col("Open"), col("High"), col("Low"), col("Close"));
        let days: Vec<usize> = (0..n).filter(|&i| [o[i], h[i], l[i], c[i]].iter().all(|v| v.is_finite())).collect();
        let mut at = vec![None; n];
        for (k, &i) in days.iter().enumerate() {
            at[i] = Some(k);
        }
        let pick = |v: &[f64]| days.iter().map(|&i| v[i]).collect();
        Vx { o: pick(&o), h: pick(&h), l: pick(&l), c: pick(&c), at }
    }
    /// A signal computed on the VIX's own bar k, mapped to the instrument's bars (0 where the VIX has no bar).
    fn on(&self, f: &dyn Fn(usize) -> i8) -> Vec<i8> {
        self.at.iter().map(|k| k.map_or(0, |k| f(k))).collect()
    }
}

// ---------------------------------------------------------------- simulator

/// TradeStation-style stop-and-reverse, one position, entries on the close (see the module header).
/// `sig[i]` = +1 buy / -1 sell / 0. `hold` = exit N bars after entry (0 = none). `xf(i, dir)` = extra exit on the close.
/// `stop` = fraction from the entry price (0 = none). `restart` = a same-direction signal on the exit bar keeps the
/// trade and restarts the count (fitted to the ATS lists); false = exit, then re-enter on that close.
fn sar(x: &Ctx, from: usize, to: usize, sig: &[i8], hold: usize, xf: Option<(&dyn Fn(usize, f64) -> bool, &str)>, stop: f64, restart: bool) -> Vec<Trade> {
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let mut out = Vec::new();
    let mut cur: Option<(Trade, usize)> = None; // trade + bar the hold count runs from
    let open = |i: usize, s: i8| Trade {
        dir: s as f64,
        legs: vec![Leg { i, px: c[i], w: 1.0 }],
        exit_i: i,
        exit_px: c[i],
        reason: String::new(),
        entry_kind: "close",
        signal_i: i,
        open: false, exits: Vec::new()
    };
    let done = |mut t: Trade, i: usize, px: f64, why: String, out: &mut Vec<Trade>| {
        t.exit_i = i;
        t.exit_px = px;
        t.signal_i = i;
        t.reason = why;
        out.push(t);
    };
    for i in from..=to {
        let s = sig[i];
        if let Some((t, ci)) = cur.take() {
            if stop > 0.0 && i > t.legs[0].i {
                let sp = t.legs[0].px * (1.0 - t.dir * stop);
                let fill = if t.dir > 0.0 { (l[i] <= sp).then(|| o[i].min(sp)) } else { (h[i] >= sp).then(|| o[i].max(sp)) };
                if let Some(px) = fill {
                    done(t, i, px, format!("{}% stop", stop * 100.0), &mut out);
                    if s != 0 && i < to {
                        cur = Some((open(i, s), i));
                    }
                    continue;
                }
            }
            if s != 0 && s as f64 == -t.dir {
                done(t, i, c[i], "Opposite signal: stop and reverse".into(), &mut out);
                if i < to {
                    cur = Some((open(i, s), i));
                }
                continue;
            }
            let rule = xf.and_then(|(f, why)| f(i, t.dir).then(|| why.to_string()));
            let timed = (hold > 0 && i - ci >= hold).then(|| format!("{hold} days held"));
            match rule.or(timed) {
                Some(_) if restart && s != 0 && i < to => cur = Some((t, i)), // same-direction re-signal on the exit bar: stay in, count restarts
                Some(why) => {
                    done(t, i, c[i], why, &mut out);
                    if s != 0 && i < to {
                        cur = Some((open(i, s), i)); // restart off: re-enter on the same close
                    }
                }
                None => cur = Some((t, ci)),
            }
            continue;
        }
        if s != 0 && i < to {
            cur = Some((open(i, s), i));
        }
    }
    if let Some((mut t, _)) = cur {
        t.exit_i = to;
        t.exit_px = c[to];
        t.signal_i = to;
        t.reason = "open".into();
        t.open = true;
        out.push(t);
    }
    out
}

/// Enter on the signal bar's close, exit at the next bar's open (CVR IV). Trades cannot overlap.
fn close_to_open(x: &Ctx, from: usize, to: usize, sig: &[i8]) -> Vec<Trade> {
    (from..to)
        .filter(|&i| sig[i] != 0)
        .map(|i| Trade {
            dir: sig[i] as f64,
            legs: vec![Leg { i, px: x.c[i], w: 1.0 }],
            exit_i: i + 1,
            exit_px: x.o[i + 1],
            reason: "Next morning's open".into(),
            entry_kind: "close",
            signal_i: i,
            open: false, exits: Vec::new()
        })
        .collect()
}

/// Simple MA that is NaN wherever its window holds a NaN (external series have gaps; the shared `sma` keeps a
/// running sum, so one NaN would blank everything after it).
fn sma_gaps(v: &[f64], n: usize) -> Vec<f64> {
    (0..v.len()).map(|i| if i + 1 >= n { v[i + 1 - n..=i].iter().sum::<f64>() / n as f64 } else { f64::NAN }).collect()
}

/// Calendar Friday (1970-01-01 was a Thursday).
fn friday(date: &str) -> bool {
    (crate::day_number(date) + 4).rem_euclid(7) == 5
}

// ---------------------------------------------------------------- rules

/// None = not a strategy of this book.
pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    params(id)?;
    let src = p.get("src").map_or(0, |v| v.round() as usize);
    Some(run_with(id, x, &Vx::of(x, src), from, to, p))
}

fn run_with(id: &str, x: &Ctx, v: &Vx, from: usize, to: usize, p: &Params) -> Vec<Trade> {
    let schema = params(id).unwrap_or_default();
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).unwrap_or_else(|| panic!("{id} has no param {k}")).def);
    let u = |k: &str| g(k).round().max(1.0) as usize;
    let (vo, vh, vl, vc) = (&v.o, &v.h, &v.l, &v.c);
    let sign = |b: bool, s: bool| if b { 1i8 } else if s { -1 } else { 0 };
    let n = x.c.len();
    let ser = |name: &str| x.series(name).map(<[f64]>::to_vec).unwrap_or_else(|| vec![f64::NAN; n]);
    let pre = |us: &'static str, nse: &'static str| if id.ends_with("_nse") { nse } else { us };
    let rs = p.get("restart").map_or(true, |v| *v >= 0.5); // cvr4 has no such param
    match id {
        "ptjmt_cvr1" => {
            let len = u("len");
            let hh = |k: usize| k + 1 >= len && vh[k + 1 - len..=k].iter().all(|&y| vh[k] >= y);
            let ll = |k: usize| k + 1 >= len && vl[k + 1 - len..=k].iter().all(|&y| vl[k] <= y);
            sar(x, from, to, &v.on(&|k| sign(hh(k) && vc[k] < vo[k], ll(k) && vc[k] > vo[k])), u("hold"), None, 0.0, rs)
        }
        "ptjmt_cvr2" => {
            let (r, ob, os) = (rsi(vc, u("rsi_n")), g("ob"), g("os"));
            sar(x, from, to, &v.on(&|k| sign(k > 0 && r[k - 1] >= ob && r[k] < r[k - 1], k > 0 && r[k - 1] <= os && r[k] > r[k - 1])), u("hold"), None, 0.0, rs)
        }
        "ptjmt_cvr3" => {
            let (ma, pct, lag) = (sma(vc, u("ma")), g("pct") / 100.0, g("ma_lag").round().max(0.0) as usize);
            let m = |k: usize| if k >= lag { ma[k - lag] } else { f64::NAN };
            let sig = v.on(&|k| sign(vl[k] > m(k) && vc[k] >= m(k) * (1.0 + pct), vh[k] < m(k) && vc[k] <= m(k) * (1.0 - pct)));
            let back = |i: usize, dir: f64| v.at[i].map_or(false, |k| k > 0 && if dir > 0.0 { vl[k] < ma[k - 1] } else { vh[k] > ma[k - 1] });
            sar(x, from, to, &sig, 0, Some((&back, "VIX traded back through yesterday's 10-day MA")), 0.0, rs)
        }
        "ptjmt_cvr4" | "ptjmt_cvr4_friday" => {
            let (pts, fri) = (g("pts") - 1e-9, id == "ptjmt_cvr4_friday"); // "1.50 or more", float-safe
            let sig = v.on(&|k| sign(vc[k] <= vo[k] - pts, vc[k] >= vo[k] + pts));
            let sig: Vec<i8> = sig.iter().enumerate().map(|(i, &s)| if !fri || friday(&x.r[i].date) { s } else { 0 }).collect();
            close_to_open(x, from, to, &sig)
        }
        "ptjmt_cvr5" => {
            let (ma, z) = (sma(vc, u("ma")), g("zone") / 100.0);
            let sig = v.on(&|k| {
                let r = vh[k] - vl[k];
                let (bot, top) = (vl[k] + r * z, vh[k] - r * z);
                sign(r > 0.0 && vo[k] <= bot && vc[k] <= bot && vc[k] > ma[k], r > 0.0 && vo[k] >= top && vc[k] >= top && vc[k] < ma[k])
            });
            sar(x, from, to, &sig, u("hold"), None, 0.0, rs)
        }
        "ptjmt_trin_reversals" | "ptjmt_trin_reversals_nse" | "ptjmt_trin_reversals_20" | "ptjmt_trin_reversals_20_nse" => {
            let (a, look) = (sma_gaps(&ser(pre("NYSE_TRIN", "NSE_TRIN")), u("ma")), u("look"));
            // yesterday's average is the highest (lowest) of the last `look` averages, yesterday included
            let ext = |i: usize, hi: bool| i >= look && a[i - look..i].iter().all(|&y| if hi { a[i - 1] >= y } else { a[i - 1] <= y });
            let sig: Vec<i8> = (0..n).map(|i| sign(ext(i, true) && a[i] < a[i - 1], ext(i, false) && a[i] > a[i - 1])).collect();
            sar(x, from, to, &sig, u("hold"), None, 0.0, rs)
        }
        "ptjmt_trin_thrusts" | "ptjmt_trin_thrusts_nse" => {
            let (t, th) = (ser(pre("NYSE_TRIN", "NSE_TRIN")), g("thrust") / 100.0);
            let sig: Vec<i8> = (0..n).map(|i| sign(i > 0 && t[i] <= t[i - 1] * (1.0 - th), i > 0 && t[i] >= t[i - 1] * (1.0 + th))).collect();
            sar(x, from, to, &sig, u("hold"), None, g("stop") / 100.0, rs)
        }
        "ptjmt_padi" | "ptjmt_padi_nse" => {
            let pre = pre("NYSE_ADV_DEC", "NSE_ADV_DEC");
            let (adv, dec, unch) = (ser(&format!("{pre}.Advances")), ser(&format!("{pre}.Declines")), ser(&format!("{pre}.Unchanged")));
            let pct = |v: &[f64]| -> Vec<f64> { (0..n).map(|i| 100.0 * v[i] / (adv[i] + dec[i] + unch[i])).collect() };
            let (aa, da) = (sma_gaps(&pct(&adv), u("avg")), sma_gaps(&pct(&dec), u("avg")));
            let (ma, lv) = (x.sma_n(u("trend")), g("level"));
            let sig: Vec<i8> = (0..n)
                .map(|i| sign(i > 0 && da[i - 1] >= lv && da[i] < da[i - 1] && x.c[i] > ma[i], i > 0 && aa[i - 1] >= lv && aa[i] < aa[i - 1] && x.c[i] < ma[i]))
                .collect();
            sar(x, from, to, &sig, u("hold"), None, g("stop") / 100.0, rs)
        }
        _ => Vec::new(),
    }
}

/// (family id, family name, variant label) for the strategy list.
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    const CVR: &str = "Connors VIX Reversals (PTJ Market Timing)";
    Some(match id {
        "ptjmt_cvr1" => ("ptjmt_cvr", CVR, "CVR I · 15-day extreme"),
        "ptjmt_cvr2" => ("ptjmt_cvr", CVR, "CVR II · RSI(5) turn"),
        "ptjmt_cvr3" => ("ptjmt_cvr", CVR, "CVR III · 10% from the 10-day MA"),
        "ptjmt_cvr4" => ("ptjmt_cvr", CVR, "CVR IV · open-to-close 1.50 pts"),
        "ptjmt_cvr4_friday" => ("ptjmt_cvr", CVR, "CVR IV · Fridays only"),
        "ptjmt_cvr5" => ("ptjmt_cvr", CVR, "CVR V · range extreme"),
        "ptjmt_trin_reversals" => ("ptjmt_trinrev", "TRIN Reversals", "30-day · NYSE TRIN"),
        "ptjmt_trin_reversals_nse" => ("ptjmt_trinrev", "TRIN Reversals", "30-day · NSE TRIN"),
        "ptjmt_trin_reversals_20" => ("ptjmt_trinrev", "TRIN Reversals", "20-day · NYSE TRIN"),
        "ptjmt_trin_reversals_20_nse" => ("ptjmt_trinrev", "TRIN Reversals", "20-day · NSE TRIN"),
        "ptjmt_trin_thrusts" => ("ptjmt_trinthrust", "TRIN Thrusts", "NYSE TRIN"),
        "ptjmt_trin_thrusts_nse" => ("ptjmt_trinthrust", "TRIN Thrusts", "NSE TRIN"),
        "ptjmt_padi" => ("ptjmt_padi", "Percent Advance/Decline Indicator", "NYSE"),
        "ptjmt_padi_nse" => ("ptjmt_padi", "Percent Advance/Decline Indicator", "NSE"),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(d: usize, o: f64, h: f64, l: f64, c: f64) -> Row {
        Row { ticker: "T".into(), date: format!("2000-01-{:02}", d + 4), open: o, high: h, low: l, close: c, volume: None }
    }

    #[test]
    fn trin_thrust_buy_stopped_then_short_exits_after_2_days() {
        // bar 1: TRIN 1.0 -> 0.6 (-40%) buy at 100. bar 2: low 98 hits the 1.5% stop (98.5, open 99 above it);
        // TRIN 0.6 -> 0.9 (+50%) sells at 99 on the same close. bar 4: two days held, cover at 97.
        let r = vec![
            row(0, 100.0, 100.2, 99.8, 100.0),
            row(1, 100.0, 100.2, 99.8, 100.0),
            row(2, 99.0, 99.2, 98.0, 99.0),
            row(3, 98.0, 98.5, 97.5, 98.0),
            row(4, 97.0, 97.5, 96.5, 97.0),
        ];
        let mut x = Ctx::new(&r, &r, &[]);
        x.extra.insert("NYSE_TRIN".into(), vec![1.0, 0.6, 0.9, 0.9, 0.9]);
        let t = run_with("ptjmt_trin_thrusts", &x, &Vx::of(&x, 0), 0, 4, &Params::new());
        let got: Vec<_> = t.iter().map(|t| (t.dir, t.legs[0].i, t.legs[0].px, t.exit_i, t.exit_px, t.open)).collect();
        assert_eq!(got, vec![(1.0, 1, 100.0, 2, 98.5, false), (-1.0, 2, 99.0, 4, 97.0, false)]);
    }

    #[test]
    fn cvr4_friday_only_and_next_open_exit() {
        // bar 2 = Thu 2000-01-06, bar 3 = Fri 2000-01-07. VIX closes 2 pts above its open on both: only Friday sells.
        let r: Vec<Row> = (0..5).map(|d| row(d, 100.0 + d as f64, 101.0 + d as f64, 99.0, 100.5 + d as f64)).collect();
        let vix: Vec<Row> = (0..5).map(|d| if d == 2 || d == 3 { row(d, 20.0, 22.5, 19.5, 22.0) } else { row(d, 20.0, 20.5, 19.5, 20.2) }).collect();
        let mut x = Ctx::new(&r, &r, &[]);
        for (k, f) in [("Open", 0), ("High", 1), ("Low", 2), ("Close", 3)] {
            x.extra.insert(format!("VIX.{k}"), vix.iter().map(|v| [v.open, v.high, v.low, v.close][f]).collect());
        }
        let t = run_with("ptjmt_cvr4_friday", &x, &Vx::of(&x, 0), 0, 4, &Params::new());
        assert!(friday("2000-01-07") && !friday("2000-01-06"));
        let got: Vec<_> = t.iter().map(|t| (t.dir, t.legs[0].i, t.exit_i, t.exit_px)).collect();
        assert_eq!(got, vec![(-1.0, 3, 4, 104.0)]);
        assert_eq!(run_with("ptjmt_cvr4", &x, &Vx::of(&x, 0), 0, 4, &Params::new()).len(), 2);
    }

    #[test]
    fn cvr5_close_exactly_on_the_third_line_signals() {
        // like SPY 1996-09-23: VIX low 16.32, high 16.74, close 16.46 = 16.32 + 0.42/3 (in the bottom third, inclusive)
        let r: Vec<Row> = (0..12).map(|d| row(d, 100.0, 101.0, 99.0, 100.0)).collect();
        let vix: Vec<Row> = (0..12).map(|d| if d == 10 { row(d, 16.32, 16.74, 16.32, 16.46) } else { row(d, 15.0, 15.5, 14.5, 15.0) }).collect();
        let mut x = Ctx::new(&r, &r, &[]);
        for (k, f) in [("Open", 0), ("High", 1), ("Low", 2), ("Close", 3)] {
            x.extra.insert(format!("VIX.{k}"), vix.iter().map(|v| [v.open, v.high, v.low, v.close][f]).collect());
        }
        let t = run_with("ptjmt_cvr5", &x, &Vx::of(&x, 0), 0, 11, &Params::new());
        assert_eq!(t.iter().map(|t| (t.dir, t.legs[0].i, t.exit_i)).collect::<Vec<_>>(), vec![(1.0, 10, 11)]);
    }

    #[test]
    fn restart_param_keeps_or_reenters_on_the_exit_bar() {
        // buy signals on bars 1 and 3; 2-day exit falls on bar 3. restart=1: one trade 1->5; restart=0: 1->3 then 3->5.
        let r: Vec<Row> = (0..7).map(|d| row(d, 100.0, 100.5, 99.5, 100.0)).collect();
        let mut x = Ctx::new(&r, &r, &[]);
        x.extra.insert("NYSE_TRIN".into(), vec![1.0, 0.6, 0.6, 0.4, 0.4, 0.4, 0.4]);
        let go = |p: &Params| run_with("ptjmt_trin_thrusts", &x, &Vx::of(&x, 0), 0, 6, p).iter().map(|t| (t.legs[0].i, t.exit_i)).collect::<Vec<_>>();
        assert_eq!(go(&Params::new()), vec![(1, 5)]);
        assert_eq!(go(&Params::from([("restart".to_string(), 0.0)])), vec![(1, 3), (3, 5)]);
    }
}
