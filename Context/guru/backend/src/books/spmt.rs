//! The TradingMarkets S&P Market Timing Course for E-mini & Options Traders (Sabo / Connors, Feb 2007). Book id "spmt".
//!
//! Book conventions (p.3): signals on SPY (and the VIX / NYSE breadth), trades on the E-mini, test 9/9/97 - 11/30/06,
//! every entry and exit on the close, 1/2 point slippage + commission. One position per strategy at a time.
//! Most single signals are the same rules as the 2010 Swing Trading College IX versions in connors.rs (ids without
//! the "spmt_" prefix); they are coded here again so this book's test period, claims and parameters stand on their own.
//!
//! External series (extra_data/us = NYSE, extra_data/india = NSE; market-wide files, so each server only sees its own exchange):
//!   `TRIN.csv`  daily closing Arms index of the exchange (NYSE for SPY, NSE for NIFTY)
//!   `ADV.csv`   advancing issues (count) of that exchange
//!   `TICK.csv`  daily closing TICK (upticks - downticks) of that exchange (none in the vault)
//!   `<TICKER>__M15.csv`   the ticker's 15-minute bars, one row per day: O01..O26, H01..., L01..., C01... (bar 01 = first 15 minutes)
#![allow(unused_imports, dead_code)]
use crate::connors::*;
use std::rc::Rc;

pub const BOOK: &str = "spmt";

/// Exact daily source contracts reviewed in library/connors-spmt/REVIEW.md.
/// These are SPMT variants of existing STC IX ideas, not twenty new algorithms.
pub const REVIEWED_DAILY: &[&str] = &[
    "spmt_eom_co", "spmt_eom_rsi", "spmt_eom_cc", "spmt_rsi2_4", "spmt_rsi4_25",
    "spmt_rsi_div8", "spmt_rsi_div2", "spmt_rsi_1", "spmt_rsi_2", "spmt_rsi_3",
    "spmt_pctb_1", "spmt_pctb_2", "spmt_price", "spmt_dmi_1", "spmt_dmi_2",
    "spmt_dmi_3", "spmt_dmi_4", "spmt_dmi_5", "spmt_short_4up", "spmt_short_strong",
];
/// Second admission (connors/spmt2-admission/SOURCE-CONTRACT.md): the VIX, NYSE/NSE TRIN and advancers rules and the
/// EOM put-sale underlying. Admission only: they keep the shared simulator they always used (the close-only runner
/// below stays keyed to REVIEWED_DAILY), so no calculation changes.
pub const REVIEWED_AUX: &[&str] = &[
    "spmt_eom_und", "spmt_vix200", "spmt_vix2", "spmt_vix4", "spmt_vix7", "spmt_vix8", "spmt_short_vix53", "spmt_short_vix101",
    "spmt_trin_1", "spmt_trin_2", "spmt_trin_3", "spmt_trin_4", "spmt_trin_5", "spmt_nyse_adv",
];
pub fn reviewed_daily(id: &str) -> bool { REVIEWED_DAILY.contains(&id) || REVIEWED_AUX.contains(&id) }
/// VIX (2) and VIX (7) read the VIX open (and high); the served CBOE VIX is close-only before this date.
pub const REAL_VIX_OHLC_FROM: &str = "1992-01-01";
pub fn reads_vix_ohlc(id: &str) -> bool { matches!(id, "spmt_vix2" | "spmt_vix7") }
/// Reviewed symbols. VIX rules: SPX/SPY on the CBOE VIX; NIFTY 50 on India VIX only for close-only VIX rules (India VIX
/// opens repeat the previous close). TRIN / advancers: SPX/SPY on NYSE breadth, NIFTY 50 on NSE breadth (declared).
/// Price-only rules keep the existing index/SPY scope.
pub fn admitted_symbol(id: &str, ticker: &str) -> bool {
    match id {
        "spmt_vix2" | "spmt_vix7" => matches!(ticker, "SPX" | "SPY"),
        "spmt_vix200" | "spmt_vix4" | "spmt_vix8" | "spmt_short_vix53" | "spmt_short_vix101" | "spmt_trin_1" | "spmt_trin_2"
        | "spmt_trin_3" | "spmt_trin_4" | "spmt_trin_5" | "spmt_nyse_adv" => matches!(ticker, "SPX" | "SPY" | "NIFTY 50"),
        _ => true,
    }
}

/// The approved batch supplies price series only. A partially observed signal
/// basket must never silently become the source's complete basket.
pub fn hold_reason(id: &str) -> Option<&'static str> {
    match id {
        "spmt_tick" => Some("SPMT TICK needs the daily closing NYSE TICK; the vault holds no TICK history (US or NSE), so this rule cannot run."),
        "spmt_multi5" | "spmt_multi9" | "spmt_mts" => Some("The complete SPMT signal basket needs VIX, TRIN, advancers and TICK. The vault has no NYSE or NSE TICK history, so the basket is always incomplete; counting only the available signals changes the strategy. The partial basket is held."),
        // spmt_eom_puts: options phase 2 (real NSE option prices); the other two need the Market Timing signal count / E-mini legs
        "spmt_put_sale" | "spmt_itm_calls" => Some("Real SPY and NIFTY option prices are now loaded, but these trades are taken on the course's Market Timing buy and sell signals (the 30-signal count, which includes the NYSE TICK the vault does not have), and the call version adds SPY/E-mini legs by the number of signals; the strike is a range ('2-3 strikes'). Counting only some signals would be a different system, so it is held."),
        "spmt_dt_gapdown" | "spmt_dt_unch" | "spmt_dt_gapup" => Some("Intraday methods are outside the approved daily batch. Actual 15-minute execution data and source-supported entry/stop ordering are required; invented ticks, currency conversions and daily paths are held."),
        "spmt_discretion" => Some("The source leaves ranking and allocation discretionary; a complete mechanical portfolio contract is unavailable."),
        _ => None,
    }
}

/// The book's 30 long Market Timing signals in RadarScreen order (p.241), then the 4 short signals.
const LONG: &[&str] = &[
    "spmt_eom_co", "spmt_eom_rsi", "spmt_eom_cc", "spmt_rsi_1", "spmt_rsi_2", "spmt_rsi_3", "spmt_rsi2_4", "spmt_rsi4_25", "spmt_rsi_div8",
    "spmt_rsi_div2", "spmt_pctb_1", "spmt_pctb_2", "spmt_price", "spmt_vix200", "spmt_vix2", "spmt_vix4", "spmt_vix7", "spmt_vix8",
    "spmt_trin_1", "spmt_trin_2", "spmt_trin_3", "spmt_trin_4", "spmt_trin_5", "spmt_dmi_1", "spmt_dmi_2", "spmt_dmi_3", "spmt_dmi_4",
    "spmt_dmi_5", "spmt_nyse_adv", "spmt_tick",
];
const SHORT: &[&str] = &["spmt_short_vix53", "spmt_short_vix101", "spmt_short_4up", "spmt_short_strong"];
const DT: &[&str] = &["spmt_dt_gapdown", "spmt_dt_unch", "spmt_dt_gapup"];

const PERIOD: &str = "9/9/97 - 11/30/06, E-mini (signals on SPY)";
const RSI_EXIT: &str = "X|Exit on the close when the RSI 2-period of SPY closes above 65";
const SHORT_EXIT: &str = "X|Exit (cover) on the close when SPY closes below its 5-day moving average";
const TRIN_NOTE: &str = "Reads the series TRIN = the daily closing Arms index ((adv issues/decl issues)/(adv volume/decl volume)) of the ticker's exchange: NYSE for US tickers (extra_data/us/TRIN.csv, rebuilt from unicorn.us.com NYSE breadth, 1965 to 2020-02-10; NYSE trading days only, 3 trading days missing: 1999-11-26, 2002-06-20, 2019-08-20, each breaks an N-days-in-a-row run), NSE for the NIFTY indices (extra_data/india/TRIN.csv, rebuilt from NSE breadth). [Not from the book] The book's TRIN is the TradeStation/CQG $TRIN feed; the rebuilt series differs on some borderline days (e.g. 27 Nov 2006: book chart ~1.78, rebuilt 1.53), which explains TRIN 2-5 having 3-8 more trades than the book. India: the book's 1.0/1.1/1.2 levels are applied unchanged to the NSE TRIN. \"Above\" = strictly greater; the run of days ends on the trigger day. The 2010 course lists TRIN without code (stc9 trin* are needs-data).";

// ---------------------------------------------------------------- catalogue

