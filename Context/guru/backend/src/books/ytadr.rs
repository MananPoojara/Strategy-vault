//! YT Strategies · Advance Decline Ratio: the team's AmiBroker AFLs in "Old Topics AFL/Advance Decline Ratio". Every
//! AFL trades one index (the `tickerName` list NIFTY | BANKNIFTY | FINNIFTY | MIDCPNIFTY | SENSEX, default NIFTY, set
//! with SetForeign or as the chart symbol) from the market breadth ratio AR = advancing / declining issues of the day.
//! Coded from the AFLs as written, with each AFL's default settings (an `Optimize()` uses its default); there is no
//! book PDF. ytadr_nday_exit is compared (17-11-2026) with the team's Best Setting Tradelist.xlsx: 743 of 744 trades equal
//! on Nifty 50 breadth; the other ids are not compared.
//!
//! Data: the AFLs read AmiBroker's composite tickers `Foreign("~Advances","C")` and `Foreign("~Declines","C")`. The
//! team's composites live in its Nifty 50 C&P database (`~ADVANCES`, `~NOCHANGE`, 03-01-2000 to 06-08-2024; the copy
//! has no `~DECLINES`) and count the Nifty 50 members (AUX1 = 1) whose close is above / below their previous close.
//! The counts are rebuilt (`jay-local/Data/breadth/build_breadth.py`) over those members as ADVANCES_N50 /
//! DECLINES_N50 (parameter `breadth` 2, the default): the rebuild equals the team's ~ADVANCES on 6,065 of 6,117 days
//! and is within 1 on 6,114 (the database was edited after the composite was calculated), and it runs to the end of
//! the database. `breadth` 0 uses the dated NSE 500 members of the NSE 500 C&P database (ADVANCES / DECLINES) and 1
//! every stock of that database (ADVANCES_ALL / DECLINES_ALL); both are wider universes than the team's composite, so
//! the AFLs' thresholds mean something else on them. Without the series the strategies give no trades and readiness
//! reports them as missing (needs_series).
//!
//! AmiBroker behaviour followed:
//! - Foreign's default fix-up: a traded-index day without a breadth value repeats the previous value (the readiness
//!   check already withholds periods with such gaps); AR = A / D, or 50 when D = 0 (the base AFL uses A when D = 0);
//! - the AFLs run their own bar loop from bar 1 over the whole history and write Buy / Sell (Short / Cover) with their
//!   prices; AmiBroker's backtester then reads those arrays (`ytroc::backtest`): entries only in the test window, an
//!   exit first and then an entry on one bar, a Sell while flat or a Buy while long is ignored, exit prices kept inside
//!   the bar's range, and a position still open at the window's end (or closed by the AFL's own last-bar exit) marked
//!   at that close. Where an AFL enters and exits on one bar (the 2-11 / 5DP AFLs test their exits on the entry bar)
//!   the backtester keeps the position and closes it on the AFL's next Sell, as AmiBroker does;
//! - inside a loop a Null element is the number -1e10; a Null in a comparison of arrays, and `Ref()` beyond the first
//!   or last bar (so the month-end test on the last data bar), count as false;
//! - the ADR AFLs' ST filter is their own loop: mean of the last `period` ATR(1) values (ATR(1) Null on bar 0), bands
//!   from the chosen price average, the band ratchet with no reset on a flip, ST bullish = Close above the plotted
//!   line. On a weekly / monthly frame the AFL takes the previous period's line and close, expands them with
//!   expandFirst and shifts them one bar forward, so a period's own values appear on its last day;
//! - the 5DP AFL's filters are the team's template loops: `ytsw::st_filter` (expandLast), the close % swing
//!   (`ytsw::close_swing`) with the ATHC loop (`ytnr7::athc`), and the bull / bear % swing written out below;
//! - RSI is AmiBroker's Wilder RSI (`yteng::rsi`), ADX AmiBroker's ADX (`connors::dmi`), ROC in percent;
//! - "month end" is `Month() != Ref(Month(), 1)`: the 5DP entries `Ref(MonthEnd, EntryDay - 1)` look ahead to the
//!   month's last trading day (in the data that is a calendar fact), as the AFLs do.
//!
//! Not coded: position sizing, MaxOpenPositions, commission and the ranking (PositionScore) of the 5DP AFL, which only
//! choose between symbols in a portfolio backtest; plotting and exploration columns. The Short branch of the
//! "Without RSI", "Two SuperTrend" and "Exit by N days" AFLs reads an undefined variable `Close0` and stops with an
//! AmiBroker error, so those ids are long only.
#![allow(dead_code)]
use crate::books::yteng::rsi as rsi_afl;
use crate::books::ytnr7::{athc, template_params, volsig17, Guard};
use crate::books::ytroc::{backtest, Px, LAST_BAR};
use crate::books::ytsw::{close_swing, compress, st_filter, true_range};
use crate::books::ytwh::bound;
use crate::connors::{dmi, Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "yt_adr";
pub const BASIC: &str = "ytadr_basic";
pub const VERSIONS: &str = "ytadr_versions";
pub const FIVE_EXIT: &str = "ytadr_5day_exit";
pub const NO_RSI: &str = "ytadr_no_rsi";
pub const TWO_ST: &str = "ytadr_two_st";
pub const NDAY: &str = "ytadr_nday_exit";
pub const P211: &str = "ytadr_211_5dp";
pub const P211_ROC: &str = "ytadr_211_5dp_roc";
pub const FDP: &str = "ytadr_5dp";
pub const IDS: [&str; 9] = [BASIC, VERSIONS, FIVE_EXIT, NO_RSI, TWO_ST, NDAY, P211, P211_ROC, FDP];

/// Breadth series by the `breadth` parameter: 0 dated NSE 500 members (AUX1), 1 all symbols of the NSE 500 C&P
/// database, 2 (default) Nifty 50 members (AUX1 of the Nifty 50 C&P database, the team's ~Advances universe).
pub const SERIES: [[&str; 2]; 3] = [["ADVANCES", "DECLINES"], ["ADVANCES_ALL", "DECLINES_ALL"], ["ADVANCES_N50", "DECLINES_N50"]];
/// Default `breadth` value (Nifty 50).
pub const DEFAULT_BREADTH: usize = 2;

/// Series index for a `breadth` value (out-of-range values fall back to the default).
pub fn breadth_index(v: Option<f64>) -> usize {
    match v.map(|v| v.round()) {
        Some(x) if x == 0.0 => 0,
        Some(x) if x == 1.0 => 1,
        _ => DEFAULT_BREADTH,
    }
}

const NULL: f64 = f64::NAN; // AFL Null in a loop: every comparison with it is false (SV-Audit)
const BIG: f64 = 9_999_999.0;

fn def(id: &'static str, name: &'static str, dir: &'static str, rules: &'static [&'static str], notes: &'static str) -> StrategyDef {
    StrategyDef { id, name, group: "YT Strategies · Advance Decline Ratio", session: 0, dir, status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &["ADVANCES_N50", "DECLINES_N50"], book: BOOK, rules, notes, claim: None }
}

const F_AR: &str = "F|AR = advancing / declining Nifty 50 issues of the day (50 when none decline); breadth 0 NSE 500, 1 all symbols";
const F_ENTRY_N: &str = "F|Consecutive on: AR at or below the entry threshold (0) on at least N (1) bars in a row; off: at least N of the last 'look back' (1) bars (a look back of 2 is read as 1, as written)";
const F_AVG: &str = "F|Average ADR filter (off): use the mean AR of the last N bars against its own threshold instead";
const F_PREV: &str = "F|Previous N candles filter (off): the previous bar ends N bars in a row of the chosen colour";
const F_ST: &str = "F|ST filter (off): the AFL's Supertrend (5, 1, (H+L)/2, daily) is bullish (Close above its line), or bearish when chosen";
const X_ST: &str = "X|ST filter on: the ST turning against the entry setting also exits";
const X_STOPS: &str = "X|PB % / HSL % (0 = off): on the High / Low (live, default) or the Close; live fills at the level, or at Open minus the buffer when the bar opens through it (not below the Low)";
const X_LAST: &str = "X|The AFL also exits on the last data bar (shown as an open trade)";
macro_rules! note_common { () => { " Trades the chosen index at its daily close (the AFL's SetForeign ticker list: NIFTY 50 by default, BANKNIFTY). The breadth series is rebuilt over the Nifty 50 members (AUX1) of the team's Nifty 50 C&P database, the universe of the team's ~Advances composite (equal on 6,065 of 6,117 days, within 1 on 6,114); breadth 0 = NSE 500 members, 1 = all NSE 500 C&P symbols. Position size, brokerage 0.04 % and penny / turnover / dead-stock settings are portfolio or stock-template options and are not modelled." }; }

pub fn catalogue() -> Vec<StrategyDef> {
    vec![
        def(BASIC, "ADR Threshold, N Candle Exit", "Long/Short", &[
            "F|AR = advancing / declining Nifty 50 issues of the day (the advancing count when none decline); breadth 0 NSE 500, 1 all symbols",
            "E|Long: AR at or below the lower threshold (0); short (Entry Type) : AR at or above the upper threshold (0); at the close",
            "X|N Candle exit: N (0 = off) bars after entry, at the close; with the 'AD Ratio' exit type there is no exit rule and an N of 0 exits on the next bar (as written)",
            "X|No ST, stop or last-bar exit",
        ], concat!("Advance Decline LongShort.afl, coded as the AFL is written (its SetForeign is commented out, so it trades the chart index).", " With the defaults (thresholds 0, N 0) a long needs a day with no advancing issue and never exits; a short enters on the first bar and never exits: set the thresholds and N.", " Brokerage 0.04 % and position sizing are portfolio settings and are not modelled. The breadth series is rebuilt over the Nifty 50 members of the team's Nifty 50 C&P database (the ~Advances universe).")),
        def(VERSIONS, "ADR Threshold with ST / RSI / Avg ADR Filters", "Long/Short", &[
            F_AR, "E|Long: AR at or below the threshold; short (Entry Type): AR at or above it; at the close", F_ENTRY_N, F_AVG, F_PREV, F_ST,
            "F|RSI filter (off): RSI(14) below (or above) the threshold (1)",
            "X|N Candles: N (0 = off) bars after entry; or AD Ratio: AR at or above (long) / at or below (short) the exit threshold; at the close",
            X_ST, X_STOPS, "X|As written the stop tests use the long formulas for shorts too (Low <= HSL, High >= PB)", X_LAST,
        ], concat!("Advance Decline LongShort_Versions.afl, coded as the AFL is written (exit tested before entry on each bar, so a new trade can start on the exit bar).", note_common!())),
        def(FIVE_EXIT, "ADR Threshold with 5DP-Day Exit", "Long/Short", &[
            F_AR, "E|Long: AR at or below the threshold; short (Entry Type): AR at or above it; at the close", F_ENTRY_N, F_AVG, F_PREV, F_ST,
            "F|RSI filter (off): RSI(14) below (or above) the threshold (1)",
            "X|N Candles (N 0 = off), AD Ratio threshold, or '5DP Entry': the bar N - 1 bars before a month's last trading day (N 0: the first trading day of the next month); at the close",
            X_ST, X_STOPS, X_LAST,
        ], concat!("Advance Decline LongShort_with5DayExit.afl, coded as the AFL is written; the 5DP exit looks ahead to the month's last trading day as the AFL does.", note_common!())),
        def(NO_RSI, "ADR Threshold, Candle-Count Exits", "Long", &[
            F_AR, "E|Long: AR at or below the threshold, at the close", F_ENTRY_N, F_AVG, F_PREV, F_ST,
            "X|From the bar after entry: N Candles (N, 0 = off), AD Ratio at or above the exit threshold, or the Nth green candle (Open < Close) / up close (Close > previous Close) after entry; at the close",
            X_ST, X_STOPS, X_LAST,
        ], concat!("Advance Decline LongShort_Versions_Without RSI.afl, coded as the AFL is written (entry tested before exit; no exit on the entry bar).", " Long only: the AFL's Short branch reads an undefined Close0 and stops with an AmiBroker error.", note_common!())),
        def(TWO_ST, "ADR Threshold with Two ST Filters", "Long", &[
            F_AR, "E|Long: AR at or below the threshold, at the close", F_ENTRY_N, F_AVG, F_PREV, F_ST,
            "F|ST 2 filter (off): a second ST of the same kind, with its own settings",
            "X|N Candles, AD Ratio, green-candle or up-close count, as in the 'Without RSI' AFL; either ST turning against its setting; at the close",
            X_STOPS, X_LAST,
        ], concat!("Advance Decline LongShort_Versions_Two_SuperTrend.afl, coded as the AFL is written (the 'Without RSI' rules plus ST 2).", " Long only: the AFL's Short branch reads an undefined Close0 and stops with an AmiBroker error.", note_common!())),
        def(NDAY, "ADR Consecutive Days with ST Filter, N-Day Exit", "Long", &[
            F_AR, "E|Long: AR at or below the threshold (0) on N (1) consecutive bars and the daily ST (5, 1) bullish, at the close", F_AVG, F_PREV,
            "X|N Candles (N 0 = off) or AD Ratio exit; the ST turning bearish; at the close", X_STOPS, "X|Buffer 0.25 %", X_LAST,
        ], concat!("Exit by N days/HSL- PB Optim/V1+V2/Advance Decline LongShort_Versions_Without RSI.afl, coded as the AFL is written (the older 'Without RSI' version: exit tested before entry, N Candles / AD Ratio exits only, ST filter and consecutive candles on, buffer 0.25 %).", " The pb/ copy has the same rules and defaults (only Optimize ranges differ); the sl+pb/ copy differs only by the ST filter off by default (st_on 0).", " Long only: the Short branch reads an undefined Close0 and stops with an AmiBroker error.", note_common!())),
        def(P211, "2-11 / 5DP Month Window with ADR Filter", "Long", &[
            "F|Window (Patch): 2-11 = from the Entry Day-th (5) trading day of a month to Exit Day (5) bars later; 5DP = from Entry Day - 1 bars before the month's last trading day to the Exit Day-th trading day of the next month",
            "F|ADR filter (off): AR at or below the threshold, consecutive or within the look back as in the other ADR AFLs",
            "F|RSI(14) and ADX(14) filters (off): below / above the threshold (1); ST filter (off) as in the ADR AFLs",
            "E|Buy at the close of a window bar while flat (the window re-arms only after it ends)",
            "X|N Candles (N 0 = off; tested from the entry bar, so N 1 exits on the entry bar), AD Ratio at or above the exit threshold, the 5DP or 2-11 exit day, the ST turning, PB / HSL; at the close",
            "X|An exit the AFL signals on its entry bar does not close the backtest position; the next Sell does",
            X_LAST,
        ], concat!("ADR Long 2-11 and 5DP.afl, coded as the AFL is written (SetForeign commented out: it trades the chart index).", " With the defaults (ADR filter off, N 0) it buys the first window bar and holds to the end of the data: choose an exit type. The 'Entry Type' list only switches the stop formulas and is not offered.", note_common!())),
        def(P211_ROC, "2-11 / 5DP Month Window with ADR and ROC Filters", "Long", &[
            "F|Window as in the 2-11 / 5DP AFL; with the 5DP or 2-11 exit chosen, the window's exit day is not a window bar",
            "F|ADR, RSI(14), ADX(14), ST filters as in the 2-11 / 5DP AFL; ROC filter (off): ROC(14) below (or above) the threshold (1)",
            "E|Buy at the close of a window bar while flat",
            "X|N Candles, AD Ratio, 5DP or 2-11 exit day, ST, PB / HSL, as in the 2-11 / 5DP AFL", X_LAST,
        ], concat!("ADR Long 2-11 and 5DP ROC.afl, coded as the AFL is written.", note_common!())),
        def(FDP, "Five Day Phenomenon with ADR Filter", "Long", &[
            "E|Buy at the close Entry Day - 1 (4) bars before the month's last trading day, when AR was at or below the threshold (0) on the N (1) bars (consecutive) or N of the look-back bars",
            "F|Candle colour: the last 2 bars green (default; red or any)",
            "F|Filter (off): ATHC (close above the highest close, off on a 20 % close swing down), monthly ST (5, 1, mid point) up, or the bull / bear % swing (44 / 23) up",
            "F|RSI(14) > 50, ADX(8) > 20, ROC(1) > 1 (or < -1): each off by default",
            "F|No entry on an upper-circuit bar (as written: (H == C == L) == O)",
            "X|The Exit Day-th (5) trading day of the next month, at the close; the ATHC regime ending or the ST turning down also exit",
            "X|Hard SL 1 % on the Low, at the SL (or Open - 0.25 % below it); profit booking 3 % on the High, at the target (or Open - 0.25 % above it, as written)",
            "X|A sell on a lower-circuit bar waits for the next bar that is not",
        ], concat!("FIVE Day Phenomena with ADR.afl (5DP + ADR), coded as the AFL is written; SetForeign(tickerName) makes it trade the index, so it runs on one index series. The 5DP entry looks ahead to the month's last trading day as the AFL does.", " The ROC / ATR / volatility ranking (PositionScore) only orders symbols in a portfolio and is not modelled; MaxOpenPositions 20, brokerage 0.65 % and sizing are portfolio settings.", " The breadth series is rebuilt over the Nifty 50 members of the team's Nifty 50 C&P database (the ~Advances universe).")),
    ]
}

// ---------------------------------------------------------------- parameters

fn p(key: &'static str, label: &'static str, def: f64, min: f64, max: f64, step: f64) -> ParamDef {
    ParamDef { key, label, def, min: min.min(def), max: max.max(def), step }
}

fn breadth_param() -> ParamDef {
    p("breadth", "Breadth universe (0 NSE 500 members AUX1, 1 all symbols, 2 Nifty 50 members)", 2.0, 0.0, 2.0, 1.0)
}

fn st_params(v: &mut Vec<ParamDef>, on: f64, fmax: f64) {
    v.push(p("st_on", "ST filter (0 off, 1 on)", on, 0.0, 1.0, 1.0));
    v.push(p("st_tf", "ST time frame (0 daily, 1 weekly, 2 monthly)", 0.0, 0.0, 2.0, 1.0));
    v.push(p("st_period", "Supertrend period", 5.0, 1.0, 10.0, 1.0));
    v.push(p("st_factor", "ST factor", 1.0, 0.5, fmax, 0.5));
    v.push(p("st_avg", "ST average (0 C, 1 O, 2 H, 3 L, 4 (O+C)/2, 5 (H+L)/2, 6 HLC/3, 7 OHLC/4)", 5.0, 0.0, 7.0, 1.0));
    v.push(p("st_bear", "ST entry filter (0 bullish, 1 bearish)", 0.0, 0.0, 1.0, 1.0));
}

fn adr_entry_params(v: &mut Vec<ParamDef>, consecutive: f64, lb_max: f64, n_max: f64, thr_max: f64) {
    v.push(p("consecutive", "Consecutive candles (0 off: within the look back, 1 on)", consecutive, 0.0, 1.0, 1.0));
    v.push(p("lookback", "Look back candles (consecutive off)", 1.0, 0.0, lb_max, 1.0));
    v.push(p("n_entry", "N consecutive candles", 1.0, 0.0, n_max, 1.0));
    v.push(p("adr_thr", "Entry ADR threshold", 0.0, 0.0, thr_max, 0.1));
}

fn stop_params(v: &mut Vec<ParamDef>, buffer: f64, hsl_max: f64) {
    v.push(p("exit_at", "Exit at (0 live: High / Low, 1 close)", 0.0, 0.0, 1.0, 1.0));
    v.push(p("pb_pct", "PB % (0 off)", 0.0, 0.0, 50.0, 0.1));
    v.push(p("hsl_pct", "HSL % (0 off)", 0.0, 0.0, hsl_max, 0.1));
    v.push(p("buffer", "Buffer %", buffer, 0.0, 10.0, 0.1));
}

fn filter_params(v: &mut Vec<ParamDef>, rsi_thr: f64) {
    v.push(p("rsi_on", "RSI filter (0 off, 1 on)", 0.0, 0.0, 1.0, 1.0));
    v.push(p("rsi_period", "RSI period", 14.0, 1.0, 80.0, 1.0));
    v.push(p("rsi_above", "RSI value (0 below, 1 above)", 0.0, 0.0, 1.0, 1.0));
    v.push(p("rsi_thr", "RSI threshold", rsi_thr, 1.0, 100.0, 1.0));
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let mut v = vec![breadth_param()];
    match id {
        BASIC => {
            v.push(p("entry_type", "Entry type (0 long, 1 short)", 0.0, 0.0, 1.0, 1.0));
            v.push(p("adr_upper", "Upper threshold (short entry)", 0.0, 0.0, 50.0, 0.1));
            v.push(p("adr_lower", "Lower threshold (long entry)", 0.0, 0.0, 2.0, 0.1));
            v.push(p("exit_type", "Exit type (0 N candle, 1 AD ratio: no rule)", 0.0, 0.0, 1.0, 1.0));
            v.push(p("exit_n", "N candles (0 off)", 0.0, 0.0, 10.0, 0.25));
        }
        VERSIONS | FIVE_EXIT | NO_RSI | TWO_ST | NDAY => {
            let kind = kind_of(id)?;
            if matches!(kind, Kind::Versions | Kind::FiveExit) {
                v.push(p("entry_type", "Entry type (0 long, 1 short)", 0.0, 0.0, 1.0, 1.0));
            }
            st_params(&mut v, if kind == Kind::NDay { 1.0 } else { 0.0 }, 10.0);
            if kind == Kind::TwoSt {
                v.push(p("st2_on", "ST 2 filter (0 off, 1 on)", 0.0, 0.0, 1.0, 1.0));
                v.push(p("st2_tf", "ST 2 time frame (0 daily, 1 weekly, 2 monthly)", 0.0, 0.0, 2.0, 1.0));
                v.push(p("st2_period", "Supertrend 2 period", 5.0, 1.0, 10.0, 1.0));
                v.push(p("st2_factor", "ST 2 factor", 1.0, 0.5, 6.0, 0.5));
                v.push(p("st2_avg", "ST 2 average (0 C .. 5 (H+L)/2 .. 7 OHLC/4)", 5.0, 0.0, 7.0, 1.0));
                v.push(p("st2_bear", "ST 2 entry filter (0 bullish, 1 bearish)", 0.0, 0.0, 1.0, 1.0));
            }
            if matches!(kind, Kind::Versions | Kind::FiveExit) {
                filter_params(&mut v, 1.0);
            }
            v.push(p("prev_on", "Previous N candles filter (0 off, 1 on)", 0.0, 0.0, 1.0, 1.0));
            v.push(p("prev_color", "Candle colour (0 green, 1 red, 2 any)", 0.0, 0.0, 2.0, 1.0));
            v.push(p("prev_n", "Previous N candles", 0.0, 0.0, 20.0, 1.0));
            v.push(p("avg_on", "Average ADR filter (0 off, 1 on)", 0.0, 0.0, 1.0, 1.0));
            v.push(p("avg_n", "N previous candles average", 1.0, 1.0, 100.0, 1.0));
            v.push(p("avg_thr", "Average threshold", 0.0, 0.0, 50.0, 0.1));
            adr_entry_params(&mut v, if kind == Kind::NDay { 1.0 } else { 0.0 }, 50.0, 20.0, 50.0);
            let types = match kind {
                Kind::Versions | Kind::NDay => "Exit type (0 N candles, 1 AD ratio)",
                Kind::FiveExit => "Exit type (0 N candles, 1 AD ratio, 2 5DP entry day)",
                _ => "Exit type (0 N candles, 1 AD ratio, 2 green candle, 3 up close)",
            };
            let tmax = match kind { Kind::Versions | Kind::NDay => 1.0, Kind::FiveExit => 2.0, _ => 3.0 };
            v.push(p("exit_type", types, 0.0, 0.0, tmax, 1.0));
            v.push(p("exit_n", "N exit candles (0 off)", 0.0, 0.0, 50.0, 1.0));
            v.push(p("exit_adr", "AD exit threshold", 0.0, 0.0, 50.0, 0.1));
            stop_params(&mut v, if kind == Kind::NDay { 0.25 } else { 0.0 }, 20.0);
        }
        P211 | P211_ROC => {
            st_params(&mut v, 0.0, 6.0);
            filter_params(&mut v, 1.0);
            v.push(p("adx_on", "ADX filter (0 off, 1 on)", 0.0, 0.0, 1.0, 1.0));
            v.push(p("adx_period", "ADX period", 14.0, 1.0, 80.0, 1.0));
            v.push(p("adx_above", "ADX value (0 below, 1 above)", 0.0, 0.0, 1.0, 1.0));
            v.push(p("adx_thr", "ADX threshold", 1.0, 1.0, 100.0, 1.0));
            if id == P211_ROC {
                v.push(p("roc_on", "ROC filter (0 off, 1 on)", 0.0, 0.0, 1.0, 1.0));
                v.push(p("roc_period", "ROC period", 14.0, 1.0, 80.0, 1.0));
                v.push(p("roc_above", "ROC value (0 below, 1 above)", 0.0, 0.0, 1.0, 1.0));
                v.push(p("roc_thr", "ROC threshold", 1.0, -100.0, 100.0, 1.0));
            }
            v.push(p("patch", "Patch to trade (0 2-11, 1 5DP)", 0.0, 0.0, 1.0, 1.0));
            v.push(p("entry_day", "Entry day", 5.0, 0.0, 40.0, 1.0));
            v.push(p("exit_day", "Exit day", 5.0, 0.0, 40.0, 1.0));
            v.push(p("adr_on", "ADR filter (0 off, 1 on)", 0.0, 0.0, 1.0, 1.0));
            adr_entry_params(&mut v, 0.0, 10.0, 6.0, 50.0);
            v.push(p("exit_type", "Exit type (0 N candles, 1 AD ratio, 2 5DP exit, 3 2-11 exit)", 0.0, 0.0, 3.0, 1.0));
            v.push(p("exit_n", "N exit candles (0 off)", 0.0, 0.0, 10.0, 1.0));
            v.push(p("exit_adr", "AD exit threshold", 0.0, 0.0, 50.0, 0.1));
            stop_params(&mut v, 0.0, 20.0);
        }
        FDP => {
            v.push(p("profit_pct", "Profit booking % (0 off)", 3.0, 0.0, 50.0, 1.0));
            v.push(p("hsl_pct", "Hard SL % (0 off)", 1.0, 0.0, 50.0, 1.0));
            v.push(p("buffer", "Selling buffer %", 0.25, 0.0, 10.0, 0.05));
            v.push(p("adr_mode", "ADR entry (0 N consecutive candles below, 1 N candles within the look back)", 0.0, 0.0, 1.0, 1.0));
            v.push(p("lookback", "Look back candles", 1.0, 1.0, 10.0, 1.0));
            v.push(p("n_entry", "Candles below threshold", 1.0, 1.0, 6.0, 1.0));
            v.push(p("adr_thr", "Entry ADR threshold", 0.0, 0.0, 50.0, 0.1));
            v.push(p("entry_day", "Entry day", 5.0, 1.0, 8.0, 1.0));
            v.push(p("exit_day", "Exit day", 5.0, 1.0, 8.0, 1.0));
            v.push(p("candle", "Candle colour (0 green, 1 red, 2 random)", 0.0, 0.0, 2.0, 1.0));
            v.push(p("candle_days", "Candle colour lookback days", 2.0, 1.0, 5.0, 1.0));
            v.push(p("filter", "Filter (0 off, 1 ATHC, 2 ST, 3 bull / bear)", 0.0, 0.0, 3.0, 1.0));
            v.push(p("athc_pct", "ATHC swing exit %", 20.0, 14.0, 24.0, 2.0));
            v.push(p("f_tf", "ST filter time frame (0 monthly, 1 weekly, 2 daily)", 0.0, 0.0, 2.0, 1.0));
            v.push(p("f_point", "ST calculation point (0 mid point, 1 close, 2 pivot)", 0.0, 0.0, 2.0, 1.0));
            v.push(p("f_period", "ST filter period", 5.0, 5.0, 25.0, 5.0));
            v.push(p("f_factor", "ST filter factor", 1.0, 1.0, 10.0, 1.0));
            v.push(p("bear_pct", "Swing % for bear market", 23.0, 10.0, 55.0, 5.0));
            v.push(p("bull_pct", "Swing % for bull market", 44.0, 10.0, 60.0, 5.0));
            v.push(p("rsi_on", "RSI filter (0 off, 1 on: RSI > value)", 0.0, 0.0, 1.0, 1.0));
            v.push(p("rsi_period", "RSI period", 14.0, 6.0, 22.0, 4.0));
            v.push(p("rsi_val", "RSI value", 50.0, 30.0, 70.0, 5.0));
            v.push(p("adx_on", "ADX filter (0 off, 1 on: ADX > value)", 0.0, 0.0, 1.0, 1.0));
            v.push(p("adx_period", "ADX period", 8.0, 6.0, 22.0, 4.0));
            v.push(p("adx_val", "ADX value", 20.0, 5.0, 30.0, 5.0));
            v.push(p("roc_on", "ROC filter (0 off, 1 on)", 0.0, 0.0, 1.0, 1.0));
            v.push(p("roc_below", "ROC threshold side (0 above, 1 below minus the threshold)", 0.0, 0.0, 1.0, 1.0));
            v.push(p("roc_days", "ROC days", 1.0, 1.0, 8.0, 1.0));
            v.push(p("roc_thr", "ROC threshold", 1.0, 1.0, 5.0, 1.0));
            v.extend(template_params());
        }
        _ => return None,
    }
    Some(v)
}

// ---------------------------------------------------------------- data and indicators

/// AR = A / D per traded-index bar (Foreign fix-up: a missing day repeats the previous count); `zero_dec` is the
/// value when D = 0 (None: A, as in the base AFL). None without the series.
fn ad_ratio(x: &Ctx, breadth: usize, zero_dec: Option<f64>) -> Option<Vec<f64>> {
    let [sa, sd] = SERIES[breadth];
    let fix = |v: &[f64]| {
        let mut o = v.to_vec();
        for i in 1..o.len() {
            if !o[i].is_finite() && o[i - 1].is_finite() {
                o[i] = o[i - 1];
            }
        }
        o
    };
    let (a, d) = (fix(x.series(sa)?), fix(x.series(sd)?));
    Some((0..a.len()).map(|i| if d[i] != 0.0 { a[i] / d[i] } else { zero_dec.unwrap_or(a[i]) }).collect())
}

/// The ADR AFLs' CalculateAvg.
fn avg_point(o: f64, h: f64, l: f64, c: f64, kind: i64) -> f64 {
    match kind {
        1 => o,
        2 => h,
        3 => l,
        4 => (o + c) / 2.0,
        5 => (h + l) / 2.0,
        6 => (h + l + c) / 3.0,
        7 => (o + h + l + c) / 4.0,
        _ => c,
    }
}

/// The ADR AFLs' ST loop on one time frame: the plotted line (IsUp ? LineDn : LineUp), NULL before it exists.
fn adr_st_line(o: &[f64], h: &[f64], l: &[f64], c: &[f64], period: usize, factor: f64, kind: i64) -> Vec<f64> {
    let m = c.len();
    let pd = period.max(1);
    let tr = true_range(h, l, c);
    let (mut lu, mut ld) = (vec![NULL; m], vec![NULL; m]);
    for i in pd..m {
        let atr = (i + 1 - pd..=i).map(|k| tr[k].unwrap_or(0.0)).sum::<f64>() / pd as f64;
        let a = avg_point(o[i], h[i], l[i], c[i], kind);
        lu[i] = a + atr * factor;
        ld[i] = a - atr * factor;
    }
    let mut up = vec![true; m];
    for i in pd..m {
        up[i] = if c[i] > lu[i - 1] { true } else if c[i] < ld[i - 1] { false } else { up[i - 1] };
        if up[i] {
            ld[i] = ld[i].max(ld[i - 1]);
        } else {
            lu[i] = lu[i].min(lu[i - 1]);
        }
    }
    (0..m).map(|i| if up[i] { ld[i] } else { lu[i] }).collect()
}

/// STBullPhase of the ADR AFLs on the chosen time frame (0 daily, 1 weekly, 2 monthly), on daily bars.
fn st_bull(x: &Ctx, tf: i64, period: usize, factor: f64, kind: i64) -> Vec<bool> {
    let n = x.c.len();
    if tf != 1 && tf != 2 {
        let line = adr_st_line(&x.o, &x.h, &x.l, &x.c, period, factor, kind);
        return (0..n).map(|i| !line[i].is_nan() && x.c[i] > line[i]).collect();
    }
    let b = compress(x, if tf == 1 { 'W' } else { 'M' });
    let mut o = vec![0.0; b.c.len()];
    for i in 0..n {
        if i == 0 || b.group[i] != b.group[i - 1] {
            o[b.group[i]] = x.o[i];
        }
    }
    let line = adr_st_line(&o, &b.h, &b.l, &b.c, period, factor, kind);
    // Ref(line, -1) on the period bars, expandFirst, then Ref(, 1) on the daily bars
    (0..n)
        .map(|d| {
            if d + 1 >= n {
                return false;
            }
            let g = b.group[d + 1];
            g >= 1 && !line[g - 1].is_nan() && b.c[g - 1] > line[g - 1]
        })
        .collect()
}

/// MainEntrySig: consecutive (count of signal bars in a row >= N) or within the look back (N of the last `lookback`
/// bars, from bar lookback + 1; a look back of 2 is read as 1).
fn main_entry(sig: &[bool], consecutive: bool, lookback: i64, n_entry: f64) -> Vec<bool> {
    let n = sig.len();
    let mut m = vec![false; n];
    if consecutive {
        let mut cnt = 0.0;
        for i in 1..n {
            cnt = if sig[i] { cnt + 1.0 } else { 0.0 };
            if cnt >= n_entry {
                m[i] = true;
            }
        }
    } else {
        let lb = if lookback == 2 { 1 } else { lookback.max(0) } as usize;
        for i in lb + 1..n {
            let mut cnt = 0.0;
            for j in (0..lb).rev() {
                if sig[i - j] {
                    cnt += 1.0;
                }
                if cnt >= n_entry {
                    m[i] = true;
                    cnt = 0.0;
                }
            }
        }
    }
    m
}

/// MainEntrySig2: the bar ends `k` bars in a row of the colour (0 green C > O, 1 red C < O, 2 any).
fn prev_candles(x: &Ctx, color: i64, k: f64) -> Vec<bool> {
    let n = x.c.len();
    let mut m = vec![false; n];
    let mut cnt = 0.0;
    for i in (k.max(0.0) as usize)..n {
        let ok = match color {
            0 => x.c[i] > x.o[i],
            1 => x.c[i] < x.o[i],
            _ => true,
        };
        cnt = if ok { cnt + 1.0 } else { 0.0 };
        if cnt >= k {
            m[i] = true;
        }
    }
    m
}

/// `Month() != Ref(Month(), 1)`; false on the last bar (Ref beyond the data is Null).
fn month_end(x: &Ctx) -> Vec<bool> {
    let n = x.c.len();
    (0..n).map(|i| i + 1 < n && x.r[i].date.get(5..7) != x.r[i + 1].date.get(5..7)).collect()
}

/// `Ref(v, k)[i]`, false beyond the data.
fn at(v: &[bool], i: usize, k: i64) -> bool {
    let j = i as i64 + k;
    j >= 0 && (j as usize) < v.len() && v[j as usize]
}

fn adx(x: &Ctx, n: usize) -> Vec<Option<f64>> {
    dmi(x.r, n.max(1)).1.into_iter().map(|v| if v.is_finite() { Some(v) } else { None }).collect()
}

/// AFL ROC(C, n) in percent; Null for the first n bars.
fn roc(c: &[f64], n: usize) -> Vec<Option<f64>> {
    (0..c.len()).map(|i| if i >= n && c[i - n] != 0.0 { Some((c[i] - c[i - n]) / c[i - n] * 100.0) } else { None }).collect()
}

/// Below / above a threshold; Null is false.
fn side(v: Option<f64>, above: bool, thr: f64) -> bool {
    v.is_some_and(|v| if above { v > thr } else { v < thr })
}

/// The 5DP AFL's bull / bear close % swing: Buy_Swing = Close > Up_Trend_Filter AND Up_Trend_Filter > 0.
fn bull_bear_buy(c: &[f64], bull: f64, bear: f64) -> Vec<bool> {
    let n = c.len();
    let mut up: Vec<f64> = c.iter().map(|v| v + v * bull / 100.0).collect();
    let mut dn: Vec<f64> = c.iter().map(|v| v - v * bear / 100.0).collect();
    let mut tr = vec![1i8; n];
    let (mut utf, mut dtf) = (vec![None; n], vec![None; n]);
    let mut change = false;
    for i in 1..n {
        tr[i] = if c[i] > up[i - 1] {
            if tr[i - 1] == -1 {
                change = true;
            }
            1
        } else if c[i] < dn[i - 1] {
            if tr[i - 1] == 1 {
                change = true;
            }
            -1
        } else {
            change = false;
            tr[i - 1]
        };
        if tr[i] > 0 && dn[i] < dn[i - 1] {
            dn[i] = dn[i - 1];
        }
        if tr[i] < 0 && up[i] > up[i - 1] {
            up[i] = up[i - 1];
        }
        if tr[i] < 0 && tr[i - 1] > 0 {
            up[i] = c[i] + c[i] * bull / 100.0;
        }
        if tr[i] > 0 && tr[i - 1] < 0 {
            dn[i] = c[i] - c[i] * bear / 100.0;
        }
        if tr[i] == 1 {
            utf[i] = Some(dn[i]);
            if change {
                utf[i - 1] = dtf[i - 1];
                change = false;
            }
        } else {
            dtf[i] = Some(up[i]);
            if change {
                dtf[i - 1] = utf[i - 1];
                change = false;
            }
        }
    }
    (0..n).map(|i| utf[i].is_some_and(|u| c[i] > u && u > 0.0)).collect()
}

/// Long exit price of the PB / HSL blocks (live fills; at the close otherwise), HSL taking precedence as written.
fn long_exit_px(x: &Ctx, i: usize, live: bool, pb: Option<f64>, hsl: Option<f64>, buf: f64) -> f64 {
    let (o, l) = (x.o[i], x.l[i]);
    let mut px = x.c[i];
    if let (true, Some(v)) = (live, pb) {
        px = if o > v { l.max(o * (1.0 - buf)) } else { v };
    }
    if let (true, Some(v)) = (live, hsl) {
        px = if o < v { l.max(o * (1.0 - buf)) } else { v };
    }
    px
}

fn short_exit_px(x: &Ctx, i: usize, live: bool, pb: Option<f64>, hsl: Option<f64>, buf: f64) -> f64 {
    let (o, h) = (x.o[i], x.h[i]);
    let mut px = x.c[i];
    if let (true, Some(v)) = (live, pb) {
        px = if o < v { h.min(o * (1.0 + buf)) } else { v };
    }
    if let (true, Some(v)) = (live, hsl) {
        px = if o > v { h.min(o * (1.0 + buf)) } else { v };
    }
    px
}

// ---------------------------------------------------------------- run

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    let n = x.c.len();
    if n == 0 || from > to || to >= n {
        return Some(Vec::new());
    }
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(0.0, |d| d.def));
    let s = match id {
        BASIC => basic(x, &g),
        P211 | P211_ROC => patch(id, x, &g),
        FDP => five_day(x, &g),
        _ => versions(kind_of(id)?, x, &g),
    };
    Some(s.map_or_else(Vec::new, |s| backtest(x, &s, from, to)))
}

