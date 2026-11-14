//! YT Strategies · 13-3 AKA 2D Short: the team's AmiBroker AFLs in "Old Topics/13-3_AKA 2D Short" (the "2 Day Short" /
//! "Five Day Phenomena" month-start shorts), each run here on one stock's own daily bars. Coded from the AFLs as
//! written, with the AFLs' own default settings; there is no book PDF and nothing here is checked against the team's
//! trade sheets or workbooks.
//!
//! Every AFL shorts at the Close of the month's Nth trading day (`Ref(StartOfMonth, -(EntryDay - 1))`, Entry Day
//! default 5) and covers at the Close of the Mth trading day after entry (Exit Day default 2), with optional entry
//! conditions (candle colour, ROC sign or threshold, RSI / ADX, a Supertrend or % swing filter on the stock's own
//! bars, election months off) and optional exits (hard SL, profit booking, the filter turning). Twelve rule sets are
//! coded; the folder's 67 AFLs are versions, optimisation copies and team-member copies of them, and each id names
//! the AFL it is coded from; the others are reached with its parameters (see each id's notes).
//!
//! AmiBroker behaviour followed:
//! - the AFL bar loop runs over the whole history (from bar 1) and writes Short / Cover with its own CoverPrice; the
//!   backtester opens trades only in the test window, exits first, ignores a Cover while flat and a Short while
//!   already short, and cover prices are kept inside the bar's range (price bound check) (`books::ytroc::backtest`);
//! - no short on a bar whose Low = Close (LC); a cover that falls on a bar whose High = Close (UC) waits for the next
//!   bar that is not UC and is taken at its Close (SkipCover); the Count runs from the bar after entry;
//! - `Ref(x, -n)` is Null for the first n bars and the first bar's StartOfMonth (Month() vs a Null) is false; ROC,
//!   RSI and ADX are Null until they exist and a comparison with them is then false;
//! - the Supertrend filter is the team's ST loop (`books::ytsw::st_filter`: mean of the last `period` ATR(1) values,
//!   bands from the calculation point, ratchet, reset on a flip) on daily, weekly or monthly bars expanded with
//!   expandLast; the % swing filters are the AFLs' close-based loops (bull % above, bear % below, with or without
//!   the all-time-high close `HHV(C, BarCount)` as each AFL has it);
//! - the stock template settings stay at their defaults: dead-stock backtest NO (no buy on the first bar, no forced
//!   last-bar cover), penny price 0, turnover filter off (so no volume is needed); position sizing, MaxOpenPositions
//!   and commission are portfolio settings and are not modelled.
//!
//! Not coded:
//! - the ROC / RSI "ranking" (PositionScore) only chooses between stocks in AmiBroker's portfolio backtest and is not
//!   modelled; the ROC sign / threshold entry conditions of those AFLs are coded;
//! - "2 DAY SHORT ON intraday nth cadle*.afl" (5 files, 4 distinct, in "2 Day Short ON Intraday nth candle_turnover
//!   divide by 7 for 1hr" and "2 Day Short/AFLs/Intraday") trade the Nth intraday candle of the month: intraday bars
//!   are not available here;
//! - "2 Day Short/DualPlot.afl" only plots two Foreign tickers (no trading rules).
#![allow(dead_code)]
use crate::books::yteng::rsi as rsi_afl;
use crate::books::ytroc::{backtest, Px};
use crate::books::ytsw::{exrem, sma, st_filter, true_range};
use crate::books::ytwh::{bound, loop_val};
use crate::connors::{dmi, Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "yt_2ds";
pub const CANDLE: &str = "yt2ds_candle_color";
pub const ROC_VALUE: &str = "yt2ds_roc_value";
pub const ADX: &str = "yt2ds_adx";
pub const RSI_ADX: &str = "yt2ds_rsi_adx";
pub const RSI_ADX_FLT: &str = "yt2ds_rsi_adx_filter";
pub const ROC_ZERO: &str = "yt2ds_roc_ranking";
pub const ROC_THR: &str = "yt2ds_roc_threshold";
pub const ROC_ZERO_FLT: &str = "yt2ds_roc_ranking_filter";
pub const ROC_THR_FLT: &str = "yt2ds_roc_threshold_filter";
pub const ST_SWING: &str = "yt2ds_st_swing_filter";
pub const CALENDAR: &str = "yt2ds_calendar_days";
pub const FRESH: &str = "yt2ds_fresh_st_swing_exit";
pub const IDS: [&str; 12] = [CANDLE, ROC_VALUE, ADX, RSI_ADX, RSI_ADX_FLT, ROC_ZERO, ROC_THR, ROC_ZERO_FLT, ROC_THR_FLT, ST_SWING, CALENDAR, FRESH];

const GROUP: &str = "YT Strategies · 13-3 AKA 2D Short";

fn def(id: &'static str, name: &'static str, rules: &'static [&'static str], notes: &'static str) -> StrategyDef {
    StrategyDef { id, name, group: GROUP, session: 0, dir: "Short", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim: None }
}

const E_DAY: &str = "F|Entry day: the month's Nth trading day (Entry Day, default 5; Ref(StartOfMonth, -(N − 1)))";
const E_LC: &str = "F|No short on a bar whose Low = Close, or on the first bar";
const E_SHORT: &str = "E|Short at the Close";
const X_DAY: &str = "X|Cover at the Close of the Mth trading day after entry (Exit Day, default 2)";
const X_UC: &str = "X|A cover on a bar whose High = Close waits for the next bar that is not, at its Close";
const F_ELEC: &str = "F|Election months: no short in May 2004, 2009, 2014 or 2019 (default on)";
const F_ELEC_OFF: &str = "F|Option (default off): no short in May 2004, 2009, 2014 or 2019 (election months)";
const X_HSL_CC: &str = "X|Hard SL (default 2%): High reaches entry × (1 + HSL%) — at that level, or at Open + 0.25% buffer when the Open is above it";
const X_PT_OPT: &str = "X|Option (default off): profit booking when the Low reaches entry × (1 − PB%) — at that level, or at Open + buffer when the Open is below it (as written)";
const ST_DAILY: &str = "F|ST filter (default): the stock's daily Supertrend (period 20, factor 1, mid point); options weekly / monthly, close / pivot";
const SW_HCL: &str = "F|%SWING option: close swing 44% up / 23% down that also turns up on a new all-time-high close";

macro_rules! tmpl {
    () => {
        " Runs on one stock at a time; the team ran these AFLs over a stock universe (NSE cash stocks). Position sizing, MaxOpenPositions (20) and commission are portfolio settings and are not modelled. Dead-stock backtest NO, penny price 0 and turnover filter off are the template defaults."
    };
}