pub fn catalogue() -> Vec<StrategyDef> {
    use Status::*;
    let d = |id, name, group, dir, status, needs_series, rules, notes, claim| StrategyDef {
        id, name, group, session: 0, dir, status, needs_volume: false, needs_vix: false, needs_series, book: BOOK, rules, notes, claim,
    };
    let vx = |s: StrategyDef| StrategyDef { needs_vix: true, ..s };
    let cl = |t, w, a, h| claim(t, w, a, h, PERIOD);
    let none: &'static [&'static str] = &[];
    let trin: &'static [&'static str] = &["TRIN"];
    let m15: &'static [&'static str] = &["M15.O01", "M15.H01", "M15.L01", "M15.C01"];
    let (g1, g2, g3, g4, g5, g6, g7, g8, g9) = (
        "I · End of the Month", "II · RSI Strategies", "III · VIX Strategies", "IV · %b Strategies", "V · Price Strategy",
        "VI · TRIN Strategies", "VII · DMI Strategies", "VIII · Short Strategies", "IX · Additional Strategies",
    );
    let mut v = vec![
        // ---- I. End of the Month (p.9-20, 168-170)
        d("spmt_eom_co", "EOM · Close < Open", g1, "Long", Live, none,
          &["F|Close of SPY is above its 200-day moving average", "F|It is the 25th or later in the month", "E|Buy after 2 or more closes where the close is below the open", "E|RSI(2) is less than or equal to 65", "E|Buy on the close", RSI_EXIT],
          "STC IX eom_co: identical rules (that course tested 1995-2009). \"2 or more closes below the open\" read as consecutive, ending today (p.18 says consecutive for the parallel rule). Day = calendar day of month. RadarScreen EOM(1). India: the same rules on the NIFTY indices.",
          cl("42", "90.48%", "0.79%", "4.1 days")),
        d("spmt_eom_rsi", "EOM · RSI", g1, "Long", Live, none,
          &["F|Close of SPY is above its 200-day moving average", "F|It is the 25th or later in the month", "E|RSI 2-period closes below 25", "E|Buy on the close", "X|Exit when RSI 2-period closes above 65"],
          "STC IX eom_rsi: identical rules. RadarScreen EOM(2). India: the same rules on the NIFTY indices.", cl("39", "89.74%", "0.86%", "4.3 days")),
        d("spmt_eom_cc", "EOM · Close < Close", g1, "Long", Live, none,
          &["F|SPY closes above its 200-day moving average", "F|It is the 25th or later in the month", "E|SPY closes 2 or more consecutive days below its previous day's close", "E|RSI(2) is less than or equal to 65", "E|Buy on the close", "X|Exit when RSI 2-period of the SPY closes above 65"],
          "STC IX eom_cc: identical rules. RadarScreen EOM(3). India: the same rules on the NIFTY indices.", cl("39", "84.62%", "0.76%", "4.1 days")),
        d("spmt_eom_und", "EOM put-sale signal · underlying", g1, "Long", Live, none,
          &["F|Market is above the 200-day MA", "E|On the 25th day of the month or later, the market drops two days in a row", "E|Signal at the close (the book sells SPY puts ~2 strikes / 2% out of the money here; this is the underlying)", "X|RSI 2-period closes above 65: exit at the next morning's open (\"close to open\")"],
          "p.168 options strategy measured on the underlying (\"89% correct on the underlying\", test 1/1/95-11/30/06). The exit level is not restated on p.168; the p.169 example exits on RSI(2) = 76.23 \"for the next morning\", read as the course's RSI(2) > 65 exit taken at the next open. No RSI(2) <= 65 entry cap (p.168 has none). Corresponds to STC IX eom_cc without the cap and with a next-open exit. India: NIFTY 50 / NIFTY BANK underlying.",
          claim("57 signals", "89%", "-", "-", "1/1/95 - 11/30/06, SPY underlying")),
        d("spmt_eom_puts", "EOM · sell SPY puts", g1, "Long", Live, none,
          &["F|Market is above the 200-day MA", "E|On the 25th day of the month or later, if the market drops two days in a row, sell the closest month SPY put options which are approximately two strike prices away (2%)", "X|Exit on the RSI exit signal (next morning)"],
          "Options phase 2: real NSE option prices, no pricing model. Signal and exit = spmt_eom_und (p.168-169): entry at the signal close, exit at the next morning's open after RSI(2) closes above 65. Put strike = the listed strike nearest 2% below the close (tie: lower) [convention: the book's 'approximately two strike prices away (2%)'; on SPY two $1 strikes were about 1.4% in 2006, on NIFTY two strikes are 0.2-1%, so the stated 2% is used, the farther and more conservative reading]. Expiry = the closest monthly expiry whose last trading day is after the entry day. Sold at the close and bought back at the next morning's open, each only if the put traded (Contracts > 0); a put with no traded open is bought back at that day's traded close, else the next traded close; a put held to its last trading day settles at intrinsic value on the index close. The open put is valued daily at its own price (traded close, else the exchange settlement price, labelled). Return = put points / index close at entry. India only (NIFTY 50, NIFTY BANK; declared application): the US option data (SPY 2008-2025, the book's instrument) has no option open, so the next-morning exit cannot be priced and SPY is held.",
          claim("57 signals", "89% (underlying)", "-", "-", "1/1/95 - 11/30/06; no position closed in the money")),
        // ---- II. RSI (p.21-49)
        d("spmt_rsi2_4", "RSI(2) < 4", g2, "Long", Live, none,
          &["F|SPY closes above its 200-day moving average", "E|RSI 2-period of the SPY closes below 4", "E|Buy on the close", "X|Exit when 2-period RSI of SPY closes above 65"],
          "STC IX rsi2_4: identical rules. RadarScreen RSI(4) (p.172 log). India: the same rules on the NIFTY indices.", cl("22", "81.82%", "1.30%", "4.7 days")),
        d("spmt_rsi4_25", "RSI(4) < 25", g2, "Long", Live, none,
          &["F|Close of SPY is above its 200-day moving average", "E|The 4-period RSI of SPY closes below 25", "E|RSI(2) is less than or equal to 65", "E|Buy at the close", "X|Exit when the 2-period RSI closes above 65"],
          "STC IX rsi4_25: identical rules. RadarScreen RSI(5) (p.172 log). India: the same rules on the NIFTY indices.", cl("52", "75%", "0.96%", "5.2 days")),
        d("spmt_rsi_div8", "RSI Div 8-day", g2, "Long", Live, none,
          &["F|SPY closes above its 200-day moving average", "E|SPY today closes below its close 8 days ago", "E|RSI 2-period of SPY today is greater than RSI(2) 8 days ago", "E|Today's RSI 2-period of SPY closes below 15", "E|Buy on the close", "X|Exit when RSI 2-period of SPY closes above 65"],
          "STC IX rsi_div8: identical rules. \"8 days ago\" = Close[8]. India: the same rules on the NIFTY indices.", cl("7", "100%", "1.18%", "5 days")),
        d("spmt_rsi_div2", "RSI Div 2-day", g2, "Long", Live, none,
          &["F|SPY closes above its 200-day moving average", "E|SPY close today is less than its close 2 days ago", "E|RSI 2-period of SPY today is greater than RSI 2-period of SPY 2 days ago", "E|Today's RSI 2-period of SPY closes below 30", "E|Buy on the close", "X|Exit when RSI 2-period of SPY closes above 65"],
          "STC IX rsi_div2: identical rules. India: the same rules on the NIFTY indices.", cl("28", "75%", "0.55%", "4.9 days")),
        d("spmt_rsi_1", "RSI-1 · 3-day RSI(2) sum < 50", g2, "Long", Live, none,
          &["F|SPY closes above its 200-day moving average", "E|The sum of the RSI 2-period of the SPY added together for 3 consecutive days is less than 50", "E|Buy on the close", "X|Exit when the RSI 2-period of the SPY closes above 65"],
          "STC IX rsi_1: identical rules. Sum = RSI2[0] + RSI2[1] + RSI2[2]. RadarScreen RSI(1) (p.21 name, p.172 log). India: the same rules on the NIFTY indices.", cl("38", "84.21%", "1.09%", "4.6 days")),
        d("spmt_rsi_2", "RSI-2 · 2-day RSI(2) sum < 40", g2, "Long", Live, none,
          &["F|SPY closes above its 200-day moving average", "E|The sum of the RSI 2-period of the SPY added together for 2 consecutive days is less than 40", "E|Buy on the close", "X|Exit when the RSI 2-period of the SPY closes above 65"],
          "STC IX rsi_2: identical rules. RadarScreen RSI(2). India: the same rules on the NIFTY indices.", cl("79", "78.48%", "0.57%", "4.8 days")),
        d("spmt_rsi_3", "RSI-3 · 2-day RSI(3) sum < 45", g2, "Long", Live, none,
          &["F|SPY closes above its 200-day moving average", "E|The sum of the RSI 3-period of the SPY added together for 2 consecutive days is less than 45", "E|Buy on the close", "X|Exit when the RSI 2-period of the SPY closes above 65"],
          "STC IX rsi_3: identical rules. Entry uses RSI(3), exit RSI(2), as written. RadarScreen RSI(3). India: the same rules on the NIFTY indices.", cl("51", "80.39%", "0.92%", "4.9 days")),
        // ---- III. VIX (p.50-72)
        vx(d("spmt_vix200", "VIX 200 · stretched 5% for 3 days", g3, "Long", Live, none,
          &["F|SPY closes above its 200-day moving average", "E|VIX is stretched 5% or more above its 10-day moving average for 3 or more consecutive days", "E|The RSI 2-period of the SPY is below 10 on the trigger day", "E|Buy on the close", "X|Exit when the RSI 2-period on the SPY closes above 65"],
          "STC IX vix200: identical rules. Stretch = VIX close >= 1.05 x SMA10(VIX close). The VIX MA length (10) is fixed by the engine's VIX alignment. RadarScreen VIX(1). India: India VIX in place of the CBOE VIX (window starts at the first India VIX bar).", cl("22", "81.82%", "1.13%", "4.8 days"))),
        vx(d("spmt_vix2", "VIX (2) · VIX RSI(2) > 90 + open above close", g3, "Long", Live, none,
          &["F|SPY closes above its 200-day moving average", "E|RSI 2-period of the VIX is above 90", "E|VIX today opens above yesterday's close", "E|RSI 2-period of the SPY is below 30", "E|Buy on the close", "X|Exit when 2-period RSI of the SPY closes above 65"],
          "STC IX vix2: identical rules. Needs the VIX open. India: India VIX.", cl("56", "78.57%", "0.77%", "4.5 days"))),
        vx(d("spmt_vix4", "VIX (4) · VIX up 5 days", g3, "Long", Live, none,
          &["F|SPY closes above its 200-day moving average", "E|VIX closes up 5 days in a row", "E|RSI 2-period of the SPY is below 30 on trigger day", "E|Buy on the close", "X|Exit when RSI 2-period of SPY closes above 65"],
          "STC IX vix4: identical rules. India: India VIX.", cl("10", "80%", "1.14%", "4.4 days"))),
        vx(d("spmt_vix7", "VIX (7) · up 4 days + gap over high", g3, "Long", Live, none,
          &["F|SPY closes above its 200-day moving average", "E|VIX closes up 4 days in a row", "E|Today's VIX opens greater than yesterday's high (gap up)", "E|Today the RSI 2-period on the SPY is below 30", "E|Buy on the close", "X|Exit when the RSI 2-period of the SPY closes above 65"],
          "STC IX vix7: identical rules. India: India VIX. NIFTY gets 0 trades, as expected: the vault builds India VIX daily bars from 1-minute data, and the minute-built prior-day high almost always exceeds the next open (8 opens above the prior high in 2,897 days, none after 4 up days), so \"open > yesterday's high\" practically never happens. This is a data artefact, not a code fault.", cl("11", "81.82%", "1.38%", "4 days"))),
        vx(d("spmt_vix8", "VIX (8) · stretched 10% for 2 days", g3, "Long", Live, none,
          &["F|SPY closes above its 200-day moving average", "E|VIX is stretched 10% or more above its 10-day moving average for 2 or more days in a row", "E|RSI 2-period of the SPY is below 30", "E|Buy on the close", "X|Exit when the RSI 2-period of the SPY closes above 65"],
          "STC IX vix8: identical rules. India: India VIX.", cl("19", "73.68%", "0.44%", "4.7 days"))),
        // ---- IV. %b (p.73-81)
        d("spmt_pctb_1", "%b < 0, 2 days + RSI", g4, "Long", Live, none,
          &["F|SPY closes above its 200-day moving average", "E|5-period %b with 1 STD is less than 0 for 2 or more days in a row", "E|RSI 2-period of SPY is less than 10", "E|Buy on the close", "X|Exit when 2-period RSI of SPY closes above 65"],
          "STC IX pctb_1: identical rules. %b = (C - lower)/(upper - lower), bands = SMA5 +/- 1 population std dev (TradeStation StdDev). RadarScreen PercB(1). India: the same rules on the NIFTY indices.", cl("52", "80.77%", "0.91%", "4.8 days")),
        d("spmt_pctb_2", "%b < 0.2 + Bottom", g4, "Long", Live, none,
          &["F|SPY closes above its 200-day moving average", "E|5-period %b with 1 STD closes below 0.2 for 3 or more consecutive days", "E|The SPY on this trigger day closes in the bottom 25% of its daily range", "E|RSI(2) is less than or equal to 65", "E|Buy on the close", "X|Exit when the RSI 2-period of the SPY closes above 65"],
          "STC IX pctb_2: identical rules. Bottom 25% = (C-L)/(H-L) <= 0.25. RadarScreen PercB(2). India: the same rules on the NIFTY indices.", cl("39", "84.62%", "1.21%", "4.8 days")),
        // ---- V. Price (p.82-86)
        d("spmt_price", "Close < Open 3 days, bottom 10% of range", g5, "Long", Live, none,
          &["F|SPY closes above its 200-day moving average", "E|Close is less than open for 3 or more consecutive days", "E|On trigger day, SPY must close in the bottom 10% of its daily range", "E|RSI(2) is less than or equal to 65", "E|Buy on the close", "X|Exit when the RSI 2-period of the SPY closes above 65"],
          "STC IX price: identical rules. The p.85 results slide is titled \"%b Less Than 0 Bottom 10% Range Results\" (slide-title error); it sits between this strategy's rules and example. RadarScreen Down(1). India: the same rules on the NIFTY indices.", cl("25", "92%", "0.79%", "4.8 days")),
    ];
    // ---- VI. TRIN (p.87-111). Names on p.87: TRIN, TRIN4day1.2(1), TRIN3day1.0(3), TRIN3day1.1(4), TRIN2day1.1(5); plain TRIN = TRIN(2).
    for (id, name, rule, c) in [
        ("spmt_trin_1", "TRIN · 4 days > 1.0 · TRIN(2)", "E|TRIN closes above 1.0 four days in a row", cl("33", "81.82%", "0.86%", "4.2 days")),
        ("spmt_trin_2", "TRIN 4-day 1.2 · TRIN(1)", "E|TRIN closes above 1.2 four days in a row", cl("5", "100%", "1.12%", "4.0 days")),
        ("spmt_trin_3", "TRIN 3-day 1.0 · TRIN(3)", "E|TRIN closes above 1.0 three days in a row", cl("57", "70.18%", "0.39%", "4.8 days")),
        ("spmt_trin_4", "TRIN 3-day 1.1 · TRIN(4)", "E|TRIN closes above 1.1 three days in a row", cl("32", "71.88%", "0.72%", "4.5 days")),
        ("spmt_trin_5", "TRIN 2-day 1.1 · TRIN(5)", "E|TRIN closes above 1.1 two days in a row", cl("74", "72.97%", "0.40%", "4.6 days")),
    ] {
        let rules: &'static [&'static str] = Box::leak(Box::new(["F|SPY closes above its 200-day moving average", rule, "E|RSI 2-period of the SPY is below 50 on trigger day", "E|Buy on the close", "X|Exit when the RSI 2-period of the SPY closes above 65"]));
        v.push(d(id, name, g6, "Long", Live, trin, rules, TRIN_NOTE, c));
    }
    // ---- VII. DMI (p.112-132)
    for (id, name, rule, c) in [
        ("spmt_dmi_1", "DMI(1) · +DMI(3) < 5", "E|+DMI 3-period closes less than 5", cl("56", "76.79%", "0.83%", "4.5 days")),
        ("spmt_dmi_2", "DMI(2) · +DMI(2) < 3", "E|+DMI 2-period closes less than 3", cl("86", "73.26%", "0.69%", "4.5 days")),
        ("spmt_dmi_3", "DMI(3) · +DMI(2) < 10, 2 days", "E|+DMI 2-period closes below 10 for 2 days in a row", cl("109", "72.48%", "0.46%", "4.7 days")),
        ("spmt_dmi_4", "DMI(4) · +DMI(2) < 10, 3 days", "E|+DMI 2-period closes below 10 for 3 days in a row", cl("76", "73.68%", "0.61%", "4.4 days")),
        ("spmt_dmi_5", "DMI(5) · +DMI(2) < 15, 4 days", "E|+DMI 2-period closes below 15 for 4 days in a row", cl("67", "74.63%", "0.64%", "4.4 days")),
    ] {
        let rules: &'static [&'static str] = Box::leak(Box::new(["F|SPY closes above its 200-day moving average", rule, "E|RSI(2) is less than or equal to 65", "E|Buy on the close", "X|Exit when the RSI 2-period of the SPY closes above 65"]));
        v.push(d(id, name, g7, "Long", Live, none, rules,
          "STC IX dmi_1..dmi_5 (same number): identical rules. +DMI = Wilder +DI (TradeStation DMIPlus). India: the same rules on the NIFTY indices.", c));
    }
    v.extend([
        // ---- VIII. Shorts (p.133-150)
        vx(d("spmt_short_vix53", "Short · VIX 5.3", g8, "Short", Live, none,
          &["F|SPY closes below its 200-day moving average", "E|VIX closes 5% or more below its 10-day moving average for 3 or more consecutive days", "E|RSI 2-period of the SPY is greater than 80 on trigger day", "E|Close is greater than or equal to its 5-day MA", "E|Sell short on the close", SHORT_EXIT],
          "STC IX vix53_short: identical rules. RadarScreen Short-VIX(1). India: India VIX on NIFTY.", cl("22", "77.27%", "0.88%", "4.5 days"))),
        vx(d("spmt_short_vix101", "Short · VIX 10.1", g8, "Short", Live, none,
          &["F|SPY closes below its 200-day moving average", "E|VIX closes 10% or more below its 10-day moving average for 1 or more days", "E|RSI 2-period of the SPY is greater than 80 on trigger day", "E|Close is greater than or equal to its 5-day MA", "E|Sell short on the close", SHORT_EXIT],
          "STC IX vix101_short: identical rules. RadarScreen Short-VIX(2). India: India VIX on NIFTY.", cl("20", "65.00%", "0.31%", "5.2 days"))),
        d("spmt_short_4up", "Short · 4 Up Days", g8, "Short", Live, none,
          &["F|SPY closes below its 200-day moving average", "E|SPY closes up four or more consecutive days in a row", "E|Close is greater than or equal to its 5-day MA", "E|Sell short on the close", SHORT_EXIT],
          "STC IX short_4up: identical rules. Up = close > previous close. RadarScreen Short-Up(1). India: the same rules on the NIFTY indices.", cl("21", "76.19%", "0.76%", "4.5 days")),
        d("spmt_short_strong", "Short · Strong 2-day", g8, "Short", Live, none,
          &["F|SPY closes below its 200-day moving average", "E|Two-day return on SPY is greater than 3%", "E|RSI 2-period of the SPY is greater than 80 on trigger day", "E|Close is greater than or equal to its 5-day MA", "E|Sell short on the close", SHORT_EXIT],
          "STC IX short_strong: identical rules. Two-day return = C/C[2] - 1. RadarScreen Short-Up(2). India: the same rules on the NIFTY indices.", cl("29", "75.86%", "0.74%", "4.9 days")),
        // ---- IX. Additional (p.151-161)
        d("spmt_nyse_adv", "NYSE Advancers", g9, "Long", Live, &["ADV"],
          &["F|SPY closes above its 200-day moving average", "E|2-period RSI of the 7-period MA of the NYSE advancers for the last 2 days added together are less than 12", "E|RSI(2) is less than or equal to 65", "E|Buy on the close", "X|Exit when the 2-period RSI of the SPY is greater than 65"],
          "Reads the series ADV = advancing issues of the ticker's exchange (NYSE for US tickers: extra_data/us/ADV.csv, unicorn.us.com, ends 2020-02-10; NSE for the NIFTY indices: extra_data/india/ADV.csv). R = RSI2(SMA7(ADV)), condition R + R[1] < 12, computed over the ticker's trading days on which the series exists (exchange-holiday rows in a breadth file never enter the average). Not in the 2010 course. India: NSE advancing issues; RSI is scale-free so 12 stays.",
          cl("52", "84.62%", "0.83%", "4.1 days")),
        d("spmt_tick", "NYSE TICK", g9, "Long", Live, &["TICK"],
          &["F|SPY closes above its 200-day moving average", "E|5-day moving average of NYSE Tick closes below 150 for 3 days in a row", "E|RSI 2-period of the SPY is less than or equal to 75 on trigger day", "E|Buy on the close", "X|Exit when the RSI 2-period of the SPY closes above 75"],
          "DATA GAP: the vault has no NYSE TICK history (no free source found), so this strategy never runs and is not a verified Live result; the code is reviewed only. Reads the series TICK = the daily closing TICK reading (upticks - downticks at the close) of the exchange (extra_data/us/TICK.csv = NYSE $TICK close). Rule taken as printed (\"below 150\", the p.161 chart's red line). No NSE TICK exists, so India stays a data gap. Not in the 2010 course.",
          cl("52", "82.69%", "0.72%", "5.2 days")),
        // ---- Multiple signals and the Market Timing System (p.6, p.164-167, p.198-200)
        d("spmt_multi5", "Five or more signals in one day", "Market Timing System", "Long", Live, none,
          &["E|Five or more of the course's long Market Timing signals trigger on the same day", "E|Buy on the close", "X|Exit when the RSI 2-period of SPY closes above 65"],
          "p.6 \"Long Only Multiple Signals\". Signal = one of the 30 long strategies (EOM 1-3, RSI 1-7, PercB 1-2, Down 1, VIX x5, TRIN 1-5, DMI 1-5, NYSE_Adv, Tick 1) taking a new entry that day (each run with one position at a time and book parameters). Every qualifying day is one of the book's trades; as they all exit on the same RSI(2) > 65 close they are held as equal legs of one position (max_legs, 6 = no cap on SPY 1997-2006), so book stats compare per leg. TRIN / ADV / TICK signals count only where those series exist for the ticker. The p.199 table (1-7 signals) is this rule with min signals = N (its row 5 equals the p.6 result). Why new entries and not raw conditions: the book's own day counts match new entries exactly and never match raw conditions (SPY): p.273, 27 Nov 2006 \"6 buy signals [2 EOM, 2 RSI, 1 %b, 1 TRIN]\" = 6 new entries (raw 7); p.283, 22 Dec 2006 \"9 signals ... 3 RSI, 1 %b, 1 VIX, 1 price, 1 DMI, 2 TRIN\" = 9 by family (raw 12); p.298, 1 Nov \"1 RSI, 1 %b and 2 DMI\", \"third day of signals\" = 4, 3rd signal day (raw 8); p.305, 2 Nov \"6 signals ... the 4th day\" = 6, 4th day (raw 9, and raw would make it the 5th day). p.raw=1 keeps the raw-condition reading.",
          claim("79", "84%", "0.99%", "4.5 days", "E-mini; PF 10.40, max loser -26.25 pts")),
        d("spmt_multi9", "Nine or more cumulative signals", "Market Timing System", "Long", Live, none,
          &["E|The cumulative count of long Market Timing signals reaches nine or more, and today triggers at least one new signal", "E|Buy on the close", "X|Exit when the RSI 2-period of SPY closes above 65"],
          "p.6. Cumulative = signals since the last day RSI(2) closed above 65 (the run of signal days a position is held through). Every day with cumulative >= 9 and a new signal is one trade (a leg, as in spmt_multi5). Same signal set as spmt_multi5. Why new entries and not raw conditions: the book's own day counts match new entries exactly and never match raw conditions (SPY): p.273, 27 Nov 2006 \"6 buy signals [2 EOM, 2 RSI, 1 %b, 1 TRIN]\" = 6 new entries (raw 7); p.283, 22 Dec 2006 \"9 signals ... 3 RSI, 1 %b, 1 VIX, 1 price, 1 DMI, 2 TRIN\" = 9 by family (raw 12); p.298, 1 Nov \"1 RSI, 1 %b and 2 DMI\", \"third day of signals\" = 4, 3rd signal day (raw 8); p.305, 2 Nov \"6 signals ... the 4th day\" = 6, 4th day (raw 9, and raw would make it the 5th day). p.raw=1 keeps the raw-condition reading.",
          claim("132", "83%", "0.99%", "4.2 days", "E-mini; PF 14.03, max loser -21.75 pts")),
        d("spmt_mts", "Market Timing System (scale-in)", "Market Timing System", "Long/Short", Live, none,
          &["E|On the first day of signals buy one E-mini, even if you get multiple signals today", "A|Anytime you are still in the first position, buy two more contracts when you have five or more cumulative signals, as long as today triggers at least one new signal", "A|On any day following your second entry, if you get a cumulative total of eight or more signals buy three more contracts (today triggers at least one new signal)", "A|On any day after the third buy, with 11 or more cumulative signals buy four more contracts (total of 10 contracts)", "E|Sells: every day you get a (short) signal you sell one contract, four contracts max", "X|Longs: the whole position exits on the close when RSI(2) of SPY closes above 65", "X|Shorts: cover on the close when SPY closes below its 5-day moving average"],
          "p.164-167. Long weights 1/2/3/4 of 10 contracts; short 1/4 per contract. Signals = new entries of the 30 long (4 short) single strategies, cumulative since the first buy. The book does not state the exit; every p.172 position exits together on the RSI(2) > 65 day, so the stack exits as one. One stack at a time (a short signal while long is ignored). Signal definition: see spmt_multi5 (p.273/283/298/305 day counts match new entries). COMPARING WITH THE BOOK: the book's figures are per entry (each buy day is one trade; 414 entries, 694 contracts), while the stats card counts positions (a whole stack, all legs exiting together, is one trade: 210 on SPY 9/97-11/06), so the card's trade count and win rate are not like for like with the claim. Ours per entry (verify_spmt.py section 3, 9/9/97-11/30/06, long + short): S&P 500 cash (GSPC) 414 entries / 740 contracts / 78.0% / 0.65% / 4.7 days; SPY 424 / 739 / 80.7% / 0.78% / 4.5 days.",
          claim("414 entries / 694 contracts (per entry; card = positions)", "76% (per entry)", "0.58% (per entry)", "4.5 days", "\"since 1997\", E-mini; PF 4.64, 2.7 trades/month")),
        // ---- Day trading (p.240-315), 15-minute bars
        d("spmt_dt_gapdown", "Day trade · gap-down open", "Day Trading (15-minute)", "Long", Live, m15,
          &["F|Buy bias: one or more Market Timing buy signals triggered yesterday", "F|Today opens below yesterday's close (gap down)", "E|Let the market trade for 15 minutes and mark the high", "E|If the market takes out this first fifteen minute high, buy (one tick above the high)", "X|Initial stop just under the low of the same (first) fifteen minute bar if filled right away; later fills use the entry bar low", "X|Trail: sell when a 15-minute bar closes below the prior bar's low", "X|If stopped out just under the first fifteen minute bar, re-enter if the market moves above the first 15-minute high again; try up to three times", "X|Flat on the close"],
          "Reads the ticker's 15-minute bars (series M15.O01..M15.C26, bar 01 = 9:30-9:45 ET / 9:15-9:30 IST). Gap down = open more than the unchanged tolerance (0.14% = $0.20 on SPY at ~140) below the prior close. Tick = 0.02% of price (one E-mini tick, 0.25 at ~1400). Fills: buy stop at max(open, trigger); stop at min(open, stop); a fill and a stop in the same bar = stopped (conservative). Re-entry (p.254 rule 6): only after a stop \"just under the first fifteen minute bar\", read as a stop at or below the first bar's low (the first-bar-low stop, or an entry-bar stop whose bar low is at/under the first bar's low); after a higher entry-bar stop or a trailing exit the day is over (rule 5's support/Fibonacci re-entry is discretionary and not coded). [Not from the book] p.reentry_any=1 re-enters after any stop (the earlier reading). Tick: [Not from the book] default 0.02% of price; p.tick_abs sets an absolute tick instead (0.01 for SPY, 0.25 for the E-mini, 0.05 for NIFTY). Size scaling by signal strength not coded (fixed size). DATA GAP: the vault has no real 15-minute bars (the only M15 file is a private synthetic 3-day test), so this strategy is not a verified Live result.",
          None),
        d("spmt_dt_unch", "Day trade · unchanged open", "Day Trading (15-minute)", "Long", Live, m15,
          &["F|Buy bias: one or more Market Timing buy signals triggered yesterday", "F|The market opens unchanged from the previous day's close (a mild gap-up can be played like the unchanged open)", "E|Let the market trade for 15 minutes; buy when it takes out the first fifteen minute high", "X|Initial stop just under the low of the same fifteen minute bar", "X|Trail: sell when a 15-minute bar closes below the prior bar's low", "X|If stopped out just under the first fifteen minute bar, re-enter above the first 15-minute high; up to three tries", "X|Flat on the close"],
          "Unchanged = open from 0.14% below the prior close up to a gap-up that is not \"substantial\" (<= 0.36% = $0.50 on SPY at ~140); p.259 plays mild gap-ups like the unchanged open, gaps between mild and substantial have no rule and are played the same way. Same mechanics as spmt_dt_gapdown (p.257 rule 8: same re-entry rule, same p.reentry_any / p.tick_abs switches). DATA GAP: no real 15-minute bars in the vault.",
          None),
        d("spmt_dt_gapup", "Day trade · gap-up open (buy zone)", "Day Trading (15-minute)", "Long", Live, m15,
          &["F|Buy bias: one or more Market Timing buy signals triggered yesterday", "F|The gap-up is substantial (above fifty cents on the SPY / 2 points on the E-mini)", "E|Buy only after a pullback: resting buy orders in a buy zone at yesterday's close, split into 4 pieces one tick apart", "E|If very bullish (many signals), also buy the unfilled pieces when the market takes out the first fifteen minute high", "X|Initial stop just under the previous day's low if it is not too far away; otherwise the low of the entry bar. A breakout fill moves the stop under that bar's low", "X|Trail: sell when a 15-minute bar closes below the prior bar's low", "X|Flat on the close"],
          "Substantial gap-up = open > 0.36% above the prior close ($0.50 on SPY at ~140). Buy zone = prior close, -1, -2, -3 ticks, 1/4 position each (the p.260 example uses 4 pieces). \"Very bullish\" = 5 or more cumulative signals (the system's add level; the book leaves it to judgement). \"Too far\" = more than 0.5% below the first fill (not in the book; parameter). Re-entry via Fibonacci (rule 7) is discretionary and not coded. [Not from the book] p.tick_abs sets an absolute tick (0.01 SPY, 0.25 E-mini, 0.05 NIFTY) instead of 0.02% of price. DATA GAP: no real 15-minute bars in the vault.",
          None),
        // ---- Options and discretion (not testable)
        d("spmt_put_sale", "Naked puts / bull put spreads on buy signals", "Options", "Long", NotTestable("needs option chains"), none,
          &["E|Sell naked puts (front month, near the money) when we get our Market Timing buy signals, or sell a near-strike put and buy an out-of-the-money put (bull put spread)", "X|Buy it back when we get our Market Timing sell signal"],
          "p.216-218, 222, 230: used after several days of signals and many cumulative signals. Example: sell SPY 142 put for 1.75, buy 137 put for 0.60, max loss 3.85.", None),
        d("spmt_itm_calls", "Combining options and E-minis", "Options", "Long", NotTestable("needs option chains"), none,
          &["E|One signal (only) the first day: buy ITM SPY call options (2-3 strikes below current) at the close", "A|Two or more signals the first day: ITM calls plus a small SPY or E-mini position", "A|More signals on day two: buy more SPY's and/or E-minis", "A|More signals on day three or later: sell volatility (naked puts or bull put spreads)", "X|Exit is with the RSI signal"],
          "p.219-222.", None),
        d("spmt_discretion", "Discretionary signal weighting (Paul Sabo)", "Discretionary Trading", "Long", NotTestable("discretionary"), none,
          &["E|Check signals 10 minutes before the close; if no signals trigger, do nothing", "E|Rank: EOM, VIX, RSI, %b, DMI, TRIN strategies", "A|More signals and more days of signals = more aggressive (up to \"all in\" with 6-7 signals on the third day)", "A|Position size: five equal allocations; half an allocation for one weak (TRIN) signal, two or three for several strong signals"],
          "p.189-231, 268-269: the book says it is \"purposely left loose\". Mechanical approximations: spmt_multi5, spmt_multi9, spmt_mts.", None),
    ]);
    v
}

