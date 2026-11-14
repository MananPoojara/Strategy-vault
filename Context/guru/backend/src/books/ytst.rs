//! YT Strategies · Supertrend: the team's AmiBroker AFLs in "Old Topics/Supertrend" that trade one series on daily
//! bars (spot Nifty / Bank Nifty; the long-short "FOR COMMODITY" AFL on any daily series). Coded from the AFLs as
//! written, with the AFLs' own default Param values; there is no book PDF and no trade sheet was compared.
//!
//! What is coded: the plain ST long / short systems (one ST, ST entry with a two-ST exit, ST 5,1 short with a ST 5,4
//! filter), the regime systems with a monthly ST of the same series (STR Regime AFLs, the bull / bear "VER2" AFLs with
//! ATR, profit-booking and time-based exits, REGIM-2 and the two-ST same-day cross), the two loop AFLs that buy while
//! the monthly ST is in SELL ("Nifty SuperTrend Entry & ATR SL Exit" and "... Profit Booking or HARD SL or Trail SL
//! Exit"), four of the "Supertrend Long_Based on Probabilty" AFLs (long only after a virtual short gained / lost
//! x%), the three ATHC % swing AFLs (entry on an all-time-high close, plain / + 5% / after 2 consecutive losing
//! virtual trades, exit on a 10% close swing or a 5% target), the ST 5,2 scale-in AFL (up to 5 units, adds on a Close
//! above the last unit's Close + ATR(14)), "ST Buy Sell at High Low - Updated", the two ST 5,1 AFLs with a bearish /
//! bullish ST 5,4 filter and ATR target and stop ("For bear market" short only / long only, "For bull market" short
//! only) and MultiTimeFrame.afl (weekly ST filter, daily ST 5,1 / 5,2 entry and exit).
//!
//! Scale-in: one Trade holds every unit as a leg of 20% of the allocation (the AFL buys 20% of the current equity
//! per unit; marked as an adaptation in its notes).
//!
//! AmiBroker behaviour followed (as in `ytsw.rs` / `ytsn.rs`):
//! - the team's ST loop: band ATR = mean of the last `period` ATR(1) values (ATR(1) is Null on the first bar, so the
//!   bands are Null until bar `period`), bands ratchet, reset on a flip, trend 1 while the bands are Null;
//! - `VarSet(DV1, ..)` inside the AFLs' `ST()` function overwrites the globals named by its string arguments (BPD1 =
//!   the ST's up-trend line, BFACTOR1 = its down-trend line); the REGIM two-ST AFL trades on these lines;
//! - the filter timeframe `IIf(.., 2160001, ..)` is treated as calendar months (432001 as weeks); `TimeFrameExpand`
//!   uses expandLast: a monthly value reaches the daily bars on the month's last bar;
//! - the "Probabilty" AFLs' own ST (loop from bar `period`, no reset on a flip) and their weekly / monthly filter
//!   path (`Ref(-1)`, expandFirst, `Ref(+1)`) are `ytsn::sniper_st`'s;
//! - Buy / Sell / Short / Cover arrays are ExRem'd only where the AFL does so; trades at the close; exit first, then
//!   entry on the same bar; in long-short mode an opposite entry reverses; ApplyStop profit / N-bar stops with
//!   ActivateStopsImmediately off are checked from the bar after entry; prices are kept within the bar's range;
//! - MA on the first bars uses the bars available so far; a comparison with Null is never true; bar loops run over
//!   all loaded bars (their state starts at the data's first bar); the deadstock (default NO), penny (0) and
//!   turnover (off) settings are the AFLs' defaults and are not offered.
//!
//! Not coded (see each family's notes and the module's final report): the hourly / 25-75 minute AFLs and the "ST
//! Fresh Running Buy-Sell Index Intraday" AFLs (intraday bars); "ST monthly single trade ..." (three AFLs), "ST trade
//! below filter", "SMA BUY SELL WITH ALL TIMEFRAME" (stock-universe runs ranked by PositionScore, one on Foreign
//! NSE500); "BB 100 MA UB 3% LB1%" (already coded in `ytbb.rs`); the SMA and S% filter branches of the regime AFLs
//! (the S% branch reads an undefined `Strend1`); "data to excel" and "ATR PLOT" (exploration / plot only); "Entry
//! when Closing is above 5,1_Scale-in with 14 days ATR_Exit below 5,1.afl" (despite its folder name a stock-universe
//! AFL, "Rajiv Shukla Ver1": 52-week-high / 200-day-MA entry ranked by PositionScore = RSI(14), no Supertrend). The
//! "ST 5-1 Sell Buy R&R" V1 / V2 / Stock AFL copies are `ytst_st_sell_buy_pb` and its 'Final' copies
//! `ytst_st_sell_buy_atr` (see their notes).
#![allow(dead_code)]
use crate::books::ytroc::atr_wilder;
use crate::books::ytsn::sniper_st;
use crate::books::ytd7::{fills as d7_fills, Exit};
use crate::books::ytnr7::athc;
use crate::books::ytsw::{close_swing, compress, cross_up, expand_last, exrem, tf_of, trade, true_range, Bars};
use crate::books::ytwh::{bound, collect, hhv, loop_val, prec2, Ev};
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "yt_st";
pub const ALL: &str = "ytst_st_all";
pub const ALL_ATR: &str = "ytst_st_all_atr_exit";
pub const S51: &str = "ytst_st51_short_st54_filter";
pub const L5254: &str = "ytst_st52_54_long";
pub const SIMPLE_L: &str = "ytst_simple_long";
pub const SIMPLE_S: &str = "ytst_simple_short";
pub const DIFF_LS: &str = "ytst_diff_entry_exit_ls";
pub const BL: &str = "ytst_regime_bull";
pub const BR: &str = "ytst_regime_bear";
pub const BULL_ATR: &str = "ytst_bull_atr";
pub const BEAR: &str = "ytst_bear";
pub const BEAR_TIME: &str = "ytst_bear_time_exit";
pub const REGIM2: &str = "ytst_regim2";
pub const REGIM_2ST: &str = "ytst_regim_2st_cross";
pub const ATR_SL: &str = "ytst_st_sell_buy_atr";
pub const PB: &str = "ytst_st_sell_buy_pb";
pub const GAIN3: &str = "ytst_gain_short_v3";
pub const GAIN4: &str = "ytst_gain_short_v4";
pub const LOSS4: &str = "ytst_loss_short_v4";
pub const LOSS5: &str = "ytst_loss_short_v5";
pub const ATHC: &str = "ytst_athc_swing_pt";
pub const ATHC_PCT: &str = "ytst_athc_pct_swing_pt";
pub const ATHC_LOSS: &str = "ytst_athc_after_losses";
pub const SCALE52: &str = "ytst_st52_scale_in_atr";
pub const HL: &str = "ytst_st_high_low_breakout";
pub const BEAR_FLT: &str = "ytst_st51_bear_filter_atr";
pub const BULL_FLT: &str = "ytst_st51_bull_filter_short";
pub const MTF: &str = "ytst_mtf_weekly_filter";
pub const IDS: [&str; 28] = [
    ALL, ALL_ATR, S51, L5254, SIMPLE_L, SIMPLE_S, DIFF_LS, BL, BR, BULL_ATR, BEAR, BEAR_TIME, REGIM2, REGIM_2ST, ATR_SL, PB, GAIN3, GAIN4, LOSS4, LOSS5, ATHC,
    ATHC_PCT, ATHC_LOSS, SCALE52, HL, BEAR_FLT, BULL_FLT, MTF,
];

const GROUP: &str = "YT Strategies · Supertrend";
const NULL: f64 = -1e10;
const BIG: f64 = 9_999_999.0;