pub fn catalogue() -> Vec<StrategyDef> {
    vec![
        def(CANDLE, "2 Day Short with Candle Colour (stock)", &[
            E_DAY,
            "F|Candle colour: any (RANDOM, default), red (Close < Open) or green (Close > Open)",
            F_ELEC_OFF, E_LC, E_SHORT, X_DAY,
            "X|Option (default off): hard SL at entry × (1 + HSL%) on the High, at that level or Open + 0.25% buffer when the Open is above it",
            X_PT_OPT, X_UC,
        ], concat!("2 Day Short/2 Day PHENOMENA Short Only with Candle Color.afl, coded as the AFL is written (Entry Day 5, Exit Day 2: the AFL's name says FIVE Day Phenomena). The same rules with other defaults or one added option: FIVE Day PHENOMENA Short Only with Candle Color.afl (2 files) and 2 Day Sh with Candle Color.afl (Param ranges only); 2 Day Short Only with Candle Color_ELECTION NO TRADE.afl (AFLs and NILAV; election months off = no_election 1); ...Candle Color including GREEN and RED.afl and 2 Day Sh with Candle Color+HSL.afl (green option and hard SL 2% = hsl_pct 2; the HSL test is High >= HSL for RANDOM and High > HSL for red / green, the cover price uses High >= HSL); Two Day Short Only with Candle Color_W PB.afl (2 versions; profit booking default 0 and High >= HSL for every colour).", tmpl!())),
        def(ROC_VALUE, "2 Day Short with ROC Value Entry and Hard SL (stock)", &[
            E_DAY,
            "F|-Value version (default): ROC(Close, 5) < −5",
            "F|+Value version: ROC(Close, 5) > +5",
            F_ELEC, E_LC, E_SHORT, X_DAY,
            "X|Option (default 0 = off): hard SL when the High is above entry × (1 + SL%) — at that level, or at Open + 0.25% when the Open is above it (also on a High = Close bar)",
            X_UC,
        ], concat!("2 Day Short/Two Day PHENOMENA Short Only with ROC positive and negative value entry+HSL/2 Day PHENOMENA Short Only with ROC positive and negative value entry+HSL.afl, coded as the AFL is written; ELECTION TRTADES default OFF = no shorts in the election months. Copies: Without Filter/Stocks/ROC/Variable Thresh/2 Day Sh_ROC +ve & -ve.afl (identical), AFLs/... and Team Member Work/NILAV/... (Param ranges only). The earlier 2 Day Short only with Positive and Negative ROC/2 Day PHENOMENA Short Only with ROC positive and negative value entry.afl has no SL and no election rule (no_election 0); its Entry Day default 5 lies below its range 11-16 and is used as written.", tmpl!())),
        def(ADX, "2 Day Short with ADX (stock)", &[
            E_DAY, "F|ADX(14) > 20", E_LC, E_SHORT, X_DAY, X_UC,
        ], concat!("2 Day Short with ADX/2 Day PHENOMENA Short Only with ADX.afl, coded as the AFL is written (ADX period and value are Optimize() with defaults 14 and 20). Its Filter Version list (Off|ST|%Swing) is declared but never used. ADX is Wilder's.", tmpl!())),
        def(RSI_ADX, "2 Day Short with RSI and ADX (stock)", &[
            E_DAY, "F|RSI(14) < 30", "F|ADX(8) > 20", "F|Candle colour: any (default) or red (Close < Open)", F_ELEC, E_LC, E_SHORT, X_DAY,
            "X|Option (default 0 = off): hard SL when the High is above entry × (1 + SL%) — at that level, or at Open + 0.25% when the Open is above it (also on a High = Close bar)",
            X_UC,
        ], concat!("2 Day Short/AFLs/2 Day Short Only with Candle Color_WITH RSI+ADX+ELEC. NO TRADE.afl, coded as the AFL is written. Copies: Team Member Work/NILAV/... (Param ranges only); Two Day Short Only with Candle Color_WITH RSI+ADX.afl (AFLs and NILAV) without the election rule (no_election 0); ..._WITH RSI+ADX+ELEC. NO TRADE+HSL.afl (AFLs and NILAV) adds the SL % option (default 0) with Optimize defaults Entry Day 9, Exit Day 1, RSI value 40 (AFLs) or 5 / 2 / 30 (NILAV). RSI and ADX are Wilder's.", tmpl!())),
        def(RSI_ADX_FLT, "2 Day Short with RSI, ADX and ST / % Swing Filter (stock)", &[
            "F|Entry day: the month's 6th trading day (Entry Day default 6)",
            "F|RSI(14) below 30 (default; option above)", "F|ADX(8) > 30", "F|Candle colour: any (default) or red",
            ST_DAILY, SW_HCL, "F|Short only while the filter is down (NONE: always)",
            F_ELEC, E_LC, E_SHORT, X_DAY,
            "X|Cover when the filter turns up",
            "X|Option (default 0 = off): hard SL when the High is above entry × (1 + SL%) — at that level, or at Open + buffer (0.25%) when the Open is above it",
            X_PT_OPT, X_UC,
        ], concat!("2 Day Short/AFLs/2 Day Short Only with Candle Color_WITH RSI+ADX+ELEC. NO TRADE+HSL+FILTER (Below-above ABC).afl, coded as the AFL is written; its ST PERIOD default 20 lies above its range 1-5 and is used as written; the swing % defaults 23 / 44 are used as written. A hard-SL cover on a High = Close bar waits like the other covers. Same rules with other defaults: NILAV's ...FILTER (Below-above ABC)......afl (Entry Day 5, ADX value 20); ...+RANKING+FILTER (Below Above TBBT).afl (Entry Day 5, ADX value 20; its RSI ranking only chooses between stocks); the _PT versions of ABC and TBBT add profit booking (default 1%) and a buffer parameter, with hard SL 2% (ABC_PT) or 0 (TBBT_PT): pt_pct / sl_pct here.", tmpl!())),
        def(ROC_ZERO, "2 Day Short with Candle Colour and ROC Sign (stock)", &[
            E_DAY,
            "F|RANDOM candle (default): ROC(Close, 12) > 0 (+Ve, default) or < 0 (-Ve)",
            "F|Red / green candle options: Close < Open or Close > Open, with no ROC condition (as written)",
            E_LC, E_SHORT, X_DAY, X_HSL_CC, X_UC,
        ], concat!("Two Day Short Only with Candle Color and +ve and -ve Threshold of ROC Ranking.afl (4 identical files: its own folder, 2 Day Short/..., 2 Day Short/AFLs and Without Filter/Stocks/ROC/0 Thresh/2 Day Sh with Candle color & 0 Threshold ROC ranking_WO Filter.afl), coded as the AFL is written. The ROC Periods default 12 lies above its range 1-6 and is used as written. The ROC ranking (PositionScore) only chooses between stocks and is not modelled. The HSL test is High >= HSL for RANDOM and High > HSL for red / green; the cover price uses High >= HSL.", tmpl!())),
        def(ROC_THR, "2 Day Short with ROC Threshold and Election Months Off (stock)", &[
            E_DAY,
            "F|+Ve (default): ROC(Close, 12) > 5; -Ve: ROC(Close, 12) < −5",
            "F|Candle colour option: only RANDOM (default) trades; red / green give no trades (the AFL's loop has only RANDOM blocks)",
            F_ELEC, E_LC, E_SHORT, X_DAY, X_HSL_CC, X_UC,
        ], concat!("2 Day Short/Two Day Short Only with Candle Color and +ve and -ve Threshold of ROC Ranking_With Election/2 Day Short Only with Candle Color and +ve and -ve Threshold of ROC Ranking_With Election..afl, coded as the AFL is written. Copies: AFLs/... (Param ranges only) and Without Filter/Stocks/ROC/0 Thresh/2 Day Sh with Candle color & Threshold ROC ranking_WO_Fil_Election ON OFF.afl (an unused ROC Value param). The ROC ranking (PositionScore) only chooses between stocks and is not modelled.", tmpl!())),
        def(ROC_ZERO_FLT, "2 Day Short with ROC Sign, Election Off and ST / % Swing Filter (stock)", &[
            E_DAY,
            "F|RANDOM candle (default): ROC(Close, 12) > 0 (+Ve, default) or < 0 (-Ve); red / green options without the ROC condition",
            ST_DAILY, SW_HCL,
            "F|Bear version (default): short only while the filter is down; bull version: only while it is up",
            F_ELEC, E_LC, E_SHORT, X_DAY,
            "X|Cover when the filter turns against the short (bear: up; bull: down)",
            X_HSL_CC, X_PT_OPT, X_UC,
        ], concat!("2 Day Short/Two Day Short Only with Candle Color and +ve and -ve  ROC Ranking_With Election+FILTER/2 Day Short Only with Candle Color and +ve and -ve  ROC Ranking_With Election+FILTER.afl, coded as the AFL is written. Same rules, other Param ranges / Optimize: AFLs/..., Error/..., Team Member Work/NILAV/..., With Filter/Stocks/ROC/Filter in Bear Market/0 Thresh/2 Day Sh 0 Treshold ROC Ranking_With Election+FILTER.afl. flt_side 1 = Filter in Bull Market/0 Thresh/...(Bull Market).afl. flt_side 2 = AFLs/...FILTER no cut on transition.afl as written: its ST filter is swapped (short while the ST is up), its % swing filter is not, and its RANDOM +Ve block covers on FLTBUY == -1 (never with ST / %SWING, the bar after entry with NONE). The _W PB versions (2 Day Short/2 day short Main Versions PB Added and AFLs) add profit booking, default 1% (pt_pct). With NONE the filter is off; with the bull version and NONE there are no shorts (FLTBUY = -1). HSL tests as in the ROC Sign id. The ROC ranking only chooses between stocks.", tmpl!())),
        def(ROC_THR_FLT, "2 Day Short with ROC Threshold, Election Off and Market Filter Option (stock)", &[
            E_DAY,
            "F|+Ve (default): ROC(Close, 12) > 5; -Ve: ROC(Close, 12) < −5; only the RANDOM candle trades",
            "F|Filter (default NONE): monthly Supertrend (period 5, factor 1, mid point), SMA(200) of the Close (daily), or close swing 44% up / 23% down",
            "F|Bear version (default): short only while the filter is down; bull version: only while it is up",
            F_ELEC, E_LC, E_SHORT, X_DAY,
            "X|Cover when the filter turns against the short",
            X_HSL_CC, X_PT_OPT, X_UC,
        ], concat!("2 Day Short/Two Day Short Only with Candle Color and +ve and -ve Threshold of ROC Ranking_With Election WITH FLT/2 Day Short Only with Candle Color and +ve and -ve Threshold of ROC Ranking_With Election WITH FLT..AFL, coded as the AFL is written. Same rules, other Param ranges / Optimize: With Filter/Stocks/ROC/Filter in Bear Market/Variable Thresh/2 Day Sh Variable Threshold of ROC Ranking_With Election W+Filter.AFL; Team Member Work/NILAV/...WITH FLT. (1).afl uses the bear swing % for both bands (bull_pct = bear_pct). flt_side 1 = Filter in Bull Market/Variable Thresh/...(Bull Market).AFL (with NONE it has no shorts). The _W PB versions add profit booking, default 1% (pt_pct). The SMA filter is coded on daily bars only: on weekly / monthly bars the AFL reads the higher-time-frame array without TimeFrameExpand, which is not reproduced, and those settings give no trades. Red / green candle options give no trades (the loop has only RANDOM blocks). The ROC ranking only chooses between stocks.", tmpl!())),
        def(ST_SWING, "2 Day Short with and without ST / % Swing Filter (stock)", &[
            E_DAY,
            "F|Filter (default Off): the stock's monthly Supertrend (period 5, factor 4, mid point) or a close swing 44% up / 23% down",
            "F|Bear version (default): short only while the filter is down; bull version: only while it is up",
            E_LC, E_SHORT, X_DAY,
            "X|Cover when the filter turns against the short",
            X_UC,
        ], concat!("2 Day Short/Two Day Short Only with and without %Swing and ST Filter/Two Day Short Only with and without %Swing and ST Filter.afl, coded as the AFL is written; its Entry Day default 5 lies below its range 10-16 and is used as written. Same rules, other Param ranges: AFLs/..., Team Member Work/NILAV/..., ...(WO Ranking).afl and With Filter/Stocks/ROC/Filter in Bear Market/Only ST and Swing filter WO ranking/2 Day Sh with ST and %swing Filter(WO Ranking).afl. flt_side 1 = Filter in Bull Market/Only ST wo Thresh/2 Day Sh with ST and %swing Filter (WO Ranking)(Bull Makret).afl.", tmpl!())),
        def(CALENDAR, "2 Day Short on Calendar Days (stock)", &[
            "F|Calendar day of the month (Day()): entry day 5, exit day 5 + 2 = 7",
            "F|Next (default): first trading day on or after the 5th; Previous: the trading day before it (unless that next bar is the 5th itself)",
            "E|Short at the Close on the 5th when flat (and the filter is down), or on the Next / Previous day even while already short (AFL precedence)",
            "F|Filter (default Off): monthly Supertrend (period 5, factor 4) or close swing 44% / 23%; it applies to the 5th only",
            "X|Cover at the Close on the 7th, or on the first trading day on or after the 2nd of a month (Next; as written), or the day before it (Previous), or when the filter turns up",
            X_UC,
        ], concat!("2 Day Short/Calendar Days/Two Day Short Only with and without %Swing and ST Filter_Calendar Days.afl, coded as the AFL is written: Buy Days 5, Sell Days 2, Entry Type Next. The entry line `Position == 0 AND Buy_Equal OR (Buy_Next AND Next) OR (Buy_Previous AND Previous AND cash filter)` follows AFL precedence, so the Next / Previous entries ignore the position, the filter and (Next) the Low = Close guard; a short while already short is ignored by the backtester. The Next exit uses Sell Days alone (first day on or after the 2nd), not 5 + 2. The Previous options look one bar ahead (Ref(.., 1)) as the AFL does.", tmpl!())),
        def(FRESH, "2 Day Short with Fresh ST and % Swing Exit (stock)", &[
            E_DAY,
            "F|ROC condition (default NONE): positive ROC(10) > 5 or negative ROC(10) < −5",
            "F|Option (default NONE): % swing filter (44% up / 23% down with the all-time-high close); short only while it is down",
            E_LC, E_SHORT,
            "X|Cover at the Close of the month's (Entry Day + Exit Day)th trading day (5 + 7 = 12th)",
            "X|Fresh ST stop: Close above the previous bar's level; level at entry = mid point + 2 × mean ATR(1) of 5 bars, then the lower of that and the new value each bar",
            "X|% swing stop: Close above the previous bar's level; level at entry = Close × 1.05, then the lower of that and Close × 1.05",
            "X|Option (default 0 = off): hard SL on High >= entry × (1 + HSL%), at that level or Open + 0.25%",
            X_UC,
        ], concat!("Without Filter/Stocks/ROC/Fresh STR and % swing/2 Day Sh_Fresh STR and % swing Exit.afl, coded as the AFL is written (Entry Day 5 and Exit Day 7 are Optimize defaults outside their ranges 12-14 and 2-4, used as written). Same rules, other Param ranges: 2 Day Short/2 Day Short +ve and -ve Threshold of ROC Ranking With Fresh STR and %swing Exit/...afl, Error/2 Day Short +ve and -ve Threshold of ROC Ranking With FLT N ST SWING EXIT_Final.afl and AFLs/...Harshit Sir Modified v1_Final.afl (hard SL default 2 = hsl_pct 2). The ROC ranking only chooses between stocks; ROCCOND is coded.", tmpl!())),
    ]
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    let days = |e: f64, x: f64| vec![p("entry_day", "Entry day (Nth trading day of the month)", e, 1.0, 31.0, 1.0), p("exit_day", "Exit day (trading days after entry)", x, 1.0, 31.0, 1.0)];
    let elec = |d: f64| p("no_election", "No shorts in election months (May 2004/09/14/19; 1 on)", d, 0.0, 1.0, 1.0);
    let hsl = |d: f64| p("hsl_pct", "Hard SL % (0 off)", d, 0.0, 12.0, 1.0);
    let sl = || p("sl_pct", "Stop loss % (0 off)", 0.0, 0.0, 15.0, 1.0);
    let buf = || p("buffer_pct", "Buffer % on a gap through a level", 0.25, 0.0, 10.0, 0.01);
    let pt = || p("pt_pct", "Profit booking % (0 off)", 0.0, 0.0, 12.0, 1.0);
    let roc_side = || p("roc_side", "ROC version (0 +Ve, 1 -Ve)", 0.0, 0.0, 1.0, 1.0);
    let roc_p = |d: f64| p("roc_period", "ROC periods", d, 1.0, 16.0, 1.0);
    let thr = || vec![p("roc_neg", "ROC -value", 5.0, 0.0, 12.0, 1.0), p("roc_pos", "ROC +value", 5.0, 0.0, 16.0, 1.0)];
    let candle = |max: f64| p("candle", if max > 1.0 { "Candle colour (0 any, 1 red, 2 green)" } else { "Candle colour (0 any, 1 red)" }, 0.0, 0.0, max, 1.0);
    let filter = |label, tf: f64, period: f64, factor: f64, sides: f64| {
        vec![
            p("flt", label, 0.0, 0.0, 3.0, 1.0),
            p("flt_tf", "Filter time frame (0 daily, 1 weekly, 2 monthly)", tf, 0.0, 2.0, 1.0),
            p("flt_point", "ST calculation point (0 mid point, 1 close, 2 pivot)", 0.0, 0.0, 2.0, 1.0),
            p("flt_period", "ST period", period, 1.0, 25.0, 1.0),
            p("flt_factor", "ST factor", factor, 0.5, 20.0, 0.5),
            p("bear_pct", "Swing % for bear market (down band)", 23.0, 1.0, 50.0, 1.0),
            p("bull_pct", "Swing % for bull market (up band)", 44.0, 1.0, 50.0, 1.0),
            p("flt_side", if sides > 1.0 { "Version (0 bear, 1 bull, 2 no cut on transition)" } else { "Version (0 bear: short while down, 1 bull: short while up)" }, 0.0, 0.0, sides, 1.0),
        ]
    };
    let mut v: Vec<ParamDef> = match id {
        CANDLE => cat(vec![days(5.0, 2.0), vec![candle(2.0), elec(0.0), hsl(0.0), buf(), pt()]]),
        ROC_VALUE => cat(vec![days(5.0, 2.0), vec![p("roc_ver", "ROC version (0 -Value: below −value, 1 +Value: above +value)", 0.0, 0.0, 1.0, 1.0), roc_p(5.0)], thr(), vec![sl(), elec(1.0)]]),
        ADX => cat(vec![days(5.0, 2.0), vec![p("adx_period", "ADX period", 14.0, 1.0, 50.0, 1.0), p("adx_val", "ADX value", 20.0, 0.0, 100.0, 5.0)]]),
        RSI_ADX => cat(vec![
            days(5.0, 2.0),
            vec![candle(1.0), p("rsi_period", "RSI period", 14.0, 2.0, 50.0, 1.0), p("rsi_val", "RSI value (below)", 30.0, 0.0, 100.0, 5.0), p("adx_period", "ADX period", 8.0, 1.0, 50.0, 1.0), p("adx_val", "ADX value", 20.0, 0.0, 100.0, 5.0), elec(1.0), sl()],
        ]),
        RSI_ADX_FLT => cat(vec![
            filter("Filter (0 ST, 1 %SWING, 2 NONE)", 0.0, 20.0, 1.0, 0.0),
            days(6.0, 2.0),
            vec![
                candle(1.0),
                p("rsi_period", "RSI period", 14.0, 2.0, 50.0, 1.0),
                p("rsi_val", "RSI value", 30.0, 0.0, 100.0, 5.0),
                p("rsi_above", "RSI side (0 below, 1 above)", 0.0, 0.0, 1.0, 1.0),
                p("adx_period", "ADX period", 8.0, 1.0, 50.0, 1.0),
                p("adx_val", "ADX value", 30.0, 0.0, 100.0, 5.0),
                elec(1.0),
                sl(),
                pt(),
                buf(),
            ],
        ]),
        ROC_ZERO => cat(vec![days(5.0, 2.0), vec![candle(2.0), roc_side(), roc_p(12.0), hsl(2.0), buf()]]),
        ROC_THR => cat(vec![days(5.0, 2.0), vec![candle(2.0), roc_side(), roc_p(12.0)], thr(), vec![elec(1.0), hsl(2.0), buf()]]),
        ROC_ZERO_FLT => cat(vec![filter("Filter (0 ST, 1 %SWING, 2 NONE)", 0.0, 20.0, 1.0, 2.0), days(5.0, 2.0), vec![candle(2.0), roc_side(), roc_p(12.0), elec(1.0), hsl(2.0), buf(), pt()]]),
        ROC_THR_FLT => cat(vec![
            filter("Filter (0 NONE, 1 ST, 2 SMA, 3 S%)", 2.0, 5.0, 1.0, 1.0),
            vec![p("sma_period", "SMA filter period", 200.0, 1.0, 300.0, 1.0)],
            days(5.0, 2.0),
            vec![candle(2.0), roc_side(), roc_p(12.0)],
            thr(),
            vec![elec(1.0), hsl(2.0), buf(), pt()],
        ]),
        ST_SWING => cat(vec![filter("Filter (0 Off, 1 ST, 2 %Swing)", 2.0, 5.0, 4.0, 1.0), days(5.0, 2.0)]),
        CALENDAR => cat(vec![
            filter("Filter (0 Off, 1 ST, 2 %Swing)", 2.0, 5.0, 4.0, 0.0),
            vec![p("buy_day", "Buy days (calendar day)", 5.0, 1.0, 31.0, 1.0), p("sell_day", "Sell days", 2.0, 1.0, 31.0, 1.0), p("entry_type", "Entry type (0 Next, 1 Previous)", 0.0, 0.0, 1.0, 1.0)],
        ]),
        FRESH => vec![
            p("flt", "Filter (0 NONE, 1 %S)", 0.0, 0.0, 1.0, 1.0),
            p("bear_pct", "Swing % for bear market (down band)", 23.0, 1.0, 50.0, 1.0),
            p("bull_pct", "Swing % for bull market (up band)", 44.0, 1.0, 50.0, 1.0),
            p("entry_day", "Entry day (Nth trading day of the month)", 5.0, 1.0, 31.0, 1.0),
            p("exit_day", "Exit day (added to the entry day)", 7.0, 1.0, 31.0, 1.0),
            p("st_point", "ST calculation point (0 mid point, 1 close, 2 pivot)", 0.0, 0.0, 2.0, 1.0),
            p("st_period", "ST period", 5.0, 1.0, 100.0, 1.0),
            p("st_factor", "ST factor", 2.0, 0.0, 300.0, 0.5),
            p("swing_pct", "Swing %", 5.0, 0.0, 50.0, 1.0),
            hsl(0.0),
            p("roc_period", "ROC period", 10.0, 1.0, 100.0, 1.0),
            p("roc_thr", "ROC threshold", 5.0, 0.0, 100.0, 1.0),
            p("roc_mode", "ROC mode (0 NONE, 1 positive, 2 negative)", 0.0, 0.0, 2.0, 1.0),
        ],
        _ => return None,
    };
    if id == RSI_ADX_FLT || id == ROC_ZERO_FLT {
        // these AFLs' filter list is ST|%SWING|NONE
        if let Some(f) = v.iter_mut().find(|d| d.key == "flt") {
            f.max = 2.0;
        }
    }
    if id == ST_SWING || id == CALENDAR {
        if let Some(f) = v.iter_mut().find(|d| d.key == "flt") {
            f.max = 2.0;
        }
    }
    Some(v)
}