// ---------------------------------------------------------------- params

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    let trend = || p("trend", "Trend filter: SMA length", 200.0, 100.0, 300.0, 25.0);
    let rsi_n = || p("rsi_n", "RSI period (exit / caps)", 2.0, 2.0, 5.0, 1.0);
    let ex = |def| p("exit_rsi", "Exit: RSI closes above", def, 50.0, 85.0, 5.0);
    let lt = |def, min, max, step| p("rsi_lt", "Entry: RSI below", def, min, max, step);
    let gt = || p("rsi_gt", "Entry: RSI above", 80.0, 60.0, 95.0, 5.0);
    let days = |def| p("days", "…for N days in a row", def, 1.0, 6.0, 1.0);
    let ma = || p("ma", "Exit: short SMA length", 5.0, 3.0, 10.0, 1.0);
    let dom = || p("dom", "Day of month from", 25.0, 15.0, 28.0, 1.0);
    let legs = || p("max_legs", "Max signal days per position (equal units)", 6.0, 1.0, 10.0, 1.0);
    let raw = || p("raw", "Count raw conditions (1) not new entries (0)", 0.0, 0.0, 1.0, 1.0);
    let long = |mut extra: Vec<ParamDef>| {
        let mut v = vec![trend(), rsi_n(), ex(65.0)];
        v.append(&mut extra);
        v
    };
    let sum = |n, d, t| vec![p("sum_n", "Summed RSI period", n, 2.0, 5.0, 1.0), p("sum_days", "Sum RSI over N days", d, 2.0, 5.0, 1.0), p("sum_lt", "RSI sum below", t, 20.0, 100.0, 5.0)];
    let b = id.strip_prefix("spmt_")?;
    Some(match b {
        "eom_co" | "eom_cc" | "eom_und" => long(vec![dom(), p("bars", if b == "eom_co" { "Closes below the open in a row" } else { "Lower closes in a row" }, 2.0, 1.0, 4.0, 1.0)]),
        "eom_puts" => long(vec![dom(), p("bars", "Lower closes in a row", 2.0, 1.0, 4.0, 1.0), p("otm", "Put strike: % below the close (book: about 2%)", 2.0, 0.0, 5.0, 0.5)]),
        "eom_rsi" => long(vec![dom(), lt(25.0, 10.0, 40.0, 5.0)]),
        "rsi2_4" => long(vec![lt(4.0, 1.0, 15.0, 1.0)]),
        "rsi4_25" => long(vec![p("slow_n", "Second RSI period", 4.0, 3.0, 6.0, 1.0), p("slow_lt", "Second RSI below", 25.0, 10.0, 40.0, 5.0)]),
        "rsi_div8" => long(vec![p("look", "Divergence lookback (days)", 8.0, 3.0, 15.0, 1.0), lt(15.0, 5.0, 30.0, 5.0)]),
        "rsi_div2" => long(vec![p("look", "Divergence lookback (days)", 2.0, 2.0, 8.0, 1.0), lt(30.0, 10.0, 40.0, 5.0)]),
        "rsi_1" => long(sum(2.0, 3.0, 50.0)),
        "rsi_2" => long(sum(2.0, 2.0, 40.0)),
        "rsi_3" => long(sum(3.0, 2.0, 45.0)),
        "pctb_1" | "pctb_2" => {
            let one = b == "pctb_1";
            let mut v = vec![p("pctb_n", "%b period", 5.0, 3.0, 10.0, 1.0), p("pctb_k", "%b band width (std devs)", 1.0, 0.5, 2.5, 0.5), days(if one { 2.0 } else { 3.0 }),
                p("pctb_lt", "%b below", if one { 0.0 } else { 0.2 }, -0.2, 0.5, 0.1)];
            v.push(if one { lt(10.0, 5.0, 30.0, 5.0) } else { p("loc", "Close in bottom % of range", 25.0, 10.0, 50.0, 5.0) });
            long(v)
        }
        "price" => long(vec![p("bars", "Closes below the open in a row", 3.0, 1.0, 5.0, 1.0), p("loc", "Close in bottom % of range", 10.0, 5.0, 30.0, 5.0)]),
        "vix200" | "vix8" => {
            let (s, d, r) = if b == "vix200" { (5.0, 3.0, 10.0) } else { (10.0, 2.0, 30.0) };
            long(vec![p("stretch", "VIX % above its 10-day SMA", s, 2.0, 15.0, 1.0), days(d), lt(r, 5.0, 40.0, 5.0)])
        }
        "vix2" => long(vec![p("vix_rsi", "VIX RSI(2) above", 90.0, 70.0, 98.0, 2.0), lt(30.0, 10.0, 40.0, 5.0)]),
        "vix4" | "vix7" => long(vec![p("up", "VIX up closes in a row", if b == "vix4" { 5.0 } else { 4.0 }, 2.0, 7.0, 1.0), lt(30.0, 10.0, 40.0, 5.0)]),
        "trin_1" | "trin_2" | "trin_3" | "trin_4" | "trin_5" => {
            let (t, d) = match b { "trin_1" => (1.0, 4.0), "trin_2" => (1.2, 4.0), "trin_3" => (1.0, 3.0), "trin_4" => (1.1, 3.0), _ => (1.1, 2.0) };
            long(vec![p("trin_gt", "TRIN closes above", t, 0.8, 2.0, 0.1), days(d), lt(50.0, 20.0, 70.0, 5.0)])
        }
        "dmi_1" | "dmi_2" | "dmi_3" | "dmi_4" | "dmi_5" => {
            let (n, d, t) = match b { "dmi_1" => (3.0, 1.0, 5.0), "dmi_2" => (2.0, 1.0, 3.0), "dmi_3" => (2.0, 2.0, 10.0), "dmi_4" => (2.0, 3.0, 10.0), _ => (2.0, 4.0, 15.0) };
            long(vec![p("di_n", "+DMI period", n, 2.0, 10.0, 1.0), days(d), p("di_lt", "+DMI below", t, 1.0, 25.0, 1.0)])
        }
        "nyse_adv" => long(vec![p("adv_ma", "MA of advancers (days)", 7.0, 3.0, 15.0, 1.0), p("adv_rsi", "RSI period of the MA", 2.0, 2.0, 5.0, 1.0),
            p("sum_days", "Sum RSI over N days", 2.0, 1.0, 4.0, 1.0), p("sum_lt", "RSI sum below", 12.0, 4.0, 40.0, 2.0)]),
        "tick" => vec![trend(), rsi_n(), ex(75.0), p("tick_ma", "MA of TICK (days)", 5.0, 3.0, 10.0, 1.0), p("tick_lt", "TICK MA below", 150.0, -300.0, 400.0, 50.0), days(3.0)],
        "short_vix53" | "short_vix101" => {
            let (s, d) = if b == "short_vix53" { (5.0, 3.0) } else { (10.0, 1.0) };
            vec![trend(), p("stretch", "VIX % below its 10-day SMA", s, 2.0, 15.0, 1.0), days(d), rsi_n(), gt(), ma()]
        }
        "short_4up" => vec![trend(), p("up", "Up closes in a row", 4.0, 2.0, 7.0, 1.0), ma()],
        "short_strong" => vec![trend(), p("rise", "2-day rise above %", 3.0, 1.0, 6.0, 0.5), rsi_n(), gt(), ma()],
        "multi5" => vec![p("min_sig", "Signals on one day, at least", 5.0, 1.0, 10.0, 1.0), rsi_n(), ex(65.0), raw(), legs()],
        "multi9" => vec![p("min_cum", "Cumulative signals, at least", 9.0, 3.0, 20.0, 1.0), rsi_n(), ex(65.0), raw(), legs()],
        "mts" => vec![p("add2", "Add #2 at cumulative signals ≥", 5.0, 2.0, 10.0, 1.0), p("add3", "Add #3 at cumulative signals ≥", 8.0, 3.0, 15.0, 1.0), p("add4", "Add #4 at cumulative signals ≥", 11.0, 4.0, 20.0, 1.0),
            p("c1", "Contracts: first buy", 1.0, 1.0, 4.0, 1.0), p("c2", "Contracts: second buy", 2.0, 0.0, 4.0, 1.0), p("c3", "Contracts: third buy", 3.0, 0.0, 6.0, 1.0), p("c4", "Contracts: fourth buy", 4.0, 0.0, 8.0, 1.0),
            p("short_max", "Short contracts max", 4.0, 0.0, 8.0, 1.0), rsi_n(), ex(65.0), ma(), raw()],
        "dt_gapdown" | "dt_unch" | "dt_gapup" => {
            let mut v = vec![p("min_sig", "Bias: signals yesterday, at least", 1.0, 1.0, 10.0, 1.0), p("unch", "Unchanged open: within % below prior close", 0.14, 0.0, 0.5, 0.02),
                p("gap_up", "Substantial gap-up above %", 0.36, 0.1, 1.0, 0.02), p("tick", "One tick, % of price", 0.02, 0.005, 0.1, 0.005),
                p("tick_abs", "One tick in price units (0 = use %)", 0.0, 0.0, 1.0, 0.01)];
            v.extend(if b == "dt_gapup" {
                vec![p("pieces", "Buy-zone pieces (1 tick apart)", 4.0, 1.0, 8.0, 1.0), p("bull", "Very bullish: cumulative signals ≥", 5.0, 1.0, 15.0, 1.0), p("max_stop", "Prior-day-low stop if within % of entry", 0.5, 0.1, 2.0, 0.1)]
            } else {
                vec![p("tries", "Tries per day", 3.0, 1.0, 5.0, 1.0), p("reentry_any", "Re-enter after any stop (1) / only a stop under the first bar (0, book)", 0.0, 0.0, 1.0, 1.0)]
            });
            v
        }
        _ => return None,
    })
}

