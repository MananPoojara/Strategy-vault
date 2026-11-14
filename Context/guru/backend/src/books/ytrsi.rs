//! YT Strategies · RSI: the team's AmiBroker AFLs in "Old Topics/RSI", each run on one instrument's daily bars (the
//! team ran them on spot Nifty / Bank Nifty; the "Stocks" sub-folder copies of the Lowest-RSI AFL and the Connors F&O
//! AFL were also run on single stocks / futures, one symbol at a time, which is what is coded). Coded from the AFLs
//! as written, with the AFLs' own default Param values; there is no book PDF and nothing here is compared with a
//! trade sheet. The separate "Tripple RSI" and "Stochastic+RSI System" folders are not part of this module.
//!
//! Coded (one id per distinct rule set; AFLs that differ only in Param ranges, Optimize() vs Param() or an option
//! share an id, the option being a parameter):
//! - `ytrsi_cross_30_40`: RSI 30 and 40 Crossover Entry and Exit;
//! - `ytrsi_trail`, `ytrsi_trail_st_v1/v2/v3`: the RSI trailing entry / exit family ("RSI Trail SL", "RSI_Trail based
//!   entry and Exit", "RSI Afls"), without filter and with the monthly Supertrend filter in versions 1-3;
//! - `ytrsi_lowest_st`, `..._swing`, `..._athc`: Lowest RSI Trailing based Entry and ST Fresh Sell Exit, plain / with
//!   the % swing filter / with the ATHC filter (hard SL, ST timeframe and ST-entry-filter are parameters);
//! - `ytrsi_breach_st`: RSI breach Buy and Exit on Fresh Sell in ST; `ytrsi_dip_st`: ST buy-sell when RSI gives a dip
//!   below 30; the four Aakash RSI AFLs; `ytrsi_connors_fo`: Connor RSI EntryExit for FO;
//! - `ytrsi_st_ma_sl`, `ytrsi_hl_lh`, `ytrsi_obos_delayed`: the three folder-root "%SL & ATR SL" / exit-version AFLs.
//!
//! AmiBroker behaviour followed:
//! - every AFL here except "ST LONG ONLY WITH RSI AND FLT" calls `SetBarsRequired(100000, 0)`, so its loops run over
//!   the whole history; the dip AFL has no such call and its loops start the longest look-back before the run start
//!   (QuickAFL, as in `ytich.rs`);
//! - RSI is Wilder's (`connors::rsi`); `IIf(BarIndex() < n, k, RSI(n))` fills the first n bars with k as written;
//! - a Null indicator value (RSI / MA warm-up) is never true in an entry comparison; state variables the AFLs set to
//!   Null read as -1e10 in the loops (the Connors AFL's unset target / stop); HHV / LLV use the bars available so far,
//!   and `HHV(C, BarCount)` is the running all-time-high close;
//! - Supertrend filters are the AFLs' ST loop (`ytsw::supertrend`) on the filter timeframe, expanded with expandLast;
//!   an "HOURLY" timeframe on daily bars leaves the bars daily (TimeFrameSet cannot go below the base interval);
//! - the backtester exits first and then enters on the same bar, trades at the close unless the AFL sets
//!   BuyPrice / SellPrice etc., keeps prices inside the bar's range, and reverses on an opposite entry; a Sell /
//!   Cover written while the loop's position is only virtual (the "fresh trade" versions) is ignored; no ExRem (none
//!   of the AFLs uses it);
//! - template settings left at their defaults: dead-stock (NO), penny price 0 and turnover filter (NO); UC = high
//!   equals close, LC = low equals close, `Prec(x, 2)` truncates to two decimals.
//!
//! Not coded: "Fast Slow RSI" (both AFLs are hourly-RSI crossovers run on hourly bars); "RSI - Index" (both run on
//! F&O intraday bars with a daily filter); the two "New_RSI_Threshold_Long_TSL_on_RSI" AFLs (they
//! `#include_once <parmar/UtilityParam.afl>`, a file not in the folder); the three AFLs in TRASH sub-folders
//! (discarded drafts of the trailing family).
#![allow(dead_code)]
use crate::books::ytsw::{close_swing, compress, expand_last, sma, supertrend, trade};
use crate::books::ytwh::{bound, hhv, llv, prec2};
use crate::connors::{atr, rsi, Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "yt_rsi";
pub const CROSS: &str = "ytrsi_cross_30_40";
pub const TRAIL: &str = "ytrsi_trail";
pub const TRAIL_V1: &str = "ytrsi_trail_st_v1";
pub const TRAIL_V2: &str = "ytrsi_trail_st_v2";
pub const TRAIL_V3: &str = "ytrsi_trail_st_v3";
pub const LOWEST: &str = "ytrsi_lowest_st";
pub const LOWEST_SW: &str = "ytrsi_lowest_st_swing";
pub const LOWEST_ATHC: &str = "ytrsi_lowest_st_athc";
pub const BREACH: &str = "ytrsi_breach_st";
pub const DIP: &str = "ytrsi_dip_st";
pub const AK_BULL: &str = "ytrsi_aakash_bull_same";
pub const AK_BEAR: &str = "ytrsi_aakash_bear_same";
pub const AK_BULL_N: &str = "ytrsi_aakash_bull_next";
pub const AK_BEAR_N: &str = "ytrsi_aakash_bear_next";
pub const CONNORS: &str = "ytrsi_connors_fo";
pub const SLT: &str = "ytrsi_st_ma_sl";
pub const HLLH: &str = "ytrsi_hl_lh";
pub const OBOS: &str = "ytrsi_obos_delayed";
pub const IDS: [&str; 18] = [CROSS, TRAIL, TRAIL_V1, TRAIL_V2, TRAIL_V3, LOWEST, LOWEST_SW, LOWEST_ATHC, BREACH, DIP, AK_BULL, AK_BEAR, AK_BULL_N, AK_BEAR_N, CONNORS, SLT, HLLH, OBOS];

const GROUP: &str = "YT Strategies · RSI";

fn def(id: &'static str, name: &'static str, dir: &'static str, rules: &'static [&'static str], notes: &'static str) -> StrategyDef {
    StrategyDef { id, name, group: GROUP, session: 0, dir, status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim: None }
}

const TRAIL_RULES: [&str; 6] = [
    "F|Long setup: RSI falls (lowest RSI noted), then rises (entry level = lowest + Entry RSI value); a lower RSI lowers both",
    "E|Long: RSI above the entry level, at the Close",
    "X|Long: after entry RSI rises (highest noted) then falls (exit level = highest - Exit RSI value); a higher RSI raises both",
    "X|Long: RSI below the exit level, at the Close",
    "F|Short side mirrored (highest RSI then fall for the entry, lowest RSI then rise for the cover)",
    "F|RSI is filled with a constant on the first RSI-period bars, as the AFLs' IIf(BarIndex() < period, ..) does",
];

pub fn catalogue() -> Vec<StrategyDef> {
    vec![
        def(CROSS, "RSI 30 and 40 Crossover Entry and Exit", "Long", &[
            "E|10-period RSI crosses below 30 (below today, above yesterday), not on a bar whose High = Close: buy at the Close",
            "X|RSI crosses above 40 while the bar count since entry is within Exit Days Count (Low != Close): at the Close",
            "X|Or the bar count reaches Exit Days Count: at the Close (on a Low = Close bar, the next bar without it)",
        ], "RSI 30 and 40 Crossover Entry and Exit/RSI 30 and 40 Crossover Entry and Exit.afl, coded as the AFL is written. Its defaults lie outside their own Param ranges and are used as written: RSI Sell Value 40 (range 50-90) and Exit Days Count 0 (range 3-10). With Exit Days Count 0 the bar count is 1 from the first bar in the trade, so neither exit can fire: the first trade is held to the end of the data; set Exit Days Count to 3-10 for the described system. A pending low-circuit skip is not cleared by the RSI exit (AFL as written)."),
        def(TRAIL, "RSI Trailing Entry and Exit (no filter)", "Long/Short", &[
            TRAIL_RULES[0], TRAIL_RULES[1], TRAIL_RULES[2], TRAIL_RULES[3], TRAIL_RULES[4], TRAIL_RULES[5],
            "F|Long entry also needs RSI below 50 (RSI on Entry candle should be less than)",
            "F|Side: 0 the Long Only AFL, 1 the Short Only AFL",
        ], "RSI with Trailing SL based Entry and Exit without filter, Long Only.afl (RSI_Trail based entry and Exit/RSI Without filter, identical copy in RSI Afls) and ... Short Only.afl, coded as the AFLs are written: the trackers are separate if-blocks, so the entry can fire on the bar that sets the entry level, and the exit tracker runs on the entry bar. The long fills the first RSI-period bars with 100, the short with 1."),
        def(TRAIL_V1, "RSI Trailing Entry and Exit with Monthly ST Filter (Ver 1)", "Long/Short", &[
            TRAIL_RULES[0], TRAIL_RULES[1], TRAIL_RULES[2], TRAIL_RULES[3], TRAIL_RULES[4], TRAIL_RULES[5],
            "F|Market filter: Supertrend(5, 1) of the monthly mid point ((H+L)/2), expanded to the daily bars",
            "F|Running: long entries only while the filter is up, a long is sold on any bar the filter is down (short mirrored)",
            "F|Fresh trade: signals are tracked whatever the filter; a trade is taken only when the filter agrees at the entry signal, and it is closed when the filter flips against it (prev bar up, now down for a long)",
        ], "RSI Trail SL/Ver 1 and RSI_Trail based entry and Exit/RSI With filter VER1, coded as the AFLs are written: ... VER1_ Bull Long_Long Only_Running.afl (two copies with the same defaults, Param and Optimize versions) = side 0, mode 0; ... Bull Long_ Long Only_fresh trade.afl = side 0, mode 1; ... VER1_Bear Short_Short Only_Running.afl = side 1, mode 0; ... VER1_Bear Short_Short Only_Fresh trade.afl = side 1, mode 1. Filter NONE (the AFL's other ParamList choice) makes the short side take no trade (it needs a down filter). In the fresh versions the loop's virtual position (taken when the filter disagrees) is tracked like a real one; its Sell / Cover is ignored by the backtester. The long fills the first RSI bars with 100, the short with 1."),
        def(TRAIL_V2, "RSI Trailing Entry and Exit with ST Filter (Ver 2)", "Long/Short", &[
            TRAIL_RULES[0], TRAIL_RULES[1], TRAIL_RULES[2], TRAIL_RULES[3], TRAIL_RULES[4], TRAIL_RULES[5],
            "F|Entry Type 0 Short Only (default): shorts only while the monthly ST filter is down, covered when it is up",
            "F|Entry Type 1 Long Only: no filter",
            "F|Entry Type 2 Long & Short: both trackers run all the time; buy / short signals reverse the position, with the AFL's filter conditions",
        ], "RSI Trail SL/Ver 2 and RSI_Trail based entry and Exit/RSI With filter VER2 (identical copies) ... with Filter VER 2.afl, coded as the AFL is written: an else-if chain, so the entry waits for a bar on which no tracker step fires. Its Long Only side reads the Exit RSI value for the entry level and the Entry RSI value for the exit level (both 20 by default). In Long & Short a filter flip from down to up writes a Cover (also while flat, which clears a pending short signal). RSI filled with 1 on the first bars."),
        def(TRAIL_V3, "RSI Trailing Entry and Exit with ST Filter and Market (Ver 3)", "Long/Short", &[
            TRAIL_RULES[0], TRAIL_RULES[1], TRAIL_RULES[2], TRAIL_RULES[3], TRAIL_RULES[4], TRAIL_RULES[5],
            "F|Select market Bull: entries only while the monthly ST filter is up, exits when it is down; Bear: the reverse; Both: no filter",
            "F|Entry Type 0 Short Only (default), 1 Long Only, 2 Long & Short (reversing)",
        ], "RSI_Trail based entry and Exit/RSI With filter VER3/RSI with Trailing SL based Entry and Exit with Filter VER 3.afl, coded as the AFL is written (defaults Short Only, market BULL: shorts taken in an up market and covered when it turns down). In Long & Short / BEAR a short is covered by a buy signal only while the filter is down, as written. RSI filled with 1 on the first bars."),
        def(LOWEST, "Lowest RSI Trailing Entry and ST Fresh Sell Exit", "Long", &[
            "F|RSI(14) falls (lowest noted), then rises: entry level = lowest RSI + 10; a lower RSI lowers both",
            "E|RSI above the entry level and below 80 (threshold), High != Close: buy at the Close",
            "X|Supertrend(5, 1) on the mid point flips to down (fresh sell): at the Close",
            "X|Optional hard SL % of the entry price: Low <= SL, at the SL or Open - buffer on a gap below it",
            "X|A sell on a Low = Close bar waits for the next bar without it, at its Close",
        ], "Lowest RSI Trailing based Entry and ST Fresh Sell Exit/AFLs/Lowest RSI Trailing based Entry and ST Fresh Sell Exit.afl, coded as the AFL is written (ST timeframe HOURLY by default, which is the daily bars here). Same id: the folder-root copy (Version 1 = 'root': a fresh sell on a Low = Close bar is dropped, not deferred), AFLs/(Hard SL) (hard SL 1%), AFLs/(ST as Filter) (ST on MONTHLY, ST entry filter Off by default) and Stocks/Stocks/(ST as Filter) (the same with hard SL 1%; run by the team on single stocks). Comments.docx notes the team's Bank Nifty daily runs."),
        def(LOWEST_SW, "Lowest RSI Trailing Entry and ST Fresh Sell Exit + % Swing Filter", "Long", &[
            "F|Lowest-RSI entry and ST fresh-sell exit as in the plain version",
            "F|% swing filter on the close: up above the band (bull 44%) or a new all-time-high close, down below the band (bear 23%)",
            "E|Entry only while the swing filter is up",
            "X|Sell on any bar the swing filter is down (also on a Low = Close bar)",
        ], "Lowest RSI Trailing based Entry and ST Fresh Sell Exit/AFLs/...+% SWING FILTER.afl, coded as the AFL is written (filter type % SWING by default). Same id: Stocks/NIFTY(BANK)/...+% SWING FILTER.afl and Stocks/Stocks/...+% SWING FILTER.afl, which add the hard SL (1% by default there; 0 here)."),
        def(LOWEST_ATHC, "Lowest RSI Trailing Entry and ST Fresh Sell Exit + ATHC Filter", "Long", &[
            "F|Lowest-RSI entry and ST fresh-sell exit as in the plain version",
            "F|ATHC: on after a close above the previous all-time-high close, off when the 20% close swing turns down",
            "E|Entry only while ATHC is on",
            "X|Sell when ATHC is off (a Low = Close bar waits)",
        ], "Lowest RSI Trailing based Entry and ST Fresh Sell Exit/Stocks/...+% ATHC FILTER.afl, coded as the AFL is written (filter type ATHC; Swing EXIT % default 20 lies outside its 1-5 range and is used as written). Same id: Stocks/Stocks/...+% ATHC FILTER.afl (hard SL 1% by default there; 0 here). The team ran these on single stocks."),
        def(BREACH, "RSI Breach Buy and Exit on Fresh Sell in ST", "Long", &[
            "E|RSI(14) crosses below 40, High != Close: buy at the Close",
            "X|Daily Supertrend(5, 2) on the mid point flips to down (fresh sell): at the Close; on a Low = Close bar, the next bar without it",
            "F|Filter Version 0 Off (default); 1 ST: entry needs daily ST(5, 4) up, also sells when it is down; 2 ATHC: entry needs ATHC on, also sells when it is off",
        ], "RSI breach Buy and Exit on Fresh Sell in ST/RSI breach Buy and Exit on Fresh Sell in ST.afl, coded as the AFL is written; RSI Afls/RSI breach Buy and Exit on Fresh Sell in ST.afl differs only in Param ranges. Its 'Buffer %' is declared but unused."),
        def(DIP, "ST Buy-Sell when RSI Dips below 30", "Long", &[
            "F|RSI(14) below 30 arms the setup (kept until a sell)",
            "E|Armed and daily Supertrend(5, 1) up, High != Close: buy at the Close",
            "X|Both Supertrend(5, 1) and Supertrend(5, 2) down, Low != Close: at the Close",
            "F|Optional filter (Off by default): ST(5, 1) on the filter timeframe; while it is down a long is sold (Low != Close) and the setup cleared",
        ], "ST buy-sell when RSI gives a dip below 30/ST LONG ONLY WITH RSI AND FLT.afl, coded as the AFL is written. It has no SetBarsRequired, so the loops (and the indicators) start the longest look-back before the run start. The AFL's ST function returns its trend with VarSet; that is taken to set the caller's BTREND1 / BTREND2 as intended."),
        def(AK_BULL, "Aakash RSI with ST and SMA Filter (Bull, Same Day)", "Long", &[
            "E|RSI(14) crosses above 60 with the market filter up: buy at the Close",
            "F|Filter 0 ST (default): monthly Supertrend(5, 1) on the mid point up; 1 SMA: Close above its 200-day SMA",
            "X|RSI falls 20 below its highest value since entry: at the Close",
        ], "Aakash RSI/Aakash RSI with ST and SMA Filter(Bull, Long Only, Same Day).afl, coded as the AFL is written. No exit on the filter."),
        def(AK_BEAR, "Aakash RSI with ST and SMA Filter (Bear, Same Day)", "Long/Short", &[
            "F|Bear market: monthly Supertrend(5, 1) down (filter 0) or Close below the 200-day SMA (filter 1)",
            "E|Long Only (default): RSI(14) crosses above 60 in a bear market: buy at the Close",
            "X|Long: RSI falls 20 below its highest value since entry: at the Close",
            "E|Short Only: RSI crosses below 40 in a bear market: short at the Close",
            "X|Short: RSI rises 20 above its lowest value since entry: at the Close",
        ], "Aakash RSI/Aakash RSI with ST and SMA Filter(Bear, Long Only and Short Only, Same Day).afl, coded as the AFL is written."),
        def(AK_BULL_N, "Aakash RSI with ST and SMA Filter (Bull, Next Day)", "Long", &[
            "F|Signal: RSI(14) crosses above 60 with the market filter up; the signal bar's High is noted",
            "E|Next bar (Entry Period 1) trades above that High with the filter up: buy at the High, or Open + 1% when it opens above",
            "F|Otherwise the signal is dropped",
            "X|RSI falls 20 below its highest value since entry: at the Close",
        ], "Aakash RSI/Aakash RSI with ST and SMA Filter(Bull, Long Only, Next Day).afl, coded as the AFL is written. In its SMA branch the drop of an unfilled signal does not reset the bar count, so after the first unfilled SMA signal no later signal can fill until a sell (AFL as written)."),
        def(AK_BEAR_N, "Aakash RSI with ST and SMA Filter (Bear, Next Day)", "Long/Short", &[
            "F|Long signal: RSI(14) crosses above 60; short signal: RSI crosses below 40; the signal bar's High / Low is noted",
            "E|Next bar breaks it: buy at the High (Open + 1% above it) / short at the Low (Open - 1% below it)",
            "F|Filter 0 none (default); 1 monthly ST down; 2 Close below the 200-day SMA; 3 RSI filter: RSI(14) of the filter timeframe below 40 at entry (the short also needs ST down)",
            "X|RSI trails 20 points from its best value since entry: at the Close",
        ], "Aakash RSI/Aakash RSI with ST and SMA Filter(Bear, Long Only and Short Only, Next Day).afl, coded as the AFL is written (Long Only, No Filter by default). The short entry has no entry-period check outside the SMA branch, and its cover does not reset the bar count (AFL as written)."),
        def(CONNORS, "Connors RSI(2) Entry Exit (F&O)", "Long/Short", &[
            "E|Long: Close above its 200-day SMA and below its 5-day MA, RSI(2) crosses below 5: buy at the Close",
            "X|Long (MA exit, default): Close below the 200-day SMA or above the 5-day MA: at the Close",
            "X|Profit target 5% / hard SL 2% (or ATR(5) x 5 / x 3): at the level, or Open - 0.25% buffer on a gap through it",
            "E|Short: mirrored with RSI crossing above 95",
            "F|Exit versions: MA, Days (bars in trade > 1), Lower Close / Lower Low (long), Higher Close / Higher High (short)",
        ], "CONNERS RSI OPTIM/Connor RSI EntryExit for FO.afl, coded as the AFL is written. A side / exit-version pair the AFL has no block for (e.g. Long with Higher Close) takes no trade. A target or stop of 0 leaves the AFL's Null level, which the loop reads as -1e10: a long then exits on the next bar (High >= -1e10), as written. The team ran it on F&O symbols; here one symbol's daily bars."),
        def(SLT, "RSI with ST(54) & MA(200) Filter and %SL & ATR SL", "Long", &[
            "E|RSI(14) crosses below 30 (version 0; Delayed: crosses above 30), High != Close and Low != Close: buy at the Close",
            "X|RSI crosses above 60 (Delayed: below 60): at the Close",
            "X|%: target 10% (High, at the level or Open - buffer), hard SL 2% and trailing SL 5% of the highest close (on the Close)",
            "X|ATR: target Close + 10 ATR(5), hard SL - 3 ATR, trailing 5 ATR below the highest close",
            "F|Filter versions: none, daily ST(5, 4) up for entry / down sells, Close above / below the 200-day MA",
            "X|A sell on a Low = Close bar is taken on the next bar without a low circuit, near the Open",
        ], "RSI with ST(54) & MA(200) Filter and with %SL & ATR SL.afl (folder root), coded as the AFL is written; Delayed 1 = RSI Delayed EntryExit with ST(54) & MA(200) Filter and with %SL & ATR SL.afl (the same AFL with the RSI crosses reversed). Version 0 'RSI with %SL' by default. A deferred sell fills at the Open on an Open = Low bar, else Open - buffer if the Low is below it, else the Low (the Close if exactly equal)."),
        def(HLLH, "RSI with Highest Low & Lowest High Entry Exit", "Long", &[
            "F|RSI(14) below 30 starts a buy zone: line = 10-bar lowest High; RSI above 60 starts a sell zone: line = 10-bar highest Low",
            "E|In the buy zone, Close above the lowest-High line (High != Close, Low != Close): buy at the Close",
            "X|Close below the highest-Low line: at the Close; plus the %SL / ATR SL and filter versions of the template",
        ], "RSI with Highest Low & Lowest High EntryExit with ST(54) & MA(200) Filter and with %SL & ATR SL.afl (folder root), coded as the AFL is written; the stop, target and filter versions are those of the RSI with ST(54) & MA(200) AFL. The RSI thresholds are read with [i] on scalars (taken as the scalar)."),
        def(OBOS, "RSI Overbought/Oversold Delayed Entry Exit (exit versions)", "Long", &[
            "E|RSI(14) crosses above 30 (leaves oversold), High != Close: buy at the Close",
            "X|RSI crosses below 60 (leaves overbought), Low != Close: at the Close; otherwise next bar without a low circuit at Open - 0.25%",
            "F|Versions: Simple RSI (default), hard % SL on the Close, hard ATR SL, daily SuperTrend(5, 1) (entry up, exit down), 200-day MA exit",
        ], "RSI OverboughtOversold Delayed EntryExit with Exit versions.afl (folder root), coded as the AFL is written. Its hard % SL and ATR SL default to 0 (off)."),
    ]
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    // the AFLs' own defaults; a declared range that excludes its default is widened to include it
    let p = |key: &'static str, label: &'static str, def: f64, min: f64, max: f64, step: f64| ParamDef { key, label, def, min: min.min(def), max: max.max(def), step };
    let st = |v: &mut Vec<ParamDef>, tf_label: &'static str, tf_def: f64, point_max: f64, n_def: f64, f_def: f64| {
        v.push(p("flt_tf", tf_label, tf_def, 0.0, 2.0, 1.0));
        v.push(p("flt_point", "Filter ST calculation point (1 mid, 2 close, 3 pivot, 4 avg, 5 MA3, 6 MA5)", 1.0, 1.0, point_max, 1.0));
        v.push(p("flt_n", "Filter ST period", n_def, 1.0, 25.0, 1.0));
        v.push(p("flt_f", "Filter ST factor", f_def, 0.5, 20.0, 0.5));
    };
    const MWD: &str = "Filter timeframe (0 monthly, 1 weekly, 2 daily)";
    const DWM: &str = "Filter timeframe (0 daily, 1 weekly, 2 monthly)";
    Some(match id {
        CROSS => vec![
            p("rsi_n", "RSI period", 10.0, 4.0, 26.0, 2.0),
            p("buy_val", "RSI buy value", 30.0, 20.0, 70.0, 10.0),
            p("sell_val", "RSI sell value", 40.0, 50.0, 90.0, 10.0),
            p("exit_days", "Exit days count", 0.0, 3.0, 10.0, 1.0),
        ],
        TRAIL => vec![
            p("side", "Side (0 long only, 1 short only)", 0.0, 0.0, 1.0, 1.0),
            p("rsi_n", "RSI period", 14.0, 2.0, 28.0, 1.0),
            p("entry_val", "Entry RSI value", 20.0, 5.0, 30.0, 5.0),
            p("exit_val", "Exit RSI value", 20.0, 5.0, 30.0, 5.0),
            p("thresh", "Long: RSI on entry candle below", 50.0, 30.0, 60.0, 5.0),
        ],
        TRAIL_V1 | TRAIL_V2 | TRAIL_V3 => {
            let mut v = match id {
                TRAIL_V1 => vec![p("side", "Side (0 long only, 1 short only)", 0.0, 0.0, 1.0, 1.0), p("fresh", "Mode (0 running, 1 fresh trade)", 0.0, 0.0, 1.0, 1.0)],
                _ => vec![p("side", "Entry type (0 short only, 1 long only, 2 long & short)", 0.0, 0.0, 2.0, 1.0)],
            };
            if id == TRAIL_V3 {
                v.push(p("mkt", "Select market (0 bull, 1 bear, 2 both)", 0.0, 0.0, 2.0, 1.0));
            }
            v.push(p("flt", "Market filter (0 none, 1 ST)", 1.0, 0.0, 1.0, 1.0));
            st(&mut v, MWD, 0.0, 6.0, 5.0, 1.0);
            v.push(p("rsi_n", "RSI period", 14.0, 1.0, 100.0, 1.0));
            v.push(p("entry_val", "Entry RSI value", 20.0, 0.0, 100.0, 0.1));
            v.push(p("exit_val", "Exit RSI value", 20.0, 0.0, 100.0, 0.1));
            v
        }
        LOWEST | LOWEST_SW | LOWEST_ATHC => {
            let mut v = vec![
                p("rsi_n", "RSI period", 14.0, 6.0, 32.0, 2.0),
                p("entry_val", "Entry RSI value", 10.0, 5.0, 35.0, 5.0),
                p("thresh", "RSI threshold value", 80.0, 40.0, 90.0, 5.0),
                p("st_tf", "ST timeframe (0 hourly = daily bars, 1 daily, 2 weekly, 3 monthly)", 0.0, 0.0, 3.0, 1.0),
                p("st_point", "ST calculation point (1 mid, 2 close, 3 pivot)", 1.0, 1.0, 3.0, 1.0),
                p("st_n", "ST period", 5.0, 1.0, 25.0, 1.0),
                p("st_f", "ST factor", 1.0, 1.0, 12.0, 0.5),
                p("hsl_pct", "Hard SL % (0 off)", 0.0, 0.0, 10.0, 1.0),
                p("buffer_pct", "Selling buffer %", 0.25, 0.0, 10.0, 0.05),
            ];
            match id {
                LOWEST => {
                    v.push(p("st_entry", "ST as entry filter (0 off, 1 on)", 0.0, 0.0, 1.0, 1.0));
                    v.push(p("root", "Version (0 AFLs folder, 1 folder-root copy)", 0.0, 0.0, 1.0, 1.0));
                }
                LOWEST_SW => {
                    v.push(p("flt_on", "Filter type (0 no filter, 1 % swing)", 1.0, 0.0, 1.0, 1.0));
                    v.push(p("bear_pct", "Swing % for bear market", 23.0, 15.0, 45.0, 5.0));
                    v.push(p("bull_pct", "Swing % for bull market", 44.0, 30.0, 50.0, 5.0));
                }
                _ => {
                    v.push(p("flt_on", "Filter type (0 no filter, 1 ATHC)", 1.0, 0.0, 1.0, 1.0));
                    v.push(p("exit_pct", "Swing exit %", 20.0, 1.0, 10.0, 1.0));
                }
            }
            v
        }
        BREACH => vec![
            p("rsi_n", "RSI period", 14.0, 1.0, 100.0, 1.0),
            p("short_val", "RSI short value", 40.0, 10.0, 80.0, 5.0),
            p("filter", "Filter version (0 off, 1 ST, 2 ATHC)", 0.0, 0.0, 2.0, 1.0),
            p("st_tf", "Exit ST timeframe (0 daily, 1 weekly, 2 monthly)", 0.0, 0.0, 2.0, 1.0),
            p("st_point", "Exit ST calculation point (1 mid, 2 close, 3 pivot)", 1.0, 1.0, 3.0, 1.0),
            p("st_n", "Exit ST period", 5.0, 5.0, 25.0, 1.0),
            p("st_f", "Exit ST factor", 2.0, 1.0, 6.0, 0.5),
            p("flt_tf", DWM, 0.0, 0.0, 2.0, 1.0),
            p("flt_point", "Filter ST calculation point (1 mid, 2 close, 3 pivot)", 1.0, 1.0, 3.0, 1.0),
            p("flt_n", "Filter ST period", 5.0, 1.0, 20.0, 1.0),
            p("flt_f", "Filter ST factor", 4.0, 1.0, 20.0, 1.0),
            p("exit_pct", "ATHC swing exit %", 20.0, 10.0, 21.0, 1.0),
        ],
        DIP => {
            let mut v = vec![
                p("rsi_n", "RSI period", 14.0, 1.0, 100.0, 1.0),
                p("rsi_lvl", "RSI deep level", 30.0, 10.0, 50.0, 10.0),
                p("st_point", "Daily ST calculation point (1 mid, 2 close, 3 pivot)", 1.0, 1.0, 3.0, 1.0),
                p("e_n", "ST period for entry", 5.0, 1.0, 20.0, 1.0),
                p("e_f", "ST factor for entry", 1.0, 1.0, 5.0, 0.5),
                p("x_n", "ST period for exit", 5.0, 1.0, 20.0, 1.0),
                p("x_f", "ST factor for exit", 2.0, 1.0, 6.0, 0.5),
                p("flt", "Filter (0 off, 1 on)", 0.0, 0.0, 1.0, 1.0),
            ];
            st(&mut v, DWM, 0.0, 3.0, 5.0, 1.0);
            v
        }
        AK_BULL | AK_BEAR | AK_BULL_N | AK_BEAR_N => {
            let mut v = Vec::new();
            if id == AK_BEAR || id == AK_BEAR_N {
                v.push(p("side", "Trade version (0 long only, 1 short only)", 0.0, 0.0, 1.0, 1.0));
            }
            v.push(if id == AK_BEAR_N { p("flt", "Market filter (0 none, 1 ST, 2 SMA, 3 RSI)", 0.0, 0.0, 3.0, 1.0) } else { p("flt", "Market filter (0 ST, 1 SMA)", 0.0, 0.0, 1.0, 1.0) });
            st(&mut v, MWD, 0.0, 6.0, 5.0, 1.0);
            v.push(p("sma_n", "MA period", 200.0, 1.0, 500.0, 1.0));
            v.push(p("rsi_n", "RSI period", 14.0, 1.0, 100.0, 1.0));
            v.push(p("buy_val", "RSI buy value", 60.0, 1.0, 100.0, 1.0));
            if id != AK_BULL && id != AK_BULL_N {
                v.push(p("short_val", "RSI short value", 40.0, 1.0, 100.0, 1.0));
            }
            v.push(p("trail", "RSI trail value", 20.0, 0.0, 100.0, 1.0));
            if id == AK_BULL_N || id == AK_BEAR_N {
                v.push(p("bs_pct", "Open %", 1.0, 0.0, 20.0, 0.001));
                v.push(p("entry_period", "Entry period", 1.0, 1.0, 100.0, 1.0));
            }
            if id == AK_BEAR_N {
                v.push(p("rf_tf", "RSI filter timeframe (0 hourly = daily bars, 1 daily, 2 weekly, 3 monthly)", 0.0, 0.0, 3.0, 1.0));
                v.push(p("rf_n", "RSI filter period", 14.0, 1.0, 100.0, 1.0));
                v.push(p("rf_bear", "RSI bear value", 40.0, 1.0, 100.0, 1.0));
            }
            v
        }
        CONNORS => vec![
            p("side", "Side (0 long only, 1 short only)", 0.0, 0.0, 1.0, 1.0),
            p("exit_ver", "Exit version (0 MA, 1 days, 2 higher close, 3 lower close, 4 higher high, 5 lower low)", 0.0, 0.0, 5.0, 1.0),
            p("stop", "PT & SL type (0 % based, 1 ATR based)", 0.0, 0.0, 1.0, 1.0),
            p("sma_n", "MA filter periods", 200.0, 1.0, 500.0, 1.0),
            p("rsi_n", "RSI period", 2.0, 2.0, 18.0, 2.0),
            p("buy_val", "RSI buy value", 5.0, 5.0, 20.0, 5.0),
            p("short_val", "RSI short value", 95.0, 80.0, 95.0, 5.0),
            p("ma_n", "MA sell / cover period", 5.0, 2.0, 12.0, 1.0),
            p("pt_pct", "Profit booking %", 5.0, 0.0, 30.0, 0.1),
            p("hsl_pct", "Hard SL %", 2.0, 0.0, 30.0, 0.01),
            p("atr_n", "ATR periods", 5.0, 1.0, 100.0, 1.0),
            p("atr_pt", "ATR profit target multiplier", 5.0, 0.0, 20.0, 1.0),
            p("atr_sl", "ATR hard SL multiplier", 3.0, 0.0, 20.0, 1.0),
            p("buffer_pct", "Buffer %", 0.25, 0.0, 10.0, 0.05),
            p("exit_days", "Days for exit", 1.0, 1.0, 8.0, 1.0),
            p("hc_n", "Higher close period", 2.0, 1.0, 3.0, 1.0),
            p("lc_n", "Lower close period", 2.0, 1.0, 3.0, 1.0),
            p("hh_n", "Higher high period", 2.0, 1.0, 3.0, 1.0),
            p("ll_n", "Lower low period", 2.0, 1.0, 3.0, 1.0),
        ],
        SLT | HLLH => {
            let mut v = Vec::new();
            if id == SLT {
                v.push(p("delayed", "Entry (0 RSI crosses into the zone, 1 delayed: crosses out of it)", 0.0, 0.0, 1.0, 1.0));
            }
            v.push(p("ver", "Version (0 %SL, 1 ATR SL, 2 ST & %SL, 3 ST & ATR SL, 4 MA & %SL, 5 MA & ATR SL)", 0.0, 0.0, 5.0, 1.0));
            v.extend([
                p("pt_pct", "Profit booking %", 10.0, 0.0, 30.0, 0.1),
                p("hsl_pct", "Hard SL %", 2.0, 0.0, 30.0, 0.01),
                p("tsl_pct", "Trailing SL %", 5.0, 0.0, 30.0, 0.01),
                p("buffer_pct", "Selling buffer %", 0.25, 0.0, 10.0, 0.05),
                p("atr_n", "ATR periods", 5.0, 1.0, 100.0, 1.0),
                p("atr_pt", "ATR profit target multiplier", 10.0, 0.0, 30.0, 1.0),
                p("atr_sl", "ATR hard SL multiplier", 3.0, 0.0, 20.0, 1.0),
                p("atr_tsl", "ATR trail SL multiplier", 5.0, 0.0, 20.0, 0.001),
            ]);
            st(&mut v, DWM, 0.0, 3.0, 5.0, 4.0);
            v.extend([
                p("ma_n", "MA period", 200.0, 1.0, 600.0, 1.0),
                p("rsi_n", "RSI period", 14.0, 1.0, 100.0, 1.0),
                p("buy_val", "RSI buy value", 30.0, 1.0, 100.0, 1.0),
                p("sell_val", "RSI sell value", 60.0, 1.0, 100.0, 1.0),
            ]);
            if id == HLLH {
                v.push(p("lh_n", "Lowest high periods", 10.0, 1.0, 100.0, 1.0));
                v.push(p("hl_n", "Highest low periods", 10.0, 1.0, 100.0, 1.0));
            }
            v
        }
        OBOS => {
            let mut v = vec![
                p("ver", "Version (0 simple RSI, 1 hard % SL, 2 hard ATR SL, 3 SuperTrend, 4 MA exit)", 0.0, 0.0, 4.0, 1.0),
                p("hsl_pct", "Hard % SL", 0.0, 0.0, 10.0, 0.001),
                p("atr_n", "ATR periods for SL", 1.0, 1.0, 100.0, 1.0),
                p("atr_m", "Hard ATR SL", 0.0, 0.0, 10.0, 0.001),
                p("buffer_pct", "Buffer sell %", 0.25, 0.0, 2.0, 0.001),
                p("rsi_n", "RSI period", 14.0, 1.0, 100.0, 1.0),
                p("buy_val", "RSI buy value", 30.0, 10.0, 50.0, 1.0),
                p("sell_val", "RSI sell value", 60.0, 50.0, 90.0, 1.0),
            ];
            st(&mut v, DWM, 0.0, 3.0, 5.0, 1.0);
            v.push(p("ma_n", "MA period", 200.0, 1.0, 600.0, 1.0));
            v
        }
        _ => return None,
    })
}

// ---------------------------------------------------------------- shared pieces

const BIG: f64 = 9_999_999.0;
/// AFL Null as a loop value.
const NUL: f64 = -1e10;

/// Per-bar Buy / Sell / Short / Cover written by an AFL loop, with their prices (Sell / Cover with a reason).
struct Sig {
    buy: Vec<Option<f64>>,
    sell: Vec<Option<(f64, &'static str)>>,
    short: Vec<Option<f64>>,
    cover: Vec<Option<(f64, &'static str)>>,
}
impl Sig {
    fn new(n: usize) -> Self {
        Sig { buy: vec![None; n], sell: vec![None; n], short: vec![None; n], cover: vec![None; n] }
    }
}

/// AmiBroker backtest of loop signals in from..=to: exit first, then entry; an opposite entry reverses; prices kept
/// inside the bar's range; a position still open at `to` is marked there.
fn backtest(x: &Ctx, s: &Sig, from: usize, to: usize) -> Vec<Trade> {
    let mut out = Vec::new();
    let mut pos: Option<(f64, usize, f64)> = None;
    for i in from..=to {
        if let Some((d, e, epx)) = pos {
            let ex = if d > 0.0 { s.sell[i].or(s.short[i].map(|p| (p, "Reversed by a short entry"))) } else { s.cover[i].or(s.buy[i].map(|p| (p, "Reversed by a buy entry"))) };
            if let Some((px, r)) = ex {
                out.push(trade(d, e, epx, i, bound(x, i, px), r, false));
                pos = None;
            }
        }
        if pos.is_none() {
            if let Some(px) = s.buy[i] {
                pos = Some((1.0, i, bound(x, i, px)));
            } else if let Some(px) = s.short[i] {
                pos = Some((-1.0, i, bound(x, i, px)));
            }
        }
    }
    if let Some((d, e, epx)) = pos {
        out.push(trade(d, e, epx, to, x.c[to], "open", true));
    }
    out
}

/// RSI(n) with the first n bars set to `fill` (the AFLs' IIf(BarIndex() < n, fill, RSI(n))); NaN fill keeps the warm-up Null.
fn rsi_fill(c: &[f64], n: usize, fill: f64) -> Vec<f64> {
    let mut r = rsi(c, n.max(1));
    for v in r.iter_mut().take(n) {
        *v = fill;
    }
    r
}

/// AFL MA(C, n) on a bar series: NaN until n values exist.
fn ma_nan(c: &[f64], n: usize) -> Vec<f64> {
    sma(c, n.max(1)).into_iter().map(|v| v.unwrap_or(f64::NAN)).collect()
}

/// The AFLs' Supertrend loop trend (1 up, -1 down) on the timeframe `tf` ('D', 'W', 'M'), expanded to the daily bars.
/// Calculation point: 1 mid (H+L)/2, 2 close, 3 pivot (H+L+C)/3, 4 (mid + pivot + close)/3, 5 MA(C, 3), 6 MA(C, 5).
fn st_trend(x: &Ctx, tf: char, point: i64, period: usize, factor: f64) -> Vec<i8> {
    let b = compress(x, tf);
    let m = b.c.len();
    let mid = |g: usize| (b.h[g] + b.l[g]) / 2.0;
    let piv = |g: usize| (b.h[g] + b.l[g] + b.c[g]) / 3.0;
    let calc: Vec<f64> = match point {
        2 => b.c.clone(),
        3 => (0..m).map(piv).collect(),
        4 => (0..m).map(|g| (mid(g) + piv(g) + b.c[g]) / 3.0).collect(),
        5 => ma_nan(&b.c, 3),
        6 => ma_nan(&b.c, 5),
        _ => (0..m).map(mid).collect(),
    };
    let (tr, _) = supertrend(&calc, &b.h, &b.l, &b.c, period.max(1), factor, factor);
    expand_last(&tr, &b)
}

fn tf_mwd(code: i64) -> char {
    match code {
        1 => 'W',
        2 => 'D',
        _ => 'M',
    }
}
fn tf_dwm(code: i64) -> char {
    match code {
        1 => 'W',
        2 => 'M',
        _ => 'D',
    }
}
/// HOURLY | DAILY | WEEKLY | MONTHLY; hourly on daily bars stays daily.
fn tf_hdwm(code: i64) -> char {
    match code {
        2 => 'W',
        3 => 'M',
        _ => 'D',
    }
}

/// Filter arrays (up, down) from an ST trend; filter off = (true, false) as the AFLs' `FLTBuy = True; FLTShort = False`.
fn filter_arrays(tr: Option<Vec<i8>>, n: usize) -> (Vec<bool>, Vec<bool>) {
    match tr {
        Some(t) => (t.iter().map(|&v| v == 1).collect(), t.iter().map(|&v| v == -1).collect()),
        None => (vec![true; n], vec![false; n]),
    }
}

/// Running all-time-high close: HHV(C, BarCount).
fn ath_close(c: &[f64]) -> Vec<f64> {
    let mut m = f64::NEG_INFINITY;
    c.iter().map(|&v| { m = m.max(v); m }).collect()
}

/// The swing / ATHC AFLs' ATHC loop: on after a close above the previous all-time-high close, off when the close
/// swing (exit %) turns down.
fn athc(c: &[f64], exit_pct: f64) -> Vec<bool> {
    let n = c.len();
    let hc = ath_close(c);
    let tr = close_swing(c, exit_pct);
    let mut a = vec![false; n];
    for i in 1..n {
        a[i] = if c[i] > hc[i - 1] { true } else if tr[i] == -1 && a[i - 1] { false } else { a[i - 1] };
    }
    a
}

/// The % swing filter of the Lowest-RSI swing AFLs (bull % above, bear % below, a new all-time-high close also up).
fn swing_filter(c: &[f64], bear: f64, bull: f64) -> Vec<i8> {
    let n = c.len();
    let hcl = ath_close(c);
    let mut up: Vec<f64> = c.iter().map(|v| v + v * bull / 100.0).collect();
    let mut dn: Vec<f64> = c.iter().map(|v| v - v * bear / 100.0).collect();
    let mut tr = vec![1i8; n];
    for i in 1..n {
        tr[i] = if c[i] > up[i - 1] || c[i] > hcl[i - 1] { 1 } else if c[i] < dn[i - 1] { -1 } else { tr[i - 1] };
        if tr[i] > 0 && dn[i] < dn[i - 1] {
            dn[i] = dn[i - 1];
        }
        if tr[i] < 0 && up[i] > up[i - 1] {
            up[i] = up[i - 1];
        }
        if tr[i] < 0 && tr[i - 1] > 0 {
            up[i] = hcl[i].min(c[i] + c[i] * bull / 100.0);
        }
        if tr[i] > 0 && tr[i - 1] < 0 {
            dn[i] = c[i] - c[i] * bear / 100.0;
        }
    }
    tr
}

// ---------------------------------------------------------------- RSI 30 / 40 crossover

fn run_cross(r: &[f64], h: &[f64], l: &[f64], c: &[f64], buy: f64, sell: f64, cd: i64) -> Sig {
    let n = c.len();
    let mut s = Sig::new(n);
    let (mut pos, mut skip, mut count) = (false, false, 0i64);
    for i in 1..n {
        if !pos && r[i] < buy && r[i - 1] > buy && h[i] != c[i] {
            s.buy[i] = Some(c[i]);
            pos = true;
        } else if pos {
            if count <= cd {
                count += 1;
            }
            let lc = l[i] == c[i];
            if r[i] > sell && r[i - 1] < sell && count <= cd && !lc {
                s.sell[i] = Some((c[i], "RSI crosses above the sell value"));
                pos = false;
                count = 0;
            } else if count == cd {
                if !lc {
                    s.sell[i] = Some((c[i], "Exit days count"));
                    pos = false;
                    count = 0;
                } else {
                    skip = true;
                }
            } else if skip && !lc {
                s.sell[i] = Some((c[i], "Blocked exit, next bar"));
                pos = false;
                count = 0;
                skip = false;
            }
        }
    }
    s
}

// ---------------------------------------------------------------- RSI trailing family

/// Loop state of the RSI trailing AFLs (names as in the AFLs).
struct Tr {
    pos: i32,
    count: i32,
    count1: i32,
    lowest: f64,
    highest: f64,
    rsi_entry: f64,
    rsi_exit: f64,
    buysig: bool,
    shortsig: bool,
}
impl Tr {
    fn new() -> Self {
        Tr { pos: 0, count: 0, count1: 0, lowest: 0.0, highest: BIG, rsi_entry: 0.0, rsi_exit: BIG, buysig: false, shortsig: false }
    }
    /// Highest-RSI tracker (AFL op 1-3): RSI_Entry = Highest - val. `gt0`: the Long & Short op 1 also needs RSI > 0.
    fn hi(&mut self, a: f64, b: f64, guard: bool, gt0: bool, val: f64) -> bool {
        if guard && (!gt0 || a > 0.0) && a > b && self.count <= 1 {
            self.highest = a;
            self.count = 1;
        } else if guard && a > 0.0 && a < b && self.count == 1 {
            self.count = 2;
            self.rsi_entry = self.highest - val;
        } else if guard && self.highest > 0.0 && a > self.highest && a > b && self.count == 2 {
            self.highest = a;
            self.rsi_entry = self.highest - val;
        } else {
            return false;
        }
        true
    }
    /// Lowest-RSI tracker (AFL op 5-7): RSI_Exit = Lowest + val.
    fn lo(&mut self, a: f64, b: f64, guard: bool, val: f64) -> bool {
        if guard && a < b && self.count1 <= 1 {
            self.lowest = a;
            self.count1 = 1;
        } else if guard && a > 0.0 && a > b && self.count1 == 1 {
            self.count1 = 2;
            self.rsi_exit = self.lowest + val;
        } else if guard && self.lowest > 0.0 && a < self.lowest && self.count1 == 2 {
            self.lowest = a;
            self.rsi_exit = self.lowest + val;
        } else {
            return false;
        }
        true
    }
    fn long_in(&mut self, a: f64) {
        self.highest = a;
        self.count = 1;
        self.pos = 1;
        self.lowest = 0.0;
        self.count1 = 0;
        self.rsi_exit = BIG;
    }
    fn long_out(&mut self, a: f64) {
        self.lowest = a;
        self.pos = 0;
        self.count = 0;
        self.rsi_entry = 0.0;
        self.count1 = 1;
        self.highest = BIG;
    }
    fn short_in(&mut self, a: f64) {
        self.lowest = a;
        self.pos = -1;
        self.count1 = 1;
        self.count = 0;
        self.rsi_entry = 0.0;
        self.highest = BIG;
    }
    fn cover_out(&mut self, a: f64) {
        self.highest = a;
        self.count = 1;
        self.pos = 0;
        self.lowest = 0.0;
        self.count1 = 0;
        self.rsi_exit = BIG;
    }
}

/// Inputs of one trailing-family loop: gate = entry allowed, force = exit forced, virt = take the "fresh trade"
/// virtual position when the gate is closed.
struct TrailIn<'a> {
    r: &'a [f64],
    c: &'a [f64],
    start: usize,
    ev: f64,
    xv: f64,
    thresh: f64,
    gate: &'a [bool],
    force: &'a [bool],
    virt: bool,
}

/// Separate-if trackers (no-filter AFLs and Ver 1): long.
fn trail_seq_long(t: &TrailIn, s: &mut Sig) {
    let mut st = Tr::new();
    for i in t.start.max(1)..t.c.len() {
        let (a, b) = (t.r[i], t.r[i - 1]);
        st.lo(a, b, st.pos == 0, t.ev);
        let cond = st.pos == 0 && a > st.rsi_exit && st.count1 == 2 && a < t.thresh;
        if cond && t.gate[i] {
            s.buy[i] = Some(t.c[i]);
            st.long_in(a);
        } else if cond && t.virt {
            st.long_in(a);
        }
        st.hi(a, b, st.pos > 0, false, t.xv);
        if st.pos > 0 && ((a < st.rsi_entry && st.count == 2) || t.force[i]) {
            s.sell[i] = Some((t.c[i], if a < st.rsi_entry && st.count == 2 { "RSI trailing exit" } else { "Filter exit" }));
            st.long_out(a);
        }
    }
}

/// Separate-if trackers: short.
fn trail_seq_short(t: &TrailIn, s: &mut Sig) {
    let mut st = Tr::new();
    for i in t.start.max(1)..t.c.len() {
        let (a, b) = (t.r[i], t.r[i - 1]);
        st.hi(a, b, st.pos == 0, false, t.ev);
        let cond = st.pos == 0 && a < st.rsi_entry && st.count == 2;
        if cond && t.gate[i] {
            s.short[i] = Some(t.c[i]);
            st.short_in(a);
        } else if cond && t.virt {
            st.short_in(a);
        }
        st.lo(a, b, st.pos < 0, t.xv);
        if st.pos < 0 && ((a > st.rsi_exit && st.count1 == 2) || t.force[i]) {
            s.cover[i] = Some((t.c[i], if a > st.rsi_exit && st.count1 == 2 { "RSI trailing exit" } else { "Filter exit" }));
            st.cover_out(a);
        }
    }
}

/// Else-if chain (Ver 2 / Ver 3 single side): the entry only on a bar where no tracker step fired.
fn trail_chain(t: &TrailIn, long: bool, s: &mut Sig) {
    let mut st = Tr::new();
    for i in t.start.max(1)..t.c.len() {
        let (a, b) = (t.r[i], t.r[i - 1]);
        if long {
            // Ver 2 / 3 long: the entry level uses the Exit RSI value, the exit level the Entry RSI value
            if !st.lo(a, b, st.pos == 0, t.xv) && st.pos == 0 && a > st.rsi_exit && st.count1 == 2 && t.gate[i] {
                s.buy[i] = Some(t.c[i]);
                st.long_in(a);
            }
            st.hi(a, b, st.pos > 0, false, t.ev);
            if st.pos > 0 && ((a < st.rsi_entry && st.count == 2) || t.force[i]) {
                s.sell[i] = Some((t.c[i], if a < st.rsi_entry && st.count == 2 { "RSI trailing exit" } else { "Filter exit" }));
                st.long_out(a);
            }
        } else {
            if !st.hi(a, b, st.pos == 0, false, t.ev) && st.pos == 0 && a < st.rsi_entry && st.count == 2 && t.gate[i] {
                s.short[i] = Some(t.c[i]);
                st.short_in(a);
            }
            st.lo(a, b, st.pos < 0, t.xv);
            if st.pos < 0 && ((a > st.rsi_exit && st.count1 == 2) || t.force[i]) {
                s.cover[i] = Some((t.c[i], if a > st.rsi_exit && st.count1 == 2 { "RSI trailing exit" } else { "Filter exit" }));
                st.cover_out(a);
            }
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Ls {
    V2,
    Bull,
    Bear,
    Both,
}

/// Long & Short (Ver 2 / Ver 3): both trackers run on every bar and raise buy / short signals.
fn trail_ls(r: &[f64], c: &[f64], start: usize, ev: f64, xv: f64, fb: &[bool], fs: &[bool], mode: Ls, s: &mut Sig) {
    let mut st = Tr::new();
    for i in start.max(1)..c.len() {
        let (a, b) = (r[i], r[i - 1]);
        if !st.hi(a, b, true, true, ev) && a < st.rsi_entry && st.count == 2 {
            st.lowest = a;
            st.count1 = 1;
            st.shortsig = true;
            st.count = 0;
            st.rsi_entry = 0.0;
            st.highest = BIG;
        }
        if !st.lo(a, b, true, xv) && a > st.rsi_exit && st.count1 == 2 {
            st.highest = a;
            st.count = 1;
            st.buysig = true;
            st.lowest = 0.0;
            st.count1 = 0;
            st.rsi_exit = BIG;
        }
        let flip_up = fs[i - 1] && fb[i];
        let flip_dn = fs[i] && fb[i - 1];
        if st.pos <= 0 {
            let cover = match mode {
                Ls::V2 => (st.pos == -1 && st.buysig && fs[i]) || flip_up,
                Ls::Bull => st.pos == -1 && (st.buysig || fs[i]),
                Ls::Bear => st.pos == -1 && st.buysig && fs[i],
                Ls::Both => st.pos == -1 && st.buysig,
            };
            if cover {
                s.cover[i] = Some((c[i], if st.buysig { "Buy signal" } else { "Filter exit" }));
                st.pos = 0;
                st.shortsig = false;
                if mode == Ls::Bull || mode == Ls::Bear {
                    st.buysig = false;
                }
            }
            let gate = match mode {
                Ls::Bull => fb[i],
                Ls::Bear => fs[i],
                _ => true,
            };
            if st.buysig && gate {
                s.buy[i] = Some(c[i]);
                st.pos = 1;
                st.buysig = false;
                st.shortsig = false;
            }
        }
        if st.pos >= 0 {
            let sell = match mode {
                Ls::V2 => st.pos == 1 && (st.shortsig || flip_dn),
                Ls::Bull => st.pos == 1 && (st.shortsig || fs[i]),
                Ls::Bear => st.pos == 1 && (st.shortsig || fb[i]),
                Ls::Both => st.pos == 1 && st.shortsig,
            };
            if sell {
                s.sell[i] = Some((c[i], if st.shortsig { "Short signal" } else { "Filter exit" }));
                st.buysig = false;
                st.pos = 0;
                if mode == Ls::Bull || mode == Ls::Bear {
                    st.shortsig = false;
                }
            }
            if mode == Ls::V2 && flip_dn {
                st.shortsig = false;
            }
            let gate = match mode {
                Ls::V2 | Ls::Bear => fs[i],
                Ls::Bull => fb[i],
                Ls::Both => true,
            };
            if st.shortsig && gate {
                s.short[i] = Some(c[i]);
                st.pos = -1;
                st.shortsig = false;
                st.buysig = false;
            }
        }
    }
}

fn run_trail(id: &str, x: &Ctx, g: &dyn Fn(&str) -> f64) -> Sig {
    let (c, n) = (&x.c, x.c.len());
    let gi = |k: &str| g(k).round() as i64;
    let rn = g("rsi_n").round().max(1.0) as usize;
    let (ev, xv) = (g("entry_val"), g("exit_val"));
    let side = gi("side");
    let mut s = Sig::new(n);
    let none = vec![false; n];
    let all = vec![true; n];
    if id == TRAIL {
        if side == 1 {
            let r = rsi_fill(c, rn, 1.0);
            trail_seq_short(&TrailIn { r: &r, c, start: rn, ev, xv, thresh: f64::INFINITY, gate: &all, force: &none, virt: false }, &mut s);
        } else {
            let r = rsi_fill(c, rn, 100.0);
            trail_seq_long(&TrailIn { r: &r, c, start: rn, ev, xv, thresh: g("thresh"), gate: &all, force: &none, virt: false }, &mut s);
        }
        return s;
    }
    let tr = (gi("flt") == 1).then(|| st_trend(x, tf_mwd(gi("flt_tf")), gi("flt_point"), g("flt_n").round().max(1.0) as usize, g("flt_f")));
    let (fb, fs) = filter_arrays(tr, n);
    let flip = |a: &[bool], b: &[bool]| -> Vec<bool> { (0..n).map(|i| i > 0 && a[i - 1] && b[i]).collect() };
    match id {
        TRAIL_V1 => {
            let fresh = gi("fresh") == 1;
            if side == 1 {
                let r = rsi_fill(c, rn, 1.0);
                let force = if fresh { flip(&fs, &fb) } else { fb.clone() };
                trail_seq_short(&TrailIn { r: &r, c, start: rn, ev, xv, thresh: f64::INFINITY, gate: &fs, force: &force, virt: fresh }, &mut s);
            } else {
                let r = rsi_fill(c, rn, 100.0);
                let force = if fresh { flip(&fb, &fs) } else { fs.clone() };
                trail_seq_long(&TrailIn { r: &r, c, start: rn, ev, xv, thresh: f64::INFINITY, gate: &fb, force: &force, virt: fresh }, &mut s);
            }
        }
        _ => {
            let r = rsi_fill(c, rn, 1.0);
            let mkt = if id == TRAIL_V3 { gi("mkt") } else { -1 };
            if side == 2 {
                let mode = match mkt {
                    0 => Ls::Bull,
                    1 => Ls::Bear,
                    2 => Ls::Both,
                    _ => Ls::V2,
                };
                trail_ls(&r, c, rn, ev, xv, &fb, &fs, mode, &mut s);
            } else {
                let long = side == 1;
                // Ver 2: short gated by the down filter and covered when it is up; long without filter.
                // Ver 3: bull gates on up and exits on down, bear the reverse, both no filter.
                let (gate, force) = match (mkt, long) {
                    (-1, false) | (1, _) => (&fs, &fb),
                    (0, _) => (&fb, &fs),
                    _ => (&all, &none),
                };
                trail_chain(&TrailIn { r: &r, c, start: rn, ev, xv, thresh: f64::INFINITY, gate, force, virt: false }, long, &mut s);
            }
        }
    }
    s
}

// ---------------------------------------------------------------- Lowest RSI trailing entry, ST fresh sell exit

#[derive(Clone, Copy, PartialEq)]
enum LowFlt {
    Plain,
    Swing,
    Athc,
}

fn run_lowest(x: &Ctx, g: &dyn Fn(&str) -> f64, kind: LowFlt) -> Sig {
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let n = c.len();
    let gi = |k: &str| g(k).round() as i64;
    let rn = g("rsi_n").round().max(1.0) as usize;
    let r = rsi_fill(c, rn, 100.0);
    let (ev, thresh) = (g("entry_val"), g("thresh"));
    let mt = st_trend(x, tf_hdwm(gi("st_tf")), gi("st_point"), g("st_n").round().max(1.0) as usize, g("st_f"));
    let (hsl_pct, buf) = (g("hsl_pct"), g("buffer_pct"));
    let st_entry = kind == LowFlt::Plain && gi("st_entry") == 1;
    let root = kind == LowFlt::Plain && gi("root") == 1;
    let on = kind != LowFlt::Plain && gi("flt_on") == 1;
    // MY_FILTER: 1 / -1 for the swing filter, 1 / 0 for ATHC, 1 when off
    let myf: Vec<i8> = match (kind, on) {
        (LowFlt::Swing, true) => swing_filter(c, g("bear_pct"), g("bull_pct")),
        (LowFlt::Athc, true) => athc(c, g("exit_pct")).into_iter().map(|a| a as i8).collect(),
        _ => vec![1; n],
    };
    let mut s = Sig::new(n);
    let (mut pos, mut skip, mut count1) = (false, false, 0i32);
    let (mut lowest, mut rsi_entry, mut hsl) = (0.0f64, BIG, 0.0f64);
    for i in 1..n {
        let (a, b) = (r[i], r[i - 1]);
        if !pos && a < b && count1 <= 1 {
            lowest = a;
            count1 = 1;
        } else if !pos && a > 0.0 && a > b && count1 == 1 {
            count1 = 2;
            rsi_entry = lowest + ev;
        } else if !pos && lowest > 0.0 && a < lowest && count1 == 2 {
            lowest = a;
            rsi_entry = lowest + ev;
        }
        let lc = l[i] == c[i];
        if !pos && a > rsi_entry && count1 == 2 && a < thresh && h[i] != c[i] && (!st_entry || mt[i] == 1) && myf[i] == 1 {
            s.buy[i] = Some(c[i]);
            pos = true;
            lowest = 0.0;
            count1 = 0;
            rsi_entry = BIG;
            if hsl_pct > 0.0 {
                hsl = (1.0 - hsl_pct * 0.01) * c[i];
            }
        } else if pos && !skip {
            let fresh = mt[i] == -1 && mt[i - 1] == 1;
            let flt_out = match kind {
                LowFlt::Swing => myf[i] == -1,
                LowFlt::Athc => myf[i] == 0,
                LowFlt::Plain => false,
            };
            let hit = l[i] <= hsl;
            if fresh || flt_out || hit {
                // the swing AFL also sells a filter exit on a Low = Close bar; the ATHC AFL tests MY_FILTER == -1 there, never true
                if !lc || (kind == LowFlt::Swing && myf[i] == -1) {
                    let px = if hit { if o[i] < hsl { o[i] - o[i] * buf / 100.0 } else { hsl } } else { c[i] };
                    s.sell[i] = Some((px, if hit { "Hard SL" } else if fresh { "ST fresh sell" } else { "Filter exit" }));
                    pos = false;
                } else if !root {
                    skip = true;
                }
            }
        } else if skip && !lc {
            s.sell[i] = Some((c[i], "Blocked exit, next bar"));
            pos = false;
            skip = false;
        }
    }
    s
}

// ---------------------------------------------------------------- RSI breach, fresh ST sell

fn run_breach(x: &Ctx, g: &dyn Fn(&str) -> f64) -> Sig {
    let (h, l, c) = (&x.h, &x.l, &x.c);
    let n = c.len();
    let gi = |k: &str| g(k).round() as i64;
    let r = rsi(c, g("rsi_n").round().max(1.0) as usize);
    let v = g("short_val");
    let strd = st_trend(x, tf_dwm(gi("st_tf")), gi("st_point"), g("st_n").round().max(1.0) as usize, g("st_f"));
    let ver = gi("filter");
    let mt = if ver == 1 { st_trend(x, tf_dwm(gi("flt_tf")), gi("flt_point"), g("flt_n").round().max(1.0) as usize, g("flt_f")) } else { vec![1; n] };
    let (ath, tr1) = if ver == 2 { (athc(c, g("exit_pct")), close_swing(c, g("exit_pct"))) } else { (vec![true; n], vec![1; n]) };
    let mut s = Sig::new(n);
    let (mut pos, mut skip) = (false, false);
    for i in 1..n {
        let lc = l[i] == c[i];
        let fresh = strd[i] == -1 && strd[i - 1] == 1;
        let cross = r[i] < v && r[i - 1] > v;
        let gate = match ver {
            1 => mt[i] == 1,
            2 => ath[i],
            _ => true,
        };
        if !pos && cross && gate && h[i] != c[i] {
            s.buy[i] = Some(c[i]);
            pos = true;
        } else if pos && (ver != 0 || !skip) {
            let (out, skip_on) = match ver {
                1 => (fresh || mt[i] == -1, fresh || mt[i] == -1),
                2 => (fresh || !ath[i], fresh || tr1[i] == -1),
                _ => (fresh, fresh),
            };
            if out && !lc {
                s.sell[i] = Some((c[i], if fresh { "ST fresh sell" } else { "Filter exit" }));
                pos = false;
            } else if skip_on && lc {
                skip = true;
            }
        }
        if skip && !lc {
            if pos {
                s.sell[i] = Some((c[i], "Blocked exit, next bar"));
            }
            pos = false;
            skip = false;
        }
    }
    s
}

// ---------------------------------------------------------------- ST buy-sell with an RSI dip

fn run_dip(x: &Ctx, g: &dyn Fn(&str) -> f64, from: usize) -> Sig {
    let n = x.c.len();
    let gi = |k: &str| g(k).round() as i64;
    let gu = |k: &str| g(k).round().max(1.0) as usize;
    let mut s = Sig::new(n);
    // no SetBarsRequired: the loops and indicators start the longest look-back before the run start
    let lead = gu("rsi_n").max(gu("e_n")).max(gu("x_n")).max(gu("flt_n"));
    let start = from.saturating_sub(lead);
    let sub = Ctx::new(&x.r[start..], &x.r[start..], &[]);
    let (h, l, c) = (&sub.h, &sub.l, &sub.c);
    let m = c.len();
    let r = rsi(c, gu("rsi_n"));
    let lvl = g("rsi_lvl");
    let t1 = st_trend(&sub, 'D', gi("st_point"), gu("e_n"), g("e_f"));
    let t2 = st_trend(&sub, 'D', gi("st_point"), gu("x_n"), g("x_f"));
    let fb: Vec<bool> = if gi("flt") == 1 { st_trend(&sub, tf_dwm(gi("flt_tf")), gi("flt_point"), gu("flt_n"), g("flt_f")).iter().map(|&v| v == 1).collect() } else { vec![true; m] };
    let (mut pos, mut armed) = (false, false);
    for i in 1..m {
        let lc = l[i] == c[i];
        if fb[i] {
            if r[i] < lvl && !armed {
                armed = true;
            }
            if armed && !pos && t1[i] == 1 && h[i] != c[i] {
                s.buy[i + start] = Some(c[i]);
                pos = true;
            } else if pos && t1[i] == -1 && t2[i] == -1 && !lc {
                s.sell[i + start] = Some((c[i], "Both STs down"));
                pos = false;
                armed = false;
            }
        } else if pos && !lc {
            s.sell[i + start] = Some((c[i], "Filter exit"));
            pos = false;
            armed = false;
        } else {
            armed = false;
        }
    }
    s
}

// ---------------------------------------------------------------- Aakash RSI

fn ak_filter(x: &Ctx, g: &dyn Fn(&str) -> f64) -> (Vec<bool>, Vec<bool>) {
    let gi = |k: &str| g(k).round() as i64;
    let tr = st_trend(x, tf_mwd(gi("flt_tf")), gi("flt_point"), g("flt_n").round().max(1.0) as usize, g("flt_f"));
    (tr.iter().map(|&v| v == 1).collect(), tr.iter().map(|&v| v == -1).collect())
}

/// Same-day versions: RSI cross at the close, RSI trailing exit. `bull`: the Bull AFL (long, filter up / above SMA).
fn run_ak_same(x: &Ctx, g: &dyn Fn(&str) -> f64, bull: bool) -> Sig {
    let c = &x.c;
    let n = c.len();
    let gi = |k: &str| g(k).round() as i64;
    let rn = g("rsi_n").round().max(1.0) as usize;
    let r = rsi(c, rn);
    let (fb, fs) = ak_filter(x, g);
    let ma = ma_nan(c, g("sma_n").round().max(1.0) as usize);
    let sma_flt = gi("flt") == 1;
    let short = !bull && gi("side") == 1;
    let gate = |i: usize| match (bull, sma_flt) {
        (true, false) => fb[i],
        (true, true) => c[i] > ma[i],
        (false, false) => fs[i],
        (false, true) => c[i] < ma[i],
    };
    let (bv, sv, trail) = (g("buy_val"), if bull { 0.0 } else { g("short_val") }, g("trail"));
    let mut s = Sig::new(n);
    let (mut pos, mut best, mut rt) = (0i32, 0.0f64, 0.0f64);
    for i in rn.max(1)..n {
        let (a, b) = (r[i], r[i - 1]);
        if !short {
            if pos == 0 && a > bv && b < bv && gate(i) {
                s.buy[i] = Some(c[i]);
                best = a;
                if trail > 0.0 {
                    rt = best - trail;
                }
                pos = 1;
            } else if pos > 0 {
                if trail > 0.0 {
                    best = a.max(best);
                    rt = best - trail;
                }
                if a < rt {
                    s.sell[i] = Some((c[i], "RSI trailing exit"));
                    pos = 0;
                    rt = 0.0;
                    best = 0.0;
                }
            }
        } else if pos == 0 && a < sv && b > sv && gate(i) {
            s.short[i] = Some(c[i]);
            best = a;
            if trail > 0.0 {
                rt = best + trail;
            }
            pos = -1;
        } else if pos < 0 {
            if trail > 0.0 {
                best = a.min(best);
                rt = best + trail;
            }
            if a > rt {
                s.cover[i] = Some((c[i], "RSI trailing exit"));
                pos = 0;
                rt = 0.0;
                best = 0.0;
            }
        }
    }
    s
}

/// Next-day versions: the RSI cross notes the signal bar's High (Low); the next bar(s) within Entry Period must break it.
fn run_ak_next(x: &Ctx, g: &dyn Fn(&str) -> f64, bull: bool) -> Sig {
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let n = c.len();
    let gi = |k: &str| g(k).round() as i64;
    let rn = g("rsi_n").round().max(1.0) as usize;
    let r = rsi(c, rn);
    let (fb, fs) = ak_filter(x, g);
    let ma = ma_nan(c, g("sma_n").round().max(1.0) as usize);
    // filter: bull AFL 0 ST / 1 SMA; bear AFL 0 none / 1 ST / 2 SMA / 3 RSI
    let flt = if bull { 1 + gi("flt").clamp(0, 1) } else { gi("flt") };
    let short = !bull && gi("side") == 1;
    let my_rsi: Vec<f64> = if !bull && flt == 3 {
        let b = compress(x, tf_hdwm(gi("rf_tf")));
        expand_last(&rsi(&b.c, g("rf_n").round().max(1.0) as usize), &b)
    } else {
        vec![f64::NAN; n]
    };
    let rf_bear = if bull { 0.0 } else { g("rf_bear") };
    let (bv, sv, trail, bs, ep) = (g("buy_val"), if bull { 0.0 } else { g("short_val") }, g("trail"), g("bs_pct"), gi("entry_period"));
    // (signal gate, entry gate, the unfilled-signal reset clears the count)
    let gates = |i: usize| -> (bool, bool, bool) {
        match (bull, flt, short) {
            (true, 1, _) => (fb[i], fb[i], true),
            (true, _, _) => (c[i] > ma[i], c[i] > ma[i], false),
            (false, 1, _) => (fs[i], fs[i], true),
            (false, 2, _) => (c[i] < ma[i], c[i] < ma[i], true),
            (false, 3, false) => (true, my_rsi[i] < rf_bear, true),
            (false, 3, true) => (fs[i], fs[i] && my_rsi[i] < rf_bear, true),
            _ => (true, true, true),
        }
    };
    let mut s = Sig::new(n);
    let (mut pos, mut abc, mut count) = (0i32, false, 0i64);
    let (mut myhigh, mut mylow, mut best, mut rt) = (0.0f64, 0.0f64, 0.0f64, 0.0f64);
    for i in rn.max(1)..n {
        let (a, b) = (r[i], r[i - 1]);
        let (sg, eg, reset_count) = gates(i);
        if !short {
            if a > bv && b < bv && sg && !abc {
                myhigh = h[i];
                abc = true;
            } else if abc {
                count += 1;
            }
            if pos == 0 && h[i] > myhigh && count > 0 && count <= ep && myhigh != 0.0 && abc && eg {
                s.buy[i] = Some(if o[i] > myhigh { o[i] + o[i] * bs / 100.0 } else { myhigh });
                best = a;
                if trail > 0.0 {
                    rt = best - trail;
                }
                pos = 1;
            } else if pos == 0 && count >= ep {
                myhigh = 0.0;
                mylow = 0.0;
                best = 0.0;
                rt = 0.0;
                abc = false;
                if reset_count {
                    count = 0;
                }
            } else if pos > 0 {
                if trail > 0.0 {
                    best = a.max(best);
                    rt = best - trail;
                }
                if a < rt {
                    s.sell[i] = Some((c[i], "RSI trailing exit"));
                    pos = 0;
                    myhigh = 0.0;
                    abc = false;
                    count = 0;
                    mylow = 0.0;
                    rt = 0.0;
                    best = 0.0;
                }
            }
        } else {
            if a < sv && b > sv && sg && !abc {
                mylow = l[i];
                abc = true;
            } else if abc {
                count += 1;
            }
            let sma = flt == 2;
            if pos == 0 && l[i] < mylow && mylow != 0.0 && (!sma || (count > 0 && count <= ep)) && abc && eg {
                s.short[i] = Some(if o[i] < mylow { o[i] - o[i] * bs / 100.0 } else { mylow });
                mylow = 0.0;
                abc = false;
                best = a;
                if trail > 0.0 {
                    rt = best + trail;
                }
                pos = -1;
            } else if pos == 0 && count >= ep {
                myhigh = 0.0;
                mylow = 0.0;
                best = 0.0;
                rt = 0.0;
                abc = false;
                count = 0;
            } else if pos < 0 {
                if trail > 0.0 {
                    best = a.min(best);
                    rt = best + trail;
                }
                if a > rt {
                    s.cover[i] = Some((c[i], "RSI trailing exit"));
                    pos = 0;
                    myhigh = 0.0;
                    abc = false;
                    mylow = 0.0;
                    rt = 0.0;
                    best = 0.0;
                    if sma {
                        count = 0;
                    }
                }
            }
        }
    }
    s
}

// ---------------------------------------------------------------- Connors RSI F&O

fn run_connors(x: &Ctx, g: &dyn Fn(&str) -> f64) -> Sig {
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let n = c.len();
    let gi = |k: &str| g(k).round() as i64;
    let gu = |k: &str| g(k).round().max(1.0) as usize;
    let (long, ver, atr_mode) = (gi("side") != 1, gi("exit_ver"), gi("stop") == 1);
    let mut s = Sig::new(n);
    // the AFL has blocks only for these pairs
    let ok = if long { matches!(ver, 0 | 1 | 3 | 5) } else { matches!(ver, 0 | 1 | 2 | 4) };
    if !ok {
        return s;
    }
    let sma = ma_nan(c, gu("sma_n"));
    let myma = ma_nan(c, gu("ma_n"));
    let r = rsi(c, gu("rsi_n"));
    let at = atr(x.r, gu("atr_n"));
    let prev = |v: Vec<f64>| -> Vec<f64> { (0..n).map(|i| if i == 0 { f64::NAN } else { v[i - 1] }).collect() };
    let higher_close = prev(hhv(c, gu("hc_n")));
    let lower_close = prev(llv(c, gu("lc_n")));
    let higher_high = prev(hhv(h, gu("hh_n")));
    let lower_low = prev(llv(l, gu("ll_n")));
    let (bv, sv, pt_pct, hsl_pct, apt, asl, buf, xd) = (g("buy_val"), g("short_val"), g("pt_pct"), g("hsl_pct"), g("atr_pt"), g("atr_sl"), g("buffer_pct") / 100.0, gi("exit_days"));
    let (mut pos, mut pt, mut sl, mut count) = (0i32, NUL, NUL, 1i64);
    for i in 1..n {
        if ver == 1 && pos != 0 {
            count += 1;
        }
        let d = if long { 1.0 } else { -1.0 };
        let entry = if long {
            c[i] > sma[i] && (ver != 0 || c[i] < myma[i]) && (ver != 1 || count == 1) && r[i] < bv && r[i - 1] > bv
        } else {
            c[i] < sma[i] && (ver != 0 || c[i] > myma[i]) && (ver != 1 || count == 1) && r[i] > sv && r[i - 1] < sv
        };
        if pos == 0 && entry && c[i] > 0.0 {
            if long {
                s.buy[i] = Some(c[i]);
            } else {
                s.short[i] = Some(c[i]);
            }
            if !atr_mode {
                if pt_pct > 0.0 {
                    pt = (1.0 + d * pt_pct * 0.01) * c[i];
                }
                if hsl_pct > 0.0 {
                    sl = (1.0 - d * hsl_pct * 0.01) * c[i];
                }
            } else {
                if apt > 0.0 {
                    pt = c[i] + d * at[i] * apt;
                }
                if asl > 0.0 {
                    sl = c[i] - d * at[i] * asl;
                }
            }
            pos = if long { 1 } else { -1 };
        } else if pos != 0 {
            // the loop compares the Null target / stop as -1e10
            let (pt_hit, sl_hit) = if long { (h[i] >= pt, l[i] <= sl) } else { (l[i] <= pt, h[i] >= sl) };
            let ma_out = if long { c[i] < sma[i] } else { c[i] > sma[i] };
            let (ver_out, ver_px): (bool, Option<f64>) = match ver {
                0 => (if long { c[i] > myma[i] } else { c[i] < myma[i] }, None),
                1 => (count > xd, None),
                2 => (c[i] > higher_close[i], None),
                3 => (c[i] < lower_close[i], None),
                4 => (h[i] >= higher_high[i], Some(higher_high[i])),
                _ => (l[i] <= lower_low[i], Some(lower_low[i])),
            };
            if ma_out || ver_out || pt_hit || sl_hit {
                let gap = |lvl: f64, above: bool| if (above && o[i] > lvl) || (!above && o[i] < lvl) { o[i] - d * o[i] * buf } else { lvl };
                let (px, why) = if pt_hit {
                    (gap(pt, long), "Profit target")
                } else if sl_hit {
                    (gap(sl, !long), "Hard SL")
                } else if let (true, Some(lvl)) = (ver_out, ver_px) {
                    (gap(lvl, !long), "Exit version level")
                } else {
                    (c[i], if ver_out { "Exit version" } else { "200-day MA" })
                };
                if long {
                    s.sell[i] = Some((px, why));
                } else {
                    s.cover[i] = Some((px, why));
                }
                pos = 0;
                pt = NUL;
                sl = NUL;
                if ver == 1 {
                    count = 1;
                }
            }
        }
    }
    s
}

// ---------------------------------------------------------------- %SL / ATR SL template (RSI, HL / LH)

#[derive(Clone, Copy, PartialEq)]
enum Ent {
    Cross,
    Delayed,
    HlLh,
}

fn run_slt(x: &Ctx, g: &dyn Fn(&str) -> f64, kind: Ent) -> Sig {
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let n = c.len();
    let gi = |k: &str| g(k).round() as i64;
    let gu = |k: &str| g(k).round().max(1.0) as usize;
    let ver = gi("ver").clamp(0, 5);
    let (atr_mode, filt) = (ver % 2 == 1, ver / 2);
    let r = rsi(c, gu("rsi_n"));
    let (bv, sv) = (g("buy_val"), g("sell_val"));
    let mt = if filt == 1 { st_trend(x, tf_dwm(gi("flt_tf")), gi("flt_point"), gu("flt_n"), g("flt_f")) } else { vec![1; n] };
    let ma = ma_nan(c, gu("ma_n"));
    let at = atr(x.r, gu("atr_n"));
    let (pt_pct, hsl_pct, tsl_pct, buf) = (g("pt_pct"), g("hsl_pct"), g("tsl_pct"), g("buffer_pct") / 100.0);
    let (apt, asl, atsl) = (g("atr_pt"), g("atr_sl"), g("atr_tsl"));
    let (lh, hl) = if kind == Ent::HlLh { (llv(h, g("lh_n").round().max(1.0) as usize), hhv(l, g("hl_n").round().max(1.0) as usize)) } else { (vec![], vec![]) };
    let (mut lh1, mut hl1): (Vec<Option<f64>>, Vec<Option<f64>>) = (vec![None; n], vec![None; n]);
    let (mut flag, mut flag1) = (false, false);
    let mut s = Sig::new(n);
    let (mut pos, mut skip) = (false, false);
    let (mut pt, mut fix, mut tsl, mut hc) = (BIG, 0.0f64, 0.0f64, 0.0f64);
    let mut atr_tsl = vec![0.0f64; n];
    let mut atrv = 0.0f64;
    for i in 1..n {
        if kind == Ent::HlLh {
            if r[i] < bv && !flag {
                flag = true;
                flag1 = false;
            } else if r[i] > sv && !flag1 {
                flag1 = true;
                flag = false;
            }
            if flag {
                lh1[i] = Some(lh[i]);
                hl1[i] = None;
            } else if flag1 {
                hl1[i] = Some(hl[i]);
                lh1[i] = None;
            } else {
                hl1[i] = hl1[i - 1];
                lh1[i] = lh1[i - 1];
            }
        }
        let sig_in = match kind {
            Ent::Cross => r[i] < bv && r[i - 1] > bv,
            Ent::Delayed => r[i] > bv && r[i - 1] < bv,
            Ent::HlLh => flag && lh1[i].map_or(false, |v| c[i] > v),
        };
        let sig_out = match kind {
            Ent::Cross => r[i] > sv && r[i - 1] < sv,
            Ent::Delayed => r[i] < sv && r[i - 1] > sv,
            Ent::HlLh => hl1[i].map_or(false, |v| c[i] < v),
        };
        let (f_in, f_out) = match filt {
            1 => (mt[i] == 1, mt[i] == -1),
            2 => (c[i] > ma[i], c[i] < ma[i]),
            _ => (true, false),
        };
        let lc = l[i] == c[i];
        if !pos && sig_in && h[i] != c[i] && !lc && f_in {
            s.buy[i] = Some(c[i]);
            if !atr_mode {
                if pt_pct > 0.0 {
                    pt = (1.0 + pt_pct * 0.01) * c[i];
                }
                if hsl_pct > 0.0 {
                    fix = (1.0 - hsl_pct * 0.01) * c[i];
                }
                if tsl_pct > 0.0 {
                    hc = c[i];
                    tsl = (1.0 - tsl_pct * 0.01) * hc;
                }
            } else {
                if apt > 0.0 {
                    pt = c[i] + at[i] * apt;
                }
                if asl > 0.0 {
                    fix = c[i] - at[i] * asl;
                }
                if atsl > 0.0 {
                    hc = c[i];
                    atr_tsl[i] = hc - at[i] * atsl;
                }
            }
            pos = true;
        } else if pos && !skip {
            if !atr_mode && tsl_pct > 0.0 {
                hc = c[i].max(hc);
            }
            if atr_mode && atsl > 0.0 {
                hc = c[i].max(hc);
                if c[i] == hc {
                    atrv = at[i];
                }
            }
            let trail_hit = if atr_mode { c[i] <= atr_tsl[i - 1] } else { c[i] <= tsl };
            if h[i] >= pt || c[i] <= fix || trail_hit || sig_out || f_out {
                if !lc {
                    let px = if h[i] >= pt { if o[i] > pt { o[i] - o[i] * buf } else { pt } } else { c[i] };
                    let why = if h[i] >= pt { "Profit target" } else if c[i] <= fix { "Hard SL" } else if trail_hit { "Trailing SL" } else if sig_out { "Exit signal" } else { "Filter exit" };
                    s.sell[i] = Some((px, why));
                    pt = BIG;
                    hc = 0.0;
                    tsl = 0.0;
                    fix = 0.0;
                    atrv = 0.0;
                    atr_tsl[i] = 0.0;
                    pos = false;
                    flag = false;
                } else {
                    skip = true;
                }
            }
            if !atr_mode && tsl_pct > 0.0 {
                tsl = (1.0 - tsl_pct * 0.01) * hc;
            }
            if atr_mode && atsl > 0.0 {
                atr_tsl[i] = if atrv > 0.0 { hc - atrv * atsl } else { atr_tsl[i - 1] };
                if atr_tsl[i] < atr_tsl[i - 1] {
                    atr_tsl[i] = atr_tsl[i - 1];
                }
            }
        } else if pos && skip && !(prec2(o[i]) == prec2(c[i]) && prec2(o[i]) == prec2(l[i])) {
            let b = o[i] - o[i] * buf;
            let px = if o[i] == l[i] { o[i] } else if l[i] < b { b } else if l[i] > b { l[i] } else { c[i] };
            s.sell[i] = Some((px, "Blocked exit, next bar"));
            pos = false;
            skip = false;
            hc = 0.0;
            tsl = 0.0;
            fix = 0.0;
            atrv = 0.0;
            atr_tsl[i] = 0.0;
            pt = BIG;
            flag = false;
        }
    }
    s
}

// ---------------------------------------------------------------- Overbought / oversold delayed, exit versions

fn run_obos(x: &Ctx, g: &dyn Fn(&str) -> f64) -> Sig {
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let n = c.len();
    let gi = |k: &str| g(k).round() as i64;
    let gu = |k: &str| g(k).round().max(1.0) as usize;
    let ver = gi("ver").clamp(0, 4);
    let r = rsi(c, gu("rsi_n"));
    let (bv, sv, hsl_pct, atr_m, buf) = (g("buy_val"), g("sell_val"), g("hsl_pct"), g("atr_m"), g("buffer_pct") / 100.0);
    let mt = if ver == 3 { st_trend(x, tf_dwm(gi("flt_tf")), gi("flt_point"), gu("flt_n"), g("flt_f")) } else { vec![1; n] };
    let ma = ma_nan(c, gu("ma_n"));
    let at = atr(x.r, gu("atr_n"));
    let mut s = Sig::new(n);
    let (mut pos, mut skip, mut fix) = (false, false, 0.0f64);
    for i in 1..n {
        let f_in = match ver {
            3 => mt[i] == 1,
            4 => c[i] > ma[i],
            _ => true,
        };
        if !pos && r[i] > bv && r[i - 1] < bv && h[i] != c[i] && f_in {
            s.buy[i] = Some(c[i]);
            if ver == 1 && hsl_pct > 0.0 {
                fix = (1.0 - hsl_pct * 0.01) * c[i];
            }
            if ver == 2 && atr_m > 0.0 {
                fix = c[i] - at[i] * atr_m;
            }
            pos = true;
            continue;
        }
        let rsi_out = r[i] < sv && r[i - 1] > sv;
        let other = match ver {
            1 | 2 => c[i] <= fix,
            3 => mt[i] == -1,
            4 => c[i] < ma[i],
            _ => false,
        };
        if pos && (rsi_out || other) && !skip {
            if l[i] != c[i] {
                s.sell[i] = Some((c[i], if rsi_out { "RSI crosses below the sell value" } else { "Exit version" }));
                pos = false;
                skip = false;
                fix = 0.0;
            } else {
                skip = true;
            }
        } else if pos && skip && !(prec2(o[i]) == prec2(c[i]) && prec2(o[i]) == prec2(l[i])) {
            s.sell[i] = Some((o[i] - o[i] * buf, "Blocked exit, next bar"));
            pos = false;
            skip = false;
            fix = 0.0;
        }
    }
    s
}

// ---------------------------------------------------------------- run

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(0.0, |d| d.def));
    let n = x.c.len();
    if n == 0 || from > to || to >= n {
        return Some(Vec::new());
    }
    let s = match id {
        CROSS => {
            let r = rsi(&x.c, g("rsi_n").round().max(1.0) as usize);
            run_cross(&r, &x.h, &x.l, &x.c, g("buy_val"), g("sell_val"), g("exit_days").round() as i64)
        }
        TRAIL | TRAIL_V1 | TRAIL_V2 | TRAIL_V3 => run_trail(id, x, &g),
        LOWEST => run_lowest(x, &g, LowFlt::Plain),
        LOWEST_SW => run_lowest(x, &g, LowFlt::Swing),
        LOWEST_ATHC => run_lowest(x, &g, LowFlt::Athc),
        BREACH => run_breach(x, &g),
        DIP => run_dip(x, &g, from),
        AK_BULL => run_ak_same(x, &g, true),
        AK_BEAR => run_ak_same(x, &g, false),
        AK_BULL_N => run_ak_next(x, &g, true),
        AK_BEAR_N => run_ak_next(x, &g, false),
        CONNORS => run_connors(x, &g),
        SLT => run_slt(x, &g, if g("delayed").round() as i64 == 1 { Ent::Delayed } else { Ent::Cross }),
        HLLH => run_slt(x, &g, Ent::HlLh),
        OBOS => run_obos(x, &g),
        _ => return None,
    };
    Some(backtest(x, &s, from, to))
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let d = catalogue().into_iter().find(|d| d.id == id)?;
    Some((d.id, d.name, "RSI"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::books::ytwh::testbars::*;

    fn flat_bars(n: usize) -> Vec<crate::Row> {
        // quiet bars with High > Close > Low so no circuit / UC / LC guard blocks anything
        bars(&(0..n).map(|k| { let c = 100.0 + (k % 3) as f64; (c, c + 1.0, c - 1.0, c) }).collect::<Vec<_>>())
    }

    #[test]
    fn catalogue_and_params_cover_every_id() {
        let cat = catalogue();
        assert_eq!(cat.len(), IDS.len());
        for id in IDS {
            assert!(cat.iter().any(|d| d.id == id && d.group == GROUP && d.id.starts_with("ytrsi_")), "{id}");
            assert!(params(id).is_some(), "{id}");
            assert_eq!(family_of(id).unwrap().2, "RSI");
            for d in params(id).unwrap() {
                assert!(d.min <= d.def && d.def <= d.max, "{id} {}", d.key);
            }
        }
    }

    #[test]
    fn cross_30_40_entry_and_exits() {
        let r = flat_bars(12);
        let x = Ctx::new(&r, &r, &[]);
        let mut rs = vec![50.0; 12];
        rs[3] = 25.0; // crosses below 30: buy at bar 3
        rs[5] = 45.0; // crosses above 40 (bar 4 is 50 -> no: needs yesterday < 40)
        rs[4] = 35.0;
        // exit days 0 (the AFL default): never exits
        let s = run_cross(&rs, &x.h, &x.l, &x.c, 30.0, 40.0, 0);
        let t = backtest(&x, &s, 0, 11);
        assert_eq!((t.len(), t[0].legs[0].i, t[0].open), (1, 3, true));
        // exit days 5: the RSI cross above 40 at bar 5 sells
        let t = backtest(&x, &run_cross(&rs, &x.h, &x.l, &x.c, 30.0, 40.0, 5), 0, 11);
        assert_eq!((t[0].legs[0].i, t[0].exit_i), (3, 5));
        // no RSI exit: the 3rd bar in the trade sells
        let mut rs2 = rs.clone();
        rs2[5] = 38.0;
        let t = backtest(&x, &run_cross(&rs2, &x.h, &x.l, &x.c, 30.0, 40.0, 3), 0, 11);
        assert_eq!(t[0].exit_i, 6);
    }

    #[test]
    fn trail_long_trackers() {
        let r = flat_bars(14);
        let x = Ctx::new(&r, &r, &[]);
        // filled 100 on bar 0; falls to 30 (lowest), rises 35 (entry level 50), 55 -> buy; 60, 70 high, 65 (exit level 50), 49 -> sell
        let rs = [100.0, 40.0, 30.0, 35.0, 45.0, 55.0, 60.0, 70.0, 65.0, 60.0, 49.0, 45.0, 44.0, 43.0];
        let all = vec![true; 14];
        let none = vec![false; 14];
        let mut s = Sig::new(14);
        trail_seq_long(&TrailIn { r: &rs, c: &x.c, start: 1, ev: 20.0, xv: 20.0, thresh: 50.0, gate: &all, force: &none, virt: false }, &mut s);
        // RSI 55 is not below the threshold 50: no entry
        assert!(s.buy.iter().all(|b| b.is_none()));
        let mut s = Sig::new(14);
        trail_seq_long(&TrailIn { r: &rs, c: &x.c, start: 1, ev: 20.0, xv: 20.0, thresh: f64::INFINITY, gate: &all, force: &none, virt: false }, &mut s);
        let t = backtest(&x, &s, 0, 13);
        assert_eq!((t[0].legs[0].i, t[0].exit_i), (5, 10));
        // the fresh-trade version with the filter down at the signal: virtual only, no trade
        let mut s = Sig::new(14);
        trail_seq_long(&TrailIn { r: &rs, c: &x.c, start: 1, ev: 20.0, xv: 20.0, thresh: f64::INFINITY, gate: &none, force: &none, virt: true }, &mut s);
        assert!(backtest(&x, &s, 0, 13).is_empty());
        // running version: a down filter on bar 7 sells
        let mut force = none.clone();
        force[7] = true;
        let mut s = Sig::new(14);
        trail_seq_long(&TrailIn { r: &rs, c: &x.c, start: 1, ev: 20.0, xv: 20.0, thresh: f64::INFINITY, gate: &all, force: &force, virt: false }, &mut s);
        assert_eq!(backtest(&x, &s, 0, 13)[0].exit_i, 7);
    }

    #[test]
    fn trail_short_and_chain() {
        let r = flat_bars(12);
        let x = Ctx::new(&r, &r, &[]);
        // rises to 80 (highest), falls 75 (entry level 60), 55 -> short; 50, 40 low, 45 (cover level 60), 61 -> cover
        let rs = [1.0, 70.0, 80.0, 75.0, 55.0, 50.0, 40.0, 45.0, 61.0, 62.0, 63.0, 64.0];
        let all = vec![true; 12];
        let none = vec![false; 12];
        let mut s = Sig::new(12);
        trail_seq_short(&TrailIn { r: &rs, c: &x.c, start: 1, ev: 20.0, xv: 20.0, thresh: f64::INFINITY, gate: &all, force: &none, virt: false }, &mut s);
        let t = backtest(&x, &s, 0, 11);
        assert_eq!((t[0].dir, t[0].legs[0].i, t[0].exit_i), (-1.0, 4, 8));
        // chain version: same here (no tracker step fires on the entry bar)
        let mut s = Sig::new(12);
        trail_chain(&TrailIn { r: &rs, c: &x.c, start: 1, ev: 20.0, xv: 20.0, thresh: f64::INFINITY, gate: &all, force: &none, virt: false }, false, &mut s);
        let t = backtest(&x, &s, 0, 11);
        assert_eq!((t[0].legs[0].i, t[0].exit_i), (4, 8));
    }

    #[test]
    fn trail_long_short_reverses() {
        let r = flat_bars(14);
        let x = Ctx::new(&r, &r, &[]);
        let rs = [1.0, 40.0, 30.0, 35.0, 55.0, 60.0, 70.0, 65.0, 49.0, 45.0, 44.0, 43.0, 42.0, 41.0];
        let (fb, fs) = (vec![true; 14], vec![false; 14]);
        let mut s = Sig::new(14);
        trail_ls(&rs, &x.c, 1, 20.0, 20.0, &fb, &fs, Ls::Both, &mut s);
        let t = backtest(&x, &s, 0, 13);
        // buy signal at 55 (lowest 30 + 20); the high tracker's level 70 - 20 = 50 breaks at 49 -> short signal: sell and short
        assert_eq!((t[0].dir, t[0].legs[0].i, t[0].exit_i), (1.0, 4, 8));
        assert_eq!((t[1].dir, t[1].legs[0].i), (-1.0, 8));
        // bull market with the filter up: the short is not allowed (gate = up) -> allowed; bear: no buy
        let mut s = Sig::new(14);
        trail_ls(&rs, &x.c, 1, 20.0, 20.0, &fb, &fs, Ls::Bear, &mut s);
        assert!(backtest(&x, &s, 0, 13).is_empty());
    }

    #[test]
    fn filters_and_swings() {
        let c: Vec<f64> = vec![100.0, 101.0, 102.0, 103.0, 80.0, 70.0, 60.0, 59.0, 104.0];
        // ATHC: on at a close above the prior all-time high, off once the 20% swing turns down
        let a = athc(&c, 20.0);
        assert!(a[1] && a[3]);
        assert!(!a[5] && !a[7]);
        assert!(a[8]);
        // swing filter: down below 23% under the ratcheted band, up again on a new all-time-high close
        let tr = swing_filter(&c, 23.0, 44.0);
        assert_eq!(tr[3], 1);
        assert_eq!(tr[4], 1); // 80 > 103 x 0.77 = 79.31
        assert_eq!(tr[5], -1);
        assert_eq!(tr[8], 1);
    }

    #[test]
    fn lowest_rsi_buys_on_trailing_rise_and_sells_on_st_flip() {
        let r = wave(300);
        let x = Ctx::new(&r, &r, &[]);
        let t = run(LOWEST, &x, 0, 299, &Params::new()).unwrap();
        assert!(!t.is_empty());
        let mt = st_trend(&x, 'D', 1, 5, 1.0);
        let rs = rsi_fill(&x.c, 14, 100.0);
        for tr in &t {
            assert!(rs[tr.legs[0].i] < 80.0);
            if !tr.open {
                assert!(mt[tr.exit_i] == -1 && mt[tr.exit_i - 1] == 1, "exit on a fresh ST sell");
            }
        }
        // a 1% hard SL: at the SL level, or below it on a gap open
        let t2 = run(LOWEST, &x, 0, 299, &ps(&[("hsl_pct", 1.0)])).unwrap();
        for tr in t2.iter().filter(|t| t.reason == "Hard SL") {
            assert!(tr.exit_px <= tr.legs[0].px * 0.99 + 1e-9);
        }
    }

    #[test]
    fn breach_and_aakash_entries() {
        let r = wave(400);
        let x = Ctx::new(&r, &r, &[]);
        let rs = rsi(&x.c, 14);
        for tr in run(BREACH, &x, 0, 399, &Params::new()).unwrap() {
            let e = tr.legs[0].i;
            assert!(rs[e] < 40.0 && rs[e - 1] > 40.0);
        }
        let ak = run(AK_BEAR, &x, 0, 399, &ps(&[("flt_tf", 2.0)])).unwrap();
        let st = st_trend(&x, 'D', 1, 5, 1.0);
        for tr in &ak {
            let e = tr.legs[0].i;
            assert!(rs[e] > 60.0 && rs[e - 1] < 60.0 && st[e] == -1);
        }
        for tr in run(AK_BULL_N, &x, 0, 399, &ps(&[("flt_tf", 2.0)])).unwrap() {
            let e = tr.legs[0].i;
            assert!(rs[e - 1] > 60.0 && rs[e - 2] < 60.0, "next-day entry after the signal bar");
            assert!(tr.legs[0].px >= x.h[e - 1] - 1e-9);
        }
    }

    #[test]
    fn connors_targets_and_null_levels() {
        // a bar series where RSI(2) crosses below 5 above the 200-day MA is hard to build; check the stop logic on the template instead
        let r = wave(400);
        let x = Ctx::new(&r, &r, &[]);
        for kv in [vec![], vec![("side", 1.0)], vec![("exit_ver", 1.0)], vec![("stop", 1.0)], vec![("sma_n", 5.0), ("pt_pct", 0.0)]] {
            for t in run(CONNORS, &x, 0, 399, &ps(&kv)).unwrap() {
                if kv.iter().any(|(k, _)| *k == "pt_pct") && !t.open {
                    assert_eq!(t.exit_i, t.legs[0].i + 1, "a 0 target is Null = -1e10: next-bar exit");
                }
            }
        }
        // a pair without an AFL block
        assert!(run(CONNORS, &x, 0, 399, &ps(&[("exit_ver", 2.0)])).unwrap().is_empty());
    }

    #[test]
    fn st_ma_sl_template_exits() {
        let r = wave(400);
        let x = Ctx::new(&r, &r, &[]);
        let rs = rsi(&x.c, 14);
        for (delayed, t) in [(false, run(SLT, &x, 0, 399, &Params::new()).unwrap()), (true, run(SLT, &x, 0, 399, &ps(&[("delayed", 1.0)])).unwrap())] {
            for tr in &t {
                let e = tr.legs[0].i;
                if delayed {
                    assert!(rs[e] > 30.0 && rs[e - 1] < 30.0);
                } else {
                    assert!(rs[e] < 30.0 && rs[e - 1] > 30.0);
                }
                if !tr.open && tr.reason == "Profit target" {
                    assert!(tr.exit_px >= tr.legs[0].px * 1.1 - 1e-9 || x.o[tr.exit_i] > tr.legs[0].px * 1.1);
                }
            }
        }
    }

    #[test]
    fn every_strategy_runs() {
        smoke(run, &IDS);
        let r = wave(400);
        let x = Ctx::new(&r, &r, &[]);
        let variants: Vec<(&str, Vec<(&str, f64)>)> = vec![
            (CROSS, vec![("exit_days", 5.0)]),
            (TRAIL, vec![("side", 1.0)]),
            (TRAIL_V1, vec![("fresh", 1.0)]),
            (TRAIL_V1, vec![("side", 1.0), ("fresh", 1.0), ("flt_tf", 2.0)]),
            (TRAIL_V2, vec![("side", 2.0), ("flt_tf", 2.0)]),
            (TRAIL_V2, vec![("side", 1.0)]),
            (TRAIL_V3, vec![("side", 2.0), ("mkt", 1.0), ("flt_tf", 1.0)]),
            (TRAIL_V3, vec![("side", 1.0), ("mkt", 2.0), ("flt_point", 6.0)]),
            (LOWEST, vec![("st_tf", 3.0), ("st_entry", 1.0), ("hsl_pct", 1.0), ("root", 1.0)]),
            (LOWEST_SW, vec![("hsl_pct", 1.0)]),
            (LOWEST_ATHC, vec![("flt_on", 0.0)]),
            (BREACH, vec![("filter", 1.0)]),
            (BREACH, vec![("filter", 2.0)]),
            (DIP, vec![("flt", 1.0), ("flt_tf", 1.0)]),
            (AK_BULL, vec![("flt", 1.0), ("sma_n", 20.0)]),
            (AK_BEAR, vec![("side", 1.0), ("flt_tf", 2.0)]),
            (AK_BEAR_N, vec![("side", 1.0), ("flt", 3.0), ("rf_tf", 2.0)]),
            (AK_BEAR_N, vec![("flt", 2.0), ("sma_n", 20.0)]),
            (CONNORS, vec![("side", 1.0), ("exit_ver", 4.0), ("stop", 1.0)]),
            (CONNORS, vec![("exit_ver", 5.0), ("sma_n", 20.0)]),
            (SLT, vec![("ver", 3.0)]),
            (SLT, vec![("ver", 5.0), ("ma_n", 20.0)]),
            (HLLH, vec![("ver", 1.0)]),
            (OBOS, vec![("ver", 3.0)]),
            (OBOS, vec![("ver", 2.0), ("atr_m", 2.0)]),
        ];
        for (id, kv) in variants {
            for (from, to) in [(0, 399), (60, 300)] {
                for t in run(id, &x, from, to, &ps(&kv)).unwrap() {
                    assert!(t.legs[0].i >= from && t.exit_i <= to && t.exit_i >= t.legs[0].i, "{id}");
                    assert!(t.exit_px >= x.l[t.exit_i] - 1e-9 && t.exit_px <= x.h[t.exit_i] + 1e-9, "{id}");
                }
            }
        }
    }
}