fn def(id: &'static str, name: &'static str, dir: &'static str, rules: &'static [&'static str], notes: &'static str) -> StrategyDef {
    StrategyDef { id, name, group: GROUP, session: 0, dir, status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim: None }
}

const ST_F: &str = "F|ST = the team's Supertrend: bands = calculation point ± factor × mean of the last `period` ATR(1); trend flips when the Close crosses the previous bar's band";
const M_F: &str = "F|Market filter: monthly Supertrend of the same series (default period 5, factor 1, mid point), expanded to daily bars on each month's last bar";
const CIRC_F: &str = "F|Upper circuit = Open, High and Close equal to 2 decimals; lower circuit = Open, Low and Close equal";
const SIDE_F: &str = "F|Side: long only, short only or long and short (default; an opposite entry reverses)";
const PROB_ST: &str = "F|ST1 / ST2: the AFL's own Supertrend (period 5, factors 1 and 1, on (Open+Close)/2; loop from bar `period`, no band reset on a flip); STBuy = Close > ST, STSell = Close < ST";
const PROB_SIG: &str = "F|Long entry signal: STBuy1 inside the test range; long exit signal: STSell1 and STSell2 inside the test range (the range's last day excluded, as the AFL's DateNum() < rangetoDate)";
const PROB_X: &str = "X|Sell at the Close on the exit signal, on the optional HSL / PB / TSL % (default 0 = off; 'live' fills at the level or Open − buffer), or on the filter turning against the trade";
const PROB_FLT: &str = "F|Optional filter (default off): the same series' ST on weekly / monthly bars, as the AFL sees it (previous completed period, the period's last day its own)";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![
        def(ALL, "ST Long-Short, Entry and Exit with the Same ST", "Long/Short", &[
            ST_F,
            "F|One ST: period 5, factor 2, on the mid point (H+L)/2",
            "E|Buy at the Close when the ST trend is up; Short when it is down",
            "X|Sell when the trend turns down; Cover when it turns up",
            SIDE_F,
        ], "ST ALL FOR INDIAN MKT - Entry & Exit with same ST.afl (folder of that name), coded as the AFL is written (ExRem'd signals, trades at the close). The copy in 'simple st' differs only in its Param ranges."),
        def(ALL_ATR, "ST Long-Short with ATR-Doubled Exit", "Long/Short", &[
            ST_F,
            "F|One ST: period 5, factor 2, on the mid point (H+L)/2",
            "F|ATR % = MA(ATR(1), 14) / Close × 100; its average = MA(ATR %, 3 years × 225 bars)",
            "E|Buy when the ST is up and ATR % < 2 × its average; Short when the ST is down and ATR % < 2 × its average",
            "X|Sell when the ST turns down or ATR % > 2 × its average; Cover when the ST turns up or ATR % > 2 × its average",
            SIDE_F,
        ], "ST ALL FOR INDIAN MKT with ATR exit condition.afl (ST entry_Exit when ATR gets doubled), coded as the AFL is written. The MA averages use the bars available so far at the start of the data. 'ATR PLOT WITH AVG ATR PCT.afl' in the same folder only plots these lines."),
        def(S51, "ST 5,1 Short Only with Daily ST 5,4 Filter", "Short", &[
            ST_F,
            "F|ST1 (trade): period 5, factor 1; ST2 (filter): period 5, factor 4; both on the mid point (calculation point option)",
            "E|Short at the Close when ST2 is up and ST1 is down; not on the first bar or a lower-circuit bar",
            "X|Cover when ST1 turns up or ST2 turns down, unless the bar is an upper circuit",
            CIRC_F,
        ], "ST 51 SHORT ONLY WITH ST 54 FLT.afl (Short & cover with 51 only when 54 DAILY filter is in BUY), coded as the AFL is written (ExRem'd). Its hard-stop and trading-start-bar Params are not used by the AFL."),
        def(L5254, "ST 5,2 Long with ST 5,4 Exit", "Long", &[
            ST_F,
            "F|ST1 (entry): period 5, factor 2; ST2 (exit): period 5, factor 4; calculation point default mid point",
            "E|Buy at the Close when ST1 is up; not on the first bar or an upper-circuit bar",
            "X|Sell when ST1 and ST2 are both down, unless the bar is a lower circuit",
            CIRC_F,
        ], "st 52 54 long only.afl (folder 'st 52 54 long only'), coded as the AFL is written (ExRem'd). The '3DAYS MA' calculation point is tested as \"3DYAS MA\" in the AFL, so choosing it keeps the mid point (kept as written). The copy in 'simple st' uses Optimize() with the same defaults. Its BUYSIG month-end block is computed but not used."),
        def(SIMPLE_L, "ST Simple Long (ST 5,1 Entry, ST 5,1 and 20,1 Exit)", "Long", &[
            ST_F,
            "F|ST1 (entry): period 5, factor 1; ST2 (exit): period 20, factor 1; both on the mid point (H+L)/2",
            "E|Buy at the Close when ST1 is up, after bar 5; not on an upper-circuit bar",
            "X|Sell when ST1 and ST2 are both down, unless the bar is a lower circuit",
            CIRC_F,
        ], "ST SIMPLE BUY SELL WITHOUT FLT with ratio long only VER2.3 - Copy.afl (Different ST Entry & exit / ST LONG only_Wo filter), coded as the AFL is written. Its ST function ignores the calculation-point list and always uses (H+L)/2. The R-multiple / SQN loop and custom backtester only report statistics. The copy in 'simple st' has Optimize() defaults 5,1 / 5,2 (set st2_period 5, st2_factor 2 for it)."),
        def(SIMPLE_S, "ST Simple Short (ST 5,2 and 5,1 Entry, ST 5,1 Exit)", "Short", &[
            ST_F,
            "F|ST1 (exit): period 5, factor 1; ST2 (entry): period 5, factor 2; both on the mid point (H+L)/2",
            "E|Short at the Close when ST2 and ST1 are both down, after bar 5; not on a lower-circuit bar",
            "X|Cover when ST1 turns up, unless the bar is a lower circuit (as written)",
            CIRC_F,
        ], "ST SIMPLE BUY SELL WITHOUT FLT with ratio short only.afl (Different ST Entry & exit / ST Short only_Wo filter), coded as the AFL is written; the cover is also blocked on a LOWER circuit bar, as in the AFL. Statistics loop and custom backtester are reporting only."),
        def(DIFF_LS, "ST Different Entry and Exit, Long-Short (Commodity)", "Long/Short", &[
            ST_F,
            "F|ST1: period 5, factor 0 (the bands are the calculation point itself); ST2: period 5, factor 1; calculation point default mid point",
            "E|Buy at the Close when ST1 is up, after bar 5",
            "E|Short when ST1 and ST2 are both down",
            "X|Sell when ST1 and ST2 are both down; Cover when ST1 turns up",
            SIDE_F,
        ], "ST WITH DIFF ENTRY EXIT Without filter_Long-Short_FOR COMMODITY.afl (Different ST Entry & exit / ST LONG-SHORT both_Wo filter), coded as the AFL is written, with its entry factor default 0. Written for commodity futures; runs on any daily series."),
        def(BL, "ST Bull Regime (Monthly ST Up): Long and Short", "Long/Short", &[
            ST_F, M_F,
            "F|ST1: period 5, factor 1; ST2: period 5, factor 2 (daily, on the calculation point, default mid point)",
            "E|Buy when the monthly ST is up and ST1 is up",
            "E|Short when the monthly ST is up and ST1 and ST2 are both down",
            "X|Sell when ST1 and ST2 are both down (option: also when the monthly ST turns down)",
            "X|Cover when ST1 turns up or the monthly ST turns down",
            SIDE_F,
        ], "ST-BL VER2.afl (STR Regime AFLs), coded as the AFL is written (its Optimize defaults). 'ST-BL VER2 (1).afl' is the same with Param defaults and the monthly-ST-down sell: set flt_exit 1 for it."),
        def(BR, "ST Bear Regime (Monthly ST Down): Long and Short", "Long/Short", &[
            ST_F, M_F,
            "F|ST1: period 5, factor 1; ST2: period 5, factor 1 (daily, on the calculation point, default mid point)",
            "E|Buy when the monthly ST is down and ST1 is up",
            "E|Short when the monthly ST is down and ST1 and ST2 are both down",
            "X|Sell when the monthly ST turns up or ST1 and ST2 are both down",
            "X|Cover when ST1 turns up",
            SIDE_F,
        ], "ST-BR VER2.afl (STR Regime AFLs), coded as the AFL is written. Its ATR risk position sizing (default NO) is not modelled."),
        def(BULL_ATR, "ST Bull Regime Long Only with ATR Filters", "Long", &[
            ST_F, M_F,
            "F|ST1: period 5, factor 4; ST2: period 5, factor 4 (daily, calculation point default mid point)",
            "F|ATR % = MA(ATR(1), 14) / Close × 100; its average = MA(ATR %, 675) after bar 687, 100 before",
            "E|Buy at the Close when the monthly ST is up, ST1 is up and ATR % < 2 × its average; after bar 5; not on an upper-circuit bar",
            "X|Sell (unless a lower-circuit bar) when ST2 is down (if ST1's factor > ST2's) or ST1 and ST2 are both down (otherwise), the monthly ST turns down, or ATR % > 2 × the ATR % of the last bar with monthly ST up and ST1 up",
            CIRC_F,
        ], "ST-BL VER2 with ratio n sqnno - Copy (2).afl (Different ST Entry & exit / ST LONG only_BULL mkt), coded as the AFL is written with its ST filter (the default). Its SMA filter branch compares the monthly close with a daily-timeframe MA array, and its S% branch reads an undefined Strend1 (an AmiBroker error), so those options are not offered. The R-multiple / SQN loop and custom backtester only report statistics."),
        def(BEAR, "ST Bear Regime with Profit Booking or Holding Period", "Long/Short", &[
            ST_F, M_F,
            "F|ST1: period 5, factor 4; ST2: period 5, factor 4 (daily, calculation point default mid point)",
            "E|Buy when the monthly ST is down and ST1 is up, after bar 5, not on an upper-circuit bar",
            "E|Short when the monthly ST is down and ST1 and ST2 are both down, not on an upper-circuit bar",
            "X|Sell (unless a lower-circuit bar) when ST2 is down (if ST1's factor > ST2's) or both are down (otherwise), or the monthly ST turns up",
            "X|Cover when ST1 turns up or the monthly ST turns up",
            "X|Optional ApplyStop profit % (0 off) and N-bar holding period (0 off), from the bar after entry",
            SIDE_F,
        ], "ST-BR VER2 with SQNNO N RISK OFRUIN.afl (Different ST Entry & exit / ST_Long only & Short only_BEAR mkt), coded as the AFL is written. The '... with profit booking.afl' copy adds ApplyStop(profit, 5%): set pb_pct 5; the '... with timebase exit.afl' copy adds ApplyStop(N-bar, 5): set hold_bars 5 (its short-day counter is not used by the signals). SQN / risk-of-ruin code only reports statistics."),
        def(BEAR_TIME, "ST Bear Regime with Time-Based Exit Only", "Long/Short", &[
            ST_F, M_F,
            "F|ST1: period 5, factor 4 (daily, calculation point default mid point)",
            "F|Flag: the first bar of a short setup (monthly ST down, ST1 down, not an upper circuit); its counter resets when ST1 turns up or after the holding period",
            "E|Buy when the monthly ST is down and ST1 is up, after bar 5, not on an upper-circuit bar",
            "E|Short when the monthly ST is down and ST1 is down, not on an upper-circuit bar",
            "X|Sell when ST1 is down or the monthly ST is up, and at least the holding period (5) has passed since the flag",
            "X|Cover when ST1 is up and at least the holding period has passed since the flag",
            "X|ApplyStop N-bar = the holding period, from the bar after entry",
            SIDE_F,
        ], "ST-BR VER2 with SQNNO N RISK OFRUIN with ONLY TIMEBASE EXIT FOR OPTIMIZE .afl (ST_Long only & Short only_BEAR mkt_With ONLY time based exit), coded as the AFL is written. BarsSince(flag) is Null before the first flag, so no time-based exit fires before it."),
        def(REGIM2, "REGIM-2: Bull Buy Only, Bear Buy and Short (Monthly ST)", "Long/Short", &[
            ST_F, M_F,
            "F|Long STs: B1 period 5, factor 1; B2 period 5, factor 2. Short STs: S1 period 5, factor 2; S2 period 5, factor 1",
            "E|Version 0: Buy when B1 is up (monthly up) or S2 is up (monthly down); Short when the monthly ST is down and S1, S2 are down",
            "X|Version 0: Sell when B1, B2 are down (monthly up) or S1, S2 are down (monthly down); Cover when S2 is up",
            "E|Versions 1-3: Buy when B1 is up; version 3 shorts when the monthly ST is down and S1 is down",
            "X|Version 1: Sell when B1, B2 are down; Cover when S1 and S2 are up or B1 is up",
            "X|Version 2: Sell when B1, B2 are down or (monthly down and S2 down); Cover when S1 and S2 are up",
            "X|Version 3: Sell when B1, B2 are down; Cover when S1 and S2 are up",
            SIDE_F,
        ], "REGIM -2 ALL- BL-BUY ONLY BR-BUY SHORT.afl (version 0) and its copies (1), (2), (3) with different signal lines (versions 1-3), STR Regime AFLs, coded as written with the ST filter (the default). Copies (2) and (3) default the S1 factor to 1 (set s1_factor 1). The SMA and S% filter branches and the ATR risk sizing (default NO) are not offered."),
        def(REGIM_2ST, "REGIM: Two-ST Same-Day Cross Entry, Monthly ST Filter", "Long/Short", &[
            ST_F, M_F,
            "F|ST1: period 5, factor 1; ST2: period 5, factor 2; exit ST: period 5, factor 2",
            "E|Buy when the monthly ST is up and the Close crosses above both STs' previous-bar down-trend lines on the same bar",
            "E|Short when the monthly ST is down and the Close crosses below both STs' previous-bar up-trend lines",
            "X|Sell when the exit ST is down or the Close crosses below both up-trend lines",
            "X|Cover when the exit ST is up or the Close crosses above both down-trend lines",
            SIDE_F,
        ], "REGIM - 2ST SAME DAY ENTRY BL-BUY ONLY BR-SHORT ONLY.afl (STR Regime AFLs), coded as the AFL is written: the lines are the BFACTOR / BPD globals its ST() function overwrites with VarSet. A down-trend line is Null on up-trend bars, so the cross fires as that line ends (AmiBroker Null handling). SMA / S% filter branches not offered."),
        def(ATR_SL, "Buy in Monthly ST Sell with ATR Target and Stop", "Long", &[
            "F|Monthly ST of the series: period 5, factor 1, mid point (expanded on each month's last bar)",
            "F|Optional ST 5,4 filter (monthly): buy only while it is up, sell when it turns down",
            "F|Re-buy state: armed when the monthly ST is down; after a hard-SL exit it waits for the monthly ST to turn from up to down",
            "E|Version 1: Buy at the Close when the monthly ST is down and armed; version 2: also down on the previous bar; version 3: down on the previous bar",
            "X|Sell at the Close when the Close ≥ entry + 3 × ATR(5) (re-armed), ≤ entry − 1 × ATR(5) (hard SL), or ≤ the ATR trailing SL (multiplier 0 = off)",
        ], "Nifty SuperTrend Entry & ATR SL Exit For Index & Fo Database.afl (folder root; same file in 'Entry when Closing is above 5,2.../Wo filter' and 'ST 5-1 Sell Buy R&R/Final/... Without 5,4 Filter', the Nifty copy using Optimize() with the same defaults), coded as the AFL is written. flt54 1 = '... with ST(54) Filter.afl' (root and 'Final/... 5,4 Filter' copies). ATR(5) is AmiBroker's Wilder ATR."),
        def(PB, "Buy in Monthly ST Sell (ST 20,1) with ATR Profit Booking, Hard and Trailing SL", "Long", &[
            "F|Monthly ST of the series: period 20, factor 1, mid point",
            "F|Re-buy cycle: the monthly ST must be up, then down; any exit restarts the cycle",
            "E|Version 1: Buy at the Close when the monthly ST is down in the cycle; version 2: also down on the previous bar; version 3: down on the previous bar",
            "F|No buy when High = Close, or when the next 30 bars repeat the bar's OHLC (frozen data, look-ahead as written)",
            "X|Sell at the Close when the Close ≥ entry + 10 × ATR(5), ≤ entry − 3 × ATR(5), or ≤ the highest Close since entry − 5 × ATR(5)",
            "X|On a bar with Low = Close the exit waits; it is taken at the next non-lower-circuit bar's Open − 0.25%",
        ], "Nifty SuperTrend Entry & Profit Booking or HARD SL or Trail SL Exit.afl (folder root; = 'ST 5-1 Sell Buy R&R/V1' and 'V2' with Param instead of Optimize), coded as the AFL is written. Its ST is on the INDEX via SetForeign (default NSENIFTY); here it is the traded series itself, as when run on Nifty. Version 3 is from the 'Stock AFL' copy (own-series ST, otherwise the same). ATR(5) is AmiBroker's Wilder ATR."),
        def(GAIN3, "Long after an x% Gain in a Virtual ST Short (Individual Profit)", "Long", &[
            PROB_ST, PROB_SIG, PROB_FLT,
            "F|Virtual short: opened at the Close on a long exit signal while no virtual short is open and the run is not continuing; closed on the next long entry signal (and filter)",
            "F|The run continues when the virtual short gained at least x% (default 5)",
            "E|Buy at the Close on a long entry signal while the run continues (and the filter allows; not on the last bar)",
            PROB_X,
            "X|After each sale the run stops when the trade made at least y% (default 0)",
        ], "Futures_Long_X%inShort_With Filter_V3.afl (Supertrend Long_Based on Probabilty / Long if x% Gain in Short), coded as the AFL is written; with the filter off it is Futures_Long_X%inShort_V3.afl. Comparisons with Null levels (HSL / PB off) are never true."),
        def(GAIN4, "Long after an x% Gain in a Virtual ST Short (Cumulative Profit)", "Long", &[
            PROB_ST, PROB_SIG, PROB_FLT,
            "F|Virtual short: opened at the Close on a long exit signal while flat (real and virtual) and the run is not continuing; closed on the next long entry signal (and filter)",
            "F|The run continues when the virtual short gained at least x% (default 5)",
            "E|Buy at the Close on a long entry signal while the run continues (and the filter allows; not on the last bar)",
            PROB_X,
            "X|The run stops once the long trades' cumulative % profit reaches y% (default 5); the total then restarts from 0",
        ], "Futures_Long_X%inShort_With Filter_V4.afl (Long if x% Gain in Short), the most complete of its group, coded as the AFL is written; with the filter off it is Futures_Long_X%inShort_V4.afl."),
        def(LOSS4, "Long after an x% Loss in a Virtual ST Short (Individual Profit)", "Long", &[
            PROB_ST, PROB_SIG, PROB_FLT,
            "F|Virtual short: opened at the Close on a long exit signal while no virtual short is open and the run is not continuing; closed on the next long entry signal",
            "F|The run continues when the virtual short's change is ≤ x% (default −5, i.e. a loss of at least 5%)",
            "E|Buy at the Close on a long entry signal while the run continues (and the filter allows; not on the last bar)",
            PROB_X,
            "X|After each sale the run stops when the trade made at least y% (default 0)",
        ], "Futures_Long_X%LossinShort_with_STFilter_V4.afl (Long if x% Loss in Short), coded as the AFL is written; with the filter off it is VirtualShortInLossandLong_ST_V4.afl. Filter timeframe default weekly."),
        def(LOSS5, "Long after an x% Loss in a Virtual ST Short (Cumulative Profit)", "Long", &[
            PROB_ST, PROB_SIG, PROB_FLT,
            "F|Virtual short: opened at the Close on a long exit signal while no virtual short is open and the run is not continuing (the cumulative total restarts there); closed on the next long entry signal",
            "F|The run continues when the virtual short's change is ≤ x% (default −5)",
            "E|Buy at the Close on a long entry signal while the run continues (and the filter allows; not on the last bar)",
            PROB_X,
            "X|The run stops once the long trades' cumulative % profit reaches y% (default 5)",
        ], "Futures_Long_X%LossinShort_with_STFilter_V5.afl (Long if x% Loss in Short), coded as the AFL is written; filter timeframe default monthly. VirtualShortInLossandLong_ST_V5.afl (no filter) also requires no real position to open a virtual short; the filter version drops that check, as coded here."),
        def(ATHC, "ATHC Entry with % Close Swing or Profit Target Exit", "Long", &[
            ATHC_F, SWING_F,
            "E|Buy at the Close when the Close beats the previous highest Close or the ATHC flag is on; not on an upper-circuit bar; after a sale not again until the swing trend has been down",
            ATHC_X,
        ], "Entry on ATHC and Exit below %Swing Or by ProfitTarget.afl (ATH entry_Exit with %swing OR profit target), coded as the AFL is written (its loop; no ExRem). Written for daily bars, positional long only; run here on the daily index. Its hard-stop % (default 0 = off), dead-stock (NO), penny (0) and turnover (off) settings are the defaults and are not offered; the R-multiple / SQN block is commented out in the AFL."),
        def(ATHC_PCT, "ATHC + 5% Entry with % Close Swing or Profit Target Exit", "Long", &[
            ATHC_F, SWING_F,
            "F|Trigger: on a new-high Close while no trigger is set, Trigger = that Close × (1 + 5%); cleared when the swing trend is down; else carried",
            "E|Buy at the Close when the ATHC entry holds and the Close is above the Trigger",
            ATHC_X,
        ], "Entry on ATHC + 5% and Exit below %Swing Or by ProfitTarget.afl (ATH+% entry_Exit with %swing OR profit target), coded as the AFL is written; otherwise the ATHC AFL line for line. The trigger is set on the new-high bar itself, so the buy comes on a later Close above it. A Null trigger reads -1e10 in the loop (only on a new high while the swing is down)."),
        def(ATHC_LOSS, "ATHC Entry after 2 Consecutive Losing Swing Trades", "Long", &[
            ATHC_F, SWING_F,
            "F|Virtual trades: buy at the Close when the ATHC flag is on (not an upper circuit), sell at the Close when the swing trend is down",
            "F|A virtual trade closed on the bar the swing turns down (Cross(Ref(TrendUp, -1), C)) counts with its % profit after 0.65% brokerage each way; loss count +1 when that profit ≤ 3% (the AFL's 'MIN PROFIT %'), reset to 0 by a trade above it",
            "E|Buy as the ATHC entry when the loss count is at least 2",
            ATHC_X,
        ], "ATHC entry After 2 consecutive losses and Exit below %Swing Or by ProfitTarget.afl (ATH entry after consec loss of ATH-%swing_Exit with %swing OR profit target), coded as the AFL is written; otherwise the ATHC AFL. The virtual trades run on their own and ignore the real ones; the brokerage 0.65 is the AFL's BROKERAGE Param, used here only for the loss count."),
        def(SCALE52, "ST 5,2 Long with ATR(14) Scale-in, ST 5,2 and 5,4 Exit", "Long", &[
            ST_F,
            "F|ST1 (entry): period 5, factor 2; ST2 (exit): period 5, factor 4; calculation point default mid point",
            "E|Buy one unit at the Close when ST1 is up; not on an upper-circuit bar",
            "E|Scale in one more unit (at most 5) at the Close when the Close is above the last unit's Close + 1 × ATR(14)",
            "X|Sell all at the Close when ST1 and ST2 are both down, unless a lower-circuit bar (a bar that scales in does not sell)",
        ], "Entry when Closing is above 5,2_Scale-in with 14 days ATR_Exit below 5,2 & 5,4.afl (folder of that name), coded as the AFL is written (its loop; Sell ExRem'd). Written for daily bars, positional long only. Adaptation: SetPositionSize(20, spsPercentOfEquity) buys 20% of the current equity per unit; here each unit is 20% of the trade's allocation (one trade, one leg per unit; units not bought are idle cash). ATR(14) is AmiBroker's Wilder ATR; while it is Null the loop reads the level as -1e10, so the first bars of the data scale in on every bar. The 'Nifty SuperTrend Entry & ATR SL Exit' copies in its subfolder are ytst_st_sell_buy_atr."),
        def(HL, "ST 5,1 Buy above the Flip High, Sell below the Flip Low", "Long", &[
            ST_F,
            "F|One ST: period 5, factor 1, on the calculation point (default mid point)",
            "F|Buy level: on the bar the ST turns up, its High + 0.25%, carried while the ST is up (Null while down); sell level: on the bar it turns down, its Low − 0.25%, carried while down (Null while up)",
            "F|Running trade: 1 when the High is above the buy level; 0 when it was 1 and the Low is below the sell level; else carried; a Null level is -1e10 in the loop",
            "E|Buy when flat and the High is above the previous bar's buy level or the running trade is 1: at the Open + 0.25% if the Open is above the level, else at the level",
            "X|Sell when the Low is below the previous bar's sell level or the running trade is 0, unless a lower-circuit bar: at the Open − 0.25% if the Open is below the level, else at the level",
        ], "ST Buy Sell at High Low - Updated.afl (ST Buy Sell at High Low; the copy in 'ST Buy when HIGH above HIGH+0.25% & Sell when LOW below LOW-0.25%' is the same file), coded as the AFL is written (its loop; no ExRem). Written for daily bars, positional long only. Inside the loop a Null level is -1e10, so while the ST is down (and on the first bars) 'High > buy level' holds: the running trade is 1 and the AFL buys at the Open + 0.25% on every flat bar there, selling on a break of the down-flip Low. A buy and a sale on the same bar: the backtester enters and ignores that bar's sale (AllowSameBarExit is not set). Fills are kept within the bar's range."),
        def(BEAR_FLT, "ST 5,1 with Bearish ST 5,4 Filter: Short Only or Long Only, ATR Target and Stop", "Long/Short", &[
            ST_F, FLT_ATR_F,
            "F|Re-entry state: armed while ST(5/1) is up (long only: down); after an exit with ST(5/1) up (long only: down) it waits for ST(5/1) to turn from down to up (long only: up to down)",
            "E|Short only (default): Short at the Close when the previous bar's ST(5/1) was up, the ST(5/4) filter is down and the state is armed (from bar 5)",
            "X|Cover when the Low reaches entry − 3 × ATR(5) (at the level, or the Open + 0.25% if it opened below), the High reaches entry + 1 × ATR(5) (at the level, or the Open + 0.25% if it opened above), or the filter turns up (at the Close)",
            "E|Long only: Buy at the Close when the previous bar's ST(5/1) was down, the filter is down and the state is armed",
            "X|Sell when the High reaches entry + 3 × ATR(5) (at the level, or the Open − 0.25% if it opened above), the Low reaches entry − 1 × ATR(5) (at the level, or the Open − 0.25% if it opened below), or the filter turns up (at the Close)",
        ], "Nifty SuperTrend Short Only and Long Only & ATR SL Exit For Index & Fo with ST Bear Filter.afl (Nifty SuperTrend Entry & ATR SL Exit For Index & Fo Database with ST(54) Filter / For bear market (Long only & short only) (Ver3 If NIFTY in ST sell, then buy on next day)), coded as the AFL is written (its loop from bar = ST period; no ExRem). Written for spot Nifty (index & F&O database) on daily bars; both STs default to the daily timeframe in this AFL. ATR(5) is AmiBroker's Wilder ATR; a Null ATR makes the levels Null (-1e10 in the loop)."),
        def(BULL_FLT, "ST 5,1 Short Only with Bullish ST 5,4 Filter, ATR Target and Stop", "Short", &[
            ST_F, FLT_ATR_F,
            "F|Re-entry state: armed while ST(5/1) is up; after a cover with ST(5/1) up it waits for ST(5/1) to turn from down to up",
            "E|Short at the Close when the previous bar's ST(5/1) was up, the ST(5/4) filter is up and the state is armed (from bar 5)",
            "X|Cover when the Low reaches entry − 3 × ATR(5) (at the level, or the Open + 0.25% if it opened below), the High reaches entry + 1 × ATR(5) (at the level, or the Open − 0.25% if it opened above, as written), or the filter turns down (at the Close)",
        ], "Nifty SuperTrend Short Only &  Index & Fo with ST Bull Filter.AFL (Nifty SuperTrend Entry & ATR SL Exit For Index & Fo Database with ST(54) Filter / For bull market (Short only) (Ver3 If NIFTY in ST sell, then buy on next day)), coded as the AFL is written (its Optimize() defaults; loop from bar = ST period; no ExRem). Written for spot Nifty (index & F&O database) on daily bars; both STs default to the daily timeframe. The stop-loss cover above a gap is the Open MINUS the buffer, as in the AFL."),
        def(MTF, "Multi-Timeframe ST: Weekly ST Filter, Daily ST 5,1 Entry, ST 5,1 / 5,2 Exit", "Long", &[
            "F|The AFL's own ST (loop from bar `period`, no band reset on a flip), on (H+L)/2 by default",
            "F|Filter: that ST (period 5, factor 1) on weekly bars as the daily bar sees it (previous completed week, the week's last day its own); bull phase = the week's Close above it",
            "F|ST1 / ST2: that ST on daily bars, period 5, factors 1 and 2",
            "E|Buy at the Close when the Close crosses above ST1 (option 'Running': is above ST1) and the filter is in its bull phase (option: bear phase)",
            "X|Sell at the Close when the Close is below ST1 before it has closed above ST2 since entry, below ST2 after that, or when the filter leaves the entry phase (option: no filter exit); a new entry can follow on the same bar",
        ], "MultiTimeFrame.afl (MultiTime Frame Supertrend), coded as the AFL is written with its default daily entry timeframe. A daily-bar trading system with a weekly (or monthly) Supertrend filter. Its header ('N Red Candles', '5-2 5-4') is stale: the code's entry STs are 5,1 and 5,2. The weekly / monthly entry-timeframe and quarterly filter options are not offered; its dead-stock lines are not used by its loop."),
    ]
}