fn gi(g: &dyn Fn(&str) -> f64, k: &str) -> i64 {
    g(k).round() as i64
}
fn gu(g: &dyn Fn(&str) -> f64, k: &str) -> usize {
    g(k).round().max(1.0) as usize
}

// ---------------------------------------------------------------- Advance Decline LongShort.afl

fn basic(x: &Ctx, g: &dyn Fn(&str) -> f64) -> Option<Px> {
    let c = &x.c;
    let n = c.len();
    let ratio = ad_ratio(x, breadth_index(Some(gi(g, "breadth") as f64)), None)?;
    let short = gi(g, "entry_type") == 1;
    let (up, lo) = (g("adr_upper"), g("adr_lower"));
    let sig: Vec<bool> = (0..n).map(|i| if short { ratio[i] >= up } else { ratio[i] <= lo }).collect();
    let n_candle = gi(g, "exit_type") == 0;
    let nx = g("exit_n");
    let mut s = Px::new(n);
    let (mut pos, mut cnt) = (false, 0.0);
    for i in 1..n {
        if pos {
            if n_candle {
                cnt += 1.0;
            }
            if cnt == nx {
                let why = if n_candle { "N candles" } else { "AD ratio type: N of 0" };
                if short { s.cover(i, c[i], why) } else { s.sell(i, c[i], why) }
                pos = false;
            }
        }
        if !pos && sig[i] {
            if short { s.short[i] = true } else { s.buy[i] = true }
            pos = true;
            cnt = 0.0;
        }
    }
    Some(s)
}