fn cat(v: Vec<Vec<ParamDef>>) -> Vec<ParamDef> {
    v.into_iter().flatten().collect()
}

// ---------------------------------------------------------------- shared pieces

const BIG: f64 = 9_999_999.0;

struct Cal {
    y: Vec<i32>,
    m: Vec<u32>,
    d: Vec<u32>,
}

fn cal(x: &Ctx) -> Cal {
    let part = |s: &str, a: usize, b: usize| s.get(a..b).and_then(|v| v.parse::<u32>().ok()).unwrap_or(0);
    Cal {
        y: x.r.iter().map(|r| part(&r.date, 0, 4) as i32).collect(),
        m: x.r.iter().map(|r| part(&r.date, 5, 7)).collect(),
        d: x.r.iter().map(|r| part(&r.date, 8, 10)).collect(),
    }
}

/// `Ref(StartOfMonth, -(day - 1))`: the month's `day`th trading day (the first bar's StartOfMonth is false).
fn nth_day(k: &Cal, day: i64) -> Vec<bool> {
    let n = k.m.len();
    let som: Vec<bool> = (0..n).map(|i| i > 0 && k.m[i] != k.m[i - 1]).collect();
    let back = (day - 1).max(0) as usize;
    (0..n).map(|i| i >= back && som[i - back]).collect()
}