fn getter<'p>(id: &str, p: &'p Params) -> impl Fn(&str) -> f64 + 'p {
    let schema = params(id).unwrap_or_default();
    let id = id.to_string();
    move |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).unwrap_or_else(|| panic!("{id} has no param {k}")).def)
}

// ---------------------------------------------------------------- signals

type V = Rc<Vec<f64>>;
type Cond<'a> = Box<dyn Fn(usize) -> bool + 'a>;
fn own(c: std::borrow::Cow<'_, [f64]>) -> V {
    Rc::new(c.into_owned())
}

/// f applied to the finite values of v only (an external series may skip days), scattered back.
fn on_valid(v: &[f64], f: impl Fn(&[f64]) -> Vec<f64>) -> Vec<f64> {
    let idx: Vec<usize> = (0..v.len()).filter(|&i| v[i].is_finite()).collect();
    let r = f(&idx.iter().map(|&i| v[i]).collect::<Vec<_>>());
    let mut out = vec![f64::NAN; v.len()];
    for (k, &i) in idx.iter().enumerate() {
        out[i] = r[k];
    }
    out
}

fn all(i: usize, d: usize, f: impl Fn(usize) -> bool) -> bool {
    i + 1 >= d && (i + 1 - d..=i).all(f)
}

/// Dominant positive directional move with a subtraction-roundoff tie budget.
/// Equal decimal high/low moves can differ after f64 subtraction. Treat only
/// differences within the input arithmetic's error bound as ties, not as +DM.
/// This fixes the source convention locally; STC IX's shared DMI is untouched.
fn positive_dm(today: &crate::Row, prior: &crate::Row) -> f64 {
    let up = today.high - prior.high;
    let down = prior.low - today.low;
    let error = f64::EPSILON * (today.high.abs() + prior.high.abs() + prior.low.abs() + today.low.abs());
    if up > 0.0 && up - down > error { up } else { 0.0 }
}