const ATHC_F: &str = "F|ATHC flag: on when the Close beats the highest earlier Close (HHV(C, BarCount), the bars so far); off when the swing trend is down; else carried";
const SWING_F: &str = "F|% swing on the Close (default 10%): bands Close ± 10%, the lower one only rising in an up trend and the upper one only falling in a down trend, reset on a flip; the trend flips when the Close crosses the previous bar's band";
const ATHC_X: &str = "X|Sell when the swing trend is down, the ATHC flag is off or the High reaches entry × (1 + 5%), unless a lower-circuit bar: at the target when the High reaches it (kept within the bar), else at the Close";
const FLT_ATR_F: &str = "F|ST(5/1): period 5, factor 1; ST(5/4) filter: period 5, factor 4; both on daily bars by default (weekly / monthly options, expanded on each period's last bar), calculation point default mid point";

// ---------------------------------------------------------------- params

const POINT3: &str = "ST calculation point (1 mid point, 2 close, 3 pivot)";
const POINT6: &str = "ST calculation point (1 mid point, 2 close, 3 pivot, 4 avg of mid / pivot / close, 5 3-day MA, 6 5-day MA)";
const SIDE: &str = "Side (1 long only, 2 short only, 3 long and short)";

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    let side = || p("side", SIDE, 3.0, 1.0, 3.0, 1.0);
    let st = |k: &'static str, l: &'static str, d: f64| p(k, l, d, 1.0, 40.0, 1.0);
    let fa = |k: &'static str, l: &'static str, d: f64| p(k, l, d, 0.0, 20.0, 0.5);
    let flt = || {
        vec![
            p("flt_tf", "Market filter timeframe (1 monthly, 2 weekly, 3 daily)", 1.0, 1.0, 3.0, 1.0),
            st("flt_period", "Market filter ST period", 5.0),
            fa("flt_factor", "Market filter ST factor", 1.0),
        ]
    };
    let prob = |x: f64, y: f64, tf: f64, tf_label: &'static str| {
        vec![
            p("x_pct", "X% in the virtual short trade", x, -100.0, 100.0, 0.1),
            p("y_pct", "Y% profit in the long trades", y, 0.0, 100.0, 0.1),
            st("st_period", "ST period", 5.0),
            p("st_factor1", "ST factor 1", 1.0, 1.0, 10.0, 0.1),
            p("st_factor2", "ST factor 2", 1.0, 1.0, 10.0, 0.1),
            p("st_avg", AVG, 4.0, 0.0, 7.0, 1.0),
            p("exit_live", "Exit at (0 close, 1 live)", 0.0, 0.0, 1.0, 1.0),
            p("hsl_pct", "HSL % (0 off)", 0.0, 0.0, 100.0, 0.1),
            p("pb_pct", "PB % (0 off)", 0.0, 0.0, 100.0, 0.1),
            p("tsl_pct", "TSL % (0 off)", 0.0, 0.0, 100.0, 0.1),
            p("buffer", "Exit buffer %", 0.25, 0.01, 1.0, 0.01),
            p("flt", "ST filter (0 off, 1 on)", 0.0, 0.0, 1.0, 1.0),
            p("flt_tf", tf_label, tf, 1.0, 3.0, 1.0),
            st("flt_period", "ST filter period", 5.0),
            p("flt_factor", "ST filter factor", 1.0, 1.0, 10.0, 0.1),
            p("flt_avg", AVG, 4.0, 0.0, 7.0, 1.0),
            p("flt_bear", "Long when the filter is (0 bullish, 1 bearish)", 0.0, 0.0, 1.0, 1.0),
        ]
    };
    const TF_LBL: &str = "ST filter timeframe (2 weekly, 3 monthly)";
    const TF3_LBL: &str = "ST filter timeframe (1 daily, 2 weekly, 3 monthly)";
    Some(match id {
        ALL => vec![st("st_period", "ST period", 5.0), fa("st_factor", "ST factor", 2.0), side()],
        ALL_ATR => vec![
            st("st_period", "ST period", 5.0),
            fa("st_factor", "ST factor", 2.0),
            p("atr_period", "ATR period for exit", 14.0, 1.0, 50.0, 1.0),
            p("atr_years", "ATR average years (× 225 bars)", 3.0, 1.0, 10.0, 1.0),
            side(),
        ],
        S51 => vec![
            st("st1_period", "ST period for short entry / exit", 5.0),
            fa("st1_factor", "ST factor for short entry / exit", 1.0),
            st("st2_period", "ST period for the filter", 5.0),
            fa("st2_factor", "ST factor for the filter", 4.0),
            p("point", POINT3, 1.0, 1.0, 3.0, 1.0),
        ],
        L5254 | DIFF_LS => {
            let (f1, f2) = if id == L5254 { (2.0, 4.0) } else { (0.0, 1.0) };
            let mut v = vec![
                st("st1_period", "ST period for entry", 5.0),
                fa("st1_factor", "ST factor for entry", f1),
                st("st2_period", "ST period for exit", 5.0),
                fa("st2_factor", "ST factor for exit", f2),
                p("point", POINT6, 1.0, 1.0, 6.0, 1.0),
            ];
            if id == DIFF_LS {
                v.push(side());
            }
            v
        }
        SIMPLE_L => vec![
            st("st1_period", "ST period for entry", 5.0),
            fa("st1_factor", "ST factor for entry", 1.0),
            st("st2_period", "ST period for exit", 20.0),
            fa("st2_factor", "ST factor for exit", 1.0),
        ],
        SIMPLE_S => vec![
            st("st1_period", "ST period for short exit", 5.0),
            fa("st1_factor", "ST factor for short exit", 1.0),
            st("st2_period", "ST period for short entry", 5.0),
            fa("st2_factor", "ST factor for short entry", 2.0),
        ],
        BL | BR => {
            let mut v = flt();
            v.extend([
                st("st1_period", "ST1 period", 5.0),
                fa("st1_factor", "ST1 factor", 1.0),
                st("st2_period", "ST2 period", 5.0),
                fa("st2_factor", "ST2 factor", if id == BL { 2.0 } else { 1.0 }),
                p("point", POINT3, 1.0, 1.0, 3.0, 1.0),
            ]);
            if id == BL {
                v.push(p("flt_exit", "Also sell when the monthly ST turns down (0 ST-BL VER2, 1 ST-BL VER2 (1))", 0.0, 0.0, 1.0, 1.0));
            }
            v.push(side());
            v
        }
        BULL_ATR | BEAR | BEAR_TIME => {
            let mut v = flt();
            v.push(p("flt_point", "Monthly ST calculation point (1 mid point, 2 close, 3 pivot, 4 avg, 5 3-bar MA, 6 5-bar MA)", 1.0, 1.0, 6.0, 1.0));
            v.extend([st("st1_period", "ST1 period", 5.0), fa("st1_factor", "ST1 factor", 4.0)]);
            if id != BEAR_TIME {
                v.extend([st("st2_period", "ST2 period", 5.0), fa("st2_factor", "ST2 factor", 4.0)]);
            }
            v.push(p("point", POINT6, 1.0, 1.0, 6.0, 1.0));
            if id == BEAR {
                v.push(p("pb_pct", "Profit booking % (ApplyStop, 0 off)", 0.0, 0.0, 20.0, 1.0));
                v.push(p("hold_bars", "Holding period, N-bar stop (0 off)", 0.0, 0.0, 1000.0, 1.0));
            }
            if id == BEAR_TIME {
                v.push(p("hold_bars", "Holding period", 5.0, 1.0, 500.0, 1.0));
            }
            if id != BULL_ATR {
                v.push(side());
            }
            v
        }
        REGIM2 => {
            let mut v = flt();
            v.extend([
                st("b1_period", "ST period for long entry", 5.0),
                fa("b1_factor", "ST factor for long entry", 1.0),
                st("b2_period", "ST period for long exit", 5.0),
                fa("b2_factor", "ST factor for long exit", 2.0),
                st("s1_period", "ST period for short entry", 5.0),
                fa("s1_factor", "ST factor for short entry", 2.0),
                st("s2_period", "ST period for short exit", 5.0),
                fa("s2_factor", "ST factor for short exit", 1.0),
                p("point", POINT3, 1.0, 1.0, 3.0, 1.0),
                p("version", "Version (0 REGIM -2 ALL, 1-3 its copies (1)-(3))", 0.0, 0.0, 3.0, 1.0),
                side(),
            ]);
            v
        }
        REGIM_2ST => {
            let mut v = flt();
            v.extend([
                st("st1_period", "ST period for entry 1", 5.0),
                fa("st1_factor", "ST factor for entry 1", 1.0),
                st("st2_period", "ST period for entry 2", 5.0),
                fa("st2_factor", "ST factor for entry 2", 2.0),
                st("st3_period", "ST period for exit", 5.0),
                fa("st3_factor", "ST factor for exit", 2.0),
                p("point", POINT3, 1.0, 1.0, 3.0, 1.0),
                side(),
            ]);
            v
        }
        ATR_SL | PB => {
            let pb = id == PB;
            vec![
                p("version", "Entry version (1, 2, 3)", 1.0, 1.0, 3.0, 1.0),
                p("atr_period", "ATR periods", 5.0, 1.0, 100.0, 1.0),
                p("pt_mult", "ATR profit target multiplier", if pb { 10.0 } else { 3.0 }, 0.0, 12.0, 1.0),
                p("sl_mult", "ATR hard SL multiplier", if pb { 3.0 } else { 1.0 }, 0.0, 3.0, 1.0),
                p("tsl_mult", "ATR trailing SL multiplier (0 off)", if pb { 5.0 } else { 0.0 }, 0.0, 20.0, 0.001),
                p("flt_tf", "ST timeframe (1 monthly, 2 weekly, 3 daily)", 1.0, 1.0, 3.0, 1.0),
                p("flt_point", POINT3, 1.0, 1.0, 3.0, 1.0),
                p("flt_period", "ST period", if pb { 20.0 } else { 5.0 }, 1.0, 40.0, 1.0),
                p("flt_factor", "ST factor", 1.0, 0.0, 40.0, 1.0),
            ]
            .into_iter()
            .chain(if pb {
                vec![p("buffer", "Selling buffer %", 0.25, 0.0, 10.0, 0.05)]
            } else {
                vec![
                    p("flt54", "ST(54) filter (0 off, 1 on: '... with ST(54) Filter.afl')", 0.0, 0.0, 1.0, 1.0),
                    p("flt54_tf", "ST(54) filter timeframe (1 monthly, 2 weekly, 3 daily)", 1.0, 1.0, 3.0, 1.0),
                    p("flt54_point", "ST(54) calculation point (1 mid point, 2 close, 3 pivot)", 1.0, 1.0, 3.0, 1.0),
                    p("flt54_period", "ST(54) period", 5.0, 1.0, 40.0, 1.0),
                    p("flt54_factor", "ST(54) factor", 4.0, 0.0, 40.0, 1.0),
                ]
            })
            .collect()
        }
        GAIN3 => prob(5.0, 0.0, 2.0, TF_LBL),
        GAIN4 => prob(5.0, 5.0, 2.0, TF_LBL),
        LOSS4 => prob(-5.0, 0.0, 2.0, TF_LBL),
        LOSS5 => prob(-5.0, 5.0, 3.0, TF3_LBL),
        ATHC | ATHC_PCT | ATHC_LOSS => {
            let mut v = vec![p("exit_pct", "EXIT % (swing)", 10.0, 1.0, 20.0, 1.0), p("pt_pct", "PT percent", 5.0, 0.0, 20.0, 0.05)];
            if id == ATHC_PCT {
                v.push(p("athc_pct", "Buy percentage from ATHC", 5.0, 1.0, 10.0, 1.0));
            }
            if id == ATHC_LOSS {
                v.extend([
                    p("loss_pct", "Min profit % (a virtual trade at or below it is a loss)", 3.0, 1.0, 3.0, 0.05),
                    p("n_loss", "Consecutive losses", 2.0, 1.0, 2.0, 1.0),
                    p("comm", "Brokerage % (virtual trades)", 0.65, 0.01, 1.0, 0.01),
                ]);
            }
            v
        }
        SCALE52 => vec![
            st("st1_period", "ST period for entry", 5.0),
            fa("st1_factor", "ST factor for entry", 2.0),
            st("st2_period", "ST period for exit", 5.0),
            fa("st2_factor", "ST factor for exit", 4.0),
            p("point", POINT3, 1.0, 1.0, 3.0, 1.0),
            p("atr_period", "ATR periods", 14.0, 5.0, 100.0, 1.0),
            p("atr_mult", "ATR multiplier", 1.0, 0.0, 2.0, 0.025),
        ],
        HL => vec![
            st("st_period", "ST period", 5.0),
            fa("st_factor", "ST factor", 1.0),
            p("point", POINT3, 1.0, 1.0, 3.0, 1.0),
            p("buffer", "Buffer %", 0.25, 0.0, 2.0, 0.001),
        ],
        BEAR_FLT | BULL_FLT => {
            let mut v = Vec::new();
            if id == BEAR_FLT {
                v.push(p("side", "Entry / exit type (1 short only, 2 long only)", 1.0, 1.0, 2.0, 1.0));
            }
            v.extend([
                p("atr_period", "ATR periods", 5.0, 1.0, 100.0, 1.0),
                p("pt_mult", "ATR profit target multiplier", 3.0, 1.0, 12.0, 1.0),
                p("sl_mult", "ATR hard SL multiplier", 1.0, 1.0, 3.0, 1.0),
                p("buffer", if id == BEAR_FLT { "Sell / cover buffer %" } else { "Cover buffer %" }, 0.25, 0.0, 10.0, 0.05),
                p("st_tf", "ST(5/1) timeframe (1 monthly, 2 weekly, 3 daily)", 3.0, 1.0, 3.0, 1.0),
                p("st_point", POINT3, 1.0, 1.0, 3.0, 1.0),
                st("st_period", "ST(5/1) period", 5.0),
                fa("st_factor", "ST(5/1) factor", 1.0),
                p("flt_tf", "ST(5/4) filter timeframe (1 monthly, 2 weekly, 3 daily)", 3.0, 1.0, 3.0, 1.0),
                p("flt_point", POINT3, 1.0, 1.0, 3.0, 1.0),
                st("flt_period", "ST(5/4) filter period", 5.0),
                fa("flt_factor", "ST(5/4) filter factor", 4.0),
            ]);
            v
        }
        MTF => vec![
            p("flt_tf", TF_LBL, 2.0, 2.0, 3.0, 1.0),
            p("flt_period", "ST filter period", 5.0, 1.0, 5.0, 1.0),
            p("flt_factor", "ST filter factor", 1.0, 1.0, 25.0, 1.0),
            p("flt_avg", AVG, 5.0, 0.0, 7.0, 1.0),
            p("flt_bear", "ST filter type (0 bullish, 1 bearish)", 0.0, 0.0, 1.0, 1.0),
            p("flt_skip_exit", "Skip filter exit (0 off, 1 on)", 0.0, 0.0, 1.0, 1.0),
            p("st_period", "ST period", 5.0, 3.0, 9.0, 2.0),
            p("st_factor1", "ST factor 1", 1.0, 0.0, 5.0, 1.0),
            p("st_factor2", "ST factor 2", 2.0, 0.0, 5.0, 1.0),
            p("st_avg", AVG, 5.0, 0.0, 7.0, 1.0),
            p("running", "ST entry type (0 fresh cross, 1 running)", 0.0, 0.0, 1.0, 1.0),
        ],
        _ => return None,
    })
}