/// The AFLs' NO_TRADE: May of 2004, 2009, 2014 and 2019.
fn election(k: &Cal) -> Vec<bool> {
    (0..k.m.len()).map(|i| k.m[i] == 5 && matches!(k.y[i], 2004 | 2009 | 2014 | 2019)).collect()
}

/// AFL ROC(C, n) in percent; Null for the first n bars.
fn roc(c: &[f64], n: usize) -> Vec<Option<f64>> {
    (0..c.len()).map(|i| if i >= n && c[i - n] != 0.0 { Some((c[i] - c[i - n]) / c[i - n] * 100.0) } else { None }).collect()
}

fn adx(x: &Ctx, n: usize) -> Vec<Option<f64>> {
    dmi(x.r, n.max(1)).1.into_iter().map(|v| if v.is_finite() { Some(v) } else { None }).collect()
}

/// Short_Cash_Filter: LC == 0 AND Close > penny price (0); turnover off, dead stock NO.
fn cash(x: &Ctx) -> Vec<bool> {
    (0..x.c.len()).map(|i| x.l[i] != x.c[i] && x.c[i] > 0.0).collect()
}

fn tf_char(code: i64) -> char {
    match code {
        2 => 'M',
        1 => 'W',
        _ => 'D',
    }
}

/// The AFLs' close % swing: Up = C + C x bull%, Dn = C - C x bear%; up when C > Up[i-1] (or, with `athc`, when C
/// tops the highest close so far), down when C < Dn[i-1]; bands ratchet and reset on a flip (with `athc` the reset
/// Up is capped at the highest close).
fn close_swing(c: &[f64], bear: f64, bull: f64, athc: bool) -> Vec<i8> {
    let n = c.len();
    let mut hcl = c.to_vec();
    for i in 1..n {
        hcl[i] = hcl[i].max(hcl[i - 1]);
    }
    let mut up: Vec<f64> = c.iter().map(|v| v + v * bull / 100.0).collect();
    let mut dn: Vec<f64> = c.iter().map(|v| v - v * bear / 100.0).collect();
    let mut tr = vec![1i8; n];
    for i in 1..n {
        tr[i] = if c[i] > up[i - 1] || (athc && c[i] > hcl[i - 1]) { 1 } else if c[i] < dn[i - 1] { -1 } else { tr[i - 1] };
        if tr[i] > 0 && dn[i] < dn[i - 1] {
            dn[i] = dn[i - 1];
        }
        if tr[i] < 0 && up[i] > up[i - 1] {
            up[i] = up[i - 1];
        }
        if tr[i] < 0 && tr[i - 1] > 0 {
            let u = c[i] + c[i] * bull / 100.0;
            up[i] = if athc { hcl[i].min(u) } else { u };
        }
        if tr[i] > 0 && tr[i - 1] < 0 {
            dn[i] = c[i] - c[i] * bear / 100.0;
        }
    }
    tr
}