fn daily_pdi(rows: &[crate::Row], n: usize) -> Vec<f64> {
    let mut dm = vec![f64::NAN; rows.len()];
    let mut tr = true_range(rows);
    if !tr.is_empty() { tr[0] = f64::NAN; }
    for i in 1..rows.len() { dm[i] = positive_dm(&rows[i], &rows[i - 1]); }
    let (sm, st) = (wilder(&dm, n), wilder(&tr, n));
    sm.iter().zip(st).map(|(&m, t)| if t > 0.0 { 100.0 * m / t } else { f64::NAN }).collect()
}

/// Full entry condition (trend filter included) of one single-signal strategy.
fn cond<'a>(id: &str, x: &'a Ctx, g: &dyn Fn(&str) -> f64) -> Option<Cond<'a>> {
    let u = |k: &str| g(k).round().max(1.0) as usize;
    let c: &'a [f64] = &x.c;
    let b = id.strip_prefix("spmt_")?;
    let tr = own(x.sma_n(u("trend")));
    let never: Cond<'a> = Box::new(|_| false);
    if b.starts_with("short_") {
        let ma = own(x.sma_n(u("ma")));
        let setup: Cond<'a> = match b {
            "short_4up" => {
                let up = u("up");
                Box::new(move |i| x.up_closes[i] >= up)
            }
            "short_strong" => {
                let (rs, rise, gt) = (own(x.rsi_n(u("rsi_n"))), g("rise") / 100.0, g("rsi_gt"));
                // Compare the price with the percentage boundary directly:
                // 103/100-1 can round above .03 and accept exact equality.
                Box::new(move |i| i >= 2 && c[i] > c[i - 2] * (1.0 + rise) && rs[i] > gt)
            }
            "short_vix53" | "short_vix101" => {
                let (rs, s, d, gt) = (own(x.rsi_n(u("rsi_n"))), x.vix.stretched(-g("stretch")), u("days"), g("rsi_gt"));
                Box::new(move |i| s[i] >= d && rs[i] > gt)
            }
            _ => return None,
        };
        return Some(Box::new(move |i| c[i] < tr[i] && setup(i) && c[i] >= ma[i]));
    }
    let rs = own(x.rsi_n(u("rsi_n")));
    let ex = g("exit_rsi");
    let cap = {
        let rs = rs.clone();
        move |i: usize| rs[i] <= ex
    };
    let dom = |i: usize| x.r[i].date[8..10].parse::<u32>().unwrap_or(0);
    let f: Cond<'a> = match b {
        "eom_co" | "eom_cc" | "eom_und" => {
            let (d, n) = (g("dom") as u32, u("bars"));
            let bars = if b == "eom_co" { &x.red_bars } else { &x.down_closes };
            let capped = b != "eom_und";
            Box::new(move |i| dom(i) >= d && bars[i] >= n && (!capped || cap(i)))
        }
        "eom_rsi" => {
            let (d, lt) = (g("dom") as u32, g("rsi_lt"));
            Box::new(move |i| dom(i) >= d && rs[i] < lt)
        }
        "rsi2_4" => {
            let lt = g("rsi_lt");
            Box::new(move |i| rs[i] < lt)
        }
        "rsi4_25" => {
            let (r4, lt) = (own(x.rsi_n(u("slow_n"))), g("slow_lt"));
            Box::new(move |i| r4[i] < lt && cap(i))
        }
        "rsi_div8" | "rsi_div2" => {
            let (k, lt) = (u("look"), g("rsi_lt"));
            Box::new(move |i| i >= k && c[i] < c[i - k] && rs[i] > rs[i - k] && rs[i] < lt)
        }
        "rsi_1" | "rsi_2" | "rsi_3" => {
            let (r, d, lt) = (own(x.rsi_n(u("sum_n"))), u("sum_days"), g("sum_lt"));
            Box::new(move |i| i + 1 >= d && (0..d).map(|k| r[i - k]).sum::<f64>() < lt)
        }
        "pctb_1" | "pctb_2" => {
            let (n, k) = (u("pctb_n"), g("pctb_k"));
            let pb = if n == 5 && k == 1.0 { own(x.pctb_n(5)) } else { Rc::new(pct_b(c, n, k)) };
            let (d, t) = (u("days"), g("pctb_lt"));
            if b == "pctb_1" {
                let lt = g("rsi_lt");
                Box::new(move |i| all(i, d, |k| pb[k] < t) && rs[i] < lt)
            } else {
                let loc = g("loc") / 100.0;
                Box::new(move |i| all(i, d, |k| pb[k] < t) && x.close_loc(i) <= loc && cap(i))
            }
        }
        "price" => {
            let (n, loc) = (u("bars"), g("loc") / 100.0);
            Box::new(move |i| x.red_bars[i] >= n && x.close_loc(i) <= loc && cap(i))
        }
        "vix200" | "vix8" => {
            let (s, d, lt) = (x.vix.stretched(g("stretch")), u("days"), g("rsi_lt"));
            Box::new(move |i| s[i] >= d && rs[i] < lt)
        }
        "vix2" => {
            let (v, lt) = (g("vix_rsi"), g("rsi_lt"));
            Box::new(move |i| x.vix.rsi2[i] > v && x.vix.gap_over_close[i] && rs[i] < lt)
        }
        "vix4" | "vix7" => {
            let (up, lt, gap) = (u("up"), g("rsi_lt"), b == "vix7");
            Box::new(move |i| x.vix.up[i] >= up && (!gap || x.vix.gap_over_high[i]) && rs[i] < lt)
        }
        "trin_1" | "trin_2" | "trin_3" | "trin_4" | "trin_5" => match x.series("TRIN") {
            Some(t) => {
                let (gt, d, lt) = (g("trin_gt"), u("days"), g("rsi_lt"));
                Box::new(move |i| all(i, d, |k| t[k] > gt) && rs[i] < lt)
            }
            None => never,
        },
        "dmi_1" | "dmi_2" | "dmi_3" | "dmi_4" | "dmi_5" => {
            let (di, d, t) = (Rc::new(daily_pdi(x.r, u("di_n"))), u("days"), g("di_lt"));
            Box::new(move |i| all(i, d, |k| di[k] < t) && cap(i))
        }
        "nyse_adv" => match x.series("ADV") {
            Some(a) => {
                let (m, n, d, lt) = (u("adv_ma"), u("adv_rsi"), u("sum_days"), g("sum_lt"));
                let r = on_valid(a, |v| {
                    let mut out = vec![f64::NAN; v.len()];
                    if v.len() >= m {
                        out[m - 1..].copy_from_slice(&rsi(&sma(v, m)[m - 1..], n));
                    }
                    out
                });
                Box::new(move |i| i + 1 >= d && (0..d).map(|k| r[i - k]).sum::<f64>() < lt && cap(i))
            }
            None => never,
        },
        "tick" => match x.series("TICK") {
            Some(t) => {
                let (m, lt, d) = (u("tick_ma"), g("tick_lt"), u("days"));
                let tm = on_valid(t, |v| sma(v, m));
                Box::new(move |i| all(i, d, |k| tm[k] < lt) && cap(i))
            }
            None => never,
        },
        _ => return None,
    };
    Some(Box::new(move |i| c[i] > tr[i] && f(i)))
}