// ---------------------------------------------------------------- the "Versions" family

#[derive(Clone, Copy, PartialEq, Debug)]
enum Kind {
    /// Versions.afl: RSI filter, exit before entry, long-formula stop tests for both sides
    Versions,
    /// with5DayExit.afl: Versions plus the "5DP Entry" exit, side-specific stop tests
    FiveExit,
    /// Versions_Without RSI.afl: entry before exit, green-candle / up-close exits, long only
    NoRsi,
    /// Two_SuperTrend.afl: NoRsi plus a second ST
    TwoSt,
    /// Exit by N days copies: exit before entry, N Candles / AD Ratio exits, long only
    NDay,
}

fn kind_of(id: &str) -> Option<Kind> {
    Some(match id {
        VERSIONS => Kind::Versions,
        FIVE_EXIT => Kind::FiveExit,
        NO_RSI => Kind::NoRsi,
        TWO_ST => Kind::TwoSt,
        NDAY => Kind::NDay,
        _ => return None,
    })
}

struct Pos {
    on: bool,
    cnt: f64,
    entry: usize,
    pb: Option<f64>,
    hsl: Option<f64>,
}

fn versions(kind: Kind, x: &Ctx, g: &dyn Fn(&str) -> f64) -> Option<Px> {
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let n = c.len();
    let ratio = ad_ratio(x, breadth_index(Some(gi(g, "breadth") as f64)), Some(50.0))?;
    let short = matches!(kind, Kind::Versions | Kind::FiveExit) && gi(g, "entry_type") == 1;
    // entry signal: AR (or its N-bar mean) against the threshold
    let navg = gu(g, "avg_n");
    let mut adr_avg = vec![0.0; n];
    for i in navg - 1..n {
        adr_avg[i] = ratio[i + 1 - navg..=i].iter().sum::<f64>() / navg as f64;
    }
    let (val, thr) = if gi(g, "avg_on") == 1 { (&adr_avg, g("avg_thr")) } else { (&ratio, g("adr_thr")) };
    let sig: Vec<bool> = (0..n).map(|i| if short { val[i] >= thr } else { val[i] <= thr }).collect();
    let main = main_entry(&sig, gi(g, "consecutive") == 1, gi(g, "lookback"), g("n_entry"));
    let prev_on = gi(g, "prev_on") == 1;
    let main2 = if prev_on { prev_candles(x, gi(g, "prev_color"), g("prev_n")) } else { vec![false; n] };
    let st_on = gi(g, "st_on") == 1;
    let bull = if st_on { st_bull(x, gi(g, "st_tf"), gu(g, "st_period"), g("st_factor"), gi(g, "st_avg")) } else { vec![false; n] };
    let st_bear = gi(g, "st_bear") == 1;
    let st2_on = kind == Kind::TwoSt && gi(g, "st2_on") == 1;
    let bull2 = if st2_on { st_bull(x, gi(g, "st2_tf"), gu(g, "st2_period"), g("st2_factor"), gi(g, "st2_avg")) } else { vec![false; n] };
    let st2_bear = gi(g, "st2_bear") == 1;
    let rsi_on = matches!(kind, Kind::Versions | Kind::FiveExit) && gi(g, "rsi_on") == 1;
    let rsi = if rsi_on { rsi_afl(c, g("rsi_period").round().max(0.0) as usize) } else { vec![None; n] };
    let (rsi_above, rsi_thr) = (gi(g, "rsi_above") == 1, g("rsi_thr"));
    // exits
    let et = gi(g, "exit_type");
    let counts = matches!(kind, Kind::NoRsi | Kind::TwoSt) && (et == 2 || et == 3);
    let nx = g("exit_n") - 1.0 + if counts { 1.0 } else { 0.0 };
    let ex = g("exit_adr");
    let me = month_end(x);
    let k5 = (g("exit_n") - 1.0).round() as i64;
    let live = gi(g, "exit_at") == 0;
    let (pbp, hslp, buf) = (g("pb_pct"), g("hsl_pct"), g("buffer") * 0.01);
    let entry_first = matches!(kind, Kind::NoRsi | Kind::TwoSt);

    let can_enter = |i: usize| {
        main[i]
            && (!rsi_on || side(rsi[i], rsi_above, rsi_thr))
            && (!st_on || if st_bear { !bull[i] } else { bull[i] })
            && (!st2_on || if st2_bear { !bull2[i] } else { bull2[i] })
            && (!prev_on || main2[i - 1])
    };
    let enter = |st: &mut Pos, s: &mut Px, i: usize| {
        if short { s.short[i] = true } else { s.buy[i] = true }
        let e = c[i];
        let k = if short { -1.0 } else { 1.0 };
        st.hsl = (hslp > 0.0).then(|| e * (1.0 - k * hslp * 0.01));
        st.pb = (pbp > 0.0).then(|| e * (1.0 + k * pbp * 0.01));
        st.on = true;
        st.cnt = 0.0;
        st.entry = i;
    };
    let check = |st: &mut Pos, s: &mut Px, i: usize| {
        let (lo, hi) = if live { (l[i], h[i]) } else { (c[i], c[i]) };
        let (hsl_hit, pb_hit) = if !short || kind == Kind::Versions {
            (st.hsl.is_some_and(|v| lo <= v), st.pb.is_some_and(|v| hi >= v))
        } else {
            (st.hsl.is_some_and(|v| hi > v), st.pb.is_some_and(|v| lo < v))
        };
        if counts && et == 2 && o[i] < c[i] {
            st.cnt += 1.0;
        }
        if counts && et == 3 && c[i] > c[i - 1] {
            st.cnt += 1.0;
        }
        let ad = et == 1 && if short { ratio[i] <= ex } else { ratio[i] >= ex };
        let five = kind == Kind::FiveExit && et == 2 && at(&me, i, k5);
        let count = (et == 0 || counts) && st.cnt == nx;
        let st1 = st_on && if st_bear { bull[i] } else { !bull[i] };
        let st2 = st2_on && if st2_bear { bull2[i] } else { !bull2[i] };
        let last = i + 1 == n;
        if hsl_hit || pb_hit || ad || five || count || st1 || st2 || last {
            let why = if hsl_hit {
                "HSL"
            } else if pb_hit {
                "PB"
            } else if ad {
                "AD ratio exit"
            } else if five {
                "5DP entry day"
            } else if count {
                match et { 2 => "Green candle count", 3 => "Up close count", _ => "N candles" }
            } else if st1 || st2 {
                "ST filter exit"
            } else {
                LAST_BAR
            };
            let (pb, hsl) = (st.pb.filter(|_| pb_hit), st.hsl.filter(|_| hsl_hit));
            if short {
                s.cover(i, bound(x, i, short_exit_px(x, i, live, pb, hsl, buf)), why);
            } else {
                s.sell(i, bound(x, i, long_exit_px(x, i, live, pb, hsl, buf)), why);
            }
            st.on = false;
            st.pb = None;
            st.hsl = None;
        }
        if et == 0 {
            st.cnt += 1.0;
        }
    };

    let mut s = Px::new(n);
    let mut st = Pos { on: false, cnt: 0.0, entry: 0, pb: None, hsl: None };
    for i in 1..n {
        if entry_first {
            if !st.on && can_enter(i) {
                enter(&mut st, &mut s, i);
            }
            if st.on && st.entry != i {
                check(&mut st, &mut s, i);
            }
        } else {
            if st.on {
                check(&mut st, &mut s, i);
            }
            if !st.on && can_enter(i) {
                enter(&mut st, &mut s, i);
            }
        }
    }
    Some(s)
}