/// Hard SL / profit booking settings of one AFL's cover block.
struct Stops {
    hsl_pct: f64,
    /// trigger High >= HSL (else High > HSL)
    ge: bool,
    /// cover price at the HSL when High >= HSL (else High > HSL)
    price_ge: bool,
    /// the HSL cover is also taken on a High = Close bar
    uc_ok: bool,
    buf: f64,
    pt_pct: f64,
}

const NO_STOPS: Stops = Stops { hsl_pct: 0.0, ge: true, price_ge: true, uc_ok: false, buf: 0.0, pt_pct: 0.0 };

/// The AFLs' month-day short loop: short on `entry`, cover on the Exit Day count, the hard SL, profit booking or
/// `cut`; a cover blocked by a High = Close bar is taken at the next such-free bar's Close.
fn day_loop(x: &Ctx, entry: &[bool], cut: &[bool], exit_day: i64, s: &Stops) -> Px {
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let n = c.len();
    let mut px = Px::new(n);
    let (mut pos, mut skip, mut count) = (false, false, 0i64);
    let (mut hsl, mut pt) = (BIG, 0.0f64);
    for i in 1..n {
        let uc = h[i] == c[i];
        if !pos && entry[i] {
            px.short[i] = true;
            pos = true;
            if s.hsl_pct > 0.0 {
                hsl = c[i] + c[i] * s.hsl_pct / 100.0;
            }
            if s.pt_pct > 0.0 {
                pt = c[i] - c[i] * s.pt_pct / 100.0;
            }
        } else if pos && !skip {
            count += 1;
            let hit_h = if s.ge { h[i] >= hsl } else { h[i] > hsl };
            let hit_p = l[i] <= pt;
            if hit_h || hit_p || count == exit_day || cut[i] {
                if !uc || (s.uc_ok && hit_h) {
                    let at_hsl = if s.price_ge { h[i] >= hsl } else { h[i] > hsl };
                    let (p, why) = if at_hsl {
                        (if o[i] > hsl { o[i] + o[i] * s.buf / 100.0 } else { hsl }, "Hard SL")
                    } else if hit_p {
                        (if o[i] < pt { o[i] + o[i] * s.buf / 100.0 } else { pt }, "Profit booking")
                    } else if count == exit_day {
                        (c[i], "Exit day")
                    } else {
                        (c[i], "Filter turned against the short")
                    };
                    px.cover(i, bound(x, i, p), why);
                    pos = false;
                    count = 0;
                    hsl = BIG;
                    pt = 0.0;
                } else {
                    skip = true;
                }
            }
        } else if skip && !uc {
            px.cover(i, c[i], "Cover after a High = Close bar");
            pos = false;
            skip = false;
            count = 0;
            hsl = BIG;
            pt = 0.0;
        }
    }
    px
}

/// The ROC-value / RSI+ADX AFLs' SL: HSL[i-1] with High > HSL, Open + 0.25%, also on a High = Close bar.
fn sl_stops(pct: f64, uc_ok: bool, buf: f64) -> Stops {
    Stops { hsl_pct: pct, ge: false, price_ge: false, uc_ok, buf, pt_pct: 0.0 }
}

/// The candle-colour AFLs' HSL: High >= HSL for RANDOM, High > HSL for red / green; price at High >= HSL.
fn cc_stops(pct: f64, candle: i64, buf: f64, pt_pct: f64) -> Stops {
    Stops { hsl_pct: pct, ge: candle == 0, price_ge: true, uc_ok: false, buf, pt_pct }
}

fn candle_ok(x: &Ctx, i: usize, candle: i64) -> bool {
    match candle {
        1 => x.c[i] < x.o[i],
        2 => x.c[i] > x.o[i],
        _ => true,
    }
}

/// FLTBuy / FLTShort as the AFLs set them (1 true, 0 false; NONE: -1 and 1).
fn filter_arrays(x: &Ctx, g: &dyn Fn(&str) -> f64, kind: i64, swing_athc: bool) -> (Vec<i8>, Vec<i8>) {
    let n = x.c.len();
    let gi = |k: &str| g(k).round() as i64;
    match kind {
        1 => {
            let tr = st_filter(x, tf_char(gi("flt_tf")), gi("flt_point") + 1, g("flt_period").round().max(1.0) as usize, g("flt_factor"));
            (tr.iter().map(|&t| (t == 1) as i8).collect(), tr.iter().map(|&t| (t == -1) as i8).collect())
        }
        2 => {
            let tr = close_swing(&x.c, g("bear_pct"), g("bull_pct"), swing_athc);
            (tr.iter().map(|&t| (t == 1) as i8).collect(), tr.iter().map(|&t| (t == -1) as i8).collect())
        }
        _ => (vec![-1; n], vec![1; n]),
    }
}