/// One single-signal strategy, one position at a time, entries and exits on the close.
fn single(id: &str, x: &Ctx, from: usize, to: usize, g: &dyn Fn(&str) -> f64) -> Option<Vec<Trade>> {
    let e = cond(id, x, g)?;
    if REVIEWED_DAILY.contains(&id) {
        return Some(daily_closes(id, x, from, to, g, &*e));
    }
    let u = |k: &str| g(k).round().max(1.0) as usize;
    let c = &x.c;
    if id.starts_with("spmt_short_") {
        let (ma, m) = (own(x.sma_n(u("ma"))), u("ma"));
        let exit = |i: usize, _: &Trade| (c[i] < ma[i]).then(|| Exit::Close(format!("Close < {m}-day SMA")));
        return Some(simulate(x, &Spec { dir: -1.0, weights: &[1.0], entry: &|i| e(i).then_some(Entry::Close), add: &no_add, exit: &exit }, from, to));
    }
    let (rs, n, ex) = (own(x.rsi_n(u("rsi_n"))), u("rsi_n"), g("exit_rsi"));
    let next_open = id == "spmt_eom_und";
    let exit = |i: usize, _: &Trade| {
        (rs[i] > ex).then(|| if next_open { Exit::NextOpen(format!("RSI({n}) > {ex}, out next open")) } else { Exit::Close(format!("RSI({n}) > {ex}")) })
    };
    Some(simulate(x, &Spec { dir: 1.0, weights: &[1.0], entry: &|i| e(i).then_some(Entry::Close), add: &no_add, exit: &exit }, from, to))
}

/// Close-only source execution, scoped here rather than changing the shared
/// simulator. A signal on the final close is a real open position; it is not
/// silently discarded because the research window has ended. No entry on an
/// exit close, one position per method, and no forced terminal liquidation.
fn daily_closes(id: &str, x: &Ctx, from: usize, to: usize, g: &dyn Fn(&str) -> f64, entry: &dyn Fn(usize) -> bool) -> Vec<Trade> {
    if from > to || to >= x.r.len() { return vec![]; }
    let short = id.starts_with("spmt_short_");
    let n = g(if short { "ma" } else { "rsi_n" }).round().max(1.0) as usize;
    let values = if short { own(x.sma_n(n)) } else { own(x.rsi_n(n)) };
    let threshold = if short { 0.0 } else { g("exit_rsi") };
    let mut cur: Option<Trade> = None;
    let mut out = Vec::new();
    for i in from..=to {
        if let Some(t) = cur.as_mut() {
            let exit = if short { x.c[i] < values[i] } else { values[i] > threshold };
            if exit {
                t.exit_i = i; t.exit_px = x.c[i]; t.signal_i = i;
                t.reason = if short { format!("Close < {n}-day SMA") } else { format!("RSI({n}) > {threshold}") };
                out.push(cur.take().unwrap());
            }
        } else if entry(i) {
            cur = Some(Trade { dir: if short { -1.0 } else { 1.0 },
                legs: vec![Leg { i, px: x.c[i], w: 1.0 }], exit_i: i, exit_px: x.c[i],
                reason: String::new(), entry_kind: "close", signal_i: i, open: false, exits: vec![] });
        }
    }
    if let Some(mut t) = cur {
        t.exit_i = to; t.exit_px = x.c[to]; t.signal_i = to;
        t.open = true; t.reason = "open".into(); out.push(t);
    }
    out
}

/// Audit raw setup conditions as well as filled records. The separate Python
/// oracle checks held-position signals, warmup and final-bar handling.
pub fn daily_signals(id: &str, x: &Ctx, p: &Params) -> Option<Vec<bool>> {
    if !REVIEWED_DAILY.contains(&id) { return None; }
    let g = getter(id, p);
    let e = cond(id, x, &g)?;
    Some((0..x.r.len()).map(e).collect())
}

/// Signals per bar from a set of single strategies (book parameters): new entries (raw = false) or raw entry conditions.
fn count(ids: &[&str], x: &Ctx, from: usize, to: usize, raw: bool) -> Vec<u32> {
    let mut n = vec![0u32; x.c.len()];
    let none = Params::new();
    for id in ids {
        let g = getter(id, &none);
        if raw {
            let e = cond(id, x, &g).unwrap();
            (from..=to).filter(|&i| e(i)).for_each(|i| n[i] += 1);
        } else {
            for t in single(id, x, from, to, &g).unwrap() {
                n[t.legs[0].i] += 1;
            }
        }
    }
    n
}

/// Cumulative signals since the last close with RSI above the exit level.
fn cumulative(n: &[u32], rs: &[f64], ex: f64) -> Vec<u32> {
    let mut cum = vec![0u32; n.len()];
    for i in 0..n.len() {
        let prev = if i > 0 && !(rs[i - 1] > ex) { cum[i - 1] } else { 0 };
        cum[i] = prev + n[i];
    }
    cum
}

// ---------------------------------------------------------------- Market Timing System (p.164-167)

fn mts(x: &Ctx, from: usize, to: usize, g: &dyn Fn(&str) -> f64) -> Vec<Trade> {
    let u = |k: &str| g(k).round().max(1.0) as usize;
    let raw = g("raw") > 0.5;
    let (nl, ns) = (count(LONG, x, from, to, raw), count(SHORT, x, from, to, raw));
    let (rs, n, ex, ma, m) = (own(x.rsi_n(u("rsi_n"))), u("rsi_n"), g("exit_rsi"), own(x.sma_n(u("ma"))), u("ma"));
    let th = [g("add2"), g("add3"), g("add4")];
    let size = [g("c1"), g("c2"), g("c3"), g("c4")];
    let total: f64 = size.iter().sum();
    let smax = g("short_max").round().max(0.0) as usize;
    let c = &x.c;
    let leg = |i: usize, w: f64| Leg { i, px: c[i], w };
    let mut out = Vec::new();
    let mut cur: Option<(Trade, f64, usize)> = None; // (trade, cumulative signals, buys made)
    for i in from..=to {
        if let Some((t, cum, buys)) = cur.as_mut() {
            let close = if t.dir > 0.0 { (rs[i] > ex).then(|| format!("RSI({n}) > {ex}")) } else { (c[i] < ma[i]).then(|| format!("Close < {m}-day SMA")) };
            if let Some(why) = close {
                t.exit_i = i;
                t.exit_px = c[i];
                t.signal_i = i;
                t.reason = why;
                out.push(cur.take().unwrap().0);
                continue;
            }
            if t.dir > 0.0 {
                *cum += nl[i] as f64;
                if nl[i] >= 1 && *buys < 4 && *cum >= th[*buys - 1] {
                    if size[*buys] > 0.0 {
                        t.legs.push(leg(i, size[*buys] / total));
                    }
                    *buys += 1;
                }
            } else if ns[i] >= 1 && t.legs.len() < smax {
                t.legs.push(leg(i, 1.0 / smax as f64));
            }
            continue;
        }
        if i == to {
            break;
        }
        let new = |dir: f64, w: f64| Trade { dir, legs: vec![leg(i, w)], exit_i: i, exit_px: c[i], reason: String::new(), entry_kind: "close", signal_i: i, open: false, exits: Vec::new() };
        if nl[i] >= 1 {
            cur = Some((new(1.0, size[0] / total), nl[i] as f64, 1));
        } else if ns[i] >= 1 && smax > 0 {
            cur = Some((new(-1.0, 1.0 / smax as f64), 0.0, 1));
        }
    }
    if let Some((mut t, _, _)) = cur {
        t.exit_i = to;
        t.exit_px = c[to];
        t.reason = "open".into();
        t.signal_i = to;
        t.open = true;
        out.push(t);
    }
    out
}

// ---------------------------------------------------------------- day trading on 15-minute bars (p.248-267)

#[derive(Clone, Copy, Debug)]
pub struct Bar {
    pub o: f64,
    pub h: f64,
    pub l: f64,
    pub c: f64,
}