const AVG: &str = "ST average value (0 close, 1 open, 2 high, 3 low, 4 (O+C)/2, 5 (H+L)/2, 6 (H+L+C)/3, 7 (O+H+L+C)/4)";

// ---------------------------------------------------------------- indicators

/// AFL MA(v, n) with the bars available so far at the start; Null inputs are skipped (None if none in the window).
pub(crate) fn ma_partial(v: &[Option<f64>], n: usize) -> Vec<Option<f64>> {
    let n = n.max(1);
    (0..v.len())
        .map(|i| {
            let w: Vec<f64> = v[(i + 1).saturating_sub(n)..=i].iter().flatten().copied().collect();
            if w.is_empty() { None } else { Some(w.iter().sum::<f64>() / w.len() as f64) }
        })
        .collect()
}

/// The AFLs' ST calculation point on bars (h, l, c): 1 mid point, 2 close, 3 pivot, 4 (mid + pivot + close) / 3,
/// 5 MA(C, 3), 6 MA(C, 5). `ma3_typo`: the AFL tests "3DYAS MA", so option 5 keeps the mid point.
pub(crate) fn calc_on(h: &[f64], l: &[f64], c: &[f64], kind: i64, ma3_typo: bool) -> Vec<f64> {
    let n = c.len();
    let mid: Vec<f64> = (0..n).map(|i| (h[i] + l[i]) / 2.0).collect();
    let cc: Vec<Option<f64>> = c.iter().map(|&v| Some(v)).collect();
    match kind {
        2 => c.to_vec(),
        3 => (0..n).map(|i| (h[i] + l[i] + c[i]) / 3.0).collect(),
        4 => (0..n).map(|i| (mid[i] + (h[i] + l[i] + c[i]) / 3.0 + c[i]) / 3.0).collect(),
        5 if !ma3_typo => ma_partial(&cc, 3).into_iter().map(|v| v.unwrap_or(f64::NAN)).collect(),
        6 => ma_partial(&cc, 5).into_iter().map(|v| v.unwrap_or(f64::NAN)).collect(),
        _ => mid,
    }
}

/// The team's ST loop (as `ytsw::supertrend`), returning (trend, up-trend line, down-trend line) as the AFL's
/// TrendUp / TrendDown arrays: Dn on up-trend bars, Up on down-trend bars, and on the bar before a flip the other
/// line's value.
pub(crate) fn st_lines(calc: &[f64], h: &[f64], l: &[f64], c: &[f64], period: usize, f: f64) -> (Vec<i8>, Vec<Option<f64>>, Vec<Option<f64>>) {
    let n = c.len();
    let trr = true_range(h, l, c);
    let p = period.max(1);
    let satr: Vec<Option<f64>> = (0..n)
        .map(|i| if i + 1 < p { None } else { trr[i + 1 - p..=i].iter().try_fold(0.0, |s, v| v.map(|v| s + v)).map(|s| s / p as f64) })
        .collect();
    let mut up: Vec<Option<f64>> = (0..n).map(|i| satr[i].map(|s| calc[i] + f * s)).collect();
    let mut dn: Vec<Option<f64>> = (0..n).map(|i| satr[i].map(|s| calc[i] - f * s)).collect();
    let mut tr = vec![1i8; n];
    let (mut lu, mut ld) = (vec![None; n], vec![None; n]);
    for i in 1..n {
        tr[i] = match (up[i - 1], dn[i - 1]) {
            (Some(u), _) if c[i] > u => 1,
            (_, Some(d)) if c[i] < d => -1,
            _ => tr[i - 1],
        };
        if let (true, Some(a), Some(b)) = (tr[i] > 0, dn[i], dn[i - 1]) {
            if a < b {
                dn[i] = Some(b);
            }
        }
        if let (true, Some(a), Some(b)) = (tr[i] < 0, up[i], up[i - 1]) {
            if a > b {
                up[i] = Some(b);
            }
        }
        if tr[i] < 0 && tr[i - 1] > 0 {
            up[i] = satr[i].map(|s| calc[i] + f * s);
        }
        if tr[i] > 0 && tr[i - 1] < 0 {
            dn[i] = satr[i].map(|s| calc[i] - f * s);
        }
        if tr[i] == 1 {
            lu[i] = dn[i];
            if tr[i - 1] == -1 {
                lu[i - 1] = ld[i - 1];
            }
        } else {
            ld[i] = up[i];
            if tr[i - 1] == 1 {
                ld[i - 1] = lu[i - 1];
            }
        }
    }
    (tr, lu, ld)
}

fn st_daily(x: &Ctx, calc: &[f64], period: usize, f: f64) -> Vec<i8> {
    st_lines(calc, &x.h, &x.l, &x.c, period, f).0
}

/// ST trend of the series on the filter timeframe (1 monthly, 2 weekly, 3 daily), expandLast to daily bars.
pub(crate) fn tf_trend(x: &Ctx, tf: i64, point: i64, period: usize, f: f64) -> Vec<i8> {
    let b: Bars = compress(x, tf_of(tf, true));
    let cp = calc_on(&b.h, &b.l, &b.c, point, false);
    let (tr, _, _) = st_lines(&cp, &b.h, &b.l, &b.c, period, f);
    expand_last(&tr, &b)
}

fn up_circuit(x: &Ctx, i: usize) -> bool {
    prec2(x.o[i]) == prec2(x.c[i]) && prec2(x.o[i]) == prec2(x.h[i])
}
fn lo_circuit(x: &Ctx, i: usize) -> bool {
    prec2(x.o[i]) == prec2(x.c[i]) && prec2(x.o[i]) == prec2(x.l[i])
}

/// ATR % = MA(ATR(1), n) / C × 100 (partial MA at the start).
fn atr_pct(x: &Ctx, n: usize) -> Vec<Option<f64>> {
    let avg = ma_partial(&true_range(&x.h, &x.l, &x.c), n);
    (0..x.c.len()).map(|i| avg[i].map(|a| a / x.c[i] * 100.0)).collect()
}

fn shift1(v: &[Option<f64>]) -> Vec<Option<f64>> {
    (0..v.len()).map(|i| if i == 0 { None } else { v[i - 1] }).collect()
}

/// AFL Cross(a, b) / Cross(b, a) with b possibly Null: no cross before b's first value, Null = -1e10 after it.
fn cross(a: &[f64], b: &[Option<f64>], above: bool) -> Vec<bool> {
    let first = b.iter().position(|v| v.is_some()).unwrap_or(b.len());
    let v = |z: Option<f64>| z.unwrap_or(NULL);
    (0..a.len())
        .map(|i| i > first && if above { a[i] > v(b[i]) && a[i - 1] <= v(b[i - 1]) } else { v(b[i]) > a[i] && v(b[i - 1]) <= a[i - 1] })
        .collect()
}

// ---------------------------------------------------------------- backtester

struct Sig {
    buy: Vec<bool>,
    sell: Vec<bool>,
    short: Vec<bool>,
    cover: Vec<bool>,
}

impl Sig {
    fn new(n: usize) -> Self {
        Sig { buy: vec![false; n], sell: vec![false; n], short: vec![false; n], cover: vec![false; n] }
    }
}