// ---------------------------------------------------------------- run

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(0.0, |d| d.def));
    let gi = |k: &str| g(k).round() as i64;
    let gu = |k: &str| g(k).round().max(1.0) as usize;
    let c = &x.c;
    let n = c.len();
    if n == 0 || from > to || to >= n {
        return Some(Vec::new());
    }
    let k = cal(x);
    let base: Vec<bool> = {
        let day = if id == CALENDAR { vec![false; n] } else { nth_day(&k, gi("entry_day")) };
        let cf = cash(x);
        let nt = election(&k);
        let no_elec = schema.iter().any(|d| d.key == "no_election") && gi("no_election") == 1;
        (0..n).map(|i| day[i] && cf[i] && !(no_elec && nt[i])).collect()
    };
    let none = vec![false; n];
    let exit_day = gi("exit_day");
    let px = match id {
        CANDLE => {
            let cc = gi("candle");
            let entry: Vec<bool> = (0..n).map(|i| base[i] && candle_ok(x, i, cc)).collect();
            day_loop(x, &entry, &none, exit_day, &cc_stops(g("hsl_pct"), cc, g("buffer_pct"), g("pt_pct")))
        }
        ROC_VALUE => {
            let r = roc(c, g("roc_period").round().max(0.0) as usize);
            let (neg, pos, plus) = (g("roc_neg"), g("roc_pos"), gi("roc_ver") == 1);
            let entry: Vec<bool> = (0..n).map(|i| base[i] && r[i].map_or(false, |v| if plus { v > pos } else { v < -neg })).collect();
            day_loop(x, &entry, &none, exit_day, &sl_stops(g("sl_pct"), true, 0.25))
        }
        ADX => {
            let a = adx(x, gu("adx_period"));
            let av = g("adx_val");
            let entry: Vec<bool> = (0..n).map(|i| base[i] && a[i].map_or(false, |v| v > av)).collect();
            day_loop(x, &entry, &none, exit_day, &NO_STOPS)
        }
        RSI_ADX | RSI_ADX_FLT => {
            let rs = rsi_afl(c, gu("rsi_period"));
            let a = adx(x, gu("adx_period"));
            let (rv, av, cc) = (g("rsi_val"), g("adx_val"), gi("candle"));
            let above = id == RSI_ADX_FLT && gi("rsi_above") == 1;
            let (fb, fs) = if id == RSI_ADX_FLT { filter_arrays(x, &g, match gi("flt") { 0 => 1, 1 => 2, _ => 0 }, true) } else { (vec![-1; n], vec![1; n]) };
            let entry: Vec<bool> = (0..n)
                .map(|i| base[i] && candle_ok(x, i, cc.min(1)) && rs[i].map_or(false, |v| if above { v > rv } else { v < rv }) && a[i].map_or(false, |v| v > av) && fs[i] == 1)
                .collect();
            let cut: Vec<bool> = fb.iter().map(|&v| v == 1).collect();
            let mut st = sl_stops(g("sl_pct"), id == RSI_ADX, 0.25);
            if id == RSI_ADX_FLT {
                st.buf = g("buffer_pct");
                st.pt_pct = g("pt_pct");
            }
            day_loop(x, &entry, &cut, exit_day, &st)
        }
        ROC_ZERO | ROC_ZERO_FLT => {
            let r = roc(c, g("roc_period").round().max(0.0) as usize);
            let (cc, neg) = (gi("candle"), gi("roc_side") == 1);
            let side = if id == ROC_ZERO_FLT { gi("flt_side") } else { 0 };
            let (fb, fs) = if id == ROC_ZERO_FLT {
                let kind = match gi("flt") { 0 => 1, 1 => 2, _ => 0 };
                let (mut fb, mut fs) = filter_arrays(x, &g, kind, true);
                if side == 2 && kind == 1 {
                    // "no cut on transition": FLTBuy = ST == -1, FLTShort = ST == 1
                    std::mem::swap(&mut fb, &mut fs);
                }
                (fb, fs)
            } else {
                (vec![-1; n], vec![1; n])
            };
            let flt_entry: Vec<bool> = (0..n).map(|i| if side == 1 { fb[i] == 1 } else { fs[i] == 1 }).collect();
            let cut: Vec<bool> = (0..n)
                .map(|i| match side {
                    1 => fs[i] == 1,
                    2 if cc == 0 && !neg => fb[i] == -1,
                    _ => fb[i] == 1,
                })
                .collect();
            let entry: Vec<bool> = (0..n)
                .map(|i| base[i] && flt_entry[i] && if cc == 0 { r[i].map_or(false, |v| if neg { v < 0.0 } else { v > 0.0 }) } else { candle_ok(x, i, cc) })
                .collect();
            let pt = if id == ROC_ZERO_FLT { g("pt_pct") } else { 0.0 };
            day_loop(x, &entry, &cut, exit_day, &cc_stops(g("hsl_pct"), cc, g("buffer_pct"), pt))
        }
        ROC_THR | ROC_THR_FLT => {
            let r = roc(c, g("roc_period").round().max(0.0) as usize);
            let (cc, neg) = (gi("candle"), gi("roc_side") == 1);
            let (nv, pv) = (g("roc_neg"), g("roc_pos"));
            let bull = id == ROC_THR_FLT && gi("flt_side") == 1;
            let (fb, fsell): (Vec<i8>, Vec<i8>) = if id == ROC_THR_FLT {
                match gi("flt") {
                    1 | 3 => {
                        let kind = if gi("flt") == 1 { 1 } else { 2 };
                        filter_arrays(x, &g, kind, false)
                    }
                    2 if gi("flt_tf") == 0 => {
                        // smbuysig: 1 when C > FLTMA[i-1], 0 when C < FLTMA[i-1], else carried (Null reads -1e10)
                        let ma = sma(c, gu("sma_period"));
                        let mut s = vec![0i8; n];
                        for i in 1..n {
                            let m = loop_val(ma[i - 1]);
                            s[i] = if c[i] > m { 1 } else if c[i] < m { 0 } else { s[i - 1] };
                        }
                        (s.iter().map(|&v| (v == 1) as i8).collect(), s.iter().map(|&v| (v == 0) as i8).collect())
                    }
                    2 => (vec![0; n], vec![0; n]), // weekly / monthly SMA: not reproduced (no trades)
                    _ => (vec![0; n], vec![1; n]),
                }
            } else {
                (vec![0; n], vec![1; n])
            };
            let entry: Vec<bool> = (0..n)
                .map(|i| {
                    let flt_ok = if bull { fb[i] == 1 } else { fsell[i] == 1 };
                    cc == 0 && base[i] && flt_ok && r[i].map_or(false, |v| if neg { v < -nv } else { v > pv })
                })
                .collect();
            let cut: Vec<bool> = (0..n).map(|i| if bull { fsell[i] == 1 } else { fb[i] == 1 }).collect();
            let pt = if id == ROC_THR_FLT { g("pt_pct") } else { 0.0 };
            day_loop(x, &entry, &cut, exit_day, &cc_stops(g("hsl_pct"), cc, g("buffer_pct"), pt))
        }
        ST_SWING => {
            let (fb, fs) = filter_arrays(x, &g, gi("flt"), false);
            let bull = gi("flt_side") == 1;
            let on = gi("flt") == 1 || gi("flt") == 2;
            let entry: Vec<bool> = (0..n).map(|i| base[i] && (!on || if bull { fb[i] == 1 } else { fs[i] == 1 })).collect();
            let cut: Vec<bool> = (0..n).map(|i| on && if bull { fs[i] == 1 } else { fb[i] == 1 }).collect();
            day_loop(x, &entry, &cut, exit_day, &NO_STOPS)
        }
        CALENDAR => calendar(x, &k, &g),
        FRESH => fresh(x, &k, &g),
        _ => return None,
    };
    Some(backtest(x, &px, from, to))
}