/// The day's 15-minute bars from the M15.* columns (bar 01 = first 15 minutes); stops at the first missing bar.
fn day_bars(x: &Ctx, i: usize) -> Vec<Bar> {
    let get = |f: &str, k: usize| x.extra.get(&format!("M15.{f}{k:02}")).map_or(f64::NAN, |s| s[i]);
    (1..=40)
        .map(|k| Bar { o: get("O", k), h: get("H", k), l: get("L", k), c: get("C", k) })
        .take_while(|b| b.o.is_finite() && b.h.is_finite() && b.l.is_finite() && b.c.is_finite())
        .collect()
}

/// Gap-down / unchanged-open setup on one day: buy stop one tick above the first bar's high, initial stop one tick under
/// the first bar's low (fill in bar 2) or under the entry bar's low (later fill, active from the next bar), trail = exit when
/// a bar closes below the prior bar's low, re-enter after a stop-out when price trades above the trigger again (up to
/// `tries` entries), flat at the close. Re-entry only after a stop under the first bar's low (p.254 rule 6) unless `any_stop`.
/// Returns (entry bar, entry px, exit px, reason).
pub fn breakout_day(b: &[Bar], tick: f64, tries: usize, any_stop: bool) -> Vec<(usize, f64, f64, &'static str)> {
    let mut out = Vec::new();
    if b.len() < 2 {
        return out;
    }
    let trig = b[0].h + tick;
    let mut k = 1;
    while k < b.len() && out.len() < tries {
        if b[k].h < trig {
            k += 1;
            continue;
        }
        let px = b[k].o.max(trig);
        if k == 1 && b[k].l <= b[0].l - tick {
            // the stop is known during the fill bar: assume it filled after the entry (conservative)
            out.push((k, px, b[k].o.min(b[0].l - tick).min(px), "stop"));
            k += 1;
            continue;
        }
        let stop = if k == 1 { b[0].l - tick } else { b[k].l - tick };
        let mut j = k + 1;
        loop {
            if j == b.len() {
                out.push((k, px, b[j - 1].c, "close"));
                return out;
            }
            if b[j].l <= stop {
                out.push((k, px, b[j].o.min(stop), "stop"));
                if !any_stop && k > 1 && b[k].l > b[0].l {
                    return out; // stopped above the first bar's low: rule 6 does not re-enter
                }
                break;
            }
            if b[j].c < b[j - 1].l {
                out.push((k, px, b[j].c, "trail"));
                return out;
            }
            j += 1;
        }
        k = j + 1;
    }
    out
}

/// Gap-up setup on one day: `pieces` resting limit buys from the prior close down, one tick apart; if `bull`, the unfilled
/// pieces are bought on a break of the first bar's high. Stop under the prior day's low if within `max_stop` (fraction) of
/// the first fill, else under the first fill bar's low; a breakout fill moves it under that bar's low. Trail and close as in
/// breakout_day. Returns ((bar, px, weight) fills, exit px, reason), or None if nothing filled.
pub fn gapup_day(b: &[Bar], prev_close: f64, prev_low: f64, tick: f64, pieces: usize, bull: bool, max_stop: f64) -> Option<(Vec<(usize, f64, f64)>, f64, &'static str)> {
    let w = 1.0 / pieces as f64;
    let mut open: Vec<bool> = vec![true; pieces];
    let mut fills: Vec<(usize, f64, f64)> = Vec::new();
    let mut stop = f64::NAN;
    let trig = b.first()?.h + tick;
    for k in 0..b.len() {
        if !fills.is_empty() {
            if b[k].l <= stop {
                return Some((fills, b[k].o.min(stop), "stop"));
            }
            if k > 0 && b[k].c < b[k - 1].l {
                return Some((fills, b[k].c, "trail"));
            }
        }
        let had = !fills.is_empty();
        for j in 0..pieces {
            let lvl = prev_close - j as f64 * tick;
            if open[j] && b[k].l <= lvl {
                open[j] = false;
                fills.push((k, b[k].o.min(lvl), w));
            }
        }
        let mut moved = false;
        if bull && k > 0 && open.iter().any(|o| *o) && b[k].h >= trig {
            let left = open.iter().filter(|o| **o).count() as f64;
            open.iter_mut().for_each(|o| *o = false);
            fills.push((k, b[k].o.max(trig), w * left));
            stop = b[k].l - tick;
            moved = true;
        }
        if !had && !fills.is_empty() && !moved {
            let s = prev_low - tick;
            stop = if (fills[0].1 - s) / fills[0].1 <= max_stop { s } else { b[k].l - tick };
            if b[k].l <= stop {
                return Some((fills, b[k].o.min(stop), "stop"));
            }
        }
    }
    let last = b.last()?.c;
    (!fills.is_empty()).then(|| (fills, last, "close"))
}

fn day_trades(id: &str, x: &Ctx, from: usize, to: usize, g: &dyn Fn(&str) -> f64) -> Vec<Trade> {
    let u = |k: &str| g(k).round().max(1.0) as usize;
    let (unch, gap_up, tick_pct, min_sig) = (g("unch") / 100.0, g("gap_up") / 100.0, g("tick") / 100.0, u("min_sig") as u32);
    let start = from.max(1);
    let nl = count(LONG, x, start - 1, to, false);
    let day = |i: usize, legs: Vec<Leg>, px: f64, why: &str, kind: &'static str| Trade { dir: 1.0, legs, exit_i: i, exit_px: px, reason: why.to_string(), entry_kind: kind, signal_i: i, open: false, exits: Vec::new() };
    let mut out = Vec::new();
    let cum = cumulative(&nl, &x.rsi2, 65.0);
    for i in start..=to {
        let pc = x.c[i - 1];
        if nl[i - 1] < min_sig {
            continue;
        }
        let b = day_bars(x, i);
        let Some(gap) = b.first().map(|b0| b0.o / pc - 1.0) else { continue };
        let tick = if g("tick_abs") > 0.0 { g("tick_abs") } else { tick_pct * pc };
        match id {
            "spmt_dt_gapup" if gap > gap_up => {
                if let Some((fills, px, why)) = gapup_day(&b, pc, x.l[i - 1], tick, u("pieces"), cum[i - 1] as f64 >= g("bull"), g("max_stop") / 100.0) {
                    out.push(day(i, fills.iter().map(|&(_, p, w)| Leg { i, px: p, w }).collect(), px, why, "limit"));
                }
            }
            "spmt_dt_gapdown" | "spmt_dt_unch" if (id == "spmt_dt_gapdown") == (gap < -unch) && gap <= gap_up => {
                for (_, e, px, why) in breakout_day(&b, tick, u("tries"), g("reentry_any") > 0.5) {
                    out.push(day(i, vec![Leg { i, px: e, w: 1.0 }], px, why, "stop"));
                }
            }
            _ => {}
        }
    }
    out
}

// ---------------------------------------------------------------- run