// ---------------------------------------------------------------- ADR Long 2-11 and 5DP (and the ROC version)

fn patch(id: &str, x: &Ctx, g: &dyn Fn(&str) -> f64) -> Option<Px> {
    let (h, l, c) = (&x.h, &x.l, &x.c);
    let n = c.len();
    let ratio = ad_ratio(x, breadth_index(Some(gi(g, "breadth") as f64)), Some(50.0))?;
    let thr = g("adr_thr");
    let sig: Vec<bool> = ratio.iter().map(|&v| v <= thr).collect();
    let main = main_entry(&sig, gi(g, "consecutive") == 1, gi(g, "lookback"), g("n_entry"));
    let adr_on = gi(g, "adr_on") == 1;
    let rsi_on = gi(g, "rsi_on") == 1;
    let rsi = rsi_afl(c, g("rsi_period").round().max(0.0) as usize);
    let adx_on = gi(g, "adx_on") == 1;
    let adxv = adx(x, gu(g, "adx_period"));
    let roc_on = id == P211_ROC && gi(g, "roc_on") == 1;
    let rocv = roc(c, gu(g, "roc_period"));
    let st_on = gi(g, "st_on") == 1;
    let bull = if st_on { st_bull(x, gi(g, "st_tf"), gu(g, "st_period"), g("st_factor"), gi(g, "st_avg")) } else { vec![false; n] };
    let st_bear = gi(g, "st_bear") == 1;
    // the month windows
    let me = month_end(x);
    let (ed, xd) = (gi(g, "entry_day"), gi(g, "exit_day"));
    let e5: Vec<bool> = (0..n).map(|i| at(&me, i, ed - 1)).collect();
    let x5: Vec<bool> = (0..n).map(|i| at(&me, i, -xd)).collect();
    let e2: Vec<bool> = (0..n).map(|i| at(&me, i, -ed)).collect();
    let x2: Vec<bool> = (0..n).map(|i| at(&me, i, -ed - xd)).collect();
    let five = gi(g, "patch") == 1;
    let (pe, px) = if five { (&e5, &x5) } else { (&e2, &x2) };
    let mut win = vec![false; n];
    let mut flag = false;
    for i in 1..n {
        if pe[i] && !flag {
            win[i] = true;
            flag = true;
        }
        if flag {
            win[i] = true;
        }
        if px[i] && flag {
            win[i] = true;
            flag = false;
        }
    }
    let et = gi(g, "exit_type");
    if id == P211_ROC && (et == 2 || et == 3) {
        for i in 1..n {
            if px[i] {
                win[i] = false;
            }
        }
    }
    let nx = g("exit_n") - 1.0;
    let ex = g("exit_adr");
    let live = gi(g, "exit_at") == 0;
    let (pbp, hslp, buf) = (g("pb_pct"), g("hsl_pct"), g("buffer") * 0.01);
    let mut s = Px::new(n);
    let (mut pos, mut cnt) = (false, 0.0);
    let (mut pb, mut hsl): (Option<f64>, Option<f64>) = (None, None);
    for i in 1..n {
        let enter = !pos
            && (!adr_on || main[i])
            && (!rsi_on || side(rsi[i], gi(g, "rsi_above") == 1, g("rsi_thr")))
            && (!adx_on || side(adxv[i], gi(g, "adx_above") == 1, g("adx_thr")))
            && (!roc_on || side(rocv[i], gi(g, "roc_above") == 1, g("roc_thr")))
            && (!st_on || if st_bear { !bull[i] } else { bull[i] })
            && win[i];
        if enter {
            s.buy[i] = true;
            pb = (pbp > 0.0).then(|| c[i] * (1.0 + pbp * 0.01));
            hsl = (hslp > 0.0).then(|| c[i] * (1.0 - hslp * 0.01));
            pos = true;
            cnt = 0.0;
        }
        if pos {
            let (lo, hi) = if live { (l[i], h[i]) } else { (c[i], c[i]) };
            let (hsl_hit, pb_hit) = (hsl.is_some_and(|v| lo <= v), pb.is_some_and(|v| hi >= v));
            let count = et == 0 && cnt == nx;
            let ad = et == 1 && ratio[i] >= ex;
            let d5 = et == 2 && x5[i];
            let d2 = et == 3 && x2[i];
            let stx = st_on && if st_bear { bull[i] } else { !bull[i] };
            let last = i + 1 == n;
            if hsl_hit || pb_hit || count || ad || d5 || d2 || stx || last {
                let why = if hsl_hit {
                    "HSL"
                } else if pb_hit {
                    "PB"
                } else if count {
                    "N candles"
                } else if ad {
                    "AD ratio exit"
                } else if d5 {
                    "5DP exit day"
                } else if d2 {
                    "2-11 exit day"
                } else if stx {
                    "ST filter exit"
                } else {
                    LAST_BAR
                };
                let price = long_exit_px(x, i, live, pb.filter(|_| pb_hit), hsl.filter(|_| hsl_hit), buf);
                s.sell(i, bound(x, i, price), why);
                pb = None;
                hsl = None;
                pos = false;
            }
            if et == 0 {
                cnt += 1.0;
            }
        }
    }
    Some(s)
}