/// Two Day Short Only with and without %Swing and ST Filter_Calendar Days.afl.
fn calendar(x: &Ctx, k: &Cal, g: &dyn Fn(&str) -> f64) -> Px {
    let (h, l, c) = (&x.h, &x.l, &x.c);
    let n = c.len();
    let gi = |s: &str| g(s).round() as i64;
    let (buy_d, sell_d) = (gi("buy_day"), gi("sell_day"));
    let day = |i: usize| k.d[i] as i64;
    let buy_eq: Vec<bool> = (0..n).map(|i| day(i) == buy_d).collect();
    let sell_eq: Vec<bool> = (0..n).map(|i| day(i) == buy_d + sell_d).collect();
    // NextMonth = Month() != Ref(Month(), 1); the last bar ends its period
    let next_m: Vec<bool> = (0..n).map(|i| i + 1 >= n || k.m[i] != k.m[i + 1]).collect();
    let buy_next = exrem(&(0..n).map(|i| day(i) >= buy_d).collect::<Vec<_>>(), &next_m);
    let sell_next = exrem(&(0..n).map(|i| day(i) >= sell_d).collect::<Vec<_>>(), &next_m);
    let buy_prev: Vec<bool> = (0..n).map(|i| i + 1 < n && buy_next[i + 1] && !buy_eq[i + 1]).collect();
    let sell_prev: Vec<bool> = (0..n).map(|i| i + 1 < n && sell_next[i + 1] && !sell_eq[i + 1] && l[i] != c[i]).collect();
    let next = gi("entry_type") == 0;
    let cf = cash(x);
    let kind = gi("flt");
    let (fb, fs) = filter_arrays(x, g, kind, false);
    let on = kind == 1 || kind == 2;
    let mut px = Px::new(n);
    let (mut pos, mut skip) = (false, false);
    for i in 1..n {
        let uc = h[i] == c[i];
        let flt = !on || fs[i] == 1;
        if (!pos && flt && buy_eq[i]) || (buy_next[i] && next) || (buy_prev[i] && !next && cf[i]) {
            px.short[i] = true;
            pos = true;
        } else if pos && !skip {
            if sell_eq[i] || (sell_next[i] && next) || (sell_prev[i] && !next) || (on && fb[i] == 1) {
                if !uc {
                    px.cover(i, c[i], if on && fb[i] == 1 && !sell_eq[i] { "Filter turned against the short" } else { "Exit day" });
                    pos = false;
                } else {
                    skip = true;
                }
            }
        } else if skip && !uc {
            px.cover(i, c[i], "Cover after a High = Close bar");
            pos = false;
            skip = false;
        }
    }
    px
}