/// AmiBroker regular backtest at the close of ExRem'd signals (side 1 long only, 2 short only, 3 long and short,
/// reversing), with optional ApplyStop profit % and N-bar stops (0 off) checked from the bar after entry.
fn backtest(x: &Ctx, s: &Sig, side: i64, pb_pct: f64, nbar: usize, from: usize, to: usize) -> Vec<Trade> {
    let buy = exrem(&s.buy, &s.sell);
    let sell = exrem(&s.sell, &buy);
    let short = exrem(&s.short, &s.cover);
    let cover = exrem(&s.cover, &short);
    let (longs, shorts) = (side != 2, side != 1);
    let mut out = Vec::new();
    let mut pos: Option<(f64, usize)> = None;
    for i in from..=to {
        if let Some((d, e)) = pos {
            let epx = x.c[e];
            let mut ex: Option<(f64, &str)> = None;
            if i > e && pb_pct > 0.0 {
                let lv = epx * (1.0 + d * pb_pct / 100.0);
                if d > 0.0 && x.h[i] >= lv {
                    ex = Some((if x.o[i] > lv { x.o[i] } else { lv }, "Profit stop"));
                } else if d < 0.0 && x.l[i] <= lv {
                    ex = Some((if x.o[i] < lv { x.o[i] } else { lv }, "Profit stop"));
                }
            }
            if ex.is_none() && nbar > 0 && i >= e + nbar {
                ex = Some((x.c[i], "N-bar stop"));
            }
            if ex.is_none() {
                let sig = if d > 0.0 { sell[i] || (side == 3 && short[i]) } else { cover[i] || (side == 3 && buy[i]) };
                if sig {
                    ex = Some((x.c[i], if d > 0.0 { "Sell signal" } else { "Cover signal" }));
                }
            }
            if let Some((px, r)) = ex {
                out.push(trade(d, e, epx, i, bound(x, i, px), r, false));
                pos = None;
            }
        }
        if pos.is_none() {
            if longs && buy[i] {
                pos = Some((1.0, i));
            } else if shorts && short[i] {
                pos = Some((-1.0, i));
            }
        }
    }
    if let Some((d, e)) = pos {
        out.push(trade(d, e, x.c[e], to, x.c[to], "open", true));
    }
    out
}

// ---------------------------------------------------------------- signal strategies

fn signals(id: &str, x: &Ctx, g: &dyn Fn(&str) -> f64) -> Sig {
    let n = x.c.len();
    let gi = |k: &str| g(k).round() as i64;
    let gu = |k: &str| g(k).round().max(1.0) as usize;
    let mut s = Sig::new(n);
    let mid = calc_on(&x.h, &x.l, &x.c, 1, false);
    let pt = |typo: bool| calc_on(&x.h, &x.l, &x.c, gi("point"), typo);
    let flt = |point: i64| tf_trend(x, gi("flt_tf"), point, gu("flt_period"), g("flt_factor"));
    match id {
        ALL | ALL_ATR => {
            let tr = st_daily(x, &mid, gu("st_period"), g("st_factor"));
            let (lo, hi): (Vec<bool>, Vec<bool>) = if id == ALL_ATR {
                let ap = atr_pct(x, gu("atr_period"));
                let avg = ma_partial(&ap, gu("atr_years") * 225);
                (0..n).map(|i| match (ap[i], avg[i]) { (Some(a), Some(m)) => (a < 2.0 * m, a > 2.0 * m), _ => (false, false) }).unzip()
            } else {
                (vec![true; n], vec![false; n])
            };
            for i in 0..n {
                s.buy[i] = tr[i] == 1 && lo[i];
                s.sell[i] = tr[i] == -1 || hi[i];
                s.short[i] = tr[i] == -1 && lo[i];
                s.cover[i] = tr[i] == 1 || hi[i];
            }
        }
        S51 => {
            let cp = pt(false);
            let (t1, t2) = (st_daily(x, &cp, gu("st1_period"), g("st1_factor")), st_daily(x, &cp, gu("st2_period"), g("st2_factor")));
            for i in 0..n {
                s.short[i] = t2[i] == 1 && t1[i] == -1 && i != 0 && !lo_circuit(x, i) && x.c[i] > 0.0;
                s.cover[i] = (t1[i] == 1 || t2[i] == -1) && !up_circuit(x, i);
            }
        }
        L5254 | DIFF_LS | SIMPLE_L | SIMPLE_S => {
            let cp = match id {
                L5254 => pt(true),
                DIFF_LS => pt(false),
                _ => mid.clone(),
            };
            let p1 = gu("st1_period");
            let (t1, t2) = (st_daily(x, &cp, p1, g("st1_factor")), st_daily(x, &cp, gu("st2_period"), g("st2_factor")));
            for i in 0..n {
                let both_dn = t1[i] == -1 && t2[i] == -1;
                match id {
                    L5254 => {
                        s.buy[i] = t1[i] == 1 && i != 0 && !up_circuit(x, i);
                        s.sell[i] = both_dn && !lo_circuit(x, i);
                    }
                    DIFF_LS => {
                        s.buy[i] = t1[i] == 1 && i > p1;
                        s.sell[i] = both_dn;
                        s.short[i] = both_dn;
                        s.cover[i] = t1[i] == 1;
                    }
                    SIMPLE_L => {
                        s.buy[i] = t1[i] == 1 && i > p1 && i != 0 && !up_circuit(x, i);
                        s.sell[i] = (both_dn || i == 0) && !lo_circuit(x, i);
                    }
                    _ => {
                        s.short[i] = both_dn && i > p1 && i != 0 && !lo_circuit(x, i);
                        s.cover[i] = (t1[i] == 1 || i == 0) && !lo_circuit(x, i);
                    }
                }
            }
        }
        BL | BR => {
            let point = gi("point");
            let cp = pt(false);
            let m = flt(point);
            let (t1, t2) = (st_daily(x, &cp, gu("st1_period"), g("st1_factor")), st_daily(x, &cp, gu("st2_period"), g("st2_factor")));
            let flt_exit = gi("flt_exit") == 1;
            for i in 0..n {
                let both_dn = t1[i] == -1 && t2[i] == -1;
                if id == BL {
                    s.buy[i] = m[i] == 1 && t1[i] == 1;
                    s.sell[i] = both_dn || (flt_exit && m[i] == -1);
                    s.short[i] = m[i] == 1 && both_dn;
                    s.cover[i] = t1[i] == 1 || m[i] == -1;
                } else {
                    s.buy[i] = m[i] == -1 && t1[i] == 1;
                    s.sell[i] = m[i] == 1 || both_dn;
                    s.short[i] = m[i] == -1 && both_dn;
                    s.cover[i] = t1[i] == 1;
                }
            }
        }
        BULL_ATR | BEAR | BEAR_TIME => {
            let cp = pt(false);
            let m = flt(gi("flt_point"));
            let p1 = gu("st1_period");
            let (f1, f2) = (g("st1_factor"), g("st2_factor"));
            let t1 = st_daily(x, &cp, p1, f1);
            let t2 = if id == BEAR_TIME { t1.clone() } else { st_daily(x, &cp, gu("st2_period"), f2) };
            let exit_dn = |i: usize| if f1 > f2 { t2[i] == -1 } else { t1[i] == -1 && t2[i] == -1 };
            match id {
                BULL_ATR => {
                    let ap = atr_pct(x, 14);
                    let avg = ma_partial(&ap, 3 * 225);
                    let mut buyday: Option<f64> = None;
                    for i in 0..n {
                        let ref_avg = if i > 3 * 225 + 12 { avg[i] } else { Some(100.0) };
                        if m[i] == 1 && t1[i] == 1 {
                            buyday = ap[i];
                        }
                        let lo = matches!((ap[i], ref_avg), (Some(a), Some(r)) if a < 2.0 * r);
                        let hi = matches!((ap[i], buyday), (Some(a), Some(b)) if a > 2.0 * b);
                        s.buy[i] = m[i] == 1 && t1[i] == 1 && i > p1 && i != 0 && !up_circuit(x, i) && lo;
                        s.sell[i] = (exit_dn(i) || m[i] == -1 || i == 0 || hi) && !lo_circuit(x, i);
                    }
                }
                BEAR => {
                    for i in 0..n {
                        s.buy[i] = m[i] == -1 && t1[i] == 1 && i > p1 && i != 0 && !up_circuit(x, i);
                        s.sell[i] = (exit_dn(i) || m[i] == 1 || i == 0) && !lo_circuit(x, i);
                        s.short[i] = m[i] == -1 && t1[i] == -1 && t2[i] == -1 && i != 0 && !up_circuit(x, i);
                        s.cover[i] = t1[i] == 1 || m[i] == 1 || i == 0;
                    }
                }
                _ => {
                    let hold = g("hold_bars").round().max(0.0) as i64;
                    let shortsig: Vec<bool> = (0..n).map(|i| m[i] == -1 && t1[i] == -1 && i != 0 && !up_circuit(x, i)).collect();
                    let (mut count, mut bcount) = (vec![0i64; n], vec![0i64; n]);
                    let mut flag = vec![false; n];
                    for i in 1..n {
                        flag[i] = shortsig[i] && count[i - 1] == 0;
                        if shortsig[i] {
                            count[i] = 1;
                            bcount[i] = bcount[i - 1] + 1;
                        } else if t1[i] == 1 {
                            count[i] = 0;
                            bcount[i] = 0;
                        } else if count[i - 1] != 0 {
                            bcount[i] = bcount[i - 1] + 1;
                            count[i] = count[i - 1];
                        } else {
                            count[i] = count[i - 1];
                        }
                        if bcount[i - 1] == hold {
                            count[i] = 0;
                            bcount[i] = 0;
                        }
                    }
                    let mut last: Option<usize> = None;
                    for i in 0..n {
                        if flag[i] {
                            last = Some(i);
                        }
                        let held = last.map_or(false, |f| (i - f) as i64 >= hold);
                        s.buy[i] = m[i] == -1 && t1[i] == 1 && i > p1 && i != 0 && !up_circuit(x, i);
                        s.sell[i] = (t1[i] == -1 && held) || (m[i] == 1 && held) || i == 0;
                        s.short[i] = shortsig[i];
                        s.cover[i] = (t1[i] == 1 && held) || i == 0;
                    }
                }
            }
        }
        REGIM2 => {
            let point = gi("point");
            let cp = pt(false);
            let m = flt(point);
            let st = |k: &str| st_daily(x, &cp, gu(&format!("{k}_period")), g(&format!("{k}_factor")));
            let (b1, b2, s1, s2) = (st("b1"), st("b2"), st("s1"), st("s2"));
            let ver = gi("version");
            for i in 0..n {
                let (bdn, sdn, sup) = (b1[i] == -1 && b2[i] == -1, s1[i] == -1 && s2[i] == -1, s1[i] == 1 && s2[i] == 1);
                let (mup, mdn) = (m[i] == 1, m[i] == -1);
                match ver {
                    1 => {
                        s.buy[i] = b1[i] == 1;
                        s.sell[i] = bdn;
                        s.short[i] = mdn && sdn;
                        s.cover[i] = sup || b1[i] == 1;
                    }
                    2 => {
                        s.buy[i] = b1[i] == 1;
                        s.sell[i] = bdn || (mdn && s2[i] == -1);
                        s.short[i] = mdn && sdn;
                        s.cover[i] = sup;
                    }
                    3 => {
                        s.buy[i] = b1[i] == 1;
                        s.sell[i] = bdn;
                        s.short[i] = mdn && s1[i] == -1;
                        s.cover[i] = sup;
                    }
                    _ => {
                        s.buy[i] = if mup { b1[i] == 1 } else { s2[i] == 1 };
                        s.sell[i] = if mup { bdn } else { sdn };
                        s.short[i] = mdn && sdn;
                        s.cover[i] = s2[i] == 1;
                    }
                }
            }
        }
        REGIM_2ST => {
            let point = gi("point");
            let cp = pt(false);
            let m = flt(point);
            let (_, lu1, ld1) = st_lines(&cp, &x.h, &x.l, &x.c, gu("st1_period"), g("st1_factor"));
            let (_, lu2, ld2) = st_lines(&cp, &x.h, &x.l, &x.c, gu("st2_period"), g("st2_factor"));
            let t3 = st_daily(x, &cp, gu("st3_period"), g("st3_factor"));
            let (up1, up2) = (cross(&x.c, &shift1(&ld1), true), cross(&x.c, &shift1(&ld2), true));
            let (dn1, dn2) = (cross(&x.c, &shift1(&lu1), false), cross(&x.c, &shift1(&lu2), false));
            for i in 0..n {
                let (both_up, both_dn) = (up1[i] && up2[i], dn1[i] && dn2[i]);
                s.buy[i] = m[i] == 1 && both_up;
                s.sell[i] = t3[i] == -1 || both_dn;
                s.short[i] = m[i] == -1 && both_dn;
                s.cover[i] = t3[i] == 1 || both_up;
            }
        }
        _ => {}
    }
    s
}

// ---------------------------------------------------------------- the two monthly-ST-sell loop AFLs