// ---------------------------------------------------------------- FIVE Day Phenomena with ADR

fn five_day(x: &Ctx, g: &dyn Fn(&str) -> f64) -> Option<Px> {
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let n = c.len();
    let ratio = ad_ratio(x, breadth_index(Some(gi(g, "breadth") as f64)), Some(50.0))?;
    let thr = g("adr_thr");
    let n_entry = g("n_entry");
    let w = if gi(g, "adr_mode") == 0 { gu(g, "n_entry") } else { gu(g, "lookback") };
    // Sum(AR <= thr, w) >= N; Sum is Null for the first w - 1 bars
    let adr_sig: Vec<bool> = (0..n).map(|i| i + 1 >= w && ratio[i + 1 - w..=i].iter().filter(|&&v| v <= thr).count() as f64 >= n_entry).collect();
    let me = month_end(x);
    let (ed, xd) = (gi(g, "entry_day"), gi(g, "exit_day"));
    let buysig: Vec<bool> = (0..n).map(|i| at(&me, i, ed - 1) && adr_sig[i]).collect();
    let mut sellsig: Vec<bool> = (0..n).map(|i| at(&me, i, -xd)).collect();
    // candle colour: the last k bars all green (C > O) or red (O > C)
    let k = gu(g, "candle_days");
    let colour = gi(g, "candle");
    let candle: Vec<bool> = (0..n)
        .map(|i| match colour {
            0 => i + 1 >= k && (i + 1 - k..=i).all(|j| c[j] > o[j]),
            1 => i + 1 >= k && (i + 1 - k..=i).all(|j| o[j] > c[j]),
            _ => true,
        })
        .collect();
    // filters
    let ftype = gi(g, "filter");
    let mut buy_f = vec![true; n];
    match ftype {
        1 => {
            let on = athc(c, &close_swing(c, g("athc_pct")), false);
            for i in 0..n {
                buy_f[i] = on[i];
                sellsig[i] = sellsig[i] || !on[i];
            }
        }
        2 => {
            let tf = match gi(g, "f_tf") { 1 => 'W', 2 => 'D', _ => 'M' };
            let point = match gi(g, "f_point") { 1 => 2, 2 => 3, _ => 1 };
            let mt = st_filter(x, tf, point, gu(g, "f_period"), g("f_factor"));
            for i in 0..n {
                buy_f[i] = mt[i] == 1;
                sellsig[i] = sellsig[i] || (i >= 1 && mt[i] == -1 && mt[i - 1] == 1);
            }
        }
        3 => buy_f = bull_bear_buy(c, g("bull_pct"), g("bear_pct")),
        _ => {}
    }
    let rsi_on = gi(g, "rsi_on") == 1;
    let rsi = rsi_afl(c, gu(g, "rsi_period"));
    let adx_on = gi(g, "adx_on") == 1;
    let adxv = adx(x, gu(g, "adx_period"));
    let roc_on = gi(g, "roc_on") == 1;
    let rocv = roc(c, gu(g, "roc_days"));
    let roc_thr = g("roc_thr");
    let roc_ok = |i: usize| if gi(g, "roc_below") == 1 { rocv[i].is_some_and(|v| v < -roc_thr) } else { rocv[i].is_some_and(|v| v > roc_thr) };
    let gd = Guard::of(g, volsig17);
    // UC = LC = H == C == L == O, evaluated left to right as AFL does
    let chain = |i: usize| (f64::from(u8::from(f64::from(u8::from(h[i] == c[i])) == l[i]))) == o[i];
    let cash = |i: usize| !chain(i) && buy_f[i] && gd.turnover(x, i) && c[i] > gd.penny && gd.dead_buy(x, i);
    let (prof, hslp, buf) = (g("profit_pct"), g("hsl_pct"), g("buffer") / 100.0);
    let mut s = Px::new(n);
    let (mut pos, mut skip) = (false, false);
    let (mut pt, mut hsl) = (BIG, 0.0);
    for i in 1..n {
        let dead = gd.dead_sell(x, i);
        let lc0 = !chain(i);
        if !pos {
            let ok = buysig[i]
                && cash(i)
                && candle[i]
                && (!rsi_on || rsi[i].is_some_and(|v| v > g("rsi_val")))
                && (!adx_on || adxv[i].is_some_and(|v| v > g("adx_val")))
                && (!roc_on || roc_ok(i));
            if ok {
                s.buy[i] = true;
                if prof > 0.0 {
                    pt = (1.0 + prof * 0.01) * c[i];
                }
                if hslp > 0.0 {
                    hsl = (1.0 - hslp * 0.01) * c[i];
                }
                pos = true;
            }
        } else if !skip {
            if l[i] <= hsl || h[i] >= pt || sellsig[i] || dead {
                if lc0 || dead {
                    let (px, why) = if l[i] <= hsl {
                        (if o[i] < hsl { o[i] - o[i] * buf } else { hsl }, "Hard SL")
                    } else if h[i] >= pt {
                        (if o[i] > pt { o[i] - o[i] * buf } else { pt }, "Profit booking")
                    } else {
                        (c[i], if dead { "Dead stock (last bar)" } else { "5DP exit day / filter" })
                    };
                    s.sell(i, bound(x, i, px), why);
                    pos = false;
                    hsl = 0.0;
                    pt = BIG;
                } else {
                    skip = true;
                }
            }
        } else if lc0 || dead {
            s.sell(i, c[i], "Deferred exit after a lower-circuit bar");
            pos = false;
            skip = false;
            hsl = 0.0;
            pt = BIG;
        }
    }
    Some(s)
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let name = catalogue().into_iter().find(|d| d.id == id)?.name;
    let fam: &'static str = IDS.iter().copied().find(|&k| k == id)?;
    Some((fam, name, "Advance Decline Ratio"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::books::ytwh::testbars::*;

    /// Context with breadth: AR = 1/3 on the `low` bars, 3 elsewhere.
    fn ctx(r: &[crate::Row], low: &[usize]) -> Ctx<'static> {
        let r: &'static [crate::Row] = Box::leak(r.to_vec().into_boxed_slice());
        let mut x = Ctx::new(r, r, &[]);
        let a: Vec<f64> = (0..r.len()).map(|i| if low.contains(&i) { 100.0 } else { 300.0 }).collect();
        let d: Vec<f64> = (0..r.len()).map(|i| if low.contains(&i) { 300.0 } else { 100.0 }).collect();
        x.extra.insert("ADVANCES_N50".into(), a);
        x.extra.insert("DECLINES_N50".into(), d);
        x
    }

    /// Flat green bars: (open 99.9, high 100.1, low 99.8, close 100).
    fn flat(n: usize) -> Vec<crate::Row> {
        bars(&vec![(99.9, 100.1, 99.8, 100.0); n])
    }

    /// Rising bars (the ADR ST is bullish).
    fn rising(n: usize) -> Vec<crate::Row> {
        bars(&(0..n).map(|k| { let c = 100.0 + k as f64; (c - 0.2, c + 0.5, c - 0.5, c) }).collect::<Vec<_>>())
    }

    fn trades(id: &str, x: &Ctx, kv: &[(&str, f64)]) -> Vec<(usize, usize, bool)> {
        run(id, x, 0, x.c.len() - 1, &ps(kv)).unwrap().into_iter().map(|t| (t.legs[0].i, t.exit_i, t.open)).collect()
    }

    #[test]
    fn no_breadth_no_trades() {
        let r = flat(60);
        let x = Ctx::new(&r, &r, &[]);
        for id in IDS {
            assert!(run(id, &x, 0, r.len() - 1, &Params::new()).unwrap().is_empty(), "{id}");
        }
    }

    #[test]
    fn breadth_gaps_repeat_the_previous_count() {
        let r = flat(4);
        let mut x = Ctx::new(&r, &r, &[]);
        x.extra.insert("ADVANCES".into(), vec![100.0, f64::NAN, 300.0, 0.0]);
        x.extra.insert("DECLINES".into(), vec![200.0, f64::NAN, 0.0, 0.0]);
        assert_eq!(ad_ratio(&x, 0, Some(50.0)).unwrap(), vec![0.5, 0.5, 50.0, 50.0]);
        assert_eq!(ad_ratio(&x, 0, None).unwrap()[2], 300.0);
        assert!(ad_ratio(&x, 1, None).is_none());
        assert!(ad_ratio(&x, 2, None).is_none());
    }

    #[test]
    fn breadth_defaults_to_nifty50() {
        assert_eq!(breadth_index(None), 2);
        assert_eq!(breadth_index(Some(0.0)), 0);
        assert_eq!(breadth_index(Some(1.0)), 1);
        assert_eq!(breadth_index(Some(2.0)), 2);
        assert_eq!(SERIES[breadth_index(None)], ["ADVANCES_N50", "DECLINES_N50"]);
    }

    #[test]
    fn basic_long_after_n_candles() {
        let x = ctx(&flat(30), &[10]);
        assert_eq!(trades(BASIC, &x, &[("adr_lower", 0.5), ("exit_n", 3.0)]), vec![(10, 13, false)]);
        // short: AR >= 2 on every other bar; enters on bar 1, exits 3 bars later, re-enters on the exit bar
        let t = run(BASIC, &x, 0, 29, &ps(&[("entry_type", 1.0), ("adr_upper", 2.0), ("exit_n", 3.0)])).unwrap();
        assert_eq!((t[0].dir, t[0].legs[0].i, t[0].exit_i, t[1].legs[0].i), (-1.0, 1, 4, 4));
    }

    #[test]
    fn versions_exit_after_n_candles_and_on_the_ad_ratio() {
        let x = ctx(&flat(30), &[10]);
        assert_eq!(trades(VERSIONS, &x, &[("adr_thr", 0.5), ("exit_n", 3.0)]), vec![(10, 13, false)]);
        // AD ratio exit: AR 3 >= 2 on the next bar
        assert_eq!(trades(VERSIONS, &x, &[("adr_thr", 0.5), ("exit_type", 1.0), ("exit_adr", 2.0)]), vec![(10, 11, false)]);
        // consecutive: two low bars in a row needed
        let x2 = ctx(&flat(30), &[10, 15, 16]);
        assert_eq!(trades(VERSIONS, &x2, &[("adr_thr", 0.5), ("consecutive", 1.0), ("n_entry", 2.0), ("exit_n", 3.0)]), vec![(16, 19, false)]);
    }

    #[test]
    fn five_day_exit_on_the_first_day_of_the_month() {
        // weekdays from 2024-01-01: bar 22 = 31 Jan, bar 23 = 1 Feb
        let x = ctx(&flat(40), &[10]);
        assert_eq!(trades(FIVE_EXIT, &x, &[("adr_thr", 0.5), ("exit_type", 2.0)]), vec![(10, 23, false)]);
        // N 3: the bar 2 bars before the month's last day (29 Jan)
        assert_eq!(trades(FIVE_EXIT, &x, &[("adr_thr", 0.5), ("exit_type", 2.0), ("exit_n", 3.0)]), vec![(10, 20, false)]);
    }

    #[test]
    fn no_rsi_counts_green_candles() {
        let x = ctx(&flat(30), &[10]);
        assert_eq!(trades(NO_RSI, &x, &[("adr_thr", 0.5), ("exit_n", 3.0)]), vec![(10, 13, false)]);
        // every bar is green: the 2nd green candle after entry
        assert_eq!(trades(NO_RSI, &x, &[("adr_thr", 0.5), ("exit_type", 2.0), ("exit_n", 2.0)]), vec![(10, 12, false)]);
    }

    #[test]
    fn two_st_needs_both_filters() {
        let x = ctx(&rising(40), &[20]);
        let kv = [("adr_thr", 0.5), ("exit_n", 3.0), ("st_on", 1.0), ("st2_on", 1.0)];
        assert_eq!(trades(TWO_ST, &x, &kv), vec![(20, 23, false)]);
        // a bearish ST 2 setting blocks the entry in a rising market
        assert!(trades(TWO_ST, &x, &[("adr_thr", 0.5), ("st_on", 1.0), ("st2_on", 1.0), ("st2_bear", 1.0)]).is_empty());
    }

    #[test]
    fn nday_defaults_need_a_bullish_st() {
        let x = ctx(&rising(40), &[20]);
        assert_eq!(trades(NDAY, &x, &[("adr_thr", 0.5), ("exit_n", 3.0)]), vec![(20, 23, false)]);
        // PB 0.3 % (120.36): the next bar opens above it (120.8), so the fill is Open - 0.25 % kept at the Low (120.5)
        let t = run(NDAY, &x, 0, 39, &ps(&[("adr_thr", 0.5), ("pb_pct", 0.3)])).unwrap();
        assert_eq!((t[0].exit_i, t[0].reason.as_str(), t[0].exit_px), (21, "PB", 120.5));
    }

    #[test]
    fn patch_211_window_and_exit_day() {
        // 2-11: entry on the 5th trading day of February (bar 27), 2-11 exit 5 bars later (bar 32)
        let x = ctx(&flat(45), &[]);
        assert_eq!(trades(P211, &x, &[("exit_type", 3.0)]), vec![(27, 32, false)]);
        assert_eq!(trades(P211_ROC, &x, &[("exit_type", 3.0)]), vec![(27, 32, false)]);
        // ADR filter on: the low-AR bar inside the window
        let x = ctx(&flat(45), &[29]);
        assert_eq!(trades(P211, &x, &[("exit_type", 3.0), ("adr_on", 1.0), ("adr_thr", 0.5)]), vec![(29, 32, false)]);
        // 5DP patch: from 4 bars before 31 Jan (bar 18) to the 5th trading day of February (bar 27)
        let x = ctx(&flat(45), &[]);
        // (and from bar 39, 4 bars before 29 Feb, a second window opens and is still held on the last bar)
        assert_eq!(trades(P211, &x, &[("patch", 1.0), ("exit_type", 2.0)]), vec![(18, 27, false), (39, 44, true)]);
    }

    #[test]
    fn patch_exit_on_the_entry_bar_keeps_the_position() {
        // N 1 exits on the entry bar in the AFL; the backtester ignores that Sell and closes on the next one (bar 28)
        let x = ctx(&flat(45), &[]);
        // and as the AFL keeps buying inside the window, each later Sell closes one trade and the Buy reopens it
        assert_eq!(trades(P211, &x, &[("exit_n", 1.0)])[..2], [(27, 28, false), (28, 29, false)]);
    }

    #[test]
    fn five_day_phenomenon_with_adr() {
        // entry 4 bars before 31 Jan (bar 18) on a low-AR day after two green bars; exit on the 5th day of February
        let x = ctx(&flat(45), &[18]);
        assert_eq!(trades(FDP, &x, &[("adr_thr", 0.5)]), vec![(18, 27, false)]);
        let x = ctx(&flat(45), &[17]);
        assert!(trades(FDP, &x, &[("adr_thr", 0.5)]).is_empty());
        // a 0.1 % profit target is hit on the next bar's high
        let x = ctx(&flat(45), &[18]);
        let t = run(FDP, &x, 0, 44, &ps(&[("adr_thr", 0.5), ("profit_pct", 0.1)])).unwrap();
        assert_eq!(t[0].exit_i, 19);
        assert!((t[0].exit_px - 100.1).abs() < 1e-9);
    }

    #[test]
    fn st_line_ratchets_and_frames() {
        let r = rising(40);
        let x = ctx(&r, &[]);
        let b = st_bull(&x, 0, 5, 1.0, 5);
        assert!(!b[4] && b[10] && b[39]);
        // weekly: the last bar has no next bar, so it is false
        let w = st_bull(&x, 1, 2, 1.0, 5);
        assert!(!w[39] && w[30]);
    }

    #[test]
    fn every_strategy_runs() {
        let r = wave(500);
        let low: Vec<usize> = (0..500).filter(|k| k % 7 == 0).collect();
        let x = ctx(&r, &low);
        let sets: [&[(&str, f64)]; 3] = [&[], &[("adr_thr", 0.5), ("exit_n", 4.0), ("pb_pct", 2.0), ("hsl_pct", 2.0), ("st_on", 1.0), ("st_tf", 2.0)], &[("adr_thr", 1.0), ("exit_type", 1.0), ("exit_adr", 2.0), ("filter", 2.0), ("rsi_on", 1.0), ("rsi_thr", 60.0)]];
        for id in IDS {
            for kv in sets {
                for (from, to) in [(0, r.len() - 1), (50, 300)] {
                    for t in run(id, &x, from, to, &ps(kv)).unwrap() {
                        assert!(t.legs[0].i >= from && t.exit_i <= to && t.legs[0].i <= t.exit_i, "{id}");
                        assert!(t.exit_px >= x.l[t.exit_i] - 1e-9 && t.exit_px <= x.h[t.exit_i] + 1e-9, "{id}");
                    }
                }
            }
        }
    }
}