/// 2 Day Sh_Fresh STR and % swing Exit.afl.
fn fresh(x: &Ctx, k: &Cal, g: &dyn Fn(&str) -> f64) -> Px {
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let n = c.len();
    let gi = |s: &str| g(s).round() as i64;
    let (ed, xd) = (gi("entry_day"), gi("exit_day"));
    let shortsig = nth_day(k, ed);
    let coversig = nth_day(k, ed + xd);
    let r = roc(c, g("roc_period").round().max(0.0) as usize);
    let lvl = g("roc_thr");
    let roccond: Vec<bool> = (0..n)
        .map(|i| match gi("roc_mode") {
            1 => r[i].map_or(false, |v| v > lvl),
            2 => r[i].map_or(false, |v| v < -lvl && v < 0.0),
            _ => true,
        })
        .collect();
    let fsell: Vec<bool> = if gi("flt") == 1 { close_swing(c, g("bear_pct"), g("bull_pct"), true).iter().map(|&t| t == -1).collect() } else { vec![true; n] };
    let cf = cash(x);
    let pd = g("st_period").round().max(1.0) as usize;
    let trr = true_range(h, l, c);
    let satr: Vec<Option<f64>> = (0..n)
        .map(|i| if i + 1 < pd { None } else { trr[i + 1 - pd..=i].iter().try_fold(0.0, |s, v| v.map(|v| s + v)).map(|s| s / pd as f64) })
        .collect();
    let point = gi("st_point");
    let stcal = |i: usize| match point {
        1 => c[i],
        2 => (h[i] + l[i] + c[i]) / 3.0,
        _ => (h[i] + l[i]) / 2.0,
    };
    let (fac, sw, hsl_pct) = (g("st_factor"), g("swing_pct"), g("hsl_pct"));
    // ST level for bar i: Null (-1e10 in the loop) until the ATR mean exists
    let st_new = |i: usize| satr[i].map(|s| stcal(i) + fac * s);
    let mut st = vec![0.0f64; n];
    let mut swing = vec![0.0f64; n];
    let mut px = Px::new(n);
    let (mut pos, mut skip) = (false, false);
    let mut hsl = BIG;
    for i in 1..n {
        let uc = h[i] == c[i];
        if !pos && shortsig[i] && cf[i] && roccond[i] && fsell[i] {
            px.short[i] = true;
            if hsl_pct > 0.0 {
                hsl = c[i] + c[i] * hsl_pct / 100.0;
            }
            st[i] = loop_val(st_new(i));
            swing[i] = c[i] + c[i] * sw / 100.0;
            pos = true;
        } else if pos && !skip {
            let hit_h = h[i] >= hsl;
            if hit_h || coversig[i] || c[i] > st[i - 1] || c[i] > swing[i - 1] {
                if !uc {
                    let (p, why) = if hit_h {
                        (if o[i] > hsl { o[i] + o[i] * 0.25 / 100.0 } else { hsl }, "Hard SL")
                    } else if coversig[i] {
                        (c[i], "Exit day")
                    } else if c[i] > st[i - 1] {
                        (c[i], "Close above the fresh ST")
                    } else {
                        (c[i], "Close above the % swing level")
                    };
                    px.cover(i, bound(x, i, p), why);
                    pos = false;
                    hsl = BIG;
                } else {
                    skip = true;
                }
            } else {
                st[i] = loop_val(st_new(i)).min(st[i - 1]);
                swing[i] = (c[i] + c[i] * sw / 100.0).min(swing[i - 1]);
            }
        } else if skip && !uc {
            px.cover(i, c[i], "Cover after a High = Close bar");
            pos = false;
            skip = false;
            hsl = BIG;
        }
    }
    px
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    if !IDS.contains(&id) {
        return None;
    }
    let d = catalogue().into_iter().find(|d| d.id == id)?;
    Some((d.id, d.name, "13-3 AKA 2D Short"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::books::ytmr::day_number;
    use crate::books::ytwh::testbars::*;
    use crate::Row;

    /// Weekday bars from `start`; each bar is (open, high, low, close).
    fn bars_from(start: &str, v: &[(f64, f64, f64, f64)]) -> Vec<Row> {
        let mut d = day_number(start).unwrap();
        v.iter()
            .map(|&(open, high, low, close)| {
                // skip Saturday (6) and Sunday (0) in the (d + 4) mod 7 weekday numbering
                while matches!((d + 4).rem_euclid(7), 0 | 6) {
                    d += 1;
                }
                let r = Row { ticker: "T".into(), date: civil(d), open, high, low, close, volume: None };
                d += 1;
                r
            })
            .collect()
    }

    fn flat(n: usize) -> Vec<(f64, f64, f64, f64)> {
        vec![(100.0, 101.0, 99.0, 100.0); n]
    }

    // From 2024-01-01 (Monday): bar 23 = Thu 1 Feb, bar 27 = Wed 7 Feb (5th trading day), bar 29 = Fri 9 Feb.
    #[test]
    fn candle_shorts_the_fifth_day_and_covers_two_days_later() {
        let v = flat(60);
        let t = go(run, CANDLE, &bars(&v), &[]);
        assert_eq!((t[0].0, t[0].1, t[0].2, t[0].3), (27, 100.0, 29, 100.0));
        // red candle option: bar 27 is not red -> no February trade
        let t = go(run, CANDLE, &bars(&v), &[("candle", 1.0)]);
        assert!(t.iter().all(|t| t.0 != 27));
        let mut v2 = v.clone();
        v2[27] = (100.5, 101.0, 99.0, 100.0);
        assert_eq!(go(run, CANDLE, &bars(&v2), &[("candle", 1.0)])[0].0, 27);
        // a Low = Close entry bar is skipped
        v2[27] = (100.5, 101.0, 100.0, 100.0);
        assert!(go(run, CANDLE, &bars(&v2), &[]).iter().all(|t| t.0 != 27));
    }

    #[test]
    fn upper_circuit_bar_delays_the_cover() {
        let mut v = flat(60);
        v[29] = (100.0, 101.0, 99.0, 101.0); // High = Close
        let t = go(run, CANDLE, &bars(&v), &[]);
        assert_eq!((t[0].0, t[0].2, t[0].3), (27, 30, 100.0));
    }

    #[test]
    fn hard_sl_at_level_or_open_plus_buffer() {
        let mut v = flat(60);
        v[28] = (100.0, 102.5, 99.0, 101.0); // HSL 102 reached intrabar
        let t = go(run, CANDLE, &bars(&v), &[("hsl_pct", 2.0)]);
        assert_eq!((t[0].2, t[0].3), (28, 102.0));
        v[28] = (103.0, 104.0, 102.5, 103.5); // gap above the HSL: open + 0.25%
        let t = go(run, CANDLE, &bars(&v), &[("hsl_pct", 2.0)]);
        assert!((t[0].3 - 103.0 * 1.0025).abs() < 1e-9);
        // red / green use High > HSL: High exactly at the HSL does not trigger
        let mut v3 = flat(60);
        v3[27] = (99.0, 101.0, 98.0, 100.0); // green
        v3[28] = (100.0, 102.0, 99.0, 100.5);
        let t = go(run, CANDLE, &bars(&v3), &[("hsl_pct", 2.0), ("candle", 2.0)]);
        assert_eq!(t[0].2, 29);
        // ... but RANDOM uses High >= HSL
        assert_eq!(go(run, CANDLE, &bars(&v3), &[("hsl_pct", 2.0)])[0].2, 28);
    }

    #[test]
    fn profit_booking_on_the_low() {
        let mut v = flat(60);
        v[28] = (100.0, 100.5, 98.5, 99.5); // PT 99 (1%)
        let t = go(run, CANDLE, &bars(&v), &[("pt_pct", 1.0)]);
        assert_eq!((t[0].2, t[0].3), (28, 99.0));
    }

    #[test]
    fn election_months_are_skipped() {
        // May 2019: 1 May is a Wednesday; the 5th trading day is 7 May
        let r = bars_from("2019-04-01", &flat(60));
        let x = Ctx::new(&r, &r, &[]);
        let all = run(CANDLE, &x, 0, r.len() - 1, &Params::new()).unwrap();
        let may: Vec<_> = all.iter().filter(|t| r[t.legs[0].i].date.starts_with("2019-05")).collect();
        assert_eq!(may.len(), 1);
        assert_eq!(r[may[0].legs[0].i].date, "2019-05-07");
        let off = run(CANDLE, &x, 0, r.len() - 1, &ps(&[("no_election", 1.0)])).unwrap();
        assert!(off.iter().all(|t| !r[t.legs[0].i].date.starts_with("2019-05")));
        // ROC value AFL: election months off by default
        let _ = run(ROC_VALUE, &x, 0, r.len() - 1, &Params::new()).unwrap();
    }

    #[test]
    fn roc_value_entry() {
        let mut v = flat(60);
        // ROC(5) at bar 27: close 93 vs 100 five bars earlier = -7%
        v[27] = (95.0, 96.0, 92.0, 93.0);
        let t = go(run, ROC_VALUE, &bars(&v), &[]);
        assert_eq!((t[0].0, t[0].2), (27, 29));
        // +Value version needs ROC > 5
        assert!(go(run, ROC_VALUE, &bars(&v), &[("roc_ver", 1.0)]).iter().all(|t| t.0 != 27));
        // SL 2%: High above 93 x 1.02 = 94.86 on bar 28 covers there, also on a High = Close bar
        v[28] = (94.0, 95.0, 93.5, 95.0);
        let t = go(run, ROC_VALUE, &bars(&v), &[("sl_pct", 2.0)]);
        assert_eq!(t[0].2, 28);
        assert!((t[0].3 - 93.0 * 1.02).abs() < 1e-9);
    }

    #[test]
    fn rsi_adx_thresholds_gate_the_entry() {
        let r = wave(300);
        let base = go(run, CANDLE, &r, &[]);
        // RSI < 101 and ADX > -1 always hold once they exist
        let all = go(run, RSI_ADX, &r, &[("rsi_val", 101.0), ("adx_val", -1.0), ("no_election", 0.0)]);
        assert!(!all.is_empty() && all.iter().all(|t| base.iter().any(|b| b.0 == t.0)));
        assert!(go(run, RSI_ADX, &r, &[("rsi_val", 0.0)]).is_empty());
        assert!(go(run, ADX, &r, &[("adx_val", 100.0)]).is_empty());
    }

    #[test]
    fn roc_sign_and_threshold() {
        let mut v = flat(60);
        v[27] = (100.0, 104.0, 99.0, 103.0); // ROC(12) = +3%
        assert_eq!(go(run, ROC_ZERO, &bars(&v), &[])[0].0, 27);
        assert!(go(run, ROC_ZERO, &bars(&v), &[("roc_side", 1.0)]).iter().all(|t| t.0 != 27));
        // threshold 5: +3% is not enough; +7% is
        assert!(go(run, ROC_THR, &bars(&v), &[]).iter().all(|t| t.0 != 27));
        v[27] = (100.0, 108.0, 99.0, 107.0);
        assert_eq!(go(run, ROC_THR, &bars(&v), &[])[0].0, 27);
        // red / green options give no trades in the threshold AFL (no such loop blocks)
        assert!(go(run, ROC_THR, &bars(&v), &[("candle", 2.0)]).is_empty());
    }

    #[test]
    fn filter_versions() {
        let mut v = flat(60);
        v[27] = (100.0, 104.0, 99.0, 103.0);
        // NONE filter: bear version trades, bull version never shorts
        assert_eq!(go(run, ROC_ZERO_FLT, &bars(&v), &[("flt", 2.0)])[0].0, 27);
        assert!(go(run, ROC_ZERO_FLT, &bars(&v), &[("flt", 2.0), ("flt_side", 1.0)]).is_empty());
        // "no cut on transition" with NONE: FLTBUY == -1 covers the bar after entry
        let t = go(run, ROC_ZERO_FLT, &bars(&v), &[("flt", 2.0), ("flt_side", 2.0)]);
        assert_eq!((t[0].0, t[0].2), (27, 28));
        // threshold AFL with its default NONE filter
        v[27] = (100.0, 108.0, 99.0, 107.0);
        assert_eq!(go(run, ROC_THR_FLT, &bars(&v), &[])[0].0, 27);
        assert!(go(run, ROC_THR_FLT, &bars(&v), &[("flt_side", 1.0)]).is_empty());
        // ST / % swing AFL: Off is the plain rule; a rising close swing (up) blocks the bear short
        assert_eq!(go(run, ST_SWING, &bars(&flat(60)), &[])[0].0, 27);
        assert!(go(run, ST_SWING, &bars(&flat(60)), &[("flt", 2.0)]).is_empty());
        assert_eq!(go(run, ST_SWING, &bars(&flat(60)), &[("flt", 2.0), ("flt_side", 1.0)])[0].0, 27);
    }

    #[test]
    fn close_swing_flips() {
        let c = [100.0, 100.0, 70.0, 70.0, 110.0];
        // 23% below 100 = 77: 70 turns it down; 44% above 70 = 100.8: 110 turns it up
        assert_eq!(close_swing(&c, 23.0, 44.0, false), vec![1, 1, -1, -1, 1]);
    }

    #[test]
    fn calendar_days_next() {
        // Jan 2024: Fri 5 Jan (bar 4) entry; 7 Jan is a Sunday, so the cover is the first day on or after
        // 2 Feb (Fri, bar 24); Mon 5 Feb (bar 25) shorts again and Wed 7 Feb (bar 27) covers.
        let t = go(run, CALENDAR, &bars(&flat(40)), &[]);
        assert_eq!((t[0].0, t[0].2), (4, 24));
        assert_eq!((t[1].0, t[1].2), (25, 27));
    }

    #[test]
    fn fresh_st_swing_and_day_exits() {
        let v = flat(60);
        // flat: ST level 100 + 2 x 2 = 104 and swing 105 never hit: cover on the 12th trading day (bar 23 + 11)
        let t = go(run, FRESH, &bars(&v), &[]);
        assert_eq!((t[0].0, t[0].2), (27, 34));
        let mut v2 = v.clone();
        v2[29] = (100.0, 105.5, 99.5, 104.5); // close above the ST level 104
        let t = go(run, FRESH, &bars(&v2), &[]);
        assert_eq!(t[0].2, 29);
    }

    #[test]
    fn every_strategy_runs() {
        smoke(run, &IDS);
        for id in IDS {
            assert!(family_of(id).is_some());
            assert_eq!(catalogue().iter().filter(|d| d.id == id).count(), 1);
        }
    }
}