/// "Nifty SuperTrend Entry & ATR SL Exit" (pb = false) and "... Profit Booking or HARD SL or Trail SL Exit" (pb = true).
fn st_sell_loop(x: &Ctx, g: &dyn Fn(&str) -> f64, pb: bool) -> Vec<Ev> {
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let n = c.len();
    let gi = |k: &str| g(k).round() as i64;
    let gu = |k: &str| g(k).round().max(1.0) as usize;
    let imt = tf_trend(x, gi("flt_tf"), gi("flt_point"), gu("flt_period"), g("flt_factor"));
    let m54: Option<Vec<i8>> = if !pb && gi("flt54") == 1 { Some(tf_trend(x, gi("flt54_tf"), gi("flt54_point"), gu("flt54_period"), g("flt54_factor"))) } else { None };
    let atr: Vec<f64> = atr_wilder(x, gu("atr_period")).into_iter().map(|v| v.unwrap_or(f64::NAN)).collect();
    let (ptm, slm, tslm) = (g("pt_mult"), g("sl_mult"), g("tsl_mult"));
    let ver = gi("version");
    let buf = if pb { g("buffer") } else { 0.0 };
    // frozen data: the next 30 bars repeat this bar's OHLC (all 30 must exist)
    let frozen: Vec<bool> = (0..n)
        .map(|i| pb && i + 30 < n && (1..=30).all(|k| o[i + k] == o[i] && h[i + k] == h[i] && l[i + k] == l[i] && c[i + k] == c[i]))
        .collect();
    // Null (ATR not yet available) makes the level Null: the loop compares it as -1e10
    let lvl = |v: f64| if v.is_nan() { NULL } else { v };
    let mut evs: Vec<Ev> = Vec::new();
    let (mut pos, mut skip, mut re_buy) = (false, false, 0i64);
    let (mut hc, mut tsl, mut fsl, mut ptl) = (0.0f64, 0.0f64, 0.0f64, BIG);
    for i in 1..n {
        if pb {
            if imt[i] == 1 && re_buy == 0 {
                re_buy = 1;
            } else if imt[i] == -1 && re_buy == 1 {
                re_buy = 2;
            }
        } else if imt[i] == -1 && re_buy == 0 {
            re_buy = 1;
        } else if imt[i] == -1 && imt[i - 1] == 1 && re_buy == 2 {
            re_buy = 1;
        }
        let trend_ok = match ver {
            2 => imt[i - 1] == -1 && imt[i] == -1,
            3 => imt[i - 1] == -1,
            _ => imt[i] == -1,
        };
        let filter_ok = m54.as_ref().map_or(true, |m| m[i] == 1);
        let cash = if pb { h[i] != c[i] && c[i] > 0.0 && !frozen[i] } else { c[i] > 0.0 };
        if !pos && trend_ok && filter_ok && re_buy == if pb { 2 } else { 1 } && cash {
            if ptm > 0.0 {
                ptl = lvl(c[i] + atr[i] * ptm);
            }
            if slm > 0.0 {
                fsl = lvl(c[i] - atr[i] * slm);
            }
            pos = true;
            evs.push(Ev { dir: 1.0, e: i, epx: c[i], exit: None });
        }
        if pos {
            if tslm > 0.0 {
                hc = c[i].max(hc);
            }
            let hit = c[i] <= tsl || c[i] >= ptl || c[i] <= fsl;
            let mut sold: Option<(f64, &'static str)> = None;
            if !pb {
                let fexit = m54.as_ref().map_or(false, |m| m[i] == -1);
                if hit || fexit {
                    let why = if c[i] >= ptl {
                        re_buy = 1;
                        "ATR profit target"
                    } else if c[i] <= fsl {
                        re_buy = 2;
                        "ATR hard SL"
                    } else if c[i] <= tsl {
                        "ATR trailing SL"
                    } else {
                        "ST(54) filter down"
                    };
                    // a profit target and a hard SL cannot both hold; the AFL sets Re_Buy in that order
                    if c[i] <= fsl && c[i] >= ptl {
                        re_buy = 2;
                    }
                    sold = Some((c[i], why));
                }
            } else if hit && !skip {
                if l[i] != c[i] || frozen[i] {
                    sold = Some((c[i], if c[i] >= ptl { "ATR profit booking" } else if c[i] <= fsl { "ATR hard SL" } else { "ATR trailing SL" }));
                } else {
                    skip = true;
                }
            } else if (!lo_circuit(x, i) && skip) || frozen[i] {
                sold = Some((o[i] - o[i] * buf / 100.0, "Blocked exit, open less buffer"));
            }
            if let Some((px, why)) = sold {
                if let Some(v) = evs.last_mut() {
                    v.exit = Some((i, bound(x, i, px), i, why));
                }
                pos = false;
                skip = false;
                hc = 0.0;
                tsl = 0.0;
                fsl = 0.0;
                ptl = BIG;
                if pb {
                    re_buy = 0;
                }
            }
            if tslm > 0.0 {
                tsl = lvl(hc - atr[i] * tslm);
            }
        }
    }
    evs
}

// ---------------------------------------------------------------- the "Probabilty" virtual-short AFLs

/// STBuy / STSell of the AFLs' own ST on the chosen timeframe (1 daily = the chart's own bars, 2 weekly, 3 monthly):
/// on a higher timeframe the daily bar sees the previous completed period, the period's last day its own, and the
/// data's last bar Null.
fn prob_st(x: &Ctx, tf: i64, period: usize, f: f64, avg: i64) -> (Vec<Option<bool>>, Vec<Option<bool>>) {
    let n = x.c.len();
    let st = |o: &[f64], h: &[f64], l: &[f64], c: &[f64]| {
        let (line, _) = sniper_st(o, h, l, c, period, f, avg);
        let buy: Vec<Option<bool>> = (0..c.len()).map(|i| line[i].map(|v| c[i] > v)).collect();
        let sell: Vec<Option<bool>> = (0..c.len()).map(|i| line[i].map(|v| c[i] < v)).collect();
        (buy, sell)
    };
    if tf != 2 && tf != 3 {
        return st(&x.o, &x.h, &x.l, &x.c);
    }
    let b = compress(x, if tf == 3 { 'M' } else { 'W' });
    let mut o = vec![0.0; b.c.len()];
    for i in (0..n).rev() {
        o[b.group[i]] = x.o[i];
    }
    let (buy, sell) = st(&o, &b.h, &b.l, &b.c);
    let see = |i: usize| -> Option<usize> { if i + 1 >= n { None } else { b.group[i + 1].checked_sub(1) } };
    ((0..n).map(|i| see(i).and_then(|g| buy[g])).collect(), (0..n).map(|i| see(i).and_then(|g| sell[g])).collect())
}

struct ProbCfg {
    gain: bool,
    cumulative: bool,
    /// the virtual short opens only without a real position
    vopen_flat: bool,
    /// the filter also gates the virtual close (gain AFLs)
    filter_vclose: bool,
}

fn prob_loop(x: &Ctx, g: &dyn Fn(&str) -> f64, cfg: ProbCfg, from: usize, to: usize) -> Vec<Ev> {
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let n = c.len();
    let gi = |k: &str| g(k).round() as i64;
    let gu = |k: &str| g(k).round().max(1.0) as usize;
    let (b1, s1) = prob_st(x, 1, gu("st_period"), g("st_factor1"), gi("st_avg"));
    let (_, s2) = prob_st(x, 1, gu("st_period"), g("st_factor2"), gi("st_avg"));
    let on = gi("flt") == 1;
    let (fb, fs) = if on { prob_st(x, gi("flt_tf"), gu("flt_period"), g("flt_factor"), gi("flt_avg")) } else { (vec![None; n], vec![None; n]) };
    let bear = gi("flt_bear") == 1;
    let t = |v: Option<bool>| v == Some(true);
    let filter_ok = |i: usize| !on || if bear { t(fs[i]) } else { t(fb[i]) };
    let filter_exit = |i: usize| on && if bear { t(fb[i]) } else { t(fs[i]) };
    // dateCheck: DateNum() >= range from AND DateNum() < range to
    let inr = |i: usize| i >= from && i < to;
    let entry_sig = |i: usize| inr(i) && t(b1[i]);
    let exit_sig = |i: usize| inr(i) && t(s1[i]) && t(s2[i]);
    let (x_pct, y_pct) = (g("x_pct"), g("y_pct"));
    let live = gi("exit_live") == 1;
    let (hsl_pct, pb_pct, tsl_pct, buf) = (g("hsl_pct"), g("pb_pct"), g("tsl_pct"), g("buffer"));
    let mut evs: Vec<Ev> = Vec::new();
    let (mut vpos, mut pos, mut cont) = (false, false, false);
    let (mut entry, mut total, mut bp) = (0.0f64, 0.0f64, 0.0f64);
    let (mut hsl, mut pbl): (Option<f64>, Option<f64>) = (None, None);
    let mut tsl: Vec<Option<f64>> = vec![None; n];
    for i in 1..n {
        tsl[i] = tsl[i - 1];
        if vpos && entry_sig(i) && (!cfg.filter_vclose || filter_ok(i)) {
            let chg = (entry - c[i]) / entry * 100.0;
            cont = if cfg.gain { entry - c[i] > 0.0 && chg >= x_pct } else { chg <= x_pct };
            vpos = false;
        }
        if cont && !pos && entry_sig(i) && i + 1 != n && filter_ok(i) {
            pos = true;
            bp = c[i];
            hsl = if hsl_pct > 0.0 { Some(bp * (1.0 - hsl_pct * 0.01)) } else { None };
            pbl = if pb_pct > 0.0 { Some(bp * (1.0 + pb_pct * 0.01)) } else { None };
            tsl[i] = if tsl_pct > 0.0 { Some(bp * (1.0 - tsl_pct * 0.01)) } else { None };
            evs.push(Ev { dir: 1.0, e: i, epx: bp, exit: None });
        } else if pos {
            let (lo, hi) = if live { (l[i], h[i]) } else { (c[i], c[i]) };
            let hsl_hit = hsl.map_or(false, |v| lo < v);
            let pb_hit = pbl.map_or(false, |v| hi > v);
            let tsl_hit = tsl[i - 1].map_or(false, |v| v != 0.0 && lo < v);
            if hsl_hit || pb_hit || tsl_hit || exit_sig(i) || filter_exit(i) || i + 1 == n {
                let mut px = c[i];
                let why = if hsl_hit { "HSL" } else if pb_hit { "PB" } else if tsl_hit { "TSL" } else if exit_sig(i) { "ST exit" } else if filter_exit(i) { "Filter exit" } else { "Last bar" };
                if live && hsl_hit {
                    let v = hsl.unwrap_or(px);
                    px = if o[i] < v { l[i].max(o[i] * (1.0 - buf * 0.01)) } else { v };
                }
                if live && pb_hit {
                    let v = pbl.unwrap_or(px);
                    px = if o[i] > v { l[i].max(o[i] * (1.0 - buf * 0.01)) } else { v };
                }
                if let Some(v) = evs.last_mut() {
                    v.exit = Some((i, bound(x, i, px), i, why));
                }
                if cfg.cumulative {
                    total += (px - bp) / bp * 100.0;
                    cont = total < y_pct;
                    if cfg.gain && total >= y_pct {
                        total = 0.0;
                    }
                } else {
                    cont = !(px >= bp * (1.0 + y_pct * 0.01));
                }
                pos = false;
            }
            if let Some(v) = tsl[i - 1] {
                if v != 0.0 {
                    tsl[i] = Some((h[i] * (1.0 - tsl_pct * 0.01)).max(v));
                }
            }
        }
        if !vpos && (!cfg.vopen_flat || !pos) && !cont && exit_sig(i) {
            entry = c[i];
            vpos = true;
            if cfg.cumulative && !cfg.gain {
                total = 0.0;
            }
        }
    }
    evs
}

// ---------------------------------------------------------------- the ATHC % swing AFLs

#[derive(Clone, Copy, PartialEq)]
enum Athc {
    Plain,
    Pct,
    Losses,
}

/// The consecutive-loss AFL's loss count: virtual trades (buy on the ATHC flag, sell on a down swing) closed on the
/// bar the swing turns down count with their % profit after brokerage both ways.
fn loss_count(x: &Ctx, tr: &[i8], flag: &[bool], g: &dyn Fn(&str) -> f64) -> Vec<i64> {
    let c = &x.c;
    let n = c.len();
    let (comm, loss_pct) = (g("comm"), g("loss_pct"));
    let mut bcount = vec![0u8; n];
    let (mut bp, mut sp): (Option<f64>, Option<f64>) = (None, None);
    let mut cnt = vec![0i64; n];
    for i in 1..n {
        let buysig = flag[i] && !up_circuit(x, i) && c[i] > 0.0;
        if buysig && bcount[i - 1] < 1 {
            bp = Some(c[i]);
            bcount[i] = 1;
        } else if tr[i] == -1 && bcount[i - 1] == 1 {
            sp = Some(c[i]);
            bcount[i] = 0;
        } else {
            bcount[i] = bcount[i - 1];
        }
        // Cross(Ref(TrendUp1, -1), C) is the bar the swing turns down (TrendUp1 = the up-trend band, Null while
        // down); Ref(TrendUp1, -1) has its first value on bar 2, so the cross can hold from bar 3
        let cross = i >= 3 && tr[i - 1] == 1 && tr[i] == -1;
        cnt[i] = cnt[i - 1];
        if let (true, Some(b), Some(s)) = (cross && bcount[i - 1] == 1, bp, sp) {
            let profit = (s - s * comm / 100.0) - (b + b * comm / 100.0);
            if profit != 0.0 {
                cnt[i] = if profit / b * 100.0 <= loss_pct { cnt[i - 1] + 1 } else { 0 };
            }
        }
    }
    cnt
}

/// "Entry on ATHC ..." (Plain), "... ATHC + 5% ..." (Pct) and "ATHC entry After 2 consecutive losses ..." (Losses).
fn athc_loop(x: &Ctx, g: &dyn Fn(&str) -> f64, kind: Athc) -> Vec<Ev> {
    let (h, c) = (&x.h, &x.c);
    let n = c.len();
    let tr = close_swing(c, g("exit_pct"));
    let flag = athc(c, &tr, false);
    let hc = hhv(c, n.max(1));
    let (pt_pct, athc_pct) = (g("pt_pct"), g("athc_pct"));
    let losses = if kind == Athc::Losses { loss_count(x, &tr, &flag, g) } else { vec![0; n] };
    let need = g("n_loss").round() as i64;
    let (mut pos, mut skip, mut t) = (false, false, false);
    let mut trig: Option<f64> = None;
    let mut pab = 0.0f64;
    let mut evs: Vec<Ev> = Vec::new();
    for i in 1..n {
        let new_high = c[i] > hc[i - 1];
        if kind == Athc::Pct {
            if new_high && !t {
                t = true;
                trig = Some(c[i] + c[i] * athc_pct / 100.0);
            } else if tr[i] == -1 {
                t = false;
                trig = None;
            }
        }
        if skip && tr[i] == -1 {
            skip = false;
        }
        let extra = match kind {
            Athc::Plain => true,
            Athc::Pct => c[i] > loop_val(trig),
            Athc::Losses => losses[i] >= need,
        };
        if !pos && (flag[i] || new_high) && extra && !up_circuit(x, i) && c[i] > 0.0 && !skip {
            pab = c[i];
            pos = true;
            evs.push(Ev { dir: 1.0, e: i, epx: c[i], exit: None });
        } else if pos {
            let pt = (1.0 + pt_pct * 0.01) * pab;
            if (tr[i] == -1 || !flag[i] || h[i] >= pt) && !lo_circuit(x, i) {
                let (px, why) = if h[i] >= pt { (pt, "Profit target") } else if tr[i] == -1 { (c[i], "% swing down") } else { (c[i], "ATHC flag off") };
                if let Some(v) = evs.last_mut() {
                    v.exit = Some((i, bound(x, i, px), i, why));
                }
                pos = false;
                skip = true;
            }
        }
    }
    evs
}

// ---------------------------------------------------------------- the ST 5,2 scale-in AFL

/// Units are SetPositionSize(20, spsPercentOfEquity) each, at most 5: one Trade, one leg (20% of the allocation) per
/// unit; positions are opened in from..=to, later legs dropped, and one still open after `to` is marked there.
fn scale_in(x: &Ctx, g: &dyn Fn(&str) -> f64, from: usize, to: usize) -> Vec<Trade> {
    const MAX_UNITS: usize = 5;
    const UNIT: f64 = 0.2;
    let c = &x.c;
    let n = c.len();
    let gu = |k: &str| g(k).round().max(1.0) as usize;
    let cp = calc_on(&x.h, &x.l, c, g("point").round() as i64, false);
    let t1 = st_daily(x, &cp, gu("st1_period"), g("st1_factor"));
    let t2 = st_daily(x, &cp, gu("st2_period"), g("st2_factor"));
    let atr = atr_wilder(x, gu("atr_period"));
    let m = g("atr_mult");
    let level = |i: usize| atr[i].map(|a| c[i] + a * m);
    let mut up_atr: Vec<Option<f64>> = vec![None; n];
    let mut units = 0usize;
    let mut book: Vec<(Vec<Leg>, Option<usize>)> = Vec::new();
    for i in 1..n {
        let ok = !up_circuit(x, i) && c[i] > 0.0;
        if units == 0 && t1[i] == 1 && ok {
            units = 1;
            up_atr[i] = level(i);
            book.push((vec![Leg { i, px: c[i], w: UNIT }], None));
        } else if units > 0 && units < MAX_UNITS && c[i] > loop_val(up_atr[i - 1]) && ok {
            units += 1;
            up_atr[i] = level(i);
            if let Some(b) = book.last_mut() {
                b.0.push(Leg { i, px: c[i], w: UNIT });
            }
        } else if t2[i] == -1 && t1[i] == -1 && !lo_circuit(x, i) {
            if units > 0 {
                if let Some(b) = book.last_mut() {
                    b.1 = Some(i);
                }
            }
            units = 0;
        } else {
            up_atr[i] = up_atr[i - 1];
        }
    }
    book.into_iter()
        .filter(|(legs, _)| legs[0].i >= from && legs[0].i <= to)
        .map(|(mut legs, exit)| {
            legs.retain(|l| l.i <= to);
            let (exit_i, reason, open) = match exit {
                Some(e) if e <= to => (e, "ST1 and ST2 down", false),
                _ => (to, "open", true),
            };
            Trade { dir: 1.0, legs, exit_i, exit_px: c[exit_i], reason: reason.into(), entry_kind: "close", signal_i: exit_i, open, exits: Vec::new() }
        })
        .collect()
}

// ---------------------------------------------------------------- ST buy above the flip high, sell below the flip low

/// "ST Buy Sell at High Low - Updated.afl": the loop's entry and exit prices (a Null level is -1e10 in the loop).
fn high_low(x: &Ctx, g: &dyn Fn(&str) -> f64) -> (Vec<Option<f64>>, Vec<Exit>) {
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let n = c.len();
    let cp = calc_on(h, l, c, g("point").round() as i64, false);
    let tr = st_daily(x, &cp, g("st_period").round().max(1.0) as usize, g("st_factor"));
    let buf = g("buffer");
    let (mut up_p, mut lo_p): (Vec<Option<f64>>, Vec<Option<f64>>) = (vec![None; n], vec![None; n]);
    let (mut abc1, mut abc2, mut pos) = (false, false, false);
    let mut rt = vec![false; n];
    let mut entry: Vec<Option<f64>> = vec![None; n];
    let mut exit: Vec<Exit> = vec![None; n];
    for i in 1..n {
        if tr[i] != tr[i - 1] && tr[i] > 0 {
            up_p[i] = Some(h[i] + h[i] * buf / 100.0);
            abc1 = true;
        } else if abc1 {
            up_p[i] = up_p[i - 1];
            if tr[i] < 0 {
                abc1 = false;
                up_p[i] = None;
            }
        }
        if tr[i] != tr[i - 1] && tr[i] < 0 {
            lo_p[i] = Some(l[i] - l[i] * buf / 100.0);
            abc2 = true;
        } else if abc2 {
            lo_p[i] = lo_p[i - 1];
            if tr[i] > 0 {
                abc2 = false;
                lo_p[i] = None;
            }
        }
        rt[i] = if h[i] > loop_val(up_p[i]) { true } else if rt[i - 1] && l[i] < loop_val(lo_p[i]) { false } else { rt[i - 1] };
        let (bl, sl) = (loop_val(up_p[i - 1]), loop_val(lo_p[i - 1]));
        if !pos && (h[i] > bl || rt[i]) && !up_circuit(x, i) && c[i] > 0.0 {
            let px = if o[i] > bl { o[i] + o[i] * buf / 100.0 } else { up_p[i - 1].unwrap_or(c[i]) };
            entry[i] = Some(bound(x, i, px));
            pos = true;
        }
        if pos && (l[i] < sl || !rt[i]) && !lo_circuit(x, i) {
            let px = if o[i] < sl { o[i] - o[i] * buf / 100.0 } else { lo_p[i - 1].unwrap_or(c[i]) };
            exit[i] = Some((bound(x, i, px), if l[i] < sl { "Low below the sell level" } else { "Running trade off" }));
            pos = false;
            rt[i] = false;
        }
    }
    (entry, exit)
}

// ---------------------------------------------------------------- ST 5,1 with an ST 5,4 bull / bear filter, ATR exits

/// The bear-filter AFL (short only or long only, `bull` false) and the bull-filter AFL (short only, `bull` true).
fn flt_atr_loop(x: &Ctx, g: &dyn Fn(&str) -> f64, bull: bool) -> Vec<Ev> {
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let n = c.len();
    let gi = |k: &str| g(k).round() as i64;
    let gu = |k: &str| g(k).round().max(1.0) as usize;
    let imt = tf_trend(x, gi("st_tf"), gi("st_point"), gu("st_period"), g("st_factor"));
    let mt = tf_trend(x, gi("flt_tf"), gi("flt_point"), gu("flt_period"), g("flt_factor"));
    let atr: Vec<f64> = atr_wilder(x, gu("atr_period")).into_iter().map(|v| v.unwrap_or(f64::NAN)).collect();
    let lvl = |v: f64| if v.is_nan() { NULL } else { v };
    let (ptm, slm, buf) = (g("pt_mult"), g("sl_mult"), g("buffer"));
    let long = !bull && gi("side") == 2;
    // the side's ST(5/1) state that arms the re-entry, and the filter state of an entry
    let (arm, want) = (if long { -1 } else { 1 }, if bull { 1 } else { -1 });
    let mut evs: Vec<Ev> = Vec::new();
    let (mut pos, mut re_buy) = (false, 0i64);
    let (mut pt, mut sl) = if long { (BIG, 0.0) } else { (0.0, BIG) };
    for i in gu("st_period")..n {
        if imt[i] == arm && re_buy == 0 {
            re_buy = 1;
        } else if imt[i] == arm && imt[i - 1] == -arm && re_buy == 2 {
            re_buy = 1;
        }
        if !pos && imt[i - 1] == arm && mt[i] == want && re_buy == 1 {
            let d = if long { 1.0 } else { -1.0 };
            if ptm > 0.0 {
                pt = lvl(c[i] + d * atr[i] * ptm);
            }
            if slm > 0.0 {
                sl = lvl(c[i] - d * atr[i] * slm);
            }
            pos = true;
            evs.push(Ev { dir: d, e: i, epx: c[i], exit: None });
        } else if pos {
            let flt_exit = mt[i] == -want;
            let out: Option<(f64, &'static str)> = if long {
                if h[i] >= pt {
                    Some((if o[i] > pt { o[i] - o[i] * buf / 100.0 } else { pt }, "ATR profit target"))
                } else if l[i] <= sl {
                    Some((if o[i] < sl { o[i] - o[i] * buf / 100.0 } else { sl }, "ATR hard SL"))
                } else if flt_exit {
                    Some((c[i], "ST(5/4) filter"))
                } else {
                    None
                }
            } else if l[i] <= pt {
                Some((if o[i] < pt { o[i] + o[i] * buf / 100.0 } else { pt }, "ATR profit target"))
            } else if h[i] >= sl {
                // the bull AFL covers a gap above the stop at the Open minus the buffer, the bear AFL plus
                Some((if o[i] > sl { if bull { o[i] - o[i] * buf / 100.0 } else { o[i] + o[i] * buf / 100.0 } } else { sl }, "ATR hard SL"))
            } else if flt_exit {
                Some((c[i], "ST(5/4) filter"))
            } else {
                None
            };
            if let Some((px, why)) = out {
                re_buy = if imt[i] == arm { 2 } else { 1 };
                if let Some(v) = evs.last_mut() {
                    v.exit = Some((i, bound(x, i, px), i, why));
                }
                pos = false;
                (pt, sl) = if long { (BIG, 0.0) } else { (0.0, BIG) };
            }
        }
    }
    evs
}

// ---------------------------------------------------------------- MultiTimeFrame.afl

fn mtf_loop(x: &Ctx, g: &dyn Fn(&str) -> f64) -> Vec<Ev> {
    let c = &x.c;
    let n = c.len();
    let gi = |k: &str| g(k).round() as i64;
    let gu = |k: &str| g(k).round().max(1.0) as usize;
    let (fb, _) = prob_st(x, gi("flt_tf"), gu("flt_period"), g("flt_factor"), gi("flt_avg"));
    let bull_phase: Vec<bool> = fb.iter().map(|v| *v == Some(true)).collect();
    let (p, avg) = (gu("st_period"), gi("st_avg"));
    let (l1, _) = sniper_st(&x.o, &x.h, &x.l, c, p, g("st_factor1"), avg);
    let (l2, _) = sniper_st(&x.o, &x.h, &x.l, c, p, g("st_factor2"), avg);
    let entry_sig: Vec<bool> = if gi("running") == 1 { (0..n).map(|i| l1[i].is_some_and(|v| c[i] > v)).collect() } else { cross_up(c, &l1) };
    let (bear, skip) = (gi("flt_bear") == 1, gi("flt_skip_exit") == 1);
    let mut evs: Vec<Ev> = Vec::new();
    let (mut pos, mut flag) = (false, false);
    for i in 1..n {
        if pos {
            if !flag && c[i] > loop_val(l2[i]) {
                flag = true;
            }
            let below1 = !flag && c[i] < loop_val(l1[i]);
            let below2 = flag && c[i] < loop_val(l2[i]);
            let flt_exit = !skip && if bear { bull_phase[i] } else { !bull_phase[i] };
            if below1 || below2 || flt_exit {
                let why = if below1 { "Close below ST1" } else if below2 { "Close below ST2" } else { "ST filter" };
                if let Some(v) = evs.last_mut() {
                    v.exit = Some((i, c[i], i, why));
                }
                pos = false;
            }
        }
        if !pos && entry_sig[i] && if bear { !bull_phase[i] } else { bull_phase[i] } {
            evs.push(Ev { dir: 1.0, e: i, epx: c[i], exit: None });
            pos = true;
            flag = false;
        }
    }
    evs
}

// ---------------------------------------------------------------- run

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(0.0, |d| d.def));
    let n = x.c.len();
    if n == 0 || from > to || to >= n {
        return Some(Vec::new());
    }
    let cfg = |gain, cumulative, vopen_flat, filter_vclose| ProbCfg { gain, cumulative, vopen_flat, filter_vclose };
    Some(match id {
        ATR_SL | PB => collect(x, st_sell_loop(x, &g, id == PB), from, to),
        GAIN3 => collect(x, prob_loop(x, &g, cfg(true, false, false, true), from, to), from, to),
        GAIN4 => collect(x, prob_loop(x, &g, cfg(true, true, true, true), from, to), from, to),
        LOSS4 => collect(x, prob_loop(x, &g, cfg(false, false, false, false), from, to), from, to),
        LOSS5 => collect(x, prob_loop(x, &g, cfg(false, true, false, false), from, to), from, to),
        ATHC => collect(x, athc_loop(x, &g, Athc::Plain), from, to),
        ATHC_PCT => collect(x, athc_loop(x, &g, Athc::Pct), from, to),
        ATHC_LOSS => collect(x, athc_loop(x, &g, Athc::Losses), from, to),
        SCALE52 => scale_in(x, &g, from, to),
        HL => {
            let (entry, exit) = high_low(x, &g);
            d7_fills(x, 1.0, &entry, &exit, from, to, "stop", false, 0.0)
        }
        BEAR_FLT => collect(x, flt_atr_loop(x, &g, false), from, to),
        BULL_FLT => collect(x, flt_atr_loop(x, &g, true), from, to),
        MTF => collect(x, mtf_loop(x, &g), from, to),
        _ => {
            let s = signals(id, x, &g);
            let side = match id {
                S51 | SIMPLE_S => 2,
                L5254 | SIMPLE_L | BULL_ATR => 1,
                _ => g("side").round() as i64,
            };
            let pb_pct = if id == BEAR { g("pb_pct") } else { 0.0 };
            let nbar = if id == BEAR || id == BEAR_TIME { g("hold_bars").round().max(0.0) as usize } else { 0 };
            backtest(x, &s, side, pb_pct, nbar, from, to)
        }
    })
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let d = catalogue().into_iter().find(|d| d.id == id)?;
    Some((d.id, d.name, "Supertrend"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::books::ytwh::testbars::*;

    fn flat(c: f64) -> (f64, f64, f64, f64) {
        (c, c + 1.0, c - 1.0, c)
    }
    /// Bars with open = (high + low) / 2 away from the close, so no circuit bar.
    fn bar(c: f64) -> (f64, f64, f64, f64) {
        (c + 0.3, c + 1.0, c - 1.0, c)
    }

    #[test]
    fn st_lines_flip_and_lines() {
        let mut v: Vec<(f64, f64, f64, f64)> = (0..10).map(|_| flat(100.0)).collect();
        v.extend((0..5).map(|k| flat(90.0 - k as f64)));
        let r = bars(&v);
        let x = Ctx::new(&r, &r, &[]);
        let mid = calc_on(&x.h, &x.l, &x.c, 1, false);
        let (tr, lu, ld) = st_lines(&mid, &x.h, &x.l, &x.c, 5, 1.0);
        // satr = mean of 5 ATR(1) = 2 from bar 5; Dn = 100 - 2 = 98; bar 10 closes 90 < 98 -> down
        assert_eq!(tr[9], 1);
        assert_eq!(tr[10], -1);
        assert_eq!(lu[8], Some(98.0));
        assert!(ld[10].is_some() && ld[9] == lu[9]);
        // the same thing through the generic trend
        assert_eq!(st_daily(&x, &mid, 5, 1.0), tr);
    }

    #[test]
    fn st_all_long_short_reverses() {
        let mut v: Vec<(f64, f64, f64, f64)> = (0..10).map(|k| bar(100.0 + k as f64)).collect();
        v.extend((0..6).map(|k| bar(95.0 - 2.0 * k as f64)));
        v.extend((0..6).map(|k| bar(100.0 + 3.0 * k as f64)));
        let r = bars(&v);
        let t = go(run, ALL, &r, &[]);
        // trend 1 from bar 0: buy at bar 0; the drop reverses to a short, the rally back to a long
        assert_eq!(t[0].0, 0);
        assert!(t.len() >= 3);
        assert_eq!(t[1].0, t[0].2);
        // long only: no shorts
        let tl = go(run, ALL, &r, &[("side", 1.0)]);
        assert!(tl.len() == 2);
    }

    #[test]
    fn atr_doubled_blocks_entry_and_exits() {
        let ap = ma_partial(&[None, Some(2.0), Some(4.0)], 2);
        assert_eq!(ap, vec![None, Some(2.0), Some(3.0)]);
        // a huge range bar lifts ATR % above twice its average: the long sells there
        let mut v: Vec<(f64, f64, f64, f64)> = (0..30).map(|k| bar(100.0 + 0.2 * k as f64)).collect();
        v.push((106.0, 140.0, 80.0, 106.5));
        v.push(bar(107.0));
        let r = bars(&v);
        let t = go(run, ALL_ATR, &r, &[("side", 1.0)]);
        // bar 0 has no ATR(1) (Null), so the first buy is bar 1
        assert_eq!(t[0].0, 1);
        assert_eq!(t[0].2, 30);
    }

    #[test]
    fn st51_short_needs_st54_up() {
        // rising, then a dip that turns ST 5,1 down but not ST 5,4: short; then the rise back covers
        let mut v: Vec<(f64, f64, f64, f64)> = (0..12).map(|k| bar(100.0 + k as f64)).collect();
        v.extend([bar(106.0), bar(105.5), bar(112.0), bar(114.0)]);
        let r = bars(&v);
        let t = go(run, S51, &r, &[]);
        assert_eq!(t.len(), 1);
        assert_eq!(t[0].0, 12);
        assert_eq!(t[0].1, 106.0);
        assert!(t[0].2 > 12);
        // a deep fall turns ST 5,4 down too: no new short then (filter), and the short covers
        let mut v2: Vec<(f64, f64, f64, f64)> = (0..12).map(|k| bar(100.0 + k as f64)).collect();
        v2.extend([bar(80.0), bar(78.0)]);
        assert!(go(run, S51, &bars(&v2), &[]).is_empty());
    }

    #[test]
    fn two_st_exit_waits_for_both() {
        // ST 5,2 turns down on a dip but ST 5,4 stays up: no sale until the deeper fall
        let mut v: Vec<(f64, f64, f64, f64)> = (0..12).map(|k| bar(100.0 + k as f64)).collect();
        v.extend([bar(106.5), bar(107.0), bar(95.0), bar(94.0)]);
        let r = bars(&v);
        let t = go(run, L5254, &r, &[]);
        assert_eq!(t[0].0, 1); // bar 0 is excluded (deadstockbuysig), ExRem keeps bar 1
        assert_eq!(t[0].2, 14);
        // simple long: buys only after bar 5
        let t2 = go(run, SIMPLE_L, &r, &[("st2_period", 5.0), ("st2_factor", 4.0)]);
        assert_eq!(t2[0].0, 6);
        assert_eq!(t2[0].2, 14);
    }

    #[test]
    fn regime_filters_by_monthly_st() {
        // 6 months of rises then a fall: the monthly ST is up; the bear regime takes no long
        let mut v: Vec<(f64, f64, f64, f64)> = (0..140).map(|k| bar(100.0 + 0.5 * k as f64)).collect();
        v.extend((0..10).map(|k| bar(160.0 - 3.0 * k as f64)));
        let r = bars(&v);
        let bull = go(run, BL, &r, &[("side", 1.0)]);
        assert!(!bull.is_empty());
        let bear = go(run, BR, &r, &[("side", 1.0)]);
        assert!(bear.is_empty());
        // bull regime shorts the daily fall while the monthly ST is still up
        let bs = go(run, BL, &r, &[("side", 2.0)]);
        assert!(bs.iter().any(|t| t.0 >= 140));
    }

    #[test]
    fn profit_and_nbar_stops() {
        let mut s = Sig::new(6);
        s.buy[1] = true;
        let v = vec![bar(100.0), bar(100.0), bar(101.0), (104.0, 106.0, 103.0, 105.0), bar(105.0), bar(105.0)];
        let r = bars(&v);
        let x = Ctx::new(&r, &r, &[]);
        let t = backtest(&x, &s, 1, 5.0, 0, 0, 5);
        assert_eq!((t[0].legs[0].i, t[0].exit_i), (1, 3));
        assert!((t[0].exit_px - 105.0).abs() < 1e-9);
        let t2 = backtest(&x, &s, 1, 0.0, 2, 0, 5);
        assert_eq!(t2[0].exit_i, 3);
        assert_eq!(t2[0].exit_px, 105.0);
    }

    #[test]
    fn two_st_cross_needs_both_lines_same_bar() {
        let up = cross(&[1.0, 1.5, 5.0], &[Some(2.0), Some(2.0), None], true);
        assert_eq!(up, vec![false, false, true]); // the Null line counts as -1e10 once the line has started
        let dn = cross(&[3.0, 1.0], &[Some(2.0), Some(2.0)], false);
        assert_eq!(dn, vec![false, true]);
    }

    /// Two years of daily bars: a rise, a long fall (monthly ST sell), then a rally.
    fn cycle() -> Vec<(f64, f64, f64, f64)> {
        let mut v: Vec<(f64, f64, f64, f64)> = (0..200).map(|k| bar(100.0 + 0.3 * k as f64)).collect();
        v.extend((0..150).map(|k| bar(160.0 - 0.4 * k as f64)));
        v.extend((0..150).map(|k| bar(100.0 + 0.5 * k as f64)));
        v
    }

    #[test]
    fn atr_sl_buys_in_monthly_sell_and_books_profit() {
        let r = bars(&cycle());
        let x = Ctx::new(&r, &r, &[]);
        let imt = tf_trend(&x, 1, 1, 5, 1.0);
        let t = go(run, ATR_SL, &r, &[]);
        assert!(!t.is_empty());
        for tr in &t {
            assert_eq!(imt[tr.0], -1, "buys only while the monthly ST is in sell");
        }
        // with the ST(54) filter (monthly ST 5,4 must be up) entries are a subset
        let f = go(run, ATR_SL, &r, &[("flt54", 1.0)]);
        assert!(f.len() <= t.len());
    }

    #[test]
    fn pb_loop_needs_up_then_down_cycle() {
        let r = bars(&cycle());
        let x = Ctx::new(&r, &r, &[]);
        let imt = tf_trend(&x, 1, 1, 20, 1.0);
        let t = go(run, PB, &r, &[]);
        for tr in &t {
            assert_eq!(imt[tr.0], -1);
            // the first bar of an up phase must precede it
            assert!(imt[..tr.0].iter().any(|&v| v == 1));
        }
    }

    #[test]
    fn virtual_short_gain_then_long() {
        // up, down 10% (virtual short from the ST sell), up again: the virtual short gained > 5% -> buy
        let mut v: Vec<(f64, f64, f64, f64)> = (0..15).map(|k| (100.0 + k as f64, 101.5 + k as f64, 99.0 + k as f64, 100.5 + k as f64)).collect();
        v.extend((0..10).map(|k| (113.0 - 2.0 * k as f64, 114.0 - 2.0 * k as f64, 111.0 - 2.0 * k as f64, 112.0 - 2.0 * k as f64)));
        v.extend((0..10).map(|k| (95.0 + 2.0 * k as f64, 97.0 + 2.0 * k as f64, 94.5 + 2.0 * k as f64, 96.5 + 2.0 * k as f64)));
        let r = bars(&v);
        let t = go(run, GAIN4, &r, &[]);
        assert_eq!(t.len(), 1);
        assert!(t[0].0 > 25);
        // the loss version waits for a short that LOST 5%: none here
        assert!(go(run, LOSS5, &r, &[]).is_empty());
        // and with x = 50% the gain is not enough
        assert!(go(run, GAIN3, &r, &[("x_pct", 50.0)]).is_empty());
    }

    fn near(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    #[test]
    fn athc_buys_new_high_and_books_profit_target() {
        // closes 100, 101, ... : bar 1 is a new high; target 101 × 1.05 = 106.05 reached by bar 6's High (107)
        let v: Vec<(f64, f64, f64, f64)> = (0..20).map(|k| bar(100.0 + k as f64)).collect();
        let t = go(run, ATHC, &bars(&v), &[]);
        assert_eq!(t.len(), 1, "no re-entry until the swing has been down");
        assert_eq!((t[0].0, t[0].1, t[0].2), (1, 101.0, 6));
        assert!(near(t[0].3, 106.05));
        // + 5%: the trigger 101 × 1.05 = 106.05 is set on bar 1; bar 7's Close 107 is above it; target 107 × 1.05
        let t = go(run, ATHC_PCT, &bars(&v), &[]);
        assert_eq!((t[0].0, t[0].2), (7, 12));
        assert!(near(t[0].3, 112.35));
    }

    #[test]
    fn athc_after_two_losing_virtual_trades() {
        // two virtual trades bought on a new high (bars 5, 8) and closed on the swing turning down (bars 6, 9) at a
        // loss; the next new high (bar 11) is the real buy, sold at the 5% target on bar 12
        let c = [100.0, 100.0, 100.0, 100.0, 100.0, 101.0, 85.0, 88.0, 102.0, 85.0, 88.0, 104.0, 110.0, 111.0];
        let r = bars(&c.iter().map(|&v| bar(v)).collect::<Vec<_>>());
        let t = go(run, ATHC_LOSS, &r, &[]);
        assert_eq!(t.len(), 1);
        assert_eq!((t[0].0, t[0].1, t[0].2), (11, 104.0, 12));
        assert!(near(t[0].3, 109.2));
        assert!(go(run, ATHC_LOSS, &r, &[("n_loss", 3.0)]).is_empty());
        // the plain ATHC AFL buys the first new high
        assert_eq!(go(run, ATHC, &r, &[])[0].0, 5);
    }

    #[test]
    fn st52_scales_in_up_to_five_units() {
        let mut v: Vec<(f64, f64, f64, f64)> = (0..20).map(|k| bar(140.0 - 2.0 * k as f64)).collect();
        v.extend((0..25).map(|k| bar(102.0 + 2.0 * k as f64)));
        v.extend((0..6).map(|k| bar(150.0 - 8.0 * k as f64)));
        let r = bars(&v);
        let x = Ctx::new(&r, &r, &[]);
        let t = run(SCALE52, &x, 0, r.len() - 1, &Params::new()).unwrap();
        // the ST starts up (bands Null): buy on bar 1; ATR(14) is still Null (-1e10 in the loop): adds on bars 2-5
        assert_eq!(t[0].legs.iter().map(|l| l.i).collect::<Vec<_>>(), vec![1, 2, 3, 4, 5]);
        assert!(t[0].legs.iter().all(|l| near(l.w, 0.2)));
        assert!(!t[0].open && t[0].exit_i < 20);
        // in the rally each add needs a Close above the previous unit's Close + ATR(14)
        let atr = atr_wilder(&x, 14);
        let t2 = t.iter().find(|t| t.legs[0].i >= 20).expect("a rally trade");
        assert!(t2.legs.len() > 1 && t2.legs.len() <= 5);
        for w in t2.legs.windows(2) {
            assert!(x.c[w[1].i] > x.c[w[0].i] + atr[w[0].i].unwrap());
        }
        assert!(t2.exit_i >= 45);
    }

    #[test]
    fn high_low_breakout_buys_on_null_level_and_sells_below_flip_low() {
        let mut v: Vec<(f64, f64, f64, f64)> = (0..12).map(|k| bar(100.0 + k as f64)).collect();
        v.extend((0..8).map(|k| bar(106.0 - 3.0 * k as f64)));
        let r = bars(&v);
        let x = Ctx::new(&r, &r, &[]);
        let tr = st_daily(&x, &calc_on(&x.h, &x.l, &x.c, 1, false), 5, 1.0);
        let d = (1..tr.len()).find(|&i| tr[i] == -1).unwrap();
        let t = go(run, HL, &r, &[]);
        // bar 1: no buy level yet (Null = -1e10): buy at the Open + 0.25%
        assert_eq!(t[0].0, 1);
        assert!(near(t[0].1, x.o[1] * 1.0025));
        // the bar after the flip down opens below the flip bar's Low − 0.25%: sold at that Open − 0.25%
        assert_eq!(t[0].2, d + 1);
        assert!(x.o[d + 1] < x.l[d] * 0.9975);
        assert!(near(t[0].3, x.o[d + 1] * 0.9975));
        // flat in the down trend: bought again at the next Open; that bar's sale is ignored (no same-bar exit)
        assert_eq!((t[1].0, t[1].2), (d + 2, d + 3));
    }

    /// A long fall, a 5-point bounce that turns ST 5,1 up (ST 5,4 stays down), then the fall resumes.
    fn bounce() -> Vec<(f64, f64, f64, f64)> {
        let mut v: Vec<(f64, f64, f64, f64)> = (0..30).map(|k| bar(160.0 - 2.0 * k as f64)).collect();
        v.push(bar(107.0));
        v.extend((0..15).map(|k| bar(105.0 - 2.0 * k as f64)));
        v
    }

    #[test]
    fn bear_filter_shorts_after_st51_up_and_books_atr_target() {
        let r = bars(&bounce());
        let x = Ctx::new(&r, &r, &[]);
        let imt = tf_trend(&x, 3, 1, 5, 1.0);
        let mt = tf_trend(&x, 3, 1, 5, 4.0);
        assert_eq!((imt[30], mt[30], mt[31]), (1, -1, -1));
        let t = run(BEAR_FLT, &x, 0, r.len() - 1, &Params::new()).unwrap();
        let s = t.iter().find(|t| t.legs[0].i == 31).expect("short on the bar after the bounce");
        assert_eq!((s.dir, s.legs[0].px), (-1.0, 105.0));
        assert_eq!(s.reason, "ATR profit target");
        let atr = atr_wilder(&x, 5)[31].unwrap();
        assert!(near(s.exit_px, 105.0 - 3.0 * atr) || s.exit_px < 105.0 - 3.0 * atr);
        for t in &t {
            assert!(t.dir < 0.0 && imt[t.legs[0].i - 1] == 1 && mt[t.legs[0].i] == -1);
        }
        // long only: buys while both STs are down, stopped at entry − 1 × ATR(5) in the fall
        let tl = run(BEAR_FLT, &x, 0, r.len() - 1, &ps(&[("side", 2.0)])).unwrap();
        assert!(!tl.is_empty());
        assert_eq!(tl[0].reason, "ATR hard SL");
        for t in &tl {
            assert!(t.dir > 0.0 && imt[t.legs[0].i - 1] == -1 && mt[t.legs[0].i] == -1);
        }
    }

    #[test]
    fn bull_filter_shorts_a_rise_and_is_stopped() {
        // rising: both STs up from the start; short on bar 5 (the loop's first bar), stopped out the next bar
        let v: Vec<(f64, f64, f64, f64)> = (0..30).map(|k| bar(100.0 + 2.0 * k as f64)).collect();
        let r = bars(&v);
        let x = Ctx::new(&r, &r, &[]);
        let t = run(BULL_FLT, &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert_eq!(t.len(), 1, "after a stop with ST 5,1 up it waits for a new down-up turn");
        assert_eq!((t[0].dir, t[0].legs[0].i, t[0].exit_i), (-1.0, 5, 6));
        assert_eq!(t[0].reason, "ATR hard SL");
        // the stop is entry + 1 × ATR(5) = 110 + 3; bar 6 (Open 112.3, High 113) reaches it: covered at the level
        let sl = 110.0 + atr_wilder(&x, 5)[5].unwrap();
        assert!(near(sl, 113.0) && x.o[6] < sl);
        assert!(near(t[0].exit_px, sl));
    }

    #[test]
    fn mtf_enters_on_daily_cross_in_weekly_bull_phase() {
        // a long rise (weekly ST up), a three-bar dip that turns the daily ST 5,1 down, then the rise resumes: the
        // Close crosses back above ST 5,1 while the weekly filter is still bullish
        let mut v: Vec<(f64, f64, f64, f64)> = (0..80).map(|k| bar(100.0 + k as f64)).collect();
        v.extend([bar(176.0), bar(173.0), bar(170.0)]);
        v.extend((1..20).map(|k| bar(170.0 + 3.0 * k as f64)));
        let r = bars(&v);
        let x = Ctx::new(&r, &r, &[]);
        let (fb, _) = prob_st(&x, 2, 5, 1.0, 5);
        let (l1, _) = sniper_st(&x.o, &x.h, &x.l, &x.c, 5, 1.0, 5);
        let t = run(MTF, &x, 0, r.len() - 1, &Params::new()).unwrap();
        for t in &t {
            let e = t.legs[0].i;
            assert_eq!(fb[e], Some(true));
            assert!(x.c[e] > l1[e].unwrap() && x.c[e - 1] <= l1[e - 1].unwrap_or(-1e10));
        }
        // one trade: bought on bar 84 (the cross back above ST 5,1); on the data's last bar the weekly filter is Null
        // (Ref(+1) past the end), so the bull phase ends there and the filter exit sells
        assert_eq!(t.iter().map(|t| (t.legs[0].i, t.exit_i, t.reason.as_str())).collect::<Vec<_>>(), vec![(84, 101, "ST filter")]);
        // a bearish filter type takes its entries outside the bull phase
        let w = wave(400);
        let xw = Ctx::new(&w, &w, &[]);
        let (fw, _) = prob_st(&xw, 2, 5, 1.0, 5);
        let tb = run(MTF, &xw, 0, w.len() - 1, &ps(&[("flt_bear", 1.0)])).unwrap();
        assert!(!tb.is_empty());
        for t in tb {
            assert_ne!(fw[t.legs[0].i], Some(true));
        }
    }

    #[test]
    fn every_strategy_runs() {
        smoke(run, &IDS);
        for id in IDS {
            assert!(family_of(id).is_some(), "{id}");
            assert!(catalogue().iter().any(|d| d.id == id));
        }
    }
}