/// Options phase 2: EOM put sale (p.168-169) on real NSE option prices; the signal/exit dates are spmt_eom_und's.
pub fn option_positions(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Result<Vec<crate::options::Pos>, String>> {
    use crate::options::{self as opt, Contract, Exit};
    if id != "spmt_eom_puts" { return None; }
    let g = getter(id, p);
    let Some(chain) = x.options else { return Some(Err(crate::books::oiss::NO_CHAIN.into())) };
    let sig = run("spmt_eom_und", x, from, to, p)?;
    let mut w = opt::Window::new(chain, x, 8);
    let mut out = Vec::new();
    let mut busy = 0;
    for t in sig {
        let i = t.legs[0].i;
        if i < busy { continue; }
        let today = opt::ymd(&x.r[i].date);
        let day = w.at(i);
        let pick = opt::expiries(day).into_iter().find(|&e| opt::ltd(x, e) > today).and_then(|e| {
            opt::nearest(&opt::strikes(day, e, true), x.c[i] * (1.0 - g("otm") / 100.0)).map(|strike| Contract { expiry: e, put: true, strike })
        });
        let Some((c, px0)) = pick.and_then(|c| w.quote(i, c).and_then(|q| q.fill_close()).map(|v| (c, v))) else { continue };
        let exit = (!t.open).then_some(t.exit_i);
        let (j, pnl, why, open, marks) = opt::follow(&mut w, x, c, -1.0, i, px0, to, &mut |k| {
            (Some(k) == exit).then(|| (Exit::Open, "RSI(2) > 65: out next open".to_string()))
        });
        busy = j + 1;
        out.push(opt::Pos { dir: 1.0, s0: x.c[i], entry_i: i, exit_i: j, pnl, open,
            reason: format!("{why} · sold {} put exp {} on {} at {px0:.2} · {}", c.strike, c.expiry, x.r[i].date, marks.text()) });
    }
    Some(Ok(out))
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if let Some(r) = option_positions(id, x, from, to, p) { return Some(crate::options::account(r.unwrap_or_default(), from, to, 0.0).trades); }
    params(id)?;
    let g = getter(id, p);
    let u = |k: &str| g(k).round().max(1.0) as usize;
    Some(match id {
        "spmt_multi5" | "spmt_multi9" => {
            let n = count(LONG, x, from, to, g("raw") > 0.5);
            let (rs, k, ex) = (own(x.rsi_n(u("rsi_n"))), u("rsi_n"), g("exit_rsi"));
            let entry: Box<dyn Fn(usize) -> bool> = if id == "spmt_multi5" {
                let m = g("min_sig") as u32;
                Box::new(move |i| n[i] >= m)
            } else {
                let (cum, m) = (cumulative(&n, &rs, ex), g("min_cum") as u32);
                Box::new(move |i| n[i] >= 1 && cum[i] >= m)
            };
            let exit = |i: usize, _: &Trade| (rs[i] > ex).then(|| Exit::Close(format!("RSI({k}) > {ex}")));
            // every qualifying day is one of the book's trades; they all exit on the same RSI close, so they are legs of one position
            let w = vec![1.0 / u("max_legs") as f64; u("max_legs")];
            simulate(x, &Spec { dir: 1.0, weights: &w, entry: &|i| entry(i).then_some(Entry::Close), add: &|i, _| entry(i), exit: &exit }, from, to)
        }
        "spmt_mts" => mts(x, from, to, &g),
        _ if DT.contains(&id) => day_trades(id, x, from, to, &g),
        _ => single(id, x, from, to, &g)?,
    })
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let b = id.strip_prefix("spmt_")?;
    let label = match b {
        "eom_co" => "Close < Open",
        "eom_rsi" => "RSI(2) < 25",
        "eom_cc" => "Close < Close",
        "eom_und" => "Put-sale signal (underlying)",
        "eom_puts" => "Sell SPY puts",
        "rsi2_4" => "RSI(2) < 4",
        "rsi4_25" => "RSI(4) < 25",
        "rsi_div8" => "Divergence 8-day",
        "rsi_div2" => "Divergence 2-day",
        "rsi_1" => "RSI-1 · 3-day sum < 50",
        "rsi_2" => "RSI-2 · 2-day sum < 40",
        "rsi_3" => "RSI-3 · RSI(3) 2-day sum < 45",
        "vix200" => "VIX 200",
        "vix2" => "VIX (2)",
        "vix4" => "VIX (4)",
        "vix7" => "VIX (7)",
        "vix8" => "VIX (8)",
        "pctb_1" => "%b < 0, 2 days + RSI",
        "pctb_2" => "%b < 0.2 + Bottom",
        "price" => "Close < Open 3 days, bottom 10%",
        "trin_1" => "4 days > 1.0",
        "trin_2" => "4 days > 1.2",
        "trin_3" => "3 days > 1.0",
        "trin_4" => "3 days > 1.1",
        "trin_5" => "2 days > 1.1",
        "dmi_1" => "DMI(1) · +DMI(3) < 5",
        "dmi_2" => "DMI(2) · +DMI(2) < 3",
        "dmi_3" => "DMI(3) · < 10, 2 days",
        "dmi_4" => "DMI(4) · < 10, 3 days",
        "dmi_5" => "DMI(5) · < 15, 4 days",
        "short_vix53" => "VIX 5.3",
        "short_vix101" => "VIX 10.1",
        "short_4up" => "4 Up Days",
        "short_strong" => "Strong 2-day",
        "nyse_adv" => "NYSE Advancers",
        "tick" => "NYSE TICK",
        "multi5" => "5+ signals in one day",
        "multi9" => "9+ cumulative signals",
        "mts" => "Scale-in system",
        "dt_gapdown" => "Gap-down open",
        "dt_unch" => "Unchanged open",
        "dt_gapup" => "Gap-up open",
        "put_sale" => "Naked puts / bull put spreads",
        "itm_calls" => "ITM calls + E-minis",
        _ => "",
    };
    let (fid, name) = match b {
        "eom_co" => ("spmt_eom_red", "SPMT · EOM Close below Open"),
        "eom_rsi" => ("spmt_eom_rsi", "SPMT · EOM RSI"),
        "eom_cc" | "eom_und" => ("spmt_eom_down", "SPMT · EOM Lower Closes"),
        "eom_puts" => ("spmt_options", "SPMT · Options"),
        "rsi_div8" | "rsi_div2" => ("spmt_rsi_divergence", "SPMT · RSI Divergence"),
        "rsi_1" | "rsi_2" | "rsi_3" => ("spmt_rsi_sum", "SPMT · Cumulative RSI"),
        "rsi2_4" | "rsi4_25" => ("spmt_rsi_level", "SPMT · RSI Level"),
        _ if b.starts_with("vix") => ("spmt_vix", "SPMT · VIX Strategies"),
        "pctb_1" => ("spmt_pctb_rsi", "SPMT · %b with RSI"),
        "pctb_2" => ("spmt_pctb_range", "SPMT · %b with Range Location"),
        "price" => ("spmt_price", "SPMT · Red Bars with Range Location"),
        _ if b.starts_with("trin") => ("spmt_trin", "SPMT · TRIN Strategies"),
        _ if b.starts_with("dmi") => ("spmt_dmi", "SPMT · DMI Strategies"),
        "short_4up" => ("spmt_short_up", "SPMT · Consecutive Up Closes"),
        "short_strong" => ("spmt_short_return", "SPMT · Strong Return with RSI"),
        "short_vix53" | "short_vix101" => ("spmt_short_vix", "SPMT · VIX Short"),
        "nyse_adv" => ("spmt_adv", "SPMT · NYSE Advancers RSI"),
        "tick" => ("spmt_tick", "SPMT · NYSE TICK"),
        "multi5" | "multi9" | "mts" => ("spmt_mts", "SPMT · Market Timing System"),
        _ if b.starts_with("dt_") => ("spmt_dt", "SPMT · Day Trading"),
        "put_sale" | "itm_calls" => ("spmt_options", "SPMT · Options"),
        "discretion" => ("spmt_discretion", "SPMT · Discretionary Trading", ),
        _ => return None,
    };
    Some((fid, name, label))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn daily_rows(n: usize) -> Vec<crate::Row> {
        (0..n).map(|i| crate::Row { ticker: "SPY".into(), date: format!("2020-01-{:02}", i + 1),
            open: 101.0, high: 102.0, low: 99.0, close: 100.0, volume: None }).collect()
    }

    #[test]
    fn daily_final_close_signal_is_an_open_position() {
        let r = daily_rows(3);
        let mut x = Ctx::new(&r, &r, &[]);
        x.ma200 = vec![110.0, 110.0, 90.0];
        x.rsi2 = vec![50.0, 50.0, 3.0];
        let t = run("spmt_rsi2_4", &x, 0, 2, &Params::new()).unwrap();
        assert_eq!(t.len(), 1);
        assert_eq!(t[0].legs[0].i, 2);
        assert!(t[0].open);
        assert_eq!(t[0].entry_kind, "close");
    }

    #[test]
    fn daily_strict_entry_exit_thresholds_and_sequential_positions() {
        let r = daily_rows(6);
        let mut x = Ctx::new(&r, &r, &[]);
        x.ma200 = vec![90.0; 6];
        x.rsi2 = vec![4.0, 3.0, 2.0, 65.0, 65.001, 3.0];
        let t = run("spmt_rsi2_4", &x, 0, 5, &Params::new()).unwrap();
        assert_eq!(t.len(), 2);
        assert_eq!((t[0].legs[0].i, t[0].exit_i), (1, 4));
        assert!(!t[0].open);
        assert!(t[1].open);
    }

    #[test]
    fn daily_eom_calendar_gate_cap_and_no_exit_bar_reentry() {
        let mut r = daily_rows(4);
        for (i, row) in r.iter_mut().enumerate() { row.date = format!("2020-01-{}", 24 + i); }
        let mut x = Ctx::new(&r, &r, &[]);
        x.ma200 = vec![90.0; 4];
        x.rsi2 = vec![65.0, 65.0, 66.0, 65.0];
        x.red_bars = vec![2; 4];
        let t = run("spmt_eom_co", &x, 0, 3, &Params::new()).unwrap();
        assert_eq!((t[0].legs[0].i, t[0].exit_i), (1, 2));
        assert_eq!(t[1].legs[0].i, 3);
    }

    #[test]
    fn daily_warmup_cannot_enter_and_zero_range_cannot_be_bottom() {
        let r = daily_rows(3);
        let x = Ctx::new(&r, &r, &[]);
        assert!(run("spmt_dmi_1", &x, 0, 2, &Params::new()).unwrap().is_empty());
        let mut r = daily_rows(4);
        r[3].open = 100.0; r[3].high = 100.0; r[3].low = 100.0;
        let mut x = Ctx::new(&r, &r, &[]);
        x.ma200 = vec![90.0; 4]; x.rsi2 = vec![70.0, 70.0, 70.0, 20.0]; x.red_bars = vec![3; 4];
        assert!(run("spmt_price", &x, 0, 3, &Params::new()).unwrap().is_empty());
    }

    #[test]
    fn daily_distinct_predicates_have_distinct_families() {
        assert_ne!(family_of("spmt_price").unwrap().0, family_of("spmt_pctb_2").unwrap().0);
        assert_ne!(family_of("spmt_rsi_div2").unwrap().0, family_of("spmt_rsi_2").unwrap().0);
        assert_eq!(family_of("spmt_dmi_1").unwrap().0, family_of("spmt_dmi_5").unwrap().0);
    }

    #[test]
    fn daily_directional_ties_do_not_create_positive_dm() {
        let mut r = daily_rows(2);
        r[0].high = 199.26; r[0].low = 197.95;
        r[1].high = 199.69; r[1].low = 197.52;
        assert_eq!(positive_dm(&r[1], &r[0]), 0.0);
        r[1].high += 0.00001;
        assert!(positive_dm(&r[1], &r[0]) > 0.43);
    }

    #[test]
    fn daily_short_accepts_ma_equality_but_covers_only_strictly_below() {
        let r = daily_rows(4);
        let mut x = Ctx::new(&r, &r, &[]);
        x.ma200 = vec![110.0; 4]; x.ma5 = vec![100.0, 100.0, 99.999, 100.001];
        x.up_closes = vec![4; 4];
        let t = run("spmt_short_4up", &x, 0, 3, &Params::new()).unwrap();
        assert_eq!(t.len(), 1); assert_eq!(t[0].dir, -1.0);
        assert_eq!((t[0].legs[0].i, t[0].exit_i), (0, 3)); assert!(!t[0].open);
        let t = run("spmt_short_4up", &x, 0, 2, &Params::new()).unwrap();
        assert!(t[0].open);
    }

    #[test]
    fn daily_strong_short_three_percent_equality_does_not_enter() {
        let mut r = daily_rows(3);
        r[0].close = 100.0; r[1].close = 101.0; r[2].close = 103.0;
        let mut x = Ctx::new(&r, &r, &[]);
        x.ma200 = vec![200.0; 3]; x.ma5 = vec![50.0; 3]; x.rsi2 = vec![90.0; 3];
        assert!(!daily_signals("spmt_short_strong", &x, &Params::new()).unwrap()[2]);
        x.c[2] = 103.00001;
        assert!(daily_signals("spmt_short_strong", &x, &Params::new()).unwrap()[2]);
    }

    fn bars(v: &[(f64, f64, f64, f64)]) -> Vec<Bar> {
        v.iter().map(|&(o, h, l, c)| Bar { o, h, l, c }).collect()
    }

    #[test]
    fn gap_down_breakout_trail_and_retry() {
        // first bar 100-101; bar 2 takes out 101 (+0.25 tick) -> buy 101.25, stop 99.75;
        // bar 3 holds, bar 4 closes 101.5 < bar 3 low 101.8 -> trail exit at that close
        let b = bars(&[(100.5, 101.0, 100.0, 100.2), (100.3, 101.5, 100.1, 101.4), (101.4, 102.5, 101.8, 102.3), (102.3, 102.4, 101.2, 101.5)]);
        assert_eq!(breakout_day(&b, 0.25, 3, false), vec![(1, 101.25, 101.5, "trail")]);
        // stopped in bar 3 (low 99.5 <= 99.75), re-bought when bar 4 trades above 101.25 again, flat at the close
        let b = bars(&[(100.5, 101.0, 100.0, 100.2), (100.3, 101.5, 100.1, 101.4), (101.0, 101.1, 99.5, 99.8), (99.8, 101.6, 99.8, 101.5), (101.5, 101.9, 101.4, 101.8)]);
        assert_eq!(breakout_day(&b, 0.25, 3, false), vec![(1, 101.25, 99.75, "stop"), (3, 101.25, 101.8, "close")]);
        // p.254 rule 6: fill in bar 3 (entry-bar low 100.5 is above the first bar's low 100.0), stopped at 100.25 in bar 4;
        // that stop is not "under the first fifteen minute bar", so no re-entry when bar 5 breaks out again (any_stop does)
        let b = bars(&[(100.5, 101.0, 100.0, 100.2), (100.3, 100.9, 100.1, 100.8), (100.8, 101.5, 100.5, 101.4), (101.0, 101.1, 100.2, 100.4), (100.4, 101.6, 100.4, 101.5)]);
        assert_eq!(breakout_day(&b, 0.25, 3, false), vec![(2, 101.25, 100.25, "stop")]);
        assert_eq!(breakout_day(&b, 0.25, 3, true), vec![(2, 101.25, 100.25, "stop"), (4, 101.25, 101.5, "close")]);
    }

    #[test]
    fn gap_up_buy_zone() {
        // prior close 100, low 99.6; opens 101 and pulls back to 99.7 in bar 2: pieces at 100 and 99.75 fill (99.5, 99.25 do not);
        // prior-low stop 99.35 is 0.65% away (> 0.5%) -> stop under the fill bar's low, 99.45; bar 3 holds; flat at the close 100.8
        let b = bars(&[(101.0, 101.2, 100.6, 100.7), (100.7, 100.8, 99.7, 100.1), (100.1, 100.9, 100.0, 100.8)]);
        let (f, px, why) = gapup_day(&b, 100.0, 99.6, 0.25, 4, false, 0.005).unwrap();
        assert_eq!(f, vec![(1, 100.0, 0.25), (1, 99.75, 0.25)]);
        assert_eq!((px, why), (100.8, "close"));
    }
}
